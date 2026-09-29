//! Attempt telemetry for S01 Phase 0: one identity per dispatch attempt,
//! and the append-only records keyed by it.
//!
//! - [`records`]: [`AttemptKey`], the attempt-open line, the settled
//!   [`AttemptVerdictRecord`], the [`RunProvenanceManifest`] and the line
//!   envelope. Route decisions reuse
//!   [`RoutingDecisionLog`](crate::routing_log::RoutingDecisionLog).
//! - [`assign`](mod@assign): the one keyed-BLAKE3 assignment function.
//! - [`writer`]: the per-run [`TelemetryWriter`] and the durable
//!   [`AttemptOrdinals`].
//!
//! A run's files live in `.roko/runs/<run_id>/`: `attempts.jsonl` holds the
//! attempt-open lines and verdicts, next to `decisions.jsonl` and
//! `manifest.json`. Dispatch does not call any of this yet: gap-96f7ed
//! threads the key through dispatch, verification and feedback.
//!
//! The schemas and field names are S01's. Some type names are not, because
//! the workspace already uses S01's names for other types: the verdict is
//! `AttemptVerdictRecord` (`verdict_scorer::VerdictRecord` exists), the
//! manifest is `RunProvenanceManifest` (roko-runtime has a `RunManifest`),
//! and likewise `GateVerdictTag`, `AttemptFailureClass`, `VerifyStepVerdict`
//! and `ConfigHashProvenance`.

pub mod assign;
pub mod records;
pub mod writer;

pub use assign::{Arm, Assignment, AssignmentUnit, LayerSpec, assign};
pub use records::{
    AttemptCost, AttemptFailureClass, AttemptIdentity, AttemptKey, AttemptOpenRecord,
    AttemptOutcome, AttemptTiming, AttemptUsage, AttemptVerdictRecord, Blame, CostSource,
    DecisionSource, ExecutedModel, GateVerdictTag, RunFile, RunProvenanceManifest, Stamped,
    TelemetryRecord, VerifyStepVerdict,
};
pub use writer::{
    AttemptOrdinals, TelemetryEvent, TelemetryWriter, TelemetryWriterConfig, TelemetryWriterStats,
};
