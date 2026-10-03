//! The per-run telemetry writer (S01 §4.7) and durable attempt ordinals
//! (S01 §4.2).
//!
//! One [`TelemetryWriter`] per run owns the run directory's append-only
//! files. Producers hand it records through a bounded channel and never
//! wait: when the channel is full the record is dropped and counted, so
//! telemetry never blocks dispatch. A worker thread stamps each line with the
//! run's next `seq`, skips records whose `record_id` is already written, and
//! appends through `roko_fs::log_rotation::append_jsonl_line_sync`. A writer
//! reopened on a run resumes `seq` and the written ids from its files.

use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread::JoinHandle;

use parking_lot::Mutex;
use roko_fs::layout::RokoLayout;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

use super::records::{
    ATTEMPT_OPEN_SCHEMA, AttemptKey, AttemptOpenRecord, AttemptVerdictRecord,
    ContentDecisionRecord, ExposureRecord, PlaceboDecisionRecord, RunFile, Stamped,
    TelemetryRecord, chain_key,
};
use crate::error::LearnError;
use crate::routing_log::RoutingDecisionLog;

/// Default capacity of a writer's channel (`channel_capacity`, S01 §5.9).
pub const DEFAULT_CHANNEL_CAPACITY: usize = 4096;

/// Per-run files never rotate: a run directory is one provenance bundle,
/// and ordinal recovery reads only the live file.
const RUN_FILE_MAX_MB: u64 = u64::MAX;

/// A record queued for a run's writer.
#[derive(Debug, Clone)]
pub enum TelemetryEvent {
    /// A `roko.attempt_open/1` line.
    AttemptOpen(Box<AttemptOpenRecord>),
    /// A `roko.verdict/1` line.
    Verdict(Box<AttemptVerdictRecord>),
    /// A `roko.decision/1` route decision.
    Decision(Box<RoutingDecisionLog>),
    /// A `roko.decision/1` content decision.
    ContentDecision(Box<ContentDecisionRecord>),
    /// A `roko.decision/1` placebo decision.
    PlaceboDecision(Box<PlaceboDecisionRecord>),
    /// A `roko.exposure/1` line.
    Exposure(Box<ExposureRecord>),
}

impl From<AttemptOpenRecord> for TelemetryEvent {
    fn from(record: AttemptOpenRecord) -> Self {
        Self::AttemptOpen(Box::new(record))
    }
}

impl From<AttemptVerdictRecord> for TelemetryEvent {
    fn from(record: AttemptVerdictRecord) -> Self {
        Self::Verdict(Box::new(record))
    }
}

impl From<RoutingDecisionLog> for TelemetryEvent {
    fn from(record: RoutingDecisionLog) -> Self {
        Self::Decision(Box::new(record))
    }
}

impl From<ContentDecisionRecord> for TelemetryEvent {
    fn from(record: ContentDecisionRecord) -> Self {
        Self::ContentDecision(Box::new(record))
    }
}

impl From<PlaceboDecisionRecord> for TelemetryEvent {
    fn from(record: PlaceboDecisionRecord) -> Self {
        Self::PlaceboDecision(Box::new(record))
    }
}

impl From<ExposureRecord> for TelemetryEvent {
    fn from(record: ExposureRecord) -> Self {
        Self::Exposure(Box::new(record))
    }
}

/// Writer settings (`[telemetry]`, S01 §5.9).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TelemetryWriterConfig {
    /// Records the channel holds before new ones are dropped.
    pub channel_capacity: usize,
}

impl Default for TelemetryWriterConfig {
    fn default() -> Self {
        Self {
            channel_capacity: DEFAULT_CHANNEL_CAPACITY,
        }
    }
}

/// A writer's counters. `dropped` feeds `manifest.closed.telemetry_dropped`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TelemetryWriterStats {
    /// Lines appended.
    pub written: u64,
    /// Records skipped because their `record_id` was already written.
    pub duplicates: u64,
    /// Records dropped because the channel was full or closed.
    pub dropped: u64,
    /// Records that could not be serialized or appended.
    pub write_errors: u64,
}

#[derive(Debug, Default)]
struct Counters {
    written: AtomicU64,
    duplicates: AtomicU64,
    dropped: AtomicU64,
    write_errors: AtomicU64,
}

