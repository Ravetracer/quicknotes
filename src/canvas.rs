//! Canvases: a free-form board with drawings, shapes, sticky notes, emoji,
//! stickers, connectors and images.
//!
//! The document is a compact JSON scene (`{"canvas":1, "items":[…]}`), edited
//! in a web view (`data/canvas/`) like the Kanban board: the page sends the
//! whole scene back after every change and it is written to the buffer.
//!
//! Images are not embedded. Pasted or dropped images are stored as files
//! named after their SHA-256 in `canvas/<note id>/` (see `store.rs`; sealed
//! with the app key while the app password is on) and the scene refers to
//! them by name (`{"t":"image","src":"<sha256>.png"}`). The page loads them
//! through the `qn-canvas://<note id>/<name>` URI scheme. The scene is the
//! only record of which images a canvas uses: files it no longer mentions are
//! removed when the canvas is closed or the app starts.
//!
//! *Save As* writes a self-contained `.qncanvas` file: a zip with
//! `canvas.json` and the images below `assets/`.

use serde_json::Value;
use std::collections::BTreeSet;
use std::io::{Read, Write};

pub const LANGUAGE: &str = "canvas";
/// File extension of canvas bundles.
pub const EXTENSION: &str = "qncanvas";
/// URI scheme of canvas images in the web view.
pub const SCHEME: &str = "qn-canvas";
/// Largest image that is added to a canvas.
pub const MAX_IMAGE: usize = 25 * 1024 * 1024;
/// Prefix of copied canvas items on the clipboard.
pub const CLIPBOARD_PREFIX: &str = "qn-canvas-items:";

const SCENE_FILE: &str = "canvas.json";
const ASSET_DIR: &str = "assets/";

/// Whether the text is a canvas scene.
pub fn is_canvas(text: &str) -> bool {
    let t = text.trim_start();
    if !t.starts_with('{') {
        return false;
    }
    let head: String = t.chars().take(32).filter(|c| !c.is_whitespace()).collect();
    head.starts_with("{\"canvas\":") && serde_json::from_str::<Value>(t).is_ok_and(|v| v.get("canvas").is_some_and(Value::is_number))
}

/// An empty canvas.
pub fn template() -> String {
    r#"{"canvas":1,"title":"","view":{"x":0,"y":0,"z":1},"items":[]}"#.to_owned()
}

/// The title set on the canvas.
pub fn title(text: &str) -> Option<String> {
    let v: Value = serde_json::from_str(text).ok()?;
    let title = v.get("title")?.as_str()?.trim();
    (!title.is_empty()).then(|| title.lines().next().unwrap_or(title).to_owned())
}

/// Number of items (connectors not counted) for the status bar.
pub fn stats(text: &str) -> usize {
    serde_json::from_str::<Value>(text)
        .ok()
        .and_then(|v| v.get("items").and_then(Value::as_array).map(|items| items.iter().filter(|i| kind(i) != "conn").count()))
        .unwrap_or(0)
}

fn kind(item: &Value) -> &str {
    item.get("t").and_then(Value::as_str).unwrap_or("")
}

/// Names of the images a scene refers to.
pub fn assets(text: &str) -> BTreeSet<String> {
    let Ok(v) = serde_json::from_str::<Value>(text) else { return BTreeSet::new() };
    let Some(items) = v.get("items").and_then(Value::as_array) else { return BTreeSet::new() };
    items_assets(items)
}

/// Images (`src`) and Pencil stencil packages (`pkg`) of items.
fn items_assets(items: &[Value]) -> BTreeSet<String> {
    items
        .iter()
        .filter_map(|i| match kind(i) {
            "image" => i.get("src").and_then(Value::as_str),
            "pencil" => i.get("pkg").and_then(Value::as_str),
            _ => None,
        })
        .filter(|n| valid_name(n))
        .map(str::to_owned)
        .collect()
}

