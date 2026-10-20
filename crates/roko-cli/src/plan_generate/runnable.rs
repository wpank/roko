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
//!
//! A step that only reads files no task of its plan writes is settled before
//! the plan runs (bug-27f00b): it fails on the base and no task can make it
//! pass, or it passes there and checks nothing its task does. Such a step
//! runs once, read-only, to tell which ([`settled_steps`]).

use std::collections::HashMap;
use std::path::Path;
use std::sync::{LazyLock, Mutex, PoisonError};
use std::time::Duration;

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

/// Files that make a workspace a Python project (bug-574c45).
const PYTHON_PROJECT_FILES: &[&str] = &[
    "pyproject.toml",
    "requirements.txt",
    "setup.cfg",
    "setup.py",
];

/// Directories a search for a project's `CMakeLists.txt` leaves out: build
/// output and vendored code (bug-574c45).
const NOT_THE_PROJECT: &[&str] = &["build", "node_modules", "target", "third_party", "vendor"];

/// Build tools that run only beside their manifest (bug-72dc0d): each tool,
/// the subcommands that need the manifest, and the manifest's file names.
/// The tool looks for it in its directory and the directories above.
const MANIFESTS: &[(&str, &[&str], &[&str])] = &[
    (
        "cargo",
        &[
            "b", "bench", "build", "c", "check", "clippy", "d", "doc", "fix", "fmt", "metadata",
            "nextest", "r", "run", "rustc", "t", "test", "tree",
        ],
        &["Cargo.toml"],
    ),
    (
        "go",
        &["build", "generate", "list", "run", "test", "vet"],
        &["go.mod"],
    ),
    (
        "npm",
        &["ci", "run", "run-script", "start", "t", "test"],
        &["package.json"],
    ),
    ("pnpm", &["run", "start", "t", "test"], &["package.json"]),
    ("yarn", &["run", "start", "test"], &["package.json"]),
];

/// The longest a step the base settles may run when generation or `plan
/// validate` runs it to see how it comes out (bug-27f00b).
const SETTLED_TIMEOUT: Duration = Duration::from_secs(10);

/// The single-letter `test` operators that test a file (`-f path`).
const FILE_TESTS: &str = "bcdefghkprsuwxGLNOS";

/// `grep`'s short options that take a value: the rest of their word, or the
/// next word.
const GREP_SHORT_VALUES: &str = "efmABCdD";

/// `grep`'s long options that take a value: after `=`, or the next word.
const GREP_LONG_VALUES: &[&str] = &[
    "after-context",
    "before-context",
    "binary-files",
    "color",
    "colour",
    "context",
    "devices",
    "directories",
    "exclude",
    "exclude-dir",
    "exclude-from",
    "file",
    "group-separator",
    "include",
    "label",
    "max-count",
    "regexp",
];

/// A verify step that cannot pass as written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unrunnable {
    /// Its task's id.
    pub task: String,
    /// Its number among its task's verify steps, from 1.
    pub step: usize,
    /// Why it cannot pass.
    pub why: String,
}

/// What this machine can run, asked once per name.
#[derive(Debug, Default)]
pub struct Probe {
    commands: Mutex<HashMap<String, bool>>,
    modules: Mutex<HashMap<String, bool>>,
    /// Whether each step the base settles passes, by command (bug-27f00b);
    /// `None` when it did not finish.
    settled: Mutex<HashMap<String, Option<bool>>>,
    /// Answer only from the names given, never asking the machine (tests).
    given_only: bool,
    /// Take every command and Python module as present, never asking the
    /// machine ([`Probe::portable`]).
    everywhere: bool,
}

