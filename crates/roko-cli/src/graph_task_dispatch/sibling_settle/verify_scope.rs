//! What a verify step reads of its working tree (gap-1920ba).
//!
//! A step that reads the whole project, such as `tsc --noEmit` or
//! `cargo check`, must not run while a sibling task that shares the tree is
//! mid-edit, or it checks a half-written file. A step that reads less waits
//! only for siblings that write where it reads. A step's scope is its
//! declared `scope`; otherwise it is inferred from its command,
//! conservatively: a command whose reads are unknown reads the whole
//! project.

use std::path::{Component, Path, PathBuf};

use crate::task_parser::VerifyStep;

/// What a verify step reads of its working tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum StepScope {
    /// Everything: a whole-project check, or a command whose reads are
    /// unknown.
    Whole,
    /// These paths, relative to the working tree. A directory covers what it
    /// holds, and an empty path is the whole tree.
    Paths(Vec<PathBuf>),
}

impl StepScope {
    /// What `step` reads when it runs in `workdir`: its declared `scope`,
    /// else what its command reads.
    pub(crate) fn of(step: &VerifyStep, workdir: &Path) -> Self {
        if step.scope.is_empty() {
            infer(&step.command, workdir)
        } else {
            declared(&step.scope)
        }
    }

    /// Whether a task that writes `files` may change what the step reads. A
    /// task that declares no files writes nothing the scheduler knows of.
    pub(crate) fn covers(&self, files: &[String]) -> bool {
        match self {
            Self::Whole => !files.is_empty(),
            Self::Paths(paths) => files
                .iter()
                .filter_map(|file| in_tree(Path::new(""), Path::new(file.trim())))
                .filter(|file| !file.as_os_str().is_empty())
                .any(|file| paths.iter().any(|path| overlaps(path, &file))),
        }
    }
}

/// Whether two paths in the working tree name the same file, or one names a
/// directory that holds the other.
fn overlaps(left: &Path, right: &Path) -> bool {
    left.starts_with(right) || right.starts_with(left)
}

/// A declared scope. A path that names the whole tree, an absolute path or
/// one outside the tree reads the whole project.
fn declared(scope: &[String]) -> StepScope {
    let mut paths = Vec::new();
    for path in scope {
        match in_tree(Path::new(""), Path::new(path.trim())) {
            Some(path) if !path.as_os_str().is_empty() => paths.push(path),
            _ => return StepScope::Whole,
        }
    }
    StepScope::Paths(paths)
}

/// Programs that read nothing a sibling writes.
const READS_NOTHING: &[&str] = &[
    "echo", "printf", "true", "false", ":", "exit", "touch", "mkdir", "sleep",
];

/// Programs that read the paths among their operands and nothing else.
const READS_OPERANDS: &[&str] = &[
    "cat", "head", "tail", "wc", "diff", "cmp", "ls", "stat", "file", "sort", "uniq",
];

/// Program prefixes that run the rest of the command as is.
const TRANSPARENT: &[&str] = &["!", "command", "exec", "time", "env", "nice"];