/// Image names of copied items (`{"from": id, "items": […]}`).
pub fn payload_assets(payload: &Value) -> BTreeSet<String> {
    payload.get("items").and_then(Value::as_array).map(|i| items_assets(i)).unwrap_or_default()
}

/// Asset file names: 64 hex digits and a known extension (no paths). Besides
/// images, `zip` names Pencil stencil packages.
pub fn valid_name(name: &str) -> bool {
    let Some((hash, ext)) = name.split_once('.') else { return false };
    hash.len() == 64
        && hash.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        && (ext == "zip" || IMAGE_TYPES.iter().any(|(e, _)| *e == ext))
}

/// Canvas images: file extension and MIME type.
pub const IMAGE_TYPES: &[(&str, &str)] =
    &[("png", "image/png"), ("jpg", "image/jpeg"), ("gif", "image/gif"), ("webp", "image/webp"), ("svg", "image/svg+xml")];

pub fn mime_type(name: &str) -> &'static str {
    let ext = name.rsplit('.').next().unwrap_or("");
    if ext == "zip" {
        return "application/zip";
    }
    IMAGE_TYPES.iter().find(|(e, _)| *e == ext).map(|(_, m)| *m).unwrap_or("application/octet-stream")
}

/// The image format of `data` (file extension), from its first bytes.
pub fn sniff(data: &[u8]) -> Option<&'static str> {
    if data.starts_with(b"\x89PNG\r\n\x1a\n") {
        return Some("png");
    }
    if data.starts_with(&[0xff, 0xd8, 0xff]) {
        return Some("jpg");
    }
    if data.starts_with(b"GIF87a") || data.starts_with(b"GIF89a") {
        return Some("gif");
    }
    if data.len() > 12 && &data[..4] == b"RIFF" && &data[8..12] == b"WEBP" {
        return Some("webp");
    }
    let head = String::from_utf8_lossy(&data[..data.len().min(2048)]).to_ascii_lowercase();
    let head = head.trim_start_matches('\u{feff}').trim_start();
    if (head.starts_with("<?xml") || head.starts_with("<svg") || head.starts_with("<!--") || head.starts_with("<!doctype svg"))
        && head.contains("<svg")
    {
        return Some("svg");
    }
    None
}

/// Name of an image file: its SHA-256 and extension.
pub fn asset_name(data: &[u8], ext: &str) -> String {
    use sha2::{Digest, Sha256};
    let hash = Sha256::digest(data);
    let hex: String = hash.iter().map(|b| format!("{b:02x}")).collect();
    format!("{hex}.{ext}")
}

/// Replaces `qn-canvas://…/<name>` references in an exported SVG with data
/// URIs, using `read` to get the image bytes.
pub fn inline_images(svg: &str, mut read: impl FnMut(&str) -> Option<Vec<u8>>) -> String {
    use base64::Engine;
    let prefix = format!("{SCHEME}://");
    let mut out = String::with_capacity(svg.len());
    let mut rest = svg;
    while let Some(i) = rest.find(&prefix) {
        out.push_str(&rest[..i]);
        let after = &rest[i + prefix.len()..];
        let end = after.find(['"', '\'', ')', ' ']).unwrap_or(after.len());
        let url = &after[..end];
        let name = url.rsplit('/').next().unwrap_or("");
        match read(name).filter(|_| valid_name(name)) {
            Some(data) => {
                out.push_str("data:");
                out.push_str(mime_type(name));
                out.push_str(";base64,");
                out.push_str(&base64::engine::general_purpose::STANDARD.encode(data));
            }
            None => {
                out.push_str(&prefix);
                out.push_str(url);
            }
        }
        rest = &after[end..];
    }
    out.push_str(rest);
    out
}

