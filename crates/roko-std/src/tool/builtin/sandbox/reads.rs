//! The reads a bash command makes of whole trees and of lists it cannot see.
//!
//! These are the Claude CLI command guard's rules for a roko config file that
//! holds a secret (`claude_cli_guard.py` in roko-agent), for
//! [`super::refuse_key_file_in_command`]. The guard and this module check the
//! same table of commands (`secret_read_cases.txt`). A command is refused
//! when it reads such a file:
//!
//! - a recursive search of a tree that holds one: `grep -r` (`-R`,
//!   `--recursive`, `-d recurse`, `rgrep`), `rg`, `ag`, `ack` and `git grep`,
//!   unless the search's glob, type or file filter leaves `roko.toml` out;
//! - a read (`cat`, `head`, `grep`, `cp` and the like) of a list the check
//!   cannot see: what `find -exec` or `fd -x` runs on, what a pipe from
//!   `find` or `fd` or a `$(find …)` hands a command, and what `xargs` or
//!   `parallel` reads. The list comes from find's starting points or fd's
//!   paths, or from the command's directory, unless a find name test or an fd
//!   pattern or extension leaves `roko.toml` out.
//!
//! A tree holds a config when its `roko.toml` does, or when it holds the
//! workspace's (found from the directory upwards), the file `ROKO_CONFIG`
//! names or the legacy `~/.config/roko/config.toml`. A command after a
//! literal `cd` or `pushd` is judged where it runs, and braces and globs in
//! a search's operands are expanded as the shell would. Commands that
//! wrappers (`sudo`, `timeout`, `xargs`), shells (`sh -c`), `eval` and
//! substitutions run are checked too.

use std::path::{Component, Path, PathBuf};

use regex::Regex;
use roko_core::child_env::is_config_with_secrets;
use roko_core::tool::ToolError;

use super::super::glob::segment_match;
use super::{MAX_COMMAND_NESTING, ends_word, normalize, resolve_symlinks, word_path};

/// Words that can start a command without being its program.
const SHELL_KEYWORDS: &[&str] = &[
    "if", "then", "else", "elif", "do", "while", "until", "!", "{", "}", "time",
];
/// Programs that run another program named later in their arguments.
const WRAPPERS: &[&str] = &[
    "sudo",
    "doas",
    "env",
    "command",
    "builtin",
    "exec",
    "nohup",
    "nice",
    "ionice",
    "time",
    "timeout",
    "gtimeout",
    "xargs",
    "stdbuf",
    "unbuffer",
    "chronic",
    "caffeinate",
    "watch",
    "flock",
    "su",
    "runuser",
    "script",
    "sg",
    "parallel",
];
/// Wrappers that hand every argument to a shell as one command line.
const STRING_WRAPPERS: &[&str] = &["watch", "sg", "parallel"];
/// Wrappers that run their command on each item of a list they read.
const BULK_RUNNERS: &[&str] = &["xargs", "parallel"];
/// Wrapper options whose value is a command line (`flock -c`, `su -c`).
const COMMAND_OPTIONS: &[&str] = &["-c", "--command", "-S", "--split-string"];
/// Wrapper options whose value is a user or group, never the program.
const USER_OPTIONS: &[&str] = &["-u", "-g", "-U", "--user", "--group", "--other-user"];
/// Wrapper options whose value is the next word (`sudo -u git`, `xargs -a
/// list`), so a `git` there is no program.
const VALUE_OPTIONS: &[&str] = &[
    "-u",
    "-g",
    "-U",
    "--user",
    "--group",
    "--other-user",
    "-a",
    "--arg-file",
    "-d",
    "--delimiter",
    "-E",
    "-I",
    "-L",
    "-n",
    "--max-args",
    "-P",
    "--max-procs",
    "-s",
    "--max-chars",
    "-C",
    "--chdir",
    "-D",
    "-h",
    "-p",
    "-r",
    "-t",
    "-T",
    "-k",
    "--kill-after",
    "--signal",
    "-w",
    "--timeout",
];
/// Multi-call binaries whose first argument names the program.
const MULTICALL: &[&str] = &["busybox", "toybox"];
/// Programs that run their arguments as shell command lines.
const SHELLS: &[&str] = &["sh", "bash", "zsh", "dash", "ksh", "fish"];
/// Programs that list a tree's files, and the find actions and fd options
/// that run a command on each one.
const FINDERS: &[&str] = &["find", "fd", "fdfind"];
const FIND_EXEC: &[&str] = &["-exec", "-execdir", "-ok", "-okdir"];
const FD_EXEC: &[&str] = &["-x", "--exec", "-X", "--exec-batch"];
/// find's tests on a file's name or path.
const FIND_NAME_TESTS: &[&str] = &[
    "-name",
    "-iname",
    "-path",
    "-ipath",
    "-wholename",
    "-iwholename",
    "-regex",
    "-iregex",
];
/// fd's options that take a value.
const FD_SHORT_VALUES: &str = "etEdSojc";
const FD_LONG_VALUES: &[&str] = &[
    "--extension",
    "--type",
    "--exclude",
    "--max-depth",
    "--min-depth",
    "--exact-depth",
    "--size",
    "--changed-within",
    "--changed-before",
    "--owner",
    "--threads",
    "--max-results",
    "--color",
    "--base-directory",
    "--path-separator",
    "--search-path",
    "--format",
    "--batch-size",
    "--ignore-file",
];
/// git's global options whose value is the next word.
const GIT_VALUE_OPTIONS: &[&str] = &[
    "-C",
    "-c",
    "--git-dir",
    "--work-tree",
    "--namespace",
    "--super-prefix",
    "--config-env",
    "--attr-source",
];
/// git grep's long options that take a value.
const GIT_GREP_LONG_VALUES: &[&str] = &[
    "--max-count",
    "--after-context",
    "--before-context",
    "--context",
    "--max-depth",
    "--threads",
];
/// Words that open and close a compound command, to follow a pipe into a
/// loop (`find . | while read f; do cat "$f"; done`).
const BLOCK_OPENERS: &[&str] = &["while", "until", "for", "select", "if", "case", "{"];
const BLOCK_CLOSERS: &[&str] = &["done", "fi", "esac", "}"];
/// Programs that print or copy the files they are given, besides the
/// searchers, and so read a secret when a list they run on names one.
const READERS: &[&str] = &[
    "cat", "tac", "head", "tail", "less", "more", "nl", "sed", "awk", "gawk", "cut", "sort", "uniq",
    "strings", "od", "xxd", "hexdump", "base64", "diff", "paste", "jq", "yq", "cp", "rsync", "tar",
    "zip",
];
/// Most words a brace expansion or a glob in one component may yield.
const EXPANSION_LIMIT: usize = 1024;

