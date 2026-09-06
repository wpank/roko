//! P4-27: HDC fingerprint migration framework.
//!
//! Provides scaffolding for migrating HDC fingerprints when the fingerprint
//! algorithm changes (e.g., different hash function, different bit width,
//! different encoding). Includes version tagging and batch migration support.

use crate::hdc::HdcVector;
use serde::{Deserialize, Serialize};

/// Current HDC fingerprint version.
pub const CURRENT_VERSION: u32 = 1;

/// A versioned HDC fingerprint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionedFingerprint {
    /// Schema version for this fingerprint.
    pub version: u32,
    /// The HDC vector.
    pub vector: HdcVector,
    /// Optional base64-encoded representation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub encoded: Option<String>,
}

impl VersionedFingerprint {
    /// Create a new fingerprint at the current version.
    #[must_use]
    pub fn new(vector: HdcVector) -> Self {
        Self {
            version: CURRENT_VERSION,
            vector,
            encoded: None,
        }
    }

    /// Whether this fingerprint needs migration to the current version.
    #[must_use]
    pub fn needs_migration(&self) -> bool {
        self.version < CURRENT_VERSION
    }
}

/// Migration result for a single fingerprint.
#[derive(Debug, Clone)]
pub enum MigrationResult {
    /// Already at the current version; no migration needed.
    AlreadyCurrent,
    /// Successfully migrated to the current version.
    Migrated {
        /// The migrated fingerprint.
        fingerprint: VersionedFingerprint,
        /// Which versions were traversed.
        from_version: u32,
    },
    /// Migration failed.
    Failed {
        /// The version that could not be migrated.
        stuck_at_version: u32,
        /// Error description.
        reason: String,
    },
}

/// A migration step from one version to the next.
pub trait MigrationStep: Send + Sync {
    /// Source version this step migrates from.
    fn from_version(&self) -> u32;
    /// Target version this step migrates to.
    fn to_version(&self) -> u32;
    /// Migrate a vector from `from_version` to `to_version`.
    fn migrate(&self, vector: &HdcVector) -> Result<HdcVector, String>;
}

/// Registry of migration steps for sequential version upgrades.
pub struct MigrationRegistry {
    steps: Vec<Box<dyn MigrationStep>>,
}

impl Default for MigrationRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl MigrationRegistry {
    /// Create an empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self { steps: Vec::new() }
    }

    /// Register a migration step.
    pub fn register(&mut self, step: Box<dyn MigrationStep>) {
        self.steps.push(step);
        self.steps.sort_by_key(|s| s.from_version());
    }

    /// Migrate a fingerprint to the current version.
    pub fn migrate(&self, fingerprint: &VersionedFingerprint) -> MigrationResult {
        if fingerprint.version >= CURRENT_VERSION {
            return MigrationResult::AlreadyCurrent;
        }

        let from_version = fingerprint.version;
        let mut current_vector = fingerprint.vector;
        let mut current_version = fingerprint.version;

        while current_version < CURRENT_VERSION {
            let step = self.steps.iter().find(|s| s.from_version() == current_version);
            match step {
                Some(step) => match step.migrate(&current_vector) {
                    Ok(migrated) => {
                        current_vector = migrated;
                        current_version = step.to_version();
                    }
                    Err(reason) => {
                        return MigrationResult::Failed {
                            stuck_at_version: current_version,
                            reason,
                        };
                    }
                },
                None => {
                    return MigrationResult::Failed {
                        stuck_at_version: current_version,
                        reason: format!(
                            "no migration step found from version {current_version}"
                        ),
                    };
                }
            }
        }

        MigrationResult::Migrated {
            fingerprint: VersionedFingerprint {
                version: CURRENT_VERSION,
                vector: current_vector,
                encoded: None,
            },
            from_version,
        }
    }

    /// Batch migrate a set of fingerprints.
    pub fn batch_migrate(
        &self,
        fingerprints: &[VersionedFingerprint],
    ) -> Vec<(usize, MigrationResult)> {
        fingerprints
            .iter()
            .enumerate()
            .map(|(i, fp)| (i, self.migrate(fp)))
            .collect()
    }
}

/// Statistics about a batch migration run.
#[derive(Debug, Clone, Default)]
pub struct MigrationStats {
    /// Number of fingerprints already current.
    pub already_current: usize,
    /// Number successfully migrated.
    pub migrated: usize,
    /// Number that failed migration.
    pub failed: usize,
}

impl MigrationStats {
    /// Compute stats from batch results.
    #[must_use]
    pub fn from_results(results: &[(usize, MigrationResult)]) -> Self {
        let mut stats = Self::default();
        for (_, result) in results {
            match result {
                MigrationResult::AlreadyCurrent => stats.already_current += 1,
                MigrationResult::Migrated { .. } => stats.migrated += 1,
                MigrationResult::Failed { .. } => stats.failed += 1,
            }
        }
        stats
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn current_version_needs_no_migration() {
        let fp = VersionedFingerprint::new(HdcVector::random());
        assert!(!fp.needs_migration());

        let registry = MigrationRegistry::new();
        let result = registry.migrate(&fp);
        assert!(matches!(result, MigrationResult::AlreadyCurrent));
    }

    #[test]
    fn old_version_without_step_fails() {
        let fp = VersionedFingerprint {
            version: 0,
            vector: HdcVector::random(),
            encoded: None,
        };
        assert!(fp.needs_migration());

        let registry = MigrationRegistry::new();
        let result = registry.migrate(&fp);
        assert!(matches!(result, MigrationResult::Failed { .. }));
    }

    #[test]
    fn batch_migration_stats() {
        let fps = vec![
            VersionedFingerprint::new(HdcVector::random()),
            VersionedFingerprint::new(HdcVector::random()),
        ];

        let registry = MigrationRegistry::new();
        let results = registry.batch_migrate(&fps);
        let stats = MigrationStats::from_results(&results);
        assert_eq!(stats.already_current, 2);
        assert_eq!(stats.migrated, 0);
        assert_eq!(stats.failed, 0);
    }
}
