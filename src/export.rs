//! PDF export: renders an HTML document in an off-screen WebKit view and
//! prints it to GTK's "Print to File" printer. Image exports convert that
//! PDF (see [`crate::images`]); the EPUB export reads the rendered page.

use gtk::prelude::*;
use gtk::{gio, glib};
use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::Duration;
use webkit6::prelude::*;

const TIMEOUT: Duration = Duration::from_secs(60);

type Done = Box<dyn FnOnce(Result<(), String>)>;

/// Paper format of the PDF.
#[derive(Clone, Copy, Debug)]
pub enum Page {
    /// A4 portrait with margins (documents).
    A4,
    /// A custom size without margins (presentation slides).
    Custom { width_mm: f64, height_mm: f64 },
    /// One page of A4 width as tall as the document (its height on screen
    /// times `factor`, as print layout differs a little), with margins.
    Tall { factor: f64 },
}

const A4_WIDTH_MM: f64 = 210.0;
const TALL_MARGIN_MM: f64 = 12.0;
/// Off-screen view and print operation, kept alive until printing is done.
type Keep = Rc<RefCell<Option<(webkit6::WebView, Option<webkit6::PrintOperation>)>>>;

/// Writes `html` as PDF to `out`. `base_uri` resolves relative images.
/// `done` is called exactly once.
pub fn pdf(
    html: String,
    base_uri: Option<String>,
    out: PathBuf,
    page: Page,
    done: impl FnOnce(Result<(), String>) + 'static,
) {
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
                print(printer.clone(), html, base, out, page, finish.clone());
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
    page: Page,
    finish: Rc<impl Fn(Result<(), String>) + 'static>,
) {
    let webview = webkit6::WebView::new();
    let keep: Keep = Rc::new(RefCell::new(Some((webview.clone(), None))));

    let keep_load = keep.clone();
    webview.connect_load_changed(move |wv, event| {
        if event != webkit6::LoadEvent::Finished {
            return;
        }
        let Page::Tall { factor } = page else {
            print_loaded(wv, &printer, &out, page, &keep_load, finish.clone());
            return;
        };
        // The content height at the printed width, in CSS pixels.
        let width_px = (A4_WIDTH_MM - 2.0 * TALL_MARGIN_MM) * 96.0 / 25.4;
        let js = format!(
            "(function(){{var e=document.documentElement,w=e.style.width;e.style.width='{width_px:.0}px';\
             var h=e.scrollHeight;e.style.width=w;return h;}})()"
        );
        let (printer, out, keep, finish) = (printer.clone(), out.clone(), keep_load.clone(), finish.clone());
        let view = wv.clone();
        wv.evaluate_javascript(&js, None, None, None::<&gio::Cancellable>, move |result| {
            let px = result.ok().map(|v| v.to_double()).filter(|h| h.is_finite() && *h > 0.0).unwrap_or(1123.0);
            let height_mm = px * 25.4 / 96.0 * factor + 2.0 * TALL_MARGIN_MM;
            print_loaded(&view, &printer, &out, Page::Tall { factor: height_mm }, &keep, finish);
        });
    });
    webview.load_html(&html, base_uri.as_deref());
}

