//! Canvases in the window: creating them, images from the clipboard, drops
//! and files, copying items between canvases, `.qncanvas` files, export and
//! the clean-up of unused images (see `canvas.rs`).

use super::*;
use crate::canvas;
use serde_json::Value;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum CanvasExport {
    Svg,
    Pdf,
    Png,
}

/// What the page's `QC.exportSvg()` returns.
#[derive(serde::Deserialize)]
struct Drawn {
    svg: String,
    width: f64,
    height: f64,
    background: String,
    /// Stencil packages of exported Pencil shapes (for their fonts).
    #[serde(default)]
    pkgs: Vec<String>,
}

/// A point in the canvas page (client pixels), `None` for the view's centre.
type Point = Option<(f64, f64)>;

fn js_point(point: Point) -> String {
    match point {
        Some((x, y)) => format!("{x}, {y}"),
        None => "null, null".to_owned(),
    }
}

/// Gives the page the fonts of a stencil package.
fn send_fonts(t: &EditorTab, pkg: &str) {
    if let Some(css) = crate::pencil::font_css(pkg).filter(|c| !c.is_empty()) {
        t.canvas_js(&format!("QC.pencilFonts({}, {})", Value::from(pkg), Value::from(css)));
    }
}

fn str_of(msg: &Value, key: &str) -> String {
    msg.get(key).and_then(Value::as_str).unwrap_or_default().to_owned()
}

