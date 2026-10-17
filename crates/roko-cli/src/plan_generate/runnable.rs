//! Verify steps that cannot pass as written (gap-ecafde): a runner this
//! machine does not have, a test that the file it names does not define, and
//! a test file that neither exists nor is written by the step's task or a
//! task before it. A plan can keep every check its source states and still
//! fail every attempt on such a step.
//!
//! Generation repairs what it can: a pytest step on a machine without pytest
//! runs the same unittest tests with `python3 -m unittest`, and a test file a
//! task's own check needs joins that task's `files`. The rest is listed for
//! the planner, and a source check whose only covering steps cannot run is
//! not kept.

use std::collections::HashMap;
use std::path::Path;
use std::sync::{LazyLock, Mutex, PoisonError};

use regex::Regex;

use super::source_checks::shell_segments;
use crate::task_parser::{TaskDef, TasksFile};

/// Programs a step may use that need no check: shell builtins and the
/// commands every system has.
const BUILTINS: &[&str] = &[
    "!", ".", ":", "[", "[[", "cd", "echo", "exit", "export", "false", "printf", "set", "source",
    "test", "true",
];

/// File extensions of the test files a check can name.
const TEST_EXTENSIONS: &[&str] = &["go", "js", "jsx", "mjs", "py", "rs", "ts", "tsx"];

/// A verify step that cannot pass as written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Unrunnable {
    /// Its task's id.
    pub task: String,
    /// Its number among its task's verify steps, from 1.
    pub step: usize,
    /// Why it cannot pass.
    pub why: String,
}

/// What this machine can run, asked once per name.
#[derive(Debug, Default)]
pub(super) struct Probe {
    commands: Mutex<HashMap<String, bool>>,
    modules: Mutex<HashMap<String, bool>>,
    /// Answer only from the names given, never asking the machine (tests).
    given_only: bool,
}

impl Probe {
    /// A probe that answers from `commands` and `modules` (`name`, whether
    /// present) alone, for tests: any other name is absent.
    #[cfg(test)]
    pub(super) fn with(commands: &[(&str, bool)], modules: &[(&str, bool)]) -> Self {
        let own = |pairs: &[(&str, bool)]| {
            pairs
                .iter()
                .map(|(name, present)| ((*name).to_string(), *present))
                .collect()
        };
        Self {
            commands: Mutex::new(own(commands)),
            modules: Mutex::new(own(modules)),
            given_only: true,
        }
    }

    /// Whether `name` is an executable on `PATH`.
    fn command(&self, name: &str) -> bool {
        *self
            .commands
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .entry(name.to_string())
            .or_insert_with(|| !self.given_only && crate::config::command_on_path(name))
    }

    /// Whether `python3` can import the top-level package of `module`.
    fn python_module(&self, module: &str) -> bool {
        let top = module.split('.').next().unwrap_or(module).to_string();
        *self
            .modules
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .entry(top.clone())
            .or_insert_with(|| {
                !self.given_only
                    && std::process::Command::new("python3")
                        .args([
                            "-c",
                            "import importlib.util, sys; \
                         sys.exit(0 if importlib.util.find_spec(sys.argv[1]) else 1)",
                            &top,
                        ])
                        .stdin(std::process::Stdio::null())
                        .stdout(std::process::Stdio::null())
                        .stderr(std::process::Stdio::null())
                        .status()
                        .is_ok_and(|status| status.success())
            })
    }

    /// Whether pytest runs here, as a command or as `python3 -m pytest`.
    fn pytest(&self) -> bool {
        self.command("pytest") || (self.command("python3") && self.python_module("pytest"))
    }
}