/// Prints the loaded page of `wv` to `out`. For [`Page::Tall`], `factor`
/// is the page height in mm by now.
fn print_loaded(
    wv: &webkit6::WebView,
    printer: &gtk::Printer,
    out: &Path,
    page: Page,
    keep: &Keep,
    finish: Rc<impl Fn(Result<(), String>) + 'static>,
) {
    let settings = gtk::PrintSettings::new();
    settings.set_printer(&printer.name());
    settings.set(gtk::PRINT_SETTINGS_OUTPUT_FILE_FORMAT, Some("pdf"));
    settings.set(gtk::PRINT_SETTINGS_OUTPUT_URI, Some(gio::File::for_path(out).uri().as_str()));

    let setup = gtk::PageSetup::new();
    let custom = |width_mm, height_mm| {
        let paper = gtk::PaperSize::new_custom("quicknotes-custom", "Custom", width_mm, height_mm, gtk::Unit::Mm);
        settings.set_paper_size(&paper);
        paper
    };
    let (paper, margin) = match page {
        Page::A4 => (gtk::PaperSize::new(Some(gtk::PAPER_NAME_A4)), 18.0),
        Page::Custom { width_mm, height_mm } => (custom(width_mm, height_mm), 0.0),
        Page::Tall { factor: height_mm } => (custom(A4_WIDTH_MM, height_mm), TALL_MARGIN_MM),
    };
    setup.set_paper_size(&paper);
    setup.set_orientation(gtk::PageOrientation::Portrait);
    setup.set_top_margin(margin, gtk::Unit::Mm);
    setup.set_bottom_margin(margin, gtk::Unit::Mm);
    setup.set_left_margin(margin, gtk::Unit::Mm);
    setup.set_right_margin(margin, gtk::Unit::Mm);

    let op = webkit6::PrintOperation::new(wv);
    op.set_print_settings(&settings);
    op.set_page_setup(&setup);
    let (keep_ok, finish_ok) = (keep.clone(), finish.clone());
    op.connect_finished(move |_| {
        keep_ok.borrow_mut().take();
        finish_ok(Ok(()));
    });
    let (keep_err, finish_err) = (keep.clone(), finish);
    op.connect_failed(move |_, err| {
        keep_err.borrow_mut().take();
        finish_err(Err(err.message().to_owned()));
    });
    if let Some((_, slot)) = keep.borrow_mut().as_mut() {
        *slot = Some(op.clone());
    }
    op.print();
}

/// Shrinks a written PDF: WebKit embeds a font file again for many runs of
/// text, so identical streams are kept once.
pub fn dedup_pdf(path: &Path) -> Result<(), String> {
    use lopdf::{Document, Object, ObjectId};
    use std::collections::HashMap;
    let mut doc = Document::load(path).map_err(|e| e.to_string())?;
    let mut first: HashMap<(String, Vec<u8>), ObjectId> = HashMap::new();
    let mut replace: HashMap<ObjectId, ObjectId> = HashMap::new();
    for (id, obj) in &doc.objects {
        if let Object::Stream(stream) = obj {
            let key = (format!("{:?}", stream.dict), stream.content.clone());
            match first.get(&key) {
                Some(keep) => {
                    replace.insert(*id, *keep);
                }
                None => {
                    first.insert(key, *id);
                }
            }
        }
    }
    if replace.is_empty() {
        return Ok(());
    }
    fn fix(obj: &mut Object, map: &HashMap<ObjectId, ObjectId>) {
        match obj {
            Object::Reference(r) => {
                if let Some(n) = map.get(r) {
                    *r = *n;
                }
            }
            Object::Array(items) => items.iter_mut().for_each(|o| fix(o, map)),
            Object::Dictionary(d) => d.iter_mut().for_each(|(_, o)| fix(o, map)),
            Object::Stream(s) => s.dict.iter_mut().for_each(|(_, o)| fix(o, map)),
            _ => {}
        }
    }
    for obj in doc.objects.values_mut() {
        fix(obj, &replace);
    }
    doc.trailer.iter_mut().for_each(|(_, o)| fix(o, &replace));
    for id in replace.keys() {
        doc.objects.remove(id);
    }
    doc.save(path).map_err(|e| e.to_string())?;
    Ok(())
}

// ----- images ------------------------------------------------------------------

/// A temporary file for an intermediate PDF.
fn temp_pdf() -> PathBuf {
    static COUNT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    let n = COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    std::env::temp_dir().join(format!("quicknotes-export-{}-{n}.pdf", std::process::id()))
}

/// Prints `html` to a PDF and returns its content.
fn pdf_bytes(html: String, base_uri: Option<String>, page: Page, done: impl FnOnce(Result<Vec<u8>, String>) + 'static) {
    let path = temp_pdf();
    let target = path.clone();
    pdf(html, base_uri, path, page, move |result| match result {
        Ok(()) => read_when_written(target, 0, Box::new(done)),
        Err(e) => done(Err(e)),
    });
}

