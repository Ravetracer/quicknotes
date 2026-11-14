//! Line-based calculator for "Calculation" tabs.
//!
//! Every line is evaluated top to bottom:
//! - `name = expression` defines a variable
//! - `expression` shows its value
//! - `expression to unit` (also `in`, `as`, `->`) converts the value
//! - `unit name = expression` defines a unit
//! - `> template` fills in single-letter variables and `{expression}` parts
//! - `#` or `//` start a comment; lines that are no calculation are ignored
//!
//! Values may carry units (`3 m + 20 cm`, see `units.rs`), or be dates and
//! times (`today + 6 weeks`, `14:00 Berlin to Tokyo`, see `calendar.rs`).
//! See `data/help.md` for the user documentation.

use crate::calendar;
use crate::units::{self, Quantity, Unit};
use jiff::civil::{Date, Time};
use jiff::tz::TimeZone;
use jiff::{SignedDuration, Span, Zoned};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum LineResult {
    /// Nothing to show (text, comment, empty line, plain `a = 5`).
    Empty,
    /// A plain number.
    Value(f64),
    /// Any other value: with units, a date, a converted value.
    Shown(String),
    /// A filled-in template.
    Text(String),
    Error(String),
}

impl LineResult {
    /// The result as plain text (what gets copied).
    pub fn display(&self) -> Option<String> {
        match self {
            LineResult::Value(v) => Some(format_number(*v)),
            LineResult::Shown(t) | LineResult::Text(t) => Some(t.clone()),
            LineResult::Error(_) | LineResult::Empty => None,
        }
    }
}

/// A calculated value.
#[derive(Debug, Clone)]
pub enum Value {
    Num(Quantity),
    Date(Date),
    /// A point in time; `time_only` shows just the time, with the day
    /// change against `base`.
    Moment { at: Zoned, time_only: bool, base: Option<Date> },
    Text(String),
}

impl Value {
    fn plain(v: f64) -> Self {
        Value::Num(Quantity::plain(v))
    }

    pub fn display(&self) -> String {
        match self {
            Value::Num(q) => q.display(),
            Value::Date(d) => calendar::format_date(*d),
            Value::Moment { at, time_only, base } => calendar::format_moment(at, *time_only, *base),
            Value::Text(t) => t.clone(),
        }
    }

    fn result(&self) -> LineResult {
        match self {
            Value::Num(q) if q.is_plain() => LineResult::Value(q.si),
            other => LineResult::Shown(other.display()),
        }
    }
}

#[derive(Default)]
struct Env {
    vars: HashMap<String, Value>,
    /// Units defined in the sheet (`unit dozen = 12`).
    units: HashMap<String, Unit>,
}

/// Evaluates all lines of `text`; one result per line (split at `\n`).
pub fn evaluate(text: &str) -> Vec<LineResult> {
    let mut env = Env::default();
    text.split('\n').map(|line| eval_line(line.trim_end_matches('\r'), &mut env)).collect()
}

/// What a conversion at the end of a line starts from (for completion).
#[derive(Debug, Clone, PartialEq)]
pub enum Source {
    Quantity(units::Dim),
    Date,
}

/// Evaluates `expr` after the lines of `before`, to offer matching targets.
pub fn source_of(before: &str, expr: &str) -> Option<Source> {
    let mut env = Env::default();
    for line in before.split('\n') {
        eval_line(line.trim_end_matches('\r'), &mut env);
    }
    let code = split_assignment(expr.trim()).map_or(expr.trim(), |(_, e)| e);
    match eval_text(code, &env).ok()?.0 {
        Value::Num(q) => Some(Source::Quantity(q.dim)),
        Value::Date(_) | Value::Moment { .. } => Some(Source::Date),
        Value::Text(_) => None,
    }
}

fn eval_line(line: &str, env: &mut Env) -> LineResult {
    let trimmed = line.trim();
    if let Some(template) = trimmed.strip_prefix('>') {
        return fill_template(template.strip_prefix(' ').unwrap_or(template), env);
    }
    let code = strip_comment(trimmed).trim();
    if code.is_empty() {
        return LineResult::Empty;
    }
    if let Some((name, expr)) = code.strip_prefix("unit ").and_then(split_assignment) {
        return define_unit(name, expr, env);
    }
    if let Some((name, expr)) = split_assignment(code) {
        return match eval_text(expr, env) {
            Ok((v, literal)) => {
                let shown = if literal { LineResult::Empty } else { v.result() };
                env.vars.insert(name.to_owned(), v);
                shown
            }
            Err(e) => LineResult::Error(e.message),
        };
    }
    // Plain expression; anything that does not parse is text.
    let Ok(line) = parse_line(code, env) else { return LineResult::Empty };
    if line.target.is_none() && (is_literal(&line.expr, env) || !mentions_value(&line.expr, env)) {
        return LineResult::Empty;
    }
    match eval_line_expr(&line, env) {
        Ok(v) => v.result(),
        // Words joined by operators ("step-by-step") are text, not typos.
        Err(e) if e.unknown_variable && !mentions_known(&line.expr, env) => LineResult::Empty,
        Err(e) => LineResult::Error(e.message),
    }
}

/// Evaluates an expression with optional conversion; also tells whether it
/// is a plain literal (`5`, `3 m`).
fn eval_text(text: &str, env: &Env) -> Result<(Value, bool), CalcError> {
    let line = parse_line(text, env)?;
    let literal = line.target.is_none() && is_literal(&line.expr, env);
    Ok((eval_line_expr(&line, env)?, literal))
}

fn eval_line_expr(line: &Line, env: &Env) -> Result<Value, CalcError> {
    let v = eval(&line.expr, env)?;
    match &line.target {
        Some(target) => convert(v, target),
        None => Ok(v),
    }
}

fn define_unit(name: &str, expr: &str, env: &mut Env) -> LineResult {
    match eval_text(expr, env) {
        Ok((Value::Num(q), _)) if q.si != 0.0 => {
            env.units.insert(name.to_owned(), Unit { symbol: name.to_owned(), dim: q.dim, factor: q.si, offset: 0.0 });
            LineResult::Empty
        }
        Ok(_) => LineResult::Error(t!("a unit needs a number that is not 0").to_owned()),
        Err(e) => LineResult::Error(e.message),
    }
}

fn strip_comment(code: &str) -> &str {
    let end = [code.find('#'), code.find("//")].into_iter().flatten().min();
    end.map_or(code, |i| &code[..i])
}

/// `name = expr` (but not `==`, `<=`, `>=`, `!=`).
fn split_assignment(code: &str) -> Option<(&str, &str)> {
    let pos = code.find('=')?;
    let (left, right) = (&code[..pos], &code[pos + 1..]);
    if right.starts_with('=') || left.ends_with(['<', '>', '!', '=']) {
        return None;
    }
    let name = left.trim();
    is_identifier(name).then_some((name, right.trim()))
}

fn is_identifier(s: &str) -> bool {
    let mut chars = s.chars();
    matches!(chars.next(), Some(c) if c.is_alphabetic() || c == '_') && chars.all(|c| c.is_alphanumeric() || c == '_')
}

const DATE_WORDS: &[&str] = &["today", "heute", "now", "jetzt", "tomorrow", "morgen", "yesterday", "gestern"];

/// Whether `name` is a unit (and not a variable).
fn is_unit(name: &str, env: &Env) -> bool {
    !env.vars.contains_key(name) && (env.units.contains_key(name) || units::lookup(name).is_some())
}

/// `5`, `-2`, `3 m`, `5 ft 3 in`, `2026-12-24`, `14:00 Berlin`: nothing
/// worth showing as a result.
fn is_literal(expr: &Expr, env: &Env) -> bool {
    let number = |e: &Expr| matches!(e, Expr::Num(_)) || matches!(e, Expr::Neg(n) if matches!(**n, Expr::Num(_)));
    match expr {
        Expr::Mixed(_) | Expr::Date(_) | Expr::Time(_) | Expr::DateTime(..) => true,
        Expr::InZone(e, _) => is_literal(e, env),
        Expr::Bin('*', a, b) => number(a) && matches!(&**b, Expr::Var(u) if is_unit(u, env)),
        e => number(e),
    }
}

/// Whether there is anything but units ("km/h" alone is text).
fn mentions_value(expr: &Expr, env: &Env) -> bool {
    match expr {
        Expr::Num(_) | Expr::Call(..) | Expr::Mixed(_) | Expr::Date(_) | Expr::Time(_) | Expr::DateTime(..) => true,
        Expr::Var(n) => env.vars.contains_key(n) || DATE_WORDS.contains(&n.as_str()),
        Expr::Neg(e) | Expr::Fact(e) | Expr::InZone(e, _) => mentions_value(e, env),
        Expr::Bin(_, a, b) => mentions_value(a, env) || mentions_value(b, env),
    }
}

