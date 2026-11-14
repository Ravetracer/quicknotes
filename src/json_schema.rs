//! JSON Schema validation (drafts 4 to 2020-12) of JSON and YAML documents
//! and schema-based completion.
//!
//! Schema documents come from a [`Source`]; they live as long as the program
//! (see `schemas.rs`), which keeps references simple. Unknown keywords and
//! `format` are ignored; patterns the `regex` crate cannot compile (look-
//! arounds, back references) are skipped.

use crate::datadoc::{Diagnostic, Member, Node, Proposal, Seg, Severity, Value};
use serde_json::Value as Json;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::ops::Range;
use std::rc::Rc;

/// Where schema documents come from.
pub trait Source {
    /// The document at `url` (without fragment); `None` while it is loading
    /// or when it cannot be loaded.
    fn load(&self, url: &str) -> Option<&'static Json>;
}

/// A schema and the base URL its references resolve against.
#[derive(Clone)]
pub struct Sch {
    pub node: &'static Json,
    pub base: Rc<str>,
}

impl Sch {
    /// The root of a schema document loaded from `base`.
    pub fn new(node: &'static Json, base: &str) -> Self {
        index_of(node, base);
        let mut s = Self { node, base: base.into() };
        s.rebase();
        s
    }

    fn child(&self, node: &'static Json) -> Self {
        let mut s = Self { node, base: self.base.clone() };
        s.rebase();
        s
    }

    /// `$id` (or draft-4 `id`) changes the base URL.
    fn rebase(&mut self) {
        let id = self.node.get("$id").or_else(|| self.node.get("id")).and_then(Json::as_str);
        if let Some(id) = id.filter(|id| !id.starts_with('#')) {
            self.base = join(&self.base, id).into();
        }
    }

    fn get(&self, key: &str) -> Option<&'static Json> {
        self.node.get(key)
    }
}

// ----- URLs ---------------------------------------------------------------

fn has_scheme(s: &str) -> bool {
    let Some(colon) = s.find(':') else { return false };
    colon > 1 && s[..colon].chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'))
}

/// `url` without its fragment.
pub fn strip_fragment(url: &str) -> &str {
    url.split('#').next().unwrap_or(url)
}

/// Resolves a reference against a base URL (RFC 3986, enough for schemas).
pub fn join(base: &str, rel: &str) -> String {
    if rel.is_empty() {
        return strip_fragment(base).to_owned();
    }
    if has_scheme(rel) {
        return rel.to_owned();
    }
    let base = strip_fragment(base);
    if rel.starts_with('#') {
        return format!("{base}{rel}");
    }
    let base = base.split('?').next().unwrap_or(base);
    let (scheme_auth, path) = match base.find("://") {
        Some(i) => {
            let after = i + 3;
            let path_start = base[after..].find('/').map_or(base.len(), |p| after + p);
            (&base[..path_start], &base[path_start..])
        }
        None => ("", base),
    };
    if let Some(rest) = rel.strip_prefix("//") {
        let scheme = scheme_auth.split("://").next().unwrap_or("https");
        return format!("{scheme}://{rest}");
    }
    let joined = if rel.starts_with('/') {
        rel.to_owned()
    } else {
        let dir = path.rfind('/').map_or("/", |i| &path[..=i]);
        format!("{dir}{rel}")
    };
    let (path_part, suffix) = match joined.find(['?', '#']) {
        Some(i) => (&joined[..i], &joined[i..]),
        None => (joined.as_str(), ""),
    };
    let mut segs: Vec<&str> = Vec::new();
    for seg in path_part.split('/') {
        match seg {
            "." => {}
            ".." => {
                if segs.len() > 1 {
                    segs.pop();
                }
            }
            s => segs.push(s),
        }
    }
    let mut out = segs.join("/");
    if path_part.ends_with("/.") || path_part.ends_with("/..") {
        out.push('/');
    }
    format!("{scheme_auth}{out}{suffix}")
}

// ----- resolution ---------------------------------------------------------

thread_local! {
    /// Embedded `$id`s and anchors per schema document (by address).
    static INDEX: RefCell<HashMap<usize, Rc<HashMap<String, &'static Json>>>> = RefCell::new(HashMap::new());
    /// All `$id`s and anchors of the documents indexed so far, so that a
    /// relative `$id` (which changes the base URL to one that cannot be
    /// downloaded) still finds its document.
    static IDS: RefCell<HashMap<String, &'static Json>> = RefCell::new(HashMap::new());
    static REGEX: RefCell<HashMap<String, Option<regex::Regex>>> = RefCell::new(HashMap::new());
}

