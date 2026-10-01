+++
id = "gap-633184"
kind = "gap"
title = "Control events vs text on bounded paths: zero-silent-drop under backpressure not tested"
status = "done"
triage = "verified"
severity = "p2"
size = "M"
goal = "visibility"
subsystem = ["roko-core/transcript_store"]
created = 2026-09-14
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "d1e3c5681"
source = "tmp/archive/tool-audit-2026-09-21/10-FINDINGS-REGISTER.md#register"
discovered_from = "audit:tmp/archive/tool-audit-2026-09-21/10-FINDINGS-REGISTER.md#register"
anchors = ["crates/roko-cli/src/tui/app/channels.rs::drain_state_events", "crates/roko-runtime/src/state_hub.rs::StateHub::subscribe_events_from", "crates/roko-runtime/src/event_bus.rs::EventBus::replay_from", "crates/roko-core/src/transcript_store.rs::is_control_event", "crates/roko-core/src/transcript_store.rs::PriorityEventChannel"]
links = { depends_on = [], blocks = [], related = ["gap-9c8ac0", "gap-836ae9"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn control_events_survive_long_stream_backpressure' crates/roko-cli/ && cargo test -p roko-cli control_events_survive_long_stream_backpressure"

[closed]
at = 2026-10-01
at_ts = "2026-10-01T19:41:31Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
size = "M"
claimed_at = "2026-10-01T16:13:33Z"
forced = false
evidence = "Gate 6c on 4bacc9d88 and its fix-up 060b62813, merged as d1e3c5681 (crates and Cargo.lock identical to the gated tree): cargo check --workspace --tests, nightly fmt and clippy -D warnings clean on 14 crates; lib tests pass (roko-cli 3395, roko-agent 2269, roko-core 1977, roko-learn 1225, roko-serve 1010, roko-gate 697, roko-compose 562, roko-graph 487, roko-dreams 260, roko-execution 245, roko-neuro 239, roko-acp 211, roko-gateway 101); all eight canaries, golden_path_suite, secret_canary and C2 pass; roko-acp, merge_proof, plan_conversion, job_lifecycle and job_runner_integration tests pass; bin 445; Cargo.lock unchanged; route inventory tests and --check-snapshot pass. Implemented in this round; the item's 2026-10-01 note names the change and its test."
+++

## Problem

No test shows that control events survive a long, fast stream when the consumer falls behind. Here
"control events" means task, gate and run terminal events, tool start and finish, and errors. The
2026-09-21 tool audit release gate "Control events survive the long-stream/backpressure scenario
with zero silent drops" is still unchecked (finding T014, "control and text events share bounded
paths without a documented loss policy").

The only drop-policy code is `PriorityEventChannel` and the `TranscriptStore` eviction in
`crates/roko-core/src/transcript_store.rs`. Only their own unit tests use them. The live path that
a user actually watches (Graph dispatch to StateHub to TUI) does not go through them.

On that live path, a slow TUI can lose events. `drain_state_events`
(`crates/roko-cli/src/tui/app/channels.rs:245`) reads a `tokio::sync::broadcast` receiver. When the
receiver lags, broadcast discards the oldest events of every kind, control and text alike. The TUI
then prints `[stream lagged: N StateHub events; snapshot resynced]` and carries on. The count is
shown, but the lost events are never re-fetched.

## Why it matters

Goal `visibility` (Live visibility: serve, dashboard, portal). If a task-finished, gate-result or
tool-finished event is lost, the live view can show a task still running after it ended, or a tool
call with no result. Nothing today would catch that regression.

Related items:
- `gap-836ae9`: live and replayed transcript parity is unproven;
- `gap-aabeff`: the TUI still consumes string tails, and tool steps are carried as text inside
  `AgentOutput`;
- `gap-5f4852`: another unchecked release gate from the same audit.

`gap-9c8ac0` is listed in `links` but has no item file.

## Where

- `crates/roko-core/src/transcript_store.rs`:
  - `is_control_event` (:609) is the control/text split: `RunStarted`, `RunFinished`,
    `ToolStarted`, `ToolFinished`, `Error`, `ProviderChanged`, `SubagentStarted` and
    `SubagentFinished` are control; the text deltas are in `is_text_delta` (:599);
  - `PriorityEventChannel` (:498) and `drop_report` (:574) are unused outside tests.
- `crates/roko-cli/src/runner/tui_bridge.rs::TuiBridge` is how Graph dispatch publishes into the
  StateHub (`StateHubSender`).
- `crates/roko-runtime/src/state_hub.rs::StateHub`:
  - `publish` updates the `DashboardSnapshot` (a `watch` channel) and then emits into `EventBus`;
  - `subscribe_events_from(next_seq)` (:1034) atomically returns the retained replay plus a live
    receiver;
  - the default capacity is 1024 (:606).
- `crates/roko-runtime/src/event_bus.rs::EventBus` is a bounded ring plus a `tokio::sync::broadcast`
  channel. `Envelope::seq` (:66) is a monotonic sequence number, and `replay_from(after_seq)`
  (:289) returns retained events.
- `crates/roko-core/src/dashboard_snapshot.rs::DashboardEvent` (:77) lists the event types on this
  path. `PlanStarted`, `PlanCompleted`, `RunCompleted`, `TaskStarted`, `TaskCompleted`,
  `TaskPhaseChanged`, `AgentSpawned` and `GateResult` are control-like. `AgentOutput` and
  `GateOutputLine` are text. Tool steps travel as `roko.stream.v1` lines inside `AgentOutput`.
- `crates/roko-cli/src/tui/app/channels.rs::drain_state_events` (:245) is the consumer that lags.
  `crates/roko-cli/src/tui/app/tests.rs` has App test helpers.
- Entry point: `roko plan run <dir>` with the dashboard attached, or `roko dashboard`.

## Current state

- Store-level tests exist:
  - `eviction_drops_text_deltas_before_control_events` (transcript_store.rs:775);
  - `control_events_never_evicted` (:800);
  - `priority_channel_never_drops_control` (:979) and `priority_channel_drops_text_under_pressure`
    (:996).
- The provider-to-dispatcher leg blocks rather than drops.
  `crates/roko-agent/tests/streaming_contracts.rs:305` shows that a blocking mpsc channel neither
  drops nor reorders text deltas.
- The StateHub-to-TUI leg is lossy under lag, as described above. Plan, task and gate state recover
  because the TUI renders from the `DashboardSnapshot` (the "RC-1" comment in
  `drain_state_events`). Transcript lines do not recover, including the tool start and finish lines
  inside `AgentOutput`.
- The TUI does not remember the last `Envelope::seq` it saw, and it does not call
  `subscribe_events_from` after a lag.
- No test named `control_events_survive_long_stream_backpressure` exists yet.

## Plan

1. Write the scenario test first, in roko-cli. Suggested location: `crates/roko-cli/src/tui/app/tests.rs`,
   or a new `crates/roko-cli/tests/` file.
   - Build a `StateHub` with a small ring, for example `StateHub::new(8)`, and attach a TUI `App`
     subscription, or call the same drain logic directly.
   - Publish a long stream without draining: thousands of `AgentOutput` chunks with
     `TaskStarted`, `GateResult` and `TaskCompleted` events, plus tool start and finish
     `roko.stream.v1` lines, mixed in.
   - Drain in several rounds, the way the TUI does (`MAX_EVENTS = 256` per tick).
   - Assert three things. (a) Every control event was delivered, or is reflected in the final
     `DashboardSnapshot` (for example every task has its terminal status). (b) The number of
     events reported as dropped equals the number actually missing, so there are no silent drops.
     (c) No control event is missing from the delivered sequence.
2. If (c) fails, which is expected once the lag exceeds the ring, fix the consumer. There are two
   options:
   - (A) Lag recovery. Recommended, because it wires existing code. Keep the last seen
     `Envelope::seq` in the TUI subscription. On `TryRecvError::Lagged`, replace the subscription
     with `StateHub::subscribe_events_from(last_seq + 1)` and process its `replay` before `live`.
     Only events older than the ring are then really lost. Report their exact count, and rebuild
     task state from the snapshot.
   - (B) A priority lane. Publish control-like `DashboardEvent`s on a separate channel that is never
     dropped, in the manner of `PriorityEventChannel`. This is more invasive: it changes the
     StateHub API used by serve (SSE/WS) as well.
3. Tool steps inside `AgentOutput` cannot be told apart from text until `gap-aabeff` gives them a
   typed form. For now, cover them through the replay recovery in (A), and note the limit in the
   test.
4. Decide what happens to `PriorityEventChannel`: wire it in under (B), or leave it with a doc
   comment saying it is not on the live path. Do not delete it in this item.

## Done when

- The named test publishes a stream longer than the StateHub ring to a lagging consumer. It shows
  that no control-like event is lost without being counted, and that every task's terminal status
  reaches the TUI state.
- If a fix was needed, the TUI recovers missed events from the replay ring after a lag. The lag
  marker gives the exact number of events that could not be recovered.
- Verify (the grep is narrowed to roko-cli, so the guard cannot pass on a same-named test in
  another crate while `cargo test -p roko-cli` runs nothing):
  `grep -rqw 'fn control_events_survive_long_stream_backpressure' crates/roko-cli/ && cargo test -p roko-cli control_events_survive_long_stream_backpressure`

## Notes

- 2026-10-01 (wk-streams): implemented on work/gap-b35a57; cargo verification deferred to the batch check.
  - The silent drop was in the replay: `drain_state_events` ran `subscription.replay.drain(..).take(256)`, and a
    `Drain` removes everything when dropped, so a replay longer than one tick lost the rest uncounted. The new
    `channels.rs::take_state_events` drains at most one tick and leaves the remainder for the next.
  - Option (A) cannot recover lagged events: `EventBus::new` gives the replay ring the broadcast channel's capacity
    (and tokio rounds the channel up to a power of two), so the ring never holds an event the channel already
    dropped. The lag marker's count, now one per tick, is the exact number lost; plan, task and gate state come from
    the snapshot. `PriorityEventChannel` stays, with a doc note that it is not on this path (Plan step 4).
  - Test `control_events_survive_long_stream_backpressure` (`tui/app/tests.rs`) publishes 1,811 events of a
    three-task run through `TuiBridge`: replayed from a 4,096-event ring all arrive over several ticks; live past an
    8-event ring, delivered plus counted equals published, the App's marker gives that count, and every task shows
    `Done`.
- Serve's SSE and WebSocket streams also subscribe to the StateHub. Fix (A) changes only the TUI
  consumer. Do not change `EventBus` capacity defaults or the `StateHub::publish` ordering (the
  snapshot is updated before the broadcast) without checking `crates/roko-serve`.
- The test must be deterministic: no sleeps, and drive publish and drain by hand.
- This is safe in parallel with most items. Avoid running it at the same time as `gap-aabeff` or
  `gap-836ae9` if either is editing `tui/app/channels.rs`.

## Original notes

TranscriptStore eviction preserves control events (On main), but release gate 'Control events survive the long-stream/backpressure scenario with zero silent drops' remains unchecked (not yet tested).

Imported without verification from:
- `tmp/archive/tool-audit-2026-09-21/10-FINDINGS-REGISTER.md#register`
- `tmp/archive/tool-audit-2026-09-21/11-IMPLEMENTATION-CHECKLIST.md#release-gate`
- `tmp/archive/tool-audit-2026-09-21/00-INDEX.md#status-note-2026-09-14`

How to verify: Search for a long-stream/backpressure test asserting no control/terminal tool events are dropped; check bounded channels on the provider->TUI path use the documented drop policy.

Verified 2026-09-28: store-level unit tests exist: crates/roko-core/src/transcript_store.rs:775 eviction_drops_text_deltas_before_control_events and :800 control_events_never_evicted, with drop accounting via drop_report (:574). No long-stream/backpressure scenario test on the provider->TUI path asserts zero silent drops of control or terminal events, so the release gate is still unchecked. Severity p2 (missing proof).

Re-checked 2026-09-29 at d9e79e9d8: unchanged. PriorityEventChannel (transcript_store.rs:498) implements the drop-text-before-control policy, but only its own unit tests use it; the provider->TUI path does not go through it. crates/roko-agent/tests/streaming_contracts.rs:305 only shows that a blocking mpsc channel neither drops nor reorders text deltas.
