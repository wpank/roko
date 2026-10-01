//! Plan-task execution Cell.
//!
//! `roko-graph` deliberately does not depend on the CLI/provider stack. Live
//! execution is supplied by an injected [`TaskDispatcher`], which lets the CLI
//! reuse its canonical routing, prompt, safety, MCP, and provider machinery.
//! A registry without that injection fails closed; it never reports synthetic
//! dry-run output as successful live work.
//!
//! ## Streaming dispatch (#274)
//!
//! [`StreamingTaskDispatcher`] extends the basic [`TaskDispatcher`] seam with
//! `dispatch_streaming` (live text/tool/usage events via a channel) and
//! `reconcile_attempt` (crash/retry idempotence). The host implementation
//! lives in `roko-cli::graph_task_dispatch`; these Graph-layer types carry
//! no CLI dependency.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use roko_core::{Body, Kind, ProtocolId, Signal, error::Result};
use serde::{Deserialize, Serialize};

use crate::cell::{Cell, CellContext, CellVersion};
use crate::workspace::{WorkspaceAcceptance, WorkspaceLease};

/// Provider-neutral task metadata preserved by plan-to-Graph conversion.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TaskExecutionSpec {
    /// Owning plan identifier.
    pub plan_id: String,
    /// Source plan directory.
    pub plan_dir: String,
    /// Human-readable task title.
    pub title: String,
    /// Optional detailed task description.
    pub description: Option<String>,
    /// Requested agent role.
    pub role: Option<String>,
    /// Complexity/tier label.
    pub tier: String,
    /// Optional author-selected model.
    pub model_hint: Option<String>,
    /// Files expected to be in scope.
    pub files: Vec<String>,
    /// Per-task timeout.
    pub timeout_secs: u64,
    /// Retry ceiling from the source task definition.
    pub max_retries: u32,
    /// Full serialized runner task definition.
    pub task_def_json: String,
    /// Hand a successful attempt's isolated checkout on instead of releasing
    /// it: a later cell of the task (the rich topology's `plan.gate`) judges
    /// that checkout, so it must outlive the dispatch. The attempt's output
    /// carries the lease (see [`TaskAttempt::lease`]).
    pub keep_workspace: bool,
}

impl TaskExecutionSpec {
    /// Decode the converter-owned TOML node configuration.
    #[must_use]
    pub fn from_config(config: &toml::Value) -> Self {
        let table = config.as_table();
        let string = |key: &str| {
            table
                .and_then(|value| value.get(key))
                .and_then(toml::Value::as_str)
                .map(ToOwned::to_owned)
        };
        let integer = |key: &str| {
            table
                .and_then(|value| value.get(key))
                .and_then(toml::Value::as_integer)
        };
        let files = table
            .and_then(|value| value.get("files"))
            .and_then(toml::Value::as_array)
            .map(|values| {
                values
                    .iter()
                    .filter_map(toml::Value::as_str)
                    .map(ToOwned::to_owned)
                    .collect()
            })
            .unwrap_or_default();

        Self {
            plan_id: string("plan_id").unwrap_or_default(),
            plan_dir: string("plan_dir").unwrap_or_default(),
            title: string("title").unwrap_or_else(|| "(unknown task)".to_string()),
            description: string("description"),
            role: string("role"),
            tier: string("tier").unwrap_or_else(|| "focused".to_string()),
            model_hint: string("model_hint"),
            files,
            timeout_secs: integer("timeout_secs")
                .and_then(|value| u64::try_from(value).ok())
                .unwrap_or(120),
            max_retries: integer("max_retries")
                .and_then(|value| u32::try_from(value).ok())
                .unwrap_or_default(),
            task_def_json: string("task_def_json").unwrap_or_default(),
            keep_workspace: table
                .and_then(|value| value.get("keep_workspace"))
                .and_then(toml::Value::as_bool)
                .unwrap_or(false),
        }
    }
}

/// Signal tag stamped on every live plan-task output with its verified gate
/// outcome (see [`TaskGateVerdict`]).
///
/// The tag travels with the output into the Activity checkpoint, so resume,
/// dashboards, and checkpoint extensions all read the same durable verdict.
pub const TASK_GATE_VERDICT_TAG: &str = "roko.gate.verdict";

/// Verified gate outcome of one plan task, carried on its output signals.
///
/// A task output is only a pass when its verdict says so: absent or
/// non-replayable verdicts must never be laundered into success.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskGateVerdict {
    /// Every authored `[[task.verify]]` step passed.
    Passed,
    /// Every authored verify step passed, or failed only on tests that also
    /// failed on the plan run's start commit, which the attempt neither
    /// caused nor was asked to fix. Replayed like a pass, but its own
    /// outcome, so it never looks like a clean pass.
    PassedWithPreexistingFailures,
    /// Every authored verify step passed on a tree the attempt left
    /// unchanged: the task's work was already there, as on a `--fresh`
    /// rerun of a finished task. Replayed like a pass, but its own outcome,
    /// since no change of the agent earned it.
    AlreadySatisfied,
    /// The task declares no verify steps; only the provider result is known.
    Unverified,
    /// Verification failed but a non-deterministic judge/review cap accepted
    /// the result anyway. Never a pass: resume re-runs it and dashboards show
    /// it apart from passed tasks. Deterministic authored verify steps never
    /// produce this verdict.
    ForcedAccept,
}

impl TaskGateVerdict {
    /// Stable tag value.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Passed => "passed",
            Self::PassedWithPreexistingFailures => "passed_with_preexisting_failures",
            Self::AlreadySatisfied => "already_satisfied",
            Self::Unverified => "unverified",
            Self::ForcedAccept => "forced_accept",
        }
    }

    /// Parse a tag value; unknown values return `None`.
    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "passed" => Some(Self::Passed),
            "passed_with_preexisting_failures" => Some(Self::PassedWithPreexistingFailures),
            "already_satisfied" => Some(Self::AlreadySatisfied),
            "unverified" => Some(Self::Unverified),
            "forced_accept" => Some(Self::ForcedAccept),
            _ => None,
        }
    }

    /// Whether a recorded output with this verdict may be replayed as a
    /// completed node on resume.
    #[must_use]
    pub const fn is_replayable(self) -> bool {
        !matches!(self, Self::ForcedAccept)
    }

    /// Read the verdict stamped on a node's outputs.
    ///
    /// The least trustworthy verdict wins when signals disagree, so a single
    /// forced-accept output can never be hidden behind a passing sibling.
    #[must_use]
    pub fn from_signals(signals: &[Signal]) -> Option<Self> {
        signals
            .iter()
            .filter_map(|signal| signal.tag(TASK_GATE_VERDICT_TAG).and_then(Self::parse))
            .max_by_key(|verdict| match verdict {
                Self::Passed => 0,
                Self::PassedWithPreexistingFailures => 1,
                Self::AlreadySatisfied => 2,
                Self::Unverified => 3,
                Self::ForcedAccept => 4,
            })
    }

    /// Stamp this verdict on every signal and refresh their content ids.
    pub fn stamp(self, signals: &mut [Signal]) {
        for signal in signals {
            signal
                .tags
                .insert(TASK_GATE_VERDICT_TAG.to_string(), self.as_str().to_string());
            signal.id = signal.content_hash();
        }
    }
}

