//! Cerebras Inference provider adapter.
//!
//! Cerebras provides ultra-fast LLM inference via an OpenAI-compatible API at
//! `api.cerebras.ai/v1`. Unlike the generic OpenAI-compat adapter, this one
//! applies Cerebras-specific optimizations for small-model tool calling:
//!
//! - **`StrictOpenAiTranslator`** — sets `strict: true` + `additionalProperties: false`
//!   on tool schemas for constrained decoding.
//! - **Few-shot examples** — injects tool-call demonstrations between the system
//!   prompt and user message so small models learn the expected format.
//! - **System prompt preamble** — explicit instruction to use the tool-call
//!   interface rather than emitting tool calls as text.

use std::sync::Arc;

use crate::Agent;
use crate::http::ReqwestPoster;
use crate::provider::{
    AgentCreationError, AgentOptions, ProviderAdapter, ProviderError, build_tool_dispatcher,
    openai_compat::tool_registry_for_options, tool_loop_max_iterations_for_profile,
};
use crate::tool_loop::ToolLoop;
use crate::tool_loop::agent_wrapper::ToolLoopAgent;
use crate::tool_loop::backends::create_openai_compat_backend;
use crate::translate::{StrictOpenAiTranslator, Translator};
use roko_core::agent::ProviderKind;
use roko_core::config::schema::{ModelProfile, ProviderConfig};
use serde_json::Value;

/// Adapter for the Cerebras Inference API.
///
/// Builds agents with strict tool schemas, few-shot examples, and a
/// tool-call preamble so that small models (Llama 3.1 8B) reliably
/// produce structured `tool_calls` instead of emitting tool invocations
/// as plain text.
pub struct CerebrasAdapter;

impl ProviderAdapter for CerebrasAdapter {
    fn kind(&self) -> ProviderKind {
        ProviderKind::CerebrasApi
    }

    fn create_agent(
        &self,
        provider: &ProviderConfig,
        model: &ModelProfile,
        options: &AgentOptions,
    ) -> Result<Box<dyn Agent>, AgentCreationError> {
        let timeout = options.effective_timeout_ms(provider.timeout_ms);
        let agent_name = if options.name.is_empty() {
            format!("cerebras:{}", model.slug)
        } else {
            options.name.clone()
        };

        if model.supports_tools {
            let (registry, tools, resolver) = tool_registry_for_options(model, options)?;
            let dispatcher = build_tool_dispatcher(registry, resolver);

            // Strict translator for constrained decoding on small models.
            let translator: Arc<dyn Translator> = Arc::new(StrictOpenAiTranslator);

            let mut tool_loop_provider = provider.clone();
            tool_loop_provider.timeout_ms = Some(timeout);
            let poster = Arc::new(ReqwestPoster::new());
            let backend = create_openai_compat_backend(&tool_loop_provider, model, poster)?;

            let tool_loop = ToolLoop::new(translator, dispatcher, backend)
                .with_max_iterations(tool_loop_max_iterations_for_profile(Some(model)))
                .with_context_token_limit(
                    usize::try_from(model.context_window).unwrap_or(usize::MAX),
                )
                .with_few_shot_messages(coding_few_shot_examples());

            // Prepend a tool-call instruction for small models that tend to
            // emit tool invocations as text instead of using the API.
            let tool_instruction = "\
You are a coding assistant. You MUST use the provided tools to complete tasks. \
Do NOT write tool calls as text or code blocks — use the tool calling interface. \
Call one tool at a time. After each tool result, decide your next action.\n\n";

            let system_prompt = match &options.system_prompt {
                Some(prompt) => format!("{tool_instruction}{prompt}"),
                None => tool_instruction.to_string(),
            };

            let mut agent = ToolLoopAgent::new(tool_loop)
                .with_tools(tools)
                .with_name(agent_name)
                .with_system_prompt(system_prompt);
            if let Some(ref dir) = options.working_dir {
                agent = agent.with_worktree_path(dir.clone());
            }
            if let Some(root) = options.effective_immune_root() {
                agent = agent.with_immune_root(root);
            }

            return Ok(Box::new(agent));
        }

        // Non-tool path — delegate to generic OpenAI-compat adapter.
        use crate::provider::openai_compat::OpenAiCompatAdapter;
        OpenAiCompatAdapter.create_agent(provider, model, options)
    }

    fn supports_local_tool_runtime(&self) -> bool {
        true
    }

    fn classify_error(&self, status: u16, body: &Value) -> ProviderError {
        super::error_classify::classify_http_status(
            status,
            body,
            super::error_classify::RetryAfterSource::ErrorRetryAfter,
        )
    }
}

/// Few-shot examples that teach small models the tool-call protocol.
///
/// Each round-trip shows: user request → assistant tool_calls → tool result → assistant summary.
/// These are injected between the system prompt and the actual user message.
fn coding_few_shot_examples() -> Vec<serde_json::Value> {
    vec![
        serde_json::json!({
            "role": "user",
            "content": "Create a new Rust project called hello-world"
        }),
        serde_json::json!({
            "role": "assistant",
            "content": serde_json::Value::Null,
            "tool_calls": [{
                "id": "ex_1",
                "type": "function",
                "function": {
                    "name": "bash",
                    "arguments": "{\"command\":\"cargo init --name hello-world\"}"
                }
            }]
        }),
        serde_json::json!({
            "role": "tool",
            "tool_call_id": "ex_1",
            "content": "     Created binary (application) package"
        }),
        serde_json::json!({
            "role": "assistant",
            "content": "I've initialized a new Rust project called hello-world."
        }),
        serde_json::json!({
            "role": "user",
            "content": "Write a hello world function in src/lib.rs"
        }),
        serde_json::json!({
            "role": "assistant",
            "content": serde_json::Value::Null,
            "tool_calls": [{
                "id": "ex_2",
                "type": "function",
                "function": {
                    "name": "write_file",
                    "arguments": "{\"path\":\"src/lib.rs\",\"content\":\"pub fn hello() -> &'static str {\\n    \\\"Hello, world!\\\"\\n}\\n\\n#[cfg(test)]\\nmod tests {\\n    use super::*;\\n\\n    #[test]\\n    fn test_hello() {\\n        assert_eq!(hello(), \\\"Hello, world!\\\");\\n    }\\n}\\n\"}"
                }
            }]
        }),
        serde_json::json!({
            "role": "tool",
            "tool_call_id": "ex_2",
            "content": "File written successfully"
        }),
        serde_json::json!({
            "role": "assistant",
            "content": "I've written a hello function with a unit test in src/lib.rs."
        }),
    ]
}
