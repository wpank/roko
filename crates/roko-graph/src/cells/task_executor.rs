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
        }
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
    Progress { fraction: f64, message: String },
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
                        Err(error) if retry < self.spec.max_retries => {
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
}
