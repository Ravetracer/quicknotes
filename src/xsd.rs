//! XML completion: the cursor context in an XML document and a simplified
//! model of XML Schemas (which elements may appear inside which, their
//! attributes and enumerated values). Names are matched by their local part.

use crate::datadoc::Proposal;
use crate::xml_lint::Location;
use quick_xml::events::Event;
use std::collections::{HashMap, HashSet};
use std::ops::Range;

// ----- schema references in documents --------------------------------------

const XSI: &str = "http://www.w3.org/2001/XMLSchema-instance";

/// The schemas named by the root element (`xsi:schemaLocation`,
/// `xsi:noNamespaceSchemaLocation`), resolved against `base`.
pub fn schema_locations(text: &str, base: Option<&str>) -> Vec<Location> {
    let mut reader = quick_xml::Reader::from_str(text);
    let mut out = Vec::new();
    loop {
        match reader.read_event() {
            Ok(Event::Start(e) | Event::Empty(e)) => {
                let attrs: Vec<(String, String)> = e
                    .attributes()
                    .flatten()
                    .map(|a| (String::from_utf8_lossy(a.key.as_ref()).into_owned(), a.unescape_value().map(|v| v.into_owned()).unwrap_or_default()))
                    .collect();
                let xsi_prefix = attrs.iter().find(|(_, v)| v == XSI).and_then(|(k, _)| k.strip_prefix("xmlns:")).unwrap_or("xsi").to_owned();
                let resolve = |loc: &str| match base {
                    Some(b) => crate::json_schema::join(b, loc),
                    None => loc.to_owned(),
                };
                for (k, v) in &attrs {
                    if *k == format!("{xsi_prefix}:schemaLocation") {
                        let parts: Vec<&str> = v.split_whitespace().collect();
                        for pair in parts.chunks(2) {
                            if let [ns, loc] = pair {
                                out.push(Location { namespace: Some((*ns).to_owned()), url: resolve(loc) });
                            }
                        }
                    } else if *k == format!("{xsi_prefix}:noNamespaceSchemaLocation") {
                        if let Some(loc) = v.split_whitespace().next() {
                            out.push(Location { namespace: None, url: resolve(loc) });
                        }
                    }
                }
                return out;
            }
            Ok(Event::Eof) | Err(_) => return out,
            _ => {}
        }
    }
}

/// The schema files a schema includes or imports (resolved against its URL).
pub fn dependencies(xsd: &str, url: &str) -> Vec<String> {
    let mut reader = quick_xml::Reader::from_str(xsd);
    let mut out = Vec::new();
    loop {
        match reader.read_event() {
            Ok(Event::Start(e) | Event::Empty(e)) => {
                if matches!(local(e.name().as_ref()).as_str(), "include" | "import" | "redefine" | "override") {
                    if let Some(loc) = attr(&e, "schemaLocation") {
                        let full = crate::json_schema::join(url, &loc);
                        if !out.contains(&full) {
                            out.push(full);
                        }
                    }
                }
            }
            Ok(Event::Eof) | Err(_) => return out,
            _ => {}
        }
    }
}

fn local(name: &[u8]) -> String {
    let s = String::from_utf8_lossy(name);
    s.rsplit(':').next().unwrap_or(&s).to_owned()
}

fn attr(e: &quick_xml::events::BytesStart, name: &str) -> Option<String> {
    e.attributes().flatten().find(|a| a.key.as_ref() == name.as_bytes()).and_then(|a| a.unescape_value().ok().map(|v| v.into_owned()))
}

// ----- a small DOM of schema files ----------------------------------------

#[derive(Debug, Default, Clone)]
struct X {
    name: String,
    attrs: Vec<(String, String)>,
    children: Vec<X>,
    text: String,
}

impl X {
    fn get(&self, key: &str) -> Option<&str> {
        self.attrs.iter().find(|(k, _)| k == key).map(|(_, v)| v.as_str())
    }

