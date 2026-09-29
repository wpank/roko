//! Verify-step analysis for the spec-quality rules.
//!
//! Verify steps run as `bash -o pipefail -c <command>` (`graph_task_dispatch.rs`), so a pipeline
//! fails when any stage fails; only a trailing `|| true`, `; exit 0` or a bare `true`/`echo`
//! makes a step unable to fail. [`vacuous_reason`] finds those steps (HF2). [`analyze_step`]
//! classes a step by the strongest thing it proves (SQ04) and records the scope of its test and
//! compile runs (SQ05).
//!
//! This is a port of the shell analysis in `benchmarks/viabilitybench/speclint/speclint.py`
//! (linter `sq-1`). The two must class every step alike, so change them together.

use std::collections::BTreeSet;
use std::sync::LazyLock;

use regex::Regex;
use serde::Serialize;

use super::compile_regex;

/// What a verify step proves, strongest first (SQ04).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum VerifyClass {
    /// Runs a test runner.
    Test,
    /// Runs a program and asserts on what it printed or left behind, or runs a check script the
    /// task does not write.
    Run,
    /// Compiles, type-checks or lints.
    Compile,
    /// Only inspects files, or runs a program and checks nothing but its exit code.
    Structural,
    /// Can never fail (HF2).
    Vacuous,
}

impl VerifyClass {
    /// The SQ04 score of a task whose strongest step has this class.
    pub const fn value(self) -> f64 {
        match self {
            Self::Test => 1.0,
            Self::Run => 0.8,
            Self::Compile => 0.5,
            Self::Structural => 0.25,
            Self::Vacuous => 0.0,
        }
    }

    /// The class name speclint writes: `test`, `run`, `compile`, `structural` or `vacuous`.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Test => "test",
            Self::Run => "run",
            Self::Compile => "compile",
            Self::Structural => "structural",
            Self::Vacuous => "vacuous",
        }
    }
}

/// How much of the workspace a test or compile run covers (SQ05).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Scope {
    /// Scoped to a package, directory, file or test name.
    Scoped,
    /// The whole workspace.
    Workspace,
}

/// The analysis of one verify step.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StepAnalysis {
    /// The strongest thing the step proves.
    pub class: VerifyClass,
    /// Why the step can never fail (HF2), when it cannot.
    pub vacuous: Option<String>,
    /// The scopes of the step's test and compile runs; empty for a vacuous step.
    pub scopes: Vec<Scope>,
    /// Classed compile because of `cargo test --no-run`.
    pub no_run: bool,
    /// Runs a program but checks only its exit code, so it stays structural (the self-exit smell).
    pub self_exit: bool,
}

const OPENERS: &[&str] = &["if", "for", "while", "until", "case", "select", "{"];
const CLOSERS: &[&str] = &["fi", "done", "esac", "}"];
const LEAD_WORDS: &[&str] = &["then", "do", "else", "elif", "!", "time"];

/// Programs, by what running them proves.
const SETUP: &[&str] = &[
    "cd",
    "pushd",
    "popd",
    "cp",
    "mv",
    "rm",
    "rmdir",
    "mkdir",
    "touch",
    "ln",
    "chmod",
    "chown",
    "export",
    "set",
    "unset",
    "source",
    ".",
    "echo",
    "printf",
    "true",
    ":",
    "false",
    "exit",
    "return",
    "sleep",
    "mktemp",
    "tee",
    "trap",
    "local",
    "read",
    "wait",
    "kill",
    "shift",
    "declare",
    "typeset",
    "let",
    "ulimit",
    "umask",
    "eval",
    "(subshell)",
    "install",
    "pwd",
];
const STRUCTURAL: &[&str] = &[
    "grep",
    "egrep",
    "fgrep",
    "rg",
    "ag",
    "test",
    "[",
    "[[",
    "ls",
    "wc",
    "head",
    "tail",
    "cat",
    "find",
    "fd",
    "stat",
    "file",
    "diff",
    "cmp",
    "jq",
    "yq",
    "awk",
    "gawk",
    "sed",
    "sort",
    "uniq",
    "cut",
    "tr",
    "xmllint",
    "git",
    "sha256sum",
    "shasum",
    "md5sum",
    "readlink",
    "realpath",
    "basename",
    "dirname",
    "du",
    "tomlq",
    "taplo",
    "column",
    "comm",
    "od",
    "xxd",
    "strings",
    "tree",
    "nl",
    "paste",
    "join",
    "expr",
    "seq",
    "date",
    "which",
    "type",
    "hash",
    "command",
];
const ASSERTING: &[&str] = &[
    "grep", "egrep", "fgrep", "rg", "ag", "test", "[", "[[", "diff", "cmp", "jq", "yq",
];
const TEST_RUNNERS: &[&str] = &[
    "pytest",
    "py.test",
    "vitest",
    "jest",
    "mocha",
    "ava",
    "tap",
    "playwright",
    "cypress",
    "karma",
    "ctest",
    "phpunit",
    "rspec",
    "nextest",
];
const COMPILERS: &[&str] = &[
    "tsc",
    "vue-tsc",
    "rustc",
    "gcc",
    "g++",
    "cc",
    "clang",
    "clang++",
    "javac",
    "kotlinc",
    "swiftc",
    "esbuild",
    "webpack",
    "rollup",
    "mypy",
    "pyright",
    "eslint",
    "ruff",
    "shellcheck",
];
const TEST_TOKENS: &[&str] = &["test", "tests", "spec", "vitest", "jest", "pytest"];
const BUILD_TOKENS: &[&str] = &[
    "build",
    "typecheck",
    "tsc",
    "compile",
    "check",
    "lint",
    "types",
];
const HARNESS_TOKENS: &[&str] = &[
    "check",
    "verify",
    "validate",
    "accept",
    "acceptance",
    "smoke",
    "e2e",
    "harness",
];
const CARGO_VALUE_FLAGS: &[&str] = &[
    "-p",
    "--package",
    "--manifest-path",
    "--test",
    "--bin",
    "--example",
    "--bench",
    "--features",
    "-F",
    "--target",
    "--target-dir",
    "--profile",
    "-j",
    "--jobs",
    "--color",
    "--message-format",
    "--exclude",
    "-Z",
    "--config",
    "--lockfile-path",
];
const TEST_BINARY_VALUE_FLAGS: &[&str] = &[
    "--test-threads",
    "--skip",
    "--format",
    "--color",
    "--logfile",
    "-Z",
];
const XARGS_VALUE_FLAGS: &[&str] = &["-I", "-n", "-L", "-P", "-d", "-E", "-s"];
const RUNNER_VALUE_FLAGS: &[&str] = &["-p", "--package"];

