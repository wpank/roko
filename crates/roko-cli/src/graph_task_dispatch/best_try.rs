//! The best try so far, per task (gap-fefdf5).
//!
//! A task's retries continue in the checkout its last try left. When that try
//! broke a verify step an earlier try had passed, the next try would start
//! from the broken tree, and each try is a new agent session, so the passing
//! code would be lost. So each failed try's tree is kept with the steps it
//! passed, and with how far it got in the first step it failed, by that
//! step's own runner: the assertions, then the test cases, its summary lines
//! count as passed (gap-54089d). When a try got less far than the task's best
//! try, fewer steps or as many and less far in the same failing step, the
//! best try's files are put back before the next try, whose feedback says so
//! and shows what the failed try had changed. Any other try continues in
//! place, as before (ties go to the latest), and so does a try that got
//! nowhere with no better try before it.
//!
//! Within one try, the tree its own checks saw is kept each time they run,
//! the agent's `run_tests` and the hand-off check alike (gap-56f761). When the
//! tree the try ends with ranks behind the best of them, as a last edit after
//! the try's time bound can make it, that tree is put back and the gate
//! judges it ([`GraphTaskDispatcher::restore_checked_best`]).
//!
//! The best try is kept beside the plan's Graph checkpoint as well
//! ([`retry_feedback::BestTry`]). A resumed run re-attaches the checkout an
//! earlier process kept, which holds the last try's tree, so a task's first
//! try in the new process starts from its best try's tree when that try got
//! anywhere (gap-586507), and otherwise continues in place, as a retry in one
//! process does (gap-54089d).
//!
//! Only a task's own checkout, a per-task worktree, is ever restored: in a
//! shared working tree, sibling tasks' edits would be undone with it.

use std::cmp::Ordering;
use std::collections::HashMap;
use std::path::Path;

use chrono::TimeZone;
use roko_gate::BuildSystem;
use roko_std::tool::builtin::task_checks::{CheckRun, read_check_runs, task_check_runs_path};

use super::diff_snapshot::{git_bytes, git_line, is_roko_state, parse_raw_diff, snapshot_tree};
use super::retry_feedback::{BestTry, StepProgress};
use super::*;

/// Most characters of the failed try's patch its feedback shows.
const PATCH_CHARS: usize = 3_000;

/// Most bytes of `git diff --raw` output read for a restore.
const RAW_LIMIT: usize = 4 * 1024 * 1024;

/// What dispatch keeps of each task's tries in this process, by task
/// (`"{plan_id}/{task_id}"`).
#[derive(Debug, Default)]
pub(super) struct BestTries {
    tasks: parking_lot::Mutex<HashMap<String, TaskTries>>,
}

#[derive(Debug, Default)]
struct TaskTries {
    /// The try that got furthest ([`BestTry::behind`]), the latest of a tie.
    best: Option<BestTry>,
    /// The best try, whose files the next try starts from, once a try got
    /// less far than it.
    restore: Option<BestTry>,
}

impl BestTries {
    /// Forget `task_id`'s tries, once it passes.
    pub(super) fn forget(&self, plan_id: &str, task_id: &str) {
        self.tasks.lock().remove(&format!("{plan_id}/{task_id}"));
    }
}

impl GraphTaskDispatcher {
    /// Keep the failed try `attempt` of `task`, which ran `steps` in its own
    /// checkout `workdir` with `step_outcomes`, and got as far as `progress`
    /// in the first step it failed. When it got less far than the task's best
    /// try in that checkout ([`BestTry::behind`]), the next try starts from
    /// the best try's files ([`Self::restore_best_try`]), and this returns
    /// the note its feedback carries, with this try's patch against them.
    #[allow(clippy::too_many_arguments)]
    pub(super) async fn settle_best_try(
        &self,
        spec: &TaskExecutionSpec,
        task: &TaskDef,
        attempt: u32,
        workdir: &Path,
        steps: &[(String, crate::task_parser::VerifyStep)],
        step_outcomes: &[(String, bool)],
        progress: Option<StepProgress>,
    ) -> Option<String> {
        // Only a task's own checkout is kept, and ever restored.
        self.workspace_provider.as_ref()?;
        let tree = snapshot_tree(workdir).await?;
        let passed: Vec<String> = steps
            .iter()
            .zip(step_outcomes)
            .filter(|(_, (_, passed))| *passed)
            .map(|((label, _), _)| label.clone())
            .collect();
        let this = BestTry {
            attempt,
            tree,
            workdir: workdir.to_path_buf(),
            passed,
            progress,
        };
        let better = {
            let mut tasks = self.best_tries.tasks.lock();
            let tries = tasks
                .entry(format!("{}/{}", spec.plan_id, task.id))
                .or_default();
            // A try in a fresh checkout starts the task's record again.
            let better = tries
                .best
                .as_ref()
                .filter(|best| best.workdir == this.workdir && this.behind(best))
                .cloned();
            if better.is_none() {
                tries.best = Some(this.clone());
            }
            tries.restore.clone_from(&better);
            better
        };
        let Some(best) = better else {
            // Kept beside the checkpoint too, for a resumed run (gap-586507).
            self.gate_retry_context
                .record_best_try(&spec.plan_id, &task.id, this);
            return None;
        };
        let patch = tree_patch(workdir, &best.tree, &this.tree).await;
        tracing::info!(
            plan_id = %spec.plan_id,
            task_id = %task.id,
            attempt,
            best = best.attempt,
            passed = this.passed.len(),
            best_passed = best.passed.len(),
            progress = ?this.progress,
            best_progress = ?best.progress,
            "the try got less far than the task's best: the next try starts from the best \
             try's files"
        );
        Some(progress_note(&best, &this, steps.len(), patch.as_deref()))
    }

