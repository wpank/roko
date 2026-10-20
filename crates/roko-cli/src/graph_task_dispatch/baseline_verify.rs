//! Pre-existing verify failures (gap-161be1).
//!
//! A task's verify step often runs a whole crate's tests. When one of them
//! already failed before the agent started, the step fails, and the task is
//! retried until it fails, though the agent neither caused that failure nor
//! was asked to fix it. So a test step that still fails after sibling
//! settlement is judged against a baseline
//! ([`GraphTaskDispatcher::judge_against_baseline`]): its tests run again on
//! the attempt's tree and on the plan run's start commit, in a detached
//! temporary worktree, and the tests that fail are compared by name.
//!
//! - Every test that fails failed on the start commit too: the step passes.
//!   Its gate becomes `pre-existing-filtered:<step>`, its output and the log
//!   name those tests, and the attempt settles as
//!   `passed_with_preexisting_failures`, never as a clean pass. The parts of
//!   the step's command after its tests, which their failure kept from
//!   running, must pass first.
//! - A test that passed on the start commit fails now: the step fails, and
//!   its output names the new failures apart from the old.
//! - A test is the task's to fix, however it fared before, when the task's
//!   text names its function, one of its modules or its test file, or the
//!   task declares a file of that name. So is a Python test in a file the
//!   attempt edits, and a Python test module that could not be loaded on the
//!   start commit when the attempt touches it (below): it must load now.
//! - Old failures alone verify nothing: unless some other test ran and
//!   passed, the step fails.
//! - What the comparison cannot tell fails the step as before: output that
//!   does not name every failing test (a build error, a crashed test binary,
//!   quiet or truncated output), a baseline that did not run or timed out, a
//!   command that runs tests in more than one part, or after a part that is
//!   not a read-only check. A regression never becomes a pass, and neither
//!   does a failure only because the start commit fails the same way: a
//!   check for work the task adds, or a build it should fix, fails there by
//!   design.
//!
//! The tests run without fail-fast on both sides, since cargo stops at the
//! first failing test target and would hide the targets after it, and
//! without the read-only checks before them (`test -f`, `grep -q`), which
//! passed on the attempt's tree and may fail on the start commit by design.
//! The baseline builds in a target directory of its own: a target directory
//! shared between worktrees lets cargo reuse the other tree's crates. Its
//! result is kept per (start commit, command): for the run in memory, so
//! tasks with the same failing step pay for one baseline and the start
//! commit's worktree serves the run until it ends, and on disk
//! (`.roko/state/baselines/<commit>.json`), so later runs from the same
//! commit, and a prompt run's check before its agent runs
//! ([`prompt_base_checks`]), pay for none. A baseline that timed out, on a
//! cold build say, is tried again by the next failure, on what it built.
//!
//! The steps judged run `cargo test`, `python -m unittest` or `pytest`
//! ([`super::test_runners`]). A Python suite whose start-commit run names no
//! failure, as pytest's does when a test module cannot be imported (its run
//! stops at collection), is judged by the test files the attempt adds or
//! touches instead (bug-5089fb): those it changed, those the task's text
//! names, and those that import a module it changed. They must run, at
//! least one test, and pass; with none of them, the failure stands, so a
//! check that did not run never verifies an attempt. A prompt run runs its
//! Python checks on its base before its agent runs ([`prompt_base_checks`])
//! and tells the agent which tests already fail.
//!
//! The filter is off with `[gates] baseline_filter = false`, in FAST mode,
//! and for an attempt that changed nothing, whose steps must pass on their
//! own. `cargo nextest` steps are not judged: their output is not libtest's.

use std::collections::{BTreeSet, HashMap};
use std::time::Duration;

use roko_core::Verdict;
use roko_fs::RokoLayout;

use super::test_runners::TestRunner;
use super::*;
use crate::task_parser::VerifyStep;

/// Longest a `git worktree add` or `remove` may take.
const GIT_TIMEOUT: Duration = Duration::from_secs(60);

// ── Baseline worktree ────────────────────────────────────────────────────

/// A detached checkout of a base revision in a temporary directory,
/// registered with the repository's worktrees. Dropping it removes the
/// worktree; one that cannot be removed is kept, and logged, rather than
/// lost.
pub(crate) struct BaselineWorktree {
    repository: PathBuf,
    checkout: PathBuf,
    parent: Option<tempfile::TempDir>,
    /// The worktree may be registered with the repository and must be
    /// removed.
    cleanup_required: bool,
}

impl BaselineWorktree {
    /// Check `revision` of `repository` out, detached, in a new temporary
    /// directory; `None` when git cannot.
    pub(crate) async fn add(repository: &Path, revision: &str) -> Option<Self> {
        let parent = tempfile::Builder::new()
            .prefix("roko-gate-baseline-")
            .tempdir()
            .ok()?;
        let worktree = Self {
            repository: repository.to_path_buf(),
            checkout: parent.path().join("checkout"),
            parent: Some(parent),
            cleanup_required: true,
        };
        let added = git(
            repository,
            &[
                "worktree".as_ref(),
                "add".as_ref(),
                "--detach".as_ref(),
                worktree.checkout.as_os_str(),
                revision.as_ref(),
            ],
        )
        .await;
        // Dropping a worktree whose add failed prunes what it left.
        added.then_some(worktree)
    }

    /// The checkout.
    pub(crate) fn path(&self) -> &Path {
        &self.checkout
    }

    /// Remove the worktree now; a failure leaves it to `Drop`.
    pub(crate) async fn remove(mut self) {
        let removed = git(
            &self.repository,
            &[
                "worktree".as_ref(),
                "remove".as_ref(),
                "--force".as_ref(),
                self.checkout.as_os_str(),
            ],
        )
        .await;
        if removed {
            self.cleanup_required = false;
        } else {
            tracing::warn!(
                path = %self.checkout.display(),
                "failed to remove temporary baseline worktree cleanly"
            );
        }
    }
}

impl Drop for BaselineWorktree {
    fn drop(&mut self) {
        if !self.cleanup_required {
            return;
        }
        let removed = std::process::Command::new("git")
            .args(["worktree", "remove", "--force"])
            .arg(&self.checkout)
            .current_dir(&self.repository)
            .env("GIT_TERMINAL_PROMPT", "0")
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .is_ok_and(|status| status.success());
        if removed {
            return;
        }

        if self.checkout.exists() {
            if let Some(parent) = self.parent.take() {
                let retained = parent.keep();
                tracing::warn!(
                    path = %retained.display(),
                    "preserving temporary baseline worktree after cleanup failure"
                );
            }
        } else {
            let _ = std::process::Command::new("git")
                .args(["worktree", "prune", "--expire", "now"])
                .current_dir(&self.repository)
                .env("GIT_TERMINAL_PROMPT", "0")
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status();
        }
    }
}

/// Run `git args` in `repository`, bounded by [`GIT_TIMEOUT`]; whether it
/// succeeded.
async fn git(repository: &Path, args: &[&std::ffi::OsStr]) -> bool {
    let output = tokio::time::timeout(
        GIT_TIMEOUT,
        tokio::process::Command::new("git")
            .args(args)
            .current_dir(repository)
            .env("GIT_TERMINAL_PROMPT", "0")
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true)
            .output(),
    )
    .await;
    matches!(output, Ok(Ok(output)) if output.status.success())
}

// ── The step's command ───────────────────────────────────────────────────

/// The parts of `command` that `&&` joins, trimmed, when nothing else joins
/// them: no `;`, `||`, background `&`, newline, subshell or command
/// substitution. Pipes and redirections (`2>&1`) stay in their part.
fn and_parts(command: &str) -> Option<Vec<&str>> {
    let bytes = command.as_bytes();
    let mut parts = Vec::new();
    let mut start = 0;
    let mut quote = None;
    let mut index = 0;
    while index < bytes.len() {
        let byte = bytes[index];
        let next = bytes.get(index + 1).copied();
        match quote {
            // Single quotes keep everything as it is.
            Some(b'\'') => {
                if byte == b'\'' {
                    quote = None;
                }
            }
            Some(_) => match byte {
                b'"' => quote = None,
                b'\\' => index += 1,
                b'`' => return None,
                b'$' if next == Some(b'(') => return None,
                _ => {}
            },
            None => match byte {
                b'\'' | b'"' => quote = Some(byte),
                b'\\' => index += 1,
                b';' | b'\n' | b'(' | b')' | b'`' => return None,
                b'|' if next == Some(b'|') => return None,
                b'&' if next == Some(b'&') => {
                    parts.push(command[start..index].trim());
                    index += 2;
                    start = index;
                    continue;
                }
                // `2>&1`, `>&2` and `&>log` redirect; any other `&` runs
                // what comes before it in the background.
                b'&' if next == Some(b'>') || (index > 0 && b"<>".contains(&bytes[index - 1])) => {}
                b'&' => return None,
                _ => {}
            },
        }
        index += 1;
    }
    if quote.is_some() {
        return None;
    }
    parts.push(command[start..].trim());
    (!parts.iter().any(|part| part.is_empty())).then_some(parts)
}

