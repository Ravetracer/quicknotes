//! YAML checking: syntax errors and duplicate keys (via `yaml-rust2`), a
//! [`Node`] tree for schema validation, yamllint-like style rules and the
//! completion context at a cursor position.

use crate::datadoc::{Diagnostic, Member, Node, Place, Seg, Spot, Value};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::ops::Range;
use yaml_rust2::parser::{Event, MarkedEventReceiver, Parser};
use yaml_rust2::scanner::{Marker, TScalarStyle};

/// Style rules that can be switched off in the preferences.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(default)]
pub struct Rules {
    pub trailing_spaces: bool,
    pub indentation: bool,
    pub truthy: bool,
    pub comments: bool,
    pub empty_lines: bool,
    pub line_length: bool,
}

impl Default for Rules {
    fn default() -> Self {
        Self { trailing_spaces: true, indentation: true, truthy: true, comments: true, empty_lines: true, line_length: false }
    }
}

/// The rules with their preference labels.
pub const RULES: &[(&str, &str, &str)] = &[
    ("trailing_spaces", "Trailing Spaces", "Spaces or tabs at the end of a line"),
    ("indentation", "Indentation", "Nested blocks indented by a different number of spaces than elsewhere in the file"),
    ("truthy", "Truthy Values", "yes, no, on, off and similar instead of true or false"),
    ("comments", "Comment Spacing", "A space after #"),
    ("empty_lines", "Blank Lines", "More than two blank lines in a row"),
    ("line_length", "Line Length", "Lines longer than 120 characters"),
];

impl Rules {
    pub fn get(&self, key: &str) -> bool {
        match key {
            "trailing_spaces" => self.trailing_spaces,
            "indentation" => self.indentation,
            "truthy" => self.truthy,
            "comments" => self.comments,
            "empty_lines" => self.empty_lines,
            "line_length" => self.line_length,
            _ => false,
        }
    }

    pub fn set(&mut self, key: &str, on: bool) {
        match key {
            "trailing_spaces" => self.trailing_spaces = on,
            "indentation" => self.indentation = on,
            "truthy" => self.truthy = on,
            "comments" => self.comments = on,
            "empty_lines" => self.empty_lines = on,
            "line_length" => self.line_length = on,
            _ => {}
        }
    }
}

const MAX_LINE: usize = 120;

pub struct Parsed {
    /// The documents of the stream; empty after a syntax error.
    pub documents: Vec<Node>,
    pub diagnostics: Vec<Diagnostic>,
}

pub fn parse(text: &str, rules: &Rules) -> Parsed {
    let chars: Vec<usize> = text.char_indices().map(|(i, _)| i).chain(std::iter::once(text.len())).collect();
    let mut b = Builder { text, chars: &chars, stack: Vec::new(), anchors: HashMap::new(), documents: Vec::new(), diags: Vec::new(), plains: Vec::new(), steps: Vec::new() };
    let result = Parser::new_from_str(text).load(&mut b, true);
    let mut diags = std::mem::take(&mut b.diags);
    let mut documents = std::mem::take(&mut b.documents);
    if let Err(e) = result {
        let at = b.byte(e.marker());
        let range = crate::datadoc::visible(text, at..at);
        diags.push(Diagnostic::error(range, scan_message(e.info())));
        documents.clear();
    } else {
        if rules.truthy {
            for (range, s, key) in &b.plains {
                if !key && is_truthy(s) {
                    diags.push(Diagnostic::warning(range.clone(), t!("Truthy value \"{}\": use true or false, or quote it", s)));
                }
            }
        }
        if rules.indentation {
            indentation(&b.steps, &mut diags);
        }
    }
    line_rules(text, rules, &mut diags);
    diags.sort_by_key(|d| d.range.start);
    Parsed { documents, diagnostics: diags }
}

