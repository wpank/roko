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
//!   task declares a file of that name.
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
//! result is kept per (start commit, command) for the run, so tasks with the
//! same failing step pay for one baseline, and the start commit's worktree
//! serves the run until it ends. A baseline that timed out, on a cold build
//! say, is tried again by the next failure, on what it built.
//!
//! The filter is off with `[gates] baseline_filter = false`, in FAST mode,
//! and for an attempt that changed nothing, whose steps must pass on their
//! own. `cargo nextest` steps are not judged: their output is not libtest's.

use std::collections::{BTreeSet, HashMap};
use std::time::Duration;

use roko_core::Verdict;
use roko_fs::RokoLayout;
use roko_gate::BuildSystem;

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

/// Where each `cargo test` of `command` ends, as a byte offset. A toolchain
/// (`cargo +nightly test`) may come between.
fn cargo_test_ends(command: &str) -> Vec<usize> {
    let mut words = Vec::new();
    let mut start = None;
    for (index, character) in command.char_indices() {
        let separator = character.is_whitespace() || "&|;(){}".contains(character);
        match (separator, start) {
            (true, Some(begin)) => {
                words.push((&command[begin..index], index));
                start = None;
            }
            (false, None) => start = Some(index),
            _ => {}
        }
    }
    if let Some(begin) = start {
        words.push((&command[begin..], command.len()));
    }
    let mut ends = Vec::new();
    for (index, (word, _)) in words.iter().enumerate() {
        if *word != "cargo" {
            continue;
        }
        let mut next = index + 1;
        if words
            .get(next)
            .is_some_and(|(word, _)| word.starts_with('+'))
        {
            next += 1;
        }
        if let Some(&("test", end)) = words.get(next) {
            ends.push(end);
        }
    }
    ends
}

/// Whether `command` runs a crate's tests through `cargo test`, alone or in
/// a composed command.
fn runs_cargo_tests(command: &str) -> bool {
    !cargo_test_ends(command).is_empty()
}

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
    /// The part of the command that runs `cargo test`, with
    /// `--no-fail-fast`: what runs, and is judged, on both sides.
    tests: String,
    /// The parts after it, which its failure kept from running.
    rest: Option<String>,
}

impl TestCommand {
    /// Split `command`, an `&&` chain ([`and_parts`]), around its one
    /// `cargo test`, when only read-only checks ([`is_check`]) come before
    /// it. Once the tests ran, the checks passed: they are left out, since
    /// the start commit may fail them by design.
    fn of(command: &str) -> Option<Self> {
        let parts = and_parts(command)?;
        let ends: Vec<Vec<usize>> = parts.iter().map(|part| cargo_test_ends(part)).collect();
        let mut testing = ends.iter().enumerate().filter(|(_, ends)| !ends.is_empty());
        let (index, part_ends) = testing.next()?;
        if testing.next().is_some() || !parts[..index].iter().all(|part| is_check(part)) {
            return None;
        }
        let &[end] = part_ends.as_slice() else {
            return None;
        };
        let part = parts[index];
        let tests = if part.contains("--no-fail-fast") {
            part.to_string()
        } else {
            format!("{} --no-fail-fast{}", &part[..end], &part[end..])
        };
        let rest = &parts[index + 1..];
        Some(Self {
            tests,
            rest: (!rest.is_empty()).then(|| rest.join(" && ")),
        })
    }
}

// ── Test results ─────────────────────────────────────────────────────────

/// The test target a cargo header line names: `Running unittests src/lib.rs
/// (target/debug/deps/demo-0f1e2d)` names `unittests src/lib.rs`, without
/// the build hash, which differs between checkouts, and `Doc-tests demo`
/// names `doc-tests demo`.
fn test_target(line: &str) -> Option<String> {
    if let Some(running) = line.strip_prefix("Running ") {
        let (source, binary) = running.rsplit_once(" (")?;
        binary.ends_with(')').then(|| source.to_string())
    } else {
        line.strip_prefix("Doc-tests ")
            .map(|name| format!("doc-tests {name}"))
    }
}

