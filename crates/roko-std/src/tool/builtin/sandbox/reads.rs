//! The reads a bash command makes of whole trees and of lists it cannot see.
//!
//! These are the Claude CLI command guard's rules for provider key files and
//! for a roko config file that holds a secret (`claude_cli_guard.py` in
//! roko-agent), for [`super::refuse_key_file_in_command`]. The guard and this
//! module check the same table of commands (`secret_read_cases.txt`). A
//! command is refused when it reads such a file in a tree:
//!
//! - a recursive search: `grep -r` (`-R`, `--recursive`, `-d recurse`,
//!   `rgrep`), `rg`, `ag`, `ack`, and `git grep`, which reads untracked files,
//!   key files among them, only with `--untracked` or `--no-index`;
//! - a read (`cat`, `head`, `grep`, `cp` and the like) of a list the check
//!   cannot see: what `find -exec` or `fd -x` runs on, what a pipe from
//!   `find` or `fd` or a `$(find …)` hands a command, and what `xargs` or
//!   `parallel` reads. The list comes from find's starting points or fd's
//!   paths, or else from the command's directory, as `ls -a` lists it.
//!
//! A read reaches a file unless its filters leave the file out: grep's
//! `--include`, `--exclude` and `--exclude-dir`; rg's `-g` (`!` excludes),
//! `-t`, `-T` and `--max-depth`; ag's `-G`, `--ignore` and `--depth`; ack's
//! `-t`, `--ignore-dir` and `--ignore-file`; git grep's pathspecs and
//! `--max-depth`; find's expression (its name, path and type tests,
//! `-prune`, `-maxdepth`, with `!`, `-o` and parentheses); fd's pattern,
//! `-e`, `-E` and `-d`. Nor does it reach a hidden file when it skips those,
//! as rg (unless a glob names it), ag and fd do without `--hidden`, or a
//! hidden directory, as a list from `ls` does.
//!
//! A tree holds a config when its `roko.toml` does, or when it holds the
//! workspace's (found from the directory upwards), the file `ROKO_CONFIG`
//! names or the legacy `~/.config/roko/config.toml`. It holds the key files
//! in its `.roko`, in that of each subdirectory two levels down, and in the
//! workspace's (in the directory the command or the call runs in, or above
//! it) and `~/.roko` when it holds those. A command after a literal `cd` or
//! `pushd` is judged where it runs, and braces and globs in a search's
//! operands are expanded as the shell would. Commands that wrappers (`sudo`,
//! `timeout`, `xargs`), shells (`sh -c`), `eval` and substitutions run are
//! checked too.
//!
//! What the check cannot follow is refused: command lines nested deeper than
//! [`MAX_COMMAND_NESTING`], a brace expansion or a glob that yields more than
//! [`EXPANSION_LIMIT`] words, and a git subcommand that is not one of git's
//! commands, since an alias can run any read, and the same command line can
//! define one.

use std::path::{Component, Path, PathBuf};
use std::rc::Rc;
use std::sync::OnceLock;

