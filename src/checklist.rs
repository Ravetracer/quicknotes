//! Checklists in plain text and Markdown: GitHub-style task items
//! (`- [ ] open`, `- [x] done`), toggling, list continuation and progress.

/// A task item found on a line. Offsets are in characters (as used by
/// `GtkTextIter`), relative to the line start.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Task {
    /// Offset of `[`; the box spans `box_start..box_start + 3`.
    pub box_start: usize,
    pub checked: bool,
    /// Offset of the task text (after `] `).
    pub text_start: usize,
}

/// A list item marker: indentation, bullet or number, and the following space.
struct Marker<'a> {
    indent: &'a str,
    bullet: Bullet,
    /// Byte length of indentation, marker and spaces.
    len: usize,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Bullet {
    Char(char),
    /// Number and delimiter (`.` or `)`).
    Number(u64, char),
}

fn marker(line: &str) -> Option<Marker<'_>> {
    let rest = line.trim_start_matches([' ', '\t']);
    let indent = &line[..line.len() - rest.len()];
    let (bullet, after) = match rest.chars().next()? {
        c @ ('-' | '*' | '+') => (Bullet::Char(c), &rest[1..]),
        c if c.is_ascii_digit() => {
            let digits = rest.find(|c: char| !c.is_ascii_digit()).unwrap_or(rest.len());
            if digits > 9 {
                return None;
            }
            let delim = rest[digits..].chars().next().filter(|c| matches!(c, '.' | ')'))?;
            (Bullet::Number(rest[..digits].parse().ok()?, delim), &rest[digits + 1..])
        }
        _ => return None,
    };
    // The marker must be followed by a space, or end the line (empty item).
    let spaces = after.len() - after.trim_start_matches([' ', '\t']).len();
    if spaces == 0 && !after.is_empty() {
        return None;
    }
    Some(Marker { indent, bullet, len: line.len() - after.len() + spaces })
}

/// Parses a task item (`- [ ] text`, `* [x] text`, `1. [X] text`).
pub fn parse(line: &str) -> Option<Task> {
    let m = marker(line)?;
    let rest = &line[m.len..];
    let mut chars = rest.chars();
    if chars.next() != Some('[') {
        return None;
    }
    let checked = match chars.next()? {
        ' ' => false,
        'x' | 'X' => true,
        _ => return None,
    };
    if chars.next() != Some(']') {
        return None;
    }
    let after = &rest[3..];
    if !(after.is_empty() || after.starts_with([' ', '\t'])) {
        return None;
    }
    let box_start = line[..m.len].chars().count();
    let text_start = box_start + 3 + usize::from(!after.is_empty());
    Some(Task { box_start, checked, text_start })
}

/// The line with its task box toggled; `None` if it is no task.
pub fn toggle(line: &str) -> Option<String> {
    let task = parse(line)?;
    Some(set_checked(line, task, !task.checked))
}

fn set_checked(line: &str, task: Task, checked: bool) -> String {
    let mut chars: Vec<char> = line.chars().collect();
    chars[task.box_start + 1] = if checked { 'x' } else { ' ' };
    chars.into_iter().collect()
}

/// What Enter does at the end of a list item.
#[derive(Debug, PartialEq, Eq)]
pub enum Continue {
    /// Start the next line with this prefix (e.g. `- [ ] `).
    Item(String),
    /// The item is empty: remove its marker and end the list.
    End,
}

/// List continuation for `line` (the text before the cursor). Tasks continue
/// as unchecked tasks, numbered items count up.
pub fn continuation(line: &str) -> Option<Continue> {
    let m = marker(line)?;
    let task = parse(line);
    let content = match task {
        Some(t) => line.chars().skip(t.text_start).collect::<String>(),
        None => line[m.len..].to_owned(),
    };
    if content.trim().is_empty() {
        return Some(Continue::End);
    }
    let bullet = match m.bullet {
        Bullet::Char(c) => c.to_string(),
        Bullet::Number(n, d) => format!("{}{d}", n + 1),
    };
    let check = if task.is_some() { "[ ] " } else { "" };
    Some(Continue::Item(format!("{}{bullet} {check}", m.indent)))
}

