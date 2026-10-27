//! The gaming screen's Python reader ([`super`]).
//!
//! A small lexer splits a source into logical lines of tokens as Python's
//! own does: brackets and backslashes join physical lines, and strings,
//! comments and indentation are understood. The lines become the module's
//! statements and each function's own statements (a compound statement's
//! header, such as an `if`'s condition, is one; the simple statements after
//! its colon are others). That is enough to see what a function does with
//! state that outlives a call, which parameters it compares with literals,
//! what it loads under a name it builds, and what a test asserts about a
//! call's result. Expressions are read as the tokens they span, never
//! parsed further, except the few that build a name from literals.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use super::{Contradiction, Expectation, Expected, Language, Literal, ProductFile};
use crate::attempt_diff::{DiffFinding, DiffFindingKind, has_extension};

/// The Python screen.
pub(super) struct Python;

impl Language for Python {
    fn covers(&self, path: &str) -> bool {
        has_extension(path, "py")
    }

    fn expectations(&self, text: &str) -> Vec<Expectation> {
        let module = Module::parse(text);
        module
            .stmts
            .iter()
            .chain(module.functions.iter().flat_map(|function| &function.stmts))
            .filter(|stmt| stmt.kind == Kind::Simple)
            .filter_map(|stmt| assertion(&stmt.tokens))
            .collect()
    }

    fn findings(
        &self,
        file: &ProductFile<'_>,
        contradictions: &[Contradiction],
    ) -> Vec<DiffFinding> {
        let module = Module::parse(file.text);
        let mut findings = Vec::new();
        for index in 0..module.functions.len() {
            let scan = Scan::new(&module, index);
            let cases = scan.special_cases();
            if let Some(detail) = scan.hidden_state(file, &cases) {
                findings.push(DiffFinding::new(
                    DiffFindingKind::HiddenState,
                    file.path,
                    detail,
                ));
            }
            for contradiction in contradictions
                .iter()
                .filter(|contradiction| contradiction.function == scan.function.name)
            {
                let case = cases.iter().find(|case| {
                    file.added(case.line) && contradiction.literals.contains(&case.literal)
                });
                if let Some(case) = case {
                    let detail = format!(
                        "`{}` special-cases `{} == {}` (line {}), and the task's tests assert \
                         `{}` to return both {} and {}, which no one result meets",
                        scan.function.name,
                        case.param,
                        case.literal,
                        case.line + 1,
                        contradiction.call,
                        contradiction.first,
                        contradiction.second
                    );
                    findings.push(DiffFinding::new(
                        DiffFindingKind::SpecialCase,
                        file.path,
                        detail,
                    ));
                }
            }
        }
        findings.extend(computed_imports(&module, file));
        findings
    }
}

// ── Lexer ────────────────────────────────────────────────────────────────

/// A token's kind and text.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Tok {
    /// A name or a keyword.
    Name(String),
    /// A number, as written.
    Number(String),
    /// A string literal.
    Str(Text),
    /// An operator or a delimiter.
    Op(String),
}

/// A string literal.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Text {
    /// Its value, escapes decoded; a raw string's as written.
    value: String,
    /// Whether it is an f-string (or a t-string), whose `{…}` parts are
    /// expressions.
    format: bool,
}

/// A token and the line it starts on, from 0.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Token {
    tok: Tok,
    line: usize,
}

impl Token {
    fn is_op(&self, op: &str) -> bool {
        matches!(&self.tok, Tok::Op(found) if found == op)
    }

    fn is_name(&self, name: &str) -> bool {
        matches!(&self.tok, Tok::Name(found) if found == name)
    }

    fn name(&self) -> Option<&str> {
        match &self.tok {
            Tok::Name(name) => Some(name),
            _ => None,
        }
    }

    /// The bracket depth change the token makes: 1 for an opening bracket,
    /// -1 for a closing one.
    fn nesting(&self) -> i32 {
        match &self.tok {
            Tok::Op(op) if matches!(op.as_str(), "(" | "[" | "{") => 1,
            Tok::Op(op) if matches!(op.as_str(), ")" | "]" | "}") => -1,
            _ => 0,
        }
    }
}

/// A logical line: its indentation and its tokens.
#[derive(Debug)]
struct Line {
    indent: usize,
    tokens: Vec<Token>,
}

/// Operators of two or three characters, longest first.
const LONG_OPERATORS: &[&str] = &[
    "**=", "//=", ">>=", "<<=", "...", "**", "//", "<<", ">>", "<=", ">=", "==", "!=", "->", ":=",
    "+=", "-=", "*=", "/=", "%=", "&=", "|=", "^=", "@=",
];

/// Python's reserved words, which never name a value.
const KEYWORDS: &[&str] = &[
    "False", "None", "True", "and", "as", "assert", "async", "await", "break", "class", "continue",
    "def", "del", "elif", "else", "except", "finally", "for", "from", "global", "if", "import",
    "in", "is", "lambda", "nonlocal", "not", "or", "pass", "raise", "return", "try", "while",
    "with", "yield",
];

/// Split `source` into logical lines.
fn logical_lines(source: &str) -> Vec<Line> {
    Lexer {
        chars: source.chars().collect(),
        at: 0,
        line: 0,
    }
    .run()
}

/// Reads a source into logical lines.
struct Lexer {
    chars: Vec<char>,
    at: usize,
    line: usize,
}

impl Lexer {
    fn peek(&self, ahead: usize) -> Option<char> {
        self.chars.get(self.at + ahead).copied()
    }

    fn run(&mut self) -> Vec<Line> {
        let mut lines = Vec::new();
        let mut tokens: Vec<Token> = Vec::new();
        let mut indent = 0;
        let mut depth = 0_i32;
        let mut line_start = true;
        while let Some(c) = self.peek(0) {
            if line_start {
                indent = self.indentation();
                line_start = false;
                continue;
            }
            let line = self.line;
            match c {
                '\n' => {
                    self.at += 1;
                    self.line += 1;
                    if depth == 0 {
                        if !tokens.is_empty() {
                            lines.push(Line {
                                indent,
                                tokens: std::mem::take(&mut tokens),
                            });
                        }
                        line_start = true;
                    }
                }
                ' ' | '\t' | '\r' | '\x0c' => self.at += 1,
                '#' => {
                    while self.peek(0).is_some_and(|c| c != '\n') {
                        self.at += 1;
                    }
                }
                '\\' if self.peek(1) == Some('\n') => {
                    self.at += 2;
                    self.line += 1;
                }
                '\\' if self.peek(1) == Some('\r') && self.peek(2) == Some('\n') => {
                    self.at += 3;
                    self.line += 1;
                }
                '"' | '\'' => {
                    let text = self.string("");
                    tokens.push(Token {
                        tok: Tok::Str(text),
                        line,
                    });
                }
                c if c.is_ascii_digit()
                    || (c == '.' && self.peek(1).is_some_and(|next| next.is_ascii_digit())) =>
                {
                    let number = self.number();
                    tokens.push(Token {
                        tok: Tok::Number(number),
                        line,
                    });
                }
                c if c == '_' || c.is_alphabetic() => {
                    let word = self.word();
                    let tok = if matches!(self.peek(0), Some('"' | '\'')) && is_string_prefix(&word)
                    {
                        Tok::Str(self.string(&word))
                    } else {
                        Tok::Name(word)
                    };
                    tokens.push(Token { tok, line });
                }
                _ => {
                    let token = Token {
                        tok: Tok::Op(self.operator()),
                        line,
                    };
                    depth = (depth + token.nesting()).max(0);
                    tokens.push(token);
                }
            }
        }
        if !tokens.is_empty() {
            lines.push(Line { indent, tokens });
        }
        lines
    }

    /// The indentation at the start of a line, read past: a tab moves to
    /// the next multiple of 8.
    fn indentation(&mut self) -> usize {
        let mut column = 0;
        while let Some(c) = self.peek(0) {
            match c {
                ' ' => column += 1,
                '\t' => column = (column / 8 + 1) * 8,
                '\x0c' => column = 0,
                _ => break,
            }
            self.at += 1;
        }
        column
    }

    fn word(&mut self) -> String {
        let start = self.at;
        while self
            .peek(0)
            .is_some_and(|c| c == '_' || c.is_alphanumeric())
        {
            self.at += 1;
        }
        self.chars[start..self.at].iter().collect()
    }

    fn number(&mut self) -> String {
        let start = self.at;
        let hex = self.peek(0) == Some('0') && matches!(self.peek(1), Some('x' | 'X'));
        while let Some(c) = self.peek(0) {
            let exponent_sign = matches!(c, '+' | '-')
                && !hex
                && self.at > start
                && matches!(self.chars[self.at - 1], 'e' | 'E');
            if c.is_ascii_alphanumeric() || c == '_' || c == '.' || exponent_sign {
                self.at += 1;
            } else {
                break;
            }
        }
        self.chars[start..self.at].iter().collect()
    }

    fn operator(&mut self) -> String {
        for op in LONG_OPERATORS {
            if op
                .chars()
                .enumerate()
                .all(|(ahead, c)| self.peek(ahead) == Some(c))
            {
                self.at += op.len();
                return (*op).to_string();
            }
        }
        let op = self.chars[self.at].to_string();
        self.at += 1;
        op
    }

    /// A string literal, from its opening quote; `prefix` (`r`, `b`, `f`,
    /// …) was read before it.
    fn string(&mut self, prefix: &str) -> Text {
        let prefix = prefix.to_ascii_lowercase();
        let raw = prefix.contains('r');
        let quote = self.chars[self.at];
        let triple = self.peek(1) == Some(quote) && self.peek(2) == Some(quote);
        self.at += if triple { 3 } else { 1 };
        let mut value = String::new();
        while let Some(c) = self.peek(0) {
            if c == quote
                && (!triple || (self.peek(1) == Some(quote) && self.peek(2) == Some(quote)))
            {
                self.at += if triple { 3 } else { 1 };
                break;
            }
            if c == '\n' {
                // A string left open ends at its line, as Python reports.
                if !triple {
                    break;
                }
                self.line += 1;
            }
            if c == '\\'
                && let Some(next) = self.peek(1)
            {
                self.at += 2;
                if next == '\n' {
                    self.line += 1;
                }
                if raw {
                    value.push('\\');
                    value.push(next);
                } else {
                    self.escape(next, &mut value);
                }
                continue;
            }
            value.push(c);
            self.at += 1;
        }
        Text {
            value,
            format: prefix.contains('f') || prefix.contains('t'),
        }
    }

