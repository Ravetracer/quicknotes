//! Markdown formatting commands of the formatting toolbar: inline styles,
//! block types, links and text colours.
//!
//! Commands work on the lines covering the selection (joined by `\n`, no
//! trailing newline); offsets are byte offsets into that text. Inline styles
//! toggle: applying one to text that already has it removes it.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Op {
    Bold,
    Italic,
    Strike,
    Code,
    /// Heading level 1–6; 0 turns headings back into normal text.
    Heading(u8),
    Quote,
    Bullet,
    Numbered,
    Task,
    CodeBlock,
    Link,
    Image,
    Rule,
    /// Text colour (`#rrggbb`) as an inline `<span>`; `None` removes it.
    Color(Option<&'static str>),
}

/// New text of the lines and the selection in it.
#[derive(Debug, PartialEq, Eq)]
pub struct Edit {
    pub text: String,
    pub sel: (usize, usize),
}

/// Text colours offered by the toolbar: label and value. Mid tones that stay
/// readable on light and dark backgrounds.
pub const COLORS: &[(&str, &str)] = &[
    ("Red", "#e01b24"),
    ("Orange", "#e66100"),
    ("Yellow", "#c88800"),
    ("Green", "#26a269"),
    ("Teal", "#2190a4"),
    ("Blue", "#3584e4"),
    ("Purple", "#9141ac"),
    ("Pink", "#c061cb"),
    ("Brown", "#986a44"),
    ("Gray", "#77767b"),
];

const COLOR_OPEN: &str = "<span style=\"color: ";
const SPAN_CLOSE: &str = "</span>";
const URL_PLACEHOLDER: &str = "https://";

pub fn apply(text: &str, sel: (usize, usize), op: Op) -> Edit {
    let sel = (sel.0.min(text.len()), sel.1.min(text.len()));
    let sel = (sel.0.min(sel.1), sel.0.max(sel.1));
    match op {
        Op::Heading(_) | Op::Quote | Op::Bullet | Op::Numbered | Op::Task => lines_op(text, sel, op),
        Op::CodeBlock => code_block(text, sel),
        Op::Rule => rule(text),
        Op::Link | Op::Image => inline_line(text, sel, op),
        _ if text[sel.0..sel.1].contains('\n') => per_line(text, sel, op),
        _ => inline_line(text, sel, op),
    }
}

// ----- inline styles ------------------------------------------------------

/// Applies an inline style to the selected part of every line.
fn per_line(text: &str, sel: (usize, usize), op: Op) -> Edit {
    let mut out = String::new();
    let (mut start, mut end) = (None, 0);
    let mut pos = 0;
    for (i, line) in text.split('\n').enumerate() {
        if i > 0 {
            out.push('\n');
        }
        let (ls, le) = (pos, pos + line.len());
        pos = le + 1;
        let (s, e) = (sel.0.max(ls), sel.1.min(le));
        if s >= e || line[s - ls..e - ls].trim().is_empty() {
            out.push_str(line);
            continue;
        }
        let new = inline_line(line, (s - ls, e - ls), op).text;
        // Select the changed part including the markup, so that the same
        // command toggles it off again.
        let prefix: usize = line.chars().zip(new.chars()).take_while(|(a, b)| a == b).map(|(c, _)| c.len_utf8()).sum();
        let suffix: usize = line[prefix..]
            .chars()
            .rev()
            .zip(new[prefix..].chars().rev())
            .take_while(|(a, b)| a == b)
            .map(|(c, _)| c.len_utf8())
            .sum();
        let base = out.len();
        start.get_or_insert(base + prefix);
        end = base + new.len() - suffix;
        out.push_str(&new);
    }
    let start = start.unwrap_or(sel.0);
    Edit { text: out, sel: (start, end.max(start)) }
}

fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Word around `pos`, if the cursor touches one.
fn word_at(line: &str, pos: usize) -> Option<(usize, usize)> {
    let start = line[..pos]
        .char_indices()
        .rev()
        .take_while(|(_, c)| is_word_char(*c))
        .last()
        .map_or(pos, |(i, _)| i);
    let end = line[pos..].char_indices().find(|(_, c)| !is_word_char(*c)).map_or(line.len(), |(i, _)| pos + i);
    (start < end).then_some((start, end))
}

/// Selection without surrounding whitespace.
fn trim_sel(line: &str, (s, e): (usize, usize)) -> (usize, usize) {
    let inner = &line[s..e];
    let start = s + (inner.len() - inner.trim_start().len());
    let end = e - (inner.len() - inner.trim_end().len());
    (start, end.max(start))
}

fn replace(line: &str, (s, e): (usize, usize), new: &str, sel: (usize, usize)) -> Edit {
    let text = format!("{}{new}{}", &line[..s], &line[e..]);
    Edit { text, sel: (s + sel.0, s + sel.1) }
}

fn inline_line(line: &str, sel: (usize, usize), op: Op) -> Edit {
    let sel = if sel.0 == sel.1 {
        match word_at(line, sel.0) {
            Some(word) => word,
            None => return insert_empty(line, sel.0, op),
        }
    } else {
        trim_sel(line, sel)
    };
    match op {
        Op::Bold => toggle_stars(line, sel, 2),
        Op::Italic => toggle_stars(line, sel, 1),
        Op::Strike => toggle_wrap(line, sel, "~~"),
        Op::Code => toggle_wrap(line, sel, "`"),
        Op::Color(color) => toggle_color(line, sel, color),
        Op::Link | Op::Image => link(line, sel, op == Op::Image),
        _ => Edit { text: line.to_owned(), sel },
    }
}

/// Inline command without text to apply it to: inserts the empty markup
/// with the cursor where the text goes.
fn insert_empty(line: &str, pos: usize, op: Op) -> Edit {
    let (open, close) = match op {
        Op::Bold => ("**".to_owned(), "**".to_owned()),
        Op::Italic => ("*".into(), "*".into()),
        Op::Strike => ("~~".into(), "~~".into()),
        Op::Code => ("`".into(), "`".into()),
        Op::Link => ("[".into(), format!("]({URL_PLACEHOLDER})")),
        Op::Image => ("![".into(), format!("]({URL_PLACEHOLDER})")),
        Op::Color(Some(c)) => (color_open(c), SPAN_CLOSE.into()),
        _ => return Edit { text: line.to_owned(), sel: (pos, pos) },
    };
    let cursor = open.len();
    replace(line, (pos, pos), &format!("{open}{close}"), (cursor, cursor))
}

fn wrap(line: &str, (s, e): (usize, usize), open: &str, close: &str) -> Edit {
    let inner = &line[s..e];
    replace(line, (s, e), &format!("{open}{inner}{close}"), (open.len(), open.len() + inner.len()))
}

fn toggle_wrap(line: &str, (s, e): (usize, usize), marker: &str) -> Edit {
    let inner = &line[s..e];
    let m = marker.len();
    if inner.len() >= 2 * m && inner.starts_with(marker) && inner.ends_with(marker) {
        let content = &inner[m..inner.len() - m];
        return replace(line, (s, e), content, (0, content.len()));
    }
    if line[..s].ends_with(marker) && line[e..].starts_with(marker) {
        return replace(line, (s - m, e + m), inner, (0, inner.len()));
    }
    wrap(line, (s, e), marker, marker)
}

fn leading_stars(s: &str) -> usize {
    s.bytes().take_while(|&b| b == b'*').count()
}

fn trailing_stars(s: &str) -> usize {
    s.bytes().rev().take_while(|&b| b == b'*').count()
}