/// The planner prompt's note on the test runners this machine has, for the
/// kinds of project `workdir` holds; `None` when it holds none of them.
pub(super) fn runners_block(workdir: &Path, probe: &Probe) -> Option<String> {
    let mut lines = Vec::new();
    let python = workdir.join("tests").is_dir()
        || workdir.join("pyproject.toml").is_file()
        || workdir.join("setup.py").is_file();
    if python {
        lines.push(if probe.pytest() {
            "- Python: `pytest` and `python3 -m unittest` both run here.".to_string()
        } else {
            "- Python: `pytest` is NOT installed here. Run Python tests with \
             `python3 -m unittest <package.module.Class.test>` (unittest ids are dotted \
             paths), never `pytest`."
                .to_string()
        });
    }
    if workdir.join("Cargo.toml").is_file() {
        lines.push("- Rust: `cargo test -p <crate> <test name>`.".to_string());
    }
    if workdir.join("package.json").is_file() && !probe.command("npm") {
        lines.push("- JavaScript: `npm` is NOT installed here.".to_string());
    }
    if workdir.join("go.mod").is_file() && !probe.command("go") {
        lines.push("- Go: `go` is NOT installed here.".to_string());
    }
    (!lines.is_empty()).then(|| {
        format!(
            "## Test runners\n\n\
             A check runs on this machine, so it uses a runner this machine has. A check that \
             names a test file runs a test that file defines, or one its task writes into it; a \
             test file the task writes is in that task's `files`.\n{}\n",
            lines.join("\n")
        )
    })
}

/// Repair what keeps `plan`'s verify steps from running and roko can fix
/// itself (gap-ecafde): a pytest step on a machine without pytest, whose
/// test files are unittest files, runs them with `python3 -m unittest`; and a
/// test file a task's own check needs, which neither exists nor is written
/// by a task before it, joins the task's `files`. Returns one line per
/// repair, for the log.
pub(super) fn repair(plan: &mut TasksFile, workdir: &Path, probe: &Probe) -> Vec<String> {
    let mut repairs = Vec::new();
    for index in 0..plan.tasks.len() {
        let written = written_before_and_by(plan, index);
        let task = &mut plan.tasks[index];
        for step in &mut task.verify {
            if let Some(command) = pytest_as_unittest(&step.command, workdir, probe) {
                repairs.push(format!(
                    "{}: `{}` runs as `{command}`: pytest is not installed",
                    task.id, step.command
                ));
                step.command = command;
            }
        }
        let mut needed: Vec<String> = Vec::new();
        for step in &task.verify {
            for path in step_files(&step.command, workdir) {
                if is_test_path(&path)
                    && !workdir.join(&path).exists()
                    && !is_written(&written, &path)
                    && !needed.contains(&path)
                {
                    needed.push(path);
                }
            }
        }
        for path in needed {
            repairs.push(format!(
                "{}: added `{path}` to its files, since its check runs it",
                task.id
            ));
            task.files.push(path);
        }
    }
    repairs
}

/// The verify steps of `plan` that cannot pass as written, in plan order.
pub(super) fn problems(plan: &TasksFile, workdir: &Path, probe: &Probe) -> Vec<Unrunnable> {
    let mut found = Vec::new();
    for (index, task) in plan.tasks.iter().enumerate() {
        let written = written_before_and_by(plan, index);
        for (number, step) in (1..).zip(&task.verify) {
            if let Some(why) = step_problem(&step.command, workdir, &written, probe) {
                found.push(Unrunnable {
                    task: task.id.clone(),
                    step: number,
                    why,
                });
            }
        }
    }
    found
}

