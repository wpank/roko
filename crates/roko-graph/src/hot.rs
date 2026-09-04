//! Hot Graph execution -- tick-driven, resident Graph instances.
//!
//! A Hot Graph is a Graph with `policy.hot` set. The Engine runs it in a
//! loop, executing all nodes once per tick, persisting outputs between ticks
//! so each tick starts from the previous tick's state.
//!
//! Hot Graphs run until:
//!   1. `HotPolicy.max_ticks` is reached, OR
//!   2. The `HotGraphHandle.cancel()` token is triggered, OR
//!   3. A node returns an unrecoverable error (non-retriable failure)
//!      and the graph policy is `FailFast`.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};

use crate::budget::{BudgetCheckpoint, BudgetEnforcer};
use crate::cell::CellContext;
use crate::engine::{GraphEngine, GraphOutput};
use crate::fingerprint::graph_execution_fingerprint;
use crate::registry::CellRegistry;
use crate::replay::{ActivityRecorder, ActivityReplayer};
use crate::types::{ExecutionClass, Graph};

const HOT_CHECKPOINT_SCHEMA_VERSION: u32 = 1;
const HOT_CHECKPOINT_MANIFEST: &str = "checkpoint.json";
const HOT_ACTIVITY_LOG: &str = "activities.jsonl";

/// Named loop levels for nested hot graph timing, per v2 spec (04-EXECUTION.md).
///
/// When a `HotPolicy` has `loop_level` set, the level's default interval is used
/// in place of `tick_interval_ms`. This allows graphs to declare their temporal
/// role (perception, planning, consolidation) without hard-coding millisecond values.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum LoopLevel {
    /// Fast: perception, reflex. Default 250 ms.
    Gamma,
    /// Medium: planning, deliberation. Default 10 000 ms.
    Theta,
    /// Slow: learning, consolidation. Default 60 000 ms.
    Delta,
}

impl LoopLevel {
    /// Return the spec-default tick interval in milliseconds for this loop level.
    pub fn default_interval_ms(self) -> u64 {
        match self {
            Self::Gamma => 250,
            Self::Theta => 10_000,
            Self::Delta => 60_000,
        }
    }
}

/// Policy controlling Hot Graph tick behavior.
///
/// Parsed from `[graph.policy.hot]` in TOML graph definitions.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HotPolicy {
    /// How long to wait between ticks (ms). 0 = run as fast as possible.
    ///
    /// Overridden by `loop_level` when that field is set.
    #[serde(default)]
    pub tick_interval_ms: u64,
    /// Stop after this many ticks. None = run until cancelled.
    #[serde(default)]
    pub max_ticks: Option<u64>,
    /// If true, persist cell output state between ticks so cells can
    /// resume from their previous output.
    #[serde(default)]
    pub persist_tick_state: bool,
    /// Named loop level. When set, the level's default interval overrides
    /// `tick_interval_ms`.
    #[serde(default)]
    pub loop_level: Option<LoopLevel>,
}

impl HotPolicy {
    /// Return the effective tick interval in milliseconds.
    ///
    /// If `loop_level` is set, returns that level's spec-default interval.
    /// Otherwise falls back to the raw `tick_interval_ms` value.
    pub fn resolve_tick_interval_ms(&self) -> u64 {
        match self.loop_level {
            Some(level) => level.default_interval_ms(),
            None => self.tick_interval_ms,
        }
    }
}

impl Default for HotPolicy {
    fn default() -> Self {
        Self {
            tick_interval_ms: 1000,
            max_ticks: None,
            persist_tick_state: false,
            loop_level: None,
        }
    }
}

/// Location and recovery behavior for a durable Hot Graph checkpoint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HotCheckpointOptions {
    /// Directory containing `checkpoint.json` and `activities.jsonl`.
    pub directory: PathBuf,
    /// Archive any current checkpoint and begin a new run.
    pub fresh: bool,
    /// Archive an invalid or drifted checkpoint and begin a new run.
    pub force_resume: bool,
}

impl HotCheckpointOptions {
    /// Create checkpoint options that resume compatible state when present.
    #[must_use]
    pub fn new(directory: impl Into<PathBuf>) -> Self {
        Self {
            directory: directory.into(),
            fresh: false,
            force_resume: false,
        }
    }
}

/// Versioned durable state committed after each complete Hot Graph tick.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HotGraphCheckpointManifest {
    /// On-disk schema version.
    pub schema_version: u32,
    /// Graph identifier stored in every Activity record.
    pub graph_id: String,
    /// Stable identity of the exact Graph definition and Hot policy.
    pub graph_fingerprint: String,
    /// Run identity shared by the manifest and Activity log.
    pub run_id: String,
    /// Activity log filename relative to the checkpoint directory.
    pub activity_log: String,
    /// First tick that has not been committed yet.
    pub next_tick: u64,
    /// Last complete output set used as stateful root input on the next tick.
    #[serde(default)]
    pub tick_state: BTreeMap<String, Vec<roko_core::Signal>>,
    /// Cumulative budget state, when this loop has an enforcer.
    #[serde(default)]
    pub budget: Option<BudgetCheckpoint>,
    /// Last manifest commit as Unix milliseconds.
    pub updated_at_ms: u128,
}

/// Error returned while preparing or committing a Hot Graph checkpoint.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{message}")]
pub struct HotCheckpointError {
    message: String,
}

impl HotCheckpointError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

/// Terminal failure from a background Hot Graph execution.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{message}")]
pub struct HotGraphFailure {
    /// Human-readable execution or persistence failure.
    pub message: String,
}

