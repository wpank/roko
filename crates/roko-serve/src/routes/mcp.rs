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
//!   `query`;
//! - `run_prompt { prompt, domain, max_usd }`, `plan_run { plan_id, resume,
//!   max_usd }`, `plan_generate { prompt }` and `run_cancel { run_id }` start,
//!   plan and stop work (9115). They answer at once with `{ run_id, state,
//!   links }` for `run_status` to follow, need the `write` scope, and are
//!   annotated so the host asks its user first; the paid ones say so in their
//!   description and in `_meta` (`roko/paid`). No tool picks a model: routing
//!   stays with the ladder.
//!
//! `run_status` and `recall` only read. There is no `remember`: personal
//! memory stays with the host (B9), and serve has no knowledge write route
//! (AD-10).

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
use crate::runtime::{PromptPlanOptions, RunOrigin};
use crate::state::{AppState, OperationStatus, RunState};
use roko_core::TaskDomain;

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
/// scope a caller needs to call it. `run_status` and `recall` only read. The
/// run tools (9115) start, plan or stop paid work in the workspace, and their
/// annotations ask the host to check with its user before calling them.
fn tools() -> Vec<(Value, &'static str)> {
    vec![
        (
            read_only_tool(
                "run_status",
                "The state of a Roko run (queued, running, succeeded, failed, unverified or \
                 cancelled), with its verdict once it has ended, its cost, how its tasks \
                 ended and at most five milestones. Waits up to wait_secs, at most 30, for \
                 the state to change. Also follows a plan_generate run.",
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
        (
            tool(
                "run_prompt",
                "Have Roko do a piece of work as one gated task, as `roko run` does: an \
                 agent works in Roko's workspace and the workspace's checks verify the \
                 result. Paid: it spends model budget, at most max_usd. Answers at once \
                 with the run's id; follow it with run_status.",
                json!({
                    "type": "object",
                    "properties": {
                        "prompt": { "type": "string", "description": "The work to do" },
                        "domain": {
                            "type": "string",
                            "description": "The kind of work, such as code, research or docs"
                        },
                        "max_usd": max_usd_schema()
                    },
                    "required": ["prompt", "max_usd"],
                    "additionalProperties": false
                }),
                starts_paid_work(),
                true,
            ),
            "write",
        ),
        (
            tool(
                "plan_run",
                "Run one of Roko's plans, or queue it behind the run in progress. Paid: it \
                 spends model budget, at most max_usd. Answers at once with the run's id \
                 and whether it is running or queued; follow it with run_status.",
                json!({
                    "type": "object",
                    "properties": {
                        "plan_id": { "type": "string", "description": "The plan's id" },
                        "resume": {
                            "type": "boolean",
                            "default": false,
                            "description": "Resume from the plan's checkpoint, not start over"
                        },
                        "max_usd": max_usd_schema()
                    },
                    "required": ["plan_id", "max_usd"],
                    "additionalProperties": false
                }),
                starts_paid_work(),
                true,
            ),
            "write",
        ),
        (
            tool(
                "plan_generate",
                "Have Roko's planner write a plan for a request, without running it. Paid: \
                 the planner model spends budget. Answers at once with a run id, which \
                 run_status follows, and the plan's id, which plan_run takes once the plan \
                 is written.",
                json!({
                    "type": "object",
                    "properties": {
                        "prompt": { "type": "string", "description": "What the plan is for" }
                    },
                    "required": ["prompt"],
                    "additionalProperties": false
                }),
                json!({
                    "readOnlyHint": false,
                    "destructiveHint": false,
                    "idempotentHint": false,
                    "openWorldHint": true,
                }),
                true,
            ),
            "write",
        ),
        (
            tool(
                "run_cancel",
                "Stop a Roko run that is running or queued: a run_prompt or plan_run run.",
                json!({
                    "type": "object",
                    "properties": {
                        "run_id": { "type": "string", "description": "The run's id" }
                    },
                    "required": ["run_id"],
                    "additionalProperties": false
                }),
                json!({
                    "readOnlyHint": false,
                    "destructiveHint": true,
                    "idempotentHint": true,
                    "openWorldHint": false,
                }),
                false,
            ),
            "write",
        ),
    ]
}

/// The spec of a tool that only reads and touches nothing outside Roko.
fn read_only_tool(name: &str, description: &str, input_schema: Value) -> Value {
    let annotations = json!({ "readOnlyHint": true, "openWorldHint": false });
    tool(name, description, input_schema, annotations, false)
}

/// The annotations of a tool that starts paid work in the workspace: it
/// changes things, reaches model providers, and a second call starts a second
/// run, so a host asks its user before calling it.
fn starts_paid_work() -> Value {
    json!({
        "readOnlyHint": false,
        "destructiveHint": true,
        "idempotentHint": false,
        "openWorldHint": true,
    })
}

/// The schema of a run's `max_usd`, its spending cap.
fn max_usd_schema() -> Value {
    json!({
        "type": "number",
        "exclusiveMinimum": 0,
        "description": "The most the run may spend, in USD"
    })
}

/// A tool's spec: its name, description, argument schema and MCP
/// annotations, with `_meta` marking a tool that spends model budget.
fn tool(
    name: &str,
    description: &str,
    input_schema: Value,
    annotations: Value,
    paid: bool,
) -> Value {
    let mut spec = json!({
        "name": name,
        "description": description,
        "inputSchema": input_schema,
        "annotations": annotations,
    });
    if paid {
        spec["_meta"] = json!({ "roko/paid": true });
    }
    spec
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
        return Ok(tool_error(&format!(
            "the {name} tool needs the {scope} scope"
        )));
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
        "run_prompt" => {
            let prompt = required_str(&arguments, name, "prompt")?.to_string();
            let options = PromptPlanOptions {
                domain: optional_domain(&arguments)?,
                max_usd: Some(run_cap(state, &arguments)?),
                origin: mcp_origin(auth),
                ..PromptPlanOptions::default()
            };
            start_prompt_run(state, prompt, options).await
        }
        "plan_run" => {
            let plan_id = required_str(&arguments, name, "plan_id")?.to_string();
            let resume = optional_bool(&arguments, "resume")?;
            let max_usd = run_cap(state, &arguments)?;
            let origin = mcp_origin(auth);
            start_plan(state, plan_id, resume, origin, max_usd).await
        }
        "plan_generate" => {
            let prompt = required_str(&arguments, name, "prompt")?.to_string();
            Ok(generate_plan(state, prompt, &mcp_origin(auth)).await)
        }
        "run_cancel" => {
            let run_id = required_str(&arguments, name, "run_id")?;
            cancel_run(state, run_id).await
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

/// The non-blank string argument `name` of a call to the tool `tool`.
fn required_str<'a>(
    arguments: &'a Value,
    tool: &str,
    name: &str,
) -> Result<&'a str, JsonRpcError> {
    arguments
        .get(name)
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| JsonRpcError::invalid_params(format!("{tool} needs a {name}")))
}

/// The optional boolean argument `name`: false when it is absent.
fn optional_bool(arguments: &Value, name: &str) -> Result<bool, JsonRpcError> {
    match arguments.get(name) {
        None | Some(Value::Null) => Ok(false),
        Some(value) => value
            .as_bool()
            .ok_or_else(|| JsonRpcError::invalid_params(format!("{name} must be true or false"))),
    }
}

/// The optional `max_usd` argument: a spending cap in USD, above zero.
fn optional_usd(arguments: &Value) -> Result<Option<f64>, JsonRpcError> {
    match arguments.get("max_usd") {
        None | Some(Value::Null) => Ok(None),
        Some(value) => value
            .as_f64()
            .filter(|usd| usd.is_finite() && *usd > 0.0)
            .map(Some)
            .ok_or_else(|| JsonRpcError::invalid_params("max_usd must be a number above 0")),
    }
}

/// The spending cap a `run_prompt` or `plan_run` call names (9116): it must
/// name one, above 0 and at most `[serve.mcp] max_run_usd`, and it becomes
/// the run's budget ceiling.
fn run_cap(state: &AppState, arguments: &Value) -> Result<f64, JsonRpcError> {
    let most = state.load_roko_config().serve.mcp.max_run_usd;
    match optional_usd(arguments)? {
        None => Err(JsonRpcError::invalid_params(format!(
            "max_usd is required: name the most this run may spend, at most {most:.2} USD"
        ))),
        Some(cap) if cap > most => Err(JsonRpcError::invalid_params(format!(
            "max_usd {cap:.2} is above the {most:.2} USD a run started over /mcp may spend \
             ([serve.mcp] max_run_usd)"
        ))),
        Some(cap) => Ok(cap),
    }
}

/// The origin of a run a chat host starts over `/mcp` (9116): the calling
/// credential's name, or `local` when auth is off.
fn mcp_origin(auth: Option<&AuthContext>) -> RunOrigin {
    let client = auth
        .and_then(|context| context.user_id.clone())
        .unwrap_or_else(|| "local".to_string());
    RunOrigin::Mcp { client }
}

/// The optional `domain` argument, as the task domain its label names.
fn optional_domain(arguments: &Value) -> Result<Option<TaskDomain>, JsonRpcError> {
    match arguments.get("domain") {
        None | Some(Value::Null) => Ok(None),
        Some(value) => value
            .as_str()
            .map(TaskDomain::from_label)
            .ok_or_else(|| JsonRpcError::invalid_params("domain must be a string")),
    }
}

/// The `run_status` tool: the run's summary ([`run_or_operation`]) once its
/// state differs from its state when the call came, once it has ended, or
/// once `wait_secs` pass.
async fn run_status(
    state: &Arc<AppState>,
    run_id: &str,
    wait_secs: u64,
) -> Result<Value, ApiError> {
    let mut summary = run_or_operation(state, run_id).await?;
    let first_state = summary["state"].clone();
    let deadline = tokio::time::Instant::now() + Duration::from_secs(wait_secs);
    while summary["active"] == true
        && summary["state"] == first_state
        && tokio::time::Instant::now() < deadline
    {
        tokio::time::sleep(WAIT_POLL).await;
        summary = run_or_operation(state, run_id).await?;
    }
    Ok(summary)
}

/// The summary of run `id` ([`super::runs::summarize_run`]) or, for the run
/// id `plan_generate` answers with, its operation's state in the same words.
async fn run_or_operation(state: &Arc<AppState>, id: &str) -> Result<Value, ApiError> {
    match super::runs::summarize_run(state, id).await {
        Err(error) if error.status == StatusCode::NOT_FOUND => {
            operation_state(state, id).await.ok_or(error)
        }
        summary => summary,
    }
}

/// The state of operation `id` as `run_status` reports a run's: `running`,
/// then `succeeded` with its result or `failed` with its error.
async fn operation_state(state: &AppState, id: &str) -> Option<Value> {
    let operations = state.operations.read().await;
    let operation = operations.get(id)?;
    let (run_state, result, error) = match &operation.status {
        OperationStatus::Running => (RunState::Running, None, None),
        OperationStatus::Completed { result } => (RunState::Succeeded, result.as_deref(), None),
        OperationStatus::Failed { error } => (RunState::Failed, None, Some(error.as_str())),
    };
    let result = result.and_then(|result| serde_json::from_str::<Value>(result).ok());
    Some(json!({
        "run_id": id,
        "kind": operation.kind,
        "state": run_state.as_str(),
        "active": !run_state.is_terminal(),
        "result": result,
        "error": error,
    }))
}

/// Refuse a run a chat host starts while the workspace has no data-model
/// boundary and does not allow running without one (9117): the host reads
/// why, before anything is queued.
fn refuse_unscreened_run(state: &AppState) -> Result<(), ApiError> {
    match state.load_roko_config().chat_run_refusal() {
        Some(reason) => Err(ApiError::forbidden(reason)),
        None => Ok(()),
    }
}

/// The `run_prompt` tool: run `prompt` as a gated one-task plan, as `POST
/// /api/run` does (9113), and answer at once. Refused without the data-model
/// boundary ([`refuse_unscreened_run`]).
async fn start_prompt_run(
    state: &Arc<AppState>,
    prompt: String,
    options: PromptPlanOptions,
) -> Result<Value, ApiError> {
    refuse_unscreened_run(state)?;
    let run_id = super::run::start_gated_run(state, prompt, None, options).await?;
    Ok(json!({
        "run_id": run_id,
        "state": RunState::Running.as_str(),
        "links": {
            "summary": format!("/api/runs/{run_id}/summary"),
            "status": format!("/api/run/{run_id}/status"),
        },
    }))
}

/// The `plan_run` tool: run plan `plan_id` from `origin`, capped at
/// `max_usd`, or queue it behind the live run, as `POST
/// /api/plans/{id}/execute` does, and answer at once. Refused without the
/// data-model boundary ([`refuse_unscreened_run`]).
async fn start_plan(
    state: &Arc<AppState>,
    plan_id: String,
    resume: bool,
    origin: RunOrigin,
    max_usd: f64,
) -> Result<Value, ApiError> {
    refuse_unscreened_run(state)?;
    let started =
        super::plans::start_plan_run_with(state, plan_id, resume, origin, Some(max_usd)).await?;
    let run_id = started.run_id;
    let run_state = if started.queued.is_some() {
        RunState::Queued
    } else {
        RunState::Running
    };
    Ok(json!({
        "run_id": run_id,
        "state": run_state.as_str(),
        "position": started.queued,
        "links": {
            "summary": format!("/api/runs/{run_id}/summary"),
            "status": format!("/api/plans/{run_id}/status"),
        },
    }))
}

/// The `plan_generate` tool: have the planner write a plan for `prompt`, as
/// `POST /api/plans/generate` does, and answer at once with the operation's
/// id, which `run_status` follows, and the plan's id, which `plan_run` takes
/// once the plan is written. The planner gets `prompt` fenced as a request
/// from `origin`, a chat host (9117).
async fn generate_plan(state: &Arc<AppState>, prompt: String, origin: &RunOrigin) -> Value {
    let (operation_id, plan_id) = super::plans::start_plan_generation(state, prompt, origin).await;
    json!({
        "run_id": operation_id,
        "plan_id": plan_id,
        "state": RunState::Running.as_str(),
        "links": {
            "operation": format!("/api/operations/{operation_id}"),
            "plan": format!("/api/plans/{plan_id}"),
        },
    })
}

/// The `run_cancel` tool: stop run `run_id`, a gated prompt run or a live or
/// queued plan run, as `POST /api/plans/{id}/cancel` stops a plan run.
async fn cancel_run(state: &Arc<AppState>, run_id: &str) -> Result<Value, ApiError> {
    match super::run::cancel_background_run(state, run_id).await {
        Some(stopped) => stopped.map_err(ApiError::conflict)?,
        None => {
            super::plans::cancel_plan_run(state, run_id).await?;
        }
    }
    Ok(json!({ "run_id": run_id, "state": RunState::Cancelled.as_str() }))
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

    /// Auth off, as a local `roko.toml` can set it.
    fn no_auth() -> ServeAuthConfig {
        ServeAuthConfig {
            enabled: false,
            ..ServeAuthConfig::default()
        }
    }

    /// A workspace config with auth off, whose chat runs may start without
    /// the data-model boundary.
    fn open_config() -> RokoConfig {
        let mut config = RokoConfig::default();
        config.serve.auth = no_auth();
        config.serve.mcp.allow_without_data_llm = true;
        config
    }

    /// The server state and full router over `runtime` in a fresh workspace
    /// with `config`.
    fn state_and_router(
        runtime: Arc<dyn crate::runtime::CliRuntime>,
        config: RokoConfig,
    ) -> (tempfile::TempDir, Arc<AppState>, Router) {
        let dir = tempfile::tempdir().expect("tempdir");
        let auth = config.serve.auth.clone();
        let deploy_backend = Arc::from(
            crate::deploy::create_backend("manual", None, None, None).expect("manual backend"),
        );
        let state = Arc::new(
            AppState::new(dir.path().to_path_buf(), runtime, config, deploy_backend)
                .expect("AppState::new"),
        );
        let router = crate::routes::build_router(Arc::clone(&state), &[], auth);
        (dir, state, router)
    }

    /// The full router over a fresh workspace, with `auth`.
    fn router(auth: ServeAuthConfig) -> (tempfile::TempDir, Router) {
        let mut config = RokoConfig::default();
        config.serve.auth = auth;
        let runtime = Arc::new(crate::runtime::NoOpRuntime);
        let (dir, _state, router) = state_and_router(runtime, config);
        (dir, router)
    }

    /// A `tools/call` request for tool `name` with `arguments`.
    fn call(id: u64, name: &str, arguments: Value) -> Value {
        json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": "tools/call",
            "params": { "name": name, "arguments": arguments },
        })
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
    /// argument schema and `readOnlyHint`, and the run tools after them. With
    /// auth on, a call without a key is refused and a read-only key may call,
    /// but not a run tool; a web page on another origin is refused, and a
    /// notification gets 202.
    #[tokio::test]
    async fn mcp_tools_list_returns_annotated_tools() {
        let list = json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/list" });
        let (_dir, open) = router(no_auth());

        let (status, body) = post_mcp(&open, &list, &[]).await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(body["id"], 1, "{body}");
        let tools = body["result"]["tools"].as_array().expect("a tool list");
        let names: Vec<&str> = tools
            .iter()
            .map(|tool| tool["name"].as_str().expect("a tool name"))
            .collect();
        assert_eq!(
            names,
            [
                "run_status",
                "recall",
                "run_prompt",
                "plan_run",
                "plan_generate",
                "run_cancel"
            ]
        );
        for tool in &tools[..2] {
            assert_eq!(tool["annotations"]["readOnlyHint"], true, "{tool}");
        }
        for tool in tools {
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
        assert_eq!(body["result"]["tools"].as_array().map(Vec::len), Some(6));
        let cancel = call(2, "run_cancel", json!({ "run_id": "run-1" }));
        let (status, body) = post_mcp(&guarded, &cancel, &[("x-api-key", reader)]).await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(body["result"]["isError"], true, "{body}");
    }

    /// 9114: `tools/call` runs the tools. `recall` answers from the knowledge
    /// store; `run_status` of a run nobody knows is a tool error the host
    /// can read; a call without its required argument, or of an unknown
    /// tool, is a JSON-RPC error.
    #[tokio::test]
    async fn mcp_tools_call_runs_recall_and_run_status() {
        let (_dir, open) = router(no_auth());

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

    /// A runtime whose plan runs record their options, then wait for a
    /// permit from `gate` before they end, and whose prompt runs record their
    /// options and succeed at once.
    struct HeldPlans {
        plans: std::sync::Mutex<Vec<crate::runtime::PlanRunOptions>>,
        prompts: std::sync::Mutex<Vec<PromptPlanOptions>>,
        gate: tokio::sync::Semaphore,
    }

    impl HeldPlans {
        fn new() -> Self {
            Self {
                plans: std::sync::Mutex::default(),
                prompts: std::sync::Mutex::default(),
                gate: tokio::sync::Semaphore::new(0),
            }
        }

        /// The run ids the plan runs ran under, in the order they started.
        fn run_ids(&self) -> Vec<Option<String>> {
            let plans = self.plans.lock().expect("lock plan runs");
            plans.iter().map(|options| options.run_id.clone()).collect()
        }
    }

    #[async_trait::async_trait]
    impl crate::runtime::CliRuntime for HeldPlans {
        async fn run_once(
            &self,
            _workdir: &std::path::Path,
            _prompt: &str,
        ) -> anyhow::Result<crate::runtime::RunResult> {
            anyhow::bail!("HeldPlans only runs plans")
        }

        async fn load_plan_summary(
            &self,
            _workdir: &std::path::Path,
            plan_id: &str,
        ) -> anyhow::Result<Option<crate::plan_types::PlanSummaryDto>> {
            Ok(Some(crate::plan_types::PlanSummaryDto {
                id: plan_id.to_string(),
                title: "Held plan".to_string(),
                task_count: 1,
                tasks_done: 0,
                tasks_failed: 0,
                completed: false,
                status: "ready".to_string(),
                superseded_by: None,
                old_format: false,
                last_error: None,
                group: None,
                estimated_minutes: None,
            }))
        }

        async fn run_prompt_plan(
            &self,
            _workdir: &std::path::Path,
            _prompt: &str,
            options: PromptPlanOptions,
        ) -> anyhow::Result<crate::runtime::PromptPlanResult> {
            let run_id = options.run_id.clone().unwrap_or_default();
            self.prompts.lock().expect("lock prompt runs").push(options);
            Ok(crate::runtime::PromptPlanResult {
                run_id,
                verdict: RunState::Succeeded,
                success: true,
                output_text: None,
                cost_usd: None,
            })
        }

        async fn run_plan_with_options(
            &self,
            _workdir: &std::path::Path,
            _plan_target: &std::path::Path,
            options: crate::runtime::PlanRunOptions,
        ) -> anyhow::Result<crate::runtime::PlanExecutionResult> {
            self.plans.lock().expect("lock plan runs").push(options);
            self.gate
                .acquire()
                .await
                .expect("the gate stays open")
                .forget();
            Ok(crate::runtime::PlanExecutionResult {
                success: true,
                output_text: None,
                gate_results: Vec::new(),
            })
        }

        fn session_status(&self, workdir: std::path::PathBuf) -> crate::runtime::SessionStatusInfo {
            crate::runtime::SessionStatusInfo {
                session_id: None,
                workdir,
                daemon_running: false,
                signal_count: None,
                episode_count: None,
                last_episode_passed: None,
            }
        }

        fn dashboard_scaffold(&self, _workdir: &std::path::Path) -> crate::runtime::DashboardInfo {
            crate::runtime::DashboardInfo {
                rendered: String::new(),
            }
        }
    }

    /// 9115: `plan_run` answers at once with the id the engine runs the plan
    /// under. A second plan on the busy workspace is queued, and `run_cancel`
    /// takes the queued run out of the queue. The run tools are annotated so
    /// the host asks its user first, and the paid ones say so.
    #[tokio::test]
    async fn mcp_plan_run_returns_engine_run_id() {
        let runtime = Arc::new(HeldPlans::new());
        let (_dir, state, open) = state_and_router(runtime.clone(), open_config());

        let list = json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/list" });
        let (_, body) = post_mcp(&open, &list, &[]).await;
        let tools = body["result"]["tools"].as_array().expect("a tool list");
        let spec = |name: &str| {
            tools
                .iter()
                .find(|tool| tool["name"] == name)
                .expect("the tool is listed")
                .clone()
        };
        for name in ["run_prompt", "plan_run"] {
            let tool = spec(name);
            assert_eq!(tool["annotations"]["destructiveHint"], true, "{tool}");
            assert_eq!(tool["annotations"]["idempotentHint"], false, "{tool}");
            assert_eq!(tool["annotations"]["openWorldHint"], true, "{tool}");
            assert_eq!(tool["_meta"]["roko/paid"], true, "{tool}");
        }
        assert_eq!(spec("plan_generate")["annotations"]["destructiveHint"], false);
        assert_eq!(spec("run_cancel")["annotations"]["idempotentHint"], true);

        let alpha = call(2, "plan_run", json!({ "plan_id": "alpha", "max_usd": 1.0 }));
        let (_, body) = post_mcp(&open, &alpha, &[]).await;
        let first = &body["result"]["structuredContent"];
        assert_eq!(first["state"], "running", "{body}");
        let first_id = first["run_id"].as_str().expect("a run id").to_string();
        tokio::time::timeout(Duration::from_secs(5), async {
            while runtime.run_ids().is_empty() {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("the run starts");
        assert_eq!(runtime.run_ids(), [Some(first_id)]);

        let beta = call(3, "plan_run", json!({ "plan_id": "beta", "max_usd": 1.0 }));
        let (_, body) = post_mcp(&open, &beta, &[]).await;
        let second = &body["result"]["structuredContent"];
        assert_eq!(second["state"], "queued", "{body}");
        assert_eq!(second["position"], 1, "{body}");
        let second_id = second["run_id"].as_str().expect("a run id").to_string();

        let cancel = call(4, "run_cancel", json!({ "run_id": second_id }));
        let (_, body) = post_mcp(&open, &cancel, &[]).await;
        let cancelled = &body["result"]["structuredContent"];
        assert_eq!(cancelled["state"], "cancelled", "{body}");
        assert!(state.plan_queue.lock().expect("lock the queue").is_empty());
        runtime.gate.add_permits(1);
    }

    /// 9116: `run_prompt` and `plan_run` over `/mcp` must name a spending cap
    /// above 0 and at most `[serve.mcp] max_run_usd`. A call without one, or
    /// with one above the maximum, is refused before any run starts; a valid
    /// cap reaches the runtime's options, with the run's chat origin.
    #[tokio::test]
    async fn mcp_run_without_budget_cap_is_refused() {
        let runtime = Arc::new(HeldPlans::new());
        let mut config = open_config();
        config.serve.mcp.max_run_usd = 3.0;
        let (_dir, state, open) = state_and_router(runtime.clone(), config);

        let refused = [
            ("run_prompt", json!({ "prompt": "fix the parser" })),
            ("run_prompt", json!({ "prompt": "fix the parser", "max_usd": 0 })),
            ("run_prompt", json!({ "prompt": "fix the parser", "max_usd": 4.5 })),
            ("plan_run", json!({ "plan_id": "alpha" })),
            ("plan_run", json!({ "plan_id": "alpha", "max_usd": 9.0 })),
        ];
        for (id, (name, arguments)) in (1..).zip(refused) {
            let (_, body) = post_mcp(&open, &call(id, name, arguments), &[]).await;
            assert!(body["error"]["message"].is_string(), "{body}");
        }
        assert!(state.active_runs.read().await.is_empty());
        assert!(state.active_plans.read().await.is_empty());
        assert!(runtime.prompts.lock().expect("lock prompt runs").is_empty());

        let prompt = json!({ "prompt": "fix the parser", "max_usd": 2.5 });
        let (_, body) = post_mcp(&open, &call(6, "run_prompt", prompt), &[]).await;
        assert_eq!(body["result"]["isError"], false, "{body}");
        let plan = json!({ "plan_id": "alpha", "max_usd": 3.0 });
        let (_, body) = post_mcp(&open, &call(7, "plan_run", plan), &[]).await;
        assert_eq!(body["result"]["isError"], false, "{body}");
        tokio::time::timeout(Duration::from_secs(5), async {
            while runtime.prompts.lock().expect("lock prompt runs").is_empty()
                || runtime.run_ids().is_empty()
            {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("both runs start");

        let chat = RunOrigin::Mcp {
            client: "local".to_string(),
        };
        let prompts = runtime.prompts.lock().expect("lock prompt runs").clone();
        assert_eq!(prompts[0].max_usd, Some(2.5));
        assert_eq!(prompts[0].origin, chat);
        let plans = runtime.plans.lock().expect("lock plan runs").clone();
        assert_eq!(plans[0].max_usd, Some(3.0));
        assert_eq!(plans[0].origin, chat);
        runtime.gate.add_permits(1);
    }

    /// 9117: without the data-model boundary a chat host's run is refused,
    /// with a reason the host can read, and nothing starts; once
    /// `[agent.data_llm]` is set it starts.
    #[tokio::test]
    async fn mcp_run_without_data_model_boundary_is_refused() {
        let runtime = Arc::new(HeldPlans::new());
        let mut config = open_config();
        config.serve.mcp.allow_without_data_llm = false;
        let (_dir, state, open) = state_and_router(runtime.clone(), config.clone());

        let calls = [
            ("run_prompt", json!({ "prompt": "fix the parser", "max_usd": 1.0 })),
            ("plan_run", json!({ "plan_id": "alpha", "max_usd": 1.0 })),
        ];
        for (id, (name, arguments)) in (1..).zip(calls) {
            let (_, body) = post_mcp(&open, &call(id, name, arguments), &[]).await;
            assert_eq!(body["result"]["isError"], true, "{body}");
            let reason = body["result"]["content"][0]["text"].as_str().unwrap_or_default();
            assert!(reason.contains("[agent.data_llm]"), "{body}");
        }
        assert!(state.active_runs.read().await.is_empty());
        assert!(state.active_plans.read().await.is_empty());

        config.agent.data_llm = Some(roko_core::config::DataLlmConfig::default());
        state.store_roko_config(config);
        let prompt = json!({ "prompt": "fix the parser", "max_usd": 1.0 });
        let (_, body) = post_mcp(&open, &call(3, "run_prompt", prompt), &[]).await;
        assert_eq!(body["result"]["isError"], false, "{body}");
        runtime.gate.add_permits(1);
    }
}
