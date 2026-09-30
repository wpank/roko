//! Synchronous Write-Ahead Log for learning state durability (S16.7, S19.1).
//!
//! # Overview
//!
//! The WAL gives crash-safe durability to learning state that would
//! otherwise be lost if the process exits between in-memory updates and the
//! next periodic snapshot flush.
//!
//! Each writer journals into its own [`WalSegment`] under
//! `.roko/learn/wal/`. Each line is a JSON-serialised [`WalEntry`] appended
//! with `sync_data()` before returning, guaranteeing that the entry reaches
//! durable storage even on power loss. `.roko/learn/wal.jsonl` is the shared
//! WAL that writers used before segments; it is still replayed, and
//! [`WalWriter`] still reads and writes it.
//!
//! # Write-Ahead Protocol
//!
//! ```text
//! in-memory update
//!      │
//!      ├──► WalSegment::append(entry)  // durable before returning
//!      │
//! snapshot save ──► WalSegment::truncate()
//!      │
//! ... process may crash here ...
//!      │
//! next open
//!      │
//!      ├──► orphaned_segments(dir)     // segments no live writer holds
//!      │
//!      └──► replay into the snapshots, then remove them
//! ```
//!
//! After a successful snapshot save (e.g. writing `cascade-router.json`), the
//! writer truncates its segment. This is the "checkpoint" step: the durable
//! snapshot now covers all state that was in the segment, so it is safe to
//! clear, and the WAL stays bounded however long the writer runs
//! (bug-7a2630).
//!
//! A writer holds an exclusive lock on its segment for as long as it lives.
//! An opener replays only segments whose lock it can take, the orphans of
//! writers that are gone, and leaves a live writer's segment to that writer:
//! the writer saves those entries itself, and replaying them as well would
//! count them twice (bug-84de98).
//!
//! # Entry Types
//!
//! | Variant | What it records |
//! |---------|----------------|
//! | `CascadeObservation` | One cascade router LinUCB arm update (model slug, context features, reward) |
//! | `ModelCallObservation` | The same update, journaled by a model-call surface (see below) |
//! | `ModelCallObservationsFolded` | Ids of model-call observations already in a snapshot |
//! | `ExperimentOutcome` | One A/B prompt experiment trial (variant ID, success flag) |
//! | `GateThresholdUpdate` | One gate rung EMA update (rung index, passed flag) |
//!
//! Each variant carries exactly the fields needed to replay the in-memory update,
//! not a full snapshot. This keeps entries small (~100-400 bytes each).
//!
//! # Model-call observations
//!
//! Model-call surfaces (feedback Path B: chat, direct dispatch, ACP, the
//! vision loop, serve) journal each observation with an `id` before they
//! apply it. In the shared `wal.jsonl` they wrote, a
//! `ModelCallObservationsFolded` marker names the ids a saved snapshot
//! contains, and replay skips those. Segments need no markers: a writer
//! truncates its segment once a saved snapshot holds its entries.
//!
//! # Crash Safety
//!
//! - `WalWriter` opens the file in append mode and calls `sync_data()` after
//!   each entry. **Do not wrap in `BufWriter`** — buffering defeats the
//!   crash-safety guarantee.
//! - [`replay_wal`] skips malformed lines (e.g. a truncated tail write from a
//!   crash during `write_all`) rather than returning an error, so a partial
//!   entry never blocks recovery.
//! - If the WAL file is absent, `replay_wal` returns an empty `Vec` and
//!   `WalWriter::open` creates it fresh.
//! - A segment becomes visible under its final name only once its writer
//!   holds the lock, so an opener never mistakes a live writer's segment
//!   for an orphan.

