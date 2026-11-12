//! Kanban boards: columns × swimlanes with cards, stored as readable Markdown.
//!
//! ```text
//! ---
//! kanban: 1
//! title: Project
//! columns:
//! - title: To do
//!   color: '#62a0ea'
//! - title: Doing
//!   color: '#f8e45c'
//!   wip: 3
//! lanes:
//! - title: Frontend
//!   collapsed: true
//! ---
//!
//! # Frontend
//!
//! ## To do
//!
//! - [ ] Card title
//!   Description, indented by two spaces
//! ```
//!
//! Lane headings (`#`) are left out for a single lane without a title.
//! Columns are matched by title, so boards can also be written by hand.

use serde::{Deserialize, Serialize};

pub const LANGUAGE: &str = "kanban";

/// Colours offered for columns (GNOME palette).
pub const PALETTE: [&str; 9] =
    ["#62a0ea", "#57e389", "#f8e45c", "#ffa348", "#ed333b", "#c061cb", "#b5835a", "#9a9996", "#2ec27e"];

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq)]
#[serde(default)]
pub struct Board {
    pub title: String,
    pub columns: Vec<Column>,
    pub lanes: Vec<Lane>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq)]
#[serde(default)]
pub struct Column {
    pub title: String,
    pub color: String,
    /// Work-in-progress limit per column; 0 = none.
    pub wip: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq)]
#[serde(default)]
pub struct Lane {
    pub title: String,
    pub collapsed: bool,
    /// Cards per column (same order and length as `Board::columns`).
    pub cells: Vec<Vec<Card>>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq)]
#[serde(default)]
pub struct Card {
    pub title: String,
    pub desc: String,
    pub done: bool,
}

/// Front matter of a board file.
#[derive(Serialize, Deserialize, Default)]
#[serde(default)]
struct Front {
    kanban: u32,
    #[serde(skip_serializing_if = "String::is_empty")]
    title: String,
    columns: Vec<FrontColumn>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    lanes: Vec<FrontLane>,
}

#[derive(Serialize, Deserialize, Default)]
#[serde(default)]
struct FrontColumn {
    title: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    color: String,
    #[serde(skip_serializing_if = "is_zero")]
    wip: u32,
}

#[derive(Serialize, Deserialize, Default)]
#[serde(default)]
struct FrontLane {
    title: String,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    collapsed: bool,
}

fn is_zero(n: &u32) -> bool {
    *n == 0
}

/// Front matter lines (without the `---` lines) and the index of the first body line.
fn split_front(lines: &[&str]) -> (Option<String>, usize) {
    if lines.first().map(|l| l.trim_end()) != Some("---") {
        return (None, 0);
    }
    match (1..lines.len()).find(|&i| matches!(lines[i].trim_end(), "---" | "...")) {
        Some(end) => (Some(lines[1..end].join("\n")), end + 1),
        None => (None, 0),
    }
}

/// The text is a board: front matter with a `kanban` key.
pub fn is_board(text: &str) -> bool {
    let lines: Vec<&str> = text.lines().take(200).collect();
    split_front(&lines).0.is_some_and(|f| f.lines().any(|l| l.starts_with("kanban:")))
}

/// The board title from the front matter.
pub fn title(text: &str) -> Option<String> {
    let lines: Vec<&str> = text.lines().take(200).collect();
    let front: Front = serde_yaml::from_str(&split_front(&lines).0?).ok()?;
    Some(front.title).filter(|t| !t.trim().is_empty())
}

fn card_line(line: &str) -> Option<Card> {
    let rest = line.strip_prefix("- ").or_else(|| line.strip_prefix("* "))?;
    let (done, title) = match rest.get(..3) {
        Some("[ ]") => (false, &rest[3..]),
        Some("[x]" | "[X]") => (true, &rest[3..]),
        _ => (false, rest),
    };
    Some(Card { title: title.trim().to_owned(), desc: String::new(), done })
}

