# B8: WebSocket event enrichment for jobs and heartbeats

## Context

**Repo:** `/Users/will/dev/nunchi/roko/roko`
**Branch:** `demo-backend`
**Language:** Rust (workspace with ~29 crates)
**Key crate paths:**
- CLI + orchestrator: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/`
- Core types: `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/`
- HTTP server: `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/`
- Agent dispatch: `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/`

**Key files:**
- Orchestrator (20K lines): `crates/roko-cli/src/orchestrate.rs`
- CLI entry: `crates/roko-cli/src/main.rs`
- Server routes: `crates/roko-serve/src/routes/mod.rs`
- Server state: `crates/roko-serve/src/state.rs`
- Server events: `crates/roko-serve/src/events.rs`
- Server WS: `crates/roko-serve/src/routes/ws.rs`

**Architecture:**
- `roko-serve` is an axum HTTP server on port 6677 with ~85 REST routes + WebSocket
- `AppState` uses `tokio::sync::RwLock` -- all lock ops are `.read().await` / `.write().await` (NOT `.unwrap()`)
- Event bus: `state.event_bus.publish(event)` -- always present, no Option wrapping
- The TUI gets data two ways: (1) StateHub push via `watch<DashboardSnapshot>` channel, (2) file polling via `DashboardData::tick()` reading `.roko/` files

### Pre-commit (MANDATORY)

```bash
cargo +nightly fmt --all
cargo clippy --workspace --no-deps -- -D warnings
cargo test --workspace
```

## What this task does

Verify that the new `ServerEvent` variants from B2 (jobs) and B7 (heartbeats) flow through the existing WebSocket handler. This is primarily a verification and testing task -- the infrastructure is already in place.

## Why this should work already

The WebSocket handler at `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/ws.rs` works by:

1. Subscribing to `state.event_bus.subscribe()` (a broadcast channel).
2. Serializing each `ServerEvent` to JSON and sending it over the socket.
3. Filtering via the `matches_filter()` function based on the `"type"` serde tag.

Since B2 and B7 added new variants to `ServerEvent` with:
```rust
#[serde(tag = "type", rename_all = "snake_case")]
```
...they automatically get a snake_case `"type"` field when serialized. The WS handler serializes any `ServerEvent` it receives from the bus. Route handlers call `state.event_bus.publish(...)` on every state change.

Therefore: **no changes to ws.rs are needed.** The existing handler already forwards all `ServerEvent` variants.

## Expected serialization shapes

The following JSON shapes will be emitted on the WS channel:

```json
// JobCreated
{"type":"job_created","job_id":"job-abc","job_type":"research","title":"Research Uniswap v4"}

// JobStateChanged
{"type":"job_state_changed","job_id":"job-abc","old_state":"open","new_state":"assigned"}

// JobSubmitted
{"type":"job_submitted","job_id":"job-abc","agent_id":"runner-xyz"}

// JobEvaluated
{"type":"job_evaluated","job_id":"job-abc","accepted":true}

// HeartbeatReceived
{"type":"heartbeat_received","sender_id":"orchestrator-abc","active_tasks":3,"active_agents":2}
```

## Steps

- [ ] **Verify serialization of new variants.** Add a test to `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/events.rs` in the existing `#[cfg(test)] mod tests` block:

```rust
    #[test]
    fn job_events_serialize_with_type_tag() {
        // JobCreated
        let event = ServerEvent::JobCreated {
            job_id: "job-123".into(),
            job_type: "research".into(),
            title: "Test research".into(),
        };
        let json = serde_json::to_value(&event).expect("serialize job created");
        assert_eq!(json["type"], "job_created", "type tag must be snake_case");
        assert_eq!(json["job_id"], "job-123");
        assert_eq!(json["job_type"], "research");
        assert_eq!(json["title"], "Test research");

        // JobStateChanged
        let event = ServerEvent::JobStateChanged {
            job_id: "job-123".into(),
            old_state: "open".into(),
            new_state: "assigned".into(),
        };
        let json = serde_json::to_value(&event).expect("serialize job state changed");
        assert_eq!(json["type"], "job_state_changed");
        assert_eq!(json["old_state"], "open");
        assert_eq!(json["new_state"], "assigned");

        // JobSubmitted
        let event = ServerEvent::JobSubmitted {
            job_id: "job-123".into(),
            agent_id: "agent-1".into(),
        };
        let json = serde_json::to_value(&event).expect("serialize job submitted");
        assert_eq!(json["type"], "job_submitted");
        assert_eq!(json["agent_id"], "agent-1");

        // JobEvaluated
        let event = ServerEvent::JobEvaluated {
            job_id: "job-123".into(),
            accepted: true,
        };
        let json = serde_json::to_value(&event).expect("serialize job evaluated");
        assert_eq!(json["type"], "job_evaluated");
        assert_eq!(json["accepted"], true);
    }

    #[test]
    fn heartbeat_event_serializes_with_type_tag() {
        let event = ServerEvent::HeartbeatReceived {
            sender_id: "orchestrator-abc".into(),
            active_tasks: 5,
            active_agents: 2,
        };
        let json = serde_json::to_value(&event).expect("serialize heartbeat");
        assert_eq!(json["type"], "heartbeat_received");
        assert_eq!(json["sender_id"], "orchestrator-abc");
        assert_eq!(json["active_tasks"], 5);
        assert_eq!(json["active_agents"], 2);
    }

    #[test]
    fn all_server_event_variants_have_type_tag() {
        // Ensure every known variant produces a "type" field when serialized.
        // This test must be kept in sync as new variants are added.
        let events: Vec<ServerEvent> = vec![
            ServerEvent::PlanStarted { plan_id: "p1".into() },
            ServerEvent::PlanCompleted { plan_id: "p1".into(), success: true },
            ServerEvent::JobCreated {
                job_id: "j1".into(),
                job_type: "research".into(),
                title: "t".into(),
            },
            ServerEvent::JobStateChanged {
                job_id: "j1".into(),
                old_state: "open".into(),
                new_state: "assigned".into(),
            },
            ServerEvent::JobSubmitted {
                job_id: "j1".into(),
                agent_id: "a1".into(),
            },
            ServerEvent::JobEvaluated {
                job_id: "j1".into(),
                accepted: true,
            },
            ServerEvent::HeartbeatReceived {
                sender_id: "s1".into(),
                active_tasks: 0,
                active_agents: 0,
            },
            ServerEvent::ServerShutdown,
        ];

        for event in &events {
            let json = serde_json::to_value(event).expect("serialize");
            assert!(
                json.get("type").is_some(),
                "event variant missing 'type' field: {json}"
            );
            let type_str = json["type"].as_str().unwrap_or("");
            assert!(
                !type_str.is_empty(),
                "event 'type' field must not be empty: {json}"
            );
            assert!(
                type_str.chars().all(|c| c.is_ascii_lowercase() || c == '_'),
                "'type' field must be snake_case, got: {type_str}"
            );
        }
    }
```

