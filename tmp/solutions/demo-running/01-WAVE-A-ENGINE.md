# Wave A: Engine Convergence + Event Architecture

## Root Cause

The streaming gap exists because `serve_runtime.rs:274` creates a disconnected `SharedStateHub`.
Events emitted by the runner never reach SSE/WS clients. The fix is straightforward wiring —
the v2 engine is already the only production path (orchestrate.rs is feature-gated OFF).

Additionally, out-of-process roko commands (PTY terminals, ACP) have no event forwarding
mechanism. The fix: a generic `HttpEventSink` that POSTs to `/api/events/ingest`.

---

## Task A1: Port Missing Legacy Features to v2

**Root cause**: 2-3 features exist in `orchestrate.rs` that v2 lacks.

**Current state** (from audit):
- `orchestrate.rs` is feature-gated: `#[cfg(feature = "legacy-orchestrate")]`
- NOT in default features — no production binary includes it
- Most "features to port" are already in v2 or dead code

**Features to port** (verify each has value):

| Feature | Legacy location | Port action |
|---------|----------------|-------------|
| Daimon/somatic modulation | orchestrate.rs (12+ sites) | Port to v2 event_loop as optional hook |
| HDC fingerprint per-episode | orchestrate.rs | Port to FeedbackFacade sink |

**What NOT to port** (dead or already in v2):
- `workspace_context()` → v2 has `generate_workspace_map()` (IS called)
- `dispatch_and_record()` → zero callers even in legacy. Delete.
- `publish_dashboard_event()` → v2 has TuiBridge (same thing)
- `emit_server_event()` → replaced by EventConsumer pipeline

**Verification**: `cargo test --workspace` passes. Ported features observable in v2 output.

---

## Task A2+A3: Wire serve_runtime.rs State Integration

**Root cause**: `serve_runtime.rs` creates a LOCAL `SharedStateHub` (line 274) disconnected
from `AppState.state_hub`. Also sets `feedback_facade: None, projection: None`.

**IMPORTANT** (audit finding): serve_runtime.rs ALREADY uses v2. Line 277 calls
`crate::runner::run()` which is the v2 event_loop. The "redirect to v2" task is a NO-OP.
The real work is wiring the state connections.

**Fix** (merged A2+A3+A4 into one task — same file, sequential dependency):

1. Add `state_hub: SharedStateHub` to `RokoCliRuntime` struct
2. Construct with `app_state.state_hub.clone()` in `state.rs`
3. In `build_runner_config()`, pass `self.state_hub.clone()` instead of `shared_state_hub()`
4. Wire FeedbackFacade (clone from `commands/plan.rs:420-508`):
   ```rust
   let feedback_facade = Arc::new(
       FeedbackFacade::new()
           .with_sink(Arc::new(EpisodeSink::at(&episodes_path)))
           .with_sink(Arc::new(RoutingObservationSink::new(cascade_router)))
           .with_sink(Arc::new(KnowledgeIngestionSink::at(&knowledge_path))),
   );
   let projection = Arc::new(Projection::new(run_uuid));
   ```
5. Set `projection: Some(projection)` in RunConfig

**Verification**:
```bash
# Terminal 1
cargo run -p roko-cli -- serve

# Terminal 2
curl -N http://127.0.0.1:6677/api/events/stream

# Terminal 3 (trigger plan execution via API)
curl -X POST http://127.0.0.1:6677/api/plans/test/execute

# Confirm: SSE stream shows events. .roko/episodes.jsonl has entries.
```

---

## Task A4: Deprecate Legacy PlanRunner

**Root cause**: Two engines means potential confusion. `orchestrate.rs` is already
feature-gated OFF but the `PlanRunner` type isn't marked deprecated.

**Current state** (audit finding): Already ~90% done:
- `orchestrate.rs` is `#[cfg(feature = "legacy-orchestrate")]`
- NOT in default features
- No production code path enters it

**Remaining actions**:
1. Add `#[deprecated(note = "Use event_loop.rs v2")]` to PlanRunner struct
2. Verify zero non-test callers (should already be true)
3. Remove `--engine legacy` flag if it still exists

**Verification**: `grep -rn 'PlanRunner::new\|PlanRunner::from' crates/ --include='*.rs' | grep -v test | grep -v target/` → empty

---

## Task A5: Event Ingest Endpoint