/// Programs that only check the tree: `test -f x`, `grep -q y z`.
const CHECKS: &[&str] = &["test", "[", "[[", "grep", "egrep", "fgrep", "rg"];

/// Whether `part` of a step's command only checks the tree: one of
/// [`CHECKS`], or `cargo check`, `clippy` or `build`, with no pipe and no
/// redirection but to `/dev/null`.
fn is_check(part: &str) -> bool {
    let words: Vec<&str> = part
        .split_whitespace()
        .filter(|word| !word.ends_with(">/dev/null"))
        .collect();
    if words.iter().any(|word| word.contains(&['|', '>', '<'][..])) {
        return false;
    }
    let words = match words.as_slice() {
        ["!", rest @ ..] => rest,
        all => all,
    };
    match words {
        [program, ..] if CHECKS.contains(program) => true,
        ["cargo", toolchain, subcommand, ..] if toolchain.starts_with('+') => {
            matches!(*subcommand, "check" | "clippy" | "build")
        }
        ["cargo", subcommand, ..] => matches!(*subcommand, "check" | "clippy" | "build"),
        _ => false,
    }
}

/// A failed test step's command, as the baseline runs it.
#[derive(Debug, PartialEq, Eq)]
struct TestCommand {
    /// The test runner it runs.
    runner: TestRunner,
    /// The part of the command that runs the tests, without fail-fast
    /// (cargo's `--no-fail-fast` added): what runs, and is judged, on both
    /// sides.
    tests: String,
    /// That part as the step wrote it, and where its runner's own words end,
    /// for a run scoped to some test files.
    part: String,
    runner_end: usize,
    /// The parts after it, which its failure kept from running.
    rest: Option<String>,
}

impl TestCommand {
    /// Split `command`, an `&&` chain ([`and_parts`]), around its one test
    /// runner ([`TestRunner::find`]), when only read-only checks
    /// ([`is_check`]) come before it. Once the tests ran, the checks passed:
    /// they are left out, since the start commit may fail them by design.
    /// `None` also when the part tells unittest or pytest to stop at its
    /// first failure.
    fn of(command: &str) -> Option<Self> {
        let parts = and_parts(command)?;
        let runs: Vec<Option<(TestRunner, usize)>> =
            parts.iter().map(|part| TestRunner::find(part)).collect();
        if parts
            .iter()
            .zip(&runs)
            .any(|(part, run)| run.is_none() && TestRunner::any_in(part))
        {
            return None;
        }
        let mut testing = runs.iter().enumerate().filter(|(_, run)| run.is_some());
        let (index, &Some((runner, runner_end))) = testing.next()? else {
            return None;
        };
        if testing.next().is_some() || !parts[..index].iter().all(|part| is_check(part)) {
            return None;
        }
        let part = parts[index];
        let tests = runner.without_fail_fast(part, runner_end)?;
        let rest = &parts[index + 1..];
        Some(Self {
            runner,
            tests,
            part: part.to_string(),
            runner_end,
            rest: (!rest.is_empty()).then(|| rest.join(" && ")),
        })
    }
}

// ── Test results ─────────────────────────────────────────────────────────

/// Whether `text` holds `word` as a word of its own, not inside a longer
/// identifier.
fn mentions(text: &str, word: &str) -> bool {
    let identifier = |character: char| character.is_alphanumeric() || character == '_';
    text.match_indices(word).any(|(at, _)| {
        !text[..at].chars().next_back().is_some_and(identifier)
            && !text[at + word.len()..]
                .chars()
                .next()
                .is_some_and(identifier)
    })
}

/// Whether `test` (libtest's `<target>: <path>`, unittest's
/// `test_env.EnvTest.test_level`, pytest's `tests/test_env.py::test_level`)
/// is the task's to fix, however it fared before: the task's `text` (title,
/// description, acceptance) names its function, one of its modules or its
/// test file, or one of the task's `files` has one of those names. Names of
/// three characters or fewer, and `tests`, tell nothing.
fn concerns_task(test: &str, text: &str, files: &[String]) -> bool {
    let (target, path) = test.split_once(": ").unwrap_or(("", test));
    // A doc-test is `<file> - <item> (line <n>)`; an integration test's
    // target is its file.
    let (file, item) = match path.split_once(" - ") {
        Some((file, item)) => (Some(file), item.split(" (line ").next().unwrap_or(item)),
        None => ((!target.starts_with("unittests ")).then_some(target), path),
    };
    let file_stem = file
        .and_then(|file| Path::new(file).file_stem())
        .and_then(|stem| stem.to_str());
    let declared: Vec<&str> = files
        .iter()
        .filter_map(|file| Path::new(file).file_stem()?.to_str())
        .collect();
    item.split(|character: char| !(character.is_alphanumeric() || character == '_'))
        .chain(file_stem)
        .filter(|name| name.len() > 3 && *name != "tests")
        .any(|name| mentions(text, name) || declared.contains(&name))
}

// ── Judgement ────────────────────────────────────────────────────────────

/// How a failed test step compares with its baseline.
#[derive(Debug, Clone, PartialEq)]
pub(super) enum Judgement {
    /// Every test that fails failed on the start commit too, and none is the
    /// task's to fix: the step passes, with these pre-existing failures.
    Preexisting(Vec<String>),
    /// The step fails: `new` passed on the start commit, `owned` failed
    /// there too but are the task's to fix, and `preexisting` failed there
    /// and are not.
    New {
        new: Vec<String>,
        owned: Vec<String>,
        preexisting: Vec<String>,
    },
    /// Its tests pass for the reason `because` gives (only pre-existing
    /// failures, or the tests the attempt adds or touches pass), but `rest`,
    /// the parts of its command after them, fails on its own: `verdict` is
    /// that run.
    RestFails {
        because: String,
        rest: String,
        verdict: Box<Verdict>,
    },
    /// The step's suite named no failure on the start commit, so it was
    /// judged by the test files the attempt adds or touches (bug-5089fb),
    /// and they pass: `command` ran them, and `passed` tests passed.
    Scoped {
        files: Vec<String>,
        command: String,
        passed: u32,
    },
    /// The same, but those tests fail, or run none: `verdict` is that run.
    ScopedFails {
        files: Vec<String>,
        verdict: Box<Verdict>,
    },
    /// Nothing verifies the attempt, for this reason: its suite named no
    /// failure on the start commit and it adds or touches no test that
    /// could stand in, or only old failures failed and no other test passed.
    /// The failure stands, and the step's output says why.
    Unverified(String),
    /// The comparison cannot tell, for this reason: the failure stands.
    Undecided(String),
}

/// Judge the tests' run on the attempt's tree, `now`, against their run on
/// the start commit, `before`, by the names of the tests `runner` says
/// fail. `task_owns` tells the tests that are the task's to fix. Old
/// failures pass the step only when some other test ran and passed.
fn judge(
    runner: TestRunner,
    now: &Verdict,
    before: &Verdict,
    task_owns: impl Fn(&str) -> bool,
) -> Judgement {
    let undecided = |reason: &str| Judgement::Undecided(reason.to_string());
    if now.passed {
        return undecided("its tests passed when run again without fail-fast");
    }
    if roko_gate::verdict_timed_out(now) {
        return undecided("its tests timed out when run again without fail-fast");
    }
    if roko_gate::verdict_timed_out(before) {
        return undecided("the baseline run timed out");
    }
    let Some(failing_now) = runner
        .failed_tests(now.detail.as_deref().unwrap_or_default())
        .filter(|failed| !failed.is_empty())
    else {
        return undecided("its output does not name every failing test");
    };
    let failing_before = if before.passed {
        BTreeSet::new()
    } else {
        match runner.failed_tests(before.detail.as_deref().unwrap_or_default()) {
            Some(failed) if !failed.is_empty() => failed,
            _ => return undecided("the baseline failed without naming every failing test"),
        }
    };
    let (mut new, mut owned, mut preexisting) = (Vec::new(), Vec::new(), Vec::new());
    for test in failing_now {
        if !failing_before.contains(&test) {
            new.push(test);
        } else if task_owns(test.as_str()) {
            owned.push(test);
        } else {
            preexisting.push(test);
        }
    }
    if !new.is_empty() || !owned.is_empty() {
        Judgement::New {
            new,
            owned,
            preexisting,
        }
    } else if runner
        .passed(now.detail.as_deref().unwrap_or_default())
        .unwrap_or(0)
        == 0
    {
        Judgement::Unverified(format!(
            "{}, and no other test ran and passed, so nothing verifies the change: add or edit \
             a test that exercises it",
            already_failed(preexisting.len())
        ))
    } else {
        Judgement::Preexisting(preexisting)
    }
}