    fn kids<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a X> + 'a {
        self.children.iter().filter(move |c| c.name == name)
    }

    fn doc(&self) -> String {
        let text = self.kids("annotation").flat_map(|a| a.kids("documentation")).map(|d| d.text.trim()).find(|t| !t.is_empty()).unwrap_or("");
        let first = text.lines().map(str::trim).find(|l| !l.is_empty()).unwrap_or("");
        if first.chars().count() > 90 {
            format!("{}…", first.chars().take(89).collect::<String>())
        } else {
            first.to_owned()
        }
    }
}

fn parse_dom(text: &str) -> Option<X> {
    let mut reader = quick_xml::Reader::from_str(text);
    let mut stack: Vec<X> = vec![X::default()];
    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) => {
                stack.push(element(&e));
            }
            Ok(Event::Empty(e)) => {
                let x = element(&e);
                stack.last_mut()?.children.push(x);
            }
            Ok(Event::End(_)) => {
                let done = stack.pop()?;
                stack.last_mut()?.children.push(done);
            }
            Ok(Event::Text(t)) => {
                if let Some(top) = stack.last_mut() {
                    top.text.push_str(&t.unescape().map(|s| s.into_owned()).unwrap_or_default());
                }
            }
            Ok(Event::CData(t)) => {
                if let Some(top) = stack.last_mut() {
                    top.text.push_str(&String::from_utf8_lossy(&t));
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => return None,
            _ => {}
        }
    }
    stack.pop()?.children.into_iter().find(|c| c.name == "schema")
}

fn element(e: &quick_xml::events::BytesStart) -> X {
    X {
        name: local(e.name().as_ref()),
        attrs: e
            .attributes()
            .flatten()
            .map(|a| (String::from_utf8_lossy(a.key.as_ref()).into_owned(), a.unescape_value().map(|v| v.into_owned()).unwrap_or_default()))
            .collect(),
        children: Vec::new(),
        text: String::new(),
    }
}

fn strip_prefix(qname: &str) -> String {
    qname.rsplit(':').next().unwrap_or(qname).to_owned()
}

// ----- the model ------------------------------------------------------------

/// Element and type definitions of a set of schema files.
#[derive(Default)]
pub struct Model {
    elements: HashMap<String, X>,
    types: HashMap<String, X>,
    groups: HashMap<String, X>,
    attr_groups: HashMap<String, X>,
    attributes: HashMap<String, X>,
    /// Elements by the head of their substitution group.
    substitutes: HashMap<String, Vec<String>>,
}

/// An element that may appear somewhere, with its documentation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Child {
    pub name: String,
    pub doc: String,
}

/// An attribute of an element.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Attr {
    pub name: String,
    pub doc: String,
    pub required: bool,
    pub values: Vec<String>,
}

impl Model {
    /// Builds the model from schema files (their text).
    pub fn new(files: &[String]) -> Self {
        let mut m = Model::default();
        for text in files {
            let Some(schema) = parse_dom(text) else { continue };
            for c in &schema.children {
                let Some(name) = c.get("name").map(str::to_owned) else {
                    if c.name == "redefine" || c.name == "override" {
                        for r in &c.children {
                            if let Some(name) = r.get("name") {
                                m.add(r, name.to_owned());
                            }
                        }
                    }
                    continue;
                };
                m.add(c, name);
            }
        }
        m
    }

    fn add(&mut self, c: &X, name: String) {
        match c.name.as_str() {
            "element" => {
                if let Some(head) = c.get("substitutionGroup") {
                    self.substitutes.entry(strip_prefix(head)).or_default().push(name.clone());
                }
                self.elements.insert(name, c.clone());
            }
            "complexType" | "simpleType" => {
                self.types.insert(name, c.clone());
            }
            "group" => {
                self.groups.insert(name, c.clone());
            }
            "attributeGroup" => {
                self.attr_groups.insert(name, c.clone());
            }
            "attribute" => {
                self.attributes.insert(name, c.clone());
            }
            _ => {}
        }
    }

    pub fn is_empty(&self) -> bool {
        self.elements.is_empty()
    }

    /// The root elements (global, not abstract).
    pub fn roots(&self) -> Vec<Child> {
        let mut out: Vec<Child> = self.elements.iter().filter(|(_, e)| e.get("abstract") != Some("true")).map(|(n, e)| Child { name: n.clone(), doc: e.doc() }).collect();
        out.sort_by(|a, b| a.name.cmp(&b.name));
        out
    }