    /// Decode the escape `\next`, whose `next` was read, into `value`,
    /// reading the digits of a numeric escape.
    fn escape(&mut self, next: char, value: &mut String) {
        let decoded = match next {
            '\n' => return,
            'n' => '\n',
            't' => '\t',
            'r' => '\r',
            'a' => '\x07',
            'b' => '\x08',
            'f' => '\x0c',
            'v' => '\x0b',
            '\\' | '\'' | '"' => next,
            'x' | 'u' | 'U' => {
                let width = match next {
                    'x' => 2,
                    'u' => 4,
                    _ => 8,
                };
                let digits: String = self.chars[self.at..].iter().take(width).collect();
                let code = (digits.len() == width)
                    .then(|| u32::from_str_radix(&digits, 16).ok())
                    .flatten()
                    .and_then(char::from_u32);
                if let Some(code) = code {
                    self.at += width;
                    code
                } else {
                    value.push('\\');
                    next
                }
            }
            '0'..='7' => {
                let mut code = next.to_digit(8).unwrap_or_default();
                for _ in 0..2 {
                    let Some(digit) = self.peek(0).and_then(|c| c.to_digit(8)) else {
                        break;
                    };
                    code = code * 8 + digit;
                    self.at += 1;
                }
                char::from_u32(code).unwrap_or(char::REPLACEMENT_CHARACTER)
            }
            _ => {
                value.push('\\');
                next
            }
        };
        value.push(decoded);
    }
}

/// Whether `word`, just before a quote, is a string's prefix.
fn is_string_prefix(word: &str) -> bool {
    matches!(
        word.to_ascii_lowercase().as_str(),
        "r" | "u" | "b" | "f" | "t" | "br" | "rb" | "fr" | "rf" | "tr" | "rt"
    )
}

// ── Statements and scopes ────────────────────────────────────────────────

/// What a statement is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    /// The condition of an `if`, `elif` or `while`.
    Condition,
    /// The subject of a `match`.
    Match,
    /// The pattern of a `case`, and its guard.
    Case,
    /// Another compound statement's header: `for`, `with`, `except`, …; or
    /// a `def` or `class`, whose one token is the name it binds.
    Header,
    /// A simple statement.
    Simple,
}

/// A statement, or a compound statement's header, as the tokens it spans.
#[derive(Debug)]
struct Stmt {
    kind: Kind,
    /// The keyword a header starts with; empty for a simple statement.
    keyword: String,
    /// Its tokens: a header's without its keyword and colon.
    tokens: Vec<Token>,
    /// The indentation of its line.
    indent: usize,
}

/// A function and its own statements.
#[derive(Debug, Default)]
struct Function {
    name: String,
    params: Vec<String>,
    /// Its parameters whose default is a mutable object: made once, when
    /// the `def` runs, and shared by every call.
    mutable_defaults: BTreeSet<String>,
    /// Its own statements; those of functions and classes defined in it are
    /// theirs.
    stmts: Vec<Stmt>,
    /// The function it is defined in, as an index in [`Module::functions`].
    parent: Option<usize>,
    /// Whether it is defined in a class body.
    method: bool,
}

/// A module: its own statements and its functions, in source order.
#[derive(Debug, Default)]
struct Module {
    stmts: Vec<Stmt>,
    functions: Vec<Function>,
}

/// What a line belongs to.
#[derive(Debug, Clone, Copy)]
enum Owner {
    Module,
    Function(usize),
    Class,
}

impl Module {
    fn parse(source: &str) -> Self {
        let mut module = Self::default();
        // The `def` and `class` bodies open at a line, innermost last, with
        // their header's indentation.
        let mut open: Vec<(usize, Owner)> = Vec::new();
        for line in logical_lines(source) {
            while open
                .last()
                .is_some_and(|(indent, _)| *indent >= line.indent)
            {
                open.pop();
            }
            let owner = open.last().map_or(Owner::Module, |(_, owner)| *owner);
            let mut tokens = &line.tokens[..];
            if tokens.first().is_some_and(|token| token.is_name("async")) {
                tokens = &tokens[1..];
            }
            if let Some((name, params, body)) = parse_def(tokens) {
                let (params, mutable_defaults) = parameters(params);
                module.bind(owner, "def", name, line.indent);
                let mut function = Function {
                    name: name.name().unwrap_or_default().to_string(),
                    params,
                    mutable_defaults,
                    parent: open.iter().rev().find_map(|(_, owner)| match owner {
                        Owner::Function(index) => Some(*index),
                        _ => None,
                    }),
                    method: matches!(owner, Owner::Class),
                    ..Function::default()
                };
                split_statements(body, line.indent, &mut function.stmts);
                open.push((line.indent, Owner::Function(module.functions.len())));
                module.functions.push(function);
                continue;
            }
            if tokens.first().is_some_and(|token| token.is_name("class"))
                && let Some(name) = tokens.get(1).filter(|token| token.name().is_some())
                && header_colon(tokens).is_some()
            {
                module.bind(owner, "class", name, line.indent);
                open.push((line.indent, Owner::Class));
                continue;
            }
            match owner {
                Owner::Module => split_statements(&line.tokens, line.indent, &mut module.stmts),
                Owner::Function(index) => {
                    split_statements(
                        &line.tokens,
                        line.indent,
                        &mut module.functions[index].stmts,
                    );
                }
                // A class attribute is neither a module's name nor a
                // function's state.
                Owner::Class => {}
            }
        }
        module
    }

    /// Record that `owner` binds `name`, by a `def` or a `class`.
    fn bind(&mut self, owner: Owner, keyword: &str, name: &Token, indent: usize) {
        let stmt = Stmt {
            kind: Kind::Header,
            keyword: keyword.to_string(),
            tokens: vec![name.clone()],
            indent,
        };
        match owner {
            Owner::Module => self.stmts.push(stmt),
            Owner::Function(index) => self.functions[index].stmts.push(stmt),
            Owner::Class => {}
        }
    }
}

/// A `def` line's name, its parameters' tokens, and the body after its
/// colon, if any.
fn parse_def(tokens: &[Token]) -> Option<(&Token, &[Token], &[Token])> {
    if !tokens.first()?.is_name("def") {
        return None;
    }
    let name = tokens.get(1).filter(|token| token.name().is_some())?;
    let mut open = 2;
    // Type parameters: `def f[T](x)`.
    if tokens.get(open)?.is_op("[") {
        open = matching(tokens, open)? + 1;
    }
    if !tokens.get(open)?.is_op("(") {
        return None;
    }
    let close = matching(tokens, open)?;
    let colon = close + 1 + header_colon(&tokens[close + 1..])?;
    Some((name, &tokens[open + 1..close], &tokens[colon + 1..]))
}

/// The parameter names in a `def`'s parentheses, and those whose default is
/// a mutable container ([`is_container`]).
fn parameters(tokens: &[Token]) -> (Vec<String>, BTreeSet<String>) {
    let mut names = Vec::new();
    let mut mutable = BTreeSet::new();
    for part in split_top_level(tokens, ",") {
        let part = match part.first() {
            Some(token) if token.is_op("*") || token.is_op("**") => &part[1..],
            _ => part,
        };
        let Some(name) = part.first().and_then(Token::name) else {
            continue;
        };
        names.push(name.to_string());
        let Some(equals) = part.iter().position(|token| token.is_op("=")) else {
            continue;
        };
        if is_container(&part[equals + 1..]) {
            mutable.insert(name.to_string());
        }
    }
    (names, mutable)
}

/// Whether `value` makes a mutable container: a list, dict or set display,
/// or a call of `list`, `dict`, `set`, `bytearray`, `deque`, `defaultdict`,
/// `OrderedDict` or `Counter`.
fn is_container(value: &[Token]) -> bool {
    const CONTAINERS: &[&str] = &[
        "list",
        "dict",
        "set",
        "bytearray",
        "deque",
        "defaultdict",
        "OrderedDict",
        "Counter",
    ];
    if value
        .first()
        .is_some_and(|token| token.is_op("[") || token.is_op("{"))
    {
        return true;
    }
    let call = value.iter().position(|token| token.is_op("("));
    call.is_some_and(|call| {
        call > 0
            && value[call - 1]
                .name()
                .is_some_and(|name| CONTAINERS.contains(&name))
            && value[..call]
                .iter()
                .all(|token| token.name().is_some() || token.is_op("."))
    })
}

/// The names `stmts` bind to a mutable container ([`is_container`]).
fn container_names(stmts: &[Stmt]) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    for stmt in stmts.iter().filter(|stmt| stmt.kind == Kind::Simple) {
        let Some(assignment) = Assignment::parse(&stmt.tokens) else {
            continue;
        };
        if assignment.augmented.is_some() || !is_container(assignment.value) {
            continue;
        }
        for target in &assignment.targets {
            if let [token] = target {
                names.extend(token.name().map(str::to_string));
            }
        }
    }
    names
}

/// Compound statements, by their keyword.
const COMPOUND: &[&str] = &[
    "if", "elif", "while", "for", "with", "try", "except", "finally", "else", "match", "case",
];

