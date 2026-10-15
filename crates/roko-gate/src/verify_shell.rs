//! How a verify step's command runs: `bash -o pipefail -c <command>`, with its early readers
//! drained.
//!
//! Under `pipefail` a pipeline fails when any of its stages fails. A stage that stops reading
//! before its input ends, such as `grep -q` at its first match or `head` after its lines, closes
//! the pipe while the stage before it may still be writing. That writer then dies of SIGPIPE
//! (status 141) or fails its write (Python exits 120 or 1, cargo 101), and the step fails although
//! what it checked held. Whether it does depends on timing (bug-11901c): the planner's
//! `cargo test -p x new_test 2>&1 | grep -q 'ok. 1 passed'` failed 19 runs in 20, and
//! `python3 -m unittest … -v 2>&1 | grep -q 'OK'` 29 in 40.
//!
//! [`drain_early_readers`] runs each such reader that a pipe feeds through a shell function. Once
//! the reader exits, the function leaves a background `head -c` reading what the writer still
//! sends, up to 64 MiB, and discarding it. The writer runs to its end, so the step fails only when
//! the writer itself fails or the reader's check does. Draining in the background means a process
//! the writer leaves running with the pipe open does not hold up the step, and the limit means a
//! writer that never ends (`yes | head -1`) still fails at once, as it did before.
//!
//! The scan follows the shell's quoting, expansions, comments, here-documents, subshells and
//! `[[ ]]` tests. A command that uses syntax it does not follow (`case`, backquotes, extended
//! globs, arrays, function definitions) runs unchanged.

use std::borrow::Cow;

/// The shell function that runs an early reader and then drains its input.
const DRAIN_FUNCTION: &str = "__roko_drain";

/// The definition of [`DRAIN_FUNCTION`], put in front of a command that calls it. It returns the
/// reader's own status and drains up to 64 MiB in the background; `command head` skips any
/// function the step defines named `head`.
const DRAIN_DEFINITION: &str = "__roko_drain() { local __roko_status=0; \"$@\" || __roko_status=$?; \
     command head -c 67108864 <&0 >/dev/null 2>&1 & return \"$__roko_status\"; }; ";

/// Programs that may stop reading before their input ends.
const EARLY_READERS: &[&str] = &[
    "grep", "egrep", "fgrep", "rg", "ag", "head", "sed", "awk", "gawk", "mawk", "nawk",
];

/// Words after which the next word still starts a command.
const KEEP_COMMAND_START: &[&[u8]] = &[
    b"!", b"{", b"if", b"then", b"elif", b"else", b"while", b"until", b"do", b"time",
];

/// The deepest nesting of subshells and substitutions a scan follows.
const MAX_DEPTH: usize = 32;

/// The arguments that run `command` as a verify step: `-o pipefail -c` and the command with its
/// early readers drained ([`drain_early_readers`]).
#[must_use]
pub fn bash_args(command: &str) -> Vec<String> {
    vec![
        "-o".to_string(),
        "pipefail".to_string(),
        "-c".to_string(),
        drain_early_readers(command).into_owned(),
    ]
}

/// `command` with its early readers drained.
///
/// Each early reader that a pipe feeds (`… | grep -q x`, `… | head -5`) runs through the drain
/// function, whose definition goes in front. A command with no such reader, or with syntax the
/// scan does not follow, comes back unchanged.
#[must_use]
pub fn drain_early_readers(command: &str) -> Cow<'_, str> {
    let readers = match Scan::new(command).run() {
        Ok(readers) if !readers.is_empty() => readers,
        _ => return Cow::Borrowed(command),
    };
    let mut drained = String::with_capacity(
        DRAIN_DEFINITION.len() + command.len() + readers.len() * (DRAIN_FUNCTION.len() + 1),
    );
    drained.push_str(DRAIN_DEFINITION);
    let mut copied = 0;
    for reader in readers {
        drained.push_str(&command[copied..reader]);
        drained.push_str(DRAIN_FUNCTION);
        drained.push(' ');
        copied = reader;
    }
    drained.push_str(&command[copied..]);
    Cow::Owned(drained)
}

