//! Prompt dispatch and backend HTTP communication.

use std::time::Duration;

use anyhow::{Context as _, Result, bail};
use serde::Deserialize;
use serde_json::json;

use super::decompose::{SubTask, decompose_message, merge_results};
use super::session::{clone_chat_agent_session, turn_result_to_dispatch_result};
use super::types::{ChatInlineDispatchError, ChatSession, DispatchMode};
use crate::chat::extract_clean_text;
use crate::dispatch_v2::DispatchResult;

use roko_learn::cost_table::CostTable;

// ---------------------------------------------------------------------------
// Prompt dispatch
// ---------------------------------------------------------------------------

/// Spawn async dispatch for a prompt, setting up the response channel.
pub(crate) fn dispatch_prompt(session: &mut ChatSession, prompt: &str) {
    use crate::inline::primitives::StreamingState;
    session.streaming = StreamingState::new("resolving...");
    let (tx, rx) = tokio::sync::mpsc::channel(1);
    session.response_rx = Some(rx);
    let text = prompt.to_string();

    match &session.dispatch {
        DispatchMode::Http {
            client,
            backend_url,
            is_sidecar,
            ..
        } => {
            let client_clone = client.clone();
            let url_owned = backend_url.clone();
            let agent_id_owned = session.agent_id.clone();
            let sidecar = *is_sidecar;
            tokio::spawn(async move {
                let result =
                    send_and_receive(&client_clone, &url_owned, &agent_id_owned, &text, sidecar)
                        .await;
                let _ = tx
                    .send(result.map(|r| r.into()).map_err(|e| e.to_string()))
                    .await;
            });
        }
        DispatchMode::Direct { .. } => {
            let _ = tx.try_send(Err(
                ChatInlineDispatchError::DirectDispatchDisabled.to_string()
            ));
        }
        DispatchMode::Session => {
            let Some(agent_session) = session.agent_session.as_ref() else {
                let _ = tx.try_send(Err("agent session unavailable".to_string()));
                return;
            };

            let mut agent_session = clone_chat_agent_session(agent_session);

            // Create a channel for live streaming events so the TUI can display
            // text deltas in real time instead of waiting for the full turn.
            let (event_tx, event_rx) =
                tokio::sync::mpsc::channel::<roko_agent::AgentRuntimeEvent>(256);

            // Install the receiver so the event loop can poll it each frame.
            session.streaming_event_rx = Some(event_rx);

            tokio::spawn(async move {
                let result = agent_session.send_turn_streaming(&text, event_tx).await;

                let mapped = match result {
                    Ok(turn) if turn.cancelled => Err("__cancelled__".to_string()),
                    Ok(turn) => Ok(turn_result_to_dispatch_result(turn)),
                    Err(error) => Err(error.to_string()),
                };
                let _ = tx.send(mapped).await;
            });
        }
    }
}

// ---------------------------------------------------------------------------
// Decomposed dispatch
// ---------------------------------------------------------------------------

/// Wrapper around `dispatch_prompt` that transparently handles multi-task
/// messages.
///
/// When `msg` decomposes into two or more sub-tasks the function dispatches
/// each one sequentially in a background task, merges the results, and sends
/// the merged `DispatchResult` through the session's `response_rx` channel —
/// identical to a single-task turn from the event loop's perspective.
///
/// When the message is a single task (no decomposition), this delegates
/// directly to `dispatch_prompt` so there is zero overhead on the common path.
pub(crate) fn dispatch_prompt_decomposed(session: &mut ChatSession, msg: &str) {
    let subtasks = decompose_message(msg);

    match subtasks {
        None => {
            // Fast path: single task — use the existing code unchanged.
            dispatch_prompt(session, msg);
        }
        Some(tasks) => {
            // Multi-task path: dispatch each sub-task sequentially and merge.
            use crate::inline::primitives::StreamingState;
            session.streaming = StreamingState::new("decomposing tasks...");
            let (tx, rx) = tokio::sync::mpsc::channel(1);
            session.response_rx = Some(rx);

            // Clone everything we need for the background task.
            let dispatch = session.dispatch.clone();
            let agent_id = session.agent_id.clone();
            let agent_session_opt = session.agent_session.as_ref().map(clone_chat_agent_session);

            tokio::spawn(async move {
                let result = run_decomposed(tasks, dispatch, &agent_id, agent_session_opt).await;
                let _ = tx.send(result.map_err(|e| e.to_string())).await;
            });
        }
    }
}

