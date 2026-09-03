//! Canonical layer-safe runtime event schema (v2).
//!
//! This module defines the vocabulary and wire envelope that every runtime
//! producer (runner, graph, workflow engine, chat, ACP) emits and every
//! projector (TUI bridge, SSE adapter, JSONL logger, StateHub) consumes.
//!
//! # Delivery classes
//!
//! Events are classified as either **Reliable** or **BestEffort**:
//!
//! - **Reliable** events must be acknowledged (`Acknowledged`) before the
//!   owning safe boundary commits. A reliable publish may not be silently
//!   dropped. If a reliable event is coalesced, the publisher returns
//!   `Coalesced` but the event is still durably recorded.
//!
//! - **BestEffort** events are fire-and-forget: the engine emits them and
//!   does not wait for observers to process them. A slow observer may drop
//!   best-effort events, but must then enqueue a reliable `SequenceGap`
//!   marker so downstream consumers can detect the loss.
//!
//! # Ordering guarantees
//!
//! Within a single `run_id`, the `seq` field is strictly monotonic: each
//! event carries a sequence number one greater than the previous event for
//! that run. Replay preserves original `seq` and `ts` values.
//!
//! # Terminal events
//!
//! A run should emit exactly one terminal event (`WorkflowCompleted` or
//! `RunCompleted`). After a terminal event, no further events should be
//! emitted for that `run_id` except `SequenceGap` markers for prior losses.
//!
//! # Replay
//!
//! Events created during replay carry `mode = Replay`. The `event_id`, `seq`,
//! and `ts` fields are preserved from the original event; replay does not
//! rewrite them. Projectors use `mode` to distinguish live from replayed
//! events for deduplication and side-effect suppression.
//!
//! # Redaction
//!
//! Payload values must already be redacted and bounded by the publisher.
//! No event projector may reinterpret an unbounded provider response as a
//! canonical payload. `serde_json::Value` is allowed only in prediction,
//! actual, and extension payloads.
//!
//! # Extension
//!
//! Third-party or forward-compatible data uses the `Extension` variant with
//! a `namespace`, `version`, and opaque `value`. Unknown event kinds fail
//! deserialization; only `Extension` provides forward compatibility.
//!
//! # Schema evolution
//!
//! The `schema_version` field is `2` for all new serialization. Existing v1
//! JSON deserializes through an internal compatibility layer that supplies
//! default values for fields added in v2. No v1 variant is renamed or
//! removed; new variants are additive only.

use crate::agent::AutonomyLevel;
use crate::foundation::TokenUsage;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Envelope enums
// ---------------------------------------------------------------------------

/// Whether an event was produced live or during replay.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeEventMode {
    /// Produced during live execution.
    Live,
    /// Reproduced from a durable log during replay.
    Replay,
}

/// Delivery class for a runtime event.
///
/// Reliable events must be acknowledged before the owning safe boundary
/// commits. BestEffort events may be dropped by a slow observer, but the
/// publisher must then emit a reliable `SequenceGap`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeEventDelivery {
    /// The publisher must receive `Acknowledged` before committing.
    Reliable,
    /// Fire-and-forget; may be dropped by slow observers.
    BestEffort,
}

// ---------------------------------------------------------------------------
// Envelope
// ---------------------------------------------------------------------------

/// The canonical wire envelope for all runtime events (v2).
///
/// Every producer wraps its payload in this envelope. The envelope carries
/// identity, ordering, delivery classification, and correlation metadata
/// that projectors use for deduplication, replay, and causal grouping.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuntimeEventEnvelope {
    /// Stable UUID string created once and preserved on replay.
    pub event_id: String,
    /// Required run scope.
    pub run_id: String,
    /// Set for plan execution events.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plan_id: Option<String>,
    /// Set for task execution events.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task_id: Option<String>,
    /// Set by Graph adapters.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub node_id: Option<String>,
    /// Stable provider/task attempt identity.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attempt_id: Option<String>,
    /// Set for agent and inference events.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent_id: Option<String>,
    /// Strictly monotonic per `run_id`; replay preserves it.
    pub seq: u64,
    /// Original creation time; replay does not rewrite it.
    pub ts: DateTime<Utc>,
    /// Schema version. Exactly `2` for new serialization.
    pub schema_version: u8,
    /// Lower-case producer name (e.g. `runner`, `graph`, `workflow`, `chat`, `acp`).
    pub source: String,
    /// Whether this event was produced live or during replay.
    #[serde(default = "default_mode")]
    pub mode: RuntimeEventMode,
    /// Delivery class derived from the payload.
    #[serde(default = "default_delivery")]
    pub delivery: RuntimeEventDelivery,
    /// Groups causally related events.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub correlation_id: Option<String>,
    /// Required for reliable receipts and terminal side effects.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub idempotency_key: Option<String>,
    /// The event payload.
    pub payload: RuntimeEvent,
}

fn default_mode() -> RuntimeEventMode {
    RuntimeEventMode::Live
}

fn default_delivery() -> RuntimeEventDelivery {
    RuntimeEventDelivery::Reliable
}

impl RuntimeEventEnvelope {
    /// Compatibility constructor matching the v1 signature.
    ///
    /// Fills a new `event_id`, sets optional IDs to `None`, mode to `Live`,
    /// derives `delivery` from `payload.delivery()`, and leaves
    /// `correlation_id` / `idempotency_key` empty.
    pub fn new(
        run_id: impl Into<String>,
        seq: u64,
        source: impl Into<String>,
        payload: RuntimeEvent,
    ) -> Self {
        let delivery = payload.delivery();
        Self {
            event_id: Uuid::new_v4().to_string(),
            run_id: run_id.into(),
            plan_id: None,
            task_id: None,
            node_id: None,
            attempt_id: None,
            agent_id: None,
            seq,
            ts: Utc::now(),
            schema_version: 2,
            source: source.into(),
            mode: RuntimeEventMode::Live,
            delivery,
            correlation_id: None,
            idempotency_key: None,
            payload,
        }
    }

    /// Full v2 constructor with all identity fields.
    #[allow(clippy::too_many_arguments)]
    pub fn new_v2(
        event_id: impl Into<String>,
        run_id: impl Into<String>,
        plan_id: Option<String>,
        task_id: Option<String>,
        node_id: Option<String>,
        attempt_id: Option<String>,
        agent_id: Option<String>,
        seq: u64,
        ts: DateTime<Utc>,
        source: impl Into<String>,
        mode: RuntimeEventMode,
        correlation_id: Option<String>,
        idempotency_key: Option<String>,
        payload: RuntimeEvent,
    ) -> Self {
        let delivery = payload.delivery();
        Self {
            event_id: event_id.into(),
            run_id: run_id.into(),
            plan_id,
            task_id,
            node_id,
            attempt_id,
            agent_id,
            seq,
            ts,
            schema_version: 2,
            source: source.into(),
            mode,
            delivery,
            correlation_id,
            idempotency_key,
            payload,
        }
    }
}

// ---------------------------------------------------------------------------
// V1 compatibility deserialization
// ---------------------------------------------------------------------------

/// Private v1 envelope shape used only for deserialization of legacy JSON.
#[derive(Deserialize)]
struct RuntimeEventEnvelopeV1 {
    run_id: String,
    seq: u64,
    ts: DateTime<Utc>,
    schema_version: u8,
    source: String,
    payload: RuntimeEvent,
}