/// The word that stands for a `$(...)` or backtick substitution.
const SUBSTITUTION: &str = "$(…)";

static ASSIGNMENT: LazyLock<Regex> =
    LazyLock::new(|| compile_regex(r"^[A-Za-z_][A-Za-z0-9_]*(?:\[[^\]]*\])?\+?="));
static PYTHON_VERSION: LazyLock<Regex> = LazyLock::new(|| compile_regex(r"^python3\.\d+$"));
static PYTHON_ASSERTS: LazyLock<Regex> =
    LazyLock::new(|| compile_regex(r"\bassert\b|\braise\b|exit\(|\bSystemExit\b"));
static NODE_ASSERTS: LazyLock<Regex> =
    LazyLock::new(|| compile_regex(r"process\.exit|\bassert\b|\bthrow\b"));

// --------------------------------------------------------------------------------------------
// Splitting a step into simple commands.

/// A simple command of a verify step.
#[derive(Clone, Debug, PartialEq, Eq)]
struct SimpleCommand {
    /// The command's words, unquoted and without redirections; a substitution is `$(…)`.
    words: Vec<String>,
    /// The operator before this command at its level: "", ";", "&&", "||", "|" or "&".
    op: &'static str,
    /// 0 at the step's top level; more inside `$(...)`, backticks, `(...)` or `<(...)`.
    depth: usize,
}

/// Split a bash command into simple commands, in the order they complete.
///
/// Quotes are removed, redirections are dropped, and the commands inside `$(...)`, backticks,
/// subshells and process substitutions come before the command that contains them.
fn parse_shell(src: &str) -> Vec<SimpleCommand> {
    let chars: Vec<char> = src.chars().collect();
    let mut out = Vec::new();
    parse_into(&chars, 0, &mut out);
    out
}

/// The command [`parse_into`] is reading.
struct Pending {
    words: Vec<String>,
    word: String,
    in_word: bool,
    /// The next word is a redirection target. It survives the end of a command, as in speclint.
    skip_next_word: bool,
    op: &'static str,
    depth: usize,
}

impl Pending {
    const fn new(depth: usize) -> Self {
        Self {
            words: Vec::new(),
            word: String::new(),
            in_word: false,
            skip_next_word: false,
            op: "",
            depth,
        }
    }

    fn end_word(&mut self) {
        if self.in_word {
            if self.skip_next_word {
                self.skip_next_word = false;
            } else {
                self.words.push(std::mem::take(&mut self.word));
            }
        }
        self.word.clear();
        self.in_word = false;
    }

    /// End the current command; `next_op` joins it to the next one.
    fn end_command(&mut self, next_op: &'static str, out: &mut Vec<SimpleCommand>) {
        self.end_word();
        if !self.words.is_empty() {
            out.push(SimpleCommand {
                words: std::mem::take(&mut self.words),
                op: self.op,
                depth: self.depth,
            });
            self.op = next_op;
        } else if matches!(next_op, "&&" | "||" | "|") {
            self.op = next_op;
        }
        // A blank line or `;` after `&&`, `||` or `|` keeps that operator: the list continues.
    }
}

fn parse_into(src: &[char], depth: usize, out: &mut Vec<SimpleCommand>) {
    let n = src.len();
    let mut cmd = Pending::new(depth);
    let mut i = 0;
    while i < n {
        let c = src[i];
        let next = src.get(i + 1).copied();
        match c {
            ' ' | '\t' | '\r' => {
                cmd.end_word();
                i += 1;
            }
            '\n' => {
                cmd.end_command(";", out);
                i += 1;
            }
            '#' if !cmd.in_word => i = find_char(src, '\n', i).unwrap_or(n),
            '\\' => {
                if next == Some('\n') {
                    i += 2;
                    continue;
                }
                if let Some(escaped) = next {
                    cmd.word.push(escaped);
                }
                cmd.in_word = true;
                i += 2;
            }
            '\'' => {
                let end = find_char(src, '\'', i + 1).unwrap_or(n);
                cmd.word.extend(&src[i + 1..end]);
                cmd.in_word = true;
                i = end + 1;
            }
            '"' => {
                i = read_double_quoted(src, i + 1, depth, out, &mut cmd.word);
                cmd.in_word = true;
            }
            '`' => {
                let end = backtick_end(src, i + 1);
                parse_into(&src[i + 1..end], depth + 1, out);
                cmd.word.push_str(SUBSTITUTION);
                cmd.in_word = true;
                i = end + 1;
            }
            '$' if next == Some('(') => {
                let end = paren_end(src, i + 2);
                // `$(( arithmetic ))` holds no commands.
                if src.get(i + 2) != Some(&'(') {
                    parse_into(&src[i + 2..end], depth + 1, out);
                }
                cmd.word.push_str(SUBSTITUTION);
                cmd.in_word = true;
                i = end + 1;
            }
            '$' if next == Some('{') => {
                let end = brace_end(src, i + 2);
                cmd.word.extend(&src[i..(end + 1).min(n)]);
                cmd.in_word = true;
                i = end + 1;
            }
            '<' | '>' => i = read_redirection(src, i, depth, out, &mut cmd),
            '|' => {
                if next == Some('|') {
                    cmd.end_command("||", out);
                    i += 2;
                } else {
                    cmd.end_command("|", out);
                    i += if next == Some('&') { 2 } else { 1 };
                }
            }
            '&' => {
                if next == Some('&') {
                    cmd.end_command("&&", out);
                    i += 2;
                } else if next == Some('>') {
                    // `&> file`, `&>> file`
                    cmd.end_word();
                    i += if src.get(i + 2) == Some(&'>') { 3 } else { 2 };
                    cmd.skip_next_word = true;
                } else {
                    cmd.end_command("&", out);
                    i += 1;
                }
            }
            ';' => {
                cmd.end_command(";", out);
                i += if next == Some(';') { 2 } else { 1 };
            }
            '(' if !cmd.in_word && cmd.words.is_empty() => {
                let end = paren_end(src, i + 1);
                parse_into(&src[i + 1..end], depth + 1, out);
                cmd.words.push("(subshell)".to_string());
                i = end + 1;
            }
            _ => {
                cmd.word.push(c);
                cmd.in_word = true;
                i += 1;
            }
        }
    }
    cmd.end_command("", out);
}

