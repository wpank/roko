+++
id = "bug-f0f108"
kind = "bug"
title = "ACP bridge crash under sustained load (analyzed, not fixed)"
status = "done"
triage = "verified"
severity = "p1"
size = "M"
goal = "hermes"
subsystem = ["roko-acp/bridge_events"]
created = 2026-09-15
updated = 2026-10-02
last_verified = 2026-10-02
last_verified_rev = "2f82da96a"
source = "tmp/archive/refactoring-audit-2026-09-21/IMPLEMENTATION-CHECKLIST.md#p0-critical-data-loss-crashes-correctness"
discovered_from = "audit:tmp/archive/refactoring-audit-2026-09-21/IMPLEMENTATION-CHECKLIST.md#p0-critical-data-loss-crashes-correctness"
anchors = ["crates/roko-acp/src/acp_adapter.rs::AcpAdapter", "crates/roko-acp/src/bridge_events/permissions.rs::request_permission_for_event", "crates/roko-acp/src/bridge_events/protocol.rs::PermissionReplyChannel", "crates/roko-acp/src/bridge_events/mod.rs:345", "crates/roko-acp/src/bridge_events/mod.rs:551"]
links = { depends_on = [], blocks = [], related = ["bug-c59522", "bug-b2a9de"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -q 'from_millis(25)' crates/roko-acp/src/bridge_events/permissions.rs && grep -rqw 'fn acp_adapter_delivers_workflow_completed_when_channel_full' crates/roko-acp/ && cargo test -p roko-acp acp_adapter_delivers_workflow_completed_when_channel_full"

[closed]
at = 2026-10-02
at_ts = "2026-10-02T08:50:28Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
size = "M"
claimed_at = "2026-10-01T17:36:42Z"
forced = false
evidence = "implemented by wk-specq (d6f084175, merged through an earlier batch-6 gate): a full channel can't drop Complete, the permission wait no longer polls, a failed experiment task is logged. acp_adapter_delivers_workflow_completed_when_channel_full and the permission tests pass in roko-acp's lib tests (gate 6i at f4347b8eb, merged in f88210c84); plan step 5 (send timeouts, assistant_text cap, concurrent requests) was delivered by bug-c638a4"
+++

## Problem

An analysis of the ACP bridge under sustained load (2026-09-21, "P0-01 ACP crash analysis") found five risk
categories. The crash sites have since been fixed. What remains is a set of ways the bridge can lose events,
hang or waste work under load:

1. P0-A: `AcpAdapter::consume` (`crates/roko-acp/src/acp_adapter.rs:188-194`) forwards every mapped event with
   `self.sender.try_send(cognitive_event)` and only logs a warning if the channel is full. If the dropped event is
   `RuntimeEvent::WorkflowCompleted` (mapped to `CognitiveEvent::Complete`, line 126), the editor never gets a
   completion. The prompt hangs with an endless spinner, and the single-threaded handler loop is stuck behind it.
2. P1-B: while waiting for the editor's permission answer, `request_permission_for_event`
   (`crates/roko-acp/src/bridge_events/permissions.rs:240-262`) wakes every 25 ms
   (`tokio::time::sleep(Duration::from_millis(25))`) only to check `reply.receiver_is_closed()`. A 30 s wait is
   about 1,200 wakeups.
3. P0-B: `bridge_events/mod.rs:345-347` runs `assign_acp_experiment` on `spawn_blocking` and then calls
   `.unwrap_or(None)`, so a panicking blocking task is silently treated as "no experiment".
4. Not re-checked since the analysis, and still present in the code:
   - 3a/P1-A: the primary event channel is `mpsc::channel(256)` (`bridge_events/mod.rs:551`), and producers use
     `send_cognitive_event` (`bridge_events/helpers.rs:120`, `.send().await` with no timeout). A slow editor
     stalls the provider stream. The three tool-loop chunk channels (`dispatch.rs:274`, `:870`, `:1040`, 256 each)
     double the buffering, with no coordinated backpressure;
   - P2-A: `assistant_text` in `bridge_events/mod.rs` (line 112) grows without a bound during streaming;
   - 4a: `run_acp_server` (`handler.rs:141`) handles one request at a time, so a long `session/prompt` blocks
     `session/new` and config requests. `session/cancel` still works, because the stream loop reads inbound
     messages.

## Why it matters

Goal `hermes`: the Nous/Hermes demo runs long agent turns through ACP in an editor. A lost completion looks like a
frozen agent, and the only recovery is restarting the editor's agent process.
Related: `bug-c59522` (ACP stability hardening, done; its closure passed this residue to this item), `bug-b2a9de`
(ACP panics, done), `spec-704c28` (Part 6 would wire `AcpAdapter` for Graph runs), `gap-ac78fb` (edits the same
permission code).

## Where

- `crates/roko-acp/src/acp_adapter.rs`: `AcpAdapter` implements `roko_core::foundation::EventConsumer`, whose
  `consume(&self, &RuntimeEvent)` is synchronous (`crates/roko-core/src/foundation.rs:872`), which is why it uses
  `try_send`.
- `crates/roko-acp/src/bridge_events/permissions.rs`: `request_permission` (30 s editor round trip, fails closed)
  and `request_permission_for_event` (the polling loop).
- `crates/roko-acp/src/bridge_events/protocol.rs:149`: `PermissionReplyChannel` wraps
  `Arc<std::sync::Mutex<Option<oneshot::Sender<PermissionDecision>>>>`; see `receiver_is_closed` (line 188).
- `crates/roko-acp/src/bridge_events/mod.rs`: the stream loop (`CognitiveEvent::PermissionRequest` handling at
  line ~191), the primary channel (line 551) and the experiment `spawn_blocking` (line 345).
- Entry point: `roko acp`, and `session/prompt` from an editor.

## Current state

- Fixed elsewhere: the `unreachable!` in `builtin_tools.rs` (now line 449), production `unwrap`/`expect`, and
  lock-poisoning recovery (`bug-b2a9de` and `bug-c59522`, both closed with evidence at `91b4745f8`).
- New finding: `AcpAdapter::new` is never called outside the unit tests in `acp_adapter.rs` (lines 223, 243,
  263). No production code registers it as an `EventConsumer`. ACP workflow runs send events directly with
  `.send().await` (`runner.rs`). So P0-A is latent today: it becomes a live hang as soon as someone wires the
  adapter, which `spec-704c28` Part 6 plans to do. P1-B, P0-B, 3a, P2-A and 4a are live.
- The analysis document is not in the repository. The facts needed are copied above.

## Plan

1. P0-A: never drop terminal events in `AcpAdapter::consume`. Recommended: keep `try_send` for ordinary events. If
   the event is `CognitiveEvent::Complete` (or another terminal event) and `try_send` returns `Full`, clone the
   sender and deliver it with `tokio::runtime::Handle::try_current()` plus `spawn(async move { sender.send(ev).await })`.
   Log at `error` if there is no runtime. Alternatives, and why not: making `EventConsumer::consume` async changes
   a roko-core trait used by every consumer; an overflow `VecDeque` drained on the next `consume` call never drains
   after the final event.
2. Add `#[tokio::test] acp_adapter_delivers_workflow_completed_when_channel_full` in `acp_adapter.rs`: make a
   channel of capacity 1, fill it, `consume` a `WorkflowCompleted`, drain the receiver, and assert that `Complete`
   arrives.
3. P1-B: replace the 25 ms poll with an event-driven wait. `oneshot::Sender::closed()` needs `&mut Sender` and must
   not be awaited under the std `Mutex`. So take the sender out of `PermissionReplyChannel` (add a
   `take_sender()`), `select!` on `sender.closed()`, the editor request and `cancel_token.cancelled()`, then send
   the decision on that sender. Update the caller in `bridge_events/mod.rs` (it calls `reply.reply(decision)`) to
   match. Keep every non-answer path returning `PermissionDecision::Reject`.
4. P0-B: replace `.unwrap_or(None)` with `.unwrap_or_else(|error| { warn!(%error, "experiment assignment task failed"); None })`.
5. Optional, or file as separate items: P1-A (`send_timeout` for non-terminal events, or a larger channel), P2-A
   (cap `assistant_text`, for example at 1 MiB, with a warning), and 4a (run `session/prompt` off the handler loop).
   4a is a design change. Do not bundle it with this fix.

## Done when

- A full channel cannot drop `Complete`, and the new test proves it.
- The permission wait no longer polls. Cancelling the prompt, the tool side timing out and the editor answering
  all still resolve the wait; the existing permission tests in `bridge_events/tests.rs` still pass.
- A failed experiment-assignment task is logged.
- `cargo clippy -p roko-acp --no-deps -- -D warnings` is clean.
- Verify: `! grep -q 'from_millis(25)' crates/roko-acp/src/bridge_events/permissions.rs && grep -rqw 'fn acp_adapter_delivers_workflow_completed_when_channel_full' crates/roko-acp/ && cargo test -p roko-acp acp_adapter_delivers_workflow_completed_when_channel_full`

## Notes

- The old verify command also required that `try_send(cognitive_event)` disappear. That forbids the recommended
  fix, which keeps `try_send` for ordinary events. It also ran a cargo test filter for a test that did not exist,
  which exits 0. The command above replaces both.
- Safety: permission handling must stay fail-closed. Never turn a timeout or a closed channel into Allow.
- Do not change the `EventConsumer` trait in roko-core for this. Other implementors depend on it: `roko-serve` (`adapters.rs`, `lib.rs`), `roko-runtime` (`http_event_sink.rs`, `jsonl_logger.rs`) and `roko-agent` (`model_call_service.rs`).
- Conflicts: `gap-ac78fb` extends `permissions.rs`, and `spec-704c28` edits `bridge_events/`. Do this item first
  or separately. Not safe to run in parallel with them.
- 2026-10-01 (wk-specq): implemented on work/bug-8dbffd; cargo verification deferred to the batch check.
  P0-A: `AcpAdapter::consume` still drops progress on a full channel, but a turn-ending event (`Complete`,
  `Failure`, `MaxTokens`) is delivered by a task on the current runtime. P1-B: `request_permission_for_event` takes
  the sender (`PermissionReplyChannel::take_sender`, replacing `receiver_is_closed`), selects on `closed()`,
  cancellation and the editor round-trip, and sends the decision itself. P0-B: a failed experiment task is logged.
  Plan step 5 (P1-A send timeouts, P2-A `assistant_text` cap, 4a concurrent requests) is not done.

## Original notes

Analysis found 5 crash-risk categories (2 potentially P0-grade): unreachable! in builtin_tools.rs, bounded channel exhaustion without backpressure, lock poisoning, SSE channel lag, WebSocket backpressure. No source changes were made.

Imported without verification from:
- `tmp/archive/refactoring-audit-2026-09-21/IMPLEMENTATION-CHECKLIST.md#p0-critical-data-loss-crashes-correctness`
- `tmp/archive/refactoring-audit-2026-09-21/P0-01-ACP-CRASH-ANALYSIS.md`

Warning: every file this item cites is gone (`crates/roko-acp/src/bridge_events.rs`) — likely obsolete or moved.

How to verify: Check roko-acp for the unreachable! in builtin_tools.rs, bounded channel send behavior under load, Mutex poisoning handling, SSE lag handling; related provider F005.

Verified 2026-09-28: the crash sites are gone, so p0 -> p1. builtin_tools.rs:449 replaced the `unreachable!`, there is no production unwrap/expect, and poisoned locks are recovered. Still open from P0-01's recommendations: P0-A, where `AcpAdapter::consume` (crates/roko-acp/src/acp_adapter.rs:189-194) still `try_send`s every event, including `WorkflowCompleted`, and only warns when the channel is full. P1-B, where bridge_events/permissions.rs:260 still polls every 25ms. Backpressure between the provider stream and the editor (3a) and the single-threaded handler loop (4a, handler.rs:141) were not re-checked.