/// Messages of the YAML scanner and their friendlier versions.
pub const SCAN_MESSAGES: &[(&str, &str)] = &[
    ("mapping values are not allowed in this context", "Mapping values are not allowed here (missing quotes or wrong indentation?)"),
    ("did not find expected key", "Expected a key here (wrong indentation?)"),
    ("did not find expected '-' indicator", "Expected a list item ('-') here (wrong indentation?)"),
    ("tabs disallowed within this context (block indentation)", "Tabs are not allowed for indentation"),
    ("illegal placement of ':' indicator", "Unexpected ':' (missing closing bracket?)"),
    ("did not find expected ',' or ']'", "Expected ',' or ']'"),
    ("did not find expected ',' or '}'", "Expected ',' or '}'"),
    ("found unexpected end of stream", "Unexpected end of the document"),
    ("found undefined alias", "Unknown alias"),
    ("while scanning a quoted scalar, found unexpected end of stream", "Unterminated string"),
    ("while parsing a block mapping, did not find expected key", "Expected a key here (wrong indentation?)"),
    ("while parsing a block collection, did not find expected '-' indicator", "Expected a list item ('-') here (wrong indentation?)"),
];

/// The scanner's messages, made a little friendlier (and translated).
fn scan_message(info: &str) -> String {
    for (en, msg) in SCAN_MESSAGES {
        if info.contains(en) {
            return crate::i18n::tr(msg).to_owned();
        }
    }
    t!("Syntax error: {}", info)
}

fn is_truthy(s: &str) -> bool {
    matches!(
        s,
        "yes" | "Yes" | "YES" | "no" | "No" | "NO" | "on" | "On" | "ON" | "off" | "Off" | "OFF" | "y" | "Y" | "n" | "N" | "True"
            | "TRUE" | "False" | "FALSE"
    )
}

enum Open {
    Seq { start: usize, flow: bool, items: Vec<Node>, anchor: usize },
    Map { start: usize, flow: bool, members: Vec<Member>, key: Option<(String, Range<usize>)>, anchor: usize, parent_key: Option<(usize, usize)>, first: bool, merged: Vec<String> },
}

/// Indentation of a nested block relative to its parent key (columns),
/// with the position of its first line.
struct Step {
    seq: bool,
    delta: usize,
    range: Range<usize>,
}

struct Builder<'a> {
    text: &'a str,
    chars: &'a [usize],
    stack: Vec<Open>,
    anchors: HashMap<usize, Node>,
    documents: Vec<Node>,
    diags: Vec<Diagnostic>,
    /// Plain scalars: range, text and whether it is a key.
    plains: Vec<(Range<usize>, String, bool)>,
    steps: Vec<Step>,
}

impl Builder<'_> {
    fn byte(&self, m: &Marker) -> usize {
        self.chars.get(m.index()).copied().unwrap_or(self.text.len())
    }

    fn in_flow(&self) -> bool {
        self.stack.iter().any(|o| matches!(o, Open::Seq { flow: true, .. } | Open::Map { flow: true, .. }))
    }

    /// Line start and column (bytes) of a byte offset.
    fn col(&self, at: usize) -> (usize, usize) {
        let line_start = self.text[..at].rfind('\n').map_or(0, |i| i + 1);
        (line_start, at - line_start)
    }

    fn line_of(&self, at: usize) -> usize {
        self.text[..at].matches('\n').count()
    }

    /// End of a scalar written at `start`.
    fn scalar_end(&self, start: usize, style: TScalarStyle, key: bool) -> usize {
        let rest = &self.text[start..];
        let line_end = rest.find('\n').map_or(self.text.len(), |i| start + i);
        match style {
            TScalarStyle::DoubleQuoted => {
                let mut esc = false;
                for (i, c) in rest.char_indices().skip(1) {
                    match c {
                        '\\' if !esc => esc = true,
                        '"' if !esc => return start + i + 1,
                        _ => esc = false,
                    }
                }
                self.text.len()
            }
            TScalarStyle::SingleQuoted => {
                let mut it = rest.char_indices().skip(1).peekable();
                while let Some((i, c)) = it.next() {
                    if c == '\'' {
                        if it.peek().is_some_and(|(_, n)| *n == '\'') {
                            it.next();
                        } else {
                            return start + i + 1;
                        }
                    }
                }
                self.text.len()
            }
            TScalarStyle::Literal | TScalarStyle::Folded => line_end,
            TScalarStyle::Plain => {
                let flow = self.in_flow();
                let line = &self.text[start..line_end];
                let mut end = line.len();
                let b = line.as_bytes();
                for (i, &c) in b.iter().enumerate() {
                    let next_space = b.get(i + 1).is_none_or(|n| *n == b' ' || *n == b'\t');
                    let stop = (c == b'#' && i > 0 && matches!(b[i - 1], b' ' | b'\t'))
                        || (c == b':' && (next_space || (flow && key)))
                        || (flow && matches!(c, b',' | b']' | b'}'));
                    if stop {
                        end = i;
                        break;
                    }
                }
                start + line[..end].trim_end().len()
            }
        }
    }

    fn add(&mut self, node: Node, anchor: usize) {
        if anchor > 0 {
            self.anchors.insert(anchor, node.clone());
        }
        match self.stack.last_mut() {
            None => self.documents.push(node),
            Some(Open::Seq { items, .. }) => items.push(node),
            Some(Open::Map { members, key, merged, .. }) => match key.take() {
                None => {
                    let k = match &node.value {
                        Value::String(s) => s.clone(),
                        Value::Number(n) => crate::datadoc::number_json(*n).to_string(),
                        Value::Bool(b) => b.to_string(),
                        Value::Null => "null".into(),
                        _ => self.text[node.range.clone()].to_owned(),
                    };
                    *key = Some((k, node.range));
                }
                Some((k, key_range)) => {
                    if k == "<<" {
                        // Merge keys: the members of the referenced mapping(s).
                        let sources: Vec<&Node> = match &node.value {
                            Value::Array(items) => items.iter().collect(),
                            _ => vec![&node],
                        };
                        for src in sources {
                            if let Value::Object(more) = &src.value {
                                for m in more {
                                    if !members.iter().any(|x| x.key == m.key) {
                                        merged.push(m.key.clone());
                                        members.push(m.clone());
                                    }
                                }
                            }
                        }
                        return;
                    }
                    if let Some(i) = members.iter().position(|m| m.key == k) {
                        // Merged members may be overridden.
                        if !merged.contains(&k) {
                            self.diags.push(Diagnostic::error(key_range.clone(), t!("Duplicate key \"{}\"", k)));
                        }
                        members.remove(i);
                    }
                    members.push(Member { key: k, key_range, value: node });
                }
            },
        }
    }

    /// Records the indentation of a block collection that is the value of a key.
    fn parent_key(&self) -> Option<(usize, usize)> {
        match self.stack.last() {
            Some(Open::Map { key: Some((_, r)), flow: false, .. }) => Some((r.start, self.line_of(r.start))),
            _ => None,
        }
    }
}

