+++
id = "gap-cd51b7"
kind = "gap"
title = "GitHub PR and failure-issue automation is never constructed at runtime"
status = "open"
triage = "verified"
severity = "p2"
goal = "features"
subsystem = ["roko-cli/github"]
created = 2026-09-28
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "ebdc0f5d5"
source = "tmp/work-management/03-roko-native-capabilities.md"
discovered_from = "doc:tmp/work-management/03-roko-native-capabilities.md"
anchors = ["crates/roko-cli/src/runner/types.rs::RunConfig::from_roko_config", "crates/roko-cli/src/runner/types.rs:2726", "crates/roko-cli/src/github_ops_impl.rs::LiveGitHubOps::from_config", "crates/roko-cli/src/github_ops.rs::GitHubOps::create_task_issue"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rn 'LiveGitHubOps::from_config' crates/roko-cli/src --include='*.rs' | grep -v github_ops_impl.rs | grep -q . && grep -rqn 'create_task_issue\\|create_pr' crates/roko-cli/src/graph_execution && cargo test -p roko-cli --lib github_ops"
+++

`GitHubOpsImpl` implements plan branches, PRs, CI checks and task-failure issues, but every production initializer passes `github_ops: None` (`runner/types.rs:2668`, `:2726`, `commands/do_cmd.rs:985`), and `create_task_issue` has no production caller.
Enabling `auto_pr` therefore has no effect; `roko github status` is read-only.
Fix: construct `GitHubOpsImpl` from `GitHubConfig` when owner, repo and token are present, inject it into the Graph plan runner, and cover PR/issue creation with a mock client test.

Rechecked 2026-09-29 at d9e79e9d8. The adapter type is LiveGitHubOps (crates/roko-cli/src/github_ops_impl.rs), not GitHubOpsImpl. The do_cmd.rs initializer was removed in 725f21e05. RunConfig.github_ops is never read, so the fix must also add a consumer on the Graph plan runner (graph_execution), not only construct the adapter. worker/cloud.rs:611 creates PRs through the GitHub MCP tool for cloud executions only; plan runs still never open PRs or failure issues.

## Notes

- 2026-10-01 (wk-filer4): implemented on work/gap-cd51b7; cargo verification deferred to the batch check. This is the first step: PRs remain (below), so the item stays open.
- 2026-10-01 (wk-filer4): What changed: `RunConfig::from_roko_config` builds `LiveGitHubOps` when `[github] auto_pr` is on, as docs/v2/GITHUB-INTEGRATION.md ("Runner workflow") describes, on a thread of its own: reqwest's blocking client panics when built on an async runtime thread in debug builds. `run_one_plan` files one `<label_prefix>task-failure` issue per failed task through the new `graph_execution/failure_issues.rs` (not for interrupted or cancelled runs). The issue body quotes the error, scrubbed and cut to 2,000 characters, in a code block. The tests use a recording fake `GitHubOps`, never GitHub.
- 2026-10-01 (wk-filer4): Left: (a) PRs. `create_plan_branch` and `open_pr` still have no caller. Batch delivery publishes nothing (`publish: false`, graph_execution/batch.rs:175 and :279), and GitHub refuses a PR whose branch has no commits, so the PR step needs a decision on when the plan branch is pushed. (b) Closing a failure issue once its task passes, and not filing a second issue when the task fails again: both need the issue number kept per task, and nothing stores it yet. (c) `auto_update_prs` PR comments.