/// Bold (`n` = 2) and italic (`n` = 1) share the marker: a run of three
/// stars is both, so italic is present when the run length is odd.
fn toggle_stars(line: &str, (s, e): (usize, usize), n: usize) -> Edit {
    let has = |run: usize| if n == 2 { run >= 2 } else { run % 2 == 1 };
    let inner = &line[s..e];
    let run_in = leading_stars(inner).min(trailing_stars(inner));
    if inner.len() > 2 * run_in && has(run_in) {
        let content = &inner[n..inner.len() - n];
        return replace(line, (s, e), content, (0, content.len()));
    }
    let run_out = trailing_stars(&line[..s]).min(leading_stars(&line[e..]));
    if has(run_out) {
        return replace(line, (s - n, e + n), inner, (0, inner.len()));
    }
    let marker = "*".repeat(n);
    wrap(line, (s, e), &marker, &marker)
}

fn color_open(color: &str) -> String {
    format!("{COLOR_OPEN}{color}\">")
}

/// Length of a colour span's opening tag at the start of `s`.
fn color_open_len(s: &str) -> Option<usize> {
    let rest = s.strip_prefix(COLOR_OPEN)?;
    let end = rest.find(['>', '<'])?;
    (rest.as_bytes()[end] == b'>').then_some(COLOR_OPEN.len() + end + 1)
}

/// Start of a colour span's opening tag that `before` ends with.
fn color_open_before(before: &str) -> Option<usize> {
    let start = before.rfind(COLOR_OPEN)?;
    (color_open_len(&before[start..]) == Some(before.len() - start)).then_some(start)
}

/// Removes all colour spans from `s` (and as many closing tags).
fn strip_colors(s: &str) -> String {
    let mut out = String::new();
    let mut open = 0;
    let mut rest = s;
    while !rest.is_empty() {
        if let Some(len) = color_open_len(rest) {
            open += 1;
            rest = &rest[len..];
        } else if open > 0 && rest.starts_with(SPAN_CLOSE) {
            open -= 1;
            rest = &rest[SPAN_CLOSE.len()..];
        } else {
            let c = rest.chars().next().unwrap_or_default();
            out.push(c);
            rest = &rest[c.len_utf8()..];
        }
    }
    out
}

fn toggle_color(line: &str, (s, e): (usize, usize), color: Option<&str>) -> Edit {
    let inner = &line[s..e];
    let new_open = color.map(color_open);
    // The selection is a whole coloured span.
    if let Some(len) = color_open_len(inner) {
        if inner.ends_with(SPAN_CLOSE) && inner.len() >= len + SPAN_CLOSE.len() {
            let content = &inner[len..inner.len() - SPAN_CLOSE.len()];
            if !content.contains(COLOR_OPEN) {
                return match &new_open {
                    Some(open) => {
                        replace(line, (s, e), &format!("{open}{content}{SPAN_CLOSE}"), (open.len(), open.len() + content.len()))
                    }
                    None => replace(line, (s, e), content, (0, content.len())),
                };
            }
        }
    }
    // The selection is the content of a coloured span.
    if let Some(start) = color_open_before(&line[..s]) {
        if line[e..].starts_with(SPAN_CLOSE) {
            let range = (start, e + SPAN_CLOSE.len());
            return match &new_open {
                Some(open) => replace(line, range, &format!("{open}{inner}{SPAN_CLOSE}"), (open.len(), open.len() + inner.len())),
                None => replace(line, range, inner, (0, inner.len())),
            };
        }
    }
    let plain = strip_colors(inner);
    match &new_open {
        Some(open) => {
            replace(line, (s, e), &format!("{open}{plain}{SPAN_CLOSE}"), (open.len(), open.len() + plain.len()))
        }
        None => replace(line, (s, e), &plain, (0, plain.len())),
    }
}

fn link(line: &str, (s, e): (usize, usize), image: bool) -> Edit {
    let inner = &line[s..e];
    let bang = if image { "!" } else { "" };
    if inner.starts_with("http://") || inner.starts_with("https://") {
        // A URL: it becomes the target, the cursor goes to the text.
        let open = bang.len() + 1;
        return replace(line, (s, e), &format!("{bang}[]({inner})"), (open, open));
    }
    let url_start = bang.len() + 1 + inner.len() + 2;
    replace(
        line,
        (s, e),
        &format!("{bang}[{inner}]({URL_PLACEHOLDER})"),
        (url_start, url_start + URL_PLACEHOLDER.len()),
    )
}

