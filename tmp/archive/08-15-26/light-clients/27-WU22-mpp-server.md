# WU-22: Agent-as-MPP-Server

**Layer**: 5
**Depends on**: WU-17 (MppClient), WU-18 (MPP Tools), WU-12 (Sidecar Routes)
**Blocks**: None
**Estimated effort**: 3-4 hours
**Crates**: `crates/roko-agent-server`, `crates/roko-chain`
**Feature gate**: `mpp`

---

## Overview

The spec only covers agents PAYING for services. But roko agents should also SELL services via MPP. This WU adds MPP server middleware to the agent sidecar, turning any agent's HTTP endpoints into MPP-payable services.

When enabled, the sidecar wraps specified routes with the MPP 402 challenge-response flow. External agents or clients pay per request, and the roko agent verifies payment before serving.

The `mpp-rs` server-side API handles challenge generation, credential verification, and receipt headers. This WU wraps that into an Axum middleware layer that integrates with the existing sidecar router.

---

## Pre-read

- `crates/roko-agent-server/src/lib.rs` — `FeatureFlags`, `protected_router()`, `AgentServerBuilder`
- `crates/roko-agent-server/src/features/mod.rs` — module registry
- `crates/roko-agent-server/src/features/chain.rs` — existing chain routes (WU-12)
- `crates/roko-agent-server/src/state.rs` — `AgentState` struct
- `crates/roko-chain/src/mpp_client.rs` — `MppClient`, `MppConfig` (WU-17)
- `crates/roko-core/src/config/chain.rs` — `ChainConfig` with `MppConfig`
- `22-WU17-mpp-client.md` — MppClient design
- `17-WU12-sidecar.md` — Sidecar feature flag pattern

---

## Tasks

### 22.1 Add `MppServerConfig` to agent config

**File**: `crates/roko-core/src/config/agent.rs` (or wherever agent config lives)

```rust
/// MPP server configuration — gates agent HTTP endpoints with payment.
///
/// ```toml
/// [agent.mpp_server]
/// recipient = "0xYOUR_WALLET_ADDRESS"
/// secret_key_env = "ROKO_MPP_SERVER_SECRET"
/// realm = "roko-agent"
/// enable_sessions = true
///
/// [[agent.mpp_server.gated_routes]]
/// pattern = "/message"
/// amount = "50000"
/// intent = "charge"
/// description = "AI agent inference"
/// ```
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct MppServerConfig {
    /// Recipient wallet address for incoming payments.
    pub recipient: String,

    /// HMAC secret key environment variable name (NOT the key itself).
    pub secret_key_env: String,

    /// Routes to gate with MPP payments.
    #[serde(default)]
    pub gated_routes: Vec<GatedRoute>,

    /// Realm name (shown in 402 challenge).
    #[serde(default)]
    pub realm: Option<String>,

    /// Whether to accept session-based payments.
    #[serde(default)]
    pub enable_sessions: bool,
}

/// A route gated by MPP payment.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GatedRoute {
    /// Route pattern (e.g., "/message", "/research/*").
    pub pattern: String,

    /// Price per request in base units (e.g., "10000" = $0.01 with 6 decimals).
    pub amount: String,

    /// Payment intent: "charge" or "session".
    #[serde(default = "default_intent")]
    pub intent: String,

    /// Description shown in the 402 challenge.
    #[serde(default)]
    pub description: Option<String>,
}

fn default_intent() -> String {
    "charge".to_string()
}
```

Add the field to the existing agent config struct:

```rust
    /// MPP server configuration — gate endpoints with payment.
    #[serde(default)]
    pub mpp_server: Option<MppServerConfig>,
