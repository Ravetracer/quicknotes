//! Reads the data of an X11 drag-and-drop operation directly from the drag
//! source.
//!
//! Workaround for GTK 4 on X11 with Qt drag sources (Dolphin, …): GTK 4
//! requests `XdndSelection` with an internal helper window as requestor. Qt
//! only hands out the drag data when the requestor is the window the drop
//! happened on, so GTK's read never completes. Here the request is made on a
//! separate X connection with the toplevel window as requestor (which is what
//! GTK 3 does). The data arrives as a property on that window, which this
//! connection watches via `PropertyNotify`.

use std::time::{Duration, Instant};
use x11rb::connection::Connection;
use x11rb::protocol::xproto::{AtomEnum, ChangeWindowAttributesAux, ConnectionExt as _, EventMask, Property};
use x11rb::protocol::Event;

const TIMEOUT: Duration = Duration::from_secs(2);

/// Converts the current drag data to `target` (a MIME type or X11 target
/// name such as `UTF8_STRING`). Blocking; run it off the main thread.
/// `window` is the XID of the toplevel window that received the drop.
pub fn read(window: u32, target: &str) -> Result<Vec<u8>, String> {
    let err = |e: &dyn std::fmt::Display| e.to_string();
    let (conn, _) = x11rb::connect(None).map_err(|e| err(&e))?;
    let atom = |name: &str| -> Result<u32, String> {
        Ok(conn
            .intern_atom(false, name.as_bytes())
            .map_err(|e| err(&e))?
            .reply()
            .map_err(|e| err(&e))?
            .atom)
    };
    let selection = atom("XdndSelection")?;
    let target = atom(target)?;
    let property = atom("QUICKNOTES_XDND_DATA")?;
    let incr = atom("INCR")?;

    let watch = |mask: EventMask| -> Result<(), String> {
        conn.change_window_attributes(window, &ChangeWindowAttributesAux::new().event_mask(mask))
            .map_err(|e| err(&e))?;
        Ok(())
    };
    watch(EventMask::PROPERTY_CHANGE)?;
    let result = (|| {
        conn.delete_property(window, property).map_err(|e| err(&e))?;
        conn.convert_selection(window, selection, target, property, x11rb::CURRENT_TIME)
            .map_err(|e| err(&e))?;
        conn.flush().map_err(|e| err(&e))?;

        let deadline = Instant::now() + TIMEOUT;
        loop {
            match conn.poll_for_event().map_err(|e| err(&e))? {
                Some(Event::PropertyNotify(ev)) if ev.atom == property && ev.state == Property::NEW_VALUE => break,
                Some(_) => {}
                None if Instant::now() > deadline => return Err("drag source did not answer".to_owned()),
                None => std::thread::sleep(Duration::from_millis(10)),
            }
        }
        let reply = conn
            .get_property(true, window, property, AtomEnum::ANY, 0, u32::MAX / 4)
            .map_err(|e| err(&e))?
            .reply()
            .map_err(|e| err(&e))?;
        if reply.type_ == incr {
            return Err("incremental transfers are not supported".to_owned());
        }
        Ok(reply.value)
    })();
    let _ = watch(EventMask::NO_EVENT);
    let _ = conn.flush();
    result
}
