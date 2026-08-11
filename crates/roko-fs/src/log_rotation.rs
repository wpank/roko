//! Size-based JSONL log rotation for `.roko/` data files.
//!
//! When episodes.jsonl, signals.jsonl, efficiency.jsonl, or other JSONL files
//! exceed a configurable size threshold, they are atomically renamed to a
//! timestamped archive and a fresh empty live file is created.
//!
//! # Archive naming
//!
//! A file `episodes.jsonl` is archived as `episodes.20260811T143022Z.jsonl`.
//! The timestamp is UTC ISO-8601 compact format. Readers and GC discover
//! timestamped archives by scanning the parent directory for files matching
//! `{stem}.{timestamp}.jsonl`.
//!
//! # Serialization safety
//!
//! Rotation never splits a JSONL record. The rename-then-recreate sequence
//! is atomic at the filesystem level: the old file is renamed (preserving
//! all complete lines), and a new empty file is created at the original path.

use std::path::{Path, PathBuf};

use crate::layout::RokoLayout;

/// Result of a single log file rotation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RotationResult {
    /// The canonical live path that was rotated (e.g. `.roko/episodes.jsonl`).
    pub original_path: PathBuf,
    /// The timestamped archive path (e.g. `.roko/episodes.20260811T143022Z.jsonl`).
    pub archive_path: PathBuf,
    /// Size of the file at rotation time, in bytes.
    pub original_size: u64,
}

/// Check if a JSONL file exceeds `max_mb` and rotate it if so.
///
/// If the file does not exist or is below the threshold, returns `Ok(None)`.
/// On rotation, the file is atomically renamed to a timestamped archive and
/// a fresh empty file is created at the original path.
///
/// # Errors
///
/// Returns an error if metadata, rename, or file creation fails.
pub async fn rotate_if_needed(
    path: &Path,
    max_mb: u64,
) -> std::io::Result<Option<RotationResult>> {
    if !path.is_file() {
        return Ok(None);
    }

    let meta = tokio::fs::metadata(path).await?;
    let size = meta.len();
    let threshold_bytes = max_mb * 1024 * 1024;

    if size < threshold_bytes {
        return Ok(None);
    }

    let archive_path = make_archive_path(path);

    // Atomic rename: moves the complete file (all JSONL lines intact).
    tokio::fs::rename(path, &archive_path).await?;

    // Create a fresh empty live file at the original path.
    tokio::fs::write(path, b"").await?;

    Ok(Some(RotationResult {
        original_path: path.to_path_buf(),
        archive_path,
        original_size: size,
    }))
}

/// Rotate all known JSONL log files under the given layout.
///
/// Checks each canonical JSONL path (episodes.jsonl, signals.jsonl,
/// efficiency.jsonl, gate-verdicts.jsonl, events.jsonl) and rotates
/// any that exceed `max_mb`.
///
/// # Errors
///
/// Returns an error if any individual rotation fails. Partial rotations
/// may have occurred; the caller should treat this as best-effort.
pub async fn rotate_all_logs(
    layout: &RokoLayout,
    max_mb: u64,
) -> std::io::Result<Vec<RotationResult>> {
    let paths = rotatable_jsonl_paths(layout);
    let mut results = Vec::new();

    for path in paths {
        if let Some(result) = rotate_if_needed(&path, max_mb).await? {
            results.push(result);
        }
    }

    Ok(results)
}

/// List all JSONL paths that are candidates for size-based rotation.
#[must_use]
pub fn rotatable_jsonl_paths(layout: &RokoLayout) -> Vec<PathBuf> {
    vec![
        layout.episodes_path(),
        layout.signals_path(),
        layout.efficiency_path(),
        layout.gate_verdicts_path(),
        layout.events_jsonl_path(),
    ]
}