/// Why `command` cannot pass, if it cannot: the first problem found.
fn step_problem(
    command: &str,
    workdir: &Path,
    written: &[String],
    probe: &Probe,
) -> Option<String> {
    for segment in shell_segments(command) {
        let words = command_words(&segment);
        let Some((program, args)) = words.split_first() else {
            continue;
        };
        let program = program.as_str();
        if BUILTINS.contains(&program) || program.contains('/') {
            continue;
        }
        if !probe.command(program) {
            return Some(format!("`{program}` is not installed"));
        }
        let module = (matches!(program, "python" | "python3")
            && args.first().is_some_and(|arg| arg == "-m"))
        .then(|| args.get(1))
        .flatten();
        if let Some(module) = module
            && !probe.python_module(module)
        {
            return Some(format!("Python module `{module}` is not installed"));
        }
    }
    for path in step_files(command, workdir) {
        if !workdir.join(&path).exists() && !is_written(written, &path) {
            return Some(format!(
                "it needs `{path}`, which does not exist and no task writes before it runs"
            ));
        }
    }
    for (path, names) in named_tests(command, workdir) {
        if is_written(written, &path) {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(workdir.join(&path)) else {
            continue;
        };
        if let Some(name) = names.iter().find(|name| !defines(&text, name)) {
            return Some(format!(
                "it names `{name}`, which `{path}` does not define and no task writes"
            ));
        }
    }
    None
}

/// A command segment's words, unquoted, without leading `VAR=value`
/// assignments and without redirections (`> out`, `2>/dev/null`, `<in`).
fn command_words(segment: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut word = String::new();
    let mut started = false;
    let mut quote: Option<char> = None;
    for c in segment.chars() {
        match quote {
            Some(open) if c == open => quote = None,
            Some(_) => word.push(c),
            None if c == '\'' || c == '"' => {
                quote = Some(c);
                started = true;
            }
            None if c.is_whitespace() => {
                if std::mem::take(&mut started) {
                    words.push(std::mem::take(&mut word));
                }
            }
            None => {
                word.push(c);
                started = true;
            }
        }
    }
    if started {
        words.push(word);
    }
    let mut kept = Vec::new();
    let mut target = false;
    for word in words {
        if std::mem::take(&mut target) {
            continue;
        }
        let bare = word.trim_start_matches(|c: char| c.is_ascii_digit());
        if bare.starts_with(['>', '<']) {
            // A bare `>` takes the next word as its file.
            target = bare.trim_start_matches(['>', '<', '&']).is_empty();
            continue;
        }
        kept.push(word);
    }
    kept.into_iter()
        .skip_while(|word| word.contains('=') && !word.starts_with('-'))
        .collect()
}

/// The files of `plan.tasks[index]` and of every task it depends on,
/// directly or not: what exists by the time the task runs.
fn written_before_and_by(plan: &TasksFile, index: usize) -> Vec<String> {
    let by_id: HashMap<&str, &TaskDef> = plan
        .tasks
        .iter()
        .map(|task| (task.id.as_str(), task))
        .collect();
    let mut written = plan.tasks[index].files.clone();
    let mut pending: Vec<&str> = plan.tasks[index]
        .depends_on
        .iter()
        .map(String::as_str)
        .collect();
    let mut seen: Vec<&str> = Vec::new();
    while let Some(id) = pending.pop() {
        if seen.contains(&id) {
            continue;
        }
        seen.push(id);
        if let Some(task) = by_id.get(id) {
            written.extend(task.files.iter().cloned());
            pending.extend(task.depends_on.iter().map(String::as_str));
        }
    }
    written
}

/// Whether `path` is one of `written`, or under a directory among them.
fn is_written(written: &[String], path: &str) -> bool {
    written.iter().any(|file| {
        let file = file.trim_end_matches('/');
        path == file
            || path
                .strip_prefix(file)
                .is_some_and(|rest| rest.starts_with('/'))
    })
}

/// Whether `path` is a test file: a source file under a `tests` or `test`
/// directory, or one whose name starts or ends with `test`.
fn is_test_path(path: &str) -> bool {
    let file = Path::new(path);
    let source = file
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| TEST_EXTENSIONS.contains(&extension));
    let stem = file
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or_default();
    source
        && (path
            .split('/')
            .any(|part| part == "tests" || part == "test")
            || stem.starts_with("test")
            || stem.ends_with("test")
            || stem.ends_with("_tests"))
}

/// The workspace files a command reads that must exist for it to pass: the
/// files `grep`, `test -f`, `cat` and pytest are given, the modules
/// `python3 -m unittest` runs, and the integration tests `cargo test --test`
/// runs.
fn step_files(command: &str, workdir: &Path) -> Vec<String> {
    let mut files = Vec::new();
    for segment in shell_segments(command) {
        let words = command_words(&segment);
        let Some((program, args)) = words.split_first() else {
            continue;
        };
        match program.as_str() {
            "grep" | "egrep" | "fgrep" | "rg" => {
                // The first operand is the pattern, unless `-e` gave one.
                let given = args.iter().any(|arg| arg == "-e" || arg.starts_with("-e"));
                let operands = args.iter().filter(|arg| !arg.starts_with('-'));
                files.extend(
                    operands
                        .skip(usize::from(!given))
                        .filter_map(|arg| file_arg(arg)),
                );
            }
            "test" | "[" | "cat" | "head" | "tail" | "wc" => {
                files.extend(args.iter().filter_map(|arg| file_arg(arg)));
            }
            "pytest" | "py.test" => files.extend(pytest_targets(args).map(|(path, _)| path)),
            "python" | "python3" if args.first().is_some_and(|arg| arg == "-m") => {
                match args.get(1).map(String::as_str) {
                    Some("pytest") => {
                        files.extend(pytest_targets(&args[2..]).map(|(path, _)| path))
                    }
                    Some("unittest") => files.extend(
                        unittest_targets(&args[2..], workdir)
                            .into_iter()
                            .map(|(path, _)| path),
                    ),
                    _ => {}
                }
            }
            "cargo" => files.extend(cargo_test_file(args, workdir)),
            _ => {}
        }
    }
    files.sort();
    files.dedup();
    files
}

