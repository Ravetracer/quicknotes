//! EPUB 3 export of Markdown and reStructuredText documents.
//!
//! The document is rendered like the HTML export; [`EXTRACT_JS`] then reads
//! the finished page (charts as SVG, typeset formulas and tunes) and splits
//! it into chapters at the top headings. [`build`] packs the chapters, a
//! table of contents, the styles and the document's images into the EPUB.

use serde::Deserialize;
use std::io::Write;
use std::path::Path;

/// Book metadata, from the front matter (`title`, `author`, `language`,
/// `date`, `description`, `publisher`, `cover`).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Meta {
    pub title: String,
    pub author: Option<String>,
    pub language: String,
    pub date: Option<String>,
    pub description: Option<String>,
    pub publisher: Option<String>,
    /// Path or URL of the cover image.
    pub cover: Option<String>,
}

impl Meta {
    pub fn from_source(source: &str, fallback_title: &str) -> Meta {
        let lines: Vec<&str> = source.lines().take(200).collect();
        let front = crate::slides::front_matter(&lines).0.unwrap_or_default();
        let get = |keys: &[&str]| {
            keys.iter().find_map(|k| match front.get(*k)? {
                serde_yaml::Value::String(s) => Some(s.trim().to_owned()),
                serde_yaml::Value::Number(n) => Some(n.to_string()),
                serde_yaml::Value::Sequence(seq) => {
                    let names: Vec<String> = seq.iter().filter_map(|v| v.as_str().map(str::to_owned)).collect();
                    (!names.is_empty()).then(|| names.join(", "))
                }
                _ => None,
            })
        };
        let language = get(&["language", "lang"]).unwrap_or_else(|| match crate::i18n::lang() {
            crate::i18n::Lang::De => "de".into(),
            crate::i18n::Lang::En => "en".into(),
        });
        Meta {
            title: get(&["title"]).filter(|t| !t.is_empty()).unwrap_or_else(|| fallback_title.to_owned()),
            author: get(&["author", "authors", "creator"]),
            language,
            date: get(&["date"]),
            description: get(&["description", "subtitle"]),
            publisher: get(&["publisher"]),
            cover: get(&["cover", "cover-image", "cover_image"]),
        }
    }
}

/// Script run on the rendered export page. Returns JSON: the chapters (XHTML
/// and headings), the local images it refers to, and whether it has
/// formulas.
pub const EXTRACT_JS: &str = r##"
(function () {
  var root = document.getElementById('content');
  // Charts: the drawn SVG, scalable.
  root.querySelectorAll('div.qn-chart').forEach(function (div) {
    var svg = div.querySelector('svg');
    if (!svg) return;
    svg = svg.cloneNode(true);
    var w = parseFloat(svg.getAttribute('width')), h = parseFloat(svg.getAttribute('height'));
    if (!svg.getAttribute('viewBox') && w && h) svg.setAttribute('viewBox', '0 0 ' + w + ' ' + h);
    svg.removeAttribute('style');
    svg.setAttribute('width', '100%');
    svg.removeAttribute('height');
    var box = document.createElement('div');
    box.className = 'qn-chart';
    box.appendChild(svg);
    div.replaceWith(box);
  });
  var n = 0;
  root.querySelectorAll('h1,h2,h3,h4,h5,h6').forEach(function (h) { if (!h.id) h.id = 'qn-h' + (++n); });
  var images = [];
  root.querySelectorAll('img').forEach(function (img) {
    var src = img.getAttribute('src') || '';
    if (!src || /^(data:|https?:)/i.test(src)) return;
    var i = images.indexOf(src);
    if (i < 0) { i = images.length; images.push(src); }
    img.setAttribute('src', 'qn-image:' + i);
    if (!img.hasAttribute('alt')) img.setAttribute('alt', '');
  });
  // Chapters at # headings, or ## when there is at most one #.
  var kids = Array.prototype.slice.call(root.children);
  var count = function (tag) { return kids.filter(function (el) { return el.tagName === tag; }).length; };
  var level = count('H1') >= 2 ? 'H1' : (count('H2') >= 1 ? 'H2' : null);
  var chapters = [];
  kids.forEach(function (el) {
    if (!chapters.length || (level && el.tagName === level && chapters[chapters.length - 1].length)) chapters.push([]);
    chapters[chapters.length - 1].push(el);
  });
  var where = {};
  chapters.forEach(function (nodes, i) {
    nodes.forEach(function (el) {
      if (el.id) where[el.id] = i;
      el.querySelectorAll('[id]').forEach(function (x) { where[x.id] = i; });
    });
  });
  root.querySelectorAll('a[href^="#"]').forEach(function (a) {
    var id = a.getAttribute('href').slice(1);
    if (id in where) a.setAttribute('href', 'chapter-' + (where[id] + 1) + '.xhtml#' + id);
  });
  var ser = new XMLSerializer();
  var out = chapters.map(function (nodes) {
    var headings = [];
    nodes.forEach(function (el) {
      var list = /^H[1-3]$/.test(el.tagName) ? [el] : Array.prototype.slice.call(el.querySelectorAll('h1,h2,h3'));
      list.forEach(function (h) { headings.push({ level: +h.tagName[1], id: h.id, text: h.textContent.trim() }); });
    });
    return { headings: headings, html: nodes.map(function (el) { return ser.serializeToString(el); }).join('\n') };
  });
  return JSON.stringify({ chapters: out, images: images, math: !!root.querySelector('.katex') });
})()
"##;

