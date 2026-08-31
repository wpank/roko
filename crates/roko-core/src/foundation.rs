//! Foundation traits for the workflow engine.
//!
//! These define the contracts between the engine and its services:
//! - `ModelCaller` - call LLMs (implemented by roko-agent)
//! - `PromptAssembler` - build system prompts (implemented by roko-compose)
//! - `FeedbackSink` - record feedback (implemented by roko-learn)
//! - `GateRunner` - run verification gates (implemented by roko-gate)
//! - `EventConsumer` - observe runtime events (implemented by adapters)
//! - `EffectExecutor` - execute side-effects (implemented by roko-runtime)

use crate::runtime_event::RuntimeEvent;
use crate::tool::ToolDef;
use crate::{Result, RokoError};
use async_trait::async_trait;
use base64::Engine as _;
use futures_core::Stream;
use std::path::PathBuf;
use std::pin::Pin;

/// Maximum decoded size accepted for one inline model image (5 MiB).
pub const MAX_MODEL_IMAGE_BYTES: usize = 5 * 1024 * 1024;
/// Maximum aggregate decoded image bytes accepted by one model call (20 MiB).
pub const MAX_MODEL_IMAGE_TOTAL_BYTES: usize = 20 * 1024 * 1024;
/// Maximum number of inline images accepted by one model call.
pub const MAX_MODEL_IMAGES: usize = 20;

/// A validated, provider-neutral inline image attached to a model request.
///
/// Provider adapters translate this representation to Anthropic `source`,
/// OpenAI `image_url`, or Gemini `inlineData` blocks at the final wire boundary.
#[derive(Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct ModelInputImage {
    /// IANA image media type. Only provider-portable raster formats are accepted.
    pub media_type: String,
    /// Standard, padded base64 payload without a `data:` URI prefix.
    pub data: String,
}

impl std::fmt::Debug for ModelInputImage {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ModelInputImage")
            .field("media_type", &self.media_type)
            .field(
                "data",
                &format_args!("[REDACTED; {} base64 bytes]", self.data.len()),
            )
            .finish()
    }
}

/// One ordered block in a provider-neutral model message.
#[derive(Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ModelInputBlock {
    /// UTF-8 text content.
    Text { text: String },
    /// Inline raster image content.
    Image { media_type: String, data: String },
}

impl std::fmt::Debug for ModelInputBlock {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Text { text } => formatter.debug_struct("Text").field("text", text).finish(),
            Self::Image { media_type, data } => formatter
                .debug_struct("Image")
                .field("media_type", media_type)
                .field(
                    "data",
                    &format_args!("[REDACTED; {} base64 bytes]", data.len()),
                )
                .finish(),
        }
    }
}

impl ModelInputBlock {
    /// Construct a text block.
    #[must_use]
    pub fn text(text: impl Into<String>) -> Self {
        Self::Text { text: text.into() }
    }

    /// Construct an inline image block.
    #[must_use]
    pub fn image(media_type: impl Into<String>, data: impl Into<String>) -> Self {
        Self::Image {
            media_type: media_type.into(),
            data: data.into(),
        }
    }
}

/// An ordered provider-neutral message used when a plain string cannot retain
/// the user's multimodal prompt semantics.
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct ModelInputMessage {
    pub role: MessageRole,
    pub content: Vec<ModelInputBlock>,
}

impl ModelInputMessage {
    /// Construct a structured message from ordered blocks.
    #[must_use]
    pub fn new(role: MessageRole, content: Vec<ModelInputBlock>) -> Self {
        Self { role, content }
    }
}

impl ModelInputImage {
    /// Construct an inline image. Call [`validate_model_input_images`] before dispatch.
    #[must_use]
    pub fn new(media_type: impl Into<String>, data: impl Into<String>) -> Self {
        Self {
            media_type: media_type.into(),
            data: data.into(),
        }
    }
}

