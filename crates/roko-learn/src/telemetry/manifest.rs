//! Reading and writing a run's `manifest.json` (S01 §5.1), and counting the
//! attempts its `attempts.jsonl` records.
//!
//! The manifest is rewritten atomically whenever a process starts or resumes
//! the run ([`RunProvenanceManifest::begin_invocation`]) and when the run
//! closes. Its closing counts come from the run's files
//! ([`AttemptTally::read`]), so they cover every invocation, not just the
//! last one.

use std::collections::BTreeSet;
use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::path::{Path, PathBuf};

use serde::Deserialize;

use super::records::{
    ATTEMPT_OPEN_SCHEMA, MANIFEST_FILE, RunFile, RunInvocation, RunProvenanceManifest,
    VERDICT_SCHEMA,
};
use crate::error::LearnError;

impl RunProvenanceManifest {
    /// Path of the manifest inside `run_dir` (`RokoLayout::run_dir(run_id)`).
    #[must_use]
    pub fn path_in(run_dir: &Path) -> PathBuf {
        run_dir.join(MANIFEST_FILE)
    }

    /// The manifest in `run_dir`, or `None` when the run has none yet.
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

    /// Write the manifest into `run_dir` atomically, creating the directory.
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

    /// Record the invocation starting now and return its 1-based ordinal.
    ///
    /// The ordinal follows the recorded ones, and the invocation counts as a
    /// resume when the run has earlier invocations. The invocation keeps its
    /// own harness build and config. The first invocation's become the
    /// run's; a resume under another build or config marks the run
    /// [`mixed_provenance`](Self::mixed_provenance). A closed run is open
    /// again, so its `closed` section is cleared until the run closes anew.
    pub fn begin_invocation(&mut self, mut invocation: RunInvocation) -> u32 {
        invocation.inv = self.next_inv();
        invocation.resumed = !self.invocations.is_empty();
        if invocation.resumed {
            let other_build = invocation
                .harness
                .as_ref()
                .is_some_and(|harness| *harness != self.harness);
            let other_config = invocation
                .config
                .as_ref()
                .is_some_and(|config| *config != self.config);
            self.mixed_provenance |= other_build || other_config;
        } else {
            if let Some(harness) = &invocation.harness {
                self.harness = harness.clone();
            }
            if let Some(config) = &invocation.config {
                self.config = config.clone();
            }
        }
        let inv = invocation.inv;
        self.invocations.push(invocation);
        self.closed = None;
        inv
    }
}

/// The attempts a run's `attempts.jsonl` records (S01 SC1:
/// `opened == settled + abandoned`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AttemptTally {
    /// Distinct attempts with an attempt-open line.
    pub opened: u64,
    /// Distinct attempts with a verdict.
    pub settled: u64,
    /// Opened attempts with no verdict: the process died, or the attempt
    /// ended without settling.
    pub abandoned: u64,
}

