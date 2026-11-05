//! HTML pages for the preview pane and for exported documents.

/// Document styles shared by preview and export. Colors come from CSS
/// variables so the preview can switch themes; covers Markdown output,
/// docutils (reStructuredText) output and PlantUML diagrams.
const DOC_CSS: &str = r##"
:root {
  --bg: #ffffff; --fg: #1f2328; --muted: #59636e; --border: #d1d9e0;
  --code-bg: #f3f4f6; --link: #1a5fb4; --quote: #6e7781;
  --note: #1a5fb4; --warn: #c64600; --error: #c01c28; --tip: #26a269;
}
html[data-theme="dark"] {
  --bg: #1e1e1e; --fg: #e6e6e6; --muted: #9a9a9a; --border: #3a3a3a;
  --code-bg: #2a2a2a; --link: #78aeed; --quote: #a0a0a0;
  --note: #78aeed; --warn: #ffa348; --error: #f66151; --tip: #8ff0a4;
}
html, body { background: var(--bg); color: var(--fg); margin: 0; }
body {
  font-family: "Cantarell", "Inter", "Segoe UI", sans-serif;
  font-size: 15px; line-height: 1.6; padding: 20px 28px 60px;
}
#content { max-width: 920px; margin: 0 auto; }
a { color: var(--link); }
h1, h2 { border-bottom: 1px solid var(--border); padding-bottom: .3em; }
h1, h2, h3, h4 { line-height: 1.25; margin-top: 1.4em; }
h1 a, h2 a, h3 a, h4 a, h5 a, h6 a { color: inherit; text-decoration: none; }
code, pre, tt { font-family: "Monospace", monospace; font-size: 0.9em; }
code, tt.literal, span.docutils.literal { background: var(--code-bg); padding: .15em .35em; border-radius: 5px; }
pre { background: var(--code-bg); padding: 12px 14px; border-radius: 8px; overflow-x: auto; line-height: 1.45; }
pre code { background: none; padding: 0; }
blockquote { margin: 0 0 1em; padding: 0 1em; color: var(--quote); border-left: 4px solid var(--border); }
table { border-collapse: collapse; margin: 1em 0; }
th, td { border: 1px solid var(--border); padding: 6px 12px; vertical-align: top; }
th { background: var(--code-bg); }
img { max-width: 100%; }
hr { border: none; border-top: 1px solid var(--border); }
li > input[type=checkbox] { margin-right: .4em; }
li > p { margin: .2em 0; }

/* reStructuredText (docutils) */
dl.field-list { display: grid; grid-template-columns: max-content auto; gap: .2em 1em; }
dl.field-list > dt { font-weight: 600; }
dl.field-list > dd { margin: 0; }
dl.field-list > dd > p { margin: 0; }
dt { font-weight: 600; }
dd { margin-left: 1.5em; }
nav.contents, div.topic, aside.topic, aside.sidebar {
  border: 1px solid var(--border); border-radius: 8px; padding: .4em 1.2em; margin: 1em 0;
}
.topic-title, .sidebar-title { font-weight: 700; margin: .4em 0; }
.admonition, aside.admonition {
  border-left: 4px solid var(--note); background: var(--code-bg);
  border-radius: 6px; padding: .3em 1em; margin: 1em 0;
}
.admonition-title { font-weight: 700; margin: .4em 0; }
.admonition.warning, .admonition.caution, .admonition.attention { border-left-color: var(--warn); }
.admonition.danger, .admonition.error { border-left-color: var(--error); }
.admonition.tip, .admonition.hint, .admonition.important { border-left-color: var(--tip); }
aside.system-message {
  border: 1px solid var(--error); border-radius: 6px; padding: .2em 1em; margin: 1em 0;
  color: var(--error); font-size: .9em;
}
.system-message-title { font-weight: 700; margin: .4em 0; }
/* Spots of hidden warnings: render as normal text */
a[href^="#system-message"] { color: inherit; text-decoration: none; }
.problematic { color: inherit; }
h1.title { text-align: center; border: none; }
.footnote-list, aside.footnote { font-size: .9em; }
aside.footnote { display: flex; gap: .5em; }
aside.footnote > .label { flex: none; }
aside.footnote > p { margin: 0; }
.line-block { margin: 1em 0; }
.line-block .line-block { margin: 0 0 0 1.5em; }
.rubric { font-weight: 700; }
figure { margin: 1em 0; }
figcaption, .caption { color: var(--muted); font-size: .9em; }
.align-center { display: block; margin-left: auto; margin-right: auto; }
.align-right { float: right; margin-left: 1em; }
.align-left { float: left; margin-right: 1em; }

