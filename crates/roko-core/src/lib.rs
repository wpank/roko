//! Roko kernel — the universal Signal type and six traits that compose to express
//! every capability in the Roko orchestration system.
//!
//! # Architecture
//!
//! The entire Roko system is built from **one noun** ([`Signal`]) and **six verbs**:
//!
//! | Trait | Purpose |
//! |---|---|
//! | [`Store`] | Store and query signals |
//! | [`Score`](traits::Score) | Rate signals along multi-dimensional axes |
//! | [`Verify`] | Verify signals against ground truth |
//! | [`Route`] | Select one signal from many candidates |
//! | [`Compose`] | Combine signals into a new signal under a budget |
//! | [`React`] | Watch signal streams and emit new signals (interventions) |
//!
//! Every capability — coding-agent spawning, verification gates, context assembly,
//! model routing, memory retrieval, chain participation, bounty markets, HDC search
//! — is one of these six verbs operating on Signals.
//!
//! # The Universal Loop
//!
//! Every operation in Roko follows the same shape:
//!
//! ```text
//! query substrate → score → route/compose → gate verdict → write back → policy fires
//! ```
//!
//! See [`select_compose_verify_persist`] for the concrete implementation.
//!
//! # Extensibility
//!
//! All enums marked `#[non_exhaustive]` can grow without breaking downstream impls.
//! New capabilities are added by implementing one of the six traits; no core
//! changes are needed.

#![allow(
    missing_docs,
    clippy::cloned_ref_to_slice_refs,
    clippy::collapsible_str_replace,
    clippy::derivable_impls,
    clippy::expect_used,
    clippy::module_name_repetitions,
    clippy::nursery,
    clippy::obfuscated_if_else,
    clippy::pedantic
)]

/// Generate a short share token: `<unix_millis_hex>-<rand_hex>`.
#[must_use]
pub fn generate_share_token() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};

    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let rand_part: u16 = (millis as u16) ^ (std::process::id() as u16);
    format!("{millis:x}-{rand_part:04x}")
}

pub mod affect;
pub mod agent;
/// Cross-cut arbitration protocol for resolving Daimon/Neuro/Dreams conflicts (INT-21).
pub mod arbitration;
pub mod attestation;
pub mod body;
pub mod build;
/// Additional Bus backend implementations: BroadcastBus, MemoryBus, MultiBus.
pub mod bus_backends;
pub mod capabilities;
pub mod catalyst;
/// The Cell trait — universal computation unit for all protocol implementations.
pub mod cell;
pub mod cfactor;
/// Canonical provider-agnostic chat message types.
pub mod chat_types;
pub mod cognitive_workspace;
pub mod conductor;
pub mod config;
/// Connector trait for external system I/O (MCP, API, Database, Blockchain, Feed, Custom).
pub mod connector;
pub mod context;
/// Five-head lexicographic corrigibility ordering (Nayebi 2024).
pub mod corrigibility;
/// Durable crash report written by the global panic hook.
pub mod crash_report;
pub mod dashboard_snapshot;
pub mod datum;
pub mod decay;
/// Central constants — import from here instead of hardcoding magic numbers.
pub mod defaults;

