//! `POST /mcp`: Roko as a tool a chat host calls over MCP (decision 13; 9106
//! put MCP first, for Hermes and OpenClaw alike).
//!
//! Streamable HTTP with one JSON response per request and no SSE stream
//! (9114): `initialize`, `ping`, `tools/list` and `tools/call`, with errors as
//! JSON-RPC errors and notifications acknowledged with 202 and no body. The
//! tool names and argument schemas are the contract with hosts, documented in
//! `docs/v3/26-HTTP-API.md`:
//!
//! - `run_status { run_id, wait_secs }` returns the run's summary, as `GET
//!   /api/runs/{run_id}/summary` does, after waiting up to `wait_secs` (at
//!   most 30) for its state to change;
//! - `recall { query, limit }` returns what the knowledge store holds on
//!   `query`.
//!
//! Both only read. There is no `remember`: personal memory stays with the host
//! (B9), and serve has no knowledge write route (AD-10).

use std::sync::Arc;
use std::time::Duration;

use axum::body::Bytes;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::post;
use axum::{Extension, Json, Router};
use roko_mcp_stdio::{JsonRpcError, JsonRpcRequest, MCP_PROTOCOL_VERSION};
use serde_json::{Value, json};

use super::middleware::{AuthContext, is_scope_sufficient};
use crate::error::ApiError;
use crate::state::AppState;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new().route("/mcp", post(mcp))
}

/// The longest `run_status` waits for a run's state to change.
const MAX_WAIT_SECS: u64 = 30;

/// How often `run_status` looks whether the run's state changed.
const WAIT_POLL: Duration = Duration::from_secs(1);

/// How many entries `recall` returns when the call names no `limit`.
const DEFAULT_RECALL_LIMIT: usize = 5;

/// The most entries `recall` returns.
const MAX_RECALL_LIMIT: usize = 50;

/// `POST /mcp` — one JSON-RPC message in, one JSON response out.
async fn mcp(
    State(state): State<Arc<AppState>>,
    auth: Option<Extension<AuthContext>>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    if !origin_allowed(&headers) {
        return ApiError::forbidden("MCP calls from a web page on another origin are refused")
            .into_response();
    }
    let request = match parse_request(&body) {
        Ok(request) => request,
        Err(error) => return rpc_response(Value::Null, Err(error)),
    };
    // A notification, such as `notifications/initialized`, gets no response.
    if request.method.starts_with("notifications/") {
        return StatusCode::ACCEPTED.into_response();
    }
    let id = request.id.clone();
    let auth = auth.map(|Extension(context)| context);
    let result = handle(&state, auth.as_ref(), request).await;
    rpc_response(id, result)
}

/// Whether the request's `Origin`, when it sends one, is this machine. MCP's
/// Streamable HTTP transport asks servers to check it, so that a web page
/// cannot reach `/mcp` through DNS rebinding; hosts call `/mcp` from outside a
/// browser and send none.
fn origin_allowed(headers: &HeaderMap) -> bool {
    let Some(origin) = headers.get(header::ORIGIN) else {
        return true;
    };
    let Some(authority) = origin.to_str().ok().and_then(|origin| {
        origin
            .strip_prefix("http://")
            .or_else(|| origin.strip_prefix("https://"))
    }) else {
        return false;
    };
    let host = match authority.strip_prefix('[') {
        Some(bracketed) => bracketed.split(']').next(),
        None => authority.split(':').next(),
    };
    matches!(host, Some("localhost" | "127.0.0.1" | "::1"))
}

/// The one JSON-RPC 2.0 request in `body`.
fn parse_request(body: &[u8]) -> Result<JsonRpcRequest, JsonRpcError> {
    let value: Value = serde_json::from_slice(body)
        .map_err(|error| JsonRpcError::parse_error(error.to_string()))?;
    if !value.is_object() {
        // The HTTP transport takes one message per request, never a batch.
        return Err(JsonRpcError::invalid_request(
            "expected one JSON-RPC request object",
        ));
    }
    let request: JsonRpcRequest = serde_json::from_value(value)
        .map_err(|error| JsonRpcError::invalid_request(error.to_string()))?;
    if request.jsonrpc != "2.0" {
        return Err(JsonRpcError::invalid_request("jsonrpc must be \"2.0\""));
    }
    Ok(request)
}

