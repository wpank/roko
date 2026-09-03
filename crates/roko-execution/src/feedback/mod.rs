//! Graph completion feedback sinks (backlog #253).
//!
//! This module owns the canonical task attempt receipt, the 12-row serial
//! settlement pipeline, and the settlement ledger for durable idempotent
//! replay.
//!
//! # Architecture
//!
//! Each terminal task attempt produces one [`TaskAttemptReceiptV1`]. The
//! [`FeedbackSettler`] drives that receipt through exactly 12 sinks in fixed
//! order. Rows 0-2 are **critical** (failure stops settlement and leaves the
//! task terminal state uncommitted). Rows 3-11 are **optional** (failure
//! records degradation and continues).
//!
//! Host adapters (CLI, serve) supply concrete [`SettlementSink`]
//! implementations that delegate to existing stores. No replacement stores
//! are created by this module.
//!
//! # Idempotency
//!
//! The [`SettlementLedger`] tracks per-sink state. On resume, the settler
//! skips rows that are already `Settled` or `Skipped` and never calls the
//! provider again.

pub mod receipt;
pub mod settler;

pub use receipt::{
    AttemptTerminalStatus, ChoiceSource, RECEIPT_SCHEMA_VERSION, TaskAttemptReceiptV1,
};
pub use settler::{
    CRITICAL_SINK_COUNT, FeedbackSettler, SINK_KEYS, SettlementEvent, SettlementEventCallback,
    SettlementLedger, SettlementOutcome, SettlementSink, SinkError, SinkFailure,
    SinkSettlementEntry, SinkSettlementState,
};
