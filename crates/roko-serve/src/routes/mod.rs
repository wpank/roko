//! Route definitions for the roko HTTP API.
//!
//! Each submodule defines handlers for a related group of endpoints. The
//! [`build_router`] function assembles them into a single [`axum::Router`]
//! with CORS and tracing middleware.

mod agents;
mod aggregator;
pub(crate) mod arenas;
pub(crate) mod auth;
mod bench;
mod chain;
pub(crate) mod config;
mod connectors;
mod defi;
mod deployments;
mod diagnosis;
mod dream;
mod event_ingest;
mod extensions;
pub(crate) mod feeds;
mod gateway;
mod groups;
mod heartbeats;
mod integrations;
mod jobs;
mod learning;
mod marketplace;
pub(crate) mod meta;
mod metrics;
pub(crate) mod middleware;
mod neuro;
mod plans;
pub(crate) mod prds;
mod projections;
mod providers;
mod rbac_middleware;
mod recipes;
pub(crate) mod registries;
mod research;
mod route_permissions;
mod run;
mod runs;
mod secrets;
pub mod shared_runs;
pub(crate) mod sse;
mod status;
mod subscriptions;
mod swe_bench;
mod team;
mod templates;
mod triggers;
mod vision_loop;
mod webhooks;
mod workflows;
mod workspaces;
mod ws;

mod proxy_ws;
mod relay_proxy;
mod rpc_proxy;

use std::convert::Infallible;
use std::net::IpAddr;
use std::num::NonZeroU32;
use std::sync::Arc;

use super::state::AppState;
use crate::adapters::SseAdapter;
use axum::body::Body;
use axum::extract::{DefaultBodyLimit, Request, State};
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use futures::stream::{self, Stream};
use governor::clock::{Clock as _, DefaultClock};
use governor::middleware::NoOpMiddleware;
use governor::state::keyed::DefaultKeyedStateStore;
use governor::state::{InMemoryState, NotKeyed};
use governor::{Quota, RateLimiter};
use roko_core::config::ServeAuthConfig;
use serde_json::{Value, json};
use tokio::sync::broadcast;
use tower_http::trace::TraceLayer;

/// Global request-body cap. Axum's default is 2 MiB; we raise it to 4 MiB so
/// reasonably sized JSON payloads (PRDs, agent manifests, plan objects) still
/// fit while keeping the cap small enough to bound memory pressure from a
/// single hostile client. Webhook routes that accept opaque `Bytes` clamp
/// further to 1 MiB locally.
pub(crate) const DEFAULT_REQUEST_BODY_LIMIT_BYTES: usize = 4 * 1024 * 1024;

/// Default global rate limit applied to every route.
///
/// 100 requests per second is generous for legitimate traffic but bounds the
/// damage of a chatty / runaway client without per-endpoint configuration.
pub(crate) const DEFAULT_GLOBAL_RATE_PER_SEC: u32 = 100;

/// Default per-key rate limit (per API key hash or per client IP).
///
/// 30 req/s per caller keeps a single key from dominating the global budget
/// while still allowing reasonable burst traffic from each caller.
pub(crate) const DEFAULT_PER_KEY_RATE_PER_SEC: u32 = 30;

/// In-memory single-bucket rate limiter shared across all requests.
type GlobalRateLimiter = RateLimiter<NotKeyed, InMemoryState, DefaultClock, NoOpMiddleware>;

/// Per-caller keyed rate limiter. Each distinct key (API-key hash or client IP)
/// gets its own independent token bucket.
type KeyedRateLimiter =
    RateLimiter<String, DefaultKeyedStateStore<String>, DefaultClock, NoOpMiddleware>;

/// Build a non-keyed governor rate limiter with a fixed `req/s` budget.
pub(crate) fn build_global_rate_limiter(per_second: u32) -> Arc<GlobalRateLimiter> {
    let per_second =
        NonZeroU32::new(per_second.max(1)).expect("rate-limit must be non-zero (max(1) above)");
    Arc::new(RateLimiter::direct(Quota::per_second(per_second)))
}

/// Build a keyed governor rate limiter with a fixed `req/s` budget per key.
pub(crate) fn build_keyed_rate_limiter(per_second: u32) -> Arc<KeyedRateLimiter> {
    let per_second =
        NonZeroU32::new(per_second.max(1)).expect("rate-limit must be non-zero (max(1) above)");
    Arc::new(RateLimiter::keyed(Quota::per_second(per_second)))
}

/// Build a keyed governor rate limiter with a per-minute quota and explicit burst.
///
/// Used for expensive per-route limits (terminal creation, inference dispatch,
/// agent registration) where a higher burst is acceptable but sustained
/// throughput must be bounded per caller.
fn build_per_route_keyed_limiter(per_minute: u32, burst: u32) -> Arc<KeyedRateLimiter> {
    let per_minute =
        NonZeroU32::new(per_minute.max(1)).expect("rate-limit must be non-zero (max(1) above)");
    let burst = NonZeroU32::new(burst.max(1)).expect("burst must be non-zero (max(1) above)");
    Arc::new(RateLimiter::keyed(
        Quota::per_minute(per_minute).allow_burst(burst),
    ))
}

/// Extract a stable rate-limit key from a request.
///
/// Priority: authenticated API key hash > client IP > fallback constant.
/// Raw API keys are never stored or logged; we hash them with SHA-256 (using
/// the same `hash_api_key` helper that the auth middleware uses).
fn rate_limit_key(req: &Request<Body>) -> String {
    use axum::http::header::AUTHORIZATION;

    // 1. Check for API key in X-Api-Key header.
    if let Some(value) = req.headers().get("X-Api-Key") {
        if let Ok(key) = value.to_str() {
            if !key.is_empty() {
                return format!("api:{}", middleware::hash_api_key(key));
            }
        }
    }

    // 2. Check for bearer token in Authorization header.
    if let Some(value) = req.headers().get(AUTHORIZATION) {
        if let Ok(value) = value.to_str() {
            if let Some(token) = middleware::extract_bearer_token(value) {
                return format!("api:{}", middleware::hash_api_key(token));
            }
        }
    }

    // 3. Fall back to client IP from X-Forwarded-For or X-Real-Ip.
    if let Some(value) = req
        .headers()
        .get("X-Forwarded-For")
        .or_else(|| req.headers().get("X-Real-Ip"))
    {
        if let Ok(value) = value.to_str() {
            // X-Forwarded-For may contain a comma-separated list; take the
            // leftmost entry (closest to the client).
            let client_ip = value.split(',').next().unwrap_or(value).trim();
            if !client_ip.is_empty() {
                return format!("ip:{client_ip}");
            }
        }
    }

    // 4. Fall back to connected peer address (requires `ConnectInfo`).
    if let Some(addr) = req
        .extensions()
        .get::<axum::extract::ConnectInfo<std::net::SocketAddr>>()
    {
        return format!("ip:{}", addr.0.ip());
    }

    // 5. Absolute fallback — treat as a single anonymous caller.
    "anon".to_string()
}

/// Middleware: reject requests once the shared global bucket has been exhausted.
///
/// Returns 429 with a stable `code = "rate_limited"` body and a `Retry-After`
/// header so clients know when to retry. The header value is in seconds,
/// clamped to at least 1.
pub(crate) async fn rate_limit_middleware(
    State(limiter): State<Arc<GlobalRateLimiter>>,
    req: Request<Body>,
    next: Next,
) -> Response {
    match limiter.check() {
        Ok(()) => next.run(req).await,
        Err(not_until) => {
            let retry_secs = not_until
                .wait_time_from(DefaultClock::default().now())
                .as_secs()
                .max(1);
            let mut headers = axum::http::HeaderMap::new();
            if let Ok(val) = retry_secs.to_string().parse::<axum::http::HeaderValue>() {
                headers.insert("retry-after", val);
            }
            (
                StatusCode::TOO_MANY_REQUESTS,
                headers,
                Json(json!({ "code": "rate_limited", "message": "global rate limit exceeded" })),
            )
                .into_response()
        }
    }
}

/// Middleware: per-caller rate limit keyed by API-key hash or client IP.
///
/// Runs before the global backstop so a single noisy caller is throttled
/// before the shared budget is affected. Returns 429 with
/// `code = "rate_limited"`, a `Retry-After` header, and a message that does
/// **not** expose the raw key.
pub(crate) async fn keyed_rate_limit_middleware(
    State(limiter): State<Arc<KeyedRateLimiter>>,
    req: Request<Body>,
    next: Next,
) -> Response {
    let key = rate_limit_key(&req);
    match limiter.check_key(&key) {
        Ok(()) => next.run(req).await,
        Err(not_until) => {
            let retry_secs = not_until
                .wait_time_from(DefaultClock::default().now())
                .as_secs()
                .max(1);
            let mut headers = axum::http::HeaderMap::new();
            if let Ok(val) = retry_secs.to_string().parse::<axum::http::HeaderValue>() {
                headers.insert("retry-after", val);
            }
            (
                StatusCode::TOO_MANY_REQUESTS,
                headers,
                Json(json!({
                    "code": "rate_limited",
                    "message": "per-caller rate limit exceeded",
                })),
            )
                .into_response()
        }
    }
}

pub use self::config::reload_config_from_disk;
pub use self::deployments::load_persisted_deployments;
pub(crate) use self::middleware::{CorsPolicy, cors_layer};
pub(crate) use self::prds::start_prd_publish_subscriber;
pub(crate) use self::ws::apply_ws_size_limits as ws_size_limits;

