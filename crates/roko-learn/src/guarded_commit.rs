//! Guarded commit (P21): the one way a learned store replaces its
//! last-known-good (LKG) version. M1's θ (8113), the cascade router (8136)
//! and the knowledge store (8137) all use it, so every self-modification
//! has one commit log, one rollback path and one provenance format.
//!
//! A store's state is opaque bytes ([`GuardedState`]). [`GuardedStore`]
//! keeps the newest [`LKG_DEPTH`] versions as `v<N>.snap` files, written to
//! a temp file and renamed into place, and one `commits.jsonl` row per
//! decision, in `.roko/learn/commits/<store>/`. [`GuardedStore::propose`]
//! runs the store's [`CommitCheck`]s, held-out and anchors, on a candidate:
//!
//! - every check passes: the candidate is the next version (`committed`);
//! - a check fails in [`GuardMode::Enforce`]: the LKG is restored into the
//!   live state and nothing is written but the row (`rolled_back`);
//! - a check fails in [`GuardMode::Observe`], decision 8103's mode for the
//!   first ten runs: the candidate is committed and the row records the
//!   rollback that would have happened (`observed`).
//!
//! [`GuardedStore::restore_top`] puts the current version back after a
//! restart, and [`GuardedStore::rollback`] restores any version still kept
//! (`restored`). A learner's version carries `ConfigSource::Evolved` with a
//! reason such as `homeostat:ep-0007/ch-0019` (S06 §4.5); there is no new
//! `ConfigSource` variant.

use std::collections::BTreeMap;
use std::io::Write as _;
use std::path::{Path, PathBuf};

use roko_core::config::{ConfigProvenance, ConfigSource};
use serde::{Deserialize, Serialize};

use crate::telemetry::records::b3_digest;

/// Versions kept per store.
pub const LKG_DEPTH: usize = 5;
/// `schema` of a `commits.jsonl` row.
pub const COMMIT_SCHEMA: &str = "roko.guarded_commit/1";
/// The stores' directory under `.roko/learn/`.
pub const COMMITS_DIR: &str = "commits";
/// The commit log in a store's directory.
pub const COMMITS_FILE: &str = "commits.jsonl";

/// A learned store under guard.
pub trait GuardedState {
    /// The state as bytes; [`Self::restore`] of them gives the state back.
    fn snapshot(&self) -> Vec<u8>;

    /// Replace the state with a snapshot.
    ///
    /// # Errors
    ///
    /// [`GuardError::BadSnapshot`] when the bytes are not a snapshot of this
    /// store.
    fn restore(&mut self, snapshot: &[u8]) -> Result<(), GuardError>;

    /// `b3:` digest of the snapshot.
    fn digest(&self) -> String {
        b3_digest(&self.snapshot())
    }
}

/// One check's result, with its reason and the numbers behind it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CheckOutcome {
    /// The check: `held_out` or `anchors`.
    pub check: String,
    /// Whether the candidate passed it.
    pub passed: bool,
    /// Why, in words.
    pub reason: String,
    /// The numbers behind it, such as `candidate_log_loss` and
    /// `lkg_log_loss`.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub numbers: BTreeMap<String, f64>,
}

impl CheckOutcome {
    /// A passed check.
    #[must_use]
    pub fn pass(check: impl Into<String>, reason: impl Into<String>) -> Self {
        Self {
            check: check.into(),
            passed: true,
            reason: reason.into(),
            numbers: BTreeMap::new(),
        }
    }

    /// A failed check.
    #[must_use]
    pub fn fail(check: impl Into<String>, reason: impl Into<String>) -> Self {
        Self {
            passed: false,
            ..Self::pass(check, reason)
        }
    }

    /// The outcome with one more number.
    #[must_use]
    pub fn with(mut self, name: impl Into<String>, value: f64) -> Self {
        self.numbers.insert(name.into(), value);
        self
    }
}

/// The checks a candidate must pass to replace the LKG (decision 8103 sets
/// them per store).
pub trait CommitCheck {
    /// The candidate against the LKG on data neither was fitted on;
    /// `lkg` is `None` before the first version.
    fn held_out(&self, candidate: &[u8], lkg: Option<&[u8]>) -> CheckOutcome;

