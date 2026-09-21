//! Stub cell implementations for graph nodes that don't have real implementations yet.
//!
//! `PassthroughCell` is a generic stub that passes input signals through unchanged
//! and logs a trace message. It is used for enricher topology cells
//! (`plan.enricher.*`), the `plan.success-boundary` anchor, and any other graph
//! nodes that still need a placeholder.
//!
//! The following topology cells have real implementations and are **no longer stubs**:
//! - `plan.task-context` → `TaskContextCell` (see `cells/task_context.rs`)
//! - `plan.compose` → `PlanComposeCell` (see `cells/plan_compose.rs`)
//! - `plan.gate` → `PlanGateCell` (see `cells/plan_gate.rs`)
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

    async fn execute(&self, input: Vec<Signal>, ctx: &CellContext) -> Result<Vec<Signal>> {
        tracing::warn!(
            cell_type = %self.name,
            node_id = %ctx.cell_id.as_deref().unwrap_or("<unknown>"),
            run_id = %ctx.run_id.as_deref().unwrap_or("<unknown>"),
            input_count = input.len(),
            "stub cell executed: '{}' is a PassthroughCell placeholder and has no real \
             implementation — outputs are identical to inputs. Replace this stub before \
             production use.",
            self.name,
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
