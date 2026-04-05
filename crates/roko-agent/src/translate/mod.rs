//! Tool-call translators (§36.c) — the bridge between canonical Roko
//! tools and each LLM backend's wire format.
//!
//! Every backend that speaks tools implements [`Translator`]. One instance
//! per backend (Claude / Ollama / `OpenAI` / `ReAct`). The future
//! `ToolLoop` calls [`Translator::render_tools`] once
//! per turn to generate the backend-specific tool spec, then
//! [`Translator::parse_calls`] to extract what the backend emitted, and
//! [`Translator::render_results`] to feed results back on the next turn.
//!
//! # Translator design
//!
//! Translators are **sync, pure functions** of their inputs. No I/O, no
//! side effects — they simply reshape data from canonical form into the
//! backend's expected shape and back. The async bits (HTTP, subprocess)
//! belong to backend adapters that sit *above* the translator.
//!
//! # Submodules
//!
//! - [`claude`] — Claude CLI (`--tools=...` flag + stream-json `tool_use` blocks)
//! - [`ollama`] — OpenAI-compatible JSON over `/api/chat`
//! - [`openai`] — `/v1/chat/completions` (mostly same wire as Ollama)
//! - [`react`] — prompt-level `ReAct` fallback for models without native tools
//! - [`capability`] — detect a model's capabilities + pick a translator
//!
//! # Research note
//!
//! Tool-call format preference is model-specific. Research shows 5–30
//! accuracy points on the table when using the wrong format (Meta-Harness,
//! `WildToolBench`, `Qwen3-coder` documented format switch above 5 tools).
//! This module is the enforcement point: each backend gets exactly the
//! format its profile says it prefers.

#![allow(clippy::module_name_repetitions)]

use roko_core::tool::{ToolCall, ToolDef, ToolFormat, ToolResult};

pub mod capability;
pub mod claude;
pub mod ollama;
pub mod openai;
pub mod react;

pub use capability::{capabilities_for, translator_for, ModelCapabilities};
pub use claude::ClaudeTranslator;
pub use ollama::OllamaTranslator;
pub use openai::OpenAiTranslator;
pub use react::ReActTranslator;

/// Bidirectional bridge between canonical tools and a backend's wire format.
///
/// Implementors are sync and **pure**: given identical inputs they must
/// produce identical outputs, and they perform no I/O.
pub trait Translator: Send + Sync {
    /// Which wire format this translator emits/parses.
    fn format(&self) -> ToolFormat;

    /// Serialize the tool catalog into the backend's expected shape.
    ///
    /// The caller passes the output through to the HTTP body (JSON tools
    /// array), a CLI flag (`--tools=Read,Edit,Bash`), or the system
    /// prompt (`ReAct` embeds schemas directly).
    fn render_tools(&self, tools: &[ToolDef]) -> RenderedTools;

    /// Parse the backend's response into a list of canonical tool calls.
    ///
    /// Returns an empty `Vec` if the response has no tool calls (the LLM
    /// answered directly). Returns `Err(TranslatorError::Malformed)` if
    /// the response *claims* tool calls but can't be parsed.
    fn parse_calls(
        &self,
        response: &BackendResponse,
    ) -> Result<Vec<ToolCall>, TranslatorError>;

    /// Serialize tool results back into the shape the backend consumes
    /// on the next turn (typically role=`"tool"`, `tool_call_id`, content).
    fn render_results(&self, results: &[(ToolCall, ToolResult)]) -> RenderedResults;
}

// ─── Payload enums ────────────────────────────────────────────────────────

/// Backend-specific tool-spec payload emitted by [`Translator::render_tools`].
#[derive(Debug, Clone)]
pub enum RenderedTools {
    /// JSON array for the HTTP body (`tools: [...]`). Used by Ollama,
    /// `OpenAI`, and any `OpenAI`-compatible gateway.
    JsonArray(serde_json::Value),
    /// CLI flag payload (e.g. `"Read,Edit,Bash"`). Used by Claude CLI.
    CliFlag(String),
    /// Text block to inline into the system prompt. Used by the `ReAct`
    /// fallback for models without native function calling.
    SystemPromptBlock(String),
}