impl MarkedEventReceiver for Builder<'_> {
    fn on_event(&mut self, ev: Event, mark: Marker) {
        let at = self.byte(&mark);
        match ev {
            Event::Scalar(value, style, anchor, tag) => {
                let is_key = matches!(self.stack.last(), Some(Open::Map { key: None, .. }));
                // Empty values come as "~" without text.
                let empty = style == TScalarStyle::Plain && value == "~" && !self.text[at..].starts_with('~');
                let range = if empty { at..at } else { at..self.scalar_end(at, style, is_key) };
                if style == TScalarStyle::Plain && !empty {
                    self.plains.push((range.clone(), value.clone(), is_key));
                }
                // The first key of a block mapping gives its indentation.
                if let Some(Open::Map { first: true, flow: false, parent_key, .. }) = self.stack.last_mut() {
                    let first = std::mem::take(parent_key);
                    if let Some(Open::Map { first, .. }) = self.stack.last_mut() {
                        *first = false;
                    }
                    if let Some((key_at, key_line)) = first {
                        if self.line_of(at) > key_line {
                            let (_, kc) = self.col(key_at);
                            let (_, c) = self.col(at);
                            self.steps.push(Step { seq: false, delta: c.saturating_sub(kc), range: range.clone() });
                        }
                    }
                }
                let plain_typed = tag.as_ref().is_none_or(|t| !t.suffix.ends_with("str"));
                let v = if empty {
                    Value::Null
                } else if style == TScalarStyle::Plain && plain_typed {
                    resolve(&value)
                } else {
                    Value::String(value)
                };
                self.add(Node::new(v, range), anchor);
            }
            Event::Alias(id) => {
                let end = self.text[at..].find(|c: char| c.is_whitespace() || matches!(c, ',' | ']' | '}')).map_or(self.text.len(), |i| at + i);
                let node = match self.anchors.get(&id) {
                    Some(n) => Node::new(n.value.clone(), at..end),
                    None => Node::new(Value::Null, at..end),
                };
                if let Some(Open::Map { first: true, .. }) = self.stack.last() {
                    if let Some(Open::Map { first, parent_key, .. }) = self.stack.last_mut() {
                        *first = false;
                        *parent_key = None;
                    }
                }
                self.add(node, 0);
            }
            Event::SequenceStart(anchor, _) => {
                let flow = self.text[at..].starts_with('[');
                // Lists directly under a key report their first item, not the dash.
                let at = if flow || self.text[at..].starts_with('-') {
                    at
                } else {
                    let (line_start, _) = self.col(at);
                    self.text[line_start..at].rfind('-').map_or(at, |i| line_start + i)
                };
                if !flow {
                    if let Some((key_at, key_line)) = self.parent_key() {
                        if self.line_of(at) > key_line {
                            let (_, kc) = self.col(key_at);
                            let (_, c) = self.col(at);
                            self.steps.push(Step { seq: true, delta: c.saturating_sub(kc), range: at..at + 1 });
                        }
                    }
                }
                self.stack.push(Open::Seq { start: at, flow, items: Vec::new(), anchor });
            }
            Event::MappingStart(anchor, _) => {
                let flow = self.text[at..].starts_with('{');
                let parent_key = if flow { None } else { self.parent_key() };
                // Block mappings report a position inside their first key.
                let start = if flow { at } else { at.saturating_sub(1) };
                self.stack.push(Open::Map { start, flow, members: Vec::new(), key: None, anchor, parent_key, first: true, merged: Vec::new() });
            }
            Event::SequenceEnd => {
                if let Some(Open::Seq { start, flow, items, anchor }) = self.stack.pop() {
                    let end = if flow { at + 1 } else { items.last().map_or(at, |n| n.range.end) };
                    self.add(Node::new(Value::Array(items), start..end.max(start)), anchor);
                }
            }
            Event::MappingEnd => {
                if let Some(Open::Map { start, flow, members, anchor, .. }) = self.stack.pop() {
                    let start = members.first().map_or(start, |m| if flow { start } else { m.key_range.start });
                    let end = if flow { at + 1 } else { members.last().map_or(at, |m| m.value.range.end.max(m.key_range.end)) };
                    self.add(Node::new(Value::Object(members), start..end.max(start)), anchor);
                }
            }
            _ => {}
        }
    }
}