use regex::{Regex, RegexBuilder};
use roko_core::child_env::{KEY_FILE_NAMES, is_config_with_secrets, is_key_file};
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
/// find's other primaries that take one argument (an `-newerXY` test takes
/// one too).
const FIND_ONE_ARGUMENT: &[&str] = &[
    "-amin",
    "-anewer",
    "-atime",
    "-cmin",
    "-cnewer",
    "-context",
    "-ctime",
    "-fls",
    "-fprint",
    "-fprint0",
    "-fstype",
    "-gid",
    "-group",
    "-ilname",
    "-inum",
    "-links",
    "-lname",
    "-maxdepth",
    "-mindepth",
    "-mmin",
    "-mtime",
    "-newer",
    "-perm",
    "-printf",
    "-regextype",
    "-samefile",
    "-size",
    "-type",
    "-uid",
    "-used",
    "-user",
    "-xtype",
];
/// find's primaries that hold true whatever the file: its actions and
/// options.
const FIND_TRUE: &[&str] = &[
    "-print",
    "-print0",
    "-printf",
    "-fprint",
    "-fprint0",
    "-fprintf",
    "-ls",
    "-fls",
    "-prune",
    "-true",
    "-depth",
    "-d",
    "-maxdepth",
    "-mindepth",
    "-xdev",
    "-mount",
    "-follow",
    "-noleaf",
    "-regextype",
    "-daystart",
    "-warn",
    "-nowarn",
    "-ignore_readdir_race",
    "-noignore_readdir_race",
];
/// find's options after which `-prune` prunes nothing.
const FIND_NO_PRUNE: &[&str] = &["-depth", "-d", "-delete"];
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
/// Commands git ships, which no alias can replace: git looks for an alias
/// only when it has no command of that name. The check asks git about the
/// others (`git --list-cmds=main,others`).
const GIT_COMMANDS: &[&str] = &[
    "add",
    "am",
    "apply",
    "bisect",
    "blame",
    "branch",
    "cat-file",
    "checkout",
    "cherry-pick",
    "clean",
    "clone",
    "commit",
    "config",
    "describe",
    "diff",
    "fetch",
    "format-patch",
    "grep",
    "help",
    "init",
    "log",
    "ls-files",
    "ls-remote",
    "ls-tree",
    "merge",
    "merge-base",
    "mv",
    "notes",
    "pull",
    "push",
    "rebase",
    "reflog",
    "remote",
    "reset",
    "restore",
    "rev-list",
    "rev-parse",
    "revert",
    "rm",
    "shortlog",
    "show",
    "show-ref",
    "stash",
    "status",
    "submodule",
    "switch",
    "tag",
    "worktree",
];
/// Words that open and close a compound command, to follow a pipe into a
/// loop (`find . | while read f; do cat "$f"; done`).
const BLOCK_OPENERS: &[&str] = &["while", "until", "for", "select", "if", "case", "{"];
const BLOCK_CLOSERS: &[&str] = &["done", "fi", "esac", "}"];
/// Programs that print or copy the files they are given, besides the
/// searchers, and so read a secret when a list they run on names one.
const READERS: &[&str] = &[
    "cat", "tac", "head", "tail", "less", "more", "nl", "sed", "awk", "gawk", "cut", "sort",
    "uniq", "strings", "od", "xxd", "hexdump", "base64", "diff", "paste", "jq", "yq", "cp",
    "rsync", "tar", "zip",
];
/// Most words a brace expansion, or paths a glob's component, may yield; a
/// command with more is refused, since the check would miss the rest.
const EXPANSION_LIMIT: usize = 4096;
/// Most subdirectories a level in which the check looks for a `.roko`
/// directory below a tree's root.
const KEY_SEARCH_DIRS: usize = 4096;

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
                    if let Some(&next) = chars.peek()
                        && is_doubled_operator(c, next)
                    {
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
    matches!(
        (c, next),
        ('&', '&') | ('|', '|' | '&') | ('>', '>') | ('<', '<')
    )
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

/// A file that a read of a tree may reach, as the read sees it.
struct TreeFile {
    /// Its path under the tree's root, `/`-separated.
    rel: String,
    /// The root as the program sees it (`.`, `src/..`).
    start: String,
    /// Where it is.
    path: PathBuf,
}

/// Where a list the check cannot see comes from: the roots it names files
/// under, and which files there it names.
#[derive(Clone)]
struct ListSource {
    roots: Vec<String>,
    reads: Rc<dyn Fn(&TreeFile) -> bool>,
}

impl ListSource {
    /// A list of anything under the command's directory (`ls | xargs cat`).
    fn anything() -> Self {
        Self {
            roots: vec![".".to_string()],
            reads: Rc::new(lists_visible),
        }
    }
}

/// Whether a list the check cannot see (`ls | xargs cat`, `xargs cat <
/// list`) may name the file at `rel`: one no hidden directory leads to, as
/// `ls -a` lists them.
fn lists_visible(file: &TreeFile) -> bool {
    !file
        .rel
        .split('/')
        .rev()
        .skip(1)
        .any(|part| part.starts_with('.'))
}

/// The state of one command line's check.
struct Walk {
    /// The directory the command being checked runs in.
    dir: PathBuf,
    /// The directory the whole command line runs in.
    call_dir: PathBuf,
    /// Where the list comes from while a command that runs on a list the
    /// check cannot see is checked.
    list: Option<ListSource>,
}

/// Refuse `command`, a shell command line run in `cwd`, when it searches a
/// tree or reads a list that holds a key file or a roko config file with a
/// secret, or does what the check cannot follow (see the module doc).
///
/// # Errors
///
/// Returns [`ToolError::KeyFileBlocked`] naming the file, or
/// [`ToolError::CommandNotAllowed`] for a command the check cannot follow.
pub(super) fn refuse_secret_reads(command: &str, cwd: &Path) -> Result<(), ToolError> {
    let mut walk = Walk {
        dir: cwd.to_path_buf(),
        call_dir: cwd.to_path_buf(),
        list: None,
    };
    check_command(&mut walk, command, 0)
}

fn check_command(walk: &mut Walk, text: &str, depth: usize) -> Result<(), ToolError> {
    if depth > MAX_COMMAND_NESTING {
        return Err(ToolError::CommandNotAllowed(
            "the command nests shells too deeply to check".to_string(),
        ));
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
            None => check_words(walk, &command.words, depth, false)?,
        }
        change_directory(walk, &command.words);
    }
    if depth > 0 {
        walk.dir = start;
    }
    Ok(())
}

