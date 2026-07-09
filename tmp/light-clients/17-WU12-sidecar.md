# WU-12: Sidecar & Serve Chain Routes

**Layer**: 3
**Depends on**: WU-7 (VerifiedChainClient), WU-9 (tool handlers)
**Blocks**: none (leaf unit)
**Estimated effort**: 2 hours
**Crates**: `crates/roko-agent-server`, `crates/roko-serve`

---

## Overview

Add chain-related HTTP routes to both the per-agent sidecar (`roko-agent-server`) and the control plane (`roko-serve`). The sidecar gets a `chain` feature flag following the existing pattern. The control plane gets new verified-state routes alongside the existing 3 chain routes.

---

## Pre-read

- `crates/roko-agent-server/src/lib.rs` — `FeatureFlags` struct (line 42-48), `protected_router()` (line 97-114), `AgentServerBuilder` methods
- `crates/roko-agent-server/src/features/mod.rs` — module registry (7 existing modules)
- `crates/roko-agent-server/src/features/health.rs` — example feature module pattern
- `crates/roko-serve/src/routes/chain.rs` — existing 3 chain routes (chain_agents, chain_bounties, chain_status)
- `crates/roko-serve/src/state.rs` — `AppState` struct

---

## Tasks

### 12.1 Add `chain` feature flag to sidecar

**File**: `crates/roko-agent-server/src/lib.rs`

Add `chain: bool` to `FeatureFlags`:
```rust
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, Copy, Default)]
struct FeatureFlags {
    messaging: bool,
    predictions: bool,
    research: bool,
    tasks: bool,
    chain: bool,  // NEW
}
```

Add to `protected_router()`:
```rust
if self.features.chain {
    router = router.merge(features::chain::router());
}
```

Add builder method to `AgentServerBuilder`:
```rust
/// Enable the chain verification feature surface.
#[must_use]
pub fn chain(mut self) -> Self {
    self.features.chain = true;
    self.capabilities.push("chain".to_string());
    self
}
```

Add to `capability_is_live()`:
```rust
"chain" => features.chain,
```

### 12.2 Create `crates/roko-agent-server/src/features/chain.rs`

```rust
//! Chain verification routes for the per-agent sidecar.
//!
//! Provides lightweight chain query endpoints that use the agent's
//! attached chain client (if configured).

use std::sync::Arc;

use axum::extract::State;
use axum::routing::get;
use axum::{Json, Router};
use serde_json::{Value, json};

use crate::state::AgentState;

pub fn router() -> Router<Arc<AgentState>> {
    Router::new()
        .route("/chain/status", get(chain_status))
        .route("/chain/head", get(chain_head))
}

/// `GET /chain/status` — chain connectivity check.
async fn chain_status(State(state): State<Arc<AgentState>>) -> Json<Value> {
    let client = state.chain_client();
    match client {
        Some(client) => {
            let block = client.block_number().await.ok();
            let chain_id = client.chain_id().await.ok();
            Json(json!({
                "connected": block.is_some(),
                "block_number": block,
                "chain_id": chain_id,
                "name": client.name(),
            }))
        }
        None => Json(json!({
            "connected": false,
            "reason": "no chain client configured",
        })),
    }
}

/// `GET /chain/head` — latest block info.
async fn chain_head(State(state): State<Arc<AgentState>>) -> Json<Value> {
    let client = match state.chain_client() {
        Some(c) => c,
        None => return Json(json!({"error": "no chain client configured"})),
    };

    match client.block_number().await {
        Ok(num) => {
            let header = client.get_block_header(num).await.ok();
            Json(json!({
                "block_number": num,
                "block_hash": header.as_ref().map(|h| &h.hash),
                "timestamp": header.as_ref().map(|h| h.timestamp),
            }))
        }
        Err(e) => Json(json!({"error": format!("{e}")})),
    }
}
```

### 12.3 Register chain module in features/mod.rs

**File**: `crates/roko-agent-server/src/features/mod.rs`

Add:
```rust
pub mod chain;
```

### 12.4 Add verified routes to roko-serve

**File**: `crates/roko-serve/src/routes/chain.rs`

Add new routes to the existing `routes()` function:
```rust
pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/chain/agents", get(chain_agents))
        .route("/chain/bounties", get(chain_bounties))
        .route("/chain/status", get(chain_status))
        // NEW — verified chain routes
        .route("/chain/head", get(chain_head))
        .route("/chain/backends", get(chain_backends))
        .route("/chain/verified/balance/:address", get(chain_verified_balance))
}
```

Add handler implementations:

