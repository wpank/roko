//! Long-lived trigger binding coordinator.
//!
//! The coordinator owns source lifetimes, filter/concurrency state, durable
//! lifecycle evidence, and dispatch into the server's CLI runtime bridge.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::{Arc, Weak};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use alloy::dyn_abi::{DynSolValue, EventExt};
use alloy::json_abi::Event as AbiEvent;
use alloy_primitives::B256;
use anyhow::{Context, Result};
use base64::Engine as _;
use chrono::{DateTime, Utc};
use chrono_tz::Tz;
use cron::Schedule;
use roko_core::capabilities::{CellCapabilities, GraphAllowList, SpaceGrant};
use roko_core::config::WatcherPathConfig;
use roko_core::secrets::{FileStore, Namespace, SecretStore};
use roko_core::trigger::{
    ConcurrencyPolicy, FileWatchEvent, FinalityRequirement, RateLimitAction, SecretRef,
    TriggerAuth, TriggerBinding, TriggerEvent, TriggerEventKind, TriggerGraduationPolicy,
    TriggerKind, TriggerLifecycleEvent, TriggerSource,
};
use roko_core::{
    Body, Capability, CapabilitySet, Kind, Provenance, Pulse, Signal, SignalStatus, Topic,
    TopicFilter,
};
use roko_plugin::{EventSource, EventSourceKind, FileWatchEventSource, SignalSender};
use roko_runtime::cancel::CancelToken;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use tokio::io::AsyncWriteExt;
use tokio::sync::{mpsc, oneshot};
use tokio_util::sync::CancellationToken;
use tracing::{debug, warn};
use uuid::Uuid;

use crate::events::ServerEvent;
use crate::runtime::TriggerExecutionScope;
use crate::state::AppState;

const COMMAND_CAPACITY: usize = 1_024;
const SOURCE_CHANNEL_CAPACITY: usize = 64;
const DEFAULT_QUEUE_DEPTH: usize = 10;
const MAX_QUEUE_DEPTH: usize = 1_024;
const DEFAULT_PARALLEL_LIMIT: usize = 16;
const MAX_PARALLEL_LIMIT: usize = 64;
const RELOAD_INTERVAL: Duration = Duration::from_millis(500);
const QUASI_FINALITY_CONFIRMATIONS: u64 = 12;
const FINAL_CONFIRMATIONS: u64 = 64;

#[derive(Clone, Debug)]
struct TimezoneCronEventSource {
    name: String,
    expression: String,
    timezone: Tz,
    signal_kind: String,
}

impl TimezoneCronEventSource {
    fn new(name: &str, expression: &str, timezone: Option<&str>) -> Result<Self> {
        let timezone = timezone.unwrap_or("UTC").parse::<Tz>().with_context(|| {
            format!("invalid IANA cron timezone '{}'", timezone.unwrap_or("UTC"))
        })?;
        Schedule::from_str(expression)
            .with_context(|| format!("invalid cron expression for trigger '{name}'"))?;
        Ok(Self {
            name: name.to_string(),
            expression: expression.to_string(),
            timezone,
            signal_kind: format!("trigger.source.{name}.cron"),
        })
    }

    fn next_after(&self, after: DateTime<Utc>) -> Result<DateTime<Utc>> {
        let schedule = Schedule::from_str(&self.expression)
            .with_context(|| format!("invalid cron expression for trigger '{}'", self.name))?;
        schedule
            .after(&after.with_timezone(&self.timezone))
            .next()
            .map(|next| next.with_timezone(&Utc))
            .with_context(|| format!("cron trigger '{}' has no future occurrence", self.name))
    }
}

#[async_trait::async_trait]
impl EventSource for TimezoneCronEventSource {
    fn name(&self) -> &'static str {
        "cron"
    }

    fn kind(&self) -> EventSourceKind {
        EventSourceKind::Cron
    }

    async fn start(
        &self,
        sender: SignalSender,
        cancel: CancellationToken,
    ) -> roko_core::Result<()> {
        let mut next = self
            .next_after(Utc::now())
            .map_err(|error| roko_core::RokoError::config(error.to_string()))?;
        loop {
            let wait = next
                .signed_duration_since(Utc::now())
                .to_std()
                .unwrap_or_default();
            tokio::select! {
                _ = cancel.cancelled() => return Ok(()),
                _ = tokio::time::sleep(wait) => {}
            }

            let local_fire = next.with_timezone(&self.timezone);
            let signal = Signal::builder(Kind::Custom(self.signal_kind.clone()))
                .body(Body::Json(json!({
                    "schedule": self.name,
                    "expression": self.expression,
                    "timezone": self.timezone.name(),
                    "scheduled_for": next,
                    "scheduled_local": local_fire.to_rfc3339(),
                })))
                .build();
            sender.send(signal).await.map_err(|_| {
                roko_core::RokoError::cancelled(format!(
                    "cron signal receiver dropped for trigger '{}'",
                    self.name
                ))
            })?;
            next = self
                .next_after(next)
                .map_err(|error| roko_core::RokoError::config(error.to_string()))?;
        }
    }
}

/// Result of submitting a trigger event to the coordinator.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TriggerSubmitStatus {
    /// A graph run was started.
    Started,
    /// The event was retained for debounce, rate-limit, or concurrency handling.
    Queued,
    /// The event was intentionally filtered or dropped by policy.
    Suppressed,
}

/// Cloneable interface to the trigger coordinator actor.
#[derive(Clone)]
pub struct TriggerRuntimeHandle {
    sender: mpsc::Sender<Command>,
}

impl TriggerRuntimeHandle {
    fn spawn(state: Weak<AppState>) -> Self {
        let (sender, receiver) = mpsc::channel(COMMAND_CAPACITY);
        let handle = Self {
            sender: sender.clone(),
        };
        tokio::spawn(TriggerCoordinator::new(state, sender.clone()).run(receiver));
        handle
    }

    /// Submit an event and wait until filter/concurrency admission completes.
    pub async fn submit(&self, event: TriggerEvent) -> Result<TriggerSubmitStatus> {
        let (reply, response) = oneshot::channel();
        self.sender
            .send(Command::Submit {
                event,
                reply: Some(reply),
            })
            .await
            .context("trigger runtime stopped")?;
        response.await.context("trigger runtime stopped")?
    }

    /// Reconcile the live coordinator with a complete binding snapshot.
    pub async fn reconcile(&self, bindings: Vec<TriggerBinding>) -> Result<()> {
        let (reply, response) = oneshot::channel();
        self.sender
            .send(Command::Reconcile {
                bindings,
                reply: Some(reply),
            })
            .await
            .context("trigger runtime stopped")?;
        response.await.context("trigger runtime stopped")
    }

    /// Record a source-side failure (for example failed webhook auth) through
    /// the same durable lifecycle path as coordinator failures.
    pub async fn record_source_error(&self, name: String, detail: Value) {
        let _ = self
            .sender
            .send(Command::SourceError { name, detail })
            .await;
    }
}

/// Start the trigger runtime once and return its shared handle.
pub async fn ensure_trigger_runtime(state: &Arc<AppState>) -> TriggerRuntimeHandle {
    state
        .trigger_runtime
        .get_or_init(|| async {
            let handle = TriggerRuntimeHandle::spawn(Arc::downgrade(state));
            start_event_observer(Arc::downgrade(state), handle.clone());
            start_pulse_observer(Arc::downgrade(state), handle.clone());
            start_disk_reloader(Arc::downgrade(state), handle.clone());
            start_shutdown_observer(Arc::downgrade(state), handle.clone());
            let bindings = state
                .trigger_bindings
                .read()
                .await
                .values()
                .cloned()
                .collect();
            if let Err(error) = handle.reconcile(bindings).await {
                warn!(%error, "initial trigger reconciliation failed");
            }
            handle
        })
        .await
        .clone()
}

enum Command {
    Reconcile {
        bindings: Vec<TriggerBinding>,
        reply: Option<oneshot::Sender<()>>,
    },
    Submit {
        event: TriggerEvent,
        reply: Option<oneshot::Sender<Result<TriggerSubmitStatus>>>,
    },
    Observe {
        event: ServerEvent,
        sequence: u64,
    },
    ObservePulse {
        pulse: Pulse,
    },
    DebounceReady {
        name: String,
        generation: u64,
    },
    RateReady {
        name: String,
        generation: u64,
    },
    FlowDone {
        name: String,
        run_id: Uuid,
        outcome: FlowOutcome,
    },
    SourceStopped {
        name: String,
        generation: u64,
        error: Option<String>,
    },
    Rearm {
        name: String,
        generation: u64,
    },
    SourceError {
        name: String,
        detail: Value,
    },
    Shutdown,
}

struct ActiveBinding {
    binding: TriggerBinding,
    signature: Vec<u8>,
    armed: bool,
    source_cancel: Option<CancellationToken>,
    source_generation: u64,
    running: HashMap<Uuid, RunningFlow>,
    concurrency_queue: VecDeque<TriggerEvent>,
    debounce_event: Option<TriggerEvent>,
    debounce_generation: u64,
    rate_history: VecDeque<u64>,
    rate_queue: VecDeque<TriggerEvent>,
    rate_generation: u64,
    signal_history: VecDeque<(u64, String, String)>,
}

struct RunningFlow {
    cancel: CancelToken,
    trace_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct ChainLogKey {
    chain_id: u64,
    block_number: u64,
    block_hash: String,
    tx_hash: String,
    log_index: u32,
}

struct PendingChainEvent {
    binding_name: String,
    required: FinalityRequirement,
    event: TriggerEvent,
}

struct DeliveredChainEvent {
    binding_name: String,
    trace_id: String,
}

impl ActiveBinding {
    fn new(binding: TriggerBinding, signature: Vec<u8>) -> Self {
        Self {
            binding,
            signature,
            armed: false,
            source_cancel: None,
            source_generation: 0,
            running: HashMap::new(),
            concurrency_queue: VecDeque::new(),
            debounce_event: None,
            debounce_generation: 0,
            rate_history: VecDeque::new(),
            rate_queue: VecDeque::new(),
            rate_generation: 0,
            signal_history: VecDeque::new(),
        }
    }
}

struct TriggerCoordinator {
    state: Weak<AppState>,
    sender: mpsc::Sender<Command>,
    bindings: HashMap<String, ActiveBinding>,
    seen_traces: HashSet<(String, String)>,
    pending_chain: HashMap<ChainLogKey, Vec<PendingChainEvent>>,
    delivered_chain: HashMap<ChainLogKey, Vec<DeliveredChainEvent>>,
    canonical_blocks: BTreeMap<u64, (String, String)>,
}

impl TriggerCoordinator {
    fn new(state: Weak<AppState>, sender: mpsc::Sender<Command>) -> Self {
        let seen_traces = state.upgrade().map_or_else(HashSet::new, |state| {
            load_seen_traces(&state.layout.triggers_dir())
        });
        Self {
            state,
            sender,
            bindings: HashMap::new(),
            seen_traces,
            pending_chain: HashMap::new(),
            delivered_chain: HashMap::new(),
            canonical_blocks: BTreeMap::new(),
        }
    }

    async fn run(mut self, mut receiver: mpsc::Receiver<Command>) {
        while let Some(command) = receiver.recv().await {
            match command {
                Command::Reconcile { bindings, reply } => {
                    self.reconcile(bindings).await;
                    if let Some(reply) = reply {
                        let _ = reply.send(());
                    }
                }
                Command::Submit { event, reply } => {
                    let result = self.submit_event(event, false).await;
                    if let Some(reply) = reply {
                        let _ = reply.send(result);
                    }
                }
                Command::Observe { event, sequence } => {
                    self.observe_event(event, sequence).await;
                }
                Command::ObservePulse { pulse } => self.observe_pulse(pulse).await,
                Command::DebounceReady { name, generation } => {
                    self.debounce_ready(&name, generation).await;
                }
                Command::RateReady { name, generation } => {
                    self.rate_ready(&name, generation).await;
                }
                Command::FlowDone {
                    name,
                    run_id,
                    outcome,
                } => self.flow_done(&name, run_id, outcome).await,
                Command::SourceStopped {
                    name,
                    generation,
                    error,
                } => self.source_stopped(&name, generation, error).await,
                Command::Rearm { name, generation } => self.rearm(&name, generation).await,
                Command::SourceError { name, detail } => {
                    if let Some(active) = self.bindings.get(&name) {
                        self.emit_lifecycle(&active.binding, TriggerEventKind::Error, None, detail)
                            .await;
                    }
                }
                Command::Shutdown => {
                    self.shutdown().await;
                    break;
                }
            }
        }
    }