/// All `$id`s and `$anchor`s inside a document, as absolute URLs.
fn index_of(doc: &'static Json, url: &str) -> Rc<HashMap<String, &'static Json>> {
    let key = doc as *const Json as usize;
    if let Some(found) = INDEX.with(|i| i.borrow().get(&key).cloned()) {
        return found;
    }
    let mut map = HashMap::new();
    fn walk(node: &'static Json, base: &str, map: &mut HashMap<String, &'static Json>, depth: usize) {
        if depth > 64 {
            return;
        }
        match node {
            Json::Object(o) => {
                let mut base = base.to_owned();
                if let Some(id) = o.get("$id").or_else(|| o.get("id")).and_then(Json::as_str) {
                    if let Some(anchor) = id.strip_prefix('#') {
                        map.insert(format!("{}#{anchor}", strip_fragment(&base)), node);
                    } else {
                        base = join(&base, id);
                        map.insert(strip_fragment(&base).to_owned(), node);
                    }
                }
                if let Some(anchor) = o.get("$anchor").or_else(|| o.get("$dynamicAnchor")).and_then(Json::as_str) {
                    map.insert(format!("{}#{anchor}", strip_fragment(&base)), node);
                }
                for (k, v) in o {
                    // `enum`, `const` and examples hold data, not schemas.
                    if !matches!(k.as_str(), "enum" | "const" | "examples" | "default") {
                        walk(v, &base, map, depth + 1);
                    }
                }
            }
            Json::Array(a) => a.iter().for_each(|v| walk(v, base, map, depth + 1)),
            _ => {}
        }
    }
    walk(doc, url, &mut map, 0);
    IDS.with(|ids| {
        let mut ids = ids.borrow_mut();
        ids.insert(strip_fragment(url).to_owned(), doc);
        for (k, v) in &map {
            ids.insert(k.clone(), v);
        }
    });
    let map = Rc::new(map);
    INDEX.with(|i| i.borrow_mut().insert(key, map.clone()));
    map
}

fn pointer(node: &'static Json, ptr: &str) -> Option<&'static Json> {
    if ptr.is_empty() {
        return Some(node);
    }
    let mut cur = node;
    for raw in ptr.strip_prefix('/')?.split('/') {
        let token = urlencoding::decode(raw).map(|c| c.into_owned()).unwrap_or_else(|_| raw.to_owned());
        let token = token.replace("~1", "/").replace("~0", "~");
        cur = match cur {
            Json::Object(o) => o.get(&token)?,
            Json::Array(a) => a.get(token.parse::<usize>().ok()?)?,
            _ => return None,
        };
    }
    Some(cur)
}

/// Resolves `$ref` at `s`; records documents that are not there yet.
fn resolve_ref(src: &dyn Source, s: &Sch, reference: &str, missing: &mut Vec<String>) -> Option<Sch> {
    let full = join(&s.base, reference);
    let (url, fragment) = match full.split_once('#') {
        Some((u, f)) => (u.to_owned(), f.to_owned()),
        None => (full.clone(), String::new()),
    };
    // The resource: a known `$id` or a document to load.
    let doc_url = url.clone();
    let known = IDS.with(|ids| ids.borrow().get(&url).copied());
    let resource = match known {
        Some(r) => r,
        None => match src.load(&doc_url) {
            Some(doc) => {
                let idx = index_of(doc, &doc_url);
                idx.get(&url).copied().unwrap_or(doc)
            }
            None => {
                if !missing.contains(&doc_url) {
                    missing.push(doc_url);
                }
                return None;
            }
        },
    };
    let base: Rc<str> = url.as_str().into();
    if fragment.is_empty() || fragment.starts_with('/') {
        let decoded = fragment.clone();
        let node = pointer(resource, &decoded)?;
        return Some(Sch { node, base });
    }
    // A named anchor.
    let node = IDS.with(|ids| ids.borrow().get(&format!("{url}#{fragment}")).copied())?;
    Some(Sch { node, base })
}

fn regex(pattern: &str) -> Option<regex::Regex> {
    REGEX.with(|r| {
        r.borrow_mut()
            .entry(pattern.to_owned())
            .or_insert_with(|| regex::RegexBuilder::new(pattern).size_limit(1 << 22).build().ok())
            .clone()
    })
}

// ----- validation ---------------------------------------------------------

/// Result of validating a value against one schema.
#[derive(Default)]
struct Eval {
    errors: Vec<Diagnostic>,
    /// The value itself has the wrong type (used to pick the best `anyOf` branch).
    type_mismatch: Option<Vec<String>>,
    /// Properties and items checked by this schema (for `unevaluated…`).
    props: HashSet<String>,
    items: usize,
}

impl Eval {
    fn ok(&self) -> bool {
        !self.errors.iter().any(|d| d.severity == Severity::Error)
    }

    fn merge(&mut self, other: Eval) {
        self.errors.extend(other.errors);
        self.props.extend(other.props);
        self.items = self.items.max(other.items);
    }

    fn annotations(&mut self, other: &Eval) {
        self.props.extend(other.props.iter().cloned());
        self.items = self.items.max(other.items);
    }
}

pub struct Validator<'a> {
    src: &'a dyn Source,
    /// Schema documents that are still missing (being downloaded).
    pub missing: Vec<String>,
    depth: usize,
}

/// Validates the documents against a schema; returns the problems and the
/// URLs of referenced schemas that are not loaded yet.
pub fn validate(src: &dyn Source, schema: Sch, root: &Node) -> (Vec<Diagnostic>, Vec<String>) {
    let mut v = Validator { src, missing: Vec::new(), depth: 0 };
    let eval = v.check(root, None, &schema);
    let mut seen = HashSet::new();
    let mut errors: Vec<Diagnostic> = eval.errors.into_iter().filter(|d| seen.insert((d.range.clone(), d.message.clone()))).collect();
    errors.sort_by_key(|d| d.range.start);
    (errors, v.missing)
}

fn type_matches(node: &Node, t: &str) -> bool {
    match (t, &node.value) {
        ("null", Value::Null) | ("boolean", Value::Bool(_)) | ("string", Value::String(_)) => true,
        ("array", Value::Array(_)) | ("object", Value::Object(_)) | ("number", Value::Number(_)) => true,
        ("integer", Value::Number(n)) => n.fract() == 0.0 && n.is_finite(),
        _ => false,
    }
}

fn types_of(s: &Json) -> Vec<String> {
    match s.get("type") {
        Some(Json::String(t)) => vec![t.clone()],
        Some(Json::Array(a)) => a.iter().filter_map(|t| t.as_str().map(str::to_owned)).collect(),
        _ => Vec::new(),
    }
}

fn json_eq(a: &Json, b: &Json) -> bool {
    match (a, b) {
        (Json::Number(x), Json::Number(y)) => x.as_f64() == y.as_f64(),
        (Json::Array(x), Json::Array(y)) => x.len() == y.len() && x.iter().zip(y).all(|(a, b)| json_eq(a, b)),
        (Json::Object(x), Json::Object(y)) => x.len() == y.len() && x.iter().all(|(k, v)| y.get(k).is_some_and(|w| json_eq(v, w))),
        _ => a == b,
    }
}

fn short(v: &Json) -> String {
    let s = v.to_string();
    if s.chars().count() > 40 {
        format!("{}…", s.chars().take(39).collect::<String>())
    } else {
        s
    }
}

fn num(v: &Json) -> Option<f64> {
    v.as_f64()
}

/// The first line of an object or array (an underline over the whole block
/// would be too much).
fn head(node: &Node, key: Option<&Range<usize>>) -> Range<usize> {
    if let Some(k) = key {
        return k.clone();
    }
    node.range.start..node.range.end.min(node.range.start + 1)
}

impl Validator<'_> {
    fn check(&mut self, node: &Node, key: Option<&Range<usize>>, s: &Sch) -> Eval {
        let mut e = Eval::default();
        if self.depth > 48 {
            return e;
        }
        let here = || match &node.value {
            Value::Object(_) | Value::Array(_) => head(node, key),
            _ => node.range.clone(),
        };
        match s.node {
            Json::Bool(true) => return e,
            Json::Bool(false) => {
                e.errors.push(Diagnostic::error(here(), t!("Value is not allowed")));
                return e;
            }
            Json::Object(_) => {}
            _ => return e,
        }
        self.depth += 1;

        for keyword in ["$ref", "$dynamicRef", "$recursiveRef"] {
            if let Some(r) = s.get(keyword).and_then(Json::as_str) {
                if let Some(target) = resolve_ref(self.src, s, r, &mut self.missing) {
                    let sub = self.check(node, key, &target);
                    if sub.type_mismatch.is_some() && e.type_mismatch.is_none() {
                        e.type_mismatch = sub.type_mismatch.clone();
                    }
                    e.merge(sub);
                }
            }
        }

        // Type.
        let types = types_of(s.node);
        if !types.is_empty() && !types.iter().any(|t| type_matches(node, t) || (t == "number" && type_matches(node, "integer"))) {
            e.errors.push(Diagnostic::error(here(), t!("Incorrect type: expected {} but found {}", types.join(" | "), node.type_name())));
            e.type_mismatch = Some(types);
            self.depth -= 1;
            return e;
        }
        let json = || node.to_json();
        if let Some(Json::Array(values)) = s.get("enum") {
            let v = json();
            if !values.iter().any(|x| json_eq(x, &v)) {
                let list: Vec<String> = values.iter().take(12).map(short).collect();
                let more = if values.len() > 12 { ", …" } else { "" };
                e.errors.push(Diagnostic::error(here(), t!("Value is not accepted. Valid values: {}", format!("{}{more}", list.join(", ")))));
            }
        }
        if let Some(c) = s.get("const") {
            if !json_eq(c, &json()) {
                e.errors.push(Diagnostic::error(here(), t!("Value must be {}", short(c))));
            }
        }

        match &node.value {
            Value::Number(n) => self.numbers(*n, node, s, &mut e),
            Value::String(text) => self.strings(text, node, s, &mut e),
            Value::Array(items) => self.arrays(items, node, key, s, &mut e),
            Value::Object(members) => self.objects(members, node, key, s, &mut e),
            _ => {}
        }

        // Combinators.
        if let Some(Json::Array(all)) = s.get("allOf") {
            for sub in all {
                let r = self.check(node, key, &s.child(sub));
                e.merge(r);
            }
        }
        if let Some(Json::Array(any)) = s.get("anyOf") {
            self.alternatives(node, key, s, any, false, &mut e);
        }
        if let Some(Json::Array(one)) = s.get("oneOf") {
            self.alternatives(node, key, s, one, true, &mut e);
        }
        if let Some(not) = s.get("not") {
            let r = self.check(node, key, &s.child(not));
            if r.ok() {
                e.errors.push(Diagnostic::error(here(), t!("Value matches a schema that is not allowed")));
            }
        }
        if let Some(cond) = s.get("if") {
            let r = self.check(node, key, &s.child(cond));
            let branch = if r.ok() {
                e.annotations(&r);
                s.get("then")
            } else {
                s.get("else")
            };
            if let Some(b) = branch {
                let r = self.check(node, key, &s.child(b));
                e.merge(r);
            }
        }

        // Unevaluated properties and items (after all subschemas).
        if let Value::Object(members) = &node.value {
            if let Some(extra) = s.get("unevaluatedProperties") {
                for m in members.iter().filter(|m| !e.props.contains(&m.key)) {
                    let r = self.check_member(m, &s.child(extra));
                    e.errors.extend(r.errors);
                }
                e.props.extend(members.iter().map(|m| m.key.clone()));
            }
        }
        if let Value::Array(items) = &node.value {
            if let Some(extra) = s.get("unevaluatedItems") {
                for item in items.iter().skip(e.items) {
                    let r = self.check(item, None, &s.child(extra));
                    e.errors.extend(r.errors);
                }
                e.items = items.len();
            }
        }
        self.depth -= 1;
        e
    }

    fn check_member(&mut self, m: &Member, s: &Sch) -> Eval {
        if s.node == &Json::Bool(false) {
            let mut e = Eval::default();
            e.errors.push(Diagnostic::error(m.key_range.clone(), t!("Property \"{}\" is not allowed", m.key)));
            return e;
        }
        let e = self.check(&m.value, Some(&m.key_range), s);
        self.deprecated(m, s, e)
    }

    /// Deprecated properties (`deprecated: true`, `deprecationMessage`).
    fn deprecated(&mut self, m: &Member, s: &Sch, mut e: Eval) -> Eval {
        let message = s.get("deprecationMessage").and_then(Json::as_str).map(str::to_owned).or_else(|| {
            (s.get("deprecated") == Some(&Json::Bool(true))).then(|| t!("\"{}\" is deprecated", m.key))
        });
        if let Some(message) = message {
            e.errors.push(Diagnostic::warning(m.key_range.clone(), message));
        }
        e
    }

    fn alternatives(&mut self, node: &Node, key: Option<&Range<usize>>, s: &Sch, branches: &'static [Json], one: bool, e: &mut Eval) {
        let results: Vec<Eval> = branches.iter().map(|b| self.check(node, key, &s.child(b))).collect();
        let matching: Vec<&Eval> = results.iter().filter(|r| r.ok()).collect();
        if !matching.is_empty() {
            for r in &matching {
                e.annotations(r);
            }
            if one && matching.len() > 1 {
                let range = match &node.value {
                    Value::Object(_) | Value::Array(_) => head(node, key),
                    _ => node.range.clone(),
                };
                e.errors.push(Diagnostic::error(range, t!("Value matches more than one of the allowed schemas")));
            }
            return;
        }
        // No branch fits: report the closest one. When none has the right
        // type, say which types would do.
        let typed: Vec<&Eval> = results.iter().filter(|r| r.type_mismatch.is_none()).collect();
        if typed.is_empty() {
            let mut types: Vec<String> = Vec::new();
            for r in &results {
                for t in r.type_mismatch.iter().flatten() {
                    if !types.contains(t) {
                        types.push(t.clone());
                    }
                }
            }
            let range = match &node.value {
                Value::Object(_) | Value::Array(_) => head(node, key),
                _ => node.range.clone(),
            };
            if !types.is_empty() {
                e.errors.push(Diagnostic::error(range, t!("Incorrect type: expected {} but found {}", types.join(" | "), node.type_name())));
                e.type_mismatch = Some(types);
            }
            return;
        }
        let best = typed.into_iter().min_by_key(|r| r.errors.iter().filter(|d| d.severity == Severity::Error).count()).expect("branch");
        e.errors.extend(best.errors.iter().cloned());
    }

    fn numbers(&mut self, n: f64, node: &Node, s: &Sch, e: &mut Eval) {
        let range = node.range.clone();
        if let Some(m) = s.get("minimum").and_then(num) {
            if s.get("exclusiveMinimum") == Some(&Json::Bool(true)) {
                if n <= m {
                    e.errors.push(Diagnostic::error(range.clone(), t!("Value must be greater than {}", m)));
                }
            } else if n < m {
                e.errors.push(Diagnostic::error(range.clone(), t!("Value is below the minimum of {}", m)));
            }
        }
        if let Some(m) = s.get("maximum").and_then(num) {
            if s.get("exclusiveMaximum") == Some(&Json::Bool(true)) {
                if n >= m {
                    e.errors.push(Diagnostic::error(range.clone(), t!("Value must be less than {}", m)));
                }
            } else if n > m {
                e.errors.push(Diagnostic::error(range.clone(), t!("Value is above the maximum of {}", m)));
            }
        }
        if let Some(m) = s.get("exclusiveMinimum").and_then(num) {
            if n <= m {
                e.errors.push(Diagnostic::error(range.clone(), t!("Value must be greater than {}", m)));
            }
        }
        if let Some(m) = s.get("exclusiveMaximum").and_then(num) {
            if n >= m {
                e.errors.push(Diagnostic::error(range.clone(), t!("Value must be less than {}", m)));
            }
        }
        if let Some(m) = s.get("multipleOf").and_then(num).filter(|m| *m > 0.0) {
            let q = n / m;
            if (q - q.round()).abs() > 1e-9 {
                e.errors.push(Diagnostic::error(range, t!("Value is not a multiple of {}", m)));
            }
        }
    }

    fn strings(&mut self, text: &str, node: &Node, s: &Sch, e: &mut Eval) {
        let range = node.range.clone();
        let len = text.chars().count() as f64;
        if let Some(m) = s.get("minLength").and_then(num) {
            if len < m {
                e.errors.push(Diagnostic::error(range.clone(), t!("String is shorter than {} characters", m)));
            }
        }
        if let Some(m) = s.get("maxLength").and_then(num) {
            if len > m {
                e.errors.push(Diagnostic::error(range.clone(), t!("String is longer than {} characters", m)));
            }
        }
        if let Some(p) = s.get("pattern").and_then(Json::as_str) {
            if regex(p).is_some_and(|re| !re.is_match(text)) {
                let message = s
                    .get("patternErrorMessage")
                    .or_else(|| s.get("errorMessage"))
                    .and_then(Json::as_str)
                    .map(str::to_owned)
                    .unwrap_or_else(|| t!("String does not match the pattern \"{}\"", p));
                e.errors.push(Diagnostic::error(range, message));
            }
        }
    }

    fn arrays(&mut self, items: &[Node], node: &Node, key: Option<&Range<usize>>, s: &Sch, e: &mut Eval) {
        let here = head(node, key);
        let mut evaluated = 0;
        // Tuples: `prefixItems` (2020-12) or an `items` array (older drafts).
        let tuple = s.get("prefixItems").or_else(|| s.get("items").filter(|i| i.is_array()));
        let rest = if s.get("prefixItems").is_some() { s.get("items") } else { s.get("additionalItems") };
        if let Some(Json::Array(prefix)) = tuple {
            for (item, sub) in items.iter().zip(prefix) {
                let r = self.check(item, None, &s.child(sub));
                e.errors.extend(r.errors);
            }
            evaluated = prefix.len().min(items.len());
            if let Some(rest) = rest {
                if rest == &Json::Bool(false) && items.len() > prefix.len() {
                    e.errors.push(Diagnostic::error(here.clone(), t!("Array has more than {} items", prefix.len())));
                } else {
                    for item in items.iter().skip(prefix.len()) {
                        let r = self.check(item, None, &s.child(rest));
                        e.errors.extend(r.errors);
                    }
                }
                evaluated = items.len();
            }
        } else if let Some(all) = s.get("items") {
            for item in items {
                let r = self.check(item, None, &s.child(all));
                e.errors.extend(r.errors);
            }
            evaluated = items.len();
        }
        e.items = e.items.max(evaluated);
        if let Some(m) = s.get("minItems").and_then(num) {
            if (items.len() as f64) < m {
                e.errors.push(Diagnostic::error(here.clone(), t!("Array has fewer than {} items", m)));
            }
        }
        if let Some(m) = s.get("maxItems").and_then(num) {
            if (items.len() as f64) > m {
                e.errors.push(Diagnostic::error(here.clone(), t!("Array has more than {} items", m)));
            }
        }
        if s.get("uniqueItems") == Some(&Json::Bool(true)) {
            let values: Vec<Json> = items.iter().map(Node::to_json).collect();
            for (i, v) in values.iter().enumerate() {
                if values[..i].iter().any(|w| json_eq(v, w)) {
                    e.errors.push(Diagnostic::error(items[i].range.clone(), t!("Array has duplicate items")));
                }
            }
        }
        if let Some(contains) = s.get("contains") {
            let hits = items.iter().filter(|item| self.check(item, None, &s.child(contains)).ok()).count() as f64;
            let min = s.get("minContains").and_then(num).unwrap_or(1.0);
            if hits < min {
                e.errors.push(Diagnostic::error(here.clone(), t!("Array does not contain a required item")));
            }
            if let Some(max) = s.get("maxContains").and_then(num) {
                if hits > max {
                    e.errors.push(Diagnostic::error(here, t!("Array has more than {} items", max)));
                }
            }
        }
    }

    fn objects(&mut self, members: &[Member], node: &Node, key: Option<&Range<usize>>, s: &Sch, e: &mut Eval) {
        let here = head(node, key);
        let props = s.get("properties").and_then(Json::as_object);
        let patterns = s.get("patternProperties").and_then(Json::as_object);
        for m in members {
            let mut matched = false;
            if let Some(sub) = props.and_then(|p| p.get(&m.key)) {
                matched = true;
                let r = self.check_member(m, &s.child(sub));
                e.errors.extend(r.errors);
            }
            for (pattern, sub) in patterns.into_iter().flatten() {
                if regex(pattern).is_some_and(|re| re.is_match(&m.key)) {
                    matched = true;
                    let r = self.check_member(m, &s.child(sub));
                    e.errors.extend(r.errors);
                }
            }
            if matched {
                e.props.insert(m.key.clone());
            } else if let Some(extra) = s.get("additionalProperties") {
                let r = self.check_member(m, &s.child(extra));
                e.errors.extend(r.errors);
                e.props.insert(m.key.clone());
            }
            if let Some(names) = s.get("propertyNames") {
                let key_node = Node::new(Value::String(m.key.clone()), m.key_range.clone());
                let r = self.check(&key_node, None, &s.child(names));
                e.errors.extend(r.errors);
            }
        }
        let has = |k: &str| members.iter().any(|m| m.key == k);
        if let Some(Json::Array(required)) = s.get("required") {
            for r in required.iter().filter_map(Json::as_str) {
                if !has(r) {
                    e.errors.push(Diagnostic::error(here.clone(), t!("Missing property \"{}\"", r)));
                }
            }
        }
        let dependent = |map: Option<&'static Json>, e: &mut Eval, this: &mut Self| {
            for (k, dep) in map.and_then(Json::as_object).into_iter().flatten() {
                let Some(m) = members.iter().find(|m| &m.key == k) else { continue };
                match dep {
                    Json::Array(names) => {
                        for n in names.iter().filter_map(Json::as_str).filter(|n| !has(n)) {
                            e.errors.push(Diagnostic::error(m.key_range.clone(), t!("Property \"{}\" requires property \"{}\"", k, n)));
                        }
                    }
                    sub => {
                        let r = this.check(node, key, &s.child(sub));
                        e.merge(r);
                    }
                }
            }
        };
        dependent(s.get("dependencies"), e, self);
        dependent(s.get("dependentRequired"), e, self);
        dependent(s.get("dependentSchemas"), e, self);
        if let Some(m) = s.get("minProperties").and_then(num) {
            if (members.len() as f64) < m {
                e.errors.push(Diagnostic::error(here.clone(), t!("Object has fewer than {} properties", m)));
            }
        }
        if let Some(m) = s.get("maxProperties").and_then(num) {
            if (members.len() as f64) > m {
                e.errors.push(Diagnostic::error(here, t!("Object has more than {} properties", m)));
            }
        }
    }
}

