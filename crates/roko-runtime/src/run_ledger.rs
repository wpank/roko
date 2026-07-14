//! Typed workflow run ledger and compatibility report adapter.
//!
//! This module is intentionally not wired into `WorkflowEngine` yet. It gives
//! future runtime packets a typed source of truth that can produce the current
//! `WorkflowRunReport` shape without replaying the event bus.

use std::path::PathBuf;

use roko_core::RuntimeEvent;
use roko_core::foundation::TokenUsage;
use roko_core::runtime_event::RuntimeEventEnvelope;
use serde::{Deserialize, Serialize};

use crate::pipeline_state::{CommitOutcome, Phase, WorkflowConfig};
use crate::workflow_engine::{GateOutcome, WorkflowRunReport};

/// Typed record for a single workflow run.
#[derive(Debug, Clone)]
pub struct RunLedger {
    /// Workflow run id.
    pub run_id: String,
    /// Original user prompt.
    pub prompt: String,
    /// Workflow configuration used by the run.
    pub workflow: WorkflowConfig,
    /// Run start timestamp in milliseconds since unix epoch.
    pub started_at_ms: u64,
    /// Ordered phase transition history.
    pub phase_history: Vec<PhaseTransitionRecord>,
    /// Agent outcomes recorded directly from effect results.
    pub agent_outcomes: Vec<AgentOutcome>,
    /// Gate outcomes recorded directly from effect results.
    pub gate_runs: Vec<GateRunOutcome>,
    /// Artifact outcomes recorded directly from effect results.
    pub artifacts: Vec<ArtifactOutcome>,
    /// Commit outcome, when commit was attempted.
    pub commit: Option<CommitOutcome>,
    /// Idempotent terminal task attempt records.
    pub task_terminals: Vec<TaskTerminalOutcome>,
    /// Idempotent typed timeout terminals keyed by exact run/task attempt.
    pub timeout_terminals: TimeoutTerminalReplay,
    /// Cancellation outcome, when cancellation was requested.
    pub cancellation: Option<CancellationOutcome>,
    /// Event persistence health observed during the run.
    pub event_persistence: EventPersistenceHealth,
    /// Last checkpoint path, when one was written.
    pub checkpoint_path: Option<PathBuf>,
}

impl RunLedger {
    /// Create an empty ledger for a run.
    pub fn new(
        run_id: impl Into<String>,
        prompt: impl Into<String>,
        workflow: WorkflowConfig,
        started_at_ms: u64,
    ) -> Self {
        Self {
            run_id: run_id.into(),
            prompt: prompt.into(),
            workflow,
            started_at_ms,
            phase_history: Vec::new(),
            agent_outcomes: Vec::new(),
            gate_runs: Vec::new(),
            artifacts: Vec::new(),
            commit: None,
            task_terminals: Vec::new(),
            timeout_terminals: TimeoutTerminalReplay::default(),
            cancellation: None,
            event_persistence: EventPersistenceHealth::default(),
            checkpoint_path: None,
        }
    }

    /// Build the legacy workflow report shape from typed ledger fields.
    ///
    /// `success`, `duration_secs`, and `events` are passed in for compatibility
    /// with the current `WorkflowEngine` report contract. This method does not
    /// inspect the event bus.
    pub fn to_report_compat(
        &self,
        success: bool,
        duration_secs: f64,
        events: Vec<RuntimeEventEnvelope>,
    ) -> WorkflowRunReport {
        let agent_fields = AgentReportFields::from_outcomes(&self.agent_outcomes);
        let (model, provider) = resolve_report_model(&agent_fields, &events);

        let gates = if self.gate_runs.is_empty() {
            gate_outcomes_from_events(&events)
        } else {
            self.gate_runs
                .iter()
                .map(GateRunOutcome::to_report_gate)
                .collect()
        };

        WorkflowRunReport {
            run_id: self.run_id.clone(),
            success,
            model,
            provider,
            prompt_summary: summarize_text(&self.prompt, 120),
            output: agent_fields
                .output
                .clone()
                .unwrap_or_else(|| default_report_output(success)),
            agent_turns: agent_fields.agent_turns,
            token_usage: agent_fields.token_usage,
            cost: agent_fields.cost(),
            duration_secs,
            gates,
            events,
            checkpoint_path: self
                .checkpoint_path
                .as_ref()
                .map(|path| path.display().to_string()),
        }
    }

