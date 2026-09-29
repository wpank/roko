+++
id = "spec-3cb55d"
kind = "spec"
title = "FAST hard-deadline interposition during dispatch startup: kill-point matrix, written for the deleted Runner-v2"
status = "open"
triage = "verified"
severity = "p2"
size = "M"
subsystem = ["roko-cli/fast"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "a17d9d766"
source = "tmp/backlog/286-fast-hard-deadline-interposition.md"
anchors = ["crates/roko-cli/src/graph_execution/fast_lane.rs::arm_plan_deadline", "crates/roko-cli/src/graph_execution/plan_runner.rs::terminate_in_flight_agents", "crates/roko-cli/src/graph_execution/plan_runner.rs::kill_in_flight_agents", "crates/roko-graph/src/engine.rs::FlowHandle::cancel", "crates/roko-cli/src/graph_task_dispatch.rs::GraphTaskDispatcher", "crates/roko-cli/src/runner/agent_stream.rs::AgentStartupControl", "crates/roko-cli/src/dispatch/mod.rs::spawn_streaming_cli_agent_controlled"]
goal = "core"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn dispatch_after_cancel_launches_no_provider' crates/roko-cli/ && cargo test -p roko-cli dispatch_after_cancel_launches_no_provider"
+++

## Problem

This spec (backlog #286) was written for the Runner-v2 event loop. That loop, its `DispatchDeadline`
(`runner/deadlines.rs`) and its attempt-ownership registry (`runner/attempt_ownership.rs`) were deleted in
`6b5da8616` on 2026-09-06, together with the 30+ kill-point unit tests the spec cites. None of its file or line
references exist any more.

The property it protected still matters on the Graph engine, which is now the only plan executor. Once the FAST
deadline or an interrupt (SIGINT, SIGTERM, closing the TUI) stops a run, no new paid provider may start, and every
agent process must be cleaned up before the run records its result. On the Graph path today:

- `graph_execution/fast_lane.rs::arm_plan_deadline` requests `PlanRunInterrupt::Terminate` when
  `ROKO_FAST_PLAN_DEADLINE_SECS` elapses.
- The plan watcher in `plan_runner.rs` (around lines 2095-2131) then does three things:
  1. calls `flow_handle.cancel()`;
  2. sends SIGTERM once to the registered agent process trees (`terminate_in_flight_agents`);
  3. after `INTERRUPT_DRAIN_TIMEOUT` (3 s), sends SIGKILL (`kill_in_flight_agents`), gives up on the graph and
     finalizes the checkpoint as interrupted. Exit code 143.
- The Graph engine checks cancellation only between nodes and between waves (`roko-graph/src/engine.rs` around
  lines 1818 and 2154). It never interrupts a node that is already running.
- `GraphTaskDispatcher::dispatch` (`graph_task_dispatch.rs`) contains no cancellation, interrupt or deadline check.

Result: a task that is still preparing when the run is stopped keeps going. It can still launch a paid provider.
The steps it may still be in are worktree acquisition, routing, prompt assembly, retrieval and hooks. If the
launch lands after the single SIGTERM sweep, the process is only caught by the SIGKILL sweep 3 s later, or by
`kill_on_drop` when the runtime shuts down. `kill_on_drop` kills the direct child only, not its process group.
No test covers any of this.

## Why it matters

- Goal `core`. An interrupted run must not spend money or leave agent processes behind. This affects every
  interrupt, not only FAST.
- FAST (`./dev.sh fast`) depends on the runner stopping cleanly inside
  `ROKO_FAST_PLAN_DEADLINE_SECS = deadline - settlement headroom` (`dev.sh` lines 286-331). Otherwise the outer
  wrapper kills the process group and the evidence bundle loses the runner's own result.
- Related:
  - `gap-4a6dcb`: other FAST features not ported to the Graph path (patch-only prompt, 90 s clamp, 6-turn cap,
    `dev-fast` gate profile, no-autofix). This deadline/cancellation property is not on that list.
  - `bug-4641e3`: forced exit and SIGHUP.
  - `gap-7c9e48`: parallel plans, which mean more tasks can be in flight at the moment of an interrupt.

## Where

- `crates/roko-cli/src/graph_execution/fast_lane.rs::arm_plan_deadline`: the FAST run timer. It is called at
  `plan_runner.rs` line ~802.
- `crates/roko-cli/src/graph_execution/plan_runner.rs`:
  - `terminate_in_flight_agents` and `kill_in_flight_agents` (lines ~355-370);
  - `live_agent_process_trees` (line ~328), which uses `roko_agent::process::registered_pids`;
  - `INTERRUPT_DRAIN_TIMEOUT` (line ~200);
  - the per-plan watch loop (lines ~2095-2131).
- `crates/roko-graph/src/engine.rs`: `FlowHandle::cancel` (line ~259) and the between-node and between-wave
  `cancel.is_cancelled()` checks.
- `crates/roko-cli/src/graph_task_dispatch.rs`:
  - `GraphTaskDispatcher` and its `TaskDispatcher::dispatch` implementation. This is where preparation happens:
    worktree, `build_routing_context`, enrichment, prompt.
  - Around lines 3626-3640, the dispatcher builds `AgentDispatchRequest` with `timeout_ms`. That request is the
    paid provider boundary.
- `crates/roko-cli/src/runner/agent_stream.rs::{AgentStartupControl, spawn_agent_controlled, interrupt_startup_child}`
  and `crates/roko-cli/src/dispatch/mod.rs::spawn_streaming_cli_agent_controlled`: the Runner-v2 startup-deadline
  machinery. It survived the deletion but has no callers at HEAD (checked with grep).
- `crates/roko-agent/src/claude_cli_agent.rs` (line ~413, `kill_on_drop(true)`): how the Graph path's CLI agents
  are spawned.

## Current state

- Done on the Graph path in `725f21e05`:
  - a run-level FAST deadline;
  - graph cancel, a SIGTERM sweep, a 3 s drain, a SIGKILL sweep;
  - the checkpoint finalized as `interrupted`;
  - exit code 143.
- The unit tests for this are in `plan_runner.rs`: `interrupt_exit_codes_follow_shell_convention`,
  `first_interrupt_request_wins`, `checkpoint_status_is_honest_about_how_a_plan_ended` and
  `closing_the_tui_mid_run_interrupts_the_run`.
- Not done:
  - Running dispatches are not cancelled.
  - There is no deadline check right before the paid launch.
  - No test runs a plan whose agent hangs during startup or preparation.
  - Nothing proves an interrupted run leaves no agent processes behind.
  - Nothing proves that `--resume-plan` after an interrupt neither duplicates a provider launch nor a gate.

## Plan

1. Give each dispatch the run's stop signal.
   - Check whether `roko-graph` passes its `CancellationToken` into the cell or dispatch context. If it does not,
     add a `tokio_util::sync::CancellationToken` to the run context.
   - Cancel it wherever `flow_handle.cancel()` is called, and hand it to `GraphTaskDispatcher::dispatch`.
2. In `dispatch`:
   - Check the token at entry.
   - Race each long awaited preparation step (worktree acquisition, retrieval, hooks, enrichment) against it with
     `tokio::select!`.
   - Check it again immediately before sending the `AgentDispatchRequest`.
   - When cancelled, release the task worktree and return a cancelled outcome. Do not write efficiency, episode or
     routing-feedback records, and do not increment attempt counters, for a provider that never started.
3. Close the late-launch window. After cancellation, also SIGTERM any agent that registers during the drain
   window: either sweep again before the SIGKILL, or re-sweep at the end of the drain.
4. `AgentStartupControl`, `spawn_agent_controlled` and `spawn_streaming_cli_agent_controlled` have no callers.
   - Recommended: delete them. The Graph path spawns CLI agents through `roko-agent` with `kill_on_drop`, and
     step 2's token covers the startup window.
   - Alternative: wire them in, if the `roko-agent` spawn cannot be raced against the token.
5. Tests: the kill-point matrix, redone for the Graph path.
   - Unit, `dispatch_after_cancel_launches_no_provider`: a `GraphTaskDispatcher` with a counting mock provider and
     a cancelled token returns cancelled, and the provider count is 0.
   - Integration, using the mock-agent harness in `crates/roko-cli/tests/graph_plan_callers.rs`: the mock agent
     writes its pid and sleeps 600 s. With `ROKO_FAST_MODE=1 ROKO_FAST_PLAN_DEADLINE_SECS=3`, assert that the run
     exits 143 within about 15 s, that the checkpoint is `interrupted`, and that the pid is gone.
   - Resume: run `plan run --resume-plan` after that interrupt. Tasks that completed are not dispatched again, and
     the interrupted task is dispatched exactly once.

## Done when

- A dispatch that is still preparing when the run is interrupted launches no provider and leaves no worktree
  lock behind.
- A FAST run whose agent hangs in startup exits 143 inside `ROKO_FAST_PLAN_DEADLINE_SECS` plus the drain, with
  the checkpoint `interrupted` and no surviving agent process.
- Resume after an interrupt neither duplicates a completed task nor duplicates a gate.
- The unused Runner-v2 startup-control code is either deleted or wired in.
- Verify:
  `grep -rqw 'fn dispatch_after_cancel_launches_no_provider' crates/roko-cli/ && cargo test -p roko-cli dispatch_after_cancel_launches_no_provider`

## Notes

- Do not bring back Runner-v2 types (`DispatchDeadline`, `AttemptOwnership`). The Graph engine and its checkpoint
  are the single owner now. Keep the fix to the dispatcher and the plan runner.
- Risky area: process cleanup and checkpoint finalization. Keep the existing interrupt exit codes (130/143) and
  the `interrupted` checkpoint status. Tests assert both.
- The integration test must isolate `HOME` and shadow `claude`/`codex`/`gemini` with the mock, as
  `graph_plan_callers.rs::run_roko` does, so no real model is called.
- Coordinate with `gap-4a6dcb` (FAST features) and `bug-4641e3` (forced exit/SIGHUP): they touch the same
  interrupt paths in `plan_runner.rs`. Not safe to run in parallel with other edits to the plan watch loop.
- The original spec's checklist, rewritten for the Graph path, is step 5. Its "timeout-diff salvage" (letting an
  edited, timed-out worktree go to the gate) has no Graph equivalent. Leave it out unless someone asks for it.

## Original notes

Imported 2026-09-29 from `tmp/backlog/286-fast-hard-deadline-interposition.md` (tmp/backlog is frozen). Not yet checked against current code: the text below is the spec as last written, including its own status notes.

## Original spec

# 286 — FAST Hard-Deadline Interposition During Dispatch Startup

> **Status: SOURCE-IMPLEMENTED / FINAL KILL-POINT MATRIX PENDING** (2026-08-31,
> `52d5f4df4` + `43a48ee26`). FAST scheduling is wake-driven; its
> non-resetting deadline interposes awaited preparation and CLI/bridge startup; Restart/Fail
> pre-cancel settlement is bounded and fail-closed; safe timeout diffs enter ordinary safety/gate
> ownership under a durable content fingerprint; terminal projections preserve degraded cleanup
> truth. Fixed-SHA benchmark automation is source-complete in `d1b94b139`; final
> hung-hook/startup/restart fixtures and representative repetitions remain pending. The integrated
> CLI build and 2,301-test library harness passed; neither substitutes for the open kill-point
> matrix below.
>
> **Status update (2026-09-01):** Kill-point matrix test fixtures added to
> `crates/roko-cli/src/runner/deadlines.rs` and `attempt_ownership.rs`: dispatch deadline expiry,
> preparation budget exhaustion, settlement headroom, FAST policy clamping, duplicate launch
> prevention, cancellation resource release, agent→gate lifecycle ownership, and full
> dispatching→agent→gate single-owner verification. The five verification checklist items now
> have focused unit tests.

**Status**: Verified (2026-09-03) — deadline interposition, kill escalation, ownership

> **Verification notes (2026-09-03):**
>
> **1. Dispatch deadline interposition across worktree/routing/hook/startup phases:**
> CONFIRMED. The `DispatchDeadline` struct (deadlines.rs:199) carries the non-resetting
> hard-run instant into every awaited dispatch operation. The interposition points are:
>
> - **Worktree preparation:** `ensure_attempt_workdir_controlled` (event_loop.rs:2150)
>   converts `DispatchDeadline` to a `tokio::time::Instant` and passes it to
>   `ensure_for_attempt_controlled` (worktree.rs:851), which races the worktree lock
>   acquisition against `cancel.cancelled()` and `await_optional_deadline(deadline)`.
>   Both `WorktreeOperationError::Cancelled` and `::Deadline` map to
>   `DispatchInterruption` (event_loop.rs:2179-2180).
>
> - **Disk budget, playbook matching, signal scoring, episode queries, pre-inference hooks:**
>   All wrapped in `await_dispatch_step` (event_loop.rs:10587) which races the future
>   against `cancel.cancelled()` and `tokio::time::sleep(remaining)`. Seven call sites
>   confirmed at lines 11338, 11403, 11507, 12293, 12316, 12380, 12542.
>
> - **Pre-launch re-check:** Immediately before the paid provider boundary
>   (event_loop.rs:12845-12867), the code explicitly re-checks
>   `dispatch_deadline.remaining(monotonic_now()).is_none()` and settles via
>   `settle_dispatch_interruption` if expired.
>
> - **CLI startup:** `checkpoint_dispatch_stage` records `DispatchStage::CliStartup`
>   (event_loop.rs:12947-12952). `startup_control` (event_loop.rs:10620) converts
>   the dispatch deadline into `AgentStartupControl` with a bounded deadline. If
>   `startup_control` returns `None` while a dispatch deadline exists, the path
>   immediately settles (event_loop.rs:12973-12983). The controlled spawn uses
>   `spawn_agent_controlled` with the startup control.
>
> - **Bridge startup:** `checkpoint_dispatch_stage` records `DispatchStage::BridgeStartup`
>   (event_loop.rs:13298-13303). Same `startup_control` pattern
>   (event_loop.rs:13318-13334). `spawn_shared_agent_bridge_controlled`
>   (factory.rs:389) races the oneshot `started_rx` against `cancel.cancelled()` and
>   `tokio::time::sleep_until(deadline)` (factory.rs:449-454).
>
> **2. Process cleanup on timeout:**
> CONFIRMED. Two cleanup mechanisms:
>
> - **CLI agent startup:** `interrupt_startup_child` (agent_stream.rs:102) calls
>   `kill_tree(child, control.cleanup_grace)` which implements a 3-step escalation:
>   close stdin, SIGTERM the process group, SIGKILL if still alive (kill.rs:33-86).
>   After kill_tree, `try_wait` confirms process death and `unregister_pid` cleans up
>   the global PID registry. `AgentStartupError::Interrupted` carries `cleanup_error`
>   and `unconfirmed` (the child handle) if cleanup failed, so the event loop can
>   retain the handle for a later cancellation retry via
>   `restore_cancellation_failure` (event_loop.rs:13089-13121).
>
> - **Bridge agent startup:** `handle.abort()` followed by `(&mut handle).await`
>   (factory.rs:456-457) terminates the spawned tokio task.
>
> - **Dispatch settlement:** `settle_dispatch_interruption` (event_loop.rs:10660)
>   calls `cancel_exact_attempt` with the `Dispatching` phase owner, which claims
>   cancellation, replaces the resource, handles `CleanupFailed` recovery, and
>   calls `task_capacity.wake()` to release the capacity permit.
>
> **3. No provider duplication on timeout:**
> CONFIRMED. Three mechanisms prevent duplicate provider launches:
>
> - **Ownership registry:** `AttemptOwnership::insert` returns `Err(Occupied)` if the
>   same attempt key already exists (attempt_ownership.rs:1412-1437). This prevents
>   a second provider launch during preparation.
>
> - **Phase transition:** `transition_claim` from `Dispatching` to `Agent` makes the
>   old `Dispatching` phase ineligible for further events
>   (attempt_ownership.rs:1602-1643). A restart replay cannot steal an active slot
>   (attempt_ownership.rs:1686-1719).
>
> - **Single-owner lifecycle:** The full `Dispatching(Preparation) -> CliStartup ->
>   BridgeStartup -> Agent -> AwaitingGate -> Gate` path maintains exactly one
>   eligible phase per step (attempt_ownership.rs:1722-1810, 1999-2052).
>
> **Test coverage:** The kill-point matrix has 30+ focused unit tests across
> `deadlines::tests` (items 1, 3, 5) and `attempt_ownership::tests` (items 2, 4).
> Key test names: `dispatch_deadline_remaining_returns_none_when_expired`,
> `hard_run_deadline_prevents_provider_launch_when_preparation_consumes_budget`,
> `dispatching_claim_blocks_duplicate_launch_during_preparation`,
> `dispatching_claim_releases_resource_on_cancellation`,
> `transition_from_dispatching_to_agent_prevents_duplicate_launch`,
> `full_lifecycle_dispatching_through_gate_has_exactly_one_owner_at_each_step`,
> `hard_run_deadline_leaves_deterministic_settlement_headroom`,
> `fast_mode_policy_clamps_without_weakening_gate_effects`
**Priority**: P1 — a slow awaited dispatch-preparation or provider-startup path can outlive the
FAST execution budget before the event loop gets another chance to settle the run
**Size**: M (2–3 days)
**Wave**: 3
**Crates**: `roko-cli`
**Depends on**: the attempt-ownership admission work landed in `a58bdbacb`
**Source**: `tmp/dev-audit/04-roko-self-hosting.md`, `tmp/dev-audit/10-p0-implementation.md`

## Background

The opt-in FAST lane now reserves one exact attempt before expensive preparation, resets the paid
attempt clock when a runtime actually launches, and gives the outer evidence wrapper settlement
headroom. That closes duplicate preparation and prevents preparation time from silently consuming
the provider budget.

At the P0 checkpoint, one lifecycle gap remained. `dispatch_action(...).await` owned the event-loop branch while it created
or loads a worktree, selects a route, assembles a prompt, runs hooks, and starts a provider. The
normal deadline checks cannot interpose until that awaited call returns. A hung provider startup
can therefore cross the internal FAST deadline before the event loop durably records the terminal
outcome. The outer wrapper will eventually kill the process group, but it is a containment layer,
not a substitute for runner-owned durable settlement.

The P0 implementation intentionally did not add timeout-diff salvage or a new nonterminal timeout
state without a typed lifecycle shared by cancellation audit, restart replay, TUI state, and gate
ownership. The expanded implementation recorded at the top of this item closes that source gap;
the checklist below keeps the final kill-point evidence separate.

## Implementation Plan

- [x] Split dispatch preparation and runtime startup into explicit deadline/cancellation phases with typed
   effects and checkpointed ownership.
- [x] Interpose the non-resetting global deadline across worktree preparation, routing, prompt hooks,
   CLI startup, and shared-agent bridge creation.
- [x] Re-check the deadline immediately before every paid provider launch. If expired, release or
   settle the exact `Dispatching` owner without incrementing launch counters.
- [x] Add bounded timeouts around provider process/bridge startup and bounded process-tree cleanup
   before terminal settlement.
- [x] Define a durable timeout-salvage gate transition before allowing an edited timed-out worktree to
   enter verification. Resume must neither duplicate the provider launch nor lose the gate.
- [x] Emit separately attributable preparation, startup, provider, gate, and settlement evidence.

## Acceptance Criteria

- [x] Deadline interposition prevents a hung worktree/prompt hook from launching a provider after the FAST global
   deadline.
- [x] CLI/bridge startup has a bounded cancel/reap/settle source path before the outer wrapper
  deadline; a final hung-startup fixture is pending.
- [x] Timeout/cancellation paths bound capacity/ownership settlement and do not increment a provider
  that never launches; unconfirmed survivors remain degraded rather than being cleared.
- [x] Restart recovery is idempotent by durable ownership/fingerprint; the exhaustive kill-point
  matrix is still pending.
- [x] Timeout-diff salvage uses an explicit checkpointed lifecycle transition and
   the ordinary safety/gate ownership path.

## Verification Checklist

- [ ] Fake a preparation hook that never resolves; assert no provider launch and one terminal.
- [ ] Fake CLI and bridge startup hangs; assert process-tree cleanup and capacity release.
- [ ] Advance the plan clock while an exact attempt is retained; assert attribution remains exact.
- [ ] Kill/restart at each new checkpoint; assert no duplicated launch or gate.
- [ ] Confirm the outer wrapper still has settlement headroom and reports the durable runner result.

## Files to Modify

| File | Change |
|---|---|
| `crates/roko-cli/src/runner/event_loop.rs` | Split cancellable preparation/startup and settle exact expiry |
| `crates/roko-cli/src/runner/deadlines.rs` | Represent preparation/startup/global deadline ownership |
| `crates/roko-cli/src/runner/attempt_ownership.rs` | Add checkpoint-safe typed transition if required |
| `crates/roko-cli/src/runner/agent_stream.rs` | Bound CLI startup and prove process cleanup |
| `crates/roko-cli/src/dispatch/factory.rs` / `dispatch_v2.rs` | Bound shared-agent bridge startup |