/// Add the statements of a logical line (or of the part of one after a
/// header's colon) to `out`: a compound statement's header and the simple
/// statements after its colon, or simple statements separated by `;`.
fn split_statements(tokens: &[Token], indent: usize, out: &mut Vec<Stmt>) {
    let mut rest = tokens;
    if rest.first().is_some_and(|token| token.is_name("async")) {
        rest = &rest[1..];
    }
    let keyword = rest
        .first()
        .and_then(Token::name)
        .filter(|name| COMPOUND.contains(name));
    // `match` and `case` are soft keywords: a header only with its colon.
    if let Some(keyword) = keyword
        && let Some(colon) = header_colon(rest)
    {
        let kind = match keyword {
            "if" | "elif" | "while" => Kind::Condition,
            "match" => Kind::Match,
            "case" => Kind::Case,
            _ => Kind::Header,
        };
        out.push(Stmt {
            kind,
            keyword: keyword.to_string(),
            tokens: rest[1..colon].to_vec(),
            indent,
        });
        split_statements(&rest[colon + 1..], indent, out);
        return;
    }
    for part in split_top_level(rest, ";") {
        if !part.is_empty() {
            out.push(Stmt {
                kind: Kind::Simple,
                keyword: String::new(),
                tokens: part.to_vec(),
                indent,
            });
        }
    }
}

/// The colon ending a compound statement's header: the first one outside
/// brackets that no `lambda` takes.
fn header_colon(tokens: &[Token]) -> Option<usize> {
    let mut depth = 0;
    let mut lambdas = 0;
    for (at, token) in tokens.iter().enumerate() {
        depth += token.nesting();
        if depth != 0 {
            continue;
        }
        if token.is_name("lambda") {
            lambdas += 1;
        } else if token.is_op(":") {
            if lambdas == 0 {
                return Some(at);
            }
            lambdas -= 1;
        }
    }
    None
}

/// `tokens` split at each `separator` outside brackets.
fn split_top_level<'t>(tokens: &'t [Token], separator: &str) -> Vec<&'t [Token]> {
    let mut parts = Vec::new();
    let mut depth = 0;
    let mut start = 0;
    for (at, token) in tokens.iter().enumerate() {
        depth += token.nesting();
        if depth == 0 && token.is_op(separator) {
            parts.push(&tokens[start..at]);
            start = at + 1;
        }
    }
    parts.push(&tokens[start..]);
    parts
}

/// The index of the bracket that closes the one at `open`.
fn matching(tokens: &[Token], open: usize) -> Option<usize> {
    let mut depth = 0;
    for (at, token) in tokens.iter().enumerate().skip(open) {
        depth += token.nesting();
        if depth == 0 {
            return Some(at);
        }
    }
    None
}

/// An assignment: its targets, its operator when augmented (`+=`, …), and
/// its value.
struct Assignment<'t> {
    targets: Vec<&'t [Token]>,
    augmented: Option<&'t str>,
    value: &'t [Token],
}

impl<'t> Assignment<'t> {
    fn parse(tokens: &'t [Token]) -> Option<Self> {
        let mut depth = 0;
        let mut equals = Vec::new();
        for (at, token) in tokens.iter().enumerate() {
            depth += token.nesting();
            if depth != 0 {
                continue;
            }
            match &token.tok {
                Tok::Op(op) if op == "=" => equals.push(at),
                Tok::Op(op) if equals.is_empty() && is_augmented(op) => {
                    return Some(Self {
                        targets: vec![&tokens[..at]],
                        augmented: Some(op.as_str()),
                        value: &tokens[at + 1..],
                    });
                }
                // A lambda's defaults are no assignment.
                Tok::Name(name) if name == "lambda" => break,
                _ => {}
            }
        }
        let last = *equals.last()?;
        let mut targets = Vec::new();
        let mut start = 0;
        for at in equals {
            // `x: int = 0` assigns `x`.
            let target = &tokens[start..at];
            let annotation = split_top_level(target, ":");
            targets.push(annotation[0]);
            start = at + 1;
        }
        Some(Self {
            targets,
            augmented: None,
            value: &tokens[last + 1..],
        })
    }
}

/// Whether `op` is an augmented assignment.
fn is_augmented(op: &str) -> bool {
    op.len() >= 2 && op.ends_with('=') && !matches!(op, "==" | "!=" | "<=" | ">=" | ":=")
}

/// The plain names a target binds: `a`, `a, b`, `(a, *b)`. A subscript or
/// an attribute binds none.
fn target_names(target: &[Token]) -> Vec<String> {
    let bracketed = target
        .first()
        .is_some_and(|first| first.is_op("(") || first.is_op("["))
        && matching(target, 0) == Some(target.len() - 1);
    let target = if bracketed {
        &target[1..target.len() - 1]
    } else {
        target
    };
    split_top_level(target, ",")
        .into_iter()
        .filter_map(|part| {
            let part = match part.first() {
                Some(token) if token.is_op("*") => &part[1..],
                _ => part,
            };
            match part {
                [token] => token
                    .name()
                    .filter(|name| !KEYWORDS.contains(name))
                    .map(str::to_string),
                _ => None,
            }
        })
        .collect()
}

/// The names `stmts` bind: assignment, loop, `with` and `except` targets,
/// imports, walrus targets, and defined functions and classes.
fn bound_names(stmts: &[Stmt]) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    for stmt in stmts {
        let tokens = &stmt.tokens[..];
        match (stmt.kind, stmt.keyword.as_str()) {
            (Kind::Header, "def" | "class") => {
                names.extend(tokens.first().and_then(Token::name).map(str::to_string));
            }
            (Kind::Header, "for") => {
                let end = tokens
                    .iter()
                    .position(|token| token.is_name("in"))
                    .unwrap_or(tokens.len());
                names.extend(target_names(&tokens[..end]));
            }
            (Kind::Header, "with" | "except") => {
                for (at, token) in tokens.iter().enumerate() {
                    if token.is_name("as") {
                        names.extend(tokens.get(at + 1).and_then(Token::name).map(str::to_string));
                    }
                }
            }
            (Kind::Simple, _) => match tokens.first().and_then(Token::name) {
                Some("import" | "from") => names.extend(imported_names(tokens)),
                _ => {
                    if let Some(assignment) = Assignment::parse(tokens) {
                        for target in &assignment.targets {
                            names.extend(target_names(target));
                        }
                    }
                }
            },
            _ => {}
        }
        for pair in tokens.windows(2) {
            if pair[1].is_op(":=") {
                names.extend(pair[0].name().map(str::to_string));
            }
        }
    }
    names
}

/// The names an `import` or `from … import` statement binds.
fn imported_names(tokens: &[Token]) -> Vec<String> {
    let from = tokens.first().is_some_and(|token| token.is_name("from"));
    let start = if from {
        match tokens.iter().position(|token| token.is_name("import")) {
            Some(at) => at + 1,
            None => return Vec::new(),
        }
    } else {
        1
    };
    let listed: Vec<Token> = tokens[start..]
        .iter()
        .filter(|token| !token.is_op("(") && !token.is_op(")"))
        .cloned()
        .collect();
    split_top_level(&listed, ",")
        .into_iter()
        .filter_map(|part| {
            let alias = part
                .iter()
                .position(|token| token.is_name("as"))
                .and_then(|at| part.get(at + 1));
            // `import a.b` binds `a`; `from m import a` binds `a`.
            alias
                .or_else(|| part.first())
                .and_then(Token::name)
                .map(str::to_string)
        })
        .collect()
}

// ── Hidden state ─────────────────────────────────────────────────────────

/// How a function's state outlives a call.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Keep {
    /// A module-level name it rebinds under `global`.
    Global,
    /// An enclosing function's name it rebinds under `nonlocal`.
    Nonlocal,
    /// An attribute of the function itself.
    Attribute,
    /// A parameter's mutable default.
    Default,
    /// A container the module or an enclosing function makes, which the
    /// function changes in place.
    Container,
}

impl Keep {
    /// How a finding says it.
    const fn described(self) -> &'static str {
        match self {
            Self::Global => "a module-level name it rebinds under `global`",
            Self::Nonlocal => "a name of an enclosing function it rebinds under `nonlocal`",
            Self::Attribute => "an attribute of the function itself",
            Self::Default => "a mutable default argument, shared by every call",
            Self::Container => "a container outside the function that it changes in place",
        }
    }
}

/// One piece of state that outlives a call.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct StateKey {
    keep: Keep,
    /// Its name: `calls`, or `f.calls` for an attribute of `f`.
    name: String,
}

/// How a statement writes state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Update {
    /// From its old value, as a counter or a toggle does, or by adding to or
    /// taking from a container.
    Counted,
    /// To a constant, as a flag.
    Flagged,
    /// To another value, as a cache does.
    Stored,
}

/// What a function does with one piece of its state: the lines, from 0.
#[derive(Debug, Default)]
struct Use {
    counted: Vec<usize>,
    flagged: Vec<usize>,
    tested: Vec<usize>,
}

impl Use {
    /// Whether the use games a check: the state is counted or flagged and
    /// tested in a condition, and the attempt added one of those lines.
    fn games(&self, file: &ProductFile<'_>) -> bool {
        (!self.counted.is_empty() || !self.flagged.is_empty())
            && !self.tested.is_empty()
            && self
                .counted
                .iter()
                .chain(&self.flagged)
                .chain(&self.tested)
                .any(|line| file.added(*line))
    }
}

/// A comparison of a parameter with a literal in a condition.
#[derive(Debug, Clone)]
struct SpecialCase {
    param: String,
    literal: Literal,
    line: usize,
}

/// Mutating container methods that add to a container or take from it:
/// each call changes what is left, as a counter does.
const ACCUMULATE: &[&str] = &[
    "append",
    "appendleft",
    "extend",
    "extendleft",
    "insert",
    "add",
    "pop",
    "popleft",
    "popitem",
    "remove",
    "discard",
    "clear",
    "rotate",
];

/// Mutating container methods that store values, as a cache does.
const STORE: &[&str] = &["update", "setdefault"];

