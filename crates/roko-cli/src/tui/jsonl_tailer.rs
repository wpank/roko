//! Generic incremental JSONL tailer with deserialization.
//!
//! Builds on [`JsonlCursor`] to provide a typed, accumulating reader for
//! append-only JSONL files. The TUI's `DashboardData::tick()` currently
//! re-reads entire files on every tick (O(N) on file size). This module
//! provides infrastructure to replace those hot paths with an incremental
//! reader that only deserializes newly appended lines.
//!
//! # Usage
//!
//! ```ignore
//! use roko_cli::tui::jsonl_tailer::IncrementalTailer;
//!
//! let mut tailer: IncrementalTailer<EfficiencyEvent> =
//!     IncrementalTailer::new(".roko/learn/efficiency.jsonl");
//!
//! // On each TUI tick:
//! let new_count = tailer.tick()?;
//! if new_count > 0 {
//!     // Only process new items.
//!     for item in tailer.items().iter().rev().take(new_count) {
//!         // ...
//!     }
//! }
//! ```

use std::path::PathBuf;

use roko_learn::efficiency::{AgentEfficiencyEvent, EfficiencyRowSchema, classify_efficiency_row};

use super::jsonl_cursor::JsonlCursor;

/// Decides whether a well-formed JSONL row belongs to the tailed record type.
///
/// Files shared by several writers use this to skip the other writers' rows
/// without counting them as parse errors.
pub type RowFilter = fn(&serde_json::Value) -> bool;

/// Accumulating, incremental reader for typed JSONL files.
///
/// Wraps a [`JsonlCursor`] and deserializes each new line into `T`,
/// accumulating all successfully parsed items in an internal `Vec`.
/// Malformed lines are skipped and counted in `parse_errors`; rows rejected
/// by the optional [`RowFilter`] are skipped and counted in `skipped_rows`.
pub struct IncrementalTailer<T> {
    cursor: JsonlCursor,
    items: Vec<T>,
    row_filter: Option<RowFilter>,
    /// Number of lines that failed deserialization (cumulative).
    pub parse_errors: usize,
    /// Number of well-formed rows the row filter skipped (cumulative).
    pub skipped_rows: usize,
}

impl<T: Clone> Clone for IncrementalTailer<T> {
    fn clone(&self) -> Self {
        Self {
            cursor: self.cursor.clone(),
            items: self.items.clone(),
            row_filter: self.row_filter,
            parse_errors: self.parse_errors,
            skipped_rows: self.skipped_rows,
        }
    }
}

impl<T: std::fmt::Debug> std::fmt::Debug for IncrementalTailer<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IncrementalTailer")
            .field("cursor", &self.cursor)
            .field("items_len", &self.items.len())
            .field("parse_errors", &self.parse_errors)
            .field("skipped_rows", &self.skipped_rows)
            .finish()
    }
}

impl<T> Default for IncrementalTailer<T> {
    fn default() -> Self {
        Self {
            cursor: JsonlCursor::default(),
            items: Vec::new(),
            row_filter: None,
            parse_errors: 0,
            skipped_rows: 0,
        }
    }
}

