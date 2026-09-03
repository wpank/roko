//! Runtime profile enum and fixed profile bundle matrix (#243).
//!
//! The profile matrix is encoded as data in [`profile_bundle_manifest`] so
//! silent drift is caught by snapshot tests. Profile `match` logic is not
//! scattered across bundle constructors.

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// RuntimeProfile
// ---------------------------------------------------------------------------

/// Execution surface profile that determines which service bundles are
/// required, optional, or forbidden.
///
/// Each profile has documented minimum safety, budget, feedback, and
/// shutdown guarantees.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeProfile {
    /// Runner-v2 plan execution: all six bundles required.
    FullPlan,
    /// Graph engine plan execution: same guarantees as `FullPlan` plus
    /// the Graph publisher port. Runner scheduler/state types are forbidden.
    GraphPlan,
    /// Single-prompt workflow execution via `roko run`.
    Workflow,
    /// Lightweight direct execution (`roko do`, `roko develop`).
    DirectLight,
    /// Per-agent HTTP sidecar (`roko agent serve`).
    AgentServer,
    /// Interactive chat REPL (`roko chat`).
    ChatLight,
    /// Authored graph execution with capability intersection.
    AuthoredGraph,
}

impl std::fmt::Display for RuntimeProfile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::FullPlan => write!(f, "FullPlan"),
            Self::GraphPlan => write!(f, "GraphPlan"),
            Self::Workflow => write!(f, "Workflow"),
            Self::DirectLight => write!(f, "DirectLight"),
            Self::AgentServer => write!(f, "AgentServer"),
            Self::ChatLight => write!(f, "ChatLight"),
            Self::AuthoredGraph => write!(f, "AuthoredGraph"),
        }
    }
}

// ---------------------------------------------------------------------------
// ProfileBundleManifest
// ---------------------------------------------------------------------------

/// Describes which bundles are mandatory for a given profile.
///
/// Used by parity tests to verify both engine paths share the same
/// required service set.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProfileBundleManifest {
    pub profile: RuntimeProfile,
    pub dispatch_required: bool,
    pub prompt_required: bool,
    pub feedback_required: bool,
    pub extensions_optional: bool,
    pub observation_required: bool,
    pub guards_required: bool,
}

/// Return the bundle manifest for a given profile.
///
/// This encodes the fixed profile matrix from #243 as data rather than
/// scattering profile `match` logic across bundle constructors.
#[must_use]
pub fn profile_bundle_manifest(profile: RuntimeProfile) -> ProfileBundleManifest {
    match profile {
        RuntimeProfile::FullPlan | RuntimeProfile::GraphPlan => ProfileBundleManifest {
            profile,
            dispatch_required: true,
            prompt_required: true,
            feedback_required: true,
            extensions_optional: true,
            observation_required: true,
            guards_required: true,
        },
        RuntimeProfile::Workflow
        | RuntimeProfile::DirectLight
        | RuntimeProfile::AgentServer
        | RuntimeProfile::ChatLight
        | RuntimeProfile::AuthoredGraph => ProfileBundleManifest {
            profile,
            dispatch_required: true,
            prompt_required: true,
            feedback_required: false,
            extensions_optional: true,
            observation_required: true,
            guards_required: true,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_display() {
        assert_eq!(RuntimeProfile::FullPlan.to_string(), "FullPlan");
        assert_eq!(RuntimeProfile::GraphPlan.to_string(), "GraphPlan");
        assert_eq!(RuntimeProfile::Workflow.to_string(), "Workflow");
        assert_eq!(RuntimeProfile::DirectLight.to_string(), "DirectLight");
        assert_eq!(RuntimeProfile::AgentServer.to_string(), "AgentServer");
        assert_eq!(RuntimeProfile::ChatLight.to_string(), "ChatLight");
        assert_eq!(RuntimeProfile::AuthoredGraph.to_string(), "AuthoredGraph");
    }

    #[test]
    fn fullplan_and_graphplan_share_mandatory_bundles() {
        let full = profile_bundle_manifest(RuntimeProfile::FullPlan);
        let graph = profile_bundle_manifest(RuntimeProfile::GraphPlan);
        assert_eq!(full.dispatch_required, graph.dispatch_required);
        assert_eq!(full.prompt_required, graph.prompt_required);
        assert_eq!(full.feedback_required, graph.feedback_required);
        assert_eq!(full.observation_required, graph.observation_required);
        assert_eq!(full.guards_required, graph.guards_required);
    }

    #[test]
    fn profile_bundle_manifest_snapshot() {
        let manifests: Vec<_> = [
            RuntimeProfile::FullPlan,
            RuntimeProfile::GraphPlan,
            RuntimeProfile::Workflow,
            RuntimeProfile::DirectLight,
            RuntimeProfile::AgentServer,
            RuntimeProfile::ChatLight,
            RuntimeProfile::AuthoredGraph,
        ]
        .iter()
        .map(|p| profile_bundle_manifest(*p))
        .collect();

        // FullPlan and GraphPlan require all bundles
        for m in &manifests[..2] {
            assert!(m.dispatch_required);
            assert!(m.prompt_required);
            assert!(m.feedback_required);
            assert!(m.observation_required);
            assert!(m.guards_required);
        }
        // Lighter profiles have optional feedback
        for m in &manifests[2..] {
            assert!(m.dispatch_required);
            assert!(m.prompt_required);
            assert!(
                !m.feedback_required,
                "feedback is optional for {}",
                m.profile
            );
            assert!(m.observation_required);
            assert!(m.guards_required);
        }
    }

    #[test]
    fn profile_serde_roundtrip() {
        let original = RuntimeProfile::FullPlan;
        let json = serde_json::to_string(&original).unwrap();
        assert_eq!(json, "\"full_plan\"");
        let deserialized: RuntimeProfile = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, original);
    }
}
