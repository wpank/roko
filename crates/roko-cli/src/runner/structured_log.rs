//! Structured JSONL logger for plan execution events.
//!
//! When `--log-file <path>` is passed to `roko plan run`, every
//! [`RunnerEvent`](super::types::RunnerEvent) emitted by the event loop
//! is serialized as a single JSON line and flushed to the file.
//! The logger is a no-op when no path is configured.

use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;
use std::sync::{Arc, Mutex};

use super::types::RunnerEvent;

/// Thread-safe structured logger backed by an optional JSONL file.
///
/// Cloneable via `Arc<Mutex<...>>` so it can be shared across the event
/// loop without requiring `&mut` at every emit site.
#[derive(Clone)]
pub struct StructuredLogger {
    inner: Option<Arc<Mutex<BufWriter<File>>>>,
}

impl StructuredLogger {
    /// Create a logger that writes to the given path.
    ///
    /// The file is created (or truncated) immediately. Returns an error
    /// if the file cannot be opened.
    pub fn open(path: &Path) -> std::io::Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let file = File::create(path)?;
        Ok(Self {
            inner: Some(Arc::new(Mutex::new(BufWriter::new(file)))),
        })
    }

    /// Create a no-op logger that discards all events.
    #[must_use]
    pub fn noop() -> Self {
        Self { inner: None }
    }

    /// Write a runner event as a single JSONL line.
    ///
    /// Silently drops events when serialization or I/O fails (structured
    /// logging must never block or crash the executor).
    pub fn log(&self, event: &RunnerEvent) {
        let Some(ref writer) = self.inner else {
            return;
        };
        let Ok(mut guard) = writer.lock() else {
            return;
        };
        if serde_json::to_writer(&mut *guard, event).is_ok() {
            let _ = guard.write_all(b"\n");
            let _ = guard.flush();
        }
    }
}
