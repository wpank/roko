//! Trigger protocol types — protocol-level definitions for the Roko trigger system.
//!
//! These types are distinct from `roko-plugin`'s manifest-level `TriggerDef` stubs.
//! This module provides full protocol fidelity: the 7-source state machine, handle
//! lifecycle, event struct, and binding configuration.
//!
//! # Architecture
//!
//! A trigger is an armed condition that fires when criteria are met, publishing a
//! `TriggerEvent` pulse. Implementors of [`TriggerProtocol`] handle a specific source
//! kind (Cron, Webhook, FileWatch, etc.).
//!
//! ```text
//! TriggerBinding ──arm()──► TriggerHandle (Armed)
//!                               │
//!                    condition  ▼
//!                           TriggerEvent published as Pulse
//!                               │
//!                    disarm()   ▼
//!                           TriggerHandle (Disarmed)
//! ```

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use std::{fs, io};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::Result;

// ── Trigger event topic constants ─────────────────────────────────────────────
// Bus/signal kind strings published when trigger lifecycle events occur.

/// Topic published when a trigger fires (condition met, Flow being spawned).
pub const TRIGGER_FIRED: &str = "trigger:fired";

/// Topic published when a new trigger binding is created.
pub const TRIGGER_CREATED: &str = "trigger:created";

/// Topic published when a trigger binding is deleted.
pub const TRIGGER_DELETED: &str = "trigger:deleted";

/// Topic published when a trigger firing is rejected by rate limiting.
pub const TRIGGER_RATE_LIMITED: &str = "trigger:rate_limited";

/// Topic published when trigger authentication fails (e.g. bad HMAC, expired token).
pub const TRIGGER_AUTH_FAILED: &str = "trigger:auth_failed";

// ── Trigger event topics and graduation ─────────────────────────────────────

/// Construct a scoped trigger lifecycle topic such as
/// `trigger.deploy.fired`.
#[must_use]
pub fn trigger_topic(name: &str, event: &str) -> String {
    format!("trigger.{name}.{event}")
}

/// Trigger lifecycle events published on the Bus.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TriggerEventKind {
    Armed,
    Fired,
    Filtered,
    Skipped,
    Queued,
    RateLimited,
    Error,
    Disarmed,
    FlowStarted,
    FlowCompleted,
}

impl TriggerEventKind {
    /// Stable topic suffix for this lifecycle event.
    #[must_use]
    pub const fn as_topic_suffix(self) -> &'static str {
        match self {
            Self::Armed => "armed",
            Self::Fired => "fired",
            Self::Filtered => "filtered",
            Self::Skipped => "skipped",
            Self::Queued => "queued",
            Self::RateLimited => "rate_limited",
            Self::Error => "error",
            Self::Disarmed => "disarmed",
            Self::FlowStarted => "flow.started",
            Self::FlowCompleted => "flow.completed",
        }
    }
}

/// Lifecycle events promoted from ephemeral Pulses to durable Signals.
pub const GRADUATION_EVENTS: [TriggerEventKind; 8] = [
    TriggerEventKind::Armed,
    TriggerEventKind::Fired,
    TriggerEventKind::Skipped,
    TriggerEventKind::RateLimited,
    TriggerEventKind::Error,
    TriggerEventKind::Disarmed,
    TriggerEventKind::FlowStarted,
    TriggerEventKind::FlowCompleted,
];

/// Durable/auditable description of a trigger lifecycle transition.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TriggerLifecycleEvent {
    /// Binding name.
    pub trigger_name: String,
    /// Lifecycle transition.
    pub kind: TriggerEventKind,
    /// Scoped bus topic.
    pub topic: String,
    /// Event time in Unix milliseconds.
    pub occurred_at_ms: u64,
    /// Graph configured for the binding.
    pub graph: String,
    /// Trace id of the firing/flow, when applicable.
    pub trace_id: Option<String>,
    /// Structured lifecycle metadata.
    pub detail: Value,
}

impl TriggerLifecycleEvent {
    /// Construct a lifecycle event using the current wall clock.
    #[must_use]
    pub fn new(
        binding: &TriggerBinding,
        kind: TriggerEventKind,
        trace_id: Option<String>,
        detail: Value,
    ) -> Self {
        Self {
            trigger_name: binding.name.clone(),
            kind,
            topic: trigger_topic(&binding.name, kind.as_topic_suffix()),
            occurred_at_ms: unix_time_ms(),
            graph: binding.graph.clone(),
            trace_id,
            detail,
        }
    }
}

/// One durable trigger firing and the lifecycle transitions correlated to it.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TriggerHistoryRecord {
    /// The original durable firing event.
    pub event: TriggerEvent,
    /// Lifecycle transitions sharing the firing's trace id, in time order.
    /// Flow transitions include their runtime `run_id` in `detail`.
    pub lifecycle: Vec<TriggerLifecycleEvent>,
}

/// Durable firing history returned by the CLI and HTTP API.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TriggerHistory {
    /// Binding name whose evidence was queried.
    pub trigger_name: String,
    /// Number of durable firing events before applying the query limit.
    pub total: usize,
    /// Most-recent firing records first.
    pub records: Vec<TriggerHistoryRecord>,
}

