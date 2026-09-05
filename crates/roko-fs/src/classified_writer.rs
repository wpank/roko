//! Writer that drains [`TranscriptRecord`]s from a [`TranscriptStore`],
//! classifies them via [`ClassifiedRecord`], and appends to a JSONL file.
//!
//! This is the production persistence bridge between the bounded in-memory
//! store (roko-core) and the classified-persistence layer (roko-fs).
//!
//! # Usage
//!
//! ```ignore
//! let writer = ClassifiedTranscriptWriter::open(path)?;
//! // During the run, periodically flush from the store:
//! writer.flush_from_store(&store);
//! // At the end of the run, flush remaining + fsync:
//! writer.finalize_from_store(&store)?;
//! ```
//!
//! # Thread safety
//!
//! The writer synchronizes through an internal `Mutex` on the file handle.
//! Multiple callers can call `flush_from_store` concurrently; writes are
//! serialized.

use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use parking_lot::Mutex;

use roko_core::tool::transcript::TranscriptRecord;
use roko_core::transcript_store::TranscriptStore;

use crate::classified_persistence::ClassifiedRecord;

/// Persists classified transcript records to an append-only JSONL file.
pub struct ClassifiedTranscriptWriter {
    path: PathBuf,
    inner: Mutex<BufWriter<std::fs::File>>,
    /// Highest sequence number already flushed.
    flushed_through: AtomicU64,
    /// Total records written.
    records_written: AtomicU64,
    /// Total write errors encountered.
    write_errors: AtomicU64,
}

/// Statistics from the writer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WriterStats {
    /// Total classified records written to disk.
    pub records_written: u64,
    /// Highest transcript sequence number flushed so far.
    pub flushed_through: u64,
    /// Number of write errors encountered.
    pub write_errors: u64,
}

impl ClassifiedTranscriptWriter {
    /// Open (or create) a classified transcript JSONL file for appending.
    pub fn open(path: impl AsRef<Path>) -> io::Result<Self> {
        let path = path.as_ref().to_path_buf();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)?;
        Ok(Self {
            path,
            inner: Mutex::new(BufWriter::new(file)),
            flushed_through: AtomicU64::new(0),
            records_written: AtomicU64::new(0),
            write_errors: AtomicU64::new(0),
        })
    }

    /// Drain records from the store that have not been flushed yet,
    /// classify each, and append to the JSONL file.
    ///
    /// Returns the number of records written in this flush.
    pub fn flush_from_store(&self, store: &TranscriptStore) -> usize {
        let cursor = self.flushed_through.load(Ordering::Relaxed);
        let records = store.replay(cursor + 1);
        if records.is_empty() {
            return 0;
        }
        self.write_classified(&records)
    }

    /// Final flush: drain remaining records, write, and fsync.
    pub fn finalize_from_store(&self, store: &TranscriptStore) -> io::Result<usize> {
        let count = self.flush_from_store(store);
        let mut writer = self.inner.lock();
        writer.flush()?;
        writer.get_ref().sync_all()?;
        Ok(count)
    }

    /// Write pre-classified records directly (useful for testing or
    /// external callers that already hold `ClassifiedRecord`s).
    pub fn write_classified_records(&self, records: &[ClassifiedRecord]) -> usize {
        let mut writer = self.inner.lock();
        let mut written = 0;
        for cr in records {
            match serde_json::to_string(cr) {
                Ok(line) => {
                    if writeln!(writer, "{line}").is_ok() {
                        written += 1;
                    } else {
                        self.write_errors.fetch_add(1, Ordering::Relaxed);
                    }
                }
                Err(_) => {
                    self.write_errors.fetch_add(1, Ordering::Relaxed);
                }
            }
        }
        self.records_written
            .fetch_add(written as u64, Ordering::Relaxed);
        written
    }

    /// Writer statistics.
    #[must_use]
    pub fn stats(&self) -> WriterStats {
        WriterStats {
            records_written: self.records_written.load(Ordering::Relaxed),
            flushed_through: self.flushed_through.load(Ordering::Relaxed),
            write_errors: self.write_errors.load(Ordering::Relaxed),
        }
    }

    /// Path to the JSONL file.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    // ── private ─────────────────────────────────────────────────────────

    fn write_classified(&self, records: &[TranscriptRecord]) -> usize {
        let mut writer = self.inner.lock();
        let mut written = 0;
        let mut max_seq = self.flushed_through.load(Ordering::Relaxed);

        for record in records {
            let classified = ClassifiedRecord::classify(record.clone());
            match serde_json::to_string(&classified) {
                Ok(line) => {
                    if writeln!(writer, "{line}").is_ok() {
                        written += 1;
                        if record.meta.sequence > max_seq {
                            max_seq = record.meta.sequence;
                        }
                    } else {
                        self.write_errors.fetch_add(1, Ordering::Relaxed);
                    }
                }
                Err(_) => {
                    self.write_errors.fetch_add(1, Ordering::Relaxed);
                }
            }
        }

        if max_seq > 0 {
            self.flushed_through.store(max_seq, Ordering::Relaxed);
        }
        self.records_written
            .fetch_add(written as u64, Ordering::Relaxed);

        // Best-effort flush (don't fail on buffered writes).
        let _ = writer.flush();
        written
    }
}