    /// Before a try of `task` in `workdir`, put back the task's best try's
    /// files when its last try got less far ([`Self::settle_best_try`]). On
    /// the task's first try in a resumed run's process, whose kept checkout
    /// holds the last try's tree, start from the best try's files when it got
    /// anywhere ([`Self::resumed_start`], gap-586507).
    pub(super) async fn restore_best_try(
        &self,
        spec: &TaskExecutionSpec,
        task: &TaskDef,
        workdir: &Path,
    ) {
        let (pending, first_here) = {
            let mut tasks = self.best_tries.tasks.lock();
            let key = format!("{}/{}", spec.plan_id, task.id);
            let first_here = !tasks.contains_key(&key);
            let tries = tasks.entry(key).or_default();
            (tries.restore.take(), first_here)
        };
        let start = match pending {
            Some(best) if best.workdir == workdir => {
                Some((best.tree, format!("try {}'s files", best.attempt)))
            }
            Some(_) => None,
            None if first_here => self.resumed_start(spec, task, workdir).await,
            None => None,
        };
        let Some((tree, from)) = start else {
            return;
        };
        let restored = match snapshot_tree(workdir).await {
            Some(now) => restore_tree(workdir, &now, &tree).await,
            None => None,
        };
        match restored {
            Some(paths) => {
                tracing::info!(
                    plan_id = %spec.plan_id,
                    task_id = %task.id,
                    from = %from,
                    files = paths.len(),
                    "the task's next try starts from its best try's files, or clean"
                );
                if let Some(tui) = &self.tui_bridge {
                    tui.best_try_restored(&spec.plan_id, &task.id, &from, &paths);
                }
            }
            None => tracing::warn!(
                plan_id = %spec.plan_id,
                task_id = %task.id,
                from = %from,
                "could not put the task's files back; the next try continues in place"
            ),
        }
    }

    /// Where the first try in this process of `task` starts, when earlier
    /// processes of the run left it failed tries (gap-586507). Its kept
    /// checkout `workdir` holds the last try's tree, so it starts from its
    /// best try's tree when that try got anywhere in this checkout
    /// ([`BestTry::scored`]). The tree and what it is; `None` to continue in
    /// place, from the last try's tree, as a retry in one process does, when
    /// no try got anywhere (gap-54089d), and for a task no earlier process
    /// tried.
    async fn resumed_start(
        &self,
        spec: &TaskExecutionSpec,
        task: &TaskDef,
        workdir: &Path,
    ) -> Option<(String, String)> {
        let book = &self.gate_retry_context;
        if book.earlier_attempts(&spec.plan_id, &task.id) == 0 {
            return None;
        }
        let best = book
            .best_try(&spec.plan_id, &task.id)
            .filter(|best| best.scored() && best.workdir == workdir)?;
        let object = format!("{}^{{tree}}", best.tree);
        let args = ["rev-parse", "--verify", "--quiet", object.as_str()];
        git_line(workdir, &args, None).await?;
        let start = (best.tree.clone(), format!("try {}'s files", best.attempt));
        // This process knows it as the task's best try from here on.
        let key = format!("{}/{}", spec.plan_id, task.id);
        if let Some(tries) = self.best_tries.tasks.lock().get_mut(&key) {
            tries.best = Some(best);
        }
        Some(start)
    }
}

impl BestTry {
    /// Whether this try got less far than `best` ([`Reach::behind`]).
    fn behind(&self, best: &Self) -> bool {
        self.reach().behind(&best.reach())
    }

    /// How far this try got.
    fn reach(&self) -> Reach {
        Reach {
            passed: self.passed.len(),
            progress: self.progress.clone(),
        }
    }

    /// Whether this try got anywhere: it passed a verify step, or the runner
    /// of the step it failed counted a test case or assertion as passed.
    fn scored(&self) -> bool {
        !self.passed.is_empty()
            || self
                .progress
                .as_ref()
                .is_some_and(|progress| progress.cases > 0 || progress.assertions > 0)
    }
}

/// How far a tree got by a task's checks: the steps it passed, and its
/// progress in the first one it failed (gap-54089d, gap-56f761).
#[derive(Debug, Clone)]
struct Reach {
    passed: usize,
    progress: Option<StepProgress>,
}

impl Reach {
    /// Whether this tree got less far than `best` (gap-54089d): it passed
    /// fewer steps, or as many and, in the same first failing step, fewer
    /// assertions, or as many and fewer test cases. Any other two tie, and a
    /// tie goes to the latest.
    fn behind(&self, best: &Self) -> bool {
        match self.passed.cmp(&best.passed) {
            Ordering::Less => true,
            Ordering::Greater => false,
            Ordering::Equal => match (&self.progress, &best.progress) {
                (Some(this), Some(best)) if this.step == best.step => {
                    (this.assertions, this.cases) < (best.assertions, best.cases)
                }
                _ => false,
            },
        }
    }

    /// How far the tree of `run`, a run of a try's own checks, got.
    fn of_run(run: &CheckRun) -> Self {
        Self {
            passed: run.passed.len(),
            progress: run
                .failed
                .as_ref()
                .map(|failed| StepProgress::of(&failed.label, &failed.output)),
        }
    }

    /// How far the tree a try of `task` ended with got by the task's own
    /// checks, its authored verify steps, as the gate ran them (`steps`,
    /// `step_outcomes`, `ran_steps`): those that passed in order, and the
    /// first that failed. `None` for a task with no checks.
    fn of_gate(
        task: &TaskDef,
        steps: &[(String, crate::task_parser::VerifyStep)],
        step_outcomes: &[(String, bool)],
        ran_steps: &[(String, roko_core::Verdict)],
    ) -> Option<Self> {
        let checks = task.verify.len();
        if checks == 0 {
            return None;
        }
        let mut passed = 0;
        for (index, (_, ok)) in step_outcomes.iter().enumerate().take(checks) {
            if *ok {
                passed += 1;
                continue;
            }
            let output = ran_steps
                .get(index)
                .and_then(|(_, verdict)| verdict.detail.as_deref())
                .unwrap_or_default();
            return Some(Self {
                passed,
                progress: Some(StepProgress::of(&steps.get(index)?.0, output)),
            });
        }
        Some(Self {
            passed,
            progress: None,
        })
    }

    /// This reach in words, of `total` checks: "0 of 4 checks passed, then 1
    /// test case and 140 assertions in `verify[0:test]`".
    fn words(&self, total: usize) -> String {
        match &self.progress {
            None => format!("all {total} checks passed"),
            Some(progress) if progress.cases > 0 || progress.assertions > 0 => format!(
                "{} of {total} checks passed, then {} in `{}`",
                self.passed,
                reached(progress),
                progress.step
            ),
            Some(progress) => format!(
                "{} of {total} checks passed, then nothing counted in `{}`",
                self.passed, progress.step
            ),
        }
    }
}