/// Backend-specific tool-result payload emitted by
/// [`Translator::render_results`] for the next turn.
#[derive(Debug, Clone)]
pub enum RenderedResults {
    /// Array of tool-result messages (`OpenAI`, Ollama, compatible gateways).
    JsonMessages(serde_json::Value),
    /// Text to splice into the prompt (`ReAct`).
    TextBlock(String),
    /// No-op — the backend owns its own tool-call loop (Claude CLI,
    /// Codex MCP). Roko does not feed results back in these cases.
    HandledByBackend,
}

/// Raw backend response passed into [`Translator::parse_calls`].
///
/// Opaque to translators except via their own [`parse_calls`](Translator::parse_calls)
/// impl — one variant per wire shape the ecosystem currently uses.
#[derive(Debug, Clone)]
pub enum BackendResponse {
    /// Single JSON object (Ollama `/api/chat`, `OpenAI`
    /// `/v1/chat/completions`, Anthropic API non-streaming).
    Json(serde_json::Value),
    /// Sequence of stream-json events (Claude CLI).
    StreamJson(Vec<serde_json::Value>),
    /// Plain-text completion (`ReAct` models).
    Text(String),
}

impl BackendResponse {
    /// Extract the final assistant text (no tool calls) from this response.
    ///
    /// Used by the multi-turn loop to obtain the final answer when the
    /// backend stops calling tools. Returns the empty string if the
    /// response has no obvious text field.
    #[must_use]
    pub fn extract_text(&self) -> String {
        match self {
            Self::Text(s) => s.clone(),
            Self::Json(v) => v
                .pointer("/message/content")
                .and_then(|x| x.as_str())
                .or_else(|| {
                    v.pointer("/choices/0/message/content")
                        .and_then(|x| x.as_str())
                })
                .unwrap_or("")
                .to_string(),
            Self::StreamJson(events) => {
                let mut buf = String::new();
                for ev in events {
                    if let Some(delta) = ev
                        .pointer("/delta/text")
                        .and_then(|x| x.as_str())
                    {
                        buf.push_str(delta);
                    } else if let Some(text) = ev
                        .pointer("/content_block/text")
                        .and_then(|x| x.as_str())
                    {
                        buf.push_str(text);
                    }
                }
                buf
            }
        }
    }
}

/// Errors a [`Translator`] may produce.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum TranslatorError {
    /// The backend response claimed tool calls but couldn't be parsed.
    #[error("malformed response: {0}")]
    Malformed(String),
    /// The translator was handed a response in a format it doesn't accept.
    #[error("unsupported format: {0:?}")]
    UnsupportedFormat(ToolFormat),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backend_response_extract_text_from_text() {
        let r = BackendResponse::Text("hello".into());
        assert_eq!(r.extract_text(), "hello");
    }

    #[test]
    fn backend_response_extract_text_from_ollama_json() {
        let r = BackendResponse::Json(serde_json::json!({
            "message": { "content": "answer" }
        }));
        assert_eq!(r.extract_text(), "answer");
    }

    #[test]
    fn backend_response_extract_text_from_openai_json() {
        let r = BackendResponse::Json(serde_json::json!({
            "choices": [{"message": {"content": "done"}}]
        }));
        assert_eq!(r.extract_text(), "done");
    }

    #[test]
    fn backend_response_extract_text_from_stream_json() {
        let r = BackendResponse::StreamJson(vec![
            serde_json::json!({"delta": {"text": "one "}}),
            serde_json::json!({"delta": {"text": "two"}}),
        ]);
        assert_eq!(r.extract_text(), "one two");
    }

    #[test]
    fn backend_response_extract_text_empty_when_absent() {
        let r = BackendResponse::Json(serde_json::json!({}));
        assert_eq!(r.extract_text(), "");
    }

    #[test]
    fn translator_error_display_shows_variants() {
        let e = TranslatorError::Malformed("bad json".into());
        assert!(e.to_string().contains("bad json"));
        let e = TranslatorError::UnsupportedFormat(ToolFormat::ReActText);
        assert!(e.to_string().contains("ReActText"));
    }
}
