//! HTML for the preview pane (Markdown and PlantUML).

use pulldown_cmark::{html, Options, Parser};

pub fn to_html(markdown: &str) -> String {
    let opts = Options::ENABLE_TABLES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_FOOTNOTES
        | Options::ENABLE_HEADING_ATTRIBUTES;
    let mut out = String::with_capacity(markdown.len() * 3 / 2);
    html::push_html(&mut out, Parser::new_ext(markdown, opts));
    out
}

pub fn escape(text: &str) -> String {
    text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

/// The preview page shell. Content is swapped via `setContent()` so the
/// scroll position survives updates.
pub fn page(dark: bool) -> String {
    let theme = if dark { "dark" } else { "light" };
    format!(
        r#"<!DOCTYPE html>
<html data-theme="{theme}">
<head>
<meta charset="utf-8">
<style>
:root {{
  --bg: #ffffff; --fg: #1f2328; --muted: #59636e; --border: #d1d9e0;
  --code-bg: #f3f4f6; --link: #1a5fb4; --quote: #6e7781;
}}
html[data-theme="dark"] {{
  --bg: #1e1e1e; --fg: #e6e6e6; --muted: #9a9a9a; --border: #3a3a3a;
  --code-bg: #2a2a2a; --link: #78aeed; --quote: #a0a0a0;
}}
html, body {{ background: var(--bg); color: var(--fg); margin: 0; }}
body {{
  font-family: "Cantarell", "Inter", "Segoe UI", sans-serif;
  font-size: 15px; line-height: 1.6; padding: 20px 28px 60px;
}}
#content {{ max-width: 920px; margin: 0 auto; }}
a {{ color: var(--link); }}
h1, h2 {{ border-bottom: 1px solid var(--border); padding-bottom: .3em; }}
h1, h2, h3, h4 {{ line-height: 1.25; margin-top: 1.4em; }}
code, pre {{ font-family: "Monospace", monospace; font-size: 0.9em; }}
code {{ background: var(--code-bg); padding: .15em .35em; border-radius: 5px; }}
pre {{ background: var(--code-bg); padding: 12px 14px; border-radius: 8px; overflow-x: auto; line-height: 1.45; }}
pre code {{ background: none; padding: 0; }}
blockquote {{ margin: 0; padding: 0 1em; color: var(--quote); border-left: 4px solid var(--border); }}
table {{ border-collapse: collapse; margin: 1em 0; }}
th, td {{ border: 1px solid var(--border); padding: 6px 12px; }}
th {{ background: var(--code-bg); }}
img {{ max-width: 100%; }}
hr {{ border: none; border-top: 1px solid var(--border); }}
li > input[type=checkbox] {{ margin-right: .4em; }}
.diagram {{ display: flex; justify-content: center; }}
.diagram .card {{ background: #ffffff; border-radius: 10px; padding: 16px; box-shadow: 0 1px 4px rgba(0,0,0,.25); max-width: 100%; overflow: auto; }}
.diagram svg {{ max-width: 100%; height: auto; }}
.message {{ color: var(--muted); font-family: "Monospace", monospace; white-space: pre-wrap; }}
.error {{ color: #e01b24; }}
</style>
<script>
window.setContent = function (html) {{ document.getElementById('content').innerHTML = html; }};
window.setTheme = function (t) {{ document.documentElement.dataset.theme = t; }};
</script>
</head>
<body><div id="content"></div></body>
</html>"#
    )
}

#[cfg(test)]
mod tests {
    #[test]
    fn renders_tables_and_tasks() {
        let html = super::to_html("| a |\n|---|\n| b |\n\n- [x] done\n");
        assert!(html.contains("<table>"));
        assert!(html.contains("checkbox"));
    }
}