use std::collections::HashSet;
use std::fs::{File, OpenOptions, TryLockError};
use std::io::{self, BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// A single durable learning event.
///
/// Each variant carries exactly the fields needed to replay the in-memory
/// update -- not the full snapshot. This keeps WAL entries small (~100-400
/// bytes each) while preserving full replay fidelity.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum WalEntry {
    /// A cascade router observation: confidence stats + LinUCB arm update.
    CascadeObservation {
        /// Model slug that was routed to.
        model_slug: String,
        /// Context feature vector passed to LinUCB at routing time.
        context_features: Vec<f64>,
        /// Index into the router's model slug list.
        model_idx: usize,
        /// Scalar reward computed from gate outcome.
        reward: f64,
        /// Whether the task gated successfully.
        success: bool,
        /// Unix timestamp in milliseconds.
        ts_ms: i64,
    },
    /// A cascade router observation journaled by a model-call surface, or by
    /// a Graph run's routing sink, before it was applied.
    ///
    /// Replay skips it once a [`WalEntry::ModelCallObservationsFolded`]
    /// marker names its `id`.
    ModelCallObservation {
        /// Unique journal id, named by the fold marker.
        id: String,
        /// Model slug that was called.
        model_slug: String,
        /// Context feature vector applied to LinUCB.
        context_features: Vec<f64>,
        /// Index into the router's model slug list.
        model_idx: usize,
        /// Scalar reward applied to LinUCB.
        reward: f64,
        /// Whether the call counts as a success.
        success: bool,
        /// Share of a full observation the `LinUCB` update carried, from 0.0
        /// to 1.0: below 1 for a dampened override outcome, which a Graph run
        /// journals too (bug-dfb28f). An entry leaves a full observation's
        /// weight out, and entries written before the field carried one.
        #[serde(
            default = "full_observation_weight",
            skip_serializing_if = "is_full_observation_weight"
        )]
        weight: f64,
        /// Unix timestamp in milliseconds.
        ts_ms: i64,
    },
    /// The listed model-call observations are in a saved `cascade-router.json`.
    ModelCallObservationsFolded {
        /// Ids of the [`WalEntry::ModelCallObservation`] entries it contains.
        ids: Vec<String>,
        /// Unix timestamp in milliseconds.
        ts_ms: i64,
    },
    /// A prompt experiment trial outcome.
    ExperimentOutcome {
        /// Variant ID that was tested.
        variant_id: String,
        /// Whether the variant succeeded.
        success: bool,
        /// Unix timestamp in milliseconds.
        ts_ms: i64,
    },
    /// A gate rung adaptive threshold EMA update.
    GateThresholdUpdate {
        /// Gate rung index (0-based).
        rung: u32,
        /// Whether the rung passed.
        passed: bool,
        /// Unix timestamp in milliseconds.
        ts_ms: i64,
    },
}

/// Append-only WAL writer backed by a plain `File`.
///
/// Writes are synchronous and call `sync_data()` after each append to
/// guarantee durability. Do NOT wrap in `BufWriter` -- buffering would
/// defeat the crash-safety guarantee.
pub struct WalWriter {
    path: PathBuf,
    file: File,
    entry_count: usize,
}

impl WalWriter {
    /// Open the WAL at `path`, creating it if absent.
    ///
    /// Returns both the writer and the existing entry count (so the
    /// caller can decide whether to replay before proceeding).
    pub fn open(path: &Path) -> io::Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        // Count existing entries before opening in append mode.
        let entry_count = if path.exists() {
            BufReader::new(File::open(path)?).lines().count()
        } else {
            0
        };
        let file = OpenOptions::new().create(true).append(true).open(path)?;
        Ok(Self {
            path: path.to_path_buf(),
            file,
            entry_count,
        })
    }

    /// Append `entry` to the WAL, flushing to the OS page cache and
    /// syncing the data to durable storage before returning.
    pub fn append(&mut self, entry: &WalEntry) -> io::Result<()> {
        write_entry(&mut self.file, entry)?;
        self.entry_count += 1;
        Ok(())
    }

    /// Number of entries written since the WAL was last truncated.
    pub fn entry_count(&self) -> usize {
        self.entry_count
    }

    /// Truncate the WAL to zero after a successful snapshot save.
    ///
    /// Reopens the file with `O_TRUNC` so that subsequent appends start
    /// from an empty file. `entry_count` resets to zero.
    pub fn truncate(&mut self) -> io::Result<()> {
        // Close the append-mode handle, open truncate, then reopen append.
        let _ = std::mem::replace(
            &mut self.file,
            OpenOptions::new()
                .write(true)
                .truncate(true)
                .open(&self.path)?,
        );
        self.file = OpenOptions::new().append(true).open(&self.path)?;
        self.entry_count = 0;
        Ok(())
    }
}