thread_local! {
    /// Listings of installed collections by package name (parsing is not free).
    static LISTINGS: std::cell::RefCell<HashMap<String, crate::library_art::Listing>> = std::cell::RefCell::new(HashMap::new());
    /// The last loaded stencil repository.
    static REPO: std::cell::RefCell<Vec<crate::library_art::RepoItem>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// What the canvas pages show of the art library.
fn library_json() -> Value {
    let collections: Vec<Value> = crate::library_art::installed()
        .iter()
        .filter_map(|m| {
            let listing = LISTINGS.with(|l| l.borrow().get(&m.pkg).cloned()).or_else(|| {
                let listing = crate::library_art::listing(m)?;
                LISTINGS.with(|l| l.borrow_mut().insert(m.pkg.clone(), listing.clone()));
                Some(listing)
            })?;
            serde_json::to_value(listing).ok()
        })
        .collect();
    let clipart: Vec<Value> = crate::library_art::clipart()
        .iter()
        .map(|c| {
            serde_json::json!({
                "id": c.id,
                "title": c.title,
                "src": c.name,
                "url": format!("{}://clipart/{}", crate::library_art::SCHEME, c.name),
            })
        })
        .collect();
    serde_json::json!({ "collections": collections, "clipart": clipart })
}

/// An image file the canvas can show: its bytes and extension.
fn read_image(path: &Path) -> Result<(Vec<u8>, &'static str), String> {
    let size = std::fs::metadata(path).map_err(|e| e.to_string())?.len();
    if size as usize > canvas::MAX_IMAGE {
        return Err(t!("{} is larger than 25 MB", path.display()));
    }
    let data = std::fs::read(path).map_err(|e| e.to_string())?;
    let ext = canvas::sniff(&data).ok_or_else(|| t!("{} is not a PNG, JPEG, GIF, WebP or SVG image", path.display()))?;
    Ok((data, ext))
}

fn is_image_file(path: &Path) -> bool {
    let ext = path.extension().map(|e| e.to_string_lossy().to_ascii_lowercase()).unwrap_or_default();
    matches!(ext.as_str(), "png" | "jpg" | "jpeg" | "gif" | "webp" | "svg")
}

impl Window {
    pub(super) fn new_canvas(self: &Rc<Self>) {
        let t = self.new_note();
        t.language_locked.set(true);
        t.set_text_irreversible(&canvas::template());
        t.set_language_id(Some(canvas::LANGUAGE));
        t.set_preview(false);
        self.mark_dirty(&t.id);
        self.refresh_title(&t.id);
        self.sync_ui();
    }

    /// Handles the `canvas-…` messages of a tab's canvas page.
    pub(super) fn connect_canvas(self: &Rc<Self>, t: &Rc<EditorTab>) {
        let weak = self.weak();
        *t.canvas_hook.borrow_mut() = Some(Box::new(move |t, msg| {
            let Some(this) = weak.upgrade() else { return };
            let point = match (msg.get("x").and_then(Value::as_f64), msg.get("y").and_then(Value::as_f64)) {
                (Some(x), Some(y)) => Some((x, y)),
                _ => None,
            };
            match msg.get("type").and_then(Value::as_str) {
                Some("canvas-paste") => this.canvas_paste(t, point),
                Some("canvas-copy") => {
                    if let Some(payload) = msg.get("payload") {
                        this.win.clipboard().set_text(&format!("{}{}", canvas::CLIPBOARD_PREFIX, payload));
                    }
                }
                Some("canvas-image") => this.canvas_choose_images(t),
                Some("canvas-library") => this.canvas_send_library(t),
                Some("canvas-clipart-search") => {
                    let q = str_of(msg, "q");
                    let page = msg.get("page").and_then(Value::as_u64).unwrap_or(1).max(1) as u32;
                    this.clipart_search(t, q, page);
                }
                Some("canvas-clipart-add") => {
                    let place = msg.get("place").and_then(Value::as_bool).unwrap_or(false);
                    this.clipart_add(t, str_of(msg, "id"), str_of(msg, "title"), place.then_some(point));
                }
                Some("canvas-clipart-place") => {
                    let src = str_of(msg, "src");
                    match crate::library_art::clipart_file(&src) {
                        Some(data) => this.canvas_add_images(t, vec![(data, "svg")], point),
                        None => this.toast(t!("The graphic is missing")),
                    }
                }
                Some("canvas-clipart-remove") => {
                    crate::library_art::remove_clipart(&str_of(msg, "id"));
                    this.refresh_library();
                }
                Some("canvas-icon") => {
                    let data = crate::library_art::package_file(&str_of(msg, "coll"), &str_of(msg, "path"));
                    match data.and_then(|d| canvas::sniff(&d).map(|ext| (d, ext))) {
                        Some(image) => this.canvas_add_images(t, vec![image], point),
                        None => this.toast(t!("The graphic is missing")),
                    }
                }
                Some("canvas-pencil-place") => this.pencil_place(t, str_of(msg, "coll"), str_of(msg, "shape"), point),
                Some("canvas-pencil-render") => this.pencil_call(t, msg, "render"),
                Some("canvas-pencil-action") => this.pencil_call(t, msg, "action"),
                Some("canvas-stencil-import") => this.stencil_import(),
                Some("canvas-stencil-browse") => this.stencil_browse(t),
                Some("canvas-stencil-install") => this.stencil_install(t, str_of(msg, "id")),
                Some("canvas-stencil-remove") => {
                    crate::library_art::remove(&str_of(msg, "id"));
                    this.refresh_library();
                    this.send_repo(t);
                }
                _ => {}
            }
        }));
    }

    /// Stores images in the canvas and places them.
    fn canvas_add_images(self: &Rc<Self>, t: &Rc<EditorTab>, images: Vec<(Vec<u8>, &str)>, point: Point) {
        let mut names = Vec::new();
        for (data, ext) in images {
            if data.len() > canvas::MAX_IMAGE {
                self.toast(t!("Images larger than 25 MB are not added"));
                continue;
            }
            match store::add_asset(&t.id, &data, ext) {
                Ok(name) => names.push(serde_json::json!({ "src": name })),
                Err(e) => self.toast(&t!("Cannot store the image: {}", e)),
            }
        }
        if !names.is_empty() {
            t.canvas_js(&format!("QC.addImages({}, {})", js_point(point), Value::from(names)));
        }
    }

    /// Adds image files to a canvas; other files are opened as tabs.
    fn canvas_add_files(self: &Rc<Self>, t: &Rc<EditorTab>, paths: Vec<PathBuf>, point: Point) {
        let mut images = Vec::new();
        for path in paths {
            if !is_image_file(&path) {
                self.open_path(path);
                continue;
            }
            match read_image(&path) {
                Ok(image) => images.push(image),
                Err(e) => self.toast(&e),
            }
        }
        self.canvas_add_images(t, images, point);
    }

    /// Ctrl+V in a canvas: files, a picture, an SVG, copied canvas items or text.
    fn canvas_paste(self: &Rc<Self>, t: &Rc<EditorTab>, point: Point) {
        let clipboard = self.win.clipboard();
        let formats = clipboard.formats();
        let (this, t) = (self.clone(), t.clone());
        glib::spawn_future_local(async move {
            if formats.contains_type(gdk::FileList::static_type()) {
                if let Ok(value) = clipboard.read_value_future(gdk::FileList::static_type(), glib::Priority::DEFAULT).await {
                    if let Ok(list) = value.get::<gdk::FileList>() {
                        let paths: Vec<PathBuf> = list.files().iter().filter_map(|f| f.path()).collect();
                        if !paths.is_empty() {
                            this.canvas_add_files(&t, paths, point);
                            return;
                        }
                    }
                }
            }
            if formats.contains_type(gdk::Texture::static_type()) {
                if let Ok(Some(texture)) = clipboard.read_texture_future().await {
                    let png = texture.save_to_png_bytes().to_vec();
                    this.canvas_add_images(&t, vec![(png, "png")], point);
                    return;
                }
            }
            if formats.contain_mime_type("image/svg+xml") {
                if let Ok((stream, _)) = clipboard.read_future(&["image/svg+xml"], glib::Priority::DEFAULT).await {
                    let mut data = Vec::new();
                    while let Ok(chunk) = stream.read_bytes_future(64 * 1024, glib::Priority::DEFAULT).await {
                        if chunk.is_empty() || data.len() > canvas::MAX_IMAGE {
                            break;
                        }
                        data.extend_from_slice(&chunk);
                    }
                    if canvas::sniff(&data) == Some("svg") {
                        this.canvas_add_images(&t, vec![(data, "svg")], point);
                        return;
                    }
                }
            }
            let Ok(Some(text)) = clipboard.read_text_future().await else { return };
            match text.strip_prefix(canvas::CLIPBOARD_PREFIX).and_then(|json| serde_json::from_str::<Value>(json).ok()) {
                Some(payload) => this.canvas_paste_items(&t, payload, point),
                None if !text.trim().is_empty() => {
                    t.canvas_js(&format!("QC.addText({}, {})", js_point(point), Value::from(text.as_str())));
                }
                None => {}
            }
        });
    }

    /// Pastes copied items; images of another canvas are copied along.
    fn canvas_paste_items(self: &Rc<Self>, t: &Rc<EditorTab>, payload: Value, point: Point) {
        let from = payload.get("from").and_then(Value::as_str).unwrap_or_default().to_owned();
        if !from.is_empty() && from != t.id {
            for name in canvas::payload_assets(&payload) {
                if let Some(data) = store::read_asset(&from, &name) {
                    if let Err(e) = store::write_asset(&t.id, &name, &data) {
                        self.toast(&t!("Cannot store the image: {}", e));
                    }
                }
            }
        }
        t.canvas_js(&format!("QC.pasteItems({}, {})", js_point(point), payload));
    }

    /// *Image…* in the canvas toolbar.
    fn canvas_choose_images(self: &Rc<Self>, t: &Rc<EditorTab>) {
        let filter = gtk::FileFilter::new();
        filter.set_name(Some(t!("Images")));
        for (_, mime) in canvas::IMAGE_TYPES {
            filter.add_mime_type(mime);
        }
        let filters = gio::ListStore::new::<gtk::FileFilter>();
        filters.append(&filter);
        let dialog = gtk::FileDialog::builder().title(t!("Add Images")).modal(true).filters(&filters).build();
        let (this, t) = (self.clone(), t.clone());
        glib::spawn_future_local(async move {
            let Ok(files) = dialog.open_multiple_future(Some(&this.win)).await else { return };
            let paths: Vec<PathBuf> =
                (0..files.n_items()).filter_map(|i| files.item(i).and_downcast::<gio::File>()).filter_map(|f| f.path()).collect();
            this.canvas_add_files(&t, paths, None);
        });
    }

    /// Something was dropped on a canvas: image files or image data.
    pub(super) async fn canvas_drop(self: &Rc<Self>, drop: &gdk::Drop, t: &Rc<EditorTab>, x: f64, y: f64) -> bool {
        let xid = self.x11_window_id();
        let formats = drop.formats();
        if formats.contain_mime_type(URI_LIST) {
            if let Some(list) = read_drop_data(drop, xid, URI_LIST).await {
                let paths = parse_uri_list(&list);
                if !paths.is_empty() {
                    self.canvas_add_files(t, paths, Some((x, y)));
                    return true;
                }
            }
        }
        for (ext, mime) in canvas::IMAGE_TYPES {
            if !formats.contain_mime_type(mime) {
                continue;
            }
            if let Some(data) = read_drop_bytes(drop, xid, mime).await.filter(|d| !d.is_empty()) {
                let ext = canvas::sniff(&data).unwrap_or(ext);
                self.canvas_add_images(t, vec![(data, ext)], Some((x, y)));
                return true;
            }
        }
        self.toast(t!("Could not read the dropped files"));
        false
    }

    // ----- art library -------------------------------------------------------

    fn canvas_send_library(&self, t: &EditorTab) {
        t.canvas_js(&format!("QC.library({})", library_json()));
    }

    /// Sends the library to every open canvas.
    pub(super) fn refresh_library(&self) {
        let json = library_json();
        for entry in self.tabs.borrow().values() {
            if let Tab::Text(t) = &entry.tab {
                if t.is_canvas() {
                    t.canvas_js(&format!("QC.library({json})"));
                }
            }
        }
    }

    fn clipart_search(self: &Rc<Self>, t: &Rc<EditorTab>, q: String, page: u32) {
        let t = t.clone();
        t.canvas_js("QC.busy('clipart', true)");
        glib::spawn_future_local(async move {
            let query = q.clone();
            let result = gio::spawn_blocking(move || crate::clipart::search(&query, page)).await.unwrap_or_else(|_| Err("failed".into()));
            let reply = match result {
                Ok(p) => serde_json::json!({ "q": q, "page": page, "items": p.items, "more": p.more }),
                Err(e) => serde_json::json!({ "q": q, "page": page, "error": t!("OpenClipart cannot be reached: {}", e) }),
            };
            t.canvas_js("QC.busy('clipart', false)");
            t.canvas_js(&format!("QC.clipartResults({reply})"));
        });
    }

    /// Saves a graphic of OpenClipart in the collection, and puts it on the
    /// canvas with `place`.
    fn clipart_add(self: &Rc<Self>, t: &Rc<EditorTab>, id: String, title: String, place: Option<Point>) {
        let (this, t) = (self.clone(), t.clone());
        t.canvas_js("QC.busy('clipart', true)");
        glib::spawn_future_local(async move {
            let oc = id.clone();
            let result = gio::spawn_blocking(move || crate::clipart::download(&oc)).await.unwrap_or_else(|_| Err("failed".into()));
            t.canvas_js("QC.busy('clipart', false)");
            match result.and_then(|svg| crate::library_art::add_clipart(&id, &title, &svg).map(|_| svg)) {
                Ok(svg) => {
                    if let Some(point) = place {
                        this.canvas_add_images(&t, vec![(svg, "svg")], point);
                    }
                    this.refresh_library();
                }
                Err(e) => this.toast(&t!("Cannot get the graphic: {}", e)),
            }
        });
    }

    fn stencil_import(self: &Rc<Self>) {
        let filter = gtk::FileFilter::new();
        filter.set_name(Some(t!("Pencil stencil collections")));
        filter.add_pattern("*.zip");
        filter.add_pattern("*.epz");
        let filters = gio::ListStore::new::<gtk::FileFilter>();
        filters.append(&filter);
        let dialog = gtk::FileDialog::builder().title(t!("Import Pencil Stencils")).modal(true).filters(&filters).build();
        let this = self.clone();
        glib::spawn_future_local(async move {
            let Ok(file) = dialog.open_future(Some(&this.win)).await else { return };
            let Some(path) = file.path() else { return };
            let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
            let result = match std::fs::metadata(&path) {
                Ok(m) if m.len() as usize > crate::library_art::MAX_PACKAGE => Err(t!("The stencil collection is larger than 60 MB").to_owned()),
                _ => std::fs::read(&path).map_err(|e| e.to_string()).and_then(|data| crate::library_art::install(&data, &name, "")),
            };
            match result {
                Ok(meta) => {
                    this.toast(&t!("Installed “{}”", meta.name));
                    this.refresh_library();
                }
                Err(e) => this.toast(&t!("Cannot import {}: {}", name, e)),
            }
        });
    }

    fn send_repo(&self, t: &EditorTab) {
        let mut items = REPO.with(|r| r.borrow().clone());
        if items.is_empty() {
            return;
        }
        crate::library_art::mark_installed(&mut items);
        t.canvas_js(&format!("QC.stencilRepo({})", serde_json::json!({ "items": items })));
    }

    fn stencil_browse(self: &Rc<Self>, t: &Rc<EditorTab>) {
        let t = t.clone();
        t.canvas_js("QC.busy('repo', true)");
        glib::spawn_future_local(async move {
            let result = gio::spawn_blocking(crate::library_art::repository).await.unwrap_or_else(|_| Err("failed".into()));
            t.canvas_js("QC.busy('repo', false)");
            match result {
                Ok(items) => {
                    REPO.with(|r| *r.borrow_mut() = items);
                    let mut items = REPO.with(|r| r.borrow().clone());
                    crate::library_art::mark_installed(&mut items);
                    t.canvas_js(&format!("QC.stencilRepo({})", serde_json::json!({ "items": items })));
                }
                Err(e) => {
                    let error = t!("The stencil repository cannot be reached: {}", e);
                    t.canvas_js(&format!("QC.stencilRepo({})", serde_json::json!({ "error": error })));
                }
            }
        });
    }

    fn stencil_install(self: &Rc<Self>, t: &Rc<EditorTab>, id: String) {
        let Some(item) = REPO.with(|r| r.borrow().iter().find(|i| i.id == id).cloned()) else { return };
        let (this, t) = (self.clone(), t.clone());
        t.canvas_js("QC.busy('install', true)");
        glib::spawn_future_local(async move {
            let url = item.url.clone();
            let result = gio::spawn_blocking(move || crate::library_art::download(&url)).await.unwrap_or_else(|_| Err("failed".into()));
            let result = result.and_then(|data| crate::library_art::install(&data, &item.url, &item.version));
            t.canvas_js("QC.busy('install', false)");
            match result {
                Ok(meta) => this.toast(&t!("Installed “{}”", meta.name)),
                Err(e) => this.toast(&t!("Cannot install “{}”: {}", item.name, e)),
            }
            this.refresh_library();
            this.send_repo(&t);
        });
    }

    // ----- live Pencil shapes ---------------------------------------------------

    /// Puts a shape of an installed collection on the canvas: the package is
    /// copied into the canvas (so it syncs with it) and the shape rendered.
    fn pencil_place(self: &Rc<Self>, t: &Rc<EditorTab>, coll: String, shape: String, point: Point) {
        let Some(meta) = crate::library_art::installed().into_iter().find(|m| m.id == coll) else {
            self.toast(t!("The stencil collection of this shape is missing"));
            return;
        };
        let data = crate::library_art::package_data(&meta.id);
        let stored = data.map(|d| store::write_asset(&t.id, &meta.pkg, &d));
        if !matches!(stored, Some(Ok(()))) {
            self.toast(t!("The stencil collection of this shape is missing"));
            return;
        }
        let (this, t) = (self.clone(), t.clone());
        let req = serde_json::json!({ "s": shape });
        let pkg = meta.pkg.clone();
        crate::pencil::request(&meta.pkg, crate::pencil::Source::Canvas(t.id.clone()), "render", req, move |result| {
            match result.and_then(|r| if r["ok"] == true { Ok(r) } else { Err(r["error"].as_str().unwrap_or("?").to_owned()) }) {
                Ok(r) => {
                    send_fonts(&t, &pkg);
                    let (x, y) = match point {
                        Some((x, y)) => (Value::from(x), Value::from(y)),
                        None => (Value::Null, Value::Null),
                    };
                    let item = serde_json::json!({
                        "x": x, "y": y, "pkg": pkg, "c": coll, "s": shape,
                        "w": r["w"], "h": r["h"], "p": r["props"], "render": r,
                    });
                    t.canvas_js(&format!("QC.addPencil({item})"));
                }
                Err(e) => this.toast(&t!("The shape cannot be drawn: {}", e)),
            }
        });
    }

    /// `canvas-pencil-render` / `canvas-pencil-action` from the page.
    fn pencil_call(self: &Rc<Self>, t: &Rc<EditorTab>, msg: &Value, func: &'static str) {
        let pkg = str_of(msg, "pkg");
        let id = str_of(msg, "id");
        let key = msg.get("key").cloned().unwrap_or(Value::Null);
        let fonts = msg.get("fonts").and_then(Value::as_bool).unwrap_or(false);
        let mut req = serde_json::json!({ "s": msg["s"], "props": msg["p"], "w": msg["w"], "h": msg["h"] });
        for k in ["oldW", "oldH", "action"] {
            if let Some(v) = msg.get(k) {
                req[k] = v.clone();
            }
        }
        let t = t.clone();
        let p = pkg.clone();
        crate::pencil::request(&pkg, crate::pencil::Source::Canvas(t.id.clone()), func, req, move |result| {
            let r = result.unwrap_or_else(|e| serde_json::json!({ "ok": false, "error": e }));
            if !fonts && r["ok"] == true {
                send_fonts(&t, &p);
            }
            let id = Value::from(id.as_str());
            match func {
                "render" => t.canvas_js(&format!("QC.pencilRendered({id}, {key}, {r})")),
                _ => t.canvas_js(&format!("QC.pencilAction({id}, {r})")),
            }
        });
    }

    // ----- files ---------------------------------------------------------

    /// Saves the canvas with its images as one `.qncanvas` file. `then` runs
    /// with whether it was saved.
    pub(super) fn save_canvas_file(self: &Rc<Self>, t: Rc<EditorTab>, then: Option<Box<dyn FnOnce(bool)>>) {
        let stem = file_name_for(&t.title());
        let dialog = gtk::FileDialog::builder()
            .title(t!("Save Canvas As"))
            .modal(true)
            .initial_name(format!("{stem}.{}", canvas::EXTENSION))
            .build();
        let this = self.clone();
        glib::spawn_future_local(async move {
            let saved = match dialog.save_future(Some(&this.win)).await.ok().and_then(|f| f.path()) {
                Some(mut path) => {
                    if path.extension().is_none() {
                        path.set_extension(canvas::EXTENSION);
                    }
                    let scene = t.text();
                    let assets = canvas::assets(&scene)
                        .into_iter()
                        .filter_map(|name| store::read_asset(&t.id, &name).map(|data| (name, data)))
                        .collect();
                    let result = canvas::write_bundle(&canvas::Bundle { scene, assets })
                        .and_then(|data| std::fs::write(&path, data).map_err(|e| e.to_string()));
                    match result {
                        Ok(()) => {
                            this.toast(&t!("Saved {}", path.display()));
                            true
                        }
                        Err(e) => {
                            this.toast(&t!("Save failed: {}", e));
                            false
                        }
                    }
                }
                None => false,
            };
            if let Some(then) = then {
                then(saved);
            }
        });
    }

    /// Opens a `.qncanvas` file as a new canvas.
    pub(super) fn import_canvas(self: &Rc<Self>, path: &Path) {
        let bundle = std::fs::read(path).map_err(|e| e.to_string()).and_then(|data| canvas::read_bundle(&data));
        let bundle = match bundle {
            Ok(b) => b,
            Err(e) => {
                self.toast(&t!("Cannot open {}: {}", path.display(), e));
                return;
            }
        };
        let mut scene = bundle.scene;
        if canvas::title(&scene).is_none() {
            if let (Ok(mut v), Some(stem)) = (serde_json::from_str::<Value>(&scene), path.file_stem()) {
                v["title"] = Value::from(stem.to_string_lossy().as_ref());
                scene = v.to_string();
            }
        }
        if !features::on(Feature::Canvas) {
            self.toast(t!("Switch on Canvases in Preferences → Features to open canvas files"));
            return;
        }
        let t = self.new_note();
        for (name, data) in &bundle.assets {
            if let Err(e) = store::write_asset(&t.id, name, data) {
                self.toast(&t!("Cannot store the image: {}", e));
            }
        }
        t.language_locked.set(true);
        t.set_text_irreversible(&scene);
        t.set_language_id(Some(canvas::LANGUAGE));
        t.set_preview(false);
        self.mark_dirty(&t.id);
        self.refresh_title(&t.id);
        self.sync_ui();
    }

    // ----- export --------------------------------------------------------

    pub(super) fn export_canvas(self: &Rc<Self>, t: &Rc<EditorTab>, format: CanvasExport) {
        let Some(webview) = t.canvas_webview() else {
            self.toast(t!("The canvas is not ready yet"));
            return;
        };
        let ext = match format {
            CanvasExport::Svg => "svg",
            CanvasExport::Pdf => "pdf",
            CanvasExport::Png => "png",
        };
        let dialog = gtk::FileDialog::builder()
            .title(t!("Export as {}", ext.to_uppercase()))
            .modal(true)
            .initial_name(format!("{}.{ext}", file_name_for(&t.title())))
            .build();
        let (this, t) = (self.clone(), t.clone());
        glib::spawn_future_local(async move {
            let Ok(file) = dialog.save_future(Some(&this.win)).await else { return };
            let Some(path) = file.path() else { return };
            let weak = this.weak();
            webview.evaluate_javascript("QC.exportSvg()", None, None, None::<&gio::Cancellable>, move |result| {
                let Some(this) = weak.upgrade() else { return };
                let drawn = result
                    .map(|v| v.to_str().to_string())
                    .map_err(|e| e.message().to_owned())
                    .and_then(|json| serde_json::from_str::<Drawn>(&json).map_err(|e| e.to_string()));
                match drawn {
                    Ok(mut drawn) => {
                        drawn.svg = canvas::inline_images(&drawn.svg, |name| store::read_asset(&t.id, name));
                        // Fonts of Pencil shapes travel inside the file.
                        let css: String = drawn.pkgs.iter().filter_map(|p| crate::pencil::font_css(p)).collect();
                        if !css.is_empty() {
                            if let Some(end) = drawn.svg.find('>') {
                                let style = format!("<defs><style>{}</style></defs>", css.replace("]]>", ""));
                                drawn.svg.insert_str(end + 1, &style);
                            }
                        }
                        this.write_canvas_export(drawn, format, path);
                    }
                    Err(e) => this.export_done(&path, Err(e)),
                }
            });
        });
    }

    fn write_canvas_export(self: &Rc<Self>, drawn: Drawn, format: CanvasExport, path: PathBuf) {
        if format == CanvasExport::Svg {
            let svg = format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n{}\n", drawn.svg);
            let result = std::fs::write(&path, svg).map_err(|e| e.to_string());
            self.export_done(&path, result);
            return;
        }
        // One page the size of the drawing (CSS pixels at 96 dpi) plus the
        // 10 mm margin of the page; PDF pages end at 5 m.
        let mm = 25.4 / 96.0;
        let scale = (4800.0 / (drawn.width * mm).max(drawn.height * mm)).min(1.0);
        let width_mm = (drawn.width * mm * scale).max(20.0) + 20.0;
        let height_mm = (drawn.height * mm * scale).max(20.0) + 20.0;
        let html = crate::mindmap::pdf_page(&drawn.svg, &drawn.background, width_mm, height_mm);
        let page = crate::export::Page::Custom { width_mm, height_mm };
        self.toast(&t!("Exporting {}…", if format == CanvasExport::Png { "PNG" } else { "PDF" }));
        let weak = self.weak();
        if format == CanvasExport::Png {
            crate::export::images(html, None, page, crate::images::ImageFormat::Png, move |result| {
                let Some(this) = weak.upgrade() else { return };
                let result = result.and_then(|images| {
                    let first = images.into_iter().next().ok_or_else(|| t!("Nothing to export").to_owned())?;
                    std::fs::write(&path, first).map_err(|e| e.to_string())
                });
                this.export_done(&path, result);
            });
            return;
        }
        let target = path.clone();
        crate::export::pdf(html, None, path, page, move |result| {
            if let Some(this) = weak.upgrade() {
                this.export_done(&target, result);
            }
        });
    }

    // ----- closing and clean-up ---------------------------------------------

    /// A canvas with content closes for good without the notes library: ask first.
    pub(super) fn canvas_close_needs_answer(&self, t: &EditorTab) -> bool {
        t.is_canvas() && !t.transient.get() && !features::on(Feature::Folders) && !canvas_is_blank(&t.text())
    }

    pub(super) fn confirm_close_canvas(self: &Rc<Self>, view: adw::TabView, page: adw::TabPage, t: Rc<EditorTab>) {
        let dialog = adw::AlertDialog::new(
            Some(&t!("Keep the canvas “{}”?", t.title())),
            Some(t!("Without note folders, a closed canvas is deleted together with its images. Save it as a file to keep it.")),
        );
        dialog.add_responses(&[("cancel", t!("_Cancel")), ("discard", t!("_Delete")), ("save", t!("_Save As…"))]);
        dialog.set_response_appearance("discard", adw::ResponseAppearance::Destructive);
        dialog.set_response_appearance("save", adw::ResponseAppearance::Suggested);
        dialog.set_default_response(Some("save"));
        dialog.set_close_response("cancel");
        let this = self.clone();
        glib::spawn_future_local(async move {
            match dialog.choose_future(&this.win).await.as_str() {
                "discard" => {
                    this.forget(&Tab::Text(t));
                    view.close_page_finish(&page, true);
                }
                "save" => {
                    let (weak, tab) = (this.weak(), t.clone());
                    this.save_canvas_file(
                        t,
                        Some(Box::new(move |saved| {
                            if saved {
                                if let Some(this) = weak.upgrade() {
                                    this.forget(&Tab::Text(tab));
                                }
                            }
                            view.close_page_finish(&page, saved);
                        })),
                    );
                }
                _ => view.close_page_finish(&page, false),
            }
        });
    }

    /// Closing a canvas: unused images go; blank canvases and canvases
    /// without note folders are deleted (with their images). Returns
    /// whether the tab was dealt with.
    pub(super) fn forget_canvas(self: &Rc<Self>, t: &EditorTab) -> bool {
        let text = t.text();
        let gone = canvas_is_blank(&text) || !features::on(Feature::Folders);
        if !gone {
            store::prune_assets(&t.id, &canvas::assets(&text));
            return false;
        }
        self.tabs.borrow_mut().remove(&t.id);
        self.dirty.borrow_mut().remove(&t.id);
        self.tab_folders.borrow_mut().remove(&t.id);
        self.library.borrow_mut().retain(|m| m.id != t.id);
        store::delete_note(&t.id);
        self.refresh_tree();
        self.schedule_persist();
        true
    }

    /// At start: image folders of deleted canvases go, as do images a canvas
    /// no longer uses.
    pub(super) fn canvas_housekeeping(&self) {
        let open: HashMap<String, String> = self
            .tabs
            .borrow()
            .iter()
            .filter_map(|(id, e)| match &e.tab {
                Tab::Text(t) => Some((id.clone(), t.text())),
                Tab::Image(_) => None,
            })
            .collect();
        let known: HashSet<String> = self.library.borrow().iter().map(|m| m.id.clone()).collect();
        for id in store::asset_owners() {
            let content = match open.get(&id) {
                Some(text) => Some(text.clone()),
                None if known.contains(&id) => store::read_note(&id),
                None => {
                    store::delete_assets(&id);
                    continue;
                }
            };
            // Unreadable notes keep their images.
            if let Some(text) = content.filter(|c| canvas::is_canvas(c)) {
                store::prune_assets(&id, &canvas::assets(&text));
            }
        }
    }
}

/// A canvas without items and title.
fn canvas_is_blank(text: &str) -> bool {
    canvas::stats(text) == 0 && canvas::title(text).is_none() && !text.contains("\"t\":\"conn\"")
}
