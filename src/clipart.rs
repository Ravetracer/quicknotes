//! OpenClipart.org: searching and downloading public-domain (CC0) graphics.
//!
//! The site has no API any more, so the search result page is read: every
//! result is an `artwork` block linking `/detail/<id>/<slug>` with a 200 px
//! thumbnail at `/image/200px/<id>`; `/download/<id>` delivers the SVG.
//! Network calls block; they run in a worker thread (see `window/canvas_ui.rs`).

use std::io::Read;
use std::time::Duration;

const BASE: &str = "https://openclipart.org";

/// One search result.
#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct Hit {
    pub id: String,
    pub title: String,
    pub thumb: String,
}

/// One page of search results.
#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct Page {
    pub items: Vec<Hit>,
    /// There is a next page.
    pub more: bool,
}

fn agent() -> ureq::Agent {
    ureq::AgentBuilder::new().timeout(Duration::from_secs(30)).user_agent("QuickNotes").build()
}

/// Searches OpenClipart; `page` starts at 1.
pub fn search(query: &str, page: u32) -> Result<Page, String> {
    let url = format!("{BASE}/search/?query={}&p={page}", urlencoding::encode(query.trim()));
    let html = agent().get(&url).call().map_err(|e| e.to_string())?.into_string().map_err(|e| e.to_string())?;
    Ok(parse_page(&html, page))
}

/// The SVG of a graphic.
pub fn download(id: &str) -> Result<Vec<u8>, String> {
    if id.is_empty() || !id.bytes().all(|b| b.is_ascii_digit()) {
        return Err("invalid id".into());
    }
    let response = agent().get(&format!("{BASE}/download/{id}")).call().map_err(|e| e.to_string())?;
    let mut data = Vec::new();
    response
        .into_reader()
        .take(crate::canvas::MAX_IMAGE as u64 + 1)
        .read_to_end(&mut data)
        .map_err(|e| e.to_string())?;
    if data.len() > crate::canvas::MAX_IMAGE {
        return Err(t!("Images larger than 25 MB are not added").into());
    }
    if crate::canvas::sniff(&data) != Some("svg") {
        return Err(t!("OpenClipart did not deliver an SVG image").into());
    }
    Ok(data)
}

/// Reads the results out of a search page.
pub fn parse_page(html: &str, page: u32) -> Page {
    let re = regex::Regex::new(
        r#"(?s)<div class="artwork">\s*<a href="/detail/(\d+)/[^"]*">.*?<img src="(/image/200px/\d+)"[^>]*?alt="([^"]*)""#,
    )
    .expect("valid regex");
    let mut items: Vec<Hit> = Vec::new();
    for c in re.captures_iter(html) {
        let id = c[1].to_owned();
        if items.iter().any(|h| h.id == id) {
            continue;
        }
        items.push(Hit { id, title: unescape(c[3].trim()), thumb: format!("{BASE}{}", &c[2]) });
    }
    let more = html.contains(&format!("?p={}&", page + 1));
    Page { items, more }
}

fn unescape(s: &str) -> String {
    s.replace("&quot;", "\"")
        .replace("&#039;", "'")
        .replace("&#39;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
}

#[cfg(test)]
mod tests {
    use super::*;

    const PAGE: &str = r#"<div class="gallery"> <div class="artwork"> <a href="/detail/348615/black-cat-with-transparency"> <picture data-ocal-clip-id="348615"> <source type="image/webp" srcset="/image/200px/348615.webp" /> <img src="/image/200px/348615" width="200" height="200" alt="Black cat with transparency " loading="lazy" /> </picture> </a> </div> <div class="artwork"> <a href="/detail/346856/cheshire-cat"> <picture> <img src="/image/200px/346856" width="200" height="200" alt="Cheshire &quot;Cat&quot; &amp; friends" loading="lazy" /> </picture> </a> </div></div>
<nav><a href="?p=0&query=cat">‹</a> <a href="?p=2&query=cat">›</a></nav>"#;

    #[test]
    fn search_pages_are_read() {
        let page = parse_page(PAGE, 1);
        assert_eq!(page.items.len(), 2);
        assert_eq!(
            page.items[0],
            Hit {
                id: "348615".into(),
                title: "Black cat with transparency".into(),
                thumb: "https://openclipart.org/image/200px/348615".into()
            }
        );
        assert_eq!(page.items[1].title, "Cheshire \"Cat\" & friends");
        assert!(page.more);
        assert!(!parse_page(PAGE, 2).more);
        assert!(parse_page("<html>nothing</html>", 1).items.is_empty());
    }

    #[test]
    fn ids_are_checked() {
        assert!(download("../x").is_err());
        assert!(download("").is_err());
    }

    /// Against the real site (`QN_IT_OPENCLIPART=1`).
    #[test]
    #[ignore]
    fn live_search() {
        if std::env::var("QN_IT_OPENCLIPART").is_err() {
            return;
        }
        let page = search("cat", 1).unwrap();
        assert!(!page.items.is_empty());
        assert!(download(&page.items[0].id).is_ok());
    }
}
