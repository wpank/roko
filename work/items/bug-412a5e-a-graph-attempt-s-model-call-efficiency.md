+++
id = "bug-412a5e"
kind = "bug"
title = "A Graph attempt's model-call efficiency row is written under its own worktree, not the workspace root"
status = "open"
triage = "verified"
severity = "p2"
goal = "truth"
size = "M"
subsystem = ["roko-cli/dispatch", "roko-fs/layout"]
created = 2026-10-03
updated = 2026-10-03
last_verified = 2026-10-03
source = "wave-6 follow-up reports 2026-10-03 (gate 6b)"
discovered_from = "gate 6b (PK03's test conflict_retry_prompt_names_the_conflict, attempt_workspace.rs)"
anchors = ["crates/roko-cli/src/dispatch_v2.rs::record_agent_dispatch_feedback", "crates/roko-fs/src/layout.rs::RokoLayout::for_project"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = ["bug-633b68"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn model_call_efficiency_row_lands_at_workspace_root_not_the_worktree' crates/roko-cli/ && cargo test -p roko-cli model_call_efficiency_row_lands_at_workspace_root_not_the_worktree"
+++

## Problem

A Graph attempt's model-call efficiency row is written under the **attempt's own per-task worktree**, not the
workspace's root `.roko/learn/`, so it gets swept into whatever the attempt commits — and conflicts with a
sibling attempt that touched the same file.

Root cause: `record_agent_dispatch_feedback` (`crates/roko-cli/src/dispatch_v2.rs`, around line 2032-2047),
the function Graph dispatch's attempts and helper calls use to record a bridge call's efficiency row and
provider health (its own doc comment: "Its callers are Graph dispatch's attempts and helper calls"):

```rust
let learn_dir = roko_fs::RokoLayout::for_project(&request.workdir).learn_dir();
```

`RokoLayout::for_project` does no root-finding — it is exactly `project_root.as_ref().join(".roko")`
(`crates/roko-fs/src/layout.rs:98-100`, confirmed by its own test `for_project_appends_dot_roko`). So
`learn_dir` resolves to `<request.workdir>/.roko/learn/`. For a Graph attempt dispatched inside a per-task
worktree, `request.workdir` **is** that worktree's path, not the workspace root — so the efficiency row lands
at `<worktree>/.roko/learn/efficiency.jsonl`.

Gate 6b's PK03 test `conflict_retry_prompt_names_the_conflict`
(`crates/roko-cli/src/graph_task_dispatch/attempt_workspace.rs`) demonstrates the effect: two sibling attempts
in per-task worktrees of a repo that doesn't ignore `.roko/` conflict on `.roko/learn/efficiency.jsonl` as
well as on the task's own file (`same.txt`), because each attempt's accepted commit carried the worktree's own
copy of that log. `crates/roko-cli/src/graph_task_dispatch/failover.rs`'s tests already read back
`workdir.join(".roko/learn/efficiency.jsonl")` (lines 1363, 1414) — i.e. the test suite's own fixtures already
treat the per-attempt workdir as where this file lives, consistent with the bug being real in production, not
just a test artifact.

## Why it matters

Goal `truth`/release hygiene. In production this means one of two bad outcomes depending on the project's
`.gitignore`: (a) `.roko/` is not ignored — a worktree attempt's model-call efficiency rows get committed into
the plan branch as unrelated noise, and parallel siblings can conflict on that file purely because of this
internal bookkeeping, unrelated to either task's actual work; or (b) `.roko/` *is* ignored — the efficiency
row is silently lost with the worktree once the attempt's lease ends, so the workspace's root
`.roko/learn/efficiency.jsonl` never sees that attempt's model-call data at all, which is the same class of
gap `bug-633b68` (done) already fixed once for the tool-immune quarantine vault ("Plan runs root the
tool-immune vault at the task lease path, so the workspace quarantine route misses it" — `immune_root` set to
`lease.path.clone()` instead of the workspace root). That fix did not touch `dispatch_v2.rs`'s efficiency path,
so the same pattern recurs here.

## Where

- `crates/roko-cli/src/dispatch_v2.rs::record_agent_dispatch_feedback` (~line 2032-2047): the `learn_dir`
  resolution.
- `crates/roko-fs/src/layout.rs::RokoLayout::for_project` / `::learn_dir` (no root-walking; exact-join only).
- `AgentDispatchRequest::workdir` — wherever it's set to the attempt's worktree path for a Graph dispatch
  (search `graph_task_dispatch.rs`/`graph_task_dispatch/streaming.rs` for where the request is built from the
  attempt's lease).
- Prior art for the fix shape: `bug-633b68` (done) — rooted the immune vault at the workspace instead of the
  lease path (`5d637b7fd`).
- Demonstrating test: `crates/roko-cli/src/graph_task_dispatch/attempt_workspace.rs::conflict_retry_prompt_names_the_conflict`.

## Current state

Unfixed. `record_agent_dispatch_feedback` has no parameter carrying the workspace root separately from the
attempt's `request.workdir`; it derives `learn_dir` from the latter only.

## Plan

1. Give `record_agent_dispatch_feedback` (or its caller) the workspace root, not just the attempt's `workdir`,
   and resolve `learn_dir` from the root (`RokoLayout::for_project(workspace_root).learn_dir()`) instead of the
   attempt's worktree.
2. Decide the same question `bug-633b68` already answered for the immune vault: per-task worktrees should never
   be the root for anything meant to be workspace-wide state (efficiency log, cascade-router observations,
   provider health, knowledge). Check whether `ModelCallFeedbackRecorder::without_cascade_router(learn_dir)`'s
   other state (cascade-router snapshot, provider-health file) shares the same mis-rooted `learn_dir` and needs
   the same fix in the same change.
3. Add a regression test: two sibling attempts in separate worktrees of a repo that does not ignore `.roko/`
   each make a model call; after both settle, the workspace root's `.roko/learn/efficiency.jsonl` has both
   rows, and neither attempt's accepted commit carries a `.roko/learn/` file at all.

## Done when

- A Graph attempt's model-call efficiency row lands in the workspace root's `.roko/learn/efficiency.jsonl`,
  never inside its own worktree.
- Two siblings committing in parallel worktrees no longer conflict on `.roko/learn/efficiency.jsonl`.
- The `[[verify]]` command passes.

## Notes

- Check whether attempt-level efficiency writes (`attempt.rs`'s "W05: Efficiency event", which already takes
  an explicit `efficiency_path` on `GraphFeedbackContext` rather than deriving one from a workdir) are already
  rooted correctly at the workspace — if so, this is specifically a `dispatch_v2.rs`/model-call-level gap, not
  an attempt-level one, and the fix should not touch the already-correct path.
- Related: `bug-633b68` (done, same bug class, different subsystem — the tool-immune vault). `bug-c55f1c` and
  `bug-ba53d1` (earlier batches today) are other failover-adjacent double-counting bugs in the same general
  area of the codebase, but unrelated mechanisms.

2026-10-03 (second concurrent pass, same gate-6b finding): confirmed `request.workdir` really is the
per-task worktree for an isolated attempt — `graph_task_dispatch.rs:1008-1010`:
`effective_workdir = lease.as_ref().map_or_else(|| self.workdir.clone(), |l| l.path.clone())`, its own
comment "Effective working directory: worktree path if isolated, else shared workdir", and this
`effective_workdir` is what becomes `AgentDispatchRequest.workdir` at lines 1164/1288. That was the one link
the original write-up above left to "search for" — it holds. Superseded `gap-b0635e` into this item: it
independently traced the (correctly-rooted) attempt-level `GraphFeedbackContext`/`build_graph_feedback_context`
path and concluded "unconfirmed" only because it didn't look at `dispatch_v2.rs`; its two still-useful open
threads, carried over here: (a) finish tracing `attempt_workspace.rs::accept_attempt` →
`graph_execution/workspaces.rs::accept` → the worktree manager's own commit/stage step, to confirm whether it
stages a targeted changeset or sweeps the whole worktree (determines whether *any* stray `.roko/` file would
reach a commit, not just this one); (b) `recording_feedback`'s doc comment ("Every record file a Graph attempt
writes, under `workdir/.roko`") is ambiguous about which `workdir` and is test-only code using the same
worktree-keyed pattern as the real bug — tighten its comment once this item's fix lands, so a future caller
doesn't copy the pattern into production.