impl GraphTaskDispatcher {
    /// Put back the best tree the try's own checks saw (gap-56f761) when the
    /// tree the try ended with ranks behind it ([`Reach::behind`]): the gate
    /// judged `task`'s `steps` on the final tree as `step_outcomes` and
    /// `ran_steps`, and the attempt `attempt_key`'s checks recorded their
    /// runs beside its checks file, the agent's `run_tests` and the hand-off
    /// check alike. Ties go to the latest run. Only a task's own checkout
    /// `workdir` is ever restored. Returns the note the next try's feedback
    /// carries; `None` when nothing was put back.
    #[allow(clippy::too_many_arguments)]
    pub(super) async fn restore_checked_best(
        &self,
        spec: &TaskExecutionSpec,
        task: &TaskDef,
        attempt_key: &str,
        workdir: &Path,
        steps: &[(String, crate::task_parser::VerifyStep)],
        step_outcomes: &[(String, bool)],
        ran_steps: &[(String, roko_core::Verdict)],
    ) -> Option<String> {
        self.workspace_provider.as_ref()?;
        let ended = Reach::of_gate(task, steps, step_outcomes, ran_steps)?;
        let runs = read_check_runs(&task_check_runs_path(&self.workdir, attempt_key));
        let (run, best) = runs
            .iter()
            .map(|run| (run, Reach::of_run(run)))
            .reduce(|best, run| if run.1.behind(&best.1) { best } else { run })?;
        if !ended.behind(&best) {
            return None;
        }
        let now = snapshot_tree(workdir).await?;
        if now == run.tree {
            return None;
        }
        let paths = restore_tree(workdir, &now, &run.tree).await?;
        // Trees that differ only in roko's own state hold the same work.
        if paths.is_empty() {
            return None;
        }
        let at = local_time(run.at_ms);
        tracing::info!(
            plan_id = %spec.plan_id,
            task_id = %task.id,
            at = %at,
            files = paths.len(),
            "the try ended on a tree worse than one its own checks saw: roko put that tree \
             back, and the checks judge it"
        );
        if let Some(tui) = &self.tui_bridge {
            tui.checked_tree_restored(&spec.plan_id, &task.id, &at, &paths);
        }
        let total = task.verify.len();
        Some(format!(
            "Kept by roko: this try's own checks saw a better tree at {at} than the one it ended \
             with: {}, against {}. Its later edits made the tree worse, so roko put the {at} tree \
             back, and the checks judged that tree. Start from it.",
            best.words(total),
            ended.words(total),
        ))
    }
}

/// `at_ms`, Unix milliseconds, as local time of day: `05:32:54`.
fn local_time(at_ms: i64) -> String {
    chrono::Local
        .timestamp_millis_opt(at_ms)
        .single()
        .map_or_else(|| at_ms.to_string(), |at| at.format("%H:%M:%S").to_string())
}

impl StepProgress {
    /// How far a try got in `step`, the first verify step it failed, whose
    /// runner printed `output`: the test cases and assertions its summary
    /// lines count as passed (gap-54089d). It reads doctest's and Catch2's
    /// `test cases: N | P passed | …` and `assertions: N | P passed | …`,
    /// ctest's `X% tests passed, F tests failed out of N`, and the cargo,
    /// pytest, Jest and go test summaries roko's test gate reads
    /// ([`roko_gate::parse_test_counts`]), each summed over the runs the step
    /// makes. What no line counts is 0.
    pub(super) fn of(step: &str, output: &str) -> Self {
        let mut cases = 0;
        let mut assertions = 0;
        let mut counted = false;
        for line in output.lines() {
            if let Some(passed) = bar_count(line, "test cases:") {
                cases += passed;
            } else if let Some(passed) = bar_count(line, "assertions:") {
                assertions += passed;
            } else if let Some(passed) = ctest_passed(line) {
                cases += passed;
            } else {
                continue;
            }
            counted = true;
        }
        if !counted {
            cases = [BuildSystem::Python, BuildSystem::Npm, BuildSystem::Go]
                .into_iter()
                .find_map(|build| roko_gate::parse_test_counts(output, build))
                .map_or(0, |count| u64::from(count.passed));
        }
        Self {
            step: step.to_string(),
            cases,
            assertions,
        }
    }
}

/// `P` of a doctest or Catch2 summary line `<label> N | P passed | …`.
fn bar_count(line: &str, label: &str) -> Option<u64> {
    let (_, counts) = line.split_once(label)?;
    let mut fields = counts.split('|').map(str::trim);
    fields.next()?.parse::<u64>().ok()?;
    fields.next()?.strip_suffix("passed")?.trim().parse().ok()
}

/// The tests passed of ctest's summary line `80% tests passed, 2 tests
/// failed out of 10`.
fn ctest_passed(line: &str) -> Option<u64> {
    let (_, counts) = line.split_once("% tests passed, ")?;
    let (failed, total) = counts.split_once(" failed out of ")?;
    let failed: u64 = failed.split_whitespace().next()?.parse().ok()?;
    let total: u64 = total.split_whitespace().next()?.parse().ok()?;
    Some(total.saturating_sub(failed))
}

/// What `progress` counted as passed, in words: `1 test case and 131
/// assertions`, or `12 test cases`.
fn reached(progress: &StepProgress) -> String {
    let count = |n: u64, noun: &str| {
        if n == 1 {
            format!("1 {noun}")
        } else {
            format!("{n} {noun}s")
        }
    };
    if progress.assertions > 0 {
        format!(
            "{} and {}",
            count(progress.cases, "test case"),
            count(progress.assertions, "assertion")
        )
    } else {
        count(progress.cases, "test case")
    }
}

/// The feedback note of a try that got less far than `best`, of the
/// `total` verify steps: how far each got, that `best`'s files come back,
/// and `patch`, what the failed try changed from them.
fn progress_note(best: &BestTry, this: &BestTry, total: usize, patch: Option<&str>) -> String {
    let within_step = best
        .progress
        .as_ref()
        .zip(this.progress.as_ref())
        .filter(|_| this.passed.len() == best.passed.len());
    let mut note = if let Some((got, this_got)) = within_step {
        format!(
            "Kept by roko: in `{}`, try {} passed {}, and this try {}. Before your next try, \
             roko puts try {}'s files back: start from its code, and fix what still fails.",
            got.step,
            best.attempt,
            reached(got),
            reached(this_got),
            best.attempt,
        )
    } else {
        let passed = best
            .passed
            .iter()
            .map(|label| format!("`{label}`"))
            .collect::<Vec<_>>()
            .join(", ");
        format!(
            "Kept by roko: try {} passed {} of {total} verify steps ({passed}), and this try \
             passed {}. Before your next try, roko puts try {}'s files back: start from its \
             code, keep those steps passing, and fix what still fails.",
            best.attempt,
            best.passed.len(),
            this.passed.len(),
            best.attempt,
        )
    };
    if let Some(patch) = patch.filter(|patch| !patch.trim().is_empty()) {
        let shown: String = patch.chars().take(PATCH_CHARS).collect();
        let cut = if shown.len() < patch.len() {
            "\n… (cut)"
        } else {
            ""
        };
        note.push_str(&format!(
            "\nWhat this try changed from them, which the restore takes back:\n```diff\n{shown}{cut}\n```"
        ));
    }
    note
}

