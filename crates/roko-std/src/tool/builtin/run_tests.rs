//! `run_tests` — run the project's tests and return structured results.
//!
//! Category: [`ToolCategory::Exec`]. Permission: read + exec.
//! Concurrency: [`ToolConcurrency::Serial`] (tests often share fixtures).
//! Idempotent: no.

use std::ffi::{OsStr, OsString};
use std::io::Read as _;
use std::path::{Component, Path};
use std::time::Instant;

use async_trait::async_trait;
use roko_core::child_env;
use roko_core::tool::{
    ToolCall, ToolCategory, ToolConcurrency, ToolContext, ToolDef, ToolError, ToolHandler,
    ToolPermission, ToolResult, ToolSchema,
};
use serde_json::{Map, Value};

use super::child_group;
use super::task_checks::{self, TAIL_BYTES, TAIL_LINES, tail_of, task_checks_path};

/// Canonical `snake_case` name.
pub const NAME: &str = "run_tests";

/// Human-readable description sent to the LLM.
pub const DESCRIPTION: &str = "Run the project's tests (or a filtered subset) and \
    return structured results: whether they passed, the counts it can read, and the end \
    of the output, where the failures are. Without build, the runner is the one the \
    checkout's root fits. With build \"checks\", run this task's own checks instead: the \
    verify steps roko runs on your work when you hand off, in order, stopping at the first \
    that fails and showing the end of its output. Run them after a change rather than \
    handing off to find out.";

/// Build the [`ToolDef`] for `run_tests`.
#[must_use]
pub fn tool_def() -> ToolDef {
    ToolDef::new(
        NAME,
        DESCRIPTION,
        ToolCategory::Exec,
        ToolPermission::executes(),
    )
    .with_parameters(ToolSchema::from_value(serde_json::json!({
        "type": "object",
        "properties": {
            "build": {
                "type": "string",
                "enum": [
                    "cargo", "npm", "node", "go", "pytest", "unittest", "forge", "make", "checks"
                ],
                "description": "Test runner: cargo (cargo test), npm (npm test), node \
                    (node --test, for *.test.ts and *.test.js files), go, pytest, unittest \
                    (python3 -m unittest), forge or make (make test); or 'checks' for this \
                    task's own checks (any language: CMake, ctest, scripts). Default: the \
                    runner the checkout's root fits (Cargo.toml: cargo, a package.json test \
                    script: npm, go.mod: go, foundry.toml: forge, a Makefile test target: \
                    make, node or Python test files: node or unittest)."
            },
            "filter": {
                "type": "string",
                "description": "Run a subset of the tests, which each runner selects its own \
                    way. cargo: a test name, which cargo's selection flags may join (-p \
                    <package> for one package's tests, --test <name>, --lib, --features \
                    <list>), and the test harness's after -- (--exact, --nocapture, \
                    --ignored). go: a test name pattern (-run), or a package path starting \
                    with ./ (./strings). pytest: a test file, directory or node id \
                    (py/test_x.py::test_y; a test file's name alone is found below the \
                    root), else a keyword expression (-k). unittest: a test module, class or \
                    method name. node: a test file's path or part of it, else a test name \
                    pattern. forge: a test name pattern (--match-test), or a .sol file \
                    (--match-path). npm: passed on to the test script (npm test -- \
                    <filter>). make: TEST=<filter> when the Makefile reads $(TEST); it \
                    takes none otherwise."
            },
            "timeout_ms": {
                "type": "integer",
                "description": "Optional wall-clock timeout in milliseconds. Default 120000 \
                    (`[timeouts] tool_execution_ms`); a longer one is honoured up to 600000 \
                    (`[timeouts] tool_execution_max_ms`)."
            }
        },
        "additionalProperties": false
    })))
    .with_concurrency(ToolConcurrency::Serial)
    .with_idempotent(false)
    .with_timeout_ms(600_000)
}

/// Handler for `run_tests` (§36.29).
///
/// Runs the test command of the runner `build` names (`cargo test`,
/// `npm test`, `node --test`, `go test ./...`, …), or of the one the
/// checkout's root fits when it names none.
///
/// The result is structured whether the tests pass or fail (bug-1f8db7):
///
/// ```json
/// { "build": "...", "status": "ok"|"failed", "exit_code": N,
///   "passed": N, "failed": N, "ignored": N, "output": "..." }
/// ```
///
/// `exit_code` is there when the tests fail, and the counts when the
/// output's summary could be read; `output` is the output's end. A runner
/// that cannot start, a run past its time, and a call naming no runner the
/// checkout fits are errors, which say what is missing and which runners
/// the checkout's tests do fit.
///
/// A `filter` reaches each runner as that runner's own way of picking tests
/// (`go test ./... -run=…`, `forge test --match-test=…`, `pytest -k …`, …;
/// bug-83d420), never as one more word it would read as a package, target
/// or path. A filter a runner cannot take, such as cargo flags that would
/// change what cargo runs or where, is an error that says what it takes.
///
/// The tests run with the environment verify steps get
/// ([`child_env::apply_gate_env`]): agent-written tests see no provider key,
/// secret-looking name or `.env`-loaded name unless
/// [`ToolContext::env_passthrough`] names it.
#[derive(Debug, Clone, Copy, Default)]
pub struct Handler;

#[async_trait]
impl ToolHandler for Handler {
    fn name(&self) -> &str {
        NAME
    }

    async fn execute(&self, call: ToolCall, ctx: &ToolContext) -> ToolResult {
        run(call, ctx, None).await
    }
}

/// A test runner `build` can name.
#[derive(Debug)]
struct Runner {
    /// Its `build` value.
    build: &'static str,
    /// The program it runs.
    program: &'static str,
    /// The command it amounts to, as an agent would type it.
    command: &'static str,
    /// The program's arguments before a filter.
    args: &'static [&'static str],
}

/// The runners, best first when a checkout fits several; `checks` runs the
/// task's own checks instead.
static RUNNERS: [Runner; 8] = [
    Runner {
        build: "cargo",
        program: "cargo",
        command: "cargo test",
        args: &["test", "--workspace"],
    },
    Runner {
        build: "npm",
        program: "npm",
        command: "npm test",
        args: &["test"],
    },
    Runner {
        build: "go",
        program: "go",
        command: "go test ./...",
        args: &["test", "./..."],
    },
    Runner {
        build: "forge",
        program: "forge",
        command: "forge test",
        args: &["test"],
    },
    Runner {
        build: "make",
        program: "make",
        command: "make test",
        args: &["test"],
    },
    Runner {
        build: "node",
        program: "node",
        command: "node --test",
        args: &["--test"],
    },
    Runner {
        build: "pytest",
        program: "python3",
        command: "python3 -m pytest",
        args: &["-m", "pytest"],
    },
    Runner {
        build: "unittest",
        program: "python3",
        command: "python3 -m unittest",
        args: &["-m", "unittest", "discover"],
    },
];

/// The lines of a passing run's output the agent gets back.
const PASSED_TAIL_LINES: usize = 15;

/// The most bytes of them.
const PASSED_TAIL_BYTES: usize = 2 * 1024;

/// The most Python test directories one `unittest` call discovers from.
const MAX_UNITTEST_DIRS: usize = 8;

/// The runner `build` names.
fn runner(build: &str) -> Option<&'static Runner> {
    RUNNERS.iter().find(|runner| runner.build == build)
}

/// Run `call`. The tests inherit `parent_env` in place of roko's own
/// environment when given, filtered by the same policy.
async fn run(
    call: ToolCall,
    ctx: &ToolContext,
    parent_env: Option<Vec<(String, OsString)>>,
) -> ToolResult {
    if !ctx.capabilities.exec {
        return ToolResult::Err(ToolError::PermissionDenied(
            "run_tests requires exec capability".into(),
        ));
    }
    let requested = match call.arguments.get("build").and_then(Value::as_str) {
        // gap-770cb5: the task's own checks, within the call's time.
        Some("checks") => {
            return task_checks::run(ctx, ctx.call_timeout(&call).budget, parent_env).await;
        }
        Some(build) => match runner(build) {
            Some(runner) => Some(runner),
            None => {
                return ToolResult::Err(ToolError::Other(format!(
                    "run_tests: unknown build system `{build}`"
                )));
            }
        },
        None => None,
    };
    let filter = call
        .arguments
        .get("filter")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|filter| !filter.is_empty());
    let env = parent_env.unwrap_or_else(child_env::process_env);
    let path = env
        .iter()
        .find(|(name, _)| name == "PATH")
        .map(|(_, value)| value.clone());
    let mut checkout = Checkout {
        root: ctx.worktree(),
        path: path.as_deref(),
        ctx,
        project: None,
    };
    // bug-1f8db7: a runner that cannot start here is an error that says so,
    // and names the runners this checkout's tests do fit.
    let runner = match requested {
        Some(runner) => runner,
        None => match checkout.pick() {
            Ok(runner) => runner,
            Err(why) => return checkout.cannot_run(&why, None),
        },
    };
    if let Some(why) = checkout.missing(runner) {
        return checkout.cannot_run(&why, Some(runner));
    }
    let root = checkout.root;
    let runs = match runner.build {
        "unittest" => checkout.project().unittest_runs(filter, root),
        _ => match runner_args(runner, filter, checkout.project(), root) {
            Ok(args) => vec![args],
            // A Makefile that takes no filter: the other runners that fit may.
            Err(why) if runner.build == "make" => return checkout.cannot_run(&why, Some(runner)),
            Err(why) => return ToolResult::Err(ToolError::Other(format!("run_tests: {why}"))),
        },
    };
    let (output, failure) = match run_all(runner, &runs, &call, ctx, &env).await {
        Ok(ran) => ran,
        Err(Unfinished::NotStarted(error)) => return checkout.could_not_start(runner, &error),
        Err(Unfinished::TimedOut(error)) => return ToolResult::Err(error),
    };
    if runner.build == "pytest" && failure.is_some() && output.contains("No module named pytest") {
        return checkout.cannot_run(
            "build \"pytest\" runs `python3 -m pytest`, and pytest is not installed for \
             python3 (No module named pytest).",
            Some(runner),
        );
    }
    ToolResult::structured(outcome(runner.build, filter, failure, &output).to_string())
}