/// Reads the PDF once the printer has finished writing it: the print
/// operation reports success before the file is complete.
fn read_when_written(path: PathBuf, tries: u32, done: Box<dyn FnOnce(Result<Vec<u8>, String>)>) {
    if let Ok(data) = std::fs::read(&path) {
        let tail = &data[data.len().saturating_sub(64)..];
        if tail.windows(5).any(|w| w == b"%%EOF") {
            let _ = std::fs::remove_file(&path);
            return done(Ok(data));
        }
    }
    if tries >= 100 {
        let _ = std::fs::remove_file(&path);
        return done(Err(t!("PDF export timed out").into()));
    }
    glib::timeout_add_local_once(Duration::from_millis(100), move || read_when_written(path, tries + 1, done));
}

/// Renders `html` as images, one per printed page. With [`Page::Tall`] the
/// document becomes one image, cut below its content.
pub fn images(
    html: String,
    base_uri: Option<String>,
    page: Page,
    format: crate::images::ImageFormat,
    done: impl FnOnce(Result<Vec<Vec<u8>>, String>) + 'static,
) {
    images_try(html, base_uri, page, format, 0, Box::new(done));
}

type ImagesDone = Box<dyn FnOnce(Result<Vec<Vec<u8>>, String>)>;

fn images_try(html: String, base_uri: Option<String>, page: Page, format: crate::images::ImageFormat, attempt: u32, done: ImagesDone) {
    let retry = (html.clone(), base_uri.clone());
    pdf_bytes(html, base_uri, page, move |result| {
        let pdf = match result {
            Ok(pdf) => pdf,
            Err(e) => return done(Err(e)),
        };
        let tall = matches!(page, Page::Tall { .. });
        glib::spawn_future_local(async move {
            let converted = gio::spawn_blocking(move || {
                // A tall page that was too short continues on a second page:
                // print again with more room.
                if tall && attempt < 3 && crate::images::page_count(&pdf) > 1 {
                    return None;
                }
                Some(crate::images::from_pdf(pdf, format, tall))
            })
            .await
            .unwrap_or_else(|_| Some(Err("image conversion failed".into())));
            match converted {
                Some(result) => done(result),
                None => {
                    let factor = match page {
                        Page::Tall { factor } => factor * 2.0,
                        other => return done(Err(format!("unexpected page {other:?}"))),
                    };
                    images_try(retry.0, retry.1, Page::Tall { factor }, format, attempt + 1, done);
                }
            }
        });
    });
}

// ----- reading the rendered page ------------------------------------------------

type ScriptDone = Box<dyn FnOnce(Result<String, String>)>;

/// Loads `html` in an off-screen view and runs `script` on the rendered
/// page (after formulas, tunes and charts are drawn); `done` gets the
/// string the script returns.
pub fn run_script(html: String, base_uri: Option<String>, script: String, done: impl FnOnce(Result<String, String>) + 'static) {
    let webview = webkit6::WebView::new();
    let keep: Rc<RefCell<Option<webkit6::WebView>>> = Rc::new(RefCell::new(Some(webview.clone())));
    let done: Rc<RefCell<Option<ScriptDone>>> = Rc::new(RefCell::new(Some(Box::new(done))));
    let finish = {
        let (done, keep) = (done.clone(), keep.clone());
        move |result: Result<String, String>| {
            keep.borrow_mut().take();
            if let Some(f) = done.borrow_mut().take() {
                f(result);
            }
        }
    };
    let finish = Rc::new(finish);
    let on_load = finish.clone();
    webview.connect_load_changed(move |wv, event| {
        if event != webkit6::LoadEvent::Finished {
            return;
        }
        let finish = on_load.clone();
        wv.evaluate_javascript(&script, None, None, None::<&gio::Cancellable>, move |result| {
            finish(result.map(|v| v.to_str().to_string()).map_err(|e| e.message().to_owned()));
        });
    });
    webview.load_html(&html, base_uri.as_deref());
    glib::timeout_add_local_once(TIMEOUT, move || finish(Err(t!("Export timed out").into())));
}

