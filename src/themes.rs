//! Colour themes: the two built-in defaults (libadwaita / Adwaita style
//! schemes) and third-party themes converted into `data/themes.json` by
//! `tools/themes/convert.py`. A theme provides a GtkSourceView style scheme,
//! libadwaita colour overrides for the window and colours for the preview.

use crate::store;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::sync::OnceLock;

pub const DEFAULT_DARK: &str = "default-dark";
pub const DEFAULT_LIGHT: &str = "default-light";
const SCHEME_PREFIX: &str = "quicknotes-";

#[derive(Deserialize, Default, Clone)]
pub struct Style {
    pub fg: Option<String>,
    #[serde(default)]
    pub bold: bool,
    #[serde(default)]
    pub italic: bool,
    #[serde(default)]
    pub underline: bool,
}

#[derive(Deserialize)]
pub struct Editor {
    pub bg: String,
    pub fg: String,
    pub line: String,
    pub selection: String,
    pub selection_fg: Option<String>,
    pub cursor: String,
    pub gutter_bg: String,
    pub gutter_fg: String,
    pub gutter_active: String,
    #[serde(rename = "match")]
    pub search_match: String,
}

#[derive(Deserialize)]
pub struct Ui {
    pub header: String,
    pub sidebar: String,
    pub accent: String,
    pub link: String,
    pub muted: String,
    pub border: String,
}

#[derive(Deserialize)]
pub struct Theme {
    pub id: String,
    pub name: String,
    pub source: String,
    pub dark: bool,
    /// `None` for the built-in defaults.
    pub editor: Option<Editor>,
    pub ui: Option<Ui>,
    #[serde(default)]
    pub syntax: BTreeMap<String, Style>,
}

/// Where the themes come from, in display order: (key, name, copyright line).
pub const SOURCES: [(&str, &str, &str); 4] = [
    ("quicknotes", "QuickNotes", ""),
    ("dracula", "Dracula", "Copyright (c) 2023 Dracula Theme"),
    ("daylerees", "Dayle Rees Colour Schemes", "Copyright (c) 2014 Dayle Rees"),
    ("ultimate", "VSCode Ultimate Themes Pack", "Copyright (c) 2025 codewithevilxd"),
];

/// Syntax roles and the GtkSourceView styles they are applied to.
const STYLE_MAP: &[(&str, &[&str])] = &[
    ("comment", &["def:comment", "def:doc-comment", "xml:comment"]),
    ("string", &["def:string", "def:character", "xml:attribute-value", "js:string"]),
    ("escape", &["def:special-char", "js:escape"]),
    ("number", &["def:number", "def:decimal", "def:floating-point", "def:base-n-integer", "def:complex"]),
    ("constant", &["def:constant", "def:boolean", "def:special-constant", "js:boolean", "js:null-value"]),
    ("builtin", &["def:builtin"]),
    ("keyword", &["def:keyword", "def:statement", "def:reserved", "css:keyword", "css:at-rule"]),
    ("type", &["def:type"]),
    ("function", &["def:function"]),
    ("variable", &["def:identifier", "sh:variable"]),
    ("operator", &["def:operator"]),
    ("preprocessor", &["def:preprocessor", "def:shebang", "xml:doctype"]),
    ("tag", &["xml:element-name"]),
    ("attribute", &["xml:attribute-name", "css:property-name"]),
    (
        "heading",
        &[
            "def:heading", "def:heading0", "def:heading1", "def:heading2", "def:heading3", "def:heading4",
            "def:heading5", "def:heading6",
        ],
    ),
    ("bold", &["def:strong-emphasis"]),
    ("italic", &["def:emphasis"]),
    ("link", &["def:link-destination", "def:link-text", "def:net-address", "def:underlined"]),
    ("inline_code", &["def:inline-code", "def:preformatted-section"]),
    ("list", &["def:list-marker", "def:link-symbol", "def:thematic-break"]),
    ("inserted", &["def:insertion"]),
    ("deleted", &["def:deletion"]),
    ("invalid", &["def:error", "xml:error"]),
];

static THEMES: OnceLock<Vec<Theme>> = OnceLock::new();

/// All themes: the defaults first, then the bundled ones by source.
pub fn all() -> &'static [Theme] {
    THEMES.get_or_init(|| {
        let builtin = |id: &str, dark| Theme {
            id: id.into(),
            name: String::new(),
            source: "quicknotes".into(),
            dark,
            editor: None,
            ui: None,
            syntax: BTreeMap::new(),
        };
        let mut list = vec![builtin(DEFAULT_DARK, true), builtin(DEFAULT_LIGHT, false)];
        match serde_json::from_str::<Vec<Theme>>(include_str!("../data/themes.json")) {
            Ok(themes) => list.extend(themes),
            Err(e) => eprintln!("quicknotes: cannot read the bundled themes: {e}"),
        }
        list
    })
}

/// The theme `id`, or the default matching `dark` if it does not exist.
pub fn find(id: &str, dark: bool) -> &'static Theme {
    let fallback = if dark { DEFAULT_DARK } else { DEFAULT_LIGHT };
    all()
        .iter()
        .find(|t| t.id == id)
        .or_else(|| all().iter().find(|t| t.id == fallback))
        .expect("built-in themes exist")
}