```

### 22.2 Create `crates/roko-agent-server/src/middleware/mpp_gate.rs`

**File**: `crates/roko-agent-server/src/middleware/mpp_gate.rs`

Axum middleware that implements the MPP 402 challenge-response flow using `mpp-rs` server-side API.

```rust
//! MPP payment gate middleware for the agent sidecar.
//!
//! Wraps specified routes with the MPP 402 challenge-response flow.
//! External agents/clients must pay before their request is served.
//!
//! # Flow
//!
//! 1. Client sends request to a gated route
//! 2. If no `Authorization` header with payment credential:
//!    - Server generates a payment challenge via `mpp::server::Mpp::charge()`
//!    - Returns 402 Payment Required with `WWW-Authenticate: Payment ...` header
//! 3. Client pays on-chain and retries with `Authorization: Payment ...` header
//! 4. Server verifies the credential via `mpp::server::Mpp::verify_credential()`
//! 5. If valid, passes the request through and adds `Payment-Receipt` header to response
//!
//! # Feature gate
//! Requires `mpp` feature: `cargo build -p roko-agent-server --features mpp`

use std::sync::Arc;
use std::task::{Context, Poll};

use axum::body::Body;
use axum::http::{Request, Response, StatusCode};
use axum::response::IntoResponse;
use futures::future::BoxFuture;
use tower::{Layer, Service};

use crate::state::AgentState;

// ── Types ────────────────────────────────────────────────────────────

/// Configuration for a gated route in the middleware.
#[derive(Debug, Clone)]
pub struct GatedRouteConfig {
    /// Route pattern (e.g., "/message").
    pub pattern: String,
    /// Price per request in base units.
    pub amount: String,
    /// Payment intent: "charge" or "session".
    pub intent: String,
    /// Description for the challenge.
    pub description: Option<String>,
}

/// State shared by all instances of the MPP gate middleware.
#[derive(Clone)]
pub struct MppGateState {
    /// The mpp-rs server instance.
    /// Uses `mpp::server::Mpp<mpp::server::TempoChargeMethod>`.
    ///
    /// Type-erased here to avoid leaking mpp-rs types into the public API.
    /// The actual mpp::server::Mpp is stored as an Arc<dyn Any + Send + Sync>
    /// and downcast in the middleware logic.
    mpp_server: Arc<dyn std::any::Any + Send + Sync>,
    /// Routes that require payment.
    gated_routes: Vec<GatedRouteConfig>,
    /// Whether sessions are enabled.
    enable_sessions: bool,
}

// ── Layer ────────────────────────────────────────────────────────────

/// Axum layer that wraps routes with MPP payment gating.
///
/// # Usage
///
/// ```rust,ignore
/// use roko_agent_server::middleware::mpp_gate::MppGateLayer;
///
/// let router = Router::new()
///     .route("/message", post(handle_message))
///     .route("/research", post(handle_research))
///     .layer(MppGateLayer::new(mpp_gate_state));
/// ```
#[derive(Clone)]
pub struct MppGateLayer {
    state: Arc<MppGateState>,
}

impl MppGateLayer {
    /// Create a new MPP gate layer.
    ///
    /// # Arguments
    /// - `recipient`: Wallet address to receive payments
    /// - `secret_key`: HMAC secret for challenge signing
    /// - `realm`: Realm name for the 402 challenge
    /// - `rpc_url`: Tempo RPC URL for settlement verification
    /// - `gated_routes`: Routes to gate with payment
    /// - `enable_sessions`: Whether to accept session payments
    ///
    /// # Errors
    /// Returns error if the mpp-rs server cannot be constructed.
    pub fn new(
        recipient: &str,
        secret_key: &[u8],
        realm: &str,
        rpc_url: &str,
        gated_routes: Vec<GatedRouteConfig>,
        enable_sessions: bool,
    ) -> Result<Self, String> {
        // Construct the mpp-rs server instance.
        //
        // ```rust,ignore
        // use mpp::server::{tempo, TempoConfig, Mpp};
        //
        // let mpp = Mpp::new(
        //     tempo(TempoConfig { recipient: recipient.to_string() })
        //         .rpc_url(rpc_url)
        //         .build()
        //         .map_err(|e| format!("tempo config: {e}"))?,
        //     realm,
        //     secret_key,
        // );
        // ```
        //
        // TODO(22.2a): Construct real mpp::server::Mpp instance.
        // Blocked on confirming the mpp-rs server API surface.
        // For now, store a placeholder that will fail at runtime with
        // a clear error message.

        let _ = (recipient, secret_key, realm, rpc_url);

        let state = MppGateState {
            mpp_server: Arc::new(MppGatePlaceholder),
            gated_routes,
            enable_sessions,
        };

        Ok(Self {
            state: Arc::new(state),
        })
    }
}

