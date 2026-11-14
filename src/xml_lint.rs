//! XML checking with libxml2 (already loaded with WebKitGTK): well-formedness
//! and validation against XML Schemas (XSD).
//!
//! Remote schemas are never fetched by libxml2 itself: an external entity
//! loader maps their URLs to the downloaded copies (see `schemas.rs`), and
//! documents are parsed with `XML_PARSE_NONET`.

use crate::datadoc::{line_starts, offset_of, visible, Diagnostic, Severity};
use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::{c_char, c_int, c_void, CStr, CString};
use std::path::PathBuf;

#[repr(C)]
struct XmlError {
    domain: c_int,
    code: c_int,
    message: *const c_char,
    level: c_int,
    file: *const c_char,
    line: c_int,
    str1: *const c_char,
    str2: *const c_char,
    str3: *const c_char,
    int1: c_int,
    int2: c_int,
    ctxt: *mut c_void,
    node: *mut c_void,
}

type ErrorFunc = unsafe extern "C" fn(*mut c_void, *const XmlError);
type EntityLoader = unsafe extern "C" fn(*const c_char, *const c_char, *mut c_void) -> *mut c_void;

#[link(name = "xml2")]
extern "C" {
    fn xmlInitParser();
    fn xmlReadMemory(buffer: *const c_char, size: c_int, url: *const c_char, encoding: *const c_char, options: c_int) -> *mut c_void;
    fn xmlFreeDoc(doc: *mut c_void);
    fn xmlSetStructuredErrorFunc(ctx: *mut c_void, handler: Option<ErrorFunc>);
    fn xmlSchemaNewParserCtxt(url: *const c_char) -> *mut c_void;
    fn xmlSchemaNewMemParserCtxt(buffer: *const c_char, size: c_int) -> *mut c_void;
    fn xmlSchemaSetParserStructuredErrors(ctxt: *mut c_void, handler: Option<ErrorFunc>, ctx: *mut c_void);
    fn xmlSchemaParse(ctxt: *mut c_void) -> *mut c_void;
    fn xmlSchemaFreeParserCtxt(ctxt: *mut c_void);
    fn xmlSchemaFree(schema: *mut c_void);
    fn xmlSchemaNewValidCtxt(schema: *mut c_void) -> *mut c_void;
    fn xmlSchemaSetValidStructuredErrors(ctxt: *mut c_void, handler: Option<ErrorFunc>, ctx: *mut c_void);
    fn xmlSchemaValidateDoc(ctxt: *mut c_void, doc: *mut c_void) -> c_int;
    fn xmlSchemaFreeValidCtxt(ctxt: *mut c_void);
    fn xmlGetExternalEntityLoader() -> Option<EntityLoader>;
    fn xmlSetExternalEntityLoader(loader: Option<EntityLoader>);
    fn xmlNewInputFromFile(ctxt: *mut c_void, filename: *const c_char) -> *mut c_void;
}

const XML_PARSE_NONET: c_int = 1 << 11;
const XML_PARSE_BIG_LINES: c_int = 1 << 22;
const LEVEL_WARNING: c_int = 1;

/// A problem as libxml2 reports it.
#[derive(Clone, Debug)]
struct Raw {
    level: c_int,
    line: usize,
    col: usize,
    message: String,
}

/// libxml2's own entity loader (the loader setting is process-wide, so the
/// fallback must be too).
static DEFAULT_LOADER: std::sync::OnceLock<usize> = std::sync::OnceLock::new();

thread_local! {
    /// Problems collected by the error handler during a call.
    static COLLECTED: RefCell<Vec<Raw>> = const { RefCell::new(Vec::new()) };
    /// Downloaded schema files by URL, for the entity loader.
    static LOCAL: RefCell<HashMap<String, PathBuf>> = RefCell::new(HashMap::new());
    /// Parsed schemas by the set of locations they were made from.
    static PARSED: RefCell<HashMap<String, Result<usize, Vec<String>>>> = RefCell::new(HashMap::new());
}