/// A program that searches file contents.
struct Searcher {
    /// The program, `grep` standing for its variants.
    name: &'static str,
    /// Short options that take a value (`-e PATTERN`), as letters.
    short_values: &'static str,
    /// Long options that take a value.
    long_values: &'static [&'static str],
    /// Options with which it lists file names and reads no file.
    lists_names: &'static [&'static str],
    /// Options that give the pattern, so that no operand is.
    pattern_options: &'static [&'static str],
}

const SEARCHERS: &[Searcher] = &[
    Searcher {
        name: "grep",
        short_values: "efmABCdD",
        long_values: &[
            "--regexp",
            "--file",
            "--max-count",
            "--after-context",
            "--before-context",
            "--context",
            "--directories",
            "--devices",
            "--include",
            "--exclude",
            "--exclude-dir",
            "--exclude-from",
            "--label",
            "--binary-files",
            "--group-separator",
        ],
        lists_names: &[],
        pattern_options: &["-e", "-f", "--regexp", "--file"],
    },
    Searcher {
        name: "rg",
        short_values: "efgtTmABCjMdEr",
        long_values: &[
            "--regexp",
            "--file",
            "--glob",
            "--iglob",
            "--type",
            "--type-not",
            "--type-add",
            "--type-clear",
            "--max-count",
            "--after-context",
            "--before-context",
            "--context",
            "--threads",
            "--max-columns",
            "--max-depth",
            "--maxdepth",
            "--encoding",
            "--engine",
            "--sort",
            "--sortr",
            "--pre",
            "--pre-glob",
            "--color",
            "--colors",
            "--context-separator",
            "--field-context-separator",
            "--field-match-separator",
            "--path-separator",
            "--replace",
            "--max-filesize",
            "--dfa-size-limit",
            "--regex-size-limit",
            "--ignore-file",
            "--hostname-bin",
            "--hyperlink-format",
        ],
        lists_names: &["--files", "--type-list"],
        pattern_options: &["-e", "-f", "--regexp", "--file"],
    },
    Searcher {
        name: "ag",
        short_values: "ABCGgmpW",
        long_values: &[
            "--after",
            "--before",
            "--context",
            "--file-search-regex",
            "--max-count",
            "--path-to-ignore",
            "--depth",
            "--ignore",
            "--ignore-dir",
            "--pager",
            "--width",
        ],
        lists_names: &["-g", "--list-file-types"],
        pattern_options: &[],
    },
    Searcher {
        name: "ack",
        short_values: "ABCgmt",
        long_values: &[
            "--after-context",
            "--before-context",
            "--context",
            "--max-count",
            "--type",
            "--ignore-dir",
            "--ignore-file",
            "--match",
            "--output",
            "--pager",
        ],
        lists_names: &["-g", "-f", "--help-types"],
        pattern_options: &["--match"],
    },
];

/// The searcher that `program` runs as: grep's variants run as grep.
fn searcher(program: &str) -> Option<&'static Searcher> {
    let name = match program {
        "egrep" | "fgrep" | "rgrep" => "grep",
        name => name,
    };
    SEARCHERS.iter().find(|searcher| searcher.name == name)
}

/// Whether `program` prints or copies the files it is given.
fn reads_files(program: &str) -> bool {
    READERS.contains(&program) || searcher(program).is_some()
}

/// Whether the wrapper scan checks a command that `program` starts.
fn is_checked(program: &str) -> bool {
    reads_files(program)
        || program == "git"
        || program == "eval"
        || FINDERS.contains(&program)
        || SHELLS.contains(&program)
        || MULTICALL.contains(&program)
        || WRAPPERS.contains(&program)
}

/// A token of a command line.
pub(super) enum Token {
    /// A word, with quotes and escapes removed.
    Word(String),
    /// An operator that ends a command: `;`, `&`, `&&`, `|`, `||`, `|&`,
    /// `(`, `)`, a newline or a backquote.
    Separator(String),
    /// A redirection operator (`<`, `>`, `>>`); the command keeps it as a word.
    Redirect(String),
}

impl Token {
    /// The word this token is, if it is one.
    pub(super) fn into_word(self) -> Option<String> {
        match self {
            Self::Word(word) => Some(word),
            Self::Separator(_) | Self::Redirect(_) => None,
        }
    }
}