/// Why a call's runs did not all finish.
enum Unfinished {
    /// One could not start.
    NotStarted(std::io::Error),
    /// The call's time ran out.
    TimedOut(ToolError),
}

/// Run `runner`'s program with each of `runs` in turn, in the checkout,
/// within the call's time: their output, stdout before stderr, and the exit
/// code of the first that failed (`None` inside for a signal).
async fn run_all(
    runner: &Runner,
    runs: &[Vec<String>],
    call: &ToolCall,
    ctx: &ToolContext,
    env: &[(String, OsString)],
) -> Result<(String, Option<Option<i32>>), Unfinished> {
    // The call's own `timeout_ms`, up to the cap, else the context's
    // (bug-c22bd9).
    let timeout = ctx.call_timeout(call);
    let started = Instant::now();
    let mut output = String::new();
    let mut failure = None;
    for args in runs {
        let mut cmd = tokio::process::Command::new(runner.program);
        cmd.args(args).current_dir(ctx.worktree());
        child_env::apply_gate_env(cmd.as_std_mut(), env.to_vec(), &ctx.env_passthrough);
        // The run leads a process group of its own, which a cancel or the
        // timeout kills whole (bug-c83c84).
        let left = timeout.budget.saturating_sub(started.elapsed());
        let ran = match child_group::output_within(cmd, left).await {
            Ok(Ok(ran)) => ran,
            Ok(Err(error)) => return Err(Unfinished::NotStarted(error)),
            Err(_) => {
                return Err(Unfinished::TimedOut(timeout.timed_out(
                    u64::try_from(timeout.budget.as_millis()).unwrap_or(u64::MAX),
                )));
            }
        };
        if runs.len() > 1 {
            output.push_str(&format!("$ {} {}\n", runner.program, args.join(" ")));
        }
        output.push_str(&String::from_utf8_lossy(&ran.stdout));
        output.push('\n');
        output.push_str(&String::from_utf8_lossy(&ran.stderr));
        if !ran.status.success() && failure.is_none() {
            failure = Some(ran.status.code());
        }
    }
    Ok((output, failure))
}

/// What a call has learnt of the checkout its tests run in.
struct Checkout<'a> {
    /// The checkout's root.
    root: &'a Path,
    /// The `PATH` the tests get, when roko has one.
    path: Option<&'a OsStr>,
    ctx: &'a ToolContext,
    /// What its root says about its tests, once a step needed it.
    project: Option<Project>,
}

impl Checkout<'_> {
    /// What the checkout's root says about its tests, read on first use.
    fn project(&mut self) -> &Project {
        let root = self.root;
        self.project.get_or_insert_with(|| Project::scan(root))
    }

    /// The runner for a call naming none: the first the checkout fits whose
    /// program is on `PATH`, or why there is none.
    fn pick(&mut self) -> Result<&'static Runner, String> {
        let (root, path) = (self.root, self.path);
        let fits = self.project().fits();
        if let Some(runner) = fits
            .iter()
            .copied()
            .find(|runner| path.is_none_or(|path| on_path(runner.program, path, root)))
        {
            return Ok(runner);
        }
        Err(match fits.first() {
            Some(runner) => format!(
                "this checkout's tests fit build \"{}\" (`{}`), and `{}` is not installed \
                 here (it is not on PATH).",
                runner.build, runner.command, runner.program
            ),
            None => "no test runner fits this checkout's root: it has no Cargo.toml, \
                     package.json with a \"test\" script, go.mod, foundry.toml, Makefile with \
                     a test target, or node or Python test files. Pass build."
                .to_string(),
        })
    }

    /// Why `runner` cannot run here, if it cannot: its program is not on
    /// `PATH`, or `npm` has no test script to run.
    fn missing(&mut self, runner: &Runner) -> Option<String> {
        if let Some(path) = self.path
            && !on_path(runner.program, path, self.root)
        {
            return Some(format!(
                "build \"{}\" runs `{}`, and `{}` is not installed here (it is not on PATH).",
                runner.build, runner.command, runner.program
            ));
        }
        if runner.build != "npm" {
            return None;
        }
        let why = match self.project().npm_test_script {
            Some(true) => return None,
            Some(false) => "its package.json has no \"test\" script",
            None => "there is no package.json",
        };
        Some(format!(
            "build \"npm\" runs `npm test`, which needs a package.json with a \"test\" script \
             at the checkout's root, and {why}."
        ))
    }

    /// The error for a run of `runner` that did not start.
    fn could_not_start(&mut self, runner: &Runner, error: &std::io::Error) -> ToolResult {
        if !self.root.is_dir() {
            return ToolResult::Err(ToolError::Other(format!(
                "run_tests: the checkout {} does not exist",
                self.root.display()
            )));
        }
        let why = if error.kind() == std::io::ErrorKind::NotFound {
            format!(
                "build \"{}\" runs `{}`, and `{}` was not found: {error}.",
                runner.build, runner.command, runner.program
            )
        } else {
            format!("`{}` could not start: {error}.", runner.command)
        };
        self.cannot_run(&why, Some(runner))
    }

    /// The error for a call whose runner cannot run, for `why`: it ends with
    /// the runners this checkout's tests fit that can, and with `checks`
    /// when the call's task has checks.
    fn cannot_run(&mut self, why: &str, runner: Option<&Runner>) -> ToolResult {
        let (root, path) = (self.root, self.path);
        let fits: Vec<String> = self
            .project()
            .fits()
            .into_iter()
            .filter(|fit| runner.is_none_or(|runner| fit.build != runner.build))
            .filter(|fit| path.is_none_or(|path| on_path(fit.program, path, root)))
            .map(|fit| format!("build \"{}\" (`{}`)", fit.build, fit.command))
            .collect();
        let mut message = format!("run_tests: {why}");
        if !fits.is_empty() {
            message.push_str(&format!(
                " This checkout's tests fit {}.",
                fits.join(" or ")
            ));
        }
        let attempt = &self.ctx.correlation.attempt_id;
        if !attempt.is_empty() && task_checks_path(&self.ctx.immune_root_path, attempt).is_file() {
            message.push_str(" Build \"checks\" runs this task's own checks.");
        }
        ToolResult::Err(ToolError::Other(message))
    }
}

/// What a checkout's root says about how its tests run.
#[derive(Debug, Default)]
struct Project {
    /// The runners its manifests fit, best first: cargo, npm (a package.json
    /// with a test script), go, forge and make (a Makefile test target).
    manifests: Vec<&'static Runner>,
    /// `None` without a package.json, else whether it has a test script.
    npm_test_script: Option<bool>,
    /// Whether its Makefile reads a `TEST` variable, through which `make
    /// test` takes a filter.
    make_test_var: bool,
    /// Whether it configures pytest.
    pytest_config: bool,
    /// Node test files (`*.test.ts`, `*.test.js`, …) below it.
    node_tests: Vec<String>,
    /// Python test files (`test*.py`, `*_test.py`) below it.
    python_tests: Vec<String>,
}

/// Directories a scan for test files skips: dependencies, build output and
/// caches, besides hidden ones.
const SKIPPED_DIRS: [&str; 7] = [
    "node_modules",
    "target",
    "__pycache__",
    "venv",
    "site-packages",
    "dist",
    "build",
];

/// The most directory entries a scan for test files reads.
const SCAN_ENTRIES: usize = 20_000;

/// How many directories deep below the root it looks.
const SCAN_DEPTH: usize = 8;

/// The most bytes of a manifest a scan reads.
const MANIFEST_BYTES: u64 = 1024 * 1024;