impl Theme {
    pub fn label(&self) -> &str {
        match self.id.as_str() {
            DEFAULT_DARK => t!("Default Dark"),
            DEFAULT_LIGHT => t!("Default Light"),
            _ => &self.name,
        }
    }

    /// Id of the GtkSourceView style scheme.
    pub fn scheme_id(&self) -> String {
        match self.id.as_str() {
            DEFAULT_DARK => "Adwaita-dark".into(),
            DEFAULT_LIGHT => "Adwaita".into(),
            id => format!("{SCHEME_PREFIX}{id}"),
        }
    }

    /// Background of the editor and preview.
    pub fn background(&self) -> &str {
        match &self.editor {
            Some(e) => &e.bg,
            None if self.dark => "#1e1e1e",
            None => "#ffffff",
        }
    }

    /// Colours for the theme preview: background, border, then four syntax colours.
    pub fn swatch(&self) -> [&str; 6] {
        match (&self.editor, &self.ui) {
            (Some(e), Some(u)) => {
                let fg = |role: &str| self.syntax.get(role).and_then(|s| s.fg.as_deref()).unwrap_or(&e.fg);
                [&e.bg, &u.border, fg("keyword"), fg("function"), fg("string"), fg("comment")]
            }
            _ if self.dark => ["#1e1e1e", "#3a3a3a", "#ffa348", "#62a0ea", "#f8e45c", "#9a9996"],
            _ => ["#ffffff", "#d1d9e0", "#c64600", "#1a5fb4", "#26a269", "#77767b"],
        }
    }

