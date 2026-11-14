//! Note templates with a Twig-like syntax.
//!
//! - `{{ field }}` inserts a value: `date`, `time`, `datetime`, `weekday`,
//!   `week`, `year`, `month`, `day`, `title`, `folder`, `user`, `clipboard`,
//!   `selection`, `cursor` (where the cursor goes) and `ask "Label"` (asked
//!   for before inserting, optionally with a default: `ask "Label" "x"`).
//!   `date("tomorrow")`, `date("+3 days")`, `date("next monday")` move the date.
//! - Filters: `{{ date|format("%d %B %Y") }}`, `upper`, `lower`, `trim`,
//!   `default("…")`.
//! - `{% include "Name" %}` inserts another template (a building block).
//! - `{% extends "Base" %}` with `{% block name %}…{% endblock %}` replaces
//!   the base's blocks; `{{ parent() }}` keeps the base's content.
//! - `{% if field %}…{% else %}…{% endif %}`, `{# comment #}` and
//!   `{% raw %}…{% endraw %}`.
//!
//! A line holding only a `{% … %}` or `{# … #}` tag leaves no empty line.

use jiff::civil::Weekday;
use jiff::{ToSpan, Zoned};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};

/// Where the cursor goes in the output (removed before inserting).
const CURSOR: char = '\u{E000}';
const MAX_DEPTH: usize = 12;

/// Finds templates by name.
pub trait Lookup {
    fn template(&self, name: &str) -> Option<String>;
}

/// Values of the fields.
#[derive(Clone, Debug, Default)]
pub struct Fields {
    pub now: Option<Zoned>,
    pub title: String,
    pub folder: String,
    pub user: String,
    pub clipboard: String,
    pub selection: String,
    /// Answers to `ask` questions by label.
    pub answers: HashMap<String, String>,
    /// German date and weekday names.
    pub german: bool,
    /// Date format of the locale (strftime).
    pub date_format: String,
}

/// A note created from a template: the template, the field values it was
/// filled in with and the text that came out (for later updates).
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(default)]
pub struct Link {
    /// The template note's id and (as a fallback) its name.
    pub id: String,
    pub name: String,
    pub now: String,
    pub title: String,
    pub folder: String,
    pub user: String,
    pub clipboard: String,
    pub selection: String,
    pub answers: BTreeMap<String, String>,
    pub german: bool,
    pub date_format: String,
    /// The filled-in template as last merged into the note.
    pub base: String,
}

impl Link {
    pub fn new(id: &str, name: &str, fields: &Fields, base: &str) -> Self {
        Self {
            id: id.to_owned(),
            name: name.to_owned(),
            now: fields.now.as_ref().map(ToString::to_string).unwrap_or_default(),
            title: fields.title.clone(),
            folder: fields.folder.clone(),
            user: fields.user.clone(),
            clipboard: fields.clipboard.clone(),
            selection: fields.selection.clone(),
            answers: fields.answers.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
            german: fields.german,
            date_format: fields.date_format.clone(),
            base: base.to_owned(),
        }
    }

    /// The fields as they were when the note was created.
    pub fn fields(&self) -> Fields {
        Fields {
            now: self.now.parse().ok(),
            title: self.title.clone(),
            folder: self.folder.clone(),
            user: self.user.clone(),
            clipboard: self.clipboard.clone(),
            selection: self.selection.clone(),
            answers: self.answers.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
            german: self.german,
            date_format: self.date_format.clone(),
        }
    }
}

// ----- merging template changes ---------------------------------------------

/// A changed range: `base[a0..a1]` became `other[b0..b1]` (in lines).
#[derive(Clone, Copy, Debug, PartialEq)]
struct Hunk {
    a0: usize,
    a1: usize,
    b0: usize,
    b1: usize,
}

/// Larger texts are not merged (the diff is quadratic).
const MAX_CELLS: usize = 6_000_000;