/// Read the redirection at `i` (`<`, `>>`, `2>&1`, `<(...)` …) and return the index after its
/// operator. The target that follows is not a word of the command.
fn read_redirection(
    src: &[char],
    i: usize,
    depth: usize,
    out: &mut Vec<SimpleCommand>,
    cmd: &mut Pending,
) -> usize {
    let n = src.len();
    if cmd.in_word && !cmd.word.is_empty() && cmd.word.chars().all(char::is_numeric) {
        // The fd of `2>`.
        cmd.word.clear();
        cmd.in_word = false;
    } else {
        cmd.end_word();
    }
    let mut j = i;
    while j < n && matches!(src[j], '<' | '>') {
        j += 1;
    }
    if j < n && src[j] == '&' {
        // `2>&1`, `>&2`
        j += 1;
        while j < n && (src[j].is_numeric() || src[j] == '-') {
            j += 1;
        }
        j
    } else if j < n && src[j] == '(' && j == i + 1 {
        // `<(...)` process substitution
        let end = paren_end(src, j + 1);
        parse_into(&src[j + 1..end], depth + 1, out);
        end + 1
    } else {
        cmd.skip_next_word = true;
        j
    }
}

fn read_double_quoted(
    src: &[char],
    mut i: usize,
    depth: usize,
    out: &mut Vec<SimpleCommand>,
    word: &mut String,
) -> usize {
    let n = src.len();
    while i < n {
        let c = src[i];
        if c == '"' {
            return i + 1;
        }
        if c == '\\' && i + 1 < n {
            word.push(src[i + 1]);
            i += 2;
        } else if c == '$' && src.get(i + 1) == Some(&'(') {
            let end = paren_end(src, i + 2);
            if src.get(i + 2) != Some(&'(') {
                parse_into(&src[i + 2..end], depth + 1, out);
            }
            word.push_str(SUBSTITUTION);
            i = end + 1;
        } else if c == '`' {
            let end = backtick_end(src, i + 1);
            parse_into(&src[i + 1..end], depth + 1, out);
            word.push_str(SUBSTITUTION);
            i = end + 1;
        } else {
            word.push(c);
            i += 1;
        }
    }
    n
}

fn skip_double_quoted(src: &[char], mut i: usize) -> usize {
    let n = src.len();
    while i < n {
        match src[i] {
            '\\' => i += 2,
            '"' => return i + 1,
            '$' if src.get(i + 1) == Some(&'(') => i = paren_end(src, i + 2) + 1,
            '`' => i = backtick_end(src, i + 1) + 1,
            _ => i += 1,
        }
    }
    n
}

/// The index of the `)` that closes a `(` ending just before `i`; `src.len()` if unbalanced.
fn paren_end(src: &[char], mut i: usize) -> usize {
    let n = src.len();
    let mut level = 1;
    while i < n {
        match src[i] {
            '\\' => {
                i += 2;
                continue;
            }
            '\'' => {
                i = find_char(src, '\'', i + 1).map_or(n, |end| end + 1);
                continue;
            }
            '"' => {
                i = skip_double_quoted(src, i + 1);
                continue;
            }
            '`' => {
                i = backtick_end(src, i + 1) + 1;
                continue;
            }
            '(' => level += 1,
            ')' => {
                level -= 1;
                if level == 0 {
                    return i;
                }
            }
            _ => {}
        }
        i += 1;
    }
    n
}

fn brace_end(src: &[char], mut i: usize) -> usize {
    let n = src.len();
    let mut level = 1;
    while i < n {
        match src[i] {
            '\\' => {
                i += 2;
                continue;
            }
            '{' => level += 1,
            '}' => {
                level -= 1;
                if level == 0 {
                    return i;
                }
            }
            _ => {}
        }
        i += 1;
    }
    n
}

fn backtick_end(src: &[char], mut i: usize) -> usize {
    let n = src.len();
    while i < n {
        match src[i] {
            '\\' => i += 2,
            '`' => return i,
            _ => i += 1,
        }
    }
    n
}

fn find_char(src: &[char], wanted: char, from: usize) -> Option<usize> {
    src.get(from..)?
        .iter()
        .position(|&c| c == wanted)
        .map(|offset| from + offset)
}

// --------------------------------------------------------------------------------------------
// HF2: steps that can never fail.

/// A top-level command as (operator, words); a compound block (`if`, `for`, `{ … }`) has no words.
type Unit<'a> = (&'static str, Option<&'a [String]>);

/// The step's top-level commands, with each compound block as one unit.
fn top_level_units(commands: &[SimpleCommand]) -> Vec<Unit<'_>> {
    let mut units = Vec::new();
    let mut level = 0_usize;
    for command in commands.iter().filter(|command| command.depth == 0) {
        let start_level = level;
        for word in &command.words {
            let word = word.as_str();
            let opener = OPENERS.contains(&word);
            if !opener && !LEAD_WORDS.contains(&word) {
                break;
            }
            if opener {
                level += 1;
            }
            if matches!(word, "for" | "case" | "select") {
                // The rest is a loop header, not a command.
                break;
            }
        }
        if command
            .words
            .first()
            .is_some_and(|word| CLOSERS.contains(&word.as_str()))
        {
            level = level.saturating_sub(1);
            continue;
        }
        if start_level == 0 {
            let words = (level == 0).then_some(command.words.as_slice());
            units.push((command.op, words));
        }
    }
    units
}