/// Validate inline images before they enter a provider request.
///
/// Validation is intentionally conservative and provider-portable: it rejects
/// unsupported MIME types, malformed base64, excessive image counts, oversized
/// individual images, and oversized aggregate payloads.
pub fn validate_model_input_images(images: &[ModelInputImage]) -> std::result::Result<(), String> {
    if images.len() > MAX_MODEL_IMAGES {
        return Err(format!(
            "image count {} exceeds the per-request limit of {MAX_MODEL_IMAGES}",
            images.len()
        ));
    }

    let mut total_bytes = 0_usize;
    for (index, image) in images.iter().enumerate() {
        if !matches!(
            image.media_type.as_str(),
            "image/png" | "image/jpeg" | "image/gif" | "image/webp"
        ) {
            return Err(format!(
                "image {} has unsupported MIME type {:?}; supported types are image/png, image/jpeg, image/gif, and image/webp",
                index + 1,
                image.media_type
            ));
        }
        if image.data.is_empty() {
            return Err(format!("image {} has an empty base64 payload", index + 1));
        }

        // Bound encoded input before allocating the decoded buffer. Four base64
        // characters represent at most three bytes; allow one padded quartet.
        let max_encoded_len = MAX_MODEL_IMAGE_BYTES.div_ceil(3) * 4;
        if image.data.len() > max_encoded_len {
            return Err(format!(
                "image {} exceeds the decoded per-image limit of {MAX_MODEL_IMAGE_BYTES} bytes",
                index + 1
            ));
        }

        let decoded = base64::engine::general_purpose::STANDARD
            .decode(image.data.as_bytes())
            .map_err(|error| format!("image {} contains invalid base64: {error}", index + 1))?;
        if decoded.len() > MAX_MODEL_IMAGE_BYTES {
            return Err(format!(
                "image {} is {} decoded bytes, exceeding the per-image limit of {MAX_MODEL_IMAGE_BYTES}",
                index + 1,
                decoded.len()
            ));
        }
        total_bytes = total_bytes
            .checked_add(decoded.len())
            .ok_or_else(|| "aggregate image size overflowed".to_string())?;
        if total_bytes > MAX_MODEL_IMAGE_TOTAL_BYTES {
            return Err(format!(
                "aggregate decoded image size {total_bytes} exceeds the per-request limit of {MAX_MODEL_IMAGE_TOTAL_BYTES} bytes"
            ));
        }
    }
    Ok(())
}

/// Validate all inline images in an ordered structured message history.
pub fn validate_model_input_messages(
    messages: &[ModelInputMessage],
) -> std::result::Result<(), String> {
    for (message_index, message) in messages.iter().enumerate() {
        if message
            .content
            .iter()
            .any(|block| matches!(block, ModelInputBlock::Image { .. }))
            && message.role != MessageRole::User
        {
            return Err(format!(
                "message {} attaches an image to a non-user role",
                message_index + 1
            ));
        }
    }
    let images = messages
        .iter()
        .flat_map(|message| &message.content)
        .filter_map(|block| match block {
            ModelInputBlock::Image { media_type, data } => {
                Some(ModelInputImage::new(media_type.clone(), data.clone()))
            }
            ModelInputBlock::Text { .. } => None,
        })
        .collect::<Vec<_>>();
    validate_model_input_images(&images)
}

/// Primitive object kinds that can be composed by the authoring system.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObjectType {
    Agent,
    Extension,
    Connector,
    Gate,
    Feed,
    Recipe,
    Plan,
    Scorer,
    Arena,
    Group,
    Knowledge,
    Config,
}

// -- ModelCaller --

/// Request to call an LLM model.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct ModelCallRequest {
    /// Model identifier (e.g., "claude-sonnet-4-20250514").
    #[serde(default)]
    pub model: String,
    /// System prompt.
    #[serde(default)]
    pub system: Option<String>,
    /// User messages.
    #[serde(default)]
    pub messages: Vec<ChatMessage>,
    /// Provider-neutral structured message history for multimodal requests.
    ///
    /// Empty preserves the legacy text-only request shape. When non-empty,
    /// this is the authoritative ordered provider payload and adapters must
    /// fail closed rather than discard unsupported blocks.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub input_messages: Vec<ModelInputMessage>,
    /// Maximum tokens to generate.
    #[serde(default)]
    pub max_tokens: Option<u32>,
    /// Temperature (0.0-1.0).
    #[serde(default)]
    pub temperature: Option<f32>,
    /// Role for model routing.
    #[serde(default)]
    pub role: Option<String>,
    /// Caller surface that originated this request.
    #[serde(default)]
    pub caller: Option<String>,
    /// Workflow run identifier.
    #[serde(default)]
    pub run_id: Option<String>,
    /// Prompt sections included in the assembled prompt.
    #[serde(default)]
    pub prompt_section_ids: Vec<String>,
    /// Knowledge entries included in the prompt or routing decision.
    #[serde(default)]
    pub knowledge_ids: Vec<String>,
    /// Per-call token and cost budget.
    #[serde(default)]
    pub budget: Option<TokenBudget>,
    /// Remaining budget at call time.
    #[serde(default)]
    pub budget_remaining: Option<f64>,
    /// Hints for model routing.
    #[serde(default)]
    pub routing_hints: Vec<String>,
    /// Cache behavior for this request.
    #[serde(default)]
    pub cache_policy: CachePolicy,
    /// Tool definitions to include in the model call.
    /// Empty means no tools are sent.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tools: Vec<ToolDef>,
}