/// Plain scalars per the YAML 1.2 core schema.
fn resolve(s: &str) -> Value {
    match s {
        "" | "~" | "null" | "Null" | "NULL" => return Value::Null,
        "true" | "True" | "TRUE" => return Value::Bool(true),
        "false" | "False" | "FALSE" => return Value::Bool(false),
        ".inf" | ".Inf" | ".INF" | "+.inf" | "+.Inf" | "+.INF" => return Value::Number(f64::INFINITY),
        "-.inf" | "-.Inf" | "-.INF" => return Value::Number(f64::NEG_INFINITY),
        ".nan" | ".NaN" | ".NAN" => return Value::Number(f64::NAN),
        _ => {}
    }
    if let Some(hex) = s.strip_prefix("0x") {
        if let Ok(v) = i64::from_str_radix(hex, 16) {
            return Value::Number(v as f64);
        }
    }
    if let Some(oct) = s.strip_prefix("0o") {
        if let Ok(v) = i64::from_str_radix(oct, 8) {
            return Value::Number(v as f64);
        }
    }
    let digits = s.strip_prefix(['-', '+']).unwrap_or(s);
    let numeric = !digits.is_empty()
        && digits.chars().next().is_some_and(|c| c.is_ascii_digit() || c == '.')
        && digits.chars().all(|c| c.is_ascii_digit() || matches!(c, '.' | 'e' | 'E' | '-' | '+'))
        && digits.chars().any(|c| c.is_ascii_digit());
    if numeric {
        if let Ok(v) = s.parse::<f64>() {
            return Value::Number(v);
        }
    }
    Value::String(s.to_owned())
}

/// Nested blocks should be indented by the same number of spaces throughout;
/// lists under a key either all indented or all not.
fn indentation(steps: &[Step], diags: &mut Vec<Diagnostic>) {
    let spaces = steps
        .iter()
        .find(|s| !s.seq && s.delta > 0)
        .or_else(|| steps.iter().find(|s| s.delta > 0))
        .map_or(2, |s| s.delta);
    let mut seq_style: Option<usize> = None;
    for s in steps {
        if s.seq {
            match seq_style {
                None if s.delta == 0 || s.delta == spaces => seq_style = Some(s.delta),
                Some(d) if d == s.delta => {}
                _ => {
                    let expected = seq_style.unwrap_or(spaces);
                    diags.push(Diagnostic::warning(s.range.clone(), t!("Wrong indentation: expected {} spaces but found {}", expected, s.delta)));
                }
            }
        } else if s.delta != spaces {
            diags.push(Diagnostic::warning(s.range.clone(), t!("Wrong indentation: expected {} spaces but found {}", spaces, s.delta)));
        }
    }
}

