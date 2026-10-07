//! The red-on-base check behind SQ06 and HF3 (3214, S07.9): a port of
//! speclint's `dynamic.py`, whose tests pin down the contract.
//!
//! [`check_plans`] runs each implementer task's pinned acceptance tests and
//! verify steps twice on a clean checkout of the plan's base commit, and
//! classifies the task ([`TaskCheck`]):
//!
//! - `fail`: the same step fails on the base in both runs. SQ06 = 1.
//! - `pass`: every step passes on the base in both runs. HF3, unless the task
//!   pins no acceptance test and every step declares `expect = "pass_on_base"`.
//! - `unknown`: the runs disagree, a step times out or cannot start, a
//!   `pass_on_base` step fails, the plan has no known base, or the steps that
//!   could tell run cargo and [`CargoSteps::Skip`] left them to the batch gate.
//!
//! Tasks that are not implementers, or have neither a pinned acceptance test
//! nor a verify step, are not run. A run stops at its first failing step, as
//! `plan run` does. Each step runs as `bash -o pipefail -c <command>` in the
//! workspace root of the base checkout, in a process group of its own, for at
//! most the timeout (120 s by default) or its own `timeout_ms` when that is
//! shorter; a timeout kills the whole group. Cargo steps build into a
//! `CARGO_TARGET_DIR` inside the scratch directory, so the reset between runs
//! keeps the build.
//!
//! The base. With [`RedOnBaseOptions::base`] every plan is checked against
//! that commit. Without it, a plan that has not run is checked against HEAD,
//! and the rest are unknown: archived plans, and plans whose `files` changed
//! in the commit that added the plan or later, since their work has probably
//! landed.
//!
//! Safety. No step runs in the checkout. Each base commit gets one
//! `git worktree add --detach` checkout in a new scratch directory outside
//! every worktree of the repo. Before each run the checkout is reset to the
//! base (`git reset --hard`, `git clean -ffdx`) and the plan's directory is
//! copied in, so an uncommitted plan brings its own files. When the check
//! ends, also on an error, SIGINT or SIGTERM, it removes the worktrees and
//! the scratch directory it created and nothing else: it creates no branch,
//! never prunes, and never touches another worktree.
//!
//! [`RedOnBaseOptions::fixture`] checks a plain directory instead: it is
//! snapshotted into a throwaway one-commit repo inside the scratch directory,
//! and that commit is the base.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fs::File;
use std::io::{Read as _, Seek as _, SeekFrom};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Output, Stdio};
use std::sync::atomic::{AtomicI32, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use anyhow::{Context as _, Result, anyhow, bail};
use roko_core::config::SpecQualityConfig;
use roko_gate::spec_quality::{RedOnBase, normpath, runs_program};
use serde::Serialize;
use sha2::{Digest, Sha256};
use toml::{Table, Value};

use crate::task_accept::{AcceptFile, PinnedAccept, pinned_verify_step};

/// How many times each task's steps run on the base.
pub const RUNS: usize = 2;

/// The longest one step may run, unless its own `timeout_ms` is shorter.
pub const STEP_TIMEOUT: Duration = Duration::from_secs(120);

/// Variables that point git at another repository. Neither the steps nor the
/// check's own git commands see them, so nothing reaches the checkout through
/// them.
const GIT_REDIRECTS: [&str; 8] = [
    "GIT_DIR",
    "GIT_WORK_TREE",
    "GIT_INDEX_FILE",
    "GIT_OBJECT_DIRECTORY",
    "GIT_ALTERNATE_OBJECT_DIRECTORIES",
    "GIT_COMMON_DIR",
    "GIT_NAMESPACE",
    "GIT_PREFIX",
];

/// How much of a failed step's output a result keeps.
const TAIL_BYTES: u64 = 4096;
const TAIL_LINES: usize = 5;
const TAIL_CHARS: usize = 400;

/// How often a running step is polled for its exit, its timeout and an
/// interrupt.
const POLL: Duration = Duration::from_millis(20);

/// The signal that interrupted a running check, or 0.
static INTERRUPTED: AtomicI32 = AtomicI32::new(0);

/// What the check does with verify steps that run cargo.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum CargoSteps {
    /// Run them like any other step.
    #[default]
    Run,
    /// Leave them to the batch gate, which proves cargo checks red (D14 as
    /// amended, gap-0ee70b). A task whose steps all run cargo is
    /// `skipped_cargo`, and so is one whose other steps pass on the base.
    Skip,
}

/// How [`check_plans`] runs.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RedOnBaseOptions {
    /// The commit to check every plan against; `None` checks a plan that has
    /// not run against HEAD.
    pub base: Option<String>,
    /// The longest one step may run; [`STEP_TIMEOUT`] when `None`.
    pub timeout: Option<Duration>,
    /// An existing directory outside every checkout of the repo, for the base
    /// checkouts; the system temp directory when `None`.
    pub scratch: Option<PathBuf>,
    /// Check a plain directory against a one-commit snapshot of it.
    pub fixture: bool,
    /// What to do with steps that run cargo.
    pub cargo: CargoSteps,
}

impl RedOnBaseOptions {
    /// The options `[spec_quality]` sets: its step timeout, and the steps
    /// that run cargo left to the batch gate unless `red_on_base_cargo` is
    /// set (gap-0ee70b).
    pub fn from_config(config: &SpecQualityConfig) -> Self {
        Self {
            timeout: Some(Duration::from_secs(config.red_on_base_timeout_secs)),
            cargo: if config.red_on_base_cargo {
                CargoSteps::Run
            } else {
                CargoSteps::Skip
            },
            ..Self::default()
        }
    }
}

/// The red-on-base results the spec gate acts on before `plan run`
/// (gap-0ee70b): every implementer task's shell checks run on the base, and
/// a check that passes there is HF3. Empty when `[spec_quality]` turns the
/// check off, and when it cannot start (a workspace outside git, say), since
/// a check that is not proven green does not block. An interrupt comes back
/// as the error, after the check removed what it created.
pub fn gate_results(
    files: &[PathBuf],
    workdir: &Path,
    config: &SpecQualityConfig,
) -> std::result::Result<BTreeMap<(String, String), RedOnBase>, Interrupted> {
    if !config.is_on() || !config.red_on_base {
        return Ok(BTreeMap::new());
    }
    match check_plans(files, workdir, &RedOnBaseOptions::from_config(config)) {
        Ok(report) => Ok(report.results()),
        Err(error) => match error.downcast_ref::<Interrupted>() {
            Some(interrupted) => Err(*interrupted),
            None => {
                let error = format!("{error:#}");
                tracing::warn!(%error, "spec gate: the red-on-base check could not run");
                Ok(BTreeMap::new())
            }
        },
    }
}

/// Why a task's red-on-base result is what it is: speclint's `OUTCOMES`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    /// The same step fails on the base in both runs.
    Fail,
    /// Every step passes on the base in both runs.
    Pass,
    /// The runs disagree.
    Flaky,
    /// A step timed out.
    Timeout,
    /// A step could not start, or a pinned acceptance test is missing.
    Error,
    /// A step that declares `expect = "pass_on_base"` fails on the base.
    BaseBroken,
    /// The plan has no known base.
    NoBase,
    /// Not run: not an implementer, or no verify step.
    NotRun,
    /// The steps that could tell run cargo, which the batch gate proves.
    SkippedCargo,
}