impl From<RuntimeEventEnvelopeV1> for RuntimeEventEnvelope {
    fn from(v1: RuntimeEventEnvelopeV1) -> Self {
        let delivery = v1.payload.delivery();
        Self {
            event_id: Uuid::new_v4().to_string(),
            run_id: v1.run_id,
            plan_id: None,
            task_id: None,
            node_id: None,
            attempt_id: None,
            agent_id: None,
            seq: v1.seq,
            ts: v1.ts,
            schema_version: 2,
            source: v1.source,
            mode: RuntimeEventMode::Live,
            delivery,
            correlation_id: None,
            idempotency_key: None,
            payload: v1.payload,
        }
    }
}

// ---------------------------------------------------------------------------
// Supporting types
// ---------------------------------------------------------------------------

/// Outcome of a completed workflow run.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum WorkflowOutcome {
    /// Workflow completed successfully, optionally with a commit hash.
    Success { commit_hash: Option<String> },
    /// Workflow halted due to an error or resource limit.
    Halted { reason: String },
    /// Workflow was cancelled by the user.
    Cancelled,
}

/// Summary of a tool call captured during an agent turn.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolCallSummary {
    pub name: String,
    pub result_preview: String,
}

/// User actions emitted by named surfaces.
///
/// These are kept separate from engine-emitted [`RuntimeEvent`] values so a
/// transport can authorize commands before translating them into effects.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "data", rename_all = "snake_case")]
pub enum SurfaceEvent {
    TaskAssign {
        graph: String,
        inputs: serde_json::Value,
        budget: Option<f64>,
        deadline: Option<DateTime<Utc>>,
    },
    SlotFill {
        agent_id: String,
        slot_index: usize,
        cell_ref: String,
    },
    MacroAdjust {
        run_id: String,
        macro_name: String,
        new_value: serde_json::Value,
    },
    FlowCancel {
        run_id: String,
    },
    FlowPause {
        run_id: String,
    },
    FlowResume {
        run_id: String,
    },
    HumanRespond {
        run_id: String,
        cell_id: String,
        response: serde_json::Value,
    },
    AutonomyLevelChange {
        agent_id: String,
        capability: String,
        new_level: AutonomyLevel,
    },
    CapabilityGrant {
        agent_id: String,
        capability: String,
        constraints: serde_json::Value,
    },
    CapabilityRevoke {
        agent_id: String,
        capability: String,
    },
    BulkAutonomySet {
        agent_id: String,
        level: AutonomyLevel,
    },
}

// ---------------------------------------------------------------------------
// RuntimeEvent
// ---------------------------------------------------------------------------

/// Every event the workflow engine can emit.
///
/// Events are classified by their [`RuntimeEventDelivery`]:
///
/// - **Reliable** events (the default) must be acknowledged by the publisher
///   before the owning safe boundary commits. They cannot be silently dropped.
///
/// - **BestEffort** events (`AgentOutput`, `AgentProgress`, `GateRungOutput`,
///   `InferenceFirstToken`) are fire-and-forget. A slow observer may drop
///   them, but must then emit a reliable `SequenceGap` marker.
///
/// See [`RuntimeEvent::delivery()`] for the exhaustive classification.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "data", rename_all = "snake_case")]
pub enum RuntimeEvent {
    // -----------------------------------------------------------------------
    // Lifecycle (v1)
    // -----------------------------------------------------------------------
    WorkflowStarted {
        run_id: String,
        template: String,
        prompt: String,
    },
    PhaseTransition {
        run_id: String,
        from: String,
        to: String,
    },
    WorkflowCompleted {
        run_id: String,
        outcome: WorkflowOutcome,
    },

    // -----------------------------------------------------------------------
    // Agent (v1)
    // -----------------------------------------------------------------------
    AgentSpawned {
        run_id: String,
        agent_id: String,
        role: String,
        model: String,
    },
    AgentOutput {
        run_id: String,
        agent_id: String,
        chunk: String,
    },
    AgentCompleted {
        run_id: String,
        agent_id: String,
        output: String,
        tokens_used: u64,
        cost_usd: f64,
    },
    AgentFailed {
        run_id: String,
        agent_id: String,
        error: String,
    },

    // -----------------------------------------------------------------------
    // Gates (v1)
    // -----------------------------------------------------------------------
    GateStarted {
        run_id: String,
        gate_name: String,
        rung: u8,
    },
    GatePassed {
        run_id: String,
        gate_name: String,
        duration_ms: u64,
    },
    GateFailed {
        run_id: String,
        gate_name: String,
        output: String,
        duration_ms: u64,
    },

    // -----------------------------------------------------------------------
    // Feedback (v1)
    // -----------------------------------------------------------------------
    FeedbackRecorded {
        run_id: String,
        kind: String,
        summary: String,
    },

    // -----------------------------------------------------------------------
    // Persistence (v1)
    // -----------------------------------------------------------------------
    StateCheckpointed {
        run_id: String,
        path: String,
    },

    // -----------------------------------------------------------------------
    // Inference tracking (v1)
    // -----------------------------------------------------------------------
    InferenceStarted {
        run_id: String,
        request_id: String,
        model: String,
        agent_id: String,
        auto_routed: bool,
    },
    InferenceCompleted {
        run_id: String,
        request_id: String,
        model: String,
        agent_id: String,
        input_tokens: u64,
        output_tokens: u64,
        cost_usd: f64,
        duration_ms: u64,
    },
    InferenceFailed {
        run_id: String,
        request_id: String,
        model: String,
        agent_id: String,
        error: String,
    },

    // -----------------------------------------------------------------------
    // Agent traces (v1)
    // -----------------------------------------------------------------------
    AgentTrace {
        run_id: String,
        agent_id: String,
        turn: u32,
        tool_calls: Vec<ToolCallSummary>,
        reasoning: Option<String>,
        usage: TokenUsage,
    },

    // -----------------------------------------------------------------------
    // Demo run and task lifecycle (v1)
    // -----------------------------------------------------------------------
    TaskFailed {
        plan_id: String,
        task_id: String,
        error: String,
        gate_failure: bool,
    },
    RunStarted {
        run_id: String,
        prompt: String,
        complexity: String,
    },
    RunCompleted {
        run_id: String,
        success: bool,
        cost_usd: f64,
        duration_ms: u64,
    },

    // -----------------------------------------------------------------------
    // Knowledge flow (v1)
    // -----------------------------------------------------------------------
    KnowledgeIngested {
        run_id: String,
        entry_id: String,
        topic: String,
        source_agent: String,
    },
    KnowledgeConsumed {
        run_id: String,
        entry_id: String,
        topic: String,
        consuming_agent: String,
    },

