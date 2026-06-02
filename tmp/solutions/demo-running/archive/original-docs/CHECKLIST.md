# Implementation Checklist: Demo Streaming Wiring

## Phase 1: Critical Path (unblocks demo)

### 1.1 Serve-side event ingest endpoint
- [ ] Add `POST /api/events/ingest` route to roko-serve
- [ ] Add `POST /api/events/ingest/batch` route (array of events)
- [ ] Add auth token validation (ROKO_SERVER_AUTH_TOKEN header)
- [ ] Publish received events to `state.event_bus` AND `state.state_hub`
- **File:** `crates/roko-serve/src/routes/events.rs` (new or extend existing)
- **Parallelizable:** Yes, independent of other work

### 1.2 Wire SharedStateHub from AppState into CliRuntime
- [ ] Add `state_hub: SharedStateHub` field to `RokoCliRuntime`
- [ ] Pass `app_state.state_hub.clone()` at construction time (in `state.rs`)
- [ ] Use `self.state_hub` instead of `shared_state_hub()` in `build_runner_config()`
- **File:** `crates/roko-cli/src/serve_runtime.rs`
- **Parallelizable:** Yes, independent of 1.1

### 1.3 Wire FeedbackFacade and Projection in serve path
- [ ] Create `EpisodeSink` in `build_runner_config()`
- [ ] Create `RoutingObservationSink` in `build_runner_config()`
- [ ] Create `KnowledgeIngestionSink` in `build_runner_config()`
- [ ] Create `FeedbackFacade` with all three sinks
- [ ] Create `Projection` with run UUID
- [ ] Set `feedback_facade: Some(...)` and `projection: Some(...)` in RunConfig
- **File:** `crates/roko-cli/src/serve_runtime.rs`
- **Depends on:** 1.2 (same file, same function)

### 1.4 Emit TaskStarted/TaskCompleted/TaskFailed as ServerEvents
- [ ] Add `ServerEvent::TaskStarted` emission in orchestrate.rs task start
- [ ] Add `ServerEvent::TaskCompleted` emission in orchestrate.rs task end
- [ ] Add `ServerEvent::TaskFailed` emission in orchestrate.rs gate failure
- **File:** `crates/roko-cli/src/orchestrate.rs`
- **Parallelizable:** Yes, independent of 1.1-1.3

---

## Phase 2: HTTP Event Sink (CLI subprocess → serve)

### 2.1 Create HttpEventSink module
- [ ] Create `crates/roko-cli/src/runner/http_sink.rs`
- [ ] Implement `HttpEventSink::from_env()` (checks ROKO_SERVE_URL)
- [ ] Implement background sender task with mpsc channel
- [ ] Implement `emit()` with try_send (non-blocking)
- [ ] Add batching optimization (50ms window, 32 event max)
- **Parallelizable:** Yes, independent

### 2.2 Integrate HttpEventSink into runner event loop
- [ ] Import and construct `HttpEventSink` in runner initialization
- [ ] Add `dashboard_event_to_server_event()` conversion function
- [ ] Call `sink.emit()` in the runner's event emission path
- **File:** `crates/roko-cli/src/runner/mod.rs` (or event loop location)
- **Depends on:** 2.1

### 2.3 Inject ROKO_SERVE_URL in PTY sessions
- [ ] Pass `serve_url` to `SessionManager::create_session()`
- [ ] Add `cmd.env("ROKO_SERVE_URL", ...)` in terminal.rs
- [ ] Add `cmd.env("ROKO_SERVER_AUTH_TOKEN", ...)` in terminal.rs
- [ ] Add `cmd.env("ROKO_SESSION_ID", ...)` in terminal.rs
- **File:** `crates/roko-serve/src/terminal.rs`
- **Parallelizable:** Yes, independent of 2.1/2.2

---

## Phase 3: ACP Bridge

