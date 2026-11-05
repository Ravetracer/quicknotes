//! Geocaching puzzle tools: classic ciphers, encodings and letter arithmetic.
//!
//! Every tool is a pure function from the selected text (plus options) to a
//! result text. The window shows an options dialog for tools that have fields
//! and inserts the result below the selection.

use std::collections::BTreeMap;

/// Option values of one tool, keyed by field key. Missing or invalid values
/// fall back to the field default.
pub type Options = BTreeMap<String, String>;

/// A configurable option of a tool.
pub enum Field {
    /// One of several values; the first one is the default.
    Choice { key: &'static str, label: &'static str, options: &'static [(&'static str, &'static str)] },
    Toggle { key: &'static str, label: &'static str, default: bool },
    Number { key: &'static str, label: &'static str, min: i32, max: i32, default: i32 },
}

impl Field {
    pub fn key(&self) -> &'static str {
        match self {
            Field::Choice { key, .. } | Field::Toggle { key, .. } | Field::Number { key, .. } => key,
        }
    }

    /// Label and option labels (for translation checks).
    #[cfg(test)]
    pub fn labels(&self) -> Vec<&'static str> {
        match self {
            Field::Choice { label, options, .. } => std::iter::once(*label).chain(options.iter().map(|o| o.1)).collect(),
            Field::Toggle { label, .. } | Field::Number { label, .. } => vec![label],
        }
    }

    fn default_value(&self) -> String {
        match self {
            Field::Choice { options, .. } => options[0].0.to_owned(),
            Field::Toggle { default, .. } => default.to_string(),
            Field::Number { default, .. } => default.to_string(),
        }
    }

    /// The stored value if it is valid for this field, else the default.
    pub fn value(&self, values: &Options) -> String {
        values
            .get(self.key())
            .filter(|v| match self {
                Field::Choice { options, .. } => options.iter().any(|(id, _)| id == v),
                Field::Toggle { .. } => *v == "true" || *v == "false",
                Field::Number { min, max, .. } => v.parse::<i32>().is_ok_and(|n| (*min..=*max).contains(&n)),
            })
            .cloned()
            .unwrap_or_else(|| self.default_value())
    }
}

pub struct Tool {
    pub id: &'static str,
    pub label: &'static str,
    pub fields: &'static [Field],
    run: fn(&str, &Opts) -> Result<String, String>,
}

impl Tool {
    pub fn run(&self, input: &str, values: &Options) -> Result<String, String> {
        (self.run)(input, &Opts { fields: self.fields, values })
    }
}

struct Opts<'a> {
    fields: &'a [Field],
    values: &'a Options,
}

impl Opts<'_> {
    fn get(&self, key: &str) -> String {
        self.fields
            .iter()
            .find(|f| f.key() == key)
            .map(|f| f.value(self.values))
            .unwrap_or_default()
    }

    fn flag(&self, key: &str) -> bool {
        self.get(key) == "true"
    }

    fn number(&self, key: &str) -> i32 {
        self.get(key).parse().unwrap_or(0)
    }
}

pub fn find(id: &str) -> Option<&'static Tool> {
    TOOLS.iter().find(|t| t.id == id)
}

/// Menu layout: sections of tool ids.
pub const MENU: &[&[&str]] = &[
    &["rot13", "rot5", "rot18", "rot47"],
    &["caesar", "letter-value", "letter-frequency", "scrabble", "prime-alphabet"],
    &["hex", "ascii", "binary", "roman", "morse", "phone"],
];

const LETTER_DIRECTION: &[(&str, &str)] = &[("encode", "Letters → numbers"), ("decode", "Numbers → letters")];