// ----- completion ---------------------------------------------------------

/// How proposals are written.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Syntax {
    Json,
    /// YAML; the indentation of the line being completed.
    Yaml(usize),
}

/// The schemas that apply at `path`, with references and combinators
/// expanded (`anyOf`/`oneOf` branches all count).
pub fn schemas_at(src: &dyn Source, root: Sch, path: &[Seg]) -> Vec<Sch> {
    let mut missing = Vec::new();
    let mut current = expand(src, root, &mut missing);
    for seg in path {
        let mut next = Vec::new();
        for s in &current {
            for sub in step(s, seg) {
                next.extend(expand(src, sub, &mut missing));
            }
        }
        current = next;
        if current.is_empty() {
            break;
        }
    }
    current
}

fn expand(src: &dyn Source, s: Sch, missing: &mut Vec<String>) -> Vec<Sch> {
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    let mut todo = vec![s];
    while let Some(s) = todo.pop() {
        if !seen.insert(s.node as *const Json as usize) || seen.len() > 400 {
            continue;
        }
        if !s.node.is_object() {
            continue;
        }
        for keyword in ["$ref", "$dynamicRef", "$recursiveRef"] {
            if let Some(r) = s.get(keyword).and_then(Json::as_str) {
                if let Some(t) = resolve_ref(src, &s, r, missing) {
                    todo.push(t);
                }
            }
        }
        for keyword in ["allOf", "anyOf", "oneOf"] {
            if let Some(Json::Array(subs)) = s.get(keyword) {
                todo.extend(subs.iter().map(|sub| s.child(sub)));
            }
        }
        for keyword in ["then", "else"] {
            if let Some(sub) = s.get(keyword) {
                todo.push(s.child(sub));
            }
        }
        out.push(s);
    }
    out
}