/// Lines inside block scalars (`|`, `>`), which hold text rather than YAML.
fn block_scalar_lines(lines: &[&str]) -> Vec<bool> {
    let mut inside = vec![false; lines.len()];
    let mut block: Option<usize> = None;
    for (i, line) in lines.iter().enumerate() {
        let indent = line.len() - line.trim_start_matches(' ').len();
        if let Some(parent) = block {
            if line.trim().is_empty() || indent > parent {
                inside[i] = true;
                continue;
            }
            block = None;
        }
        let code = strip_comment(line).trim_end();
        let starts_block = code
            .rsplit([' ', ':', '-'])
            .next()
            .is_some_and(|last| (last.starts_with('|') || last.starts_with('>')) && last[1..].chars().all(|c| matches!(c, '+' | '-' | '0'..='9')))
            && (code.contains(": ") || code.trim_start().starts_with("- ") || code.trim() == "|" || code.trim() == ">");
        if starts_block {
            block = Some(indent);
        }
    }
    inside
}

/// The line without its comment (a `#` after whitespace, outside quotes).
fn strip_comment(line: &str) -> &str {
    match comment_start(line) {
        Some(i) => &line[..i],
        None => line,
    }
}

fn comment_start(line: &str) -> Option<usize> {
    let mut quote: Option<char> = None;
    let mut prev = ' ';
    for (i, c) in line.char_indices() {
        match quote {
            Some(q) if c == q => quote = None,
            Some(_) => {}
            None if (c == '"' || c == '\'') && (prev == ' ' || prev == ':' || prev == '[' || prev == '{' || prev == ',' || i == 0 || prev == '-') => quote = Some(c),
            None if c == '#' && (prev == ' ' || prev == '\t' || i == 0) => return Some(i),
            None => {}
        }
        prev = c;
    }
    None
}

fn line_rules(text: &str, rules: &Rules, diags: &mut Vec<Diagnostic>) {
    let lines: Vec<&str> = text.split('\n').collect();
    let blocks = block_scalar_lines(&lines);
    let mut offset = 0;
    let mut blank_run = 0;
    let mut blank_start = 0;
    for (i, raw) in lines.iter().enumerate() {
        let line = raw.strip_suffix('\r').unwrap_or(raw);
        if rules.trailing_spaces {
            let trimmed = line.trim_end_matches([' ', '\t']).len();
            if trimmed < line.len() {
                diags.push(Diagnostic::warning(offset + trimmed..offset + line.len(), t!("Trailing spaces")));
            }
        }
        if rules.line_length {
            let n = line.chars().count();
            if n > MAX_LINE {
                let from = line.char_indices().nth(MAX_LINE).map_or(0, |(b, _)| b);
                diags.push(Diagnostic::warning(offset + from..offset + line.len(), t!("Line too long ({} > {} characters)", n, MAX_LINE)));
            }
        }
        if rules.comments && !blocks[i] {
            if let Some(c) = comment_start(line) {
                let after = line[c + 1..].chars().next();
                let shebang = i == 0 && c == 0 && after == Some('!');
                if after.is_some_and(|ch| ch != ' ' && ch != '#' && ch != '\t') && !shebang {
                    diags.push(Diagnostic::warning(offset + c..offset + c + 1, t!("Missing space after '#'")));
                }
            }
        }
        if rules.empty_lines {
            if line.trim().is_empty() && !blocks[i] && i + 1 < lines.len() {
                if blank_run == 0 {
                    blank_start = offset;
                }
                blank_run += 1;
            } else {
                if blank_run > 2 && i + 1 < lines.len() + 1 {
                    diags.push(Diagnostic::warning(blank_start..offset, t!("Too many blank lines ({} > 2)", blank_run)));
                }
                blank_run = 0;
            }
        }
        offset += raw.len() + 1;
    }
}

// ----- completion context -------------------------------------------------

