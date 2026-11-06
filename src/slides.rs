//! Presentations: Markdown split into slides by `---` lines, with a few
//! extensions for transitions, step-by-step reveal, positioning, background
//! images and presenter notes. The syntax is documented in `data/help.md`.
//!
//! This module parses the source into a [`Deck`] (one HTML `<section>` per
//! slide) and builds the HTML pages for the editor preview, the presentation
//! windows (audience and presenter view) and the exports. Navigation,
//! animations and the presenter view live in `data/slides/engine.js`.

use crate::{html, markdown};
use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};
use serde::Serialize;

/// GtkSourceView language id of presentations (`data/slides.lang`).
pub const LANGUAGE: &str = "slides";

const ENGINE_JS: &str = include_str!("../data/slides/engine.js");
const TEMPLATE_EN: &str = include_str!("../data/slides-template.md");
const TEMPLATE_DE: &str = include_str!("../data/slides-template.de.md");

/// Content of a new presentation: a short example of the syntax.
pub fn template() -> &'static str {
    match crate::i18n::lang() {
        crate::i18n::Lang::De => TEMPLATE_DE,
        crate::i18n::Lang::En => TEMPLATE_EN,
    }
}
const DECK_CSS: &str = include_str!("../data/slides/deck.css");

/// Slide transitions (`transition:` directive).
pub const TRANSITIONS: &[&str] = &[
    "none", "fade", "slide-left", "slide-right", "slide-up", "slide-down", "cover", "zoom", "zoom-out", "flip", "cube",
    "rotate", "blur",
];

/// Animations of steps (`::: step …` containers, `step:` directive).
pub const STEP_ANIMATIONS: &[&str] = &[
    "none", "fade", "fade-up", "fade-down", "fade-left", "fade-right", "zoom", "zoom-out", "flip", "rotate", "blur",
    "bounce", "drop", "wipe",
];

const THEMES: &[&str] = &["default", "dark", "ocean", "sunset"];

/// An animation with its duration.
#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
pub struct Anim {
    pub name: String,
    pub ms: u32,
}

impl Anim {
    fn new(name: &str, ms: u32) -> Self {
        Self { name: name.to_owned(), ms }
    }

    /// Parses e.g. `zoom 0.8s`, `fade-up slow`, `600ms`; unknown names keep the default.
    fn parse(value: &str, default: &Anim, allowed: &[&str]) -> Anim {
        let mut anim = default.clone();
        for token in value.split_whitespace() {
            let token = token.to_ascii_lowercase();
            if let Some(ms) = duration_ms(&token) {
                anim.ms = ms;
            } else if let Some(name) = animation_name(&token, allowed) {
                anim.name = name.to_owned();
            }
        }
        anim
    }
}

fn animation_name<'a>(token: &str, allowed: &[&'a str]) -> Option<&'a str> {
    let token = match token {
        "slide" | "push" => "slide-left",
        "zoom-in" => "zoom",
        "appear" => "none",
        t => t,
    };
    allowed.iter().find(|a| **a == token).copied()
}

/// `300ms`, `0.5s`, `2s`, or the speeds `fast`, `normal`, `slow`.
fn duration_ms(token: &str) -> Option<u32> {
    match token {
        "fast" => return Some(300),
        "normal" => return Some(600),
        "slow" => return Some(1000),
        _ => {}
    }
    let (num, factor) = if let Some(n) = token.strip_suffix("ms") {
        (n, 1.0)
    } else {
        (token.strip_suffix('s')?, 1000.0)
    };
    let v: f64 = num.parse().ok()?;
    (v.is_finite() && v >= 0.0).then(|| (v * factor).round().min(60_000.0) as u32)
}

/// Settings of the whole presentation (front matter) and, after directives
/// are applied, of a single slide.
#[derive(Clone, Debug)]
struct Config {
    title: Option<String>,
    theme: String,
    width: u32,
    height: u32,
    transition: Anim,
    step: Anim,
    paginate: bool,
    header: Option<String>,
    footer: Option<String>,
    background: Option<String>,
    color: Option<String>,
    align: Option<String>,
    valign: Option<String>,
    class: Option<String>,
    /// Planned length of the talk in seconds (shown in the presenter view).
    duration: Option<u32>,
    style: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            title: None,
            theme: "default".into(),
            width: 1280,
            height: 720,
            transition: Anim::new("none", 500),
            step: Anim::new("fade", 400),
            paginate: false,
            header: None,
            footer: None,
            background: None,
            color: None,
            align: None,
            valign: None,
            class: None,
            duration: None,
            style: String::new(),
        }
    }
}