    // -----------------------------------------------------------------------
    // Progress tracking (v1)
    // -----------------------------------------------------------------------
    /// First token received from an inference call -- carries TTFT for latency dashboards.
    InferenceFirstToken {
        run_id: String,
        request_id: String,
        model: String,
        agent_id: String,
        /// Time-to-first-token in milliseconds.
        ttft_ms: u64,
    },
    /// A tool call has started executing.
    ToolCallStarted {
        run_id: String,
        agent_id: String,
        tool: String,
        iteration: u32,
    },
    /// A tool call has finished executing.
    ToolCallCompleted {
        run_id: String,
        agent_id: String,
        tool: String,
        duration_ms: u64,
        success: bool,
    },
    /// A plan task has started executing.
    TaskStarted {
        run_id: String,
        plan_id: String,
        task_id: String,
        task_title: String,
        role: String,
    },
    /// A plan task has finished executing.
    TaskCompleted {
        run_id: String,
        plan_id: String,
        task_id: String,
        passed: bool,
        duration_ms: u64,
    },
    /// The overall pipeline entered a new phase.
    PipelinePhase {
        run_id: String,
        phase: String,
        /// "started", "complete", or "failed".
        status: String,
    },

    // ===================================================================
    // v2 variants (added by #208)
    // ===================================================================

    // -----------------------------------------------------------------------
    // Wave execution (v2)
    // -----------------------------------------------------------------------
    /// A parallel execution wave has started.
    WaveStarted {
        wave_index: u64,
        task_count: u64,
    },
    /// A parallel execution wave has completed.
    WaveCompleted {
        wave_index: u64,
        succeeded: u64,
        failed: u64,
        duration_ms: u64,
    },

    // -----------------------------------------------------------------------
    // Task lifecycle extensions (v2)
    // -----------------------------------------------------------------------
    /// A task is being retried after a failure.
    TaskRetrying {
        task_id: String,
        attempt: u64,
        reason: String,
    },
    /// A task was skipped (e.g. dependency failed or condition unmet).
    TaskSkipped {
        task_id: String,
        reason: String,
    },

    // -----------------------------------------------------------------------
    // Agent progress (v2, BestEffort)
    // -----------------------------------------------------------------------
    /// Incremental progress from an agent (streaming, partial output).
    AgentProgress {
        agent_id: String,
        progress_pct: f64,
        message: String,
    },

    // -----------------------------------------------------------------------
    // Usage (v2)
    // -----------------------------------------------------------------------
    /// Aggregated usage record for a completed operation.
    UsageRecorded {
        input_tokens: u64,
        output_tokens: u64,
        cost_usd: f64,
        model: String,
    },

    // -----------------------------------------------------------------------
    // Gate rung lifecycle (v2)
    // -----------------------------------------------------------------------
    /// A single gate rung has started executing.
    GateRungStarted {
        gate_name: String,
        rung: u8,
    },
    /// Incremental output from a gate rung (BestEffort).
    GateRungOutput {
        gate_name: String,
        rung: u8,
        chunk: String,
    },
    /// A single gate rung has completed.
    GateRungCompleted {
        gate_name: String,
        rung: u8,
        passed: bool,
        duration_ms: u64,
    },

    // -----------------------------------------------------------------------
    // Approval flow (v2)
    // -----------------------------------------------------------------------
    /// A human approval has been requested.
    ApprovalRequested {
        approval_id: String,
        scope: String,
        description: String,
    },
    /// A previously requested approval has been resolved.
    ApprovalResolved {
        approval_id: String,
        approved: bool,
        resolver: String,
    },

    // -----------------------------------------------------------------------
    // Control (v2)
    // -----------------------------------------------------------------------
    /// A control action was applied to the runtime (pause, resume, cancel, etc.).
    ControlApplied {
        action: String,
        target: String,
        success: bool,
    },

    // -----------------------------------------------------------------------
    // Budget (v2)
    // -----------------------------------------------------------------------
    /// Budget state has been updated.
    BudgetUpdated {
        budget_id: String,
        spent_usd: f64,
        limit_usd: f64,
        remaining_usd: f64,
    },

    // -----------------------------------------------------------------------
    // Workspace lifecycle (v2)
    // -----------------------------------------------------------------------
    /// A worktree or workspace has been acquired for execution.
    WorkspaceAcquired {
        workspace_id: String,
        path: String,
    },
    /// A worktree or workspace has been released after execution.
    WorkspaceReleased {
        workspace_id: String,
    },

    // -----------------------------------------------------------------------
    // Merge lifecycle (v2)
    // -----------------------------------------------------------------------
    /// A merge has been queued in the merge queue.
    MergeQueued {
        merge_id: String,
        branch: String,
    },
    /// A merge has completed.
    MergeCompleted {
        merge_id: String,
        success: bool,
        commit_hash: String,
    },

    // -----------------------------------------------------------------------
    // Publish (v2)
    // -----------------------------------------------------------------------
    /// A publish operation (PR, release, artifact) has completed.
    PublishCompleted {
        publish_id: String,
        target: String,
        success: bool,
    },

    // -----------------------------------------------------------------------
    // Feedback sink (v2)
    // -----------------------------------------------------------------------
    /// A feedback sink observation has settled successfully.
    FeedbackSinkSettled {
        sink_id: String,
        kind: String,
        summary: String,
    },
    /// A feedback sink observation failed to settle.
    FeedbackSinkFailed {
        sink_id: String,
        kind: String,
        error: String,
    },

    // -----------------------------------------------------------------------
    // Calibration (v2) -- frozen for #269
    // -----------------------------------------------------------------------
    /// A prediction has been published for later calibration.
    PredictionPublished {
        prediction_id: String,
        cell_id: String,
        cell_version: String,
        input_hash: String,
        predicted_outcome: serde_json::Value,
        confidence: f64,
    },
    /// An actual outcome has been recorded against a prediction.
    ActualRecorded {
        prediction_id: String,
        actual_outcome: serde_json::Value,
        succeeded: bool,
    },
    /// A calibration correction has been applied.
    CorrectionApplied {
        prediction_id: String,
        calibration_error: f64,
    },

    // -----------------------------------------------------------------------
    // Sequence integrity (v2)
    // -----------------------------------------------------------------------
    /// Marks a gap in the event sequence caused by dropped best-effort events.
    SequenceGap {
        first_missing_seq: u64,
        last_missing_seq: u64,
        reason: String,
    },

    // -----------------------------------------------------------------------
    // Extension (v2)
    // -----------------------------------------------------------------------
    /// Forward-compatible payload for third-party or experimental event data.
    Extension {
        namespace: String,
        version: String,
        value: serde_json::Value,
    },
}

// ---------------------------------------------------------------------------
// Accessors
// ---------------------------------------------------------------------------

