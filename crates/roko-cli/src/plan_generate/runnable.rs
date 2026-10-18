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

/// The command that runs a source check stated as unittest ids (bug-8fe462),
/// such as `tests.test_osc.NoiseTest.*` or
/// `tests.test_svf.SVFTest.test_lowpass_*`, `test_bandpass_mode`, so the planner
/// need not invent test names: a class wildcard runs the class, a method
/// prefix runs the matching methods its file defines (or `-k <prefix>` over
/// the module when the file does not exist yet), and a bare method name
/// after an id belongs to that id's class. `None` when `text` names no
/// unittest id or `python3` is not installed.
pub(super) fn check_command(text: &str, workdir: &Path, probe: &Probe) -> Option<String> {
    static SPAN: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"`([^`]+)`").expect("valid code span regex"));
    static ID: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"^[A-Za-z_]\w*(\.[A-Za-z_]\w*)+(\.\w*\*)?$").expect("valid unittest id regex")
    });
    static METHOD: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"^test\w*\*?$").expect("valid method regex"));
    let mut ids: Vec<String> = Vec::new();
    let mut selections: Vec<String> = Vec::new();
    let mut class: Option<String> = None;
    for span in SPAN
        .captures_iter(text)
        .filter_map(|captures| captures.get(1))
    {
        let span = span.as_str().trim();
        let id = if ID.is_match(span) {
            span.to_string()
        } else if METHOD.is_match(span)
            && let Some(class) = &class
        {
            format!("{class}.{span}")
        } else {
            continue;
        };
        let (Some((module, _)), Some((head, last))) =
            (module_file(&id, workdir), id.rsplit_once('.'))
        else {
            continue;
        };
        if head != module {
            class = Some(head.to_string());
        }
        match last.strip_suffix('*') {
            None => ids.push(id.clone()),
            Some("") => ids.push(head.to_string()),
            Some(prefix) => {
                let defined = class_methods(&head, workdir)
                    .into_iter()
                    .filter(|method| method.starts_with(prefix))
                    .map(|method| format!("{head}.{method}"))
                    .collect::<Vec<_>>();
                if defined.is_empty() {
                    selections.push(format!("python3 -m unittest -k {prefix} {module}"));
                } else {
                    ids.extend(defined);
                }
            }
        }
    }
    if (ids.is_empty() && selections.is_empty()) || !probe.command("python3") {
        return None;
    }
    let mut commands = Vec::new();
    if !ids.is_empty() {
        commands.push(format!("python3 -m unittest {}", ids.join(" ")));
    }
    commands.extend(selections);
    Some(commands.join(" && "))
}

/// The dotted module of a unittest `id` and its file: the longest leading
/// part that is a `.py` file in `workdir`, or, for a test module not written
/// yet, the part up to the first capitalized name (a class). `None` when
/// neither a file nor a `test` package or module names it.
fn module_file(id: &str, workdir: &Path) -> Option<(String, String)> {
    let parts: Vec<&str> = id.split('.').collect();
    let existing = (1..parts.len()).rev().find_map(|split| {
        let file = format!("{}.py", parts[..split].join("/"));
        workdir
            .join(&file)
            .is_file()
            .then(|| (parts[..split].join("."), file))
    });
    existing.or_else(|| {
        let split = parts
            .iter()
            .position(|part| part.starts_with(char::is_uppercase))?;
        let module = &parts[..split];
        module
            .iter()
            .any(|part| part.starts_with("test"))
            .then(|| (module.join("."), format!("{}.py", module.join("/"))))
    })
}

