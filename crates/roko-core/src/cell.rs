//! The Cell trait — universal computation unit for all protocol implementations.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio_util::sync::CancellationToken;

use crate::bus_backends::BusErased;
use crate::error::{Result, RokoError};
use crate::traits::Substrate;
use crate::{Engram, Kind};

// ─── ProtocolId ─────────────────────────────────────────────────────────────

/// Typed identifier for the 9 protocol conformances a Cell can declare.
///
/// Used for compile-time protocol checking and runtime introspection.
/// Matches the 9 protocols defined in 02-CELL.md section 2.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ProtocolId {
    /// Persisted signal storage (Substrate trait).
    Store,
    /// Signal scoring (Score trait).
    Score,
    /// Gate / verification (Verify trait).
    Verify,
    /// Signal routing (Route trait).
    Route,
    /// Prompt / signal composition (Compose trait).
    Compose,
    /// Reactive policy (React trait).
    React,
    /// Passive observation emitter (Observe trait).
    Observe,
    /// External connection lifecycle (Connect trait).
    Connect,
    /// Event-driven trigger (Trigger trait).
    Trigger,
}

/// Stable identifier for a Cell instance.
pub type CellId = String;

/// Semantic version tuple for Cell implementations.
pub type CellVersion = (u32, u32, u32);

// ─── CellContext ────────────────────────────────────────────────────────────

/// Runtime context passed to Cell::execute(). Provides access to
/// shared infrastructure without cells needing to manage their own.
pub struct CellContext {
    /// Pub/sub transport for ephemeral Pulses (type-erased).
    pub bus: Arc<dyn BusErased>,
    /// Durable storage for Engrams.
    pub store: Arc<dyn Substrate>,
    /// Cancellation token for cooperative shutdown.
    pub cancel: CancellationToken,
    /// Trace context for observability.
    pub trace_id: Option<String>,
    /// Run identifier (if executing within a Graph/Flow).
    pub run_id: Option<String>,
    /// Remaining budget for this execution (USD).
    pub budget_remaining: Option<f64>,
}

impl CellContext {
    /// Construct a new `CellContext` with the required infrastructure handles.
    #[must_use]
    pub fn new(
        bus: Arc<dyn BusErased>,
        store: Arc<dyn Substrate>,
        cancel: CancellationToken,
    ) -> Self {
        Self {
            bus,
            store,
            cancel,
            trace_id: None,
            run_id: None,
            budget_remaining: None,
        }
    }
}

// ─── Capabilities ───────────────────────────────────────────────────────────

/// Declares the runtime capabilities a Cell requires to execute.
///
/// This is declaration only -- enforcement is the Graph/Engine's job.
/// By default all capabilities are `false` (via `Default`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Capabilities {
    /// Cell makes outbound network calls (HTTP, DNS, WebSocket, etc.).
    pub network: bool,
    /// Cell reads from or writes to the local filesystem.
    pub file_system: bool,
    /// Cell spawns child processes or runs shell commands.
    pub subprocess: bool,
    /// Cell invokes an LLM API (OpenAI, Anthropic, Ollama, etc.).
    pub llm_access: bool,
    /// Cell interacts with an on-chain contract or node RPC.
    pub chain_access: bool,
    /// Cell publishes Pulses to the shared Bus.
    pub bus_publish: bool,
    /// Cell writes Engrams to the durable Store.
    pub store_write: bool,
}

impl Capabilities {
    /// All capabilities disabled -- safe sandbox default.
    #[must_use]
    pub fn none() -> Self {
        Self::default()
    }

    /// All capabilities enabled -- unrestricted execution.
    #[must_use]
    pub fn all() -> Self {
        Self {
            network: true,
            file_system: true,
            subprocess: true,
            llm_access: true,
            chain_access: true,
            bus_publish: true,
            store_write: true,
        }
    }

    /// Returns `true` if every capability required by `self` is also present
    /// in `other`.
    #[must_use]
    pub fn is_subset_of(&self, other: &Capabilities) -> bool {
        (!self.network || other.network)
            && (!self.file_system || other.file_system)
            && (!self.subprocess || other.subprocess)
            && (!self.llm_access || other.llm_access)
            && (!self.chain_access || other.chain_access)
            && (!self.bus_publish || other.bus_publish)
            && (!self.store_write || other.store_write)
    }