#[cfg(test)]
mod demo {
    use super::*;
    use crate::images::ImageFormat;

    /// Exports a sample document as images and EPUB into `$QN_EXPORT_DEMO`
    /// (manual check; needs a display, run with `--test-threads=1`).
    #[test]
    #[ignore]
    fn export_demo() {
        let Ok(dir) = std::env::var("QN_EXPORT_DEMO") else { return };
        let dir = PathBuf::from(dir);
        gtk::init().unwrap();
        let mut md = String::from("---\ntitle: Export demo\nauthor: QuickNotes\n---\n# Chapter one\n\nSome *text* with a formula $e^{i\\pi}+1=0$ and a [link](#chapter-two).\n\n![pic](pic.png)\n\n| Month | Revenue |\n|---|---|\n| Jan | 1200 |\n| Feb | 1500 |\n\n```chart\ntype: bar\ntitle: Revenue\n```\n\n## Section 1.1\n\n");
        for i in 0..40 {
            md.push_str(&format!("Paragraph {i}: lorem ipsum dolor sit amet, consectetur adipiscing elit.\n\n"));
        }
        md.push_str("# Chapter two {#chapter-two}\n\n$$\\int_0^1 x^2\\,dx = \\tfrac13$$\n\n```abc\nX:1\nK:C\nCDEF GABc|\n```\n");
        std::fs::write(dir.join("pic.png"), crate::images::tests_png()).unwrap();
        let body = crate::markdown::to_html(&md);
        let html = crate::html::standalone("Export demo", &body);
        let base = Some(gio::File::for_path(&dir).uri().to_string() + "/");
        let jobs: Rc<std::cell::Cell<u32>> = Rc::new(std::cell::Cell::new(0));
        let main = glib::MainLoop::new(None, false);
        let finished = {
            let (jobs, main) = (jobs.clone(), main.clone());
            move || {
                jobs.set(jobs.get() - 1);
                if jobs.get() == 0 {
                    main.quit();
                }
            }
        };
        let finished = Rc::new(finished);
        for (name, page, format) in [
            ("tall", Page::Tall { factor: 1.3 }, ImageFormat::Png),
            ("tall", Page::Tall { factor: 1.3 }, ImageFormat::Svg),
            ("tall", Page::Tall { factor: 0.3 }, ImageFormat::Jpeg),
            ("page", Page::A4, ImageFormat::Png),
        ] {
            jobs.set(jobs.get() + 1);
            let (dir, finished) = (dir.clone(), finished.clone());
            images(html.clone(), base.clone(), page, format, move |result| {
                let images = result.expect("images");
                for (i, img) in images.iter().enumerate() {
                    std::fs::write(dir.join(format!("{name}-{i}.{}", format.ext())), img).unwrap();
                }
                eprintln!("{name} {:?}: {} image(s)", format, images.len());
                finished();
            });
        }
        jobs.set(jobs.get() + 1);
        let meta = crate::epub::Meta::from_source(&md, "Export demo");
        let (out, finished2, base_dir) = (dir.clone(), finished.clone(), dir.clone());
        run_script(html, base, crate::epub::EXTRACT_JS.to_owned(), move |json| {
            let extract: crate::epub::Extract = serde_json::from_str(&json.expect("script")).expect("json");
            eprintln!("epub: {} chapters, {} images, math {}", extract.chapters.len(), extract.images.len(), extract.math);
            let epub = crate::epub::build(&meta, extract, Some(&base_dir), &crate::html::ebook_css(), crate::notation::katex_css()).unwrap();
            std::fs::write(out.join("demo.epub"), epub).unwrap();
            finished2();
        });
        glib::timeout_add_local_once(Duration::from_secs(120), {
            let main = main.clone();
            move || main.quit()
        });
        main.run();
        assert_eq!(jobs.get(), 0, "exports did not finish");
    }
}
