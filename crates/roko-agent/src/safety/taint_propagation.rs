//! Monotonic trust-origin taint propagation across signal lineage.

use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use parking_lot::Mutex;
use roko_core::{ContentHash, Signal, Taint};
use serde::{Deserialize, Serialize};

pub use roko_core::provenance::TrustOriginTaintLevel;

/// Compatibility name used by the E34 trust-origin IFC contract.
pub type TaintLevel = TrustOriginTaintLevel;

/// Strict maximum serialized size accepted for a taint-tracker snapshot.
const MAX_TAINT_TRACKER_BYTES: u64 = 4 * 1024 * 1024;

/// Why a hash is tracked as tainted, independent of its lattice level.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TaintReason {
    ExternalSource { detail: String },
    UserInput { detail: String },
    ToolFailure { detail: String },
    Propagated,
    Stale { detail: String },
    Custom { category: String, detail: String },
}

impl TaintReason {
    pub fn external(detail: impl Into<String>) -> Self {
        Self::ExternalSource {
            detail: detail.into(),
        }
    }

    pub fn user_input(detail: impl Into<String>) -> Self {
        Self::UserInput {
            detail: detail.into(),
        }
    }

    pub fn tool_failure(detail: impl Into<String>) -> Self {
        Self::ToolFailure {
            detail: detail.into(),
        }
    }

    pub fn stale(detail: impl Into<String>) -> Self {
        Self::Stale {
            detail: detail.into(),
        }
    }

    pub fn custom(category: impl Into<String>, detail: impl Into<String>) -> Self {
        Self::Custom {
            category: category.into(),
            detail: detail.into(),
        }
    }