/// Read durable firing and lifecycle evidence from a trigger directory.
///
/// `directory` is the `.roko/triggers` directory. Missing evidence files are
/// treated as an empty history; malformed durable evidence is reported rather
/// than silently omitted.
pub fn load_trigger_history(
    directory: &Path,
    trigger_name: &str,
    limit: usize,
) -> io::Result<TriggerHistory> {
    if !valid_trigger_name(trigger_name) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "invalid trigger name",
        ));
    }

    let mut events = Vec::new();
    match fs::read_dir(directory.join("events")) {
        Ok(entries) => {
            for entry in entries {
                let path = entry?.path();
                if path.extension().and_then(|extension| extension.to_str()) != Some("json") {
                    continue;
                }
                let bytes = fs::read(&path)?;
                let event: TriggerEvent = serde_json::from_slice(&bytes).map_err(|error| {
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("parse trigger event {}: {error}", path.display()),
                    )
                })?;
                if event.trigger_id == trigger_name {
                    events.push(event);
                }
            }
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(error),
    }
    events.sort_by_key(|event| std::cmp::Reverse(event.fired_at_ms));
    let total = events.len();
    events.truncate(limit);

    let selected_traces: std::collections::HashSet<&str> =
        events.iter().map(|event| event.trace_id.as_str()).collect();
    let mut lifecycle_by_trace: BTreeMap<String, Vec<TriggerLifecycleEvent>> = BTreeMap::new();
    match fs::read_to_string(directory.join("lifecycle.jsonl")) {
        Ok(contents) => {
            for (index, line) in contents.lines().enumerate() {
                if line.trim().is_empty() {
                    continue;
                }
                let lifecycle: TriggerLifecycleEvent =
                    serde_json::from_str(line).map_err(|error| {
                        io::Error::new(
                            io::ErrorKind::InvalidData,
                            format!("parse trigger lifecycle line {}: {error}", index + 1),
                        )
                    })?;
                let Some(trace_id) = lifecycle.trace_id.as_deref() else {
                    continue;
                };
                if lifecycle.trigger_name == trigger_name && selected_traces.contains(trace_id) {
                    lifecycle_by_trace
                        .entry(trace_id.to_string())
                        .or_default()
                        .push(lifecycle);
                }
            }
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(error),
    }
    for lifecycle in lifecycle_by_trace.values_mut() {
        lifecycle.sort_by_key(|event| event.occurred_at_ms);
    }

    let records = events
        .into_iter()
        .map(|event| {
            let lifecycle = lifecycle_by_trace
                .remove(&event.trace_id)
                .unwrap_or_default();
            TriggerHistoryRecord { event, lifecycle }
        })
        .collect();
    Ok(TriggerHistory {
        trigger_name: trigger_name.to_string(),
        total,
        records,
    })
}

fn unix_time_ms() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}

// ── Binding trust graduation ─────────────────────────────────────────────────

/// Controls when a trigger binding is auto-promoted to a higher trust tier.
///
/// Distinct from [`GraduationPolicy`](crate::config::graduation::GraduationPolicy),
/// which governs Pulse-to-Signal promotion. This policy also provides the
/// canonical lifecycle-event graduation predicate via [`Self::should_graduate`].
/// Its enum variants describe promotion of trigger *bindings* themselves
/// (e.g. from draft/sandbox to production).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "policy", rename_all = "snake_case")]
pub enum TriggerGraduationPolicy {
    /// Only explicit manual promotion (no auto-graduation).
    ManualOnly,
    /// Auto-promote after `count` consecutive successful firings.
    AfterSuccesses {
        /// Number of consecutive successful firings required.
        count: u32,
    },
    /// Auto-promote after the binding has existed for at least `min_age_hours`.
    TimeBased {
        /// Minimum age in hours before auto-promotion is allowed.
        min_age_hours: u64,
    },
}

impl TriggerGraduationPolicy {
    /// Whether a trigger lifecycle event should be retained as a durable Signal.
    #[must_use]
    pub fn should_graduate(kind: &TriggerEventKind) -> bool {
        GRADUATION_EVENTS.contains(kind)
    }
}

impl Default for TriggerGraduationPolicy {
    fn default() -> Self {
        Self::ManualOnly
    }
}

// ── Placeholder type aliases ──────────────────────────────────────────────────
// These will be replaced with concrete types as CellRef/GraphRef/SpaceId/etc. land.

/// Unique identifier for a trigger binding.
pub type TriggerId = String;

/// Reference to a Graph (placeholder until Graph types exist).
pub type GraphRef = String;

/// Space identifier for capability scoping.
pub type SpaceId = String;

/// Trace identifier for correlating trigger events with resulting flows.
pub type TraceId = String;

/// Author reference for manual triggers.
pub type Author = String;

/// JSONPath expression (placeholder string).
pub type Expr = String;

/// Signal reference (placeholder).
pub type SignalRef = String;

// ── TriggerProtocol ───────────────────────────────────────────────────────────

/// Protocol for trigger implementations.
///
/// Each trigger kind (Cron, Webhook, FileWatch, Bus, ChainEvent, Manual,
/// SignalPattern) implements this trait. When armed, the implementation sets up
/// the event subscription (timer, Axum route, notify watcher, etc.) and returns a
/// [`TriggerHandle`]. When the condition fires, the implementation publishes a
/// `TriggerEvent` pulse on `trigger:{name}:fired`.
///
/// # Anti-patterns
///
/// Do NOT add runtime/bus dependencies to this trait's method signatures in
/// roko-core — the Bus type parameter would create a dependency cycle. Implementors
/// live in crates that depend on roko-core, not the other way around.
#[async_trait]
pub trait TriggerProtocol: Send + Sync {
    /// Arm the trigger. Sets up event subscription and returns a handle.
    ///
    /// The implementation must:
    /// 1. Validate the binding's kind-specific config.
    /// 2. Set up the subscription/watcher/route/timer.
    /// 3. Return a `TriggerHandle` in the `Armed` state.
    async fn arm(&self, binding: TriggerBinding) -> Result<TriggerHandle>;

    /// Disarm the trigger. Tears down subscriptions and cleans up resources.
    ///
    /// After disarming, the handle's state transitions to `Disarmed`.
    async fn disarm(&self, handle: TriggerHandle) -> Result<()>;
}