    async fn reconcile(&mut self, bindings: Vec<TriggerBinding>) {
        let incoming_names: HashSet<_> = bindings.iter().map(|b| b.name.clone()).collect();
        let removed: Vec<_> = self
            .bindings
            .keys()
            .filter(|name| !incoming_names.contains(*name))
            .cloned()
            .collect();
        for name in removed {
            if let Some(mut active) = self.bindings.remove(&name) {
                self.disarm(&mut active, "binding removed").await;
            }
        }

        for binding in bindings {
            let signature = match serde_json::to_vec(&binding) {
                Ok(signature) => signature,
                Err(error) => {
                    warn!(trigger = %binding.name, %error, "cannot fingerprint trigger binding");
                    continue;
                }
            };
            let unchanged = self
                .bindings
                .get(&binding.name)
                .is_some_and(|active| active.signature == signature);
            if unchanged {
                continue;
            }
            if let Some(mut old) = self.bindings.remove(&binding.name) {
                self.disarm(&mut old, "binding reloaded").await;
            }
            let mut active = ActiveBinding::new(binding, signature);
            if active.binding.enabled {
                self.arm(&mut active).await;
            } else {
                self.emit_lifecycle(
                    &active.binding,
                    TriggerEventKind::Disarmed,
                    None,
                    json!({"reason": "binding disabled"}),
                )
                .await;
            }
            self.bindings.insert(active.binding.name.clone(), active);
        }
    }

    async fn arm(&mut self, active: &mut ActiveBinding) {
        active.armed = true;
        self.emit_lifecycle(
            &active.binding,
            TriggerEventKind::Armed,
            None,
            json!({"source": source_label(&active.binding.kind)}),
        )
        .await;

        match start_source(&active.binding, self.state.clone(), self.sender.clone()) {
            Ok(Some((cancel, generation))) => {
                active.source_cancel = Some(cancel);
                active.source_generation = generation;
            }
            Ok(None) => {}
            Err(error) => {
                self.emit_lifecycle(
                    &active.binding,
                    TriggerEventKind::Error,
                    None,
                    json!({"phase": "arm", "error": error.to_string()}),
                )
                .await;
            }
        }
    }

    async fn disarm(&mut self, active: &mut ActiveBinding, reason: &str) {
        if let Some(cancel) = active.source_cancel.take() {
            cancel.cancel();
        }
        for flow in active.running.values() {
            flow.cancel.cancel();
        }
        active.running.clear();
        active.concurrency_queue.clear();
        active.rate_queue.clear();
        active.debounce_event = None;
        if active.armed {
            active.armed = false;
            self.emit_lifecycle(
                &active.binding,
                TriggerEventKind::Disarmed,
                None,
                json!({"reason": reason}),
            )
            .await;
        }
    }

    async fn submit_event(
        &mut self,
        event: TriggerEvent,
        bypass_debounce: bool,
    ) -> Result<TriggerSubmitStatus> {
        let name = event.trigger_id.clone();
        let Some(mut active) = self.bindings.remove(&name) else {
            anyhow::bail!("trigger '{name}' not found");
        };
        let result = if !active.binding.enabled || !active.armed {
            Err(anyhow::anyhow!("trigger '{name}' is disabled"))
        } else {
            self.admit_event(&mut active, event, bypass_debounce).await
        };
        self.bindings.insert(name, active);
        result
    }

    async fn admit_event(
        &mut self,
        active: &mut ActiveBinding,
        event: TriggerEvent,
        bypass_debounce: bool,
    ) -> Result<TriggerSubmitStatus> {
        if !payload_matches(&event.payload, active.binding.filter.as_ref()) {
            self.emit_lifecycle(
                &active.binding,
                TriggerEventKind::Filtered,
                Some(event.trace_id),
                json!({"reason": "payload did not match"}),
            )
            .await;
            return Ok(TriggerSubmitStatus::Suppressed);
        }

        if !bypass_debounce
            && let Some(delay_ms) = active
                .binding
                .filter
                .as_ref()
                .and_then(|filter| filter.debounce_ms)
            && delay_ms > 0
        {
            active.debounce_event = Some(event.clone());
            active.debounce_generation = active.debounce_generation.wrapping_add(1);
            let generation = active.debounce_generation;
            let sender = self.sender.clone();
            let name = active.binding.name.clone();
            tokio::spawn(async move {
                tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                let _ = sender
                    .send(Command::DebounceReady { name, generation })
                    .await;
            });
            self.emit_lifecycle(
                &active.binding,
                TriggerEventKind::Queued,
                Some(event.trace_id),
                json!({"reason": "debounce", "delay_ms": delay_ms}),
            )
            .await;
            return Ok(TriggerSubmitStatus::Queued);
        }

        if let Some(rate_limit) = active
            .binding
            .filter
            .as_ref()
            .and_then(|filter| filter.rate_limit.clone())
        {
            let now = now_ms();
            while active
                .rate_history
                .front()
                .is_some_and(|fired| now.saturating_sub(*fired) >= rate_limit.window_ms)
            {
                active.rate_history.pop_front();
            }
            if active.rate_history.len() >= rate_limit.max_fires as usize {
                self.emit_lifecycle(
                    &active.binding,
                    TriggerEventKind::RateLimited,
                    Some(event.trace_id.clone()),
                    json!({
                        "max_fires": rate_limit.max_fires,
                        "window_ms": rate_limit.window_ms,
                        "action": rate_limit.on_limit,
                    }),
                )
                .await;
                match rate_limit.on_limit {
                    RateLimitAction::Drop => return Ok(TriggerSubmitStatus::Suppressed),
                    RateLimitAction::Warn => {
                        warn!(trigger = %active.binding.name, "trigger rate limit exceeded; firing because action=warn");
                    }
                    RateLimitAction::Queue => {
                        let limit = queue_depth(&active.binding.concurrency);
                        if active.rate_queue.len() >= limit {
                            self.emit_lifecycle(
                                &active.binding,
                                TriggerEventKind::Skipped,
                                Some(event.trace_id),
                                json!({"reason": "rate-limit queue full", "max_depth": limit}),
                            )
                            .await;
                            return Ok(TriggerSubmitStatus::Suppressed);
                        }
                        active.rate_queue.push_back(event.clone());
                        self.schedule_rate_wakeup(active, rate_limit.window_ms);
                        self.emit_lifecycle(
                            &active.binding,
                            TriggerEventKind::Queued,
                            Some(event.trace_id),
                            json!({"reason": "rate_limit", "max_depth": limit}),
                        )
                        .await;
                        return Ok(TriggerSubmitStatus::Queued);
                    }
                }
            }
            active.rate_history.push_back(now);
        }

        self.apply_concurrency(active, event).await
    }

    async fn apply_concurrency(
        &mut self,
        active: &mut ActiveBinding,
        event: TriggerEvent,
    ) -> Result<TriggerSubmitStatus> {
        match active.binding.concurrency.clone() {
            ConcurrencyPolicy::Queue { .. } if !active.running.is_empty() => {
                let max_depth = queue_depth(&active.binding.concurrency);
                if active.concurrency_queue.len() >= max_depth {
                    self.emit_lifecycle(
                        &active.binding,
                        TriggerEventKind::Skipped,
                        Some(event.trace_id),
                        json!({"reason": "concurrency queue full", "max_depth": max_depth}),
                    )
                    .await;
                    return Ok(TriggerSubmitStatus::Suppressed);
                }
                active.concurrency_queue.push_back(event.clone());
                self.emit_lifecycle(
                    &active.binding,
                    TriggerEventKind::Queued,
                    Some(event.trace_id),
                    json!({"reason": "flow already running", "max_depth": max_depth}),
                )
                .await;
                Ok(TriggerSubmitStatus::Queued)
            }
            ConcurrencyPolicy::Skip if !active.running.is_empty() => {
                self.emit_lifecycle(
                    &active.binding,
                    TriggerEventKind::Skipped,
                    Some(event.trace_id),
                    json!({"reason": "flow already running"}),
                )
                .await;
                Ok(TriggerSubmitStatus::Suppressed)
            }
            ConcurrencyPolicy::CancelRunning if !active.running.is_empty() => {
                for flow in active.running.values() {
                    flow.cancel.cancel();
                }
                self.launch_flow(active, event).await
            }
            ConcurrencyPolicy::Parallel { .. }
                if active.running.len() >= parallel_limit(&active.binding.concurrency) =>
            {
                self.emit_lifecycle(
                    &active.binding,
                    TriggerEventKind::Skipped,
                    Some(event.trace_id),
                    json!({
                        "reason": "parallel limit reached",
                        "max_concurrent": parallel_limit(&active.binding.concurrency),
                    }),
                )
                .await;
                Ok(TriggerSubmitStatus::Suppressed)
            }
            _ => self.launch_flow(active, event).await,
        }
    }

    async fn launch_flow(
        &mut self,
        active: &mut ActiveBinding,
        mut event: TriggerEvent,
    ) -> Result<TriggerSubmitStatus> {
        let trace_key = (active.binding.name.clone(), event.trace_id.clone());
        if self.seen_traces.contains(&trace_key) {
            self.emit_lifecycle(
                &active.binding,
                TriggerEventKind::Skipped,
                Some(event.trace_id),
                json!({"reason": "duplicate trace"}),
            )
            .await;
            return Ok(TriggerSubmitStatus::Suppressed);
        }
        let Some(state) = self.state.upgrade() else {
            anyhow::bail!("server state dropped");
        };
        let execution_scope = resolve_trigger_execution_scope(&state, &active.binding, &mut event)?;
        let mapped_event = map_event_input(&active.binding, event.clone())?;
        self.persist_fired_event(&active.binding, &event).await?;
        self.seen_traces.insert(trace_key);

        if let Some(state) = self.state.upgrade() {
            state.event_bus.publish(ServerEvent::TriggerFired {
                trigger_name: active.binding.name.clone(),
                event: event.clone(),
            });
        }
        self.emit_lifecycle(
            &active.binding,
            TriggerEventKind::Fired,
            Some(event.trace_id.clone()),
            json!({"source": &event.source, "event": &event}),
        )
        .await;
        let run_id = Uuid::new_v4();
        let cancel = state.cancel.child();
        active.running.insert(
            run_id,
            RunningFlow {
                cancel: cancel.clone(),
                trace_id: event.trace_id.clone(),
            },
        );
        let effective_capabilities = execution_scope.capabilities.as_ref().map(|capabilities| {
            let mut names = capabilities
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>();
            names.sort();
            names
        });
        self.emit_lifecycle(
            &active.binding,
            TriggerEventKind::FlowStarted,
            Some(event.trace_id.clone()),
            json!({
                "graph": &active.binding.graph,
                "run_id": run_id,
                "space_id": execution_scope.space_id.as_deref(),
                "effective_capabilities": effective_capabilities,
            }),
        )
        .await;
        let sender = self.sender.clone();
        let name = active.binding.name.clone();
        let graph = state.workdir.join(&active.binding.graph);
        let workdir = state.workdir.clone();
        let runtime = Arc::clone(&state.runtime);
        tokio::spawn(async move {
            let outcome = tokio::select! {
                _ = cancel.cancelled() => FlowOutcome::Cancelled,
                result = runtime.run_trigger_graph_scoped(
                    &workdir,
                    &graph,
                    &mapped_event,
                    &execution_scope,
                ) => {
                    match result {
                        Ok(result) => FlowOutcome::Completed {
                            success: result.success,
                            output: result.output_text,
                        },
                        Err(error) => FlowOutcome::Failed(error.to_string()),
                    }
                }
            };
            let _ = sender
                .send(Command::FlowDone {
                    name,
                    run_id,
                    outcome,
                })
                .await;
        });
        Ok(TriggerSubmitStatus::Started)
    }

    async fn debounce_ready(&mut self, name: &str, generation: u64) {
        let Some(mut active) = self.bindings.remove(name) else {
            return;
        };
        if active.debounce_generation == generation
            && let Some(event) = active.debounce_event.take()
            && let Err(error) = self.admit_event(&mut active, event, true).await
        {
            self.emit_lifecycle(
                &active.binding,
                TriggerEventKind::Error,
                None,
                json!({"phase": "debounce", "error": error.to_string()}),
            )
            .await;
        }
        self.bindings.insert(name.to_string(), active);
    }

