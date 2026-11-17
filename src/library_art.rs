//! The art library of canvases: installed Pencil stencil collections and the
//! "OpenClipArt" collection of graphics picked on openclipart.org.
//!
//! Layout below the data folder:
//! - `stencils/<collection id>/package.zip` the collection as downloaded or
//!   imported, `stencils/<collection id>/meta.json` its [`Meta`]
//! - `clipart/<sha256>.svg` saved graphics, `clipart/clipart.json` their
//!   titles ([`Clip`])
//! - `library.json` collections and graphics removed here (for the sync)
//!
//! A Pencil collection is a zip with `Definition.xml` (shapes, shortcuts,
//! scripts) and its icons, bitmaps and fonts; icon bundles ("resource
//! bundles") only list folders of SVG/PNG files. Pages read the files through
//! `qn-library://stencils/<id>/<path>` and `qn-library://clipart/<name>`.
//!
//! With *Sync Stencils and Clip Art* on, the [`Index`] of the library is
//! synced (see `github.rs`); the packages and graphics travel in the shared
//! image store.

use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::io::{Cursor, Read};
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Largest stencil package.
pub const MAX_PACKAGE: usize = 60 * 1024 * 1024;
/// Index of Pencil's stencil repository.
pub const REPOSITORY: &str = "https://raw.githubusercontent.com/evolus/stencils-repository/master/repository.xml";
pub const SCHEME: &str = "qn-library";

fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or_default()
}

fn stencils_dir() -> PathBuf {
    crate::store::data_dir().join("stencils")
}

fn clipart_dir() -> PathBuf {
    crate::store::data_dir().join("clipart")
}

/// Collection ids as folder names.
fn safe_id(id: &str) -> Option<&str> {
    let ok = !id.is_empty() && id.len() <= 120 && id.bytes().all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b)) && !id.starts_with('.');
    ok.then_some(id)
}

// ===== stencil collections ===================================================

/// An installed collection.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(default)]
pub struct Meta {
    pub id: String,
    pub name: String,
    pub version: String,
    /// Where it came from (repository URL or file name).
    pub source: String,
    /// Content name of the package (`<sha256>.zip`).
    pub pkg: String,
    /// When it was installed (seconds since 1970).
    pub added: u64,
}

#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    /// Pencil shapes.
    Shapes,
    /// An icon bundle: folders of SVG/PNG files.
    Icons,
}

/// What a page shows of a collection.
#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct Listing {
    pub id: String,
    pub name: String,
    pub kind: Kind,
    pub description: String,
    pub shapes: Vec<ShapeEntry>,
    /// Icon bundles: package paths of the icons.
    pub icons: Vec<String>,
    /// URL prefix of the package's files.
    pub base: String,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct ShapeEntry {
    pub id: String,
    pub name: String,
    /// URL of the palette icon ("" without one).
    pub icon: String,
    pub shortcut: bool,
}

/// The parsed head of a package.
#[derive(Clone, Debug, PartialEq)]
pub struct Package {
    pub id: String,
    pub name: String,
    pub description: String,
    pub author: String,
    /// Folder of `Definition.xml` inside the zip ("" or "dir/").
    pub root: String,
    /// (id, display name, icon path, shortcut, hidden)
    pub shapes: Vec<(String, String, String, bool, bool)>,
    /// Icon folders of a resource bundle.
    pub resources: Vec<String>,
    /// All file paths below `root`.
    pub files: Vec<String>,
}

/// Reads a Pencil collection zip.
pub fn read_package(data: &[u8]) -> Result<Package, String> {
    let not_pencil = || t!("Not a Pencil stencil collection").to_string();
    let mut zip = zip::ZipArchive::new(Cursor::new(data)).map_err(|_| not_pencil())?;
    let names: Vec<String> = zip.file_names().filter_map(|n| n.ok().map(|n| n.into_owned())).collect();
    let def = names
        .iter()
        .filter(|n| n.rsplit('/').next() == Some("Definition.xml"))
        .min_by_key(|n| n.matches('/').count())
        .cloned()
        .ok_or_else(not_pencil)?;
    let root = def.strip_suffix("Definition.xml").unwrap_or("").to_owned();
    let mut xml = String::new();
    zip.by_name(&def).map_err(|e| e.to_string())?.take(64 * 1024 * 1024).read_to_string(&mut xml).map_err(|e| e.to_string())?;
    let mut package = parse_definition(&xml).ok_or_else(not_pencil)?;
    package.files = names
        .iter()
        .filter(|n| !n.ends_with('/'))
        .filter_map(|n| n.strip_prefix(root.as_str()))
        .map(str::to_owned)
        .collect();
    package.root = root;
    if package.id.is_empty() {
        return Err(not_pencil());
    }
    Ok(package)
}

