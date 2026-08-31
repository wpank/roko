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
