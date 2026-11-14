//! JSON checking: a strict parser that reports syntax problems with their
//! positions (trailing commas, comments, single quotes, duplicate keys, …)
//! and builds a [`Node`] tree for schema validation, plus the completion
//! context at a cursor position.

use crate::datadoc::{Diagnostic, Member, Node, Place, Seg, Spot, Value};
use std::ops::Range;

pub struct Parsed {
    /// The document; `None` when it is empty or could not be read to the end.
    pub root: Option<Node>,
    pub diagnostics: Vec<Diagnostic>,
}

const MAX_DEPTH: usize = 512;

pub fn parse(text: &str) -> Parsed {
    let mut p = Parser { s: text, b: text.as_bytes(), pos: 0, diags: Vec::new(), depth: 0 };
    p.skip_ws();
    if p.pos >= p.b.len() {
        return Parsed { root: None, diagnostics: p.diags };
    }
    let root = p.value().ok();
    if root.is_some() {
        p.skip_ws();
        if p.pos < p.b.len() {
            let range = p.token_range();
            p.diags.push(Diagnostic::error(range, t!("Unexpected content after the end of the document")));
        }
    }
    Parsed { root, diagnostics: p.diags }
}

struct Parser<'a> {
    s: &'a str,
    b: &'a [u8],
    pos: usize,
    diags: Vec<Diagnostic>,
    depth: usize,
}

/// Parse failure; the diagnostic is already recorded.
struct Fatal;

type R<T> = Result<T, Fatal>;

