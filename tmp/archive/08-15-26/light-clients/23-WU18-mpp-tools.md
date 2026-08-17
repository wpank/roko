# WU-18: MPP Tool Handlers & Sidecar Routes

**Layer**: 4
**Depends on**: WU-17 (MppClient), WU-9 (Chain Tool Handlers), WU-12 (Sidecar Routes)
**Blocks**: WU-15 (Demo Scenario)
**Estimated effort**: 2-3 hours
**Crates**: `crates/roko-chain`, `crates/roko-cli`, `crates/roko-agent-server`, `crates/roko-serve`
**Feature gate**: `mpp` (on roko-chain); unconditional in CLI (uses `Option<MppClient>`)

---

## Overview

Add 3 MPP (Machine Payments Protocol) tool definitions, wire them into the existing `ChainToolHandler` dispatch, and expose corresponding HTTP routes on both the per-agent sidecar and the control plane. This gives agents the ability to pay for services, manage payment sessions, and discover MPP-enabled endpoints — all with on-chain settlement verification via the light client.

The tool definitions live in `roko-chain` (extending the existing `tools.rs`). The dispatch logic lives in `roko-cli` (extending `ChainToolHandler` from WU-9). The HTTP surface mirrors the pattern established in WU-12.

---

## Pre-read

- `crates/roko-chain/src/tools.rs` — existing `CHAIN_TOOL_NAMES` and tool definition functions
- `crates/roko-cli/src/chain_handler.rs` — `ChainToolHandler` struct and `execute()` match block (extended in WU-9)
- `crates/roko-cli/src/chain_registry.rs` — `chain_handler_map()`, `chain_handler_map_with_rpc()`
- `crates/roko-agent-server/src/features/chain.rs` — sidecar chain routes (created in WU-12)
- `crates/roko-serve/src/routes/chain.rs` — control plane chain routes (extended in WU-12)

---

## Tasks

### 18.1 Add MPP tool definitions

**File**: `crates/roko-chain/src/tools.rs` (extends existing `CHAIN_DOMAIN_TOOLS`)

Add 3 new tool definitions:

```rust
pub fn mpp_pay_tool_def() -> ToolDef {
    ToolDef {
        name: "chain.mpp_pay".into(),
        description: "Pay for a service via Tempo MPP (Machine Payments Protocol). \
            Sends an HTTP request with MPP payment authorization, verifies on-chain \
            settlement via light-client, and returns both the service response and \
            a cryptographically verified payment receipt.".into(),
        parameters: serde_json::json!({
            "type": "object",
            "properties": {
                "service_url": {
                    "type": "string",
                    "description": "URL of the MPP-enabled service endpoint"
                },
                "method": {
                    "type": "string",
                    "enum": ["GET", "POST"],
                    "description": "HTTP method (default: GET)"
                },
                "body": {
                    "type": "object",
                    "description": "Optional JSON request body for POST requests"
                }
            },
            "required": ["service_url"]
        }),
        category: "chain.mpp".into(),
    }
}

pub fn mpp_session_tool_def() -> ToolDef {
    ToolDef {
        name: "chain.mpp_session".into(),
        description: "Open or use an MPP session for repeated payments to a service. \
            Sessions use on-chain payment channels — pay once to open, then send \
            off-chain vouchers per request. Call with action='open' to start, \
            action='request' to send a paid request, action='close' to settle.".into(),
        parameters: serde_json::json!({
            "type": "object",
            "properties": {
                "service_url": {
                    "type": "string",
                    "description": "URL of the MPP-enabled service"
                },
                "action": {
                    "type": "string",
                    "enum": ["open", "request", "close"],
                    "description": "Session action"
                },
                "body": {
                    "type": "object",
                    "description": "Request body (for 'request' action)"
                },
                "deposit": {
                    "type": "string",
                    "description": "Deposit amount in base units (for 'open' action)"
                }
            },
            "required": ["service_url", "action"]
        }),
        category: "chain.mpp".into(),
    }
}

pub fn mpp_discover_tool_def() -> ToolDef {
    ToolDef {
        name: "chain.mpp_discover".into(),
        description: "Discover MPP-enabled services by fetching their OpenAPI \
            description. Returns available endpoints, pricing, accepted payment \
            methods, and service capabilities.".into(),
        parameters: serde_json::json!({
            "type": "object",
            "properties": {
                "service_url": {
                    "type": "string",
                    "description": "Base URL of the service to discover (fetches /openapi.json)"
                }
            },
            "required": ["service_url"]
        }),
        category: "chain.mpp".into(),
    }
}
```

Add the 3 new names to `CHAIN_TOOL_NAMES`:
```rust
"chain.mpp_pay",
"chain.mpp_session",
"chain.mpp_discover",
```