/// Check one command. `maybe_argument`: its first word may be an argument
/// of a wrapper rather than the program the wrapper runs.
fn check_words(
    walk: &mut Walk,
    words: &[String],
    depth: usize,
    maybe_argument: bool,
) -> Result<(), ToolError> {
    let words = command_start(words);
    let Some(first) = words.first() else {
        return Ok(());
    };
    let program = program_name(first);
    if reads_files(program) {
        check_listed_read(walk)?;
    }
    if MULTICALL.contains(&program) {
        check_words(walk, &words[1..], depth, maybe_argument)
    } else if WRAPPERS.contains(&program) {
        check_wrapped(walk, program, words, depth, maybe_argument)
    } else if program == "git" {
        check_git(walk, &words[1..], maybe_argument)
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
/// reads: its global options' values and its subcommand. After a word that
/// is not an option, an assignment, a duration, a user or another wrapper,
/// such a word may be an argument instead (`timeout 5 grep git src`). A
/// command handed over as one string (`watch 'grep -r x .'`, `flock l -c
/// '…'`) is checked as a command line. `xargs` and `parallel` run their
/// command on a list they read.
fn check_wrapped(
    walk: &mut Walk,
    program: &str,
    words: &[String],
    depth: usize,
    maybe_argument: bool,
) -> Result<(), ToolError> {
    let source = BULK_RUNNERS.contains(&program).then(ListSource::anything);
    with_list(walk, source, |walk| {
        let mut after_argument = maybe_argument;
        let mut git_words = Vec::new();
        for index in 1..words.len() {
            if git_words.contains(&index) {
                continue;
            }
            let (word, previous) = (&words[index], words[index - 1].as_str());
            let is_user = USER_OPTIONS.contains(&previous);
            let name = program_name(word);
            if name == "git" && !VALUE_OPTIONS.contains(&previous) {
                git_words.extend(git_argument_indices(words, index));
            }
            if is_checked(name) && !is_user {
                check_words(walk, &words[index..], depth, after_argument)?;
            }
            let (option, value) = word.split_once('=').unwrap_or((word.as_str(), ""));
            if STRING_WRAPPERS.contains(&program) || COMMAND_OPTIONS.contains(&previous) {
                check_command(walk, word, depth + 1)?;
            } else if COMMAND_OPTIONS.contains(&option) && !value.is_empty() {
                check_command(walk, value, depth + 1)?;
            }
            if !(word.starts_with('-')
                || is_assignment(word)
                || is_duration(word)
                || is_user
                || WRAPPERS.contains(&name))
            {
                after_argument = true;
            }
        }
        Ok(())
    })
}

/// Whether `word` is a duration (`timeout 5`, `sleep 1.5s`).
fn is_duration(word: &str) -> bool {
    let number = word.strip_suffix(['s', 'm', 'h', 'd']).unwrap_or(word);
    !number.is_empty()
        && number
            .bytes()
            .all(|byte| byte.is_ascii_digit() || byte == b'.')
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
    with_list(walk, Some(source), |walk| {
        check_words(walk, command, depth, false)
    })
}

/// Refuse a read of the list the command runs on when it may name a key
/// file or a roko config file that holds a secret.
fn check_listed_read(walk: &Walk) -> Result<(), ToolError> {
    match &walk.list {
        Some(list) => check_tree_reads(&list.roots, &walk.dir, &walk.call_dir, &*list.reads),
        None => Ok(()),
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

/// find's starting points, and what it lists from them: a file its
/// expression may hold true, under no directory the expression surely
/// prunes, within `-maxdepth` and `-mindepth`. An expression the check
/// cannot parse lists every file.
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
    let words = arguments.get(index + roots.len()..).unwrap_or_default();
    if roots.is_empty() {
        roots.push(".".to_string());
    }
    let expression = FindParser::parse(words);
    let depth_option = |option: &str| {
        words
            .windows(2)
            .rev()
            .filter(|pair| pair[0] == option)
            .find_map(|pair| depth_limit(&pair[1]))
    };
    let max_depth = depth_option("-maxdepth");
    let min_depth = depth_option("-mindepth").unwrap_or(0);
    let prunes = !words
        .iter()
        .any(|word| FIND_NO_PRUNE.contains(&word.as_str()));
    let reads = move |file: &TreeFile| {
        let parts: Vec<&str> = file.rel.split('/').collect();
        if parts.len() < min_depth || max_depth.is_some_and(|limit| parts.len() > limit) {
            return false;
        }
        let Some(expression) = &expression else {
            return true;
        };
        let pruned = (1..parts.len()).any(|count| {
            let directory = printed_path(&file.start, &parts[..count].join("/"));
            prunes && expression.evaluate(&directory, Some("d")).1
        });
        let printed = printed_path(&file.start, &file.rel);
        !pruned && expression.evaluate(&printed, file_kind(&file.path)).0 != Some(false)
    };
    ListSource {
        roots,
        reads: Rc::new(reads),
    }
}

/// find's expression.
enum FindNode {
    Not(Box<Self>),
    /// `-a`, or two primaries side by side.
    And(Box<Self>, Box<Self>),
    Or(Box<Self>, Box<Self>),
    Comma(Box<Self>, Box<Self>),
    /// A primary and its argument; that of `-exec` is its `;` or `+`.
    Primary(String, Option<String>),
}

impl FindNode {
    /// What the expression gives for the entry find prints as `path`, of
    /// file type `kind` (find's letter, `None` when unknown): `Some(true)`,
    /// `Some(false)`, or `None` when the check cannot tell; and whether it
    /// surely prunes the entry. An operand that find may skip (after a true
    /// `-o`) prunes nothing for sure.
    fn evaluate(&self, path: &str, kind: Option<&str>) -> (Option<bool>, bool) {
        let (operator, left, right) = match self {
            Self::Primary(name, argument) => {
                let value = find_primary(name, argument.as_deref().unwrap_or_default(), path, kind);
                return (value, name == "-prune");
            }
            Self::Not(operand) => {
                let (value, prunes) = operand.evaluate(path, kind);
                return (value.map(|value| !value), prunes);
            }
            Self::And(left, right) => ("and", left, right),
            Self::Or(left, right) => ("or", left, right),
            Self::Comma(left, right) => (",", left, right),
        };
        let (value, prunes) = left.evaluate(path, kind);
        // Whether the right operand runs: after a comma always, after -a
        // when the left one is true, after -o when it is false.
        let runs = match operator {
            "," => Some(true),
            "and" => value,
            _ => value.map(|value| !value),
        };
        if runs == Some(false) {
            return (value, prunes);
        }
        let (right_value, right_prunes) = right.evaluate(path, kind);
        let prunes = prunes || (runs == Some(true) && right_prunes);
        if runs == Some(true) {
            return (right_value, prunes);
        }
        // The left operand is unknown: only the right one can decide.
        let decided = operator == "or";
        ((right_value == Some(decided)).then_some(decided), prunes)
    }
}

/// A parser of find's expression.
struct FindParser<'a> {
    words: &'a [String],
    position: usize,
}

impl FindParser<'_> {
    /// find's expression (`words`, after its starting points), or `None`
    /// when the words do not parse.
    fn parse(words: &[String]) -> Option<FindNode> {
        if words.is_empty() {
            return Some(FindNode::Primary("-true".to_string(), None));
        }
        let mut parser = FindParser { words, position: 0 };
        let node = parser.alternatives()?;
        (parser.position == words.len()).then_some(node)
    }

    fn peek(&self) -> Option<&str> {
        self.words.get(self.position).map(String::as_str)
    }

    fn alternatives(&mut self) -> Option<FindNode> {
        let mut node = self.disjunction()?;
        while self.peek() == Some(",") {
            self.position += 1;
            node = FindNode::Comma(Box::new(node), Box::new(self.disjunction()?));
        }
        Some(node)
    }

    fn disjunction(&mut self) -> Option<FindNode> {
        let mut node = self.conjunction()?;
        while matches!(self.peek(), Some("-o" | "-or")) {
            self.position += 1;
            node = FindNode::Or(Box::new(node), Box::new(self.conjunction()?));
        }
        Some(node)
    }

    fn conjunction(&mut self) -> Option<FindNode> {
        let mut node = self.primary()?;
        while let Some(word) = self.peek()
            && !matches!(word, "-o" | "-or" | "," | ")")
        {
            if matches!(word, "-a" | "-and") {
                self.position += 1;
            }
            node = FindNode::And(Box::new(node), Box::new(self.primary()?));
        }
        Some(node)
    }

    fn primary(&mut self) -> Option<FindNode> {
        let word = self.peek()?.to_string();
        self.position += 1;
        if word == "!" || word == "-not" {
            return Some(FindNode::Not(Box::new(self.primary()?)));
        }
        if word == "(" {
            let node = self.alternatives()?;
            if self.peek() != Some(")") {
                return None;
            }
            self.position += 1;
            return Some(node);
        }
        let mut argument = None;
        if FIND_EXEC.contains(&word.as_str()) {
            while self.peek().is_some_and(|word| !matches!(word, ";" | "+")) {
                self.position += 1;
            }
            argument = Some(self.peek().unwrap_or(";").to_string());
            self.position += 1;
        } else if FIND_NAME_TESTS.contains(&word.as_str())
            || FIND_ONE_ARGUMENT.contains(&word.as_str())
            || word.starts_with("-newer")
        {
            argument = Some(self.peek().unwrap_or_default().to_string());
            self.position += 1;
        } else if word == "-fprintf" {
            self.position += 2;
        }
        // Past the end, the position stops at the end, as with no argument.
        self.position = self.position.min(self.words.len());
        Some(FindNode::Primary(word, argument))
    }
}

/// What a find primary gives for the entry printed as `path`: its name,
/// path and type tests as find decides them; `Some(true)` for actions and
/// options; `None` for `-exec … ;` and the tests the check does not model.
/// find's regexes are emacs regexes matched against the whole path, so a
/// miss there proves nothing.
fn find_primary(name: &str, argument: &str, path: &str, kind: Option<&str>) -> Option<bool> {
    match name {
        "-name" | "-iname" | "-path" | "-ipath" | "-wholename" | "-iwholename" => {
            let text = if matches!(name, "-name" | "-iname") {
                path.rsplit('/').next().unwrap_or(path)
            } else {
                path
            };
            Some(if name.starts_with("-i") {
                glob_match(&argument.to_lowercase(), &text.to_lowercase())
            } else {
                glob_match(argument, text)
            })
        }
        "-regex" | "-iregex" => {
            // The pattern compiles alone, so it cannot break out of the anchors.
            let whole = RegexBuilder::new(&format!("^(?:{argument})$"))
                .case_insensitive(name == "-iregex")
                .build();
            (Regex::new(argument).is_ok() && whole.is_ok_and(|regex| regex.is_match(path)))
                .then_some(true)
        }
        "-type" => kind.map(|kind| argument.split(',').any(|letter| letter == kind)),
        "-false" => Some(false),
        _ if FIND_EXEC.contains(&name) => (argument == "+").then_some(true),
        _ => FIND_TRUE.contains(&name).then_some(true),
    }
}

/// How find prints the entry at `rel` under the starting point `start`.
fn printed_path(start: &str, rel: &str) -> String {
    format!("{}/{rel}", start.trim_end_matches('/'))
}

/// find's type letter (`f`, `d` or `l`) for the file at `path`, or `None`.
fn file_kind(path: &Path) -> Option<&'static str> {
    let kind = path.symlink_metadata().ok()?.file_type();
    if kind.is_dir() {
        Some("d")
    } else if kind.is_symlink() {
        Some("l")
    } else {
        kind.is_file().then_some("f")
    }
}