fn mentions_known(expr: &Expr, env: &Env) -> bool {
    match expr {
        Expr::Num(_) | Expr::Mixed(_) | Expr::Date(_) | Expr::Time(_) | Expr::DateTime(..) => true,
        Expr::Var(n) => env.vars.contains_key(n) || n == "pi",
        Expr::Neg(e) | Expr::Fact(e) | Expr::InZone(e, _) => mentions_known(e, env),
        Expr::Bin(_, a, b) => mentions_known(a, env) || mentions_known(b, env),
        Expr::Call(_, args) => !args.is_empty(),
    }
}

// ===== templates ===========================================================

/// Replaces single letters that are defined variables and `{expr[:fmt]}`.
/// `\x` keeps the character `x` literally.
fn fill_template(template: &str, env: &Env) -> LineResult {
    let mut out = String::with_capacity(template.len() + 8);
    let mut chars = template.chars();
    while let Some(c) = chars.next() {
        match c {
            '\\' => {
                if let Some(next) = chars.next() {
                    out.push(next);
                }
            }
            '{' => {
                let inner: String = chars.by_ref().take_while(|&c| c != '}').collect();
                let (expr, spec) = match inner.rsplit_once(':') {
                    Some((e, s)) if !s.trim().contains(' ') && !e.trim().is_empty() => (e, Some(s.trim())),
                    _ => (inner.as_str(), None),
                };
                let value = match eval_text(expr, env) {
                    Ok((v, _)) => v,
                    Err(e) => return LineResult::Error(format!("{{{}}}: {}", inner.trim(), e.message)),
                };
                let text = match (&value, spec) {
                    (Value::Num(q), Some(_)) if q.mixed.is_empty() => {
                        let unit = units::format_units(&q.units);
                        apply_format(q.number(), spec).map(|n| if unit.is_empty() { n } else { format!("{n} {unit}") })
                    }
                    (Value::Num(q), None) if q.is_plain() => Ok(format_number(q.si)),
                    (v, _) => Ok(v.display()),
                };
                match text {
                    Ok(s) => out.push_str(&s),
                    Err(e) => return LineResult::Error(e),
                }
            }
            c if c.is_alphabetic() => match env.vars.get(c.encode_utf8(&mut [0; 4]) as &str) {
                Some(v) => out.push_str(&v.display()),
                None => out.push(c),
            },
            c => out.push(c),
        }
    }
    LineResult::Text(out)
}

/// `3` = at least 3 digits before the point (zero-padded), `.2` = 2 decimals,
/// `3.2` = both.
fn apply_format(value: f64, spec: Option<&str>) -> Result<String, String> {
    let Some(spec) = spec.filter(|s| !s.is_empty()) else {
        return Ok(format_number(value));
    };
    let bad = || t!("invalid format \":{}\" (use e.g. :3, :.2 or :3.2)", spec);
    let (width, decimals) = match spec.split_once('.') {
        Some((w, d)) => (w, Some(d.parse::<usize>().map_err(|_| bad())?)),
        None => (spec, None),
    };
    let width = if width.is_empty() { 0 } else { width.parse::<usize>().map_err(|_| bad())? };
    if width > 30 || decimals.is_some_and(|d| d > 15) {
        return Err(bad());
    }
    let text = match decimals {
        Some(d) => format!("{:.*}", d, value),
        None => format_number(value),
    };
    let (sign, digits) = match text.strip_prefix('-') {
        Some(rest) => ("-", rest),
        None => ("", text.as_str()),
    };
    let int_len = digits.find('.').unwrap_or(digits.len());
    let pad = "0".repeat(width.saturating_sub(int_len));
    Ok(format!("{sign}{pad}{digits}"))
}

/// Integers without decimals, otherwise up to 10 decimals without trailing zeros.
pub fn format_number(v: f64) -> String {
    if v.fract().abs() < 1e-9 && v.abs() < 1e15 {
        let i = v.round() as i64;
        return if i == 0 { "0".into() } else { i.to_string() };
    }
    let s = format!("{v:.10}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s == "-0" { "0".into() } else { s.to_owned() }
}

// ===== tokens ==============================================================

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Num(f64),
    Ident(String),
    Op(char),
    LParen,
    RParen,
    Comma,
    /// `->` or `→`
    Arrow,
    Date(Date),
    Time(Time),
}

#[derive(Debug, Clone)]
struct Token {
    tok: Tok,
    /// Byte range in the source.
    start: usize,
    end: usize,
}

const CURRENCY_SIGNS: &[char] = &['€', '$', '£', '¥', '₹', '₺', '₩', '₪'];
const SUPERSCRIPTS: &str = "⁰¹²³⁴⁵⁶⁷⁸⁹";

fn tokenize(s: &str) -> Result<Vec<Token>, String> {
    let chars: Vec<(usize, char)> = s.char_indices().collect();
    let at = |i: usize| chars.get(i).map(|c| c.1);
    let pos = |i: usize| chars.get(i).map_or(s.len(), |c| c.0);
    let mut toks = Vec::new();
    let mut push = |tok: Tok, from: usize, to: usize| toks.push(Token { tok, start: pos(from), end: pos(to) });
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i].1;
        let starts_number = c.is_ascii_digit() || (c == '.' && at(i + 1).is_some_and(|d| d.is_ascii_digit()));
        match c {
            c if c.is_whitespace() => i += 1,
            _ if starts_number => {
                if let Some(found) = date_or_time(&chars, i) {
                    let (tok, len) = found?;
                    push(tok, i, i + len);
                    i += len;
                } else {
                    let (n, len) = number(&chars, i)?;
                    push(Tok::Num(n), i, i + len);
                    i += len;
                }
            }
            c if CURRENCY_SIGNS.contains(&c) => {
                // `$5` = `5 $`
                if at(i + 1).is_some_and(|d| d.is_ascii_digit() || d == '.') {
                    let (n, len) = number(&chars, i + 1)?;
                    push(Tok::Num(n), i + 1, i + 1 + len);
                    push(Tok::Ident(c.to_string()), i, i + 1);
                    i += 1 + len;
                } else {
                    push(Tok::Ident(c.to_string()), i, i + 1);
                    i += 1;
                }
            }
            '°' => {
                let scale = at(i + 1).filter(|n| matches!(n, 'C' | 'F'));
                match scale {
                    Some(sc) if !at(i + 2).is_some_and(|n| n.is_alphanumeric()) => {
                        push(Tok::Ident(format!("°{sc}")), i, i + 2);
                        i += 2;
                    }
                    _ => {
                        push(Tok::Ident("°".into()), i, i + 1);
                        i += 1;
                    }
                }
            }
            '′' | '″' => {
                push(Tok::Ident(c.to_string()), i, i + 1);
                i += 1;
            }
            c if c == '⁻' || SUPERSCRIPTS.contains(c) => {
                let start = i;
                let negative = c == '⁻';
                if negative {
                    i += 1;
                }
                let mut n = 0u32;
                let digits_start = i;
                while let Some(d) = at(i).and_then(|d| SUPERSCRIPTS.chars().position(|s| s == d)) {
                    n = n * 10 + d as u32;
                    i += 1;
                }
                if i == digits_start {
                    return Err(t!("unexpected \"{}\"", c));
                }
                push(Tok::Op('^'), start, start);
                if negative {
                    push(Tok::Op('-'), start, start + 1);
                }
                push(Tok::Num(n as f64), digits_start, i);
            }
            c if c.is_alphabetic() || c == '_' => {
                let start = i;
                while at(i).is_some_and(|c| (c.is_alphanumeric() || c == '_') && c != '⁻' && !SUPERSCRIPTS.contains(c)) {
                    i += 1;
                }
                let word: String = chars[start..i].iter().map(|c| c.1).collect();
                push(Tok::Ident(word), start, i);
            }
            '*' if at(i + 1) == Some('*') => {
                push(Tok::Op('^'), i, i + 2);
                i += 2;
            }
            '-' if at(i + 1) == Some('>') => {
                push(Tok::Arrow, i, i + 2);
                i += 2;
            }
            '→' => {
                push(Tok::Arrow, i, i + 1);
                i += 1;
            }
            '+' | '-' | '*' | '/' | '%' | '^' | '!' => {
                push(Tok::Op(c), i, i + 1);
                i += 1;
            }
            '×' | '·' => {
                push(Tok::Op('*'), i, i + 1);
                i += 1;
            }
            '÷' | ':' => {
                push(Tok::Op('/'), i, i + 1);
                i += 1;
            }
            '−' => {
                push(Tok::Op('-'), i, i + 1);
                i += 1;
            }
            '(' | '[' => {
                push(Tok::LParen, i, i + 1);
                i += 1;
            }
            ')' | ']' => {
                push(Tok::RParen, i, i + 1);
                i += 1;
            }
            ',' | ';' => {
                push(Tok::Comma, i, i + 1);
                i += 1;
            }
            c => return Err(t!("unexpected \"{}\"", c)),
        }
    }
    Ok(toks)
}