impl Counters {
    fn snapshot(&self) -> TelemetryWriterStats {
        TelemetryWriterStats {
            written: self.written.load(Ordering::Relaxed),
            duplicates: self.duplicates.load(Ordering::Relaxed),
            dropped: self.dropped.load(Ordering::Relaxed),
            write_errors: self.write_errors.load(Ordering::Relaxed),
        }
    }
}

/// Appends one run's telemetry lines from a worker thread (S01 §4.7).
///
/// Dropping the writer closes its channel; the worker then writes what is
/// queued and exits. [`Self::close`] does the same and waits for it.
#[derive(Debug)]
pub struct TelemetryWriter {
    sender: mpsc::Sender<TelemetryEvent>,
    counters: Arc<Counters>,
    worker: Option<JoinHandle<()>>,
}

impl TelemetryWriter {
    /// Start the writer for the run directory `run_dir`.
    ///
    /// # Errors
    ///
    /// Returns an error when the run's existing files cannot be read or the
    /// worker thread cannot start.
    pub fn spawn(
        run_dir: impl Into<PathBuf>,
        config: TelemetryWriterConfig,
    ) -> Result<Self, LearnError> {
        let (mut writer, worker) = Self::unstarted(run_dir.into(), config)?;
        writer.worker = Some(worker.start()?);
        Ok(writer)
    }

    /// Start the writer for `run_id` in `layout`, i.e. `.roko/runs/<run_id>/`.
    ///
    /// # Errors
    ///
    /// See [`Self::spawn`].
    pub fn for_run(
        layout: &RokoLayout,
        run_id: &str,
        config: TelemetryWriterConfig,
    ) -> Result<Self, LearnError> {
        Self::spawn(layout.run_dir(run_id), config)
    }

    /// A writer whose worker has not started, so records stay queued.
    fn unstarted(
        run_dir: PathBuf,
        config: TelemetryWriterConfig,
    ) -> Result<(Self, Worker), LearnError> {
        let (sender, receiver) = mpsc::channel(config.channel_capacity.max(1));
        let counters = Arc::new(Counters::default());
        let worker = Worker::resume(run_dir, receiver, Arc::clone(&counters))?;
        let writer = Self {
            sender,
            counters,
            worker: None,
        };
        Ok((writer, worker))
    }

    /// Queue `record` without blocking. Returns `false`, and counts the
    /// record as dropped, when the channel is full or closed.
    pub fn submit(&self, record: impl Into<TelemetryEvent>) -> bool {
        let queued = self.sender.try_send(record.into()).is_ok();
        if !queued {
            self.counters.dropped.fetch_add(1, Ordering::Relaxed);
        }
        queued
    }

    /// The counters so far.
    #[must_use]
    pub fn stats(&self) -> TelemetryWriterStats {
        self.counters.snapshot()
    }

    /// Close the channel, wait until every queued record is written, and
    /// return the final counters.
    pub fn close(mut self) -> TelemetryWriterStats {
        let worker = self.worker.take();
        let counters = Arc::clone(&self.counters);
        drop(self);
        let panicked = worker.is_some_and(|worker| worker.join().is_err());
        if panicked {
            tracing::warn!("telemetry writer thread panicked");
        }
        counters.snapshot()
    }
}

/// The consuming side of a writer. It owns the run files, the next `seq`
/// and the ids already written.
struct Worker {
    run_dir: PathBuf,
    receiver: mpsc::Receiver<TelemetryEvent>,
    counters: Arc<Counters>,
    next_seq: u64,
    written_ids: HashSet<String>,
}

impl Worker {
    /// Recover the next `seq` and the written ids from the run's files.
    fn resume(
        run_dir: PathBuf,
        receiver: mpsc::Receiver<TelemetryEvent>,
        counters: Arc<Counters>,
    ) -> Result<Self, LearnError> {
        let mut next_seq = 1;
        let mut written_ids = HashSet::new();
        for file in RunFile::ALL {
            for line in read_lines(&file.path_in(&run_dir))? {
                if let Some(seq) = line.seq {
                    next_seq = next_seq.max(seq.saturating_add(1));
                }
                if let Some(record_id) = line.record_id {
                    written_ids.insert(record_id);
                }
            }
        }
        Ok(Self {
            run_dir,
            receiver,
            counters,
            next_seq,
            written_ids,
        })
    }