// ── TriggerHandle ─────────────────────────────────────────────────────────────

/// A live handle to an armed trigger.
///
/// Returned by [`TriggerProtocol::arm`]. The handle carries the binding
/// configuration and current state. Pass this to [`TriggerProtocol::disarm`]
/// to clean up.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TriggerHandle {
    /// Unique identifier for this armed instance.
    pub id: TriggerId,
    /// The binding that was armed.
    pub binding: TriggerBinding,
    /// When the trigger was armed (millis since UNIX epoch).
    pub armed_at_ms: u64,
    /// Current lifecycle state.
    pub state: TriggerState,
}

impl TriggerHandle {
    /// Create a new handle in the `Armed` state.
    #[must_use]
    pub fn new_armed(id: TriggerId, binding: TriggerBinding) -> Self {
        let armed_at_ms = unix_time_ms();
        Self {
            id,
            binding,
            armed_at_ms,
            state: TriggerState::Armed,
        }
    }

    /// Returns true if the trigger is currently capable of firing.
    #[must_use]
    pub fn is_active(&self) -> bool {
        matches!(self.state, TriggerState::Armed | TriggerState::Firing)
    }
}

// ── TriggerState ─────────────────────────────────────────────────────────────

/// Lifecycle state of an armed trigger.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum TriggerState {
    /// Watching for conditions; ready to fire.
    Armed,
    /// Currently executing a fire (condition met, Flow being spawned).
    Firing,
    /// Temporarily suppressed until the given epoch (millis since UNIX epoch).
    Cooldown {
        /// Epoch ms at which the cooldown expires and the trigger returns to `Armed`.
        until_ms: u64,
    },
    /// Explicitly disarmed; no longer watching.
    Disarmed,
    /// Encountered an unrecoverable error; requires manual intervention.
    Failed {
        /// Human-readable error description.
        error: String,
    },
}

// ── TriggerEvent ─────────────────────────────────────────────────────────────

/// An event fired by a trigger.
///
/// Published as a [`Pulse`](crate::Pulse) on `trigger:{name}:fired`. The Trigger
/// Engine subscribes to `trigger:*:fired` and spawns the bound Graph as a Flow.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TriggerEvent {
    /// The trigger that fired.
    pub trigger_id: TriggerId,
    /// When the event fired (millis since UNIX epoch).
    pub fired_at_ms: u64,
    /// The event payload (kind-specific structure).
    pub payload: Value,
    /// Which source produced this event.
    pub source: TriggerSource,
    /// Space this event originated in (for Space capability scoping).
    pub space_id: Option<SpaceId>,
    /// Trace ID for correlating the trigger event with the resulting Flow.
    pub trace_id: TraceId,
}

impl TriggerEvent {
    /// Create a new trigger event with the current timestamp.
    #[must_use]
    pub fn new(
        trigger_id: TriggerId,
        payload: Value,
        source: TriggerSource,
        trace_id: TraceId,
    ) -> Self {
        let fired_at_ms = unix_time_ms();
        Self {
            trigger_id,
            fired_at_ms,
            payload,
            source,
            space_id: None,
            trace_id,
        }
    }

    /// Set the space scope for this event.
    #[must_use]
    pub fn with_space(mut self, space_id: SpaceId) -> Self {
        self.space_id = Some(space_id);
        self
    }
}

// ── TriggerSource ─────────────────────────────────────────────────────────────

/// The 7 possible origins of a trigger event.
///
/// Each variant carries the kind-specific metadata that was captured when the
/// condition fired.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TriggerSource {
    /// A cron schedule fired.
    Cron {
        /// The cron expression that matched.
        expression: String,
    },
    /// An inbound HTTP webhook matched.
    Webhook {
        /// HTTP method (GET, POST, etc.).
        method: String,
        /// Path that received the request.
        path: String,
        /// Headers from the inbound request (selected subset, not all headers).
        headers: BTreeMap<String, String>,
    },
    /// A filesystem event was detected.
    FileWatch {
        /// The path that changed.
        path: PathBuf,
        /// The kind of filesystem event.
        event: FileWatchEvent,
    },
    /// A matching pulse was received on the Bus.
    Bus {
        /// Bus topic the pulse arrived on.
        topic: String,
        /// Sequence number of the matching pulse.
        pulse_seq: u64,
    },
    /// An on-chain event was indexed.
    ChainEvent {
        /// Chain ID (EIP-155).
        chain_id: u64,
        /// Block number containing the event.
        block_number: u64,
        /// Transaction hash that emitted the event.
        tx_hash: String,
    },
    /// Manually fired by a user or API call.
    Manual {
        /// The author who triggered it.
        user: Author,
    },
    /// A pattern of signals matched.
    SignalPattern {
        /// The signals that matched the pattern.
        matched_signals: Vec<SignalRef>,
    },
}

/// Filesystem event kinds for [`TriggerSource::FileWatch`].
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FileWatchEvent {
    Created,
    Modified,
    Deleted,
    Renamed,
    /// Any of the above.
    Any,
}

// ── TriggerBinding ────────────────────────────────────────────────────────────

/// Persistent, TOML-defined configuration connecting an event source to a Graph.
///
/// Bindings survive process restarts — they are stored in `.roko/triggers/`
/// and re-armed on startup. See `TriggerProtocol::arm` for the arming contract.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TriggerBinding {
    /// Unique name for this binding (used as the file stem in `.roko/triggers/`).
    pub name: String,
    /// The trigger kind and its source-specific configuration.
    pub kind: TriggerKind,
    /// The Graph to fire when the trigger activates.
    pub graph: GraphRef,
    /// How to map the trigger event payload to Graph input signals.
    pub input_mapping: Option<TriggerInputMapping>,
    /// What to do if the trigger fires while a previous Flow is still running.
    pub concurrency: ConcurrencyPolicy,
    /// Additional conditions that must be met beyond the kind-specific matching.
    pub filter: Option<TriggerFilter>,
    /// Whether this binding is currently enabled (can be toggled without deletion).
    pub enabled: bool,
    /// Space this trigger runs within (restricts capability grants for fired Flows).
    pub space: Option<SpaceId>,
    /// Authentication configuration for the trigger source.
    pub auth: Option<TriggerAuth>,
    /// When (if ever) this binding should be auto-promoted to a higher trust tier.
    #[serde(default)]
    pub graduation_policy: TriggerGraduationPolicy,
}

