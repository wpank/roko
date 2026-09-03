//! CLI host adapter for Graph execution control commands.
//!
//! Bridges `ExecutionControlService` from `roko-graph` into the CLI-layer
//! runner control infrastructure (pause/resume/cancel/approve/reset).
//!
//! The adapter translates typed `ExecutionCommandKind` variants into
//! runner-v2 control signals without coupling the Graph engine to CLI
//! internals.

use std::path::PathBuf;

/// Adapter that translates Graph execution control commands into
/// runner-v2 control signals.
///
/// Constructed with a workspace root for command routing.
#[derive(Debug, Clone)]
pub struct GraphControlAdapter {
    /// Workspace root for control command context.
    workdir: PathBuf,
}

impl GraphControlAdapter {
    /// Create a new adapter rooted at the given workspace.
    #[must_use]
    pub fn new(workdir: PathBuf) -> Self {
        Self { workdir }
    }

    /// Workspace root this adapter dispatches commands for.
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
        let adapter = GraphControlAdapter::new(PathBuf::from("/workspace"));
        assert_eq!(adapter.workdir(), std::path::Path::new("/workspace"));
    }
}