/// Discover timestamped JSONL archive generations for a given live file.
///
/// Scans the parent directory for files matching `{stem}.{timestamp}.jsonl`
/// where `{stem}` is the base name without `.jsonl`. Returns paths sorted
/// oldest-first by filename (which sorts chronologically due to ISO-8601
/// timestamp format).
///
/// # Errors
///
/// Returns an error if the parent directory cannot be read.
pub async fn discover_archives(live_path: &Path) -> std::io::Result<Vec<PathBuf>> {
    let parent = match live_path.parent() {
        Some(p) => p,
        None => return Ok(Vec::new()),
    };

    let stem = match live_path.file_stem().and_then(|s| s.to_str()) {
        Some(s) => s.to_string(),
        None => return Ok(Vec::new()),
    };

    if !parent.is_dir() {
        return Ok(Vec::new());
    }

    let prefix = format!("{stem}.");
    let suffix = ".jsonl";

    let mut archives = Vec::new();
    let mut entries = tokio::fs::read_dir(parent).await?;
    while let Some(entry) = entries.next_entry().await? {
        let name = entry.file_name();
        let name_str = match name.to_str() {
            Some(s) => s,
            None => continue,
        };

        // Match pattern: {stem}.{timestamp}.jsonl
        // The live file itself is {stem}.jsonl — skip it.
        if name_str.starts_with(&prefix) && name_str.ends_with(suffix) && name_str != format!("{stem}.jsonl") {
            // Verify the middle part looks like a timestamp (basic check).
            let middle = &name_str[prefix.len()..name_str.len() - suffix.len()];
            if is_timestamp_like(middle) {
                archives.push(parent.join(name_str));
            }
        }
    }

    // Sort by filename — ISO-8601 timestamps sort chronologically.
    archives.sort();
    Ok(archives)
}

/// Construct a timestamped archive path from a live JSONL path.
///
/// `episodes.jsonl` becomes `episodes.20260811T143022Z.jsonl`.
fn make_archive_path(live_path: &Path) -> PathBuf {
    let parent = live_path.parent().unwrap_or(Path::new("."));
    let stem = live_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("log");

    let now = chrono::Utc::now();
    let timestamp = now.format("%Y%m%dT%H%M%SZ");

    parent.join(format!("{stem}.{timestamp}.jsonl"))
}