impl Config {
    /// Applies one `key: value` setting (front matter or slide directive).
    fn set(&mut self, key: &str, value: &str) {
        let value = value.trim();
        let opt = || (!value.is_empty()).then(|| value.to_owned());
        match key {
            "title" => self.title = opt(),
            "theme" => {
                let v = value.to_ascii_lowercase();
                if THEMES.contains(&v.as_str()) {
                    self.theme = v;
                }
            }
            "aspect" | "size" => {
                (self.width, self.height) = match value {
                    "4:3" => (960, 720),
                    "16:10" => (1280, 800),
                    _ => (1280, 720),
                }
            }
            "transition" => self.transition = Anim::parse(value, &self.transition, TRANSITIONS),
            "step" | "steps" => self.step = Anim::parse(value, &self.step, STEP_ANIMATIONS),
            "paginate" => self.paginate = matches!(value.to_ascii_lowercase().as_str(), "true" | "yes" | "on" | "1"),
            "header" => self.header = opt(),
            "footer" => self.footer = opt(),
            "background" | "bg" => self.background = opt(),
            "color" => self.color = opt(),
            "align" => self.align = horizontal(value).map(str::to_owned),
            "valign" => self.valign = vertical(value).map(str::to_owned),
            "class" => self.class = opt(),
            "duration" => self.duration = talk_duration(value),
            "style" => self.style = value.to_owned(),
            _ => {}
        }
    }
}

const SLIDE_KEYS: &[&str] = &[
    "transition", "step", "steps", "theme", "paginate", "header", "footer", "background", "bg", "color", "align", "valign",
    "class",
];

fn horizontal(v: &str) -> Option<&'static str> {
    match v.trim().to_ascii_lowercase().as_str() {
        "left" => Some("left"),
        "center" | "centre" => Some("center"),
        "right" => Some("right"),
        _ => None,
    }
}

fn vertical(v: &str) -> Option<&'static str> {
    match v.trim().to_ascii_lowercase().as_str() {
        "top" => Some("top"),
        "middle" | "center" | "centre" => Some("middle"),
        "bottom" => Some("bottom"),
        _ => None,
    }
}

/// `20m`, `20min`, `1h30m`, `90s`, `20` (minutes).
fn talk_duration(v: &str) -> Option<u32> {
    let v = v.trim().to_ascii_lowercase().replace(' ', "");
    if let Ok(min) = v.parse::<f64>() {
        return (min > 0.0).then_some((min * 60.0) as u32);
    }
    let mut total = 0.0;
    let mut num = String::new();
    let mut chars = v.chars().peekable();
    while let Some(c) = chars.next() {
        if c.is_ascii_digit() || c == '.' {
            num.push(c);
            continue;
        }
        let n: f64 = num.parse().ok()?;
        num.clear();
        // Skip the rest of the unit word ("min", "h", "sec").
        while chars.peek().is_some_and(|c| c.is_ascii_alphabetic()) {
            chars.next();
        }
        total += match c {
            'h' => n * 3600.0,
            'm' => n * 60.0,
            's' => n,
            _ => return None,
        };
    }
    if !num.is_empty() {
        total += num.parse::<f64>().ok()? * 60.0;
    }
    (total > 0.0).then_some(total as u32)
}

// ===== splitting ==========================================================

/// A slide's lines in the source.
struct RawSlide<'a> {
    /// Line of the `---` separator (0 for the first slide).
    start: usize,
    /// First line of the slide's content.
    body: usize,
    lines: Vec<&'a str>,
}

/// Opening/closing code fence of a line: (character, length).
fn fence_marker(line: &str) -> Option<(char, usize)> {
    let t = line.trim_start();
    if line.len() - t.len() > 3 {
        return None;
    }
    let c = t.chars().next().filter(|c| *c == '`' || *c == '~')?;
    let n = t.chars().take_while(|x| *x == c).count();
    (n >= 3).then_some((c, n))
}

/// Tracks fenced code blocks line by line.
#[derive(Default)]
struct Fences(Option<(char, usize)>);

impl Fences {
    /// Feeds a line; returns whether it is part of a code block (including the fences).
    fn code(&mut self, line: &str) -> bool {
        match (self.0, fence_marker(line)) {
            (Some((c, n)), Some((c2, n2))) if c == c2 && n2 >= n && line.trim().chars().all(|x| x == c) => {
                self.0 = None;
                true
            }
            (Some(_), _) => true,
            (None, Some(m)) => {
                self.0 = Some(m);
                true
            }
            (None, None) => false,
        }
    }
}

fn is_separator(line: &str) -> bool {
    line.trim_end() == "---"
}

/// The YAML front matter (if the source starts with one) and the index of
/// the first line after it.
fn front_matter(lines: &[&str]) -> (Option<serde_yaml::Mapping>, usize) {
    if !lines.first().is_some_and(|l| is_separator(l)) {
        return (None, 0);
    }
    let end = (1..lines.len()).find(|&i| is_separator(lines[i]) || lines[i].trim_end() == "...");
    if let Some(end) = end {
        if let Ok(serde_yaml::Value::Mapping(map)) = serde_yaml::from_str(&lines[1..end].join("\n")) {
            return (Some(map), end + 1);
        }
    }
    // Not front matter: a leading separator.
    (None, 1)
}

