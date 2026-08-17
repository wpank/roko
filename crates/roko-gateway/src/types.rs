//! Provider-neutral inference protocol types.

use std::ops::{Add, AddAssign};

use roko_core::foundation::{ChatMessage, MessageRole};
use roko_core::tool::ToolDef;
use serde::{Deserialize, Serialize};

/// Stable agent identifier at the gateway boundary.
pub type AgentId = String;
/// Canonical message type shared with the workflow foundation.
pub type Message = ChatMessage;
/// Canonical tool definition shared with the tool registry.
pub type ToolSchema = ToolDef;

/// Cognitive tier associated with an inference request.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Tier {
    /// Fast, mechanical inference.
    T0,
    /// Standard deliberative inference.
    #[default]
    T1,
    /// Reflective, high-complexity inference.
    T2,
}

/// Coarse operating regime used to select an exact-cache TTL.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CacheRegime {
    /// Standard operating conditions.
    #[default]
    Normal,
    /// Low-volatility operation.
    Calm,
    /// Rapidly changing context.
    Volatile,
    /// Active failures or incident response.
    Crisis,
}

/// One observed tool call supplied by an agent loop.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolCallObservation {
    /// Canonical tool name.
    pub tool_name: String,
    /// Provider-neutral argument payload.
    pub arguments: serde_json::Value,
}

impl ToolCallObservation {
    /// Content-address the argument payload for loop detection.
    #[must_use]
    pub fn arguments_hash(&self) -> [u8; 32] {
        *blake3::hash(self.arguments.to_string().as_bytes()).as_bytes()
    }
}

/// Metadata used for routing, tenancy, accounting, and progress detection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InferenceMeta {
    /// Conversation/session identity.
    pub session_id: String,
    /// Calling agent identity.
    pub agent_id: AgentId,
    /// Requested cognitive tier.
    pub tier: Tier,
    /// Budget remaining at dispatch time, in microdollars.
    pub budget_remaining: u64,
    /// Cache namespace. Different namespaces never share semantic hits.
    #[serde(default = "default_namespace")]
    pub namespace: String,
    /// Current cache regime.
    #[serde(default)]
    pub regime: CacheRegime,
    /// Whether batch pricing applies.
    #[serde(default)]
    pub is_batch: bool,
    /// Broad task category hint for CascadeRouter.
    #[serde(default)]
    pub task_category: Option<String>,
    /// Complexity-band hint for CascadeRouter.
    #[serde(default)]
    pub complexity: Option<String>,
    /// Agent-role hint for CascadeRouter.
    #[serde(default)]
    pub agent_role: Option<String>,
    /// Current loop iteration.
    #[serde(default = "default_iteration")]
    pub iteration: u32,
    /// Tool calls made since the previous gateway request.
    #[serde(default)]
    pub tool_calls: Vec<ToolCallObservation>,
    /// Hash or identity of newly observed tool-result content.
    #[serde(default)]
    pub progress_marker: Option<String>,
}

impl Default for InferenceMeta {
    fn default() -> Self {
        Self {
            session_id: String::new(),
            agent_id: String::new(),
            tier: Tier::default(),
            budget_remaining: 0,
            namespace: default_namespace(),
            regime: CacheRegime::default(),
            is_batch: false,
            task_category: None,
            complexity: None,
            agent_role: None,
            iteration: default_iteration(),
            tool_calls: Vec::new(),
            progress_marker: None,
        }
    }
}

fn default_namespace() -> String {
    "default".to_string()
}

const fn default_iteration() -> u32 {
    1
}

/// Extended-thinking activation mode.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ThinkingMode {
    /// Extended thinking is enabled.
    Enabled,
    /// Extended thinking is disabled.
    #[default]
    Disabled,
}

/// Provider-neutral extended-thinking configuration.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThinkingConfig {
    /// Activation mode (`type` on provider wire formats).
    #[serde(rename = "type")]
    pub kind: ThinkingMode,
    /// Explicit token budget, if supplied by the caller.
    #[serde(default)]
    pub budget_tokens: Option<u32>,
}