```rust
/// `GET /api/chain/head` — latest verified block head.
async fn chain_head(State(state): State<Arc<AppState>>) -> Result<Json<Value>, ApiError> {
    let client = state
        .chain_client
        .as_ref()
        .ok_or_else(|| ApiError::bad_request("chain client not configured"))?;

    let block = client
        .provider()
        .get_block_number()
        .await
        .map_err(|e| ApiError::internal(format!("block_number failed: {e}")))?;

    let block_resp = client
        .provider()
        .get_block_by_number(alloy::eips::BlockNumberOrTag::Number(block), false.into())
        .await
        .map_err(|e| ApiError::internal(format!("get_block failed: {e}")))?;

    Ok(Json(json!({
        "block_number": block,
        "block_hash": block_resp.as_ref().map(|b| format!("{:#x}", b.header.hash)),
        "state_root": block_resp.as_ref().map(|b| format!("{:#x}", b.header.state_root)),
        "timestamp": block_resp.as_ref().map(|b| b.header.timestamp),
    })))
}

/// `GET /api/chain/backends` — list configured chain backends.
async fn chain_backends(State(state): State<Arc<AppState>>) -> Result<Json<Value>, ApiError> {
    let config = state.load_roko_config();
    let backends = config.chain.resolve_backends();

    let entries: Vec<Value> = backends.iter().map(|(name, entry)| {
        json!({
            "name": name,
            "rpc_url": entry.rpc_url,
            "chain_id": entry.chain_id,
            "consensus": entry.consensus,
            "label": entry.label,
        })
    }).collect();

    Ok(Json(json!({
        "backends": entries,
        "default": config.chain.default_backend_name(),
    })))
}

/// `GET /api/chain/verified/balance/:address` — verified balance lookup.
async fn chain_verified_balance(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(address): axum::extract::Path<String>,
) -> Result<Json<Value>, ApiError> {
    // TODO: Wire to VerifiedChainClient once BackendPool is in AppState (WU-13)
    // For now, return regular balance via the existing chain client
    let client = state
        .chain_client
        .as_ref()
        .ok_or_else(|| ApiError::bad_request("chain client not configured"))?;

    let block = client
        .provider()
        .get_block_number()
        .await
        .map_err(|e| ApiError::internal(format!("block_number failed: {e}")))?;

    let addr: Address = address
        .parse()
        .map_err(|e| ApiError::bad_request(format!("invalid address: {e}")))?;

    let balance = client
        .provider()
        .get_balance(addr)
        .await
        .map_err(|e| ApiError::internal(format!("get_balance failed: {e}")))?;

    Ok(Json(json!({
        "address": address,
        "balance_wei": balance.to_string(),
        "block_number": block,
        "trust_level": "rpc_trusted",
        "note": "verified client not yet wired — using direct RPC",
    })))
}
```

### 12.5 Tests

**Sidecar tests** — add to `crates/roko-agent-server/src/lib.rs` test module:

```rust
#[tokio::test]
async fn chain_routes_not_present_without_feature() {
    let server = AgentServer::builder()
        .agent_id("agent-1")
        .build()
        .expect("server");
    let router = server.router();

    let resp = router
        .oneshot(
            Request::builder()
                .uri("/chain/status")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn chain_routes_present_with_feature() {
    let server = AgentServer::builder()
        .agent_id("agent-1")
        .chain()
        .build()
        .expect("server");
    let router = server.router();

    let resp = router
        .oneshot(
            Request::builder()
                .uri("/chain/status")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    // Should be OK (200) — returns "not configured" JSON, not 404
    assert_eq!(resp.status(), StatusCode::OK);
}
```

**Serve tests** — add chain event serialization test to `crates/roko-serve/src/events.rs`:

```rust
#[test]
fn chain_events_serialize() {
    let events = vec![
        ServerEvent::ChainNewBlock {
            backend: "tempo".into(),
            block_number: 42,
            block_hash: "0xabc".into(),
            timestamp: 1700000000,
        },
        ServerEvent::ChainEventsMatched {
            backend: "tempo".into(),
            block_number: 42,
            event_count: 3,
            summary: "3 events matched".into(),
        },
        ServerEvent::ChainWatcherHealth {
            backend: "tempo".into(),
            healthy: true,
            message: "ok".into(),
        },
    ];

    for event in &events {
        let json = serde_json::to_value(event).expect("serialize");
        assert!(json["type"].as_str().is_some());
    }
}
```

---

## Verification Checklist

- [ ] `FeatureFlags` has `chain: bool` field
- [ ] `AgentServerBuilder::chain()` method enables the feature
- [ ] `features/chain.rs` has `/chain/status` and `/chain/head` routes
- [ ] Chain routes return JSON even when no client is configured (graceful degradation)
- [ ] `features/mod.rs` includes `pub mod chain;`
- [ ] `protected_router()` conditionally merges chain routes
- [ ] `capability_is_live()` handles `"chain"` string
- [ ] roko-serve has `/chain/head`, `/chain/backends`, `/chain/verified/balance/:address` routes
- [ ] New routes added to existing `routes()` function (not a separate router)
- [ ] Test: chain routes NOT present when feature is disabled (404)
- [ ] Test: chain routes present when feature is enabled (200 with "not configured" JSON)
- [ ] `cargo test -p roko-agent-server` passes
- [ ] `cargo test -p roko-serve` passes
- [ ] `cargo test --workspace` — no breakage