    /// GtkSourceView style scheme (bundled themes only).
    fn scheme_xml(&self) -> Option<String> {
        let e = self.editor.as_ref()?;
        let u = self.ui.as_ref()?;
        let mut x = String::new();
        let _ = writeln!(x, r#"<?xml version="1.0" encoding="UTF-8"?>"#);
        let _ = writeln!(
            x,
            r#"<style-scheme id="{}" name="{}" version="1.0">"#,
            xml_escape(&self.scheme_id()),
            xml_escape(&self.name)
        );
        let _ = writeln!(x, "  <description>{}</description>", xml_escape(&self.source));
        let mut style = |name: &str, attrs: &[(&str, &str)]| {
            let _ = write!(x, r#"  <style name="{name}""#);
            for (k, v) in attrs {
                let _ = write!(x, r#" {k}="{}""#, xml_escape(v));
            }
            x.push_str("/>\n");
        };
        style("text", &[("foreground", &e.fg), ("background", &e.bg)]);
        let selection_fg = e.selection_fg.as_deref().unwrap_or(&e.fg);
        style("selection", &[("foreground", selection_fg), ("background", &e.selection)]);
        style("selection-unfocused", &[("foreground", selection_fg), ("background", &e.selection)]);
        style("cursor", &[("foreground", &e.cursor)]);
        style("secondary-cursor", &[("foreground", &u.muted)]);
        style("current-line", &[("background", &e.line)]);
        style("line-numbers", &[("foreground", &e.gutter_fg), ("background", &e.gutter_bg)]);
        style("current-line-number", &[("foreground", &e.gutter_active), ("background", &e.line), ("bold", "true")]);
        style("right-margin", &[("foreground", &u.border), ("background", &u.border)]);
        style("draw-spaces", &[("foreground", &u.border)]);
        style("bracket-match", &[("background", &e.selection), ("bold", "true")]);
        style("bracket-mismatch", &[("foreground", "#ffffff"), ("background", "#c01c28")]);
        style("search-match", &[("background", &e.search_match)]);
        for (role, names) in STYLE_MAP {
            let Some(s) = self.syntax.get(*role) else { continue };
            let mut attrs: Vec<(&str, &str)> = Vec::new();
            if let Some(fg) = &s.fg {
                attrs.push(("foreground", fg));
            }
            for (on, key) in [(s.bold, "bold"), (s.italic, "italic"), (s.underline, "underline")] {
                if on {
                    attrs.push((key, if key == "underline" { "single" } else { "true" }));
                }
            }
            if attrs.is_empty() {
                continue;
            }
            for name in *names {
                style(name, &attrs);
            }
        }
        x.push_str("</style-scheme>\n");
        Some(x)
    }

    /// libadwaita colour overrides (empty for the defaults).
    pub fn ui_css(&self) -> String {
        let (Some(e), Some(u)) = (&self.editor, &self.ui) else { return String::new() };
        let accent_fg = if luminance(&u.accent) > 0.4 { "#000000" } else { "#ffffff" };
        // Accent as text colour needs contrast against the background.
        let accent_text = if contrast(&u.accent, &e.bg) >= 3.0 { u.accent.clone() } else { mix(&u.accent, &e.fg, 0.5) };
        let raised = mix(&e.bg, &e.fg, if self.dark { 0.06 } else { 0.03 });
        let dialog = mix(&e.bg, &u.header, 0.5);
        let colors = [
            ("window_bg_color", e.bg.as_str()),
            ("window_fg_color", &e.fg),
            ("view_bg_color", &e.bg),
            ("view_fg_color", &e.fg),
            ("headerbar_bg_color", &u.header),
            ("headerbar_fg_color", &e.fg),
            ("headerbar_backdrop_color", &u.header),
            ("sidebar_bg_color", &u.sidebar),
            ("sidebar_fg_color", &e.fg),
            ("sidebar_backdrop_color", &u.sidebar),
            ("secondary_sidebar_bg_color", &u.sidebar),
            ("secondary_sidebar_fg_color", &e.fg),
            ("card_bg_color", &raised),
            ("card_fg_color", &e.fg),
            ("dialog_bg_color", &dialog),
            ("dialog_fg_color", &e.fg),
            ("popover_bg_color", &dialog),
            ("popover_fg_color", &e.fg),
            ("thumbnail_bg_color", &raised),
            ("thumbnail_fg_color", &e.fg),
            ("accent_bg_color", &u.accent),
            ("accent_fg_color", accent_fg),
            ("accent_color", &accent_text),
        ];
        let mut css = String::new();
        for (name, value) in colors {
            let _ = writeln!(css, "@define-color {name} {value};");
        }
        css
    }

    /// CSS custom properties for the document preview (empty for the defaults).
    pub fn preview_vars(&self) -> String {
        let (Some(e), Some(u)) = (&self.editor, &self.ui) else { return String::new() };
        let code_bg = mix(&e.bg, &e.fg, if self.dark { 0.07 } else { 0.05 });
        format!(
            "--bg:{};--fg:{};--muted:{};--border:{};--code-bg:{code_bg};--link:{};--quote:{};",
            e.bg, e.fg, u.muted, u.border, u.link, u.muted
        )
    }
}

/// Writes the style schemes of the bundled themes and registers their folder.
pub fn register() {
    let dir = store::data_dir().join("styles");
    let _ = std::fs::create_dir_all(&dir);
    let mut wanted = std::collections::HashSet::new();
    for theme in all() {
        let Some(xml) = theme.scheme_xml() else { continue };
        let file = dir.join(format!("{}.xml", theme.scheme_id()));
        wanted.insert(file.clone());
        if std::fs::read_to_string(&file).ok().as_deref() != Some(xml.as_str()) {
            let _ = std::fs::write(&file, xml);
        }
    }
    // Schemes of themes that are no longer bundled.
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            let ours = path.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.starts_with(SCHEME_PREFIX));
            if ours && !wanted.contains(&path) {
                let _ = std::fs::remove_file(path);
            }
        }
    }
    let sm = sourceview5::StyleSchemeManager::default();
    let dir = dir.to_string_lossy().into_owned();
    if !sm.search_path().iter().any(|p| p.as_str() == dir) {
        sm.prepend_search_path(&dir);
    }
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

fn rgb(hex: &str) -> [f64; 3] {
    let h = hex.trim_start_matches('#');
    let channel = |i: usize| h.get(i..i + 2).and_then(|s| u8::from_str_radix(s, 16).ok()).unwrap_or(0) as f64 / 255.0;
    [channel(0), channel(2), channel(4)]
}

fn mix(a: &str, b: &str, t: f64) -> String {
    let (a, b) = (rgb(a), rgb(b));
    let c: Vec<u8> = (0..3).map(|i| ((a[i] * (1.0 - t) + b[i] * t) * 255.0).round() as u8).collect();
    format!("#{:02x}{:02x}{:02x}", c[0], c[1], c[2])
}

fn luminance(hex: &str) -> f64 {
    let lin = |v: f64| if v <= 0.03928 { v / 12.92 } else { ((v + 0.055) / 1.055).powf(2.4) };
    let [r, g, b] = rgb(hex);
    0.2126 * lin(r) + 0.7152 * lin(g) + 0.0722 * lin(b)
}

fn contrast(a: &str, b: &str) -> f64 {
    let (la, lb) = (luminance(a), luminance(b));
    (la.max(lb) + 0.05) / (la.min(lb) + 0.05)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_themes_load() {
        let themes = all();
        assert!(themes.len() > 100, "only {} themes", themes.len());
        let mut ids = std::collections::HashSet::new();
        for t in themes {
            assert!(ids.insert(&t.id), "duplicate theme id {}", t.id);
            if t.source != "quicknotes" {
                let xml = t.scheme_xml().expect("scheme");
                assert!(xml.contains(r#"<style name="text""#), "{}", t.id);
                assert!(!t.ui_css().is_empty());
            }
        }
        assert!(SOURCES.iter().all(|(key, _, _)| themes.iter().any(|t| t.source == *key)));
    }

    #[test]
    fn unknown_ids_fall_back() {
        assert_eq!(find("nope", true).id, DEFAULT_DARK);
        assert_eq!(find("nope", false).id, DEFAULT_LIGHT);
        assert_eq!(find("dracula", false).id, "dracula");
    }

    #[test]
    fn colour_math() {
        assert_eq!(mix("#000000", "#ffffff", 0.5), "#808080");
        assert!(contrast("#000000", "#ffffff") > 20.0);
    }
}