    /// Record a workflow phase transition.
    pub fn record_phase_transition(&mut self, from: Phase, to: Phase, at_ms: u64) {
        self.phase_history
            .push(PhaseTransitionRecord { from, to, at_ms });
    }

    /// Record an agent completion outcome.
    #[allow(clippy::too_many_arguments)]
    pub fn record_agent_completed(
        &mut self,
        role: impl Into<String>,
        output: impl Into<String>,
        files_changed: u32,
        requested_model: impl Into<String>,
        final_model: impl Into<String>,
        provider_id: Option<String>,
        usage: TokenUsage,
    ) {
        self.agent_outcomes.push(AgentOutcome::Completed {
            role: role.into(),
            output: output.into(),
            files_changed,
            requested_model: requested_model.into(),
            routed_model: None,
            final_model: final_model.into(),
            provider_id: provider_id.unwrap_or_default(),
            usage,
            request_id: None,
        });
    }

    /// Record an agent failure outcome.
    pub fn record_agent_failed(
        &mut self,
        role: impl Into<String>,
        kind: EffectErrorKind,
        message: impl Into<String>,
    ) {
        self.agent_outcomes.push(AgentOutcome::Failed {
            role: role.into(),
            kind,
            message: message.into(),
        });
    }

    /// Record a gate outcome available at workflow-owner level.
    pub fn record_gate_run(
        &mut self,
        name: impl Into<String>,
        passed: bool,
        output: Option<String>,
        duration_ms: u64,
    ) {
        self.gate_runs.push(GateRunOutcome {
            name: name.into(),
            passed,
            output,
            duration_ms,
        });
    }

    /// Record a commit outcome.
    pub fn record_commit(&mut self, outcome: CommitOutcome) {
        self.commit = Some(outcome);
    }

    /// Record a terminal task attempt exactly once by plan/task/attempt.
    ///
    /// Returns true only when the terminal record was newly inserted.
    pub fn record_task_terminal(&mut self, outcome: TaskTerminalOutcome) -> bool {
        if self.task_terminals.iter().any(|existing| {
            existing.plan_id == outcome.plan_id
                && existing.task_id == outcome.task_id
                && existing.attempt == outcome.attempt
        }) {
            return false;
        }
        self.commit = Some(outcome.commit_outcome.clone());
        self.task_terminals.push(outcome);
        true
    }

    /// Record one exact timeout terminal, rejecting a conflicting duplicate.
    pub fn record_timeout_terminal(
        &mut self,
        entry: TimeoutLedgerEntry,
    ) -> Result<bool, TimeoutLedgerConflict> {
        if entry.run_id() != self.run_id {
            return Err(TimeoutLedgerConflict::RunMismatch {
                ledger_run_id: self.run_id.clone(),
                entry_run_id: entry.run_id().to_string(),
            });
        }
        self.timeout_terminals.record(entry)
    }

    /// Record a cancellation request observed by the workflow owner.
    pub fn record_cancellation_requested(&mut self, phase: Phase, requested_at_ms: u64) {
        self.cancellation = Some(CancellationOutcome::Requested {
            phase,
            requested_at_ms,
        });
    }
}