/// What `command` reads when it runs in `workdir`, inferred conservatively.
///
/// It follows `cd`, `cargo` limited to packages (`-p X` reads `crates/X`
/// when that directory exists), and read-only file tools such as `test`,
/// `grep` and `cat` with explicit paths. Any other program reads the whole
/// project, or the directory the command changed into, and so does any
/// shell syntax this does not follow: subshells, substitutions and
/// variables.
fn infer(command: &str, workdir: &Path) -> StepScope {
    let Some(commands) = simple_commands(command) else {
        return StepScope::Whole;
    };
    let mut base = PathBuf::new();
    let mut paths = Vec::new();
    for words in commands {
        let Some(words) = strip_redirects(&words, &base, workdir, &mut paths) else {
            return StepScope::Whole;
        };
        let words = skip_prefixes(&words);
        let Some((program, args)) = words.split_first() else {
            continue;
        };
        let program = Path::new(program)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default();
        let read = match program {
            "cd" => match args {
                [dir] if !dir.starts_with('-') => match place(&base, Path::new(dir), workdir) {
                    Some(Place::Tree(dir)) => {
                        base = dir;
                        continue;
                    }
                    _ => return StepScope::Whole,
                },
                _ => return StepScope::Whole,
            },
            "cargo" => cargo_reads(args, &base, workdir),
            "test" | "[" | "[[" => Some(test_operands(args)),
            "grep" | "egrep" | "fgrep" | "rg" => grep_operands(program, args),
            program if READS_OPERANDS.contains(&program) => Some(
                args.iter()
                    .filter(|arg| !arg.starts_with('-'))
                    .cloned()
                    .collect(),
            ),
            program if READS_NOTHING.contains(&program) => Some(Vec::new()),
            // A whole-project tool run in a subdirectory reads that
            // subdirectory; in the tree's root, it reads everything.
            _ => (!base.as_os_str().is_empty()).then(|| vec![".".to_string()]),
        };
        let Some(read) = read else {
            return StepScope::Whole;
        };
        for operand in read {
            match place(&base, &glob_free(&operand), workdir) {
                Some(Place::Tree(path)) => paths.push(path),
                Some(Place::Outside) => {}
                None => return StepScope::Whole,
            }
        }
    }
    StepScope::Paths(paths)
}

/// What a `cargo` command reads: the directories of the packages it names
/// with `-p`/`--package`, or the directory it runs in. `None` for the whole
/// project: `--workspace`, no package in the tree's root, or a package
/// whose directory is not `crates/<name>`.
fn cargo_reads(args: &[String], base: &Path, workdir: &Path) -> Option<Vec<String>> {
    let mut packages = Vec::new();
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--workspace" | "--all" => return None,
            "-p" | "--package" => packages.push(args.next()?.clone()),
            other => {
                if let Some(package) = other
                    .strip_prefix("--package=")
                    .or_else(|| other.strip_prefix("-p").filter(|rest| !rest.is_empty()))
                {
                    packages.push(package.to_string());
                }
            }
        }
    }
    if packages.is_empty() {
        return (!base.as_os_str().is_empty()).then(|| vec![".".to_string()]);
    }
    packages
        .iter()
        .map(|package| {
            let dir = Path::new("crates").join(package);
            workdir
                .join(base)
                .join(&dir)
                .is_dir()
                .then(|| dir.display().to_string())
        })
        .collect()
}

/// The paths a `test` or `[` expression checks: the operands of its file
/// tests.
fn test_operands(args: &[String]) -> Vec<String> {
    const FILE_TESTS: &[&str] = &[
        "-e", "-f", "-d", "-s", "-r", "-w", "-x", "-L", "-h", "-S", "-p", "-b", "-c",
    ];
    args.windows(2)
        .filter(|pair| FILE_TESTS.contains(&pair[0].as_str()))
        .map(|pair| pair[1].clone())
        .collect()
}

/// The paths a `grep` or `rg` reads: its operands after the pattern, and a
/// pattern file. `None` when it searches the current directory recursively
/// and that is the tree's root.
fn grep_operands(program: &str, args: &[String]) -> Option<Vec<String>> {
    // Options that take the next word as their value.
    const WITH_VALUE: &[&str] = &[
        "-m",
        "-A",
        "-B",
        "-C",
        "-g",
        "-t",
        "-T",
        "--max-count",
        "--after-context",
        "--before-context",
        "--context",
        "--glob",
        "--type",
        "--type-not",
    ];
    let mut recursive = program == "rg";
    let mut pattern_given = false;
    let mut operands = Vec::new();
    let mut reads = Vec::new();
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--" => operands.extend(args.by_ref().cloned()),
            "-e" | "--regexp" => {
                pattern_given = true;
                args.next();
            }
            "-f" | "--file" => {
                pattern_given = true;
                reads.extend(args.next().cloned());
            }
            "-r" | "-R" | "--recursive" => recursive = true,
            option if WITH_VALUE.contains(&option) => {
                args.next();
            }
            option if option.starts_with("--regexp=") || option.starts_with("--file=") => {
                pattern_given = true;
            }
            option if option.starts_with("--") => {}
            option if option.starts_with('-') && option.len() > 1 => {
                recursive |= option.contains(['r', 'R']);
            }
            operand => operands.push(operand.to_string()),
        }
    }
    let paths = if pattern_given {
        operands
    } else {
        operands.into_iter().skip(1).collect()
    };
    if paths.is_empty() && recursive {
        return None;
    }
    reads.extend(paths);
    Some(reads)
}