/// `git diff` from tree `from` to tree `to` in `workdir`'s repository.
async fn tree_patch(workdir: &Path, from: &str, to: &str) -> Option<String> {
    let (bytes, _) = git_bytes(
        workdir,
        &["diff", "--no-color", "--no-ext-diff", from, to],
        None,
        PATCH_CHARS * 4,
    )
    .await?;
    Some(String::from_utf8_lossy(&bytes).into_owned())
}

/// Make `workdir`'s files, whose tree is `now`, those of tree `to`: each path
/// the two differ in is written from `to` through a scratch index, so the
/// checkout's own index is left alone, or removed when `to` lacks it.
/// Ignored files, such as build trees, are never touched. The paths it
/// changed, or `None` when git failed.
async fn restore_tree(workdir: &Path, now: &str, to: &str) -> Option<Vec<String>> {
    let (raw, truncated) = git_bytes(
        workdir,
        &[
            "diff",
            "--raw",
            "-z",
            "--no-renames",
            "--no-abbrev",
            now,
            to,
        ],
        None,
        RAW_LIMIT,
    )
    .await?;
    if truncated {
        return None;
    }
    // roko's own state in the checkout is never the try's work.
    let changes: Vec<_> = parse_raw_diff(&raw)
        .into_iter()
        .filter(|change| !is_roko_state(&change.path))
        .collect();
    let scratch = tempfile::tempdir().ok()?;
    let index = scratch.path().join("index");
    git_bytes(workdir, &["read-tree", to], Some(&index), 1024).await?;
    let mut written = Vec::new();
    for change in &changes {
        if change.new_blob.is_some() {
            written.push(change.path.as_str());
        } else if let Err(error) = tokio::fs::remove_file(workdir.join(&change.path)).await {
            tracing::debug!(path = %change.path, %error, "could not remove a file the best try lacks");
        }
    }
    if !written.is_empty() {
        let mut args = vec!["checkout-index", "-f", "--"];
        args.extend(written);
        git_bytes(workdir, &args, Some(&index), 64 * 1024).await?;
    }
    Some(changes.into_iter().map(|change| change.path).collect())
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use tempfile::tempdir;

    use super::*;
    use crate::graph_execution::WorktreeExecutionWorkspaceProvider;
    use crate::graph_task_dispatch::diff_snapshot::tests::git;
    use crate::graph_task_dispatch::tests::{
        make_spec, make_test_dispatcher_with, no_auto_fix, tool_call_turn, verify_step,
    };
    use crate::orchestrator::worktree::{WorktreeConfig, WorktreeManager};

    const RUN: &str = "graph-best-try-run";

    /// Marks the task's own prompts, so helper calls do not count as tries.
    const MARKER: &str = "BEST-TRY-TASK";

    /// A fake Claude CLI for the task's tries, by their ordinal: try 1 makes
    /// verify[0] and verify[1] pass; try 2 breaks verify[0] and leaves
    /// verify[2] failing; try 3 notes what its checkout holds and whether its
    /// prompt says roko put try 1's files back, then makes every step pass.
    const SCRIPT: &str = r#"#!/bin/sh
set -eu
prompt=$(cat)
case "$prompt $*" in
  *BEST-TRY-TASK*) ;;
  *)
    printf '%s\n' '{"type":"content_block_delta","delta":{"text":"a helper answer"}}'
    printf '%s\n' '{"type":"result","session_id":"sess-h","model":"claude-sonnet-4-6","total_cost_usd":0.0,"usage":{"input_tokens":5,"output_tokens":10}}'
    exit 0 ;;
esac
dir=$(dirname -- "$0")
n=$(( $(cat "$dir/tries" 2>/dev/null || echo 0) + 1 ))
echo "$n" > "$dir/tries"
case "$n" in
  1) printf 'A\n' > a.txt; printf 'B\n' > b.txt ;;
  2) printf 'broken\n' > a.txt; printf 'half\n' > c.txt ;;
  *)
    cat a.txt > "$dir/seen-a"
    if [ -f c.txt ]; then echo present > "$dir/seen-c"; else echo absent > "$dir/seen-c"; fi
    case "$prompt $*" in *"Kept by roko: try 1"*) echo told > "$dir/told" ;; esac
    printf 'C\n' > c.txt ;;
esac
printf '%s\n' '{"type":"content_block_delta","delta":{"text":"edited"}}'
printf '%s\n' '{"type":"result","session_id":"sess-b","model":"claude-sonnet-4-6","total_cost_usd":0.0,"usage":{"input_tokens":5,"output_tokens":10}}'
"#;

    /// A fake Claude CLI whose first try passes no step (a.txt holds X), and
    /// whose second notes what its checkout holds, then makes every step pass.
    const NOTHING_SCRIPT: &str = r#"#!/bin/sh
set -eu
prompt=$(cat)
case "$prompt $*" in
  *BEST-TRY-TASK*) ;;
  *)
    printf '%s\n' '{"type":"content_block_delta","delta":{"text":"a helper answer"}}'
    printf '%s\n' '{"type":"result","session_id":"sess-h","model":"claude-sonnet-4-6","total_cost_usd":0.0,"usage":{"input_tokens":5,"output_tokens":10}}'
    exit 0 ;;
esac
dir=$(dirname -- "$0")
n=$(( $(cat "$dir/tries" 2>/dev/null || echo 0) + 1 ))
echo "$n" > "$dir/tries"
case "$n" in
  1) printf 'X\n' > a.txt ;;
  *)
    cat a.txt > "$dir/seen-a" 2>/dev/null || echo missing > "$dir/seen-a"
    case "$prompt $*" in *"Kept by roko"*) echo told > "$dir/told" ;; esac
    printf 'A\n' > a.txt; printf 'B\n' > b.txt; printf 'C\n' > c.txt ;;