pub mod demurrage;
/// Canonical single-unit duration parser (`ms`, `s`, `m`, `h`, `d`).
pub mod duration;
pub mod dispatch_plan;
/// Domain profiles for agent specialization: gate defaults, tool sets, context templates.
pub mod domain_profile;
pub mod engram;
pub mod error;
/// Payload contracts carried by MCP, A2A, and x402 transports.
pub mod exoskeleton;
pub mod extension;
/// Feed trait for agent-produced data streams (Raw, Derived, Composite, Meta).
pub mod feed;
/// Feed-to-Bus routing over the canonical Bus contract.
pub mod feed_bus_bridge;
/// Runtime feed Cell primitives.
pub mod feed_cell;
/// Supervised feed lifecycle and discovery.
pub mod feed_runtime;
/// Built-in and derived feed implementations.
pub mod feeds;
/// Forensic replay engine for causal decision reconstruction (SAFE-12).
pub mod forensic;
pub mod foundation;
/// Shared GitOps types: `GitOpsConfig`, `GitOpsRetryPolicy`, `ConfigDrift`.
pub mod gitops;
pub mod groups;
pub mod hash;
/// Heartbeat protocol types for health monitoring.
pub mod heartbeat;
/// Cognitive immune system -- quarantine, anomaly detection, incident linking.
pub mod immune;
/// Atomic file I/O utilities for crash-safe state persistence.
pub mod io;
/// Marketplace job types shared between serve, TUI, and CLI.
pub mod job;
pub mod kind;
pub mod language;
pub mod lens_circuit_breaker;
pub mod lens_registry;
pub mod loop_tick;
pub mod metric;
pub mod namespace;
pub mod obs {
    #[path = "../obs/health.rs"]
    pub mod health;
    #[path = "../obs/histograms.rs"]
    pub mod histograms;
    #[path = "../obs/lens.rs"]
    pub mod lens;
    #[path = "../obs/metrics.rs"]
    pub mod metrics;
    #[path = "../obs/schema.rs"]
    pub mod schema;
    #[path = "../obs/scrub.rs"]
    pub mod scrub;
    #[path = "../obs/telemetry_observe.rs"]
    pub mod telemetry_observe;

    pub use health::{
        AlwaysUpProbe, DegradedReason, HealthStatus, NamedProbe, Probe, ProbeRegistry,
        ReadinessStatus,
    };
    pub use histograms::{Histogram, HistogramSnapshot, LLM_LATENCY_BUCKETS};
    pub use lens::{
        CollectorLens, CostLens, LatencyLens, Lens, LensRegistry, LensScope, LensSnapshot,
        TokenUsageLens, default_registry,
    };
    pub use metrics::{
        Counter, Gauge, LabelSet, MetricKind, MetricRegistry, MetricSnapshot, MetricValue,
        STANDARD_METRICS, register_standard_metrics,
    };
    pub use schema::{CanonicalMetricSchema, MetricDescriptor, MetricSchema, SCHEMA_VERSION};
    pub use scrub::{LogScrubber, REDACTED};
    pub use telemetry_observe::{PeriodicObserver, TelemetryObservation, TelemetryObserve};
}
pub mod operating_frequency;
pub mod phase;
/// Executor-neutral plan mutation contract (v1) — deterministic DAG mutation kernel.
pub mod plan_mutation;
pub mod plugin;
pub mod policy_manifest;
pub mod polyglot;
pub mod prediction;
pub mod project;
pub mod provenance;
pub mod provider_catalog;
pub mod pulse;
pub mod query;
/// Pure-data DAGs of score operations.
pub mod recipe;
/// Atomic TOML recipe persistence.
pub mod recipe_store;
/// Shared retention policy type for all data-management subsystems.
pub mod retention;
pub mod runtime_event;
pub mod score;
pub mod secrets;
pub mod shutdown;
/// Signal — the primary type name (re-exports from `engram` module).
pub mod signal;
pub mod signal_kinds;
pub mod task;
pub mod telemetry_observe;
pub mod telemetry_projections;
pub mod temperament;
pub mod todo_dag;
pub mod tool;
pub mod traits;
/// Bounded in-memory transcript store with lossless control events.
pub mod transcript_store;
/// Trigger protocol types — TriggerProtocol, TriggerBinding, TriggerEvent, TriggerHandle,
/// TriggerState, TriggerSource, and related configuration types.
pub mod trigger;
pub mod usage;
pub mod verdict;
/// Relay envelopes, room names, recovery, and backpressure contracts.
pub mod wire_protocol;
pub mod workspace;