    fn schedule_rate_wakeup(&self, active: &mut ActiveBinding, window_ms: u64) {
        active.rate_generation = active.rate_generation.wrapping_add(1);
        let generation = active.rate_generation;
        let name = active.binding.name.clone();
        let sender = self.sender.clone();
        let elapsed = active
            .rate_history
            .front()
            .map_or(0, |fired| now_ms().saturating_sub(*fired));
        let delay = window_ms.saturating_sub(elapsed).max(1);
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(delay)).await;
            let _ = sender.send(Command::RateReady { name, generation }).await;
        });
    }

    async fn rate_ready(&mut self, name: &str, generation: u64) {
        let Some(mut active) = self.bindings.remove(name) else {
            return;
        };
        if active.rate_generation == generation
            && let Some(event) = active.rate_queue.pop_front()
        {
            if let Err(error) = self.admit_event(&mut active, event, true).await {
                self.emit_lifecycle(
                    &active.binding,
                    TriggerEventKind::Error,
                    None,
                    json!({"phase": "rate_limit", "error": error.to_string()}),
                )
                .await;
            }
            if !active.rate_queue.is_empty()
                && let Some(window_ms) = active
                    .binding
                    .filter
                    .as_ref()
                    .and_then(|filter| filter.rate_limit.as_ref())
                    .map(|limit| limit.window_ms)
            {
                self.schedule_rate_wakeup(&mut active, window_ms);
            }
        }
        self.bindings.insert(name.to_string(), active);
    }

    async fn flow_done(&mut self, name: &str, run_id: Uuid, outcome: FlowOutcome) {
        let Some(mut active) = self.bindings.remove(name) else {
            return;
        };
        let Some(flow) = active.running.remove(&run_id) else {
            self.bindings.insert(name.to_string(), active);
            return;
        };
        let (success, detail) = match outcome {
            FlowOutcome::Completed { success, output } => (
                success,
                json!({
                    "run_id": run_id,
                    "success": success,
                    "output": output,
                }),
            ),
            FlowOutcome::Failed(error) => (
                false,
                json!({
                    "run_id": run_id,
                    "success": false,
                    "error": error,
                }),
            ),
            FlowOutcome::Cancelled => (
                false,
                json!({
                    "run_id": run_id,
                    "success": false,
                    "cancelled": true,
                }),
            ),
        };
        self.emit_lifecycle(
            &active.binding,
            TriggerEventKind::FlowCompleted,
            Some(flow.trace_id),
            detail,
        )
        .await;
        if !success {
            debug!(trigger = %active.binding.name, "trigger flow did not succeed");
        }
        if active.running.is_empty()
            && let Some(event) = active.concurrency_queue.pop_front()
            && let Err(error) = self.launch_flow(&mut active, event).await
        {
            self.emit_lifecycle(
                &active.binding,
                TriggerEventKind::Error,
                None,
                json!({"phase": "queued_flow", "error": error.to_string()}),
            )
            .await;
        }
        self.bindings.insert(name.to_string(), active);
    }

    async fn source_stopped(&mut self, name: &str, generation: u64, error: Option<String>) {
        let Some(mut active) = self.bindings.remove(name) else {
            return;
        };
        if active.source_generation != generation {
            self.bindings.insert(name.to_string(), active);
            return;
        }
        active.source_cancel = None;
        self.emit_lifecycle(
            &active.binding,
            TriggerEventKind::Error,
            None,
            json!({
                "phase": "source",
                "error": error.unwrap_or_else(|| "source stopped unexpectedly".to_string()),
            }),
        )
        .await;
        let sender = self.sender.clone();
        let rearm_name = name.to_string();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_secs(1)).await;
            let _ = sender
                .send(Command::Rearm {
                    name: rearm_name,
                    generation,
                })
                .await;
        });
        self.bindings.insert(name.to_string(), active);
    }

    async fn rearm(&mut self, name: &str, generation: u64) {
        let Some(mut active) = self.bindings.remove(name) else {
            return;
        };
        if active.binding.enabled
            && active.source_cancel.is_none()
            && active.source_generation == generation
        {
            match start_source(&active.binding, self.state.clone(), self.sender.clone()) {
                Ok(Some((cancel, new_generation))) => {
                    active.source_cancel = Some(cancel);
                    active.source_generation = new_generation;
                }
                Ok(None) => {}
                Err(error) => {
                    self.emit_lifecycle(
                        &active.binding,
                        TriggerEventKind::Error,
                        None,
                        json!({"phase": "rearm", "error": error.to_string()}),
                    )
                    .await;
                }
            }
        }
        self.bindings.insert(name.to_string(), active);
    }

    async fn observe_event(&mut self, event: ServerEvent, sequence: u64) {
        if matches!(
            event,
            ServerEvent::TriggerFired { .. } | ServerEvent::TriggerLifecycle { .. }
        ) {
            return;
        }

        if let ServerEvent::WebhookReceived { signal } = &event {
            self.observe_signal(signal, sequence).await;
        }
        if let ServerEvent::ChainContractEvent {
            block_number,
            tx_hash,
            contract,
            event_name,
            decoded,
            raw_evidence_available,
            ..
        } = &event
            && !raw_evidence_available
        {
            self.observe_chain(
                *block_number,
                tx_hash,
                contract,
                event_name,
                decoded.clone(),
            )
            .await;
        }
        if let ServerEvent::ChainBlock {
            number,
            hash,
            parent_hash,
            ..
        } = &event
        {
            self.observe_chain_block(*number, hash, parent_hash).await;
        }
        match &event {
            ServerEvent::ChainLogObserved {
                chain_id,
                block_number,
                block_hash,
                tx_hash,
                log_index,
                contract,
                topics,
                data,
                finality,
                removed,
            } => {
                self.observe_raw_chain_log(
                    *chain_id,
                    *block_number,
                    block_hash,
                    tx_hash,
                    *log_index,
                    contract,
                    topics,
                    data,
                    *finality,
                    *removed,
                )
                .await;
            }
            ServerEvent::ChainFinalityUpdated {
                chain_id,
                block_hash,
                finality,
            } => {
                self.promote_chain_finality(*chain_id, block_hash, *finality)
                    .await;
            }
            ServerEvent::ChainReorg {
                chain_id,
                orphaned_block_hashes,
            } => {
                self.invalidate_chain_reorg(*chain_id, orphaned_block_hashes)
                    .await;
            }
            _ => {}
        }
        if let Some((topic, payload)) = server_bus_projection(&event) {
            self.observe_bus(&topic, sequence, payload).await;
        }
    }

    async fn observe_pulse(&mut self, pulse: Pulse) {
        let mut payload = body_value(&pulse.body);
        if let Some(space_id) = pulse.tag("space_id") {
            if let Some(object) = payload.as_object_mut() {
                object.insert("_space_id".to_string(), Value::String(space_id.to_string()));
            } else {
                payload = json!({"data": payload, "_space_id": space_id});
            }
        }
        self.observe_bus(&pulse.topic.to_string(), pulse.seq, payload)
            .await;
    }

    async fn observe_signal(&mut self, signal: &Signal, sequence: u64) {
        let topic = signal.kind.to_string();
        let mut payload = body_value(&signal.body);
        if let Some(space_id) = signal.tags.get("space_id") {
            if let Some(object) = payload.as_object_mut() {
                object.insert("_space_id".to_string(), Value::String(space_id.clone()));
            } else {
                payload = json!({"data": payload, "_space_id": space_id});
            }
        }
        self.observe_bus(&topic, sequence, payload.clone()).await;

        let now = now_ms();
        let names: Vec<_> = self.bindings.keys().cloned().collect();
        for name in names {
            let Some(mut active) = self.bindings.remove(&name) else {
                continue;
            };
            let TriggerKind::SignalPattern(pattern) = &active.binding.kind else {
                self.bindings.insert(name, active);
                continue;
            };
            if !active.binding.enabled || !active.armed {
                self.bindings.insert(name, active);
                continue;
            }
            if let Some(space_id) = active.binding.space.as_deref()
                && signal.tags.get("space_id").map(String::as_str) != Some(space_id)
            {
                self.bindings.insert(name, active);
                continue;
            }
            let window_ms = pattern.window_secs.saturating_mul(1_000);
            active
                .signal_history
                .retain(|(seen, _, _)| now.saturating_sub(*seen) <= window_ms);
            active
                .signal_history
                .push_back((now, topic.clone(), signal.id.to_string()));
            let matched = pattern.required_kinds.iter().all(|required| {
                active
                    .signal_history
                    .iter()
                    .any(|(_, kind, _)| kind == required)
            });
            if matched {
                let matched_signals = pattern
                    .required_kinds
                    .iter()
                    .filter_map(|required| {
                        active
                            .signal_history
                            .iter()
                            .rev()
                            .find(|(_, kind, _)| kind == required)
                            .map(|(_, _, id)| id.clone())
                    })
                    .collect();
                active.signal_history.clear();
                let event = TriggerEvent::new(
                    name.clone(),
                    payload.clone(),
                    TriggerSource::SignalPattern { matched_signals },
                    Uuid::new_v4().to_string(),
                );
                if let Err(error) = self.admit_event(&mut active, event, false).await {
                    warn!(trigger = %name, %error, "signal-pattern trigger failed");
                }
            }
            self.bindings.insert(name, active);
        }
    }

    async fn observe_bus(&mut self, topic: &str, sequence: u64, payload: Value) {
        let names: Vec<_> = self.bindings.keys().cloned().collect();
        for name in names {
            let matches = self.bindings.get(&name).is_some_and(|active| {
                active.binding.enabled
                    && active.armed
                    && matches!(
                        &active.binding.kind,
                        TriggerKind::Bus(config) if wildcard_matches(&config.topic, topic)
                    )
            });
            if !matches {
                continue;
            }
            let mut event = TriggerEvent::new(
                name.clone(),
                payload.clone(),
                TriggerSource::Bus {
                    topic: topic.to_string(),
                    pulse_seq: sequence,
                },
                Uuid::new_v4().to_string(),
            );
            if let Some(space_id) = payload
                .get("space_id")
                .or_else(|| payload.get("_space_id"))
                .and_then(Value::as_str)
            {
                event = event.with_space(space_id.to_string());
            }
            if let Err(error) = self.submit_event(event, false).await {
                warn!(trigger = %name, %error, "bus trigger failed");
            }
        }
    }

    async fn observe_chain_block(&mut self, number: u64, hash: &str, parent_hash: &str) {
        let chain_id = self
            .state
            .upgrade()
            .and_then(|state| state.load_roko_config().chain.chain_id)
            .unwrap_or_default();
        let hash = hash.to_ascii_lowercase();
        let parent_hash = parent_hash.to_ascii_lowercase();
        if self
            .canonical_blocks
            .get(&number)
            .is_some_and(|(known, _)| known == &hash)
        {
            self.promote_confirmed_chain_events(chain_id, number).await;
            return;
        }

        let mut orphaned = Vec::new();
        if number > 0
            && self
                .canonical_blocks
                .get(&(number - 1))
                .is_some_and(|(known, _)| known != &parent_hash)
        {
            let ancestor = self
                .canonical_blocks
                .iter()
                .find_map(|(height, (known, _))| (known == &parent_hash).then_some(*height));
            let first_orphan =
                ancestor.map_or_else(|| number.saturating_sub(1), |height| height + 1);
            orphaned.extend(
                self.canonical_blocks
                    .range(first_orphan..)
                    .map(|(_, (known, _))| known.clone()),
            );
            self.canonical_blocks
                .retain(|height, _| *height < first_orphan);
        } else if self.canonical_blocks.contains_key(&number) {
            orphaned.extend(
                self.canonical_blocks
                    .range(number..)
                    .map(|(_, (known, _))| known.clone()),
            );
            self.canonical_blocks.retain(|height, _| *height < number);
        }
        if !orphaned.is_empty() {
            self.invalidate_chain_reorg(chain_id, &orphaned).await;
        }

        self.canonical_blocks.insert(number, (hash, parent_hash));
        self.promote_confirmed_chain_events(chain_id, number).await;
    }

    async fn promote_confirmed_chain_events(&mut self, chain_id: u64, head: u64) {
        let mut invalid = HashSet::new();
        let mut promotions: BTreeSet<(String, FinalityRequirement)> = BTreeSet::new();
        for key in self
            .pending_chain
            .keys()
            .filter(|key| key.chain_id == chain_id)
        {
            match self.canonical_blocks.get(&key.block_number) {
                Some((canonical, _)) if canonical == &key.block_hash => {
                    let confirmations = head.saturating_sub(key.block_number);
                    let finality = if confirmations >= FINAL_CONFIRMATIONS {
                        FinalityRequirement::Final
                    } else if confirmations >= QUASI_FINALITY_CONFIRMATIONS {
                        FinalityRequirement::QuasiFinalized
                    } else {
                        FinalityRequirement::Reversible
                    };
                    promotions.insert((key.block_hash.clone(), finality));
                }
                Some(_) => {
                    invalid.insert(key.block_hash.clone());
                }
                None => {}
            }
        }
        if !invalid.is_empty() {
            self.invalidate_chain_reorg(chain_id, &invalid.into_iter().collect::<Vec<_>>())
                .await;
        }
        for (block_hash, finality) in promotions {
            self.promote_chain_finality(chain_id, &block_hash, finality)
                .await;
        }
    }

    async fn observe_chain(
        &mut self,
        block_number: u64,
        tx_hash: &str,
        contract: &str,
        event_name: &str,
        decoded: Value,
    ) {
        let chain_id = self
            .state
            .upgrade()
            .and_then(|state| state.load_roko_config().chain.chain_id)
            .unwrap_or_default();
        let names: Vec<_> = self.bindings.keys().cloned().collect();
        for name in names {
            let matches = self.bindings.get(&name).is_some_and(|active| {
                active.binding.enabled
                    && active.armed
                    && matches!(&active.binding.kind, TriggerKind::ChainEvent(config)
                        if config.chain_id == chain_id
                            && config.contract.eq_ignore_ascii_case(contract)
                            && config.abi.is_none()
                            && config.finality == FinalityRequirement::Reversible
                            && chain_event_matches(&config.event_signature, event_name))
            });
            if !matches {
                continue;
            }
            let event = TriggerEvent::new(
                name.clone(),
                json!({
                    "chain_id": chain_id,
                    "block_number": block_number,
                    "tx_hash": tx_hash,
                    "contract": contract,
                    "event_name": event_name,
                    "decoded": decoded,
                    "finality": FinalityRequirement::Reversible,
                    "legacy_predecoded": true,
                }),
                TriggerSource::ChainEvent {
                    chain_id,
                    block_number,
                    tx_hash: tx_hash.to_string(),
                },
                Uuid::new_v4().to_string(),
            );
            if let Err(error) = self.submit_event(event, false).await {
                warn!(trigger = %name, %error, "chain trigger failed");
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    async fn observe_raw_chain_log(
        &mut self,
        chain_id: u64,
        block_number: u64,
        block_hash: &str,
        tx_hash: &str,
        log_index: u32,
        contract: &str,
        topics: &[String],
        data: &str,
        finality: FinalityRequirement,
        removed: bool,
    ) {
        if removed {
            self.invalidate_chain_reorg(chain_id, &[block_hash.to_string()])
                .await;
            return;
        }

        let key = ChainLogKey {
            chain_id,
            block_number,
            block_hash: block_hash.to_ascii_lowercase(),
            tx_hash: tx_hash.to_ascii_lowercase(),
            log_index,
        };
        let candidates: Vec<_> = self
            .bindings
            .iter()
            .filter_map(|(name, active)| match &active.binding.kind {
                TriggerKind::ChainEvent(config)
                    if active.binding.enabled
                        && active.armed
                        && config.chain_id == chain_id
                        && config.contract.eq_ignore_ascii_case(contract) =>
                {
                    Some((name.clone(), config.clone()))
                }
                _ => None,
            })
            .collect();

        for (name, config) in candidates {
            if self.chain_log_seen_for_binding(&key, &name) {
                continue;
            }
            let decoded = match decode_chain_log(&config, topics, data) {
                Ok(Some(decoded)) => decoded,
                Ok(None) => continue,
                Err(error) => {
                    if let Some(binding) = self
                        .bindings
                        .get(&name)
                        .map(|active| active.binding.clone())
                    {
                        self.emit_lifecycle(
                            &binding,
                            TriggerEventKind::Error,
                            None,
                            json!({
                                "phase": "chain_decode",
                                "block_hash": block_hash,
                                "tx_hash": tx_hash,
                                "log_index": log_index,
                                "error": error.to_string(),
                            }),
                        )
                        .await;
                    }
                    continue;
                }
            };
            let event = TriggerEvent::new(
                name.clone(),
                json!({
                    "chain_id": chain_id,
                    "block_number": block_number,
                    "block_hash": block_hash,
                    "tx_hash": tx_hash,
                    "log_index": log_index,
                    "contract": contract,
                    "event_signature": config.event_signature,
                    "decoded": decoded,
                    "finality": finality,
                }),
                TriggerSource::ChainEvent {
                    chain_id,
                    block_number,
                    tx_hash: tx_hash.to_string(),
                },
                Uuid::new_v4().to_string(),
            );

            if finality >= config.finality {
                let trace_id = event.trace_id.clone();
                match self.submit_event(event, false).await {
                    Ok(TriggerSubmitStatus::Started | TriggerSubmitStatus::Queued) => {
                        self.delivered_chain.entry(key.clone()).or_default().push(
                            DeliveredChainEvent {
                                binding_name: name,
                                trace_id,
                            },
                        );
                    }
                    Ok(TriggerSubmitStatus::Suppressed) => {}
                    Err(error) => warn!(trigger = %name, %error, "chain trigger failed"),
                }
            } else {
                self.pending_chain
                    .entry(key.clone())
                    .or_default()
                    .push(PendingChainEvent {
                        binding_name: name,
                        required: config.finality,
                        event,
                    });
            }
        }
    }

    fn chain_log_seen_for_binding(&self, key: &ChainLogKey, name: &str) -> bool {
        self.pending_chain
            .get(key)
            .is_some_and(|pending| pending.iter().any(|entry| entry.binding_name == name))
            || self
                .delivered_chain
                .get(key)
                .is_some_and(|delivered| delivered.iter().any(|entry| entry.binding_name == name))
    }

    async fn promote_chain_finality(
        &mut self,
        chain_id: u64,
        block_hash: &str,
        finality: FinalityRequirement,
    ) {
        let block_hash = block_hash.to_ascii_lowercase();
        let keys: Vec<_> = self
            .pending_chain
            .keys()
            .filter(|key| key.chain_id == chain_id && key.block_hash == block_hash)
            .cloned()
            .collect();
        for key in keys {
            let Some(pending) = self.pending_chain.remove(&key) else {
                continue;
            };
            let mut retained = Vec::new();
            for mut entry in pending {
                if finality < entry.required {
                    retained.push(entry);
                    continue;
                }
                if let Some(payload) = entry.event.payload.as_object_mut() {
                    payload.insert("finality".to_string(), json!(finality));
                }
                let trace_id = entry.event.trace_id.clone();
                match self.submit_event(entry.event, false).await {
                    Ok(TriggerSubmitStatus::Started | TriggerSubmitStatus::Queued) => {
                        self.delivered_chain.entry(key.clone()).or_default().push(
                            DeliveredChainEvent {
                                binding_name: entry.binding_name,
                                trace_id,
                            },
                        );
                    }
                    Ok(TriggerSubmitStatus::Suppressed) => {}
                    Err(error) => {
                        warn!(trigger = %entry.binding_name, %error, "finalized chain trigger failed");
                    }
                }
            }
            if !retained.is_empty() {
                self.pending_chain.insert(key, retained);
            }
        }
    }

    async fn invalidate_chain_reorg(&mut self, chain_id: u64, orphaned_hashes: &[String]) {
        let orphaned: HashSet<_> = orphaned_hashes
            .iter()
            .map(|hash| hash.to_ascii_lowercase())
            .collect();
        self.pending_chain
            .retain(|key, _| key.chain_id != chain_id || !orphaned.contains(&key.block_hash));

        let invalidated: Vec<_> = self
            .delivered_chain
            .keys()
            .filter(|key| key.chain_id == chain_id && orphaned.contains(&key.block_hash))
            .cloned()
            .collect();
        for key in invalidated {
            let delivered = self.delivered_chain.remove(&key).unwrap_or_default();
            for entry in delivered {
                if let Some(binding) = self
                    .bindings
                    .get(&entry.binding_name)
                    .map(|active| active.binding.clone())
                {
                    self.emit_lifecycle(
                        &binding,
                        TriggerEventKind::Error,
                        Some(entry.trace_id),
                        json!({
                            "phase": "chain_reorg",
                            "reorg_invalidated": true,
                            "chain_id": chain_id,
                            "block_number": key.block_number,
                            "block_hash": key.block_hash,
                            "tx_hash": key.tx_hash,
                            "log_index": key.log_index,
                        }),
                    )
                    .await;
                }
            }
        }
    }

    async fn persist_fired_event(
        &self,
        binding: &TriggerBinding,
        event: &TriggerEvent,
    ) -> Result<()> {
        let Some(state) = self.state.upgrade() else {
            anyhow::bail!("server state dropped");
        };
        let directory = state.layout.triggers_dir().join("events");
        tokio::fs::create_dir_all(&directory).await?;
        let path = directory.join(event_file_name(&binding.name, &event.trace_id));
        let temporary = path.with_extension("json.tmp");
        let bytes = serde_json::to_vec_pretty(event)?;
        tokio::fs::write(&temporary, bytes).await?;
        tokio::fs::rename(&temporary, &path).await?;
        Ok(())
    }

    async fn emit_lifecycle(
        &self,
        binding: &TriggerBinding,
        kind: TriggerEventKind,
        trace_id: Option<String>,
        detail: Value,
    ) {
        let event = TriggerLifecycleEvent::new(binding, kind, trace_id, detail);
        let Some(state) = self.state.upgrade() else {
            return;
        };
        let path = state.layout.triggers_dir().join("lifecycle.jsonl");
        if let Some(parent) = path.parent()
            && let Err(error) = tokio::fs::create_dir_all(parent).await
        {
            warn!(%error, "failed to create trigger lifecycle directory");
        }
        match tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .await
        {
            Ok(mut file) => {
                if let Ok(mut bytes) = serde_json::to_vec(&event) {
                    bytes.push(b'\n');
                    if let Err(error) = file.write_all(&bytes).await {
                        warn!(%error, "failed to append trigger lifecycle event");
                    }
                }
            }
            Err(error) => warn!(%error, "failed to open trigger lifecycle log"),
        }
        state.event_bus.publish(ServerEvent::TriggerLifecycle {
            event: event.clone(),
        });
        let graduates = TriggerGraduationPolicy::should_graduate(&kind);
        let mut signal = Signal::builder(Kind::Custom(event.topic.clone()))
            .body(Body::Json(
                serde_json::to_value(&event).unwrap_or_else(|_| json!({})),
            ))
            .provenance(Provenance::agent("trigger-runtime"))
            .tag("trigger", event.trigger_name.clone())
            .tag("topic", event.topic.clone());
        if let Some(space_id) = binding.space.as_deref() {
            signal = signal.tag("space_id", space_id);
        }
        if graduates {
            signal = signal.status(SignalStatus::Consolidated);
        }
        let signal = signal.build();
        let pulse = signal.to_pulse(
            Topic::new(event.topic.clone()),
            state.pulse_bus.total_published(),
        );
        if let Err(error) = roko_core::Bus::publish(state.pulse_bus.as_ref(), pulse) {
            warn!(trigger = %binding.name, %error, "failed to publish trigger lifecycle Pulse");
        }
        if graduates {
            if let Err(error) = state.signal_store.put(signal).await {
                warn!(trigger = %binding.name, %error, "failed to graduate trigger lifecycle event");
            }
        }
    }

    async fn shutdown(&mut self) {
        let names: Vec<_> = self.bindings.keys().cloned().collect();
        for name in names {
            if let Some(mut active) = self.bindings.remove(&name) {
                self.disarm(&mut active, "server shutdown").await;
            }
        }
    }
}

enum FlowOutcome {
    Completed {
        success: bool,
        output: Option<String>,
    },
    Failed(String),
    Cancelled,
}

fn start_event_observer(state: Weak<AppState>, handle: TriggerRuntimeHandle) {
    tokio::spawn(async move {
        let Some(state) = state.upgrade() else {
            return;
        };
        let mut receiver = state.event_bus.subscribe();
        loop {
            tokio::select! {
                _ = state.cancel.cancelled() => break,
                result = receiver.recv() => match result {
                    Ok(envelope) => {
                        if handle.sender.send(Command::Observe {
                            event: envelope.payload,
                            sequence: envelope.seq,
                        }).await.is_err() {
                            break;
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(skipped)) => {
                        warn!(skipped, "trigger event observer lagged");
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                }
            }
        }
    });
}

fn start_pulse_observer(state: Weak<AppState>, handle: TriggerRuntimeHandle) {
    let Some(state) = state.upgrade() else {
        return;
    };
    let Ok(mut receiver) = roko_core::Bus::subscribe(state.pulse_bus.as_ref(), TopicFilter::All)
    else {
        warn!("failed to subscribe trigger coordinator to Pulse Bus");
        return;
    };
    tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = state.cancel.cancelled() => break,
                pulse = receiver.recv() => {
                    let Some(pulse) = pulse else {
                        break;
                    };
                    if handle.sender.send(Command::ObservePulse { pulse }).await.is_err() {
                        break;
                    }
                }
            }
        }
    });
}