impl Project {
    /// Read what the checkout at `root` says about its tests.
    fn scan(root: &Path) -> Self {
        let read = |name: &str| read_manifest(&root.join(name));
        let npm_test_script = read("package.json").map(|text| has_npm_test_script(&text));
        let makefiles: Vec<String> = ["GNUmakefile", "makefile", "Makefile"]
            .into_iter()
            .filter_map(read)
            .collect();
        let make_test = makefiles.iter().any(|text| has_make_test_target(text));
        let make_test_var = makefiles
            .iter()
            .any(|text| text.contains("$(TEST)") || text.contains("${TEST}"));
        let manifests = [
            ("cargo", root.join("Cargo.toml").is_file()),
            ("npm", npm_test_script == Some(true)),
            ("go", root.join("go.mod").is_file()),
            ("forge", root.join("foundry.toml").is_file()),
            ("make", make_test),
        ]
        .into_iter()
        .filter(|(_, fits)| *fits)
        .filter_map(|(build, _)| runner(build))
        .collect();
        let pytest_config = root.join("pytest.ini").is_file()
            || root.join("conftest.py").is_file()
            || read("pyproject.toml").is_some_and(|text| text.contains("[tool.pytest"))
            || read("setup.cfg").is_some_and(|text| text.contains("[tool:pytest]"))
            || read("tox.ini").is_some_and(|text| text.contains("[pytest]"));
        let (node_tests, python_tests) = test_files(root);
        Self {
            manifests,
            npm_test_script,
            make_test_var,
            pytest_config,
            node_tests,
            python_tests,
        }
    }

    /// The runners this checkout's tests fit, best first.
    fn fits(&self) -> Vec<&'static Runner> {
        let tests = [
            ("node", !self.node_tests.is_empty()),
            ("pytest", self.pytest_config),
            ("unittest", !self.python_tests.is_empty()),
        ]
        .into_iter()
        .filter(|(_, fits)| *fits)
        .filter_map(|(build, _)| runner(build));
        self.manifests.iter().copied().chain(tests).collect()
    }

    /// `python3 -m unittest discover` from each directory holding Python
    /// test files, but not from a package inside another of them: discovery
    /// from that one finds its tests. `filter` picks tests by module, class
    /// or method name (`-k`).
    fn unittest_runs(&self, filter: Option<&str>, root: &Path) -> Vec<Vec<String>> {
        let mut dirs: Vec<&str> = self
            .python_tests
            .iter()
            .map(|file| file.rsplit_once('/').map_or(".", |(dir, _)| dir))
            .collect();
        dirs.sort_unstable();
        dirs.dedup();
        let all = dirs.clone();
        dirs.retain(|dir| {
            let inside_another = all.iter().any(|other| {
                other != dir && (*other == "." || dir.starts_with(&format!("{other}/")))
            });
            !(inside_another && root.join(dir).join("__init__.py").is_file())
        });
        if dirs.is_empty() {
            dirs.push(".");
        }
        dirs.truncate(MAX_UNITTEST_DIRS);
        let pattern = filter.map(|filter| {
            let name = filter.rsplit('/').next().unwrap_or(filter);
            name.strip_suffix(".py").unwrap_or(name)
        });
        dirs.into_iter()
            .map(|dir| {
                ["-m", "unittest", "discover", "-s", dir]
                    .into_iter()
                    .chain(pattern.into_iter().flat_map(|pattern| ["-k", pattern]))
                    .map(String::from)
                    .collect()
            })
            .collect()
    }
}

/// `node --test`'s arguments for `filter`: the test files whose path holds
/// it, else a test name pattern. A checkout with TypeScript test files runs
/// them with Node's type stripping.
fn node_args(filter: Option<&str>, tests: &[String]) -> Vec<String> {
    let files: Vec<&String> = filter.map_or_else(Vec::new, |filter| {
        tests.iter().filter(|test| test.contains(filter)).collect()
    });
    let mut args = Vec::new();
    if tests
        .iter()
        .any(|test| matches!(extension(test), "ts" | "mts" | "cts"))
    {
        args.push("--experimental-strip-types".to_string());
    }
    args.push("--test".to_string());
    if let Some(filter) = filter
        && files.is_empty()
    {
        args.push(format!("--test-name-pattern={filter}"));
    }
    args.extend(files.into_iter().cloned());
    args
}

/// The arguments of `runner`'s one run for `filter`, which each runner takes
/// its own way (bug-83d420), or why it cannot take this filter. `unittest`
/// has [`Project::unittest_runs`].
fn runner_args(
    runner: &Runner,
    filter: Option<&str>,
    project: &Project,
    root: &Path,
) -> Result<Vec<String>, String> {
    if runner.build == "node" {
        return Ok(node_args(filter, &project.node_tests));
    }
    let mut args: Vec<String> = runner.args.iter().map(|arg| (*arg).to_string()).collect();
    let Some(filter) = filter else {
        return Ok(args);
    };
    match runner.build {
        "cargo" => return cargo_args(filter),
        "go" if is_go_package(filter) => args = vec!["test".to_string(), filter.to_string()],
        "go" => args.push(format!("-run={filter}")),
        "forge" if extension(filter) == "sol" => args.push(format!("--match-path={filter}")),
        "forge" => args.push(format!("--match-test={filter}")),
        "pytest" => args.extend(pytest_args(filter, &project.python_tests, root)?),
        "npm" if filter.starts_with('-') => {
            return Err(format!(
                "for npm, filter goes to the test script (`npm test -- <filter>`) as a test \
                 file or name, and `{filter}` begins with `-`: run_tests does not pass on the \
                 test runner's own options."
            ));
        }
        "npm" => args.extend(["--".to_string(), filter.to_string()]),
        "make" => args.push(make_arg(filter, project.make_test_var)?),
        build => return Err(format!("build \"{build}\" takes no filter.")),
    }
    Ok(args)
}

/// Cargo's flags a `cargo` filter may carry, each with whether it takes a
/// value: which packages and targets to test, with which features and
/// profile. Any other (`--config`, `--manifest-path`, `--target-dir`, `-Z`,
/// …) could change what cargo runs or where, so a filter that gives one is
/// refused.
const CARGO_FLAGS: [(&str, bool); 23] = [
    ("-p", true),
    ("--package", true),
    ("--workspace", false),
    ("--all", false),
    ("--exclude", true),
    ("--lib", false),
    ("--bins", false),
    ("--bin", true),
    ("--tests", false),
    ("--test", true),
    ("--examples", false),
    ("--example", true),
    ("--benches", false),
    ("--bench", true),
    ("--all-targets", false),
    ("--doc", false),
    ("-F", true),
    ("--features", true),
    ("--all-features", false),
    ("--no-default-features", false),
    ("--release", false),
    ("--profile", true),
    ("--no-fail-fast", false),
];

/// The test harness's flags a `cargo` filter may carry after `--`, each
/// with whether it takes a value.
const HARNESS_FLAGS: [(&str, bool); 10] = [
    ("--exact", false),
    ("--nocapture", false),
    ("--no-capture", false),
    ("--show-output", false),
    ("--ignored", false),
    ("--include-ignored", false),
    ("--skip", true),
    ("--test-threads", true),
    ("--quiet", false),
    ("--list", false),
];

/// `cargo test`'s arguments for `filter` (bug-83d420): a test name, or a
/// filter's words when it carries cargo's selection flags (`-p alpha --test
/// accept_word_count`), each flag one of [`CARGO_FLAGS`], or after `--` one
/// of [`HARNESS_FLAGS`]. The whole workspace is tested unless the filter
/// picks packages: with `--workspace`, cargo would ignore its `-p`.
fn cargo_args(filter: &str) -> Result<Vec<String>, String> {
    let words: Vec<&str> = filter.split_whitespace().collect();
    let mut harness = false;
    let mut whole_workspace = true;
    for word in &words {
        // A test name or a flag's value: what cargo reads as a flag starts
        // with `-`.
        if !word.starts_with('-') {
            continue;
        }
        if *word == "--" && !harness {
            harness = true;
            continue;
        }
        let flags: &[(&str, bool)] = if harness {
            &HARNESS_FLAGS
        } else {
            &CARGO_FLAGS
        };
        let (flag, attached) = split_flag(word);
        // A flag taking no value with more attached (`-qZ`) is more flags.
        if !flags
            .iter()
            .any(|(name, takes_value)| *name == flag && (*takes_value || attached.is_none()))
        {
            return Err(format!(
                "filter `{filter}` gives cargo `{word}`, which run_tests does not pass on. For \
                 cargo, filter is a test name (`cargo test <filter>`), which cargo's selection \
                 flags may join: `-p <package>` runs one package's tests, `--test <name>` one \
                 integration test target, `--lib` the library's unit tests, `--features <list>` \
                 turns features on; the test harness's flags (`--exact`, `--nocapture`, \
                 `--ignored`, `--skip <name>`) go after `--`."
            ));
        }
        if !harness && matches!(flag, "-p" | "--package" | "--workspace" | "--all") {
            whole_workspace = false;
        }
    }
    let mut args = vec!["test".to_string()];
    if whole_workspace {
        args.push("--workspace".to_string());
    }
    args.extend(words.into_iter().map(String::from));
    Ok(args)
}

/// A flag word's flag and the value attached to it, if any: `--features=a`
/// and `-palpha` hold one.
fn split_flag(word: &str) -> (&str, Option<&str>) {
    if word.starts_with("--") {
        return word
            .split_once('=')
            .map_or((word, None), |(flag, value)| (flag, Some(value)));
    }
    let end = word.char_indices().nth(2).map_or(word.len(), |(at, _)| at);
    let (flag, value) = word.split_at(end);
    (flag, (!value.is_empty()).then_some(value))
}

