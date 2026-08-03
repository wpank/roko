//! Shared retention policy type.
//!
//! [`RetentionPolicy`] is the canonical retention configuration used by all
//! subsystems that manage bounded data stores. Domain-specific engines
//! (filesystem GC, episode compaction, observability rotation) consume this
//! shared type directly or project it through explicit adapters.

use serde::{Deserialize, Serialize};

/// Canonical retention policy shared across all data-management subsystems.
///
/// Individual engines may only use a subset of the fields. Fields that don't
/// apply to a particular engine are ignored by that engine.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetentionPolicy {
    /// Maximum number of primary records to keep (e.g. episodes, signals).
    /// Oldest records are removed first when the limit is exceeded.
    #[serde(default = "default_max_entries")]
    pub max_entries: usize,

    /// Maximum age of records in days. Records older than this are pruned.
    #[serde(default = "default_max_age_days")]
    pub max_age_days: u32,

    /// Maximum age of run/archive directories in days.
    #[serde(default = "default_max_run_age_days")]
    pub max_run_age_days: u32,

    /// Size in MB above which GC/compaction is recommended.
    #[serde(default = "default_size_threshold_mb")]
    pub size_threshold_mb: u64,

    /// Maximum number of cache entries (context-pack, etc.).
    #[serde(default = "default_max_cache_entries")]
    pub max_cache_entries: usize,
}

fn default_max_entries() -> usize {
    200
}
fn default_max_age_days() -> u32 {
    90
}
fn default_max_run_age_days() -> u32 {
    7
}
fn default_size_threshold_mb() -> u64 {
    500
}
fn default_max_cache_entries() -> usize {
    2000
}

impl Default for RetentionPolicy {
    fn default() -> Self {
        Self {
            max_entries: default_max_entries(),
            max_age_days: default_max_age_days(),
            max_run_age_days: default_max_run_age_days(),
            size_threshold_mb: default_size_threshold_mb(),
            max_cache_entries: default_max_cache_entries(),
        }
    }
}