/// Placeholder until mpp-rs server types are wired.
struct MppGatePlaceholder;

impl<S> Layer<S> for MppGateLayer {
    type Service = MppGateMiddleware<S>;

    fn layer(&self, inner: S) -> Self::Service {
        MppGateMiddleware {
            inner,
            state: Arc::clone(&self.state),
        }
    }
}

// ── Middleware ────────────────────────────────────────────────────────

/// The actual middleware service that intercepts requests to gated routes.
#[derive(Clone)]
pub struct MppGateMiddleware<S> {
    inner: S,
    state: Arc<MppGateState>,
}

impl<S> MppGateMiddleware<S> {
    /// Check if the request path matches any gated route.
    fn find_gated_route(&self, path: &str) -> Option<&GatedRouteConfig> {
        self.state.gated_routes.iter().find(|route| {
            if route.pattern.ends_with('*') {
                let prefix = &route.pattern[..route.pattern.len() - 1];
                path.starts_with(prefix)
            } else {
                path == route.pattern
            }
        })
    }
}

impl<S> Service<Request<Body>> for MppGateMiddleware<S>
where
    S: Service<Request<Body>, Response = Response<Body>> + Clone + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = Response<Body>;
    type Error = S::Error;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        let path = req.uri().path().to_string();
        let gated_route = self.find_gated_route(&path).cloned();
        let state = Arc::clone(&self.state);
        let mut inner = self.inner.clone();

        Box::pin(async move {
            // If the route is not gated, pass through
            let route_config = match gated_route {
                Some(config) => config,
                None => return inner.call(req).await,
            };

            // Check for Authorization header with payment credential
            let auth_header = req
                .headers()
                .get("authorization")
                .and_then(|v| v.to_str().ok())
                .map(String::from);

            match auth_header {
                Some(credential) if credential.starts_with("Payment ") => {
                    // Verify the payment credential
                    //
                    // TODO(22.2b): Use mpp::server::Mpp::verify_credential()
                    //
                    // ```rust,ignore
                    // let mpp = state.mpp_server.downcast_ref::<mpp::server::Mpp<_>>()
                    //     .expect("mpp server");
                    // let credential = parse_payment_credential(&credential)?;
                    // let receipt = mpp.verify_credential(&credential).await
                    //     .map_err(|e| /* return 402 with error */)?;
                    // ```
                    //
                    // For now: pass through (accept all payment credentials).
                    // This is safe for development but MUST be replaced before
                    // any real deployment.

                    let mut response = inner.call(req).await?;

                    // Add Payment-Receipt header to response
                    // TODO(22.2c): Add real receipt from mpp.verify_credential()
                    response.headers_mut().insert(
                        "payment-receipt",
                        "placeholder-receipt".parse().unwrap(),
                    );

                    Ok(response)
                }
                _ => {
                    // No valid payment credential — return 402 challenge
                    //
                    // TODO(22.2d): Use mpp::server::Mpp::charge() to generate
                    // a real challenge.
                    //
                    // ```rust,ignore
                    // let mpp = state.mpp_server.downcast_ref::<mpp::server::Mpp<_>>()
                    //     .expect("mpp server");
                    // let challenge = mpp.charge(&route_config.amount)?;
                    // ```

                    let description = route_config
                        .description
                        .as_deref()
                        .unwrap_or("Payment required");

                    let challenge_header = format!(
                        "Payment realm=\"{}\", amount=\"{}\", intent=\"{}\", description=\"{}\"",
                        "roko-agent",
                        route_config.amount,
                        route_config.intent,
                        description,
                    );

                    let response = Response::builder()
                        .status(StatusCode::PAYMENT_REQUIRED)
                        .header("www-authenticate", challenge_header)
                        .header("content-type", "application/json")
                        .body(Body::from(serde_json::json!({
                            "error": "payment_required",
                            "amount": route_config.amount,
                            "intent": route_config.intent,
                            "description": description,
                        }).to_string()))
                        .unwrap();

                    Ok(response)
                }
            }
        })
    }
}