impl Parser<'_> {
    fn peek(&self) -> Option<u8> {
        self.b.get(self.pos).copied()
    }

    fn fatal<T>(&mut self, range: Range<usize>, message: impl Into<String>) -> R<T> {
        self.diags.push(Diagnostic::error(range, message));
        Err(Fatal)
    }

    fn skip_ws(&mut self) {
        loop {
            match self.peek() {
                Some(b' ' | b'\t' | b'\n' | b'\r') => self.pos += 1,
                Some(b'/') if matches!(self.b.get(self.pos + 1), Some(b'/' | b'*')) => {
                    let start = self.pos;
                    if self.b[self.pos + 1] == b'/' {
                        self.pos = self.s[start..].find('\n').map_or(self.b.len(), |i| start + i);
                    } else {
                        self.pos = self.s[start + 2..].find("*/").map_or(self.b.len(), |i| start + 2 + i + 2);
                    }
                    let end = self.s[start..self.pos].find('\n').map_or(self.pos, |i| start + i);
                    self.diags.push(Diagnostic::error(start..end, t!("Comments are not allowed in JSON")));
                }
                _ => return,
            }
        }
    }

    /// The range of the token at the current position (a word or one character).
    fn token_range(&self) -> Range<usize> {
        let start = self.pos;
        let word = word_end(self.s, start);
        if word > start {
            return start..word;
        }
        let len = self.s[start..].chars().next().map_or(0, char::len_utf8);
        start..start + len
    }

    /// Describes what is at the current position, for "expected … but found …".
    fn found(&self) -> String {
        match self.peek() {
            None => t!("the end of the document").into(),
            Some(b'"') => t!("a string").into(),
            Some(b'0'..=b'9' | b'-') => t!("a number").into(),
            Some(_) => format!("'{}'", &self.s[self.token_range()]),
        }
    }

    fn value(&mut self) -> R<Node> {
        let start = self.pos;
        match self.peek() {
            None => self.fatal(start..start, t!("Unexpected end of the document")),
            Some(b'{') => self.object(),
            Some(b'[') => self.array(),
            Some(b'"' | b'\'') => {
                let (s, range) = self.string()?;
                Ok(Node::new(Value::String(s), range))
            }
            Some(b'-' | b'+' | b'.' | b'0'..=b'9') => self.number(),
            Some(_) => {
                let range = self.token_range();
                let value = match &self.s[range.clone()] {
                    "true" => Value::Bool(true),
                    "false" => Value::Bool(false),
                    "null" => Value::Null,
                    word => {
                        let message = t!("Unexpected '{}'", word);
                        return self.fatal(range, message);
                    }
                };
                self.pos = range.end;
                Ok(Node::new(value, range))
            }
        }
    }

    fn enter(&mut self) -> R<()> {
        self.depth += 1;
        if self.depth > MAX_DEPTH {
            let range = self.pos..self.pos + 1;
            return self.fatal(range, t!("Nested too deeply"));
        }
        Ok(())
    }

    /// Starts a value (after a missing comma)?
    fn at_value_start(&self) -> bool {
        matches!(self.peek(), Some(b'{' | b'[' | b'"' | b'-' | b'0'..=b'9' | b't' | b'f' | b'n'))
    }

    fn object(&mut self) -> R<Node> {
        self.enter()?;
        let start = self.pos;
        self.pos += 1;
        let mut members: Vec<Member> = Vec::new();
        self.skip_ws();
        if self.peek() == Some(b'}') {
            self.pos += 1;
            self.depth -= 1;
            return Ok(Node::new(Value::Object(members), start..self.pos));
        }
        loop {
            let (key, key_range) = match self.peek() {
                Some(b'"' | b'\'') => self.string()?,
                Some(c) if c == b'_' || c == b'$' || c.is_ascii_alphabetic() => {
                    let range = self.token_range();
                    self.diags.push(Diagnostic::error(range.clone(), t!("Property names must be in double quotes")));
                    self.pos = range.end;
                    (self.s[range.clone()].to_owned(), range)
                }
                _ => {
                    let message = t!("Expected a property name but found {}", self.found());
                    let range = self.token_range();
                    return self.fatal(range, message);
                }
            };
            self.skip_ws();
            if self.peek() != Some(b':') {
                let message = t!("Expected ':' but found {}", self.found());
                let range = self.token_range();
                return self.fatal(range, message);
            }
            self.pos += 1;
            self.skip_ws();
            let value = self.value()?;
            if members.iter().any(|m| m.key == key) {
                self.diags.push(Diagnostic::warning(key_range.clone(), t!("Duplicate property \"{}\"", key)));
            }
            members.push(Member { key, key_range, value });
            self.skip_ws();
            match self.peek() {
                Some(b',') => {
                    let comma = self.pos;
                    self.pos += 1;
                    self.skip_ws();
                    if self.peek() == Some(b'}') {
                        self.diags.push(Diagnostic::error(comma..comma + 1, t!("Trailing comma")));
                        self.pos += 1;
                        break;
                    }
                }
                Some(b'}') => {
                    self.pos += 1;
                    break;
                }
                Some(b'"') => {
                    let at = self.pos;
                    self.diags.push(Diagnostic::error(at..at + 1, t!("Missing ','")));
                }
                _ => {
                    let message = t!("Expected ',' or '}' but found {}", self.found());
                    let range = self.token_range();
                    return self.fatal(range, message);
                }
            }
        }
        self.depth -= 1;
        Ok(Node::new(Value::Object(members), start..self.pos))
    }

    fn array(&mut self) -> R<Node> {
        self.enter()?;
        let start = self.pos;
        self.pos += 1;
        let mut items = Vec::new();
        self.skip_ws();
        if self.peek() == Some(b']') {
            self.pos += 1;
            self.depth -= 1;
            return Ok(Node::new(Value::Array(items), start..self.pos));
        }
        loop {
            items.push(self.value()?);
            self.skip_ws();
            match self.peek() {
                Some(b',') => {
                    let comma = self.pos;
                    self.pos += 1;
                    self.skip_ws();
                    if self.peek() == Some(b']') {
                        self.diags.push(Diagnostic::error(comma..comma + 1, t!("Trailing comma")));
                        self.pos += 1;
                        break;
                    }
                }
                Some(b']') => {
                    self.pos += 1;
                    break;
                }
                _ if self.at_value_start() => {
                    let at = self.pos;
                    self.diags.push(Diagnostic::error(at..at + 1, t!("Missing ','")));
                }
                _ => {
                    let message = t!("Expected ',' or ']' but found {}", self.found());
                    let range = self.token_range();
                    return self.fatal(range, message);
                }
            }
        }
        self.depth -= 1;
        Ok(Node::new(Value::Array(items), start..self.pos))
    }

    /// A string at the current position (double or, reported, single quoted).
    fn string(&mut self) -> R<(String, Range<usize>)> {
        let start = self.pos;
        let quote = self.b[start];
        if quote == b'\'' {
            self.diags.push(Diagnostic::error(start..start + 1, t!("Strings must use double quotes")));
        }
        self.pos += 1;
        let mut out = String::new();
        loop {
            let Some(c) = self.s[self.pos..].chars().next() else {
                return self.fatal(start..self.pos, t!("Unterminated string"));
            };
            let at = self.pos;
            self.pos += c.len_utf8();
            match c {
                c if c as u32 == quote as u32 => break,
                '\n' => {
                    return self.fatal(start..at, t!("Unterminated string"));
                }
                '\\' => {
                    let Some(e) = self.s[self.pos..].chars().next() else { continue };
                    self.pos += e.len_utf8();
                    match e {
                        '"' => out.push('"'),
                        '\\' => out.push('\\'),
                        '/' => out.push('/'),
                        'b' => out.push('\u{8}'),
                        'f' => out.push('\u{c}'),
                        'n' => out.push('\n'),
                        'r' => out.push('\r'),
                        't' => out.push('\t'),
                        'u' => match self.unicode_escape() {
                            Some(c) => out.push(c),
                            None => self.diags.push(Diagnostic::error(at..self.pos, t!("Invalid escape sequence"))),
                        },
                        '\'' if quote == b'\'' => out.push('\''),
                        _ => self.diags.push(Diagnostic::error(at..self.pos, t!("Invalid escape sequence"))),
                    }
                }
                c if (c as u32) < 0x20 => {
                    self.diags.push(Diagnostic::error(at..self.pos, t!("Control characters must be escaped")));
                    out.push(c);
                }
                c => out.push(c),
            }
        }
        Ok((out, start..self.pos))
    }

    /// The four hex digits after `\u` (and a following low surrogate).
    fn unicode_escape(&mut self) -> Option<char> {
        let hex = |p: &mut Self| -> Option<u32> {
            let digits = p.s.get(p.pos..p.pos + 4)?;
            let v = u32::from_str_radix(digits, 16).ok()?;
            p.pos += 4;
            Some(v)
        };
        let first = hex(self)?;
        if (0xD800..0xDC00).contains(&first) && self.s[self.pos..].starts_with("\\u") {
            let save = self.pos;
            self.pos += 2;
            if let Some(second) = hex(self).filter(|v| (0xDC00..0xE000).contains(v)) {
                return char::from_u32(0x10000 + ((first - 0xD800) << 10) + (second - 0xDC00));
            }
            self.pos = save;
        }
        Some(char::from_u32(first).unwrap_or('\u{FFFD}'))
    }

    fn number(&mut self) -> R<Node> {
        let start = self.pos;
        let end = self.s[start..]
            .find(|c: char| !(c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.')))
            .map_or(self.s.len(), |i| start + i);
        self.pos = end;
        let text = &self.s[start..end];
        let value = if valid_number(text) {
            text.parse::<f64>().unwrap_or(0.0)
        } else {
            self.diags.push(Diagnostic::error(start..end, t!("Invalid number")));
            text.trim_start_matches('+').parse::<f64>().unwrap_or(0.0)
        };
        Ok(Node::new(Value::Number(value), start..end))
    }
}

/// Matches the JSON number grammar.
fn valid_number(s: &str) -> bool {
    let b = s.as_bytes();
    let mut i = 0;
    if b.get(i) == Some(&b'-') {
        i += 1;
    }
    match b.get(i) {
        Some(b'0') => i += 1,
        Some(b'1'..=b'9') => {
            while b.get(i).is_some_and(u8::is_ascii_digit) {
                i += 1;
            }
        }
        _ => return false,
    }
    if b.get(i) == Some(&b'.') {
        i += 1;
        let digits = i;
        while b.get(i).is_some_and(u8::is_ascii_digit) {
            i += 1;
        }
        if i == digits {
            return false;
        }
    }
    if matches!(b.get(i), Some(b'e' | b'E')) {
        i += 1;
        if matches!(b.get(i), Some(b'+' | b'-')) {
            i += 1;
        }
        let digits = i;
        while b.get(i).is_some_and(u8::is_ascii_digit) {
            i += 1;
        }
        if i == digits {
            return false;
        }
    }
    i == b.len()
}

fn word_end(s: &str, start: usize) -> usize {
    s[start..].find(|c: char| !(c.is_alphanumeric() || c == '_' || c == '$')).map_or(s.len(), |i| start + i)
}

// ----- completion context -------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Expect {
    Key,
    Colon,
    Value,
    After,
}

