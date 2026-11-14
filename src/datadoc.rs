//! Shared types of the data format checks (JSON, YAML, XML): problems found
//! in a document and parsed values with their source positions.

use serde_json::Value as Json;
use std::ops::Range;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Error,
    Warning,
}

/// A problem at a byte range of the document.
#[derive(Clone, Debug, PartialEq)]
pub struct Diagnostic {
    pub range: Range<usize>,
    pub severity: Severity,
    pub message: String,
}

impl Diagnostic {
    pub fn new(range: Range<usize>, severity: Severity, message: impl Into<String>) -> Self {
        Self { range, severity, message: message.into() }
    }

    pub fn error(range: Range<usize>, message: impl Into<String>) -> Self {
        Self::new(range, Severity::Error, message)
    }

    pub fn warning(range: Range<usize>, message: impl Into<String>) -> Self {
        Self::new(range, Severity::Warning, message)
    }
}

/// A parsed value and where it is in the source.
#[derive(Clone, Debug, PartialEq)]
pub struct Node {
    pub value: Value,
    pub range: Range<usize>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<Node>),
    Object(Vec<Member>),
}

/// A property of an object.
#[derive(Clone, Debug, PartialEq)]
pub struct Member {
    pub key: String,
    pub key_range: Range<usize>,
    pub value: Node,
}

impl Node {
    pub fn new(value: Value, range: Range<usize>) -> Self {
        Self { value, range }
    }

    /// The JSON type name ("integer" for whole numbers).
    pub fn type_name(&self) -> &'static str {
        match &self.value {
            Value::Null => "null",
            Value::Bool(_) => "boolean",
            Value::Number(n) if n.fract() == 0.0 && n.is_finite() => "integer",
            Value::Number(_) => "number",
            Value::String(_) => "string",
            Value::Array(_) => "array",
            Value::Object(_) => "object",
        }
    }

    pub fn to_json(&self) -> Json {
        match &self.value {
            Value::Null => Json::Null,
            Value::Bool(b) => Json::Bool(*b),
            Value::Number(n) => number_json(*n),
            Value::String(s) => Json::String(s.clone()),
            Value::Array(items) => Json::Array(items.iter().map(Node::to_json).collect()),
            Value::Object(members) => Json::Object(members.iter().map(|m| (m.key.clone(), m.value.to_json())).collect()),
        }
    }

    pub fn member(&self, key: &str) -> Option<&Member> {
        match &self.value {
            Value::Object(members) => members.iter().find(|m| m.key == key),
            _ => None,
        }
    }

    /// The node at a path below this one.
    #[cfg(test)]
    pub fn at(&self, path: &[Seg]) -> Option<&Node> {
        let Some((first, rest)) = path.split_first() else { return Some(self) };
        let next = match (first, &self.value) {
            (Seg::Key(k), Value::Object(members)) => &members.iter().find(|m| &m.key == k)?.value,
            (Seg::Index(i), Value::Array(items)) => items.get(*i)?,
            _ => return None,
        };
        next.at(rest)
    }
}

pub fn number_json(n: f64) -> Json {
    if n.fract() == 0.0 && n.abs() < 9.0e15 {
        Json::from(n as i64)
    } else {
        serde_json::Number::from_f64(n).map_or(Json::Null, Json::Number)
    }
}

/// A step of a path into a document.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Seg {
    Key(String),
    Index(usize),
}

/// What the cursor is on when completing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Spot {
    /// A property name of the object at the path; names already there are listed.
    Key { existing: Vec<String> },
    /// The value at the path.
    Value,
}

/// A completion request: where the replaced text starts and ends (byte
/// offsets), the path and what is completed there.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Place {
    pub path: Vec<Seg>,
    pub spot: Spot,
    pub range: Range<usize>,
    /// The text typed so far (without quotes).
    pub prefix: String,
}

/// A completion proposal.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Proposal {
    pub label: String,
    pub insert: String,
    pub info: String,
    /// Characters to move the cursor back after inserting (into quotes or brackets).
    pub back: usize,
}

/// Byte offset of the start of each line.
pub fn line_starts(text: &str) -> Vec<usize> {
    std::iter::once(0).chain(text.match_indices('\n').map(|(i, _)| i + 1)).collect()
}

/// Byte offset of a 0-based line and character column (clamped to the line).
pub fn offset_of(text: &str, starts: &[usize], line: usize, col: usize) -> usize {
    let Some(&start) = starts.get(line) else { return text.len() };
    let end = starts.get(line + 1).map_or(text.len(), |e| e - 1);
    let line_text = &text[start..end];
    start + line_text.char_indices().nth(col).map_or(line_text.len(), |(i, _)| i)
}

/// Widens an empty range to the character before or after it, so it can be
/// underlined.
pub fn visible(text: &str, range: Range<usize>) -> Range<usize> {
    if range.end > range.start {
        return range;
    }
    let at = range.start.min(text.len());
    if let Some(c) = text[at..].chars().next().filter(|c| *c != '\n') {
        return at..at + c.len_utf8();
    }
    match text[..at].chars().next_back() {
        Some(c) if c != '\n' => at - c.len_utf8()..at,
        _ => at..at,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn positions() {
        let text = "ab\nüx\n";
        let starts = line_starts(text);
        assert_eq!(starts, vec![0, 3, 7]);
        assert_eq!(offset_of(text, &starts, 1, 1), 5);
        assert_eq!(offset_of(text, &starts, 1, 9), 6);
        assert_eq!(visible(text, 2..2), 1..2);
        assert_eq!(visible(text, 0..0), 0..1);
    }
}
