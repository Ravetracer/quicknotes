//! In-app help window, rendered from `data/help.md`.

use crate::{html, markdown};
use adw::prelude::*;
use gtk::gio;
use webkit6::prelude::*;

const HELP_EN: &str = include_str!("../data/help.md");
const HELP_DE: &str = include_str!("../data/help.de.md");

/// Whether the help for a section or marker key (see [`Feature::key`]) is
/// shown; keys of no feature always are.
fn enabled(key: &str) -> bool {
    crate::features::LIST.iter().find(|(f, ..)| f.key() == key).is_none_or(|(f, ..)| crate::features::on(*f))
}

/// The help for the features that are on: `## … {#key}` sections of
/// switched-off features and their links in the contents are left out, and
/// so are `<!-- if key -->` … `<!-- end -->` blocks (`<!-- if !key -->`:
/// shown only while the feature is off).
fn filter(help: &str, enabled: impl Fn(&str) -> bool) -> String {
    let mut out: Vec<String> = Vec::new();
    let mut hidden_section = false;
    let mut blocks: Vec<bool> = Vec::new();
    for line in help.lines() {
        let trimmed = line.trim();
        if let Some(cond) = trimmed.strip_prefix("<!-- if ").and_then(|c| c.strip_suffix("-->")) {
            let cond = cond.trim();
            let show = match cond.strip_prefix('!') {
                Some(key) => !enabled(key),
                None => enabled(cond),
            };
            blocks.push(show);
            continue;
        }
        if trimmed == "<!-- end -->" {
            blocks.pop();
            continue;
        }
        if line.starts_with("## ") {
            hidden_section = section_id(line).is_some_and(|id| !enabled(id));
        }
        if !hidden_section && blocks.iter().all(|b| *b) {
            out.push(line.to_owned());
        }
    }
    let mut text = out.join("\n");
    // Contents: the first paragraph with links to sections.
    if let Some(start) = text.find("](#") {
        let start = text[..start].rfind("\n\n").map_or(0, |i| i + 2);
        let end = text[start..].find("\n\n").map_or(text.len(), |i| start + i);
        let items: Vec<&str> = text[start..end]
            .split('·')
            .map(str::trim)
            .filter(|item| !item.is_empty())
            .filter(|item| link_target(item).is_none_or(&enabled))
            .collect();
        text.replace_range(start..end, &items.join(" · "));
    }
    text
}

/// `id` of a heading `## Title {#id}`.
fn section_id(heading: &str) -> Option<&str> {
    heading.trim_end().strip_suffix('}')?.rsplit_once("{#").map(|(_, id)| id)
}

/// `id` of a link `[Text](#id)`.
fn link_target(item: &str) -> Option<&str> {
    item.split_once("](#")?.1.split(')').next()
}

/// What the user is working on, for opening the help at the right section.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Doc<'a> {
    Note,
    Markdown,
    Flashcards,
    Slides,
    Kanban,
    Mindmap,
    Calc,
    /// reStructuredText, HTML or PlantUML with a preview.
    Preview,
    /// Source code in a language (GtkSourceView id).
    Code(&'a str),
    Image,
}

/// Where the cursor is.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Cursor<'a> {
    /// Info string of the fenced code block around the cursor.
    pub fence: Option<&'a str>,
    pub in_table: bool,
    pub on_task: bool,
}

/// The help section (`{#id}`) for the current document and cursor.
pub fn topic(doc: Doc<'_>, cursor: Cursor<'_>, emmet: bool) -> &'static str {
    let markdown = matches!(doc, Doc::Note | Doc::Markdown | Doc::Slides | Doc::Flashcards);
    if markdown {
        match cursor.fence.and_then(|f| f.split_whitespace().next()) {
            Some("chart") => return "charts",
            Some("math" | "abc") => return "notation",
            _ => {}
        }
        if cursor.in_table {
            return "tables";
        }
        if cursor.on_task && doc != Doc::Flashcards {
            return "checklists";
        }
    }
    match doc {
        Doc::Flashcards => "flashcards",
        Doc::Slides => "slides",
        Doc::Kanban => "kanban",
        Doc::Mindmap => "mindmaps",
        Doc::Calc => "calc",
        Doc::Markdown => "formatting",
        Doc::Preview => "previews",
        Doc::Code(crate::mongo::LANGUAGE) | Doc::Code("json5") => "mongodb",
        Doc::Code("json" | "yaml" | "xml" | "xslt") if crate::features::on(crate::features::Feature::DataFormats) => "dataformats",
        Doc::Code(_) if emmet => "emmet",
        Doc::Code(_) => "devtools",
        Doc::Note | Doc::Image => "notes",
    }
}