impl Probe {
    /// A probe that takes every command and Python module as present, so
    /// the problems it lets a check find hold on any machine (gap-ff78e1):
    /// static `plan validate` reports the same plan the same way everywhere.
    #[must_use]
    pub fn portable() -> Self {
        Self {
            everywhere: true,
            ..Self::default()
        }
    }

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
            ..Self::default()
        }
    }

    /// Whether `command`, a step the base settles, passes when it runs once
    /// in `workdir` (bug-27f00b). It runs even for a probe of given names:
    /// it only reads files of the workspace.
    fn settled_passes(&self, command: &str, workdir: &Path) -> Option<bool> {
        *self
            .settled
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .entry(command.to_string())
            .or_insert_with(|| run_settled(command, workdir))
    }

    /// Whether `name` is an executable on `PATH`.
    pub(super) fn command(&self, name: &str) -> bool {
        self.everywhere
            || *self
                .commands
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .entry(name.to_string())
                .or_insert_with(|| !self.given_only && crate::config::command_on_path(name))
    }

    /// Whether `python3` can import the top-level package of `module`.
    fn python_module(&self, module: &str) -> bool {
        if self.everywhere {
            return true;
        }
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
/// kinds of project `workdir` holds; `None` when it holds none of them. A
/// project is Python by its own files or a Python file in `tests/`, not by a
/// `tests/` directory, which a C++ project has too; a CMake project gets the
/// cmake and ctest it builds and tests with (bug-574c45).
pub(super) fn runners_block(workdir: &Path, probe: &Probe) -> Option<String> {
    let mut lines = Vec::new();
    let python = PYTHON_PROJECT_FILES
        .iter()
        .any(|file| workdir.join(file).is_file())
        || holds_extension(&workdir.join("tests"), "py", 2);
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
    if holds_cmake_lists(workdir, 2) {
        let line = match (probe.command("cmake"), probe.command("ctest")) {
            (true, true) if workdir.join("CMakePresets.json").is_file() => {
                "- CMake: `cmake` and `ctest` run here. Build and test with the presets: `cmake \
                 --preset <name>`, `cmake --build --preset <name>`, `ctest --preset <name> -R \
                 <test>`."
            }
            (true, true) => {
                "- CMake: `cmake` and `ctest` run here: `cmake -S . -B build`, `cmake --build \
                 build`, `ctest --test-dir build -R <test>`."
            }
            _ => "- CMake: `cmake` or `ctest` is NOT installed here.",
        };
        lines.push(line.to_string());
    }
    if workdir.join("package.json").is_file() && !probe.command("npm") {
        lines.push("- JavaScript: `npm` is NOT installed here.".to_string());
    }
    if workdir.join("go.mod").is_file() && !probe.command("go") {
        lines.push("- Go: `go` is NOT installed here.".to_string());
    }
    // bug-8723fc: a project of another kind has no cargo build to check.
    if !lines.is_empty() && !workdir.join("Cargo.toml").is_file() {
        lines.push("- Rust: there is no `Cargo.toml` here, so no check runs `cargo`.".to_string());
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

/// Whether `dir`, or a directory under it at most `depth` levels down,
/// holds a file with `extension`.
fn holds_extension(dir: &Path, extension: &str, depth: usize) -> bool {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return false;
    };
    entries.filter_map(Result::ok).any(|entry| {
        let path = entry.path();
        if entry.file_type().is_ok_and(|kind| kind.is_dir()) {
            depth > 0 && holds_extension(&path, extension, depth - 1)
        } else {
            path.extension()
                .is_some_and(|found| found == std::ffi::OsStr::new(extension))
        }
    })
}

/// Whether `dir`, or a directory under it at most `depth` levels down that is
/// not build output or vendored code, holds a `CMakeLists.txt`: a seed's
/// `tests/spec/CMakeLists.txt` makes a CMake project before the top one is
/// written.
pub(super) fn holds_cmake_lists(dir: &Path, depth: usize) -> bool {
    if dir.join("CMakeLists.txt").is_file() {
        return true;
    }
    depth > 0
        && std::fs::read_dir(dir).is_ok_and(|entries| {
            entries.filter_map(Result::ok).any(|entry| {
                let name = entry.file_name();
                let name = name.to_string_lossy();
                entry.file_type().is_ok_and(|kind| kind.is_dir())
                    && !name.starts_with('.')
                    && !NOT_THE_PROJECT.contains(&name.as_ref())
                    && holds_cmake_lists(&entry.path(), depth - 1)
            })
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

/// What an implementer needs to build, run its own checks and patch a file
/// (bug-d758f6).
const SHELL_TOOLS: &[&str] = &["bash", "edit_file"];

/// Whether `task` is an implementer: its role, or the default when it names
/// none.
fn is_implementer(task: &TaskDef) -> bool {
    task.role
        .as_deref()
        .map(str::trim)
        .is_none_or(|role| role.is_empty() || role == "implementer")
}

/// Whether `task`'s `allowed_tools` leave out a shell or `edit_file`; an
/// empty list leaves the role's tools as they are.
fn lists_out_shell(task: &TaskDef) -> bool {
    task.allowed_tools
        .as_ref()
        .filter(|tools| !tools.is_empty())
        .is_some_and(|tools| {
            SHELL_TOOLS
                .iter()
                .any(|tool| !tools.iter().any(|kept| kept == tool))
        })
}

/// Give each implementer task its role's own tools when its `allowed_tools`
/// leave out a shell or `edit_file`, or its `denied_tools` hold a tool it
/// needs to build, run its checks or write (bug-d758f6). Planners copied the
/// generator prompt's three-tool examples into every task, and an
/// implementer kept to read_file, write_file and grep cannot build, run its
/// own checks or see a compiler error, so each compile error costs a whole
/// try. Such a list goes, and `denied_tools` lose bash, edit_file and
/// write_file. Returns one line per task repaired.
pub(super) fn repair_shell_tools(plan: &mut TasksFile) -> Vec<String> {
    let mut repairs = Vec::new();
    for task in &mut plan.tasks {
        if !is_implementer(task) {
            continue;
        }
        let mut why = Vec::new();
        if lists_out_shell(task)
            && let Some(tools) = task.allowed_tools.take()
        {
            why.push(format!(
                "its allowed_tools [{}] left out bash or edit_file",
                tools.join(", ")
            ));
        }
        if let Some(denied) = task.denied_tools.as_mut() {
            let held: Vec<String> = denied
                .iter()
                .filter(|tool| matches!(tool.as_str(), "bash" | "edit_file" | "write_file"))
                .cloned()
                .collect();
            if !held.is_empty() {
                denied.retain(|tool| !held.contains(tool));
                why.push(format!("its denied_tools held {}", held.join(", ")));
            }
        }
        if !why.is_empty() {
            repairs.push(format!(
                "{}: keeps an implementer's own tools, bash and edit_file among them: {}",
                task.id,
                why.join(", and ")
            ));
        }
    }
    repairs
}

/// The implementer tasks of `plan` that cannot run their own checks
/// (bug-d758f6): one of their verify steps runs more than file reads
/// ([`read_only_files`]), but their `allowed_tools` leave out bash or their
/// `denied_tools` hold it. Each with that step's command.
#[must_use]
pub fn implementers_without_a_shell(plan: &TasksFile) -> Vec<(String, String)> {
    plan.tasks
        .iter()
        .filter(|task| is_implementer(task))
        .filter(|task| {
            let listed_out = task
                .allowed_tools
                .as_ref()
                .filter(|tools| !tools.is_empty())
                .is_some_and(|tools| !tools.iter().any(|tool| tool == "bash"));
            let denied = task
                .denied_tools
                .iter()
                .flatten()
                .any(|tool| tool == "bash");
            listed_out || denied
        })
        .filter_map(|task| {
            let step = task
                .verify
                .iter()
                .find(|step| read_only_files(&step.command).is_none())?;
            Some((task.id.clone(), step.command.clone()))
        })
        .collect()
}

/// Extensions of the source files a CMake target lists (bug-daa54e).
const CMAKE_SOURCES: &[&str] = &["c", "cc", "cpp", "cxx", "m", "mm"];

/// File extensions of C and C++ headers.
const C_HEADERS: &[&str] = &["h", "hh", "hpp", "hxx"];

/// Give each implementer task that writes C or C++ files and has no verify
/// step a check that builds them with the workspace's CMake (bug-8723fc):
/// configure and build with the preset named for the files' first directory,
/// or the first preset (`cmake --preset core && cmake --build --preset core`).
/// The TOML fix-up gives only a task that writes Rust a cargo check, so a task
/// in a workspace with no build roko knows keeps no step. Returns one line per
/// step added.
pub(super) fn repair_build_checks(
    plan: &mut TasksFile,
    workdir: &Path,
    probe: &Probe,
) -> Vec<String> {
    let Some(cmake) = super::build_system::CmakeCommands::of(workdir) else {
        return Vec::new();
    };
    let mut added = Vec::new();
    for task in &mut plan.tasks {
        let implementer = task
            .role
            .as_deref()
            .is_none_or(|role| role == "implementer");
        let source = task.files.iter().find(|file| {
            Path::new(file)
                .extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| {
                    CMAKE_SOURCES.contains(&extension) || C_HEADERS.contains(&extension)
                })
        });
        let Some(source) = source.filter(|_| implementer && task.verify.is_empty()) else {
            continue;
        };
        if !probe.command("cmake") {
            continue;
        }
        let command = cmake.build_command(source.split('/').next());
        added.push(format!(
            "{}: added `{command}`: it had no verify step, and this workspace builds with CMake",
            task.id
        ));
        task.verify.push(crate::task_parser::VerifyStep {
            phase: "compile".to_string(),
            command,
            fail_msg: Some("the task's sources must build".to_string()),
            timeout_ms: crate::task_parser::default_verify_timeout(),
            scope: Vec::new(),
            covers: Vec::new(),
            expect: None,
        });
    }
    added
}

/// Add to each task that creates a C or C++ source the CMake file that must
/// list it (bug-daa54e): the nearest `CMakeLists.txt` above the source, one
/// that exists or that the task or a task before it writes, unless it
/// collects its sources with a glob. Phase 1's oscillator, filter and envelope
/// tasks each created a source under `core/`, whose `CMakeLists.txt` names
/// every source, without listing that file: each edit was out of scope. A
/// build-registration file may sit in several tasks' `files`: generation
/// counts it as neither split ownership nor an overlap
/// ([`roko_graph::exclusion::is_registration_file`]). Returns one line per
/// file added.
pub(super) fn repair_build_lists(plan: &mut TasksFile, workdir: &Path) -> Vec<String> {
    let mut repairs = Vec::new();
    for index in 0..plan.tasks.len() {
        let written = written_before_and_by(plan, index);
        let mut lists: Vec<(String, String)> = Vec::new();
        for source in created_by(plan, index, workdir) {
            let compiled = Path::new(&source)
                .extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| CMAKE_SOURCES.contains(&extension));
            if !compiled {
                continue;
            }
            let Some(list) = cmake_list_for(&source, workdir, &written) else {
                continue;
            };
            let task = &plan.tasks[index];
            if !task.files.contains(&list) && !lists.iter().any(|(added, _)| *added == list) {
                lists.push((list, source));
            }
        }
        for (list, source) in lists {
            let task = &mut plan.tasks[index];
            repairs.push(format!(
                "{}: added `{list}` to its files: it creates `{source}`, which that CMake file \
                 lists",
                task.id
            ));
            task.files.push(list);
        }
    }
    repairs
}

/// The `CMakeLists.txt` that lists `source`: the nearest one in a directory
/// above it, that exists in `workdir` or is among `written`; `None` when
/// there is none, or when it collects sources with `file(GLOB …)`.
fn cmake_list_for(source: &str, workdir: &Path, written: &[String]) -> Option<String> {
    let mut dir = Path::new(source).parent();
    while let Some(current) = dir {
        let list = current.join("CMakeLists.txt");
        let list = list.to_string_lossy().trim_start_matches("./").to_string();
        let on_disk = workdir.join(&list);
        if on_disk.is_file() {
            let text = std::fs::read_to_string(&on_disk).unwrap_or_default();
            return (!text.contains("GLOB")).then_some(list);
        }
        if is_written(written, &list) {
            return Some(list);
        }
        dir = current.parent();
    }
    None
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
/// task that writes a Python module the task uses, when the task does not
/// already wait for it and waiting would not make a cycle. A task uses a
/// module when a test file of its own checks (one that exists in `workdir`)
/// imports it, or when its title, description or acceptance criteria name
/// something the plan's test files import from it (`from synth.osc import
/// Oscillator` makes `Oscillator` a name of `synth/osc.py`). Each as the
/// task, the task to wait for, and the module's file.
pub fn missing_dependencies(plan: &TasksFile, workdir: &Path) -> Vec<(String, String, String)> {
    // The test files each task's checks run, read once.
    let tests: Vec<Vec<String>> = plan
        .tasks
        .iter()
        .map(|task| {
            task.verify
                .iter()
                .flat_map(|step| step_files(&step.command, workdir))
                .filter(|file| {
                    Path::new(file)
                        .extension()
                        .is_some_and(|extension| extension == "py")
                })
                .filter_map(|file| std::fs::read_to_string(workdir.join(file)).ok())
                .collect()
        })
        .collect();
    // What the test files import, by name: `Oscillator` -> `synth/osc.py`.
    let mut names: Vec<(String, String)> = Vec::new();
    for text in tests.iter().flatten() {
        for (name, module) in imported_names(text) {
            if !names.contains(&(name.clone(), module.clone())) {
                names.push((name, module));
            }
        }
    }
    let mut missing = Vec::new();
    for (task, texts) in plan.tasks.iter().zip(&tests) {
        let mut used: Vec<String> = texts.iter().flat_map(|text| imported_files(text)).collect();
        let prose = std::iter::once(task.title.as_str())
            .chain(task.description.as_deref())
            .chain(task.acceptance.iter().map(String::as_str))
            .collect::<Vec<_>>()
            .join("\n");
        used.extend(
            names
                .iter()
                .filter(|(name, _)| mentions(&prose, name))
                .map(|(_, module)| module.clone()),
        );
        for other in &plan.tasks {
            if other.id == task.id || task.depends_on.contains(&other.id) {
                continue;
            }
            let Some(module) = other.files.iter().find(|file| used.contains(file)) else {
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

/// The names a Python source imports from workspace modules, each with its
/// module's file: `from synth.osc import Oscillator, Noise` gives
/// (`Oscillator`, `synth/osc.py`) and (`Noise`, `synth/osc.py`). Names of
/// fewer than three characters are left out.
fn imported_names(text: &str) -> Vec<(String, String)> {
    static FROM: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"^\s*from\s+([A-Za-z_][\w.]*)\s+import\s+\(?([\w\s,]+)")
            .expect("valid from-import regex")
    });
    let mut names = Vec::new();
    for captures in text.lines().filter_map(|line| FROM.captures(line)) {
        let (Some(module), Some(imported)) = (captures.get(1), captures.get(2)) else {
            continue;
        };
        let file = format!("{}.py", module.as_str().replace('.', "/"));
        for name in imported.as_str().split(',') {
            let name = name.split_whitespace().next().unwrap_or_default();
            if name.len() >= 3 {
                names.push((name.to_string(), file.clone()));
            }
        }
    }
    names
}

/// Whether `prose` names `name` as a whole word, in its own case.
fn mentions(prose: &str, name: &str) -> bool {
    prose.match_indices(name).any(|(start, _)| {
        let before = prose[..start].chars().next_back();
        let after = prose[start + name.len()..].chars().next();
        let joined = |c: Option<char>| c.is_some_and(|c| c.is_alphanumeric() || c == '_');
        !joined(before) && !joined(after)
    })
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
                "{task}: depends on {dependency}, which writes `{module}` it uses"
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

/// The verify steps of `plan` that cannot pass as written, in plan order:
/// one of [`step_problem`]'s, or a step the base settles that fails there
/// (bug-27f00b).
pub(super) fn problems(plan: &TasksFile, workdir: &Path, probe: &Probe) -> Vec<Unrunnable> {
    let settled = settled_steps_with(plan, workdir, |command| {
        probe.settled_passes(command, workdir)
    });
    // Round v7, phase-0 finding #2: a step that needs a later task's file
    // says which task writes it.
    let later = later_files(plan, workdir);
    let mut found = Vec::new();
    for (index, task) in plan.tasks.iter().enumerate() {
        let written = written_before_and_by(plan, index);
        for (number, step) in (1..).zip(&task.verify) {
            let needs_later = later
                .iter()
                .find(|later| !later.guessed && later.task == task.id && later.step == number)
                .map(LaterFile::why);
            let why = needs_later
                .or_else(|| step_problem(&step.command, workdir, &written, probe))
                .or_else(|| {
                    settled
                        .iter()
                        .find(|settled| {
                            settled.task == task.id
                                && settled.step == number
                                && settled.passes == Some(false)
                        })
                        .map(SettledStep::why)
                });
            if let Some(why) = why {
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

/// The verify steps of `plan` that cannot pass as written, in plan order,
/// for `plan validate` (gap-ff78e1): [`step_problem`]'s reasons. With a
/// [`Probe::portable`], those that hold on any machine: a build tool without
/// its manifest, a file that does not exist and no task writes before the
/// step runs, or a test its file does not define. A probe of this machine
/// adds a runner, or a Python module, it does not have.
#[must_use]
pub fn unrunnable_steps(plan: &TasksFile, workdir: &Path, probe: &Probe) -> Vec<Unrunnable> {
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

/// A task none of whose checks can see its work (bug-27f00b): the base
/// settles each of its verify steps, and each passes there, so they pass
/// whatever the task does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Unchecked {
    /// The task's id.
    pub task: String,
    /// The files its steps read.
    pub reads: Vec<String>,
}

/// The tasks of `plan` none of whose checks can see their work, in plan
/// order.
pub(super) fn unchecked(plan: &TasksFile, workdir: &Path, probe: &Probe) -> Vec<Unchecked> {
    let settled = settled_steps_with(plan, workdir, |command| {
        probe.settled_passes(command, workdir)
    });
    plan.tasks
        .iter()
        .filter(|task| !task.verify.is_empty())
        .filter_map(|task| {
            let steps: Vec<&SettledStep> = settled
                .iter()
                .filter(|settled| settled.task == task.id)
                .collect();
            let blind = steps.len() == task.verify.len()
                && steps.iter().all(|settled| settled.passes == Some(true));
            blind.then(|| {
                let mut reads: Vec<String> = steps
                    .iter()
                    .flat_map(|settled| settled.reads.iter().cloned())
                    .collect();
                reads.sort();
                reads.dedup();
                Unchecked {
                    task: task.id.clone(),
                    reads,
                }
            })
        })
        .collect()
}

/// A verify step the base settles (bug-27f00b): it only reads files, none
/// of which a task of its plan writes, so whatever the tasks do, it comes out
/// as it does on the base.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettledStep {
    /// Its task's id.
    pub task: String,
    /// Its number among its task's verify steps, from 1.
    pub step: usize,
    /// The workspace files it reads.
    pub reads: Vec<String>,
    /// Whether it passes on the base; `None` when it did not finish there.
    pub passes: Option<bool>,
}

impl SettledStep {
    /// How the step comes out, and why, said after the step: "fails on the
    /// base, and no task can make it pass: it reads only `notes.md`, which no
    /// task of this plan writes".
    #[must_use]
    pub fn outcome(&self) -> String {
        let only = only_reads(&self.reads);
        match self.passes {
            Some(false) => format!("fails on the base, and no task can make it pass: {only}"),
            Some(true) => {
                format!("passes on the base, so it checks nothing its task does: {only}")
            }
            None => format!("is decided by the base before any task runs: {only}"),
        }
    }

    /// [`Self::outcome`] as a reason: "it fails on the base, …".
    #[must_use]
    pub fn why(&self) -> String {
        format!("it {}", self.outcome())
    }
}

/// Why the base settles a step that reads `reads`: "it reads only
/// `notes.md`, which no task of this plan writes".
#[must_use]
pub fn only_reads(reads: &[String]) -> String {
    let named: Vec<String> = reads.iter().map(|path| format!("`{path}`")).collect();
    format!(
        "it reads only {}, which no task of this plan writes",
        named.join(", ")
    )
}

/// The verify steps of `plan` the base settles (bug-27f00b), in plan order.
/// A step is settled when it only reads files ([`read_only_files`]) that
/// exist in `workdir`, that git does not ignore (a build output is no file
/// of the base) and that no task of the plan writes ([`settled_reads`]), and
/// every step before it in its task only reads files too; a task that pins
/// an acceptance test, whose run comes first, has none. Each runs once in
/// `workdir`, read-only, as `plan run` runs a check, to tell how it comes
/// out.
#[must_use]
pub fn settled_steps(plan: &TasksFile, workdir: &Path) -> Vec<SettledStep> {
    settled_steps_with(plan, workdir, |command| run_settled(command, workdir))
}

/// [`settled_steps`], with `run` telling whether a step passes.
fn settled_steps_with(
    plan: &TasksFile,
    workdir: &Path,
    run: impl Fn(&str) -> Option<bool>,
) -> Vec<SettledStep> {
    let writes = plan_writes(plan);
    let mut found = Vec::new();
    for task in &plan.tasks {
        if task
            .accept
            .as_ref()
            .is_some_and(|accept| !accept.files.is_empty())
        {
            continue;
        }
        for (number, step) in (1..).zip(&task.verify) {
            // A step after one that does more than read may read what that
            // one wrote.
            if read_only_files(&step.command).is_none() {
                break;
            }
            if let Some(reads) = settled_reads(&step.command, workdir, &writes) {
                found.push(SettledStep {
                    task: task.id.clone(),
                    step: number,
                    reads,
                    passes: run(&step.command),
                });
            }
        }
    }
    found
}

/// Whether a step that only reads files passes when it runs once in
/// `workdir`, as `plan run` runs a check; `None` when it did not finish
/// within [`SETTLED_TIMEOUT`], could not start, or a program it names is
/// missing.
fn run_settled(command: &str, workdir: &Path) -> Option<bool> {
    let result = crate::spec_red_on_base::run_step(
        command,
        workdir,
        SETTLED_TIMEOUT,
        &workdir.join("target"),
    )
    .ok()?;
    match result.exit? {
        0 => Some(true),
        127 => None,
        code => (code > 0).then_some(false),
    }
}

/// The files a verify step reads when the base settles it (bug-27f00b): it
/// only reads files ([`read_only_files`]), at least one, and each of them
/// exists in `root`, is not ignored by git there, and is not one that
/// `writes` (what the plan's tasks write) may change. `None` otherwise.
#[must_use]
pub fn settled_reads(command: &str, root: &Path, writes: &[String]) -> Option<Vec<String>> {
    let reads = read_only_files(command)?;
    let settled = !reads.is_empty()
        && reads.iter().all(|path| {
            !is_roko_state(path)
                && !may_write(writes, path)
                && root.join(path).exists()
                && !git_ignores(root, path)
        });
    settled.then_some(reads)
}

/// Whether `path` is roko's own state, which [`roko_state_read`] reports
/// apart.
fn is_roko_state(path: &str) -> bool {
    path == ".roko" || path.starts_with(".roko/")
}

/// The first path under `.roko/`, roko's own state, that `command` reads
/// (bug-27f00b). No task writes it and roko changes it as it runs, so a
/// check of it checks no task's work.
#[must_use]
pub fn roko_state_read(command: &str, workdir: &Path) -> Option<String> {
    step_files(command, workdir)
        .into_iter()
        .chain(read_only_files(command).unwrap_or_default())
        .find(|path| is_roko_state(path))
}

/// What the tasks of `plan` may write: their `files`, and the pinned tests
/// the run copies into place. A task whose role may write files and that
/// names none may write anything, which `.` stands for.
fn plan_writes(plan: &TasksFile) -> Vec<String> {
    let mut writes = Vec::new();
    for task in &plan.tasks {
        writes.extend(task.files.iter().cloned());
        if let Some(accept) = &task.accept {
            writes.extend(accept.files.iter().map(|entry| entry.dest.clone()));
        }
        let role = task.role.as_deref().unwrap_or("implementer");
        if task.files.is_empty() && crate::task_parser::role_capabilities(role).write {
            writes.push(".".to_string());
        }
    }
    writes
}

/// Whether writing `writes` can change what reading `path` shows: one of
/// them is `path`, a file under it, or a directory it is in. A glob stands
/// for its directory, and a path outside the workspace for all of it.
fn may_write(writes: &[String], path: &str) -> bool {
    writes.iter().any(|written| {
        let literal = written
            .find(['*', '?', '['])
            .map_or(written.as_str(), |glob| {
                written[..glob].rsplit_once('/').map_or(".", |(dir, _)| dir)
            });
        let Some(written) = inside_workspace(literal) else {
            return true;
        };
        written == "."
            || path == "."
            || written == path
            || path
                .strip_prefix(written.as_str())
                .is_some_and(|rest| rest.starts_with('/'))
            || written
                .strip_prefix(path)
                .is_some_and(|rest| rest.starts_with('/'))
    })
}

/// Whether git ignores `path` in the checkout at `root`; false outside git.
fn git_ignores(root: &Path, path: &str) -> bool {
    let mut git = std::process::Command::new("git");
    git.args(["check-ignore", "--quiet", "--", path])
        .current_dir(root)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    for name in crate::spec_red_on_base::GIT_REDIRECTS {
        git.env_remove(name);
    }
    git.status().is_ok_and(|status| status.success())
}

/// The workspace files `command` reads, when all it does is read files
/// (bug-27f00b): each of its commands runs `grep` (or `egrep`, `fgrep`),
/// `test` or `[` on one file (`-f path`), `cat`, `head`, `tail` or `wc`,
/// maybe negated with `!`, and they are joined by `&&`, `||`, `|` or `;`.
/// Paths come back relative to the workspace, without `./`. `None` when it
/// runs anything else, or when the shell could make it read other files or
/// write one: a variable or a substitution, a glob, a group or subshell, a
/// redirection to a file other than `/dev/null`, a path outside the
/// workspace, a `tail -f`, or a recursive `grep` of no named path.
#[must_use]
pub fn read_only_files(command: &str) -> Option<Vec<String>> {
    let mut reads = Vec::new();
    for (words, inputs) in plain_commands(command)? {
        let words: Vec<&str> = words
            .iter()
            .map(String::as_str)
            .skip_while(|word| *word == "!")
            .collect();
        let files = command_reads(&words)?;
        reads.extend(files);
        reads.extend(inputs);
    }
    let mut reads = reads
        .iter()
        .map(|path| inside_workspace(path))
        .collect::<Option<Vec<String>>>()?;
    reads.sort();
    reads.dedup();
    Some(reads)
}

/// The files one plain command reads, when it is a `grep`, a file `test`,
/// `cat`, `head`, `tail` or `wc` that only reads; `None` for any other.
fn command_reads(words: &[&str]) -> Option<Vec<String>> {
    let (program, args) = words.split_first()?;
    Some(match *program {
        "grep" | "egrep" | "fgrep" => grep_files(args)?,
        "test" => file_test(args)?,
        "[" => file_test(args.strip_suffix(&["]"])?)?,
        "cat" | "wc" => operand_files(args, &[])?,
        "head" => operand_files(args, &["-n", "-c", "--lines", "--bytes"])?,
        "tail" => {
            let follows = args.iter().any(|arg| {
                arg.starts_with("--follow")
                    || (arg.starts_with('-') && !arg.starts_with("--") && arg.contains(['f', 'F']))
            });
            if follows {
                return None;
            }
            operand_files(args, &["-n", "-c", "--lines", "--bytes"])?
        }
        _ => return None,
    })
}

/// The workspace files `command` cannot pass without, when `&&` alone joins
/// its commands, so that a missing file fails it (round v7, phase-0 finding
/// #2): what a `grep`, a file `test`, `cat`, `head`, `tail` or `wc` among them
/// reads, the files `<` gives them, and each program it runs by its path. A
/// negated command (`! grep …`, `test ! -f …`) needs nothing. `None` for a
/// command that is not such a chain.
pub(super) fn needed_files(command: &str) -> Option<Vec<String>> {
    let line = plain_line(command)?;
    if line.other_join {
        return None;
    }
    let mut needed = Vec::new();
    for (words, inputs) in &line.commands {
        let words: Vec<&str> = words.iter().map(String::as_str).collect();
        if words.contains(&"!") {
            continue;
        }
        if let Some(program) = words.first().filter(|program| program.contains('/')) {
            needed.push((*program).to_string());
        }
        needed.extend(command_reads(&words).unwrap_or_default());
        needed.extend(inputs.iter().cloned());
    }
    let mut needed: Vec<String> = needed
        .iter()
        .filter_map(|path| inside_workspace(path))
        .collect();
    needed.sort();
    needed.dedup();
    Some(needed)
}

/// A verify step that needs a file the workspace lacks and that only tasks
/// its own task does not wait for write (round v7, phase-0 finding #2): it
/// cannot pass when its task ends.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaterFile {
    /// The step's task.
    pub task: String,
    /// The step's number among its task's verify steps, from 1.
    pub step: usize,
    /// The file it needs.
    pub path: String,
    /// The tasks that write the file.
    pub writers: Vec<String>,
    /// Whether every writer waits for the step's task, so runs after it;
    /// otherwise a writer may run alongside it.
    pub after: bool,
    /// How the step needs a file it does not name: its test includes it
    /// (`its test includes \`ui/UiFrame.h\``), or uses the type it declares
    /// (`its test uses \`UiFrame\``). `None` when the step names the file.
    pub how: Option<String>,
    /// Whether the need is a guess: the step's test uses the name of the
    /// type the file declares, by the file's name. A guess is a warning, and
    /// never sends a plan back.
    pub guessed: bool,
}

impl LaterFile {
    /// Why the step cannot pass, for a person or the planner.
    #[must_use]
    pub fn why(&self) -> String {
        let writers = self.writers.join(", ");
        let how = self
            .how
            .as_ref()
            .map_or_else(String::new, |how| format!(" ({how})"));
        let (writes, runs) = if self.writers.len() > 1 {
            ("write", "run")
        } else {
            ("writes", "runs")
        };
        if self.after {
            format!(
                "it needs `{}`{how}, which does not exist until {writers} {writes} it, and \
                 {writers} {runs} after {}: check it in {writers}, or have {} write it",
                self.path, self.task, self.task
            )
        } else {
            format!(
                "it needs `{}`{how}, which does not exist until {writers} {writes} it, and \
                 {writers} may run alongside {}: have {} wait for {writers}, or check it there",
                self.path, self.task, self.task
            )
        }
    }
}

/// The verify steps of `plan` that need a file `root` lacks and that only
/// tasks their own task does not wait for write: such a step cannot pass when
/// its task ends. A step needs a file it names ([`needed_files`]); in a CMake
/// workspace, also one its C or C++ test includes, or whose type the test uses
/// ([`TestCode`]). A plan imported from mori checked that each widget file
/// existed in the task that wrote their module list, before the tasks that
/// wrote the widgets; Undertow's first phase-4 spec gave its plumbing task a
/// test whose helpers used `UiFrame`, which only the next task's
/// `plugin/ui/UiFrame.h` declares.
#[must_use]
pub fn later_files(plan: &TasksFile, root: &Path) -> Vec<LaterFile> {
    let mut code = TestCode::of(root);
    let mut found = Vec::new();
    for (index, task) in plan.tasks.iter().enumerate() {
        let written = written_before_and_by(plan, index);
        for (number, step) in (1..).zip(&task.verify) {
            let mut needs: Vec<(String, Option<(String, bool)>)> = needed_files(&step.command)
                .unwrap_or_default()
                .into_iter()
                .map(|path| (path, None))
                .collect();
            if let Some(code) = code.as_mut()
                && let Some(compiled) = code.compiled_by(&step.command)
            {
                let later = plan
                    .tasks
                    .iter()
                    .filter(|other| other.id != task.id)
                    .flat_map(|other| &other.files)
                    .filter(|file| !root.join(file).exists() && !is_written(&written, file));
                for file in later {
                    if needs.iter().any(|(path, _)| path == file) {
                        continue;
                    }
                    if let Some(how) = code.needs(&compiled, file) {
                        needs.push((file.clone(), Some(how)));
                    }
                }
            }
            for (path, how) in needs {
                if root.join(&path).exists() || is_written(&written, &path) {
                    continue;
                }
                let writers: Vec<&TaskDef> = plan
                    .tasks
                    .iter()
                    .filter(|other| other.id != task.id && is_written(&other.files, &path))
                    .collect();
                if writers.is_empty() {
                    continue;
                }
                let after = writers
                    .iter()
                    .all(|writer| waits_for(plan, &writer.id, &task.id));
                let (how, guessed) =
                    how.map_or((None, false), |(how, guessed)| (Some(how), guessed));
                found.push(LaterFile {
                    task: task.id.clone(),
                    step: number,
                    path,
                    writers: writers.iter().map(|writer| writer.id.clone()).collect(),
                    after,
                    how,
                    guessed,
                });
            }
        }
    }
    found
}

/// What a step's tests compile ([`TestCode::compiled_by`]).
#[derive(Debug, Default)]
struct Compiled {
    /// The `#include "…"` paths no workspace file answers.
    includes: Vec<String>,
    /// The identifiers the code uses.
    words: std::collections::HashSet<String>,
}

/// The most C and C++ files [`TestCode`] reads of a workspace.
const TEST_CODE_FILES: usize = 4000;

/// How deep [`TestCode`] follows a test's `#include "…"` into the workspace.
const INCLUDE_DEPTH: usize = 3;

/// The C and C++ code of a CMake workspace that a step's tests compile
/// (round v7, phase-0 finding #2): for each test, the files it includes and
/// the identifiers its code uses, comments and strings aside, through the
/// workspace headers it includes; and the identifiers its non-test code
/// already uses. Read once per plan, as steps ask.
#[derive(Debug, Default)]
struct TestCode {
    /// The workspace's C and C++ files, relative, outside build output and
    /// vendored code.
    files: Vec<String>,
    /// The identifiers of the files that are not tests, when read.
    production: Option<std::collections::HashSet<String>>,
    /// By file: the includes it names and the identifiers it uses.
    read: HashMap<String, (Vec<String>, std::collections::HashSet<String>)>,
    root: std::path::PathBuf,
}

impl TestCode {
    /// The workspace's C and C++ code, when it is a CMake project.
    fn of(root: &Path) -> Option<Self> {
        if !holds_cmake_lists(root, 2) {
            return None;
        }
        let mut files = Vec::new();
        let mut pending = vec![std::path::PathBuf::new()];
        while let Some(dir) = pending.pop() {
            let Ok(entries) = std::fs::read_dir(root.join(&dir)) else {
                continue;
            };
            for entry in entries.filter_map(Result::ok) {
                let name = entry.file_name().to_string_lossy().into_owned();
                let path = dir.join(&name);
                if entry.file_type().is_ok_and(|kind| kind.is_dir()) {
                    if !name.starts_with('.') && !NOT_THE_PROJECT.contains(&name.as_str()) {
                        pending.push(path);
                    }
                } else if is_c_family(&name) && files.len() < TEST_CODE_FILES {
                    files.push(path.to_string_lossy().replace('\\', "/"));
                }
            }
        }
        Some(Self {
            files,
            root: root.to_path_buf(),
            ..Self::default()
        })
    }

    /// The code the C or C++ tests `command` runs compile: the includes no
    /// workspace file answers, and the identifiers it uses, through the
    /// workspace headers it includes. `None` when it runs no such test.
    fn compiled_by(&mut self, command: &str) -> Option<Compiled> {
        let tests = self.tests_of(command);
        if tests.is_empty() {
            return None;
        }
        let mut includes = Vec::new();
        let mut words = std::collections::HashSet::new();
        let mut pending: Vec<(String, usize)> = tests.into_iter().map(|test| (test, 0)).collect();
        let mut seen = Vec::new();
        while let Some((path, depth)) = pending.pop() {
            if seen.contains(&path) {
                continue;
            }
            seen.push(path.clone());
            let (named, used) = self.read(&path);
            words.extend(used);
            for include in named {
                let found = self.resolve(&path, &include);
                if found.is_empty() {
                    includes.push(include);
                } else if depth < INCLUDE_DEPTH {
                    pending.extend(found.into_iter().map(|found| (found, depth + 1)));
                }
            }
        }
        Some(Compiled { includes, words })
    }

    /// How `compiled`, a step's test code, needs `file`, which a later task
    /// writes, and whether that is a guess: it includes it, or, a guess, uses
    /// the type its name declares (`UiFrame` for `plugin/ui/UiFrame.h`) that
    /// no non-test code uses yet.
    fn needs(&mut self, compiled: &Compiled, file: &str) -> Option<(String, bool)> {
        if let Some(include) = compiled
            .includes
            .iter()
            .find(|include| file == include.as_str() || file.ends_with(&format!("/{include}")))
        {
            return Some((format!("its test includes `{include}`"), false));
        }
        let stem = Path::new(file).file_stem()?.to_str()?;
        let type_like = stem.len() >= 4
            && stem.starts_with(|c: char| c.is_ascii_uppercase())
            && stem.contains(|c: char| c.is_ascii_lowercase());
        (type_like && compiled.words.contains(stem) && !self.production().contains(stem))
            .then(|| (format!("its test uses `{stem}`"), true))
    }

    /// The C and C++ test files `command` runs: one it names by path, or one
    /// named for a word of it (`state` gives `tests/spec/p4/state_test.cpp`).
    fn tests_of(&self, command: &str) -> Vec<String> {
        let words: Vec<String> = shell_segments(command)
            .iter()
            .flat_map(|segment| command_words(segment).into_iter().skip(1))
            .collect();
        let mut tests = Vec::new();
        for word in &words {
            let path = word.trim_start_matches("./");
            if is_c_family(path) && self.files.iter().any(|file| file == path) {
                tests.push(path.to_string());
            }
            if word.len() < 3 || !word.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
                continue;
            }
            for file in &self.files {
                let stem = Path::new(file)
                    .file_stem()
                    .and_then(|stem| stem.to_str())
                    .unwrap_or_default();
                let named = [
                    format!("{word}_test"),
                    format!("test_{word}"),
                    format!("{word}_tests"),
                ];
                if named.iter().any(|name| name == stem) && !tests.contains(file) {
                    tests.push(file.clone());
                }
            }
        }
        tests
    }

    /// The workspace files an `#include "include"` in `from` can name: the
    /// one beside it, else each whose path ends with it.
    fn resolve(&self, from: &str, include: &str) -> Vec<String> {
        let beside = Path::new(from)
            .parent()
            .map(|dir| dir.join(include).to_string_lossy().replace('\\', "/"));
        if let Some(beside) = beside.filter(|beside| self.files.contains(beside)) {
            return vec![beside];
        }
        self.files
            .iter()
            .filter(|file| file.as_str() == include || file.ends_with(&format!("/{include}")))
            .cloned()
            .collect()
    }

    /// The `#include "…"` paths and the identifiers of `path`, read once.
    fn read(&mut self, path: &str) -> (Vec<String>, std::collections::HashSet<String>) {
        if let Some(read) = self.read.get(path) {
            return read.clone();
        }
        let text = std::fs::read_to_string(self.root.join(path)).unwrap_or_default();
        let read = (quoted_includes(&text), code_identifiers(&text));
        self.read.insert(path.to_string(), read.clone());
        read
    }

    /// The identifiers the workspace's non-test C and C++ code uses.
    fn production(&mut self) -> &std::collections::HashSet<String> {
        if self.production.is_none() {
            let mut words = std::collections::HashSet::new();
            for file in self.files.iter().filter(|file| !in_test_tree(file)) {
                let text = std::fs::read_to_string(self.root.join(file)).unwrap_or_default();
                words.extend(code_identifiers(&text));
            }
            self.production = Some(words);
        }
        self.production.get_or_insert_with(Default::default)
    }
}

/// Whether `name` is a C or C++ source or header.
fn is_c_family(name: &str) -> bool {
    Path::new(name)
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            CMAKE_SOURCES.contains(&extension) || C_HEADERS.contains(&extension)
        })
}

/// Whether the C or C++ file `path` is test code: under a `test` or `tests`
/// directory, or named as a test. ([`is_test_path`] knows only the
/// languages whose tests a check names by file.)
fn in_test_tree(path: &str) -> bool {
    let stem = Path::new(path)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or_default();
    path.split('/')
        .any(|part| part == "test" || part == "tests")
        || stem.ends_with("_test")
        || stem.starts_with("test_")
}

/// The paths of the `#include "…"` lines of C or C++ `text`.
fn quoted_includes(text: &str) -> Vec<String> {
    static INCLUDE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r#"(?m)^\s*#\s*include\s*"([^"]+)""#).expect("valid include regex")
    });
    INCLUDE
        .captures_iter(text)
        .filter_map(|captures| captures.get(1))
        .map(|path| path.as_str().to_string())
        .collect()
}

/// The identifiers of C or C++ `text`, outside comments, string literals,
/// character literals and preprocessor lines.
fn code_identifiers(text: &str) -> std::collections::HashSet<String> {
    let mut words = std::collections::HashSet::new();
    let mut word = String::new();
    let mut chars = text.chars().peekable();
    let mut line_start = true;
    while let Some(c) = chars.next() {
        let end_word = |word: &mut String, words: &mut std::collections::HashSet<String>| {
            if !word.is_empty() {
                words.insert(std::mem::take(word));
            }
        };
        match c {
            '/' if chars.peek() == Some(&'/') => {
                end_word(&mut word, &mut words);
                for next in chars.by_ref() {
                    if next == '\n' {
                        break;
                    }
                }
                line_start = true;
                continue;
            }
            '/' if chars.peek() == Some(&'*') => {
                end_word(&mut word, &mut words);
                chars.next();
                let mut star = false;
                for next in chars.by_ref() {
                    if star && next == '/' {
                        break;
                    }
                    star = next == '*';
                }
                continue;
            }
            '#' if line_start => {
                end_word(&mut word, &mut words);
                for next in chars.by_ref() {
                    if next == '\n' {
                        break;
                    }
                }
                line_start = true;
                continue;
            }
            '"' | '\'' => {
                end_word(&mut word, &mut words);
                let quote = c;
                while let Some(next) = chars.next() {
                    if next == '\\' {
                        chars.next();
                    } else if next == quote {
                        break;
                    }
                }
            }
            c if c.is_ascii_alphanumeric() || c == '_' => word.push(c),
            _ => end_word(&mut word, &mut words),
        }
        line_start = c == '\n' || (line_start && c.is_whitespace());
    }
    if !word.is_empty() {
        words.insert(word);
    }
    words
}

/// The simple commands of `command` as their words, each with the files a
/// `<` redirection reads into it, when the shell would run them as written:
/// joined by `&&`, `||`, `|`, `|&`, `;` or a newline, with quoted or plain
/// words. `None` when the shell would expand a word (`$`, a backtick, a
/// glob, a leading `~`), group commands (`(`, `{`, `[[`), run one in the
/// background, read a here-document, or send output to a file other than
/// `/dev/null`; and on a `#` comment or an unclosed quote.
fn plain_commands(command: &str) -> Option<Vec<(Vec<String>, Vec<String>)>> {
    plain_line(command).map(|line| line.commands)
}

/// What [`plain_commands`] reads of `command`, with whether anything but
/// `&&` joins two of its commands.
fn plain_line(command: &str) -> Option<PlainLine> {
    let mut line = PlainLine::default();
    let mut chars = command.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\'' => {
                line.started = true;
                loop {
                    match chars.next()? {
                        '\'' => break,
                        inner => line.word.push(inner),
                    }
                }
            }
            '"' => {
                line.started = true;
                loop {
                    match chars.next()? {
                        '"' => break,
                        '$' | '`' => return None,
                        '\\' => match chars.next()? {
                            '\n' => {}
                            escaped @ ('"' | '\\') => line.word.push(escaped),
                            escaped => {
                                line.word.push('\\');
                                line.word.push(escaped);
                            }
                        },
                        inner => line.word.push(inner),
                    }
                }
            }
            '\\' => match chars.next()? {
                '\n' => {}
                escaped => line.push(escaped),
            },
            ' ' | '\t' => line.end_word()?,
            '&' if chars.peek() == Some(&'>') => {
                // `&>file` sends both streams to a file.
                chars.next();
                if chars.peek() == Some(&'>') {
                    chars.next();
                }
                line.end_word()?;
                line.target = Some(Redirect::Output);
            }
            // `cmd &` runs in the background.
            '&' if chars.peek() != Some(&'&') => return None,
            '\n' | ';' | '|' | '&' => {
                // A lone `&` ended the parse above, so `&` here starts `&&`.
                let and = c == '&';
                if c != '\n' && c != ';' && matches!(chars.peek(), Some('&' | '|')) {
                    chars.next();
                }
                line.other_join |= !and;
                line.end_command()?;
            }
            '>' | '<' => {
                // Digits right before it name the descriptor it redirects.
                if line.started
                    && !line.word.is_empty()
                    && line.word.chars().all(|c| c.is_ascii_digit())
                {
                    line.word.clear();
                    line.started = false;
                } else {
                    line.end_word()?;
                }
                if line.target.is_some() {
                    return None;
                }
                if c == '<' {
                    // `<(…)`, a here-document, `<&` and `<>` read no named
                    // file.
                    if matches!(chars.peek(), Some('(' | '<' | '&' | '>')) {
                        return None;
                    }
                    line.target = Some(Redirect::Input);
                    continue;
                }
                if matches!(chars.peek(), Some('>' | '|')) {
                    chars.next();
                }
                if chars.peek() == Some(&'(') {
                    return None;
                }
                if chars.peek() == Some(&'&') {
                    chars.next();
                    if chars
                        .peek()
                        .is_some_and(|next| next.is_ascii_digit() || *next == '-')
                    {
                        // A copy of another descriptor (`2>&1`): no file.
                        while chars
                            .peek()
                            .is_some_and(|next| next.is_ascii_digit() || *next == '-')
                        {
                            chars.next();
                        }
                        continue;
                    }
                }
                line.target = Some(Redirect::Output);
            }
            '[' | ']' if !line.started && chars.peek().is_none_or(|next| next.is_whitespace()) => {
                line.push(c);
            }
            '$' | '`' | '(' | ')' | '{' | '}' | '*' | '?' | '[' | ']' => return None,
            '~' | '#' if !line.started => return None,
            _ => line.push(c),
        }
    }
    line.end_command()?;
    Some(line)
}

/// What a redirection wants as its next word.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Redirect {
    /// The file a command reads (`< file`).
    Input,
    /// The file a command writes (`> file`, `2>> file`, `&> file`).
    Output,
}