fn digit_run(chars: &[(usize, char)], i: usize) -> usize {
    chars[i.min(chars.len())..].iter().take_while(|c| c.1.is_ascii_digit()).count()
}

fn digits_value(chars: &[(usize, char)], i: usize, len: usize) -> i64 {
    chars[i..i + len].iter().fold(0, |n, c| n * 10 + c.1.to_digit(10).unwrap_or(0) as i64)
}

/// `2026-12-24`, `24.12.2026`, `14:30`, `14:30:15`.
fn date_or_time(chars: &[(usize, char)], i: usize) -> Option<Result<(Tok, usize), String>> {
    let at = |k: usize| chars.get(k).map(|c| c.1);
    let ends = |k: usize| !at(k).is_some_and(|c| c.is_ascii_digit());
    let d1 = digit_run(chars, i);
    // ISO date
    if d1 == 4 && at(i + 4) == Some('-') {
        let d2 = digit_run(chars, i + 5);
        if (1..=2).contains(&d2) && at(i + 5 + d2) == Some('-') {
            let k = i + 6 + d2;
            let d3 = digit_run(chars, k);
            if (1..=2).contains(&d3) && ends(k + d3) {
                let date = calendar::date(digits_value(chars, i, 4), digits_value(chars, i + 5, d2), digits_value(chars, k, d3));
                return Some(date.map(|d| (Tok::Date(d), k + d3 - i)));
            }
        }
    }
    // German date
    if (1..=2).contains(&d1) && at(i + d1) == Some('.') {
        let d2 = digit_run(chars, i + d1 + 1);
        if (1..=2).contains(&d2) && at(i + d1 + 1 + d2) == Some('.') {
            let k = i + d1 + 2 + d2;
            if digit_run(chars, k) == 4 && ends(k + 4) {
                let date = calendar::date(digits_value(chars, k, 4), digits_value(chars, i + d1 + 1, d2), digits_value(chars, i, d1));
                return Some(date.map(|d| (Tok::Date(d), k + 4 - i)));
            }
        }
    }
    // Time
    if (1..=2).contains(&d1) && at(i + d1) == Some(':') && digit_run(chars, i + d1 + 1) == 2 {
        let (h, m) = (digits_value(chars, i, d1), digits_value(chars, i + d1 + 1, 2));
        let mut len = d1 + 3;
        let mut sec = 0;
        if at(i + len) == Some(':') && digit_run(chars, i + len + 1) == 2 {
            sec = digits_value(chars, i + len + 1, 2);
            len += 3;
        }
        if h <= 23 && m <= 59 && sec <= 59 && ends(i + len) {
            let time = Time::new(h as i8, m as i8, sec as i8, 0).ok()?;
            return Some(Ok((Tok::Time(time), len)));
        }
    }
    None
}

/// A decimal number (`1.5`, `2e6`) or `0x1F`, `0b101`, `0o17`.
fn number(chars: &[(usize, char)], i: usize) -> Result<(f64, usize), String> {
    let at = |k: usize| chars.get(k).map(|c| c.1);
    if at(i) == Some('0') {
        let radix = match at(i + 1) {
            Some('x' | 'X') => 16,
            Some('b' | 'B') => 2,
            Some('o' | 'O') => 8,
            _ => 0,
        };
        if radix > 0 {
            let len = chars[i + 2..].iter().take_while(|c| c.1.is_digit(radix)).count();
            if len > 0 {
                let text: String = chars[i + 2..i + 2 + len].iter().map(|c| c.1).collect();
                let n = i64::from_str_radix(&text, radix).map_err(|_| t!("invalid number \"{}\"", text))?;
                return Ok((n as f64, len + 2));
            }
        }
    }
    let mut k = i;
    while at(k).is_some_and(|c| c.is_ascii_digit() || c == '.') {
        k += 1;
    }
    // Exponent: `2e6`, `1.5E-3`
    if matches!(at(k), Some('e' | 'E')) {
        let sign = usize::from(matches!(at(k + 1), Some('+' | '-')));
        if at(k + 1 + sign).is_some_and(|c| c.is_ascii_digit()) {
            k += 1 + sign;
            while at(k).is_some_and(|c| c.is_ascii_digit()) {
                k += 1;
            }
        }
    }
    let text: String = chars[i..k].iter().map(|c| c.1).collect();
    let n = text.parse::<f64>().map_err(|_| t!("invalid number \"{}\"", text))?;
    Ok((n, k - i))
}

// ===== expressions =========================================================

#[derive(Debug, Clone, PartialEq)]
enum Expr {
    Num(f64),
    Var(String),
    Neg(Box<Expr>),
    Fact(Box<Expr>),
    Bin(char, Box<Expr>, Box<Expr>),
    Call(String, Vec<Expr>),
    /// `3 h 20 min`, `5 ft 3 in`
    Mixed(Vec<(Expr, String)>),
    Date(Date),
    Time(Time),
    DateTime(Date, Time),
    /// A date or time in a time zone (`14:00 Tokyo`).
    InZone(Box<Expr>, String),
}

struct CalcError {
    message: String,
    unknown_variable: bool,
}

impl From<String> for CalcError {
    fn from(message: String) -> Self {
        Self { message, unknown_variable: false }
    }
}

fn err<T>(message: impl Into<String>) -> Result<T, CalcError> {
    Err(CalcError { message: message.into(), unknown_variable: false })
}

/// A parsed line: expression and optional conversion target.
struct Line {
    expr: Expr,
    target: Option<Target>,
}

const CONVERT_WORDS: &[&str] = &["to", "in", "as", "nach"];

fn parse_line(code: &str, env: &Env) -> Result<Line, String> {
    let toks = tokenize(code)?;
    // `… to unit`: the last conversion word with a valid target wins.
    for k in (1..toks.len().saturating_sub(1)).rev() {
        let is_convert = match &toks[k].tok {
            Tok::Arrow => true,
            Tok::Ident(w) => CONVERT_WORDS.contains(&w.as_str()),
            _ => false,
        };
        if !is_convert {
            continue;
        }
        let Some(target) = parse_target(&code[toks[k].end..], env) else { continue };
        if let Ok(expr) = parse_tokens(code, toks[..k].to_vec()) {
            return Ok(Line { expr, target: Some(target) });
        }
    }
    Ok(Line { expr: parse_tokens(code, toks)?, target: None })
}

fn parse_tokens(src: &str, toks: Vec<Token>) -> Result<Expr, String> {
    if toks.is_empty() {
        return Err(t!("empty expression").into());
    }
    let mut p = Parser { src, toks, pos: 0 };
    let expr = p.expr()?;
    match p.peek() {
        None => Ok(expr),
        Some(t) => Err(t!("unexpected {}", describe(t))),
    }
}

fn describe(t: &Tok) -> String {
    match t {
        Tok::Num(n) => format_number(*n),
        Tok::Ident(s) => format!("\"{s}\""),
        Tok::Op(c) => format!("\"{c}\""),
        Tok::LParen => "\"(\"".into(),
        Tok::RParen => "\")\"".into(),
        Tok::Comma => "\",\"".into(),
        Tok::Arrow => "\"->\"".into(),
        Tok::Date(d) => d.to_string(),
        Tok::Time(t) => t.strftime("%H:%M").to_string(),
    }
}

struct Parser<'a> {
    src: &'a str,
    toks: Vec<Token>,
    pos: usize,
}