pub use affect::{BehavioralState, DaimonPolicy, EmotionalTag, PadVector};
pub use agent::{
    AgentBackend, AgentRole, ModelSpec, ModelTier, ProviderKind, ReasoningEffort, TaskRequirements,
    ToolPermissions, TurnBudget, score_model_for_task, select_model_for_task,
    select_model_for_task_with_bonus,
};
pub use arbitration::{
    ArbitrationConfig, ArbitrationOutcome, Arbitrator, Subsystem, SubsystemGuidance,
};
pub use attestation::{Attestation, ChainAttestation, Ed25519Signature, PublicKey};
pub use body::Body;
pub use build::{BuildCommand, BuildSystem};
pub use bus_backends::{
    BroadcastBus, BroadcastBusReceiver, BusErased, MemoryBus, MemoryBusReceiver, MultiBus,
};
pub use capabilities::{
    CachedCapabilityChecker, Capability, CapabilityCheck, CapabilitySet, CellCapabilities,
    GraphAllowList, SpaceGrant, capabilities_for_taint, effective_capabilities,
};
pub use catalyst::{CatalystImpactSummary, CatalystScorer, CatalystSignalSource};
pub use cell::*;
pub use cfactor::{CFactorPolicy, CFactorSource, CFactorSummary};
pub use chat_types::{
    ChatMessage, ChatRequest, ChatResponse, ContentBlock, FinishReason, ImageUrl, MessageContent,
    RequestOptions, ResponseFormat, ResponseMetadata, SessionState, ToolCallFunction,
    ToolCallMessage, ToolChoice, Usage,
};
pub use cognitive_workspace::{
    COGNITIVE_WORKSPACE_SCHEMA_VERSION, CapabilityGrant, CognitiveWorkspace, ContextPolicyAuditRef,
    ContextRejectionAudit, ContextRejectionAuditReason, ContextScopeAudit, ContextSectionAudit,
    InvocationGateOutcome, InvocationReviewVerdictOutcome, ModelChoice, OutputParseResult,
    PolicyVersionRef, PromptSectionAudit, RewardObservation, TaskInvocationContract,
};
pub use conductor::{CognitiveSignal, ConductorDecision, ConductorEvaluation};
pub use crash_report::{
    CrashReport, build_crash_report, clear_active_plan, clear_active_provider, clear_active_task,
    crash_report_path, has_recent_crash_report, read_crash_report, set_active_plan,
    set_active_provider, set_active_task, write_crash_report, CRASH_REPORT_FILENAME,
};
pub use config::graduation::{GraduationConfig, GraduationPolicy};
pub use connector::{
    ConnectorConfig, ConnectorHealth, ConnectorInfo, ConnectorKind, ConnectorRegistry,
    ConnectorStatus,
};
pub use context::Context;
pub use datum::Datum;
pub use decay::Decay;
pub use demurrage::{Demurrage, DemurrageConfig, demurrage_tick};
pub use duration::{DurationParseError, parse_duration, parse_duration_ms};
pub use dispatch_plan::{
    ConfigBag, DispatchAttempt, DispatchAttemptKind, DispatchAuthStatus, DispatchCaller,
    DispatchPlan, DispatchRequest, DispatchRequirement, FallbackPolicy, ProviderDispatchError,
    TransportAuth, TransportPlan,
};
pub use domain_profile::{DomainProfile, TypedContext};
pub use engram::{Engram, EngramBuilder, GraduationError, HdcFingerprint, SignalStatus};
pub use error::{Result, RokoError};
pub use feed::{
    FeedAccess, FeedInfo, FeedKind, FeedPricingConfig, FeedRegistry, FeedRuntimeStatus,
    PaymentProtocol, PricingTier, SessionPricing,
};
pub use feed_bus_bridge::{FeedBusBridge, FeedRouteStats};
pub use feed_cell::{
    ConnectorOps, FeedCell, FeedCellConfig, FeedPulse, FeedStatus, FeedTrigger, FeedTriggerOps,
    MemoryFeedStore, NoopConnector, StoreOps, UnavailableTrigger,
};
pub use feed_runtime::{FeedHandle, ReconnectPolicy, RuntimeRegistry};
pub use feeds::{
    DerivedFeedCell, EpisodeOutcomeFeed, EpisodeOutcomeFeedConfig, FeedTransform, FileWatchFeed,
    ProviderHealthFeed, ProviderHealthSample, ProviderHealthSnapshot,
};
pub use forensic::{
    ForensicReplay, ForensicReplayLogger, GateVerdictRecord, PolicyDecisionRecord, PolicyOutcome,
    ReconstructionStep, RouterAlternative, RouterDecisionRecord, ScoredReference, StepStatus,
};
pub use foundation::{
    BoxModelStream, ChatMessage as FoundationChatMessage, Effect, EffectExecutor, EffectOutcome,
    EventConsumer, FeedbackEvent, FeedbackSink, GateClassification, GateConfig, GateReport,
    GateRunner, GateVerdict, MessageRole, ModelCallRequest, ModelCallResponse, ModelCaller,
    ModelInputBlock, ModelInputImage, ModelInputMessage, ModelStreamEvent, PromptAssembler,
    PromptSpec, ShellGateCommand, TokenUsage, model_call_failure_to_stream,
    model_call_response_to_stream, validate_model_input_images, validate_model_input_messages,
};
pub use groups::{
    AssignmentStrategy, CoordinationMode, Group, GroupConfig, GroupContextBidder, GroupEvent,
    GroupId, GroupInvitation, GroupMember, GroupPheromone, InvitationId, InvitationStatus,
    InviteRequest, InviteResponse, KnowledgePolicy, LeaderConfig, MemberPermissions, MemberRole,
    PheromoneDeposit, PheromoneFieldSummary, PheromoneQuery, TaskAssignment, TaskCompletion,
};
pub use hash::ContentHash;
pub use heartbeat::{
    AgentEndpoints, DEFAULT_HEARTBEAT_INTERVAL_SECS, HEARTBEAT_RING_CAPACITY, HeartbeatPayload,
    NetworkStats, SenderInfo,
};
pub use immune::{
    AnomalyScore, DEFAULT_QUARANTINE_VAULT_CAPACITY, ImmuneAssessment, ImmuneContainment,
    ImmunePerception, ImmunePipeline, ImmunePipelineResult, ImmuneResponse, ImmuneValidation,
    IncidentLink, IncidentRelation, MAX_QUARANTINE_VAULT_BYTES, QuarantineDecision,
    QuarantineEntry, QuarantineStatus, QuarantineVault, ResponseAction, ThreatSeverity,
};
pub use kind::{Kind, KindEntry, KindRegistry};
pub use language::{Import, ImportKind, LanguageProvider, Symbol, SymbolKind, Visibility};
pub use loop_tick::{SignalSelectionOutcome, select_compose_verify_persist};
pub use metric::{ConfigHash, Headlines, TaskMetric, compute_headlines};
pub use operating_frequency::{
    OperatingFrequency, OperatingFrequencyAffect, OperatingFrequencyScheduleContext,
    OperatingFrequencyScheduler,
};
pub use phase::{FailureKind, PhaseKind, PlanPhase, is_monotonic_progression, valid_transitions};
pub use plan_mutation::{
    MutationAuthorKind, MutationAuthorV1, MutationEvidenceV1, MutablePlanV1, MutableTaskV1,
    PlanMutationErrorV1, PlanMutationOpV1, PlanMutationResultV1, PlanMutationV1, apply_mutation,
    canonical_fingerprint,
};
pub use policy_manifest::{
    BUILTIN_ROLE_POLICY_MANIFEST_PATH, BUILTIN_ROLE_POLICY_MANIFEST_TOML,
    CURRENT_POLICY_MANIFEST_SCHEMA_VERSION, CapabilityDeclaration, ContextPolicyRef,
    EffectiveCapabilities, FallbackBehavior, GateExpectation, InclusionMode, InclusionRule,
    MANIFEST_BACKED_BUILTIN_ROLE_IDS, ManifestError, ManifestLookupError, ManifestValidationError,
    OutputFormat, OutputSchemaExpectation, PolicyProvenance, PromptBudgetPolicy, PromptPolicy,
    PromptPolicySection, PromptSectionSource, RolePolicyManifest, RoleProfile, RoleSafetyPolicy,
    SectionBudget, ToolCapabilityPolicy, intersect_capabilities,
};
pub use polyglot::{PolyglotProject, detect_polyglot};
pub use prediction::{
    AccuracyStats, CalibrationStats, CalibrationTracker, ChainCondition, ChainMetric,
    ChainQueryPayload, ChainTarget, ChangeContext, CodingMetric, CodingQueryPayload, CodingScope,
    ExponentialMovingAverage, OperationsMetric, OperationsQueryPayload, Oracle, OracleDomain,
    OracleQuery, PredictedValue, Prediction, PredictionAccuracy, PredictionCalibrationSource,
    PredictionCalibrationSummary, PredictionInterval, PredictionOutcome, PredictionPolicy,
    PredictionProvenance, PredictionStore, PredictiveScorer, QueryPayload, ResearchMetric,
    ResearchQueryPayload, ResidualCorrector, SourceReference,
};
pub use project::{
    DetectedBuildSystem, Language, ProjectInfo, detect_from_files,
    detect_from_files_with_cargo_toml,
};
pub use provenance::{
    Provenance, ProvenanceCoherenceCheck, ProvenanceCoherenceIssue, Taint, TaintInfo, TaintLevel,
};
pub use pulse::{PolicyOutputs, Pulse, PulseBuilder, Topic, TopicFilter};
pub use query::{Budget, Query};
pub use recipe::{Recipe, RecipeEdge, RecipeNode, ScoreOp};
pub use recipe_store::RecipeStore;
pub use roko_primitives::HdcVector;
pub use runtime_event::{
    RuntimeEvent, RuntimeEventDelivery, RuntimeEventMode, RuntimeEventProjector,
    RuntimeEventPublishDisposition, RuntimeEventPublisher, ToolCallSummary, WorkflowOutcome,
};
pub use score::Score;
pub use signal::{ArtifactKind, ArtifactLineage, ArtifactRef, Signal, SignalBuilder};
pub use signal_kinds::*;
pub use task::{
    GlobalTaskId, PlanStatus, Task, TaskCategory, TaskComplexityBand, TaskContextWeight,
    TaskDomain, TaskMeta, TaskQualityProfile, TaskReasoningLevel, TaskSpeedPriority, TaskStatus,
};
pub use usage::{UsageObservation, UsageSource};
// Note: tool::FailureKind (for tool-call failures) is NOT re-exported here to avoid
// collision with phase::FailureKind (for PlanPhase failures); reach it via
// `roko_core::tool::FailureKind`.
pub use dashboard_snapshot::{
    AffectSnapshot, AgentState as DashboardAgentState, AgentTopology, AgentTopologyEdge,
    AgentTopologyNode, CFactorBucket, DashboardEvent, DashboardEventLogEntry, DashboardSnapshot,
    DiagnosisSeverity, DiagnosisSummary, EfficiencyBucket, ExperimentWinnerSummary, FailureEntry,
    KnowledgeBrowseEntry as DashboardKnowledgeBrowseEntry, SnapshotStats, TrendBucket,
    TrendBuckets,
};
pub use job::{
    CreateJobRequest, FileJobStore, JobError, JobEvaluation, JobFilter, JobGateResult,
    JobPriority, JobProgressEntry, JobStats, JobStatus, JobSubmission, JobType,
    LegacyMigrationDiagnostic, MalformedJobFile, MarketplaceJob, PrdSummary,
    TaskSummary as JobTaskSummary,
};
pub use lens_registry::{LensConfig, LensRegistration, LensRegistry, parse_scope};
pub use namespace::{
    Channel, ChannelDirection, CognitiveNamespace, NamespaceAcl, NamespaceRegistry, RateLimitConfig,
};
pub use telemetry_observe::{
    AlertLevel, AnomalyDirection, AnomalyLevel, AnomalyPayload, BudgetAlertPayload, CFactorPayload,
    CostReportPayload, DriftPayload, EfficiencyPayload, ErrorCategory, ErrorPayload,
    LatencyPayload, LensScope, ObservableEvent, ObservableEventKind, PassFailCounts,
    QualityPayload, TelemetryEventSink, TelemetryObserve, TrendDirection, TrendPayload,
    UsagePayload, citation_reciprocity, delivery_rate, hdc_diversity, peer_prediction_accuracy,
    turn_taking_entropy,
};
pub use temperament::Temperament;
pub use todo_dag::{
    TodoChanges, TodoDag, TodoDagError, TodoDelta, TodoItem, TodoSnapshot, TodoSource, TodoStatus,
};
pub use tool::{
    ArmEntry, Artifact, AuditSink, BanditKey, CancelSource, CancelToken, EpsilonGreedyBandit,
    FailureTrace, FormatBandit, KeywordOverlapScorer, MemoryPointer, MetricsKey, MetricsSink,
    ProfileBandit, RewardConfig, ToolCall, ToolCategory, ToolConcurrency, ToolContext, ToolDef,
    ToolError, ToolExecutionEnvelope, ToolExecutionRecord, ToolFormat, ToolFormatProfile,
    ToolHandler, ToolLifecycleStatus, ToolMetrics, ToolOutcome, ToolPermission, ToolRegistry,
    ToolRelevanceScorer, ToolResult, ToolSchema, ToolSource, ToolTrace, ToolTraceEvent,
    TraceBuilder, TraceFinishGuard, TraceId, TraceSink, TraceStep, TranscriptEvent,
    TranscriptEventMeta, TranscriptRecord, VecToolRegistry, classify_tool_error, compute_reward,
    galileo_tsq, profile_for_model,
};
pub use traits::{
    Bus, ColdStore, Compose, Connect, Observe, React, Route, Store, Substrate, Trigger, Verify,
};
pub use transcript_store::{
    ChannelDropReport, PriorityEventChannel, ReplayMismatch, StoreError, StoreStats,
    TranscriptFilter, TranscriptPage, TranscriptReplayContract, TranscriptStore,
};
pub use trigger::{
    Author, BusTrigger, ChainEventTrigger, ConcurrencyPolicy, CronTrigger, Expr, FileWatchEvent,
    FileWatchTrigger, GRADUATION_EVENTS, GraphRef, InputFieldMapping, RateLimit, RateLimitAction,
    SecretRef, SignalPatternTrigger, SignalRef, SpaceId, TRIGGER_AUTH_FAILED, TRIGGER_CREATED,
    TRIGGER_DELETED, TRIGGER_FIRED, TRIGGER_RATE_LIMITED, TraceId as TriggerTraceId, TriggerAuth,
    TriggerBinding, TriggerEvent, TriggerEventKind, TriggerFilter, TriggerGraduationPolicy,
    TriggerHandle, TriggerHistory, TriggerHistoryRecord, TriggerId, TriggerInputMapping,
    TriggerKind, TriggerProtocol, TriggerSource, TriggerState, WebhookTrigger,
    load_trigger_history, trigger_topic,
};
// Note: The `Score` protocol trait (formerly `Scorer`) is NOT re-exported at
// the crate root to avoid colliding with the `Score` value struct. Access it
// via `roko_core::traits::Score` or import with an alias.
pub use verdict::{
    Outcome, PublishPipelineResult, PublishStage, PublishStageResult, Selection, TestCount, Verdict,
};
#[allow(deprecated)]
pub use workspace::Workspace;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn share_token_format() {
        let token = generate_share_token();
        // Token must be non-empty and contain a hyphen separator.
        assert!(!token.is_empty(), "token should not be empty");
        assert!(
            token.contains('-'),
            "token should contain a hyphen: {token}"
        );
        // Both parts should be valid hex.
        let parts: Vec<&str> = token.splitn(2, '-').collect();
        assert_eq!(parts.len(), 2, "token should have two parts: {token}");
        assert!(
            u128::from_str_radix(parts[0], 16).is_ok(),
            "first part should be hex: {}",
            parts[0]
        );
        assert!(
            u16::from_str_radix(parts[1], 16).is_ok(),
            "second part should be 4-char hex: {}",
            parts[1]
        );
    }
}
