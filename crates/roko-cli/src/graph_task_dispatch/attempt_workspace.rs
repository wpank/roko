//! Attempt worktrees of Graph task dispatch: accepting a successful attempt
//! onto its plan branch (gap-3b5361), holding it for a person's approval
//! first when its plan asks for that (gap-0d64d5), and the attempt and
//! checkout a task's output names (bug-50caf2).

use roko_graph::workspace::{
    ExecutionWorkspaceProvider, WorkspaceAcceptRequest, WorkspaceAcceptance, WorkspaceError,
    WorkspaceLease, WorkspaceReleasePolicy,
};

use super::*;

impl GraphTaskDispatcher {
    /// Checkout generation of the task `task_key` (`"{plan_id}/{task_id}"`):
    /// its worktree is the workspace attempt `(plan, task, generation)`.
    /// Retries of a task share that checkout, so a retry resumes the work its
    /// predecessor left. When the plan branch refuses the task's work as
    /// conflicting, the task moves on to a fresh checkout of the plan's
    /// accepted tip.
    pub(super) fn worktree_generation(&self, task_key: &str) -> u32 {
        self.worktree_generations
            .lock()
            .get(task_key)
            .copied()
            .unwrap_or_default()
    }

    /// Accept the successful attempt `settled` of `task`, which ran in `lease`
    /// (gap-3b5361): commit its checkout and fold it into the plan branch,
    /// which the plan's later attempts start from.
    ///
    /// Acceptance follows the settled `verdict`: the verdicts a completed
    /// task replays (passed, unverified) are accepted, a forced accept never
    /// is. The checkout is kept for review either way. Work that conflicts
    /// with the plan's accepted work fails the attempt, and the task's next
    /// attempt starts over in a fresh checkout of the plan branch; any other
    /// refusal fails the task without a retry. When the plan holds its tasks
    /// for approval, the attempt waits for it first ([`Self::await_review`]).
    #[allow(clippy::too_many_arguments)]
    pub(super) async fn accept_attempt(
        &self,
        spec: &TaskExecutionSpec,
        task: &TaskDef,
        settled: &SettledAttempt,
        verdict: TaskGateVerdict,
        provider: &dyn ExecutionWorkspaceProvider,
        lease: &WorkspaceLease,
        ctx: &CellContext,
    ) -> Result<Option<WorkspaceAcceptance>> {
        if verdict.is_replayable() && self.holds_for_approval(&spec.plan_id) {
            if let Err(error) = self.await_review(spec, task, settled, lease, ctx).await {
                if let Err(release) = provider
                    .release(lease, WorkspaceReleasePolicy::RetainForReview)
                    .await
                {
                    tracing::warn!(
                        plan_id = %spec.plan_id,
                        task_id = %task.id,
                        error = %release,
                        "could not keep the attempt's worktree for review"
                    );
                }
                return Err(error);
            }
        }
        let accepted = if verdict.is_replayable() {
            let request = WorkspaceAcceptRequest {
                run_id: settled.key().run_id,
                attempt_key: settled.attempt_key().to_string(),
                verdict: verdict.as_str().to_string(),
                title: task.title.clone(),
            };
            Some(provider.accept(lease, &request).await)
        } else {
            None
        };
        if let Err(error) = provider
            .release(lease, WorkspaceReleasePolicy::RetainForReview)
            .await
        {
            tracing::warn!(
                plan_id = %spec.plan_id,
                task_id = %task.id,
                %error,
                "could not keep the attempt's worktree for review"
            );
        }
        match accepted {
            None => {
                tracing::warn!(
                    plan_id = %spec.plan_id,
                    task_id = %task.id,
                    verdict = verdict.as_str(),
                    worktree = %lease.path.display(),
                    "the attempt's verdict does not let its work land; kept its worktree"
                );
                Ok(None)
            }
            Some(Ok(acceptance)) => {
                tracing::info!(
                    plan_id = %spec.plan_id,
                    task_id = %task.id,
                    attempt_key = settled.attempt_key(),
                    plan_branch = %acceptance.plan_branch,
                    accepted_commit = %acceptance.accepted_commit,
                    worktree = %lease.path.display(),
                    "accepted the attempt onto its plan branch"
                );
                Ok(Some(acceptance))
            }
            Some(Err(WorkspaceError::Conflict(reason))) => {
                *self
                    .worktree_generations
                    .lock()
                    .entry(format!("{}/{}", spec.plan_id, task.id))
                    .or_default() += 1;
                Err(RokoError::Verify {
                    gate: "plan-branch".to_string(),
                    message: format!(
                        "attempt {} passed its gates, but its work conflicts with the plan's \
                         accepted work, so it was not accepted ({reason}); its worktree is kept \
                         at {}, and the next attempt starts over from the plan branch",
                        settled.attempt_key(),
                        lease.path.display()
                    ),
                })
            }
            Some(Err(error)) => Err(RokoError::Rejected(format!(
                "attempt {} passed its gates, but was not accepted onto its plan branch: \
                 {error}; its worktree is kept at {}",
                settled.attempt_key(),
                lease.path.display()
            ))),
        }
    }