/// Ctrl+Enter on some lines: lines that are all tasks are checked (or
/// unchecked when all are done already); otherwise every non-blank line
/// becomes a task, list items keep their bullet.
pub fn toggle_lines(lines: &[&str]) -> Vec<String> {
    let tasks: Vec<Option<Task>> = lines.iter().map(|l| parse(l)).collect();
    let non_blank = || lines.iter().zip(&tasks).filter(|(l, _)| !l.trim().is_empty());
    if non_blank().all(|(_, t)| t.is_some()) && non_blank().count() > 0 {
        let check = non_blank().any(|(_, t)| t.is_some_and(|t| !t.checked));
        return lines
            .iter()
            .zip(&tasks)
            .map(|(l, t)| match t {
                Some(t) => set_checked(l, *t, check),
                None => (*l).to_owned(),
            })
            .collect();
    }
    lines
        .iter()
        .zip(&tasks)
        .map(|(l, t)| {
            if t.is_some() || l.trim().is_empty() {
                return (*l).to_owned();
            }
            match marker(l) {
                Some(m) => format!("{}[ ] {}", &l[..m.len], &l[m.len..]),
                None => {
                    let text = l.trim_start_matches([' ', '\t']);
                    format!("{}- [ ] {text}", &l[..l.len() - text.len()])
                }
            }
        })
        .collect()
}

/// Done and total tasks in `text`, ignoring fenced code blocks.
pub fn progress(text: &str) -> (usize, usize) {
    let (mut done, mut total) = (0, 0);
    let mut fence: Option<&str> = None;
    for line in text.lines() {
        let trimmed = line.trim_start();
        if let Some(f) = fence {
            if trimmed.starts_with(f) {
                fence = None;
            }
            continue;
        }
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            fence = Some(&trimmed[..3]);
            continue;
        }
        if let Some(t) = parse(line) {
            total += 1;
            done += usize::from(t.checked);
        }
    }
    (done, total)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_tasks() {
        assert_eq!(parse("- [ ] buy milk"), Some(Task { box_start: 2, checked: false, text_start: 6 }));
        assert_eq!(parse("  * [x] done").map(|t| (t.box_start, t.checked)), Some((4, true)));
        assert_eq!(parse("12. [X] numbered").map(|t| t.box_start), Some(4));
        assert_eq!(parse("- [ ]").map(|t| t.text_start), Some(5));
        assert_eq!(parse("— [ ] ä").map(|t| t.box_start), None);
        assert_eq!(parse("- ä [ ]"), None);
        assert_eq!(parse("- [ ]x"), None);
        assert_eq!(parse("-[ ] no space"), None);
        assert_eq!(parse("[ ] no marker"), None);
        assert_eq!(parse("- [y] other"), None);
    }

    #[test]
    fn box_offsets_count_characters() {
        // Indentation with tabs, multi-byte text after the box.
        assert_eq!(parse("\t- [ ] äöü").map(|t| (t.box_start, t.text_start)), Some((3, 7)));
    }

    #[test]
    fn toggles() {
        assert_eq!(toggle("- [ ] a").as_deref(), Some("- [x] a"));
        assert_eq!(toggle("- [X] a").as_deref(), Some("- [ ] a"));
        assert_eq!(toggle("- a"), None);
    }

    #[test]
    fn continues_lists() {
        assert_eq!(continuation("- [x] done"), Some(Continue::Item("- [ ] ".into())));
        assert_eq!(continuation("  * item"), Some(Continue::Item("  * ".into())));
        assert_eq!(continuation("9) nine"), Some(Continue::Item("10) ".into())));
        assert_eq!(continuation("- [ ] "), Some(Continue::End));
        assert_eq!(continuation("- "), Some(Continue::End));
        assert_eq!(continuation("plain"), None);
        assert_eq!(continuation("-not a list"), None);
    }

    #[test]
    fn toggles_line_ranges() {
        assert_eq!(toggle_lines(&["idea", "", "  - bullet", "- [x] t"]), ["- [ ] idea", "", "  - [ ] bullet", "- [x] t"]);
        assert_eq!(toggle_lines(&["- [ ] a", "- [x] b"]), ["- [x] a", "- [x] b"]);
        assert_eq!(toggle_lines(&["- [x] a", "", "- [x] b"]), ["- [ ] a", "", "- [ ] b"]);
        assert_eq!(toggle_lines(&[""]), [""]);
    }

    #[test]
    fn counts_progress_outside_code() {
        let text = "- [x] a\n- [ ] b\n```\n- [x] code\n```\n* [X] c\n";
        assert_eq!(progress(text), (2, 3));
    }
}
