//! Line-based calculator for "Calculation" tabs.
//!
//! Every line is evaluated top to bottom:
//! - `name = expression` defines a variable
//! - `expression` shows its value
//! - `> template` fills in single-letter variables and `{expression}` parts
//! - `#` or `//` start a comment; lines that are no calculation are ignored
//!
//! See `data/help.md` for the user documentation.

use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum LineResult {
    /// Nothing to show (text, comment, empty line, plain `a = 5`).
    Empty,
    Value(f64),
    /// A filled-in template.
    Text(String),
    Error(String),
}

impl LineResult {
    /// The result as plain text (what gets copied).
    pub fn display(&self) -> Option<String> {
        match self {
            LineResult::Value(v) => Some(format_number(*v)),
            LineResult::Text(t) => Some(t.clone()),
            LineResult::Error(_) | LineResult::Empty => None,
        }
    }
}

type Vars = HashMap<String, f64>;

/// Evaluates all lines of `text`; one result per line (split at `\n`).
pub fn evaluate(text: &str) -> Vec<LineResult> {
    let mut vars = Vars::new();
    text.split('\n')
        .map(|line| eval_line(line.trim_end_matches('\r'), &mut vars))
        .collect()
}

fn eval_line(line: &str, vars: &mut Vars) -> LineResult {
    let trimmed = line.trim();
    if let Some(template) = trimmed.strip_prefix('>') {
        return fill_template(template.strip_prefix(' ').unwrap_or(template), vars);
    }
    let code = strip_comment(trimmed).trim();
    if code.is_empty() {
        return LineResult::Empty;
    }
    if let Some((name, expr)) = split_assignment(code) {
        return match parse(expr).map_err(CalcError::from).and_then(|e| eval(&e, vars)) {
            Ok(v) => {
                vars.insert(name.to_owned(), v);
                // `a = 5` would only repeat itself.
                if is_plain_number(expr) {
                    LineResult::Empty
                } else {
                    LineResult::Value(v)
                }
            }
            Err(e) => LineResult::Error(e.message),
        };
    }
    // Plain expression; anything that does not parse is text.
    let Ok(expr) = parse(code) else { return LineResult::Empty };
    match &expr {
        Expr::Num(_) => return LineResult::Empty,
        Expr::Var(name) if !vars.contains_key(name) => return LineResult::Empty,
        _ => {}
    }
    match eval(&expr, vars) {
        Ok(v) => LineResult::Value(v),
        // Words joined by operators ("step-by-step") are text, not typos.
        Err(e) if e.unknown_variable && !mentions_known(&expr, vars) => LineResult::Empty,
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

fn is_plain_number(expr: &str) -> bool {
    matches!(parse(expr), Ok(Expr::Num(_)) | Ok(Expr::Neg(_)))
        && expr.trim_start_matches(['-', '+', ' ']).chars().all(|c| c.is_ascii_digit() || c == '.')
}

fn mentions_known(expr: &Expr, vars: &Vars) -> bool {
    match expr {
        Expr::Num(_) => true,
        Expr::Var(n) => vars.contains_key(n) || n == "pi",
        Expr::Neg(e) | Expr::Fact(e) => mentions_known(e, vars),
        Expr::Bin(_, a, b) => mentions_known(a, vars) || mentions_known(b, vars),
        Expr::Call(_, args) => !args.is_empty(),
    }
}

// ===== templates ===========================================================

/// Replaces single letters that are defined variables and `{expr[:fmt]}`.
/// `\x` keeps the character `x` literally.
fn fill_template(template: &str, vars: &Vars) -> LineResult {
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
                    Some((e, s)) => (e, Some(s.trim())),
                    None => (inner.as_str(), None),
                };
                let value = match parse(expr).map_err(CalcError::from).and_then(|e| eval(&e, vars)) {
                    Ok(v) => v,
                    Err(e) => return LineResult::Error(format!("{{{}}}: {}", inner.trim(), e.message)),
                };
                match apply_format(value, spec) {
                    Ok(s) => out.push_str(&s),
                    Err(e) => return LineResult::Error(e),
                }
            }
            c if c.is_alphabetic() => match vars.get(c.encode_utf8(&mut [0; 4]) as &str) {
                Some(v) => out.push_str(&format_number(*v)),
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

// ===== expressions =========================================================

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Num(f64),
    Ident(String),
    Op(char),
    LParen,
    RParen,
    Comma,
}

#[derive(Debug, Clone, PartialEq)]
enum Expr {
    Num(f64),
    Var(String),
    Neg(Box<Expr>),
    Fact(Box<Expr>),
    Bin(char, Box<Expr>, Box<Expr>),
    Call(String, Vec<Expr>),
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

fn tokenize(s: &str) -> Result<Vec<Tok>, String> {
    let mut toks = Vec::new();
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        match c {
            c if c.is_whitespace() => i += 1,
            c if c.is_ascii_digit() || (c == '.' && chars.get(i + 1).is_some_and(|d| d.is_ascii_digit())) => {
                let start = i;
                while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
                    i += 1;
                }
                let text: String = chars[start..i].iter().collect();
                let n = text.parse::<f64>().map_err(|_| t!("invalid number \"{}\"", text))?;
                toks.push(Tok::Num(n));
            }
            c if c.is_alphabetic() || c == '_' => {
                let start = i;
                while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                    i += 1;
                }
                toks.push(Tok::Ident(chars[start..i].iter().collect()));
            }
            '*' if chars.get(i + 1) == Some(&'*') => {
                toks.push(Tok::Op('^'));
                i += 2;
            }
            '+' | '-' | '*' | '/' | '%' | '^' | '!' => {
                toks.push(Tok::Op(c));
                i += 1;
            }
            '×' | '·' => {
                toks.push(Tok::Op('*'));
                i += 1;
            }
            '÷' | ':' => {
                toks.push(Tok::Op('/'));
                i += 1;
            }
            '−' => {
                toks.push(Tok::Op('-'));
                i += 1;
            }
            '(' | '[' => {
                toks.push(Tok::LParen);
                i += 1;
            }
            ')' | ']' => {
                toks.push(Tok::RParen);
                i += 1;
            }
            ',' | ';' => {
                toks.push(Tok::Comma);
                i += 1;
            }
            c => return Err(t!("unexpected \"{}\"", c)),
        }
    }
    Ok(toks)
}