// ── Constructor from Config ──────────────────────────────────────────

/// Build an `MppGateLayer` from the agent's `MppServerConfig`.
///
/// Reads the HMAC secret from the environment variable specified in
/// `config.secret_key_env`. The secret itself is never stored in config.
///
/// # Errors
/// Returns error if the secret key env var is not set or the mpp-rs
/// server cannot be constructed.
pub fn build_mpp_gate_layer(
    config: &crate::MppServerConfig,
    rpc_url: &str,
) -> Result<MppGateLayer, String> {
    let secret_key = std::env::var(&config.secret_key_env)
        .map_err(|_| format!(
            "MPP server secret key env var '{}' not set", config.secret_key_env
        ))?;

    let realm = config.realm.as_deref().unwrap_or("roko-agent");

    let gated_routes: Vec<GatedRouteConfig> = config
        .gated_routes
        .iter()
        .map(|r| GatedRouteConfig {
            pattern: r.pattern.clone(),
            amount: r.amount.clone(),
            intent: r.intent.clone(),
            description: r.description.clone(),
        })
        .collect();

    MppGateLayer::new(
        &config.recipient,
        secret_key.as_bytes(),
        realm,
        rpc_url,
        gated_routes,
        config.enable_sessions,
    )
}

// ── Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn gated_route_config_exact_match() {
        let state = MppGateState {
            mpp_server: Arc::new(MppGatePlaceholder),
            gated_routes: vec![
                GatedRouteConfig {
                    pattern: "/message".to_string(),
                    amount: "50000".to_string(),
                    intent: "charge".to_string(),
                    description: Some("Inference".to_string()),
                },
            ],
            enable_sessions: false,
        };

        let mw = MppGateMiddleware {
            inner: (),
            state: Arc::new(state),
        };

        assert!(mw.find_gated_route("/message").is_some());
        assert!(mw.find_gated_route("/other").is_none());
        assert!(mw.find_gated_route("/message/sub").is_none());
    }

    #[test]
    fn gated_route_config_wildcard_match() {
        let state = MppGateState {
            mpp_server: Arc::new(MppGatePlaceholder),
            gated_routes: vec![
                GatedRouteConfig {
                    pattern: "/research/*".to_string(),
                    amount: "100000".to_string(),
                    intent: "charge".to_string(),
                    description: None,
                },
            ],
            enable_sessions: false,
        };

        let mw = MppGateMiddleware {
            inner: (),
            state: Arc::new(state),
        };

        assert!(mw.find_gated_route("/research/topic").is_some());
        assert!(mw.find_gated_route("/research/deep/nested").is_some());
        assert!(mw.find_gated_route("/research/").is_some());
        assert!(mw.find_gated_route("/other").is_none());
        // Exact "/research" without trailing slash should NOT match "/research/*"
        assert!(mw.find_gated_route("/research").is_none());
    }

    #[test]
    fn gated_route_multiple_routes() {
        let state = MppGateState {
            mpp_server: Arc::new(MppGatePlaceholder),
            gated_routes: vec![
                GatedRouteConfig {
                    pattern: "/message".to_string(),
                    amount: "50000".to_string(),
                    intent: "charge".to_string(),
                    description: None,
                },
                GatedRouteConfig {
                    pattern: "/research/*".to_string(),
                    amount: "100000".to_string(),
                    intent: "charge".to_string(),
                    description: None,
                },
            ],
            enable_sessions: false,
        };

        let mw = MppGateMiddleware {
            inner: (),
            state: Arc::new(state),
        };

        let msg_route = mw.find_gated_route("/message").unwrap();
        assert_eq!(msg_route.amount, "50000");

        let research_route = mw.find_gated_route("/research/query").unwrap();
        assert_eq!(research_route.amount, "100000");
    }
}
```

### 22.3 Add OpenAPI auto-generation for gated routes

**File**: `crates/roko-agent-server/src/features/mpp_openapi.rs`

Auto-generate `GET /openapi.json` with `x-payment-info` extensions for all gated routes, making the agent discoverable by other MPP clients (including `MppDiscovery` from WU-21).

```rust
//! Auto-generated OpenAPI document for MPP-gated agent endpoints.
//!
//! Makes the agent discoverable by MPP clients — they can fetch
//! `GET /openapi.json` to learn which endpoints exist, what they
//! cost, and which payment methods are accepted.