impl TriggerBinding {
    /// Create a minimal enabled binding with default concurrency.
    #[must_use]
    pub fn new(name: impl Into<String>, kind: TriggerKind, graph: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            kind,
            graph: graph.into(),
            input_mapping: None,
            concurrency: ConcurrencyPolicy::Queue { max_depth: None },
            filter: None,
            enabled: true,
            space: None,
            auth: None,
            graduation_policy: TriggerGraduationPolicy::default(),
        }
    }

    /// Validate fields that are used as filesystem/runtime identifiers.
    ///
    /// # Errors
    ///
    /// Returns invalid-data when the name could escape `.roko/triggers/` or
    /// when the graph reference is empty or contains parent traversal.
    pub fn validate(&self) -> io::Result<()> {
        if !valid_trigger_name(&self.name) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "trigger name must contain only ASCII letters, digits, '.', '_' or '-' and must not be '.' or '..'",
            ));
        }
        let graph = Path::new(&self.graph);
        if self.graph.trim().is_empty()
            || graph.is_absolute()
            || graph.components().any(|component| {
                matches!(
                    component,
                    std::path::Component::ParentDir
                        | std::path::Component::RootDir
                        | std::path::Component::Prefix(_)
                )
            })
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "trigger graph must be a non-empty worktree-relative path without parent traversal",
            ));
        }
        match &self.kind {
            TriggerKind::Cron(config) if config.expression.trim().is_empty() => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "cron expression must not be empty",
                ));
            }
            TriggerKind::Webhook(config)
                if !config.path.starts_with('/')
                    || config.path.contains("..")
                    || config.path.contains(['?', '#']) =>
            {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "webhook path must be an absolute URL path without traversal, query, or fragment",
                ));
            }
            TriggerKind::FileWatch(config)
                if config.path.is_absolute()
                    || config.path.components().any(|component| {
                        matches!(
                            component,
                            std::path::Component::ParentDir
                                | std::path::Component::RootDir
                                | std::path::Component::Prefix(_)
                        )
                    }) =>
            {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "file-watch path must be worktree-relative without parent traversal",
                ));
            }
            TriggerKind::Bus(config) if config.topic.trim().is_empty() => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "bus topic must not be empty",
                ));
            }
            TriggerKind::SignalPattern(config)
                if config.required_kinds.is_empty()
                    || config
                        .required_kinds
                        .iter()
                        .any(|kind| kind.trim().is_empty())
                    || config.window_secs == 0 =>
            {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "signal pattern requires non-empty kinds and a non-zero window",
                ));
            }
            _ => {}
        }
        match self.concurrency {
            ConcurrencyPolicy::Queue { max_depth: Some(0) } => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "queue concurrency max_depth must be greater than zero",
                ));
            }
            ConcurrencyPolicy::Parallel {
                max_concurrent: Some(0),
            } => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "parallel concurrency max_concurrent must be greater than zero",
                ));
            }
            _ => {}
        }
        if let Some(rate_limit) = self
            .filter
            .as_ref()
            .and_then(|filter| filter.rate_limit.as_ref())
            && (rate_limit.max_fires == 0 || rate_limit.window_ms == 0)
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "rate limit max_fires and window_ms must be greater than zero",
            ));
        }
        Ok(())
    }

    // ── TOML persistence ─────────────────────────────────────────────────

    /// Serialize this binding to TOML and write it to `path`.
    ///
    /// Creates parent directories if they do not exist.
    ///
    /// # Errors
    ///
    /// Returns an I/O error if serialization or writing fails.
    pub fn save_to_file(&self, path: &Path) -> io::Result<()> {
        self.validate()?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let toml_str = toml::to_string_pretty(self).map_err(|e| {
            io::Error::new(io::ErrorKind::InvalidData, format!("TOML serialize: {e}"))
        })?;
        let temporary = path.with_extension("toml.tmp");
        fs::write(&temporary, toml_str)?;
        fs::rename(temporary, path)
    }

    /// Deserialize a `TriggerBinding` from a TOML file at `path`.
    ///
    /// # Errors
    ///
    /// Returns an I/O error if the file cannot be read or contains invalid TOML.
    pub fn load_from_file(path: &Path) -> io::Result<Self> {
        let text = fs::read_to_string(path)?;
        let binding: Self = toml::from_str(&text).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("TOML parse {}: {e}", path.display()),
            )
        })?;
        binding.validate()?;
        if let Some(file_name) = path.file_stem().and_then(|stem| stem.to_str())
            && file_name != binding.name
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "trigger binding name '{}' does not match file name '{file_name}'",
                    binding.name
                ),
            ));
        }
        Ok(binding)
    }

    /// Save every binding in `bindings` to `dir`, one file per binding.
    ///
    /// Each file is named `{binding.name}.toml`. The directory is created
    /// if it does not exist.
    ///
    /// # Errors
    ///
    /// Returns an I/O error if the directory cannot be created or any
    /// binding fails to serialize/write.
    pub fn save_all(dir: &Path, bindings: &[TriggerBinding]) -> io::Result<()> {
        fs::create_dir_all(dir)?;
        for binding in bindings {
            let file = dir.join(format!("{}.toml", binding.name));
            binding.save_to_file(&file)?;
        }
        Ok(())
    }

    /// Load all `*.toml` files in `dir` as `TriggerBinding`s.
    ///
    /// Non-`.toml` files are silently skipped. If `dir` does not exist,
    /// returns an empty `Vec` rather than an error.
    ///
    /// # Errors
    ///
    /// Returns an I/O error if the directory cannot be read, or any
    /// `.toml` file contains invalid TOML.
    pub fn load_all(dir: &Path) -> io::Result<Vec<TriggerBinding>> {
        let entries = match fs::read_dir(dir) {
            Ok(entries) => entries,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(e) => return Err(e),
        };

        let mut bindings = Vec::new();
        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("toml") {
                bindings.push(Self::load_from_file(&path)?);
            }
        }
        // Sort by name for deterministic ordering.
        bindings.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(bindings)
    }
}