/// Whether a `go` filter is a package path inside the checkout (`./strings`,
/// `./...`), which runs in place of `./...`.
fn is_go_package(filter: &str) -> bool {
    (filter == "." || filter.starts_with("./"))
        && !filter.contains(char::is_whitespace)
        && !Path::new(filter)
            .components()
            .any(|part| part == Component::ParentDir)
}

/// `python3 -m pytest`'s arguments for `filter` (bug-83d420): a test file,
/// directory or node id (`py/test_x.py::test_y`) inside the checkout as
/// given, or with a test file named by its name or its path's end where the
/// scan found it below the root (`test_x.py` for `py/test_x.py`); else a
/// keyword expression (`-k`). A filter that begins with `-` is refused, as
/// pytest would read it as one of its own options.
fn pytest_args(filter: &str, tests: &[String], root: &Path) -> Result<Vec<String>, String> {
    if filter.starts_with('-') {
        return Err(format!(
            "for pytest, filter is a test file, directory or node id (`py/test_x.py::test_y`) \
             or a keyword expression (`-k`), and `{filter}` begins with `-`: run_tests does not \
             pass on pytest's own options."
        ));
    }
    let (path, node) = match filter.split_once("::") {
        Some((path, node)) => (path, Some(node)),
        None => (filter, None),
    };
    let path = Path::new(path);
    let mut path = path.strip_prefix(root).unwrap_or(path);
    if path.as_os_str().is_empty() {
        path = Path::new(".");
    }
    let is_path = node.is_some()
        || path.extension().is_some_and(|extension| extension == "py")
        || path.components().count() > 1
        || root.join(path).exists();
    if !is_path {
        return Ok(vec!["-k".to_string(), filter.to_string()]);
    }
    if !path
        .components()
        .all(|part| matches!(part, Component::Normal(_) | Component::CurDir))
    {
        return Err(format!(
            "for pytest, filter `{filter}` names a path outside the checkout; name a test \
             file, directory or node id inside it."
        ));
    }
    let with_node =
        |file: &str| node.map_or_else(|| file.to_string(), |node| format!("{file}::{node}"));
    let given = path.to_string_lossy();
    if !root.join(path).exists() {
        let suffix = format!("/{given}");
        let found: Vec<String> = tests
            .iter()
            .filter(|test| test.ends_with(&suffix))
            .map(|test| with_node(test))
            .collect();
        if !found.is_empty() {
            return Ok(found);
        }
    }
    // pytest says so when nothing is there.
    Ok(vec![with_node(&given)])
}

/// `make test`'s argument for `filter` (bug-83d420): `TEST=<filter>` when
/// the Makefile reads a `TEST` variable (`reads_test`) and the filter is a
/// plain test name, as the Makefile's commands read it as shell words; else
/// why `make test` takes no filter. A word after `make test` would be one
/// more target to build.
fn make_arg(filter: &str, reads_test: bool) -> Result<String, String> {
    if !reads_test {
        return Err(
            "build \"make\" runs `make test`, which takes no filter here: the Makefile reads no \
             TEST variable to pass one in. Run it without filter, or name the runner its tests \
             use."
                .to_string(),
        );
    }
    let plain = !filter.starts_with('-')
        && filter
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | ':' | '/' | '-'));
    if !plain {
        return Err(format!(
            "build \"make\" passes a filter as `make test TEST=<filter>`, which the Makefile's \
             commands read as shell words, so the filter must be a plain test name (letters, \
             digits and _ . : / -), and `{filter}` is not."
        ));
    }
    Ok(format!("TEST={filter}"))
}

/// The structured answer of a run that ran: `failure` holds the exit code of
/// the first run that failed (`None` inside for a signal).
fn outcome(build: &str, filter: Option<&str>, failure: Option<Option<i32>>, output: &str) -> Value {
    let mut payload = Map::new();
    payload.insert("build".into(), build.into());
    let status = if failure.is_some() { "failed" } else { "ok" };
    payload.insert("status".into(), status.into());
    if let Some(code) = failure {
        payload.insert("exit_code".into(), code.into());
    }
    if let Some(counts) = read_counts(output) {
        payload.insert("passed".into(), counts.passed.into());
        payload.insert("failed".into(), counts.failed.into());
        payload.insert("ignored".into(), counts.ignored.into());
        if counts.passed == 0 && counts.failed == 0 && counts.ignored == 0 {
            let note = if filter.is_some() {
                "no test matched the filter"
            } else {
                "no tests ran"
            };
            payload.insert("note".into(), note.into());
        }
    }
    let (lines, bytes) = if failure.is_some() {
        (TAIL_LINES, TAIL_BYTES)
    } else {
        (PASSED_TAIL_LINES, PASSED_TAIL_BYTES)
    };
    payload.insert(
        "output".into(),
        tail_of(output.trim_end(), lines, bytes).into(),
    );
    Value::Object(payload)
}

#[derive(Debug, Default, PartialEq, Eq)]
struct Counts {
    passed: u32,
    failed: u32,
    ignored: u32,
}

/// The counts in a run's summary lines, in cargo's, Node's, forge's,
/// pytest's, go's or `unittest`'s words, whichever runner printed them
/// (`make test` may run any of them).
fn read_counts(output: &str) -> Option<Counts> {
    cargo_counts(output)
        .or_else(|| node_counts(output))
        .or_else(|| forge_counts(output))
        .or_else(|| pytest_counts(output))
        .or_else(|| go_counts(output))
        .or_else(|| unittest_counts(output))
}

/// Cargo's `test result: ok. 5 passed; 2 failed; 1 ignored` lines, summed.
fn cargo_counts(output: &str) -> Option<Counts> {
    let mut counts = Counts::default();
    let mut found = false;
    for line in output.lines() {
        let line = line.trim();
        if line.starts_with("test result:") {
            found = true;
            counts.passed += extract(line, "passed");
            counts.failed += extract(line, "failed");
            counts.ignored += extract(line, "ignored");
        }
    }
    found.then_some(counts)
}

fn extract(line: &str, label: &str) -> u32 {
    for part in line.split(';') {
        let p = part.trim();
        if let Some(rest) = p.strip_suffix(label).map(str::trim_end)
            && let Some(num) = rest.split_whitespace().last()
            && let Ok(n) = num.parse::<u32>()
        {
            return n;
        }
    }
    0
}

/// `node --test`'s summary: `# pass 7`, `# fail 1`, … as TAP prints them, or
/// `ℹ pass 7` as its spec reporter does.
fn node_counts(output: &str) -> Option<Counts> {
    let mut counts = Counts::default();
    let mut found = false;
    for line in output.lines() {
        let line = line.trim();
        let Some(summary) = line.strip_prefix("# ").or_else(|| line.strip_prefix("ℹ ")) else {
            continue;
        };
        let Some((label, number)) = summary.split_once(' ') else {
            continue;
        };
        let Ok(number) = number.trim().parse::<u32>() else {
            continue;
        };
        match label {
            "pass" => counts.passed += number,
            "fail" | "cancelled" => counts.failed += number,
            "skipped" | "todo" => counts.ignored += number,
            _ => continue,
        }
        found |= matches!(label, "pass" | "fail");
    }
    found.then_some(counts)
}

/// forge's `Suite result: ok. 1 passed; 1 failed; 0 skipped; finished in …`
/// lines, summed across its suites; its `Ran N tests for …` and `Ran N test
/// suites in …` lines are not `unittest`'s (bug-f831b3).
fn forge_counts(output: &str) -> Option<Counts> {
    let mut counts = Counts::default();
    let mut found = false;
    for line in output.lines() {
        let Some(rest) = line.trim().strip_prefix("Suite result:") else {
            continue;
        };
        found = true;
        counts.passed += extract(rest, "passed");
        counts.failed += extract(rest, "failed");
        counts.ignored += extract(rest, "skipped");
    }
    found.then_some(counts)
}

/// pytest's final `N passed, N failed, N skipped, N error(s) in …s` line
/// (`=== … ===` around it, or none under `-q`); only the last such line
/// counts, as pytest prints one summary for the whole run.
fn pytest_counts(output: &str) -> Option<Counts> {
    let mut counts = Counts::default();
    let mut found = false;
    for line in output.lines() {
        let line = line.trim().trim_matches('=').trim();
        let summary = line.split(" in ").next().unwrap_or(line);
        let mut this_line = Counts::default();
        let mut matched = false;
        for part in summary.split(',') {
            let Some((number, label)) = part.trim().split_once(' ') else {
                continue;
            };
            let Ok(number) = number.parse::<u32>() else {
                continue;
            };
            match label {
                "passed" => this_line.passed += number,
                "failed" => this_line.failed += number,
                "error" | "errors" => this_line.failed += number,
                "skipped" => this_line.ignored += number,
                _ => continue,
            }
            matched = true;
        }
        if matched {
            counts = this_line;
            found = true;
        }
    }
    found.then_some(counts)
}