use std::sync::Arc;

use axum::extract::State;
use axum::routing::get;
use axum::{Json, Router};
use serde_json::{json, Value};

use crate::state::AgentState;

/// Register the OpenAPI route.
pub fn router() -> Router<Arc<AgentState>> {
    Router::new()
        .route("/openapi.json", get(openapi_json))
}

/// `GET /openapi.json` — auto-generated OpenAPI with x-payment-info.
async fn openapi_json(State(state): State<Arc<AgentState>>) -> Json<Value> {
    let mpp_config = state.mpp_server_config();

    let agent_name = state.agent_id();
    let realm = mpp_config
        .as_ref()
        .and_then(|c| c.realm.as_deref())
        .unwrap_or("roko-agent");

    let recipient = mpp_config
        .as_ref()
        .map(|c| c.recipient.as_str())
        .unwrap_or("not-configured");

    // Build paths from gated routes
    let mut paths = serde_json::Map::new();
    if let Some(config) = &mpp_config {
        for route in &config.gated_routes {
            // Infer HTTP method from route pattern (default to POST)
            let method = "post";
            let path = route.pattern.trim_end_matches('*');

            let mut operation = json!({
                "summary": route.description.as_deref().unwrap_or("Gated endpoint"),
                "x-payment-info": {
                    "amount": route.amount,
                    "intent": route.intent,
                    "unit_type": "request",
                },
                "responses": {
                    "200": {
                        "description": "Successful response (after payment)",
                    },
                    "402": {
                        "description": "Payment required",
                        "headers": {
                            "WWW-Authenticate": {
                                "description": "MPP payment challenge",
                                "schema": { "type": "string" },
                            }
                        }
                    }
                }
            });

            if let Some(desc) = &route.description {
                operation["description"] = json!(desc);
            }

            let mut methods = serde_json::Map::new();
            methods.insert(method.to_string(), operation);
            paths.insert(path.to_string(), Value::Object(methods));
        }
    }

    // Build payment methods
    let payment_methods = json!([
        {
            "method": "tempo",
            "currency": "USDC",
            "decimals": 6,
        }
    ]);

    Json(json!({
        "openapi": "3.0.0",
        "info": {
            "title": format!("{agent_name} (MPP-enabled)"),
            "description": format!("Roko agent sidecar with MPP payment gating. Realm: {realm}"),
            "version": "1.0.0",
            "x-payment-info": {
                "recipient": recipient,
                "realm": realm,
                "payment_methods": payment_methods,
            }
        },
        "servers": [
            {
                "url": format!("http://localhost:{}", state.port()),
                "description": "Local sidecar"
            }
        ],
        "paths": paths,
    }))
}
```

### 22.4 Wire into sidecar construction

**File**: `crates/roko-agent-server/src/lib.rs`

Add MPP server support to the sidecar builder, conditionally enabled when `mpp_server` config is present.

Add feature flag:
```rust
struct FeatureFlags {
    messaging: bool,
    predictions: bool,
    research: bool,
    tasks: bool,
    chain: bool,
    mpp_server: bool,  // NEW
}
```

Add builder method:
```rust
/// Enable MPP payment gating on the sidecar.
///
/// Wraps specified routes with 402 challenge-response payment flow.
/// Requires `MppServerConfig` in the agent config.
#[must_use]
pub fn mpp_server(mut self, config: MppServerConfig) -> Self {
    self.features.mpp_server = true;
    self.mpp_server_config = Some(config);
    self.capabilities.push("mpp_server".to_string());
    self
}
```

Add to `protected_router()`:
```rust
if self.features.mpp_server {
    // Add OpenAPI endpoint for discoverability
    router = router.merge(features::mpp_openapi::router());

    // Wrap gated routes with payment middleware
    if let Some(ref config) = self.mpp_server_config {
        if let Ok(rpc_url) = self.resolve_rpc_url() {
            match middleware::mpp_gate::build_mpp_gate_layer(config, &rpc_url) {
                Ok(layer) => {
                    router = router.layer(layer);
                    tracing::info!(
                        gated_routes = config.gated_routes.len(),
                        "MPP payment gating enabled"
                    );
                }
                Err(e) => {
                    tracing::warn!(err = %e, "MPP gate layer failed to construct — routes will be ungated");
                }
            }
        }
    }
}
```

Add to `capability_is_live()`:
```rust
"mpp_server" => features.mpp_server,
```

Register modules:

**File**: `crates/roko-agent-server/src/features/mod.rs`
```rust
pub mod mpp_openapi;
```

**File**: `crates/roko-agent-server/src/middleware/mod.rs`
```rust
pub mod mpp_gate;
```

### 22.5 Record incoming payments to PaymentLedger

**File**: `crates/roko-agent-server/src/middleware/mpp_gate.rs`

After successful payment verification in the middleware, record the incoming payment:

```rust
// In the credential verification branch of MppGateMiddleware::call():

