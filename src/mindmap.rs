//! Mind maps: a Markdown outline drawn as a map (see `data/mindmap`).
//!
//! ```text
//! ---
//! mindmap: 1
//! theme: pastel          # default, pastel, vivid, mono, dark, ocean, forest
//! layout: both           # both, right, down
//! font: Cantarell
//! ---
//! # Central topic
//! - Branch {color: #e01b24}
//!   - **Sub topic**
//!   - [ ] a task
//! ## Another branch {folded}
//! - item below the heading
//! ```
//!
//! The first `#` heading is the centre; further headings and nested list
//! items are the branches. `{color: …}` and `{folded}` at the end of a line
//! set a branch colour and fold the branch. Text keeps its Markdown styling
//! (bold, italic, code, strikethrough, links).
//!
//! The map is laid out and drawn as SVG in the preview page; the same page
//! renders the exports (SVG, and PDF on an A4 page turned to fit).

use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};
use serde::{Deserialize, Serialize};

pub const LANGUAGE: &str = "mindmap";

const MAP_CSS: &str = include_str!("../data/mindmap/mindmap.css");
const MAP_JS: &str = include_str!("../data/mindmap/mindmap.js");

/// Built-in themes (the names the page knows).
pub const THEMES: &[&str] = &["default", "pastel", "vivid", "mono", "dark", "ocean", "forest"];
pub const LAYOUTS: &[&str] = &["both", "right", "down"];

#[derive(Serialize, Debug, Default, PartialEq)]
pub struct Map {
    pub title: String,
    pub theme: String,
    pub layout: String,
    pub font: String,
    /// Branch colours replacing the theme's.
    pub colors: Vec<String>,
    pub root: Node,
}

#[derive(Serialize, Debug, Default, Clone, PartialEq)]
pub struct Node {
    /// Source line (0-based).
    pub line: usize,
    pub runs: Vec<Run>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub folded: bool,
    /// A task item: done or not.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task: Option<bool>,
    pub children: Vec<Node>,
}

