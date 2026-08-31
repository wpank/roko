//! JsonlLogger -- persists RuntimeEvents to a JSONL file.
//!
//! Each event is serialized as a single JSON line with a timestamp, enabling
//! replay and state reconstruction.

use roko_core::RuntimeEvent;
pub use roko_core::foundation::EventConsumer;
use roko_core::runtime_event::RuntimeEventEnvelope;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fmt::Write as _;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

/// Logger that writes RuntimeEvents as JSONL (one JSON object per line).
pub struct JsonlLogger {
    path: PathBuf,
    seq: AtomicU64,
    writer: Mutex<Option<std::io::BufWriter<std::fs::File>>>,
    run_writers: Mutex<HashMap<PathBuf, std::io::BufWriter<std::fs::File>>>,
}

const MAX_OPEN_RUN_WRITERS: usize = 32;

impl JsonlLogger {
    /// Create a new JsonlLogger writing to the given path.
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            seq: AtomicU64::new(0),
            writer: Mutex::new(None),
            run_writers: Mutex::new(HashMap::new()),
        }
    }

    /// Create from the standard .roko directory.
    pub fn from_roko_dir(roko_dir: &Path) -> Self {
        Self::new(roko_dir.join("runtime-events.jsonl"))
    }

    /// Path to the log file.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Resolve the safe per-run index written alongside the global log.
    ///
    /// The run identifier is validated, then SHA-256 hashed; it is never used
    /// as a path component. Readers can therefore resolve the same file without
    /// maintaining a second identifier mapping.
    pub fn run_path(&self, run_id: &str) -> Result<PathBuf, &'static str> {
        if run_id.is_empty() || run_id.len() > 128 {
            return Err("run id must contain 1..=128 bytes");
        }
        if !run_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':'))
            || run_id.contains("..")
        {
            return Err("run id contains unsupported characters");
        }

        let stem = self
            .path
            .file_stem()
            .and_then(|value| value.to_str())
            .filter(|value| !value.is_empty())
            .ok_or("global log has no UTF-8 file stem")?;
        let parent = self.path.parent().ok_or("global log has no parent")?;
        let digest = Sha256::digest(run_id.as_bytes());
        let mut encoded = String::with_capacity(digest.len() * 2);
        for byte in digest {
            let _ = write!(&mut encoded, "{byte:02x}");
        }
        Ok(parent
            .join(format!("{stem}-by-run"))
            .join(format!("{encoded}.jsonl")))
    }

    /// Flush buffered derived records for one run before a read-side query.
    ///
    /// The global compatibility log is already flushed independently. A
    /// missing writer is normal for an inactive or not-yet-observed run.
    pub fn flush_run(&self, run_id: &str) -> std::io::Result<()> {
        let Ok(run_path) = self.run_path(run_id) else {
            return Ok(());
        };
        let mut writers = self
            .run_writers
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(writer) = writers.get_mut(&run_path) {
            writer.flush()?;
        }
        Ok(())
    }

    fn ensure_writer(&self) -> std::io::Result<()> {
        let mut writer = self
            .writer
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if writer.is_none() {
            if let Some(parent) = self.path.parent() {
                std::fs::create_dir_all(parent)?;
            }

            let file = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&self.path)?;
            *writer = Some(std::io::BufWriter::new(file));
        }
        drop(writer);

        Ok(())
    }

    /// Persist one event and return its exact post-append byte cursor in the
    /// derived per-run index.
    ///
    /// The cursor is measured while the per-run writer mutex is still held, so
    /// another producer using this logger cannot append between this event and
    /// the offset observation. The derived writer is flushed, but not fsynced;
    /// the global compatibility log remains the authoritative durable record.
    pub fn consume_with_run_cursor(&self, event: &RuntimeEvent) -> Option<u64> {
        match self.write_event(event, true) {
            Ok(cursor) => cursor,
            Err(error) => {
                tracing::warn!(
                    run_id = event.run_id(),
                    %error,
                    "failed to persist runtime event before live publication",
                );
                None
            }
        }
    }

    fn write_event(
        &self,
        event: &RuntimeEvent,
        require_run_cursor: bool,
    ) -> std::io::Result<Option<u64>> {
        self.ensure_writer()?;

        let envelope = RuntimeEventEnvelope::new(
            event.run_id(),
            self.seq.fetch_add(1, Ordering::Relaxed),
            "jsonl_logger",
            event.clone(),
        );

        let mut json = serde_json::to_string(&envelope)
            .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidData, err))?;
        json.push('\n');

        let mut writer = self
            .writer
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(ref mut w) = *writer {
            w.write_all(json.as_bytes())?;
            w.flush()?;
        }
        drop(writer);

        // Keep a bounded set of per-run append handles. Clearing the cache
        // closes and flushes old writers; it never deletes an index. The global
        // log remains the durable compatibility source if this secondary write
        // encounters an error.
        let run_cursor = match self.write_run_index(event, json.as_bytes(), require_run_cursor) {
            Ok(cursor) => cursor,
            Err(error) => {
                tracing::warn!(
                    run_id = event.run_id(),
                    %error,
                    "failed to append derived per-run runtime event index",
                );
                None
            }
        };

        Ok(run_cursor)
    }

    fn write_run_index(
        &self,
        event: &RuntimeEvent,
        json: &[u8],
        require_cursor: bool,
    ) -> std::io::Result<Option<u64>> {
        let Ok(run_path) = self.run_path(event.run_id()) else {
            return Ok(None);
        };
        if let Some(parent) = run_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut writers = self
            .run_writers
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if !writers.contains_key(&run_path) && writers.len() >= MAX_OPEN_RUN_WRITERS {
            writers.clear();
        }
        if !writers.contains_key(&run_path) {
            let file = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&run_path)?;
            writers.insert(run_path.clone(), std::io::BufWriter::new(file));
        }
        if let Some(run_writer) = writers.get_mut(&run_path) {
            run_writer.write_all(json)?;
            if require_cursor || runtime_index_flush_boundary(event) {
                run_writer.flush()?;
            }
            if require_cursor {
                // The mutex is deliberately held across flush + metadata so a
                // concurrent producer cannot make this event inherit a later
                // event's cursor.
                return Ok(Some(run_writer.get_ref().metadata()?.len()));
            }
        }
        Ok(None)
    }
}

