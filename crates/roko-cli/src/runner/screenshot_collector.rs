//! Event-driven screenshot collector for `plan run --screenshots`.
//!
//! Captures text-mode TUI snapshots at significant runner lifecycle events
//! (plan startup, task/gate/wave completion, agent spawn/exit, errors) and
//! writes them to `.roko/screenshots/run-<timestamp>/` with a `manifest.json`
//! linking each screenshot to its trigger event.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

use chrono::Utc;
use serde::{Deserialize, Serialize};

/// A single entry in the manifest timeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestEntry {
    /// Sequential index (0-based).
    pub index: usize,
    /// File name relative to the run directory.
    pub file: String,
    /// ISO 8601 timestamp.
    pub timestamp: String,
    /// Elapsed seconds since collector was created.
    pub elapsed_secs: f64,
    /// Event kind that triggered the capture.
    pub event: String,
    /// Optional contextual detail (plan_id, task_id, gate_name, etc.).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

/// Top-level manifest written alongside the screenshots.
#[derive(Debug, Serialize, Deserialize)]
pub struct Manifest {
    /// ISO 8601 start time.
    pub started_at: String,
    /// Run directory path.
    pub run_dir: String,
    /// List of captured screenshots.
    pub entries: Vec<ManifestEntry>,
}

/// Collects event-driven text screenshots during plan execution.
///
/// Thread-safe: the collector uses interior mutability so that the runner
/// event loop can call `capture()` from any context.
pub struct ScreenshotCollector {
    /// Directory for this run's screenshots.
    run_dir: PathBuf,
    /// Monotonic counter for file numbering.
    counter: AtomicUsize,
    /// Start time for elapsed calculation.
    start: Instant,
    /// Start time as ISO 8601 for the manifest.
    start_iso: String,
    /// Accumulated manifest entries (guarded for thread safety).
    entries: Mutex<Vec<ManifestEntry>>,
}

impl ScreenshotCollector {
    /// Create a new collector. Creates the run directory on disk.
    ///
    /// Returns `None` if the directory could not be created.
    pub fn new(workdir: &Path) -> Option<Self> {
        let ts = Utc::now().format("%Y%m%d-%H%M%S").to_string();
        let run_dir = workdir
            .join(".roko")
            .join("screenshots")
            .join(format!("run-{ts}"));
        std::fs::create_dir_all(&run_dir).ok()?;
        Some(Self {
            run_dir,
            counter: AtomicUsize::new(0),
            start: Instant::now(),
            start_iso: Utc::now().to_rfc3339(),
            entries: Mutex::new(Vec::new()),
        })
    }

    /// Capture a text snapshot to disk.
    ///
    /// `event` is a short label like "plan_started" or "task_completed".
    /// `detail` is optional context (plan_id, task_id, etc.).
    /// `content` is the text-mode terminal buffer.
    pub fn capture(&self, event: &str, detail: Option<&str>, content: &str) {
        let idx = self.counter.fetch_add(1, Ordering::Relaxed);
        let file_name = format!("{idx:04}-{event}.txt");
        let file_path = self.run_dir.join(&file_name);

        // Write the text snapshot.
        if let Ok(mut f) = std::fs::File::create(&file_path) {
            let _ = f.write_all(content.as_bytes());
        }

        // Record the manifest entry.
        let entry = ManifestEntry {
            index: idx,
            file: file_name,
            timestamp: Utc::now().to_rfc3339(),
            elapsed_secs: self.start.elapsed().as_secs_f64(),
            event: event.to_string(),
            detail: detail.map(str::to_string),
        };
        if let Ok(mut entries) = self.entries.lock() {
            entries.push(entry);
        }
    }

    /// Flush the manifest to `manifest.json` in the run directory.
    pub fn flush_manifest(&self) {
        let entries = self.entries.lock().map(|e| e.clone()).unwrap_or_default();
        let manifest = Manifest {
            started_at: self.start_iso.clone(),
            run_dir: self.run_dir.display().to_string(),
            entries,
        };
        let manifest_path = self.run_dir.join("manifest.json");
        if let Ok(json) = serde_json::to_string_pretty(&manifest) {
            let _ = std::fs::write(&manifest_path, json);
        }
    }

    /// Return the run directory path.
    #[must_use]
    pub fn run_dir(&self) -> &Path {
        &self.run_dir
    }
}

impl Drop for ScreenshotCollector {
    fn drop(&mut self) {
        self.flush_manifest();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn capture_creates_files_and_manifest() {
        let dir = tempdir().unwrap();
        let workdir = dir.path();
        std::fs::create_dir_all(workdir.join(".roko")).unwrap();

        let collector = ScreenshotCollector::new(workdir).unwrap();
        collector.capture("plan_started", Some("my-plan"), "=== Dashboard ===\n...");
        collector.capture("task_completed", Some("T01"), "=== Task Done ===\n...");
        collector.flush_manifest();

        let run_dir = collector.run_dir();
        assert!(run_dir.join("0000-plan_started.txt").exists());
        assert!(run_dir.join("0001-task_completed.txt").exists());
        assert!(run_dir.join("manifest.json").exists());

        let manifest: Manifest =
            serde_json::from_str(&std::fs::read_to_string(run_dir.join("manifest.json")).unwrap())
                .unwrap();
        assert_eq!(manifest.entries.len(), 2);
        assert_eq!(manifest.entries[0].event, "plan_started");
        assert_eq!(manifest.entries[1].event, "task_completed");
    }
}
