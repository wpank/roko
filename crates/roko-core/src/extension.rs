//! Extension trait for composable agent behavior.
//!
//! Extensions hook into 8 layers of the agent tick pipeline. Each hook has
//! a default no-op implementation so extensions only override what they need.
//!
//! # Layers (execution order)
//!
//! | Layer | # | Hooks | Purpose |
//! |-------|---|-------|---------|
//! | Foundation | 0 | `on_init`, `on_shutdown` | Lifecycle setup/teardown |
//! | Perception | 1 | `on_observe`, `on_filter` | Raw input processing |
//! | Memory | 2 | `on_retrieve`, `on_store` | Knowledge access |
//! | Cognition | 3 | `pre_inference`, `post_inference`, `on_gate` | LLM interaction |
//! | Action | 4 | `pre_action`, `post_action`, `on_tool_call` | Tool execution |
//! | Social | 5 | `on_message_send`, `on_message_receive` | Inter-agent |
//! | Meta | 6 | `on_reflect`, `on_cost_update` | Self-monitoring |
//! | Recovery | 7 | `on_error` | Fault handling |
//! | Cross-cutting | — | `on_tick_start`, `on_tick_end`, `on_slot_assigned`, `on_slot_completed` | All layers |
//!
//! Extensions are loaded from `roko.toml` under `[agent.extensions]` and
//! `[agent.roles.<role>.extensions]`.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

// ── CaMeL IFC types (E30-T01) ─────────────────────────────────────────

/// Security trust tier for CaMeL information-flow control.
///
/// Ordered from most trusted to least: `Trusted < Local < External < Untrusted`.
/// A tag's taint level never decreases as data flows through handlers.
///
/// Distinct from [`crate::TaintLevel`] (Public/Internal/Confidential/Secret
/// classification). `CamelTaintLevel` tracks the *trust origin* of capability-
/// bearing data flowing through extension hooks.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CamelTaintLevel {
    /// In-tree, fully audited code paths.
    Trusted = 0,
    /// Local user config or local-disk extensions.
    Local = 1,
    /// Network APIs or remote agents.
    External = 2,
    /// Untrusted third-party input (e.g. tool output, user data).
    Untrusted = 3,
}

impl Default for CamelTaintLevel {
    fn default() -> Self {
        Self::Trusted
    }
}

impl std::fmt::Display for CamelTaintLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Trusted => write!(f, "trusted"),
            Self::Local => write!(f, "local"),
            Self::External => write!(f, "external"),
            Self::Untrusted => write!(f, "untrusted"),
        }
    }
}

/// How a handler transformed the data flowing through it.
///
/// Recorded in each [`ProvenanceEntry`] so consumers can reconstruct the full
/// transformation history of a [`CamelTag`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TagOperation {
    /// Data passed through unchanged.
    Passthrough,
    /// Data was modified or enriched by this handler.
    Transform,
    /// Data was merged from multiple upstream tags.
    Merge,
}

impl std::fmt::Display for TagOperation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Passthrough => write!(f, "passthrough"),
            Self::Transform => write!(f, "transform"),
            Self::Merge => write!(f, "merge"),
        }
    }
}

/// A single entry in a [`CamelTag`]'s chain of custody.
///
/// Records which handler touched the data, when, and what it did to it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProvenanceEntry {
    /// Name of the handler (extension name or internal subsystem).
    pub handler: String,
    /// Wall-clock time at which this handler processed the data.
    pub timestamp: DateTime<Utc>,
    /// What the handler did to the data.
    pub operation: TagOperation,
}

impl ProvenanceEntry {
    /// Construct a new entry stamped with the current UTC time.
    pub fn now(handler: impl Into<String>, operation: TagOperation) -> Self {
        Self {
            handler: handler.into(),
            timestamp: Utc::now(),
            operation,
        }
    }
}

impl std::fmt::Display for ProvenanceEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}@{}({})",
            self.handler,
            self.timestamp.format("%Y-%m-%dT%H:%M:%SZ"),
            self.operation,
        )
    }
}

/// A set of capability strings granted to a data flow.
///
/// `intersection` implements the conservative CaMeL rule: when data flows
/// through a handler, only capabilities held by *both* parties survive.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilitySet {
    /// Capability strings (e.g. `"read_disk"`, `"call_llm"`).
    pub capabilities: HashSet<String>,
}

impl CapabilitySet {
    /// Construct an empty capability set.
    pub fn empty() -> Self {
        Self::default()
    }

    /// Construct from an iterator of strings.
    pub fn from_strings<I>(iter: I) -> Self
    where
        I: IntoIterator,
        I::Item: Into<String>,
    {
        Self {
            capabilities: iter.into_iter().map(Into::into).collect(),
        }
    }

    /// Capabilities present in *both* sets (conservative propagation).
    #[must_use]
    pub fn intersection(&self, other: &Self) -> Self {
        Self {
            capabilities: self
                .capabilities
                .intersection(&other.capabilities)
                .cloned()
                .collect(),
        }
    }

    /// Capabilities present in *either* set.
    #[must_use]
    pub fn union(&self, other: &Self) -> Self {
        Self {
            capabilities: self
                .capabilities
                .union(&other.capabilities)
                .cloned()
                .collect(),
        }
    }

    /// Number of capabilities.
    pub fn len(&self) -> usize {
        self.capabilities.len()
    }

    /// Whether the set is empty.
    pub fn is_empty(&self) -> bool {
        self.capabilities.is_empty()
    }
}

impl std::fmt::Display for CapabilitySet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut caps: Vec<&str> = self.capabilities.iter().map(String::as_str).collect();
        caps.sort_unstable();
        write!(f, "{{{}}}", caps.join(", "))
    }
}

/// CaMeL information-flow control tag.
///
/// Tracks the capability provenance of values flowing through extension hooks:
///
/// - **capabilities**: what operations this data grants/requires.
/// - **provenance**: ordered chain of handlers that have processed this tag.
/// - **taint_level**: trust origin — can only *increase* (trust can only *decrease*).
///
/// The no-elevation rule: untrusted data cannot be laundered into trusted data
/// by passing it through an intermediate handler.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CamelTag {
    /// Capabilities associated with this data flow.
    pub capabilities: CapabilitySet,
    /// Ordered chain of handlers that have processed this tag.
    pub provenance: Vec<ProvenanceEntry>,
    /// Trust classification — can only increase (worsen) over time.
    pub taint_level: CamelTaintLevel,
}

impl CamelTag {
    /// Construct a fresh tag with no provenance history.
    pub fn new<I>(capabilities: I, taint_level: CamelTaintLevel) -> Self
    where
        I: IntoIterator,
        I::Item: Into<String>,
    {
        Self {
            capabilities: CapabilitySet::from_strings(capabilities),
            provenance: Vec::new(),
            taint_level,
        }
    }

    /// Construct a fully trusted tag with the given capabilities.
    pub fn trusted<I>(capabilities: I) -> Self
    where
        I: IntoIterator,
        I::Item: Into<String>,
    {
        Self::new(capabilities, CamelTaintLevel::Trusted)
    }

    /// Propagate this tag through `handler`, appending a provenance entry.
    ///
    /// The taint level is never decreased. Returns a new `CamelTag`; the
    /// original is unchanged.
    #[must_use]
    pub fn propagate(&self, handler: &str, operation: TagOperation) -> CamelTag {
        let mut provenance = self.provenance.clone();
        provenance.push(ProvenanceEntry::now(handler, operation));
        CamelTag {
            capabilities: self.capabilities.clone(),
            provenance,
            taint_level: self.taint_level,
        }
    }