/// The weight of a full observation, which a [`WalEntry::ModelCallObservation`]
/// without one carried.
const fn full_observation_weight() -> f64 {
    1.0
}

/// Whether an observation carried a full observation's weight, which its
/// entry leaves out.
#[allow(clippy::trivially_copy_pass_by_ref)] // serde passes a reference
fn is_full_observation_weight(weight: &f64) -> bool {
    (weight - 1.0).abs() < f64::EPSILON
}

/// Append one `entry` to the WAL at `path`, creating it if absent, and sync
/// it to durable storage before returning.
///
/// For writers that journal an entry now and then: unlike
/// [`WalWriter::open`], it does not count the existing entries first.
pub fn append_entry(path: &Path, entry: &WalEntry) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    write_entry(&mut file, entry)
}

/// Empty the WAL at `path`, once saved snapshots hold all its entries.
pub fn truncate_wal(path: &Path) -> io::Result<()> {
    OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(path)?
        .sync_all()
}

fn write_entry(file: &mut File, entry: &WalEntry) -> io::Result<()> {
    let mut line =
        serde_json::to_string(entry).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    line.push('\n');
    file.write_all(line.as_bytes())?;
    file.sync_data()
}

/// Ids of the model-call observations that a saved snapshot already contains.
///
/// Replay must skip these: applying them again would count them twice.
pub fn folded_model_call_ids(entries: &[WalEntry]) -> HashSet<&str> {
    entries
        .iter()
        .filter_map(|entry| match entry {
            WalEntry::ModelCallObservationsFolded { ids, .. } => Some(ids),
            _ => None,
        })
        .flatten()
        .map(String::as_str)
        .collect()
}

/// Read and deserialize all entries from `path`.
///
/// Malformed lines are logged as warnings and skipped rather than
/// returning an error, so that a partially-written tail entry (from a
/// crash during `write_all`) does not block replay.
///
/// Returns an empty `Vec` if the WAL file does not exist.
pub fn replay_wal(path: &Path) -> io::Result<Vec<WalEntry>> {
    let file = match File::open(path) {
        Ok(f) => f,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(vec![]),
        Err(e) => return Err(e),
    };
    read_entries(&file)
}

/// Deserialize every entry in `file`, skipping malformed lines.
fn read_entries(file: &File) -> io::Result<Vec<WalEntry>> {
    let mut entries = Vec::new();
    for (i, line) in BufReader::new(file).lines().enumerate() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        match serde_json::from_str::<WalEntry>(&line) {
            Ok(entry) => entries.push(entry),
            Err(e) => {
                tracing::warn!(line = i, error = %e, "[wal] skipping malformed entry");
            }
        }
    }
    Ok(entries)
}

// ── Per-writer segments ─────────────────────────────────────────────────

/// Directory beside `wal.jsonl` that holds one [`WalSegment`] per writer.
pub const SEGMENTS_DIR: &str = "wal";

/// A WAL file that one writer owns, locked for as long as the writer lives.
///
/// Only its writer appends to a segment, and the writer truncates it once a
/// saved snapshot holds its entries, so the segment stays bounded however
/// long the writer runs (bug-7a2630). The lock tells an opener that the
/// entries are a live writer's to save, not orphans to replay (bug-84de98).
/// The writer releases the lock when it drops the segment, rather than
/// leaving that to the file's close (bug-779ae7).
#[derive(Debug)]
pub struct WalSegment {
    path: PathBuf,
    file: File,
    entry_count: usize,
}

impl WalSegment {
    /// Create a segment in `dir` and lock it.
    ///
    /// The segment is locked under a staging name and only then renamed
    /// into place, so an opener never finds it unlocked while its writer
    /// lives.
    pub fn create(dir: &Path) -> io::Result<Self> {
        std::fs::create_dir_all(dir)?;
        let name = format!("{}-{}", std::process::id(), uuid::Uuid::new_v4());
        let staging = dir.join(format!("{name}.jsonl.tmp"));
        let path = dir.join(format!("{name}.jsonl"));
        let file = OpenOptions::new()
            .create_new(true)
            .append(true)
            .open(&staging)?;
        if let Err(error) = file.lock().and_then(|()| std::fs::rename(&staging, &path)) {
            let _ = std::fs::remove_file(&staging);
            return Err(error);
        }
        Ok(Self {
            path,
            file,
            entry_count: 0,
        })
    }