fn parse(s: &str) -> Result<Expr, String> {
    let toks = tokenize(s)?;
    if toks.is_empty() {
        return Err(t!("empty expression").into());
    }
    let mut p = Parser { toks, pos: 0 };
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
    }
}

struct Parser {
    toks: Vec<Tok>,
    pos: usize,
}

impl Parser {
    fn peek(&self) -> Option<&Tok> {
        self.toks.get(self.pos)
    }

    fn next(&mut self) -> Option<Tok> {
        let t = self.toks.get(self.pos).cloned();
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

    // term := unary (('*' | '/' | '%' | implicit) unary)*
    fn term(&mut self) -> Result<Expr, String> {
        let mut left = self.unary()?;
        loop {
            if let Some(op) = self.eat_op(&['*', '/', '%']) {
                left = Expr::Bin(op, Box::new(left), Box::new(self.unary()?));
            } else if self.implicit_multiplication() {
                left = Expr::Bin('*', Box::new(left), Box::new(self.unary()?));
            } else {
                return Ok(left);
            }
        }
    }

    /// `2a`, `2(a+b)`, `(a+b)(c+d)`, `(a)b` – after a number or `)`.
    fn implicit_multiplication(&self) -> bool {
        let prev = self.pos.checked_sub(1).and_then(|i| self.toks.get(i));
        matches!(prev, Some(Tok::Num(_)) | Some(Tok::RParen))
            && matches!(self.peek(), Some(Tok::Ident(_)) | Some(Tok::LParen) | Some(Tok::Num(_)))
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
            Some(t) => Err(t!("unexpected {}", describe(&t))),
            None => Err(t!("incomplete expression").into()),
        }
    }
}