/// A running Hot Graph instance.
///
/// Returned by [`start_hot`]. Callers use this handle to observe tick progress,
/// cancel the loop, and wait for completion.
pub struct HotGraphHandle {
    /// Cancellation token -- call `.cancel()` to stop the tick loop.
    cancel: CancellationToken,
    /// Monotonic tick counter (incremented after each completed tick).
    tick: Arc<AtomicU64>,
    /// Most recent graph output (from the last completed tick).
    last_output: Arc<parking_lot::Mutex<Option<GraphOutput>>>,
    /// Background task handle (taken by `wait`).
    join_handle: parking_lot::Mutex<Option<JoinHandle<()>>>,
    /// Optional per-loop budget enforcement.
    ///
    /// When set, the tick loop checks budget before each tick, skips expensive
    /// nodes when exhausted, and passes `budget_remaining` to each `CellContext`.
    budget: Option<Arc<BudgetEnforcer>>,
    /// Terminal execution or checkpoint failure, if the loop stopped abnormally.
    failure: Arc<parking_lot::Mutex<Option<HotGraphFailure>>>,
}

impl HotGraphHandle {
    /// Request cancellation of the Hot Graph tick loop.
    pub fn cancel(&self) {
        self.cancel.cancel();
    }

    /// Return the number of completed ticks.
    pub fn tick_count(&self) -> u64 {
        self.tick.load(Ordering::Relaxed)
    }

    /// Return a clone of the most recent graph output, if any tick has completed.
    pub fn last_output(&self) -> Option<GraphOutput> {
        self.last_output.lock().clone()
    }

    /// Wait for the Hot Graph to finish (either max_ticks reached or cancelled).
    ///
    /// Can be called multiple times; only the first call awaits the background
    /// task. Subsequent calls return immediately.
    pub async fn wait(&self) {
        let _ = self.wait_result().await;
    }

    /// Wait for completion and surface execution, persistence, or task failures.
    pub async fn wait_result(&self) -> Result<(), HotGraphFailure> {
        let handle = self.join_handle.lock().take();
        if let Some(handle) = handle
            && let Err(error) = handle.await
        {
            let mut failure = self.failure.lock();
            if failure.is_none() {
                *failure = Some(HotGraphFailure {
                    message: format!("Hot Graph task failed: {error}"),
                });
            }
        }
        self.failure().map_or(Ok(()), Err)
    }

    /// Check if the background task is still running.
    pub fn is_running(&self) -> bool {
        let guard = self.join_handle.lock();
        match &*guard {
            Some(h) => !h.is_finished(),
            None => false,
        }
    }

    /// Return a reference to the budget enforcer, if one was provided.
    pub fn budget(&self) -> Option<&BudgetEnforcer> {
        self.budget.as_deref()
    }

    /// Return the terminal failure, if the loop stopped abnormally.
    #[must_use]
    pub fn failure(&self) -> Option<HotGraphFailure> {
        self.failure.lock().clone()
    }
}

/// Start a Hot Graph -- a tick-driven, resident graph execution loop.
///
/// The graph is executed repeatedly according to the `HotPolicy`:
/// - Each tick runs the full graph once using its configured concurrency policy.
/// - Between ticks, the loop waits for `tick_interval_ms` (or checks cancellation).
/// - After `max_ticks` (if set), the loop exits.
///
/// Returns a `HotGraphHandle` immediately; execution continues on a background task.
///
/// # Errors
///
/// Returns an error if the graph has no `hot` policy in its metadata labels.
pub fn start_hot(
    graph: Graph,
    registry: CellRegistry,
    policy: HotPolicy,
    parent_cancel: Option<CancellationToken>,
) -> HotGraphHandle {
    start_hot_with_budget(graph, registry, policy, parent_cancel, None)
}

/// Start a Hot Graph with optional per-loop budget enforcement.
///
/// Identical to [`start_hot`] but accepts an optional [`BudgetEnforcer`]. When
/// provided, each tick's [`CellContext::budget_remaining`] is populated from
/// the enforcer, and the enforcer is available via [`HotGraphHandle::budget`].
#[allow(clippy::too_many_lines)]
pub fn start_hot_with_budget(
    mut graph: Graph,
    registry: CellRegistry,
    policy: HotPolicy,
    parent_cancel: Option<CancellationToken>,
    budget: Option<BudgetEnforcer>,
) -> HotGraphHandle {
    configure_hot_graph(&mut graph, &policy);
    start_hot_engine(graph, registry, policy, parent_cancel, budget, None)
}

/// Start or resume a crash-recoverable Hot Graph.
///
/// A successful tick commits its next tick number, retained outputs, cumulative
/// budget, and flushed Activity records. If a process stops during a tick, a
/// restart replays completed Activities from that interrupted tick and executes
/// only the missing work.
///
/// # Errors
/// Returns an error for corrupt, incomplete, or graph-drifted checkpoint state.
pub fn start_hot_resumable(
    graph: Graph,
    registry: CellRegistry,
    policy: HotPolicy,
    parent_cancel: Option<CancellationToken>,
    checkpoint: HotCheckpointOptions,
) -> Result<HotGraphHandle, HotCheckpointError> {
    start_hot_resumable_with_budget(graph, registry, policy, parent_cancel, None, checkpoint)
}

/// Start or resume a crash-recoverable Hot Graph with cumulative budget state.
///
/// # Errors
/// Returns an error for corrupt, incomplete, drifted, or budget-incompatible
/// checkpoint state.
pub fn start_hot_resumable_with_budget(
    mut graph: Graph,
    registry: CellRegistry,
    policy: HotPolicy,
    parent_cancel: Option<CancellationToken>,
    mut budget: Option<BudgetEnforcer>,
    checkpoint: HotCheckpointOptions,
) -> Result<HotGraphHandle, HotCheckpointError> {
    configure_hot_graph(&mut graph, &policy);
    let prepared = prepare_hot_checkpoint(&graph, &checkpoint, &mut budget)?;
    Ok(start_hot_engine(
        graph,
        registry,
        policy,
        parent_cancel,
        budget,
        Some(prepared),
    ))
}

