# F1: Add ISFR REST API Endpoints to roko-serve

## Context

The demo-app needs REST endpoints to query ISFR keeper status, current rates, historical rates, and source health. These are consumed by the frontend tile (F4) via `getJson("roko", "/isfr/...")`.

## File to Create

- `crates/roko-serve/src/routes/isfr.rs` (NEW)

## Files to Modify

- `crates/roko-serve/src/routes/mod.rs` — add `mod isfr;` + `.merge(isfr::routes())`
- `crates/roko-serve/src/state.rs` — add optional `ISFRState` to `AppState`
- `crates/roko-serve/src/events.rs` — add ISFR event variants to `ServerEvent`

## Pre-Check

```bash
# See existing route pattern
grep -n "pub fn routes\(\)" crates/roko-serve/src/routes/*.rs | head -10
# See how routes are registered
grep -n "merge(" crates/roko-serve/src/routes/mod.rs | tail -10
# See AppState fields
grep -n "pub struct AppState" crates/roko-serve/src/state.rs
```

## Implementation

### Step 1: Add ISFRState to AppState

In `crates/roko-serve/src/state.rs`, add:

**Note**: Use `roko_chain::isfr_sources::CompositeRate` (from C1, not C2 — the type is
defined in the isfr_sources module). The keeper re-exports it. If roko-serve doesn't
depend on roko-chain, add it to Cargo.toml:
```toml
roko-chain = { path = "../roko-chain" }
```

```rust
use tokio::sync::RwLock;
use roko_chain::isfr_sources::CompositeRate;

/// Shared ISFR keeper state exposed via API.
#[derive(Debug, Default)]
pub struct ISFRState {
    /// Most recent composite rate.
    pub current_rate: RwLock<Option<CompositeRate>>,
    /// Historical rates (bounded ring, last 256).
    pub rate_history: RwLock<Vec<CompositeRate>>,
    /// Source health snapshots.
    pub sources: RwLock<Vec<ISFRSourceSnapshot>>,
    /// Whether keeper is running.
    pub keeper_running: std::sync::atomic::AtomicBool,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ISFRSourceSnapshot {
    pub id: String,
    pub name: String,
    pub class: String,
    pub weight: f64,
    pub last_rate_bps: Option<u64>,
    pub health: String,
    pub last_poll_ms: Option<i64>,
}
```

Add to `AppState`. Wrap in `Arc` so it can be cloned into the PublishFn closure:
```rust
pub struct AppState {
    // ... existing fields ...
    pub isfr: Arc<ISFRState>,
}
```

In `AppState::new()` or wherever it's constructed, initialize:
```rust
isfr: Arc::new(ISFRState::default()),
```

### Step 2: Create `crates/roko-serve/src/routes/isfr.rs`

```rust
//! ISFR API endpoints — keeper status, rates, sources.

use std::sync::Arc;
use axum::extract::State;
use axum::routing::get;
use axum::{Json, Router};
use serde::Serialize;

use crate::error::ApiError;
use crate::state::AppState;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/isfr/status", get(isfr_status))
        .route("/isfr/current", get(isfr_current_rate))
        .route("/isfr/history", get(isfr_rate_history))
        .route("/isfr/sources", get(isfr_sources))
}

// ─── GET /api/isfr/status ─────────────────────────────────────

#[derive(Serialize)]
struct ISFRStatusResponse {
    enabled: bool,
    keeper_running: bool,
    sources_count: usize,
    current_rate_bps: Option<u64>,
    current_confidence: Option<f64>,
    current_epoch: Option<u64>,
    poll_interval_secs: u64,
    epoch_duration_secs: u64,
}

async fn isfr_status(
    State(state): State<Arc<AppState>>,
) -> Result<Json<ISFRStatusResponse>, ApiError> {
    // NOTE: load_roko_config() is SYNC (returns Arc<RokoConfig>, not a future).
    let config = state.load_roko_config();

    let current = state.isfr.current_rate.read().await;
    let sources = state.isfr.sources.read().await;
    let running = state.isfr.keeper_running.load(std::sync::atomic::Ordering::Relaxed);

    Ok(Json(ISFRStatusResponse {
        enabled: config.isfr.enabled,
        keeper_running: running,
        sources_count: sources.len(),
        current_rate_bps: current.as_ref().map(|r| r.composite_bps),
        current_confidence: current.as_ref().map(|r| r.confidence_bps as f64 / 10_000.0),
        current_epoch: None, // ISFRKeeper doesn't track epoch yet
        poll_interval_secs: config.isfr.poll_interval_secs,
        epoch_duration_secs: config.isfr.epoch_duration_secs,
    }))
}

// ─── GET /api/isfr/current ────────────────────────────────────

async fn isfr_current_rate(
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let current = state.isfr.current_rate.read().await;
    match current.as_ref() {
        Some(rate) => Ok(Json(serde_json::to_value(rate).unwrap_or_default())),
        None => Ok(Json(serde_json::json!({
            "error": "no rate computed yet",
            "hint": "start the keeper with `roko isfr start`"
        }))),
    }
}

// ─── GET /api/isfr/history?limit=N ────────────────────────────

#[derive(serde::Deserialize)]
struct HistoryQuery {
    limit: Option<usize>,
}

async fn isfr_rate_history(
    State(state): State<Arc<AppState>>,
    axum::extract::Query(q): axum::extract::Query<HistoryQuery>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let history = state.isfr.rate_history.read().await;
    let limit = q.limit.unwrap_or(50).min(256);
    let rates: Vec<_> = history.iter().rev().take(limit).collect();
    Ok(Json(serde_json::json!({ "rates": rates, "total": history.len() })))
}

// ─── GET /api/isfr/sources ────────────────────────────────────

async fn isfr_sources(
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let sources = state.isfr.sources.read().await;
    Ok(Json(serde_json::json!({ "sources": *sources })))
}
```