/// `words` without prefixes such as `!`, `env`, `time` or leading variable
/// assignments.
fn skip_prefixes(words: &[String]) -> &[String] {
    let mut rest = words;
    while let Some((first, tail)) = rest.split_first() {
        let assignment = first.split_once('=').is_some_and(|(name, _)| {
            name.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_')
                && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        });
        if TRANSPARENT.contains(&first.as_str()) || assignment {
            rest = tail;
        } else {
            break;
        }
    }
    rest
}

/// `words` without redirections. A file in the tree read through `<` joins
/// `reads`; an output target is dropped. `None` for a here-document or an
/// input that climbs out of the tree.
fn strip_redirects(
    words: &[String],
    base: &Path,
    workdir: &Path,
    reads: &mut Vec<PathBuf>,
) -> Option<Vec<String>> {
    let mut kept = Vec::new();
    let mut words = words.iter();
    while let Some(word) = words.next() {
        let operator = word.trim_start_matches(|c: char| c.is_ascii_digit() || c == '&');
        if operator.starts_with("<<") {
            return None;
        }
        if let Some(target) = operator.strip_prefix('<') {
            let target = if target.is_empty() {
                words.next()?.clone()
            } else {
                target.to_string()
            };
            match place(base, Path::new(&target), workdir)? {
                Place::Tree(path) => reads.push(path),
                Place::Outside => {}
            }
        } else if let Some(target) = operator.strip_prefix('>') {
            let target = target.trim_start_matches(['>', '&', '|']);
            if target.is_empty() && !operator.contains('&') {
                words.next();
            }
        } else {
            kept.push(word.clone());
        }
    }
    Some(kept)
}

/// `path` without its first component that holds a glob character and
/// everything after it: the directory the glob searches.
fn glob_free(path: &str) -> PathBuf {
    Path::new(path)
        .components()
        .take_while(|component| {
            !component
                .as_os_str()
                .to_string_lossy()
                .contains(['*', '?', '['])
        })
        .collect()
}

/// Where a path a command names points.
enum Place {
    /// A place in the working tree, relative to its root.
    Tree(PathBuf),
    /// Outside the working tree, where no sibling writes.
    Outside,
}

/// Where `path` points when the command runs in `base` (relative to the
/// working tree `workdir`). An absolute path inside `workdir` is in the
/// tree; any other absolute path, or a home path, is outside it. `None` for
/// a relative path that climbs out of the tree.
fn place(base: &Path, path: &Path, workdir: &Path) -> Option<Place> {
    if path.has_root() {
        return Some(match path.strip_prefix(workdir) {
            Ok(inside) => Place::Tree(in_tree(Path::new(""), inside)?),
            Err(_) => Place::Outside,
        });
    }
    if path.to_string_lossy().starts_with('~') {
        return Some(Place::Outside);
    }
    in_tree(base, path).map(Place::Tree)
}

/// `path` relative to the working tree's root, when it names a place in the
/// tree: resolved against `base`, the directory the command changed into,
/// without `.` components and with each `..` removing the component before
/// it. `None` for an absolute path, a home path, or one that climbs out of
/// the tree.
fn in_tree(base: &Path, path: &Path) -> Option<PathBuf> {
    if path.has_root() || path.to_string_lossy().starts_with('~') {
        return None;
    }
    let mut resolved = PathBuf::new();
    for component in base.join(path).components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                if !resolved.pop() {
                    return None;
                }
            }
            Component::Normal(part) => resolved.push(part),
            Component::RootDir | Component::Prefix(_) => return None,
        }
    }
    Some(resolved)
}