fn always_succeeds(words: Option<&[String]>) -> bool {
    let rest: Vec<&str> = words
        .unwrap_or_default()
        .iter()
        .map(String::as_str)
        .filter(|word| !is_assignment(word))
        .collect();
    match rest.as_slice() {
        [] => false,
        ["exit", "0"] => true,
        [head, ..] => matches!(*head, "true" | ":" | "echo" | "printf"),
    }
}

fn is_exit_zero(words: &[String]) -> bool {
    matches!(words, [exit, code] if exit == "exit" && code == "0")
}

/// Why a verify step can never fail (HF2), or `None` when it can.
pub fn vacuous_reason(command: &str) -> Option<String> {
    if command.trim().is_empty() {
        return Some("empty command".to_string());
    }
    let commands = parse_shell(command);
    let units = top_level_units(&commands);
    if units.is_empty() {
        return Some("no command".to_string());
    }
    // `;` and `&` start a new list; the step's status is that of its last list.
    let mut lists: Vec<Vec<Unit<'_>>> = vec![Vec::new()];
    for unit in units {
        if matches!(unit.0, ";" | "&") && lists.last().is_some_and(|list| !list.is_empty()) {
            lists.push(Vec::new());
        }
        if let Some(list) = lists.last_mut() {
            list.push(unit);
        }
    }
    if lists
        .iter()
        .any(|list| matches!(list.as_slice(), [(_, Some(words))] if is_exit_zero(words)))
    {
        return Some("unconditional `exit 0`".to_string());
    }
    // Group the last list into pipelines; with pipefail a pipeline fails if any stage fails.
    let mut pipelines: Vec<(&str, Vec<Option<&[String]>>)> = Vec::new();
    for (op, words) in lists.pop().unwrap_or_default() {
        match pipelines.last_mut() {
            Some((_, stages)) if op == "|" => stages.push(words),
            _ => pipelines.push((op, vec![words])),
        }
    }
    let (final_op, stages) = pipelines.last()?;
    if !stages.iter().all(|words| always_succeeds(*words)) {
        return None;
    }
    let shown = stages
        .first()
        .copied()
        .flatten()
        .map(|words| words.join(" "))
        .unwrap_or_default();
    if pipelines.len() == 1 {
        Some(format!("the step only runs `{shown}`"))
    } else if *final_op == "||" {
        Some(format!("ends in `|| {shown}`"))
    } else {
        None
    }
}

// --------------------------------------------------------------------------------------------
// SQ04 and SQ05: what each program proves.

/// What running one program inside a verify step proves.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum UseKind {
    Test,
    Compile,
    Structural,
    Assert,
    Exec,
    Setup,
}

/// One program invocation inside a verify step.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ProgramUse {
    kind: UseKind,
    /// Test and compile runs only: how much they cover.
    scope: Option<Scope>,
    /// `cargo test --no-run`.
    no_run: bool,
    /// An exec of a check script the task does not write.
    harness: bool,
}

impl ProgramUse {
    const fn of(kind: UseKind) -> Self {
        Self {
            kind,
            scope: None,
            no_run: false,
            harness: false,
        }
    }

    const fn scoped(kind: UseKind, scope: Scope) -> Self {
        Self {
            kind,
            scope: Some(scope),
            no_run: false,
            harness: false,
        }
    }
}

/// Class a verify step by the strongest thing it proves (SQ04), and flag it when it can never
/// fail (HF2).
///
/// `task_files` are the task's own outputs. Running one of them and checking only its exit code
/// is the self-exit smell, which S07 counts as structural.
pub fn analyze_step(command: &str, task_files: &BTreeSet<String>) -> StepAnalysis {
    let mut uses = Vec::new();
    let mut cwd = String::new();
    for simple in parse_shell(command) {
        let argv = strip_wrappers(&simple.words);
        if argv.first().is_some_and(|word| word == "cd") {
            let target = first_positional(&argv[1..]);
            cwd = if target.is_empty() || target.starts_with(['$', '~', '/', '-']) {
                String::new()
            } else {
                resolve(&cwd, target)
            };
            if cwd == "." || cwd.starts_with("..") {
                cwd.clear();
            }
            continue;
        }
        uses.extend(classify(&simple.words, &cwd, task_files));
    }

    // A run counts only when an assertion follows it (a pipe into grep, a test on its output or
    // on the state it left), or when it is a check script the task does not write.
    let mut run = false;
    let mut seen_exec = false;
    for program in &uses {
        match program.kind {
            UseKind::Exec => {
                seen_exec = true;
                run |= program.harness;
            }
            UseKind::Assert if seen_exec => run = true,
            _ => {}
        }
    }
    let has = |kind: UseKind| uses.iter().any(|program| program.kind == kind);
    let class = if has(UseKind::Test) {
        VerifyClass::Test
    } else if run {
        VerifyClass::Run
    } else if has(UseKind::Compile) {
        VerifyClass::Compile
    } else {
        VerifyClass::Structural
    };
    let mut analysis = StepAnalysis {
        class,
        vacuous: None,
        scopes: uses.iter().filter_map(|program| program.scope).collect(),
        no_run: class == VerifyClass::Compile && uses.iter().any(|program| program.no_run),
        self_exit: class == VerifyClass::Structural && seen_exec,
    };
    if let Some(reason) = vacuous_reason(command) {
        analysis.class = VerifyClass::Vacuous;
        analysis.vacuous = Some(reason);
        analysis.scopes.clear();
    }
    analysis
}

