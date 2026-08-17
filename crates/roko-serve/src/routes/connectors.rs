//! Authenticated connector transport lifecycle routes.
//!
//! - `GET    /api/connectors` — list canonical secret-free descriptors
//! - `POST   /api/connectors` — register and supervise an HTTP JSON transport
//! - `DELETE /api/connectors/{name}` — disconnect and unregister
//! - `GET    /api/connectors/{name}/health` — perform a real health check
//! - `POST   /api/connectors/{name}/restart` — reset bounded reconnect supervision
//! - `POST   /api/connectors/{name}/query` — perform a GET-style query
//! - `POST   /api/connectors/{name}/execute` — perform a POST-style execution

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

use axum::body::Body;
use axum::extract::{ConnectInfo as PeerConnectInfo, Path, State};
use axum::http::{Request, StatusCode};
use axum::middleware::Next;
use axum::response::Response;
use axum::routing::{get, post};
use axum::{Json, Router};
use roko_core::connector::{
    ConnectConfig, ConnectorHealth, ConnectorInfo, ConnectorKind, ConnectorManifest,
    ExecuteRequest, ExecuteResponse, QueryRequest, QueryResponse, ReconnectStrategy,
};
use roko_runtime::{
    ConnectorRuntimeStatus, ConnectorSupervisionStatus, ConnectorSupervisorOptions,
};
use serde::{Deserialize, Serialize};

use crate::error::ApiError;
use crate::routes::middleware::AuthContext;
use crate::state::AppState;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/connectors", get(list_connectors).post(create_connector))
        .route(
            "/connectors/{name}",
            axum::routing::delete(delete_connector),
        )
        .route("/connectors/{name}/health", get(connector_health))
        .route("/connectors/{name}/restart", post(restart_connector))
        .route("/connectors/{name}/query", post(query_connector))
        .route("/connectors/{name}/execute", post(execute_connector))
        .route_layer(axum::middleware::from_fn(require_connector_access))
}

#[derive(Deserialize)]
struct CreateConnectorRequest {
    name: String,
    kind: ConnectorKind,
    endpoint: String,
    #[serde(default)]
    auth: Option<String>,
    #[serde(default)]
    headers: Option<HashMap<String, String>>,
    #[serde(default = "default_timeout_ms")]
    timeout_ms: u64,
    #[serde(default)]
    description: String,
    #[serde(default)]
    capabilities: Vec<String>,
    #[serde(default = "default_health_interval_secs")]
    health_interval_secs: u64,
    #[serde(default = "default_reconnect_strategy")]
    reconnect_strategy: ReconnectStrategy,
    #[serde(default = "default_reconnect_attempts")]
    max_reconnect_attempts: u32,
}

const fn default_timeout_ms() -> u64 {
    5_000
}

const fn default_health_interval_secs() -> u64 {
    30
}

const fn default_reconnect_attempts() -> u32 {
    3
}

