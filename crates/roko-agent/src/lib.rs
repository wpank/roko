//! Agent backends — async executors that take a prompt and emit output signals.
//!
//! # Why a dedicated trait?
//!
//! The six core Roko traits (Store, Score, Verify, Route, Compose, React)
//! capture composition, verification, and decision-making. An **Agent** is
//! different: it's an async executor with potentially long-running side
//! effects (subprocess management, file edits, LLM API calls).
//!
//! Rather than contort an agent into a Verify or Compose, Roko adds the
//! [`Agent`] trait as a capability extension. The core stays clean; agent
//! impls live in this crate.
//!
//! # Implementations
//!
//! - [`MockAgent`] — deterministic, for tests
//! - [`ExecAgent`] — spawns an external CLI, pipes prompt to stdin, captures stdout
//! - [`ClaudeCliAgent`] — Claude CLI adapter with tool allowlists and resume support
//! - [`OllamaAgent`] — direct Ollama `/api/chat` adapter
//! - [`OllamaLlmBackend`] — Ollama tool-loop backend

#![allow(
    dead_code,
    missing_docs,
    unused_assignments,
    unused_variables,
    clippy::borrowed_box,
    clippy::doc_lazy_continuation,
    clippy::double_must_use,
    clippy::expect_used,
    clippy::large_enum_variant,
    clippy::module_name_repetitions,
    clippy::needless_borrow,
    clippy::never_loop,
    clippy::nursery,
    clippy::pedantic,
    clippy::ptr_arg,
    clippy::too_many_arguments,
    clippy::unnecessary_lazy_evaluations,
    clippy::unnecessary_sort_by,
    clippy::unwrap_used
)]

pub mod agent;
/// Short-lived content-addressed response cache for identical backend requests.
pub mod cache;
/// Explicit capability state enum replacing boolean flags (T008).
pub mod capability_state;
pub mod chat_types;
pub mod claude_agent;
pub mod claude_cli_agent;
pub mod codex_agent;
pub mod composition;
pub mod cursor_agent;
pub mod cursor_cli_agent;
pub mod dispatcher;
pub mod error;
pub mod exec;
/// File-backed response cache for demo determinism.
pub mod file_cache;
pub mod gateway_events;
pub mod gemini;
pub mod harness;
pub mod hermes;
pub mod http;
/// Automatic immune screening at the canonical provider final-output boundary.
pub mod immune_boundary;
mod immune_evidence;
pub mod introspection;
pub mod lifecycle;
pub mod mcp;
pub mod metamorphosis;
pub mod mock;
pub mod model_call_service;
pub mod multi_pool;
mod multimodal;
pub mod nl_to_format;
pub mod observer;
pub mod ollama;
pub mod openai_agent;
pub mod openai_compat_backend;
pub mod openclaw;
pub mod parity_matrix;
pub mod perplexity;
pub mod pool;
pub mod process;
pub mod provider;
/// Provider-change tracking and attempt-scoped attribution (T009).
pub mod provider_change;
pub mod rate_limit;
pub mod retry;
pub mod runtime_events;
pub mod safety;
pub mod session;
pub mod streaming;
pub mod task_runner;
pub mod testutil;
pub mod token_estimator;
mod tool_immune;
pub mod tool_loop;
pub mod translate;
pub mod usage;