/// What the program of one simple command proves; `bash -c`, `npx` and `npm exec` yield the uses
/// of the commands they run.
fn classify(words: &[String], cwd: &str, task_files: &BTreeSet<String>) -> Vec<ProgramUse> {
    let argv = strip_wrappers(words);
    let Some((raw, args)) = argv.split_first() else {
        return Vec::new();
    };
    let name = path_name(raw);
    let prog = name.as_str();
    match prog {
        "sh" | "bash" | "zsh" | "dash" => classify_shell(args, cwd, task_files),
        "cargo" => vec![classify_cargo(args, cwd)],
        "npm" | "pnpm" | "yarn" | "bun" => classify_package_script(args, cwd, task_files),
        "npx" | "pnpx" | "bunx" => classify(skip_flags(args, RUNNER_VALUE_FLAGS), cwd, task_files),
        "next" | "vite" => {
            if matches!(first_positional(args), "build" | "lint") {
                vec![ProgramUse::scoped(UseKind::Compile, scope_for(cwd, false))]
            } else {
                vec![ProgramUse::of(UseKind::Exec)]
            }
        }
        "node" | "deno" => vec![classify_node(prog, args, cwd, task_files)],
        "go" => vec![classify_go(args, cwd)],
        "make" | "just" | "task" => vec![classify_make(args, cwd)],
        _ if TEST_RUNNERS.contains(&prog) => {
            let narrow = has_positional(args, &["run", "watch", "dev", "related", "test"])
                || contains(args, "-k")
                || contains(args, "-t");
            vec![ProgramUse::scoped(UseKind::Test, scope_for(cwd, narrow))]
        }
        _ if COMPILERS.contains(&prog) => vec![classify_compiler(prog, args, cwd)],
        _ if is_python(prog) => vec![classify_python(args, cwd, task_files)],
        _ if ASSERTING.contains(&prog) => vec![ProgramUse::of(UseKind::Assert)],
        _ if STRUCTURAL.contains(&prog) => vec![ProgramUse::of(UseKind::Structural)],
        _ if SETUP.contains(&prog) => vec![ProgramUse::of(UseKind::Setup)],
        _ if raw.contains('/') || raw.starts_with('$') => {
            vec![script_use(raw, args, cwd, task_files)]
        }
        _ => vec![ProgramUse::of(UseKind::Exec)],
    }
}

/// `sh`, `bash`, `zsh`, `dash`: `-c` runs its script's commands, `-n` only parses, and anything
/// else runs a script.
fn classify_shell(args: &[String], cwd: &str, task_files: &BTreeSet<String>) -> Vec<ProgramUse> {
    if let Some(index) = position(args, "-c") {
        let Some(script) = args.get(index + 1) else {
            return Vec::new();
        };
        return parse_shell(script)
            .iter()
            .flat_map(|simple| classify(&simple.words, cwd, task_files))
            .collect();
    }
    if contains(args, "-n") {
        return vec![ProgramUse::scoped(UseKind::Compile, Scope::Scoped)];
    }
    let script = first_positional(args);
    if script.is_empty() {
        return vec![ProgramUse::of(UseKind::Setup)];
    }
    vec![script_use(script, after(args, script), cwd, task_files)]
}

/// `cargo <subcommand>`.
fn classify_cargo(args: &[String], cwd: &str) -> ProgramUse {
    let sub = first_positional(args);
    let run_scope = if cwd.is_empty() {
        cargo_scope(sub, args)
    } else {
        Scope::Scoped
    };
    match sub {
        "test" | "t" if contains(args, "--no-run") => ProgramUse {
            no_run: true,
            ..ProgramUse::scoped(UseKind::Compile, run_scope)
        },
        "test" | "t" | "nextest" => ProgramUse::scoped(UseKind::Test, run_scope),
        "check" | "c" | "build" | "b" | "clippy" | "doc" | "rustc" | "bench" => {
            ProgramUse::scoped(UseKind::Compile, run_scope)
        }
        "run" | "r" => ProgramUse::of(UseKind::Exec),
        "fetch" | "install" | "update" | "generate-lockfile" | "clean" => {
            ProgramUse::of(UseKind::Setup)
        }
        _ => ProgramUse::of(UseKind::Structural),
    }
}

/// Whether a cargo run names a package, manifest, test target or test filter.
fn cargo_scope(sub: &str, args: &[String]) -> Scope {
    let rest = position(args, sub).map_or(args, |index| &args[index + 1..]);
    let mut positional = false;
    let mut after_dashes = false;
    let mut i = 0;
    while i < rest.len() {
        let arg = rest[i].as_str();
        if arg == "--" && !after_dashes {
            after_dashes = true;
        } else if after_dashes && TEST_BINARY_VALUE_FLAGS.contains(&arg) {
            i += 1;
        } else if !after_dashes && names_cargo_target(arg) {
            return Scope::Scoped;
        } else if !after_dashes && CARGO_VALUE_FLAGS.contains(&arg) {
            i += 1;
        } else if !arg.starts_with('-') {
            positional = true;
        }
        i += 1;
    }
    if matches!(sub, "test" | "t" | "nextest") && positional && !contains(args, "--workspace") {
        return Scope::Scoped;
    }
    Scope::Workspace
}

/// A cargo flag that names a package, manifest or target: `-p x`, `-px`, `--test=x` …
fn names_cargo_target(arg: &str) -> bool {
    matches!(
        arg,
        "-p" | "--package" | "--manifest-path" | "--test" | "--bin" | "--example"
    ) || ["--package=", "--manifest-path=", "--test=", "--bin="]
        .iter()
        .any(|prefix| arg.starts_with(prefix))
        || (arg.starts_with("-p") && arg.len() > 2)
}