/// Caller-surface constants for `ModelCallRequest::caller`.
///
/// These are plain string constants rather than an enum because the `caller`
/// field is `Option<String>`. The constants prevent typos and serve as a
/// single source of truth for the set of recognised caller surfaces.
pub mod caller {
    pub const CLI: &str = "cli";
    pub const SERVE: &str = "serve";
    pub const RESEARCH: &str = "research";
    pub const DREAMS: &str = "dreams";
}

/// Cache behaviour for this request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum CachePolicy {
    /// Use the default L1 cache behaviour.
    #[default]
    Default,
    /// Skip cache lookup but still store the result.
    Bypass,
    /// Skip cache lookup AND discard any prior cached result for this key.
    ForceRefresh,
}

/// Token and cost budget for a single model call.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize, Default)]
pub struct TokenBudget {
    /// Maximum input tokens the gateway should accept.
    pub max_input: Option<u64>,
    /// Maximum output tokens requested from the provider.
    pub max_output: Option<u64>,
    /// Maximum cost in USD for this single call.
    pub max_cost_usd: Option<f64>,
}

/// Errors specific to the gateway pipeline (not provider errors).
#[derive(Debug, Clone, thiserror::Error)]
pub enum GatewayError {
    #[error("provider error: {0}")]
    ProviderError(String),
    #[error("budget exceeded: {detail}")]
    BudgetExceeded { detail: String },
    #[error("rate limited: retry after {retry_after_ms:?} ms")]
    RateLimited { retry_after_ms: Option<u64> },
    #[error("cache error: {0}")]
    CacheError(String),
    #[error("request cancelled")]
    Cancelled,
    #[error("convergence detected after {consecutive} identical outputs")]
    ConvergenceDetected { consecutive: u32 },
}

impl From<GatewayError> for RokoError {
    fn from(error: GatewayError) -> Self {
        // TODO(converge): Map to RokoError::Other once that variant exists in roko-core.
        RokoError::invalid(error.to_string())
    }
}

/// A single chat message.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ChatMessage {
    pub role: MessageRole,
    pub content: String,
}

/// Message role in a conversation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum MessageRole {
    System,
    User,
    Assistant,
}

/// Response from a model call.
#[derive(Debug, Clone)]
pub struct ModelCallResponse {
    pub content: String,
    pub model: String,
    pub usage: TokenUsage,
    pub stop_reason: Option<String>,
    /// Gateway request id, set by ModelCallService when the call flows through the gateway.
    pub request_id: Option<String>,
}

/// Token usage and cost from a model call.
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TokenUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub total_tokens: u64,
    pub cost_usd: f64,
}

/// Provider-agnostic stream event for model calls.
#[derive(Debug, Clone, PartialEq)]
pub enum ModelStreamEvent {
    /// Model dispatch has started for the selected model.
    Started {
        /// Selected model id.
        model: String,
    },
    /// Incremental assistant text.
    ContentDelta {
        /// Text delta content.
        text: String,
    },
    /// Token/cost usage became available.
    Usage {
        /// Usage observation for the call.
        usage: TokenUsage,
    },
    /// Model call completed successfully.
    Completed {
        /// Provider stop reason when available.
        stop_reason: Option<String>,
    },
    /// Model call failed.
    Failed {
        /// Failure message.
        error: String,
    },
    /// Model call was cancelled.
    Cancelled,
    /// One dispatch attempt failed before a final fallback/error.
    AttemptFailed {
        /// Attempted model id.
        model: String,
        /// Failure message for this attempt.
        error: String,
    },
}

/// Boxed model stream returned by streaming adapters.
pub type BoxModelStream = Pin<Box<dyn Stream<Item = ModelStreamEvent> + Send + 'static>>;

