//! MongoDB shell (mongosh) and JSON5 support: a tolerant formatter and the
//! conversion between shell syntax and MongoDB Extended JSON v2.
//! Everything here is pure and independent of the UI.

use base64::Engine;

/// GtkSourceView language id (`data/mongodb.lang`).
pub const LANGUAGE: &str = "mongodb";

/// Maximum line width before a document or array is broken into lines.
const WIDTH: usize = 80;
const INDENT: &str = "    ";

// ===== lexer =============================================================

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Open(char),
    Close(char),
    Comma,
    Colon,
    Semi,
    Dot,
    LineComment,
    BlockComment,
    Str,
    Num,
    Word,
    Regex,
    Op,
}

#[derive(Clone, Debug)]
struct Tok<'a> {
    kind: Kind,
    text: &'a str,
    /// Line breaks in the whitespace before the token (at most 2).
    nl: u8,
    /// Whether whitespace precedes the token.
    space: bool,
    line: usize,
}

impl Tok<'_> {
    fn is_comment(&self) -> bool {
        matches!(self.kind, Kind::LineComment | Kind::BlockComment)
    }
}

/// Multi-character operators, longest first.
const OPERATORS: &[&str] = &[
    ">>>=", "===", "!==", "**=", "<<=", ">>=", ">>>", "<=>", "?->", "??=", "=>", "->", "==", "!=", "<=", ">=", "&&", "||", "??", "++", "--",
    "+=", "-=", "*=", "/=", "%=", "&=", "|=", "^=", "**", "<<", ">>",
];

/// Words after which a `/` starts a regular expression rather than a division.
const REGEX_AFTER_WORDS: &[&str] = &["return", "typeof", "case", "do", "else", "in", "of", "new", "delete", "void", "throw"];

fn is_ident_start(c: char) -> bool {
    c.is_alphabetic() || c == '_' || c == '$'
}

fn is_ident_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_' || c == '$'
}

/// Splits mongosh / JSON5 source into tokens. With `php`, `#` starts a comment,
/// `::` is an operator, strings may span lines and there are no regex literals.
fn lex(src: &str, php: bool) -> Result<Vec<Tok<'_>>, String> {
    let mut toks: Vec<Tok> = Vec::new();
    let mut chars = src.char_indices().peekable();
    let (mut line, mut nl, mut space) = (1usize, 0u8, false);
    let peek_at = |i: usize| src[i..].chars().next();

    while let Some(&(start, c)) = chars.peek() {
        if c == '\n' {
            line += 1;
            nl = (nl + 1).min(2);
            space = true;
            chars.next();
            continue;
        }
        if c.is_whitespace() || c == '\u{feff}' {
            space = true;
            chars.next();
            continue;
        }
        let tok_line = line;
        chars.next();
        let next = peek_at(start + c.len_utf8());
        let prev = toks.iter().rev().find(|t| !t.is_comment());
        let kind = match c {
            '{' | '[' | '(' => Kind::Open(c),
            '}' | ']' | ')' => Kind::Close(c),
            ',' => Kind::Comma,
            ':' if php && next == Some(':') => {
                chars.next();
                Kind::Op
            }
            ':' => Kind::Colon,
            ';' => Kind::Semi,
            '.' if !next.is_some_and(|n| n.is_ascii_digit()) => Kind::Dot,
            '"' | '\'' | '`' => {
                let mut escaped = false;
                loop {
                    let Some((_, ch)) = chars.next() else {
                        return Err(t!("Unterminated string in line {}", tok_line));
                    };
                    if ch == '\n' {
                        if c != '`' && !escaped && !php {
                            return Err(t!("Unterminated string in line {}", tok_line));
                        }
                        line += 1;
                    }
                    if escaped {
                        escaped = false;
                    } else if ch == '\\' {
                        escaped = true;
                    } else if ch == c {
                        break;
                    }
                }
                Kind::Str
            }
            '#' if php && next != Some('[') => {
                while chars.peek().is_some_and(|&(_, ch)| ch != '\n') {
                    chars.next();
                }
                Kind::LineComment
            }
            '/' if next == Some('/') => {
                while chars.peek().is_some_and(|&(_, ch)| ch != '\n') {
                    chars.next();
                }
                Kind::LineComment
            }
            '/' if next == Some('*') => {
                chars.next();
                let mut star = false;
                loop {
                    let Some((_, ch)) = chars.next() else {
                        return Err(t!("Unterminated comment in line {}", tok_line));
                    };
                    if ch == '\n' {
                        line += 1;
                    }
                    if star && ch == '/' {
                        break;
                    }
                    star = ch == '*';
                }
                Kind::BlockComment
            }
            '/' if !php && prev.is_none_or(|p| match p.kind {
                Kind::Open(_) | Kind::Comma | Kind::Colon | Kind::Semi | Kind::Op => true,
                Kind::Word => REGEX_AFTER_WORDS.contains(&p.text),
                _ => false,
            }) =>
            {
                let (mut escaped, mut class) = (false, false);
                loop {
                    let Some((_, ch)) = chars.next() else {
                        return Err(t!("Unterminated regular expression in line {}", tok_line));
                    };
                    match ch {
                        '\n' => return Err(t!("Unterminated regular expression in line {}", tok_line)),
                        _ if escaped => escaped = false,
                        '\\' => escaped = true,
                        '[' => class = true,
                        ']' => class = false,
                        '/' if !class => break,
                        _ => {}
                    }
                }
                while chars.peek().is_some_and(|&(_, ch)| ch.is_ascii_alphabetic()) {
                    chars.next();
                }
                Kind::Regex
            }
            c if c.is_ascii_digit() || c == '.' => {
                let mut last = c;
                while let Some(&(_, ch)) = chars.peek() {
                    let exponent_sign = matches!(ch, '+' | '-')
                        && matches!(last, 'e' | 'E')
                        && !src[start..].starts_with("0x")
                        && !src[start..].starts_with("0X");
                    if !(ch.is_ascii_alphanumeric() || ch == '.' || ch == '_' || exponent_sign) {
                        break;
                    }
                    last = ch;
                    chars.next();
                }
                Kind::Num
            }
            c if is_ident_start(c) => {
                while chars.peek().is_some_and(|&(_, ch)| is_ident_char(ch)) {
                    chars.next();
                }
                Kind::Word
            }
            _ => {
                if let Some(op) = OPERATORS.iter().find(|op| src[start..].starts_with(*op)) {
                    for _ in 1..op.len() {
                        chars.next();
                    }
                }
                Kind::Op
            }
        };
        let end = chars.peek().map_or(src.len(), |&(i, _)| i);
        toks.push(Tok { kind, text: &src[start..end], nl, space, line: tok_line });
        nl = 0;
        space = false;
    }
    Ok(toks)
}

// ===== bracket tree ======================================================

#[derive(Debug)]
enum Node<'a> {
    Tok(Tok<'a>),
    Group { open: Tok<'a>, items: Vec<Node<'a>>, close: Tok<'a> },
}

impl<'a> Node<'a> {
    fn first(&self) -> &Tok<'a> {
        match self {
            Node::Tok(t) => t,
            Node::Group { open, .. } => open,
        }
    }

    fn last(&self) -> &Tok<'a> {
        match self {
            Node::Tok(t) => t,
            Node::Group { close, .. } => close,
        }
    }
}

fn matching(open: char) -> char {
    match open {
        '{' => '}',
        '[' => ']',
        _ => ')',
    }
}

