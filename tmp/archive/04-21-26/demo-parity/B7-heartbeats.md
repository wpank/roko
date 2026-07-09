# B7: Heartbeat protocol (wire types + server routes + orchestrator emission)

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

Three pieces:
1. **Wire types** in `roko-core` for the heartbeat HTTP payload (distinct from the CLI-local `HeartbeatSnapshot` at `crates/roko-cli/src/heartbeat.rs`)
2. **Server routes** in `roko-serve` to receive and query heartbeats
3. **Orchestrator emission** from `orchestrate.rs` to POST heartbeats at a configurable interval

## Important: no file collision

- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/heartbeat.rs` ALREADY EXISTS with `HeartbeatClock`, `HeartbeatSnapshot`, `HeartbeatProbeKind` etc.
- The new file is at `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/heartbeat.rs` -- different crate, no collision.
- The core types define the HTTP wire format. The CLI types define the local scheduling. They are complementary.

## Steps

- [ ] **Create wire types in roko-core.** Create `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/heartbeat.rs`:

```rust
//! Heartbeat wire types for the HTTP control plane.
//!
//! These types define the payload shape for `POST /api/heartbeats` and
//! `GET /api/heartbeats`. They are separate from the CLI-side
//! `HeartbeatClock` / `HeartbeatSnapshot` which handle local scheduling.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Default heartbeat emission interval in seconds.
///
/// Used by the orchestrator when no override is configured.
pub const DEFAULT_HEARTBEAT_INTERVAL_SECS: u64 = 30;

/// Capacity of the server-side heartbeat ring buffer.
pub const HEARTBEAT_RING_CAPACITY: usize = 1000;

/// Payload POSTed to the server on each heartbeat tick.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HeartbeatPayload {
    /// Identifier of the sender (orchestrator instance, agent, etc.).
    pub sender_id: String,
    /// Wall-clock timestamp.
    pub timestamp: DateTime<Utc>,
    /// Number of active tasks being worked on.
    #[serde(default)]
    pub active_tasks: usize,
    /// Number of completed tasks in the current session.
    #[serde(default)]
    pub completed_tasks: usize,
    /// Number of failed tasks in the current session.
    #[serde(default)]
    pub failed_tasks: usize,
    /// Number of active agent processes.
    #[serde(default)]
    pub active_agents: usize,
    /// Cumulative spend in USD for the session.
    #[serde(default)]
    pub session_spend_usd: f64,
    /// Which operating frequency triggered this heartbeat (gamma/theta/delta).
    #[serde(default)]
    pub frequency: String,
    /// Freeform metrics (e.g. `"completion_rate": 0.75`).
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub metrics: HashMap<String, f64>,
    /// Triggered probe labels for diagnostics.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub triggered_probes: Vec<String>,
}

/// Response from `GET /api/network/stats`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkStats {
    /// Number of heartbeats received since server start.
    pub total_heartbeats: usize,
    /// Number of unique senders.
    pub unique_senders: usize,
    /// Per-sender summaries, sorted by last-seen descending.
    pub senders: Vec<SenderInfo>,
}

