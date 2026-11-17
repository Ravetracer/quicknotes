//! Live Pencil shapes on canvases.
//!
//! Pencil stencils are programs: their shapes compute geometry, colours and
//! text with JavaScript expressions. A clean-room engine (`data/pencil/`)
//! evaluates them in one hidden web view that is shared by all canvases:
//! - it has no script-message bridge, a content filter blocks every network,
//!   file and custom-scheme load, and it never navigates – stencil code runs
//!   there and nowhere else;
//! - packages are handed over as file maps (text as strings, everything else
//!   as data URLs), the results come back as JSON (sanitised SVG markup,
//!   property schema, text-edit and handle positions, actions).
//!
//! Requests are queued and run one after the other.

use gtk::{gio, glib};
use serde_json::Value;
use std::cell::{Cell, RefCell};
use std::collections::{HashMap, VecDeque};
use std::io::Read;
use std::rc::Rc;
use webkit6::prelude::*;

const ENGINE_JS: &str = include_str!("../data/pencil/engine.js");
const PAGE: &str = include_str!("../data/pencil/renderer.html");

/// Blocks every load except data: URLs and the page itself (WebKit's
/// filter patterns know no alternation: one rule per scheme).
const FILTER: &str = r#"[
{"trigger":{"url-filter":"^https?:"},"action":{"type":"block"}},
{"trigger":{"url-filter":"^wss?:"},"action":{"type":"block"}},
{"trigger":{"url-filter":"^ftp:"},"action":{"type":"block"}},
{"trigger":{"url-filter":"^file:"},"action":{"type":"block"}},
{"trigger":{"url-filter":"^blob:"},"action":{"type":"block"}},
{"trigger":{"url-filter":"^qn-"},"action":{"type":"block"}}
]"#;

pub type Done = Box<dyn FnOnce(Result<Value, String>)>;

/// Where the package of a request comes from.
#[derive(Clone, Debug)]
pub enum Source {
    /// The image folder of a canvas (note id).
    Canvas(String),
    /// An installed collection (collection id).
    Library(String),
}

struct Job {
    pkg: String,
    source: Source,
    /// `render`, `action`, `resume` or `collectionSettings`.
    func: &'static str,
    req: Value,
    done: Done,
}

struct Renderer {
    webview: webkit6::WebView,
    ready: Cell<bool>,
    busy: Cell<bool>,
    queue: RefCell<VecDeque<Job>>,
    /// Loaded packages and their font CSS.
    fonts: RefCell<HashMap<String, String>>,
}

thread_local! {
    static RENDERER: RefCell<Option<Rc<Renderer>>> = const { RefCell::new(None) };
}

fn renderer() -> Rc<Renderer> {
    if let Some(r) = RENDERER.with(|r| r.borrow().clone()) {
        return r;
    }
    let ucm = webkit6::UserContentManager::new();
    let webview = webkit6::WebView::builder().user_content_manager(&ucm).build();
    if let Some(settings) = webkit6::prelude::WebViewExt::settings(&webview) {
        settings.set_enable_javascript(true);
        settings.set_allow_file_access_from_file_urls(false);
        settings.set_allow_universal_access_from_file_urls(false);
        settings.set_enable_developer_extras(false);
        settings.set_enable_write_console_messages_to_stdout(std::env::var_os("QN_WEB_CONSOLE").is_some());
    }
    let r = Rc::new(Renderer {
        webview: webview.clone(),
        ready: Cell::new(false),
        busy: Cell::new(false),
        queue: RefCell::new(VecDeque::new()),
        fonts: RefCell::new(HashMap::new()),
    });
    // The page never navigates once it is loaded.
    let loaded = Rc::new(Cell::new(false));
    let flag = loaded.clone();
    webview.connect_decide_policy(move |_, decision, kind| {
        if kind == webkit6::PolicyDecisionType::NavigationAction && flag.get() {
            decision.ignore();
            return true;
        }
        false
    });
    let weak = Rc::downgrade(&r);
    webview.connect_load_changed(move |_, event| {
        if event != webkit6::LoadEvent::Finished {
            return;
        }
        loaded.set(true);
        if let Some(r) = weak.upgrade() {
            r.ready.set(true);
            next(&r);
        }
    });
    RENDERER.with(|slot| *slot.borrow_mut() = Some(r.clone()));

    // Install the content filter first, then load the page.
    let store_dir = dirs::cache_dir().unwrap_or_else(std::env::temp_dir).join("quicknotes").join("content-filters");
    let store = webkit6::UserContentFilterStore::new(&store_dir.to_string_lossy());
    let page = PAGE.replace("{ENGINE_JS}", &ENGINE_JS.replace("</script", "<\\/script"));
    let wv = webview.clone();
    store.save("quicknotes-pencil", &glib::Bytes::from_static(FILTER.as_bytes()), None::<&gio::Cancellable>, move |filter| {
        match filter {
            Ok(filter) => {
                ucm.add_filter(&filter);
                wv.load_html(&page, None);
            }
            // Without the filter the engine does not run at all.
            Err(e) => eprintln!("quicknotes: cannot set up the Pencil sandbox: {e}"),
        }
    });
    r
}