/// What [`plain_commands`] has read: the commands so far, and the command,
/// word and redirection it is reading.
#[derive(Debug, Default)]
struct PlainLine {
    commands: Vec<(Vec<String>, Vec<String>)>,
    words: Vec<String>,
    inputs: Vec<String>,
    word: String,
    /// Whether a word is being read; a quoted empty word is one.
    started: bool,
    target: Option<Redirect>,
    /// Whether anything but `&&` joins two of its commands.
    other_join: bool,
}

impl PlainLine {
    /// Add `c` to the word being read.
    fn push(&mut self, c: char) {
        self.word.push(c);
        self.started = true;
    }

    /// End the word being read: a word of the command, or the target of the
    /// redirection before it. `None` for output to a file other than
    /// `/dev/null`.
    fn end_word(&mut self) -> Option<()> {
        if !std::mem::take(&mut self.started) {
            return Some(());
        }
        let word = std::mem::take(&mut self.word);
        match self.target.take() {
            Some(Redirect::Input) => self.inputs.push(word),
            Some(Redirect::Output) => (word == "/dev/null").then_some(())?,
            None => self.words.push(word),
        }
        Some(())
    }

    /// End the command being read. `None` for a redirection without its
    /// file, or one with no command.
    fn end_command(&mut self) -> Option<()> {
        self.end_word()?;
        if self.target.is_some() {
            return None;
        }
        if !self.words.is_empty() {
            let command = (
                std::mem::take(&mut self.words),
                std::mem::take(&mut self.inputs),
            );
            self.commands.push(command);
        } else if !self.inputs.is_empty() {
            return None;
        }
        Some(())
    }
}

