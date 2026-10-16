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
//! - `confirm_pending { run_id }` lists the outcomes a run's tasks wait for
//!   their person to confirm (a `confirm` rung, 9137), and `confirm_answer {
//!   run_id, task_id, attempt_key, approve, note }` records the person's
//!   answer in the review log, as the review route does.
//!
//! - `effects_pending { run_id }` lists the tool calls runs hold for
//!   approval (9131), with a summary of their arguments that leaves out the
//!   values, and `effect_decide { effect_id, approve, note }` approves one,
//!   which applies it once and checks its receipt, or rejects it (9138), as
//!   `/api/effects` does.
//!
//! `run_status`, `recall`, `confirm_pending` and `effects_pending` only
//! read. There is no `remember`: personal memory stays with the host (B9),
//! and serve has no knowledge write route (AD-10).

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
use crate::runtime::{EffectDecisionInput, PromptPlanOptions, RunOrigin};
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
                 result, or queue it behind the run in progress. Paid: it spends model \
                 budget, at most max_usd. Answers at once with the run's id and whether it \
                 is running or queued (with its position); follow it with run_status.",
                json!({
                    "type": "object",
                    "properties": {
                        "prompt": { "type": "string", "description": "The work to do" },
                        "title": title_schema(),
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
                        "prompt": { "type": "string", "description": "What the plan is for" },
                        "title": title_schema()
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
        (
            read_only_tool(
                "confirm_pending",
                "The outcomes a Roko run's tasks wait for their person to confirm: for each, \
                 the task, the attempt, the question to ask, a short summary of the work and \
                 when the rung stops waiting (deadline_at; the task then ends unverified). \
                 Ask the person, then answer with confirm_answer.",
                json!({
                    "type": "object",
                    "properties": {
                        "run_id": { "type": "string", "description": "The run's id" }
                    },
                    "required": ["run_id"],
                    "additionalProperties": false
                }),
            ),
            "read",
        ),
        (
            tool(
                "confirm_answer",
                "Give the person's own answer to a confirmation a Roko task waits for: approve \
                 true confirms the outcome, false fails the attempt with the note as its \
                 feedback. Never answer for the person.",
                json!({
                    "type": "object",
                    "properties": {
                        "run_id": { "type": "string", "description": "The run's id" },
                        "task_id": { "type": "string", "description": "The task's id" },
                        "attempt_key": {
                            "type": "string",
                            "description": "The attempt, as confirm_pending names it"
                        },
                        "approve": {
                            "type": "boolean",
                            "description": "Whether the person confirms the outcome"
                        },
                        "note": { "type": "string", "description": "What the person said" }
                    },
                    "required": ["run_id", "task_id", "attempt_key", "approve"],
                    "additionalProperties": false
                }),
                json!({
                    "readOnlyHint": false,
                    "destructiveHint": false,
                    "idempotentHint": true,
                    "openWorldHint": false,
                }),
                false,
            ),
            "write",
        ),
        (
            read_only_tool(
                "effects_pending",
                "The tool calls Roko runs hold for their person's approval instead of running \
                 them (sends, posts, payments): each with its tool, its MCP server and a \
                 summary of its arguments without their values, and the decisions made. \
                 Ask the person, then answer with effect_decide.",
                json!({
                    "type": "object",
                    "properties": {
                        "run_id": { "type": "string", "description": "Only this run's effects" }
                    },
                    "additionalProperties": false
                }),
            ),
            "read",
        ),
        (
            tool(
                "effect_decide",
                "Give the person's own decision on a held tool call: approve true runs it once \
                 and checks that its target shows the effect, false drops it. Answers the \
                 decision's record with the call's result and the receipt verdicts. Never \
                 decide for the person.",
                json!({
                    "type": "object",
                    "properties": {
                        "effect_id": {
                            "type": "string",
                            "description": "The effect, as effects_pending names it"
                        },
                        "approve": {
                            "type": "boolean",
                            "description": "Whether the person approves the call"
                        },
                        "note": { "type": "string", "description": "What the person said" }
                    },
                    "required": ["effect_id", "approve"],
                    "additionalProperties": false
                }),
                json!({
                    "readOnlyHint": false,
                    "destructiveHint": true,
                    "idempotentHint": false,
                    "openWorldHint": true,
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
/// The most characters of a run's title a tool takes (gap-c2efe7).
const MAX_TITLE_CHARS: usize = 120;

/// The `title` argument of the tools that start work (gap-c2efe7).
fn title_schema() -> Value {
    json!({
        "type": "string",
        "maxLength": MAX_TITLE_CHARS,
        "description": "A short name for the work, for people: the run's title, its branch's \
                        description and its commit's subject. Default: the request's first \
                        sentence"
    })
}

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
                title: optional_title(&arguments)?,
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
            let title = optional_title(&arguments)?;
            Ok(generate_plan(state, prompt, &mcp_origin(auth), title).await)
        }
        "run_cancel" => {
            let run_id = required_str(&arguments, name, "run_id")?;
            cancel_run(state, run_id).await
        }
        "confirm_pending" => {
            let run_id = required_str(&arguments, name, "run_id")?;
            Ok(json!({ "pending": confirm_holds(&state.workdir, run_id) }))
        }
        "confirm_answer" => {
            let run_id = required_str(&arguments, name, "run_id")?;
            let task_id = required_str(&arguments, name, "task_id")?;
            let attempt_key = required_str(&arguments, name, "attempt_key")?;
            let approve = arguments
                .get("approve")
                .and_then(Value::as_bool)
                .ok_or_else(|| JsonRpcError::invalid_params("approve must be true or false"))?;
            let note = arguments
                .get("note")
                .and_then(Value::as_str)
                .unwrap_or_default();
            let answer = ConfirmAnswer {
                run_id,
                task_id,
                attempt_key,
                approve,
                note,
            };
            confirm_answer(state, &answer).await
        }
        "effects_pending" => {
            let run_id = arguments.get("run_id").and_then(Value::as_str);
            super::effects::list_effects(state, run_id).await
        }
        "effect_decide" => {
            let effect_id = required_str(&arguments, name, "effect_id")?;
            let approve = arguments
                .get("approve")
                .and_then(Value::as_bool)
                .ok_or_else(|| JsonRpcError::invalid_params("approve must be true or false"))?;
            let decision = EffectDecisionInput {
                approve,
                note: arguments
                    .get("note")
                    .and_then(Value::as_str)
                    .map(str::to_string),
                decided_by: auth
                    .and_then(|context| context.user_id.clone())
                    .unwrap_or_else(|| "local-mcp".to_string()),
            };
            super::effects::decide_effect(state, effect_id, decision).await
        }
        _ => return Err(unknown()),
    };
    Ok(match outcome {
        Ok(value) => tool_result(&value),
        Err(error) => tool_failure(&error),
    })
}

/// The confirmations (9137) the tasks of run `run_id` wait for: their review
/// holds whose `kind` is `confirm`, oldest first, with what a host shows its
/// user, and how long the rung waits for an answer: `timeout_secs` and the
/// `deadline_at` it gives from `held_at` (gap-25d9cc).
fn confirm_holds(workdir: &std::path::Path, run_id: &str) -> Vec<Value> {
    let root = roko_fs::RokoLayout::for_project(workdir)
        .state_dir()
        .join("review-holds");
    let mut holds: Vec<Value> = std::fs::read_dir(root)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|plan| std::fs::read_dir(plan.path()).ok())
        .flatten()
        .flatten()
        .filter_map(|hold| serde_json::from_slice::<Value>(&std::fs::read(hold.path()).ok()?).ok())
        .filter(|hold| hold["kind"] == "confirm" && hold["run_id"] == run_id)
        .map(|hold| {
            json!({
                "plan_id": hold["plan_id"],
                "task_id": hold["task_id"],
                "title": hold["title"],
                "attempt_key": hold["attempt_key"],
                "question": hold["question"],
                "summary": hold["summary"],
                "held_at": hold["held_at"],
                "timeout_secs": hold["timeout_secs"],
                "deadline_at": confirm_deadline(&hold),
            })
        })
        .collect();
    holds.sort_by(|a, b| a["held_at"].as_str().cmp(&b["held_at"].as_str()));
    holds
}

/// When the confirm rung that wrote `hold` stops waiting and the task ends
/// unverified: its `held_at` plus its `timeout_secs`, in RFC 3339; null when
/// the hold names neither (gap-25d9cc).
fn confirm_deadline(hold: &Value) -> Value {
    let held_at = hold["held_at"]
        .as_str()
        .and_then(|at| chrono::DateTime::parse_from_rfc3339(at).ok());
    let timeout = hold["timeout_secs"]
        .as_u64()
        .and_then(|secs| i64::try_from(secs).ok())
        .and_then(chrono::TimeDelta::try_seconds);
    match (held_at, timeout) {
        (Some(held_at), Some(timeout)) => held_at
            .checked_add_signed(timeout)
            .map_or(Value::Null, |deadline| json!(deadline.to_rfc3339())),
        _ => Value::Null,
    }
}

/// A `confirm_answer` call's arguments.
struct ConfirmAnswer<'a> {
    run_id: &'a str,
    task_id: &'a str,
    attempt_key: &'a str,
    approve: bool,
    note: &'a str,
}

/// Record `answer` to the confirmation its task waits for (9137) in the
/// review log the held attempt reads, as the review route records a
/// decision. A task that waits for no such confirmation is an error.
async fn confirm_answer(
    state: &Arc<AppState>,
    answer: &ConfirmAnswer<'_>,
) -> Result<Value, ApiError> {
    let held = confirm_holds(&state.workdir, answer.run_id)
        .into_iter()
        .find(|hold| hold["task_id"] == answer.task_id && hold["attempt_key"] == answer.attempt_key)
        .ok_or_else(|| {
            ApiError::not_found(format!(
                "task {} of run {} waits for no confirmation of attempt {}",
                answer.task_id, answer.run_id, answer.attempt_key
            ))
        })?;
    let plan_id = held["plan_id"].as_str().unwrap_or_default();
    let decision = if answer.approve {
        "approved"
    } else {
        "rejected"
    };
    super::plans::record_review(
        &state.workdir,
        plan_id,
        answer.task_id,
        decision,
        answer.note,
        Some(answer.attempt_key),
    )
    .await;
    Ok(json!({
        "run_id": answer.run_id,
        "task_id": answer.task_id,
        "attempt_key": answer.attempt_key,
        "decision": decision,
    }))
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
fn required_str<'a>(arguments: &'a Value, tool: &str, name: &str) -> Result<&'a str, JsonRpcError> {
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
/// The `title` argument (gap-c2efe7), when given and not blank.
fn optional_title(arguments: &Value) -> Result<Option<String>, JsonRpcError> {
    match arguments.get("title") {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(title)) if title.chars().count() > MAX_TITLE_CHARS => {
            Err(JsonRpcError::invalid_params(format!(
                "title is longer than {MAX_TITLE_CHARS} characters"
            )))
        }
        Some(Value::String(title)) => {
            Ok(Some(title.trim().to_string()).filter(|title| !title.is_empty()))
        }
        Some(_) => Err(JsonRpcError::invalid_params("title must be a string")),
    }
}

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
/// then `succeeded` with its result or `failed` with its error. A running
/// operation says how long it has run, and one that reports progress
/// carries its latest report, and keeps it once it ends (gap-11d870).
async fn operation_state(state: &AppState, id: &str) -> Option<Value> {
    let operations = state.operations.read().await;
    let operation = operations.get(id)?;
    let (run_state, result, error) = match &operation.status {
        OperationStatus::Running => (RunState::Running, None, None),
        OperationStatus::Completed { result } => (RunState::Succeeded, result.as_deref(), None),
        OperationStatus::Failed { error } => (RunState::Failed, None, Some(error.as_str())),
    };
    let result = result.and_then(|result| serde_json::from_str::<Value>(result).ok());
    let mut summary = json!({
        "run_id": id,
        "kind": operation.kind,
        "state": run_state.as_str(),
        "active": !run_state.is_terminal(),
        "result": result,
        "error": error,
    });
    if !run_state.is_terminal() {
        summary["elapsed_secs"] = json!(operation.started.elapsed().as_secs());
    }
    if let Some(progress) = &operation.progress {
        let progress = progress.borrow().clone();
        if !progress.is_null() {
            summary["progress"] = progress;
        }
    }
    Some(summary)
}

/// Refuse a run a chat host starts while the workspace has no data-model
/// boundary and does not allow running without one (9117), while no model
/// its tasks can be routed to runs in roko's own tool loop (gap-cf427e), or
/// while none of those has a price its `max_usd` could count (gap-8548e3):
/// the host reads why, before anything is queued and before the plan is
/// looked up. The error's details carry the machine-readable codes,
/// `refusal` (the first) and `refusals` (all that apply).
fn refuse_unscreened_run(state: &AppState) -> Result<(), ApiError> {
    let config = state.load_roko_config();
    let prices =
        roko_core::pricing_snapshot::PriceSnapshot::shared(&config.pricing, &state.workdir);
    match config.chat_run_refusal(prices.as_deref()) {
        Some(refusal) => Err(ApiError {
            details: Some(Box::new(json!({
                "refusal": refusal.code(),
                "refusals": refusal.codes(),
            }))),
            ..ApiError::forbidden(refusal.message)
        }),
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
    let started = super::run::start_gated_run(state, prompt, None, options).await?;
    let run_id = started.run_id;
    // While another run holds the workspace, the run waits in the run queue
    // (gap-9a8d66).
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
async fn generate_plan(
    state: &Arc<AppState>,
    prompt: String,
    origin: &RunOrigin,
    title: Option<String>,
) -> Value {
    let (operation_id, plan_id) =
        super::plans::start_plan_generation(state, prompt, origin, title).await;
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
/// queued plan run, as `POST /api/plans/{id}/cancel` stops a plan run. A run
/// whose roko process stopped while it ran is over already: the answer says
/// `interrupted` and why (bug-dceafd).
async fn cancel_run(state: &Arc<AppState>, run_id: &str) -> Result<Value, ApiError> {
    let mut answer = json!({ "run_id": run_id, "state": RunState::Cancelled.as_str() });
    match super::run::cancel_background_run(state, run_id).await {
        Some(stopped) => stopped.map_err(ApiError::conflict)?,
        // A prompt run still waiting in the run queue (gap-9a8d66).
        None if super::run::cancel_queued_prompt_run(state, run_id).await => {}
        None => {
            let cancelled = super::plans::cancel_plan_run(state, run_id).await?;
            for field in ["state", "reason"] {
                if let Some(value) = cancelled.get(field) {
                    answer[field] = value.clone();
                }
            }
        }
    }
    Ok(answer)
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

/// The failure `error` of a tool, for the calling model to read, and for a
/// host as `structuredContent.error`: its `code` (`not_found`, `forbidden`,
/// …), its `message`, and its machine-readable `data` when it has some, such
/// as a refused chat run's `refusal` (gap-cf427e).
fn tool_failure(error: &ApiError) -> Value {
    let mut failure = tool_error(&error.message);
    let mut detail = json!({ "code": error.code, "message": error.message });
    if let Some(data) = error.details.as_deref() {
        detail["data"] = data.clone();
    }
    failure["structuredContent"] = json!({ "error": detail });
    failure
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
    /// the data-model boundary, and whose tasks run on a model roko's own
    /// tool loop runs (gap-cf427e).
    fn open_config() -> RokoConfig {
        let mut config = cli_only_config();
        config.serve.mcp.allow_without_data_llm = true;
        add_tool_loop_model(&mut config);
        config
    }

    /// A workspace config with auth off whose only provider is the Claude
    /// CLI, which every model routing names runs on. The providers keys in
    /// the environment would synthesize are configured without a key, so
    /// none of them can run a task.
    fn cli_only_config() -> RokoConfig {
        use roko_core::agent::ProviderKind;
        use roko_core::config::schema::{ModelProfile, ProviderConfig};

        let mut config = RokoConfig::default();
        config.serve.auth = no_auth();
        config.providers.clear();
        config.models.clear();
        config.routing.fallback_models.clear();
        config.agent.fallback_model = None;
        config.agent.default_model = "cli-sonnet".to_string();
        config.providers.insert(
            "claude_cli".to_string(),
            ProviderConfig {
                kind: ProviderKind::ClaudeCli,
                command: Some("claude".to_string()),
                ..ProviderConfig::default()
            },
        );
        for (id, kind) in [
            ("anthropic", ProviderKind::AnthropicApi),
            ("openai", ProviderKind::OpenAiCompat),
            ("gemini", ProviderKind::GeminiApi),
            ("perplexity", ProviderKind::PerplexityApi),
        ] {
            let keyless = ProviderConfig {
                kind,
                api_key_env: Some("ROKO_TEST_MCP_KEY_NEVER_SET".to_string()),
                ..ProviderConfig::default()
            };
            config.providers.insert(id.to_string(), keyless);
        }
        config.models.insert(
            "cli-sonnet".to_string(),
            ModelProfile {
                provider: "claude_cli".to_string(),
                slug: "claude-sonnet-4-6".to_string(),
                supports_tools: true,
                ..ModelProfile::default()
            },
        );
        config
    }

    /// Add `api-model`, on an OpenAI-compatible API whose key `config`'s
    /// `[agent] env` sets, so roko's own tool loop can run it, with a price, so
    /// a chat run's `max_usd` can count its calls (gap-8548e3).
    fn add_tool_loop_model(config: &mut RokoConfig) {
        use roko_core::agent::ProviderKind;
        use roko_core::config::schema::{ModelProfile, ProviderConfig};

        config.providers.insert(
            "api".to_string(),
            ProviderConfig {
                kind: ProviderKind::OpenAiCompat,
                base_url: Some("http://127.0.0.1:9/v1".to_string()),
                api_key_env: Some("ROKO_TEST_MCP_API_KEY".to_string()),
                ..ProviderConfig::default()
            },
        );
        config.agent.env = Some(vec![(
            "ROKO_TEST_MCP_API_KEY".to_string(),
            "test-key".to_string(),
        )]);
        config.models.insert(
            "api-model".to_string(),
            ModelProfile {
                provider: "api".to_string(),
                slug: "api-model-1".to_string(),
                supports_tools: true,
                cost_input_per_m: Some(1.0),
                cost_output_per_m: Some(2.0),
                ..ModelProfile::default()
            },
        );
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
                "run_cancel",
                "confirm_pending",
                "confirm_answer",
                "effects_pending",
                "effect_decide"
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
        assert_eq!(body["result"]["tools"].as_array().map(Vec::len), Some(10));
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

    /// gap-c2efe7: `run_prompt` and `plan_generate` take an optional `title`, which a
    /// prompt run's options carry; a title over 120 characters, or one that
    /// is not a string, is refused.
    #[tokio::test]
    async fn mcp_run_prompt_takes_a_title() {
        let runtime = Arc::new(HeldPlans::new());
        let (_dir, _state, open) = state_and_router(runtime.clone(), open_config());

        let list = json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/list" });
        let (_, body) = post_mcp(&open, &list, &[]).await;
        let tools = body["result"]["tools"].as_array().expect("a tool list");
        for name in ["run_prompt", "plan_generate"] {
            let tool = tools
                .iter()
                .find(|tool| tool["name"] == name)
                .expect("the tool is listed");
            assert_eq!(
                tool["inputSchema"]["properties"]["title"]["type"], "string",
                "{tool}"
            );
        }

        let named = call(
            2,
            "run_prompt",
            json!({ "prompt": "Add a flag. Then a test.", "title": " Version flag ", "max_usd": 1.0 }),
        );
        let (_, body) = post_mcp(&open, &named, &[]).await;
        assert!(
            body["result"]["structuredContent"]["run_id"].is_string(),
            "{body}"
        );
        eventually("the prompt run starts", || {
            !runtime.prompts.lock().expect("lock prompt runs").is_empty()
        })
        .await;
        let title = runtime.prompts.lock().expect("lock prompt runs")[0]
            .title
            .clone();
        assert_eq!(title.as_deref(), Some("Version flag"));

        for (id, title) in [(3, json!("x".repeat(121))), (4, json!(7))] {
            let refused = call(
                id,
                "run_prompt",
                json!({ "prompt": "Add a flag.", "title": title, "max_usd": 1.0 }),
            );
            let (_, body) = post_mcp(&open, &refused, &[]).await;
            assert!(
                body["error"]["message"]
                    .as_str()
                    .is_some_and(|m| m.contains("title")),
                "{body}"
            );
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
        assert_eq!(
            spec("plan_generate")["annotations"]["destructiveHint"],
            false
        );
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

    /// A runtime whose prompt runs record their options, then wait for a
    /// permit from `gate` before they succeed; whose plan runs record theirs
    /// and succeed at once; and that reports `holder` as holding the
    /// workspace's runner lock outside the server.
    struct HeldPrompts {
        prompts: std::sync::Mutex<Vec<PromptPlanOptions>>,
        plans: std::sync::Mutex<Vec<crate::runtime::PlanRunOptions>>,
        gate: tokio::sync::Semaphore,
        holder: std::sync::Mutex<Option<String>>,
    }

    impl HeldPrompts {
        fn new() -> Self {
            Self {
                prompts: std::sync::Mutex::default(),
                plans: std::sync::Mutex::default(),
                gate: tokio::sync::Semaphore::new(0),
                holder: std::sync::Mutex::default(),
            }
        }

        /// The run ids the prompt runs ran under, in the order they started.
        fn prompt_ids(&self) -> Vec<String> {
            let prompts = self.prompts.lock().expect("lock prompt runs");
            prompts
                .iter()
                .map(|options| options.run_id.clone().unwrap_or_default())
                .collect()
        }

        fn plan_ids(&self) -> Vec<String> {
            let plans = self.plans.lock().expect("lock plan runs");
            plans
                .iter()
                .map(|options| options.run_id.clone().unwrap_or_default())
                .collect()
        }
    }

    #[async_trait::async_trait]
    impl crate::runtime::CliRuntime for HeldPrompts {
        async fn run_once(
            &self,
            _workdir: &std::path::Path,
            _prompt: &str,
        ) -> anyhow::Result<crate::runtime::RunResult> {
            anyhow::bail!("HeldPrompts runs gated prompts and plans only")
        }

        async fn load_plan_summary(
            &self,
            workdir: &std::path::Path,
            plan_id: &str,
        ) -> anyhow::Result<Option<crate::plan_types::PlanSummaryDto>> {
            crate::runtime::CliRuntime::load_plan_summary(&HeldPlans::new(), workdir, plan_id).await
        }

        async fn run_prompt_plan(
            &self,
            _workdir: &std::path::Path,
            _prompt: &str,
            options: PromptPlanOptions,
        ) -> anyhow::Result<crate::runtime::PromptPlanResult> {
            let run_id = options.run_id.clone().unwrap_or_default();
            self.prompts.lock().expect("lock prompt runs").push(options);
            self.gate
                .acquire()
                .await
                .expect("the gate stays open")
                .forget();
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
            Ok(crate::runtime::PlanExecutionResult {
                success: true,
                output_text: None,
                gate_results: Vec::new(),
            })
        }

        fn runner_lock_holder(&self, _workdir: &std::path::Path) -> Option<String> {
            self.holder.lock().expect("lock the holder").clone()
        }

        fn session_status(&self, workdir: std::path::PathBuf) -> crate::runtime::SessionStatusInfo {
            crate::runtime::CliRuntime::session_status(&HeldPlans::new(), workdir)
        }

        fn dashboard_scaffold(&self, workdir: &std::path::Path) -> crate::runtime::DashboardInfo {
            crate::runtime::CliRuntime::dashboard_scaffold(&HeldPlans::new(), workdir)
        }
    }

    /// Wait until `ready` holds, for up to 5 s.
    async fn eventually(what: &str, ready: impl Fn() -> bool) {
        tokio::time::timeout(Duration::from_secs(5), async {
            while !ready() {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap_or_else(|_| panic!("{what}"));
    }

    /// gap-9a8d66, bug-09e21e: a second `run_prompt` while a run holds the
    /// workspace waits in the run queue, which it shares with plan runs, and
    /// starts when the runs before it end; it never fails because the
    /// workspace is busy after its id was handed out. `run_status` answers
    /// for a queued run, `run_cancel` takes one out of the queue and its id
    /// still answers, and a run outside the server holding the workspace
    /// refuses a prompt run before it gets an id.
    #[tokio::test]
    async fn a_second_prompt_run_waits_or_is_refused_up_front() {
        let runtime = Arc::new(HeldPrompts::new());
        let (_dir, state, open) = state_and_router(runtime.clone(), open_config());
        let prompt =
            |id: u64, text: &str| call(id, "run_prompt", json!({ "prompt": text, "max_usd": 1.0 }));
        let structured = |body: &Value| body["result"]["structuredContent"].clone();

        let (_, body) = post_mcp(&open, &prompt(1, "add a --json flag"), &[]).await;
        let first = structured(&body);
        assert_eq!(first["state"], "running", "{body}");
        assert!(first["position"].is_null(), "{body}");
        let first_id = first["run_id"].as_str().expect("a run id").to_string();
        eventually("the first run starts", || runtime.prompt_ids().len() == 1).await;

        let (_, body) = post_mcp(&open, &prompt(2, "add a categories command"), &[]).await;
        let second = structured(&body);
        assert_eq!(second["state"], "queued", "{body}");
        assert_eq!(second["position"], 1, "{body}");
        let second_id = second["run_id"].as_str().expect("a run id").to_string();
        let status = call(3, "run_status", json!({ "run_id": second_id }));
        let (_, body) = post_mcp(&open, &status, &[]).await;
        assert_eq!(structured(&body)["state"], "queued", "{body}");
        assert_eq!(structured(&body)["position"], 1, "{body}");

        // Plan runs wait in the same order.
        let plan = call(4, "plan_run", json!({ "plan_id": "alpha", "max_usd": 1.0 }));
        let (_, body) = post_mcp(&open, &plan, &[]).await;
        assert_eq!(structured(&body)["state"], "queued", "{body}");
        assert_eq!(structured(&body)["position"], 2, "{body}");
        let plan_id = structured(&body)["run_id"]
            .as_str()
            .expect("a run id")
            .to_string();

        let (_, body) = post_mcp(&open, &prompt(5, "add a report"), &[]).await;
        assert_eq!(structured(&body)["position"], 3, "{body}");
        let third_id = structured(&body)["run_id"]
            .as_str()
            .expect("a run id")
            .to_string();
        let cancel = call(6, "run_cancel", json!({ "run_id": third_id }));
        let (_, body) = post_mcp(&open, &cancel, &[]).await;
        assert_eq!(structured(&body)["state"], "cancelled", "{body}");
        let status = call(7, "run_status", json!({ "run_id": third_id }));
        let (_, body) = post_mcp(&open, &status, &[]).await;
        assert_eq!(structured(&body)["state"], "cancelled", "{body}");
        assert_eq!(state.queued_runs(), 2);

        // The first run ends: the oldest queued run, the second prompt, starts
        // under the id it was handed; the plan run starts after it.
        runtime.gate.add_permits(1);
        eventually("the second run starts", || runtime.prompt_ids().len() == 2).await;
        assert_eq!(runtime.prompt_ids(), [first_id.clone(), second_id.clone()]);
        assert!(
            runtime.plan_ids().is_empty(),
            "the plan run waits for the second prompt"
        );
        runtime.gate.add_permits(1);
        eventually("the plan run starts", || runtime.plan_ids().len() == 1).await;
        assert_eq!(runtime.plan_ids(), [plan_id]);
        for run_id in [&first_id, &second_id] {
            let status = call(8, "run_status", json!({ "run_id": run_id }));
            let (_, body) = post_mcp(&open, &status, &[]).await;
            assert_eq!(structured(&body)["state"], "succeeded", "{body}");
        }

        // A run outside the server holds the workspace: refused before any id.
        tokio::time::timeout(Duration::from_secs(5), async {
            while state.live_plan_runs().await > 0 || state.queued_runs() > 0 {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("the plan run ends");
        *runtime.holder.lock().expect("lock the holder") =
            Some("a roko plan runner (PID 42)".to_string());
        let (_, body) = post_mcp(&open, &prompt(9, "add a flag"), &[]).await;
        assert_eq!(body["result"]["isError"], true, "{body}");
        let reason = body["result"]["content"][0]["text"]
            .as_str()
            .unwrap_or_default();
        assert!(
            reason.contains("outside this server") && reason.contains("PID 42"),
            "{body}"
        );
        assert_eq!(runtime.prompt_ids().len(), 2);
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
            (
                "run_prompt",
                json!({ "prompt": "fix the parser", "max_usd": 0 }),
            ),
            (
                "run_prompt",
                json!({ "prompt": "fix the parser", "max_usd": 4.5 }),
            ),
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

    /// gap-5fda00: a prompt run the `run_prompt` tool starts publishes the
    /// live agent output the server's plan runs do: the runtime gets the
    /// server's effective setting.
    #[tokio::test]
    async fn prompt_runs_take_the_live_output_setting() {
        use roko_core::config::serve::LiveAgentOutput;

        let runtime = Arc::new(HeldPlans::new());
        let mut config = open_config();
        config.serve.mcp.max_run_usd = 3.0;
        let (_dir, state, open) = state_and_router(runtime.clone(), config);
        state.configure_live_agent_output(LiveAgentOutput::Trusted);

        let prompt = json!({ "prompt": "fix the parser", "max_usd": 1.0 });
        let (_, body) = post_mcp(&open, &call(1, "run_prompt", prompt), &[]).await;
        assert_eq!(body["result"]["isError"], false, "{body}");
        tokio::time::timeout(Duration::from_secs(5), async {
            while runtime.prompts.lock().expect("lock prompt runs").is_empty() {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("the run starts");

        let prompts = runtime.prompts.lock().expect("lock prompt runs").clone();
        assert_eq!(prompts[0].live_agent_output, Some(LiveAgentOutput::Trusted));
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
            (
                "run_prompt",
                json!({ "prompt": "fix the parser", "max_usd": 1.0 }),
            ),
            ("plan_run", json!({ "plan_id": "alpha", "max_usd": 1.0 })),
        ];
        for (id, (name, arguments)) in (1..).zip(calls) {
            let (_, body) = post_mcp(&open, &call(id, name, arguments), &[]).await;
            assert_eq!(body["result"]["isError"], true, "{body}");
            let reason = body["result"]["content"][0]["text"]
                .as_str()
                .unwrap_or_default();
            assert!(reason.contains("[agent.data_llm]"), "{body}");
        }
        assert!(state.active_runs.read().await.is_empty());
        assert!(state.active_plans.read().await.is_empty());

        // A data model roko calls over an API, with its key set.
        config.agent.data_llm = Some(roko_core::config::DataLlmConfig {
            model: "api-model".to_string(),
            ..roko_core::config::DataLlmConfig::default()
        });
        state.store_roko_config(config);
        let prompt = json!({ "prompt": "fix the parser", "max_usd": 1.0 });
        let (_, body) = post_mcp(&open, &call(3, "run_prompt", prompt), &[]).await;
        assert_eq!(body["result"]["isError"], false, "{body}");
        runtime.gate.add_permits(1);
    }

    /// gap-8548e3: a chat host's run is held to its `max_usd`, so it is
    /// refused before anything is queued while no model it can run on has a
    /// price, with `no_price` as its machine-readable refusal, and runs once
    /// one has.
    #[tokio::test]
    async fn chat_run_refused_without_a_priced_model() {
        let mut config = cli_only_config();
        config.serve.mcp.allow_without_data_llm = true;
        add_tool_loop_model(&mut config);
        let unpriced = config.models.get_mut("api-model").expect("api-model");
        unpriced.cost_input_per_m = None;
        unpriced.cost_output_per_m = None;
        let runtime = Arc::new(crate::runtime::NoOpRuntime);
        let (_dir, state, open) = state_and_router(runtime, config.clone());

        let probe = json!({ "plan_id": "roko-doctor-probe-1a2b", "max_usd": 0.01 });
        let (_, body) = post_mcp(&open, &call(1, "plan_run", probe.clone()), &[]).await;
        let error = &body["result"]["structuredContent"]["error"];
        assert_eq!(error["code"], "forbidden", "{body}");
        assert_eq!(error["data"]["refusal"], "no_price", "{body}");
        assert_eq!(error["data"]["refusals"], json!(["no_price"]));
        let text = body["result"]["content"][0]["text"]
            .as_str()
            .unwrap_or_default();
        assert!(text.contains("(api-model)"), "{text}");
        assert!(text.contains("cost_input_per_m"), "{text}");
        assert!(state.active_plans.read().await.is_empty());

        add_tool_loop_model(&mut config);
        state.store_roko_config(config);
        let (_, body) = post_mcp(&open, &call(2, "plan_run", probe), &[]).await;
        let error = &body["result"]["structuredContent"]["error"];
        assert_eq!(error["code"], "not_found", "{body}");
    }

    /// gap-cf427e: a chat host's run stages its outbound tool calls, which a
    /// CLI agent cannot, so a workspace whose only provider is a CLI agent
    /// refuses `run_prompt` and `plan_run` before anything is queued, with
    /// the machine-readable `refusal` under `structuredContent.error.data`.
    /// The doctor's probe, a `plan_run` of a plan that does not exist, gets
    /// the refusal first, and "not found" once a model roko's own tool loop
    /// runs is configured.
    #[tokio::test]
    async fn chat_run_refused_without_a_tool_loop_model() {
        let mut config = cli_only_config();
        config.serve.mcp.allow_without_data_llm = true;
        let runtime = Arc::new(crate::runtime::NoOpRuntime);
        let (_dir, state, open) = state_and_router(runtime, config.clone());

        let calls = [
            (
                "run_prompt",
                json!({ "prompt": "fix the parser", "max_usd": 1.0 }),
            ),
            (
                "plan_run",
                json!({ "plan_id": "roko-doctor-probe-1a2b", "max_usd": 0.01 }),
            ),
        ];
        for (id, (name, arguments)) in (1..).zip(calls.clone()) {
            let (_, body) = post_mcp(&open, &call(id, name, arguments), &[]).await;
            let result = &body["result"];
            assert_eq!(result["isError"], true, "{body}");
            let error = &result["structuredContent"]["error"];
            assert_eq!(error["code"], "forbidden", "{body}");
            assert_eq!(error["data"]["refusal"], "no_tool_loop_model", "{body}");
            assert_eq!(error["data"]["refusals"], json!(["no_tool_loop_model"]));
            let text = result["content"][0]["text"].as_str().unwrap_or_default();
            assert_eq!(error["message"], text, "{body}");
            assert!(text.contains("roko's own tool loop"), "{text}");
            assert!(
                text.contains("runs on claude_cli, an agent that runs its own tools"),
                "{text}"
            );
            assert!(text.contains("has no [models.*] entry"), "{text}");
            assert!(!text.to_lowercase().contains("not found"), "{text}");
            assert!(!text.to_lowercase().starts_with("no "), "{text}");
        }
        assert!(state.active_runs.read().await.is_empty());
        assert!(state.active_plans.read().await.is_empty());

        // Without the boundary as well, both blockers apply, the boundary
        // first and in the words older hosts match.
        let mut unscreened = config.clone();
        unscreened.serve.mcp.allow_without_data_llm = false;
        state.store_roko_config(unscreened);
        let (_, body) = post_mcp(&open, &call(3, calls[1].0, calls[1].1.clone()), &[]).await;
        let data = &body["result"]["structuredContent"]["error"]["data"];
        assert_eq!(data["refusal"], "no_data_boundary", "{body}");
        assert_eq!(
            data["refusals"],
            json!(["no_data_boundary", "no_tool_loop_model"])
        );
        let text = body["result"]["content"][0]["text"]
            .as_str()
            .unwrap_or_default();
        assert!(text.contains("data-model boundary"), "{text}");

        // With a model roko's own tool loop runs, the probe's plan is simply
        // not found.
        add_tool_loop_model(&mut config);
        state.store_roko_config(config);
        let (_, body) = post_mcp(&open, &call(4, calls[1].0, calls[1].1.clone()), &[]).await;
        assert_eq!(body["result"]["isError"], true, "{body}");
        let error = &body["result"]["structuredContent"]["error"];
        assert_eq!(error["code"], "not_found", "{body}");
        assert!(error.get("data").is_none(), "{body}");
        let text = body["result"]["content"][0]["text"]
            .as_str()
            .unwrap_or_default();
        assert!(text.contains("not found"), "{text}");
    }

    /// gap-25d9cc: `confirm_pending` lists each confirmation with the rung's
    /// `timeout_secs` and the `deadline_at` it gives from `held_at`, so a host
    /// can say how long its person has before the task ends unverified; a hold
    /// without a timeout has no deadline.
    #[tokio::test]
    async fn confirm_pending_reports_the_rungs_deadline() {
        let (dir, open) = router(no_auth());
        let holds = roko_fs::RokoLayout::for_project(dir.path())
            .state_dir()
            .join("review-holds")
            .join("p1");
        std::fs::create_dir_all(&holds).expect("the holds directory");
        let hold = |task: &str, timeout: Value| {
            json!({
                "schema_version": 1,
                "kind": "confirm",
                "plan_id": "p1",
                "task_id": task,
                "title": "Write the report",
                "run_id": "run-1",
                "attempt_key": format!("run-1:p1:{task}:1"),
                "question": "Is this what you asked for?",
                "summary": "REPORT.md: …",
                "timeout_secs": timeout,
                "held_at": "2026-10-13T14:28:53.126917+00:00",
            })
        };
        for (task, timeout) in [("T1", json!(1800)), ("T2", Value::Null)] {
            std::fs::write(
                holds.join(format!("{task}.json")),
                hold(task, timeout).to_string(),
            )
            .expect("write the hold");
        }

        let pending = call(1, "confirm_pending", json!({ "run_id": "run-1" }));
        let (_, body) = post_mcp(&open, &pending, &[]).await;
        let items = body["result"]["structuredContent"]["pending"]
            .as_array()
            .expect("the pending list")
            .clone();
        let item = |task: &str| {
            items
                .iter()
                .find(|item| item["task_id"] == task)
                .expect("the task's hold")
                .clone()
        };
        let timed = item("T1");
        assert_eq!(timed["timeout_secs"], 1800, "{body}");
        assert_eq!(
            timed["deadline_at"], "2026-10-13T14:58:53.126917+00:00",
            "{body}"
        );
        let untimed = item("T2");
        assert!(untimed["deadline_at"].is_null(), "{body}");
    }

    /// A runtime that holds one staged effect, `effect-1`, until it is
    /// decided, and counts how often it applied it.
    #[derive(Default)]
    struct StagedEffects {
        applied: std::sync::Mutex<usize>,
    }

    #[async_trait::async_trait]
    impl crate::runtime::CliRuntime for StagedEffects {
        async fn run_once(
            &self,
            _workdir: &std::path::Path,
            _prompt: &str,
        ) -> anyhow::Result<crate::runtime::RunResult> {
            anyhow::bail!("not used")
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

        async fn list_effects(
            &self,
            _workdir: &std::path::Path,
            _run_id: Option<&str>,
        ) -> anyhow::Result<Value> {
            let waiting = if *self.applied.lock().expect("applied") == 0 {
                json!([{
                    "effect_id": "effect-1",
                    "tool": "mail.send",
                    "server": "mail",
                    "arguments": { "to": "string (15 chars)" },
                }])
            } else {
                json!([])
            };
            Ok(json!({ "waiting": waiting, "decided": [] }))
        }

        async fn decide_effect(
            &self,
            _workdir: &std::path::Path,
            effect_id: &str,
            decision: EffectDecisionInput,
        ) -> Result<Value, crate::runtime::EffectDecisionError> {
            let mut applied = self.applied.lock().expect("applied");
            if *applied > 0 {
                return Err(crate::runtime::EffectDecisionError::AlreadyDecided(
                    effect_id.to_string(),
                    "applied".to_string(),
                ));
            }
            *applied += usize::from(decision.approve);
            Ok(json!({
                "effect_id": effect_id,
                "outcome": "applied",
                "decided_by": decision.decided_by,
                "receipts": [{ "rung": "delivered", "passed": true }],
            }))
        }
    }

    /// 9138: a staged effect appears in `effects_pending` with its tool and
    /// a summary of its arguments, and `effect_decide` approves it once,
    /// answering the record with its receipt; a second decision is the
    /// tool's error, and the runtime applied the effect once.
    #[tokio::test]
    async fn mcp_effects_pending_lists_staged_effects() {
        let runtime = Arc::new(StagedEffects::default());
        let shared: Arc<dyn crate::runtime::CliRuntime> = runtime.clone();
        let (_dir, _state, router) = state_and_router(shared, open_config());

        let (status, body) = post_mcp(&router, &call(1, "effects_pending", json!({})), &[]).await;
        assert_eq!(status, StatusCode::OK, "{body}");
        let waiting = &body["result"]["structuredContent"]["waiting"];
        assert_eq!(waiting[0]["effect_id"], "effect-1", "{body}");
        assert_eq!(waiting[0]["tool"], "mail.send", "{body}");

        let decide = json!({ "effect_id": "effect-1", "approve": true, "note": "go ahead" });
        let (_, body) = post_mcp(&router, &call(2, "effect_decide", decide.clone()), &[]).await;
        let record = &body["result"]["structuredContent"];
        assert_eq!(record["outcome"], "applied", "{body}");
        assert_eq!(record["receipts"][0]["passed"], true, "{body}");
        assert_eq!(record["decided_by"], "local-mcp", "{body}");
        let (_, body) = post_mcp(&router, &call(3, "effect_decide", decide), &[]).await;
        assert_eq!(body["result"]["isError"], true, "{body}");
        assert_eq!(*runtime.applied.lock().expect("applied"), 1);
    }
}