/// Why a scan gave up: the command uses syntax it does not follow.
#[derive(Debug)]
struct Unsupported;

type Scanned<T = ()> = Result<T, Unsupported>;

/// A left-to-right scan of a command for the early readers that pipes feed.
struct Scan<'a> {
    src: &'a [u8],
    at: usize,
    depth: usize,
    /// The byte offsets of the early readers found, in order.
    readers: Vec<usize>,
    /// Here-documents whose bodies start after the next newline: each delimiter, and whether its
    /// lines may start with tabs (`<<-`).
    heredocs: Vec<(Vec<u8>, bool)>,
}

impl<'a> Scan<'a> {
    const fn new(command: &'a str) -> Self {
        Self {
            src: command.as_bytes(),
            at: 0,
            depth: 0,
            readers: Vec::new(),
            heredocs: Vec::new(),
        }
    }

    fn run(mut self) -> Scanned<Vec<usize>> {
        self.list(false)?;
        Ok(self.readers)
    }

    fn peek(&self, ahead: usize) -> Option<u8> {
        self.src.get(self.at + ahead).copied()
    }

    fn skip(&mut self, count: usize) {
        self.at = (self.at + count).min(self.src.len());
    }

    fn skip_blanks(&mut self) {
        while matches!(self.peek(0), Some(b' ' | b'\t')) {
            self.at += 1;
        }
    }

    /// Scan a list of commands to the end of the command or, when `nested`, through the `)` that
    /// closes it.
    fn list(&mut self, nested: bool) -> Scanned {
        self.depth += 1;
        if self.depth > MAX_DEPTH {
            return Err(Unsupported);
        }
        // The next word starts a command.
        let mut command_start = true;
        // A pipe feeds the command that starts next.
        let mut piped = false;
        while let Some(byte) = self.peek(0) {
            match byte {
                b' ' | b'\t' => self.at += 1,
                b'\n' => {
                    self.at += 1;
                    self.heredoc_bodies()?;
                    command_start = true;
                }
                b'\\' if self.peek(1) == Some(b'\n') => self.at += 2,
                b'#' => self.skip_comment(),
                // `;;` and `;&` end `case` arms.
                b';' if matches!(self.peek(1), Some(b';' | b'&')) => return Err(Unsupported),
                b';' => {
                    self.at += 1;
                    (command_start, piped) = (true, false);
                }
                b'&' if self.peek(1) == Some(b'>') => {
                    self.skip(2);
                    if self.peek(0) == Some(b'>') {
                        self.at += 1;
                    }
                    self.redirect_target()?;
                    piped = false;
                }
                b'&' => {
                    self.skip(if self.peek(1) == Some(b'&') { 2 } else { 1 });
                    (command_start, piped) = (true, false);
                }
                b'|' => {
                    piped = self.peek(1) != Some(b'|');
                    self.skip(if matches!(self.peek(1), Some(b'|' | b'&')) {
                        2
                    } else {
                        1
                    });
                    command_start = true;
                }
                b'<' | b'>' => {
                    self.redirection()?;
                    piped = false;
                }
                b'(' if !command_start => return Err(Unsupported),
                b'(' => {
                    if self.peek(1) == Some(b'(') {
                        self.at += 2;
                        self.arithmetic()?;
                    } else {
                        self.at += 1;
                        self.list(true)?;
                    }
                    (command_start, piped) = (false, false);
                }
                b')' if nested => {
                    self.at += 1;
                    self.depth -= 1;
                    return Ok(());
                }
                b')' => return Err(Unsupported),
                _ => {
                    let start = self.at;
                    let plain = self.word()?;
                    let word = &self.src[start..self.at];
                    let assignment = is_assignment(word);
                    if piped && !assignment {
                        if plain && is_early_reader(word) {
                            self.readers.push(start);
                        }
                        piped = false;
                    }
                    if command_start {
                        if plain && word == b"case" {
                            return Err(Unsupported);
                        }
                        if plain && word == b"[[" {
                            self.conditional()?;
                            command_start = false;
                            continue;
                        }
                        command_start = assignment || (plain && KEEP_COMMAND_START.contains(&word));
                    }
                }
            }
        }
        // An unclosed subshell or substitution, or a here-document with no body.
        if nested || !self.heredocs.is_empty() {
            return Err(Unsupported);
        }
        self.depth -= 1;
        Ok(())
    }

