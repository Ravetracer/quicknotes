//! The changelog (`data/changelog*.md`): what's new after an update, the
//! release notes of the About dialog, and the check for a newer release on
//! GitHub.

use std::cmp::Ordering;

const CHANGELOG_EN: &str = include_str!("../data/changelog.md");
const CHANGELOG_DE: &str = include_str!("../data/changelog.de.md");

/// Releases of QuickNotes (the latest one is checked at start).
const LATEST_RELEASE: &str = "https://api.github.com/repos/Ravetracer/quicknotes/releases/latest";

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// A version's entry: `## 0.18.0 – 2026-11-14` and its lines.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    pub version: String,
    pub date: String,
    pub body: String,
}

fn text() -> &'static str {
    match crate::i18n::lang() {
        crate::i18n::Lang::De => CHANGELOG_DE,
        crate::i18n::Lang::En => CHANGELOG_EN,
    }
}

pub fn entries_of(text: &str) -> Vec<Entry> {
    let mut out: Vec<Entry> = Vec::new();
    for line in text.lines() {
        if let Some(head) = line.strip_prefix("## ") {
            let mut parts = head.split(['–', '-']).map(str::trim);
            let version = parts.next().unwrap_or("").to_owned();
            let date = parts.collect::<Vec<_>>().join("-");
            out.push(Entry { version, date, body: String::new() });
        } else if let Some(e) = out.last_mut() {
            e.body.push_str(line);
            e.body.push('\n');
        }
    }
    for e in &mut out {
        e.body = e.body.trim().to_owned();
    }
    out
}

/// `0.18.0` → (0, 18, 0); a leading `v` is ignored.
fn parse(v: &str) -> Option<(u64, u64, u64)> {
    let mut it = v.trim().trim_start_matches('v').split('.').map(|p| p.split(|c: char| !c.is_ascii_digit()).next().unwrap_or("").parse::<u64>());
    Some((it.next()?.ok()?, it.next().unwrap_or(Ok(0)).ok()?, it.next().unwrap_or(Ok(0)).ok()?))
}

pub fn compare(a: &str, b: &str) -> Ordering {
    match (parse(a), parse(b)) {
        (Some(x), Some(y)) => x.cmp(&y),
        _ => Ordering::Equal,
    }
}

/// The entries after `last` up to this version (only this version's when
/// `last` is unknown).
pub fn since(last: &str) -> Vec<Entry> {
    entries_of(text())
        .into_iter()
        .filter(|e| compare(&e.version, VERSION) != Ordering::Greater)
        .filter(|e| if last.is_empty() { compare(&e.version, VERSION) == Ordering::Equal } else { compare(&e.version, last) == Ordering::Greater })
        .collect()
}

/// The "What's new" document for the entries since `last`.
pub fn whats_new(last: &str) -> Option<String> {
    let entries = since(last);
    if entries.is_empty() {
        return None;
    }
    let mut out = format!("# {}\n\n", t!("What's New in QuickNotes {}", VERSION));
    for e in entries {
        out.push_str(&format!("## {} ({})\n\n{}\n\n", e.version, e.date, e.body));
    }
    out.push_str(&format!("---\n\n{}\n", t!("The complete history is in *About QuickNotes*.")));
    Some(out)
}

fn escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

/// Inline Markdown (`**bold**`, `*em*`, `` `code` ``) to the markup the About
/// dialog understands (`em`, `code`).
fn inline(s: &str) -> String {
    let mut out = String::new();
    let mut rest = escape(s);
    loop {
        let next = ["**", "`", "*"].iter().filter_map(|m| rest.find(m).map(|i| (i, *m))).min_by_key(|(i, m)| (*i, std::cmp::Reverse(m.len())));
        let Some((i, marker)) = next else { break };
        let after = &rest[i + marker.len()..];
        let Some(end) = after.find(marker) else { break };
        let tag = if marker == "`" { "code" } else { "em" };
        out.push_str(&rest[..i]);
        out.push_str(&format!("<{tag}>{}</{tag}>", &after[..end]));
        rest = after[end + marker.len()..].to_owned();
    }
    out.push_str(&rest);
    out
}

/// All entries as release notes for the About dialog (`<p>`, `<ul>`, `<li>`).
pub fn release_notes() -> String {
    let mut out = String::new();
    for e in entries_of(text()) {
        out.push_str(&format!("<p><em>{} – {}</em></p><ul>", escape(&e.version), escape(&e.date)));
        for line in e.body.lines() {
            if let Some(item) = line.trim().strip_prefix("- ") {
                out.push_str(&format!("<li>{}</li>", inline(item)));
            }
        }
        out.push_str("</ul>");
    }
    out
}

/// The latest release on GitHub: its version and page, when it is newer
/// than this one.
pub fn newer_release() -> Result<Option<(String, String)>, String> {
    let response = ureq::get(LATEST_RELEASE)
        .set("User-Agent", "QuickNotes")
        .set("Accept", "application/vnd.github+json")
        .timeout(std::time::Duration::from_secs(10))
        .call()
        .map_err(|e| e.to_string())?;
    let json: serde_json::Value = response.into_json().map_err(|e| e.to_string())?;
    let tag = json.get("tag_name").and_then(|v| v.as_str()).unwrap_or("").to_owned();
    let url = json.get("html_url").and_then(|v| v.as_str()).unwrap_or("").to_owned();
    Ok((compare(&tag, VERSION) == Ordering::Greater && !url.is_empty()).then(|| (tag.trim_start_matches('v').to_owned(), url)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn both_changelogs_list_the_same_versions() {
        let en: Vec<String> = entries_of(CHANGELOG_EN).into_iter().map(|e| e.version).collect();
        let de: Vec<String> = entries_of(CHANGELOG_DE).into_iter().map(|e| e.version).collect();
        assert_eq!(en, de);
        assert_eq!(en.first().map(String::as_str), Some(VERSION), "the changelog starts with this version");
        assert_eq!(en.last().map(String::as_str), Some("0.1.0"));
        for pair in en.windows(2) {
            assert_eq!(compare(&pair[0], &pair[1]), Ordering::Greater);
        }
    }

    #[test]
    fn versions_compare() {
        assert_eq!(compare("v0.18.0", "0.17.3"), Ordering::Greater);
        assert_eq!(compare("0.9.0", "0.10.0"), Ordering::Less);
        assert_eq!(compare("1.0", "1.0.0"), Ordering::Equal);
    }

    #[test]
    fn whats_new_and_release_notes() {
        let doc = whats_new("0.16.0").unwrap();
        assert!(doc.contains(&format!("## {VERSION}")));
        assert!(doc.contains("## 0.17.0"));
        assert!(!doc.contains("## 0.16.0"));
        assert!(whats_new(VERSION).is_none());
        let notes = release_notes();
        assert!(notes.starts_with(&format!("<p><em>{VERSION} – ")));
        assert!(notes.contains("<li>First version"));
        assert_eq!(inline("**A** & `b` *c*"), "<em>A</em> &amp; <code>b</code> <em>c</em>");
    }
}