    /// The declaration of the element at the end of `path` (local names from the root).
    fn decl(&self, path: &[String]) -> Option<X> {
        let (first, rest) = path.split_first()?;
        let mut current = self.elements.get(first)?.clone();
        for name in rest {
            let next = self.child_decls(&current).into_iter().find(|d| d.get("name").map(str::to_owned) == Some(name.clone()) || d.get("ref").map(strip_prefix) == Some(name.clone()));
            current = match next {
                Some(d) => self.deref(&d),
                None => self.elements.get(name)?.clone(),
            };
        }
        Some(current)
    }

    /// An element reference resolved to its global declaration.
    fn deref(&self, d: &X) -> X {
        match d.get("ref") {
            Some(r) => self.elements.get(&strip_prefix(r)).cloned().unwrap_or_else(|| d.clone()),
            None => d.clone(),
        }
    }

    /// The type definition of an element declaration.
    fn type_of(&self, decl: &X) -> Option<X> {
        if let Some(t) = decl.children.iter().find(|c| c.name == "complexType" || c.name == "simpleType") {
            return Some(t.clone());
        }
        let t = decl.get("type")?;
        self.types.get(&strip_prefix(t)).cloned().or_else(|| Some(X { name: "builtin".into(), attrs: vec![("name".into(), strip_prefix(t))], ..Default::default() }))
    }

    /// Element declarations allowed inside an element.
    fn child_decls(&self, decl: &X) -> Vec<X> {
        let mut out = Vec::new();
        if let Some(t) = self.type_of(decl) {
            self.particles(&t, &mut out, 0);
        }
        out
    }

    fn particles(&self, x: &X, out: &mut Vec<X>, depth: usize) {
        if depth > 24 {
            return;
        }
        for c in &x.children {
            match c.name.as_str() {
                "element" => {
                    out.push(c.clone());
                    let name = c.get("ref").map(strip_prefix).or_else(|| c.get("name").map(str::to_owned));
                    for sub in name.and_then(|n| self.substitutes.get(&n)).into_iter().flatten() {
                        if let Some(d) = self.elements.get(sub) {
                            out.push(d.clone());
                        }
                    }
                }
                "sequence" | "choice" | "all" | "complexContent" => self.particles(c, out, depth + 1),
                "group" => match c.get("ref").and_then(|r| self.groups.get(&strip_prefix(r))) {
                    Some(g) => self.particles(g, out, depth + 1),
                    None => self.particles(c, out, depth + 1),
                },
                "extension" | "restriction" => {
                    if c.name == "extension" {
                        if let Some(base) = c.get("base").and_then(|b| self.types.get(&strip_prefix(b))) {
                            self.particles(base, out, depth + 1);
                        }
                    }
                    self.particles(c, out, depth + 1);
                }
                _ => {}
            }
        }
    }

    fn attr_decls(&self, x: &X, out: &mut Vec<X>, depth: usize) {
        if depth > 24 {
            return;
        }
        for c in &x.children {
            match c.name.as_str() {
                "attribute" => out.push(match c.get("ref") {
                    Some(r) => self.attributes.get(&strip_prefix(r)).cloned().map_or_else(|| c.clone(), |mut a| {
                        if let Some(u) = c.get("use") {
                            a.attrs.push(("use".into(), u.into()));
                        }
                        a
                    }),
                    None => c.clone(),
                }),
                "attributeGroup" => match c.get("ref").and_then(|r| self.attr_groups.get(&strip_prefix(r))) {
                    Some(g) => self.attr_decls(g, out, depth + 1),
                    None => self.attr_decls(c, out, depth + 1),
                },
                "complexContent" | "simpleContent" => self.attr_decls(c, out, depth + 1),
                "extension" | "restriction" => {
                    if let Some(base) = c.get("base").and_then(|b| self.types.get(&strip_prefix(b))) {
                        self.attr_decls(base, out, depth + 1);
                    }
                    self.attr_decls(c, out, depth + 1);
                }
                _ => {}
            }
        }
    }