pub static TOOLS: &[Tool] = &[
    Tool { id: "rot13", label: "ROT13", fields: &[], run: |s, _| Ok(rotate_letters(s, 13)) },
    Tool { id: "rot5", label: "ROT5 (digits)", fields: &[], run: |s, _| Ok(rotate_digits(s, 5)) },
    Tool { id: "rot18", label: "ROT18 (letters + digits)", fields: &[], run: |s, _| Ok(rotate_digits(&rotate_letters(s, 13), 5)) },
    Tool { id: "rot47", label: "ROT47", fields: &[], run: |s, _| Ok(rot47(s)) },
    Tool {
        id: "caesar",
        label: "Caesar Rotation",
        fields: &[
            Field::Number { key: "shift", label: "Shift", min: -25, max: 25, default: 13 },
            Field::Toggle { key: "all", label: "Show all 25 rotations", default: false },
        ],
        run: caesar_tool,
    },
    Tool {
        id: "letter-value",
        label: "Letter Value",
        fields: &[
            Field::Choice { key: "direction", label: "Direction", options: LETTER_DIRECTION },
            Field::Choice { key: "order", label: "Alphabet", options: &[("az", "A=1 … Z=26"), ("za", "Z=1 … A=26 (reverse)")] },
            Field::Choice {
                key: "umlauts",
                label: "Umlauts and ß",
                options: &[("expand", "As AE, OE, UE, SS"), ("extend", "Ä=27, Ö=28, Ü=29, ß=30"), ("skip", "Ignore")],
            },
            Field::Toggle { key: "values", label: "Value of each letter", default: true },
            Field::Toggle { key: "word_sums", label: "Sum of each word", default: true },
            Field::Toggle { key: "sum", label: "Sum of all letters", default: true },
            Field::Toggle { key: "letter_checksums", label: "Checksum of each letter value", default: false },
            Field::Toggle { key: "checksum", label: "Checksum of the sum", default: true },
            Field::Toggle { key: "digital_root", label: "Single-digit checksum of the sum", default: true },
        ],
        run: letter_value_tool,
    },
    Tool {
        id: "letter-frequency",
        label: "Letter Frequency",
        fields: &[
            Field::Choice { key: "sort", label: "Order", options: &[("count", "Most frequent first"), ("alpha", "Alphabetical")] },
            Field::Toggle { key: "digits", label: "Count digits too", default: false },
            Field::Toggle { key: "missing", label: "List missing letters A–Z", default: false },
        ],
        run: frequency_tool,
    },
    Tool {
        id: "scrabble",
        label: "Scrabble Value",
        fields: &[Field::Choice { key: "lang", label: "Letter set", options: &[("de", "German"), ("en", "English")] }],
        run: scrabble_tool,
    },
    Tool {
        id: "prime-alphabet",
        label: "Prime Alphabet",
        fields: &[
            Field::Choice { key: "direction", label: "Direction", options: LETTER_DIRECTION },
            Field::Toggle { key: "sum", label: "Sums", default: true },
        ],
        run: prime_tool,
    },
    Tool {
        id: "hex",
        label: "Hexadecimal",
        fields: &[Field::Choice {
            key: "mode",
            label: "Conversion",
            options: &[
                ("hex-to-text", "Hex → text"),
                ("hex-to-dec", "Hex → decimal"),
                ("text-to-hex", "Text → hex"),
                ("dec-to-hex", "Decimal → hex"),
            ],
        }],
        run: hex_tool,
    },
    Tool {
        id: "ascii",
        label: "ASCII Codes",
        fields: &[Field::Choice {
            key: "mode",
            label: "Conversion",
            options: &[("codes-to-text", "Codes → text"), ("text-to-codes", "Text → codes")],
        }],
        run: ascii_tool,
    },
    Tool {
        id: "binary",
        label: "Binary",
        fields: &[
            Field::Choice {
                key: "mode",
                label: "Conversion",
                options: &[
                    ("bin-to-dec", "Binary → decimal"),
                    ("bin-to-text", "Binary → text"),
                    ("dec-to-bin", "Decimal → binary"),
                    ("text-to-bin", "Text → binary"),
                ],
            },
            Field::Toggle { key: "pad", label: "Pad numbers to full bytes", default: false },
        ],
        run: binary_tool,
    },
    Tool {
        id: "roman",
        label: "Roman Numerals",
        fields: &[
            Field::Choice {
                key: "direction",
                label: "Direction",
                options: &[("auto", "Detect"), ("to-arabic", "Roman → Arabic"), ("to-roman", "Arabic → Roman")],
            },
            Field::Choice {
                key: "large",
                label: "Thousands from 4000 on",
                options: &[("overline", "Overline (X̄ = 10000)"), ("brackets", "Brackets ((X) = 10000)")],
            },
        ],
        run: roman_tool,
    },
    Tool {
        id: "morse",
        label: "Morse Code",
        fields: &[
            Field::Choice {
                key: "direction",
                label: "Direction",
                options: &[("auto", "Detect"), ("encode", "Text → Morse"), ("decode", "Morse → text")],
            },
            Field::Choice { key: "dash", label: "Dash symbol", options: &[("-", "Minus  -"), ("_", "Underscore  _")] },
            Field::Choice {
                key: "word_sep",
                label: "Word separator",
                options: &[("slash", "Slash  /"), ("pipe", "Bar  |"), ("space", "Three spaces")],
            },
        ],
        run: morse_tool,
    },
    Tool {
        id: "phone",
        label: "Phone Keypad",
        fields: &[
            Field::Choice { key: "direction", label: "Direction", options: &[("encode", "Text → keys"), ("decode", "Keys → text")] },
            Field::Choice {
                key: "style",
                label: "Notation",
                options: &[
                    ("multitap", "Multi-tap (GEO = 4 33 666)"),
                    ("position", "Key + position (GEO = 41 32 63)"),
                    ("key", "Key only / T9 (GEO = 436)"),
                ],
            },
        ],
        run: phone_tool,
    },
];

// ===== helpers ============================================================

#[derive(Clone, Copy, PartialEq, Eq)]
enum Umlauts {
    /// Ä → AE, Ö → OE, Ü → UE, ß → SS
    Expand,
    /// Kept as the letters Ä, Ö, Ü, ß
    Keep,
    Skip,
}

/// Upper-case letters of each whitespace-separated word (other characters
/// are dropped, words without letters are left out).
fn words(text: &str, umlauts: Umlauts) -> Vec<Vec<char>> {
    text.split_whitespace()
        .map(|w| {
            let mut out = Vec::new();
            for c in w.chars() {
                let c = match c {
                    'ß' | 'ẞ' => 'ß',
                    c => c.to_uppercase().next().unwrap_or(c),
                };
                match (c, umlauts) {
                    ('A'..='Z', _) => out.push(c),
                    ('Ä' | 'Ö' | 'Ü' | 'ß', Umlauts::Keep) => out.push(c),
                    ('Ä' | 'Ö' | 'Ü' | 'ß', Umlauts::Expand) => out.extend(match c {
                        'Ä' => ['A', 'E'],
                        'Ö' => ['O', 'E'],
                        'Ü' => ['U', 'E'],
                        _ => ['S', 'S'],
                    }),
                    _ => {}
                }
            }
            out
        })
        .filter(|w| !w.is_empty())
        .collect()
}

fn join<T: ToString>(items: impl IntoIterator<Item = T>, sep: &str) -> String {
    items.into_iter().map(|i| i.to_string()).collect::<Vec<_>>().join(sep)
}

/// Numbers per word: `7 5 15 | 3 1`.
fn join_words(words: &[Vec<u64>]) -> String {
    join(words.iter().map(|w| join(w, " ")), " | ")
}

fn digit_sum(mut n: u64) -> u64 {
    let mut sum = 0;
    while n > 0 {
        sum += n % 10;
        n /= 10;
    }
    sum
}

/// Repeated digit sum until a single digit is left (1432 → 10 → 1).
fn digital_root(mut n: u64) -> u64 {
    while n >= 10 {
        n = digit_sum(n);
    }
    n
}

