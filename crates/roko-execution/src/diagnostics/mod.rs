//! Shared diagnostic and execution preflight service.
//!
//! Consolidates workspace health checks that were previously duplicated
//! across `doctor.rs`, `preflight.rs`, `config_cmd.rs`, and `chat_inline.rs`.
//!
//! # Architecture
//!
//! - [`types`] -- Core types: `DiagnosticCheckId`, `DiagnosticSeverity`,
//!   `DiagnosticFinding`, `DiagnosticRequest`, `DiagnosticReport`.
//! - [`checks`] -- One function per check ID (11 total).
//! - [`service`] -- `DiagnosticService::run()` dispatches selected checks
//!   and returns a sorted report.
//!
//! # Usage
//!
//! ```ignore
//! use roko_execution::diagnostics::{DiagnosticService, DiagnosticRequest, DiagnosticCheckId};
//! use std::collections::BTreeSet;
//!
//! let request = DiagnosticRequest {
//!     workdir: ".".into(),
//!     selected: [DiagnosticCheckId::Config, DiagnosticCheckId::Git]
//!         .into_iter().collect(),
//!     profile: None,
//!     allow_repairs: false,
//! };
//! let report = DiagnosticService::run(&request);
//! ```

pub mod checks;
pub mod service;
pub mod types;

pub use service::{DiagnosticRepairError, DiagnosticService};
pub use types::{
    DiagnosticCheckId, DiagnosticFinding, DiagnosticRemediation, DiagnosticReport,
    DiagnosticRequest, DiagnosticSeverity,
};