/// The `test` methods class `id` (`module.Class`) defines in its file, in
/// file order; none when the file or the class is not there.
fn class_methods(id: &str, workdir: &Path) -> Vec<String> {
    static DEF: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"^\s+def\s+(test\w*)\s*\(").expect("valid def regex"));
    let Some((module, class)) = id.rsplit_once('.') else {
        return Vec::new();
    };
    let file = format!("{}.py", module.replace('.', "/"));
    let Ok(text) = std::fs::read_to_string(workdir.join(file)) else {
        return Vec::new();
    };
    let header = format!("class {class}");
    let mut inside = false;
    let mut methods = Vec::new();
    for line in text.lines() {
        if !line.starts_with([' ', '\t']) && !line.trim().is_empty() {
            inside = line
                .strip_prefix(&header)
                .is_some_and(|rest| rest.starts_with(['(', ':']));
            continue;
        }
        if inside && let Some(name) = DEF.captures(line).and_then(|captures| captures.get(1)) {
            methods.push(name.as_str().to_string());
        }
    }
    methods
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

/// The files of `plan.tasks[index]` that neither exist in `workdir` nor are
/// written by a task it depends on: what the task itself creates.
fn created_by(plan: &TasksFile, index: usize, workdir: &Path) -> Vec<String> {
    let task = &plan.tasks[index];
    let before: Vec<String> = written_before_and_by(plan, index)
        .into_iter()
        .skip(task.files.len())
        .collect();
    task.files
        .iter()
        .filter(|file| !workdir.join(file).exists() && !is_written(&before, file))
        .cloned()
        .collect()
}

/// The tasks of `plan` that create a file (one that neither exists nor is
/// written by a task before them) while their `allowed_tools` lack
/// `write_file`, or their `denied_tools` hold it (bug-df9aff): no try can
/// create the file. Each with the first such file.
pub fn creates_without_write(plan: &TasksFile, workdir: &Path) -> Vec<(String, String)> {
    (0..plan.tasks.len())
        .filter_map(|index| {
            let task = &plan.tasks[index];
            let barred = task
                .allowed_tools
                .as_ref()
                .is_some_and(|allowed| !allowed.iter().any(|tool| tool == "write_file"))
                || task
                    .denied_tools
                    .as_ref()
                    .is_some_and(|denied| denied.iter().any(|tool| tool == "write_file"));
            let created = barred.then(|| created_by(plan, index, workdir))?;
            created
                .into_iter()
                .next()
                .map(|file| (task.id.clone(), file))
        })
        .collect()
}

/// Give each task that creates a file the tool to write it (bug-df9aff):
/// `write_file` joins an `allowed_tools` that lacks it and leaves a
/// `denied_tools` that holds it. Returns one line per task repaired.
pub(super) fn repair_write_tools(plan: &mut TasksFile, workdir: &Path) -> Vec<String> {
    let barred = creates_without_write(plan, workdir);
    let mut repairs = Vec::new();
    for (id, file) in barred {
        let Some(task) = plan.tasks.iter_mut().find(|task| task.id == id) else {
            continue;
        };
        if let Some(allowed) = task.allowed_tools.as_mut()
            && !allowed.iter().any(|tool| tool == "write_file")
        {
            allowed.push("write_file".to_string());
        }
        if let Some(denied) = task.denied_tools.as_mut() {
            denied.retain(|tool| tool != "write_file");
        }
        repairs.push(format!(
            "{id}: may use write_file, since it creates `{file}`"
        ));
    }
    repairs
}

/// The dependencies `plan` misses (bug-df9aff): for each task, each other
/// task whose `files` write a Python module that a test file of the task's
/// own checks (one that exists in `workdir`) imports, when the task does not
/// already wait for it and waiting would not make a cycle. Each as the task,
/// the task to wait for, and the module's file.
pub fn missing_dependencies(plan: &TasksFile, workdir: &Path) -> Vec<(String, String, String)> {
    let mut missing = Vec::new();
    for task in &plan.tasks {
        let mut imported: Vec<String> = Vec::new();
        for step in &task.verify {
            for test_file in step_files(&step.command, workdir) {
                let is_python = Path::new(&test_file)
                    .extension()
                    .is_some_and(|extension| extension == "py");
                if !is_python {
                    continue;
                }
                let Ok(text) = std::fs::read_to_string(workdir.join(&test_file)) else {
                    continue;
                };
                imported.extend(imported_files(&text));
            }
        }
        for other in &plan.tasks {
            if other.id == task.id || task.depends_on.contains(&other.id) {
                continue;
            }
            let Some(module) = other.files.iter().find(|file| imported.contains(file)) else {
                continue;
            };
            if waits_for(plan, &task.id, &other.id) || waits_for(plan, &other.id, &task.id) {
                continue;
            }
            missing.push((task.id.clone(), other.id.clone(), module.clone()));
        }
    }
    missing
}