/// Convert a complete call response into the shared stream shape.
#[must_use]
pub fn model_call_response_to_stream(response: ModelCallResponse) -> BoxModelStream {
    Box::pin(futures_util::stream::iter(vec![
        ModelStreamEvent::Started {
            model: response.model.clone(),
        },
        ModelStreamEvent::ContentDelta {
            text: response.content,
        },
        ModelStreamEvent::Usage {
            usage: response.usage,
        },
        ModelStreamEvent::Completed {
            stop_reason: response.stop_reason,
        },
    ]))
}

/// Convert a failed call result into the shared stream shape.
#[must_use]
pub fn model_call_failure_to_stream(error: impl ToString) -> BoxModelStream {
    Box::pin(futures_util::stream::iter(vec![ModelStreamEvent::Failed {
        error: error.to_string(),
    }]))
}

/// Call an LLM model. Wraps provider selection, streaming, cost tracking.
#[async_trait]
pub trait ModelCaller: Send + Sync {
    /// Single-shot model call, returns complete response.
    async fn call(&self, req: ModelCallRequest) -> Result<ModelCallResponse>;

    /// Stream a model call through the shared provider-agnostic event shape.
    async fn stream(&self, req: ModelCallRequest) -> Result<BoxModelStream> {
        Ok(match self.call(req).await {
            Ok(response) => model_call_response_to_stream(response),
            Err(error) => model_call_failure_to_stream(error),
        })
    }
}

// -- PromptAssembler --

/// Specification for assembling a system prompt.
#[derive(Debug, Clone, Default)]
pub struct PromptSpec {
    /// Agent role (determines identity layer).
    pub role: Option<String>,
    /// Task description.
    pub task: Option<String>,
    /// Working directory for convention detection.
    pub workdir: Option<PathBuf>,
    /// Gate feedback from prior iterations.
    pub gate_feedback: Vec<String>,
    /// Anti-patterns to include.
    pub anti_patterns: Vec<String>,
}

/// Assemble a system prompt for a given role and context.
#[async_trait]
pub trait PromptAssembler: Send + Sync {
    /// Build a complete system prompt from the spec.
    async fn assemble(&self, spec: PromptSpec) -> Result<String>;

    /// Prompt section ids included by the most recent assembly.
    fn last_prompt_section_ids(&self) -> Vec<String>;

    /// Knowledge entry ids included by the most recent assembly.
    fn last_knowledge_ids(&self) -> Vec<String>;
}

// -- FeedbackSink --

/// A feedback event to record.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum FeedbackEvent {
    /// Feedback from a model call.
    ModelCall {
        #[serde(default)]
        run_id: Option<String>,
        #[serde(default)]
        request_id: Option<String>,
        #[serde(default)]
        prompt_section_ids: Vec<String>,
        #[serde(default)]
        knowledge_ids: Vec<String>,
        #[serde(default)]
        model: Option<String>,
        #[serde(default)]
        provider: Option<String>,
        #[serde(default)]
        token_usage: Option<u64>,
        #[serde(default)]
        cost: Option<f64>,
        role: String,
        input_tokens: u64,
        output_tokens: u64,
        cost_usd: f64,
        latency_ms: u64,
        success: bool,
    },
    /// Feedback from a gate execution.
    GateResult {
        run_id: String,
        gate_name: String,
        passed: bool,
        duration_ms: u64,
    },
    /// Feedback from a workflow completion.
    WorkflowComplete {
        event_type: String,
        run_id: String,
        model: Option<String>,
        success: bool,
        outcome: String,
        total_cost_usd: f64,
        total_tokens: u64,
        duration_ms: u64,
    },
}

/// Record feedback from model calls, gate results, and workflow outcomes.
#[async_trait]
pub trait FeedbackSink: Send + Sync {
    /// Record a feedback event.
    async fn record(&self, event: FeedbackEvent) -> Result<()>;

    /// Flush any buffered feedback events.
    async fn flush(&self) -> Result<()>;
}

// -- GateRunner --

/// Configuration for a gate run.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ShellGateCommand {
    /// Program to invoke.
    pub program: String,
    /// Args to pass.
    pub args: Vec<String>,
    /// Timeout in milliseconds.
    pub timeout_ms: u64,
}

/// Configuration for a gate run.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GateConfig {
    /// Working directory to verify.
    pub workdir: PathBuf,
    /// Which gates to run (e.g., ["compile", "test", "clippy"]).
    pub enabled_gates: Vec<String>,
    /// Configured shell commands, consumed by shell/custom:shell gate entries.
    pub shell_gates: Vec<ShellGateCommand>,
    /// Maximum rung to run (0-6).
    pub max_rung: Option<u8>,
}