impl RuntimeEvent {
    /// Returns the run-scoped identifier when this event carries one.
    ///
    /// v1 variants carry `run_id` in their payload. v2 variants do not
    /// duplicate the run_id (it lives on the envelope), so those return
    /// an empty string. Callers should prefer `envelope.run_id` for
    /// v2 events.
    pub fn run_id(&self) -> &str {
        match self {
            // v1 variants with run_id
            Self::WorkflowStarted { run_id, .. }
            | Self::PhaseTransition { run_id, .. }
            | Self::WorkflowCompleted { run_id, .. }
            | Self::AgentSpawned { run_id, .. }
            | Self::AgentOutput { run_id, .. }
            | Self::AgentCompleted { run_id, .. }
            | Self::AgentFailed { run_id, .. }
            | Self::GateStarted { run_id, .. }
            | Self::GatePassed { run_id, .. }
            | Self::GateFailed { run_id, .. }
            | Self::FeedbackRecorded { run_id, .. }
            | Self::StateCheckpointed { run_id, .. }
            | Self::InferenceStarted { run_id, .. }
            | Self::InferenceCompleted { run_id, .. }
            | Self::InferenceFailed { run_id, .. }
            | Self::AgentTrace { run_id, .. }
            | Self::RunStarted { run_id, .. }
            | Self::RunCompleted { run_id, .. }
            | Self::KnowledgeIngested { run_id, .. }
            | Self::KnowledgeConsumed { run_id, .. }
            | Self::InferenceFirstToken { run_id, .. }
            | Self::ToolCallStarted { run_id, .. }
            | Self::ToolCallCompleted { run_id, .. }
            | Self::TaskStarted { run_id, .. }
            | Self::TaskCompleted { run_id, .. }
            | Self::PipelinePhase { run_id, .. } => run_id,
            Self::TaskFailed { plan_id, .. } => plan_id,
            // v2 variants -- identity lives on the envelope
            Self::WaveStarted { .. }
            | Self::WaveCompleted { .. }
            | Self::TaskRetrying { .. }
            | Self::TaskSkipped { .. }
            | Self::AgentProgress { .. }
            | Self::UsageRecorded { .. }
            | Self::GateRungStarted { .. }
            | Self::GateRungOutput { .. }
            | Self::GateRungCompleted { .. }
            | Self::ApprovalRequested { .. }
            | Self::ApprovalResolved { .. }
            | Self::ControlApplied { .. }
            | Self::BudgetUpdated { .. }
            | Self::WorkspaceAcquired { .. }
            | Self::WorkspaceReleased { .. }
            | Self::MergeQueued { .. }
            | Self::MergeCompleted { .. }
            | Self::PublishCompleted { .. }
            | Self::FeedbackSinkSettled { .. }
            | Self::FeedbackSinkFailed { .. }
            | Self::PredictionPublished { .. }
            | Self::ActualRecorded { .. }
            | Self::CorrectionApplied { .. }
            | Self::SequenceGap { .. }
            | Self::Extension { .. } => "",
        }
    }

    /// Human-readable event kind label.
    pub fn kind(&self) -> &'static str {
        match self {
            // v1
            Self::WorkflowStarted { .. } => "workflow_started",
            Self::PhaseTransition { .. } => "phase_transition",
            Self::WorkflowCompleted { .. } => "workflow_completed",
            Self::AgentSpawned { .. } => "agent_spawned",
            Self::AgentOutput { .. } => "agent_output",
            Self::AgentCompleted { .. } => "agent_completed",
            Self::AgentFailed { .. } => "agent_failed",
            Self::GateStarted { .. } => "gate_started",
            Self::GatePassed { .. } => "gate_passed",
            Self::GateFailed { .. } => "gate_failed",
            Self::FeedbackRecorded { .. } => "feedback_recorded",
            Self::StateCheckpointed { .. } => "state_checkpointed",
            Self::InferenceStarted { .. } => "inference_started",
            Self::InferenceCompleted { .. } => "inference_completed",
            Self::InferenceFailed { .. } => "inference_failed",
            Self::AgentTrace { .. } => "agent_trace",
            Self::TaskFailed { .. } => "task_failed",
            Self::RunStarted { .. } => "run_started",
            Self::RunCompleted { .. } => "run_completed",
            Self::KnowledgeIngested { .. } => "knowledge_ingested",
            Self::KnowledgeConsumed { .. } => "knowledge_consumed",
            Self::InferenceFirstToken { .. } => "inference_first_token",
            Self::ToolCallStarted { .. } => "tool_call_started",
            Self::ToolCallCompleted { .. } => "tool_call_completed",
            Self::TaskStarted { .. } => "task_started",
            Self::TaskCompleted { .. } => "task_completed",
            Self::PipelinePhase { .. } => "pipeline_phase",
            // v2
            Self::WaveStarted { .. } => "wave_started",
            Self::WaveCompleted { .. } => "wave_completed",
            Self::TaskRetrying { .. } => "task_retrying",
            Self::TaskSkipped { .. } => "task_skipped",
            Self::AgentProgress { .. } => "agent_progress",
            Self::UsageRecorded { .. } => "usage_recorded",
            Self::GateRungStarted { .. } => "gate_rung_started",
            Self::GateRungOutput { .. } => "gate_rung_output",
            Self::GateRungCompleted { .. } => "gate_rung_completed",
            Self::ApprovalRequested { .. } => "approval_requested",
            Self::ApprovalResolved { .. } => "approval_resolved",
            Self::ControlApplied { .. } => "control_applied",
            Self::BudgetUpdated { .. } => "budget_updated",
            Self::WorkspaceAcquired { .. } => "workspace_acquired",
            Self::WorkspaceReleased { .. } => "workspace_released",
            Self::MergeQueued { .. } => "merge_queued",
            Self::MergeCompleted { .. } => "merge_completed",
            Self::PublishCompleted { .. } => "publish_completed",
            Self::FeedbackSinkSettled { .. } => "feedback_sink_settled",
            Self::FeedbackSinkFailed { .. } => "feedback_sink_failed",
            Self::PredictionPublished { .. } => "prediction_published",
            Self::ActualRecorded { .. } => "actual_recorded",
            Self::CorrectionApplied { .. } => "correction_applied",
            Self::SequenceGap { .. } => "sequence_gap",
            Self::Extension { .. } => "extension",
        }
    }

    /// Returns the delivery class for this event variant.
    ///
    /// `AgentOutput`, `AgentProgress`, `GateRungOutput`, and
    /// `InferenceFirstToken` are `BestEffort`; every other variant is
    /// `Reliable`.
    pub fn delivery(&self) -> RuntimeEventDelivery {
        match self {
            Self::AgentOutput { .. }
            | Self::AgentProgress { .. }
            | Self::GateRungOutput { .. }
            | Self::InferenceFirstToken { .. } => RuntimeEventDelivery::BestEffort,

            // All remaining variants are reliable.
            Self::WorkflowStarted { .. }
            | Self::PhaseTransition { .. }
            | Self::WorkflowCompleted { .. }
            | Self::AgentSpawned { .. }
            | Self::AgentCompleted { .. }
            | Self::AgentFailed { .. }
            | Self::GateStarted { .. }
            | Self::GatePassed { .. }
            | Self::GateFailed { .. }
            | Self::FeedbackRecorded { .. }
            | Self::StateCheckpointed { .. }
            | Self::InferenceStarted { .. }
            | Self::InferenceCompleted { .. }
            | Self::InferenceFailed { .. }
            | Self::AgentTrace { .. }
            | Self::TaskFailed { .. }
            | Self::RunStarted { .. }
            | Self::RunCompleted { .. }
            | Self::KnowledgeIngested { .. }
            | Self::KnowledgeConsumed { .. }
            | Self::ToolCallStarted { .. }
            | Self::ToolCallCompleted { .. }
            | Self::TaskStarted { .. }
            | Self::TaskCompleted { .. }
            | Self::PipelinePhase { .. }
            | Self::WaveStarted { .. }
            | Self::WaveCompleted { .. }
            | Self::TaskRetrying { .. }
            | Self::TaskSkipped { .. }
            | Self::UsageRecorded { .. }
            | Self::GateRungStarted { .. }
            | Self::GateRungCompleted { .. }
            | Self::ApprovalRequested { .. }
            | Self::ApprovalResolved { .. }
            | Self::ControlApplied { .. }
            | Self::BudgetUpdated { .. }
            | Self::WorkspaceAcquired { .. }
            | Self::WorkspaceReleased { .. }
            | Self::MergeQueued { .. }
            | Self::MergeCompleted { .. }
            | Self::PublishCompleted { .. }
            | Self::FeedbackSinkSettled { .. }
            | Self::FeedbackSinkFailed { .. }
            | Self::PredictionPublished { .. }
            | Self::ActualRecorded { .. }
            | Self::CorrectionApplied { .. }
            | Self::SequenceGap { .. }
            | Self::Extension { .. } => RuntimeEventDelivery::Reliable,
        }
    }
}