/// "Values", "Word sums" (for several words) and "Sum" lines.
fn values_report(values: &[Vec<u64>], with_sums: bool) -> String {
    let mut lines = vec![t!("Values: {}", join_words(values))];
    if with_sums {
        if values.len() > 1 {
            lines.push(t!("Word sums: {}", join(values.iter().map(|w| w.iter().sum::<u64>()), " | ")));
        }
        lines.push(t!("Sum: {}", values.iter().flatten().sum::<u64>()));
    }
    lines.join("\n")
}

/// Replaces every number in the input by a letter; `|` and `/` separate
/// words. Unknown numbers become `?`. Lines are kept.
fn decode_numbers(input: &str, letter: impl Fn(u64) -> Option<char>) -> Result<String, String> {
    let mut found = false;
    let mut lines = Vec::new();
    for line in input.lines() {
        let mut out = String::new();
        let mut num = String::new();
        for c in line.chars().chain([' ']) {
            if c.is_ascii_digit() {
                num.push(c);
                continue;
            }
            if !num.is_empty() {
                out.push(num.parse().ok().and_then(&letter).unwrap_or('?'));
                num.clear();
                found = true;
            }
            if matches!(c, '|' | '/') && !out.is_empty() && !out.ends_with(' ') {
                out.push(' ');
            }
        }
        lines.push(out.trim_end().to_owned());
    }
    if !found {
        return Err(t!("No numbers in the selection").into());
    }
    Ok(lines.join("\n"))
}

/// Tokens separated by whitespace, commas or semicolons.
fn tokens(input: &str) -> Vec<&str> {
    input
        .split(|c: char| c.is_whitespace() || matches!(c, ',' | ';'))
        .filter(|t| !t.is_empty())
        .collect()
}

fn parse_radix(token: &str, radix: u32) -> Result<u128, String> {
    u128::from_str_radix(token, radix).map_err(|_| match radix {
        2 => t!("\"{}\" is not a valid binary number", token),
        16 => t!("\"{}\" is not a valid hex number", token),
        _ => t!("\"{}\" is not a valid decimal number", token),
    })
}

fn chunked(token: &str, size: usize) -> Vec<&str> {
    (0..token.len()).step_by(size).map(|i| &token[i..(i + size).min(token.len())]).collect()
}

/// Character codes to text: byte values are read as UTF-8 (falling back to
/// Latin-1), larger values as Unicode code points.
fn codes_to_text(codes: &[u128]) -> Result<String, String> {
    if codes.iter().all(|&c| c <= 0xFF) {
        let bytes: Vec<u8> = codes.iter().map(|&c| c as u8).collect();
        return Ok(match String::from_utf8(bytes) {
            Ok(s) => s,
            Err(e) => e.into_bytes().iter().map(|&b| b as char).collect(),
        });
    }
    codes
        .iter()
        .map(|&c| {
            u32::try_from(c)
                .ok()
                .and_then(char::from_u32)
                .ok_or_else(|| t!("{} is not a valid character code", c))
        })
        .collect()
}

// ===== rotations ==========================================================

fn rotate(c: char, base: char, n: i32, shift: i32) -> char {
    let i = c as i32 - base as i32;
    char::from_u32((base as i32 + (i + shift).rem_euclid(n)) as u32).unwrap_or(c)
}

pub fn rotate_letters(text: &str, shift: i32) -> String {
    text.chars()
        .map(|c| match c {
            'a'..='z' => rotate(c, 'a', 26, shift),
            'A'..='Z' => rotate(c, 'A', 26, shift),
            _ => c,
        })
        .collect()
}

pub fn rotate_digits(text: &str, shift: i32) -> String {
    text.chars().map(|c| if c.is_ascii_digit() { rotate(c, '0', 10, shift) } else { c }).collect()
}

pub fn rot47(text: &str) -> String {
    text.chars().map(|c| if ('!'..='~').contains(&c) { rotate(c, '!', 94, 47) } else { c }).collect()
}

fn caesar_tool(input: &str, o: &Opts) -> Result<String, String> {
    if o.flag("all") {
        return Ok(join((1..=25).map(|n| format!("ROT{n:<2} {}", rotate_letters(input, n))), "\n"));
    }
    Ok(rotate_letters(input, o.number("shift")))
}

// ===== letter values ======================================================

fn letter_value(c: char, reverse: bool) -> Option<u64> {
    match c {
        'A'..='Z' => {
            let v = c as u64 - 'A' as u64 + 1;
            Some(if reverse { 27 - v } else { v })
        }
        'Ä' => Some(27),
        'Ö' => Some(28),
        'Ü' => Some(29),
        'ß' => Some(30),
        _ => None,
    }
}

fn letter_for_value(n: u64, reverse: bool, umlauts: bool) -> Option<char> {
    match n {
        1..=26 => {
            let i = if reverse { 26 - n } else { n - 1 };
            char::from_u32('A' as u32 + i as u32)
        }
        27..=30 if umlauts => Some(['Ä', 'Ö', 'Ü', 'ß'][(n - 27) as usize]),
        _ => None,
    }
}