/// Shapes, shortcuts and resource folders of a `Definition.xml`.
fn parse_definition(xml: &str) -> Option<Package> {
    use quick_xml::events::Event;
    let mut reader = quick_xml::Reader::from_str(xml);
    let mut depth = 0usize;
    let mut package: Option<Package> = None;
    let mut in_script = false;
    let mut scripts = String::new();
    loop {
        let event = reader.read_event().ok()?;
        let (start, empty) = match &event {
            Event::Start(e) => (Some(e.clone()), false),
            Event::Empty(e) => (Some(e.clone()), true),
            _ => (None, false),
        };
        if let Some(e) = start {
            let name = String::from_utf8_lossy(e.local_name().as_ref()).into_owned();
            let attr = |key: &str| -> String {
                e.attributes()
                    .flatten()
                    .find(|a| a.key.local_name().as_ref() == key.as_bytes())
                    .and_then(|a| a.unescape_value().ok().map(|v| v.into_owned()))
                    .unwrap_or_default()
            };
            match (depth, name.as_str()) {
                (0, "Shapes") => {
                    package = Some(Package {
                        id: attr("id"),
                        name: attr("displayName"),
                        description: attr("description"),
                        author: attr("author"),
                        root: String::new(),
                        shapes: Vec::new(),
                        resources: Vec::new(),
                        files: Vec::new(),
                    });
                }
                (1, "Shape") | (1, "Shortcut") => {
                    if let Some(p) = package.as_mut() {
                        let shortcut = name == "Shortcut";
                        let id = if shortcut && attr("id").is_empty() { format!("sc:{}", attr("displayName")) } else { attr("id") };
                        let icon = attr("icon").split('?').next().unwrap_or("").to_owned();
                        p.shapes.push((id, attr("displayName"), icon, shortcut, attr("system") == "true"));
                    }
                }
                (1, "Script") => in_script = !empty,
                _ => {}
            }
            if !empty {
                depth += 1;
            }
            continue;
        }
        match event {
            Event::End(_) => {
                depth = depth.saturating_sub(1);
                if depth == 1 {
                    in_script = false;
                }
            }
            Event::Text(t) if in_script => scripts.push_str(&String::from_utf8_lossy(&t)),
            Event::CData(t) if in_script => scripts.push_str(&String::from_utf8_lossy(&t)),
            Event::Eof => break,
            _ => {}
        }
    }
    let mut package = package?;
    if let Some(i) = scripts.find("RESOURCE_LIST") {
        let re = regex::Regex::new(r#"prefix\s*:\s*["']([^"']+)["']"#).expect("valid regex");
        package.resources = re.captures_iter(&scripts[i..]).map(|c| c[1].trim_matches('/').to_owned()).collect();
    }
    Some(package)
}

fn meta_path(id: &str) -> PathBuf {
    stencils_dir().join(id).join("meta.json")
}

fn package_path(id: &str) -> PathBuf {
    stencils_dir().join(id).join("package.zip")
}

/// Installs (or replaces) a collection.
pub fn install(data: &[u8], source: &str, version: &str) -> Result<Meta, String> {
    if data.len() > MAX_PACKAGE {
        return Err(t!("The stencil collection is larger than 60 MB").into());
    }
    let package = read_package(data)?;
    let id = safe_id(&package.id).ok_or_else(|| t!("Not a Pencil stencil collection").to_string())?.to_owned();
    let meta = Meta {
        id: id.clone(),
        name: if package.name.is_empty() { id.clone() } else { package.name },
        version: version.to_owned(),
        source: source.to_owned(),
        pkg: crate::canvas::asset_name(data, "zip"),
        added: now(),
    };
    let dir = stencils_dir().join(&id);
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    std::fs::write(package_path(&id), data).map_err(|e| e.to_string())?;
    std::fs::write(meta_path(&id), serde_json::to_vec_pretty(&meta).unwrap_or_default()).map_err(|e| e.to_string())?;
    ZIPS.with(|z| z.borrow_mut().remove(&id));
    forget_removal(&format!("st:{id}"));
    Ok(meta)
}

/// The installed collections, by name.
pub fn installed() -> Vec<Meta> {
    let Ok(entries) = std::fs::read_dir(stencils_dir()) else { return Vec::new() };
    let mut out: Vec<Meta> = entries
        .flatten()
        .filter_map(|e| std::fs::read(e.path().join("meta.json")).ok())
        .filter_map(|data| serde_json::from_slice::<Meta>(&data).ok())
        .filter(|m| safe_id(&m.id).is_some() && package_path(&m.id).is_file())
        .collect();
    out.sort_by_key(|m| m.name.to_lowercase());
    out
}

pub fn remove(id: &str) {
    if let Some(id) = safe_id(id) {
        let _ = std::fs::remove_dir_all(stencils_dir().join(id));
        ZIPS.with(|z| z.borrow_mut().remove(id));
        record_removal(&format!("st:{id}"));
    }
}

/// The package bytes of a collection.
pub fn package_data(id: &str) -> Option<Vec<u8>> {
    std::fs::read(package_path(safe_id(id)?)).ok()
}

/// What the canvas page shows of an installed collection.
pub fn listing(meta: &Meta) -> Option<Listing> {
    let data = package_data(&meta.id)?;
    let package = read_package(&data).ok()?;
    let base = format!("{SCHEME}://stencils/{}/", meta.id);
    let shapes: Vec<ShapeEntry> = package
        .shapes
        .iter()
        .filter(|(_, _, _, _, hidden)| !hidden)
        .map(|(id, name, icon, shortcut, _)| ShapeEntry {
            id: id.clone(),
            name: if name.is_empty() { id.clone() } else { name.clone() },
            icon: if icon.is_empty() { String::new() } else { format!("{base}{icon}") },
            shortcut: *shortcut,
        })
        .collect();
    let icons: Vec<String> = if shapes.is_empty() {
        let prefixes: Vec<String> = package.resources.iter().map(|p| format!("{p}/")).collect();
        package
            .files
            .iter()
            .filter(|f| prefixes.iter().any(|p| f.starts_with(p.as_str())))
            .filter(|f| f.ends_with(".svg") || f.ends_with(".png"))
            .cloned()
            .collect()
    } else {
        Vec::new()
    };
    let kind = if shapes.is_empty() && !icons.is_empty() { Kind::Icons } else { Kind::Shapes };
    Some(Listing { id: meta.id.clone(), name: meta.name.clone(), kind, description: package.description, shapes, icons, base })
}

thread_local! {
    /// Opened packages (collection id → zip), for the URI scheme.
    static ZIPS: RefCell<HashMap<String, zip::ZipArchive<Cursor<Vec<u8>>>>> = RefCell::new(HashMap::new());
}

/// A file of an installed collection (`path` relative to `Definition.xml`).
pub fn package_file(id: &str, path: &str) -> Option<Vec<u8>> {
    let id = safe_id(id)?;
    if path.split('/').any(|s| s == "..") {
        return None;
    }
    ZIPS.with(|zips| {
        let mut zips = zips.borrow_mut();
        if !zips.contains_key(id) {
            let zip = zip::ZipArchive::new(Cursor::new(package_data(id)?)).ok()?;
            zips.insert(id.to_owned(), zip);
        }
        let zip = zips.get_mut(id)?;
        let root = zip
            .file_names()
            .filter_map(|n| n.ok().map(|n| n.into_owned()))
            .filter(|n| n.rsplit('/').next() == Some("Definition.xml"))
            .min_by_key(|n| n.matches('/').count())
            .map(|n| n.trim_end_matches("Definition.xml").to_owned())
            .unwrap_or_default();
        let mut file = zip.by_name(&format!("{root}{path}")).ok()?;
        let mut data = Vec::new();
        file.read_to_end(&mut data).ok()?;
        Some(data)
    })
}

/// MIME type of a package or clip-art file.
pub fn mime_type(path: &str) -> &'static str {
    match path.rsplit('.').next().unwrap_or("").to_ascii_lowercase().as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "svg" => "image/svg+xml",
        "ttf" => "font/ttf",
        "otf" => "font/otf",
        "woff" => "font/woff",
        "woff2" => "font/woff2",
        "xml" => "application/xml",
        "css" => "text/css",
        _ => "application/octet-stream",
    }
}