/// Line differences between `a` and `b` (longest common subsequence).
fn diff(a: &[&str], b: &[&str]) -> Option<Vec<Hunk>> {
    // Common head and tail first: templates usually change in one place.
    let head = a.iter().zip(b).take_while(|(x, y)| x == y).count();
    let tail = a[head..].iter().rev().zip(b[head..].iter().rev()).take_while(|(x, y)| x == y).count();
    let (am, bm) = (&a[head..a.len() - tail], &b[head..b.len() - tail]);
    let (n, m) = (am.len(), bm.len());
    if n.saturating_mul(m) > MAX_CELLS {
        return None;
    }
    // lcs[i][j]: length of the common subsequence of am[i..] and bm[j..].
    let mut lcs = vec![0u32; (n + 1) * (m + 1)];
    let at = |i: usize, j: usize| i * (m + 1) + j;
    for i in (0..n).rev() {
        for j in (0..m).rev() {
            lcs[at(i, j)] = if am[i] == bm[j] { lcs[at(i + 1, j + 1)] + 1 } else { lcs[at(i + 1, j)].max(lcs[at(i, j + 1)]) };
        }
    }
    let mut hunks = Vec::new();
    let (mut i, mut j) = (0, 0);
    let mut open: Option<(usize, usize)> = None;
    while i < n || j < m {
        if i < n && j < m && am[i] == bm[j] {
            if let Some((a0, b0)) = open.take() {
                hunks.push(Hunk { a0: a0 + head, a1: i + head, b0: b0 + head, b1: j + head });
            }
            i += 1;
            j += 1;
        } else {
            open.get_or_insert((i, j));
            if j < m && (i == n || lcs[at(i, j + 1)] >= lcs[at(i + 1, j)]) {
                j += 1;
            } else {
                i += 1;
            }
        }
    }
    if let Some((a0, b0)) = open {
        hunks.push(Hunk { a0: a0 + head, a1: n + head, b0: b0 + head, b1: m + head });
    }
    Some(hunks)
}

/// Applies the changes from `base` to `theirs` to `ours`, except where
/// `ours` changed the same lines (there `ours` wins). `None` when the texts
/// are too large to compare.
pub fn merge(base: &str, theirs: &str, ours: &str) -> Option<String> {
    let b: Vec<&str> = base.split_inclusive('\n').collect();
    let t: Vec<&str> = theirs.split_inclusive('\n').collect();
    let o: Vec<&str> = ours.split_inclusive('\n').collect();
    let th = diff(&b, &t)?;
    let oh = diff(&b, &o)?;
    // Changes of both sides, grouped where they change the same lines (or
    // insert at the same place). Changes on neighbouring lines stay apart:
    // a note's text right below a template's heading must not block an
    // update of the heading.
    let mut all: Vec<(Hunk, bool)> = th.iter().map(|h| (*h, true)).chain(oh.iter().map(|h| (*h, false))).collect();
    all.sort_by_key(|(h, _)| (h.a0, h.a1));
    let clash = |g: &Hunk, h: &Hunk| {
        let inside = |p: usize, r: &Hunk| r.a0 < p && p < r.a1;
        (g.a0 < h.a1 && h.a0 < g.a1)
            || (g.a0 == g.a1 && h.a0 == h.a1 && g.a0 == h.a0)
            || (g.a0 == g.a1 && inside(g.a0, h))
            || (h.a0 == h.a1 && inside(h.a0, g))
    };
    let mut chunks: Vec<Vec<(Hunk, bool)>> = Vec::new();
    for (h, theirs_side) in all {
        match chunks.last_mut() {
            Some(c) if c.iter().any(|(g, _)| clash(g, &h)) => c.push((h, theirs_side)),
            _ => chunks.push(vec![(h, theirs_side)]),
        }
    }
    // Position in a side's text of a base position, given that side's
    // changes before it.
    let shift = |hunks: &[Hunk], before: usize| -> isize {
        hunks.iter().filter(|h| h.a0 < before && h.a1 <= before).map(|h| (h.b1 - h.b0) as isize - (h.a1 - h.a0) as isize).sum()
    };
    let mut out = String::new();
    let mut pos = 0;
    for chunk in &chunks {
        let s = chunk.iter().map(|(h, _)| h.a0).min().unwrap_or(0);
        let e = chunk.iter().map(|(h, _)| h.a1).max().unwrap_or(s);
        out.extend(b[pos..s].iter().copied());
        let ours_changed = chunk.iter().any(|(_, t)| !t);
        let (lines, hunks) = if ours_changed { (&o, &oh) } else { (&t, &th) };
        let inside: isize = chunk.iter().filter(|(_, t)| *t != ours_changed).map(|(h, _)| (h.b1 - h.b0) as isize - (h.a1 - h.a0) as isize).sum();
        let from = (s as isize + shift(hunks, s)) as usize;
        let to = (e as isize + shift(hunks, s) + inside) as usize;
        out.extend(lines[from..to].iter().copied());
        pos = e;
    }
    out.extend(b[pos..].iter().copied());
    Some(out)
}

/// A question of an `ask` field.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Question {
    pub label: String,
    pub default: String,
}

/// The filled-in template and where the cursor goes (byte offset).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Output {
    pub text: String,
    pub cursor: Option<usize>,
}

// ----- parsing ----------------------------------------------------------------