    /// Where the segment lives.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Entries appended since the segment was created or last truncated.
    pub fn entry_count(&self) -> usize {
        self.entry_count
    }

    /// Append `entry` and sync it to durable storage before returning.
    pub fn append(&mut self, entry: &WalEntry) -> io::Result<()> {
        write_entry(&mut self.file, entry)?;
        self.entry_count += 1;
        Ok(())
    }

    /// Drop every entry, once a saved snapshot holds them all.
    pub fn truncate(&mut self) -> io::Result<()> {
        self.file.set_len(0)?;
        self.file.sync_data()?;
        self.entry_count = 0;
        Ok(())
    }
}

impl Drop for WalSegment {
    fn drop(&mut self) {
        // An empty segment has nothing to replay. The writer still holds the
        // lock here, so no opener is reading it.
        if self.entry_count == 0 {
            let _ = std::fs::remove_file(&self.path);
        }
        // Closing the file does not release the lock while another process
        // shares the open file: any child process forked while the segment
        // was open does, until its exec. An opener would then skip the gone
        // writer's entries as a live writer's (bug-779ae7).
        let _ = self.file.unlock();
    }
}

/// Append `entry` to the writer's segment in `dir`, creating the segment on
/// the writer's first append.
pub fn append_to_segment(
    segment: &mut Option<WalSegment>,
    dir: &Path,
    entry: &WalEntry,
) -> io::Result<()> {
    let segment = match segment {
        Some(segment) => segment,
        slot @ None => slot.insert(WalSegment::create(dir)?),
    };
    segment.append(entry)
}

/// A segment whose writer is gone, locked by the opener that replays it.
#[derive(Debug)]
pub struct OrphanedSegment {
    path: PathBuf,
    file: File,
    entries: Vec<WalEntry>,
}

impl OrphanedSegment {
    /// The entries its writer journaled and never saved.
    pub fn entries(&self) -> &[WalEntry] {
        &self.entries
    }

    /// Delete the segment once saved snapshots hold its entries.
    ///
    /// The segment is emptied before the lock is released, so no other
    /// opener replays it again, even where an open file cannot be removed.
    pub fn remove(self) -> io::Result<()> {
        self.file.set_len(0)?;
        match std::fs::remove_file(&self.path) {
            // Another opener replayed and removed it first.
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
            removed => removed,
        }
    }
}

