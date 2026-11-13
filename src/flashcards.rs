//! Flash cards in Markdown (the syntax of Obsidian's Spaced Repetition
//! plugin, plus multiple choice):
//!
//! - `Question::Answer` – a card on one line; `:::` makes two cards (both
//!   directions).
//! - Lines of a question, a line `?` and lines of the answer – a card over
//!   several lines (`??` for both directions). A blank line ends it.
//! - An answer of `- [x]` / `- [ ]` items is a multiple-choice card with
//!   check boxes, `- (x)` / `- ( )` items one with radio buttons (exactly one
//!   right). Lines after the items explain the answer.
//!
//! The front matter (`flashcards: true`) can set the deck (`deck: A/B`),
//! headings form sub-decks below it. Progress is kept by
//! [`crate::learning`]; this module parses documents and renders the deck
//! overview and the pages of a study session.

use crate::learning::{self, CardState, Counts, LogStats, Store};
use crate::{html, markdown};
use serde::Serialize;
use std::collections::HashSet;

pub const LANGUAGE: &str = "flashcards";

#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    /// Question and answer.
    Basic,
    /// The reverse side of a two-way card (answer asked).
    Reverse,
    /// Multiple choice.
    Choice,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct Choice {
    /// Markdown.
    pub text: String,
    pub correct: bool,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct Card {
    pub key: String,
    /// Deck path, from the top deck down.
    pub deck: Vec<String>,
    /// Source line (0-based) of the question.
    pub line: usize,
    pub kind: Kind,
    /// Markdown of question and answer (for choice cards: the explanation).
    pub front: String,
    pub back: String,
    pub choices: Vec<Choice>,
    /// Check boxes (any number right) instead of radio buttons.
    pub multiple: bool,
}

impl Card {
    /// The question as plain text (for recognising edited cards).
    pub fn front_text(&self) -> String {
        self.front.split_whitespace().collect::<Vec<_>>().join(" ")
    }

    pub fn deck_path(&self) -> String {
        self.deck.join("/")
    }
}

/// Settings from the front matter.
#[derive(Clone, Debug, PartialEq)]
pub struct Settings {
    pub title: Option<String>,
    pub deck: Vec<String>,
    /// `auto`, `light`, `dark`, `sepia`, `night`, `contrast`.
    pub theme: String,
    pub accent: Option<String>,
    pub background: Option<String>,
    pub color: Option<String>,
    /// New cards per day and deck.
    pub new_per_day: usize,
}

impl Default for Settings {
    fn default() -> Self {
        Self { title: None, deck: Vec::new(), theme: "auto".into(), accent: None, background: None, color: None, new_per_day: 20 }
    }
}

/// A parsed flash card document.
#[derive(Debug, Default)]
pub struct Doc {
    pub settings: Settings,
    pub cards: Vec<Card>,
    /// Line (0-based) and description of cards with mistakes.
    pub problems: Vec<(usize, String)>,
}

/// Whether a Markdown document declares itself as flash cards
/// (`flashcards: true` in the front matter).
pub fn is_flashcards(source: &str) -> bool {
    let lines: Vec<&str> = source.lines().take(60).collect();
    let (Some(map), _) = crate::slides::front_matter(&lines) else { return false };
    ["flashcards", "flash-cards", "cards"].iter().any(|k| {
        map.get(*k).is_some_and(|v| match v {
            serde_yaml::Value::Bool(b) => *b,
            serde_yaml::Value::String(s) => matches!(s.as_str(), "true" | "yes"),
            _ => false,
        })
    })
}

/// A new flash card document with an example of every card type.
pub fn template() -> String {
    format!(
        "---\nflashcards: true\ndeck: {deck}\n---\n\n# {basics}\n\n{q1}::{a1}\n\nMitochondria:::{a2}\n\n{q3}\n?\n{a3}\n\n# {choice}\n\n{q4}\n?\n- [x] Mitochondrion\n- [x] Ribosome\n- [ ] {wall}\n\n{q5}\n?\n- ( ) Mars\n- (x) Jupiter\n- ( ) Saturn\n{expl}\n",
        deck = t!("Examples"),
        basics = t!("Basics"),
        q1 = t!("Capital of France"),
        a1 = t!("Paris"),
        a2 = t!("Powerhouse of the cell"),
        q3 = t!("What is osmosis?"),
        a3 = t!("Water moving through a membrane\nfrom low to high concentration."),
        choice = t!("Multiple choice"),
        q4 = t!("Which are organelles?"),
        wall = t!("Cell wall"),
        q5 = t!("The largest planet?"),
        expl = t!("Jupiter has more than twice the mass of all other planets together."),
    )
}

// ----- parsing -------------------------------------------------------------------

fn yaml_text(v: &serde_yaml::Value) -> Option<String> {
    match v {
        serde_yaml::Value::String(s) => Some(s.trim().to_owned()),
        serde_yaml::Value::Number(n) => Some(n.to_string()),
        _ => None,
    }
}

fn settings(front: Option<&serde_yaml::Mapping>) -> Settings {
    let mut s = Settings::default();
    let Some(front) = front else { return s };
    let get = |keys: &[&str]| keys.iter().find_map(|k| front.get(*k).and_then(yaml_text)).filter(|v| !v.is_empty());
    s.title = get(&["title"]);
    s.deck = get(&["deck"]).map(|d| split_path(&d)).unwrap_or_default();
    if let Some(t) = get(&["theme"]) {
        s.theme = t.to_lowercase();
    }
    s.accent = get(&["accent", "accent-color"]);
    s.background = get(&["background", "background-color"]);
    s.color = get(&["color", "text-color"]);
    if let Some(n) = get(&["new-per-day", "new_per_day", "new"]).and_then(|n| n.parse().ok()) {
        s.new_per_day = n;
    }
    s
}

fn split_path(path: &str) -> Vec<String> {
    path.split('/').map(str::trim).filter(|p| !p.is_empty()).map(str::to_owned).collect()
}

/// Level and text of a Markdown heading line.
fn heading(line: &str) -> Option<(usize, String)> {
    let level = line.chars().take_while(|c| *c == '#').count();
    if !(1..=6).contains(&level) {
        return None;
    }
    let rest = &line[level..];
    if !rest.starts_with([' ', '\t']) && !rest.is_empty() {
        return None;
    }
    let mut text = rest.trim().trim_end_matches('#').trim().to_owned();
    if let Some(i) = text.rfind(" {#").filter(|_| text.ends_with('}')) {
        text.truncate(i);
    }
    Some((level, text))
}

/// Byte offset of `sep` in `line` outside inline code.
fn find_outside_code(line: &str, sep: &str) -> Option<usize> {
    let mut in_code = false;
    let bytes = line.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'`' {
            in_code = !in_code;
        } else if !in_code && line[i..].starts_with(sep) && (i == 0 || bytes[i - 1] != b'\\') {
            return Some(i);
        }
        i += 1;
    }
    None
}