unsafe extern "C" fn collect(_ctx: *mut c_void, e: *const XmlError) {
    let Some(e) = e.as_ref() else { return };
    let message = if e.message.is_null() { String::new() } else { CStr::from_ptr(e.message).to_string_lossy().trim().to_owned() };
    let raw = Raw { level: e.level, line: e.line.max(0) as usize, col: e.int2.max(0) as usize, message };
    COLLECTED.with(|c| c.borrow_mut().push(raw));
}

unsafe extern "C" fn load_entity(url: *const c_char, id: *const c_char, ctxt: *mut c_void) -> *mut c_void {
    if !url.is_null() {
        let u = CStr::from_ptr(url).to_string_lossy().into_owned();
        if let Some(path) = LOCAL.with(|l| l.borrow().get(&u).cloned()) {
            if let Ok(p) = CString::new(path.to_string_lossy().as_bytes()) {
                return xmlNewInputFromFile(ctxt, p.as_ptr());
            }
        }
        if u.starts_with("http://") || u.starts_with("https://") {
            return std::ptr::null_mut();
        }
    }
    match DEFAULT_LOADER.get() {
        Some(&f) => {
            let f: EntityLoader = std::mem::transmute(f);
            f(url, id, ctxt)
        }
        None => std::ptr::null_mut(),
    }
}

/// Runs `f` with the error handler and entity loader in place.
fn with_handlers<T>(f: impl FnOnce() -> T) -> (T, Vec<Raw>) {
    unsafe {
        xmlInitParser();
        let previous = xmlGetExternalEntityLoader();
        if let Some(p) = previous.filter(|p| *p as usize != load_entity as EntityLoader as usize) {
            DEFAULT_LOADER.get_or_init(|| p as usize);
        }
        xmlSetExternalEntityLoader(Some(load_entity));
        xmlSetStructuredErrorFunc(std::ptr::null_mut(), Some(collect));
        COLLECTED.with(|c| c.borrow_mut().clear());
        let result = f();
        xmlSetStructuredErrorFunc(std::ptr::null_mut(), None);
        xmlSetExternalEntityLoader(previous);
        (result, COLLECTED.with(|c| std::mem::take(&mut *c.borrow_mut())))
    }
}

/// Tells the entity loader where downloaded schemas are; clears parsed schemas.
pub fn set_local_files(files: HashMap<String, PathBuf>) {
    LOCAL.with(|l| *l.borrow_mut() = files);
    PARSED.with(|p| {
        for schema in p.borrow_mut().drain().filter_map(|(_, v)| v.ok()) {
            unsafe { xmlSchemaFree(schema as *mut c_void) };
        }
    });
}

/// A schema to validate against: its namespace (`None` for
/// `noNamespaceSchemaLocation`) and URL or file path.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Location {
    pub namespace: Option<String>,
    pub url: String,
}

/// The parsed schema for a set of locations (cached), or its problems.
fn schema_for(locations: &[Location]) -> Result<usize, Vec<String>> {
    let key = locations.iter().map(|l| format!("{} {}", l.namespace.as_deref().unwrap_or(""), l.url)).collect::<Vec<_>>().join("\n");
    if let Some(found) = PARSED.with(|p| p.borrow().get(&key).cloned()) {
        return found;
    }
    let (schema, raws) = with_handlers(|| unsafe {
        let ctxt = if let [only] = locations {
            let url = CString::new(only.url.as_bytes()).unwrap_or_default();
            xmlSchemaNewParserCtxt(url.as_ptr())
        } else {
            // Several namespaces: a schema that imports them all.
            let mut doc = String::from("<xs:schema xmlns:xs=\"http://www.w3.org/2001/XMLSchema\">");
            for l in locations.iter().filter(|l| l.namespace.is_none()) {
                doc.push_str(&format!("<xs:include schemaLocation=\"{}\"/>", escape(&l.url)));
            }
            for l in locations.iter().filter(|l| l.namespace.is_some()) {
                doc.push_str(&format!(
                    "<xs:import namespace=\"{}\" schemaLocation=\"{}\"/>",
                    escape(l.namespace.as_deref().unwrap_or("")),
                    escape(&l.url)
                ));
            }
            doc.push_str("</xs:schema>");
            xmlSchemaNewMemParserCtxt(doc.as_ptr() as *const c_char, doc.len() as c_int)
        };
        if ctxt.is_null() {
            return std::ptr::null_mut();
        }
        xmlSchemaSetParserStructuredErrors(ctxt, Some(collect), std::ptr::null_mut());
        let schema = xmlSchemaParse(ctxt);
        xmlSchemaFreeParserCtxt(ctxt);
        schema
    });
    let result = if schema.is_null() {
        let mut problems: Vec<String> = raws.into_iter().filter(|r| r.level > LEVEL_WARNING).map(|r| r.message).collect();
        if problems.is_empty() {
            problems.push(t!("The schema could not be read").into());
        }
        Err(problems)
    } else {
        Ok(schema as usize)
    };
    PARSED.with(|p| p.borrow_mut().insert(key, result.clone()));
    result
}

