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

fn items_assets(items: &[Value]) -> BTreeSet<String> {
    items
        .iter()
        .filter(|i| kind(i) == "image")
        .filter_map(|i| i.get("src").and_then(Value::as_str))
        .filter(|n| valid_name(n))
        .map(str::to_owned)
        .collect()
}

/// Image names of copied items (`{"from": id, "items": […]}`).
pub fn payload_assets(payload: &Value) -> BTreeSet<String> {
    payload.get("items").and_then(Value::as_array).map(|i| items_assets(i)).unwrap_or_default()
}

/// Image file names: 64 hex digits and a known extension (no paths).
pub fn valid_name(name: &str) -> bool {
    let Some((hash, ext)) = name.split_once('.') else { return false };
    hash.len() == 64 && hash.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)) && IMAGE_TYPES.iter().any(|(e, _)| *e == ext)
}

/// Canvas images: file extension and MIME type.
pub const IMAGE_TYPES: &[(&str, &str)] =
    &[("png", "image/png"), ("jpg", "image/jpeg"), ("gif", "image/gif"), ("webp", "image/webp"), ("svg", "image/svg+xml")];

pub fn mime_type(name: &str) -> &'static str {
    let ext = name.rsplit('.').next().unwrap_or("");
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
        security.register_uri_scheme_as_secure(SCHEME);
        security.register_uri_scheme_as_cors_enabled(SCHEME);
    }
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