/// `command` split into simple commands of words, with quotes removed.
/// `&&`, `||`, `;`, `|`, `&` and newlines separate commands, and so do the
/// words `{` and `}`. `None` for syntax this does not follow: a subshell,
/// a command or process substitution, a variable, brace expansion, or an
/// unterminated quote.
fn simple_commands(command: &str) -> Option<Vec<Vec<String>>> {
    let mut commands: Vec<Vec<String>> = vec![Vec::new()];
    let mut word = String::new();
    let mut in_word = false;
    let mut chars = command.chars().peekable();
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
                        '$' | '`' => return None,
                        '\\' => word.push(chars.next()?),
                        c => word.push(c),
                    }
                }
            }
            '\\' => match chars.next()? {
                '\n' => {}
                escaped => {
                    in_word = true;
                    word.push(escaped);
                }
            },
            '$' | '`' | '(' | ')' => return None,
            '#' if !in_word => while chars.next_if(|&c| c != '\n').is_some() {},
            '>' | '<' => {
                // A redirection operator stays in its word: `2>&1` is one.
                in_word = true;
                word.push(c);
                while let Some(next) = chars.next_if(|&next| matches!(next, '>' | '<' | '&' | '-'))
                {
                    word.push(next);
                }
            }
            ';' | '&' | '|' | '\n' => {
                finish_word(&mut commands, &mut word, &mut in_word)?;
                if commands.last().is_some_and(|last| !last.is_empty()) {
                    commands.push(Vec::new());
                }
            }
            c if c.is_whitespace() => finish_word(&mut commands, &mut word, &mut in_word)?,
            c => {
                in_word = true;
                word.push(c);
            }
        }
    }
    finish_word(&mut commands, &mut word, &mut in_word)?;
    commands.retain(|words| !words.is_empty());
    Some(commands)
}

