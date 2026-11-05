//! PDF export: renders an HTML document in an off-screen WebKit view and
//! prints it to GTK's "Print to File" printer.

use gtk::prelude::*;
use gtk::{gio, glib};
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::Duration;
use webkit6::prelude::*;

const TIMEOUT: Duration = Duration::from_secs(60);

type Done = Box<dyn FnOnce(Result<(), String>)>;
/// Off-screen view and print operation, kept alive until printing is done.
type Keep = Rc<RefCell<Option<(webkit6::WebView, Option<webkit6::PrintOperation>)>>>;

/// Writes `html` as PDF to `out`. `base_uri` resolves relative images.
/// `done` is called exactly once.
pub fn pdf(html: String, base_uri: Option<String>, out: PathBuf, done: impl FnOnce(Result<(), String>) + 'static) {
    let done: Rc<RefCell<Option<Done>>> = Rc::new(RefCell::new(Some(Box::new(done))));
    let finish = {
        let done = done.clone();
        move |result: Result<(), String>| {
            if let Some(f) = done.borrow_mut().take() {
                f(result);
            }
        }
    };
    let finish = Rc::new(finish);

    // The printer is looked up by its properties: its name is localized
    // ("Print to File", "In Datei drucken", …) and a wrong name could end up
    // on a real printer.
    let started = Rc::new(std::cell::Cell::new(false));
    let job = Rc::new(RefCell::new(Some((html, base_uri, out))));
    let on_printer = {
        let (started, finish) = (started.clone(), finish.clone());
        move |printer: &gtk::Printer| -> bool {
            if started.get() || !is_file_printer(printer) {
                return false;
            }
            started.set(true);
            if let Some((html, base, out)) = job.borrow_mut().take() {
                print(printer.clone(), html, base, out, finish.clone());
            }
            true
        }
    };
    // The callback runs on the main thread, but gtk-rs requires `Send`.
    let on_printer = glib::thread_guard::ThreadGuard::new(on_printer);
    gtk::enumerate_printers(move |printer| (on_printer.get_ref())(printer), false);
    // Without a file printer nothing would ever call back.
    let finish_timeout = finish.clone();
    glib::timeout_add_local_once(Duration::from_secs(10), move || {
        if !started.get() {
            finish_timeout(Err(t!("GTK's \"Print to File\" printer is not available").into()));
        }
    });
    let finish_overall = finish;
    glib::timeout_add_local_once(TIMEOUT, move || finish_overall(Err(t!("PDF export timed out").into())));
}

fn is_file_printer(printer: &gtk::Printer) -> bool {
    printer.is_virtual() && printer.icon_name().as_str() == "document-save" && printer.accepts_pdf()
}

fn print(
    printer: gtk::Printer,
    html: String,
    base_uri: Option<String>,
    out: PathBuf,
    finish: Rc<impl Fn(Result<(), String>) + 'static>,
) {
    let webview = webkit6::WebView::new();
    let keep: Keep = Rc::new(RefCell::new(Some((webview.clone(), None))));

    let keep_load = keep.clone();
    webview.connect_load_changed(move |wv, event| {
        if event != webkit6::LoadEvent::Finished {
            return;
        }
        let settings = gtk::PrintSettings::new();
        settings.set_printer(&printer.name());
        settings.set(gtk::PRINT_SETTINGS_OUTPUT_FILE_FORMAT, Some("pdf"));
        settings.set(gtk::PRINT_SETTINGS_OUTPUT_URI, Some(gio::File::for_path(&out).uri().as_str()));

        let setup = gtk::PageSetup::new();
        setup.set_paper_size(&gtk::PaperSize::new(Some(gtk::PAPER_NAME_A4)));
        setup.set_top_margin(18.0, gtk::Unit::Mm);
        setup.set_bottom_margin(18.0, gtk::Unit::Mm);
        setup.set_left_margin(18.0, gtk::Unit::Mm);
        setup.set_right_margin(18.0, gtk::Unit::Mm);

        let op = webkit6::PrintOperation::new(wv);
        op.set_print_settings(&settings);
        op.set_page_setup(&setup);
        let (keep_ok, finish_ok) = (keep_load.clone(), finish.clone());
        op.connect_finished(move |_| {
            keep_ok.borrow_mut().take();
            finish_ok(Ok(()));
        });
        let (keep_err, finish_err) = (keep_load.clone(), finish.clone());
        op.connect_failed(move |_, err| {
            keep_err.borrow_mut().take();
            finish_err(Err(err.message().to_owned()));
        });
        if let Some((_, slot)) = keep_load.borrow_mut().as_mut() {
            *slot = Some(op.clone());
        }
        op.print();
    });
    webview.load_html(&html, base_uri.as_deref());
}
