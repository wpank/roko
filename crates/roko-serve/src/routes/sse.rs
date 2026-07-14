//! SSE endpoint for real-time dashboard event streaming.
//!
//! Clients connect at `/api/events` and receive `DashboardEvent` payloads as
//! SSE `data:` frames. Each event carries a monotonic `id:` for reconnection.

use std::convert::Infallible;
use std::sync::Arc;

use axum::Router;
use axum::extract::State;
use axum::http::{HeaderMap, HeaderValue};
use axum::response::IntoResponse;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::routing::get;
use futures::stream::{self, StreamExt};
use tokio::sync::broadcast;
use tracing::warn;

use crate::state::AppState;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/events", get(sse_handler))
        .route("/sse", get(sse_handler))
}

/// Response headers that instruct HTTP/2 proxies (Railway, Nginx, Cloudflare)
/// to disable buffering so SSE frames reach the client immediately.
pub(crate) fn sse_response_headers() -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert("X-Accel-Buffering", HeaderValue::from_static("no"));
    headers.insert(
        "Cache-Control",
        HeaderValue::from_static("no-cache, no-store, no-transform, must-revalidate"),
    );
    headers.insert("Connection", HeaderValue::from_static("keep-alive"));
    headers
}

/// `GET /api/events` and `GET /api/sse` — SSE stream of dashboard events.
async fn sse_handler(headers: HeaderMap, State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let replay_from = replay_start(&headers);

    // Cap the replay to 256 events to prevent a reconnecting client from
    // materializing the entire ring buffer into memory at once.
    let replay = state
        .state_hub
        .replay_from(replay_from)
        .into_iter()
        .take(256)
        .map(|envelope| {
            let data = serde_json::to_string(&envelope.payload).unwrap_or_default();
            Ok::<_, Infallible>(Event::default().data(data).id(envelope.seq.to_string()))
        });

    let live_state = Arc::clone(&state);
    let live = stream::unfold(
        (state.state_hub.subscribe_events(), live_state),
        |(mut rx, state)| async move {
            match rx.recv().await {
                Ok(envelope) => {
                    let data = serde_json::to_string(&envelope.payload).unwrap_or_default();
                    let event = Event::default().data(data).id(envelope.seq.to_string());
                    Some((Ok(event), (rx, state)))
                }
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    warn!(n, "SSE client lagged; sending materialized snapshot resync");
                    let last_materialized_seq = state.state_hub.total_published().saturating_sub(1);
                    let data = serde_json::json!({
                        "missed_events": n,
                        "last_materialized_seq": last_materialized_seq,
                        "snapshot": state.state_hub.current_snapshot(),
                    });
                    let event = Event::default()
                        .event("gap")
                        .data(data.to_string())
                        .id(last_materialized_seq.to_string());
                    Some((Ok(event), (rx, state)))
                }
                Err(broadcast::error::RecvError::Closed) => None,
            }
        },
    );

    // Use a shorter keep-alive interval than the default 15s to survive
    // aggressive proxy timeouts (Railway 30s, Nginx 60s). The "keepalive"
    // text triggers a proper SSE comment event in clients that ignore
    // empty comments.
    let sse = Sse::new(stream::iter(replay).chain(live)).keep_alive(
        KeepAlive::new()
            .interval(std::time::Duration::from_secs(8))
            .text("keepalive"),
    );

    (sse_response_headers(), sse)
}

fn replay_start(headers: &HeaderMap) -> u64 {
    headers
        .get("Last-Event-ID")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<u64>().ok())
        .map_or(0, |last_seen| last_seen.saturating_add(1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replay_starts_after_last_acknowledged_event() {
        let mut headers = HeaderMap::new();
        assert_eq!(replay_start(&headers), 0);

        headers.insert("Last-Event-ID", HeaderValue::from_static("41"));
        assert_eq!(replay_start(&headers), 42);

        headers.insert(
            "Last-Event-ID",
            HeaderValue::from_static("18446744073709551615"),
        );
        assert_eq!(replay_start(&headers), u64::MAX);
    }
}