fn tree(toks: Vec<Tok<'_>>) -> Result<Vec<Node<'_>>, String> {
    let mut stack: Vec<(Tok, Vec<Node>)> = Vec::new();
    let mut top: Vec<Node> = Vec::new();
    for tok in toks {
        match tok.kind {
            Kind::Open(_) => stack.push((tok, std::mem::take(&mut top))),
            Kind::Close(c) => {
                let Some((open, parent)) = stack.pop() else {
                    return Err(t!("Unbalanced \"{}\" in line {}", c, tok.line));
                };
                let Kind::Open(o) = open.kind else { unreachable!() };
                if matching(o) != c {
                    return Err(t!("Unbalanced \"{}\" in line {}", c, tok.line));
                }
                let items = std::mem::replace(&mut top, parent);
                top.push(Node::Group { open, items, close: tok });
            }
            _ => top.push(Node::Tok(tok)),
        }
    }
    if let Some((open, _)) = stack.pop() {
        return Err(t!("Unbalanced \"{}\" in line {}", open.text, open.line));
    }
    Ok(top)
}

// ===== formatter =========================================================

#[derive(PartialEq, Eq)]
enum Gap {
    None,
    Space,
    Line,
}

/// What goes between two adjacent tokens (separators are handled by the caller).
fn gap(prev: &Tok, next: &Tok) -> Gap {
    use Kind::*;
    if prev.kind == LineComment {
        return Gap::Line;
    }
    if next.is_comment() || prev.kind == BlockComment {
        return if next.nl > 0 { Gap::Line } else { Gap::Space };
    }
    // Keep line breaks that may end a statement (automatic semicolon insertion).
    if next.nl > 0
        && matches!(prev.kind, Word | Num | Str | Regex | Close(_))
        && matches!(next.kind, Word | Num | Str | Regex | Open('{'))
    {
        return Gap::Line;
    }
    if matches!(next.kind, Comma | Colon | Semi | Dot | Close(_)) || matches!(prev.kind, Open(_) | Dot) {
        return Gap::None;
    }
    if matches!(prev.kind, Comma | Colon | Semi) {
        return Gap::Space;
    }
    if next.space {
        Gap::Space
    } else {
        Gap::None
    }
}

/// Whether the comma at `i` is a trailing one (only comments follow).
fn trailing_comma(nodes: &[Node], i: usize) -> bool {
    nodes[i + 1..].iter().all(|n| n.first().is_comment())
}

/// Arrays of several documents (e.g. pipelines) always get one element per line.
/// In PHP, documents are arrays with `=>` keys.
fn force_break(open: char, items: &[Node], php: bool) -> bool {
    if open != '[' {
        return false;
    }
    let is_document = |n: &&Node| match n {
        Node::Group { open, items, .. } if php => {
            open.kind == Kind::Open('[') && items.iter().any(|i| i.first().kind == Kind::Op && i.first().text == "=>")
        }
        n => n.first().kind == Kind::Open('{'),
    };
    let elements: Vec<&Node> = items.iter().filter(|n| n.first().kind != Kind::Comma).collect();
    elements.len() >= 2 && elements.iter().all(is_document)
}

/// Flat rendering of a group, or `None` if it can't stay on one line or gets longer than `limit`.
fn flat(open: &Tok, items: &[Node], close: &Tok, limit: usize, php: bool) -> Option<String> {
    let Kind::Open(o) = open.kind else { return None };
    if force_break(o, items, php) {
        return None;
    }
    let pad = o == '{' && !items.is_empty();
    let mut s = String::from(open.text);
    if pad {
        s.push(' ');
    }
    let mut prev: Option<&Tok> = None;
    for (i, node) in items.iter().enumerate() {
        let first = node.first();
        match first.kind {
            Kind::LineComment | Kind::BlockComment | Kind::Semi => return None,
            Kind::Comma => {
                if !trailing_comma(items, i) {
                    s.push(',');
                    prev = Some(first);
                }
                continue;
            }
            _ => {}
        }
        if let Some(p) = prev {
            match gap(p, first) {
                Gap::Line => return None,
                Gap::Space => s.push(' '),
                Gap::None => {}
            }
        }
        match node {
            Node::Tok(t) => s.push_str(t.text),
            Node::Group { open, items, close } => s.push_str(&flat(open, items, close, limit, php)?),
        }
        if s.len() > limit {
            return None;
        }
        prev = Some(node.last());
    }
    if pad {
        s.push(' ');
    }
    s.push_str(close.text);
    Some(s)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    /// Top level: `;` ends a line.
    Statements,
    /// Inside `{}` / `[]`: `,` and `;` end a line.
    Items,
    /// Inside `()`: everything stays on the line.
    Inline,
}

struct Printer {
    out: String,
    /// PHP arrays: `[` groups with `=>` are documents, broken arrays get a trailing comma.
    php: bool,
    /// True right after a line break and its indentation.
    fresh: bool,
}

impl Printer {
    fn push(&mut self, s: &str) {
        self.out.push_str(s);
        self.fresh = false;
    }

    fn newline(&mut self, level: usize, blank: bool) {
        let trimmed = self.out.trim_end_matches([' ', '\t']).len();
        self.out.truncate(trimmed);
        self.out.push('\n');
        if blank {
            self.out.push('\n');
        }
        for _ in 0..level {
            self.out.push_str(INDENT);
        }
        self.fresh = true;
    }

    fn column(&self) -> usize {
        self.out[self.out.rfind('\n').map_or(0, |i| i + 1)..].chars().count()
    }

    fn seq(&mut self, nodes: &[Node], level: usize, mode: Mode) {
        // Line breaks forced inside parentheses get one extra indentation level.
        let nl_level = level + usize::from(mode == Mode::Inline);
        let mut prev: Option<&Tok> = None;
        let mut i = 0;
        while i < nodes.len() {
            let node = &nodes[i];
            let first = node.first();
            let breaks = match first.kind {
                Kind::Comma if mode == Mode::Inline || trailing_comma(nodes, i) => {
                    if !trailing_comma(nodes, i) {
                        self.push(",");
                        prev = Some(first);
                    }
                    i += 1;
                    continue;
                }
                Kind::Comma => mode == Mode::Items,
                Kind::Semi => mode != Mode::Inline,
                _ => false,
            };
            if breaks {
                self.push(first.text);
                i += 1;
                // A comment on the same line stays behind the separator.
                if let Some(c) = nodes.get(i).filter(|n| n.first().is_comment() && n.first().nl == 0) {
                    self.push(" ");
                    self.push(c.first().text);
                    i += 1;
                }
                if let Some(n) = nodes.get(i) {
                    self.newline(nl_level, n.first().nl >= 2);
                }
                prev = None;
                continue;
            }
            if let Some(p) = prev {
                match gap(p, first) {
                    Gap::Line => self.newline(nl_level, first.nl >= 2),
                    Gap::Space if !self.fresh => self.push(" "),
                    _ => {}
                }
            }
            self.node(node, level);
            prev = Some(node.last());
            i += 1;
        }
    }

    fn node(&mut self, node: &Node, level: usize) {
        let Node::Group { open, items, close } = node else {
            self.push(node.first().text);
            return;
        };
        let Kind::Open(o) = open.kind else { return };
        if o == '(' {
            self.push(open.text);
            self.seq(items, level, Mode::Inline);
            if items.last().is_some_and(|n| n.first().kind == Kind::LineComment) {
                self.newline(level, false);
            }
            self.push(close.text);
            return;
        }
        let room = WIDTH.saturating_sub(self.column());
        if let Some(s) = flat(open, items, close, room, self.php) {
            if s.chars().count() <= room {
                self.push(&s);
                return;
            }
        }
        self.push(open.text);
        self.newline(level + 1, false);
        self.seq(items, level + 1, Mode::Items);
        if self.php && items.last().is_some_and(|n| !n.first().is_comment() && n.first().kind != Kind::Comma) {
            self.push(",");
        }
        self.newline(level, false);
        self.push(close.text);
    }
}

fn is_word_char(c: Option<char>) -> bool {
    c.is_some_and(is_ident_char)
}

fn minify(nodes: &[Node], out: &mut String) {
    let mut prev: Option<&Tok> = None;
    for (i, node) in nodes.iter().enumerate() {
        let first = node.first();
        if first.is_comment() || (first.kind == Kind::Comma && trailing_comma(nodes, i)) {
            continue;
        }
        if let Some(p) = prev {
            match gap(p, first) {
                Gap::Line => out.push('\n'),
                Gap::Space => {
                    let (a, b) = (p.text.chars().last(), first.text.chars().next());
                    let needed = (is_word_char(a) && is_word_char(b))
                        || (a == b && matches!(a, Some('+') | Some('-')))
                        || (first.kind == Kind::Regex && p.kind == Kind::Word);
                    if needed {
                        out.push(' ');
                    }
                }
                Gap::None => {}
            }
        }
        match node {
            Node::Tok(t) => out.push_str(t.text),
            Node::Group { open, items, close } => {
                out.push_str(open.text);
                minify(items, out);
                out.push_str(close.text);
            }
        }
        prev = Some(node.last());
    }
}

/// Re-formats mongosh / JSON5 text. Keys, quotes, numbers and comments are kept
/// as written; only whitespace changes. `pretty == false` minifies (and drops comments).
pub fn reformat(input: &str, pretty: bool) -> Result<String, String> {
    reformat_with(input, pretty, false)
}

fn reformat_with(input: &str, pretty: bool, php: bool) -> Result<String, String> {
    let nodes = tree(lex(input, php)?)?;
    if !pretty {
        let mut out = String::with_capacity(input.len());
        minify(&nodes, &mut out);
        return Ok(out);
    }
    let mut p = Printer { out: String::with_capacity(input.len() + input.len() / 4), php, fresh: true };
    p.seq(&nodes, 0, Mode::Statements);
    let trimmed = p.out.trim_end().len();
    p.out.truncate(trimmed);
    p.out.push('\n');
    Ok(p.out)
}

// ===== expressions =======================================================

#[derive(Clone, Debug, PartialEq)]
enum Expr {
    Obj(Vec<(String, Expr)>),
    Arr(Vec<Expr>),
    Str(String),
    /// Number literal as written (with sign), or `Infinity` / `NaN`.
    Num(String),
    Regex(String, String),
    /// Identifiers, including `true`, `false`, `null` and `undefined`.
    Ident(String),
    Member(Box<Expr>, String),
    Index(Box<Expr>, Box<Expr>),
    Call { callee: Box<Expr>, args: Vec<Expr>, new: bool },
    Decl(String, String, Box<Expr>),
}

impl Expr {
    fn call(name: &str, args: Vec<Expr>) -> Expr {
        Expr::Call { callee: Box::new(Expr::Ident(name.into())), args, new: false }
    }

    fn obj(entries: Vec<(&str, Expr)>) -> Expr {
        Expr::Obj(entries.into_iter().map(|(k, v)| (k.to_owned(), v)).collect())
    }

    fn str(s: impl Into<String>) -> Expr {
        Expr::Str(s.into())
    }

    fn is_value(&self) -> bool {
        matches!(self, Expr::Obj(_) | Expr::Arr(_))
    }
}

struct Statement {
    expr: Expr,
    semicolon: bool,
}

struct Parser<'a> {
    toks: Vec<Tok<'a>>,
    pos: usize,
}

fn unexpected(tok: Option<&Tok>) -> String {
    match tok {
        Some(t) => t!("Unexpected \"{}\" in line {}", t.text, t.line),
        None => t!("Unexpected end of input").into(),
    }
}

impl<'a> Parser<'a> {
    fn new(input: &'a str) -> Result<Self, String> {
        let toks = lex(input, false)?.into_iter().filter(|t| !t.is_comment()).collect();
        Ok(Parser { toks, pos: 0 })
    }

