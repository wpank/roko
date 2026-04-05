//! Safety subsystems for Roko orchestrator.
//!
//! These modules gate privileged operations (capability tokens), record
//! tamper-evident audit trails, grant scoped permits, detect runaway loops,
//! propagate taint across signals, and enforce sandboxing.

pub mod audit_chain;
pub mod capability_tokens;
pub mod loop_guard;
pub mod permit;
pub mod sandboxing;
pub mod taint_propagation;
