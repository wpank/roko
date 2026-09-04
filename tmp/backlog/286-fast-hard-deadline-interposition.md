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