/// A depth limit given as a whole number (`-maxdepth 2`).
fn depth_limit(value: &str) -> Option<usize> {
    if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    value.parse().ok()
}

/// fd's search paths, and what it lists from them: no hidden file without
/// `-H` or `-u`, nothing an `-E` glob leaves out or deeper than `-d`, and
/// only the names its pattern (a regex, a glob with `-g`, a string with
/// `-F`, the whole path with `-p`) and its extensions (`-e`) match, case
/// ignored.
fn fd_list(arguments: &[String]) -> ListSource {
    let (options, operands) = search_arguments(arguments, FD_SHORT_VALUES, FD_LONG_VALUES);
    let has = |names: &[&str]| {
        options
            .iter()
            .any(|(name, _)| names.contains(&name.as_str()))
    };
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
    let pattern = operands.first().cloned().unwrap_or_default();
    let extensions: Vec<String> = options
        .iter()
        .filter(|(name, _)| name == "-e" || name == "--extension")
        .map(|(_, value)| value.to_lowercase().trim_start_matches('.').to_string())
        .collect();
    let hidden = has(&["-H", "--hidden", "-u", "--unrestricted"]);
    let full_path = has(&["-p", "--full-path"]);
    let globbing = has(&["-g", "--glob"]);
    let fixed = has(&["-F", "--fixed-strings"]);
    let reads = move |file: &TreeFile| {
        let parts: Vec<&str> = file.rel.split('/').collect();
        let name = parts.last().copied().unwrap_or_default().to_lowercase();
        if !hidden && parts.iter().any(|part| part.starts_with('.')) {
            return false;
        }
        if !extensions.is_empty()
            && !extensions
                .iter()
                .any(|extension| name.ends_with(&format!(".{extension}")))
        {
            return false;
        }
        for (option, value) in &options {
            if matches!(option.as_str(), "-E" | "--exclude")
                && glob_excludes(value, &file.rel, false)
            {
                return false;
            }
            if matches!(option.as_str(), "-d" | "--max-depth")
                && depth_limit(value).is_some_and(|limit| parts.len() > limit)
            {
                return false;
            }
        }
        if pattern.is_empty() {
            return true;
        }
        let target = if full_path {
            file.path.to_string_lossy().to_lowercase()
        } else {
            name
        };
        if globbing {
            glob_match(&pattern.to_lowercase(), &target)
        } else if fixed {
            target.contains(&pattern.to_lowercase())
        } else {
            RegexBuilder::new(&pattern)
                .case_insensitive(true)
                .build()
                .ok()
                .is_none_or(|regex| regex.is_match(&target))
        }
    };
    ListSource {
        roots,
        reads: Rc::new(reads),
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

/// Refuse a recursive search (`grep -r`, `rg`, `ag`, `ack`) that reads a key
/// file or a roko config file holding a secret ([`search_reads`]).
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
    if !names
        .iter()
        .any(|name| searcher.pattern_options.contains(name))
        && !operands.is_empty()
    {
        operands.remove(0);
    }
    if operands.is_empty() {
        operands.push(".".to_string());
    }
    check_tree_reads(&operands, &walk.dir, &walk.call_dir, &|file| {
        search_reads(&options, searcher.name, &file.rel)
    })
}

/// Whether a recursive search reads the file at `rel` in the tree it
/// searches. grep and ack read hidden files and directories; rg and ag do
/// with `--hidden` or `-uu` (ag `-u`), and rg with a glob that names one.
/// Its filters can leave the file out (see the module doc).
fn search_reads(options: &[(String, String)], kind: &str, rel: &str) -> bool {
    let parts: Vec<&str> = rel.split('/').collect();
    let name = parts.last().copied().unwrap_or_default();
    let dirs = &parts[..parts.len() - 1];
    let mut includes = Vec::new();
    let mut types = Vec::new();
    let mut skipped_types = Vec::new();
    let mut hidden = matches!(kind, "grep" | "ack");
    let mut unrestricted = 0;
    let mut typed = true;
    for (option, value) in options {
        let excluded = match (kind, option.as_str()) {
            ("grep", "--include") => {
                includes.push(value.as_str());
                false
            }
            ("rg", "-g" | "--glob" | "--iglob") => match value.strip_prefix('!') {
                Some(glob) => glob_excludes(glob, rel, option == "--iglob"),
                None => {
                    includes.push(value.as_str());
                    false
                }
            },
            ("ag", "--ignore" | "--ignore-dir") => glob_excludes(value, rel, false),
            ("grep", "--exclude") => glob_match(value, name),
            ("grep", "--exclude-dir") => dirs
                .iter()
                .any(|dir| glob_match(value.trim_end_matches('/'), dir)),
            ("ack", "--ignore-dir") => dirs.iter().any(|dir| ack_filter(value, dir)),
            ("ack", "--ignore-file") => ack_filter(value, name),
            ("ag", "-G" | "--file-search-regex") => {
                Regex::new(value).is_ok_and(|regex| !regex.is_match(rel))
            }
            ("rg" | "ack", "-t" | "--type") => {
                match value.strip_prefix("no") {
                    Some(skipped) => skipped_types.push(skipped),
                    None => types.push(value.as_str()),
                }
                false
            }
            ("rg", "-T" | "--type-not") => {
                skipped_types.push(value.as_str());
                false
            }
            ("rg", "--type-add") => {
                // A type it adds may cover the file.
                typed = false;
                false
            }
            ("rg" | "ag", "--hidden") | ("rg", "-.") => {
                hidden = true;
                false
            }
            ("rg", "--no-hidden") => {
                hidden = false;
                false
            }
            ("rg" | "ag", "-u" | "--unrestricted") => {
                unrestricted += 1;
                false
            }
            // rg counts the files in the root as one level deep, ag as none.
            ("rg", "-d" | "--max-depth" | "--maxdepth") | ("ag", "--depth") => depth_limit(value)
                .is_some_and(|limit| parts.len() - usize::from(kind == "ag") > limit),
            _ => false,
        };
        if excluded {
            return false;
        }
    }
    let hidden = hidden || unrestricted >= if kind == "rg" { 2 } else { 1 };
    let glob_names = |count: usize| {
        let path = parts[..count].join("/");
        kind == "rg" && includes.iter().any(|pattern| glob_includes(pattern, &path))
    };
    if !hidden
        && (1..=parts.len()).any(|count| parts[count - 1].starts_with('.') && !glob_names(count))
    {
        return false;
    }
    if !includes.is_empty() && !includes.iter().any(|pattern| glob_includes(pattern, rel)) {
        return false;
    }
    let file_type = file_type(name);
    if typed
        && !types.is_empty()
        && !types
            .iter()
            .any(|kind| *kind == "all" || Some(*kind) == file_type)
    {
        return false;
    }
    !skipped_types.iter().any(|kind| Some(*kind) == file_type)
}

/// The rg file type (`rg --type-list`) of a key file or roko config file
/// named `name`: `toml`, `json`, or `sh` for a `.env` file.
fn file_type(name: &str) -> Option<&'static str> {
    match name.rsplit_once('.')?.1 {
        "toml" => Some("toml"),
        "json" => Some("json"),
        "env" => Some("sh"),
        _ => None,
    }
}