// ===== Pencil's repository ===================================================

/// A collection of Pencil's stencil repository.
#[derive(Serialize, Clone, Debug, PartialEq, Default)]
pub struct RepoItem {
    pub id: String,
    pub name: String,
    pub description: String,
    pub author: String,
    pub license: String,
    pub version: String,
    pub url: String,
    pub thumb: String,
    pub installed: bool,
    pub update: bool,
}

fn agent() -> ureq::Agent {
    ureq::AgentBuilder::new().timeout(Duration::from_secs(120)).user_agent("QuickNotes").build()
}

/// Loads the repository index (blocking).
pub fn repository() -> Result<Vec<RepoItem>, String> {
    let xml = agent().get(REPOSITORY).call().map_err(|e| e.to_string())?.into_string().map_err(|e| e.to_string())?;
    Ok(parse_repository(&xml))
}

pub fn parse_repository(xml: &str) -> Vec<RepoItem> {
    let block = regex::Regex::new(r"(?is)<collection\b[^>]*>(.*?)</collection>").expect("valid regex");
    let field = |body: &str, tag: &str| -> String {
        let re = regex::Regex::new(&format!(r"(?is)<{tag}>(.*?)</{tag}>")).expect("valid regex");
        re.captures(body).map(|c| xml_unescape(c[1].trim())).unwrap_or_default()
    };
    block
        .captures_iter(xml)
        .map(|c| {
            let body = &c[1];
            RepoItem {
                id: field(body, "id"),
                name: field(body, "displayName"),
                description: field(body, "description"),
                author: field(body, "author"),
                license: field(body, "license"),
                version: field(body, "version"),
                url: field(body, "url"),
                thumb: field(body, "thumbnail"),
                installed: false,
                update: false,
            }
        })
        .filter(|i| !i.id.is_empty() && i.url.starts_with("https://"))
        .collect()
}