/// A line split into its list dashes, the start of its content and the key
/// (if it is `key:` or `key: value`), columns in bytes.
struct Line<'a> {
    dashes: Vec<usize>,
    rest: usize,
    key: Option<&'a str>,
}

fn split_line(line: &str) -> Line<'_> {
    let mut i = line.len() - line.trim_start_matches(' ').len();
    let mut dashes = Vec::new();
    let b = line.as_bytes();
    while b.get(i) == Some(&b'-') && matches!(b.get(i + 1), Some(b' ') | None) {
        dashes.push(i);
        i += 1;
        while b.get(i) == Some(&b' ') {
            i += 1;
        }
    }
    let content = strip_comment(&line[i..]);
    let key = key_of(content);
    Line { dashes, rest: i, key }
}

/// The key of `key:` / `key: value` / `"key": value`.
fn key_of(content: &str) -> Option<&str> {
    let content = content.trim_end();
    let colon = if content.starts_with(['"', '\'']) {
        let q = content.as_bytes()[0] as char;
        let close = content[1..].find(q)? + 1;
        let after = &content[close + 1..];
        if !after.starts_with(':') {
            return None;
        }
        return Some(&content[1..close]);
    } else {
        content.find(": ").or_else(|| content.ends_with(':').then(|| content.len() - 1))?
    };
    let key = content[..colon].trim_end();
    (!key.is_empty() && !key.starts_with(['{', '[', '&', '*', '!', '|', '>', '#'])).then_some(key)
}

fn is_blank_or_comment(line: &str) -> bool {
    let t = line.trim_start();
    t.is_empty() || t.starts_with('#') || t == "---" || t == "..."
}

/// The path to the block that a line at `line_no` with content at column
/// `col` belongs to (the dashes on that line are not included).
fn parent_path(lines: &[&str], line_no: usize, col: usize) -> Vec<Seg> {
    let mut limit = col;
    let mut path = Vec::new();
    for j in (0..line_no).rev() {
        if limit == 0 {
            break;
        }
        let line = lines[j];
        if is_blank_or_comment(line) {
            continue;
        }
        let indent = line.len() - line.trim_start_matches(' ').len();
        if indent >= limit {
            continue;
        }
        let l = split_line(line);
        if l.rest < limit {
            if let Some(key) = l.key {
                path.push(Seg::Key(key.to_owned()));
                limit = l.rest;
            }
        }
        for &d in l.dashes.iter().rev() {
            if d < limit {
                path.push(Seg::Index(item_index(lines, j, d)));
                limit = d;
            }
        }
    }
    path.reverse();
    path
}

/// Position of the list item with its dash at column `col` on line `line_no`.
fn item_index(lines: &[&str], line_no: usize, col: usize) -> usize {
    let mut n = 0;
    for j in (0..line_no).rev() {
        let line = lines[j];
        if is_blank_or_comment(line) {
            continue;
        }
        let indent = line.len() - line.trim_start_matches(' ').len();
        if indent < col {
            break;
        }
        if indent == col && split_line(line).dashes.first() == Some(&col) {
            n += 1;
        }
    }
    n
}

/// Keys of the mapping whose keys start at column `col` around line `line_no`.
fn sibling_keys(lines: &[&str], line_no: usize, col: usize) -> Vec<String> {
    let mut keys = Vec::new();
    let take = |line: &str, keys: &mut Vec<String>| -> bool {
        if is_blank_or_comment(line) {
            return true;
        }
        let l = split_line(line);
        let indent = line.len() - line.trim_start_matches(' ').len();
        if l.rest == col {
            if let Some(k) = l.key {
                keys.push(k.to_owned());
            }
            // A list item's first line ends the mapping upwards.
            return l.dashes.is_empty();
        }
        indent > col
    };
    for j in (0..line_no).rev() {
        if !take(lines[j], &mut keys) {
            break;
        }
    }
    for line in lines.iter().skip(line_no + 1) {
        let l = split_line(line);
        if !is_blank_or_comment(line) && !l.dashes.is_empty() && l.dashes[0] < col {
            break;
        }
        if !take(line, &mut keys) {
            break;
        }
    }
    keys
}