/// The tokens of `text`, a shell command line, or `None` when a quote is
/// left open.
pub(super) fn shell_tokens(text: &str) -> Option<Vec<Token>> {
    let mut tokens = Vec::new();
    let mut word = String::new();
    let mut in_word = false;
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\'' => {
                in_word = true;
                loop {
                    match chars.next()? {
                        '\'' => break,
                        c => word.push(c),
                    }
                }
            }
            '"' => {
                in_word = true;
                loop {
                    match chars.next()? {
                        '"' => break,
                        '\\' => match chars.next()? {
                            '\n' => {}
                            c @ ('$' | '`' | '"' | '\\') => word.push(c),
                            c => {
                                word.push('\\');
                                word.push(c);
                            }
                        },
                        c => word.push(c),
                    }
                }
            }
            '\\' => match chars.next() {
                Some('\n') | None => {}
                Some(c) => {
                    in_word = true;
                    word.push(c);
                }
            },
            c if ends_word(c) => {
                if in_word {
                    tokens.push(Token::Word(std::mem::take(&mut word)));
                    in_word = false;
                }
                if c == '\n' {
                    tokens.push(Token::Separator(";".to_string()));
                } else if !c.is_ascii_whitespace() {
                    let mut operator = c.to_string();
                    if let Some(&next) = chars.peek() && is_doubled_operator(c, next) {
                        operator.push(next);
                        chars.next();
                    }
                    tokens.push(if matches!(c, '<' | '>') {
                        Token::Redirect(operator)
                    } else {
                        Token::Separator(operator)
                    });
                }
            }
            c => {
                in_word = true;
                word.push(c);
            }
        }
    }
    if in_word {
        tokens.push(Token::Word(word));
    }
    Some(tokens)
}

/// Whether `c` and `next` make one operator (`&&`, `||`, `|&`, `>>`, `<<`).
const fn is_doubled_operator(c: char, next: char) -> bool {
    matches!((c, next), ('&', '&') | ('|', '|' | '&') | ('>', '>') | ('<', '<'))
}

/// The tokens of `text`; with a quote left open, as in a heredoc, those of
/// the text without its quotes.
fn tokens(text: &str) -> Vec<Token> {
    shell_tokens(text)
        .or_else(|| shell_tokens(&text.replace(['\'', '"'], "")))
        .unwrap_or_default()
}

/// One command of a command line.
struct SimpleCommand {
    words: Vec<String>,
    /// The words of the find or fd command whose output a pipe brings this
    /// command, directly or into a loop.
    fed: Option<Vec<String>>,
}

/// The commands of `text`, split at its operators.
fn simple_commands(text: &str) -> Vec<SimpleCommand> {
    let mut commands = Vec::new();
    let mut words: Vec<String> = Vec::new();
    let mut finds: Option<Vec<String>> = None;
    let mut fed: Option<Vec<String>> = None;
    // Compound commands opened since the pipe.
    let mut blocks = 0_usize;
    for token in tokens(text) {
        let operator = match token {
            Token::Word(word) | Token::Redirect(word) => {
                if fed.is_some() && command_start(&words).is_empty() {
                    if BLOCK_OPENERS.contains(&word.as_str()) {
                        blocks += 1;
                    } else if BLOCK_CLOSERS.contains(&word.as_str()) && blocks > 0 {
                        blocks -= 1;
                    }
                }
                words.push(word);
                continue;
            }
            Token::Separator(operator) => operator,
        };
        if !words.is_empty() {
            if finds.is_none() && runs_finder(&words) {
                finds = Some(words.clone());
            }
            commands.push(SimpleCommand {
                words: std::mem::take(&mut words),
                fed: fed.clone(),
            });
        }
        match operator.as_str() {
            "|" | "|&" => {
                if fed.is_none() {
                    fed.clone_from(&finds);
                }
            }
            "(" => {
                if fed.is_some() {
                    blocks += 1;
                }
            }
            ")" => {
                if fed.is_some() && blocks > 0 {
                    blocks -= 1;
                }
            }
            _ if blocks == 0 => {
                finds = None;
                fed = None;
            }
            _ => {}
        }
    }
    if !words.is_empty() {
        commands.push(SimpleCommand { words, fed });
    }
    commands
}

/// The command lines that `$(…)` and backquote substitutions in `text` run.
fn substitutions(text: &str) -> Vec<&str> {
    let bytes = text.as_bytes();
    let mut found = Vec::new();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'$' && bytes.get(index + 1) == Some(&b'(') {
            let start = index + 2;
            let mut depth = 1_usize;
            let mut end = start;
            while end < bytes.len() && depth > 0 {
                match bytes[end] {
                    b'(' => depth += 1,
                    b')' => depth -= 1,
                    _ => {}
                }
                end += 1;
            }
            if depth > 0 {
                found.push(&text[start..]);
                break;
            }
            found.push(&text[start..end - 1]);
            index = end;
        } else if bytes[index] == b'`' {
            let start = index + 1;
            let Some(length) = text[start..].find('`') else {
                found.push(&text[start..]);
                break;
            };
            found.push(&text[start..start + length]);
            index = start + length + 1;
        } else {
            index += 1;
        }
    }
    found
}

/// `words` without the keywords and assignments that can precede a program.
fn command_start(words: &[String]) -> &[String] {
    let skipped = words
        .iter()
        .take_while(|word| SHELL_KEYWORDS.contains(&word.as_str()) || is_assignment(word))
        .count();
    &words[skipped..]
}

/// Whether `word` assigns a variable (`NAME=value`).
fn is_assignment(word: &str) -> bool {
    word.split_once('=').is_some_and(|(name, _)| {
        name.chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
            && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
    })
}

