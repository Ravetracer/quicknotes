//! Markdown to HTML.

use pulldown_cmark::{html, Event, Options, Parser};

const OPTIONS: Options = Options::ENABLE_TABLES
    .union(Options::ENABLE_STRIKETHROUGH)
    .union(Options::ENABLE_TASKLISTS)
    .union(Options::ENABLE_FOOTNOTES)
    .union(Options::ENABLE_HEADING_ATTRIBUTES);

pub fn to_html(markdown: &str) -> String {
    let mut out = String::with_capacity(markdown.len() * 3 / 2);
    html::push_html(&mut out, Parser::new_ext(markdown, OPTIONS));
    out
}

/// HTML for the live preview: task checkboxes are clickable and carry their
/// source line (`data-line`, 0-based) so a click can toggle the task.
pub fn to_html_interactive(markdown: &str) -> String {
    let (mut line, mut scanned) = (0usize, 0usize);
    let events = Parser::new_ext(markdown, OPTIONS).into_offset_iter().map(|(event, range)| match event {
        Event::TaskListMarker(checked) => {
            line += markdown[scanned..range.start].matches('\n').count();
            scanned = range.start;
            let checked = if checked { " checked" } else { "" };
            Event::InlineHtml(format!("<input type=\"checkbox\" class=\"task\" data-line=\"{line}\"{checked}>").into())
        }
        e => e,
    });
    let mut out = String::with_capacity(markdown.len() * 3 / 2);
    html::push_html(&mut out, events);
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

    #[test]
    fn interactive_tasks_carry_their_line() {
        let html = super::to_html_interactive("# T\n\n- [ ] a\n- [x] b\n\n```\n- [ ] code\n```\n  - [ ] c\n");
        assert!(html.contains(r#"data-line="2">"#), "{html}");
        assert!(html.contains(r#"data-line="3" checked>"#), "{html}");
        assert!(html.contains(r#"data-line="8">"#), "{html}");
        assert_eq!(html.matches("class=\"task\"").count(), 3);
    }
}