#[derive(Debug, Deserialize, Default)]
pub struct Extract {
    pub chapters: Vec<Chapter>,
    pub images: Vec<String>,
    #[serde(default)]
    pub math: bool,
}

#[derive(Debug, Deserialize, Default)]
pub struct Chapter {
    pub headings: Vec<Heading>,
    pub html: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Heading {
    pub level: u8,
    pub id: String,
    pub text: String,
}

/// A resource of the book.
struct Item {
    id: String,
    href: String,
    media_type: &'static str,
    properties: Vec<&'static str>,
    data: Vec<u8>,
}

fn media_type(path: &str) -> Option<&'static str> {
    let ext = path.rsplit('.').next()?.to_ascii_lowercase();
    Some(match ext.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "svg" => "image/svg+xml",
        "webp" => "image/webp",
        _ => return None,
    })
}

/// Reads an image the document refers to (relative to `base_dir`).
fn read_image(src: &str, base_dir: Option<&Path>) -> Option<(Vec<u8>, &'static str)> {
    let path = match src.strip_prefix("file://") {
        Some(p) => urlencoding::decode(p).ok()?.into_owned().into(),
        None => {
            let src = urlencoding::decode(src).map(|s| s.into_owned()).unwrap_or_else(|_| src.to_owned());
            let p = std::path::PathBuf::from(src.strip_prefix("~/").map_or(src.clone(), |rest| {
                dirs::home_dir().map(|h| h.join(rest).to_string_lossy().into_owned()).unwrap_or(src.clone())
            }));
            if p.is_absolute() {
                p
            } else {
                base_dir?.join(p)
            }
        }
    };
    let media = media_type(&path.to_string_lossy())?;
    Some((std::fs::read(&path).ok()?, media))
}

