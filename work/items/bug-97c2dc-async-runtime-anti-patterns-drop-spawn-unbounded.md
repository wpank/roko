+++
id = "bug-97c2dc"
kind = "bug"
title = "Async Runtime Anti-Patterns (Drop+Spawn, Unbounded Channels, Mutex Across Await)"
status = "open"
triage = "verified"
severity = "p2"
size = "M"
goal = "tooling"
subsystem = ["roko-agent"]
created = 2026-09-07
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "ebdc0f5d5"
source = "tmp/backlog/archive/101-async-runtime-anti-patterns.md#101 — Async Runtime Anti-Patterns (Drop+Spawn, Unbounded Channels, Mutex…"
discovered_from = "audit:tmp/backlog/archive/101-async-runtime-anti-patterns.md#101 — Async Runtime Anti-Patterns (Drop+Spawn, Unbounded Channels, Mutex…"
anchors = ["crates/roko-agent-server/src/features/messaging.rs::stream_prompt", "crates/roko-agent-server/src/state.rs::DispatchLike", "crates/roko-agent/src/harness/acp_client.rs::AcpStdioClient", "crates/roko-agent/src/harness/acp_client.rs::connect"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -n 'unbounded_channel' crates/roko-agent-server/src/features/messaging.rs && ! sed -n '1,/#\\[cfg(test)\\]/p' crates/roko-agent/src/harness/acp_client.rs | grep -q 'unbounded_channel' && grep -rqw 'fn notification_backlog_does_not_block_responses' crates/roko-agent/src/ && cargo test -p roko-agent notification_backlog_does_not_block_responses"
+++

## Problem

Two production code paths still queue async events on `tokio::sync::mpsc::unbounded_channel()`, so a slow
consumer or a chatty producer can grow memory without limit:

1. The per-agent sidecar's WebSocket stream (`roko agent serve`, `/stream`): `stream_prompt` creates an
   unbounded channel for model stream events and forwards them to the socket. A slow WebSocket client lets
   events pile up in memory.
2. The ACP stdio client used by the Hermes and OpenClaw providers (`AcpStdioClient`): the notification queue,
   the turn-done queue and the JSON-RPC response queue are all unbounded. The notification receiver is only
   drained during a turn, so notifications that arrive between turns accumulate.

This item started as backlog spec 101 with three anti-patterns: `tokio::spawn` inside `Drop`, unbounded
channels, and a mutex held across `.await`. Only the unbounded channels remain.

Expected: every production channel in these paths is bounded, with a stated overflow policy that cannot
deadlock the reader task.

## Why it matters

- Goal `tooling` (code hygiene). The risk is latent memory growth in long-running processes: the sidecar and
  Hermes/OpenClaw sessions.
- The ACP client matters to goal `hermes` as well: it carries every Hermes turn. A careless fix there (bounded
  channel with `send().await` in the reader) can deadlock a session, which is worse than the current risk.
- No other open item covers this.

## Where

- `crates/roko-agent-server/src/features/messaging.rs::stream_prompt` (`:153`; channel at `:171`): creates
  `mpsc::unbounded_channel()`, spawns `dispatcher.dispatch_streaming(request, event_tx)`, and forwards each
  `StreamEvent` to the socket with `send_socket_payload(...).await`.
- `crates/roko-agent-server/src/state.rs::DispatchLike` (`:256-270`): trait method
  `dispatch_streaming(&self, request, event_tx: mpsc::UnboundedSender<StreamEvent>)`. The default
  implementation drops `event_tx` and calls `dispatch`.
- `crates/roko-agent-server/src/state.rs` `impl DispatchLike for BackendMessageDispatcher` (`:283-330`):
  takes `_event_tx` and ignores it. It collects the whole backend stream with `collect_stream_to_response`.
- Other `DispatchLike` implementors: `crates/roko-cli/src/agent_serve.rs:885` (`ServingAgentDispatcher`, uses
  the default), and test mocks in `messaging.rs:303-320`, `features/logs.rs:150`,
  `tests/relay_registration.rs:96`, `tests/sidecar_integration.rs:37`. Only the trait, `BackendMessageDispatcher`
  and the `messaging.rs` mock name `UnboundedSender<StreamEvent>`.
- `crates/roko-agent/src/harness/acp_client.rs::AcpStdioClient`: fields `response_rx`, `notification_tx/rx`,
  `turn_done_tx/rx` (`:291-295`); `AcpStdioClient::new` creates the notification and turn-done channels
  (`:308-309`); `connect` (`:425`) creates the response channel (`:469`) and spawns the stdout reader task, which
  routes responses to `resp_tx` (`:528`), stop-reason responses also to `turn_done_tx` (`:525`), and
  notifications and server requests to `notification_tx` (`:540`). `recv_response` (`~:631`) skips responses
  with other ids. `take_notification_rx` / `return_notification_rx` (`:861-867`) and the turn-done
  equivalents (`:875-881`) lend the receivers to a turn.
- Consumers: `crates/roko-agent/src/hermes/acp_agent.rs` (takes and returns `notification_rx` around each turn,
  `:306-401`, `:515-532`) and `crates/roko-agent/src/openclaw/acp_agent.rs`.
- Not affected: the `dispatch_streaming` methods in `crates/roko-graph/src/cells/task_executor.rs:410` and
  `crates/roko-cli/src/graph_task_dispatch.rs:4128` belong to a different trait.

## Current state

- Issue 1 (spawn in `Drop`) is fixed. `RestartRecovery::drop` guards its spawn with
  `tokio::runtime::Handle::try_current()` (`crates/roko-runtime/src/connector_runtime.rs:197`), and
  `spawn_cleanup` does too (`:589`). `CursorCliAgent` has no spawning `Drop`.
- Issue 3 (mutex across `.await`) is fixed in the Cursor agent. `CursorCliAgent` no longer holds a connection
  mutex, and `spawn_connection` holds only the global startup lock (re-checked 2026-09-29).
- Issue 2: `crates/roko-agent/src/cursor_cli_agent.rs` has no `unbounded_channel` left. Four production sites
  remain: `messaging.rs:171` and `acp_client.rs:308`, `:309`, `:469`. Every other hit in
  `crates/roko-agent/src` and `crates/roko-agent-server/src` is inside a `#[cfg(test)]` module
  (`acp_client.rs:1132+`, `relay_client.rs:1565`).
- Side observation: no production `DispatchLike` sends stream events today (`BackendMessageDispatcher` ignores
  the sender, and `ServingAgentDispatcher` uses the default), so the sidecar `/stream` socket only gets the final
  `done` frame. The unbounded growth there is latent until streaming is wired. Wiring live streaming is out of
  scope here.
- Recent commits on these files: `c41e78c7a`, `244f564e1`, `72e0a76b8`; none bounded these channels.

## Plan

1. Sidecar stream (`roko-agent-server`):
   - Change `DispatchLike::dispatch_streaming` to take `mpsc::Sender<StreamEvent>` and update the default
     implementation, `BackendMessageDispatcher` and the `messaging.rs` test mock.
   - In `stream_prompt`, use `mpsc::channel(256)`. Producers call `send(..).await`, which applies backpressure
     to the stream task. The consumer loop is already the only reader, so this cannot deadlock.
2. ACP stdio client (`roko-agent`). Choose the overflow policy explicitly, because the stdout reader task feeds
   all three channels and the notification receiver is not drained between turns:
   - (a) bounded channels with `send().await` in the reader. Simple, but if nobody drains notifications the
     reader blocks, responses stop arriving, and the caller's `recv_response` hangs. Rejected.
   - (b) Recommended: bounded channels with `try_send` in the reader and a clear policy on `Full`:
     - plain notifications: drop, `tracing::warn!` once per burst, and keep a dropped-count;
     - server requests (`server_request_id.is_some()`, for example `session/request_permission`): never drop
       silently. Write a JSON-RPC error reply for that id to the child's stdin, or reserve room for them in a
       separate small channel, so the agent is not left waiting;
     - responses: size the channel for the maximum number of in-flight requests (for example 64).
       `recv_response` already discards stale ids, so the channel drains on the next request.
   - (c) keep unbounded channels and add a counter cap. This reimplements a bounded channel; not recommended.
   - Update the field types (`mpsc::Receiver`/`Sender`), `take_*`/`return_*` signatures, the Hermes and OpenClaw
     consumers, and the test constructors in `acp_client.rs` (`:1132-1473`).
3. Tests:
   - `roko-agent`: `notification_backlog_does_not_block_responses`. Fill the notification channel past capacity
     with no consumer, then assert that a response for a pending request still reaches `recv_response`.
   - `roko-agent-server`: a mock dispatcher that sends more than 256 events to a slow reader completes without
     error and delivers events in order.

## Done when

- `messaging.rs` and the non-test part of `acp_client.rs` contain no `unbounded_channel`.
- The overflow policy for ACP notifications and server requests is implemented and documented on the fields.
- `notification_backlog_does_not_block_responses` passes; the existing `roko-agent` and `roko-agent-server`
  tests pass.
- Verify:
  `! grep -n 'unbounded_channel' crates/roko-agent-server/src/features/messaging.rs && ! sed -n '1,/#\[cfg(test)\]/p' crates/roko-agent/src/harness/acp_client.rs | grep -q 'unbounded_channel' && grep -rqw 'fn notification_backlog_does_not_block_responses' crates/roko-agent/src/ && cargo test -p roko-agent notification_backlog_does_not_block_responses`

## Notes

- The current verify command only checks `messaging.rs`, so it would pass with the three ACP client channels
  still unbounded. The command above also covers `acp_client.rs` (non-test part only; its first `#[cfg(test)]`
  is at `:917`) and guards the new test.
- The ACP client change touches the Hermes and OpenClaw providers. Run their tests (`cargo test -p roko-agent
  hermes`, `openclaw`) and, if possible, one live Hermes turn before closing.
- The two parts are independent. The sidecar part is a small, safe change; the ACP client part needs care.
  Splitting into two commits is fine.
- Parallel safety: the sidecar part is isolated. The ACP client part conflicts with any concurrent work in
  `crates/roko-agent/src/harness/acp_client.rs` or `hermes/acp_agent.rs`.
- 2026-10-01 (wk-guard2): implemented on work/bug-a70def; cargo verification deferred to the batch check.
- Sidecar: `DispatchLike::dispatch_streaming` takes an `mpsc::Sender<StreamEvent>` and `stream_prompt` uses `mpsc::channel(256)` (test `stream_delivers_more_events_than_the_channel_holds`). ACP client: the response (64), notification (1024) and turn-done (16) queues are bounded, and the stdout reader, now `ServerMessageRouter`, only calls `try_send`. Notifications leave the last 64 notification slots to server requests and are dropped past that, with one warning per burst and the count logged when room returns; a server request is dropped only when the queue is completely full, with an error naming it; a full response or turn-done queue drops with a warning. The fields document the policy. Test `notification_backlog_does_not_block_responses`; the two fixture tests now drive the real router instead of a copy of its logic.
- Server requests get reserved room rather than an error reply, because the reader does not own stdin and nothing answers server requests today (OpenClaw only logs `session/request_permission`). Still to do before closing: `cargo test -p roko-agent hermes` and `openclaw`, and one live Hermes turn if possible.

## Original notes

latent crash and memory exhaustion risks in production dispatch paths. Tokio, the async runtime used throughout roko, has three well-known async anti-patterns that cause real failures in production:

Imported without verification from:
- `tmp/backlog/archive/101-async-runtime-anti-patterns.md#101 — Async Runtime Anti-Patterns (Drop+Spawn, Unbounded Channels, Mutex…`

How to verify: Check: No `tokio::spawn` in `Drop` impls without a `Handle::try_current()` guard.; `CursorCliAgent::Drop` does not spawn; it logs a warning if the connection was not; All three `unbounded_channel` call sites in `cursor_cli_agent.rs` replaced with [evidence: 00-STATUS-SUMMARY 3. Open / P1 -- High (Open): M | 2 |]

Verified 2026-09-28: partly fixed. The crash-class Drop+spawn part is gone, so p1 -> p2. Fixed: connector_runtime.rs:197 guards its Drop spawn with `Handle::try_current()`, and cursor_cli_agent.rs has no Drop spawn and no `unbounded_channel`. Still open: crates/roko-agent-server/src/features/messaging.rs:171 still uses `mpsc::unbounded_channel()` for the streaming event channel (acceptance item 4 of tmp/backlog/archive/101-async-runtime-anti-patterns.md). Issue 3 (mutex held across `.await`) was not re-checked.

Re-checked 2026-09-29: issue 1 (Drop+spawn) and issue 3 (mutex across await in the Cursor agent) are resolved. CursorCliAgent no longer keeps a connection mutex; spawn_connection holds only the global startup lock. Issue 2 remains: messaging.rs:171 still uses mpsc::unbounded_channel() for the streaming event channel, and harness/acp_client.rs has three more production unbounded channels (:308, :309, :469) that the backlog's acceptance grep over crates/roko-agent/src would also flag.