#[derive(Clone, Debug)]
struct Frame {
    object: bool,
    expect: Expect,
    key: Option<String>,
    index: usize,
    keys: Vec<String>,
}

impl Frame {
    fn seg(&self) -> Seg {
        if self.object {
            Seg::Key(self.key.clone().unwrap_or_default())
        } else {
            Seg::Index(self.index)
        }
    }
}

/// What can be completed at `cursor` (a byte offset): a property name or a
/// value, with the path to it. Works on unfinished documents.
pub fn place(text: &str, cursor: usize) -> Option<Place> {
    let b = text.as_bytes();
    let mut stack: Vec<Frame> = Vec::new();
    let mut root_done = false;
    let mut i = 0;
    // Set once the cursor is reached: the place and the depth of its object
    // (whose remaining keys are still collected).
    let mut found: Option<(Place, usize)> = None;
    let seen_cursor = |stack: &Vec<Frame>, root_done: bool, token: Option<(Range<usize>, String)>| -> Option<Place> {
        let top = stack.last();
        let expect = top.map_or(if root_done { Expect::After } else { Expect::Value }, |f| f.expect);
        let (range, prefix) = token.unwrap_or((cursor..cursor, String::new()));
        let mut path: Vec<Seg> = stack.iter().take(stack.len().saturating_sub(1)).map(Frame::seg).collect();
        // The containers' own segments: each frame's key/index is the path
        // to its current child, so the path to the container itself is the
        // segments of the frames above it.
        let spot = match (top, expect) {
            (Some(f), Expect::Key) if f.object => Spot::Key { existing: f.keys.clone() },
            (Some(f), Expect::Value) => {
                path.push(f.seg());
                Spot::Value
            }
            (None, Expect::Value) => Spot::Value,
            _ => return None,
        };
        Some(Place { path, spot, range, prefix })
    };
    while i < b.len() {
        if found.is_none() && i >= cursor {
            found = seen_cursor(&stack, root_done, None).map(|p| (p, stack.len()));
            found.as_ref()?;
        }
        let c = b[i];
        match c {
            b' ' | b'\t' | b'\n' | b'\r' => i += 1,
            b'/' if b.get(i + 1) == Some(&b'/') => i = text[i..].find('\n').map_or(b.len(), |n| i + n),
            b'/' if b.get(i + 1) == Some(&b'*') => i = text[i + 2..].find("*/").map_or(b.len(), |n| i + 2 + n + 2),
            b'{' | b'[' => {
                stack.push(Frame { object: c == b'{', expect: if c == b'{' { Expect::Key } else { Expect::Value }, key: None, index: 0, keys: Vec::new() });
                i += 1;
            }
            b'}' | b']' => {
                if let Some((_, depth)) = &found {
                    if stack.len() == *depth {
                        break;
                    }
                }
                stack.pop();
                after_value(&mut stack, &mut root_done);
                i += 1;
            }
            b',' => {
                if let Some(f) = stack.last_mut() {
                    if f.object {
                        f.expect = Expect::Key;
                        f.key = None;
                    } else {
                        f.index += 1;
                        f.expect = Expect::Value;
                    }
                }
                i += 1;
            }
            b':' => {
                if let Some(f) = stack.last_mut().filter(|f| f.object) {
                    f.expect = Expect::Value;
                }
                i += 1;
            }
            b'"' | b'\'' => {
                let start = i;
                let mut j = i + 1;
                let mut closed = false;
                while j < b.len() {
                    match b[j] {
                        b'\\' => j += 2,
                        b'\n' => break,
                        q if q == c => {
                            j += 1;
                            closed = true;
                            break;
                        }
                        _ => j += 1,
                    }
                }
                let j = j.min(b.len());
                let content_end = if closed { j - 1 } else { j };
                let content = text.get(start + 1..content_end).unwrap_or("").to_owned();
                // The cursor is inside this string.
                if found.is_none() && start < cursor && (cursor < j || (!closed && cursor <= j)) {
                    let prefix = text.get(start + 1..cursor.min(content_end)).unwrap_or("").to_owned();
                    let p = seen_cursor(&stack, root_done, Some((start..j, prefix)))?;
                    found = Some((p, stack.len()));
                    // Keys of the object are collected below; the typed key is not one.
                    if let Some(f) = stack.last_mut() {
                        if f.object && f.expect == Expect::Key {
                            f.expect = Expect::Colon;
                        } else {
                            after_value(&mut stack, &mut root_done);
                        }
                    }
                    i = j;
                    continue;
                }
                match stack.last_mut() {
                    Some(f) if f.object && f.expect == Expect::Key => {
                        if let Some((p, depth)) = &mut found {
                            if *depth == stack.len() {
                                if let Spot::Key { existing } = &mut p.spot {
                                    existing.push(content.clone());
                                }
                            }
                        }
                        let f = stack.last_mut().expect("frame");
                        f.keys.push(content.clone());
                        f.key = Some(content);
                        f.expect = Expect::Colon;
                    }
                    _ => after_value(&mut stack, &mut root_done),
                }
                i = j;
            }
            _ => {
                let start = i;
                let end = text[i..]
                    .find(|ch: char| !(ch.is_alphanumeric() || matches!(ch, '_' | '$' | '.' | '-' | '+')))
                    .map_or(b.len(), |n| i + n)
                    .max(i + text[i..].chars().next().map_or(1, char::len_utf8));
                if found.is_none() && start < cursor && cursor <= end {
                    let prefix = text[start..cursor].to_owned();
                    let p = seen_cursor(&stack, root_done, Some((start..end, prefix)))?;
                    found = Some((p, stack.len()));
                }
                after_value(&mut stack, &mut root_done);
                i = end;
            }
        }
    }
    if found.is_none() {
        found = seen_cursor(&stack, root_done, None).map(|p| (p, stack.len()));
    }
    found.map(|(p, _)| p)
}