fn configure_hot_graph(graph: &mut Graph, policy: &HotPolicy) {
    graph.policy.mode = crate::types::GraphMode::Hot;
    graph.policy.hot = Some(policy.clone());
}

#[allow(clippy::too_many_lines)]
fn start_hot_engine(
    graph: Graph,
    registry: CellRegistry,
    policy: HotPolicy,
    parent_cancel: Option<CancellationToken>,
    budget: Option<BudgetEnforcer>,
    checkpoint: Option<PreparedHotCheckpoint>,
) -> HotGraphHandle {
    let cancel = parent_cancel.map(|p| p.child_token()).unwrap_or_default();
    let initial_tick = checkpoint
        .as_ref()
        .map_or(0, |state| state.manifest.next_tick);
    let tick = Arc::new(AtomicU64::new(initial_tick));
    let last_output: Arc<parking_lot::Mutex<Option<GraphOutput>>> =
        Arc::new(parking_lot::Mutex::new(None));
    let budget_arc = budget.map(Arc::new);
    let failure = Arc::new(parking_lot::Mutex::new(None));
    let parallel_execution = graph.policy.max_concurrent_nodes > 1;
    let graph_name = graph.metadata.name.clone();

    let mut engine = GraphEngine::new(graph, registry);

    // Validate edge type compatibility before spawning work.
    if let Err(error) = engine.validate_for_start() {
        *failure.lock() = Some(HotGraphFailure {
            message: format!("Hot Graph edge validation failed: {error}"),
        });
    }

    let mut checkpoint_state = checkpoint;
    if let Some(state) = checkpoint_state.as_mut() {
        let restored: HashMap<_, _> = std::mem::take(&mut state.manifest.tick_state)
            .into_iter()
            .collect();
        if let Err(error) = engine.restore_tick_state(restored) {
            *failure.lock() = Some(HotGraphFailure {
                message: format!("restore Hot Graph tick state: {error}"),
            });
        }
        engine = engine.with_recorder(
            state
                .recorder
                .take()
                .expect("prepared checkpoint recorder is present"),
        );
        if let Some(replayer) = state.replayer.take() {
            engine = engine.with_replayer(replayer);
        }
    }

    let cancel_clone = cancel.clone();
    let tick_clone = tick.clone();
    let output_clone = last_output.clone();
    let budget_clone = budget_arc.clone();
    let failure_clone = failure.clone();

    let join_handle = tokio::spawn(async move {
        let mut current_tick = initial_tick;
        let mut budget_warned = false;

        if failure_clone.lock().is_some() {
            return;
        }

        let tick_interval_ms = policy.resolve_tick_interval_ms();
        info!(
            graph = %graph_name,
            max_ticks = ?policy.max_ticks,
            tick_interval_ms,
            loop_level = ?policy.loop_level,
            has_budget = budget_clone.is_some(),
            "hot graph started"
        );

        loop {
            // Check cancellation before each tick.
            if cancel_clone.is_cancelled() {
                info!(graph = %graph_name, ticks = current_tick, "hot graph cancelled");
                break;
            }

            // Check max_ticks limit.
            if let Some(max) = policy.max_ticks
                && current_tick >= max
            {
                info!(
                    graph = %graph_name,
                    ticks = current_tick,
                    "hot graph reached max_ticks"
                );
                break;
            }

            // Build per-tick CellContext with budget info when available.
            let mut ctx = if let Some(ref enforcer) = budget_clone {
                let remaining = enforcer.remaining_cost_usd();

                // Log once when budget first becomes exhausted.
                if enforcer.is_exhausted() && !budget_warned {
                    budget_warned = true;
                    warn!(
                        graph = %graph_name,
                        tick = current_tick,
                        cost_usd = enforcer.cost_usd(),
                        tokens_used = enforcer.tokens_used(),
                        "budget exhausted -- skipping expensive nodes"
                    );
                }

                let mut c = CellContext::new();
                if let Some(r) = remaining {
                    c = c.with_budget(r);
                }
                c
            } else {
                CellContext::new()
            };
            if let Some(state) = checkpoint_state.as_ref() {
                ctx = ctx.with_run_id(state.manifest.run_id.clone());
            }

            // Execute one tick of the graph.
            let execution = if parallel_execution {
                engine.execute_parallel_at_tick(&ctx, current_tick).await
            } else {
                engine.execute_at_tick(&ctx, current_tick).await
            };
            match execution {
                Ok(output) => {
                    info!(
                        graph = %graph_name,
                        tick = current_tick,
                        success = output.success,
                        nodes = output.node_results.len(),
                        "hot graph tick complete"
                    );
                    *output_clone.lock() = Some(output);
                }
                Err(e) => {
                    error!(
                        graph = %graph_name,
                        tick = current_tick,
                        error = %e,
                        "hot graph tick failed"
                    );
                    *failure_clone.lock() = Some(HotGraphFailure {
                        message: format!("Hot Graph tick {current_tick} failed: {e}"),
                    });
                    // On error, break the loop (conservative: FailFast for hot graphs).
                    break;
                }
            }

            let next_tick = current_tick.saturating_add(1);
            if let Some(state) = checkpoint_state.as_mut() {
                state.manifest.next_tick = next_tick;
                state.manifest.tick_state = engine.tick_state_snapshot().into_iter().collect();
                state.manifest.budget = budget_clone.as_ref().map(|budget| budget.checkpoint());
                state.manifest.updated_at_ms = unix_ms();
                if let Err(error) = write_manifest_atomic(&state.manifest_path, &state.manifest) {
                    error!(
                        graph = %graph_name,
                        tick = current_tick,
                        error = %error,
                        "commit Hot Graph checkpoint failed"
                    );
                    *failure_clone.lock() = Some(HotGraphFailure {
                        message: format!(
                            "commit Hot Graph checkpoint after tick {current_tick}: {error}"
                        ),
                    });
                    break;
                }
            }

            current_tick = next_tick;
            tick_clone.store(current_tick, Ordering::Relaxed);

            // Wait for next tick interval, respecting cancellation.
            if tick_interval_ms > 0 {
                let sleep_dur = Duration::from_millis(tick_interval_ms);
                tokio::select! {
                    () = tokio::time::sleep(sleep_dur) => {}
                    () = cancel_clone.cancelled() => {
                        info!(graph = %graph_name, ticks = current_tick, "hot graph cancelled during sleep");
                        break;
                    }
                }
            } else {
                // Yield to let cancellation propagate even at tick_interval_ms = 0.
                tokio::task::yield_now().await;
                if cancel_clone.is_cancelled() {
                    break;
                }
            }
        }

        info!(
            graph = %graph_name,
            total_ticks = current_tick,
            "hot graph stopped"
        );
    });

    HotGraphHandle {
        cancel,
        tick,
        last_output,
        join_handle: parking_lot::Mutex::new(Some(join_handle)),
        budget: budget_arc,
        failure,
    }
}