    /// The candidate against human-edited anchors and invariants.
    fn anchors(&self, candidate: &[u8]) -> CheckOutcome;
}

/// What a proposal, or a rollback, came to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommitDecision {
    /// Every check passed: the candidate is the new version.
    Committed,
    /// A check failed in enforce mode: the LKG was restored.
    RolledBack,
    /// A check failed in observe mode: the candidate was committed, and
    /// the row records the rollback that would have happened.
    Observed,
    /// A person or a restart restored a kept version.
    Restored,
}

/// What a failed check does.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GuardMode {
    /// Commit anyway and log the would-be rollback (decision 8103: the
    /// first ten runs, until a person switches the store to enforce).
    #[default]
    Observe,
    /// Restore the LKG.
    Enforce,
}

/// Who proposes a version, and why.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Proposer {
    /// The learner or person: `homeostat`, `router`, `knowledge`, `human`.
    pub actor: String,
    /// The version's provenance.
    pub provenance: ConfigProvenance,
}

impl Proposer {
    /// A learner's version: `ConfigSource::Evolved` with `reason`.
    #[must_use]
    pub fn evolved(actor: impl Into<String>, store: &str, reason: impl Into<String>) -> Self {
        Self {
            actor: actor.into(),
            provenance: ConfigProvenance::evolved(store, reason),
        }
    }

    /// A person's rollback from the command line.
    #[must_use]
    pub fn human(store: &str, reason: impl Into<String>) -> Self {
        Self {
            actor: "human".to_string(),
            provenance: ConfigProvenance::cli_override(store, reason),
        }
    }
}

/// One `commits.jsonl` row.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CommitRow {
    /// [`COMMIT_SCHEMA`].
    pub schema: String,
    /// The version the store holds after the decision.
    pub version: u64,
    /// The version it held before; `None` for the first.
    pub parent: Option<u64>,
    /// `b3:` digest of the proposed (or restored) snapshot.
    pub digest: String,
    /// The checks' outcomes; empty for a restore.
    pub checks: Vec<CheckOutcome>,
    /// What happened.
    pub decision: CommitDecision,
    /// The store's mode at the time.
    pub mode: GuardMode,
    /// Who proposed it.
    pub actor: String,
    /// Where the value came from: `evolved` for a learner.
    pub source: ConfigSource,
    /// Why, such as `homeostat:ep-0007/ch-0019`.
    pub reason: Option<String>,
    /// RFC 3339 time of the row.
    pub ts: String,
}

/// Why a guarded store cannot do what was asked.
#[derive(Debug, thiserror::Error)]
pub enum GuardError {
    /// A file of the store could not be read or written.
    #[error("guarded store I/O at {path}: {source}")]
    Io {
        /// The file.
        path: String,
        /// The I/O error.
        #[source]
        source: std::io::Error,
    },
    /// A `commits.jsonl` row is not a commit row.
    #[error("guarded store row at {path}: {source}")]
    Row {
        /// The commit log.
        path: String,
        /// The parse error.
        #[source]
        source: serde_json::Error,
    },
    /// The store name could escape the commits directory.
    #[error("`{store}` is not a store name")]
    InvalidStore {
        /// The name.
        store: String,
    },
    /// The version is not kept, or never existed.
    #[error("version {version} of `{store}` is not kept")]
    NoSuchVersion {
        /// The store.
        store: String,
        /// The version.
        version: u64,
    },
    /// A check failed in enforce mode before any version exists.
    #[error("`{store}` has no last-known-good version to restore")]
    NothingToRestore {
        /// The store.
        store: String,
    },
    /// The bytes are not a snapshot of the store.
    #[error("not a snapshot of `{store}`: {reason}")]
    BadSnapshot {
        /// The store.
        store: String,
        /// Why.
        reason: String,
    },
}

/// One learned store's versions and commit log, in
/// `.roko/learn/commits/<store>/`.
#[derive(Debug, Clone)]
pub struct GuardedStore {
    dir: PathBuf,
    store: String,
    mode: GuardMode,
    current: Option<u64>,
    next_version: u64,
}

