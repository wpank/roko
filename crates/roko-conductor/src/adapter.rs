//! Adapter wrapping conductor watchers as trigger-compatible event sources (E31-T04).
//!
//! This module bridges conductor watcher outputs to the generic `TriggerProtocol`
//! interface so that conductor health/stuck/circuit-breaker events can be used
//! as first-class trigger sources in the execution graph.

use serde::{Deserialize, Serialize};

/// Well-known action label: signal that a plan step failed.
pub const ACTION_FAIL: &str = "fail";

/// Well-known action label: signal that a supervisor should restart an agent.
pub const ACTION_RESTART: &str = "restart";

/// Output emitted by a [`TriggerCellAdapter`] when a conductor event fires.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TriggerCellOutput {
    /// The action label (e.g. [`ACTION_FAIL`], [`ACTION_RESTART`]).
    pub action: String,
    /// Human-readable reason for the trigger.
    pub reason: String,
}

/// Adapter that wraps a conductor watcher and exposes it as a trigger source.
///
/// Downstream graph cells subscribe to this adapter's output stream rather
/// than polling the conductor directly.
#[derive(Debug)]
pub struct TriggerCellAdapter {
    /// Name of the underlying conductor watcher.
    pub watcher_name: String,
}

impl TriggerCellAdapter {
    /// Create a new adapter for the named watcher.
    #[must_use]
    pub fn new(watcher_name: impl Into<String>) -> Self {
        Self {
            watcher_name: watcher_name.into(),
        }
    }
}