/// Build the complete API router with all route groups and middleware.
pub fn build_router(
    state: Arc<AppState>,
    cors_origins: &[String],
    api_auth: ServeAuthConfig,
) -> Router {
    state
        .sse_adapter
        .set_state_hub_consumer(crate::dashboard_event_bridge(&state));
    state.sse_adapter.start_runtime_event_subscription();

    let roko_config = state.load_roko_config();
    let cors = middleware::cors_layer(&middleware::CorsPolicy {
        origins: cors_origins.to_vec(),
        unsafe_public: roko_config.server.unsafe_public_cors,
        auth_enabled: api_auth.enabled,
    });
    let terminal_enabled = roko_config.serve.terminal_enabled;

    // Per-route keyed rate limiters for expensive endpoint groups.
    // These are checked per-caller (key = API key hash or client IP) and bound
    // specific route groups independently of the global backstop.
    let terminal_create_limiter = build_per_route_keyed_limiter(2, 3);
    let infer_limiter = build_per_route_keyed_limiter(30, 10);
    let agent_reg_limiter = build_per_route_keyed_limiter(5, 5);

    // Replay the durable arena event outbox before accepting new mutations.
    // Publication is at-least-once: a crash after publish but before cursor
    // persistence may duplicate an event, but can never silently lose it.
    if let Ok(runtime) = tokio::runtime::Handle::try_current() {
        let arena_state = Arc::clone(&state);
        runtime.spawn(async move {
            if let Err(error) = arena_state
                .arenas
                .project_pending(arena_state.pulse_bus.as_ref())
                .await
            {
                tracing::warn!(%error, "arena startup event replay remains pending");
            }
        });
    }

    let api = Router::new()
        .merge(crate::openapi::routes())
        .merge(status::routes())
        .merge(jobs::routes())
        .merge(heartbeats::routes())
        .merge(plans::routes())
        .merge(prds::routes())
        .merge(run::routes().layer(axum::middleware::from_fn_with_state(
            Arc::clone(&infer_limiter),
            keyed_rate_limit_middleware,
        )))
        .merge(runs::routes())
        .merge(research::routes())
        .merge(subscriptions::routes())
        .merge(templates::routes())
        .merge(aggregator::routes())
        .merge(arenas::routes())
        .merge(meta::routes())
        .merge(agents::routes().layer(axum::middleware::from_fn_with_state(
            agent_reg_limiter,
            keyed_rate_limit_middleware,
        )))
        .merge(learning::routes())
        .merge(marketplace::routes())
        .merge(defi::routes())
        .merge(registries::routes())
        .merge(config::routes())
        .merge(deployments::routes())
        .merge(diagnosis::routes())
        .merge(integrations::routes())
        .merge(projections::routes())
        .merge(neuro::routes())
        .merge(dream::routes())
        .merge(event_ingest::routes())
        .merge(extensions::routes())
        .merge(
            gateway::routes().layer(axum::middleware::from_fn_with_state(
                infer_limiter,
                keyed_rate_limit_middleware,
            )),
        )
        .merge(chain::routes())
        .merge(connectors::routes())
        .merge(feeds::routes())
        .merge(recipes::routes())
        .merge(groups::routes())
        .merge(auth::routes())
        .merge(secrets::routes())
        .merge(vision_loop::routes())
        .merge(team::routes())
        .merge(team::join_routes())
        .merge(bench::routes())
        .merge(swe_bench::routes())
        .merge(triggers::routes())
        .merge(workflows::routes())
        .merge(workspaces::routes())
        .merge(shared_runs::auth_routes())
        .merge(webhooks::authenticated_routes())
        .nest("/providers", providers::router())
        .nest("/models", providers::models_router())
        .nest("/routing", providers::routing_router())
        .merge(sse::routes())
        .merge(rpc_proxy::routes())
        .route("/workflow/events", get(workflow_sse_handler));

    let api = if api_auth.enabled {
        api.layer(axum::middleware::from_fn_with_state(
            Arc::clone(&state),
            rbac_middleware::require_route_permission,
        ))
        .layer(axum::middleware::from_fn_with_state(
            Arc::clone(&state),
            middleware::require_scope,
        ))
        .layer(axum::middleware::from_fn_with_state(
            Arc::clone(&state),
            middleware::require_api_key,
        ))
    } else {
        api
    };

    // Install the API fallback before nesting. An outer fallback can observe a
    // prefix-stripped URI after `/api` routing and accidentally serve the SPA.
    let api = api.fallback(api_not_found);

    // Secret-scrubbing layer: redacts API keys / tokens from JSON responses.
    let scrubber = Arc::clone(&state.scrubber);
    let api = api.layer(axum::middleware::from_fn_with_state(
        scrubber,
        middleware::scrub_secrets,
    ));

    // Terminal routes always require auth + scope when enabled, even on loopback.
    let terminal = if terminal_enabled {
        crate::terminal::routes()
            .layer(axum::middleware::from_fn_with_state(
                Arc::clone(&state),
                rbac_middleware::require_route_permission,
            ))
            .layer(axum::middleware::from_fn_with_state(
                Arc::clone(&state),
                middleware::require_scope,
            ))
            .layer(axum::middleware::from_fn_with_state(
                Arc::clone(&state),
                middleware::require_api_key,
            ))
            // Per-route rate limiter applied outermost so it runs before auth.
            .layer(axum::middleware::from_fn_with_state(
                terminal_create_limiter,
                keyed_rate_limit_middleware,
            ))
    } else {
        crate::terminal::disabled_routes()
    };

    let ws = if api_auth.enabled {
        ws::routes().layer(axum::middleware::from_fn_with_state(
            Arc::clone(&state),
            middleware::require_api_key,
        ))
    } else {
        ws::routes()
    };

    let relay = if api_auth.enabled {
        relay_proxy::routes()
            .layer(axum::middleware::from_fn_with_state(
                Arc::clone(&state),
                rbac_middleware::require_route_permission,
            ))
            .layer(axum::middleware::from_fn_with_state(
                Arc::clone(&state),
                middleware::require_scope,
            ))
            .layer(axum::middleware::from_fn_with_state(
                Arc::clone(&state),
                middleware::require_api_key,
            ))
    } else {
        relay_proxy::routes()
    };

    let router = Router::new()
        // Top-level liveness probe — no auth, no /api prefix.
        .route("/health", get(top_level_health))
        // Top-level readiness probe — no auth, no /api prefix.
        .route("/ready", get(top_level_ready))
        // Standard Prometheus scrape endpoint — no auth, no /api prefix.
        .route("/metrics", get(metrics::metrics_handler))
        .merge(webhooks::public_routes())
        .merge(triggers::public_routes())
        // Public share-receipt reader: no auth required so recipients can
        // open share links without a roko API key.
        .merge(shared_runs::public_routes())
        // PTY terminal sessions for web UI — gated by config and bind policy.
        .merge(terminal)
        .nest("/api", api)
        .merge(ws)
        .merge(relay)
        // API/WS typos are JSON 404s; browser routes retain the SPA fallback.
        .fallback(crate::serve_api_or_spa_fallback);

    let rate_limiter = build_global_rate_limiter(roko_config.server.rate_limit_per_sec);
    let keyed_limiter = build_keyed_rate_limiter(roko_config.server.rate_limit_per_key_per_sec);

    router
        .layer(DefaultBodyLimit::max(DEFAULT_REQUEST_BODY_LIMIT_BYTES))
        // Global backstop runs outermost (checked second).
        .layer(axum::middleware::from_fn_with_state(
            rate_limiter,
            rate_limit_middleware,
        ))
        // Per-caller keyed limit runs innermost (checked first).
        .layer(axum::middleware::from_fn_with_state(
            keyed_limiter,
            keyed_rate_limit_middleware,
        ))
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        .with_state(state)
}

async fn api_not_found(req: Request) -> Response {
    let path = req.uri().path().to_string();
    (
        StatusCode::NOT_FOUND,
        Json(json!({
            "error": "not_found",
            "message": format!("No API route matches {path}"),
        })),
    )
        .into_response()
}

/// `GET /health` — bare liveness probe for load balancers and external tools.
///
/// Returns 200 while the process is alive. For richer telemetry use
/// `GET /api/health`.
async fn top_level_health(State(state): State<Arc<AppState>>) -> Json<Value> {
    Json(json!({
        "status": "ok",
        "version": env!("CARGO_PKG_VERSION"),
        "uptime_secs": state.started_at.elapsed().as_secs(),
    }))
}

/// `GET /ready` — readiness probe for platforms that drain shutting-down
/// instances before stopping them.
async fn top_level_ready(State(state): State<Arc<AppState>>) -> (StatusCode, Json<Value>) {
    if state.cancel.is_cancelled() {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({
                "status": "shutting_down",
                "version": env!("CARGO_PKG_VERSION"),
                "uptime_secs": state.started_at.elapsed().as_secs(),
            })),
        );
    }

    (
        StatusCode::OK,
        Json(json!({
            "status": "ok",
            "version": env!("CARGO_PKG_VERSION"),
            "uptime_secs": state.started_at.elapsed().as_secs(),
        })),
    )
}

/// `GET /api/workflow/events` — RuntimeEvent-typed SSE stream for WorkflowEngine.
async fn workflow_sse_handler(
    State(state): State<Arc<AppState>>,
) -> impl axum::response::IntoResponse {
    let adapter: &Arc<SseAdapter> = &state.sse_adapter;
    let rx = adapter.subscribe();
    let sse = workflow_sse_from_adapter(rx);
    (sse::sse_response_headers(), sse)
}

fn workflow_sse_from_adapter(
    rx: broadcast::Receiver<crate::adapters::SseEvent>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let stream = stream::unfold(rx, |mut rx| async move {
        loop {
            match rx.recv().await {
                Ok(sse_event) => {
                    let data = serde_json::to_string(&sse_event).unwrap_or_default();
                    let event = Event::default().event(sse_event.kind.clone()).data(data);
                    return Some((Ok(event), rx));
                }
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    tracing::warn!(n, "workflow SSE client lagged");
                    continue;
                }
                Err(broadcast::error::RecvError::Closed) => return None,
            }
        }
    });

    Sse::new(stream).keep_alive(
        KeepAlive::new()
            .interval(std::time::Duration::from_secs(8))
            .text("keepalive"),
    )
}

