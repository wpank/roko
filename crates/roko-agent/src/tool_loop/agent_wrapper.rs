//! `ToolLoopAgent` — wrap [`ToolLoop`](super::ToolLoop) in the runtime-facing
//! [`Agent`](crate::agent::Agent) trait.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use roko_core::extension::CamelTaintLevel;
use roko_core::tool::{
    AuditSink, CancelToken, CorrelationEnvelope, MetricsSink, NeverCancel, NoopAuditSink,
    NoopMetricsSink, NoopTraceSink, ToolContext, ToolDef, ToolPermission, TraceSink,
};
use roko_core::{Body, Context, Kind, Signal};
use roko_fs::RokoLayout;

use crate::agent::{Agent, AgentResult, derived_output};
use crate::multimodal::{anthropic_messages, gemini_messages, openai_messages};
use crate::streaming::StreamChunk;
use crate::task_runner::task_id_from_context;
use roko_core::{ModelInputMessage, validate_model_input_messages};

use super::{StopReason, ToolLoop, ToolLoopOutput, ToolLoopTurnTrace};

use tokio::sync::mpsc;

/// Runtime-facing wrapper that lets the orchestrator drive [`ToolLoop`] via
/// the existing [`Agent`] trait.
pub struct ToolLoopAgent {
    tool_loop: ToolLoop,
    system_prompt: Option<String>,
    tools: Vec<ToolDef>,
    name: String,
    worktree_path: PathBuf,
    immune_root_path: Option<PathBuf>,
    input_messages: Vec<ModelInputMessage>,
    input_format: MultimodalInputFormat,
    // Production dispatch policy fields (T026/T027/T032)
    timeout: Duration,
    capabilities: ToolPermission,
    audit_sink: Arc<dyn AuditSink>,
    trace_sink: Arc<dyn TraceSink>,
    metrics_sink: Arc<dyn MetricsSink>,
    cancel_token: Arc<dyn CancelToken>,
    correlation: CorrelationEnvelope,
}

/// Provider-facing format used for the initial structured message history.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum MultimodalInputFormat {
    /// Anthropic `image/source` blocks (also accepted by Gemini-native conversion).
    #[default]
    Anthropic,
    /// OpenAI `image_url` content parts.
    OpenAi,
    /// Gemini-native `parts` with inline image data.
    Gemini,
}

impl ToolLoopAgent {
    /// Construct a wrapper around an existing tool loop.
    #[must_use]
    pub fn new(tool_loop: ToolLoop) -> Self {
        let worktree_path = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        Self {
            tool_loop,
            system_prompt: None,
            tools: Vec::new(),
            name: "tool-loop".to_string(),
            immune_root_path: None,
            worktree_path,
            input_messages: Vec::new(),
            input_format: MultimodalInputFormat::default(),
            timeout: Duration::from_secs(60),
            capabilities: ToolPermission {
                read: true,
                write: true,
                exec: true,
                git: true,
                network: false,
            },
            audit_sink: Arc::new(NoopAuditSink),
            trace_sink: Arc::new(NoopTraceSink),
            metrics_sink: Arc::new(NoopMetricsSink),
            cancel_token: Arc::new(NeverCancel),
            correlation: CorrelationEnvelope::empty(),
        }
    }