pub fn parse(text: &str) -> Board {
    let lines: Vec<&str> = text.lines().collect();
    let (front, body) = split_front(&lines);
    let front: Front = front.and_then(|f| serde_yaml::from_str(&f).ok()).unwrap_or_default();
    let mut board = Board {
        title: front.title,
        columns: front
            .columns
            .into_iter()
            .map(|c| Column { title: c.title, color: c.color, wip: c.wip })
            .collect(),
        lanes: Vec::new(),
    };
    // Current lane and column, the open card, and blank lines inside its description.
    let (mut lane, mut col): (Option<usize>, Option<usize>) = (None, None);
    let mut card: Option<(usize, usize)> = None;
    let mut blanks = 0;
    let mut used: Vec<bool> = Vec::new();
    for line in &lines[body..] {
        if let Some(title) = line.strip_prefix("# ").or_else(|| (line.trim_end() == "#").then_some("")) {
            let i = board.lanes.len();
            let collapsed = front.lanes.get(i).is_some_and(|l| l.title == title.trim() && l.collapsed);
            board.lanes.push(Lane { title: title.trim().to_owned(), collapsed, cells: Vec::new() });
            (lane, col, card) = (Some(i), None, None);
            used = vec![false; board.columns.len()];
            continue;
        }
        if let Some(title) = line.strip_prefix("## ") {
            let title = title.trim();
            if lane.is_none() {
                board.lanes.push(Lane::default());
                used = vec![false; board.columns.len()];
                lane = Some(board.lanes.len() - 1);
            }
            // First column with this title not used in this lane yet.
            let i = match (0..board.columns.len()).find(|&i| board.columns[i].title == title && !used[i]) {
                Some(i) => i,
                None => {
                    board.columns.push(Column { title: title.to_owned(), ..Default::default() });
                    used.push(false);
                    board.columns.len() - 1
                }
            };
            used[i] = true;
            (col, card) = (Some(i), None);
            continue;
        }
        let (Some(l), Some(c)) = (lane, col) else { continue };
        if let Some(new) = card_line(line) {
            let cells = &mut board.lanes[l].cells;
            if cells.len() <= c {
                cells.resize(c + 1, Vec::new());
            }
            cells[c].push(new);
            card = Some((c, cells[c].len() - 1));
            blanks = 0;
            continue;
        }
        let Some((c, k)) = card else { continue };
        if line.trim().is_empty() {
            blanks += 1;
            continue;
        }
        let text = line.strip_prefix("  ").or_else(|| line.strip_prefix('\t')).unwrap_or(line);
        let desc = &mut board.lanes[l].cells[c][k].desc;
        if !desc.is_empty() {
            desc.push_str(&"\n".repeat(blanks + 1));
        }
        desc.push_str(text);
        blanks = 0;
    }
    if board.lanes.is_empty() {
        let mut lane = Lane::default();
        if let Some(f) = front.lanes.first() {
            lane.title = f.title.clone();
            lane.collapsed = f.collapsed;
        }
        board.lanes.push(lane);
    }
    normalize(&mut board);
    board
}

/// Every lane has a cell per column; columns have a colour.
pub fn normalize(board: &mut Board) {
    for (i, c) in board.columns.iter_mut().enumerate() {
        if c.color.is_empty() {
            c.color = PALETTE[i % PALETTE.len()].to_owned();
        }
    }
    if board.lanes.is_empty() {
        board.lanes.push(Lane::default());
    }
    for lane in &mut board.lanes {
        lane.cells.resize(board.columns.len(), Vec::new());
        for cell in &mut lane.cells {
            for card in cell.iter_mut() {
                // Titles are single lines; the rest belongs to the description.
                if let Some((first, rest)) = card.title.split_once('\n') {
                    card.desc = if card.desc.is_empty() { rest.to_owned() } else { format!("{rest}\n{}", card.desc) };
                    card.title = first.to_owned();
                }
                card.title = card.title.trim().to_owned();
                card.desc = card.desc.trim_end().to_owned();
            }
        }
    }
}