fn bind_is_loopback(bind: &str) -> bool {
    let host = bind
        .strip_prefix('[')
        .and_then(|value| value.strip_suffix(']'))
        .unwrap_or(bind);

    if host.eq_ignore_ascii_case("localhost") {
        return true;
    }

    host.parse::<IpAddr>().is_ok_and(|addr| addr.is_loopback())
}

#[cfg(test)]
mod tests {
    use super::*;

    use axum::body::Body;
    use axum::http::{Method, Request, StatusCode};
    use http_body_util::BodyExt as _;
    use roko_core::config::{RokoConfig, ServeAuthConfig};
    use serde_json::Value;
    use tempfile::tempdir;
    use tower::ServiceExt as _;

    use crate::deploy::create_backend;
    use crate::runtime::NoOpRuntime;

    fn build_test_state_and_router(
        config: RokoConfig,
    ) -> (tempfile::TempDir, Arc<AppState>, axum::Router) {
        let dir = tempdir().expect("tempdir");
        let deploy = Arc::from(create_backend("manual", None, None, None).expect("manual backend"));
        let state = Arc::new(
            AppState::new(
                dir.path().to_path_buf(),
                Arc::new(NoOpRuntime),
                config.clone(),
                deploy,
            )
            .expect("AppState::new"),
        );
        let router = build_router(Arc::clone(&state), &[], config.serve.auth.clone());
        (dir, state, router)
    }

    fn build_test_router(config: RokoConfig) -> (tempfile::TempDir, axum::Router) {
        let (dir, _state, router) = build_test_state_and_router(config);
        (dir, router)
    }

    fn build_test_router_at(
        workdir: &std::path::Path,
        config: RokoConfig,
    ) -> (Arc<AppState>, axum::Router) {
        let deploy = Arc::from(create_backend("manual", None, None, None).expect("manual backend"));
        let state = Arc::new(
            AppState::new(
                workdir.to_path_buf(),
                Arc::new(NoOpRuntime),
                config.clone(),
                deploy,
            )
            .expect("AppState::new"),
        );
        let router = build_router(Arc::clone(&state), &[], config.serve.auth.clone());
        (state, router)
    }

    async fn get_json(router: &axum::Router, uri: &str) -> (StatusCode, Value) {
        let req = Request::builder()
            .uri(uri)
            .body(Body::empty())
            .expect("build request");
        let resp = router.clone().oneshot(req).await.expect("oneshot");
        let status = resp.status();
        let body = resp
            .into_body()
            .collect()
            .await
            .expect("collect body")
            .to_bytes();
        let json: Value = serde_json::from_slice(&body).unwrap_or(Value::Null);
        (status, json)
    }

    async fn authenticated_json(
        router: &axum::Router,
        method: Method,
        uri: &str,
        api_key: &str,
        body: Value,
    ) -> (StatusCode, Value) {
        let request = Request::builder()
            .method(method)
            .uri(uri)
            .header("X-Api-Key", api_key)
            .header("Content-Type", "application/json")
            .body(Body::from(body.to_string()))
            .expect("build authenticated JSON request");
        let response = router.clone().oneshot(request).await.expect("response");
        let status = response.status();
        let bytes = response
            .into_body()
            .collect()
            .await
            .expect("collect response")
            .to_bytes();
        let body = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
        (status, body)
    }

    fn meta_root_request(name: &str) -> Value {
        let manifest = roko_agent::lifecycle::AgentExtendedManifest::new(
            roko_agent::lifecycle::AgentCoreManifest::new(format!("bounded meta-agent {name}")),
        );
        let expires_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("test clock")
            .as_secs()
            .saturating_add(3_600);
        serde_json::json!({
            "name": name,
            "manifest": serde_json::to_value(manifest).expect("manifest JSON"),
            "role": "implementer",
            "grant": {
                "tools": {
                    "read": true,
                    "write": true,
                    "exec": true,
                    "git": false,
                    "network": false
                },
                "data_scopes": [],
                "network_hosts": [],
                "max_cost_usd": 1.0,
                "expires_at": expires_at,
                "spawn": {
                    "remaining_depth": 2,
                    "max_children": 4,
                    "max_retries": 2
                }
            }
        })
    }

    #[tokio::test]
    async fn terminal_routes_are_disabled_by_default() {
        let (_dir, app) = build_test_router(RokoConfig::default());
        let (status, body) = get_json(&app, "/api/terminal/sessions").await;

        assert_eq!(status, StatusCode::FORBIDDEN);
        assert_eq!(body["error"], "Terminal disabled");
        assert_eq!(
            body["hint"],
            "Set serve.terminal_enabled=true or use --enable-terminal"
        );
    }

    #[tokio::test]
    async fn top_level_health_and_ready_are_available_without_auth() {
        let mut config = RokoConfig::default();
        config.serve.auth = ServeAuthConfig {
            enabled: true,
            api_key: "health-secret".into(),
            api_keys: Vec::new(),
            privy_app_id: None,
            jwks_providers: Vec::new(),
            privy_workspace_id: None,
            privy_allowed_roles: Vec::new(),
            enforcement_mode: Default::default(),
            invite_expiry_days: 7,
        };

        let (_dir, app) = build_test_router(config);

        let (status, body) = get_json(&app, "/health").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["status"], "ok");
        assert_eq!(body["version"], env!("CARGO_PKG_VERSION"));
        assert!(body["uptime_secs"].as_u64().is_some());