#[derive(Clone, Debug, PartialEq)]
enum Node {
    Text(String),
    Print(Expr),
    Include(String),
    Block(String, Vec<Node>),
    If(Expr, Vec<Node>, Vec<Node>),
}

#[derive(Clone, Debug, PartialEq)]
struct Expr {
    base: Base,
    filters: Vec<(String, Vec<String>)>,
}

#[derive(Clone, Debug, PartialEq)]
enum Base {
    Field(String, Vec<String>),
    Text(String),
    Ask(String, String),
    Parent,
}

struct Parsed {
    extends: Option<String>,
    nodes: Vec<Node>,
}

/// Splits the source into text and tags; lines that only hold a statement
/// or comment tag are dropped entirely.
fn tokens(src: &str) -> Result<Vec<(bool, String)>, String> {
    let mut out: Vec<(bool, String)> = Vec::new();
    let mut rest = src;
    let mut text = String::new();
    while !rest.is_empty() {
        let Some(at) = rest.find('{') else {
            text.push_str(rest);
            break;
        };
        let (before, tail) = rest.split_at(at);
        text.push_str(before);
        let close = if tail.starts_with("{{") {
            "}}"
        } else if tail.starts_with("{%") {
            "%}"
        } else if tail.starts_with("{#") {
            "#}"
        } else {
            text.push('{');
            rest = &tail[1..];
            continue;
        };
        let end = tail[2..].find(close).ok_or_else(|| t!("Unclosed tag: {}", tail.chars().take(20).collect::<String>()))? + 2;
        let inner = &tail[2..end];
        let tag_len = end + 2;
        if close != "}}" {
            // A tag alone on its line takes the line with it.
            let line_before = text.rsplit('\n').next().unwrap_or(&text);
            let after = &tail[tag_len..];
            let line_after = after.split('\n').next().unwrap_or(after);
            if line_before.trim().is_empty() && line_after.trim().is_empty() && after.contains('\n') {
                let keep = text.len() - line_before.len();
                text.truncate(keep);
                rest = &after[line_after.len() + 1..];
            } else if line_before.trim().is_empty() && line_after.trim().is_empty() && !text.is_empty() {
                let keep = text.len() - line_before.len();
                text.truncate(keep);
                rest = &after[line_after.len()..];
            } else {
                rest = after;
            }
        } else {
            rest = &tail[tag_len..];
        }
        if close == "#}" {
            continue;
        }
        if !text.is_empty() {
            out.push((false, std::mem::take(&mut text)));
        }
        let kind = if close == "}}" { "=" } else { "%" };
        out.push((true, format!("{kind}{}", inner.trim())));
        // `{% raw %}`: everything up to `{% endraw %}` is text.
        if close == "%}" && inner.trim() == "raw" {
            let Some(end) = rest.find("{% endraw %}").or_else(|| rest.find("{%endraw%}")) else {
                return Err(t!("Missing {% endraw %}").into());
            };
            out.pop();
            let raw = &rest[..end];
            out.push((false, raw.strip_prefix('\n').unwrap_or(raw).to_owned()));
            rest = &rest[end..];
            rest = &rest[rest.find("%}").map_or(rest.len(), |i| i + 2)..];
            rest = rest.strip_prefix('\n').unwrap_or(rest);
        }
    }
    if !text.is_empty() {
        out.push((false, text));
    }
    Ok(out)
}

/// A string literal at the start of `s` and the rest.
fn string_lit(s: &str) -> Option<(String, &str)> {
    let s = s.trim_start();
    let q = s.chars().next().filter(|c| *c == '"' || *c == '\'')?;
    let end = s[1..].find(q)? + 1;
    Some((s[1..end].to_owned(), &s[end + 1..]))
}

fn ident(s: &str) -> (&str, &str) {
    let s = s.trim_start();
    let end = s.find(|c: char| !(c.is_alphanumeric() || c == '_')).unwrap_or(s.len());
    (&s[..end], &s[end..])
}

/// `( "a", "b" )` at the start of `s`.
fn args(s: &str) -> Result<(Vec<String>, &str), String> {
    let s = s.trim_start();
    let Some(mut rest) = s.strip_prefix('(') else { return Ok((Vec::new(), s)) };
    let mut out = Vec::new();
    loop {
        rest = rest.trim_start();
        if let Some(r) = rest.strip_prefix(')') {
            return Ok((out, r));
        }
        if let Some((v, r)) = string_lit(rest) {
            out.push(v);
            rest = r.trim_start();
        } else {
            let end = rest.find([',', ')']).ok_or_else(|| t!("Missing ')'"))?;
            out.push(rest[..end].trim().to_owned());
            rest = &rest[end..];
        }
        rest = rest.strip_prefix(',').unwrap_or(rest);
    }
}

