//! Text transformations: beautifiers for data formats and small text tools.
//! Everything here is pure and independent of the UI.

use base64::Engine;
use quick_xml::events::Event;
use quick_xml::{Reader, Writer};

pub type FmtResult = Result<String, String>;

/// Language id (GtkSourceView) that fits the output of a tool, if any.
pub fn output_language(tool: &str) -> Option<&'static str> {
    match tool {
        "json-pretty" | "json-minify" | "yaml-to-json" => Some("json"),
        "xml-pretty" | "xml-minify" => Some("xml"),
        "yaml-pretty" | "json-to-yaml" => Some("yaml"),
        "sql-pretty" => Some("sql"),
        _ => None,
    }
}

/// Applies the tool with the given id to `input`.
/// `language` is the current language id and is used by `beautify`/`minify` to pick a format.
pub fn apply(tool: &str, input: &str, language: Option<&str>) -> Result<(String, Option<&'static str>), String> {
    let tool = match tool {
        "beautify" => detect(input, language)
            .map(|f| match f {
                Format::Json => "json-pretty",
                Format::Xml => "xml-pretty",
                Format::Yaml => "yaml-pretty",
                Format::Sql => "sql-pretty",
            })
            .ok_or("Couldn't detect the format. Pick one in the Format menu.")?,
        "minify" => match detect(input, language) {
            Some(Format::Json) => "json-minify",
            Some(Format::Xml) => "xml-minify",
            _ => return Err("Minify works for JSON and XML only.".into()),
        },
        t => t,
    };
    let out = match tool {
        "json-pretty" => json_reformat(input, Some(4)),
        "json-minify" => json_reformat(input, None),
        "xml-pretty" => xml_reformat(input, true),
        "xml-minify" => xml_reformat(input, false),
        "yaml-pretty" => yaml_pretty(input),
        "sql-pretty" => Ok(sql_pretty(input)),
        "json-to-yaml" => json_to_yaml(input),
        "yaml-to-json" => yaml_to_json(input),
        "sort-lines" => Ok(map_lines(input, |l| l.sort())),
        "sort-lines-ci" => Ok(map_lines(input, |l| l.sort_by_key(|s| s.to_lowercase()))),
        "reverse-lines" => Ok(map_lines(input, |l| l.reverse())),
        "unique-lines" => Ok(map_lines(input, |l| {
            let mut seen = std::collections::HashSet::new();
            l.retain(|s| seen.insert(s.clone()));
        })),
        "trim-trailing" => Ok(map_lines(input, |l| {
            for s in l.iter_mut() {
                s.truncate(s.trim_end().len());
            }
        })),
        "remove-empty-lines" => Ok(map_lines(input, |l| l.retain(|s| !s.trim().is_empty()))),
        "upper" => Ok(input.to_uppercase()),
        "lower" => Ok(input.to_lowercase()),
        "base64-encode" => Ok(base64::engine::general_purpose::STANDARD.encode(input)),
        "base64-decode" => base64_decode(input),
        "url-encode" => Ok(urlencoding::encode(input).into_owned()),
        "url-decode" => urlencoding::decode(input)
            .map(|s| s.into_owned())
            .map_err(|e| format!("URL decode: {e}")),
        "json-escape" => serde_json::to_string(input).map_err(|e| e.to_string()),
        "json-unescape" => json_unescape(input),
        other => Err(format!("Unknown tool: {other}")),
    }?;
    Ok((out, output_language(tool)))
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Format {
    Json,
    Xml,
    Yaml,
    Sql,
}

pub fn detect(input: &str, language: Option<&str>) -> Option<Format> {
    match language {
        Some("json") => return Some(Format::Json),
        Some("xml") | Some("xslt") | Some("svg") => return Some(Format::Xml),
        Some("yaml") => return Some(Format::Yaml),
        Some("sql") => return Some(Format::Sql),
        _ => {}
    }
    let t = input.trim_start();
    if t.starts_with('{') || t.starts_with('[') {
        Some(Format::Json)
    } else if t.starts_with('<') {
        Some(Format::Xml)
    } else {
        None
    }
}

/// Re-indents JSON without parsing into a value model, so key order, number
/// literals and duplicate keys are preserved exactly. `indent == None` minifies.
pub fn json_reformat(input: &str, indent: Option<usize>) -> FmtResult {
    // Validate first to get a proper error message with position.
    serde_json::from_str::<serde::de::IgnoredAny>(input).map_err(|e| format!("Invalid JSON: {e}"))?;

    let unit = " ".repeat(indent.unwrap_or(0));
    let pretty = indent.is_some();
    let mut out = String::with_capacity(input.len() + input.len() / 4);
    let mut level = 0usize;
    let mut chars = input.chars().peekable();
    let newline = |out: &mut String, level: usize| {
        out.push('\n');
        for _ in 0..level {
            out.push_str(&unit);
        }
    };

    while let Some(c) = chars.next() {
        match c {
            '"' => {
                out.push('"');
                let mut escaped = false;
                for c in chars.by_ref() {
                    out.push(c);
                    if escaped {
                        escaped = false;
                    } else if c == '\\' {
                        escaped = true;
                    } else if c == '"' {
                        break;
                    }
                }
            }
            '{' | '[' => {
                // Keep empty containers compact.
                while chars.peek().is_some_and(|c| c.is_whitespace()) {
                    chars.next();
                }
                let close = if c == '{' { '}' } else { ']' };
                if chars.peek() == Some(&close) {
                    chars.next();
                    out.push(c);
                    out.push(close);
                } else {
                    out.push(c);
                    level += 1;
                    if pretty {
                        newline(&mut out, level);
                    }
                }
            }
            '}' | ']' => {
                level = level.saturating_sub(1);
                if pretty {
                    newline(&mut out, level);
                }
                out.push(c);
            }
            ',' => {
                out.push(',');
                if pretty {
                    newline(&mut out, level);
                }
            }
            ':' => out.push_str(if pretty { ": " } else { ":" }),
            c if c.is_whitespace() => {}
            c => out.push(c),
        }
    }
    if pretty {
        out.push('\n');
    }
    Ok(out)
}

pub fn xml_reformat(input: &str, pretty: bool) -> FmtResult {
    let mut reader = Reader::from_str(input);
    reader.config_mut().trim_text(true);
    let mut writer = if pretty {
        Writer::new_with_indent(Vec::new(), b' ', 4)
    } else {
        Writer::new(Vec::new())
    };
    let mut depth: i64 = 0;
    loop {
        match reader.read_event() {
            Ok(Event::Eof) => break,
            Ok(e) => {
                match &e {
                    Event::Start(_) => depth += 1,
                    Event::End(_) => depth -= 1,
                    _ => {}
                }
                writer.write_event(e).map_err(|e| format!("XML: {e}"))?;
            }
            Err(e) => {
                return Err(format!("Invalid XML at byte {}: {e}", reader.error_position()));
            }
        }
    }
    if depth != 0 {
        return Err("Invalid XML: unclosed element".into());
    }
    let mut out = String::from_utf8(writer.into_inner()).map_err(|e| e.to_string())?;
    if pretty {
        out.push('\n');
    }
    Ok(out)
}

fn yaml_docs<T: serde::de::DeserializeOwned>(input: &str) -> Result<Vec<T>, String> {
    serde_yaml::Deserializer::from_str(input)
        .map(|doc| T::deserialize(doc).map_err(|e| format!("Invalid YAML: {e}")))
        .collect()
}

pub fn yaml_pretty(input: &str) -> FmtResult {
    let docs: Vec<serde_yaml::Value> = yaml_docs(input)?;
    let parts: Result<Vec<String>, String> = docs
        .iter()
        .map(|d| serde_yaml::to_string(d).map_err(|e| e.to_string()))
        .collect();
    Ok(parts?.join("---\n"))
}

pub fn json_to_yaml(input: &str) -> FmtResult {
    let v: serde_json::Value = serde_json::from_str(input).map_err(|e| format!("Invalid JSON: {e}"))?;
    serde_yaml::to_string(&v).map_err(|e| e.to_string())
}

pub fn yaml_to_json(input: &str) -> FmtResult {
    let docs: Vec<serde_json::Value> = yaml_docs(input)?;
    let value = match docs.len() {
        0 => serde_json::Value::Null,
        1 => docs.into_iter().next().unwrap(),
        _ => serde_json::Value::Array(docs),
    };
    let mut out = serde_json::to_string_pretty(&value).map_err(|e| e.to_string())?;
    out.push('\n');
    // serde_json pretty-prints with 2 spaces; re-indent to match `json-pretty`.
    json_reformat(&out, Some(4))
}

pub fn sql_pretty(input: &str) -> String {
    let opts = sqlformat::FormatOptions {
        indent: sqlformat::Indent::Spaces(4),
        uppercase: true,
        lines_between_queries: 2,
    };
    let mut out = sqlformat::format(input, &sqlformat::QueryParams::None, opts);
    out.push('\n');
    out
}

fn base64_decode(input: &str) -> FmtResult {
    let cleaned: String = input.chars().filter(|c| !c.is_whitespace()).collect();
    let engines = [
        base64::engine::general_purpose::STANDARD,
        base64::engine::general_purpose::STANDARD_NO_PAD,
        base64::engine::general_purpose::URL_SAFE,
        base64::engine::general_purpose::URL_SAFE_NO_PAD,
    ];
    let bytes = engines
        .iter()
        .find_map(|e| e.decode(&cleaned).ok())
        .ok_or("Not valid Base64")?;
    String::from_utf8(bytes).map_err(|_| "Decoded data is not UTF-8 text".into())
}

fn json_unescape(input: &str) -> FmtResult {
    let t = input.trim();
    let quoted = if t.starts_with('"') && t.ends_with('"') && t.len() >= 2 {
        t.to_string()
    } else {
        format!("\"{t}\"")
    };
    serde_json::from_str::<String>(&quoted).map_err(|e| format!("Invalid JSON string: {e}"))
}

/// Applies `f` to the lines of `input`, keeping a trailing newline if present.
fn map_lines(input: &str, f: impl FnOnce(&mut Vec<String>)) -> String {
    let trailing = input.ends_with('\n');
    let mut lines: Vec<String> = input.lines().map(str::to_owned).collect();
    f(&mut lines);
    let mut out = lines.join("\n");
    if trailing {
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_pretty_preserves_order_and_numbers() {
        let out = json_reformat(r#"{"b":1.50,"a":[1,2,{}],"c":"x, \"y\": z","d":[ ]}"#, Some(4)).unwrap();
        assert_eq!(
            out,
            "{\n    \"b\": 1.50,\n    \"a\": [\n        1,\n        2,\n        {}\n    ],\n    \"c\": \"x, \\\"y\\\": z\",\n    \"d\": []\n}\n"
        );
    }

    #[test]
    fn json_minify() {
        let out = json_reformat("{\n  \"a\" : [ 1 , 2 ],\n  \"s\": \"a b\"\n}", None).unwrap();
        assert_eq!(out, r#"{"a":[1,2],"s":"a b"}"#);
    }

    #[test]
    fn json_invalid() {
        assert!(json_reformat("{\"a\":}", Some(4)).is_err());
    }

    #[test]
    fn xml_pretty() {
        let out = xml_reformat("<a><b x=\"1\">t &amp; u</b><c/></a>", true).unwrap();
        assert_eq!(out, "<a>\n    <b x=\"1\">t &amp; u</b>\n    <c/>\n</a>\n");
        assert!(xml_reformat("<a><b></a>", true).is_err());
        assert!(xml_reformat("<a><b>", true).is_err());
    }

    #[test]
    fn yaml_roundtrip() {
        assert_eq!(yaml_pretty("a:   1\nb: [x, y]\n").unwrap(), "a: 1\nb:\n- x\n- y\n");
        assert_eq!(json_to_yaml(r#"{"z":1,"a":2}"#).unwrap(), "z: 1\na: 2\n");
        assert_eq!(yaml_to_json("a: 1\n").unwrap(), "{\n    \"a\": 1\n}\n");
    }

    #[test]
    fn beautify_detects_format() {
        let (out, lang) = apply("beautify", "[1,2]", None).unwrap();
        assert_eq!(out, "[\n    1,\n    2\n]\n");
        assert_eq!(lang, Some("json"));
        assert!(apply("beautify", "hello", None).is_err());
    }

    #[test]
    fn line_tools() {
        assert_eq!(apply("sort-lines", "b\na\n", None).unwrap().0, "a\nb\n");
        assert_eq!(apply("unique-lines", "a\nb\na", None).unwrap().0, "a\nb");
        assert_eq!(apply("trim-trailing", "a  \nb\t\n", None).unwrap().0, "a\nb\n");
    }

    #[test]
    fn encodings() {
        assert_eq!(apply("base64-encode", "hi", None).unwrap().0, "aGk=");
        assert_eq!(apply("base64-decode", "aGk", None).unwrap().0, "hi");
        assert_eq!(apply("url-encode", "a b&c", None).unwrap().0, "a%20b%26c");
        assert_eq!(apply("json-unescape", r#"a\n\"b\""#, None).unwrap().0, "a\n\"b\"");
        assert_eq!(apply("json-escape", "a\"b", None).unwrap().0, r#""a\"b""#);
    }
}