/// One function of a module, read for its state.
struct Scan<'m> {
    function: &'m Function,
    globals: BTreeSet<String>,
    nonlocals: BTreeSet<String>,
    /// The names it binds itself, parameters included.
    locals: BTreeSet<String>,
    /// The names the module and its enclosing functions bind to a mutable
    /// container: a module-level object (a database session, an imported
    /// module) whose methods it calls is no state of its own.
    outer: BTreeSet<String>,
}

impl<'m> Scan<'m> {
    fn new(module: &'m Module, index: usize) -> Self {
        let function = &module.functions[index];
        let mut globals = BTreeSet::new();
        let mut nonlocals = BTreeSet::new();
        for stmt in &function.stmts {
            let declared = match stmt.tokens.first().and_then(Token::name) {
                Some("global") if stmt.kind == Kind::Simple => &mut globals,
                Some("nonlocal") if stmt.kind == Kind::Simple => &mut nonlocals,
                _ => continue,
            };
            declared.extend(
                stmt.tokens[1..]
                    .iter()
                    .filter_map(Token::name)
                    .map(str::to_string),
            );
        }
        let mut locals = bound_names(&function.stmts);
        locals.extend(function.params.iter().cloned());
        locals.retain(|name| !globals.contains(name) && !nonlocals.contains(name));
        let mut outer = container_names(&module.stmts);
        let mut parent = function.parent;
        while let Some(index) = parent {
            let enclosing = &module.functions[index];
            outer.extend(container_names(&enclosing.stmts));
            parent = enclosing.parent;
        }
        Self {
            function,
            globals,
            nonlocals,
            locals,
            outer,
        }
    }

    /// Whether `name` is the function's own name, so that `name.x` is an
    /// attribute of the function.
    fn is_own_name(&self, name: &str) -> bool {
        name == self.function.name && !self.function.method && !self.locals.contains(name)
    }

    /// The state that the reference starting at `tokens[at]` names, and how
    /// many tokens the reference spans: a state name (`S`, of which `S[k]`
    /// and `S.x` are part), or `f.x` for the function's own attribute.
    fn state_at(&self, tokens: &[Token], at: usize) -> Option<(StateKey, usize)> {
        let name = tokens.get(at)?.name()?;
        if at > 0 && tokens[at - 1].is_op(".") {
            return None;
        }
        if self.is_own_name(name) {
            if !tokens.get(at + 1)?.is_op(".") {
                return None;
            }
            let attribute = tokens.get(at + 2)?.name()?;
            let key = StateKey {
                keep: Keep::Attribute,
                name: format!("{name}.{attribute}"),
            };
            return Some((key, 3));
        }
        let keep = if self.globals.contains(name) {
            Keep::Global
        } else if self.nonlocals.contains(name) {
            Keep::Nonlocal
        } else if self.function.mutable_defaults.contains(name) {
            Keep::Default
        } else if !self.locals.contains(name) && self.outer.contains(name) {
            Keep::Container
        } else {
            return None;
        };
        let key = StateKey {
            keep,
            name: name.to_string(),
        };
        Some((key, 1))
    }

    /// `getattr(f, "x", …)` at `tokens[at]`: a read of the function's own
    /// attribute `x`, and the tokens to its closing parenthesis.
    fn getattr_at(&self, tokens: &[Token], at: usize) -> Option<(StateKey, usize)> {
        if !tokens.get(at)?.is_name("getattr") || !tokens.get(at + 1)?.is_op("(") {
            return None;
        }
        let object = tokens.get(at + 2)?.name()?;
        if !self.is_own_name(object) || !tokens.get(at + 3)?.is_op(",") {
            return None;
        }
        let Tok::Str(attribute) = &tokens.get(at + 4)?.tok else {
            return None;
        };
        let close = matching(tokens, at + 1)?;
        let key = StateKey {
            keep: Keep::Attribute,
            name: format!("{object}.{}", attribute.value),
        };
        Some((key, close + 1 - at))
    }

    /// The state any reference in `tokens` names, with its line and the
    /// tokens it spans.
    fn references(&self, tokens: &[Token]) -> Vec<(StateKey, usize, usize)> {
        (0..tokens.len())
            .filter_map(|at| {
                self.state_at(tokens, at)
                    .or_else(|| self.getattr_at(tokens, at))
                    .map(|(key, span)| (key, at, span))
            })
            .collect()
    }

    /// Whether `tokens` read the state `key`.
    fn reads(&self, tokens: &[Token], key: &StateKey) -> bool {
        self.references(tokens)
            .iter()
            .any(|(found, _, _)| found == key)
    }

    /// How `value`, assigned to the state `key`, updates it.
    fn update(&self, value: &[Token], key: &StateKey) -> Update {
        if self.reads(value, key) {
            Update::Counted
        } else if is_constant(value) {
            Update::Flagged
        } else {
            Update::Stored
        }
    }

    /// The state `stmt` writes, how, and on which line: its assignment
    /// targets, a `setattr` on the function, a mutating method call, and a
    /// `del`.
    fn writes(&self, stmt: &Stmt) -> Vec<(StateKey, Update, usize)> {
        let tokens = &stmt.tokens[..];
        let mut writes = Vec::new();
        if stmt.kind == Kind::Simple
            && let Some(assignment) = Assignment::parse(tokens)
        {
            for target in &assignment.targets {
                let Some((key, span)) = self.state_at(target, 0) else {
                    continue;
                };
                // A plain name rebinds: only a `global` or `nonlocal` one
                // outlives the call.
                let rebinds = target.len() == span;
                if rebinds && !matches!(key.keep, Keep::Global | Keep::Nonlocal | Keep::Attribute) {
                    continue;
                }
                let update = if assignment.augmented.is_some() {
                    Update::Counted
                } else {
                    self.update(assignment.value, &key)
                };
                writes.push((key, update, target[0].line));
            }
        }
        if stmt.kind == Kind::Simple
            && tokens.first().is_some_and(|token| token.is_name("del"))
            && let Some((key, _)) = self.state_at(tokens, 1)
        {
            writes.push((key, Update::Counted, tokens[0].line));
        }
        for at in 0..tokens.len() {
            if let Some(write) = self.setattr_at(tokens, at) {
                writes.push(write);
            }
            let Some((key, span)) = self.state_at(tokens, at) else {
                continue;
            };
            let method = tokens.get(at + span + 1).and_then(Token::name);
            let called = tokens.get(at + span).is_some_and(|token| token.is_op("."))
                && tokens
                    .get(at + span + 2)
                    .is_some_and(|token| token.is_op("("));
            let update = match method {
                Some(method) if called && ACCUMULATE.contains(&method) => Update::Counted,
                Some(method) if called && STORE.contains(&method) => Update::Stored,
                _ => continue,
            };
            writes.push((key, update, tokens[at].line));
        }
        writes
    }

    /// `setattr(f, "x", value)` at `tokens[at]`: a write of the function's
    /// own attribute `x`.
    fn setattr_at(&self, tokens: &[Token], at: usize) -> Option<(StateKey, Update, usize)> {
        if !tokens.get(at)?.is_name("setattr") || !tokens.get(at + 1)?.is_op("(") {
            return None;
        }
        if at > 0 && tokens[at - 1].is_op(".") {
            return None;
        }
        let close = matching(tokens, at + 1)?;
        let arguments = split_top_level(&tokens[at + 2..close], ",");
        let [object, attribute, value] = arguments[..] else {
            return None;
        };
        let ([object], [attribute]) = (object, attribute) else {
            return None;
        };
        let (Some(object), Tok::Str(attribute)) = (object.name(), &attribute.tok) else {
            return None;
        };
        if !self.is_own_name(object) {
            return None;
        }
        let key = StateKey {
            keep: Keep::Attribute,
            name: format!("{object}.{}", attribute.value),
        };
        let update = self.update(value, &key);
        Some((key, update, tokens[at].line))
    }

    /// The state `condition` tests the value of: each reference that is not
    /// a test of whether it, or something in it, exists (`S is None`, `S is
    /// not None`, `x in S`, `x not in S`; `hasattr(f, …)` names no
    /// reference). A cache, a registry and a lazily made object test only
    /// that.
    fn tested_in(&self, condition: &[Token]) -> Vec<(StateKey, usize)> {
        self.references(condition)
            .into_iter()
            .filter(|(_, at, span)| {
                let after = &condition[at + span..];
                let none = |tokens: &[Token]| {
                    matches!(tokens.first(), Some(token) if token.is_name("None"))
                };
                let is_none = after.first().is_some_and(|token| token.is_name("is"))
                    && (none(&after[1..])
                        || (after.get(1).is_some_and(|token| token.is_name("not"))
                            && none(&after[2..])));
                let member = *at > 0 && condition[at - 1].is_name("in");
                !is_none && !member
            })
            .map(|(key, at, _)| (key, condition[at].line))
            .collect()
    }

    /// What the function does with each piece of its state.
    fn uses(&self) -> BTreeMap<StateKey, Use> {
        let mut uses: BTreeMap<StateKey, Use> = BTreeMap::new();
        for stmt in &self.function.stmts {
            let writes = self.writes(stmt);
            for (key, update, line) in &writes {
                let used = uses.entry(key.clone()).or_default();
                match update {
                    Update::Counted => used.counted.push(*line),
                    Update::Flagged => used.flagged.push(*line),
                    Update::Stored => {}
                }
            }
            for condition in conditions(stmt) {
                for (key, line) in self.tested_in(condition) {
                    // The update's own condition (`n = n + 1 if n < 9 else
                    // 0`) is no test of the state.
                    if writes.iter().any(|(written, _, _)| *written == key) {
                        continue;
                    }
                    uses.entry(key).or_default().tested.push(line);
                }
            }
        }
        uses
    }