    /// Scan one word, up to the first unquoted metacharacter, and return whether it is plain: no
    /// quotes, escapes or expansions.
    fn word(&mut self) -> Scanned<bool> {
        let mut plain = true;
        while let Some(byte) = self.peek(0) {
            match byte {
                b' ' | b'\t' | b'\n' | b'|' | b'&' | b';' | b'<' | b'>' | b')' => break,
                // An extended glob, an array or a function definition.
                b'(' | b'`' => return Err(Unsupported),
                b'\\' => {
                    self.skip(2);
                    plain = false;
                }
                b'\'' => {
                    self.single_quoted()?;
                    plain = false;
                }
                b'"' => {
                    self.at += 1;
                    self.double_quoted()?;
                    plain = false;
                }
                b'$' => {
                    self.dollar()?;
                    plain = false;
                }
                _ => self.at += 1,
            }
        }
        Ok(plain)
    }

    /// Scan the expansion or quoting that starts at a `$`.
    fn dollar(&mut self) -> Scanned {
        match (self.peek(1), self.peek(2)) {
            (Some(b'\''), _) => {
                self.at += 2;
                self.ansi_c_quoted()
            }
            (Some(b'"'), _) => {
                self.at += 2;
                self.double_quoted()
            }
            (Some(b'('), Some(b'(')) => {
                self.at += 3;
                self.arithmetic()
            }
            (Some(b'('), _) => {
                self.at += 2;
                self.list(true)
            }
            (Some(b'{'), _) => {
                self.at += 2;
                self.parameter()
            }
            // `$[ ]`, the old arithmetic.
            (Some(b'['), _) => Err(Unsupported),
            _ => {
                self.at += 1;
                Ok(())
            }
        }
    }

    /// Skip a single-quoted string at its opening `'`.
    fn single_quoted(&mut self) -> Scanned {
        let close = self.src[self.at + 1..]
            .iter()
            .position(|&byte| byte == b'\'')
            .ok_or(Unsupported)?;
        self.at += close + 2;
        Ok(())
    }

    /// Skip a `$'…'` string after its opening quote.
    fn ansi_c_quoted(&mut self) -> Scanned {
        while let Some(byte) = self.peek(0) {
            match byte {
                b'\\' => self.skip(2),
                b'\'' => {
                    self.at += 1;
                    return Ok(());
                }
                _ => self.at += 1,
            }
        }
        Err(Unsupported)
    }

    /// Scan a double-quoted string after its opening quote, through its closing one.
    fn double_quoted(&mut self) -> Scanned {
        while let Some(byte) = self.peek(0) {
            match byte {
                b'"' => {
                    self.at += 1;
                    return Ok(());
                }
                b'\\' => self.skip(2),
                b'`' => return Err(Unsupported),
                b'$' if matches!(self.peek(1), Some(b'(' | b'{' | b'[')) => self.dollar()?,
                _ => self.at += 1,
            }
        }
        Err(Unsupported)
    }

    /// Skip a parameter expansion after its `${`, through its `}`.
    fn parameter(&mut self) -> Scanned {
        while let Some(byte) = self.peek(0) {
            match byte {
                b'}' => {
                    self.at += 1;
                    return Ok(());
                }
                b'\\' => self.skip(2),
                b'\'' => self.single_quoted()?,
                b'"' => {
                    self.at += 1;
                    self.double_quoted()?;
                }
                b'`' => return Err(Unsupported),
                b'$' => self.dollar()?,
                _ => self.at += 1,
            }
        }
        Err(Unsupported)
    }

