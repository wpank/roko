+++
id = "gap-a9c156"
kind = "gap"
title = "PK77 Domains and assistant: Accept a scratch_dir attempt: copy changed files back, and refuse when the base moved (+2 more)"
status = "open"
triage = "verified"
severity = "p3"
goal = "hermes"
rank = 77
size = "L"
subsystem = ["roko-cli/graph-dispatch"]
created = 2026-10-02
updated = 2026-10-02
last_verified = 2026-10-02
source = "tmp/backlog/2026-10-02-complete-and-wire PK77"
anchors = ["crates/roko-cli/src/graph_task_dispatch/attempt_workspace.rs"]
lane = "rust-cold"
parent = "spec-0b3a32"
links = { depends_on = ["gap-ce1d11", "gap-3c3729", "gap-99c9ae"], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn scratch_dir_accept_copies_back_and_refuses_conflicts' crates/roko-cli/ && cargo test -p roko-cli scratch_dir_accept_copies_back_and_refuses_conflicts"

[[verify]]
command = "grep -rqw 'fn confirm_rung_holds_until_the_host_answers' crates/roko-cli/ && cargo test -p roko-cli confirm_rung_holds_until_the_host_answers"

[[verify]]
command = "grep -rqw 'fn mcp_effects_pending_lists_staged_effects' crates/roko-serve/ && cargo test -p roko-serve mcp_effects_pending_lists_staged_effects"
+++

## Problem

This package delivers 3 tasks of the backlog `tmp/backlog/2026-10-02-complete-and-wire/` (package PK77, slice 91xx, phase 9), in this order. Each task's full specification (Problem, Why it matters, Where, Current state, Plan, Done when, Notes and its verify) is in its file: read each one completely before starting it.

| # | Task | Size | Sev | Title | File |
|---|---|---|---|---|---|
| 1 | 9136 | M | p3 | Accept a scratch_dir attempt: copy changed files back, and refuse when the base moved | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9136-accept-scratch-dir-attempt-copy-back.md` |
| 2 | 9137 | M | p3 | The confirm rung: the person confirms the outcome in chat before the task counts as confirmed | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9137-confirm-rung-user-confirms-in-chat.md` |
| 3 | 9138 | S | p3 | /mcp tools for staged effects, so the host can ask its user to approve them | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9138-mcp-tools-for-pending-effects.md` |

## Why it matters

Phase 9: domains, assistant, held and parked work, cleanup, showcase and deploy. The slice's epic, with its goal and scope, is `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9100-domains-and-the-assistant.md`. The whole order is in `00-INDEX.md` and `PACKAGES.md` in the backlog folder; Will's decisions are in its `DECISIONS.md`.

## Where

Files the tasks change: `crates/roko-cli/src/graph_task_dispatch/attempt_workspace.rs`, `crates/roko-cli/src/graph_task_dispatch/pack_rungs.rs`, `crates/roko-cli/src/orchestrator/scratch.rs`, `crates/roko-serve/src/routes/mcp.rs`.

## Current state

The tasks were checked against `2c3ea9f73` on 2026-10-02. Re-check each task's anchors and premise at your base commit before implementing it, and report a task that is already done instead of redoing it.

## Plan

1. Work through the tasks in the order above. For each: read its file, implement its Plan, write the test it names, and make one commit per task whose message ends with `Backlog-Task: <task id>`, `Work-Item: <this item's id>` and `Executor: claude-agent`.
2. Follow `BUILD-RULES.md` in the backlog folder. Workers run no cargo: Rust is checked by the coordinator's batched gate. Python and doc checks you may run.
3. If a task cannot be done (a premise is false, a decision is missing, or its verify cannot pass), stop at that task, keep the earlier commits, and report it; do not skip ahead to tasks that depend on it.

## Done when

- Every task's verify command passes (this item's `[[verify]]` list, one entry per task), after the coordinator's batched gate.
- Each task's own "Done when" holds (see its file).

## Notes

- Waits on: PK74 (gap-ce1d11), PK75 (gap-3c3729), PK76 (gap-99c9ae).
- Suggested model: opus.

## Progress

- 9136: implemented at bf61230c2
- 9137: implemented at 17812b8b9
- 9138: implemented at 37b6b0cab

The Rust tasks were checked statically (verify greps, hand formatting, the route inventory check); cargo verification is left to the batch gate. Notes: 9136 serializes copy-backs with a process-wide lock plus the base-manifest check (exclusive_files is still a hint plan run does not act on), and a plan held for approval refuses a scratch_dir acceptance; 9137 makes confirm a built rung kind (the unbuilt-kind test now uses receipt), labels the step confirmed_by_user in the attempt record, and still counts a yes as Passed in the task verdict; 9138 adds a values-free argument summary to the effects report.
