//! Obsidian vaults: importing a vault into the notes library, and the
//! Obsidian flavour of Markdown in the preview (wikilinks, embeds,
//! callouts, highlights, comments and tags).
//!
//! The note text is kept as written; the preview understands the syntax.
//! Attachments of imported notes are copied to `attachments/<vault>` in
//! the data folder, where `![[image.png]]` finds them.

use pulldown_cmark::{Event, Tag, TagEnd};
use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// URL scheme of wikilinks in the preview (`qn-note:Title`).
pub const NOTE_SCHEME: &str = "qn-note:";

const IMAGE_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "gif", "svg", "webp", "bmp", "avif", "ico"];
const MAX_DEPTH: usize = 32;
/// More files than this is no vault (e.g. the home folder picked by mistake).
const MAX_FILES: usize = 50_000;

// ===== import ===============================================================

/// A note of a scanned vault.
#[derive(Debug, Clone, PartialEq)]
pub struct Note {
    /// Path inside the vault (`Work/Ideas.md`), with `/` separators.
    pub rel: String,
    /// Folder names from the vault root down to the note.
    pub dirs: Vec<String>,
    pub title: String,
    pub content: String,
}

/// The notes and referenced attachments of a vault.
#[derive(Debug, Default)]
pub struct Scan {
    pub name: String,
    pub notes: Vec<Note>,
    /// Attachments used by the notes: source file and path inside the vault.
    pub attachments: Vec<(PathBuf, String)>,
}

/// Folder name for the attachments of a vault.
pub fn attachment_dir(name: &str) -> PathBuf {
    let safe: String = name.chars().map(|c| if c == '/' || c == '\\' || c.is_control() { '_' } else { c }).collect();
    crate::store::data_dir().join("attachments").join(safe.trim_start_matches('.'))
}

fn walk(dir: &Path, rel: &str, depth: usize, files: &mut Vec<String>) -> std::io::Result<()> {
    if depth > MAX_DEPTH {
        return Ok(());
    }
    let mut entries: Vec<_> = fs::read_dir(dir)?.flatten().collect();
    entries.sort_by_key(|e| e.file_name());
    for entry in entries {
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') {
            continue;
        }
        let Ok(meta) = fs::symlink_metadata(entry.path()) else { continue };
        let child = if rel.is_empty() { name.clone() } else { format!("{rel}/{name}") };
        if meta.is_dir() {
            walk(&entry.path(), &child, depth + 1, files)?;
        } else if meta.is_file() || (meta.file_type().is_symlink() && entry.path().is_file()) {
            files.push(child);
        }
        if files.len() > MAX_FILES {
            return Err(std::io::Error::other(t!("more than {} files – is this an Obsidian vault?", MAX_FILES)));
        }
    }
    Ok(())
}

fn is_note(rel: &str) -> bool {
    rel.to_lowercase().ends_with(".md")
}

