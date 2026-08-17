# Demo Streaming: Problem Statement & Architecture

## The Problem

The demo IDE frontend connects to `roko serve` via SSE/WebSocket and expects real-time
`ServerEvent` updates for plan execution, agent activity, inference tracking, etc.
The frontend types define 50+ event variants. The backend emits ~20 of them — and only
from routes that perform inline operations (bench, job creation). The most important
event source — **plan execution** — is completely disconnected from the serve event bus.

## Root Causes

### 1. Plan execution runs in an isolated runtime

When `roko serve` dispatches plan execution, it calls `CliRuntime::run_plan` which
invokes `build_runner_config()` in `crates/roko-cli/src/serve_runtime.rs`:

```rust
// serve_runtime.rs lines 571-572
feedback_facade: None,   // no learning/knowledge sinks
projection: None,        // no event projection/SSE mirroring
```

The runner also creates its own disconnected `SharedStateHub` (line 274) rather than
using `AppState.state_hub`. Events published by the runner never reach SSE/WS clients.

**Contrast with CLI path:** `commands/plan.rs` creates a real `FeedbackFacade` with 3
sinks, a `Projection`, and wires `StateHub` → the TUI bridge sees all events.

### 2. set_server_event_bus() defined but never called

`orchestrate.rs` line 5264 defines:
```rust
pub fn set_server_event_bus(&mut self, bus: BusSender<ServerEvent>) {
    self.server_event_bus = Some(bus);
}
```

This is **never called** from the serve path. The `emit_server_event()` method (line 5694)
checks `self.server_event_bus` and `self.state_hub_sender` — both are always `None` when
running via `roko serve`.

### 3. ACP is completely isolated

`roko-acp` has its own `CognitiveEvent` taxonomy (TokenChunk, ToolCallStart, etc.) in
`bridge_events.rs`. There are **zero** references to `StateHub`, `DashboardEvent`, or
`ServerEvent` in the entire `roko-acp` crate. ACP sessions communicate only with the
editor via stdio — events never reach the serve layer.

### 4. PTY terminal sessions are fire-and-forget

`roko-serve/src/terminal.rs` spawns shells via `portable-pty`. Output goes to a `tracing`
log target (`roko_serve::terminal::command_event`), not to the `ServerEvent` bus.
No `ROKO_SERVE_URL` or equivalent is injected into the spawned shell environment.

## Architecture Diagram

```
┌──────────────────────────────────────────────────────────────────────┐
│                         FRONTEND (demo-app)                          │
│   EventSource(/api/events/stream) ←── expects 50+ ServerEvent types  │
└───────────────────────────────────┬──────────────────────────────────┘
                                    │ SSE / WebSocket
┌───────────────────────────────────▼──────────────────────────────────┐
│                         roko-serve (:6677)                            │
│                                                                      │
│  AppState.state_hub ──→ SSE handler ──→ clients                      │
│  AppState.event_bus ──→ WS handler  ──→ clients                      │
│       ▲                                                              │
│       │ ✗ NEVER CONNECTED                                            │
│       │                                                              │
│  CliRuntime::run_plan ──→ build_runner_config()                      │
│       │                      feedback_facade: None                   │
│       │                      projection: None                        │
│       │                                                              │
│       └──→ runner::run() ──→ own SharedStateHub (disconnected)       │
│                 │                                                     │
│                 ├──→ orchestrate.rs emit_server_event()               │
│                 │        server_event_bus: None  ← never wired       │
│                 │        state_hub_sender: None  ← never wired       │
│                 │                                                     │
│                 └──→ TuiBridge.publish(DashboardEvent)                │
│                          → goes to disconnected hub → /dev/null       │
└──────────────────────────────────────────────────────────────────────┘

┌───────────────────────┐     ┌──────────────────────────────┐
│     roko-acp          │     │     terminal.rs (PTY)        │
│                       │     │                              │
│ CognitiveEvent ──→    │     │ CommandEvent ──→ tracing     │
│   stdio to editor     │     │   (no ServerEvent bus)       │
│   (no serve wiring)   │     │   (no ROKO_SERVE_URL env)    │
└───────────────────────┘     └──────────────────────────────┘
```

## Event Flow (current state)

| Source | Events reach serve bus? | Events reach frontend? |
|--------|------------------------|----------------------|
| Bench runs (inline in route handler) | YES | YES |
| Job operations (inline in route handler) | YES | YES |
| Template/deploy (inline in route handler) | YES | YES |
| Plan execution via `roko serve` | NO | NO |
| Plan execution via `roko plan run` (CLI) | N/A (no server) | N/A |
| ACP sessions | NO | NO |
| PTY terminal sessions | NO | NO |
| Agent dispatch (serve/dispatch.rs) | Partial (AgentOutput, GateResult only) | Partial |

## Key Files

| File | Role |
|------|------|
| `crates/roko-cli/src/serve_runtime.rs` | Where the gap lives — `build_runner_config()` |
| `crates/roko-cli/src/orchestrate.rs` | `emit_server_event()`, `set_server_event_bus()`, `set_state_hub()` |
| `crates/roko-cli/src/runner/tui_bridge.rs` | Where DashboardEvents are published from runner |
| `crates/roko-core/src/state_hub.rs` | SharedStateHub definition, publish logic |
| `crates/roko-serve/src/state.rs` | AppState.state_hub creation (line 528) |
| `crates/roko-serve/src/events.rs` | ServerEvent enum (50+ variants) |
| `crates/roko-acp/src/bridge_events.rs` | ACP's isolated CognitiveEvent system |
| `crates/roko-serve/src/terminal.rs` | PTY spawn (no event bus wiring) |
| `demo/demo-app/src/transport/types.ts` | Frontend event type definitions |