/// Shows the help, scrolled to the section `topic` (if it is there – it
/// may belong to a switched-off feature).
pub fn show(parent: &impl IsA<gtk::Window>, theme: &crate::themes::Theme, zoom: f64, topic: Option<&str>) {
    let help = match crate::i18n::lang() {
        crate::i18n::Lang::De => HELP_DE,
        crate::i18n::Lang::En => HELP_EN,
    };
    let body = markdown::to_html(&filter(help, enabled));
    let page = html::static_page(t!("QuickNotes Help"), &body, theme);

    let webview = webkit6::WebView::new();
    webview.set_vexpand(true);
    webview.set_zoom_level(zoom);
    webview.set_background_color(&crate::editor::preview_bg(theme));
    if let Some(settings) = WebViewExt::settings(&webview) {
        // No scripts from the page; the app's own (scrolling) may run.
        settings.set_enable_javascript(true);
        settings.set_enable_javascript_markup(false);
    }
    if let Some(id) = topic.filter(|id| id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')) {
        let js = format!("var e = document.getElementById('{id}'); if (e) e.scrollIntoView();");
        webview.connect_load_changed(move |wv, event| {
            if event == webkit6::LoadEvent::Finished {
                wv.evaluate_javascript(&js, None, None, None::<&gio::Cancellable>, |_| {});
            }
        });
    }
    // In-page links (#section) navigate within the help, others open in the browser.
    webview.connect_decide_policy(|_, decision, kind| {
        if kind != webkit6::PolicyDecisionType::NavigationAction {
            return false;
        }
        let Some(nav) = decision.downcast_ref::<webkit6::NavigationPolicyDecision>() else {
            return false;
        };
        let Some(mut action) = nav.navigation_action() else { return false };
        if action.navigation_type() != webkit6::NavigationType::LinkClicked {
            return false;
        }
        let uri = action.request().and_then(|r| r.uri()).unwrap_or_default();
        if uri.starts_with("about:blank#") {
            return false;
        }
        gtk::UriLauncher::new(&uri).launch(None::<&gtk::Window>, None::<&gio::Cancellable>, |_| {});
        decision.ignore();
        true
    });
    webview.load_html(&page, None);

    let toolbar = adw::ToolbarView::new();
    toolbar.add_top_bar(&adw::HeaderBar::new());
    toolbar.set_content(Some(&webview));
    let window = adw::Window::builder()
        .title(t!("QuickNotes Help"))
        .default_width(880)
        .default_height(780)
        .transient_for(parent)
        .content(&toolbar)
        .build();
    window.present();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn off(keys: &'static [&'static str]) -> impl Fn(&str) -> bool {
        move |k: &str| !keys.contains(&k)
    }

    #[test]
    fn filters_sections_contents_and_blocks() {
        let help = "# Help\n\n**Contents:** [A](#a) · [Charts](#charts) ·\n[Kanban](#kanban) · [B](#b)\n\n## A {#a}\n\nx\n<!-- if charts -->\nchart line\n<!-- end -->\n<!-- if !charts -->\nno charts\n<!-- end -->\n\n## Charts {#charts}\n\n### Sub\n\nc\n\n## B {#b}\n\nb\n";
        let out = filter(help, off(&["charts"]));
        assert!(out.contains("**Contents:** [A](#a) · [Kanban](#kanban) · [B](#b)\n"), "{out}");
        assert!(!out.contains("chart line") && out.contains("no charts"));
        assert!(!out.contains("## Charts") && !out.contains("### Sub") && out.contains("## B {#b}"));
        assert!(!out.contains("<!--"));
        let all = filter(help, |_| true);
        assert!(all.contains("chart line") && !all.contains("no charts") && all.contains("### Sub"));
    }

    #[test]
    fn topics_follow_document_and_cursor() {
        let none = Cursor::default();
        assert_eq!(topic(Doc::Flashcards, none, false), "flashcards");
        assert_eq!(topic(Doc::Markdown, none, false), "formatting");
        assert_eq!(topic(Doc::Markdown, Cursor { fence: Some("chart"), ..none }, false), "charts");
        assert_eq!(topic(Doc::Slides, Cursor { fence: Some("math"), ..none }, false), "notation");
        assert_eq!(topic(Doc::Note, Cursor { in_table: true, ..none }, false), "tables");
        assert_eq!(topic(Doc::Note, Cursor { on_task: true, ..none }, false), "checklists");
        assert_eq!(topic(Doc::Flashcards, Cursor { on_task: true, ..none }, false), "flashcards");
        assert_eq!(topic(Doc::Code("html"), Cursor { fence: Some("chart"), ..none }, true), "emmet");
        assert_eq!(topic(Doc::Code("json"), none, false), "dataformats");
        assert_eq!(topic(Doc::Code("sql"), none, false), "devtools");
        assert_eq!(topic(Doc::Code(crate::mongo::LANGUAGE), none, false), "mongodb");
        assert_eq!(topic(Doc::Image, none, false), "notes");
    }

    #[test]
    fn every_topic_is_a_section() {
        let docs = [
            Doc::Note, Doc::Markdown, Doc::Flashcards, Doc::Slides, Doc::Kanban, Doc::Calc, Doc::Preview,
            Doc::Code("rust"), Doc::Code(crate::mongo::LANGUAGE), Doc::Image,
        ];
        let cursors = [
            Cursor::default(),
            Cursor { fence: Some("chart"), ..Default::default() },
            Cursor { fence: Some("abc"), ..Default::default() },
            Cursor { in_table: true, ..Default::default() },
            Cursor { on_task: true, ..Default::default() },
        ];
        for help in [HELP_EN, HELP_DE] {
            for doc in docs {
                for cursor in cursors {
                    for emmet in [false, true] {
                        let id = topic(doc, cursor, emmet);
                        assert!(help.contains(&format!("{{#{id}}}")), "missing section #{id}");
                    }
                }
            }
        }
    }

    #[test]
    fn real_help_files_filter_cleanly() {
        for help in [HELP_EN, HELP_DE] {
            let ifs = help.matches("<!-- if ").count();
            assert_eq!(ifs, help.matches("<!-- end -->").count(), "unbalanced markers");
            let keys: Vec<&str> = crate::features::LIST.iter().map(|(f, ..)| f.key()).collect();
            let minimal = filter(help, |k| !keys.contains(&k));
            for key in ["charts", "slides", "kanban", "geocaching", "devtools", "flashcards", "folders", "sync", "mindmaps", "dataformats"] {
                assert!(!minimal.contains(&format!("{{#{key}}}")), "{key}");
                assert!(!minimal.contains(&format!("](#{key})")), "{key}");
                assert!(filter(help, |_| true).contains(&format!("{{#{key}}}")), "{key}");
            }
            assert!(!minimal.contains("<!--") && !minimal.contains("PlantUML-Install") && !minimal.contains("PlantUML installation"));
            assert!(!minimal.contains("Ctrl+F5") && !minimal.contains("Strg+F5"));
            assert!(!minimal.contains("Ctrl+Alt+M") && !minimal.contains("Strg+Alt+M"));
            assert!(!minimal.contains("PHP-Array") && !minimal.contains("PHP array"));
            // No link points to a section that is gone.
            for link in minimal.split("](#").skip(1) {
                let id = link.split(')').next().unwrap();
                assert!(minimal.contains(&format!("{{#{id}}}")), "dangling link #{id}");
            }
        }
    }
}