fn after_value(stack: &mut [Frame], root_done: &mut bool) {
    match stack.last_mut() {
        Some(f) => f.expect = Expect::After,
        None => *root_done = true,
    }
}

/// The text right after a key being completed already has its ':'.
pub fn has_colon(text: &str, end: usize) -> bool {
    text.get(end..).is_some_and(|rest| rest.trim_start_matches([' ', '\t']).starts_with(':'))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn messages(text: &str) -> Vec<String> {
        parse(text).diagnostics.into_iter().map(|d| d.message).collect()
    }

    #[test]
    fn valid_documents() {
        let p = parse(r#"{"a": [1, 2.5e3, -0.5, true, null], "b": {"c": "xé\n"}}"#);
        assert!(p.diagnostics.is_empty());
        let root = p.root.unwrap();
        assert_eq!(root.to_json(), serde_json::json!({"a": [1, 2500, -0.5, true, null], "b": {"c": "xé\n"}}));
        assert_eq!(root.member("b").unwrap().key_range, 36..39);
        assert!(parse("  \n").root.is_none());
        assert!(parse("  \n").diagnostics.is_empty());
    }

    #[test]
    fn syntax_problems() {
        assert_eq!(messages("{\"a\": 1,}"), ["Trailing comma"]);
        assert_eq!(messages("[1, 2,]"), ["Trailing comma"]);
        assert_eq!(messages("{'a': 1}"), ["Strings must use double quotes"]);
        assert_eq!(messages("{a: 1}"), ["Property names must be in double quotes"]);
        assert_eq!(messages("{\"a\": 1, \"a\": 2}"), ["Duplicate property \"a\""]);
        assert_eq!(messages("// x\n{}"), ["Comments are not allowed in JSON"]);
        assert_eq!(messages("[01, +1, 1.]"), ["Invalid number", "Invalid number", "Invalid number"]);
        assert_eq!(messages("{\"a\": 1 \"b\": 2}"), ["Missing ','"]);
        assert_eq!(messages("{\"a\" 1}"), ["Expected ':' but found a number"]);
        assert_eq!(messages("{\"a\": 1"), ["Expected ',' or '}' but found the end of the document"]);
        assert_eq!(messages("[1, NaN]"), ["Unexpected 'NaN'"]);
        assert_eq!(messages("\"abc\n\""), ["Unterminated string", ]);
        assert_eq!(messages("{} x"), ["Unexpected content after the end of the document"]);
        assert_eq!(messages("\"a\\x\""), ["Invalid escape sequence"]);
        let p = parse("{\"a\": tru}");
        assert!(p.root.is_none());
        assert_eq!(p.diagnostics[0].range, 6..9);
    }

    #[test]
    fn completion_places() {
        let at = |s: &str| {
            let cursor = s.find('|').unwrap();
            place(&s.replace('|', ""), cursor)
        };
        let p = at(r#"{"a": 1, "b": {"c": 2, |}, "d": 3}"#).unwrap();
        assert_eq!(p.path, vec![Seg::Key("b".into())]);
        assert_eq!(p.spot, Spot::Key { existing: vec!["c".into()] });
        let p = at(r#"{"a": 1, "|"#).unwrap();
        assert_eq!(p.path, vec![]);
        assert_eq!(p.spot, Spot::Key { existing: vec!["a".into()] });
        assert_eq!(p.range, 9..10);
        let p = at(r#"{"a": 1, "x|y": 2, "z": 3}"#).unwrap();
        assert_eq!(p.prefix, "x");
        assert_eq!(p.range, 9..13);
        assert_eq!(p.spot, Spot::Key { existing: vec!["a".into(), "z".into()] });
        let p = at(r#"{"a": {"b": [1, {"c": |"#).unwrap();
        assert_eq!(p.path, vec![Seg::Key("a".into()), Seg::Key("b".into()), Seg::Index(1), Seg::Key("c".into())]);
        assert_eq!(p.spot, Spot::Value);
        let p = at(r#"{"a": "v|"}"#).unwrap();
        assert_eq!((p.prefix.as_str(), p.range.clone(), p.spot), ("v", 6..9, Spot::Value));
        let p = at(r#"{"a": tr|}"#).unwrap();
        assert_eq!((p.prefix.as_str(), p.range.clone()), ("tr", 6..8));
        assert!(at(r#"{"a": 1 |}"#).is_none());
        assert!(at(r#"{"a" |: 1}"#).is_none());
        assert_eq!(at("|").unwrap().spot, Spot::Value);
        assert!(has_colon("\"a\" : 1", 3));
    }
}