struct PreparedHotCheckpoint {
    manifest_path: PathBuf,
    manifest: HotGraphCheckpointManifest,
    recorder: Option<ActivityRecorder>,
    replayer: Option<ActivityReplayer>,
}

fn prepare_hot_checkpoint(
    graph: &Graph,
    options: &HotCheckpointOptions,
    budget: &mut Option<BudgetEnforcer>,
) -> Result<PreparedHotCheckpoint, HotCheckpointError> {
    let manifest_path = options.directory.join(HOT_CHECKPOINT_MANIFEST);
    let activities_path = options.directory.join(HOT_ACTIVITY_LOG);
    let fingerprint = graph_execution_fingerprint(graph)
        .map_err(|error| HotCheckpointError::new(format!("fingerprint Hot Graph: {error}")))?;

    if options.fresh {
        archive_checkpoint_files(&manifest_path, &activities_path)?;
    }

    if !options.fresh && manifest_path.exists() {
        let resume = load_hot_checkpoint(
            graph,
            &manifest_path,
            &activities_path,
            &fingerprint,
            budget,
        );
        match resume {
            Ok(prepared) => return Ok(prepared),
            Err(error) if options.force_resume => {
                warn!(error = %error, "archiving invalid Hot Graph checkpoint");
                archive_checkpoint_files(&manifest_path, &activities_path)?;
            }
            Err(error) => return Err(error),
        }
    } else if !options.fresh && activities_path.exists() {
        if !options.force_resume {
            return Err(HotCheckpointError::new(format!(
                "found Hot Graph Activity log {} without its manifest; use a fresh or forced resume",
                activities_path.display()
            )));
        }
        archive_checkpoint_files(&manifest_path, &activities_path)?;
    }

    create_hot_checkpoint(
        graph,
        &manifest_path,
        &activities_path,
        fingerprint,
        budget.as_ref(),
    )
}

fn load_hot_checkpoint(
    graph: &Graph,
    manifest_path: &Path,
    activities_path: &Path,
    fingerprint: &str,
    budget: &mut Option<BudgetEnforcer>,
) -> Result<PreparedHotCheckpoint, HotCheckpointError> {
    let bytes = std::fs::read(manifest_path).map_err(|error| {
        HotCheckpointError::new(format!(
            "read Hot Graph checkpoint {}: {error}",
            manifest_path.display()
        ))
    })?;
    let mut manifest: HotGraphCheckpointManifest =
        serde_json::from_slice(&bytes).map_err(|error| {
            HotCheckpointError::new(format!(
                "parse Hot Graph checkpoint {}: {error}",
                manifest_path.display()
            ))
        })?;
    validate_hot_manifest(
        graph,
        &manifest,
        activities_path,
        fingerprint,
        budget.is_some(),
    )?;
    if !activities_path.is_file() {
        return Err(HotCheckpointError::new(format!(
            "Hot Graph checkpoint references missing Activity log {}",
            activities_path.display()
        )));
    }

    let replayer =
        ActivityReplayer::load_scoped(activities_path, &manifest.graph_id, &manifest.run_id)
            .map_err(|error| {
                HotCheckpointError::new(format!(
                    "load Hot Graph Activity checkpoint {}: {error}",
                    activities_path.display()
                ))
            })?;
    let activity_nodes: HashSet<_> = graph
        .inner
        .node_weights()
        .filter(|node| node.execution_class == ExecutionClass::Activity)
        .map(|node| node.id.clone())
        .collect();
    replayer
        .validate_checkpoint(&activity_nodes, manifest.next_tick)
        .map_err(|error| {
            HotCheckpointError::new(format!("validate Activity checkpoint: {error}"))
        })?;

    if let (Some(enforcer), Some(checkpoint)) = (budget.as_mut(), manifest.budget.as_ref()) {
        enforcer.restore_checkpoint(checkpoint);
    }
    let recorder =
        ActivityRecorder::create(&manifest.run_id, activities_path).map_err(|error| {
            HotCheckpointError::new(format!(
                "open Hot Graph Activity checkpoint {}: {error}",
                activities_path.display()
            ))
        })?;
    manifest.updated_at_ms = unix_ms();
    Ok(PreparedHotCheckpoint {
        manifest_path: manifest_path.to_path_buf(),
        manifest,
        recorder: Some(recorder),
        replayer: Some(replayer),
    })
}