impl std::fmt::Debug for ClassifiedTranscriptWriter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClassifiedTranscriptWriter")
            .field("path", &self.path)
            .field("inner", &"<Mutex<BufWriter<File>>>")
            .field(
                "flushed_through",
                &self.flushed_through.load(Ordering::Relaxed),
            )
            .field(
                "records_written",
                &self.records_written.load(Ordering::Relaxed),
            )
            .field("write_errors", &self.write_errors.load(Ordering::Relaxed))
            .finish()
    }
}

/// Read classified records back from a JSONL file.
///
/// Skips malformed lines (forward compatibility). Returns records in
/// file order.
pub fn read_classified_jsonl(path: impl AsRef<Path>) -> io::Result<Vec<ClassifiedRecord>> {
    let content = std::fs::read_to_string(path)?;
    Ok(content
        .lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect())
}

// ─── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use roko_core::tool::transcript::{TranscriptEvent, TranscriptEventMeta};

    fn make_meta(seq: u64) -> TranscriptEventMeta {
        TranscriptEventMeta {
            run_id: "run-1".into(),
            turn_id: 0,
            agent_id: "agent-1".into(),
            sequence: seq,
            timestamp_ms: 1_700_000_000_000 + (seq as i64),
            provider: "test".into(),
            model: "test-model".into(),
            parent_event_id: None,
            task_id: None,
            attempt_id: None,
        }
    }

    fn delta_record(seq: u64, text: &str) -> TranscriptRecord {
        TranscriptRecord {
            schema_version: TranscriptRecord::CURRENT_SCHEMA_VERSION,
            meta: make_meta(seq),
            event: TranscriptEvent::AssistantDelta { text: text.into() },
        }
    }

    fn control_record(seq: u64) -> TranscriptRecord {
        TranscriptRecord {
            schema_version: TranscriptRecord::CURRENT_SCHEMA_VERSION,
            meta: make_meta(seq),
            event: TranscriptEvent::RunFinished {
                success: true,
                total_turns: 1,
                total_tool_calls: 0,
                wall_ms: 100,
            },
        }
    }

    #[test]
    fn flush_from_store_writes_classified_jsonl() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("transcript.jsonl");
        let writer = ClassifiedTranscriptWriter::open(&path).unwrap();
        let store = TranscriptStore::new(100);

        store.append(delta_record(1, "hello")).unwrap();
        store.append(control_record(2)).unwrap();

        let written = writer.flush_from_store(&store);
        assert_eq!(written, 2);

        // Read back and verify.
        let records = read_classified_jsonl(&path).unwrap();
        assert_eq!(records.len(), 2);
        assert_eq!(records[0].record.meta.sequence, 1);
        assert_eq!(records[1].record.meta.sequence, 2);
    }

    #[test]
    fn incremental_flush_only_writes_new_records() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("transcript.jsonl");
        let writer = ClassifiedTranscriptWriter::open(&path).unwrap();
        let store = TranscriptStore::new(100);

        store.append(delta_record(1, "first")).unwrap();
        assert_eq!(writer.flush_from_store(&store), 1);

        store.append(delta_record(2, "second")).unwrap();
        assert_eq!(writer.flush_from_store(&store), 1);

        // Total on disk should be 2.
        let records = read_classified_jsonl(&path).unwrap();
        assert_eq!(records.len(), 2);

        let stats = writer.stats();
        assert_eq!(stats.records_written, 2);
        assert_eq!(stats.flushed_through, 2);
    }

    #[test]
    fn finalize_fsyncs() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("transcript.jsonl");
        let writer = ClassifiedTranscriptWriter::open(&path).unwrap();
        let store = TranscriptStore::new(100);

        store.append(delta_record(1, "data")).unwrap();
        let count = writer.finalize_from_store(&store).unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn secret_canary_redacted_in_persisted_output() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("transcript.jsonl");
        let writer = ClassifiedTranscriptWriter::open(&path).unwrap();
        let store = TranscriptStore::new(100);

        // Insert a record containing a secret pattern.
        store
            .append(delta_record(1, "my key is sk-ant-api-fake-key-12345"))
            .unwrap();
        writer.flush_from_store(&store);

        // Read the raw file and verify the secret is NOT present.
        let raw = std::fs::read_to_string(&path).unwrap();
        assert!(
            !raw.contains("sk-ant-api-fake-key"),
            "secret pattern must not appear in persisted JSONL"
        );
        assert!(
            raw.contains("[REDACTED]"),
            "redaction placeholder must be present"
        );

        // Verify the classified record metadata.
        let records = read_classified_jsonl(&path).unwrap();
        assert_eq!(records.len(), 1);
        assert!(records[0].was_redacted());
        assert_eq!(
            records[0].classification,
            crate::classified_persistence::Classification::Sensitive
        );
    }

    #[test]
    fn live_replay_equivalence_through_persistence() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("transcript.jsonl");
        let writer = ClassifiedTranscriptWriter::open(&path).unwrap();
        let store = TranscriptStore::new(100);

        // Simulate a live session.
        let live_records = vec![
            delta_record(1, "thinking..."),
            delta_record(2, "The answer is 42."),
            control_record(3),
        ];
        for r in &live_records {
            store.append(r.clone()).unwrap();
        }
        writer.finalize_from_store(&store).unwrap();

        // Read back persisted records.
        let persisted = read_classified_jsonl(&path).unwrap();

        // Verify sequence order matches live order.
        let live_seqs: Vec<u64> = live_records.iter().map(|r| r.meta.sequence).collect();
        let persisted_seqs: Vec<u64> = persisted.iter().map(|r| r.record.meta.sequence).collect();
        assert_eq!(live_seqs, persisted_seqs, "live=replay sequence mismatch");

        // Verify event types match.
        let live_types: Vec<&str> = live_records
            .iter()
            .map(|r| event_type_name(&r.event))
            .collect();
        let persisted_types: Vec<&str> = persisted
            .iter()
            .map(|r| event_type_name(&r.record.event))
            .collect();
        assert_eq!(
            live_types, persisted_types,
            "live=replay event type mismatch"
        );
    }

    #[test]
    fn eviction_preserves_control_events_in_persistence() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("transcript.jsonl");
        let writer = ClassifiedTranscriptWriter::open(&path).unwrap();

        // Small store: capacity 3.
        let store = TranscriptStore::new(3);

        // Fill: 2 deltas + 1 control.
        store.append(delta_record(1, "delta1")).unwrap();
        store.append(delta_record(2, "delta2")).unwrap();
        store.append(control_record(3)).unwrap();

        // Flush all 3.
        writer.flush_from_store(&store);

        // Add 2 more deltas (triggers eviction of oldest delta).
        store.append(delta_record(4, "delta3")).unwrap();
        store.append(delta_record(5, "delta4")).unwrap();

        // Flush again (only new records).
        writer.flush_from_store(&store);

        // The persisted file should have all 5 records (even though the
        // in-memory store evicted some). Persistence captures records
        // as they arrive, before eviction.
        let persisted = read_classified_jsonl(&path).unwrap();

        // The control event (seq=3) must be present in persisted output.
        assert!(
            persisted.iter().any(|r| r.record.meta.sequence == 3),
            "control event must survive in persistence"
        );
    }

    #[test]
    fn multiple_secret_patterns_all_redacted() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("transcript.jsonl");
        let writer = ClassifiedTranscriptWriter::open(&path).unwrap();
        let store = TranscriptStore::new(100);

        let secrets = [
            "token: ghp_1234567890abcdef",
            "key: sk-proj-abc123xyz",
            "aws: AKIA1234567890ABCDEF",
            "slack: xoxb-123-456-abc",
        ];
        for (i, secret) in secrets.iter().enumerate() {
            store.append(delta_record((i + 1) as u64, secret)).unwrap();
        }
        writer.flush_from_store(&store);

        let raw = std::fs::read_to_string(&path).unwrap();
        for pattern in &["ghp_", "sk-proj", "AKIA", "xoxb-"] {
            assert!(
                !raw.contains(pattern),
                "secret pattern {pattern} must not appear in persisted output"
            );
        }
    }

    #[test]
    fn read_classified_jsonl_skips_malformed() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("transcript.jsonl");

        // Write one valid record and one garbage line.
        let store = TranscriptStore::new(100);
        store.append(delta_record(1, "valid")).unwrap();
        let writer = ClassifiedTranscriptWriter::open(&path).unwrap();
        writer.flush_from_store(&store);

        // Append garbage.
        let mut f = std::fs::OpenOptions::new()
            .append(true)
            .open(&path)
            .unwrap();
        use std::io::Write;
        writeln!(f, "{{not valid json").unwrap();
        writeln!(f, "").unwrap(); // empty line

        let records = read_classified_jsonl(&path).unwrap();
        assert_eq!(records.len(), 1, "should skip malformed lines");
    }

    fn event_type_name(event: &TranscriptEvent) -> &'static str {
        match event {
            TranscriptEvent::RunStarted { .. } => "run_started",
            TranscriptEvent::AssistantDelta { .. } => "assistant_delta",
            TranscriptEvent::ReasoningDelta { .. } => "reasoning_delta",
            TranscriptEvent::ToolStarted { .. } => "tool_started",
            TranscriptEvent::ToolOutputDelta { .. } => "tool_output_delta",
            TranscriptEvent::ToolFinished { .. } => "tool_finished",
            TranscriptEvent::TodoSnapshot { .. } => "todo_snapshot",
            TranscriptEvent::SubagentStarted { .. } => "subagent_started",
            TranscriptEvent::SubagentUpdate { .. } => "subagent_update",
            TranscriptEvent::SubagentFinished { .. } => "subagent_finished",
            TranscriptEvent::Usage { .. } => "usage",
            TranscriptEvent::ProviderChanged { .. } => "provider_changed",
            TranscriptEvent::Warning { .. } => "warning",
            TranscriptEvent::Error { .. } => "error",
            TranscriptEvent::RunFinished { .. } => "run_finished",
            _ => "unknown",
        }
    }
}
