//! Checks and completion for JSON, YAML and XML notes: finds the schema
//! (assigned to the note, named in the document or from the SchemaStore
//! catalog) and runs the format's checks.

use crate::datadoc::{Diagnostic, Proposal, Spot};
use crate::json_schema::{self, Sch, Syntax};
use crate::schemas::{self, Loader};
use crate::xml_lint::Location;
use std::ops::Range;
use std::path::PathBuf;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Json,
    Yaml,
    Xml,
}

impl Kind {
    pub fn of(language: Option<&str>) -> Option<Self> {
        match language? {
            "json" => Some(Self::Json),
            "yaml" => Some(Self::Yaml),
            "xml" | "xslt" => Some(Self::Xml),
            _ => None,
        }
    }
}

/// What the checks know about the note.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Context {
    /// Schema assigned to the note (URL or path).
    pub assigned: Option<String>,
    /// The note's file.
    pub path: Option<PathBuf>,
    /// The note's title (matched against the catalog when there is no file).
    pub title: Option<String>,
    pub rules: crate::yaml_lint::Rules,
}

impl Context {
    fn base_url(&self) -> Option<String> {
        self.path.as_deref().map(schemas::file_url)
    }

    /// A schema reference resolved against the note's file.
    fn resolve(&self, reference: &str) -> String {
        let reference = reference.trim();
        if reference.starts_with('/') {
            return schemas::file_url(std::path::Path::new(reference));
        }
        if let Some(home) = reference.strip_prefix("~/") {
            return schemas::file_url(&dirs::home_dir().unwrap_or_default().join(home));
        }
        match self.base_url() {
            Some(base) => json_schema::join(&base, reference),
            None => reference.to_owned(),
        }
    }

    /// The name matched against the SchemaStore catalog.
    fn catalog_name(&self) -> Option<String> {
        match &self.path {
            Some(p) => Some(p.to_string_lossy().into_owned()),
            None => self.title.clone().filter(|t| t.contains('.') && !t.contains(char::is_whitespace)),
        }
    }
}

/// Where the schema in use comes from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Origin {
    Assigned,
    Document,
    Catalog,
}

/// Result of checking a document.
#[derive(Default)]
pub struct Report {
    pub diagnostics: Vec<Diagnostic>,
    /// The schema (or schemas) used, with where they come from.
    pub schema: Option<(String, Origin)>,
    /// Schemas are still downloading; check again when they are there.
    pub loading: bool,
}

/// The schema of a JSON or YAML document and the range of its reference in
/// the document (for problems loading it).
fn json_schema_url(kind: Kind, text: &str, root: Option<&crate::datadoc::Node>, ctx: &Context) -> Option<(String, Origin, Option<Range<usize>>)> {
    if let Some(a) = ctx.assigned.as_deref().filter(|a| !a.trim().is_empty()) {
        return Some((ctx.resolve(a), Origin::Assigned, None));
    }
    if kind == Kind::Yaml {
        // `# yaml-language-server: $schema=<url>`
        for (i, line) in text.lines().enumerate().take(50) {
            if let Some(at) = line.find("$schema=").filter(|_| line.trim_start().starts_with('#')) {
                let url = line[at + 8..].trim();
                let start: usize = text.lines().take(i).map(|l| l.len() + 1).sum::<usize>() + at + 8;
                return Some((ctx.resolve(url), Origin::Document, Some(start..start + url.len())));
            }
        }
    }
    if let Some(m) = root.and_then(|r| r.member("$schema")) {
        if let crate::datadoc::Value::String(url) = &m.value.value {
            return Some((ctx.resolve(url), Origin::Document, Some(m.value.range.clone())));
        }
    }
    let name = ctx.catalog_name()?;
    schemas::catalog_match(&name).map(|url| (url, Origin::Catalog, None))
}

fn xml_locations(text: &str, ctx: &Context) -> (Vec<Location>, Option<Origin>) {
    if let Some(a) = ctx.assigned.as_deref().filter(|a| !a.trim().is_empty()) {
        return (vec![Location { namespace: None, url: ctx.resolve(a) }], Some(Origin::Assigned));
    }
    let base = ctx.base_url();
    let found = crate::xsd::schema_locations(text, base.as_deref());
    let origin = (!found.is_empty()).then_some(Origin::Document);
    // libxml2 reads local schemas by path.
    let found = found
        .into_iter()
        .map(|l| match schemas::local_path(&l.url) {
            Some(p) => Location { namespace: l.namespace, url: p.to_string_lossy().into_owned() },
            None => l,
        })
        .collect();
    (found, origin)
}