/// Per-sender summary in network stats.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SenderInfo {
    /// Sender identifier.
    pub sender_id: String,
    /// Last heartbeat timestamp.
    pub last_seen: DateTime<Utc>,
    /// Total heartbeats from this sender.
    pub count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heartbeat_payload_partial_eq() {
        let ts = Utc::now();
        let a = HeartbeatPayload {
            sender_id: "orch-1".into(),
            timestamp: ts,
            active_tasks: 3,
            completed_tasks: 10,
            failed_tasks: 1,
            active_agents: 2,
            session_spend_usd: 0.05,
            frequency: "theta".into(),
            metrics: HashMap::new(),
            triggered_probes: vec![],
        };
        let b = a.clone();
        assert_eq!(a, b);
    }

    #[test]
    fn heartbeat_payload_round_trips() {
        let ts = chrono::DateTime::parse_from_rfc3339("2026-04-21T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let payload = HeartbeatPayload {
            sender_id: "test".into(),
            timestamp: ts,
            active_tasks: 5,
            completed_tasks: 0,
            failed_tasks: 0,
            active_agents: 1,
            session_spend_usd: 0.0,
            frequency: "gamma".into(),
            metrics: HashMap::new(),
            triggered_probes: vec![],
        };
        let json = serde_json::to_string(&payload).expect("serialize");
        let decoded: HeartbeatPayload = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(payload, decoded);
    }

    #[test]
    fn empty_metrics_omitted_from_json() {
        let ts = Utc::now();
        let payload = HeartbeatPayload {
            sender_id: "test".into(),
            timestamp: ts,
            active_tasks: 0,
            completed_tasks: 0,
            failed_tasks: 0,
            active_agents: 0,
            session_spend_usd: 0.0,
            frequency: String::new(),
            metrics: HashMap::new(),
            triggered_probes: vec![],
        };
        let json = serde_json::to_string(&payload).expect("serialize");
        assert!(!json.contains("metrics"), "empty metrics should be omitted");
    }
}
```

- [ ] **Register in roko-core lib.rs.** Open `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/lib.rs`. Add `pub mod heartbeat;` after `pub mod hash;` (line 75). Add re-exports:
  ```rust
  pub use heartbeat::{
      HeartbeatPayload, NetworkStats, SenderInfo,
      DEFAULT_HEARTBEAT_INTERVAL_SECS, HEARTBEAT_RING_CAPACITY,
  };
  ```

- [ ] **Add heartbeat storage to AppState.** Open `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/state.rs`.

  Add imports (check first -- `VecDeque` may already be imported):
  ```rust
  use std::collections::VecDeque;
  use roko_core::heartbeat::{HeartbeatPayload, HEARTBEAT_RING_CAPACITY};
  ```

  Add field to `AppState` after `aggregator_cache`:
  ```rust
      /// Recent heartbeats, ring-buffered to at most `HEARTBEAT_RING_CAPACITY` entries.
      pub heartbeats: RwLock<VecDeque<HeartbeatPayload>>,
  ```

  In `AppState::new_with_daimon_strategy()`, in the `Self { ... }` block, add:
  ```rust
              heartbeats: RwLock::new(VecDeque::with_capacity(HEARTBEAT_RING_CAPACITY)),
  ```

- [ ] **Add ServerEvent variant for heartbeats.** Open `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/events.rs`. Add before `ServerShutdown`:
  ```rust
      /// A heartbeat was received from an orchestrator or agent.
      HeartbeatReceived {
          sender_id: String,
          active_tasks: usize,
          active_agents: usize,
      },
  ```

- [ ] **Create heartbeat routes.** Create `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/heartbeats.rs`:

```rust
//! Heartbeat API routes.
//!
//! `POST /api/heartbeats` -- accept a heartbeat from an orchestrator or agent.
//! `GET  /api/heartbeats` -- list recent heartbeats (most recent first).
//! `GET  /api/network/stats` -- aggregate network statistics.

use std::collections::HashMap;
use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::{json, Value};

use roko_core::heartbeat::{
    HeartbeatPayload, NetworkStats, SenderInfo, HEARTBEAT_RING_CAPACITY,
};

use crate::error::ApiError;
use crate::events::ServerEvent;
use crate::state::AppState;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/heartbeats", post(receive_heartbeat).get(list_heartbeats))
        .route("/network/stats", get(network_stats))
}

/// `POST /api/heartbeats` -- accept and store a heartbeat.
async fn receive_heartbeat(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<HeartbeatPayload>,
) -> Result<impl IntoResponse, ApiError> {
    let sender_id = payload.sender_id.clone();
    let active_tasks = payload.active_tasks;
    let active_agents = payload.active_agents;

    {
        let mut hb = state.heartbeats.write().await;
        if hb.len() >= HEARTBEAT_RING_CAPACITY {
            hb.pop_front();
        }
        hb.push_back(payload);
    }

    state.event_bus.publish(ServerEvent::HeartbeatReceived {
        sender_id: sender_id.clone(),
        active_tasks,
        active_agents,
    });

    Ok((
        StatusCode::ACCEPTED,
        Json(json!({ "status": "ok", "sender_id": sender_id })),
    ))
}