    /// Enumerated values of a simple type (or `true`/`false` for booleans).
    fn values(&self, t: &X, depth: usize) -> Vec<String> {
        if depth > 16 {
            return Vec::new();
        }
        if t.name == "builtin" {
            return match t.get("name") {
                Some("boolean") => vec!["true".into(), "false".into()],
                _ => Vec::new(),
            };
        }
        let mut out = Vec::new();
        for c in &t.children {
            match c.name.as_str() {
                "restriction" | "simpleContent" | "extension" => {
                    out.extend(c.kids("enumeration").filter_map(|e| e.get("value").map(str::to_owned)));
                    if out.is_empty() {
                        if let Some(base) = c.get("base") {
                            let base_t = self.types.get(&strip_prefix(base)).cloned().unwrap_or_else(|| X { name: "builtin".into(), attrs: vec![("name".into(), strip_prefix(base))], ..Default::default() });
                            out.extend(self.values(&base_t, depth + 1));
                        }
                    }
                    if out.is_empty() {
                        out.extend(self.values(c, depth + 1));
                    }
                }
                "union" => {
                    for m in c.get("memberTypes").unwrap_or("").split_whitespace() {
                        if let Some(mt) = self.types.get(&strip_prefix(m)) {
                            out.extend(self.values(mt, depth + 1));
                        }
                    }
                    for st in c.kids("simpleType") {
                        out.extend(self.values(st, depth + 1));
                    }
                }
                "simpleType" => out.extend(self.values(c, depth + 1)),
                _ => {}
            }
        }
        out
    }

    /// Elements allowed inside the element at `path` (the root elements for an empty path).
    pub fn children(&self, path: &[String]) -> Vec<Child> {
        if path.is_empty() {
            return self.roots();
        }
        let Some(decl) = self.decl(path) else { return Vec::new() };
        let mut seen = HashSet::new();
        self.child_decls(&decl)
            .into_iter()
            .map(|d| self.deref(&d))
            .filter(|d| d.get("abstract") != Some("true"))
            .filter_map(|d| {
                let name = d.get("name")?.to_owned();
                seen.insert(name.clone()).then(|| Child { name, doc: d.doc() })
            })
            .collect()
    }

    pub fn attributes(&self, path: &[String]) -> Vec<Attr> {
        let Some(decl) = self.decl(path) else { return Vec::new() };
        let Some(t) = self.type_of(&decl) else { return Vec::new() };
        let mut decls = Vec::new();
        self.attr_decls(&t, &mut decls, 0);
        let mut seen = HashSet::new();
        decls
            .into_iter()
            .filter(|a| a.get("use") != Some("prohibited"))
            .filter_map(|a| {
                let name = a.get("name")?.to_owned();
                if !seen.insert(name.clone()) {
                    return None;
                }
                let values = match a.children.iter().find(|c| c.name == "simpleType") {
                    Some(st) => self.values(st, 0),
                    None => a
                        .get("type")
                        .map(|t| self.types.get(&strip_prefix(t)).cloned().unwrap_or_else(|| X { name: "builtin".into(), attrs: vec![("name".into(), strip_prefix(t))], ..Default::default() }))
                        .map(|t| self.values(&t, 0))
                        .unwrap_or_default(),
                };
                let mut values = values;
                if let Some(f) = a.get("fixed").or_else(|| a.get("default")) {
                    if !values.iter().any(|v| v == f) {
                        values.insert(0, f.to_owned());
                    }
                }
                Some(Attr { name, doc: a.doc(), required: a.get("use") == Some("required"), values })
            })
            .collect()
    }

    /// Enumerated values of the text of the element at `path`.
    pub fn text_values(&self, path: &[String]) -> Vec<String> {
        let Some(decl) = self.decl(path) else { return Vec::new() };
        match self.type_of(&decl) {
            Some(t) if t.name != "complexType" || t.kids("simpleContent").next().is_some() => self.values(&t, 0),
            _ => Vec::new(),
        }
    }
}