**Status - 2026-05-05**: IMPLEMENTED + TESTED (task 105).
Route-level tests in `crates/roko-serve/src/routes/event_ingest.rs`:
- `single_ingest_returns_202` -- canonical agent_output JSON
- `batch_ingest_returns_202` -- two mixed RuntimeEvent variants
- `batch_over_1000_returns_error` -- enforces 1000-event batch max
- `single_ingest_reaches_sse_adapter` -- event reaches SSE subscriber
- `single_ingest_reaches_jsonl_logger` -- event written to `.roko/runtime-events.jsonl`
- `non_loopback_without_auth_is_forbidden` -- security model enforced

**Design**: Single universal HTTP endpoint:

```
POST /api/events/ingest
Content-Type: application/json
Authorization: Bearer {ROKO_SERVER_AUTH_TOKEN}  (optional)

Body: RuntimeEvent JSON (serde: tag="kind", content="data", rename_all="snake_case")
```

Canonical body example:
```json
{"kind":"agent_output","data":{"run_id":"task105","agent_id":"manual","chunk":"hello-ingest"}}
```

On receipt:
1. Deserialize to `RuntimeEvent`
2. Pass through `consume_runtime_event` -> `SseAdapter.consume(event)` + `JsonlLogger.consume(event)`
3. Return 202 Accepted

**Batch variant**:
```
POST /api/events/ingest/batch
Body: [RuntimeEvent, RuntimeEvent, ...]  (max 1000 events)
```

**Security**: Localhost-only by default. Configurable via bearer token (`ROKO_SERVER_AUTH_TOKEN`
or `server.auth_token` in config), IP allowlist (`serve.event_ingest_allowlist`), or
`serve.auth.enabled`. Non-loopback requests without any of these are rejected with 403.

**Verification**:
```bash
curl -i -X POST http://127.0.0.1:6677/api/events/ingest \
  -H "Content-Type: application/json" \
  -d '{"kind":"agent_output","data":{"run_id":"task105","agent_id":"manual","chunk":"hello-ingest"}}'
# Returns 202, event appears in SSE stream and .roko/runtime-events.jsonl
```

---

## Task A6: HttpEventSink (Generic, in roko-runtime)

**Status - 2026-05-05**: IMPLEMENTED + TESTED (task 105).
Tests in `crates/roko-runtime/src/http_event_sink.rs`:
- `batch_endpoint_trims_trailing_slash` -- URL construction correctness
- `emit_does_not_block_when_channel_saturated` -- try_send non-blocking guarantee
- `from_env_returns_none_when_unset` -- env var absent returns None
- `sink_posts_to_batch_endpoint` -- axum mock server captures posted batch
- `sink_includes_bearer_token` -- Authorization header verification
- `sink_batches_up_to_max` -- batch size <= 32 enforced

**Root cause**: Subprocesses (PTY roko commands, ACP sessions) emit events locally but
have no connection to the server's event pipeline.

**Design**: Generic non-blocking HTTP sink in `roko-runtime` (NOT roko-cli):

```rust
// crates/roko-runtime/src/http_event_sink.rs
pub struct HttpEventSink {
    tx: mpsc::Sender<RuntimeEvent>,
}

impl HttpEventSink {
    /// Creates sink from ROKO_SERVE_URL env var. Returns None if not set.
    pub fn from_env() -> Option<Self> {
        let url = std::env::var("ROKO_SERVE_URL").ok()?;
        let (tx, rx) = mpsc::channel(256);
        // Spawn background task: batch (50ms window, 32 max) + POST
        tokio::spawn(Self::batch_and_post(rx, url));
        Some(Self { tx })
    }

    pub fn emit(&self, event: RuntimeEvent) {
        let _ = self.tx.try_send(event); // Non-blocking, drops if full
    }
}
```

**Why roko-runtime** (not roko-cli): Both CLI subprocesses AND ACP processes need this.
Placing it in roko-runtime makes it reusable without either depending on the other.

**Integration in v2 event_loop**: Alongside existing EventConsumer emissions:
```rust
if let Some(sink) = &http_sink {
    sink.emit(event.clone());
}
```

**Verification**:
```bash
# Terminal 1: serve
cargo run -p roko-cli -- serve

# Terminal 2: SSE watch
curl -N http://127.0.0.1:6677/api/events/stream

# Terminal 3: plan run with sink
ROKO_SERVE_URL=http://127.0.0.1:6677 cargo run -p roko-cli -- plan run plans/test/

# Confirm: events in terminal 2
```

---

## Task A7: PTY Environment Injection