impl<T: serde::de::DeserializeOwned> IncrementalTailer<T> {
    /// Create a new tailer for the given JSONL file.
    ///
    /// No I/O happens until [`tick`](Self::tick) is called.
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            cursor: JsonlCursor::new(path),
            items: Vec::new(),
            row_filter: None,
            parse_errors: 0,
            skipped_rows: 0,
        }
    }

    /// Create a tailer that only deserializes rows `row_filter` accepts.
    ///
    /// Rejected rows are skipped without counting as parse errors.
    pub fn with_row_filter(path: impl Into<PathBuf>, row_filter: RowFilter) -> Self {
        Self {
            row_filter: Some(row_filter),
            ..Self::new(path)
        }
    }

    /// Read and deserialize newly appended lines since the last tick.
    ///
    /// Returns the number of *successfully parsed* new items added.
    /// On file truncation, the cursor resets and previously accumulated
    /// items are cleared so the file is re-read from the beginning.
    pub fn tick(&mut self) -> std::io::Result<usize> {
        let prev_offset = self.cursor.offset();
        let raw_lines = self.cursor.read_new_lines()?;

        // Detect truncation: cursor reset its offset below our previous.
        if self.cursor.offset() < prev_offset && raw_lines.is_empty() {
            // File was truncated but no new lines yet — clear accumulator
            // and wait for the next tick to pick up fresh data.
            tracing::info!(path = %self.cursor.path().display(), "JSONL file truncated — resync from beginning");
            self.items.clear();
            self.parse_errors = 0;
            self.skipped_rows = 0;
            return Ok(0);
        }

        // If the cursor read lines starting from 0 and we had items,
        // it means a truncation happened and the cursor re-read from start.
        if prev_offset > 0 && self.cursor.offset() > 0 && !raw_lines.is_empty() {
            // Check if cursor internally reset (offset moved backward from
            // our perspective). The cursor handles the reset internally;
            // we detect it by checking if new offset < old offset + new bytes.
            let new_bytes: u64 = raw_lines.iter().map(|l| l.len() as u64 + 1).sum();
            if self.cursor.offset() == new_bytes && prev_offset > new_bytes {
                tracing::info!(path = %self.cursor.path().display(), "JSONL file truncated — resync from beginning");
                self.items.clear();
                self.parse_errors = 0;
                self.skipped_rows = 0;
            }
        }

        let mut added = 0;
        for line in &raw_lines {
            if line.is_empty() {
                continue;
            }
            let parsed = match self.row_filter {
                None => serde_json::from_str::<T>(line),
                Some(accepts) => match serde_json::from_str::<serde_json::Value>(line) {
                    Ok(row) if !accepts(&row) => {
                        self.skipped_rows += 1;
                        continue;
                    }
                    Ok(row) => serde_json::from_value::<T>(row),
                    Err(error) => Err(error),
                },
            };
            match parsed {
                Ok(item) => {
                    self.items.push(item);
                    added += 1;
                }
                Err(_e) => {
                    self.parse_errors += 1;
                    tracing::warn!(
                        path = %self.cursor.path().display(),
                        errors = self.parse_errors,
                        "JSONL parse errors detected"
                    );
                }
            }
        }

        Ok(added)
    }

    /// All successfully parsed items accumulated so far.
    pub fn items(&self) -> &[T] {
        &self.items
    }

    /// Number of accumulated items.
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Whether no items have been accumulated.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Current byte offset into the file.
    pub fn offset(&self) -> u64 {
        self.cursor.offset()
    }

    /// Path being tailed (for diagnostics).
    pub fn path(&self) -> &std::path::Path {
        self.cursor.path()
    }
}

