//! The M2 loop-liveness audit (S03). Every learning loop registers a
//! contract, and the auditor measures whether the loop's learned state
//! reaches the executed decision (exposure ε), changes it (net influence ι)
//! and pays for itself (benefit β) against a randomized default policy π⁰.
//!
//! - [`spec`]: the contract ([`LoopSpec`]), the closed reason codes and the
//!   registry: the embedded `loops.toml`, merged by loop id with a
//!   workspace's `.roko/learn/loop-registry.toml`.
//! - [`assign`]: loop layers, holdout schedules, nesting, the all-off arm
//!   and composed propensities over S01's one assignment function.
//! - [`arm_set`]: S02.P1-14's per-chain arm set, drawn once over those
//!   layers at attempt open and inherited by the chain's retries.
//!
//! The other modules hold one later piece of S03 each, so that they can be
//! built in parallel: [`exposure`] (ε, ι), [`estimators`] (β), [`cs`]
//! (confidence sequences), [`state`] (the state machine), [`ledger`] (the
//! A-LOOP rows), [`census`], [`canary`], [`faults`] and [`sim`].
//!
//! Per-run records (decisions, faults) stay in the run directory; the
//! cross-run loop-audit rows go in `.roko/learn` (decision 2201).

pub mod arm_set;
pub mod assign;
pub mod canary;
pub mod census;
pub mod cs;
pub mod estimators;
pub mod exposure;
pub mod faults;
pub mod ledger;
pub mod sim;
pub mod spec;
pub mod state;

pub use spec::{
    AuditState, Layer, Lifecycle, LoopId, LoopSpec, Qualifier, ReasonCode, ReceiptKind, Registry,
    RegistryError, StaticFinding,
};