    fn start(self) -> Result<JoinHandle<()>, LearnError> {
        let path = self.run_dir.display().to_string();
        std::thread::Builder::new()
            .name("roko-telemetry".to_string())
            .spawn(move || self.run())
            .map_err(|source| LearnError::Io { path, source })
    }

    fn run(mut self) {
        while let Some(event) = self.receiver.blocking_recv() {
            match event {
                TelemetryEvent::AttemptOpen(record) => self.write(&*record),
                TelemetryEvent::Verdict(record) => self.write(&*record),
                TelemetryEvent::Decision(record) => self.write(&*record),
                TelemetryEvent::ContentDecision(record) => self.write(&*record),
                TelemetryEvent::PlaceboDecision(record) => self.write(&*record),
                TelemetryEvent::Exposure(record) => self.write(&*record),
            }
        }
    }

    fn write<R: TelemetryRecord>(&mut self, record: &R) {
        let record_id = record.record_id();
        if !self.written_ids.insert(record_id.clone()) {
            self.counters.duplicates.fetch_add(1, Ordering::Relaxed);
            return;
        }
        let line = Stamped {
            schema_version: R::SCHEMA.to_string(),
            record_id,
            seq: self.next_seq,
            ts: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            record,
        };
        self.next_seq += 1;
        let path = R::FILE.path_in(&self.run_dir);
        match append_line(&path, &line) {
            Ok(()) => {
                self.counters.written.fetch_add(1, Ordering::Relaxed);
            }
            Err(error) => {
                self.written_ids.remove(&line.record_id);
                self.counters.write_errors.fetch_add(1, Ordering::Relaxed);
                tracing::warn!(path = %path.display(), %error, "telemetry append failed");
            }
        }
    }
}

/// Serialize `line` and append it to `path`.
fn append_line<T: Serialize>(path: &Path, line: &T) -> io::Result<()> {
    let bytes = serde_json::to_vec(line)?;
    roko_fs::log_rotation::append_jsonl_line_sync(path, &bytes, RUN_FILE_MAX_MB)?;
    Ok(())
}

/// The fields of a run-file line that resuming reads.
#[derive(Debug, Deserialize)]
struct LineProbe {
    #[serde(default)]
    schema_version: Option<String>,
    #[serde(default)]
    record_id: Option<String>,
    #[serde(default)]
    seq: Option<u64>,
    #[serde(default)]
    chain_key: Option<String>,
    #[serde(default)]
    attempt: Option<u32>,
}

/// Read the lines of a run file. A missing file has none, and malformed
/// lines are skipped.
fn read_lines(path: &Path) -> Result<Vec<LineProbe>, LearnError> {
    let io_error = |source: io::Error| LearnError::Io {
        path: path.display().to_string(),
        source,
    };
    let file = match File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(io_error(error)),
    };
    let mut lines = Vec::new();
    for line in BufReader::new(file).lines() {
        let line = line.map_err(io_error)?;
        if let Ok(probe) = serde_json::from_str::<LineProbe>(line.trim()) {
            lines.push(probe);
        }
    }
    Ok(lines)
}

/// The highest attempt ordinal of each chain, recovered from a run's
/// `attempts.jsonl` (S01 §4.2).
///
/// Minting continues past it, so a resumed run never reuses an attempt key,
/// and an attempt that crashed after its open line keeps its ordinal.
#[derive(Debug, Default)]
pub struct AttemptOrdinals {
    highest: Mutex<HashMap<String, u32>>,
}

impl AttemptOrdinals {
    /// Recover the ordinals from the attempt-open lines in `run_dir`.
    ///
    /// # Errors
    ///
    /// Returns an error when `attempts.jsonl` exists but cannot be read.
    pub fn load(run_dir: &Path) -> Result<Self, LearnError> {
        let mut highest = HashMap::new();
        for line in read_lines(&RunFile::Attempts.path_in(run_dir))? {
            if line.schema_version.as_deref() != Some(ATTEMPT_OPEN_SCHEMA) {
                continue;
            }
            if let (Some(chain), Some(attempt)) = (line.chain_key, line.attempt) {
                let ordinal = highest.entry(chain).or_insert(0);
                *ordinal = attempt.max(*ordinal);
            }
        }
        Ok(Self {
            highest: Mutex::new(highest),
        })
    }

