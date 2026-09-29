+++
id = "bug-a3760a"
kind = "bug"
title = "The Graph engine's merge step runs git checkout in the user's working tree"
status = "open"
triage = "verified"
severity = "p1"
goal = "core"
subsystem = ["roko-cli/graph-execution"]
created = 2026-09-28
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/cybernetic-harness/assessment-2026-09-28/process-capacity.md"
discovered_from = "audit:tmp/cybernetic-harness/assessment-2026-09-28/process-capacity.md"
anchors = ["crates/roko-cli/src/graph_execution/delivery.rs::git_merge", "crates/roko-cli/src/graph_execution/delivery.rs:119"]
links = { depends_on = [], blocks = [], related = ["gap-a85a1f", "gap-3b5361"], supersedes = [], duplicate_of = "" }
+++
`git_merge` merges a task branch by running `git checkout <target>` with `current_dir` set to the workspace (`graph_execution/delivery.rs:116-119`). That switches the branch of the user's own working tree. It fails on, or carries over, uncommitted changes, and it races with anything else using that tree: an editor, another session, a second plan.

Fix: merge in a dedicated worktree or with git plumbing (`git merge-tree` plus `update-ref`), so the user's checkout is never touched.