/// Settle a failed test step's `verdict` by its `judgement`, and say so in
/// its output and the log; whether the step now passes.
pub(super) fn apply(
    verdict: &mut Verdict,
    judgement: Judgement,
    plan_id: &str,
    task_id: &str,
) -> bool {
    let step = verdict.gate.clone();
    match judgement {
        Judgement::Preexisting(tests) => {
            let count = already_failed(tests.len());
            let tests = tests.join(", ");
            tracing::info!(
                plan_id,
                task_id,
                step = %step,
                tests = %tests,
                "verify step failed only on tests that failed on the plan run's start commit too; \
                 it passes"
            );
            verdict.passed = true;
            verdict.gate = format!("pre-existing-filtered:{step}");
            verdict.reason = format!("{count}: {tests}");
            append_note(
                verdict,
                &format!(
                    "[roko] pre-existing failures, so the step passes: {count}, on the plan \
                     run's start commit: {tests}"
                ),
            );
            true
        }
        Judgement::Scoped {
            files,
            command,
            passed,
        } => {
            let files = files.join(", ");
            tracing::info!(
                plan_id,
                task_id,
                step = %step,
                files = %files,
                passed,
                "verify step's suite named no failure on the plan run's start commit; the tests \
                 the attempt adds or touches pass"
            );
            verdict.passed = true;
            verdict.gate = format!("pre-existing-filtered:{step}");
            verdict.reason = format!(
                "the suite already failed before the change without naming its failures; the \
                 tests this change adds or touches pass: {files}"
            );
            append_note(
                verdict,
                &format!(
                    "[roko] the suite already failed on the plan run's start commit, and its \
                     output named no failing test, so the step was judged by the tests this \
                     change adds or touches: `{command}`: {passed} passed"
                ),
            );
            true
        }
        Judgement::ScopedFails {
            files,
            verdict: scoped,
        } => {
            let files = files.join(", ");
            tracing::info!(
                plan_id,
                task_id,
                step = %step,
                files = %files,
                "verify step's suite named no failure on the plan run's start commit, and the \
                 tests the attempt adds or touches fail"
            );
            *verdict = Verdict {
                gate: step,
                ..*scoped
            };
            verdict.passed = false;
            append_note(
                verdict,
                &format!(
                    "[roko] the suite already failed on the plan run's start commit without \
                     naming its failures, so the step was judged by the tests this change adds \
                     or touches ({files}): they must run and pass"
                ),
            );
            false
        }
        Judgement::Unverified(reason) => {
            tracing::info!(
                plan_id,
                task_id,
                step = %step,
                reason = %reason,
                "verify step failure stands: nothing can verify the attempt"
            );
            append_note(verdict, &format!("[roko] {reason}"));
            false
        }
        Judgement::New {
            new,
            owned,
            preexisting,
        } => {
            tracing::info!(
                plan_id,
                task_id,
                step = %step,
                new = %new.join(", "),
                owned = %owned.join(", "),
                preexisting = %preexisting.join(", "),
                "verify step fails on tests that passed on the plan run's start commit, or that \
                 are the task's to fix"
            );
            for (tests, note) in [
                (
                    &preexisting,
                    "failed before this run too, not this task's to fix",
                ),
                (&owned, "failed before this run too, but this task's to fix"),
                (
                    &new,
                    "new failures, which did not fail on the plan run's start commit",
                ),
            ] {
                if !tests.is_empty() {
                    append_note(verdict, &format!("[roko] {note}: {}", tests.join(", ")));
                }
            }
            false
        }
        Judgement::RestFails {
            because,
            rest,
            verdict: rest_run,
        } => {
            tracing::info!(
                plan_id,
                task_id,
                step = %step,
                because = %because,
                rest = %rest,
                "verify step's tests pass against the plan run's start commit, but the rest of \
                 the step fails"
            );
            *verdict = *rest_run;
            append_note(
                verdict,
                &format!("[roko] {because}, but the rest of the step fails: `{rest}`"),
            );
            false
        }
        Judgement::Undecided(reason) => {
            tracing::info!(
                plan_id,
                task_id,
                step = %step,
                reason = %reason,
                "verify step failure stands: the baseline comparison cannot tell it from a \
                 pre-existing one"
            );
            false
        }
    }
}

/// How a settlement counts pre-existing failures: "3 tests already failed
/// before the change".
fn already_failed(count: usize) -> String {
    let tests = if count == 1 { "test" } else { "tests" };
    format!("{count} {tests} already failed before the change")
}

/// Append `note` to `verdict`'s output, on a line of its own.
fn append_note(verdict: &mut Verdict, note: &str) {
    verdict.detail = Some(match verdict.detail.take() {
        Some(detail) if !detail.trim().is_empty() => format!("{}\n{note}", detail.trim_end()),
        _ => note.to_string(),
    });
}

// ── Baseline runs ────────────────────────────────────────────────────────

/// A plan run's baselines: the start commit's runs of failed test steps'
/// tests, per (commit, command), and the worktree each commit's runs share,
/// removed when the dispatcher is dropped.
#[derive(Default)]
pub(super) struct Baselines {
    runs: parking_lot::Mutex<HashMap<(String, String), Arc<tokio::sync::OnceCell<Verdict>>>>,
    /// `None` when the commit could not be checked out.
    worktrees: tokio::sync::Mutex<HashMap<String, Option<Arc<BaselineWorktree>>>>,
}

impl Baselines {
    /// The worktree of `commit` in `repository`, checked out on first use.
    async fn worktree(&self, repository: &Path, commit: &str) -> Option<Arc<BaselineWorktree>> {
        let mut worktrees = self.worktrees.lock().await;
        if let Some(worktree) = worktrees.get(commit) {
            return worktree.clone();
        }
        let worktree = BaselineWorktree::add(repository, commit)
            .await
            .map(Arc::new);
        if worktree.is_none() {
            tracing::warn!(
                commit,
                "could not check the plan run's start commit out for a verify baseline"
            );
        }
        worktrees.insert(commit.to_string(), worktree.clone());
        worktree
    }
}

/// The plan run of `attempt_key` (`{run}:{plan}:{task}:{attempt}`).
fn attempt_run<'a>(attempt_key: &'a str, plan_id: &str, task_id: &str) -> Option<&'a str> {
    let (chain, _) = attempt_key.rsplit_once(':')?;
    chain.strip_suffix(&format!(":{plan_id}:{task_id}"))
}

/// The text of `task` that says what it is for: its title, description and
/// acceptance.
fn task_text(task: &TaskDef) -> String {
    let mut text = task.title.clone();
    if let Some(description) = &task.description {
        text.push('\n');
        text.push_str(description);
    }
    for criterion in &task.acceptance {
        text.push('\n');
        text.push_str(criterion);
    }
    text
}

impl GraphTaskDispatcher {
    /// Judge a verify step of `task` that still fails after sibling
    /// settlement, as `failed`, against its baseline (see the module docs):
    /// `None` when the filter does not apply (it is off, in FAST mode, on an
    /// attempt that changed nothing, or to a step that runs no test runner).
    ///
    /// The step is done reading the attempt's tree in `workdir`: its runs
    /// here read it as the step does ([`Self::run_on_attempt`]), and the
    /// baseline runs in a checkout of its own.
    #[allow(clippy::too_many_arguments)]
    pub(super) async fn judge_against_baseline(
        &self,
        spec: &TaskExecutionSpec,
        task: &TaskDef,
        attempt_key: &str,
        workdir: &Path,
        step_label: &str,
        step: &VerifyStep,
        failed: &Verdict,
        unchanged_tree: bool,
    ) -> Option<Judgement> {
        if !self.config.gates.baseline_filter
            || unchanged_tree
            || crate::runner::gate_dispatch::fast_mode_enabled()
            || !TestRunner::any_in(&step.command)
        {
            return None;
        }
        let undecided = |reason: &str| Some(Judgement::Undecided(reason.to_string()));
        let Some(command) = TestCommand::of(&step.command) else {
            return undecided(
                "its command runs tests in more than one part, after a part that is not a \
                 read-only check, or stops at the first failure",
            );
        };
        // Tests that failed show the parts before them passed.
        if !command
            .runner
            .tests_failed(failed.detail.as_deref().unwrap_or_default())
        {
            return undecided(
                "no test failed in it: its tests did not build, or another part of its command \
                 failed",
            );
        }
        let Some(base) = self
            .run_base_commit(attempt_run(attempt_key, &spec.plan_id, &task.id))
            .await
        else {
            return undecided("the plan run's start commit is unknown");
        };
        let now = if command.tests == step.command.trim() {
            failed.clone()
        } else {
            self.run_on_attempt(spec, task, workdir, step_label, step, &command.tests)
                .await
        };
        let Some(before) = self
            .baseline_run(spec, task, &base, &command.tests, step_label, step)
            .await
        else {
            return undecided("the baseline did not run");
        };
        let text = task_text(task);
        let changed = self.changed_files(attempt_key);
        let touched = if command.runner.runs_files() {
            touched_tests(command.runner, workdir, &changed, &text)
        } else {
            Vec::new()
        };
        // A Python suite that named no failure on the start commit, as a
        // pytest run stopped at collection does, is judged by the tests the
        // attempt adds or touches (bug-5089fb).
        let named_before = before.passed
            || command
                .runner
                .failed_tests(before.detail.as_deref().unwrap_or_default())
                .is_some_and(|failed| !failed.is_empty());
        let judgement = if !named_before
            && command.runner.runs_files()
            && !roko_gate::verdict_timed_out(&before)
        {
            self.judge_by_touched_tests(spec, task, workdir, step_label, step, &command, touched)
                .await
        } else {
            judge(command.runner, &now, &before, |test| {
                concerns_task(test, &text, &task.files)
                    || touched
                        .iter()
                        .any(|file| match command.runner.failure_in(test, file) {
                            // A test file that could not load on the start
                            // commit, which the attempt touches, must load.
                            Some(true) => true,
                            // A test in a file the attempt edits is its own.
                            Some(false) => changed.contains(file),
                            None => false,
                        })
            })
        };
        // What its command runs after the tests must pass too.
        let because = match &judgement {
            Judgement::Preexisting(preexisting) => Some(format!(
                "its tests failed only on tests that failed before this run too ({})",
                preexisting.join(", ")
            )),
            Judgement::Scoped { files, .. } => Some(format!(
                "its tests failed only as the suite did before the change, and the tests this \
                 change adds or touches pass ({})",
                files.join(", ")
            )),
            _ => None,
        };
        let (Some(because), Some(rest)) = (because, command.rest) else {
            return Some(judgement);
        };
        let rest_run = self
            .run_on_attempt(spec, task, workdir, step_label, step, &rest)
            .await;
        Some(if rest_run.passed {
            judgement
        } else {
            Judgement::RestFails {
                because,
                rest,
                verdict: Box::new(rest_run),
            }
        })
    }