/// `GET /api/heartbeats` -- list recent heartbeats (most recent first, limit 100).
async fn list_heartbeats(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Value>, ApiError> {
    let hb = state.heartbeats.read().await;
    let items: Vec<&HeartbeatPayload> = hb.iter().rev().take(100).collect();
    Ok(Json(json!({
        "heartbeats": items,
        "count": items.len(),
        "total_stored": hb.len(),
    })))
}

/// `GET /api/network/stats` -- aggregate heartbeat statistics in a single pass.
async fn network_stats(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Value>, ApiError> {
    let hb = state.heartbeats.read().await;

    // Single-pass aggregation: track (last_seen, count) per sender.
    let mut sender_map: HashMap<String, (chrono::DateTime<chrono::Utc>, usize)> = HashMap::new();
    for beat in hb.iter() {
        let entry = sender_map
            .entry(beat.sender_id.clone())
            .or_insert((beat.timestamp, 0));
        if beat.timestamp > entry.0 {
            entry.0 = beat.timestamp;
        }
        entry.1 += 1;
    }

    let mut senders: Vec<SenderInfo> = sender_map
        .into_iter()
        .map(|(sender_id, (last_seen, count))| SenderInfo {
            sender_id,
            last_seen,
            count,
        })
        .collect();

    // Sort by last_seen descending so the most recently active sender is first.
    senders.sort_by(|a, b| b.last_seen.cmp(&a.last_seen));

    let stats = NetworkStats {
        total_heartbeats: hb.len(),
        unique_senders: senders.len(),
        senders,
    };

    Ok(Json(
        serde_json::to_value(stats).unwrap_or_else(|_| json!({})),
    ))
}
```

- [ ] **Wire into mod.rs.** Open `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/mod.rs`. Add `mod heartbeats;` among the other module declarations. In `build_router()`, add `.merge(heartbeats::routes())` in the `api` builder.

- [ ] **Emit heartbeats from orchestrate.rs.** Open `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/orchestrate.rs`. Find the main loop. Read `run_all()` to understand which variables track active/completed/failed tasks.

  Add near the top of the file:
  ```rust
  use roko_core::heartbeat::{HeartbeatPayload, DEFAULT_HEARTBEAT_INTERVAL_SECS};
  ```

  Before the main loop, add:
  ```rust
  let heartbeat_client = reqwest::Client::new();
  let heartbeat_sender_id = format!("orchestrator-{}", &uuid::Uuid::new_v4().to_string()[..8]);
  let heartbeat_interval = std::time::Duration::from_secs(DEFAULT_HEARTBEAT_INTERVAL_SECS);
  // MOCK: make configurable via roko.toml [orchestrator.heartbeat_interval_secs]
  let mut last_heartbeat = std::time::Instant::now();
  ```

  Inside the loop body, after other periodic checks:
  ```rust
  // Fire-and-forget heartbeat on each interval tick.
  // Failure is intentionally ignored: if roko-serve is not running, the
  // orchestrator must not crash.
  if last_heartbeat.elapsed() >= heartbeat_interval {
      last_heartbeat = std::time::Instant::now();
      let payload = HeartbeatPayload {
          sender_id: heartbeat_sender_id.clone(),
          timestamp: chrono::Utc::now(),
          active_tasks: 0,        // MOCK: derive from executor state
          completed_tasks: 0,     // MOCK: derive from executor state
          failed_tasks: 0,        // MOCK: derive from executor state
          active_agents: 0,       // MOCK: derive from process supervisor
          session_spend_usd: 0.0, // MOCK: derive from plan_costs
          frequency: "theta".to_string(), // MOCK: derive from heartbeat clock
          metrics: std::collections::HashMap::new(),
          triggered_probes: Vec::new(),
      };
      let client = heartbeat_client.clone();
      tokio::spawn(async move {
          let url = "http://localhost:6677/api/heartbeats"; // MOCK: make configurable
          if let Err(e) = client.post(url).json(&payload).send().await {
              tracing::debug!("heartbeat POST failed (non-fatal): {e}");
          }
      });
  }
  ```

  NOTE: Replace the zero values with real executor state. Each `// MOCK:` comment indicates a wire-up point.

## Verification

```bash
cd /Users/will/dev/nunchi/roko/roko

# Compile all affected crates
cargo check -p roko-core -p roko-serve -p roko-cli 2>&1 | head -30

# Run core heartbeat tests
cargo test -p roko-core -- heartbeat --nocapture

# Run serve tests
cargo test -p roko-serve 2>&1 | tail -20

# Clippy across workspace
cargo clippy --workspace --no-deps -- -D warnings 2>&1 | head -20

# Format check
cargo +nightly fmt --all -- --check

# Smoke test (requires a running server):
# Terminal 1: cargo run -p roko-cli -- serve
# Terminal 2:
#   curl -s -X POST http://localhost:6677/api/heartbeats \
#     -H 'Content-Type: application/json' \
#     -d '{"sender_id":"test","timestamp":"2026-04-21T00:00:00Z","active_tasks":3}' | jq .
#   # Expected: {"status":"ok","sender_id":"test"} with HTTP 202
#
#   curl -s http://localhost:6677/api/heartbeats | jq .count
#   # Expected: 1
#
#   curl -s http://localhost:6677/api/network/stats | jq .
#   # Expected: unique_senders=1, senders[0].sender_id="test"
```

Expected: heartbeat POST returns 202, GET returns stored heartbeats, network stats aggregates by sender in a single pass, tests pass.