fn parse_expr(s: &str) -> Result<Expr, String> {
    let mut parts = split_filters(s).into_iter();
    let head = parts.next().unwrap_or_default();
    let head = head.trim();
    let base = if let Some((text, _)) = string_lit(head) {
        Base::Text(text)
    } else {
        let (name, rest) = ident(head);
        match name {
            "" => return Err(t!("Empty field").into()),
            "ask" => {
                let (label, rest) = string_lit(rest).ok_or_else(|| t!("ask needs a label in quotes"))?;
                let default = string_lit(rest).map(|(d, _)| d).unwrap_or_default();
                Base::Ask(label, default)
            }
            "parent" => Base::Parent,
            _ => {
                let (a, _) = args(rest)?;
                Base::Field(name.to_owned(), a)
            }
        }
    };
    let mut filters = Vec::new();
    for f in parts {
        let (name, rest) = ident(&f);
        let (a, _) = args(rest)?;
        filters.push((name.to_owned(), a));
    }
    Ok(Expr { base, filters })
}

/// Splits at `|` outside of quotes.
fn split_filters(s: &str) -> Vec<String> {
    let mut out = vec![String::new()];
    let mut quote: Option<char> = None;
    for c in s.chars() {
        match (quote, c) {
            (None, '|') => out.push(String::new()),
            (None, '"' | '\'') => {
                quote = Some(c);
                out.last_mut().expect("part").push(c);
            }
            (Some(q), c) if c == q => {
                quote = None;
                out.last_mut().expect("part").push(c);
            }
            _ => out.last_mut().expect("part").push(c),
        }
    }
    out
}

fn parse(src: &str) -> Result<Parsed, String> {
    let toks = tokens(src)?;
    let mut extends = None;
    let mut pos = 0;
    let nodes = parse_nodes(&toks, &mut pos, &mut extends, None)?;
    Ok(Parsed { extends, nodes })
}

/// Parses until `{% end… %}` (or `{% else %}` inside an if).
fn parse_nodes(toks: &[(bool, String)], pos: &mut usize, extends: &mut Option<String>, until: Option<&str>) -> Result<Vec<Node>, String> {
    let mut out = Vec::new();
    while *pos < toks.len() {
        let (tag, text) = &toks[*pos];
        *pos += 1;
        if !tag {
            out.push(Node::Text(text.clone()));
            continue;
        }
        if let Some(expr) = text.strip_prefix('=') {
            out.push(Node::Print(parse_expr(expr)?));
            continue;
        }
        let stmt = &text[1..];
        let (word, rest) = ident(stmt);
        match word {
            "include" => {
                let (name, _) = string_lit(rest).ok_or_else(|| t!("include needs a template name in quotes"))?;
                out.push(Node::Include(name));
            }
            "extends" => {
                let (name, _) = string_lit(rest).ok_or_else(|| t!("extends needs a template name in quotes"))?;
                *extends = Some(name);
            }
            "block" => {
                let (name, _) = ident(rest);
                let body = parse_nodes(toks, pos, extends, Some("endblock"))?;
                out.push(Node::Block(name.to_owned(), body));
            }
            "if" => {
                let cond = parse_expr(rest)?;
                let then = parse_nodes(toks, pos, extends, Some("endif"))?;
                // `parse_nodes` stopped at `else` or `endif`.
                let stopped_at_else = toks.get(*pos - 1).is_some_and(|(_, t)| t.trim_start_matches('%').trim() == "else");
                let otherwise = if stopped_at_else { parse_nodes(toks, pos, extends, Some("endif"))? } else { Vec::new() };
                out.push(Node::If(cond, then, otherwise));
            }
            "else" if until == Some("endif") => return Ok(out),
            w if Some(w) == until => return Ok(out),
            "endblock" | "endif" | "else" => return Err(t!("Unexpected {% {} %}", word)),
            _ => return Err(t!("Unknown tag {% {} %}", word)),
        }
    }
    match until {
        Some(end) => Err(t!("Missing {% {} %}", end)),
        None => Ok(out),
    }
}

// ----- rendering ----------------------------------------------------------------

struct Renderer<'a> {
    lookup: &'a dyn Lookup,
    fields: &'a Fields,
    /// Collected questions (when only asking what to ask).
    asked: RefCell<Vec<Question>>,
}

fn blocks_of(nodes: &[Node], out: &mut HashMap<String, Vec<Node>>) {
    for n in nodes {
        match n {
            Node::Block(name, body) => {
                out.entry(name.clone()).or_insert_with(|| body.clone());
                blocks_of(body, out);
            }
            Node::If(_, a, b) => {
                blocks_of(a, out);
                blocks_of(b, out);
            }
            _ => {}
        }
    }
}