/// Signal tags of a [`TaskAttempt`].
const TASK_PLAN_ID_TAG: &str = "plan_id";
const TASK_ID_TAG: &str = "task_id";
const TASK_RUN_ID_TAG: &str = "run_id";
const TASK_ATTEMPT_KEY_TAG: &str = "attempt.key";
const TASK_ATTEMPT_TAG: &str = "workspace.attempt";
const TASK_WORKSPACE_TAG: &str = "workspace.path";
const TASK_WORKSPACE_LEASE_TAG: &str = "workspace.lease";
const TASK_ATTEMPT_COMMIT_TAG: &str = "workspace.attempt_commit";
const TASK_PLAN_BRANCH_TAG: &str = "workspace.plan_branch";
const TASK_ACCEPTED_COMMIT_TAG: &str = "workspace.accepted_commit";

/// The attempt that produced a plan task's output, stamped on its signals
/// ([`Self::stamp`]).
///
/// The task's later cells read it back ([`Self::from_signals`]) to act on that
/// exact attempt: the rich topology's `plan.gate` gates the checkout named
/// here, never the process's working directory.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TaskAttempt {
    /// Owning plan.
    pub plan_id: String,
    /// Task within the plan.
    pub task_id: String,
    /// Graph run the attempt belongs to.
    pub run_id: Option<String>,
    /// Durable attempt key (`run:plan:task:ordinal`).
    pub attempt_key: Option<String>,
    /// 1-based ordinal of the attempt within its run.
    pub attempt: u32,
    /// Isolated checkout the attempt ran in. `None` when it ran in the shared
    /// working tree, which is the operator's own checkout.
    pub workspace: Option<PathBuf>,
    /// Lease of that checkout, when the executor handed it on unreleased
    /// ([`TaskExecutionSpec::keep_workspace`]) for a later cell to settle.
    pub lease: Option<WorkspaceLease>,
    /// Where the attempt's work landed, once it was accepted onto its plan's
    /// branch.
    pub accepted: Option<WorkspaceAcceptance>,
}

impl TaskAttempt {
    /// Stamp this attempt on every signal and refresh their content ids. A
    /// field that is `None` clears its tag, so a cell that settled the
    /// attempt's lease can stamp that over the tags it passes on.
    pub fn stamp(&self, signals: &mut [Signal]) {
        let lease = self
            .lease
            .as_ref()
            .and_then(|lease| serde_json::to_string(lease).ok());
        let accepted = self.accepted.as_ref();
        let tags = [
            (TASK_PLAN_ID_TAG, Some(self.plan_id.clone())),
            (TASK_ID_TAG, Some(self.task_id.clone())),
            (TASK_RUN_ID_TAG, self.run_id.clone()),
            (TASK_ATTEMPT_KEY_TAG, self.attempt_key.clone()),
            (TASK_ATTEMPT_TAG, Some(self.attempt.to_string())),
            (
                TASK_WORKSPACE_TAG,
                self.workspace
                    .as_ref()
                    .map(|path| path.display().to_string()),
            ),
            (TASK_WORKSPACE_LEASE_TAG, lease),
            (
                TASK_ATTEMPT_COMMIT_TAG,
                accepted.map(|accepted| accepted.attempt_commit.clone()),
            ),
            (
                TASK_PLAN_BRANCH_TAG,
                accepted.map(|accepted| accepted.plan_branch.clone()),
            ),
            (
                TASK_ACCEPTED_COMMIT_TAG,
                accepted.map(|accepted| accepted.accepted_commit.clone()),
            ),
        ];
        for signal in signals {
            for (tag, value) in &tags {
                match value {
                    Some(value) => signal.tags.insert((*tag).to_string(), value.clone()),
                    None => signal.tags.remove(*tag),
                };
            }
            signal.id = signal.content_hash();
        }
    }

    /// Read back the attempt stamped on a task's output: the first signal
    /// that names one. `Err` says why no usable attempt is named, so callers
    /// can fail closed instead of guessing.
    pub fn from_signals(signals: &[Signal]) -> std::result::Result<Self, String> {
        let signal = signals
            .iter()
            .find(|signal| signal.tag(TASK_ATTEMPT_TAG).is_some())
            .ok_or_else(|| {
                format!("no input names the attempt that produced it (`{TASK_ATTEMPT_TAG}` tag)")
            })?;
        let tag = |key: &str| signal.tag(key).map(ToOwned::to_owned);
        let ordinal = tag(TASK_ATTEMPT_TAG).unwrap_or_default();
        let attempt = ordinal
            .parse::<u32>()
            .ok()
            .filter(|attempt| *attempt > 0)
            .ok_or_else(|| {
                format!("`{TASK_ATTEMPT_TAG}` is `{ordinal}`, not a 1-based attempt ordinal")
            })?;
        let lease = tag(TASK_WORKSPACE_LEASE_TAG)
            .map(|lease| serde_json::from_str::<WorkspaceLease>(&lease))
            .transpose()
            .map_err(|error| {
                format!("`{TASK_WORKSPACE_LEASE_TAG}` is not a workspace lease: {error}")
            })?;
        let accepted = match (
            tag(TASK_ATTEMPT_COMMIT_TAG),
            tag(TASK_PLAN_BRANCH_TAG),
            tag(TASK_ACCEPTED_COMMIT_TAG),
        ) {
            (Some(attempt_commit), Some(plan_branch), Some(accepted_commit)) => {
                Some(WorkspaceAcceptance {
                    attempt_commit,
                    plan_branch,
                    accepted_commit,
                })
            }
            _ => None,
        };
        Ok(Self {
            plan_id: tag(TASK_PLAN_ID_TAG).unwrap_or_default(),
            task_id: tag(TASK_ID_TAG).unwrap_or_default(),
            run_id: tag(TASK_RUN_ID_TAG),
            attempt_key: tag(TASK_ATTEMPT_KEY_TAG),
            attempt,
            workspace: tag(TASK_WORKSPACE_TAG).map(PathBuf::from),
            lease,
            accepted,
        })
    }
}