    /// Attach a system prompt that is prepended on the first turn.
    #[must_use]
    pub fn with_system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = Some(prompt.into());
        self
    }

    /// Attach the tool definitions exposed to the model.
    #[must_use]
    pub fn with_tools(mut self, tools: Vec<ToolDef>) -> Self {
        self.tools = tools;
        self
    }

    /// Override the display name used by logs and the orchestrator.
    #[must_use]
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// Override the worktree root used for tool execution.
    #[must_use]
    pub fn with_worktree_path(mut self, worktree_path: impl Into<PathBuf>) -> Self {
        self.worktree_path = worktree_path.into();
        self
    }

    /// Override the canonical root for durable immune controls and evidence.
    #[must_use]
    pub fn with_immune_root(mut self, immune_root: impl Into<PathBuf>) -> Self {
        let immune_root = immune_root.into();
        self.immune_root_path = Some(immune_root.canonicalize().unwrap_or(immune_root));
        self
    }

    /// Attach an ordered provider-neutral multimodal message history.
    #[must_use]
    pub fn with_input_messages(mut self, messages: Vec<ModelInputMessage>) -> Self {
        self.input_messages = messages;
        self
    }

    /// Select the provider-facing initial-message format.
    #[must_use]
    pub const fn with_multimodal_input_format(
        mut self,
        input_format: MultimodalInputFormat,
    ) -> Self {
        self.input_format = input_format;
        self
    }

    /// Override the tool-call timeout used for the production `ToolContext`.
    #[must_use]
    pub const fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Override the capability flags granted to tool handlers.
    #[must_use]
    pub const fn with_capabilities(mut self, capabilities: ToolPermission) -> Self {
        self.capabilities = capabilities;
        self
    }

    /// Attach a real audit sink for production dispatch.
    #[must_use]
    pub fn with_audit_sink(mut self, sink: Arc<dyn AuditSink>) -> Self {
        self.audit_sink = sink;
        self
    }

    /// Attach a real trace sink for production dispatch.
    #[must_use]
    pub fn with_trace_sink(mut self, sink: Arc<dyn TraceSink>) -> Self {
        self.trace_sink = sink;
        self
    }

    /// Attach a real metrics sink for production dispatch.
    #[must_use]
    pub fn with_metrics_sink(mut self, sink: Arc<dyn MetricsSink>) -> Self {
        self.metrics_sink = sink;
        self
    }

    /// Wire the active agent cancellation token (T027).
    #[must_use]
    pub fn with_cancel_token(mut self, token: Arc<dyn CancelToken>) -> Self {
        self.cancel_token = token;
        self
    }

    /// Attach correlation metadata for trace/audit joining (T032).
    #[must_use]
    pub fn with_correlation(mut self, correlation: CorrelationEnvelope) -> Self {
        self.correlation = correlation;
        self
    }

    /// Build a production [`ToolContext`] using the configured sinks,
    /// cancel token, capabilities, and correlation data.
    fn build_tool_context(&self) -> ToolContext {
        ToolContext::production(
            &self.worktree_path,
            self.timeout,
            self.capabilities,
            Arc::clone(&self.audit_sink),
            Arc::clone(&self.trace_sink),
            Arc::clone(&self.metrics_sink),
            Arc::clone(&self.cancel_token),
            self.correlation.clone(),
        )
        .with_immune_root(
            self.immune_root_path
                .as_deref()
                .unwrap_or(&self.worktree_path),
        )
        .with_taint_level(CamelTaintLevel::External)
    }

    fn structured_messages(&self) -> Vec<serde_json::Value> {
        let mut messages = match self.input_format {
            MultimodalInputFormat::Anthropic => anthropic_messages(&self.input_messages),
            MultimodalInputFormat::OpenAi => openai_messages(&self.input_messages),
            MultimodalInputFormat::Gemini => gemini_messages(&self.input_messages),
        };
        if let Some(system) = self
            .system_prompt
            .as_deref()
            .map(str::trim)
            .filter(|system| !system.is_empty())
        {
            messages.insert(0, serde_json::json!({"role": "system", "content": system}));
        }
        messages
    }

    fn output_signal(
        &self,
        input: &Signal,
        text: &str,
        stop_reason: &str,
        iterations: usize,
    ) -> Signal {
        let builder = derived_output(input, Kind::AgentOutput, Body::text(text))
            .tag("stop_reason", stop_reason)
            .tag("iterations", iterations.to_string());
        match self.model_slug() {
            Some(slug) => builder.tag("model", slug).build(),
            None => builder.build(),
        }
    }

    /// Model slug configured on the tool loop's model profile, if any.
    ///
    /// The tool loop aggregates legacy [`crate::usage::Usage`] per turn, which
    /// has no model field; the profile attached via
    /// [`ToolLoop::with_model_profile`] is the only model identity available
    /// at this layer.
    fn model_slug(&self) -> Option<&str> {
        self.tool_loop
            .model_profile
            .as_ref()
            .map(|profile| profile.slug.as_str())
            .filter(|slug| !slug.is_empty())
    }

    /// Stamp the configured model slug onto the usage observation so model
    /// attribution survives the legacy `Usage` conversion.
    fn attach_model(&self, mut result: AgentResult) -> AgentResult {
        if let Some(slug) = self.model_slug()
            && let Some(usage_obs) = result.usage_obs.as_mut()
        {
            usage_obs.model = Some(slug.to_string());
        }
        result
    }

    fn checkpoint_path(&self, ctx: &Context) -> Option<PathBuf> {
        let task_id = task_id_from_context(ctx);
        if task_id.is_empty() {
            return None;
        }

        let safe_task_id = task_id.replace(['/', '\\'], "_");
        Some(
            RokoLayout::for_project(&self.worktree_path)
                .state_dir()
                .join(format!("tool-loop-{safe_task_id}.json")),
        )
    }

    fn attach_trace_metadata(
        mut result: AgentResult,
        input: &Signal,
        output: &ToolLoopOutput,
    ) -> AgentResult {
        result.trace.extend(
            output
                .turn_traces
                .iter()
                .map(|trace| Self::trace_signal(input, trace)),
        );
        result
    }

    fn trace_signal(input: &Signal, trace: &ToolLoopTurnTrace) -> Signal {
        let tool_calls = trace
            .tool_calls
            .iter()
            .enumerate()
            .map(|(idx, call)| {
                serde_json::json!({
                    "name": call.name.as_str(),
                    "result_preview": trace.tool_results.get(idx).cloned().unwrap_or_default(),
                })
            })
            .collect::<Vec<_>>();

        derived_output(
            input,
            Kind::Custom("agent.trace".to_string()),
            Body::Json(serde_json::json!({
                "turn": trace.turn,
                "tool_calls": tool_calls,
                "reasoning": trace.reasoning.clone(),
                "usage": {
                    "input_tokens": trace.usage.input_tokens,
                    "output_tokens": trace.usage.output_tokens,
                    "cache_read_tokens": trace.usage.cache_read_tokens,
                    "cache_write_tokens": trace.usage.cache_create_tokens,
                    "total_tokens": trace.usage.total_tokens(),
                    "cost_usd": trace.usage.cost_usd,
                },
            })),
        )
        .build()
    }
}

