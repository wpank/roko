//! Legacy workflow facade marker (#258).
//!
//! #276 retired `WorkflowEngine`. A single prompt (`roko run`, the
//! single-agent `roko do` routes) now runs as a one-task plan through the
//! Graph engine via [`crate::run::run_prompt`]; this module keeps the frozen
//! marker and the `--engine` flag tests.

// ---------------------------------------------------------------------------
// Frozen legacy facade marker (#258)
// ---------------------------------------------------------------------------

/// Marker documenting that the legacy WorkflowEngine execution path has been
/// retired by #276. Prompts now run as one-task Graph plans.
pub const LEGACY_WORKFLOW_ENGINE_FROZEN: &str =
    "frozen by #258, retired by #276; all execution uses graph templates";

#[cfg(test)]
mod tests {
    use super::*;

    // ── Engine flag resolution ───────────────────────────────────────

    #[test]
    fn resolve_engine_flag_none_is_graph() {
        let result = crate::run::resolve_engine_flag(None);
        assert_eq!(result, "graph");
    }

    #[test]
    fn resolve_engine_flag_graph_is_graph() {
        let result = crate::run::resolve_engine_flag(Some("graph"));
        assert_eq!(result, "graph");
    }

    #[test]
    fn resolve_engine_flag_graph_canary_is_graph() {
        let result = crate::run::resolve_engine_flag(Some("graph_canary"));
        assert_eq!(result, "graph");
    }

    #[test]
    fn resolve_engine_flag_unknown_falls_back_to_graph() {
        let result = crate::run::resolve_engine_flag(Some("bogus"));
        assert_eq!(result, "graph");
    }

    // ── Frozen legacy marker ─────────────────────────────────────────

    #[test]
    fn frozen_marker_exists() {
        assert!(LEGACY_WORKFLOW_ENGINE_FROZEN.contains("frozen"));
        assert!(LEGACY_WORKFLOW_ENGINE_FROZEN.contains("#258"));
    }
}