// ----- block types --------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq)]
enum List {
    Bullet,
    Numbered,
    Task,
}

/// List marker at the start of `rest` (after the indentation) and its length.
fn list_marker(rest: &str) -> Option<(List, usize)> {
    let b = rest.as_bytes();
    if matches!(b.first(), Some(b'-' | b'*' | b'+')) && b.get(1) == Some(&b' ') {
        let task = matches!(rest.get(2..5), Some("[ ]" | "[x]" | "[X]")) && matches!(b.get(5), Some(b' ') | None);
        return Some(if task { (List::Task, rest.len().min(6)) } else { (List::Bullet, 2) });
    }
    let digits = b.iter().take_while(|c| c.is_ascii_digit()).count();
    if (1..=9).contains(&digits) && matches!(b.get(digits), Some(b'.' | b')')) && matches!(b.get(digits + 1), Some(b' ') | None) {
        return Some((List::Numbered, (digits + 2).min(rest.len())));
    }
    None
}

/// Heading level at the start of `line` and the length of its marker.
fn heading(line: &str) -> Option<(u8, usize)> {
    let hashes = line.bytes().take_while(|&b| b == b'#').count();
    let ok = (1..=6).contains(&hashes) && matches!(line.as_bytes().get(hashes), Some(b' ') | None);
    ok.then(|| (hashes as u8, (hashes + 1).min(line.len())))
}

fn split_indent(line: &str) -> (&str, &str) {
    let n = line.len() - line.trim_start_matches([' ', '\t']).len();
    line.split_at(n)
}

fn lines_op(text: &str, sel: (usize, usize), op: Op) -> Edit {
    let lines: Vec<&str> = text.split('\n').collect();
    let single = lines.len() == 1;
    // Empty lines are left alone, unless the command is for a single one.
    let applies = |l: &str| single || !l.trim().is_empty();
    let target = match op {
        Op::Bullet => Some(List::Bullet),
        Op::Numbered => Some(List::Numbered),
        Op::Task => Some(List::Task),
        _ => None,
    };
    let all_quoted = lines.iter().filter(|l| applies(l)).all(|l| l.trim_start().starts_with('>'));
    let all_listed = target.is_some_and(|t| {
        lines.iter().filter(|l| applies(l)).all(|l| list_marker(split_indent(l).1).is_some_and(|(k, _)| k == t))
    });

    let mut number = 0;
    let new: Vec<String> = lines
        .iter()
        .map(|&line| {
            if !applies(line) {
                return if op == Op::Quote && !all_quoted { ">".to_owned() } else { line.to_owned() };
            }
            match op {
                Op::Heading(level) => {
                    let (current, len) = heading(line).unwrap_or((0, 0));
                    let rest = &line[len..];
                    if level == 0 || level == current {
                        rest.to_owned()
                    } else {
                        format!("{} {rest}", "#".repeat(level as usize))
                    }
                }
                Op::Quote => {
                    if all_quoted {
                        let (indent, rest) = split_indent(line);
                        let rest = rest.strip_prefix('>').unwrap_or(rest);
                        format!("{indent}{}", rest.strip_prefix(' ').unwrap_or(rest))
                    } else {
                        format!("> {line}")
                    }
                }
                _ => {
                    let (indent, rest) = split_indent(line);
                    let rest = list_marker(rest).map_or(rest, |(_, len)| &rest[len..]);
                    if all_listed {
                        return format!("{indent}{rest}");
                    }
                    let marker = match target {
                        Some(List::Numbered) => {
                            number += 1;
                            format!("{number}. ")
                        }
                        Some(List::Task) => "- [ ] ".into(),
                        _ => "- ".into(),
                    };
                    format!("{indent}{marker}{rest}")
                }
            }
        })
        .collect();
    let out = new.join("\n");
    let sel = if single && sel.0 == sel.1 {
        // Keep the cursor on the same character, behind the new marker.
        let old_marker = text.len() - marker_free(text, op).len();
        let new_marker = out.len() - marker_free(&out, op).len();
        let pos = (sel.0.max(old_marker) - old_marker + new_marker).min(out.len());
        (pos, pos)
    } else {
        (0, out.len())
    };
    Edit { text: out, sel }
}