// ----- cursor context ---------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Spot {
    /// After `<`: an element name.
    Element,
    /// After `</`: the name of the element to close.
    Close(String),
    /// An attribute name inside a start tag; the attributes already there.
    AttrName { element: String, existing: Vec<String> },
    /// Inside an attribute value.
    AttrValue { element: String, attr: String },
    /// Text content of an element.
    Text,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Place {
    /// Open elements (local names), outermost first; for attributes and
    /// text the element itself is the last one.
    pub path: Vec<String>,
    /// The open element names as written (with prefixes), for closing tags.
    pub open: Vec<String>,
    pub spot: Spot,
    pub range: Range<usize>,
    pub prefix: String,
}

fn name_end(text: &str, from: usize) -> usize {
    text[from..].find(|c: char| !(c.is_alphanumeric() || matches!(c, '_' | '-' | '.' | ':'))).map_or(text.len(), |i| from + i)
}

fn local_name(name: &str) -> String {
    name.rsplit(':').next().unwrap_or(name).to_owned()
}

/// The completion context at `cursor` (a byte offset).
pub fn place(text: &str, cursor: usize) -> Option<Place> {
    let mut open: Vec<String> = Vec::new();
    let mut i = 0;
    let b = text.as_bytes();
    while i < cursor {
        let Some(lt) = text[i..cursor].find('<').map(|p| i + p) else {
            // Text up to the cursor.
            let start = text[..cursor].rfind(|c: char| c == '>' || c.is_whitespace()).map_or(i, |p| p + 1).max(i);
            let path = open.iter().map(|n| local_name(n)).collect();
            return Some(Place { path, open, spot: Spot::Text, range: start..cursor, prefix: text[start..cursor].to_owned() });
        };
        let rest = &text[lt..];
        let skip_to = |end: &str, from: usize| text[from..].find(end).map(|p| from + p + end.len());
        if rest.starts_with("<!--") {
            i = skip_to("-->", lt + 4)?;
            if i > cursor {
                return None;
            }
            continue;
        }
        if rest.starts_with("<![CDATA[") {
            i = skip_to("]]>", lt + 9)?;
            if i > cursor {
                return None;
            }
            continue;
        }
        if rest.starts_with("<?") || rest.starts_with("<!") {
            i = skip_to(">", lt + 2)?;
            if i > cursor {
                return None;
            }
            continue;
        }
        if rest.starts_with("</") {
            let name_start = lt + 2;
            let end = name_end(text, name_start);
            if cursor <= end {
                let current = open.last().cloned().unwrap_or_default();
                let path = open.iter().map(|n| local_name(n)).collect();
                return Some(Place { path, open: open.clone(), spot: Spot::Close(current), range: name_start..end, prefix: text[name_start..cursor].to_owned() });
            }
            let name = &text[name_start..end];
            if let Some(pos) = open.iter().rposition(|n| n == name) {
                open.truncate(pos);
            }
            i = text[end..].find('>').map_or(text.len(), |p| end + p + 1);
            continue;
        }
        // A start tag.
        let name_start = lt + 1;
        let end = name_end(text, name_start);
        if cursor <= end {
            let path = open.iter().map(|n| local_name(n)).collect();
            return Some(Place { path, open: open.clone(), spot: Spot::Element, range: name_start..end, prefix: text[name_start..cursor].to_owned() });
        }
        let name = text[name_start..end].to_owned();
        // Attributes up to `>`.
        let mut j = end;
        let mut existing: Vec<String> = Vec::new();
        let mut closed = None;
        while j < b.len() {
            if j >= cursor && closed.is_none() {
                break;
            }
            match b[j] {
                b'>' => {
                    closed = Some(b[j - 1] == b'/');
                    j += 1;
                    break;
                }
                b'"' | b'\'' => {
                    let q = b[j];
                    let close = text[j + 1..].find(q as char).map(|p| j + 1 + p);
                    let value_end = close.unwrap_or(text.len());
                    if cursor > j && cursor <= value_end {
                        let mut path: Vec<String> = open.iter().map(|n| local_name(n)).collect();
                        path.push(local_name(&name));
                        let attr = existing.last().cloned().unwrap_or_default();
                        return Some(Place {
                            path,
                            open: open.clone(),
                            spot: Spot::AttrValue { element: local_name(&name), attr: local_name(&attr) },
                            range: j + 1..value_end,
                            prefix: text[j + 1..cursor].to_owned(),
                        });
                    }
                    j = close.map_or(text.len(), |c| c + 1);
                }
                c if c.is_ascii_alphabetic() || c == b'_' => {
                    let a_end = name_end(text, j);
                    if cursor <= a_end && cursor >= j {
                        let mut path: Vec<String> = open.iter().map(|n| local_name(n)).collect();
                        path.push(local_name(&name));
                        let mut others = existing.clone();
                        // Attributes after the cursor count as well.
                        let tag_end = text[a_end..].find('>').map_or(text.len(), |p| a_end + p);
                        others.extend(attr_names(&text[a_end..tag_end]));
                        return Some(Place {
                            path,
                            open: open.clone(),
                            spot: Spot::AttrName { element: local_name(&name), existing: others },
                            range: j..a_end,
                            prefix: text[j..cursor].to_owned(),
                        });
                    }
                    existing.push(text[j..a_end].to_owned());
                    j = a_end;
                }
                _ => j += 1,
            }
        }
        match closed {
            None => {
                // Cursor inside the tag, between attributes.
                let mut path: Vec<String> = open.iter().map(|n| local_name(n)).collect();
                path.push(local_name(&name));
                let in_space = cursor > 0 && text[..cursor].ends_with(char::is_whitespace);
                if !in_space {
                    return None;
                }
                let tag_end = text[cursor..].find('>').map_or(text.len(), |p| cursor + p);
                existing.extend(attr_names(&text[cursor..tag_end]));
                return Some(Place { path, open, spot: Spot::AttrName { element: local_name(&name), existing }, range: cursor..cursor, prefix: String::new() });
            }
            Some(self_closing) => {
                if !self_closing {
                    open.push(name);
                }
                i = j;
            }
        }
    }
    let path = open.iter().map(|n| local_name(n)).collect();
    Some(Place { path, open, spot: Spot::Text, range: cursor..cursor, prefix: String::new() })
}