/// Runtime seam for executing a converted plan task.
#[async_trait::async_trait]
pub trait TaskDispatcher: Send + Sync {
    /// Dispatch one task through the host's real agent runtime.
    async fn dispatch(
        &self,
        spec: &TaskExecutionSpec,
        input: Vec<Signal>,
        ctx: &CellContext,
    ) -> Result<Vec<Signal>>;

    /// Dispatch one task using the typed request contract (#247).
    ///
    /// Default implementation delegates to [`dispatch`](Self::dispatch) for
    /// backward compatibility. Host implementations should override this.
    async fn dispatch_request(
        &self,
        request: &TaskDispatchRequest,
        ctx: &CellContext,
    ) -> Result<TaskDispatchOutcome> {
        let output = self
            .dispatch(&request.spec, request.input.clone(), ctx)
            .await?;
        Ok(TaskDispatchOutcome {
            attempt_id: request.attempt_id.clone(),
            outcome: TaskDispatchOutcomeKind::Succeeded,
            provider_id: request.provider.clone().unwrap_or_default(),
            model: request.model.clone().unwrap_or_default(),
            input_tokens: None,
            output_tokens: None,
            cost_usd: None,
            changed_files: Vec::new(),
            wall_duration: Duration::ZERO,
            output,
        })
    }

    /// Dispatch with streaming events (#247).
    ///
    /// Default implementation calls [`dispatch_request`](Self::dispatch_request)
    /// and sends no intermediate events. The terminal outcome is returned
    /// directly.
    async fn dispatch_stream(
        &self,
        request: &TaskDispatchRequest,
        ctx: &CellContext,
        _event_tx: tokio::sync::mpsc::Sender<GraphTaskEvent>,
    ) -> Result<TaskDispatchOutcome> {
        self.dispatch_request(request, ctx).await
    }

    /// Reconcile a previously started attempt after crash/restart (#247).
    ///
    /// Default implementation returns `AllocateNew` so that an interrupted
    /// task is safely retried on resume. The checkpoint only records completed
    /// activities; if an activity is not in the checkpoint it was interrupted
    /// mid-execution and should be re-executed. Host implementations that
    /// have durable start evidence (e.g. a ledger that records attempt start
    /// before the provider call) may override this to return `FailAmbiguous`
    /// when the evidence indicates the provider may have already been invoked.
    async fn reconcile_attempt(
        &self,
        attempt_id: &str,
        _request_fingerprint: &str,
    ) -> AttemptReconciliation {
        AttemptReconciliation::AllocateNew {
            attempt_id: format!("{attempt_id}-retry"),
        }
    }
}

// ─── Streaming dispatch contract (#274) ─────────────────────────────────────

/// Event forwarded from a provider invocation during streaming dispatch.
///
/// Text and progress events may coalesce across channel sends. Tool
/// boundaries, usage, attempt receipts, and terminal outcomes are reliable
/// and must not be dropped.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[allow(missing_docs)]
pub enum GraphTaskEvent {
    /// Incremental assistant text (may coalesce).
    Text { text: String },
    /// A tool invocation boundary.
    ToolCall { id: String, name: String },
    /// A tool invocation result.
    ToolOutput { id: String, output: String },
    /// Token/cost usage update from the provider.
    Usage {
        input_tokens: u64,
        output_tokens: u64,
        cost_usd: Option<f64>,
    },
    /// Task execution progress indicator (may coalesce).
    ///
    /// `message` is truncated to 4096 bytes at the nearest valid UTF-8 boundary.
    Progress {
        message: String,
        completed: Option<u32>,
        total: Option<u32>,
    },
    /// The attempt has started (before any provider output).
    AttemptStarted { attempt_id: String },
    /// The attempt has reached a terminal state.
    AttemptTerminal {
        attempt_id: String,
        outcome: TaskDispatchOutcomeKind,
    },
}

/// Terminal classification of a task dispatch attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskDispatchOutcomeKind {
    /// The provider completed successfully.
    Succeeded,
    /// The provider returned an unsuccessful result.
    Failed,
    /// The dispatch was cancelled by the guard/supervisor.
    Cancelled,
    /// The dispatch exceeded its deadline.
    TimedOut,
}

/// Concrete outcome of a single streaming task dispatch.
#[derive(Debug, Clone)]
pub struct TaskDispatchOutcome {
    /// Unique attempt identity for receipt recording.
    pub attempt_id: String,
    /// Terminal outcome classification.
    pub outcome: TaskDispatchOutcomeKind,
    /// Provider identifier that handled the attempt.
    pub provider_id: String,
    /// Model slug the provider actually used.
    pub model: String,
    /// Input tokens from actual provider usage. `None` when the provider
    /// did not report usage.
    pub input_tokens: Option<u64>,
    /// Output tokens from actual provider usage.
    pub output_tokens: Option<u64>,
    /// Actual cost in USD from the provider. `None` when the provider did
    /// not report cost; never a configured-model estimate.
    pub cost_usd: Option<f64>,
    /// Files changed relative to the lease base, when available.
    pub changed_files: Vec<String>,
    /// Wall-clock duration of the attempt.
    pub wall_duration: Duration,
    /// The output signals from the provider, if the attempt succeeded.
    pub output: Vec<Signal>,
}

/// Result of reconciling a previously started attempt on resume.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AttemptReconciliation {
    /// Terminal evidence exists for a completed attempt. The host should
    /// reuse this committed result without re-invoking the provider.
    ReuseCommitted {
        /// The committed attempt identity.
        attempt_id: String,
    },
    /// No evidence that the previous attempt started. A new attempt ID
    /// was allocated and the host should proceed with a fresh dispatch.
    AllocateNew {
        /// Freshly allocated attempt identity.
        attempt_id: String,
    },
    /// The previous attempt started but cannot be conclusively reconciled.
    /// The host must not retry the provider call.
    FailAmbiguous {
        /// The ambiguous attempt identity.
        attempt_id: String,
        /// Diagnostic reason for the ambiguity.
        reason: String,
    },
}

/// Durable identity recorder for provider attempt start/terminal receipts.
///
/// The production implementation (#251) supplies durable recording; this
/// packet uses a no-op fake so the host integration can be tested without
/// a ledger dependency.
#[async_trait::async_trait]
pub trait ProviderAttemptRecorder: Send + Sync {
    /// Record that an attempt with `attempt_id` is about to start.
    async fn record_start(&self, attempt_id: &str, spec: &TaskExecutionSpec) -> Result<()>;

    /// Record the terminal outcome of an attempt.
    async fn record_terminal(&self, attempt_id: &str, outcome: &TaskDispatchOutcome) -> Result<()>;

