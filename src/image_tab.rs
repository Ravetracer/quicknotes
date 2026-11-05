//! Read-only image viewer tab with zoom (Ctrl+wheel, buttons).

use crate::store::{TabKind, TabMeta};
use gtk::prelude::*;
use gtk::{gdk, gdk_pixbuf, glib};
use std::cell::Cell;
use std::path::PathBuf;
use std::rc::Rc;

pub const EXTENSIONS: &[&str] = &[
    "png", "jpg", "jpeg", "gif", "webp", "svg", "svgz", "bmp", "ico", "tif", "tiff", "avif", "heic", "tga",
];

pub fn is_image(path: &std::path::Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| EXTENSIONS.contains(&e.to_ascii_lowercase().as_str()))
}

type ZoomAction = Box<dyn Fn(&Rc<ImageTab>)>;

pub struct ImageTab {
    pub id: String,
    pub root: gtk::Overlay,
    pub path: PathBuf,
    picture: gtk::Picture,
    scrolled: gtk::ScrolledWindow,
    zoom_label: gtk::Label,
    /// `None` = fit to window.
    zoom: Cell<Option<f64>>,
    width: i32,
    height: i32,
    is_svg: bool,
    /// Scale at which the current SVG texture was rasterized.
    svg_scale: Cell<f64>,
    scroll_acc: Cell<f64>,
    bytes: u64,
}

impl ImageTab {
    pub fn new(id: String, path: PathBuf) -> Result<Rc<Self>, String> {
        let texture = gdk::Texture::from_filename(&path).map_err(|e| e.message().to_owned())?;
        let is_svg = path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("svg") || e.eq_ignore_ascii_case("svgz"));

        let picture = gtk::Picture::for_paintable(&texture);
        picture.set_can_shrink(true);
        picture.set_content_fit(gtk::ContentFit::ScaleDown);

        let scrolled = gtk::ScrolledWindow::builder()
            .child(&picture)
            .hexpand(true)
            .vexpand(true)
            .build();
        scrolled.add_css_class("image-canvas");

        let root = gtk::Overlay::new();
        root.set_child(Some(&scrolled));
        root.set_widget_name(&id);

        let zoom_label = gtk::Label::new(Some("Fit"));
        zoom_label.set_width_chars(5);