fn validate_hot_manifest(
    graph: &Graph,
    manifest: &HotGraphCheckpointManifest,
    activities_path: &Path,
    fingerprint: &str,
    has_budget: bool,
) -> Result<(), HotCheckpointError> {
    if manifest.schema_version != HOT_CHECKPOINT_SCHEMA_VERSION {
        return Err(HotCheckpointError::new(format!(
            "Hot Graph checkpoint schema {} is unsupported (expected {HOT_CHECKPOINT_SCHEMA_VERSION})",
            manifest.schema_version
        )));
    }
    if manifest.graph_id != graph.metadata.name {
        return Err(HotCheckpointError::new(format!(
            "Hot Graph checkpoint is for graph '{}' rather than '{}'",
            manifest.graph_id, graph.metadata.name
        )));
    }
    if manifest.graph_fingerprint != fingerprint {
        return Err(HotCheckpointError::new(
            "Hot Graph definition or policy has changed",
        ));
    }
    if activities_path.file_name().and_then(|name| name.to_str())
        != Some(manifest.activity_log.as_str())
    {
        return Err(HotCheckpointError::new(
            "Hot Graph checkpoint references a different Activity log",
        ));
    }
    if manifest.budget.is_some() != has_budget {
        return Err(HotCheckpointError::new(
            "Hot Graph checkpoint budget state does not match the resumed loop",
        ));
    }
    if !graph
        .policy
        .hot
        .as_ref()
        .is_some_and(|policy| policy.persist_tick_state)
        && !manifest.tick_state.is_empty()
    {
        return Err(HotCheckpointError::new(
            "stateless Hot Graph checkpoint contains retained tick state",
        ));
    }
    if let Some(node_id) = manifest
        .tick_state
        .keys()
        .find(|node_id| !graph.node_map.contains_key(*node_id))
    {
        return Err(HotCheckpointError::new(format!(
            "Hot Graph checkpoint contains unknown node state '{node_id}'"
        )));
    }
    Ok(())
}

fn create_hot_checkpoint(
    graph: &Graph,
    manifest_path: &Path,
    activities_path: &Path,
    fingerprint: String,
    budget: Option<&BudgetEnforcer>,
) -> Result<PreparedHotCheckpoint, HotCheckpointError> {
    let directory = manifest_path.parent().ok_or_else(|| {
        HotCheckpointError::new("Hot Graph checkpoint manifest has no parent directory")
    })?;
    std::fs::create_dir_all(directory).map_err(|error| {
        HotCheckpointError::new(format!(
            "create Hot Graph checkpoint directory {}: {error}",
            directory.display()
        ))
    })?;
    let run_id = format!("hot-{}-{}", graph.metadata.name, uuid::Uuid::new_v4());
    let recorder = ActivityRecorder::create_fresh(&run_id, activities_path).map_err(|error| {
        HotCheckpointError::new(format!(
            "create Hot Graph Activity log {}: {error}",
            activities_path.display()
        ))
    })?;
    let manifest = HotGraphCheckpointManifest {
        schema_version: HOT_CHECKPOINT_SCHEMA_VERSION,
        graph_id: graph.metadata.name.clone(),
        graph_fingerprint: fingerprint,
        run_id,
        activity_log: HOT_ACTIVITY_LOG.to_string(),
        next_tick: 0,
        tick_state: BTreeMap::new(),
        budget: budget.map(BudgetEnforcer::checkpoint),
        updated_at_ms: unix_ms(),
    };
    write_manifest_atomic(manifest_path, &manifest)?;
    Ok(PreparedHotCheckpoint {
        manifest_path: manifest_path.to_path_buf(),
        manifest,
        recorder: Some(recorder),
        replayer: None,
    })
}

fn archive_checkpoint_files(
    manifest_path: &Path,
    activities_path: &Path,
) -> Result<(), HotCheckpointError> {
    let suffix = format!("bak.{}.{}", unix_ms(), uuid::Uuid::new_v4());
    for path in [manifest_path, activities_path] {
        if path.exists() {
            let extension = path
                .extension()
                .and_then(|value| value.to_str())
                .unwrap_or("state");
            let backup = path.with_extension(format!("{extension}.{suffix}"));
            std::fs::rename(path, &backup).map_err(|error| {
                HotCheckpointError::new(format!(
                    "archive Hot Graph checkpoint {} to {}: {error}",
                    path.display(),
                    backup.display()
                ))
            })?;
        }
    }
    Ok(())
}

fn write_manifest_atomic(
    path: &Path,
    manifest: &HotGraphCheckpointManifest,
) -> Result<(), HotCheckpointError> {
    let parent = path.parent().ok_or_else(|| {
        HotCheckpointError::new("Hot Graph checkpoint manifest has no parent directory")
    })?;
    std::fs::create_dir_all(parent).map_err(|error| {
        HotCheckpointError::new(format!(
            "create Hot Graph checkpoint directory {}: {error}",
            parent.display()
        ))
    })?;
    let temporary = path.with_extension(format!("json.tmp.{}", uuid::Uuid::new_v4()));
    let bytes = serde_json::to_vec_pretty(manifest).map_err(|error| {
        HotCheckpointError::new(format!("serialize Hot Graph checkpoint: {error}"))
    })?;
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&temporary)
        .map_err(|error| {
            HotCheckpointError::new(format!(
                "create Hot Graph checkpoint {}: {error}",
                temporary.display()
            ))
        })?;
    file.write_all(&bytes).map_err(|error| {
        HotCheckpointError::new(format!(
            "write Hot Graph checkpoint {}: {error}",
            temporary.display()
        ))
    })?;
    file.sync_all().map_err(|error| {
        HotCheckpointError::new(format!(
            "sync Hot Graph checkpoint {}: {error}",
            temporary.display()
        ))
    })?;
    std::fs::rename(&temporary, path).map_err(|error| {
        HotCheckpointError::new(format!(
            "commit Hot Graph checkpoint {}: {error}",
            path.display()
        ))
    })?;
    File::open(parent)
        .and_then(|directory| directory.sync_all())
        .map_err(|error| {
            HotCheckpointError::new(format!(
                "sync Hot Graph checkpoint directory {}: {error}",
                parent.display()
            ))
        })
}

