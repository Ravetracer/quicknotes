//! Data for charts: the Markdown tables of a document and a tolerant parser
//! for their cells (numbers in English or German notation, with currency
//! signs, units, percent signs or thousands separators, and dates).

use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag, TagEnd};

/// A table of a Markdown document.
#[derive(Debug, Clone, PartialEq)]
pub struct Table {
    /// First and last line (0-based).
    pub first: usize,
    pub last: usize,
    pub header: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

/// A heading of a Markdown document.
#[derive(Debug, Clone, PartialEq)]
pub struct Heading {
    pub line: usize,
    pub level: u8,
    pub text: String,
}

/// Tables and headings of a Markdown document.
#[derive(Debug, Default)]
pub struct Outline {
    pub tables: Vec<Table>,
    pub headings: Vec<Heading>,
}

pub fn outline(markdown: &str) -> Outline {
    let starts: Vec<usize> = std::iter::once(0).chain(markdown.match_indices('\n').map(|(i, _)| i + 1)).collect();
    let line_of = |offset: usize| starts.partition_point(|&s| s <= offset).saturating_sub(1);
    let mut out = Outline::default();
    let mut table: Option<Table> = None;
    let mut row: Vec<String> = Vec::new();
    let mut cell: Option<String> = None;
    let mut heading: Option<(usize, u8, String)> = None;
    let opts = Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH | Options::ENABLE_MATH;
    for (event, range) in Parser::new_ext(markdown, opts).into_offset_iter() {
        match event {
            Event::Start(Tag::Table(_)) => {
                let last = line_of(range.end.saturating_sub(1).max(range.start));
                table = Some(Table { first: line_of(range.start), last, header: Vec::new(), rows: Vec::new() });
            }
            Event::End(TagEnd::Table) => out.tables.extend(table.take()),
            Event::Start(Tag::TableHead | Tag::TableRow) => row.clear(),
            Event::End(TagEnd::TableHead) => {
                if let Some(t) = table.as_mut() {
                    t.header = std::mem::take(&mut row);
                }
            }
            Event::End(TagEnd::TableRow) => {
                if let Some(t) = table.as_mut() {
                    t.rows.push(std::mem::take(&mut row));
                }
            }
            Event::Start(Tag::TableCell) => cell = Some(String::new()),
            Event::End(TagEnd::TableCell) => row.push(cell.take().unwrap_or_default().trim().to_owned()),
            Event::Start(Tag::Heading { level, .. }) => heading = Some((line_of(range.start), level_number(level), String::new())),
            Event::End(TagEnd::Heading(_)) => {
                if let Some((line, level, text)) = heading.take() {
                    out.headings.push(Heading { line, level, text: text.trim().to_owned() });
                }
            }
            Event::Text(t) | Event::Code(t) | Event::InlineMath(t) | Event::DisplayMath(t) => {
                if let Some(c) = cell.as_mut() {
                    c.push_str(&t);
                } else if let Some((_, _, h)) = heading.as_mut() {
                    h.push_str(&t);
                }
            }
            Event::SoftBreak | Event::HardBreak => {
                if let Some(c) = cell.as_mut() {
                    c.push(' ');
                }
            }
            _ => {}
        }
    }
    out
}

fn level_number(level: HeadingLevel) -> u8 {
    match level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}

// ----- cells ------------------------------------------------------------------

/// A parsed cell.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Null,
    Num(f64),
    /// ISO date (`YYYY-MM-DD`, optionally with ` HH:MM:SS`).
    Date(String),
    Text(String),
}

/// What a column holds (ignoring empty cells).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Empty,
    Number,
    Date,
    Text,
}

#[derive(Debug, Clone)]
pub struct Column {
    pub name: String,
    /// Cell texts as written.
    pub raw: Vec<String>,
    pub values: Vec<Value>,
    pub kind: Kind,
    /// Currency or unit shared by all numbers of the column (`€`, `%`, `kg`).
    pub unit: Option<String>,
}