impl Renderer<'_> {
    fn template(&self, name: &str, depth: usize) -> Result<String, String> {
        if depth > MAX_DEPTH {
            return Err(t!("Templates include each other too deeply").into());
        }
        let src = self.lookup.template(name).ok_or_else(|| t!("Template “{}” not found", name))?;
        self.render_src(&src, depth)
    }

    /// Renders a template, resolving `extends`: the chain's blocks, most
    /// derived first, fill the root template.
    fn render_src(&self, src: &str, depth: usize) -> Result<String, String> {
        let mut parsed = parse(src)?;
        let mut chain: Vec<HashMap<String, Vec<Node>>> = Vec::new();
        let mut level = depth;
        while let Some(base) = parsed.extends.take() {
            level += 1;
            if level > MAX_DEPTH + depth {
                return Err(t!("Templates include each other too deeply").into());
            }
            let mut blocks = HashMap::new();
            blocks_of(&parsed.nodes, &mut blocks);
            chain.push(blocks);
            let base_src = self.lookup.template(&base).ok_or_else(|| t!("Template “{}” not found", base))?;
            parsed = parse(&base_src)?;
        }
        let mut out = String::new();
        self.nodes(&parsed.nodes, &chain, None, depth, &mut out)?;
        Ok(out)
    }

    /// `parent`: the content of the block being overridden (for `parent()`).
    fn nodes(&self, nodes: &[Node], chain: &[HashMap<String, Vec<Node>>], parent: Option<(&[Node], usize)>, depth: usize, out: &mut String) -> Result<(), String> {
        for n in nodes {
            match n {
                Node::Text(t) => out.push_str(t),
                Node::Print(e) => {
                    if e.base == Base::Parent {
                        if let Some((body, level)) = parent {
                            self.nodes(body, &chain[level.min(chain.len())..], None, depth, out)?;
                        }
                        continue;
                    }
                    out.push_str(&self.expr(e)?);
                }
                Node::Include(name) => out.push_str(&self.template(name, depth + 1)?),
                Node::Block(name, body) => {
                    // The most derived override wins; `parent()` there means this body.
                    match chain.iter().enumerate().find_map(|(i, c)| c.get(name).map(|b| (i, b))) {
                        Some((i, child)) => {
                            let child = child.clone();
                            // The parent of level i is level i+1 (or this base body).
                            let parent_body: Vec<Node> = chain[i + 1..].iter().find_map(|c| c.get(name).cloned()).unwrap_or_else(|| body.clone());
                            self.nodes(&child, chain, Some((&parent_body, i + 1)), depth, out)?;
                        }
                        None => self.nodes(body, chain, None, depth, out)?,
                    }
                }
                Node::If(cond, then, otherwise) => {
                    let value = self.expr(cond)?;
                    let branch = if value.trim().is_empty() || value == "false" || value == "0" { otherwise } else { then };
                    self.nodes(branch, chain, parent, depth, out)?;
                }
            }
        }
        Ok(())
    }

    fn expr(&self, e: &Expr) -> Result<String, String> {
        let mut date: Option<(Zoned, String)> = None;
        let mut value = match &e.base {
            Base::Text(t) => t.clone(),
            Base::Parent => String::new(),
            Base::Ask(label, default) => {
                let mut asked = self.asked.borrow_mut();
                if !asked.iter().any(|q| &q.label == label) {
                    asked.push(Question { label: label.clone(), default: default.clone() });
                }
                self.fields.answers.get(label).cloned().unwrap_or_else(|| default.clone())
            }
            Base::Field(name, args) => {
                let now = self.fields.now.clone().unwrap_or_else(Zoned::now);
                match name.as_str() {
                    "date" => {
                        let d = shift(&now, args.first().map(String::as_str).unwrap_or(""))?;
                        date = Some((d, self.fields.date_format.clone()));
                        String::new()
                    }
                    "time" => {
                        date = Some((now, "%H:%M".into()));
                        String::new()
                    }
                    "datetime" => {
                        date = Some((now, String::new())); // date format + time
                        String::new()
                    }
                    "weekday" => weekday_name(now.weekday(), self.fields.german).to_owned(),
                    "week" => now.strftime("%V").to_string().trim_start_matches('0').to_owned(),
                    "year" => now.year().to_string(),
                    "month" => format!("{:02}", now.month()),
                    "day" => format!("{:02}", now.day()),
                    "title" => self.fields.title.clone(),
                    "folder" => self.fields.folder.clone(),
                    "user" => self.fields.user.clone(),
                    "clipboard" => self.fields.clipboard.clone(),
                    "selection" => self.fields.selection.clone(),
                    "cursor" => CURSOR.to_string(),
                    other => return Err(t!("Unknown field “{}”", other)),
                }
            }
        };
        for (name, args) in &e.filters {
            match name.as_str() {
                "format" => {
                    let f = args.first().cloned().unwrap_or_default();
                    if let Some((_, fmt)) = &mut date {
                        *fmt = f;
                    }
                }
                "upper" | "lower" | "trim" | "default" | "capitalize" => {
                    if let Some((d, fmt)) = date.take() {
                        value = self.format_date(&d, &fmt);
                    }
                    value = match name.as_str() {
                        "upper" => value.to_uppercase(),
                        "lower" => value.to_lowercase(),
                        "trim" => value.trim().to_owned(),
                        "capitalize" => {
                            let mut c = value.chars();
                            c.next().map(|f| f.to_uppercase().collect::<String>() + c.as_str()).unwrap_or_default()
                        }
                        _ if value.is_empty() => args.first().cloned().unwrap_or_default(),
                        _ => value,
                    };
                }
                other => return Err(t!("Unknown filter “{}”", other)),
            }
        }
        if let Some((d, fmt)) = date {
            value = self.format_date(&d, &fmt);
        }
        Ok(value)
    }

    fn format_date(&self, d: &Zoned, fmt: &str) -> String {
        let fmt = if fmt.is_empty() { format!("{} %H:%M", self.fields.date_format) } else { fmt.to_owned() };
        format_date(d, &fmt, self.fields.german)
    }
}