fn letter_value_tool(input: &str, o: &Opts) -> Result<String, String> {
    let reverse = o.get("order") == "za";
    let umlauts = match o.get("umlauts").as_str() {
        "extend" => Umlauts::Keep,
        "skip" => Umlauts::Skip,
        _ => Umlauts::Expand,
    };
    if o.get("direction") == "decode" {
        return decode_numbers(input, |n| letter_for_value(n, reverse, umlauts == Umlauts::Keep));
    }
    let values: Vec<Vec<u64>> = words(input, umlauts)
        .iter()
        .map(|w| w.iter().filter_map(|&c| letter_value(c, reverse)).collect())
        .collect();
    if values.is_empty() {
        return Err(t!("No letters in the selection").into());
    }
    let sum: u64 = values.iter().flatten().sum();
    let mut lines = Vec::new();
    if o.flag("values") {
        lines.push(t!("Letter values: {}", join_words(&values)));
    }
    if o.flag("word_sums") && values.len() > 1 {
        lines.push(t!("Word values: {}", join(values.iter().map(|w| w.iter().sum::<u64>()), " | ")));
    }
    if o.flag("sum") {
        lines.push(t!("Sum of all letters: {}", sum));
    }
    if o.flag("letter_checksums") {
        let sums: Vec<Vec<u64>> = values.iter().map(|w| w.iter().map(|&v| digit_sum(v)).collect()).collect();
        lines.push(t!("Letter checksums: {}", join_words(&sums)));
        lines.push(t!("Sum of letter checksums: {}", sums.iter().flatten().sum::<u64>()));
    }
    if o.flag("checksum") {
        lines.push(t!("Checksum of the sum: {}", digit_sum(sum)));
    }
    if o.flag("digital_root") {
        lines.push(t!("Single-digit checksum: {}", digital_root(sum)));
    }
    if lines.is_empty() {
        return Err(t!("Switch on at least one output").into());
    }
    Ok(lines.join("\n"))
}

fn frequency_tool(input: &str, o: &Opts) -> Result<String, String> {
    let digits = o.flag("digits");
    let mut counts: BTreeMap<char, usize> = BTreeMap::new();
    for c in input.chars() {
        let c = if c == 'ß' { c } else { c.to_uppercase().next().unwrap_or(c) };
        if c.is_alphabetic() || (digits && c.is_ascii_digit()) {
            *counts.entry(c).or_default() += 1;
        }
    }
    if counts.is_empty() {
        return Err(t!("No letters in the selection").into());
    }
    let total: usize = counts.values().sum();
    let missing: String = ('A'..='Z').filter(|c| !counts.contains_key(c)).collect();
    let mut list: Vec<(char, usize)> = counts.into_iter().collect();
    if o.get("sort") == "count" {
        list.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    }
    let mut lines = vec![
        join(list.iter().map(|(c, n)| format!("{c}={n}")), " "),
        t!("Total: {}, different: {}", total, list.len()),
    ];
    if o.flag("missing") {
        lines.push(t!("Missing: {}", if missing.is_empty() { t!("none").to_owned() } else { join(missing.chars(), " ") }));
    }
    Ok(lines.join("\n"))
}

fn scrabble_value(c: char, german: bool) -> Option<u64> {
    let v = if german {
        match c {
            'E' | 'N' | 'S' | 'I' | 'R' | 'T' | 'U' | 'A' | 'D' => 1,
            'H' | 'G' | 'L' | 'O' => 2,
            'M' | 'B' | 'W' | 'Z' => 3,
            'C' | 'F' | 'K' | 'P' => 4,
            'Ä' | 'J' | 'Ü' | 'V' => 6,
            'Ö' | 'X' => 8,
            'Q' | 'Y' => 10,
            _ => return None,
        }
    } else {
        match c {
            'A' | 'E' | 'I' | 'O' | 'U' | 'L' | 'N' | 'S' | 'T' | 'R' => 1,
            'D' | 'G' => 2,
            'B' | 'C' | 'M' | 'P' => 3,
            'F' | 'H' | 'V' | 'W' | 'Y' => 4,
            'K' => 5,
            'J' | 'X' => 8,
            'Q' | 'Z' => 10,
            _ => return None,
        }
    };
    Some(v)
}

fn scrabble_tool(input: &str, o: &Opts) -> Result<String, String> {
    let german = o.get("lang") == "de";
    // German Scrabble has tiles for Ä, Ö, Ü but not for ß.
    let input = input.replace(['ß', 'ẞ'], "SS");
    let umlauts = if german { Umlauts::Keep } else { Umlauts::Expand };
    let values: Vec<Vec<u64>> = words(&input, umlauts)
        .iter()
        .map(|w| w.iter().filter_map(|&c| scrabble_value(c, german)).collect())
        .collect();
    if values.is_empty() {
        return Err(t!("No letters in the selection").into());
    }
    Ok(values_report(&values, true))
}

const PRIMES: [u64; 26] = [
    2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37, 41, 43, 47, 53, 59, 61, 67, 71, 73, 79, 83, 89, 97, 101,
];

fn prime_tool(input: &str, o: &Opts) -> Result<String, String> {
    if o.get("direction") == "decode" {
        return decode_numbers(input, |n| {
            PRIMES.iter().position(|&p| p == n).and_then(|i| char::from_u32('A' as u32 + i as u32))
        });
    }
    let values: Vec<Vec<u64>> = words(input, Umlauts::Expand)
        .iter()
        .map(|w| w.iter().map(|&c| PRIMES[(c as u8 - b'A') as usize]).collect())
        .collect();
    if values.is_empty() {
        return Err(t!("No letters in the selection").into());
    }
    Ok(values_report(&values, o.flag("sum")))
}

// ===== number systems =====================================================

fn hex_tool(input: &str, o: &Opts) -> Result<String, String> {
    let strip = |t: &'_ str| -> String {
        let t = t.strip_prefix("0x").or_else(|| t.strip_prefix("0X")).or_else(|| t.strip_prefix('#')).unwrap_or(t);
        t.to_owned()
    };
    match o.get("mode").as_str() {
        "text-to-hex" => Ok(join(input.bytes().map(|b| format!("{b:02X}")), " ")),
        "dec-to-hex" => {
            let nums = tokens(input).iter().map(|t| parse_radix(t, 10)).collect::<Result<Vec<_>, _>>()?;
            non_empty(join(nums.iter().map(|n| format!("{n:X}")), " "))
        }
        "hex-to-dec" => {
            let nums = tokens(input).iter().map(|t| parse_radix(&strip(t), 16)).collect::<Result<Vec<_>, _>>()?;
            non_empty(join(nums, " "))
        }
        _ => {
            let toks: Vec<String> = tokens(input).iter().map(|t| strip(t)).collect();
            // A single run of hex digits like "47454F" is read as byte pairs.
            let toks: Vec<&str> = match toks.as_slice() {
                [one] if one.len() > 2 && one.len().is_multiple_of(2) && one.is_ascii() => chunked(one, 2),
                _ => toks.iter().map(String::as_str).collect(),
            };
            let codes = toks.iter().map(|t| parse_radix(t, 16)).collect::<Result<Vec<_>, _>>()?;
            non_empty(codes_to_text(&codes)?)
        }
    }
}