    /// Check whether a previous attempt has terminal evidence on disk.
    async fn has_terminal_evidence(&self, attempt_id: &str) -> bool;

    /// Check whether a previous attempt has started-but-not-terminal evidence.
    async fn has_started_evidence(&self, attempt_id: &str) -> bool;
}

/// Lease path and identity that the host must validate before dispatch.
///
/// The host does not acquire or release the lease; it only validates that
/// the caller has already acquired it and that the workdir is the lease
/// path (not a shared checkout).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskLease {
    /// Absolute path to the acquired worktree lease.
    pub path: PathBuf,
    /// Fingerprint of the lease (e.g. content hash at acquisition time).
    pub fingerprint: String,
}

/// Extended dispatcher seam for streaming task execution (#274).
///
/// Callers that only need synchronous output should continue using
/// [`TaskDispatcher`]. `StreamingTaskDispatcher` adds live event
/// forwarding and crash-safe attempt reconciliation.
#[async_trait::async_trait]
pub trait StreamingTaskDispatcher: TaskDispatcher {
    /// Dispatch one task with live streaming events.
    ///
    /// Events are sent through `event_tx` as they arrive from the provider.
    /// The channel uses bounded capacity; text/progress may coalesce but
    /// tool boundaries, usage, and terminal outcomes are reliable.
    ///
    /// `lease` must reference an already-acquired worktree. The dispatcher
    /// validates the path and fingerprint but does not acquire or release it.
    ///
    /// `recorder` receives attempt start/terminal receipts. Use
    /// [`NoopAttemptRecorder`] when durable recording is not yet available.
    async fn dispatch_streaming(
        &self,
        spec: &TaskExecutionSpec,
        input: Vec<Signal>,
        ctx: &CellContext,
        lease: &TaskLease,
        event_tx: tokio::sync::mpsc::Sender<GraphTaskEvent>,
        recorder: &dyn ProviderAttemptRecorder,
    ) -> Result<TaskDispatchOutcome>;

    /// Reconcile a previously started attempt after a crash or restart.
    ///
    /// Returns `ReuseCommitted` when terminal evidence exists (no provider
    /// call needed), `AllocateNew` when the attempt provably never started,
    /// and `FailAmbiguous` when the attempt started but cannot be reconciled
    /// (the caller must not retry).
    async fn reconcile_attempt(
        &self,
        spec: &TaskExecutionSpec,
        previous_attempt_id: &str,
        recorder: &dyn ProviderAttemptRecorder,
    ) -> AttemptReconciliation;
}

/// No-op attempt recorder for the additive integration packet.
///
/// Production recording (#251) replaces this before #256 activation.
#[derive(Debug, Clone, Copy)]
pub struct NoopAttemptRecorder;

#[async_trait::async_trait]
impl ProviderAttemptRecorder for NoopAttemptRecorder {
    async fn record_start(&self, _attempt_id: &str, _spec: &TaskExecutionSpec) -> Result<()> {
        Ok(())
    }
    async fn record_terminal(
        &self,
        _attempt_id: &str,
        _outcome: &TaskDispatchOutcome,
    ) -> Result<()> {
        Ok(())
    }
    async fn has_terminal_evidence(&self, _attempt_id: &str) -> bool {
        false
    }
    async fn has_started_evidence(&self, _attempt_id: &str) -> bool {
        false
    }
}

// ─── Spec-named type aliases ─────────────────────────────────────────────────

/// Spec name for [`GraphTaskEvent`] (#247).
pub type TaskDispatchEvent = GraphTaskEvent;

/// Spec name for [`TaskDispatchOutcomeKind`] (#247).
pub type TaskDispatchStatus = TaskDispatchOutcomeKind;

/// Spec name for [`AttemptReconciliation`] (#247).
pub type AttemptReconcileDecision = AttemptReconciliation;

// ─── Typed dispatch request (#247) ──────────────────────────────────────────

/// Maximum byte length for a `Progress` message before truncation.
pub const PROGRESS_MESSAGE_MAX_BYTES: usize = 4096;

/// Provider-neutral request for a single task dispatch.
///
/// Contains everything the host needs to route, execute, and record one
/// provider attempt. The `TaskExecutorCell` constructs this before calling
/// `dispatch_request`.
#[derive(Debug, Clone)]
pub struct TaskDispatchRequest {
    /// Plan-scoped task identifier (node ID).
    pub task_id: String,
    /// Unique attempt identity (ULID). Generated before dispatch and
    /// recorded against reconciliation state.
    pub attempt_id: String,
    /// Owning plan identifier.
    pub plan_id: String,
    /// Run identifier (from `CellContext.run_id`).
    pub run_id: Option<String>,
    /// Node identifier within the graph.
    pub node_id: Option<String>,
    /// Resolved agent role (e.g. "implementer", "reviewer").
    pub role: String,
    /// Resolved effort/complexity tier (e.g. "mechanical", "focused").
    pub effort: String,
    /// Resolved provider identifier (from cascade routing).
    pub provider: Option<String>,
    /// Resolved model slug (from cascade routing or hint).
    pub model: Option<String>,
    /// Input signals from upstream cells.
    pub input: Vec<Signal>,
    /// Working directory for the dispatch (lease path).
    pub workdir: PathBuf,
    /// Effective capability intersection for this task.
    pub capabilities: Vec<roko_core::Capability>,
    /// Unix millisecond deadline for this dispatch.
    pub deadline_ms: Option<i64>,
    /// Budget ceiling in micro-USD for this attempt.
    pub budget_micro_usd: Option<u64>,
    /// Tool policy override (serialized JSON).
    pub tool_policy: Option<serde_json::Value>,
    /// Full task execution spec (backward compat).
    pub spec: TaskExecutionSpec,
}

// ─── Provider attempt receipt (#247) ────────────────────────────────────────

/// Durable receipt for a single provider attempt.
///
/// Every provider call produces exactly one terminal receipt. The
/// `attempt_id` matches the pre-recorded identity from the dispatch
/// request. Receipts are ordered by `started_at_ms` within a task.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderAttemptReceipt {
    /// Stable attempt identity (ULID).
    pub attempt_id: String,
    /// BLAKE3 fingerprint of the dispatch request (for deduplication).
    pub request_fingerprint: String,
    /// Provider-assigned request identifier (opaque).
    pub provider_request_id: Option<String>,
    /// Unix millisecond timestamp when the attempt started.
    pub started_at_ms: u64,
    /// Unix millisecond timestamp when the attempt completed.
    pub completed_at_ms: u64,
    /// Terminal status of the attempt.
    pub status: TaskDispatchOutcomeKind,
    /// Input tokens consumed (provider-reported).
    pub input_tokens: u64,
    /// Output tokens produced (provider-reported).
    pub output_tokens: u64,
    /// Actual cost in micro-USD (provider-reported).
    pub cost_micro_usd: u64,
    /// Error message, if the attempt failed.
    pub error: Option<String>,
}