pub fn escape(text: &str) -> String {
    text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

/// The EPUB file.
pub fn build(meta: &Meta, extract: Extract, base_dir: Option<&Path>, css: &str, math_css: &str) -> Result<Vec<u8>, String> {
    let mut items: Vec<Item> = Vec::new();
    // Images, with their place-holder `qn-image:N` replaced.
    let mut image_hrefs: Vec<Option<String>> = Vec::new();
    for (i, src) in extract.images.iter().enumerate() {
        match read_image(src, base_dir) {
            Some((data, media)) => {
                let ext = src.rsplit('.').next().unwrap_or("img").to_ascii_lowercase();
                let href = format!("images/image-{i}.{ext}");
                items.push(Item { id: format!("image-{i}"), href: href.clone(), media_type: media, properties: vec![], data });
                image_hrefs.push(Some(href));
            }
            None => image_hrefs.push(None),
        }
    }
    let fix_images = |html: &str| -> String {
        let mut out = html.to_owned();
        for (i, (href, src)) in image_hrefs.iter().zip(&extract.images).enumerate().rev() {
            let target = href.clone().unwrap_or_else(|| escape(src));
            out = out.replace(&format!("\"qn-image:{i}\""), &format!("\"{target}\""));
        }
        out
    };
    let cover = meta.cover.as_deref().and_then(|c| read_image(c, base_dir).map(|(data, media)| (c, data, media)));
    if let Some((src, data, media)) = cover {
        let ext = src.rsplit('.').next().unwrap_or("img").to_ascii_lowercase();
        items.push(Item {
            id: "cover-image".into(),
            href: format!("images/cover.{ext}"),
            media_type: media,
            properties: vec!["cover-image"],
            data,
        });
    }
    let has_cover = items.iter().any(|i| i.id == "cover-image");

    let lang = escape(&meta.language);
    let title = escape(&meta.title);
    let page = |title: &str, body: &str, math: bool| -> String {
        let math_link = if math { "\n<link rel=\"stylesheet\" type=\"text/css\" href=\"katex.css\"/>" } else { "" };
        format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!DOCTYPE html>\n<html xmlns=\"http://www.w3.org/1999/xhtml\" xmlns:epub=\"http://www.idpf.org/2007/ops\" xml:lang=\"{lang}\" lang=\"{lang}\">\n<head>\n<meta charset=\"UTF-8\"/>\n<title>{title}</title>\n<link rel=\"stylesheet\" type=\"text/css\" href=\"style.css\"/>{math_link}\n</head>\n<body>\n{body}\n</body>\n</html>\n"
        )
    };

    let mut spine: Vec<String> = Vec::new();
    if let Some(cover) = items.iter().find(|i| i.id == "cover-image") {
        let body = format!("<div class=\"cover\"><img src=\"{}\" alt=\"{title}\"/></div>", cover.href);
        items.push(Item {
            id: "cover".into(),
            href: "cover.xhtml".into(),
            media_type: "application/xhtml+xml",
            properties: vec![],
            data: page(&title, &body, false).into_bytes(),
        });
        spine.push("cover".into());
    }
    let chapters: Vec<Chapter> = if extract.chapters.is_empty() { vec![Chapter::default()] } else { extract.chapters };
    let mut toc: Vec<(String, Vec<Heading>)> = Vec::new();
    for (i, chapter) in chapters.iter().enumerate() {
        let href = format!("chapter-{}.xhtml", i + 1);
        let html = fix_images(&chapter.html);
        let mut properties = Vec::new();
        if html.contains("<svg") {
            properties.push("svg");
        }
        if html.contains("<math") {
            properties.push("mathml");
        }
        let chapter_title = chapter.headings.first().map_or(meta.title.as_str(), |h| h.text.as_str());
        let body = format!("<main id=\"content\">\n{html}\n</main>");
        items.push(Item {
            id: format!("chapter-{}", i + 1),
            href: href.clone(),
            media_type: "application/xhtml+xml",
            properties,
            data: page(&escape(chapter_title), &body, extract.math).into_bytes(),
        });
        spine.push(format!("chapter-{}", i + 1));
        toc.push((href, chapter.headings.clone()));
    }

    // Table of contents: chapters with the headings one level below.
    let mut nav = String::new();
    let mut ncx = String::new();
    let mut order = 0;
    for (i, (href, headings)) in toc.iter().enumerate() {
        let top = headings.iter().map(|h| h.level).min().unwrap_or(1);
        let (label, anchor) = match headings.iter().find(|h| h.level == top) {
            Some(h) => (h.text.clone(), format!("{href}#{}", h.id)),
            None if i == 0 => (meta.title.clone(), href.clone()),
            None => (format!("{} {}", meta.title, i + 1), href.clone()),
        };
        order += 1;
        nav.push_str(&format!("<li><a href=\"{}\">{}</a>", escape(&anchor), escape(&label)));
        ncx.push_str(&format!(
            "<navPoint id=\"np{order}\" playOrder=\"{order}\"><navLabel><text>{}</text></navLabel><content src=\"{}\"/>",
            escape(&label),
            escape(&anchor)
        ));
        let subs: Vec<&Heading> = headings.iter().filter(|h| h.level == top + 1).collect();
        if !subs.is_empty() {
            nav.push_str("<ol>");
            for h in subs {
                order += 1;
                let anchor = escape(&format!("{href}#{}", h.id));
                nav.push_str(&format!("<li><a href=\"{anchor}\">{}</a></li>", escape(&h.text)));
                ncx.push_str(&format!(
                    "<navPoint id=\"np{order}\" playOrder=\"{order}\"><navLabel><text>{}</text></navLabel><content src=\"{anchor}\"/></navPoint>",
                    escape(&h.text)
                ));
            }
            nav.push_str("</ol>");
        }
        nav.push_str("</li>\n");
        ncx.push_str("</navPoint>\n");
    }
    let contents = escape(t!("Contents"));
    let nav_body = format!("<nav epub:type=\"toc\" id=\"toc\">\n<h1>{contents}</h1>\n<ol>\n{nav}</ol>\n</nav>");
    items.push(Item {
        id: "nav".into(),
        href: "nav.xhtml".into(),
        media_type: "application/xhtml+xml",
        properties: vec!["nav"],
        data: page(&contents, &nav_body, false).into_bytes(),
    });
    let uid = uuid();
    items.push(Item {
        id: "ncx".into(),
        href: "toc.ncx".into(),
        media_type: "application/x-dtbncx+xml",
        properties: vec![],
        data: format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<ncx xmlns=\"http://www.daisy.org/z3986/2005/ncx/\" version=\"2005-1\">\n<head><meta name=\"dtb:uid\" content=\"urn:uuid:{uid}\"/></head>\n<docTitle><text>{title}</text></docTitle>\n<navMap>\n{ncx}</navMap>\n</ncx>\n"
        )
        .into_bytes(),
    });
    items.push(Item { id: "style".into(), href: "style.css".into(), media_type: "text/css", properties: vec![], data: css.as_bytes().to_vec() });
    if extract.math {
        items.push(Item { id: "katex".into(), href: "katex.css".into(), media_type: "text/css", properties: vec![], data: math_css.as_bytes().to_vec() });
    }

    let mut metadata = format!(
        "<dc:identifier id=\"uid\">urn:uuid:{uid}</dc:identifier>\n<dc:title>{title}</dc:title>\n<dc:language>{lang}</dc:language>\n<meta property=\"dcterms:modified\">{}</meta>\n",
        now_utc()
    );
    for (tag, value) in [("creator", &meta.author), ("date", &meta.date), ("description", &meta.description), ("publisher", &meta.publisher)] {
        if let Some(v) = value {
            metadata.push_str(&format!("<dc:{tag}>{}</dc:{tag}>\n", escape(v)));
        }
    }
    if has_cover {
        metadata.push_str("<meta name=\"cover\" content=\"cover-image\"/>\n");
    }
    let manifest: String = items
        .iter()
        .map(|i| {
            let props = if i.properties.is_empty() { String::new() } else { format!(" properties=\"{}\"", i.properties.join(" ")) };
            format!("<item id=\"{}\" href=\"{}\" media-type=\"{}\"{props}/>\n", i.id, escape(&i.href), i.media_type)
        })
        .collect();
    let spine: String = spine.iter().map(|id| format!("<itemref idref=\"{id}\"/>\n")).collect();
    let opf = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<package xmlns=\"http://www.idpf.org/2007/opf\" version=\"3.0\" unique-identifier=\"uid\" xml:lang=\"{lang}\">\n<metadata xmlns:dc=\"http://purl.org/dc/elements/1.1/\">\n{metadata}</metadata>\n<manifest>\n{manifest}</manifest>\n<spine toc=\"ncx\">\n{spine}</spine>\n</package>\n"
    );

    zip(&opf, &items).map_err(|e| e.to_string())
}