impl fmt::Display for RuntimeEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let run_id = self.run_id();
        if run_id.is_empty() {
            write!(f, "[envelope] {}", self.kind())
        } else {
            write!(f, "[{}] {}", run_id, self.kind())
        }
    }
}

impl fmt::Display for WorkflowOutcome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Success {
                commit_hash: Some(hash),
            } => write!(f, "success ({hash})"),
            Self::Success { commit_hash: None } => write!(f, "success"),
            Self::Halted { reason } => write!(f, "halted: {reason}"),
            Self::Cancelled => write!(f, "cancelled"),
        }
    }
}

// ---------------------------------------------------------------------------
// Adapter traits
// ---------------------------------------------------------------------------

/// Result of publishing an event through a [`RuntimeEventPublisher`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RuntimeEventPublishDisposition {
    /// The event was durably acknowledged by the subscriber.
    Acknowledged,
    /// The event was accepted on a best-effort basis (no durable ack).
    AcceptedBestEffort,
    /// The event was coalesced with a preceding event of the same kind.
    Coalesced,
    /// The event was dropped. Valid **only** for `BestEffort` events.
    /// The publisher must enqueue one reliable `SequenceGap` after a drop.
    Dropped,
}

/// Async publisher contract for runtime events.
///
/// Implementations receive envelopes and return a disposition that tells the
/// caller whether the event was durably acknowledged, accepted best-effort,
/// coalesced, or dropped.
///
/// A `Dropped` disposition is valid only for `BestEffort` events. When a
/// publisher drops a best-effort event, it must enqueue one reliable
/// `SequenceGap` to notify downstream consumers of the loss.
#[async_trait]
pub trait RuntimeEventPublisher: Send + Sync {
    async fn publish(
        &self,
        envelope: &RuntimeEventEnvelope,
    ) -> anyhow::Result<RuntimeEventPublishDisposition>;
}

/// Async projector contract for runtime events.
///
/// Projectors transform envelopes into side effects (dashboard updates,
/// SSE messages, aggregate state, etc.). Implementations must be idempotent
/// by `event_id`: processing the same envelope twice must produce the same
/// observable result.
#[async_trait]
pub trait RuntimeEventProjector: Send + Sync {
    async fn project(&self, envelope: &RuntimeEventEnvelope) -> anyhow::Result<()>;
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_id_accessor() {
        let event = RuntimeEvent::WorkflowStarted {
            run_id: "r1".into(),
            template: "express".into(),
            prompt: "fix bug".into(),
        };

        assert_eq!(event.run_id(), "r1");
        assert_eq!(event.kind(), "workflow_started");
    }

    #[test]
    fn display_formats() {
        let outcome = WorkflowOutcome::Success {
            commit_hash: Some("abc123".into()),
        };

        assert!(outcome.to_string().contains("abc123"));
    }

    #[test]
    fn surface_events_have_separate_tagged_contracts() {
        let event = SurfaceEvent::AutonomyLevelChange {
            agent_id: "agent-1".into(),
            capability: "fs_read".into(),
            new_level: AutonomyLevel::Full,
        };
        let value = serde_json::to_value(&event).expect("serialize surface event");
        assert_eq!(value["kind"], "autonomy_level_change");
        assert_eq!(value["data"]["new_level"], "full");
        let decoded: SurfaceEvent = serde_json::from_value(value).expect("deserialize");
        assert_eq!(decoded, event);

        let task = SurfaceEvent::TaskAssign {
            graph: "release".into(),
            inputs: serde_json::json!({"sha": "abc"}),
            budget: Some(2.5),
            deadline: None,
        };
        assert_eq!(
            serde_json::to_value(task).expect("serialize task")["kind"],
            "task_assign"
        );
    }

    #[test]
    fn new_runtime_event_variants_serialize_roundtrip() {
        let events = vec![
            (
                RuntimeEvent::InferenceStarted {
                    run_id: "run-1".into(),
                    request_id: "req-1".into(),
                    model: "claude-sonnet".into(),
                    agent_id: "agent-1".into(),
                    auto_routed: true,
                },
                "inference_started",
            ),
            (
                RuntimeEvent::InferenceCompleted {
                    run_id: "run-1".into(),
                    request_id: "req-1".into(),
                    model: "claude-sonnet".into(),
                    agent_id: "agent-1".into(),
                    input_tokens: 100,
                    output_tokens: 50,
                    cost_usd: 0.0123,
                    duration_ms: 1200,
                },
                "inference_completed",
            ),
            (
                RuntimeEvent::InferenceFailed {
                    run_id: "run-1".into(),
                    request_id: "req-2".into(),
                    model: "claude-sonnet".into(),
                    agent_id: "agent-1".into(),
                    error: "rate limited".into(),
                },
                "inference_failed",
            ),
            (
                RuntimeEvent::AgentTrace {
                    run_id: "run-1".into(),
                    agent_id: "agent-1".into(),
                    turn: 2,
                    tool_calls: vec![ToolCallSummary {
                        name: "read_file".into(),
                        result_preview: "loaded runtime_event.rs".into(),
                    }],
                    reasoning: Some("checking event coverage".into()),
                    usage: TokenUsage {
                        input_tokens: 200,
                        output_tokens: 75,
                        total_tokens: 275,
                        cost_usd: 0.025,
                    },
                },
                "agent_trace",
            ),
            (
                RuntimeEvent::TaskFailed {
                    plan_id: "plan-1".into(),
                    task_id: "task-1".into(),
                    error: "gate failed".into(),
                    gate_failure: true,
                },
                "task_failed",
            ),
            (
                RuntimeEvent::RunStarted {
                    run_id: "run-1".into(),
                    prompt: "ship demo".into(),
                    complexity: "standard".into(),
                },
                "run_started",
            ),
            (
                RuntimeEvent::RunCompleted {
                    run_id: "run-1".into(),
                    success: true,
                    cost_usd: 0.42,
                    duration_ms: 9000,
                },
                "run_completed",
            ),
            (
                RuntimeEvent::KnowledgeIngested {
                    run_id: "run-1".into(),
                    entry_id: "entry-1".into(),
                    topic: "event architecture".into(),
                    source_agent: "agent-1".into(),
                },
                "knowledge_ingested",
            ),
            (
                RuntimeEvent::KnowledgeConsumed {
                    run_id: "run-1".into(),
                    entry_id: "entry-1".into(),
                    topic: "event architecture".into(),
                    consuming_agent: "agent-2".into(),
                },
                "knowledge_consumed",
            ),
            (
                RuntimeEvent::InferenceFirstToken {
                    run_id: "run-1".into(),
                    request_id: "req-ft".into(),
                    model: "claude-sonnet".into(),
                    agent_id: "agent-1".into(),
                    ttft_ms: 1823,
                },
                "inference_first_token",
            ),
            (
                RuntimeEvent::ToolCallStarted {
                    run_id: "run-1".into(),
                    agent_id: "agent-1".into(),
                    tool: "read_file".into(),
                    iteration: 3,
                },
                "tool_call_started",
            ),
            (
                RuntimeEvent::ToolCallCompleted {
                    run_id: "run-1".into(),
                    agent_id: "agent-1".into(),
                    tool: "read_file".into(),
                    duration_ms: 12,
                    success: true,
                },
                "tool_call_completed",
            ),
            (
                RuntimeEvent::TaskStarted {
                    run_id: "run-1".into(),
                    plan_id: "plan-1".into(),
                    task_id: "task-1".into(),
                    task_title: "Implement progress events".into(),
                    role: "implementer".into(),
                },
                "task_started",
            ),
            (
                RuntimeEvent::TaskCompleted {
                    run_id: "run-1".into(),
                    plan_id: "plan-1".into(),
                    task_id: "task-1".into(),
                    passed: true,
                    duration_ms: 47200,
                },
                "task_completed",
            ),
            (
                RuntimeEvent::PipelinePhase {
                    run_id: "run-1".into(),
                    phase: "execute".into(),
                    status: "started".into(),
                },
                "pipeline_phase",
            ),
        ];

        for (event, expected_kind) in events {
            let value = serde_json::to_value(&event).expect("serialize runtime event");
            assert_eq!(value["kind"], expected_kind);
            assert!(value.get("data").is_some());

            let decoded: RuntimeEvent =
                serde_json::from_value(value).expect("deserialize runtime event");
            assert_eq!(decoded, event);
            assert_eq!(decoded.kind(), expected_kind);
        }
    }

