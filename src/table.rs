//! Markdown tables (GitHub style): parsing, formatting and editing
//! operations in the spirit of Obsidian's "Advanced Tables".
//!
//! All positions are character offsets (as used by `GtkTextIter`).

use unicode_width::UnicodeWidthStr;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Align {
    #[default]
    None,
    Left,
    Center,
    Right,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Op {
    /// Format only, cursor stays in its cell.
    Format,
    /// Tab: next cell (a new column after the last one).
    NextCell,
    /// Shift+Tab: previous cell.
    PrevCell,
    /// Enter: same column in the next row (new row at the end; an empty last
    /// row is removed and the cursor leaves the table).
    NextRow,
    InsertRowAbove,
    InsertRowBelow,
    DeleteRow,
    MoveRowUp,
    MoveRowDown,
    InsertColumnLeft,
    InsertColumnRight,
    DeleteColumn,
    MoveColumnLeft,
    MoveColumnRight,
    /// Sorts the body rows by the cursor column (`true`: ascending).
    Sort(bool),
    Align(Align),
}

/// Result of an operation: the new table lines and where the cursor goes.
#[derive(Debug, PartialEq, Eq)]
pub struct Edit {
    pub lines: Vec<String>,
    pub cursor_line: usize,
    pub cursor_col: usize,
    /// Characters selected from the cursor on (the cell text after moving).
    pub select: usize,
}

#[derive(Clone, Debug, PartialEq)]
struct Table {
    indent: String,
    /// `rows[0]` is the header.
    rows: Vec<Vec<String>>,
    aligns: Vec<Align>,
}

/// Cursor inside the table: row (0 = header), column and offset in the cell text.
#[derive(Clone, Copy, Debug)]
struct Pos {
    row: usize,
    col: usize,
    offset: usize,
}

/// Cursor at the start of a cell, selecting its text (like Advanced Tables).
const SELECT: usize = usize::MAX;

pub fn is_table_line(line: &str) -> bool {
    line.trim_start().starts_with('|')
}

/// Byte offsets of the unescaped pipes in `s`.
fn pipes(s: &str) -> Vec<usize> {
    let mut out = Vec::new();
    let mut escaped = false;
    for (i, c) in s.char_indices() {
        match c {
            _ if escaped => escaped = false,
            '\\' => escaped = true,
            '|' => out.push(i),
            _ => {}
        }
    }
    out
}

fn split_cells(line: &str) -> Vec<String> {
    let t = line.trim();
    let inner = t.strip_prefix('|').unwrap_or(t);
    let mut cells = Vec::new();
    let mut start = 0;
    for p in pipes(inner) {
        cells.push(inner[start..p].trim().to_owned());
        start = p + 1;
    }
    // Without a closing pipe the rest is the last cell.
    if start < inner.len() || cells.is_empty() {
        cells.push(inner[start..].trim().to_owned());
    }
    cells
}

fn separator_align(cell: &str) -> Option<Align> {
    let c = cell.trim();
    let (left, rest) = match c.strip_prefix(':') {
        Some(r) => (true, r),
        None => (false, c),
    };
    let (right, dashes) = match rest.strip_suffix(':') {
        Some(r) => (true, r),
        None => (false, rest),
    };
    if dashes.is_empty() || !dashes.chars().all(|c| c == '-') {
        return None;
    }
    Some(match (left, right) {
        (true, true) => Align::Center,
        (true, false) => Align::Left,
        (false, true) => Align::Right,
        (false, false) => Align::None,
    })
}

fn is_separator(cells: &[String]) -> bool {
    !cells.is_empty() && cells.iter().all(|c| separator_align(c).is_some())
}

fn width(s: &str) -> usize {
    UnicodeWidthStr::width(s)
}

impl Table {
    /// Parses table lines; also returns whether line 1 is a separator.
    fn parse(lines: &[&str]) -> (Self, bool) {
        let indent: String = lines
            .first()
            .map(|l| l.chars().take_while(|c| *c == ' ' || *c == '\t').collect())
            .unwrap_or_default();
        let mut rows: Vec<Vec<String>> = lines.iter().map(|l| split_cells(l)).collect();
        let mut aligns = Vec::new();
        let has_sep = rows.len() >= 2 && is_separator(&rows[1]);
        if has_sep {
            aligns = rows.remove(1).iter().map(|c| separator_align(c).unwrap_or_default()).collect();
        }
        let mut table = Table { indent, rows, aligns };
        table.normalize();
        (table, has_sep)
    }

    fn cols(&self) -> usize {
        self.aligns.len()
    }

    /// Same number of cells in every row (at least one column and one row).
    fn normalize(&mut self) {
        if self.rows.is_empty() {
            self.rows.push(Vec::new());
        }
        let cols = self.rows.iter().map(Vec::len).max().unwrap_or(0).max(self.aligns.len()).max(1);
        for row in &mut self.rows {
            row.resize(cols, String::new());
        }
        self.aligns.resize(cols, Align::None);
    }

    fn widths(&self) -> Vec<usize> {
        (0..self.cols())
            .map(|c| self.rows.iter().map(|r| width(&r[c])).max().unwrap_or(0).max(3))
            .collect()
    }

    /// Rendered line of `row` (0 = header) in the output.
    fn line_of(row: usize) -> usize {
        if row == 0 {
            0
        } else {
            row + 1
        }
    }

    /// Renders the table and returns the lines plus the cursor position for `pos`.
    fn render(&self, pos: Pos) -> Edit {
        let widths = self.widths();
        let mut lines = Vec::with_capacity(self.rows.len() + 1);
        let (mut cursor_col, mut select) = (0, 0);
        for (r, row) in self.rows.iter().enumerate() {
            let mut line = format!("{}|", self.indent);
            for (c, cell) in row.iter().enumerate() {
                let pad = widths[c] - width(cell);
                let left = match self.aligns[c] {
                    Align::Right => pad,
                    Align::Center => pad / 2,
                    Align::None | Align::Left => 0,
                };
                line.push(' ');
                line.push_str(&" ".repeat(left));
                if r == pos.row && c == pos.col {
                    let len = cell.chars().count();
                    cursor_col = line.chars().count();
                    if pos.offset == SELECT {
                        select = len;
                    } else {
                        cursor_col += pos.offset.min(len);
                    }
                }
                line.push_str(cell);
                line.push_str(&" ".repeat(pad - left));
                line.push_str(" |");
            }
            lines.push(line);
            if r == 0 {
                let mut sep = format!("{}|", self.indent);
                for (c, w) in widths.iter().enumerate() {
                    let (l, r) = match self.aligns[c] {
                        Align::None => ("", ""),
                        Align::Left => (":", ""),
                        Align::Right => ("", ":"),
                        Align::Center => (":", ":"),
                    };
                    let dashes = w - l.len() - r.len();
                    sep.push_str(&format!(" {l}{}{r} |", "-".repeat(dashes)));
                }
                lines.push(sep);
            }
        }
        Edit { lines, cursor_line: Self::line_of(pos.row), cursor_col, select }
    }
}

/// Maps a cursor in the source lines to a cell.
fn locate(lines: &[&str], has_sep: bool, table: &Table, line: usize, col: usize) -> Pos {
    let row = match (has_sep, line) {
        (false, l) => l,
        (true, 0) => 0,
        // On the separator: the first body row (or the header).
        (true, 1) => usize::from(table.rows.len() > 1),
        (true, l) => l - 1,
    }
    .min(table.rows.len() - 1);
    let text = lines.get(line).copied().unwrap_or_default();
    let byte = text.char_indices().nth(col).map_or(text.len(), |(i, _)| i);
    let before: Vec<usize> = pipes(text).into_iter().filter(|&p| p < byte).collect();
    let cell = before.len().saturating_sub(1).min(table.cols() - 1);
    // Offset in the trimmed cell text.
    let offset = before.last().map_or(0, |&p| {
        let after_pipe = &text[p + 1..byte];
        let leading = after_pipe.len() - after_pipe.trim_start().len();
        after_pipe[leading..].chars().count()
    });
    // On the separator line: select the cell text.
    let offset = if has_sep && line == 1 { SELECT } else { offset };
    Pos { row, col: cell, offset }
}

/// Applies `op` to the table made of `lines` with the cursor at
/// `cursor_line`/`cursor_col` (relative to the table). `None` if the
/// operation does not apply (e.g. deleting the only column).
pub fn apply(lines: &[&str], cursor_line: usize, cursor_col: usize, op: Op) -> Option<Edit> {
    if lines.is_empty() {
        return None;
    }
    let (mut t, has_sep) = Table::parse(lines);
    let mut pos = locate(lines, has_sep, &t, cursor_line, cursor_col);
    let (rows, cols) = (t.rows.len(), t.cols());
    let end = SELECT;
    let empty_row = || vec![String::new(); cols];
    match op {
        Op::Format => {}
        Op::NextCell => {
            if pos.col + 1 == cols {
                for row in &mut t.rows {
                    row.push(String::new());
                }
                t.aligns.push(Align::None);
            }
            pos = Pos { col: pos.col + 1, offset: end, ..pos };
        }
        Op::PrevCell => {
            pos = match (pos.row, pos.col) {
                (_, c) if c > 0 => Pos { col: c - 1, offset: end, ..pos },
                (r, _) if r > 0 => Pos { row: r - 1, col: cols - 1, offset: end },
                _ => Pos { offset: end, ..pos },
            };
        }
        Op::NextRow => {
            if pos.row + 1 < rows {
                pos = Pos { row: pos.row + 1, offset: end, ..pos };
            } else if pos.row > 0 && t.rows[pos.row].iter().all(String::is_empty) {
                // Leave the table: drop the empty row and continue after a
                // blank line (text right below a table would still belong to it).
                t.rows.pop();
                let mut edit = t.render(Pos { row: 0, col: 0, offset: 0 });
                edit.lines.extend([String::new(), String::new()]);
                edit.cursor_line = edit.lines.len() - 1;
                edit.cursor_col = 0;
                edit.select = 0;
                return Some(edit);
            } else {
                t.rows.push(empty_row());
                pos = Pos { row: rows, offset: end, ..pos };
            }
        }
        Op::InsertRowAbove | Op::InsertRowBelow => {
            let at = if op == Op::InsertRowAbove { pos.row.max(1) } else { pos.row + 1 };
            t.rows.insert(at, empty_row());
            pos = Pos { row: at, offset: end, ..pos };
        }
        Op::DeleteRow => {
            if rows == 1 {
                return None;
            }
            t.rows.remove(pos.row);
            pos = Pos { row: pos.row.min(rows - 2), offset: end, ..pos };
        }
        Op::MoveRowUp => {
            if pos.row < 2 {
                return None;
            }
            t.rows.swap(pos.row, pos.row - 1);
            pos.row -= 1;
        }
        Op::MoveRowDown => {
            if pos.row == 0 || pos.row + 1 >= rows {
                return None;
            }
            t.rows.swap(pos.row, pos.row + 1);
            pos.row += 1;
        }
        Op::InsertColumnLeft | Op::InsertColumnRight => {
            let at = pos.col + usize::from(op == Op::InsertColumnRight);
            for row in &mut t.rows {
                row.insert(at, String::new());
            }
            t.aligns.insert(at, Align::None);
            pos = Pos { col: at, offset: end, ..pos };
        }
        Op::DeleteColumn => {
            if cols == 1 {
                return None;
            }
            for row in &mut t.rows {
                row.remove(pos.col);
            }
            t.aligns.remove(pos.col);
            pos = Pos { col: pos.col.min(cols - 2), offset: end, ..pos };
        }
        Op::MoveColumnLeft | Op::MoveColumnRight => {
            let other = match op {
                Op::MoveColumnLeft if pos.col > 0 => pos.col - 1,
                Op::MoveColumnRight if pos.col + 1 < cols => pos.col + 1,
                _ => return None,
            };
            for row in &mut t.rows {
                row.swap(pos.col, other);
            }
            t.aligns.swap(pos.col, other);
            pos.col = other;
        }
        Op::Sort(ascending) => {
            let col = pos.col;
            t.rows[1..].sort_by(|a, b| {
                let o = compare_cells(&a[col], &b[col]);
                if ascending {
                    o
                } else {
                    o.reverse()
                }
            });
        }
        Op::Align(a) => t.aligns[pos.col] = a,
    }
    Some(t.render(pos))
}

/// Numbers compare numerically, everything else case-insensitively;
/// empty cells go last.
fn compare_cells(a: &str, b: &str) -> std::cmp::Ordering {
    use std::cmp::Ordering;
    let num = |s: &str| s.replace(',', ".").trim_end_matches('%').trim().parse::<f64>().ok();
    match (a.is_empty(), b.is_empty()) {
        (true, true) => return Ordering::Equal,
        (true, false) => return Ordering::Greater,
        (false, true) => return Ordering::Less,
        _ => {}
    }
    match (num(a), num(b)) {
        (Some(x), Some(y)) => x.partial_cmp(&y).unwrap_or(Ordering::Equal),
        _ => a.to_lowercase().cmp(&b.to_lowercase()),
    }
}

/// Template for *Insert Table*: header with `cols` columns and one empty row.
pub fn template(cols: usize, indent: &str) -> Edit {
    let header: Vec<String> = (1..=cols).map(|i| t!("Column {}", i)).collect();
    let t = Table {
        indent: indent.to_owned(),
        rows: vec![header, vec![String::new(); cols]],
        aligns: vec![Align::None; cols],
    };
    t.render(Pos { row: 1, col: 0, offset: 0 })
}

/// Formats every table (with a separator line) outside fenced code blocks.
pub fn format_all(text: &str) -> String {
    let lines: Vec<&str> = text.split('\n').collect();
    let mut out: Vec<String> = Vec::with_capacity(lines.len());
    let mut fence: Option<&str> = None;
    let mut i = 0;
    while i < lines.len() {
        let trimmed = lines[i].trim_start();
        if let Some(f) = fence {
            if trimmed.starts_with(f) {
                fence = None;
            }
        } else if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            fence = Some(&trimmed[..3]);
        } else if is_table_line(lines[i]) {
            let end = (i..lines.len()).find(|&j| !is_table_line(lines[j])).unwrap_or(lines.len());
            let block: Vec<&str> = lines[i..end].iter().map(|l| l.trim_end_matches('\r')).collect();
            if block.len() >= 2 && is_separator(&split_cells(block[1])) {
                if let Some(edit) = apply(&block, 0, 0, Op::Format) {
                    out.extend(edit.lines);
                    i = end;
                    continue;
                }
            }
        }
        out.push(lines[i].to_owned());
        i += 1;
    }
    out.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(src: &str, line: usize, col: usize, op: Op) -> Edit {
        let lines: Vec<&str> = src.lines().collect();
        apply(&lines, line, col, op).expect("applies")
    }

    fn text(e: &Edit) -> String {
        e.lines.join("\n")
    }

    #[test]
    fn formats_and_adds_separator() {
        let e = run("|a|bb|\n|x|", 0, 1, Op::Format);
        assert_eq!(text(&e), "| a   | bb  |\n| --- | --- |\n| x   |     |");
        assert_eq!((e.cursor_line, e.cursor_col), (0, 2));
    }

    #[test]
    fn keeps_alignment_and_pads_accordingly() {
        let e = run("| l | c | r |\n|:-|:-:|-:|\n| left | mid | right |", 0, 0, Op::Format);
        assert_eq!(
            text(&e),
            "| l    |  c  |     r |\n| :--- | :-: | ----: |\n| left | mid | right |"
        );
    }

    #[test]
    fn wide_characters_and_escaped_pipes() {
        let e = run("| 日本 | a\\|b |\n|---|---|\n| x | y |", 0, 0, Op::Format);
        assert_eq!(text(&e), "| 日本 | a\\|b |\n| ---- | ---- |\n| x    | y    |");
    }

    #[test]
    fn tab_moves_and_extends() {
        let src = "| a | b |\n| --- | --- |\n| 1 | 2 |";
        // The next cell's text is selected.
        let e = run(src, 0, 3, Op::NextCell);
        assert_eq!((e.cursor_line, e.cursor_col, e.select), (0, 8, 1));
        // After the last cell: a new column.
        let e = run(src, 2, 8, Op::NextCell);
        assert_eq!(e.lines[0], "| a   | b   |     |");
        assert_eq!((e.cursor_line, e.cursor_col, e.select), (2, 14, 0));
        let e = run("| Name", 0, 6, Op::NextCell);
        assert_eq!(text(&e), "| Name |     |\n| ---- | --- |");
        assert_eq!((e.cursor_line, e.cursor_col), (0, 9));
    }

    #[test]
    fn shift_tab_goes_back() {
        let src = "| a | b |\n| --- | --- |\n| 1 | 2 |";
        let e = run(src, 2, 2, Op::PrevCell);
        assert_eq!((e.cursor_line, e.cursor_col, e.select), (0, 8, 1));
    }

    #[test]
    fn enter_moves_down_adds_rows_and_leaves_empty_rows() {
        let src = "| a | b |\n| --- | --- |\n| 1 | 2 |";
        let e = run(src, 0, 8, Op::NextRow);
        assert_eq!((e.cursor_line, e.cursor_col, e.select), (2, 8, 1));
        let e = run(src, 2, 2, Op::NextRow);
        assert_eq!(e.lines.len(), 4);
        let e = run(&format!("{src}\n|  |  |"), 3, 2, Op::NextRow);
        assert_eq!(e.lines.len(), 5);
        assert_eq!(e.lines[3..], ["", ""]);
        assert_eq!((e.cursor_line, e.cursor_col), (4, 0));
    }

    #[test]
    fn rows_and_columns() {
        let src = "| a | b |\n| --- | --- |\n| 1 | 2 |\n| 3 | 4 |";
        assert_eq!(run(src, 3, 2, Op::MoveRowUp).lines[2], "| 3   | 4   |");
        assert!(apply(&src.lines().collect::<Vec<_>>(), 2, 2, Op::MoveRowUp).is_none());
        assert_eq!(run(src, 2, 2, Op::DeleteRow).lines.len(), 3);
        assert_eq!(run(src, 0, 2, Op::InsertRowAbove).lines[2], "|     |     |");
        assert_eq!(run(src, 2, 2, Op::InsertRowBelow).lines[3], "|     |     |");
        assert_eq!(run(src, 2, 2, Op::MoveColumnRight).lines[0], "| b   | a   |");
        assert_eq!(run(src, 2, 2, Op::InsertColumnLeft).lines[0], "|     | a   | b   |");
        assert_eq!(run(src, 2, 7, Op::DeleteColumn).lines[0], "| a   |");
        let one = "| a |\n| --- |";
        assert!(apply(&one.lines().collect::<Vec<_>>(), 0, 2, Op::DeleteColumn).is_none());
    }

    #[test]
    fn sorts_numbers_and_text() {
        let src = "| n | s |\n|---|---|\n| 10 | b |\n| 9 | A |\n|  | c |";
        let e = run(src, 2, 2, Op::Sort(true));
        assert_eq!(e.lines[2..].iter().map(|l| l.split('|').nth(1).unwrap().trim()).collect::<Vec<_>>(), ["9", "10", ""]);
        let e = run(src, 2, 7, Op::Sort(false));
        assert_eq!(e.lines[2..].iter().map(|l| l.split('|').nth(2).unwrap().trim()).collect::<Vec<_>>(), ["c", "b", "A"]);
    }

    #[test]
    fn aligns_column() {
        let e = run("| a | b |\n|---|---|", 0, 7, Op::Align(Align::Right));
        assert_eq!(e.lines[1], "| --- | --: |");
    }

    #[test]
    fn keeps_indentation() {
        let e = run("  |a|\n  |-|", 0, 3, Op::Format);
        assert_eq!(text(&e), "  | a   |\n  | --- |");
    }

    #[test]
    fn formats_all_tables_outside_code() {
        let src = "Text | x\n\n|a|b|\n|-|-|\n|1|2|\n\n```\n|a|b|\n|-|-|\n```\n";
        let out = format_all(src);
        assert!(out.contains("| a   | b   |\n| --- | --- |\n| 1   | 2   |"));
        assert!(out.contains("```\n|a|b|\n|-|-|\n```\n"));
        assert!(out.starts_with("Text | x\n"));
    }

    #[test]
    fn template_has_header_and_row() {
        let e = template(2, "");
        assert_eq!(text(&e), "| Column 1 | Column 2 |\n| -------- | -------- |\n|          |          |");
        assert_eq!((e.cursor_line, e.cursor_col), (2, 2));
    }
}