fn start_shutdown_observer(state: Weak<AppState>, handle: TriggerRuntimeHandle) {
    tokio::spawn(async move {
        let Some(state) = state.upgrade() else {
            return;
        };
        state.cancel.cancelled().await;
        let _ = handle.sender.send(Command::Shutdown).await;
    });
}

fn start_disk_reloader(state: Weak<AppState>, handle: TriggerRuntimeHandle) {
    tokio::spawn(async move {
        let Some(state) = state.upgrade() else {
            return;
        };
        let mut interval = tokio::time::interval(RELOAD_INTERVAL);
        loop {
            tokio::select! {
                _ = state.cancel.cancelled() => break,
                _ = interval.tick() => {}
            }
            match TriggerBinding::load_all(&state.layout.triggers_dir()) {
                Ok(bindings) => {
                    *state.trigger_bindings.write().await = bindings
                        .iter()
                        .cloned()
                        .map(|binding| (binding.name.clone(), binding))
                        .collect();
                    if handle.reconcile(bindings).await.is_err() {
                        break;
                    }
                }
                Err(error) => {
                    warn!(%error, "trigger reload rejected; retaining last valid snapshot")
                }
            }
            claim_inbox(&state, &handle).await;
        }
    });
}

async fn claim_inbox(state: &AppState, handle: &TriggerRuntimeHandle) {
    let inbox = state.layout.triggers_dir().join("inbox");
    let mut entries = match tokio::fs::read_dir(&inbox).await {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return,
        Err(error) => {
            warn!(%error, "failed to read trigger inbox");
            return;
        }
    };
    while let Ok(Some(entry)) = entries.next_entry().await {
        let path = entry.path();
        if path.extension().and_then(|extension| extension.to_str()) != Some("json") {
            continue;
        }
        let event = match tokio::fs::read(&path)
            .await
            .ok()
            .and_then(|bytes| serde_json::from_slice::<TriggerEvent>(&bytes).ok())
        {
            Some(event) => event,
            None => {
                warn!(path = %path.display(), "invalid trigger inbox event");
                let invalid = path.with_extension("json.invalid");
                let _ = tokio::fs::rename(&path, invalid).await;
                continue;
            }
        };
        match handle.submit(event).await {
            Ok(_) => {
                if let Err(error) = tokio::fs::remove_file(&path).await {
                    warn!(path = %path.display(), %error, "failed to remove claimed trigger event");
                }
            }
            Err(error) => debug!(path = %path.display(), %error, "trigger inbox event retained"),
        }
    }
}

