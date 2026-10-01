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
//! - [`manifest`]: reading and writing `manifest.json`, and the
//!   [`AttemptTally`] of a run's attempts.
//! - [`report`]: the read-only `check` and `route_report` over a run's
//!   files (`roko learn telemetry`).
//!
//! A run's files live in `.roko/runs/<run_id>/`: `attempts.jsonl` holds the
//! attempt-open lines and verdicts, next to `decisions.jsonl` and
//! `manifest.json`. Graph task dispatch mints the key, writes both lines and
//! stamps the legacy efficiency and cost rows with it ([`AttemptKeyed`]).
//!
//! The schemas and field names are S01's. Some type names are not, because
//! the workspace already uses S01's names for other types: the verdict is
//! `AttemptVerdictRecord` (`verdict_scorer::VerdictRecord` exists), the
//! manifest is `RunProvenanceManifest` (roko-runtime has a `RunManifest`),
//! and likewise `GateVerdictTag`, `AttemptFailureClass`, `VerifyStepVerdict`
//! and `ConfigHashProvenance`.

pub mod assign;
pub mod manifest;
pub mod records;
pub mod report;
pub mod writer;

pub use assign::{Arm, Assignment, AssignmentUnit, LayerSpec, assign};
pub use manifest::AttemptTally;
pub use records::{
    AttemptCost, AttemptFailureClass, AttemptIdentity, AttemptKey, AttemptKeyed, AttemptLadder,
    AttemptOpenRecord, AttemptOutcome, AttemptTiming, AttemptUsage, AttemptVerdictRecord, Blame,
    CostSource, DecisionSource, ExecutedModel, FailoverRefusal, GateVerdictTag, HelperCallsUsage,
    LadderReason, RunFile, RunProvenanceManifest, Stamped, TelemetryRecord, VerifyStepVerdict,
};
pub use writer::{
    AttemptOrdinals, TelemetryEvent, TelemetryWriter, TelemetryWriterConfig, TelemetryWriterStats,
};