/// Whether `name` is safe as a trigger identifier and TOML file stem.
#[must_use]
pub fn valid_trigger_name(name: &str) -> bool {
    !name.is_empty()
        && name != "."
        && name != ".."
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
}

// ── TriggerKind ───────────────────────────────────────────────────────────────

/// Per-source configuration for a trigger binding.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TriggerKind {
    Cron(CronTrigger),
    Webhook(WebhookTrigger),
    FileWatch(FileWatchTrigger),
    Bus(BusTrigger),
    ChainEvent(ChainEventTrigger),
    Manual,
    SignalPattern(SignalPatternTrigger),
}

/// Cron trigger: fires on a schedule.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CronTrigger {
    /// Standard cron expression (e.g. `"0 * * * *"` = every hour).
    pub expression: String,
    /// Optional timezone (defaults to UTC).
    pub timezone: Option<String>,
}

/// Webhook trigger: fires on inbound HTTP requests.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WebhookTrigger {
    /// HTTP method to match (e.g. `"POST"`). `None` = match any.
    pub method: Option<String>,
    /// Path suffix to mount (e.g. `"/hook/my-trigger"`).
    pub path: String,
}

/// FileWatch trigger: fires on filesystem events.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FileWatchTrigger {
    /// Path to watch (file or directory).
    pub path: PathBuf,
    /// Which filesystem events to react to.
    pub events: Vec<FileWatchEvent>,
    /// Glob pattern for filtering within a watched directory.
    pub glob: Option<String>,
}

/// Bus trigger: fires when a matching pulse arrives.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BusTrigger {
    /// Topic filter (supports `*` wildcards, e.g. `"gate.verdict.*"`).
    pub topic: String,
}

/// ChainEvent trigger: fires on indexed on-chain events.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChainEventTrigger {
    /// EIP-155 chain ID.
    pub chain_id: u64,
    /// Contract address to watch (checksummed hex).
    pub contract: String,
    /// ABI event signature to filter (e.g. `"Transfer(address,address,uint256)"`).
    pub event_signature: String,
    /// Optional JSON ABI used to decode indexed and non-indexed log fields.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub abi: Option<Value>,
    /// Minimum finality required before the trigger may fire.
    #[serde(default)]
    pub finality: FinalityRequirement,
}

/// Confidence required for an on-chain log to fire a trigger.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FinalityRequirement {
    /// Fire immediately. The resulting event may later be invalidated by a reorg.
    Reversible,
    /// Wait for a high-confidence confirmation threshold.
    #[default]
    QuasiFinalized,
    /// Wait for the chain's final confirmation threshold.
    Final,
}

/// SignalPattern trigger: fires when a pattern of signals is detected.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SignalPatternTrigger {
    /// Human-readable description of the pattern.
    pub description: String,
    /// Signal kinds that must all appear within the time window.
    pub required_kinds: Vec<String>,
    /// Time window in seconds within which all signals must appear.
    pub window_secs: u64,
}

// ── TriggerInputMapping ───────────────────────────────────────────────────────

/// Defines how to map trigger event payload fields to Graph input signals.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TriggerInputMapping {
    /// Individual field mappings.
    pub mappings: Vec<InputFieldMapping>,
}

/// A single field mapping from trigger event payload to Graph input.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InputFieldMapping {
    /// JSONPath expression selecting a value from the trigger event payload.
    pub from: String,
    /// Target field name in the Graph's input signals.
    pub to: String,
    /// Optional transformation to apply to the selected value.
    pub transform: Option<Expr>,
}

// ── ConcurrencyPolicy ─────────────────────────────────────────────────────────

/// What to do when a trigger fires while a previous Flow is still running.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ConcurrencyPolicy {
    /// Buffer firings in a queue. If the queue is full, drop the new firing.
    Queue {
        /// Maximum queue depth. `None` = unbounded (not recommended for production).
        max_depth: Option<usize>,
    },
    /// Silently drop new firings while a Flow is running.
    Skip,
    /// Cancel the currently running Flow and start a new one.
    CancelRunning,
    /// Allow multiple concurrent Flows from this trigger.
    Parallel {
        /// Maximum concurrency. `None` = unbounded.
        max_concurrent: Option<usize>,
    },
}

// ── TriggerFilter ─────────────────────────────────────────────────────────────

/// Additional conditions that must be met before a trigger actually fires.
///
/// Applied after kind-specific matching, in the order: event_kind →
/// where_clause → matches → debounce → rate_limit → custom_filter.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TriggerFilter {
    /// Only fire for specific payload string patterns (applied as `contains`).
    pub matches: Option<BTreeMap<String, Value>>,
    /// Minimum time between firings (debounce). Duration in milliseconds.
    pub debounce_ms: Option<u64>,
    /// Rate limiting configuration.
    pub rate_limit: Option<RateLimit>,
}