    /// Skip an arithmetic expression after its `((` or `$((`, through the `))` that closes it.
    fn arithmetic(&mut self) -> Scanned {
        let mut open = 2_usize;
        while let Some(byte) = self.peek(0) {
            match byte {
                b'(' => open += 1,
                b')' => {
                    open -= 1;
                    if open == 0 {
                        self.at += 1;
                        return Ok(());
                    }
                }
                b'\'' => {
                    self.single_quoted()?;
                    continue;
                }
                b'"' => {
                    self.at += 1;
                    self.double_quoted()?;
                    continue;
                }
                b'`' => return Err(Unsupported),
                b'\\' => self.at += 1,
                _ => {}
            }
            self.skip(1);
        }
        Err(Unsupported)
    }

    /// Skip a `[[ ]]` test after its `[[`, through its `]]`. Inside it `|`, `&`, `<`, `>` and
    /// parentheses are operators of the test or of a `=~` pattern, not of the shell.
    fn conditional(&mut self) -> Scanned {
        let mut word_start = true;
        while let Some(byte) = self.peek(0) {
            if word_start
                && byte == b']'
                && self.peek(1) == Some(b']')
                && matches!(
                    self.peek(2),
                    None | Some(b' ' | b'\t' | b'\n' | b';' | b'&' | b'|' | b')')
                )
            {
                self.at += 2;
                return Ok(());
            }
            word_start = matches!(byte, b' ' | b'\t' | b'\n');
            match byte {
                b'\\' => self.skip(2),
                b'\'' => self.single_quoted()?,
                b'"' => {
                    self.at += 1;
                    self.double_quoted()?;
                }
                b'`' => return Err(Unsupported),
                b'$' => self.dollar()?,
                _ => self.at += 1,
            }
        }
        Err(Unsupported)
    }

    /// Scan a redirection at its `<` or `>`: its operator and its target. A process substitution
    /// is scanned as a nested list.
    fn redirection(&mut self) -> Scanned {
        match (self.peek(0), self.peek(1), self.peek(2)) {
            (Some(b'<'), Some(b'<'), Some(b'<')) => {
                self.at += 3;
                self.redirect_target()
            }
            (Some(b'<'), Some(b'<'), Some(b'-')) => {
                self.at += 3;
                self.heredoc(true)
            }
            (Some(b'<'), Some(b'<'), _) => {
                self.at += 2;
                self.heredoc(false)
            }
            (_, Some(b'('), _) => {
                self.at += 2;
                self.list(true)
            }
            // `>>`, `>|`, `>&`, `<&` and `<>`.
            (Some(b'>'), Some(b'>' | b'|' | b'&'), _) | (Some(b'<'), Some(b'&' | b'>'), _) => {
                self.at += 2;
                self.redirect_target()
            }
            _ => {
                self.at += 1;
                self.redirect_target()
            }
        }
    }

    /// Scan the word a redirection operator reads or writes.
    fn redirect_target(&mut self) -> Scanned {
        self.skip_blanks();
        if matches!(self.peek(0), Some(b'<' | b'>')) && self.peek(1) == Some(b'(') {
            self.at += 2;
            return self.list(true);
        }
        let start = self.at;
        self.word()?;
        if self.at == start {
            return Err(Unsupported);
        }
        Ok(())
    }

    /// Scan a here-document's delimiter, after its `<<` or `<<-`; its body follows the next
    /// newline.
    fn heredoc(&mut self, strip_tabs: bool) -> Scanned {
        self.skip_blanks();
        let start = self.at;
        self.word()?;
        if self.at == start {
            return Err(Unsupported);
        }
        let delimiter = unquote(&self.src[start..self.at]);
        self.heredocs.push((delimiter, strip_tabs));
        Ok(())
    }