    /// Hold the verified attempt `settled` of `task`, which ran in `lease`,
    /// until a person approves or rejects it (gap-0d64d5).
    ///
    /// What the attempt changed goes to the task's review hold
    /// ([`roko_fs::RokoLayout::review_hold`]), where `roko serve`'s review routes
    /// show it. The hold then waits for a decision on this attempt in the
    /// review log ([`roko_fs::RokoLayout::reviews_log`]), which
    /// `POST /api/plans/:id/tasks/:task_id/review` writes. Approval returns
    /// `Ok`, and the attempt is accepted. A rejection (or a skip) fails the
    /// attempt with `gate: "review"`, and the reviewer's note is the next
    /// attempt's feedback. A cancelled run ends the hold without a decision.
    /// The hold is removed either way.
    pub(super) async fn await_review(
        &self,
        spec: &TaskExecutionSpec,
        task: &TaskDef,
        settled: &SettledAttempt,
        lease: &WorkspaceLease,
        ctx: &CellContext,
    ) -> Result<()> {
        let layout = roko_fs::RokoLayout::for_project(&self.workdir);
        let hold_path = layout.review_hold(&spec.plan_id, &task.id);
        let attempt_key = settled.attempt_key().to_string();
        let diff = crate::orchestrator::worktree::attempt_review_diff(&lease.path)
            .await
            .map_err(|error| {
                RokoError::Rejected(format!(
                    "attempt {attempt_key} needs a review, but its change could not be read: \
                     {error}"
                ))
            })?;
        let hold = serde_json::json!({
            "schema_version": 1,
            "plan_id": spec.plan_id,
            "task_id": task.id,
            "title": task.title,
            "run_id": settled.key().run_id,
            "attempt_key": attempt_key,
            "attempt": settled.key().attempt,
            "worktree": lease.path,
            "branch": lease.branch,
            "base": diff.base,
            "numstat": diff.numstat,
            "patch": diff.patch,
            "held_at": chrono::Utc::now().to_rfc3339(),
        });
        write_review_hold(&hold_path, &hold)?;
        tracing::info!(
            plan_id = %spec.plan_id,
            task_id = %task.id,
            attempt_key,
            hold = %hold_path.display(),
            "the verified attempt waits for a review before it is accepted"
        );
        if let Some(tui) = &self.tui_bridge {
            tui.gate_result_with_output(
                &spec.plan_id,
                &task.id,
                "review",
                false,
                Some("waiting for a review: approve or reject the task's diff"),
            );
        }

        let reviews = layout.reviews_log();
        let decision = loop {
            if let Some(decision) = review_decision(&reviews, &spec.plan_id, &task.id, &attempt_key)
            {
                break Some(decision);
            }
            if ctx.is_cancelled() {
                break None;
            }
            tokio::time::sleep(REVIEW_POLL_INTERVAL).await;
        };
        if let Err(error) = std::fs::remove_file(&hold_path) {
            tracing::warn!(hold = %hold_path.display(), %error, "could not remove the review hold");
        }
        let Some((decision, note)) = decision else {
            return Err(RokoError::Cancelled(format!(
                "the run was cancelled while attempt {attempt_key} waited for a review"
            )));
        };
        if decision == "approved" {
            tracing::info!(plan_id = %spec.plan_id, task_id = %task.id, attempt_key, "a reviewer approved the attempt");
            return Ok(());
        }
        let note = if note.trim().is_empty() {
            "no note".to_string()
        } else {
            note
        };
        let message = format!("a reviewer {decision} attempt {attempt_key}: {note}");
        if let Some(feedback) = GateFeedback::from_raw(&message) {
            self.gate_retry_context.record(
                &spec.plan_id,
                &task.id,
                feedback,
                settled.key().attempt.saturating_add(1),
            );
        }
        Err(RokoError::Verify {
            gate: "review".to_string(),
            message,
        })
    }
}