    /// The highest ordinal recorded for `chain_key`, or 0 when there is none.
    #[must_use]
    pub fn highest(&self, chain_key: &str) -> u32 {
        self.highest.lock().get(chain_key).copied().unwrap_or(0)
    }

    /// Mint the next attempt key of a chain: one past its highest ordinal.
    pub fn mint(&self, run_id: &str, plan_id: &str, task_id: &str) -> AttemptKey {
        let chain = chain_key(run_id, plan_id, task_id);
        let mut highest = self.highest.lock();
        let ordinal = highest.entry(chain).or_insert(0);
        *ordinal = ordinal.saturating_add(1);
        AttemptKey::new(run_id, plan_id, task_id, *ordinal)
    }
}

#[cfg(test)]
mod tests {
    use super::{AttemptOrdinals, TelemetryWriter, TelemetryWriterConfig, TelemetryWriterStats};
    use crate::routing_log::RoutingDecisionLog;
    use crate::telemetry::records::{
        ATTEMPT_OPEN_SCHEMA, AttemptIdentity, AttemptKey, AttemptOpenRecord, AttemptOutcome,
        AttemptVerdictRecord, DECISION_SCHEMA, RunFile, Stamped, VERDICT_SCHEMA,
    };
    use roko_fs::layout::RokoLayout;
    use tempfile::TempDir;

    const RUN: &str = "gr-7f3c2a91";
    const PLAN: &str = "loop-census";