impl Parser<'_> {
    fn peek(&self) -> Option<&Tok> {
        self.toks.get(self.pos).map(|t| &t.tok)
    }

    fn peek_at(&self, offset: usize) -> Option<&Tok> {
        self.toks.get(self.pos + offset).map(|t| &t.tok)
    }

    fn next(&mut self) -> Option<Tok> {
        let t = self.peek().cloned();
        self.pos += 1;
        t
    }

    fn eat_op(&mut self, ops: &[char]) -> Option<char> {
        match self.peek() {
            Some(Tok::Op(c)) if ops.contains(c) => {
                let c = *c;
                self.pos += 1;
                Some(c)
            }
            _ => None,
        }
    }

    // expr := term (('+' | '-') term)*
    fn expr(&mut self) -> Result<Expr, String> {
        let mut left = self.term()?;
        while let Some(op) = self.eat_op(&['+', '-']) {
            left = Expr::Bin(op, Box::new(left), Box::new(self.term()?));
        }
        Ok(left)
    }

    // term := implicit (('*' | '/' | '%') implicit)*
    fn term(&mut self) -> Result<Expr, String> {
        let mut left = self.implicit()?;
        while let Some(op) = self.eat_op(&['*', '/', '%']) {
            left = Expr::Bin(op, Box::new(left), Box::new(self.implicit()?));
        }
        Ok(left)
    }

    // implicit := unary unary*   (binds tighter than `/`: 120 km / 1.5 h)
    fn implicit(&mut self) -> Result<Expr, String> {
        let mut factors = vec![self.unary()?];
        loop {
            if self.implicit_multiplication() || self.mixed_continues(&factors) {
                factors.push(self.unary()?);
            } else {
                break;
            }
        }
        if factors.len() >= 4 && factors.len().is_multiple_of(2) && factors.chunks(2).all(|p| is_number_unit(&p[0], &p[1])) {
            let pairs = factors
                .chunks(2)
                .map(|p| match &p[1] {
                    Expr::Var(u) => (p[0].clone(), u.clone()),
                    _ => unreachable!(),
                })
                .collect();
            return Ok(Expr::Mixed(pairs));
        }
        let mut it = factors.into_iter();
        let first = it.next().ok_or_else(|| t!("incomplete expression").to_owned())?;
        Ok(it.fold(first, |a, b| Expr::Bin('*', Box::new(a), Box::new(b))))
    }

    /// `2a`, `2(a+b)`, `(a+b)(c+d)`, `(a)b` – after a number or `)`.
    fn implicit_multiplication(&self) -> bool {
        let prev = self.pos.checked_sub(1).and_then(|i| self.toks.get(i)).map(|t| &t.tok);
        matches!(prev, Some(Tok::Num(_)) | Some(Tok::RParen))
            && matches!(self.peek(), Some(Tok::Ident(_)) | Some(Tok::LParen) | Some(Tok::Num(_)))
    }

    /// `3 h` followed by `20 min`.
    fn mixed_continues(&self, factors: &[Expr]) -> bool {
        factors.len().is_multiple_of(2)
            && factors.chunks(2).all(|p| is_number_unit(&p[0], &p[1]))
            && matches!(self.peek(), Some(Tok::Num(_)))
            && matches!(self.peek_at(1), Some(Tok::Ident(_)))
    }

    // unary := ('-' | '+') unary | power
    fn unary(&mut self) -> Result<Expr, String> {
        match self.eat_op(&['-', '+']) {
            Some('-') => Ok(Expr::Neg(Box::new(self.unary()?))),
            Some(_) => self.unary(),
            None => self.power(),
        }
    }

    // power := postfix ('^' unary)?   (right-associative, -2^2 = -4)
    fn power(&mut self) -> Result<Expr, String> {
        let base = self.postfix()?;
        if self.eat_op(&['^']).is_some() {
            return Ok(Expr::Bin('^', Box::new(base), Box::new(self.unary()?)));
        }
        Ok(base)
    }

    // postfix := primary '!'*
    fn postfix(&mut self) -> Result<Expr, String> {
        let mut e = self.primary()?;
        while self.eat_op(&['!']).is_some() {
            e = Expr::Fact(Box::new(e));
        }
        Ok(e)
    }

    fn primary(&mut self) -> Result<Expr, String> {
        match self.next() {
            Some(Tok::Num(n)) => Ok(Expr::Num(n)),
            Some(Tok::Ident(name)) => {
                if self.peek() == Some(&Tok::LParen) {
                    self.pos += 1;
                    let mut args = Vec::new();
                    if self.peek() != Some(&Tok::RParen) {
                        loop {
                            args.push(self.expr()?);
                            if self.peek() == Some(&Tok::Comma) {
                                self.pos += 1;
                            } else {
                                break;
                            }
                        }
                    }
                    match self.next() {
                        Some(Tok::RParen) => Ok(Expr::Call(name, args)),
                        _ => Err(t!("missing \")\"").into()),
                    }
                } else {
                    Ok(Expr::Var(name))
                }
            }
            Some(Tok::LParen) => {
                let e = self.expr()?;
                match self.next() {
                    Some(Tok::RParen) => Ok(e),
                    _ => Err(t!("missing \")\"").into()),
                }
            }
            Some(Tok::Date(d)) => {
                let e = match self.peek() {
                    Some(Tok::Time(t)) => {
                        let t = *t;
                        self.pos += 1;
                        Expr::DateTime(d, t)
                    }
                    _ => Expr::Date(d),
                };
                Ok(self.zone_suffix(e))
            }
            Some(Tok::Time(t)) => Ok(self.zone_suffix(Expr::Time(t))),
            Some(t) => Err(t!("unexpected {}", describe(&t))),
            None => Err(t!("incomplete expression").into()),
        }
    }

    /// A time zone after a date or time: the longest run of following
    /// tokens that names one (`New York`, `Europe/Berlin`, `UTC+2`).
    fn zone_suffix(&mut self, e: Expr) -> Expr {
        let Some(first) = self.toks.get(self.pos) else { return e };
        if !matches!(first.tok, Tok::Ident(_)) {
            return e;
        }
        let mut best = None;
        for len in 1..=5 {
            let Some(last) = self.toks.get(self.pos + len - 1) else { break };
            let usable = matches!(&last.tok, Tok::Ident(w) if !CONVERT_WORDS.contains(&w.as_str()))
                || matches!(last.tok, Tok::Op('/' | '+' | '-') | Tok::Num(_));
            if !usable {
                break;
            }
            let text = &self.src[first.start..last.end];
            if calendar::zone(text).is_some() {
                best = Some((len, text.to_owned()));
            }
        }
        match best {
            Some((len, name)) => {
                self.pos += len;
                Expr::InZone(Box::new(e), name)
            }
            None => e,
        }
    }
}

fn is_number_unit(n: &Expr, u: &Expr) -> bool {
    let number = matches!(n, Expr::Num(_)) || matches!(n, Expr::Neg(x) if matches!(**x, Expr::Num(_)));
    number && matches!(u, Expr::Var(_))
}

// ===== conversion targets ===================================================

#[derive(Debug, Clone, Copy, PartialEq)]
enum Special {
    Hex,
    Bin,
    Oct,
    Roman,
    Unix,
    JulianDay,
    Julian,
    Hebrew,
    Islamic,
    Week,
    Weekday,
    DayOfYear,
    Iso,
    Date,
}

impl Special {
    fn parse(word: &str) -> Option<Self> {
        Some(match word.to_lowercase().as_str() {
            "hex" | "hexadecimal" => Self::Hex,
            "bin" | "binary" | "binär" => Self::Bin,
            "oct" | "octal" | "oktal" => Self::Oct,
            "roman" | "römisch" => Self::Roman,
            "unix" | "timestamp" | "epoch" => Self::Unix,
            "jd" | "jdn" => Self::JulianDay,
            "julian" | "julianisch" => Self::Julian,
            "hebrew" | "jewish" | "hebräisch" | "jüdisch" => Self::Hebrew,
            "islamic" | "hijri" | "islamisch" => Self::Islamic,
            "week" | "kw" | "woche" => Self::Week,
            "weekday" | "wochentag" => Self::Weekday,
            "dayofyear" | "doy" => Self::DayOfYear,
            "iso" => Self::Iso,
            "date" | "datum" => Self::Date,
            _ => return None,
        })
    }
}

/// Words offered after `to` besides units (for completion).
pub const SPECIAL_TARGETS: &[(&str, &str)] = &[
    ("hex", "hexadecimal"),
    ("bin", "binary"),
    ("oct", "octal"),
    ("roman", "Roman numerals"),
    ("date", "date of a Unix timestamp"),
];
pub const DATE_TARGETS: &[(&str, &str)] = &[
    ("weekday", "day of the week"),
    ("week", "ISO week"),
    ("dayofyear", "day of the year"),
    ("unix", "Unix timestamp"),
    ("jd", "Julian day number"),
    ("julian", "Julian calendar"),
    ("hebrew", "Hebrew calendar"),
    ("hijri", "Islamic calendar"),
    ("iso", "ISO 8601"),
];

enum UnitTarget {
    Expr(Quantity),
    Mixed(Vec<Unit>),
}

struct Target {
    text: String,
    special: Option<Special>,
    zone: Option<TimeZone>,
    units: Option<Result<UnitTarget, String>>,
}

fn parse_target(text: &str, env: &Env) -> Option<Target> {
    let text = text.trim();
    if text.is_empty() {
        return None;
    }
    let special = Special::parse(text);
    let zone = calendar::zone(text);
    let words: Vec<&str> = text.split([' ', ',']).filter(|w| !w.is_empty()).collect();
    let units = match units::mixed_target(&words) {
        Some(r) => Some(r.map(UnitTarget::Mixed)),
        None => unit_expression(text, env).map(|r| r.map(UnitTarget::Expr)),
    };
    if special.is_none() && zone.is_none() && units.is_none() {
        return None;
    }
    Some(Target { text: text.to_owned(), special, zone, units })
}