/// A list marker at the start of a line (`- `, `* `, `1. `).
fn strip_list_marker(text: &str) -> &str {
    let t = text.trim_start();
    for m in ["- ", "* ", "+ "] {
        if let Some(rest) = t.strip_prefix(m) {
            return rest;
        }
    }
    let digits = t.chars().take_while(char::is_ascii_digit).count();
    if digits > 0 {
        if let Some(rest) = t[digits..].strip_prefix(". ") {
            return rest;
        }
    }
    t
}

/// A choice item: `- [x] text`, `- ( ) text`. Returns (correct, check box, text).
fn choice_item(line: &str) -> Option<(bool, bool, &str)> {
    let t = line.trim_start();
    let t = ["- ", "* ", "+ "].iter().find_map(|m| t.strip_prefix(m))?;
    let (open, close, multiple) = match t.chars().next()? {
        '[' => ('[', ']', true),
        '(' => ('(', ')', false),
        _ => return None,
    };
    let mut chars = t.chars();
    chars.next();
    let mark = chars.next()?;
    if chars.next()? != close || !matches!(mark, ' ' | 'x' | 'X') {
        return None;
    }
    let _ = open;
    let text = t[3..].trim();
    Some((mark != ' ', multiple, text))
}

/// FNV-1a: a hash that stays the same across program versions.
fn fnv(text: &str) -> String {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in text.bytes() {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    format!("{h:016x}")
}

struct Parser<'a> {
    lines: &'a [&'a str],
    doc: Doc,
    deck: Vec<String>,
}

impl Parser<'_> {
    fn push(&mut self, kind: Kind, line: usize, front: String, back: String, choices: Vec<Choice>, multiple: bool) {
        let tag = match kind {
            Kind::Basic => "b",
            Kind::Reverse => "r",
            Kind::Choice => "c",
        };
        let normalized = front.split_whitespace().collect::<Vec<_>>().join(" ");
        let base = fnv(&format!("{tag}\u{1}{normalized}"));
        let mut key = base.clone();
        let mut n = 1;
        while self.doc.cards.iter().any(|c| c.key == key) {
            n += 1;
            key = format!("{base}~{n}");
        }
        self.doc.cards.push(Card { key, deck: self.deck.clone(), line, kind, front, back, choices, multiple });
    }

    /// A block of lines between blank lines.
    fn block(&mut self, block: &[usize]) {
        if block.is_empty() {
            return;
        }
        // A `?` / `??` line splits question and answer (outside code blocks).
        let mut fence = false;
        let mut sep = None;
        for (pos, &i) in block.iter().enumerate() {
            let t = self.lines[i].trim();
            if t.starts_with("```") || t.starts_with("~~~") {
                fence = !fence;
            } else if !fence && (t == "?" || t == "??") {
                sep = Some((pos, t == "??"));
                break;
            }
        }
        if let Some((pos, both)) = sep {
            let question: Vec<&str> = block[..pos].iter().map(|&i| self.lines[i]).collect();
            let answer: Vec<&str> = block[pos + 1..].iter().map(|&i| self.lines[i]).collect();
            let line = block[0];
            if question.is_empty() || question.iter().all(|l| l.trim().is_empty()) {
                self.doc.problems.push((block[pos], t!("A card without a question").into()));
                return;
            }
            let front = question.join("\n").trim().to_owned();
            self.multiline(line, front, &answer, both);
            return;
        }
        let mut fence = false;
        for &i in block {
            let line = self.lines[i];
            let t = line.trim();
            if t.starts_with("```") || t.starts_with("~~~") {
                fence = !fence;
                continue;
            }
            if fence {
                continue;
            }
            let (pos, len, both) = match (find_outside_code(line, ":::"), find_outside_code(line, "::")) {
                (Some(p), _) => (p, 3, true),
                (None, Some(p)) => (p, 2, false),
                _ => continue,
            };
            let front = strip_list_marker(&line[..pos]).trim().to_owned();
            let back = line[pos + len..].trim().to_owned();
            if front.is_empty() || back.is_empty() {
                continue;
            }
            self.push(Kind::Basic, i, front.clone(), back.clone(), Vec::new(), false);
            if both {
                self.push(Kind::Reverse, i, back, front, Vec::new(), false);
            }
        }
    }

    fn multiline(&mut self, line: usize, front: String, answer: &[&str], both: bool) {
        // Leading choice items (indented lines continue an item).
        let mut choices: Vec<Choice> = Vec::new();
        let mut multiple = false;
        let mut rest = 0;
        for (n, l) in answer.iter().enumerate() {
            if let Some((correct, check, text)) = choice_item(l) {
                multiple |= check;
                choices.push(Choice { text: text.to_owned(), correct });
                rest = n + 1;
            } else if !choices.is_empty() && l.starts_with([' ', '\t']) {
                if let Some(c) = choices.last_mut() {
                    c.text.push(' ');
                    c.text.push_str(l.trim());
                }
                rest = n + 1;
            } else {
                break;
            }
        }
        if choices.len() >= 2 {
            let correct = choices.iter().filter(|c| c.correct).count();
            if correct == 0 {
                self.doc.problems.push((line, t!("No answer is marked as right ([x] or (x))").into()));
            } else if !multiple && correct > 1 {
                self.doc.problems.push((line, t!("Radio buttons ( ) allow only one right answer; use [ ] for several").into()));
            }
            let explanation = answer[rest..].join("\n").trim().to_owned();
            self.push(Kind::Choice, line, front, explanation, choices, multiple);
            return;
        }
        let back = answer.join("\n").trim().to_owned();
        if back.is_empty() {
            self.doc.problems.push((line, t!("A card without an answer").into()));
            return;
        }
        self.push(Kind::Basic, line, front.clone(), back.clone(), Vec::new(), false);
        if both {
            self.push(Kind::Reverse, line, back, front, Vec::new(), false);
        }
    }
}