/// Make each task wait for the tasks that write the modules its checks'
/// tests import ([`missing_dependencies`]). Returns one line per
/// dependency added.
pub(super) fn repair_dependencies(plan: &mut TasksFile, workdir: &Path) -> Vec<String> {
    let mut repairs = Vec::new();
    for (task, dependency, module) in missing_dependencies(plan, workdir) {
        // An earlier addition may have made this one a cycle.
        if waits_for(plan, &dependency, &task) {
            continue;
        }
        if let Some(waiting) = plan.tasks.iter_mut().find(|candidate| candidate.id == task)
            && !waiting.depends_on.contains(&dependency)
        {
            waiting.depends_on.push(dependency.clone());
            repairs.push(format!(
                "{task}: depends on {dependency}, which writes `{module}` its checks import"
            ));
        }
    }
    repairs
}

/// Whether task `from` waits for task `target`, directly or not.
fn waits_for(plan: &TasksFile, from: &str, target: &str) -> bool {
    let mut pending = vec![from];
    let mut seen: Vec<&str> = Vec::new();
    while let Some(id) = pending.pop() {
        if seen.contains(&id) {
            continue;
        }
        seen.push(id);
        let Some(task) = plan.tasks.iter().find(|task| task.id == id) else {
            continue;
        };
        if task
            .depends_on
            .iter()
            .any(|dependency| dependency == target)
        {
            return true;
        }
        pending.extend(task.depends_on.iter().map(String::as_str));
    }
    false
}

/// The workspace files a Python source's absolute imports may name:
/// `import a.b` gives `a/b.py` and `a/b/__init__.py`, and `from a.b import
/// c` gives `a/b/c.py` (a submodule) and `a/b.py`.
fn imported_files(text: &str) -> Vec<String> {
    static IMPORT: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"^\s*import\s+([\w.]+(?:\s+as\s+\w+)?(?:\s*,\s*[\w.]+(?:\s+as\s+\w+)?)*)")
            .expect("valid import regex")
    });
    static FROM: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"^\s*from\s+([A-Za-z_][\w.]*)\s+import\s+\(?([\w\s,]+)")
            .expect("valid from-import regex")
    });
    let as_file = |module: &str| module.replace('.', "/");
    let mut files = Vec::new();
    for line in text.lines() {
        if let Some(modules) = IMPORT.captures(line).and_then(|captures| captures.get(1)) {
            for module in modules.as_str().split(',') {
                let module = module.split_whitespace().next().unwrap_or_default();
                files.push(format!("{}.py", as_file(module)));
                files.push(format!("{}/__init__.py", as_file(module)));
            }
        } else if let Some(captures) = FROM.captures(line) {
            let (Some(module), Some(names)) = (captures.get(1), captures.get(2)) else {
                continue;
            };
            let module = module.as_str();
            files.push(format!("{}.py", as_file(module)));
            for name in names.as_str().split(',') {
                let name = name.split_whitespace().next().unwrap_or_default();
                if !name.is_empty() {
                    files.push(format!("{}/{name}.py", as_file(module)));
                }
            }
        }
    }
    files
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
pub(super) mod tests {
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