fn start_source(
    binding: &TriggerBinding,
    state: Weak<AppState>,
    sender: mpsc::Sender<Command>,
) -> Result<Option<(CancellationToken, u64)>> {
    let generation = now_ms().wrapping_add(rand_generation());
    let cancel = CancellationToken::new();
    let source_cancel = cancel.clone();
    let name = binding.name.clone();
    let (signal_sender, mut signal_receiver) = mpsc::channel(SOURCE_CHANNEL_CAPACITY);

    let source: Box<dyn EventSource> = match &binding.kind {
        TriggerKind::Cron(config) => Box::new(TimezoneCronEventSource::new(
            &binding.name,
            &config.expression,
            config.timezone.as_deref(),
        )?),
        TriggerKind::FileWatch(config) => {
            let Some(state) = state.upgrade() else {
                anyhow::bail!("server state dropped");
            };
            Box::new(FileWatchEventSource::from_paths([WatcherPathConfig {
                directory: state.workdir.join(&config.path),
                include: config.glob.iter().cloned().collect(),
                exclude: Vec::new(),
            }]))
        }
        _ => return Ok(None),
    };

    let source_name = name.clone();
    let stop_sender = sender.clone();
    let stop_cancel = cancel.clone();
    tokio::spawn(async move {
        let result = source.start(signal_sender, stop_cancel.clone()).await;
        if !stop_cancel.is_cancelled() {
            let _ = stop_sender
                .send(Command::SourceStopped {
                    name: source_name,
                    generation,
                    error: result.err().map(|error| error.to_string()),
                })
                .await;
        }
    });

    let event_binding = binding.clone();
    tokio::spawn(async move {
        while let Some(signal) = signal_receiver.recv().await {
            let Some(event) = source_signal_event(&event_binding, signal) else {
                continue;
            };
            if sender
                .send(Command::Submit { event, reply: None })
                .await
                .is_err()
            {
                break;
            }
        }
    });
    Ok(Some((source_cancel, generation)))
}

fn source_signal_event(binding: &TriggerBinding, signal: Signal) -> Option<TriggerEvent> {
    let payload = body_value(&signal.body);
    let source = match &binding.kind {
        TriggerKind::Cron(config) => TriggerSource::Cron {
            expression: config.expression.clone(),
        },
        TriggerKind::FileWatch(config) => {
            let event = match signal.kind.to_string().as_str() {
                roko_core::FS_CREATED => FileWatchEvent::Created,
                roko_core::FS_MODIFIED => FileWatchEvent::Modified,
                roko_core::FS_DELETED => FileWatchEvent::Deleted,
                roko_core::FS_RENAMED => FileWatchEvent::Renamed,
                _ => FileWatchEvent::Any,
            };
            if !config.events.is_empty()
                && !config.events.contains(&FileWatchEvent::Any)
                && !config.events.contains(&event)
            {
                return None;
            }
            let path = payload
                .get("path")
                .and_then(Value::as_str)
                .map(PathBuf::from)
                .unwrap_or_else(|| config.path.clone());
            TriggerSource::FileWatch { path, event }
        }
        _ => return None,
    };
    Some(TriggerEvent::new(
        binding.name.clone(),
        payload,
        source,
        Uuid::new_v4().to_string(),
    ))
}

fn source_label(kind: &TriggerKind) -> &'static str {
    match kind {
        TriggerKind::Cron(_) => "cron",
        TriggerKind::Webhook(_) => "webhook",
        TriggerKind::FileWatch(_) => "file_watch",
        TriggerKind::Bus(_) => "bus",
        TriggerKind::ChainEvent(_) => "chain_event",
        TriggerKind::Manual => "manual",
        TriggerKind::SignalPattern(_) => "signal_pattern",
    }
}

fn body_value(body: &Body) -> Value {
    match body {
        Body::Json(value) => value.clone(),
        Body::Text(text) => Value::String(text.clone()),
        Body::Bytes(bytes) => json!({
            "base64": base64::engine::general_purpose::STANDARD.encode(bytes),
        }),
        Body::Empty => json!({}),
    }
}

/// Resolve a scoped trigger against the workspace's authoritative `[space]`
/// policy and the target Graph's allow-list. Scoped triggers fail closed when
/// either declaration is absent.
fn resolve_trigger_execution_scope(
    state: &AppState,
    binding: &TriggerBinding,
    event: &mut TriggerEvent,
) -> Result<TriggerExecutionScope> {
    let Some(space_id) = binding.space.as_deref() else {
        return Ok(TriggerExecutionScope {
            space_id: event.space_id.clone(),
            capabilities: None,
        });
    };

    if matches!(event.source, TriggerSource::Bus { .. }) && event.space_id.is_none() {
        anyhow::bail!(
            "space-scoped bus trigger '{}' rejected an event without a Space partition",
            binding.name
        );
    }
    if let Some(origin) = event.space_id.as_deref() {
        anyhow::ensure!(
            origin == space_id,
            "trigger '{}' is scoped to Space '{space_id}' but event originated in Space '{origin}'",
            binding.name
        );
    } else {
        event.space_id = Some(space_id.to_string());
    }

    let config_path = state.workdir.join("roko.toml");
    let config_text = std::fs::read_to_string(&config_path)
        .with_context(|| format!("read Space policy {}", config_path.display()))?;
    let config: toml::Value = toml::from_str(&config_text)
        .with_context(|| format!("parse Space policy {}", config_path.display()))?;
    let space = config
        .get("space")
        .and_then(toml::Value::as_table)
        .context("space-scoped triggers require a [space] policy in roko.toml")?;
    let configured_id = space
        .get("id")
        .and_then(toml::Value::as_str)
        .context("[space].id is required for space-scoped triggers")?;
    anyhow::ensure!(
        configured_id == space_id,
        "trigger Space '{space_id}' does not match workspace Space '{configured_id}'"
    );

    let visible_graphs = space
        .get("visible_graphs")
        .or_else(|| space.get("graphs"))
        .and_then(toml::Value::as_array)
        .context("[space].visible_graphs is required for space-scoped triggers")?;
    let graph_visible = visible_graphs.iter().any(|pattern| {
        pattern
            .as_str()
            .is_some_and(|pattern| wildcard_matches(pattern, &binding.graph))
    });
    anyhow::ensure!(
        graph_visible,
        "Graph '{}' is not visible in Space '{space_id}'",
        binding.graph
    );

    let workspace_root = state
        .workdir
        .canonicalize()
        .with_context(|| format!("resolve workspace root {}", state.workdir.display()))?;
    let graph_path = state.workdir.join(&binding.graph);
    let canonical_graph = graph_path
        .canonicalize()
        .with_context(|| format!("resolve trigger Graph {}", graph_path.display()))?;
    anyhow::ensure!(
        canonical_graph.starts_with(&workspace_root),
        "trigger Graph resolves outside the workspace"
    );

    let space_capabilities = space
        .get("capabilities")
        .and_then(toml::Value::as_table)
        .map(capability_set_from_table)
        .context("[space.capabilities] is required for space-scoped triggers")?;
    let graph_capabilities = load_graph_capabilities(&canonical_graph)?;
    let effective = roko_core::capabilities::effective_capabilities(
        &CellCapabilities::all(),
        &GraphAllowList(graph_capabilities),
        &SpaceGrant(space_capabilities),
    );
    anyhow::ensure!(
        effective.contains(Capability::Execute),
        "Space/Graph capability intersection denies Graph execution"
    );

    Ok(TriggerExecutionScope {
        space_id: Some(space_id.to_string()),
        capabilities: Some(effective),
    })
}

fn load_graph_capabilities(path: &Path) -> Result<CapabilitySet> {
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("read trigger Graph capability policy {}", path.display()))?;
    let document: toml::Value = toml::from_str(&text)
        .with_context(|| format!("parse trigger Graph capability policy {}", path.display()))?;
    let allow = document
        .get("graph")
        .and_then(|graph| graph.get("capabilities"))
        .and_then(|capabilities| capabilities.get("allow"))
        .and_then(toml::Value::as_array)
        .context("space-scoped Graphs require [graph.capabilities].allow")?;
    let names = allow.iter().flat_map(|entry| match entry {
        toml::Value::String(name) => vec![name.as_str()],
        toml::Value::Table(table) => table.keys().map(String::as_str).collect(),
        _ => Vec::new(),
    });
    Ok(capability_set_from_names(names))
}

fn capability_set_from_table(table: &toml::map::Map<String, toml::Value>) -> CapabilitySet {
    capability_set_from_names(
        table
            .iter()
            .filter(|(_, value)| capability_grant_enabled(value))
            .map(|(name, _)| name.as_str()),
    )
}

fn capability_grant_enabled(value: &toml::Value) -> bool {
    match value {
        toml::Value::Boolean(enabled) => *enabled,
        toml::Value::Table(_) => true,
        toml::Value::Array(values) => !values.is_empty(),
        toml::Value::String(value) => !value.trim().is_empty(),
        toml::Value::Integer(value) => *value != 0,
        toml::Value::Float(value) => *value != 0.0,
        toml::Value::Datetime(_) => false,
    }
}

fn capability_set_from_names<'a>(names: impl IntoIterator<Item = &'a str>) -> CapabilitySet {
    let mut capabilities = Vec::new();
    for name in names {
        let normalized = name
            .chars()
            .filter(|character| character.is_ascii_alphanumeric())
            .flat_map(char::to_lowercase)
            .collect::<String>();
        let mapped: &[Capability] = match normalized.as_str() {
            "read" => &[Capability::Read],
            "fsread" | "filesystemread" => &[Capability::Read, Capability::FileSystem],
            "write" => &[Capability::Write],
            "fswrite" | "filesystemwrite" => &[Capability::Write, Capability::FileSystem],
            "execute" | "shell" | "llm" => &[Capability::Execute],
            "network" | "net" => &[Capability::Network],
            "filesystem" => &[Capability::FileSystem],
            "secret" | "secrets" => &[Capability::Secret],
            "chainwrite" => &[Capability::Write, Capability::Network],
            _ => &[],
        };
        capabilities.extend_from_slice(mapped);
    }
    CapabilitySet::from(capabilities)
}

fn payload_matches(payload: &Value, filter: Option<&roko_core::trigger::TriggerFilter>) -> bool {
    let Some(matches) = filter.and_then(|filter| filter.matches.as_ref()) else {
        return true;
    };
    matches.iter().all(|(path, expected)| {
        let actual = lookup_value(payload, path);
        match (actual, expected) {
            (Some(Value::String(actual)), Value::String(expected)) => actual.contains(expected),
            (Some(actual), expected) => actual == expected,
            _ => false,
        }
    })
}

fn lookup_value<'a>(value: &'a Value, path: &str) -> Option<&'a Value> {
    let path = path.trim();
    if path == "$" || path.is_empty() {
        return Some(value);
    }
    if path.starts_with('/') {
        return value.pointer(path);
    }

    let mut remaining = path.strip_prefix('$').unwrap_or(path);
    let mut current = value;
    while !remaining.is_empty() {
        if let Some(rest) = remaining.strip_prefix('.') {
            remaining = rest;
            continue;
        }
        if let Some(rest) = remaining.strip_prefix('[') {
            let end = rest.find(']')?;
            let selector = &rest[..end];
            let selector = selector
                .strip_prefix('\'')
                .and_then(|value| value.strip_suffix('\''))
                .or_else(|| {
                    selector
                        .strip_prefix('"')
                        .and_then(|value| value.strip_suffix('"'))
                });
            current = if let Some(key) = selector {
                current.get(key)?
            } else {
                current.get(rest[..end].parse::<usize>().ok()?)?
            };
            remaining = &rest[end + 1..];
            continue;
        }
        let end = remaining.find(['.', '[']).unwrap_or(remaining.len());
        current = current.get(&remaining[..end])?;
        remaining = &remaining[end..];
    }
    Some(current)
}

fn map_event_input(binding: &TriggerBinding, mut event: TriggerEvent) -> Result<TriggerEvent> {
    let Some(mapping) = &binding.input_mapping else {
        return Ok(event);
    };
    let original = event.payload.clone();
    let mut inputs = serde_json::Map::new();
    for field in &mapping.mappings {
        if let Some(value) = lookup_value(&original, &field.from) {
            let value = apply_input_transform(value.clone(), field.transform.as_deref())
                .with_context(|| {
                    format!("apply input mapping '{}' -> '{}'", field.from, field.to)
                })?;
            insert_mapped_value(&mut inputs, &field.to, value)?;
        }
    }
    event.payload = json!({"event": original, "inputs": inputs});
    Ok(event)
}

fn insert_mapped_value(
    root: &mut serde_json::Map<String, Value>,
    path: &str,
    value: Value,
) -> Result<()> {
    let components: Vec<_> = path
        .trim()
        .trim_start_matches("$.")
        .split('.')
        .filter(|component| !component.is_empty())
        .collect();
    anyhow::ensure!(!components.is_empty(), "mapping target must not be empty");
    let mut current = root;
    for component in &components[..components.len() - 1] {
        let entry = current
            .entry((*component).to_string())
            .or_insert_with(|| Value::Object(serde_json::Map::new()));
        current = entry
            .as_object_mut()
            .with_context(|| format!("mapping target '{path}' conflicts with a scalar"))?;
    }
    current.insert(components[components.len() - 1].to_string(), value);
    Ok(())
}