    /// A route decision row with no attempt key yet.
    const ROUTE_DECISION: &str = r#"{
        "timestamp": "2026-10-02T14:03:11.402Z",
        "trace_id": "trace-t2",
        "task_id": "T2",
        "requested_model": "gpt-oss-120b",
        "role": "implementer",
        "task_complexity": "focused",
        "selected_provider": "cerebras",
        "selected_model": "gpt-oss-120b",
        "routing_stage": "ucb",
        "routing_reason": "highest_ucb_score",
        "candidates": []
    }"#;

    fn key(task: &str, attempt: u32) -> AttemptKey {
        AttemptKey::new(RUN, PLAN, task, attempt)
    }

    fn open(task: &str, attempt: u32) -> AttemptOpenRecord {
        let identity = AttemptIdentity::new(&key(task, attempt));
        AttemptOpenRecord::new(identity, 1_759_413_791_402)
    }

    fn verdict(task: &str, attempt: u32) -> AttemptVerdictRecord {
        let identity = AttemptIdentity::new(&key(task, attempt));
        AttemptVerdictRecord::settle(identity, AttemptOutcome::Passed, true)
    }

    fn run_file_lines(dir: &TempDir, file: RunFile) -> Vec<serde_json::Value> {
        std::fs::read_to_string(file.path_in(dir.path()))
            .expect("read run file")
            .lines()
            .map(|line| serde_json::from_str(line).expect("parse run-file line"))
            .collect()
    }

    #[test]
    fn writer_stamps_seq_and_skips_duplicate_record_ids() {
        let dir = TempDir::new().expect("tempdir");
        let config = TelemetryWriterConfig::default();
        let writer = TelemetryWriter::spawn(dir.path(), config).expect("spawn writer");
        let mut decision: RoutingDecisionLog =
            serde_json::from_str(ROUTE_DECISION).expect("parse decision");
        decision.attempt_key = Some(key("T2", 1).attempt_key());

        assert!(writer.submit(open("T2", 1)));
        assert!(writer.submit(verdict("T2", 1)));
        assert!(writer.submit(verdict("T2", 1)));
        assert!(writer.submit(decision));
        let stats = writer.close();
        let expected = TelemetryWriterStats {
            written: 3,
            duplicates: 1,
            dropped: 0,
            write_errors: 0,
        };
        assert_eq!(stats, expected);

        let attempts = run_file_lines(&dir, RunFile::Attempts);
        assert_eq!(attempts.len(), 2);
        assert_eq!(attempts[0]["schema_version"], ATTEMPT_OPEN_SCHEMA);
        assert_eq!(attempts[0]["seq"], 1);
        assert_eq!(attempts[1]["schema_version"], VERDICT_SCHEMA);
        assert_eq!(attempts[1]["seq"], 2);
        assert_eq!(attempts[1]["attempt_key"], "gr-7f3c2a91:loop-census:T2:1");
        let decisions = run_file_lines(&dir, RunFile::Decisions);
        assert_eq!(decisions.len(), 1);
        assert_eq!(decisions[0]["schema_version"], DECISION_SCHEMA);
        assert_eq!(decisions[0]["seq"], 3);
        assert_eq!(decisions[0]["selected_model"], "gpt-oss-120b");

        let line: Stamped<AttemptVerdictRecord> =
            serde_json::from_value(attempts[1].clone()).expect("parse verdict line");
        assert_eq!(line.record, verdict("T2", 1));
    }

    #[test]
    fn writer_resumes_seq_and_written_ids_after_reopen() {
        let dir = TempDir::new().expect("tempdir");
        let config = TelemetryWriterConfig::default();
        let first = TelemetryWriter::spawn(dir.path(), config).expect("spawn writer");
        assert!(first.submit(open("T2", 1)));
        assert!(first.submit(verdict("T2", 1)));
        assert_eq!(first.close().written, 2);

        let second = TelemetryWriter::spawn(dir.path(), config).expect("respawn writer");
        assert!(second.submit(verdict("T2", 1)));
        assert!(second.submit(open("T2", 2)));
        let stats = second.close();
        assert_eq!((stats.written, stats.duplicates), (1, 1));

        let seqs: Vec<u64> = run_file_lines(&dir, RunFile::Attempts)
            .iter()
            .map(|line| line["seq"].as_u64().expect("seq"))
            .collect();
        assert_eq!(seqs, [1, 2, 3]);
    }

    #[test]
    fn writer_counts_records_dropped_when_the_channel_is_full() {
        let dir = TempDir::new().expect("tempdir");
        let config = TelemetryWriterConfig {
            channel_capacity: 2,
        };
        let run_dir = dir.path().to_path_buf();
        let (mut writer, worker) = TelemetryWriter::unstarted(run_dir, config).expect("writer");
        let queued: Vec<bool> = (1..=5)
            .map(|attempt| writer.submit(open("T1", attempt)))
            .collect();
        assert_eq!(queued, [true, true, false, false, false]);
        assert_eq!(writer.stats().dropped, 3);

        writer.worker = Some(worker.start().expect("start worker"));
        let stats = writer.close();
        assert_eq!((stats.written, stats.dropped), (2, 3));
        assert_eq!(run_file_lines(&dir, RunFile::Attempts).len(), 2);
    }

    #[test]
    fn writer_for_run_writes_into_the_layout_run_dir() {
        let dir = TempDir::new().expect("tempdir");
        let layout = RokoLayout::new(dir.path().join(".roko"));
        let config = TelemetryWriterConfig::default();
        let writer = TelemetryWriter::for_run(&layout, RUN, config).expect("spawn writer");
        assert!(writer.submit(open("T1", 1)));
        assert_eq!(writer.close().written, 1);
        assert!(layout.run_dir(RUN).join("attempts.jsonl").is_file());
    }

    #[test]
    fn attempt_ordinals_continue_from_the_attempts_log() {
        let dir = TempDir::new().expect("tempdir");
        let fresh = AttemptOrdinals::load(dir.path()).expect("load empty run");
        assert_eq!(fresh.mint(RUN, PLAN, "T2"), key("T2", 1));

        let config = TelemetryWriterConfig::default();
        let writer = TelemetryWriter::spawn(dir.path(), config).expect("spawn writer");
        assert!(writer.submit(open("T2", 1)));
        assert!(writer.submit(verdict("T2", 1)));
        // The process dies after this open line: the attempt is abandoned.
        assert!(writer.submit(open("T2", 2)));
        assert_eq!(writer.close().written, 3);

        let resumed = AttemptOrdinals::load(dir.path()).expect("load run");
        assert_eq!(resumed.highest("gr-7f3c2a91:loop-census:T2"), 2);
        assert_eq!(resumed.mint(RUN, PLAN, "T2"), key("T2", 3));
        assert_eq!(resumed.mint(RUN, PLAN, "T2"), key("T2", 4));
        assert_eq!(resumed.mint(RUN, PLAN, "T3"), key("T3", 1));
    }
}