    fn peek(&self) -> Option<&Tok<'a>> {
        self.toks.get(self.pos)
    }

    fn next(&mut self) -> Option<Tok<'a>> {
        let t = self.toks.get(self.pos).cloned();
        self.pos += 1;
        t
    }

    fn eat(&mut self, kind: Kind) -> bool {
        if self.peek().is_some_and(|t| t.kind == kind) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    fn expect(&mut self, kind: Kind) -> Result<(), String> {
        if self.eat(kind) {
            Ok(())
        } else {
            Err(unexpected(self.peek()))
        }
    }

    fn program(&mut self) -> Result<Vec<Statement>, String> {
        let mut out = Vec::new();
        while self.peek().is_some() {
            if self.eat(Kind::Semi) {
                continue;
            }
            let expr = self.statement()?;
            out.push(Statement { expr, semicolon: self.eat(Kind::Semi) });
        }
        Ok(out)
    }

    fn statement(&mut self) -> Result<Expr, String> {
        if let Some(t) = self.peek().filter(|t| t.kind == Kind::Word && matches!(t.text, "const" | "let" | "var")) {
            let kind = t.text.to_owned();
            self.pos += 1;
            let name = match self.next() {
                Some(t) if t.kind == Kind::Word => t.text.to_owned(),
                t => return Err(unexpected(t.as_ref())),
            };
            match self.next() {
                Some(t) if t.kind == Kind::Op && t.text == "=" => {}
                t => return Err(unexpected(t.as_ref())),
            }
            return Ok(Expr::Decl(kind, name, Box::new(self.expr()?)));
        }
        self.expr()
    }

    fn expr(&mut self) -> Result<Expr, String> {
        let Some(t) = self.peek().cloned() else { return Err(unexpected(None)) };
        match (t.kind, t.text) {
            (Kind::Op, "-") | (Kind::Op, "+") => {
                self.pos += 1;
                match self.expr()? {
                    Expr::Num(n) if t.text == "+" => Ok(Expr::Num(n)),
                    Expr::Num(n) => Ok(Expr::Num(match n.strip_prefix('-') {
                        Some(rest) => rest.to_owned(),
                        None => format!("-{n}"),
                    })),
                    _ => Err(unexpected(Some(&t))),
                }
            }
            (Kind::Word, "new") => {
                self.pos += 1;
                match self.expr()? {
                    Expr::Call { callee, args, .. } => Ok(Expr::Call { callee, args, new: true }),
                    callee => Ok(Expr::Call { callee: Box::new(callee), args: Vec::new(), new: true }),
                }
            }
            _ => {
                let base = self.primary()?;
                self.postfix(base)
            }
        }
    }

    fn postfix(&mut self, mut base: Expr) -> Result<Expr, String> {
        loop {
            if self.eat(Kind::Dot) {
                match self.next() {
                    Some(t) if t.kind == Kind::Word => base = Expr::Member(Box::new(base), t.text.to_owned()),
                    t => return Err(unexpected(t.as_ref())),
                }
            } else if self.eat(Kind::Open('(')) {
                let args = self.list(')')?;
                base = Expr::Call { callee: Box::new(base), args, new: false };
            } else if self.eat(Kind::Open('[')) {
                let index = self.expr()?;
                self.expect(Kind::Close(']'))?;
                base = Expr::Index(Box::new(base), Box::new(index));
            } else {
                return Ok(base);
            }
        }
    }

    /// Comma separated expressions up to the closing bracket (trailing comma allowed).
    fn list(&mut self, close: char) -> Result<Vec<Expr>, String> {
        let mut out = Vec::new();
        loop {
            if self.eat(Kind::Close(close)) {
                return Ok(out);
            }
            out.push(self.expr()?);
            if !self.eat(Kind::Comma) {
                self.expect(Kind::Close(close))?;
                return Ok(out);
            }
        }
    }

    fn primary(&mut self) -> Result<Expr, String> {
        let Some(t) = self.next() else { return Err(unexpected(None)) };
        Ok(match t.kind {
            Kind::Open('{') => {
                let mut entries = Vec::new();
                loop {
                    if self.eat(Kind::Close('}')) {
                        break;
                    }
                    let key = match self.next() {
                        Some(k) if k.kind == Kind::Str => unquote(k.text).ok_or_else(|| unexpected(Some(&k)))?,
                        Some(k) if matches!(k.kind, Kind::Word | Kind::Num) => k.text.to_owned(),
                        k => return Err(unexpected(k.as_ref())),
                    };
                    self.expect(Kind::Colon)?;
                    entries.push((key, self.expr()?));
                    if !self.eat(Kind::Comma) {
                        self.expect(Kind::Close('}'))?;
                        break;
                    }
                }
                Expr::Obj(entries)
            }
            Kind::Open('[') => Expr::Arr(self.list(']')?),
            Kind::Open('(') => {
                let e = self.expr()?;
                self.expect(Kind::Close(')'))?;
                e
            }
            Kind::Str => Expr::Str(unquote(t.text).ok_or_else(|| unexpected(Some(&t)))?),
            Kind::Num => Expr::Num(t.text.to_owned()),
            Kind::Regex => {
                let end = t.text.rfind('/').unwrap_or(0);
                Expr::Regex(t.text[1..end].to_owned(), t.text[end + 1..].to_owned())
            }
            Kind::Word if matches!(t.text, "Infinity" | "NaN") => Expr::Num(t.text.to_owned()),
            Kind::Word => Expr::Ident(t.text.to_owned()),
            _ => return Err(unexpected(Some(&t))),
        })
    }
}

fn parse(input: &str) -> Result<Vec<Statement>, String> {
    Parser::new(input)?.program()
}

/// Decodes a JS string literal (with quotes). `None` for template literals with `${…}`.
fn unquote(lit: &str) -> Option<String> {
    let quote = lit.chars().next()?;
    let body = &lit[1..lit.len() - 1];
    if quote == '`' && body.contains("${") {
        return None;
    }
    let mut out = String::with_capacity(body.len());
    let mut chars = body.chars().peekable();
    let hex = |chars: &mut std::iter::Peekable<std::str::Chars>, n: usize| -> Option<u32> {
        let s: String = (0..n).filter_map(|_| chars.next()).collect();
        (s.len() == n).then(|| u32::from_str_radix(&s, 16).ok()).flatten()
    };
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next()? {
            'n' => out.push('\n'),
            't' => out.push('\t'),
            'r' => out.push('\r'),
            'b' => out.push('\u{8}'),
            'f' => out.push('\u{c}'),
            'v' => out.push('\u{b}'),
            '0' => out.push('\0'),
            '\n' => {}
            'x' => out.push(char::from_u32(hex(&mut chars, 2)?)?),
            'u' if chars.peek() == Some(&'{') => {
                chars.next();
                let s: String = chars.by_ref().take_while(|&c| c != '}').collect();
                out.push(char::from_u32(u32::from_str_radix(&s, 16).ok()?)?);
            }
            'u' => {
                let hi = hex(&mut chars, 4)?;
                if (0xd800..0xdc00).contains(&hi) && chars.peek() == Some(&'\\') {
                    chars.next();
                    chars.next();
                    let lo = hex(&mut chars, 4)?;
                    out.push(char::from_u32(0x10000 + ((hi - 0xd800) << 10) + (lo.checked_sub(0xdc00)?))?);
                } else {
                    out.push(char::from_u32(hi).unwrap_or('\u{fffd}'));
                }
            }
            other => out.push(other),
        }
    }
    Some(out)
}

/// Whether the text after leading comments looks like MongoDB shell code or a
/// JSON5 value. Used for language detection.
pub fn sniff(input: &str) -> bool {
    let mut body = input.trim_start();
    loop {
        if let Some(rest) = body.strip_prefix("//") {
            body = rest.split_once('\n').map_or("", |(_, r)| r).trim_start();
        } else if let Some(rest) = body.strip_prefix("/*") {
            body = rest.split_once("*/").map_or("", |(_, r)| r).trim_start();
        } else {
            break;
        }
    }
    if body.starts_with("db.") || body.starts_with("db[") {
        return true;
    }
    let declaration = ["const ", "let ", "var "].iter().any(|k| body.starts_with(k));
    if !(body.starts_with('{') || body.starts_with('[') || declaration) {
        return false;
    }
    parse(input).is_ok_and(|stmts| {
        !stmts.is_empty()
            && stmts.iter().all(|s| match &s.expr {
                Expr::Decl(_, _, v) => v.is_value(),
                e => e.is_value(),
            })
    })
}

// ===== printing ==========================================================

fn is_plain_key(k: &str) -> bool {
    let mut chars = k.chars();
    chars.next().is_some_and(is_ident_start) && chars.all(is_ident_char)
}

fn json_str(s: &str) -> String {
    serde_json::to_string(s).unwrap_or_default()
}

/// Prints an expression in mongosh syntax on one line.
fn shell(e: &Expr, out: &mut String) {
    let list = |items: &[Expr], out: &mut String| {
        for (i, item) in items.iter().enumerate() {
            if i > 0 {
                out.push_str(", ");
            }
            shell(item, out);
        }
    };
    match e {
        Expr::Obj(entries) => {
            out.push('{');
            for (i, (k, v)) in entries.iter().enumerate() {
                out.push_str(if i > 0 { ", " } else { " " });
                out.push_str(&if is_plain_key(k) { k.clone() } else { json_str(k) });
                out.push_str(": ");
                shell(v, out);
            }
            out.push_str(if entries.is_empty() { "}" } else { " }" });
        }
        Expr::Arr(items) => {
            out.push('[');
            list(items, out);
            out.push(']');
        }
        Expr::Str(s) => out.push_str(&json_str(s)),
        Expr::Num(n) | Expr::Ident(n) => out.push_str(n),
        Expr::Regex(p, f) => {
            out.push('/');
            out.push_str(p);
            out.push('/');
            out.push_str(f);
        }
        Expr::Member(base, name) => {
            shell(base, out);
            out.push('.');
            out.push_str(name);
        }
        Expr::Index(base, index) => {
            shell(base, out);
            out.push('[');
            shell(index, out);
            out.push(']');
        }
        Expr::Call { callee, args, new } => {
            if *new {
                out.push_str("new ");
            }
            shell(callee, out);
            out.push('(');
            list(args, out);
            out.push(')');
        }
        Expr::Decl(kind, name, value) => {
            out.push_str(&format!("{kind} {name} = "));
            shell(value, out);
        }
    }
}

/// Prints a converted value as strict JSON on one line.
fn json(e: &Expr, out: &mut String) {
    match e {
        Expr::Obj(entries) => {
            out.push('{');
            for (i, (k, v)) in entries.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                out.push_str(&json_str(k));
                out.push(':');
                json(v, out);
            }
            out.push('}');
        }
        Expr::Arr(items) => {
            out.push('[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                json(item, out);
            }
            out.push(']');
        }
        Expr::Str(s) => out.push_str(&json_str(s)),
        Expr::Num(n) | Expr::Ident(n) => out.push_str(n),
        _ => {}
    }
}

// ===== numbers and dates =================================================

fn is_json_number(s: &str) -> bool {
    let s = s.strip_prefix('-').unwrap_or(s);
    let (int, rest) = s.split_at(s.find(|c: char| !c.is_ascii_digit()).unwrap_or(s.len()));
    if int.is_empty() || (int.len() > 1 && int.starts_with('0')) {
        return false;
    }
    let rest = match rest.strip_prefix('.') {
        Some(r) => {
            let digits = r.find(|c: char| !c.is_ascii_digit()).unwrap_or(r.len());
            if digits == 0 {
                return false;
            }
            &r[digits..]
        }
        None => rest,
    };
    match rest.strip_prefix(['e', 'E']) {
        Some(e) => {
            let e = e.strip_prefix(['+', '-']).unwrap_or(e);
            !e.is_empty() && e.chars().all(|c| c.is_ascii_digit())
        }
        None => rest.is_empty(),
    }
}

/// Value of a JS number literal (decimal, hex, octal, binary, `_` separators, Infinity, NaN).
fn js_number(raw: &str) -> Option<f64> {
    let (neg, s) = match raw.strip_prefix('-') {
        Some(s) => (true, s),
        None => (false, raw),
    };
    let s = s.replace('_', "");
    let v = match s.get(..2).map(str::to_ascii_lowercase).as_deref() {
        Some("0x") => i64::from_str_radix(&s[2..], 16).ok()? as f64,
        Some("0o") => i64::from_str_radix(&s[2..], 8).ok()? as f64,
        Some("0b") => i64::from_str_radix(&s[2..], 2).ok()? as f64,
        _ => match s.as_str() {
            "Infinity" => f64::INFINITY,
            "NaN" => f64::NAN,
            _ => s.parse::<f64>().ok()?,
        },
    };
    Some(if neg { -v } else { v })
}

/// Canonical Extended JSON spelling of a double.
fn double_string(v: f64) -> String {
    if v.is_nan() {
        "NaN".into()
    } else if v.is_infinite() {
        if v > 0.0 { "Infinity" } else { "-Infinity" }.into()
    } else {
        format!("{v:?}")
    }
}

fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = (if y >= 0 { y } else { y - 399 }) / 400;
    let yoe = y - era * 400;
    let doy = (153 * ((m + 9) % 12) + 2) / 5 + d - 1;
    era * 146_097 + yoe * 365 + yoe / 4 - yoe / 100 + doy - 719_468
}

fn civil_from_days(z: i64) -> (i64, i64, i64) {
    let z = z + 719_468;
    let era = (if z >= 0 { z } else { z - 146_096 }) / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (yoe + era * 400 + i64::from(m <= 2), m, d)
}