/// `npm`, `pnpm`, `yarn`, `bun`: setup, `exec` of another program, or a package script classed
/// by its name.
fn classify_package_script(
    args: &[String],
    cwd: &str,
    task_files: &BTreeSet<String>,
) -> Vec<ProgramUse> {
    let sub = first_positional(args);
    let scoped_flag = args.iter().any(|arg| {
        matches!(
            arg.as_str(),
            "--prefix" | "-C" | "--dir" | "--filter" | "-w" | "--workspace"
        )
    });
    let run_scope = scope_for(cwd, scoped_flag || contains(args, "--"));
    if matches!(
        sub,
        "ci" | "install" | "i" | "add" | "remove" | "uninstall" | "link" | "audit" | "outdated"
    ) {
        return vec![ProgramUse::of(UseKind::Setup)];
    }
    if matches!(sub, "exec" | "dlx" | "x") {
        return classify(after(args, sub), cwd, task_files);
    }
    if matches!(sub, "test" | "t" | "tst") {
        return vec![ProgramUse::scoped(UseKind::Test, run_scope)];
    }
    let script = if matches!(sub, "run" | "run-script") {
        first_positional(after(args, sub))
    } else {
        sub
    };
    let tokens = name_tokens(script);
    if has_token(&tokens, TEST_TOKENS) {
        vec![ProgramUse::scoped(UseKind::Test, run_scope)]
    } else if has_token(&tokens, BUILD_TOKENS) {
        vec![ProgramUse::scoped(UseKind::Compile, run_scope)]
    } else {
        vec![ProgramUse::of(UseKind::Exec)]
    }
}

/// `tsc`, `eslint`, `rustc` and the other compilers and linters.
fn classify_compiler(prog: &str, args: &[String], cwd: &str) -> ProgramUse {
    let narrow = match prog {
        "tsc" | "vue-tsc" => contains(args, "-p") || contains(args, "--project"),
        "eslint" | "ruff" | "mypy" | "pyright" | "shellcheck" => args
            .iter()
            .any(|arg| !arg.starts_with('-') && arg != "check" && arg != "."),
        _ => true,
    };
    ProgramUse::scoped(UseKind::Compile, scope_for(cwd, narrow))
}

fn is_python(prog: &str) -> bool {
    matches!(prog, "python" | "python3" | "python2" | "py" | "pypy3")
        || PYTHON_VERSION.is_match(prog)
}

/// `python -m <module>`, `python -c <code>` or `python <script>`.
fn classify_python(args: &[String], cwd: &str, task_files: &BTreeSet<String>) -> ProgramUse {
    if let Some(index) = position(args, "-m") {
        let module = args.get(index + 1).map_or("", String::as_str);
        let rest = args.get(index + 2..).unwrap_or_default();
        return match module {
            "pytest" | "unittest" | "doctest" | "nose2" => ProgramUse::scoped(
                UseKind::Test,
                scope_for(cwd, has_positional(rest, &["discover"])),
            ),
            "py_compile" | "compileall" | "mypy" | "pyright" => {
                ProgramUse::scoped(UseKind::Compile, Scope::Scoped)
            }
            "json.tool" | "tomllib" => ProgramUse::of(UseKind::Assert),
            "pip" | "venv" | "ensurepip" => ProgramUse::of(UseKind::Setup),
            _ => ProgramUse::of(UseKind::Exec),
        };
    }
    if let Some(index) = position(args, "-c") {
        let code = args.get(index + 1).map_or("", String::as_str);
        return if PYTHON_ASSERTS.is_match(code) {
            ProgramUse::of(UseKind::Assert)
        } else {
            ProgramUse::of(UseKind::Structural)
        };
    }
    let script = first_positional(args);
    if script.is_empty() {
        return ProgramUse::of(UseKind::Setup);
    }
    script_use(script, after(args, script), cwd, task_files)
}

/// `node` and `deno`: a test run, a syntax check, inline code or a script.
fn classify_node(
    prog: &str,
    args: &[String],
    cwd: &str,
    task_files: &BTreeSet<String>,
) -> ProgramUse {
    let mut args = args;
    if prog == "deno" {
        let sub = first_positional(args);
        if sub == "test" {
            let narrow = has_positional(args.get(1..).unwrap_or_default(), &[]);
            return ProgramUse::scoped(UseKind::Test, scope_for(cwd, narrow));
        }
        if sub == "check" {
            return ProgramUse::scoped(UseKind::Compile, Scope::Scoped);
        }
        if sub == "run" {
            args = after(args, sub);
        }
    }
    if contains(args, "--test") {
        return ProgramUse::scoped(UseKind::Test, scope_for(cwd, has_positional(args, &[])));
    }
    if contains(args, "--check") || contains(args, "-c") {
        return ProgramUse::scoped(UseKind::Compile, Scope::Scoped);
    }
    for flag in ["-e", "--eval", "-p", "--print"] {
        if let Some(index) = position(args, flag) {
            let code = args.get(index + 1).map_or("", String::as_str);
            return if NODE_ASSERTS.is_match(code) {
                ProgramUse::of(UseKind::Assert)
            } else {
                ProgramUse::of(UseKind::Structural)
            };
        }
    }
    let script = first_positional(args);
    if script.is_empty() {
        return ProgramUse::of(UseKind::Setup);
    }
    script_use(script, after(args, script), cwd, task_files)
}

/// `go test`, `go build`, `go vet` or another `go` command.
fn classify_go(args: &[String], cwd: &str) -> ProgramUse {
    let sub = first_positional(args);
    let rest: &[String] = if sub.is_empty() {
        &[]
    } else {
        after(args, sub)
    };
    let packages: Vec<&str> = rest
        .iter()
        .map(String::as_str)
        .filter(|arg| !arg.starts_with('-'))
        .collect();
    let whole = packages.is_empty() || packages.contains(&"./...") || packages.contains(&"...");
    let run_scope = scope_for(cwd, contains(rest, "-run") || !whole);
    match sub {
        "test" => ProgramUse::scoped(UseKind::Test, run_scope),
        "build" | "vet" => ProgramUse::scoped(UseKind::Compile, run_scope),
        _ => ProgramUse::of(UseKind::Exec),
    }
}

/// `make`, `just`, `task`: classed by the target's name; no target is a build.
fn classify_make(args: &[String], cwd: &str) -> ProgramUse {
    let target = first_positional(args);
    let tokens = name_tokens(target);
    let run_scope = scope_for(cwd, false);
    if has_token(&tokens, TEST_TOKENS) {
        ProgramUse::scoped(UseKind::Test, run_scope)
    } else if has_token(&tokens, BUILD_TOKENS) || target.is_empty() {
        ProgramUse::scoped(UseKind::Compile, run_scope)
    } else {
        ProgramUse::of(UseKind::Exec)
    }
}