/// go's `--- PASS:`, `--- FAIL:` and `--- SKIP:` lines, counted one per
/// top-level test: an indented one is a subtest's and is not counted again.
/// `go test` without `-v` prints no `--- PASS:` lines, so a passing,
/// non-verbose run reads no counts at all.
fn go_counts(output: &str) -> Option<Counts> {
    let mut counts = Counts::default();
    let mut found = false;
    for line in output.lines() {
        if line.starts_with("--- PASS: ") {
            counts.passed += 1;
        } else if line.starts_with("--- FAIL: ") {
            counts.failed += 1;
        } else if line.starts_with("--- SKIP: ") {
            counts.ignored += 1;
        } else {
            continue;
        }
        found = true;
    }
    found.then_some(counts)
}

/// `unittest`'s `Ran 8 tests in 0.001s` and `FAILED (failures=1, errors=1,
/// skipped=2)` or `OK (skipped=2)`: what ran less what failed or was skipped
/// passed.
fn unittest_counts(output: &str) -> Option<Counts> {
    let mut counts = Counts::default();
    let mut found = false;
    for line in output.lines() {
        let line = line.trim();
        if let Some(ran) = line.strip_prefix("Ran ")
            && let Some(number) = unittest_ran(ran)
        {
            counts.passed += number;
            found = true;
            continue;
        }
        let Some(detail) = line
            .strip_prefix("FAILED (")
            .or_else(|| line.strip_prefix("OK ("))
        else {
            continue;
        };
        for part in detail.trim_end_matches(')').split(',') {
            let Some((label, number)) = part.trim().split_once('=') else {
                continue;
            };
            let Ok(number) = number.parse::<u32>() else {
                continue;
            };
            match label {
                "failures" | "errors" | "unexpected successes" => counts.failed += number,
                "skipped" => counts.ignored += number,
                _ => continue,
            }
            counts.passed = counts.passed.saturating_sub(number);
        }
    }
    found.then_some(counts)
}

/// The count in `unittest`'s own `Ran 8 tests in 0.001s`, read only when the
/// words after the number are exactly `test`/`tests` then `in`: forge's `Ran
/// 2 tests for …` and `Ran 1 test suite in …` are not this, though both
/// start with `Ran <N> test`.
fn unittest_ran(rest: &str) -> Option<u32> {
    let mut words = rest.split_whitespace();
    let number = words.next()?.parse::<u32>().ok()?;
    (matches!(words.next(), Some("test" | "tests")) && words.next() == Some("in")).then_some(number)
}

/// The test files below `root`: Node's (`*.test.ts`, `*-test.js`, …) and
/// Python's (`test*.py`, `*_test.py`), as paths relative to it.
///
/// The scan skips hidden directories and [`SKIPPED_DIRS`], follows no
/// symlink, and reads at most [`SCAN_ENTRIES`] entries [`SCAN_DEPTH`] deep.
fn test_files(root: &Path) -> (Vec<String>, Vec<String>) {
    let mut node = Vec::new();
    let mut python = Vec::new();
    let mut dirs = vec![(root.to_path_buf(), 0_usize)];
    let mut entries_read = 0_usize;
    'scan: while let Some((dir, depth)) = dirs.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            entries_read += 1;
            if entries_read > SCAN_ENTRIES {
                break 'scan;
            }
            let file_name = entry.file_name();
            let Some(name) = file_name.to_str() else {
                continue;
            };
            let Ok(kind) = entry.file_type() else {
                continue;
            };
            if kind.is_dir() {
                if depth < SCAN_DEPTH && !name.starts_with('.') && !SKIPPED_DIRS.contains(&name) {
                    dirs.push((entry.path(), depth + 1));
                }
                continue;
            }
            let tests = if is_node_test(name) {
                &mut node
            } else if is_python_test(name) {
                &mut python
            } else {
                continue;
            };
            if kind.is_file()
                && let Ok(relative) = entry.path().strip_prefix(root)
            {
                tests.push(relative.to_string_lossy().replace('\\', "/"));
            }
        }
    }
    node.sort();
    python.sort();
    (node, python)
}

/// `name`'s extension, or "" without one.
fn extension(name: &str) -> &str {
    name.rsplit_once('.').map_or("", |(_, extension)| extension)
}

/// Whether `name` is a test file `node --test` runs.
fn is_node_test(name: &str) -> bool {
    let Some((stem, extension)) = name.rsplit_once('.') else {
        return false;
    };
    matches!(extension, "js" | "mjs" | "cjs" | "ts" | "mts" | "cts")
        && (stem == "test"
            || stem.starts_with("test-")
            || [".test", "-test", "_test"]
                .iter()
                .any(|suffix| stem.ends_with(suffix)))
}

/// Whether `name` is a test file `unittest` or pytest finds.
fn is_python_test(name: &str) -> bool {
    name.rsplit_once('.').is_some_and(|(stem, extension)| {
        extension == "py" && (stem.starts_with("test") || stem.ends_with("_test"))
    })
}

/// Whether `package_json` has a test script other than npm's placeholder.
fn has_npm_test_script(package_json: &str) -> bool {
    serde_json::from_str::<Value>(package_json)
        .ok()
        .and_then(|manifest| {
            manifest
                .get("scripts")?
                .get("test")?
                .as_str()
                .map(str::to_string)
        })
        .is_some_and(|script| !script.trim().is_empty() && !script.contains("no test specified"))
}

/// Whether `makefile` has a `test` target.
fn has_make_test_target(makefile: &str) -> bool {
    makefile.lines().any(|line| {
        !line.starts_with(['\t', ' ', '.', '#'])
            && line.split_once(':').is_some_and(|(targets, rest)| {
                !rest.starts_with('=') && targets.split_whitespace().any(|target| target == "test")
            })
    })
}

/// The text of the manifest at `path`, at most [`MANIFEST_BYTES`] of it.
fn read_manifest(path: &Path) -> Option<String> {
    let file = std::fs::File::open(path).ok()?;
    let mut text = String::new();
    file.take(MANIFEST_BYTES).read_to_string(&mut text).ok()?;
    Some(text)
}

/// Whether `program` is an executable file in a directory of `path`, a
/// relative one taken from the checkout's `root`, where the tests run.
fn on_path(program: &str, path: &OsStr, root: &Path) -> bool {
    std::env::split_paths(path).any(|dir| {
        let dir = if dir.is_relative() {
            root.join(dir)
        } else {
            dir
        };
        is_executable(&dir.join(program))
    })
}

#[cfg(unix)]
fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(path)
        .is_ok_and(|meta| meta.is_file() && (meta.permissions().mode() & 0o111) != 0)
}