**Root cause**: `terminal.rs` spawns shells without `ROKO_SERVE_URL`, so roko commands
in PTY can't forward events.

**Fix**: In `terminal.rs` SessionManager, inject:
```rust
cmd.env("ROKO_SERVE_URL", &serve_url);
cmd.env("ROKO_SESSION_ID", &session_id);
if let Some(token) = &auth_token {
    cmd.env("ROKO_SERVER_AUTH_TOKEN", token);
}
```

The `serve_url` comes from AppState (which knows its own address).

**Verification**: Open terminal via WS, run `echo $ROKO_SERVE_URL`, confirm it's set.
Then run `roko plan run` in the PTY and confirm SSE stream shows events.

---

## Task A8: ACP Event Bridge

**Status - 2026-05-05**: IMPLEMENTED + TESTED (task 105).
Tests in `crates/roko-acp/src/event_forward.rs`:
- `token_chunk_maps_to_agent_output` -- TokenChunk -> AgentOutput
- `thinking_chunk_maps_to_feedback_recorded` -- ThinkingChunk -> FeedbackRecorded(acp_thinking)
- `tool_call_start_maps_to_gate_started` -- ToolCallStart -> GateStarted
- `tool_call_completed_maps_to_gate_passed` -- Completed status -> GatePassed
- `tool_call_failed_maps_to_gate_failed_with_content` -- Failed status + content summarization
- `tool_call_pending/in_progress_maps_to_none` -- non-terminal statuses produce no event
- `plan_update_maps_to_feedback_recorded` -- PlanUpdate -> FeedbackRecorded(acp_plan_update)
- `mcp_status_maps_to_feedback_recorded` -- McpStatus -> FeedbackRecorded(acp_mcp_status)
- `complete_end_turn_maps_to_agent_completed` -- EndTurn -> AgentCompleted with token count
- `complete_cancelled_maps_to_workflow_cancelled` -- Cancelled -> WorkflowCompleted::Cancelled
- `complete_max_tokens/refusal_maps_to_workflow_halted` -- terminal stops -> Halted
- `failure_maps_to_agent_failed` -- Failure -> AgentFailed
- `max_tokens_event_maps_to_workflow_halted` -- MaxTokens variant -> Halted
- `summarize_content_*` -- content block summarization helpers
- `stop_reason_label_covers_all_variants` -- all StopReason variants mapped

**Root cause**: roko-acp has `CognitiveEvent` (8 variants in `bridge_events.rs`).
ACP communicates with editors via stdio and needs a forwarding path to the serve layer.

**Design**: Reuse the same `HttpEventSink` from A6 (it's in roko-runtime, ACP can depend on it):

Current implementation lives in `crates/roko-acp/src/event_forward.rs`:
- `CognitiveEvent::TokenChunk(chunk)` -> `RuntimeEvent::AgentOutput { run_id, agent_id, chunk }`
- `CognitiveEvent::ThinkingChunk(chunk)` -> `RuntimeEvent::FeedbackRecorded`
- `CognitiveEvent::ToolCallStart` / `ToolCallComplete` -> gate started/passed/failed events
- `CognitiveEvent::Complete` -> `AgentCompleted` or `WorkflowCompleted`
- `CognitiveEvent::Failure` -> `RuntimeEvent::AgentFailed`

**Key insight** (audit): A8 is NOT a separate sink implementation — it's a thin adapter
over the generic HttpEventSink from A6. The code is ~50 lines of mapping logic.

**Verification**: Set `ROKO_SERVE_URL`, start ACP session, send prompt, confirm
events appear in SSE stream.

---

## Dependency Graph

```
A1 (port 2-3 features) ─┐
                         ├─→ A2+A3 (wire StateHub + FeedbackFacade + Projection)
A4 (deprecate legacy)  ─┘

A5 (ingest endpoint) → A6 (HttpEventSink) → A7 (PTY env)
                                           → A8 (ACP bridge)
```

**Critical path for Milestone 1**: A2+A3 (wire serve state) + A5+A6 (ingest + sink)

A1 and A4 are cleanup — important but not blocking the demo.

---

## What This Wave Achieves

After Wave A:
- `roko serve` + plan execution → events visible in SSE stream
- PTY-spawned commands → events forwarded to server
- ACP sessions → events forwarded to server
- Episodes, routing, knowledge all persist from serve-initiated runs
- Legacy engine formally deprecated (already functionally dead)

Total new code: ~400-600 lines (mostly wiring existing pieces together)