/// Tailer over `.roko/learn/efficiency.jsonl`.
///
/// The `FeedbackService` shares that file; its `feedback_event/v1` rows are
/// valid JSON of another shape, so they are skipped rather than reported as
/// parse errors.
pub fn efficiency_tailer(path: impl Into<PathBuf>) -> IncrementalTailer<AgentEfficiencyEvent> {
    IncrementalTailer::with_row_filter(path, |row| {
        classify_efficiency_row(row) != EfficiencyRowSchema::FeedbackEvent
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;
    use std::fs::{self, OpenOptions};
    use std::io::Write;
    use tempfile::tempdir;

    #[derive(Debug, Deserialize, PartialEq)]
    struct TestEvent {
        kind: String,
        value: i64,
    }

    fn append(path: &std::path::Path, text: &str) {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .expect("open for append");
        file.write_all(text.as_bytes()).expect("append bytes");
    }

    #[test]
    fn reads_and_deserializes_incrementally() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("events.jsonl");
        append(&path, r#"{"kind":"a","value":1}"#);
        append(&path, "\n");

        let mut tailer: IncrementalTailer<TestEvent> = IncrementalTailer::new(&path);

        let n = tailer.tick().expect("first tick");
        assert_eq!(n, 1);
        assert_eq!(tailer.len(), 1);
        assert_eq!(tailer.items()[0].kind, "a");

        // Append more.
        append(&path, r#"{"kind":"b","value":2}"#);
        append(&path, "\n");
        append(&path, r#"{"kind":"c","value":3}"#);
        append(&path, "\n");

        let n = tailer.tick().expect("second tick");
        assert_eq!(n, 2);
        assert_eq!(tailer.len(), 3);

        // Idle tick.
        let n = tailer.tick().expect("idle tick");
        assert_eq!(n, 0);
        assert_eq!(tailer.len(), 3);
    }

    #[test]
    fn skips_malformed_lines() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("events.jsonl");
        append(&path, r#"{"kind":"ok","value":1}"#);
        append(&path, "\n");
        append(&path, "NOT VALID JSON\n");
        append(&path, r#"{"kind":"ok2","value":2}"#);
        append(&path, "\n");

        let mut tailer: IncrementalTailer<TestEvent> = IncrementalTailer::new(&path);
        let n = tailer.tick().expect("tick");
        assert_eq!(n, 2);
        assert_eq!(tailer.parse_errors, 1);
        assert_eq!(tailer.items()[0].kind, "ok");
        assert_eq!(tailer.items()[1].kind, "ok2");
    }

    #[test]
    fn handles_missing_file() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("nonexistent.jsonl");

        let mut tailer: IncrementalTailer<TestEvent> = IncrementalTailer::new(&path);
        let n = tailer.tick().expect("missing file tick");
        assert_eq!(n, 0);
        assert!(tailer.is_empty());
    }

    #[test]
    fn handles_truncation() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("events.jsonl");
        append(&path, r#"{"kind":"a","value":1}"#);
        append(&path, "\n");
        append(&path, r#"{"kind":"b","value":2}"#);
        append(&path, "\n");

        let mut tailer: IncrementalTailer<TestEvent> = IncrementalTailer::new(&path);
        let n = tailer.tick().expect("first tick");
        assert_eq!(n, 2);

        // Truncate and write fresh data.
        fs::write(&path, r#"{"kind":"fresh","value":99}"#.to_owned() + "\n").expect("truncate");

        let n = tailer.tick().expect("after truncation");
        assert_eq!(n, 1);
        // Old items should be cleared, only fresh remains.
        assert_eq!(tailer.len(), 1);
        assert_eq!(tailer.items()[0].kind, "fresh");
    }

    #[test]
    fn efficiency_tailer_skips_feedback_rows_without_parse_errors() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("efficiency.jsonl");

        // Per-turn efficiency rows: one current (schema-tagged), one legacy
        // row that predates the `schema` discriminator.
        let current = AgentEfficiencyEvent {
            agent_id: "T01:1".to_string(),
            task_id: "T01".to_string(),
            duration_ms: 1_200,
            ..AgentEfficiencyEvent::default()
        };
        let mut legacy = serde_json::to_value(AgentEfficiencyEvent {
            agent_id: "T02:1".to_string(),
            task_id: "T02".to_string(),
            duration_ms: 3_400,
            ..AgentEfficiencyEvent::default()
        })
        .expect("serialize legacy event");
        legacy
            .as_object_mut()
            .expect("event serializes as an object")
            .remove("schema");

        // Per-call rows the FeedbackService writes into the same file, with
        // and without the schema discriminator.
        let feedback = r#"{"cost_usd":0.0,"input_tokens":0,"kind":"model_call","latency_ms":2600,"model":"claude-sonnet-4-6","output_tokens":0,"provider":"claude_cli","request_id":"dispatch-v2-demo/T01","role":"dispatch_v2","schema":"feedback_event/v1","success":false,"ts":"2026-09-05T15:40:28Z"}"#;
        let legacy_feedback = r#"{"kind":"gate_result","success":true,"latency_ms":40}"#;

        for line in [
            serde_json::to_string(&current).expect("serialize current event"),
            feedback.to_string(),
            legacy.to_string(),
            legacy_feedback.to_string(),
        ] {
            append(&path, &line);
            append(&path, "\n");
        }

        let mut tailer = efficiency_tailer(&path);
        assert_eq!(tailer.tick().expect("tick"), 2);
        assert_eq!(tailer.parse_errors, 0);
        assert_eq!(tailer.skipped_rows, 2);
        let task_ids: Vec<&str> = tailer
            .items()
            .iter()
            .map(|event| event.task_id.as_str())
            .collect();
        assert_eq!(task_ids, ["T01", "T02"]);
        assert_eq!(tailer.items()[1].duration_ms, 3_400);

        // Genuinely malformed rows are still reported.
        append(&path, "NOT VALID JSON\n");
        assert_eq!(tailer.tick().expect("second tick"), 0);
        assert_eq!(tailer.parse_errors, 1);
        assert_eq!(tailer.skipped_rows, 2);
    }
}