#[async_trait]
impl Agent for ToolLoopAgent {
    async fn run(&self, input: &Signal, ctx: &Context) -> AgentResult {
        let prompt = input.body.as_text().unwrap_or_default();
        let tool_ctx = self.build_tool_context();
        let tool_loop = match self.checkpoint_path(ctx) {
            Some(path) => self.tool_loop.clone().with_checkpoint_path(path),
            None => self.tool_loop.clone(),
        };
        let output = if self.input_messages.is_empty() {
            tool_loop
                .run(
                    self.system_prompt.as_deref().unwrap_or(""),
                    prompt,
                    &self.tools,
                    &tool_ctx,
                )
                .await
        } else if let Err(error) = validate_model_input_messages(&self.input_messages) {
            return AgentResult::fail(self.output_signal(
                input,
                &format!("invalid image input: {error}"),
                "backend_error",
                0,
            ));
        } else {
            tool_loop
                .run_messages(self.structured_messages(), &self.tools, &tool_ctx)
                .await
        };

        let result = match &output.stop_reason {
            StopReason::Stop => AgentResult::ok(self.output_signal(
                input,
                &output.final_text,
                "stop",
                output.iterations,
            ))
            .with_usage(output.total_usage),
            StopReason::MaxIterations => AgentResult::fail(self.output_signal(
                input,
                &format!("Max iterations ({}) reached", output.iterations),
                "max_iterations",
                output.iterations,
            ))
            .with_usage(output.total_usage),
            StopReason::Cancelled => AgentResult::fail(self.output_signal(
                input,
                "Tool loop cancelled",
                "cancelled",
                output.iterations,
            ))
            .with_usage(output.total_usage),
            StopReason::BackendError(err) => AgentResult::fail(self.output_signal(
                input,
                err,
                "backend_error",
                output.iterations,
            ))
            .with_usage(output.total_usage),
            StopReason::BudgetExhausted => AgentResult::fail(self.output_signal(
                input,
                "Budget exhausted",
                "budget_exhausted",
                output.iterations,
            ))
            .with_usage(output.total_usage),
        };

        let result = Self::attach_trace_metadata(result, input, &output);
        self.attach_model(result)
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn backend_id(&self) -> &'static str {
        self.tool_loop.backend_id()
    }