/// The files a `grep` reads: its operands after the pattern, and a pattern
/// file (`-f`). `None` for a recursive `grep` of no named path, which
/// searches the directory or its input depending on the `grep`.
fn grep_files(args: &[&str]) -> Option<Vec<String>> {
    let mut files = Vec::new();
    let mut operands = Vec::new();
    let mut pattern = false;
    let mut recursive = false;
    let mut only_operands = false;
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        if only_operands || !arg.starts_with('-') || *arg == "-" {
            operands.push(*arg);
            continue;
        }
        if *arg == "--" {
            only_operands = true;
            continue;
        }
        if let Some(long) = arg.strip_prefix("--") {
            let (name, value) = match long.split_once('=') {
                Some((name, value)) => (name, Some(value)),
                None => (long, None),
            };
            recursive |= matches!(name, "recursive" | "dereference-recursive");
            if GREP_LONG_VALUES.contains(&name) {
                let value = match value {
                    Some(value) => value,
                    None => args.next()?,
                };
                pattern |= matches!(name, "regexp" | "file");
                if matches!(name, "file" | "exclude-from") {
                    files.push(value.to_string());
                }
            }
            continue;
        }
        // A cluster of short options: `-qE`, `-e pattern`, `-epattern`.
        let letters = &arg[1..];
        for (index, letter) in letters.char_indices() {
            recursive |= matches!(letter, 'r' | 'R');
            if GREP_SHORT_VALUES.contains(letter) {
                let rest = &letters[index + letter.len_utf8()..];
                let value = if rest.is_empty() { *args.next()? } else { rest };
                pattern |= matches!(letter, 'e' | 'f');
                if letter == 'f' {
                    files.push(value.to_string());
                }
                break;
            }
        }
    }
    let mut operands = operands.into_iter();
    if !pattern {
        operands.next()?;
    }
    let named: Vec<String> = operands
        .filter(|operand| *operand != "-")
        .map(str::to_string)
        .collect();
    if recursive && named.is_empty() {
        return None;
    }
    files.extend(named);
    Some(files)
}