    /// Skip the bodies of the here-documents waiting for this newline.
    fn heredoc_bodies(&mut self) -> Scanned {
        for (delimiter, strip_tabs) in std::mem::take(&mut self.heredocs) {
            loop {
                if self.at >= self.src.len() {
                    return Err(Unsupported);
                }
                let end = self.src[self.at..]
                    .iter()
                    .position(|&byte| byte == b'\n')
                    .map_or(self.src.len(), |offset| self.at + offset);
                let mut line = &self.src[self.at..end];
                if strip_tabs {
                    while let [b'\t', rest @ ..] = line {
                        line = rest;
                    }
                }
                self.at = (end + 1).min(self.src.len());
                if line == delimiter.as_slice() {
                    break;
                }
            }
        }
        Ok(())
    }

    /// Skip a comment up to, not through, the newline that ends it.
    fn skip_comment(&mut self) {
        self.at = self.src[self.at..]
            .iter()
            .position(|&byte| byte == b'\n')
            .map_or(self.src.len(), |offset| self.at + offset);
    }
}

/// Whether `word` assigns a variable (`NAME=…`, `NAME+=…`, `NAME[i]=…`), which may come before a
/// command's name.
fn is_assignment(word: &[u8]) -> bool {
    let name = word
        .iter()
        .position(|&byte| !(byte.is_ascii_alphanumeric() || byte == b'_'))
        .unwrap_or(word.len());
    if name == 0 || word[0].is_ascii_digit() {
        return false;
    }
    let mut rest = &word[name..];
    if let [b'[', subscript @ ..] = rest {
        let Some(close) = subscript.iter().position(|&byte| byte == b']') else {
            return false;
        };
        rest = &subscript[close + 1..];
    }
    if let [b'+', after @ ..] = rest {
        rest = after;
    }
    rest.first() == Some(&b'=')
}

/// Whether the plain word `word` names an early reader, bare or by path.
fn is_early_reader(word: &[u8]) -> bool {
    let name = word.rsplit(|&byte| byte == b'/').next().unwrap_or(word);
    EARLY_READERS.iter().any(|reader| reader.as_bytes() == name)
}