        let (status, body) = get_json(&app, "/ready").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["status"], "ok");
        assert_eq!(body["version"], env!("CARGO_PKG_VERSION"));
        assert!(body["uptime_secs"].as_u64().is_some());
    }

    #[tokio::test]
    async fn top_level_ready_reports_shutting_down_after_cancellation() {
        let (_dir, state, app) = build_test_state_and_router(RokoConfig::default());
        state.cancel.cancel();

        let (status, body) = get_json(&app, "/ready").await;

        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(body["status"], "shutting_down");
        assert_eq!(body["version"], env!("CARGO_PKG_VERSION"));
        assert!(body["uptime_secs"].as_u64().is_some());
    }

    #[tokio::test]
    async fn marketplace_mutations_deny_a_read_only_workspace_key() {
        let plaintext = "marketplace-viewer";
        let mut config = RokoConfig::default();
        config.serve.auth.enabled = true;
        config.serve.auth.api_keys = vec![roko_core::config::ApiKeyEntry {
            name: "marketplace-viewer".into(),
            key_hash: middleware::hash_api_key(plaintext),
            scope: "read".into(),
            created_at: "2026-08-16T00:00:00Z".into(),
            expires_at: None,
            last_used_at: None,
            previous_key_hashes: Vec::new(),
        }];
        let (_dir, app) = build_test_router(config);

        let read = Request::builder()
            .uri("/api/marketplace/browse")
            .header("X-Api-Key", plaintext)
            .body(Body::empty())
            .expect("build marketplace read");
        assert_eq!(
            app.clone()
                .oneshot(read)
                .await
                .expect("read response")
                .status(),
            StatusCode::OK
        );

        for uri in ["/api/marketplace/publish", "/api/marketplace/fork"] {
            let mutation = Request::builder()
                .method("POST")
                .uri(uri)
                .header("X-Api-Key", plaintext)
                .body(Body::empty())
                .expect("build marketplace mutation");
            let response = app
                .clone()
                .oneshot(mutation)
                .await
                .expect("mutation response");
            assert_eq!(response.status(), StatusCode::FORBIDDEN, "POST {uri}");
        }
    }

    #[tokio::test]
    async fn registry_lifecycle_is_authenticated_admin_only_and_queryable() {
        let viewer = "registry-viewer";
        let writer = "registry-writer";
        let admin = "registry-admin";
        let mut config = RokoConfig::default();
        config.serve.auth.enabled = true;
        config.serve.auth.api_keys = vec![
            roko_core::config::ApiKeyEntry {
                name: "registry-viewer".into(),
                key_hash: middleware::hash_api_key(viewer),
                scope: "read".into(),
                created_at: "2026-08-16T00:00:00Z".into(),
                expires_at: None,
                last_used_at: None,
                previous_key_hashes: Vec::new(),
            },
            roko_core::config::ApiKeyEntry {
                name: "registry-writer".into(),
                key_hash: middleware::hash_api_key(writer),
                scope: "write".into(),
                created_at: "2026-08-16T00:00:00Z".into(),
                expires_at: None,
                last_used_at: None,
                previous_key_hashes: Vec::new(),
            },
            roko_core::config::ApiKeyEntry {
                name: "registry-admin".into(),
                key_hash: middleware::hash_api_key(admin),
                scope: "admin".into(),
                created_at: "2026-08-16T00:00:00Z".into(),
                expires_at: None,
                last_used_at: None,
                previous_key_hashes: Vec::new(),
            },
        ];
        let (_dir, app) = build_test_router(config);

        let unauthenticated = Request::builder()
            .uri("/api/registries/passports")
            .body(Body::empty())
            .expect("build unauthenticated registry request");
        assert_eq!(
            app.clone()
                .oneshot(unauthenticated)
                .await
                .expect("unauthenticated response")
                .status(),
            StatusCode::UNAUTHORIZED
        );

        let read = Request::builder()
            .uri("/api/registries/passports")
            .header("X-Api-Key", viewer)
            .body(Body::empty())
            .expect("build registry read");
        assert_eq!(
            app.clone()
                .oneshot(read)
                .await
                .expect("read response")
                .status(),
            StatusCode::OK
        );

        let payload = serde_json::json!({
            "owner": "did:key:operator",
            "capabilities": ["knowledge"],
            "system_prompt_hash": "11".repeat(32),
            "initial_stake": 0,
        });
        let denied = Request::builder()
            .method("POST")
            .uri("/api/registries/passports")
            .header("X-Api-Key", viewer)
            .header("Content-Type", "application/json")
            .body(Body::from(payload.to_string()))
            .expect("build denied registry mutation");
        assert_eq!(
            app.clone()
                .oneshot(denied)
                .await
                .expect("denied response")
                .status(),
            StatusCode::FORBIDDEN
        );

        for (method, uri) in [
            (Method::POST, "/api/registries/passports/1/transfer"),
            (Method::PUT, "/api/registries/passports/1/metadata"),
            (Method::POST, "/api/registries/passports/1/delegations"),
            (
                Method::DELETE,
                "/api/registries/passports/1/delegations/2?owner=test",
            ),
            (Method::POST, "/api/registries/knowledge"),
            (Method::POST, "/api/registries/knowledge/abc/validate"),
            (Method::POST, "/api/registries/knowledge/abc/challenge"),
            (
                Method::POST,
                "/api/registries/knowledge/challenges/abc/resolve",
            ),
            (Method::POST, "/api/registries/indexer/sync"),
            (Method::POST, "/api/registries/indexer/rebuild"),
        ] {
            let denied = Request::builder()
                .method(method.clone())
                .uri(uri)
                .header("X-Api-Key", viewer)
                .header("Content-Type", "application/json")
                .body(Body::from("{}"))
                .expect("build denied registry mutation");
            assert_eq!(
                app.clone()
                    .oneshot(denied)
                    .await
                    .expect("denied response")
                    .status(),
                StatusCode::FORBIDDEN,
                "{method} {uri}"
            );
        }

        let writer_denied = Request::builder()
            .method(Method::POST)
            .uri("/api/registries/indexer/rebuild")
            .header("X-Api-Key", writer)
            .body(Body::empty())
            .expect("build write-scoped registry mutation");
        assert_eq!(
            app.clone()
                .oneshot(writer_denied)
                .await
                .expect("write-scoped response")
                .status(),
            StatusCode::FORBIDDEN
        );

        let created = Request::builder()
            .method("POST")
            .uri("/api/registries/passports")
            .header("X-Api-Key", admin)
            .header("Content-Type", "application/json")
            .body(Body::from(payload.to_string()))
            .expect("build admin registry mutation");
        assert_eq!(
            app.clone()
                .oneshot(created)
                .await
                .expect("created response")
                .status(),
            StatusCode::CREATED
        );

        let detail = Request::builder()
            .uri("/api/registries/passports/1")
            .header("X-Api-Key", viewer)
            .body(Body::empty())
            .expect("build registry detail read");
        let response = app.clone().oneshot(detail).await.expect("detail response");
        assert_eq!(response.status(), StatusCode::OK);
        let body = response
            .into_body()
            .collect()
            .await
            .expect("collect registry detail")
            .to_bytes();
        let body: Value = serde_json::from_slice(&body).expect("registry detail JSON");
        assert_eq!(body["passport"]["owner"], "did:key:operator");

        for owner in ["did:key:validator", "did:key:challenger"] {
            let (status, _) = authenticated_json(
                &app,
                Method::POST,
                "/api/registries/passports",
                admin,
                serde_json::json!({
                    "owner": owner,
                    "capabilities": ["knowledge"],
                    "system_prompt_hash": "12".repeat(32),
                    "initial_stake": 0,
                }),
            )
            .await;
            assert_eq!(status, StatusCode::CREATED);
        }

        let (status, _) = authenticated_json(
            &app,
            Method::POST,
            "/api/registries/passports/1/transfer",
            admin,
            serde_json::json!({
                "from": "did:key:operator",
                "to": "did:key:new-owner",
                "block": 7,
            }),
        )
        .await;
        assert_eq!(status, StatusCode::OK);

        let (status, _) = authenticated_json(
            &app,
            Method::PUT,
            "/api/registries/passports/1/metadata",
            admin,
            serde_json::json!({
                "owner": "did:key:new-owner",
                "service_endpoints": ["https://agent.example"],
                "feeds": ["feed://knowledge"],
            }),
        )
        .await;
        assert_eq!(status, StatusCode::OK);

        let (status, _) = authenticated_json(
            &app,
            Method::POST,
            "/api/registries/passports/1/delegations",
            admin,
            serde_json::json!({
                "owner": "did:key:new-owner",
                "delegatee": 2,
                "capabilities": ["knowledge"],
                "expiry_block": 100,
                "current_block": 7,
            }),
        )
        .await;
        assert_eq!(status, StatusCode::OK);

        let (status, knowledge) = authenticated_json(
            &app,
            Method::POST,
            "/api/registries/knowledge",
            admin,
            serde_json::json!({
                "publisher_id": 1,
                "content_hash": "22".repeat(32),
                "tags": ["defi"],
                "published_at": 10,
            }),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);
        let entry_id = knowledge["entry_id"].as_str().expect("published entry id");

        let (status, _) = authenticated_json(
            &app,
            Method::POST,
            &format!("/api/registries/knowledge/{entry_id}/validate"),
            admin,
            serde_json::json!({ "validator_id": 2 }),
        )
        .await;
        assert_eq!(status, StatusCode::OK);

        let (status, challenge) = authenticated_json(
            &app,
            Method::POST,
            &format!("/api/registries/knowledge/{entry_id}/challenge"),
            admin,
            serde_json::json!({
                "challenger_id": 3,
                "evidence_hash": "33".repeat(32),
                "reason": "counter-evidence",
                "resolution_deadline": 100,
            }),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);
        let challenge_id = challenge["challenge_id"].as_str().expect("challenge id");

        let (status, _) = authenticated_json(
            &app,
            Method::POST,
            &format!("/api/registries/knowledge/challenges/{challenge_id}/resolve"),
            admin,
            serde_json::json!({ "upheld": false }),
        )
        .await;
        assert_eq!(status, StatusCode::OK);

        let (status, _) = authenticated_json(
            &app,
            Method::DELETE,
            "/api/registries/passports/1/delegations/2?owner=did:key:new-owner",
            admin,
            serde_json::json!({}),
        )
        .await;
        assert_eq!(status, StatusCode::NO_CONTENT);

        let history = Request::builder()
            .uri("/api/registries/passports/1/history")
            .header("X-Api-Key", viewer)
            .body(Body::empty())
            .expect("build history request");
        let response = app.oneshot(history).await.expect("history response");
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn arena_service_is_authenticated_classified_and_live() {
        let mut config = RokoConfig::default();
        config.serve.auth.enabled = true;
        config.serve.auth.api_key = "arena-admin".into();
        let (_dir, app) = build_test_router(config);

        let (status, body) = get_json(&app, "/api/arenas").await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert_eq!(body["code"], "unauthorized");

        let request = Request::builder()
            .uri("/api/arenas")
            .header("X-Api-Key", "arena-admin")
            .body(Body::empty())
            .expect("build arena request");
        let response = app.clone().oneshot(request).await.expect("oneshot");
        assert_eq!(response.status(), StatusCode::OK);

        let request = Request::builder()
            .method("POST")
            .uri("/api/arenas/demo/attempts")
            .header("X-Api-Key", "arena-admin")
            .body(Body::empty())
            .expect("build arena attempt request");
        let response = app.oneshot(request).await.expect("oneshot");
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let body = response
            .into_body()
            .collect()
            .await
            .expect("collect body")
            .to_bytes();
        let body: Value = serde_json::from_slice(&body).expect("JSON body");
        assert_eq!(body["code"], "invalid_json");
    }

    #[tokio::test]
    async fn arena_mutations_fail_closed_when_serve_auth_is_disabled() {
        let mut config = RokoConfig::default();
        config.serve.auth.enabled = false;
        let (_dir, app) = build_test_router(config);
        let request = Request::builder()
            .method(Method::POST)
            .uri("/api/arenas")
            .header("Content-Type", "application/json")
            .body(Body::from(
                serde_json::json!({
                    "name": "unauthorized",
                    "category": "coding",
                    "task_source": "static",
                    "scoring": { "binary": "test_suite_pass" },
                    "aggregation": "median",
                    "ground_truth": "test_suite",
                })
                .to_string(),
            ))
            .expect("build unauthenticated arena mutation");
        let response = app.oneshot(request).await.expect("arena response");
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn arena_mutation_denies_a_read_only_workspace_key() {
        let plaintext = "arena-viewer";
        let mut config = RokoConfig::default();
        config.serve.auth.enabled = true;
        config.serve.auth.api_keys = vec![roko_core::config::ApiKeyEntry {
            name: "arena-viewer".into(),
            key_hash: middleware::hash_api_key(plaintext),
            scope: "read".into(),
            created_at: "2026-08-16T00:00:00Z".into(),
            expires_at: None,
            last_used_at: None,
            previous_key_hashes: Vec::new(),
        }];
        let (_dir, app) = build_test_router(config);

        let read = Request::builder()
            .uri("/api/arenas")
            .header("X-Api-Key", plaintext)
            .body(Body::empty())
            .expect("build arena read");
        assert_eq!(
            app.clone()
                .oneshot(read)
                .await
                .expect("read response")
                .status(),
            StatusCode::OK
        );

        let mutation = Request::builder()
            .method("POST")
            .uri("/api/arenas/demo/attempts")
            .header("X-Api-Key", plaintext)
            .body(Body::empty())
            .expect("build arena mutation");
        let response = app.oneshot(mutation).await.expect("mutation response");
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        let body = response
            .into_body()
            .collect()
            .await
            .expect("collect denial")
            .to_bytes();
        let body: Value = serde_json::from_slice(&body).expect("JSON denial");
        assert_eq!(body["code"], "insufficient_scope");
    }

    #[tokio::test]
    async fn arena_owner_and_admin_settle_external_evidence_and_project_events() {
        let owner = "arena-owner-key";
        let participant = "arena-participant-key";
        let admin = "arena-admin-key";
        let reader = "arena-reader-key";
        let mut config = RokoConfig::default();
        config.serve.auth.enabled = true;
        config.serve.auth.api_keys = [
            ("arena-owner", owner, "write"),
            ("arena-participant", participant, "write"),
            ("arena-admin", admin, "admin"),
            ("arena-reader", reader, "read"),
        ]
        .into_iter()
        .map(|(name, key, scope)| roko_core::config::ApiKeyEntry {
            name: name.to_string(),
            key_hash: middleware::hash_api_key(key),
            scope: scope.to_string(),
            created_at: "2026-08-16T00:00:00Z".into(),
            expires_at: None,
            last_used_at: None,
            previous_key_hashes: Vec::new(),
        })
        .collect();
        let dir = tempdir().expect("tempdir");
        let (state, app) = build_test_router_at(dir.path(), config);

        let (status, created) = authenticated_json(
            &app,
            Method::POST,
            "/api/arenas",
            owner,
            serde_json::json!({
                "name": "external scoring arena",
                "category": "coding",
                "task_source": "static",
                "scoring": { "binary": "test_suite_pass" },
                "aggregation": "median",
                "max_attempts_per_agent": 2,
                "cooldown_blocks": 0,
                "ground_truth": "test_suite",
            }),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);
        let arena_id = created["arena_id"].as_str().expect("arena id");

        let (status, denial) = authenticated_json(
            &app,
            Method::PATCH,
            &format!("/api/arenas/{arena_id}"),
            participant,
            serde_json::json!({ "action": "activate" }),
        )
        .await;
        assert_eq!(status, StatusCode::FORBIDDEN);
        assert_eq!(denial["code"], "forbidden");

        let (status, _) = authenticated_json(
            &app,
            Method::PATCH,
            &format!("/api/arenas/{arena_id}"),
            owner,
            serde_json::json!({ "action": "activate" }),
        )
        .await;
        assert_eq!(status, StatusCode::OK);

        let (status, _) = authenticated_json(
            &app,
            Method::POST,
            &format!("/api/arenas/{arena_id}/attempts"),
            participant,
            serde_json::json!({
                "agent_identity_id": 22,
                "task_hash": "22".repeat(32),
            }),
        )
        .await;
        assert_eq!(status, StatusCode::FORBIDDEN);

        let (status, started) = authenticated_json(
            &app,
            Method::POST,
            &format!("/api/arenas/{arena_id}/attempts"),
            participant,
            serde_json::json!({ "task_hash": "22".repeat(32) }),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);
        let attempt_id = started["attempt_id"].as_str().expect("attempt id");

        let (status, _) = authenticated_json(
            &app,
            Method::POST,
            &format!("/api/arenas/{arena_id}/attempts/{attempt_id}/submit"),
            participant,
            serde_json::json!({ "output_hash": "33".repeat(32) }),
        )
        .await;
        assert_eq!(status, StatusCode::OK);

        let settlement = serde_json::json!({
            "source": "test_suite",
            "evidence_hash": "44".repeat(32),
            "subject_output_hash": "33".repeat(32),
            "settlement": {
                "outcome": "completed",
                "score": 0.8,
                "gate_verdicts": [true]
            }
        });
        let (status, _) = authenticated_json(
            &app,
            Method::POST,
            &format!("/api/arenas/{arena_id}/attempts/{attempt_id}/settle"),
            participant,
            settlement.clone(),
        )
        .await;
        assert_eq!(status, StatusCode::FORBIDDEN);

        let mut wrong_subject = settlement.clone();
        wrong_subject["subject_output_hash"] = serde_json::json!("55".repeat(32));
        let (status, _) = authenticated_json(
            &app,
            Method::POST,
            &format!("/api/arenas/{arena_id}/attempts/{attempt_id}/settle"),
            admin,
            wrong_subject,
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);

        let mut wrong_source = settlement.clone();
        wrong_source["source"] = serde_json::json!({ "external_oracle": "wrong" });
        let (status, _) = authenticated_json(
            &app,
            Method::POST,
            &format!("/api/arenas/{arena_id}/attempts/{attempt_id}/settle"),
            admin,
            wrong_source,
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);

        let (status, settled) = authenticated_json(
            &app,
            Method::POST,
            &format!("/api/arenas/{arena_id}/attempts/{attempt_id}/settle"),
            admin,
            settlement,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(settled["attempt"]["state"], "completed");
        assert_eq!(settled["attempt"]["score"], 0.8);

        let request = Request::builder()
            .uri(format!("/api/arenas/{arena_id}/leaderboard"))
            .header("X-Api-Key", reader)
            .body(Body::empty())
            .expect("build leaderboard request");
        let response = app.oneshot(request).await.expect("leaderboard response");
        assert_eq!(response.status(), StatusCode::OK);
        let pulses = state.pulse_bus.replay_from(
            0,
            Some(&roko_core::TopicFilter::Prefix("arena.".to_string())),
        );
        assert!(
            pulses
                .iter()
                .any(|pulse| pulse.topic.0.as_str() == "arena.attempt_completed")
        );
    }

    #[tokio::test]
    async fn meta_mutations_fail_closed_when_serve_auth_is_disabled() {
        let mut config = RokoConfig::default();
        config.serve.auth.enabled = false;
        let (_dir, app) = build_test_router(config);
        let request = Request::builder()
            .method(Method::POST)
            .uri("/api/meta/agents")
            .header("Content-Type", "application/json")
            .body(Body::from(meta_root_request("unauthenticated").to_string()))
            .expect("build unauthenticated meta proposal");
        let response = app.oneshot(request).await.expect("meta response");
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn meta_activation_is_owned_arena_bound_single_use_and_fail_closed() {
        let owner = "meta-owner-key";
        let scorer = "meta-scorer-key";
        let intruder = "meta-intruder-key";
        let reader = "meta-reader-key";
        let mut config = RokoConfig::default();
        config.serve.auth.enabled = true;
        config.serve.auth.api_keys = [
            ("meta-owner", owner, "admin"),
            ("meta-scorer", scorer, "admin"),
            ("meta-intruder", intruder, "agent:write"),
            ("meta-reader", reader, "read"),
        ]
        .into_iter()
        .map(|(name, key, scope)| roko_core::config::ApiKeyEntry {
            name: name.to_string(),
            key_hash: middleware::hash_api_key(key),
            scope: scope.to_string(),
            created_at: "2026-08-16T00:00:00Z".into(),
            expires_at: None,
            last_used_at: None,
            previous_key_hashes: Vec::new(),
        })
        .collect();
        let dir = tempdir().expect("tempdir");
        let (_state, app) = build_test_router_at(dir.path(), config.clone());

        let (status, denial) = authenticated_json(
            &app,
            Method::POST,
            "/api/meta/agents",
            reader,
            meta_root_request("read-only"),
        )
        .await;
        assert_eq!(status, StatusCode::FORBIDDEN);
        assert_eq!(denial["code"], "insufficient_scope");

        let (status, denial) = authenticated_json(
            &app,
            Method::POST,
            "/api/meta/agents",
            intruder,
            meta_root_request("non-admin-root"),
        )
        .await;
        assert_eq!(status, StatusCode::FORBIDDEN);
        assert_eq!(denial["code"], "forbidden");

        let (status, proposal) = authenticated_json(
            &app,
            Method::POST,
            "/api/meta/agents",
            owner,
            meta_root_request("accepted"),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED, "{proposal}");
        let proposal_id = proposal["id"].as_str().expect("proposal id");
        let artifact_hash = proposal["activation_artifact_hash"]
            .as_str()
            .expect("artifact hash");

        let request = Request::builder()
            .uri(format!("/api/meta/agents/{proposal_id}"))
            .header("X-Api-Key", intruder)
            .body(Body::empty())
            .expect("build foreign meta read");
        let response = app.clone().oneshot(request).await.expect("foreign read");
        assert_eq!(response.status(), StatusCode::FORBIDDEN);

        let (status, arena) = authenticated_json(
            &app,
            Method::POST,
            "/api/arenas",
            owner,
            serde_json::json!({
                "name": "meta activation arena",
                "category": "coding",
                "task_source": "static",
                "scoring": { "binary": "test_suite_pass" },
                "aggregation": "median",
                "max_attempts_per_agent": 8,
                "cooldown_blocks": 0,
                "ground_truth": "test_suite"
            }),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED, "{arena}");
        let arena_id = arena["arena_id"].as_str().expect("arena id");
        let (status, _) = authenticated_json(
            &app,
            Method::PATCH,
            &format!("/api/arenas/{arena_id}"),
            owner,
            serde_json::json!({ "action": "activate" }),
        )
        .await;
        assert_eq!(status, StatusCode::OK);

        let (status, attempt) = authenticated_json(
            &app,
            Method::POST,
            &format!("/api/arenas/{arena_id}/attempts"),
            owner,
            serde_json::json!({ "task_hash": "21".repeat(32) }),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED, "{attempt}");
        let attempt_id = attempt["attempt_id"].as_str().expect("attempt id");
        let (status, _) = authenticated_json(
            &app,
            Method::POST,
            &format!("/api/arenas/{arena_id}/attempts/{attempt_id}/submit"),
            owner,
            serde_json::json!({ "output_hash": artifact_hash }),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        let evidence_hash = "44".repeat(32);
        let (status, settled) = authenticated_json(
            &app,
            Method::POST,
            &format!("/api/arenas/{arena_id}/attempts/{attempt_id}/settle"),
            scorer,
            serde_json::json!({
                "source": "test_suite",
                "evidence_hash": evidence_hash,
                "subject_output_hash": artifact_hash,
                "settlement": {
                    "outcome": "completed",
                    "score": 1.0,
                    "gate_verdicts": [true]
                }
            }),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{settled}");

        let validation = serde_json::json!({
            "arena_id": arena_id,
            "attempt_id": attempt_id,
            "evidence_hash": "44".repeat(32)
        });
        let (status, active) = authenticated_json(
            &app,
            Method::POST,
            &format!("/api/meta/agents/{proposal_id}/validate"),
            owner,
            validation.clone(),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{active}");
        assert_eq!(active["state"], "active");
        assert_eq!(
            active["acceptance_evidence"]["subject_output_hash"]
                .as_array()
                .expect("bound output hash")
                .len(),
            32
        );
        assert_eq!(
            active["safety_evidence"]["decision"]["verdicts"]
                .as_array()
                .expect("five-head verdicts")
                .len(),
            5
        );

        // A completed activation cannot replay the validation route/evidence.
        let (status, replay) = authenticated_json(
            &app,
            Method::POST,
            &format!("/api/meta/agents/{proposal_id}/validate"),
            owner,
            validation,
        )
        .await;
        assert_eq!(status, StatusCode::CONFLICT, "{replay}");

        let (status, morphed) = authenticated_json(
            &app,
            Method::POST,
            &format!("/api/meta/agents/{proposal_id}/morph"),
            owner,
            serde_json::json!({ "role": "auditor" }),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{morphed}");
        assert_eq!(morphed["role"], "auditor");
        assert_eq!(morphed["previous_role"], "implementer");
        assert_eq!(morphed["grant"]["tools"]["write"], false);
        assert_eq!(morphed["grant"]["tools"]["exec"], false);

        let (status, rolled_back) = authenticated_json(
            &app,
            Method::POST,
            &format!("/api/meta/agents/{proposal_id}/morph/rollback"),
            owner,
            serde_json::json!({}),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{rolled_back}");
        assert_eq!(rolled_back["role"], "implementer");
        assert_eq!(rolled_back["grant"], rolled_back["activation_grant"]);

        let (status, deactivated) = authenticated_json(
            &app,
            Method::POST,
            &format!("/api/meta/agents/{proposal_id}/deactivate"),
            owner,
            serde_json::json!({}),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{deactivated}");
        assert_eq!(deactivated["state"], "deactivated");

        let (status, missing) = authenticated_json(
            &app,
            Method::POST,
            "/api/meta/agents",
            owner,
            meta_root_request("missing-arena"),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED, "{missing}");
        let missing_id = missing["id"].as_str().expect("missing proposal id");
        let (status, rejected) = authenticated_json(
            &app,
            Method::POST,
            &format!("/api/meta/agents/{missing_id}/validate"),
            owner,
            serde_json::json!({
                "arena_id": "51".repeat(32),
                "attempt_id": "52".repeat(32),
                "evidence_hash": "53".repeat(32)
            }),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{rejected}");
        assert_eq!(rejected["state"], "rejected");

        let (status, failed) = authenticated_json(
            &app,
            Method::POST,
            "/api/meta/agents",
            owner,
            meta_root_request("failed-gates"),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED, "{failed}");
        let failed_id = failed["id"].as_str().expect("failed proposal id");
        let failed_artifact = failed["activation_artifact_hash"]
            .as_str()
            .expect("failed artifact");
        let (status, failed_attempt) = authenticated_json(
            &app,
            Method::POST,
            &format!("/api/arenas/{arena_id}/attempts"),
            owner,
            serde_json::json!({ "task_hash": "61".repeat(32) }),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED, "{failed_attempt}");
        let failed_attempt_id = failed_attempt["attempt_id"]
            .as_str()
            .expect("failed attempt id");
        let (status, _) = authenticated_json(
            &app,
            Method::POST,
            &format!("/api/arenas/{arena_id}/attempts/{failed_attempt_id}/submit"),
            owner,
            serde_json::json!({ "output_hash": failed_artifact }),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        let (status, _) = authenticated_json(
            &app,
            Method::POST,
            &format!("/api/arenas/{arena_id}/attempts/{failed_attempt_id}/settle"),
            scorer,
            serde_json::json!({
                "source": "test_suite",
                "evidence_hash": "64".repeat(32),
                "subject_output_hash": failed_artifact,
                "settlement": {
                    "outcome": "completed",
                    "score": 1.0,
                    "gate_verdicts": [false]
                }
            }),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        let (status, rejected) = authenticated_json(
            &app,
            Method::POST,
            &format!("/api/meta/agents/{failed_id}/validate"),
            owner,
            serde_json::json!({
                "arena_id": arena_id,
                "attempt_id": failed_attempt_id,
                "evidence_hash": "64".repeat(32)
            }),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{rejected}");
        assert_eq!(rejected["state"], "rejected");

        let (_restarted_state, restarted) = build_test_router_at(dir.path(), config);
        let request = Request::builder()
            .uri(format!("/api/meta/agents/{proposal_id}"))
            .header("X-Api-Key", owner)
            .body(Body::empty())
            .expect("build restarted meta read");
        let response = restarted.oneshot(request).await.expect("restarted read");
        assert_eq!(response.status(), StatusCode::OK);
        let body = response
            .into_body()
            .collect()
            .await
            .expect("collect restarted meta")
            .to_bytes();
        let body: Value = serde_json::from_slice(&body).expect("restarted meta JSON");
        assert_eq!(body["state"], "deactivated");
    }

    #[tokio::test]
    async fn defi_stubs_are_authenticated_classified_and_explicit() {
        let mut config = RokoConfig::default();
        config.serve.auth.enabled = true;
        config.serve.auth.api_key = "defi-admin".into();
        let (_dir, app) = build_test_router(config);

        let (status, body) = get_json(&app, "/api/defi/instruments").await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert_eq!(body["code"], "unauthorized");

        let routes = [
            ("GET", "/api/defi/instruments"),
            ("POST", "/api/defi/bonds"),
            ("GET", "/api/defi/bonds/bond-1"),
            ("POST", "/api/defi/options/price"),
            ("POST", "/api/defi/insurance"),
            ("POST", "/api/defi/insurance/policy-1/claims"),
            ("GET", "/api/defi/indices"),
            ("GET", "/api/defi/risk/portfolio"),
        ];
        for (method, uri) in routes {
            let request = Request::builder()
                .method(method)
                .uri(uri)
                .header("X-Api-Key", "defi-admin")
                .body(Body::empty())
                .expect("build DeFi stub request");
            let response = app.clone().oneshot(request).await.expect("oneshot");
            assert_eq!(
                response.status(),
                StatusCode::NOT_IMPLEMENTED,
                "{method} {uri}"
            );
            let body = response
                .into_body()
                .collect()
                .await
                .expect("collect DeFi stub body")
                .to_bytes();
            let body: Value = serde_json::from_slice(&body).expect("JSON body");
            assert_eq!(body["status"], "not_implemented");
            assert_eq!(body["message"], "DeFi product endpoints are Phase 2");
        }
    }

    #[tokio::test]
    async fn defi_mutation_denies_a_read_only_workspace_key() {
        let plaintext = "defi-viewer";
        let mut config = RokoConfig::default();
        config.serve.auth.enabled = true;
        config.serve.auth.api_keys = vec![roko_core::config::ApiKeyEntry {
            name: "defi-viewer".into(),
            key_hash: middleware::hash_api_key(plaintext),
            scope: "read".into(),
            created_at: "2026-08-16T00:00:00Z".into(),
            expires_at: None,
            last_used_at: None,
            previous_key_hashes: Vec::new(),
        }];
        let (_dir, app) = build_test_router(config);

        let read = Request::builder()
            .uri("/api/defi/instruments")
            .header("X-Api-Key", plaintext)
            .body(Body::empty())
            .expect("build DeFi read");
        assert_eq!(
            app.clone()
                .oneshot(read)
                .await
                .expect("read response")
                .status(),
            StatusCode::NOT_IMPLEMENTED
        );

        for uri in [
            "/api/defi/bonds",
            "/api/defi/options/price",
            "/api/defi/insurance",
            "/api/defi/insurance/policy-1/claims",
        ] {
            let mutation = Request::builder()
                .method("POST")
                .uri(uri)
                .header("X-Api-Key", plaintext)
                .body(Body::empty())
                .expect("build DeFi mutation");
            let response = app
                .clone()
                .oneshot(mutation)
                .await
                .expect("mutation response");
            assert_eq!(response.status(), StatusCode::FORBIDDEN, "POST {uri}");
            let body = response
                .into_body()
                .collect()
                .await
                .expect("collect denial")
                .to_bytes();
            let body: Value = serde_json::from_slice(&body).expect("JSON denial");
            assert_eq!(body["code"], "insufficient_scope");
        }
    }

    #[tokio::test]
    async fn terminal_routes_require_auth_even_on_loopback() {
        let mut config = RokoConfig::default();
        config.serve.terminal_enabled = true;
        // Loopback bind, no auth configured — terminal still requires auth.

        let (_dir, app) = build_test_router(config);
        let (status, body) = get_json(&app, "/api/terminal/sessions").await;

        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert_eq!(body["code"], "unauthorized");
    }

    #[tokio::test]
    async fn terminal_routes_require_auth_on_public_bind() {
        let mut config = RokoConfig::default();
        config.server.bind = "0.0.0.0".into();
        config.serve.terminal_enabled = true;
        config.serve.auth = ServeAuthConfig {
            enabled: true,
            api_key: "terminal-secret".into(),
            api_keys: Vec::new(),
            privy_app_id: None,
            jwks_providers: Vec::new(),
            privy_workspace_id: None,
            privy_allowed_roles: Vec::new(),
            enforcement_mode: Default::default(),
            invite_expiry_days: 7,
        };

        let (_dir, app) = build_test_router(config);
        let (status, body) = get_json(&app, "/api/terminal/sessions").await;

        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert_eq!(body["code"], "unauthorized");
    }

    /// Terminal routes require `terminal:write` (or `write`/`admin`) scope.
    #[tokio::test]
    async fn terminal_requires_scope() {
        use crate::routes::middleware::required_scope_for;
        use axum::http::Method;

        // POST to terminal sessions requires terminal:write scope.
        let scope = required_scope_for(&Method::POST, "/api/terminal/sessions");
        assert_eq!(scope, "terminal:write");

        // GET is read-only — scope is "read".
        let ws_scope = required_scope_for(&Method::GET, "/ws/terminal/abc-123");
        assert_eq!(ws_scope, "read");

        // DELETE (destroy session) requires terminal:write.
        let del_scope = required_scope_for(&Method::DELETE, "/api/terminal/sessions/abc-123");
        assert_eq!(del_scope, "terminal:write");

        // POST to terminal input requires terminal:write.
        let input_scope = required_scope_for(&Method::POST, "/api/terminal/sessions/abc-123/input");
        assert_eq!(input_scope, "terminal:write");
    }

    /// Verify that `DefaultBodyLimit::max(4 MiB)` rejects a 4 MiB + 1 byte
    /// body via axum's standard 413 path. We isolate the layer with a tiny
    /// `Bytes`-extracting handler so the assertion holds independent of
    /// route-specific parsing (which would otherwise mask the body cap).
    #[tokio::test]
    async fn body_size_limit_returns_413_for_oversized_payload() {
        async fn echo_body(_: axum::body::Bytes) -> StatusCode {
            StatusCode::OK
        }

        let app = axum::Router::new()
            .route("/echo", axum::routing::post(echo_body))
            .layer(DefaultBodyLimit::max(DEFAULT_REQUEST_BODY_LIMIT_BYTES));

        let oversized = vec![b'a'; DEFAULT_REQUEST_BODY_LIMIT_BYTES + 1];
        let req = Request::builder()
            .method("POST")
            .uri("/echo")
            .body(Body::from(oversized))
            .expect("build request");
        let resp = app.clone().oneshot(req).await.expect("oneshot");
        assert_eq!(resp.status(), StatusCode::PAYLOAD_TOO_LARGE);

        // Sanity-check that an in-budget body still goes through.
        let in_budget = vec![b'a'; 1024];
        let req = Request::builder()
            .method("POST")
            .uri("/echo")
            .body(Body::from(in_budget))
            .expect("build request");
        let resp = app.oneshot(req).await.expect("oneshot");
        assert_eq!(resp.status(), StatusCode::OK);
    }

    /// Drive the rate-limit middleware directly with a tiny budget. We use
    /// `axum::middleware::from_fn_with_state` rather than `build_router` so
    /// we don't have to defeat the production limiter (100 req/s is too high
    /// to exhaust deterministically in a unit test).
    #[tokio::test]
    async fn rate_limit_middleware_returns_429_when_exceeded() {
        let limiter = build_global_rate_limiter(2);
        let app = axum::Router::new()
            .route("/ping", axum::routing::get(|| async { "pong" }))
            .layer(axum::middleware::from_fn_with_state(
                limiter,
                rate_limit_middleware,
            ));

        // First two requests fit inside the per-second budget.
        for _ in 0..2 {
            let req = Request::builder()
                .uri("/ping")
                .body(Body::empty())
                .expect("build request");
            let resp = app.clone().oneshot(req).await.expect("oneshot");
            assert_eq!(resp.status(), StatusCode::OK);
        }

        // The third immediate request must be throttled because the bucket
        // has been drained (governor refills at 2 tokens/sec, so a burst of
        // 3 within the same instant is guaranteed to overflow).
        let req = Request::builder()
            .uri("/ping")
            .body(Body::empty())
            .expect("build request");
        let resp = app.oneshot(req).await.expect("oneshot");
        assert_eq!(resp.status(), StatusCode::TOO_MANY_REQUESTS);
        let body = resp
            .into_body()
            .collect()
            .await
            .expect("collect body")
            .to_bytes();
        let json: Value = serde_json::from_slice(&body).expect("json body");
        assert_eq!(json["code"], "rate_limited");
    }

    /// The 429 response from the rate-limit middleware must include a
    /// `Retry-After` header whose value is a positive integer (seconds to wait).
    #[tokio::test]
    async fn rate_limit_429_includes_retry_after_header() {
        let limiter = build_global_rate_limiter(2);
        let app = axum::Router::new()
            .route("/ping", axum::routing::get(|| async { "pong" }))
            .layer(axum::middleware::from_fn_with_state(
                limiter,
                rate_limit_middleware,
            ));

        // Drain the budget.
        for _ in 0..2 {
            let req = Request::builder()
                .uri("/ping")
                .body(Body::empty())
                .expect("build request");
            let _ = app.clone().oneshot(req).await.expect("oneshot");
        }

        // The next request must be throttled and carry `Retry-After`.
        let req = Request::builder()
            .uri("/ping")
            .body(Body::empty())
            .expect("build request");
        let resp = app.oneshot(req).await.expect("oneshot");
        assert_eq!(resp.status(), StatusCode::TOO_MANY_REQUESTS);

        let header_val = resp
            .headers()
            .get("retry-after")
            .expect("Retry-After header must be present on 429");
        let secs: u64 = header_val
            .to_str()
            .expect("Retry-After must be ASCII")
            .parse()
            .expect("Retry-After must be a non-negative integer");
        assert!(
            secs >= 1,
            "Retry-After must be at least 1 second, got {secs}"
        );
    }

    /// With `auth.enabled = true`, unauthenticated requests to `/relay/*`
    /// (both HTTP and the WS upgrade paths) must be rejected with 401.
    #[tokio::test]
    async fn relay_requires_auth_when_enabled() {
        let mut config = RokoConfig::default();
        config.serve.auth = ServeAuthConfig {
            enabled: true,
            api_key: "relay-secret".into(),
            api_keys: Vec::new(),
            privy_app_id: None,
            jwks_providers: Vec::new(),
            privy_workspace_id: None,
            privy_allowed_roles: Vec::new(),
            enforcement_mode: Default::default(),
            invite_expiry_days: 7,
        };

        let (_dir, app) = build_test_router(config);

        // GET /relay/health — unauthenticated → 401
        let (status, body) = get_json(&app, "/relay/health").await;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "GET /relay/health");
        assert_eq!(body["code"], "unauthorized");

        // POST /relay/agents — unauthenticated → 401
        let req = Request::builder()
            .method("POST")
            .uri("/relay/agents")
            .body(Body::empty())
            .expect("build request");
        let resp = app.clone().oneshot(req).await.expect("oneshot");
        assert_eq!(
            resp.status(),
            StatusCode::UNAUTHORIZED,
            "POST /relay/agents"
        );

        // GET /relay/agents/ws (WS upgrade path) — unauthenticated → 401
        let (status, body) = get_json(&app, "/relay/agents/ws").await;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "GET /relay/agents/ws");
        assert_eq!(body["code"], "unauthorized");
    }

    /// With `auth.enabled = false`, relay routes are accessible without a key
    /// (behavior unchanged from before the auth gating).
    #[tokio::test]
    async fn relay_requires_auth_skipped_when_disabled() {
        let mut config = RokoConfig::default();
        config.serve.auth = ServeAuthConfig {
            enabled: false,
            api_key: String::new(),
            api_keys: Vec::new(),
            privy_app_id: None,
            jwks_providers: Vec::new(),
            privy_workspace_id: None,
            privy_allowed_roles: Vec::new(),
            enforcement_mode: Default::default(),
            invite_expiry_days: 7,
        };

        let (_dir, app) = build_test_router(config);

        // Without a relay URL configured, we expect 503 (not 401),
        // proving the auth layer was skipped.
        let (status, body) = get_json(&app, "/relay/health").await;
        assert_eq!(
            status,
            StatusCode::SERVICE_UNAVAILABLE,
            "GET /relay/health without auth"
        );
        assert_eq!(body["code"], "agent_relay_not_configured");
    }

    #[tokio::test]
    async fn build_router_returns_structured_rbac_denial() {
        let mut config = RokoConfig::default();
        config.serve.auth.enabled = true;
        config.serve.auth.api_key = "admin-key".into();
        let (_dir, app) = build_test_router(config);

        let req = Request::builder()
            .method("POST")
            .uri("/api/team/invite")
            .header("X-Api-Key", "admin-key")
            .header("Content-Type", "application/json")
            .body(Body::from(r#"{"email":"invitee@example.com"}"#))
            .expect("build request");
        let response = app.oneshot(req).await.expect("oneshot");
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        let body = response
            .into_body()
            .collect()
            .await
            .expect("collect body")
            .to_bytes();
        let body: Value = serde_json::from_slice(&body).expect("JSON body");
        assert_eq!(body["error"], "forbidden");
        assert_eq!(body["permission"], "team:manage");
        assert_eq!(body["role"], "admin");
    }

    #[tokio::test]
    async fn build_router_allows_owner_team_mutation() {
        let plaintext = "owner-key-secret";
        let mut config = RokoConfig::default();
        config.serve.auth.enabled = true;
        config.serve.auth.api_keys = vec![roko_core::config::ApiKeyEntry {
            name: "owner-key".into(),
            key_hash: middleware::hash_api_key(plaintext),
            scope: "owner".into(),
            created_at: chrono::Utc::now().to_rfc3339(),
            expires_at: None,
            last_used_at: None,
            previous_key_hashes: Vec::new(),
        }];
        let (dir, app) = build_test_router(config);
        let team_dir = dir.path().join(".roko").join("team");
        std::fs::create_dir_all(&team_dir).expect("create team dir");
        std::fs::write(
            team_dir.join("members.json"),
            serde_json::to_vec_pretty(&serde_json::json!([{
                "id": "owner-key",
                "email": "owner@example.com",
                "role": "owner",
                "joined_at": chrono::Utc::now().to_rfc3339(),
            }]))
            .expect("serialize member"),
        )
        .expect("write members");

        let req = Request::builder()
            .method("POST")
            .uri("/api/team/invite")
            .header("X-Api-Key", plaintext)
            .header("Content-Type", "application/json")
            .body(Body::from(r#"{"email":"invitee@example.com"}"#))
            .expect("build request");
        let response = app.oneshot(req).await.expect("oneshot");
        assert_eq!(response.status(), StatusCode::CREATED);
        let body = response
            .into_body()
            .collect()
            .await
            .expect("collect body")
            .to_bytes();
        let body: Value = serde_json::from_slice(&body).expect("JSON body");
        assert_eq!(body["role"], "member");
        assert!(
            body["invite_token"]
                .as_str()
                .is_some_and(|token| !token.is_empty())
        );
    }

    #[tokio::test]
    async fn auth_audit_query_requires_secrets_read_permission() {
        let plaintext = "viewer-key-secret";
        let mut config = RokoConfig::default();
        config.serve.auth.enabled = true;
        config.serve.auth.api_keys = vec![roko_core::config::ApiKeyEntry {
            name: "viewer-key".into(),
            key_hash: middleware::hash_api_key(plaintext),
            scope: "read".into(),
            created_at: chrono::Utc::now().to_rfc3339(),
            expires_at: None,
            last_used_at: None,
            previous_key_hashes: Vec::new(),
        }];
        let (_dir, app) = build_test_router(config);
        let req = Request::builder()
            .uri("/api/auth/audit")
            .header("X-Api-Key", plaintext)
            .body(Body::empty())
            .expect("build request");
        let response = app.oneshot(req).await.expect("oneshot");
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        let body = response
            .into_body()
            .collect()
            .await
            .expect("collect body")
            .to_bytes();
        let body: Value = serde_json::from_slice(&body).expect("JSON body");
        assert_eq!(body["error"], "forbidden");
        assert_eq!(body["permission"], "secrets:read");
        assert_eq!(body["role"], "viewer");
    }

    /// Two callers with different API keys get independent rate-limit budgets.
    /// Exhausting one key's budget must not affect the other.
    #[tokio::test]
    async fn rate_limit_is_keyed_per_api_key() {
        // Budget of 2 req/s per key.
        let limiter = build_keyed_rate_limiter(2);
        let app = axum::Router::new()
            .route("/ping", axum::routing::get(|| async { "pong" }))
            .layer(axum::middleware::from_fn_with_state(
                limiter,
                keyed_rate_limit_middleware,
            ));

        // Key A: exhaust its budget (2 requests succeed, 3rd is throttled).
        for i in 0..2 {
            let req = Request::builder()
                .uri("/ping")
                .header("X-Api-Key", "key-alpha")
                .body(Body::empty())
                .unwrap_or_else(|_| panic!("build request A-{i}"));
            let resp = app.clone().oneshot(req).await.expect("oneshot A");
            assert_eq!(resp.status(), StatusCode::OK, "key-alpha request {i}");
        }

        let req = Request::builder()
            .uri("/ping")
            .header("X-Api-Key", "key-alpha")
            .body(Body::empty())
            .expect("build request A-overflow");
        let resp = app.clone().oneshot(req).await.expect("oneshot A-overflow");
        assert_eq!(
            resp.status(),
            StatusCode::TOO_MANY_REQUESTS,
            "key-alpha must be throttled after exhausting its budget"
        );

        // Key B: still has a full budget — requests must succeed.
        for i in 0..2 {
            let req = Request::builder()
                .uri("/ping")
                .header("X-Api-Key", "key-beta")
                .body(Body::empty())
                .unwrap_or_else(|_| panic!("build request B-{i}"));
            let resp = app.clone().oneshot(req).await.expect("oneshot B");
            assert_eq!(
                resp.status(),
                StatusCode::OK,
                "key-beta request {i} must not be affected by key-alpha's exhaustion"
            );
        }

        // Verify the response body carries the correct error code.
        let req = Request::builder()
            .uri("/ping")
            .header("X-Api-Key", "key-beta")
            .body(Body::empty())
            .expect("build request B-overflow");
        let resp = app.oneshot(req).await.expect("oneshot B-overflow");
        assert_eq!(resp.status(), StatusCode::TOO_MANY_REQUESTS);
        let body = resp
            .into_body()
            .collect()
            .await
            .expect("collect body")
            .to_bytes();
        let json: Value = serde_json::from_slice(&body).expect("json body");
        assert_eq!(json["code"], "rate_limited");
    }

    // --- E04-T19: route-to-scope manifest ------------------------------------

    /// Every mutating route registered in `build_router` must be explicitly
    /// classified by [`middleware::ROUTE_SCOPE_MANIFEST`].
    #[test]
    fn route_scope_manifest_matches_router() {
        use axum::http::Method;
        use middleware::{ROUTE_SCOPE_MANIFEST, required_scope_for};

        let router_routes: &[(&str, &str)] = &[
            // admin
            ("/api/api-keys", "admin"),
            ("/api/api-keys/test-key", "admin"),
            ("/api/secrets/ns/key", "admin"),
            ("/api/secrets/ns/key/test", "admin"),
            ("/api/config", "admin"),
            ("/api/config/reload", "admin"),
            // agent:write
            ("/api/agents/register", "agent:write"),
            ("/api/agents/create", "agent:write"),
            ("/api/agents/123/stop", "agent:write"),
            ("/api/agents/123/message", "agent:write"),
            ("/api/agents/123/start", "agent:write"),
            ("/api/agents/123/restart", "agent:write"),
            ("/api/agents/123/token", "agent:write"),
            ("/api/events/ingest", "agent:write"),
            ("/api/events/ingest/batch", "agent:write"),
            ("/relay/agents", "agent:write"),
            ("/relay/agents/123", "agent:write"),
            // plan:write
            ("/api/plans", "plan:write"),
            ("/api/plans/generate", "plan:write"),
            ("/api/plans/123/execute", "plan:write"),
            ("/api/plans/123/pause", "plan:write"),
            ("/api/plans/123/resume", "plan:write"),
            ("/api/plans/123/chat", "plan:write"),
            ("/api/plans/123/estimate", "plan:write"),
            ("/api/plans/123/tasks/t1/review", "plan:write"),
            ("/api/prds/ideas", "plan:write"),
            ("/api/prd/consolidate", "plan:write"),
            ("/api/prds/consolidate", "plan:write"),
            ("/api/prds/my-slug/draft", "plan:write"),
            ("/api/prds/my-slug/promote", "plan:write"),
            ("/api/prds/my-slug/plan", "plan:write"),
            // terminal:write
            ("/api/terminal/sessions", "terminal:write"),
            ("/ws/terminal/abc-123", "terminal:write"),
            // write (explicit in manifest)
            ("/api/workspaces", "write"),
            ("/api/workspaces/abc", "write"),
            ("/api/jobs", "write"),
            ("/api/jobs/match", "write"),
            ("/api/jobs/123/assign", "write"),
            ("/api/jobs/123/execute", "write"),
            ("/api/jobs/123/cancel", "write"),
            ("/api/run", "write"),
            ("/api/runs/123/share", "write"),
            ("/api/dream/run", "write"),
            ("/api/deployments", "write"),
            ("/api/deployments/123/task", "write"),
            ("/api/deployments/123/callback", "write"),
            ("/api/research/topic", "write"),
            ("/api/research/enhance-prd/my-slug", "write"),
            ("/api/research/analyze", "write"),
            ("/api/subscriptions", "write"),
            ("/api/subscriptions/123/enable", "write"),
            ("/api/subscriptions/123/disable", "write"),
            ("/api/templates", "write"),
            ("/api/templates/my-tmpl/deploy", "write"),
            ("/api/heartbeats", "write"),
            ("/api/neuro/query", "write"),
            ("/api/inference/complete", "write"),
            ("/api/inference/batch/submit", "write"),
            ("/api/bench/run", "write"),
            ("/api/bench/runs", "write"),
            ("/api/bench/runs/123/cancel", "write"),
            ("/api/bench/suites", "write"),
            ("/api/bench/swe/run", "write"),
            ("/api/connectors", "write"),
            ("/api/feeds", "write"),
            ("/api/feeds/123", "write"),
            ("/api/groups", "write"),
            ("/api/groups/grp-123/invite", "write"),
            ("/api/groups/grp-123/members/agent-1", "write"),
            ("/api/groups/grp-123/knowledge", "write"),
            ("/api/groups/grp-123/pheromones", "write"),
            ("/api/groups/grp-123/message", "write"),
            ("/api/invitations/inv-123/accept", "write"),
            ("/api/invitations/inv-123/reject", "write"),
            ("/api/rpc", "write"),
            ("/api/vision-loop", "write"),
            ("/api/vision-loop/run123/cancel", "write"),
            ("/api/team/join", "read"),
            ("/api/team/invite", "write"),
            ("/api/team/members/did:test", "write"),
            ("/api/webhooks/generic", "write"),
            ("/api/providers/openai/test", "write"),
        ];

        for (path, expected_scope) in router_routes {
            let got = required_scope_for(&Method::POST, path);
            assert_eq!(
                got, *expected_scope,
                "POST {path}: expected scope '{expected_scope}', got '{got}'"
            );
        }

        // Read-only methods must always return "read" regardless of path.
        for method in [Method::GET, Method::HEAD, Method::OPTIONS] {
            for (path, _) in router_routes {
                assert_eq!(
                    required_scope_for(&method, path),
                    "read",
                    "{method} {path} must be 'read'"
                );
            }
        }

        // No duplicate prefixes in the manifest (structural guard).
        let prefixes: Vec<&str> = ROUTE_SCOPE_MANIFEST.iter().map(|e| e.prefix).collect();
        for (i, p) in prefixes.iter().enumerate() {
            assert!(
                !prefixes[i + 1..].contains(p),
                "duplicate manifest prefix: {p}"
            );
        }
    }
}
