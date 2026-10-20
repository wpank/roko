//! The best try so far, per task (gap-fefdf5).
//!
//! A task's retries continue in the checkout its last try left. When that try
//! broke a verify step an earlier try had passed, the next try would start
//! from the broken tree, and each try is a new agent session, so the passing
//! code would be lost. So each failed try's tree is kept with the steps it
//! passed. When a try passes fewer steps than the task's best try, the best
//! try's files are put back before the next try, whose feedback says so and
//! shows what the failed try had changed. A try that passes as many steps or
//! more continues in place, as before (ties go to the latest), and so does a
//! try that passed nothing with no better try before it.
//!
//! The best try is kept beside the plan's Graph checkpoint as well
//! ([`retry_feedback::BestTry`]). A resumed run re-attaches the checkout an
//! earlier process kept, which holds the last try's tree, so a task's first
//! try in the new process starts from its best try's tree when that try passed
//! a step, and clean from the checkout's base otherwise (gap-586507).
//!
//! Only a task's own checkout, a per-task worktree, is ever restored: in a
//! shared working tree, sibling tasks' edits would be undone with it.

use std::collections::HashMap;
use std::path::Path;

use super::diff_snapshot::{git_bytes, git_line, is_roko_state, parse_raw_diff, snapshot_tree};
use super::retry_feedback::BestTry;
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
    /// The try that passed the most verify steps, the latest of a tie.
    best: Option<BestTry>,
    /// The best try, whose files the next try starts from, once a try passed
    /// fewer steps than it.
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
    /// checkout `workdir` with `step_outcomes`. When it passed fewer steps
    /// than the task's best try in that checkout, the next try starts from
    /// the best try's files ([`Self::restore_best_try`]), and this returns
    /// the note its feedback carries, with this try's patch against them.
    pub(super) async fn settle_best_try(
        &self,
        spec: &TaskExecutionSpec,
        task: &TaskDef,
        attempt: u32,
        workdir: &Path,
        steps: &[(String, crate::task_parser::VerifyStep)],
        step_outcomes: &[(String, bool)],
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
                .filter(|best| {
                    best.workdir == this.workdir && this.passed.len() < best.passed.len()
                })
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
            "the try passed fewer verify steps than the task's best: the next try starts from \
             the best try's files"
        );
        Some(progress_note(&best, &this, steps.len(), patch.as_deref()))
    }

    /// Before a try of `task` in `workdir`, put back the task's best try's
    /// files when its last try passed fewer steps ([`Self::settle_best_try`]).
    /// On the task's first try in a resumed run's process, whose kept
    /// checkout holds the last try's tree, start from the best try's files,
    /// or clean ([`Self::resumed_start`], gap-586507).
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
    /// best try's tree when that try passed a step in this checkout, and
    /// clean, from the checkout's base, otherwise: never from the last try's
    /// tree. The tree and what it is; `None` for a task no earlier process
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
        let kept = book
            .best_try(&spec.plan_id, &task.id)
            .filter(|best| !best.passed.is_empty() && best.workdir == workdir);
        if let Some(best) = kept {
            let object = format!("{}^{{tree}}", best.tree);
            let args = ["rev-parse", "--verify", "--quiet", object.as_str()];
            if git_line(workdir, &args, None).await.is_some() {
                let start = (best.tree.clone(), format!("try {}'s files", best.attempt));
                // This process knows it as the task's best try from here on.
                let key = format!("{}/{}", spec.plan_id, task.id);
                if let Some(tries) = self.best_tries.tasks.lock().get_mut(&key) {
                    tries.best = Some(best);
                }
                return Some(start);
            }
        }
        let args = ["rev-parse", "--verify", "--quiet", "HEAD^{tree}"];
        let base = git_line(workdir, &args, None).await?;
        Some((base, "the checkout's base".to_string()))
    }
}

/// The feedback note of a try that passed fewer of the `total` verify steps
/// than `best`: what passed, that its files come back, and `patch`, what
/// the failed try changed from them.
fn progress_note(best: &BestTry, this: &BestTry, total: usize, patch: Option<&str>) -> String {
    let passed = best
        .passed
        .iter()
        .map(|label| format!("`{label}`"))
        .collect::<Vec<_>>()
        .join(", ");
    let mut note = format!(
        "Kept by roko: try {} passed {} of {total} verify steps ({passed}), and this try passed {}. \
         Before your next try, roko puts try {}'s files back: start from its code, keep those \
         steps passing, and fix what still fails.",
        best.attempt,
        best.passed.len(),
        this.passed.len(),
        best.attempt,
    );
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
        make_spec, make_test_dispatcher_with, no_auto_fix, verify_step,
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
            no_auto_fix,
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
        task.verify = vec![
            verify_step("test", "grep -qx A a.txt"),
            verify_step("test", "grep -qx B b.txt"),
            verify_step("test", "grep -qx C c.txt"),
        ];
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

    /// gap-586507: when no try of a resumed task passed a step, its first try
    /// in the new process starts clean, from its checkout's base, never from
    /// the last try's tree.
    #[tokio::test]
    async fn a_resumed_task_that_passed_nothing_starts_clean() {
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
            "missing",
            "the resumed try starts clean"
        );
    }
}