/// Execute each sub-task sequentially and return the merged result.
async fn run_decomposed(
    tasks: Vec<SubTask>,
    dispatch: DispatchMode,
    agent_id: &str,
    agent_session_opt: Option<crate::chat_session::ChatAgentSession>,
) -> Result<DispatchResult> {
    let mut collected: Vec<(SubTask, DispatchResult)> = Vec::with_capacity(tasks.len());

    for task in tasks {
        let result = dispatch_single(&dispatch, agent_id, &task.text, &agent_session_opt).await?;
        collected.push((task, result));
    }

    Ok(merge_results(collected))
}

/// Dispatch a single sub-task text using the session's dispatch mode and
/// return the `DispatchResult`.
async fn dispatch_single(
    dispatch: &DispatchMode,
    agent_id: &str,
    text: &str,
    agent_session_opt: &Option<crate::chat_session::ChatAgentSession>,
) -> Result<DispatchResult> {
    match dispatch {
        DispatchMode::Http {
            client,
            backend_url,
            is_sidecar,
        } => {
            let resp = send_and_receive(client, backend_url, agent_id, text, *is_sidecar).await?;
            Ok(resp.into())
        }
        DispatchMode::Direct { .. } => Err(anyhow::anyhow!(
            ChatInlineDispatchError::DirectDispatchDisabled.to_string()
        )),
        DispatchMode::Session => {
            let Some(agent_session) = agent_session_opt.as_ref() else {
                anyhow::bail!("agent session unavailable for decomposed dispatch");
            };
            let mut session_clone = clone_chat_agent_session(agent_session);
            // We don't wire live streaming events for sub-tasks; each result is
            // collected atomically before the merged response is surfaced.
            let (event_tx, _event_rx) =
                tokio::sync::mpsc::channel::<roko_agent::AgentRuntimeEvent>(256);
            let turn = session_clone
                .send_turn_streaming(text, event_tx)
                .await
                .context("sub-task dispatch")?;
            if turn.cancelled {
                anyhow::bail!("__cancelled__");
            }
            Ok(turn_result_to_dispatch_result(turn))
        }
    }
}

// ---------------------------------------------------------------------------
// Cost helpers
// ---------------------------------------------------------------------------

/// Compute naive Opus-rate baseline cost for savings comparison.
pub(crate) fn naive_opus_cost(input_tokens: u64, output_tokens: u64) -> f64 {
    (input_tokens as f64 * 15.0 / 1_000_000.0) + (output_tokens as f64 * 75.0 / 1_000_000.0)
}

/// Calculate cost from a dispatch result using the cost table.
pub(crate) fn cost_from_result(table: &CostTable, result: &DispatchResult) -> f64 {
    table.calculate(
        &result.model,
        &roko_agent::Usage {
            input_tokens: result.input_tokens as u32,
            output_tokens: result.output_tokens as u32,
            ..Default::default()
        },
    )
}

// ---------------------------------------------------------------------------
// Backend communication
// ---------------------------------------------------------------------------

/// HTTP response wrapper that can convert into [`DispatchResult`].
struct HttpResponse {
    text: String,
    model: String,
    input_tokens: u64,
    output_tokens: u64,
}

impl From<HttpResponse> for DispatchResult {
    fn from(r: HttpResponse) -> Self {
        Self {
            text: r.text,
            model: r.model,
            input_tokens: r.input_tokens,
            output_tokens: r.output_tokens,
            tool_outputs: Vec::new(),
            session_id: None,
        }
    }
}