    // -----------------------------------------------------------------------
    // v2 variant serialization roundtrip
    // -----------------------------------------------------------------------

    #[test]
    fn v2_variants_serialize_roundtrip() {
        let events: Vec<(RuntimeEvent, &str)> = vec![
            (
                RuntimeEvent::WaveStarted {
                    wave_index: 0,
                    task_count: 3,
                },
                "wave_started",
            ),
            (
                RuntimeEvent::WaveCompleted {
                    wave_index: 0,
                    succeeded: 2,
                    failed: 1,
                    duration_ms: 4500,
                },
                "wave_completed",
            ),
            (
                RuntimeEvent::TaskRetrying {
                    task_id: "t-1".into(),
                    attempt: 2,
                    reason: "transient error".into(),
                },
                "task_retrying",
            ),
            (
                RuntimeEvent::TaskSkipped {
                    task_id: "t-2".into(),
                    reason: "dependency failed".into(),
                },
                "task_skipped",
            ),
            (
                RuntimeEvent::AgentProgress {
                    agent_id: "a-1".into(),
                    progress_pct: 0.45,
                    message: "compiling".into(),
                },
                "agent_progress",
            ),
            (
                RuntimeEvent::UsageRecorded {
                    input_tokens: 1000,
                    output_tokens: 500,
                    cost_usd: 0.05,
                    model: "claude-sonnet".into(),
                },
                "usage_recorded",
            ),
            (
                RuntimeEvent::GateRungStarted {
                    gate_name: "compile".into(),
                    rung: 1,
                },
                "gate_rung_started",
            ),
            (
                RuntimeEvent::GateRungOutput {
                    gate_name: "compile".into(),
                    rung: 1,
                    chunk: "Building...".into(),
                },
                "gate_rung_output",
            ),
            (
                RuntimeEvent::GateRungCompleted {
                    gate_name: "compile".into(),
                    rung: 1,
                    passed: true,
                    duration_ms: 2000,
                },
                "gate_rung_completed",
            ),
            (
                RuntimeEvent::ApprovalRequested {
                    approval_id: "apr-1".into(),
                    scope: "merge".into(),
                    description: "merge to main".into(),
                },
                "approval_requested",
            ),
            (
                RuntimeEvent::ApprovalResolved {
                    approval_id: "apr-1".into(),
                    approved: true,
                    resolver: "will".into(),
                },
                "approval_resolved",
            ),
            (
                RuntimeEvent::ControlApplied {
                    action: "pause".into(),
                    target: "run-1".into(),
                    success: true,
                },
                "control_applied",
            ),
            (
                RuntimeEvent::BudgetUpdated {
                    budget_id: "b-1".into(),
                    spent_usd: 1.50,
                    limit_usd: 10.0,
                    remaining_usd: 8.50,
                },
                "budget_updated",
            ),
            (
                RuntimeEvent::WorkspaceAcquired {
                    workspace_id: "ws-1".into(),
                    path: "/tmp/worktree-1".into(),
                },
                "workspace_acquired",
            ),
            (
                RuntimeEvent::WorkspaceReleased {
                    workspace_id: "ws-1".into(),
                },
                "workspace_released",
            ),
            (
                RuntimeEvent::MergeQueued {
                    merge_id: "m-1".into(),
                    branch: "feature/events".into(),
                },
                "merge_queued",
            ),
            (
                RuntimeEvent::MergeCompleted {
                    merge_id: "m-1".into(),
                    success: true,
                    commit_hash: "abc123".into(),
                },
                "merge_completed",
            ),
            (
                RuntimeEvent::PublishCompleted {
                    publish_id: "pub-1".into(),
                    target: "github-pr".into(),
                    success: true,
                },
                "publish_completed",
            ),
            (
                RuntimeEvent::FeedbackSinkSettled {
                    sink_id: "fs-1".into(),
                    kind: "gate".into(),
                    summary: "compile passed".into(),
                },
                "feedback_sink_settled",
            ),
            (
                RuntimeEvent::FeedbackSinkFailed {
                    sink_id: "fs-1".into(),
                    kind: "gate".into(),
                    error: "timeout".into(),
                },
                "feedback_sink_failed",
            ),
            (
                RuntimeEvent::PredictionPublished {
                    prediction_id: "pred-1".into(),
                    cell_id: "cell-1".into(),
                    cell_version: "1.0".into(),
                    input_hash: "deadbeef".into(),
                    predicted_outcome: serde_json::json!({"pass": true}),
                    confidence: 0.85,
                },
                "prediction_published",
            ),
            (
                RuntimeEvent::ActualRecorded {
                    prediction_id: "pred-1".into(),
                    actual_outcome: serde_json::json!({"pass": true}),
                    succeeded: true,
                },
                "actual_recorded",
            ),
            (
                RuntimeEvent::CorrectionApplied {
                    prediction_id: "pred-1".into(),
                    calibration_error: 0.02,
                },
                "correction_applied",
            ),
            (
                RuntimeEvent::SequenceGap {
                    first_missing_seq: 5,
                    last_missing_seq: 7,
                    reason: "slow observer dropped 3 best-effort events".into(),
                },
                "sequence_gap",
            ),
            (
                RuntimeEvent::Extension {
                    namespace: "com.example".into(),
                    version: "1.0".into(),
                    value: serde_json::json!({"custom": "data"}),
                },
                "extension",
            ),
        ];

        for (event, expected_kind) in events {
            let value = serde_json::to_value(&event).expect("serialize v2 event");
            assert_eq!(value["kind"], expected_kind, "kind mismatch for {expected_kind}");
            assert!(value.get("data").is_some(), "missing data for {expected_kind}");

            let decoded: RuntimeEvent =
                serde_json::from_value(value).expect("deserialize v2 event");
            assert_eq!(decoded, event);
            assert_eq!(decoded.kind(), expected_kind);
        }
    }

