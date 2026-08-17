#![deny(unsafe_code)]
#![allow(missing_docs)]

use std::sync::Arc;

use axum::{
    Json, Router,
    extract::DefaultBodyLimit,
    extract::{
        Path, Query, State, WebSocketUpgrade,
        ws::{Message, WebSocket},
    },
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use futures::{SinkExt, StreamExt};
use serde_json::{Value, json};
use tower_http::trace::TraceLayer;
use tracing::warn;

const AGENT_HELLO_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);

pub mod bus;
pub mod chain_watcher;
pub mod protocol;
pub mod registry;
pub mod state;

pub use bus::{TopicBus, TopicBusConfig};

use bus::RelayMailbox;
use protocol::{
    AgentInboundFrame, RelayEvent, RelayMessageRequest, RelayOutboundFrame, TopicEnvelope,
};
use state::{AwaitMessageError, BeginMessageError, RegisteredAgent, RelayState};

pub fn app(state: Arc<RelayState>) -> Router {
    let mut router = Router::new()
        .route("/relay/health", get(health))
        .route("/relay/agents", get(list_agents))
        .route("/relay/agents/ws", get(agent_ws))
        .route("/relay/cards/{id}", get(get_card))
        .route("/relay/messages", post(forward_message))
        .route("/relay/events/ws", get(events_ws))
        .route("/relay/workspaces", get(list_workspaces))
        .route("/relay/workspaces/register", post(register_workspace))
        .route(
            "/relay/workspaces/{id}/heartbeat",
            post(workspace_heartbeat),
        )
        .route(
            "/relay/workspaces/{id}",
            axum::routing::delete(unregister_workspace),
        )
        // Feed registration endpoints
        .route("/relay/feeds", get(list_feeds))
        .route("/relay/feeds/{agent_id}", get(agent_feeds))
        // Feed metadata endpoints (A5)
        .route("/relay/topics", get(list_topics))
        .route("/relay/topics/{topic}/messages", get(topic_messages))
        .route("/relay/topics/{topic}/subscribers", get(topic_subscribers));
    if state.registry().is_some_and(|registry| registry.can_read()) {
        router = router
            .route("/registry/extensions/{name}", get(get_registry_extension))
            .route(
                "/registry/extensions/{name}/resolve",
                get(resolve_registry_extension),
            )
            .route(
                "/registry/extensions/{name}/versions/{version}",
                get(get_registry_extension_version),
            );
        if state
            .registry()
            .is_some_and(|registry| registry.can_publish())
        {
            router = router.route(
                "/registry/extensions",
                post(publish_registry_extension)
                    .layer(DefaultBodyLimit::max(registry::MAX_PUBLISH_BODY_BYTES)),
            );
        }
    }
    router.layer(TraceLayer::new_for_http()).with_state(state)
}

async fn get_registry_extension(
    State(state): State<Arc<RelayState>>,
    Path(name): Path<String>,
    Query(query): Query<RegistryRequirementQuery>,
) -> Result<Json<roko_plugin::registry::RegistryPackage>, (StatusCode, Json<Value>)> {
    let registry = state.registry().ok_or_else(registry_unavailable)?;
    registry
        .resolve(&name, query.requirement.as_deref().unwrap_or("*"))
        .map(Json)
        .map_err(registry_error)
}

#[derive(Default, serde::Deserialize)]
struct RegistryRequirementQuery {
    requirement: Option<String>,
}

async fn resolve_registry_extension(
    State(state): State<Arc<RelayState>>,
    Path(name): Path<String>,
    Query(query): Query<RegistryRequirementQuery>,
) -> Result<Json<roko_plugin::registry::ResolvedRegistryGraph>, (StatusCode, Json<Value>)> {
    let registry = state.registry().ok_or_else(registry_unavailable)?;
    registry
        .resolve_graph(&name, query.requirement.as_deref().unwrap_or("*"))
        .map(Json)
        .map_err(registry_error)
}

async fn get_registry_extension_version(
    State(state): State<Arc<RelayState>>,
    Path((name, version)): Path<(String, String)>,
) -> Result<Json<roko_plugin::registry::RegistryPackage>, (StatusCode, Json<Value>)> {
    let registry = state.registry().ok_or_else(registry_unavailable)?;
    registry
        .get(&name, &version)
        .map(Json)
        .map_err(registry_error)
}