impl Outcome {
    /// Every outcome, in the order the summary lists them.
    pub const ALL: [Self; 9] = [
        Self::Fail,
        Self::Pass,
        Self::Flaky,
        Self::Timeout,
        Self::Error,
        Self::BaseBroken,
        Self::NoBase,
        Self::NotRun,
        Self::SkippedCargo,
    ];

    /// The outcome and what it means for the score, as the summary prints it.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Fail => "fail: red on the base (SQ06 = 1)",
            Self::Pass => "pass: every step passes (HF3)",
            Self::Flaky => "unknown: the runs disagree",
            Self::Timeout => "unknown: a step timed out",
            Self::Error => "unknown: a step could not start",
            Self::BaseBroken => "unknown: a pass_on_base step fails",
            Self::NoBase => "unknown: no known base",
            Self::NotRun => "not run: no implementer verify",
            Self::SkippedCargo => "unknown: skipped: cargo (batch gate)",
        }
    }
}

/// One step of one run.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct StepResult {
    /// The step's number, from 1: pinned acceptance tests first, then the
    /// task's verify steps.
    pub step: usize,
    /// The `[task.accept]` source that a pinned test's step runs.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accept: Option<String>,
    /// The exit code, or the negated signal that ended the step; `None` when
    /// it timed out or could not start.
    pub exit: Option<i32>,
    /// How long it ran, in seconds.
    pub secs: f64,
    /// Whether it hit its timeout.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub timed_out: bool,
    /// Why it could not start.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// The last lines of its output, when it failed or timed out.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tail: Option<String>,
}

/// One task's red-on-base result: speclint's `red_on_base_detail`.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct TaskCheck {
    /// The `tasks.toml` path, relative to the workspace root when inside it.
    pub plan_path: String,
    /// The task id.
    pub task_id: String,
    /// The result the score uses.
    pub red_on_base: RedOnBase,
    /// Why the result is what it is.
    pub outcome: Outcome,
    /// What happened, in words.
    pub reason: String,
    /// The base commit; `None` when the plan has none.
    pub base: Option<String>,
    /// Why that base: `--base <rev>`, `HEAD: the plan has not run`, or why
    /// the plan has none.
    pub base_reason: String,
    /// Per run, the steps that ran.
    pub runs: Vec<Vec<StepResult>>,
}

impl TaskCheck {
    fn new(plan_path: &str, task_id: &str, outcome: Outcome, reason: String) -> Self {
        Self {
            plan_path: plan_path.to_string(),
            task_id: task_id.to_string(),
            red_on_base: RedOnBase::Unknown,
            outcome,
            reason,
            base: None,
            base_reason: String::new(),
            runs: Vec::new(),
        }
    }
}

/// The red-on-base results of a set of plans.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct RedOnBaseReport {
    /// What the check did with steps that run cargo.
    pub cargo: CargoSteps,
    /// One check per task, in file order.
    pub checks: Vec<TaskCheck>,
}

impl RedOnBaseReport {
    /// The results by (plan path, task id), as
    /// [`roko_gate::spec_quality::lint_files_with`] and the spec gate take
    /// them.
    pub fn results(&self) -> BTreeMap<(String, String), RedOnBase> {
        self.checks
            .iter()
            .map(|check| {
                let key = (check.plan_path.clone(), check.task_id.clone());
                (key, check.red_on_base)
            })
            .collect()
    }

    /// The summary `plan validate --dynamic` prints: tasks by outcome, then
    /// the bases the plans were checked against.
    pub fn render_text(&self) -> String {
        let mut text = String::from("red on base (tasks)\n");
        for outcome in Outcome::ALL {
            let count = self
                .checks
                .iter()
                .filter(|check| check.outcome == outcome)
                .count();
            if count > 0 {
                text.push_str(&format!("  {:<40}{count:>6}\n", outcome.label()));
            }
        }
        let mut bases: BTreeMap<&str, Option<&str>> = BTreeMap::new();
        for check in &self.checks {
            if check.outcome != Outcome::NotRun {
                bases.insert(&check.plan_path, check.base.as_deref());
            }
        }
        let checked: BTreeSet<&str> = bases.values().flatten().map(|sha| short(sha)).collect();
        let checked: Vec<&str> = checked.into_iter().collect();
        text.push_str(&format!(
            "  plans checked: {} (base {}); without a known base: {}",
            bases.values().filter(|sha| sha.is_some()).count(),
            if checked.is_empty() {
                "-".to_string()
            } else {
                checked.join(", ")
            },
            bases.values().filter(|sha| sha.is_none()).count(),
        ));
        text
    }
}

/// The check stopped on SIGINT or SIGTERM, after it removed what it created.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Interrupted {
    /// The signal's number.
    pub signal: i32,
}

impl fmt::Display for Interrupted {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "the red-on-base check was interrupted by signal {}",
            self.signal
        )
    }
}

impl std::error::Error for Interrupted {}

/// Check every implementer task of `files` on its base. `root` is the
/// workspace the plans run in.
///
/// Fails when the check cannot start (`root` is not in a git work tree and
/// `fixture` is off, the base is unknown, or the scratch directory is
/// unsafe), when a git command fails, and with [`Interrupted`] on SIGINT or
/// SIGTERM. In every case it first removes what it created.
pub fn check_plans(
    files: &[PathBuf],
    root: &Path,
    options: &RedOnBaseOptions,
) -> Result<RedOnBaseReport> {
    let root = std::fs::canonicalize(root)
        .with_context(|| format!("the workspace root {} does not exist", root.display()))?;
    let _signals = InterruptGuard::install();
    let mut trees = BaseTrees::open(&root, options.scratch.as_deref(), options.fixture)?;
    let head = trees.resolve(options.base.as_deref().unwrap_or("HEAD"))?;
    let mut checks = Vec::new();
    for file in files {
        let path = std::fs::canonicalize(file).unwrap_or_else(|_| file.clone());
        // A plan that does not parse is the linter's to report.
        let Some(data) = std::fs::read_to_string(&path)
            .ok()
            .and_then(|text| toml::from_str::<Table>(&text).ok())
        else {
            continue;
        };
        checks.extend(check_plan(&path, &data, &mut trees, &head, options)?);
    }
    Ok(RedOnBaseReport {
        cargo: options.cargo,
        checks,
    })
}

