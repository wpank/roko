//! Route definitions for the roko HTTP API.
//!
//! Each submodule defines handlers for a related group of endpoints. The
//! [`build_router`] function assembles them into a single [`axum::Router`]
//! with CORS and tracing middleware.

mod agents;
mod aggregator;
mod auth;
mod bench;
mod chain;
pub(crate) mod config;
mod connectors;
mod deployments;
mod diagnosis;
mod dream;
mod event_ingest;
pub(crate) mod feeds;
mod gateway;
mod heartbeats;
mod integrations;
mod isfr;
mod jobs;
mod learning;
mod metrics;
mod middleware;
mod neuro;
mod plans;
pub(crate) mod prds;
mod projections;
mod providers;
mod research;
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
use crate::error::ApiError;
use axum::body::Body;
use axum::extract::{DefaultBodyLimit, Request, State};
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::Response;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::routing::get;
use axum::{Json, Router};
use futures::stream::{self, Stream};
use governor::clock::DefaultClock;
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
/// Returns 429 with a stable `code = "rate_limited"` body so clients can
/// distinguish throttling from auth/validation errors.
pub(crate) async fn rate_limit_middleware(
    State(limiter): State<Arc<GlobalRateLimiter>>,
    req: Request<Body>,
    next: Next,
) -> Result<Response, ApiError> {
    if limiter.check().is_err() {
        return Err(ApiError {
            status: StatusCode::TOO_MANY_REQUESTS,
            code: "rate_limited".into(),
            message: format!(
                "global rate limit exceeded ({DEFAULT_GLOBAL_RATE_PER_SEC} requests/sec)"
            ),
            details: None,
        });
    }
    Ok(next.run(req).await)
}

/// Middleware: per-caller rate limit keyed by API-key hash or client IP.
///
/// Runs before the global backstop so a single noisy caller is throttled
/// before the shared budget is affected. Returns 429 with
/// `code = "rate_limited"` and a message that does **not** expose the raw key.
pub(crate) async fn keyed_rate_limit_middleware(
    State(limiter): State<Arc<KeyedRateLimiter>>,
    req: Request<Body>,
    next: Next,
) -> Result<Response, ApiError> {
    let key = rate_limit_key(&req);
    if limiter.check_key(&key).is_err() {
        return Err(ApiError {
            status: StatusCode::TOO_MANY_REQUESTS,
            code: "rate_limited".into(),
            message: format!(
                "per-caller rate limit exceeded ({DEFAULT_PER_KEY_RATE_PER_SEC} requests/sec)"
            ),
            details: None,
        });
    }
    Ok(next.run(req).await)
}

pub use self::config::reload_config_from_disk;
pub use self::deployments::load_persisted_deployments;
pub(crate) use self::middleware::cors_layer;
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
    let cors = middleware::cors_layer(cors_origins, roko_config.server.unsafe_public_cors);
    let terminal_enabled = roko_config.serve.terminal_enabled;

    let api = Router::new()
        .merge(crate::openapi::routes())
        .merge(status::routes())
        .merge(jobs::routes())
        .merge(heartbeats::routes())
        .merge(plans::routes())
        .merge(prds::routes())
        .merge(run::routes())
        .merge(runs::routes())
        .merge(research::routes())
        .merge(subscriptions::routes())
        .merge(templates::routes())
        .merge(aggregator::routes())
        .merge(agents::routes())
        .merge(learning::routes())
        .merge(config::routes())
        .merge(deployments::routes())
        .merge(diagnosis::routes())
        .merge(integrations::routes())
        .merge(projections::routes())
        .merge(neuro::routes())
        .merge(dream::routes())
        .merge(event_ingest::routes())
        .merge(gateway::routes())
        .merge(chain::routes())
        .merge(connectors::routes())
        .merge(feeds::routes())
        .merge(isfr::routes())
        .merge(auth::routes())
        .merge(secrets::routes())
        .merge(vision_loop::routes())
        .merge(team::routes())
        .merge(bench::routes())
        .merge(swe_bench::routes())
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
        api.layer(axum::middleware::from_fn(middleware::require_scope))
            .layer(axum::middleware::from_fn_with_state(
                Arc::clone(&state),
                middleware::require_api_key,
            ))
    } else {
        api
    };

    // Secret-scrubbing layer: redacts API keys / tokens from JSON responses.
    let scrubber = Arc::clone(&state.scrubber);
    let api = api.layer(axum::middleware::from_fn_with_state(
        scrubber,
        middleware::scrub_secrets,
    ));

    // Terminal routes always require auth + scope when enabled, even on loopback.
    let terminal = if terminal_enabled {
        crate::terminal::routes()
            .layer(axum::middleware::from_fn(middleware::require_scope))
            .layer(axum::middleware::from_fn_with_state(
                Arc::clone(&state),
                middleware::require_api_key,
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
            .layer(axum::middleware::from_fn(middleware::require_scope))
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
        // Public share-receipt reader: no auth required so recipients can
        // open share links without a roko API key.
        .merge(shared_runs::public_routes())
        // PTY terminal sessions for web UI — gated by config and bind policy.
        .merge(terminal)
        .nest("/api", api)
        .merge(ws)
        .merge(relay)
        // SPA fallback — serves embedded React app for all unmatched routes
        .fallback(crate::embedded::serve_embedded);

    let rate_limiter = build_global_rate_limiter(DEFAULT_GLOBAL_RATE_PER_SEC);
    let keyed_limiter = build_keyed_rate_limiter(DEFAULT_PER_KEY_RATE_PER_SEC);

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
    use axum::http::{Request, StatusCode};
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
            privy_workspace_id: None,
            privy_allowed_roles: Vec::new(),
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
            privy_workspace_id: None,
            privy_allowed_roles: Vec::new(),
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
            privy_workspace_id: None,
            privy_allowed_roles: Vec::new(),
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
            privy_workspace_id: None,
            privy_allowed_roles: Vec::new(),
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
            ("/api/rpc", "write"),
            ("/api/vision-loop", "write"),
            ("/api/vision-loop/run123/cancel", "write"),
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
