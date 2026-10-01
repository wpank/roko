//! Named workflow graph templates (#257).
//!
//! Each canonical template maps to a pipeline band from the existing
//! `WorkflowConfig` infrastructure: whether it reviews, whether it plans a
//! strategy first, and how many iterations and autofix attempts it gets.
//! The [`WorkflowGraphController`](super::controller::WorkflowGraphController)
//! reads these descriptors. The subgraph builders that once turned them into
//! graphs had no production caller and were deleted (gap-3505fb); plan runs
//! build their topology with `roko_graph::ProductionPlanTopology`.
//!
//! # Canonical Templates
//!
//! | Name | Review | Strategy | Max iter | Max autofix | Aliases |
//! |------|--------|----------|----------|-------------|---------|
//! | mechanical | no | no | 1 | 1 | express, standard |
//! | focused | yes | no | 2 | 2 | |
//! | integrative | yes | no | 3 | 2 | |
//! | architectural | yes | yes | 3 | 2 | full |

use std::fmt;

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Template schema version. Included in idempotency keys.
pub const TEMPLATE_VERSION: u32 = 1;

/// All canonical template names, in order.
pub const CANONICAL_NAMES: &[&str] = &["mechanical", "focused", "integrative", "architectural"];

// ---------------------------------------------------------------------------
// Template descriptor
// ---------------------------------------------------------------------------

/// Descriptor for a named workflow template.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowTemplateDescriptor {
    /// Canonical template name.
    pub name: String,
    /// Template schema version.
    pub version: u32,
    /// Whether a review phase is included after successful gating.
    pub has_review: bool,
    /// Whether a strategy phase precedes implementation.
    pub has_strategy: bool,
    /// Maximum implement-gate(-review) iterations.
    pub max_iterations: u32,
    /// Maximum autofix attempts per gate failure within one iteration.
    pub max_autofix_attempts: u32,
    /// Whether git commit is enabled (can be disabled via no-commit mode).
    pub commit_enabled: bool,
}

impl WorkflowTemplateDescriptor {
    /// Create the `mechanical` template: implement -> gate -> commit.
    #[must_use]
    pub fn mechanical() -> Self {
        Self {
            name: "mechanical".to_string(),
            version: TEMPLATE_VERSION,
            has_review: false,
            has_strategy: false,
            max_iterations: 1,
            max_autofix_attempts: 1,
            commit_enabled: true,
        }
    }

    /// Create the `focused` template: implement -> gate -> review -> commit.
    #[must_use]
    pub fn focused() -> Self {
        Self {
            name: "focused".to_string(),
            version: TEMPLATE_VERSION,
            has_review: true,
            has_strategy: false,
            max_iterations: 2,
            max_autofix_attempts: 2,
            commit_enabled: true,
        }
    }

    /// Create the `integrative` template: implement -> gate -> review -> commit (more iterations).
    #[must_use]
    pub fn integrative() -> Self {
        Self {
            name: "integrative".to_string(),
            version: TEMPLATE_VERSION,
            has_review: true,
            has_strategy: false,
            max_iterations: 3,
            max_autofix_attempts: 2,
            commit_enabled: true,
        }
    }

    /// Create the `architectural` template: strategy -> implement -> gate -> review -> commit.
    #[must_use]
    pub fn architectural() -> Self {
        Self {
            name: "architectural".to_string(),
            version: TEMPLATE_VERSION,
            has_review: true,
            has_strategy: true,
            max_iterations: 3,
            max_autofix_attempts: 2,
            commit_enabled: true,
        }
    }

    /// Builder: disable the commit phase (no-commit mode).
    #[must_use]
    pub fn with_commit_disabled(mut self) -> Self {
        self.commit_enabled = false;
        self
    }
}

impl fmt::Display for WorkflowTemplateDescriptor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}@{}", self.name, self.version)
    }
}

// ---------------------------------------------------------------------------
// Alias resolution
// ---------------------------------------------------------------------------

/// Frozen alias table: maps legacy/convenience names to canonical names.
///
/// Unknown names are not resolved and produce an error listing all valid names.
const ALIAS_TABLE: &[(&str, &str)] = &[
    ("express", "mechanical"),
    ("standard", "mechanical"),
    ("full", "architectural"),
    // Canonical names map to themselves.
    ("mechanical", "mechanical"),
    ("focused", "focused"),
    ("integrative", "integrative"),
    ("architectural", "architectural"),
];

/// Resolve a template name (canonical or alias) to its canonical name.
///
/// Returns `None` for unknown names.
#[must_use]
pub fn resolve_template_name(name: &str) -> Option<&'static str> {
    let lower = name.to_lowercase();
    ALIAS_TABLE
        .iter()
        .find(|(alias, _)| *alias == lower.as_str())
        .map(|(_, canonical)| *canonical)
}