- [ ] **Verify WS filter matching works for new events.** The `matches_filter` function in ws.rs uses the `"type"` field for matching. Confirm that clients can filter on the new event types. No code changes are needed -- just verify the behavior:

  - Filter `"job"` (substring) matches `"job_created"`, `"job_state_changed"`, `"job_submitted"`, `"job_evaluated"`.
  - Filter `"heartbeat"` matches `"heartbeat_received"`.
  - Filter `""` (empty) matches all events.

- [ ] **Read event_bus.rs to confirm the flow.** Open `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/event_bus.rs` and verify:

  1. Route handler calls `state.event_bus.publish(ServerEvent::JobCreated { ... })`.
  2. `EventBus::publish()` wraps it in an `Envelope` with a sequence number.
  3. Sends via `broadcast::Sender::send()`.
  4. WS handler receives via `rx.recv()` (which is `broadcast::Receiver`).
  5. Serializes to JSON and sends over the socket.

  No changes needed. Document the flow in a code comment near `EventBus::publish()` if it is not already documented.

- [ ] **Manual WS verification using websocat.** After the server is running:

  ```bash
  # Install websocat if not present:
  # cargo install websocat
  # or: brew install websocat

  # Terminal 1: start the server
  cargo run -p roko-cli -- serve

  # Terminal 2: connect to the WS and subscribe to job + heartbeat events
  websocat ws://localhost:6677/ws
  # After connection, send a subscription message:
  {"subscribe":["job","heartbeat"]}

  # Terminal 3: POST a job and a heartbeat to trigger WS events
  curl -s -X POST http://localhost:6677/api/jobs \
    -H 'Content-Type: application/json' \
    -d '{"title":"WS test job","description":"verify ws delivery","job_type":"research"}' | jq .

  curl -s -X POST http://localhost:6677/api/heartbeats \
    -H 'Content-Type: application/json' \
    -d '{"sender_id":"ws-test","timestamp":"2026-04-21T00:00:00Z","active_tasks":1}' | jq .

  # Terminal 2 should show:
  # {"type":"job_created","job_id":"job-...","job_type":"research","title":"WS test job"}
  # {"type":"heartbeat_received","sender_id":"ws-test","active_tasks":1,"active_agents":0}
  ```

- [ ] **Alternative: use wscat if websocat is not available.**

  ```bash
  npm install -g wscat
  wscat -c ws://localhost:6677/ws
  # In wscat, type: {"subscribe":["job","heartbeat"]}
  ```

- [ ] **Document the subscription protocol.** Any WS client can filter for specific event types:

  ```json
  // Subscribe to all job and heartbeat events:
  { "subscribe": ["job_created", "job_state_changed", "job_submitted", "job_evaluated", "heartbeat_received"] }

  // Or use substring matching (if the server supports it):
  { "subscribe": ["job", "heartbeat"] }

  // Or receive everything:
  { "subscribe": [] }
  ```

## Verification

```bash
cd /Users/will/dev/nunchi/roko/roko

# Run the event serialization tests
cargo test -p roko-serve -- events:: --nocapture

# Run WS tests if any
cargo test -p roko-serve -- ws:: --nocapture

# Full workspace test
cargo test --workspace 2>&1 | tail -30

# Clippy
cargo clippy --workspace --no-deps -- -D warnings 2>&1 | head -20

# Format check
cargo +nightly fmt --all -- --check
```

Expected: all three new serialization tests pass, the `all_server_event_variants_have_type_tag` test passes, WS delivers events to connected clients.