fn apply_input_transform(value: Value, transform: Option<&str>) -> Result<Value> {
    let Some(transform) = transform.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(value);
    };
    match transform.to_ascii_lowercase().as_str() {
        "identity" => Ok(value),
        "trim" => Ok(Value::String(
            value
                .as_str()
                .context("trim transform requires a string")?
                .trim()
                .to_string(),
        )),
        "lower" | "lowercase" => Ok(Value::String(
            value
                .as_str()
                .context("lowercase transform requires a string")?
                .to_lowercase(),
        )),
        "upper" | "uppercase" => Ok(Value::String(
            value
                .as_str()
                .context("uppercase transform requires a string")?
                .to_uppercase(),
        )),
        "string" | "to_string" => Ok(Value::String(match value {
            Value::String(value) => value,
            other => other.to_string(),
        })),
        "number" | "to_number" => match value {
            Value::Number(_) => Ok(value),
            Value::String(value) => value
                .parse::<serde_json::Number>()
                .map(Value::Number)
                .context("number transform requires a JSON number"),
            _ => anyhow::bail!("number transform requires a string or number"),
        },
        "boolean" | "to_bool" => match value {
            Value::Bool(_) => Ok(value),
            Value::String(value) if value.eq_ignore_ascii_case("true") => Ok(Value::Bool(true)),
            Value::String(value) if value.eq_ignore_ascii_case("false") => Ok(Value::Bool(false)),
            _ => anyhow::bail!("boolean transform requires true or false"),
        },
        "json" | "parse_json" => match value {
            Value::String(value) => serde_json::from_str(&value).context("parse JSON transform"),
            _ => anyhow::bail!("JSON transform requires a string"),
        },
        _ => anyhow::bail!("unsupported input transform '{transform}'"),
    }
}

fn queue_depth(policy: &ConcurrencyPolicy) -> usize {
    match policy {
        ConcurrencyPolicy::Queue { max_depth } => max_depth
            .unwrap_or(DEFAULT_QUEUE_DEPTH)
            .clamp(1, MAX_QUEUE_DEPTH),
        _ => DEFAULT_QUEUE_DEPTH,
    }
}

fn parallel_limit(policy: &ConcurrencyPolicy) -> usize {
    match policy {
        ConcurrencyPolicy::Parallel { max_concurrent } => max_concurrent
            .unwrap_or(DEFAULT_PARALLEL_LIMIT)
            .clamp(1, MAX_PARALLEL_LIMIT),
        _ => 1,
    }
}

fn wildcard_matches(pattern: &str, candidate: &str) -> bool {
    if pattern == "*" {
        return true;
    }
    let parts: Vec<_> = pattern.split('*').collect();
    if parts.len() == 1 {
        return pattern == candidate;
    }
    let mut remainder = candidate;
    for (index, part) in parts.iter().enumerate() {
        if part.is_empty() {
            continue;
        }
        if index == 0 && !pattern.starts_with('*') {
            let Some(next) = remainder.strip_prefix(part) else {
                return false;
            };
            remainder = next;
        } else if let Some(position) = remainder.find(part) {
            remainder = &remainder[position + part.len()..];
        } else {
            return false;
        }
    }
    pattern.ends_with('*') || remainder.is_empty()
}

fn chain_event_matches(signature: &str, event_name: &str) -> bool {
    signature == event_name
        || signature
            .split_once('(')
            .is_some_and(|(name, _)| name == event_name)
}

fn decode_chain_log(
    config: &roko_core::trigger::ChainEventTrigger,
    topics: &[String],
    data: &str,
) -> Result<Option<Value>> {
    let expected = AbiEvent::parse(&config.event_signature)
        .with_context(|| format!("parse event signature '{}'", config.event_signature))?;
    let event = if let Some(abi) = config.abi.as_ref() {
        find_abi_event(abi, expected.selector()).with_context(|| {
            format!(
                "ABI does not define event signature '{}'",
                config.event_signature
            )
        })?
    } else {
        expected
    };

    let parsed_topics = topics
        .iter()
        .map(|topic| {
            B256::from_str(topic).with_context(|| format!("invalid EVM log topic '{topic}'"))
        })
        .collect::<Result<Vec<_>>>()?;
    if !event.anonymous && parsed_topics.first().copied() != Some(event.selector()) {
        return Ok(None);
    }
    let data = data.strip_prefix("0x").unwrap_or(data);
    let data = decode_hex(data).context("invalid EVM log data hex")?;
    let decoded = event
        .decode_log_parts(parsed_topics, &data)
        .with_context(|| format!("decode EVM event '{}'", event.signature()))?;

    let mut indexed = decoded.indexed.into_iter();
    let mut body = decoded.body.into_iter();
    let mut object = serde_json::Map::new();
    for (index, parameter) in event.inputs.iter().enumerate() {
        let value = if parameter.indexed {
            indexed.next()
        } else {
            body.next()
        }
        .with_context(|| format!("decoded event field {index} is missing"))?;
        let name = if parameter.name.is_empty() {
            format!("arg_{index}")
        } else {
            parameter.name.clone()
        };
        object.insert(name, dyn_sol_value_json(value));
    }
    Ok(Some(Value::Object(object)))
}

fn find_abi_event(abi: &Value, selector: B256) -> Option<AbiEvent> {
    let values = abi
        .get("abi")
        .and_then(Value::as_array)
        .or_else(|| abi.as_array());
    if let Some(values) = values {
        return values.iter().find_map(|value| {
            let event = serde_json::from_value::<AbiEvent>(value.clone()).ok()?;
            (event.selector() == selector).then_some(event)
        });
    }
    serde_json::from_value::<AbiEvent>(abi.clone())
        .ok()
        .filter(|event| event.selector() == selector)
}

fn dyn_sol_value_json(value: DynSolValue) -> Value {
    match value {
        DynSolValue::Bool(value) => Value::Bool(value),
        DynSolValue::Int(value, _) => Value::String(value.to_string()),
        DynSolValue::Uint(value, _) => Value::String(value.to_string()),
        DynSolValue::FixedBytes(value, length) => {
            Value::String(format!("0x{}", bytes_hex(&value.as_slice()[..length])))
        }
        DynSolValue::Address(value) => Value::String(format!("{value:#x}")),
        DynSolValue::Function(value) => Value::String(format!("{value:#x}")),
        DynSolValue::Bytes(value) => Value::String(format!("0x{}", bytes_hex(&value))),
        DynSolValue::String(value) => Value::String(value),
        DynSolValue::Array(values)
        | DynSolValue::FixedArray(values)
        | DynSolValue::Tuple(values) => {
            Value::Array(values.into_iter().map(dyn_sol_value_json).collect())
        }
    }
}

fn bytes_hex(bytes: &[u8]) -> String {
    bytes.iter().fold(
        String::with_capacity(bytes.len() * 2),
        |mut output, byte| {
            write!(output, "{byte:02x}").expect("writing to a String cannot fail");
            output
        },
    )
}

fn server_bus_projection(event: &ServerEvent) -> Option<(String, Value)> {
    match event {
        ServerEvent::WebhookReceived { .. }
        | ServerEvent::ChainContractEvent { .. }
        | ServerEvent::TriggerFired { .. }
        | ServerEvent::TriggerLifecycle { .. } => None,
        ServerEvent::FeedTick { topic, payload, .. } => Some((topic.clone(), payload.clone())),
        _ => {
            let payload = serde_json::to_value(event).ok()?;
            let event_type = payload.get("type")?.as_str()?;
            Some((format!("server.{event_type}"), payload))
        }
    }
}

fn event_file_name(name: &str, trace_id: &str) -> String {
    let digest = Sha256::digest(trace_id.as_bytes());
    let suffix = bytes_hex(&digest[..12]);
    format!("{name}-{suffix}.json")
}

fn load_seen_traces(directory: &Path) -> HashSet<(String, String)> {
    let events = directory.join("events");
    let Ok(entries) = std::fs::read_dir(events) else {
        return HashSet::new();
    };
    entries
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| std::fs::read(entry.path()).ok())
        .filter_map(|bytes| serde_json::from_slice::<TriggerEvent>(&bytes).ok())
        .map(|event| (event.trigger_id, event.trace_id))
        .collect()
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}

fn rand_generation() -> u64 {
    let bytes = Uuid::new_v4();
    u64::from_le_bytes(bytes.as_bytes()[..8].try_into().unwrap_or_default())
}

/// Resolve a trigger secret reference without exposing its value in events.
pub(crate) fn resolve_trigger_secret(state: &AppState, reference: &SecretRef) -> Result<String> {
    match reference {
        SecretRef::Env { var } => std::env::var(var)
            .with_context(|| format!("trigger secret environment variable '{var}' is unavailable")),
        SecretRef::File { path } => {
            let path = if path.is_absolute() {
                path.clone()
            } else {
                state.workdir.join(path)
            };
            std::fs::read_to_string(&path)
                .with_context(|| format!("read trigger secret file {}", path.display()))
                .map(|secret| secret.trim_end().to_string())
        }
        SecretRef::Store { key } => {
            let namespace = Namespace::parse(key)
                .with_context(|| format!("invalid trigger secret store key '{key}'"))?;
            let store = FileStore::open(state.layout.root().join("secrets.toml"))?;
            store
                .get(&namespace)?
                .with_context(|| format!("trigger secret store key '{key}' is unavailable"))
        }
    }
}

/// Verify webhook authentication for a binding.
pub(crate) fn verify_webhook_auth(
    state: &AppState,
    binding: &TriggerBinding,
    headers: &axum::http::HeaderMap,
    body: &[u8],
    client_identity: Option<&crate::trigger_tls::VerifiedClientIdentity>,
) -> Result<()> {
    use hmac::{Hmac, Mac};

    match binding.auth.as_ref().unwrap_or(&TriggerAuth::None) {
        TriggerAuth::None => Ok(()),
        TriggerAuth::BearerToken { secret } => {
            let expected = resolve_trigger_secret(state, secret)?;
            let supplied = headers
                .get(axum::http::header::AUTHORIZATION)
                .and_then(|value| value.to_str().ok())
                .and_then(|value| value.strip_prefix("Bearer "))
                .context("missing bearer token")?;
            let mut verifier = Hmac::<Sha256>::new_from_slice(expected.as_bytes())?;
            verifier.update(b"roko-trigger-bearer");
            let expected_mac = verifier.finalize().into_bytes();
            let mut supplied_verifier = Hmac::<Sha256>::new_from_slice(supplied.as_bytes())?;
            supplied_verifier.update(b"roko-trigger-bearer");
            supplied_verifier.verify_slice(&expected_mac)?;
            Ok(())
        }
        TriggerAuth::HmacSha256 { secret, header } => {
            let secret = resolve_trigger_secret(state, secret)?;
            let signature = headers
                .get(header)
                .and_then(|value| value.to_str().ok())
                .context("missing HMAC signature")?
                .strip_prefix("sha256=")
                .unwrap_or_else(|| {
                    headers
                        .get(header)
                        .and_then(|value| value.to_str().ok())
                        .unwrap_or_default()
                });
            let decoded = decode_hex(signature).context("invalid HMAC signature encoding")?;
            let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes())?;
            mac.update(body);
            mac.verify_slice(&decoded)?;
            Ok(())
        }
        TriggerAuth::MutualTls { .. } => client_identity
            .context("webhook requires a client certificate verified by the TLS transport")
            .map(|_| ()),
    }
}