impl AttemptTally {
    /// Count `(schema_version, attempt_key)` pairs of attempt lines; lines of
    /// other schemas are ignored.
    pub fn from_lines<'a>(lines: impl IntoIterator<Item = (&'a str, &'a str)>) -> Self {
        let mut opened = BTreeSet::new();
        let mut settled = BTreeSet::new();
        for (schema, attempt_key) in lines {
            match schema {
                ATTEMPT_OPEN_SCHEMA => {
                    opened.insert(attempt_key);
                }
                VERDICT_SCHEMA => {
                    settled.insert(attempt_key);
                }
                _ => {}
            }
        }
        let abandoned = opened.difference(&settled).count();
        Self {
            opened: opened.len() as u64,
            settled: settled.len() as u64,
            abandoned: abandoned as u64,
        }
    }

    /// Count the attempts in `run_dir`'s `attempts.jsonl`. A missing file
    /// holds none, and lines that do not parse are skipped.
    ///
    /// # Errors
    ///
    /// Returns an error when the file exists but cannot be read.
    pub fn read(run_dir: &Path) -> Result<Self, LearnError> {
        #[derive(Deserialize)]
        struct Line {
            schema_version: String,
            attempt_key: String,
        }

        let path = RunFile::Attempts.path_in(run_dir);
        let io_error = |source: io::Error| LearnError::Io {
            path: path.display().to_string(),
            source,
        };
        let file = match File::open(&path) {
            Ok(file) => file,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Self::default()),
            Err(error) => return Err(io_error(error)),
        };
        let mut lines = Vec::new();
        for line in BufReader::new(file).lines() {
            let line = line.map_err(io_error)?;
            if let Ok(line) = serde_json::from_str::<Line>(line.trim()) {
                lines.push(line);
            }
        }
        Ok(Self::from_lines(lines.iter().map(|line| {
            (line.schema_version.as_str(), line.attempt_key.as_str())
        })))
    }
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;
    use crate::telemetry::records::{
        AttemptIdentity, AttemptKey, AttemptOpenRecord, AttemptOutcome, AttemptVerdictRecord,
        ConfigHashProvenance, HarnessProvenance, RUN_MANIFEST_SCHEMA, RunClosed,
    };
    use crate::telemetry::writer::{TelemetryWriter, TelemetryWriterConfig};

    const RUN: &str = "gr-7f3c2a91";

    fn identity(task: &str, attempt: u32) -> AttemptIdentity {
        AttemptIdentity::new(&AttemptKey::new(RUN, "loop-census", task, attempt))
    }

    #[test]
    fn manifest_invocations_number_resumes_and_reopen_a_closed_run() {
        let dir = TempDir::new().expect("tempdir");
        assert_eq!(RunProvenanceManifest::load(dir.path()).expect("load"), None);

        let mut manifest = RunProvenanceManifest::new(RUN, "plan_run");
        let first = manifest.begin_invocation(RunInvocation {
            pid: 41,
            ..RunInvocation::default()
        });
        manifest.closed = Some(RunClosed {
            status: "interrupted".to_string(),
            ..RunClosed::default()
        });
        manifest.store(dir.path()).expect("store");

        let mut resumed = RunProvenanceManifest::load(dir.path())
            .expect("load")
            .expect("the stored manifest");
        assert_eq!(resumed, manifest);
        let second = resumed.begin_invocation(RunInvocation::default());
        assert_eq!((first, second), (1, 2));
        let resumes: Vec<bool> = resumed.invocations.iter().map(|i| i.resumed).collect();
        assert_eq!(resumes, [false, true]);
        assert_eq!(resumed.closed, None, "a resumed run is open again");
        assert_eq!(resumed.schema_version, RUN_MANIFEST_SCHEMA);
    }

    /// An invocation that ran `sha` with config `hash`.
    fn build(sha: &str, hash: &str) -> RunInvocation {
        RunInvocation {
            harness: Some(HarnessProvenance {
                sha: sha.to_string(),
                ..HarnessProvenance::default()
            }),
            config: Some(ConfigHashProvenance {
                hash: hash.to_string(),
                ..ConfigHashProvenance::default()
            }),
            ..RunInvocation::default()
        }
    }

    #[test]
    fn a_resume_under_another_build_records_its_own_harness_and_config() {
        let dir = TempDir::new().expect("tempdir");
        let mut manifest = RunProvenanceManifest::new(RUN, "plan_run");
        manifest.begin_invocation(build("aaa1111", "b3:one"));
        assert_eq!(
            manifest.harness.sha, "aaa1111",
            "the first build is the run's"
        );
        assert_eq!(manifest.config.hash, "b3:one");
        manifest.begin_invocation(build("aaa1111", "b3:one"));
        assert!(
            !manifest.mixed_provenance,
            "the same build and config resumed"
        );
        manifest.store(dir.path()).expect("store");

        let mut resumed = RunProvenanceManifest::load(dir.path())
            .expect("load")
            .expect("the stored manifest");
        assert_eq!(resumed.begin_invocation(build("bbb2222", "b3:one")), 3);
        assert!(resumed.mixed_provenance, "another build resumed the run");
        assert_eq!(
            resumed.harness.sha, "aaa1111",
            "the run keeps its first build"
        );
        resumed.store(dir.path()).expect("store");

        // The file says which build and config each invocation ran under.
        let stored = std::fs::read_to_string(RunProvenanceManifest::path_in(dir.path()))
            .expect("read manifest.json");
        let json: serde_json::Value = serde_json::from_str(&stored).expect("manifest JSON");
        assert_eq!(json["mixed_provenance"], true);
        let invocations: Vec<(u64, &str, &str)> = json["invocations"]
            .as_array()
            .expect("invocations")
            .iter()
            .map(|invocation| {
                (
                    invocation["inv"].as_u64().unwrap_or(0),
                    invocation["harness"]["sha"].as_str().unwrap_or_default(),
                    invocation["config"]["hash"].as_str().unwrap_or_default(),
                )
            })
            .collect();
        assert_eq!(
            invocations,
            [
                (1, "aaa1111", "b3:one"),
                (2, "aaa1111", "b3:one"),
                (3, "bbb2222", "b3:one"),
            ]
        );

        // A config change alone mixes a run too, including one whose earlier
        // invocations predate per-invocation provenance.
        let mut legacy = RunProvenanceManifest::new(RUN, "plan_run");
        legacy.harness.sha = "aaa1111".to_string();
        legacy.config.hash = "b3:one".to_string();
        legacy.invocations.push(RunInvocation {
            inv: 1,
            ..RunInvocation::default()
        });
        legacy.begin_invocation(build("aaa1111", "b3:one"));
        assert!(!legacy.mixed_provenance);
        legacy.begin_invocation(build("aaa1111", "b3:two"));
        assert!(legacy.mixed_provenance, "another config resumed the run");
    }

    #[test]
    fn attempt_tally_counts_abandoned_attempts_from_the_attempts_log() {
        let dir = TempDir::new().expect("tempdir");
        assert_eq!(
            AttemptTally::read(dir.path()).expect("tally"),
            AttemptTally::default()
        );
        let writer = TelemetryWriter::spawn(dir.path(), TelemetryWriterConfig::default())
            .expect("spawn writer");
        for (task, attempt) in [("T1", 1), ("T2", 1), ("T2", 2)] {
            assert!(writer.submit(AttemptOpenRecord::new(identity(task, attempt), 0)));
        }
        for (task, attempt) in [("T1", 1), ("T2", 1)] {
            let verdict =
                AttemptVerdictRecord::settle(identity(task, attempt), AttemptOutcome::Passed, true);
            assert!(writer.submit(verdict));
        }
        assert_eq!(writer.close().written, 5);

        let tally = AttemptTally::read(dir.path()).expect("tally");
        let expected = AttemptTally {
            opened: 3,
            settled: 2,
            abandoned: 1,
        };
        assert_eq!(tally, expected);
    }
}
