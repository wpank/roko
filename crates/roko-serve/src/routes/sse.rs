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
use roko_core::dashboard_snapshot::DashboardSnapshot;
use roko_runtime::event_bus::Envelope;
use roko_runtime::state_hub::StateHubCursorSnapshot;
use serde::Serialize;
use tokio::sync::broadcast;
use tracing::warn;

use crate::state::AppState;

const MAX_REPLAY_EVENTS: usize = 256;

#[derive(Serialize)]
struct GapPayload {
    missed_events: u64,
    last_materialized_seq: u64,
    snapshot: DashboardSnapshot,
}

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
    let requested_seq = replay_start(&headers);
    let subscription = state.state_hub.subscribe_events_from(requested_seq);
    let needs_snapshot = replay_requires_snapshot(
        requested_seq,
        subscription.cursor.next_seq,
        &subscription.replay,
        MAX_REPLAY_EVENTS,
    );
    let live_floor = subscription.cursor.next_seq;

    // Never truncate replay silently. If the cursor has fallen out of the ring
    // or the retained suffix is too large, replace it with one explicit snapshot
    // gap frame and continue live from that snapshot's atomic cursor.
    let replay = if needs_snapshot {
        vec![Ok::<_, Infallible>(gap_event(gap_payload(
            requested_seq,
            subscription.cursor,
        )))]
    } else {
        subscription
            .replay
            .into_iter()
            .map(dashboard_event)
            .map(Ok::<_, Infallible>)
            .collect()
    };

    let live_state = Arc::clone(&state);
    let live = stream::unfold(
        (subscription.live, live_state, live_floor),
        |(mut rx, state, mut live_floor)| async move {
            loop {
                match rx.recv().await {
                    Ok(envelope) => {
                        if envelope.seq < live_floor {
                            continue;
                        }
                        live_floor = envelope.seq.saturating_add(1);
                        return Some((Ok(dashboard_event(envelope)), (rx, state, live_floor)));
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        warn!(n, "SSE client lagged; sending materialized snapshot resync");
                        let expected_seq = live_floor;
                        let cursor = state.state_hub.cursor_snapshot();
                        live_floor = cursor.next_seq;
                        return Some((
                            Ok(gap_event(gap_payload(expected_seq, cursor))),
                            (rx, state, live_floor),
                        ));
                    }
                    Err(broadcast::error::RecvError::Closed) => return None,
                }
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

fn dashboard_event(envelope: Envelope<roko_core::DashboardEvent>) -> Event {
    let data = serde_json::to_string(&envelope.payload).unwrap_or_default();
    Event::default().data(data).id(envelope.seq.to_string())
}

fn gap_event(payload: GapPayload) -> Event {
    let event_id = payload.last_materialized_seq.to_string();
    let data = serde_json::to_string(&payload).unwrap_or_default();
    Event::default().event("gap").data(data).id(event_id)
}

fn gap_payload(requested_seq: u64, cursor: StateHubCursorSnapshot) -> GapPayload {
    GapPayload {
        missed_events: cursor.next_seq.saturating_sub(requested_seq),
        last_materialized_seq: cursor.next_seq.saturating_sub(1),
        snapshot: cursor.snapshot,
    }
}

fn replay_requires_snapshot(
    requested_seq: u64,
    next_seq: u64,
    replay: &[Envelope<roko_core::DashboardEvent>],
    max_replay: usize,
) -> bool {
    if requested_seq >= next_seq {
        return false;
    }
    if replay.len() > max_replay {
        return true;
    }
    if replay.first().map(|event| event.seq) != Some(requested_seq)
        || replay.last().map(|event| event.seq.saturating_add(1)) != Some(next_seq)
    {
        return true;
    }
    replay
        .windows(2)
        .any(|pair| pair[0].seq.saturating_add(1) != pair[1].seq)
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
    use roko_runtime::StateHub;

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

    #[test]
    fn reconnect_replays_exact_suffix_without_snapshot() {
        let hub = StateHub::new(8);
        for plan_id in ["plan-0", "plan-1"] {
            hub.publish(roko_core::DashboardEvent::PlanStarted {
                plan_id: plan_id.into(),
            });
        }

        let subscription = hub.subscribe_events_from(1);
        assert!(!replay_requires_snapshot(
            1,
            subscription.cursor.next_seq,
            &subscription.replay,
            MAX_REPLAY_EVENTS,
        ));
        assert_eq!(
            subscription
                .replay
                .iter()
                .map(|event| event.seq)
                .collect::<Vec<_>>(),
            vec![1]
        );
    }

    #[test]
    fn evicted_cursor_requires_snapshot_with_latest_state() {
        let hub = StateHub::new(2);
        for index in 0..3 {
            hub.publish(roko_core::DashboardEvent::PlanStarted {
                plan_id: format!("plan-{index}"),
            });
        }

        let subscription = hub.subscribe_events_from(0);
        assert!(replay_requires_snapshot(
            0,
            subscription.cursor.next_seq,
            &subscription.replay,
            MAX_REPLAY_EVENTS,
        ));
        let payload = gap_payload(0, subscription.cursor);
        assert_eq!(payload.missed_events, 3);
        assert_eq!(payload.last_materialized_seq, 2);
        assert!(payload.snapshot.plans.contains_key("plan-2"));
    }

    #[test]
    fn oversized_replay_requires_snapshot_instead_of_truncation() {
        let hub = StateHub::new(MAX_REPLAY_EVENTS + 1);
        for index in 0..=MAX_REPLAY_EVENTS {
            hub.publish(roko_core::DashboardEvent::PlanStarted {
                plan_id: format!("plan-{index}"),
            });
        }

        let subscription = hub.subscribe_events_from(0);
        assert_eq!(subscription.replay.len(), MAX_REPLAY_EVENTS + 1);
        assert!(replay_requires_snapshot(
            0,
            subscription.cursor.next_seq,
            &subscription.replay,
            MAX_REPLAY_EVENTS,
        ));
    }
}
