//! Deterministic replay infrastructure for Activity nodes.
//!
//! The Workflow/Activity split ensures reproducible graph executions:
//! - **Workflow** nodes are deterministic (routing, scoring, composition) and
//!   can always be re-derived from inputs. They are never recorded.
//! - **Activity** nodes are non-deterministic (LLM calls, tool use, external
//!   APIs) and must be recorded so replay can substitute the original outputs
//!   without re-invoking the external system.
//!
//! # Usage
//!
//! ```rust,ignore
//! // Record a run
//! let recorder = ActivityRecorder::create("run-42", "/tmp/run-42.jsonl")?;
//! let engine = GraphEngine::new(graph, registry).with_recorder(recorder);
//! engine.execute(&ctx).await?;
//!
//! // Replay the run deterministically
//! let replayer = ActivityReplayer::load("/tmp/run-42.jsonl")?;
//! let engine = GraphEngine::new(graph, registry).with_replayer(replayer);
//! engine.execute(&ctx).await?;
//! ```

use std::collections::{HashMap, HashSet};
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// A single recorded Activity node execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordEntry {
    /// Identifier of the graph that produced this entry.
    pub graph_id: String,
    /// Unique identifier for the specific run (e.g. UUID or timestamp-slug).
    pub run_id: String,
    /// The node within the graph that was executed.
    pub node_id: String,
    /// Tick index (0 for one-shot graphs; increments for Hot Graph ticks).
    pub tick: u64,
    /// Outputs produced by the node's cell execution.
    #[serde(alias = "engrams")]
    pub signals: Vec<roko_core::Signal>,
}

/// Writes Activity node outputs to a JSONL file for later replay.
///
/// Each successful Activity execution is appended as one JSON line.
/// The file is flushed after every write so partial runs are recoverable.
pub struct ActivityRecorder {
    run_id: String,
    path: PathBuf,
    writer: BufWriter<File>,
}

impl ActivityRecorder {
    /// Open (or create) a JSONL file at `path` and prepare it for recording.
    ///
    /// # Errors
    /// Returns an `std::io::Error` if the file cannot be opened or created.
    pub fn create(run_id: impl Into<String>, path: impl AsRef<Path>) -> std::io::Result<Self> {
        let path = path.as_ref().to_path_buf();
        let file = OpenOptions::new().create(true).append(true).open(&path)?;
        Ok(Self {
            run_id: run_id.into(),
            path,
            writer: BufWriter::new(file),
        })
    }

    /// Create a new JSONL recording, truncating any file already at `path`.
    ///
    /// Callers should use this for a new run and [`Self::create`] only when
    /// appending to a validated checkpoint from the same run.
    pub fn create_fresh(
        run_id: impl Into<String>,
        path: impl AsRef<Path>,
    ) -> std::io::Result<Self> {
        let path = path.as_ref().to_path_buf();
        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&path)?;
        Ok(Self {
            run_id: run_id.into(),
            path,
            writer: BufWriter::new(file),
        })
    }

    /// Return the path of the underlying JSONL file.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Return the run ID this recorder was created with.
    #[must_use]
    pub fn run_id(&self) -> &str {
        &self.run_id
    }

    /// Append a completed Activity node execution to the JSONL file.
    ///
    /// The write is followed by an immediate flush so the record is durable
    /// even if the process is interrupted mid-run.
    ///
    /// # Errors
    /// Returns an `std::io::Error` if the write or flush fails.
    pub fn record(
        &mut self,
        graph_id: &str,
        node_id: &str,
        tick: u64,
        signals: Vec<roko_core::Signal>,
    ) -> std::io::Result<()> {
        let entry = RecordEntry {
            graph_id: graph_id.to_string(),
            run_id: self.run_id.clone(),
            node_id: node_id.to_string(),
            tick,
            signals,
        };
        let line = serde_json::to_string(&entry)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        self.writer.write_all(line.as_bytes())?;
        self.writer.write_all(b"\n")?;
        self.writer.flush()?;
        self.writer.get_ref().sync_data()
    }
}

