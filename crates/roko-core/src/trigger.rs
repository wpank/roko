//! Trigger protocol types for roko-core.
//!
//! This module provides the trigger binding, event, and protocol types used
//! by the agent runtime to register and evaluate triggers. It mirrors the
//! design of the roko-plugin manifest trigger DSL but at the core layer
//! without any plugin-specific dependencies.
//!
//! # Key types
//!
//! - [`TriggerProtocol`] — the wire protocol for trigger registration.
//! - [`TriggerBinding`] — a concrete binding between a trigger and its action.
//! - [`TriggerEvent`] — a fired trigger event carried on the event bus.
//! - [`TriggerKind`] — discriminant enum.
//! - [`TriggerHandle`] — a cancellable handle to a registered trigger.
//! - [`TriggerState`] — runtime lifecycle state of a trigger.

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

// ─── basic identifiers ────────────────────────────────────────────────────────

/// A unique trigger identifier (newtype around `String`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TriggerId(pub String);

/// A plan/graph space identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SpaceId(pub String);

/// A graph reference (name or UUID).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct GraphRef(pub String);

/// An opaque trace ID for correlation.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TraceId(pub String);

/// A reference to a named secret.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecretRef {
    /// The name/key of the secret.
    pub name: String,
}

/// A reference to a named signal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignalRef {
    /// The kind of the signal being referenced.
    pub kind: String,
}

// ─── rate limit ───────────────────────────────────────────────────────────────

/// Action to take when a rate limit is exceeded.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RateLimitAction {
    /// Queue the trigger until the window resets.
    Queue,
    /// Drop the trigger silently.
    Drop,
    /// Return an error to the caller.
    Error,
}

/// Rate-limiting configuration for a trigger.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RateLimit {
    /// Maximum number of fires in the window.
    pub max: u32,
    /// Window duration in seconds.
    pub window_secs: u64,
    /// Action when the limit is exceeded.
    pub action: RateLimitAction,
}

// ─── concurrency policy ───────────────────────────────────────────────────────

/// Policy for concurrent trigger evaluations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConcurrencyPolicy {
    /// Allow any number of concurrent evaluations.
    Allow,
    /// Queue new evaluations until the current one finishes.
    Queue,
    /// Skip if an evaluation is already running.
    Skip,
    /// Abort the current evaluation and start a new one.
    Replace,
}

// ─── expression ───────────────────────────────────────────────────────────────

/// A simple expression for trigger condition guards.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Expr(pub String);

// ─── auth ─────────────────────────────────────────────────────────────────────

/// Authentication configuration for webhook triggers.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TriggerAuth {
    /// Secret used for HMAC signature verification.
    pub secret: Option<SecretRef>,
    /// Required HTTP header name/value pairs.
    pub headers: HashMap<String, String>,
}

// ─── filter ───────────────────────────────────────────────────────────────────

/// A filter applied to incoming trigger events.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TriggerFilter {
    /// CEL expression that must evaluate to `true`.
    pub expr: Option<Expr>,
    /// Signal kinds to match (for signal-pattern triggers).
    pub signal_kinds: Vec<String>,
}

// ─── input mapping ────────────────────────────────────────────────────────────

/// A mapping from trigger event fields to graph input fields.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InputFieldMapping {
    /// Source field path in the trigger event.
    pub from: String,
    /// Destination field name in the graph input.
    pub to: String,
}

/// Input mapping configuration for a trigger.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TriggerInputMapping {
    /// Field mappings.
    pub fields: Vec<InputFieldMapping>,
    /// Default input values.
    pub defaults: HashMap<String, serde_json::Value>,
}

// ─── trigger source ───────────────────────────────────────────────────────────

/// The runtime source that fires a trigger.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum TriggerSource {
    /// Cron-based schedule.
    Cron { schedule: String },
    /// Signal pattern match.
    SignalPattern { kinds: Vec<String> },
    /// File system watch.
    FileWatch { paths: Vec<String> },
    /// Inbound webhook.
    Webhook { path: String },
    /// Bus event subscription.
    Bus { topic: String },
    /// Chain event subscription.
    ChainEvent { event_signature: String },
    /// Explicit manual invocation.
    Manual,
}

// ─── concrete trigger types ───────────────────────────────────────────────────

/// A cron-scheduled trigger.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CronTrigger {
    /// POSIX cron expression (e.g. `"0 * * * *"` = every hour).
    pub schedule: String,
    /// Optional timezone (defaults to UTC).
    pub timezone: Option<String>,
    /// Rate-limit settings.
    pub rate_limit: Option<RateLimit>,
    /// Concurrency policy.
    pub concurrency: ConcurrencyPolicy,
}

/// A trigger that fires when a signal matching a pattern arrives on the bus.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SignalPatternTrigger {
    /// Signal kinds to watch.
    pub kinds: Vec<String>,
    /// Optional CEL filter expression.
    pub filter: Option<Expr>,
    /// Concurrency policy.
    pub concurrency: ConcurrencyPolicy,
}

