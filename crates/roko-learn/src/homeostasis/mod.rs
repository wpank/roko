//! M1, the ultrastable controller (S06): a second-order loop that keeps four
//! essential variables inside the S5 viability bounds by step changes to
//! `HarnessParams` (θ, `roko_core::config::harness_params`).
//!
//! M1 reads settled task resolutions, never one attempt's signal ring: it is
//! not the conductor, shares none of its state and never restarts or fails an
//! attempt. Its parts, one module each:
//!
//! - [`resolution`]: the fold of settled verdicts (or pre-S01 logs) into task
//!   resolutions, M1's unit (A-RES).
//! - [`ev`]: the essential-variable estimators, Schmitt bands and the drive.
//! - [`spc`]: the CUSUM and EWMA charts (moved from roko-gate, which
//!   re-exports them).
//! - [`detect`]: change detectors over those charts.
//! - [`policy`]: the `SafetyBox` validator and the read-only S5 policy.
//! - [`catalog`]: the move catalog and the requisite-variety matrix.
//! - [`controller`]: IDLE, SEARCH and HOLD.
//! - [`lkg`]: θ's last-known-good versions.
//! - [`ledger`]: the A-CTL controller records.
//! - [`streams`]: historical and synthetic resolution streams for replay.
//! - [`saso`]: step-response metrics and IAE.
//! - [`replay`]: the full-information replay evaluator.
//! - [`holdout`]: the static `harness_policy` holdout.
//! - [`priors`]: move priors seeded from the self-model (M3).
//! - [`coupling`]: audit coupling with M4.

pub mod catalog;
pub mod controller;
pub mod coupling;
pub mod detect;
pub mod ev;
pub mod holdout;
pub mod ledger;
pub mod lkg;
pub mod policy;
pub mod priors;
pub mod replay;
pub mod resolution;
pub mod saso;
pub mod spc;
pub mod streams;