fn escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('"', "&quot;").replace('<', "&lt;")
}

/// Checks a document: well-formedness and, with `locations`, validity.
pub fn check(text: &str, base_url: Option<&str>, locations: &[Location]) -> Vec<Diagnostic> {
    if text.trim().is_empty() {
        return Vec::new();
    }
    let url = base_url.and_then(|u| CString::new(u).ok());
    let (doc, raws) = with_handlers(|| unsafe {
        xmlReadMemory(
            text.as_ptr() as *const c_char,
            text.len() as c_int,
            url.as_ref().map_or(std::ptr::null(), |u| u.as_ptr()),
            std::ptr::null(),
            XML_PARSE_NONET | XML_PARSE_BIG_LINES,
        )
    });
    let starts = line_starts(text);
    let mut out: Vec<Diagnostic> = raws.iter().map(|r| to_diagnostic(text, &starts, r)).collect();
    if doc.is_null() {
        return out;
    }
    if !locations.is_empty() {
        match schema_for(locations) {
            Ok(schema) => {
                let (_, raws) = with_handlers(|| unsafe {
                    let ctxt = xmlSchemaNewValidCtxt(schema as *mut c_void);
                    if !ctxt.is_null() {
                        xmlSchemaSetValidStructuredErrors(ctxt, Some(collect), std::ptr::null_mut());
                        xmlSchemaValidateDoc(ctxt, doc);
                        xmlSchemaFreeValidCtxt(ctxt);
                    }
                });
                out.extend(raws.iter().map(|r| to_diagnostic(text, &starts, r)));
            }
            Err(problems) => {
                let range = root_tag(text);
                for p in problems.into_iter().take(3) {
                    out.push(Diagnostic::warning(range.clone(), t!("Schema problem: {}", p)));
                }
            }
        }
    }
    unsafe { xmlFreeDoc(doc) };
    out
}

/// The name of the root element (where schema problems are shown).
fn root_tag(text: &str) -> std::ops::Range<usize> {
    let mut at = 0;
    while let Some(i) = text[at..].find('<') {
        let start = at + i;
        let rest = &text[start + 1..];
        if !rest.starts_with(['?', '!']) {
            let len = rest.find(|c: char| c.is_whitespace() || c == '>' || c == '/').unwrap_or(rest.len());
            return start + 1..start + 1 + len;
        }
        at = start + 1;
    }
    0..0
}

/// Maps a libxml2 problem to a range: the reported column, or for validity
/// problems (which only have a line) the element or attribute named in the
/// message.
fn to_diagnostic(text: &str, starts: &[usize], r: &Raw) -> Diagnostic {
    let severity = if r.level <= LEVEL_WARNING { Severity::Warning } else { Severity::Error };
    let line = r.line.saturating_sub(1).min(starts.len().saturating_sub(1));
    let line_start = starts.get(line).copied().unwrap_or(0);
    let line_end = starts.get(line + 1).map_or(text.len(), |e| e - 1);
    let line_text = &text[line_start..line_end];
    let quoted = |label: &str| -> Option<String> {
        let at = r.message.find(label)? + label.len();
        let rest = &r.message[at..];
        let name = &rest[..rest.find('\'')?];
        // `{namespace}local` → local
        Some(name.rsplit('}').next().unwrap_or(name).to_owned())
    };
    let mut range = None;
    if let Some(attr) = quoted("attribute '") {
        range = find_name(line_text, &attr, "=").map(|(s, e)| line_start + s..line_start + e);
    }
    if range.is_none() {
        if let Some(el) = quoted("Element '") {
            range = find_name(line_text, &el, "<").map(|(s, e)| line_start + s..line_start + e);
        }
    }
    let range = range.unwrap_or_else(|| {
        if r.col > 0 {
            let at = offset_of(text, starts, line, r.col - 1);
            visible(text, at..at)
        } else {
            let indent = line_text.len() - line_text.trim_start().len();
            line_start + indent..line_end
        }
    });
    let message = r.message.trim_end_matches('.').to_owned() + ".";
    Diagnostic::new(range, severity, message.replace("..", "."))
}