/// Basic check whether a string looks like a compact ISO-8601 timestamp.
///
/// Accepts strings like `20260811T143022Z` (15 characters, digits + T + Z).
fn is_timestamp_like(s: &str) -> bool {
    // Compact ISO-8601: 20260811T143022Z → 16 chars
    s.len() >= 15
        && s.len() <= 20
        && s.chars()
            .all(|c| c.is_ascii_digit() || c == 'T' || c == 'Z')
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn rotation_no_op_when_below_threshold() {
        let tmp = TempDir::new().expect("tempdir");
        let path = tmp.path().join("episodes.jsonl");
        // Write 10 bytes — well below 1 MB.
        tokio::fs::write(&path, "small data\n").await.unwrap();

        let result = rotate_if_needed(&path, 1).await.unwrap();
        assert!(result.is_none(), "should not rotate a small file");
        assert!(path.exists(), "original file should still exist");
    }

    #[tokio::test]
    async fn rotation_no_op_when_file_missing() {
        let tmp = TempDir::new().expect("tempdir");
        let path = tmp.path().join("nonexistent.jsonl");

        let result = rotate_if_needed(&path, 1).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn rotation_archives_large_file() {
        let tmp = TempDir::new().expect("tempdir");
        let path = tmp.path().join("episodes.jsonl");

        // Write data that exceeds the threshold.
        // Use max_mb=0 so any non-empty file triggers rotation.
        let data = "{\"id\":1}\n{\"id\":2}\n{\"id\":3}\n";
        tokio::fs::write(&path, data).await.unwrap();

        let result = rotate_if_needed(&path, 0).await.unwrap();
        assert!(result.is_some(), "should rotate when exceeding threshold");

        let rotation = result.unwrap();
        assert_eq!(rotation.original_path, path);
        assert!(
            rotation.archive_path.exists(),
            "archive file should exist: {:?}",
            rotation.archive_path
        );
        assert!(path.exists(), "fresh live file should exist");

        // The live file should be empty after rotation.
        let live_contents = tokio::fs::read_to_string(&path).await.unwrap();
        assert!(live_contents.is_empty(), "fresh live file should be empty");

        // The archive should have all the original data.
        let archive_contents = tokio::fs::read_to_string(&rotation.archive_path)
            .await
            .unwrap();
        assert_eq!(archive_contents, data);
        assert_eq!(rotation.original_size, data.len() as u64);
    }

    #[tokio::test]
    async fn rotation_preserves_complete_jsonl_lines() {
        let tmp = TempDir::new().expect("tempdir");
        let path = tmp.path().join("signals.jsonl");

        let mut data = String::new();
        for i in 0..100 {
            data.push_str(&format!("{{\"line\":{i}}}\n"));
        }
        tokio::fs::write(&path, &data).await.unwrap();

        let result = rotate_if_needed(&path, 0).await.unwrap().unwrap();

        // Verify the archive contains valid complete JSONL.
        let archive_data = tokio::fs::read_to_string(&result.archive_path)
            .await
            .unwrap();
        let line_count = archive_data
            .lines()
            .filter(|l| !l.trim().is_empty())
            .count();
        assert_eq!(line_count, 100, "all 100 lines should be in the archive");

        // Verify each line is valid JSON.
        for line in archive_data.lines() {
            if line.trim().is_empty() {
                continue;
            }
            serde_json::from_str::<serde_json::Value>(line)
                .expect("each archived line must be valid JSON");
        }
    }

    #[tokio::test]
    async fn discover_archives_finds_timestamped_files() {
        let tmp = TempDir::new().expect("tempdir");
        let live = tmp.path().join("episodes.jsonl");
        let archive1 = tmp.path().join("episodes.20260101T000000Z.jsonl");
        let archive2 = tmp.path().join("episodes.20260601T120000Z.jsonl");
        let unrelated = tmp.path().join("other.jsonl");

        tokio::fs::write(&live, "live\n").await.unwrap();
        tokio::fs::write(&archive1, "old\n").await.unwrap();
        tokio::fs::write(&archive2, "newer\n").await.unwrap();
        tokio::fs::write(&unrelated, "unrelated\n").await.unwrap();

        let archives = discover_archives(&live).await.unwrap();
        assert_eq!(archives.len(), 2);
        assert_eq!(archives[0], archive1);
        assert_eq!(archives[1], archive2);
    }

    #[tokio::test]
    async fn discover_archives_returns_empty_when_no_archives() {
        let tmp = TempDir::new().expect("tempdir");
        let live = tmp.path().join("episodes.jsonl");
        tokio::fs::write(&live, "data\n").await.unwrap();

        let archives = discover_archives(&live).await.unwrap();
        assert!(archives.is_empty());
    }

    #[tokio::test]
    async fn discover_archives_returns_empty_for_missing_file() {
        let tmp = TempDir::new().expect("tempdir");
        let live = tmp.path().join("nonexistent.jsonl");

        let archives = discover_archives(&live).await.unwrap();
        assert!(archives.is_empty());
    }

    #[tokio::test]
    async fn rotate_all_logs_processes_layout_paths() {
        let tmp = TempDir::new().expect("tempdir");
        let layout = RokoLayout::for_project(tmp.path());
        layout.ensure_dirs().await.unwrap();

        // Write some data to episodes and efficiency paths.
        let episodes = layout.episodes_path();
        let efficiency = layout.efficiency_path();

        tokio::fs::write(&episodes, "episode data\n").await.unwrap();
        tokio::fs::write(&efficiency, "efficiency data\n")
            .await
            .unwrap();

        // Use threshold=0 to force rotation of any non-empty file.
        let results = rotate_all_logs(&layout, 0).await.unwrap();

        // Both files should have been rotated.
        assert_eq!(results.len(), 2, "should rotate episodes and efficiency");
        assert!(episodes.exists(), "fresh episodes.jsonl should exist");
        assert!(efficiency.exists(), "fresh efficiency.jsonl should exist");
    }

    #[test]
    fn archive_path_has_correct_format() {
        let live = PathBuf::from("/project/.roko/episodes.jsonl");
        let archive = make_archive_path(&live);

        let name = archive.file_name().unwrap().to_str().unwrap();
        assert!(name.starts_with("episodes."), "should start with stem");
        assert!(name.ends_with(".jsonl"), "should end with .jsonl");
        assert!(name.len() > "episodes..jsonl".len(), "should have timestamp");

        // Extract the timestamp part.
        let middle = &name["episodes.".len()..name.len() - ".jsonl".len()];
        assert!(is_timestamp_like(middle), "middle should be a timestamp: {middle}");
    }

    #[test]
    fn is_timestamp_like_accepts_valid() {
        assert!(is_timestamp_like("20260811T143022Z"));
        assert!(is_timestamp_like("20260101T000000Z"));
    }

    #[test]
    fn is_timestamp_like_rejects_invalid() {
        assert!(!is_timestamp_like("short"));
        assert!(!is_timestamp_like("not-a-timestamp!"));
        assert!(!is_timestamp_like("v1-legacy"));
    }

    #[test]
    fn rotatable_paths_covers_expected_files() {
        let layout = RokoLayout::new("/test/.roko");
        let paths = rotatable_jsonl_paths(&layout);

        let names: Vec<String> = paths
            .iter()
            .filter_map(|p| p.file_name().and_then(|n| n.to_str()).map(String::from))
            .collect();

        assert!(names.contains(&"episodes.jsonl".to_string()));
        assert!(names.contains(&"signals.jsonl".to_string()));
        assert!(names.contains(&"efficiency.jsonl".to_string()));
        assert!(names.contains(&"gate-verdicts.jsonl".to_string()));
        assert!(names.contains(&"events.jsonl".to_string()));
    }
}