esac
printf '%s\n' '{"type":"content_block_delta","delta":{"text":"edited"}}'
printf '%s\n' '{"type":"result","session_id":"sess-b","model":"claude-sonnet-4-6","total_cost_usd":0.0,"usage":{"input_tokens":5,"output_tokens":10}}'
"#;

    /// A fake Claude CLI for a task with one verify step that no try passes
    /// ([`DOCTEST_STEP`]): try 1 gets 131 of its 132 assertions passing, try 2
    /// regresses to 77 of 149 and leaves c.txt behind, and try 3 notes what
    /// its checkout holds and whether its prompt says roko put try 1's files
    /// back, then makes the step pass.
    const SCORE_SCRIPT: &str = r#"#!/bin/sh
set -eu
prompt=$(cat)
case "$prompt $*" in
  *BEST-TRY-TASK*) ;;
  *)
    printf '%s\n' '{"type":"content_block_delta","delta":{"text":"a helper answer"}}'
    printf '%s\n' '{"type":"result","session_id":"sess-h","model":"claude-sonnet-4-6","total_cost_usd":0.0,"usage":{"input_tokens":5,"output_tokens":10}}'
    exit 0 ;;
esac
dir=$(dirname -- "$0")
n=$(( $(cat "$dir/tries" 2>/dev/null || echo 0) + 1 ))
echo "$n" > "$dir/tries"
case "$n" in
  1) printf 'good\n' > a.txt ;;
  2) printf 'bad\n' > a.txt; printf 'half\n' > c.txt ;;
  *)
    cat a.txt > "$dir/seen-a"
    if [ -f c.txt ]; then echo present > "$dir/seen-c"; else echo absent > "$dir/seen-c"; fi
    case "$prompt $*" in *"Kept by roko"*"try 1"*) echo told > "$dir/told" ;; esac
    printf 'done\n' > a.txt ;;
