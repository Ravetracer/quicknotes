//! reStructuredText to HTML, using the `docutils-core` Rust port.

use crate::html::escape;
use docutils_core::settings::Settings;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::Path;

/// Report level ERROR: INFO/WARNING messages are mostly noise while typing.
const REPORT_LEVEL: u8 = 3;

/// Renders the document body as HTML. `base_dir` resolves `include` and
/// other relative paths. Errors are shown inline, like docutils does.
pub fn to_html(source: &str, base_dir: Option<&Path>) -> String {
    let settings = Settings {
        report_level: REPORT_LEVEL,
        ..Settings::default()
    };
    let base = base_dir.map(Path::to_path_buf);
    // The full document (not the fragment API) keeps the promoted document
    // title. docutils-core is young; never let a parser panic take the app down.
    match catch_unwind(AssertUnwindSafe(|| {
        docutils_core::core::publish_string_with(source, &settings, base)
    })) {
        Ok(Ok((page, _))) => body_of(&page).to_owned(),
        Ok(Err(halt)) => format!("<p class=\"message error\">{}</p>", escape(&format!("{halt:?}"))),
        Err(_) => "<p class=\"message error\">The reStructuredText renderer failed on this document.</p>".to_owned(),
    }
}

/// Content of `<main>` (or `<body>`) of a complete HTML page.
fn body_of(page: &str) -> &str {
    for (open, close) in [("<main>", "</main>"), ("<body>", "</body>")] {
        if let (Some(start), Some(end)) = (page.find(open), page.rfind(close)) {
            if start + open.len() <= end {
                return page[start + open.len()..end].trim();
            }
        }
    }
    page
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_basic_rst() {
        let html = to_html("Title\n=====\n\nSome *text* and ``code``.\n\n.. note:: Careful.\n", None);
        assert!(html.contains("<h1 class=\"title\">Title</h1>"));
        assert!(html.contains("<em>text</em>"));
        assert!(html.contains("admonition"));
        assert!(!html.contains("<body>"));
    }

    #[test]
    fn hides_warnings_shows_errors() {
        let html = to_html("Text \"*\" here.\n\n.. include:: does-not-exist.rst\n", None);
        assert!(!html.contains("WARNING/2"));
        assert!(html.contains("ERROR/3"));
    }
}