pub fn to_markdown(board: &Board) -> String {
    let lane_headings = board.lanes.len() > 1 || board.lanes.iter().any(|l| !l.title.is_empty());
    let front = Front {
        kanban: 1,
        title: board.title.clone(),
        columns: board
            .columns
            .iter()
            .map(|c| FrontColumn { title: c.title.clone(), color: c.color.clone(), wip: c.wip })
            .collect(),
        lanes: board
            .lanes
            .iter()
            .filter(|_| lane_headings)
            .map(|l| FrontLane { title: l.title.clone(), collapsed: l.collapsed })
            .collect(),
    };
    let mut out = format!("---\n{}---\n", serde_yaml::to_string(&front).unwrap_or_default());
    for lane in &board.lanes {
        if lane_headings {
            out.push_str(&format!("\n# {}\n", one_line(&lane.title)));
        }
        for (column, cards) in board.columns.iter().zip(&lane.cells) {
            // Empty columns are left out unless another column has the same
            // title (columns are matched by title when reading).
            let unique = board.columns.iter().filter(|c| c.title == column.title).count() == 1;
            if cards.is_empty() && unique {
                continue;
            }
            out.push_str(&format!("\n## {}\n", one_line(&column.title)));
            if !cards.is_empty() {
                out.push('\n');
            }
            for card in cards {
                let check = if card.done { "x" } else { " " };
                out.push_str(&format!("- [{check}] {}\n", one_line(&card.title)));
                for line in card.desc.lines() {
                    if line.trim().is_empty() {
                        out.push('\n');
                    } else {
                        out.push_str(&format!("  {line}\n"));
                    }
                }
            }
        }
    }
    out
}

fn one_line(s: &str) -> String {
    s.replace(['\n', '\r'], " ").trim().to_owned()
}

/// Number of cards and of done cards.
pub fn stats(board: &Board) -> (usize, usize) {
    let cards = board.lanes.iter().flat_map(|l| l.cells.iter().flatten());
    cards.fold((0, 0), |(n, d), c| (n + 1, d + usize::from(c.done)))
}

/// A new board: three columns and a first card.
pub fn template() -> String {
    let column = |title: &str, color: &str| Column { title: title.to_owned(), color: color.to_owned(), wip: 0 };
    let card = Card { title: t!("Drag me to another column").to_owned(), ..Default::default() };
    let board = Board {
        title: t!("Board").to_owned(),
        columns: vec![column(t!("To do"), PALETTE[0]), column(t!("Doing"), PALETTE[2]), column(t!("Done"), PALETTE[1])],
        lanes: vec![Lane { title: String::new(), collapsed: false, cells: vec![vec![card], vec![], vec![]] }],
    };
    to_markdown(&board)
}

const BOARD_CSS: &str = include_str!("../data/kanban/board.css");
const BOARD_JS: &str = include_str!("../data/kanban/board.js");