/// Parses a flash card document. `fallback_deck` names the top deck when
/// the front matter sets neither `deck` nor `title`.
pub fn parse(source: &str, fallback_deck: &str) -> Doc {
    let lines: Vec<&str> = source.lines().collect();
    let (front, first) = crate::slides::front_matter(&lines);
    let settings = settings(front.as_ref());
    let root = if !settings.deck.is_empty() {
        settings.deck.clone()
    } else {
        vec![settings.title.clone().unwrap_or_else(|| fallback_deck.to_owned())]
    };
    let mut p = Parser { lines: &lines, doc: Doc { settings, ..Default::default() }, deck: root.clone() };
    let mut headings: Vec<(usize, String)> = Vec::new();
    let mut block: Vec<usize> = Vec::new();
    let mut fence: Option<String> = None;
    for (i, line) in lines.iter().enumerate().skip(first) {
        let t = line.trim_start();
        if let Some(f) = &fence {
            block.push(i);
            if t.starts_with(f.as_str()) {
                fence = None;
            }
            continue;
        }
        if t.starts_with("```") || t.starts_with("~~~") {
            fence = Some(t.chars().take_while(|c| *c == '`' || *c == '~').collect());
            block.push(i);
            continue;
        }
        if t.is_empty() {
            p.block(&std::mem::take(&mut block));
            continue;
        }
        if let Some((level, text)) = heading(line) {
            p.block(&std::mem::take(&mut block));
            headings.retain(|(l, _)| *l < level);
            headings.push((level, text));
            p.deck = root.iter().cloned().chain(headings.iter().map(|(_, t)| t.clone())).filter(|t| !t.is_empty()).collect();
            continue;
        }
        block.push(i);
    }
    p.block(&block);
    p.doc
}

// ----- decks ---------------------------------------------------------------------

/// A deck with its sub-decks; counts include the sub-decks.
#[derive(Serialize, Debug, Clone, Default)]
pub struct DeckNode {
    pub name: String,
    pub path: String,
    pub counts: Counts,
    pub children: Vec<DeckNode>,
}

impl DeckNode {
    fn insert(&mut self, path: &[String], prefix: String, state: Option<&CardState>, today: i64) {
        self.counts.add(state, today);
        let Some((first, rest)) = path.split_first() else { return };
        let child_path = if prefix.is_empty() { first.clone() } else { format!("{prefix}/{first}") };
        let i = match self.children.iter().position(|c| &c.name == first) {
            Some(i) => i,
            None => {
                self.children.push(DeckNode { name: first.clone(), path: child_path.clone(), ..Default::default() });
                self.children.len() - 1
            }
        };
        self.children[i].insert(rest, child_path, state, today);
    }
}

/// The deck tree of cards with their states.
pub fn tree(cards: &[(&Card, Option<&CardState>)], today: i64) -> DeckNode {
    let mut root = DeckNode::default();
    for (card, state) in cards {
        root.insert(&card.deck, String::new(), *state, today);
    }
    root
}

/// Whether `card` is in the deck `path` or one of its sub-decks.
pub fn in_deck(card: &Card, path: &str) -> bool {
    let deck = card.deck_path();
    path.is_empty() || deck == path || deck.starts_with(&format!("{path}/"))
}

// ----- theme ---------------------------------------------------------------------

type Rgb = (f64, f64, f64);

fn parse_color(s: &str) -> Option<Rgb> {
    let hex = s.trim().strip_prefix('#')?;
    let hex: String = match hex.len() {
        3 => hex.chars().flat_map(|c| [c, c]).collect(),
        6 => hex.to_owned(),
        _ => return None,
    };
    let v = u32::from_str_radix(&hex, 16).ok()?;
    Some((f64::from((v >> 16) & 255), f64::from((v >> 8) & 255), f64::from(v & 255)))
}

fn hex((r, g, b): Rgb) -> String {
    format!("#{:02x}{:02x}{:02x}", r.round() as u8, g.round() as u8, b.round() as u8)
}