/// The JSON-RPC response to request `id`: its result, or its error.
fn rpc_response(id: Value, result: Result<Value, JsonRpcError>) -> Response {
    let body = match result {
        Ok(result) => json!({ "jsonrpc": "2.0", "id": id, "result": result }),
        Err(error) => json!({ "jsonrpc": "2.0", "id": id, "error": error }),
    };
    Json(body).into_response()
}

/// Answer one MCP request.
async fn handle(
    state: &Arc<AppState>,
    auth: Option<&AuthContext>,
    request: JsonRpcRequest,
) -> Result<Value, JsonRpcError> {
    match request.method.as_str() {
        "initialize" => Ok(json!({
            "protocolVersion": MCP_PROTOCOL_VERSION,
            "capabilities": { "tools": {} },
            "serverInfo": {
                "name": "roko",
                "version": env!("CARGO_PKG_VERSION"),
            },
        })),
        "ping" => Ok(json!({})),
        "tools/list" => {
            let tools: Vec<Value> = tools().into_iter().map(|(spec, _)| spec).collect();
            Ok(json!({ "tools": tools }))
        }
        "tools/call" => call_tool(state, auth, &request.params).await,
        method => Err(JsonRpcError::method_not_found(method)),
    }
}

/// The tools `/mcp` offers, as `tools/list` lists them, each with the least
/// scope a caller needs to call it.
fn tools() -> [(Value, &'static str); 2] {
    [
        (
            read_only_tool(
                "run_status",
                "The state of a Roko run (queued, running, succeeded, failed, unverified or \
                 cancelled), with its verdict once it has ended, its cost, how its tasks \
                 ended and at most five milestones. Waits up to wait_secs, at most 30, for \
                 the state to change.",
                json!({
                    "type": "object",
                    "properties": {
                        "run_id": { "type": "string", "description": "The run's id" },
                        "wait_secs": {
                            "type": "integer",
                            "minimum": 0,
                            "maximum": MAX_WAIT_SECS,
                            "default": 0,
                            "description": "How long to wait for the run's state to change"
                        }
                    },
                    "required": ["run_id"],
                    "additionalProperties": false
                }),
            ),
            "read",
        ),
        (
            read_only_tool(
                "recall",
                "What Roko's knowledge store holds on a query: the lessons and facts its \
                 runs left, most relevant first.",
                json!({
                    "type": "object",
                    "properties": {
                        "query": { "type": "string", "description": "What to recall" },
                        "limit": {
                            "type": "integer",
                            "minimum": 1,
                            "maximum": MAX_RECALL_LIMIT,
                            "default": DEFAULT_RECALL_LIMIT,
                            "description": "The most entries to return"
                        }
                    },
                    "required": ["query"],
                    "additionalProperties": false
                }),
            ),
            "read",
        ),
    ]
}

/// The spec of a tool that only reads and touches nothing outside Roko.
fn read_only_tool(name: &str, description: &str, input_schema: Value) -> Value {
    json!({
        "name": name,
        "description": description,
        "inputSchema": input_schema,
        "annotations": {
            "readOnlyHint": true,
            "openWorldHint": false,
        },
    })
}

/// Call the tool `params` names with its arguments, once the caller's scope
/// covers it. A failure of the tool itself, such as an unknown run, is a
/// result with `isError: true` the calling model can read; a call that names
/// no known tool, or bad arguments, is a JSON-RPC error.
async fn call_tool(
    state: &Arc<AppState>,
    auth: Option<&AuthContext>,
    params: &Value,
) -> Result<Value, JsonRpcError> {
    let name = params
        .get("name")
        .and_then(Value::as_str)
        .ok_or_else(|| JsonRpcError::invalid_params("tools/call needs a tool name"))?;
    let unknown = || JsonRpcError::invalid_params(format!("unknown tool: {name}"));
    let (_, scope) = tools()
        .into_iter()
        .find(|(spec, _)| spec["name"] == name)
        .ok_or_else(unknown)?;
    // Without auth every tool may be called, as every route may.
    if let Some(context) = auth
        && !is_scope_sufficient(&context.scope, scope)
    {
        return Ok(tool_error(&format!("the {name} tool needs the {scope} scope")));
    }
    let arguments = params
        .get("arguments")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let outcome = match name {
        "run_status" => {
            let (run_id, wait_secs) = run_status_args(&arguments)?;
            run_status(state, &run_id, wait_secs).await
        }
        "recall" => {
            let (query, limit) = recall_args(&arguments)?;
            super::neuro::query_knowledge(state, &query, limit)
        }
        _ => return Err(unknown()),
    };
    Ok(match outcome {
        Ok(value) => tool_result(&value),
        Err(error) => tool_error(&error.message),
    })
}

/// The `run_id` and `wait_secs` of a `run_status` call; a longer wait is cut
/// to [`MAX_WAIT_SECS`].
fn run_status_args(arguments: &Value) -> Result<(String, u64), JsonRpcError> {
    let run_id = arguments
        .get("run_id")
        .and_then(Value::as_str)
        .filter(|run_id| !run_id.trim().is_empty())
        .ok_or_else(|| JsonRpcError::invalid_params("run_status needs a run_id"))?;
    let wait_secs = match arguments.get("wait_secs") {
        None | Some(Value::Null) => 0,
        Some(wait) => wait.as_u64().ok_or_else(|| {
            JsonRpcError::invalid_params("wait_secs must be a whole number of seconds")
        })?,
    };
    Ok((run_id.to_string(), wait_secs.min(MAX_WAIT_SECS)))
}

/// The `query` and `limit` of a `recall` call; `limit` is kept between 1 and
/// [`MAX_RECALL_LIMIT`].
fn recall_args(arguments: &Value) -> Result<(String, usize), JsonRpcError> {
    let query = arguments
        .get("query")
        .and_then(Value::as_str)
        .ok_or_else(|| JsonRpcError::invalid_params("recall needs a query"))?;
    let limit = match arguments.get("limit") {
        None | Some(Value::Null) => DEFAULT_RECALL_LIMIT,
        Some(limit) => limit
            .as_u64()
            .and_then(|limit| usize::try_from(limit).ok())
            .ok_or_else(|| JsonRpcError::invalid_params("limit must be a whole number"))?,
    };
    Ok((query.to_string(), limit.clamp(1, MAX_RECALL_LIMIT)))
}

/// The `run_status` tool: the run's summary
/// ([`super::runs::summarize_run`]) once its state differs from its state
/// when the call came, once it has ended, or once `wait_secs` pass.
async fn run_status(
    state: &Arc<AppState>,
    run_id: &str,
    wait_secs: u64,
) -> Result<Value, ApiError> {
    let mut summary = super::runs::summarize_run(state, run_id).await?;
    let first_state = summary["state"].clone();
    let deadline = tokio::time::Instant::now() + Duration::from_secs(wait_secs);
    while summary["active"] == true
        && summary["state"] == first_state
        && tokio::time::Instant::now() < deadline
    {
        tokio::time::sleep(WAIT_POLL).await;
        summary = super::runs::summarize_run(state, run_id).await?;
    }
    Ok(summary)
}

/// A tool's result: its JSON as text, as every MCP client reads it, and as
/// structured content.
fn tool_result(value: &Value) -> Value {
    json!({
        "content": [{ "type": "text", "text": value.to_string() }],
        "structuredContent": value,
        "isError": false,
    })
}

/// A tool's failure, for the calling model to read.
fn tool_error(message: &str) -> Value {
    json!({
        "content": [{ "type": "text", "text": message }],
        "isError": true,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use roko_core::config::ServeAuthConfig;
    use roko_core::config::schema::RokoConfig;
    use tower::ServiceExt;

    /// The full router over a fresh workspace, with `auth`.
    fn router(auth: ServeAuthConfig) -> (tempfile::TempDir, Router) {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut config = RokoConfig::default();
        config.serve.auth = auth.clone();
        let deploy_backend = Arc::from(
            crate::deploy::create_backend("manual", None, None, None).expect("manual backend"),
        );
        let state = Arc::new(
            AppState::new(
                dir.path().to_path_buf(),
                Arc::new(crate::runtime::NoOpRuntime),
                config,
                deploy_backend,
            )
            .expect("AppState::new"),
        );
        (dir, crate::routes::build_router(state, &[], auth))
    }

    /// `POST /mcp` with `message` and `headers`: the status and JSON body.
    async fn post_mcp(
        router: &Router,
        message: &Value,
        headers: &[(&str, &str)],
    ) -> (StatusCode, Value) {
        let mut request = Request::builder()
            .method("POST")
            .uri("/mcp")
            .header(header::CONTENT_TYPE, "application/json");
        for (name, value) in headers {
            request = request.header(*name, *value);
        }
        let request = request
            .body(Body::from(message.to_string()))
            .expect("build the request");
        let response = router.clone().oneshot(request).await.expect("a response");
        let status = response.status();
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("the response body");
        let body = serde_json::from_slice(&body).unwrap_or(Value::Null);
        (status, body)
    }

    /// 9114: `tools/list` returns `run_status` and `recall`, each with its
    /// argument schema and `readOnlyHint`. With auth on, a call without a key
    /// is refused and a read-only key may call; a web page on another origin
    /// is refused, and a notification gets 202.
    #[tokio::test]
    async fn mcp_tools_list_returns_annotated_tools() {
        let list = json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/list" });
        let (_dir, open) = router(ServeAuthConfig::default());

        let (status, body) = post_mcp(&open, &list, &[]).await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(body["id"], 1, "{body}");
        let tools = body["result"]["tools"].as_array().expect("a tool list");
        let names: Vec<&str> = tools
            .iter()
            .map(|tool| tool["name"].as_str().expect("a tool name"))
            .collect();
        assert_eq!(names, ["run_status", "recall"]);
        for tool in tools {
            assert_eq!(tool["annotations"]["readOnlyHint"], true, "{tool}");
            assert_eq!(tool["inputSchema"]["type"], "object", "{tool}");
        }
        assert_eq!(tools[0]["inputSchema"]["required"], json!(["run_id"]));
        assert_eq!(tools[1]["inputSchema"]["required"], json!(["query"]));

        let initialized = json!({ "jsonrpc": "2.0", "method": "notifications/initialized" });
        let (status, _) = post_mcp(&open, &initialized, &[]).await;
        assert_eq!(status, StatusCode::ACCEPTED);
        let (status, _) = post_mcp(&open, &list, &[("origin", "http://evil.example")]).await;
        assert_eq!(status, StatusCode::FORBIDDEN);
        let (status, _) = post_mcp(&open, &list, &[("origin", "http://localhost:5173")]).await;
        assert_eq!(status, StatusCode::OK);

        let reader = "mcp-read-only-key";
        let (_dir, guarded) = router(ServeAuthConfig {
            enabled: true,
            api_key: "mcp-admin-key".into(),
            api_keys: vec![roko_core::config::ApiKeyEntry {
                name: "reader".into(),
                key_hash: super::super::middleware::hash_api_key(reader),
                scope: "read".into(),
                created_at: "2026-10-02T00:00:00Z".into(),
                expires_at: None,
                last_used_at: None,
                previous_key_hashes: Vec::new(),
            }],
            ..ServeAuthConfig::default()
        });
        let (status, body) = post_mcp(&guarded, &list, &[]).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "{body}");
        let (status, body) = post_mcp(&guarded, &list, &[("x-api-key", reader)]).await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(body["result"]["tools"].as_array().map(Vec::len), Some(2));
    }

    /// 9114: `tools/call` runs the tools. `recall` answers from the knowledge
    /// store; `run_status` of a run nobody knows is a tool error the host
    /// can read; a call without its required argument, or of an unknown
    /// tool, is a JSON-RPC error.
    #[tokio::test]
    async fn mcp_tools_call_runs_recall_and_run_status() {
        let (_dir, open) = router(ServeAuthConfig::default());
        let call = |id: u64, name: &str, arguments: Value| {
            json!({
                "jsonrpc": "2.0",
                "id": id,
                "method": "tools/call",
                "params": { "name": name, "arguments": arguments },
            })
        };

        let recall = call(1, "recall", json!({ "query": "flaky tests" }));
        let (status, body) = post_mcp(&open, &recall, &[]).await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(body["result"]["isError"], false, "{body}");
        assert_eq!(body["result"]["structuredContent"]["total"], 0, "{body}");

        let status_call = call(2, "run_status", json!({ "run_id": "no-such-run" }));
        let (status, body) = post_mcp(&open, &status_call, &[]).await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(body["result"]["isError"], true, "{body}");

        let missing = call(3, "run_status", json!({}));
        let (_, body) = post_mcp(&open, &missing, &[]).await;
        assert!(body["error"]["code"].is_i64(), "{body}");
        assert_eq!(body["id"], 3, "{body}");
        let unknown = call(4, "remember", json!({ "text": "x" }));
        let (_, body) = post_mcp(&open, &unknown, &[]).await;
        assert!(body["error"]["message"].is_string(), "{body}");
    }
}
