//! Small utility functions: event-to-update mapping, error formatting,
//! session transport helpers.

use std::time::Duration;

use tokio::{
    io::{AsyncRead, AsyncWrite},
    sync::mpsc,
};
use tracing::{debug, warn};

use crate::transport::StdioTransport;
use crate::types::{ContentBlock, SessionUpdate, ToolCallStatus};

use super::{BridgeEventsError, CognitiveEvent, Result};

pub(crate) fn map_event_to_update(event: CognitiveEvent) -> Option<SessionUpdate> {
    match event {
        CognitiveEvent::TokenChunk(text) => Some(SessionUpdate::AgentMessageChunk {
            content: text_block(text),
            _meta: None,
        }),
        CognitiveEvent::ThinkingChunk(text) => Some(SessionUpdate::AgentThoughtChunk {
            content: text_block(text),
        }),
        CognitiveEvent::ToolCallStart {
            tool_call_id,
            title,
            kind,
            locations,
        } => Some(SessionUpdate::ToolCall {
            tool_call_id,
            title,
            kind,
            status: ToolCallStatus::InProgress,
            content: Vec::new(),
            locations,
        }),
        CognitiveEvent::ToolCallComplete {
            tool_call_id,
            status,
            content,
        } => Some(SessionUpdate::ToolCallUpdate {
            tool_call_id,
            status,
            content,
            locations: None,
        }),
        CognitiveEvent::PlanUpdate { entries } => Some(SessionUpdate::Plan { entries }),
        CognitiveEvent::McpStatus { statuses } => Some(roko_meta_update("mcpStatus", &statuses)),
        CognitiveEvent::Complete { .. }
        | CognitiveEvent::Failure { .. }
        | CognitiveEvent::MaxTokens
        | CognitiveEvent::PermissionRequest { .. } => {
            tracing::warn!(
                "unexpected terminal/async cognitive event reached update mapping; skipping"
            );
            None
        }
    }
}

/// Wraps roko-only session data, such as MCP startup status or the cost budget, as a
/// spec `session_info_update` that carries it under `_meta.roko.<key>`. Spec clients
/// drop session updates whose `sessionUpdate` they do not know.
pub(crate) fn roko_meta_update(key: &str, value: &impl serde::Serialize) -> SessionUpdate {
    let mut roko = serde_json::Map::new();
    roko.insert(
        key.to_owned(),
        serde_json::to_value(value).unwrap_or_default(),
    );
    SessionUpdate::SessionInfoUpdate {
        title: None,
        _meta: Some(serde_json::json!({ "roko": roko })),
    }
}

pub(crate) fn dispatch_failure_update(message: String) -> SessionUpdate {
    SessionUpdate::AgentMessageChunk {
        content: text_block(message),
        _meta: None,
    }
}

/// Pattern-match known error strings and return an actionable user-facing message.
/// The original error is always appended so nothing is suppressed.
pub(crate) fn format_acp_error_for_user(error: &str) -> String {
    // Missing API key – extract the env var name and tell the user how to set it.
    if let Some(rest) = error
        .strip_prefix("Missing API key: env var ")
        .or_else(|| error.strip_prefix("missing API key: env var "))
    {
        let var = rest.split_whitespace().next().unwrap_or(rest);
        return format!(
            "Set {var} in your environment. Run: export {var}=your-key\n\n(Original error: {error})"
        );
    }

    // OpenAI-compat models that reject max_tokens in favour of max_completion_tokens.
    if error.contains("max_tokens is not supported") {
        return format!(
            "This model needs use_max_completion_tokens. Auto-fixing...\n\n(Original error: {error})"
        );
    }

    // Model not found / not configured.
    if error.contains("model not found") || error.contains("model_not_found") {
        return format!(
            "Model isn't configured or doesn't exist. Check your roko.toml [models] section.\n\n(Original error: {error})"
        );
    }

    // Rate limiting (HTTP 429 or explicit rate_limit error code).
    if error.contains("rate_limit") || error.contains("429") {
        return format!("Rate limited. Waiting 30s before retry...\n\n(Original error: {error})");
    }

    // Context length exceeded.
    if error.contains("context_length_exceeded") || error.contains("maximum context length") {
        return format!(
            "Prompt too long for this model. Consider truncating or switching to a model with a larger context window.\n\n(Original error: {error})"
        );
    }

    // No pattern matched – return as-is.
    error.to_string()
}