fn xml_unescape(s: &str) -> String {
    s.replace("&lt;", "<").replace("&gt;", ">").replace("&quot;", "\"").replace("&apos;", "'").replace("&amp;", "&")
}

/// Marks what is installed (and older than the repository's version).
pub fn mark_installed(items: &mut [RepoItem]) {
    let installed = installed();
    for item in items {
        if let Some(m) = installed.iter().find(|m| m.id == item.id) {
            item.installed = true;
            item.update = !item.version.is_empty() && m.version != item.version;
        }
    }
}

/// Downloads a package (blocking).
pub fn download(url: &str) -> Result<Vec<u8>, String> {
    if !url.starts_with("https://") {
        return Err("invalid URL".into());
    }
    let response = agent().get(url).call().map_err(|e| e.to_string())?;
    let mut data = Vec::new();
    response.into_reader().take(MAX_PACKAGE as u64 + 1).read_to_end(&mut data).map_err(|e| e.to_string())?;
    if data.len() > MAX_PACKAGE {
        return Err(t!("The stencil collection is larger than 60 MB").into());
    }
    Ok(data)
}

// ===== clip art =================================================================

/// A saved graphic.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(default)]
pub struct Clip {
    /// OpenClipart id.
    pub id: String,
    pub title: String,
    /// File name (`<sha256>.svg`).
    pub name: String,
    pub added: u64,
}

fn clipart_index() -> PathBuf {
    clipart_dir().join("clipart.json")
}

pub fn clipart() -> Vec<Clip> {
    std::fs::read(clipart_index())
        .ok()
        .and_then(|d| serde_json::from_slice::<Vec<Clip>>(&d).ok())
        .unwrap_or_default()
        .into_iter()
        .filter(|c| crate::canvas::valid_name(&c.name) && clipart_dir().join(&c.name).is_file())
        .collect()
}

fn save_clipart(items: &[Clip]) -> Result<(), String> {
    std::fs::create_dir_all(clipart_dir()).map_err(|e| e.to_string())?;
    std::fs::write(clipart_index(), serde_json::to_vec_pretty(items).unwrap_or_default()).map_err(|e| e.to_string())
}