### 3.1 Create ACP event sink
- [ ] Create `crates/roko-acp/src/event_sink.rs`
- [ ] Implement `HttpEventSink::from_env()` for ACP
- [ ] Implement `CognitiveEvent → ServerEvent` mapping
- [ ] Fire-and-forget async POST
- **Parallelizable:** Yes, independent

### 3.2 Wire sink into ACP session runner
- [ ] Construct `HttpEventSink` in session initialization
- [ ] Call `sink.emit()` for each `CognitiveEvent` produced
- **File:** `crates/roko-acp/src/bridge_events.rs` (or runner)
- **Depends on:** 3.1

### 3.3 Inject env vars when ACP is spawned from serve
- [ ] Find where roko-serve spawns/manages ACP processes
- [ ] Inject `ROKO_SERVE_URL` and `ROKO_ACP_AGENT_ID`
- **Parallelizable with:** 3.1/3.2

---

## Phase 4: Rich Event Coverage

### 4.1 Inference tracking events
- [ ] Define `InferenceObserver` trait in roko-agent
- [ ] Implement observer in roko-cli (emits ServerEvent)
- [ ] Wire observer into dispatcher's LLM call path
- [ ] Emit InferenceStarted before HTTP call
- [ ] Emit InferenceCompleted after response
- [ ] Emit InferenceFailed on error
- **Files:** `crates/roko-agent/src/dispatcher/mod.rs`, `crates/roko-cli/src/`
- **Parallelizable:** Yes

### 4.2 AgentTrace per-turn events
- [ ] Emit AgentTrace in agent tool loop (per turn)
- [ ] Include reasoning, tool_calls, usage in trace
- **File:** `crates/roko-agent/src/` (tool loop)
- **Parallelizable:** Yes

### 4.3 Somatic marker events
- [ ] Emit SomaticMarkerFired when daimon fires marker in orchestrate.rs
- **File:** `crates/roko-cli/src/orchestrate.rs`
- **Parallelizable:** Yes

### 4.4 Config/Strategy reload events
- [ ] Emit ConfigReloaded when config watcher triggers
- [ ] Emit StrategyReloaded when goals/tactics change
- **Parallelizable:** Yes

---

## Parallelization Guide

```
Phase 1 (can run simultaneously):
  ├── 1.1 (ingest endpoint)      │ Independent
  ├── 1.2 + 1.3 (state hub)     │ Sequential pair
  └── 1.4 (task events)          │ Independent

Phase 2 (can run simultaneously):
  ├── 2.1 → 2.2 (http sink)     │ Sequential pair
  └── 2.3 (pty env)              │ Independent

Phase 3 (can run simultaneously):
  ├── 3.1 → 3.2 (acp sink)      │ Sequential pair
  └── 3.3 (acp env injection)    │ Independent

Phase 4 (all independent):
  ├── 4.1 (inference)
  ├── 4.2 (agent trace)
  ├── 4.3 (somatic)
  └── 4.4 (config reload)
```

## Estimated Scope

| Phase | Files touched | Lines of code (est.) |
|-------|--------------|---------------------|
| Phase 1 | 3-4 | ~150 |
| Phase 2 | 3-4 | ~200 |
| Phase 3 | 3-4 | ~150 |
| Phase 4 | 5-6 | ~300 |

## Verification Matrix

| What to test | How |
|-------------|-----|
| Events reach SSE | `curl -N http://localhost:6677/api/events/stream` during plan run |
| Events reach WS | Connect WS to `/api/ws`, confirm events arrive |
| Frontend renders | Open demo-app, trigger plan, see activity |
| CLI→serve forwarding | Run `roko plan run` with ROKO_SERVE_URL, check SSE |
| PTY→serve forwarding | Open terminal tab, run roko command, check SSE |
| ACP→serve forwarding | Start ACP with ROKO_SERVE_URL, send prompt, check SSE |
| Auth works | Send event without token → 401; with token → 202 |
| Batching works | Send burst of 100 events, confirm they arrive batched |