/// A here-document delimiter as written, without its quotes and backslashes.
fn unquote(word: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(word.len());
    let mut bytes = word.iter();
    while let Some(&byte) = bytes.next() {
        match byte {
            b'\'' | b'"' => {}
            b'\\' => out.extend(bytes.next()),
            _ => out.push(byte),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use std::process::{Command, Output, Stdio};
    use std::time::{Duration, Instant};

    use super::*;

    /// A writer that prints the line a check looks for, then more than any pipe holds.
    const LONG_WRITER: &str = "{ echo OK; seq 1 200000; }";

    /// What `command` becomes after the drain function's definition.
    fn drained(command: &str) -> Option<String> {
        match drain_early_readers(command) {
            Cow::Borrowed(_) => None,
            Cow::Owned(drained) => Some(
                drained
                    .strip_prefix(DRAIN_DEFINITION)
                    .expect("a drained command starts with the definition")
                    .to_string(),
            ),
        }
    }

    /// Run `args` under `bash` as a verify step would, with no input.
    fn bash(args: &[String]) -> Output {
        Command::new("bash")
            .args(args)
            .stdin(Stdio::null())
            .output()
            .expect("run bash")
    }

    /// The plain `bash -o pipefail -c` verify steps ran with before the drain.
    fn undrained_args(command: &str) -> Vec<String> {
        ["-o", "pipefail", "-c", command]
            .map(str::to_string)
            .to_vec()
    }

    #[test]
    fn readers_fed_by_a_pipe_are_drained() {
        let cases = [
            (
                "cargo test -p x new_test 2>&1 | grep -q 'ok. 1 passed'",
                "cargo test -p x new_test 2>&1 | __roko_drain grep -q 'ok. 1 passed'",
            ),
            (
                "python3 -m unittest tests.test_budgets -v 2>&1 | grep -q 'OK'",
                "python3 -m unittest tests.test_budgets -v 2>&1 | __roko_drain grep -q 'OK'",
            ),
            (
                "cd demo/hello-server && cargo build --quiet 2>&1 | head -20",
                "cd demo/hello-server && cargo build --quiet 2>&1 | __roko_drain head -20",
            ),
            (
                "head -1 out.md | grep -qF '# Roko plan runner works'",
                "head -1 out.md | __roko_drain grep -qF '# Roko plan runner works'",
            ),
            ("a | b | head -1", "a | b | __roko_drain head -1"),
            ("a |& rg -q x", "a |& __roko_drain rg -q x"),
            (
                "a | grep -m1 x | wc -l",
                "a | __roko_drain grep -m1 x | wc -l",
            ),
            ("a|grep -q x", "a|__roko_drain grep -q x"),
            ("a |\n  grep -q x", "a |\n  __roko_drain grep -q x"),
            ("a | \\\n  grep -q x", "a | \\\n  __roko_drain grep -q x"),
            (
                "a | # note\n grep -q x",
                "a | # note\n __roko_drain grep -q x",
            ),
            (
                "a | LC_ALL=C grep -q x",
                "a | LC_ALL=C __roko_drain grep -q x",
            ),
            (
                "a | /usr/bin/grep -q x",
                "a | __roko_drain /usr/bin/grep -q x",
            ),
            (
                "out=$(a | head -1) && test -n \"$out\"",
                "out=$(a | __roko_drain head -1) && test -n \"$out\"",
            ),
            (
                "test \"$(a | grep -c x)\" = 1",
                "test \"$(a | __roko_drain grep -c x)\" = 1",
            ),
            (
                "(cd d && a | grep -q x)",
                "(cd d && a | __roko_drain grep -q x)",
            ),
            (
                "{ a; b; } | grep -q x",
                "{ a; b; } | __roko_drain grep -q x",
            ),
            (
                "if a | grep -q x; then echo y; fi",
                "if a | __roko_drain grep -q x; then echo y; fi",
            ),
            ("! a | grep -q x", "! a | __roko_drain grep -q x"),
            (
                "diff <(a | head -5) expected.txt",
                "diff <(a | __roko_drain head -5) expected.txt",
            ),
            (
                "while read -r f; do echo \"$f\" | grep -q x && n=1; done < list",
                "while read -r f; do echo \"$f\" | __roko_drain grep -q x && n=1; done < list",
            ),
            (
                "[[ $v =~ ^(a|b)$ ]] && a | grep -q x",
                "[[ $v =~ ^(a|b)$ ]] && a | __roko_drain grep -q x",
            ),
            (
                "n=$(( 1 | 2 )); (( n > 1 )) && a | awk 'NR==1{exit}'",
                "n=$(( 1 | 2 )); (( n > 1 )) && a | __roko_drain awk 'NR==1{exit}'",
            ),
            (
                "echo ${v//|/,} | sed q",
                "echo ${v//|/,} | __roko_drain sed q",
            ),
            (
                "python3 - <<'EOF' | grep -q OK\nprint('a | grep -q b')\nEOF\na | head -1",
                "python3 - <<'EOF' | __roko_drain grep -q OK\nprint('a | grep -q b')\nEOF\n\
                 a | __roko_drain head -1",
            ),
            (
                "cat <<-EOF | grep -q x\n\tx | head\n\tEOF",
                "cat <<-EOF | __roko_drain grep -q x\n\tx | head\n\tEOF",
            ),
            (
                "a 2>&1 >| out.log | grep -q x",
                "a 2>&1 >| out.log | __roko_drain grep -q x",
            ),
        ];
        for (command, expected) in cases {
            assert_eq!(drained(command).as_deref(), Some(expected), "{command:?}");
        }
    }

    #[test]
    fn commands_without_a_piped_reader_run_unchanged() {
        let unchanged = [
            "",
            "grep -q x file",
            "cargo test 2>&1 | tail -n 20",
            "a | xargs grep -q x",
            "a || grep -q x file",
            "a && head -1 file",
            "echo 'a | grep -q x'",
            "echo \"a | grep -q x\"",
            "echo a\\|grep -q x",
            "a | \"grep\" -q x",
            "a | g\\rep -q x",
            "a | 2>/dev/null grep -q x",
            "a | grepx -q x",
            "a >| out | tee log",
            "[[ $v == a || $v == b ]]",
            "echo $(( 7 | 8 ))",
            "test -f demo/fibonacci/README.md && for h in '# fibonacci' '## Usage'; do \
             grep -Fqx \"$h\" demo/fibonacci/README.md || exit 1; done",
            "python3 -c 'import re; assert re.search(r\"\\|.*Minute.*\\|\", \"| Minute |\")'",
        ];
        for command in unchanged {
            assert!(
                matches!(drain_early_readers(command), Cow::Borrowed(_)),
                "{command:?}"
            );
        }
    }

    #[test]
    fn syntax_the_scan_does_not_follow_runs_unchanged() {
        let unsupported = [
            "case $v in a|b) a | grep -q x ;; esac",
            "a | grep -q `echo x`",
            "echo \"`a | grep x`\"",
            "shopt -s extglob; ls @(a|b) | grep -q x",
            "arr=(a b); a | grep -q x",
            "f() { a | grep -q x; }; f",
            "echo $[1|2] | grep -q x",
            "a | grep -q 'unterminated",
            "a | grep -q \"unterminated",
            "(a | grep -q x",
            "a) | grep -q x",
            "cat <<EOF | grep -q x\nno delimiter",
        ];
        for command in unsupported {
            assert!(
                matches!(drain_early_readers(command), Cow::Borrowed(_)),
                "{command:?}"
            );
        }
    }

    #[test]
    fn assignments_are_told_from_command_names() {
        for word in ["A=1", "_x+=y", "a[1]=z", "LC_ALL=C", "x=$(a)"] {
            assert!(is_assignment(word.as_bytes()), "{word}");
        }
        for word in ["=x", "1a=x", "grep", "a[1=x", "./a=b", "a-b=c"] {
            assert!(!is_assignment(word.as_bytes()), "{word}");
        }
    }

    /// bug-11901c: a writer with output left after the line `grep -q` stops at dies of a broken
    /// pipe under `pipefail`, every run, so the step fails although the check held. Drained, the
    /// step passes.
    #[test]
    fn a_reader_that_stops_early_no_longer_fails_the_step() {
        let command = format!("{LONG_WRITER} | grep -q OK");
        let undrained = bash(&undrained_args(&command));
        assert!(!undrained.status.success(), "{undrained:?}");

        for _ in 0..20 {
            let output = bash(&bash_args(&command));
            assert!(output.status.success(), "{output:?}");
        }
    }

    /// The drain hides only the broken pipe: a writer that fails still fails the step with its
    /// own status, and a check that does not hold still fails it.
    #[test]
    fn a_failing_writer_or_check_still_fails_the_step() {
        let failing_writer = bash(&bash_args(
            "{ echo OK; seq 1 200000; exit 3; } | grep -q OK",
        ));
        assert_eq!(failing_writer.status.code(), Some(3), "{failing_writer:?}");

        let missing = bash(&bash_args(&format!("{LONG_WRITER} | grep -q NOT-THERE")));
        assert_eq!(missing.status.code(), Some(1), "{missing:?}");

        let negated = bash(&bash_args(&format!("! {LONG_WRITER} | grep -q OK")));
        assert_eq!(negated.status.code(), Some(1), "{negated:?}");
    }

    /// A writer that never ends still fails at once rather than running until the step times
    /// out, and a process the writer leaves holding the pipe does not hold up the step.
    #[test]
    fn the_drain_neither_waits_on_an_endless_writer_nor_on_a_leftover_process() {
        let started = Instant::now();
        let endless = bash(&bash_args("yes | head -1 >/dev/null"));
        assert!(!endless.status.success(), "{endless:?}");

        // The leftover subshell writes to the pipe; only its stderr is redirected.
        let leftover = Command::new("bash")
            .args(bash_args(
                "{ echo OK; (sleep 30; echo late) 2>/dev/null & } | grep -q OK",
            ))
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .expect("run bash");
        assert!(leftover.success());
        assert!(
            started.elapsed() < Duration::from_secs(20),
            "took {:?}",
            started.elapsed()
        );
    }
}