fn non_empty(s: String) -> Result<String, String> {
    if s.is_empty() {
        Err(t!("Nothing to convert in the selection").into())
    } else {
        Ok(s)
    }
}

/// Number at `run[i..i + n]`, if there are enough digits.
fn digits_at(run: &str, i: usize, n: usize) -> Option<u32> {
    run.get(i..i + n).and_then(|s| s.parse().ok())
}

/// Splits a run of printable ASCII codes without separators ("717970").
fn split_ascii_run(run: &str) -> Result<Vec<u128>, String> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < run.len() {
        if let Some(v) = digits_at(run, i, 3).filter(|v| (100..=126).contains(v)) {
            out.push(v as u128);
            i += 3;
        } else if let Some(v) = digits_at(run, i, 2).filter(|v| (32..=99).contains(v)) {
            out.push(v as u128);
            i += 2;
        } else {
            return Err(t!("Can't split \"{}\" into ASCII codes – please separate them", run));
        }
    }
    Ok(out)
}

fn ascii_tool(input: &str, o: &Opts) -> Result<String, String> {
    if o.get("mode") == "text-to-codes" {
        return non_empty(join(input.chars().map(|c| c as u32), " "));
    }
    let runs: Vec<&str> = input.split(|c: char| !c.is_ascii_digit()).filter(|t| !t.is_empty()).collect();
    let codes = match runs.as_slice() {
        [] => return Err(t!("No numbers in the selection").into()),
        [one] if one.len() > 3 => split_ascii_run(one)?,
        _ => runs.iter().map(|t| parse_radix(t, 10)).collect::<Result<Vec<_>, _>>()?,
    };
    codes_to_text(&codes)
}

fn binary_tool(input: &str, o: &Opts) -> Result<String, String> {
    let pad = |s: String| {
        let width = s.len().div_ceil(8) * 8;
        format!("{s:0>width$}")
    };
    match o.get("mode").as_str() {
        "text-to-bin" => non_empty(join(input.bytes().map(|b| format!("{b:08b}")), " ")),
        "dec-to-bin" => {
            let nums = tokens(input).iter().map(|t| parse_radix(t, 10)).collect::<Result<Vec<_>, _>>()?;
            let pad_on = o.flag("pad");
            non_empty(join(nums.iter().map(|n| if pad_on { pad(format!("{n:b}")) } else { format!("{n:b}") }), " "))
        }
        "bin-to-text" => {
            let toks = tokens(input);
            let toks = if toks.len() == 1 && toks[0].len() > 8 && toks[0].len().is_multiple_of(8) {
                chunked(toks[0], 8)
            } else {
                toks
            };
            let codes = toks.iter().map(|t| parse_radix(t, 2)).collect::<Result<Vec<_>, _>>()?;
            non_empty(codes_to_text(&codes)?)
        }
        _ => {
            let nums = tokens(input).iter().map(|t| parse_radix(t, 2)).collect::<Result<Vec<_>, _>>()?;
            non_empty(join(nums, " "))
        }
    }
}

// ===== Roman numerals =====================================================

fn roman_value(c: char) -> Option<u64> {
    Some(match c.to_ascii_uppercase() {
        'I' => 1,
        'V' => 5,
        'X' => 10,
        'L' => 50,
        'C' => 100,
        'D' => 500,
        'M' => 1000,
        _ => return None,
    })
}

fn is_overline(c: char) -> bool {
    matches!(c, '\u{0305}' | '\u{0304}' | '\u{033F}')
}

/// Parses Roman numerals, also non-standard ones (IIII, IIX). Thousands are
/// written with an overline (X̄ = 10000) or in brackets ((X) = 10000).
pub fn parse_roman(token: &str) -> Option<u64> {
    let mut values = Vec::new();
    let mut depth = 0u32;
    let mut chars = token.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '(' => depth += 1,
            ')' => depth = depth.checked_sub(1)?,
            _ => {
                let mut v = roman_value(c)?.checked_mul(1000u64.checked_pow(depth)?)?;
                let mut bars = false;
                while chars.peek().is_some_and(|&n| is_overline(n)) {
                    chars.next();
                    bars = true;
                }
                if bars {
                    v = v.checked_mul(1000)?;
                }
                values.push(v);
            }
        }
    }
    if depth != 0 || values.is_empty() {
        return None;
    }
    // From the right: a value smaller than the largest one seen is subtracted.
    let (mut total, mut max) = (0i128, 0u64);
    for &v in values.iter().rev() {
        if v < max {
            total -= v as i128;
        } else {
            total += v as i128;
            max = v;
        }
    }
    u64::try_from(total).ok().filter(|&n| n > 0)
}

fn roman_basic(mut n: u64) -> String {
    const TABLE: [(u64, &str); 13] = [
        (1000, "M"),
        (900, "CM"),
        (500, "D"),
        (400, "CD"),
        (100, "C"),
        (90, "XC"),
        (50, "L"),
        (40, "XL"),
        (10, "X"),
        (9, "IX"),
        (5, "V"),
        (4, "IV"),
        (1, "I"),
    ];
    let mut out = String::new();
    for (v, s) in TABLE {
        while n >= v {
            out.push_str(s);
            n -= v;
        }
    }
    out
}