/// Whether `line` starts a test binary's run: `running 3 tests`.
fn starts_test_run(line: &str) -> bool {
    line.strip_prefix("running ")
        .and_then(|rest| {
            rest.strip_suffix(" tests")
                .or_else(|| rest.strip_suffix(" test"))
        })
        .is_some_and(|count| !count.is_empty() && count.bytes().all(|byte| byte.is_ascii_digit()))
}

/// The failed tests in a `cargo test` run's output, each named by its test
/// target (`unittests src/lib.rs: tests::adds`). `None` unless every test
/// binary that started printed its result, with its failures named one by
/// one, and the binaries pair up with cargo's target headers.
///
/// Cargo prints the headers on stderr and the test binaries print on
/// stdout, which the gate output keeps apart, so they pair up in order.
fn failed_tests(output: &str) -> Option<BTreeSet<String>> {
    let mut targets = Vec::new();
    // Per test binary that started: the tests it failed, and whether it
    // printed its result.
    let mut runs: Vec<(Vec<&str>, bool)> = Vec::new();
    for line in output.lines().map(str::trim) {
        if let Some(target) = test_target(line) {
            targets.push(target);
        } else if starts_test_run(line) {
            if runs.last().is_some_and(|(_, finished)| !finished) {
                return None;
            }
            runs.push((Vec::new(), false));
        } else if line.starts_with("test result:") {
            let (failed, finished) = runs.last_mut().filter(|(_, finished)| !*finished)?;
            let counts = roko_gate::parse_test_counts(line, BuildSystem::Cargo)?;
            if counts.failed as usize != failed.len() {
                return None;
            }
            *finished = true;
        } else if let Some(name) = line
            .strip_prefix("test ")
            .and_then(|rest| rest.strip_suffix(" ... FAILED"))
        {
            let (failed, _) = runs.last_mut().filter(|(_, finished)| !*finished)?;
            failed.push(name);
        }
    }
    if runs.is_empty() || runs.len() != targets.len() || runs.iter().any(|(_, finished)| !finished)
    {
        return None;
    }
    Some(
        targets
            .iter()
            .zip(&runs)
            .flat_map(|(target, (failed, _))| {
                failed.iter().map(move |name| format!("{target}: {name}"))
            })
            .collect(),
    )
}

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

/// Whether `test` (`<target>: <path>`) is the task's to fix, however it
/// fared before: the task's `text` (title, description, acceptance) names
/// its function, one of its modules or its test file, or one of the task's
/// `files` has one of those names. Names of three characters or fewer, and
/// `tests`, tell nothing.
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
    /// Its tests failed only on `preexisting` failures, but `rest`, the
    /// parts of its command after them, fails on its own: `verdict` is that
    /// run.
    RestFails {
        preexisting: Vec<String>,
        rest: String,
        verdict: Box<Verdict>,
    },
    /// The comparison cannot tell, for this reason: the failure stands.
    Undecided(String),
}