impl GuardedStore {
    /// Open the store `store` under `learn_dir`, creating its directory.
    /// After a restart the current version is the last row's, so
    /// [`Self::restore_top`] can put it back.
    ///
    /// # Errors
    ///
    /// [`GuardError::InvalidStore`] for a name with a path separator or a
    /// leading dot, and I/O or row errors reading the directory.
    pub fn open(learn_dir: &Path, store: &str, mode: GuardMode) -> Result<Self, GuardError> {
        if store.is_empty() || store.starts_with('.') || store.contains(['/', '\\']) {
            return Err(GuardError::InvalidStore {
                store: store.to_string(),
            });
        }
        let dir = learn_dir.join(COMMITS_DIR).join(store);
        std::fs::create_dir_all(&dir).map_err(|source| io_error(&dir, source))?;
        let mut opened = Self {
            dir,
            store: store.to_string(),
            mode,
            current: None,
            next_version: 1,
        };
        let rows = opened.rows()?;
        let versions = opened.versions()?;
        opened.current = rows
            .last()
            .map(|row| row.version)
            .or_else(|| versions.last().copied());
        let highest = rows
            .iter()
            .map(|row| row.version)
            .chain(versions.iter().copied())
            .max()
            .unwrap_or(0);
        opened.next_version = highest + 1;
        Ok(opened)
    }

    /// The store's directory.
    #[must_use]
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// The store's mode.
    #[must_use]
    pub const fn mode(&self) -> GuardMode {
        self.mode
    }

    /// Switch the mode: a person's choice (decision 8103).
    pub fn set_mode(&mut self, mode: GuardMode) {
        self.mode = mode;
    }

    /// The version the store holds; `None` before the first.
    #[must_use]
    pub const fn current(&self) -> Option<u64> {
        self.current
    }

    /// The versions kept on disk, oldest first.
    ///
    /// # Errors
    ///
    /// I/O errors reading the directory.
    pub fn versions(&self) -> Result<Vec<u64>, GuardError> {
        let entries = std::fs::read_dir(&self.dir).map_err(|source| io_error(&self.dir, source))?;
        let mut versions = Vec::new();
        for entry in entries {
            let entry = entry.map_err(|source| io_error(&self.dir, source))?;
            let name = entry.file_name();
            let version = name
                .to_str()
                .and_then(|name| name.strip_prefix('v'))
                .and_then(|name| name.strip_suffix(".snap"))
                .and_then(|number| number.parse::<u64>().ok());
            versions.extend(version);
        }
        versions.sort_unstable();
        Ok(versions)
    }