/// Text of a line after its marker for `op` (heading, quote or list marker).
fn marker_free(line: &str, op: Op) -> &str {
    match op {
        Op::Heading(_) => &line[heading(line).map_or(0, |(_, len)| len)..],
        Op::Quote => {
            let (_, rest) = split_indent(line);
            match rest.strip_prefix('>') {
                Some(r) => r.strip_prefix(' ').unwrap_or(r),
                None => line,
            }
        }
        _ => {
            let (_, rest) = split_indent(line);
            list_marker(rest).map_or(rest, |(_, len)| &rest[len..])
        }
    }
}

fn code_block(text: &str, sel: (usize, usize)) -> Edit {
    let lines: Vec<&str> = text.split('\n').collect();
    let fenced = lines.len() >= 2 && lines[0].trim_start().starts_with("```") && lines[lines.len() - 1].trim() == "```";
    if fenced {
        let inner = lines[1..lines.len() - 1].join("\n");
        let len = inner.len();
        return Edit { text: inner, sel: (0, len) };
    }
    let out = format!("```\n{text}\n```");
    let start = 4;
    let sel = if sel.0 == sel.1 { (start + sel.0, start + sel.0) } else { (start, start + text.len()) };
    Edit { text: out, sel }
}

/// Horizontal rule below the lines, set off by blank lines (directly under
/// text, `---` would turn it into a heading).
fn rule(text: &str) -> Edit {
    let out = if text.trim().is_empty() { "\n---\n".to_owned() } else { format!("{text}\n\n---\n") };
    let len = out.len();
    Edit { text: out, sel: (len, len) }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Applies `op` to `text`, where `«` and `»` mark the selection (`|`
    /// marks the cursor), and returns the result with the new selection marked.
    fn run(text: &str, op: Op) -> String {
        let (plain, sel) = parse(text);
        let edit = apply(&plain, sel, op);
        mark(&edit.text, edit.sel)
    }

    fn parse(text: &str) -> (String, (usize, usize)) {
        if let Some(p) = text.find('|') {
            let plain = text.replacen('|', "", 1);
            return (plain, (p, p));
        }
        let s = text.find('«').unwrap();
        let e = text.find('»').unwrap() - '«'.len_utf8();
        (text.replacen('«', "", 1).replacen('»', "", 1), (s, e))
    }

    fn mark(text: &str, (s, e): (usize, usize)) -> String {
        if s == e {
            format!("{}|{}", &text[..s], &text[s..])
        } else {
            format!("{}«{}»{}", &text[..s], &text[s..e], &text[e..])
        }
    }

    #[test]
    fn bold_wraps_and_unwraps() {
        assert_eq!(run("a «word» b", Op::Bold), "a **«word»** b");
        assert_eq!(run("a **«word»** b", Op::Bold), "a «word» b");
        assert_eq!(run("a «**word**» b", Op::Bold), "a «word» b");
        assert_eq!(run("a «word »b", Op::Bold), "a **«word»** b");
    }

    #[test]
    fn cursor_in_word_formats_the_word() {
        assert_eq!(run("a wo|rd b", Op::Italic), "a *«word»* b");
        assert_eq!(run("a | b", Op::Bold), "a **|** b");
    }

    #[test]
    fn italic_and_bold_share_stars() {
        assert_eq!(run("**«x»**", Op::Italic), "***«x»***");
        assert_eq!(run("***«x»***", Op::Italic), "**«x»**");
        assert_eq!(run("***«x»***", Op::Bold), "*«x»*");
        assert_eq!(run("*«x»*", Op::Bold), "***«x»***");
        assert_eq!(run("«*x*»", Op::Italic), "«x»");
    }

    #[test]
    fn strike_and_code() {
        assert_eq!(run("«x»", Op::Strike), "~~«x»~~");
        assert_eq!(run("~~«x»~~", Op::Strike), "«x»");
        assert_eq!(run("use «cargo»", Op::Code), "use `«cargo»`");
        assert_eq!(run("use `«cargo»`", Op::Code), "use «cargo»");
    }

    #[test]
    fn inline_style_per_line() {
        assert_eq!(run("«one\n\ntwo»", Op::Bold), "«**one**\n\n**two**»");
    }

    #[test]
    fn headings() {
        assert_eq!(run("Ti|tle", Op::Heading(2)), "## Ti|tle");
        assert_eq!(run("## Ti|tle", Op::Heading(2)), "Ti|tle");
        assert_eq!(run("## Ti|tle", Op::Heading(1)), "# Ti|tle");
        assert_eq!(run("### Ti|tle", Op::Heading(0)), "Ti|tle");
        assert_eq!(run("|", Op::Heading(1)), "# |");
    }

    #[test]
    fn quotes() {
        assert_eq!(run("«a\n\nb»", Op::Quote), "«> a\n>\n> b»");
        assert_eq!(run("«> a\n>\n> b»", Op::Quote), "«a\n\nb»");
    }

    #[test]
    fn lists() {
        assert_eq!(run("«a\nb»", Op::Bullet), "«- a\n- b»");
        assert_eq!(run("«- a\n- b»", Op::Bullet), "«a\nb»");
        assert_eq!(run("«- a\n  - b»", Op::Numbered), "«1. a\n  2. b»");
        assert_eq!(run("«1. a\n2. b»", Op::Task), "«- [ ] a\n- [ ] b»");
        assert_eq!(run("«- [x] a\n- [ ] b»", Op::Task), "«a\nb»");
        assert_eq!(run("it|em", Op::Bullet), "- it|em");
        assert_eq!(run("- it|em", Op::Task), "- [ ] it|em");
    }

    #[test]
    fn code_blocks() {
        assert_eq!(run("«a\nb»", Op::CodeBlock), "```\n«a\nb»\n```");
        assert_eq!(run("«```\na\n```»", Op::CodeBlock), "«a»");
        assert_eq!(run("|", Op::CodeBlock), "```\n|\n```");
    }

    #[test]
    fn links_and_images() {
        assert_eq!(run("see «docs»", Op::Link), "see [docs](«https://»)");
        assert_eq!(run("«https://x.org»", Op::Image), "![|](https://x.org)");
        assert_eq!(run("a | b", Op::Link), "a [|](https://) b");
    }

    #[test]
    fn rules() {
        assert_eq!(run("te|xt", Op::Rule), "text\n\n---\n|");
        assert_eq!(run("|", Op::Rule), "\n---\n|");
    }

    #[test]
    fn colors() {
        let red = "<span style=\"color: #e01b24\">";
        let blue = "<span style=\"color: #3584e4\">";
        assert_eq!(run("a «b» c", Op::Color(Some("#e01b24"))), format!("a {red}«b»</span> c"));
        // Recolour: selection is the content or the whole span.
        assert_eq!(run(&format!("{red}«b»</span>"), Op::Color(Some("#3584e4"))), format!("{blue}«b»</span>"));
        assert_eq!(run(&format!("«{red}b</span>»"), Op::Color(Some("#3584e4"))), format!("{blue}«b»</span>"));
        // Remove.
        assert_eq!(run(&format!("{red}«b»</span>"), Op::Color(None)), "«b»");
        assert_eq!(run(&format!("«x {red}b</span> y»"), Op::Color(None)), "«x b y»");
        assert_eq!(run("a | c", Op::Color(Some("#e01b24"))), format!("a {red}|</span> c"));
    }

    #[test]
    fn unicode_offsets() {
        assert_eq!(run("ä «ö» ü", Op::Bold), "ä **«ö»** ü");
        assert_eq!(run("grü|n", Op::Code), "`«grün»`");
    }
}