/// Resolve a template name to its descriptor.
///
/// # Errors
///
/// Returns an error listing all valid canonical names and aliases if the name
/// is not recognized.
pub fn resolve_template(name: &str) -> Result<WorkflowTemplateDescriptor, TemplateResolutionError> {
    let canonical = resolve_template_name(name).ok_or_else(|| TemplateResolutionError {
        requested: name.to_string(),
        valid_canonical: CANONICAL_NAMES.iter().map(|s| (*s).to_string()).collect(),
        valid_aliases: ALIAS_TABLE
            .iter()
            .filter(|(alias, canonical)| alias != canonical)
            .map(|(alias, canonical)| format!("{alias} -> {canonical}"))
            .collect(),
    })?;

    Ok(match canonical {
        "mechanical" => WorkflowTemplateDescriptor::mechanical(),
        "focused" => WorkflowTemplateDescriptor::focused(),
        "integrative" => WorkflowTemplateDescriptor::integrative(),
        "architectural" => WorkflowTemplateDescriptor::architectural(),
        _ => unreachable!("canonical name validated by resolve_template_name"),
    })
}

/// Error returned when a template name cannot be resolved.
#[derive(Debug, Clone, thiserror::Error)]
#[error(
    "unknown workflow template '{requested}'; valid canonical names: {valid_canonical:?}; \
     aliases: {valid_aliases:?}"
)]
pub struct TemplateResolutionError {
    /// The name that was requested.
    pub requested: String,
    /// Valid canonical template names.
    pub valid_canonical: Vec<String>,
    /// Valid aliases with their targets.
    pub valid_aliases: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Alias resolution ───────────────────────────────────────────────

    #[test]
    fn resolve_canonical_names() {
        for name in CANONICAL_NAMES {
            assert_eq!(resolve_template_name(name), Some(*name));
        }
    }

    #[test]
    fn resolve_aliases() {
        assert_eq!(resolve_template_name("express"), Some("mechanical"));
        assert_eq!(resolve_template_name("standard"), Some("mechanical"));
        assert_eq!(resolve_template_name("full"), Some("architectural"));
    }

    #[test]
    fn resolve_case_insensitive() {
        assert_eq!(resolve_template_name("MECHANICAL"), Some("mechanical"));
        assert_eq!(resolve_template_name("Express"), Some("mechanical"));
        assert_eq!(resolve_template_name("FULL"), Some("architectural"));
    }

    #[test]
    fn resolve_unknown_returns_none() {
        assert_eq!(resolve_template_name("bogus"), None);
    }

    #[test]
    fn resolve_template_unknown_returns_error() {
        let err = resolve_template("bogus").unwrap_err();
        assert_eq!(err.requested, "bogus");
        assert_eq!(err.valid_canonical.len(), 4);
        assert!(!err.valid_aliases.is_empty());
    }

    #[test]
    fn resolve_template_all_canonical() {
        for name in CANONICAL_NAMES {
            let desc = resolve_template(name).unwrap();
            assert_eq!(desc.name, *name);
            assert_eq!(desc.version, TEMPLATE_VERSION);
        }
    }

    #[test]
    fn resolve_template_alias_express() {
        let desc = resolve_template("express").unwrap();
        assert_eq!(desc.name, "mechanical");
    }

    #[test]
    fn resolve_template_alias_full() {
        let desc = resolve_template("full").unwrap();
        assert_eq!(desc.name, "architectural");
    }

    // ── Template properties ────────────────────────────────────────────

    #[test]
    fn mechanical_properties() {
        let d = WorkflowTemplateDescriptor::mechanical();
        assert!(!d.has_review);
        assert!(!d.has_strategy);
        assert_eq!(d.max_iterations, 1);
        assert_eq!(d.max_autofix_attempts, 1);
        assert!(d.commit_enabled);
    }

    #[test]
    fn focused_properties() {
        let d = WorkflowTemplateDescriptor::focused();
        assert!(d.has_review);
        assert!(!d.has_strategy);
        assert_eq!(d.max_iterations, 2);
    }

    #[test]
    fn integrative_properties() {
        let d = WorkflowTemplateDescriptor::integrative();
        assert!(d.has_review);
        assert!(!d.has_strategy);
        assert_eq!(d.max_iterations, 3);
    }

    #[test]
    fn architectural_properties() {
        let d = WorkflowTemplateDescriptor::architectural();
        assert!(d.has_review);
        assert!(d.has_strategy);
        assert_eq!(d.max_iterations, 3);
    }

    #[test]
    fn no_commit_mode() {
        let d = WorkflowTemplateDescriptor::mechanical().with_commit_disabled();
        assert!(!d.commit_enabled);
    }

    #[test]
    fn display_format() {
        let d = WorkflowTemplateDescriptor::mechanical();
        assert_eq!(d.to_string(), "mechanical@1");
    }

    // ── Serde ──────────────────────────────────────────────────────────

    #[test]
    fn descriptor_serde_roundtrip() {
        for name in CANONICAL_NAMES {
            let d = resolve_template(name).unwrap();
            let json = serde_json::to_string(&d).unwrap();
            let back: WorkflowTemplateDescriptor = serde_json::from_str(&json).unwrap();
            assert_eq!(back, d);
        }
    }
}