fn gate_outcomes_from_events(events: &[RuntimeEventEnvelope]) -> Vec<GateOutcome> {
    events
        .iter()
        .filter_map(|envelope| match &envelope.payload {
            RuntimeEvent::GatePassed {
                gate_name,
                duration_ms,
                ..
            } => Some(GateOutcome {
                name: gate_name.clone(),
                passed: true,
                output: None,
                duration_ms: *duration_ms,
            }),
            RuntimeEvent::GateFailed {
                gate_name,
                output,
                duration_ms,
                ..
            } => Some(GateOutcome {
                name: gate_name.clone(),
                passed: false,
                output: Some(output.clone()),
                duration_ms: *duration_ms,
            }),
            _ => None,
        })
        .collect()
}

#[derive(Debug, Default)]
struct AgentReportFields {
    agent_turns: u32,
    token_usage: u64,
    cost_total: f64,
    saw_cost: bool,
    output: Option<String>,
    selected_agent: Option<(String, String)>,
    fallback_requested_model: Option<String>,
}

impl AgentReportFields {
    fn from_outcomes(outcomes: &[AgentOutcome]) -> Self {
        let mut fields = Self::default();
        for outcome in outcomes {
            fields.agent_turns = fields.agent_turns.saturating_add(1);
            match outcome {
                AgentOutcome::Completed {
                    role,
                    output,
                    final_model,
                    provider_id,
                    requested_model,
                    usage,
                    ..
                } => fields.record_completed(
                    role,
                    output,
                    final_model,
                    provider_id,
                    requested_model,
                    usage,
                ),
                AgentOutcome::Failed { message, .. } => fields.record_failed(message),
            }
        }
        fields
    }

    fn record_completed(
        &mut self,
        role: &str,
        output: &str,
        final_model: &str,
        provider_id: &str,
        requested_model: &str,
        usage: &TokenUsage,
    ) {
        self.output = Some(output.to_string());
        self.token_usage = self.token_usage.saturating_add(usage.total_tokens);
        self.cost_total += usage.cost_usd;
        self.saw_cost = true;
        if self.selected_agent.is_none() || role == "implementer" {
            self.selected_agent = Some((final_model.to_string(), provider_id.to_string()));
        }
        if self.fallback_requested_model.is_none() {
            if let Some(model) = non_empty(requested_model) {
                self.fallback_requested_model = Some(model.to_string());
            }
        }
    }

    fn record_failed(&mut self, message: &str) {
        if self.output.is_none() {
            self.output = Some(message.to_string());
        }
    }

    fn cost(&self) -> Option<f64> {
        self.saw_cost.then_some(self.cost_total)
    }
}

fn resolve_report_model(
    fields: &AgentReportFields,
    events: &[RuntimeEventEnvelope],
) -> (String, Option<String>) {
    if let Some((model, provider)) = &fields.selected_agent {
        return (model.clone(), non_empty(provider).map(ToOwned::to_owned));
    }

    let model = fields
        .fallback_requested_model
        .clone()
        .or_else(|| spawned_model_from_events(events))
        .unwrap_or_else(|| "unconfigured".to_string());
    (model, None)
}

fn spawned_model_from_events(events: &[RuntimeEventEnvelope]) -> Option<String> {
    let mut implementer: Option<String> = None;
    let mut first: Option<String> = None;
    for envelope in events {
        if let RuntimeEvent::AgentSpawned { role, model, .. } = &envelope.payload {
            if let Some(model) = non_empty(model) {
                if role == "implementer" {
                    implementer = Some(model.to_string());
                    break;
                }
                first.get_or_insert_with(|| model.to_string());
            }
        }
    }
    implementer.or(first)
}

fn default_report_output(success: bool) -> String {
    if success {
        "success".to_string()
    } else {
        "workflow did not produce agent output".to_string()
    }
}

/// A recorded workflow phase transition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PhaseTransitionRecord {
    /// Previous phase.
    pub from: Phase,
    /// New phase.
    pub to: Phase,
    /// Timestamp in milliseconds since unix epoch.
    pub at_ms: u64,
}

