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

pub fn show(parent: &impl IsA<gtk::Window>, theme: &crate::themes::Theme, zoom: f64) {
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
        settings.set_enable_javascript(false);
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
    fn real_help_files_filter_cleanly() {
        for help in [HELP_EN, HELP_DE] {
            let ifs = help.matches("<!-- if ").count();
            assert_eq!(ifs, help.matches("<!-- end -->").count(), "unbalanced markers");
            let keys: Vec<&str> = crate::features::LIST.iter().map(|(f, ..)| f.key()).collect();
            let minimal = filter(help, |k| !keys.contains(&k));
            for key in ["charts", "slides", "kanban", "geocaching", "devtools"] {
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