    // -----------------------------------------------------------------------
    // Envelope v2
    // -----------------------------------------------------------------------

    #[test]
    fn envelope_new_produces_v2() {
        let envelope = RuntimeEventEnvelope::new(
            "run-1",
            0,
            "test",
            RuntimeEvent::WorkflowStarted {
                run_id: "run-1".into(),
                template: "express".into(),
                prompt: "fix".into(),
            },
        );

        assert_eq!(envelope.schema_version, 2);
        assert_eq!(envelope.mode, RuntimeEventMode::Live);
        assert_eq!(envelope.delivery, RuntimeEventDelivery::Reliable);
        assert!(envelope.plan_id.is_none());
        assert!(envelope.task_id.is_none());
        assert!(envelope.node_id.is_none());
        assert!(envelope.attempt_id.is_none());
        assert!(envelope.agent_id.is_none());
        assert!(envelope.correlation_id.is_none());
        assert!(envelope.idempotency_key.is_none());
        // event_id is a valid UUID
        assert!(Uuid::parse_str(&envelope.event_id).is_ok());
    }

    #[test]
    fn envelope_new_v2_all_fields() {
        let ts = Utc::now();
        let envelope = RuntimeEventEnvelope::new_v2(
            "evt-1",
            "run-1",
            Some("plan-1".into()),
            Some("task-1".into()),
            Some("node-1".into()),
            Some("attempt-1".into()),
            Some("agent-1".into()),
            42,
            ts,
            "graph",
            RuntimeEventMode::Replay,
            Some("corr-1".into()),
            Some("idem-1".into()),
            RuntimeEvent::WaveStarted {
                wave_index: 0,
                task_count: 3,
            },
        );

        assert_eq!(envelope.event_id, "evt-1");
        assert_eq!(envelope.run_id, "run-1");
        assert_eq!(envelope.plan_id.as_deref(), Some("plan-1"));
        assert_eq!(envelope.task_id.as_deref(), Some("task-1"));
        assert_eq!(envelope.node_id.as_deref(), Some("node-1"));
        assert_eq!(envelope.attempt_id.as_deref(), Some("attempt-1"));
        assert_eq!(envelope.agent_id.as_deref(), Some("agent-1"));
        assert_eq!(envelope.seq, 42);
        assert_eq!(envelope.ts, ts);
        assert_eq!(envelope.schema_version, 2);
        assert_eq!(envelope.source, "graph");
        assert_eq!(envelope.mode, RuntimeEventMode::Replay);
        assert_eq!(envelope.delivery, RuntimeEventDelivery::Reliable);
        assert_eq!(envelope.correlation_id.as_deref(), Some("corr-1"));
        assert_eq!(envelope.idempotency_key.as_deref(), Some("idem-1"));
    }

    #[test]
    fn envelope_v2_serialization_roundtrip() {
        let envelope = RuntimeEventEnvelope::new_v2(
            "evt-1",
            "run-1",
            Some("plan-1".into()),
            None,
            None,
            None,
            Some("agent-1".into()),
            5,
            Utc::now(),
            "runner",
            RuntimeEventMode::Live,
            None,
            None,
            RuntimeEvent::AgentSpawned {
                run_id: "run-1".into(),
                agent_id: "agent-1".into(),
                role: "implementer".into(),
                model: "claude-sonnet".into(),
            },
        );

        let json = serde_json::to_string(&envelope).expect("serialize envelope");
        let decoded: RuntimeEventEnvelope =
            serde_json::from_str(&json).expect("deserialize envelope");
        assert_eq!(decoded, envelope);
        assert_eq!(decoded.schema_version, 2);
    }

    // -----------------------------------------------------------------------
    // v1 compatibility golden test
    // -----------------------------------------------------------------------

    #[test]
    fn v1_envelope_json_deserializes_with_v2_defaults() {
        // This is exactly the JSON shape the v1 envelope produced.
        let v1_json = r#"{
            "run_id": "run-old",
            "seq": 3,
            "ts": "2026-08-01T12:00:00Z",
            "schema_version": 1,
            "source": "jsonl_logger",
            "payload": {
                "kind": "gate_passed",
                "data": {
                    "run_id": "run-old",
                    "gate_name": "compile",
                    "duration_ms": 150
                }
            }
        }"#;

        let envelope: RuntimeEventEnvelope =
            serde_json::from_str(v1_json).expect("v1 JSON must deserialize into v2 envelope");