/// How often a held attempt looks for its review decision.
const REVIEW_POLL_INTERVAL: std::time::Duration = std::time::Duration::from_millis(250);

/// Write `hold` to `path` whole: to a temporary file beside it, then renamed.
fn write_review_hold(path: &Path, hold: &serde_json::Value) -> Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| RokoError::Invalid(format!("no directory for {}", path.display())))?;
    std::fs::create_dir_all(parent)?;
    let temporary = path.with_extension(format!("json.tmp.{}", uuid::Uuid::new_v4().simple()));
    std::fs::write(&temporary, serde_json::to_vec_pretty(hold)?)?;
    std::fs::rename(&temporary, path)?;
    Ok(())
}

/// The last decision in the review log `reviews` on attempt `attempt_key`
/// of `task_id` in `plan_id`: `approved`, `rejected` or `skipped`, and the
/// reviewer's note.
fn review_decision(
    reviews: &Path,
    plan_id: &str,
    task_id: &str,
    attempt_key: &str,
) -> Option<(String, String)> {
    let log = std::fs::read_to_string(reviews).ok()?;
    log.lines()
        .rev()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .find(|entry| {
            entry["plan_id"] == plan_id
                && entry["task_id"] == task_id
                && entry["attempt_key"] == attempt_key
        })
        .and_then(|entry| {
            let decision = entry["decision"].as_str()?.to_string();
            let note = entry["comment"].as_str().unwrap_or_default().to_string();
            Some((decision, note))
        })
}

#[cfg(test)]
mod tests {
    use std::process::Command as StdCommand;

    use roko_graph::workspace::{
        WorkspaceAttemptId, WorkspaceLeaseState, WorkspaceReconcileResult,
    };
    use tempfile::tempdir;

    use super::*;
    use crate::graph_execution::WorktreeExecutionWorkspaceProvider;
    use crate::graph_task_dispatch::tests::{
        make_spec, make_test_dispatcher_with, no_auto_fix, verify_step,
    };
    use crate::orchestrator::worktree::{WorktreeConfig, WorktreeManager};

    const RUN: &str = "graph-worktree-run";

    /// Provider that adds `feature.txt` to the directory it runs in.
    const WRITES_FEATURE_PROVIDER: &str = r#"#!/bin/sh
set -eu
cat >/dev/null
printf 'feature\n' > feature.txt
printf '%s\n' '{"type":"content_block_delta","delta":{"text":"wrote the feature"}}'
printf '%s\n' '{"type":"result","session_id":"sess-w","model":"claude-sonnet-4-6","total_cost_usd":0.01,"usage":{"input_tokens":5,"output_tokens":10}}'
"#;