/// `today`, `tomorrow`, `yesterday`, `+3 days`, `-1 week`, `next monday`, `last friday`.
fn shift(now: &Zoned, spec: &str) -> Result<Zoned, String> {
    let s = spec.trim().to_lowercase();
    let bad = || t!("Unknown date “{}”", spec);
    let add = |n: i64, unit: &str| -> Result<Zoned, String> {
        let span = match unit.trim_end_matches('s') {
            "day" => n.days(),
            "week" => n.weeks(),
            "month" => n.months(),
            "year" => n.years(),
            _ => return Err(bad()),
        };
        now.checked_add(span).map_err(|e| e.to_string())
    };
    match s.as_str() {
        "" | "today" => return Ok(now.clone()),
        "tomorrow" => return add(1, "day"),
        "yesterday" => return add(-1, "day"),
        _ => {}
    }
    let mut words = s.split_whitespace();
    let first = words.next().unwrap_or("");
    let second = words.next().unwrap_or("");
    if let Ok(n) = first.parse::<i64>() {
        return add(n, second);
    }
    if first == "next" || first == "last" {
        let target = weekday(second).ok_or_else(bad)?;
        let step = if first == "next" { 1 } else { -1 };
        let mut d = add(step, "day")?;
        for _ in 0..7 {
            if d.weekday() == target {
                return Ok(d);
            }
            d = d.checked_add(step.days()).map_err(|e| e.to_string())?;
        }
    }
    if let Some(target) = weekday(first) {
        // The coming one (today counts).
        let mut d = now.clone();
        for _ in 0..7 {
            if d.weekday() == target {
                return Ok(d);
            }
            d = d.checked_add(1.days()).map_err(|e| e.to_string())?;
        }
    }
    Err(bad())
}

fn weekday(s: &str) -> Option<Weekday> {
    Some(match s {
        "monday" | "mon" => Weekday::Monday,
        "tuesday" | "tue" => Weekday::Tuesday,
        "wednesday" | "wed" => Weekday::Wednesday,
        "thursday" | "thu" => Weekday::Thursday,
        "friday" | "fri" => Weekday::Friday,
        "saturday" | "sat" => Weekday::Saturday,
        "sunday" | "sun" => Weekday::Sunday,
        _ => return None,
    })
}

const DAYS_DE: [&str; 7] = ["Montag", "Dienstag", "Mittwoch", "Donnerstag", "Freitag", "Samstag", "Sonntag"];
const MONTHS_DE: [&str; 12] = ["Januar", "Februar", "März", "April", "Mai", "Juni", "Juli", "August", "September", "Oktober", "November", "Dezember"];

fn weekday_name(w: Weekday, german: bool) -> &'static str {
    let i = w.to_monday_zero_offset() as usize;
    if german {
        DAYS_DE[i]
    } else {
        ["Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday", "Sunday"][i]
    }
}