/// End the current word. A `{` or `}` word separates commands; brace
/// expansion inside a word is not followed.
fn finish_word(
    commands: &mut Vec<Vec<String>>,
    word: &mut String,
    in_word: &mut bool,
) -> Option<()> {
    if !*in_word {
        return Some(());
    }
    *in_word = false;
    let finished = std::mem::take(word);
    match finished.as_str() {
        "{" | "}" => {
            if commands.last().is_some_and(|last| !last.is_empty()) {
                commands.push(Vec::new());
            }
        }
        text if text.contains(['{', '}']) => return None,
        _ => commands.last_mut()?.push(finished),
    }
    Some(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scope(command: &str) -> StepScope {
        infer(command, Path::new("/nonexistent-workdir"))
    }

    fn paths(paths: &[&str]) -> StepScope {
        StepScope::Paths(paths.iter().map(PathBuf::from).collect())
    }

    #[test]
    fn whole_project_checks_read_everything() {
        for command in [
            "cargo check",
            "cargo clippy --workspace -- -D warnings",
            "npx tsc --noEmit",
            "npm test",
            "make",
            "bash -c 'cargo test'",
        ] {
            assert_eq!(scope(command), StepScope::Whole, "{command}");
        }
    }

    #[test]
    fn file_tools_read_their_paths() {
        assert_eq!(scope("test -f T5.done"), paths(&["T5.done"]));
        assert_eq!(
            scope("grep -q 'fn add' src/add.rs && [ -d docs ]"),
            paths(&["src/add.rs", "docs"])
        );
        assert_eq!(
            scope("grep -rn -e pattern -f pats.txt src/ lib"),
            paths(&["pats.txt", "src", "lib"])
        );
        assert_eq!(scope("wc -l < notes.md"), paths(&["notes.md"]));
        assert_eq!(scope("grep -q x src/*.rs"), paths(&["src"]));
        assert_eq!(scope("echo ok > /tmp/out.log && true"), paths(&[]));
    }

    #[test]
    fn a_tool_in_a_subdirectory_reads_that_subdirectory() {
        // The 08b verify step: the whole-project `tsc` reads apps/portal.
        assert_eq!(
            scope(
                "cd apps/portal && { [ -d node_modules ] || npm ci --silent; } && \
                 ./node_modules/.bin/tsc --noEmit 2>&1"
            ),
            paths(&["apps/portal/node_modules", "apps/portal", "apps/portal"])
        );
        assert_eq!(
            scope("cd web && npm test && cd .. && cargo test"),
            StepScope::Whole
        );
    }

    #[test]
    fn cargo_reads_the_packages_it_names_when_their_directories_exist() {
        let workdir = tempfile::tempdir().expect("workdir");
        std::fs::create_dir_all(workdir.path().join("crates/roko-graph")).expect("crate");
        let scope = |command: &str| infer(command, workdir.path());
        assert_eq!(
            scope("cargo test -p roko-graph --lib ready"),
            paths(&["crates/roko-graph"])
        );
        assert_eq!(
            scope("cargo check --package=roko-graph"),
            paths(&["crates/roko-graph"])
        );
        assert_eq!(scope("cargo test -p elsewhere"), StepScope::Whole);
    }

    #[test]
    fn absolute_paths_inside_the_working_tree_are_in_it() {
        let scope = |command: &str| infer(command, Path::new("/repo"));
        assert_eq!(scope("test -f /repo/src/lib.rs"), paths(&["src/lib.rs"]));
        assert_eq!(scope("cat /tmp/x.log < /dev/null"), paths(&[]));
        assert_eq!(scope("cd /repo/web && npm test"), paths(&["web"]));
    }

    #[test]
    fn syntax_it_does_not_follow_reads_everything() {
        for command in [
            "test -f \"$TARGET\"",
            "(cd web && npm test) && grep -q x README.md",
            "cat $(ls)",
            "cat <<EOF\nx\nEOF",
            "cat file{1,2}.txt",
            "cd /tmp && cat x",
            "cat ../outside.txt",
            "grep -rq pattern",
        ] {
            assert_eq!(scope(command), StepScope::Whole, "{command}");
        }
    }

    #[test]
    fn a_declared_scope_wins_unless_it_names_the_whole_tree() {
        let step = |scope: &[&str]| VerifyStep {
            phase: "compile".to_string(),
            command: "cargo check".to_string(),
            fail_msg: None,
            timeout_ms: 1_000,
            scope: scope.iter().map(ToString::to_string).collect(),
        };
        let workdir = Path::new("/repo");
        assert_eq!(StepScope::of(&step(&[]), workdir), StepScope::Whole);
        assert_eq!(
            StepScope::of(&step(&["crates/a/", "./docs"]), workdir),
            paths(&["crates/a", "docs"])
        );
        assert_eq!(StepScope::of(&step(&["."]), workdir), StepScope::Whole);
        assert_eq!(StepScope::of(&step(&["/abs"]), workdir), StepScope::Whole);
    }

    #[test]
    fn a_scope_covers_the_writers_inside_it() {
        let files = |paths: &[&str]| paths.iter().map(ToString::to_string).collect::<Vec<_>>();
        let web = paths(&["apps/portal"]);
        assert!(web.covers(&files(&["apps/portal/src/PlanView.tsx"])));
        assert!(web.covers(&files(&["apps"])));
        assert!(!web.covers(&files(&["apps/portal-legacy/x.ts", "crates/a/src/lib.rs"])));
        assert!(!web.covers(&[]));
        assert!(StepScope::Whole.covers(&files(&["anything.rs"])));
        assert!(!StepScope::Whole.covers(&[]));
        assert!(paths(&[""]).covers(&files(&["anything.rs"])));
    }
}