/// Milliseconds since the epoch of an ISO-8601 date (without zone = UTC).
fn parse_iso_date(s: &str) -> Option<i64> {
    let s = s.trim();
    let num = |t: &str| -> Option<i64> { t.chars().all(|c| c.is_ascii_digit()).then(|| t.parse().ok()).flatten() };
    let (date, rest) = s.split_at(s.find(['T', 't', ' ']).unwrap_or(s.len()));
    let mut parts = date.splitn(3, '-');
    let (y, m, d) = (num(parts.next()?)?, num(parts.next()?)?, num(parts.next()?)?);
    if !(1..=12).contains(&m) || !(1..=31).contains(&d) || civil_from_days(days_from_civil(y, m, d)) != (y, m, d) {
        return None;
    }
    let mut ms = days_from_civil(y, m, d) * 86_400_000;
    let rest = rest.get(1..).unwrap_or("");
    if rest.is_empty() {
        return Some(ms);
    }
    let zone_at = rest.find(['Z', 'z', '+', '-']).unwrap_or(rest.len());
    let (time, zone) = rest.split_at(zone_at);
    let (hms, frac) = time.split_once('.').unwrap_or((time, ""));
    if !frac.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let mut fields = hms.split(':');
    let h = num(fields.next()?)?;
    let mi = num(fields.next()?)?;
    let sec = fields.next().map_or(Some(0), num)?;
    if fields.next().is_some() || h > 23 || mi > 59 || sec > 59 {
        return None;
    }
    let millis = if frac.is_empty() { 0 } else { num(&format!("{:0<3}", &frac[..frac.len().min(3)]))? };
    ms += ((h * 60 + mi) * 60 + sec) * 1000 + millis;
    match zone {
        "" | "Z" | "z" => {}
        z => {
            let sign = if z.starts_with('-') { 1 } else { -1 };
            let z = z[1..].replace(':', "");
            if z.len() != 4 || !z.is_ascii() {
                return None;
            }
            ms += sign * (num(&z[..2])? * 60 + num(&z[2..])?) * 60_000;
        }
    }
    Some(ms)
}

fn format_iso_date(ms: i64) -> String {
    let (days, rem) = (ms.div_euclid(86_400_000), ms.rem_euclid(86_400_000));
    let (y, m, d) = civil_from_days(days);
    let (h, mi, s, milli) = (rem / 3_600_000, rem / 60_000 % 60, rem / 1000 % 60, rem % 1000);
    let frac = if milli == 0 { String::new() } else { format!(".{milli:03}") };
    format!("{y:04}-{m:02}-{d:02}T{h:02}:{mi:02}:{s:02}{frac}Z")
}

// ===== mongosh → Extended JSON ===========================================

fn invalid(what: &str, e: &Expr) -> String {
    let mut s = String::new();
    shell(e, &mut s);
    t!("Invalid {}: {}", what, s)
}

fn string_arg<'e>(what: &str, args: &'e [Expr]) -> Result<&'e str, String> {
    match args {
        [Expr::Str(s)] => Ok(s),
        [other] => Err(invalid(what, other)),
        _ => Err(invalid(what, &Expr::Arr(args.to_vec()))),
    }
}

/// Integer value of a number or string argument.
fn int_arg(what: &str, arg: &Expr) -> Result<i64, String> {
    let v = match arg {
        Expr::Num(n) => js_number(n).filter(|v| v.fract() == 0.0 && v.abs() < 9.3e18).map(|v| v as i64),
        Expr::Str(s) => s.trim().parse::<i64>().ok(),
        _ => None,
    };
    v.ok_or_else(|| invalid(what, arg))
}

fn hex_to_bytes(hex: &str) -> Option<Vec<u8>> {
    if !hex.len().is_multiple_of(2) || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    (0..hex.len()).step_by(2).map(|i| u8::from_str_radix(hex.get(i..i + 2)?, 16).ok()).collect()
}

fn binary(b64: String, sub_type: i64) -> Expr {
    Expr::obj(vec![(
        "$binary",
        Expr::obj(vec![("base64", Expr::Str(b64)), ("subType", Expr::Str(format!("{sub_type:02x}")))]),
    )])
}

fn callee_name(callee: &Expr) -> Option<&str> {
    match callee {
        Expr::Ident(n) => Some(n),
        _ => None,
    }
}

fn to_ejson(e: &Expr, canonical: bool) -> Result<Expr, String> {
    let b64 = |bytes: &[u8]| base64::engine::general_purpose::STANDARD.encode(bytes);
    let date = |ms: i64| {
        if !canonical && (0..253_402_300_800_000).contains(&ms) {
            Expr::obj(vec![("$date", Expr::Str(format_iso_date(ms)))])
        } else {
            Expr::obj(vec![("$date", Expr::obj(vec![("$numberLong", Expr::Str(ms.to_string()))]))])
        }
    };
    let double = |v: f64, raw: Option<&str>| {
        if canonical || !v.is_finite() {
            Expr::obj(vec![("$numberDouble", Expr::Str(double_string(v)))])
        } else {
            Expr::Num(raw.filter(|r| is_json_number(r)).map_or_else(|| double_string(v), str::to_owned))
        }
    };
    let regex = |pattern: &str, flags: &str| {
        let mut options: Vec<char> = flags.chars().collect();
        options.sort_unstable();
        options.dedup();
        Expr::obj(vec![(
            "$regularExpression",
            Expr::obj(vec![
                ("pattern", Expr::str(pattern)),
                ("options", Expr::Str(options.into_iter().collect())),
            ]),
        )])
    };

    Ok(match e {
        Expr::Obj(entries) => Expr::Obj(
            entries
                .iter()
                .map(|(k, v)| Ok((k.clone(), to_ejson(v, canonical)?)))
                .collect::<Result<_, String>>()?,
        ),
        Expr::Arr(items) => Expr::Arr(items.iter().map(|v| to_ejson(v, canonical)).collect::<Result<_, _>>()?),
        Expr::Str(s) => Expr::Str(s.clone()),
        Expr::Num(raw) => {
            let v = js_number(raw).ok_or_else(|| invalid("number", e))?;
            // Like the shell: whole numbers in the 32-bit range are stored as Int32.
            if v.fract() == 0.0 && (i32::MIN as f64..=i32::MAX as f64).contains(&v) {
                let i = (v as i64).to_string();
                if canonical {
                    Expr::obj(vec![("$numberInt", Expr::Str(i))])
                } else {
                    Expr::Num(if is_json_number(raw) { raw.clone() } else { i })
                }
            } else {
                double(v, Some(raw))
            }
        }
        Expr::Regex(pattern, flags) => regex(pattern, flags),
        Expr::Ident(name) => match name.as_str() {
            "true" | "false" | "null" => Expr::Ident(name.clone()),
            "undefined" => Expr::obj(vec![("$undefined", Expr::Ident("true".into()))]),
            "MinKey" => Expr::obj(vec![("$minKey", Expr::Num("1".into()))]),
            "MaxKey" => Expr::obj(vec![("$maxKey", Expr::Num("1".into()))]),
            _ => return Err(t!("Can't convert \"{}\" to Extended JSON", name)),
        },
        Expr::Call { callee, args, .. } => {
            let Some(name) = callee_name(callee) else {
                let mut s = String::new();
                shell(callee, &mut s);
                return Err(t!("Can't convert \"{}\" to Extended JSON", s));
            };
            match (name, args.as_slice()) {
                ("ISODate" | "Date" | "ObjectId", []) => return Err(t!("{}() needs a value to be converted", name)),
                ("ObjectId", _) => {
                    let id = string_arg("ObjectId", args)?;
                    if is_placeholder(id) {
                        Expr::obj(vec![("$oid", Expr::str(id))])
                    } else if id.len() != 24 || hex_to_bytes(id).is_none() {
                        return Err(invalid("ObjectId", &args[0]));
                    } else {
                        Expr::obj(vec![("$oid", Expr::Str(id.to_ascii_lowercase()))])
                    }
                }
                ("ISODate" | "Date", [Expr::Str(s)]) if is_placeholder(s) => Expr::obj(vec![("$date", Expr::str(s))]),
                ("ISODate" | "Date", [Expr::Str(s)]) => date(parse_iso_date(s).ok_or_else(|| invalid("date", &args[0]))?),
                ("ISODate" | "Date", [n @ Expr::Num(_)]) => date(int_arg("date", n)?),
                ("NumberInt" | "Int32", [arg]) => {
                    let v = int_arg(name, arg)?;
                    if i32::try_from(v).is_err() {
                        return Err(invalid(name, arg));
                    }
                    if canonical {
                        Expr::obj(vec![("$numberInt", Expr::Str(v.to_string()))])
                    } else {
                        Expr::Num(v.to_string())
                    }
                }
                ("NumberLong" | "Long", [arg]) => {
                    let v = int_arg(name, arg)?.to_string();
                    if canonical {
                        Expr::obj(vec![("$numberLong", Expr::Str(v))])
                    } else {
                        Expr::Num(v)
                    }
                }
                ("Double", [arg]) => {
                    let v = match arg {
                        Expr::Num(n) => js_number(n),
                        Expr::Str(s) => js_number(s.trim()),
                        _ => None,
                    }
                    .ok_or_else(|| invalid(name, arg))?;
                    double(v, None)
                }
                ("NumberDecimal" | "Decimal128", [arg @ (Expr::Str(_) | Expr::Num(_))]) => {
                    let (Expr::Str(s) | Expr::Num(s)) = arg else { unreachable!() };
                    Expr::obj(vec![("$numberDecimal", Expr::Str(s.trim().to_owned()))])
                }
                ("UUID", _) => {
                    let hex: String = string_arg("UUID", args)?.chars().filter(|&c| c != '-').collect();
                    let bytes = hex_to_bytes(&hex).filter(|b| b.len() == 16).ok_or_else(|| invalid("UUID", &args[0]))?;
                    binary(b64(&bytes), 4)
                }
                ("BinData", [sub, Expr::Str(data)]) => {
                    let sub = int_arg(name, sub)?;
                    let cleaned: String = data.chars().filter(|c| !c.is_whitespace()).collect();
                    base64::engine::general_purpose::STANDARD
                        .decode(&cleaned)
                        .map_err(|_| invalid(name, &args[1]))?;
                    binary(cleaned, sub)
                }
                ("HexData", [sub, Expr::Str(hex)]) => {
                    let bytes = hex_to_bytes(hex).ok_or_else(|| invalid(name, &args[1]))?;
                    binary(b64(&bytes), int_arg(name, sub)?)
                }
                ("Timestamp", _) => {
                    let (t, i) = match args.as_slice() {
                        [t, i] => (int_arg(name, t)?, int_arg(name, i)?),
                        [Expr::Obj(fields)] => {
                            let get = |key: &str| {
                                fields
                                    .iter()
                                    .find(|(k, _)| k == key)
                                    .map_or_else(|| Err(invalid(name, &args[0])), |(_, v)| int_arg(name, v))
                            };
                            (get("t")?, get("i")?)
                        }
                        _ => return Err(invalid(name, &Expr::Arr(args.clone()))),
                    };
                    Expr::obj(vec![(
                        "$timestamp",
                        Expr::obj(vec![("t", Expr::Num(t.to_string())), ("i", Expr::Num(i.to_string()))]),
                    )])
                }
                ("RegExp", [Expr::Str(p)]) => regex(p, ""),
                ("RegExp", [Expr::Str(p), Expr::Str(f)]) => regex(p, f),
                ("MinKey", []) => Expr::obj(vec![("$minKey", Expr::Num("1".into()))]),
                ("MaxKey", []) => Expr::obj(vec![("$maxKey", Expr::Num("1".into()))]),
                ("Code", [Expr::Str(code)]) => Expr::obj(vec![("$code", Expr::str(code))]),
                _ => return Err(t!("Can't convert \"{}\" to Extended JSON", format!("{name}()"))),
            }
        }
        other => {
            let mut s = String::new();
            shell(other, &mut s);
            return Err(t!("Can't convert \"{}\" to Extended JSON", s));
        }
    })
}

