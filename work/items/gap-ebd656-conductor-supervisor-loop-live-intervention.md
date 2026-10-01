+++
id = "gap-ebd656"
kind = "gap"
title = "Conductor Supervisor Loop (Live Intervention)"
status = "open"
triage = "verified"
severity = "p1"
size = "L"
goal = "features"
hold = "Set aside per tldr/05 §3; Will chose hold over park on 2026-09-29 (dec-e70592). Remove this line to revive."
subsystem = ["roko-cli/runner"]
created = 2026-09-07
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "a17d9d766"
source = "tmp/backlog/archive/178-conductor-supervisor-loop.md#178 — Conductor Supervisor Loop (Live Intervention)"
discovered_from = "audit:tmp/backlog/archive/178-conductor-supervisor-loop.md#178 — Conductor Supervisor Loop (Live Intervention)"
anchors = ["crates/roko-core/src/conductor.rs::ConductorDecision", "crates/roko-conductor/src/interventions.rs::WorstSeverityPolicy", "crates/roko-conductor/src/conductor.rs::Conductor::record_intervention_outcome", "crates/roko-cli/src/graph_execution/feedback.rs::ConductorSink", "crates/roko-cli/src/graph_execution/control_adapter.rs", "crates/roko-cli/src/tui/widgets/conductor_panel.rs::build_conductor_snapshot", "crates/roko-core/src/config/schema.rs::ConductorConfig"]
links = { depends_on = ["spec-a0403b"], blocks = [], related = ["gap-fab31c"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn conductor_nudge_reaches_silent_task' crates/roko-cli/ && cargo test -p roko-cli conductor_nudge_reaches_silent_task && grep -rqw 'fn conductor_force_advance_skips_stalled_task' crates/roko-cli/ && cargo test -p roko-cli conductor_force_advance_skips_stalled_task"
+++

## Problem

During `roko plan run`, nothing watches running agents and steps in. An agent that goes silent, loops on
compile failures or stalls on one task runs until its raw `timeout_secs` expires, burning tokens and time. The
conductor pieces exist but are not connected to Graph runs:

- the 12 watchers;
- the `InterventionPolicy`;
- the circuit breaker;
- the `Nudge` / `ForceAdvance` decisions;
- the `[conductor]` thresholds;
- the TUI conductor panel.

Mori ran a supervision loop every 2 s that turned watcher signals into typed interventions: nudge, restart,
force-advance, skip reviews. Its thresholds were silence 180 s, compile fails 3, task stall 300 s, context
pressure 80 %, phase timeout 1800 s.

Expected: while a Graph plan runs, the conductor evaluates task activity at an interval and executes finer
interventions than restart/fail. The actions:

- nudge a silent agent;
- restart a looping attempt;
- move past a stalled task in a controlled way;
- fail the run when the circuit breaker trips.

Each action is logged as a structured event, shown in the TUI, and its outcome is fed back for threshold
learning.

## Why it matters

Goal `features`. It is the behavioural safety net above raw timeouts, and it matters most for long unattended
self-hosting runs.

- Depends on `spec-a0403b` (p0, core: Graph engine watchdog integration). That item feeds Graph task events
  into the conductor ring and runs a supervision ticker beside the Graph run. This item adds what the ticker
  should do with the decisions.
- Related: `gap-fab31c` (parked: the old supervisor's actions only logged; "route conductor actions to the
  Graph execution control adapter, with a test for each action").

## Where

- `crates/roko-core/src/conductor.rs::ConductorDecision` (:91): `Continue`, `Nudge { watcher, message,
  task_id }`, `ForceAdvance { watcher, reason, task_id }`, `Restart`, `Fail`. There are constructor helpers
  (`nudge`, `force_advance`).
- `crates/roko-conductor/src/conductor.rs`: `Conductor::from_config` (:240), `evaluate_full` (:371: circuit
  breaker, then watchers, then policy), `record_intervention_outcome` (:341).
- `crates/roko-conductor/src/interventions.rs`:
  - `WorstSeverityPolicy` maps severity Info → `Continue`, Warning → `Restart`, Critical → `Fail` (:45-48);
  - `BanditPolicy` maps `InjectHint` to `Continue` (:250).
  - Neither ever produces `Nudge` or `ForceAdvance`.
- `crates/roko-conductor/src/stuck_detection.rs::StuckThresholds` (:168): silence 180 s, compile fail 3,
  task stall 300 s, context pressure 0.80.
- `crates/roko-core/src/config/schema.rs::ConductorConfig` (:1689): `[conductor]` already has `watchers`,
  `silence_timeout_secs`, `compile_fail_threshold`, `task_stall_secs` and `context_pressure_pct` ("consumed
  by `Conductor::from_config`").
- `crates/roko-cli/src/graph_execution/feedback.rs::ConductorSink` (:602): the only conductor-named hook on the
  Graph path. It runs after settlement and only records provider success/failure in `ProviderHealthRegistry`.
- `crates/roko-cli/src/runner/conductor_adapter.rs` (`ConductorRing`, `ConductorRingSink`) and
  `crates/roko-cli/src/runner/types.rs` (`RunnerEvent::conductor_intervention` :1982; `Conductor::from_config`
  :2598): Runner-v2-era bridge code. The only production caller of `Conductor::from_config` is here, off the
  Graph path.
- `crates/roko-cli/src/execution_control.rs::ExecutionCommandKind` (:59): `Pause`, `Resume`, `SoftRetry`,
  `Repair`, `ReverifyGates`, `Skip`, `Cancel`, `Approve`, `RejectApproval`, `Reset`.
  `crates/roko-cli/src/graph_execution/control_adapter.rs` maps them onto `roko_graph::control` and
  coordinates `ProcessSupervisor` shutdown. This is where conductor actions should be routed.
- `crates/roko-cli/src/tui/widgets/conductor_panel.rs` (`build_conductor_snapshot` :121) and
  `tui/state/mod.rs::refresh_conductor_snapshot` (:3506). The panel shows watcher status from alert signals of
  kind `conductor:alert:<watcher>`, plus diagnoses and `[conductor]` thresholds. Nothing on the Graph path
  emits those alerts.
- `crates/roko-cli/src/graph_task_dispatch.rs`: per-attempt dispatch. `turn_cap_resume_note` is an existing
  example of appending a note to a retry prompt, the natural shape for a nudge.

Entry point: `roko plan run <dir>` → `graph_execution::plan_runner::run_graph_plan` → engine →
`GraphTaskDispatcher`.

## Current state

Checked at `a17d9d766`. Parts of backlog #178 landed:

- the `Nudge` / `ForceAdvance` variants (commit `69824f63c`);
- the `[conductor]` thresholds;
- the TUI conductor panel widget.

The live loop itself does not exist:

- The Runner-v2 supervision tick (`conductor_supervision_tick` in `runner/event_loop.rs`) was deleted with
  Runner-v2 (`6b5da8616`, 2026-09-06). Its actions only logged anyway (`gap-fab31c`).
- The Graph path constructs no `Conductor`, feeds no ring, and runs no tick (`spec-a0403b`).
- No policy emits `Nudge` or `ForceAdvance`.
- `record_intervention_outcome` has no callers.
- The TUI panel gets no alerts from Graph runs.

## Plan

Do this after `spec-a0403b` has the ring feed and ticker in place.

1. **Emit the finer decisions.** In `roko-conductor`, map watcher outputs to decisions by watcher and history,
   not only severity:
   - silence (`silence_timeout_secs`) or compile-fail threshold → `Nudge` for the task;
   - the same task still stalled after a nudge (`task_stall_secs`) → `Restart` of that attempt;
   - still stalled after the restart → `ForceAdvance`;
   - circuit breaker → `Fail`.

   Keep `WorstSeverityPolicy` as the fallback. Add unit tests per mapping.
2. **Execute the decisions through Graph control, not ad-hoc.** In the ticker from `spec-a0403b`:
   - `Nudge`: kill the task's current attempt through `ProcessSupervisor`, then retry with the nudge message
     appended to the prompt (as `turn_cap_resume_note` does). Record the reason "conductor nudge". CLI
     providers run in `--print` mode and cannot take input mid-run. For API or ACP tool-loop providers,
     injecting into the next turn is a later option.
   - `Restart`: `SoftRetry` of that task's attempt.
   - `ForceAdvance`: `Skip` of that task via `control_adapter.rs`. Dependents then follow the engine's
     skip rules.
   - `Fail`: `Cancel` the plan.
3. **Do not mark a stalled task completed.** The original spec wanted ForceAdvance to mark the task
   `Completed` with an override annotation. That breaks two invariants:
   - "authored verify steps … never force-accepted" (`settle_task_verification` doc);
   - checkpoint replay requires a passing gate verdict for verify-bearing tasks
     (`graph_checkpoint.rs::verdict_required_nodes`).

   Use an explicit skip or fail. If an operator wants a completed state, that is an approval
   (`Approve`/`RejectApproval`), not an automatic action.
4. **Surface it.**
   - Publish each intervention as a dashboard event (watcher, decision, task, reason) through the run's
     `state_hub`.
   - Emit a `conductor:alert:<watcher>` signal so `build_conductor_snapshot` shows watcher health.
   - Write a line to the plan's `activities.jsonl`.
5. **Learn from it.** After the intervened attempt settles, call `Conductor::record_intervention_outcome`
   with success or failure. `ConductorSink` in `feedback.rs` is the natural place, keyed by attempt id.
6. **Tests** with a fake provider that stays silent, driving a short `silence_timeout_secs`:
   - `conductor_nudge_reaches_silent_task`: attempt killed, retried with the nudge text, event published;
   - `conductor_force_advance_skips_stalled_task`: after nudge and restart, the task is skipped (not
     completed), dependents are handled, and the event is published.

## Done when

- In a Graph run with a silent fake agent and short thresholds, the conductor nudges, then restarts, then
  skips the task. Each step appears as a structured event in the run's event stream and in the TUI conductor
  panel.
- No conductor action marks a verify-bearing task completed without a passing verdict.
- Intervention outcomes reach `record_intervention_outcome`.
- `[conductor]` thresholds in `roko.toml` change when the interventions fire.
- The suggested verify passes:
  `grep -rqw 'fn conductor_nudge_reaches_silent_task' crates/roko-cli/ && cargo test -p roko-cli conductor_nudge_reaches_silent_task && grep -rqw 'fn conductor_force_advance_skips_stalled_task' crates/roko-cli/ && cargo test -p roko-cli conductor_force_advance_skips_stalled_task`.
  The current `[[verify]]` (the Graph path calls `Conductor::from_config` or `evaluate_full`) passes as soon
  as `spec-a0403b` lands, even if no intervention is executed.

## Notes

- Hard dependency: `spec-a0403b`. Do not build a second ticker here. Extend the one it adds.
- Killing agents and skipping tasks are control-plane actions. Route them through `control_adapter.rs` /
  `ExecutionControlService` so receipts, run-ID checks and `ProcessSupervisor` shutdown apply. Never signal
  processes directly.
- Keep interventions off by default, or behind a `[conductor]` switch, until the tests and one live run show
  they do not kill healthy long-running tasks (for example a slow `cargo test`). Silence detection has to
  count gate and verify activity as liveness, not only agent output.
- `runner/conductor_adapter.rs` may be reused for the ring, but it lives under the Runner-v2-era `runner/`
  module. Move what is needed rather than adding Graph dependencies on `runner::types`.
- Parallel safety: touches `graph_task_dispatch.rs`, `graph_execution/feedback.rs`, `control_adapter.rs` and
  `roko-conductor`. It conflicts with `spec-a0403b` if both run at the same time; do them in sequence.

## Original notes

without a live intervention loop, stuck agents burn tokens indefinitely; the conductor ring buffer and 12 watchers exist but nothing reads or acts on their signals during plan execution. Mori's conductor ran a live supervision loop: every 2 seconds, the conductor evaluated signal data from…

Imported without verification from:
- `tmp/backlog/archive/178-conductor-supervisor-loop.md#178 — Conductor Supervisor Loop (Live Intervention)`
- `tmp/backlog/_archive/_mori-old-gaps.md#MO-08`
- `tmp/archive/CONSOLIDATED-BACKLOG-2026-09-23.md#UXP-10 (UX/TUI Parity: Partial Items (13 items f)`
- `tmp/archive/MASTER-ACTION-PLAN-2026-09-23.md#4.9 Parity items requiring runner/infras PX.5`

Some cited files are gone: `crates/roko-cli/src/runner/event_loop.rs`.

How to verify: Check: Conductor supervision tick reads signals from the ring buffer every interval and produces typed interventions beyond Restart/Fail.; A stalled agent (no output for `silence_timeout_secs`) triggers a Nudge or Restart depending on severity.; A… [evidence: CONSOLIDATED UXP-10: Tick+thresholds wired; actions only log; 00-STATUS-SUMMARY 3. Open / P1 -- High (Open): M | 3 |]

Verified 2026-09-28: still true, worse than recorded - the Runner-v2 supervision tick (whose actions only logged, gap-fab31c) was deleted and the Graph engine has no conductor tick at all (see spec-a0403b); the only Graph-side conductor hook is the post-settlement ConductorSink (graph_execution/feedback.rs:602).
- 2026-10-01 (coordinator): gap-1a7f9c wires Graph gate verdicts to the conductor's watchers. With only Continue/Restart/Fail, a gate-driven Restart cancels the task's next attempt within 5 s: it can end a stuck task early under max_retries and kill an attempt the ladder escalated, and learn_restart_outcome scores the restart on provider success, not on verify (wk-honestbench). So gap-1a7f9c lands gate-driven Restarts as advisory only on the Graph path. This item should supply the real reaction (Nudge with the repeated diagnostic, or escalation), switch those decisions back on, and score restarts on verify.