/// The canvas page; its content is set with `QC.update(scene)`.
pub fn page(dark: bool, vars: &str, id: &str) -> String {
    let strings: serde_json::Map<String, Value> = STRINGS
        .iter()
        .map(|(k, v)| ((*k).to_owned(), Value::from(crate::i18n::tr(v))))
        .collect();
    let strings = Value::Object(strings).to_string().replace("</", "<\\/");
    let palette = serde_json::to_string(&crate::kanban::PALETTE).unwrap_or_default();
    let id = Value::from(id).to_string().replace("</", "<\\/");
    let theme = if dark { "dark" } else { "light" };
    format!(
        r#"<!DOCTYPE html>
<html data-theme="{theme}" style="{vars}">
<head>
<meta charset="utf-8">
<style>{CANVAS_CSS}</style>
</head>
<body>
<div id="app"></div>
<script>window.QC_STRINGS = {strings}; window.QC_PALETTE = {palette}; window.QC_ID = {id};</script>
<script>{CANVAS_JS}</script>
</body>
</html>"#
    )
}

const CANVAS_CSS: &str = include_str!("../data/canvas/canvas.css");
const CANVAS_JS: &str = include_str!("../data/canvas/canvas.js");

/// Strings of the canvas page (key, English text; translated in `i18n.rs`).
const STRINGS: &[(&str, &str)] = &[
    ("title", "Untitled canvas"),
    ("hint", "Pick a tool on the left – draw, write, add sticky notes or paste images."),
    ("select", "Select"),
    ("hand", "Hand"),
    ("pen", "Pen"),
    ("highlighter", "Highlighter"),
    ("eraser", "Eraser"),
    ("shapes", "Shapes"),
    ("rect", "Rectangle"),
    ("ellipse", "Ellipse"),
    ("diamond", "Diamond"),
    ("triangle", "Triangle"),
    ("line", "Line"),
    ("arrow", "Arrow"),
    ("connector", "Connector"),
    ("text", "Text"),
    ("sticky", "Sticky note"),
    ("emoji", "Emoji and stickers"),
    ("image", "Image…"),
    ("emojiTab", "Emoji"),
    ("stickers", "Stickers"),
    ("search", "Search"),
    ("noResults", "Nothing found"),
    ("catSmileys", "Smileys"),
    ("catPeople", "People and gestures"),
    ("catNature", "Nature"),
    ("catFood", "Food and drink"),
    ("catActivity", "Activities and travel"),
    ("catObjects", "Objects"),
    ("catSymbols", "Symbols"),
    ("color", "Colour"),
    ("fill", "Fill"),
    ("stickyColor", "Note colour"),
    ("auto", "Automatic"),
    ("none", "None"),
    ("custom", "Custom colour"),
    ("thin", "Thin"),
    ("medium", "Medium"),
    ("thick", "Thick"),
    ("solid", "Solid"),
    ("dashed", "Dashed"),
    ("dotted", "Dotted"),
    ("fontSize", "Font size"),
    ("bold", "Bold"),
    ("alignLeft", "Align left"),
    ("alignCenter", "Centre"),
    ("alignRight", "Align right"),
    ("rounded", "Rounded corners"),
    ("start", "Start"),
    ("end", "End"),
    ("dot", "Dot"),
    ("straight", "Straight"),
    ("elbow", "Elbow"),
    ("curve", "Curved"),
    ("opacity", "Opacity"),
    ("front", "Bring to front"),
    ("back", "Send to back"),
    ("group", "Group"),
    ("ungroup", "Ungroup"),
    ("lock", "Lock"),
    ("unlock", "Unlock"),
    ("delete", "Delete"),
    ("zoomIn", "Zoom in"),
    ("zoomOut", "Zoom out"),
    ("zoomReset", "Zoom to 100 %"),
    ("fit", "Fit to content"),
    ("snap", "Snap to grid"),
    ("frame", "Frame"),
    ("catBasic", "Basic shapes"),
    ("catFrames", "Frames"),
    ("alignLefts", "Align left edges"),
    ("alignCentres", "Align centres"),
    ("alignRights", "Align right edges"),
    ("alignTops", "Align top edges"),
    ("alignMiddles", "Align middles"),
    ("alignBottoms", "Align bottom edges"),
    ("distH", "Distribute horizontally"),
    ("distV", "Distribute vertically"),
    ("sameW", "Same width"),
    ("sameH", "Same height"),
    ("tCentral", "Central topic"),
    ("tIdea", "Idea"),
    ("tStart", "Start"),
    ("tStep", "Step"),
    ("tDecision", "Decision?"),
    ("tEnd", "End"),
    ("tYes", "Yes"),
    ("tNo", "No"),
    ("tTitle", "Title"),
    ("templates", "Start with a template"),
    ("align", "Align and distribute"),
    ("catFlow", "Flowchart"),
    ("catUi", "Wireframe"),
    ("catMobile", "Mobile"),
    ("catMisc", "Shapes and symbols"),
    ("fA4p", "A4 portrait"),
    ("fA4l", "A4 landscape"),
    ("fSlide", "Slide 16:9"),
    ("fDesktop", "Desktop"),
    ("fPhone", "Phone"),
    ("tplBlank", "Blank"),
    ("tplBrainstorm", "Brainstorm"),
    ("tplRetro", "Retrospective"),
    ("tWell", "Went well"),
    ("tImprove", "To improve"),
    ("tActions", "Action items"),
    ("tplFlow", "Flowchart"),
    ("tplWire", "Wireframe"),
    ("tplMobile", "Mobile screen"),
    ("tplSwot", "SWOT analysis"),
    ("tS", "Strengths"),
    ("tW", "Weaknesses"),
    ("tO", "Opportunities"),
    ("tT", "Threats"),
    ("tplWeek", "Weekly plan"),
    ("mon", "Monday"),
    ("tue", "Tuesday"),
    ("wed", "Wednesday"),
    ("thu", "Thursday"),
    ("fri", "Friday"),
    ("sat", "Saturday"),
    ("sun", "Sunday"),
    ("s_process", "Process"),
    ("s_terminator", "Start / end"),
    ("s_decision", "Decision"),
    ("s_data", "Data"),
    ("s_document", "Document"),
    ("s_multidoc", "Documents"),
    ("s_database", "Database"),
    ("s_predefined", "Predefined process"),
    ("s_manual", "Manual input"),
    ("s_preparation", "Preparation"),
    ("s_delay", "Delay"),
    ("s_offpage", "Off-page reference"),
    ("s_onpage", "Connector point"),
    ("s_annotation", "Annotation"),
    ("s_window", "Window"),
    ("s_browser", "Browser"),
    ("s_dialog", "Dialog"),
    ("s_button", "Button"),
    ("s_primary", "Primary button"),
    ("s_textfield", "Text field"),
    ("s_textarea", "Text area"),
    ("s_checkbox", "Checkbox"),
    ("s_radio", "Radio button"),
    ("s_toggle", "Switch"),
    ("s_dropdown", "Dropdown"),
    ("s_slider", "Slider"),
    ("s_progress", "Progress bar"),
    ("s_tabs", "Tabs"),
    ("s_list", "List"),
    ("s_table", "Table"),
    ("s_image", "Image"),
    ("s_avatar", "Avatar"),
    ("s_icon", "Icon"),
    ("s_heading", "Heading"),
    ("s_paragraph", "Paragraph"),
    ("s_link", "Link"),
    ("s_breadcrumb", "Breadcrumb"),
    ("s_pagination", "Pagination"),
    ("s_search", "Search field"),
    ("s_card", "Card"),
    ("s_navbar", "Navigation bar"),
    ("s_sidebar", "Sidebar"),
    ("s_tooltip", "Tooltip"),
    ("s_phone", "Phone"),
    ("s_tablet", "Tablet"),
    ("s_statusbar", "Status bar"),
    ("s_appbar", "App bar"),
    ("s_bottomnav", "Bottom navigation"),
    ("s_fab", "Action button"),
    ("s_listitem", "List item"),
    ("s_speech", "Speech bubble"),
    ("s_thought", "Thought bubble"),
    ("s_callout", "Callout"),
    ("s_cloud", "Cloud"),
    ("s_star", "Star"),
    ("s_heart", "Heart"),
    ("s_brace", "Curly brace"),
    ("s_banner", "Banner"),
    ("s_arrowR", "Arrow right"),
    ("s_arrowL", "Arrow left"),
    ("s_arrowU", "Arrow up"),
    ("s_arrowD", "Arrow down"),
    ("s_arrowLR", "Double arrow"),
    ("st_check", "Check"),
    ("st_cross", "Cross"),
    ("st_star", "Star"),
    ("st_heart", "Heart"),
    ("st_up", "Thumbs up"),
    ("st_down", "Thumbs down"),
    ("st_question", "Question"),
    ("st_bang", "Exclamation"),
    ("st_idea", "Idea"),
    ("st_flag", "Flag"),
    ("st_warning", "Warning"),
    ("st_smile", "Smile"),
    ("st_sad", "Sad"),
    ("st_fire", "Fire"),
    ("st_rocket", "Rocket"),
    ("st_plus1", "Plus one"),
    ("catClipart", "OpenClipArt"),
    ("browseClipart", "Browse OpenClipart…"),
    ("clipartTitle", "OpenClipart"),
    ("clipartSearch", "Search OpenClipart"),
    ("clipartHint", "Search the free clip art of OpenClipart.org."),
    ("clipartCredit", "OpenClipart.org · public domain (CC0)"),
    ("clipartSave", "Save to my collection"),
    ("clipartSaved", "In my collection"),
    ("clipartEmpty", "Nothing saved yet – browse OpenClipart and pick graphics."),
    ("more", "More"),
    ("searching", "Loading…"),
    ("removeItem", "Remove from the collection"),
    ("removeColl", "Remove collection"),
    ("confirmRemove", "Click again to remove"),
    ("showMore", "Show more ({} of {})"),
    ("properties", "Properties"),
    ("noProps", "This shape has no properties."),
    ("pGeneral", "General"),
    ("pActions", "Actions"),
    ("pWidth", "Width"),
    ("pSize", "Size"),
    ("pItalic", "Italic"),
    ("pFontName", "Font"),
    ("pAlpha", "Opacity"),
    ("pImage", "(image)"),
    ("pRendering", "Drawing…"),
    ("pFailed", "This shape cannot be drawn"),
    ("pKnob", "Drag to adjust"),
    ("importStencils", "Import Pencil stencils…"),
    ("getStencils", "Get more stencils…"),
    ("repoTitle", "Pencil stencils"),
    ("repoHint", "Collections from the official Pencil repository."),
    ("install", "Install"),
    ("update", "Update"),
    ("installed", "Installed"),
    ("remove", "Remove"),
    ("back", "Back"),
    ("by", "by {}"),
];