/// What can be completed at `cursor` (a byte offset): property names and/or
/// values with their paths. A list item could be either, so both are given.
pub fn places(text: &str, cursor: usize) -> Vec<Place> {
    let line_start = text[..cursor].rfind('\n').map_or(0, |i| i + 1);
    let line_end = text[cursor..].find('\n').map_or(text.len(), |i| cursor + i);
    let lines: Vec<&str> = text.split('\n').collect();
    let line_no = text[..line_start].matches('\n').count();
    if block_scalar_lines(&lines)[line_no] {
        return Vec::new();
    }
    let before = &text[line_start..cursor];
    let whole = &text[line_start..line_end];
    if comment_start(before).is_some() || before.trim_start().starts_with("---") {
        return Vec::new();
    }
    let l = split_line(before);
    let content = &before[l.rest.min(before.len())..];
    let mut path = parent_path(&lines, line_no, l.dashes.first().copied().unwrap_or(l.rest));
    for &d in &l.dashes {
        let index = if Some(&d) == l.dashes.first() { item_index(&lines, line_no, d) } else { 0 };
        path.push(Seg::Index(index));
    }
    let word_end = |from: usize| -> usize {
        whole[from..].find([':', ' ', '#']).map_or(whole.len(), |i| from + i)
    };
    if let Some(colon) = content.find(": ").or_else(|| content.strip_suffix(':').map(|c| c.len())) {
        // `key: val|`
        let key = content[..colon].trim().trim_matches(['"', '\'']);
        if key.is_empty() || content[colon + 1..].contains(": ") {
            return Vec::new();
        }
        let value_start = colon + 1 + (content[colon + 1..].len() - content[colon + 1..].trim_start().len());
        let typed = &content[value_start..];
        if typed.starts_with(['[', '{', '|', '>', '&', '*']) {
            return Vec::new();
        }
        let start = line_start + l.rest + value_start;
        let end = line_start + strip_comment(whole).trim_end().len().max(cursor - line_start);
        let mut value_path = path;
        value_path.push(Seg::Key(key.to_owned()));
        return vec![Place { path: value_path, spot: Spot::Value, range: start..end, prefix: typed.trim_matches(['"', '\'']).to_owned() }];
    }
    if content.contains(['[', '{', '#']) {
        return Vec::new();
    }
    let start = line_start + l.rest;
    let end = line_start + word_end(l.rest).max(cursor - line_start);
    let prefix = content.trim_matches(['"', '\'']).to_owned();
    let existing = sibling_keys(&lines, line_no, l.rest);
    let mut out = vec![Place { path: path.clone(), spot: Spot::Key { existing: existing.into_iter().filter(|k| *k != content).collect() }, range: start..end, prefix: prefix.clone() }];
    if !l.dashes.is_empty() {
        out.push(Place { path, spot: Spot::Value, range: start..end, prefix });
    }
    out
}

/// The text right after a key being completed already has its ':'.
pub fn has_colon(text: &str, end: usize) -> bool {
    text.get(end..).is_some_and(|rest| rest.starts_with(':'))
}

/// Indentation (in spaces) of the line holding byte offset `at`.
pub fn indent_at(text: &str, at: usize) -> usize {
    let line_start = text[..at].rfind('\n').map_or(0, |i| i + 1);
    let l = split_line(&text[line_start..]);
    l.rest
}