/// The checks of one plan's tasks.
fn check_plan(
    path: &Path,
    data: &Table,
    trees: &mut BaseTrees,
    head: &str,
    options: &RedOnBaseOptions,
) -> Result<Vec<TaskCheck>> {
    let rel = relative_path(path, &trees.root);
    let plan_dir = path.parent().unwrap_or(&trees.root).to_path_buf();
    let tasks: Vec<&Table> = tables(data.get("task")).collect();
    let mut plan_base: Option<(Option<String>, String)> = None;
    let mut checks = Vec::new();
    for task in &tasks {
        let id = text(task.get("id"));
        let role = text(task.get("role"));
        let role = match role.trim() {
            "" => "implementer",
            role => role,
        };
        if role != "implementer" {
            let reason = format!("not run: a {role} task");
            checks.push(TaskCheck::new(&rel, &id, Outcome::NotRun, reason));
            continue;
        }
        if tables(task.get("verify")).next().is_none() && accept_files(task).is_empty() {
            let reason = "not run: no verify step".to_string();
            checks.push(TaskCheck::new(&rel, &id, Outcome::NotRun, reason));
            continue;
        }
        // Only regression checks (`expect = "pass_on_base"`), such as the
        // workspace gates `roko run` writes: the task cannot be HF3, so its
        // steps are not run on the base.
        let regression_only = accept_files(task).is_empty()
            && tables(task.get("verify")).all(|step| text(step.get("expect")) == "pass_on_base");
        if regression_only {
            let reason = "not run: every step is a regression check (pass_on_base)".to_string();
            checks.push(TaskCheck::new(&rel, &id, Outcome::NotRun, reason));
            continue;
        }
        let (sha, why) = match &plan_base {
            Some(known) => known.clone(),
            None => {
                let known = match &options.base {
                    Some(base) => (Some(head.to_string()), format!("--base {base}")),
                    None => trees.plan_base(path, &rel, &tasks, head)?,
                };
                plan_base = Some(known.clone());
                known
            }
        };
        let mut check = match sha {
            Some(sha) => check_task(&rel, &id, task, trees, &sha, &plan_dir, options)?,
            None => TaskCheck::new(&rel, &id, Outcome::NoBase, why.clone()),
        };
        check.base_reason = why;
        checks.push(check);
    }
    Ok(checks)
}

/// One step to run on the base.
struct Step {
    /// Its number among the task's steps, from 1.
    number: usize,
    command: String,
    timeout: Duration,
    /// `expect`: `pass_on_base` for a regression step.
    expect: String,
    /// The `[task.accept]` source a pinned test's step runs.
    accept: Option<String>,
}

/// Run a task's pinned acceptance tests, then its verify steps, on the base,
/// twice, and classify the result.
fn check_task(
    rel: &str,
    id: &str,
    task: &Table,
    trees: &mut BaseTrees,
    sha: &str,
    plan_dir: &Path,
    options: &RedOnBaseOptions,
) -> Result<TaskCheck> {
    let mut check = TaskCheck::new(rel, id, Outcome::Error, String::new());
    check.base = Some(sha.to_string());
    let accept = accept_files(task);
    let missing: Vec<&str> = accept
        .iter()
        .filter(|entry| !plan_dir.join(&entry.src).is_file())
        .map(|entry| entry.src.as_str())
        .collect();
    if !missing.is_empty() {
        check.reason = format!(
            "pinned acceptance test missing from the plan: {}",
            missing.join(", ")
        );
        return Ok(check);
    }
    let cap = options.timeout.unwrap_or(STEP_TIMEOUT);
    let steps = task_steps(
        id,
        task,
        &accept,
        &trees.plan_copy(sha, plan_dir),
        plan_dir,
        cap,
    )?;
    let (steps, skipped): (Vec<Step>, Vec<Step>) = steps.into_iter().partition(|step| {
        options.cargo == CargoSteps::Run || !runs_program(&step.command, "cargo")
    });
    if steps.is_empty() {
        check.outcome = Outcome::SkippedCargo;
        check.reason =
            "skipped: cargo: every step runs cargo, which the batch gate proves red".to_string();
        return Ok(check);
    }
    check.runs = run_steps(&steps, trees, sha, plan_dir)?;
    let (red_on_base, outcome, reason) = classify(&steps, &check.runs);
    if outcome == Outcome::Pass && !skipped.is_empty() {
        let numbers: Vec<String> = skipped.iter().map(|step| step.number.to_string()).collect();
        check.outcome = Outcome::SkippedCargo;
        check.reason = format!(
            "skipped: cargo: the other steps pass on the base, and steps {} run cargo, which \
             the batch gate proves red",
            numbers.join(", ")
        );
    } else {
        check.red_on_base = red_on_base;
        check.outcome = outcome;
        check.reason = reason;
    }
    Ok(check)
}

/// The task's steps in the order `plan run` runs them: its pinned acceptance
/// tests, then its verify steps. A pinned test runs the plan's `src` from
/// `plan_copy`, the plan's directory in the base checkout, in place of the
/// pinned copy, since the base has no pin store.
fn task_steps(
    id: &str,
    task: &Table,
    accept: &[AcceptFile],
    plan_copy: &Path,
    plan_dir: &Path,
    cap: Duration,
) -> Result<Vec<Step>> {
    let mut steps = Vec::new();
    for entry in accept {
        let bytes = std::fs::read(plan_dir.join(&entry.src))
            .with_context(|| format!("read the pinned acceptance test {}", entry.src))?;
        let pinned = PinnedAccept {
            stored: plan_copy.join(&entry.src),
            sha256: format!("{:x}", Sha256::digest(&bytes)),
        };
        steps.push(Step {
            number: steps.len() + 1,
            command: pinned_verify_step(id, entry, &pinned).command,
            timeout: limit(entry.timeout_ms, cap),
            expect: String::new(),
            accept: Some(entry.src.clone()),
        });
    }
    for verify in tables(task.get("verify")) {
        let timeout_ms = verify
            .get("timeout_ms")
            .and_then(Value::as_integer)
            .and_then(|ms| u64::try_from(ms).ok());
        steps.push(Step {
            number: steps.len() + 1,
            command: text(verify.get("command")),
            timeout: limit(timeout_ms, cap),
            expect: text(verify.get("expect")),
            accept: None,
        });
    }
    Ok(steps)
}

/// A step's own `timeout_ms` when it is positive and shorter than `cap`.
fn limit(timeout_ms: Option<u64>, cap: Duration) -> Duration {
    timeout_ms
        .filter(|ms| *ms > 0)
        .map_or(cap, |ms| cap.min(Duration::from_millis(ms)))
}

/// The task's `[task.accept]` tests that compile into a verify step when the
/// plan loads: entries that name a `src`, a `dest` and a `runner` and have a
/// positive `count` (speclint's `accept_tests`).
fn accept_files(task: &Table) -> Vec<AcceptFile> {
    let entries = task
        .get("accept")
        .and_then(Value::as_table)
        .and_then(|accept| accept.get("files"));
    tables(entries)
        .filter_map(|entry| {
            let field = |key: &str| text(entry.get(key)).trim().to_string();
            let (src, dest, runner) = (field("src"), field("dest"), text(entry.get("runner")));
            let count = entry.get("count").and_then(Value::as_integer)?;
            let count = u32::try_from(count).ok().filter(|count| *count > 0)?;
            if src.is_empty() || dest.is_empty() || runner.trim().is_empty() {
                return None;
            }
            let timeout_ms = entry
                .get("timeout_ms")
                .and_then(Value::as_integer)
                .and_then(|ms| u64::try_from(ms).ok());
            Some(AcceptFile {
                src,
                dest,
                runner,
                count,
                timeout_ms,
            })
        })
        .collect()
}

