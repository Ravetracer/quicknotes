//! Language (syntax highlighting) helpers on top of GtkSourceView.

use crate::store;
use gtk::gio;
use sourceview5::{Language, LanguageManager};
use std::path::Path;

/// Bundled language definitions: (file name, content).
const BUNDLED: &[(&str, &str)] = &[
    ("plantuml.lang", include_str!("../data/plantuml.lang")),
    ("calc.lang", include_str!("../data/calc.lang")),
    ("slides.lang", include_str!("../data/slides.lang")),
    ("mongodb.lang", include_str!("../data/mongodb.lang")),
    ("kanban.lang", include_str!("../data/kanban.lang")),
    ("flashcards.lang", include_str!("../data/flashcards.lang")),
    ("mindmap.lang", include_str!("../data/mindmap.lang")),
];

/// Registers bundled language definitions. Must run before any language is used.
pub fn register() {
    let dir = store::data_dir().join("language-specs");
    for (name, content) in BUNDLED {
        let file = dir.join(name);
        if std::fs::read_to_string(&file).ok().as_deref() != Some(*content) {
            let _ = std::fs::create_dir_all(&dir);
            let _ = std::fs::write(&file, content);
        }
    }
    let lm = LanguageManager::default();
    let mut paths: Vec<String> = lm.search_path().iter().map(|s| s.to_string()).collect();
    paths.insert(0, dir.to_string_lossy().into_owned());
    let refs: Vec<&str> = paths.iter().map(String::as_str).collect();
    lm.set_search_path(&refs);
}

pub fn by_id(id: &str) -> Option<Language> {
    LanguageManager::default().language(id)
}

/// All visible languages as (id, name), sorted by name.
pub fn all() -> Vec<(String, String)> {
    let lm = LanguageManager::default();
    let mut list: Vec<(String, String)> = lm
        .language_ids()
        .iter()
        .filter_map(|id| lm.language(id))
        .filter(|l| !l.is_hidden())
        .map(|l| (l.id().to_string(), l.name().to_string()))
        .collect();
    list.sort_by_key(|(_, name)| name.to_lowercase());
    list
}

/// Default file extension for a language, taken from its globs (e.g. `*.json` -> `json`).
pub fn extension(id: &str) -> Option<String> {
    let lang = by_id(id)?;
    lang.globs()
        .iter()
        .find_map(|g| g.strip_prefix("*.").filter(|e| !e.contains(['*', '?', '['])).map(str::to_owned))
}

/// Guesses the language from the file name and/or content.
pub fn guess(path: Option<&Path>, content: &str) -> Option<Language> {
    let lm = LanguageManager::default();
    if let Some(path) = path {
        let name = path.file_name().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default();
        if name.ends_with(".kanban.md") {
            return by_id(crate::kanban::LANGUAGE);
        }
        if name.ends_with(".cards.md") || name.ends_with(".flashcards.md") {
            return by_id(crate::flashcards::LANGUAGE);
        }
        if name.ends_with(".mindmap.md") {
            return by_id(crate::mindmap::LANGUAGE);
        }
        if name.ends_with(".slides.md") {
            return by_id(crate::slides::LANGUAGE);
        }
        if name.ends_with(".mongodb.js") {
            return by_id(crate::mongo::LANGUAGE);
        }
        let head = &content.as_bytes()[..content.len().min(4096)];
        let (content_type, _) = gio::content_type_guess(Some(path), head);
        if let Some(l) = lm.guess_language(Some(path), Some(&content_type)) {
            if l.id() == "markdown" && crate::kanban::is_board(content) {
                return by_id(crate::kanban::LANGUAGE);
            }
            if l.id() == "markdown" && crate::flashcards::is_flashcards(content) {
                return by_id(crate::flashcards::LANGUAGE);
            }
            if l.id() == "markdown" && crate::mindmap::is_mindmap(content) {
                return by_id(crate::mindmap::LANGUAGE);
            }
            if l.id() == "markdown" && crate::slides::is_presentation(content) {
                return by_id(crate::slides::LANGUAGE);
            }
            return Some(l);
        }
        if path.extension().is_some() {
            return None;
        }
    }
    sniff(content).and_then(by_id)
}

/// The first MiB of `t` (cut at a character boundary).
fn sample_of(t: &str) -> &str {
    let mut end = t.len().min(1 << 20);
    while !t.is_char_boundary(end) {
        end -= 1;
    }
    &t[..end]
}