pub fn to_roman(n: u64, brackets: bool) -> Result<String, String> {
    match n {
        0 => Err(t!("0 has no Roman numeral").into()),
        1..=3999 => Ok(roman_basic(n)),
        4000..=3_999_999 => {
            let high = roman_basic(n / 1000);
            let high = if brackets {
                format!("({high})")
            } else {
                high.chars().flat_map(|c| [c, '\u{0305}']).collect()
            };
            Ok(high + &roman_basic(n % 1000))
        }
        _ => Err(t!("{} is too large for Roman numerals", n)),
    }
}

/// Calls `f` for every token (letters, digits, brackets, overlines) and
/// replaces it with the result; separators are kept.
fn map_tokens(input: &str, mut f: impl FnMut(&str) -> Result<Option<String>, String>) -> Result<String, String> {
    let mut out = String::new();
    let mut tok = String::new();
    let mut flush = |out: &mut String, tok: &mut String| -> Result<(), String> {
        if !tok.is_empty() {
            out.push_str(&f(tok.as_str())?.unwrap_or_else(|| tok.clone()));
            tok.clear();
        }
        Ok(())
    };
    for c in input.chars() {
        if c.is_alphanumeric() || matches!(c, '(' | ')') || is_overline(c) {
            tok.push(c);
        } else {
            flush(&mut out, &mut tok)?;
            out.push(c);
        }
    }
    flush(&mut out, &mut tok)?;
    Ok(out)
}

fn is_number(token: &str) -> bool {
    !token.is_empty() && token.chars().all(|c| c.is_ascii_digit())
}

fn roman_tool(input: &str, o: &Opts) -> Result<String, String> {
    let brackets = o.get("large") == "brackets";
    let to_arabic = match o.get("direction").as_str() {
        "to-arabic" => true,
        "to-roman" => false,
        // Detect: convert Roman numerals if there are any, else numbers.
        _ => {
            let mut any = false;
            map_tokens(input, |t| {
                any |= parse_roman(t).is_some();
                Ok(None)
            })?;
            any
        }
    };
    let mut converted = 0;
    let out = map_tokens(input, |t| {
        let r = if to_arabic {
            parse_roman(t).map(|n| n.to_string())
        } else if is_number(t) {
            Some(to_roman(t.parse().map_err(|_| t!("{} is too large for Roman numerals", t))?, brackets)?)
        } else {
            None
        };
        converted += r.is_some() as usize;
        Ok(r)
    })?;
    if converted == 0 {
        return Err(if to_arabic { t!("No Roman numerals in the selection") } else { t!("No numbers in the selection") }.to_owned());
    }
    Ok(out)
}

// ===== Morse code =========================================================

const MORSE: &[(char, &str)] = &[
    ('A', ".-"),
    ('B', "-..."),
    ('C', "-.-."),
    ('D', "-.."),
    ('E', "."),
    ('F', "..-."),
    ('G', "--."),
    ('H', "...."),
    ('I', ".."),
    ('J', ".---"),
    ('K', "-.-"),
    ('L', ".-.."),
    ('M', "--"),
    ('N', "-."),
    ('O', "---"),
    ('P', ".--."),
    ('Q', "--.-"),
    ('R', ".-."),
    ('S', "..."),
    ('T', "-"),
    ('U', "..-"),
    ('V', "...-"),
    ('W', ".--"),
    ('X', "-..-"),
    ('Y', "-.--"),
    ('Z', "--.."),
    ('0', "-----"),
    ('1', ".----"),
    ('2', "..---"),
    ('3', "...--"),
    ('4', "....-"),
    ('5', "....."),
    ('6', "-...."),
    ('7', "--..."),
    ('8', "---.."),
    ('9', "----."),
    ('Ä', ".-.-"),
    ('Ö', "---."),
    ('Ü', "..--"),
    ('ß', "...--.."),
    ('.', ".-.-.-"),
    (',', "--..--"),
    ('?', "..--.."),
    ('\'', ".----."),
    ('!', "-.-.--"),
    ('/', "-..-."),
    ('(', "-.--."),
    (')', "-.--.-"),
    ('&', ".-..."),
    (':', "---..."),
    (';', "-.-.-."),
    ('=', "-...-"),
    ('+', ".-.-."),
    ('-', "-....-"),
    ('_', "..--.-"),
    ('"', ".-..-."),
    ('$', "...-..-"),
    ('@', ".--.-."),
];

fn normalize_morse(c: char) -> char {
    match c {
        '·' | '•' | '∙' | '*' => '.',
        '_' | '−' | '–' | '—' => '-',
        '\t' => ' ',
        c => c,
    }
}

fn is_morse(input: &str) -> bool {
    let input = input.trim();
    !input.is_empty()
        && input.chars().map(normalize_morse).all(|c| matches!(c, '.' | '-' | '/' | '|') || c.is_whitespace())
}

fn morse_encode(input: &str, dash: &str, word_sep: &str) -> Result<String, String> {
    let lines: Vec<String> = input
        .lines()
        .map(|line| {
            let words = line.split_whitespace().map(|word| {
                let codes = word.chars().filter_map(|c| {
                    let c = if c == 'ß' { c } else { c.to_uppercase().next().unwrap_or(c) };
                    MORSE.iter().find(|(m, _)| *m == c).map(|(_, code)| code.replace('-', dash))
                });
                join(codes, " ")
            });
            join(words.filter(|w| !w.is_empty()), word_sep)
        })
        .collect();
    non_empty(lines.join("\n").trim().to_owned())
}

fn morse_decode(input: &str) -> Result<String, String> {
    if !input.chars().map(normalize_morse).any(|c| c == '.' || c == '-') {
        return Err(t!("No Morse code in the selection").into());
    }
    let lines = input.lines().map(|line| {
        let line: String = line.chars().map(normalize_morse).collect();
        let line = line.replace(['/', '|'], "   ");
        let mut words = vec![String::new()];
        let mut gap = false;
        for tok in line.split(' ') {
            if tok.is_empty() {
                gap = true;
                continue;
            }
            if gap && !words.last().is_some_and(String::is_empty) {
                words.push(String::new());
            }
            gap = false;
            let letter = MORSE.iter().find(|(_, code)| *code == tok).map_or('?', |(c, _)| *c);
            if let Some(w) = words.last_mut() {
                w.push(letter);
            }
        }
        words.join(" ").trim().to_owned()
    });
    Ok(join(lines, "\n"))
}