/// Run `steps` [`RUNS`] times, each run from the clean base and stopping at
/// its first failing step. A step that timed out or could not start ends the
/// check: the task is unknown whatever another run does.
fn run_steps(
    steps: &[Step],
    trees: &mut BaseTrees,
    sha: &str,
    plan_dir: &Path,
) -> Result<Vec<Vec<StepResult>>> {
    let mut runs = Vec::new();
    for _ in 0..RUNS {
        if let Some(signal) = interrupted() {
            return Err(Interrupted { signal }.into());
        }
        let cwd = trees.reset(sha, plan_dir)?;
        let scratch = trees.scratch_dir()?.to_string_lossy().into_owned();
        let mut results = Vec::new();
        for step in steps {
            let mut result = run_step(&step.command, &cwd, step.timeout, &trees.cargo_target())?;
            result.step = step.number;
            result.accept = step.accept.clone();
            result.tail = result.tail.map(|tail| tail.replace(&scratch, "$SCRATCH"));
            let passed = result.exit == Some(0);
            results.push(result);
            if !passed {
                break;
            }
        }
        let decided = results.last().is_some_and(|last| last.exit.is_none());
        runs.push(results);
        if decided {
            break;
        }
    }
    Ok(runs)
}

/// How a running step ended.
enum Ended {
    Exited(ExitStatus),
    TimedOut,
    Interrupted(i32),
}

/// Run one step as `plan run` does: `bash -o pipefail -c <command>` in `cwd`,
/// in a process group of its own that is killed when the step ends, times out
/// or is interrupted.
fn run_step(
    command: &str,
    cwd: &Path,
    timeout: Duration,
    cargo_target: &Path,
) -> Result<StepResult> {
    let mut out = tempfile::tempfile().context("create a step's output file")?;
    let mut bash = Command::new("bash");
    bash.args(["-o", "pipefail", "-c", command])
        .current_dir(cwd)
        .env("PWD", cwd)
        .env("CARGO_TARGET_DIR", cargo_target)
        .stdin(Stdio::null())
        .stdout(out.try_clone().context("share a step's output file")?)
        .stderr(out.try_clone().context("share a step's output file")?);
    for name in GIT_REDIRECTS {
        bash.env_remove(name);
    }
    #[cfg(unix)]
    std::os::unix::process::CommandExt::process_group(&mut bash, 0);
    let start = Instant::now();
    let mut child = match bash.spawn() {
        Ok(child) => child,
        Err(error) => {
            return Ok(StepResult {
                error: Some(error.to_string()),
                ..StepResult::default()
            });
        }
    };
    let ended = loop {
        if let Some(status) = child.try_wait().context("wait for a step")? {
            break Ended::Exited(status);
        }
        if let Some(signal) = interrupted() {
            break Ended::Interrupted(signal);
        }
        if start.elapsed() >= timeout {
            break Ended::TimedOut;
        }
        std::thread::sleep(POLL);
    };
    // Also after a clean exit: whatever the step left running goes with it.
    kill_group(child.id());
    let _ = child.wait();
    let secs = (start.elapsed().as_secs_f64() * 100.0).round() / 100.0;
    match ended {
        Ended::Interrupted(signal) => Err(Interrupted { signal }.into()),
        Ended::TimedOut => Ok(StepResult {
            secs,
            timed_out: true,
            tail: Some(tail(&mut out)),
            ..StepResult::default()
        }),
        Ended::Exited(status) => {
            let exit = exit_code(status);
            Ok(StepResult {
                exit,
                secs,
                tail: (exit != Some(0)).then(|| tail(&mut out)),
                ..StepResult::default()
            })
        }
    }
}

/// Kill a step's process group.
fn kill_group(pid: u32) {
    #[cfg(unix)]
    if let Some(pid) = i32::try_from(pid)
        .ok()
        .and_then(rustix::process::Pid::from_raw)
    {
        let _ = rustix::process::kill_process_group(pid, rustix::process::Signal::KILL);
    }
    #[cfg(not(unix))]
    let _ = pid;
}

/// The exit code, or the negated signal that ended the step, as Python's
/// `returncode` gives it.
fn exit_code(status: ExitStatus) -> Option<i32> {
    #[cfg(unix)]
    if let Some(signal) = std::os::unix::process::ExitStatusExt::signal(&status) {
        return Some(-signal);
    }
    status.code()
}

/// The last lines of a step's output, as speclint keeps them.
fn tail(out: &mut File) -> String {
    let size = out.seek(SeekFrom::End(0)).unwrap_or(0);
    let mut bytes = Vec::new();
    if out
        .seek(SeekFrom::Start(size.saturating_sub(TAIL_BYTES)))
        .is_ok()
    {
        let _ = out.read_to_end(&mut bytes);
    }
    let text = String::from_utf8_lossy(&bytes);
    let lines: Vec<&str> = text
        .lines()
        .map(str::trim_end)
        .filter(|line| !line.trim().is_empty())
        .collect();
    let joined = lines[lines.len().saturating_sub(TAIL_LINES)..].join("\n");
    let skip = joined.chars().count().saturating_sub(TAIL_CHARS);
    joined.chars().skip(skip).collect()
}

/// The red-on-base result of one task's runs, with its outcome and reason.
fn classify(steps: &[Step], runs: &[Vec<StepResult>]) -> (RedOnBase, Outcome, String) {
    let unknown = |outcome, reason| (RedOnBase::Unknown, outcome, reason);
    // Per run: `None` when every step passed, else the step it failed at.
    let mut verdicts: Vec<Option<usize>> = Vec::new();
    for (number, results) in (1..).zip(runs) {
        let Some(last) = results.last() else {
            continue;
        };
        let step = last.step;
        if let Some(error) = &last.error {
            return unknown(
                Outcome::Error,
                format!("step {step} could not start: {error}"),
            );
        }
        if last.timed_out {
            let reason = format!(
                "step {step} timed out after {} s in run {number}",
                last.secs
            );
            return unknown(Outcome::Timeout, reason);
        }
        if last.exit == Some(0) {
            verdicts.push(None);
            continue;
        }
        let expect = steps
            .iter()
            .find(|candidate| candidate.number == step)
            .map_or("", |candidate| candidate.expect.as_str());
        if expect == "pass_on_base" {
            let reason = format!(
                "step {step} expects to pass on the base but exits {} in run {number}",
                exit_text(last.exit)
            );
            return unknown(Outcome::BaseBroken, reason);
        }
        verdicts.push(Some(step));
    }
    if verdicts.windows(2).any(|pair| pair[0] != pair[1]) {
        let said: Vec<String> = (1..)
            .zip(&verdicts)
            .map(|(number, verdict)| match verdict {
                None => format!("run {number} passes"),
                Some(step) => format!("run {number} fails at step {step}"),
            })
            .collect();
        let reason = format!("the runs disagree: {}", said.join(", "));
        return unknown(Outcome::Flaky, reason);
    }
    match verdicts.first() {
        Some(None) => (
            RedOnBase::Pass,
            Outcome::Pass,
            "every step passes on the base in both runs".to_string(),
        ),
        Some(Some(step)) => {
            let exits: Vec<String> = runs
                .iter()
                .map(|results| exit_text(results.last().and_then(|last| last.exit)))
                .collect();
            let reason = format!(
                "step {step} fails on the base in both runs (exit {})",
                exits.join(", ")
            );
            (RedOnBase::Fail, Outcome::Fail, reason)
        }
        None => unknown(Outcome::Error, "no step ran".to_string()),
    }
}