    /// Returns the union of two `Capabilities` sets.
    #[must_use]
    pub fn union(&self, other: &Capabilities) -> Capabilities {
        Capabilities {
            network: self.network || other.network,
            file_system: self.file_system || other.file_system,
            subprocess: self.subprocess || other.subprocess,
            llm_access: self.llm_access || other.llm_access,
            chain_access: self.chain_access || other.chain_access,
            bus_publish: self.bus_publish || other.bus_publish,
            store_write: self.store_write || other.store_write,
        }
    }
}

// ─── TypeSchema ─────────────────────────────────────────────────────────────

/// Describes the input or output type contract of a Cell for edge validation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TypeSchema {
    /// Accepts any input.
    Any,
    /// Accepts signals of a specific Kind.
    OfKind(Kind),
    /// Accepts signals matching a JSON schema string.
    JsonSchema(String),
}

impl TypeSchema {
    /// Check if an output of type `self` is compatible as input to a cell
    /// expecting `target`.
    ///
    /// Two `JsonSchema` variants are compatible when their schema strings are
    /// equal (exact match).
    #[must_use]
    pub fn is_compatible_with(&self, target: &TypeSchema) -> bool {
        match (self, target) {
            (_, TypeSchema::Any) => true,
            (TypeSchema::Any, _) => true,
            (TypeSchema::OfKind(a), TypeSchema::OfKind(b)) => a == b,
            (TypeSchema::JsonSchema(a), TypeSchema::JsonSchema(b)) => a == b,
            _ => false,
        }
    }

    /// Check whether the given `signal` satisfies this schema.
    ///
    /// - `Any` always passes.
    /// - `OfKind(k)` passes when `signal.kind == k`.
    /// - `JsonSchema(_)` always passes (full structural validation is deferred).
    #[must_use]
    pub fn validate_signal(&self, signal: &Engram) -> bool {
        match self {
            TypeSchema::Any => true,
            TypeSchema::OfKind(k) => &signal.kind == k,
            TypeSchema::JsonSchema(_) => true,
        }
    }

    /// Combine two schemas at a merge point, conservatively.
    ///
    /// - If either side is `Any`, the union is `Any`.
    /// - If both are `OfKind` with the same kind, the result is that `OfKind`.
    /// - Otherwise the result is `Any` (conservative fallback).
    #[must_use]
    pub fn union(self, other: TypeSchema) -> TypeSchema {
        match (self, other) {
            (TypeSchema::Any, _) | (_, TypeSchema::Any) => TypeSchema::Any,
            (TypeSchema::OfKind(a), TypeSchema::OfKind(b)) if a == b => TypeSchema::OfKind(a),
            _ => TypeSchema::Any,
        }
    }
}

// ─── Cell trait ─────────────────────────────────────────────────────────────