fn split(source: &str) -> (Option<serde_yaml::Mapping>, Vec<RawSlide<'_>>) {
    let lines: Vec<&str> = source.lines().collect();
    let (front, first) = front_matter(&lines);
    let mut slides = vec![RawSlide { start: 0, body: first, lines: Vec::new() }];
    let mut fences = Fences::default();
    for (i, line) in lines.iter().enumerate().skip(first) {
        if !fences.code(line) && is_separator(line) {
            slides.push(RawSlide { start: i, body: i + 1, lines: Vec::new() });
        } else if let Some(s) = slides.last_mut() {
            s.lines.push(line);
        }
    }
    let blank = |s: &RawSlide| s.lines.iter().all(|l| l.trim().is_empty());
    if slides.len() > 1 && slides.last().is_some_and(blank) {
        slides.pop();
    }
    if slides.len() > 1 && blank(&slides[0]) {
        slides.remove(0);
    }
    if slides.len() == 1 && blank(&slides[0]) {
        slides.clear();
    }
    (front, slides)
}

/// Whether a Markdown document declares itself as presentation in its front
/// matter (`presentation: true`; `marp: true` is accepted as well).
pub fn is_presentation(source: &str) -> bool {
    let lines: Vec<&str> = source.lines().take(60).collect();
    let (Some(map), _) = front_matter(&lines) else { return false };
    ["presentation", "slides", "marp"].iter().any(|k| {
        map.get(*k).is_some_and(|v| match v {
            serde_yaml::Value::Bool(b) => *b,
            serde_yaml::Value::String(s) => matches!(s.as_str(), "true" | "yes"),
            _ => false,
        })
    })
}

/// Start lines (separator line, first content line) of all slides.
pub fn slide_lines(source: &str) -> Vec<(usize, usize)> {
    split(source).1.iter().map(|s| (s.start, s.body)).collect()
}

/// Index of the slide containing `line`, given the result of [`slide_lines`].
pub fn slide_at(lines: &[(usize, usize)], line: usize) -> usize {
    lines.iter().rposition(|(start, _)| *start <= line).unwrap_or(0)
}

// ===== slides =============================================================

#[derive(Serialize, Clone, Debug)]
pub struct Slide {
    /// The complete `<section>` element.
    pub html: String,
    /// Presenter notes (HTML).
    pub notes: String,
    /// Transition used when this slide is entered.
    #[serde(rename = "t")]
    pub transition: Anim,
    /// Text of the first heading, shown in the presenter view.
    pub title: String,
}

#[derive(Serialize, Clone, Debug)]
pub struct Deck {
    pub title: String,
    #[serde(rename = "w")]
    pub width: u32,
    #[serde(rename = "h")]
    pub height: u32,
    /// Planned length in seconds.
    pub duration: Option<u32>,
    pub style: String,
    pub slides: Vec<Slide>,
    /// Source lines of each slide: (separator, first content line).
    #[serde(skip)]
    pub lines: Vec<(usize, usize)>,
}

impl Deck {
    pub fn json(&self) -> String {
        script_safe(&serde_json::to_string(self).unwrap_or_else(|_| "null".into()))
    }

    /// Page size in millimetres (CSS pixels at 96 dpi), for PDF export.
    pub fn page_mm(&self) -> (f64, f64) {
        (self.width as f64 * 25.4 / 96.0, self.height as f64 * 25.4 / 96.0)
    }
}

/// JSON embedded in a `<script>` must not contain `</`.
fn script_safe(json: &str) -> String {
    json.replace("</", "<\\/").replace("<!--", "<\\!--")
}

pub fn parse(source: &str) -> Deck {
    let (front, raw) = split(source);
    let mut cfg = Config::default();
    if let Some(map) = front {
        for (k, v) in map {
            if let (Some(k), Some(v)) = (yaml_text(&k), yaml_text(&v)) {
                cfg.set(&k.to_ascii_lowercase(), &v);
            }
        }
    }
    let lines = raw.iter().map(|s| (s.start, s.body)).collect();
    let slides: Vec<Slide> = raw.iter().enumerate().map(|(i, s)| slide(&cfg, i + 1, &s.lines)).collect();
    let title = cfg
        .title
        .clone()
        .or_else(|| slides.iter().map(|s| s.title.clone()).find(|t| !t.is_empty()))
        .unwrap_or_default();
    Deck {
        title,
        width: cfg.width,
        height: cfg.height,
        duration: cfg.duration,
        style: cfg.style.clone(),
        slides,
        lines,
    }
}