/// The program a word names: its last path component (`/usr/bin/grep`).
fn program_name(word: &str) -> &str {
    word.rsplit('/').next().unwrap_or(word)
}

/// Whether a command runs find or fd, directly or through a wrapper.
fn runs_finder(words: &[String]) -> bool {
    let words = command_start(words);
    let Some(first) = words.first() else {
        return false;
    };
    let program = program_name(first);
    if WRAPPERS.contains(&program) || MULTICALL.contains(&program) {
        words[1..]
            .iter()
            .any(|word| FINDERS.contains(&program_name(word)))
    } else {
        FINDERS.contains(&program)
    }
}

/// Where a list the check cannot see comes from: the roots it names files
/// under, and whether a filter leaves `roko.toml` out of it.
#[derive(Clone, Debug)]
struct ListSource {
    roots: Vec<String>,
    excludes_config: bool,
}

impl ListSource {
    /// A list of anything under the command's directory (`ls | xargs cat`).
    fn anything() -> Self {
        Self {
            roots: vec![".".to_string()],
            excludes_config: false,
        }
    }
}

/// The state of one command line's check.
struct Walk {
    /// The directory the command being checked runs in.
    dir: PathBuf,
    /// Where the list comes from while a command that runs on a list the
    /// check cannot see is checked.
    list: Option<ListSource>,
}

/// Refuse `command`, a shell command line run in `cwd`, when it searches a
/// tree or reads a list that holds a roko config file with a secret (see
/// the module doc).
///
/// # Errors
///
/// Returns [`ToolError::KeyFileBlocked`] naming the config file.
pub(super) fn refuse_secret_reads(command: &str, cwd: &Path) -> Result<(), ToolError> {
    let mut walk = Walk {
        dir: cwd.to_path_buf(),
        list: None,
    };
    check_command(&mut walk, command, 0)
}

fn check_command(walk: &mut Walk, text: &str, depth: usize) -> Result<(), ToolError> {
    if depth > MAX_COMMAND_NESTING {
        return Ok(());
    }
    // A nested command line (`sh -c '…'`, `$(…)`) starts in the directory
    // around it, and its `cd` does not move the commands after it.
    let start = walk.dir.clone();
    // A substitution can hand a command what find lists (`cat $(find .)`).
    let mut listed = None;
    for inner in substitutions(text) {
        check_command(walk, inner, depth + 1)?;
        if listed.is_none() {
            listed = simple_commands(inner)
                .into_iter()
                .find(|command| runs_finder(&command.words))
                .map(|command| command.words);
        }
    }
    for command in simple_commands(text) {
        match command.fed.as_deref().or(listed.as_deref()) {
            Some(finder) => check_found(walk, &command.words, depth, Some(finder))?,
            None => check_words(walk, &command.words, depth)?,
        }
        change_directory(walk, &command.words);
    }
    if depth > 0 {
        walk.dir = start;
    }
    Ok(())
}

fn check_words(walk: &mut Walk, words: &[String], depth: usize) -> Result<(), ToolError> {
    let words = command_start(words);
    let Some(first) = words.first() else {
        return Ok(());
    };
    let program = program_name(first);
    if reads_files(program) {
        check_listed_read(walk)?;
    }
    if MULTICALL.contains(&program) {
        check_words(walk, &words[1..], depth)
    } else if WRAPPERS.contains(&program) {
        check_wrapped(walk, program, words, depth)
    } else if program == "git" {
        check_git(walk, &words[1..])
    } else if program == "find" {
        check_find(walk, &words[1..], depth)
    } else if FINDERS.contains(&program) {
        check_fd(walk, &words[1..], depth)
    } else if let Some(found) = searcher(program) {
        check_search(walk, program, found, &words[1..])
    } else if SHELLS.contains(&program) {
        for argument in &words[1..] {
            if !argument.starts_with('-') {
                check_command(walk, argument, depth + 1)?;
            }
        }
        Ok(())
    } else if program == "eval" {
        check_command(walk, &words[1..].join(" "), depth + 1)
    } else {
        Ok(())
    }
}

/// Check the commands a wrapper (`words[0]`, named `program`) runs.
///
/// An option's value can name a program too (`xargs -a git …`), so every
/// later word that names a program this check follows starts a command to
/// check, except a user or group (`sudo -u git`) and the words git itself
/// reads: its global options' values and its subcommand. A command handed
/// over as one string (`watch 'grep -r x .'`, `flock l -c '…'`) is checked
/// as a command line. `xargs` and `parallel` run their command on a list
/// they read.
fn check_wrapped(
    walk: &mut Walk,
    program: &str,
    words: &[String],
    depth: usize,
) -> Result<(), ToolError> {
    let source = BULK_RUNNERS.contains(&program).then(ListSource::anything);
    with_list(walk, source, |walk| {
        let mut git_words = Vec::new();
        for index in 1..words.len() {
            if git_words.contains(&index) {
                continue;
            }
            let (word, previous) = (&words[index], words[index - 1].as_str());
            let name = program_name(word);
            if name == "git" && !VALUE_OPTIONS.contains(&previous) {
                git_words.extend(git_argument_indices(words, index));
            }
            if is_checked(name) && !USER_OPTIONS.contains(&previous) {
                check_words(walk, &words[index..], depth)?;
            }
            let (option, value) = word.split_once('=').unwrap_or((word.as_str(), ""));
            if STRING_WRAPPERS.contains(&program) || COMMAND_OPTIONS.contains(&previous) {
                check_command(walk, word, depth + 1)?;
            } else if COMMAND_OPTIONS.contains(&option) && !value.is_empty() {
                check_command(walk, value, depth + 1)?;
            }
        }
        Ok(())
    })
}