/// A piece of text with one style; `\n` in `t` breaks the line.
#[derive(Serialize, Debug, Default, Clone, PartialEq)]
pub struct Run {
    pub t: String,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub b: bool,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub i: bool,
    /// Code (also formulas).
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub c: bool,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub s: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub href: Option<String>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct FrontMatter {
    mindmap: Option<serde_yaml::Value>,
    title: Option<String>,
    theme: Option<String>,
    layout: Option<String>,
    font: Option<String>,
    colors: Vec<String>,
}

/// The front matter (YAML between `---` lines at the top) and the line
/// after it.
fn front_matter(lines: &[&str]) -> (Option<FrontMatter>, usize) {
    match crate::slides::front_matter(lines) {
        (Some(yaml), end) => (serde_yaml::from_value::<FrontMatter>(serde_yaml::Value::Mapping(yaml)).ok(), end),
        (None, end) => (None, end),
    }
}

/// Whether `text` is a mind map (front matter with `mindmap:`).
pub fn is_mindmap(text: &str) -> bool {
    if !text.starts_with("---") {
        return false;
    }
    let lines: Vec<&str> = text.split('\n').take(60).collect();
    matches!(front_matter(&lines), (Some(fm), _) if fm.mindmap.is_some())
}

/// Splits `{color: #e01b24, folded}` off the end of a node text.
fn attributes(text: &str) -> (&str, Option<String>, bool) {
    let trimmed = text.trim_end();
    let Some(body) = trimmed.strip_suffix('}') else { return (text, None, false) };
    let Some(open) = body.rfind('{') else { return (text, None, false) };
    let inner = &body[open + 1..];
    let mut color = None;
    let mut folded = false;
    for part in inner.split(',').map(str::trim).filter(|p| !p.is_empty()) {
        match part.split_once(':') {
            Some((k, v)) if matches!(k.trim(), "color" | "colour" | "farbe") => color = Some(v.trim().to_owned()),
            Some(_) => return (text, None, false),
            None if matches!(part, "folded" | "collapsed" | "zu") => folded = true,
            None if part.starts_with('#') && part.len() > 1 => color = Some(part.to_owned()),
            None => return (text, None, false),
        }
    }
    (body[..open].trim_end(), color, folded)
}

/// Markdown inline text as styled runs.
fn runs(text: &str) -> Vec<Run> {
    let options = Options::ENABLE_STRIKETHROUGH | Options::ENABLE_MATH;
    let (mut b, mut i, mut s) = (0, 0, 0);
    let mut links: Vec<String> = Vec::new();
    let mut out: Vec<Run> = Vec::new();
    let push = |out: &mut Vec<Run>, t: &str, c: bool, b: i32, i: i32, s: i32, links: &[String]| {
        if t.is_empty() {
            return;
        }
        let run = Run { t: t.to_owned(), b: b > 0, i: i > 0, c, s: s > 0, href: links.last().cloned() };
        match out.last_mut() {
            Some(last) if Run { t: String::new(), ..last.clone() } == Run { t: String::new(), ..run.clone() } => last.t.push_str(t),
            _ => out.push(run),
        }
    };
    for event in Parser::new_ext(text, options) {
        match event {
            Event::Start(Tag::Strong) => b += 1,
            Event::End(TagEnd::Strong) => b -= 1,
            Event::Start(Tag::Emphasis) => i += 1,
            Event::End(TagEnd::Emphasis) => i -= 1,
            Event::Start(Tag::Strikethrough) => s += 1,
            Event::End(TagEnd::Strikethrough) => s -= 1,
            Event::Start(Tag::Link { dest_url, .. }) => links.push(dest_url.into_string()),
            Event::End(TagEnd::Link) => {
                links.pop();
            }
            Event::Text(t) | Event::Html(t) | Event::InlineHtml(t) => push(&mut out, &t, false, b, i, s, &links),
            Event::Code(t) | Event::InlineMath(t) | Event::DisplayMath(t) => push(&mut out, &t, true, b, i, s, &links),
            Event::SoftBreak | Event::HardBreak => push(&mut out, "\n", false, b, i, s, &links),
            Event::End(TagEnd::Paragraph) if !out.is_empty() => push(&mut out, "\n", false, 0, 0, 0, &[]),
            _ => {}
        }
    }
    // No line break after the last paragraph.
    while out.last().is_some_and(|r| r.t.ends_with('\n')) {
        let last = out.last_mut().expect("checked");
        last.t.pop();
        if last.t.is_empty() {
            out.pop();
        }
    }
    out
}

/// How a node is written in the source.
#[derive(Clone, Debug, PartialEq)]
enum Form {
    Heading(usize),
    Item { indent: usize, marker: String },
    Plain,
}

/// A node line before the tree is built.
struct Flat {
    depth: usize,
    line: usize,
    /// Last source line of the node (with continuation lines).
    last: usize,
    form: Form,
    text: String,
}

fn list_item(line: &str) -> Option<(usize, &str, &str)> {
    let indent: usize = line.chars().take_while(|c| c.is_whitespace()).map(|c| if c == '\t' { 4 } else { 1 }).sum();
    let rest = line.trim_start();
    let marker_len = if rest.starts_with(['-', '*', '+']) {
        1
    } else {
        let digits = rest.chars().take_while(char::is_ascii_digit).count();
        if digits == 0 || !rest[digits..].starts_with(['.', ')']) {
            return None;
        }
        digits + 1
    };
    let after = &rest[marker_len..];
    if !(after.is_empty() || after.starts_with([' ', '\t'])) {
        return None;
    }
    Some((indent, &rest[..marker_len], after.trim_start()))
}

fn heading(line: &str) -> Option<(usize, &str)> {
    let level = line.chars().take_while(|c| *c == '#').count();
    if level == 0 || level > 6 {
        return None;
    }
    let rest = &line[level..];
    (rest.is_empty() || rest.starts_with([' ', '\t'])).then(|| (level, rest.trim().trim_end_matches('#').trim_end()))
}

/// The nodes of a source, in order, with their tree depth.
struct Scanned {
    fm: FrontMatter,
    start: usize,
    lines: usize,
    root: Option<Flat>,
    flat: Vec<Flat>,
}

fn scan(text: &str) -> Scanned {
    let lines: Vec<&str> = text.split('\n').map(|l| l.trim_end_matches('\r')).collect();
    let (fm, start) = front_matter(&lines);
    let mut flat: Vec<Flat> = Vec::new();
    let mut root: Option<Flat> = None;
    // Depth of the heading the following list items hang from.
    let mut base = 0;
    // Indentation of the open list levels.
    let mut indents: Vec<usize> = Vec::new();
    let mut fence = false;
    for (n, line) in lines.iter().enumerate().skip(start) {
        let trimmed = line.trim();
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            fence = !fence;
            continue;
        }
        if fence || trimmed.is_empty() || trimmed.starts_with("<!--") {
            continue;
        }
        if let Some((level, title)) = heading(line) {
            indents.clear();
            if level == 1 && root.is_none() {
                root = Some(Flat { depth: 0, line: n, last: n, form: Form::Heading(1), text: title.to_owned() });
                base = 0;
                continue;
            }
            let depth = level.max(2) - 1;
            flat.push(Flat { depth, line: n, last: n, form: Form::Heading(level), text: title.to_owned() });
            base = depth;
            continue;
        }
        if let Some((indent, marker, item)) = list_item(line) {
            while indents.last().is_some_and(|i| *i >= indent) {
                indents.pop();
            }
            indents.push(indent);
            let form = Form::Item { indent, marker: marker.to_owned() };
            flat.push(Flat { depth: base + indents.len(), line: n, last: n, form, text: item.to_owned() });
            continue;
        }
        // Text without a marker: the centre if there is none yet, else a
        // further line of the node above.
        match (flat.last_mut(), &mut root) {
            (Some(f), _) | (None, Some(f)) => {
                f.text.push('\n');
                f.text.push_str(trimmed);
                f.last = n;
            }
            (None, None) => root = Some(Flat { depth: 0, line: n, last: n, form: Form::Plain, text: trimmed.to_owned() }),
        }
    }
    // A node is at most one level below the one before it.
    let mut previous = 0;
    for f in &mut flat {
        f.depth = f.depth.clamp(1, previous + 1);
        previous = f.depth;
    }
    Scanned { fm: fm.unwrap_or_default(), start, lines: lines.len(), root, flat }
}

