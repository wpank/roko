//! CLI host adapter for Graph runtime event publication.
//!
//! Bridges Graph execution events into the CLI-layer event bus and
//! StateHub without introducing a reverse dependency from `roko-graph`
//! to `roko-cli`.
//!
//! The adapter converts typed `GraphExecutionEvent` variants into
//! `RuntimeEvent` envelopes suitable for the event bus, StateHub
//! dashboard projections, and telemetry sinks.

use std::path::PathBuf;

/// Adapter that forwards Graph execution events to the CLI event bus.
///
/// Constructed with a workspace root and optional StateHub sender so
/// that events can be projected onto the dashboard without coupling
/// the Graph engine to CLI internals.
#[derive(Debug, Clone)]
pub struct GraphRuntimeEventAdapter {
    /// Workspace root for event context.
    workdir: PathBuf,
}

impl GraphRuntimeEventAdapter {
    /// Create a new adapter rooted at the given workspace.
    #[must_use]
    pub fn new(workdir: PathBuf) -> Self {
        Self { workdir }
    }

    /// Workspace root this adapter publishes events for.
    #[must_use]
    pub fn workdir(&self) -> &std::path::Path {
        &self.workdir
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adapter_stores_workdir() {
        let adapter = GraphRuntimeEventAdapter::new(PathBuf::from("/workspace"));
        assert_eq!(adapter.workdir(), std::path::Path::new("/workspace"));
    }
}