/// Reads a vault. Markdown links to files of the vault (`![](img/a.png)`)
/// are pointed at their copy below `dest`; Obsidian syntax stays as it is.
pub fn scan(root: &Path, dest: &Path) -> Result<Scan, String> {
    let mut files = Vec::new();
    walk(root, "", 0, &mut files).map_err(|e| e.to_string())?;
    let name = root.file_name().map_or_else(|| "Obsidian".to_owned(), |n| n.to_string_lossy().into_owned());
    let others: HashSet<&str> = files.iter().filter(|f| !is_note(f)).map(String::as_str).collect();
    let mut by_name: HashMap<String, Vec<&str>> = HashMap::new();
    for f in &others {
        let file = f.rsplit('/').next().unwrap_or(f).to_lowercase();
        by_name.entry(file).or_default().push(f);
    }
    for list in by_name.values_mut() {
        list.sort_by_key(|f| (f.matches('/').count(), f.to_string()));
    }
    // Obsidian finds a file by its path in the vault, next to the note, or
    // by its name anywhere.
    let resolve = |note_dir: &str, target: &str| -> Option<String> {
        let target = urlencoding::decode(target).map(Cow::into_owned).unwrap_or_else(|_| target.to_owned());
        let target = target.trim_start_matches("./");
        let joined = normalize(&if note_dir.is_empty() { target.to_owned() } else { format!("{note_dir}/{target}") });
        for candidate in [Some(target.to_owned()), joined] {
            if let Some(c) = candidate.filter(|c| others.contains(c.as_str())) {
                return Some(c);
            }
        }
        let file = target.rsplit('/').next()?.to_lowercase();
        by_name.get(&file).and_then(|l| l.first()).map(|s| (*s).to_owned())
    };

    let mut used: Vec<String> = Vec::new();
    let mut notes = Vec::new();
    for rel in files.iter().filter(|f| is_note(f)) {
        let bytes = fs::read(root.join(rel)).map_err(|e| format!("{rel}: {e}"))?;
        let text = String::from_utf8_lossy(&bytes);
        let mut content = text.trim_start_matches('\u{feff}').replace("\r\n", "\n");
        let (dir, file) = rel.rsplit_once('/').unwrap_or(("", rel));
        for target in wiki_targets(&content) {
            if let Some(found) = resolve(dir, &target) {
                used.push(found);
            }
        }
        content = rewrite_links(&content, |target| {
            let found = resolve(dir, target)?;
            used.push(found.clone());
            Some(file_uri(&dest.join(&found)))
        });
        let title = file[..file.len() - 3].to_owned();
        let dirs = if dir.is_empty() { Vec::new() } else { dir.split('/').map(str::to_owned).collect() };
        notes.push(Note { rel: rel.clone(), dirs, title, content });
    }
    used.sort();
    used.dedup();
    let attachments = used.into_iter().map(|rel| (root.join(&rel), rel)).collect();
    Ok(Scan { name, notes, attachments })
}

/// Resolves `..` and `.` in a relative path; `None` if it leaves the vault.
fn normalize(path: &str) -> Option<String> {
    let mut parts: Vec<&str> = Vec::new();
    for p in path.split('/') {
        match p {
            "" | "." => {}
            ".." => {
                parts.pop()?;
            }
            p => parts.push(p),
        }
    }
    Some(parts.join("/"))
}

/// Copies the attachments of a scan below `dest`; returns how many.
pub fn copy_attachments(scan: &Scan, dest: &Path) -> Result<usize, String> {
    let mut copied = 0;
    for (src, rel) in &scan.attachments {
        let target = dest.join(rel);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        fs::copy(src, &target).map_err(|e| format!("{rel}: {e}"))?;
        copied += 1;
    }
    invalidate_index();
    Ok(copied)
}

/// Targets of `[[…]]` and `![[…]]` outside code (without `#heading` and `|alias`).
fn wiki_targets(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in code_free_lines(text) {
        let mut rest = line.as_str();
        while let Some(start) = rest.find("[[") {
            let after = &rest[start + 2..];
            let Some(end) = after.find("]]") else { break };
            let (target, _, _) = split_wikilink(&after[..end]);
            if !target.is_empty() {
                out.push(target.to_owned());
            }
            rest = &after[end + 2..];
        }
    }
    out
}

/// Lines outside fenced code blocks, inline code removed.
fn code_free_lines(text: &str) -> Vec<String> {
    let mut fence: Option<String> = None;
    let mut out = Vec::new();
    for line in text.split('\n') {
        if let Some(f) = &fence {
            if line.trim_start().starts_with(f.as_str()) {
                fence = None;
            }
            continue;
        }
        if let Some(f) = fence_start(line) {
            fence = Some(f);
            continue;
        }
        let mut kept = String::new();
        let mut in_code = false;
        for c in line.chars() {
            if c == '`' {
                in_code = !in_code;
            } else if !in_code {
                kept.push(c);
            }
        }
        out.push(kept);
    }
    out
}