    /// bug-8fe462: a check stated as a unittest wildcard gets the command
    /// that runs its real tests: a class wildcard runs the class, a method
    /// prefix the matching methods its file defines (or `-k` while the file
    /// does not exist), and a bare method name joins the id before it.
    #[test]
    fn wildcard_checks_expand_to_their_tests() {
        let temp = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir_all(temp.path().join("tests")).expect("tests");
        std::fs::write(
            temp.path().join("tests/test_svf.py"),
            "import unittest\n\nclass SVFTest(unittest.TestCase):\n    \
             def test_lowpass_attenuates_highs(self):\n        pass\n\n    \
             def test_lowpass_passes_lows(self):\n        pass\n\n    \
             def test_highpass_mode(self):\n        pass\n\n\
             class OtherTest(unittest.TestCase):\n    def test_lowpass_elsewhere(self):\n        pass\n",
        )
        .expect("a test module");
        let probe = Probe::with(&[("python3", true)], &[]);
        let run = |text: &str| check_command(text, temp.path(), &probe);
        assert_eq!(
            run("INV-4 | `tests.test_osc.NoiseTest.*` | white: |mean| < 0.05").as_deref(),
            Some("python3 -m unittest tests.test_osc.NoiseTest")
        );
        assert_eq!(
            run("INV-5 | `tests.test_svf.SVFTest.test_lowpass_*` | LP 400 Hz").as_deref(),
            Some(
                "python3 -m unittest tests.test_svf.SVFTest.test_lowpass_attenuates_highs \
                 tests.test_svf.SVFTest.test_lowpass_passes_lows"
            )
        );
        assert_eq!(
            run("INV-6 | `tests.test_svf.SVFTest.test_highpass_mode`, `test_bandpass_mode` | HP")
                .as_deref(),
            Some(
                "python3 -m unittest tests.test_svf.SVFTest.test_highpass_mode \
                 tests.test_svf.SVFTest.test_bandpass_mode"
            )
        );
        assert_eq!(
            run("`tests.test_env.ADSRTest.test_adsr_*` shapes the envelope").as_deref(),
            Some("python3 -m unittest -k test_adsr_ tests.test_env")
        );
        assert_eq!(run("`TuringMachine` at `change=0.0` repeats"), None);
        assert_eq!(
            check_command(
                "`tests.test_osc.NoiseTest.*`",
                temp.path(),
                &Probe::with(&[], &[])
            ),
            None,
            "no python3"
        );
    }

    /// The shape of round v6's G1 plan (bug-df9aff): four tasks that each
    /// create a module with `allowed_tools = ["read_file", "edit_file",
    /// "grep"]`, and a render task whose tests import the other three
    /// modules but that waits for no task. Returns the workspace and plan.
    pub(crate) fn g1_plan() -> (tempfile::TempDir, TasksFile) {
        let temp = tempfile::tempdir().expect("tempdir");
        for dir in ["synth", "tests"] {
            std::fs::create_dir_all(temp.path().join(dir)).expect("a directory");
        }
        std::fs::write(
            temp.path().join("synth/__init__.py"),
            "SAMPLE_RATE = 48_000\n",
        )
        .expect("the package");
        for (module, imports) in [
            ("osc", "from synth.osc import Oscillator\n"),
            ("svf", "from synth.svf import SVF\n"),
            ("env", "from synth.env import ADSR\n"),
            (
                "render",
                "from synth import osc, svf\nimport synth.env\nfrom synth.render import render\n",
            ),
        ] {
            std::fs::write(
                temp.path().join(format!("tests/test_{module}.py")),
                format!("import unittest\n{imports}\nclass T(unittest.TestCase):\n    pass\n"),
            )
            .expect("a test module");
        }
        let task = |id: &str, module: &str| {
            format!(
                "[[task]]\nid = \"{id}\"\ntitle = \"{module}\"\nstatus = \"ready\"\n\
                 role = \"implementer\"\ntier = \"focused\"\nfiles = [\"synth/{module}.py\"]\n\
                 allowed_tools = [\"read_file\", \"edit_file\", \"grep\"]\ndepends_on = []\n\n\
                 [[task.verify]]\nphase = \"test\"\n\
                 command = \"python3 -m unittest tests.test_{module}\"\n\n"
            )
        };
        let plan = TasksFile::parse_str(&format!(
            "[meta]\nplan = \"g1\"\ntotal = 4\nstatus = \"ready\"\n\n{}{}{}{}",
            task("T1", "osc"),
            task("T2", "svf"),
            task("T3", "env"),
            task("T4", "render")
        ))
        .expect("a plan");
        (temp, plan)
    }