// ===== images in the web view ================================================

/// Serves `qn-canvas://<note id>/<name>` from the canvas' image folder (once
/// per process, on the default web context all previews share).
pub fn register_scheme() {
    use gtk::{gio, glib};
    thread_local! {
        static DONE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    }
    if DONE.with(|d| d.replace(true)) {
        return;
    }
    let Some(context) = webkit6::WebContext::default() else { return };
    if let Some(security) = context.security_manager() {
        for scheme in [SCHEME, crate::library_art::SCHEME] {
            security.register_uri_scheme_as_secure(scheme);
            security.register_uri_scheme_as_cors_enabled(scheme);
        }
    }
    // Files of the art library: `stencils/<collection>/<path>`, `clipart/<name>`.
    context.register_uri_scheme(crate::library_art::SCHEME, |request| {
        let uri = request.uri().map(|u| u.to_string()).unwrap_or_default();
        let rest = uri.strip_prefix(&format!("{}://", crate::library_art::SCHEME)).unwrap_or("");
        let rest = urlencoding::decode(rest.split(['?', '#']).next().unwrap_or("")).map(|r| r.into_owned()).unwrap_or_default();
        let data = match rest.split_once('/') {
            Some(("stencils", path)) => path.split_once('/').and_then(|(id, file)| crate::library_art::package_file(id, file)),
            Some(("clipart", name)) => crate::library_art::clipart_file(name),
            _ => None,
        };
        match data {
            Some(data) => {
                let len = data.len() as i64;
                let stream = gio::MemoryInputStream::from_bytes(&glib::Bytes::from_owned(data));
                request.finish(&stream, len, Some(crate::library_art::mime_type(&rest)));
            }
            None => {
                let mut e = glib::Error::new(gio::IOErrorEnum::NotFound, "not found");
                request.finish_error(&mut e);
            }
        }
    });
    context.register_uri_scheme(SCHEME, |request| {
        let uri = request.uri().map(|u| u.to_string()).unwrap_or_default();
        let rest = uri.strip_prefix(&format!("{SCHEME}://")).unwrap_or("");
        let rest = rest.split(['?', '#']).next().unwrap_or("");
        let data = rest.split_once('/').and_then(|(id, name)| crate::store::read_asset(id, name).map(|d| (d, name.to_owned())));
        match data {
            Some((data, name)) => {
                let len = data.len() as i64;
                let stream = gio::MemoryInputStream::from_bytes(&glib::Bytes::from_owned(data));
                request.finish(&stream, len, Some(mime_type(&name)));
            }
            None => {
                let mut e = glib::Error::new(gio::IOErrorEnum::NotFound, "image not found");
                request.finish_error(&mut e);
            }
        }
    });
}

