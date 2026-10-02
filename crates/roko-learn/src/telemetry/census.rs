//! A run's wiring census, `census.json` (`roko.census/1`, S01 §5.8): which
//! learning components the dispatcher that ran it had.
//!
//! A plan run writes it into the run directory when the run's manifest
//! opens, and rewrites it on resume, since the build may differ. Two runs of
//! one plan, one with the routing sink and one without, differ here. The
//! per-loop half of S01 §5.8 (`loops[]`) is S03's loop census; it is not
//! built here.

use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::LearnError;

/// `schema_version` of `census.json` (S01 §5.8).
pub const CENSUS_SCHEMA: &str = "roko.census/1";
/// `.roko/runs/<run_id>/census.json`, rewritten atomically each time the
/// run starts or resumes.
pub const CENSUS_FILE: &str = "census.json";

/// `roko.census/1`: the learning components of a run's dispatcher.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CensusReport {
    /// Always [`CENSUS_SCHEMA`].
    pub schema_version: String,
    /// The run.
    pub run_id: String,
    /// Git commit of the harness build that wrote the census.
    pub harness_sha: String,
    /// Whether that build's tree was dirty.
    #[serde(default)]
    pub dirty: bool,
    /// Every component the census names, wired or not.
    #[serde(default)]
    pub components: Vec<CensusComponent>,
}

/// One learning component of a census (S01 §5.8 `components[]`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CensusComponent {
    /// Stable id, e.g. `sink.routing`.
    pub id: String,
    /// What it does: `sink`, `store` or `reader`.
    pub kind: String,
    /// Whether the dispatcher had it.
    pub wired: bool,
    /// What provides it, or why it is missing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

impl CensusReport {
    /// The census of run `run_id`, written by the harness build `harness_sha`.
    pub fn new(
        run_id: impl Into<String>,
        harness_sha: impl Into<String>,
        dirty: bool,
        components: Vec<CensusComponent>,
    ) -> Self {
        Self {
            schema_version: CENSUS_SCHEMA.to_string(),
            run_id: run_id.into(),
            harness_sha: harness_sha.into(),
            dirty,
            components,
        }
    }

    /// Path of the census inside `run_dir` (`RokoLayout::run_dir(run_id)`).
    #[must_use]
    pub fn path_in(run_dir: &Path) -> PathBuf {
        run_dir.join(CENSUS_FILE)
    }

    /// The census in `run_dir`, or `None` when the run has none: runs
    /// before the census had none.
    ///
    /// # Errors
    ///
    /// Returns an error when the file exists but cannot be read or parsed.
    pub fn load(run_dir: &Path) -> Result<Option<Self>, LearnError> {
        let path = Self::path_in(run_dir);
        let bytes = match std::fs::read(&path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(source) => {
                return Err(LearnError::Io {
                    path: path.display().to_string(),
                    source,
                });
            }
        };
        serde_json::from_slice(&bytes)
            .map(Some)
            .map_err(|error| LearnError::Corrupt {
                path: path.display().to_string(),
                reason: error.to_string(),
            })
    }

    /// Write the census into `run_dir` atomically, creating the directory.
    ///
    /// # Errors
    ///
    /// Returns an error when the file cannot be written.
    pub fn store(&self, run_dir: &Path) -> Result<(), LearnError> {
        let path = Self::path_in(run_dir);
        roko_fs::atomic::atomic_write_json(&path, self).map_err(|source| LearnError::Io {
            path: path.display().to_string(),
            source,
        })
    }

    /// Ids of the components the dispatcher lacked.
    #[must_use]
    pub fn unwired(&self) -> Vec<&str> {
        self.components
            .iter()
            .filter(|component| !component.wired)
            .map(|component| component.id.as_str())
            .collect()
    }

    /// The component `id`, if the census names it.
    #[must_use]
    pub fn component(&self, id: &str) -> Option<&CensusComponent> {
        self.components.iter().find(|component| component.id == id)
    }
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;

    fn component(id: &str, kind: &str, wired: bool) -> CensusComponent {
        CensusComponent {
            id: id.to_string(),
            kind: kind.to_string(),
            wired,
            detail: None,
        }
    }

    /// The S01 §5.8 example's components, without its `loops[]` (S03's).
    const CENSUS_EXAMPLE: &str = r#"{
        "schema_version": "roko.census/1", "harness_sha": "725f21e05", "dirty": true,
        "run_id": "gr-7f3c2a91",
        "components": [
            {"id": "sink.routing", "kind": "sink", "wired": true},
            {"id": "sink.knowledge_ingestion", "kind": "sink", "wired": false,
             "expected_missing": true}
        ]
    }"#;

    #[test]
    fn census_round_trips_and_lists_unwired_components() {
        let example: CensusReport = serde_json::from_str(CENSUS_EXAMPLE).expect("example");
        assert_eq!(example.schema_version, CENSUS_SCHEMA);
        assert_eq!(example.unwired(), ["sink.knowledge_ingestion"]);
        assert!(example.component("sink.routing").is_some_and(|c| c.wired));

        let dir = TempDir::new().expect("tempdir");
        assert_eq!(CensusReport::load(dir.path()).expect("load nothing"), None);
        let components = vec![
            component("store.attempt_log", "store", true),
            component("sink.section_effect", "sink", false),
        ];
        let census = CensusReport::new("gr-1", "abc123", false, components);
        census.store(dir.path()).expect("store the census");
        let loaded = CensusReport::load(dir.path()).expect("load the census");
        assert_eq!(loaded.as_ref(), Some(&census));
        assert_eq!(census.unwired(), ["sink.section_effect"]);

        std::fs::write(CensusReport::path_in(dir.path()), "not json").expect("corrupt it");
        assert!(CensusReport::load(dir.path()).is_err());
    }
}