    /// Merge multiple tags into one.
    ///
    /// - **Capabilities**: intersection of all inputs (conservative).
    /// - **Taint level**: maximum (worst) of all inputs.
    /// - **Provenance**: concatenation in input order.
    ///
    /// An empty slice returns a fully-trusted, empty-capability tag.
    #[must_use]
    pub fn merge(tags: &[&CamelTag]) -> CamelTag {
        if tags.is_empty() {
            return CamelTag {
                capabilities: CapabilitySet::empty(),
                provenance: Vec::new(),
                taint_level: CamelTaintLevel::Trusted,
            };
        }

        let capabilities = tags
            .iter()
            .map(|t| t.capabilities.clone())
            .reduce(|acc, c| acc.intersection(&c))
            .unwrap_or_default();

        let taint_level = tags
            .iter()
            .map(|t| t.taint_level)
            .max()
            .unwrap_or(CamelTaintLevel::Trusted);

        let provenance = tags
            .iter()
            .flat_map(|t| t.provenance.iter().cloned())
            .collect();

        CamelTag {
            capabilities,
            provenance,
            taint_level,
        }
    }
}

impl Default for CamelTag {
    fn default() -> Self {
        Self {
            capabilities: CapabilitySet::empty(),
            provenance: Vec::new(),
            taint_level: CamelTaintLevel::Trusted,
        }
    }
}

impl std::fmt::Display for CamelTag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "CamelTag(taint={}, caps={}, provenance=[{}])",
            self.taint_level,
            self.capabilities,
            self.provenance
                .iter()
                .map(|p| p.to_string())
                .collect::<Vec<_>>()
                .join(", "),
        )
    }
}

// ── Typed hook parameter structs (C2) ─────────────────────────────────

/// Pre-inference hook context. Passed mutably so extensions can modify
/// the request before it reaches the LLM.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InferenceRequest {
    /// Plan this inference belongs to.
    pub plan_id: String,
    /// Task being executed.
    pub task: String,
    /// Agent role (e.g. "engineer", "reviewer").
    pub role: String,
    /// Model being called (e.g. "claude-sonnet-4-20250514").
    pub model: String,
    /// Estimated prompt token count.
    pub prompt_tokens: usize,
    /// Escape hatch for truly dynamic / extension-specific data.
    pub extra: serde_json::Value,
}

/// Post-inference hook context. Passed mutably so extensions can annotate
/// or transform the response.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InferenceResponse {
    /// Plan this inference belongs to.
    pub plan_id: String,
    /// Task that was executed.
    pub task: String,
    /// Agent role.
    pub role: String,
    /// Model that was called.
    pub model: String,
    /// Whether the inference succeeded.
    pub success: bool,
    /// Estimated cost in USD.
    pub cost_usd: f64,
    /// Wall-clock duration in milliseconds.
    pub wall_ms: u64,
    /// Escape hatch for truly dynamic / extension-specific data.
    pub extra: serde_json::Value,
}

/// Verify evaluation result passed to `on_gate`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GateEvent {
    /// Plan the gate belongs to.
    pub plan_id: String,
    /// Verify that ran (e.g. "compile", "test", "clippy").
    pub gate_name: String,
    /// Whether the gate passed.
    pub passed: bool,
    /// Verify rung (e.g. "rung-1", "rung-3").
    pub rung: String,
    /// How long the gate took in milliseconds.
    pub duration_ms: u64,
    /// Verify-specific details (diagnostics, counts, etc.).
    pub details: serde_json::Value,
}

/// Error context for recovery hooks.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ErrorEvent {
    /// Human-readable error message.
    pub error_message: String,
    /// Where the error originated (e.g. "agent_dispatch", "gate_pipeline").
    pub source: String,
    /// Escape hatch for extension-specific context.
    pub extra: serde_json::Value,
}

/// Generic observation for the perception layer.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Observation {
    /// Where this observation came from.
    pub source: String,
    /// The observation payload.
    pub data: serde_json::Value,
}

/// Tool call event for action-layer hooks.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolCallEvent {
    /// Name of the tool being invoked.
    pub tool_name: String,
    /// Arguments passed to the tool.
    pub arguments: serde_json::Value,
    /// Result of the tool call, if available (post-action only).
    pub result: Option<serde_json::Value>,
}

/// Cost update event for the meta layer.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CostUpdate {
    /// Model that incurred the cost.
    pub model: String,
    /// Input tokens consumed.
    pub tokens_in: u64,
    /// Output tokens produced.
    pub tokens_out: u64,
    /// Cost in USD.
    pub cost_usd: f64,
}

/// Inter-agent message for social-layer hooks.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AgentMessage {
    /// Sender agent name.
    pub from: String,
    /// Recipient agent name.
    pub to: String,
    /// Message payload.
    pub payload: serde_json::Value,
}

/// Memory retrieval context.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RetrievalResult {
    /// The query that was issued.
    pub query: String,
    /// Retrieved entries (mutable so extensions can augment).
    pub entries: Vec<serde_json::Value>,
}

/// Memory store context.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StoreEntry {
    /// Key or topic being stored.
    pub key: String,
    /// The entry payload.
    pub data: serde_json::Value,
}

/// Reflection state for the meta layer.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReflectionState {
    /// Current agent state snapshot.
    pub state: serde_json::Value,
}

// ── Existing enums & structs ──────────────────────────────────────────

/// Layer in the agent tick pipeline where an extension runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExtensionLayer {
    /// Lifecycle: init/shutdown.
    Foundation = 0,
    /// Input processing: observe/filter.
    Perception = 1,
    /// Knowledge store: retrieve/store.
    Memory = 2,
    /// LLM interaction: pre/post inference, gating.
    Cognition = 3,
    /// Tool execution: pre/post action, tool calls.
    Action = 4,
    /// Inter-agent messaging.
    Social = 5,
    /// Self-monitoring: reflect, cost tracking.
    Meta = 6,
    /// Fault handling and recovery.
    Recovery = 7,
}

/// What to do before an action executes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ActionDecision {
    /// Allow the action to proceed.
    Proceed,
    /// Block the action with an explanation.
    Block(String),
    /// Rewrite the action with a modified version.
    Rewrite(String),
}

/// What to do when a tool is called.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ToolDecision {
    /// Allow the tool call.
    Allow,
    /// Deny the tool call with a reason.
    Deny(String),
    /// Allow but with modified arguments.
    Rewrite(String),
}

/// What to do when an error occurs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RecoveryAction {
    /// Propagate the error up.
    Propagate,
    /// Retry the failed operation.
    Retry,
    /// Skip the failed step and continue.
    Skip,
    /// Substitute a fallback value.
    Fallback(String),
}

/// An adjustment suggested during reflection.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Adjustment {
    /// What is being adjusted.
    pub target: String,
    /// The adjustment to make.
    pub value: serde_json::Value,
    /// Confidence in this adjustment (0.0-1.0).
    pub confidence: f64,
}

/// Metadata about a loaded extension.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExtensionMeta {
    /// Unique extension name.
    pub name: String,
    /// Layer this extension operates in.
    pub layer: ExtensionLayer,
    /// Whether failure in this extension is fatal.
    #[serde(default)]
    pub optional: bool,
    /// Hard dependencies: other extension names that must load first.
    /// Cross-layer references are silently ignored — layer order handles them.
    #[serde(default)]
    pub depends_on: Vec<String>,
    /// Soft dependencies: extensions to order before this one if present,
    /// but whose absence does not cause an error or affect ordering.
    #[serde(default)]
    pub soft_depends_on: Vec<String>,
    /// Extension version.
    #[serde(default)]
    pub version: String,
}