fn step(s: &Sch, seg: &Seg) -> Vec<Sch> {
    let mut out = Vec::new();
    match seg {
        Seg::Key(k) => {
            if let Some(sub) = s.get("properties").and_then(|p| p.get(k)) {
                out.push(s.child(sub));
            }
            for (pattern, sub) in s.get("patternProperties").and_then(Json::as_object).into_iter().flatten() {
                if regex(pattern).is_some_and(|re| re.is_match(k)) {
                    out.push(s.child(sub));
                }
            }
            if out.is_empty() {
                if let Some(sub) = s.get("additionalProperties").filter(|v| v.is_object()) {
                    out.push(s.child(sub));
                }
            }
        }
        Seg::Index(i) => {
            let tuple = s.get("prefixItems").or_else(|| s.get("items").filter(|v| v.is_array()));
            if let Some(sub) = tuple.and_then(|t| t.get(*i)) {
                out.push(s.child(sub));
            } else if let Some(sub) = s.get("items").filter(|v| v.is_object()) {
                out.push(s.child(sub));
            } else if let Some(sub) = s.get("additionalItems").filter(|v| v.is_object()) {
                out.push(s.child(sub));
            }
        }
    }
    out
}

fn description(s: &Json) -> String {
    let d = s.get("description").or_else(|| s.get("markdownDescription")).or_else(|| s.get("title")).and_then(Json::as_str).unwrap_or("");
    let first = d.lines().next().unwrap_or("").trim();
    if first.chars().count() > 90 {
        format!("{}…", first.chars().take(89).collect::<String>())
    } else {
        first.to_owned()
    }
}