// ─── Truncation helper ──────────────────────────────────────────────────────

/// Truncate `s` to at most `max_bytes` bytes at a valid UTF-8 char boundary.
#[must_use]
pub fn truncate_utf8(s: &str, max_bytes: usize) -> &str {
    if s.len() <= max_bytes {
        return s;
    }
    let mut end = max_bytes;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    &s[..end]
}

enum TaskExecutionMode {
    /// Explicit diagnostic mode. This is never selected by the production
    /// plan Graph path.
    DryRun,
    /// Real host-owned provider dispatch.
    Live(Arc<dyn TaskDispatcher>),
    /// No runtime was injected. Execution must fail closed.
    Unconfigured,
}

/// Cell backing every plan task produced by `plan_to_graph`.
pub struct TaskExecutorCell {
    spec: TaskExecutionSpec,
    mode: TaskExecutionMode,
}

impl TaskExecutorCell {
    /// Construct an explicitly synthetic diagnostic cell.
    #[must_use]
    pub fn dry_run(config: toml::Value) -> Self {
        Self {
            spec: TaskExecutionSpec::from_config(&config),
            mode: TaskExecutionMode::DryRun,
        }
    }

    /// Construct a live cell using the host-supplied dispatcher.
    #[must_use]
    pub fn live(config: toml::Value, dispatcher: Arc<dyn TaskDispatcher>) -> Self {
        Self {
            spec: TaskExecutionSpec::from_config(&config),
            mode: TaskExecutionMode::Live(dispatcher),
        }
    }

    /// Construct a fail-closed cell for registries without a host runtime.
    #[must_use]
    pub fn unconfigured(config: toml::Value) -> Self {
        Self {
            spec: TaskExecutionSpec::from_config(&config),
            mode: TaskExecutionMode::Unconfigured,
        }
    }
}

impl Default for TaskExecutorCell {
    fn default() -> Self {
        Self::unconfigured(toml::Value::Table(toml::map::Map::new()))
    }
}