/// Whether ack's filter (`is:NAME`, `ext:EXT,…`, `match:/REGEX/`, or a bare
/// name) matches the file or directory `name`.
fn ack_filter(value: &str, name: &str) -> bool {
    match value.split_once(':') {
        None => name == value,
        Some(("is", argument)) => name == argument,
        Some(("ext", argument)) => name
            .rsplit_once('.')
            .is_some_and(|(_, extension)| argument.split(',').any(|listed| listed == extension)),
        Some(("match", argument)) => {
            Regex::new(argument.trim_matches('/')).is_ok_and(|regex| regex.is_match(name))
        }
        Some(_) => false,
    }
}

/// Whether a glob that lets files in (grep `--include`, rg `-g`, a git
/// pathspec) may match the file at `rel`, by name or by path. Braces are
/// expanded and case ignored, so that a doubt counts as a match.
fn glob_includes(pattern: &str, rel: &str) -> bool {
    let rel = rel.to_lowercase();
    let name = rel.rsplit('/').next().unwrap_or(&rel);
    let Ok(variants) = expand_braces(&pattern.to_lowercase()) else {
        return true;
    };
    variants.iter().any(|variant| {
        let mut variant = variant.trim_start_matches('/');
        while let Some(rest) = variant.strip_prefix("**/") {
            variant = rest;
        }
        glob_match(variant, name) || glob_match(variant, &rel)
    })
}