/// Runs `func` (`render` or `action`) of the engine with `req` (which gets
/// `pkg`), loading the package first when needed.
pub fn request(pkg: &str, source: Source, func: &'static str, mut req: Value, done: impl FnOnce(Result<Value, String>) + 'static) {
    if !crate::canvas::valid_name(pkg) || !pkg.ends_with(".zip") {
        done(Err("invalid package".into()));
        return;
    }
    req["pkg"] = Value::from(pkg);
    let r = renderer();
    r.queue.borrow_mut().push_back(Job { pkg: pkg.to_owned(), source, func, req, done: Box::new(done) });
    next(&r);
}

/// The `@font-face` rules of a loaded package.
pub fn font_css(pkg: &str) -> Option<String> {
    RENDERER.with(|r| r.borrow().as_ref().and_then(|r| r.fonts.borrow().get(pkg).cloned()))
}

/// Runs the next queued job.
fn next(r: &Rc<Renderer>) {
    if !r.ready.get() || r.busy.get() {
        return;
    }
    let Some(job) = r.queue.borrow_mut().pop_front() else { return };
    r.busy.set(true);
    if r.fonts.borrow().contains_key(&job.pkg) {
        run(r, job);
        return;
    }
    let files = match package_files(&job.pkg, &job.source) {
        Ok(f) => f,
        Err(e) => {
            finish(r, job.done, Err(e));
            return;
        }
    };
    let arg = serde_json::json!({ "pkg": job.pkg, "files": files }).to_string();
    let r2 = r.clone();
    call(r, "loadPackage", arg, move |result| {
        match result.and_then(|v| if v["ok"] == true { Ok(v) } else { Err(v["error"].as_str().unwrap_or("cannot load").to_owned()) }) {
            Ok(v) => {
                let css = v["fontCss"].as_str().unwrap_or_default().to_owned();
                r2.fonts.borrow_mut().insert(job.pkg.clone(), css);
                run(&r2, job);
            }
            Err(e) => finish(&r2, job.done, Err(e)),
        }
    });
}

fn run(r: &Rc<Renderer>, job: Job) {
    let r2 = r.clone();
    let done = job.done;
    call(r, job.func, job.req.to_string(), move |result| finish(&r2, done, result));
}

fn finish(r: &Rc<Renderer>, done: Done, result: Result<Value, String>) {
    done(result);
    r.busy.set(false);
    let r = r.clone();
    // Let the main loop breathe between jobs.
    glib::idle_add_local_once(move || next(&r));
}

/// Calls `PE.<func>(arg)` and parses the JSON it returns.
fn call(r: &Renderer, func: &str, arg: String, done: impl FnOnce(Result<Value, String>) + 'static) {
    let dict = glib::VariantDict::new(None);
    dict.insert("arg", &arg);
    let body = format!("return await PE.{func}(arg);");
    r.webview.call_async_javascript_function(&body, Some(&dict.end()), None, None, None::<&gio::Cancellable>, move |result| {
        let parsed = result
            .map_err(|e| e.message().to_owned())
            .and_then(|v| serde_json::from_str::<Value>(&v.to_str()).map_err(|e| e.to_string()));
        done(parsed);
    });
}

/// The files of a package: text files as strings, the rest as data URLs.
fn package_files(pkg: &str, source: &Source) -> Result<serde_json::Map<String, Value>, String> {
    let data = match source {
        Source::Canvas(id) => crate::store::read_asset(id, pkg),
        Source::Library(id) => crate::library_art::package_data(id),
    }
    .ok_or_else(|| t!("The stencil collection of this shape is missing").to_string())?;
    unpack(&data)
}

pub fn unpack(data: &[u8]) -> Result<serde_json::Map<String, Value>, String> {
    use base64::Engine;
    let package = crate::library_art::read_package(data)?;
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(data)).map_err(|e| e.to_string())?;
    let mut files = serde_json::Map::new();
    for path in &package.files {
        let Ok(mut file) = zip.by_name(&format!("{}{path}", package.root)) else { continue };
        let mut bytes = Vec::new();
        if file.read_to_end(&mut bytes).is_err() {
            continue;
        }
        let ext = path.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
        let text = matches!(ext.as_str(), "xml" | "svg" | "css" | "js" | "txt" | "xhtml" | "html" | "json");
        let value = if text {
            String::from_utf8_lossy(&bytes).into_owned()
        } else {
            // Some packages ship SVG text under .png names.
            let mime = match crate::canvas::sniff(&bytes) {
                Some(kind) => crate::canvas::mime_type(&format!("x.{kind}")),
                None => crate::library_art::mime_type(path),
            };
            format!("data:{mime};base64,{}", base64::engine::general_purpose::STANDARD.encode(&bytes))
        };
        files.insert(path.clone(), Value::from(value));
    }
    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packages_are_unpacked() {
        let zip = crate::library_art::tests::package_zip(
            "W",
            crate::library_art::tests::SHAPES,
            &[("icons/button.png", b"\x89PNG\r\n\x1a\nx"), ("bitmaps/fake.png", b"<svg xmlns='http://www.w3.org/2000/svg'/>")],
        );
        let files = unpack(&zip).unwrap();
        assert!(files["Definition.xml"].as_str().unwrap().contains("test.Wire"));
        assert!(files["icons/button.png"].as_str().unwrap().starts_with("data:image/png;base64,"));
        assert!(files["bitmaps/fake.png"].as_str().unwrap().starts_with("data:image/svg+xml;base64,"));
    }
}