impl Column {
    /// Parses the cells of a column. `decimal` is the decimal separator for
    /// numbers that could be read either way (`1.234`); `None` decides by
    /// the other cells of the column and falls back to `fallback`.
    pub fn parse(name: &str, raw: Vec<String>, decimal: Option<char>, fallback: char) -> Self {
        let decimal = decimal.unwrap_or_else(|| infer_decimal(&raw).unwrap_or(fallback));
        let numbers: Vec<Option<Number>> = raw.iter().map(|c| number(c, decimal)).collect();
        let nulls = raw.iter().filter(|c| is_null(c)).count();
        let filled = raw.len() - nulls;
        let count_numbers = numbers.iter().flatten().count();
        let dates: Vec<Option<String>> = raw.iter().map(|c| date(c)).collect();
        let count_dates = dates.iter().flatten().count();
        let kind = if filled == 0 {
            Kind::Empty
        } else if count_numbers == filled {
            Kind::Number
        } else if count_dates == filled {
            Kind::Date
        } else {
            Kind::Text
        };
        let values = raw
            .iter()
            .enumerate()
            .map(|(i, c)| match kind {
                _ if is_null(c) => Value::Null,
                Kind::Number => numbers[i].as_ref().map_or(Value::Null, |n| Value::Num(n.value)),
                Kind::Date => dates[i].clone().map_or(Value::Null, Value::Date),
                _ => Value::Text(c.clone()),
            })
            .collect();
        let unit = if kind == Kind::Number {
            let mut units = numbers.iter().flatten().map(|n| n.unit.clone());
            let first = units.next().flatten();
            first.filter(|u| units.all(|o| o.as_deref() == Some(u.as_str())))
        } else {
            None
        };
        Self { name: name.to_owned(), raw, values, kind, unit }
    }

    pub fn num(&self, row: usize) -> Option<f64> {
        match self.values.get(row) {
            Some(Value::Num(v)) => Some(*v),
            _ => None,
        }
    }

    /// The cell as text (for names and categories).
    pub fn label(&self, row: usize) -> String {
        self.raw.get(row).map(|s| s.trim().to_owned()).unwrap_or_default()
    }
}

/// Texts that stand for "no value".
pub fn is_null(cell: &str) -> bool {
    let c = cell.trim();
    c.is_empty()
        || matches!(c, "-" | "–" | "—" | "?" | "/" | "n/a" | "N/A" | "NA" | "n.a." | "null" | "NULL" | "nan" | "NaN" | "#N/A" | "k.A." | "k. A.")
}

/// A number read from a cell.
#[derive(Debug, Clone, PartialEq)]
pub struct Number {
    pub value: f64,
    /// Currency sign or unit before or after the number.
    pub unit: Option<String>,
}

/// The separators of a number's digits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Shape {
    /// Digits only.
    Plain,
    /// Clearly this decimal separator (`1,5`, `1.234,5`, `1,234,567`).
    Decimal(char),
    /// One separator before three digits (`1.234`): a decimal separator
    /// or a thousands separator.
    Ambiguous(char),
}

const CURRENCIES: &[char] = &['$', '€', '£', '¥', '₹', '₽', '₩', '¢', '₿', '₺', '₴', '₪', '฿'];
const MINUS: &[char] = &['-', '−', '–'];
const GROUPING: &[char] = &['\'', '’', '_'];
const SPACES: &[char] = &[' ', '\u{a0}', '\u{202f}', '\u{2009}'];

/// Splits a cell into prefix, digits (with separators and exponent) and
/// suffix. `None` if it holds no digits.
fn split_number(cell: &str) -> Option<(&str, &str, &str)> {
    let start = cell.find(|c: char| c.is_ascii_digit())?;
    // A leading separator belongs to the number (".5").
    let start = match cell[..start].chars().last() {
        Some(c @ ('.' | ',')) => start - c.len_utf8(),
        _ => start,
    };
    let chars: Vec<(usize, char)> = cell[start..].char_indices().map(|(i, c)| (i + start, c)).collect();
    let digit_at = |i: usize| chars.get(i).is_some_and(|(_, c)| c.is_ascii_digit());
    let mut i = 0;
    let mut end = start;
    while i < chars.len() {
        let c = chars[i].1;
        let separator = (c == '.' || c == ',' || GROUPING.contains(&c)) && digit_at(i + 1);
        let space = SPACES.contains(&c) && i > 0 && digit_at(i - 1) && digit_at(i + 1);
        let take = if c.is_ascii_digit() || separator || space {
            1
        } else if (c == 'e' || c == 'E') && i > 0 && digit_at(i - 1) {
            // Exponent: e5, e-5, e+5; ends the number.
            let sign = matches!(chars.get(i + 1), Some((_, '+' | '-')));
            let first = i + 1 + usize::from(sign);
            if !digit_at(first) {
                break;
            }
            let mut j = first;
            while digit_at(j) {
                j += 1;
            }
            end = chars.get(j).map_or(cell.len(), |(o, _)| *o);
            return Some((&cell[..start], &cell[start..end], &cell[end..]));
        } else {
            0
        };
        if take == 0 {
            break;
        }
        i += 1;
        end = chars.get(i).map_or(cell.len(), |(o, _)| *o);
    }
    Some((&cell[..start], &cell[start..end], &cell[end..]))
}