fn luminance((r, g, b): Rgb) -> f64 {
    let ch = |c: f64| {
        let c = c / 255.0;
        if c <= 0.039_28 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * ch(r) + 0.7152 * ch(g) + 0.0722 * ch(b)
}

/// WCAG contrast ratio (1 to 21).
pub fn contrast(a: &str, b: &str) -> f64 {
    match (parse_color(a), parse_color(b)) {
        (Some(a), Some(b)) => {
            let (la, lb) = (luminance(a), luminance(b));
            (la.max(lb) + 0.05) / (la.min(lb) + 0.05)
        }
        _ => 1.0,
    }
}

/// `fg`, darkened or lightened until it has at least `min` contrast to `bg`.
pub fn readable(fg: &str, bg: &str, min: f64) -> String {
    let (Some(f), Some(b)) = (parse_color(fg), parse_color(bg)) else { return fg.to_owned() };
    if contrast(fg, bg) >= min {
        return hex(f);
    }
    let target: Rgb = if luminance(b) > 0.4 { (0.0, 0.0, 0.0) } else { (255.0, 255.0, 255.0) };
    for step in 1..=20 {
        let t = f64::from(step) / 20.0;
        let mixed = (f.0 + (target.0 - f.0) * t, f.1 + (target.1 - f.1) * t, f.2 + (target.2 - f.2) * t);
        if contrast(&hex(mixed), bg) >= min {
            return hex(mixed);
        }
    }
    hex(target)
}

/// Colours of the study window.
#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct Colors {
    pub bg: String,
    pub card: String,
    pub fg: String,
    pub muted: String,
    pub accent: String,
    /// Text on the accent colour.
    pub on_accent: String,
    pub good: String,
    pub bad: String,
    pub border: String,
}