    /// Judge a step whose suite named no failure on the plan run's start
    /// commit by `files`, the test files the attempt adds or touches
    /// ([`touched_tests`]), run alone on its tree: at least one test must
    /// run, and every one pass (bug-5089fb).
    #[allow(clippy::too_many_arguments)]
    async fn judge_by_touched_tests(
        &self,
        spec: &TaskExecutionSpec,
        task: &TaskDef,
        workdir: &Path,
        step_label: &str,
        step: &VerifyStep,
        command: &TestCommand,
        files: Vec<String>,
    ) -> Judgement {
        let Some(scoped) =
            command
                .runner
                .scoped(&command.part, command.runner_end, &files, workdir)
        else {
            return Judgement::Unverified(
                "the suite already failed on the plan run's start commit without naming its \
                 failures, and this change adds or touches no test file that can run in its \
                 place, so nothing verifies it: add or edit a test that exercises the change"
                    .to_string(),
            );
        };
        tracing::info!(
            plan_id = %spec.plan_id,
            task_id = %task.id,
            command = %scoped,
            "a verify step's suite named no failure on the plan run's start commit; running the \
             tests the attempt adds or touches"
        );
        let run = self
            .run_on_attempt(spec, task, workdir, step_label, step, &scoped)
            .await;
        let passed = command
            .runner
            .all_passed(run.detail.as_deref().unwrap_or_default())
            .filter(|passed| run.passed && *passed > 0);
        match passed {
            Some(passed) => Judgement::Scoped {
                files,
                command: scoped,
                passed,
            },
            None => Judgement::ScopedFails {
                files,
                verdict: Box::new(run),
            },
        }
    }

    /// The commit the plan run started from, as its manifest records it,
    /// else the repository's `HEAD`.
    async fn run_base_commit(&self, run_id: Option<&str>) -> Option<String> {
        let layout = RokoLayout::for_project(&self.workdir);
        let recorded = run_id
            .and_then(|run_id| {
                roko_learn::telemetry::RunProvenanceManifest::load(&layout.run_dir(run_id))
                    .ok()
                    .flatten()
            })
            .and_then(|manifest| manifest.workspace.base_commit)
            .filter(|commit| !commit.trim().is_empty());
        if recorded.is_some() {
            return recorded;
        }
        head_commit(&self.workdir).await
    }

    /// `command` run on `base`, once per run, in the base commit's worktree,
    /// with a target directory of its own, or as an earlier run from `base`
    /// kept it ([`kept_baseline`]); `None` when it could not run. A run that
    /// timed out is returned but not kept.
    async fn baseline_run(
        &self,
        spec: &TaskExecutionSpec,
        task: &TaskDef,
        base: &str,
        command: &str,
        step_label: &str,
        step: &VerifyStep,
    ) -> Option<Verdict> {
        let run = Arc::clone(
            self.baselines
                .runs
                .lock()
                .entry((base.to_string(), command.to_string()))
                .or_default(),
        );
        let mut timed_out = None;
        let timed_out_slot = &mut timed_out;
        let kept = run
            .get_or_try_init(|| async move {
                if let Some(kept) = kept_baseline(&self.workdir, base, command) {
                    return Ok(kept);
                }
                let worktree = self
                    .baselines
                    .worktree(&self.workdir, base)
                    .await
                    .ok_or(())?;
                let payload = GatePayload::in_dir(worktree.path())
                    .with_label(format!("{}/{} baseline", spec.plan_id, task.id))
                    .with_target_dir(worktree.path().join("target"))
                    .with_env_passthrough(self.config.gates.env_passthrough.iter().cloned());
                tracing::info!(
                    plan_id = %spec.plan_id,
                    task_id = %task.id,
                    base,
                    command,
                    "running a failed verify step's tests on the plan run's start commit"
                );
                let verdict = self
                    .run_verify_command(
                        spec,
                        task,
                        &self.workdir,
                        &payload,
                        command,
                        step_label,
                        step,
                    )
                    .await;
                if roko_gate::verdict_timed_out(&verdict) {
                    // A cold build may not finish in one step's time: the
                    // next failure tries again, on what this one built.
                    *timed_out_slot = Some(verdict);
                    return Err(());
                }
                keep_baseline(&self.workdir, base, command, &verdict);
                Ok(verdict)
            })
            .await
            .cloned();
        kept.ok().or(timed_out)
    }

    /// Run `command` for verify step `step_label` of `task` on the attempt's
    /// tree in `workdir`, reading it as the step's own run does: once no
    /// sibling is mid-edit where the step reads, keeping siblings from
    /// starting to edit there meanwhile.
    async fn run_on_attempt(
        &self,
        spec: &TaskExecutionSpec,
        task: &TaskDef,
        workdir: &Path,
        step_label: &str,
        step: &VerifyStep,
        command: &str,
    ) -> Verdict {
        let scope = super::sibling_settle::StepScope::of(step, workdir);
        let _reading = self
            .in_flight
            .begin_step(&super::sibling_settle::StepRead {
                plan_id: &spec.plan_id,
                task_id: &task.id,
                label: step_label,
                workdir,
                scope: &scope,
                limit: Duration::from_secs(self.config.gates.sibling_settle_secs),
            })
            .await;
        let payload = GatePayload::in_dir(workdir)
            .with_label(format!("{}/{}", spec.plan_id, task.id))
            .with_env_passthrough(self.config.gates.env_passthrough.iter().cloned());
        self.run_verify_command(spec, task, workdir, &payload, command, step_label, step)
            .await
    }

    /// Run `command` as verify step `step_label` with `payload`, queued on
    /// the compile lock of `lock_dir`'s repository as the step's own run is.
    #[allow(clippy::too_many_arguments)]
    async fn run_verify_command(
        &self,
        spec: &TaskExecutionSpec,
        task: &TaskDef,
        lock_dir: &Path,
        payload: &GatePayload,
        command: &str,
        step_label: &str,
        step: &VerifyStep,
    ) -> Verdict {
        let _compile_permit = super::verification::verify_compile_permit(
            lock_dir,
            self.config.gates.compile_concurrency,
            step,
            &spec.plan_id,
            &task.id,
        )
        .await;
        run_gate_command(payload, command, step_label, step).await
    }
}

/// Run `command` as verify step `step_label` with `payload`.
async fn run_gate_command(
    payload: &GatePayload,
    command: &str,
    step_label: &str,
    step: &VerifyStep,
) -> Verdict {
    let signal = Signal::builder(Kind::Task)
        .body(Body::from_json(payload).unwrap_or_else(|_| Body::text("gate-payload-fallback")))
        .build();
    let gate = ShellGate::verify_step(command)
        .with_timeout_ms(step.timeout_ms)
        .with_name(step_label)
        .with_phase(&step.phase);
    gate.verify(&signal, &Context::now()).await
}