/// The indices of the words that the git at `words[index]` reads before its
/// subcommand's arguments: its global options' values and the subcommand.
fn git_argument_indices(words: &[String], index: usize) -> Vec<usize> {
    let mut indices = Vec::new();
    let mut index = index + 1;
    while words.get(index).is_some_and(|word| word.starts_with('-')) {
        if GIT_VALUE_OPTIONS.contains(&words[index].as_str()) {
            indices.push(index + 1);
            index += 2;
        } else {
            index += 1;
        }
    }
    indices.push(index);
    indices
}

/// Run `check` while a command runs on a list the check cannot see, from
/// `source` (the command's directory when `None`); an outer list stays.
/// With no `source` and no list, nothing changes.
fn with_list<T>(
    walk: &mut Walk,
    source: Option<ListSource>,
    check: impl FnOnce(&mut Walk) -> T,
) -> T {
    let previous = walk.list.clone();
    if walk.list.is_none() {
        walk.list = source;
    }
    let result = check(walk);
    walk.list = previous;
    result
}

/// Check a command that runs on what the find or fd command `finder`
/// lists (`find -exec`, `fd -x`, `find . | xargs`, `$(find …)`).
fn check_found(
    walk: &mut Walk,
    command: &[String],
    depth: usize,
    finder: Option<&[String]>,
) -> Result<(), ToolError> {
    let source = finder
        .and_then(finder_list)
        .unwrap_or_else(ListSource::anything);
    with_list(walk, Some(source), |walk| check_words(walk, command, depth))
}

/// Refuse a read of the list the command runs on when it may name a roko
/// config file that holds a secret.
fn check_listed_read(walk: &Walk) -> Result<(), ToolError> {
    match &walk.list {
        Some(list) if !list.excludes_config => check_search_roots(&list.roots, &walk.dir),
        _ => Ok(()),
    }
}

/// Where the list a find or fd command prints comes from.
fn finder_list(words: &[String]) -> Option<ListSource> {
    let words = command_start(words);
    words.iter().enumerate().find_map(|(index, word)| {
        let name = program_name(word);
        if name == "find" {
            Some(find_list(&words[index + 1..]))
        } else if FINDERS.contains(&name) {
            Some(fd_list(&words[index + 1..]))
        } else {
            None
        }
    })
}

/// find's starting points, and whether its name tests leave `roko.toml`
/// out: at least one test, none matching it, and no `-not` or `!` to turn
/// one around.
fn find_list(arguments: &[String]) -> ListSource {
    let mut index = 0;
    while let Some(argument) = arguments.get(index)
        && (matches!(argument.as_str(), "-H" | "-L" | "-P" | "-D") || argument.starts_with("-O"))
    {
        index += if argument == "-D" { 2 } else { 1 };
    }
    let mut roots: Vec<String> = arguments
        .iter()
        .skip(index)
        .take_while(|argument| {
            !argument.starts_with('-') && !matches!(argument.as_str(), "(" | "!" | ",")
        })
        .cloned()
        .collect();
    if roots.is_empty() {
        roots.push(".".to_string());
    }
    let tests: Vec<(&str, &str)> = arguments
        .windows(2)
        .filter(|pair| FIND_NAME_TESTS.contains(&pair[0].as_str()))
        .map(|pair| (pair[0].as_str(), pair[1].as_str()))
        .collect();
    let negated = arguments
        .iter()
        .any(|argument| argument == "-not" || argument == "!");
    let excludes_config = !tests.is_empty()
        && !negated
        && !tests
            .iter()
            .any(|(test, pattern)| find_test_matches_config(test, pattern));
    ListSource {
        roots,
        excludes_config,
    }
}

/// Whether a find name test (`-name '*.toml'`) can match `roko.toml`.
fn find_test_matches_config(test: &str, pattern: &str) -> bool {
    let pattern = if matches!(test, "-iname" | "-ipath" | "-iwholename" | "-iregex") {
        pattern.to_lowercase()
    } else {
        pattern.to_string()
    };
    match test {
        "-name" | "-iname" => glob_match(&pattern, "roko.toml"),
        "-regex" | "-iregex" => Regex::new(&pattern)
            .ok()
            .is_none_or(|regex| regex.is_match("./roko.toml")),
        _ => glob_match(&pattern, "./roko.toml") || glob_match(&pattern, "roko.toml"),
    }
}

/// fd's search paths, and whether its pattern (a regex, or a glob with
/// `-g`) or its extensions (`-e`) leave `roko.toml` out.
fn fd_list(arguments: &[String]) -> ListSource {
    let (options, operands) = search_arguments(arguments, FD_SHORT_VALUES, FD_LONG_VALUES);
    let extensions: Vec<&str> = options
        .iter()
        .filter(|(name, _)| name == "-e" || name == "--extension")
        .map(|(_, value)| value.as_str())
        .collect();
    let mut roots: Vec<String> = operands.iter().skip(1).cloned().collect();
    roots.extend(
        options
            .iter()
            .filter(|(name, _)| name == "--search-path")
            .map(|(_, value)| value.clone()),
    );
    if roots.is_empty() {
        roots.push(".".to_string());
    }
    let mut excludes_config = !extensions.is_empty() && !extensions.contains(&"toml");
    if let Some(pattern) = operands.first().filter(|pattern| !pattern.is_empty()) {
        let globbing = options
            .iter()
            .any(|(name, _)| name == "-g" || name == "--glob");
        excludes_config = excludes_config
            || if globbing {
                !glob_match(pattern, "roko.toml")
            } else {
                Regex::new(pattern).is_ok_and(|regex| !regex.is_match("roko.toml"))
            };
    }
    ListSource {
        roots,
        excludes_config,
    }
}