fn fence_start(line: &str) -> Option<String> {
    let t = line.trim_start();
    for ch in ['`', '~'] {
        let n = t.chars().take_while(|c| *c == ch).count();
        if n >= 3 {
            return Some(ch.to_string().repeat(n));
        }
    }
    None
}

/// Replaces relative targets of Markdown links and images (`](path)`)
/// outside code with what `map` returns.
fn rewrite_links(text: &str, mut map: impl FnMut(&str) -> Option<String>) -> String {
    let mut out = String::with_capacity(text.len());
    let mut fence: Option<String> = None;
    for (i, line) in text.split('\n').enumerate() {
        if i > 0 {
            out.push('\n');
        }
        if let Some(f) = &fence {
            if line.trim_start().starts_with(f.as_str()) {
                fence = None;
            }
            out.push_str(line);
            continue;
        }
        if let Some(f) = fence_start(line) {
            fence = Some(f);
            out.push_str(line);
            continue;
        }
        let mut rest = line;
        let mut in_code = false;
        while !rest.is_empty() {
            let next_tick = rest.find('`');
            let next_link = if in_code { None } else { rest.find("](") };
            match (next_tick, next_link) {
                (Some(t), l) if l.is_none_or(|l| t < l) => {
                    out.push_str(&rest[..=t]);
                    in_code = !in_code;
                    rest = &rest[t + 1..];
                }
                (_, Some(l)) => {
                    out.push_str(&rest[..l + 2]);
                    rest = &rest[l + 2..];
                    let end = rest.find([')', ' ']).unwrap_or(rest.len());
                    let target = &rest[..end];
                    let relative = !target.is_empty() && !target.contains("://") && !target.starts_with(['#', '/', '<']) && !target.starts_with("mailto:");
                    match relative.then(|| map(target)).flatten() {
                        Some(new) => out.push_str(&new),
                        None => out.push_str(target),
                    }
                    rest = &rest[end..];
                }
                _ => {
                    out.push_str(rest);
                    rest = "";
                }
            }
        }
    }
    out
}

fn file_uri(path: &Path) -> String {
    let encoded: Vec<String> = path.to_string_lossy().split('/').map(|s| urlencoding::encode(s).into_owned()).collect();
    format!("file://{}", encoded.join("/"))
}

// ===== attachments of the preview ==========================================

static INDEX: Mutex<Option<HashMap<String, PathBuf>>> = Mutex::new(None);

/// Forgets the attachment index (after an import).
pub fn invalidate_index() {
    *INDEX.lock().unwrap_or_else(|e| e.into_inner()) = None;
}

fn build_index() -> HashMap<String, PathBuf> {
    let base = crate::store::data_dir().join("attachments");
    let mut files = Vec::new();
    let _ = walk(&base, "", 0, &mut files);
    let mut index = HashMap::new();
    for rel in files {
        let path = base.join(&rel);
        // By name, and by path inside the vault (without the vault folder).
        let inside = rel.split_once('/').map_or(rel.as_str(), |(_, r)| r).to_lowercase();
        let name = rel.rsplit('/').next().unwrap_or(&rel).to_lowercase();
        index.entry(inside).or_insert_with(|| path.clone());
        index.entry(name).or_insert(path);
    }
    index
}

/// URL of an embedded file: its copy from an imported vault, otherwise the
/// name relative to the document.
fn attachment_url(target: &str) -> String {
    let key = target.trim_start_matches("./").to_lowercase();
    let mut guard = INDEX.lock().unwrap_or_else(|e| e.into_inner());
    let index = guard.get_or_insert_with(build_index);
    let found = index.get(&key).or_else(|| index.get(key.rsplit('/').next().unwrap_or(&key)));
    match found {
        Some(path) => file_uri(path),
        None => target.split('/').map(|s| urlencoding::encode(s).into_owned()).collect::<Vec<_>>().join("/"),
    }
}

// ===== preview ==============================================================

fn escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