/// Checks a document.
pub fn check(kind: Kind, text: &str, ctx: &Context) -> Report {
    let mut report = Report::default();
    match kind {
        Kind::Json | Kind::Yaml => {
            let (roots, diags) = match kind {
                Kind::Json => {
                    let p = crate::json_lint::parse(text);
                    (p.root.into_iter().collect::<Vec<_>>(), p.diagnostics)
                }
                _ => {
                    let p = crate::yaml_lint::parse(text, &ctx.rules);
                    (p.documents, p.diagnostics)
                }
            };
            report.diagnostics = diags;
            // An unfinished document still names its schema.
            let head = roots.first().cloned().or_else(|| schema_only(kind, text));
            let Some((url, origin, at)) = json_schema_url(kind, text, head.as_ref(), ctx) else { return report };
            report.schema = Some((url.clone(), origin.clone()));
            let Some(doc) = schemas::json(&url) else {
                if schemas::is_loading(&url) {
                    report.loading = true;
                } else if origin != Origin::Catalog {
                    let reason = schemas::failure(&url).unwrap_or_else(|| t!("not found").into());
                    let range = at.unwrap_or(0..text.find('\n').unwrap_or(text.len()));
                    report.diagnostics.push(Diagnostic::warning(range, t!("Schema could not be loaded: {}", reason)));
                }
                return report;
            };
            let has_errors = report.diagnostics.iter().any(|d| d.severity == crate::datadoc::Severity::Error);
            for root in roots.iter().filter(|_| !has_errors || kind == Kind::Yaml) {
                let (errors, missing) = json_schema::validate(&Loader, Sch::new(doc, &url), root);
                report.diagnostics.extend(errors);
                if missing.iter().any(|m| schemas::is_loading(m)) {
                    report.loading = true;
                }
            }
        }
        Kind::Xml => {
            let (locations, origin) = xml_locations(text, ctx);
            if let Some(origin) = origin {
                let names: Vec<String> = locations.iter().map(|l| l.url.clone()).collect();
                report.schema = Some((names.join(", "), origin));
                match schemas::xsd_state(&names) {
                    schemas::XsdState::Ready => {
                        report.diagnostics = crate::xml_lint::check(text, ctx.base_url().as_deref(), &locations);
                    }
                    state => {
                        report.diagnostics = crate::xml_lint::check(text, ctx.base_url().as_deref(), &[]);
                        match state {
                            schemas::XsdState::Loading => report.loading = true,
                            schemas::XsdState::Failed(url, reason) => {
                                let range = root_name(text);
                                report.diagnostics.push(Diagnostic::warning(range, t!("Schema could not be loaded: {}", format!("{url}: {reason}"))));
                            }
                            schemas::XsdState::Ready => {}
                        }
                    }
                }
            } else {
                report.diagnostics = crate::xml_lint::check(text, ctx.base_url().as_deref(), &[]);
            }
        }
    }
    report.diagnostics.sort_by_key(|d| (d.range.start, d.severity));
    report
}

fn root_name(text: &str) -> Range<usize> {
    let start = text.find('<').map_or(0, |i| i + 1);
    let mut s = start;
    while text[s..].starts_with(['?', '!']) {
        s = match text[s..].find('<') {
            Some(i) => s + i + 1,
            None => return 0..0,
        };
    }
    let end = text[s..].find(|c: char| c.is_whitespace() || c == '>' || c == '/').map_or(text.len(), |i| s + i);
    s..end
}