/// The repository's `HEAD` commit in `workdir`; `None` outside a repository
/// or before its first commit.
async fn head_commit(workdir: &Path) -> Option<String> {
    let output = tokio::time::timeout(
        GIT_TIMEOUT,
        tokio::process::Command::new("git")
            .args(["rev-parse", "--verify", "HEAD"])
            .current_dir(workdir)
            .env("GIT_TERMINAL_PROMPT", "0")
            .stdin(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .kill_on_drop(true)
            .output(),
    )
    .await
    .ok()?
    .ok()?;
    let head = String::from_utf8_lossy(&output.stdout).trim().to_string();
    (output.status.success() && !head.is_empty()).then_some(head)
}

// ── Baselines kept per commit ────────────────────────────────────────────

/// `.roko/state/baselines/<commit>.json`: the verdicts check commands got
/// on `commit`, by command; `None` for a commit that is not a hex id.
fn baseline_path(workdir: &Path, commit: &str) -> Option<PathBuf> {
    (!commit.is_empty() && commit.bytes().all(|byte| byte.is_ascii_hexdigit())).then(|| {
        RokoLayout::for_project(workdir)
            .state_dir()
            .join("baselines")
            .join(format!("{commit}.json"))
    })
}

/// The verdicts kept for `commit`, by command.
fn kept_baselines(path: &Path) -> HashMap<String, Verdict> {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

/// The verdict `command` got on `commit` in an earlier run, if it was kept.
fn kept_baseline(workdir: &Path, commit: &str, command: &str) -> Option<Verdict> {
    kept_baselines(&baseline_path(workdir, commit)?).remove(command)
}

/// Keep `verdict`, the run of `command` on `commit`, for later runs from the
/// same commit. A write that fails costs only a later run of it.
fn keep_baseline(workdir: &Path, commit: &str, command: &str, verdict: &Verdict) {
    let Some(path) = baseline_path(workdir, commit) else {
        return;
    };
    let mut kept = kept_baselines(&path);
    kept.insert(command.to_string(), verdict.clone());
    if let Err(error) = roko_fs::atomic::atomic_write_json(&path, &kept) {
        tracing::warn!(
            path = %path.display(),
            %error,
            "could not keep a verify step's run on the start commit"
        );
    }
}

// ── Tests a change adds or touches ───────────────────────────────────────

/// The most test files a step is scoped to.
const TOUCHED_TESTS_LIMIT: usize = 50;

/// The most files [`test_files`] looks at.
const TEST_FILES_SCAN_LIMIT: usize = 20_000;

/// Directories no test file of the project is under.
const SKIPPED_DIRS: &[&str] = &[
    "node_modules",
    "target",
    "__pycache__",
    "venv",
    "site-packages",
    "build",
    "dist",
];

/// The test files `runner` collects in `workdir`, as relative paths with
/// `/` separators, outside hidden directories and [`SKIPPED_DIRS`].
fn test_files(runner: TestRunner, workdir: &Path) -> Vec<String> {
    let mut found = Vec::new();
    let mut seen = 0;
    let mut pending = vec![(workdir.to_path_buf(), 0)];
    while let Some((dir, depth)) = pending.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            seen += 1;
            if seen > TEST_FILES_SCAN_LIMIT {
                return found;
            }
            let name = entry.file_name().to_string_lossy().into_owned();
            let path = entry.path();
            let Ok(kind) = entry.file_type() else {
                continue;
            };
            if kind.is_dir() {
                if depth < 8 && !name.starts_with('.') && !SKIPPED_DIRS.contains(&name.as_str()) {
                    pending.push((path, depth + 1));
                }
            } else if let Ok(relative) = path.strip_prefix(workdir) {
                let relative = relative.to_string_lossy().replace('\\', "/");
                if runner.is_test_file(&relative) {
                    found.push(relative);
                }
            }
        }
    }
    found
}

/// The module a Python file is, by its path (`synth/env.py` is
/// `synth.env`, `synth/__init__.py` is `synth`); `None` for another file.
fn python_module(path: &str) -> Option<String> {
    let module = path.strip_suffix(".py")?.replace('/', ".");
    let module = module.strip_suffix(".__init__").unwrap_or(&module);
    (!module.is_empty()).then(|| module.to_string())
}

/// The modules a Python source imports, as dotted names: `import a.b` gives
/// `a.b`; `from a import b, c` gives `a`, `a.b` and `a.c`. Relative imports
/// are left out.
fn python_imports(source: &str) -> Vec<String> {
    let mut modules = Vec::new();
    for line in source.lines().map(str::trim) {
        if let Some(names) = line.strip_prefix("import ") {
            modules.extend(
                names
                    .split(',')
                    .filter_map(|name| name.split_whitespace().next())
                    .map(str::to_string),
            );
        } else if let Some((module, names)) = line
            .strip_prefix("from ")
            .and_then(|rest| rest.split_once(" import "))
        {
            let module = module.trim();
            if module.starts_with('.') {
                continue;
            }
            modules.push(module.to_string());
            modules.extend(
                names
                    .trim_matches(|c| c == '(' || c == ')')
                    .split(',')
                    .filter_map(|name| name.split_whitespace().next())
                    .filter(|name| *name != "*")
                    .map(|name| format!("{module}.{name}")),
            );
        }
    }
    modules
}

/// Whether importing `imported` loads `changed`, a module the attempt
/// changed: it is that module, a module in that package, or the same module
/// under a shorter path (`pkg.env` for `src/pkg/env.py`).
fn imports_module(imported: &str, changed: &str) -> bool {
    imported == changed
        || imported.starts_with(&format!("{changed}."))
        || changed.ends_with(&format!(".{imported}"))
}

/// The test files of `runner` in `workdir` that stand in for a suite that
/// named no failure on the plan run's start commit (bug-5089fb): those the
/// attempt changed (`changed`), those whose name the task's `text` names
/// (`tests.test_env`), and those that import a module the attempt changed.
/// Relative paths, sorted, at most [`TOUCHED_TESTS_LIMIT`].
fn touched_tests(
    runner: TestRunner,
    workdir: &Path,
    changed: &[String],
    text: &str,
) -> Vec<String> {
    let modules: Vec<String> = changed
        .iter()
        .filter(|path| !runner.is_test_file(path))
        .filter_map(|path| python_module(path))
        .collect();
    let mut touched: Vec<String> = test_files(runner, workdir)
        .into_iter()
        .filter(|path| {
            let stem = Path::new(path)
                .file_stem()
                .and_then(|stem| stem.to_str())
                .unwrap_or_default();
            changed.contains(path)
                || (stem.len() > 3 && mentions(text, stem))
                || (!modules.is_empty()
                    && std::fs::read_to_string(workdir.join(path)).is_ok_and(|source| {
                        python_imports(&source).iter().any(|imported| {
                            modules
                                .iter()
                                .any(|module| imports_module(imported, module))
                        })
                    }))
        })
        .collect();
    touched.sort();
    touched.truncate(TOUCHED_TESTS_LIMIT);
    touched
}

// ── A prompt run's checks on its base ────────────────────────────────────

/// The most failing tests a [`BaseCheck`] lists.
const LISTED_TESTS_LIMIT: usize = 12;

/// A prompt run's check that already fails on its base commit
/// ([`prompt_base_checks`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BaseCheck {
    /// The check's command, as the run's task has it.
    pub(crate) command: String,
    /// The base commit, abbreviated.
    pub(crate) base: String,
    /// The tests that fail there, when its output names them all.
    pub(crate) failing: Option<Vec<String>>,
}

impl BaseCheck {
    /// The check on the base, for people: "the check `…` already fails on
    /// the base commit 0123456789ab: 3 tests already failed before the
    /// change: test_osc, test_render, test_svf".
    pub(crate) fn summary(&self) -> String {
        let Self {
            command,
            base,
            failing,
        } = self;
        match failing {
            Some(failing) => {
                let mut names: Vec<String> =
                    failing.iter().take(LISTED_TESTS_LIMIT).cloned().collect();
                if failing.len() > LISTED_TESTS_LIMIT {
                    names.push(format!("and {} more", failing.len() - LISTED_TESTS_LIMIT));
                }
                format!(
                    "the check `{command}` already fails on the base commit {base}: {}: {}",
                    already_failed(failing.len()),
                    names.join(", ")
                )
            }
            None => format!(
                "the check `{command}` already fails on the base commit {base}, and its output \
                 names no failing test"
            ),
        }
    }

    /// What the run's agent is told, as a `[task.context]` anti-pattern,
    /// which its prompt shows after "Avoid: ".
    pub(crate) fn agent_note(&self) -> String {
        let summary = self.summary();
        if self.failing.is_some() {
            format!(
                "chasing test failures that predate this change: {summary}. They are not this \
                 task's to fix unless the request asks for them: the check passes when every \
                 test that passed before still passes and the tests this change adds or edits \
                 pass."
            )
        } else {
            format!(
                "ending without a test that runs: {summary}, as when a test module cannot be \
                 imported. So the change is judged by the test files it adds or edits, the ones \
                 the request names, and the ones that import a module it changes: at least one \
                 test must run, and every one must pass."
            )
        }
    }
}