fn yaml_text(v: &serde_yaml::Value) -> Option<String> {
    match v {
        serde_yaml::Value::String(s) => Some(s.clone()),
        serde_yaml::Value::Number(n) => Some(n.to_string()),
        serde_yaml::Value::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}

/// `key: value` lines of an HTML comment, if all of them are slide directives.
/// A leading `_` (Marp's "this slide only") is accepted.
fn directives(comment: &str) -> Option<Vec<(String, String)>> {
    let mut out = Vec::new();
    for part in comment.split(['\n', ';']) {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        let (k, v) = part.split_once(':')?;
        let k = k.trim().trim_start_matches('_').to_ascii_lowercase();
        if !SLIDE_KEYS.contains(&k.as_str()) {
            return None;
        }
        out.push((k, v.trim().to_owned()));
    }
    (!out.is_empty()).then_some(out)
}

/// An open `:::` container.
struct Open {
    closing: String,
    columns: bool,
}

/// A background image (`![bg …](url)`).
struct Background {
    url: String,
    size: &'static str,
    side: Option<&'static str>,
    split: Option<String>,
}

fn slide(global: &Config, number: usize, lines: &[&str]) -> Slide {
    let mut cfg = global.clone();
    let mut body = String::new();
    let mut notes: Vec<String> = Vec::new();
    let mut in_notes = false;
    let mut fences = Fences::default();
    let mut stack: Vec<Open> = Vec::new();
    let mut group = 0;

    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];
        i += 1;
        if fences.code(line) {
            if in_notes {
                notes.push(line.to_owned());
            } else {
                body.push_str(line);
                body.push('\n');
            }
            continue;
        }
        let trimmed = line.trim();
        if !in_notes && trimmed == "???" {
            in_notes = true;
            continue;
        }
        // HTML comments at the start of a line: directives or notes.
        if trimmed.starts_with("<!--") {
            let mut text = trimmed.trim_start_matches("<!--").to_owned();
            let mut closed = text.contains("-->");
            while !closed && i < lines.len() {
                text.push('\n');
                text.push_str(lines[i]);
                closed = lines[i].contains("-->");
                i += 1;
            }
            let (inner, rest) = text.split_once("-->").unwrap_or((&text, ""));
            match directives(inner) {
                Some(list) => list.iter().for_each(|(k, v)| cfg.set(k, v)),
                None if !inner.trim().is_empty() => notes.push(inner.trim().to_owned()),
                None => {}
            }
            if !rest.trim().is_empty() {
                body.push_str(rest.trim());
                body.push('\n');
            }
            continue;
        }
        if in_notes {
            notes.push(line.to_owned());
            continue;
        }
        if let Some(args) = trimmed.strip_prefix(":::") {
            let args = args.trim_start_matches(':').trim();
            if args.is_empty() {
                if let Some(open) = stack.pop() {
                    body.push_str("\n\n");
                    body.push_str(&open.closing);
                    body.push_str("\n\n");
                }
            } else {
                group += 1;
                let (html, open) = container(args, group);
                body.push_str("\n\n");
                body.push_str(&html);
                body.push_str("\n\n");
                stack.push(open);
            }
            continue;
        }
        if trimmed == "|||" && stack.last().is_some_and(|o| o.columns) {
            body.push_str("\n\n</div>\n<div class=\"qn-col\">\n\n");
            continue;
        }
        body.push_str(line);
        body.push('\n');
    }
    while let Some(open) = stack.pop() {
        body.push_str("\n\n");
        body.push_str(&open.closing);
        body.push('\n');
    }

    let mut backgrounds = Vec::new();
    let content = render_markdown(&body, Some(&mut backgrounds));
    let title = html::first_heading(&content).unwrap_or_default();
    let notes_html = if notes.iter().all(|n| n.trim().is_empty()) {
        String::new()
    } else {
        markdown::to_html(&notes.join("\n\n"))
    };
    Slide {
        html: section(&cfg, number, &content, &backgrounds),
        notes: notes_html,
        transition: cfg.transition.clone(),
        title,
    }
}