/// Running a script: a test when its name says so; otherwise an exec, which is a harness when
/// its name says it checks something and the task does not write it.
fn script_use(
    target: &str,
    script_args: &[String],
    cwd: &str,
    task_files: &BTreeSet<String>,
) -> ProgramUse {
    let path = resolve(cwd, target);
    let tokens = name_tokens(&path_name(target));
    if has_token(&tokens, TEST_TOKENS) {
        return ProgramUse::scoped(UseKind::Test, scope_for(cwd, !script_args.is_empty()));
    }
    ProgramUse {
        harness: has_token(&tokens, HARNESS_TOKENS) && !task_files.contains(&path),
        ..ProgramUse::of(UseKind::Exec)
    }
}

/// `Scoped` inside a `cd` target or when the run narrows itself; `Workspace` otherwise.
const fn scope_for(cwd: &str, narrow: bool) -> Scope {
    if narrow || !cwd.is_empty() {
        Scope::Scoped
    } else {
        Scope::Workspace
    }
}

/// The words of a simple command without leading keywords, variable assignments and wrappers
/// such as `env`, `timeout 60` or `xargs -I{}`. Empty for a keyword that starts no command
/// (`fi`, `for` …).
fn strip_wrappers(words: &[String]) -> &[String] {
    let rest = skip_leading(words, |word| {
        LEAD_WORDS.contains(&word) || matches!(word, "if" | "while" | "until" | "{")
    });
    match rest.first().map(String::as_str) {
        None => return &[],
        Some(word)
            if CLOSERS.contains(&word)
                || matches!(word, "for" | "case" | "select" | "function" | "in") =>
        {
            return &[];
        }
        Some(_) => {}
    }
    let mut rest = skip_leading(rest, is_assignment);
    while let Some(first) = rest.first() {
        match path_name(first).as_str() {
            "env" => {
                rest = skip_leading(&rest[1..], |arg| arg.starts_with('-') || is_assignment(arg));
            }
            "timeout" | "gtimeout" => {
                // `timeout [flags] DURATION command…`
                rest = skip_leading(&rest[1..], |arg| arg.starts_with('-'));
                rest = rest.get(1..).unwrap_or_default();
            }
            "nice" | "nohup" | "exec" | "builtin" | "stdbuf" | "time" => {
                rest = skip_leading(&rest[1..], |arg| arg.starts_with('-'));
            }
            "xargs" => rest = skip_flags(&rest[1..], XARGS_VALUE_FLAGS),
            _ => break,
        }
    }
    rest
}

fn skip_leading(words: &[String], skip: impl Fn(&str) -> bool) -> &[String] {
    let start = words
        .iter()
        .position(|word| !skip(word.as_str()))
        .unwrap_or(words.len());
    &words[start..]
}

/// The arguments after the leading flags, skipping the value of each flag in `value_flags`.
fn skip_flags<'a>(args: &'a [String], value_flags: &[&str]) -> &'a [String] {
    let mut rest = args;
    while let Some(flag) = rest.first().filter(|arg| arg.starts_with('-')) {
        let takes_value = value_flags.contains(&flag.as_str());
        rest = &rest[1..];
        if takes_value && !rest.is_empty() {
            rest = &rest[1..];
        }
    }
    rest
}

fn is_assignment(word: &str) -> bool {
    ASSIGNMENT.is_match(word)
}

/// The first argument that is not a flag (`-x`, `+nightly`), or `""`.
fn first_positional(args: &[String]) -> &str {
    args.iter()
        .map(String::as_str)
        .find(|arg| !arg.starts_with(['-', '+']))
        .unwrap_or_default()
}

/// Whether some argument is neither a flag nor in `skip`.
fn has_positional(args: &[String], skip: &[&str]) -> bool {
    args.iter()
        .any(|arg| !arg.starts_with('-') && !skip.contains(&arg.as_str()))
}

fn position(args: &[String], word: &str) -> Option<usize> {
    args.iter().position(|arg| arg == word)
}

fn contains(args: &[String], word: &str) -> bool {
    args.iter().any(|arg| arg == word)
}

/// The arguments after the first `word`.
fn after<'a>(args: &'a [String], word: &str) -> &'a [String] {
    position(args, word)
        .and_then(|index| args.get(index + 1..))
        .unwrap_or_default()
}

/// The lowercase parts of a program, script or target name, split at `-`, `_`, `.`, `:` and `/`.
fn name_tokens(name: &str) -> Vec<String> {
    name.to_lowercase()
        .split(['-', '_', '.', ':', '/'])
        .filter(|token| !token.is_empty())
        .map(str::to_string)
        .collect()
}

fn has_token(tokens: &[String], set: &[&str]) -> bool {
    tokens.iter().any(|token| set.contains(&token.as_str()))
}

/// The last component of a `/`-separated path, like Python's `PurePosixPath(path).name`: `""`
/// for `/` or `.`.
fn path_name(path: &str) -> String {
    path.split('/')
        .rfind(|part| !part.is_empty() && *part != ".")
        .unwrap_or_default()
        .to_string()
}

/// `path` relative to the step's `cd` target, normalized; `$…`, `~…` and absolute paths are kept.
fn resolve(cwd: &str, path: &str) -> String {
    if path.starts_with(['$', '~', '/']) {
        path.to_string()
    } else if cwd.is_empty() {
        normpath(path)
    } else {
        normpath(&format!("{cwd}/{path}"))
    }
}