pub(crate) async fn emit_dispatch_failure(
    event_sender: &mpsc::Sender<CognitiveEvent>,
    message: String,
) {
    let message = format_acp_error_for_user(&message);
    send_cognitive_event(event_sender, CognitiveEvent::Failure { message }).await;
}

/// How long a progress event waits for room in a full event channel before it
/// is dropped, so an editor that stops reading stalls the provider stream only
/// this long per event.
pub(crate) const PROGRESS_EVENT_SEND_TIMEOUT: Duration = Duration::from_secs(30);

/// Most assistant text one prompt keeps for history, episodes and the
/// post-dispatch check. The editor still receives every chunk.
pub(crate) const MAX_ASSISTANT_TEXT_BYTES: usize = 1024 * 1024;

pub(crate) async fn send_cognitive_event(
    event_sender: &mpsc::Sender<CognitiveEvent>,
    event: CognitiveEvent,
) {
    send_cognitive_event_within(event_sender, event, PROGRESS_EVENT_SEND_TIMEOUT).await;
}

/// Sends `event`, giving up on a progress event after `progress_timeout` in a
/// full channel. Events that end the turn and permission requests wait for room:
/// the editor and the tool loop depend on them.
pub(crate) async fn send_cognitive_event_within(
    event_sender: &mpsc::Sender<CognitiveEvent>,
    event: CognitiveEvent,
    progress_timeout: Duration,
) {
    if event.ends_turn() || matches!(event, CognitiveEvent::PermissionRequest { .. }) {
        if event_sender.send(event).await.is_err() {
            debug!("cognitive event receiver dropped before event could be delivered");
        }
        return;
    }
    match event_sender.send_timeout(event, progress_timeout).await {
        Ok(()) => {}
        Err(mpsc::error::SendTimeoutError::Timeout(_)) => {
            warn!(
                timeout_ms = progress_timeout.as_millis(),
                "cognitive event channel stayed full; progress event dropped"
            );
        }
        Err(mpsc::error::SendTimeoutError::Closed(_)) => {
            debug!("cognitive event receiver dropped before event could be delivered");
        }
    }
}

/// Appends `chunk` to `assistant_text` up to [`MAX_ASSISTANT_TEXT_BYTES`],
/// cutting at a char boundary. Returns `true` when this chunk reached the cap.
pub(crate) fn append_assistant_text(assistant_text: &mut String, chunk: &str) -> bool {
    let room = MAX_ASSISTANT_TEXT_BYTES.saturating_sub(assistant_text.len());
    if chunk.len() <= room {
        assistant_text.push_str(chunk);
        return false;
    }
    let mut end = room;
    while !chunk.is_char_boundary(end) {
        end -= 1;
    }
    assistant_text.push_str(&chunk[..end]);
    room > 0
}

pub(crate) async fn send_session_update<R, W>(
    transport: &mut StdioTransport<R, W>,
    session_id: &str,
    update: SessionUpdate,
) -> Result<()>
where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    let update_value = serde_json::to_value(update)?;
    let params = serde_json::json!({
        "sessionId": session_id,
        "update": update_value,
    });
    transport
        .send_notification("session/update", params)
        .await
        .map_err(BridgeEventsError::from)
}

pub(crate) fn workflow_template_name(template: &crate::pipeline::WorkflowTemplate) -> &'static str {
    match template {
        crate::pipeline::WorkflowTemplate::Express => "express",
        crate::pipeline::WorkflowTemplate::Standard => "standard",
        crate::pipeline::WorkflowTemplate::Full => "full",
    }
}

pub(crate) fn text_block(text: String) -> ContentBlock {
    ContentBlock::Text { text }
}