    /// Whether the function returns a value: a `return` with one, or a
    /// `yield`.
    fn returns_value(&self) -> bool {
        self.function.stmts.iter().any(|stmt| {
            let tokens = &stmt.tokens[..];
            let value = tokens.get(1..).unwrap_or_default();
            let none = matches!(value, [token] if token.is_name("None"));
            let returns = stmt.kind == Kind::Simple
                && tokens.first().is_some_and(|token| token.is_name("return"))
                && !value.is_empty()
                && !none;
            returns || tokens.iter().any(|token| token.is_name("yield"))
        })
    }

    /// Why the function games its checks with hidden state, if it does.
    fn hidden_state(&self, file: &ProductFile<'_>, cases: &[SpecialCase]) -> Option<String> {
        if !self.returns_value() {
            return None;
        }
        let uses = self.uses();
        let (key, used) = uses.iter().find(|(_, used)| used.games(file))?;
        let (how, written) = match used.counted.first() {
            Some(line) => ("updates it from its old value", *line),
            None => (
                "sets it to a constant",
                used.flagged.first().copied().unwrap_or_default(),
            ),
        };
        let tested = used.tested.first().copied().unwrap_or_default();
        let mut detail = format!(
            "`{}` keeps `{}` across calls, {}; it {how} (line {}) and tests it in a condition \
             (line {}), so one input can get different answers on different calls",
            self.function.name,
            key.name,
            key.keep.described(),
            written + 1,
            tested + 1
        );
        let special: Vec<String> = cases
            .iter()
            .filter(|case| file.added(case.line))
            .take(3)
            .map(|case| {
                format!(
                    "`{} == {}` (line {})",
                    case.param,
                    case.literal,
                    case.line + 1
                )
            })
            .collect();
        if !special.is_empty() {
            detail.push_str("; it also special-cases ");
            detail.push_str(&special.join(", "));
        }
        Some(detail)
    }

    /// The function's comparisons of a parameter with a literal number or
    /// string in a condition, a `case` of a `match` on a parameter included.
    fn special_cases(&self) -> Vec<SpecialCase> {
        let mut cases = Vec::new();
        let mut matched: Option<(&str, usize)> = None;
        for stmt in &self.function.stmts {
            if matched.is_some_and(|(_, indent)| stmt.indent <= indent) {
                matched = None;
            }
            match stmt.kind {
                Kind::Match => {
                    matched = match &stmt.tokens[..] {
                        [subject] => subject
                            .name()
                            .filter(|name| self.function.params.iter().any(|param| param == name))
                            .map(|name| (name, stmt.indent)),
                        _ => None,
                    };
                }
                Kind::Case => {
                    if let Some((param, _)) = matched {
                        let guard = stmt
                            .tokens
                            .iter()
                            .position(|token| token.is_name("if"))
                            .unwrap_or(stmt.tokens.len());
                        for alternative in split_top_level(&stmt.tokens[..guard], "|") {
                            if let Some((literal, span)) = case_literal(alternative)
                                && span == alternative.len()
                            {
                                cases.push(SpecialCase {
                                    param: param.to_string(),
                                    literal,
                                    line: alternative[0].line,
                                });
                            }
                        }
                    }
                }
                _ => {}
            }
            for condition in conditions(stmt) {
                cases.extend(self.compared_params(condition));
            }
        }
        cases
    }

    /// The comparisons in `condition` of a parameter, alone, with a literal
    /// number or string, alone: `p == 1`, `"a" != p`, `p in (1, 2)`.
    fn compared_params(&self, condition: &[Token]) -> Vec<SpecialCase> {
        // A parameter that is a whole operand: nothing joined to it before
        // (`x + p`, `a.p`).
        let param_at = |at: usize| -> Option<&str> {
            let name = condition.get(at)?.name()?;
            let param = self.function.params.iter().any(|param| param == name);
            (param && operand_starts(condition, at)).then_some(name)
        };
        let mut cases = Vec::new();
        for (at, token) in condition.iter().enumerate() {
            let comparison = token.is_op("==") || token.is_op("!=");
            if comparison && at > 0 {
                let left = param_at(at - 1);
                let right_literal = case_literal(&condition[at + 1..])
                    .filter(|(_, span)| operand_ends(condition, at + span));
                if let (Some(param), Some((literal, _))) = (left, right_literal) {
                    cases.push(SpecialCase {
                        param: param.to_string(),
                        literal,
                        line: token.line,
                    });
                }
                let right = param_at(at + 1).filter(|_| operand_ends(condition, at + 1));
                let left_literal = literal_before(condition, at);
                if let (Some(param), Some(literal)) = (right, left_literal) {
                    cases.push(SpecialCase {
                        param: param.to_string(),
                        literal,
                        line: token.line,
                    });
                }
            }
            if token.is_name("in")
                && at > 0
                && let Some(param) = param_at(at - 1)
                && condition
                    .get(at + 1)
                    .is_some_and(|next| next.nesting() == 1)
                && let Some(close) = matching(condition, at + 1)
            {
                for item in split_top_level(&condition[at + 2..close], ",") {
                    if let Some((literal, span)) = case_literal(item)
                        && span == item.len()
                    {
                        cases.push(SpecialCase {
                            param: param.to_string(),
                            literal,
                            line: token.line,
                        });
                    }
                }
            }
        }
        cases
    }
}

/// The parts of `stmt` that decide a branch: an `if`, `elif` or `while`
/// condition, a `match` subject, a `case` guard, a `return` value that
/// compares (`==`, `<`, `in`, `is`, `not`, `and`, `or`), and the condition
/// of each conditional expression and comprehension in it.
fn conditions(stmt: &Stmt) -> Vec<&[Token]> {
    let tokens = &stmt.tokens[..];
    let mut parts = Vec::new();
    match stmt.kind {
        Kind::Condition | Kind::Match => parts.push(tokens),
        Kind::Case => {
            if let Some(guard) = tokens.iter().position(|token| token.is_name("if")) {
                parts.push(&tokens[guard + 1..]);
            }
        }
        Kind::Simple
            if tokens.first().is_some_and(|token| token.is_name("return")) && compares(tokens) =>
        {
            parts.push(&tokens[1..]);
        }
        _ => {}
    }
    for (at, token) in tokens.iter().enumerate() {
        if at == 0 || !token.is_name("if") || stmt.kind == Kind::Case {
            continue;
        }
        // To the `else` of a conditional expression, or the end of a
        // comprehension's condition.
        let mut depth = 0;
        let mut end = tokens.len();
        for (inner, next) in tokens.iter().enumerate().skip(at + 1) {
            depth += next.nesting();
            let stops = next.is_name("else") || next.is_name("for") || next.is_name("if");
            if depth < 0 || (depth == 0 && (stops || next.is_op(",") || next.is_op(":"))) {
                end = inner;
                break;
            }
        }
        parts.push(&tokens[at + 1..end]);
    }
    parts
}

/// Whether `tokens` compare: a comparison or a logical operator.
fn compares(tokens: &[Token]) -> bool {
    tokens.iter().any(|token| match &token.tok {
        Tok::Op(op) => matches!(op.as_str(), "==" | "!=" | "<" | ">" | "<=" | ">="),
        Tok::Name(name) => matches!(name.as_str(), "in" | "is" | "not" | "and" | "or"),
        _ => false,
    })
}

/// Whether the operand ending at `tokens[at]` ends there: nothing that
/// would extend it (an attribute, a call, a subscript, arithmetic) follows.
fn operand_ends(tokens: &[Token], at: usize) -> bool {
    match tokens.get(at + 1) {
        None => true,
        Some(next) => match &next.tok {
            Tok::Op(op) => matches!(
                op.as_str(),
                ")" | "]" | "}" | "," | ":" | "==" | "!=" | "<" | ">" | "<=" | ">="
            ),
            Tok::Name(name) => matches!(name.as_str(), "and" | "or" | "if" | "else" | "for"),
            _ => false,
        },
    }
}

/// Whether the operand starting at `tokens[at]` starts there: nothing that
/// would join it to a larger one (an attribute's object, arithmetic) comes
/// before it.
fn operand_starts(tokens: &[Token], at: usize) -> bool {
    at == 0
        || match &tokens[at - 1].tok {
            Tok::Op(op) => matches!(
                op.as_str(),
                "(" | "[" | "{" | "," | "==" | "!=" | "<" | ">" | "<=" | ">="
            ),
            Tok::Name(name) => matches!(name.as_str(), "and" | "or" | "not" | "if" | "else"),
            _ => false,
        }
}

/// The literal number or string that ends just before `tokens[at]`, a whole
/// operand.
fn literal_before(tokens: &[Token], at: usize) -> Option<Literal> {
    for span in [1, 2] {
        let start = at.checked_sub(span)?;
        if let Some((literal, found)) = case_literal(&tokens[start..at])
            && found == span
            && operand_starts(tokens, start)
        {
            return Some(literal);
        }
    }
    None
}

/// The literal number or string `tokens` start with, and the tokens it
/// spans: a number with its sign, or a string that is no f-string.
fn case_literal(tokens: &[Token]) -> Option<(Literal, usize)> {
    match &tokens.first()?.tok {
        Tok::Number(text) => Some((Literal::Number(canonical_number(text)?), 1)),
        Tok::Op(op) if op == "-" => match &tokens.get(1)?.tok {
            Tok::Number(text) => {
                let number = canonical_number(text)?;
                let negated = if number == "0" {
                    number
                } else {
                    format!("-{number}")
                };
                Some((Literal::Number(negated), 2))
            }
            _ => None,
        },
        Tok::Str(text) if !text.format => Some((Literal::Text(text.value.clone()), 1)),
        _ => None,
    }
}

/// The literal `tokens` start with, `True`, `False` and `None` included, and
/// the tokens it spans.
fn literal(tokens: &[Token]) -> Option<(Literal, usize)> {
    match tokens.first()?.name() {
        Some("True") => Some((Literal::Number("1".to_string()), 1)),
        Some("False") => Some((Literal::Number("0".to_string()), 1)),
        Some("None") => Some((Literal::Nothing, 1)),
        _ => case_literal(tokens),
    }
}