/// `s` as a YAML scalar (quoted when it would otherwise be read differently).
pub fn scalar(s: &str) -> String {
    let plain = !s.is_empty()
        && matches!(resolve(s), Value::String(_))
        && !is_truthy(s)
        && !s.starts_with([' ', '-', '?', ':', ',', '[', ']', '{', '}', '#', '&', '*', '!', '|', '>', '\'', '"', '%', '@', '`'])
        && !s.ends_with(' ')
        && !s.contains(": ")
        && !s.contains(" #")
        && !s.contains('\n');
    if plain {
        s.to_owned()
    } else {
        serde_json::to_string(s).unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn messages(text: &str) -> Vec<String> {
        parse(text, &Rules::default()).diagnostics.into_iter().map(|d| d.message).collect()
    }

    #[test]
    fn builds_tree() {
        let text = "ä: 1\nkey:\n  - a\n  - b: \"x\"\n    c: [1, 2.5, {d: null}]\nz: |\n  lit\n  more\nw: plain text  # c\nn:\nt: true\n";
        let p = parse(text, &Rules::default());
        assert!(p.diagnostics.is_empty(), "{:?}", p.diagnostics);
        let root = &p.documents[0];
        assert_eq!(
            root.to_json(),
            json!({"ä": 1, "key": ["a", {"b": "x", "c": [1, 2.5, {"d": null}]}], "z": "lit\nmore\n", "w": "plain text", "n": null, "t": true})
        );
        let w = root.member("w").unwrap();
        assert_eq!(&text[w.value.range.clone()], "plain text");
        assert_eq!(&text[w.key_range.clone()], "w");
        let b = root.at(&[Seg::Key("key".into()), Seg::Index(1)]).unwrap();
        assert_eq!(&text[b.member("b").unwrap().value.range.clone()], "\"x\"");
    }

    #[test]
    fn anchors_and_merges() {
        let p = parse("base: &b\n  x: 1\n  y: 2\nother:\n  <<: *b\n  y: 3\n", &Rules::default());
        assert!(p.diagnostics.is_empty(), "{:?}", p.diagnostics);
        assert_eq!(p.documents[0].to_json(), json!({"base": {"x": 1, "y": 2}, "other": {"x": 1, "y": 3}}));
    }

    #[test]
    fn problems() {
        assert_eq!(messages("a: 1\na: 2\n"), ["Duplicate key \"a\""]);
        assert_eq!(messages("a:\n\tb: 1\n"), ["Tabs are not allowed for indentation"]);
        assert_eq!(messages("a: 1 \n"), ["Trailing spaces"]);
        assert_eq!(messages("a: yes\nb: \"no\"\n"), ["Truthy value \"yes\": use true or false, or quote it"]);
        assert_eq!(messages("#bad\n# good\na: 1 #x\n"), ["Missing space after '#'", "Missing space after '#'"]);
        assert_eq!(messages("a: 1\n\n\n\nb: 2\n"), ["Too many blank lines (3 > 2)"]);
        assert_eq!(messages("a:\n  b:\n     c: 1\n"), ["Wrong indentation: expected 2 spaces but found 3"]);
        assert_eq!(messages("a:\n  - 1\nb:\n- 2\n"), ["Wrong indentation: expected 2 spaces but found 0"]);
        assert!(messages("a:\n- 1\nb:\n  c: |\n    #x  \n").contains(&"Trailing spaces".to_string()));
        assert!(!messages("a: |\n  #x\n").iter().any(|m| m.contains('#')));
        let p = parse("a: [1, 2\nb: 3\n", &Rules::default());
        assert!(p.documents.is_empty());
        assert_eq!(p.diagnostics.len(), 1);
    }

    #[test]
    fn completion_places() {
        let at = |s: &str| {
            let cursor = s.find('|').unwrap();
            places(&s.replace('|', ""), cursor)
        };
        let p = at("a: 1\nb:\n  c: 2\n  d|\n  e: 3\n");
        assert_eq!(p.len(), 1);
        assert_eq!(p[0].path, vec![Seg::Key("b".into())]);
        assert_eq!(p[0].spot, Spot::Key { existing: vec!["c".into(), "e".into()] });
        assert_eq!(p[0].prefix, "d");
        let p = at("items:\n  - name: a\n    va|\n");
        assert_eq!(p[0].path, vec![Seg::Key("items".into()), Seg::Index(0)]);
        assert_eq!(p[0].spot, Spot::Key { existing: vec!["name".into()] });
        let p = at("items:\n  - name: a\n  - name: b\n    kind: x|\n");
        assert_eq!(p[0].path, vec![Seg::Key("items".into()), Seg::Index(1), Seg::Key("kind".into())]);
        assert_eq!((p[0].spot.clone(), p[0].prefix.as_str()), (Spot::Value, "x"));
        let p = at("list:\n  - |");
        assert_eq!(p.len(), 2);
        assert_eq!(p[1].path, vec![Seg::Key("list".into()), Seg::Index(0)]);
        let p = at("a:\n  b: \"q|\"\n");
        assert_eq!((p[0].prefix.as_str(), p[0].range.clone()), ("q", 8..11));
        assert!(at("a: 1 # x|\n").is_empty());
        assert!(places("a: |\n  te\n", 9).is_empty());
        assert_eq!(scalar("hello world"), "hello world");
        assert_eq!(scalar("yes"), "\"yes\"");
        assert_eq!(scalar("1.5"), "\"1.5\"");
        assert_eq!(scalar("a: b"), "\"a: b\"");
    }
}