/// Opening HTML of a `::: …` container and how to close it.
///
/// Words: `step` (appears as a whole), `steps` (children/list items appear one
/// by one), `dim` (dims earlier steps), an animation name, a duration or speed,
/// `delay=…`, `place`/`top`/`middle`/`bottom` (positioned on the slide) with
/// `left`/`center`/`right`, `x=… y=… w=…`, `columns` or a ratio like `2:1`,
/// `size=…` (font size) and other words as CSS classes.
fn container(args: &str, group: usize) -> (String, Open) {
    let mut place = false;
    let (mut v, mut h) = (None, None);
    let (mut x, mut y, mut w) = (None, None, None);
    let mut step: Option<&str> = None;
    let mut anim: Option<&str> = None;
    let (mut dur, mut delay) = (None, None);
    let mut dim = false;
    let mut columns: Option<String> = None;
    let mut size = None;
    let mut classes: Vec<String> = Vec::new();

    for token in args.split_whitespace() {
        let lower = token.to_ascii_lowercase();
        if let Some((key, value)) = lower.split_once('=') {
            match key {
                "x" | "left" => (x, place) = (Some(length(value, "%")), true),
                "y" | "top" => (y, place) = (Some(length(value, "%")), true),
                "w" | "width" => w = Some(length(value, "%")),
                "size" | "font" => size = Some(length(value, "em")),
                "delay" => delay = duration_ms(value).or_else(|| duration_ms(&format!("{value}ms"))),
                "dur" | "duration" | "speed" => dur = duration_ms(value),
                "anim" | "animation" => anim = animation_name(value, STEP_ANIMATIONS),
                _ => {}
            }
            continue;
        }
        match lower.as_str() {
            "place" | "at" => place = true,
            "top" | "middle" | "bottom" => {
                v = vertical(&lower);
                place = true;
            }
            "left" | "center" | "centre" | "right" => h = horizontal(&lower),
            "step" | "steps" | "fragment" | "fragments" => {
                step = Some(if lower.ends_with('s') { "steps" } else { "step" })
            }
            "dim" => dim = true,
            "columns" | "cols" => columns = columns.or(Some(String::new())),
            t if t.contains(':') && t.split(':').all(|p| p.parse::<u32>().is_ok_and(|n| n > 0)) => {
                columns = Some(t.split(':').map(|p| format!("{p}fr")).collect::<Vec<_>>().join(" "));
            }
            t => {
                if let Some(ms) = duration_ms(t) {
                    dur = Some(ms);
                } else if let Some(a) = animation_name(t, STEP_ANIMATIONS) {
                    anim = Some(a);
                } else {
                    let class: String = token
                        .trim_start_matches('.')
                        .chars()
                        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
                        .collect();
                    if !class.is_empty() {
                        classes.push(class);
                    }
                }
            }
        }
    }
    // An animation or duration alone makes the block a step.
    if step.is_none() && (anim.is_some() || dur.is_some() || dim) {
        step = Some("step");
    }

    let mut html = String::new();
    let mut closing = String::new();
    if place {
        let mut style = String::new();
        if x.is_some() || y.is_some() {
            style.push_str(&format!(
                "left:{};top:{};",
                x.as_deref().unwrap_or("auto"),
                y.as_deref().unwrap_or("auto")
            ));
            // Percentages refer to the slide here, not to the shrink-wrapped box.
            if let Some(w) = w.take() {
                style.push_str(&format!("width:{w};"));
            }
            html.push_str(&format!("<div class=\"qn-place qn-at\" style=\"{}\">\n", html::escape(&style)));
        } else {
            let justify = match h {
                Some("left") => "flex-start",
                Some("right") => "flex-end",
                _ => "center",
            };
            let align = match v {
                Some("top") => "flex-start",
                Some("bottom") => "flex-end",
                _ => "center",
            };
            style.push_str(&format!("justify-content:{justify};align-items:{align};"));
            if let Some(h) = h {
                style.push_str(&format!("text-align:{h};"));
            }
            html.push_str(&format!("<div class=\"qn-place\" style=\"{style}\">\n"));
        }
        closing.push_str("</div>\n");
    }

    let mut class = vec!["qn-box".to_owned()];
    if columns.is_some() {
        class.push("qn-columns".into());
    }
    class.extend(classes);
    let mut style = String::new();
    if !place {
        if let Some(h) = h {
            style.push_str(&format!("text-align:{h};"));
        }
    }
    if let Some(w) = &w {
        style.push_str(&format!("width:{w};"));
    }
    if let Some(s) = &size {
        style.push_str(&format!("font-size:{s};"));
    }
    if let Some(cols) = columns.as_deref().filter(|c| !c.is_empty()) {
        style.push_str(&format!("grid-template-columns:{cols};grid-auto-flow:row;"));
    }
    let mut attrs = format!("class=\"{}\"", html::escape(&class.join(" ")));
    if !style.is_empty() {
        attrs.push_str(&format!(" style=\"{}\"", html::escape(&style)));
    }
    if let Some(step) = step {
        attrs.push_str(&format!(" data-qn-{step}=\"\" data-group=\"{group}\""));
        if let Some(a) = anim {
            attrs.push_str(&format!(" data-anim=\"{a}\""));
        }
        if let Some(ms) = dur {
            attrs.push_str(&format!(" data-ms=\"{ms}\""));
        }
        if let Some(ms) = delay {
            attrs.push_str(&format!(" data-delay=\"{ms}\""));
        }
        if dim {
            attrs.push_str(" data-dim=\"\"");
        }
    }
    html.push_str(&format!("<div {attrs}>"));
    let mut inner_closing = String::from("</div>");
    if columns.is_some() {
        html.push_str("\n<div class=\"qn-col\">");
        inner_closing.insert_str(0, "</div>\n");
    }
    closing.insert_str(0, &format!("{inner_closing}\n"));
    (html, Open { closing, columns: columns.is_some() })
}

/// A CSS length; plain numbers get `unit`.
fn length(value: &str, unit: &str) -> String {
    let value = value.trim();
    let valid = value.chars().all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '%');
    if !valid || value.is_empty() {
        return "auto".into();
    }
    if value.parse::<f64>().is_ok() {
        format!("{value}{unit}")
    } else {
        value.to_owned()
    }
}