/// Send a message and receive the response.
///
/// When `is_sidecar` is true, sends to `{url}/message` with `{"prompt": message}`.
/// When false, sends to `{url}/api/agents/{agent_id}/message` with `{"message": message}`.
///
/// For run-id polling, `base_url` is used as the serve URL to query status.
async fn send_and_receive(
    client: &reqwest::Client,
    base_url: &str,
    agent_id: &str,
    message: &str,
    is_sidecar: bool,
) -> Result<HttpResponse> {
    // For polling, we use base_url (which is the serve URL in the non-sidecar case).
    let poll_base = base_url;
    let (url, body) = if is_sidecar {
        (
            format!("{}/message", base_url.trim_end_matches('/')),
            json!({ "prompt": message }),
        )
    } else {
        (
            format!(
                "{}/api/agents/{agent_id}/message",
                base_url.trim_end_matches('/')
            ),
            json!({ "message": message }),
        )
    };

    let response = client
        .post(&url)
        .json(&body)
        .timeout(Duration::from_secs(
            roko_core::config::timeouts::DEFAULT_AGENT_TIMEOUT_SECS,
        ))
        .send()
        .await
        .with_context(|| format!("POST {url} — is `roko serve` running?"))?;

    if !response.status().is_success() {
        let status = response.status();
        let body_text = response.text().await.unwrap_or_default();
        // Try to extract a meaningful error message from JSON response
        let detail = serde_json::from_str::<serde_json::Value>(&body_text)
            .ok()
            .and_then(|v| v.get("error").and_then(|e| e.as_str()).map(String::from))
            .unwrap_or(body_text);
        bail!("{status}: {detail}");
    }

    #[derive(Deserialize)]
    struct Resp {
        #[serde(default)]
        response: Option<String>,
        #[serde(default)]
        run_id: Option<String>,
        #[serde(default)]
        model: Option<String>,
        #[serde(default)]
        input_tokens: Option<u64>,
        #[serde(default)]
        output_tokens: Option<u64>,
    }

    let resp: Resp = response.json().await.context("decode response")?;

    let resp_model = resp.model.unwrap_or_default();
    let resp_input = resp.input_tokens.unwrap_or(0);
    let resp_output = resp.output_tokens.unwrap_or(0);

    if let Some(text) = resp.response.filter(|s| !s.trim().is_empty()) {
        let clean = extract_clean_text(&text);
        let input_tokens = if resp_input > 0 {
            resp_input
        } else {
            (message.len() as u64) / 4
        };
        let output_tokens = if resp_output > 0 {
            resp_output
        } else {
            (clean.len() as u64) / 4
        };
        let model = if resp_model.is_empty() {
            "http".to_string()
        } else {
            resp_model
        };
        return Ok(HttpResponse {
            text: clean,
            model,
            input_tokens,
            output_tokens,
        });
    }

    if let Some(run_id) = resp.run_id.filter(|s| !s.trim().is_empty()) {
        // Poll for completion
        let status_url = format!(
            "{}/api/run/{run_id}/status",
            poll_base.trim_end_matches('/'),
        );
        loop {
            tokio::time::sleep(Duration::from_millis(500)).await;
            let status_resp = client
                .get(&status_url)
                .send()
                .await
                .context("poll status")?;

            #[derive(Deserialize)]
            struct StatusResp {
                #[serde(default)]
                finished: bool,
                #[serde(default)]
                output_text: Option<String>,
                #[serde(default)]
                error: Option<String>,
                #[serde(default)]
                model: Option<String>,
                #[serde(default)]
                input_tokens: Option<u64>,
                #[serde(default)]
                output_tokens: Option<u64>,
            }

            let status: StatusResp = status_resp.json().await.context("decode status")?;
            if status.finished {
                if let Some(err) = status.error.filter(|s| !s.trim().is_empty()) {
                    bail!("agent error: {err}");
                }
                let text = status.output_text.unwrap_or_default();
                let input_tokens = status
                    .input_tokens
                    .unwrap_or_else(|| (message.len() as u64) / 4);
                let output_tokens = status
                    .output_tokens
                    .unwrap_or_else(|| (text.len() as u64) / 4);
                let model = status.model.unwrap_or_else(|| "http".to_string());
                return Ok(HttpResponse {
                    text,
                    model,
                    input_tokens,
                    output_tokens,
                });
            }
        }
    }

    bail!("no response or run_id in reply");
}