/// Rate limiting for trigger firings.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RateLimit {
    /// Maximum number of fires within the window.
    pub max_fires: u32,
    /// Window duration in milliseconds.
    pub window_ms: u64,
    /// What to do when the rate limit is exceeded.
    pub on_limit: RateLimitAction,
}

/// Action to take when a rate limit is exceeded.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RateLimitAction {
    /// Silently discard the firing.
    Drop,
    /// Buffer the firing (subject to [`ConcurrencyPolicy`]).
    Queue,
    /// Log a warning but still fire.
    Warn,
}

// ── TriggerAuth ───────────────────────────────────────────────────────────────

/// Authentication configuration for a trigger source.
///
/// Secrets are never stored inline — only references to where the secret
/// lives (env var name, store key, or file path).
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TriggerAuth {
    /// No authentication required.
    None,
    /// HMAC-SHA256 signature verification (e.g. GitHub webhooks).
    HmacSha256 {
        /// Reference to the shared secret.
        secret: SecretRef,
        /// Name of the header carrying the signature (e.g. `"X-Hub-Signature-256"`).
        header: String,
    },
    /// Bearer token verification.
    BearerToken {
        /// Reference to the expected bearer token.
        secret: SecretRef,
    },
    /// Mutual TLS client certificate authentication.
    MutualTls {
        /// Path to the PEM certificate chain presented by the Roko HTTPS server.
        cert: PathBuf,
        /// Reference to the PEM private key for the Roko HTTPS server.
        key: SecretRef,
        /// PEM trust anchor used to authenticate webhook client certificates.
        client_ca: PathBuf,
    },
}