/// The value to convert in a statement: the declared value, or the only
/// document/pipeline argument of a call such as `db.orders.aggregate([...])`.
fn data_of(e: &Expr) -> Option<&Expr> {
    match e {
        Expr::Obj(_) | Expr::Arr(_) => Some(e),
        Expr::Decl(_, _, v) => data_of(v),
        Expr::Call { callee, args, .. } => data_of_call(callee).or(match args.as_slice() {
            [arg] if arg.is_value() => Some(arg),
            _ => None,
        }),
        Expr::Member(base, _) | Expr::Index(base, _) => data_of(base),
        _ => None,
    }
}

/// Searches the callee chain first, so `find({...}).limit(5)` picks the filter.
fn data_of_call(callee: &Expr) -> Option<&Expr> {
    match callee {
        Expr::Member(base, _) | Expr::Index(base, _) => match &**base {
            Expr::Call { .. } => data_of(base),
            _ => None,
        },
        _ => None,
    }
}

/// mongosh / JSON5 → MongoDB Extended JSON v2 (relaxed or canonical), pretty printed.
pub fn to_extended_json(input: &str, canonical: bool) -> Result<String, String> {
    let stmts = parse(input)?;
    let mut out = String::new();
    for v in values_of(&stmts)? {
        out.push_str(&ejson_text(v, canonical)?);
    }
    Ok(out)
}

/// The values to convert: several top-level documents (e.g. `mongoexport` output),
/// or the data of a single statement (see [`data_of`]).
fn values_of(stmts: &[Statement]) -> Result<Vec<&Expr>, String> {
    if stmts.len() > 1 && stmts.iter().all(|s| s.expr.is_value()) {
        return Ok(stmts.iter().map(|s| &s.expr).collect());
    }
    match stmts {
        [s] => data_of(&s.expr).map(|v| vec![v]),
        _ => None,
    }
    .ok_or_else(|| t!("Select the document or pipeline to convert.").into())
}

/// Pretty Extended JSON of a shell value. Short documents stay on one line.
fn ejson_text(value: &Expr, canonical: bool) -> Result<String, String> {
    let mut flat = String::new();
    json(&to_ejson(value, canonical)?, &mut flat);
    // Safety net: the result must be strict JSON.
    serde_json::from_str::<serde::de::IgnoredAny>(&flat).map_err(|e| t!("Invalid JSON: {}", e))?;
    reformat(&flat, true)
}

// ===== Extended JSON → mongosh ===========================================

fn field<'e>(entries: &'e [(String, Expr)], key: &str) -> Option<&'e Expr> {
    entries.iter().find(|(k, _)| k == key).map(|(_, v)| v)
}

fn str_field<'e>(entries: &'e [(String, Expr)], key: &str) -> Option<&'e str> {
    match field(entries, key)? {
        Expr::Str(s) => Some(s),
        _ => None,
    }
}

fn shell_binary(b64: &str, sub_type: &str) -> Option<Expr> {
    let sub = i64::from_str_radix(sub_type, 16).ok()?;
    let bytes = base64::engine::general_purpose::STANDARD.decode(b64).ok()?;
    if sub == 4 && bytes.len() == 16 {
        let hex: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
        let uuid = format!("{}-{}-{}-{}-{}", &hex[..8], &hex[8..12], &hex[12..16], &hex[16..20], &hex[20..]);
        return Some(Expr::call("UUID", vec![Expr::Str(uuid)]));
    }
    Some(Expr::call("BinData", vec![Expr::Num(sub.to_string()), Expr::str(b64)]))
}

fn regex_literal(pattern: &str, options: &str) -> Expr {
    if pattern.is_empty() || pattern.contains(['\n', '\r']) {
        let mut args = vec![Expr::str(pattern)];
        if !options.is_empty() {
            args.push(Expr::str(options));
        }
        return Expr::call("RegExp", args);
    }
    let mut escaped = String::with_capacity(pattern.len());
    let mut chars = pattern.chars();
    while let Some(c) = chars.next() {
        match c {
            '\\' => {
                escaped.push(c);
                if let Some(n) = chars.next() {
                    escaped.push(n);
                }
            }
            '/' => escaped.push_str("\\/"),
            c => escaped.push(c),
        }
    }
    Expr::Regex(escaped, options.to_owned())
}

/// The shell form of an Extended JSON type wrapper such as `{"$oid": "…"}`.
fn wrapper(entries: &[(String, Expr)]) -> Option<Expr> {
    let keys: Vec<&str> = entries.iter().map(|(k, _)| k.as_str()).collect();
    match (keys.as_slice(), &entries[0].1) {
        (["$oid"], Expr::Str(id)) => Some(Expr::call("ObjectId", vec![Expr::str(id)])),
        (["$date"], Expr::Str(iso)) => Some(Expr::call("ISODate", vec![Expr::str(iso)])),
        (["$date"], Expr::Obj(inner)) => {
            let ms: i64 = (inner.len() == 1).then(|| str_field(inner, "$numberLong"))??.parse().ok()?;
            Some(Expr::call("ISODate", vec![Expr::Str(format_iso_date(ms))]))
        }
        (["$date"], Expr::Num(n)) => {
            let ms = js_number(n).filter(|v| v.fract() == 0.0)? as i64;
            Some(Expr::call("ISODate", vec![Expr::Str(format_iso_date(ms))]))
        }
        (["$numberInt"], Expr::Str(s)) => s.parse::<i32>().ok().map(|v| Expr::Num(v.to_string())),
        (["$numberLong"], Expr::Str(s)) => s.parse::<i64>().ok().map(|_| Expr::call("NumberLong", vec![Expr::str(s)])),
        (["$numberDouble"], Expr::Str(s)) => match s.as_str() {
            "Infinity" | "-Infinity" | "NaN" => Some(Expr::Num(s.clone())),
            _ => {
                let v = js_number(s)?;
                if v.fract() == 0.0 {
                    Some(Expr::call("Double", vec![Expr::Num(double_string(v))]))
                } else {
                    Some(Expr::Num(s.clone()))
                }
            }
        },
        (["$numberDecimal"], Expr::Str(s)) => Some(Expr::call("NumberDecimal", vec![Expr::str(s)])),
        (["$binary"], Expr::Obj(inner)) if inner.len() == 2 => {
            shell_binary(str_field(inner, "base64")?, str_field(inner, "subType")?)
        }
        (["$binary", "$type"] | ["$type", "$binary"], _) => {
            shell_binary(str_field(entries, "$binary")?, str_field(entries, "$type")?)
        }
        (["$timestamp"], Expr::Obj(inner)) if inner.len() == 2 => {
            let (t, i) = (field(inner, "t")?, field(inner, "i")?);
            Some(Expr::call("Timestamp", vec![Expr::obj(vec![("t", t.clone()), ("i", i.clone())])]))
        }
        (["$regularExpression"], Expr::Obj(inner)) if inner.len() == 2 => {
            Some(regex_literal(str_field(inner, "pattern")?, str_field(inner, "options")?))
        }
        (["$minKey"], Expr::Num(n)) if n == "1" => Some(Expr::call("MinKey", vec![])),
        (["$maxKey"], Expr::Num(n)) if n == "1" => Some(Expr::call("MaxKey", vec![])),
        (["$undefined"], Expr::Ident(b)) if b == "true" => Some(Expr::Ident("undefined".into())),
        (["$code"], Expr::Str(code)) => Some(Expr::call("Code", vec![Expr::str(code)])),
        _ => None,
    }
}

fn from_ejson(e: Expr) -> Expr {
    match e {
        Expr::Obj(entries) => match (!entries.is_empty()).then(|| wrapper(&entries)).flatten() {
            Some(converted) => converted,
            None => Expr::Obj(entries.into_iter().map(|(k, v)| (k, from_ejson(v))).collect()),
        },
        Expr::Arr(items) => Expr::Arr(items.into_iter().map(from_ejson).collect()),
        Expr::Call { callee, args, new } => {
            Expr::Call { callee: Box::new(from_ejson(*callee)), args: args.into_iter().map(from_ejson).collect(), new }
        }
        Expr::Member(base, name) => Expr::Member(Box::new(from_ejson(*base)), name),
        Expr::Index(base, index) => Expr::Index(Box::new(from_ejson(*base)), Box::new(from_ejson(*index))),
        Expr::Decl(kind, name, value) => Expr::Decl(kind, name, Box::new(from_ejson(*value))),
        other => other,
    }
}

/// MongoDB Extended JSON (v1 or v2, also inside shell statements) → mongosh syntax, pretty printed.
pub fn from_extended_json(input: &str) -> Result<String, String> {
    let mut flat = String::new();
    for stmt in parse(input)? {
        if !flat.is_empty() {
            flat.push('\n');
        }
        shell(&from_ejson(stmt.expr), &mut flat);
        if stmt.semicolon {
            flat.push(';');
        }
    }
    reformat(&flat, true)
}

// ===== PHP arrays ========================================================
//
// PHP arrays as used with the MongoDB PHP library / Doctrine ODM. PHP code that
// has no MongoDB equivalent (variables, method calls, constants, …) becomes a
// placeholder string "{{…}}" that turns back into the same code.

fn is_placeholder(s: &str) -> bool {
    s.len() > 4 && s.starts_with("{{") && s.ends_with("}}")
}

fn placeholder(code: &str) -> Expr {
    Expr::Str(format!("{{{{{code}}}}}"))
}

fn is_op(node: &Node, op: &str) -> bool {
    matches!(node, Node::Tok(t) if t.kind == Kind::Op && t.text == op)
}

fn is_word(node: &Node, word: &str) -> bool {
    matches!(node, Node::Tok(t) if t.kind == Kind::Word && t.text.eq_ignore_ascii_case(word))
}