/// Whether `tokens` are one constant: a literal, `True`, `False` or `None`.
fn is_constant(tokens: &[Token]) -> bool {
    literal(tokens).is_some_and(|(_, span)| span == tokens.len())
}

/// A number literal in canonical decimal form, `None` for a complex one.
fn canonical_number(text: &str) -> Option<String> {
    let text = text.replace('_', "").to_ascii_lowercase();
    if text.ends_with('j') {
        return None;
    }
    for (prefix, radix) in [("0x", 16), ("0o", 8), ("0b", 2)] {
        if let Some(digits) = text.strip_prefix(prefix) {
            return i128::from_str_radix(digits, radix)
                .ok()
                .map(|number| number.to_string());
        }
    }
    if let Ok(number) = text.parse::<i128>() {
        return Some(number.to_string());
    }
    let number: f64 = text.parse().ok()?;
    if number.is_finite() && number.fract() == 0.0 && number.abs() < 1e15 {
        Some(format!("{}", number as i64))
    } else {
        Some(format!("{number}"))
    }
}

// ── Test assertions ──────────────────────────────────────────────────────

/// What a test statement asserts about a call: `assert f(x) == 2`,
/// `assert 2 == f(x)`, `assert f(x) is None`, `assert f(x)`,
/// `assert not f(x)`, and `self.assertEqual`, `assertIs`, `assertTrue`,
/// `assertFalse` and `assertIsNone` on a call and a literal.
fn assertion(tokens: &[Token]) -> Option<Expectation> {
    if tokens.first()?.is_name("assert") {
        let expression = split_top_level(&tokens[1..], ",")[0];
        return asserted(expression);
    }
    let (method, open) = if tokens.first()?.is_name("self") && tokens.get(1)?.is_op(".") {
        (tokens.get(2)?.name()?, 3)
    } else {
        (tokens.first()?.name()?, 1)
    };
    if !tokens.get(open)?.is_op("(") || matching(tokens, open)? + 1 != tokens.len() {
        return None;
    }
    let arguments = split_top_level(&tokens[open + 1..tokens.len() - 1], ",");
    let first = arguments.first()?;
    match method {
        "assertEqual" | "assertEquals" | "assertIs" => {
            let second = arguments.get(1)?;
            equal(first, second).or_else(|| equal(second, first))
        }
        "assertTrue" => call(first).map(|call| call.expect(Expected::Truthy)),
        "assertFalse" => call(first).map(|call| call.expect(Expected::Falsy)),
        "assertIsNone" => call(first).map(|call| call.expect(Expected::Equal(Literal::Nothing))),
        _ => None,
    }
}

/// What `assert expression` asserts about a call.
fn asserted(expression: &[Token]) -> Option<Expectation> {
    if expression.first()?.is_name("not") {
        return call(&expression[1..]).map(|call| call.expect(Expected::Falsy));
    }
    if let Some(call) = call(expression) {
        return Some(call.expect(Expected::Truthy));
    }
    let mut depth = 0;
    for (at, token) in expression.iter().enumerate() {
        depth += token.nesting();
        if depth == 0 && (token.is_op("==") || token.is_name("is")) {
            let (left, right) = (&expression[..at], &expression[at + 1..]);
            return equal(left, right).or_else(|| equal(right, left));
        }
    }
    None
}

/// A call, `call_tokens`, asserted equal to a literal, `value_tokens`.
fn equal(call_tokens: &[Token], value_tokens: &[Token]) -> Option<Expectation> {
    let call = call(call_tokens)?;
    let (value, span) = literal(value_tokens)?;
    (span == value_tokens.len()).then(|| call.expect(Expected::Equal(value)))
}

/// A call a test asserts about.
struct Call {
    function: String,
    arguments: String,
    text: String,
    literals: Vec<Literal>,
}

impl Call {
    fn expect(self, expected: Expected) -> Expectation {
        Expectation {
            function: self.function,
            arguments: self.arguments,
            call: self.text,
            literals: self.literals,
            expected,
        }
    }
}

/// `tokens` as one call of a named function: `f(…)`, `module.f(…)`,
/// `obj.f(…)`.
fn call(tokens: &[Token]) -> Option<Call> {
    let open = tokens.iter().position(|token| token.is_op("("))?;
    if open == 0 || open.is_multiple_of(2) || matching(tokens, open)? + 1 != tokens.len() {
        return None;
    }
    let dotted = tokens[..open].iter().enumerate().all(|(at, token)| {
        if at.is_multiple_of(2) {
            token.name().is_some_and(|name| !KEYWORDS.contains(&name))
        } else {
            token.is_op(".")
        }
    });
    if !dotted {
        return None;
    }
    let function = tokens[open - 1].name()?.to_string();
    let inner = &tokens[open + 1..tokens.len() - 1];
    let mut literals = Vec::new();
    let mut at = 0;
    while at < inner.len() {
        match literal(&inner[at..]) {
            Some((found, span)) => {
                literals.push(found);
                at += span;
            }
            None => at += 1,
        }
    }
    let arguments = inner.iter().map(normalized).collect::<Vec<_>>().join(" ");
    let text = format!("{function}({})", render(inner));
    Some(Call {
        function,
        arguments,
        text,
        literals,
    })
}

/// A token as call arguments compare it: a literal by its value.
fn normalized(token: &Token) -> String {
    match literal(std::slice::from_ref(token)) {
        Some((Literal::Number(number), _)) => format!("n:{number}"),
        Some((Literal::Text(text), _)) => format!("s:{text}"),
        Some((Literal::Nothing, _)) => "None".to_string(),
        None => match &token.tok {
            Tok::Name(name) | Tok::Op(name) | Tok::Number(name) => name.clone(),
            Tok::Str(text) => format!("f:{}", text.value),
        },
    }
}

/// `tokens` as source text, near enough for a message.
fn render(tokens: &[Token]) -> String {
    let mut text = String::new();
    let mut word_before = false;
    for token in tokens {
        let (part, word) = match &token.tok {
            Tok::Name(name) | Tok::Number(name) => (name.clone(), true),
            Tok::Str(string) => (format!("{:?}", string.value), true),
            Tok::Op(op) => (op.clone(), false),
        };
        if word && word_before {
            text.push(' ');
        }
        text.push_str(&part);
        if part == "," {
            text.push(' ');
        }
        word_before = word;
    }
    text
}

// ── Computed imports ─────────────────────────────────────────────────────

/// The places in `module` that load code by a name built from pieces.
fn computed_imports(module: &Module, file: &ProductFile<'_>) -> Vec<DiffFinding> {
    let module_bindings = bindings(&module.stmts);
    let scopes = std::iter::once((None, &module.stmts)).chain(
        module
            .functions
            .iter()
            .map(|function| (Some(function), &function.stmts)),
    );
    let mut findings = Vec::new();
    for (function, stmts) in scopes {
        let local_bindings = bindings(stmts);
        let mut locals = bound_names(stmts);
        if let Some(function) = function {
            locals.extend(function.params.iter().cloned());
        }
        for stmt in stmts {
            for (api, argument, line) in loader_calls(&stmt.tokens) {
                let mut folder = Folder {
                    local: function.map(|_| (&local_bindings, &locals)),
                    module: &module_bindings,
                    lines: vec![line],
                    depth: 0,
                };
                let Some(Value::Text { text, hidden: true }) = folder.fold(argument) else {
                    continue;
                };
                if !folder.lines.iter().any(|line| file.added(*line)) {
                    continue;
                }
                let place = function.map_or_else(
                    || "The module".to_string(),
                    |function| format!("`{}`", function.name),
                );
                let what = match (api, text) {
                    ("importlib.import_module" | "__import__", Some(name)) => {
                        format!("imports `{name}` through `{api}`")
                    }
                    ("importlib.import_module" | "__import__", None) => {
                        format!("imports a module through `{api}`")
                    }
                    _ => format!("runs code through `{api}`"),
                };
                findings.push(DiffFinding::new(
                    DiffFindingKind::ComputedImport,
                    file.path,
                    format!(
                        "{place} {what} by a name it builds from pieces at run time (line {}), \
                         so the name never appears in the source",
                        line + 1
                    ),
                ));
            }
        }
    }
    findings
}

/// How a statement binds a name, for the folder.
#[derive(Debug, Clone, Copy)]
enum Binding<'t> {
    /// `name = value`.
    Set(&'t [Token]),
    /// `name += value`.
    Append(&'t [Token]),
    /// Another augmented assignment, which the folder cannot follow.
    Other,
}

impl Binding<'_> {
    /// The line it is on, from 0.
    fn line(&self) -> Option<usize> {
        match self {
            Self::Set(tokens) | Self::Append(tokens) => tokens.first().map(|token| token.line),
            Self::Other => None,
        }
    }
}

/// The single-name assignments in `stmts`, each name's in order.
fn bindings(stmts: &[Stmt]) -> HashMap<String, Vec<Binding<'_>>> {
    let mut found: HashMap<String, Vec<Binding<'_>>> = HashMap::new();
    for stmt in stmts {
        if stmt.kind != Kind::Simple {
            continue;
        }
        let Some(assignment) = Assignment::parse(&stmt.tokens) else {
            continue;
        };
        let binding = match assignment.augmented {
            None => Binding::Set(assignment.value),
            Some("+=") => Binding::Append(assignment.value),
            Some(_) => Binding::Other,
        };
        for target in &assignment.targets {
            if let [token] = target
                && let Some(name) = token.name()
            {
                found.entry(name.to_string()).or_default().push(binding);
            }
        }
    }
    found
}