/// Typed result of an agent effect.
#[derive(Debug, Clone)]
pub enum AgentOutcome {
    /// Agent completed and returned model output.
    Completed {
        /// Agent role.
        role: String,
        /// Final textual output.
        output: String,
        /// Number of files changed by the agent.
        files_changed: u32,
        /// Requested model before routing.
        requested_model: String,
        /// Routed model, when routing changed the request.
        routed_model: Option<String>,
        /// Actual final model used.
        final_model: String,
        /// Actual provider id used.
        provider_id: String,
        /// Usage reported by the model provider.
        usage: TokenUsage,
        /// Provider or gateway request id.
        request_id: Option<String>,
    },
    /// Agent failed.
    Failed {
        /// Agent role.
        role: String,
        /// Error category.
        kind: EffectErrorKind,
        /// Human-readable failure details.
        message: String,
    },
}

/// Typed result of a gate effect.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GateRunOutcome {
    /// Gate name.
    pub name: String,
    /// Whether the gate passed under the legacy report shape.
    pub passed: bool,
    /// Optional gate output or failure details.
    pub output: Option<String>,
    /// Gate runtime in milliseconds.
    pub duration_ms: u64,
}

/// Durable terminal record for one runner task attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskTerminalOutcome {
    /// Plan containing the task.
    pub plan_id: String,
    /// Task identifier within the plan.
    pub task_id: String,
    /// Monotonic attempt number for this task.
    pub attempt: u32,
    /// Whether the task reached the successful terminal.
    pub passed: bool,
    /// Terminal reason for failures or persistence errors.
    pub reason: Option<String>,
    /// Files durably attributed to this task.
    pub output_files: Vec<String>,
    /// Commit outcome observed before success was recorded.
    pub commit_outcome: CommitOutcome,
    /// Gate duration associated with terminalization.
    pub duration_ms: u64,
    /// Timestamp in milliseconds since unix epoch.
    pub timestamp_ms: u64,
}

/// CLI-independent identity of one task attempt in a timeout terminal.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TimeoutTaskAttemptRef {
    /// Plan containing the task.
    pub plan_id: String,
    /// Task identifier within the plan.
    pub task_id: String,
    /// Monotonic attempt number for the task.
    pub attempt: u32,
}

/// CLI-independent gate-like effect kind retained in timeout audit records.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TimeoutEffectKind {
    /// Verification probe before agent dispatch.
    Preflight,
    /// Post-agent task gate.
    Gate,
    /// Plan-level verification.
    PlanVerify,
    /// Merge or post-merge verification.
    Merge,
}

/// Exact asynchronous effect identity retained in a timeout terminal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimeoutEffectRef {
    /// Exact task attempt owning the effect.
    pub attempt: TimeoutTaskAttemptRef,
    /// Gate-like effect category.
    pub kind: TimeoutEffectKind,
    /// Gate rung, including canonical synthetic rungs.
    pub rung: u32,
    /// Dispatch generation distinguishing repeated effects.
    pub generation: u64,
}

/// Cause of a durable timeout terminal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TimeoutTerminalKind {
    /// Non-resetting run hard cap.
    HardRun,
    /// Whole task-attempt deadline.
    TaskAttempt,
    /// Gate-like effect deadline.
    GateEffect,
    /// Agent output-silence deadline.
    AgentSilence,
    /// Scheduler useful-progress deadline.
    SchedulerNoProgress,
    /// Owned producer ended without a matching completion.
    LostEffect,
}