/// A filesystem-watch trigger.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FileWatchTrigger {
    /// Paths or globs to watch.
    pub paths: Vec<String>,
    /// Events to observe (`"create"`, `"modify"`, `"delete"`).
    pub events: Vec<String>,
    /// Debounce delay in milliseconds.
    pub debounce_ms: Option<u64>,
}

/// An inbound webhook trigger.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WebhookTrigger {
    /// HTTP path to listen on (relative to the serve base URL).
    pub path: String,
    /// Accepted HTTP methods.
    pub methods: Vec<String>,
    /// Authentication configuration.
    pub auth: Option<TriggerAuth>,
    /// Rate-limit settings.
    pub rate_limit: Option<RateLimit>,
}

/// A bus-subscription trigger.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BusTrigger {
    /// Bus topic to subscribe to.
    pub topic: String,
    /// Optional filter expression.
    pub filter: Option<Expr>,
    /// Concurrency policy.
    pub concurrency: ConcurrencyPolicy,
}

/// A chain-event trigger.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChainEventTrigger {
    /// Solidity-style event signature (e.g. `"Transfer(address,address,uint256)"`).
    pub event_signature: String,
    /// Contract address filter (optional).
    pub contract: Option<String>,
    /// Concurrency policy.
    pub concurrency: ConcurrencyPolicy,
}

/// A filesystem-watch event reported to a [`FileWatchTrigger`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileWatchEvent {
    /// Kind of filesystem event.
    pub kind: String,
    /// Affected path.
    pub path: String,
}

// ─── TriggerKind ─────────────────────────────────────────────────────────────

/// Discriminant enum for the kind of a trigger.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TriggerKind {
    /// Fires on a cron schedule.
    Cron,
    /// Fires when a matching signal is observed.
    SignalPattern,
    /// Fires on filesystem changes.
    FileWatch,
    /// Fires on an inbound webhook request.
    Webhook,
    /// Fires on a bus message.
    Bus,
    /// Fires on a chain event.
    ChainEvent,
    /// Fires only on explicit manual invocation.
    Manual,
}

// ─── TriggerState ─────────────────────────────────────────────────────────────

/// Runtime lifecycle state of a registered trigger.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TriggerState {
    /// The trigger is registered and actively watching for events.
    Active,
    /// The trigger is registered but suspended (not evaluating).
    Suspended,
    /// The trigger has been unregistered.
    Cancelled,
    /// The trigger encountered an error during evaluation.
    Faulted,
}

// ─── TriggerEvent ─────────────────────────────────────────────────────────────

/// A fired trigger event, carried on the internal event bus.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerEvent {
    /// The ID of the trigger that fired.
    pub trigger_id: TriggerId,
    /// The kind of trigger.
    pub kind: TriggerKind,
    /// Timestamp when the event fired (Unix seconds).
    pub fired_at: i64,
    /// Payload data for the event (varies by trigger kind).
    pub payload: serde_json::Value,
    /// Optional correlation trace ID.
    pub trace_id: Option<TraceId>,
}

// ─── TriggerBinding ───────────────────────────────────────────────────────────

/// A concrete binding between a trigger source and a graph/action target.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerBinding {
    /// Unique trigger identifier.
    pub id: TriggerId,
    /// Human-readable label.
    pub label: Option<String>,
    /// The trigger source that fires this binding.
    pub source: TriggerSource,
    /// Target graph reference.
    pub graph: Option<GraphRef>,
    /// Space context.
    pub space: Option<SpaceId>,
    /// Input mapping from event fields to graph inputs.
    pub input_mapping: Option<TriggerInputMapping>,
    /// Filter to apply before firing.
    pub filter: Option<TriggerFilter>,
    /// Whether the trigger is currently enabled.
    pub enabled: bool,
}

// ─── TriggerHandle ────────────────────────────────────────────────────────────

/// A cancellable handle to a registered trigger.
///
/// Drop this handle or call [`TriggerHandle::cancel`] to deregister the
/// trigger.
#[derive(Debug)]
pub struct TriggerHandle {
    /// Trigger ID.
    pub id: TriggerId,
    /// Internal cancellation sender.
    cancel_tx: tokio::sync::watch::Sender<bool>,
}

impl TriggerHandle {
    /// Create a new handle for `id`.
    pub fn new(id: TriggerId) -> (Self, tokio::sync::watch::Receiver<bool>) {
        let (cancel_tx, cancel_rx) = tokio::sync::watch::channel(false);
        (Self { id, cancel_tx }, cancel_rx)
    }

    /// Signal cancellation of the trigger.
    pub fn cancel(&self) {
        let _ = self.cancel_tx.send(true);
    }
}

// ─── TriggerProtocol ──────────────────────────────────────────────────────────

/// Wire-protocol request/response types for trigger registration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerProtocol {
    /// The binding to register.
    pub binding: TriggerBinding,
    /// Optional authentication token for the registration call.
    pub auth_token: Option<String>,
}

// ─── Author ───────────────────────────────────────────────────────────────────

/// The author or origin of a trigger.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Author {
    /// Created by a user.
    User(String),
    /// Created by a plan task.
    Plan(String),
    /// Created by the system.
    System,
}
