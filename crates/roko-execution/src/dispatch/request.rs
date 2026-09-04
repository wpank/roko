//! Layer-safe dispatch request types.
//!
//! These types carry the neutral information needed to dispatch an agent
//! without depending on runner state, TUI types, or CLI argument structures.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Neutral dispatch request that any execution surface can construct.
///
/// This is the layer-3 equivalent of the per-call `DispatchContext` in the
/// CLI dispatch module. CLI code converts its `DispatchContext` into this
/// type; Graph and serve callers construct it directly.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DispatchRequest {
    /// Plan ID this task belongs to.
    pub plan_id: String,
    /// Logical role name ("implementer", "reviewer", ...).
    pub role: String,
    /// Working directory for the agent.
    pub workdir: PathBuf,
    /// Model override from CLI / config.
    pub model_hint: Option<String>,
    /// Highest-priority model slug override (manual operator decision).
    pub force_backend: Option<String>,
    /// Remaining USD budget for the plan.
    pub budget_remaining_usd: f64,
    /// Attempt number (0 = first try).
    pub attempt: u32,
    /// Output files from completed dependency tasks.
    pub dependency_outputs: Vec<(String, Vec<String>)>,
}

impl Default for DispatchRequest {
    fn default() -> Self {
        Self {
            plan_id: String::new(),
            role: "implementer".into(),
            workdir: PathBuf::from("."),
            model_hint: None,
            force_backend: None,
            budget_remaining_usd: 5.0,
            attempt: 0,
            dependency_outputs: Vec::new(),
        }
    }
}