/// Universal computation unit. Every protocol trait (Substrate, Scorer, Gate,
/// Router, Composer, Policy) requires `Cell` as a supertrait, giving the
/// execution engine identity, cost estimation, and protocol introspection.
#[async_trait]
pub trait Cell: Send + Sync + 'static {
    /// Unique identifier for this cell instance.
    fn cell_id(&self) -> &str;
    /// Human-readable name for display and logging.
    fn cell_name(&self) -> &str;
    /// Semantic version of this cell's implementation.
    fn cell_version(&self) -> CellVersion {
        (0, 1, 0)
    }
    /// Protocol conformances this cell declares (typed).
    fn protocols(&self) -> Vec<ProtocolId> {
        Vec::new()
    }
    /// Convenience: check if this cell conforms to a given protocol.
    fn has_protocol(&self, id: ProtocolId) -> bool {
        self.protocols().contains(&id)
    }
    /// Runtime capabilities this cell requires to execute.
    ///
    /// The engine uses this to verify the execution scope grants sufficient
    /// capabilities before dispatching. Default is all-false (sandboxed).
    fn capabilities(&self) -> Capabilities {
        Capabilities::default()
    }
    /// Estimated USD cost per invocation, when known.
    fn estimated_cost(&self) -> Option<f64> {
        None
    }
    /// Estimated wall-clock duration per invocation, when known.
    fn estimated_duration(&self) -> Option<Duration> {
        None
    }

    // ─── v2 additions ───────────────────────────────────────────────────

    /// Describes the input type this cell expects. `None` means untyped.
    fn input_schema(&self) -> Option<&TypeSchema> {
        None
    }

    /// Describes the output type this cell produces. `None` means untyped.
    fn output_schema(&self) -> Option<&TypeSchema> {
        None
    }

    /// Execute this cell. Default returns an error -- override in implementations.
    async fn execute(&self, input: Vec<Engram>, ctx: &CellContext) -> Result<Vec<Engram>> {
        let _ = (input, ctx);
        Err(RokoError::Invalid(format!(
            "{}: execute() not implemented",
            self.cell_name()
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn schema_any() -> TypeSchema {
        TypeSchema::Any
    }

    fn schema_task() -> TypeSchema {
        TypeSchema::OfKind(Kind::Task)
    }

    fn schema_episode() -> TypeSchema {
        TypeSchema::OfKind(Kind::Episode)
    }

    fn schema_json(s: &str) -> TypeSchema {
        TypeSchema::JsonSchema(s.to_string())
    }

    #[test]
    fn any_is_compatible_with_anything() {
        assert!(schema_any().is_compatible_with(&schema_any()));
        assert!(schema_any().is_compatible_with(&schema_task()));
        assert!(schema_any().is_compatible_with(&schema_json("{}")));
    }

    #[test]
    fn anything_is_compatible_with_any() {
        assert!(schema_task().is_compatible_with(&schema_any()));
        assert!(schema_json("{}").is_compatible_with(&schema_any()));
    }

    #[test]
    fn of_kind_compatible_when_same() {
        assert!(schema_task().is_compatible_with(&schema_task()));
    }

    #[test]
    fn of_kind_not_compatible_when_different() {
        assert!(!schema_task().is_compatible_with(&schema_episode()));
        assert!(!schema_episode().is_compatible_with(&schema_task()));
    }

    #[test]
    fn json_schema_compatible_when_strings_equal() {
        let a = schema_json(r#"{"type":"object"}"#);
        let b = schema_json(r#"{"type":"object"}"#);
        assert!(a.is_compatible_with(&b));
    }

    #[test]
    fn json_schema_not_compatible_when_strings_differ() {
        let a = schema_json(r#"{"type":"object"}"#);
        let b = schema_json(r#"{"type":"string"}"#);
        assert!(!a.is_compatible_with(&b));
    }

    #[test]
    fn of_kind_and_json_schema_not_compatible() {
        assert!(!schema_task().is_compatible_with(&schema_json("{}")));
        assert!(!schema_json("{}").is_compatible_with(&schema_task()));
    }

    #[test]
    fn any_validates_all_signals() {
        let task = Engram::builder(Kind::Task).build();
        let ep = Engram::builder(Kind::Episode).build();
        assert!(schema_any().validate_signal(&task));
        assert!(schema_any().validate_signal(&ep));
    }

    #[test]
    fn of_kind_validates_matching_signal() {
        let task = Engram::builder(Kind::Task).build();
        let ep = Engram::builder(Kind::Episode).build();
        assert!(schema_task().validate_signal(&task));
        assert!(!schema_task().validate_signal(&ep));
    }

    #[test]
    fn json_schema_validates_all_signals_best_effort() {
        let task = Engram::builder(Kind::Task).build();
        let ep = Engram::builder(Kind::Episode).build();
        assert!(schema_json("{}").validate_signal(&task));
        assert!(schema_json(r#"{"type":"object"}"#).validate_signal(&ep));
    }

    #[test]
    fn union_with_any_gives_any() {
        assert_eq!(schema_task().union(schema_any()), TypeSchema::Any);
        assert_eq!(schema_any().union(schema_task()), TypeSchema::Any);
        assert_eq!(schema_any().union(schema_any()), TypeSchema::Any);
    }

    #[test]
    fn union_of_same_kind_gives_that_kind() {
        assert_eq!(
            schema_task().union(schema_task()),
            TypeSchema::OfKind(Kind::Task)
        );
    }

    #[test]
    fn union_of_different_kinds_gives_any() {
        assert_eq!(schema_task().union(schema_episode()), TypeSchema::Any);
    }

    #[test]
    fn union_of_json_schemas_gives_any() {
        assert_eq!(schema_json("{}").union(schema_json("{}")), TypeSchema::Any);
    }

    #[test]
    fn capabilities_none_is_default() {
        assert_eq!(Capabilities::none(), Capabilities::default());
    }

    #[test]
    fn capabilities_subset_checks() {
        let net = Capabilities {
            network: true,
            ..Default::default()
        };
        let all = Capabilities::all();
        assert!(net.is_subset_of(&all));
        assert!(!all.is_subset_of(&net));
    }

    #[test]
    fn capabilities_union_merges() {
        let a = Capabilities {
            network: true,
            ..Default::default()
        };
        let b = Capabilities {
            llm_access: true,
            ..Default::default()
        };
        let u = a.union(&b);
        assert!(u.network);
        assert!(u.llm_access);
        assert!(!u.file_system);
    }
}