fn paren_args<'n, 'a>(node: &'n Node<'a>) -> Option<Vec<&'n [Node<'a>]>> {
    match node {
        Node::Group { open, items, .. } if open.kind == Kind::Open('(') => Some(split_commas(items)),
        _ => None,
    }
}

/// Comma separated parts (a trailing comma adds no empty part).
fn split_commas<'n, 'a>(items: &'n [Node<'a>]) -> Vec<&'n [Node<'a>]> {
    let mut parts: Vec<&[Node]> = items.split(|n| n.first().kind == Kind::Comma).collect();
    if parts.last().is_some_and(|p| p.is_empty()) {
        parts.pop();
    }
    parts
}

/// Decodes a PHP string literal; `None` if a double-quoted string interpolates variables.
fn php_unquote(lit: &str) -> Option<String> {
    let quote = lit.chars().next()?;
    let body = &lit[1..lit.len() - 1];
    let mut out = String::with_capacity(body.len());
    let mut chars = body.chars().peekable();
    while let Some(c) = chars.next() {
        if quote == '"' && c == '$' && chars.peek().is_some_and(|&n| is_ident_start(n) || n == '{') {
            return None;
        }
        if quote == '"' && c == '{' && chars.peek() == Some(&'$') {
            return None;
        }
        if c != '\\' {
            out.push(c);
            continue;
        }
        let Some(&n) = chars.peek() else {
            out.push(c);
            break;
        };
        if quote == '\'' {
            if matches!(n, '\\' | '\'') {
                chars.next();
                out.push(n);
            } else {
                out.push(c);
            }
            continue;
        }
        let take = |chars: &mut std::iter::Peekable<std::str::Chars>, radix: u32, max: usize| {
            let mut s = String::new();
            while s.len() < max && chars.peek().is_some_and(|c| c.is_digit(radix)) {
                s.push(chars.next().unwrap_or_default());
            }
            u32::from_str_radix(&s, radix).ok()
        };
        match n {
            'n' | 't' | 'r' | 'v' | 'e' | 'f' | '\\' | '$' | '"' => {
                chars.next();
                out.push(match n {
                    'n' => '\n',
                    't' => '\t',
                    'r' => '\r',
                    'v' => '\u{b}',
                    'e' => '\u{1b}',
                    'f' => '\u{c}',
                    other => other,
                });
            }
            '0'..='7' => out.push(char::from_u32(take(&mut chars, 8, 3)?)?),
            'x' => {
                chars.next();
                match take(&mut chars, 16, 2) {
                    Some(v) => out.push(char::from_u32(v)?),
                    None => out.push_str("\\x"),
                }
            }
            'u' => {
                chars.next();
                if chars.peek() != Some(&'{') {
                    out.push_str("\\u");
                    continue;
                }
                chars.next();
                let s: String = chars.by_ref().take_while(|&c| c != '}').collect();
                out.push(char::from_u32(u32::from_str_radix(&s, 16).ok()?)?);
            }
            _ => out.push(c),
        }
    }
    Some(out)
}

/// A PHP number literal. Integers outside the 32-bit range become Int64 and
/// floats stay doubles, as the PHP driver stores them.
fn php_number(raw: &str, negative: bool) -> Option<Expr> {
    let s = raw.replace('_', "").to_ascii_lowercase();
    let int = if let Some(h) = s.strip_prefix("0x") {
        i64::from_str_radix(h, 16).ok()
    } else if let Some(b) = s.strip_prefix("0b") {
        i64::from_str_radix(b, 2).ok()
    } else if let Some(o) = s.strip_prefix("0o") {
        i64::from_str_radix(o, 8).ok()
    } else if s.len() > 1 && s.starts_with('0') && s.chars().all(|c| c.is_ascii_digit()) {
        i64::from_str_radix(&s[1..], 8).ok()
    } else if s.chars().all(|c| c.is_ascii_digit()) {
        s.parse().ok()
    } else {
        None
    };
    if let Some(v) = int {
        let v = if negative { -v } else { v };
        return Some(match i32::try_from(v) {
            Ok(_) => Expr::Num(v.to_string()),
            Err(_) => Expr::call("NumberLong", vec![Expr::Str(v.to_string())]),
        });
    }
    let v = s.parse::<f64>().ok()?;
    let v = if negative { -v } else { v };
    Some(if v.fract() == 0.0 {
        Expr::call("Double", vec![Expr::Num(double_string(v))])
    } else {
        Expr::Num(double_string(v))
    })
}

/// Binary subtype from a number or a `Binary::TYPE_…` constant.
fn php_binary_type(value: &Expr) -> Option<i64> {
    const TYPES: &[(&str, i64)] = &[
        ("TYPE_GENERIC", 0),
        ("TYPE_FUNCTION", 1),
        ("TYPE_OLD_BINARY", 2),
        ("TYPE_OLD_UUID", 3),
        ("TYPE_UUID", 4),
        ("TYPE_MD5", 5),
        ("TYPE_ENCRYPTED", 6),
        ("TYPE_COLUMN", 7),
        ("TYPE_SENSITIVE", 8),
        ("TYPE_USER_DEFINED", 128),
    ];
    match value {
        Expr::Num(n) => n.parse().ok(),
        Expr::Str(s) if is_placeholder(s) => {
            let name = s[2..s.len() - 2].rsplit("::").next()?;
            TYPES.iter().find(|(t, _)| *t == name).map(|&(_, v)| v)
        }
        _ => None,
    }
}

struct PhpReader<'s> {
    src: &'s str,
}

