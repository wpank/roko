//! Stub cell implementations for graph nodes that don't have real implementations yet.
//!
//! `PassthroughCell` is a generic stub that passes input signals through unchanged
//! and logs a trace message. It is used for topology cells (plan.task-context,
//! plan.enricher.*, etc.) and any other graph nodes that need a placeholder.
//!
//! The legacy cognitive loop names (`signal-reader`, `relevance-scorer`, etc.)
//! are no longer stubs -- they are registered in `default_registry()` as aliases
//! for the real typed Cell implementations in `cells::cognitive`.

use std::time::Duration;

use roko_core::{ProtocolId, Signal, error::Result};

use crate::cell::{Cell, CellContext, CellVersion};

/// Stub cell that passes input signals through unchanged.
///
/// Used as a placeholder until the real implementation is built.
/// Each instance carries a name so logs indicate which stub was invoked.
pub struct PassthroughCell {
    /// Cell type name this stub represents.
    pub name: String,
}

impl PassthroughCell {
    /// Create a new passthrough stub with the given name.
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }
}

#[async_trait::async_trait]
impl Cell for PassthroughCell {
    fn cell_id(&self) -> &str {
        &self.name
    }

    fn cell_name(&self) -> &str {
        &self.name
    }

    fn cell_version(&self) -> CellVersion {
        (0, 1, 0)
    }

    fn is_stub(&self) -> bool {
        true
    }

    fn protocols(&self) -> Vec<ProtocolId> {
        Vec::new()
    }

    fn estimated_cost(&self) -> Option<f64> {
        None
    }

    fn estimated_duration(&self) -> Option<Duration> {
        Some(Duration::from_millis(1))
    }

    async fn execute(&self, input: Vec<Signal>, _ctx: &CellContext) -> Result<Vec<Signal>> {
        tracing::info!(
            cell = %self.name,
            input_count = input.len(),
            "PassthroughCell '{}' -- {} input signals (stub)",
            self.name,
            input.len()
        );
        Ok(input)
    }
}

/// Legacy cognitive loop alias names.
///
/// These names are registered in `default_registry()` as aliases for the real
/// cognitive Cell implementations (`SenseCell`, `AssessCell`, etc.). They are
/// no longer `PassthroughCell` stubs -- each delegates to the corresponding
/// typed Cell from `cells::cognitive`.
pub const COGNITIVE_LOOP_ALIASES: &[&str] = &[
    "signal-reader",
    "relevance-scorer",
    "system-prompt-builder",
    "claude-agent",
    "gate-pipeline",
    "store-writer",
    "event-publisher",
];