fn shape(digits: &str) -> Option<Shape> {
    let dots = digits.matches('.').count();
    let commas = digits.matches(',').count();
    if dots == 0 && commas == 0 {
        return Some(Shape::Plain);
    }
    if dots > 0 && commas > 0 {
        let last = if digits.rfind('.') > digits.rfind(',') { '.' } else { ',' };
        let other = if last == '.' { ',' } else { '.' };
        let (int, _) = digits.rsplit_once(last)?;
        return (digits.matches(last).count() == 1 && grouped(int, other)).then_some(Shape::Decimal(last));
    }
    let sep = if dots > 0 { '.' } else { ',' };
    let other = if sep == '.' { ',' } else { '.' };
    if dots + commas > 1 {
        return grouped(digits, sep).then_some(Shape::Decimal(other));
    }
    let (int, frac) = digits.split_once(sep)?;
    if frac.len() != 3 || int.is_empty() || int == "0" || int.len() > 3 {
        Some(Shape::Decimal(sep))
    } else {
        Some(Shape::Ambiguous(sep))
    }
}

/// Whether `digits` are groups of three digits separated by `sep` (the
/// first group 1–3 digits).
fn grouped(digits: &str, sep: char) -> bool {
    let mut groups = digits.split(sep);
    let first = groups.next().unwrap_or("");
    (1..=3).contains(&first.len()) && groups.all(|g| g.len() == 3)
}

/// Reads a number from a cell; `decimal` resolves ambiguous separators.
pub fn number(cell: &str, decimal: char) -> Option<Number> {
    let cell = cell.trim();
    if cell.is_empty() {
        return None;
    }
    // Accounting notation: (123) is negative.
    let (cell, mut negative) = match cell.strip_prefix('(').and_then(|c| c.strip_suffix(')')) {
        Some(inner) => (inner.trim(), true),
        None => (cell, false),
    };
    let (prefix, digits, suffix) = split_number(cell)?;
    let (mantissa, exponent) = match digits.find(['e', 'E']) {
        Some(i) => (&digits[..i], digits[i + 1..].parse::<i32>().ok()?),
        None => (digits, 0),
    };
    let mantissa: String = mantissa.chars().filter(|c| !GROUPING.contains(c) && !SPACES.contains(c)).collect();
    let decimal = match shape(&mantissa)? {
        Shape::Plain => '.',
        Shape::Decimal(c) => c,
        Shape::Ambiguous(c) => {
            if c == decimal {
                c
            } else if c == '.' {
                ','
            } else {
                '.'
            }
        }
    };
    let plain: String = mantissa
        .chars()
        .filter_map(|c| match c {
            '.' | ',' if c == decimal => Some('.'),
            '.' | ',' => None,
            c => Some(c),
        })
        .collect();
    let mut value: f64 = plain.parse().ok()?;
    if exponent != 0 {
        value *= 10f64.powi(exponent);
    }
    let (prefix_unit, prefix_minus) = prefix_unit(prefix)?;
    negative ^= prefix_minus;
    let suffix_unit = suffix_unit(suffix)?;
    if negative {
        value = -value;
    }
    let unit = prefix_unit.or(suffix_unit);
    value.is_finite().then_some(Number { value, unit })
}

/// Currency or currency code before a number, and whether there is a minus
/// sign. `None` if the prefix is no part of a number ("Q1", "v2").
fn prefix_unit(prefix: &str) -> Option<(Option<String>, bool)> {
    let mut minus = false;
    let mut unit = String::new();
    let mut letters = String::new();
    for c in prefix.chars() {
        if MINUS.contains(&c) {
            minus = true;
        } else if c == '+' || c == '~' || c == '≈' || SPACES.contains(&c) {
            if !letters.is_empty() {
                unit = std::mem::take(&mut letters);
            }
        } else if CURRENCIES.contains(&c) {
            unit.push(c);
        } else if c.is_alphabetic() {
            letters.push(c);
        } else {
            return None;
        }
    }
    // Letters must be set off from the digits ("EUR 5", not "Q1").
    if !letters.is_empty() || unit.chars().count() > 4 {
        return None;
    }
    Some(((!unit.is_empty()).then_some(unit), minus))
}