fn runtime_index_flush_boundary(event: &RuntimeEvent) -> bool {
    matches!(
        event,
        RuntimeEvent::WorkflowCompleted { .. }
            | RuntimeEvent::RunCompleted { .. }
            | RuntimeEvent::TaskCompleted { .. }
            | RuntimeEvent::TaskFailed { .. }
            | RuntimeEvent::AgentCompleted { .. }
            | RuntimeEvent::AgentFailed { .. }
            | RuntimeEvent::GatePassed { .. }
            | RuntimeEvent::GateFailed { .. }
            | RuntimeEvent::InferenceFailed { .. }
    )
}

impl EventConsumer for JsonlLogger {
    fn consume(&self, event: &RuntimeEvent) {
        let _ = self.write_event(event, false);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_events_to_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("events.jsonl");
        let logger = JsonlLogger::new(path.clone());

        logger.consume(&RuntimeEvent::AgentSpawned {
            run_id: "r1".into(),
            agent_id: "a1".into(),
            role: "implementer".into(),
            model: "model".into(),
        });

        logger.consume(&RuntimeEvent::GatePassed {
            run_id: "r1".into(),
            gate_name: "compile".into(),
            duration_ms: 100,
        });

        let content = std::fs::read_to_string(&path).unwrap();
        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(lines.len(), 2);

        let first: RuntimeEventEnvelope = serde_json::from_str(lines[0]).unwrap();
        let second: RuntimeEventEnvelope = serde_json::from_str(lines[1]).unwrap();
        assert_eq!(first.payload.kind(), "agent_spawned");
        assert_eq!(second.payload.kind(), "gate_passed");

        let run_content = std::fs::read_to_string(logger.run_path("r1").unwrap()).unwrap();
        assert_eq!(run_content.lines().count(), 2);
        assert!(!logger.run_path("r1").unwrap().to_string_lossy().contains("r1"));
    }

    #[test]
    fn run_path_rejects_traversal() {
        let logger = JsonlLogger::new(PathBuf::from("/tmp/events.jsonl"));
        assert!(logger.run_path("../../outside").is_err());
        assert!(logger.run_path("with/slash").is_err());
    }

    #[test]
    fn live_cursors_are_exact_monotonic_run_offsets() {
        let dir = tempfile::tempdir().unwrap();
        let logger = JsonlLogger::new(dir.path().join("events.jsonl"));
        let first = RuntimeEvent::AgentSpawned {
            run_id: "r1".into(),
            agent_id: "a1".into(),
            role: "implementer".into(),
            model: "model".into(),
        };
        let second = RuntimeEvent::GatePassed {
            run_id: "r1".into(),
            gate_name: "compile".into(),
            duration_ms: 100,
        };

        let first_cursor = logger
            .consume_with_run_cursor(&first)
            .expect("first cursor");
        let second_cursor = logger
            .consume_with_run_cursor(&second)
            .expect("second cursor");

        assert!(first_cursor < second_cursor);
        assert_eq!(
            std::fs::metadata(logger.run_path("r1").unwrap())
                .unwrap()
                .len(),
            second_cursor,
        );
    }
}