/// Reads previously recorded Activity outputs and substitutes them during
/// replay instead of re-executing the Activity cell.
///
/// The key is `(node_id, tick)` — graph_id and run_id are stored in the entry
/// but are not used for lookup (the replayer is already scoped to one file).
pub struct ActivityReplayer {
    /// Map from (node_id, tick) to the recorded output signals.
    entries: HashMap<(String, u64), Vec<roko_core::Signal>>,
}

impl ActivityReplayer {
    /// Load a JSONL recording file produced by [`ActivityRecorder`].
    ///
    /// Lines that fail to parse are silently skipped with a `tracing::warn`
    /// rather than aborting the load, so partial recordings remain usable.
    ///
    /// # Errors
    /// Returns an `std::io::Error` if the file cannot be opened.
    pub fn load(path: impl AsRef<Path>) -> std::io::Result<Self> {
        Self::load_inner(path.as_ref(), None)
    }

    /// Load a recording and reject malformed entries or entries from another
    /// graph or run.
    ///
    /// This is the fail-closed entry point for durable runtime checkpoints.
    pub fn load_scoped(
        path: impl AsRef<Path>,
        expected_graph_id: &str,
        expected_run_id: &str,
    ) -> std::io::Result<Self> {
        Self::load_inner(path.as_ref(), Some((expected_graph_id, expected_run_id)))
    }

    fn load_inner(path: &Path, expected: Option<(&str, &str)>) -> std::io::Result<Self> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let mut entries: HashMap<(String, u64), Vec<roko_core::Signal>> = HashMap::new();