/// Reads a mind map; `fallback` names the centre when there is no heading.
pub fn parse(text: &str, fallback: &str) -> Map {
    let Scanned { fm, start, root, flat, .. } = scan(text);
    let root_flat = root.unwrap_or(Flat {
        depth: 0,
        line: start,
        last: start,
        form: Form::Plain,
        text: fm.title.clone().filter(|t| !t.trim().is_empty()).unwrap_or_else(|| fallback.to_owned()),
    });
    let mut root = node(&root_flat);
    // Build the tree: a stack of open nodes by depth.
    let mut stack: Vec<Node> = vec![];
    let attach = |stack: &mut Vec<Node>, root: &mut Node| {
        let done = stack.pop().expect("non-empty");
        match stack.last_mut() {
            Some(parent) => parent.children.push(done),
            None => root.children.push(done),
        }
    };
    for f in &flat {
        while stack.len() >= f.depth {
            attach(&mut stack, &mut root);
        }
        stack.push(node(f));
    }
    while !stack.is_empty() {
        attach(&mut stack, &mut root);
    }
    let pick = |v: Option<String>, allowed: &[&str]| v.map(|s| s.trim().to_lowercase()).filter(|s| allowed.contains(&s.as_str()));
    Map {
        title: fm.title.unwrap_or_default(),
        theme: pick(fm.theme, THEMES).unwrap_or_else(|| "default".into()),
        layout: pick(fm.layout, LAYOUTS).unwrap_or_else(|| "both".into()),
        font: fm.font.unwrap_or_default(),
        colors: fm.colors,
        root,
    }
}

/// The next marker of a list: `-` stays, `3.` becomes `4.`.
fn next_marker(marker: &str) -> String {
    match marker.strip_suffix(['.', ')']) {
        Some(n) => match n.parse::<u64>() {
            Ok(v) => format!("{}{}", v + 1, &marker[n.len()..]),
            Err(_) => marker.to_owned(),
        },
        None => marker.to_owned(),
    }
}

/// A new child `child` of the node on `line`: the line to insert it at
/// and its text, written like its siblings (or as a nested list item).
pub fn new_child(text: &str, line: usize, child: &str) -> Option<(usize, String)> {
    let child = child.split_whitespace().collect::<Vec<_>>().join(" ");
    if child.is_empty() {
        return None;
    }
    let sc = scan(text);
    let is_root = sc.root.as_ref().map_or(line == sc.start, |r| r.line == line);
    let (depth, parent, begin) = if is_root {
        (0, sc.root.as_ref(), 0)
    } else {
        let i = sc.flat.iter().position(|f| f.line == line)?;
        (sc.flat[i].depth, Some(&sc.flat[i]), i + 1)
    };
    let end = begin + sc.flat[begin..].iter().take_while(|f| f.depth > depth).count();
    let subtree = &sc.flat[begin..end];
    let last_line = subtree.iter().map(|f| f.last).chain(parent.map(|p| p.last)).max();
    let at = match last_line {
        Some(l) => l + 1,
        // Only a front matter (or nothing): append.
        None => sc.lines,
    };
    let sibling = subtree.iter().rev().find(|f| f.depth == depth + 1);
    let mut prefix = match (sibling, parent.map(|p| &p.form)) {
        (Some(s), _) => match &s.form {
            Form::Heading(level) => format!("{} ", "#".repeat(*level)),
            Form::Item { indent, marker } => format!("{}{} ", " ".repeat(*indent), next_marker(marker)),
            Form::Plain => "- ".into(),
        },
        (None, Some(Form::Item { indent, marker })) => format!("{}- ", " ".repeat(indent + marker.len() + 1)),
        (None, _) => "- ".into(),
    };
    if sibling.is_some_and(|s| s.text.starts_with("[ ] ") || s.text.starts_with("[x] ") || s.text.starts_with("[X] ")) {
        prefix.push_str("[ ] ");
    }
    Some((at, format!("{prefix}{child}")))
}