fn check_find(walk: &mut Walk, arguments: &[String], depth: usize) -> Result<(), ToolError> {
    let finder: Vec<String> = std::iter::once("find".to_string())
        .chain(arguments.iter().cloned())
        .collect();
    for (index, argument) in arguments.iter().enumerate() {
        if FIND_EXEC.contains(&argument.as_str()) {
            let command: Vec<String> = arguments[index + 1..]
                .iter()
                .take_while(|word| !matches!(word.as_str(), ";" | "+"))
                .cloned()
                .collect();
            check_found(walk, &command, depth, Some(finder.as_slice()))?;
        }
    }
    Ok(())
}

/// `fd -x` runs every word after it, up to a `;`, on each result.
fn check_fd(walk: &mut Walk, arguments: &[String], depth: usize) -> Result<(), ToolError> {
    for (index, argument) in arguments.iter().enumerate() {
        let (option, value) = argument.split_once('=').unwrap_or((argument.as_str(), ""));
        if FD_EXEC.contains(&option) {
            let command: Vec<String> = (!value.is_empty())
                .then(|| value.to_string())
                .into_iter()
                .chain(arguments[index + 1..].iter().cloned())
                .collect();
            let finder: Vec<String> = std::iter::once("fd".to_string())
                .chain(arguments[..index].iter().cloned())
                .collect();
            return check_found(walk, &command, depth, Some(finder.as_slice()));
        }
    }
    Ok(())
}

/// A command's options, as (name, value) pairs, and its operands.
/// `short_values` and `long_values` are the options that take a value.
fn search_arguments(
    arguments: &[String],
    short_values: &str,
    long_values: &[&str],
) -> (Vec<(String, String)>, Vec<String>) {
    let mut options = Vec::new();
    let mut operands = Vec::new();
    let mut index = 0;
    while let Some(word) = arguments.get(index) {
        index += 1;
        if word == "--" {
            operands.extend(arguments[index..].iter().cloned());
            break;
        }
        if word.starts_with("--") {
            let (name, value) = match word.split_once('=') {
                Some((name, value)) => (name.to_string(), value.to_string()),
                None if long_values.contains(&word.as_str()) && index < arguments.len() => {
                    index += 1;
                    (word.clone(), arguments[index - 1].clone())
                }
                None => (word.clone(), String::new()),
            };
            options.push((name, value));
        } else if word.len() > 1 && word.starts_with('-') {
            // A cluster of short options; one that takes a value ends it
            // (`-rnm1`, `-re PATTERN`).
            for (position, letter) in word.char_indices().skip(1) {
                if short_values.contains(letter) {
                    let rest = &word[position + letter.len_utf8()..];
                    let value = if rest.is_empty() && index < arguments.len() {
                        index += 1;
                        arguments[index - 1].clone()
                    } else {
                        rest.to_string()
                    };
                    options.push((format!("-{letter}"), value));
                    break;
                }
                options.push((format!("-{letter}"), String::new()));
            }
        } else {
            operands.push(word.clone());
        }
    }
    (options, operands)
}

/// Refuse a recursive search (`grep -r`, `rg`, `ag`, `ack`) that reads a
/// roko config file holding a secret, unless its filters leave it out.
fn check_search(
    walk: &Walk,
    program: &str,
    searcher: &Searcher,
    arguments: &[String],
) -> Result<(), ToolError> {
    let (options, mut operands) =
        search_arguments(arguments, searcher.short_values, searcher.long_values);
    let names: Vec<&str> = options.iter().map(|(name, _)| name.as_str()).collect();
    if names.iter().any(|name| searcher.lists_names.contains(name)) {
        return Ok(());
    }
    let recursive = program == "rgrep"
        || names.iter().any(|name| {
            matches!(
                *name,
                "-r" | "-R" | "--recursive" | "--dereference-recursive"
            )
        })
        || options.iter().any(|(name, value)| {
            matches!(name.as_str(), "-d" | "--directories") && value == "recurse"
        });
    // A file grep names without recursing is checked with the other words.
    if searcher.name == "grep" && !recursive {
        return Ok(());
    }
    if !names.iter().any(|name| searcher.pattern_options.contains(name)) && !operands.is_empty() {
        operands.remove(0);
    }
    if search_skips_config(&options, searcher.name) {
        return Ok(());
    }
    if operands.is_empty() {
        operands.push(".".to_string());
    }
    check_search_roots(&operands, &walk.dir)
}