fn matches_prefix(label: &str, prefix: &str) -> bool {
    prefix.is_empty() || label.to_lowercase().contains(&prefix.to_lowercase())
}

/// The value written in the document's syntax.
fn literal(v: &Json, syntax: Syntax) -> String {
    match (syntax, v) {
        (Syntax::Yaml(_), Json::String(s)) => crate::yaml_lint::scalar(s),
        (Syntax::Yaml(_), Json::Null) => "null".into(),
        (Syntax::Yaml(_), Json::Array(_) | Json::Object(_)) => v.to_string(),
        _ => v.to_string(),
    }
}

/// Property names for an object at `path`.
pub fn key_proposals(src: &dyn Source, root: Sch, path: &[Seg], existing: &[String], prefix: &str, syntax: Syntax, has_colon: bool) -> Vec<Proposal> {
    let schemas = schemas_at(src, root, path);
    let mut required: HashSet<String> = HashSet::new();
    for s in &schemas {
        for r in s.get("required").and_then(Json::as_array).into_iter().flatten().filter_map(Json::as_str) {
            required.insert(r.to_owned());
        }
    }
    let mut out: Vec<(bool, Proposal)> = Vec::new();
    let mut seen: HashSet<String> = existing.iter().cloned().collect();
    for s in &schemas {
        for (name, sub) in s.get("properties").and_then(Json::as_object).into_iter().flatten() {
            if !matches_prefix(name, prefix) || !seen.insert(name.clone()) {
                continue;
            }
            // Expand references for the description and type of the property.
            let parts = expand(src, s.child(sub), &mut Vec::new());
            let info = parts.iter().map(|p| description(p.node)).find(|d| !d.is_empty()).unwrap_or_default();
            let types: Vec<String> = parts.iter().flat_map(|p| types_of(p.node)).collect();
            let default = parts.iter().find_map(|p| p.get("default"));
            let deprecated = parts.iter().any(|p| p.get("deprecated") == Some(&Json::Bool(true)) || p.get("deprecationMessage").is_some());
            if deprecated {
                continue;
            }
            let (insert, back) = key_insert(name, &types, default, syntax, has_colon);
            let req = required.contains(name);
            let label = if req { format!("{name} *") } else { name.clone() };
            out.push((req, Proposal { label, insert, info, back }));
        }
    }
    out.sort_by_key(|(req, _)| !*req);
    out.into_iter().map(|(_, p)| p).collect()
}