/// Finds `name` (optionally prefixed) on a line, after `<` for elements or
/// before `=` for attributes.
fn find_name(line: &str, name: &str, marker: &str) -> Option<(usize, usize)> {
    let mut at = 0;
    while let Some(i) = line[at..].find(name) {
        let s = at + i;
        let e = s + name.len();
        let before = &line[..s];
        let before_ok = before.ends_with('<') || before.ends_with(':') || before.ends_with(char::is_whitespace);
        let after = line[e..].trim_start();
        let ok = if marker == "<" {
            before_ok && (before.trim_end_matches(|c: char| c.is_alphanumeric() || c == '_' || c == '-' || c == '.').ends_with('<') || before.ends_with('<'))
                && line[e..].starts_with(|c: char| c.is_whitespace() || c == '>' || c == '/')
        } else {
            before_ok && after.starts_with('=')
        };
        if ok {
            let start = if marker == "<" && before.ends_with(':') {
                before.rfind('<').map_or(s, |p| p + 1)
            } else {
                s
            };
            return Some((start, e));
        }
        at = e;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn messages(text: &str, locations: &[Location]) -> Vec<(String, String)> {
        check(text, None, locations).into_iter().map(|d| (text[d.range.clone()].to_owned(), d.message)).collect()
    }

    #[test]
    fn well_formedness() {
        assert!(messages("<a><b/></a>", &[]).is_empty());
        let m = messages("<a>\n  <b></c>\n</a>", &[]);
        assert!(!m.is_empty());
        assert!(m[0].1.contains("mismatch"), "{m:?}");
        let m = messages("<a x=\"1\" x=\"2\"/>", &[]);
        assert!(m[0].1.contains("redefined"), "{m:?}");
    }

    #[test]
    fn validates_against_xsd() {
        let dir = std::env::temp_dir().join(format!("qn-xsd-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let xsd = dir.join("s.xsd");
        std::fs::write(
            &xsd,
            r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">
  <xs:element name="config">
    <xs:complexType>
      <xs:sequence>
        <xs:element name="port" type="xs:int"/>
        <xs:element name="mode" minOccurs="0">
          <xs:simpleType><xs:restriction base="xs:string"><xs:enumeration value="a"/><xs:enumeration value="b"/></xs:restriction></xs:simpleType>
        </xs:element>
      </xs:sequence>
      <xs:attribute name="version" type="xs:int" use="required"/>
    </xs:complexType>
  </xs:element>
</xs:schema>"#,
        )
        .unwrap();
        let loc = [Location { namespace: None, url: xsd.to_string_lossy().into_owned() }];
        assert!(messages("<config version=\"1\"><port>80</port></config>", &loc).is_empty());
        let m = messages("<config version=\"x\">\n  <port>eighty</port>\n  <mode>c</mode>\n</config>", &loc);
        assert_eq!(m.len(), 3, "{m:?}");
        assert_eq!(m[0].0, "version");
        assert_eq!(m[1].0, "port");
        assert_eq!(m[2].0, "mode");
        // A schema served from a URL comes from the local copy.
        let mut files = HashMap::new();
        files.insert("https://example.test/s.xsd".to_string(), xsd.clone());
        set_local_files(files);
        let remote = [Location { namespace: None, url: "https://example.test/s.xsd".into() }];
        assert_eq!(messages("<config/>", &remote).len(), 2);
        let missing = [Location { namespace: None, url: "https://example.test/missing.xsd".into() }];
        assert!(messages("<config/>", &missing)[0].1.starts_with("Schema problem"));
        std::fs::remove_dir_all(dir).ok();
    }
}
