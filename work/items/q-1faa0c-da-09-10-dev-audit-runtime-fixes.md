+++
id = "q-1faa0c"
kind = "question"
title = "Dev-audit runtime fixes unverified on the Graph engine after Runner-v2 deletion"
status = "open"
triage = "verified"
severity = "p1"
size = "L"
goal = "core"
subsystem = ["roko-cli/graph_execution"]
created = 2026-09-14
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "a17d9d766"
source = "tmp/dev-audit/09-additional-live-run-findings.md#Timeout loses provider usage and cost"
discovered_from = "audit:tmp/dev-audit/09-additional-live-run-findings.md#Timeout loses provider usage and cost"
anchors = ["crates/roko-agent/src/claude_cli_agent.rs::ClaudeCliAgent::failure", "crates/roko-agent/src/exec.rs:643", "crates/roko-cli/src/graph_task_dispatch.rs:3796", "crates/roko-cli/src/graph_execution/plan_runner.rs::run_graph_plan", "crates/roko-cli/src/graph_execution/event_log.rs::run_recorded", "crates/roko-cli/src/graph_checkpoint.rs::GraphCheckpointStatus", "crates/roko-cli/src/graph_execution/fast_lane.rs::arm_plan_deadline"]
links = { depends_on = ["bug-690dc6"], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "test -f crates/roko-cli/tests/graph_timeout_matrix.rs && cargo test -p roko-cli --test graph_timeout_matrix -- --include-ignored"
+++

## Problem

A 2026-08-31 live run (dev-audit, run `run-1788163153495`) found five runtime defects that were then
fixed inside Runner-v2's `runner/event_loop.rs`. That file was deleted on 2026-09-06 (`6b5da8616`), so
the fixes went with it, and nobody has checked which of them the Graph engine (the only plan executor)
provides. The question this item answers: for each fix, does a Graph `roko plan run` behave correctly,
and is there a test that proves it?

The five behaviours, and the answer at HEAD (`d9e79e9d8`, static reading):

| # | Behaviour required | Graph path at HEAD |
|---|---|---|
| 1 | A provider that emits usage and then times out keeps its tokens, model, provider and cost | **Missing.** Claude CLI and Codex timeouts kill the process and return a failure built from `StreamUsage::default()`: zero tokens, no model tag. The Graph dispatcher records $0. Owner: `bug-690dc6` |
| 2 | Terminal projections agree (event, checkpoint, status, PID registry, ledger, exit code) and finalization is idempotent | **Unproven.** Graph writes different projections from Runner-v2 (see Current state); nothing tests that they agree after success, failure, timeout, cancel and resume |
| 3 | Settlement at shutdown is bounded | **Partly done** in `725f21e05`: after an interrupt the graph gets `INTERRUPT_DRAIN_TIMEOUT` (3 s), then agents are SIGKILLed and the checkpoint is finalized `interrupted`; `FORCED_EXIT_GRACE` 10 s. No test covers an agent that ignores SIGTERM |
| 4 | A timed-out attempt's diff is still gated ("timeout salvage") | **Missing.** A failed or timed-out dispatch returns an error before any verify step runs |
| 5 | FAST deadlines | **Run deadline done** (`graph_execution/fast_lane.rs`, `725f21e05`); per-attempt clamps tracked in `gap-4a6dcb` |

## Why it matters

Goal `core`. Timeouts are the most expensive failures, and today they look free in cost reports,
budgets (`bug-ae28ac`, per-task caps) and learning. Operators and `roko status`/dashboard read the
terminal projections; if they disagree, a failed run can show as running or a live agent can be orphaned.
Related: `bug-690dc6` (timeout usage at $0), `gap-4a6dcb` (FAST per-attempt clamps), `bug-230de6`
(`default_engine_does_real_work` still `#[ignore]`d), `gap-6ca8fb` (parked kill-point harness),
`gap-d0f3ee` (dead Runner-v2 PID helpers).

## Where

- `crates/roko-agent/src/claude_cli_agent.rs` about lines 1025-1037: timeout branch, `kill_tree` then
  `self.failure(..)` (line 265: zero `StreamUsage`); the stdout reader is never awaited on this branch.
  The success/turn-cap path parses `parse_stream_usage(&stdout)` and calls `failure_with_stream_usage`.
- `crates/roko-agent/src/exec.rs` about lines 643-650: the same pattern for Codex (`failure_signal`).
- `crates/roko-cli/src/graph_task_dispatch.rs` about lines 3796-3866: `if !dispatch.result.success`
  emits feedback, releases the worktree with `RetainForFailure`, returns `RokoError::Agent`; no verify.
- `crates/roko-cli/src/graph_execution/plan_runner.rs`: `run_graph_plan` (line 701; with `--log-file` it
  delegates to `event_log::run_recorded`), `INTERRUPT_DRAIN_TIMEOUT` / `FORCED_EXIT_GRACE` (lines
  198-203), watch loop (about line 2080), `kill_in_flight_agents` (line 367).
- `crates/roko-cli/src/graph_checkpoint.rs::GraphCheckpointStatus` (`Running`, `Succeeded`, `Failed`,
  `Cancelled`, `Interrupted`): the durable terminal state under `.roko/state/graph/<plan>/`.
- `crates/roko-cli/src/graph_execution/event_log.rs::run_recorded`: `--log-file` JSONL with
  `run.completed { outcome, exit_code }`.
- `crates/roko-agent/src/process/registry.rs`: PID registry (per-owner files since `725f21e05`).
- `crates/roko-cli/tests/graph_plan_callers.rs`: harness (real `roko` binary, mock agent script shadowing
  `claude`/`codex`/`gemini` on `PATH`) to copy for the matrix.

## Current state

- Graph projections are `checkpoint.json`, `activities.jsonl`, `costs.json` (per plan), the `--log-file`
  JSONL, `.roko/learn/costs.jsonl` and `efficiency.jsonl`, the PID registry, and the process exit code
  (SIGINT 130, SIGTERM 143). Runner-v2's `state-snapshot.json`, `status.json` and `run-ledger.jsonl` are
  not written by Graph runs; resume maps the old snapshot paths to the Graph checkpoint root
  (`graph_checkpoint.rs` `DEFAULT_RUNNER_RESUME_PATHS`).
- No `crates/roko-cli/tests/graph_timeout_matrix.rs`. Nearby tests: `graph_budget_resume.rs`,
  `resume_cycle_e2e.rs`, `runner_crash_recovery.rs` (the last targets Runner-era code).

## Plan

1. Land `bug-690dc6` first (usage survives timeout): on the timeout branch, after `kill_tree`, await
   the stdout/stderr readers with a short bound, parse `parse_stream_usage`, and return
   `failure_with_stream_usage`; same in `exec.rs`. Mark the usage partial.
2. Write `crates/roko-cli/tests/graph_timeout_matrix.rs` with the `graph_plan_callers.rs` harness and a
   one-task plan per case, with `timeouts.agent_dispatch_secs` (or task `timeout_secs`) set to a few
   seconds:
   - `timeout_keeps_usage`: the mock prints a stream-json usage/assistant event, then sleeps past the
     timeout. Assert the task fails, and `.roko/learn/costs.jsonl` has non-zero tokens and the model.
   - `terminal_projections_agree`: success, verify failure, timeout, and SIGTERM mid-task. For each,
     the exit code, the `--log-file` `run.completed.outcome`, and `checkpoint.json` status agree, and no
     registered PID for the run is left alive.
   - `interrupt_settles_when_agent_ignores_sigterm`: the mock traps SIGTERM; after SIGTERM `roko` exits
     143 within `INTERRUPT_DRAIN_TIMEOUT + FORCED_EXIT_GRACE` plus slack, and the mock process is gone.
   - `resume_after_timeout_is_idempotent`: `--resume-plan` after a timeout does not duplicate cost rows
     or events of completed tasks.
3. Timeout salvage (#4), a design choice. (a) Drop it: the partial diff stays in the tree and the next
   attempt continues from it (as turn-cap retries already do, `turn_cap_resume_note`). (b) Port it: run
   authored verify on the timed-out attempt's diff and accept the task if it passes. Recommended: (a),
   plus a timeout resume note like the turn-cap one. It is simpler, and a timed-out agent has not said it
   is done; (b) risks accepting half-finished work that happens to compile. Record the choice here.
4. Answer the question in this item with the result of the matrix; file a bug per failing case.

## Done when

- Each of the five behaviours is either proven by a named test in `graph_timeout_matrix.rs` or
  explicitly dropped (salvage) with the reason recorded.
- A timed-out attempt shows non-zero tokens and its model in `.roko/learn/costs.jsonl`.
- Verify: `test -f crates/roko-cli/tests/graph_timeout_matrix.rs && cargo test -p roko-cli --test graph_timeout_matrix -- --include-ignored`

## Notes

- The matrix uses real subprocesses and signals: Unix only (`#![cfg(unix)]`), temp workspaces only,
  never the repo's own `.roko/`. Keep timeouts short so the test stays under a minute or two.
- The PID assertion must only consider PIDs owned by the test's workspace; the registry is keyed by
  workspace since `725f21e05`.
- `bug-690dc6` touches `roko-agent` provider code shared by every dispatch path (chat, serve, ACP).
- The test file is new, so it is safe to write in parallel; the fixes touch `graph_task_dispatch.rs`
  and `plan_runner.rs`, which many items edit.

## Original notes

Timeout usage/provider-identity preservation, idempotent terminal projections (snapshot/status/PID/ledger), bounded conductor settlement, timeout-diff gate salvage and FAST deadlines were built in Runner-v2 event_loop.rs; their kill-point/timeout matrix never ran and Graph parity is unproven (TD-...

Imported without verification from:
- `tmp/dev-audit/09-additional-live-run-findings.md#Timeout loses provider usage and cost`
- `tmp/dev-audit/09-additional-live-run-findings.md#Final persistence contradicts the terminal event`
- `tmp/dev-audit/11-implementation-status.md#Status Update (2026-09-01)`
- `tmp/docs-audit/07-TECH-DEBT.md#TD-08: Graph Engine / Runner-v2 Parity Gap`
- `tmp/dogfood/2026-09-19-session.md#Next Steps`
- `tmp/dogfood/2026-09-20-final-session.md#P2 (Medium)`

Some cited files are gone: `.roko/state/state-snapshot.json`.

How to verify: Graph-engine fixture: provider emits usage then hangs past deadline; verify usage/model/provider persisted and event/snapshot/status/PID/ledger agree.

Verified 2026-09-28: No Graph-path counterpart found: graph_task_dispatch.rs has no timeout usage/cost salvage, and nothing under graph_execution/ or graph_checkpoint.rs writes agent-pids.json or run-ledger.jsonl. The Runner-v2 implementations went with runner/event_loop.rs (6b5da8616), and the proposed Graph-engine timeout fixture does not exist. The question stays open.

Re-verified 2026-09-29 at d9e79e9d8: unchanged. There is still no Graph-path timeout usage salvage, no PID/ledger terminal projection, and no kill-point/timeout fixture. The 725f21e05 FAST deadline (graph_execution/fast_lane.rs) covers only the run deadline part of this list.
