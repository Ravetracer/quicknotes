//! Adventures for game masters: a Markdown note with front matter
//! `adventure: 1`. `#` headings are chapters; fenced blocks hold character
//! sheets (```` ```character ````, YAML), custom dice (```` ```dice ````,
//! `Name: face, face, …`), the game state (```` ```session ````, written by
//! the GM screen) and the session log (```` ```log ````).
//!
//! The GM screen (`data/adventure/`) shows the chapter at the cursor, the
//! party, a dice tray, timers, initiative and the log; its actions come back
//! here as [`Action`]s and change the note's text.

use crate::dice::{self, CustomDie, Rng};
use serde::Serialize;
use serde_yaml::{Mapping, Value as Yaml};
use std::sync::OnceLock;

pub const LANGUAGE: &str = "adventure";

const CSS: &str = include_str!("../data/adventure/adventure.css");
const JS: &str = include_str!("../data/adventure/adventure.js");

/// Log entries shown in the GM screen.
const LOG_SHOWN: usize = 60;

// ----- structure ---------------------------------------------------------------

/// The front matter: lines 1.. up to the closing `---`, and the line after it.
fn front_matter(lines: &[&str]) -> (Option<Mapping>, usize) {
    if lines.first().map(|l| l.trim()) != Some("---") {
        return (None, 0);
    }
    let Some(end) = lines.iter().skip(1).position(|l| matches!(l.trim(), "---" | "...")) else { return (None, 0) };
    let yaml = lines[1..end + 1].join("\n");
    match serde_yaml::from_str::<Yaml>(&yaml) {
        Ok(Yaml::Mapping(m)) => (Some(m), end + 2),
        _ => (None, 0),
    }
}

/// The `title:` of the front matter.
pub fn title(text: &str) -> Option<String> {
    let lines: Vec<&str> = text.split('\n').take(60).collect();
    let fm = front_matter(&lines).0?;
    fm.get(Yaml::String("title".into())).map(text_of).filter(|t| !t.trim().is_empty())
}

pub fn is_adventure(text: &str) -> bool {
    if !text.starts_with("---") {
        return false;
    }
    let lines: Vec<&str> = text.split('\n').take(60).collect();
    matches!(front_matter(&lines).0, Some(m) if m.contains_key("adventure"))
}

fn text_of(v: &Yaml) -> String {
    match v {
        Yaml::String(s) => s.clone(),
        Yaml::Number(n) => n.to_string(),
        Yaml::Bool(b) => b.to_string(),
        Yaml::Null => String::new(),
        Yaml::Sequence(items) => items.iter().map(text_of).collect::<Vec<_>>().join(", "),
        Yaml::Mapping(m) => m.iter().map(|(k, v)| format!("{}: {}", text_of(k), text_of(v))).collect::<Vec<_>>().join(", "),
        Yaml::Tagged(t) => text_of(&t.value),
    }
}

/// A fenced block: its info string, the lines of its body and of its fences.
#[derive(Clone, Debug)]
struct Block {
    kind: String,
    open: usize,
    close: usize,
    body: String,
}

fn blocks(lines: &[&str], from: usize) -> Vec<Block> {
    let mut out = Vec::new();
    let mut i = from;
    while i < lines.len() {
        let t = lines[i].trim_start();
        let fence = if t.starts_with("```") { "```" } else if t.starts_with("~~~") { "~~~" } else { "" };
        if fence.is_empty() {
            i += 1;
            continue;
        }
        let kind = t.trim_start_matches(['`', '~']).split_whitespace().next().unwrap_or("").to_lowercase();
        let close = (i + 1..lines.len()).find(|&j| lines[j].trim_start().starts_with(fence) && lines[j].trim().trim_start_matches(['`', '~']).is_empty());
        let close = close.unwrap_or(lines.len());
        let body = lines[i + 1..close.min(lines.len())].join("\n");
        out.push(Block { kind, open: i, close, body });
        i = close + 1;
    }
    out
}

// ----- the calendar --------------------------------------------------------------

#[derive(Clone, Debug, PartialEq)]
pub struct Calendar {
    months: Vec<(String, i64)>,
}

impl Calendar {
    fn standard() -> Self {
        let names: Vec<&str> = match crate::i18n::lang() {
            crate::i18n::Lang::De => vec!["Januar", "Februar", "März", "April", "Mai", "Juni", "Juli", "August", "September", "Oktober", "November", "Dezember"],
            crate::i18n::Lang::En => vec!["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"],
        };
        let days = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
        Self { months: names.into_iter().map(str::to_owned).zip(days).collect() }
    }

    /// `Name 30, Name 31, …`
    fn parse(spec: &str) -> Option<Self> {
        let months: Vec<(String, i64)> = spec
            .split(',')
            .filter_map(|m| {
                let m = m.trim();
                let (name, days) = m.rsplit_once(char::is_whitespace)?;
                Some((name.trim().to_owned(), days.trim().parse().ok().filter(|d: &i64| *d > 0)?))
            })
            .collect();
        (!months.is_empty()).then_some(Self { months })
    }

    fn year_days(&self) -> i64 {
        self.months.iter().map(|(_, d)| d).sum()
    }

    /// Minutes since the start of year 0.
    fn to_minutes(&self, year: i64, month: i64, day: i64, hour: i64, minute: i64) -> i64 {
        let before: i64 = self.months.iter().take((month - 1).max(0) as usize).map(|(_, d)| d).sum();
        ((year * self.year_days() + before + day - 1) * 24 + hour) * 60 + minute
    }

    /// `(year, month, day, hour, minute)` of a minute count.
    fn split_minutes(&self, minutes: i64) -> (i64, i64, i64, i64, i64) {
        let minute = minutes.rem_euclid(60);
        let hours = minutes.div_euclid(60);
        let hour = hours.rem_euclid(24);
        let days = hours.div_euclid(24);
        let year = days.div_euclid(self.year_days());
        let mut day = days.rem_euclid(self.year_days());
        let mut month = 1;
        for (_, d) in &self.months {
            if day < *d {
                break;
            }
            day -= d;
            month += 1;
        }
        (year, month, day + 1, hour, minute)
    }