/// Durable terminal timeout for one exact task attempt.
///
/// The shape intentionally matches the historical runner `timeout` payload,
/// allowing previously written exact-attempt records to deserialize without a
/// CLI dependency. `observed_at_ms` is audit data only; deadline restoration
/// uses the monotonic elapsed duration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskTimeoutTerminal {
    /// Cause of the timeout terminal.
    pub kind: TimeoutTerminalKind,
    /// Exact task attempt terminalized by the timeout.
    pub attempt: TimeoutTaskAttemptRef,
    /// Exact gate-like effect, when applicable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effect: Option<TimeoutEffectRef>,
    /// Generic ownership-effect generation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner_effect: Option<u64>,
    /// Configured deadline duration.
    pub limit_ms: u64,
    /// Monotonic elapsed duration observed by the enforcing process.
    pub monotonic_elapsed_ms: u64,
    /// Unix timestamp retained only for audit display and event ordering.
    pub observed_at_ms: u64,
}

/// Exact idempotency key for a task timeout terminal.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TimeoutTerminalKey {
    /// Stable runner invocation identifier.
    pub run_id: String,
    /// Plan containing the task.
    pub plan_id: String,
    /// Task identifier within the plan.
    pub task_id: String,
    /// Monotonic attempt number for the task.
    pub attempt: u32,
}

/// Typed JSONL entry in the runtime run ledger.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TimeoutLedgerEntry {
    /// One exact terminal timeout record.
    TimeoutRecorded {
        /// Stable runner invocation identifier.
        run_id: String,
        /// Typed timeout terminal.
        timeout: TaskTimeoutTerminal,
    },
}

impl TimeoutLedgerEntry {
    /// Construct an exact terminal-timeout entry.
    pub fn timeout_recorded(run_id: impl Into<String>, timeout: TaskTimeoutTerminal) -> Self {
        Self::TimeoutRecorded {
            run_id: run_id.into(),
            timeout,
        }
    }

    /// Stable runner invocation identifier.
    pub fn run_id(&self) -> &str {
        match self {
            Self::TimeoutRecorded { run_id, .. } => run_id,
        }
    }

    /// Typed terminal timeout payload.
    pub fn timeout(&self) -> &TaskTimeoutTerminal {
        match self {
            Self::TimeoutRecorded { timeout, .. } => timeout,
        }
    }

    /// Compute the exact idempotency key.
    pub fn key(&self) -> TimeoutTerminalKey {
        let timeout = self.timeout();
        TimeoutTerminalKey {
            run_id: self.run_id().to_string(),
            plan_id: timeout.attempt.plan_id.clone(),
            task_id: timeout.attempt.task_id.clone(),
            attempt: timeout.attempt.attempt,
        }
    }
}

/// Monotonic deadline data safe to restore in a new process.
///
/// Wall-clock audit timestamps are deliberately absent. A new monotonic
/// baseline is derived only from elapsed duration, saturating at the new
/// process origin rather than converting unix time into an `Instant`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimeoutDeadlineIntent {
    /// Timeout cause whose deadline intent is restored.
    pub kind: TimeoutTerminalKind,
    /// Configured deadline duration.
    pub limit_ms: u64,
    /// Previously observed monotonic elapsed duration.
    pub elapsed_ms: u64,
}

impl TaskTimeoutTerminal {
    /// Return restart-safe monotonic deadline intent, excluding wall time.
    pub fn deadline_intent(&self) -> TimeoutDeadlineIntent {
        TimeoutDeadlineIntent {
            kind: self.kind,
            limit_ms: self.limit_ms,
            elapsed_ms: self.monotonic_elapsed_ms,
        }
    }
}

impl TimeoutDeadlineIntent {
    /// Remaining deadline budget after the persisted monotonic elapsed time.
    pub fn remaining_ms(self) -> u64 {
        self.limit_ms.saturating_sub(self.elapsed_ms)
    }

    /// Derive a baseline in the new process's monotonic domain.
    pub fn resumed_baseline_ms(self, monotonic_now_ms: u64) -> u64 {
        monotonic_now_ms.saturating_sub(self.elapsed_ms)
    }
}

/// Ordered, idempotent replay of typed task timeout terminals.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TimeoutTerminalReplay {
    entries: Vec<TimeoutLedgerEntry>,
}