async fn publish_registry_extension(
    State(state): State<Arc<RelayState>>,
    headers: HeaderMap,
    Json(package): Json<roko_plugin::registry::RegistryPackage>,
) -> Result<(StatusCode, Json<roko_plugin::registry::RegistryPackage>), (StatusCode, Json<Value>)> {
    let registry = state.registry().ok_or_else(registry_unavailable)?;
    let authorization = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .ok_or_else(|| {
            (
                StatusCode::UNAUTHORIZED,
                Json(json!({ "error": "publisher bearer token is required" })),
            )
        })?;
    let token = authorization.strip_prefix("Bearer ").ok_or_else(|| {
        (
            StatusCode::UNAUTHORIZED,
            Json(json!({ "error": "publisher authorization must use Bearer" })),
        )
    })?;
    registry
        .publish(package, token)
        .map(|outcome| {
            let status = if outcome.created {
                StatusCode::CREATED
            } else {
                StatusCode::OK
            };
            (status, Json(outcome.package))
        })
        .map_err(registry_error)
}

fn registry_unavailable() -> (StatusCode, Json<Value>) {
    (
        StatusCode::NOT_FOUND,
        Json(json!({ "error": "extension registry is not configured" })),
    )
}

fn registry_error(error: registry::RegistryError) -> (StatusCode, Json<Value>) {
    let status = match error {
        registry::RegistryError::Unauthorized(_) => StatusCode::UNAUTHORIZED,
        registry::RegistryError::NotFound(_) => StatusCode::NOT_FOUND,
        registry::RegistryError::Conflict(_) => StatusCode::CONFLICT,
        registry::RegistryError::Invalid(_) | registry::RegistryError::MissingDependency(_) => {
            StatusCode::UNPROCESSABLE_ENTITY
        }
        registry::RegistryError::Storage(_) => StatusCode::INTERNAL_SERVER_ERROR,
    };
    (status, Json(json!({ "error": error.to_string() })))
}

async fn health() -> &'static str {
    "ok"
}

async fn list_agents(State(state): State<Arc<RelayState>>) -> Json<Vec<protocol::ConnectedAgent>> {
    Json(state.list_agents())
}

async fn list_workspaces(
    State(state): State<Arc<RelayState>>,
) -> Json<Vec<protocol::ConnectedWorkspace>> {
    Json(state.list_workspaces())
}

async fn register_workspace(
    State(state): State<Arc<RelayState>>,
    Json(hello): Json<protocol::WorkspaceHello>,
) -> impl IntoResponse {
    if state.register_workspace(hello) {
        StatusCode::OK
    } else {
        StatusCode::UNPROCESSABLE_ENTITY
    }
}

async fn workspace_heartbeat(
    State(state): State<Arc<RelayState>>,
    Path(id): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> impl IntoResponse {
    let agents_count = body
        .get("agents_count")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0);
    let agents_count = u32::try_from(agents_count).unwrap_or(u32::MAX);
    state.workspace_heartbeat(&id, agents_count);
    StatusCode::OK
}

async fn unregister_workspace(
    State(state): State<Arc<RelayState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    state.unregister_workspace(&id);
    StatusCode::OK
}

async fn get_card(
    State(state): State<Arc<RelayState>>,
    Path(id): Path<String>,
) -> Result<Json<Value>, StatusCode> {
    state.card(&id).map(Json).ok_or(StatusCode::NOT_FOUND)
}

async fn forward_message(
    State(state): State<Arc<RelayState>>,
    Json(request): Json<RelayMessageRequest>,
) -> Result<Json<protocol::RelayMessageResponse>, (StatusCode, Json<Value>)> {
    let pending = state.begin_message(request).map_err(begin_message_error)?;
    pending
        .await_response()
        .await
        .map(Json)
        .map_err(await_message_error)
}