    /// bug-df9aff: in G1's plan no task could create its module, and render
    /// could run before the modules its tests import. Repaired, every task
    /// may write, and render waits for the three.
    #[test]
    fn g1_tasks_may_write_their_modules_and_render_waits_for_them() {
        let (temp, mut plan) = g1_plan();
        let barred = creates_without_write(&plan, temp.path());
        assert_eq!(
            barred,
            [
                ("T1".to_string(), "synth/osc.py".to_string()),
                ("T2".to_string(), "synth/svf.py".to_string()),
                ("T3".to_string(), "synth/env.py".to_string()),
                ("T4".to_string(), "synth/render.py".to_string()),
            ]
        );
        let missing: Vec<(String, String)> = missing_dependencies(&plan, temp.path())
            .into_iter()
            .map(|(task, dependency, _)| (task, dependency))
            .collect();
        assert_eq!(
            missing,
            [
                ("T4".to_string(), "T1".to_string()),
                ("T4".to_string(), "T2".to_string()),
                ("T4".to_string(), "T3".to_string()),
            ]
        );

        let written = repair_write_tools(&mut plan, temp.path());
        assert_eq!(written.len(), 4, "{written:?}");
        assert_eq!(
            written[0],
            "T1: may use write_file, since it creates `synth/osc.py`"
        );
        let waits = repair_dependencies(&mut plan, temp.path());
        assert_eq!(
            waits[0],
            "T4: depends on T1, which writes `synth/osc.py` its checks import"
        );
        assert_eq!(plan.tasks[3].depends_on, ["T1", "T2", "T3"]);
        assert!(plan.tasks.iter().all(|task| {
            task.allowed_tools
                .as_ref()
                .is_some_and(|tools| tools.iter().any(|tool| tool == "write_file"))
        }));
        assert!(creates_without_write(&plan, temp.path()).is_empty());
        assert!(missing_dependencies(&plan, temp.path()).is_empty());
    }

    /// bug-df9aff: `plan validate` warns about G1's plan: each task creates
    /// a module it may not write (PLAN_052), and the render task's tests
    /// import modules other tasks write without waiting for them (PLAN_053).
    #[test]
    fn plan_validate_warns_about_g1_tasks_that_cannot_create_or_wait() {
        let (temp, plan) = g1_plan();
        let plan_dir = temp.path().join("plans/g1");
        std::fs::create_dir_all(&plan_dir).expect("the plan directory");
        std::fs::write(
            plan_dir.join("tasks.toml"),
            toml::to_string_pretty(&plan).expect("the plan as TOML"),
        )
        .expect("tasks.toml");
        let report = crate::plan_validate::validate_plans_dir_with_workdir(
            &temp.path().join("plans"),
            None,
            Some(temp.path()),
        )
        .expect("validate");
        let found = |rule: &str| {
            report.plans[0]
                .diagnostics
                .iter()
                .filter(|diagnostic| diagnostic.rule_id == rule)
                .count()
        };
        assert_eq!(found("PLAN_052"), 4, "{:?}", report.plans[0].diagnostics);
        assert_eq!(found("PLAN_053"), 3, "{:?}", report.plans[0].diagnostics);
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