/// The checks of a prompt run that already fail on its base commit, run
/// before its agent runs (bug-5089fb): those of `steps` that run a Python
/// test suite (unittest or pytest).
///
/// A prompt run's checks are the workspace's rungs, which should pass before
/// the change too (`expect = "pass_on_base"`), so a half-built project whose
/// suite is red before the change could never pass one. Each check runs once
/// per base commit, in a checkout of its own, and its run is kept
/// ([`keep_baseline`]), so the judgement of a failed attempt against the base
/// finds it there. Cargo suites are left to that judgement: a base build in a
/// target directory of its own costs more than it saves. None when the
/// baseline filter is off, in FAST mode, or outside a git repository.
pub(crate) async fn prompt_base_checks(
    workdir: &Path,
    steps: &[VerifyStep],
    gates: &roko_core::config::GatesConfig,
) -> Vec<BaseCheck> {
    if !gates.baseline_filter || crate::runner::gate_dispatch::fast_mode_enabled() {
        return Vec::new();
    }
    let checks: Vec<(usize, &VerifyStep, TestCommand)> = steps
        .iter()
        .enumerate()
        .filter_map(|(index, step)| {
            let command = TestCommand::of(&step.command)?;
            command
                .runner
                .runs_files()
                .then_some((index, step, command))
        })
        .collect();
    if checks.is_empty() {
        return Vec::new();
    }
    let Some(base) = head_commit(workdir).await else {
        return Vec::new();
    };
    let mut worktree: Option<Option<BaselineWorktree>> = None;
    let mut failing = Vec::new();
    for (index, step, command) in checks {
        let verdict = match kept_baseline(workdir, &base, &command.tests) {
            Some(kept) => kept,
            None => {
                if worktree.is_none() {
                    worktree = Some(BaselineWorktree::add(workdir, &base).await);
                }
                let Some(Some(checkout)) = &worktree else {
                    break;
                };
                let label = super::verification::verify_step_label(index, &step.phase);
                let payload = GatePayload::in_dir(checkout.path())
                    .with_label(format!("{label} on the base commit"))
                    .with_target_dir(checkout.path().join("target"))
                    .with_env_passthrough(gates.env_passthrough.iter().cloned());
                tracing::info!(
                    base = %base,
                    command = %command.tests,
                    "running a prompt run's check on its base commit before its agent runs"
                );
                let verdict = run_gate_command(&payload, &command.tests, &label, step).await;
                if roko_gate::verdict_timed_out(&verdict) {
                    continue;
                }
                keep_baseline(workdir, &base, &command.tests, &verdict);
                verdict
            }
        };
        if !verdict.passed {
            failing.push(BaseCheck {
                command: step.command.clone(),
                base: base.get(..12).unwrap_or(&base).to_string(),
                failing: command
                    .runner
                    .failed_tests(verdict.detail.as_deref().unwrap_or_default())
                    .filter(|failed| !failed.is_empty())
                    .map(|failed| failed.into_iter().collect()),
            });
        }
    }
    if let Some(Some(checkout)) = worktree {
        checkout.remove().await;
    }
    failing
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `cargo test` output of the lib target with `failing` tests failed and
    /// one passing, as the gate output keeps it: the test binary's stdout,
    /// then cargo's stderr.
    fn libtest_output(failing: &[&str]) -> String {
        let mut stdout = format!(
            "\nrunning {} tests\ntest tests::passes ... ok\n",
            failing.len() + 1
        );
        for name in failing {
            stdout.push_str(&format!("test {name} ... FAILED\n"));
        }
        stdout.push_str(&format!(
            "\ntest result: {}. 1 passed; {} failed; 0 ignored; 0 measured; 0 filtered out\n",
            if failing.is_empty() { "ok" } else { "FAILED" },
            failing.len()
        ));
        format!(
            "{stdout}\n---stderr---\n   Compiling demo v0.1.0 (/work/demo)\n    Finished `test` \
             profile in 0.5s\n     Running unittests src/lib.rs (target/debug/deps/demo-0f1e2d)\n"
        )
    }

    /// The failed tests libtest's `output` names.
    fn failed_tests(output: &str) -> Option<BTreeSet<String>> {
        TestRunner::Cargo.failed_tests(output)
    }

    /// A failed run of a cargo test command, as `ShellGate` reports it.
    fn failed_run(output: &str) -> Verdict {
        Verdict::fail("verify[0:test]", "exit code: 101")
            .with_detail(output)
            .with_error_digest(roko_gate::render_failure_classification(
                &roko_gate::classify_gate_failure("verify[0:test]", output),
            ))
    }

    #[test]
    fn failed_tests_name_each_failure_by_its_target() {
        let failed = failed_tests(&libtest_output(&["tests::a", "tests::b"])).expect("parsed");
        assert_eq!(
            failed.into_iter().collect::<Vec<_>>(),
            [
                "unittests src/lib.rs: tests::a",
                "unittests src/lib.rs: tests::b"
            ]
        );
        assert_eq!(failed_tests(&libtest_output(&[])), Some(BTreeSet::new()));

        // Two targets, their headers on stderr after both binaries' output.
        let two_targets = "running 1 test\ntest a ... FAILED\n\ntest result: FAILED. 0 passed; \
                           1 failed; 0 ignored\n\nrunning 1 test\ntest a ... FAILED\n\ntest \
                           result: FAILED. 0 passed; 1 failed; 0 ignored\n---stderr---\n     \
                           Running unittests src/lib.rs (target/debug/deps/demo-1)\n     \
                           Running tests/it.rs (target/debug/deps/it-2)\n";
        assert_eq!(
            failed_tests(two_targets).map(|failed| failed.into_iter().collect::<Vec<_>>()),
            Some(vec![
                "tests/it.rs: a".to_string(),
                "unittests src/lib.rs: a".to_string()
            ])
        );

        // A count the named failures do not account for, a binary that
        // never printed its result, or no results at all.
        let unnamed = libtest_output(&["tests::a"]).replace("test tests::a ... FAILED\n", "");
        assert_eq!(failed_tests(&unnamed), None);
        let crashed = libtest_output(&["tests::a"]).replace("\ntest result:", "\nno result:");
        assert_eq!(failed_tests(&crashed), None);
        let headless = libtest_output(&["tests::a"]).replace("Running", "Started");
        assert_eq!(failed_tests(&headless), None);
        assert_eq!(
            failed_tests("error[E0308]: mismatched types\nerror: could not compile `demo`"),
            None
        );
    }

    #[test]
    fn test_commands_split_around_their_tests() {
        let split =
            |command: &str| TestCommand::of(command).map(|command| (command.tests, command.rest));
        assert_eq!(
            split("cargo test -p demo"),
            Some(("cargo test --no-fail-fast -p demo".to_string(), None))
        );
        assert_eq!(
            split(
                "test -f src/widget.rs && grep -q 'fn widget' src/lib.rs 2>/dev/null && \
                 cargo +nightly test -p demo 2>&1 | tail -50"
            ),
            Some((
                "cargo +nightly test --no-fail-fast -p demo 2>&1 | tail -50".to_string(),
                None
            ))
        );
        assert_eq!(
            split("cargo test --no-fail-fast && cargo clippy -p demo -- -D warnings"),
            Some((
                "cargo test --no-fail-fast".to_string(),
                Some("cargo clippy -p demo -- -D warnings".to_string())
            ))
        );
        for command in [
            "cargo test -p a && cargo test -p b",
            "cd crates/demo && cargo test",
            "cargo test; cargo clippy",
            "cargo test || true",
            "cargo test & wait",
            "(cargo test)",
            "cargo test -p \"$(echo demo)\"",
            "cargo build | tee log && cargo test",
        ] {
            assert_eq!(split(command), None, "{command}");
        }
        assert!(TestRunner::any_in("grep -q x y && cargo test -p demo"));
        assert!(!TestRunner::any_in("cargo clippy -p demo -- -D warnings"));
        assert!(!TestRunner::any_in("cargo nextest run -p demo"));
        let unittest = TestCommand::of(
            "test -d tests && PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s tests -q",
        )
        .expect("a unittest step");
        assert_eq!(unittest.runner, TestRunner::Unittest);
        assert_eq!(
            unittest.tests,
            "PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s tests -q"
        );
        assert_eq!(TestCommand::of("python3 -m pytest -x tests"), None);
    }

    #[test]
    fn a_failure_that_failed_before_is_preexisting() {
        let output = libtest_output(&["tests::broken_before"]);
        assert_eq!(
            judge(
                TestRunner::Cargo,
                &failed_run(&output),
                &failed_run(&output),
                |_| false
            ),
            Judgement::Preexisting(vec![
                "unittests src/lib.rs: tests::broken_before".to_string()
            ])
        );
    }

    #[test]
    fn a_failure_the_baseline_passed_is_new() {
        let judgement = judge(
            TestRunner::Cargo,
            &failed_run(&libtest_output(&["tests::a", "tests::b"])),
            &failed_run(&libtest_output(&["tests::a"])),
            |_| false,
        );
        assert_eq!(
            judgement,
            Judgement::New {
                new: vec!["unittests src/lib.rs: tests::b".to_string()],
                owned: Vec::new(),
                preexisting: vec!["unittests src/lib.rs: tests::a".to_string()],
            }
        );
    }

    /// A test the task owns is its to fix, whatever the baseline says; a
    /// clean baseline makes every failure new; and what names no failing
    /// tests, a build error the start commit shares say, is never excused.
    #[test]
    fn baseline_judgement_never_excuses_what_it_cannot_tell() {
        let output = libtest_output(&["tests::broken_before"]);
        let owned = judge(
            TestRunner::Cargo,
            &failed_run(&output),
            &failed_run(&output),
            |test| test.ends_with("broken_before"),
        );
        assert!(
            matches!(&owned, Judgement::New { owned, .. } if owned.len() == 1),
            "{owned:?}"
        );

        let clean = Verdict::pass("verify[0:test]").with_detail(libtest_output(&[]));
        let regression = judge(TestRunner::Cargo, &failed_run(&output), &clean, |_| false);
        assert!(
            matches!(&regression, Judgement::New { new, .. } if new.len() == 1),
            "{regression:?}"
        );

        let compile = "error[E0308]: mismatched types\nerror: could not compile `demo`";
        let crashed = output.replace("\ntest result:", "\nno result:");
        for (now, before) in [
            (failed_run(compile), failed_run(compile)),
            (failed_run(&crashed), failed_run(&output)),
            (failed_run(&output), failed_run(compile)),
            (clean.clone(), failed_run(&output)),
        ] {
            let judgement = judge(TestRunner::Cargo, &now, &before, |_| false);
            assert!(
                matches!(judgement, Judgement::Undecided(_)),
                "{judgement:?}"
            );
        }
    }

    #[test]
    fn a_test_the_task_names_or_edits_is_its_own() {
        let unit = "unittests src/lib.rs: routes::jobs::tests::cancel_returns_200";
        let integration = "tests/job_lifecycle.rs: cancel_returns_200";
        let doc = "doc-tests demo: src/routes/jobs.rs - routes::jobs::cancel (line 12)";
        let none: &[String] = &[];
        assert!(concerns_task(unit, "Fix cancel_returns_200", none));
        assert!(concerns_task(unit, "Add a jobs endpoint", none));
        assert!(concerns_task(integration, "Make job_lifecycle pass", none));
        assert!(concerns_task(doc, "Document jobs", none));
        assert!(concerns_task(
            unit,
            "Add a widget",
            &["crates/demo/src/routes/jobs.rs".to_string()]
        ));
        assert!(!concerns_task(unit, "Add a widget", none));
        assert!(!concerns_task(unit, "Add a jobsite and more tests", none));
        assert!(!concerns_task(
            "unittests src/lib.rs: tests::adds",
            "Add tests",
            &["crates/demo/src/lib.rs".to_string()]
        ));
    }

    #[test]
    fn applying_a_judgement_says_so_in_the_step_output() {
        let output = libtest_output(&["tests::broken_before"]);
        let mut verdict = failed_run(&output);
        assert!(apply(
            &mut verdict,
            Judgement::Preexisting(vec!["unittests src/lib.rs: tests::broken_before".into()]),
            "p1",
            "T01",
        ));
        assert!(verdict.passed);
        assert_eq!(verdict.gate, "pre-existing-filtered:verify[0:test]");
        let detail = verdict.detail.as_deref().unwrap_or_default();
        assert!(
            detail
                .lines()
                .last()
                .is_some_and(|line| line.starts_with("[roko] pre-existing failures")
                    && line.ends_with("tests::broken_before")),
            "{detail}"
        );

        let mut verdict = failed_run(&output);
        let rest_run = Verdict::fail("verify[0:test]", "exit code: 1").with_detail("lint error\n");
        assert!(!apply(
            &mut verdict,
            Judgement::RestFails {
                because: "its tests failed only on tests that failed before this run too \
                          (unittests src/lib.rs: tests::broken_before)"
                    .into(),
                rest: "cargo clippy".into(),
                verdict: Box::new(rest_run),
            },
            "p1",
            "T01",
        ));
        assert!(!verdict.passed);
        assert_eq!(verdict.reason, "exit code: 1");
        let detail = verdict.detail.as_deref().unwrap_or_default();
        assert!(
            detail.starts_with("lint error\n[roko] its tests failed only"),
            "{detail}"
        );
        assert!(
            detail.ends_with("the rest of the step fails: `cargo clippy`"),
            "{detail}"
        );

        let mut verdict = failed_run(&output);
        assert!(!apply(
            &mut verdict,
            Judgement::Undecided("the baseline did not run".into()),
            "p1",
            "T01",
        ));
        assert_eq!(verdict, failed_run(&output));
    }

    /// A unittest run on pocket-synth's seed (bug-5089fb): its test modules
    /// in `missing` could not import, the others ran `passing` tests that
    /// passed and `failing` that failed.
    fn unittest_run(missing: &[&str], passing: usize, failing: &[&str]) -> Verdict {
        let mut output = String::from("---stderr---\n");
        for module in missing {
            output.push_str(&format!(
                "======\nERROR: {module} (unittest.loader._FailedTest.{module})\n------\n\
                 ImportError: Failed to import test module: {module}\n\n"
            ));
        }
        for test in failing {
            let (id, name) = test.rsplit_once('.').expect("a test id");
            output.push_str(&format!("======\nFAIL: {name} ({id}.{name})\n------\n\n"));
        }
        let failed = missing.len() + failing.len();
        output.push_str(&format!(
            "------\nRan {} tests in 0.010s\n\n",
            passing + failed
        ));
        if failed == 0 {
            output.push_str("OK\n");
            return Verdict::pass("verify[0:test]").with_detail(output);
        }
        output.push_str(&format!(
            "FAILED (failures={}, errors={})\n",
            failing.len(),
            missing.len()
        ));
        Verdict::fail("verify[0:test]", "exit code: 1").with_detail(output)
    }

    /// bug-5089fb: on a base where every test module fails to import, a
    /// change that makes one import is judged by its tests, while the
    /// modules it does not touch may keep failing; old failures with no test
    /// passing verify nothing.
    #[test]
    fn a_red_python_base_is_judged_by_new_failures() {
        let before = unittest_run(&["test_env", "test_osc", "test_svf"], 0, &[]);
        let fixed = unittest_run(&["test_osc", "test_svf"], 4, &[]);
        assert_eq!(
            judge(TestRunner::Unittest, &fixed, &before, |_| false),
            Judgement::Preexisting(vec!["test_osc".to_string(), "test_svf".to_string()])
        );
        let broken = unittest_run(
            &["test_osc", "test_svf"],
            3,
            &["test_env.ADSRTest.test_level"],
        );
        let judgement = judge(TestRunner::Unittest, &broken, &before, |_| false);
        assert!(
            matches!(&judgement, Judgement::New { new, .. } if new == &["test_env.ADSRTest.test_level"]),
            "a test the change lets run must pass: {judgement:?}"
        );
        let owned = judge(TestRunner::Unittest, &before, &before, |test| {
            test == "test_env"
        });
        assert!(
            matches!(&owned, Judgement::New { owned, .. } if owned == &["test_env"]),
            "{owned:?}"
        );
        let nothing = judge(TestRunner::Unittest, &before, &before, |_| false);
        assert!(
            matches!(&nothing, Judgement::Unverified(reason)
                if reason.starts_with("3 tests already failed before the change")),
            "{nothing:?}"
        );
    }

    #[test]
    fn the_tests_a_change_touches_are_found_by_file_name_and_import() {
        let workdir = tempfile::tempdir().expect("tempdir");
        let write = |path: &str, text: &str| {
            let path = workdir.path().join(path);
            std::fs::create_dir_all(path.parent().expect("a parent")).expect("dirs");
            std::fs::write(path, text).expect("write");
        };
        write(
            "tests/test_env.py",
            "import unittest\nfrom synth.env import ADSR\n",
        );
        write("tests/test_osc.py", "from synth import osc\n");
        write("tests/test_render.py", "import synth.render as render\n");
        write("tests/helpers.py", "from synth.env import ADSR\n");
        write(".venv/lib/test_env.py", "from synth.env import ADSR\n");
        let touched = |changed: &[&str], text: &str| {
            let changed: Vec<String> = changed.iter().map(ToString::to_string).collect();
            touched_tests(TestRunner::Unittest, workdir.path(), &changed, text)
        };
        assert_eq!(touched(&["synth/env.py"], ""), ["tests/test_env.py"]);
        assert_eq!(touched(&["synth/osc.py"], ""), ["tests/test_osc.py"]);
        assert_eq!(
            touched(&["synth/__init__.py"], ""),
            [
                "tests/test_env.py",
                "tests/test_osc.py",
                "tests/test_render.py"
            ]
        );
        assert_eq!(
            touched(&["README.md"], "Make tests.test_render pass"),
            ["tests/test_render.py"]
        );
        assert_eq!(touched(&["tests/test_osc.py"], ""), ["tests/test_osc.py"]);
        assert!(touched(&["README.md"], "Fix the docs").is_empty());
        assert_eq!(
            python_imports("from a.b import (c, d as e)\nimport f, g.h\nfrom . import x\n"),
            ["a.b", "a.b.c", "a.b.d", "f", "g.h"]
        );
        assert_eq!(
            python_module("src/pkg/__init__.py").as_deref(),
            Some("src.pkg")
        );
        assert!(imports_module("pkg.env", "src.pkg.env"));
        assert!(!imports_module("pkg.envelope", "pkg.env"));
    }

    #[test]
    fn a_baseline_is_kept_per_commit_and_command() {
        let workdir = tempfile::tempdir().expect("tempdir");
        let commit = "0123456789abcdef0123456789abcdef01234567";
        let verdict = unittest_run(&["test_env"], 0, &[]);
        assert_eq!(
            kept_baseline(workdir.path(), commit, "python3 -m unittest"),
            None
        );
        keep_baseline(workdir.path(), commit, "python3 -m unittest", &verdict);
        keep_baseline(
            workdir.path(),
            commit,
            "pytest",
            &Verdict::pass("verify[1:test]"),
        );
        assert_eq!(
            kept_baseline(workdir.path(), commit, "python3 -m unittest"),
            Some(verdict.clone())
        );
        assert!(
            workdir
                .path()
                .join(format!(".roko/state/baselines/{commit}.json"))
                .is_file()
        );
        assert_eq!(kept_baseline(workdir.path(), "feedface", "pytest"), None);
        keep_baseline(workdir.path(), "../escape", "pytest", &verdict);
        assert_eq!(kept_baseline(workdir.path(), "../escape", "pytest"), None);
    }

    #[test]
    fn a_base_check_names_the_tests_that_already_fail() {
        let check = BaseCheck {
            command: "python3 -m unittest discover -s tests".to_string(),
            base: "0123456789ab".to_string(),
            failing: Some(vec!["test_osc".to_string(), "test_svf".to_string()]),
        };
        assert_eq!(
            check.summary(),
            "the check `python3 -m unittest discover -s tests` already fails on the base commit \
             0123456789ab: 2 tests already failed before the change: test_osc, test_svf"
        );
        assert!(
            check
                .agent_note()
                .starts_with("chasing test failures that predate")
        );
        let many = BaseCheck {
            failing: Some((0..15).map(|index| format!("t{index:02}")).collect()),
            ..check.clone()
        };
        assert!(
            many.summary().ends_with("t10, t11, and 3 more"),
            "{}",
            many.summary()
        );
        let unnamed = BaseCheck {
            failing: None,
            ..check
        };
        assert!(
            unnamed
                .summary()
                .ends_with("and its output names no failing test")
        );
        assert!(unnamed.agent_note().contains("at least one test must run"));
    }

    #[test]
    fn the_attempt_key_names_its_run() {
        assert_eq!(
            attempt_run("graph-1:p1:T01:3", "p1", "T01"),
            Some("graph-1")
        );
        assert_eq!(attempt_run("graph-1:p2:T01:3", "p1", "T01"), None);
    }

    // ── Through dispatch ─────────────────────────────────────────────────

    use std::os::unix::fs::PermissionsExt;

    use crate::graph_task_dispatch::diff_snapshot::tests::commit_repo;
    use crate::graph_task_dispatch::tests::{
        make_spec, make_test_dispatcher_with, no_auto_fix, verify_step,
    };

    /// A stand-in for `cargo test`, committed with the base: the tests that
    /// `failing.txt` lists fail. Like cargo, it prints its target header on
    /// stderr and the test binary's output on stdout.
    const FAKE_CARGO: &str = r#"#!/bin/sh
failed=$(grep -c . failing.txt)
echo "     Running unittests src/lib.rs (target/debug/deps/demo-0f1e2d)" >&2
echo ""
echo "running $((failed + 1)) tests"
echo "test tests::passes ... ok"
grep . failing.txt | while IFS= read -r name; do echo "test $name ... FAILED"; done
echo ""
if [ "$failed" -eq 0 ]; then
  echo "test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out"
  exit 0
fi
echo "test result: FAILED. 1 passed; $failed failed; 0 ignored; 0 measured; 0 filtered out"
exit 101
"#;

    /// The task's verify step: the crate's tests, through [`FAKE_CARGO`].
    const TEST_STEP: &str = r#"PATH="$PWD/bin:$PATH" cargo test -p demo"#;

    /// A provider whose agent writes `widget.txt` and runs `extra`.
    fn editing_provider(extra: &str) -> String {
        format!(
            r#"#!/bin/sh
set -eu
cat >/dev/null
echo widget > widget.txt
{extra}
printf '%s\n' '{{"type":"content_block_delta","delta":{{"text":"done"}}}}'
printf '%s\n' '{{"type":"result","session_id":"s","model":"claude-sonnet-4-6","total_cost_usd":0.01,"usage":{{"input_tokens":5,"output_tokens":10}}}}'
"#
        )
    }

    /// Commit [`FAKE_CARGO`], as `bin/cargo`, and `failing.txt`, listing
    /// `failing`, as the base of a new git repository in `dir`.
    fn commit_base(dir: &Path, failing: &str) {
        std::fs::create_dir_all(dir.join("bin")).expect("bin dir");
        let cargo = dir.join("bin/cargo");
        std::fs::write(&cargo, FAKE_CARGO).expect("write fake cargo");
        std::fs::set_permissions(&cargo, std::fs::Permissions::from_mode(0o755))
            .expect("make fake cargo executable");
        commit_repo(dir, &[("failing.txt", failing)]);
    }

    /// Dispatch one attempt of a task whose agent runs `extra` after its
    /// edit and whose verify step is `step`, in a repository whose base
    /// fails `tests::broken_before`; with the gate results the dashboard
    /// got, as `(gate, passed, output)`.
    async fn dispatch_over_a_broken_base(
        extra: &str,
        step: &str,
    ) -> (Result<Vec<Signal>>, Vec<(String, bool, String)>) {
        let temp = tempfile::tempdir().expect("tempdir");
        let hub = crate::state_hub::StateHub::new(256);
        let mut events = hub.subscribe_events();
        let (dispatcher, mut task) = make_test_dispatcher_with(
            &temp,
            &editing_provider(extra),
            no_auto_fix,
            GraphFeedbackContext::default(),
            |dispatcher| dispatcher.with_tui_bridge(TuiBridge::new(hub.sender())),
        )
        .await;
        commit_base(temp.path(), "tests::broken_before\n");
        task.verify = vec![verify_step("test", step)];
        let result = dispatcher
            .dispatch(&make_spec(&task), Vec::new(), &CellContext::new())
            .await;
        let mut gates = Vec::new();
        loop {
            match events.try_recv() {
                Ok(envelope) => {
                    if let roko_core::DashboardEvent::GateResult {
                        gate,
                        passed,
                        output_text,
                        ..
                    } = envelope.payload
                    {
                        gates.push((gate, passed, output_text.unwrap_or_default()));
                    }
                }
                Err(tokio::sync::broadcast::error::TryRecvError::Lagged(_)) => {}
                Err(_) => break,
            }
        }
        (result, gates)
    }

    /// The verify failure message of a rejected attempt.
    fn verify_message(result: Result<Vec<Signal>>) -> String {
        match result {
            Err(RokoError::Verify { message, .. }) => message,
            Err(error) => panic!("expected a verify failure, got {error}"),
            Ok(_) => panic!("expected the attempt to be rejected"),
        }
    }

    /// gap-161be1: a test step that fails only on a test that also failed
    /// on the plan run's start commit passes. Its attempt settles as passed
    /// with pre-existing failures, never as a clean pass, and the dashboard's
    /// gate result names the test.
    #[tokio::test]
    async fn preexisting_verify_failure_is_filtered() {
        let (result, gates) = dispatch_over_a_broken_base("", TEST_STEP).await;
        let outputs = result.expect("a failure the base had too does not reject the task");
        assert_eq!(
            TaskGateVerdict::from_signals(&outputs),
            Some(TaskGateVerdict::PassedWithPreexistingFailures)
        );
        let (gate, passed, output) = gates.last().expect("a gate result");
        assert_eq!(gate, "pre-existing-filtered:verify[0:test]", "{gates:?}");
        assert!(*passed, "{gates:?}");
        assert!(
            output.contains(
                "1 test already failed before the change, on the plan run's start commit: \
                 unittests src/lib.rs: tests::broken_before"
            ),
            "{output}"
        );
    }

    /// gap-161be1: a test that passed on the base and fails now rejects the
    /// task, though the base fails another test, and the failure names the
    /// new test apart from the old.
    #[tokio::test]
    async fn new_failure_on_top_of_preexisting_still_rejects() {
        let (result, gates) =
            dispatch_over_a_broken_base("echo tests::broken_now >> failing.txt", TEST_STEP).await;
        let message = verify_message(result);
        let new = message
            .lines()
            .find(|line| line.contains("new failures"))
            .unwrap_or_else(|| panic!("no new failures named: {message}"));
        assert!(new.contains("tests::broken_now"), "{message}");
        assert!(!new.contains("tests::broken_before"), "{message}");
        assert!(
            message
                .lines()
                .any(|line| line.contains("failed before") && line.contains("tests::broken_before")),
            "{message}"
        );
        assert!(
            gates
                .iter()
                .all(|(gate, passed, _)| !passed && !gate.starts_with("pre-existing-filtered")),
            "{gates:?}"
        );
    }

    /// gap-161be1: what a step runs after its tests, which their failure
    /// kept from running, must pass before pre-existing failures are let
    /// go.
    #[tokio::test]
    async fn the_rest_of_a_step_still_runs_after_preexisting_failures() {
        let step = format!("{TEST_STEP} && test -f absent.txt");
        let (result, _) = dispatch_over_a_broken_base("", &step).await;
        let message = verify_message(result);
        assert!(
            message.contains("but the rest of the step fails: `test -f absent.txt`"),
            "{message}"
        );
        assert!(message.contains("tests::broken_before"), "{message}");
    }
}