fn key_insert(name: &str, types: &[String], default: Option<&Json>, syntax: Syntax, has_colon: bool) -> (String, usize) {
    let one = |t: &str| types.len() == 1 && types[0] == t;
    match syntax {
        Syntax::Json => {
            let key = serde_json::to_string(name).unwrap_or_default();
            if has_colon {
                return (key, 0);
            }
            if let Some(d) = default.filter(|d| !d.is_object() && !d.is_array()) {
                return (format!("{key}: {d}"), 0);
            }
            if one("string") {
                (format!("{key}: \"\""), 1)
            } else if one("object") {
                (format!("{key}: {{}}"), 1)
            } else if one("array") {
                (format!("{key}: []"), 1)
            } else {
                (format!("{key}: "), 0)
            }
        }
        Syntax::Yaml(indent) => {
            let key = crate::yaml_lint::scalar(name);
            if has_colon {
                return (key, 0);
            }
            let pad = " ".repeat(indent + 2);
            if let Some(d) = default.filter(|d| !d.is_object() && !d.is_array()) {
                return (format!("{key}: {}", literal(d, syntax)), 0);
            }
            if one("object") {
                (format!("{key}:\n{pad}"), 0)
            } else if one("array") {
                (format!("{key}:\n{pad}- "), 0)
            } else {
                (format!("{key}: "), 0)
            }
        }
    }
}

