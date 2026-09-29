//! Attempt worktrees of Graph task dispatch: accepting a successful attempt
//! onto its plan branch (gap-3b5361), and the attempt and checkout a task's
//! output names (bug-50caf2).

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
    /// refusal fails the task without a retry.
    pub(super) async fn accept_attempt(
        &self,
        spec: &TaskExecutionSpec,
        task: &TaskDef,
        settled: &SettledAttempt,
        verdict: TaskGateVerdict,
        provider: &dyn ExecutionWorkspaceProvider,
        lease: &WorkspaceLease,
    ) -> Result<Option<WorkspaceAcceptance>> {
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