/// Renders Markdown; images get the size options from their alt text, and
/// `![bg …]` images are collected as slide backgrounds.
fn render_markdown(md: &str, mut backgrounds: Option<&mut Vec<Background>>) -> String {
    let opts = Options::ENABLE_TABLES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_FOOTNOTES
        | Options::ENABLE_HEADING_ATTRIBUTES;
    let mut events = Vec::new();
    let mut image: Option<(String, String, String)> = None;
    for event in Parser::new_ext(md, opts) {
        match event {
            Event::Start(Tag::Image { dest_url, title, .. }) => {
                image = Some((dest_url.into_string(), title.into_string(), String::new()));
            }
            Event::End(TagEnd::Image) => {
                if let Some((url, title, alt)) = image.take() {
                    let html = image_html(&url, &title, &alt, backgrounds.as_deref_mut());
                    events.push(Event::InlineHtml(html.into()));
                }
            }
            Event::Text(t) | Event::Code(t) if image.is_some() => {
                if let Some((_, _, alt)) = image.as_mut() {
                    alt.push_str(&t);
                }
            }
            _ if image.is_some() => {}
            other => events.push(other),
        }
    }
    let mut out = String::with_capacity(md.len() * 3 / 2);
    pulldown_cmark::html::push_html(&mut out, events.into_iter());
    out.replace("<p></p>\n", "")
}

fn resolve_url(url: &str) -> String {
    match url.strip_prefix("~/") {
        Some(rest) => {
            let home = dirs::home_dir().unwrap_or_default();
            format!("file://{}/{}", home.to_string_lossy().trim_end_matches('/'), rest)
        }
        None => url.to_owned(),
    }
}

/// `<img>` for an image; options in the alt text: `w=300` / `width:300px`,
/// `h=…`, `left`/`right`/`center`. `bg [contain|cover|auto] [left|right] [40%]`
/// makes it a background image (returns an empty string).
fn image_html(url: &str, title: &str, alt: &str, backgrounds: Option<&mut Vec<Background>>) -> String {
    let url = resolve_url(url);
    let mut tokens = alt.split_whitespace().peekable();
    if tokens.peek().is_some_and(|t| t.eq_ignore_ascii_case("bg")) {
        if let Some(list) = backgrounds {
            tokens.next();
            let mut bg = Background { url, size: "cover", side: None, split: None };
            for t in tokens {
                match t.to_ascii_lowercase().as_str() {
                    "contain" | "fit" => bg.size = "contain",
                    "cover" => bg.size = "cover",
                    "auto" | "original" => bg.size = "auto",
                    "left" => bg.side = Some("left"),
                    "right" => bg.side = Some("right"),
                    p if p.ends_with('%') && p[..p.len() - 1].parse::<f64>().is_ok() => bg.split = Some(p.to_owned()),
                    _ => {}
                }
            }
            list.push(bg);
            return String::new();
        }
    }
    let mut style = String::new();
    let mut words = Vec::new();
    for t in tokens {
        let lower = t.to_ascii_lowercase();
        let kv = lower.split_once('=').or_else(|| lower.split_once(':'));
        match kv {
            Some(("w" | "width", v)) => style.push_str(&format!("width:{};", length(v, "px"))),
            Some(("h" | "height", v)) => style.push_str(&format!("height:{};", length(v, "px"))),
            _ => match lower.as_str() {
                "center" => style.push_str("display:block;margin-left:auto;margin-right:auto;"),
                "left" => style.push_str("float:left;margin-right:1em;"),
                "right" => style.push_str("float:right;margin-left:1em;"),
                _ => words.push(t),
            },
        }
    }
    let mut html = format!("<img src=\"{}\" alt=\"{}\"", html::escape(&url), html::escape(&words.join(" ")));
    if !title.is_empty() {
        html.push_str(&format!(" title=\"{}\"", html::escape(title)));
    }
    if !style.is_empty() {
        html.push_str(&format!(" style=\"{style}\""));
    }
    html.push('>');
    html
}

/// Inline Markdown (header, footer) without the surrounding paragraph.
fn inline_markdown(text: &str) -> String {
    let html = render_markdown(text, None);
    let html = html.trim();
    html.strip_prefix("<p>").and_then(|h| h.strip_suffix("</p>")).unwrap_or(html).to_owned()
}