/// Values for the value at `path`.
pub fn value_proposals(src: &dyn Source, root: Sch, path: &[Seg], prefix: &str, syntax: Syntax) -> Vec<Proposal> {
    let schemas = schemas_at(src, root, path);
    let mut out: Vec<Proposal> = Vec::new();
    let mut seen = HashSet::new();
    let mut add = |v: &Json, info: String, out: &mut Vec<Proposal>| {
        let text = literal(v, syntax);
        let label = match v {
            Json::String(s) => s.clone(),
            _ => text.clone(),
        };
        if matches_prefix(&label, prefix) && seen.insert(text.clone()) {
            out.push(Proposal { label, insert: text, info, back: 0 });
        }
    };
    for s in &schemas {
        let info = description(s.node);
        if let Some(Json::Array(values)) = s.get("enum") {
            let docs = s.get("enumDescriptions").or_else(|| s.get("markdownEnumDescriptions")).and_then(Json::as_array);
            for (i, v) in values.iter().enumerate() {
                let d = docs.and_then(|d| d.get(i)).and_then(Json::as_str).map_or_else(|| info.clone(), str::to_owned);
                add(v, d, &mut out);
            }
        }
        if let Some(c) = s.get("const") {
            add(c, info.clone(), &mut out);
        }
        if let Some(d) = s.get("default") {
            add(d, t!("Default").into(), &mut out);
        }
        for ex in s.get("examples").and_then(Json::as_array).into_iter().flatten() {
            add(ex, t!("Example").into(), &mut out);
        }
        let types = types_of(s.node);
        if types.iter().any(|t| t == "boolean") {
            add(&Json::Bool(true), String::new(), &mut out);
            add(&Json::Bool(false), String::new(), &mut out);
        }
        if types.iter().any(|t| t == "null") {
            add(&Json::Null, String::new(), &mut out);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    struct Docs(HashMap<String, &'static Json>);

    impl Source for Docs {
        fn load(&self, url: &str) -> Option<&'static Json> {
            self.0.get(url).copied()
        }
    }

    fn docs(list: &[(&str, Json)]) -> Docs {
        Docs(list.iter().map(|(u, v)| (u.to_string(), &*Box::leak(Box::new(v.clone())))).collect())
    }

    fn check(schema: Json, text: &str) -> Vec<String> {
        let d = docs(&[("https://x.test/s.json", schema)]);
        let root = crate::json_lint::parse(text).root.unwrap();
        let (errors, _) = validate(&d, Sch::new(d.load("https://x.test/s.json").unwrap(), "https://x.test/s.json"), &root);
        errors.into_iter().map(|e| e.message).collect()
    }

    #[test]
    fn joins_urls() {
        assert_eq!(join("https://a.b/x/y.json", "z.json"), "https://a.b/x/z.json");
        assert_eq!(join("https://a.b/x/y.json", "../z.json#/d"), "https://a.b/z.json#/d");
        assert_eq!(join("https://a.b/x/y.json#/q", "#/defs/a"), "https://a.b/x/y.json#/defs/a");
        assert_eq!(join("https://a.b/x/y.json", "/r.json"), "https://a.b/r.json");
        assert_eq!(join("file:///home/u/s.json", "t.json"), "file:///home/u/t.json");
        assert_eq!(join("https://a.b/x/y.json", "https://c.d/e"), "https://c.d/e");
    }

    #[test]
    fn validates_keywords() {
        let schema = json!({
            "type": "object",
            "required": ["name", "port"],
            "additionalProperties": false,
            "properties": {
                "name": {"type": "string", "minLength": 2, "pattern": "^[a-z]+$"},
                "port": {"type": "integer", "minimum": 1, "maximum": 65535},
                "mode": {"enum": ["a", "b"]},
                "tags": {"type": "array", "items": {"type": "string"}, "uniqueItems": true},
                "old": {"deprecated": true}
            }
        });
        assert!(check(schema.clone(), r#"{"name": "web", "port": 80}"#).is_empty());
        assert_eq!(
            check(schema.clone(), r#"{"name": "W", "port": 0.5, "mode": "c", "tags": ["x", "x", 1], "x": 1, "old": 1}"#),
            [
                "String is shorter than 2 characters",
                "String does not match the pattern \"^[a-z]+$\"",
                "Incorrect type: expected integer but found number",
                "Value is not accepted. Valid values: \"a\", \"b\"",
                "Array has duplicate items",
                "Incorrect type: expected string but found integer",
                "Property \"x\" is not allowed",
                "\"old\" is deprecated",
            ]
        );
        assert_eq!(check(schema, r#"{"name": "ab"}"#), ["Missing property \"port\""]);
    }

    #[test]
    fn references_and_combinators() {
        let schema = json!({
            "$defs": {"port": {"type": "integer", "maximum": 10}},
            "properties": {
                "a": {"$ref": "#/$defs/port"},
                "b": {"anyOf": [{"type": "string"}, {"type": "boolean"}]},
                "c": {"oneOf": [{"type": "object", "required": ["x"]}, {"type": "object", "required": ["y"]}]},
                "d": {"$ref": "other.json#/$defs/n"},
                "e": {"if": {"properties": {"k": {"const": 1}}}, "then": {"required": ["v"]}},
                "f": {"allOf": [{"properties": {"p": {}}}], "unevaluatedProperties": false}
            }
        });
        let other = json!({"$defs": {"n": {"type": "null"}}});
        let d = docs(&[("https://x.test/s.json", schema), ("https://x.test/other.json", other)]);
        let run = |text: &str| {
            let root = crate::json_lint::parse(text).root.unwrap();
            let (errors, missing) = validate(&d, Sch::new(d.load("https://x.test/s.json").unwrap(), "https://x.test/s.json"), &root);
            assert!(missing.is_empty());
            errors.into_iter().map(|e| e.message).collect::<Vec<_>>()
        };
        assert!(run(r#"{"a": 3, "b": true, "c": {"x": 1}, "d": null, "e": {"k": 2}, "f": {"p": 1}}"#).is_empty());
        assert_eq!(
            run(r#"{"a": 11, "b": 1, "c": {"x": 1, "y": 2}, "d": 1, "e": {"k": 1}, "f": {"q": 1}}"#),
            [
                "Value is above the maximum of 10",
                "Incorrect type: expected string | boolean but found integer",
                "Value matches more than one of the allowed schemas",
                "Incorrect type: expected null but found integer",
                "Missing property \"v\"",
                "Property \"q\" is not allowed",
            ]
        );
    }

    #[test]
    fn relative_ids_keep_local_references() {
        let schema = json!({"$id": "renamed.json", "properties": {"a": {"$ref": "#/definitions/n"}}, "definitions": {"n": {"type": "integer"}}});
        assert_eq!(check(schema, r#"{"a": "x"}"#), ["Incorrect type: expected integer but found string"]);
    }

    #[test]
    fn missing_documents_are_reported() {
        let d = docs(&[("https://x.test/s.json", json!({"$ref": "https://y.test/t.json"}))]);
        let root = crate::json_lint::parse("{}").root.unwrap();
        let (_, missing) = validate(&d, Sch::new(d.load("https://x.test/s.json").unwrap(), "https://x.test/s.json"), &root);
        assert_eq!(missing, ["https://y.test/t.json"]);
    }

    #[test]
    fn completes_keys_and_values() {
        let schema = json!({
            "required": ["name"],
            "properties": {
                "image": {"type": "string", "description": "Image to run"},
                "name": {"type": "string"},
                "ports": {"type": "array", "items": {"$ref": "#/$defs/proto"}},
                "env": {"type": "object"},
                "restart": {"enum": ["no", "always"], "default": "no"}
            },
            "$defs": {"proto": {"enum": ["tcp", "udp"]}}
        });
        let d = docs(&[("https://x.test/s.json", schema)]);
        let root = || Sch::new(d.load("https://x.test/s.json").unwrap(), "https://x.test/s.json");
        let keys = key_proposals(&d, root(), &[], &["env".into()], "", Syntax::Json, false);
        let labels: Vec<&str> = keys.iter().map(|p| p.label.as_str()).collect();
        assert_eq!(labels, ["name *", "image", "ports", "restart"]);
        assert_eq!((keys[1].insert.as_str(), keys[1].back, keys[1].info.as_str()), ("\"image\": \"\"", 1, "Image to run"));
        let keys = key_proposals(&d, root(), &[], &[], "po", Syntax::Yaml(2), false);
        assert_eq!(keys[0].insert, "ports:\n    - ");
        let values = value_proposals(&d, root(), &[Seg::Key("ports".into()), Seg::Index(0)], "", Syntax::Json);
        assert_eq!(values.iter().map(|p| p.insert.as_str()).collect::<Vec<_>>(), ["\"tcp\"", "\"udp\""]);
        let values = value_proposals(&d, root(), &[Seg::Key("restart".into())], "", Syntax::Yaml(0));
        assert_eq!(values.iter().map(|p| p.insert.as_str()).collect::<Vec<_>>(), ["\"no\"", "always"]);
    }
}