/// Canonical metadata for a gate verdict.
///
/// The concrete gate implementation populates this metadata so lower-layer
/// consumers do not need to depend on a gate registry or reconstruct the
/// classification from a gate name.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct GateClassification {
    /// Canonical verification-pipeline rung, when the gate belongs to that
    /// pipeline. Standalone and unknown gates have no canonical rung.
    #[serde(default)]
    pub canonical_rung: Option<u8>,
    /// Whether the gate produces a deterministic binary result.
    #[serde(default)]
    pub deterministic: bool,
}

/// Result from a single gate.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GateVerdict {
    pub gate_name: String,
    /// Classification supplied by the injected gate runner.
    #[serde(default)]
    pub classification: GateClassification,
    /// True when the gate ran and succeeded.
    pub passed: bool,
    /// True when the gate did not run.
    #[serde(default)]
    pub skipped: bool,
    /// Why the gate did not run.
    #[serde(default)]
    pub skip_reason: Option<String>,
    pub output: String,
    pub duration_ms: u64,
}

/// Report from running a set of gates.
#[derive(Debug, Clone)]
pub struct GateReport {
    pub verdicts: Vec<GateVerdict>,
}

impl GateReport {
    /// Returns true if all gates passed.
    #[must_use]
    pub fn all_passed(&self) -> bool {
        self.verdicts.iter().all(|v| v.passed && !v.skipped)
    }

    /// Returns the first failing gate, if any.
    #[must_use]
    pub fn first_failure(&self) -> Option<&GateVerdict> {
        self.verdicts.iter().find(|v| !v.passed)
    }

    /// Collects all failure outputs for agent feedback.
    #[must_use]
    pub fn failure_summary(&self) -> String {
        self.verdicts
            .iter()
            .filter(|v| !v.passed)
            .map(|v| format!("{}: {}", v.gate_name, v.output))
            .collect::<Vec<_>>()
            .join("\n\n")
    }
}

/// Run a set of verification gates against a working directory.
#[async_trait]
pub trait GateRunner: Send + Sync {
    /// Execute gates per the config, returning a report.
    async fn run_gates(&self, config: GateConfig) -> Result<GateReport>;
}

// -- EventConsumer --

/// Consume RuntimeEvents for side-effects (logging, UI updates, etc).
///
/// Consumers must be non-blocking. If they need async work, they should
/// buffer internally and process asynchronously.
pub trait EventConsumer: Send + Sync {
    /// Called for each event emitted by the workflow engine.
    fn consume(&self, event: &RuntimeEvent);

    /// Consume an event and return its exact durable per-run byte cursor when
    /// the consumer owns that persistence boundary.
    ///
    /// Non-durable consumers keep the default behavior. Workflow publishers
    /// carry the first returned cursor on the live bus envelope so reconnecting
    /// clients can suppress replay/live overlap without persisting twice.
    fn consume_with_cursor(&self, event: &RuntimeEvent) -> Option<u64> {
        self.consume(event);
        None
    }
}

// -- EffectExecutor --

/// A side-effect the workflow engine needs to execute.
#[derive(Debug, Clone)]
pub enum Effect {
    /// Spawn an agent with the given role and prompt.
    SpawnAgent {
        run_id: String,
        role: String,
        model: String,
        system_prompt: String,
        user_prompt: String,
        workdir: PathBuf,
    },
    /// Run verification gates.
    RunGates { run_id: String, config: GateConfig },
    /// Create a git commit.
    Commit {
        run_id: String,
        workdir: PathBuf,
        message: String,
    },
    /// Persist a state checkpoint.
    Checkpoint {
        run_id: String,
        state_json: String,
        path: PathBuf,
    },
}

/// Outcome from executing an effect.
#[derive(Debug, Clone)]
pub enum EffectOutcome {
    /// Agent completed with output.
    AgentDone {
        agent_id: String,
        output: String,
        tokens_used: u64,
        cost_usd: f64,
        files_changed: Vec<String>,
    },
    /// Gates completed.
    GatesDone { report: GateReport },
    /// Commit created.
    CommitDone { hash: String, message: String },
    /// Checkpoint saved.
    CheckpointDone { path: String },
    /// Effect failed.
    Failed { error: String },
}