/// The tests a command names inside files: `path::Class::test` for pytest,
/// `module.Class.test` for unittest, by file.
fn named_tests(command: &str, workdir: &Path) -> Vec<(String, Vec<String>)> {
    let mut named = Vec::new();
    for segment in shell_segments(command) {
        let words = command_words(&segment);
        let Some((program, args)) = words.split_first() else {
            continue;
        };
        match program.as_str() {
            "pytest" | "py.test" => named.extend(pytest_targets(args)),
            "python" | "python3" if args.first().is_some_and(|arg| arg == "-m") => {
                match args.get(1).map(String::as_str) {
                    Some("pytest") => named.extend(pytest_targets(&args[2..])),
                    Some("unittest") => named.extend(unittest_targets(&args[2..], workdir)),
                    _ => {}
                }
            }
            _ => {}
        }
    }
    named.retain(|(_, names)| !names.is_empty());
    named
}

/// A command argument that names a workspace file: a relative path with a
/// file extension, outside glob or option syntax.
fn file_arg(arg: &str) -> Option<String> {
    let plain = !arg.starts_with('-')
        && !arg.starts_with('/')
        && !arg.contains(['*', '?', '$', '{', '<', '>'])
        && Path::new(arg).extension().is_some();
    plain.then(|| arg.trim_start_matches("./").to_string())
}

/// pytest's node ids (`path.py::Class::test`) among `args`, as the file and
/// the names after it; an option's value and a `-k` expression are skipped.
fn pytest_targets(args: &[String]) -> impl Iterator<Item = (String, Vec<String>)> + '_ {
    let mut skip = false;
    args.iter().filter_map(move |arg| {
        if std::mem::take(&mut skip) {
            return None;
        }
        if matches!(arg.as_str(), "-k" | "-m" | "-p" | "-c" | "--rootdir" | "-o") {
            skip = true;
            return None;
        }
        if arg.starts_with('-') {
            return None;
        }
        let mut parts = arg.split("::");
        let path = file_arg(parts.next()?)?;
        let names = parts
            .map(|name| name.split('[').next().unwrap_or(name).to_string())
            .filter(|name| !name.is_empty() && !name.contains('*'))
            .collect();
        Some((path, names))
    })
}

/// unittest's dotted ids among `args`, as the module's file and the names
/// after it: the longest leading part of the id that is a `.py` file in the
/// workspace. An id no file resolves stays whole, as the file it would be.
fn unittest_targets(args: &[String], workdir: &Path) -> Vec<(String, Vec<String>)> {
    if args.first().is_some_and(|arg| arg == "discover") {
        return Vec::new();
    }
    // An option's value (`-k pattern`) is not an id.
    let mut value = false;
    args.iter()
        .filter(|arg| {
            let id = !std::mem::take(&mut value) && !arg.starts_with('-');
            value = matches!(arg.as_str(), "-k" | "-p" | "-s" | "-t");
            id
        })
        .filter_map(|id| {
            let parts: Vec<&str> = id.split('.').collect();
            let resolved = (1..=parts.len()).rev().find_map(|split| {
                let file = format!("{}.py", parts[..split].join("/"));
                workdir.join(&file).is_file().then(|| {
                    let names = parts[split..]
                        .iter()
                        .map(|name| (*name).to_string())
                        .collect();
                    (file, names)
                })
            });
            resolved.or_else(|| {
                // The test module a task writes: the id up to its first
                // capitalized part (a class), else all of it.
                let split = parts
                    .iter()
                    .position(|part| part.starts_with(char::is_uppercase))
                    .unwrap_or(parts.len());
                (split > 0).then(|| (format!("{}.py", parts[..split].join("/")), Vec::new()))
            })
        })
        .collect()
}