esac
printf '%s\n' '{"type":"content_block_delta","delta":{"text":"edited"}}'
printf '%s\n' '{"type":"result","session_id":"sess-b","model":"claude-sonnet-4-6","total_cost_usd":0.0,"usage":{"input_tokens":5,"output_tokens":10}}'
"#;

    /// One verify step, two doctest cases in it, that passes once a.txt holds
    /// `done`: with `good` doctest counts 1 of 2 cases and 131 of 132
    /// assertions passing, with `bad` 0 of 2 and 77 of 149.
    const DOCTEST_STEP: &str = "case \"$(cat a.txt 2>/dev/null)\" in \
        good) printf '[doctest] test cases: 2 | 1 passed | 1 failed | 0 skipped\\n[doctest] \
        assertions: 132 | 131 passed | 1 failed |\\n'; exit 1 ;; \
        bad) printf '[doctest] test cases: 2 | 0 passed | 2 failed | 0 skipped\\n[doctest] \
        assertions: 149 | 77 passed | 72 failed |\\n'; exit 1 ;; \
        done) exit 0 ;; *) exit 1 ;; esac";

    /// A repository whose `main` has one commit, and the root of its attempt
    /// worktrees, outside it.
    fn repo_with_worktrees() -> (tempfile::TempDir, tempfile::TempDir) {
        let repo = tempdir().expect("repo");
        git(repo.path(), &["init", "--quiet", "-b", "main"]);
        std::fs::write(repo.path().join("README.md"), "base\n").expect("write README");
        git(repo.path(), &["add", "README.md"]);
        git(repo.path(), &["commit", "--quiet", "-m", "base"]);
        (repo, tempdir().expect("worktrees"))
    }

    /// The verdict rows of the run's attempt log under `runs_dir`.
    fn verdicts(runs_dir: &Path) -> Vec<serde_json::Value> {
        std::fs::read_to_string(runs_dir.join(RUN).join("attempts.jsonl"))
            .expect("the run's attempt log")
            .lines()
            .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
            .filter(|line| line["schema_version"] == "roko.verdict/1")
            .collect()
    }

    /// A dispatcher whose task runs in its own worktree under `worktrees` of
    /// the repository `repo`, with `script` as its agent, and the task's
    /// spec: three verify steps, a.txt, b.txt and c.txt hold A, B and C. With
    /// `resumable`, the plan runs as Graph checkpoint run [`RUN`], with its
    /// retry feedback beside it in `repo`: a second such dispatcher stands in
    /// for a resumed process of the run, and re-attaches the checkouts the
    /// first one kept.
    async fn dispatcher_in(
        repo: &tempfile::TempDir,
        worktrees: &Path,
        script: &str,
        resumable: bool,
    ) -> (Arc<GraphTaskDispatcher>, TaskExecutionSpec) {
        let verify = vec![
            verify_step("test", "grep -qx A a.txt"),
            verify_step("test", "grep -qx B b.txt"),
            verify_step("test", "grep -qx C c.txt"),
        ];
        dispatcher_with(repo, worktrees, script, resumable, verify).await
    }

    /// [`dispatcher_in`], with `verify` as the task's verify steps.
    async fn dispatcher_with(
        repo: &tempfile::TempDir,
        worktrees: &Path,
        script: &str,
        resumable: bool,
        verify: Vec<crate::task_parser::VerifyStep>,
    ) -> (Arc<GraphTaskDispatcher>, TaskExecutionSpec) {
        dispatcher_configured(repo, worktrees, script, resumable, verify, no_auto_fix).await
    }

    /// [`dispatcher_with`], with `configure` run on its config.
    async fn dispatcher_configured(
        repo: &tempfile::TempDir,
        worktrees: &Path,
        script: &str,
        resumable: bool,
        verify: Vec<crate::task_parser::VerifyStep>,
        configure: impl FnOnce(&mut RokoConfig),
    ) -> (Arc<GraphTaskDispatcher>, TaskExecutionSpec) {
        let provider = Arc::new(WorktreeExecutionWorkspaceProvider::new(
            WorktreeManager::new(WorktreeConfig {
                repo_root: repo.path().to_path_buf(),
                base_branch: "HEAD".to_string(),
                worktrees_root: worktrees.to_path_buf(),
                max_live: None,
                idle_ttl: std::time::Duration::from_secs(3600),
            }),
        ));
        let workspaces = Arc::clone(&provider);
        let (dispatcher, mut task) = make_test_dispatcher_with(
            repo,
            script,
            configure,
            GraphFeedbackContext {
                runs_dir: Some(repo.path().join(".roko/runs")),
                ..GraphFeedbackContext::default()
            },
            |dispatcher| dispatcher.with_workspace_provider(workspaces),
        )
        .await;
        task.description = Some(format!(
            "{MARKER}: make a.txt, b.txt and c.txt hold A, B and C."
        ));
        task.verify = verify;
        let mut spec = make_spec(&task);
        spec.max_retries = 2;
        if resumable {
            provider
                .manager()
                .begin_plan_run(&spec.plan_id, RUN)
                .await
                .expect("the plan run begins");
            dispatcher.attach_retry_feedback(
                &spec.plan_id,
                repo.path().join(".roko/retry-feedback.json"),
                RUN,
            );
        }
        (dispatcher, spec)
    }

    /// [`dispatcher_in`] a fresh repository, not resumable. Returns the
    /// repository (where `script` leaves its notes), its worktrees, its runs
    /// directory, the dispatcher and the task's spec.
    async fn fixture(
        script: &str,
    ) -> (
        tempfile::TempDir,
        tempfile::TempDir,
        std::path::PathBuf,
        Arc<GraphTaskDispatcher>,
        TaskExecutionSpec,
    ) {
        let (repo, worktrees) = repo_with_worktrees();
        let (dispatcher, spec) = dispatcher_in(&repo, worktrees.path(), script, false).await;
        let runs_dir = repo.path().join(".roko/runs");
        (repo, worktrees, runs_dir, dispatcher, spec)
    }

    /// What `script` noted in `repo` as `name`, trimmed; empty when it noted
    /// nothing.
    fn note(repo: &tempfile::TempDir, name: &str) -> String {
        std::fs::read_to_string(repo.path().join(name))
            .unwrap_or_default()
            .trim()
            .to_string()
    }

    /// gap-54089d: a failing step's runner says how far it got, in each
    /// runner's own words, summed over the runs the step makes.
    #[test]
    fn a_failing_steps_runner_says_how_far_it_got() {
        let counts = |output: &str| {
            let progress = StepProgress::of("verify[0:test]", output);
            (progress.cases, progress.assertions)
        };
        let doctest = "[doctest] test cases:   2 |   1 passed |   1 failed |   0 skipped\n\
                       [doctest] assertions: 132 | 131 passed |   1 failed |\n\
                       [doctest] Status: FAILURE!";
        assert_eq!(counts(doctest), (1, 131));
        let catch2 = "test cases:  3 |  2 passed | 1 failed\nassertions: 40 | 38 passed | 2 failed";
        assert_eq!(counts(catch2), (2, 38));
        let cargo = "test result: ok. 5 passed; 0 failed; 0 ignored\n\
                     test result: FAILED. 7 passed; 1 failed; 0 ignored";
        assert_eq!(counts(cargo), (12, 0));
        assert_eq!(counts("===== 1 failed, 12 passed in 0.52s ====="), (12, 0));
        assert_eq!(counts("80% tests passed, 2 tests failed out of 10"), (8, 0));
        assert_eq!(counts("error: could not compile `undertow`"), (0, 0));
    }

    /// gap-54089d: tries rank by steps passed, then, in the same first
    /// failing step, by assertions and then test cases passed; anything else
    /// ties, and a tie goes to the latest.
    #[test]
    fn a_try_is_behind_one_that_got_further() {
        let try_at = |passed: &[&str], step: &str, cases: u64, assertions: u64| BestTry {
            attempt: 1,
            tree: "tree".to_string(),
            workdir: std::path::PathBuf::from("/checkout"),
            passed: passed.iter().map(ToString::to_string).collect(),
            progress: Some(StepProgress {
                step: step.to_string(),
                cases,
                assertions,
            }),
        };
        let near = try_at(&[], "verify[0:test]", 1, 131);
        let regressed = try_at(&[], "verify[0:test]", 0, 77);
        assert!(regressed.behind(&near));
        assert!(!near.behind(&regressed));
        assert!(!near.behind(&near.clone()), "a tie goes to the latest");
        let one_step = try_at(&["verify[0:test]"], "verify[1:test]", 0, 0);
        assert!(near.behind(&one_step), "steps come first");
        let more_cases = try_at(&[], "verify[0:test]", 2, 40);
        assert!(more_cases.behind(&near), "assertions come before cases");
        assert!(
            try_at(&[], "verify[0:test]", 0, 131).behind(&near),
            "then cases"
        );
        let elsewhere = try_at(&[], "verify[1:test]", 0, 0);
        assert!(!elsewhere.behind(&near), "another failing step ties");
        assert!(near.scored() && one_step.scored());
        assert!(!try_at(&[], "verify[0:test]", 0, 0).scored());
    }

    /// gap-fefdf5, the shape of Undertow phase 2's T3: try 1 passes 2 of 3
    /// steps; try 2 breaks a step try 1 passed, and its later steps still run,
    /// so its trade shows; roko puts try 1's files back before try 3, whose
    /// prompt says so, and try 3 finishes from them.
    #[tokio::test]
    async fn a_retry_after_a_regression_starts_from_the_best_try() {
        let (repo, _worktrees, runs_dir, dispatcher, spec) = fixture(SCRIPT).await;
        let ctx = CellContext::new().with_run_id(RUN.to_string());

        dispatcher
            .dispatch(&spec, Vec::new(), &ctx)
            .await
            .expect_err("try 1 fails verify[2]");
        dispatcher
            .dispatch(&spec, Vec::new(), &ctx)
            .await
            .expect_err("try 2 breaks verify[0]");
        dispatcher
            .dispatch(&spec, Vec::new(), &ctx)
            .await
            .expect("try 3 passes from try 1's files");

        assert_eq!(
            note(&repo, "seen-a"),
            "A",
            "try 3 starts from try 1's a.txt"
        );
        assert_eq!(note(&repo, "seen-c"), "absent", "try 2's c.txt is gone");
        assert_eq!(
            note(&repo, "told"),
            "told",
            "try 3's prompt says roko restored try 1"
        );

        dispatcher.close_run_attempts(RUN);
        let rows = verdicts(&runs_dir);
        let ran = |row: &serde_json::Value| -> Vec<Option<bool>> {
            row["steps"]
                .as_array()
                .expect("steps")
                .iter()
                .map(|step| step["passed"].as_bool())
                .collect()
        };
        assert_eq!(rows.len(), 3, "{rows:?}");
        assert_eq!(ran(&rows[0]), [Some(true), Some(true), Some(false)]);
        // After the regression its other steps ran: the trade shows.
        assert_eq!(ran(&rows[1]), [Some(false), Some(true), Some(false)]);
        assert_eq!(ran(&rows[2]), [Some(true), Some(true), Some(true)]);
    }

    /// gap-fefdf5: a try that passed no step, with no better try before it,
    /// leaves its work for the next try, which continues in place as before,
    /// with no restore.
    #[tokio::test]
    async fn a_try_that_passed_nothing_continues_in_place() {
        let (repo, _worktrees, _runs_dir, dispatcher, spec) = fixture(NOTHING_SCRIPT).await;
        let ctx = CellContext::new().with_run_id(RUN.to_string());
        dispatcher
            .dispatch(&spec, Vec::new(), &ctx)
            .await
            .expect_err("try 1 passes no step");
        dispatcher
            .dispatch(&spec, Vec::new(), &ctx)
            .await
            .expect("try 2 passes");
        assert_eq!(note(&repo, "seen-a"), "X", "try 2 starts from try 1's work");
        assert_eq!(note(&repo, "told"), "", "nothing was restored");
    }

    /// gap-586507: a resumed run re-attaches the checkout its earlier process
    /// kept, which holds the last try's broken tree. The task's first try in
    /// the new process starts from its best try's files instead.
    #[tokio::test]
    async fn a_resumed_task_starts_from_its_best_try() {
        let (repo, worktrees) = repo_with_worktrees();
        let ctx = CellContext::new().with_run_id(RUN.to_string());
        {
            let (dispatcher, spec) = dispatcher_in(&repo, worktrees.path(), SCRIPT, true).await;
            dispatcher
                .dispatch(&spec, Vec::new(), &ctx)
                .await
                .expect_err("try 1 fails verify[2]");
            dispatcher
                .dispatch(&spec, Vec::new(), &ctx)
                .await
                .expect_err("try 2 breaks verify[0]");
        }
        // A new process of the same run.
        let (dispatcher, spec) = dispatcher_in(&repo, worktrees.path(), SCRIPT, true).await;
        dispatcher
            .dispatch(&spec, Vec::new(), &ctx)
            .await
            .expect("the resumed try passes from try 1's files");
        assert_eq!(
            note(&repo, "seen-a"),
            "A",
            "the resumed try starts from try 1's a.txt"
        );
        assert_eq!(note(&repo, "seen-c"), "absent", "try 2's c.txt is gone");
    }

    /// gap-54089d: when no try of a resumed task got anywhere, no step passed
    /// and no runner counted a pass, its first try in the new process
    /// continues in place, from the last try's tree, as a retry in one process
    /// does. Starting clean threw away phase 3's near-done sequencer.
    #[tokio::test]
    async fn a_resumed_task_that_got_nowhere_continues_in_place() {
        let (repo, worktrees) = repo_with_worktrees();
        let ctx = CellContext::new().with_run_id(RUN.to_string());
        {
            let (dispatcher, spec) =
                dispatcher_in(&repo, worktrees.path(), NOTHING_SCRIPT, true).await;
            dispatcher
                .dispatch(&spec, Vec::new(), &ctx)
                .await
                .expect_err("try 1 passes no step");
        }
        let (dispatcher, spec) = dispatcher_in(&repo, worktrees.path(), NOTHING_SCRIPT, true).await;
        dispatcher
            .dispatch(&spec, Vec::new(), &ctx)
            .await
            .expect("the resumed try passes");
        assert_eq!(
            note(&repo, "seen-a"),
            "X",
            "the resumed try continues from try 1's work"
        );
    }

    /// gap-54089d, the shape of Undertow phase 3's T1: no try passes the
    /// task's one verify step, so every try ties at 0 steps. Try 1 gets 131
    /// of the step's 132 assertions passing, and try 2 regresses to 77 of
    /// 149. Ranked by the failing step's own counts, try 1 is the best: roko
    /// puts its files back before try 3, whose prompt says so, and try 3
    /// finishes from them.
    #[tokio::test]
    async fn tries_that_pass_the_same_steps_rank_by_the_failing_steps_passed_cases() {
        let (repo, worktrees) = repo_with_worktrees();
        let verify = vec![verify_step("test", DOCTEST_STEP)];
        let (dispatcher, spec) =
            dispatcher_with(&repo, worktrees.path(), SCORE_SCRIPT, false, verify).await;
        let ctx = CellContext::new().with_run_id(RUN.to_string());

        dispatcher
            .dispatch(&spec, Vec::new(), &ctx)
            .await
            .expect_err("try 1 gets 131 of 132 assertions");
        dispatcher
            .dispatch(&spec, Vec::new(), &ctx)
            .await
            .expect_err("try 2 regresses to 77 of 149");
        dispatcher
            .dispatch(&spec, Vec::new(), &ctx)
            .await
            .expect("try 3 passes from try 1's files");

        assert_eq!(
            note(&repo, "seen-a"),
            "good",
            "try 3 starts from try 1's a.txt"
        );
        assert_eq!(note(&repo, "seen-c"), "absent", "try 2's c.txt is gone");
        assert_eq!(
            note(&repo, "told"),
            "told",
            "try 3's prompt says roko restored try 1"
        );
    }

    /// One verify step in phase 3's shape for a try judged on its checked
    /// tree: with a.txt `good` doctest counts 1 of 2 cases and 140 of 149
    /// assertions passing, and with `broken` the build fails, as a
    /// compiler's error, with nothing counted.
    const CHECKED_STEP: &str = "case \"$(cat a.txt 2>/dev/null)\" in \
        good) printf '[doctest] test cases: 2 | 1 passed | 1 failed | 0 skipped\\n[doctest] \
        assertions: 149 | 140 passed | 9 failed |\\n'; exit 1 ;; \
        broken) echo \"sequencer.cpp:12:5: error: no member named 'load' in \
        'undertow::Pattern'\" >&2; exit 2 ;; \
        *) exit 1 ;; esac";

    /// gap-56f761, the shape of Undertow phase 3's T1#1 (run 7b8cb121): the
    /// try's own checks see 140 of its step's 149 assertions pass, then its
    /// last edit breaks the build, and the gate would judge only that tree.
    /// roko puts back the tree its checks saw, the gate judges that one, and
    /// the next try starts from it, its prompt saying so.
    #[tokio::test]
    async fn a_try_is_judged_on_its_best_checked_tree_when_its_last_edit_is_worse() {
        let (repo, worktrees) = repo_with_worktrees();
        let write = |content: &str| serde_json::json!({ "path": "a.txt", "content": content });
        let read = serde_json::json!({ "path": "a.txt" });
        let (base_url, requests) = crate::graph_task_dispatch::tests::spawn_openai_mock(vec![
            // Try 1: a good tree its checks see, then a last edit that breaks it.
            tool_call_turn("w1", "write_file", write("good\n")),
            tool_call_turn("c1", "run_tests", serde_json::json!({ "build": "checks" })),
            tool_call_turn("r1", "read_file", read.clone()),
            tool_call_turn("w2", "write_file", write("broken\n")),
            crate::graph_task_dispatch::tests::final_turn("done"),
            // Try 2: what its checkout holds.
            tool_call_turn("r2", "read_file", read),
            crate::graph_task_dispatch::tests::final_turn("done"),
        ]);
        let verify = vec![verify_step("test", CHECKED_STEP)];
        let helpers = crate::graph_task_dispatch::tests::VERIFY_PROVIDER;
        let (dispatcher, spec) =
            dispatcher_configured(&repo, worktrees.path(), helpers, false, verify, |config| {
                no_auto_fix(config);
                api_implementer(config, &base_url);
            })
            .await;
        let spec = pinned(spec, "api-model");
        let ctx = CellContext::new().with_run_id(RUN.to_string());

        dispatcher
            .dispatch(&spec, Vec::new(), &ctx)
            .await
            .expect_err("try 1 still fails its step, at 140 of 149");
        dispatcher
            .dispatch(&spec, Vec::new(), &ctx)
            .await
            .expect_err("try 2 fails it the same");

        let requests = requests.lock();
        let said = |index: usize| {
            requests
                .get(index)
                .map(ToString::to_string)
                .unwrap_or_default()
        };
        let try_two = said(5);
        assert!(
            try_two.contains("this try's own checks saw a better tree"),
            "try 2's prompt says what roko kept: {try_two}"
        );
        assert!(try_two.contains("140 assertions"), "{try_two}");
        let read_back = requests
            .get(6)
            .and_then(|request| request["messages"].as_array())
            .and_then(|messages| {
                messages
                    .iter()
                    .find(|message| message["tool_call_id"] == "r2")
                    .map(|message| message["content"].to_string())
            })
            .unwrap_or_default();
        assert!(
            read_back.contains("good") && !read_back.contains("broken"),
            "try 2 starts from the checked tree: {read_back}"
        );
    }

    /// `spec` with its task pinned to `model`, the hint dispatch reads from
    /// the task itself.
    fn pinned(mut spec: TaskExecutionSpec, model: &str) -> TaskExecutionSpec {
        let mut task: TaskDef = serde_json::from_str(&spec.task_def_json).expect("the task");
        task.model_hint = Some(model.to_string());
        spec.task_def_json = serde_json::to_string(&task).expect("the task");
        spec.model_hint = task.model_hint;
        spec
    }

    /// `config` with `api-model`, an OpenAI-compatible model at `base_url`,
    /// for a task that names it as its model, the helper calls still on the
    /// fake CLI, and no hand-off check, so a try ends when its agent hands
    /// back.
    fn api_implementer(config: &mut RokoConfig, base_url: &str) {
        config.providers.insert(
            "mock_api".to_string(),
            roko_core::config::schema::ProviderConfig {
                kind: roko_core::agent::ProviderKind::OpenAiCompat,
                base_url: Some(base_url.to_string()),
                api_key_env: Some("PATH".to_string()),
                timeout_ms: Some(crate::graph_task_dispatch::tests::FIXTURE_PROVIDER_TIMEOUT_MS),
                ..roko_core::config::schema::ProviderConfig::default()
            },
        );
        config.models.insert(
            "api-model".to_string(),
            roko_core::config::schema::ModelProfile {
                provider: "mock_api".to_string(),
                slug: "api-model-1".to_string(),
                context_window: 128_000,
                max_output: Some(1_024),
                max_tools: Some(32),
                supports_tools: true,
                tool_format: "openai_json".to_string(),
                cost_input_per_m: Some(1.0),
                cost_output_per_m: Some(2.0),
                ..roko_core::config::schema::ModelProfile::default()
            },
        );
        // Helper calls go to the fake CLI, which may then be chosen for them.
        config
            .models
            .get_mut("stream-model")
            .expect("the fake CLI's model")
            .supports_tools = true;
        config.routing.fast_task_model = "stream-model".to_string();
        config.gates.hand_off_continues = 0;
        config.conductor.silence_timeout_secs = 0;
        config.conductor.task_stall_secs = 0;
    }

    /// gap-54089d: a resumed task whose tries all failed its one step starts
    /// from the try that got furthest in it, by the step's own counts, not
    /// from the last try's tree its kept checkout holds.
    #[tokio::test]
    async fn a_resumed_task_starts_from_its_furthest_try_in_a_failing_step() {
        let (repo, worktrees) = repo_with_worktrees();
        let ctx = CellContext::new().with_run_id(RUN.to_string());
        let verify = || vec![verify_step("test", DOCTEST_STEP)];
        {
            let (dispatcher, spec) =
                dispatcher_with(&repo, worktrees.path(), SCORE_SCRIPT, true, verify()).await;
            dispatcher
                .dispatch(&spec, Vec::new(), &ctx)
                .await
                .expect_err("try 1 gets 131 of 132 assertions");
            dispatcher
                .dispatch(&spec, Vec::new(), &ctx)
                .await
                .expect_err("try 2 regresses to 77 of 149");
        }
        let (dispatcher, spec) =
            dispatcher_with(&repo, worktrees.path(), SCORE_SCRIPT, true, verify()).await;
        dispatcher
            .dispatch(&spec, Vec::new(), &ctx)
            .await
            .expect("the resumed try passes from try 1's files");
        assert_eq!(
            note(&repo, "seen-a"),
            "good",
            "the resumed try starts from try 1's a.txt"
        );
        assert_eq!(note(&repo, "seen-c"), "absent", "try 2's c.txt is gone");
    }
}
