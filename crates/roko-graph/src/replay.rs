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

use std::collections::{BTreeSet, HashMap, HashSet};
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::cells::task_executor::TaskGateVerdict;
use crate::engine::NodeTiming;

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
    /// When the node became ready: every node it depends on had settled
    /// (milliseconds since the Unix epoch).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ready_at_ms: Option<u64>,
    /// When the node got a slot and its cell started. The time it waited for
    /// a slot is `dispatched_at_ms - ready_at_ms`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dispatched_at_ms: Option<u64>,
}

/// A record of a durable Activity log with the number of its line.
///
/// Lines are numbered from 0. The log is append-only, so a record keeps its
/// line number, and a resume refuses a record by that number instead of
/// rewriting the log (gap-dc1d16).
#[derive(Debug, Clone)]
pub struct LoggedRecord {
    /// Zero-based number of the line that holds the record.
    pub line: u64,
    /// The record.
    pub entry: RecordEntry,
}

/// The records in `log`, the bytes of a durable Activity log, in order.
///
/// Every reader of a checkpoint's log parses it with this function
/// (gap-dc1d16). Blank lines are skipped. A line that is not a record of
/// graph `graph_id` and run `run_id` is an `InvalidData` error: a durable
/// checkpoint never drops a record silently. Pass only the log's committed
/// prefix, since a partial last line is an error like any other.
pub fn activity_records<'a>(
    log: &'a [u8],
    graph_id: &'a str,
    run_id: &'a str,
) -> impl Iterator<Item = std::io::Result<LoggedRecord>> + 'a {
    log.split(|byte| *byte == b'\n')
        .enumerate()
        .filter_map(move |(index, line)| {
            let line = line.trim_ascii();
            if line.is_empty() {
                return None;
            }
            Some(scoped_record(line, index as u64, graph_id, run_id))
        })
}

/// Parse `line`, line `index` of a durable Activity log, as a record of
/// graph `graph_id` and run `run_id`.
fn scoped_record(
    line: &[u8],
    index: u64,
    graph_id: &str,
    run_id: &str,
) -> std::io::Result<LoggedRecord> {
    let entry: RecordEntry = serde_json::from_slice(line).map_err(|error| {
        invalid_data(format!(
            "unparseable Activity record at line {}: {error}",
            index + 1
        ))
    })?;
    if entry.graph_id != graph_id || entry.run_id != run_id {
        return Err(invalid_data(format!(
            "record at line {} belongs to graph/run '{}/{}', expected '{graph_id}/{run_id}'",
            index + 1,
            entry.graph_id,
            entry.run_id,
        )));
    }
    Ok(LoggedRecord { line: index, entry })
}

fn invalid_data(message: String) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::InvalidData, message)
}