    #[must_use]
    pub fn category(&self) -> &str {
        match self {
            Self::ExternalSource { .. } => "external_source",
            Self::UserInput { .. } => "user_input",
            Self::ToolFailure { .. } => "tool_failure",
            Self::Propagated => "propagated",
            Self::Stale { .. } => "stale",
            Self::Custom { category, .. } => category,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct TaintEntry {
    level: TaintLevel,
    reason: TaintReason,
    derived_from: Vec<ContentHash>,
}

/// Durable-in-memory evidence emitted for each successful propagation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PropagationAudit {
    pub hash: ContentHash,
    pub parents: Vec<ContentHash>,
    pub resulting_level: TaintLevel,
}

/// Serializable snapshot of [`TaintTracker`] state for durable persistence.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
struct TaintTrackerSnapshot {
    taints: HashMap<ContentHash, TaintEntry>,
    audit: Vec<PropagationAudit>,
}

#[derive(Debug, Default)]
struct TrackerState {
    taints: HashMap<ContentHash, TaintEntry>,
    audit: Vec<PropagationAudit>,
}

impl TrackerState {
    fn to_snapshot(&self) -> TaintTrackerSnapshot {
        TaintTrackerSnapshot {
            taints: self.taints.clone(),
            audit: self.audit.clone(),
        }
    }

    fn from_snapshot(snap: TaintTrackerSnapshot) -> Self {
        Self {
            taints: snap.taints,
            audit: snap.audit,
        }
    }
}

/// Thread-safe tracker for taint across a derived-signal DAG.
#[derive(Debug, Default)]
pub struct TaintTracker {
    state: Mutex<TrackerState>,
}

/// Join all input trust-origin levels. Empty input is trusted.
#[must_use]
pub fn propagate_taint(inputs: &[TaintLevel]) -> TaintLevel {
    inputs
        .iter()
        .copied()
        .fold(TaintLevel::Trusted, TaintLevel::join)
}

impl TaintTracker {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Mark a hash without ever lowering an existing lattice level.
    pub fn mark_tainted(&self, hash: ContentHash, reason: TaintReason, level: TaintLevel) {
        let mut state = self.state.lock();
        state
            .taints
            .entry(hash)
            .and_modify(|entry| {
                entry.level = entry.level.join(level);
                entry.reason = reason.clone();
            })
            .or_insert(TaintEntry {
                level,
                reason,
                derived_from: Vec::new(),
            });
    }

    #[must_use]
    pub fn get_level(&self, hash: &ContentHash) -> Option<TaintLevel> {
        self.state.lock().taints.get(hash).map(|entry| entry.level)
    }

    /// Return both boolean taint status and its trust-origin lattice level.
    #[must_use]
    pub fn is_tainted(&self, hash: &ContentHash) -> (bool, Option<TaintLevel>) {
        let level = self.get_level(hash);
        (level.is_some(), level)
    }

    #[must_use]
    pub fn reason(&self, hash: &ContentHash) -> Option<TaintReason> {
        self.state
            .lock()
            .taints
            .get(hash)
            .map(|entry| entry.reason.clone())
    }

    /// Immediate tainted parents recorded for a derived hash.
    #[must_use]
    pub fn derived_from(&self, hash: &ContentHash) -> Vec<ContentHash> {
        self.state
            .lock()
            .taints
            .get(hash)
            .map_or_else(Vec::new, |entry| entry.derived_from.clone())
    }

    /// Propagate the join of all tracked parent levels to `child`.
    pub fn propagate(&self, parents: &[ContentHash], child: ContentHash) -> bool {
        let mut state = self.state.lock();
        let tainted_parents: Vec<(ContentHash, TaintLevel)> = parents
            .iter()
            .filter_map(|hash| state.taints.get(hash).map(|entry| (*hash, entry.level)))
            .collect();
        if tainted_parents.is_empty() {
            return false;
        }

        let inherited = tainted_parents
            .iter()
            .map(|(_, level)| *level)
            .fold(TaintLevel::Trusted, TaintLevel::join);
        let entry = state.taints.entry(child).or_insert(TaintEntry {
            level: TaintLevel::Trusted,
            reason: TaintReason::Propagated,
            derived_from: Vec::new(),
        });
        entry.level = entry.level.join(inherited);
        for (parent, _) in &tainted_parents {
            if !entry.derived_from.contains(parent) {
                entry.derived_from.push(*parent);
            }
        }
        let resulting_level = entry.level;
        state.audit.push(PropagationAudit {
            hash: child,
            parents: parents.to_vec(),
            resulting_level,
        });
        drop(state);

        tracing::info!(
            hash = %child,
            parents = ?parents,
            resulting_level = %resulting_level,
            "taint propagated to derived signal"
        );
        true
    }

    /// Register a signal from its provenance trust decision.
    pub fn observe_signal(&self, signal: &Signal) -> bool {
        let level = signal.effective_taint();
        if !signal.provenance.is_tainted() && level == TaintLevel::Trusted {
            return false;
        }
        let reason = match &signal.provenance.taint {
            Taint::Clean => TaintReason::custom("origin", signal.provenance.author.clone()),
            Taint::UserInput { detail } => TaintReason::user_input(detail.clone()),
            Taint::UnverifiedSource { detail } => TaintReason::external(detail.clone()),
            Taint::ToolFailure { detail } => TaintReason::tool_failure(detail.clone()),
            Taint::StaleData { threshold_ms } => {
                TaintReason::stale(format!("older than {threshold_ms}ms"))
            }
            Taint::Propagated { .. } => TaintReason::Propagated,
            other => TaintReason::custom(
                other.category(),
                other.detail().unwrap_or("tainted provenance"),
            ),
        };
        self.mark_tainted(signal.id, reason, level);
        true
    }

    #[must_use]
    pub fn audit_log(&self) -> Vec<PropagationAudit> {
        self.state.lock().audit.clone()
    }

    /// Every tracked hash with its lattice level.
    #[must_use]
    pub fn levels(&self) -> Vec<(ContentHash, TaintLevel)> {
        self.state
            .lock()
            .taints
            .iter()
            .map(|(hash, entry)| (*hash, entry.level))
            .collect()
    }

    pub fn clear(&self) {
        *self.state.lock() = TrackerState::default();
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.state.lock().taints.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.state.lock().taints.is_empty()
    }

    /// Atomically persist the tracker's current state to disk.
    ///
    /// The snapshot captures all taint entries and the propagation audit log.
    /// A temporary file is written and renamed atomically so a partial write
    /// never overwrites the last good snapshot.
    pub fn save(&self, path: impl AsRef<Path>) -> io::Result<()> {
        let snapshot = self.state.lock().to_snapshot();
        let path = path.as_ref();
        let parent = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .map_or_else(|| PathBuf::from("."), Path::to_path_buf);
        fs::create_dir_all(&parent)?;
        let file_name = path.file_name().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "taint-tracker path has no file name",
            )
        })?;
        static SAVE_SEQUENCE: AtomicU64 = AtomicU64::new(0);
        let sequence = SAVE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let temporary = parent.join(format!(
            ".{}.{}.{}.tmp",
            file_name.to_string_lossy(),
            std::process::id(),
            sequence,
        ));
        let result = (|| {
            let file = OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(&temporary)?;
            let mut writer = BufWriter::new(file);
            serde_json::to_writer_pretty(&mut writer, &snapshot)
                .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
            writer.write_all(b"\n")?;
            writer.flush()?;
            writer.get_ref().sync_all()?;
            drop(writer);
            fs::rename(&temporary, path)?;
            #[cfg(unix)]
            File::open(&parent)?.sync_all()?;
            Ok(())
        })();
        if result.is_err() {
            let _ = fs::remove_file(&temporary);
        }
        result
    }

    /// Restore a previously persisted tracker snapshot from disk.
    ///
    /// Returns a new [`TaintTracker`] with the saved taint entries and audit
    /// log. If the file does not exist the returned tracker starts empty
    /// (equivalent to [`TaintTracker::new`]). Other IO and parse errors are
    /// returned as errors.
    pub fn load(path: impl AsRef<Path>) -> io::Result<Self> {
        let path = path.as_ref();
        if !path.exists() {
            return Ok(Self::new());
        }
        let file = File::open(path)?;
        if file.metadata()?.len() > MAX_TAINT_TRACKER_BYTES {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "taint-tracker snapshot exceeds its byte limit",
            ));
        }
        let mut bytes = Vec::new();
        file.take(MAX_TAINT_TRACKER_BYTES.saturating_add(1))
            .read_to_end(&mut bytes)?;
        if bytes.len() as u64 > MAX_TAINT_TRACKER_BYTES {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "taint-tracker snapshot exceeds its byte limit",
            ));
        }
        let snapshot: TaintTrackerSnapshot = serde_json::from_slice(&bytes)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        Ok(Self {
            state: Mutex::new(TrackerState::from_snapshot(snapshot)),
        })
    }

    /// The tracker's state as JSON, the snapshot [`Self::save`] writes, for a
    /// store such as a checkpoint extension.
    #[must_use]
    pub fn to_json(&self) -> serde_json::Value {
        let snapshot = self.state.lock().to_snapshot();
        serde_json::to_value(snapshot).unwrap_or_default()
    }

    /// Rebuild a tracker from [`Self::to_json`]'s output.
    ///
    /// # Errors
    ///
    /// Returns an [`io::ErrorKind::InvalidData`] error when `value` is not
    /// such a snapshot.
    pub fn from_json(value: serde_json::Value) -> io::Result<Self> {
        let snapshot: TaintTrackerSnapshot = serde_json::from_value(value)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        Ok(Self {
            state: Mutex::new(TrackerState::from_snapshot(snapshot)),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use roko_core::{Kind, Provenance};

    fn hash(value: &[u8]) -> ContentHash {
        ContentHash::of(value)
    }

    #[test]
    fn explicit_marks_are_monotonic_and_queryable() {
        let tracker = TaintTracker::new();
        let id = hash(b"source");
        tracker.mark_tainted(id, TaintReason::external("api"), TaintLevel::External);
        tracker.mark_tainted(id, TaintReason::user_input("retry"), TaintLevel::Local);

        assert_eq!(tracker.is_tainted(&id), (true, Some(TaintLevel::External)));
        assert_eq!(tracker.reason(&id).unwrap().category(), "user_input");
    }

    #[test]
    fn propagation_joins_parents_tracks_derivation_and_audit() {
        let tracker = TaintTracker::new();
        let local = hash(b"local");
        let hostile = hash(b"hostile");
        let child = hash(b"child");
        tracker.mark_tainted(local, TaintReason::user_input("cli"), TaintLevel::Local);
        tracker.mark_tainted(
            hostile,
            TaintReason::external("unsigned"),
            TaintLevel::Untrusted,
        );

        assert!(tracker.propagate(&[local, hostile], child));
        assert_eq!(tracker.get_level(&child), Some(TaintLevel::Untrusted));
        assert_eq!(tracker.derived_from(&child), vec![local, hostile]);
        assert_eq!(
            tracker.audit_log(),
            vec![PropagationAudit {
                hash: child,
                parents: vec![local, hostile],
                resulting_level: TaintLevel::Untrusted,
            }]
        );
    }

    #[test]
    fn clean_parents_do_not_create_taint_or_audit_evidence() {
        let tracker = TaintTracker::new();
        let child = hash(b"clean child");
        assert!(!tracker.propagate(&[hash(b"clean")], child));
        assert_eq!(tracker.is_tainted(&child), (false, None));
        assert!(tracker.audit_log().is_empty());
    }

    #[test]
    fn signal_observation_uses_effective_trust_origin() {
        let tracker = TaintTracker::new();
        let signal = Signal::builder(Kind::Task)
            .provenance(Provenance::external("webhook"))
            .build();
        assert!(tracker.observe_signal(&signal));
        assert_eq!(tracker.get_level(&signal.id), Some(TaintLevel::External));
    }

    #[test]
    fn tracker_save_and_load_roundtrips_state() {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join("taint-tracker.json");

        let tracker = TaintTracker::new();
        let a = hash(b"source-a");
        let b = hash(b"source-b");
        let child = hash(b"child");
        tracker.mark_tainted(a, TaintReason::external("webhook"), TaintLevel::External);
        tracker.mark_tainted(b, TaintReason::user_input("cli"), TaintLevel::Local);
        // join(External, Local) = External (External > Local in the lattice)
        assert!(tracker.propagate(&[a, b], child));

        tracker.save(&path).expect("save tracker");

        let restored = TaintTracker::load(&path).expect("load tracker");
        assert_eq!(restored.get_level(&a), Some(TaintLevel::External));
        assert_eq!(restored.get_level(&b), Some(TaintLevel::Local));
        assert_eq!(restored.get_level(&child), Some(TaintLevel::External));
        assert_eq!(restored.derived_from(&child), vec![a, b]);
        assert_eq!(restored.audit_log().len(), 1);
    }

    #[test]
    fn tracker_json_roundtrips_state_and_refuses_other_values() {
        let tracker = TaintTracker::new();
        let parent = hash(b"parent");
        let child = hash(b"child");
        tracker.mark_tainted(parent, TaintReason::external("api"), TaintLevel::Untrusted);
        assert!(tracker.propagate(&[parent], child));

        let restored = TaintTracker::from_json(tracker.to_json()).expect("restore tracker");
        assert_eq!(restored.get_level(&child), Some(TaintLevel::Untrusted));
        assert_eq!(restored.derived_from(&child), vec![parent]);
        assert_eq!(restored.audit_log(), tracker.audit_log());
        let mut levels = restored.levels();
        levels.sort_by_key(|(hash, _)| hash.to_hex());
        let mut expected = vec![
            (parent, TaintLevel::Untrusted),
            (child, TaintLevel::Untrusted),
        ];
        expected.sort_by_key(|(hash, _)| hash.to_hex());
        assert_eq!(levels, expected);
        let error = TaintTracker::from_json(serde_json::json!({"taints": 3}))
            .err()
            .expect("not a snapshot");
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    }

    #[test]
    fn tracker_load_returns_empty_when_file_missing() {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join("nonexistent.json");
        let tracker = TaintTracker::load(&path).expect("load from missing file returns empty");
        assert!(tracker.is_empty());
        assert!(tracker.audit_log().is_empty());
    }
}