### 18.2 Add MPP dispatch arms to ChainToolHandler

**File**: `crates/roko-cli/src/chain_handler.rs`

Add `mpp_client: Option<Arc<MppClient>>` field to `ChainToolHandler`:
```rust
pub struct ChainToolHandler {
    pub client: Arc<dyn ChainClient>,
    pub wallet: Option<Arc<dyn ChainWallet>>,
    pub tool_name: String,
    pub rpc_url: Option<String>,
    pub verified_client: Option<Arc<VerifiedChainClient>>,
    // NEW
    pub mpp_client: Option<Arc<MppClient>>,
}
```

Add match arms in `execute()`:
```rust
"chain.mpp_pay" => self.handle_mpp_pay(args).await,
"chain.mpp_session" => self.handle_mpp_session(args).await,
"chain.mpp_discover" => self.handle_mpp_discover(args).await,
```

Handler methods:

```rust
async fn handle_mpp_pay(&self, args: &Value) -> ToolResult {
    let mpp = self.mpp_client.as_ref()
        .ok_or_else(|| ToolError::Other("MPP not configured — add [chain.mpp] to roko.toml".into()))?;
    let service_url = args["service_url"].as_str()
        .ok_or_else(|| ToolError::Other("missing 'service_url' parameter".into()))?;
    let method = args.get("method").and_then(|v| v.as_str()).unwrap_or("GET");
    let body = args.get("body").cloned();

    let payment = mpp.pay_one_time(service_url, method, body.as_ref()).await
        .map_err(|e| ToolError::Other(format!("mpp_pay failed: {e}")))?;

    ToolResult::structured(serde_json::json!({
        "service_response": payment.response_body,
        "settlement": {
            "tx_hash": payment.tx_hash,
            "amount": payment.amount,
            "token": payment.token,
            "block_number": payment.block_number,
            "trust_level": format!("{:?}", payment.trust_level),
            "verified_at": payment.verified_at,
        }
    }))
}

async fn handle_mpp_session(&self, args: &Value) -> ToolResult {
    let mpp = self.mpp_client.as_ref()
        .ok_or_else(|| ToolError::Other("MPP not configured — add [chain.mpp] to roko.toml".into()))?;
    let service_url = args["service_url"].as_str()
        .ok_or_else(|| ToolError::Other("missing 'service_url' parameter".into()))?;
    let action = args["action"].as_str()
        .ok_or_else(|| ToolError::Other("missing 'action' parameter".into()))?;

    match action {
        "open" => {
            let deposit = args.get("deposit").and_then(|v| v.as_str());
            let session = mpp.session_open(service_url, deposit).await
                .map_err(|e| ToolError::Other(format!("session open failed: {e}")))?;
            ToolResult::structured(serde_json::json!({
                "action": "opened",
                "session_id": session.id,
                "service_url": service_url,
                "deposit": session.deposit,
                "channel_address": session.channel_address,
            }))
        }
        "request" => {
            let body = args.get("body").cloned();
            let resp = mpp.session_request(service_url, body.as_ref()).await
                .map_err(|e| ToolError::Other(format!("session request failed: {e}")))?;
            ToolResult::structured(serde_json::json!({
                "action": "request",
                "service_response": resp.response_body,
                "voucher_seq": resp.voucher_seq,
                "cumulative_amount": resp.cumulative_amount,
            }))
        }
        "close" => {
            let settlement = mpp.session_close(service_url).await
                .map_err(|e| ToolError::Other(format!("session close failed: {e}")))?;
            ToolResult::structured(serde_json::json!({
                "action": "closed",
                "tx_hash": settlement.tx_hash,
                "final_amount": settlement.final_amount,
                "voucher_count": settlement.voucher_count,
                "block_number": settlement.block_number,
            }))
        }
        other => Err(ToolError::Other(format!("unknown session action: {other} (expected open/request/close)"))),
    }
}

async fn handle_mpp_discover(&self, args: &Value) -> ToolResult {
    let mpp = self.mpp_client.as_ref()
        .ok_or_else(|| ToolError::Other("MPP not configured — add [chain.mpp] to roko.toml".into()))?;
    let service_url = args["service_url"].as_str()
        .ok_or_else(|| ToolError::Other("missing 'service_url' parameter".into()))?;

    let spec = mpp.discover(service_url).await
        .map_err(|e| ToolError::Other(format!("mpp_discover failed: {e}")))?;

    ToolResult::structured(serde_json::json!({
        "service_url": service_url,
        "title": spec.title,
        "description": spec.description,
        "endpoints": spec.endpoints,
        "payment_methods": spec.payment_methods,
        "pricing": spec.pricing,
    }))
}
```