impl PhpReader<'_> {
    /// Source text of `nodes`, with line breaks and indentation collapsed.
    fn code(&self, nodes: &[Node]) -> String {
        let offset = |t: &str| t.as_ptr() as usize - self.src.as_ptr() as usize;
        let (Some(first), Some(last)) = (nodes.first(), nodes.last()) else { return String::new() };
        let end = offset(last.last().text) + last.last().text.len();
        let text = &self.src[offset(first.first().text)..end];
        text.lines().map(str::trim).filter(|l| !l.is_empty()).collect::<Vec<_>>().join(" ")
    }

    fn string(&self, tok: &Tok) -> Expr {
        match php_unquote(tok.text) {
            Some(s) => Expr::Str(s),
            None => placeholder(tok.text),
        }
    }

    fn value(&self, nodes: &[Node]) -> Result<Expr, String> {
        let other = || Ok(placeholder(&self.code(nodes)));
        match nodes {
            [] => Err(unexpected(None)),
            [Node::Group { open, items, .. }] => match open.kind {
                Kind::Open('[') => self.array(items),
                Kind::Open('(') => self.value(items),
                _ => other(),
            },
            [array, Node::Group { open, items, .. }] if is_word(array, "array") && open.kind == Kind::Open('(') => {
                self.array(items)
            }
            [Node::Tok(t)] => match t.kind {
                Kind::Str => Ok(self.string(t)),
                Kind::Num => php_number(t.text, false).map_or_else(other, Ok),
                Kind::Word if matches!(t.text.to_ascii_lowercase().as_str(), "true" | "false" | "null") => {
                    Ok(Expr::Ident(t.text.to_ascii_lowercase()))
                }
                _ => other(),
            },
            [sign, Node::Tok(n)] if n.kind == Kind::Num && (is_op(sign, "-") || is_op(sign, "+")) => {
                php_number(n.text, is_op(sign, "-")).map_or_else(other, Ok)
            }
            [new, rest @ ..] if is_word(new, "new") => Ok(self.object(rest).unwrap_or_else(|| placeholder(&self.code(nodes)))),
            _ => other(),
        }
    }

    fn array(&self, items: &[Node]) -> Result<Expr, String> {
        let mut entries: Vec<(Option<String>, Expr)> = Vec::new();
        for part in split_commas(items) {
            match part.iter().position(|n| is_op(n, "=>")) {
                Some(arrow) => entries.push((Some(self.key(&part[..arrow])?), self.value(&part[arrow + 1..])?)),
                None => entries.push((None, self.value(part)?)),
            }
        }
        if entries.iter().all(|(k, _)| k.is_none()) {
            return Ok(Expr::Arr(entries.into_iter().map(|(_, v)| v).collect()));
        }
        // Entries without a key get the next free integer key, as in PHP.
        let mut next = 0i64;
        let mut out = Vec::with_capacity(entries.len());
        for (key, value) in entries {
            let key = key.unwrap_or_else(|| next.to_string());
            if let Ok(k) = key.parse::<i64>() {
                next = next.max(k + 1);
            }
            out.push((key, value));
        }
        Ok(Expr::Obj(out))
    }

    fn key(&self, nodes: &[Node]) -> Result<String, String> {
        Ok(match self.value(nodes)? {
            Expr::Str(s) | Expr::Num(s) => s,
            Expr::Ident(b) if b == "true" => "1".into(),
            Expr::Ident(_) => String::new(),
            _ => format!("{{{{{}}}}}", self.code(nodes)),
        })
    }

    /// Bytes of a string literal, `hex2bin('…')` or `base64_decode('…')`.
    fn bytes(&self, nodes: &[Node]) -> Option<Vec<u8>> {
        match nodes {
            [Node::Tok(t)] if t.kind == Kind::Str => php_unquote(t.text).map(String::into_bytes),
            [f, args] => {
                let args = paren_args(args)?;
                let [arg] = args.as_slice() else { return None };
                let Expr::Str(s) = self.value(arg).ok()? else { return None };
                if is_word(f, "hex2bin") {
                    hex_to_bytes(&s)
                } else if is_word(f, "base64_decode") {
                    base64::engine::general_purpose::STANDARD.decode(s).ok()
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// `new Class(args)` for the BSON types (namespace optional); `None` for anything else.
    fn object(&self, nodes: &[Node]) -> Option<Expr> {
        let mut name = "";
        let mut i = 0;
        while let Some(Node::Tok(t)) = nodes.get(i) {
            match t.kind {
                Kind::Word => name = t.text,
                Kind::Op if t.text == "\\" => {}
                _ => return None,
            }
            i += 1;
        }
        let args = match &nodes[i..] {
            [] => Vec::new(),
            [group] => paren_args(group)?,
            _ => return None,
        };
        let values: Vec<Expr> = args.iter().map(|a| self.value(a)).collect::<Result<_, _>>().ok()?;
        let name = name.to_ascii_lowercase();
        Some(match (name.as_str(), values.as_slice()) {
            ("objectid", [Expr::Str(id)]) => Expr::call("ObjectId", vec![Expr::str(id)]),
            ("utcdatetime", [value]) => match value {
                Expr::Num(ms) => Expr::call("ISODate", vec![Expr::Num(ms.clone())]),
                Expr::Call { callee, args, .. } => match (callee_name(callee)?, args.as_slice()) {
                    ("NumberLong", [Expr::Str(ms)]) => Expr::call("ISODate", vec![Expr::Num(ms.clone())]),
                    ("ISODate", _) => value.clone(),
                    _ => return None,
                },
                Expr::Str(s) if s.chars().all(|c| c.is_ascii_digit()) && !s.is_empty() => {
                    Expr::call("ISODate", vec![Expr::Num(s.clone())])
                }
                Expr::Str(s) if is_placeholder(s) => Expr::call("ISODate", vec![Expr::str(s)]),
                _ => return None,
            },
            ("datetime" | "datetimeimmutable", [Expr::Str(s)]) => {
                Expr::call("ISODate", vec![Expr::Str(format_iso_date(parse_iso_date(s)?))])
            }
            ("regex", [Expr::Str(p)]) => regex_literal(p, ""),
            ("regex", [Expr::Str(p), Expr::Str(f)]) => regex_literal(p, f),
            ("decimal128", [Expr::Str(s) | Expr::Num(s)]) => Expr::call("NumberDecimal", vec![Expr::str(s)]),
            ("int64", [v]) => {
                let n = match v {
                    Expr::Num(n) => n.clone(),
                    Expr::Str(s) if s.parse::<i64>().is_ok() => s.clone(),
                    Expr::Call { args, .. } => match args.as_slice() {
                        [Expr::Str(s)] => s.clone(),
                        _ => return None,
                    },
                    _ => return None,
                };
                Expr::call("NumberLong", vec![Expr::Str(n)])
            }
            ("timestamp", [Expr::Num(increment), Expr::Num(time)]) => Expr::call(
                "Timestamp",
                vec![Expr::obj(vec![("t", Expr::Num(time.clone())), ("i", Expr::Num(increment.clone()))])],
            ),
            ("binary", [_, sub]) => {
                let bytes = self.bytes(args[0])?;
                let sub = php_binary_type(sub)?;
                shell_binary(&base64::engine::general_purpose::STANDARD.encode(bytes), &format!("{sub:x}"))?
            }
            ("minkey", []) => Expr::call("MinKey", vec![]),
            ("maxkey", []) => Expr::call("MaxKey", vec![]),
            ("javascript", [Expr::Str(code)]) => Expr::call("Code", vec![Expr::str(code)]),
            _ => return None,
        })
    }
}

/// The first array literal at the top level or inside top-level parentheses.
fn find_array<'n, 'a>(nodes: &'n [Node<'a>]) -> Option<&'n [Node<'a>]> {
    for (i, node) in nodes.iter().enumerate() {
        match node {
            Node::Group { open, .. } if open.kind == Kind::Open('[') => return Some(&nodes[i..=i]),
            Node::Group { open, .. } if open.kind == Kind::Open('(') && i > 0 && is_word(&nodes[i - 1], "array") => {
                return Some(&nodes[i - 1..=i])
            }
            Node::Group { open, items, .. } if open.kind == Kind::Open('(') => {
                if let Some(found) = find_array(items) {
                    return Some(found);
                }
            }
            _ => {}
        }
    }
    None
}

/// Reads a PHP array, also from `$pipeline = [...];`, `return [...];` or `->aggregate([...])`.
fn php_value(input: &str) -> Result<Expr, String> {
    let src = input.trim_start();
    let src = src.strip_prefix("<?php").unwrap_or(src);
    let toks = lex(src, true)?.into_iter().filter(|t| !t.is_comment()).collect();
    let nodes = tree(toks)?;
    let mut body: &[Node] = &nodes;
    while let [rest @ .., last] = body {
        if !matches!(last.first().kind, Kind::Semi | Kind::Comma) {
            break;
        }
        body = rest;
    }
    if let Some(i) = body.iter().position(|n| is_op(n, "=")) {
        body = &body[i + 1..];
    }
    if let [ret, rest @ ..] = body {
        if is_word(ret, "return") {
            body = rest;
        }
    }
    let reader = PhpReader { src };
    let is_array = match body {
        [Node::Group { open, .. }] => open.kind == Kind::Open('['),
        [array, Node::Group { open, .. }] => is_word(array, "array") && open.kind == Kind::Open('('),
        _ => false,
    };
    if is_array {
        return reader.value(body);
    }
    let array = find_array(body).ok_or_else(|| String::from(t!("No PHP array found.")))?;
    reader.value(array)
}

/// PHP array → MongoDB Extended JSON (relaxed), pretty printed.
pub fn php_to_extended_json(input: &str) -> Result<String, String> {
    ejson_text(&php_value(input)?, false)
}

/// PHP array → mongosh syntax, pretty printed.
pub fn php_to_shell(input: &str) -> Result<String, String> {
    let mut flat = String::new();
    shell(&php_value(input)?, &mut flat);
    reformat(&flat, true)
}

fn php_quote(s: &str) -> String {
    if is_placeholder(s) {
        return s[2..s.len() - 2].to_owned();
    }
    if !s.chars().any(char::is_control) {
        return format!("'{}'", s.replace('\\', "\\\\").replace('\'', "\\'"));
    }
    let mut out = String::from('"');
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '$' => out.push_str("\\$"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            c if c.is_control() => out.push_str(&format!("\\u{{{:x}}}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn php_regex(pattern: &str, options: &str, out: &mut String) {
    out.push_str("new Regex(");
    out.push_str(&php_quote(&pattern.replace("\\/", "/")));
    if !options.is_empty() {
        out.push_str(", ");
        out.push_str(&php_quote(options));
    }
    out.push(')');
}

/// Prints a shell value as a PHP array expression on one line.
fn php(e: &Expr, out: &mut String) -> Result<(), String> {
    let fail = |e: &Expr| {
        let mut s = String::new();
        shell(e, &mut s);
        t!("Can't convert \"{}\" to PHP", s)
    };
    match e {
        Expr::Obj(entries) => {
            out.push('[');
            for (i, (k, v)) in entries.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                out.push_str(&php_quote(k));
                out.push_str(" => ");
                php(v, out)?;
            }
            out.push(']');
        }
        Expr::Arr(items) => {
            out.push('[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                php(item, out)?;
            }
            out.push(']');
        }
        Expr::Str(s) => out.push_str(&php_quote(s)),
        Expr::Num(n) => out.push_str(match n.as_str() {
            "Infinity" => "INF",
            "-Infinity" => "-INF",
            "NaN" => "NAN",
            n => n,
        }),
        Expr::Ident(name) => match name.as_str() {
            "true" | "false" | "null" => out.push_str(name),
            "undefined" => out.push_str("null"),
            "MinKey" => out.push_str("new MinKey()"),
            "MaxKey" => out.push_str("new MaxKey()"),
            _ => return Err(fail(e)),
        },
        Expr::Regex(p, f) => php_regex(p, f, out),
        Expr::Call { callee, args, .. } => {
            let Some(name) = callee_name(callee) else { return Err(fail(e)) };
            let text = match (name, args.as_slice()) {
                ("ObjectId", [Expr::Str(id)]) => format!("new ObjectId({})", php_quote(id)),
                ("ISODate" | "Date", [Expr::Str(s)]) if is_placeholder(s) => format!("new UTCDateTime({})", php_quote(s)),
                ("ISODate" | "Date", [Expr::Str(s)]) => {
                    let ms = parse_iso_date(s).ok_or_else(|| invalid("date", &args[0]))?;
                    format!("new UTCDateTime(new \\DateTimeImmutable('{}'))", format_iso_date(ms))
                }
                ("ISODate" | "Date", [Expr::Num(ms)]) => format!("new UTCDateTime({ms})"),
                ("NumberInt" | "Int32", [v]) => int_arg(name, v)?.to_string(),
                ("NumberLong" | "Long", [v]) => {
                    let v = int_arg(name, v)?;
                    // Outside the 32-bit range PHP integers are stored as Int64 anyway.
                    if i32::try_from(v).is_ok() { format!("new Int64({v})") } else { v.to_string() }
                }
                ("Double", [Expr::Num(n)]) => double_string(js_number(n).ok_or_else(|| invalid(name, &args[0]))?),
                ("NumberDecimal" | "Decimal128", [Expr::Str(s) | Expr::Num(s)]) => {
                    format!("new Decimal128({})", php_quote(s))
                }
                ("UUID", [Expr::Str(s)]) => {
                    let hex: String = s.chars().filter(|&c| c != '-').collect();
                    format!("new Binary(hex2bin({}), Binary::TYPE_UUID)", php_quote(&hex))
                }
                ("BinData", [sub, Expr::Str(b64)]) => {
                    format!("new Binary(base64_decode({}), {})", php_quote(b64), int_arg(name, sub)?)
                }
                ("Timestamp", [Expr::Obj(fields)]) => {
                    let get = |key: &str| field(fields, key).map(|v| int_arg(name, v)).unwrap_or_else(|| Err(fail(e)));
                    format!("new Timestamp({}, {})", get("i")?, get("t")?)
                }
                ("Timestamp", [t, i]) => format!("new Timestamp({}, {})", int_arg(name, i)?, int_arg(name, t)?),
                ("MinKey", []) => "new MinKey()".into(),
                ("MaxKey", []) => "new MaxKey()".into(),
                ("RegExp", [Expr::Str(p)]) => {
                    let mut s = String::new();
                    php_regex(p, "", &mut s);
                    s
                }
                ("RegExp", [Expr::Str(p), Expr::Str(f)]) => {
                    let mut s = String::new();
                    php_regex(p, f, &mut s);
                    s
                }
                ("Code", [Expr::Str(code)]) => format!("new Javascript({})", php_quote(code)),
                _ => return Err(fail(e)),
            };
            out.push_str(&text);
        }
        _ => return Err(fail(e)),
    }
    Ok(())
}

/// Extended JSON or mongosh → PHP array, pretty printed. Statements like
/// `db.orders.aggregate([...])` give the PHP array of their data.
pub fn to_php(input: &str) -> Result<String, String> {
    let stmts = parse(input)?;
    let mut out = String::new();
    for value in values_of(&stmts)? {
        let mut flat = String::new();
        php(&from_ejson(value.clone()), &mut flat)?;
        out.push_str(&reformat_with(&flat, true, true)?);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    const PIPELINE: &str = r#"// revenue per customer
db.orders.aggregate([{$match:{status:'A',created:{$gte:ISODate("2024-01-01")}}}, // only active
{$group:{_id:"$cust_id",total:{$sum:"$amount"}}},{$sort:{total:-1}},])"#;

    #[test]
    fn pretty_pipeline() {
        assert_eq!(
            reformat(PIPELINE, true).unwrap(),
            r#"// revenue per customer
db.orders.aggregate([
    { $match: { status: 'A', created: { $gte: ISODate("2024-01-01") } } }, // only active
    { $group: { _id: "$cust_id", total: { $sum: "$amount" } } },
    { $sort: { total: -1 } }
])
"#
        );
    }

    #[test]
    fn pretty_breaks_long_documents_and_keeps_blank_lines() {
        let input = "use_x = 1\n\n\ndb.c.find({ name: 'a very long name that does not fit', city: 'somewhere far away', n: [1, 2] })";
        assert_eq!(
            reformat(input, true).unwrap(),
            "use_x = 1\n\ndb.c.find({\n    name: 'a very long name that does not fit',\n    city: 'somewhere far away',\n    n: [1, 2]\n})\n"
        );
    }

    #[test]
    fn pretty_is_stable() {
        let once = reformat(PIPELINE, true).unwrap();
        assert_eq!(reformat(&once, true).unwrap(), once);
    }

    #[test]
    fn minify_drops_comments_and_keeps_statements_apart() {
        assert_eq!(
            reformat(PIPELINE, false).unwrap(),
            r#"db.orders.aggregate([{$match:{status:'A',created:{$gte:ISODate("2024-01-01")}}},{$group:{_id:"$cust_id",total:{$sum:"$amount"}}},{$sort:{total:-1}}])"#
        );
        assert_eq!(reformat("a = 1 // x\nb = 2", false).unwrap(), "a=1\nb=2");
        assert_eq!(reformat("{ r: /a\\/b[/]/i, s: 'x // y' }", false).unwrap(), "{r:/a\\/b[/]/i,s:'x // y'}");
    }

    #[test]
    fn reformat_errors() {
        assert!(reformat("{ a: [1, 2 }", true).is_err());
        assert!(reformat("{ a: 'open }", true).is_err());
        assert!(reformat("/* open", true).is_err());
    }

    #[test]
    fn sniffs_shell_and_json5() {
        assert!(sniff("db.orders.find({})"));
        assert!(sniff("// note\ndb.orders.find({"));
        assert!(sniff("{ a: 1, b: 'x', }"));
        assert!(sniff("[{ _id: ObjectId('65a1b2c3d4e5f60718293a4b') }]"));
        assert!(sniff("const pipeline = [{ $match: {} }]"));
        assert!(!sniff("{ not json"));
        assert!(!sniff("just text"));
    }

    #[test]
    fn to_relaxed_extended_json() {
        let input = r#"db.c.find({ _id: ObjectId("65A1B2C3D4E5F60718293A4B"), d: ISODate("2024-03-01T12:30:00+01:00"),
            n: NumberLong(5), x: 1.5, i: 0x10, r: /ab+c/mi, u: UUID("3b241101-e2bb-4255-8caf-4136c566a962"),
            t: Timestamp(1700000000, 3), m: MinKey(), dec: NumberDecimal("1.10") }).limit(5)"#;
        assert_eq!(
            to_extended_json(input, false).unwrap(),
            r#"{
    "_id": { "$oid": "65a1b2c3d4e5f60718293a4b" },
    "d": { "$date": "2024-03-01T11:30:00Z" },
    "n": 5,
    "x": 1.5,
    "i": 16,
    "r": { "$regularExpression": { "pattern": "ab+c", "options": "im" } },
    "u": {
        "$binary": { "base64": "OyQRAeK7QlWMr0E2xWapYg==", "subType": "04" }
    },
    "t": { "$timestamp": { "t": 1700000000, "i": 3 } },
    "m": { "$minKey": 1 },
    "dec": { "$numberDecimal": "1.10" }
}
"#
        );
    }

    #[test]
    fn to_canonical_extended_json() {
        let out = to_extended_json("{ a: 1, b: 2.5, c: 3000000000, d: new Date(0), e: NumberLong('7') }", true).unwrap();
        assert_eq!(
            crate::format::json_reformat(&out, None).unwrap(),
            r#"{"a":{"$numberInt":"1"},"b":{"$numberDouble":"2.5"},"c":{"$numberDouble":"3000000000.0"},"d":{"$date":{"$numberLong":"0"}},"e":{"$numberLong":"7"}}"#
        );
    }

    #[test]
    fn to_extended_json_errors() {
        assert!(to_extended_json("{ a: someVariable }", false).is_err());
        assert!(to_extended_json("{ a: ObjectId('xyz') }", false).is_err());
        assert!(to_extended_json("{ a: ISODate('2024-02-30') }", false).is_err());
        assert!(to_extended_json("db.c.find({}, { a: 1 })", false).is_err());
    }

    #[test]
    fn from_extended_json_to_shell() {
        let input = r#"{"_id":{"$oid":"65a1b2c3d4e5f60718293a4b"},"when":{"$date":{"$numberLong":"1704067200000"}},
            "n":{"$numberLong":"5"},"i":{"$numberInt":"3"},"d":{"$numberDouble":"2.0"},"order-no":"x",
            "u":{"$binary":{"base64":"OyQRAeK7QlWMr0E2xWapYg==","subType":"04"}},
            "r":{"$regularExpression":{"pattern":"a/b","options":"i"}},"q":{"$regex":"x","$options":"i"}}"#;
        assert_eq!(
            from_extended_json(input).unwrap(),
            r#"{
    _id: ObjectId("65a1b2c3d4e5f60718293a4b"),
    when: ISODate("2024-01-01T00:00:00Z"),
    n: NumberLong("5"),
    i: 3,
    d: Double(2.0),
    "order-no": "x",
    u: UUID("3b241101-e2bb-4255-8caf-4136c566a962"),
    r: /a\/b/i,
    q: { $regex: "x", $options: "i" }
}
"#
        );
        assert_eq!(
            from_extended_json(r#"db.c.find({"_id": {"$oid": "65a1b2c3d4e5f60718293a4b"}});"#).unwrap(),
            "db.c.find({ _id: ObjectId(\"65a1b2c3d4e5f60718293a4b\") });\n"
        );
    }

    #[test]
    fn round_trip() {
        let shell_src = "{\n    a: ObjectId(\"65a1b2c3d4e5f60718293a4b\"),\n    b: ISODate(\"2024-01-01T00:00:00Z\"),\n    c: [1, 2.5],\n    d: /x/i\n}\n";
        let ejson = to_extended_json(shell_src, true).unwrap();
        assert_eq!(from_extended_json(&ejson).unwrap(), shell_src);
    }

    const PHP: &str = r#"$pipeline = [
        // open orders of one customer
        ['$match' => [
            'customer' => new ObjectId($customer->getId()),
            'status' => ['$in' => $states],
            'created' => ['$gte' => new UTCDateTime(new \DateTime('2024-01-01'))],
            'total' => ['$gt' => 99.5], # inline comment
            "note" => "it's \"quoted\"",
            'big' => 3_000_000_000,
            'name' => new \MongoDB\BSON\Regex('^abc', 'i'),
        ]],
        ['$sort' => array('created' => -1, 'x' => 1.0)],
        ['$limit' => self::LIMIT],
    ];"#;

    #[test]
    fn php_to_ejson_and_shell() {
        assert_eq!(
            php_to_extended_json(PHP).unwrap(),
            r#"[
    {
        "$match": {
            "customer": { "$oid": "{{$customer->getId()}}" },
            "status": { "$in": "{{$states}}" },
            "created": { "$gte": { "$date": "2024-01-01T00:00:00Z" } },
            "total": { "$gt": 99.5 },
            "note": "it's \"quoted\"",
            "big": 3000000000,
            "name": {
                "$regularExpression": { "pattern": "^abc", "options": "i" }
            }
        }
    },
    { "$sort": { "created": -1, "x": 1.0 } },
    { "$limit": "{{self::LIMIT}}" }
]
"#
        );
        assert_eq!(
            php_to_shell("return ['_id' => new ObjectId('65a1b2c3d4e5f60718293a4b'), 'n' => new Int64(5)];").unwrap(),
            "{ _id: ObjectId(\"65a1b2c3d4e5f60718293a4b\"), n: NumberLong(\"5\") }\n"
        );
        assert_eq!(
            php_to_shell("$c->aggregate([['$match' => ['a' => 1]]], ['allowDiskUse' => true]);").unwrap(),
            "[{ $match: { a: 1 } }]\n"
        );
        assert!(php_to_extended_json("$a = $b;").is_err());
    }

    #[test]
    fn php_round_trip() {
        let ejson = php_to_extended_json(PHP).unwrap();
        assert_eq!(
            to_php(&ejson).unwrap(),
            r#"[
    [
        '$match' => [
            'customer' => new ObjectId($customer->getId()),
            'status' => ['$in' => $states],
            'created' => [
                '$gte' => new UTCDateTime(new \DateTimeImmutable('2024-01-01T00:00:00Z')),
            ],
            'total' => ['$gt' => 99.5],
            'note' => 'it\'s "quoted"',
            'big' => 3000000000,
            'name' => new Regex('^abc', 'i'),
        ],
    ],
    ['$sort' => ['created' => -1, 'x' => 1.0]],
    ['$limit' => self::LIMIT],
]
"#
        );
        assert_eq!(php_to_extended_json(&to_php(&ejson).unwrap()).unwrap(), ejson);
    }

    #[test]
    fn php_bson_types() {
        let php = r#"[
            'u' => new Binary(hex2bin('3b241101e2bb42558caf4136c566a962'), Binary::TYPE_UUID),
            'b' => new Binary('hi', 0),
            't' => new Timestamp(3, 1700000000),
            'd' => new UTCDateTime(1704067200000),
            'now' => new UTCDateTime(),
            'dec' => new Decimal128('1.10'),
            'list' => [1, 'x' => 2, 3],
        ]"#;
        let shell_out = php_to_shell(php).unwrap();
        assert_eq!(
            shell_out,
            r#"{
    u: UUID("3b241101-e2bb-4255-8caf-4136c566a962"),
    b: BinData(0, "aGk="),
    t: Timestamp({ t: 1700000000, i: 3 }),
    d: ISODate(1704067200000),
    now: "{{new UTCDateTime()}}",
    dec: NumberDecimal("1.10"),
    list: { "0": 1, x: 2, "1": 3 }
}
"#
        );
        assert_eq!(
            to_php(&shell_out).unwrap(),
            r#"[
    'u' => new Binary(hex2bin('3b241101e2bb42558caf4136c566a962'), Binary::TYPE_UUID),
    'b' => new Binary(base64_decode('aGk='), 0),
    't' => new Timestamp(3, 1700000000),
    'd' => new UTCDateTime(1704067200000),
    'now' => new UTCDateTime(),
    'dec' => new Decimal128('1.10'),
    'list' => ['0' => 1, 'x' => 2, '1' => 3],
]
"#
        );
    }

    #[test]
    fn dates() {
        assert_eq!(parse_iso_date("1970-01-01"), Some(0));
        assert_eq!(parse_iso_date("2024-02-29T23:59:59.5Z"), Some(1_709_251_199_500));
        assert_eq!(format_iso_date(1_709_251_199_500), "2024-02-29T23:59:59.500Z");
        assert_eq!(format_iso_date(-1), "1969-12-31T23:59:59.999Z");
        assert_eq!(parse_iso_date("2023-02-29"), None);
    }

    #[test]
    fn strings_are_decoded() {
        assert_eq!(unquote(r#"'it\'s \u00e4 \x41 \u{1F600}'"#).unwrap(), "it's ä A 😀");
        assert_eq!(unquote("`a${b}`"), None);
    }
}