/// The file a `test` expression tests: `[!] -f path`, or another of the
/// [`FILE_TESTS`]. `None` for any other expression.
fn file_test(args: &[&str]) -> Option<Vec<String>> {
    let args = match args {
        ["!", rest @ ..] => rest,
        _ => args,
    };
    match args {
        [operator, path]
            if operator.len() == 2
                && operator
                    .strip_prefix('-')
                    .is_some_and(|letter| FILE_TESTS.contains(letter)) =>
        {
            Some(vec![(*path).to_string()])
        }
        _ => None,
    }
}

/// The files `cat`, `wc`, `head` or `tail` read: their operands, besides
/// `-` (their input). Each option in `value_options` takes the next word as
/// its value unless the value is attached.
fn operand_files(args: &[&str], value_options: &[&str]) -> Option<Vec<String>> {
    let mut files = Vec::new();
    let mut only_operands = false;
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        if only_operands || !arg.starts_with('-') || *arg == "-" {
            if *arg != "-" {
                files.push((*arg).to_string());
            }
            continue;
        }
        if *arg == "--" {
            only_operands = true;
        } else if value_options.contains(arg) {
            args.next()?;
        }
    }
    Some(files)
}

/// `path` relative to the workspace, without `.` parts and with `..` folded;
/// `.` for the workspace itself. `None` when it is empty, absolute, or
/// leaves the workspace.
fn inside_workspace(path: &str) -> Option<String> {
    if path.is_empty() || path.starts_with('/') {
        return None;
    }
    let mut parts: Vec<&str> = Vec::new();
    for part in path.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop()?;
            }
            part => parts.push(part),
        }
    }
    Some(if parts.is_empty() {
        ".".to_string()
    } else {
        parts.join("/")
    })
}

/// Why `program` with `args` cannot run in `workdir` (bug-72dc0d): it is a
/// build tool whose manifest neither the workspace (or a directory above it)
/// has nor a task writes before the step runs, such as `cargo test -p
/// undertow_core` in a CMake project. `None` when it is no such tool, or a
/// flag points it at a manifest elsewhere.
fn missing_manifest(
    program: &str,
    args: &[String],
    workdir: &Path,
    written: &[String],
) -> Option<String> {
    let (_, subcommands, manifests) = MANIFESTS.iter().find(|(tool, _, _)| *tool == program)?;
    let elsewhere = ["--manifest-path", "--prefix", "--dir", "--cwd", "-C"];
    if args.iter().any(|arg| {
        elsewhere
            .iter()
            .any(|flag| arg == flag || arg.starts_with(&format!("{flag}=")))
    }) {
        return None;
    }
    let subcommand = args
        .iter()
        .find(|arg| !arg.starts_with('-') && !arg.starts_with('+'))?;
    // `go run main.go` builds the files it is given, with no module.
    if !subcommands.contains(&subcommand.as_str())
        || (program == "go" && args.iter().any(|arg| arg.ends_with(".go")))
    {
        return None;
    }
    let present = workdir
        .ancestors()
        .any(|dir| manifests.iter().any(|name| dir.join(name).is_file()));
    let comes = manifests.iter().any(|name| is_written(written, name));
    (!present && !comes).then(|| {
        format!(
            "it runs `{program} {subcommand}`, which needs a `{}`: this workspace has none, and \
             no task writes one before it runs",
            manifests[0]
        )
    })
}