        let tab = Rc::new(Self {
            id,
            root,
            bytes: path.metadata().map(|m| m.len()).unwrap_or(0),
            path,
            picture,
            scrolled,
            zoom_label,
            zoom: Cell::new(None),
            width: texture.width(),
            height: texture.height(),
            is_svg,
            svg_scale: Cell::new(1.0),
            scroll_acc: Cell::new(0.0),
        });
        tab.build_controls();
        Ok(tab)
    }

    fn build_controls(self: &Rc<Self>) {
        let bar = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        bar.add_css_class("osd");
        bar.add_css_class("toolbar");
        bar.add_css_class("image-controls");
        bar.set_halign(gtk::Align::End);
        bar.set_valign(gtk::Align::End);
        bar.set_margin_end(18);
        bar.set_margin_bottom(18);

        let button = |icon: &str, tip: &str, f: ZoomAction| {
            let b = gtk::Button::builder().icon_name(icon).tooltip_text(tip).build();
            let weak = Rc::downgrade(self);
            b.connect_clicked(move |_| {
                if let Some(t) = weak.upgrade() {
                    f(&t)
                }
            });
            b
        };
        bar.append(&button("zoom-out-symbolic", "Zoom out", Box::new(|t| t.step(-1.0))));
        bar.append(&self.zoom_label);
        bar.append(&button("zoom-in-symbolic", "Zoom in", Box::new(|t| t.step(1.0))));
        bar.append(&button("zoom-original-symbolic", "Original size", Box::new(|t| t.set_zoom(Some(1.0)))));
        bar.append(&button("zoom-fit-best-symbolic", "Fit to window", Box::new(|t| t.set_zoom(None))));
        self.root.add_overlay(&bar);

        let scroll = gtk::EventControllerScroll::new(gtk::EventControllerScrollFlags::VERTICAL);
        scroll.set_propagation_phase(gtk::PropagationPhase::Capture);
        let weak = Rc::downgrade(self);
        scroll.connect_scroll(move |c, _, dy| {
            if !c.current_event_state().contains(gdk::ModifierType::CONTROL_MASK) {
                return glib::Propagation::Proceed;
            }
            if let Some(t) = weak.upgrade() {
                let acc = t.scroll_acc.get() + dy;
                let steps = acc.trunc();
                t.scroll_acc.set(acc - steps);
                if steps != 0.0 {
                    t.step(-steps);
                }
            }
            glib::Propagation::Stop
        });
        self.root.add_controller(scroll);
    }

    /// Effective zoom factor, also when in fit mode.
    fn current_zoom(&self) -> f64 {
        self.zoom.get().unwrap_or_else(|| {
            let (w, h) = (self.scrolled.width() as f64, self.scrolled.height() as f64);
            if w <= 0.0 || h <= 0.0 || self.width == 0 || self.height == 0 {
                1.0
            } else {
                (w / self.width as f64).min(h / self.height as f64).min(1.0)
            }
        })
    }

    fn step(&self, steps: f64) {
        let z = self.current_zoom() * 1.15f64.powf(steps);
        self.set_zoom(Some(z));
    }

    pub fn set_zoom(&self, zoom: Option<f64>) {
        let zoom = zoom.map(|z| z.clamp(0.02, 32.0));
        self.zoom.set(zoom);
        match zoom {
            None => {
                self.picture.set_size_request(-1, -1);
                self.picture.set_halign(gtk::Align::Fill);
                self.picture.set_valign(gtk::Align::Fill);
                self.picture.set_content_fit(gtk::ContentFit::ScaleDown);
                self.zoom_label.set_text("Fit");
            }
            Some(z) => {
                let (w, h) = ((self.width as f64 * z) as i32, (self.height as f64 * z) as i32);
                self.picture.set_size_request(w.max(1), h.max(1));
                self.picture.set_halign(gtk::Align::Center);
                self.picture.set_valign(gtk::Align::Center);
                self.picture.set_content_fit(gtk::ContentFit::Fill);
                self.zoom_label.set_text(&format!("{:.0}%", z * 100.0));
                self.rerender_svg(z);
            }
        }
    }

    /// SVGs are rasterized at their natural size; re-render them sharp when zoomed in.
    fn rerender_svg(&self, zoom: f64) {
        if !self.is_svg || zoom <= self.svg_scale.get() {
            return;
        }
        let scale = zoom.min(8.0);
        let (w, h) = ((self.width as f64 * scale) as i32, (self.height as f64 * scale) as i32);
        if let Ok(pixbuf) = gdk_pixbuf::Pixbuf::from_file_at_scale(&self.path, w, h, true) {
            #[allow(deprecated)]
            self.picture.set_paintable(Some(&gdk::Texture::for_pixbuf(&pixbuf)));
            self.svg_scale.set(scale);
        }
    }

    pub fn title(&self) -> String {
        self.path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "Image".into())
    }

    pub fn info(&self) -> String {
        let ext = self
            .path
            .extension()
            .map(|e| e.to_string_lossy().to_uppercase())
            .unwrap_or_default();
        format!("{ext} · {}×{} px · {}", self.width, self.height, human_size(self.bytes))
    }

    pub fn meta(&self) -> TabMeta {
        TabMeta {
            id: self.id.clone(),
            kind: TabKind::Image,
            path: Some(self.path.clone()),
            ..Default::default()
        }
    }
}

pub fn human_size(bytes: u64) -> String {
    const UNITS: [&str; 4] = ["B", "KB", "MB", "GB"];
    let mut v = bytes as f64;
    let mut unit = 0;
    while v >= 1024.0 && unit < UNITS.len() - 1 {
        v /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} B")
    } else {
        format!("{v:.1} {}", UNITS[unit])
    }
}