/// The colours for a theme name (`dark_app`: the app uses a dark theme),
/// with every text colour readable on its background.
pub fn colors(settings: &Settings, dark_app: bool) -> Colors {
    let theme = match settings.theme.as_str() {
        "auto" | "" => if dark_app { "dark" } else { "light" },
        t => t,
    };
    let (bg, card, fg, accent) = match theme {
        "dark" => ("#1e1e1e", "#2a2a2a", "#e6e6e6", "#78aeed"),
        "sepia" => ("#f1e7d0", "#fbf5e6", "#3b2f1e", "#8a5a1c"),
        "night" => ("#0f1b2d", "#17263d", "#e3ecf7", "#6cb4ff"),
        "contrast" => ("#000000", "#000000", "#ffffff", "#ffd400"),
        _ => ("#f4f5f7", "#ffffff", "#1f2328", "#1a5fb4"),
    };
    let bg = settings.background.as_deref().and_then(parse_color).map(hex).unwrap_or_else(|| bg.to_owned());
    let card = if settings.background.is_some() { bg.clone() } else { card.to_owned() };
    let fg = readable(settings.color.as_deref().unwrap_or(fg), &card, 7.0);
    let accent = readable(settings.accent.as_deref().filter(|a| parse_color(a).is_some()).unwrap_or(accent), &card, 3.0);
    let on_accent = if contrast("#ffffff", &accent) >= contrast("#111111", &accent) { "#ffffff" } else { "#111111" };
    let dark = parse_color(&card).is_some_and(|c| luminance(c) < 0.4);
    let (good, bad) = if dark { ("#57e389", "#ff7b72") } else { ("#1a7f37", "#c01c28") };
    let mid = |a: &str, b: &str| {
        let (a, b) = (parse_color(a).unwrap_or((0.0, 0.0, 0.0)), parse_color(b).unwrap_or((0.0, 0.0, 0.0)));
        hex(((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0, (a.2 + b.2) / 2.0))
    };
    Colors {
        muted: readable(&mid(&fg, &card), &card, 4.5),
        border: mid(&mid(&fg, &card), &card),
        good: readable(good, &card, 4.5),
        bad: readable(bad, &card, 4.5),
        on_accent: on_accent.to_owned(),
        bg,
        card,
        fg,
        accent,
    }
}

// ----- study session ---------------------------------------------------------------

/// A card as shown in the study window.
#[derive(Serialize, Debug)]
pub struct StudyCard {
    pub key: String,
    pub deck: String,
    pub kind: Kind,
    pub front: String,
    pub back: String,
    pub choices: Vec<(String, bool)>,
    pub multiple: bool,
    /// When the card would come back after each answer (learn mode).
    pub next: Vec<String>,
    /// Plain question text (sent back with the answer).
    pub text: String,
}

/// The cards to study in the deck `path`: due cards first, then new ones
/// (at most `new_per_day` a day); in practice mode all of them, shuffled.
pub fn session_cards<'a>(
    cards: &[(&'a Card, Option<&CardState>)],
    path: &str,
    practice: bool,
    new_today: usize,
    new_per_day: usize,
    today: i64,
) -> Vec<&'a Card> {
    let in_deck: Vec<&(&Card, Option<&CardState>)> = cards.iter().filter(|(c, _)| in_deck(c, path)).collect();
    if practice {
        let mut all: Vec<&Card> = in_deck.iter().map(|(c, _)| *c).collect();
        shuffle(&mut all);
        return all;
    }
    let mut due: Vec<(&Card, i64)> = in_deck.iter().filter_map(|(c, s)| s.filter(|s| s.due <= today).map(|s| (*c, s.due))).collect();
    due.sort_by_key(|(_, d)| *d);
    let mut out: Vec<&Card> = due.into_iter().map(|(c, _)| c).collect();
    shuffle(&mut out);
    let new_left = new_per_day.saturating_sub(new_today);
    out.extend(in_deck.iter().filter(|(_, s)| s.is_none()).map(|(c, _)| *c).take(new_left));
    out
}

/// Shuffles in place (random order for every session).
fn shuffle<T>(items: &mut [T]) {
    let mut seed = [0u8; 8];
    let _ = getrandom::getrandom(&mut seed);
    let mut x = u64::from_le_bytes(seed) | 1;
    for i in (1..items.len()).rev() {
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        items.swap(i, (x % (i as u64 + 1)) as usize);
    }
}

/// How long until a card comes back, e.g. "10 min", "3 d", "2 mo".
fn interval_text(days: f64) -> String {
    if days < 1.0 {
        t!("now").into()
    } else if days < 30.0 {
        t!("{} d", days.round())
    } else if days < 365.0 {
        t!("{} mo", (days / 30.0).round())
    } else {
        t!("{} y", ((days / 365.0) * 10.0).round() / 10.0)
    }
}

/// The page data of a study session.
pub fn study_card(card: &Card, state: Option<&CardState>, practice: bool, today: i64) -> StudyCard {
    let next = if practice {
        Vec::new()
    } else {
        (learning::AGAIN..=learning::EASY).map(|r| interval_text(learning::schedule(state, r, today, 0).interval)).collect()
    };
    StudyCard {
        key: card.key.clone(),
        deck: card.deck.join(" › "),
        kind: card.kind,
        front: markdown::to_html(&card.front),
        back: markdown::to_html(&card.back),
        choices: card.choices.iter().map(|c| (inline(&c.text), c.correct)).collect(),
        multiple: card.multiple,
        next,
        text: card.front_text(),
    }
}

/// Markdown of a single line, without the paragraph around it.
fn inline(text: &str) -> String {
    let html = markdown::to_html(text);
    let trimmed = html.trim();
    trimmed.strip_prefix("<p>").and_then(|h| h.strip_suffix("</p>")).unwrap_or(trimmed).to_owned()
}

const STUDY_CSS: &str = include_str!("../data/flashcards/study.css");
const STUDY_JS: &str = include_str!("../data/flashcards/study.js");

/// Texts of the study window.
fn study_labels() -> serde_json::Value {
    serde_json::json!({
        "show": t!("Show Answer"),
        "check": t!("Check"),
        "ratings": [t!("Again"), t!("Hard"), t!("Good"), t!("Easy")],
        "right": t!("Right"),
        "wrong": t!("Wrong"),
        "left": t!("{} left"),
        "done": t!("Done!"),
        "noCards": t!("No cards to study right now."),
        "summary": t!("{} cards · {} right at the first try · {}"),
        "repeat": t!("Practise the Wrong Ones"),
        "close": t!("Close"),
        "keys": t!("Space: show answer · 1–4: rate · Esc: close · F11: full screen"),
        "practice": t!("Practice"),
    })
}

/// The page of a study session.
pub fn study_page(title: &str, cards: &[StudyCard], colors: &Colors, practice: bool) -> String {
    let data = serde_json::json!({
        "title": title,
        "cards": cards,
        "practice": practice,
        "labels": study_labels(),
    });
    let json = serde_json::to_string(&data).unwrap_or_default().replace("</", "<\\/");
    let all: String = cards.iter().map(|c| format!("{}{}", c.front, c.back)).collect();
    let libs = crate::notation::Libs::needed(&all).head();
    let vars = format!(
        "--bg:{};--card:{};--fg:{};--muted:{};--accent:{};--on-accent:{};--good:{};--bad:{};--border:{}",
        colors.bg, colors.card, colors.fg, colors.muted, colors.accent, colors.on_accent, colors.good, colors.bad, colors.border
    );
    let title = html::escape(title);
    format!(
        "<!DOCTYPE html>\n<html style=\"{vars}\">\n<head>\n<meta charset=\"utf-8\">\n<title>{title}</title>\n<style>{STUDY_CSS}</style>\n{libs}</head>\n<body>\n<div id=\"app\"></div>\n<script>window.QF_DATA = {json};</script>\n<script>{STUDY_JS}</script>\n</body>\n</html>"
    )
}

// ----- overview --------------------------------------------------------------------

/// A document's cards with their stored states.
pub struct Studied<'a> {
    pub doc: &'a Doc,
    pub states: Vec<Option<&'a CardState>>,
}

impl<'a> Studied<'a> {
    pub fn new(doc: &'a Doc, doc_id: &str, store: &'a Store) -> Self {
        let texts: Vec<String> = doc.cards.iter().map(Card::front_text).collect();
        let refs: Vec<learning::CardRef<'_>> =
            doc.cards.iter().zip(&texts).map(|(c, t)| learning::CardRef { key: &c.key, front: t }).collect();
        let states = store.resolve(doc_id, &refs).into_iter().map(|s| s.map(|(_, st)| st)).collect();
        Self { doc, states }
    }

    pub fn pairs(&self) -> Vec<(&'a Card, Option<&'a CardState>)> {
        self.doc.cards.iter().zip(self.states.iter().copied()).collect()
    }
}

fn minutes(ms: u64) -> String {
    if ms < 60_000 {
        return t!("{} s", ms / 1000);
    }
    let min = (ms as f64 / 60_000.0).round() as u64;
    if min < 60 {
        t!("{} min", min)
    } else {
        t!("{} h {} min", min / 60, min % 60)
    }
}