// ===== bundles ================================================================

/// A `.qncanvas` file: the scene and its images.
pub struct Bundle {
    pub scene: String,
    pub assets: Vec<(String, Vec<u8>)>,
}

pub fn write_bundle(bundle: &Bundle) -> Result<Vec<u8>, String> {
    use zip::write::SimpleFileOptions;
    use zip::CompressionMethod;
    let mut zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    let deflated = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    let stored = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
    let err = |e: zip::result::ZipError| e.to_string();
    zip.start_file(SCENE_FILE, deflated).map_err(err)?;
    zip.write_all(bundle.scene.as_bytes()).map_err(|e| e.to_string())?;
    for (name, data) in &bundle.assets {
        // PNG, JPEG, GIF and WebP are compressed already.
        let options = if name.ends_with(".svg") { deflated } else { stored };
        zip.start_file(format!("{ASSET_DIR}{name}"), options).map_err(err)?;
        zip.write_all(data).map_err(|e| e.to_string())?;
    }
    Ok(zip.finish().map_err(err)?.into_inner())
}

pub fn read_bundle(data: &[u8]) -> Result<Bundle, String> {
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(data)).map_err(|e| e.to_string())?;
    let mut scene = String::new();
    zip.by_name(SCENE_FILE)
        .map_err(|_| t!("Not a canvas file").to_string())?
        .take(64 * 1024 * 1024)
        .read_to_string(&mut scene)
        .map_err(|e| e.to_string())?;
    if !is_canvas(&scene) {
        return Err(t!("Not a canvas file").to_string());
    }
    let used = assets(&scene);
    let mut out = Vec::new();
    for name in used {
        let Ok(file) = zip.by_name(&format!("{ASSET_DIR}{name}")) else { continue };
        let mut data = Vec::new();
        file.take(MAX_IMAGE as u64 + 1).read_to_end(&mut data).map_err(|e| e.to_string())?;
        // Only images whose content matches their name.
        if data.len() <= MAX_IMAGE && asset_name(&data, name.rsplit('.').next().unwrap_or("")) == name {
            out.push((name, data));
        }
    }
    Ok(Bundle { scene, assets: out })
}