/// The integration test file `cargo test [-p crate] --test name` runs, when
/// its crate's directory is known: `crates/<crate>`, `<crate>`, or the
/// workspace root for the root package.
fn cargo_test_file(args: &[String], workdir: &Path) -> Option<String> {
    let after = |flag: &str| {
        args.iter()
            .position(|arg| arg == flag)
            .and_then(|index| args.get(index + 1))
    };
    if args.first().map(String::as_str) != Some("test") {
        return None;
    }
    let name = after("--test").filter(|name| !name.contains(['*', '?']))?;
    let file = match after("-p").or_else(|| after("--package")) {
        Some(krate) => [format!("crates/{krate}"), krate.clone()]
            .into_iter()
            .find(|dir| workdir.join(dir).is_dir())
            .unwrap_or_else(|| format!("crates/{krate}")),
        None => String::new(),
    };
    let file = if file.is_empty() {
        format!("tests/{name}.rs")
    } else {
        format!("{file}/tests/{name}.rs")
    };
    Some(file)
}

/// Whether `text` defines `name`: a Python `def`/`class`, a Rust `fn`, or a
/// JavaScript `function`/test title.
fn defines(text: &str, name: &str) -> bool {
    static WORD: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"^[A-Za-z_][A-Za-z0-9_]*$").expect("valid name regex"));
    if !WORD.is_match(name) {
        return true;
    }
    let pattern = format!(r"\b(def|class|fn|function)\s+{}\b", regex::escape(name));
    Regex::new(&pattern).is_ok_and(|definition| definition.is_match(text))
}