/// Execute a side-effect (spawn agent, run gates, commit, checkpoint).
///
/// The state machine decides WHAT to do; the EffectExecutor decides HOW.
#[async_trait]
pub trait EffectExecutor: Send + Sync {
    /// Execute the given effect, returning the outcome.
    async fn execute(&self, effect: Effect) -> Result<EffectOutcome>;
}

// -- AffectPolicy --

/// Behavioral state of the affect engine.
///
/// Mirrors `roko_core::BehavioralState` from `affect.rs` but is re-exported
/// here for self-contained trait signatures. Use the canonical enum from
/// `roko_core::BehavioralState` — do NOT duplicate the definition.
/// (The type is already `pub` in roko-core via `affect.rs`.)
/// Affect context snapshot provided before dispatching a task.
#[derive(Debug, Clone)]
pub struct AffectContext {
    /// Current behavioral state classification.
    pub behavioral_state: crate::BehavioralState,
    /// Current PAD vector: [Pleasure, Arousal, Dominance], each in [-1.0, 1.0].
    pub pad: [f32; 3],
    /// Human-readable emotional label, if available.
    pub emotional_tag: Option<String>,
}

/// Modulation parameters applied to dispatch configuration.
///
/// The affect policy fills these in; the effect driver applies them.
#[derive(Debug, Clone)]
pub struct DispatchModulation {
    /// Tier bias: -1.0 (prefer cheapest model) to +1.0 (prefer most capable model).
    pub tier_bias: f32,
    /// Multiplier on the default turn limit. 1.0 = no change.
    pub turn_limit_factor: f32,
    /// Exploration rate in [0.0, 1.0]. Higher = more exploratory routing.
    pub exploration_rate: f32,
}

impl Default for DispatchModulation {
    fn default() -> Self {
        Self {
            tier_bias: 0.0,
            turn_limit_factor: 1.0,
            exploration_rate: 0.0,
        }
    }
}

/// Policy trait for behavioral affect modulation in workflow execution.
///
/// The canonical implementation is `DaimonPolicy` in `roko-daimon`.
/// When affect is disabled, use `NoOpAffectPolicy` which returns neutral defaults.
#[async_trait]
pub trait AffectPolicy: Send + Sync {
    /// Called before dispatching a task. Returns an affect context snapshot.
    fn pre_dispatch(&self, task_id: &str, role: &str) -> AffectContext;

    /// Called after a task completes (success or failure).
    fn on_task_outcome(&mut self, task_id: &str, succeeded: bool, tokens_used: u64, cost_usd: f64);

    /// Called after a gate verdict.
    fn on_gate_result(&mut self, gate_name: &str, passed: bool, rung: u8, confidence: f64);

    /// Modulate dispatch parameters based on current affect state.
    fn modulate_dispatch(&self, role: &str, params: &mut DispatchModulation);

    /// Get the current behavioral state for logging/display.
    fn behavioral_state(&self) -> crate::BehavioralState;

    /// Persist affect state to disk.
    async fn persist(&self) -> Result<()>;
}

/// No-op implementation of `AffectPolicy` for when affect modulation is disabled.
///
/// All methods return neutral defaults. No state is tracked or persisted.
pub struct NoOpAffectPolicy;

#[async_trait]
impl AffectPolicy for NoOpAffectPolicy {
    fn pre_dispatch(&self, _task_id: &str, _role: &str) -> AffectContext {
        AffectContext {
            behavioral_state: crate::BehavioralState::Engaged,
            pad: [0.0, 0.0, 0.0],
            emotional_tag: None,
        }
    }

    fn on_task_outcome(
        &mut self,
        _task_id: &str,
        _succeeded: bool,
        _tokens_used: u64,
        _cost_usd: f64,
    ) {
    }

    fn on_gate_result(&mut self, _gate_name: &str, _passed: bool, _rung: u8, _confidence: f64) {}

    fn modulate_dispatch(&self, _role: &str, _params: &mut DispatchModulation) {}

    fn behavioral_state(&self) -> crate::BehavioralState {
        crate::BehavioralState::Engaged
    }

    async fn persist(&self) -> Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_util::StreamExt;

    fn tiny_png() -> ModelInputImage {
        ModelInputImage::new("image/png", "iVBORw0KGgo=")
    }

    #[test]
    fn model_input_images_validate_portable_payloads() {
        assert!(validate_model_input_images(&[tiny_png()]).is_ok());
        assert!(validate_model_input_images(&[]).is_ok());
    }