        for (line_num, line) in reader.lines().enumerate() {
            let line = line?;
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            match serde_json::from_str::<RecordEntry>(trimmed) {
                Ok(entry) => {
                    if let Some((expected_graph_id, expected_run_id)) = expected
                        && (entry.graph_id != expected_graph_id || entry.run_id != expected_run_id)
                    {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            format!(
                                "record at line {} belongs to graph/run '{}/{}', expected '{}/{}'",
                                line_num + 1,
                                entry.graph_id,
                                entry.run_id,
                                expected_graph_id,
                                expected_run_id
                            ),
                        ));
                    }
                    let key = (entry.node_id, entry.tick);
                    if entries.insert(key.clone(), entry.signals).is_some() && expected.is_some() {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            format!(
                                "duplicate Activity record at line {} for node '{}' tick {}",
                                line_num + 1,
                                key.0,
                                key.1
                            ),
                        ));
                    }
                }
                Err(e) => {
                    if expected.is_some() {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            format!("unparseable Activity record at line {}: {e}", line_num + 1),
                        ));
                    }
                    tracing::warn!(
                        line = line_num + 1,
                        error = %e,
                        "replay: skipping unparseable JSONL line"
                    );
                }
            }
        }

        Ok(Self { entries })
    }

    /// Look up the recorded outputs for the given node at the given tick.
    ///
    /// Returns `None` if no matching entry exists, which signals the engine to
    /// fall back to live execution for this node.
    #[must_use]
    pub fn lookup(&self, node_id: &str, tick: u64) -> Option<&Vec<roko_core::Signal>> {
        self.entries.get(&(node_id.to_string(), tick))
    }

    /// Return the total number of recorded entries loaded.
    #[must_use]
    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }

    /// Validate that every record belongs to a current Activity node and does
    /// not come from a tick later than the interrupted checkpoint tick.
    ///
    /// # Errors
    /// Returns `InvalidData` for an unknown node or impossible future tick.
    pub fn validate_checkpoint(
        &self,
        activity_node_ids: &HashSet<String>,
        interrupted_tick: u64,
    ) -> std::io::Result<()> {
        for (node_id, tick) in self.entries.keys() {
            if !activity_node_ids.contains(node_id) {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!(
                        "Activity checkpoint contains unknown or non-Activity node '{node_id}'"
                    ),
                ));
            }
            if *tick > interrupted_tick {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!(
                        "Activity checkpoint contains future tick {tick} after checkpoint tick {interrupted_tick}"
                    ),
                ));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use roko_core::{Body, Kind, Signal};
    use tempfile::NamedTempFile;

    fn make_signal(text: &str) -> Signal {
        Signal::builder(Kind::Task).body(Body::text(text)).build()
    }

    #[test]
    fn record_and_replay_roundtrip() {
        let tmp = NamedTempFile::new().unwrap();
        let path = tmp.path().to_path_buf();

        // Record two entries.
        let mut rec = ActivityRecorder::create("run-1", &path).unwrap();
        rec.record("my-graph", "node-a", 0, vec![make_signal("hello")])
            .unwrap();
        rec.record("my-graph", "node-b", 0, vec![make_signal("world")])
            .unwrap();
        drop(rec);

        // Replay and look up.
        let rep = ActivityReplayer::load(&path).unwrap();
        assert_eq!(rep.entry_count(), 2);

        let found = rep.lookup("node-a", 0).unwrap();
        assert_eq!(found.len(), 1);

        let found_b = rep.lookup("node-b", 0).unwrap();
        assert_eq!(found_b.len(), 1);

        // Unknown node returns None.
        assert!(rep.lookup("node-c", 0).is_none());
    }

    #[test]
    fn lookup_miss_returns_none() {
        let tmp = NamedTempFile::new().unwrap();
        let mut rec = ActivityRecorder::create("run-x", tmp.path()).unwrap();
        rec.record("g", "n", 0, vec![]).unwrap();
        drop(rec);

        let rep = ActivityReplayer::load(tmp.path()).unwrap();
        // Wrong tick
        assert!(rep.lookup("n", 1).is_none());
        // Wrong node
        assert!(rep.lookup("m", 0).is_none());
    }

    #[test]
    fn empty_file_loads_cleanly() {
        let tmp = NamedTempFile::new().unwrap();
        let rep = ActivityReplayer::load(tmp.path()).unwrap();
        assert_eq!(rep.entry_count(), 0);
    }

    #[test]
    fn record_preserves_run_id() {
        let tmp = NamedTempFile::new().unwrap();
        let rec = ActivityRecorder::create("my-run-id", tmp.path()).unwrap();
        assert_eq!(rec.run_id(), "my-run-id");
    }

    #[test]
    fn create_fresh_truncates_a_previous_run() {
        let tmp = NamedTempFile::new().unwrap();
        let mut old = ActivityRecorder::create("old", tmp.path()).unwrap();
        old.record("g", "old-node", 0, vec![]).unwrap();
        drop(old);

        let mut fresh = ActivityRecorder::create_fresh("new", tmp.path()).unwrap();
        fresh.record("g", "new-node", 0, vec![]).unwrap();
        drop(fresh);

        let rep = ActivityReplayer::load_scoped(tmp.path(), "g", "new").unwrap();
        assert_eq!(rep.entry_count(), 1);
        assert!(rep.lookup("old-node", 0).is_none());
        assert!(rep.lookup("new-node", 0).is_some());
    }

    #[test]
    fn scoped_load_rejects_a_different_run() {
        let tmp = NamedTempFile::new().unwrap();
        let mut rec = ActivityRecorder::create_fresh("run-a", tmp.path()).unwrap();
        rec.record("g", "node", 0, vec![]).unwrap();
        drop(rec);

        let error = ActivityReplayer::load_scoped(tmp.path(), "g", "run-b")
            .err()
            .expect("run mismatch must fail");
        assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
    }

    #[test]
    fn scoped_load_rejects_a_corrupt_record() {
        let tmp = NamedTempFile::new().unwrap();
        std::fs::write(tmp.path(), b"not-json\n").unwrap();

        let error = ActivityReplayer::load_scoped(tmp.path(), "g", "run")
            .err()
            .expect("corrupt checkpoint must fail closed");
        assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
    }
}