/// Sandboxing/distribution tier for an extension package.
///
/// Determines the isolation level and trust granted to an extension.
/// Tiers are ordered from least trusted (Prompts) to most trusted (NativeRust).
///
/// See v2 spec §3 (12-EXTENSIONS.md) for the full 5-tier SPI table.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PackageTier {
    /// Markdown/TOML front-matter declaring hook behavior (no execution).
    Prompts,
    /// TOML profile bundles that configure built-in extensions.
    Config,
    /// TOML manifests for subprocess/HTTP/MCP hooks (OS process isolation).
    Declarative,
    /// Compiled WASM implementing extension hooks (fuel-metered sandbox).
    Wasm,
    /// `impl Extension` compiled in-tree (process-level trust).
    NativeRust,
}

/// Full extension manifest as authored in TOML.
///
/// Extends [`ExtensionMeta`] with packaging metadata from the v2 spec (§3).
/// The `tier` field determines the sandboxing/execution model; WASM and
/// NativeRust tiers require additional runtime support beyond this struct.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExtensionManifest {
    /// Unique extension name.
    pub name: String,
    /// Semantic version string.
    pub version: String,
    /// Human-readable description of what this extension does.
    #[serde(default)]
    pub description: String,
    /// Layer this extension operates in.
    pub layer: ExtensionLayer,
    /// Other extension names that must be loaded before this one.
    #[serde(default)]
    pub depends_on: Vec<String>,
    /// Whether failure to load/run this extension is non-fatal.
    #[serde(default)]
    pub optional: bool,
    /// Categorization tags (e.g. "observability", "security").
    #[serde(default)]
    pub tags: Vec<String>,
    /// Packaging and sandboxing tier.
    pub tier: PackageTier,
    /// Per-hook timeout override in milliseconds (overrides chain default).
    #[serde(default)]
    pub timeout_ms: Option<u64>,
    /// Extension-specific configuration.
    #[serde(default)]
    pub config: serde_json::Value,
}

// ── ExtensionManifest validation ──────────────────────────────────────

/// Error returned when an [`ExtensionManifest`] fails validation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ManifestValidationError {
    /// Human-readable description of what failed.
    pub message: String,
}

impl std::fmt::Display for ManifestValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "invalid extension manifest: {}", self.message)
    }
}

impl std::error::Error for ManifestValidationError {}

impl ExtensionManifest {
    /// Validate the manifest, returning an error if any required field is
    /// missing or has an invalid format.
    ///
    /// Validation rules:
    /// - `name` must be non-empty and contain only alphanumeric chars, `-`, and `_`
    /// - `version` must be non-empty and follow `MAJOR.MINOR.PATCH` semver format
    pub fn validate(&self) -> Result<(), ManifestValidationError> {
        if self.name.is_empty() {
            return Err(ManifestValidationError {
                message: "name must not be empty".to_string(),
            });
        }
        if !self
            .name
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
        {
            return Err(ManifestValidationError {
                message: format!(
                    "name `{}` contains invalid characters \
                     (only alphanumeric, '-', '_' are allowed)",
                    self.name
                ),
            });
        }

        if self.version.is_empty() {
            return Err(ManifestValidationError {
                message: "version must not be empty".to_string(),
            });
        }
        if !is_valid_semver(&self.version) {
            return Err(ManifestValidationError {
                message: format!(
                    "version `{}` is not valid semver (expected MAJOR.MINOR.PATCH)",
                    self.version
                ),
            });
        }

        Ok(())
    }

    /// Convert this manifest into an [`ExtensionMeta`] for use by the runtime.
    pub fn into_meta(self) -> ExtensionMeta {
        ExtensionMeta {
            name: self.name,
            layer: self.layer,
            optional: self.optional,
            depends_on: self.depends_on,
            soft_depends_on: Vec::new(),
            version: self.version,
        }
    }
}

/// Returns `true` if `v` is a valid semver string (`MAJOR.MINOR.PATCH`
/// with an optional pre-release suffix separated by `-`).
fn is_valid_semver(v: &str) -> bool {
    let core = v.split('-').next().unwrap_or("");
    let parts: Vec<&str> = core.split('.').collect();
    if parts.len() != 3 {
        return false;
    }
    parts
        .iter()
        .all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()))
}

// ── Extension trait (async + typed parameters) ────────────────────────

/// Extension trait for composable agent behavior.
///
/// All hooks have default no-op implementations. Extensions only override
/// the hooks they need. Hooks are called in layer order (Foundation first,
/// Recovery last), and within a layer in the order extensions are listed
/// in the configuration.
///
/// All hooks are async (E1) and use typed parameter structs (C2) instead
/// of raw `serde_json::Value`.
///
/// # Error handling
///
/// If an extension hook returns `Err`, the error is:
/// - Logged and ignored if `optional = true`
/// - Propagated to the caller if `optional = false` (default)
#[async_trait::async_trait]
pub trait Extension: Send + Sync {
    /// Unique name identifying this extension.
    fn name(&self) -> &str;

    /// Which layer this extension belongs to.
    fn layer(&self) -> ExtensionLayer;

    /// Metadata for this extension (name, layer, optional, dependencies).
    fn meta(&self) -> ExtensionMeta {
        ExtensionMeta {
            name: self.name().to_string(),
            layer: self.layer(),
            optional: false,
            depends_on: Vec::new(),
            soft_depends_on: Vec::new(),
            version: String::new(),
        }
    }

    // ── Foundation (Layer 0) ────────────────────────────────────────

    /// Called once when the agent starts. Use for setup, connections, etc.
    async fn on_init(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        Ok(())
    }

    /// Called once when the agent shuts down. Use for cleanup.
    async fn on_shutdown(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        Ok(())
    }

    // ── Perception (Layer 1) ────────────────────────────────────────

    /// Called when new observations arrive. Can enrich or annotate them.
    async fn on_observe(
        &self,
        _observation: &mut Observation,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        Ok(())
    }

    /// Called to filter observations before they reach cognition.
    async fn on_filter(
        &self,
        _observations: &mut Vec<Observation>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        Ok(())
    }

    // ── Memory (Layer 2) ────────────────────────────────────────────

    /// Called when retrieving from knowledge store. Can augment results.
    async fn on_retrieve(
        &self,
        _results: &mut RetrievalResult,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        Ok(())
    }

    /// Called when storing to knowledge. Can transform or filter.
    async fn on_store(
        &self,
        _entry: &mut StoreEntry,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        Ok(())
    }

    // ── Cognition (Layer 3) ─────────────────────────────────────────

    /// Called before sending a request to the LLM. Can modify the request.
    async fn pre_inference(
        &self,
        _request: &mut InferenceRequest,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        Ok(())
    }

    /// Called after receiving a response from the LLM. Can modify or log.
    async fn post_inference(
        &self,
        _response: &mut InferenceResponse,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        Ok(())
    }

    /// Called during gating decisions. Can influence pass/fail.
    async fn on_gate(
        &self,
        _event: &mut GateEvent,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        Ok(())
    }

    // ── Action (Layer 4) ────────────────────────────────────────────

    /// Called before an action executes. Can block, allow, or rewrite.
    async fn pre_action(
        &self,
        _event: &ToolCallEvent,
    ) -> Result<ActionDecision, Box<dyn std::error::Error + Send + Sync>> {
        Ok(ActionDecision::Proceed)
    }

    /// Called after an action completes.
    async fn post_action(
        &self,
        _event: &ToolCallEvent,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        Ok(())
    }