impl TimeoutTerminalReplay {
    /// Insert a terminal exactly once, rejecting a conflicting same-key fact.
    pub fn record(&mut self, entry: TimeoutLedgerEntry) -> Result<bool, TimeoutLedgerConflict> {
        let key = entry.key();
        if let Some(existing) = self.entries.iter().find(|existing| existing.key() == key) {
            if existing == &entry {
                return Ok(false);
            }
            return Err(TimeoutLedgerConflict::ConflictingTerminal { key });
        }
        self.entries.push(entry);
        Ok(true)
    }

    /// Ordered unique timeout terminals.
    pub fn entries(&self) -> &[TimeoutLedgerEntry] {
        &self.entries
    }

    /// Look up an exact timeout terminal.
    pub fn get(&self, key: &TimeoutTerminalKey) -> Option<&TimeoutLedgerEntry> {
        self.entries.iter().find(|entry| entry.key() == *key)
    }
}

/// A duplicate timeout key carried a different terminal fact.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TimeoutLedgerConflict {
    /// An exact key was already occupied by a different timeout fact.
    #[error("timeout terminal conflicts with the existing exact key {key:?}")]
    ConflictingTerminal {
        /// Exact occupied key.
        key: TimeoutTerminalKey,
    },
    /// An entry was offered to a run ledger with a different run identifier.
    #[error("timeout entry run {entry_run_id} does not belong to ledger {ledger_run_id}")]
    RunMismatch {
        /// Run identifier owned by the in-memory ledger.
        ledger_run_id: String,
        /// Run identifier carried by the rejected entry.
        entry_run_id: String,
    },
}

impl GateRunOutcome {
    fn to_report_gate(&self) -> GateOutcome {
        GateOutcome {
            name: self.name.clone(),
            passed: self.passed,
            output: self.output.clone(),
            duration_ms: self.duration_ms,
        }
    }
}

/// Typed result of artifact production or validation.
#[derive(Debug, Clone, PartialEq)]
pub enum ArtifactOutcome {
    /// Artifact exists and validation passed.
    Valid {
        /// Artifact category, such as `prd` or `plan`.
        artifact_type: String,
        /// Path to the validated artifact.
        path: PathBuf,
        /// Validation report or related metadata.
        report: serde_json::Value,
    },
    /// Artifact exists or was attempted, but validation failed.
    Invalid {
        /// Artifact category, such as `prd` or `plan`.
        artifact_type: String,
        /// Path to the invalid artifact, when it exists.
        path: Option<PathBuf>,
        /// Validation report or related metadata.
        report: serde_json::Value,
    },
    /// Required artifact was not produced.
    NotProduced {
        /// Artifact category, such as `prd` or `plan`.
        artifact_type: String,
        /// Human-readable reason the artifact was not produced.
        reason: String,
    },
    /// Artifact validation could not run.
    ValidationUnavailable {
        /// Artifact category, such as `prd` or `plan`.
        artifact_type: String,
        /// Human-readable reason validation was unavailable.
        reason: String,
    },
}

/// Typed cancellation record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CancellationOutcome {
    /// Cancellation was requested before another effect started.
    Requested {
        /// Phase active when cancellation was observed.
        phase: Phase,
        /// Timestamp in milliseconds since unix epoch when cancellation was observed.
        requested_at_ms: u64,
    },
    /// Cancellation interrupted an in-flight effect.
    Interrupted {
        /// Identifier for the interrupted effect.
        effect_id: String,
    },
    /// Cancellation waited for an in-flight effect to finish.
    WaitedForEffect {
        /// Identifier for the effect that was allowed to finish.
        effect_id: String,
    },
}

/// Event persistence health for the run.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum EventPersistenceHealth {
    /// No persistence issue observed.
    #[default]
    Healthy,
    /// Event persistence is best-effort or unavailable.
    Degraded {
        /// Human-readable degradation reason.
        reason: String,
    },
    /// Event persistence failed.
    Failed {
        /// Human-readable failure reason.
        reason: String,
    },
}