### Step 3: Register routes

In `crates/roko-serve/src/routes/mod.rs`:

```rust
mod isfr;
```

And in the `build_router()` function's merge chain:
```rust
.merge(isfr::routes())
```

### Step 4: Add ISFR event variants

In `crates/roko-serve/src/events.rs`, add to the `ServerEvent` enum.

**Field alignment**: These fields match CompositeRate from C1 (flat, all u64 bps values).
The frontend IsfrRateEvent type must match this shape.

```rust
/// New ISFR composite rate computed by keeper.
IsfrRateComputed {
    composite_bps: u64,
    lending_bps: u64,
    structured_bps: u64,
    funding_bps: u64,
    staking_bps: u64,
    confidence_bps: u64,   // 0–10000 (not f64)
    source_count: usize,
    timestamp_ms: i64,
},

/// ISFR source health changed.
IsfrSourceHealthChanged {
    source_id: String,
    health: String,
    last_rate_bps: Option<u64>,
},

/// ISFR keeper started/stopped.
IsfrKeeperStateChanged {
    running: bool,
},
```

### Step 5: Wire keeper publish to AppState

When the keeper computes a rate, it should update `AppState.isfr`. This is done via the `PublishFn` callback set on the keeper (from E2's `isfr start` command).

**Critical notes**:
- `PublishFn` is `Arc<dyn Fn(&str, &str, serde_json::Value) + Send + Sync>` — it's a sync closure.
- `event_bus.publish()` is the correct method (NOT `.emit()`) — verify with `grep "fn publish" crates/roko-serve/src/`.
- `ISFRState` uses `tokio::sync::RwLock`, so async writes must happen inside a spawned task.
- `CompositeRate` from C2 has flat fields: `composite_bps`, `lending_bps`, `structured_bps`, `funding_bps`, `staking_bps`, `confidence_bps`, `timestamp_ms`, `readings`.

```rust
// In the CLI or wherever keeper is started with access to AppState:
let isfr_state = Arc::clone(&app_state.isfr);
let event_bus = app_state.event_bus.clone();

let publish_fn: PublishFn = Arc::new(move |_topic, msg_type, payload| {
    if msg_type == "rate_update" {
        if let Ok(rate) = serde_json::from_value::<CompositeRate>(payload.clone()) {
            // Update current rate.
            let isfr = isfr_state.clone();
            tokio::spawn(async move {
                *isfr.current_rate.write().await = Some(rate.clone());
                let mut history = isfr.rate_history.write().await;
                history.push(rate.clone());
                if history.len() > 256 {
                    history.remove(0);
                }
            });

            // Publish event for SSE/WS subscribers.
            let event = ServerEvent::IsfrRateComputed {
                composite_bps: rate.composite_bps,
                lending_bps: rate.lending_bps,
                structured_bps: rate.structured_bps,
                funding_bps: rate.funding_bps,
                staking_bps: rate.staking_bps,
                confidence_bps: rate.confidence_bps,
                source_count: rate.readings.len(),
                timestamp_ms: rate.timestamp_ms as i64,
            };
            event_bus.publish(event);
        }
    }
});
keeper.set_publish_fn(publish_fn);
```

## Verification

```bash
cargo build -p roko-serve
cargo test -p roko-serve
# Start serve + keeper, then:
# curl http://localhost:6677/api/isfr/status
# curl http://localhost:6677/api/isfr/current
# curl http://localhost:6677/api/isfr/history?limit=5
# curl http://localhost:6677/api/isfr/sources
```

## Dependencies

- C2 (CompositeRate type)
- E1 (ISFRSection in config)
- E2 (keeper start wiring — for the publish_fn hookup)