const CONTAINER: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<container version=\"1.0\" xmlns=\"urn:oasis:names:tc:opendocument:xmlns:container\">\n<rootfiles><rootfile full-path=\"OEBPS/content.opf\" media-type=\"application/oebps-package+xml\"/></rootfiles>\n</container>\n";

fn zip(opf: &str, items: &[Item]) -> zip::result::ZipResult<Vec<u8>> {
    use zip::write::SimpleFileOptions;
    use zip::CompressionMethod;
    let mut zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    // The mimetype comes first and uncompressed.
    zip.start_file("mimetype", SimpleFileOptions::default().compression_method(CompressionMethod::Stored))?;
    zip.write_all(b"application/epub+zip")?;
    let deflated = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    zip.start_file("META-INF/container.xml", deflated)?;
    zip.write_all(CONTAINER.as_bytes())?;
    zip.start_file("OEBPS/content.opf", deflated)?;
    zip.write_all(opf.as_bytes())?;
    for item in items {
        zip.start_file(format!("OEBPS/{}", item.href), deflated)?;
        zip.write_all(&item.data)?;
    }
    Ok(zip.finish()?.into_inner())
}

/// A random UUID (version 4).
fn uuid() -> String {
    let mut b = [0u8; 16];
    getrandom::getrandom(&mut b).expect("system random number generator");
    b[6] = (b[6] & 0x0f) | 0x40;
    b[8] = (b[8] & 0x3f) | 0x80;
    let h: String = b.iter().map(|x| format!("{x:02x}")).collect();
    format!("{}-{}-{}-{}-{}", &h[..8], &h[8..12], &h[12..16], &h[16..20], &h[20..])
}