    /// The commit log, oldest first.
    ///
    /// # Errors
    ///
    /// I/O errors, or a row that does not parse.
    pub fn rows(&self) -> Result<Vec<CommitRow>, GuardError> {
        let path = self.dir.join(COMMITS_FILE);
        let text = match std::fs::read_to_string(&path) {
            Ok(text) => text,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(source) => return Err(io_error(&path, source)),
        };
        text.lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| {
                serde_json::from_str(line).map_err(|source| GuardError::Row {
                    path: path.display().to_string(),
                    source,
                })
            })
            .collect()
    }

    /// The snapshot of a kept version.
    ///
    /// # Errors
    ///
    /// [`GuardError::NoSuchVersion`] when it is not kept.
    pub fn snapshot(&self, version: u64) -> Result<Vec<u8>, GuardError> {
        let path = self.snapshot_path(version);
        match std::fs::read(&path) {
            Ok(bytes) => Ok(bytes),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                Err(GuardError::NoSuchVersion {
                    store: self.store.clone(),
                    version,
                })
            }
            Err(source) => Err(io_error(&path, source)),
        }
    }

    /// Propose `state`, as the learner left it, as the next version. A
    /// rolled-back proposal restores the LKG into `state`.
    ///
    /// # Errors
    ///
    /// [`GuardError::NothingToRestore`] when a check fails in enforce mode
    /// before any version exists (the state keeps the candidate), and I/O
    /// errors.
    pub fn propose<S: GuardedState>(
        &mut self,
        state: &mut S,
        checks: &dyn CommitCheck,
        proposer: &Proposer,
    ) -> Result<CommitDecision, GuardError> {
        let candidate = state.snapshot();
        let parent = self.current;
        let lkg = match parent {
            Some(version) => Some(self.snapshot(version)?),
            None => None,
        };
        let outcomes = vec![
            checks.held_out(&candidate, lkg.as_deref()),
            checks.anchors(&candidate),
        ];
        let decision = if outcomes.iter().all(|outcome| outcome.passed) {
            CommitDecision::Committed
        } else if self.mode == GuardMode::Observe {
            CommitDecision::Observed
        } else {
            CommitDecision::RolledBack
        };
        let version = if decision == CommitDecision::RolledBack {
            let (Some(version), Some(bytes)) = (parent, lkg.as_deref()) else {
                return Err(GuardError::NothingToRestore {
                    store: self.store.clone(),
                });
            };
            state.restore(bytes)?;
            version
        } else {
            self.commit(&candidate)?
        };
        let digest = b3_digest(&candidate);
        let row = self.row(version, parent, digest, outcomes, decision, proposer);
        self.append(&row)?;
        Ok(decision)
    }

    /// Restore the kept version `to` into `state` and make it current.
    ///
    /// # Errors
    ///
    /// [`GuardError::NoSuchVersion`] when `to` is not kept, a snapshot the
    /// state rejects, and I/O errors.
    pub fn rollback<S: GuardedState>(
        &mut self,
        state: &mut S,
        to: u64,
        proposer: &Proposer,
    ) -> Result<(), GuardError> {
        let bytes = self.snapshot(to)?;
        state.restore(&bytes)?;
        let parent = self.current;
        self.current = Some(to);
        let row = self.row(
            to,
            parent,
            b3_digest(&bytes),
            Vec::new(),
            CommitDecision::Restored,
            proposer,
        );
        self.append(&row)
    }

    /// Put the current version back into `state`, as at a start after an
    /// abnormal exit. Returns the version restored; `None` before the first.
    ///
    /// # Errors
    ///
    /// As [`Self::rollback`].
    pub fn restore_top<S: GuardedState>(&self, state: &mut S) -> Result<Option<u64>, GuardError> {
        let Some(version) = self.current else {
            return Ok(None);
        };
        state.restore(&self.snapshot(version)?)?;
        Ok(Some(version))
    }

    fn snapshot_path(&self, version: u64) -> PathBuf {
        self.dir.join(format!("v{version}.snap"))
    }

    /// Write `bytes` as the next version, make it current and drop versions
    /// beyond [`LKG_DEPTH`].
    fn commit(&mut self, bytes: &[u8]) -> Result<u64, GuardError> {
        let version = self.next_version;
        let path = self.snapshot_path(version);
        let temp = path.with_extension("snap.tmp");
        let mut file = std::fs::File::create(&temp).map_err(|source| io_error(&temp, source))?;
        file.write_all(bytes)
            .and_then(|()| file.sync_all())
            .map_err(|source| io_error(&temp, source))?;
        std::fs::rename(&temp, &path).map_err(|source| io_error(&path, source))?;
        self.next_version += 1;
        self.current = Some(version);
        let versions = self.versions()?;
        let excess = versions.len().saturating_sub(LKG_DEPTH);
        for &old in &versions[..excess] {
            if Some(old) != self.current {
                let old_path = self.snapshot_path(old);
                std::fs::remove_file(&old_path).map_err(|source| io_error(&old_path, source))?;
            }
        }
        Ok(version)
    }

    fn row(
        &self,
        version: u64,
        parent: Option<u64>,
        digest: String,
        checks: Vec<CheckOutcome>,
        decision: CommitDecision,
        proposer: &Proposer,
    ) -> CommitRow {
        CommitRow {
            schema: COMMIT_SCHEMA.to_string(),
            version,
            parent,
            digest,
            checks,
            decision,
            mode: self.mode,
            actor: proposer.actor.clone(),
            source: proposer.provenance.source.clone(),
            reason: proposer.provenance.reason.clone(),
            ts: chrono::Utc::now().to_rfc3339(),
        }
    }

    fn append(&self, row: &CommitRow) -> Result<(), GuardError> {
        let path = self.dir.join(COMMITS_FILE);
        let mut line = serde_json::to_string(row).map_err(|source| GuardError::Row {
            path: path.display().to_string(),
            source,
        })?;
        line.push('\n');
        std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .and_then(|mut file| file.write_all(line.as_bytes()))
            .map_err(|source| io_error(&path, source))
    }
}

