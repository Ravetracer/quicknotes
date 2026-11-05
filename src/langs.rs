//! Language (syntax highlighting) helpers on top of GtkSourceView.

use crate::store;
use gtk::gio;
use sourceview5::{Language, LanguageManager};
use std::path::Path;

const PLANTUML_LANG: &str = include_str!("../data/plantuml.lang");

/// Registers bundled language definitions. Must run before any language is used.
pub fn register() {
    let dir = store::data_dir().join("language-specs");
    let file = dir.join("plantuml.lang");
    if std::fs::read_to_string(&file).ok().as_deref() != Some(PLANTUML_LANG) {
        let _ = std::fs::create_dir_all(&dir);
        let _ = std::fs::write(&file, PLANTUML_LANG);
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
        let head = &content.as_bytes()[..content.len().min(4096)];
        let (content_type, _) = gio::content_type_guess(Some(path), head);
        if let Some(l) = lm.guess_language(Some(path), Some(&content_type)) {
            return Some(l);
        }
        if path.extension().is_some() {
            return None;
        }
    }
    sniff(content).and_then(by_id)
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
        let sample = &t[..t.len().min(1 << 20)];
        if serde_json::from_str::<serde::de::IgnoredAny>(sample).is_ok() {
            return Some("json");
        }
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
        assert_eq!(sniff("<?xml version=\"1.0\"?><a/>"), Some("xml"));
        assert_eq!(sniff("@startuml\nA -> B\n@enduml"), Some("plantuml"));
        assert_eq!(sniff("# Title\n\ntext"), Some("markdown"));
        assert_eq!(sniff("#!/bin/bash\necho"), Some("sh"));
        assert_eq!(sniff("just a note"), None);
    }
}
