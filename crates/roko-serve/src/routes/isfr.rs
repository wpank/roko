//! ISFR REST API endpoints — keeper status, current rate, history, sources, SSE stream.
//!
//! Endpoints:
//!   GET /api/isfr/status   — keeper running flag, config params, counts
//!   GET /api/isfr/current  — most recent composite rate (or 204-style JSON hint)
//!   GET /api/isfr/history  — bounded ring of historical rates (?limit=N, max 256)
//!   GET /api/isfr/sources  — per-source health snapshots
//!   GET /api/isfr/stream   — SSE stream filtered to ISFR events only

use std::convert::Infallible;
use std::sync::Arc;

use axum::Json;
use axum::Router;
use axum::extract::{Query, State};
use axum::response::IntoResponse;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::routing::get;
use futures::stream::{self, StreamExt};
use roko_core::DashboardEvent;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use tracing::warn;

use crate::error::ApiError;
use crate::routes::sse::sse_response_headers;
use crate::state::AppState;

/// Register all ISFR routes. Called from `build_router()` via `.merge(isfr::routes())`.
pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/isfr/status", get(isfr_status))
        .route("/isfr/current", get(isfr_current_rate))
        .route("/isfr/history", get(isfr_rate_history))
        .route("/isfr/sources", get(isfr_sources))
        .route("/isfr/stream", get(isfr_stream))
}

// ─── GET /api/isfr/status ────────────────────────────────────────────────────

#[derive(Serialize)]
struct ISFRStatusResponse {
    /// Whether ISFR features are enabled in roko.toml.
    enabled: bool,
    /// Whether the keeper background task is currently running.
    keeper_running: bool,
    /// Number of source health entries tracked.
    sources_count: usize,
    /// Most recent composite rate in basis points (null when no rate yet).
    current_rate_bps: Option<u64>,
    /// Confidence as a 0.0–1.0 fraction (null when no rate yet).
    current_confidence: Option<f64>,
    /// Current keeper epoch number.
    current_epoch: u64,
    /// Source poll interval from config (seconds).
    poll_interval_secs: u64,
    /// Epoch duration from config (seconds).
    epoch_duration_secs: u64,
    /// ISFROracle contract address (if deployed).
    oracle_address: Option<String>,
}

async fn isfr_status(
    State(state): State<Arc<AppState>>,
) -> Result<Json<ISFRStatusResponse>, ApiError> {
    // load_roko_config() is sync — returns Arc<RokoConfig>.
    let config = state.load_roko_config();

    let current = state.isfr.current_rate.read().await;
    let sources = state.isfr.sources.read().await;
    let running = state
        .isfr
        .keeper_running
        .load(std::sync::atomic::Ordering::Relaxed);

    let epoch = state
        .isfr
        .current_epoch
        .load(std::sync::atomic::Ordering::Relaxed);
    let oracle_addr = state
        .isfr
        .contract_addresses
        .read()
        .await
        .as_ref()
        .and_then(|c| c.isfr_oracle.clone());

    Ok(Json(ISFRStatusResponse {
        enabled: config.isfr.enabled,
        keeper_running: running,
        sources_count: sources.len(),
        current_rate_bps: current.as_ref().map(|r| r.composite_bps),
        // confidence_bps is 0–10000 (basis points of confidence), convert to 0.0–1.0
        current_confidence: current.as_ref().map(|r| r.confidence_bps as f64 / 10_000.0),
        current_epoch: epoch,
        poll_interval_secs: config.isfr.poll_interval_secs,
        epoch_duration_secs: config.isfr.epoch_duration_secs,
        oracle_address: oracle_addr,
    }))
}

// ─── GET /api/isfr/current ───────────────────────────────────────────────────

async fn isfr_current_rate(
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let current = state.isfr.current_rate.read().await;
    match current.as_ref() {
        Some(rate) => Ok(Json(
            serde_json::to_value(rate).unwrap_or(serde_json::Value::Null),
        )),
        None => Ok(Json(serde_json::json!({
            "error": "no rate computed yet",
            "hint": "start the keeper with `roko isfr start`"
        }))),
    }
}