fn exit_text(exit: Option<i32>) -> String {
    exit.map_or_else(|| "none".to_string(), |code| code.to_string())
}

// --------------------------------------------------------------------------
// Base checkouts.

/// Detached checkouts of base commits, in a scratch directory only this
/// check uses. Dropping it removes the worktrees it added and the scratch
/// directory, and nothing else.
struct BaseTrees {
    /// The workspace root, canonical.
    root: PathBuf,
    fixture: bool,
    /// The scratch directory; `None` once removed.
    scratch: Option<PathBuf>,
    /// The repository the checkouts come from: the workspace's, or the
    /// fixture snapshot.
    repo: PathBuf,
    /// The workspace root, relative to the repository's top level.
    prefix: PathBuf,
    /// The repository's `worktrees/` admin directory.
    admin: PathBuf,
    /// Base sha -> (its checkout, the checkout's admin directory).
    trees: BTreeMap<String, (PathBuf, PathBuf)>,
}

impl BaseTrees {
    fn open(root: &Path, scratch_parent: Option<&Path>, fixture: bool) -> Result<Self> {
        let parent = scratch_parent.map_or_else(std::env::temp_dir, Path::to_path_buf);
        let parent = std::fs::canonicalize(&parent)
            .ok()
            .filter(|parent| parent.is_dir())
            .ok_or_else(|| anyhow!("the scratch directory {} does not exist", parent.display()))?;
        let mut trees = Self {
            root: root.to_path_buf(),
            fixture,
            scratch: None,
            repo: root.to_path_buf(),
            prefix: PathBuf::new(),
            admin: PathBuf::new(),
            trees: BTreeMap::new(),
        };
        let checkouts = trees.checkouts()?;
        if let Some(checkout) = checkouts
            .iter()
            .find(|checkout| parent.starts_with(checkout))
        {
            bail!(
                "the scratch directory {} is inside the checkout {}",
                parent.display(),
                checkout.display()
            );
        }
        trees.scratch = Some(make_scratch(&parent)?);
        if fixture {
            trees.repo = trees.snapshot()?;
        }
        let common = git_ok(
            &["rev-parse", "--path-format=absolute", "--git-common-dir"],
            &trees.repo,
        )?;
        trees.admin = canonical(Path::new(common.trim())).join("worktrees");
        Ok(trees)
    }

    /// Every checkout of the repository, once `repo` and `prefix` are set;
    /// just the root in fixture mode.
    fn checkouts(&mut self) -> Result<Vec<PathBuf>> {
        if self.fixture {
            if self.root.join(".git").exists() {
                bail!(
                    "--fixture needs a plain directory, and {} is a git checkout",
                    self.root.display()
                );
            }
            return Ok(vec![self.root.clone()]);
        }
        let top = git(&["rev-parse", "--show-toplevel"], &self.root)
            .ok()
            .filter(|output| output.status.success())
            .ok_or_else(|| {
                anyhow!(
                    "{} is not in a git work tree (use --fixture for a plain directory)",
                    self.root.display()
                )
            })?;
        self.repo = canonical(Path::new(String::from_utf8_lossy(&top.stdout).trim()));
        self.prefix = self
            .root
            .strip_prefix(&self.repo)
            .map_err(|_| {
                anyhow!(
                    "the workspace root {} is outside its repo {}",
                    self.root.display(),
                    self.repo.display()
                )
            })?
            .to_path_buf();
        let listing = git_ok(&["worktree", "list", "--porcelain"], &self.repo)?;
        Ok(listing
            .lines()
            .filter_map(|line| line.strip_prefix("worktree "))
            .map(|path| canonical(Path::new(path)))
            .collect())
    }

    /// The workspace root copied into a one-commit repo in the scratch
    /// directory.
    fn snapshot(&self) -> Result<PathBuf> {
        let repo = self.scratch_dir()?.join("snapshot");
        copy_tree(&self.root, &repo)?;
        git_ok(&["init", "--quiet"], &repo)?;
        git_ok(&["add", "--all"], &repo)?;
        git_ok(
            &[
                "-c",
                "user.name=roko",
                "-c",
                "user.email=roko@localhost",
                "-c",
                "commit.gpgsign=false",
                "-c",
                "core.hooksPath=/dev/null",
                "commit",
                "--quiet",
                "--no-verify",
                "--allow-empty",
                "-m",
                "roko red-on-base fixture",
            ],
            &repo,
        )?;
        Ok(repo)
    }

    fn scratch_dir(&self) -> Result<&Path> {
        self.scratch
            .as_deref()
            .ok_or_else(|| anyhow!("the red-on-base scratch directory is gone"))
    }