/// Writes Activity node outputs to a JSONL file for later replay.
///
/// Each successful Activity execution is appended as one JSON line.
/// The file is flushed after every write so partial runs are recoverable.
pub struct ActivityRecorder {
    run_id: String,
    path: PathBuf,
    writer: BufWriter<File>,
    /// Commits each record once it is durable (see
    /// [`Self::with_commit_hook`]).
    commit_hook: Option<Box<dyn Fn(&[u8]) -> std::io::Result<()> + Send + Sync>>,
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
            commit_hook: None,
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
            commit_hook: None,
        })
    }

    /// Call `hook` with the bytes of each record once they are written and
    /// synced, before [`Self::record`] returns.
    ///
    /// A checkpoint commits the record there (gap-dc1d16), so a record whose
    /// hook did not return is uncommitted. An error from `hook` fails the
    /// record, whose bytes stay in the log.
    #[must_use]
    pub fn with_commit_hook(
        mut self,
        hook: impl Fn(&[u8]) -> std::io::Result<()> + Send + Sync + 'static,
    ) -> Self {
        self.commit_hook = Some(Box::new(hook));
        self
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
    /// even if the process is interrupted mid-run. The process's secrets are
    /// redacted from the record's strings first
    /// ([`roko_core::obs::scrub_secrets_in_jsonl`]), so a replayed output
    /// carries the redaction instead of the secret; signal ids stay as
    /// recorded.
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
        self.record_timed(graph_id, node_id, tick, signals, NodeTiming::default())
    }

    /// Append a completed Activity node execution, with when the node became
    /// ready and when it was dispatched, to the JSONL file.
    ///
    /// # Errors
    /// Returns an `std::io::Error` if the write or flush fails, or the commit
    /// hook fails.
    pub fn record_timed(
        &mut self,
        graph_id: &str,
        node_id: &str,
        tick: u64,
        signals: Vec<roko_core::Signal>,
        timing: NodeTiming,
    ) -> std::io::Result<()> {
        let entry = RecordEntry {
            graph_id: graph_id.to_string(),
            run_id: self.run_id.clone(),
            node_id: node_id.to_string(),
            tick,
            signals,
            ready_at_ms: timing.ready_at_ms,
            dispatched_at_ms: timing.dispatched_at_ms,
        };
        let line = serde_json::to_string(&entry)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        let mut record = roko_core::obs::scrub_secrets_in_jsonl(&line).into_owned();
        record.push('\n');
        self.writer.write_all(record.as_bytes())?;
        self.writer.flush()?;
        self.writer.get_ref().sync_data()?;
        match &self.commit_hook {
            Some(hook) => hook(record.as_bytes()),
            None => Ok(()),
        }
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
    /// Records refused for replay: their gate verdict forbids it (for
    /// example a forced accept), or the caller refused them by line. They are
    /// never substituted, so the node re-executes.
    rejected: Vec<(String, u64)>,
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
        let log = std::fs::read(path)?;
        let records = log
            .split(|byte| *byte == b'\n')
            .enumerate()
            .filter(|(_, line)| !line.trim_ascii().is_empty())
            .filter_map(
                |(index, line)| match serde_json::from_slice::<RecordEntry>(line) {
                    Ok(entry) => Some(LoggedRecord {
                        line: index as u64,
                        entry,
                    }),
                    Err(e) => {
                        tracing::warn!(
                            line = index + 1,
                            error = %e,
                            "replay: skipping unparseable JSONL line"
                        );
                        None
                    }
                },
            );
        Self::build(records, &BTreeSet::new(), false)
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
        Self::load_scoped_committed(path, expected_graph_id, expected_run_id, u64::MAX)
    }

    /// [`Self::load_scoped`] over the first `committed_len` bytes of the
    /// recording only, such as its [`committed_activity_len`]: what a resume
    /// replays once [`set_aside_uncommitted_activities`] has run. It changes
    /// no file.
    ///
    /// # Errors
    /// See [`Self::load_scoped`].
    pub fn load_scoped_committed(
        path: impl AsRef<Path>,
        expected_graph_id: &str,
        expected_run_id: &str,
        committed_len: u64,
    ) -> std::io::Result<Self> {
        let mut log = std::fs::read(path)?;
        log.truncate(usize::try_from(committed_len).unwrap_or(usize::MAX));
        let records = activity_records(&log, expected_graph_id, expected_run_id)
            .collect::<std::io::Result<Vec<_>>>()?;
        Self::from_records(records, &BTreeSet::new())
    }

    /// Build a replayer from a durable log's records, as [`activity_records`]
    /// reads them, leaving out those whose line numbers are in `refused`.
    ///
    /// A refused record stays in the log, so a resume refuses records without
    /// rewriting committed bytes (gap-dc1d16). Like a record whose gate
    /// verdict is not a pass, it is never substituted: its node re-executes,
    /// and the node's fresh record supersedes it.
    ///
    /// # Errors
    /// Returns `InvalidData` for two replayable records of one node and tick.
    pub fn from_records(
        records: impl IntoIterator<Item = LoggedRecord>,
        refused: &BTreeSet<u64>,
    ) -> std::io::Result<Self> {
        Self::build(records, refused, true)
    }

    fn build(
        records: impl IntoIterator<Item = LoggedRecord>,
        refused: &BTreeSet<u64>,
        strict: bool,
    ) -> std::io::Result<Self> {
        let mut entries: HashMap<(String, u64), Vec<roko_core::Signal>> = HashMap::new();
        let mut rejected = Vec::new();
        for LoggedRecord { line, entry } in records {
            let key = (entry.node_id, entry.tick);
            if refused.contains(&line) {
                rejected.push(key);
                continue;
            }
            // A recorded output whose verdict is not a pass (e.g. a forced
            // accept) must never be replayed as a completed node. Skipping it
            // also lets the re-executed node's fresh record supersede it
            // without a duplicate error.
            if TaskGateVerdict::from_signals(&entry.signals)
                .is_some_and(|verdict| !verdict.is_replayable())
            {
                tracing::warn!(
                    node_id = %key.0,
                    tick = key.1,
                    "replay: recorded Activity output is not verified; node will re-run"
                );
                rejected.push(key);
                continue;
            }
            if entries.insert(key.clone(), entry.signals).is_some() && strict {
                return Err(invalid_data(format!(
                    "duplicate Activity record at line {} for node '{}' tick {}",
                    line + 1,
                    key.0,
                    key.1
                )));
            }
        }

        Ok(Self { entries, rejected })
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

    /// `(node_id, tick)` keys of records refused for replay: their gate
    /// verdict is not a pass, or the caller refused them by line.
    #[must_use]
    pub fn rejected_entries(&self) -> &[(String, u64)] {
        &self.rejected
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

/// Length in bytes of the complete records at the start of Activity log `log`.
///
/// That is everything up to and including its last newline.
/// [`ActivityRecorder`] ends each record with a newline, so the bytes after
/// the last one are a record whose write did not finish.
#[must_use]
pub fn complete_records_len(log: &[u8]) -> u64 {
    log.iter()
        .rposition(|byte| *byte == b'\n')
        .map_or(0, |last| last as u64 + 1)
}

/// Length in bytes of the committed part of the Activity log at `path`:
/// everything up to and including its last newline (see
/// [`set_aside_uncommitted_activities`]). A missing log has none.
///
/// # Errors
/// Returns an `std::io::Error` if the log exists but cannot be read.
pub fn committed_activity_len(path: impl AsRef<Path>) -> std::io::Result<u64> {
    match std::fs::read(path) {
        Ok(bytes) => Ok(complete_records_len(&bytes)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(0),
        Err(error) => Err(error),
    }
}

/// Set aside the unfinished end of the Activity log at `path`.
///
/// The bytes after the log's last newline move to
/// `<log>.uncommitted.<unix ms>`, and the log is cut back to its last complete
/// record. Such an end is a record whose write did not finish, for example a
/// line a crash tore: replaying it fails, and appending after it would corrupt
/// the next record. Returns the file that now holds those bytes, or `None`
/// when the log ends in a complete record or is missing.
///
/// # Errors
/// Returns an `std::io::Error` if the log cannot be read or cut back, or the
/// bytes cannot be saved. The log is cut only once they are saved.
pub fn set_aside_uncommitted_activities(
    path: impl AsRef<Path>,
) -> std::io::Result<Option<PathBuf>> {
    let path = path.as_ref();
    match std::fs::read(path) {
        Ok(bytes) => set_aside_after(path, &bytes, complete_records_len(&bytes)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

/// Set aside the bytes of the Activity log at `path` after its first `kept`.
///
/// They move to `<log>.uncommitted.<unix ms>`, and the log is cut back to
/// `kept` bytes, such as the prefix a checkpoint committed (gap-dc1d16).
/// Returns the file that now holds those bytes, or `None` when the log holds
/// no more than `kept` bytes or is missing.
///
/// # Errors
/// Returns an `std::io::Error` if the log cannot be read or cut back, or the
/// bytes cannot be saved. The log is cut only once they are saved.
pub fn set_aside_activities_after(
    path: impl AsRef<Path>,
    kept: u64,
) -> std::io::Result<Option<PathBuf>> {
    let path = path.as_ref();
    match std::fs::read(path) {
        Ok(bytes) => set_aside_after(path, &bytes, kept),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

/// Move `log`, the bytes of the Activity log at `path`, past its first
/// `kept` bytes to a file of their own, and cut the log back to `kept`.
fn set_aside_after(path: &Path, log: &[u8], kept: u64) -> std::io::Result<Option<PathBuf>> {
    let rest = match usize::try_from(kept).ok().and_then(|kept| log.get(kept..)) {
        Some(rest) if !rest.is_empty() => rest,
        _ => return Ok(None),
    };
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let mut aside = path.as_os_str().to_owned();
    aside.push(format!(".uncommitted.{millis}"));
    let aside = PathBuf::from(aside);
    {
        let mut file = File::create_new(&aside)?;
        file.write_all(rest)?;
        file.sync_all()?;
    }
    let file = OpenOptions::new().write(true).open(path)?;
    file.set_len(kept)?;
    file.sync_all()?;
    Ok(Some(aside))
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

    #[test]
    fn a_torn_last_record_is_set_aside_and_never_replayed() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("activities.jsonl");
        let mut rec = ActivityRecorder::create_fresh("run", &path).unwrap();
        rec.record("g", "done", 0, vec![make_signal("a")]).unwrap();
        drop(rec);
        let committed = std::fs::read(&path).unwrap();
        // The process died while it appended the next record.
        let torn = b"{\"graph_id\":\"g\",\"run_id\":\"run\",\"node_id\":\"ne";
        let mut log = OpenOptions::new().append(true).open(&path).unwrap();
        log.write_all(torn).unwrap();
        drop(log);

        assert!(ActivityReplayer::load_scoped(&path, "g", "run").is_err());
        let len = committed_activity_len(&path).unwrap();
        assert_eq!(len, committed.len() as u64);
        let rep = ActivityReplayer::load_scoped_committed(&path, "g", "run", len).unwrap();
        assert_eq!(rep.entry_count(), 1);
        assert_eq!(
            std::fs::read(&path).unwrap().len(),
            committed.len() + torn.len()
        );

        let aside = set_aside_uncommitted_activities(&path)
            .unwrap()
            .expect("the torn record is set aside");
        assert_eq!(std::fs::read(&aside).unwrap(), torn);
        assert_eq!(std::fs::read(&path).unwrap(), committed);
        assert_eq!(set_aside_uncommitted_activities(&path).unwrap(), None);

        // Appends start on a line of their own again.
        let mut rec = ActivityRecorder::create("run", &path).unwrap();
        rec.record("g", "next", 0, vec![make_signal("b")]).unwrap();
        drop(rec);
        let rep = ActivityReplayer::load_scoped(&path, "g", "run").unwrap();
        assert_eq!(rep.entry_count(), 2);
    }

    /// gap-dc1d16: every reader of a durable log parses it the same way.
    #[test]
    fn activity_records_number_every_line_and_fail_closed() {
        let tmp = NamedTempFile::new().unwrap();
        let mut rec = ActivityRecorder::create_fresh("run", tmp.path()).unwrap();
        rec.record("g", "a", 0, vec![make_signal("a")]).unwrap();
        drop(rec);
        // A blank line keeps its number.
        let mut file = OpenOptions::new().append(true).open(tmp.path()).unwrap();
        file.write_all(b"\n").unwrap();
        drop(file);
        let mut rec = ActivityRecorder::create("run", tmp.path()).unwrap();
        rec.record("g", "b", 0, vec![make_signal("b")]).unwrap();
        drop(rec);

        let log = std::fs::read(tmp.path()).unwrap();
        let records: Vec<LoggedRecord> = activity_records(&log, "g", "run")
            .collect::<std::io::Result<_>>()
            .unwrap();
        let lines: Vec<(u64, &str)> = records
            .iter()
            .map(|record| (record.line, record.entry.node_id.as_str()))
            .collect();
        assert_eq!(lines, [(0, "a"), (2, "b")]);

        let foreign = activity_records(&log, "g", "other-run").next().unwrap();
        assert_eq!(foreign.unwrap_err().kind(), std::io::ErrorKind::InvalidData);
        let torn = &log[..log.len() - 2];
        let error = activity_records(torn, "g", "run")
            .nth(1)
            .unwrap()
            .unwrap_err();
        assert!(
            error
                .to_string()
                .contains("unparseable Activity record at line 3")
        );
    }

    /// gap-dc1d16: a resume refuses a record by its line, and the log keeps
    /// it; the node's record from its next run is the one replayed.
    #[test]
    fn records_refused_by_line_stay_in_the_log_and_never_replay() {
        let tmp = NamedTempFile::new().unwrap();
        let mut rec = ActivityRecorder::create_fresh("run", tmp.path()).unwrap();
        rec.record("g", "kept", 0, vec![make_signal("a")]).unwrap();
        rec.record("g", "refused", 0, vec![make_signal("unverified")])
            .unwrap();
        rec.record(
            "g",
            "refused",
            0,
            vec![verdict_signal("fixed", TaskGateVerdict::Passed)],
        )
        .unwrap();
        drop(rec);
        let log = std::fs::read(tmp.path()).unwrap();
        let records = || {
            activity_records(&log, "g", "run")
                .collect::<std::io::Result<Vec<_>>>()
                .unwrap()
        };

        // Without the refusal, the node's two records collide.
        assert!(ActivityReplayer::from_records(records(), &BTreeSet::new()).is_err());
        let rep = ActivityReplayer::from_records(records(), &BTreeSet::from([1])).unwrap();
        assert_eq!(rep.entry_count(), 2);
        assert_eq!(rep.rejected_entries(), &[("refused".to_string(), 0)]);
        let replayed = rep.lookup("refused", 0).expect("the re-run's record");
        assert_eq!(replayed[0].body.as_text().unwrap(), "fixed");
        assert_eq!(std::fs::read(tmp.path()).unwrap(), log);
    }

    /// gap-dc1d16: the commit hook sees exactly the bytes each record added
    /// to the log, after they are synced.
    #[test]
    fn the_commit_hook_sees_each_durable_record() {
        let tmp = NamedTempFile::new().unwrap();
        let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let hook_seen = std::sync::Arc::clone(&seen);
        let mut rec = ActivityRecorder::create_fresh("run", tmp.path())
            .unwrap()
            .with_commit_hook(move |record| {
                hook_seen.lock().unwrap().extend_from_slice(record);
                Ok(())
            });
        rec.record("g", "a", 0, vec![make_signal("a")]).unwrap();
        rec.record("g", "b", 0, vec![make_signal("b")]).unwrap();
        drop(rec);
        assert_eq!(*seen.lock().unwrap(), std::fs::read(tmp.path()).unwrap());

        // A failed commit fails the record, whose bytes stay in the log.
        let mut rec = ActivityRecorder::create("run", tmp.path())
            .unwrap()
            .with_commit_hook(|_| Err(std::io::Error::other("commit refused")));
        let error = rec.record("g", "c", 0, Vec::new()).unwrap_err();
        assert_eq!(error.to_string(), "commit refused");
        let log = std::fs::read(tmp.path()).unwrap();
        assert_eq!(activity_records(&log, "g", "run").count(), 3);
    }

    #[test]
    fn set_aside_after_keeps_exactly_the_committed_prefix() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("activities.jsonl");
        let mut rec = ActivityRecorder::create_fresh("run", &path).unwrap();
        rec.record("g", "committed", 0, Vec::new()).unwrap();
        let committed = std::fs::read(&path).unwrap();
        rec.record("g", "uncommitted", 0, Vec::new()).unwrap();
        drop(rec);
        let log = std::fs::read(&path).unwrap();
        let kept = committed.len() as u64;

        let aside = set_aside_activities_after(&path, kept)
            .unwrap()
            .expect("the uncommitted record is set aside");
        assert_eq!(std::fs::read(&aside).unwrap(), &log[committed.len()..]);
        assert_eq!(std::fs::read(&path).unwrap(), committed);
        assert_eq!(set_aside_activities_after(&path, kept).unwrap(), None);
        assert_eq!(complete_records_len(&committed), kept);
    }

    fn verdict_signal(text: &str, verdict: TaskGateVerdict) -> Signal {
        let mut signals = vec![make_signal(text)];
        verdict.stamp(&mut signals);
        signals.remove(0)
    }

    #[test]
    fn forced_accept_record_is_never_replayed_and_can_be_superseded() {
        let tmp = NamedTempFile::new().unwrap();
        let mut rec = ActivityRecorder::create_fresh("run", tmp.path()).unwrap();
        rec.record(
            "g",
            "forced",
            0,
            vec![verdict_signal("BLOCK", TaskGateVerdict::ForcedAccept)],
        )
        .unwrap();
        rec.record(
            "g",
            "verified",
            0,
            vec![verdict_signal("ok", TaskGateVerdict::Passed)],
        )
        .unwrap();
        // The re-executed node appends a fresh, verified record.
        rec.record(
            "g",
            "forced",
            0,
            vec![verdict_signal("fixed", TaskGateVerdict::Passed)],
        )
        .unwrap();
        drop(rec);

        let rep = ActivityReplayer::load_scoped(tmp.path(), "g", "run").unwrap();
        assert_eq!(rep.entry_count(), 2);
        assert_eq!(rep.rejected_entries(), &[("forced".to_string(), 0)]);
        let replayed = rep.lookup("forced", 0).expect("superseding record");
        assert_eq!(replayed[0].body.as_text().unwrap(), "fixed");
    }

    #[test]
    fn forced_accept_record_alone_forces_re_execution() {
        let tmp = NamedTempFile::new().unwrap();
        let mut rec = ActivityRecorder::create_fresh("run", tmp.path()).unwrap();
        rec.record(
            "g",
            "t01",
            0,
            vec![verdict_signal("BLOCK", TaskGateVerdict::ForcedAccept)],
        )
        .unwrap();
        drop(rec);

        let rep = ActivityReplayer::load_scoped(tmp.path(), "g", "run").unwrap();
        assert!(rep.lookup("t01", 0).is_none());
        assert_eq!(rep.entry_count(), 0);
    }

    /// gap-3006e9: `record_timed` writes when the node became ready and was
    /// dispatched. `record` writes neither time, like records from before
    /// they existed, and both kinds still load.
    #[test]
    fn records_carry_the_node_timing_and_untimed_records_still_load() {
        let tmp = NamedTempFile::new().unwrap();
        let mut rec = ActivityRecorder::create_fresh("run", tmp.path()).unwrap();
        let timing = NodeTiming {
            ready_at_ms: Some(1_000),
            dispatched_at_ms: Some(1_250),
        };
        rec.record_timed("g", "new", 0, vec![make_signal("a")], timing)
            .unwrap();
        rec.record("g", "old", 0, vec![make_signal("b")]).unwrap();
        drop(rec);

        let written = std::fs::read_to_string(tmp.path()).unwrap();
        let entries: Vec<RecordEntry> = written
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        assert_eq!(entries[0].ready_at_ms, Some(1_000));
        assert_eq!(entries[0].dispatched_at_ms, Some(1_250));
        assert_eq!(entries[1].ready_at_ms, None);
        assert_eq!(entries[1].dispatched_at_ms, None);
        assert!(!written.lines().nth(1).unwrap().contains("ready_at_ms"));

        let rep = ActivityReplayer::load_scoped(tmp.path(), "g", "run").unwrap();
        assert_eq!(rep.entry_count(), 2);
    }
}