    fn git(dir: &Path, args: &[&str]) -> String {
        let output = StdCommand::new("git")
            .current_dir(dir)
            .args(args)
            .env_remove("GIT_DIR")
            .env_remove("GIT_WORK_TREE")
            .env_remove("GIT_INDEX_FILE")
            .output()
            .expect("run git");
        assert!(
            output.status.success(),
            "git {args:?} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8_lossy(&output.stdout).trim().to_string()
    }

    /// A repository whose `main` has one commit, and the worktree root for its
    /// attempts (outside the repository).
    fn repo_with_worktrees() -> (tempfile::TempDir, tempfile::TempDir) {
        let repo = tempdir().expect("repo");
        git(repo.path(), &["init", "-b", "main"]);
        for (key, value) in [
            ("user.email", "operator@example.test"),
            ("user.name", "Operator"),
            ("commit.gpgsign", "false"),
        ] {
            git(repo.path(), &["config", key, value]);
        }
        std::fs::write(repo.path().join("README.md"), "base\n").expect("write README");
        git(repo.path(), &["add", "README.md"]);
        git(repo.path(), &["commit", "-m", "base"]);
        (repo, tempdir().expect("worktrees"))
    }

    fn worktree_provider(repo: &Path, worktrees: &Path) -> Arc<WorktreeExecutionWorkspaceProvider> {
        Arc::new(WorktreeExecutionWorkspaceProvider::new(
            WorktreeManager::new(WorktreeConfig {
                repo_root: repo.to_path_buf(),
                base_branch: "HEAD".to_string(),
                worktrees_root: worktrees.to_path_buf(),
                max_live: None,
                idle_ttl: std::time::Duration::from_secs(3600),
            }),
        ))
    }

    /// A dispatcher for the repository `repo` whose tasks each run in their
    /// own worktree from `provider`, and whose agent writes `feature.txt`.
    async fn isolated_dispatcher(
        repo: &tempfile::TempDir,
        provider: Arc<dyn ExecutionWorkspaceProvider>,
    ) -> (Arc<GraphTaskDispatcher>, TaskDef) {
        let (dispatcher, mut task) = make_test_dispatcher_with(
            repo,
            WRITES_FEATURE_PROVIDER,
            no_auto_fix,
            GraphFeedbackContext::default(),
            |dispatcher| dispatcher.with_workspace_provider(provider),
        )
        .await;
        task.verify = vec![verify_step("structural", "test -f feature.txt")];
        (dispatcher, task)
    }

    /// The operator's checkout is where it was: on `main` at `head`, with no
    /// file of the agent's.
    fn assert_operator_checkout_untouched(repo: &Path, head: &str) {
        assert_eq!(git(repo, &["symbolic-ref", "--short", "HEAD"]), "main");
        assert_eq!(git(repo, &["rev-parse", "HEAD"]), head);
        assert!(!repo.join("feature.txt").exists());
    }

    /// bug-50caf2, gap-3b5361: with `keep_workspace` (the rich topology), the
    /// output names the attempt, its key and its worktree, and hands the
    /// worktree on, agent changes and all. The task's plan gate judges that
    /// worktree, then accepts the attempt onto its plan branch. The
    /// operator's checkout never changes.
    #[tokio::test]
    async fn graph_output_hands_the_attempt_worktree_on_to_the_gate() {
        let (repo, worktrees) = repo_with_worktrees();
        let head = git(repo.path(), &["rev-parse", "HEAD"]);
        let provider = worktree_provider(repo.path(), worktrees.path());
        let (dispatcher, task) = isolated_dispatcher(&repo, provider.clone()).await;
        let mut spec = make_spec(&task);
        spec.keep_workspace = true;
        let ctx = CellContext::new().with_run_id(RUN.to_string());

        let outputs = dispatcher
            .dispatch(&spec, Vec::new(), &ctx)
            .await
            .expect("the verified attempt passes");

        let attempt = TaskAttempt::from_signals(&outputs).expect("the output names its attempt");
        assert_eq!(attempt.plan_id, spec.plan_id);
        assert_eq!(attempt.task_id, task.id);
        assert_eq!(attempt.run_id.as_deref(), Some(RUN));
        assert_eq!(attempt.attempt, 1);
        let key = format!("{RUN}:{}:{}:1", spec.plan_id, task.id);
        assert_eq!(attempt.attempt_key.as_deref(), Some(key.as_str()));
        let worktree = attempt.workspace.expect("the attempt ran in a worktree");
        assert!(
            worktree.starts_with(worktrees.path()),
            "{}",
            worktree.display()
        );
        let lease = attempt.lease.expect("the worktree is handed on");
        assert_eq!(lease.path, worktree);
        assert_eq!(attempt.accepted, None, "the gate accepts, not the executor");
        assert_eq!(
            std::fs::read_to_string(worktree.join("feature.txt")).expect("the change is kept"),
            "feature\n"
        );
        assert_eq!(
            TaskGateVerdict::from_signals(&outputs),
            Some(TaskGateVerdict::Passed)
        );
        assert_operator_checkout_untouched(repo.path(), &head);

        // The task's plan gate judges that worktree: it sees the agent's
        // change, which the process's own directory does not have.
        let gate = roko_graph::cells::PlanGateCell::from_config(&toml::Value::Table(
            [
                ("task_id", task.id.as_str()),
                ("plan_id", spec.plan_id.as_str()),
                ("title", task.title.as_str()),
            ]
            .into_iter()
            .map(|(key, value)| (key.to_string(), toml::Value::String(value.to_string())))
            .collect(),
        ));
        let gate_ctx = ctx.clone().with_resources(roko_graph::cell::CellResources {
            gates: Some(Arc::new(PassesWhereTheFeatureIs)),
            workspaces: Some(provider),
        });
        let verdict = roko_graph::Cell::execute(&gate, outputs, &gate_ctx)
            .await
            .expect("the gate runs");
        assert_eq!(verdict[0].tag("gate.passed"), Some("true"));

        // Passing, the attempt is accepted onto its plan branch.
        let settled = TaskAttempt::from_signals(&verdict).expect("the verdict names its attempt");
        assert_eq!(settled.lease, None);
        let accepted = settled.accepted.expect("the gate accepted the attempt");
        let plan_branch = format!("roko/plan/{}", spec.plan_id);
        assert_eq!(accepted.plan_branch, plan_branch);
        assert_eq!(
            git(repo.path(), &["rev-parse", &plan_branch]),
            accepted.accepted_commit
        );
        let landed = format!("{plan_branch}:feature.txt");
        assert_eq!(git(repo.path(), &["show", &landed]), "feature");
        assert_operator_checkout_untouched(repo.path(), &head);
    }

    /// Passes a rung only in a tree that holds `feature.txt`.
    struct PassesWhereTheFeatureIs;

    #[async_trait::async_trait]
    impl roko_core::SharedGateEvaluator for PassesWhereTheFeatureIs {
        async fn verify_rung(
            &self,
            request: &roko_core::SharedGateRequest,
        ) -> std::result::Result<roko_core::SharedGateVerdict, roko_core::SharedGateError> {
            Ok(if request.worktree_path.join("feature.txt").is_file() {
                roko_core::SharedGateVerdict::pass(&request.rung)
            } else {
                roko_core::SharedGateVerdict::fail(&request.rung, vec!["no feature.txt".into()])
            })
        }
    }

    /// Start dispatching `spec` in the background, in run `RUN`.
    fn dispatch_in_background(
        dispatcher: &Arc<GraphTaskDispatcher>,
        spec: &TaskExecutionSpec,
    ) -> tokio::task::JoinHandle<Result<Vec<Signal>>> {
        let dispatcher = Arc::clone(dispatcher);
        let spec = spec.clone();
        tokio::spawn(async move {
            let ctx = CellContext::new().with_run_id(RUN.to_string());
            dispatcher.dispatch(&spec, Vec::new(), &ctx).await
        })
    }

    /// The review hold at `path`, once the held attempt has written it.
    async fn held_review(path: &Path) -> serde_json::Value {
        for _ in 0..600 {
            if let Ok(bytes) = std::fs::read(path) {
                return serde_json::from_slice(&bytes).expect("review hold JSON");
            }
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
        panic!("no review hold at {}", path.display());
    }

    /// Append a reviewer's `decision` on the held attempt `hold` to the
    /// review log, as `roko serve` records it.
    fn decide(repo: &Path, hold: &serde_json::Value, decision: &str, comment: &str) {
        use std::io::Write as _;

        let entry = serde_json::json!({
            "plan_id": hold["plan_id"],
            "task_id": hold["task_id"],
            "attempt_key": hold["attempt_key"],
            "decision": decision,
            "comment": comment,
        });
        let log = roko_fs::RokoLayout::for_project(repo).reviews_log();
        std::fs::create_dir_all(log.parent().expect("state dir")).expect("state dir");
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(log)
            .expect("review log");
        file.write_all(format!("{entry}\n").as_bytes()).expect("record the decision");
    }

    /// gap-0d64d5: with its plan holding tasks for approval, a verified
    /// attempt waits with its diff in the task's review hold, and nothing
    /// reaches the plan branch until a reviewer approves it.
    #[tokio::test]
    async fn approval_hold_blocks_merge_until_approved() {
        let (repo, worktrees) = repo_with_worktrees();
        let head = git(repo.path(), &["rev-parse", "HEAD"]);
        let provider = worktree_provider(repo.path(), worktrees.path());
        let (dispatcher, task) = isolated_dispatcher(&repo, provider).await;
        let spec = make_spec(&task);
        dispatcher.hold_for_approval(&spec.plan_id);
        let hold_path =
            roko_fs::RokoLayout::for_project(repo.path()).review_hold(&spec.plan_id, &task.id);
        let plan_branch = format!("refs/heads/roko/plan/{}", spec.plan_id);
        let merged = || {
            StdCommand::new("git")
                .current_dir(repo.path())
                .args(["rev-parse", "--verify", "--quiet", &plan_branch])
                .status()
                .expect("run git")
                .success()
        };

        let running = dispatch_in_background(&dispatcher, &spec);
        let hold = held_review(&hold_path).await;
        assert_eq!(hold["task_id"], task.id.as_str());
        assert_eq!(hold["base"], head.as_str());
        let key = format!("{RUN}:{}:{}:1", spec.plan_id, task.id);
        assert_eq!(hold["attempt_key"], key.as_str());
        assert!(
            hold["numstat"]
                .as_str()
                .is_some_and(|stat| stat.contains("feature.txt")),
            "{hold}"
        );
        assert!(
            hold["patch"]
                .as_str()
                .is_some_and(|patch| patch.contains("+feature")),
            "{hold}"
        );
        // Held: the attempt waits, and nothing merged.
        tokio::time::sleep(std::time::Duration::from_millis(600)).await;
        assert!(!running.is_finished());
        assert!(!merged());

        decide(repo.path(), &hold, "approved", "");
        let outputs = running
            .await
            .expect("dispatch task")
            .expect("the approved attempt passes");
        let accepted = TaskAttempt::from_signals(&outputs)
            .ok()
            .and_then(|attempt| attempt.accepted)
            .expect("the approved attempt is accepted");
        assert!(merged());
        let landed = format!("{}:feature.txt", accepted.accepted_commit);
        assert_eq!(git(repo.path(), &["show", &landed]), "feature");
        assert!(!hold_path.exists(), "the hold is removed");
        assert_operator_checkout_untouched(repo.path(), &head);
    }

    /// gap-0d64d5: a rejection fails the held attempt without merging it,
    /// and the reviewer's note is the next attempt's feedback.
    #[tokio::test]
    async fn a_rejected_attempt_fails_with_the_reviewers_note() {
        let (repo, worktrees) = repo_with_worktrees();
        let head = git(repo.path(), &["rev-parse", "HEAD"]);
        let provider = worktree_provider(repo.path(), worktrees.path());
        let (dispatcher, task) = isolated_dispatcher(&repo, provider).await;
        let spec = make_spec(&task);
        dispatcher.hold_for_approval(&spec.plan_id);
        let hold_path =
            roko_fs::RokoLayout::for_project(repo.path()).review_hold(&spec.plan_id, &task.id);

        let running = dispatch_in_background(&dispatcher, &spec);
        let hold = held_review(&hold_path).await;
        decide(
            repo.path(),
            &hold,
            "rejected",
            "name the file after the feature",
        );
        let error = running
            .await
            .expect("dispatch task")
            .expect_err("the rejected attempt fails");

        assert!(
            matches!(&error, RokoError::Verify { gate, message }
                if gate == "review" && message.contains("name the file after the feature")),
            "{error:?}"
        );
        let plan_branch = format!("roko/plan/{}", spec.plan_id);
        assert!(git(repo.path(), &["branch", "--list", &plan_branch]).is_empty());
        let feedback = dispatcher
            .gate_retry_context
            .next_attempt(&spec.plan_id, &task.id, 1)
            .feedback
            .expect("the next attempt gets the review");
        assert!(
            feedback
                .raw_output
                .contains("name the file after the feature"),
            "{feedback:?}"
        );
        assert!(!hold_path.exists(), "the hold is removed");
        assert_operator_checkout_untouched(repo.path(), &head);
    }

    /// bug-50caf2: an attempt in the shared working tree names itself but no
    /// worktree, so no gate can mistake the operator's checkout for its own.
    #[tokio::test]
    async fn graph_output_in_the_shared_tree_names_no_worktree() {
        let temp = tempdir().expect("tempdir");
        let (dispatcher, mut task) = make_test_dispatcher_with(
            &temp,
            WRITES_FEATURE_PROVIDER,
            no_auto_fix,
            GraphFeedbackContext::default(),
            |dispatcher| dispatcher,
        )
        .await;
        task.verify = vec![verify_step("structural", "test -f feature.txt")];
        let mut spec = make_spec(&task);
        spec.keep_workspace = true;
        let ctx = CellContext::new().with_run_id(RUN.to_string());

        let outputs = dispatcher
            .dispatch(&spec, Vec::new(), &ctx)
            .await
            .expect("the verified attempt passes");

        let attempt = TaskAttempt::from_signals(&outputs).expect("the output names its attempt");
        assert_eq!(attempt.attempt, 1);
        assert_eq!(attempt.workspace, None);
        assert_eq!(attempt.lease, None);
    }

    /// gap-3b5361: a passed attempt is committed and accepted onto its plan
    /// branch `roko/plan/<plan>`, its worktree is kept for review, and the
    /// output records where its work landed (in the Graph checkpoint). The
    /// plan's next attempt starts from that commit. The operator's checkout
    /// never changes.
    #[tokio::test]
    async fn passed_attempt_is_accepted_onto_its_plan_branch() {
        let (repo, worktrees) = repo_with_worktrees();
        let head = git(repo.path(), &["rev-parse", "HEAD"]);
        let provider = worktree_provider(repo.path(), worktrees.path());
        let (dispatcher, task) = isolated_dispatcher(&repo, provider).await;
        let spec = make_spec(&task);
        let ctx = CellContext::new().with_run_id(RUN.to_string());

        let outputs = dispatcher
            .dispatch(&spec, Vec::new(), &ctx)
            .await
            .expect("the verified attempt passes");

        let attempt = TaskAttempt::from_signals(&outputs).expect("the output names its attempt");
        assert_eq!(attempt.lease, None);
        let accepted = attempt.accepted.expect("the attempt was accepted");
        let plan_branch = format!("roko/plan/{}", spec.plan_id);
        assert_eq!(accepted.plan_branch, plan_branch);
        assert_eq!(accepted.accepted_commit, accepted.attempt_commit);
        assert_eq!(
            git(repo.path(), &["rev-parse", &plan_branch]),
            accepted.accepted_commit
        );
        let landed = format!("{plan_branch}:feature.txt");
        assert_eq!(git(repo.path(), &["show", &landed]), "feature");
        assert_eq!(
            git(repo.path(), &["rev-parse", &format!("{plan_branch}^")]),
            head
        );
        let message = git(repo.path(), &["log", "-1", "--format=%B", &plan_branch]);
        let key = format!("{RUN}:{}:{}:1", spec.plan_id, task.id);
        for trailer in [
            format!("Roko-Attempt: {key}"),
            format!("Roko-Run: {RUN}"),
            "Roko-Verdict: passed".to_string(),
        ] {
            assert!(
                message.contains(&trailer),
                "{trailer} missing from:\n{message}"
            );
        }
        // The worktree is kept, clean: its work is committed.
        let worktree = attempt.workspace.expect("the attempt ran in a worktree");
        assert_eq!(git(&worktree, &["status", "--porcelain"]), "");
        assert_operator_checkout_untouched(repo.path(), &head);

        // The plan's next attempt starts from the accepted commit.
        let mut next = task.clone();
        next.id = "T-NEXT".to_string();
        let outputs = dispatcher
            .dispatch(&make_spec(&next), Vec::new(), &ctx)
            .await
            .expect("the next attempt passes");
        let next_accepted = TaskAttempt::from_signals(&outputs)
            .expect("attempt")
            .accepted
            .expect("accepted");
        assert_eq!(
            git(
                repo.path(),
                &["rev-parse", &format!("{}^", next_accepted.attempt_commit)]
            ),
            accepted.accepted_commit
        );
        assert_eq!(
            git(repo.path(), &["rev-parse", &plan_branch]),
            next_accepted.accepted_commit
        );
        assert_operator_checkout_untouched(repo.path(), &head);
    }

    /// A worktree provider whose plan branch refuses every attempt as
    /// conflicting, and that counts the checkouts it hands out.
    struct RefusesAsConflicting {
        inner: Arc<WorktreeExecutionWorkspaceProvider>,
        acquired: parking_lot::Mutex<Vec<WorkspaceAttemptId>>,
    }

    #[async_trait::async_trait]
    impl ExecutionWorkspaceProvider for RefusesAsConflicting {
        async fn acquire(
            &self,
            attempt_id: &WorkspaceAttemptId,
        ) -> std::result::Result<WorkspaceLease, WorkspaceError> {
            self.acquired.lock().push(attempt_id.clone());
            self.inner.acquire(attempt_id).await
        }

        async fn reconcile(
            &self,
            lease: &WorkspaceLease,
        ) -> std::result::Result<WorkspaceReconcileResult, WorkspaceError> {
            self.inner.reconcile(lease).await
        }

        async fn reset_for_retry(
            &self,
            previous: &WorkspaceLease,
            next_attempt_id: &WorkspaceAttemptId,
        ) -> std::result::Result<WorkspaceLease, WorkspaceError> {
            self.inner.reset_for_retry(previous, next_attempt_id).await
        }

        async fn release(
            &self,
            lease: &WorkspaceLease,
            policy: WorkspaceReleasePolicy,
        ) -> std::result::Result<WorkspaceLeaseState, WorkspaceError> {
            self.inner.release(lease, policy).await
        }

        async fn accept(
            &self,
            _lease: &WorkspaceLease,
            _request: &WorkspaceAcceptRequest,
        ) -> std::result::Result<WorkspaceAcceptance, WorkspaceError> {
            Err(WorkspaceError::Conflict("feature.txt".to_string()))
        }
    }

    /// gap-3b5361: an attempt whose work conflicts with the plan branch is
    /// not accepted and fails as a retryable gate failure, and the task's
    /// next attempt gets a fresh checkout of the plan branch instead of the
    /// conflicting one.
    #[tokio::test]
    async fn conflicting_attempt_fails_and_the_retry_starts_over() {
        let (repo, worktrees) = repo_with_worktrees();
        let provider = Arc::new(RefusesAsConflicting {
            inner: worktree_provider(repo.path(), worktrees.path()),
            acquired: parking_lot::Mutex::new(Vec::new()),
        });
        let (dispatcher, task) = isolated_dispatcher(&repo, provider.clone()).await;
        let spec = make_spec(&task);
        let ctx = CellContext::new().with_run_id(RUN.to_string());
        let task_key = format!("{}/{}", spec.plan_id, task.id);

        for generation in 0..2 {
            assert_eq!(dispatcher.worktree_generation(&task_key), generation);
            let error = dispatcher
                .dispatch(&spec, Vec::new(), &ctx)
                .await
                .expect_err("a conflicting attempt is not accepted");
            assert!(
                matches!(&error, RokoError::Verify { gate, .. } if gate == "plan-branch"),
                "{error:?}"
            );
        }
        let checkouts: Vec<u32> = provider
            .acquired
            .lock()
            .iter()
            .map(|id| id.attempt)
            .collect();
        assert_eq!(checkouts, [0, 1], "each retry gets a fresh checkout");
        assert!(git(repo.path(), &["branch", "--list", "roko/plan/*"]).is_empty());
    }
}