/// Saves a graphic in the collection; returns its file name.
pub fn add_clipart(id: &str, title: &str, svg: &[u8]) -> Result<String, String> {
    let name = crate::canvas::asset_name(svg, "svg");
    std::fs::create_dir_all(clipart_dir()).map_err(|e| e.to_string())?;
    let path = clipart_dir().join(&name);
    if !path.is_file() {
        std::fs::write(&path, svg).map_err(|e| e.to_string())?;
    }
    let mut items = clipart();
    items.retain(|c| c.id != id);
    items.push(Clip { id: id.to_owned(), title: title.to_owned(), name: name.clone(), added: now() });
    save_clipart(&items)?;
    forget_removal(&format!("oc:{id}"));
    Ok(name)
}

pub fn remove_clipart(id: &str) {
    let mut items = clipart();
    let gone: Vec<Clip> = items.iter().filter(|c| c.id == id).cloned().collect();
    items.retain(|c| c.id != id);
    let _ = save_clipart(&items);
    for c in gone.iter().filter(|c| !items.iter().any(|i| i.name == c.name)) {
        let _ = std::fs::remove_file(clipart_dir().join(&c.name));
    }
    record_removal(&format!("oc:{id}"));
}

/// A saved graphic's file.
pub fn clipart_file(name: &str) -> Option<Vec<u8>> {
    if !crate::canvas::valid_name(name) {
        return None;
    }
    std::fs::read(clipart_dir().join(name)).ok()
}

// ===== sync =======================================================================

/// The library as it is synced (`quicknotes/library.enc`).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(default)]
pub struct Index {
    pub collections: Vec<Meta>,
    pub clipart: Vec<Clip>,
    /// Removed entries (`st:<id>`, `oc:<id>`) and when.
    pub removed: BTreeMap<String, u64>,
}

impl Index {
    /// Content names of the packages and graphics (in the image store).
    pub fn files(&self) -> Vec<String> {
        self.collections.iter().map(|c| c.pkg.clone()).chain(self.clipart.iter().map(|c| c.name.clone())).collect()
    }
}

fn removals_path() -> PathBuf {
    crate::store::data_dir().join("library.json")
}

fn removals() -> BTreeMap<String, u64> {
    std::fs::read(removals_path()).ok().and_then(|d| serde_json::from_slice(&d).ok()).unwrap_or_default()
}

fn save_removals(r: &BTreeMap<String, u64>) {
    let _ = std::fs::write(removals_path(), serde_json::to_vec(r).unwrap_or_default());
}

fn record_removal(key: &str) {
    let mut r = removals();
    r.insert(key.to_owned(), now());
    save_removals(&r);
}

fn forget_removal(key: &str) {
    let mut r = removals();
    if r.remove(key).is_some() {
        save_removals(&r);
    }
}

/// The local library.
pub fn local_index() -> Index {
    Index { collections: installed(), clipart: clipart(), removed: removals() }
}

/// Merges two libraries: everything of both, newer entries win, removals win
/// over older entries.
pub fn merge(a: &Index, b: &Index) -> Index {
    let mut removed = a.removed.clone();
    for (k, t) in &b.removed {
        let e = removed.entry(k.clone()).or_insert(*t);
        *e = (*e).max(*t);
    }
    let gone = |key: String, added: u64| removed.get(&key).is_some_and(|t| *t >= added);
    let mut collections: BTreeMap<String, Meta> = BTreeMap::new();
    for m in a.collections.iter().chain(&b.collections) {
        if collections.get(&m.id).is_none_or(|old| old.added < m.added) {
            collections.insert(m.id.clone(), m.clone());
        }
    }
    let mut clipart: BTreeMap<String, Clip> = BTreeMap::new();
    for c in a.clipart.iter().chain(&b.clipart) {
        if clipart.get(&c.id).is_none_or(|old| old.added < c.added) {
            clipart.insert(c.id.clone(), c.clone());
        }
    }
    Index {
        collections: collections.into_values().filter(|m| !gone(format!("st:{}", m.id), m.added)).collect(),
        clipart: clipart.into_values().filter(|c| !gone(format!("oc:{}", c.id), c.added)).collect(),
        removed,
    }
}

