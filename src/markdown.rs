//! Markdown to HTML.

use pulldown_cmark::{html, Event, Options, Parser};
use std::borrow::Cow;

const OPTIONS: Options = Options::ENABLE_TABLES
    .union(Options::ENABLE_STRIKETHROUGH)
    .union(Options::ENABLE_TASKLISTS)
    .union(Options::ENABLE_FOOTNOTES)
    .union(Options::ENABLE_HEADING_ATTRIBUTES)
    .union(Options::ENABLE_MATH);

pub fn to_html(markdown: &str) -> String {
    let markdown = prepare(markdown);
    let mut out = String::with_capacity(markdown.len() * 3 / 2);
    html::push_html(&mut out, Parser::new_ext(&markdown, OPTIONS));
    out
}

/// The source as it is rendered: chart blocks resolved and the front matter
/// hidden. Lines stay where they are.
fn prepare(markdown: &str) -> Cow<'_, str> {
    let markdown = crate::charts::expand(markdown);
    if !markdown.starts_with("---") {
        return markdown;
    }
    let lines: Vec<&str> = markdown.split('\n').collect();
    let (Some(_), end) = crate::slides::front_matter(&lines) else { return markdown };
    let blank = "\n".repeat(end);
    Cow::Owned(blank + &lines[end..].join("\n"))
}

/// HTML for the live preview: task checkboxes are clickable and carry their
/// source line (`data-line`, 0-based) so a click can toggle the task.
pub fn to_html_interactive(markdown: &str) -> String {
    let markdown = &*prepare(markdown);
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

    #[test]
    fn hides_front_matter_and_keeps_lines() {
        let html = super::to_html_interactive("---\ncharts:\n  theme: dark\n---\n# T\n- [ ] a\n");
        assert!(!html.contains("charts"), "{html}");
        assert!(!html.contains("<hr"), "{html}");
        assert!(html.contains(r#"data-line="5">"#), "{html}");
        // A leading rule without YAML stays.
        assert!(super::to_html("---\n\ntext\n").contains("<hr"));
    }

    #[test]
    fn resolves_charts() {
        let html = super::to_html("| a | b |\n|---|---|\n| x | 1 |\n\n```chart\ntype: pie\n```\n");
        assert!(html.contains("<code class=\"language-qn-chart\">{\"option\""), "{html}");
    }
}