fn eval(expr: &Expr, vars: &Vars) -> Result<f64, CalcError> {
    let v = match expr {
        Expr::Num(n) => *n,
        Expr::Var(name) => match vars.get(name) {
            Some(v) => *v,
            None if name == "pi" => std::f64::consts::PI,
            None => {
                return Err(CalcError { message: t!("unknown variable \"{}\"", name), unknown_variable: true })
            }
        },
        Expr::Neg(e) => -eval(e, vars)?,
        Expr::Fact(e) => factorial(eval(e, vars)?)?,
        Expr::Bin(op, a, b) => {
            let (a, b) = (eval(a, vars)?, eval(b, vars)?);
            match op {
                '+' => a + b,
                '-' => a - b,
                '*' => a * b,
                '/' if b == 0.0 => return err(t!("division by zero")),
                '/' => a / b,
                '%' if b == 0.0 => return err(t!("division by zero")),
                '%' => a % b,
                '^' => a.powf(b),
                _ => return err(t!("unknown operator \"{}\"", op)),
            }
        }
        Expr::Call(name, args) => call(name, args, vars)?,
    };
    if v.is_finite() {
        Ok(v)
    } else {
        err(t!("result is not a finite number"))
    }
}

fn factorial(n: f64) -> Result<f64, CalcError> {
    if n < 0.0 || n.fract() != 0.0 || n > 170.0 {
        return err(t!("factorial needs a whole number from 0 to 170"));
    }
    Ok((1..=n as u64).map(|i| i as f64).product())
}

fn call(name: &str, args: &[Expr], vars: &Vars) -> Result<f64, CalcError> {
    // `a(b+c)` with a variable `a` means multiplication.
    if let (Some(v), [arg]) = (vars.get(name), args) {
        return Ok(v * eval(arg, vars)?);
    }
    let values: Vec<f64> = args.iter().map(|a| eval(a, vars)).collect::<Result<_, _>>()?;
    let one = |f: fn(f64) -> f64| -> Result<f64, CalcError> {
        match values.as_slice() {
            [x] => Ok(f(*x)),
            _ => err(t!("{}() takes one value", name)),
        }
    };
    let list = || -> Result<&[f64], CalcError> {
        if values.is_empty() {
            err(t!("{}() needs at least one value", name))
        } else {
            Ok(&values)
        }
    };
    match name {
        "sqrt" => match values.as_slice() {
            [x] if *x < 0.0 => err(t!("square root of a negative number")),
            _ => one(f64::sqrt),
        },
        "abs" => one(f64::abs),
        "floor" => one(f64::floor),
        "ceil" => one(f64::ceil),
        "trunc" | "int" => one(f64::trunc),
        "round" => match values.as_slice() {
            [x] => Ok(x.round()),
            [x, d] => {
                let f = 10f64.powi(*d as i32);
                Ok((x * f).round() / f)
            }
            _ => err(t!("round() takes a value and optional decimals")),
        },
        "sin" => one(f64::sin),
        "cos" => one(f64::cos),
        "tan" => one(f64::tan),
        "asin" => one(f64::asin),
        "acos" => one(f64::acos),
        "atan" => one(f64::atan),
        "rad" => one(f64::to_radians),
        "deg" => one(f64::to_degrees),
        "ln" => one(f64::ln),
        "log" => one(f64::log10),
        "log2" => one(f64::log2),
        "exp" => one(f64::exp),
        "min" => Ok(list()?.iter().copied().fold(f64::INFINITY, f64::min)),
        "max" => Ok(list()?.iter().copied().fold(f64::NEG_INFINITY, f64::max)),
        "sum" => Ok(list()?.iter().sum()),
        "avg" => Ok(list()?.iter().sum::<f64>() / values.len() as f64),
        "qs" | "digitsum" => one(digit_sum),
        "iqs" | "digitroot" => one(|x| {
            let mut v = digit_sum(x);
            while v >= 10.0 {
                v = digit_sum(v);
            }
            v
        }),
        _ => err(t!("unknown function \"{}\"", name)),
    }
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
    }

    #[test]
    fn templates_keep_unknown_and_escaped_letters() {
        let r = run("N = 1\na = 4\n> \\Na° N{a+1}");
        assert_eq!(r[2], Text("N4° 15".into()));
        assert!(matches!(run("> {x+1}")[0], Error(_)));
        assert!(matches!(run("a=1\n> {a:x}")[1], Error(_)));
    }
}
