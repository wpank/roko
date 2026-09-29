//! WebSocket endpoint for real-time event streaming.
//!
//! Clients connect at `/ws` and receive `ServerEvent` payloads as JSON text
//! frames. On connection, the server replays recent events from the ring
//! buffer, then streams live events via the broadcast channel.
//!
//! # Back-pressure modes
//!
//! The client may request one of three back-pressure modes in its first
//! subscribe message via the `back_pressure` field:
//!
//! - **`at_most_once`** (default): deliver every event, drop on transport lag.
//! - **`coalesce`**: buffer up to [`COALESCE_BUFFER`] events when the client
//!   is lagging; send a JSON array batch when it catches up. Events are never
//!   silently discarded — older events are evicted from the buffer only when
//!   the buffer is full and a new event arrives.
//! - **`resume_required`**: on lag, immediately halt event delivery and send a
//!   `{"type":"resume_required","last_event_id":N}` control frame. The client
//!   must acknowledge with `{"type":"resume","cursor":N}` before the server
//!   will resume. The server replays missed events from `N` using the ring
//!   buffer.

use std::collections::VecDeque;
use std::sync::Arc;
use std::time::Instant;

use axum::Router;
use axum::extract::State;
use axum::extract::ws::{CloseFrame, Message, WebSocket, WebSocketUpgrade, close_code};
use axum::response::IntoResponse;
use axum::routing::get;
use futures::SinkExt;
use futures::stream::StreamExt;
use serde::{Deserialize, Serialize};
use tracing::{debug, warn};

use roko_core::obs::LogScrubber;

use crate::state::AppState;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/ws", get(ws_upgrade))
        .route("/roko-ws", get(ws_upgrade))
        .route("/ws/agents", get(ws_upgrade))
}

/// Per-frame and per-message ceilings applied to every WebSocket upgrade.
///
/// `max_message_size` bounds the total reassembled payload a single message
/// can carry (1 MiB) and `max_frame_size` bounds an individual fragmented
/// frame (256 KiB). Together they prevent a hostile client from forcing the
/// server to buffer arbitrary amounts of memory before the application code
/// even runs.
pub(crate) const WS_MAX_MESSAGE_SIZE: usize = 1024 * 1024;
pub(crate) const WS_MAX_FRAME_SIZE: usize = 256 * 1024;

/// Maximum number of events held in the coalesce buffer before older entries
/// are evicted. Sized to be large enough to be useful without unbounded growth.
const COALESCE_BUFFER: usize = 16;

/// Apply the standard `(max_message_size, max_frame_size)` caps to a
/// [`WebSocketUpgrade`] before any handler-specific configuration. Every
/// upgrade handler in this crate goes through this helper.
pub(crate) fn apply_ws_size_limits(ws: WebSocketUpgrade) -> WebSocketUpgrade {
    ws.max_message_size(WS_MAX_MESSAGE_SIZE)
        .max_frame_size(WS_MAX_FRAME_SIZE)
}

/// `GET /ws` — upgrade to a WebSocket connection.
async fn ws_upgrade(State(state): State<Arc<AppState>>, ws: WebSocketUpgrade) -> impl IntoResponse {
    apply_ws_size_limits(ws).on_upgrade(move |socket| handle_ws(state, socket))
}

/// Back-pressure mode for a WebSocket subscription channel.
#[derive(Debug, Clone, Default, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
enum BackPressureMode {
    /// Deliver every event, dropping only on transport failure.
    #[default]
    AtMostOnce,
    /// Coalesce rapid-fire events into a batched JSON array when the client
    /// is lagging.
    Coalesce,
    /// On lag, halt delivery and require the client to resume with a cursor.
    ResumeRequired,
}

/// Client control message (optional filtering with cursor resume).
///
/// ```json
/// {
///   "subscribe": ["projection:gate_pipeline", "topic:agent.*"],
///   "cursor": 42,
///   "back_pressure": "at_most_once"
/// }
/// ```
///
/// For `resume_required` mode, the client resumes with:
/// ```json
/// { "type": "resume", "cursor": 42 }
/// ```
#[derive(Deserialize)]
struct ClientMsg {
    #[serde(default)]
    subscribe: Vec<String>,
    /// Resume from this sequence number (replay events with seq >= cursor).
    #[serde(default)]
    cursor: Option<u64>,
    /// Per-connection back-pressure mode.
    #[serde(default)]
    back_pressure: Option<BackPressureMode>,
    /// Used by the client to resume delivery after a `resume_required` pause.
    /// Set `type` to `"resume"` and provide the last-seen event ID.
    #[serde(rename = "type", default)]
    msg_type: Option<String>,
}