// ─── GET /api/isfr/history?limit=N ──────────────────────────────────────────

#[derive(Deserialize)]
struct HistoryQuery {
    /// Maximum number of history entries to return (default: 50, cap: 256).
    limit: Option<usize>,
}

async fn isfr_rate_history(
    State(state): State<Arc<AppState>>,
    Query(q): Query<HistoryQuery>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let history = state.isfr.rate_history.read().await;
    let limit = q.limit.unwrap_or(50).min(256);
    // Most recent first: iterate in reverse, collect up to `limit` entries.
    let rates: Vec<_> = history.iter().rev().take(limit).collect();
    // Return the array directly so the frontend can call history.map(...).
    Ok(Json(
        serde_json::to_value(&rates).unwrap_or(serde_json::Value::Array(vec![])),
    ))
}

// ─── GET /api/isfr/sources ───────────────────────────────────────────────────

async fn isfr_sources(
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let sources = state.isfr.sources.read().await;
    // Return the array directly so the frontend can call sources.map(...).
    Ok(Json(
        serde_json::to_value(&*sources).unwrap_or(serde_json::Value::Array(vec![])),
    ))
}

// ─── GET /api/isfr/stream (SSE) ─────────────────────────────────────────────

/// Returns `true` for the three ISFR `DashboardEvent` variants; everything
/// else is suppressed so this endpoint does not become a duplicate dashboard
/// stream.
fn is_isfr_event(event: &DashboardEvent) -> bool {
    matches!(
        event,
        DashboardEvent::IsfrRateComputed { .. }
            | DashboardEvent::IsfrSourceHealthChanged { .. }
            | DashboardEvent::IsfrKeeperStateChanged { .. }
    )
}

/// `GET /api/isfr/stream` — SSE stream filtered to ISFR events only.
///
/// Reuses the same `StateHub` broadcast channel that powers `/api/events`, but
/// emits only `IsfrRateComputed`, `IsfrSourceHealthChanged`, and
/// `IsfrKeeperStateChanged` frames. Keepalive and no-buffer headers match the
/// main SSE endpoint.
async fn isfr_stream(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    // Subscribe from the current head — no replay for the ISFR-filtered stream;
    // clients that need historical data use the REST endpoints above.
    let subscription = state.state_hub.subscribe_events_from(0);
    let scrubber = Arc::clone(&state.scrubber);

    // Replay: filter retained events to ISFR variants only.
    let replay: Vec<Result<Event, Infallible>> = subscription
        .replay
        .into_iter()
        .filter(|e| is_isfr_event(&e.payload))
        .map(|e| Ok(isfr_sse_event(e, &scrubber)))
        .collect();

    let live_floor = subscription.cursor.next_seq;
    let live = stream::unfold(
        (subscription.live, live_floor, scrubber),
        |(mut rx, mut floor, scrubber)| async move {
            loop {
                match rx.recv().await {
                    Ok(envelope) => {
                        if envelope.seq < floor {
                            continue;
                        }
                        floor = envelope.seq.saturating_add(1);
                        if !is_isfr_event(&envelope.payload) {
                            continue;
                        }
                        return Some((
                            Ok::<_, Infallible>(isfr_sse_event(envelope, &scrubber)),
                            (rx, floor, scrubber),
                        ));
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        warn!(n, "ISFR SSE client lagged; skipping missed events");
                        // Unlike the main SSE stream we do not send a gap/snapshot
                        // frame — ISFR clients recover via the REST endpoints.
                        continue;
                    }
                    Err(broadcast::error::RecvError::Closed) => return None,
                }
            }
        },
    );

    let sse = Sse::new(stream::iter(replay).chain(live)).keep_alive(
        KeepAlive::new()
            .interval(std::time::Duration::from_secs(8))
            .text("keepalive"),
    );

    (sse_response_headers(), sse)
}

fn isfr_sse_event(
    envelope: roko_runtime::event_bus::Envelope<DashboardEvent>,
    scrubber: &roko_core::obs::LogScrubber,
) -> Event {
    let data = scrubber.scrub(&serde_json::to_string(&envelope.payload).unwrap_or_default());
    Event::default().data(data).id(envelope.seq.to_string())
}