    /// Called when a tool is invoked. Can allow, deny, or rewrite.
    async fn on_tool_call(
        &self,
        _event: &ToolCallEvent,
    ) -> Result<ToolDecision, Box<dyn std::error::Error + Send + Sync>> {
        Ok(ToolDecision::Allow)
    }

    // ── Social (Layer 5) ────────────────────────────────────────────

    /// Called before sending a message to another agent.
    async fn on_message_send(
        &self,
        _message: &mut AgentMessage,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        Ok(())
    }

    /// Called when receiving a message from another agent.
    async fn on_message_receive(
        &self,
        _message: &mut AgentMessage,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        Ok(())
    }

    // ── Meta (Layer 6) ──────────────────────────────────────────────

    /// Called during the reflection phase. Returns suggested adjustments.
    async fn on_reflect(
        &self,
        _state: &ReflectionState,
    ) -> Result<Vec<Adjustment>, Box<dyn std::error::Error + Send + Sync>> {
        Ok(Vec::new())
    }

    /// Called when cost data is updated (tokens, USD).
    async fn on_cost_update(
        &self,
        _cost: &CostUpdate,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        Ok(())
    }

    // ── Recovery (Layer 7) ──────────────────────────────────────────

    /// Called when an error occurs. Determines recovery strategy.
    async fn on_error(
        &self,
        _event: &ErrorEvent,
    ) -> Result<RecoveryAction, Box<dyn std::error::Error + Send + Sync>> {
        Ok(RecoveryAction::Propagate)
    }

    // ── Cross-cutting hooks (all layers) ────────────────────────────

    /// Called at the start of each agent tick regardless of layer.
    ///
    /// `tick` is a monotonically increasing counter (0-based) for the
    /// current agent session. These hooks fire for every extension in the
    /// chain, not just those in a specific layer.
    async fn on_tick_start(
        &self,
        _tick: u64,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        Ok(())
    }

    /// Called at the end of each agent tick regardless of layer.
    async fn on_tick_end(
        &self,
        _tick: u64,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        Ok(())
    }

    /// Called when a task slot is assigned to an agent.
    ///
    /// `slot` is the slot/task identifier string.
    /// `task` is the full task descriptor as a JSON value.
    async fn on_slot_assigned(
        &self,
        _slot: &str,
        _task: &serde_json::Value,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        Ok(())
    }

    /// Called when a task slot completes (success or failure).
    ///
    /// `slot` is the slot/task identifier string.
    /// `result` is the task result/outcome as a JSON value.
    async fn on_slot_completed(
        &self,
        _slot: &str,
        _result: &serde_json::Value,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        Ok(())
    }
}

// ── ExtensionChain ────────────────────────────────────────────────────

/// An ordered chain of extensions, executed in layer order.
pub struct ExtensionChain {
    extensions: Vec<Box<dyn Extension>>,
}

impl ExtensionChain {
    /// Create an empty chain.
    pub fn new() -> Self {
        Self {
            extensions: Vec::new(),
        }
    }

    /// Add an extension to the chain. Extensions are sorted by layer on build.
    pub fn add(&mut self, ext: Box<dyn Extension>) {
        self.extensions.push(ext);
    }

    /// Sort extensions by layer (stable sort preserves config order within layer).
    pub fn sort_by_layer(&mut self) {
        self.extensions.sort_by_key(|e| e.layer() as u8);
    }

    /// Number of loaded extensions.
    pub fn len(&self) -> usize {
        self.extensions.len()
    }

    /// Whether the chain is empty.
    pub fn is_empty(&self) -> bool {
        self.extensions.is_empty()
    }

    /// Initialize all extensions in order.
    pub async fn init_all(&mut self) -> Vec<(String, Box<dyn std::error::Error + Send + Sync>)> {
        let mut errors = Vec::new();
        for ext in &mut self.extensions {
            if let Err(e) = ext.on_init().await {
                errors.push((ext.name().to_string(), e));
            }
        }
        errors
    }

    /// Shut down all extensions in reverse order.
    pub async fn shutdown_all(
        &mut self,
    ) -> Vec<(String, Box<dyn std::error::Error + Send + Sync>)> {
        let mut errors = Vec::new();
        for ext in self.extensions.iter_mut().rev() {
            if let Err(e) = ext.on_shutdown().await {
                errors.push((ext.name().to_string(), e));
            }
        }
        errors
    }

    /// Run pre_inference hooks (Cognition layer only).
    pub async fn run_pre_inference(
        &self,
        request: &mut InferenceRequest,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        for ext in self
            .extensions
            .iter()
            .filter(|e| e.layer() == ExtensionLayer::Cognition)
        {
            ext.pre_inference(request).await?;
        }
        Ok(())
    }

    /// Run post_inference hooks (Cognition layer only).
    pub async fn run_post_inference(
        &self,
        response: &mut InferenceResponse,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        for ext in self
            .extensions
            .iter()
            .filter(|e| e.layer() == ExtensionLayer::Cognition)
        {
            ext.post_inference(response).await?;
        }
        Ok(())
    }

    /// Run on_gate hooks (Cognition layer only).
    pub async fn run_on_gate(
        &self,
        event: &mut GateEvent,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        for ext in self
            .extensions
            .iter()
            .filter(|e| e.layer() == ExtensionLayer::Cognition)
        {
            ext.on_gate(event).await?;
        }
        Ok(())
    }

    /// Run pre_action hooks (Action layer only). Returns first Block/Rewrite.
    pub async fn run_pre_action(
        &self,
        event: &ToolCallEvent,
    ) -> Result<ActionDecision, Box<dyn std::error::Error + Send + Sync>> {
        for ext in self
            .extensions
            .iter()
            .filter(|e| e.layer() == ExtensionLayer::Action)
        {
            match ext.pre_action(event).await? {
                ActionDecision::Proceed => continue,
                decision => return Ok(decision),
            }
        }
        Ok(ActionDecision::Proceed)
    }

    /// Run on_tool_call hooks (Action layer only). Returns first Deny/Rewrite.
    pub async fn run_on_tool_call(
        &self,
        event: &ToolCallEvent,
    ) -> Result<ToolDecision, Box<dyn std::error::Error + Send + Sync>> {
        for ext in self
            .extensions
            .iter()
            .filter(|e| e.layer() == ExtensionLayer::Action)
        {
            match ext.on_tool_call(event).await? {
                ToolDecision::Allow => continue,
                decision => return Ok(decision),
            }
        }
        Ok(ToolDecision::Allow)
    }

    /// Run on_error hooks (Recovery layer only). Returns first non-Propagate.
    pub async fn run_on_error(
        &self,
        event: &ErrorEvent,
    ) -> Result<RecoveryAction, Box<dyn std::error::Error + Send + Sync>> {
        for ext in self
            .extensions
            .iter()
            .filter(|e| e.layer() == ExtensionLayer::Recovery)
        {
            match ext.on_error(event).await? {
                RecoveryAction::Propagate => continue,
                action => return Ok(action),
            }
        }
        Ok(RecoveryAction::Propagate)
    }

    /// List all extension metadata.
    pub fn metadata(&self) -> Vec<ExtensionMeta> {
        self.extensions.iter().map(|e| e.meta()).collect()
    }

    // ── Cross-cutting chain runners ───────────────────────────────────

    /// Run on_tick_start hooks across ALL extensions regardless of layer.
    pub async fn run_on_tick_start(
        &self,
        tick: u64,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        for ext in &self.extensions {
            ext.on_tick_start(tick).await?;
        }
        Ok(())
    }