async fn agent_ws(State(state): State<Arc<RelayState>>, ws: WebSocketUpgrade) -> Response {
    let Some(permit) = state.try_admit_agent_socket() else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({ "error": "relay agent socket capacity reached" })),
        )
            .into_response();
    };
    ws.max_message_size(2 * 1024 * 1024)
        .max_frame_size(2 * 1024 * 1024)
        .on_upgrade(move |socket| handle_agent_socket(state, socket, permit))
        .into_response()
}

async fn events_ws(State(state): State<Arc<RelayState>>, ws: WebSocketUpgrade) -> Response {
    let Some(permit) = state.try_admit_events_socket() else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({ "error": "relay events socket capacity reached" })),
        )
            .into_response();
    };
    ws.max_message_size(2 * 1024 * 1024)
        .max_frame_size(2 * 1024 * 1024)
        .on_upgrade(move |socket| handle_events_socket(state, socket, permit))
        .into_response()
}

fn begin_message_error(error: BeginMessageError) -> (StatusCode, Json<Value>) {
    match error {
        BeginMessageError::UnknownAgent => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "unknown agent" })),
        ),
        BeginMessageError::NotConnected => (
            StatusCode::BAD_GATEWAY,
            Json(json!({ "error": "agent connection is not writable" })),
        ),
        BeginMessageError::Capacity => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({ "error": "relay pending response capacity reached" })),
        ),
        BeginMessageError::InvalidRequest => (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(json!({ "error": "invalid or oversized relay message" })),
        ),
    }
}

fn await_message_error(error: AwaitMessageError) -> (StatusCode, Json<Value>) {
    match error {
        AwaitMessageError::Timeout => (
            StatusCode::GATEWAY_TIMEOUT,
            Json(json!({ "error": "agent response timed out" })),
        ),
        AwaitMessageError::Agent(error) => {
            (StatusCode::BAD_GATEWAY, Json(json!({ "error": error })))
        }
    }
}

async fn handle_agent_socket(
    state: Arc<RelayState>,
    socket: WebSocket,
    _permit: tokio::sync::OwnedSemaphorePermit,
) {
    let (mut sink, mut stream) = socket.split();
    let Ok(Some(first_frame)) =
        tokio::time::timeout(AGENT_HELLO_TIMEOUT, next_text_frame(&mut stream)).await
    else {
        let _ = sink.close().await;
        return;
    };

    let hello = match serde_json::from_str::<AgentInboundFrame>(&first_frame) {
        Ok(AgentInboundFrame::Hello(hello)) => hello,
        Ok(_) => {
            let _ = send_raw_json(
                &mut sink,
                json!({
                    "error": "first frame must be hello"
                }),
            )
            .await;
            let _ = sink.close().await;
            return;
        }
        Err(error) => {
            let _ = send_raw_json(
                &mut sink,
                json!({
                    "error": format!("invalid hello frame: {error}")
                }),
            )
            .await;
            let _ = sink.close().await;
            return;
        }
    };

    let outbound_tx = state.bus.delivery_mailbox();
    outbound_tx.pause_topics();
    let Ok(RegisteredAgent {
        session_id,
        agent_id,
    }) = state.register_agent(hello, outbound_tx.clone())
    else {
        let _ = send_raw_json(
            &mut sink,
            json!({ "error": "relay connection capacity reached" }),
        )
        .await;
        let _ = sink.close().await;
        return;
    };

    let writer_mailbox = outbound_tx.clone();
    let writer = tokio::spawn(async move {
        while let Some(frame) = writer_mailbox.recv_shared().await {
            let close_after = matches!(
                frame.frame(),
                RelayOutboundFrame::ResumeRequired { .. } | RelayOutboundFrame::Superseded(_)
            );
            if sink
                .send(Message::Text(frame.encoded().clone()))
                .await
                .is_err()
            {
                break;
            }
            if close_after {
                let _ = sink.close().await;
                break;
            }
        }
    });

    let _ = outbound_tx.send(RelayOutboundFrame::Ack {
        event: "hello".to_string(),
    });

    let mut subscription_initialized = false;
    while let Some(message) = stream.next().await {
        match message {
            Ok(Message::Text(text)) => {
                if !handle_agent_frame(
                    &state,
                    &agent_id,
                    session_id,
                    &outbound_tx,
                    &mut subscription_initialized,
                    text.as_str(),
                ) {
                    break;
                }
            }
            Ok(Message::Close(_)) => break,
            Ok(_) => {}
            Err(error) => {
                warn!(%agent_id, %error, "relay websocket receive failed");
                break;
            }
        }
    }

    state.unregister_agent(&agent_id, session_id);
    outbound_tx.close();
    writer.abort();
}