/// Unit after a number (`%`, `€`, `kg`, `km/h`, `°C`). `None` if the suffix
/// is no unit (more digits, as in dates or versions).
fn suffix_unit(suffix: &str) -> Option<Option<String>> {
    let unit = suffix.trim().trim_end_matches('*').trim();
    if unit.is_empty() {
        return Some(None);
    }
    let ok = unit.chars().count() <= 12
        && unit.chars().all(|c| {
            c.is_alphabetic() || CURRENCIES.contains(&c) || "%‰°/.²³µ'\"".contains(c) || SPACES.contains(&c)
        });
    ok.then(|| Some(unit.to_owned()))
}

/// The decimal separator the unambiguous numbers of a column use.
pub fn infer_decimal(cells: &[String]) -> Option<char> {
    let (mut point, mut comma) = (0, 0);
    for cell in cells {
        let Some((_, digits, _)) = split_number(cell.trim()) else { continue };
        let mantissa: String = digits
            .split(['e', 'E'])
            .next()
            .unwrap_or("")
            .chars()
            .filter(|c| !GROUPING.contains(c) && !SPACES.contains(c))
            .collect();
        match shape(&mantissa) {
            Some(Shape::Decimal('.')) => point += 1,
            Some(Shape::Decimal(',')) => comma += 1,
            _ => {}
        }
    }
    match point.cmp(&comma) {
        std::cmp::Ordering::Greater => Some('.'),
        std::cmp::Ordering::Less => Some(','),
        std::cmp::Ordering::Equal => None,
    }
}

