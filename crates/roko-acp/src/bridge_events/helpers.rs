//! Small utility functions: event-to-update mapping, error formatting,
//! session transport helpers.

use tokio::{
    io::{AsyncRead, AsyncWrite},
    sync::mpsc,
};
use tracing::debug;

use crate::transport::StdioTransport;
use crate::types::{ContentBlock, SessionUpdate, ToolCallStatus};

use super::{BridgeEventsError, CognitiveEvent, Result};

pub(crate) fn map_event_to_update(event: CognitiveEvent) -> SessionUpdate {
    match event {
        CognitiveEvent::TokenChunk(text) => SessionUpdate::AgentMessageChunk {
            content: text_block(text),
            _meta: None,
        },
        CognitiveEvent::ThinkingChunk(text) => SessionUpdate::AgentThoughtChunk {
            content: text_block(text),
        },
        CognitiveEvent::ToolCallStart {
            tool_call_id,
            title,
            kind,
            locations,
        } => SessionUpdate::ToolCall {
            tool_call_id,
            title,
            kind,
            status: ToolCallStatus::InProgress,
            content: Vec::new(),
            locations,
        },
        CognitiveEvent::ToolCallComplete {
            tool_call_id,
            status,
            content,
        } => SessionUpdate::ToolCallUpdate {
            tool_call_id,
            status,
            content,
            locations: None,
        },
        CognitiveEvent::PlanUpdate { entries } => SessionUpdate::Plan { entries },
        CognitiveEvent::McpStatus { statuses } => SessionUpdate::McpStatusUpdate { statuses },
        CognitiveEvent::Complete { .. }
        | CognitiveEvent::Failure { .. }
        | CognitiveEvent::MaxTokens
        | CognitiveEvent::PermissionRequest { .. } => {
            unreachable!("terminal/async cognitive events are handled before update mapping")
        }
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

pub(crate) async fn emit_dispatch_failure(event_sender: &mpsc::Sender<CognitiveEvent>, message: String) {
    let message = format_acp_error_for_user(&message);
    send_cognitive_event(event_sender, CognitiveEvent::Failure { message }).await;
}

pub(crate) async fn send_cognitive_event(event_sender: &mpsc::Sender<CognitiveEvent>, event: CognitiveEvent) {
    if event_sender.send(event).await.is_err() {
        debug!("cognitive event receiver dropped before event could be delivered");
    }
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