pub use agent::{Agent, AgentResult};
pub use chat_types::{ChatRequest, RequestOptions, ResponseFormat, ToolChoice};
pub use claude_cli_agent::ClaudeCliAgent;
pub use composition::{AgentComposition, CompositeAgent, MergeStrategy, SkillSelector};
pub use error::AgentError;
pub use exec::ExecAgent;
pub use gateway_events::{AggregateStats, GatewayEvent, GatewayEventWriter, GatewayProjection};
pub use gemini::{
    GeminiCompatAgent, GeminiEmbedAgent, GeminiMetadata, GeminiNativeAgent, GenerateContentRequest,
    GenerateContentResponse, GroundingMetadata,
};
pub use harness::{
    AcpError, AcpEvent, AcpInitResponse, AcpNotification, AcpPromptPayload, AcpPromptResult,
    AcpStdioClient, AcpStdioConfig, BearerAuth, CancelMode, CapabilityMismatch, ChildProcessRunner,
    ClaudeStreamJsonParser, CliOutput, EventParser, HarnessAdapter, HarnessCapabilities,
    HarnessError, HarnessEvent, HarnessProbe, HarnessRegistry, HarnessService,
    HarnessTaskRequirements, HealthReport, McpMode, NewSessionOpts, OneShotMode, ProbeError,
    RegistryConfig, RegistryError, ScrubbedEnv, ServiceEndpoint, ServiceStatus, SessionId,
    SessionResumeMode, SpawnedChild, StreamingMode, ToolInjection, TransportFlavor,
    harness_events_to_agent_result, validate_for_task,
};
pub use hermes::{
    CrashRecoveryConfig, HERMES_TOOL_PROGRESS_EVENT, HermesAcpAgent, HermesAcpConfig, HermesConfig,
    HermesFlavor, HermesGatewayService, HermesHttpAgent, HermesOneShotAgent, HermesOneShotConfig,
    ToolProgressInspector, probe_hermes,
};
pub use http::{HttpPoster, ReqwestPoster, shared_http_client, shared_http_client_from};
pub use immune_boundary::{
    AgentIsolationControl, ImmuneScreenedAgent, ProviderBoundaryRecord,
    detect_provider_output_anomaly, quarantine_store_path,
};
pub use introspection::{AgentIdentity, Intervention, MetacognitiveMonitor, Turn};
pub use lifecycle::*;
pub use metamorphosis::{MorphError, MorphableAgent, RoleProfile};
pub use mock::MockAgent;
pub use model_call_service::ModelCallService;
pub use multi_pool::{KillReport, MultiAgentPool, WarmEntry};
pub use observer::{InferenceObserver, NoopInferenceObserver};
pub use ollama::agent::{OllamaAgent, OllamaLlmBackend};
pub use openai_compat_backend::OpenAiCompatLlmBackend;
pub use openclaw::{
    ConfigError as OpenClawConfigError, InferEnvelope, InferError, InferEventParser, InferOutput,
    OpenClawAcpAgent, OpenClawAcpConfig, OpenClawConfig, OpenClawGatewayService,
    OpenClawInferAgent, OpenClawInferConfig, TransportHint, probe_openclaw_infer,
};
pub use parity_matrix::{
    Capability as ParityCapability, CapabilityState, ProviderCapabilityMatrix,
    ProviderCapabilityRow, provider_label,
};
pub use perplexity::{
    Annotation, PerplexityChatAgent, PerplexityDeepResearchAgent, PerplexityEmbedAgent,
    PerplexityMetadata, PerplexitySearchClient, SearchOptions, SearchResult,
};
pub use pool::{AgentInstanceId, AgentPool, AgentTask, InstanceStatus, TaskOutcome};
pub use provider::{
    ProviderAdapter, adapter_for_kind, create_agent_for_model, current_safety_layer,
    with_scoped_safety_layer,
};
pub use rate_limit::{
    AcquireOutcome, ProviderHealthChecker, ProviderRateLimitSnapshot, ProviderRateLimiter,
    RateLimitError,
};
pub use roko_core::{
    BUILTIN_ROLE_POLICY_MANIFEST_PATH, BUILTIN_ROLE_POLICY_MANIFEST_TOML,
    MANIFEST_BACKED_BUILTIN_ROLE_IDS, PromptPolicy, RolePolicyManifest,
    RoleProfile as ManifestRoleProfile, ToolCapabilityPolicy,
};
pub use runtime_events::{AgentEventStream, AgentRuntimeEvent};
pub use safety::{
    AgentWarrant, Capability, CapabilityError, DataSink, HookDecision, SafetyAuditRecord,
    SafetyHook, SafetyLayer, SafetyViolation, TaintLabel, TaintedString, ToolPermissionPolicy,
    ViolationSeverity, ViolationType, check_capability, check_tool_permission, delegate,
};
pub use session::{
    AgentInvocationSession, InvocationState, ResumeValidationError, ReuseScope, WarmReusePolicy,
    WarmReuseRequest, fingerprint_text, validate_resume_request,
};
pub use streaming::{
    ClaudeCliParser, OpenAiSseParser, StreamJsonParser, UnifiedStreamEvent,
};
pub use task_runner::{
    AgentEvent, Anomaly, AnomalyDetector, BudgetAction, BudgetGuardrail, ConductorAction,
    ConductorBandit, CostTable, EventBus, ModelPricing, TaskResult, TaskRunner, TaskRunnerError,
};
pub use token_estimator::{
    ContextWindowStatus, check_context_window, context_window_for_slug, estimate_prompt_tokens,
    estimate_tokens,
};
pub use tool_immune::{
    ToolBoundaryEffect, ToolBoundaryRecord, ToolControl, ToolControlState,
    detect_tool_result_anomaly, quarantine_vault_path, tool_controls_path,
};
pub use tool_loop::{
    OnTurnCallback, StreamEvent, StreamEventKind, ToolLoopAgent, TurnConfig, TurnProgress,
    collect_stream_to_response, response_to_synthetic_stream,
};
pub use usage::{Usage, UsageObservation, UsageSource};