/// The segments in `dir` whose writers are gone, with their entries.
///
/// A segment whose writer still lives is skipped: that writer saves its
/// entries itself (bug-84de98). Each orphan comes back locked, so no other
/// opener replays it at the same time. A missing `dir` holds no segments.
pub fn orphaned_segments(dir: &Path) -> io::Result<Vec<OrphanedSegment>> {
    let listing = match std::fs::read_dir(dir) {
        Ok(listing) => listing,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error),
    };
    let mut orphans = Vec::new();
    for dir_entry in listing {
        let path = dir_entry?.path();
        if path
            .extension()
            .is_none_or(|extension| extension != "jsonl")
        {
            continue;
        }
        let file = match OpenOptions::new().read(true).write(true).open(&path) {
            Ok(file) => file,
            // Its writer removed it after saving.
            Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
            Err(error) => return Err(error),
        };
        match file.try_lock() {
            Ok(()) => {}
            Err(TryLockError::WouldBlock) => continue,
            Err(TryLockError::Error(error)) => return Err(error),
        }
        let entries = read_entries(&file)?;
        orphans.push(OrphanedSegment {
            path,
            file,
            entries,
        });
    }
    Ok(orphans)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn append_and_reopen_preserves_entries() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("wal.jsonl");

        {
            let mut w = WalWriter::open(&path).unwrap();
            assert_eq!(w.entry_count(), 0);

            w.append(&WalEntry::CascadeObservation {
                model_slug: "claude-sonnet-4-5".into(),
                context_features: vec![0.1, 0.2, 0.3],
                model_idx: 0,
                reward: 0.85,
                success: true,
                ts_ms: 1_000_000,
            })
            .unwrap();

            w.append(&WalEntry::GateThresholdUpdate {
                rung: 2,
                passed: false,
                ts_ms: 1_000_001,
            })
            .unwrap();

            assert_eq!(w.entry_count(), 2);
        }

        // Reopen and verify count survives.
        let w2 = WalWriter::open(&path).unwrap();
        assert_eq!(w2.entry_count(), 2);

        // Replay should produce both entries.
        let entries = replay_wal(&path).unwrap();
        assert_eq!(entries.len(), 2);
        assert!(matches!(entries[0], WalEntry::CascadeObservation { .. }));
        assert!(matches!(entries[1], WalEntry::GateThresholdUpdate { .. }));
    }

    #[test]
    fn truncate_resets_file_and_count() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("wal.jsonl");

        let mut w = WalWriter::open(&path).unwrap();
        w.append(&WalEntry::ExperimentOutcome {
            variant_id: "v1".into(),
            success: true,
            ts_ms: 42,
        })
        .unwrap();
        assert_eq!(w.entry_count(), 1);

        w.truncate().unwrap();
        assert_eq!(w.entry_count(), 0);

        // File should be empty.
        let entries = replay_wal(&path).unwrap();
        assert!(entries.is_empty());

        // Can still append after truncation.
        w.append(&WalEntry::GateThresholdUpdate {
            rung: 0,
            passed: true,
            ts_ms: 43,
        })
        .unwrap();
        assert_eq!(w.entry_count(), 1);
        let entries = replay_wal(&path).unwrap();
        assert_eq!(entries.len(), 1);
    }

    #[test]
    fn malformed_tail_is_skipped() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("wal.jsonl");

        // Write a valid entry followed by a malformed line.
        let mut w = WalWriter::open(&path).unwrap();
        w.append(&WalEntry::CascadeObservation {
            model_slug: "claude-haiku-4-5".into(),
            context_features: vec![1.0],
            model_idx: 1,
            reward: 0.5,
            success: false,
            ts_ms: 99,
        })
        .unwrap();
        drop(w);

        // Append a broken line directly.
        let mut f = OpenOptions::new().append(true).open(&path).unwrap();
        f.write_all(b"{\"kind\":\"cascade_observation\",\"broken\n")
            .unwrap();

        let entries = replay_wal(&path).unwrap();
        assert_eq!(entries.len(), 1);
        assert!(matches!(entries[0], WalEntry::CascadeObservation { .. }));
    }

    #[test]
    fn append_entry_creates_the_wal_and_folds_by_id() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("learn").join("wal.jsonl");

        for id in ["obs-1", "obs-2"] {
            append_entry(
                &path,
                &WalEntry::ModelCallObservation {
                    id: id.into(),
                    model_slug: "model-a".into(),
                    context_features: vec![0.0; 18],
                    model_idx: 0,
                    reward: 1.0,
                    success: true,
                    weight: 1.0,
                    ts_ms: 1,
                },
            )
            .unwrap();
        }
        append_entry(
            &path,
            &WalEntry::ModelCallObservationsFolded {
                ids: vec!["obs-1".into()],
                ts_ms: 2,
            },
        )
        .unwrap();

        let entries = replay_wal(&path).unwrap();
        assert_eq!(entries.len(), 3);
        let folded = folded_model_call_ids(&entries);
        assert!(folded.contains("obs-1"));
        assert!(!folded.contains("obs-2"));
    }

    #[test]
    fn only_segments_whose_writer_is_gone_are_orphans() {
        // bug-84de98: a live writer's segment is its own to save; an opener
        // replays only the segments of writers that are gone.
        let dir = TempDir::new().unwrap();
        let segments = dir.path().join(SEGMENTS_DIR);
        let entry = WalEntry::GateThresholdUpdate {
            rung: 1,
            passed: true,
            ts_ms: 1,
        };

        let mut live = WalSegment::create(&segments).unwrap();
        live.append(&entry).unwrap();
        assert!(
            orphaned_segments(&segments).unwrap().is_empty(),
            "a live writer's segment is not an orphan"
        );

        let mut saved = WalSegment::create(&segments).unwrap();
        saved.append(&entry).unwrap();
        saved.truncate().unwrap();
        // Its entries are saved, so the writer removes it when it goes.
        drop(saved);

        // This writer dies with an unsaved entry.
        drop(live);
        let orphans = orphaned_segments(&segments).unwrap();
        assert_eq!(orphans.len(), 1);
        assert_eq!(orphans[0].entries().len(), 1);
        for orphan in orphans {
            orphan.remove().unwrap();
        }
        assert_eq!(std::fs::read_dir(&segments).unwrap().count(), 0);
    }

    /// bug-779ae7: a child process that shares a segment's open file (one
    /// forked while the segment was open, until its exec) does not keep the
    /// segment locked once its writer drops it.
    #[cfg(unix)]
    #[test]
    fn a_dropped_writers_segment_is_an_orphan_while_a_child_shares_its_file() {
        let dir = TempDir::new().unwrap();
        let segments = dir.path().join(SEGMENTS_DIR);
        let entry = WalEntry::GateThresholdUpdate {
            rung: 1,
            passed: true,
            ts_ms: 1,
        };
        let mut segment = WalSegment::create(&segments).unwrap();
        segment.append(&entry).unwrap();
        let mut child = std::process::Command::new("sleep")
            .arg("30")
            .stdin(segment.file.try_clone().unwrap())
            .spawn()
            .unwrap();

        drop(segment);
        let orphans = orphaned_segments(&segments);
        child.kill().unwrap();
        child.wait().unwrap();
        let orphans = orphans.unwrap();
        assert_eq!(orphans.len(), 1, "the writer is gone");
        assert_eq!(orphans[0].entries().len(), 1);
    }

    #[test]
    fn a_model_call_observation_without_a_weight_carried_a_full_one() {
        let line = r#"{"kind":"model_call_observation","id":"obs-1","model_slug":"model-a","context_features":[],"model_idx":0,"reward":1.0,"success":true,"ts_ms":1}"#;
        let entry: WalEntry = serde_json::from_str(line).unwrap();
        let WalEntry::ModelCallObservation { weight, .. } = &entry else {
            panic!("a model-call observation: {entry:?}");
        };
        assert!((weight - 1.0).abs() < f64::EPSILON, "{weight}");
    }

    #[test]
    fn replay_missing_file_returns_empty() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("nonexistent.jsonl");
        let entries = replay_wal(&path).unwrap();
        assert!(entries.is_empty());
    }

    #[test]
    fn serde_roundtrip_all_variants() {
        let entries = vec![
            WalEntry::CascadeObservation {
                model_slug: "model-a".into(),
                context_features: vec![0.0; 18],
                model_idx: 0,
                reward: 1.0,
                success: true,
                ts_ms: 100,
            },
            WalEntry::ExperimentOutcome {
                variant_id: "variant-b".into(),
                success: false,
                ts_ms: 200,
            },
            WalEntry::GateThresholdUpdate {
                rung: 5,
                passed: true,
                ts_ms: 300,
            },
            WalEntry::ModelCallObservation {
                id: "obs-1".into(),
                model_slug: "model-c".into(),
                context_features: vec![0.0; 18],
                model_idx: 2,
                reward: 0.0,
                success: false,
                weight: 1.0,
                ts_ms: 400,
            },
            WalEntry::ModelCallObservation {
                id: "obs-2".into(),
                model_slug: "model-c".into(),
                context_features: vec![0.0; 18],
                model_idx: 2,
                reward: 0.4,
                success: true,
                weight: 0.5,
                ts_ms: 450,
            },
            WalEntry::ModelCallObservationsFolded {
                ids: vec!["obs-1".into()],
                ts_ms: 500,
            },
        ];
        for entry in &entries {
            let json = serde_json::to_string(entry).unwrap();
            let deserialized: WalEntry = serde_json::from_str(&json).unwrap();
            // Verify kind tag is present.
            let val: serde_json::Value = serde_json::from_str(&json).unwrap();
            assert!(val.get("kind").is_some());
            // Verify roundtrip produces same JSON.
            let json2 = serde_json::to_string(&deserialized).unwrap();
            assert_eq!(json, json2);
        }
    }
}