/// A unit expression like `km/h` or `L/100km` (no variables).
fn unit_expression(text: &str, env: &Env) -> Option<Result<Quantity, String>> {
    let toks = tokenize(text).ok()?;
    let expr = parse_tokens(text, toks).ok()?;
    let units_only = Env { vars: HashMap::new(), units: env.units.clone() };
    if !only_units(&expr, &units_only) {
        return None;
    }
    match eval(&expr, &units_only) {
        Ok(Value::Num(q)) if !q.is_plain() && q.si != 0.0 => Some(Ok(q)),
        Ok(_) => None,
        Err(e) if e.unknown_variable => None,
        Err(e) => Some(Err(e.message)),
    }
}

fn only_units(expr: &Expr, env: &Env) -> bool {
    match expr {
        Expr::Num(_) => true,
        Expr::Var(n) => is_unit(n, env),
        Expr::Neg(e) => only_units(e, env),
        Expr::Bin(_, a, b) => only_units(a, env) && only_units(b, env),
        _ => false,
    }
}

fn convert(v: Value, target: &Target) -> Result<Value, CalcError> {
    let cannot = |what: String| err(t!("cannot convert {} to \"{}\"", what, target.text));
    match v {
        Value::Num(q) => {
            match (target.special, &target.units) {
                (Some(sp @ (Special::Hex | Special::Bin | Special::Oct | Special::Roman | Special::Date)), _) => {
                    return number_special(&q, sp);
                }
                (_, Some(Ok(UnitTarget::Expr(tq)))) => return convert_units(q, tq, &target.text),
                (_, Some(Ok(UnitTarget::Mixed(list)))) => {
                    if list.first().is_some_and(|u| u.dim == q.dim) {
                        let mut q = q;
                        q.mixed = list.clone();
                        return Ok(Value::Num(q));
                    }
                }
                (_, Some(Err(e))) => return err(e.clone()),
                _ => {}
            }
            cannot(units::describe_dim(&q.dim))
        }
        Value::Date(d) => {
            if let Some(sp) = target.special {
                return date_special(d, None, sp);
            }
            if let Some(tz) = &target.zone {
                let at = d.to_zoned(TimeZone::system()).map_err(|e| e.to_string())?;
                return Ok(Value::Moment { at: at.with_time_zone(tz.clone()), time_only: false, base: None });
            }
            cannot(t!("a date").to_owned())
        }
        Value::Moment { at, time_only, base } => {
            if let Some(sp) = target.special {
                return date_special(at.date(), Some(&at), sp);
            }
            if let Some(tz) = &target.zone {
                let base = if time_only { base.or(Some(at.date())) } else { None };
                return Ok(Value::Moment { at: at.with_time_zone(tz.clone()), time_only, base });
            }
            cannot(t!("a time").to_owned())
        }
        Value::Text(_) => cannot(t!("text").to_owned()),
    }
}

fn convert_units(q: Quantity, tq: &Quantity, text: &str) -> Result<Value, CalcError> {
    let unit = match tq.units.as_slice() {
        [(u, 1)] if (u.factor - tq.si).abs() <= f64::EPSILON * tq.si.abs() => u.clone(),
        _ => Unit { symbol: text.to_owned(), dim: tq.dim, factor: tq.si, offset: 0.0 },
    };
    if q.dim == tq.dim {
        return Ok(Value::Num(Quantity { si: q.si, dim: q.dim, units: vec![(unit, 1)], delta: q.delta, mixed: Vec::new() }));
    }
    // Reciprocal units: mpg ↔ L/100km
    let inverse = q.dim.map(|x| -x);
    if inverse == tq.dim && q.dim != units::NONE {
        if q.si == 0.0 {
            return err(t!("division by zero"));
        }
        return Ok(Value::Num(Quantity { si: 1.0 / q.si, dim: tq.dim, units: vec![(unit, 1)], delta: false, mixed: Vec::new() }));
    }
    err(t!("cannot convert {} to {}", units::describe_dim(&q.dim), units::describe_dim(&tq.dim)))
}

fn number_special(q: &Quantity, sp: Special) -> Result<Value, CalcError> {
    if !q.is_plain() {
        return err(t!("needs a plain number"));
    }
    let v = q.si;
    if sp == Special::Date {
        return moment_from_unix(v);
    }
    if v.fract() != 0.0 || v.abs() >= 9.007e15 {
        return err(t!("needs a whole number"));
    }
    let n = v as i64;
    let sign = if n < 0 { "-" } else { "" };
    let a = n.unsigned_abs();
    Ok(Value::Text(match sp {
        Special::Hex => format!("{sign}0x{a:X}"),
        Special::Bin => format!("{sign}0b{a:b}"),
        Special::Oct => format!("{sign}0o{a:o}"),
        _ => roman(n).ok_or_else(|| t!("Roman numerals go from 1 to 3999").to_owned())?,
    }))
}