/// Whether a glob that leaves files out (rg `-g '!…'`, ag `--ignore`, fd
/// `-E`) surely matches the file at `rel` or a directory it is in, by name
/// or by path from the root; `fold` ignores case (rg `--iglob`).
fn glob_excludes(pattern: &str, rel: &str, fold: bool) -> bool {
    let mut pattern = pattern.trim_start_matches('/').trim_end_matches('/');
    while let Some(rest) = pattern.strip_prefix("**/") {
        pattern = rest;
    }
    let (pattern, rel) = if fold {
        (pattern.to_lowercase(), rel.to_lowercase())
    } else {
        (pattern.to_string(), rel.to_string())
    };
    let parts: Vec<&str> = rel.split('/').collect();
    parts.iter().any(|part| glob_match(&pattern, part))
        || (1..=parts.len()).any(|count| glob_match(&pattern, &parts[..count].join("/")))
}

/// Whether `text` matches the shell glob `pattern`, a `*` matching any run
/// of characters, `/` included.
fn glob_match(pattern: &str, text: &str) -> bool {
    segment_match(pattern.as_bytes(), text.as_bytes())
}

/// git's global options before its subcommand; `git grep` searches. A
/// subcommand that is not one of git's commands is refused, unless git may
/// be an argument there (`timeout 5 grep git src`): an alias can run any
/// command, a read this check refuses among them, and the same command line
/// can define one (`git -c alias.x='!cat roko.toml' x`).
fn check_git(walk: &Walk, arguments: &[String], maybe_argument: bool) -> Result<(), ToolError> {
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
    let Some(subcommand) = arguments.get(index) else {
        return Ok(());
    };
    if subcommand.contains(['$', '`']) {
        return Err(ToolError::CommandNotAllowed(
            "a git subcommand taken from a variable cannot be checked".to_string(),
        ));
    }
    if !is_git_command(subcommand) {
        if maybe_argument {
            return Ok(());
        }
        return Err(ToolError::CommandNotAllowed(format!(
            "git {subcommand} is not one of git's commands, and the check does not follow \
             aliases, which can run a read it refuses"
        )));
    }
    if subcommand == "grep" {
        check_git_grep(&arguments[index + 1..], &directory, &walk.call_dir)?;
    }
    Ok(())
}