    /// The commit `rev` names.
    fn resolve(&self, rev: &str) -> Result<String> {
        let spec = format!("{rev}^{{commit}}");
        let output = git(
            &["rev-parse", "--verify", "--quiet", spec.as_str()],
            &self.repo,
        )?;
        if !output.status.success() {
            bail!("unknown base {rev:?} in {}", self.repo.display());
        }
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    /// The commit to check a plan against when no base is given, and why;
    /// `None` when the plan has no known base.
    fn plan_base(
        &self,
        plan: &Path,
        rel: &str,
        tasks: &[&Table],
        head: &str,
    ) -> Result<(Option<String>, String)> {
        if rel.split('/').any(|part| part == "archive") {
            let why = "archived plan: its pre-change commit is not recorded; check it with --base";
            return Ok((None, why.to_string()));
        }
        let outputs: BTreeSet<String> = tasks.iter().flat_map(|task| task_outputs(task)).collect();
        Ok(match self.landed(plan, &outputs)? {
            Some(sha) => (
                None,
                format!(
                    "its files changed in {}, at or after the commit that added the plan, so its \
                     work has probably landed; check it with --base <pre-change commit>",
                    short(&sha)
                ),
            ),
            None => (
                Some(head.to_string()),
                "HEAD: the plan has not run".to_string(),
            ),
        })
    }

    /// The latest commit, from the one that added `plan` on, that touched the
    /// plan's files.
    fn landed(&self, plan: &Path, outputs: &BTreeSet<String>) -> Result<Option<String>> {
        let Ok(rel) = plan.strip_prefix(&self.root) else {
            return Ok(None);
        };
        let tracked = self.prefix.join(rel).to_string_lossy().into_owned();
        let added = git_ok(
            &[
                "log",
                "--follow",
                "--diff-filter=A",
                "--format=%H",
                "--",
                tracked.as_str(),
            ],
            &self.repo,
        )?;
        // Not committed yet, so it has not run.
        let Some(first) = added.split_whitespace().last() else {
            return Ok(None);
        };
        // A root commit has no state from before the plan to compare with:
        // only later commits count.
        let parents = git_ok(&["rev-list", "--parents", "-n", "1", first], &self.repo)?;
        let span = if parents.split_whitespace().count() == 1 {
            format!("{first}..HEAD")
        } else {
            format!("{first}^..HEAD")
        };
        let paths: Vec<String> = outputs
            .iter()
            .map(|output| normpath(&self.prefix.join(output).to_string_lossy()))
            .filter(|path| !path.starts_with(['/', '~', '$']) && !path.starts_with(".."))
            .collect();
        if paths.is_empty() {
            return Ok(None);
        }
        let mut args = vec!["log", "-1", "--format=%H", span.as_str(), "--"];
        args.extend(paths.iter().map(String::as_str));
        let latest = git_ok(&args, &self.repo)?;
        Ok(Some(latest.trim().to_string()).filter(|sha| !sha.is_empty()))
    }

    /// Where the checkout of `sha` lives.
    fn tree_path(&self, sha: &str) -> PathBuf {
        let scratch = self.scratch.clone().unwrap_or_default();
        scratch.join(format!("base-{}", short(sha)))
    }

    /// Where `plan_dir` sits in the checkout of `sha`: its copy there when it
    /// is inside the workspace, else the directory itself.
    fn plan_copy(&self, sha: &str, plan_dir: &Path) -> PathBuf {
        match plan_dir.strip_prefix(&self.root) {
            Ok(rel) => self.tree_path(sha).join(&self.prefix).join(rel),
            Err(_) => plan_dir.to_path_buf(),
        }
    }

    /// Where cargo steps build: in the scratch directory, outside every
    /// checkout, so a reset keeps the build.
    fn cargo_target(&self) -> PathBuf {
        let scratch = self.scratch.clone().unwrap_or_default();
        scratch.join("cargo-target")
    }

    /// The checkout of `sha` and its admin directory, added on first use.
    fn checkout(&mut self, sha: &str) -> Result<(PathBuf, PathBuf)> {
        if let Some(tree) = self.trees.get(sha) {
            return Ok(tree.clone());
        }
        let tree = self.tree_path(sha);
        let tree_arg = tree.to_string_lossy().into_owned();
        git_ok(
            &[
                "-c",
                "core.hooksPath=/dev/null",
                "worktree",
                "add",
                "--detach",
                "--quiet",
                tree_arg.as_str(),
                sha,
            ],
            &self.repo,
        )?;
        // Recorded first, so that it is removed even if reading it fails.
        self.trees
            .insert(sha.to_string(), (tree.clone(), PathBuf::new()));
        let dot_git = std::fs::read_to_string(tree.join(".git"))
            .with_context(|| format!("read {}", tree.join(".git").display()))?;
        let gitdir = dot_git.trim();
        let gitdir = gitdir.strip_prefix("gitdir:").unwrap_or(gitdir).trim();
        let admin = canonical(&tree.join(gitdir));
        self.trees
            .insert(sha.to_string(), (tree.clone(), admin.clone()));
        Ok((tree, admin))
    }

    /// The workspace root in the checkout of `sha`, reset to the base, with
    /// `plan_dir` copied in.
    fn reset(&mut self, sha: &str, plan_dir: &Path) -> Result<PathBuf> {
        let (tree, admin) = self.checkout(sha)?;
        // An explicit git dir and work tree: even if a step broke the
        // checkout's .git file, these commands cannot reach another
        // repository.
        let git_dir = format!("--git-dir={}", admin.display());
        let work_tree = format!("--work-tree={}", tree.display());
        let (git_dir, work_tree) = (git_dir.as_str(), work_tree.as_str());
        git_ok(
            &[git_dir, work_tree, "reset", "--hard", "--quiet", sha],
            &tree,
        )?;
        git_ok(&[git_dir, work_tree, "clean", "-ffdxq"], &tree)?;
        let workspace = tree.join(&self.prefix);
        // A fixture snapshot already holds its plans, and a plan at the
        // workspace root has no directory of its own to copy.
        if !self.fixture
            && plan_dir != self.root.as_path()
            && let Ok(rel) = plan_dir.strip_prefix(&self.root)
        {
            copy_tree(plan_dir, &workspace.join(rel))?;
        }
        Ok(workspace)
    }

    fn close(&mut self) {
        for (tree, admin) in std::mem::take(&mut self.trees).into_values() {
            let tree_arg = tree.to_string_lossy().into_owned();
            let removed = git(
                &[
                    "worktree",
                    "remove",
                    "--force",
                    "--force",
                    tree_arg.as_str(),
                ],
                &self.repo,
            )
            .is_ok_and(|output| output.status.success());
            // A step may have deleted the checkout's .git file: then remove
            // only what this check added.
            if !removed {
                let _ = std::fs::remove_dir_all(&tree);
                if admin.parent() == Some(self.admin.as_path()) {
                    let _ = std::fs::remove_dir_all(&admin);
                }
            }
        }
        if let Some(scratch) = self.scratch.take() {
            let _ = std::fs::remove_dir_all(scratch);
        }
    }
}

impl Drop for BaseTrees {
    fn drop(&mut self) {
        self.close();
    }
}

/// A new directory in `parent` that only this check uses.
fn make_scratch(parent: &Path) -> Result<PathBuf> {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_nanos());
    for attempt in 0..100_u32 {
        let name = format!("roko-red-on-base-{}-{stamp}-{attempt}", std::process::id());
        let dir = parent.join(name);
        match std::fs::create_dir(&dir) {
            Ok(()) => return Ok(canonical(&dir)),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(error) => {
                return Err(error).with_context(|| {
                    format!("create a scratch directory in {}", parent.display())
                });
            }
        }
    }
    bail!("cannot create a scratch directory in {}", parent.display())
}

/// Copy `from` into `to`, keeping symlinks and leaving out `.git`; files that
/// exist are overwritten.
fn copy_tree(from: &Path, to: &Path) -> Result<()> {
    std::fs::create_dir_all(to).with_context(|| format!("create {}", to.display()))?;
    let entries = std::fs::read_dir(from).with_context(|| format!("read {}", from.display()))?;
    for entry in entries {
        let entry = entry.with_context(|| format!("read {}", from.display()))?;
        if entry.file_name() == ".git" {
            continue;
        }
        let (source, target) = (entry.path(), to.join(entry.file_name()));
        let kind = entry.file_type()?;
        if kind.is_dir() {
            copy_tree(&source, &target)?;
        } else if kind.is_symlink() {
            let link = std::fs::read_link(&source)?;
            let _ = std::fs::remove_file(&target);
            #[cfg(unix)]
            std::os::unix::fs::symlink(&link, &target)
                .with_context(|| format!("link {}", target.display()))?;
            #[cfg(not(unix))]
            let _ = link;
        } else {
            std::fs::copy(&source, &target)
                .with_context(|| format!("copy {} to {}", source.display(), target.display()))?;
        }
    }
    Ok(())
}

/// Run git in `cwd` without the variables that point it at another
/// repository.
fn git(args: &[&str], cwd: &Path) -> Result<Output> {
    let mut command = Command::new("git");
    command.args(args).current_dir(cwd).stdin(Stdio::null());
    for name in GIT_REDIRECTS {
        command.env_remove(name);
    }
    command
        .output()
        .with_context(|| format!("run git {}", args.join(" ")))
}