    /// Run on_tick_end hooks across ALL extensions regardless of layer.
    pub async fn run_on_tick_end(
        &self,
        tick: u64,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        for ext in &self.extensions {
            ext.on_tick_end(tick).await?;
        }
        Ok(())
    }

    /// Run on_slot_assigned hooks across ALL extensions regardless of layer.
    pub async fn run_on_slot_assigned(
        &self,
        slot: &str,
        task: &serde_json::Value,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        for ext in &self.extensions {
            ext.on_slot_assigned(slot, task).await?;
        }
        Ok(())
    }

    /// Run on_slot_completed hooks across ALL extensions regardless of layer.
    pub async fn run_on_slot_completed(
        &self,
        slot: &str,
        result: &serde_json::Value,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        for ext in &self.extensions {
            ext.on_slot_completed(slot, result).await?;
        }
        Ok(())
    }
}

impl Default for ExtensionChain {
    fn default() -> Self {
        Self::new()
    }
}

// ── HookRunner ────────────────────────────────────────────────────────

/// Error produced when a hook fails or times out during dispatch.
///
/// Individual `HookError`s are logged but do not abort the dispatch loop —
/// the `HookRunner` isolates failures so one bad extension cannot prevent
/// others from receiving the hook call.
#[derive(Debug)]
pub struct HookError {
    /// Name of the extension that produced the error.
    pub extension: String,
    /// Human-readable description of the failure.
    pub message: String,
    /// Whether the failure was due to a timeout (vs. an `Err` from the hook).
    pub timed_out: bool,
}

impl std::fmt::Display for HookError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.timed_out {
            write!(
                f,
                "extension '{}' timed out: {}",
                self.extension, self.message
            )
        } else {
            write!(f, "extension '{}' error: {}", self.extension, self.message)
        }
    }
}

impl std::error::Error for HookError {}

/// Dispatches cross-cutting lifecycle hooks to every registered extension
/// with per-extension timeout and error isolation.
///
/// Unlike `ExtensionChain`'s `run_on_tick_*` / `run_on_slot_*` methods —
/// which propagate the first error with `?` — `HookRunner` logs and
/// continues on each extension failure, collecting all errors for the
/// caller to inspect.
pub struct HookRunner {
    chain: ExtensionChain,
    /// Per-hook call timeout. Applied independently to each extension.
    timeout: std::time::Duration,
}

impl HookRunner {
    /// Create a new `HookRunner` wrapping `chain` with the given per-hook `timeout`.
    pub fn new(chain: ExtensionChain, timeout: std::time::Duration) -> Self {
        Self { chain, timeout }
    }

    /// Create a `HookRunner` with the default 5-second timeout.
    pub fn with_default_timeout(chain: ExtensionChain) -> Self {
        Self::new(chain, std::time::Duration::from_secs(5))
    }

    /// Access the inner chain mutably.
    pub fn chain_mut(&mut self) -> &mut ExtensionChain {
        &mut self.chain
    }

    /// Access the inner chain immutably.
    pub fn chain(&self) -> &ExtensionChain {
        &self.chain
    }

    /// Dispatch `on_tick_start` to every extension with fault isolation.
    pub async fn dispatch_tick_start(&self, tick: u64) -> Vec<HookError> {
        let mut errors = Vec::new();
        for ext in &self.chain.extensions {
            let name = ext.name().to_string();
            match tokio::time::timeout(self.timeout, ext.on_tick_start(tick)).await {
                Ok(Ok(())) => {}
                Ok(Err(e)) => {
                    tracing::warn!(extension = %name, error = %e,
                        "on_tick_start returned error (isolated, continuing)");
                    errors.push(HookError {
                        extension: name,
                        message: e.to_string(),
                        timed_out: false,
                    });
                }
                Err(_) => {
                    tracing::warn!(extension = %name, timeout_ms = self.timeout.as_millis(),
                        "on_tick_start timed out (isolated, continuing)");
                    errors.push(HookError {
                        extension: name,
                        message: format!("timed out after {}ms", self.timeout.as_millis()),
                        timed_out: true,
                    });
                }
            }
        }
        errors
    }

    /// Dispatch `on_tick_end` to every extension with fault isolation.
    pub async fn dispatch_tick_end(&self, tick: u64) -> Vec<HookError> {
        let mut errors = Vec::new();
        for ext in &self.chain.extensions {
            let name = ext.name().to_string();
            match tokio::time::timeout(self.timeout, ext.on_tick_end(tick)).await {
                Ok(Ok(())) => {}
                Ok(Err(e)) => {
                    tracing::warn!(extension = %name, error = %e,
                        "on_tick_end returned error (isolated, continuing)");
                    errors.push(HookError {
                        extension: name,
                        message: e.to_string(),
                        timed_out: false,
                    });
                }
                Err(_) => {
                    tracing::warn!(extension = %name, timeout_ms = self.timeout.as_millis(),
                        "on_tick_end timed out (isolated, continuing)");
                    errors.push(HookError {
                        extension: name,
                        message: format!("timed out after {}ms", self.timeout.as_millis()),
                        timed_out: true,
                    });
                }
            }
        }
        errors
    }

    /// Dispatch `on_slot_assigned` to every extension with fault isolation.
    pub async fn dispatch_slot_assigned(
        &self,
        slot: &str,
        task: &serde_json::Value,
    ) -> Vec<HookError> {
        let mut errors = Vec::new();
        for ext in &self.chain.extensions {
            let name = ext.name().to_string();
            match tokio::time::timeout(self.timeout, ext.on_slot_assigned(slot, task)).await {
                Ok(Ok(())) => {}
                Ok(Err(e)) => {
                    tracing::warn!(extension = %name, error = %e,
                        "on_slot_assigned returned error (isolated, continuing)");
                    errors.push(HookError {
                        extension: name,
                        message: e.to_string(),
                        timed_out: false,
                    });
                }
                Err(_) => {
                    tracing::warn!(extension = %name, timeout_ms = self.timeout.as_millis(),
                        "on_slot_assigned timed out (isolated, continuing)");
                    errors.push(HookError {
                        extension: name,
                        message: format!("timed out after {}ms", self.timeout.as_millis()),
                        timed_out: true,
                    });
                }
            }
        }
        errors
    }