/// A reference to a secret — never the secret value itself.
///
/// Distinct from `roko_core::secrets::SecretSource` (which describes *where*
/// a resolved secret came from) — `SecretRef` is a *pointer* to where the
/// secret should be looked up at runtime.
///
/// All variants use struct form (not tuple/newtype) to allow internal tagging
/// (`#[serde(tag = "kind")]`) to work with serde's JSON serializer.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SecretRef {
    /// Read from an environment variable with this name.
    Env {
        /// Name of the environment variable.
        var: String,
    },
    /// Look up this key in the Roko secret store.
    Store {
        /// Key in the secret store.
        key: String,
    },
    /// Read from a file at this path.
    File {
        /// Path to the file containing the secret.
        path: PathBuf,
    },
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trigger_handle_new_armed() {
        let binding = TriggerBinding::new("test-trigger", TriggerKind::Manual, "plans/test.toml");
        let handle = TriggerHandle::new_armed("handle-1".to_string(), binding);
        assert_eq!(handle.id, "handle-1");
        assert!(matches!(handle.state, TriggerState::Armed));
        assert!(handle.is_active());
        assert!(handle.armed_at_ms > 0);
    }

    #[test]
    fn trigger_state_is_active() {
        let firing_handle = TriggerHandle {
            id: "x".to_string(),
            binding: TriggerBinding::new("x", TriggerKind::Manual, "g"),
            armed_at_ms: 0,
            state: TriggerState::Firing,
        };
        assert!(firing_handle.is_active());

        let disarmed_handle = TriggerHandle {
            id: "x".to_string(),
            binding: TriggerBinding::new("x", TriggerKind::Manual, "g"),
            armed_at_ms: 0,
            state: TriggerState::Disarmed,
        };
        assert!(!disarmed_handle.is_active());

        let armed_handle = TriggerHandle {
            id: "x".to_string(),
            binding: TriggerBinding::new("x", TriggerKind::Manual, "g"),
            armed_at_ms: 0,
            state: TriggerState::Armed,
        };
        assert!(armed_handle.is_active());
    }

    #[test]
    fn trigger_event_new() {
        let event = TriggerEvent::new(
            "my-trigger".to_string(),
            serde_json::json!({"key": "value"}),
            TriggerSource::Manual {
                user: "will".to_string(),
            },
            "trace-123".to_string(),
        );
        assert_eq!(event.trigger_id, "my-trigger");
        assert!(event.space_id.is_none());
        assert_eq!(event.trace_id, "trace-123");
    }

    #[test]
    fn trigger_event_with_space() {
        let event = TriggerEvent::new(
            "t".to_string(),
            serde_json::json!(null),
            TriggerSource::Manual {
                user: "admin".to_string(),
            },
            "trace-abc".to_string(),
        )
        .with_space("alpha".to_string());
        assert_eq!(event.space_id.as_deref(), Some("alpha"));
    }

    #[test]
    fn trigger_binding_defaults() {
        let b = TriggerBinding::new(
            "cron-job",
            TriggerKind::Cron(CronTrigger {
                expression: "0 * * * *".to_string(),
                timezone: None,
            }),
            "plans/hourly.toml",
        );
        assert!(b.enabled);
        assert!(b.filter.is_none());
        assert!(b.auth.is_none());
        assert!(b.space.is_none());
        assert!(matches!(
            b.concurrency,
            ConcurrencyPolicy::Queue { max_depth: None }
        ));
    }

    #[test]
    fn trigger_binding_roundtrip_json() {
        let b = TriggerBinding::new(
            "webhook-hook",
            TriggerKind::Webhook(WebhookTrigger {
                method: Some("POST".to_string()),
                path: "/hook/test".to_string(),
            }),
            "plans/on-webhook.toml",
        );
        let json = serde_json::to_string(&b).expect("serialise");
        let b2: TriggerBinding = serde_json::from_str(&json).expect("deserialise");
        assert_eq!(b.name, b2.name);
        assert_eq!(b.graph, b2.graph);
        assert_eq!(b.enabled, b2.enabled);
    }

    // ── TOML persistence tests ────────────────────────────────────────────

    #[test]
    fn save_load_round_trip() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("my-trigger.toml");
        let binding = TriggerBinding::new(
            "my-trigger",
            TriggerKind::Cron(CronTrigger {
                expression: "0 */6 * * *".to_string(),
                timezone: Some("America/New_York".to_string()),
            }),
            "plans/cron-job.toml",
        );
        binding.save_to_file(&path).expect("save");
        let loaded = TriggerBinding::load_from_file(&path).expect("load");
        assert_eq!(loaded.name, "my-trigger");
        assert_eq!(loaded.graph, "plans/cron-job.toml");
        assert!(loaded.enabled);
    }

    #[test]
    fn save_creates_parent_dirs() {
        let dir = tempfile::tempdir().expect("tempdir");
        let nested = dir
            .path()
            .join("a")
            .join("b")
            .join("c")
            .join("trigger.toml");
        let binding = TriggerBinding::new("nested", TriggerKind::Manual, "g");
        binding.save_to_file(&nested).expect("save nested");
        assert!(nested.exists());
    }

    #[test]
    fn save_all_load_all_round_trip() {
        let dir = tempfile::tempdir().expect("tempdir");
        let triggers_dir = dir.path().join("triggers");

        let bindings = vec![
            TriggerBinding::new("alpha", TriggerKind::Manual, "g1"),
            TriggerBinding::new(
                "beta",
                TriggerKind::Bus(BusTrigger {
                    topic: "gate.*".to_string(),
                }),
                "g2",
            ),
        ];

        TriggerBinding::save_all(&triggers_dir, &bindings).expect("save_all");
        let loaded = TriggerBinding::load_all(&triggers_dir).expect("load_all");
        assert_eq!(loaded.len(), 2);
        // Sorted by name.
        assert_eq!(loaded[0].name, "alpha");
        assert_eq!(loaded[1].name, "beta");
    }

    #[test]
    fn load_all_missing_dir_returns_empty() {
        let dir = tempfile::tempdir().expect("tempdir");
        let missing = dir.path().join("nonexistent");
        let result = TriggerBinding::load_all(&missing).expect("load_all missing");
        assert!(result.is_empty());
    }

    #[test]
    fn load_all_skips_non_toml_files() {
        let dir = tempfile::tempdir().expect("tempdir");
        let triggers_dir = dir.path().join("triggers");
        std::fs::create_dir_all(&triggers_dir).expect("mkdir");

        // Write a valid binding.
        let binding = TriggerBinding::new("real", TriggerKind::Manual, "g");
        binding
            .save_to_file(&triggers_dir.join("real.toml"))
            .expect("save");

        // Write a non-TOML file that should be ignored.
        std::fs::write(triggers_dir.join("readme.txt"), "not a trigger").expect("write txt");
        std::fs::write(triggers_dir.join("data.json"), "{}").expect("write json");

        let loaded = TriggerBinding::load_all(&triggers_dir).expect("load_all");
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].name, "real");
    }

    #[test]
    fn load_from_file_invalid_toml() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("bad.toml");
        std::fs::write(&path, "this is not valid TOML {{{{").expect("write");
        let err = TriggerBinding::load_from_file(&path).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
    }

    #[test]
    fn load_from_file_missing_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("does-not-exist.toml");
        let err = TriggerBinding::load_from_file(&path).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::NotFound);
    }

    #[test]
    fn round_trip_complex_binding() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("webhook-deploy.toml");

        let binding = TriggerBinding {
            name: "webhook-deploy".to_string(),
            kind: TriggerKind::Webhook(WebhookTrigger {
                method: Some("POST".to_string()),
                path: "/hook/deploy".to_string(),
            }),
            graph: "plans/deploy-pipeline.toml".to_string(),
            input_mapping: Some(TriggerInputMapping {
                mappings: vec![InputFieldMapping {
                    from: "$.payload.ref".to_string(),
                    to: "branch".to_string(),
                    transform: None,
                }],
            }),
            concurrency: ConcurrencyPolicy::CancelRunning,
            filter: Some(TriggerFilter {
                matches: None,
                debounce_ms: Some(5000),
                rate_limit: Some(RateLimit {
                    max_fires: 10,
                    window_ms: 60_000,
                    on_limit: RateLimitAction::Drop,
                }),
            }),
            enabled: true,
            space: Some("production".to_string()),
            auth: Some(TriggerAuth::HmacSha256 {
                secret: SecretRef::Env {
                    var: "WEBHOOK_SECRET".to_string(),
                },
                header: "X-Hub-Signature-256".to_string(),
            }),
            graduation_policy: TriggerGraduationPolicy::AfterSuccesses { count: 5 },
        };

        binding.save_to_file(&path).expect("save complex");
        let loaded = TriggerBinding::load_from_file(&path).expect("load complex");
        assert_eq!(loaded.name, "webhook-deploy");
        assert_eq!(loaded.graph, "plans/deploy-pipeline.toml");
        assert!(loaded.filter.is_some());
        assert_eq!(loaded.space.as_deref(), Some("production"));
    }

    #[test]
    fn secret_ref_variants_serialize() {
        let env_ref = SecretRef::Env {
            var: "MY_SECRET".to_string(),
        };
        let store_ref = SecretRef::Store {
            key: "my.key".to_string(),
        };
        let file_ref = SecretRef::File {
            path: PathBuf::from("/run/secrets/token"),
        };

        // All variants must round-trip through JSON without panicking.
        for sr in [env_ref, store_ref, file_ref] {
            let json = serde_json::to_string(&sr).expect("serialize SecretRef");
            let _: SecretRef = serde_json::from_str(&json).expect("deserialize SecretRef");
        }
    }

    #[test]
    fn trigger_source_all_variants_serialize() {
        let sources = vec![
            TriggerSource::Cron {
                expression: "*/5 * * * *".to_string(),
            },
            TriggerSource::Webhook {
                method: "POST".to_string(),
                path: "/hook".to_string(),
                headers: BTreeMap::new(),
            },
            TriggerSource::FileWatch {
                path: PathBuf::from("/tmp/watch"),
                event: FileWatchEvent::Modified,
            },
            TriggerSource::Bus {
                topic: "gate.*".to_string(),
                pulse_seq: 42,
            },
            TriggerSource::ChainEvent {
                chain_id: 1,
                block_number: 100,
                tx_hash: "0xabc".to_string(),
            },
            TriggerSource::Manual {
                user: "alice".to_string(),
            },
            TriggerSource::SignalPattern {
                matched_signals: vec!["s1".to_string()],
            },
        ];

        for src in sources {
            let json = serde_json::to_string(&src).expect("serialize TriggerSource");
            let _: TriggerSource = serde_json::from_str(&json).expect("deserialize TriggerSource");
        }
    }

    // ── Trigger event topic constant tests ───────────────────────────────

    #[test]
    fn trigger_topic_constants_have_correct_values() {
        assert_eq!(TRIGGER_FIRED, "trigger:fired");
        assert_eq!(TRIGGER_CREATED, "trigger:created");
        assert_eq!(TRIGGER_DELETED, "trigger:deleted");
        assert_eq!(TRIGGER_RATE_LIMITED, "trigger:rate_limited");
        assert_eq!(TRIGGER_AUTH_FAILED, "trigger:auth_failed");
    }

    #[test]
    fn trigger_topic_constants_are_unique() {
        let topics = [
            TRIGGER_FIRED,
            TRIGGER_CREATED,
            TRIGGER_DELETED,
            TRIGGER_RATE_LIMITED,
            TRIGGER_AUTH_FAILED,
        ];
        let mut seen = std::collections::HashSet::new();
        for topic in &topics {
            assert!(seen.insert(*topic), "duplicate topic constant: {topic}");
        }
    }

    // ── Trigger event graduation tests ──────────────────────────────────

    #[test]
    fn scoped_trigger_topics_follow_bus_contract() {
        assert_eq!(trigger_topic("deploy", "fired"), "trigger.deploy.fired");
    }

    #[test]
    fn only_durable_trigger_events_graduate() {
        for kind in GRADUATION_EVENTS {
            assert!(TriggerGraduationPolicy::should_graduate(&kind));
        }
        assert!(!TriggerGraduationPolicy::should_graduate(
            &TriggerEventKind::Filtered
        ));
        assert!(!TriggerGraduationPolicy::should_graduate(
            &TriggerEventKind::Queued
        ));
    }

    // ── TriggerGraduationPolicy tests ──────────────────────────────────

    #[test]
    fn graduation_policy_default_is_manual_only() {
        let policy = TriggerGraduationPolicy::default();
        assert_eq!(policy, TriggerGraduationPolicy::ManualOnly);
    }

    #[test]
    fn graduation_policy_all_variants_roundtrip_json() {
        let policies = [
            TriggerGraduationPolicy::ManualOnly,
            TriggerGraduationPolicy::AfterSuccesses { count: 10 },
            TriggerGraduationPolicy::TimeBased { min_age_hours: 48 },
        ];
        for policy in &policies {
            let json = serde_json::to_string(policy).expect("serialize");
            let decoded: TriggerGraduationPolicy =
                serde_json::from_str(&json).expect("deserialize");
            assert_eq!(&decoded, policy);
        }
    }

    #[test]
    fn graduation_policy_serde_tags() {
        // ManualOnly should serialize with policy tag "manual_only".
        let json = serde_json::to_string(&TriggerGraduationPolicy::ManualOnly).unwrap();
        assert!(json.contains("\"manual_only\""), "json: {json}");

        // AfterSuccesses should include the count field.
        let json =
            serde_json::to_string(&TriggerGraduationPolicy::AfterSuccesses { count: 3 }).unwrap();
        assert!(json.contains("\"after_successes\""), "json: {json}");
        assert!(json.contains("\"count\":3"), "json: {json}");

        // TimeBased should include min_age_hours.
        let json = serde_json::to_string(&TriggerGraduationPolicy::TimeBased { min_age_hours: 24 })
            .unwrap();
        assert!(json.contains("\"time_based\""), "json: {json}");
        assert!(json.contains("\"min_age_hours\":24"), "json: {json}");
    }

    #[test]
    fn binding_new_has_default_graduation_policy() {
        let b = TriggerBinding::new("test", TriggerKind::Manual, "g");
        assert_eq!(b.graduation_policy, TriggerGraduationPolicy::ManualOnly);
    }

    #[test]
    fn binding_graduation_policy_survives_json_roundtrip() {
        let mut b = TriggerBinding::new("grad-test", TriggerKind::Manual, "g");
        b.graduation_policy = TriggerGraduationPolicy::AfterSuccesses { count: 7 };
        let json = serde_json::to_string(&b).expect("serialize");
        let b2: TriggerBinding = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(
            b2.graduation_policy,
            TriggerGraduationPolicy::AfterSuccesses { count: 7 }
        );
    }

    #[test]
    fn binding_without_graduation_policy_deserializes_to_default() {
        // Simulate JSON from before the graduation_policy field existed.
        let json = r#"{
            "name": "legacy",
            "kind": {"type": "manual"},
            "graph": "g",
            "input_mapping": null,
            "concurrency": {"kind": "skip"},
            "filter": null,
            "enabled": true,
            "space": null,
            "auth": null
        }"#;
        let b: TriggerBinding = serde_json::from_str(json).expect("deserialize legacy");
        assert_eq!(b.graduation_policy, TriggerGraduationPolicy::ManualOnly);
    }
}
