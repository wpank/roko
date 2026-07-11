# Shared worktree destroys task ownership

- Severity: critical
- Area: isolation / Git

Concurrent tasks use one plan worktree and Cargo target. Agents edit moving shared state; gates test unrelated sibling changes; Cargo commands repeatedly contend on package/build locks; task commits can sweep changes from another agent.

After failure the worktree contains 185 insertions and 52 deletions across `orchestrate.rs`, `gate_dispatch.rs`, `merge.rs`, and `rung_dispatch.rs`, with no provenance mapping. Snapshot `files_changed` is empty.

Use task/attempt worktrees or serialize plan tasks. Commits and gates must operate on an immutable task-owned diff, then merge through a queue with conflict and rollback handling.