/// Completion at `cursor`: the range to replace and the proposals.
pub fn complete(kind: Kind, text: &str, cursor: usize, ctx: &Context) -> Option<(Range<usize>, Vec<Proposal>)> {
    match kind {
        Kind::Json | Kind::Yaml => {
            let places = match kind {
                Kind::Json => crate::json_lint::place(text, cursor).into_iter().collect::<Vec<_>>(),
                _ => crate::yaml_lint::places(text, cursor),
            };
            let first = places.first()?;
            let range = first.range.clone();
            // The schema reference is read from the last good parse of the
            // top of the document; an unfinished document still has it.
            let root = match kind {
                Kind::Json => crate::json_lint::parse(text).root,
                _ => crate::yaml_lint::parse(text, &crate::yaml_lint::Rules::default()).documents.into_iter().next(),
            };
            let root = root.or_else(|| schema_only(kind, text));
            let (url, _, _) = json_schema_url(kind, text, root.as_ref(), ctx)?;
            let doc = schemas::json(&url)?;
            let syntax = match kind {
                Kind::Json => Syntax::Json,
                _ => Syntax::Yaml(crate::yaml_lint::indent_at(text, cursor)),
            };
            let mut out = Vec::new();
            for p in &places {
                let root = Sch::new(doc, &url);
                match &p.spot {
                    Spot::Key { existing } => {
                        let colon = match kind {
                            Kind::Json => crate::json_lint::has_colon(text, p.range.end),
                            _ => crate::yaml_lint::has_colon(text, p.range.end),
                        };
                        out.extend(json_schema::key_proposals(&Loader, root, &p.path, existing, &p.prefix, syntax, colon));
                    }
                    Spot::Value => out.extend(json_schema::value_proposals(&Loader, root, &p.path, &p.prefix, syntax)),
                }
            }
            (!out.is_empty()).then_some((range, out))
        }
        Kind::Xml => {
            let place = crate::xsd::place(text, cursor)?;
            let (locations, _) = xml_locations(text, ctx);
            let urls: Vec<String> = locations.iter().map(|l| l.url.clone()).collect();
            let model = (!urls.is_empty()).then(|| schemas::xsd_model(&urls)).flatten();
            let out = crate::xsd::proposals(text, &place, model.as_deref());
            (!out.is_empty()).then_some((place.range, out))
        }
    }
}

/// Just the `$schema` of a document that does not parse (yet).
fn schema_only(kind: Kind, text: &str) -> Option<crate::datadoc::Node> {
    use crate::datadoc::{Member, Node, Value};
    let re = match kind {
        Kind::Json => regex::Regex::new(r#""\$schema"\s*:\s*"([^"]+)""#).ok()?,
        _ => regex::Regex::new(r#"(?m)^\$schema:\s*["']?([^"'\s]+)"#).ok()?,
    };
    let c = re.captures(text)?;
    let m = c.get(1)?;
    let value = Node::new(Value::String(m.as_str().to_owned()), m.range());
    Some(Node::new(Value::Object(vec![Member { key: "$schema".into(), key_range: m.range(), value }]), 0..text.len()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checks_with_local_schema() {
        let dir = std::env::temp_dir().join(format!("qn-assist-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("s.json"), r#"{"type": "object", "properties": {"port": {"type": "integer"}, "mode": {"enum": ["a", "b"]}}}"#).unwrap();
        let ctx = Context { path: Some(dir.join("doc.json")), ..Default::default() };
        let r = check(Kind::Json, r#"{"$schema": "s.json", "port": "x"}"#, &ctx);
        assert_eq!(r.diagnostics.iter().map(|d| d.message.as_str()).collect::<Vec<_>>(), ["Incorrect type: expected integer but found string"]);
        assert_eq!(r.schema.unwrap().1, Origin::Document);
        let yaml = "# yaml-language-server: $schema=s.json\nport: 1\nmode: c\n";
        let r = check(Kind::Yaml, yaml, &Context { path: Some(dir.join("doc.yaml")), ..Default::default() });
        assert_eq!(r.diagnostics.len(), 1);
        let text = "# yaml-language-server: $schema=s.json\nmode: \n";
        let (range, items) = complete(Kind::Yaml, text, text.len() - 1, &Context { path: Some(dir.join("doc.yaml")), ..Default::default() }).unwrap();
        assert_eq!(range, text.len() - 1..text.len() - 1);
        assert_eq!(items.iter().map(|p| p.insert.as_str()).collect::<Vec<_>>(), ["a", "b"]);
        let text = "{\"$schema\": \"s.json\", \"p";
        let (_, items) = complete(Kind::Json, text, text.len(), &Context { path: Some(dir.join("doc.json")), ..Default::default() }).unwrap();
        assert_eq!(items[0].insert, "\"port\": ");
        let missing = check(Kind::Json, r#"{"$schema": "nope.json"}"#, &ctx);
        assert!(missing.diagnostics[0].message.starts_with("Schema could not be loaded"));
        std::fs::remove_dir_all(dir).ok();
    }
}