fn section(cfg: &Config, number: usize, content: &str, backgrounds: &[Background]) -> String {
    let mut classes = vec!["qn-slide".to_owned(), format!("qn-theme-{}", cfg.theme)];
    if let Some(c) = &cfg.class {
        classes.extend(
            c.split_whitespace()
                .map(|w| w.chars().filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_').collect()),
        );
    }
    if let Some(a) = &cfg.align {
        classes.push(format!("qn-align-{a}"));
    }
    if let Some(v) = &cfg.valign {
        classes.push(format!("qn-valign-{v}"));
    }
    let mut style = String::new();
    if let Some(bg) = &cfg.background {
        style.push_str(&format!("background:{bg};"));
    }
    if let Some(c) = &cfg.color {
        style.push_str(&format!("--qn-fg:{c};--qn-heading:{c};color:{c};"));
    }

    // Split backgrounds (`![bg left 40%]`) take one side, the content the other.
    let side = backgrounds.iter().find_map(|b| b.side);
    let mut bgs = String::new();
    if !backgrounds.is_empty() {
        if let Some(side) = side {
            classes.push(format!("qn-split-{side}"));
            let split = backgrounds.iter().find_map(|b| b.split.clone()).unwrap_or_else(|| "50%".into());
            style.push_str(&format!("--qn-split:{split};"));
        }
        bgs.push_str("<div class=\"qn-bgs\">");
        for b in backgrounds {
            let url = b.url.replace('\\', "%5C").replace('\'', "%27").replace(['\n', '\r'], "");
            bgs.push_str(&format!(
                "<div class=\"qn-bg\" style=\"{}\"></div>",
                html::escape(&format!("background-image:url('{url}');background-size:{};", b.size))
            ));
        }
        bgs.push_str("</div>");
    }

    let mut out = format!(
        "<section class=\"{}\" style=\"{}\" data-step-anim=\"{}\" data-step-ms=\"{}\">",
        html::escape(&classes.join(" ")),
        html::escape(&style),
        cfg.step.name,
        cfg.step.ms
    );
    out.push_str(&bgs);
    if let Some(h) = &cfg.header {
        out.push_str(&format!("<header class=\"qn-header\">{}</header>", inline_markdown(h)));
    }
    out.push_str("<div class=\"qn-content\">");
    out.push_str(content);
    out.push_str("</div>");
    if let Some(f) = &cfg.footer {
        out.push_str(&format!("<footer class=\"qn-footer\">{}</footer>", inline_markdown(f)));
    }
    if cfg.paginate {
        out.push_str(&format!("<div class=\"qn-page\">{number}</div>"));
    }
    out.push_str("</section>");
    out
}

// ===== pages ==============================================================

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mode {
    /// Side-by-side preview in the editor (one slide, follows the cursor).
    Preview,
    /// What the audience sees.
    Audience,
    /// Notes, timer, current and next slide.
    Presenter,
    /// Exported HTML file: audience view with keyboard navigation.
    Export,
    /// One slide per page, all steps shown (PDF export).
    Print,
}

impl Mode {
    fn name(self) -> &'static str {
        match self {
            Mode::Preview => "preview",
            Mode::Audience => "audience",
            Mode::Presenter => "presenter",
            Mode::Export => "export",
            Mode::Print => "print",
        }
    }
}

/// Translated texts used by the engine.
fn labels() -> String {
    let map = serde_json::json!({
        "slide": t!("Slide"),
        "noSlides": t!("No slides yet. Separate slides with a line containing only ---"),
        "notes": t!("Notes"),
        "noNotes": t!("No notes for this slide"),
        "next": t!("Next"),
        "nextStep": t!("Next step"),
        "nextSlide": t!("Next slide"),
        "end": t!("End of presentation"),
        "play": t!("Play the animations of this slide"),
        "oneStep": t!("1 step"),
        "steps": t!("{} steps"),
        "prev": t!("Previous slide"),
        "nextBtn": t!("Next slide"),
        "elapsed": t!("Elapsed time – click or T to pause, R to reset"),
        "remaining": t!("remaining"),
        "overtime": t!("over time"),
        "black": t!("Screen is black"),
        "white": t!("Screen is white"),
        "blank": t!("Black screen (B)"),
        "swap": t!("Swap screens (S)"),
        "exit": t!("End presentation (Esc)"),
        "smaller": t!("Smaller notes (−)"),
        "larger": t!("Larger notes (+)"),
        "help": t!("Keys (H)"),
        "goto": t!("Go to slide"),
        "keys": [
            [t!("→ ↓ Space PgDn Enter, click"), t!("Next step or slide")],
            [t!("← ↑ PgUp Backspace, right click"), t!("Previous step or slide")],
            [t!("Home / End"), t!("First / last slide")],
            [t!("Number + Enter"), t!("Go to slide")],
            [t!("B or .  /  W or ,"), t!("Black / white screen")],
            ["P", t!("Show/hide presenter view (one screen)")],
            ["S", t!("Swap screens")],
            ["F", t!("Toggle fullscreen")],
            ["T / R", t!("Pause / reset the timer")],
            ["+ / −", t!("Notes font size")],
            ["Esc", t!("End presentation")],
        ],
    });
    script_safe(&map.to_string())
}

/// Options for the presentation pages.
#[derive(Default, Clone, Copy)]
pub struct PageOptions {
    pub start: usize,
    /// Number of presentation windows (2 = audience and presenter on own screens).
    pub screens: usize,
    pub dark: bool,
}