/// [`git`], failing unless it succeeds; its stdout.
fn git_ok(args: &[&str], cwd: &Path) -> Result<String> {
    let output = git(args, cwd)?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!(
            "git {} failed in {}: {}",
            args.join(" "),
            cwd.display(),
            stderr.trim()
        );
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

fn canonical(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

/// `path` relative to `root`, or the whole path when it is outside, as the
/// linter keys its records.
fn relative_path(path: &Path, root: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .into_owned()
}

fn short(sha: &str) -> &str {
    &sha[..sha.len().min(12)]
}

/// A string field, or `""` when it is absent or not a string.
fn text(value: Option<&Value>) -> String {
    value
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

/// The tables of an array field; nothing when it is absent or not an array.
fn tables(value: Option<&Value>) -> impl Iterator<Item = &Table> {
    value
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_table)
}

/// The files a task writes: `files` and `write_files`, trimmed.
fn task_outputs(task: &Table) -> Vec<String> {
    ["files", "write_files"]
        .into_iter()
        .filter_map(|key| task.get(key).and_then(Value::as_array))
        .flatten()
        .filter_map(Value::as_str)
        .map(str::trim)
        .filter(|path| !path.is_empty())
        .map(str::to_string)
        .collect()
}

// --------------------------------------------------------------------------
// Interrupts.

/// Routes SIGINT and SIGTERM to [`INTERRUPTED`] while a check runs, so that
/// it kills its step and removes its worktrees before it stops. The first
/// guard saves the process's actions and the last one restores them.
struct InterruptGuard;

/// How many guards are live, and the actions the first one replaced.
#[cfg(unix)]
static HANDLERS: std::sync::Mutex<(usize, Vec<(libc::c_int, libc::sigaction)>)> =
    std::sync::Mutex::new((0, Vec::new()));

impl InterruptGuard {
    fn install() -> Self {
        #[cfg(unix)]
        {
            let mut handlers = HANDLERS
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if handlers.0 == 0 {
                INTERRUPTED.store(0, Ordering::SeqCst);
                handlers.1 = [libc::SIGINT, libc::SIGTERM]
                    .into_iter()
                    .filter_map(replace_action)
                    .collect();
            }
            handlers.0 += 1;
        }
        Self
    }
}

impl Drop for InterruptGuard {
    fn drop(&mut self) {
        #[cfg(unix)]
        {
            let mut handlers = HANDLERS
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            handlers.0 = handlers.0.saturating_sub(1);
            if handlers.0 == 0 {
                for (signal, previous) in std::mem::take(&mut handlers.1) {
                    restore_action(signal, &previous);
                }
            }
        }
    }
}

/// Install [`on_interrupt`] for `signal`; the action it replaced.
#[cfg(unix)]
#[allow(unsafe_code)]
fn replace_action(signal: libc::c_int) -> Option<(libc::c_int, libc::sigaction)> {
    // SAFETY: both structs are zero-initialized plain C data, filled in
    // before sigaction(2) reads them, and the handler only stores to an
    // atomic, which is async-signal-safe.
    unsafe {
        let mut action: libc::sigaction = std::mem::zeroed();
        action.sa_sigaction = on_interrupt as *const () as libc::sighandler_t;
        libc::sigemptyset(&raw mut action.sa_mask);
        let mut previous: libc::sigaction = std::mem::zeroed();
        (libc::sigaction(signal, &raw const action, &raw mut previous) == 0)
            .then_some((signal, previous))
    }
}

/// Put back the action [`replace_action`] replaced.
#[cfg(unix)]
#[allow(unsafe_code)]
fn restore_action(signal: libc::c_int, previous: &libc::sigaction) {
    // SAFETY: `previous` is the action sigaction(2) returned for `signal`.
    unsafe {
        libc::sigaction(signal, previous, std::ptr::null_mut());
    }
}

#[cfg(unix)]
extern "C" fn on_interrupt(signal: libc::c_int) {
    INTERRUPTED.store(signal, Ordering::SeqCst);
}

/// The signal that interrupted the running check, if one did.
fn interrupted() -> Option<i32> {
    let signal = INTERRUPTED.load(Ordering::SeqCst);
    (signal != 0).then_some(signal)
}

#[cfg(test)]
mod tests {
    use super::*;
    use roko_core::config::SpecQualityConfig;

    const GIT_CONFIG: [&str; 10] = [
        "-c",
        "user.name=roko-test",
        "-c",
        "user.email=roko-test@localhost",
        "-c",
        "commit.gpgsign=false",
        "-c",
        "core.hooksPath=/dev/null",
        "-c",
        "init.defaultBranch=main",
    ];

    /// T1 is red on the base (no greet.sh), T2 is green (config.ini already
    /// sets retries), and T3 passes in the first run only: COUNTER is a file
    /// outside the checkout.
    const PLAN: &str = r#"[meta]
plan = "p"

[[task]]
id = "T1"
title = "Greeting script"
description = "Add `greet.sh`, which prints `hello, world`."
role = "implementer"
files = ["greet.sh"]

[[task.verify]]
phase = "test"
command = "bash greet.sh | grep -qx 'hello, world'"

[[task]]
id = "T2"
title = "Retry limit"
description = "Set `retries` in `config.ini`."
role = "implementer"
files = ["config.ini"]

[[task.verify]]
phase = "structural"
command = "grep -q '^retries' config.ini"

[[task]]
id = "T3"
title = "Flaky check"
description = "Make `greet.sh` print once per call."
role = "implementer"
files = ["greet.sh"]

[[task.verify]]
phase = "test"
command = 'n=$(cat COUNTER 2>/dev/null || echo 0); echo $((n + 1)) > COUNTER; test "$n" -eq 0'
"#;

    fn run_git(repo: &Path, args: &[&str]) -> String {
        let output = Command::new("git")
            .args(GIT_CONFIG)
            .args(args)
            .current_dir(repo)
            .output()
            .expect("run git");
        assert!(
            output.status.success(),
            "git {args:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8_lossy(&output.stdout).into_owned()
    }

    /// What a check must leave as it found it: the worktrees, the refs, the
    /// checkout's status and the worktree admin directories.
    fn repo_state(repo: &Path) -> Vec<String> {
        let mut admin: Vec<String> = std::fs::read_dir(repo.join(".git/worktrees"))
            .map(|entries| {
                entries
                    .filter_map(Result::ok)
                    .map(|entry| entry.file_name().to_string_lossy().into_owned())
                    .collect()
            })
            .unwrap_or_default();
        admin.sort();
        vec![
            run_git(repo, &["worktree", "list", "--porcelain"]),
            run_git(repo, &["for-each-ref", "--format=%(refname) %(objectname)"]),
            run_git(repo, &["status", "--porcelain", "--untracked-files=all"]),
            admin.join(","),
        ]
    }

    /// 3214: on a repo whose base already passes T2's verify step, T2 is
    /// `pass` and gets HF3 (so the spec gate blocks it), T1's red step scores
    /// SQ06 = 1, T3's flaky step is `unknown`, and the check leaves no
    /// worktree, branch, admin directory or scratch file behind.
    #[test]
    fn spec_quality_dynamic_flags_a_verify_green_on_base() {
        let temp = tempfile::tempdir().expect("tempdir");
        let repo = temp.path().join("repo");
        std::fs::create_dir_all(repo.join("plans/p")).expect("plan dir");
        let counter = temp.path().join("flaky-counter");
        let plan_text = PLAN.replace("COUNTER", &counter.to_string_lossy());
        std::fs::write(repo.join("plans/p/tasks.toml"), plan_text).expect("write the plan");
        std::fs::write(repo.join("config.ini"), "retries = 3\n").expect("write config.ini");
        run_git(&repo, &["init", "--quiet"]);
        run_git(&repo, &["add", "--all"]);
        run_git(&repo, &["commit", "--quiet", "-m", "base"]);
        let scratch = temp.path().join("scratch");
        std::fs::create_dir_all(&scratch).expect("scratch dir");
        let before = repo_state(&repo);

        let plan = repo.join("plans/p/tasks.toml");
        let options = RedOnBaseOptions {
            timeout: Some(Duration::from_secs(30)),
            scratch: Some(scratch.clone()),
            ..RedOnBaseOptions::default()
        };
        let report = check_plans(&[plan.clone()], &repo, &options).expect("check the plan");
        let check = |id: &str| {
            let check = report.checks.iter().find(|check| check.task_id == id);
            check.expect("checked").clone()
        };
        let (t1, t2, t3) = (check("T1"), check("T2"), check("T3"));
        assert_eq!(
            (t1.red_on_base, t1.outcome),
            (RedOnBase::Fail, Outcome::Fail),
            "{t1:?}"
        );
        assert_eq!(
            (t2.red_on_base, t2.outcome),
            (RedOnBase::Pass, Outcome::Pass),
            "{t2:?}"
        );
        assert_eq!(
            (t3.red_on_base, t3.outcome),
            (RedOnBase::Unknown, Outcome::Flaky),
            "{t3:?}"
        );
        assert_eq!(t1.plan_path, "plans/p/tasks.toml");
        assert_eq!(t2.base_reason, "HEAD: the plan has not run");
        assert_eq!(t1.runs.len(), RUNS, "{t1:?}");

        let quality =
            roko_gate::spec_quality::lint_files_with(&[plan.clone()], &repo, &report.results());
        let record = |id: &str| {
            let record = quality.tasks.iter().find(|record| record.task_id == id);
            record.expect("scored").clone()
        };
        assert!((record("T1").rules["SQ06"] - 1.0).abs() < f64::EPSILON);
        assert!(!record("T1").hard_fail.contains(&"HF3"));
        assert!(
            record("T2").hard_fail.contains(&"HF3"),
            "{:?}",
            record("T2")
        );
        assert!(!record("T3").hard_fail.contains(&"HF3"));
        assert!(record("T3").unknown.contains(&"HF3"));

        let gate = crate::spec_gate::check_plans(
            &[plan],
            &repo,
            &SpecQualityConfig::default(),
            &report.results(),
        );
        let blocked: Vec<&str> = gate
            .blocked()
            .map(|decision| decision.task_id.as_str())
            .collect();
        assert_eq!(blocked, ["T2"], "{gate:?}");

        assert_eq!(repo_state(&repo), before, "the check left something behind");
        let left: Vec<PathBuf> = std::fs::read_dir(&scratch)
            .expect("read the scratch dir")
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .collect();
        assert!(left.is_empty(), "{left:?}");
    }

    /// gap-0ee70b: by default the check runs shell checks on the base and
    /// skips cargo ones. A shell check that already passes (T1) is HF3 and
    /// the spec gate refuses the plan; a real one (T2) is red; the cargo one
    /// (T3) never runs and is `skipped: cargo`. Off, or outside git, the
    /// gate gets no results and blocks nothing.
    #[test]
    fn red_on_base_runs_shell_checks_and_skips_cargo() {
        let temp = tempfile::tempdir().expect("tempdir");
        let repo = temp.path().join("repo");
        std::fs::create_dir_all(repo.join("plans/p")).expect("plan dir");
        let marker = temp.path().join("cargo-ran");
        let plan_text = PLAN
            .replace(
                "n=$(cat COUNTER 2>/dev/null || echo 0); echo $((n + 1)) > COUNTER; test \"$n\" -eq 0",
                &format!("cargo test -p demo greet && touch {}", marker.display()),
            )
            .replace("[[task]]\nid = \"T1\"", "[[task]]\nid = \"T9\"")
            .replace("[[task]]\nid = \"T2\"", "[[task]]\nid = \"T1\"")
            .replace("[[task]]\nid = \"T9\"", "[[task]]\nid = \"T2\"");
        std::fs::write(repo.join("plans/p/tasks.toml"), plan_text).expect("write the plan");
        std::fs::write(repo.join("config.ini"), "retries = 3\n").expect("write config.ini");
        run_git(&repo, &["init", "--quiet"]);
        run_git(&repo, &["add", "--all"]);
        run_git(&repo, &["commit", "--quiet", "-m", "base"]);
        let plan = repo.join("plans/p/tasks.toml");

        let config = SpecQualityConfig::default();
        assert!(config.red_on_base && !config.red_on_base_cargo);
        let options = RedOnBaseOptions::from_config(&config);
        assert_eq!(options.cargo, CargoSteps::Skip);
        let report = check_plans(&[plan.clone()], &repo, &options).expect("check the plan");
        let outcome = |id: &str| {
            let check = report.checks.iter().find(|check| check.task_id == id);
            let check = check.expect("checked");
            (check.red_on_base, check.outcome, check.reason.clone())
        };
        assert_eq!(outcome("T1").0, RedOnBase::Pass, "{:?}", outcome("T1"));
        assert_eq!(outcome("T2").0, RedOnBase::Fail, "{:?}", outcome("T2"));
        let (cargo, cargo_outcome, reason) = outcome("T3");
        assert_eq!(
            (cargo, cargo_outcome),
            (RedOnBase::Unknown, Outcome::SkippedCargo)
        );
        assert!(reason.starts_with("skipped: cargo"), "{reason}");
        assert!(!marker.exists(), "the cargo step ran");

        let gate = crate::spec_gate::check_plans(
            &[plan.clone()],
            &repo,
            &config,
            &gate_results(&[plan.clone()], &repo, &config).expect("not interrupted"),
        );
        let blocked: Vec<&str> = gate
            .blocked()
            .map(|decision| decision.task_id.as_str())
            .collect();
        assert_eq!(blocked, ["T1"], "{gate:?}");

        let off = SpecQualityConfig {
            red_on_base: false,
            ..SpecQualityConfig::default()
        };
        let results = gate_results(&[plan.clone()], &repo, &off).expect("not interrupted");
        assert!(results.is_empty());
        let plain = tempfile::tempdir().expect("tempdir");
        let outside = plain.path().join("plans/p");
        std::fs::create_dir_all(&outside).expect("plan dir");
        std::fs::copy(&plan, outside.join("tasks.toml")).expect("copy the plan");
        let results = gate_results(&[outside.join("tasks.toml")], plain.path(), &config);
        assert!(results.expect("not interrupted").is_empty());
    }
}