/// A request entering the inference gateway.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct InferenceRequest {
    /// Model hint or explicit model slug. Empty/`auto` delegates to CascadeRouter.
    #[serde(default)]
    pub model: String,
    /// Ordered conversation messages.
    pub messages: Vec<Message>,
    /// Maximum generated tokens.
    #[serde(default)]
    pub max_tokens: Option<u32>,
    /// Sampling temperature.
    #[serde(default)]
    pub temperature: Option<f32>,
    /// Callable tool definitions.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<ToolSchema>>,
    /// Whether the caller requested incremental delivery.
    #[serde(default)]
    pub stream: bool,
    /// Extended-thinking configuration.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thinking: Option<ThinkingConfig>,
    /// Routing, attribution, and budget metadata.
    pub metadata: InferenceMeta,
}

impl InferenceRequest {
    /// Prepend one-shot guidance to the first system message, or insert one.
    pub fn prepend_system_guidance(&mut self, guidance: &str) {
        if guidance.trim().is_empty() {
            return;
        }
        if let Some(system) = self
            .messages
            .iter_mut()
            .find(|message| message.role == MessageRole::System)
        {
            system.content = format!("{guidance}\n\n{}", system.content);
        } else {
            self.messages.insert(
                0,
                Message {
                    role: MessageRole::System,
                    content: guidance.to_string(),
                },
            );
        }
    }

    /// Text used for semantic caching and convergence fingerprints.
    #[must_use]
    pub fn semantic_text(&self) -> String {
        self.messages
            .iter()
            .map(|message| message.content.as_str())
            .collect::<Vec<_>>()
            .join("\n")
    }
}

/// Why a provider stopped producing tokens.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StopReason {
    /// Provider completed the assistant turn.
    #[default]
    EndTurn,
    /// Output limit was reached.
    MaxTokens,
    /// Provider emitted a tool invocation.
    ToolUse,
    /// Provider content policy stopped generation.
    ContentFilter,
}

/// Detailed provider token accounting.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenUsage {
    /// Input/prompt tokens.
    pub input_tokens: u64,
    /// Generated tokens, including provider-reported reasoning when applicable.
    pub output_tokens: u64,
    /// Input tokens served from provider prefix cache.
    pub cache_read_input_tokens: u64,
    /// Input tokens written to provider prefix cache.
    pub cache_creation_input_tokens: u64,
    /// Anthropic-style extended-thinking tokens.
    pub thinking_tokens: u64,
    /// OpenAI-style reasoning tokens.
    pub reasoning_tokens: u64,
}

impl Add for TokenUsage {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            input_tokens: self.input_tokens.saturating_add(rhs.input_tokens),
            output_tokens: self.output_tokens.saturating_add(rhs.output_tokens),
            cache_read_input_tokens: self
                .cache_read_input_tokens
                .saturating_add(rhs.cache_read_input_tokens),
            cache_creation_input_tokens: self
                .cache_creation_input_tokens
                .saturating_add(rhs.cache_creation_input_tokens),
            thinking_tokens: self.thinking_tokens.saturating_add(rhs.thinking_tokens),
            reasoning_tokens: self.reasoning_tokens.saturating_add(rhs.reasoning_tokens),
        }
    }
}

impl AddAssign for TokenUsage {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

/// A completed provider response.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct InferenceResponse {
    /// Assistant text.
    pub text: String,
    /// Provider stop reason.
    pub stop_reason: StopReason,
    /// Detailed usage.
    pub usage: TokenUsage,
    /// Model that served the request.
    pub model: String,
    /// End-to-end latency.
    pub latency_ms: u64,
    /// Whether a fallback served the request.
    #[serde(default)]
    pub fallback: bool,
    /// Originally selected model when fallback occurred.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub original_model: Option<String>,
}

/// One incremental inference-stream item.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct InferenceChunk {
    /// Text delta, if any.
    #[serde(default)]
    pub delta: String,
    /// Usage update, normally present on the final item.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub usage: Option<TokenUsage>,
    /// Stop reason, normally present on the final item.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stop_reason: Option<StopReason>,
    /// Model producing the stream.
    #[serde(default)]
    pub model: String,
    /// True on the terminal item.
    #[serde(default)]
    pub done: bool,
}