/// Judge the tests' run on the attempt's tree, `now`, against their run on
/// the start commit, `before`, by the names of the tests that fail.
/// `task_owns` tells the tests that are the task's to fix.
fn judge(now: &Verdict, before: &Verdict, task_owns: impl Fn(&str) -> bool) -> Judgement {
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
    let Some(failing_now) =
        failed_tests(now.detail.as_deref().unwrap_or_default()).filter(|failed| !failed.is_empty())
    else {
        return undecided("its output does not name every failing test");
    };
    let failing_before = if before.passed {
        BTreeSet::new()
    } else {
        match failed_tests(before.detail.as_deref().unwrap_or_default()) {
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
    if new.is_empty() && owned.is_empty() {
        Judgement::Preexisting(preexisting)
    } else {
        Judgement::New {
            new,
            owned,
            preexisting,
        }
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
            verdict.reason = format!("failed only on tests that failed before this run: {tests}");
            append_note(
                verdict,
                &format!(
                    "[roko] pre-existing failures, so the step passes: these tests failed on \
                     the plan run's start commit too: {tests}"
                ),
            );
            true
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
                    "new failures, which passed on the plan run's start commit",
                ),
            ] {
                if !tests.is_empty() {
                    append_note(verdict, &format!("[roko] {note}: {}", tests.join(", ")));
                }
            }
            false
        }
        Judgement::RestFails {
            preexisting,
            rest,
            verdict: rest_run,
        } => {
            let tests = preexisting.join(", ");
            tracing::info!(
                plan_id,
                task_id,
                step = %step,
                tests = %tests,
                rest = %rest,
                "verify step's tests failed only on tests that failed on the plan run's start \
                 commit too, but the rest of the step fails"
            );
            *verdict = *rest_run;
            append_note(
                verdict,
                &format!(
                    "[roko] its tests failed only on tests that failed before this run too \
                     ({tests}), but the rest of the step fails: `{rest}`"
                ),
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
    /// attempt that changed nothing, or to a step that runs no `cargo test`).
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
            || !runs_cargo_tests(&step.command)
        {
            return None;
        }
        let undecided = |reason: &str| Some(Judgement::Undecided(reason.to_string()));
        let Some(command) = TestCommand::of(&step.command) else {
            return undecided(
                "its command runs tests in more than one part, or after a part that is not a \
                 read-only check",
            );
        };
        // Tests that failed show the parts before them passed.
        let tests_failed = roko_gate::parse_test_counts(
            failed.detail.as_deref().unwrap_or_default(),
            BuildSystem::Cargo,
        )
        .is_some_and(|counts| counts.failed > 0);
        if !tests_failed {
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
        let judgement = judge(&now, &before, |test| {
            concerns_task(test, &text, &task.files)
        });
        match (judgement, command.rest) {
            (Judgement::Preexisting(preexisting), Some(rest)) => {
                let rest_run = self
                    .run_on_attempt(spec, task, workdir, step_label, step, &rest)
                    .await;
                Some(if rest_run.passed {
                    Judgement::Preexisting(preexisting)
                } else {
                    Judgement::RestFails {
                        preexisting,
                        rest,
                        verdict: Box::new(rest_run),
                    }
                })
            }
            (judgement, _) => Some(judgement),
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
        let output = tokio::time::timeout(
            GIT_TIMEOUT,
            tokio::process::Command::new("git")
                .args(["rev-parse", "--verify", "HEAD"])
                .current_dir(&self.workdir)
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

    /// `command` run on `base`, once per run, in the base commit's worktree,
    /// with a target directory of its own; `None` when it could not run. A
    /// run that timed out is returned but not kept.
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
        let signal = Signal::builder(Kind::Task)
            .body(Body::from_json(payload).unwrap_or_else(|_| Body::text("gate-payload-fallback")))
            .build();
        let gate = ShellGate::new(
            "bash",
            vec![
                "-o".into(),
                "pipefail".into(),
                "-c".into(),
                command.to_string(),
            ],
        )
        .with_timeout_ms(step.timeout_ms)
        .with_name(step_label)
        .with_phase(&step.phase);
        let _compile_permit = super::verification::verify_compile_permit(
            lock_dir,
            self.config.gates.compile_concurrency,
            step,
            &spec.plan_id,
            &task.id,
        )
        .await;
        gate.verify(&signal, &Context::now()).await
    }
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
        assert!(runs_cargo_tests("grep -q x y && cargo test -p demo"));
        assert!(!runs_cargo_tests("cargo clippy -p demo -- -D warnings"));
        assert!(!runs_cargo_tests("cargo nextest run -p demo"));
    }

    #[test]
    fn a_failure_that_failed_before_is_preexisting() {
        let output = libtest_output(&["tests::broken_before"]);
        assert_eq!(
            judge(&failed_run(&output), &failed_run(&output), |_| false),
            Judgement::Preexisting(vec![
                "unittests src/lib.rs: tests::broken_before".to_string()
            ])
        );
    }

    #[test]
    fn a_failure_the_baseline_passed_is_new() {
        let judgement = judge(
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
        let owned = judge(&failed_run(&output), &failed_run(&output), |test| {
            test.ends_with("broken_before")
        });
        assert!(
            matches!(&owned, Judgement::New { owned, .. } if owned.len() == 1),
            "{owned:?}"
        );

        let clean = Verdict::pass("verify[0:test]").with_detail(libtest_output(&[]));
        let regression = judge(&failed_run(&output), &clean, |_| false);
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
            let judgement = judge(&now, &before, |_| false);
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
                preexisting: vec!["unittests src/lib.rs: tests::broken_before".into()],
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
                "the plan run's start commit too: unittests src/lib.rs: tests::broken_before"
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
