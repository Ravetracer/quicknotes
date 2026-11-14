//! Completion for calculation sheets: unit names after a number, and
//! matching targets after `to` / `in` (units of the same kind, time zones
//! and calendars for dates).

use crate::calc::{self, Source};
use crate::slides_assist::Item;
use crate::units;

const CONVERT_WORDS: &[&str] = &["to", "in", "as", "nach"];
const MAX_ITEMS: usize = 200;

fn is_word(c: char) -> bool {
    c.is_alphanumeric() || matches!(c, '_' | '°' | '/' | '²' | '³' | 'µ')
}

/// The left side of a conversion when `head` ends with `to`, `in`, `->`, …
fn conversion_left(head: &str) -> Option<&str> {
    for arrow in ["->", "→"] {
        if let Some(left) = head.strip_suffix(arrow) {
            return Some(left).filter(|l| !l.trim().is_empty());
        }
    }
    let (left, word) = head.rsplit_once(char::is_whitespace)?;
    (CONVERT_WORDS.contains(&word) && !left.trim().is_empty()).then_some(left)
}

/// Proposals at the end of `line` (the cursor line up to the cursor);
/// `before` holds the lines above. Returns the byte offset where the
/// completed word starts.
pub fn proposals(before: &str, line: &str) -> Option<(usize, Vec<Item>)> {
    let trimmed = line.trim_start();
    if trimmed.starts_with('>') || line.contains('#') || line.contains("//") {
        return None;
    }
    let start = line.char_indices().rev().take_while(|(_, c)| is_word(*c)).last().map_or(line.len(), |(i, _)| i);
    let word = &line[start..];
    let head = line[..start].trim_end();
    let items = if let Some(left) = conversion_left(head).filter(|_| start > head.len()) {
        targets(calc::source_of(before, left))
    } else if !word.is_empty()
        && !word.starts_with(|c: char| c.is_ascii_digit())
        && head.ends_with(|c: char| c.is_ascii_digit())
        && start > head.len()
    {
        unit_items(|_| true)
    } else {
        return None;
    };
    let lower = word.to_lowercase();
    let mut matching: Vec<Item> = items
        .into_iter()
        .filter(|i| i.label.to_lowercase().starts_with(&lower) && i.label != word)
        .take(MAX_ITEMS)
        .collect();
    // Exact case first: "M" → MB before mb.
    matching.sort_by_key(|i| !i.label.starts_with(word));
    (!matching.is_empty()).then_some((start, matching))
}

fn item(label: &str, info: String) -> Item {
    Item { label: label.to_owned(), insert: label.to_owned(), info }
}

fn unit_items(keep: impl Fn(&units::Dim) -> bool) -> Vec<Item> {
    let mut items: Vec<Item> = units::entries()
        .iter()
        .filter(|e| e.common && keep(&e.dim()))
        .map(|e| item(&e.symbol, format!("{} · {}", e.name, e.category.label())))
        .collect();
    // Composed units of the converter (km/h, L/100km, m²).
    for (category, exprs) in units::DIALOG {
        for expr in *exprs {
            if items.iter().any(|i| i.label == *expr) {
                continue;
            }
            if let Some(Source::Quantity(dim)) = calc::source_of("", &format!("1 {expr}")) {
                if keep(&dim) {
                    items.push(item(expr, category.label().to_owned()));
                }
            }
        }
    }
    items
}

fn targets(source: Option<Source>) -> Vec<Item> {
    match source {
        Some(Source::Quantity(dim)) if dim == units::NONE => calc::SPECIAL_TARGETS
            .iter()
            .map(|(label, info)| item(label, crate::i18n::tr(info).to_owned()))
            .collect(),
        Some(Source::Quantity(dim)) => {
            let inverse = dim.map(|x| -x);
            let mut items = unit_items(|d| *d == dim || *d == inverse);
            // Several units at once: `to h min`, `to ft in`.
            for expr in ["h min", "d h min", "min s", "ft in"] {
                if let Some(Source::Quantity(d)) = calc::source_of("", &format!("1 {}", expr.split(' ').next().unwrap_or(""))) {
                    if d == dim {
                        items.push(item(expr, t!("several units").to_owned()));
                    }
                }
            }
            items
        }
        Some(Source::Date) => {
            let mut items: Vec<Item> = calc::DATE_TARGETS
                .iter()
                .map(|(label, info)| item(label, crate::i18n::tr(info).to_owned()))
                .collect();
            items.extend(["UTC", "local"].iter().map(|z| item(z, t!("time zone").to_owned())));
            items.extend(crate::calendar::city_names().into_iter().map(|c| item(c, t!("time zone").to_owned())));
            items
        }
        None => unit_items(|_| true),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn labels(before: &str, line: &str) -> Vec<String> {
        proposals(before, line).map(|(_, items)| items.into_iter().map(|i| i.label).collect()).unwrap_or_default()
    }

    #[test]
    fn units_after_numbers() {
        let l = labels("", "5 k");
        assert!(l.contains(&"km".to_owned()) && l.contains(&"kg".to_owned()), "{l:?}");
        assert!(labels("", "abc k").is_empty());
        assert_eq!(proposals("", "x = 12 km").map(|p| p.0), Some(7));
    }

    #[test]
    fn targets_of_the_same_kind() {
        let l = labels("a = 3 m", "a to ");
        assert!(l.contains(&"ft".to_owned()) && !l.contains(&"kg".to_owned()), "{l:?}");
        let l = labels("", "5 km/h to m");
        assert!(l.contains(&"mph".to_owned()) && l.contains(&"m/s".to_owned()), "{l:?}");
        let l = labels("", "6 L/100km to m");
        assert!(l.contains(&"mpg".to_owned()), "{l:?}");
        let l = labels("", "14:00 to Tok");
        assert_eq!(l, vec!["Tokyo".to_owned()]);
        let l = labels("", "today to ");
        assert!(l.contains(&"hebrew".to_owned()), "{l:?}");
        let l = labels("", "255 to ");
        assert!(l.contains(&"hex".to_owned()), "{l:?}");
    }
}