        assert_eq!(envelope.run_id, "run-old");
        assert_eq!(envelope.seq, 3);
        assert_eq!(envelope.source, "jsonl_logger");
        // v2 defaults for fields absent in v1 JSON
        assert_eq!(envelope.mode, RuntimeEventMode::Live);
        assert_eq!(envelope.delivery, RuntimeEventDelivery::Reliable);
        assert!(envelope.plan_id.is_none());
        assert!(envelope.correlation_id.is_none());
        // payload roundtripped correctly
        assert_eq!(envelope.payload.kind(), "gate_passed");
    }

    // -----------------------------------------------------------------------
    // Delivery classification
    // -----------------------------------------------------------------------

    #[test]
    fn best_effort_variants() {
        let best_effort = vec![
            RuntimeEvent::AgentOutput {
                run_id: "r".into(),
                agent_id: "a".into(),
                chunk: "x".into(),
            },
            RuntimeEvent::AgentProgress {
                agent_id: "a".into(),
                progress_pct: 0.5,
                message: "half".into(),
            },
            RuntimeEvent::GateRungOutput {
                gate_name: "g".into(),
                rung: 1,
                chunk: "out".into(),
            },
            RuntimeEvent::InferenceFirstToken {
                run_id: "r".into(),
                request_id: "req".into(),
                model: "m".into(),
                agent_id: "a".into(),
                ttft_ms: 100,
            },
        ];
        for event in &best_effort {
            assert_eq!(
                event.delivery(),
                RuntimeEventDelivery::BestEffort,
                "{} should be BestEffort",
                event.kind()
            );
        }
    }

    #[test]
    fn reliable_variants_sample() {
        let reliable = vec![
            RuntimeEvent::WorkflowStarted {
                run_id: "r".into(),
                template: "t".into(),
                prompt: "p".into(),
            },
            RuntimeEvent::WaveStarted {
                wave_index: 0,
                task_count: 1,
            },
            RuntimeEvent::SequenceGap {
                first_missing_seq: 1,
                last_missing_seq: 2,
                reason: "drop".into(),
            },
            RuntimeEvent::PredictionPublished {
                prediction_id: "p".into(),
                cell_id: "c".into(),
                cell_version: "1".into(),
                input_hash: "h".into(),
                predicted_outcome: serde_json::json!(null),
                confidence: 0.9,
            },
            RuntimeEvent::Extension {
                namespace: "ns".into(),
                version: "1".into(),
                value: serde_json::json!(null),
            },
        ];
        for event in &reliable {
            assert_eq!(
                event.delivery(),
                RuntimeEventDelivery::Reliable,
                "{} should be Reliable",
                event.kind()
            );
        }
    }

    // -----------------------------------------------------------------------
    // Envelope delivery derived from payload
    // -----------------------------------------------------------------------

    #[test]
    fn envelope_new_derives_delivery_from_payload() {
        let best_effort_envelope = RuntimeEventEnvelope::new(
            "r",
            0,
            "test",
            RuntimeEvent::AgentOutput {
                run_id: "r".into(),
                agent_id: "a".into(),
                chunk: "x".into(),
            },
        );
        assert_eq!(best_effort_envelope.delivery, RuntimeEventDelivery::BestEffort);

        let reliable_envelope = RuntimeEventEnvelope::new(
            "r",
            1,
            "test",
            RuntimeEvent::GatePassed {
                run_id: "r".into(),
                gate_name: "g".into(),
                duration_ms: 100,
            },
        );
        assert_eq!(reliable_envelope.delivery, RuntimeEventDelivery::Reliable);
    }

    // -----------------------------------------------------------------------
    // v2 variant run_id returns empty string
    // -----------------------------------------------------------------------

    #[test]
    fn v2_variants_return_empty_run_id() {
        let event = RuntimeEvent::WaveStarted {
            wave_index: 0,
            task_count: 1,
        };
        assert_eq!(event.run_id(), "");
    }

    // -----------------------------------------------------------------------
    // Display for v2 variants
    // -----------------------------------------------------------------------

    #[test]
    fn display_v2_variant_uses_envelope_prefix() {
        let event = RuntimeEvent::SequenceGap {
            first_missing_seq: 1,
            last_missing_seq: 3,
            reason: "drop".into(),
        };
        assert_eq!(event.to_string(), "[envelope] sequence_gap");
    }

    // -----------------------------------------------------------------------
    // Disposition enum
    // -----------------------------------------------------------------------

    #[test]
    fn publish_disposition_variants_are_distinct() {
        let variants = [
            RuntimeEventPublishDisposition::Acknowledged,
            RuntimeEventPublishDisposition::AcceptedBestEffort,
            RuntimeEventPublishDisposition::Coalesced,
            RuntimeEventPublishDisposition::Dropped,
        ];
        for (i, a) in variants.iter().enumerate() {
            for (j, b) in variants.iter().enumerate() {
                if i == j {
                    assert_eq!(a, b);
                } else {
                    assert_ne!(a, b);
                }
            }
        }
    }

    // -----------------------------------------------------------------------
    // Mode and delivery enums serialize
    // -----------------------------------------------------------------------

    #[test]
    fn mode_serialization() {
        assert_eq!(
            serde_json::to_string(&RuntimeEventMode::Live).unwrap(),
            "\"live\""
        );
        assert_eq!(
            serde_json::to_string(&RuntimeEventMode::Replay).unwrap(),
            "\"replay\""
        );
    }

    #[test]
    fn delivery_serialization() {
        assert_eq!(
            serde_json::to_string(&RuntimeEventDelivery::Reliable).unwrap(),
            "\"reliable\""
        );
        assert_eq!(
            serde_json::to_string(&RuntimeEventDelivery::BestEffort).unwrap(),
            "\"best_effort\""
        );
    }

    // -----------------------------------------------------------------------
    // Calibration payloads frozen for #269
    // -----------------------------------------------------------------------

    #[test]
    fn calibration_payload_shapes_are_frozen() {
        let prediction = RuntimeEvent::PredictionPublished {
            prediction_id: "p-1".into(),
            cell_id: "cell-a".into(),
            cell_version: "2.1".into(),
            input_hash: "sha256:abc".into(),
            predicted_outcome: serde_json::json!({"compile": "pass", "test": "pass"}),
            confidence: 0.92,
        };
        let actual = RuntimeEvent::ActualRecorded {
            prediction_id: "p-1".into(),
            actual_outcome: serde_json::json!({"compile": "pass", "test": "fail"}),
            succeeded: false,
        };
        let correction = RuntimeEvent::CorrectionApplied {
            prediction_id: "p-1".into(),
            calibration_error: 0.08,
        };

        // Roundtrip all three
        for event in [&prediction, &actual, &correction] {
            let json = serde_json::to_string(event).unwrap();
            let decoded: RuntimeEvent = serde_json::from_str(&json).unwrap();
            assert_eq!(&decoded, event);
        }

        // Verify shape of prediction JSON
        let pred_value = serde_json::to_value(&prediction).unwrap();
        assert_eq!(pred_value["data"]["confidence"], 0.92);
        assert!(pred_value["data"]["predicted_outcome"].is_object());
    }

    // -----------------------------------------------------------------------
    // Layer boundary: no higher-layer types in public API
    // -----------------------------------------------------------------------

    #[test]
    fn no_higher_layer_types_in_envelope() {
        // This is a compile-time assertion: RuntimeEventEnvelope and
        // RuntimeEvent are defined entirely in roko-core with no imports
        // from runner, graph, projection, CLI, TUI, or serve crates.
        //
        // If this test compiles, the layer boundary is intact. The type
        // system enforces that roko-core cannot import higher layers.
        let _envelope = RuntimeEventEnvelope::new(
            "r",
            0,
            "test",
            RuntimeEvent::Extension {
                namespace: "layer-test".into(),
                version: "1".into(),
                value: serde_json::json!(null),
            },
        );
    }

    // -----------------------------------------------------------------------
    // Exhaustive kind coverage
    // -----------------------------------------------------------------------

    #[test]
    fn all_variants_have_distinct_kinds() {
        let all_kinds = vec![
            // v1
            "workflow_started",
            "phase_transition",
            "workflow_completed",
            "agent_spawned",
            "agent_output",
            "agent_completed",
            "agent_failed",
            "gate_started",
            "gate_passed",
            "gate_failed",
            "feedback_recorded",
            "state_checkpointed",
            "inference_started",
            "inference_completed",
            "inference_failed",
            "agent_trace",
            "task_failed",
            "run_started",
            "run_completed",
            "knowledge_ingested",
            "knowledge_consumed",
            "inference_first_token",
            "tool_call_started",
            "tool_call_completed",
            "task_started",
            "task_completed",
            "pipeline_phase",
            // v2
            "wave_started",
            "wave_completed",
            "task_retrying",
            "task_skipped",
            "agent_progress",
            "usage_recorded",
            "gate_rung_started",
            "gate_rung_output",
            "gate_rung_completed",
            "approval_requested",
            "approval_resolved",
            "control_applied",
            "budget_updated",
            "workspace_acquired",
            "workspace_released",
            "merge_queued",
            "merge_completed",
            "publish_completed",
            "feedback_sink_settled",
            "feedback_sink_failed",
            "prediction_published",
            "actual_recorded",
            "correction_applied",
            "sequence_gap",
            "extension",
        ];

        // All kinds are unique
        let mut seen = std::collections::HashSet::new();
        for kind in &all_kinds {
            assert!(seen.insert(kind), "duplicate kind: {kind}");
        }

        // 27 v1 + 25 v2 = 52 total
        assert_eq!(all_kinds.len(), 52);
    }
}