/// A complete HTML page for `mode`. The preview page starts empty and is
/// filled with `QN.update(deck, slide)`.
pub fn page(mode: Mode, deck: Option<&Deck>, opts: PageOptions) -> String {
    let title = html::escape(deck.map(|d| d.title.as_str()).filter(|t| !t.is_empty()).unwrap_or("Presentation"));
    let user_style = deck.map(|d| d.style.replace("</", "<\\/")).unwrap_or_default();
    let theme = if opts.dark { "dark" } else { "light" };
    let deck_json = deck.map(Deck::json).unwrap_or_else(|| "null".into());
    let options = serde_json::json!({ "start": opts.start, "screens": opts.screens });
    let print_css = match (mode, deck) {
        (Mode::Print, Some(d)) => format!("@page {{ size: {}px {}px; margin: 0; }}", d.width, d.height),
        _ => String::new(),
    };
    format!(
        r#"<!DOCTYPE html>
<html data-mode="{mode}" data-theme="{theme}">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<meta name="generator" content="QuickNotes">
<title>{title}</title>
<style>{DECK_CSS}{print_css}</style>
<style id="qn-user-style">{user_style}</style>
</head>
<body>
<script>{ENGINE_JS}</script>
<script>QN.init("{mode}", {deck_json}, {labels}, {options});</script>
</body>
</html>
"#,
        mode = mode.name(),
        labels = labels(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_slides_outside_code() {
        let src = "# A\n---\n# B\n```\n---\n```\n---\n";
        let deck = parse(src);
        assert_eq!(deck.slides.len(), 2);
        assert!(deck.slides[1].html.contains("<pre><code>---"));
        assert_eq!(deck.lines, vec![(0, 0), (1, 2)]);
    }

    #[test]
    fn reads_front_matter() {
        let src = "---\npresentation: true\ntheme: dark\ntransition: zoom 0.8s\naspect: 4:3\nduration: 1h30m\n---\n# Hi\n";
        assert!(is_presentation(src));
        let deck = parse(src);
        assert_eq!(deck.slides.len(), 1);
        assert_eq!((deck.width, deck.height), (960, 720));
        assert_eq!(deck.slides[0].transition, Anim::new("zoom", 800));
        assert!(deck.slides[0].html.contains("qn-theme-dark"));
        assert_eq!(deck.duration, Some(5400));
        assert_eq!(deck.title, "Hi");
        assert!(!is_presentation("---\ntitle: x\n---\n"));
    }

    #[test]
    fn leading_separator_is_no_front_matter() {
        let deck = parse("---\n# A\n---\n# B");
        assert_eq!(deck.slides.len(), 2);
        assert_eq!(deck.lines[0], (0, 1));
    }

    #[test]
    fn directives_and_notes() {
        let src = "<!-- transition: slide-up fast; background: #123 -->\n# T\n<!-- a note -->\n???\nMore *notes*\n";
        let s = &parse(src).slides[0];
        assert_eq!(s.transition, Anim::new("slide-up", 300));
        assert!(s.html.contains("background:#123"));
        assert!(s.notes.contains("a note"));
        assert!(s.notes.contains("<em>notes</em>"));
        assert!(!s.html.contains("notes"));
    }

    #[test]
    fn containers() {
        let src = "::: steps fade-up 0.3s dim\n- a\n- b\n:::\n::: place bottom right\nX\n:::\n::: columns 2:1\nL\n|||\nR\n:::";
        let html = &parse(src).slides[0].html;
        assert!(html.contains("data-qn-steps=\"\""));
        assert!(html.contains("data-anim=\"fade-up\""));
        assert!(html.contains("data-ms=\"300\""));
        assert!(html.contains("data-dim"));
        assert!(html.contains("<li>a</li>"));
        assert!(html.contains("justify-content:flex-end;align-items:flex-end"));
        assert!(html.contains("grid-template-columns:2fr 1fr"));
        assert_eq!(html.matches("class=\"qn-col\"").count(), 2);
        assert_eq!(html.matches("<div").count(), html.matches("</div>").count());
    }

    #[test]
    fn anim_word_makes_a_step() {
        let html = &parse("::: zoom\n![](a.png)\n:::").slides[0].html;
        assert!(html.contains("data-qn-step=\"\""));
        assert!(html.contains("data-anim=\"zoom\""));
    }

    #[test]
    fn images() {
        let s = &parse("![bg right 40%](b.jpg)\n\n![w=300 center Logo](~/x.png)").slides[0];
        assert!(s.html.contains("qn-split-right"));
        assert!(s.html.contains("--qn-split:40%"));
        assert!(s.html.contains("background-image:url(&#39;b.jpg&#39;)") || s.html.contains("url('b.jpg')"));
        assert!(s.html.contains("width:300px"));
        assert!(s.html.contains("alt=\"Logo\""));
        assert!(!s.html.contains("<p></p>"));
    }

    #[test]
    fn durations() {
        assert_eq!(duration_ms("0.5s"), Some(500));
        assert_eq!(duration_ms("250ms"), Some(250));
        assert_eq!(duration_ms("slow"), Some(1000));
        assert_eq!(duration_ms("fade"), None);
        assert_eq!(talk_duration("20m"), Some(1200));
        assert_eq!(talk_duration("20"), Some(1200));
        assert_eq!(talk_duration("1h 15min"), Some(4500));
    }

    #[test]
    fn slide_at_cursor() {
        let lines = slide_lines("---\ntheme: dark\n---\n# A\n---\n# B\n");
        assert_eq!(lines, vec![(0, 3), (4, 5)]);
        assert_eq!(slide_at(&lines, 1), 0);
        assert_eq!(slide_at(&lines, 4), 1);
        assert_eq!(slide_at(&lines, 9), 1);
    }

    #[test]
    fn page_is_script_safe() {
        let deck = parse("# </script><script>alert(1)</script>");
        let page = page(Mode::Audience, Some(&deck), PageOptions::default());
        assert_eq!(page.matches("</script>").count(), 2);
    }
}
