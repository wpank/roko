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
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/work-management/03-roko-native-capabilities.md"
discovered_from = "doc:tmp/work-management/03-roko-native-capabilities.md"
anchors = ["crates/roko-cli/src/runner/types.rs:2668", "crates/roko-cli/src/runner/types.rs:2726", "crates/roko-cli/src/commands/do_cmd.rs:985"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++

`GitHubOpsImpl` implements plan branches, PRs, CI checks and task-failure issues, but every production initializer passes `github_ops: None` (`runner/types.rs:2668`, `:2726`, `commands/do_cmd.rs:985`), and `create_task_issue` has no production caller.
Enabling `auto_pr` therefore has no effect; `roko github status` is read-only.
Fix: construct `GitHubOpsImpl` from `GitHubConfig` when owner, repo and token are present, inject it into the Graph plan runner, and cover PR/issue creation with a mock client test.