#[cfg(test)]
mod tests {
    use super::*;

    const PNG: &[u8] = b"\x89PNG\r\n\x1a\nrest";

    fn scene_with(name: &str) -> String {
        format!(
            r#"{{"canvas":1,"title":"Ideas\nmore","items":[{{"id":"a","t":"image","src":"{name}","x":0,"y":0,"w":10,"h":10}},{{"id":"b","t":"sticky","txt":"x"}},{{"id":"c","t":"conn"}},{{"id":"d","t":"image","src":"../etc/passwd"}}]}}"#
        )
    }

    #[test]
    fn detection() {
        assert!(is_canvas(&template()));
        assert!(is_canvas("  {\n  \"canvas\": 1, \"items\": []}"));
        assert!(!is_canvas("{\"kanban\":1}"));
        assert!(!is_canvas("{\"canvas\":1, broken"));
        assert!(!is_canvas("# canvas"));
    }

    #[test]
    fn titles_stats_and_assets() {
        let name = asset_name(PNG, "png");
        let scene = scene_with(&name);
        assert_eq!(title(&scene).as_deref(), Some("Ideas"));
        assert_eq!(title(&template()), None);
        assert_eq!(stats(&scene), 3);
        assert_eq!(assets(&scene).into_iter().collect::<Vec<_>>(), [name]);
    }

