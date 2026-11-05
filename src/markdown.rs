//! Markdown to HTML.

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

#[cfg(test)]
mod tests {
    #[test]
    fn renders_tables_and_tasks() {
        let html = super::to_html("| a |\n|---|\n| b |\n\n- [x] done\n");
        assert!(html.contains("<table>"));
        assert!(html.contains("checkbox"));
    }
}