    fn supports_streaming(&self) -> bool {
        true
    }

    async fn run_streaming(
        &self,
        input: &Signal,
        ctx: &Context,
        event_tx: mpsc::Sender<StreamChunk>,
    ) -> AgentResult {
        let prompt = input.body.as_text().unwrap_or_default();
        let tool_ctx = self.build_tool_context();
        let tool_loop = match self.checkpoint_path(ctx) {
            Some(path) => self.tool_loop.clone().with_checkpoint_path(path),
            None => self.tool_loop.clone(),
        };
        let output = if self.input_messages.is_empty() {
            tool_loop
                .run_streaming(
                    self.system_prompt.as_deref().unwrap_or(""),
                    prompt,
                    &self.tools,
                    &tool_ctx,
                    event_tx,
                )
                .await
        } else if let Err(error) = validate_model_input_messages(&self.input_messages) {
            return AgentResult::fail(self.output_signal(
                input,
                &format!("invalid image input: {error}"),
                "backend_error",
                0,
            ));
        } else {
            tool_loop
                .run_messages_streaming(
                    self.structured_messages(),
                    &self.tools,
                    &tool_ctx,
                    event_tx,
                )
                .await
        };

        let result = match &output.stop_reason {
            StopReason::Stop => AgentResult::ok(self.output_signal(
                input,
                &output.final_text,
                "stop",
                output.iterations,
            ))
            .with_usage(output.total_usage),
            StopReason::MaxIterations => AgentResult::fail(self.output_signal(
                input,
                &format!("Max iterations ({}) reached", output.iterations),
                "max_iterations",
                output.iterations,
            ))
            .with_usage(output.total_usage),
            StopReason::Cancelled => AgentResult::fail(self.output_signal(
                input,
                "Tool loop cancelled",
                "cancelled",
                output.iterations,
            ))
            .with_usage(output.total_usage),
            StopReason::BackendError(err) => AgentResult::fail(self.output_signal(
                input,
                err,
                "backend_error",
                output.iterations,
            ))
            .with_usage(output.total_usage),
            StopReason::BudgetExhausted => AgentResult::fail(self.output_signal(
                input,
                "Budget exhausted",
                "budget_exhausted",
                output.iterations,
            ))
            .with_usage(output.total_usage),
        };

        let result = Self::attach_trace_metadata(result, input, &output);
        self.attach_model(result)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::*;
    use crate::dispatcher::HandlerResolver;
    use crate::provider::build_tool_dispatcher;
    use crate::tool_loop::{LlmBackend, LlmError, ToolLoop};
    use crate::translate::{
        BackendResponse, RenderedResults, RenderedTools, Translator, TranslatorError,
    };
    use roko_core::tool::{
        ToolCall, ToolCategory, ToolConcurrency, ToolFormat, ToolHandler, ToolPermission,
        ToolResult, VecToolRegistry,
    };

    struct MockTranslator;

    impl Translator for MockTranslator {
        fn format(&self) -> ToolFormat {
            ToolFormat::OpenAiJson
        }

        fn render_tools(&self, _tools: &[ToolDef]) -> RenderedTools {
            RenderedTools::JsonArray(serde_json::json!([]))
        }

        fn parse_calls(
            &self,
            response: &BackendResponse,
        ) -> Result<Vec<ToolCall>, TranslatorError> {
            let BackendResponse::Json(ref value) = *response else {
                return Ok(Vec::new());
            };
            let Some(calls) = value.get("tool_calls").and_then(|value| value.as_array()) else {
                return Ok(Vec::new());
            };

            Ok(calls
                .iter()
                .map(|call| {
                    ToolCall::new(
                        call["id"].as_str().unwrap_or_default(),
                        call["name"].as_str().unwrap_or_default(),
                        call.get("arguments")
                            .cloned()
                            .unwrap_or_else(|| serde_json::json!({})),
                    )
                })
                .collect())
        }

        fn render_results(&self, results: &[(ToolCall, ToolResult)]) -> RenderedResults {
            let messages: Vec<serde_json::Value> = results
                .iter()
                .map(|(call, result)| {
                    let content = match result {
                        ToolResult::Ok { content, .. } => content.clone(),
                        ToolResult::Err(err) => format!("error: {err}"),
                    };
                    serde_json::json!({
                        "role": "tool",
                        "tool_call_id": call.id,
                        "content": content,
                    })
                })
                .collect();
            RenderedResults::JsonMessages(serde_json::json!(messages))
        }
    }

    struct EchoHandler;

    #[async_trait]
    impl ToolHandler for EchoHandler {
        fn name(&self) -> &str {
            "echo"
        }

        async fn execute(&self, call: ToolCall, _ctx: &ToolContext) -> ToolResult {
            ToolResult::text(call.arguments.to_string())
        }
    }

    struct TwoStepBackend {
        call_count: AtomicUsize,
    }

    impl TwoStepBackend {
        fn new() -> Self {
            Self {
                call_count: AtomicUsize::new(0),
            }
        }
    }

    #[async_trait]
    impl LlmBackend for TwoStepBackend {
        async fn send_turn(
            &self,
            _messages: &[serde_json::Value],
            _tools: &RenderedTools,
            _session: &crate::translate::SessionState,
        ) -> Result<BackendResponse, LlmError> {
            let call = self.call_count.fetch_add(1, Ordering::SeqCst);
            if call == 0 {
                Ok(BackendResponse::Json(serde_json::json!({
                    "tool_calls": [{
                        "id": "call-1",
                        "name": "echo",
                        "arguments": { "value": 1 }
                    }]
                })))
            } else {
                Ok(BackendResponse::Json(
                    serde_json::json!({"message": {"content": "final answer"}}),
                ))
            }
        }
    }

    struct ErrorBackend;

    #[async_trait]
    impl LlmBackend for ErrorBackend {
        async fn send_turn(
            &self,
            _messages: &[serde_json::Value],
            _tools: &RenderedTools,
            _session: &crate::translate::SessionState,
        ) -> Result<BackendResponse, LlmError> {
            Err(LlmError::Backend("server error".into()))
        }
    }

    fn test_tools() -> Vec<ToolDef> {
        vec![
            ToolDef::new(
                "echo",
                "echo args",
                ToolCategory::Meta,
                ToolPermission::read_only(),
            )
            .with_concurrency(ToolConcurrency::Parallel),
        ]
    }

    fn make_tool_loop(backend: Arc<dyn LlmBackend>) -> ToolLoop {
        let registry: Arc<dyn roko_core::tool::ToolRegistry> =
            Arc::new(VecToolRegistry::from_tools(test_tools()));
        let resolver: Arc<dyn HandlerResolver> =
            Arc::new(|name: &str| -> Option<Arc<dyn ToolHandler>> {
                if name == "echo" {
                    Some(Arc::new(EchoHandler) as Arc<dyn ToolHandler>)
                } else {
                    None
                }
            });
        let dispatcher = build_tool_dispatcher(registry, resolver);
        let translator: Arc<dyn Translator> = Arc::new(MockTranslator);
        ToolLoop::new(translator, dispatcher, backend)
    }

    #[test]
    fn tool_loop_selects_provider_correct_multimodal_initial_format() {
        let input = vec![roko_core::ModelInputMessage::new(
            roko_core::MessageRole::User,
            vec![
                roko_core::ModelInputBlock::text("before"),
                roko_core::ModelInputBlock::image("image/png", "aGVsbG8="),
                roko_core::ModelInputBlock::text("after"),
            ],
        )];

        let anthropic = ToolLoopAgent::new(make_tool_loop(Arc::new(ErrorBackend)))
            .with_input_messages(input.clone())
            .with_multimodal_input_format(MultimodalInputFormat::Anthropic)
            .structured_messages();
        assert_eq!(anthropic[0]["content"][0]["text"], "before");
        assert_eq!(anthropic[0]["content"][1]["source"]["data"], "aGVsbG8=");
        assert_eq!(anthropic[0]["content"][2]["text"], "after");

        let openai = ToolLoopAgent::new(make_tool_loop(Arc::new(ErrorBackend)))
            .with_input_messages(input.clone())
            .with_multimodal_input_format(MultimodalInputFormat::OpenAi)
            .structured_messages();
        assert_eq!(openai[0]["content"][0]["text"], "before");
        assert_eq!(
            openai[0]["content"][1]["image_url"]["url"],
            "data:image/png;base64,aGVsbG8="
        );
        assert_eq!(openai[0]["content"][2]["text"], "after");

        let gemini = ToolLoopAgent::new(make_tool_loop(Arc::new(ErrorBackend)))
            .with_input_messages(input)
            .with_multimodal_input_format(MultimodalInputFormat::Gemini)
            .structured_messages();
        assert_eq!(gemini[0]["parts"][0]["text"], "before");
        assert_eq!(gemini[0]["parts"][1]["inlineData"]["data"], "aGVsbG8=");
        assert_eq!(gemini[0]["parts"][2]["text"], "after");
    }

    #[tokio::test]
    async fn tool_loop_agent_wrapper_runs_tool_loop() {
        let agent = ToolLoopAgent::new(make_tool_loop(Arc::new(TwoStepBackend::new())))
            .with_name("glm-tool-loop")
            .with_system_prompt("system prompt")
            .with_tools(test_tools())
            .with_worktree_path("/tmp");
        let ancestor = Signal::builder(Kind::Prompt)
            .body(Body::text("ancestor"))
            .build();
        let input = Signal::builder(Kind::Prompt)
            .body(Body::text("call the tool"))
            .lineage([ancestor.id])
            .build();

        let result = agent.run(&input, &Context::now()).await;

        assert!(result.success);
        assert_eq!(
            result.output.body.as_text().expect("text output"),
            "final answer"
        );
        assert_eq!(result.output.lineage, vec![ancestor.id, input.id]);
        assert_eq!(result.output.tag("stop_reason"), Some("stop"));
        assert_eq!(result.output.tag("iterations"), Some("1"));
        assert_eq!(agent.name(), "glm-tool-loop");
        assert!(agent.supports_streaming());
    }

    #[tokio::test]
    async fn tool_loop_agent_wrapper_maps_backend_errors_to_failures() {
        let agent = ToolLoopAgent::new(make_tool_loop(Arc::new(ErrorBackend)))
            .with_tools(test_tools())
            .with_worktree_path("/tmp");
        let input = Signal::builder(Kind::Prompt)
            .body(Body::text("fail"))
            .build();

        let result = agent.run(&input, &Context::now()).await;

        assert!(!result.success);
        assert_eq!(
            result.output.body.as_text().expect("text output"),
            "backend error: server error"
        );
        assert_eq!(result.output.tag("stop_reason"), Some("backend_error"));
    }

    #[tokio::test]
    async fn tool_loop_agent_stamps_configured_model_on_result() {
        let profile = roko_core::config::schema::ModelProfile {
            provider: "openai_compat".to_string(),
            slug: "gpt-5.6-sol".to_string(),
            ..Default::default()
        };
        let tool_loop = make_tool_loop(Arc::new(TwoStepBackend::new())).with_model_profile(profile);
        let agent = ToolLoopAgent::new(tool_loop)
            .with_tools(test_tools())
            .with_worktree_path("/tmp");
        let input = Signal::builder(Kind::Prompt)
            .body(Body::text("call the tool"))
            .build();

        let result = agent.run(&input, &Context::now()).await;

        assert!(result.success);
        assert_eq!(result.output.tag("model"), Some("gpt-5.6-sol"));
        let usage_obs = result.usage_obs.expect("usage_obs populated");
        assert_eq!(
            usage_obs.model.as_deref(),
            Some("gpt-5.6-sol"),
            "usage_obs must carry the configured model slug"
        );
    }
}