/// `command` with its pytest runs as `python3 -m unittest` runs, when pytest
/// is not installed here, `python3` is, and every file it runs exists and
/// holds unittest tests; `None` otherwise.
fn pytest_as_unittest(command: &str, workdir: &Path, probe: &Probe) -> Option<String> {
    if probe.pytest() || !probe.command("python3") {
        return None;
    }
    let mut rewritten = command.to_string();
    let mut changed = false;
    for segment in shell_segments(command) {
        let words = command_words(&segment);
        let args: &[String] = match words.as_slice() {
            [program, args @ ..] if program == "pytest" || program == "py.test" => args,
            [program, flag, module, args @ ..]
                if (program == "python3" || program == "python")
                    && flag == "-m"
                    && module == "pytest" =>
            {
                args
            }
            _ => continue,
        };
        // Only quiet and fail-fast options carry over.
        if args
            .iter()
            .any(|arg| arg.starts_with('-') && !matches!(arg.as_str(), "-q" | "-x" | "-v"))
        {
            return None;
        }
        let mut ids = Vec::new();
        for target in args.iter().filter(|arg| !arg.starts_with('-')) {
            let mut parts = target.split("::");
            let path = file_arg(parts.next()?)?;
            let text = std::fs::read_to_string(workdir.join(&path)).ok()?;
            if !text.contains("unittest") {
                return None;
            }
            let module = path.strip_suffix(".py")?.replace('/', ".");
            let names: Vec<&str> = parts.collect();
            if names.iter().any(|name| name.contains(['[', '*'])) {
                return None;
            }
            ids.push(
                std::iter::once(module.as_str())
                    .chain(names)
                    .collect::<Vec<_>>()
                    .join("."),
            );
        }
        if ids.is_empty() || !rewritten.contains(&segment) {
            return None;
        }
        rewritten = rewritten.replacen(
            &segment,
            &format!("python3 -m unittest {}", ids.join(" ")),
            1,
        );
        changed = true;
    }
    changed.then_some(rewritten)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A unittest test module and a plan whose one task runs a test of it
    /// with the given command and writes `files`.
    fn fixture(command: &str, files: &[&str]) -> (tempfile::TempDir, TasksFile) {
        let temp = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir_all(temp.path().join("tests")).expect("tests");
        std::fs::write(
            temp.path().join("tests/test_osc.py"),
            "import unittest\n\nclass OscillatorTest(unittest.TestCase):\n    \
             def test_engine_0_produces_output(self):\n        pass\n",
        )
        .expect("a test module");
        let files: Vec<String> = files.iter().map(|file| format!("\"{file}\"")).collect();
        let plan = TasksFile::parse_str(&format!(
            "[meta]\nplan = \"p\"\ntotal = 1\nstatus = \"ready\"\n\n\
             [[task]]\nid = \"T1\"\ntitle = \"t\"\nstatus = \"ready\"\nrole = \"implementer\"\n\
             tier = \"focused\"\nfiles = [{}]\n\n\
             [[task.verify]]\nphase = \"test\"\ncommand = {command:?}\n",
            files.join(", ")
        ))
        .expect("a plan");
        (temp, plan)
    }

    /// Without pytest, a pytest step over unittest tests runs them with
    /// `python3 -m unittest`, and then it can pass.
    #[test]
    fn a_pytest_step_runs_as_unittest_without_pytest() {
        let probe = Probe::with(
            &[("pytest", false), ("python3", true), ("grep", true)],
            &[("pytest", false), ("unittest", true)],
        );
        let (temp, mut plan) = fixture(
            "grep -q 'class Oscillator' synth/osc.py && pytest -q \
             tests/test_osc.py::OscillatorTest::test_engine_0_produces_output",
            &["synth/osc.py"],
        );
        assert_eq!(
            problems(&plan, temp.path(), &probe)[0].why,
            "`pytest` is not installed"
        );
        let repairs = repair(&mut plan, temp.path(), &probe);
        assert_eq!(repairs.len(), 1, "{repairs:?}");
        assert_eq!(
            plan.tasks[0].verify[0].command,
            "grep -q 'class Oscillator' synth/osc.py && python3 -m unittest \
             tests.test_osc.OscillatorTest.test_engine_0_produces_output"
        );
        assert_eq!(problems(&plan, temp.path(), &probe), []);
    }

    /// A step that names a test its file does not define cannot pass, unless
    /// its task writes that file.
    #[test]
    fn a_test_the_file_does_not_define_cannot_pass() {
        let probe = Probe::with(&[("python3", true)], &[("unittest", true)]);
        let (temp, plan) = fixture(
            "python3 -m unittest tests.test_osc.OscillatorTest.test_lowpass_low_gain",
            &["synth/osc.py"],
        );
        let found = problems(&plan, temp.path(), &probe);
        assert_eq!(
            found[0].why,
            "it names `test_lowpass_low_gain`, which `tests/test_osc.py` does not define and no \
             task writes"
        );
        let (temp, plan) = fixture(
            "python3 -m unittest tests.test_osc.OscillatorTest.test_lowpass_low_gain",
            &["tests/test_osc.py"],
        );
        assert_eq!(problems(&plan, temp.path(), &probe), []);
    }

    /// A test file a task's check runs, which nothing has written, joins
    /// the task's files; a missing file that is not a test is reported.
    #[test]
    fn a_test_file_the_check_runs_joins_its_task_files() {
        let probe = Probe::with(&[("cargo", true), ("grep", true)], &[]);
        let (temp, mut plan) = fixture(
            "grep -q 'fn test_sc1' crates/seq/tests/tm_sc1.rs && cargo test -p seq --test tm_sc1",
            &["crates/seq/src/tm.rs"],
        );
        let repairs = repair(&mut plan, temp.path(), &probe);
        assert_eq!(
            repairs,
            ["T1: added `crates/seq/tests/tm_sc1.rs` to its files, since its check runs it"]
        );
        assert_eq!(problems(&plan, temp.path(), &probe), []);

        let (temp, plan) = fixture("grep -q seq docs/notes.md", &["crates/seq/src/tm.rs"]);
        assert_eq!(
            problems(&plan, temp.path(), &Probe::with(&[("grep", true)], &[]))[0].why,
            "it needs `docs/notes.md`, which does not exist and no task writes before it runs"
        );
    }

    /// The prompt says which runners this machine has.
    #[test]
    fn the_runner_note_says_pytest_is_missing() {
        let temp = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir_all(temp.path().join("tests")).expect("tests");
        let probe = Probe::with(
            &[("pytest", false), ("python3", true)],
            &[("pytest", false)],
        );
        let block = runners_block(temp.path(), &probe).expect("a Python project");
        assert!(block.contains("`pytest` is NOT installed here"), "{block}");
        assert!(runners_block(&temp.path().join("tests"), &probe).is_none());
    }
}