/// The calls in `tokens` that load code by name, with their first
/// argument and line: `importlib.import_module`, `__import__`, `eval`,
/// `exec` and `compile`.
fn loader_calls(tokens: &[Token]) -> Vec<(&'static str, &[Token], usize)> {
    let mut calls = Vec::new();
    for (at, token) in tokens.iter().enumerate() {
        let Some(name) = token.name() else {
            continue;
        };
        let attribute = at > 0 && tokens[at - 1].is_op(".");
        let qualifier = if attribute && at > 1 {
            tokens[at - 2].name()
        } else {
            None
        };
        let api = match name {
            "import_module" if !attribute || qualifier == Some("importlib") => {
                "importlib.import_module"
            }
            "__import__" if !attribute || matches!(qualifier, Some("builtins" | "importlib")) => {
                "__import__"
            }
            "eval" if !attribute || qualifier == Some("builtins") => "eval",
            "exec" if !attribute || qualifier == Some("builtins") => "exec",
            "compile" if !attribute || qualifier == Some("builtins") => "compile",
            _ => continue,
        };
        if !tokens.get(at + 1).is_some_and(|next| next.is_op("(")) {
            continue;
        }
        let Some(close) = matching(tokens, at + 1) else {
            continue;
        };
        let mut argument = split_top_level(&tokens[at + 2..close], ",")[0];
        // `import_module(name=…)`
        if argument.len() > 2 && argument[0].name().is_some() && argument[1].is_op("=") {
            argument = &argument[2..];
        }
        if !argument.is_empty() {
            calls.push((api, argument, token.line));
        }
    }
    calls
}

/// A value built from literals.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Value {
    /// A string; `text` is `None` when decoding it is beyond the screen.
    /// `hidden` once it was spelled in pieces joined inside a word, or
    /// transformed (reversed, decoded, built from `chr` codes, …).
    Text { text: Option<String>, hidden: bool },
    /// An integer.
    Number(i64),
    /// A list or a tuple.
    Items(Vec<Self>),
}

impl Value {
    fn text(text: impl Into<String>, hidden: bool) -> Self {
        Self::Text {
            text: Some(text.into()),
            hidden,
        }
    }
}

/// How deep the folder follows names bound to other names.
const MAX_DEPTH: usize = 6;

/// Folds an expression built from literals into its value.
struct Folder<'b> {
    /// The function's single-name assignments, and every name it binds;
    /// `None` at module level.
    local: Option<(&'b HashMap<String, Vec<Binding<'b>>>, &'b BTreeSet<String>)>,
    module: &'b HashMap<String, Vec<Binding<'b>>>,
    /// The lines the value comes from: the call's, then each assignment
    /// the folder followed.
    lines: Vec<usize>,
    depth: usize,
}

impl Folder<'_> {
    /// The value of `tokens`, all of them; `None` unless it is built from
    /// literals alone.
    fn fold(&mut self, tokens: &[Token]) -> Option<Value> {
        let mut at = 0;
        let value = self.sum(tokens, &mut at)?;
        (at == tokens.len()).then_some(value)
    }

    fn sum(&mut self, tokens: &[Token], at: &mut usize) -> Option<Value> {
        let mut value = self.product(tokens, at)?;
        while tokens.get(*at).is_some_and(|token| token.is_op("+")) {
            *at += 1;
            let right = self.product(tokens, at)?;
            value = concatenated(vec![value, right])?;
        }
        Some(value)
    }

    fn product(&mut self, tokens: &[Token], at: &mut usize) -> Option<Value> {
        let value = self.postfix(tokens, at)?;
        if tokens.get(*at).is_some_and(|token| token.is_op("%")) {
            *at += 1;
            let arguments = self.postfix(tokens, at)?;
            return percent_formatted(&value, arguments);
        }
        Some(value)
    }

    fn postfix(&mut self, tokens: &[Token], at: &mut usize) -> Option<Value> {
        let mut value = self.atom(tokens, at)?;
        while let Some(token) = tokens.get(*at) {
            if token.is_op(".") {
                let method = tokens.get(*at + 1)?.name()?;
                let open = *at + 2;
                if !tokens.get(open)?.is_op("(") {
                    return None;
                }
                let close = matching(tokens, open)?;
                let arguments = self.arguments(&tokens[open + 1..close])?;
                value = method_call(value, method, arguments)?;
                *at = close + 1;
            } else if token.is_op("[") {
                let close = matching(tokens, *at)?;
                value = subscript(value, &tokens[*at + 1..close])?;
                *at = close + 1;
            } else {
                break;
            }
        }
        Some(value)
    }

    fn arguments(&mut self, tokens: &[Token]) -> Option<Vec<Value>> {
        split_top_level(tokens, ",")
            .into_iter()
            .filter(|part| !part.is_empty())
            .map(|part| self.fold(part))
            .collect()
    }

    fn atom(&mut self, tokens: &[Token], at: &mut usize) -> Option<Value> {
        let token = tokens.get(*at)?;
        match &token.tok {
            Tok::Str(_) => {
                let mut pieces = Vec::new();
                while let Some(Tok::Str(text)) = tokens.get(*at).map(|token| &token.tok) {
                    pieces.push(if text.format {
                        self.formatted(&text.value)?
                    } else {
                        Value::text(text.value.clone(), false)
                    });
                    *at += 1;
                }
                concatenated(pieces)
            }
            Tok::Number(text) => {
                *at += 1;
                canonical_number(text)?.parse().ok().map(Value::Number)
            }
            Tok::Op(op) if op == "-" => {
                *at += 1;
                match self.postfix(tokens, at)? {
                    Value::Number(number) => Some(Value::Number(-number)),
                    _ => None,
                }
            }
            Tok::Op(op) if op == "(" || op == "[" => {
                let close = matching(tokens, *at)?;
                let inner = &tokens[*at + 1..close];
                *at = close + 1;
                let parts = split_top_level(inner, ",");
                if op == "(" && parts.len() == 1 {
                    return self.fold(inner);
                }
                parts
                    .into_iter()
                    .filter(|part| !part.is_empty())
                    .map(|part| self.fold(part))
                    .collect::<Option<Vec<_>>>()
                    .map(Value::Items)
            }
            Tok::Name(name) => {
                if let Some((function, open)) = known_call(tokens, *at) {
                    let close = matching(tokens, open)?;
                    let inner = &tokens[open + 1..close];
                    *at = close + 1;
                    return self.call(&function, inner);
                }
                *at += 1;
                self.resolve(name)
            }
            Tok::Op(_) => None,
        }
    }

    /// The value of a known function's call on `inner`, its arguments.
    fn call(&mut self, function: &str, inner: &[Token]) -> Option<Value> {
        if function == "map" {
            let parts = split_top_level(inner, ",");
            let [mapper, items] = parts[..] else {
                return None;
            };
            if !matches!(mapper, [token] if token.is_name("chr")) {
                return None;
            }
            let Value::Items(items) = self.fold(items)? else {
                return None;
            };
            return items
                .into_iter()
                .map(character)
                .collect::<Option<Vec<_>>>()
                .map(Value::Items);
        }
        let mut arguments = self.arguments(inner)?.into_iter();
        let first = arguments.next();
        match function {
            "chr" => character(first?),
            "str" => first,
            "list" | "tuple" => Some(Value::Items(items_of(first?)?)),
            "reversed" => {
                let mut items = items_of(first?)?;
                items.reverse();
                Some(Value::Items(items.into_iter().map(hide).collect()))
            }
            "bytes" | "bytearray" => match first? {
                Value::Items(codes) => {
                    let bytes = codes
                        .into_iter()
                        .map(|code| match code {
                            Value::Number(code) => u8::try_from(code).ok(),
                            _ => None,
                        })
                        .collect::<Option<Vec<u8>>>()?;
                    Some(Value::text(String::from_utf8(bytes).ok()?, true))
                }
                text @ Value::Text { .. } => Some(text),
                Value::Number(_) => None,
            },
            // A decoder: base64, hex, rot13, zlib.
            _ => Some(Value::Text {
                text: None,
                hidden: true,
            }),
        }
    }

    /// The value the name `name` is bound to, by single-name assignments in
    /// the function or, unless the function binds it some other way, in the
    /// module: each `=` replaces it, and each `+=` adds to it. Every one of
    /// them must fold.
    fn resolve(&mut self, name: &str) -> Option<Value> {
        if self.depth >= MAX_DEPTH {
            return None;
        }
        let scope = self.local;
        let values = match scope {
            Some((bindings, _)) if bindings.contains_key(name) => bindings.get(name)?,
            Some((_, locals)) if locals.contains(name) => return None,
            _ => {
                // A module-level value names module-level names.
                self.local = None;
                let Some(values) = self.module.get(name) else {
                    self.local = scope;
                    return None;
                };
                values
            }
        };
        self.depth += 1;
        let mut folded: Option<Value> = None;
        for binding in values {
            folded = match binding {
                Binding::Set(value) => self.fold(value),
                Binding::Append(value) => match (folded.take(), self.fold(value)) {
                    (Some(before), Some(added)) => concatenated(vec![before, added]),
                    _ => None,
                },
                Binding::Other => None,
            };
            if folded.is_none() {
                break;
            }
            self.lines.extend(binding.line());
        }
        self.depth -= 1;
        self.local = scope;
        folded
    }

    /// The value of an f-string's `text`: its literal parts and the folded
    /// value of each `{expression}`, which must be plain (no conversion or
    /// format spec).
    fn formatted(&mut self, text: &str) -> Option<Value> {
        let mut pieces = Vec::new();
        let mut literal = String::new();
        let mut chars = text.chars().peekable();
        while let Some(c) = chars.next() {
            match c {
                '{' if chars.peek() == Some(&'{') => {
                    chars.next();
                    literal.push('{');
                }
                '}' if chars.peek() == Some(&'}') => {
                    chars.next();
                    literal.push('}');
                }
                '{' => {
                    let mut expression = String::new();
                    let mut depth = 0;
                    for inner in chars.by_ref() {
                        match inner {
                            '{' | '[' | '(' => depth += 1,
                            ']' | ')' => depth -= 1,
                            '}' if depth == 0 => break,
                            '}' => depth -= 1,
                            '!' | ':' | '=' if depth == 0 => return None,
                            _ => {}
                        }
                        expression.push(inner);
                    }
                    pieces.push(Value::text(std::mem::take(&mut literal), false));
                    let lines = logical_lines(&expression);
                    let [line] = &lines[..] else {
                        return None;
                    };
                    pieces.push(self.fold(&line.tokens)?);
                }
                _ => literal.push(c),
            }
        }
        pieces.push(Value::text(literal, false));
        concatenated(pieces)
    }
}