    /// Dispatch `on_slot_completed` to every extension with fault isolation.
    pub async fn dispatch_slot_completed(
        &self,
        slot: &str,
        result: &serde_json::Value,
    ) -> Vec<HookError> {
        let mut errors = Vec::new();
        for ext in &self.chain.extensions {
            let name = ext.name().to_string();
            match tokio::time::timeout(self.timeout, ext.on_slot_completed(slot, result)).await {
                Ok(Ok(())) => {}
                Ok(Err(e)) => {
                    tracing::warn!(extension = %name, error = %e,
                        "on_slot_completed returned error (isolated, continuing)");
                    errors.push(HookError {
                        extension: name,
                        message: e.to_string(),
                        timed_out: false,
                    });
                }
                Err(_) => {
                    tracing::warn!(extension = %name, timeout_ms = self.timeout.as_millis(),
                        "on_slot_completed timed out (isolated, continuing)");
                    errors.push(HookError {
                        extension: name,
                        message: format!("timed out after {}ms", self.timeout.as_millis()),
                        timed_out: true,
                    });
                }
            }
        }
        errors
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestExtension {
        name: String,
        layer: ExtensionLayer,
    }

    #[async_trait::async_trait]
    impl Extension for TestExtension {
        fn name(&self) -> &str {
            &self.name
        }
        fn layer(&self) -> ExtensionLayer {
            self.layer
        }
    }

    #[test]
    fn chain_sorts_by_layer() {
        let mut chain = ExtensionChain::new();
        chain.add(Box::new(TestExtension {
            name: "recovery-ext".into(),
            layer: ExtensionLayer::Recovery,
        }));
        chain.add(Box::new(TestExtension {
            name: "cognition-ext".into(),
            layer: ExtensionLayer::Cognition,
        }));
        chain.add(Box::new(TestExtension {
            name: "foundation-ext".into(),
            layer: ExtensionLayer::Foundation,
        }));

        chain.sort_by_layer();
        let names: Vec<_> = chain.extensions.iter().map(|e| e.name()).collect();
        assert_eq!(names, &["foundation-ext", "cognition-ext", "recovery-ext"]);
    }

    #[tokio::test]
    async fn chain_init_shutdown_order() {
        let mut chain = ExtensionChain::new();
        chain.add(Box::new(TestExtension {
            name: "ext-a".into(),
            layer: ExtensionLayer::Foundation,
        }));
        chain.add(Box::new(TestExtension {
            name: "ext-b".into(),
            layer: ExtensionLayer::Cognition,
        }));

        let init_errors = chain.init_all().await;
        assert!(init_errors.is_empty());

        let shutdown_errors = chain.shutdown_all().await;
        assert!(shutdown_errors.is_empty());
    }

    #[test]
    fn metadata_reflects_extensions() {
        let mut chain = ExtensionChain::new();
        chain.add(Box::new(TestExtension {
            name: "my-ext".into(),
            layer: ExtensionLayer::Social,
        }));

        let meta = chain.metadata();
        assert_eq!(meta.len(), 1);
        assert_eq!(meta[0].name, "my-ext");
        assert_eq!(meta[0].layer, ExtensionLayer::Social);
    }

    #[tokio::test]
    async fn tool_call_allow_by_default() {
        let chain = ExtensionChain::new();
        let event = ToolCallEvent {
            tool_name: "bash".into(),
            arguments: serde_json::json!({}),
            result: None,
        };
        let decision = chain.run_on_tool_call(&event).await.unwrap();
        assert_eq!(decision, ToolDecision::Allow);
    }

    #[tokio::test]
    async fn action_proceed_by_default() {
        let chain = ExtensionChain::new();
        let event = ToolCallEvent {
            tool_name: "bash".into(),
            arguments: serde_json::json!({}),
            result: None,
        };
        let decision = chain.run_pre_action(&event).await.unwrap();
        assert_eq!(decision, ActionDecision::Proceed);
    }

    #[tokio::test]
    async fn error_propagate_by_default() {
        let chain = ExtensionChain::new();
        let event = ErrorEvent {
            error_message: "test error".into(),
            source: "test".into(),
            extra: serde_json::Value::Null,
        };
        let action = chain.run_on_error(&event).await.unwrap();
        assert_eq!(action, RecoveryAction::Propagate);
    }

    #[tokio::test]
    async fn pre_inference_typed_struct() {
        let chain = ExtensionChain::new();
        let mut req = InferenceRequest {
            plan_id: "plan-1".into(),
            task: "task-1".into(),
            role: "engineer".into(),
            model: "claude-sonnet-4-20250514".into(),
            prompt_tokens: 1000,
            extra: serde_json::Value::Null,
        };
        // No extensions, should pass through cleanly.
        chain.run_pre_inference(&mut req).await.unwrap();
        assert_eq!(req.plan_id, "plan-1");
    }

    #[tokio::test]
    async fn post_inference_typed_struct() {
        let chain = ExtensionChain::new();
        let mut resp = InferenceResponse {
            plan_id: "plan-1".into(),
            task: "task-1".into(),
            role: "engineer".into(),
            model: "claude-sonnet-4-20250514".into(),
            success: true,
            cost_usd: 0.01,
            wall_ms: 500,
            extra: serde_json::Value::Null,
        };
        chain.run_post_inference(&mut resp).await.unwrap();
        assert!(resp.success);
    }

    #[tokio::test]
    async fn on_gate_typed_struct() {
        let chain = ExtensionChain::new();
        let mut event = GateEvent {
            plan_id: "plan-1".into(),
            gate_name: "compile".into(),
            passed: true,
            rung: "rung-1".into(),
            duration_ms: 200,
            details: serde_json::Value::Null,
        };
        chain.run_on_gate(&mut event).await.unwrap();
        assert!(event.passed);
    }
    // ── CaMeL IFC tests (E30-T01) ──────────────────────────────────────

    #[test]
    fn camel_taint_level_ordering() {
        assert!(CamelTaintLevel::Trusted < CamelTaintLevel::Local);
        assert!(CamelTaintLevel::Local < CamelTaintLevel::External);
        assert!(CamelTaintLevel::External < CamelTaintLevel::Untrusted);
    }

    #[test]
    fn camel_taint_level_display() {
        assert_eq!(CamelTaintLevel::Trusted.to_string(), "trusted");
        assert_eq!(CamelTaintLevel::Local.to_string(), "local");
        assert_eq!(CamelTaintLevel::External.to_string(), "external");
        assert_eq!(CamelTaintLevel::Untrusted.to_string(), "untrusted");
    }

    #[test]
    fn tag_operation_display() {
        assert_eq!(TagOperation::Passthrough.to_string(), "passthrough");
        assert_eq!(TagOperation::Transform.to_string(), "transform");
        assert_eq!(TagOperation::Merge.to_string(), "merge");
    }

    #[test]
    fn capability_set_intersection() {
        let a = CapabilitySet::from_strings(["read_disk", "call_llm", "write_disk"]);
        let b = CapabilitySet::from_strings(["read_disk", "call_llm"]);
        let result = a.intersection(&b);
        assert!(result.capabilities.contains("read_disk"));
        assert!(result.capabilities.contains("call_llm"));
        assert!(!result.capabilities.contains("write_disk"));
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn capability_set_union() {
        let a = CapabilitySet::from_strings(["read_disk"]);
        let b = CapabilitySet::from_strings(["call_llm"]);
        let result = a.union(&b);
        assert_eq!(result.len(), 2);
        assert!(result.capabilities.contains("read_disk"));
        assert!(result.capabilities.contains("call_llm"));
    }

    #[test]
    fn camel_tag_new_and_trusted() {
        let tag = CamelTag::new(["read_disk".to_string()], CamelTaintLevel::Local);
        assert_eq!(tag.taint_level, CamelTaintLevel::Local);
        assert!(tag.capabilities.capabilities.contains("read_disk"));
        assert!(tag.provenance.is_empty());

        let trusted = CamelTag::trusted(["call_llm".to_string()]);
        assert_eq!(trusted.taint_level, CamelTaintLevel::Trusted);
    }

    #[test]
    fn camel_tag_propagate_appends_provenance() {
        let tag = CamelTag::new(["read_disk".to_string()], CamelTaintLevel::Trusted);
        let after = tag.propagate("my-extension", TagOperation::Passthrough);

        assert!(tag.provenance.is_empty());
        assert_eq!(after.provenance.len(), 1);
        assert_eq!(after.provenance[0].handler, "my-extension");
        assert_eq!(after.provenance[0].operation, TagOperation::Passthrough);
        assert_eq!(after.taint_level, CamelTaintLevel::Trusted);
        assert!(after.capabilities.capabilities.contains("read_disk"));
    }

    #[test]
    fn camel_tag_propagate_chaining() {
        let tag = CamelTag::trusted(["cap-a".to_string()]);
        let a1 = tag.propagate("handler-1", TagOperation::Passthrough);
        let a2 = a1.propagate("handler-2", TagOperation::Transform);

        assert_eq!(a2.provenance.len(), 2);
        assert_eq!(a2.provenance[0].handler, "handler-1");
        assert_eq!(a2.provenance[1].handler, "handler-2");
        assert_eq!(a2.provenance[1].operation, TagOperation::Transform);
    }

    #[test]
    fn camel_tag_merge_empty() {
        let result = CamelTag::merge(&[]);
        assert_eq!(result.taint_level, CamelTaintLevel::Trusted);
        assert!(result.capabilities.is_empty());
        assert!(result.provenance.is_empty());
    }

    #[test]
    fn camel_tag_merge_intersects_capabilities() {
        let a = CamelTag::new(
            ["cap-a".to_string(), "cap-b".to_string()],
            CamelTaintLevel::Trusted,
        );
        let b = CamelTag::new(
            ["cap-b".to_string(), "cap-c".to_string()],
            CamelTaintLevel::Local,
        );

        let merged = CamelTag::merge(&[&a, &b]);
        assert_eq!(merged.capabilities.len(), 1);
        assert!(merged.capabilities.capabilities.contains("cap-b"));
        assert_eq!(merged.taint_level, CamelTaintLevel::Local);
    }

    #[test]
    fn camel_tag_merge_worst_taint_wins() {
        let trusted = CamelTag::trusted(["c".to_string()]);
        let untrusted = CamelTag::new(["c".to_string()], CamelTaintLevel::Untrusted);
        let external = CamelTag::new(["c".to_string()], CamelTaintLevel::External);

        let merged = CamelTag::merge(&[&trusted, &external, &untrusted]);
        assert_eq!(merged.taint_level, CamelTaintLevel::Untrusted);
    }

    #[test]
    fn camel_tag_merge_concatenates_provenance() {
        let mut a = CamelTag::trusted(["c".to_string()]);
        a.provenance
            .push(ProvenanceEntry::now("handler-a", TagOperation::Passthrough));

        let mut b = CamelTag::trusted(["c".to_string()]);
        b.provenance
            .push(ProvenanceEntry::now("handler-b", TagOperation::Transform));

        let merged = CamelTag::merge(&[&a, &b]);
        assert_eq!(merged.provenance.len(), 2);
        assert_eq!(merged.provenance[0].handler, "handler-a");
        assert_eq!(merged.provenance[1].handler, "handler-b");
    }

    #[test]
    fn camel_tag_serde_roundtrip() {
        let tag = CamelTag::new(["read_disk".to_string()], CamelTaintLevel::External);
        let json = serde_json::to_string(&tag).unwrap();
        let decoded: CamelTag = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.taint_level, CamelTaintLevel::External);
        assert!(decoded.capabilities.capabilities.contains("read_disk"));
    }

    #[test]
    fn camel_taint_level_serde() {
        let json = serde_json::to_string(&CamelTaintLevel::External).unwrap();
        assert_eq!(json, r#""external""#);
        let decoded: CamelTaintLevel = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, CamelTaintLevel::External);
    }

    // ── Cross-cutting hook tests via ExtensionChain (E30-T04) ─────────

    #[tokio::test]
    async fn chain_run_on_tick_start_empty() {
        let chain = ExtensionChain::new();
        chain.run_on_tick_start(0).await.unwrap();
    }

    #[tokio::test]
    async fn chain_run_on_tick_end_empty() {
        let chain = ExtensionChain::new();
        chain.run_on_tick_end(0).await.unwrap();
    }

    #[tokio::test]
    async fn chain_run_on_slot_assigned_empty() {
        let chain = ExtensionChain::new();
        chain
            .run_on_slot_assigned("slot-1", &serde_json::json!({}))
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn chain_run_on_slot_completed_empty() {
        let chain = ExtensionChain::new();
        chain
            .run_on_slot_completed("slot-1", &serde_json::json!({"status": "ok"}))
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn chain_cross_cutting_hooks_visit_all_layers() {
        // Extensions from different layers should all receive cross-cutting hooks.
        let mut chain = ExtensionChain::new();
        chain.add(Box::new(TestExtension {
            name: "foundation-ext".into(),
            layer: ExtensionLayer::Foundation,
        }));
        chain.add(Box::new(TestExtension {
            name: "action-ext".into(),
            layer: ExtensionLayer::Action,
        }));
        chain.add(Box::new(TestExtension {
            name: "recovery-ext".into(),
            layer: ExtensionLayer::Recovery,
        }));
        // All no-op, should not error.
        chain.run_on_tick_start(42).await.unwrap();
        chain.run_on_tick_end(42).await.unwrap();
        chain
            .run_on_slot_assigned("t1", &serde_json::json!({}))
            .await
            .unwrap();
        chain
            .run_on_slot_completed("t1", &serde_json::json!({}))
            .await
            .unwrap();
    }

    // ── HookRunner tests (E30-T04) ────────────────────────────────────

    /// Extension whose cross-cutting hooks always return an error.
    struct FailingExtension {
        name: String,
        layer: ExtensionLayer,
    }

    #[async_trait::async_trait]
    impl Extension for FailingExtension {
        fn name(&self) -> &str {
            &self.name
        }
        fn layer(&self) -> ExtensionLayer {
            self.layer
        }
        async fn on_tick_start(
            &self,
            _tick: u64,
        ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            Err("tick start failed".into())
        }
        async fn on_tick_end(
            &self,
            _tick: u64,
        ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            Err("tick end failed".into())
        }
        async fn on_slot_assigned(
            &self,
            _slot: &str,
            _task: &serde_json::Value,
        ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            Err("slot assigned failed".into())
        }
        async fn on_slot_completed(
            &self,
            _slot: &str,
            _result: &serde_json::Value,
        ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            Err("slot completed failed".into())
        }
    }

    /// Extension whose on_tick_start sleeps longer than the configured timeout.
    struct SlowExtension {
        name: String,
        layer: ExtensionLayer,
        delay_ms: u64,
    }

    #[async_trait::async_trait]
    impl Extension for SlowExtension {
        fn name(&self) -> &str {
            &self.name
        }
        fn layer(&self) -> ExtensionLayer {
            self.layer
        }
        async fn on_tick_start(
            &self,
            _tick: u64,
        ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            tokio::time::sleep(std::time::Duration::from_millis(self.delay_ms)).await;
            Ok(())
        }
    }

    #[tokio::test]
    async fn hook_runner_empty_chain_no_errors() {
        let chain = ExtensionChain::new();
        let runner = HookRunner::with_default_timeout(chain);
        let errors = runner.dispatch_tick_start(0).await;
        assert!(errors.is_empty());
    }

    #[tokio::test]
    async fn hook_runner_tick_start_success_all_extensions() {
        let mut chain = ExtensionChain::new();
        chain.add(Box::new(TestExtension {
            name: "a".into(),
            layer: ExtensionLayer::Foundation,
        }));
        chain.add(Box::new(TestExtension {
            name: "b".into(),
            layer: ExtensionLayer::Cognition,
        }));
        let runner = HookRunner::with_default_timeout(chain);
        // TestExtension::on_tick_start returns Ok, so no errors expected.
        let errors = runner.dispatch_tick_start(42).await;
        assert!(errors.is_empty(), "expected no errors, got: {:?}", errors);
    }

    #[tokio::test]
    async fn hook_runner_tick_end_success() {
        let mut chain = ExtensionChain::new();
        chain.add(Box::new(TestExtension {
            name: "a".into(),
            layer: ExtensionLayer::Foundation,
        }));
        let runner = HookRunner::with_default_timeout(chain);
        let errors = runner.dispatch_tick_end(1).await;
        assert!(errors.is_empty());
    }

    #[tokio::test]
    async fn hook_runner_slot_assigned_success() {
        let mut chain = ExtensionChain::new();
        chain.add(Box::new(TestExtension {
            name: "obs-ext".into(),
            layer: ExtensionLayer::Meta,
        }));
        let runner = HookRunner::with_default_timeout(chain);
        let task = serde_json::json!({"id": "task-1", "title": "Do work"});
        let errors = runner.dispatch_slot_assigned("slot-1", &task).await;
        assert!(errors.is_empty());
    }

    #[tokio::test]
    async fn hook_runner_slot_completed_success() {
        let mut chain = ExtensionChain::new();
        chain.add(Box::new(TestExtension {
            name: "obs-ext".into(),
            layer: ExtensionLayer::Meta,
        }));
        let runner = HookRunner::with_default_timeout(chain);
        let result = serde_json::json!({"status": "ok"});
        let errors = runner.dispatch_slot_completed("slot-1", &result).await;
        assert!(errors.is_empty());
    }

    #[tokio::test]
    async fn hook_runner_isolates_single_failing_extension() {
        let mut chain = ExtensionChain::new();
        chain.add(Box::new(FailingExtension {
            name: "fail-ext".into(),
            layer: ExtensionLayer::Foundation,
        }));
        chain.add(Box::new(TestExtension {
            name: "ok-ext".into(),
            layer: ExtensionLayer::Cognition,
        }));
        let runner = HookRunner::with_default_timeout(chain);
        let errors = runner.dispatch_tick_start(0).await;
        assert_eq!(errors.len(), 1, "expected exactly one error");
        assert_eq!(errors[0].extension, "fail-ext");
        assert!(
            !errors[0].timed_out,
            "error should not be flagged as timeout"
        );
        assert!(
            errors[0].message.contains("tick start failed"),
            "unexpected message: {}",
            errors[0].message
        );
    }

    #[tokio::test]
    async fn hook_runner_error_isolation_slot_assigned() {
        let mut chain = ExtensionChain::new();
        chain.add(Box::new(FailingExtension {
            name: "bad-ext".into(),
            layer: ExtensionLayer::Recovery,
        }));
        chain.add(Box::new(TestExtension {
            name: "good-ext".into(),
            layer: ExtensionLayer::Action,
        }));
        let runner = HookRunner::with_default_timeout(chain);
        let task = serde_json::json!({});
        let errors = runner.dispatch_slot_assigned("slot-x", &task).await;
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].extension, "bad-ext");
        assert!(!errors[0].timed_out);
    }

    #[tokio::test]
    async fn hook_runner_error_isolation_slot_completed() {
        let mut chain = ExtensionChain::new();
        chain.add(Box::new(FailingExtension {
            name: "bad-ext".into(),
            layer: ExtensionLayer::Meta,
        }));
        chain.add(Box::new(TestExtension {
            name: "ok-ext".into(),
            layer: ExtensionLayer::Social,
        }));
        let runner = HookRunner::with_default_timeout(chain);
        let result = serde_json::json!({"status": "failed"});
        let errors = runner.dispatch_slot_completed("slot-y", &result).await;
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].extension, "bad-ext");
    }

    #[tokio::test]
    async fn hook_runner_tick_end_error_isolation() {
        let mut chain = ExtensionChain::new();
        chain.add(Box::new(FailingExtension {
            name: "bad-ext".into(),
            layer: ExtensionLayer::Foundation,
        }));
        chain.add(Box::new(TestExtension {
            name: "ok-ext".into(),
            layer: ExtensionLayer::Recovery,
        }));
        let runner = HookRunner::with_default_timeout(chain);
        let errors = runner.dispatch_tick_end(7).await;
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].extension, "bad-ext");
        assert!(errors[0].message.contains("tick end failed"));
    }

    #[tokio::test]
    async fn hook_runner_timeout_detected() {
        // SlowExtension sleeps 200ms but our timeout is 10ms.
        let mut chain = ExtensionChain::new();
        chain.add(Box::new(SlowExtension {
            name: "slow-ext".into(),
            layer: ExtensionLayer::Meta,
            delay_ms: 200,
        }));
        let runner = HookRunner::new(chain, std::time::Duration::from_millis(10));
        let errors = runner.dispatch_tick_start(0).await;
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].extension, "slow-ext");
        assert!(errors[0].timed_out, "expected timed_out=true, got false");
        assert!(
            errors[0].message.contains("timed out after"),
            "unexpected message: {}",
            errors[0].message
        );
    }

    #[tokio::test]
    async fn hook_runner_timeout_does_not_block_next_extension() {
        let mut chain = ExtensionChain::new();
        chain.add(Box::new(SlowExtension {
            name: "slow-ext".into(),
            layer: ExtensionLayer::Foundation,
            delay_ms: 200,
        }));
        chain.add(Box::new(TestExtension {
            name: "fast-ext".into(),
            layer: ExtensionLayer::Cognition,
        }));
        let runner = HookRunner::new(chain, std::time::Duration::from_millis(10));
        let errors = runner.dispatch_tick_start(0).await;
        assert_eq!(
            errors.len(),
            1,
            "expected 1 error (slow-ext timeout), got {:?}",
            errors
        );
        assert_eq!(errors[0].extension, "slow-ext");
    }

    #[tokio::test]
    async fn hook_runner_hook_error_display_non_timeout() {
        let err = HookError {
            extension: "my-ext".into(),
            message: "boom".into(),
            timed_out: false,
        };
        let s = err.to_string();
        assert!(s.contains("my-ext"), "expected extension name in: {s}");
        assert!(s.contains("boom"), "expected message in: {s}");
        assert!(s.contains("error"), "expected 'error' in: {s}");
    }

    #[tokio::test]
    async fn hook_runner_hook_error_display_timeout() {
        let err = HookError {
            extension: "slow-ext".into(),
            message: "timed out after 5000ms".into(),
            timed_out: true,
        };
        let s = err.to_string();
        assert!(s.contains("slow-ext"), "expected extension name in: {s}");
        assert!(s.contains("timed out"), "expected 'timed out' in: {s}");
    }

    #[tokio::test]
    async fn hook_runner_multiple_failing_extensions_all_collected() {
        let mut chain = ExtensionChain::new();
        chain.add(Box::new(FailingExtension {
            name: "fail-1".into(),
            layer: ExtensionLayer::Foundation,
        }));
        chain.add(Box::new(FailingExtension {
            name: "fail-2".into(),
            layer: ExtensionLayer::Cognition,
        }));
        chain.add(Box::new(TestExtension {
            name: "ok-ext".into(),
            layer: ExtensionLayer::Recovery,
        }));
        let runner = HookRunner::with_default_timeout(chain);
        let errors = runner.dispatch_tick_end(5).await;
        assert_eq!(
            errors.len(),
            2,
            "both failing extensions should be reported"
        );
        let names: Vec<&str> = errors.iter().map(|e| e.extension.as_str()).collect();
        assert!(names.contains(&"fail-1"), "missing fail-1 in {:?}", names);
        assert!(names.contains(&"fail-2"), "missing fail-2 in {:?}", names);
    }

    #[tokio::test]
    async fn hook_runner_chain_mut_allows_late_add() {
        let chain = ExtensionChain::new();
        let mut runner = HookRunner::with_default_timeout(chain);
        runner.chain_mut().add(Box::new(TestExtension {
            name: "late-ext".into(),
            layer: ExtensionLayer::Social,
        }));
        let errors = runner.dispatch_tick_start(0).await;
        assert!(errors.is_empty(), "late-added extension should work fine");
        assert_eq!(runner.chain().len(), 1);
    }
}