/// `Note#Heading|Alias` → (target, heading, alias).
fn split_wikilink(inner: &str) -> (&str, Option<&str>, Option<&str>) {
    let (link, alias) = match inner.split_once('|') {
        Some((l, a)) => (l, Some(a.trim())),
        None => (inner, None),
    };
    let (target, heading) = match link.split_once('#') {
        Some((t, h)) => (t.trim(), Some(h.trim())),
        None => (link.trim(), None),
    };
    (target, heading, alias)
}

fn extension(target: &str) -> Option<String> {
    let file = target.rsplit('/').next()?;
    let (_, ext) = file.rsplit_once('.')?;
    Some(ext.to_lowercase())
}

fn note_href(target: &str) -> String {
    format!("{NOTE_SCHEME}{}", urlencoding::encode(target))
}

/// HTML for `[[inner]]` (`embed` for `![[inner]]`).
fn wikilink_html(inner: &str, embed: bool) -> String {
    let (target, heading, alias) = split_wikilink(inner);
    let ext = extension(target).filter(|e| e != "md");
    if embed {
        if let Some(ext) = &ext {
            let url = escape(&attachment_url(target));
            if IMAGE_EXTENSIONS.contains(&ext.as_str()) {
                // `|300` or `|300x200` sets the size.
                let size = alias
                    .and_then(|a| {
                        let (w, h) = a.split_once('x').unwrap_or((a, ""));
                        let w = w.trim().parse::<u32>().ok()?;
                        Some(match h.trim().parse::<u32>() {
                            Ok(h) => format!(" width=\"{w}\" height=\"{h}\""),
                            Err(_) => format!(" width=\"{w}\""),
                        })
                    })
                    .unwrap_or_default();
                return format!("<img class=\"embed\" src=\"{url}\" alt=\"{}\"{size}>", escape(target));
            }
            return format!("<a class=\"embed\" href=\"{url}\">📎 {}</a>", escape(alias.unwrap_or(target)));
        }
    }
    if ext.is_some() {
        return format!("<a href=\"{}\">{}</a>", escape(&attachment_url(target)), escape(alias.unwrap_or(target)));
    }
    let text = match (alias, heading) {
        (Some(a), _) if !a.is_empty() => a.to_owned(),
        (_, Some(h)) if target.is_empty() => h.to_owned(),
        (_, Some(h)) => format!("{target} › {h}"),
        _ => target.to_owned(),
    };
    if target.is_empty() {
        // A heading of the same note.
        return format!("<a class=\"wikilink\" href=\"#\">{}</a>", escape(&text));
    }
    let class = if embed { "wikilink embed" } else { "wikilink" };
    let arrow = if embed { "↪ " } else { "" };
    format!("<a class=\"{class}\" href=\"{}\">{arrow}{}</a>", escape(&note_href(target)), escape(&text))
}

fn is_tag_char(c: char) -> bool {
    c.is_alphanumeric() || matches!(c, '_' | '-' | '/')
}

/// Obsidian syntax to HTML inside Markdown, line by line (lines stay where
/// they are): wikilinks and embeds, `==highlights==`, `%%comments%%` and
/// `#tags`. Code is left alone.
pub fn preprocess(markdown: &str) -> Cow<'_, str> {
    if !["[[", "==", "%%", "#"].iter().any(|p| markdown.contains(p)) {
        return Cow::Borrowed(markdown);
    }
    let mut out = String::with_capacity(markdown.len() + 64);
    let mut fence: Option<String> = None;
    let mut comment = false;
    for (i, line) in markdown.split('\n').enumerate() {
        if i > 0 {
            out.push('\n');
        }
        if let Some(f) = &fence {
            if line.trim_start().starts_with(f.as_str()) {
                fence = None;
            }
            out.push_str(line);
            continue;
        }
        if !comment {
            if let Some(f) = fence_start(line) {
                fence = Some(f);
                out.push_str(line);
                continue;
            }
        }
        inline(line, &mut out, &mut comment);
    }
    Cow::Owned(out)
}