fn morse_tool(input: &str, o: &Opts) -> Result<String, String> {
    let decode = match o.get("direction").as_str() {
        "encode" => false,
        "decode" => true,
        _ => is_morse(input),
    };
    if decode {
        return morse_decode(input);
    }
    let sep = match o.get("word_sep").as_str() {
        "pipe" => " | ",
        "space" => "   ",
        _ => " / ",
    };
    morse_encode(input, &o.get("dash"), sep)
}

// ===== phone keypad =======================================================

const KEYPAD: [&str; 10] = ["", "", "ABC", "DEF", "GHI", "JKL", "MNO", "PQRS", "TUV", "WXYZ"];

fn key_of(c: char) -> Option<(usize, usize)> {
    KEYPAD.iter().enumerate().find_map(|(key, letters)| letters.find(c).map(|pos| (key, pos + 1)))
}

fn phone_tool(input: &str, o: &Opts) -> Result<String, String> {
    let style = o.get("style");
    if o.get("direction") == "encode" {
        let words: Vec<String> = words(input, Umlauts::Expand)
            .iter()
            .map(|w| {
                let codes = w.iter().filter_map(|&c| key_of(c)).map(|(key, pos)| match style.as_str() {
                    "position" => format!("{key}{pos}"),
                    "key" => key.to_string(),
                    _ => key.to_string().repeat(pos),
                });
                join(codes, if style == "key" { "" } else { " " })
            })
            .collect();
        return non_empty(words.join(if style == "key" { " " } else { " | " }));
    }
    if style == "key" {
        return Err(t!("Key-only (T9) codes are ambiguous and can't be decoded – use multi-tap or key + position").into());
    }
    // 0, | and / separate words.
    let input = input.replace(['|', '/'], " 0 ");
    let mut out = String::new();
    for token in input.split_whitespace() {
        if token.chars().all(|c| c == '0') {
            out.push(' ');
            continue;
        }
        let digits: Vec<usize> = token
            .chars()
            .map(|c| c.to_digit(10).map(|d| d as usize))
            .collect::<Option<_>>()
            .ok_or_else(|| t!("\"{}\" is not a key sequence", token))?;
        if style == "position" {
            if !digits.len().is_multiple_of(2) {
                return Err(t!("\"{}\" is not a list of key + position pairs", token));
            }
            for pair in digits.chunks(2) {
                out.push(pair[1].checked_sub(1).and_then(|p| KEYPAD[pair[0]].chars().nth(p)).unwrap_or('?'));
            }
        } else {
            // Runs of the same key: 4 33 666 or 433666.
            let mut i = 0;
            while i < digits.len() {
                let n = digits[i..].iter().take_while(|&&d| d == digits[i]).count();
                let letters = KEYPAD[digits[i]];
                out.push(match digits[i] {
                    0 => ' ',
                    _ if letters.is_empty() => '?',
                    _ => letters.chars().nth((n - 1) % letters.len()).unwrap_or('?'),
                });
                i += n;
            }
        }
    }
    non_empty(join(out.split_whitespace(), " "))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(id: &str, input: &str, opts: &[(&str, &str)]) -> Result<String, String> {
        let values = opts.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
        find(id).unwrap().run(input, &values)
    }

    #[test]
    fn menu_ids_exist() {
        for id in MENU.iter().flat_map(|s| s.iter()) {
            assert!(find(id).is_some(), "{id}");
        }
    }

    #[test]
    fn rotations() {
        assert_eq!(run("rot13", "Geocache 42", &[]).unwrap(), "Trbpnpur 42");
        assert_eq!(run("rot5", "N 52° 31.123", &[]).unwrap(), "N 07° 86.678");
        assert_eq!(run("rot18", "Abc 123", &[]).unwrap(), "Nop 678");
        assert_eq!(run("rot47", "Hello!", &[]).unwrap(), "w6==@P");
        assert_eq!(run("caesar", "abc", &[("shift", "3")]).unwrap(), "def");
        assert_eq!(run("caesar", "def", &[("shift", "-3")]).unwrap(), "abc");
        assert_eq!(run("caesar", "a", &[("all", "true")]).unwrap().lines().count(), 25);
    }

    #[test]
    fn letter_values() {
        let out = run("letter-value", "Geo Cache", &[]).unwrap();
        assert_eq!(
            out,
            "Letter values: 7 5 15 | 3 1 3 8 5\nWord values: 27 | 20\nSum of all letters: 47\nChecksum of the sum: 11\nSingle-digit checksum: 2"
        );
        let only = |extra: (&str, &str)| {
            let mut o = vec![("values", "false"), ("word_sums", "false"), ("sum", "false"), ("checksum", "false"), ("digital_root", "false")];
            o.push(extra);
            run("letter-value", "Größe", &o)
        };
        // G R O E S S E = 7 18 15 5 19 19 5
        assert_eq!(only(("sum", "true")).unwrap(), "Sum of all letters: 88");
        assert_eq!(
            only(("letter_checksums", "true")).unwrap(),
            "Letter checksums: 7 9 6 5 10 10 5\nSum of letter checksums: 52"
        );
        assert_eq!(run("letter-value", "Ä", &[("umlauts", "extend"), ("values", "true")]).unwrap().lines().next().unwrap(), "Letter values: 27");
        assert_eq!(run("letter-value", "z", &[("order", "za"), ("sum", "true")]).unwrap().lines().next().unwrap(), "Letter values: 1");
        assert_eq!(run("letter-value", "7-5-15 | 3,1,3,8,5", &[("direction", "decode")]).unwrap(), "GEO CACHE");
        assert_eq!(run("letter-value", "1 99", &[("direction", "decode")]).unwrap(), "A?");
        assert_eq!(digital_root(1432), 1);
        assert_eq!(digital_root(32), 5);
    }

    #[test]
    fn frequency() {
        assert_eq!(run("letter-frequency", "Abba c", &[]).unwrap(), "A=2 B=2 C=1\nTotal: 5, different: 3");
        assert!(run("letter-frequency", "Abc", &[("missing", "true")]).unwrap().ends_with("Missing: D E F G H I J K L M N O P Q R S T U V W X Y Z"));
    }

    #[test]
    fn scrabble_and_primes() {
        assert_eq!(run("scrabble", "Quiz", &[]).unwrap(), "Values: 10 1 1 3\nSum: 15");
        assert_eq!(run("scrabble", "Quiz", &[("lang", "en")]).unwrap(), "Values: 10 1 1 10\nSum: 22");
        assert_eq!(run("scrabble", "Öl", &[]).unwrap(), "Values: 8 2\nSum: 10");
        assert_eq!(run("prime-alphabet", "abz", &[]).unwrap(), "Values: 2 3 101\nSum: 106");
        assert_eq!(run("prime-alphabet", "2 3 101 4", &[("direction", "decode")]).unwrap(), "ABZ?");
    }

    #[test]
    fn hex() {
        assert_eq!(run("hex", "47 45 4F", &[]).unwrap(), "GEO");
        assert_eq!(run("hex", "0x47454f", &[]).unwrap(), "GEO");
        assert_eq!(run("hex", "C3 A4", &[]).unwrap(), "ä");
        assert_eq!(run("hex", "FF, 0x10", &[("mode", "hex-to-dec")]).unwrap(), "255 16");
        assert_eq!(run("hex", "255 16", &[("mode", "dec-to-hex")]).unwrap(), "FF 10");
        assert_eq!(run("hex", "GEO", &[("mode", "text-to-hex")]).unwrap(), "47 45 4F");
        assert!(run("hex", "XYZ", &[]).is_err());
    }

    #[test]
    fn ascii() {
        assert_eq!(run("ascii", "71 69 79", &[]).unwrap(), "GEO");
        assert_eq!(run("ascii", "71,69,79", &[]).unwrap(), "GEO");
        assert_eq!(run("ascii", "7169791013", &[]).unwrap_err().contains("split"), true);
        assert_eq!(run("ascii", "716979101", &[]).unwrap(), "GEOe");
        assert_eq!(run("ascii", "Hi", &[("mode", "text-to-codes")]).unwrap(), "72 105");
    }

    #[test]
    fn binary() {
        assert_eq!(run("binary", "101 11111111", &[]).unwrap(), "5 255");
        assert_eq!(run("binary", "5", &[("mode", "dec-to-bin")]).unwrap(), "101");
        assert_eq!(run("binary", "5", &[("mode", "dec-to-bin"), ("pad", "true")]).unwrap(), "00000101");
        assert_eq!(run("binary", "010001110100010101001111", &[("mode", "bin-to-text")]).unwrap(), "GEO");
        assert_eq!(run("binary", "GE", &[("mode", "text-to-bin")]).unwrap(), "01000111 01000101");
    }

    #[test]
    fn roman() {
        assert_eq!(parse_roman("MCMXCIV"), Some(1994));
        assert_eq!(parse_roman("IIII"), Some(4));
        assert_eq!(parse_roman("IIX"), Some(8));
        assert_eq!(parse_roman("X\u{305}L\u{305}"), Some(40000));
        assert_eq!(parse_roman("(XL)"), Some(40000));
        assert_eq!(parse_roman("(XL)CCCXLV"), Some(40345));
        assert_eq!(parse_roman("ABC"), None);
        assert_eq!(to_roman(1994, false).unwrap(), "MCMXCIV");
        assert_eq!(to_roman(40345, false).unwrap(), "X\u{305}L\u{305}CCCXLV");
        assert_eq!(to_roman(40345, true).unwrap(), "(XL)CCCXLV");
        assert_eq!(run("roman", "XII, IV and xl.", &[]).unwrap(), "12, 4 and 40.");
        assert_eq!(run("roman", "N 52 13", &[]).unwrap(), "N LII XIII");
        assert_eq!(run("roman", "12", &[("direction", "to-arabic")]).unwrap_err(), "No Roman numerals in the selection");
    }

    #[test]
    fn morse() {
        assert_eq!(run("morse", "SOS Geo", &[]).unwrap(), "... --- ... / --. . ---");
        assert_eq!(run("morse", "Geo", &[("dash", "_"), ("direction", "encode")]).unwrap(), "__. . ___");
        assert_eq!(run("morse", "... --- ... / --. . ---", &[]).unwrap(), "SOS GEO");
        assert_eq!(run("morse", "__. . ___   ...", &[]).unwrap(), "GEO S");
        assert_eq!(run("morse", ".-.- ........", &[]).unwrap(), "Ä?");
        assert!(run("morse", "abc", &[("direction", "decode")]).is_err());
    }

    #[test]
    fn phone() {
        assert_eq!(run("phone", "Geo Cache", &[]).unwrap(), "4 33 666 | 222 2 222 44 33");
        assert_eq!(run("phone", "Geo", &[("style", "position")]).unwrap(), "41 32 63");
        assert_eq!(run("phone", "Geo Cache", &[("style", "key")]).unwrap(), "436 22243");
        assert_eq!(run("phone", "4 33 666 0 222 2", &[("direction", "decode")]).unwrap(), "GEO CA");
        assert_eq!(run("phone", "433666", &[("direction", "decode")]).unwrap(), "GEO");
        assert_eq!(run("phone", "41 32 63 | 22", &[("direction", "decode"), ("style", "position")]).unwrap(), "GEO B");
        assert!(run("phone", "436", &[("direction", "decode"), ("style", "key")]).is_err());
    }
}