    #[test]
    fn model_input_images_fail_closed_on_mime_base64_count_and_size() {
        let mut invalid_mime = tiny_png();
        invalid_mime.media_type = "image/svg+xml".to_string();
        assert!(validate_model_input_images(&[invalid_mime]).is_err());

        let invalid_base64 = ModelInputImage::new("image/png", "not base64!");
        assert!(validate_model_input_images(&[invalid_base64]).is_err());

        assert!(validate_model_input_images(&vec![tiny_png(); MAX_MODEL_IMAGES + 1]).is_err());

        let oversized = ModelInputImage::new(
            "image/png",
            "A".repeat(MAX_MODEL_IMAGE_BYTES.div_ceil(3) * 4 + 4),
        );
        assert!(validate_model_input_images(&[oversized]).is_err());
    }

    #[test]
    fn model_input_messages_reject_non_user_images_and_redact_payloads() {
        let payload = "c2Vuc2l0aXZlLWltYWdlLWJ5dGVz";
        let message = ModelInputMessage::new(
            MessageRole::Assistant,
            vec![ModelInputBlock::image("image/png", payload)],
        );

        assert!(validate_model_input_messages(&[message.clone()]).is_err());
        let debug = format!("{message:?}");
        assert!(!debug.contains(payload));
        assert!(debug.contains("REDACTED"));
    }

    #[test]
    fn model_input_images_enforce_aggregate_decoded_limit() {
        let decoded = vec![0_u8; MAX_MODEL_IMAGE_BYTES];
        let encoded = base64::engine::general_purpose::STANDARD.encode(decoded);
        let images = (0..5)
            .map(|_| ModelInputImage::new("image/png", encoded.clone()))
            .collect::<Vec<_>>();

        assert!(validate_model_input_images(&images).is_err());
    }

    #[derive(Clone)]
    struct StubModelCaller {
        response: std::result::Result<ModelCallResponse, String>,
    }

    #[async_trait]
    impl ModelCaller for StubModelCaller {
        async fn call(&self, _req: ModelCallRequest) -> Result<ModelCallResponse> {
            self.response.clone().map_err(RokoError::invalid)
        }
    }

    #[tokio::test]
    async fn model_stream_maps_successful_call_response() {
        let usage = TokenUsage {
            input_tokens: 3,
            output_tokens: 5,
            total_tokens: 8,
            cost_usd: 0.25,
        };
        let caller = StubModelCaller {
            response: Ok(ModelCallResponse {
                content: "hello".to_string(),
                model: "model-a".to_string(),
                usage: usage.clone(),
                stop_reason: Some("end_turn".to_string()),
                request_id: Some("req-1".to_string()),
            }),
        };

        let events = caller
            .stream(ModelCallRequest::default())
            .await
            .unwrap()
            .collect::<Vec<_>>()
            .await;

        assert_eq!(
            events,
            vec![
                ModelStreamEvent::Started {
                    model: "model-a".to_string()
                },
                ModelStreamEvent::ContentDelta {
                    text: "hello".to_string()
                },
                ModelStreamEvent::Usage { usage },
                ModelStreamEvent::Completed {
                    stop_reason: Some("end_turn".to_string())
                }
            ]
        );
    }

    #[tokio::test]
    async fn model_stream_maps_failed_call_response() {
        let caller = StubModelCaller {
            response: Err("provider unavailable".to_string()),
        };

        let events = caller
            .stream(ModelCallRequest::default())
            .await
            .unwrap()
            .collect::<Vec<_>>()
            .await;

        assert_eq!(
            events,
            vec![ModelStreamEvent::Failed {
                error: "invalid input: provider unavailable".to_string()
            }]
        );
    }

    #[test]
    fn object_type_contract_contains_twelve_unique_primitives() {
        let all = [
            ObjectType::Agent,
            ObjectType::Extension,
            ObjectType::Connector,
            ObjectType::Gate,
            ObjectType::Feed,
            ObjectType::Recipe,
            ObjectType::Plan,
            ObjectType::Scorer,
            ObjectType::Arena,
            ObjectType::Group,
            ObjectType::Knowledge,
            ObjectType::Config,
        ];
        assert_eq!(
            all.into_iter()
                .collect::<std::collections::HashSet<_>>()
                .len(),
            12
        );
        assert_eq!(
            serde_json::to_value(ObjectType::Knowledge).expect("serialize object type"),
            "knowledge"
        );
    }
}