Each handler checks `self.mpp_client.is_some()` first and returns a clear error if MPP is not configured.

### 18.3 Add sidecar routes

**File**: `crates/roko-agent-server/src/features/chain.rs` (extends existing, created in WU-12)

Add routes to the existing `router()` function:
```rust
pub fn router() -> Router<Arc<AgentState>> {
    Router::new()
        .route("/chain/status", get(chain_status))
        .route("/chain/head", get(chain_head))
        // NEW — MPP routes
        .route("/chain/mpp/pay", post(mpp_pay))
        .route("/chain/mpp/session", post(mpp_session))
        .route("/chain/mpp/discover", get(mpp_discover))
}
```

Add handler implementations:

```rust
/// `POST /chain/mpp/pay` — one-time MPP payment.
async fn mpp_pay(
    State(state): State<Arc<AgentState>>,
    Json(body): Json<MppPayRequest>,
) -> Result<Json<MppPayResponse>, AppError> {
    let mpp = state.mpp_client()
        .ok_or_else(|| AppError::bad_request("MPP client not configured"))?;
    let payment = mpp.pay_one_time(&body.service_url, body.method.as_deref().unwrap_or("GET"), body.body.as_ref()).await
        .map_err(|e| AppError::internal(format!("mpp_pay failed: {e}")))?;
    Ok(Json(MppPayResponse {
        service_response: payment.response_body,
        settlement: VerifiedPaymentSummary {
            tx_hash: payment.tx_hash,
            amount: payment.amount,
            token: payment.token,
            block_number: payment.block_number,
            trust_level: format!("{:?}", payment.trust_level),
            verified_at: payment.verified_at,
        },
    }))
}

/// `POST /chain/mpp/session` — session lifecycle (open/request/close).
async fn mpp_session(
    State(state): State<Arc<AgentState>>,
    Json(body): Json<MppSessionRequest>,
) -> Result<Json<MppSessionResponse>, AppError> {
    let mpp = state.mpp_client()
        .ok_or_else(|| AppError::bad_request("MPP client not configured"))?;
    let result = match body.action.as_str() {
        "open" => {
            let session = mpp.session_open(&body.service_url, body.deposit.as_deref()).await
                .map_err(|e| AppError::internal(format!("session open: {e}")))?;
            serde_json::json!({
                "action": "opened",
                "session_id": session.id,
                "deposit": session.deposit,
                "channel_address": session.channel_address,
            })
        }
        "request" => {
            let resp = mpp.session_request(&body.service_url, body.body.as_ref()).await
                .map_err(|e| AppError::internal(format!("session request: {e}")))?;
            serde_json::json!({
                "action": "request",
                "service_response": resp.response_body,
                "voucher_seq": resp.voucher_seq,
                "cumulative_amount": resp.cumulative_amount,
            })
        }
        "close" => {
            let settlement = mpp.session_close(&body.service_url).await
                .map_err(|e| AppError::internal(format!("session close: {e}")))?;
            serde_json::json!({
                "action": "closed",
                "tx_hash": settlement.tx_hash,
                "final_amount": settlement.final_amount,
                "voucher_count": settlement.voucher_count,
            })
        }
        other => return Err(AppError::bad_request(format!("unknown action: {other}"))),
    };
    Ok(Json(MppSessionResponse { result }))
}

/// `GET /chain/mpp/discover?url=<service_url>` — discover service capabilities.
async fn mpp_discover(
    State(state): State<Arc<AgentState>>,
    Query(params): Query<MppDiscoverParams>,
) -> Result<Json<MppDiscoverResponse>, AppError> {
    let mpp = state.mpp_client()
        .ok_or_else(|| AppError::bad_request("MPP client not configured"))?;
    let spec = mpp.discover(&params.url).await
        .map_err(|e| AppError::internal(format!("mpp_discover: {e}")))?;
    Ok(Json(MppDiscoverResponse {
        service_url: params.url,
        title: spec.title,
        description: spec.description,
        endpoints: spec.endpoints,
        payment_methods: spec.payment_methods,
        pricing: spec.pricing,
    }))
}
```

Request/response types:

```rust
#[derive(Deserialize)]
pub struct MppPayRequest {
    pub service_url: String,
    pub method: Option<String>,  // "GET" or "POST", default "GET"
    pub body: Option<serde_json::Value>,
}

#[derive(Serialize)]
pub struct MppPayResponse {
    pub service_response: serde_json::Value,
    pub settlement: VerifiedPaymentSummary,
}

#[derive(Serialize)]
pub struct VerifiedPaymentSummary {
    pub tx_hash: String,
    pub amount: String,
    pub token: String,
    pub block_number: u64,
    pub trust_level: String,
    pub verified_at: u64,
}

#[derive(Deserialize)]
pub struct MppSessionRequest {
    pub service_url: String,
    pub action: String,  // "open", "request", "close"
    pub body: Option<serde_json::Value>,
    pub deposit: Option<String>,
}

#[derive(Serialize)]
pub struct MppSessionResponse {
    pub result: serde_json::Value,
}

#[derive(Deserialize)]
pub struct MppDiscoverParams {
    pub url: String,
}

#[derive(Serialize)]
pub struct MppDiscoverResponse {
    pub service_url: String,
    pub title: String,
    pub description: String,
    pub endpoints: serde_json::Value,
    pub payment_methods: Vec<String>,
    pub pricing: serde_json::Value,
}
```

### 18.4 Add roko-serve routes

**File**: `crates/roko-serve/src/routes/chain.rs` (extends existing)

Add to the existing `routes()` function:
```rust
// NEW — MPP routes
.route("/chain/mpp/pay", post(mpp_pay))
.route("/chain/mpp/session", post(mpp_session))
.route("/chain/mpp/discover", get(mpp_discover))
```

These mirror the sidecar routes but go through the serve-level `AppState` which includes the global `BackendPool` and `MppClient`. The handler implementations follow the same pattern as the sidecar handlers, extracting `MppClient` from `state.mpp_client` instead of `AgentState`.

### 18.5 Add MCP tool entries

The 3 MPP tools should be registered as MCP tools in the sidecar's MCP tool catalog, following the pattern used by existing chain MCP tools. Each tool entry maps to the corresponding tool definition from 18.1:

- `chain.mpp_pay` — maps to `mpp_pay_tool_def()`
- `chain.mpp_session` — maps to `mpp_session_tool_def()`
- `chain.mpp_discover` — maps to `mpp_discover_tool_def()`

### 18.6 Update chain_registry.rs

**File**: `crates/roko-cli/src/chain_registry.rs`

Add `chain.mpp_pay`, `chain.mpp_session`, `chain.mpp_discover` to the handler map. Update `chain_handler_map_with_rpc()` to accept optional `MppClient`:

```rust
pub fn chain_handler_map_with_rpc(
    client: Arc<dyn ChainClient>,
    wallet: Option<Arc<dyn ChainWallet>>,
    rpc_url: Option<String>,
    verified_client: Option<Arc<VerifiedChainClient>>,
    mpp_client: Option<Arc<MppClient>>,  // NEW
) -> HashMap<String, Arc<dyn ToolHandler>> {
    CHAIN_TOOL_NAMES
        .iter()
        .map(|&name| {
            let h: Arc<dyn ToolHandler> = Arc::new(ChainToolHandler {
                client: Arc::clone(&client),
                wallet: wallet.clone(),
                tool_name: name.to_string(),
                rpc_url: rpc_url.clone(),
                verified_client: verified_client.clone(),
                mpp_client: mpp_client.clone(),  // NEW
            });
            (name.to_string(), h)
        })
        .collect()
}
```

Update all call sites to pass `None` for `mpp_client` (WU-13 or orchestrator wiring will supply the real value).

---

## Verification Checklist

- [ ] 3 new tool definitions in `crates/roko-chain/src/tools.rs`
- [ ] `CHAIN_TOOL_NAMES` includes `chain.mpp_pay`, `chain.mpp_session`, `chain.mpp_discover`
- [ ] `ChainToolHandler` has `mpp_client: Option<Arc<MppClient>>` field
- [ ] 3 new match arms in `execute()` method
- [ ] Each handler checks for `mpp_client` and returns clear error if missing
- [ ] `chain_handler_map_with_rpc()` accepts `mpp_client` parameter
- [ ] All existing call sites updated (pass `None` for backward compatibility)
- [ ] Sidecar has `POST /chain/mpp/pay`, `POST /chain/mpp/session`, `GET /chain/mpp/discover` routes
- [ ] roko-serve has `POST /api/chain/mpp/pay`, `POST /api/chain/mpp/session`, `GET /api/chain/mpp/discover` routes
- [ ] Request/response types derive correct serde traits
- [ ] MCP tool catalog includes 3 MPP tools
- [ ] `cargo build -p roko-chain --features mpp`
- [ ] `cargo build -p roko-cli --features mpp`
- [ ] `cargo test -p roko-chain --features mpp`
- [ ] `cargo clippy --workspace --no-deps -- -D warnings`

---

## Open Questions

1. Should `mpp_discover` cache OpenAPI responses? (The discovery spec recommends 5-minute cache)
2. Should session state persist across agent restarts? (Currently mpp-rs `TempoSessionProvider` is in-memory)
3. How should MPP tool results feed into the episode log? (Payments are high-value events)