    /// `1492-3-15 14:30`
    fn parse_time(&self, s: &str) -> Option<i64> {
        let (date, time) = s.trim().split_once(' ').unwrap_or((s.trim(), "0:00"));
        let mut d = date.split('-').map(|p| p.trim().parse::<i64>());
        let (y, m, day) = (d.next()?.ok()?, d.next()?.ok()?, d.next()?.ok()?);
        let (h, min) = time.trim().split_once(':')?;
        Some(self.to_minutes(y, m, day, h.parse().ok()?, min.parse().ok()?))
    }

    fn format_time(&self, minutes: i64) -> String {
        let (y, m, d, h, min) = self.split_minutes(minutes);
        format!("{y:04}-{m:02}-{d:02} {h:02}:{min:02}")
    }

    /// `15 March 1492, 14:30`
    fn label(&self, minutes: i64) -> String {
        let (y, m, d, h, min) = self.split_minutes(minutes);
        let month = self.months.get((m - 1) as usize).map_or("?", |(n, _)| n.as_str());
        format!("{d}. {month} {y}, {h:02}:{min:02}")
    }
}

// ----- character sheets ------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct RollButton {
    pub label: String,
    pub notation: String,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Field {
    pub label: String,
    pub value: String,
    pub rolls: Vec<RollButton>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Sheet {
    pub name: String,
    /// player, npc or monster
    pub kind: String,
    /// The line of the opening fence.
    pub line: usize,
    pub chapter: Option<usize>,
    pub hp: Option<(i64, i64)>,
    pub count: i64,
    pub conditions: Vec<String>,
    pub fields: Vec<Field>,
    /// Initiative roll.
    pub init: String,
}

/// D&D-style ability modifier.
fn modifier(score: i64) -> i64 {
    (score - 10).div_euclid(2)
}

fn signed(n: i64) -> String {
    if n >= 0 {
        format!("+{n}")
    } else {
        format!("−{}", -n)
    }
}

fn notation_signed(n: i64) -> String {
    if n >= 0 {
        format!("+{n}")
    } else {
        n.to_string()
    }
}

/// Roll buttons found in a text: dice notation (`1d6+3`), attack bonuses
/// (`+5` → d20) and DSA values (`AT 14` → d20 ≤ 14).
fn rolls_in(text: &str, system: &str) -> Vec<RollButton> {
    static DICE: OnceLock<regex::Regex> = OnceLock::new();
    static AT: OnceLock<regex::Regex> = OnceLock::new();
    static BONUS: OnceLock<regex::Regex> = OnceLock::new();
    let mut out = Vec::new();
    let dice_re = DICE.get_or_init(|| regex::Regex::new(r"\b\d*[dDwW](?:\d+|%|F)(?:\s*[+-]\s*\d+)*(?:kh\d+|kl\d+|dl\d+|dh\d+|!)?").expect("regex"));
    for m in dice_re.find_iter(text) {
        let n = m.as_str().replace(' ', "");
        if dice::is_notation(&n) {
            out.push(RollButton { label: n.clone(), notation: n });
        }
    }
    let at_re = AT.get_or_init(|| regex::Regex::new(r"\b(AT|PA|AW|FK)\s*(\d+)").expect("regex"));
    for c in at_re.captures_iter(text) {
        out.push(RollButton { label: format!("{} {}", &c[1], &c[2]), notation: format!("1d20<={}", &c[2]) });
    }
    if system != "dsa5" {
        let bonus_re = BONUS.get_or_init(|| regex::Regex::new(r"(?:^|[\s,(])([+-]\d+)\b").expect("regex"));
        let dice_spans: Vec<std::ops::Range<usize>> = dice_re.find_iter(text).map(|m| m.range()).collect();
        for c in bonus_re.captures_iter(text) {
            let m = c.get(1).expect("group");
            if dice_spans.iter().any(|r| r.contains(&m.start())) {
                continue;
            }
            out.insert(0, RollButton { label: format!("d20{}", m.as_str()), notation: format!("1d20{}", m.as_str()) });
        }
    }
    out
}

fn parse_hp(v: &Yaml) -> Option<(i64, i64)> {
    let s = text_of(v);
    let s = s.split_whitespace().next().unwrap_or("");
    match s.split_once('/') {
        Some((c, m)) => Some((c.trim().parse().ok()?, m.trim().parse().ok()?)),
        None => {
            let n = s.parse().ok()?;
            Some((n, n))
        }
    }
}

const STAT_KEYS: &[&str] = &["stats", "attributes", "abilities", "eigenschaften", "werte"];

fn sheet(block: &Block, system: &str, chapter: Option<usize>) -> Sheet {
    let map = match serde_yaml::from_str::<Yaml>(&block.body) {
        Ok(Yaml::Mapping(m)) => m,
        _ => Mapping::new(),
    };
    let get = |k: &str| map.iter().find(|(key, _)| text_of(key).eq_ignore_ascii_case(k)).map(|(_, v)| v);
    let system = get("system").or_else(|| get("preset")).map(text_of).unwrap_or_else(|| system.to_owned());
    let name = get("name").map(text_of).unwrap_or_else(|| "?".into());
    let kind = get("type").or_else(|| get("kind")).map(text_of).unwrap_or_else(|| "npc".into()).to_lowercase();
    let hp = get("hp").or_else(|| get("lep")).and_then(parse_hp);
    let count = get("count").and_then(|v| text_of(v).parse().ok()).unwrap_or(1).max(1);
    let conditions = match get("conditions").or_else(|| get("zustände")) {
        Some(Yaml::Sequence(items)) => items.iter().map(text_of).collect(),
        Some(v) if !text_of(v).is_empty() => text_of(v).split(',').map(|c| c.trim().to_owned()).collect(),
        _ => Vec::new(),
    };
    // Stats first: their values feed DSA skill checks.
    let mut stats: Vec<(String, i64)> = Vec::new();
    for key in STAT_KEYS {
        if let Some(Yaml::Mapping(m)) = get(key) {
            for (k, v) in m {
                if let Ok(n) = text_of(v).trim().parse::<i64>() {
                    stats.push((text_of(k), n));
                }
            }
        }
    }
    let stat = |name: &str| stats.iter().find(|(k, _)| k.eq_ignore_ascii_case(name)).map(|(_, v)| *v);
    let mut fields = Vec::new();
    let mut init_bonus: Option<String> = None;
    for (k, v) in &map {
        let label = text_of(k);
        let key = label.to_lowercase();
        if matches!(key.as_str(), "name" | "type" | "kind" | "system" | "preset" | "count" | "conditions" | "zustände") {
            continue;
        }
        if key == "init" || key == "initiative" || key == "ini" {
            init_bonus = Some(text_of(v));
        }
        let mut field = Field { label: label.clone(), value: text_of(v), rolls: Vec::new() };
        match v {
            Yaml::Mapping(m) if STAT_KEYS.contains(&key.as_str()) => {
                let mut parts = Vec::new();
                for (sk, sv) in m {
                    let (sk, sv) = (text_of(sk), text_of(sv));
                    let Ok(n) = sv.trim().parse::<i64>() else {
                        parts.push(format!("{sk} {sv}"));
                        continue;
                    };
                    match system.as_str() {
                        "dsa5" | "dsa" => {
                            parts.push(format!("{sk} {n}"));
                            field.rolls.push(RollButton { label: format!("{sk} {n}"), notation: format!("1d20<={n}") });
                        }
                        "generic" => parts.push(format!("{sk} {n}")),
                        _ => {
                            let m = modifier(n);
                            parts.push(format!("{sk} {n} ({})", signed(m)));
                            field.rolls.push(RollButton { label: format!("{sk} {}", signed(m)), notation: format!("1d20{}", notation_signed(m)) });
                        }
                    }
                }
                field.value = parts.join(" · ");
            }
            Yaml::Mapping(m) => {
                // Skills, saves: `Stealth: +5` or DSA `Klettern: MU/GE/KK 6`.
                let mut parts = Vec::new();
                for (sk, sv) in m {
                    let (sk, sv) = (text_of(sk), text_of(sv));
                    match sv.trim().parse::<i64>() {
                        Ok(n) if !system.starts_with("dsa") => parts.push(format!("{sk} {}", signed(n))),
                        _ => parts.push(format!("{sk} {sv}")),
                    }
                    let words: Vec<&str> = sv.split_whitespace().collect();
                    let three: Option<Vec<i64>> = words.first().and_then(|w| w.split('/').map(stat).collect::<Option<Vec<i64>>>()).filter(|v| v.len() == 3);
                    if let Some(values) = three {
                        let fw = words.get(1).and_then(|w| w.parse::<i64>().ok()).unwrap_or(0);
                        field.rolls.push(RollButton {
                            label: sk.clone(),
                            notation: format!("check {}/{}/{} {fw}", values[0], values[1], values[2]),
                        });
                    } else if let Ok(n) = sv.trim().replace('−', "-").parse::<i64>() {
                        let notation = if matches!(system.as_str(), "dsa5" | "dsa") { format!("1d20<={n}") } else { format!("1d20{}", notation_signed(n)) };
                        field.rolls.push(RollButton { label: format!("{sk} {}", if system.starts_with("dsa") { n.to_string() } else { signed(n) }), notation });
                    } else {
                        for mut r in rolls_in(&sv, &system) {
                            r.label = format!("{sk}: {}", r.label);
                            field.rolls.push(r);
                        }
                    }
                }
                field.value = parts.join(" · ");
            }
            Yaml::Sequence(items) => {
                let mut parts = Vec::new();
                for item in items {
                    let t = text_of(item);
                    let head = t.split([',', '+']).next().unwrap_or("").trim().to_owned();
                    for mut r in rolls_in(&t, &system) {
                        if !head.is_empty() && items.len() > 1 {
                            r.label = format!("{head}: {}", r.label);
                        }
                        field.rolls.push(r);
                    }
                    parts.push(t);
                }
                field.value = parts.join(" · ");
            }
            _ if matches!(key.as_str(), "hp" | "lep" | "init" | "initiative" | "ini") => continue,
            _ => field.rolls = rolls_in(&field.value, &system),
        }
        fields.push(field);
    }
    let init = match init_bonus {
        Some(b) if dice::is_notation(&b) => b,
        Some(b) => match b.trim().replace('−', "-").trim_start_matches('+').parse::<i64>() {
            Ok(n) if system.starts_with("dsa") => format!("1d6+{n}"),
            Ok(n) => format!("1d20{}", notation_signed(n)),
            Err(_) => "1d20".into(),
        },
        None => match stat("DEX").or_else(|| stat("GES")) {
            Some(dex) if !system.starts_with("dsa") && system != "generic" => format!("1d20{}", notation_signed(modifier(dex))),
            _ => "1d20".into(),
        },
    };
    Sheet { name, kind, line: block.open, chapter, hp, count, conditions, fields, init }
}

// ----- the game state ----------------------------------------------------------------

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct Combatant {
    pub name: String,
    pub init: i64,
    /// Hit points of combatants without their own sheet (monster copies).
    pub hp: Option<(i64, i64)>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Session {
    /// Play time in seconds.
    pub played: i64,
    /// World time in minutes (see [`Calendar`]); `None` until set.
    pub world: Option<i64>,
    pub round: i64,
    pub turn: usize,
    pub order: Vec<Combatant>,
}

fn parse_session(body: &str, cal: &Calendar) -> Session {
    let mut s = Session::default();
    let mut in_init = false;
    for line in body.lines() {
        let t = line.trim();
        if let Some(entry) = t.strip_prefix("- ").filter(|_| in_init) {
            // `Name: 18` or `Goblin 1: 12 (5/7)`
            if let Some((name, rest)) = entry.rsplit_once(':') {
                let rest = rest.trim();
                let (init, hp) = match rest.split_once('(') {
                    Some((i, h)) => (i.trim(), parse_hp(&Yaml::String(h.trim_end_matches(')').to_owned()))),
                    None => (rest, None),
                };
                s.order.push(Combatant { name: name.trim().to_owned(), init: init.parse().unwrap_or(0), hp });
            }
            continue;
        }
        in_init = false;
        let Some((k, v)) = t.split_once(':') else { continue };
        let v = v.trim();
        match k.trim() {
            "played" => s.played = parse_duration(v),
            "world" => s.world = cal.parse_time(v),
            "round" => s.round = v.parse().unwrap_or(0),
            "turn" => s.turn = v.parse().unwrap_or(0),
            "initiative" => in_init = true,
            _ => {}
        }
    }
    s
}

/// `1h 23m 10s`
fn parse_duration(s: &str) -> i64 {
    let mut total = 0;
    for part in s.split_whitespace() {
        let (n, unit) = part.split_at(part.find(|c: char| !c.is_ascii_digit()).unwrap_or(part.len()));
        let n: i64 = n.parse().unwrap_or(0);
        total += match unit {
            "h" => n * 3600,
            "m" | "min" => n * 60,
            _ => n,
        };
    }
    total
}

fn format_duration(secs: i64) -> String {
    format!("{}h {:02}m {:02}s", secs / 3600, secs % 3600 / 60, secs % 60)
}

fn session_text(s: &Session, cal: &Calendar) -> String {
    let mut out = format!("played: {}\n", format_duration(s.played));
    if let Some(w) = s.world {
        out.push_str(&format!("world: {}\n", cal.format_time(w)));
    }
    if !s.order.is_empty() {
        out.push_str(&format!("round: {}\nturn: {}\ninitiative:\n", s.round, s.turn));
        for c in &s.order {
            match c.hp {
                Some((cur, max)) => out.push_str(&format!("  - {}: {} ({cur}/{max})\n", c.name, c.init)),
                None => out.push_str(&format!("  - {}: {}\n", c.name, c.init)),
            }
        }
    }
    out
}

// ----- the document -----------------------------------------------------------------

#[derive(Clone, Debug, Serialize)]
pub struct Chapter {
    pub title: String,
    pub line: usize,
    pub html: String,
}

#[derive(Clone, Debug)]
pub struct Doc {
    pub title: String,
    pub calendar: Calendar,
    pub chapters: Vec<Chapter>,
    pub sheets: Vec<Sheet>,
    pub dice: Vec<CustomDie>,
    pub session: Session,
    session_block: Option<(usize, usize)>,
    pub log: Vec<String>,
    log_block: Option<(usize, usize)>,
}

pub fn parse(text: &str, fallback_title: &str) -> Doc {
    let lines: Vec<&str> = text.split('\n').collect();
    let (fm, start) = front_matter(&lines);
    let fm = fm.unwrap_or_default();
    let get = |k: &str| fm.get(Yaml::String(k.into())).map(text_of);
    let system = get("system").unwrap_or_else(|| "dnd5e".into()).to_lowercase();
    let calendar = get("calendar").and_then(|c| Calendar::parse(&c)).unwrap_or_else(Calendar::standard);
    let title = get("title").unwrap_or_else(|| fallback_title.to_owned());
    let all = blocks(&lines, start);
    let in_block = |line: usize| all.iter().any(|b| b.open <= line && line <= b.close);

    // Chapters: `#` headings; text before the first one is an introduction.
    let mut heads: Vec<(usize, String)> = (start..lines.len())
        .filter(|&i| !in_block(i) && lines[i].starts_with("# "))
        .map(|i| (i, lines[i][2..].trim().to_owned()))
        .collect();
    let intro = heads.first().map_or(lines.len(), |(l, _)| *l);
    if (start..intro).any(|i| !lines[i].trim().is_empty() && !in_block(i) || all.iter().any(|b| b.open == i && b.kind == "character")) {
        heads.insert(0, (start, title.clone()));
    }
    let chapter_of = |line: usize| heads.iter().rposition(|(l, _)| *l <= line);

    let mut sheets = Vec::new();
    let mut dice_list = Vec::new();
    let mut session = Session::default();
    let mut session_block = None;
    let mut log = Vec::new();
    let mut log_block = None;
    for b in &all {
        match b.kind.as_str() {
            "character" | "charakter" | "npc" | "monster" => sheets.push(sheet(b, &system, chapter_of(b.open))),
            "dice" | "würfel" => {
                for l in b.body.lines() {
                    if let Some((name, faces)) = l.split_once(':') {
                        dice_list.push(CustomDie { name: name.trim().to_owned(), faces: faces.split(',').map(|f| f.trim().to_owned()).collect() });
                    }
                }
            }
            "session" => {
                session = parse_session(&b.body, &calendar);
                session_block = Some((b.open, b.close));
            }
            "log" => {
                log = b.body.lines().filter(|l| !l.trim().is_empty()).map(str::to_owned).collect();
                log_block = Some((b.open, b.close));
            }
            _ => {}
        }
    }

    let mut chapters = Vec::new();
    for (i, (line, title)) in heads.iter().enumerate() {
        let end = heads.get(i + 1).map_or(lines.len(), |(l, _)| *l);
        let body_start = if *line == start && heads.len() > i && !lines.get(*line).is_some_and(|l| l.starts_with("# ")) { *line } else { line + 1 };
        // The body with sheets as placeholders and the state blocks left out.
        let mut md = String::new();
        let mut j = body_start;
        while j < end {
            if let Some(b) = all.iter().find(|b| b.open == j) {
                if let Some(idx) = sheets.iter().position(|s| s.line == j) {
                    md.push_str(&format!("\n<div class=\"sheet-slot\" data-sheet=\"{idx}\"></div>\n\n"));
                    j = b.close + 1;
                    continue;
                }
                if matches!(b.kind.as_str(), "dice" | "würfel" | "session" | "log") {
                    j = b.close + 1;
                    continue;
                }
            }
            md.push_str(lines[j]);
            md.push('\n');
            j += 1;
        }
        chapters.push(Chapter { title: title.clone(), line: *line, html: crate::markdown::to_html(&md) });
    }
    Doc { title, calendar, chapters, sheets, dice: dice_list, session, session_block, log, log_block }
}

/// The chapter holding `line`.
pub fn chapter_at(doc: &Doc, line: usize) -> usize {
    doc.chapters.iter().rposition(|c| c.line <= line).unwrap_or(0)
}

// ----- actions -------------------------------------------------------------------------

/// Something done in the GM screen.
#[derive(Clone, Debug, PartialEq)]
pub enum Action {
    Roll { notation: String, who: String, label: String },
    /// Hit points of a combatant or sheet, changed by `delta`.
    Hp { name: String, delta: i64 },
    /// World time forward (or back).
    Time { minutes: i64 },
    /// Sets the world time (`1492-03-15 14:30`).
    SetTime(String),
    /// Play time to add (seconds).
    Played(i64),
    /// Rolls initiative for the party and the chapter's NPCs and monsters.
    Initiative { chapter: usize },
    Next,
    Previous,
    EndCombat,
    Remove(String),
    Note(String),
}

/// The new text after an action and a message for the user (e.g. a roll result).
pub fn apply(text: &str, doc: &Doc, action: &Action, rng: &mut dyn Rng, now: &jiff::Zoned) -> Result<String, String> {
    let mut session = doc.session.clone();
    let mut lines_out: Vec<String> = Vec::new();
    let mut text = text.to_owned();
    match action {
        Action::Roll { notation, who, label } => {
            let r = dice::roll(notation, &doc.dice, rng)?;
            let crit = match r.critical {
                Some(true) => " ★",
                Some(false) => " ✗",
                None => "",
            };
            let who = if who.is_empty() { String::new() } else { format!("{who}: ") };
            let label = if label.is_empty() || label == notation { String::new() } else { format!("{label} ") };
            lines_out.push(format!("{who}{label}{} → **{}**{crit} ({})", r.notation, r.summary, r.detail));
        }
        Action::Hp { name, delta } => {
            if let Some(c) = session.order.iter_mut().find(|c| &c.name == name && c.hp.is_some()) {
                let (cur, max) = c.hp.expect("hp");
                let new = cur + delta;
                c.hp = Some((new, max));
                lines_out.push(format!("{name}: {} {cur} → {new}", t!("HP")));
            } else if let Some(s) = doc.sheets.iter().find(|s| &s.name == name) {
                let Some((cur, max)) = s.hp else { return Ok(text) };
                let new = cur + delta;
                text = set_sheet_hp(&text, s.line, new, max);
                lines_out.push(format!("{name}: {} {cur} → {new}", t!("HP")));
            } else {
                return Ok(text);
            }
        }
        Action::Time { minutes } => {
            let from = session.world.unwrap_or_else(|| doc.calendar.to_minutes(1, 1, 1, 8, 0));
            session.world = Some(from + minutes);
            lines_out.push(t!("Time: {}", doc.calendar.label(from + minutes)));
        }
        Action::SetTime(spec) => {
            let w = doc.calendar.parse_time(spec).ok_or_else(|| t!("Unknown time “{}”", spec))?;
            session.world = Some(w);
            lines_out.push(t!("Time: {}", doc.calendar.label(w)));
        }
        Action::Played(secs) => session.played = (session.played + secs).max(0),
        Action::Initiative { chapter } => {
            let mut order = Vec::new();
            let mut rolls = Vec::new();
            for s in doc.sheets.iter().filter(|s| s.kind == "player" || (s.chapter == Some(*chapter) && s.kind != "player")) {
                for n in 1..=s.count {
                    let name = if s.count > 1 { format!("{} {n}", s.name) } else { s.name.clone() };
                    let r = dice::roll(&s.init, &doc.dice, rng)?;
                    let init = r.total.unwrap_or(0);
                    rolls.push(format!("{name} {init}"));
                    order.push(Combatant { name, init, hp: if s.count > 1 { s.hp.map(|(_, m)| (m, m)) } else { None } });
                }
            }
            order.sort_by_key(|c| std::cmp::Reverse(c.init));
            session.order = order;
            session.round = 1;
            session.turn = 0;
            lines_out.push(t!("Initiative: {}", rolls.join(", ")));
        }
        Action::Next | Action::Previous => {
            if session.order.is_empty() {
                return Ok(text);
            }
            let n = session.order.len();
            if *action == Action::Next {
                session.turn += 1;
                if session.turn >= n {
                    session.turn = 0;
                    session.round += 1;
                }
            } else if session.turn == 0 {
                session.turn = n - 1;
                session.round = (session.round - 1).max(1);
            } else {
                session.turn -= 1;
            }
            lines_out.push(t!("Round {}: {}", session.round, session.order[session.turn.min(n - 1)].name));
        }
        Action::EndCombat => {
            session.order.clear();
            session.round = 0;
            session.turn = 0;
            lines_out.push(t!("Combat over").into());
        }
        Action::Remove(name) => {
            if let Some(i) = session.order.iter().position(|c| &c.name == name) {
                session.order.remove(i);
                if session.turn > i || session.turn >= session.order.len() {
                    session.turn = session.turn.saturating_sub(1);
                }
            }
        }
        Action::Note(note) => {
            if note.trim().is_empty() {
                return Ok(text);
            }
            lines_out.push(note.trim().to_owned());
        }
    }
    // The text may have changed above (hit points); re-read the block positions.
    let fresh = parse(&text, &doc.title);
    if session != doc.session {
        text = set_block(&text, fresh.session_block, "session", &session_text(&session, &doc.calendar), None);
    }
    if !lines_out.is_empty() {
        let fresh = parse(&text, &doc.title);
        let stamp = now.strftime("%H:%M").to_string();
        let day = now.strftime("%Y-%m-%d").to_string();
        let mut log = fresh.log.clone();
        if !log.iter().any(|l| l.contains(&day)) {
            log.push(format!("— {day} —"));
        }
        for l in lines_out {
            log.push(format!("[{stamp}] {l}"));
        }
        text = set_block(&text, fresh.log_block, "log", &(log.join("\n") + "\n"), fresh.session_block);
    }
    Ok(text)
}

/// Replaces a fenced block (or appends it at the end, after `after`).
fn set_block(text: &str, block: Option<(usize, usize)>, kind: &str, body: &str, _after: Option<(usize, usize)>) -> String {
    let lines: Vec<&str> = text.split('\n').collect();
    let new_block = format!("```{kind}\n{body}```");
    match block {
        Some((open, close)) => {
            let mut out: Vec<String> = lines[..open].iter().map(|s| s.to_string()).collect();
            out.push(new_block);
            out.extend(lines.get(close + 1..).unwrap_or(&[]).iter().map(|s| s.to_string()));
            out.join("\n")
        }
        None => {
            let mut t = text.trim_end_matches('\n').to_owned();
            t.push_str("\n\n");
            t.push_str(&new_block);
            t.push('\n');
            t
        }
    }
}

/// Sets `hp: cur/max` in the sheet whose fence opens on `line`.
fn set_sheet_hp(text: &str, line: usize, cur: i64, max: i64) -> String {
    let mut lines: Vec<String> = text.split('\n').map(str::to_owned).collect();
    for l in lines.iter_mut().skip(line + 1) {
        if l.trim_start().starts_with("```") {
            break;
        }
        let trimmed = l.trim_start();
        let lower = trimmed.to_lowercase();
        if lower.starts_with("hp:") || lower.starts_with("lep:") {
            let indent = &l[..l.len() - trimmed.len()];
            let key = &trimmed[..trimmed.find(':').unwrap_or(0)];
            let rest = trimmed[key.len() + 1..].trim();
            let suffix = rest.split_once(char::is_whitespace).map(|(_, s)| format!(" {s}")).unwrap_or_default();
            *l = format!("{indent}{key}: {cur}/{max}{suffix}");
            break;
        }
    }
    lines.join("\n")
}

// ----- the GM screen ------------------------------------------------------------------------

#[derive(Serialize)]
struct Model<'a> {
    title: &'a str,
    current: usize,
    chapters: &'a [Chapter],
    sheets: &'a [Sheet],
    dice: &'a [CustomDie],
    played: i64,
    world: Option<String>,
    round: i64,
    turn: usize,
    order: Vec<serde_json::Value>,
    log: Vec<String>,
}

/// The data the GM screen shows (see `adventure.js`).
pub fn model(doc: &Doc, cursor_line: usize) -> String {
    let order = doc
        .session
        .order
        .iter()
        .map(|c| {
            let sheet = doc.sheets.iter().position(|s| s.name == c.name);
            let hp = c.hp.or_else(|| sheet.and_then(|i| doc.sheets[i].hp));
            serde_json::json!({"name": c.name, "init": c.init, "hp": hp, "sheet": sheet})
        })
        .collect();
    let log = doc.log.iter().rev().take(LOG_SHOWN).cloned().collect();
    let m = Model {
        title: &doc.title,
        current: chapter_at(doc, cursor_line),
        chapters: &doc.chapters,
        sheets: &doc.sheets,
        dice: &doc.dice,
        played: doc.session.played,
        world: doc.session.world.map(|w| doc.calendar.label(w)),
        round: doc.session.round,
        turn: doc.session.turn,
        order,
        log,
    };
    serde_json::to_string(&m).unwrap_or_else(|_| "null".into()).replace("</", "<\\/")
}

/// The GM screen page; data comes with `GM.update(model)`.
pub fn page(dark: bool, vars: &str) -> String {
    let strings = serde_json::json!({
        "chapters": t!("Chapters"), "party": t!("Party"), "dice": t!("Dice"), "initiative": t!("Initiative"),
        "log": t!("Log"), "roll": t!("Roll"), "advantage": t!("Advantage"), "disadvantage": t!("Disadvantage"),
        "notation": t!("e.g. 2d6+3, 4d6dl1, 6d10>=8, 4dF"), "playTime": t!("Play time"), "start": t!("Start"),
        "pause": t!("Pause"), "worldTime": t!("World time"), "setTime": t!("Set the world time"),
        "rollInit": t!("Roll initiative"), "next": t!("Next turn"), "prev": t!("Previous turn"), "end": t!("End combat"),
        "round": t!("Round"), "remove": t!("Remove"), "note": t!("Add a note to the log"), "hpMinus": t!("Damage"),
        "hpPlus": t!("Healing"), "inChapter": t!("In this chapter"), "noInit": t!("No combat running. Rolls initiative for the party and this chapter's NPCs and monsters."),
        "gotoLine": t!("Show in the editor"), "count": t!("Number"), "rest": t!("Rest"), "day": t!("Day"),
        "prevChapter": t!("Previous chapter"), "nextChapter": t!("Next chapter"), "custom": t!("Custom dice"),
        "amount": t!("Amount"), "who": t!("GM"),
    })
    .to_string()
    .replace("</", "<\\/");
    let theme = if dark { "dark" } else { "light" };
    format!(
        r#"<!DOCTYPE html>
<html data-theme="{theme}" style="{vars}">
<head><meta charset="utf-8"><style>{CSS}</style></head>
<body>
<div id="gm"></div>
<script>window.GM_STRINGS = {strings};</script>
<script>{JS}</script>
</body>
</html>"#
    )
}

// ----- new documents ----------------------------------------------------------------------------

/// Character sheet presets: (id, label, sheet).
pub fn presets() -> Vec<(&'static str, &'static str, &'static str)> {
    vec![
        ("generic", "Generic Character", "```character\nname: New character\ntype: player\nhp: 10/10\nstats: {Strength: 3, Agility: 3, Mind: 3}\nskills: {Fighting: +2, Sneaking: +1}\nweapons:\n  - Sword +2, 1d6+1\nitems: []\nconditions: []\nnotes: \n```\n"),
        ("dnd5e", "D&D 5e Character", "```character\nname: New hero\ntype: player\nsystem: dnd5e\nplayer: \nclass: Fighter 1\nhp: 12/12\nac: 16\ninit: +1\nstats: {STR: 16, DEX: 12, CON: 14, INT: 10, WIS: 12, CHA: 8}\nsaves: {STR: +5, CON: +4}\nskills: {Athletics: +5, Perception: +3}\nweapons:\n  - Longsword +5, 1d8+3\n  - Handaxe +5, 1d6+3\nitems: [Chain mail, Shield, Explorer's pack]\nconditions: []\n```\n"),
        ("dnd5e-monster", "D&D 5e Monster", "```character\nname: Goblin\ntype: monster\nsystem: dnd5e\ncount: 3\nhp: 7/7\nac: 15\ninit: +2\nstats: {STR: 8, DEX: 14, CON: 10, INT: 10, WIS: 8, CHA: 8}\nweapons:\n  - Scimitar +4, 1d6+2\n  - Shortbow +4, 1d6+2\nnotes: Nimble Escape\n```\n"),
        ("pf2e", "Pathfinder 2e Character", "```character\nname: New hero\ntype: player\nsystem: pf2e\nancestry: Human\nclass: Rogue 1\nhp: 15/15\nac: 18\ninit: +7\nstats: {STR: 10, DEX: 18, CON: 12, INT: 14, WIS: 12, CHA: 10}\nsaves: {Fortitude: +4, Reflex: +9, Will: +4}\nskills: {Stealth: +7, Thievery: +7, Perception: +6}\nweapons:\n  - Rapier +7, 1d6+4\n  - Shortbow +7, 1d6\nitems: [Leather armor, Thieves' toolkit]\nconditions: []\n```\n"),
        ("dsa5", "DSA 5 Character", "```character\nname: Neue Heldin\ntype: player\nsystem: dsa5\nspezies: Mensch\nprofession: Kriegerin\nlep: 32/32\nini: 12\nstats: {MU: 14, KL: 11, IN: 12, CH: 11, FF: 12, GE: 13, KO: 14, KK: 14}\nskills: {Klettern: MU/GE/KK 6, Sinnesschärfe: KL/IN/IN 5, Kraftakt: KO/KK/KK 7}\nweapons:\n  - Schwert AT 14 PA 8, 1d6+4\nitems: [Kettenhemd, Rucksack]\nconditions: []\n```\n"),
    ]
}

/// A new adventure with examples.
pub fn template() -> String {
    match crate::i18n::lang() {
        crate::i18n::Lang::De => TEMPLATE_DE.to_owned(),
        crate::i18n::Lang::En => TEMPLATE_EN.to_owned(),
    }
}

const TEMPLATE_EN: &str = "---\nadventure: 1\ntitle: The Sunken Tower\nsystem: dnd5e\ncalendar: Hammer 30, Alturiak 30, Ches 30, Tarsakh 30, Mirtul 30, Kythorn 30, Flamerule 30, Eleasis 30, Eleint 30, Marpenoth 30, Uktar 30, Nightal 30\n---\n# Party\n\n```character\nname: Aria\ntype: player\nplayer: Kim\nhp: 24/24\nac: 15\ninit: +3\nstats: {STR: 10, DEX: 16, CON: 12, INT: 14, WIS: 10, CHA: 8}\nskills: {Stealth: +5, Perception: +2}\nweapons:\n  - Shortsword +5, 1d6+3\n  - Shortbow +5, 1d6+3\nitems: [Rope, Torch, Healing potion]\n```\n\n```character\nname: Borin\ntype: player\nplayer: Sam\nhp: 31/31\nac: 18\nstats: {STR: 16, DEX: 10, CON: 16, INT: 8, WIS: 12, CHA: 10}\nweapons:\n  - Battleaxe +5, 1d8+3\nconditions: []\n```\n\n# Chapter 1: The Village\n\n> Fog lies over the marsh. In the distance, the tower leans out of the water.\n\n- The innkeeper knows a way through the marsh (Persuasion `1d20+2`, DC 12).\n- At night, wolves howl: roll `1d6`, on a 1 they attack.\n\n```character\nname: Old Mara\ntype: npc\nhp: 9/9\nnotes: Innkeeper, knows the old paths\n```\n\n# Chapter 2: The Tower\n\nThe door is jammed (Athletics `1d20+5`, DC 15).\n\n```character\nname: Skeleton\ntype: monster\ncount: 3\nhp: 13/13\nac: 13\ninit: +2\nweapons:\n  - Shortsword +4, 1d6+2\n```\n\n```dice\nOmen: Doom, Hope, Hope, -, -, -\n```\n";

const TEMPLATE_DE: &str = "---\nadventure: 1\ntitle: Der versunkene Turm\nsystem: dsa5\ncalendar: Praios 30, Rondra 30, Efferd 30, Travia 30, Boron 30, Hesinde 30, Firun 30, Tsa 30, Phex 30, Peraine 30, Ingerimm 30, Rahja 30, Namenlose Tage 5\n---\n# Gruppe\n\n```character\nname: Alrike\ntype: player\nplayer: Kim\nsystem: dsa5\nlep: 30/30\nini: 13\nstats: {MU: 13, KL: 12, IN: 14, CH: 11, FF: 13, GE: 14, KO: 12, KK: 11}\nskills: {Sinnesschärfe: KL/IN/IN 7, Verbergen: MU/IN/GE 6}\nweapons:\n  - Säbel AT 13 PA 7, 1d6+3\nitems: [Seil, Fackel, Heiltrank]\n```\n\n```character\nname: Boromil\ntype: player\nplayer: Sam\nsystem: dsa5\nlep: 36/36\nini: 11\nstats: {MU: 14, KL: 10, IN: 12, CH: 11, FF: 11, GE: 12, KO: 15, KK: 15}\nskills: {Kraftakt: KO/KK/KK 8}\nweapons:\n  - Orknase AT 14 PA 6, 1d6+5\nconditions: []\n```\n\n# Kapitel 1: Das Dorf\n\n> Nebel liegt über dem Sumpf. In der Ferne ragt der Turm schief aus dem Wasser.\n\n- Die Wirtin kennt einen Weg durch den Sumpf (Überreden `check 12/13/11 4`).\n- Nachts heulen Wölfe: `1d6` würfeln, bei 1 greifen sie an.\n\n```character\nname: Mutter Mara\ntype: npc\nlep: 20/20\nnotes: Wirtin, kennt die alten Pfade\n```\n\n# Kapitel 2: Der Turm\n\nDie Tür klemmt (Kraftakt, erschwert um 2).\n\n```character\nname: Skelett\ntype: monster\nsystem: dsa5\ncount: 3\nlep: 15/15\nini: 10\nweapons:\n  - Kurzschwert AT 11 PA 4, 1d6+2\n```\n\n```dice\nOmen: Unheil, Hoffnung, Hoffnung, -, -, -\n```\n";

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dice::tests::Fixed;

    fn now() -> jiff::Zoned {
        "2026-11-14T20:15:00+01:00[Europe/Berlin]".parse().unwrap()
    }

    #[test]
    fn parses_the_template() {
        let doc = parse(TEMPLATE_EN, "x");
        assert_eq!(doc.title, "The Sunken Tower");
        let titles: Vec<&str> = doc.chapters.iter().map(|c| c.title.as_str()).collect();
        assert_eq!(titles, ["Party", "Chapter 1: The Village", "Chapter 2: The Tower"]);
        assert_eq!(doc.sheets.len(), 4);
        let aria = &doc.sheets[0];
        assert_eq!((aria.kind.as_str(), aria.hp, aria.chapter, aria.init.as_str()), ("player", Some((24, 24)), Some(0), "1d20+3"));
        let stats = aria.fields.iter().find(|f| f.label == "stats").unwrap();
        assert!(stats.value.starts_with("STR 10 (+0) · DEX 16 (+3)"));
        assert_eq!(stats.rolls[1].notation, "1d20+3");
        let weapons = aria.fields.iter().find(|f| f.label == "weapons").unwrap();
        let notations: Vec<&str> = weapons.rolls.iter().map(|r| r.notation.as_str()).collect();
        assert_eq!(notations, ["1d20+5", "1d6+3", "1d20+5", "1d6+3"]);
        assert_eq!(doc.sheets[1].init, "1d20+0");
        assert_eq!(doc.sheets[3].count, 3);
        assert_eq!(doc.sheets[3].chapter, Some(2));
        assert_eq!(doc.dice[0].faces.len(), 6);
        assert!(doc.chapters[1].html.contains("<code>1d20+2</code>"));
        assert!(doc.chapters[2].html.contains("data-sheet=\"3\""));
        assert!(!doc.chapters[2].html.contains("Omen"));
        assert!(is_adventure(TEMPLATE_EN) && is_adventure(TEMPLATE_DE));
    }

    #[test]
    fn dsa_sheets() {
        let doc = parse(TEMPLATE_DE, "x");
        let a = &doc.sheets[0];
        assert_eq!((a.hp, a.init.as_str()), (Some((30, 30)), "1d6+13"));
        let skills = a.fields.iter().find(|f| f.label == "skills").unwrap();
        assert_eq!(skills.rolls[0].notation, "check 12/14/14 7");
        let weapons = a.fields.iter().find(|f| f.label == "weapons").unwrap();
        let notations: Vec<&str> = weapons.rolls.iter().map(|r| r.notation.as_str()).collect();
        assert_eq!(notations, ["1d6+3", "1d20<=13", "1d20<=7"]);
        assert_eq!(doc.calendar.months.len(), 13);
    }

    #[test]
    fn actions_change_the_text() {
        let run = |text: &str, action: Action, values: &[u32]| {
            let doc = parse(text, "x");
            apply(text, &doc, &action, &mut Fixed(values.to_vec(), 0), &now()).unwrap()
        };
        let t = run(TEMPLATE_EN, Action::Roll { notation: "1d20+5".into(), who: "Aria".into(), label: "Stealth".into() }, &[12]);
        assert!(t.ends_with("```log\n— 2026-11-14 —\n[20:15] Aria: Stealth 1d20+5 → **17** ([12] + 5)\n```\n"), "{t}");
        let t = run(&t, Action::Hp { name: "Aria".into(), delta: -5 }, &[1]);
        assert!(t.contains("hp: 19/24\n"));
        assert!(t.contains("[20:15] Aria: HP 24 → 19"));
        let t = run(&t, Action::Time { minutes: 90 }, &[1]);
        assert!(t.contains("```session\nplayed: 0h 00m 00s\nworld: 0001-01-01 09:30\n```"), "{t}");
        let t = run(&t, Action::SetTime("1492-3-15 14:30".into()), &[1]);
        let doc = parse(&t, "x");
        assert_eq!(doc.calendar.label(doc.session.world.unwrap()), "15. Ches 1492, 14:30");
        let t = run(&t, Action::Played(3725), &[1]);
        assert!(t.contains("played: 1h 02m 05s"));
        // Initiative: two players and three skeletons (chapter 2).
        let t = run(&t, Action::Initiative { chapter: 2 }, &[10, 15, 3, 20, 7]);
        let doc = parse(&t, "x");
        let order: Vec<(&str, i64)> = doc.session.order.iter().map(|c| (c.name.as_str(), c.init)).collect();
        assert_eq!(order, [("Skeleton 2", 22), ("Borin", 15), ("Aria", 13), ("Skeleton 3", 9), ("Skeleton 1", 5)]);
        assert_eq!(doc.session.order[0].hp, Some((13, 13)));
        let t = run(&t, Action::Hp { name: "Skeleton 2".into(), delta: -6 }, &[1]);
        assert!(t.contains("- Skeleton 2: 22 (7/13)"));
        let mut t = t;
        for _ in 0..5 {
            t = run(&t, Action::Next, &[1]);
        }
        let doc = parse(&t, "x");
        assert_eq!((doc.session.round, doc.session.turn), (2, 0));
        let t = run(&t, Action::EndCombat, &[1]);
        assert!(parse(&t, "x").session.order.is_empty());
        let t = run(&t, Action::Note("The tower collapses".into()), &[1]);
        assert!(t.contains("[20:15] The tower collapses\n```"));
        assert_eq!(t.matches("```session").count(), 1);
        assert_eq!(t.matches("```log").count(), 1);
    }

    #[test]
    fn calendar_round_trip() {
        let cal = Calendar::parse("A 10, B 20").unwrap();
        let m = cal.to_minutes(3, 2, 20, 23, 59);
        assert_eq!(cal.split_minutes(m), (3, 2, 20, 23, 59));
        assert_eq!(cal.split_minutes(m + 1), (4, 1, 1, 0, 0));
        assert_eq!(cal.format_time(m), "0003-02-20 23:59");
    }
}
