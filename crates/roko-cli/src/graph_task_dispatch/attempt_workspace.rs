//! Tests of the attempt identity and checkout a Graph task's output names
//! (bug-50caf2).

use std::process::Command as StdCommand;

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

pub(super) fn git(dir: &Path, args: &[&str]) -> String {
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
pub(super) fn repo_with_worktrees() -> (tempfile::TempDir, tempfile::TempDir) {
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

pub(super) fn worktree_provider(
    repo: &Path,
    worktrees: &Path,
) -> Arc<WorktreeExecutionWorkspaceProvider> {
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

/// A dispatcher for the repository `repo` whose tasks each run in their own
/// worktree under `worktrees`, and whose provider writes `feature.txt`.
pub(super) async fn isolated_dispatcher(
    repo: &tempfile::TempDir,
    worktrees: &Path,
) -> (Arc<GraphTaskDispatcher>, TaskDef) {
    let provider = worktree_provider(repo.path(), worktrees);
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

/// bug-50caf2: with `keep_workspace` (the rich topology), the output names
/// the attempt, its key and its worktree, and hands the worktree on, agent
/// changes and all, for the task's gate to judge. The operator's checkout
/// never sees the change.
#[tokio::test]
async fn graph_output_hands_the_attempt_worktree_on_to_the_gate() {
    let (repo, worktrees) = repo_with_worktrees();
    let (dispatcher, task) = isolated_dispatcher(&repo, worktrees.path()).await;
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
    assert_eq!(
        std::fs::read_to_string(worktree.join("feature.txt")).expect("the change is kept"),
        "feature\n"
    );
    assert!(!repo.path().join("feature.txt").exists());
    assert_eq!(
        TaskGateVerdict::from_signals(&outputs),
        Some(TaskGateVerdict::Passed)
    );

    // The task's plan gate judges that worktree: it sees the agent's change,
    // which the process's own directory does not have.
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
    });
    let verdict = roko_graph::Cell::execute(&gate, outputs, &gate_ctx)
        .await
        .expect("the gate runs");
    assert_eq!(verdict[0].tag("gate.passed"), Some("true"));
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
