//! Shared GitOps types used by both `roko-runtime` and `roko-agent`.
//!
//! These types describe the configuration and runtime state for a GitOps
//! reconciliation loop: pulling desired agent configuration from a Git
//! repository, detecting drift, and optionally self-healing.

use serde::{Deserialize, Serialize};

/// `GitOps` configuration source for lifecycle-managed agent configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GitOpsConfig {
    /// Git repository URL.
    pub repo_url: String,
    /// Branch, tag, or commit SHA.
    pub target_revision: String,
    /// Relative config path within the repository.
    pub path: String,
    /// Poll interval in seconds.
    pub poll_interval_secs: u64,
    /// Automatically apply detected changes.
    pub auto_sync: bool,
    /// Revert manual drift back to the Git state.
    pub self_heal: bool,
    /// Remove config keys absent from desired state.
    pub prune: bool,
    /// Number of historical revisions retained for rollback.
    pub revision_history_limit: usize,
    /// Retry policy for failed reconciliation.
    pub retry: GitOpsRetryPolicy,
}

impl Default for GitOpsConfig {
    fn default() -> Self {
        Self {
            repo_url: String::new(),
            target_revision: "main".into(),
            path: ".".into(),
            poll_interval_secs: 60,
            auto_sync: true,
            self_heal: true,
            prune: false,
            revision_history_limit: 10,
            retry: GitOpsRetryPolicy::default(),
        }
    }
}

/// Retry policy for `GitOps` synchronization failures.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GitOpsRetryPolicy {
    /// Maximum retry attempts. `-1` means unlimited.
    pub limit: i32,
    /// Initial backoff in seconds.
    pub initial_backoff_secs: u64,
    /// Backoff multiplier.
    pub factor: f64,
    /// Maximum backoff in seconds.
    pub max_backoff_secs: u64,
}

impl Default for GitOpsRetryPolicy {
    fn default() -> Self {
        Self {
            limit: 5,
            initial_backoff_secs: 5,
            factor: 2.0,
            max_backoff_secs: 180,
        }
    }
}

/// Result of a `GitOps` drift-detection pass.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum ConfigDrift {
    /// Actual state matches the desired state.
    InSync {
        /// Git revision used for comparison.
        revision: String,
    },
    /// Actual state diverges from the desired state.
    Drifted {
        /// Git revision used for comparison.
        revision: String,
        /// Divergent configuration keys.
        diverged_keys: Vec<String>,
        /// Last known good revision.
        last_known_good: String,
    },
    /// Git source was unreachable.
    SourceUnreachable {
        /// Human-readable connection or authentication error.
        error: String,
    },
}