fn unix_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::AtomicU64;

    use async_trait::async_trait;
    use roko_core::{Body, Kind, Signal};
    use tempfile::tempdir;

    use super::*;
    use crate::budget::{BudgetLimits, BudgetTracker};
    use crate::cell::Cell;
    use crate::loader::load_from_str;

    fn noop_registry() -> CellRegistry {
        let mut r = CellRegistry::new();
        r.register("noop", |_| {
            Box::new(crate::cells::stubs::PassthroughCell::new("noop"))
        });
        r
    }

    fn hot_graph(name: &str) -> Graph {
        load_from_str(&format!(
            r#"
[graph]
name = "{name}"

[[nodes]]
id = "counter"
cell_type = "counter"
execution_class = "activity"
"#
        ))
        .expect("graph")
    }

    fn checkpoint_manifest(directory: &Path) -> HotGraphCheckpointManifest {
        serde_json::from_slice(
            &std::fs::read(directory.join(HOT_CHECKPOINT_MANIFEST)).expect("manifest"),
        )
        .expect("valid manifest")
    }

    struct CountingCell {
        executions: Arc<AtomicU64>,
        cancel_after: Option<(u64, CancellationToken)>,
    }

    #[async_trait]
    impl Cell for CountingCell {
        fn cell_id(&self) -> &str {
            "counter"
        }

        fn cell_name(&self) -> &str {
            "counter"
        }

        async fn execute(
            &self,
            input: Vec<Signal>,
            _ctx: &CellContext,
        ) -> roko_core::error::Result<Vec<Signal>> {
            let previous = input
                .last()
                .and_then(|signal| signal.body.as_text().ok())
                .and_then(|text| text.parse::<u64>().ok())
                .unwrap_or(0);
            let execution = self.executions.fetch_add(1, Ordering::Relaxed) + 1;
            if let Some((after, cancel)) = &self.cancel_after
                && execution == *after
            {
                cancel.cancel();
            }
            Ok(vec![
                Signal::builder(Kind::Task)
                    .body(Body::text((previous + 1).to_string()))
                    .build(),
            ])
        }
    }

    fn counting_registry(
        executions: Arc<AtomicU64>,
        cancel_after: Option<(u64, CancellationToken)>,
    ) -> CellRegistry {
        let mut registry = CellRegistry::new();
        registry.register("counter", move |_| {
            Box::new(CountingCell {
                executions: executions.clone(),
                cancel_after: cancel_after.clone(),
            })
        });
        registry
    }

    #[tokio::test]
    async fn hot_graph_respects_max_ticks() {
        let toml_str = r#"
[graph]
name = "tick-test"

[[nodes]]
id = "a"
cell_type = "noop"
"#;
        let graph = load_from_str(toml_str).unwrap();
        let policy = HotPolicy {
            tick_interval_ms: 0,
            max_ticks: Some(3),
            persist_tick_state: false,
            loop_level: None,
        };
        let handle = start_hot(graph, noop_registry(), policy, None);
        handle.wait().await;
        assert_eq!(handle.tick_count(), 3);
    }

    #[tokio::test]
    async fn hot_graph_cancels_cleanly() {
        let toml_str = r#"
[graph]
name = "cancel-test"

[[nodes]]
id = "a"
cell_type = "noop"
"#;
        let graph = load_from_str(toml_str).unwrap();
        let policy = HotPolicy {
            tick_interval_ms: 100,
            max_ticks: None,
            persist_tick_state: false,
            loop_level: None,
        };
        let handle = start_hot(graph, noop_registry(), policy, None);

        // Let it run a few ticks.
        tokio::time::sleep(Duration::from_millis(50)).await;

        // Cancel.
        handle.cancel();

        // Wait with a timeout to ensure it doesn't hang.
        let result = tokio::time::timeout(Duration::from_millis(500), handle.wait()).await;
        assert!(
            result.is_ok(),
            "hot graph should stop within 500ms of cancel"
        );
    }

    #[tokio::test]
    async fn hot_graph_zero_interval_runs_fast() {
        let toml_str = r#"
[graph]
name = "fast-test"

[[nodes]]
id = "a"
cell_type = "noop"
"#;
        let graph = load_from_str(toml_str).unwrap();
        let policy = HotPolicy {
            tick_interval_ms: 0,
            max_ticks: Some(10),
            persist_tick_state: false,
            loop_level: None,
        };
        let handle = start_hot(graph, noop_registry(), policy, None);
        handle.wait().await;
        assert_eq!(handle.tick_count(), 10);
    }

    #[tokio::test]
    async fn hot_graph_last_output_available() {
        let toml_str = r#"
[graph]
name = "output-test"

[[nodes]]
id = "a"
cell_type = "noop"
"#;
        let graph = load_from_str(toml_str).unwrap();
        let policy = HotPolicy {
            tick_interval_ms: 0,
            max_ticks: Some(1),
            persist_tick_state: false,
            loop_level: None,
        };
        let handle = start_hot(graph, noop_registry(), policy, None);
        handle.wait().await;
        let output = handle.last_output();
        assert!(output.is_some());
        assert!(output.unwrap().success);
    }

    #[tokio::test]
    async fn hot_graph_with_budget_passes_remaining() {
        let toml_str = r#"
[graph]
name = "budget-test"

[[nodes]]
id = "a"
cell_type = "noop"
"#;
        let graph = load_from_str(toml_str).unwrap();
        let policy = HotPolicy {
            tick_interval_ms: 0,
            max_ticks: Some(3),
            persist_tick_state: false,
            loop_level: None,
        };
        let tracker = BudgetTracker::with_limits(BudgetLimits {
            max_tokens: Some(10_000),
            max_cost_usd: Some(5.0),
            deadline: None,
        });
        let enforcer = BudgetEnforcer::new(tracker);
        let handle = start_hot_with_budget(graph, noop_registry(), policy, None, Some(enforcer));
        handle.wait().await;
        assert_eq!(handle.tick_count(), 3);
        // Budget handle is accessible.
        assert!(handle.budget().is_some());
        assert!(!handle.budget().unwrap().is_exhausted());
    }

    #[tokio::test]
    async fn hot_graph_without_budget_has_no_handle() {
        let toml_str = r#"
[graph]
name = "no-budget-test"

[[nodes]]
id = "a"
cell_type = "noop"
"#;
        let graph = load_from_str(toml_str).unwrap();
        let policy = HotPolicy {
            tick_interval_ms: 0,
            max_ticks: Some(1),
            persist_tick_state: false,
            loop_level: None,
        };
        let handle = start_hot(graph, noop_registry(), policy, None);
        handle.wait().await;
        assert!(handle.budget().is_none());
    }

    #[test]
    fn loop_level_default_intervals() {
        assert_eq!(LoopLevel::Gamma.default_interval_ms(), 250);
        assert_eq!(LoopLevel::Theta.default_interval_ms(), 10_000);
        assert_eq!(LoopLevel::Delta.default_interval_ms(), 60_000);
    }

    #[test]
    fn resolve_tick_interval_uses_loop_level_when_set() {
        let policy = HotPolicy {
            tick_interval_ms: 5000,
            max_ticks: None,
            persist_tick_state: false,
            loop_level: Some(LoopLevel::Gamma),
        };
        // loop_level should override tick_interval_ms
        assert_eq!(policy.resolve_tick_interval_ms(), 250);
    }

    #[test]
    fn resolve_tick_interval_falls_back_to_raw_when_no_level() {
        let policy = HotPolicy {
            tick_interval_ms: 3000,
            max_ticks: None,
            persist_tick_state: false,
            loop_level: None,
        };
        assert_eq!(policy.resolve_tick_interval_ms(), 3000);
    }

    #[tokio::test]
    async fn resumable_hot_graph_restores_tick_and_retained_outputs() {
        let temp = tempdir().expect("tempdir");
        let checkpoint_dir = temp.path().join("hot");
        let policy = HotPolicy {
            tick_interval_ms: 0,
            max_ticks: Some(4),
            persist_tick_state: true,
            loop_level: None,
        };
        let first_cancel = CancellationToken::new();
        let first_executions = Arc::new(AtomicU64::new(0));
        let first = start_hot_resumable(
            hot_graph("durable"),
            counting_registry(first_executions.clone(), Some((2, first_cancel.clone()))),
            policy.clone(),
            Some(first_cancel),
            HotCheckpointOptions::new(&checkpoint_dir),
        )
        .expect("start first lifetime");
        first.wait_result().await.expect("first lifetime");
        assert_eq!(first.tick_count(), 2);
        assert_eq!(checkpoint_manifest(&checkpoint_dir).next_tick, 2);

        let second_executions = Arc::new(AtomicU64::new(0));
        let second = start_hot_resumable(
            hot_graph("durable"),
            counting_registry(second_executions.clone(), None),
            policy,
            None,
            HotCheckpointOptions::new(&checkpoint_dir),
        )
        .expect("resume second lifetime");
        second.wait_result().await.expect("second lifetime");

        assert_eq!(second.tick_count(), 4);
        assert_eq!(second_executions.load(Ordering::Relaxed), 2);
        let manifest = checkpoint_manifest(&checkpoint_dir);
        assert_eq!(manifest.next_tick, 4);
        assert_eq!(
            manifest.tick_state["counter"][0]
                .body
                .as_text()
                .expect("text state"),
            "4"
        );
    }

    #[tokio::test]
    async fn interrupted_tick_replays_completed_activity_without_reexecution() {
        let temp = tempdir().expect("tempdir");
        let checkpoint_dir = temp.path().join("hot");
        let policy = HotPolicy {
            tick_interval_ms: 0,
            max_ticks: Some(1),
            persist_tick_state: false,
            loop_level: None,
        };
        let cancelled = CancellationToken::new();
        cancelled.cancel();
        let initial = start_hot_resumable(
            hot_graph("interrupted"),
            counting_registry(Arc::new(AtomicU64::new(0)), None),
            policy.clone(),
            Some(cancelled),
            HotCheckpointOptions::new(&checkpoint_dir),
        )
        .expect("create checkpoint");
        initial.wait_result().await.expect("cancel cleanly");

        let manifest = checkpoint_manifest(&checkpoint_dir);
        let mut recorder =
            ActivityRecorder::create(&manifest.run_id, checkpoint_dir.join(HOT_ACTIVITY_LOG))
                .expect("append interrupted result");
        recorder
            .record(
                "interrupted",
                "counter",
                0,
                vec![
                    Signal::builder(Kind::Task)
                        .body(Body::text("recorded"))
                        .build(),
                ],
            )
            .expect("record completed Activity");
        drop(recorder);

        let executions = Arc::new(AtomicU64::new(0));
        let resumed = start_hot_resumable(
            hot_graph("interrupted"),
            counting_registry(executions.clone(), None),
            policy,
            None,
            HotCheckpointOptions::new(&checkpoint_dir),
        )
        .expect("resume interrupted tick");
        resumed.wait_result().await.expect("replay succeeds");
        assert_eq!(executions.load(Ordering::Relaxed), 0);
        assert_eq!(resumed.tick_count(), 1);
    }

    #[tokio::test]
    async fn drift_and_corrupt_activity_logs_fail_closed() {
        let temp = tempdir().expect("tempdir");
        let checkpoint_dir = temp.path().join("hot");
        let policy = HotPolicy {
            tick_interval_ms: 0,
            max_ticks: Some(1),
            persist_tick_state: false,
            loop_level: None,
        };
        let cancelled = CancellationToken::new();
        cancelled.cancel();
        let initial = start_hot_resumable(
            hot_graph("validated"),
            counting_registry(Arc::new(AtomicU64::new(0)), None),
            policy.clone(),
            Some(cancelled),
            HotCheckpointOptions::new(&checkpoint_dir),
        )
        .expect("create checkpoint");
        initial.wait_result().await.expect("cancel cleanly");

        let original_run = checkpoint_manifest(&checkpoint_dir).run_id;
        let drifted = start_hot_resumable(
            hot_graph("different"),
            counting_registry(Arc::new(AtomicU64::new(0)), None),
            policy.clone(),
            None,
            HotCheckpointOptions::new(&checkpoint_dir),
        )
        .err()
        .expect("graph drift must fail");
        assert!(drifted.to_string().contains("rather than"));

        std::fs::write(checkpoint_dir.join(HOT_ACTIVITY_LOG), b"not-json\n").expect("corrupt log");
        let corrupt = start_hot_resumable(
            hot_graph("validated"),
            counting_registry(Arc::new(AtomicU64::new(0)), None),
            policy,
            None,
            HotCheckpointOptions::new(&checkpoint_dir),
        )
        .err()
        .expect("corrupt log must fail");
        assert!(corrupt.to_string().contains("unparseable Activity record"));

        let forced_cancel = CancellationToken::new();
        forced_cancel.cancel();
        let mut forced_options = HotCheckpointOptions::new(&checkpoint_dir);
        forced_options.force_resume = true;
        let replacement = start_hot_resumable(
            hot_graph("validated"),
            counting_registry(Arc::new(AtomicU64::new(0)), None),
            HotPolicy {
                tick_interval_ms: 0,
                max_ticks: Some(1),
                persist_tick_state: false,
                loop_level: None,
            },
            Some(forced_cancel),
            forced_options,
        )
        .expect("forced replacement");
        replacement.wait_result().await.expect("cancel cleanly");
        assert_ne!(checkpoint_manifest(&checkpoint_dir).run_id, original_run);
        let archived = std::fs::read_dir(&checkpoint_dir)
            .expect("checkpoint directory")
            .filter_map(Result::ok)
            .filter_map(|entry| entry.file_name().into_string().ok())
            .filter(|name| name.contains(".bak."))
            .count();
        assert_eq!(archived, 2);
    }

    #[tokio::test]
    async fn resume_restores_cumulative_budget_state() {
        let temp = tempdir().expect("tempdir");
        let checkpoint_dir = temp.path().join("hot");
        let policy = HotPolicy {
            tick_interval_ms: 0,
            max_ticks: Some(1),
            persist_tick_state: false,
            loop_level: None,
        };
        let limits = BudgetLimits {
            max_tokens: Some(100),
            max_cost_usd: Some(10.0),
            deadline: None,
        };
        let tracker = BudgetTracker::with_limits(limits.clone());
        let first_budget = BudgetEnforcer::new(tracker);
        first_budget.record("before", 7, 1.25, Duration::from_millis(12));
        let cancelled = CancellationToken::new();
        cancelled.cancel();
        let initial = start_hot_resumable_with_budget(
            hot_graph("budget-resume"),
            counting_registry(Arc::new(AtomicU64::new(0)), None),
            policy.clone(),
            Some(cancelled),
            Some(first_budget),
            HotCheckpointOptions::new(&checkpoint_dir),
        )
        .expect("create budget checkpoint");
        initial.wait_result().await.expect("cancel cleanly");

        let resumed_cancel = CancellationToken::new();
        resumed_cancel.cancel();
        let resumed = start_hot_resumable_with_budget(
            hot_graph("budget-resume"),
            counting_registry(Arc::new(AtomicU64::new(0)), None),
            policy,
            Some(resumed_cancel),
            Some(BudgetEnforcer::new(BudgetTracker::with_limits(limits))),
            HotCheckpointOptions::new(&checkpoint_dir),
        )
        .expect("resume budget checkpoint");
        resumed.wait_result().await.expect("cancel cleanly");
        let budget = resumed.budget().expect("budget");
        assert_eq!(budget.tokens_used(), 7);
        assert!((budget.cost_usd() - 1.25).abs() < f64::EPSILON);
        assert_eq!(budget.breakdown().len(), 1);
    }

    struct BlockingCell {
        started: Arc<tokio::sync::Notify>,
        release: Arc<tokio::sync::Notify>,
    }

    #[async_trait]
    impl Cell for BlockingCell {
        fn cell_id(&self) -> &str {
            "counter"
        }

        fn cell_name(&self) -> &str {
            "counter"
        }

        async fn execute(
            &self,
            _input: Vec<Signal>,
            _ctx: &CellContext,
        ) -> roko_core::error::Result<Vec<Signal>> {
            self.started.notify_one();
            self.release.notified().await;
            Ok(Vec::new())
        }
    }

    #[tokio::test]
    async fn checkpoint_commit_failure_is_visible_on_handle() {
        let temp = tempdir().expect("tempdir");
        let checkpoint_dir = temp.path().join("hot");
        let started = Arc::new(tokio::sync::Notify::new());
        let release = Arc::new(tokio::sync::Notify::new());
        let mut registry = CellRegistry::new();
        let factory_started = started.clone();
        let factory_release = release.clone();
        registry.register("counter", move |_| {
            Box::new(BlockingCell {
                started: factory_started.clone(),
                release: factory_release.clone(),
            })
        });
        let handle = start_hot_resumable(
            hot_graph("commit-failure"),
            registry,
            HotPolicy {
                tick_interval_ms: 0,
                max_ticks: Some(1),
                persist_tick_state: false,
                loop_level: None,
            },
            None,
            HotCheckpointOptions::new(&checkpoint_dir),
        )
        .expect("start");
        started.notified().await;
        std::fs::rename(&checkpoint_dir, temp.path().join("moved")).expect("move checkpoint");
        std::fs::write(&checkpoint_dir, b"blocks directory recreation")
            .expect("replace directory with file");
        release.notify_one();

        let failure = handle
            .wait_result()
            .await
            .expect_err("commit failure must surface");
        assert!(failure.to_string().contains("commit Hot Graph checkpoint"));
        assert_eq!(handle.tick_count(), 0);
        assert!(handle.failure().is_some());
    }
}