/// Typed effect error category.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EffectErrorKind {
    /// Effect was cancelled.
    Cancelled,
    /// Effect exceeded its timeout.
    TimedOut,
    /// Effect exceeded a budget limit.
    BudgetExceeded,
    /// Required authentication was missing.
    AuthMissing,
    /// Provider was unavailable.
    ProviderUnavailable,
    /// Prompt assembly failed before dispatch.
    PromptAssemblyFailed,
    /// Required tool was unavailable.
    ToolUnavailable,
    /// Error category is unknown.
    Unknown,
}

fn summarize_text(text: &str, max_chars: usize) -> String {
    let trimmed = text.trim();
    if trimmed.chars().count() <= max_chars {
        return trimmed.to_string();
    }

    let mut summary: String = trimmed.chars().take(max_chars.saturating_sub(3)).collect();
    summary.push_str("...");
    summary
}

fn non_empty(value: &str) -> Option<&str> {
    let trimmed = value.trim();
    (!trimmed.is_empty()).then_some(trimmed)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn timeout_entry(observed_at_ms: u64) -> TimeoutLedgerEntry {
        let attempt = TimeoutTaskAttemptRef {
            plan_id: "plan".into(),
            task_id: "T1".into(),
            attempt: 2,
        };
        TimeoutLedgerEntry::timeout_recorded(
            "run-1",
            TaskTimeoutTerminal {
                kind: TimeoutTerminalKind::LostEffect,
                attempt: attempt.clone(),
                effect: Some(TimeoutEffectRef {
                    attempt,
                    kind: TimeoutEffectKind::Gate,
                    rung: 1,
                    generation: 17,
                }),
                owner_effect: Some(17),
                limit_ms: 500,
                monotonic_elapsed_ms: 725,
                observed_at_ms,
            },
        )
    }

    #[test]
    fn run_ledger_report_compat_uses_agent_model_provider_and_usage() {
        let mut ledger = RunLedger::new(
            "run-1",
            "implement the requested change",
            WorkflowConfig::express(),
            1_700_000_000_000,
        );
        ledger.agent_outcomes.push(AgentOutcome::Completed {
            role: "implementer".into(),
            output: "done".into(),
            files_changed: 2,
            requested_model: "requested-model".into(),
            routed_model: Some("routed-model".into()),
            final_model: "final-model".into(),
            provider_id: "provider-a".into(),
            usage: TokenUsage {
                input_tokens: 10,
                output_tokens: 20,
                total_tokens: 30,
                cost_usd: 0.42,
            },
            request_id: Some("req-1".into()),
        });

        let report = ledger.to_report_compat(true, 1.5, Vec::new());

        assert_eq!(report.run_id, "run-1");
        assert!(report.success);
        assert_eq!(report.model, "final-model");
        assert_eq!(report.provider.as_deref(), Some("provider-a"));
        assert_eq!(report.output, "done");
        assert_eq!(report.agent_turns, 1);
        assert_eq!(report.token_usage, 30);
        assert_eq!(report.cost, Some(0.42));
        assert!(report.events.is_empty());
    }

    #[test]
    fn run_ledger_report_compat_preserves_shape_with_cancellation() {
        let mut ledger = RunLedger::new(
            "run-cancelled",
            "stop the run",
            WorkflowConfig::express(),
            1_700_000_000_000,
        );

        ledger.record_cancellation_requested(Phase::Implementing, 1_700_000_000_123);

        assert_eq!(
            ledger.cancellation,
            Some(CancellationOutcome::Requested {
                phase: Phase::Implementing,
                requested_at_ms: 1_700_000_000_123,
            })
        );

        let report = ledger.to_report_compat(false, 0.25, Vec::new());

        assert!(!report.success);
        assert_eq!(report.model, "unconfigured");
        assert_eq!(report.output, "workflow did not produce agent output");
        assert_eq!(report.agent_turns, 0);
        assert_eq!(report.token_usage, 0);
        assert_eq!(report.cost, None);
        assert!(report.gates.is_empty());
        assert!(report.events.is_empty());
    }

    #[test]
    fn run_ledger_records_task_terminal_once_per_attempt() {
        let mut ledger = RunLedger::new("run-1", "prompt", WorkflowConfig::express(), 1);
        let outcome = TaskTerminalOutcome {
            plan_id: "plan".into(),
            task_id: "T1".into(),
            attempt: 2,
            passed: true,
            reason: None,
            output_files: vec!["src/lib.rs".into()],
            commit_outcome: CommitOutcome::Created {
                hash: "abc123".into(),
            },
            duration_ms: 42,
            timestamp_ms: 100,
        };

        assert!(ledger.record_task_terminal(outcome.clone()));
        assert!(!ledger.record_task_terminal(outcome));
        assert_eq!(ledger.task_terminals.len(), 1);
        assert_eq!(
            ledger.commit,
            Some(CommitOutcome::Created {
                hash: "abc123".into()
            })
        );
    }

    #[test]
    fn typed_timeout_terminal_is_serde_stable_and_idempotent_on_replay() {
        let entry = timeout_entry(123);
        let encoded = serde_json::to_string(&entry).unwrap();
        assert!(encoded.contains("\"kind\":\"timeout_recorded\""));
        let decoded: TimeoutLedgerEntry = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded, entry);

        let mut replay = TimeoutTerminalReplay::default();
        assert_eq!(replay.record(decoded.clone()), Ok(true));
        assert_eq!(replay.record(decoded), Ok(false));
        assert_eq!(replay.entries(), &[entry]);
    }

    #[test]
    fn timeout_replay_rejects_conflicting_fact_for_same_exact_key() {
        let first = timeout_entry(123);
        let mut conflicting = timeout_entry(123);
        let TimeoutLedgerEntry::TimeoutRecorded { timeout, .. } = &mut conflicting;
        timeout.kind = TimeoutTerminalKind::GateEffect;

        let mut replay = TimeoutTerminalReplay::default();
        assert_eq!(replay.record(first), Ok(true));
        assert!(matches!(
            replay.record(conflicting),
            Err(TimeoutLedgerConflict::ConflictingTerminal { .. })
        ));
        assert_eq!(replay.entries().len(), 1);
    }

    #[test]
    fn run_ledger_keys_timeout_by_run_plan_task_and_attempt() {
        let mut ledger = RunLedger::new("run-1", "prompt", WorkflowConfig::express(), 1);
        let entry = timeout_entry(123);
        let key = entry.key();
        assert_eq!(ledger.record_timeout_terminal(entry.clone()), Ok(true));
        assert_eq!(ledger.record_timeout_terminal(entry), Ok(false));
        assert!(ledger.timeout_terminals.get(&key).is_some());

        let wrong_run =
            TimeoutLedgerEntry::timeout_recorded("run-2", timeout_entry(123).timeout().clone());
        assert!(matches!(
            ledger.record_timeout_terminal(wrong_run),
            Err(TimeoutLedgerConflict::RunMismatch { .. })
        ));
    }

    #[test]
    fn resume_deadline_intent_never_converts_wall_clock_to_monotonic_time() {
        let low_wall = timeout_entry(0);
        let high_wall = timeout_entry(u64::MAX);
        let low_intent = low_wall.timeout().deadline_intent();
        let high_intent = high_wall.timeout().deadline_intent();

        assert_eq!(low_intent, high_intent);
        assert_eq!(low_intent.remaining_ms(), 0);
        assert_eq!(low_intent.resumed_baseline_ms(1_000), 275);
        assert_eq!(low_intent.resumed_baseline_ms(10), 0);
    }
}