/// Whether a search's filters leave `roko.toml` out: grep's `--include` and
/// `--exclude`, rg's `-g` (`!` excludes), `-t` and `-T`, ag's `-G` and
/// `--ignore`, and ack's `-t` and `--ignore-file`.
fn search_skips_config(options: &[(String, String)], kind: &str) -> bool {
    let mut includes = Vec::new();
    let mut excludes = Vec::new();
    let mut types = Vec::new();
    let mut skipped_types = Vec::new();
    for (name, value) in options {
        match (kind, name.as_str()) {
            ("grep", "--include") => includes.push(value.as_str()),
            ("grep", "--exclude") | ("ag", "--ignore") => excludes.push(value.as_str()),
            ("rg", "-g" | "--glob" | "--iglob") => match value.strip_prefix('!') {
                Some(excluded) => excludes.push(excluded),
                None => includes.push(value.as_str()),
            },
            ("rg" | "ack", "-t" | "--type") => match value.strip_prefix("no") {
                Some(skipped) => skipped_types.push(skipped),
                None => types.push(value.as_str()),
            },
            ("rg", "-T" | "--type-not") => skipped_types.push(value.as_str()),
            ("ag", "-G" | "--file-search-regex") => {
                if Regex::new(value).is_ok_and(|regex| !regex.is_match("roko.toml")) {
                    return true;
                }
            }
            ("ack", "--ignore-file") if matches!(value.as_str(), "ext:toml" | "is:roko.toml") => {
                return true;
            }
            _ => {}
        }
    }
    (!includes.is_empty()
        && !includes
            .iter()
            .any(|pattern| glob_matches_config(pattern, false)))
        || excludes
            .iter()
            .any(|pattern| glob_matches_config(pattern, true))
        || (!types.is_empty() && !types.iter().any(|name| matches!(*name, "toml" | "all")))
        || skipped_types.contains(&"toml")
}

/// Whether a file glob (grep `--include`, rg `-g`) matches `roko.toml`.
/// Unless `strict`, a glob that names toml counts too, as a brace glob may
/// (`*.{rs,toml}`).
fn glob_matches_config(pattern: &str, strict: bool) -> bool {
    let lowered = pattern.to_lowercase();
    let mut pattern = lowered.trim_start_matches('/');
    while let Some(rest) = pattern.strip_prefix("**/") {
        pattern = rest;
    }
    glob_match(pattern, "roko.toml") || (!strict && pattern.contains("toml"))
}

/// Whether `text` matches the shell glob `pattern`, a `*` matching any run
/// of characters, `/` included.
fn glob_match(pattern: &str, text: &str) -> bool {
    segment_match(pattern.as_bytes(), text.as_bytes())
}

/// git's global options before its subcommand; `git grep` searches.
fn check_git(walk: &Walk, arguments: &[String]) -> Result<(), ToolError> {
    let mut directory = walk.dir.clone();
    let mut index = 0;
    while let Some(argument) = arguments.get(index)
        && argument.starts_with('-')
    {
        if GIT_VALUE_OPTIONS.contains(&argument.as_str()) {
            if argument == "-C"
                && let Some(value) = arguments.get(index + 1)
                && let Some(path) = word_path(value, &directory)
            {
                directory = normalize(&path);
            }
            index += 2;
        } else {
            index += 1;
        }
    }
    if arguments.get(index).is_some_and(|subcommand| subcommand == "grep") {
        check_git_grep(&arguments[index + 1..], &directory)?;
    }
    Ok(())
}

/// `git grep` reads the tracked files under the directory git runs in, or
/// under the paths and pathspecs it names; a revision it names reads that
/// commit's tree of the same directory.
fn check_git_grep(arguments: &[String], directory: &Path) -> Result<(), ToolError> {
    let (options, mut operands) = search_arguments(arguments, "efmABC", GIT_GREP_LONG_VALUES);
    if !options.iter().any(|(name, _)| name == "-e" || name == "-f") && !operands.is_empty() {
        operands.remove(0);
    }
    let mut roots = Vec::new();
    let mut narrowed = false;
    for operand in operands {
        if operand.contains(['*', '?', '[']) {
            narrowed = true;
            if glob_matches_config(&operand, false) {
                roots.push(".".to_string());
            }
        } else if directory.join(&operand).exists() {
            narrowed = true;
            roots.push(operand);
        }
    }
    if !narrowed {
        roots.push(".".to_string());
    }
    check_search_roots(&roots, directory)
}

/// Refuse a search of `roots`, resolved against `directory`, that reads a
/// roko config file holding a secret; a brace or a glob in a root is
/// expanded as the shell would.
fn check_search_roots(roots: &[String], directory: &Path) -> Result<(), ToolError> {
    for root in roots {
        for variant in expand_braces(root) {
            if variant.contains(['$', '`', '{']) {
                continue;
            }
            for path in expand(&variant, directory) {
                if let Some(config) = reads_secret_config(&path, directory) {
                    return Err(ToolError::KeyFileBlocked(config));
                }
            }
        }
    }
    Ok(())
}

/// The roko config file holding a secret that a recursive read of `path`
/// reaches: `path` itself, or one in the tree under it of those roko reads,
/// the `roko.toml` in `path`, the workspace's (in `cwd` or above it, as roko
/// finds it), the file `ROKO_CONFIG` names and the legacy
/// `~/.config/roko/config.toml`.
fn reads_secret_config(path: &Path, cwd: &Path) -> Option<PathBuf> {
    if !path.is_dir() {
        return [path.to_path_buf(), resolve_symlinks(path)]
            .into_iter()
            .find(|candidate| is_config_with_secrets(candidate));
    }
    let root = path.canonicalize().ok()?;
    let config_home = std::env::var_os("XDG_CONFIG_HOME")
        .filter(|dir| !dir.is_empty())
        .map(PathBuf::from)
        .or_else(|| word_path("~/.config", cwd));
    [
        Some(root.join("roko.toml")),
        workspace_config(cwd),
        std::env::var_os("ROKO_CONFIG").map(PathBuf::from),
        config_home.map(|dir| dir.join("roko").join("config.toml")),
    ]
    .into_iter()
    .flatten()
    .find(|config| {
        config
            .parent()
            .and_then(|dir| dir.canonicalize().ok())
            .is_some_and(|dir| dir.starts_with(&root))
            && is_config_with_secrets(config)
    })
}