fn node(f: &Flat) -> Node {
    let (text, color, folded) = attributes(&f.text);
    let (task, text) = match text.get(..4) {
        Some("[ ] ") => (Some(false), &text[4..]),
        Some("[x] " | "[X] ") => (Some(true), &text[4..]),
        _ => (None, text),
    };
    Node { line: f.line, runs: runs(text), color, folded, task, children: Vec::new() }
}

/// A new mind map.
pub fn template() -> String {
    match crate::i18n::lang() {
        crate::i18n::Lang::De => "---\nmindmap: 1\ntheme: default\nlayout: both\n---\n# Projektidee\n- Ziele {color: #1c71d8}\n  - **Hauptziel**\n  - Wäre schön\n- Aufgaben\n  - [x] Skizze\n  - [ ] Recherche\n- Beteiligte\n  - Anna\n  - Ben\n- Notizen {folded}\n  - Details mit `Code` und [Links](https://example.org)\n".into(),
        crate::i18n::Lang::En => "---\nmindmap: 1\ntheme: default\nlayout: both\n---\n# Project idea\n- Goals {color: #1c71d8}\n  - **Main goal**\n  - Nice to have\n- Tasks\n  - [x] Sketch\n  - [ ] Research\n- People\n  - Anna\n  - Ben\n- Notes {folded}\n  - Details with `code` and [links](https://example.org)\n".into(),
    }
}

fn json_for_script<T: Serialize>(value: &T) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "null".into()).replace("</", "<\\/")
}

/// The preview page; maps are sent with `QM.update(map)`.
pub fn page(dark: bool, vars: &str) -> String {
    shell(dark, vars, "QM.init({interactive: true});")
}

/// A page that draws `map` once, for the exports (see [`EXPORT_SCRIPT`]).
/// The default theme is drawn light.
pub fn export_page(map: &Map) -> String {
    shell(false, "", &format!("QM.init({{interactive: false}}); QM.update({});", json_for_script(map)))
}

/// Returns `{"svg": "...", "width": w, "height": h}` from an export page.
pub const EXPORT_SCRIPT: &str = "JSON.stringify(QM.exportSvg())";

fn shell(dark: bool, vars: &str, init: &str) -> String {
    let strings = json_for_script(&serde_json::json!({
        "zoomIn": t!("Zoom in (+)"),
        "zoomOut": t!("Zoom out (−)"),
        "fit": t!("Fit to view (0)"),
        "fold": t!("Fold or unfold"),
        "jump": t!("Double-click: go to the line"),
        "add": t!("Add a branch"),
        "newBranch": t!("New branch – Enter adds it, Esc closes"),
    }));
    let theme = if dark { "dark" } else { "light" };
    format!(
        r#"<!DOCTYPE html>
<html data-theme="{theme}" style="{vars}">
<head>
<meta charset="utf-8">
<style>{MAP_CSS}</style>
</head>
<body>
<div id="map"></div>
<script>window.QM_STRINGS = {strings};</script>
<script>{MAP_JS}</script>
<script>{init}</script>
</body>
</html>"#
    )
}