fn io_error(path: &Path, source: std::io::Error) -> GuardError {
    GuardError::Io {
        path: path.display().to_string(),
        source,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A store holding one number.
    #[derive(Debug, Default)]
    struct Counter(i64);

    impl GuardedState for Counter {
        fn snapshot(&self) -> Vec<u8> {
            self.0.to_string().into_bytes()
        }

        fn restore(&mut self, snapshot: &[u8]) -> Result<(), GuardError> {
            let text = std::str::from_utf8(snapshot).unwrap_or_default();
            self.0 = text.parse().map_err(|_| GuardError::BadSnapshot {
                store: "counter".to_string(),
                reason: format!("{text:?} is not a number"),
            })?;
            Ok(())
        }
    }

    /// Held out: no more than 2 below the LKG. Anchors: never negative.
    struct NoRegression;

    fn value(bytes: &[u8]) -> i64 {
        std::str::from_utf8(bytes)
            .ok()
            .and_then(|text| text.parse().ok())
            .unwrap_or(i64::MIN)
    }

    impl CommitCheck for NoRegression {
        fn held_out(&self, candidate: &[u8], lkg: Option<&[u8]>) -> CheckOutcome {
            let Some(lkg) = lkg else {
                return CheckOutcome::pass("held_out", "no last-known-good yet");
            };
            let (candidate, lkg) = (value(candidate), value(lkg));
            let outcome = if candidate >= lkg.saturating_sub(2) {
                CheckOutcome::pass("held_out", "within 2 of the last-known-good")
            } else {
                CheckOutcome::fail("held_out", "more than 2 below the last-known-good")
            };
            outcome
                .with("candidate", candidate as f64)
                .with("lkg", lkg as f64)
        }

        fn anchors(&self, candidate: &[u8]) -> CheckOutcome {
            if value(candidate) >= 0 {
                CheckOutcome::pass("anchors", "not negative")
            } else {
                CheckOutcome::fail("anchors", "negative")
            }
        }
    }

    fn proposer(change: &str) -> Proposer {
        Proposer::evolved(
            "homeostat",
            "counter",
            format!("homeostat:ep-0001/{change}"),
        )
    }

    #[test]
    fn guarded_commit_rolls_back_on_failed_check() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut store =
            GuardedStore::open(dir.path(), "counter", GuardMode::Enforce).expect("open");
        assert_eq!(store.dir(), dir.path().join("commits").join("counter"));
        let mut state = Counter(10);
        let checks = NoRegression;

        // The first version and a better one commit.
        assert_eq!(
            store
                .propose(&mut state, &checks, &proposer("ch-1"))
                .expect("v1"),
            CommitDecision::Committed
        );
        state.0 = 12;
        assert_eq!(
            store
                .propose(&mut state, &checks, &proposer("ch-2"))
                .expect("v2"),
            CommitDecision::Committed
        );
        assert_eq!(store.current(), Some(2));

        // A held-out regression rolls back to the LKG in enforce mode.
        state.0 = 5;
        assert_eq!(
            store
                .propose(&mut state, &checks, &proposer("ch-3"))
                .expect("rolled back"),
            CommitDecision::RolledBack
        );
        assert_eq!(state.0, 12, "the live state is the LKG again");
        assert_eq!(store.current(), Some(2));
        assert_eq!(store.versions().expect("versions"), [1, 2]);

        // In observe mode a failed anchor commits and logs the would-be
        // rollback.
        store.set_mode(GuardMode::Observe);
        state.0 = -1;
        assert_eq!(
            store
                .propose(&mut state, &checks, &proposer("ch-4"))
                .expect("observed"),
            CommitDecision::Observed
        );
        assert_eq!(state.0, -1);
        assert_eq!(store.current(), Some(3));

        // A person rolls back to version 2.
        store
            .rollback(&mut state, 2, &Proposer::human("counter", "undo ch-4"))
            .expect("rollback");
        assert_eq!(state.0, 12);
        assert_eq!(store.current(), Some(2));

        let rows = store.rows().expect("rows");
        let decisions: Vec<CommitDecision> = rows.iter().map(|row| row.decision).collect();
        assert_eq!(
            decisions,
            [
                CommitDecision::Committed,
                CommitDecision::Committed,
                CommitDecision::RolledBack,
                CommitDecision::Observed,
                CommitDecision::Restored
            ]
        );
        let versions: Vec<(u64, Option<u64>)> =
            rows.iter().map(|row| (row.version, row.parent)).collect();
        assert_eq!(
            versions,
            [
                (1, None),
                (2, Some(1)),
                (2, Some(2)),
                (3, Some(2)),
                (2, Some(3))
            ]
        );
        let rolled_back = &rows[2];
        assert_eq!(rolled_back.mode, GuardMode::Enforce);
        assert_eq!(rolled_back.digest, b3_digest(b"5"));
        assert!(!rolled_back.checks[0].passed);
        assert_eq!(rolled_back.checks[0].numbers["lkg"], 12.0);
        assert_eq!(rolled_back.source, ConfigSource::Evolved);
        assert_eq!(rolled_back.actor, "homeostat");
        assert_eq!(
            rolled_back.reason.as_deref(),
            Some("homeostat:ep-0001/ch-3")
        );
        assert_eq!(rows[4].actor, "human");
        assert!(rows[4].checks.is_empty());

        // Enforce mode with nothing to restore refuses, and writes nothing.
        let mut empty =
            GuardedStore::open(dir.path(), "fresh", GuardMode::Enforce).expect("open fresh");
        let mut negative = Counter(-3);
        assert!(matches!(
            empty.propose(&mut negative, &checks, &proposer("ch-1")),
            Err(GuardError::NothingToRestore { .. })
        ));
        assert!(empty.rows().expect("rows").is_empty());
        assert!(matches!(
            GuardedStore::open(dir.path(), "../escape", GuardMode::Enforce),
            Err(GuardError::InvalidStore { .. })
        ));
    }

    #[test]
    fn lkg_stack_survives_restart() {
        let dir = tempfile::tempdir().expect("tempdir");
        let checks = NoRegression;
        {
            let mut store =
                GuardedStore::open(dir.path(), "counter", GuardMode::Enforce).expect("open");
            let mut state = Counter::default();
            for value in 1..=7 {
                state.0 = value;
                let decision = store
                    .propose(&mut state, &checks, &proposer("ch"))
                    .expect("commit");
                assert_eq!(decision, CommitDecision::Committed);
            }
            // Only the newest five versions are kept, and no temp file stays.
            assert_eq!(store.versions().expect("versions"), [3, 4, 5, 6, 7]);
            let leftovers = std::fs::read_dir(store.dir())
                .expect("read dir")
                .filter_map(Result::ok)
                .filter(|entry| entry.file_name().to_string_lossy().ends_with(".tmp"))
                .count();
            assert_eq!(leftovers, 0);
        }

        // A restart finds the current version and restores it.
        let mut store =
            GuardedStore::open(dir.path(), "counter", GuardMode::Enforce).expect("reopen");
        assert_eq!(store.current(), Some(7));
        let mut state = Counter::default();
        assert_eq!(store.restore_top(&mut state).expect("restore"), Some(7));
        assert_eq!(state.0, 7);

        // A rollback survives the next restart too.
        store
            .rollback(&mut state, 4, &Proposer::human("counter", "back to 4"))
            .expect("rollback to 4");
        assert!(matches!(
            store.rollback(&mut state, 1, &Proposer::human("counter", "too old")),
            Err(GuardError::NoSuchVersion { version: 1, .. })
        ));
        drop(store);
        let mut store =
            GuardedStore::open(dir.path(), "counter", GuardMode::Enforce).expect("reopen again");
        let mut restarted = Counter::default();
        assert_eq!(store.restore_top(&mut restarted).expect("restore"), Some(4));
        assert_eq!(restarted.0, 4);

        // Version numbers never repeat after a restart.
        restarted.0 = 5;
        store
            .propose(&mut restarted, &checks, &proposer("ch-next"))
            .expect("commit after restart");
        assert_eq!(store.current(), Some(8));
        let last = store.rows().expect("rows").pop().expect("a row");
        assert_eq!((last.version, last.parent), (8, Some(4)));

        // A snapshot the state cannot read is refused.
        std::fs::write(store.dir().join("v8.snap"), b"not a number").expect("corrupt v8");
        assert!(matches!(
            store.restore_top(&mut Counter::default()),
            Err(GuardError::BadSnapshot { .. })
        ));
    }
}