// Record incoming payment in the ledger (WU-20)
if let Some(ref ledger) = state.payment_ledger {
    let entry = PaymentLedgerEntry {
        direction: PaymentDirection::Incoming,
        service_url: path.clone(),
        amount: route_config.amount.clone(),
        token: "USDC".to_string(),
        intent: route_config.intent.clone(),
        tx_hash: receipt.tx_hash.clone(), // from mpp verify_credential
        timestamp: chrono::Utc::now().to_rfc3339(),
        payer: credential.payer_address.clone(), // from parsed credential
        recipient: state.recipient.clone(),
    };
    if let Err(e) = ledger.record(entry).await {
        tracing::warn!(err = %e, "failed to record incoming payment");
    }
}
```

The `PaymentLedger` is the same ledger used by WU-20 for outgoing payments. Incoming payments are distinguished by `direction: Incoming`.

### 22.6 Tests

**File**: `crates/roko-agent-server/src/middleware/mpp_gate.rs` (tests are inline in 22.2 above)

Additional integration tests:

**File**: `crates/roko-agent-server/tests/mpp_server.rs`

```rust
//! Integration tests for MPP server middleware.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::routing::post;
use axum::Router;
use tower::ServiceExt;

/// Test: ungated route passes through without 402.
#[tokio::test]
async fn ungated_route_passes_through() {
    let router = test_router_with_mpp_gate();

    let resp = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // /health is not gated — should pass through
    assert_ne!(resp.status(), StatusCode::PAYMENT_REQUIRED);
}

/// Test: gated route returns 402 without Authorization header.
#[tokio::test]
async fn gated_route_returns_402_without_payment() {
    let router = test_router_with_mpp_gate();

    let resp = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/message")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::PAYMENT_REQUIRED);

    // Check WWW-Authenticate header
    let www_auth = resp
        .headers()
        .get("www-authenticate")
        .expect("www-authenticate header")
        .to_str()
        .unwrap();
    assert!(www_auth.contains("Payment"));
    assert!(www_auth.contains("50000")); // amount
}