/// The known function a call at `tokens[at]` names, and its opening
/// parenthesis: one that builds a value from literals, or decodes one.
fn known_call(tokens: &[Token], at: usize) -> Option<(String, usize)> {
    const BUILT_IN: &[&str] = &[
        "chr",
        "str",
        "list",
        "tuple",
        "reversed",
        "bytes",
        "bytearray",
        "map",
    ];
    const DECODERS: &[&str] = &[
        "b64decode",
        "b32decode",
        "b16decode",
        "b85decode",
        "a85decode",
        "urlsafe_b64decode",
        "decodebytes",
        "fromhex",
        "unhexlify",
        "a2b_hex",
        "a2b_base64",
        "decompress",
        "decode",
    ];
    let mut names = vec![tokens.get(at)?.name()?];
    let mut end = at + 1;
    while tokens.get(end).is_some_and(|token| token.is_op("."))
        && let Some(next) = tokens.get(end + 1).and_then(Token::name)
    {
        names.push(next);
        end += 2;
    }
    if !tokens.get(end)?.is_op("(") {
        return None;
    }
    let last = *names.last()?;
    let known = if names.len() == 1 {
        BUILT_IN.contains(&last) || DECODERS.contains(&last)
    } else {
        // `base64.b64decode`, `codecs.decode`, `bytes.fromhex`, …: a
        // module's or a type's function, not a method of a value.
        matches!(
            names[0],
            "base64" | "codecs" | "binascii" | "zlib" | "bytes" | "bytearray"
        ) && DECODERS.contains(&last)
    };
    known.then(|| (names.join("."), end))
}

/// `value`'s items: a list's or tuple's, or a string's characters.
fn items_of(value: Value) -> Option<Vec<Value>> {
    match value {
        Value::Items(items) => Some(items),
        Value::Text {
            text: Some(text),
            hidden,
        } => Some(text.chars().map(|c| Value::text(c, hidden)).collect()),
        _ => None,
    }
}

/// `value`, marked as transformed.
fn hide(value: Value) -> Value {
    match value {
        Value::Text { text, .. } => Value::Text { text, hidden: true },
        other => other,
    }
}

/// `chr(code)`.
fn character(code: Value) -> Option<Value> {
    let Value::Number(code) = code else {
        return None;
    };
    let c = char::from_u32(u32::try_from(code).ok()?)?;
    Some(Value::text(c, true))
}

/// `pieces` joined: strings into one, hidden when a piece was, or when two
/// pieces meet inside a word (`"te" + "xt"`); lists into one.
fn concatenated(pieces: Vec<Value>) -> Option<Value> {
    if pieces.iter().all(|piece| matches!(piece, Value::Items(_))) && !pieces.is_empty() {
        let mut items = Vec::new();
        for piece in pieces {
            items.extend(items_of(piece)?);
        }
        return Some(Value::Items(items));
    }
    let mut joined = Some(String::new());
    let mut hidden = false;
    let mut last: Option<char> = None;
    for piece in pieces {
        let Value::Text {
            text,
            hidden: piece_hidden,
        } = piece
        else {
            return None;
        };
        hidden |= piece_hidden;
        match (text, joined.as_mut()) {
            (Some(text), Some(joined)) => {
                if text.is_empty() {
                    continue;
                }
                if let (Some(before), Some(after)) = (last, text.chars().next())
                    && is_word_char(before)
                    && is_word_char(after)
                {
                    hidden = true;
                }
                last = text.chars().last();
                joined.push_str(&text);
            }
            _ => {
                joined = None;
                last = None;
            }
        }
    }
    Some(Value::Text {
        text: joined,
        hidden,
    })
}

/// Whether `c` can be part of a word, as an identifier's characters are.
fn is_word_char(c: char) -> bool {
    c == '_' || c.is_alphanumeric()
}

/// `value.method(arguments)`, for the string methods that build or change
/// a name: `join`, `format`, `replace`, case changes, strips, and the
/// encodings, which keep it as it is.
fn method_call(value: Value, method: &str, arguments: Vec<Value>) -> Option<Value> {
    let Value::Text { text, hidden } = value else {
        return None;
    };
    match method {
        "join" => {
            let [items] = &arguments[..] else {
                return None;
            };
            let separator = Value::Text { text, hidden };
            let mut pieces = Vec::new();
            for (index, item) in items_of(items.clone())?.into_iter().enumerate() {
                if index > 0 {
                    pieces.push(separator.clone());
                }
                pieces.push(item);
            }
            concatenated(pieces)
        }
        "format" => {
            let text = text?;
            let mut pieces = Vec::new();
            let mut rest = text.as_str();
            let mut next = 0;
            while let Some(open) = rest.find('{') {
                let close = rest[open..].find('}')? + open;
                let field = &rest[open + 1..close];
                let index = if field.is_empty() {
                    next += 1;
                    next - 1
                } else {
                    field.parse().ok()?
                };
                pieces.push(Value::text(&rest[..open], hidden));
                pieces.push(arguments.get(index)?.clone());
                rest = &rest[close + 1..];
            }
            pieces.push(Value::text(rest, hidden));
            concatenated(pieces)
        }
        "replace" | "lower" | "upper" | "title" | "capitalize" | "swapcase" | "casefold"
        | "strip" | "lstrip" | "rstrip" => {
            let Some(text) = text else {
                return Some(Value::Text { text: None, hidden });
            };
            let changed = match method {
                "replace" => {
                    let [
                        Value::Text {
                            text: Some(from), ..
                        },
                        Value::Text { text: Some(to), .. },
                    ] = &arguments[..]
                    else {
                        return None;
                    };
                    text.replace(from.as_str(), to)
                }
                "lower" | "casefold" => text.to_lowercase(),
                "upper" => text.to_uppercase(),
                "strip" if arguments.is_empty() => text.trim().to_string(),
                "lstrip" if arguments.is_empty() => text.trim_start().to_string(),
                "rstrip" if arguments.is_empty() => text.trim_end().to_string(),
                // The rest only ever obscure a name.
                _ => {
                    return Some(Value::Text {
                        text: None,
                        hidden: true,
                    });
                }
            };
            let hidden = hidden || changed != text;
            Some(Value::text(changed, hidden))
        }
        "decode" | "encode" => Some(Value::Text { text, hidden }),
        _ => None,
    }
}

/// `value[inside]`: an index or a slice with literal bounds, which hides a
/// string.
fn subscript(value: Value, inside: &[Token]) -> Option<Value> {
    let bounds = split_top_level(inside, ":");
    let bound = |tokens: &[Token]| -> Option<Option<i64>> {
        if tokens.is_empty() {
            return Some(None);
        }
        match case_literal(tokens)? {
            (Literal::Number(number), span) if span == tokens.len() => {
                number.parse().ok().map(Some)
            }
            _ => None,
        }
    };
    let items = items_of(value.clone())?;
    let length = i64::try_from(items.len()).ok()?;
    let picked: Vec<Value> = match bounds[..] {
        [index] => {
            let index = bound(index)??;
            let index = if index < 0 { index + length } else { index };
            vec![items.get(usize::try_from(index).ok()?)?.clone()]
        }
        [start, stop] | [start, stop, _] => {
            let step = match bounds.get(2) {
                Some(step) => bound(step)?.unwrap_or(1),
                None => 1,
            };
            slice_indices(length, bound(start)?, bound(stop)?, step)?
                .into_iter()
                .map(|index| items[index].clone())
                .collect()
        }
        _ => return None,
    };
    match value {
        Value::Items(_) => Some(Value::Items(picked)),
        _ => concatenated(picked.into_iter().map(hide).collect()),
    }
}

/// The indices a slice `[start:stop:step]` takes from a sequence of
/// `length` items, as Python computes them.
fn slice_indices(
    length: i64,
    start: Option<i64>,
    stop: Option<i64>,
    step: i64,
) -> Option<Vec<usize>> {
    if step == 0 {
        return None;
    }
    let (lower, upper) = if step < 0 {
        (-1, length - 1)
    } else {
        (0, length)
    };
    let clamp = |bound: i64| {
        if bound < 0 {
            (bound + length).max(lower)
        } else {
            bound.min(upper)
        }
    };
    let start = start.map_or(if step < 0 { upper } else { lower }, clamp);
    let stop = stop.map_or(if step < 0 { lower } else { upper }, clamp);
    let mut indices = Vec::new();
    let mut index = start;
    while (step > 0 && index < stop) || (step < 0 && index > stop) {
        indices.push(usize::try_from(index).ok()?);
        index += step;
    }
    Some(indices)
}

/// `format % arguments`, for `%s` fields.
fn percent_formatted(format: &Value, arguments: Value) -> Option<Value> {
    let Value::Text {
        text: Some(format),
        hidden,
    } = format
    else {
        return None;
    };
    let arguments = match arguments {
        Value::Items(items) => items,
        text @ Value::Text { .. } => vec![text],
        Value::Number(_) => return None,
    };
    let mut arguments = arguments.into_iter();
    let mut pieces = Vec::new();
    let mut literal = String::new();
    let mut chars = format.chars();
    while let Some(c) = chars.next() {
        if c != '%' {
            literal.push(c);
            continue;
        }
        match chars.next()? {
            '%' => literal.push('%'),
            's' => {
                pieces.push(Value::text(std::mem::take(&mut literal), *hidden));
                pieces.push(arguments.next()?);
            }
            _ => return None,
        }
    }
    pieces.push(Value::text(literal, *hidden));
    concatenated(pieces)
}