fn roman(n: i64) -> Option<String> {
    if !(1..=3999).contains(&n) {
        return None;
    }
    const PARTS: [(i64, &str); 13] = [
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
    let mut rest = n;
    let mut s = String::new();
    for (value, letters) in PARTS {
        while rest >= value {
            s.push_str(letters);
            rest -= value;
        }
    }
    Some(s)
}

fn moment_from_unix(seconds: f64) -> Result<Value, CalcError> {
    let ts = jiff::Timestamp::from_millisecond((seconds * 1000.0).round() as i64).map_err(|e| e.to_string())?;
    Ok(Value::Moment { at: ts.to_zoned(TimeZone::system()), time_only: false, base: None })
}

fn date_special(d: Date, at: Option<&Zoned>, sp: Special) -> Result<Value, CalcError> {
    let local_midnight = || d.to_zoned(TimeZone::system()).map_err(|e| CalcError::from(e.to_string()));
    Ok(match sp {
        Special::Unix => {
            let ts = match at {
                Some(z) => z.timestamp(),
                None => local_midnight()?.timestamp(),
            };
            Value::plain(ts.as_millisecond() as f64 / 1000.0)
        }
        Special::JulianDay => match at {
            Some(z) => Value::plain(z.timestamp().as_millisecond() as f64 / 86_400_000.0 + 2440587.5),
            None => Value::plain(calendar::julian_day_number(d) as f64),
        },
        Special::Julian => Value::Text(calendar::julian_text(d)),
        Special::Hebrew => Value::Text(calendar::hebrew_text(d)),
        Special::Islamic => Value::Text(calendar::islamic_text(d)),
        Special::Week => Value::Text(calendar::week_text(d)),
        Special::Weekday => Value::Text(calendar::weekday_name(d).to_owned()),
        Special::DayOfYear => Value::plain(d.day_of_year() as f64),
        Special::Iso => Value::Text(match at {
            Some(z) => z.strftime("%Y-%m-%dT%H:%M:%S%:z").to_string(),
            None => d.to_string(),
        }),
        Special::Date => Value::Date(d),
        Special::Hex | Special::Bin | Special::Oct | Special::Roman => return err(t!("needs a plain number")),
    })
}

// ===== evaluation ===========================================================

fn today() -> Date {
    Zoned::now().date()
}

fn resolve(name: &str, env: &Env) -> Result<Value, CalcError> {
    if let Some(v) = env.vars.get(name) {
        return Ok(v.clone());
    }
    let day = |offset: i64| -> Result<Value, CalcError> {
        let d = today().checked_add(Span::new().days(offset)).map_err(|e| e.to_string())?;
        Ok(Value::Date(d))
    };
    match name {
        "today" | "heute" => return day(0),
        "tomorrow" | "morgen" => return day(1),
        "yesterday" | "gestern" => return day(-1),
        "now" | "jetzt" => return Ok(Value::Moment { at: Zoned::now(), time_only: false, base: None }),
        "pi" => return Ok(Value::plain(std::f64::consts::PI)),
        _ => {}
    }
    if let Some(u) = env.units.get(name) {
        return Ok(Value::Num(Quantity::unit(u.clone(), 1)));
    }
    match units::lookup(name) {
        Some(Ok(q)) => Ok(Value::Num(q)),
        Some(Err(e)) => err(e),
        None => Err(CalcError { message: t!("unknown variable \"{}\"", name), unknown_variable: true }),
    }
}

/// The unit a name stands for (not a variable).
fn unit_of(name: &str, env: &Env) -> Option<Result<Quantity, CalcError>> {
    if env.vars.contains_key(name) {
        return None;
    }
    if let Some(u) = env.units.get(name) {
        return Some(Ok(Quantity::unit(u.clone(), 1)));
    }
    units::lookup(name).map(|r| r.map_err(CalcError::from))
}

fn num(v: Value, what: &str) -> Result<Quantity, CalcError> {
    match v {
        Value::Num(q) => Ok(q),
        _ => err(t!("{} needs numbers", what)),
    }
}

fn plain(v: Value, what: &str) -> Result<f64, CalcError> {
    let q = num(v, what)?;
    if q.dim == units::NONE {
        Ok(q.si)
    } else {
        err(t!("{} needs a plain number", what))
    }
}

fn eval(expr: &Expr, env: &Env) -> Result<Value, CalcError> {
    let v = match expr {
        Expr::Num(n) => Value::plain(*n),
        Expr::Var(name) => resolve(name, env)?,
        Expr::Neg(e) => match eval(e, env)? {
            Value::Num(q) => Value::Num(q.with_number(-q.number())),
            _ => return err(t!("cannot negate a date")),
        },
        Expr::Fact(e) => Value::plain(factorial(plain(eval(e, env)?, "!")?)?),
        // `20 °C`: a number with a unit (offsets apply).
        Expr::Bin('*', a, b) if matches!(&**b, Expr::Var(u) if unit_of(u, env).is_some()) => {
            let Expr::Var(u) = &**b else { unreachable!() };
            let unit = unit_of(u, env).expect("checked")?;
            match eval(a, env)? {
                Value::Num(q) if q.is_plain() => Value::Num(Quantity::literal(q.si, &unit)),
                Value::Num(q) => Value::Num(q.mul(&unit, 1)),
                _ => return err(t!("cannot multiply a date")),
            }
        }
        Expr::Bin(op, a, b) => binary(*op, eval(a, env)?, eval(b, env)?)?,
        Expr::Call(name, args) => call(name, args, env)?,
        Expr::Mixed(pairs) => {
            let mut parts = Vec::new();
            for (n, u) in pairs {
                let unit = match unit_of(u, env) {
                    Some(r) => r?,
                    None => num(resolve(u, env)?, u)?,
                };
                parts.push(Quantity::literal(plain(eval(n, env)?, u)?, &unit));
            }
            let same = parts.windows(2).all(|w| w[0].dim == w[1].dim) && parts[0].dim != units::NONE;
            let mut it = parts.into_iter();
            let first = it.next().expect("at least two parts");
            if same {
                Value::Num(it.try_fold(first, |acc, q| acc.add(&q, false))?)
            } else {
                Value::Num(it.fold(first, |acc, q| acc.mul(&q, 1)))
            }
        }
        Expr::Date(d) => Value::Date(*d),
        Expr::Time(t) => at_time(today(), *t, TimeZone::system(), true)?,
        Expr::DateTime(d, t) => at_time(*d, *t, TimeZone::system(), false)?,
        Expr::InZone(e, name) => {
            let tz = calendar::zone(name).ok_or_else(|| t!("unknown time zone \"{}\"", name))?;
            match &**e {
                Expr::Time(t) => at_time(today(), *t, tz, true)?,
                Expr::DateTime(d, t) => at_time(*d, *t, tz, false)?,
                Expr::Date(d) => at_time(*d, Time::midnight(), tz, false)?,
                _ => return err(t!("unknown time zone \"{}\"", name)),
            }
        }
    };
    if let Value::Num(q) = &v {
        if !q.si.is_finite() {
            return err(t!("result is not a finite number"));
        }
    }
    Ok(v)
}

fn at_time(d: Date, t: Time, tz: TimeZone, time_only: bool) -> Result<Value, CalcError> {
    let at = d.to_datetime(t).to_zoned(tz).map_err(|e| e.to_string())?;
    let base = time_only.then(|| at.date());
    Ok(Value::Moment { at, time_only, base })
}

fn binary(op: char, a: Value, b: Value) -> Result<Value, CalcError> {
    use Value::*;
    match (op, a, b) {
        ('+', Num(x), Num(y)) => Ok(Num(x.add(&y, false)?)),
        ('-', Num(x), Num(y)) => Ok(Num(x.add(&y, true)?)),
        ('+', Num(q), d @ (Date(_) | Moment { .. })) | ('+', d @ (Date(_) | Moment { .. }), Num(q)) => shift(d, &q, 1.0),
        ('-', d @ (Date(_) | Moment { .. }), Num(q)) => shift(d, &q, -1.0),
        ('-', Date(x), Date(y)) => {
            let days = (calendar::fixed(x) - calendar::fixed(y)) as f64;
            let day = units::lookup("d").expect("day unit").map_err(CalcError::from)?;
            Ok(Num(Quantity::literal(days, &day)))
        }
        ('-', x @ (Date(_) | Moment { .. }), y @ (Date(_) | Moment { .. })) => {
            let ms = |v: &Value| -> Result<i64, CalcError> {
                Ok(match v {
                    Moment { at, .. } => at.timestamp().as_millisecond(),
                    Date(d) => d.to_zoned(TimeZone::system()).map_err(|e| e.to_string())?.timestamp().as_millisecond(),
                    _ => 0,
                })
            };
            Ok(Num(duration((ms(&x)? - ms(&y)?) as f64 / 1000.0)))
        }
        ('*', Num(x), Num(y)) => Ok(Num(x.mul(&y, 1))),
        ('/', Num(x), Num(y)) => {
            if y.si == 0.0 {
                return err(t!("division by zero"));
            }
            Ok(Num(x.mul(&y, -1)))
        }
        ('%', Num(x), Num(y)) => {
            if y.si == 0.0 {
                return err(t!("division by zero"));
            }
            if x.dim != y.dim && !y.is_plain() {
                return err(t!("cannot convert {} to {}", units::describe_dim(&y.dim), units::describe_dim(&x.dim)));
            }
            let r = if y.is_plain() { x.number() % y.si } else { x.si % y.si / (x.si / x.number()) };
            Ok(Num(x.with_number(r)))
        }
        ('^', Num(x), Num(y)) => {
            if !y.is_plain() {
                return err(t!("the exponent must be a plain number"));
            }
            Ok(Num(x.pow(y.si)?))
        }
        (_, Text(_), _) | (_, _, Text(_)) => err(t!("text cannot be calculated")),
        (op, _, _) => err(t!("\"{}\" does not work with dates", op)),
    }
}

/// A time span, shown as "2 d 3 h 15 min".
fn duration(seconds: f64) -> Quantity {
    let names: &[&str] = match seconds.abs() {
        s if s >= 86400.0 => &["d", "h", "min"],
        s if s >= 3600.0 => &["h", "min"],
        _ => &["min", "s"],
    };
    let mut q = Quantity::literal(seconds, &units::lookup("s").expect("second").expect("no rate"));
    if let Some(Ok(list)) = units::mixed_target(names) {
        q.mixed = list;
    }
    q
}

/// Date/time plus or minus a time span. Whole days, weeks, months and years
/// count in the calendar (`2026-01-31 + 1 month` = 2026-02-28).
fn shift(v: Value, q: &Quantity, sign: f64) -> Result<Value, CalcError> {
    if q.dim != units::lookup("s").expect("second").expect("no rate").dim {
        return err(t!("only time spans can be added to a date"));
    }
    let n = q.number() * sign;
    let calendar_unit = match q.units.as_slice() {
        [(u, 1)] if q.mixed.is_empty() && n.fract() == 0.0 => match u.symbol.as_str() {
            "d" => Some(Span::new().try_days(n as i64)),
            "wk" => Some(Span::new().try_weeks(n as i64)),
            "month" => Some(Span::new().try_months(n as i64)),
            "yr" => Some(Span::new().try_years(n as i64)),
            _ => None,
        },
        _ => None,
    };
    let fail = |e: jiff::Error| CalcError::from(e.to_string());
    if let Some(span) = calendar_unit {
        let span = span.map_err(fail)?;
        return Ok(match v {
            Value::Date(d) => Value::Date(d.checked_add(span).map_err(fail)?),
            Value::Moment { at, time_only, base } => Value::Moment { at: at.checked_add(span).map_err(fail)?, time_only, base },
            other => other,
        });
    }
    let duration = SignedDuration::try_from_secs_f64(q.si * sign).map_err(fail)?;
    let (at, time_only, base) = match v {
        Value::Date(d) => (d.to_zoned(TimeZone::system()).map_err(fail)?, false, None),
        Value::Moment { at, time_only, base } => (at, time_only, base),
        other => return Ok(other),
    };
    Ok(Value::Moment { at: at.checked_add(duration).map_err(fail)?, time_only, base })
}

fn factorial(n: f64) -> Result<f64, CalcError> {
    if n < 0.0 || n.fract() != 0.0 || n > 170.0 {
        return err(t!("factorial needs a whole number from 0 to 170"));
    }
    Ok((1..=n as u64).map(|i| i as f64).product())
}

fn call(name: &str, args: &[Expr], env: &Env) -> Result<Value, CalcError> {
    // `a(b+c)` with a variable `a` means multiplication.
    if let (Some(v), [arg]) = (env.vars.get(name), args) {
        return binary('*', v.clone(), eval(arg, env)?);
    }
    let values: Vec<Value> = args.iter().map(|a| eval(a, env)).collect::<Result<_, _>>()?;
    if let Some(v) = date_function(name, &values)? {
        return Ok(v);
    }
    let qs: Vec<Quantity> = values.into_iter().map(|v| num(v, &format!("{name}()"))).collect::<Result<_, _>>()?;
    let one = || -> Result<&Quantity, CalcError> {
        match qs.as_slice() {
            [x] => Ok(x),
            _ => err(t!("{}() takes one value", name)),
        }
    };
    // A function of the displayed number that keeps the unit.
    let keep = |f: fn(f64) -> f64| -> Result<Value, CalcError> {
        let x = one()?;
        Ok(Value::Num(x.with_number(f(x.number()))))
    };
    let pure = |f: fn(f64) -> f64| -> Result<Value, CalcError> {
        let x = one()?;
        if x.dim != units::NONE {
            return err(t!("{}() needs a plain number", name));
        }
        Ok(Value::plain(f(x.si)))
    };
    let trig = |f: fn(f64) -> f64| -> Result<Value, CalcError> {
        let x = one()?;
        if !units::is_angle(x) {
            return err(t!("{}() needs an angle or a plain number", name));
        }
        Ok(Value::plain(f(x.si)))
    };
    let list = || -> Result<&[Quantity], CalcError> {
        if qs.is_empty() {
            err(t!("{}() needs at least one value", name))
        } else {
            Ok(&qs)
        }
    };
    let pick = |greater: bool| -> Result<Value, CalcError> {
        let l = list()?;
        let mut best = l[0].clone();
        for q in &l[1..] {
            if q.dim != best.dim {
                return err(t!("{}() needs values of the same kind", name));
            }
            if if greater { q.si > best.si } else { q.si < best.si } {
                best = q.clone();
            }
        }
        Ok(Value::Num(best))
    };
    let total = || -> Result<Quantity, CalcError> {
        let l = list()?;
        l[1..].iter().try_fold(l[0].clone(), |acc, q| acc.add(q, false).map_err(CalcError::from))
    };
    match name {
        "sqrt" => {
            let x = one()?;
            if x.si < 0.0 {
                return err(t!("square root of a negative number"));
            }
            Ok(Value::Num(x.pow(0.5)?))
        }
        "abs" => keep(f64::abs),
        "floor" => keep(f64::floor),
        "ceil" => keep(f64::ceil),
        "trunc" | "int" => keep(f64::trunc),
        "round" => match qs.as_slice() {
            [x] => Ok(Value::Num(x.with_number(x.number().round()))),
            [x, d] if d.is_plain() => {
                let f = 10f64.powi(d.si as i32);
                Ok(Value::Num(x.with_number((x.number() * f).round() / f)))
            }
            _ => err(t!("round() takes a value and optional decimals")),
        },
        "sin" => trig(f64::sin),
        "cos" => trig(f64::cos),
        "tan" => trig(f64::tan),
        "asin" => pure(f64::asin),
        "acos" => pure(f64::acos),
        "atan" => pure(f64::atan),
        "rad" => pure(f64::to_radians),
        "deg" => {
            let x = one()?;
            if !units::is_angle(x) {
                return err(t!("{}() needs an angle or a plain number", name));
            }
            Ok(Value::plain(x.si.to_degrees()))
        }
        "ln" => pure(f64::ln),
        "log" => pure(f64::log10),
        "log2" => pure(f64::log2),
        "exp" => pure(f64::exp),
        "min" => pick(false),
        "max" => pick(true),
        "sum" => Ok(Value::Num(total()?)),
        "avg" => {
            let sum = total()?;
            Ok(Value::Num(sum.mul(&Quantity::plain(qs.len() as f64), -1)))
        }
        "qs" | "digitsum" => pure(digit_sum),
        "iqs" | "digitroot" => pure(|x| {
            let mut v = digit_sum(x);
            while v >= 10.0 {
                v = digit_sum(v);
            }
            v
        }),
        _ => err(t!("unknown function \"{}\"", name)),
    }
}

/// Date functions; `None` if `name` is none.
fn date_function(name: &str, values: &[Value]) -> Result<Option<Value>, CalcError> {
    let date_arg = || -> Result<Date, CalcError> {
        match values {
            [Value::Date(d)] => Ok(*d),
            [Value::Moment { at, .. }] => Ok(at.date()),
            _ => err(t!("{}() needs a date", name)),
        }
    };
    let ymd = || -> Result<(i64, i64, i64), CalcError> {
        match values {
            [y, m, d] => {
                let whole = |v: &Value| -> Result<i64, CalcError> {
                    let x = plain(v.clone(), name)?;
                    if x.fract() != 0.0 {
                        return err(t!("{}() needs whole numbers", name));
                    }
                    Ok(x as i64)
                };
                Ok((whole(y)?, whole(m)?, whole(d)?))
            }
            _ => err(t!("{}() takes year, month and day", name)),
        }
    };
    let gregorian = |rd: i64| -> Result<Option<Value>, CalcError> { Ok(Some(Value::Date(calendar::from_fixed(rd)?))) };
    Ok(Some(match name {
        "date" => {
            let (y, m, d) = ymd()?;
            Value::Date(calendar::date(y, m, d)?)
        }
        "weekday" => Value::Text(calendar::weekday_name(date_arg()?).to_owned()),
        "week" => Value::plain(date_arg()?.iso_week_date().week() as f64),
        "dayofyear" => Value::plain(date_arg()?.day_of_year() as f64),
        "year" => Value::plain(date_arg()?.year() as f64),
        "month" => Value::plain(date_arg()?.month() as f64),
        "day" => Value::plain(date_arg()?.day() as f64),
        "unix" => match values {
            [v] => moment_from_unix(plain(v.clone(), name)?)?,
            _ => return err(t!("{}() takes one value", name)),
        },
        "jd" => match values {
            [v] => Value::Date(calendar::from_julian_day_number(plain(v.clone(), name)?.floor() as i64)?),
            _ => return err(t!("{}() takes one value", name)),
        },
        "julian" => {
            let (y, m, d) = ymd()?;
            return gregorian(calendar::fixed_from_julian(y, m, d));
        }
        "hebrew" => {
            let (y, m, d) = ymd()?;
            return gregorian(calendar::fixed_from_hebrew(y, m, d));
        }
        "hijri" | "islamic" => {
            let (y, m, d) = ymd()?;
            return gregorian(calendar::fixed_from_islamic(y, m, d));
        }
        _ => return Ok(None),
    }))
}

/// Sum of all digits of the number as displayed (sign and point ignored).
fn digit_sum(x: f64) -> f64 {
    format_number(x).chars().filter_map(|c| c.to_digit(10)).sum::<u32>() as f64
}

#[cfg(test)]
mod tests {
    use super::*;
    use LineResult::*;

    fn run(text: &str) -> Vec<LineResult> {
        evaluate(text)
    }

    fn value(expr: &str) -> f64 {
        match run(&format!("x = {expr}"))[0] {
            Value(v) => v,
            Empty => expr.trim().parse().unwrap(),
            ref other => panic!("{expr}: {other:?}"),
        }
    }

    fn shown(expr: &str) -> String {
        match run(expr).pop().unwrap() {
            Shown(s) => s,
            Value(v) => format_number(v),
            other => panic!("{expr}: {other:?}"),
        }
    }

    #[test]
    fn arithmetic_and_precedence() {
        assert_eq!(value("1 + 2 * 3"), 7.0);
        assert_eq!(value("(1 + 2) * 3"), 9.0);
        assert_eq!(value("2 ^ 3 ^ 2"), 512.0);
        assert_eq!(value("-2 ^ 2"), -4.0);
        assert_eq!(value("10 % 4"), 2.0);
        assert_eq!(value("7 / 2"), 3.5);
        assert_eq!(value("2 ** 10"), 1024.0);
        assert_eq!(value("5!"), 120.0);
        assert_eq!(value("6 × 7 ÷ 2"), 21.0);
        assert_eq!(value("[2 + 3] * 2"), 10.0);
        assert_eq!(value("12 : 4"), 3.0);
        assert_eq!(value("2e3 + 0x10 + 0b11"), 2019.0);
    }

    #[test]
    fn variables_and_assignments() {
        let r = run("a = 5\nb = 10\nd = a+b\nd * 2\n2a + 3(b - a)");
        assert_eq!(r, vec![Empty, Empty, Value(15.0), Value(30.0), Value(25.0)]);
    }

    #[test]
    fn redefinition_applies_from_that_line() {
        let r = run("a = 1\na + 1\na = 10\na + 1");
        assert_eq!(r[1], Value(2.0));
        assert_eq!(r[3], Value(11.0));
    }

    #[test]
    fn variable_followed_by_parenthesis_multiplies() {
        assert_eq!(run("a = 3\na(2+1)")[1], Value(9.0));
    }

    #[test]
    fn errors() {
        assert_eq!(run("x = 1/0")[0], Error("division by zero".into()));
        assert_eq!(run("a = 1\nb = a + c")[1], Error("unknown variable \"c\"".into()));
        assert!(matches!(run("x = (1 + 2")[0], Error(_)));
        assert!(matches!(run("x = sqrt(-1)")[0], Error(_)));
        assert!(matches!(run("x = 1 +")[0], Error(_)));
    }

    #[test]
    fn text_and_comments_are_ignored() {
        let r = run("Stage 1: count windows\nstep-by-step\n# comment\nhello\n\na = 2 # two\na * 3 // six");
        assert_eq!(r, vec![Empty, Empty, Empty, Empty, Empty, Empty, Value(6.0)]);
        // Units and conversion words alone are text as well.
        assert_eq!(run("km/h\ns-t\nTotal in EUR\n5 min to go\n20 cm\ntoday is nice"), vec![Empty; 6]);
    }

    #[test]
    fn typos_next_to_known_values_are_reported() {
        assert!(matches!(run("a = 1\na + bb")[1], Error(_)));
        assert!(matches!(run("x + 1")[0], Error(_)));
    }

    #[test]
    fn functions() {
        assert_eq!(value("sqrt(16) + abs(-2)"), 6.0);
        assert_eq!(value("round(2.71828, 2)"), 2.72);
        assert_eq!(value("min(4, 2, 8) + max(1, 9)"), 11.0);
        assert_eq!(value("qs(1234)"), 10.0);
        assert_eq!(value("iqs(98765)"), 8.0);
        assert_eq!(value("round(cos(rad(60)), 6)"), 0.5);
        assert_eq!(value("floor(pi * 100)"), 314.0);
    }

    #[test]
    fn number_formatting() {
        assert_eq!(format_number(0.1 + 0.2), "0.3");
        assert_eq!(format_number(15.0), "15");
        assert_eq!(format_number(-2.5), "-2.5");
        assert_eq!(format_number(1.0 / 3.0), "0.3333333333");
        assert_eq!(format_number(-0.0), "0");
    }

    #[test]
    fn templates_fill_letters() {
        let r = run("a = 5\nb = 2\nc = 1\nd = 2\ne = 3\nf = 4\ng = 5\n> Nab° cd.efg");
        assert_eq!(r[7], Text("N52° 12.345".into()));
    }

    #[test]
    fn templates_with_expressions_and_formats() {
        let r = run("a = 7\nb = 3\n> N 52° {a*b:2}.{a*100+b:3} E 013° {b:2}.{a*11:3}");
        assert_eq!(r[2], Text("N 52° 21.703 E 013° 03.077".into()));
        let r = run("a = 7\n> {a:3} {a/4:.2} {a/4:2.1} {-a:3}");
        assert_eq!(r[1], Text("007 1.75 01.8 -007".into()));
        let r = run("w = 3 m\n> {w to ft:.1} / {w to cm}");
        assert_eq!(r[1], Text("9.8 ft / 300 cm".into()));
    }

    /// The examples of data/help.md.
    #[test]
    fn help_examples() {
        let r = run("a = 5\nb = 10\nd = a + b        # = 15\nd * 2            # = 30\n(a + b) / 4      # = 3.75");
        assert_eq!(&r[2..], &[Value(15.0), Value(30.0), Value(3.75)]);
        assert_eq!(value("17 % 5"), 2.0);
        assert_eq!(value("2 ^ 10"), 1024.0);
        assert_eq!(value("round(sin(rad(30)), 10)"), 0.5);
        let r = run("a = 7\nb = 3\n> N 52° {a*b}.{a*100+b} E 013° {b*4}.{(a+b)*12}");
        assert_eq!(r[2], Text("N 52° 21.703 E 013° 12.120".into()));
        let r = run("x = 7.256\n> {x:3} {x:.2} {x:2.1}");
        assert_eq!(r[1], Text("007.256 7.26 07.3".into()));
        // Units
        assert_eq!(shown("a = 3 m + 20 cm"), "3.2 m");
        assert_eq!(shown("a = 3 m + 20 cm\na to ft"), "10.4987 ft");
        assert_eq!(shown("speed = 120 km / 1.5 h"), "80 km/h");
        assert_eq!(shown("speed = 120 km / 1.5 h\nspeed in m/s"), "22.2222 m/s");
        assert_eq!(shown("100 °F to °C"), "37.7778 °C");
        assert_eq!(shown("5 km to mi"), "3.1069 mi");
        assert_eq!(shown("2 h 30 min to min"), "150 min");
        assert_eq!(shown("100 min to h min"), "1 h 40 min");
        assert_eq!(shown("5 ft 3 in to cm"), "160.02 cm");
        assert_eq!(shown("6 L/100km to mpg"), "39.2024 mpg");
        assert_eq!(shown("1 GiB to MB"), "1073.74 MB");
        assert_eq!(shown("unit floor = 2.8 m\n5 floor to m"), "14 m");
        assert_eq!(shown("unit dozen = 12\n3 dozen + 1"), "37");
        assert_eq!(shown("2 kW * 3 h to kWh"), "6 kWh");
        assert_eq!(shown("255 to hex"), "0xFF");
        assert_eq!(shown("2026 to roman"), "MMXXVI");
        assert_eq!(shown("sin(90°)"), "1");
        assert_eq!(shown("1.5 m2 * 2"), "3 m²");
        // Dates
        assert_eq!(shown("2026-01-31 + 1 month"), "Sat 2026-02-28");
        assert_eq!(shown("24.12.2026 to weekday"), "Thursday");
        assert!(matches!(run("now in New York")[0], Shown(_)));
        assert!(matches!(run("2026-12-24 - today")[0], Shown(_)));
        assert!(matches!(run("1700000000 to date")[0], Shown(_)));
        assert!(matches!(run("heute + 6 weeks")[0], Shown(_)));
    }

    #[test]
    fn unit_errors() {
        assert_eq!(run("5 kg + 3 m")[0], Error("cannot add mass and length".into()));
        assert!(matches!(run("5 kg to m")[0], Error(_)));
    }

    #[test]
    fn temperatures() {
        assert_eq!(shown("20 °C to °F"), "68 °F");
        assert_eq!(shown("-40 °C to °F"), "-40 °F");
        assert_eq!(shown("0 K to °C"), "-273.15 °C");
        assert_eq!(shown("25 °C - 20 °C"), "5 °C");
        assert_eq!(shown("25 °C - 20 °C to K"), "5 K");
        assert_eq!(shown("20 °C + 5 K"), "25 °C");
    }

    #[test]
    fn currencies() {
        crate::rates::set_for_tests(&[("USD", 1.08), ("GBP", 0.86)]);
        assert_eq!(shown("250 EUR in USD"), "270.00 USD");
        assert_eq!(shown("$100 to €"), "92.59 EUR");
        assert_eq!(shown("10 EUR + 10 USD"), "19.26 EUR");
        assert!(matches!(run("5 JPY to EUR")[0], Error(_)));
    }

    #[test]
    fn dates() {
        assert_eq!(shown("2026-12-24 - 2026-11-14"), "40 d");
        assert_eq!(shown("24.12.2026 - 14.11.2026"), "40 d");
        assert_eq!(shown("2026-01-31 + 1 month to iso"), "2026-02-28");
        assert_eq!(shown("2026-11-14 + 6 weeks to iso"), "2026-12-26");
        assert_eq!(shown("2026-12-24 to dayofyear"), "358");
        assert_eq!(shown("2026-12-24 to jd"), "2461399");
        assert_eq!(shown("1945-11-12 to julian"), "30 October 1945 (Julian)");
        assert_eq!(shown("2024-10-03 to hebrew"), "1 Tishrei 5785 (Hebrew)");
        assert_eq!(shown("2026-12-28 to week"), "week 53 of 2026");
        assert_eq!(shown("date(2026, 12, 24) to weekday"), "Thursday");
        assert_eq!(shown("julian(1945, 10, 30) to iso"), "1945-11-12");
        assert_eq!(shown("unix(0) to unix"), "0");
        assert_eq!(shown("2026-03-01 00:00 UTC to unix"), "1772323200");
        assert_eq!(shown("2026-12-24 18:00 Berlin - 2026-12-24 12:00 Berlin"), "6 h");
        assert_eq!(shown("2026-12-24 18:00 UTC - 2026-12-20 06:30 UTC"), "4 d 11 h 30 min");
        assert!(matches!(run("x = 2026-02-30")[0], Error(_)));
    }

    #[test]
    fn time_zones() {
        assert_eq!(shown("2026-07-01 14:00 Berlin to Tokyo"), "Wed 2026-07-01 21:00 JST");
        assert_eq!(shown("2026-01-15 09:30 New York to Europe/Berlin"), "Thu 2026-01-15 15:30 CET");
        assert_eq!(shown("2026-01-15 23:00 UTC to UTC+2"), "Fri 2026-01-16 01:00 +02");
        let r = shown("22:00 Berlin to Tokyo");
        assert!(r == "06:00 JST (+1 d)" || r == "05:00 JST (+1 d)", "{r}");
    }
}
