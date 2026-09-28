+++
id = "find-c71b6e"
kind = "finding"
title = "GitHub Workflow: Idempotent Comments, 401 Detection, and Pre-Merge Check"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-cli"]
created = 2026-09-07
updated = 2026-09-28
source = "tmp/backlog/archive/98-github-workflow-robustness.md#98 — GitHub Workflow: Idempotent Comments, 401 Detection, and Pre-Merge Check"
discovered_from = "audit:tmp/backlog/archive/98-github-workflow-robustness.md#98 — GitHub Workflow: Idempotent Comments, 401 Detection, and Pre-Merge Check"
anchors = ["crates/roko-cli", "crates/roko-mcp-github", "crates/roko-cli/src/runner/github_workflow.rs", "crates/roko-cli/src/github_ops_impl.rs", "crates/roko-cli/src/github_ops.rs", "crates/roko-mcp-github/src/lib.rs", "LiveGitHubOps::comment_pr", "GitHubOps"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
reliability; duplicate PR comments and silent auth failures degrade the GitHub integration in production. The roko runner posts GitHub comments and attempts PR merges as part of the GitHub workflow integration wired in E46. This integration currently has three independent correctness gaps that…

Imported without verification from:
- `tmp/backlog/archive/98-github-workflow-robustness.md#98 — GitHub Workflow: Idempotent Comments, 401 Detection, and Pre-Merge Check`

Some cited files are gone: `crates/roko-cli/src/runner/github_workflow.rs`.

How to verify: Check: When a task gate comment is posted twice for the same `task_id`, the PR ends up with exactly one comment (the second call updates the first).; When `GITHUB_TOKEN` has expired, the error message contains the string "token may have expired" or… [evidence: 00-STATUS-SUMMARY 3. Open / P2 -- Medium (Open): M | 3 |]