fn decode_hex(value: &str) -> Option<Vec<u8>> {
    if value.len() % 2 != 0 {
        return None;
    }
    value
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let high = (pair[0] as char).to_digit(16)?;
            let low = (pair[1] as char).to_digit(16)?;
            Some(((high << 4) | low) as u8)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::collections::BTreeMap;
    use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

    use async_trait::async_trait;
    use roko_core::config::schema::RokoConfig;

    use crate::deploy::create_backend;
    use crate::runtime::{
        CliRuntime, DashboardInfo, PlanExecutionResult, RunResult, SessionStatusInfo,
    };

    #[derive(Default)]
    struct RecordingRuntime {
        calls: tokio::sync::Mutex<Vec<TriggerEvent>>,
        delay_ms: AtomicU64,
        active: Arc<AtomicUsize>,
        max_active: Arc<AtomicUsize>,
        changed: tokio::sync::Notify,
    }

    struct ActiveGuard {
        active: Arc<AtomicUsize>,
    }

    impl Drop for ActiveGuard {
        fn drop(&mut self) {
            self.active.fetch_sub(1, Ordering::SeqCst);
        }
    }

    impl RecordingRuntime {
        fn with_delay(delay_ms: u64) -> Self {
            Self {
                delay_ms: AtomicU64::new(delay_ms),
                ..Self::default()
            }
        }

        async fn wait_for_calls(&self, expected: usize) {
            tokio::time::timeout(Duration::from_secs(3), async {
                loop {
                    if self.calls.lock().await.len() >= expected {
                        return;
                    }
                    self.changed.notified().await;
                }
            })
            .await
            .expect("trigger calls");
        }
    }

    #[async_trait]
    impl CliRuntime for RecordingRuntime {
        async fn run_once(&self, _workdir: &Path, _prompt: &str) -> Result<RunResult> {
            unreachable!("trigger tests use run_trigger_graph")
        }

        async fn run_trigger_graph(
            &self,
            _workdir: &Path,
            _graph: &Path,
            event: &TriggerEvent,
        ) -> Result<PlanExecutionResult> {
            let current = self.active.fetch_add(1, Ordering::SeqCst) + 1;
            self.max_active.fetch_max(current, Ordering::SeqCst);
            let _guard = ActiveGuard {
                active: Arc::clone(&self.active),
            };
            self.calls.lock().await.push(event.clone());
            self.changed.notify_waiters();
            tokio::time::sleep(Duration::from_millis(self.delay_ms.load(Ordering::SeqCst))).await;
            Ok(PlanExecutionResult {
                success: true,
                output_text: Some("graph executed".to_string()),
                gate_results: Vec::new(),
            })
        }

        fn session_status(&self, workdir: PathBuf) -> SessionStatusInfo {
            SessionStatusInfo {
                session_id: None,
                workdir,
                daemon_running: false,
                signal_count: None,
                episode_count: None,
                last_episode_passed: None,
            }
        }

        fn dashboard_scaffold(&self, _workdir: &Path) -> DashboardInfo {
            DashboardInfo {
                rendered: String::new(),
            }
        }
    }

    fn manual_binding(name: &str) -> TriggerBinding {
        TriggerBinding::new(name, TriggerKind::Manual, "graphs/test.toml")
    }

    fn utc(value: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(value)
            .expect("valid timestamp")
            .with_timezone(&Utc)
    }

    #[test]
    fn cron_uses_iana_timezone_and_dst_offsets() {
        let source =
            TimezoneCronEventSource::new("business-open", "0 0 9 * * *", Some("America/New_York"))
                .expect("timezone schedule");

        assert_eq!(
            source.next_after(utc("2026-01-15T13:59:59Z")).unwrap(),
            utc("2026-01-15T14:00:00Z")
        );
        assert_eq!(
            source.next_after(utc("2026-07-15T12:59:59Z")).unwrap(),
            utc("2026-07-15T13:00:00Z")
        );
    }

    #[test]
    fn cron_rejects_unknown_timezone_and_skips_dst_gap() {
        assert!(
            TimezoneCronEventSource::new("bad-zone", "0 0 9 * * *", Some("Mars/Olympus")).is_err()
        );

        let source =
            TimezoneCronEventSource::new("dst-gap", "0 30 2 * * *", Some("America/New_York"))
                .expect("timezone schedule");
        assert_eq!(
            source.next_after(utc("2026-03-08T06:59:59Z")).unwrap(),
            utc("2026-03-09T06:30:00Z"),
            "the nonexistent spring-forward 02:30 must not become an imaginary instant"
        );
    }

    fn test_state(
        root: &Path,
        runtime: Arc<RecordingRuntime>,
        bindings: &[TriggerBinding],
    ) -> Arc<AppState> {
        let layout = roko_fs::RokoLayout::for_project(root);
        TriggerBinding::save_all(&layout.triggers_dir(), bindings).expect("save bindings");
        let deploy_backend =
            Arc::from(create_backend("manual", None, None, None).expect("manual backend"));
        Arc::new(
            AppState::new(
                root.to_path_buf(),
                runtime,
                RokoConfig::default(),
                deploy_backend,
            )
            .expect("app state"),
        )
    }

    fn manual_event(name: &str, value: u64) -> TriggerEvent {
        TriggerEvent::new(
            name.to_string(),
            json!({"value": value}),
            TriggerSource::Manual {
                user: "test".to_string(),
            },
            Uuid::new_v4().to_string(),
        )
    }

    async fn shutdown(state: &AppState) {
        state.cancel.cancel();
        tokio::time::sleep(Duration::from_millis(25)).await;
    }

    #[tokio::test]
    async fn startup_arms_and_manual_submit_executes_with_durable_evidence() {
        let directory = tempfile::tempdir().expect("tempdir");
        let runtime = Arc::new(RecordingRuntime::default());
        let state = test_state(
            directory.path(),
            Arc::clone(&runtime),
            &[manual_binding("one")],
        );
        assert!(state.trigger_bindings.read().await.contains_key("one"));

        let handle = ensure_trigger_runtime(&state).await;
        assert_eq!(
            handle.submit(manual_event("one", 7)).await.expect("submit"),
            TriggerSubmitStatus::Started
        );
        runtime.wait_for_calls(1).await;
        tokio::time::sleep(Duration::from_millis(50)).await;

        assert!(
            state
                .layout
                .triggers_dir()
                .join("lifecycle.jsonl")
                .is_file()
        );
        assert_eq!(
            std::fs::read_dir(state.layout.triggers_dir().join("events"))
                .expect("events")
                .count(),
            1
        );
        assert!(state.layout.root().join("engrams.jsonl").is_file());
        shutdown(&state).await;
    }

    fn erc20_transfer_binding(name: &str) -> TriggerBinding {
        TriggerBinding::new(
            name,
            TriggerKind::ChainEvent(roko_core::trigger::ChainEventTrigger {
                chain_id: 8453,
                contract: "0x000000000000000000000000000000000000cafe".to_string(),
                event_signature: "Transfer(address,address,uint256)".to_string(),
                abi: Some(json!([{
                    "anonymous": false,
                    "inputs": [
                        {"indexed": true, "name": "from", "type": "address"},
                        {"indexed": true, "name": "to", "type": "address"},
                        {"indexed": false, "name": "value", "type": "uint256"}
                    ],
                    "name": "Transfer",
                    "type": "event"
                }])),
                finality: FinalityRequirement::QuasiFinalized,
            }),
            "graphs/chain.toml",
        )
    }

    fn transfer_log(block_hash: &str, tx_hash: &str, log_index: u32) -> ServerEvent {
        ServerEvent::ChainLogObserved {
            chain_id: 8453,
            block_number: 100,
            block_hash: block_hash.to_string(),
            tx_hash: tx_hash.to_string(),
            log_index,
            contract: "0x000000000000000000000000000000000000CAFE".to_string(),
            topics: vec![
                "0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef".to_string(),
                "0x0000000000000000000000000000000000000000000000000000000000000011".to_string(),
                "0x0000000000000000000000000000000000000000000000000000000000000022".to_string(),
            ],
            data: "0x0000000000000000000000000000000000000000000000000000000000000064".to_string(),
            finality: FinalityRequirement::Reversible,
            removed: false,
        }
    }

    fn configure_test_chain(state: &AppState) {
        let mut config = state.load_roko_config().as_ref().clone();
        config.chain.chain_id = Some(8453);
        state.store_roko_config(config);
    }

    fn publish_watcher_transfer(
        state: &Arc<AppState>,
        block_number: u64,
        block_hash: &str,
        tx_hash: &str,
        log_index: u32,
    ) {
        crate::publish_chain_watcher_payload(
            state,
            "chain:log",
            serde_json::to_value(roko_chain::block_watcher::RawLogInfo {
                block_number,
                block_hash: block_hash.to_string(),
                tx_hash: tx_hash.to_string(),
                log_index,
                contract: "0x000000000000000000000000000000000000CAFE".to_string(),
                topics: vec![
                    "0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef"
                        .to_string(),
                    "0x0000000000000000000000000000000000000000000000000000000000000011"
                        .to_string(),
                    "0x0000000000000000000000000000000000000000000000000000000000000022"
                        .to_string(),
                ],
                data: "0x0000000000000000000000000000000000000000000000000000000000000064"
                    .to_string(),
            })
            .unwrap(),
        );
    }

    fn publish_watcher_block(state: &Arc<AppState>, number: u64, hash: &str, parent_hash: &str) {
        crate::publish_chain_watcher_payload(
            state,
            "chain:block",
            serde_json::to_value(roko_chain::block_watcher::BlockInfo {
                number,
                hash: hash.to_string(),
                parent_hash: parent_hash.to_string(),
                timestamp: number,
                gas_used: 0,
                gas_limit: 30_000_000,
                tx_count: 0,
                base_fee_per_gas: None,
            })
            .unwrap(),
        );
    }

    #[tokio::test]
    async fn bundled_watcher_raw_log_decodes_and_promotes_once_at_finality() {
        let directory = tempfile::tempdir().expect("tempdir");
        let runtime = Arc::new(RecordingRuntime::default());
        let state = test_state(
            directory.path(),
            Arc::clone(&runtime),
            &[erc20_transfer_binding("watcher-transfer")],
        );
        configure_test_chain(&state);
        ensure_trigger_runtime(&state).await;

        publish_watcher_transfer(&state, 100, "0xblock100", "0xtx", 0);
        publish_watcher_transfer(&state, 100, "0xBLOCK100", "0xTX", 0);
        publish_watcher_block(&state, 100, "0xblock100", "0xblock99");
        tokio::time::sleep(Duration::from_millis(30)).await;
        assert!(runtime.calls.lock().await.is_empty());

        let mut parent = "0xblock100".to_string();
        for number in 101..=112 {
            let hash = format!("0xblock{number}");
            publish_watcher_block(&state, number, &hash, &parent);
            parent = hash;
        }
        runtime.wait_for_calls(1).await;
        let calls = runtime.calls.lock().await;
        assert_eq!(calls.len(), 1, "duplicate watcher logs must be idempotent");
        assert_eq!(calls[0].payload["decoded"]["value"], json!("100"));
        assert_eq!(
            calls[0].payload["finality"],
            json!(FinalityRequirement::QuasiFinalized)
        );
        drop(calls);

        publish_watcher_block(&state, 112, "0xBLOCK112", "0xblock111");
        publish_watcher_transfer(&state, 100, "0xblock100", "0xtx", 0);
        tokio::time::sleep(Duration::from_millis(30)).await;
        assert_eq!(runtime.calls.lock().await.len(), 1);
        shutdown(&state).await;
    }

    #[tokio::test]
    async fn bundled_watcher_block_reorg_invalidates_delivered_log() {
        let directory = tempfile::tempdir().expect("tempdir");
        let runtime = Arc::new(RecordingRuntime::default());
        let mut binding = erc20_transfer_binding("watcher-reorg");
        let TriggerKind::ChainEvent(config) = &mut binding.kind else {
            unreachable!();
        };
        config.finality = FinalityRequirement::Reversible;
        let state = test_state(directory.path(), Arc::clone(&runtime), &[binding]);
        configure_test_chain(&state);
        ensure_trigger_runtime(&state).await;

        publish_watcher_block(&state, 200, "0xparent", "0xgrandparent");
        publish_watcher_transfer(&state, 201, "0xorphan", "0xtx-orphan", 0);
        publish_watcher_transfer(&state, 201, "0xORPHAN", "0xTX-ORPHAN", 0);
        publish_watcher_block(&state, 201, "0xorphan", "0xparent");
        runtime.wait_for_calls(1).await;
        assert_eq!(runtime.calls.lock().await.len(), 1);

        crate::publish_chain_watcher_payload(
            &state,
            "chain:reorg",
            serde_json::to_value(roko_chain::block_watcher::ChainReorgInfo {
                orphaned_block_hashes: vec!["0xORPHAN".to_string()],
            })
            .unwrap(),
        );
        publish_watcher_block(&state, 202, "0xnew-head", "0xreplacement-201");
        tokio::time::sleep(Duration::from_millis(50)).await;
        let lifecycle =
            std::fs::read_to_string(state.layout.triggers_dir().join("lifecycle.jsonl"))
                .expect("lifecycle evidence");
        assert!(lifecycle.contains("reorg_invalidated"));
        assert!(lifecycle.contains("0xorphan"));
        assert_eq!(runtime.calls.lock().await.len(), 1);
        shutdown(&state).await;
    }

    #[tokio::test]
    async fn chain_log_decodes_abi_waits_for_finality_and_handles_reorgs() {
        let directory = tempfile::tempdir().expect("tempdir");
        let runtime = Arc::new(RecordingRuntime::default());
        let state = test_state(
            directory.path(),
            Arc::clone(&runtime),
            &[erc20_transfer_binding("transfer")],
        );
        ensure_trigger_runtime(&state).await;

        state
            .event_bus
            .publish(transfer_log("0xorphan", "0xtx1", 0));
        tokio::time::sleep(Duration::from_millis(30)).await;
        assert!(runtime.calls.lock().await.is_empty());
        state.event_bus.publish(ServerEvent::ChainReorg {
            chain_id: 8453,
            orphaned_block_hashes: vec!["0xORPHAN".to_string()],
        });
        state.event_bus.publish(ServerEvent::ChainFinalityUpdated {
            chain_id: 8453,
            block_hash: "0xorphan".to_string(),
            finality: FinalityRequirement::Final,
        });
        tokio::time::sleep(Duration::from_millis(30)).await;
        assert!(
            runtime.calls.lock().await.is_empty(),
            "a log orphaned before finality must never fire"
        );

        state
            .event_bus
            .publish(transfer_log("0xcanonical", "0xtx2", 1));
        state.event_bus.publish(ServerEvent::ChainFinalityUpdated {
            chain_id: 8453,
            block_hash: "0xCANONICAL".to_string(),
            finality: FinalityRequirement::QuasiFinalized,
        });
        runtime.wait_for_calls(1).await;
        let calls = runtime.calls.lock().await;
        assert_eq!(
            calls[0].payload["decoded"]["from"],
            json!("0x0000000000000000000000000000000000000011")
        );
        assert_eq!(
            calls[0].payload["decoded"]["to"],
            json!("0x0000000000000000000000000000000000000022")
        );
        assert_eq!(calls[0].payload["decoded"]["value"], json!("100"));
        assert_eq!(
            calls[0].payload["finality"],
            json!(FinalityRequirement::QuasiFinalized)
        );
        drop(calls);

        state.event_bus.publish(ServerEvent::ChainFinalityUpdated {
            chain_id: 8453,
            block_hash: "0xcanonical".to_string(),
            finality: FinalityRequirement::Final,
        });
        tokio::time::sleep(Duration::from_millis(30)).await;
        assert_eq!(
            runtime.calls.lock().await.len(),
            1,
            "finality upgrades are idempotent"
        );

        state.event_bus.publish(ServerEvent::ChainReorg {
            chain_id: 8453,
            orphaned_block_hashes: vec!["0xcanonical".to_string()],
        });
        tokio::time::sleep(Duration::from_millis(30)).await;
        let lifecycle =
            std::fs::read_to_string(state.layout.triggers_dir().join("lifecycle.jsonl"))
                .expect("lifecycle evidence");
        assert!(lifecycle.contains("reorg_invalidated"));
        shutdown(&state).await;
    }

    #[tokio::test]
    async fn scoped_trigger_enforces_partition_visibility_and_capability_intersection() {
        let directory = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir_all(directory.path().join("graphs")).unwrap();
        std::fs::write(
            directory.path().join("roko.toml"),
            r#"
[space]
id = "alpha"
visible_graphs = ["graphs/scoped.toml"]

[space.capabilities]
read = true
execute = true
network = false
"#,
        )
        .unwrap();
        std::fs::write(
            directory.path().join("graphs/scoped.toml"),
            r#"
[graph]
name = "scoped"

[graph.capabilities]
allow = ["read", "execute", "network"]

[[nodes]]
id = "root"
cell_type = "noop"
"#,
        )
        .unwrap();

        let runtime = Arc::new(RecordingRuntime::default());
        let mut binding = TriggerBinding::new("scoped", TriggerKind::Manual, "graphs/scoped.toml");
        binding.space = Some("alpha".to_string());
        let state = test_state(directory.path(), Arc::clone(&runtime), &[binding.clone()]);

        let mut event = manual_event("scoped", 1);
        let scope = resolve_trigger_execution_scope(&state, &binding, &mut event).unwrap();
        assert_eq!(event.space_id.as_deref(), Some("alpha"));
        let capabilities = scope.capabilities.expect("effective capabilities");
        assert!(capabilities.contains(Capability::Read));
        assert!(capabilities.contains(Capability::Execute));
        assert!(!capabilities.contains(Capability::Network));

        let handle = ensure_trigger_runtime(&state).await;
        assert_eq!(
            handle.submit(manual_event("scoped", 2)).await.unwrap(),
            TriggerSubmitStatus::Started
        );
        runtime.wait_for_calls(1).await;
        assert_eq!(
            runtime.calls.lock().await[0].space_id.as_deref(),
            Some("alpha")
        );

        let bus_without_partition = TriggerEvent::new(
            "scoped".to_string(),
            json!({}),
            TriggerSource::Bus {
                topic: "work.ready".to_string(),
                pulse_seq: 1,
            },
            "bus-no-space".to_string(),
        );
        assert!(
            resolve_trigger_execution_scope(&state, &binding, &mut bus_without_partition.clone())
                .is_err()
        );

        let mut hidden = binding;
        hidden.graph = "graphs/hidden.toml".to_string();
        std::fs::write(
            directory.path().join("graphs/hidden.toml"),
            "[graph]\nname = \"hidden\"\n[graph.capabilities]\nallow = [\"execute\"]\n",
        )
        .unwrap();
        assert!(
            resolve_trigger_execution_scope(&state, &hidden, &mut manual_event("scoped", 3))
                .is_err()
        );
        shutdown(&state).await;
    }

    #[tokio::test]
    async fn debounce_keeps_only_last_event_and_rate_drop_suppresses_excess() {
        let directory = tempfile::tempdir().expect("tempdir");
        let runtime = Arc::new(RecordingRuntime::default());
        let mut binding = manual_binding("filtered");
        binding.filter = Some(roko_core::trigger::TriggerFilter {
            matches: None,
            debounce_ms: Some(50),
            rate_limit: Some(roko_core::trigger::RateLimit {
                max_fires: 1,
                window_ms: 10_000,
                on_limit: RateLimitAction::Drop,
            }),
        });
        let state = test_state(directory.path(), Arc::clone(&runtime), &[binding]);
        let handle = ensure_trigger_runtime(&state).await;
        for value in 1..=3 {
            assert_eq!(
                handle
                    .submit(manual_event("filtered", value))
                    .await
                    .expect("debounce submit"),
                TriggerSubmitStatus::Queued
            );
        }
        runtime.wait_for_calls(1).await;
        assert_eq!(runtime.calls.lock().await[0].payload["value"], 3);

        // After the debounce window this event reaches rate limiting and drops.
        assert_eq!(
            handle
                .submit(manual_event("filtered", 4))
                .await
                .expect("second debounce"),
            TriggerSubmitStatus::Queued
        );
        tokio::time::sleep(Duration::from_millis(100)).await;
        assert_eq!(runtime.calls.lock().await.len(), 1);
        shutdown(&state).await;
    }

    #[tokio::test]
    async fn queue_skip_and_parallel_policies_are_bounded() {
        let directory = tempfile::tempdir().expect("tempdir");
        let runtime = Arc::new(RecordingRuntime::with_delay(150));
        let mut queued = manual_binding("queued");
        queued.concurrency = ConcurrencyPolicy::Queue { max_depth: Some(1) };
        let mut skipped = manual_binding("skipped");
        skipped.concurrency = ConcurrencyPolicy::Skip;
        let mut parallel = manual_binding("parallel");
        parallel.concurrency = ConcurrencyPolicy::Parallel {
            max_concurrent: Some(2),
        };
        let state = test_state(
            directory.path(),
            Arc::clone(&runtime),
            &[queued, skipped, parallel],
        );
        let handle = ensure_trigger_runtime(&state).await;

        assert_eq!(
            handle.submit(manual_event("queued", 1)).await.unwrap(),
            TriggerSubmitStatus::Started
        );
        assert_eq!(
            handle.submit(manual_event("queued", 2)).await.unwrap(),
            TriggerSubmitStatus::Queued
        );
        assert_eq!(
            handle.submit(manual_event("queued", 3)).await.unwrap(),
            TriggerSubmitStatus::Suppressed
        );

        assert_eq!(
            handle.submit(manual_event("skipped", 1)).await.unwrap(),
            TriggerSubmitStatus::Started
        );
        assert_eq!(
            handle.submit(manual_event("skipped", 2)).await.unwrap(),
            TriggerSubmitStatus::Suppressed
        );

        assert_eq!(
            handle.submit(manual_event("parallel", 1)).await.unwrap(),
            TriggerSubmitStatus::Started
        );
        assert_eq!(
            handle.submit(manual_event("parallel", 2)).await.unwrap(),
            TriggerSubmitStatus::Started
        );
        assert_eq!(
            handle.submit(manual_event("parallel", 3)).await.unwrap(),
            TriggerSubmitStatus::Suppressed
        );
        assert_eq!(runtime.max_active.load(Ordering::SeqCst), 4);
        runtime.wait_for_calls(5).await;
        shutdown(&state).await;
    }

    #[tokio::test]
    async fn cancel_running_and_disable_reload_cancel_active_flow() {
        let directory = tempfile::tempdir().expect("tempdir");
        let runtime = Arc::new(RecordingRuntime::with_delay(5_000));
        let mut binding = manual_binding("replace");
        binding.concurrency = ConcurrencyPolicy::CancelRunning;
        let state = test_state(directory.path(), Arc::clone(&runtime), &[binding.clone()]);
        let handle = ensure_trigger_runtime(&state).await;

        handle.submit(manual_event("replace", 1)).await.unwrap();
        runtime.wait_for_calls(1).await;
        handle.submit(manual_event("replace", 2)).await.unwrap();
        runtime.wait_for_calls(2).await;
        tokio::time::timeout(Duration::from_secs(1), async {
            while runtime.active.load(Ordering::SeqCst) != 1 {
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("old flow cancelled");

        binding.enabled = false;
        handle.reconcile(vec![binding]).await.expect("disable");
        tokio::time::timeout(Duration::from_secs(1), async {
            while runtime.active.load(Ordering::SeqCst) != 0 {
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("active flow cancelled on disable");
        assert!(handle.submit(manual_event("replace", 3)).await.is_err());
        shutdown(&state).await;
    }

    #[tokio::test]
    async fn durable_cli_inbox_is_claimed_once() {
        let directory = tempfile::tempdir().expect("tempdir");
        let runtime = Arc::new(RecordingRuntime::default());
        let state = test_state(
            directory.path(),
            Arc::clone(&runtime),
            &[manual_binding("inbox")],
        );
        let inbox = state.layout.triggers_dir().join("inbox");
        std::fs::create_dir_all(&inbox).unwrap();
        let event = manual_event("inbox", 9);
        let path = inbox.join("event.json");
        std::fs::write(&path, serde_json::to_vec(&event).unwrap()).unwrap();

        ensure_trigger_runtime(&state).await;
        runtime.wait_for_calls(1).await;
        tokio::time::timeout(Duration::from_secs(2), async {
            while path.exists() {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("inbox claimed");
        tokio::time::sleep(Duration::from_millis(600)).await;
        assert_eq!(runtime.calls.lock().await.len(), 1);
        shutdown(&state).await;
    }

    #[tokio::test]
    async fn bus_trigger_observes_real_pulse_bus_traffic() {
        let directory = tempfile::tempdir().expect("tempdir");
        let runtime = Arc::new(RecordingRuntime::default());
        let binding = TriggerBinding::new(
            "pulse-listener",
            TriggerKind::Bus(roko_core::trigger::BusTrigger {
                topic: "work.ready".to_string(),
            }),
            "graphs/pulse.toml",
        );
        let state = test_state(directory.path(), Arc::clone(&runtime), &[binding]);
        ensure_trigger_runtime(&state).await;

        let pulse = Pulse::builder(
            state.pulse_bus.total_published(),
            Topic::new("work.ready"),
            Kind::Task,
        )
        .body(Body::Json(json!({"task": "compile"})))
        .build();
        roko_core::Bus::publish(state.pulse_bus.as_ref(), pulse).expect("publish pulse");

        runtime.wait_for_calls(1).await;
        let calls = runtime.calls.lock().await;
        assert_eq!(calls[0].payload["task"], "compile");
        assert!(matches!(
            calls[0].source,
            TriggerSource::Bus { ref topic, .. } if topic == "work.ready"
        ));
        drop(calls);
        shutdown(&state).await;
    }

    #[test]
    fn wildcard_and_payload_matching_cover_declared_filter_shapes() {
        assert!(wildcard_matches("gate.*.passed", "gate.compile.passed"));
        assert!(!wildcard_matches("gate.*.passed", "gate.compile.failed"));
        let filter = roko_core::trigger::TriggerFilter {
            matches: Some(BTreeMap::from([
                ("nested.name".to_string(), json!("deploy")),
                ("/count".to_string(), json!(2)),
            ])),
            debounce_ms: None,
            rate_limit: None,
        };
        assert!(payload_matches(
            &json!({"nested": {"name": "deploy-production"}, "count": 2}),
            Some(&filter),
        ));
    }

    #[test]
    fn input_mapping_supports_jsonpath_nested_targets_and_transforms() {
        let mut binding = manual_binding("mapped");
        binding.input_mapping = Some(roko_core::trigger::TriggerInputMapping {
            mappings: vec![
                roko_core::trigger::InputFieldMapping {
                    from: "$.pull.head.ref".to_string(),
                    to: "git.branch".to_string(),
                    transform: Some("lowercase".to_string()),
                },
                roko_core::trigger::InputFieldMapping {
                    from: "$.items[0].id".to_string(),
                    to: "issue.number".to_string(),
                    transform: Some("number".to_string()),
                },
            ],
        });
        let event = TriggerEvent::new(
            "mapped".to_string(),
            json!({
                "pull": {"head": {"ref": "FEATURE/ONE"}},
                "items": [{"id": "42"}],
            }),
            TriggerSource::Manual {
                user: "test".to_string(),
            },
            "mapped-trace".to_string(),
        );

        let mapped = map_event_input(&binding, event).expect("map event");
        assert_eq!(mapped.payload["inputs"]["git"]["branch"], "feature/one");
        assert_eq!(mapped.payload["inputs"]["issue"]["number"], 42);
        assert_eq!(
            mapped.payload["event"]["pull"]["head"]["ref"],
            "FEATURE/ONE"
        );
    }
}