/// Content-based detection for snippets without a file name.
pub fn sniff(content: &str) -> Option<&'static str> {
    let t = content.trim_start();
    if t.is_empty() {
        return None;
    }
    if t.starts_with("@start") {
        return Some("plantuml");
    }
    if t.starts_with('{') || t.starts_with('[') {
        let sample = sample_of(t);
        if serde_json::from_str::<serde::de::IgnoredAny>(sample).is_ok() {
            return Some("json");
        }
    }
    if crate::mongo::sniff(sample_of(t)) {
        return Some(crate::mongo::LANGUAGE);
    }
    if t.starts_with("<?xml") {
        return Some("xml");
    }
    let lower = t[..t.len().min(64)].to_ascii_lowercase();
    if lower.starts_with("<!doctype html") || lower.starts_with("<html") {
        return Some("html");
    }
    if t.starts_with('<') && t.contains("</") {
        return Some("xml");
    }
    if let Some(first) = t.lines().next() {
        if first.starts_with("#!") {
            return match first {
                f if f.contains("python") => Some("python3"),
                f if f.contains("bash") || f.ends_with("/sh") || f.contains("zsh") => Some("sh"),
                f if f.contains("node") => Some("js"),
                f if f.contains("php") => Some("php"),
                _ => None,
            };
        }
        if first.starts_with("<?php") {
            return Some("php");
        }
        if first == "---" {
            if crate::kanban::is_board(t) {
                return Some(crate::kanban::LANGUAGE);
            }
            if crate::flashcards::is_flashcards(t) {
                return Some(crate::flashcards::LANGUAGE);
            }
            if crate::mindmap::is_mindmap(t) {
                return Some(crate::mindmap::LANGUAGE);
            }
            if crate::slides::is_presentation(t) {
                return Some(crate::slides::LANGUAGE);
            }
            return Some("yaml");
        }
    }
    let md_lines = t
        .lines()
        .take(200)
        .filter(|l| {
            l.starts_with("# ") || l.starts_with("## ") || l.starts_with("### ") || l.starts_with("```")
        })
        .count();
    if md_lines >= 1 && (t.starts_with('#') || md_lines >= 2) {
        return Some("markdown");
    }
    None
}

#[cfg(test)]
mod tests {
    use super::sniff;

    #[test]
    fn sniffs_common_formats() {
        assert_eq!(sniff("  {\"a\": 1}"), Some("json"));
        assert_eq!(sniff("{ not json"), None);
        assert_eq!(sniff("{ a: 1, b: 'x' }"), Some("mongodb"));
        assert_eq!(sniff("// orders\ndb.orders.find({})"), Some("mongodb"));
        assert_eq!(sniff("{\"$oid\": \"x\"}"), Some("json"));
        assert_eq!(sniff("<?xml version=\"1.0\"?><a/>"), Some("xml"));
        assert_eq!(sniff("@startuml\nA -> B\n@enduml"), Some("plantuml"));
        assert_eq!(sniff("# Title\n\ntext"), Some("markdown"));
        assert_eq!(sniff("#!/bin/bash\necho"), Some("sh"));
        assert_eq!(sniff("just a note"), None);
        assert_eq!(sniff("---\npresentation: true\n---\n# Hi"), Some("slides"));
        assert_eq!(sniff("---\nkey: value\n"), Some("yaml"));
        assert_eq!(sniff("---\nkanban: 1\n---\n## To do\n"), Some("kanban"));
        assert_eq!(sniff("---\nflashcards: true\n---\nQ::A\n"), Some("flashcards"));
    }
}

#[cfg(test)]
mod lang_file_check {
    use gtk::glib::translate::{from_glib_full, ToGlibPtr};
    use sourceview5::prelude::*;

    /// Compiles all regular expressions of `data/mongodb.lang` by highlighting a
    /// sample. Uses the FFI constructors: the safe ones need a display.
    #[test]
    fn mongodb_lang_compiles() {
        gtk::glib::log_set_always_fatal(gtk::glib::LogLevels::LEVEL_WARNING | gtk::glib::LogLevels::LEVEL_CRITICAL);
        let lm: sourceview5::LanguageManager = unsafe { from_glib_full(sourceview5::ffi::gtk_source_language_manager_new()) };
        let mut paths: Vec<String> = lm.search_path().iter().map(|s| s.to_string()).collect();
        paths.insert(0, concat!(env!("CARGO_MANIFEST_DIR"), "/data").into());
        lm.set_search_path(&paths.iter().map(String::as_str).collect::<Vec<_>>());
        let lang = lm.language("mongodb").expect("language loads");
        let buf: sourceview5::Buffer =
            unsafe { from_glib_full(sourceview5::ffi::gtk_source_buffer_new_with_language(lang.to_glib_none().0)) };
        buf.set_text("// c\ndb.o.aggregate([{ $match: { 'a': /x\\/y/i, b: \"$f\", n: -1.5e3, id: ObjectId(\"x\") } }]) /* b */");
        let (s, e) = buf.bounds();
        buf.ensure_highlight(&s, &e);
    }
}