/// The `roko.toml` roko loads in `cwd`: the first in `cwd` or above it.
fn workspace_config(cwd: &Path) -> Option<PathBuf> {
    let start = cwd.canonicalize().unwrap_or_else(|_| cwd.to_path_buf());
    start
        .ancestors()
        .map(|dir| dir.join("roko.toml"))
        .find(|config| config.is_file())
}

/// The paths `word` names, resolved against `cwd` (`~` against `HOME`): a
/// glob's matches, or the word itself.
pub(super) fn expand(word: &str, cwd: &Path) -> Vec<PathBuf> {
    if !word.contains(['*', '?', '[']) {
        return word_path(word, cwd).into_iter().collect();
    }
    if word.contains(['$', '`', '{']) {
        return Vec::new();
    }
    let pattern = match word.strip_prefix('~') {
        Some(rest) if rest.is_empty() || rest.starts_with('/') => {
            let Some(home) = word_path("~", cwd) else {
                return Vec::new();
            };
            home.join(rest.trim_start_matches('/'))
        }
        _ => cwd.join(word),
    };
    glob_paths(&pattern)
}

/// The existing paths that `pattern`, an absolute path with globs in some
/// of its components, matches. A glob does not match a hidden name unless
/// it starts with a dot.
fn glob_paths(pattern: &Path) -> Vec<PathBuf> {
    let mut found = vec![PathBuf::new()];
    for component in pattern.components() {
        let name = component.as_os_str().to_string_lossy();
        if !(matches!(component, Component::Normal(_)) && name.contains(['*', '?', '['])) {
            for path in &mut found {
                path.push(component.as_os_str());
            }
            continue;
        }
        let mut next = Vec::new();
        for dir in &found {
            let Ok(entries) = std::fs::read_dir(dir) else {
                continue;
            };
            for entry in entries.flatten() {
                let file_name = entry.file_name();
                let file_name = file_name.to_string_lossy();
                if (!file_name.starts_with('.') || name.starts_with('.'))
                    && glob_match(&name, &file_name)
                {
                    next.push(dir.join(entry.file_name()));
                }
                if next.len() >= EXPANSION_LIMIT {
                    break;
                }
            }
        }
        found = next;
    }
    found.retain(|path| path.exists());
    found
}

/// The words a shell brace expansion makes of `word` (`a{b,c}d` gives `abd`
/// and `acd`), or `word` itself when it has none. A `${…}` is no brace
/// expansion, and neither is a `{}` without a comma in it.
pub(super) fn expand_braces(word: &str) -> Vec<String> {
    let mut words = Vec::new();
    expand_braces_into(word, &mut words);
    words
}

fn expand_braces_into(word: &str, words: &mut Vec<String>) {
    let bytes = word.as_bytes();
    let mut depth = 0_usize;
    let mut start = 0;
    let mut commas = Vec::new();
    for (index, &byte) in bytes.iter().enumerate() {
        match byte {
            b'{' if index == 0 || bytes[index - 1] != b'$' => {
                if depth == 0 {
                    start = index;
                    commas.clear();
                }
                depth += 1;
            }
            b'}' if depth > 0 => {
                depth -= 1;
                if depth == 0 && !commas.is_empty() {
                    let mut last = start + 1;
                    for &comma in commas.iter().chain(std::iter::once(&index)) {
                        let part = &word[last..comma];
                        let variant = format!("{}{part}{}", &word[..start], &word[index + 1..]);
                        expand_braces_into(&variant, words);
                        if words.len() >= EXPANSION_LIMIT {
                            return;
                        }
                        last = comma + 1;
                    }
                    return;
                }
            }
            b',' if depth == 1 => commas.push(index),
            _ => {}
        }
    }
    words.push(word.to_string());
}

/// The directories the commands of a command line may run in: `cwd`, and
/// each literal `cd` or `pushd` target in it, nested command lines
/// included.
pub(super) fn call_directories(command: &str, cwd: &Path) -> Vec<PathBuf> {
    let mut directories = vec![cwd.to_path_buf()];
    collect_directories(command, cwd, 0, &mut directories);
    directories
}

fn collect_directories(command: &str, cwd: &Path, depth: usize, directories: &mut Vec<PathBuf>) {
    let mut current = cwd.to_path_buf();
    for command in simple_commands(command) {
        if let Some(target) = cd_target(&command.words, &current) {
            current = target;
            directories.push(current.clone());
        }
        for word in &command.words {
            if depth < MAX_COMMAND_NESTING && word.contains(char::is_whitespace) {
                collect_directories(word, &current, depth + 1, directories);
            }
        }
    }
}

/// Where `words` moves to from `dir` when it is a `cd` or `pushd` to a
/// literal directory (home when it names none), else `None`: `cd -` and a
/// variable are unknown.
fn cd_target(words: &[String], dir: &Path) -> Option<PathBuf> {
    let words = command_start(words);
    let first = words.first()?;
    if !matches!(program_name(first), "cd" | "pushd") || words.iter().any(|word| word == "-") {
        return None;
    }
    let target = words[1..]
        .iter()
        .find(|word| !word.starts_with('-'))
        .map_or("~", String::as_str);
    word_path(target, dir).map(|path| normalize(&path))
}

/// Follow `words` when it is a `cd` or `pushd`, so the commands after it
/// are judged where they run.
fn change_directory(walk: &mut Walk, words: &[String]) {
    if let Some(target) = cd_target(words, &walk.dir) {
        walk.dir = target;
    }
}