/// An A4 page (turned to suit the map) with the map's SVG, for the PDF.
/// Sizes are absolute: the off-screen print view has no viewport height.
pub fn pdf_page(svg: &str, background: &str, width_mm: f64, height_mm: f64) -> String {
    format!(
        r#"<!DOCTYPE html>
<html><head><meta charset="utf-8"><style>
@page {{ size: {width_mm}mm {height_mm}mm; margin: 0; }}
html, body {{ margin: 0; background: {background}; -webkit-print-color-adjust: exact; print-color-adjust: exact; }}
.page {{ box-sizing: border-box; width: {width_mm}mm; height: {h}mm; padding: 10mm; overflow: hidden;
  display: flex; align-items: center; justify-content: center; }}
.page svg {{ display: block; width: 100%; height: 100%; }}
</style></head>
<body><div class="page">{svg}</div></body></html>"#,
        h = height_mm - 0.5
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn texts(node: &Node) -> String {
        node.runs.iter().map(|r| r.t.as_str()).collect()
    }

    #[test]
    fn outline_becomes_a_tree() {
        let map = parse(
            "---\nmindmap: 1\ntheme: Pastel\nlayout: down\n---\n# Centre\n- A {color: #e01b24}\n  - A1\n    - A1a\n  - A2\n- B {folded}\n\n## C\n- C1\n  more text\n1. C2\n",
            "x",
        );
        assert_eq!(map.theme, "pastel");
        assert_eq!(map.layout, "down");
        assert_eq!(texts(&map.root), "Centre");
        let kids: Vec<String> = map.root.children.iter().map(texts).collect();
        assert_eq!(kids, ["A", "B", "C"]);
        let a = &map.root.children[0];
        assert_eq!(a.color.as_deref(), Some("#e01b24"));
        assert_eq!(a.children.len(), 2);
        assert_eq!(texts(&a.children[0].children[0]), "A1a");
        assert!(map.root.children[1].folded);
        let c = &map.root.children[2];
        assert_eq!(c.children.iter().map(texts).collect::<Vec<_>>(), ["C1\nmore text", "C2"]);
        assert_eq!(c.line, 12);
    }

    #[test]
    fn markdown_runs() {
        let r = runs("**bold** and *it* `code` ~~gone~~ [link](https://x.org)");
        assert_eq!(r[0], Run { t: "bold".into(), b: true, ..Default::default() });
        assert!(r.iter().any(|x| x.i && x.t == "it"));
        assert!(r.iter().any(|x| x.c && x.t == "code"));
        assert!(r.iter().any(|x| x.s && x.t == "gone"));
        assert!(r.iter().any(|x| x.href.as_deref() == Some("https://x.org") && x.t == "link"));
    }

    #[test]
    fn tasks_attributes_and_fallbacks() {
        let map = parse("- [x] done {#33d17a}\n- [ ] open\n", "My map");
        assert_eq!(texts(&map.root), "My map");
        assert_eq!(map.root.children[0].task, Some(true));
        assert_eq!(map.root.children[0].color.as_deref(), Some("#33d17a"));
        assert_eq!(texts(&map.root.children[1]), "open");
        // Braces that are no attributes stay text.
        let map = parse("# Set {a, b}\n", "x");
        assert_eq!(texts(&map.root), "Set {a, b}");
        // Plain first line as centre, deeper levels clamped.
        let map = parse("Topic\n    - deep\n", "x");
        assert_eq!(texts(&map.root), "Topic");
        assert_eq!(texts(&map.root.children[0]), "deep");
        assert!(is_mindmap("---\nmindmap: 1\n---\n# x"));
        assert!(!is_mindmap("# x\n- y"));
    }

    #[test]
    fn new_children_follow_their_siblings() {
        let src = "---\nmindmap: 1\n---\n# Centre\n- A\n  - A1\n    more\n- B\n\n## C\n1. C1\n";
        // Under the centre: after everything, like the last direct child (a heading).
        assert_eq!(new_child(src, 3, "D"), Some((11, "## D".into())));
        // A list item with children: after its last child, same indent.
        assert_eq!(new_child(src, 4, "A2"), Some((7, "  - A2".into())));
        // A leaf: nested one level deeper.
        assert_eq!(new_child(src, 5, "x  y"), Some((7, "    - x y".into())));
        assert_eq!(new_child(src, 7, "B1"), Some((8, "  - B1".into())));
        // Numbered lists count on.
        assert_eq!(new_child(src, 9, "C2"), Some((11, "2. C2".into())));
        // Tasks get a checkbox.
        assert_eq!(new_child("# T\n- [x] a\n", 0, "b"), Some((2, "- [ ] b".into())));
        // No centre: the new branch is appended.
        assert_eq!(new_child("---\nmindmap: 1\n---", 3, "first"), Some((3, "- first".into())));
        assert_eq!(new_child(src, 4, "  "), None);
        // The result parses where expected.
        let (at, line) = new_child(src, 4, "A2").unwrap();
        let mut lines: Vec<&str> = src.split('\n').collect();
        lines.insert(at, &line);
        let map = parse(&lines.join("\n"), "x");
        assert_eq!(map.root.children[0].children.len(), 2);
    }

    #[test]
    fn template_parses() {
        let map = parse(&template(), "x");
        assert_eq!(map.root.children.len(), 4);
        assert!(map.root.children[3].folded);
    }
}