    #[test]
    fn stencil_packages_are_assets() {
        let pkg = asset_name(b"zip", "zip");
        let scene = format!(r#"{{"canvas":1,"items":[{{"id":"p","t":"pencil","pkg":"{pkg}","c":"x","s":"button"}}]}}"#);
        assert_eq!(assets(&scene).into_iter().collect::<Vec<_>>(), [pkg]);
    }

    #[test]
    fn names_are_checked() {
        let name = asset_name(PNG, "png");
        assert!(valid_name(&name));
        assert!(!valid_name(&name.replace(".png", ".exe")));
        assert!(!valid_name("../x.png"));
        assert!(!valid_name(&format!("a{name}")));
        assert_eq!(mime_type(&name), "image/png");
    }

    #[test]
    fn sniffing() {
        assert_eq!(sniff(PNG), Some("png"));
        assert_eq!(sniff(&[0xff, 0xd8, 0xff, 0xe0]), Some("jpg"));
        assert_eq!(sniff(b"GIF89a..."), Some("gif"));
        assert_eq!(sniff(b"RIFF\0\0\0\0WEBPVP8 "), Some("webp"));
        assert_eq!(sniff(b"<?xml version=\"1.0\"?>\n<svg xmlns=\"http://www.w3.org/2000/svg\"/>"), Some("svg"));
        assert_eq!(sniff(b"hello"), None);
    }

    #[test]
    fn images_are_inlined() {
        let name = asset_name(PNG, "png");
        let svg = format!(r#"<svg><image href="qn-canvas://abc/{name}"/><image href="qn-canvas://abc/missing.png"/></svg>"#);
        let out = inline_images(&svg, |n| (n == name).then(|| PNG.to_vec()));
        assert!(out.contains("href=\"data:image/png;base64,iVBORw0KGgpyZXN0\""), "{out}");
        assert!(out.contains("qn-canvas://abc/missing.png"), "{out}");
    }

    #[test]
    fn page_strings_are_translated() {
        for (_, en) in STRINGS {
            assert!(crate::i18n::has_german(en), "missing German translation: {en}");
        }
    }

    #[test]
    fn bundles_round_trip() {
        let name = asset_name(PNG, "png");
        let scene = scene_with(&name);
        let bundle = Bundle { scene: scene.clone(), assets: vec![(name.clone(), PNG.to_vec())] };
        let data = write_bundle(&bundle).unwrap();
        let back = read_bundle(&data).unwrap();
        assert_eq!(back.scene, scene);
        assert_eq!(back.assets, vec![(name, PNG.to_vec())]);
        assert!(read_bundle(b"not a zip").is_err());
    }

    #[test]
    fn bundles_drop_tampered_images() {
        let name = asset_name(PNG, "png");
        let bundle = Bundle { scene: scene_with(&name), assets: vec![(name, b"other".to_vec())] };
        let back = read_bundle(&write_bundle(&bundle).unwrap()).unwrap();
        assert!(back.assets.is_empty());
    }
}
