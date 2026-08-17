//! Signal — the primary type name for the universal datum.
//!
//! `Signal` is now defined directly in the `engram` module as
//! `pub type Signal = Engram`. This module re-exports for convenience.

use std::fmt;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

pub use crate::engram::{
    Engram as Signal, EngramBuilder as SignalBuilder, GraduationError, HdcFingerprint, SignalStatus,
};

/// Publishable marketplace artifact classification.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactKind {
    Snippet,
    PromptPreset,
    Criterion,
    CellComposition,
    CellScript,
    Profile,
    Graph,
    Rack,
    TriggerBinding,
    SpaceTemplate,
    KnowledgeBundle,
}

/// Stable marketplace identity in `@publisher/name@version` form.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ArtifactRef {
    pub publisher: String,
    pub name: String,
    pub version: String,
}

impl ArtifactRef {
    /// Validate that each component can be represented without ambiguity.
    pub fn validate(&self) -> crate::Result<()> {
        let publisher = self.publisher.trim().trim_start_matches('@');
        if publisher.is_empty()
            || self.name.trim().is_empty()
            || self.version.trim().is_empty()
            || publisher.contains(['/', '@'])
            || self.name.contains(['/', '@'])
            || self.version.contains(['/', '@'])
            || publisher.chars().any(char::is_whitespace)
            || self.name.chars().any(char::is_whitespace)
            || self.version.chars().any(char::is_whitespace)
        {
            return Err(crate::RokoError::invalid(
                "artifact ref components must be non-empty and contain no '/', '@', or whitespace",
            ));
        }
        Ok(())
    }
}

impl fmt::Display for ArtifactRef {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "@{}/{}@{}",
            self.publisher.trim().trim_start_matches('@'),
            self.name.trim(),
            self.version.trim()
        )
    }
}

/// Provenance retained when an artifact is forked or composed by reference.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactLineage {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub forked_from: Option<ArtifactRef>,
    #[serde(default)]
    pub composed_from: Vec<ArtifactRef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub forked_at: Option<DateTime<Utc>>,
}

#[cfg(test)]
mod marketplace_tests {
    use super::*;

    #[test]
    fn artifact_ref_display_is_canonical_and_never_double_prefixes() {
        let artifact = ArtifactRef {
            publisher: "@nunchi".to_owned(),
            name: "strict-review".to_owned(),
            version: "2.0.0-rc.1".to_owned(),
        };
        artifact.validate().expect("valid artifact ref");
        assert_eq!(artifact.to_string(), "@nunchi/strict-review@2.0.0-rc.1");
    }

    #[test]
    fn artifact_ref_validation_rejects_ambiguous_segments() {
        for artifact in [
            ArtifactRef {
                publisher: String::new(),
                name: "name".to_owned(),
                version: "1.0.0".to_owned(),
            },
            ArtifactRef {
                publisher: "owner/name".to_owned(),
                name: "name".to_owned(),
                version: "1.0.0".to_owned(),
            },
            ArtifactRef {
                publisher: "owner".to_owned(),
                name: "bad name".to_owned(),
                version: "1.0.0".to_owned(),
            },
        ] {
            assert!(artifact.validate().is_err(), "accepted {artifact:?}");
        }
    }

    #[test]
    fn all_eleven_kinds_and_lineage_round_trip() {
        let kinds = [
            ArtifactKind::Snippet,
            ArtifactKind::PromptPreset,
            ArtifactKind::Criterion,
            ArtifactKind::CellComposition,
            ArtifactKind::CellScript,
            ArtifactKind::Profile,
            ArtifactKind::Graph,
            ArtifactKind::Rack,
            ArtifactKind::TriggerBinding,
            ArtifactKind::SpaceTemplate,
            ArtifactKind::KnowledgeBundle,
        ];
        assert_eq!(kinds.len(), 11);
        for kind in kinds {
            let encoded = serde_json::to_string(&kind).expect("serialize kind");
            assert_eq!(
                serde_json::from_str::<ArtifactKind>(&encoded).expect("restore kind"),
                kind
            );
        }

        let parent = ArtifactRef {
            publisher: "alice".to_owned(),
            name: "review".to_owned(),
            version: "1.0.0".to_owned(),
        };
        let lineage = ArtifactLineage {
            forked_from: Some(parent.clone()),
            composed_from: vec![parent],
            forked_at: Some(Utc::now()),
        };
        let encoded = serde_json::to_string(&lineage).expect("serialize lineage");
        assert_eq!(
            serde_json::from_str::<ArtifactLineage>(&encoded).expect("restore lineage"),
            lineage
        );
    }
}