fn attr_names(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = s;
    while let Some(eq) = rest.find('=') {
        let name = rest[..eq].trim_end().rsplit(|c: char| c.is_whitespace() || c == '"' || c == '\'').next().unwrap_or("");
        if !name.is_empty() {
            out.push(name.to_owned());
        }
        rest = &rest[eq + 1..];
        let trimmed = rest.trim_start();
        if let Some(q) = trimmed.chars().next().filter(|c| *c == '"' || *c == '\'') {
            let after = &trimmed[1..];
            rest = after.find(q).map_or("", |p| &after[p + 1..]);
        }
    }
    out
}

/// Element and attribute names used in the document (for completion without a schema).
fn used_names(text: &str) -> (Vec<String>, HashMap<String, Vec<String>>) {
    let mut reader = quick_xml::Reader::from_str(text);
    reader.config_mut().check_end_names = false;
    let mut elements: Vec<String> = Vec::new();
    let mut attrs: HashMap<String, Vec<String>> = HashMap::new();
    loop {
        match reader.read_event() {
            Ok(Event::Start(e) | Event::Empty(e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).into_owned();
                if !elements.contains(&name) {
                    elements.push(name.clone());
                }
                let list = attrs.entry(local_name(&name)).or_default();
                for a in e.attributes().flatten() {
                    let n = String::from_utf8_lossy(a.key.as_ref()).into_owned();
                    if !list.contains(&n) {
                        list.push(n);
                    }
                }
            }
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
    }
    (elements, attrs)
}

fn matches(label: &str, prefix: &str) -> bool {
    prefix.is_empty() || label.to_lowercase().contains(&prefix.to_lowercase())
}

/// Proposals at a place, from the schema model (if any) and the document.
pub fn proposals(text: &str, place: &Place, model: Option<&Model>) -> Vec<Proposal> {
    let model = model.filter(|m| !m.is_empty());
    let mut out: Vec<Proposal> = Vec::new();
    match &place.spot {
        Spot::Close(name) => {
            if !name.is_empty() {
                let has_gt = text[place.range.end..].starts_with('>');
                out.push(Proposal { label: name.clone(), insert: if has_gt { name.clone() } else { format!("{name}>") }, info: t!("Close element").into(), back: 0 });
            }
        }
        Spot::Element => {
            let prefix_ns = place.prefix.rsplit_once(':').map(|(p, _)| format!("{p}:")).unwrap_or_default();
            let followed = text[place.range.end..].trim_start().starts_with(['>', '/']) || text[place.range.end..].starts_with(char::is_whitespace);
            let item = |name: &str, doc: String| {
                let full = format!("{prefix_ns}{name}");
                if followed {
                    Proposal { label: full.clone(), insert: full, info: doc, back: 0 }
                } else {
                    let close = format!("</{full}>");
                    Proposal { label: full.clone(), insert: format!("{full}>{close}"), info: doc, back: close.chars().count() }
                }
            };
            if let Some(current) = place.open.last() {
                if !followed && matches(&format!("/{current}"), &place.prefix) {
                    out.push(Proposal { label: format!("/{current}"), insert: format!("/{current}>"), info: t!("Close element").into(), back: 0 });
                }
            }
            match model {
                Some(m) => {
                    for c in m.children(&place.path) {
                        if matches(&c.name, &place.prefix) {
                            out.push(item(&c.name, c.doc));
                        }
                    }
                }
                None => {
                    for name in used_names(text).0 {
                        if matches(&name, &place.prefix) && name != place.prefix {
                            out.push(Proposal { label: name.clone(), insert: name, info: String::new(), back: 0 });
                        }
                    }
                }
            }
        }
        Spot::AttrName { element, existing } => {
            let followed = text[place.range.end..].starts_with('=');
            match model {
                Some(m) => {
                    let mut attrs = m.attributes(&place.path);
                    attrs.sort_by_key(|a| !a.required);
                    for a in attrs {
                        if existing.contains(&a.name) || !matches(&a.name, &place.prefix) {
                            continue;
                        }
                        let label = if a.required { format!("{} *", a.name) } else { a.name.clone() };
                        let (insert, back) = if followed { (a.name.clone(), 0) } else { (format!("{}=\"\"", a.name), 1) };
                        out.push(Proposal { label, insert, info: a.doc, back });
                    }
                }
                None => {
                    for a in used_names(text).1.remove(element).unwrap_or_default() {
                        if existing.contains(&a) || !matches(&a, &place.prefix) || a == place.prefix {
                            continue;
                        }
                        let (insert, back) = if followed { (a.clone(), 0) } else { (format!("{a}=\"\""), 1) };
                        out.push(Proposal { label: a, insert, info: String::new(), back });
                    }
                }
            }
        }
        Spot::AttrValue { attr, .. } => {
            if let Some(m) = model {
                if let Some(a) = m.attributes(&place.path).into_iter().find(|a| &a.name == attr) {
                    for v in a.values {
                        if matches(&v, &place.prefix) {
                            out.push(Proposal { label: v.clone(), insert: v, info: a.doc.clone(), back: 0 });
                        }
                    }
                }
            }
        }
        Spot::Text => {
            if let Some(m) = model {
                for v in m.text_values(&place.path) {
                    if matches(&v, &place.prefix) {
                        out.push(Proposal { label: v.clone(), insert: v, info: String::new(), back: 0 });
                    }
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const XSD: &str = r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">
  <xs:element name="config" type="Config"/>
  <xs:complexType name="Config">
    <xs:sequence>
      <xs:element name="server" type="Server" maxOccurs="unbounded"/>
      <xs:group ref="extras"/>
    </xs:sequence>
    <xs:attributeGroup ref="common"/>
  </xs:complexType>
  <xs:complexType name="Server">
    <xs:complexContent>
      <xs:extension base="Base">
        <xs:sequence>
          <xs:element name="port" type="xs:int"><xs:annotation><xs:documentation>TCP port</xs:documentation></xs:annotation></xs:element>
          <xs:element name="mode" type="Mode"/>
        </xs:sequence>
        <xs:attribute name="secure" type="xs:boolean"/>
      </xs:extension>
    </xs:complexContent>
  </xs:complexType>
  <xs:complexType name="Base">
    <xs:sequence><xs:element name="name" type="xs:string"/></xs:sequence>
    <xs:attribute name="id" type="xs:ID" use="required"/>
  </xs:complexType>
  <xs:simpleType name="Mode"><xs:restriction base="xs:string"><xs:enumeration value="fast"/><xs:enumeration value="safe"/></xs:restriction></xs:simpleType>
  <xs:group name="extras"><xs:choice><xs:element name="log" type="xs:string"/></xs:choice></xs:group>
  <xs:attributeGroup name="common"><xs:attribute name="version" type="xs:string"/></xs:attributeGroup>
</xs:schema>"#;

    fn at(s: &str) -> (String, Place) {
        let cursor = s.find('|').unwrap();
        let text = s.replace('|', "");
        let p = place(&text, cursor).unwrap();
        (text, p)
    }

    fn labels(s: &str, model: Option<&Model>) -> Vec<String> {
        let (text, p) = at(s);
        proposals(&text, &p, model).into_iter().map(|p| p.label).collect()
    }

    #[test]
    fn finds_schema_locations() {
        let doc = r#"<?xml version="1.0"?><c:config xmlns:c="urn:c" xmlns:x="http://www.w3.org/2001/XMLSchema-instance" x:schemaLocation="urn:c c.xsd urn:d https://e.test/d.xsd"/>"#;
        assert_eq!(
            schema_locations(doc, Some("file:///home/u/doc.xml")),
            [
                Location { namespace: Some("urn:c".into()), url: "file:///home/u/c.xsd".into() },
                Location { namespace: Some("urn:d".into()), url: "https://e.test/d.xsd".into() },
            ]
        );
        assert_eq!(dependencies(r#"<xs:schema xmlns:xs="x"><xs:include schemaLocation="b.xsd"/><xs:import namespace="n" schemaLocation="../c.xsd"/></xs:schema>"#, "https://a.test/s/a.xsd"), ["https://a.test/s/b.xsd", "https://a.test/c.xsd"]);
    }

    #[test]
    fn places() {
        let (_, p) = at("<a><b x=\"1\" |y=\"2\"></b></a>");
        assert_eq!(p.path, ["a", "b"]);
        assert_eq!(p.spot, Spot::AttrName { element: "b".into(), existing: vec!["x".into(), "y".into()] });
        let (_, p) = at("<a><!-- <x> --><b></b><c v=\"q|\"");
        assert_eq!(p.spot, Spot::AttrValue { element: "c".into(), attr: "v".into() });
        assert_eq!(p.prefix, "q");
        let (_, p) = at("<a><b/><se|");
        assert_eq!((p.path.clone(), p.spot.clone(), p.prefix.as_str()), (vec!["a".to_string()], Spot::Element, "se"));
        let (_, p) = at("<a><b></|");
        assert_eq!(p.spot, Spot::Close("b".into()));
        let (_, p) = at("<a><mode>fa|</mode></a>");
        assert_eq!((p.spot.clone(), p.prefix.as_str(), p.path.clone()), (Spot::Text, "fa", vec!["a".to_string(), "mode".to_string()]));
    }

    #[test]
    fn proposals_from_schema() {
        let m = Model::new(&[XSD.to_string()]);
        assert_eq!(labels("<|", Some(&m)), ["config"]);
        assert_eq!(labels("<config><|", Some(&m)), ["/config", "server", "log"]);
        assert_eq!(labels("<config><server><|", Some(&m)), ["/server", "name", "port", "mode"]);
        assert_eq!(labels("<config><server |", Some(&m)), ["id *", "secure"]);
        assert_eq!(labels("<config><server secure=\"|\"", Some(&m)), ["true", "false"]);
        assert_eq!(labels("<config><server><mode>|", Some(&m)), ["fast", "safe"]);
        assert_eq!(labels("<config |", Some(&m)), ["version"]);
        let (text, p) = at("<config><serv|");
        let first = proposals(&text, &p, Some(&m)).into_iter().find(|p| p.label == "server").unwrap();
        assert_eq!((first.insert.as_str(), first.back), ("server></server>", 9));
    }

    #[test]
    fn proposals_without_schema() {
        assert_eq!(labels("<r><item key=\"1\"/><item |", None), ["key"]);
        assert_eq!(labels("<r><item key=\"1\"/><other/><item k|", None), ["key"]);
        assert_eq!(labels("<r><item/><it|", None), ["item"]);
    }
}