#[async_trait::async_trait]
impl Cell for TaskExecutorCell {
    fn cell_id(&self) -> &'static str {
        "task-executor"
    }

    fn cell_name(&self) -> &'static str {
        "TaskExecutorCell"
    }

    fn cell_version(&self) -> CellVersion {
        (0, 2, 0)
    }

    fn protocols(&self) -> Vec<ProtocolId> {
        Vec::new()
    }

    fn estimated_cost(&self) -> Option<f64> {
        None
    }

    fn estimated_duration(&self) -> Option<Duration> {
        Some(Duration::from_secs(self.spec.timeout_secs.max(1)))
    }

    async fn execute(&self, input: Vec<Signal>, ctx: &CellContext) -> Result<Vec<Signal>> {
        match &self.mode {
            TaskExecutionMode::DryRun => {
                tracing::info!(
                    plan = %self.spec.plan_id,
                    task = %self.spec.title,
                    "TaskExecutorCell explicit dry-run: skipping agent dispatch"
                );
                Ok(vec![
                    Signal::builder(Kind::AgentOutput)
                        .body(Body::text(format!(
                            "task-output:dry-run:{}",
                            self.spec.title
                        )))
                        .build(),
                ])
            }
            TaskExecutionMode::Live(dispatcher) => {
                let mut retry = 0_u32;
                loop {
                    match dispatcher.dispatch(&self.spec, input.clone(), ctx).await {
                        Ok(output) => return Ok(output),
                        // A non-retryable gateway error (e.g. every candidate
                        // provider is out of usage) fails identically on an
                        // immediate retry, and so does a gate's rejection
                        // (e.g. a plan branch that refused the attempt's
                        // work), so surface them at once. A cancellation
                        // means the run is stopping, which a retry would
                        // only delay.
                        Err(error)
                            if retry < self.spec.max_retries
                                && !matches!(
                                    error,
                                    roko_core::error::RokoError::Gateway {
                                        retryable: false,
                                        ..
                                    } | roko_core::error::RokoError::Rejected(_)
                                        | roko_core::error::RokoError::Cancelled(_)
                                ) =>
                        {
                            retry = retry.saturating_add(1);
                            tracing::warn!(
                                plan = %self.spec.plan_id,
                                task = %self.spec.title,
                                attempt = retry,
                                max_retries = self.spec.max_retries,
                                error = %error,
                                "TaskExecutorCell provider dispatch failed; retrying"
                            );
                        }
                        Err(error) => return Err(error),
                    }
                }
            }
            TaskExecutionMode::Unconfigured => Err(roko_core::error::RokoError::Agent {
                backend: "graph-task-executor".to_string(),
                message: format!(
                    "live dispatcher is not configured for plan task `{}`; refusing synthetic success",
                    self.spec.title
                ),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use parking_lot::Mutex;

    use super::*;

    #[derive(Default)]
    struct CapturingDispatcher {
        calls: Mutex<Vec<(TaskExecutionSpec, usize, Option<String>)>>,
    }

    #[derive(Default)]
    struct FailsOnceDispatcher {
        calls: AtomicUsize,
    }

    #[async_trait::async_trait]
    impl TaskDispatcher for FailsOnceDispatcher {
        async fn dispatch(
            &self,
            _spec: &TaskExecutionSpec,
            _input: Vec<Signal>,
            _ctx: &CellContext,
        ) -> Result<Vec<Signal>> {
            if self.calls.fetch_add(1, Ordering::SeqCst) == 0 {
                return Err(roko_core::error::RokoError::Agent {
                    backend: "test".to_string(),
                    message: "transient".to_string(),
                });
            }
            Ok(vec![
                Signal::builder(Kind::AgentOutput)
                    .body(Body::text("retry-output"))
                    .build(),
            ])
        }
    }

    #[async_trait::async_trait]
    impl TaskDispatcher for CapturingDispatcher {
        async fn dispatch(
            &self,
            spec: &TaskExecutionSpec,
            input: Vec<Signal>,
            ctx: &CellContext,
        ) -> Result<Vec<Signal>> {
            self.calls
                .lock()
                .push((spec.clone(), input.len(), ctx.cell_id.clone()));
            Ok(vec![
                Signal::builder(Kind::AgentOutput)
                    .body(Body::text("real-provider-output"))
                    .build(),
            ])
        }
    }

    fn config() -> toml::Value {
        toml::from_str(
            r#"
plan_id = "plan-a"
plan_dir = "/work/plans/plan-a"
title = "Implement the feature"
description = "Make the runtime real"
role = "implementer"
tier = "focused"
model_hint = "test-model"
files = ["src/lib.rs"]
timeout_secs = 42
max_retries = 2
task_def_json = "{}"
"#,
        )
        .expect("valid task config")
    }

    #[tokio::test]
    async fn live_dispatch_uses_injected_runtime_and_preserves_task_metadata() {
        let dispatcher = Arc::new(CapturingDispatcher::default());
        let cell = TaskExecutorCell::live(config(), dispatcher.clone());
        let input = Signal::builder(Kind::AgentOutput)
            .body(Body::text("dependency output"))
            .build();
        let output = cell
            .execute(
                vec![input],
                &CellContext::new().with_cell_id("task-1".to_string()),
            )
            .await
            .expect("live dispatch");

        assert_eq!(
            output[0].body.as_text().expect("text"),
            "real-provider-output"
        );
        let calls = dispatcher.calls.lock();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0.plan_id, "plan-a");
        assert_eq!(calls[0].0.title, "Implement the feature");
        assert_eq!(calls[0].0.files, ["src/lib.rs"]);
        assert_eq!(calls[0].1, 1);
        assert_eq!(calls[0].2.as_deref(), Some("task-1"));
    }

    #[tokio::test]
    async fn unconfigured_live_cell_fails_instead_of_reporting_dry_run_success() {
        let error = TaskExecutorCell::unconfigured(config())
            .execute(Vec::new(), &CellContext::new())
            .await
            .expect_err("missing live runtime must fail");
        assert!(error.to_string().contains("refusing synthetic success"));
    }

    #[tokio::test]
    async fn live_dispatch_honors_the_task_retry_ceiling() {
        let dispatcher = Arc::new(FailsOnceDispatcher::default());
        let cell = TaskExecutorCell::live(config(), dispatcher.clone());
        let output = cell
            .execute(Vec::new(), &CellContext::new())
            .await
            .expect("second attempt succeeds");

        assert_eq!(dispatcher.calls.load(Ordering::SeqCst), 2);
        assert_eq!(output[0].body.as_text().expect("text"), "retry-output");
    }

    #[derive(Default)]
    struct ExhaustedDispatcher {
        calls: AtomicUsize,
    }

    #[async_trait::async_trait]
    impl TaskDispatcher for ExhaustedDispatcher {
        async fn dispatch(
            &self,
            _spec: &TaskExecutionSpec,
            _input: Vec<Signal>,
            _ctx: &CellContext,
        ) -> Result<Vec<Signal>> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Err(roko_core::error::RokoError::Gateway {
                category: "provider_exhausted",
                retryable: false,
                message: "every candidate provider is out of usage".to_string(),
            })
        }
    }

    #[tokio::test]
    async fn non_retryable_gateway_error_is_not_retried() {
        let dispatcher = Arc::new(ExhaustedDispatcher::default());
        let cell = TaskExecutorCell::live(config(), dispatcher.clone());
        let error = cell
            .execute(Vec::new(), &CellContext::new())
            .await
            .expect_err("exhausted providers fail the task");

        assert_eq!(dispatcher.calls.load(Ordering::SeqCst), 1);
        assert!(matches!(
            error,
            roko_core::error::RokoError::Gateway {
                category: "provider_exhausted",
                ..
            }
        ));
    }

    #[derive(Default)]
    struct StoppedDispatcher {
        calls: AtomicUsize,
    }

    #[async_trait::async_trait]
    impl TaskDispatcher for StoppedDispatcher {
        async fn dispatch(
            &self,
            _spec: &TaskExecutionSpec,
            _input: Vec<Signal>,
            _ctx: &CellContext,
        ) -> Result<Vec<Signal>> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Err(roko_core::error::RokoError::cancelled(
                "its plan run is stopping",
            ))
        }
    }

    /// An attempt its stopping run cancelled is not retried (bug-2b1ddc).
    #[tokio::test]
    async fn a_cancelled_dispatch_is_not_retried() {
        let dispatcher = Arc::new(StoppedDispatcher::default());
        let cell = TaskExecutorCell::live(config(), dispatcher.clone());
        let error = cell
            .execute(Vec::new(), &CellContext::new())
            .await
            .expect_err("a stopped attempt fails the task");

        assert_eq!(dispatcher.calls.load(Ordering::SeqCst), 1);
        assert!(matches!(error, roko_core::error::RokoError::Cancelled(_)));
    }

    #[tokio::test]
    async fn dry_run_is_explicit_and_uses_configured_title() {
        let output = TaskExecutorCell::dry_run(config())
            .execute(Vec::new(), &CellContext::new())
            .await
            .expect("explicit dry-run");
        assert_eq!(
            output[0].body.as_text().expect("text"),
            "task-output:dry-run:Implement the feature"
        );
    }

    // ── #247 typed dispatch request contract ────────────────────────────

    fn test_request() -> TaskDispatchRequest {
        TaskDispatchRequest {
            task_id: "T1".to_string(),
            attempt_id: "01JARH5QVXP4T3K9WNFG8M2D".to_string(),
            plan_id: "plan-a".to_string(),
            run_id: Some("run-1".to_string()),
            node_id: Some("node-1".to_string()),
            role: "implementer".to_string(),
            effort: "focused".to_string(),
            provider: Some("anthropic".to_string()),
            model: Some("claude-4".to_string()),
            input: vec![
                Signal::builder(Kind::AgentOutput)
                    .body(Body::text("upstream"))
                    .build(),
            ],
            workdir: PathBuf::from("/tmp/work"),
            capabilities: Vec::new(),
            deadline_ms: Some(1_000_000),
            budget_micro_usd: Some(500_000),
            tool_policy: None,
            spec: TaskExecutionSpec::from_config(&config()),
        }
    }

    #[tokio::test]
    async fn dispatch_request_default_delegates_to_dispatch() {
        let dispatcher = Arc::new(CapturingDispatcher::default());
        let request = test_request();
        let outcome = dispatcher
            .dispatch_request(&request, &CellContext::new())
            .await
            .expect("dispatch_request");

        assert_eq!(outcome.attempt_id, request.attempt_id);
        assert_eq!(outcome.outcome, TaskDispatchOutcomeKind::Succeeded);
        assert!(!outcome.output.is_empty());
    }

    #[tokio::test]
    async fn dispatch_stream_default_returns_outcome_without_events() {
        let dispatcher = Arc::new(CapturingDispatcher::default());
        let request = test_request();
        let (tx, mut rx) = tokio::sync::mpsc::channel::<GraphTaskEvent>(16);

        let outcome = dispatcher
            .dispatch_stream(&request, &CellContext::new(), tx)
            .await
            .expect("dispatch_stream");

        assert_eq!(outcome.outcome, TaskDispatchOutcomeKind::Succeeded);
        // Default impl sends no events.
        assert!(rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn reconcile_attempt_default_returns_allocate_new() {
        let dispatcher = Arc::new(CapturingDispatcher::default());
        let result = dispatcher
            .reconcile_attempt("attempt-1", "fingerprint-1")
            .await;
        match result {
            AttemptReconciliation::AllocateNew { attempt_id } => {
                assert!(
                    attempt_id.contains("attempt-1"),
                    "new attempt ID must reference the original: {attempt_id}"
                );
            }
            other => panic!("expected AllocateNew for unimplemented reconcile, got {other:?}"),
        }
    }

    // ── Provider attempt receipt ────────────────────────────────────────

    #[test]
    fn provider_attempt_receipt_roundtrip() {
        let receipt = ProviderAttemptReceipt {
            attempt_id: "01JARH5QVXP4T3K9WNFG8M2D".to_string(),
            request_fingerprint: "blake3-abc".to_string(),
            provider_request_id: Some("req-xyz".to_string()),
            started_at_ms: 1_000_000,
            completed_at_ms: 1_005_000,
            status: TaskDispatchOutcomeKind::Succeeded,
            input_tokens: 1500,
            output_tokens: 800,
            cost_micro_usd: 4200,
            error: None,
        };
        let json = serde_json::to_string(&receipt).expect("serialize");
        let deser: ProviderAttemptReceipt = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(receipt, deser);
    }

    #[test]
    fn provider_attempt_receipt_uses_u64_cost() {
        let receipt = ProviderAttemptReceipt {
            attempt_id: "a".to_string(),
            request_fingerprint: "f".to_string(),
            provider_request_id: None,
            started_at_ms: 0,
            completed_at_ms: 1,
            status: TaskDispatchOutcomeKind::Failed,
            input_tokens: 0,
            output_tokens: 0,
            cost_micro_usd: 1_000_000,
            error: Some("budget".to_string()),
        };
        // Verify cost is integer arithmetic.
        assert_eq!(receipt.cost_micro_usd, 1_000_000_u64);
    }

    // ── Progress variant uses completed/total ───────────────────────────

    #[test]
    fn progress_event_uses_completed_total() {
        let event = GraphTaskEvent::Progress {
            message: "step 3 of 5".to_string(),
            completed: Some(3),
            total: Some(5),
        };
        match event {
            GraphTaskEvent::Progress {
                completed, total, ..
            } => {
                assert_eq!(completed, Some(3));
                assert_eq!(total, Some(5));
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn progress_serde_roundtrip() {
        let event = GraphTaskEvent::Progress {
            message: "processing".to_string(),
            completed: None,
            total: None,
        };
        let json = serde_json::to_string(&event).expect("serialize");
        let deser: GraphTaskEvent = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(event, deser);
    }

    // ── Type aliases ────────────────────────────────────────────────────

    #[test]
    fn type_aliases_are_identical() {
        // Compile-time proof that the aliases resolve correctly.
        let _event: TaskDispatchEvent = GraphTaskEvent::Text {
            text: "t".to_string(),
        };
        let _status: TaskDispatchStatus = TaskDispatchOutcomeKind::Succeeded;
        let _decision: AttemptReconcileDecision = AttemptReconciliation::AllocateNew {
            attempt_id: "a".to_string(),
        };
    }

    // ── Truncation helper ───────────────────────────────────────────────

    #[test]
    fn truncate_utf8_within_limit() {
        assert_eq!(truncate_utf8("hello", 10), "hello");
    }

    #[test]
    fn truncate_utf8_at_exact_boundary() {
        assert_eq!(truncate_utf8("hello", 5), "hello");
    }

    #[test]
    fn truncate_utf8_mid_ascii() {
        assert_eq!(truncate_utf8("hello world", 5), "hello");
    }

    #[test]
    fn truncate_utf8_respects_char_boundary() {
        // 'é' is 2 bytes in UTF-8.
        let s = "café";
        assert_eq!(s.len(), 5); // c=1 a=1 f=1 é=2
        // Cutting at 4 would split 'é'; should back up to 3.
        assert_eq!(truncate_utf8(s, 4), "caf");
        // Cutting at 5 keeps the whole string.
        assert_eq!(truncate_utf8(s, 5), s);
    }

    // ── Gate verdict tag ────────────────────────────────────────────────

    #[test]
    fn gate_verdict_stamp_round_trips_and_refreshes_ids() {
        let mut signals = vec![
            Signal::builder(Kind::AgentOutput)
                .body(Body::text("done"))
                .build(),
        ];
        let before = signals[0].id;
        TaskGateVerdict::Passed.stamp(&mut signals);
        assert_eq!(
            signals[0].tag(TASK_GATE_VERDICT_TAG),
            Some(TaskGateVerdict::Passed.as_str())
        );
        assert_ne!(signals[0].id, before);
        assert_eq!(signals[0].id, signals[0].content_hash());
        assert_eq!(
            TaskGateVerdict::from_signals(&signals),
            Some(TaskGateVerdict::Passed)
        );
    }

    #[test]
    fn gate_verdict_least_trustworthy_signal_wins() {
        let mut passed = vec![Signal::builder(Kind::AgentOutput).build()];
        TaskGateVerdict::Passed.stamp(&mut passed);
        let mut forced = vec![
            Signal::builder(Kind::AgentOutput)
                .body(Body::text("accepted"))
                .build(),
        ];
        TaskGateVerdict::ForcedAccept.stamp(&mut forced);
        let mixed: Vec<Signal> = passed.into_iter().chain(forced).collect();
        let verdict = TaskGateVerdict::from_signals(&mixed).expect("verdict");
        assert_eq!(verdict, TaskGateVerdict::ForcedAccept);
        assert!(!verdict.is_replayable());
    }

    #[test]
    fn gate_verdict_absent_or_unknown_is_none() {
        let untagged = vec![Signal::builder(Kind::AgentOutput).build()];
        assert_eq!(TaskGateVerdict::from_signals(&untagged), None);
        let unknown = vec![
            Signal::builder(Kind::AgentOutput)
                .tag(TASK_GATE_VERDICT_TAG, "maybe")
                .build(),
        ];
        assert_eq!(TaskGateVerdict::from_signals(&unknown), None);
        assert!(TaskGateVerdict::Passed.is_replayable());
        assert!(TaskGateVerdict::Unverified.is_replayable());
    }

    #[test]
    fn already_satisfied_is_replayable_and_ranks_below_passed() {
        let verdict = TaskGateVerdict::AlreadySatisfied;
        assert_eq!(TaskGateVerdict::parse(verdict.as_str()), Some(verdict));
        assert!(verdict.is_replayable());

        let mut passed = vec![Signal::builder(Kind::AgentOutput).build()];
        TaskGateVerdict::Passed.stamp(&mut passed);
        let mut satisfied = vec![
            Signal::builder(Kind::AgentOutput)
                .body(Body::text("already done"))
                .build(),
        ];
        verdict.stamp(&mut satisfied);
        let mixed: Vec<Signal> = passed.into_iter().chain(satisfied).collect();
        assert_eq!(TaskGateVerdict::from_signals(&mixed), Some(verdict));
    }

    /// gap-161be1: a pass over pre-existing failures replays like a pass but
    /// never hides behind a clean one.
    #[test]
    fn passed_with_preexisting_failures_is_replayable_and_ranks_below_passed() {
        let verdict = TaskGateVerdict::PassedWithPreexistingFailures;
        assert_eq!(verdict.as_str(), "passed_with_preexisting_failures");
        assert_eq!(TaskGateVerdict::parse(verdict.as_str()), Some(verdict));
        assert!(verdict.is_replayable());

        let mut passed = vec![Signal::builder(Kind::AgentOutput).build()];
        TaskGateVerdict::Passed.stamp(&mut passed);
        let mut filtered = vec![
            Signal::builder(Kind::AgentOutput)
                .body(Body::text("old failures only"))
                .build(),
        ];
        verdict.stamp(&mut filtered);
        let mixed: Vec<Signal> = passed.into_iter().chain(filtered).collect();
        assert_eq!(TaskGateVerdict::from_signals(&mixed), Some(verdict));
    }

    fn handed_on_attempt() -> TaskAttempt {
        let attempt_id = crate::workspace::WorkspaceAttemptId {
            plan_id: "plan-a".to_string(),
            task_id: "T1".to_string(),
            attempt: 0,
        };
        TaskAttempt {
            plan_id: "plan-a".to_string(),
            task_id: "T1".to_string(),
            run_id: Some("run-7".to_string()),
            attempt_key: Some("run-7:plan-a:T1:2".to_string()),
            attempt: 2,
            workspace: Some(PathBuf::from("/wt/attempt-1")),
            lease: Some(WorkspaceLease {
                lease_id: "attempt-1".to_string(),
                lease_fingerprint: attempt_id.fingerprint(),
                attempt_id,
                path: PathBuf::from("/wt/attempt-1"),
                branch: "roko/attempt/attempt-1".to_string(),
                base_revision: "HEAD".to_string(),
            }),
            accepted: None,
        }
    }

    /// gap-3b5361: an accepted attempt names where its work landed, and a
    /// cell that settled a handed-on lease stamps it away.
    #[test]
    fn task_attempt_records_acceptance_and_clears_a_settled_lease() {
        let handed_on = handed_on_attempt();
        let mut signals = vec![Signal::builder(Kind::AgentOutput).build()];
        handed_on.stamp(&mut signals);

        let settled = TaskAttempt {
            lease: None,
            accepted: Some(WorkspaceAcceptance {
                attempt_commit: "a".repeat(40),
                plan_branch: "roko/plan/plan-a".to_string(),
                accepted_commit: "b".repeat(40),
            }),
            ..handed_on
        };
        settled.stamp(&mut signals);
        assert_eq!(signals[0].tag("workspace.lease"), None);
        assert_eq!(TaskAttempt::from_signals(&signals), Ok(settled));
    }

    #[derive(Default)]
    struct RejectsDispatcher {
        calls: AtomicUsize,
    }

    #[async_trait::async_trait]
    impl TaskDispatcher for RejectsDispatcher {
        async fn dispatch(
            &self,
            _spec: &TaskExecutionSpec,
            _input: Vec<Signal>,
            _ctx: &CellContext,
        ) -> Result<Vec<Signal>> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Err(roko_core::error::RokoError::Rejected(
                "the plan branch refused the work".to_string(),
            ))
        }
    }

    /// gap-3b5361: a gate's rejection fails the same way on a retry, so it
    /// is surfaced at once.
    #[tokio::test]
    async fn a_rejection_is_not_retried() {
        let dispatcher = Arc::new(RejectsDispatcher::default());
        let cell = TaskExecutorCell::live(config(), dispatcher.clone());
        let error = cell
            .execute(Vec::new(), &CellContext::new())
            .await
            .expect_err("the rejection fails the task");
        assert!(matches!(error, roko_core::error::RokoError::Rejected(_)));
        assert_eq!(dispatcher.calls.load(Ordering::SeqCst), 1);
    }

    /// bug-50caf2: the attempt, its checkout and its lease survive the trip
    /// through the output tags, and stamping refreshes the content ids.
    #[test]
    fn task_attempt_stamp_round_trips_and_refreshes_ids() {
        let attempt = handed_on_attempt();
        let mut signals = vec![
            Signal::builder(Kind::AgentOutput)
                .body(Body::text("done"))
                .build(),
        ];
        let before = signals[0].id;
        attempt.stamp(&mut signals);
        assert_ne!(signals[0].id, before);
        assert_eq!(signals[0].id, signals[0].content_hash());
        assert_eq!(signals[0].tag("task_id"), Some("T1"));
        assert_eq!(TaskAttempt::from_signals(&signals), Ok(attempt));
    }

    /// An output that names no attempt, or no usable one, is refused with a
    /// reason rather than read as some default attempt.
    #[test]
    fn task_attempt_refuses_missing_or_malformed_tags() {
        let untagged = vec![Signal::builder(Kind::AgentOutput).build()];
        let error = TaskAttempt::from_signals(&untagged).unwrap_err();
        assert!(error.contains("workspace.attempt"), "{error}");

        for (tag, value) in [
            ("workspace.attempt", "0"),
            ("workspace.attempt", "two"),
            ("workspace.lease", "{not a lease"),
        ] {
            let mut signal = Signal::builder(Kind::AgentOutput).build();
            signal
                .tags
                .insert("workspace.attempt".to_string(), "1".to_string());
            signal.tags.insert(tag.to_string(), value.to_string());
            let error = TaskAttempt::from_signals(&[signal]).unwrap_err();
            assert!(error.contains(tag), "{tag}={value}: {error}");
        }
    }

    #[test]
    fn keep_workspace_is_read_from_the_node_config() {
        assert!(!TaskExecutionSpec::from_config(&config()).keep_workspace);
        let mut table = config().as_table().cloned().expect("table");
        table.insert("keep_workspace".to_string(), toml::Value::Boolean(true));
        let spec = TaskExecutionSpec::from_config(&toml::Value::Table(table));
        assert!(spec.keep_workspace);
    }
}