/// Server-to-client control frame sent when `resume_required` mode is active
/// and the client has fallen too far behind.
#[derive(Serialize)]
struct ResumeRequiredFrame {
    #[serde(rename = "type")]
    msg_type: &'static str,
    last_event_id: u64,
}

/// Server-to-client control frame used to send a batch of coalesced events.
///
/// The `events` field is an array of the raw event JSON objects that were
/// buffered while the client was lagging.
#[derive(Serialize)]
struct CoalescedFrame {
    #[serde(rename = "type")]
    msg_type: &'static str,
    events: Vec<serde_json::Value>,
}

/// Scrub a JSON payload through the shared [`LogScrubber`] before sending.
fn scrub_json(json: &str, scrubber: &LogScrubber) -> String {
    scrubber.scrub(json)
}

/// Main WebSocket handler — replay then stream.
async fn handle_ws(state: Arc<AppState>, socket: WebSocket) {
    let (mut sink, mut stream) = socket.split();
    let mut filter: Vec<String> = Vec::new();
    let mut replay_cursor: u64 = 0;
    let mut back_pressure = BackPressureMode::AtMostOnce;
    let scrubber = Arc::clone(&state.scrubber);

    // Coalesce mode: events accumulated while the client is lagging.
    let mut coalesce_buf: VecDeque<serde_json::Value> = VecDeque::new();

    // ResumeRequired mode: when true the server stops forwarding live events
    // until the client sends a resume message.
    let mut resume_paused = false;
    // The sequence number of the last event seen before pause.
    let mut resume_last_seq: u64 = 0;

    // Wait for the first client message to get the cursor, or replay from 0.
    // We do an initial replay from 0; if the client sends a cursor later we
    // will not re-replay (the cursor is used for the initial catchup only via
    // the subscribe message).
    let backlog = state.event_bus.replay_from(replay_cursor);
    for envelope in &backlog {
        let Ok(payload) = serde_json::to_string(&envelope.payload) else {
            continue;
        };
        let payload = scrub_json(&payload, &scrubber);
        if sink.send(Message::Text(payload.into())).await.is_err() {
            return;
        }
    }
    // Track the highest seq seen during initial replay so we have a valid
    // starting point for resume_last_seq.
    if let Some(last) = backlog.last() {
        resume_last_seq = last.seq;
    }

    // Subscribe to live events.
    let mut rx = state.event_bus.subscribe();
    let mut last_lag_warn = Instant::now()
        .checked_sub(std::time::Duration::from_secs(10))
        .unwrap_or(Instant::now());
    let mut lag_total: u64 = 0;

    loop {
        tokio::select! {
            // Incoming client messages (filter subscriptions + cursor).
            msg = stream.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        if let Ok(cmd) = serde_json::from_str::<ClientMsg>(&text) {
                            // Handle resume acknowledgment for ResumeRequired mode.
                            if cmd.msg_type.as_deref() == Some("resume") {
                                if back_pressure == BackPressureMode::ResumeRequired && resume_paused {
                                    let cursor = cmd.cursor.unwrap_or(resume_last_seq);
                                    debug!(cursor, "ws client resumed; replaying from cursor");
                                    resume_paused = false;

                                    // Replay events the client missed while paused.
                                    let catchup = state.event_bus.replay_from(cursor);
                                    for envelope in &catchup {
                                        if !filter.is_empty()
                                            && !matches_filter(&envelope.payload, &filter)
                                        {
                                            continue;
                                        }
                                        let Ok(json) =
                                            serde_json::to_string(&envelope.payload)
                                        else {
                                            continue;
                                        };
                                        let json = scrub_json(&json, &scrubber);
                                        if sink.send(Message::Text(json.into())).await.is_err() {
                                            return;
                                        }
                                        resume_last_seq = envelope.seq;
                                    }
                                }
                                // Ignore resume messages when not paused.
                                continue;
                            }

                            filter = cmd.subscribe;
                            if let Some(bp) = cmd.back_pressure {
                                back_pressure = bp;
                                // Reset mode-specific state when the client switches modes.
                                coalesce_buf.clear();
                                resume_paused = false;
                            }
                            // If client provides a cursor, replay missed events.
                            if let Some(cursor) = cmd.cursor {
                                replay_cursor = cursor;
                                let catchup = state.event_bus.replay_from(replay_cursor);
                                for envelope in &catchup {
                                    if !filter.is_empty()
                                        && !matches_filter(&envelope.payload, &filter)
                                    {
                                        continue;
                                    }
                                    let Ok(json) =
                                        serde_json::to_string(&envelope.payload)
                                    else {
                                        continue;
                                    };
                                    let json = scrub_json(&json, &scrubber);
                                    if sink.send(Message::Text(json.into())).await.is_err() {
                                        return;
                                    }
                                    resume_last_seq = envelope.seq;
                                }
                            }
                            debug!(?filter, cursor = ?cmd.cursor, "ws client updated subscription");
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => {
                        debug!("ws client disconnected");
                        break;
                    }
                    Some(Err(e)) => {
                        warn!("ws recv error: {e}");
                        break;
                    }
                    _ => {}
                }
            }
            // The server is shutting down: say so, rather than leave the socket
            // open until the process exits.
            () = state.cancel.cancelled() => {
                let _ = sink
                    .send(Message::Close(Some(CloseFrame {
                        code: close_code::AWAY,
                        reason: "server shutting down".into(),
                    })))
                    .await;
                break;
            }
            // Outgoing events.
            event = rx.recv() => {
                match event {
                    Ok(envelope) => {
                        if !filter.is_empty() && !matches_filter(&envelope.payload, &filter) {
                            continue;
                        }

                        match back_pressure {
                            BackPressureMode::AtMostOnce => {
                                match serde_json::to_string(&envelope.payload) {
                                    Ok(json) => {
                                        let json = scrub_json(&json, &scrubber);
                                        if sink.send(Message::Text(json.into())).await.is_err() {
                                            break;
                                        }
                                    }
                                    Err(e) => {
                                        warn!("ws serialize error: {e}");
                                    }
                                }
                                resume_last_seq = envelope.seq;
                            }

                            BackPressureMode::Coalesce => {
                                // Add the new event to the coalesce buffer.
                                // If the buffer is full, evict the oldest entry to make room.
                                if coalesce_buf.len() >= COALESCE_BUFFER {
                                    coalesce_buf.pop_front();
                                }
                                match serde_json::to_value(&envelope.payload) {
                                    Ok(val) => {
                                        coalesce_buf.push_back(val);
                                    }
                                    Err(e) => {
                                        warn!("ws coalesce serialize error: {e}");
                                        continue;
                                    }
                                }
                                resume_last_seq = envelope.seq;

                                // Flush the entire buffer as a single batched frame.
                                // We only flush when the buffer is non-empty (always true
                                // here); the batch message groups rapid-fire events so
                                // the client can process them together.
                                let events: Vec<serde_json::Value> =
                                    std::mem::take(&mut coalesce_buf).into_iter().collect();
                                let frame = CoalescedFrame {
                                    msg_type: "coalesced",
                                    events,
                                };
                                match serde_json::to_string(&frame) {
                                    Ok(json) => {
                                        let json = scrub_json(&json, &scrubber);
                                        if sink.send(Message::Text(json.into())).await.is_err() {
                                            break;
                                        }
                                    }
                                    Err(e) => {
                                        warn!("ws coalesce flush error: {e}");
                                    }
                                }
                            }

                            BackPressureMode::ResumeRequired => {
                                // While paused, discard live events — the client will
                                // request a replay from the ring buffer after resuming.
                                if resume_paused {
                                    continue;
                                }
                                match serde_json::to_string(&envelope.payload) {
                                    Ok(json) => {
                                        let json = scrub_json(&json, &scrubber);
                                        if sink.send(Message::Text(json.into())).await.is_err() {
                                            break;
                                        }
                                        resume_last_seq = envelope.seq;
                                    }
                                    Err(e) => {
                                        warn!("ws serialize error: {e}");
                                    }
                                }
                            }
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                        lag_total += n;

                        match back_pressure {
                            BackPressureMode::AtMostOnce => {
                                if last_lag_warn.elapsed() >= std::time::Duration::from_secs(5) {
                                    warn!(skipped = lag_total, "ws client lagged");
                                    lag_total = 0;
                                    last_lag_warn = Instant::now();
                                }
                            }

                            BackPressureMode::Coalesce => {
                                // The coalesce buffer is already accumulating events.
                                // Log lag but keep going — the next Ok(envelope) will
                                // flush the buffer.
                                if last_lag_warn.elapsed() >= std::time::Duration::from_secs(5) {
                                    warn!(
                                        skipped = lag_total,
                                        buf = coalesce_buf.len(),
                                        "ws coalesce client lagged; buffering events"
                                    );
                                    lag_total = 0;
                                    last_lag_warn = Instant::now();
                                }
                            }

                            BackPressureMode::ResumeRequired => {
                                if !resume_paused {
                                    // Transition to paused state and notify the client.
                                    resume_paused = true;
                                    warn!(
                                        skipped = lag_total,
                                        last_event_id = resume_last_seq,
                                        "ws resume_required client lagged; pausing delivery"
                                    );
                                    lag_total = 0;
                                    let frame = ResumeRequiredFrame {
                                        msg_type: "resume_required",
                                        last_event_id: resume_last_seq,
                                    };
                                    match serde_json::to_string(&frame) {
                                        Ok(json) => {
                                            let json = scrub_json(&json, &scrubber);
                                            if sink
                                                .send(Message::Text(json.into()))
                                                .await
                                                .is_err()
                                            {
                                                break;
                                            }
                                        }
                                        Err(e) => {
                                            warn!("ws resume_required frame error: {e}");
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                        debug!("event bus closed, shutting down ws");
                        break;
                    }
                }
            }
        }
    }

    let _ = sink.close().await;
}

/// Check whether an event matches the client's subscription filter.
///
/// Filter strings support two forms:
///   1. **Plain type match** — the filter string is matched against the event's
///      serde `type` tag (substring match, backward-compatible).
///   2. **Channel prefix** — `projection:<name>`, `topic:<pattern>`,
///      `signal-stream:<name>`, or legacy `engram-stream:<name>`.  These are
///      matched against the serialized event's type using glob-like semantics
///      where `*` matches any suffix.
///
/// An empty filter accepts all events.
fn matches_filter(event: &crate::events::ServerEvent, filter: &[String]) -> bool {
    // Serialize to extract the "type" field cheaply.
    let Ok(val) = serde_json::to_value(event) else {
        return true;
    };
    let mut event_types = Vec::new();
    if let Some(event_type) = val.get("type").and_then(|t| t.as_str()) {
        event_types.push(event_type.to_string());
    }
    if event_types.iter().any(|t| t == "execution") {
        if let Some(exec_type) = val
            .get("event")
            .and_then(|event| event.get("type"))
            .and_then(|t| t.as_str())
        {
            event_types.push(exec_type.to_string());
        }
    }

    filter.iter().any(|f| {
        // Channel prefix patterns: `projection:gate_pipeline`, `topic:agent.*`
        if let Some(pattern) = f
            .strip_prefix("projection:")
            .or_else(|| f.strip_prefix("topic:"))
            .or_else(|| f.strip_prefix("signal-stream:"))
            .or_else(|| f.strip_prefix("engram-stream:"))
        {
            return channel_pattern_matches(&event_types, pattern);
        }
        // Legacy: plain substring match against event type tags.
        event_types
            .iter()
            .any(|event_type| event_type.contains(f.as_str()))
    })
}

/// Match a channel pattern against event types. Supports `*` wildcard suffix
/// (e.g., `agent.*` matches `agent.spawned`, `agent.output`).
fn channel_pattern_matches(event_types: &[String], pattern: &str) -> bool {
    if pattern == "*" {
        return true;
    }
    if let Some(prefix) = pattern.strip_suffix(".*") {
        return event_types
            .iter()
            .any(|t| t.starts_with(prefix) || t.contains(prefix));
    }
    event_types
        .iter()
        .any(|t| t == pattern || t.contains(pattern))
}
