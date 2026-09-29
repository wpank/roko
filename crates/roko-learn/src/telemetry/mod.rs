//! Attempt telemetry for S01 Phase 0: one identity per dispatch attempt,
//! and the append-only records keyed by it.
//!
//! - [`records`]: [`AttemptKey`], the attempt-open line, the settled
//!   [`VerdictRecord`], the [`RunManifest`] and the line envelope. Route
//!   decisions reuse [`RoutingDecisionLog`](crate::routing_log::RoutingDecisionLog).
//! - [`assign`](mod@assign): the one keyed-BLAKE3 assignment function.
//! - [`writer`]: the per-run [`TelemetryWriter`] and the durable
//!   [`AttemptOrdinals`].
//!
//! A run's files live in `.roko/runs/<run_id>/`: `attempts.jsonl` holds the
//! attempt-open lines and verdicts, next to `decisions.jsonl` and
//! `manifest.json`. Dispatch does not call any of this yet: gap-96f7ed
//! threads the key through dispatch, verification and feedback.

pub mod assign;
pub mod records;
pub mod writer;

pub use assign::{Arm, Assignment, AssignmentUnit, LayerSpec, assign};
pub use records::{
    AttemptCost, AttemptIdentity, AttemptKey, AttemptOpenRecord, AttemptOutcome, AttemptTiming,
    AttemptUsage, Blame, CostSource, DecisionSource, ExecutedModel, FailureClass, GateVerdict,
    RunFile, RunManifest, Stamped, StepVerdict, TelemetryRecord, VerdictRecord,
};
pub use writer::{
    AttemptOrdinals, TelemetryEvent, TelemetryWriter, TelemetryWriterConfig, TelemetryWriterStats,
};