fn inline(line: &str, out: &mut String, comment: &mut bool) {
    let chars: Vec<char> = line.chars().collect();
    let text = |from: usize, to: usize| chars[from..to].iter().collect::<String>();
    let find = |from: usize, pat: &str| -> Option<usize> {
        let p: Vec<char> = pat.chars().collect();
        (from..chars.len().saturating_sub(p.len() - 1)).find(|&k| chars[k..k + p.len()] == p[..])
    };
    let mut mark = false;
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        let at = |pat: &str| pat.chars().enumerate().all(|(k, p)| chars.get(i + k) == Some(&p));
        if *comment {
            if at("%%") {
                *comment = false;
                i += 2;
            } else {
                i += 1;
            }
            continue;
        }
        if at("%%") {
            *comment = true;
            i += 2;
            continue;
        }
        match c {
            '\\' => {
                out.push(c);
                if let Some(n) = chars.get(i + 1) {
                    out.push(*n);
                }
                i += 2;
            }
            '`' => {
                // Inline code: copied as it is up to the closing run.
                let run = chars[i..].iter().take_while(|c| **c == '`').count();
                let ticks = "`".repeat(run);
                match find(i + run, &ticks) {
                    Some(end) => {
                        out.push_str(&text(i, end + run));
                        i = end + run;
                    }
                    None => {
                        out.push_str(&ticks);
                        i += run;
                    }
                }
            }
            '!' if at("![[") => match find(i + 3, "]]") {
                Some(end) => {
                    out.push_str(&wikilink_html(&text(i + 3, end), true));
                    i = end + 2;
                }
                None => {
                    out.push(c);
                    i += 1;
                }
            },
            '[' if at("[[") => match find(i + 2, "]]") {
                Some(end) => {
                    out.push_str(&wikilink_html(&text(i + 2, end), false));
                    i = end + 2;
                }
                None => {
                    out.push(c);
                    i += 1;
                }
            },
            '=' if at("==") => {
                let opens = !mark && chars.get(i + 2).is_some_and(|n| !n.is_whitespace() && *n != '=') && find(i + 3, "==").is_some();
                if opens {
                    out.push_str("<mark>");
                    mark = true;
                } else if mark && i > 0 && !chars[i - 1].is_whitespace() {
                    out.push_str("</mark>");
                    mark = false;
                } else {
                    out.push_str("==");
                }
                i += 2;
            }
            '#' if (i == 0 || chars[i - 1].is_whitespace()) && chars.get(i + 1).is_some_and(|n| is_tag_char(*n)) => {
                let end = (i + 1..chars.len()).find(|&k| !is_tag_char(chars[k])).unwrap_or(chars.len());
                let tag = text(i + 1, end);
                if tag.chars().all(|c| c.is_ascii_digit()) {
                    out.push('#');
                    i += 1;
                } else {
                    out.push_str(&format!("<span class=\"tag\">#{}</span>", escape(&tag)));
                    i = end;
                }
            }
            c => {
                out.push(c);
                i += 1;
            }
        }
    }
    if mark {
        out.push_str("</mark>");
    }
}

/// Callout kind and its colour group (see the preview CSS).
fn callout_group(kind: &str) -> &'static str {
    match kind {
        "tip" | "hint" | "important" | "success" | "check" | "done" => "tip",
        "warning" | "caution" | "attention" | "question" | "help" | "faq" => "warn",
        "failure" | "fail" | "missing" | "danger" | "error" | "bug" => "error",
        "example" => "example",
        "quote" | "cite" => "quote",
        "abstract" | "summary" | "tldr" | "todo" => "abstract",
        _ => "note",
    }
}