/// The current time as `YYYY-MM-DDThh:mm:ssZ`.
fn now_utc() -> String {
    let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs() as i64);
    let (y, m, d) = crate::import::civil(secs.div_euclid(86_400));
    let t = secs.rem_euclid(86_400);
    format!("{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z", t / 3600, t % 3600 / 60, t % 60)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;

    fn files(epub: &[u8]) -> Vec<(String, String)> {
        let mut zip = zip::ZipArchive::new(std::io::Cursor::new(epub)).unwrap();
        (0..zip.len())
            .map(|i| {
                let mut f = zip.by_index(i).unwrap();
                let mut s = String::new();
                let _ = f.read_to_string(&mut s);
                (f.name().unwrap().into_owned(), s)
            })
            .collect()
    }

    fn well_formed(xml: &str) {
        let mut reader = quick_xml::Reader::from_str(xml);
        loop {
            match reader.read_event() {
                Ok(quick_xml::events::Event::Eof) => break,
                Err(e) => panic!("{e}: {xml}"),
                _ => {}
            }
        }
    }

    #[test]
    fn metadata_from_front_matter() {
        let m = Meta::from_source("---\ntitle: Book\nauthor: [Ann, Bob]\nlang: de\ncover: c.png\n---\n# X\n", "Fallback");
        assert_eq!((m.title.as_str(), m.author.as_deref(), m.language.as_str()), ("Book", Some("Ann, Bob"), "de"));
        assert_eq!(m.cover.as_deref(), Some("c.png"));
        assert_eq!(Meta::from_source("# Only text\n", "Fallback").title, "Fallback");
    }

    #[test]
    fn builds_a_valid_package() {
        let dir = std::env::temp_dir().join(format!("qn-epub-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("pic.png"), b"\x89PNG fake").unwrap();
        let heading = |level, id: &str, text: &str| Heading { level, id: id.into(), text: text.into() };
        let extract = Extract {
            chapters: vec![
                Chapter {
                    headings: vec![heading(1, "a", "One & Two"), heading(2, "a1", "Sub")],
                    html: "<h1 xmlns=\"http://www.w3.org/1999/xhtml\" id=\"a\">One &amp; Two</h1><p xmlns=\"http://www.w3.org/1999/xhtml\"><img src=\"qn-image:0\" alt=\"\"/><img src=\"qn-image:1\" alt=\"\"/></p>".into(),
                },
                Chapter {
                    headings: vec![heading(1, "b", "Three")],
                    html: "<h1 xmlns=\"http://www.w3.org/1999/xhtml\" id=\"b\">Three</h1><svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 1 1\"/>".into(),
                },
            ],
            images: vec!["pic.png".into(), "missing.png".into()],
            math: false,
        };
        let meta = Meta { title: "T <1>".into(), language: "en".into(), author: Some("Me".into()), ..Default::default() };
        let epub = build(&meta, extract, Some(&dir), "body{}", "").unwrap();
        let files = files(&epub);
        assert_eq!(files[0], ("mimetype".into(), "application/epub+zip".into()));
        let get = |name: &str| files.iter().find(|f| f.0 == name).map(|f| f.1.clone()).unwrap_or_else(|| panic!("{name}"));
        for (name, content) in &files {
            if name.ends_with(".xhtml") || name.ends_with(".opf") || name.ends_with(".ncx") || name.ends_with(".xml") {
                well_formed(content);
            }
        }
        let opf = get("OEBPS/content.opf");
        assert!(opf.contains("<dc:title>T &lt;1&gt;</dc:title>") && opf.contains("<dc:creator>Me</dc:creator>"));
        assert!(opf.contains("href=\"chapter-2.xhtml\" media-type=\"application/xhtml+xml\" properties=\"svg\""));
        assert!(opf.contains("images/image-0.png") && !opf.contains("image-1"));
        let c1 = get("OEBPS/chapter-1.xhtml");
        assert!(c1.contains("src=\"images/image-0.png\"") && c1.contains("src=\"missing.png\""));
        let nav = get("OEBPS/nav.xhtml");
        assert!(nav.contains("<a href=\"chapter-1.xhtml#a\">One &amp; Two</a><ol><li><a href=\"chapter-1.xhtml#a1\">Sub</a>"));
        assert!(get("OEBPS/toc.ncx").contains("playOrder=\"3\""));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn uuids_and_dates() {
        let u = uuid();
        assert_eq!((u.len(), &u[14..15]), (36, "4"));
        assert_ne!(u, uuid());
        let now = now_utc();
        assert!(now.ends_with('Z') && now.len() == 20, "{now}");
    }
}