/// Why `command` cannot pass, if it cannot: the first problem found.
fn step_problem(
    command: &str,
    workdir: &Path,
    written: &[String],
    probe: &Probe,
) -> Option<String> {
    // After a `cd`, a command runs in a directory this check does not follow.
    let mut moved = false;
    for segment in shell_segments(command) {
        let words = command_words(&segment);
        let Some((program, args)) = words.split_first() else {
            continue;
        };
        let program = program.as_str();
        moved |= matches!(program, "cd" | "pushd");
        if BUILTINS.contains(&program) || program.contains('/') {
            continue;
        }
        if !probe.command(program) {
            return Some(format!("`{program}` is not installed"));
        }
        if !moved && let Some(why) = missing_manifest(program, args, workdir, written) {
            return Some(why);
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
    if let Some(path) = roko_state_read(command, workdir) {
        return Some(format!(
            "it reads `{path}`, roko's own state, which no task writes and roko changes as it runs"
        ));
    }
    // A missing file, or a test its file lacks, fails the step only when
    // nothing but `&&` joins its commands as the shell reads them: after
    // `||`, a pipe or a `;` it may pass all the same (gap-ff78e1).
    if !plain_line(command).is_some_and(|line| !line.other_join) {
        return None;
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
pub(super) fn command_words(segment: &str) -> Vec<String> {
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
/// runs; up to its first `cd`, and none of a negated command.
fn step_files(command: &str, workdir: &Path) -> Vec<String> {
    let mut files = Vec::new();
    for segment in shell_segments(command) {
        let words = command_words(&segment);
        let Some((program, args)) = words.split_first() else {
            continue;
        };
        // After a `cd` a path is another directory's (gap-ff78e1), and a
        // negated command (`test ! -e x`, `! grep …`) needs no file.
        if matches!(program.as_str(), "cd" | "pushd") {
            break;
        }
        if program == "!" || args.iter().any(|arg| arg == "!") {
            continue;
        }
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
        // After a `cd`, a test's path is another directory's.
        if matches!(program.as_str(), "cd" | "pushd") {
            break;
        }
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
        && !arg.contains(['*', '?', '$', '{', '<', '>', '(', ')'])
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
        std::fs::write(temp.path().join("Cargo.toml"), "[workspace]\n").expect("a cargo workspace");
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
            "T4: depends on T1, which writes `synth/osc.py` it uses"
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

    /// bug-df9aff: pocket-synth's render tests import only `synth.render`.
    /// The render task still waits for the modules whose classes its
    /// description names, as the other tasks' tests import them from there.
    #[test]
    fn a_task_naming_another_modules_classes_waits_for_it() {
        let (temp, mut plan) = g1_plan();
        std::fs::write(
            temp.path().join("tests/test_render.py"),
            "import unittest\nfrom synth.render import render\n",
        )
        .expect("the render tests");
        plan.tasks[3].description =
            Some("Add render() that builds the patch from Oscillator, SVF and ADSR.".to_string());
        let waits = repair_dependencies(&mut plan, temp.path());
        assert_eq!(plan.tasks[3].depends_on, ["T1", "T2", "T3"], "{waits:?}");
        assert!(
            plan.tasks[..3]
                .iter()
                .all(|task| task.depends_on.is_empty())
        );
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

    /// The workspace of round v7's no-spec baseline (bug-27f00b): a README
    /// and the hello-world demo plan `hermes roko init` adds. Returns it and
    /// the plan the planner wrote there, which is not in it yet.
    pub(crate) fn no_spec_baseline() -> (tempfile::TempDir, String) {
        let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/no_spec_baseline");
        let temp = tempfile::tempdir().expect("tempdir");
        let demo = Path::new("plans/demos/demo-hello-world");
        std::fs::create_dir_all(temp.path().join(demo)).expect("the demo plan's directory");
        for file in [
            Path::new("README.md"),
            &demo.join("plan.md"),
            &demo.join("tasks.toml"),
        ] {
            std::fs::copy(fixture.join(file), temp.path().join(file)).expect("a fixture file");
        }
        let plan = std::fs::read_to_string(
            fixture.join("plans/create-a-roko-plan-for-a-macos-vst3-synthesizer/tasks.toml"),
        )
        .expect("the baseline plan");
        (temp, plan)
    }

    /// bug-27f00b: as generation sees the baseline, each of its 11 checks
    /// is a step that cannot pass. Grepping phrases the demo has instead,
    /// T1's checks pass on the base whatever T1 does: no check of it can see
    /// its work.
    #[test]
    fn the_baselines_settled_checks_cannot_pass_or_see_nothing() {
        let (temp, baseline) = no_spec_baseline();
        let probe = Probe::with(&[("grep", true)], &[]);
        let plan = TasksFile::parse_str(&baseline).expect("the baseline plan");
        let found = problems(&plan, temp.path(), &probe);
        assert_eq!(found.len(), 11, "{found:?}");
        assert_eq!(
            found[0].why,
            "it fails on the base, and no task can make it pass: it reads only \
             `plans/demos/demo-hello-world/plan.md`, which no task of this plan writes"
        );
        assert_eq!(unchecked(&plan, temp.path(), &probe), []);

        let passing = baseline
            .replace("'product definition'", "'hello world'")
            .replace(
                "grep -q 'open_questions = \\[\\]' plans/demos/demo-hello-world/tasks.toml",
                "grep -q 'HELLO-1' plans/demos/demo-hello-world/tasks.toml",
            );
        let plan = TasksFile::parse_str(&passing).expect("the plan");
        assert_eq!(
            unchecked(&plan, temp.path(), &probe),
            [Unchecked {
                task: "T1".to_string(),
                reads: vec![
                    "plans/demos/demo-hello-world/plan.md".to_string(),
                    "plans/demos/demo-hello-world/tasks.toml".to_string(),
                ],
            }]
        );
        assert_eq!(problems(&plan, temp.path(), &probe).len(), 9);
    }

    /// bug-27f00b: `plan validate` runs each check the base settles. The
    /// baseline's 11 fail, so each is a PLAN_054 error and the plan does not
    /// run; a check that passes there checks nothing its task does, a
    /// PLAN_055 warning. A check of a file a task writes is neither.
    #[test]
    fn plan_validate_reports_checks_the_base_settles() {
        let (temp, baseline) = no_spec_baseline();
        let plan_dir = temp
            .path()
            .join("plans/create-a-roko-plan-for-a-macos-vst3-synthesizer");
        std::fs::create_dir_all(&plan_dir).expect("the plan directory");
        std::fs::write(plan_dir.join("tasks.toml"), &baseline).expect("tasks.toml");
        let report = crate::plan_validate::validate_plans_dir_with_workdir(
            &plan_dir,
            None,
            Some(temp.path()),
        )
        .expect("validate");
        let found = |rule: &str| {
            report.plans[0]
                .diagnostics
                .iter()
                .filter(|diagnostic| diagnostic.rule_id == rule)
                .map(|diagnostic| diagnostic.message.clone())
                .collect::<Vec<_>>()
        };
        let never = found("PLAN_054");
        assert_eq!(never.len(), 11, "{:?}", report.plans[0].diagnostics);
        assert!(
            never.contains(
                &"task 'T1' verify step 1 fails on the base, and no task can make it pass: it \
                  reads only `plans/demos/demo-hello-world/plan.md`, which no task of this plan \
                  writes"
                    .to_string()
            ),
            "{never:?}"
        );
        assert_eq!(report.exit_code(false), 1, "the plan cannot run");

        // The same checks of phrases the demo has pass there, and check
        // nothing; a scribe that writes the file it checks is fine.
        let passing = baseline
            .replace("'product definition'", "'hello world'")
            .replace(
                "files = []\ngoal = \"The plan reads as one coherent roadmap",
                "files = [\"docs/roadmap.md\"]\ngoal = \"The plan reads as one coherent roadmap",
            )
            .replace(
                "grep -q 'milestone roadmap' plans/demos/demo-hello-world/plan.md",
                "grep -q 'milestone roadmap' docs/roadmap.md",
            );
        std::fs::write(plan_dir.join("tasks.toml"), &passing).expect("tasks.toml");
        let report = crate::plan_validate::validate_plans_dir_with_workdir(
            &plan_dir,
            None,
            Some(temp.path()),
        )
        .expect("validate");
        let rules: Vec<(&str, Option<&str>)> = report.plans[0]
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.rule_id.starts_with("PLAN_05"))
            .map(|diagnostic| (diagnostic.rule_id.as_str(), diagnostic.task_id.as_deref()))
            .collect();
        assert!(rules.contains(&("PLAN_055", Some("T1"))), "{rules:?}");
        assert_eq!(
            rules.iter().filter(|(rule, _)| *rule == "PLAN_054").count(),
            9,
            "T1's first step passes now, and T5's other step reads its own file: {rules:?}"
        );
    }

    /// gap-ff78e1: `plan validate` warns about a step that cannot pass as
    /// written, as generation finds it, for a reason that holds on any
    /// machine (PLAN_059): a build tool without its manifest, a test its file
    /// does not define, a file that does not exist and no task writes. It asks
    /// no machine, so a runner this one lacks is no finding there; asked, as
    /// `--dynamic` asks, it is a PLAN_060 warning. A step another rule names
    /// is left to it: a later task's file is PLAN_058's, roko's own state
    /// PLAN_056's. A path after a `cd`, a negated test and a step that `||`
    /// joins need no file (the portal plans' steps).
    #[test]
    fn plan_validate_warns_about_steps_that_cannot_pass() {
        let (temp, _) = fixture("true", &[]);
        let plan_dir = temp.path().join("plans/p");
        std::fs::create_dir_all(&plan_dir).expect("the plan directory");
        let step =
            |command: &str| format!("[[task.verify]]\nphase = \"test\"\ncommand = {command:?}\n");
        let toml = [
            "[meta]\nplan = \"p\"\ntotal = 2\nstatus = \"ready\"\n\n\
             [[task]]\nid = \"T1\"\ntitle = \"t\"\nstatus = \"ready\"\nrole = \"implementer\"\n\
             tier = \"focused\"\nfiles = [\"synth/osc.py\"]\n"
                .to_string(),
            step("roko-no-such-runner check synth"),
            step("python3 -m unittest tests.test_osc.OscillatorTest.test_lowpass_low_gain"),
            step("grep -q seq docs/notes.md"),
            step("grep -q Filter synth/filter.py"),
            step("grep -q done .roko/state/graph/p/checkpoint.json"),
            step("cargo test -p synth"),
            step("cd apps/portal && test -f out/index.html"),
            step("test ! -e api/safe-api.ts"),
            step("grep -q seq docs/notes.md || true"),
            "[[task]]\nid = \"T2\"\ntitle = \"t\"\nstatus = \"ready\"\nrole = \"implementer\"\n\
             tier = \"focused\"\nfiles = [\"synth/filter.py\"]\ndepends_on = [\"T1\"]\n"
                .to_string(),
            step("grep -q Filter synth/filter.py"),
        ]
        .join("\n");
        std::fs::write(plan_dir.join("tasks.toml"), toml).expect("tasks.toml");
        let warning = crate::plan_validate::Severity::Warning;
        let found = |report: &crate::plan_validate::ValidationReport, rule: &str| {
            report.plans[0]
                .diagnostics
                .iter()
                .filter(|diagnostic| diagnostic.rule_id == rule)
                .map(|diagnostic| (diagnostic.severity, diagnostic.message.clone()))
                .collect::<Vec<_>>()
        };
        let portable = [
            "task 'T1' verify step 2 cannot pass as written: it names `test_lowpass_low_gain`, \
             which `tests/test_osc.py` does not define and no task writes",
            "task 'T1' verify step 3 cannot pass as written: it needs `docs/notes.md`, which does \
             not exist and no task writes before it runs",
            "task 'T1' verify step 6 cannot pass as written: it runs `cargo test`, which needs a \
             `Cargo.toml`: this workspace has none, and no task writes one before it runs",
        ]
        .map(|message| (warning, message.to_string()));

        let report = crate::plan_validate::validate_plans_dir_with_workdir(
            &plan_dir,
            None,
            Some(temp.path()),
        )
        .expect("validate");
        let diagnostics = &report.plans[0].diagnostics;
        assert_eq!(found(&report, "PLAN_059"), portable, "{diagnostics:?}");
        assert_eq!(found(&report, "PLAN_060"), [], "{diagnostics:?}");
        // Steps 4 and 5 are another rule's.
        assert_eq!(found(&report, "PLAN_058").len(), 1, "{diagnostics:?}");
        assert_eq!(found(&report, "PLAN_056").len(), 1, "{diagnostics:?}");

        let here =
            crate::plan_validate::validate_plans_dir_on_this_machine(&plan_dir, None, temp.path())
                .expect("validate on this machine");
        let diagnostics = &here.plans[0].diagnostics;
        assert_eq!(found(&here, "PLAN_059"), portable, "{diagnostics:?}");
        assert_eq!(
            found(&here, "PLAN_060"),
            [(
                warning,
                "task 'T1' verify step 1 cannot run on this machine: `roko-no-such-runner` is not \
                 installed"
                    .to_string()
            )],
            "{diagnostics:?}"
        );
    }

    /// bug-daa54e: round v7's phase-1 spec says its oscillator, filter and
    /// envelope tasks, which run in parallel, each add their source to
    /// `core/CMakeLists.txt`, and the plan left that file out of their
    /// `files`. Generation adds it to each task that creates a source under
    /// `core/` (and to the voice task, whose source must be listed too), and
    /// the generated-plan checks let the five tasks share it, parallel ones
    /// included. A `CMakeLists.txt` that globs its sources needs no entry.
    #[test]
    fn a_shared_file_the_spec_says_a_task_edits_joins_its_files() {
        let fixture =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/p1_final_check/tasks.toml");
        let mut plan = TasksFile::parse(&fixture).expect("the phase-1 plan");
        let temp = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir_all(temp.path().join("core/src")).expect("core");
        std::fs::write(
            temp.path().join("core/CMakeLists.txt"),
            "add_library(undertow_core STATIC src/params.cpp src/engine.cpp)\n",
        )
        .expect("the core list");
        for source in ["core/src/params.cpp", "core/src/engine.cpp"] {
            std::fs::write(temp.path().join(source), "// phase 0\n").expect("a source");
        }
        let repairs = repair_build_lists(&mut plan, temp.path());
        assert_eq!(repairs.len(), 4, "{repairs:?}");
        assert_eq!(
            repairs[0],
            "T1: added `core/CMakeLists.txt` to its files: it creates `core/src/oscillator.cpp`, \
             which that CMake file lists"
        );
        for task in &plan.tasks {
            assert!(
                task.files.iter().any(|file| file == "core/CMakeLists.txt"),
                "{}: {:?}",
                task.id,
                task.files
            );
        }
        let policy = crate::plan_policy::PlanExecutionPolicy::generated(8);
        let shared: Vec<String> = crate::plan_policy::validate_plan_budgets(&plan, policy)
            .into_iter()
            .filter(|issue| issue.message.contains("core/CMakeLists.txt"))
            .map(|issue| issue.to_string())
            .collect();
        assert_eq!(shared, Vec::<String>::new());
        assert!(
            repair_build_lists(&mut plan, temp.path()).is_empty(),
            "added once"
        );

        let mut plan = TasksFile::parse(&fixture).expect("the phase-1 plan");
        std::fs::write(
            temp.path().join("core/CMakeLists.txt"),
            "file(GLOB CORE_SOURCES src/*.cpp)\nadd_library(undertow_core STATIC ${CORE_SOURCES})\n",
        )
        .expect("a globbing list");
        assert_eq!(
            repair_build_lists(&mut plan, temp.path()),
            Vec::<String>::new()
        );
    }

    /// bug-d758f6: round v7's phase-0 plan for a C++/JUCE synth gave each of
    /// its four implementers `allowed_tools = ["read_file", "write_file",
    /// "grep"]` while their checks run cmake, ctest and `tools/spec.sh`.
    /// `plan validate` warns about each (PLAN_057), and generation's repair
    /// gives each its role's own tools back, which the warning then accepts.
    /// A scribe's list stays, and an implementer whose list has a shell and
    /// edit_file keeps it.
    #[test]
    fn implementers_whose_checks_run_code_get_a_shell() {
        let fixture =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/p0_tool_lists/tasks.toml");
        let text = std::fs::read_to_string(&fixture).expect("the phase-0 plan");
        let mut plan = TasksFile::parse_str(&text).expect("the phase-0 plan parses");
        let stuck = implementers_without_a_shell(&plan);
        let ids: Vec<&str> = stuck.iter().map(|(task, _)| task.as_str()).collect();
        assert_eq!(ids, ["T1", "T2", "T3", "T4"]);
        assert_eq!(
            stuck[0].1,
            "grep -q 'option(UNDERTOW_BUILD_PLUGIN' CMakeLists.txt && cmake --preset core"
        );

        let temp = tempfile::tempdir().expect("tempdir");
        let plan_dir = temp.path().join("plans/p0");
        std::fs::create_dir_all(&plan_dir).expect("the plan directory");
        std::fs::write(plan_dir.join("tasks.toml"), &text).expect("tasks.toml");
        let report = crate::plan_validate::validate_plans_dir_with_workdir(
            &plan_dir,
            None,
            Some(temp.path()),
        )
        .expect("validate");
        let warned: Vec<&str> = report.plans[0]
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.rule_id == "PLAN_057")
            .filter_map(|diagnostic| diagnostic.task_id.as_deref())
            .collect();
        assert_eq!(warned, ["T1", "T2", "T3", "T4"]);

        let repairs = repair_shell_tools(&mut plan);
        assert_eq!(repairs.len(), 4, "{repairs:?}");
        assert_eq!(
            repairs[0],
            "T1: keeps an implementer's own tools, bash and edit_file among them: its \
             allowed_tools [read_file, write_file, grep] left out bash or edit_file"
        );
        assert!(plan.tasks.iter().all(|task| task.allowed_tools.is_none()));
        assert_eq!(implementers_without_a_shell(&plan), []);

        let mut plan = TasksFile::parse_str(
            "[meta]\nplan = \"p\"\n\n\
             [[task]]\nid = \"T1\"\ntitle = \"t\"\nrole = \"implementer\"\n\
             allowed_tools = [\"read_file\", \"bash\", \"edit_file\"]\ndenied_tools = [\"write_file\"]\n\n\
             [[task]]\nid = \"T2\"\ntitle = \"notes\"\nrole = \"scribe\"\n\
             allowed_tools = [\"read_file\", \"write_file\"]\n",
        )
        .expect("a plan");
        let repairs = repair_shell_tools(&mut plan);
        assert_eq!(
            repairs,
            [
                "T1: keeps an implementer's own tools, bash and edit_file among them: its \
              denied_tools held write_file"
            ]
        );
        assert_eq!(
            plan.tasks[0].allowed_tools.as_deref(),
            Some(
                &[
                    "read_file".to_string(),
                    "bash".to_string(),
                    "edit_file".to_string()
                ][..]
            )
        );
        assert!(
            plan.tasks[0]
                .denied_tools
                .as_ref()
                .is_some_and(Vec::is_empty)
        );
        assert_eq!(
            plan.tasks[1].allowed_tools.as_deref(),
            Some(&["read_file".to_string(), "write_file".to_string()][..])
        );
    }

    /// bug-27f00b: a check of roko's own state checks no task's work, even
    /// where git ignores `.roko/` and the base cannot settle it: generation
    /// sends it back, and `plan validate` warns (PLAN_056).
    #[test]
    fn a_check_of_roko_state_is_no_tasks_check() {
        let (temp, plan) = fixture(
            "test -f .roko/state/graph/p/checkpoint.json",
            &["tests/test_osc.py"],
        );
        std::fs::create_dir_all(temp.path().join(".roko/state/graph/p")).expect("roko state");
        std::fs::write(
            temp.path().join(".roko/state/graph/p/checkpoint.json"),
            "{}",
        )
        .expect("a checkpoint");
        let probe = Probe::with(&[("grep", true)], &[]);
        assert_eq!(
            problems(&plan, temp.path(), &probe)[0].why,
            "it reads `.roko/state/graph/p/checkpoint.json`, roko's own state, which no task \
             writes and roko changes as it runs"
        );
        let plan_dir = temp.path().join("plans/p");
        std::fs::create_dir_all(&plan_dir).expect("the plan directory");
        std::fs::write(
            plan_dir.join("tasks.toml"),
            toml::to_string_pretty(&plan).expect("the plan as TOML"),
        )
        .expect("tasks.toml");
        let report = crate::plan_validate::validate_plans_dir_with_workdir(
            &plan_dir,
            None,
            Some(temp.path()),
        )
        .expect("validate");
        let rules: Vec<&str> = report.plans[0]
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.rule_id.as_str())
            .collect();
        assert!(rules.contains(&"PLAN_056"), "{rules:?}");
    }

    /// bug-27f00b: a step the base settles only reads files: `grep`, a file
    /// `test`, `cat`, `head`, `tail` and `wc`, on quoted or plain words. Any
    /// other program, an expansion, a glob, a redirection to a file or a path
    /// outside the workspace makes it more than that.
    #[test]
    fn a_step_that_only_reads_files_names_them() {
        let reads = |command: &str| read_only_files(command);
        assert_eq!(
            reads("grep -q 'open_questions = \\[\\]' plans/demos/demo-hello-world/tasks.toml"),
            Some(vec!["plans/demos/demo-hello-world/tasks.toml".to_string()])
        );
        assert_eq!(
            reads("test -f docs/spec.md && grep -qE '^- AC[0-9]+:' ./docs/spec.md"),
            Some(vec!["docs/spec.md".to_string()])
        );
        assert_eq!(
            reads("[ -d src ] && ! grep -rq TODO src 2>/dev/null"),
            Some(vec!["src".to_string()])
        );
        assert_eq!(
            reads("cat a.txt | grep -c x | wc -l; head -n 5 b.txt < c.txt"),
            Some(vec![
                "a.txt".to_string(),
                "b.txt".to_string(),
                "c.txt".to_string()
            ])
        );
        assert_eq!(
            reads("grep -e one -f patterns.txt notes.md"),
            Some(vec!["notes.md".to_string(), "patterns.txt".to_string()])
        );
        for command in [
            "cargo test -p x",
            "grep -q x $FILE",
            "grep -q x src/*.rs",
            "grep -q x \"$(cat list)\"",
            "test -f a.txt > out.txt",
            "cd docs && grep -q x spec.md",
            "grep -q x /etc/hosts",
            "grep -q x ../other/notes.md",
            "tail -f log.txt",
            "test \"$x\" = y",
            "test -n word",
            "grep -rq TODO",
            "LC_ALL=C grep -q x a.txt",
            "grep -q x a.txt &",
            "[[ -f a.txt ]]",
            "{ grep -q x a.txt; }",
            "grep -q 'unclosed a.txt",
        ] {
            assert_eq!(reads(command), None, "{command}");
        }
    }

    /// Round v7, phase-0 finding #2: a plan imported from mori (05
    /// terminal-widgets) checks, in the task that writes the widgets' module
    /// list, that a widget's file exists, though only a widget task that
    /// waits for it writes that file: the step can never pass, a PLAN_058
    /// error. A writer that may run alongside is a warning. A negated test, a
    /// chain with `||`, and a file that exists, that the task writes or that
    /// a task it waits for writes are fine.
    #[test]
    fn a_check_needing_a_later_tasks_file_is_plan_058() {
        let temp = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir_all(temp.path().join("src")).expect("src");
        std::fs::write(temp.path().join("src/main.rs"), "fn main() {}\n").expect("main");
        let task = |id: &str, files: &str, depends_on: &str, steps: &[&str]| {
            let steps: String = steps
                .iter()
                .map(|command| {
                    format!("[[task.verify]]\nphase = \"check\"\ncommand = {command:?}\n\n")
                })
                .collect();
            format!(
                "[[task]]\nid = \"{id}\"\ntitle = \"Task {id}\"\nstatus = \"ready\"\n\
                 role = \"implementer\"\ntier = \"focused\"\nfiles = [{files}]\n\
                 depends_on = [{depends_on}]\n\n{steps}"
            )
        };
        let toml = format!(
            "[meta]\nplan = \"widgets\"\ntotal = 4\nstatus = \"ready\"\n\n{}{}{}{}",
            task("T1", "\"src/palette.rs\"", "", &["test -f src/palette.rs"]),
            task(
                "T2",
                "\"src/widgets/mod.rs\"",
                "\"T1\"",
                &[
                    "test -f src/widgets/mod.rs && test -f src/palette.rs && test -f src/main.rs",
                    "test -f src/widgets/sparkline.rs",
                    "test ! -f src/widgets/gauge.rs && test -f src/widgets/mod.rs",
                    "test -f src/widgets/gauge.rs || true",
                    "grep -q Feed src/widgets/feed.rs",
                ],
            ),
            task(
                "T3",
                "\"src/widgets/sparkline.rs\", \"src/widgets/gauge.rs\"",
                "\"T2\"",
                &[],
            ),
            task("T4", "\"src/widgets/feed.rs\"", "\"T1\"", &[]),
        );
        let plan = TasksFile::parse_str(&toml).expect("a plan");
        let later = later_files(&plan, temp.path());
        let found: Vec<(&str, usize, &str, bool)> = later
            .iter()
            .map(|later| {
                (
                    later.task.as_str(),
                    later.step,
                    later.path.as_str(),
                    later.after,
                )
            })
            .collect();
        assert_eq!(
            found,
            [
                ("T2", 2, "src/widgets/sparkline.rs", true),
                ("T2", 5, "src/widgets/feed.rs", false),
            ]
        );
        assert!(
            later[0].why().contains("T3 runs after T2"),
            "{}",
            later[0].why()
        );

        let plan_dir = temp.path().join("plans/widgets");
        std::fs::create_dir_all(&plan_dir).expect("the plan directory");
        std::fs::write(plan_dir.join("tasks.toml"), &toml).expect("tasks.toml");
        let report = crate::plan_validate::validate_plans_dir_with_workdir(
            &plan_dir,
            None,
            Some(temp.path()),
        )
        .expect("validate");
        let flagged: Vec<(String, Option<&str>)> = report.plans[0]
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.rule_id == "PLAN_058")
            .map(|diagnostic| {
                (
                    format!("{:?}", diagnostic.severity),
                    diagnostic.task_id.as_deref(),
                )
            })
            .collect();
        assert_eq!(
            flagged,
            [
                ("Error".to_string(), Some("T2")),
                ("Warning".to_string(), Some("T2")),
            ]
        );
        assert_eq!(report.exit_code(false), 1, "the plan cannot run");
    }

    /// Round v7, phase 4's first spec (Undertow a4108ae): the plumbing task
    /// checked P4.02 with `tools/spec.sh 4 state`, whose test includes the
    /// editor helpers, which use `UiFrame`; only the next task writes
    /// `plugin/ui/UiFrame.h`, so the check could not compile when its task
    /// ended. A test that includes a later task's header directly is caught
    /// the same way. A type used only in a comment or a string, one the
    /// existing non-test code already uses, and a test whose task waits for
    /// the writer are fine.
    #[test]
    fn a_test_that_uses_a_later_tasks_type_is_plan_058() {
        let temp = tempfile::tempdir().expect("tempdir");
        let root = temp.path();
        for (path, text) in [
            ("CMakeLists.txt", "add_subdirectory(plugin)\n"),
            (
                "plugin/PluginEditor.h",
                "#pragma once\nclass UndertowEditor {};\n",
            ),
            (
                "plugin/PluginProcessor.h",
                "#pragma once\nclass UndertowProcessor {};\n",
            ),
            (
                "tests/spec/support/spec_ui.h",
                "#pragma once\n#include \"PluginEditor.h\"\n#include <memory>\n\
                 // A Theme is drawn here.\ninline UiFrame idleFrame() { UiFrame f; return f; }\n\
                 inline const char* label() { return \"Look\"; }\n",
            ),
            (
                "tests/spec/p4/state_test.cpp",
                "#include \"doctest.h\"\n#include \"spec_ui.h\"\n\
                 TEST_CASE(\"P4.02 the pattern is saved\") { auto frame = idleFrame(); }\n",
            ),
            (
                "tests/spec/p4/frame_test.cpp",
                "#include \"doctest.h\"\n#include \"ui/Displays.h\"\nTEST_CASE(\"P4.06\") {}\n",
            ),
            (
                "tests/spec/p4/editor_test.cpp",
                "#include \"spec_ui.h\"\nTEST_CASE(\"P4.03\") { UndertowEditor editor; }\n",
            ),
        ] {
            let path = root.join(path);
            std::fs::create_dir_all(path.parent().expect("a parent")).expect("mkdir");
            std::fs::write(path, text).expect("a file");
        }
        let task = |id: &str, files: &str, depends_on: &str, steps: &[&str]| {
            let steps: String = steps
                .iter()
                .map(|command| {
                    format!("[[task.verify]]\nphase = \"test\"\ncommand = {command:?}\n\n")
                })
                .collect();
            format!(
                "[[task]]\nid = \"{id}\"\ntitle = \"Task {id}\"\nstatus = \"ready\"\n\
                 role = \"implementer\"\ntier = \"focused\"\nfiles = [{files}]\n\
                 depends_on = [{depends_on}]\n\n{steps}"
            )
        };
        let toml = format!(
            "[meta]\nplan = \"p4\"\ntotal = 3\nstatus = \"ready\"\n\n{}{}{}",
            task(
                "T1",
                "\"plugin/PluginProcessor.cpp\"",
                "",
                &[
                    "tools/spec.sh 4 state 'P4.02*'",
                    "tools/spec.sh 4 frame",
                    "tools/spec.sh 4 editor",
                ],
            ),
            task(
                "T2",
                "\"plugin/ui/UiFrame.h\", \"plugin/ui/Theme.h\", \"plugin/ui/Look.h\"",
                "\"T1\"",
                &["tools/spec.sh 4 state"],
            ),
            task("T3", "\"plugin/ui/Displays.h\"", "\"T2\"", &[]),
        );
        let plan = TasksFile::parse_str(&toml).expect("a plan");
        let later = later_files(&plan, root);
        let found: Vec<(&str, usize, &str, Option<&str>)> = later
            .iter()
            .map(|later| {
                (
                    later.task.as_str(),
                    later.step,
                    later.path.as_str(),
                    later.how.as_deref(),
                )
            })
            .collect();
        assert_eq!(
            found,
            [
                (
                    "T1",
                    1,
                    "plugin/ui/UiFrame.h",
                    Some("its test uses `UiFrame`")
                ),
                (
                    "T1",
                    2,
                    "plugin/ui/Displays.h",
                    Some("its test includes `ui/Displays.h`")
                ),
                (
                    "T1",
                    3,
                    "plugin/ui/UiFrame.h",
                    Some("its test uses `UiFrame`")
                ),
            ]
        );
        assert!(
            later[0]
                .why()
                .starts_with("it needs `plugin/ui/UiFrame.h` (its test uses `UiFrame`)"),
            "{}",
            later[0].why()
        );
        let guessed: Vec<bool> = later.iter().map(|later| later.guessed).collect();
        assert_eq!(guessed, [true, false, true]);
        // A guess is a warning; a missing include an error. Generation acts
        // on the include only.
        let plan_dir = root.join("plans/p4");
        std::fs::create_dir_all(&plan_dir).expect("the plan directory");
        std::fs::write(plan_dir.join("tasks.toml"), &toml).expect("tasks.toml");
        let report =
            crate::plan_validate::validate_plans_dir_with_workdir(&plan_dir, None, Some(root))
                .expect("validate");
        let mut severities: Vec<(String, bool)> = report.plans[0]
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.rule_id == "PLAN_058")
            .map(|diagnostic| {
                (
                    format!("{:?}", diagnostic.severity),
                    diagnostic.message.contains("its test uses"),
                )
            })
            .collect();
        severities.sort();
        assert_eq!(
            severities,
            [
                ("Error".to_string(), false),
                ("Warning".to_string(), true),
                ("Warning".to_string(), true),
            ]
        );
        let stuck: Vec<usize> = problems(&plan, root, &Probe::with(&[], &[]))
            .iter()
            .map(|stuck| stuck.step)
            .collect();
        assert_eq!(stuck, [2]);

        // A Rust workspace's tests are not read.
        std::fs::remove_file(root.join("CMakeLists.txt")).expect("no CMake");
        assert!(later_files(&plan, root).is_empty());
    }

    /// bug-72dc0d: phase 2's planner checked a CMake project's tasks with
    /// `cargo`. A build tool whose manifest neither the workspace nor a
    /// directory above it has, and no task writes before the step runs,
    /// cannot run there; with its manifest, one a task writes, a flag that
    /// points elsewhere, or after a `cd`, it can.
    #[test]
    fn a_build_tool_without_its_manifest_cannot_run() {
        let temp = tempfile::tempdir().expect("tempdir");
        let probe = Probe::with(&[("cargo", true), ("go", true), ("npm", true)], &[]);
        let problem = |command: &str, written: &[&str]| {
            let written: Vec<String> = written.iter().map(|path| (*path).to_string()).collect();
            step_problem(command, temp.path(), &written, &probe)
        };
        let why = problem(
            "test -f core/src/rumble.cpp && cargo test -p undertow_core rumble",
            &["core/src/rumble.cpp"],
        )
        .expect("no Cargo.toml");
        assert!(
            why.contains("`cargo test`") && why.contains("`Cargo.toml`"),
            "{why}"
        );
        assert!(problem("cargo check --workspace", &[]).is_some());
        assert!(problem("npm test", &[]).is_some());
        assert!(problem("go test ./...", &[]).is_some());
        for command in [
            "go run main.go",
            "cargo --version",
            "cargo test --manifest-path sub/Cargo.toml",
            "cd sub && cargo test",
        ] {
            assert_eq!(problem(command, &[]), None, "{command}");
        }
        assert_eq!(problem("cargo test -p roko", &["Cargo.toml"]), None);
        std::fs::write(temp.path().join("Cargo.toml"), "[workspace]\n").expect("a manifest");
        assert_eq!(problem("cargo test -p undertow_core rumble", &[]), None);
    }

    /// The prompt says which runners this machine has.
    #[test]
    fn the_runner_note_says_pytest_is_missing() {
        let temp = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir_all(temp.path().join("tests")).expect("tests");
        std::fs::write(temp.path().join("tests/test_osc.py"), "import unittest\n")
            .expect("a Python test");
        let probe = Probe::with(
            &[("pytest", false), ("python3", true)],
            &[("pytest", false)],
        );
        let block = runners_block(temp.path(), &probe).expect("a Python project");
        assert!(block.contains("`pytest` is NOT installed here"), "{block}");
        assert!(runners_block(&temp.path().join("tests"), &probe).is_none());
    }

    /// bug-574c45: Undertow, a C++/CMake repo, has a `tests/` directory of
    /// C++ spec tests and was told "pytest is NOT installed here" and nothing
    /// of cmake. A `tests/` directory alone makes no Python project; a
    /// `CMakeLists.txt` two levels down makes a CMake one, and a Python test
    /// file still makes a Python one.
    #[test]
    fn the_runner_note_names_cmake_not_python() {
        let temp = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir_all(temp.path().join("tests/spec/p0")).expect("tests");
        std::fs::write(
            temp.path().join("tests/spec/CMakeLists.txt"),
            "add_executable(spec)\n",
        )
        .expect("the spec tests' CMakeLists.txt");
        std::fs::write(
            temp.path().join("tests/spec/p0/engine_test.cpp"),
            "int main() {}\n",
        )
        .expect("a spec test");
        let probe = Probe::with(
            &[
                ("cmake", true),
                ("ctest", true),
                ("pytest", false),
                ("python3", true),
            ],
            &[("pytest", false)],
        );
        let block = runners_block(temp.path(), &probe).expect("a CMake project");
        assert!(
            block.contains("- CMake: `cmake` and `ctest` run here"),
            "{block}"
        );
        assert!(
            !block.contains("Python") && !block.contains("pytest"),
            "{block}"
        );

        std::fs::write(temp.path().join("CMakePresets.json"), "{}\n").expect("presets");
        let block = runners_block(temp.path(), &probe).expect("a CMake project");
        assert!(
            block.contains("`ctest --preset <name> -R <test>`"),
            "{block}"
        );

        std::fs::write(
            temp.path().join("tests/test_render.py"),
            "import unittest\n",
        )
        .expect("a Python test");
        let block = runners_block(temp.path(), &probe).expect("both");
        assert!(block.contains("`pytest` is NOT installed here"), "{block}");
        assert!(block.contains("- CMake:"), "{block}");
    }
}