fn handle_agent_frame(
    state: &Arc<RelayState>,
    agent_id: &str,
    session_id: uuid::Uuid,
    outbound_tx: &RelayMailbox,
    subscription_initialized: &mut bool,
    text: &str,
) -> bool {
    let frame = serde_json::from_str::<AgentInboundFrame>(text);
    state
        .with_current_session(agent_id, session_id, || {
            handle_current_agent_frame(
                state,
                agent_id,
                session_id,
                outbound_tx,
                subscription_initialized,
                frame,
            )
        })
        .unwrap_or(false)
}

#[allow(clippy::too_many_lines)]
fn handle_current_agent_frame(
    state: &Arc<RelayState>,
    agent_id: &str,
    session_id: uuid::Uuid,
    outbound_tx: &RelayMailbox,
    subscription_initialized: &mut bool,
    frame: Result<AgentInboundFrame, serde_json::Error>,
) -> bool {
    match frame {
        Ok(AgentInboundFrame::Card { card, card_uri }) => {
            let frame = if state.update_card(agent_id, card, card_uri) {
                RelayOutboundFrame::Ack {
                    event: "card".to_string(),
                }
            } else {
                RelayOutboundFrame::Error {
                    message_id: None,
                    error: "invalid or oversized relay card".to_owned(),
                }
            };
            let _ = outbound_tx.send(frame);
            true
        }
        Ok(AgentInboundFrame::Response {
            message_id,
            response,
        }) => {
            state.resolve_response(agent_id, session_id, &message_id, Ok(response));
            true
        }
        Ok(AgentInboundFrame::Error { message_id, error }) => {
            state.agent_error(agent_id, session_id, message_id.clone(), error.clone());
            if message_id.is_none() {
                let _ = outbound_tx.send(RelayOutboundFrame::Error {
                    message_id: None,
                    error,
                });
            }
            true
        }
        Ok(AgentInboundFrame::Ping) => {
            let _ = outbound_tx.send(RelayOutboundFrame::Pong);
            true
        }
        Ok(AgentInboundFrame::Hello(_)) => {
            let _ = outbound_tx.send(RelayOutboundFrame::Error {
                message_id: None,
                error: "agent already registered on this socket".to_string(),
            });
            true
        }
        Ok(AgentInboundFrame::Subscribe { request, topic }) => {
            if *subscription_initialized {
                let _ = outbound_tx.send(RelayOutboundFrame::Error {
                    message_id: None,
                    error: "subscriptions are initialized once; reconnect to change cursor rooms"
                        .to_owned(),
                });
                return true;
            }
            let mut rooms = request.rooms;
            let last_seq = request.last_seq;
            if let Some(topic) = topic {
                rooms.push(topic);
            }
            rooms.sort();
            rooms.dedup();
            if rooms.is_empty() || rooms.len() > 64 {
                let _ = outbound_tx.send(RelayOutboundFrame::Error {
                    message_id: None,
                    error: "subscribe requires 1-64 valid rooms".to_owned(),
                });
                return true;
            }
            // Replay is explicit through Resume, so subscription changes never
            // ambiguously duplicate retained history.
            if state
                .bus
                .subscribe_and_recover(
                    agent_id,
                    &rooms,
                    last_seq,
                    |budget| state.relay_snapshot_with_budget(budget),
                    outbound_tx,
                )
                .is_err()
            {
                let _ = outbound_tx.send(RelayOutboundFrame::Error {
                    message_id: None,
                    error: "invalid or unavailable relay room batch".to_owned(),
                });
                return true;
            }
            *subscription_initialized = true;
            let _ = outbound_tx.send(RelayOutboundFrame::Ack {
                event: format!("subscribed:{}", rooms.join(",")),
            });
            true
        }
        Ok(AgentInboundFrame::Resume(_)) => {
            // Replay is coupled to subscription installation through
            // SubscribeMessage::last_seq. A live-socket Resume cannot prevent
            // the writer from already having dequeued a newer live frame.
            let _ = outbound_tx.send(RelayOutboundFrame::Error {
                message_id: None,
                error: "resume requires reconnect with subscribe.last_seq".to_owned(),
            });
            true
        }
        Ok(AgentInboundFrame::Ack { room, seq }) => {
            if state.bus.acknowledge(agent_id, &room, seq).is_err() {
                let _ = outbound_tx.send(RelayOutboundFrame::Error {
                    message_id: None,
                    error: "invalid relay acknowledgement".to_owned(),
                });
            }
            true
        }
        Ok(AgentInboundFrame::Unsubscribe { request, topic }) => {
            let mut rooms = request.rooms;
            if let Some(topic) = topic {
                rooms.push(topic);
            }
            rooms.sort();
            rooms.dedup();
            for room in &rooms {
                tracing::debug!(%agent_id, %room, "unsubscribe");
                if state.bus.try_unsubscribe(agent_id, room).is_err() {
                    let _ = outbound_tx.send(RelayOutboundFrame::Error {
                        message_id: None,
                        error: "invalid relay room".to_owned(),
                    });
                    return true;
                }
            }
            let _ = outbound_tx.send(RelayOutboundFrame::Ack {
                event: format!("unsubscribed:{}", rooms.join(",")),
            });
            true
        }
        Ok(AgentInboundFrame::Publish {
            topic,
            msg_type,
            payload,
        }) => {
            tracing::debug!(%agent_id, %topic, %msg_type, "publish");
            let envelope = TopicEnvelope::new(&topic, &msg_type, payload).with_publisher(agent_id);
            let (seq, _delivered) = match state.try_publish_topic(envelope, Some(agent_id)) {
                Ok(published) => published,
                Err(_) => {
                    let _ = outbound_tx.send(RelayOutboundFrame::Error {
                        message_id: None,
                        error: "invalid relay topic or message type".to_owned(),
                    });
                    return true;
                }
            };
            let _ = outbound_tx.send(RelayOutboundFrame::Ack {
                event: format!("published:{topic}:{seq}"),
            });
            true
        }
        Ok(AgentInboundFrame::RegisterFeed { feed }) => {
            tracing::debug!(%agent_id, feed_id = %feed.feed_id, "register_feed");
            let frame = if state.register_feed(agent_id, feed) {
                RelayOutboundFrame::Ack {
                    event: "feed_registered".to_string(),
                }
            } else {
                RelayOutboundFrame::Error {
                    message_id: None,
                    error: "invalid feed or relay feed capacity reached".to_owned(),
                }
            };
            let _ = outbound_tx.send(frame);
            true
        }
        Ok(AgentInboundFrame::UnregisterFeed { feed_id }) => {
            tracing::debug!(%agent_id, %feed_id, "unregister_feed");
            state.unregister_feed(agent_id, &feed_id);
            let _ = outbound_tx.send(RelayOutboundFrame::Ack {
                event: "feed_unregistered".to_string(),
            });
            true
        }
        Err(error) => {
            let _ = outbound_tx.send(RelayOutboundFrame::Error {
                message_id: None,
                error: format!("invalid frame: {error}"),
            });
            true
        }
    }
}