fn deck_html(node: &DeckNode, out: &mut String, depth: usize) {
    let c = &node.counts;
    let pct = |n: usize| if c.total == 0 { 0.0 } else { n as f64 * 100.0 / c.total as f64 };
    let learned = c.young + c.mature;
    let path = html::escape(&node.path);
    out.push_str(&format!(
        "<div class=\"qf-deck\" style=\"--depth:{depth}\"><div class=\"qf-row\"><div class=\"qf-info\"><span class=\"qf-name\">{name}</span>\
         <span class=\"qf-counts\">{counts}</span>\
         <span class=\"qf-bar\" title=\"{title}\"><i class=\"m\" style=\"width:{m:.1}%\"></i><i class=\"y\" style=\"width:{y:.1}%\"></i><i class=\"l\" style=\"width:{l:.1}%\"></i></span></div>\
         <div class=\"qf-actions\"><button data-qn-msg=\"learn\" data-deck=\"{path}\" data-mode=\"learn\"{disabled}>{learn}</button>\
         <button class=\"flat\" data-qn-msg=\"learn\" data-deck=\"{path}\" data-mode=\"practice\">{practice}</button></div></div>",
        name = html::escape(&node.name),
        title = html::escape(&t!("{} learned of {} · {} mature", learned, c.total, c.mature)),
        m = pct(c.mature),
        y = pct(c.young),
        l = pct(c.learning),
        counts = html::escape(&t!("{} due · {} new · {} cards", c.due, c.new, c.total)),
        disabled = if c.due + c.new == 0 { " disabled" } else { "" },
        learn = t!("Learn"),
        practice = t!("Practice"),
    ));
    for child in &node.children {
        deck_html(child, out, depth + 1);
    }
    out.push_str("</div>");
}