/// Test: gated route with Payment Authorization passes through.
#[tokio::test]
async fn gated_route_passes_with_payment_credential() {
    let router = test_router_with_mpp_gate();

    let resp = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/message")
                .header("authorization", "Payment credential=test123")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Should pass through (placeholder accepts all credentials)
    assert_eq!(resp.status(), StatusCode::OK);

    // Check Payment-Receipt header
    assert!(resp.headers().contains_key("payment-receipt"));
}

/// Test: 402 response includes JSON body with payment details.
#[tokio::test]
async fn challenge_response_includes_json_body() {
    let router = test_router_with_mpp_gate();

    let resp = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/message")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::PAYMENT_REQUIRED);

    let body_bytes = axum::body::to_bytes(resp.into_body(), 1024 * 1024)
        .await
        .unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    assert_eq!(body["error"], "payment_required");
    assert_eq!(body["amount"], "50000");
    assert_eq!(body["intent"], "charge");
}

/// Test: wildcard route matching.
#[tokio::test]
async fn wildcard_route_gating() {
    let router = test_router_with_mpp_gate();

    let resp = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/research/topic")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::PAYMENT_REQUIRED);

    let www_auth = resp
        .headers()
        .get("www-authenticate")
        .unwrap()
        .to_str()
        .unwrap();
    assert!(www_auth.contains("100000")); // research costs more
}