/// `> [!type] Title` block quotes become callouts; `[!type]-` and
/// `[!type]+` fold (closed or open).
pub fn callouts(events: Vec<Event<'_>>) -> Vec<Event<'_>> {
    let mut out = Vec::with_capacity(events.len());
    // Per open block quote: the closing HTML of a callout.
    let mut stack: Vec<Option<&'static str>> = Vec::new();
    let mut i = 0;
    while i < events.len() {
        match &events[i] {
            Event::Start(Tag::BlockQuote(_)) => {
                if let Some((html, close, skip, reopen)) = callout_start(&events, i) {
                    out.push(Event::Html(html.into()));
                    if reopen {
                        out.push(Event::Start(Tag::Paragraph));
                    }
                    stack.push(Some(close));
                    i = skip;
                    continue;
                }
                stack.push(None);
                out.push(events[i].clone());
            }
            Event::End(TagEnd::BlockQuote(_)) => match stack.pop().flatten() {
                Some(close) => out.push(Event::Html(close.into())),
                None => out.push(events[i].clone()),
            },
            e => out.push(e.clone()),
        }
        i += 1;
    }
    out
}

/// For a block quote at `i` that starts with `[!type]`: the opening HTML,
/// the closing HTML, the index to continue at and whether the first
/// paragraph goes on after the title line.
fn callout_start(events: &[Event<'_>], i: usize) -> Option<(String, &'static str, usize, bool)> {
    if !matches!(events.get(i + 1), Some(Event::Start(Tag::Paragraph))) {
        return None;
    }
    let mut first = String::new();
    let mut k = i + 2;
    while let Some(Event::Text(t)) = events.get(k) {
        first.push_str(t);
        k += 1;
    }
    let rest = first.strip_prefix("[!")?;
    let close = rest.find(']')?;
    let kind = rest[..close].trim().to_lowercase();
    if kind.is_empty() || !kind.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
        return None;
    }
    let after = &rest[close + 1..];
    let (fold, title) = match after.chars().next() {
        Some(f @ ('+' | '-')) => (Some(f), after[1..].trim()),
        _ => (None, after.trim()),
    };
    let title = if title.is_empty() {
        let mut c = kind.chars();
        c.next().map(|f| f.to_uppercase().collect::<String>() + c.as_str()).unwrap_or_default()
    } else {
        title.to_owned()
    };
    let (next, reopen) = match events.get(k) {
        Some(Event::End(TagEnd::Paragraph)) => (k + 1, false),
        Some(Event::SoftBreak | Event::HardBreak) => (k + 1, true),
        // Formatting in the title line: the rest stays in the body.
        _ => (k, true),
    };
    let group = callout_group(&kind);
    let kind = escape(&kind);
    let title = escape(&title);
    Some(match fold {
        Some(f) => {
            let open = if f == '+' { " open" } else { "" };
            (
                format!("<details class=\"callout {group}\" data-callout=\"{kind}\"{open}><summary class=\"callout-title\">{title}</summary><div class=\"callout-content\">"),
                "</div></details>",
                next,
                reopen,
            )
        }
        None => (
            format!("<div class=\"callout {group}\" data-callout=\"{kind}\"><div class=\"callout-title\">{title}</div><div class=\"callout-content\">"),
            "</div></div>",
            next,
            reopen,
        ),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn html(md: &str) -> String {
        crate::markdown::to_html(md)
    }

    #[test]
    fn wikilinks_and_embeds() {
        let h = html("See [[Other Note]] and [[Other Note#Part|that part]] or [[#Local]].");
        assert!(h.contains("<a class=\"wikilink\" href=\"qn-note:Other%20Note\">Other Note</a>"), "{h}");
        assert!(h.contains(">that part</a>"), "{h}");
        assert!(h.contains("href=\"#\">Local</a>"), "{h}");
        let h = html("![[pic one.png|300]]\n\n![[Some Note]]");
        assert!(h.contains("<img class=\"embed\" src=\"pic%20one.png\" alt=\"pic one.png\" width=\"300\">"), "{h}");
        assert!(h.contains("↪ Some Note"), "{h}");
    }

    #[test]
    fn highlights_comments_and_tags() {
        let h = html("A ==marked **bold**== word %%hidden%% #todo and #123 but `==code== #no`.");
        assert!(h.contains("<mark>marked <strong>bold</strong></mark>"), "{h}");
        assert!(!h.contains("hidden"), "{h}");
        assert!(h.contains("<span class=\"tag\">#todo</span>"), "{h}");
        assert!(h.contains("#123") && !h.contains("tag\">#123"), "{h}");
        assert!(h.contains("<code>==code== #no</code>"), "{h}");
        let h = html("before\n%%\nsecret\n%%\nafter\n\n# Heading");
        assert!(!h.contains("secret") && h.contains("after") && h.contains("<h1>Heading</h1>"), "{h}");
        let h = html("```\n[[not a link]] ==x==\n```");
        assert!(h.contains("[[not a link]] ==x=="), "{h}");
    }

    #[test]
    fn callouts_render() {
        let h = html("> [!warning] Careful\n> Hot **stuff**\n\n> plain quote");
        assert!(h.contains("<div class=\"callout warn\" data-callout=\"warning\"><div class=\"callout-title\">Careful</div>"), "{h}");
        assert!(h.contains("<p>Hot <strong>stuff</strong></p>"), "{h}");
        assert!(h.contains("<blockquote>\n<p>plain quote</p>"), "{h}");
        let h = html("> [!tip]-\n> folded");
        assert!(h.contains("<details class=\"callout tip\" data-callout=\"tip\"><summary class=\"callout-title\">Tip</summary>"), "{h}");
        assert!(h.contains("folded") && h.contains("</div></details>"), "{h}");
    }

    #[test]
    fn interactive_tasks_keep_lines() {
        let h = crate::markdown::to_html_interactive("%%\na\n%%\n- [ ] task [[Link]]");
        assert!(h.contains("data-line=\"3\""), "{h}");
    }

    #[test]
    fn scans_a_vault() {
        let dir = std::env::temp_dir().join(format!("qn-obsidian-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let vault = dir.join("My Vault");
        fs::create_dir_all(vault.join(".obsidian")).unwrap();
        fs::create_dir_all(vault.join("Work/Deep")).unwrap();
        fs::create_dir_all(vault.join("assets")).unwrap();
        fs::write(vault.join(".obsidian/app.json"), "{}").unwrap();
        fs::write(vault.join("Start.md"), "\u{feff}# Hi\r\n![[logo.png]] ![x](assets/chart%20one.png) [web](https://x.org/a.png)").unwrap();
        fs::write(vault.join("Work/Deep/Plan.md"), "![up](../../assets/logo.png)\n`![](assets/nope.png)`").unwrap();
        fs::write(vault.join("assets/logo.png"), b"png").unwrap();
        fs::write(vault.join("assets/chart one.png"), b"png").unwrap();
        fs::write(vault.join("assets/unused.png"), b"png").unwrap();
        let dest = dir.join("dest");
        let scan = scan(&vault, &dest).unwrap();
        assert_eq!(scan.name, "My Vault");
        let rels: Vec<&str> = scan.notes.iter().map(|n| n.rel.as_str()).collect();
        assert_eq!(rels, ["Start.md", "Work/Deep/Plan.md"]);
        let start = &scan.notes[0];
        assert_eq!(start.title, "Start");
        assert!(start.content.starts_with("# Hi\n![[logo.png]] ![x](file://"), "{}", start.content);
        assert!(start.content.contains("/dest/assets/chart%20one.png)"), "{}", start.content);
        assert!(start.content.contains("(https://x.org/a.png)"));
        let plan = &scan.notes[1];
        assert_eq!(plan.dirs, ["Work", "Deep"]);
        assert!(plan.content.contains("/dest/assets/logo.png)"), "{}", plan.content);
        assert!(plan.content.contains("`![](assets/nope.png)`"), "{}", plan.content);
        let used: Vec<&str> = scan.attachments.iter().map(|(_, r)| r.as_str()).collect();
        assert_eq!(used, ["assets/chart one.png", "assets/logo.png"]);
        assert_eq!(copy_attachments(&scan, &dest).unwrap(), 2);
        assert!(dest.join("assets/logo.png").exists());
        let _ = fs::remove_dir_all(&dir);
    }
}