/// The overview of documents' decks: progress, statistics and the cards.
/// `show_cards` lists the cards (the preview of a single document).
pub fn overview_html(docs: &[Studied<'_>], store: &Store, today: i64, show_cards: bool) -> String {
    let pairs: Vec<(&Card, Option<&CardState>)> = docs.iter().flat_map(Studied::pairs).collect();
    let mut out = String::from("<div class=\"qf-overview\">\n");
    out.push_str(OVERVIEW_CSS);
    let root = tree(&pairs, today);
    if pairs.is_empty() {
        out.push_str(&format!("<p class=\"qf-empty\">{}</p>", html::escape(t!("No flash cards yet. Write `Question::Answer`, or see the help (F1)."))));
    }
    out.push_str("<section class=\"qf-decks\">");
    for deck in &root.children {
        deck_html(deck, &mut out, 0);
    }
    out.push_str("</section>\n");

    // Statistics.
    let keys: HashSet<&str> = pairs.iter().map(|(c, _)| c.key.as_str()).collect();
    let st: LogStats = store.log_stats(&keys, today);
    if st.reviews > 0 {
        let rate = st.correct as f64 * 100.0 / st.reviews as f64;
        let items = [
            (t!("Today"), t!("{} answers · {}", st.today_reviews, minutes(st.today_ms))),
            (t!("Streak"), if st.streak == 1 { t!("1 day").to_owned() } else { t!("{} days", st.streak) }),
            (t!("Right"), format!("{rate:.0} %")),
            (t!("All time"), t!("{} answers · {}", st.reviews, minutes(st.ms))),
        ];
        out.push_str(&format!("<h2>{}</h2>\n<div class=\"qf-stats\">", t!("Statistics")));
        for (label, value) in items {
            out.push_str(&format!("<div><span>{}</span><b>{}</b></div>", html::escape(label), html::escape(&value)));
        }
        out.push_str("</div>\n");
        out.push_str(&chart(&st, today));
    }

    for studied in docs {
        if studied.doc.problems.is_empty() {
            continue;
        }
        out.push_str(&format!("<h2>{}</h2>\n<ul class=\"qf-problems\">", t!("Problems")));
        for (line, text) in &studied.doc.problems {
            out.push_str(&format!("<li>{} {}: {}</li>", t!("Line"), line + 1, html::escape(text)));
        }
        out.push_str("</ul>\n");
    }

    if show_cards && !pairs.is_empty() {
        out.push_str(&format!("<details class=\"qf-cards\"><summary>{}</summary>\n", html::escape(&t!("All cards ({})", pairs.len()))));
        for (card, state) in &pairs {
            let badge = match card.kind {
                Kind::Basic => t!("Card"),
                Kind::Reverse => t!("Reverse side"),
                Kind::Choice if card.multiple => t!("Several right"),
                Kind::Choice => t!("One right"),
            };
            let due = match state {
                None => t!("new").to_owned(),
                Some(s) if s.due <= today => t!("due").to_owned(),
                Some(s) => t!("in {} d", s.due - today),
            };
            let answer = if card.kind == Kind::Choice {
                let items: String = card
                    .choices
                    .iter()
                    .map(|c| format!("<li class=\"{}\">{}</li>", if c.correct { "ok" } else { "" }, inline(&c.text)))
                    .collect();
                format!("<ul class=\"qf-choices\">{items}</ul>{}", markdown::to_html(&card.back))
            } else {
                markdown::to_html(&card.back)
            };
            out.push_str(&format!(
                "<div class=\"qf-card\"><div class=\"qf-meta\"><span class=\"qf-badge\">{}</span> {} · {}</div><div class=\"qf-q\">{}</div><div class=\"qf-a\">{}</div></div>\n",
                html::escape(badge),
                html::escape(&card.deck.join(" › ")),
                html::escape(&due),
                markdown::to_html(&card.front),
                answer
            ));
        }
        out.push_str("</details>\n");
    }
    out.push_str("</div>\n");
    out
}

/// Answers of the last 30 days as a chart (see [`crate::charts`]).
fn chart(st: &LogStats, today: i64) -> String {
    let labels: Vec<String> = st
        .days
        .iter()
        .map(|(d, _, _)| if *d == today { t!("today").to_owned() } else { format!("−{}", today - d) })
        .collect();
    let option = serde_json::json!({
        "backgroundColor": "transparent",
        "animation": false,
        "tooltip": { "trigger": "axis" },
        "legend": { "bottom": 0 },
        "grid": { "left": 16, "right": 16, "top": 16, "bottom": 40 },
        "xAxis": { "type": "category", "data": labels },
        "yAxis": { "type": "value", "minInterval": 1 },
        "color": ["#26a269", "#e01b24"],
        "series": [
            { "name": t!("Right"), "type": "bar", "stack": "a", "data": st.days.iter().map(|d| d.1).collect::<Vec<_>>() },
            { "name": t!("Wrong"), "type": "bar", "stack": "a", "data": st.days.iter().map(|d| d.2).collect::<Vec<_>>() },
        ],
    });
    let json = serde_json::json!({ "option": option, "height": "220px", "theme": "auto" });
    format!(
        "<h3>{}</h3>\n<pre><code class=\"language-qn-chart\">{}</code></pre>\n",
        t!("Answers in the last 30 days"),
        html::escape(&serde_json::to_string(&json).unwrap_or_default())
    )
}

const OVERVIEW_CSS: &str = r#"<style>
.qf-overview button { font: inherit; font-size: .85em; padding: .25em .8em; border-radius: 6px; border: 1px solid var(--link); background: var(--link); color: var(--bg); cursor: pointer; }
.qf-overview button.flat { background: transparent; color: var(--link); }
.qf-overview button:disabled { opacity: .45; cursor: default; }
.qf-deck { margin-left: calc(var(--depth) * 0 + 0px); }
.qf-deck .qf-deck { margin-left: 1.2em; }
.qf-row { display: flex; align-items: center; gap: .8em; padding: .5em 0; border-bottom: 1px solid var(--border); }
.qf-info { flex: 1 1 auto; min-width: 0; display: grid; grid-template-columns: 1fr; gap: .15em; }
.qf-name { font-weight: 600; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.qf-actions { flex: none; display: flex; gap: .4em; }
.qf-bar { height: .45em; border-radius: 4px; background: var(--code-bg); overflow: hidden; display: flex; max-width: 16em; }
.qf-bar i { display: block; height: 100%; }
.qf-bar .m { background: #26a269; } .qf-bar .y { background: #8ff0a4; } .qf-bar .l { background: #f6d32d; }
.qf-counts { color: var(--muted); font-size: .8em; }
.qf-stats { display: grid; grid-template-columns: repeat(auto-fit, minmax(10em, 1fr)); gap: .6em; margin: .5em 0 1em; }
.qf-stats div { background: var(--code-bg); border-radius: 8px; padding: .5em .8em; display: flex; flex-direction: column; }
.qf-stats span { color: var(--muted); font-size: .8em; }
.qf-problems li { color: var(--error); }
.qf-cards summary { cursor: pointer; font-weight: 600; margin: 1em 0 .5em; }
.qf-card { border: 1px solid var(--border); border-radius: 8px; padding: .4em .8em; margin: .5em 0; }
.qf-meta { color: var(--muted); font-size: .8em; }
.qf-badge { background: var(--code-bg); border-radius: 4px; padding: 0 .4em; }
.qf-q > :last-child, .qf-a > :last-child { margin-bottom: .3em; }
.qf-a { border-top: 1px dashed var(--border); margin-top: .3em; padding-top: .3em; }
.qf-choices li.ok { font-weight: 600; color: var(--tip); }
.qf-empty { color: var(--muted); }
</style>
"#;

#[cfg(test)]
mod tests {
    use super::*;

    const DOC: &str = "---\nflashcards: true\ndeck: Biology/Cells\n---\n\n# Basics\n\nCapital of France::Paris\nMitochondria:::Powerhouse of the cell\n`a::b` is code\n- Term::Definition\n\nExplain osmosis\n?\nWater moves through a membrane\nfrom low to high concentration.\n\n## Choice\n\nWhich are organelles?\n?\n- [x] Mitochondrion\n- [x] Ribosome\n- [ ] Cell wall\nRibosomes have no membrane.\n\nLargest planet?\n?\n- ( ) Mars\n- (x) Jupiter\n\n```\nnot::a card\n?\n```\n\nTwo-way\nquestion\n??\nanswer\n";

    #[test]
    fn detects_documents() {
        assert!(is_flashcards(DOC));
        assert!(!is_flashcards("---\ntitle: x\n---\nA::B\n"));
        assert!(is_flashcards(&template()));
    }

    #[test]
    fn parses_every_card_type() {
        let doc = parse(DOC, "Fallback");
        let summary: Vec<(Kind, &str, &str, String)> =
            doc.cards.iter().map(|c| (c.kind, c.front.as_str(), c.back.as_str(), c.deck_path())).collect();
        assert_eq!(summary[0], (Kind::Basic, "Capital of France", "Paris", "Biology/Cells/Basics".into()));
        assert_eq!(summary[1].0, Kind::Basic);
        assert_eq!((summary[2].0, summary[2].1, summary[2].2), (Kind::Reverse, "Powerhouse of the cell", "Mitochondria"));
        assert_eq!((summary[3].1, summary[3].2), ("Term", "Definition"));
        assert_eq!(summary[4].1, "Explain osmosis");
        assert_eq!(summary[4].2, "Water moves through a membrane\nfrom low to high concentration.");
        let choice = &doc.cards[5];
        assert_eq!((choice.kind, choice.multiple, choice.deck_path().as_str()), (Kind::Choice, true, "Biology/Cells/Basics/Choice"));
        assert_eq!(choice.choices.iter().map(|c| c.correct).collect::<Vec<_>>(), [true, true, false]);
        assert_eq!(choice.back, "Ribosomes have no membrane.");
        let radio = &doc.cards[6];
        assert!(!radio.multiple && radio.choices[1].correct);
        assert_eq!((doc.cards[7].kind, doc.cards[8].kind), (Kind::Basic, Kind::Reverse));
        assert_eq!(doc.cards[7].front, "Two-way\nquestion");
        assert_eq!(doc.cards.len(), 9, "{summary:#?}");
        assert!(doc.problems.is_empty(), "{:?}", doc.problems);
        assert_eq!(doc.cards[0].line, 7);
    }

    #[test]
    fn keys_are_stable_and_unique() {
        let a = parse("Q::A\n", "D");
        let b = parse("# Other deck\n\nQ::Changed answer\n", "D");
        assert_eq!(a.cards[0].key, b.cards[0].key);
        let dup = parse("Q::A\nQ::B\n", "D");
        assert_ne!(dup.cards[0].key, dup.cards[1].key);
        assert_ne!(parse("Q:::A\n", "D").cards[0].key, parse("A::Q\n", "D").cards[0].key);
    }

    #[test]
    fn reports_problems() {
        let doc = parse("Q\n?\n- ( ) a\n- ( ) b\n\nR\n?\n- (x) a\n- (x) b\n\n?\nanswer only\n\nNo answer\n?\n", "D");
        assert_eq!(doc.problems.len(), 4, "{:?}", doc.problems);
    }

    #[test]
    fn decks_from_title_or_fallback() {
        assert_eq!(parse("A::B\n", "Notes").cards[0].deck, ["Notes"]);
        assert_eq!(parse("---\ntitle: Vocab\n---\nA::B\n", "Notes").cards[0].deck, ["Vocab"]);
        let doc = parse(DOC, "x");
        let pairs: Vec<(&Card, Option<&CardState>)> = doc.cards.iter().map(|c| (c, None)).collect();
        let root = tree(&pairs, 0);
        assert_eq!(root.counts.total, 9);
        let cells = &root.children[0].children[0];
        assert_eq!((cells.path.as_str(), cells.counts.new), ("Biology/Cells", 9));
        assert_eq!(cells.children[0].children[0].path, "Biology/Cells/Basics/Choice");
        assert!(in_deck(&doc.cards[5], "Biology/Cells/Basics"));
        assert!(!in_deck(&doc.cards[0], "Biology/Cells/Basics/Choice"));
        assert!(!in_deck(&doc.cards[0], "Biology/Cell"));
    }

    #[test]
    fn sessions_prefer_due_cards_and_limit_new_ones() {
        let doc = parse("a::1\nb::2\nc::3\nd::4\n", "D");
        let due = learning::schedule(None, learning::AGAIN, 10, 0);
        let later = learning::schedule(None, learning::EASY, 10, 0);
        let states = [Some(&due), Some(&later), None, None];
        let pairs: Vec<(&Card, Option<&CardState>)> = doc.cards.iter().zip(states).collect();
        let s = session_cards(&pairs, "", false, 0, 1, 10);
        assert_eq!(s.iter().map(|c| c.front.as_str()).collect::<Vec<_>>(), ["a", "c"]);
        assert_eq!(session_cards(&pairs, "", false, 1, 1, 10).len(), 1);
        assert_eq!(session_cards(&pairs, "", true, 0, 1, 10).len(), 4);
        assert!(session_cards(&pairs, "Other", true, 0, 1, 10).is_empty());
    }

    #[test]
    fn colours_stay_readable() {
        assert!((contrast("#000000", "#ffffff") - 21.0).abs() < 0.01);
        let s = Settings { accent: Some("#ffff00".into()), color: Some("#cccccc".into()), ..Default::default() };
        let c = colors(&s, false);
        assert!(contrast(&c.fg, &c.card) >= 7.0, "{c:?}");
        assert!(contrast(&c.accent, &c.card) >= 3.0, "{c:?}");
        assert!(contrast(&c.on_accent, &c.accent) >= 4.5, "{c:?}");
        for theme in ["light", "dark", "sepia", "night", "contrast", "auto"] {
            let c = colors(&Settings { theme: theme.into(), ..Default::default() }, true);
            for (fg, min) in [(&c.fg, 7.0), (&c.muted, 4.5), (&c.good, 4.5), (&c.bad, 4.5), (&c.accent, 3.0)] {
                assert!(contrast(fg, &c.card) >= min, "{theme}: {c:?}");
            }
        }
        assert_eq!(colors(&Settings::default(), true).card, "#2a2a2a");
    }

    #[test]
    fn renders_overview_and_study_page() {
        let doc = parse(DOC, "x");
        let store = Store::default();
        let studied = [Studied::new(&doc, "doc", &store)];
        let html = overview_html(&studied, &store, 0, true);
        assert!(html.contains("data-deck=\"Biology/Cells/Basics/Choice\""));
        assert!(html.contains("qf-card"));
        let cards: Vec<StudyCard> = doc.cards.iter().map(|c| study_card(c, None, false, 0)).collect();
        assert_eq!(cards[0].next.len(), 4);
        let page = study_page("T", &cards, &colors(&Settings::default(), false), false);
        assert!(page.contains("QF_DATA") && !page.contains("</script>\n<script>{"));
        let script_body = page.split("window.QF_DATA = ").nth(1).unwrap();
        assert!(!script_body.split("</script>").next().unwrap().contains("</"));
    }
}