/// The board page; its content is set with `QK.update(board)`.
pub fn page(dark: bool, vars: &str) -> String {
    let strings: serde_json::Map<String, serde_json::Value> = [
        ("addCard", t!("Add card")),
        ("addColumn", t!("Column")),
        ("addLane", t!("Swimlane")),
        ("rename", t!("Rename")),
        ("delete", t!("Delete")),
        ("confirmDelete", t!("Click again to delete")),
        ("moveLeft", t!("Move left")),
        ("moveRight", t!("Move right")),
        ("moveUp", t!("Move up")),
        ("moveDown", t!("Move down")),
        ("wip", t!("Limit (WIP)")),
        ("color", t!("Colour")),
        ("newColumn", t!("New column")),
        ("newLane", t!("Swimlane {}")),
        ("boardTitle", t!("Board title")),
        ("cardHint", t!("First line: title, further lines: description")),
        ("collapse", t!("Collapse / expand")),
        ("noColumns", t!("Add a column to start.")),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_owned(), serde_json::Value::from(v)))
    .collect();
    let strings = serde_json::Value::Object(strings).to_string().replace("</", "<\\/");
    let palette = serde_json::to_string(&PALETTE).unwrap_or_default();
    let theme = if dark { "dark" } else { "light" };
    format!(
        r#"<!DOCTYPE html>
<html data-theme="{theme}" style="{vars}">
<head>
<meta charset="utf-8">
<style>{BOARD_CSS}</style>
</head>
<body>
<div id="app"></div>
<script>window.QK_STRINGS = {strings}; window.QK_PALETTE = {palette};</script>
<script>{BOARD_JS}</script>
</body>
</html>"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Board {
        let card = |t: &str, d: &str, done| Card { title: t.into(), desc: d.into(), done };
        Board {
            title: "Project".into(),
            columns: vec![
                Column { title: "To do".into(), color: "#62a0ea".into(), wip: 0 },
                Column { title: "Doing".into(), color: "#f8e45c".into(), wip: 3 },
            ],
            lanes: vec![
                Lane {
                    title: "Frontend".into(),
                    collapsed: true,
                    cells: vec![vec![card("A", "line 1\n\nline 3", false)], vec![card("B", "", true)]],
                },
                Lane { title: "Backend".into(), collapsed: false, cells: vec![vec![], vec![card("C", "", false)]] },
            ],
        }
    }

    #[test]
    fn round_trips() {
        let board = sample();
        let md = to_markdown(&board);
        assert!(md.starts_with("---\nkanban: 1\ntitle: Project\n"), "{md}");
        assert!(md.contains("# Frontend\n\n## To do\n\n- [ ] A\n  line 1\n\n  line 3\n\n## Doing\n\n- [x] B\n"), "{md}");
        assert_eq!(parse(&md), board);
        assert!(is_board(&md));
    }

    #[test]
    fn single_untitled_lane_has_no_heading() {
        let mut board = sample();
        board.lanes.truncate(1);
        board.lanes[0].title.clear();
        board.lanes[0].collapsed = false;
        let md = to_markdown(&board);
        assert!(!md.contains("\n# "), "{md}");
        assert!(!md.contains("lanes:"), "{md}");
        assert_eq!(parse(&md), board);
    }

    #[test]
    fn hand_written_boards() {
        let md = "## Todo\n- one\n* [x] two\n  more\n\n## Done\n- [ ] three\nloose text\n";
        let b = parse(md);
        assert_eq!(b.columns.iter().map(|c| c.title.as_str()).collect::<Vec<_>>(), ["Todo", "Done"]);
        assert_eq!(b.lanes.len(), 1);
        let cells = &b.lanes[0].cells;
        assert_eq!(cells[0].len(), 2);
        assert!(cells[0][1].done);
        assert_eq!(cells[0][1].desc, "more");
        assert_eq!(cells[1][0].desc, "loose text");
        assert!(!b.columns[0].color.is_empty());
        assert!(!is_board(md));
    }

    #[test]
    fn duplicate_column_titles_stay_apart() {
        let mut board = sample();
        board.columns[1].title = "To do".into();
        assert_eq!(parse(&to_markdown(&board)), board);
    }

    #[test]
    fn titles_are_single_lines() {
        let mut board = sample();
        board.lanes[0].cells[0][0].title = "Title\nmore text".into();
        board.lanes[0].cells[0][0].desc = "old".into();
        normalize(&mut board);
        assert_eq!(board.lanes[0].cells[0][0].title, "Title");
        assert_eq!(board.lanes[0].cells[0][0].desc, "more text\nold");
    }

    #[test]
    fn template_is_a_board() {
        let b = parse(&template());
        assert_eq!(b.columns.len(), 3);
        assert_eq!(stats(&b), (1, 0));
    }
}