/* PlantUML */
.diagram { display: flex; justify-content: center; }
.diagram .card { background: #ffffff; border-radius: 10px; padding: 16px; box-shadow: 0 1px 4px rgba(0,0,0,.25); max-width: 100%; overflow: auto; }
.diagram svg { max-width: 100%; height: auto; }
.message { color: var(--muted); font-family: "Monospace", monospace; white-space: pre-wrap; }
.error { color: var(--error); }
"##;

/// Extra styles for exported documents (screen and print/PDF).
const EXPORT_CSS: &str = r#"
@media print {
  body { padding: 0; font-size: 11pt; }
  #content { max-width: none; }
  pre { white-space: pre-wrap; overflow-wrap: anywhere; }
  a { color: inherit; }
}
pre, blockquote, table, img, figure, .admonition { break-inside: avoid; }
h1, h2, h3, h4 { break-after: avoid; }
"#;

pub fn escape(text: &str) -> String {
    text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

/// The preview page shell. Content is swapped via `setContent()` so the
/// scroll position survives updates.
pub fn preview_shell(dark: bool) -> String {
    let theme = if dark { "dark" } else { "light" };
    format!(
        r#"<!DOCTYPE html>
<html data-theme="{theme}">
<head>
<meta charset="utf-8">
<style>{DOC_CSS}</style>
<script>
window.setContent = function (html) {{ document.getElementById('content').innerHTML = html; }};
window.setTheme = function (t) {{ document.documentElement.dataset.theme = t; }};
</script>
</head>
<body><div id="content"></div></body>
</html>"#
    )
}

/// A self-contained HTML document (light theme) for export.
pub fn standalone(title: &str, body: &str) -> String {
    let title = escape(title);
    format!(
        r#"<!DOCTYPE html>
<html lang="en" data-theme="light">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<meta name="generator" content="QuickNotes">
<title>{title}</title>
<style>{DOC_CSS}{EXPORT_CSS}</style>
</head>
<body>
<main id="content">
{body}
</main>
</body>
</html>
"#
    )
}

/// Text of the first heading (h1–h3) in an HTML fragment, used as document title.
pub fn first_heading(body: &str) -> Option<String> {
    let start = ["<h1", "<h2", "<h3"].iter().filter_map(|t| body.find(t)).min()?;
    let level = &body[start + 1..start + 3];
    let open_end = start + body[start..].find('>')? + 1;
    let close = open_end + body[open_end..].find(&format!("</{level}>"))?;
    let mut text = String::new();
    let mut in_tag = false;
    for c in body[open_end..close].chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            c if !in_tag => text.push(c),
            _ => {}
        }
    }
    let text = text
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'");
    let text = text.trim();
    (!text.is_empty()).then(|| text.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_first_heading() {
        assert_eq!(first_heading("<p>x</p><h2 id=\"a\"><a href=\"#\">Tom &amp; Jerry</a></h2><h1>B</h1>").as_deref(), Some("Tom & Jerry"));
        assert_eq!(first_heading("<p>none</p>"), None);
    }

    #[test]
    fn standalone_escapes_title() {
        let doc = standalone("<x>", "<p>body</p>");
        assert!(doc.contains("<title>&lt;x&gt;</title>"));
        assert!(doc.contains("<p>body</p>"));
    }
}