/// strftime with German day and month names when wanted.
pub fn format_date(d: &Zoned, fmt: &str, german: bool) -> String {
    let fmt = if german {
        let i = d.weekday().to_monday_zero_offset() as usize;
        let m = d.month() as usize - 1;
        fmt.replace("%A", DAYS_DE[i])
            .replace("%a", &DAYS_DE[i][..2])
            .replace("%B", MONTHS_DE[m])
            .replace("%b", &MONTHS_DE[m].chars().take(3).collect::<String>())
    } else {
        fmt.to_owned()
    };
    // Literal text must not look like a directive to strftime.
    let mut out = String::new();
    let mut spec = String::new();
    let mut chars = fmt.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '%' {
            spec.push(c);
            while let Some(&n) = chars.peek() {
                spec.push(n);
                chars.next();
                if n.is_ascii_alphabetic() || n == '%' {
                    break;
                }
            }
            out.push_str(&d.strftime(&spec).to_string());
            spec.clear();
        } else {
            out.push(c);
        }
    }
    out
}

/// The date format of the locale (`LC_TIME`, `LC_ALL`, `LANG`).
pub fn locale_date_format() -> String {
    let locale = ["LC_ALL", "LC_TIME", "LANG"].iter().filter_map(|v| std::env::var(v).ok()).find(|v| !v.is_empty()).unwrap_or_default();
    let lang = locale.split(['.', '@']).next().unwrap_or("");
    match lang {
        l if l.starts_with("de") || l.starts_with("ru") || l.starts_with("pl") || l.starts_with("cs") || l.starts_with("fi") || l.starts_with("nb") => "%d.%m.%Y",
        "en_US" => "%m/%d/%Y",
        l if l.starts_with("en") || l.starts_with("fr") || l.starts_with("es") || l.starts_with("it") || l.starts_with("pt") => "%d/%m/%Y",
        l if l.starts_with("nl") => "%d-%m-%Y",
        _ => "%Y-%m-%d",
    }
    .to_owned()
}

/// The questions (`ask` fields) of a template and the templates it uses.
pub fn questions(src: &str, lookup: &dyn Lookup, fields: &Fields) -> Result<Vec<Question>, String> {
    let r = Renderer { lookup, fields, asked: RefCell::new(Vec::new()) };
    r.render_src(src, 0)?;
    Ok(r.asked.into_inner())
}