async fn handle_events_socket(
    state: Arc<RelayState>,
    socket: WebSocket,
    _permit: tokio::sync::OwnedSemaphorePermit,
) {
    let (mut sink, mut stream) = socket.split();
    let mut events = state.subscribe_events();

    loop {
        tokio::select! {
            incoming = stream.next() => {
                match incoming {
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Err(error)) => {
                        warn!(%error, "relay events websocket receive failed");
                        break;
                    }
                    _ => {}
                }
            }
            event = events.recv() => {
                match event {
                    Ok(event) => {
                        if send_event(&mut sink, &event).await.is_err() {
                            break;
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(skipped)) => {
                        if send_raw_json(
                            &mut sink,
                            json!({
                                "type": "lagged",
                                "skipped": skipped,
                            }),
                        )
                        .await
                        .is_err()
                        {
                            break;
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                }
            }
        }
    }

    let _ = sink.close().await;
}

async fn send_event(
    sink: &mut futures::stream::SplitSink<WebSocket, Message>,
    event: &RelayEvent,
) -> Result<(), ()> {
    let payload = serde_json::to_value(event).map_err(|_| ())?;
    send_raw_json(sink, payload).await
}

async fn send_raw_json(
    sink: &mut futures::stream::SplitSink<WebSocket, Message>,
    payload: Value,
) -> Result<(), ()> {
    sink.send(Message::Text(payload.to_string().into()))
        .await
        .map_err(|_| ())
}

async fn next_text_frame(stream: &mut futures::stream::SplitStream<WebSocket>) -> Option<String> {
    loop {
        match stream.next().await {
            Some(Ok(Message::Text(text))) => return Some(text.to_string()),
            Some(Ok(Message::Close(_)) | Err(_)) | None => return None,
            Some(Ok(_)) => {}
        }
    }
}

// ── Feed metadata endpoints (A5) ────────────────────────────────────────────

/// Query parameters for the `GET /relay/topics/:topic/messages` endpoint.
#[derive(Debug, serde::Deserialize)]
struct TopicMessagesQuery {
    /// Maximum number of messages to return (default 50, max 200).
    limit: Option<usize>,
}

/// `GET /relay/topics` — list all active topics with subscriber counts.
async fn list_topics(State(state): State<Arc<RelayState>>) -> Json<Value> {
    let mut stats = state.bus.topic_stats();
    stats.sort_by(|a, b| a.0.cmp(&b.0));
    let topics: Vec<Value> = stats
        .iter()
        .map(|(topic, count)| {
            json!({
                "topic": topic,
                "subscribers": count,
            })
        })
        .collect();
    Json(json!({ "topics": topics }))
}

/// `GET /relay/topics/:topic/messages` — get recent messages from the ring buffer.
async fn topic_messages(
    State(state): State<Arc<RelayState>>,
    Path(topic): Path<String>,
    Query(params): Query<TopicMessagesQuery>,
) -> Json<Value> {
    let limit = params.limit.unwrap_or(50).min(200);
    let messages: Vec<Value> = state
        .bus
        .peek_ring_limited(&topic, limit)
        .into_iter()
        .rev()
        .take(limit)
        .map(|env| {
            json!({
                "seq": env.seq,
                "topic": env.topic,
                "msg_type": env.msg_type,
                "payload": env.payload,
                "publisher_id": env.publisher_id,
                "timestamp_ms": env.timestamp_ms,
            })
        })
        .collect();
    Json(json!({ "topic": topic, "messages": messages }))
}

/// `GET /relay/topics/:topic/subscribers` — subscriber count for a topic.
async fn topic_subscribers(
    State(state): State<Arc<RelayState>>,
    Path(topic): Path<String>,
) -> Json<Value> {
    let count = state.bus.subscriber_count(&topic);
    Json(json!({
        "topic": topic,
        "subscriber_count": count,
    }))
}

// ── Feed registration endpoints ──────────────────────────────────────────────

/// `GET /relay/feeds` — list all feeds across all agents.
async fn list_feeds(State(state): State<Arc<RelayState>>) -> Json<Value> {
    let all = state.list_feeds();
    let feeds: Vec<Value> = all
        .into_iter()
        .flat_map(|(agent_id, feeds)| {
            feeds.into_iter().map(move |feed| {
                json!({
                    "agent_id": agent_id,
                    "feed_id": feed.feed_id,
                    "topic": feed.topic,
                    "name": feed.name,
                    "description": feed.description,
                    "kind": feed.kind,
                    "rate": feed.rate,
                    "schema": feed.schema,
                })
            })
        })
        .collect();
    Json(json!({ "feeds": feeds }))
}

/// `GET /relay/feeds/:agent_id` — list feeds for a specific agent.
async fn agent_feeds(
    State(state): State<Arc<RelayState>>,
    Path(agent_id): Path<String>,
) -> Json<Value> {
    let feeds = state.agent_feeds(&agent_id);
    Json(json!({ "agent_id": agent_id, "feeds": feeds }))
}