/// Python's `posixpath.normpath`: drop empty and `.` parts and fold `..` where it can.
fn normpath(path: &str) -> String {
    if path.is_empty() {
        return ".".to_string();
    }
    let initial_slashes = if path.starts_with("//") && !path.starts_with("///") {
        2
    } else {
        usize::from(path.starts_with('/'))
    };
    let mut parts: Vec<&str> = Vec::new();
    for part in path.split('/') {
        if part.is_empty() || part == "." {
            continue;
        }
        if part != ".." || (initial_slashes == 0 && parts.is_empty()) || parts.last() == Some(&"..")
        {
            parts.push(part);
        } else {
            parts.pop();
        }
    }
    let normalized = format!("{}{}", "/".repeat(initial_slashes), parts.join("/"));
    if normalized.is_empty() {
        ".".to_string()
    } else {
        normalized
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn class_of(command: &str) -> VerifyClass {
        analyze_step(command, &BTreeSet::new()).class
    }

    /// speclint's `test_step_classes`.
    #[test]
    fn step_classes_match_speclint() {
        let cases = [
            (
                "cargo test -p roko-cli --lib plan_validate",
                VerifyClass::Test,
            ),
            (
                "cargo test -p roko-serve --lib --no-run 2>&1 | tail -10",
                VerifyClass::Compile,
            ),
            (
                "cargo check -p roko-cli 2>&1 | tail -5",
                VerifyClass::Compile,
            ),
            (
                "grep -q 'cargo test --workspace' justfile && grep -q 'cargo clippy' justfile",
                VerifyClass::Structural,
            ),
            (
                "cd apps/portal && { [ -d node_modules ] || npm ci --silent; } && npm test",
                VerifyClass::Test,
            ),
            (
                "cd apps/portal && node scripts/vitest-min.mjs src/lib/runState.test.ts 56",
                VerifyClass::Test,
            ),
            (
                "cd apps/portal && ./node_modules/.bin/tsc --noEmit",
                VerifyClass::Compile,
            ),
            (
                "cargo run -p roko-cli --quiet -- plan queue show --help 2>&1 | grep -q 'show'",
                VerifyClass::Run,
            ),
            (
                "test \"$(sh demo/x/validate.sh)\" = 'demo: PASS'",
                VerifyClass::Run,
            ),
            (
                "cargo build -p roko-cli && bash plans/p/_harness/live-events-check.sh",
                VerifyClass::Run,
            ),
            (
                "python3 -m unittest tests.visible.test_migrate",
                VerifyClass::Test,
            ),
            (
                "python3 -c 'import json; json.load(open(\"a.json\"))'",
                VerifyClass::Structural,
            ),
            ("bash -n scripts/deploy.sh", VerifyClass::Compile),
            (
                "grep -c 'cmd_do' src/develop.rs | xargs -I{} test {} -le 4",
                VerifyClass::Structural,
            ),
            ("cargo test -p x || true", VerifyClass::Vacuous),
        ];
        for (command, class) in cases {
            assert_eq!(class_of(command), class, "{command}");
        }
    }

    /// speclint's `test_running_the_tasks_own_script_is_self_exit`.
    #[test]
    fn running_the_tasks_own_script_is_self_exit() {
        let own = BTreeSet::from(["scripts/migrate.sh".to_string()]);
        let step = analyze_step("bash scripts/migrate.sh", &own);
        assert_eq!(step.class, VerifyClass::Structural);
        assert!(step.self_exit);
    }

    /// speclint's `test_vacuous_steps`.
    #[test]
    fn vacuous_steps_match_speclint() {
        let cases = [
            ("true", true),
            (":", true),
            ("echo ok", true),
            ("  ", true),
            ("cargo test -p x || true", true),
            ("cargo test -p x || :", true),
            ("grep -q x f; exit 0", true),
            ("grep -q x f; echo done", true),
            ("grep -q x f && echo FAIL || echo PASS", true),
            ("test $(grep -c x f || true) -eq 3", false),
            ("cargo check -p x 2>&1 | tail -5", false),
            ("grep -q x f && echo ok", false),
            ("for h in a b; do grep -q \"$h\" f || exit 1; done", false),
            ("if [ -f x ]; then exit 1; else exit 0; fi", false),
            (
                "cd app && { [ -d node_modules ] || npm ci; } && npm test",
                false,
            ),
        ];
        for (command, vacuous) in cases {
            assert_eq!(vacuous_reason(command).is_some(), vacuous, "{command}");
        }
    }

    #[test]
    fn vacuous_reasons_name_the_step_that_cannot_fail() {
        assert_eq!(
            vacuous_reason("echo ok").as_deref(),
            Some("the step only runs `echo ok`")
        );
        assert_eq!(
            vacuous_reason("cargo test -p x || true").as_deref(),
            Some("ends in `|| true`")
        );
        assert_eq!(
            vacuous_reason("grep -q x f; exit 0").as_deref(),
            Some("unconditional `exit 0`")
        );
    }

    #[test]
    fn scopes_follow_packages_and_cd_targets() {
        let scopes = |command: &str| analyze_step(command, &BTreeSet::new()).scopes;
        assert_eq!(
            scopes("cargo test -p fixture --lib config"),
            [Scope::Scoped]
        );
        assert_eq!(scopes("cargo test --workspace"), [Scope::Workspace]);
        assert_eq!(scopes("cargo test config"), [Scope::Scoped]);
        assert_eq!(scopes("cd app && npm test"), [Scope::Scoped]);
        assert_eq!(scopes("npm test"), [Scope::Workspace]);
        assert_eq!(scopes("grep -q x f"), Vec::<Scope>::new());
    }

    #[test]
    fn cargo_test_no_run_is_compile() {
        let step = analyze_step(
            "cargo test -p roko-cli --no-run 2>&1 | tail -3",
            &BTreeSet::new(),
        );
        assert_eq!(step.class, VerifyClass::Compile);
        assert!(step.no_run);
    }

    #[test]
    fn paths_normalize_like_python() {
        assert_eq!(normpath("a/./b/../c"), "a/c");
        assert_eq!(normpath("../a"), "../a");
        assert_eq!(normpath("a/.."), ".");
        assert_eq!(normpath(""), ".");
        assert_eq!(path_name("./node_modules/.bin/tsc"), "tsc");
        assert_eq!(path_name("a/."), "a");
        assert_eq!(path_name("."), "");
        assert_eq!(
            resolve("apps/portal", "scripts/x.sh"),
            "apps/portal/scripts/x.sh"
        );
    }
}