/// Reads a date: `2026-03-01`, `2026/03/01`, `01.03.2026`, `2026-03`,
/// optionally with a time (`14:30`, `14:30:15`). Returns ISO text.
pub fn date(cell: &str) -> Option<String> {
    let cell = cell.trim();
    let (day, time) = match cell.split_once(['T', ' ']) {
        Some((d, t)) => (d, Some(t.trim())),
        None => (cell, None),
    };
    let num = |s: &str, len: std::ops::RangeInclusive<usize>| -> Option<u32> {
        (len.contains(&s.len()) && s.bytes().all(|b| b.is_ascii_digit())).then(|| s.parse().ok()).flatten()
    };
    let parts: Vec<&str> = day.split(['-', '/', '.']).collect();
    let sep = day.chars().find(|c| matches!(c, '-' | '/' | '.'))?;
    let (y, m, d) = match (sep, parts.as_slice()) {
        ('-' | '/', [y, m, d]) => (num(y, 4..=4)?, num(m, 1..=2)?, num(d, 1..=2)?),
        ('-', [y, m]) if time.is_none() => (num(y, 4..=4)?, num(m, 1..=2)?, 1),
        ('.', [d, m, y]) => (num(y, 4..=4)?, num(m, 1..=2)?, num(d, 1..=2)?),
        _ => return None,
    };
    if !(1..=12).contains(&m) || !(1..=31).contains(&d) {
        return None;
    }
    let mut out = format!("{y:04}-{m:02}-{d:02}");
    if let Some(time) = time {
        let time = time.trim_end_matches('Z');
        let t: Vec<&str> = time.split(':').collect();
        let (h, min, s) = match t.as_slice() {
            [h, m] => (num(h, 1..=2)?, num(m, 2..=2)?, 0),
            [h, m, s] => (num(h, 1..=2)?, num(m, 2..=2)?, num(s.split('.').next()?, 2..=2)?),
            _ => return None,
        };
        if h > 23 || min > 59 || s > 59 {
            return None;
        }
        out.push_str(&format!(" {h:02}:{min:02}:{s:02}"));
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn n(cell: &str) -> Option<f64> {
        number(cell, '.').map(|n| n.value)
    }

    fn de(cell: &str) -> Option<f64> {
        number(cell, ',').map(|n| n.value)
    }

    #[test]
    fn plain_numbers() {
        assert_eq!(n("42"), Some(42.0));
        assert_eq!(n("-3.5"), Some(-3.5));
        assert_eq!(n("+7"), Some(7.0));
        assert_eq!(n("−2"), Some(-2.0));
        assert_eq!(n(".5"), Some(0.5));
        assert_eq!(n("1.5e3"), Some(1500.0));
        assert_eq!(n("2E-2"), Some(0.02));
        assert_eq!(n("(120)"), Some(-120.0));
    }

    #[test]
    fn separators() {
        assert_eq!(n("1,234.56"), Some(1234.56));
        assert_eq!(n("1.234,56"), Some(1234.56));
        assert_eq!(n("1.234.567"), Some(1234567.0));
        assert_eq!(n("1,234,567"), Some(1234567.0));
        assert_eq!(n("1,5"), Some(1.5));
        assert_eq!(n("0,125"), Some(0.125));
        assert_eq!(n("1234,567"), Some(1234.567));
        assert_eq!(n("12 345"), Some(12345.0));
        assert_eq!(n("1'234.5"), Some(1234.5));
        assert_eq!(n("1_000"), Some(1000.0));
        // Ambiguous: decided by the hint.
        assert_eq!(n("1.234"), Some(1.234));
        assert_eq!(de("1.234"), Some(1234.0));
        assert_eq!(n("1,234"), Some(1234.0));
        assert_eq!(de("1,234"), Some(1.234));
        // Not numbers.
        assert_eq!(n("1.2.3"), None);
        assert_eq!(n("1,23,4"), None);
    }

    #[test]
    fn units_and_currencies() {
        let u = |c: &str| number(c, '.').and_then(|n| n.unit);
        assert_eq!(n("12 %"), Some(12.0));
        assert_eq!(u("12%").as_deref(), Some("%"));
        assert_eq!(n("€ 1.200,50"), Some(1200.5));
        assert_eq!(u("$5").as_deref(), Some("$"));
        assert_eq!(n("-€5"), Some(-5.0));
        assert_eq!(n("€-5"), Some(-5.0));
        assert_eq!(n("EUR 12"), Some(12.0));
        assert_eq!(u("EUR 12").as_deref(), Some("EUR"));
        assert_eq!(n("12 kg"), Some(12.0));
        assert_eq!(u("80 km/h").as_deref(), Some("km/h"));
        assert_eq!(n("21,5 °C"), Some(21.5));
        assert_eq!(n("5*"), Some(5.0));
        assert_eq!(n("Q1"), None);
        assert_eq!(n("v2"), None);
        assert_eq!(n("2026-01-05"), None);
        assert_eq!(n("12:30"), None);
        assert_eq!(n("abc"), None);
        assert_eq!(n(""), None);
    }

    #[test]
    fn dates() {
        assert_eq!(date("2026-03-01").as_deref(), Some("2026-03-01"));
        assert_eq!(date("2026/3/1").as_deref(), Some("2026-03-01"));
        assert_eq!(date("01.03.2026").as_deref(), Some("2026-03-01"));
        assert_eq!(date("2026-03").as_deref(), Some("2026-03-01"));
        assert_eq!(date("2026-03-01 14:30").as_deref(), Some("2026-03-01 14:30:00"));
        assert_eq!(date("2026-03-01T14:30:15Z").as_deref(), Some("2026-03-01 14:30:15"));
        assert_eq!(date("2026-13-01"), None);
        assert_eq!(date("1.5"), None);
        assert_eq!(date("2026"), None);
        assert_eq!(date("Jan"), None);
    }

    #[test]
    fn columns() {
        let col = |cells: &[&str]| Column::parse("c", cells.iter().map(|s| s.to_string()).collect(), None, '.');
        let c = col(&["1.234", "2.500", "1,5"]);
        assert_eq!(c.kind, Kind::Number);
        assert_eq!(c.num(0), Some(1234.0));
        assert_eq!(c.num(2), Some(1.5));
        let c = col(&["10 %", "", "-", "20 %"]);
        assert_eq!((c.kind, c.unit.as_deref()), (Kind::Number, Some("%")));
        assert_eq!(c.values[1], Value::Null);
        let c = col(&["2026-01-01", "02.01.2026"]);
        assert_eq!(c.kind, Kind::Date);
        assert_eq!(col(&["Q1", "Q2"]).kind, Kind::Text);
        assert_eq!(col(&["", "n/a"]).kind, Kind::Empty);
        assert_eq!(col(&["5 €", "3 $"]).unit, None);
    }

    #[test]
    fn finds_tables_and_headings() {
        let md = "# Sales\n\n| Month | **Revenue** |\n|---|--:|\n| Jan | `1,5` |\n| Feb | 2 |\n\n## Costs\n\n| a | b \\| c |\n|---|---|\n| 1 |\n";
        let o = outline(md);
        assert_eq!(o.tables.len(), 2);
        assert_eq!(o.tables[0].header, ["Month", "Revenue"]);
        assert_eq!(o.tables[0].rows, [["Jan", "1,5"], ["Feb", "2"]]);
        assert_eq!((o.tables[0].first, o.tables[0].last), (2, 5));
        assert_eq!(o.tables[1].header, ["a", "b | c"]);
        assert_eq!(o.tables[1].first, 9);
        assert_eq!(o.headings.iter().map(|h| (h.line, h.level, h.text.as_str())).collect::<Vec<_>>(), [(0, 1, "Sales"), (7, 2, "Costs")]);
    }
}