/// Fills in a template.
pub fn render(src: &str, lookup: &dyn Lookup, fields: &Fields) -> Result<Output, String> {
    let r = Renderer { lookup, fields, asked: RefCell::new(Vec::new()) };
    let text = r.render_src(src, 0)?;
    let cursor = text.find(CURSOR);
    Ok(Output { text: text.replace(CURSOR, ""), cursor })
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Lib(Vec<(&'static str, &'static str)>);

    impl Lookup for Lib {
        fn template(&self, name: &str) -> Option<String> {
            self.0.iter().find(|(n, _)| n.eq_ignore_ascii_case(name)).map(|(_, t)| t.to_string())
        }
    }

    fn fields() -> Fields {
        let now: Zoned = "2026-11-14T09:30:00+01:00[Europe/Berlin]".parse().unwrap();
        Fields {
            now: Some(now),
            title: "Notes".into(),
            folder: "Work".into(),
            user: "Kim".into(),
            clipboard: "clip".into(),
            selection: String::new(),
            answers: HashMap::from([("Topic".to_string(), "Budget".to_string())]),
            german: false,
            date_format: "%Y-%m-%d".into(),
        }
    }

    fn run(src: &str, lib: &Lib) -> String {
        render(src, lib, &fields()).unwrap().text
    }

    #[test]
    fn fields_and_filters() {
        let lib = Lib(vec![]);
        assert_eq!(run("{{ date }} {{ time }} {{ weekday }} W{{ week }}", &lib), "2026-11-14 09:30 Saturday W46");
        assert_eq!(run("{{ date|format(\"%d %B %Y\") }}", &lib), "14 November 2026");
        assert_eq!(run("{{ date(\"tomorrow\") }} {{ date(\"+2 weeks\") }} {{ date(\"next monday\") }} {{ date(\"last friday\") }}", &lib), "2026-11-15 2026-11-28 2026-11-16 2026-11-13");
        assert_eq!(run("{{ ask \"Topic\" }} {{ ask \"Who\" \"me\" }} {{ title|upper }} {{ selection|default(\"-\") }}", &lib), "Budget me NOTES -");
        let mut f = fields();
        f.german = true;
        f.date_format = "%d.%m.%Y".into();
        assert_eq!(render("{{ weekday }}, {{ date|format(\"%A, %d. %B\") }} {{ datetime }}", &lib, &f).unwrap().text, "Samstag, Samstag, 14. November 14.11.2026 09:30");
        let out = render("a{{ cursor }}b", &lib, &fields()).unwrap();
        assert_eq!((out.text.as_str(), out.cursor), ("ab", Some(1)));
        assert!(render("{{ nope }}", &lib, &fields()).is_err());
    }

    #[test]
    fn includes_extends_and_blocks() {
        let lib = Lib(vec![
            ("Signature", "-- \n{{ user }}"),
            ("Base", "# {{ ask \"Topic\" }}\n{% block body %}default body\n{% endblock %}\n{% include \"Signature\" %}\n"),
            ("Meeting", "{% extends \"Base\" %}\n{% block body %}\n## Attendees\n- {{ cursor }}\n{{ parent() }}{% endblock %}\n"),
        ]);
        assert_eq!(run("{% include \"Signature\" %}", &lib), "-- \nKim");
        let out = render(&lib.template("Meeting").unwrap(), &lib, &fields()).unwrap();
        assert_eq!(out.text, "# Budget\n## Attendees\n- \ndefault body\n-- \nKim");
        assert_eq!(out.cursor, Some("# Budget\n## Attendees\n- ".len()));
        let q = questions(&lib.template("Meeting").unwrap(), &lib, &fields()).unwrap();
        assert_eq!(q, [Question { label: "Topic".into(), default: String::new() }]);
        assert!(render("{% include \"Missing\" %}", &lib, &fields()).unwrap_err().contains("Missing"));
        let looped = Lib(vec![("A", "{% include \"A\" %}")]);
        assert!(render("{% include \"A\" %}", &looped, &fields()).is_err());
    }

    #[test]
    fn merges_template_changes() {
        let base = "# Title\nIntro\n## Notes\n\nFooter\n";
        // Template changes the headline, the user wrote notes.
        let theirs = "# New title\nIntro\n## Notes\n\nFooter\n";
        let ours = "# Title\nIntro\n## Notes\nmy text\nmore\n\nFooter\n";
        assert_eq!(merge(base, theirs, ours).unwrap(), "# New title\nIntro\n## Notes\nmy text\nmore\n\nFooter\n");
        // The user changed the headline: their version stays.
        let ours = "# My title\nIntro\n## Notes\n\nFooter\n";
        assert_eq!(merge(base, theirs, ours).unwrap(), ours);
        // Additions and removals in the template.
        let theirs = "# Title\nIntro\nNew line\n## Notes\n\n";
        let ours = "# Title\nIntro\n## Notes\nx\n\nFooter\n";
        assert_eq!(merge(base, theirs, ours).unwrap(), "# Title\nIntro\nNew line\n## Notes\nx\n\n");
        // Nothing changed in the template.
        assert_eq!(merge(base, base, "anything\n").unwrap(), "anything\n");
        // A change right next to the user's text still comes through.
        let base = "## Attendees\n- \n\nNext\n";
        let theirs = "## Attendees of the meeting\n- \n\nNext\n";
        let ours = "## Attendees\n- Kim, Sam\n\nNext\n";
        assert_eq!(merge(base, theirs, ours).unwrap(), "## Attendees of the meeting\n- Kim, Sam\n\nNext\n");
        // Both insert at the same place: the note wins.
        assert_eq!(merge("a\n", "a\nt\n", "a\no\n").unwrap(), "a\no\n");
        // A template line inserted between two lines the user changed.
        assert_eq!(merge("a\nb\n", "a\nx\nb\n", "A\nB\n").unwrap(), "A\nB\n");
        // Unchanged note: it becomes the new template text.
        assert_eq!(merge(base, "a\nb\n", base).unwrap(), "a\nb\n");
    }

    #[test]
    fn links_keep_their_fields() {
        let mut f = fields();
        f.answers.insert("Who".into(), "Kim".into());
        let link = Link::new("id1", "Meeting", &f, "text");
        let json = serde_json::to_string(&link).unwrap();
        let back: Link = serde_json::from_str(&json).unwrap();
        let lib = Lib(vec![]);
        assert_eq!(render("{{ date }} {{ ask \"Who\" }}", &lib, &back.fields()).unwrap().text, "2026-11-14 Kim");
    }

    #[test]
    fn conditions_comments_and_raw() {
        let lib = Lib(vec![]);
        assert_eq!(run("{% if selection %}sel{% else %}none{% endif %}", &lib), "none");
        assert_eq!(run("a\n{# note #}\nb {# x #}c", &lib), "a\nb c");
        assert_eq!(run("{% raw %}{{ date }}{% endraw %}", &lib), "{{ date }}");
        assert_eq!(run("x\n  {% if title %}\n  yes\n  {% endif %}\ny", &lib), "x\n  yes\ny");
        assert_eq!(run("{ not a tag }", &lib), "{ not a tag }");
        assert!(render("{% if title %}x", &lib, &fields()).is_err());
    }
}