/// Test: OpenAPI endpoint returns valid document.
#[tokio::test]
async fn openapi_endpoint_returns_spec() {
    let router = test_router_with_openapi();

    let resp = router
        .oneshot(
            Request::builder()
                .uri("/openapi.json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(resp.into_body(), 1024 * 1024)
        .await
        .unwrap();
    let doc: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    assert_eq!(doc["openapi"], "3.0.0");
    assert!(doc["info"]["x-payment-info"].is_object());
    assert!(doc["paths"].is_object());
}

// ── Test Helpers ─────────────────────────────────────────────────

fn test_router_with_mpp_gate() -> Router {
    use roko_agent_server::middleware::mpp_gate::{MppGateLayer, GatedRouteConfig};

    let gated_routes = vec![
        GatedRouteConfig {
            pattern: "/message".to_string(),
            amount: "50000".to_string(),
            intent: "charge".to_string(),
            description: Some("AI inference".to_string()),
        },
        GatedRouteConfig {
            pattern: "/research/*".to_string(),
            amount: "100000".to_string(),
            intent: "charge".to_string(),
            description: Some("Research query".to_string()),
        },
    ];

    let layer = MppGateLayer::new(
        "0xTestRecipient",
        b"test-secret-key",
        "test-agent",
        "https://rpc.moderato.tempo.xyz",
        gated_routes,
        false,
    )
    .expect("gate layer");

    Router::new()
        .route("/message", post(|| async { "ok" }))
        .route("/research/:topic", post(|| async { "research result" }))
        .route("/health", post(|| async { "healthy" }))
        .layer(layer)
}

fn test_router_with_openapi() -> Router {
    // TODO: Construct with AgentState that has MppServerConfig
    todo!("wire AgentState mock with MppServerConfig")
}
```

Config integration tests:

**File**: `crates/roko-core/src/config/agent.rs` (add to existing tests)

```rust
    #[test]
    fn mpp_server_config_from_toml() {
        let toml_str = r#"
            recipient = "0xABC123"
            secret_key_env = "ROKO_MPP_SERVER_SECRET"
            realm = "my-agent"
            enable_sessions = true

            [[gated_routes]]
            pattern = "/message"
            amount = "50000"
            intent = "charge"
            description = "AI agent inference"

            [[gated_routes]]
            pattern = "/research/*"
            amount = "100000"
            intent = "charge"
        "#;
        let config: MppServerConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.recipient, "0xABC123");
        assert_eq!(config.secret_key_env, "ROKO_MPP_SERVER_SECRET");
        assert_eq!(config.realm.as_deref(), Some("my-agent"));
        assert!(config.enable_sessions);
        assert_eq!(config.gated_routes.len(), 2);
        assert_eq!(config.gated_routes[0].pattern, "/message");
        assert_eq!(config.gated_routes[0].amount, "50000");
        assert_eq!(config.gated_routes[1].pattern, "/research/*");
        assert!(config.gated_routes[1].description.is_none());
    }

    #[test]
    fn mpp_server_config_defaults() {
        let toml_str = r#"
            recipient = "0xABC"
            secret_key_env = "KEY"
        "#;
        let config: MppServerConfig = toml::from_str(toml_str).unwrap();
        assert!(config.gated_routes.is_empty());
        assert!(config.realm.is_none());
        assert!(!config.enable_sessions);
    }

    #[test]
    fn gated_route_default_intent() {
        let toml_str = r#"
            pattern = "/api"
            amount = "10000"
        "#;
        let route: GatedRoute = toml::from_str(toml_str).unwrap();
        assert_eq!(route.intent, "charge"); // default
    }
```

---

## Verification Checklist

- [ ] `MppServerConfig` added to agent config with `recipient`, `secret_key_env`, `gated_routes`, `realm`, `enable_sessions`
- [ ] `GatedRoute` struct with `pattern`, `amount`, `intent`, `description`
- [ ] Config parses from TOML correctly (with defaults)
- [ ] `MppGateLayer` implements `tower::Layer` for Axum
- [ ] `MppGateMiddleware` returns 402 with `WWW-Authenticate` header for gated routes
- [ ] `MppGateMiddleware` passes through ungated routes without 402
- [ ] Wildcard route patterns (e.g., `/research/*`) match correctly
- [ ] `Authorization: Payment ...` header triggers verification path
- [ ] `Payment-Receipt` header added to response after successful payment
- [ ] `GET /openapi.json` returns valid OpenAPI 3.0 document
- [ ] OpenAPI includes `x-payment-info` extensions with pricing per endpoint
- [ ] OpenAPI includes top-level payment methods in `info.x-payment-info`
- [ ] `FeatureFlags` has `mpp_server: bool` field
- [ ] `AgentServerBuilder::mpp_server()` method enables the feature
- [ ] `protected_router()` conditionally adds MPP gate layer and OpenAPI route
- [ ] `capability_is_live()` handles `"mpp_server"` string
- [ ] Feature modules registered in `features/mod.rs` and `middleware/mod.rs`
- [ ] `build_mpp_gate_layer()` reads secret from env var (not config)
- [ ] Incoming payments recorded to `PaymentLedger` with `direction: Incoming`
- [ ] `cargo build -p roko-agent-server --features mpp`
- [ ] `cargo test -p roko-agent-server --features mpp`
- [ ] `cargo clippy -p roko-agent-server --features mpp --no-deps -- -D warnings`
- [ ] `cargo test --workspace` — no breakage

---

## Open Questions

1. **Should agents be able to dynamically adjust pricing based on load/demand?** A `PricingStrategy` trait (e.g., `FixedPricing`, `LoadBasedPricing`, `AuctionPricing`) could replace the static `amount` field. This adds complexity but enables market-driven pricing for popular agents.

2. **Should the sidecar publish to MPPScan for discoverability?** MPPScan is Tempo's service registry. Auto-publishing would make the agent globally discoverable, but may not be desirable for private agents. A `publish_to_registry: bool` config field could control this.

3. **How should rate limiting interact with payment gating?** Pay-per-request IS rate limiting (bounded by wallet balance). However, a malicious client could flood the 402 challenge endpoint without paying. The challenge generation itself should be rate-limited to prevent denial-of-service via challenge exhaustion.

4. **Session payment lifecycle**: Should sessions auto-close after inactivity? The mpp-rs session provider holds a payment channel open until explicitly closed. An idle timeout (e.g., 30 minutes) would prevent channel funds from being locked indefinitely.

5. **Revenue reporting**: Should incoming payment totals be exposed via a `GET /mpp/revenue` endpoint? This would let the agent operator monitor earnings without checking the chain directly.