fn default_reconnect_strategy() -> ReconnectStrategy {
    ReconnectStrategy::ExponentialBackoff {
        base_ms: 250,
        max_ms: 30_000,
        jitter: true,
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct ConnectorListResponse {
    connectors: Vec<ConnectorInfo>,
    total: usize,
    healthy: usize,
}

#[derive(Debug, Serialize, Deserialize)]
struct DeleteConnectorResponse {
    name: String,
    deleted: bool,
}

#[derive(Debug, Serialize)]
struct ConnectorHealthResponse {
    health: ConnectorHealth,
    #[serde(skip_serializing_if = "Option::is_none")]
    supervisor: Option<ConnectorSupervisionStatus>,
}

/// `GET /api/connectors` — list canonical descriptors without transport config.
async fn list_connectors(
    State(state): State<Arc<AppState>>,
) -> Result<Json<ConnectorListResponse>, ApiError> {
    let registry = state.connectors.read().await;
    let connectors = registry.list().to_vec();
    let total = connectors.len();
    let healthy = registry.healthy_count();
    Ok(Json(ConnectorListResponse {
        connectors,
        total,
        healthy,
    }))
}

/// `POST /api/connectors` — register a supervised concrete HTTP transport.
async fn create_connector(
    State(state): State<Arc<AppState>>,
    Json(request): Json<CreateConnectorRequest>,
) -> Result<(StatusCode, Json<ConnectorRuntimeStatus>), ApiError> {
    let manifest = ConnectorManifest {
        name: request.name,
        kind: request.kind,
        version: "1".to_owned(),
        description: request.description,
        config_schema: None,
        capabilities: request.capabilities,
        health_interval_secs: request.health_interval_secs,
        reconnect_strategy: request.reconnect_strategy,
    };
    let config = ConnectConfig {
        endpoint: request.endpoint,
        auth: request.auth,
        headers: request.headers,
        timeout_ms: request.timeout_ms,
    };
    let status = state
        .connector_runtime
        .register_http(
            manifest,
            config,
            ConnectorSupervisorOptions {
                max_reconnect_attempts: request.max_reconnect_attempts,
            },
        )
        .await
        .map_err(connector_error)?;
    Ok((StatusCode::CREATED, Json(status)))
}

/// `DELETE /api/connectors/{name}` — gracefully disconnect and unregister.
async fn delete_connector(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> Result<Json<DeleteConnectorResponse>, ApiError> {
    let mut deleted = state
        .connector_runtime
        .unregister(&name)
        .await
        .map_err(connector_error)?;
    if !deleted {
        // Descriptor-only built-ins still use the canonical registry and can
        // be removed even though they have no concrete transport lifecycle.
        deleted = state.connectors.write().await.unregister(&name);
    }
    if !deleted {
        return Err(ApiError::not_found(format!("connector '{name}' not found")));
    }
    Ok(Json(DeleteConnectorResponse { name, deleted }))
}

/// `GET /api/connectors/{name}/health` — run transport health when managed.
async fn connector_health(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> Result<Json<ConnectorHealthResponse>, ApiError> {
    if state.connector_runtime.status(&name).await.is_ok() {
        let status = state
            .connector_runtime
            .refresh_health(&name)
            .await
            .map_err(connector_error)?;
        return Ok(Json(ConnectorHealthResponse {
            health: status.connector.health,
            supervisor: Some(status.supervisor),
        }));
    }
    let health = state
        .connectors
        .read()
        .await
        .get(&name)
        .map(|info| info.health.clone())
        .ok_or_else(|| ApiError::not_found(format!("connector '{name}' not found")))?;
    Ok(Json(ConnectorHealthResponse {
        health,
        supervisor: None,
    }))
}

/// `POST /api/connectors/{name}/restart` — reset bounds and reconnect.
async fn restart_connector(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> Result<Json<ConnectorRuntimeStatus>, ApiError> {
    ensure_managed(&state, &name).await?;
    state
        .connector_runtime
        .restart(&name)
        .await
        .map(Json)
        .map_err(connector_error)
}

/// `POST /api/connectors/{name}/query` — issue an idempotent GET-style request.
async fn query_connector(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
    Json(request): Json<QueryRequest>,
) -> Result<Json<QueryResponse>, ApiError> {
    ensure_managed(&state, &name).await?;
    state
        .connector_runtime
        .query(&name, request)
        .await
        .map(Json)
        .map_err(connector_error)
}

/// `POST /api/connectors/{name}/execute` — issue a mutating POST-style request.
async fn execute_connector(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
    Json(request): Json<ExecuteRequest>,
) -> Result<Json<ExecuteResponse>, ApiError> {
    ensure_managed(&state, &name).await?;
    state
        .connector_runtime
        .execute(&name, request)
        .await
        .map(Json)
        .map_err(connector_error)
}

async fn ensure_managed(state: &AppState, name: &str) -> Result<(), ApiError> {
    state
        .connector_runtime
        .status(name)
        .await
        .map(|_| ())
        .map_err(|_| ApiError::not_found(format!("managed connector '{name}' not found")))
}

async fn require_connector_access(
    request: Request<Body>,
    next: Next,
) -> Result<Response, ApiError> {
    let authenticated = request.extensions().get::<AuthContext>().is_some();
    let loopback = request
        .extensions()
        .get::<PeerConnectInfo<SocketAddr>>()
        .is_some_and(|peer| peer.0.ip().is_loopback());
    if !authenticated && !loopback {
        return Err(ApiError::forbidden(
            "connector lifecycle requires authentication or a loopback connection",
        ));
    }
    Ok(next.run(request).await)
}

fn connector_error(error: roko_core::RokoError) -> ApiError {
    match error {
        roko_core::RokoError::Invalid(message) | roko_core::RokoError::User(message) => {
            ApiError::bad_request(message)
        }
        roko_core::RokoError::PermissionDenied(_) => {
            ApiError::forbidden("connector transport authorization failed")
        }
        roko_core::RokoError::Transport(_)
        | roko_core::RokoError::Timeout { .. }
        | roko_core::RokoError::RateLimited(_) => ApiError {
            status: StatusCode::BAD_GATEWAY,
            code: "connector_transport_error".to_owned(),
            message: "connector transport operation failed".to_owned(),
            details: None,
        },
        _ => ApiError::internal("connector lifecycle operation failed"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use axum::body::Body;
    use axum::http::Request;
    use axum::middleware::from_fn;
    use tower::ServiceExt;

    use crate::routes::middleware::AuthMethod;

    fn guarded_app() -> Router {
        Router::new()
            .route("/", get(|| async { StatusCode::NO_CONTENT }))
            .route_layer(from_fn(require_connector_access))
    }

    fn request() -> Request<Body> {
        Request::builder()
            .uri("/")
            .body(Body::empty())
            .expect("guard request")
    }

    #[tokio::test]
    async fn connector_routes_fail_closed_without_authentication_or_peer_identity() {
        let response = guarded_app()
            .oneshot(request())
            .await
            .expect("guard response");
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn connector_routes_reject_non_loopback_unauthenticated_peers() {
        let mut request = request();
        request.extensions_mut().insert(PeerConnectInfo(
            "198.51.100.8:49152"
                .parse::<SocketAddr>()
                .expect("remote peer"),
        ));
        let response = guarded_app()
            .oneshot(request)
            .await
            .expect("guard response");
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn connector_routes_allow_loopback_when_global_auth_is_disabled() {
        let mut request = request();
        request.extensions_mut().insert(PeerConnectInfo(
            "127.0.0.1:49152"
                .parse::<SocketAddr>()
                .expect("loopback peer"),
        ));
        let response = guarded_app()
            .oneshot(request)
            .await
            .expect("guard response");
        assert_eq!(response.status(), StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn connector_routes_allow_a_globally_authenticated_request() {
        let mut request = request();
        request.extensions_mut().insert(AuthContext {
            method: AuthMethod::ApiKey,
            scope: "admin".to_owned(),
            user_id: Some("connector-test".to_owned()),
        });
        let response = guarded_app()
            .oneshot(request)
            .await
            .expect("guard response");
        assert_eq!(response.status(), StatusCode::NO_CONTENT);
    }
}