#[cfg(not(unix))]
fn is_executable(path: &Path) -> bool {
    path.is_file() || path.with_extension("exe").is_file()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cargo_counts_read_the_summary_line() {
        let out = "test result: ok. 5 passed; 2 failed; 1 ignored; 0 measured";
        assert_eq!(
            cargo_counts(out),
            Some(Counts {
                passed: 5,
                failed: 2,
                ignored: 1
            })
        );
    }

    #[test]
    fn cargo_counts_add_up_every_summary_line() {
        let out = "test result: ok. 3 passed; 0 failed; 0 ignored\n\
                   test result: FAILED. 2 passed; 1 failed; 0 ignored";
        let counts = cargo_counts(out).expect("counts");
        assert_eq!(counts.passed, 5);
        assert_eq!(counts.failed, 1);
    }

    #[test]
    fn no_summary_line_reads_no_counts() {
        assert_eq!(read_counts(""), None);
        assert_eq!(read_counts("make: Nothing to be done for `test'."), None);
    }

    /// bug-1f8db7: the golden path's TypeScript tests run under `node --test`
    /// and its Python ones under `unittest`; their summaries are read too.
    #[test]
    fn node_and_unittest_summaries_are_read() {
        let tap = "not ok 2 - fails on purpose\n1..2\n# tests 2\n# suites 0\n# pass 1\n\
                   # fail 1\n# cancelled 0\n# skipped 0\n# todo 0\n# duration_ms 159.8";
        assert_eq!(
            read_counts(tap),
            Some(Counts {
                passed: 1,
                failed: 1,
                ignored: 0
            })
        );
        let unittest = "......F.s\n------\nRan 9 tests in 0.003s\n\nFAILED (failures=1, \
                        skipped=1)";
        assert_eq!(
            read_counts(unittest),
            Some(Counts {
                passed: 7,
                failed: 1,
                ignored: 1
            })
        );
        assert_eq!(
            read_counts("Ran 1 test in 0.000s\n\nOK"),
            Some(Counts {
                passed: 1,
                failed: 0,
                ignored: 0
            })
        );
    }

    /// bug-f831b3: a forge run's `Ran N tests for …` and `Ran N test suites
    /// in …` lines are not `unittest`'s, so they are not counted as passed
    /// tests, and its own `Suite result: … passed; … failed; … skipped`
    /// lines are, summed across suites; a failure is counted, not dropped.
    /// pytest's and go's summaries, which carried no counts at all, are read
    /// too.
    #[test]
    fn forge_pytest_and_go_summaries_are_read() {
        let one_suite = "Ran 2 tests for test/Counter.t.sol:CounterTest\n\
                          [FAIL: assertion failed] test_Increment() (gas: 0)\n\
                          [PASS] testFuzz_SetNumber(uint256) (runs: 256)\n\
                          Suite result: FAILED. 1 passed; 1 failed; 0 skipped; finished in \
                          7.62ms (7.13ms CPU time)\n\n\
                          Ran 1 test suite in 9.89ms (7.62ms CPU time): 1 tests passed, 1 \
                          failed, 0 skipped (2 total tests)";
        assert_eq!(
            read_counts(one_suite),
            Some(Counts {
                passed: 1,
                failed: 1,
                ignored: 0
            }),
            "a forge suite's own passed/failed, not its `Ran` lines read as unittest's"
        );

        let three_suites = "Ran 2 tests for test/A.t.sol:ATest\n\
                             Suite result: ok. 2 passed; 0 failed; 0 skipped; finished in 1ms\n\n\
                             Ran 2 tests for test/B.t.sol:BTest\n\
                             Suite result: ok. 2 passed; 0 failed; 0 skipped; finished in 1ms\n\n\
                             Ran 2 tests for test/C.t.sol:CTest\n\
                             Suite result: ok. 2 passed; 0 failed; 0 skipped; finished in 1ms\n\n\
                             Ran 3 test suites in 3ms: 6 tests passed, 0 failed, 0 skipped (6 \
                             total tests)";
        assert_eq!(
            read_counts(three_suites),
            Some(Counts {
                passed: 6,
                failed: 0,
                ignored: 0
            }),
            "three suites of two must sum to 6, not 9"
        );

        let pytest = "=========================== FAILURES ============================\n\
                      ___________________________ test_two ____________________________\n\
                      ===== 3 passed, 1 failed in 0.12s =====";
        assert_eq!(
            read_counts(pytest),
            Some(Counts {
                passed: 3,
                failed: 1,
                ignored: 0
            })
        );

        let go_default = "--- FAIL: TestX (0.00s)\n    x_test.go:10: expected 1, got 2\nFAIL\n\
                           exit status 1\nFAIL\texample.com/m\t0.002s";
        assert_eq!(
            read_counts(go_default),
            Some(Counts {
                passed: 0,
                failed: 1,
                ignored: 0
            }),
            "go without -v prints no --- PASS: lines, so passed stays 0"
        );

        let go_verbose = "--- PASS: TestParent (0.00s)\n    --- PASS: TestParent/Sub (0.00s)\n\
                           --- SKIP: TestB (0.00s)\nPASS\nok  \texample.com/m\t0.002s";
        assert_eq!(
            read_counts(go_verbose),
            Some(Counts {
                passed: 1,
                failed: 0,
                ignored: 1
            }),
            "an indented subtest's line is not counted again"
        );
    }

    #[test]
    fn test_files_and_manifests_are_recognised() {
        assert!(is_node_test("strings.test.ts"));
        assert!(is_node_test("camel_case.accept.test.ts"));
        assert!(is_node_test("test-utils.mjs"));
        assert!(!is_node_test("strings.ts"));
        assert!(!is_node_test("latest.js"));
        assert!(is_python_test("test_textutil.py"));
        assert!(is_python_test("textutil_test.py"));
        assert!(!is_python_test("textutil.py"));
        assert!(has_npm_test_script(r#"{"scripts":{"test":"node --test"}}"#));
        assert!(!has_npm_test_script(r#"{"type":"module"}"#));
        assert!(!has_npm_test_script(
            r#"{"scripts":{"test":"echo \"Error: no test specified\" && exit 1"}}"#
        ));
        assert!(has_make_test_target(
            "all: build\n\ntest: build\n\tcargo test\n"
        ));
        assert!(!has_make_test_target(
            ".PHONY: test\nTEST := 1\nbuild:\n\techo test:\n"
        ));
    }

    /// A checkout like the golden path's: Rust at the root, Python tests in
    /// `py/`, TypeScript tests in `ts/` with a package.json and no test script.
    fn polyglot_checkout() -> tempfile::TempDir {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();
        for (file, text) in [
            ("Cargo.toml", "[workspace]\n"),
            ("py/textutil.py", "def shout(s):\n    return s.upper()\n"),
            ("py/test_textutil.py", "import unittest\n"),
            ("ts/package.json", "{\n  \"type\": \"module\"\n}\n"),
            (
                "ts/strings.ts",
                "export const reverse = (s: string) => s;\n",
            ),
            (
                "ts/strings.test.ts",
                "import { test } from \"node:test\";\n",
            ),
            (
                "ts/camel_case.accept.test.ts",
                "import { test } from \"node:test\";\n",
            ),
            ("target/debug/test_stale.py", ""),
            ("ts/node_modules/dep/index.test.js", ""),
        ] {
            let path = root.join(file);
            std::fs::create_dir_all(path.parent().expect("parent")).expect("dir");
            std::fs::write(path, text).expect("write");
        }
        dir
    }

    #[test]
    fn a_scan_finds_the_runners_a_polyglot_checkout_fits() {
        let checkout = polyglot_checkout();

        let project = Project::scan(checkout.path());

        let fits: Vec<&str> = project.fits().iter().map(|runner| runner.build).collect();
        assert_eq!(fits, ["cargo", "node", "unittest"]);
        assert_eq!(project.npm_test_script, None);
        assert_eq!(
            project.node_tests,
            ["ts/camel_case.accept.test.ts", "ts/strings.test.ts"]
        );
        assert_eq!(project.python_tests, ["py/test_textutil.py"]);
        assert_eq!(
            project.unittest_runs(Some("py/test_textutil.py"), checkout.path()),
            [[
                "-m",
                "unittest",
                "discover",
                "-s",
                "py",
                "-k",
                "test_textutil"
            ]]
        );
        assert_eq!(
            node_args(Some("camel_case"), &project.node_tests),
            [
                "--experimental-strip-types",
                "--test",
                "ts/camel_case.accept.test.ts"
            ]
        );
        assert_eq!(
            node_args(Some("shout"), &project.node_tests),
            [
                "--experimental-strip-types",
                "--test",
                "--test-name-pattern=shout"
            ]
        );
    }

    /// A directory of stand-in runners, each a script that records its
    /// arguments in `args.txt` in the directory it runs in, prints `stdout`
    /// and exits with `code`, for a `PATH` holding nothing else.
    #[cfg(unix)]
    fn runners(bin: &Path, scripts: &[(&str, &str, i32)]) -> Vec<(String, OsString)> {
        use std::os::unix::fs::PermissionsExt;

        for (program, stdout, code) in scripts {
            let path = bin.join(program);
            std::fs::write(
                &path,
                format!(
                    "#!/bin/sh\nprintf '%s\\n' \"$@\" > args.txt\nprintf '%s\\n' '{stdout}'\n\
                     exit {code}\n"
                ),
            )
            .expect("write runner");
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755))
                .expect("chmod runner");
        }
        vec![("PATH".to_string(), bin.as_os_str().to_os_string())]
    }

    #[cfg(unix)]
    fn call(arguments: serde_json::Value) -> ToolCall {
        ToolCall::new("c", NAME, arguments)
    }

    /// bug-1f8db7: in the golden-path runs `build: "npm"` failed with "spawn
    /// failed: No such file or directory (os error 2)": npm was not
    /// installed, and the checkout's TypeScript tests run under `node --test`.
    /// The error says which runner is missing and which runners fit.
    #[cfg(unix)]
    #[tokio::test]
    async fn a_missing_runner_is_named_with_the_runners_that_fit() {
        let checkout = polyglot_checkout();
        let bin = tempfile::tempdir().expect("bin");
        let env = runners(bin.path(), &[("node", "# pass 1", 0), ("cargo", "", 0)]);
        let ctx = ToolContext::testing(checkout.path());

        let result = run(
            call(serde_json::json!({ "build": "npm", "filter": "strings.test.ts" })),
            &ctx,
            Some(env),
        )
        .await;

        let ToolResult::Err(ToolError::Other(why)) = &result else {
            panic!("a missing runner is an error: {result:?}");
        };
        assert!(
            why.contains("`npm` is not installed here (it is not on PATH)"),
            "{why}"
        );
        assert!(
            why.contains(
                "This checkout's tests fit build \"cargo\" (`cargo test`) or build \"node\" \
                 (`node --test`)."
            ),
            "{why}"
        );
        assert!(!why.contains("spawn failed"), "{why}");
    }

    /// bug-1f8db7: npm installed is not enough: `npm test` needs a test
    /// script at the checkout's root.
    #[cfg(unix)]
    #[tokio::test]
    async fn npm_without_a_test_script_says_so() {
        let checkout = polyglot_checkout();
        let bin = tempfile::tempdir().expect("bin");
        let env = runners(bin.path(), &[("npm", "", 0), ("node", "", 0)]);
        let ctx = ToolContext::testing(checkout.path());

        let result = run(call(serde_json::json!({ "build": "npm" })), &ctx, Some(env)).await;

        let ToolResult::Err(ToolError::Other(why)) = &result else {
            panic!("npm has nothing to run: {result:?}");
        };
        assert!(why.contains("and there is no package.json"), "{why}");
        assert!(why.contains("build \"node\" (`node --test`)"), "{why}");
        assert!(!checkout.path().join("args.txt").exists(), "npm ran: {why}");
    }

    /// bug-1f8db7: `build: "node"` runs the TypeScript tests the golden path
    /// writes, with type stripping, and a filter naming a file runs that
    /// file. A failing run is a structured result, not a tool failure.
    #[cfg(unix)]
    #[tokio::test]
    async fn node_runs_the_typescript_tests_and_a_failure_is_a_result() {
        let checkout = polyglot_checkout();
        let bin = tempfile::tempdir().expect("bin");
        let env = runners(
            bin.path(),
            &[(
                "node",
                "not ok 1 - camelCase joins words\n# pass 3\n# fail 1",
                1,
            )],
        );
        let ctx = ToolContext::testing(checkout.path());

        let result = run(
            call(serde_json::json!({ "build": "node", "filter": "camel_case" })),
            &ctx,
            Some(env),
        )
        .await;

        let ToolResult::Ok { is_structured, .. } = &result else {
            panic!("a failing run is a result: {result:?}");
        };
        assert!(is_structured);
        let payload: Value = serde_json::from_str(&result.text_content()).expect("json");
        assert_eq!(payload["build"], "node");
        assert_eq!(payload["status"], "failed");
        assert_eq!(payload["exit_code"], 1);
        assert_eq!(payload["passed"], 3);
        assert_eq!(payload["failed"], 1);
        assert!(
            payload["output"]
                .as_str()
                .is_some_and(|output| output.contains("not ok 1 - camelCase joins words")),
            "{payload}"
        );
        let args = std::fs::read_to_string(checkout.path().join("args.txt")).expect("args");
        assert_eq!(
            args,
            "--experimental-strip-types\n--test\nts/camel_case.accept.test.ts\n"
        );
    }

    /// What a `run_tests` call for `build` and `filter` ran in `checkout`:
    /// the stand-in runner's arguments, one per line; or the error that
    /// refused the call, when nothing ran.
    #[cfg(unix)]
    async fn ran_with(
        checkout: &Path,
        env: &[(String, OsString)],
        build: &str,
        filter: &str,
    ) -> Result<String, String> {
        let recorded = checkout.join("args.txt");
        let _ = std::fs::remove_file(&recorded);
        let ctx = ToolContext::testing(checkout);
        let result = run(
            call(serde_json::json!({ "build": build, "filter": filter })),
            &ctx,
            Some(env.to_vec()),
        )
        .await;
        match result {
            ToolResult::Err(ToolError::Other(why)) => {
                assert!(!recorded.exists(), "{build} ran although refused: {why}");
                Err(why)
            }
            ToolResult::Ok { .. } => Ok(std::fs::read_to_string(&recorded).expect("args")),
            other => panic!("{build} {filter}: {other:?}"),
        }
    }

    /// bug-83d420: `run_tests` added its filter to every runner's command as
    /// one more word, so go read it as a package, make as a target, pytest as
    /// a path, and cargo flags in it reached cargo as one word (`invalid
    /// character ' ' in package name` in the 2026-10-19 golden-path run 3).
    /// Each runner now gets it as its own option, and a filter a runner
    /// cannot take is refused before anything runs.
    #[cfg(unix)]
    #[tokio::test]
    async fn a_filter_reaches_each_runner_as_its_own_option() {
        let checkout = polyglot_checkout();
        let root = checkout.path();
        std::fs::write(
            root.join("package.json"),
            r#"{"scripts":{"test":"node --test"}}"#,
        )
        .expect("package.json");
        std::fs::write(root.join("Makefile"), "test:\n\tcargo test $(TEST)\n").expect("make");
        let bin = tempfile::tempdir().expect("bin");
        let programs = ["cargo", "go", "forge", "python3", "npm", "make", "node"];
        let scripts: Vec<(&str, &str, i32)> = programs.iter().map(|p| (*p, "", 0)).collect();
        let env = runners(bin.path(), &scripts);
        let absolute = format!("{}/py/test_textutil.py::test_shout", root.display());

        for (build, filter, expected) in [
            ("cargo", "shout", "test\n--workspace\nshout\n"),
            (
                "cargo",
                "-p alpha --test accept_word_count",
                "test\n-p\nalpha\n--test\naccept_word_count\n",
            ),
            (
                "cargo",
                "--lib -- --exact tests::shout",
                "test\n--workspace\n--lib\n--\n--exact\ntests::shout\n",
            ),
            ("go", "TestShout", "test\n./...\n-run=TestShout\n"),
            ("go", "./strings", "test\n./strings\n"),
            (
                "forge",
                "test_Increment",
                "test\n--match-test=test_Increment\n",
            ),
            (
                "forge",
                "test/Counter.t.sol",
                "test\n--match-path=test/Counter.t.sol\n",
            ),
            (
                "pytest",
                "shout and not slow",
                "-m\npytest\n-k\nshout and not slow\n",
            ),
            (
                "pytest",
                "test_textutil.py::test_shout",
                "-m\npytest\npy/test_textutil.py::test_shout\n",
            ),
            (
                "pytest",
                "py/test_textutil.py",
                "-m\npytest\npy/test_textutil.py\n",
            ),
            (
                "pytest",
                absolute.as_str(),
                "-m\npytest\npy/test_textutil.py::test_shout\n",
            ),
            (
                "unittest",
                "test_textutil",
                "-m\nunittest\ndiscover\n-s\npy\n-k\ntest_textutil\n",
            ),
            (
                "node",
                "shout",
                "--experimental-strip-types\n--test\n--test-name-pattern=shout\n",
            ),
            ("npm", "strings", "test\n--\nstrings\n"),
            ("make", "shout", "test\nTEST=shout\n"),
        ] {
            assert_eq!(
                ran_with(root, &env, build, filter).await.as_deref(),
                Ok(expected),
                "{build} with filter `{filter}`"
            );
        }

        for (build, filter, why) in [
            (
                "cargo",
                "--config build.rustc-wrapper=/bin/sh",
                "gives cargo `--config`",
            ),
            (
                "cargo",
                "-p alpha --manifest-path ../other/Cargo.toml",
                "gives cargo `--manifest-path`",
            ),
            ("cargo", "shout --exact", "go after `--`"),
            ("pytest", "-p some_plugin", "begins with `-`"),
            ("pytest", "../other/test_x.py", "outside the checkout"),
            ("npm", "--require=./x.js", "begins with `-`"),
            ("make", "shout; touch pwned", "must be a plain test name"),
        ] {
            let refused = ran_with(root, &env, build, filter)
                .await
                .expect_err("the filter is refused");
            assert!(refused.contains(why), "{build} `{filter}`: {refused}");
        }

        // A Makefile that reads no TEST takes no filter; the runners that
        // fit are named.
        std::fs::write(root.join("Makefile"), "test:\n\tcargo test\n").expect("make");
        let refused = ran_with(root, &env, "make", "shout")
            .await
            .expect_err("make takes no filter");
        assert!(refused.contains("reads no TEST variable"), "{refused}");
        assert!(
            refused.contains("build \"cargo\" (`cargo test`)"),
            "{refused}"
        );
    }

    /// bug-1f8db7: a call naming no runner gets the one the checkout fits
    /// that is installed: here `unittest`, from the directory its tests are
    /// in, as the golden path's own checks run them.
    #[cfg(unix)]
    #[tokio::test]
    async fn a_call_naming_no_runner_gets_one_the_checkout_fits() {
        let checkout = tempfile::tempdir().expect("checkout");
        let tests = checkout.path().join("py");
        std::fs::create_dir_all(&tests).expect("py");
        std::fs::write(tests.join("test_textutil.py"), "import unittest\n").expect("test");
        let bin = tempfile::tempdir().expect("bin");
        let env = runners(bin.path(), &[("python3", "Ran 2 tests in 0.001s", 0)]);
        let ctx = ToolContext::testing(checkout.path());

        let result = run(call(serde_json::json!({})), &ctx, Some(env)).await;

        let payload: Value = serde_json::from_str(&result.text_content())
            .unwrap_or_else(|error| panic!("{error}: {result:?}"));
        assert_eq!(payload["build"], "unittest");
        assert_eq!(payload["status"], "ok");
        assert_eq!(payload["passed"], 2);
        let args = std::fs::read_to_string(checkout.path().join("args.txt")).expect("args");
        assert_eq!(args, "-m\nunittest\ndiscover\n-s\npy\n");
    }

    /// bug-1f8db7: `python3 -m pytest` without pytest says pytest is missing
    /// and that the checkout's tests fit `unittest`, rather than reporting a
    /// test failure.
    #[cfg(unix)]
    #[tokio::test]
    async fn a_missing_pytest_module_is_named() {
        let checkout = polyglot_checkout();
        let bin = tempfile::tempdir().expect("bin");
        let env = runners(
            bin.path(),
            &[("python3", "/usr/bin/python3: No module named pytest", 1)],
        );
        let ctx = ToolContext::testing(checkout.path());

        let result = run(
            call(serde_json::json!({ "build": "pytest" })),
            &ctx,
            Some(env),
        )
        .await;

        let ToolResult::Err(ToolError::Other(why)) = &result else {
            panic!("pytest is missing: {result:?}");
        };
        assert!(why.contains("pytest is not installed for python3"), "{why}");
        assert!(
            why.contains("build \"unittest\" (`python3 -m unittest`)"),
            "{why}"
        );
    }

    /// bug-1f8db7: a failing `cargo test` is a result with the failure in
    /// it. Cargo prints the failure last; the 400 characters of the old
    /// error showed only the passing crates before it.
    #[cfg(unix)]
    #[tokio::test]
    async fn a_failing_cargo_test_returns_its_failure() {
        let checkout = polyglot_checkout();
        let bin = tempfile::tempdir().expect("bin");
        let mut stdout = String::new();
        for krate in 0..30 {
            stdout.push_str(&format!(
                "running 1 test\ntest tests::case_{krate} ... ok\n\ntest result: ok. 1 \
                 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out\n\n"
            ));
        }
        stdout.push_str(
            "running 1 test\ntest tests::shout ... FAILED\n\nfailures:\n\n---- tests::shout \
             stdout ----\nthread tests::shout panicked: expected HELLO, got hello\n\ntest \
             result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out",
        );
        let env = runners(bin.path(), &[("cargo", stdout.as_str(), 101)]);
        let ctx = ToolContext::testing(checkout.path());

        let result = run(
            call(serde_json::json!({ "build": "cargo" })),
            &ctx,
            Some(env),
        )
        .await;

        let ToolResult::Ok { .. } = &result else {
            panic!("a failing run is a result: {result:?}");
        };
        let payload: Value = serde_json::from_str(&result.text_content()).expect("json");
        assert_eq!(payload["status"], "failed");
        assert_eq!(payload["exit_code"], 101);
        assert_eq!(payload["passed"], 30);
        assert_eq!(payload["failed"], 1);
        let output = payload["output"].as_str().expect("output");
        assert!(
            output.contains("thread tests::shout panicked: expected HELLO, got hello"),
            "{output}"
        );
        assert!(output.len() <= TAIL_BYTES, "{}", output.len());
    }

    /// gap-770cb5: `build: "checks"` runs the checks dispatch left for the
    /// call's attempt, in the checkout, and stops at the first that fails
    /// with the end of its output; once the work is fixed they pass. A call
    /// of no attempt, or of one with no checks, says so. bug-1f8db7: a
    /// failing check is a result, as a failing test run is.
    #[cfg(unix)]
    #[tokio::test]
    async fn run_tests_runs_the_task_checks() {
        use super::super::task_checks::{TaskCheck, task_checks_path};

        let dir = tempfile::tempdir().expect("tempdir");
        let mut ctx = ToolContext::testing(dir.path());
        let call = || ToolCall::new("c", NAME, serde_json::json!({"build": "checks"}));
        let result = run(call(), &ctx, None).await;
        assert!(
            matches!(&result, ToolResult::Err(ToolError::Other(why)) if why.contains("no task")),
            "{result:?}"
        );
        ctx.correlation.attempt_id = "run-1:plan-a:T1:1".to_string();
        let result = run(call(), &ctx, None).await;
        assert!(
            matches!(&result, ToolResult::Err(ToolError::Other(why)) if why.contains("no checks")),
            "{result:?}"
        );

        let checks = vec![
            TaskCheck {
                label: "verify[0:build]".to_string(),
                command: "echo building; test -f fixed.txt || { echo 'error: not fixed' >&2; \
                          exit 2; }"
                    .to_string(),
                timeout_ms: Some(30_000),
            },
            TaskCheck {
                label: "verify[1:test]".to_string(),
                command: "echo tested".to_string(),
                timeout_ms: None,
            },
        ];
        let path = task_checks_path(&ctx.immune_root_path, &ctx.correlation.attempt_id);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("dir");
        std::fs::write(&path, serde_json::to_vec(&checks).expect("json")).expect("checks");

        let result = run(call(), &ctx, None).await;
        let ToolResult::Ok { .. } = &result else {
            panic!("the unfixed work's failing check is a result: {result:?}");
        };
        let payload: Value = serde_json::from_str(&result.text_content()).expect("json");
        assert_eq!(payload["status"], "failed");
        let summary = payload["summary"].as_str().expect("summary");
        assert!(
            summary.contains("check 1 of 2 (verify[0:build]) failed (exit 2)"),
            "{summary}"
        );
        assert_eq!(payload["output"], "building\nerror: not fixed");

        std::fs::write(dir.path().join("fixed.txt"), "ok\n").expect("fix");
        let result = run(call(), &ctx, None).await;
        let ToolResult::Ok { .. } = &result else {
            panic!("the fixed work passes: {result:?}");
        };
        let text = format!("{result:?}");
        assert!(text.contains("verify[1:test]"), "{text}");
        assert!(text.contains(r#"\"status\":\"ok\""#), "{text}");
    }

    /// reg-d2deba: a run of an attempt's checks in a worktree of its own
    /// records what it changed in the person's checkout, the workspace root,
    /// as a check writing through a path compiled in from that checkout does;
    /// roko's own `.roko/` there is no change of the run's.
    #[cfg(unix)]
    #[tokio::test]
    async fn a_check_run_records_what_it_changed_in_the_persons_checkout() {
        use super::super::task_checks::{
            TaskCheck, read_check_runs, task_check_runs_path, task_checks_path,
        };

        let git = |dir: &std::path::Path, args: &[&str]| {
            let status = std::process::Command::new("git")
                .current_dir(dir)
                .args(args)
                .env_remove("GIT_DIR")
                .env_remove("GIT_WORK_TREE")
                .env_remove("GIT_INDEX_FILE")
                .status()
                .expect("git");
            assert!(status.success(), "git {args:?}");
        };
        let person = tempfile::tempdir().expect("the person's checkout");
        let worktree = tempfile::tempdir().expect("the attempt's worktree");
        for dir in [person.path(), worktree.path()] {
            git(dir, &["init", "--quiet"]);
        }
        let mut ctx = ToolContext::testing(worktree.path());
        ctx.immune_root_path = person.path().to_path_buf();
        ctx.correlation.attempt_id = "run-1:plan-a:T1:1".to_string();
        let shot = person.path().join("art/shot.png");
        let checks = vec![TaskCheck {
            label: "verify[0:test]".to_string(),
            command: format!(
                "mkdir -p '{}' && printf png > '{}'",
                person.path().join("art").display(),
                shot.display()
            ),
            timeout_ms: Some(30_000),
        }];
        let path = task_checks_path(&ctx.immune_root_path, &ctx.correlation.attempt_id);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("dir");
        std::fs::write(&path, serde_json::to_vec(&checks).expect("json")).expect("checks");
        let call = ToolCall::new("c", NAME, serde_json::json!({"build": "checks"}));

        let result = run(call, &ctx, None).await;

        assert!(matches!(result, ToolResult::Ok { .. }), "{result:?}");
        assert!(shot.is_file(), "the check wrote into the person's checkout");
        let runs = read_check_runs(&task_check_runs_path(
            &ctx.immune_root_path,
            &ctx.correlation.attempt_id,
        ));
        assert_eq!(runs.len(), 1, "{runs:?}");
        assert_eq!(runs[0].passed, ["verify[0:test]"]);
        assert_eq!(runs[0].checkout_written, ["art/shot.png"]);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn run_tests_env_excludes_provider_keys() {
        let dir = tempfile::tempdir().expect("tempdir");
        // The test target records the environment it runs with.
        std::fs::write(
            dir.path().join("Makefile"),
            "test:\n\tenv > child-env.txt\n",
        )
        .expect("write Makefile");
        let path = std::env::var_os("PATH")
            .filter(|path| !path.is_empty())
            .unwrap_or_else(|| "/usr/bin:/bin".into());
        let mut parent = vec![("PATH".to_string(), path)];
        for (name, value) in [
            ("CARGO_HOME", "/tmp/tests-cargo"),
            ("OPENAI_API_KEY", "sk-test-not-real"),
            ("ANTHROPIC_API_KEY", "sk-ant-test-not-real"),
            ("MY_SECRET_TOKEN", "tok-test-not-real"),
        ] {
            parent.push((name.to_string(), OsString::from(value)));
        }
        let call = ToolCall::new("c", NAME, serde_json::json!({"build": "make"}));
        let ctx = ToolContext::testing(dir.path());

        let result = run(call, &ctx, Some(parent)).await;

        assert!(
            matches!(result, ToolResult::Ok { .. }),
            "make test failed: {result:?}"
        );
        let env = std::fs::read_to_string(dir.path().join("child-env.txt"))
            .expect("make test records its env");
        for leaked in [
            "sk-test-not-real",
            "sk-ant-test-not-real",
            "tok-test-not-real",
        ] {
            assert!(!env.contains(leaked), "{leaked} leaked:\n{env}");
        }
        assert!(
            env.contains("CARGO_HOME=/tmp/tests-cargo"),
            "toolchain env missing:\n{env}"
        );
    }
}