/// Whether `git <name>` runs one of git's commands rather than an alias: a
/// command git ships, or a git-* program it finds.
fn is_git_command(name: &str) -> bool {
    static LISTED: OnceLock<Vec<String>> = OnceLock::new();
    GIT_COMMANDS.contains(&name)
        || LISTED
            .get_or_init(listed_git_commands)
            .iter()
            .any(|command| command == name)
}

/// The commands `git --list-cmds=main,others` lists, git's own and the git-*
/// programs on `PATH`; none when git cannot run.
fn listed_git_commands() -> Vec<String> {
    std::process::Command::new("git")
        .arg("--list-cmds=main,others")
        .stdin(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| {
            String::from_utf8_lossy(&output.stdout)
                .split_whitespace()
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

/// `git grep` reads the tracked files under the directory git runs in, or
/// under the paths and pathspecs it names, down to `--max-depth`, and the
/// untracked ones too with `--untracked` or `--no-index`: a key file is
/// never tracked. A revision it names reads that commit's tree of the same
/// directory.
fn check_git_grep(
    arguments: &[String],
    directory: &Path,
    call_dir: &Path,
) -> Result<(), ToolError> {
    let (options, mut operands) = search_arguments(arguments, "efmABC", GIT_GREP_LONG_VALUES);
    if !options.iter().any(|(name, _)| name == "-e" || name == "-f") && !operands.is_empty() {
        operands.remove(0);
    }
    let (globs, others): (Vec<String>, Vec<String>) = operands
        .into_iter()
        .partition(|operand| operand.contains(['*', '?', '[']));
    let paths: Vec<String> = others
        .into_iter()
        .filter(|operand| directory.join(operand).exists())
        .collect();
    let untracked = options
        .iter()
        .any(|(name, _)| name == "--untracked" || name == "--no-index");
    let max_depth = options
        .iter()
        .rev()
        .filter(|(name, _)| name == "--max-depth")
        .find_map(|(_, value)| depth_limit(value));
    let reads = |narrowed: bool| {
        let globs = &globs;
        move |file: &TreeFile| {
            if !untracked && (is_key_file(&file.path) || is_key_file(&resolve_symlinks(&file.path)))
            {
                return false;
            }
            if max_depth.is_some_and(|limit| file.rel.matches('/').count() > limit) {
                return false;
            }
            !narrowed
                || globs
                    .iter()
                    .any(|pattern| glob_includes(pattern, &file.rel))
        }
    };
    if !paths.is_empty() || globs.is_empty() {
        let roots = if paths.is_empty() {
            vec![".".to_string()]
        } else {
            paths
        };
        check_tree_reads(&roots, directory, call_dir, &reads(false))?;
    }
    if !globs.is_empty() {
        check_tree_reads(&[".".to_string()], directory, call_dir, &reads(true))?;
    }
    Ok(())
}

/// Refuse a read of the trees at `roots`, resolved against `directory`,
/// that reaches a file agents must not read ([`sensitive_files`]) in one, as
/// `reads` decides. A brace or a glob in a root is expanded as the shell
/// would. `call_dir` is the directory the command line runs in.
fn check_tree_reads(
    roots: &[String],
    directory: &Path,
    call_dir: &Path,
    reads: &dyn Fn(&TreeFile) -> bool,
) -> Result<(), ToolError> {
    for root in roots {
        for variant in expand_braces(root)? {
            if variant.contains(['$', '`', '{']) {
                continue;
            }
            let word = match variant.strip_prefix('~') {
                Some(rest) if rest.is_empty() || rest.starts_with('/') => word_path("~", directory)
                    .map_or_else(
                        || variant.clone(),
                        |home| format!("{}{rest}", home.display()),
                    ),
                _ => variant.clone(),
            };
            for path in expand(&variant, directory)? {
                if !path.is_dir() {
                    if let Some(config) = [path.clone(), resolve_symlinks(&path)]
                        .into_iter()
                        .find(|candidate| is_config_with_secrets(candidate))
                    {
                        return Err(ToolError::KeyFileBlocked(config));
                    }
                    continue;
                }
                let start = if variant.contains(['*', '?', '[']) && !Path::new(&word).is_absolute()
                {
                    // The shell passes a glob's match as the glob gives it.
                    let rel = path.strip_prefix(directory).unwrap_or(&path).display();
                    if word.starts_with("./") {
                        format!("./{rel}")
                    } else {
                        rel.to_string()
                    }
                } else {
                    word.clone()
                };
                let Ok(top) = path.canonicalize() else {
                    continue;
                };
                for found in sensitive_files(&top, directory, call_dir) {
                    let rel = found
                        .strip_prefix(&top)
                        .unwrap_or(&found)
                        .components()
                        .map(|part| part.as_os_str().to_string_lossy())
                        .collect::<Vec<_>>()
                        .join("/");
                    let file = TreeFile {
                        rel,
                        start: start.clone(),
                        path: found,
                    };
                    if reads(&file) {
                        return Err(ToolError::KeyFileBlocked(file.path));
                    }
                }
            }
        }
    }
    Ok(())
}

/// The files in the tree at `top`, a canonical directory, that agents must
/// not read, of those the check finds without walking the whole tree: the
/// roko config files holding a secret among those roko reads (the
/// `roko.toml` in `top`, the workspace's in `cwd` or above it, the file
/// `ROKO_CONFIG` names and the legacy `~/.config/roko/config.toml`), and the
/// key files in the `.roko` directories of [`key_directories`].
fn sensitive_files(top: &Path, cwd: &Path, call_dir: &Path) -> Vec<PathBuf> {
    let config_home = std::env::var_os("XDG_CONFIG_HOME")
        .filter(|dir| !dir.is_empty())
        .map(PathBuf::from)
        .or_else(|| word_path("~/.config", cwd));
    let configs = [
        Some(top.join("roko.toml")),
        workspace_config(cwd),
        std::env::var_os("ROKO_CONFIG").map(PathBuf::from),
        config_home.map(|dir| dir.join("roko").join("config.toml")),
    ];
    let mut found: Vec<PathBuf> = configs
        .into_iter()
        .flatten()
        .filter_map(|config| {
            let dir = config.parent()?.canonicalize().ok()?;
            Some(dir.join(config.file_name()?))
        })
        .filter(|config| config.starts_with(top) && is_config_with_secrets(config))
        .collect();
    for directory in key_directories(top, cwd, call_dir) {
        let roko = if directory.file_name().is_some_and(|name| name == ".roko") {
            directory
        } else {
            directory.join(".roko")
        };
        if roko.starts_with(top) && roko.is_dir() {
            found.extend(
                KEY_FILE_NAMES
                    .iter()
                    .map(|name| roko.join(name))
                    .filter(|key| key.symlink_metadata().is_ok()),
            );
        }
    }
    found
}

/// The directories whose `.roko` holds key files that a read of the tree at
/// `top` may reach: `top`, its subdirectories two levels down (not under a
/// hidden one, at most [`KEY_SEARCH_DIRS`] a level), `cwd`, `call_dir`,
/// their ancestors, and `HOME`.
fn key_directories(top: &Path, cwd: &Path, call_dir: &Path) -> Vec<PathBuf> {
    let mut directories = vec![top.to_path_buf()];
    let mut level = vec![top.to_path_buf()];
    for _ in 0..2 {
        let mut below = Vec::new();
        for directory in &level {
            if below.len() >= KEY_SEARCH_DIRS {
                break;
            }
            let hidden = directory
                .file_name()
                .is_some_and(|name| name.to_string_lossy().starts_with('.'));
            if directory == top || !hidden {
                let Ok(entries) = std::fs::read_dir(directory) else {
                    continue;
                };
                below.extend(
                    entries
                        .flatten()
                        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_dir()))
                        .map(|entry| entry.path()),
                );
            }
        }
        below.truncate(KEY_SEARCH_DIRS);
        directories.extend(below.iter().cloned());
        level = below;
    }
    for start in [cwd, call_dir] {
        let start = start.canonicalize().unwrap_or_else(|_| start.to_path_buf());
        directories.extend(start.ancestors().map(Path::to_path_buf));
    }
    directories.extend(word_path("~", top).and_then(|home| home.canonicalize().ok()));
    directories
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
///
/// # Errors
///
/// Returns [`ToolError::CommandNotAllowed`] for a glob with more than
/// [`EXPANSION_LIMIT`] matches in a component.
pub(super) fn expand(word: &str, cwd: &Path) -> Result<Vec<PathBuf>, ToolError> {
    if !word.contains(['*', '?', '[']) {
        return Ok(word_path(word, cwd).into_iter().collect());
    }
    if word.contains(['$', '`', '{']) {
        return Ok(Vec::new());
    }
    let pattern = match word.strip_prefix('~') {
        Some(rest) if rest.is_empty() || rest.starts_with('/') => {
            let Some(home) = word_path("~", cwd) else {
                return Ok(Vec::new());
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
fn glob_paths(pattern: &Path) -> Result<Vec<PathBuf>, ToolError> {
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
                if next.len() > EXPANSION_LIMIT {
                    return Err(ToolError::CommandNotAllowed(
                        "a glob in the command matches more files than the check can follow"
                            .to_string(),
                    ));
                }
            }
        }
        found = next;
    }
    found.retain(|path| path.exists());
    Ok(found)
}

/// The words a shell brace expansion makes of `word` (`a{b,c}d` gives `abd`
/// and `acd`), or `word` itself when it has none. A `${…}` is no brace
/// expansion, and neither is a `{}` without a comma in it.
///
/// # Errors
///
/// Returns [`ToolError::CommandNotAllowed`] for an expansion that makes more
/// than [`EXPANSION_LIMIT`] words.
pub(super) fn expand_braces(word: &str) -> Result<Vec<String>, ToolError> {
    let mut words = Vec::new();
    expand_braces_into(word, &mut words)?;
    Ok(words)
}

fn expand_braces_into(word: &str, words: &mut Vec<String>) -> Result<(), ToolError> {
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
                        expand_braces_into(&variant, words)?;
                        last = comma + 1;
                    }
                    return Ok(());
                }
            }
            b',' if depth == 1 => commas.push(index),
            _ => {}
        }
    }
    words.push(word.to_string());
    if words.len() > EXPANSION_LIMIT {
        return Err(ToolError::CommandNotAllowed(
            "a brace expansion in the command makes more words than the check can follow"
                .to_string(),
        ));
    }
    Ok(())
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