/// Files of the local library by content name (for uploads).
pub fn local_file(name: &str) -> Option<Vec<u8>> {
    if let Some(m) = installed().into_iter().find(|m| m.pkg == name) {
        return package_data(&m.id);
    }
    clipart_file(name)
}

/// Brings the local library to `index`; `files` are fetched packages and
/// graphics by content name. Returns whether anything changed.
pub fn apply(index: &Index, files: &HashMap<String, Vec<u8>>) -> bool {
    let local = local_index();
    let mut changed = false;
    for m in &local.collections {
        if !index.collections.iter().any(|n| n.id == m.id) {
            remove(&m.id);
            changed = true;
        }
    }
    for m in &index.collections {
        let have = local.collections.iter().any(|l| l.id == m.id && l.pkg == m.pkg);
        if have {
            continue;
        }
        let Some(data) = files.get(&m.pkg).cloned().or_else(|| local_file(&m.pkg)) else { continue };
        if crate::canvas::asset_name(&data, "zip") != m.pkg {
            continue;
        }
        if install(&data, &m.source, &m.version).is_ok() {
            // Keep the synced metadata (time stamps).
            let _ = std::fs::write(meta_path(&m.id), serde_json::to_vec_pretty(m).unwrap_or_default());
            changed = true;
        }
    }
    let mut items: Vec<Clip> = Vec::new();
    for c in &index.clipart {
        let path = clipart_dir().join(&c.name);
        if !path.is_file() {
            match files.get(&c.name) {
                Some(data) if crate::canvas::asset_name(data, "svg") == c.name => {
                    let _ = std::fs::create_dir_all(clipart_dir());
                    if std::fs::write(&path, data).is_err() {
                        continue;
                    }
                }
                _ => continue,
            }
        }
        items.push(c.clone());
    }
    if items != local.clipart {
        for c in local.clipart.iter().filter(|c| !items.iter().any(|i| i.name == c.name)) {
            let _ = std::fs::remove_file(clipart_dir().join(&c.name));
        }
        let _ = save_clipart(&items);
        changed = true;
    }
    if index.removed != local.removed {
        save_removals(&index.removed);
    }
    changed
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub(crate) fn package_zip(id: &str, def: &str, extra: &[(&str, &[u8])]) -> Vec<u8> {
        use std::io::Write;
        let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
        let opts = zip::write::SimpleFileOptions::default();
        zip.start_file(format!("{id}/Definition.xml"), opts).unwrap();
        zip.write_all(def.as_bytes()).unwrap();
        for (name, data) in extra {
            zip.start_file(format!("{id}/{name}"), opts).unwrap();
            zip.write_all(data).unwrap();
        }
        zip.finish().unwrap().into_inner()
    }

    pub(crate) const SHAPES: &str = r#"<Shapes xmlns="http://www.evolus.vn/Namespace/Pencil" id="test.Wire" displayName="Test &amp; Wire" description="d">
  <Script comments="x"><![CDATA[ collection.X = 1; ]]></Script>
  <Shape id="button" displayName="Button" icon="icons/button.png?token=1"><Properties/><p:Content xmlns:p="http://www.evolus.vn/Namespace/Pencil"><rect id="r"/></p:Content></Shape>
  <Shape id="base" displayName="Base" system="true"/>
  <Shortcut to="button" displayName="Big button" icon="icons/big.png"><PropertyValue name="box">200,40</PropertyValue></Shortcut>
</Shapes>"#;

    const ICONS: &str = r#"<Shapes xmlns="http://www.evolus.vn/Namespace/Pencil" id="icons.Test" displayName="Icons">
  <Script comments="Resource script"><![CDATA[
collection.RESOURCE_LIST = [ {name: "<All>", prefix: "icons", type: "svg"} ];
]]></Script>
</Shapes>"#;

    #[test]
    fn packages_are_read() {
        let p = read_package(&package_zip("Wire", SHAPES, &[("icons/button.png", b"png")])).unwrap();
        assert_eq!((p.id.as_str(), p.name.as_str(), p.root.as_str()), ("test.Wire", "Test & Wire", "Wire/"));
        assert_eq!(p.shapes.len(), 3);
        assert_eq!(p.shapes[0], ("button".into(), "Button".into(), "icons/button.png".into(), false, false));
        assert!(p.shapes[1].4, "system shapes are hidden");
        assert_eq!(p.shapes[2].0, "sc:Big button");
        assert!(p.files.contains(&"icons/button.png".to_owned()));
        let i = read_package(&package_zip("I", ICONS, &[("icons/a-b.svg", b"<svg/>")])).unwrap();
        assert_eq!(i.resources, ["icons"]);
        assert!(read_package(b"no zip").is_err());
    }

    #[test]
    fn installing_listing_and_removing() {
        let dir = crate::store::tests::temp_data_dir("stencils");
        let meta = install(&package_zip("Wire", SHAPES, &[("icons/button.png", b"png")]), "file.zip", "1.0").unwrap();
        assert_eq!(meta.id, "test.Wire");
        assert!(meta.pkg.ends_with(".zip") && crate::canvas::valid_name(&meta.pkg));
        assert_eq!(installed(), [meta.clone()]);
        let l = listing(&meta).unwrap();
        assert_eq!(l.kind, Kind::Shapes);
        assert_eq!(l.shapes.len(), 2);
        assert_eq!(l.shapes[0].icon, "qn-library://stencils/test.Wire/icons/button.png");
        assert_eq!(package_file("test.Wire", "icons/button.png").as_deref(), Some(&b"png"[..]));
        assert_eq!(package_file("test.Wire", "../meta.json"), None);

        let icons = install(&package_zip("I", ICONS, &[("icons/a-b.svg", b"<svg/>"), ("other/x.svg", b"<svg/>")]), "", "").unwrap();
        let l = listing(&icons).unwrap();
        assert_eq!((l.kind, l.icons.clone()), (Kind::Icons, vec!["icons/a-b.svg".to_owned()]));

        remove("test.Wire");
        assert_eq!(installed().len(), 1);
        assert!(local_index().removed.contains_key("st:test.Wire"));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn clip_art_collection() {
        let dir = crate::store::tests::temp_data_dir("clipart");
        let name = add_clipart("42", "Cat", b"<svg/>").unwrap();
        assert_eq!(clipart().len(), 1);
        assert_eq!(clipart_file(&name).as_deref(), Some(&b"<svg/>"[..]));
        remove_clipart("42");
        assert!(clipart().is_empty() && clipart_file(&name).is_none());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn libraries_merge() {
        let m = |id: &str, added| Meta { id: id.into(), added, pkg: format!("{id}.zip"), ..Default::default() };
        let c = |id: &str, added| Clip { id: id.into(), added, name: format!("{id}.svg"), ..Default::default() };
        let a = Index { collections: vec![m("x", 5), m("y", 1)], clipart: vec![c("1", 3)], removed: BTreeMap::from([("oc:2".into(), 9)]) };
        let b = Index { collections: vec![m("x", 7), m("z", 2)], clipart: vec![c("2", 4)], removed: BTreeMap::from([("st:y".into(), 2)]) };
        let out = merge(&a, &b);
        assert_eq!(out.collections.iter().map(|m| (m.id.as_str(), m.added)).collect::<Vec<_>>(), [("x", 7), ("z", 2)]);
        assert_eq!(out.clipart.iter().map(|c| c.id.as_str()).collect::<Vec<_>>(), ["1"], "removed later than added");
        assert_eq!(merge(&b, &a), out);
        // Added again after the removal: it stays.
        let again = Index { collections: vec![m("y", 3)], ..Default::default() };
        assert_eq!(merge(&out, &again).collections.len(), 3);
    }

    #[test]
    fn repository_index() {
        let xml = r#"<Collections><Collection required-version="4.0.0"><id>a.b</id><displayName>A &amp; B</displayName>
            <version>1.0</version><url>https://example.org/a.zip</url><thumbnail>https://example.org/a.png</thumbnail></Collection>
            <collection><id>c</id><displayName>C</displayName><url>http://insecure/c.zip</url></collection>
            <collection><id>d</id><displayName>D</displayName><url>https://example.org/d.zip</url></collection></Collections>"#;
        let items = parse_repository(xml);
        assert_eq!(items.iter().map(|i| i.id.as_str()).collect::<Vec<_>>(), ["a.b", "d"]);
        assert_eq!(items[0].name, "A & B");
    }
}
