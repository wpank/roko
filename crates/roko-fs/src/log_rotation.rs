//! Size-based JSONL log rotation for `.roko/` data files.
//!
//! When episodes.jsonl, engrams.jsonl, efficiency.jsonl, or other JSONL files
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

use std::collections::BTreeSet;
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::layout::RokoLayout;
use fs2::FileExt;

/// Result of a single log file rotation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RotationResult {
    /// The canonical live path that was rotated (e.g. `.roko/episodes.jsonl`).
    pub original_path: PathBuf,
    /// The timestamped archive path (e.g. `.roko/episodes.20260811T143022Z.jsonl`).
    pub archive_path: PathBuf,
    /// Size of the file at rotation time, in bytes.
    pub original_size: u64,
    /// Size of the completed archive generation, in bytes.
    pub archive_size: u64,
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
pub async fn rotate_if_needed(path: &Path, max_mb: u64) -> std::io::Result<Option<RotationResult>> {
    let path = path.to_path_buf();
    tokio::task::spawn_blocking(move || rotate_if_needed_sync(&path, max_mb))
        .await
        .map_err(|error| std::io::Error::other(format!("log rotation task failed: {error}")))?
}

/// Append one already-serialized JSONL record while holding the same advisory
/// lock used by rotation.
///
/// `line` may omit its trailing newline; one is added when necessary. The
/// size check, optional rotation, append, flush, and data sync are one locked
/// operation, so a record cannot be stranded in an archived generation by a
/// concurrent lifecycle rotation.
pub fn append_jsonl_line_sync(
    path: &Path,
    line: &[u8],
    max_mb: u64,
) -> std::io::Result<Option<RotationResult>> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let _lock = lock_jsonl(path)?;
    let rotation = rotate_if_needed_unlocked(path, max_mb)?;
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    file.write_all(line)?;
    if !line.ends_with(b"\n") {
        file.write_all(b"\n")?;
    }
    file.flush()?;
    file.sync_data()?;
    Ok(rotation)
}

/// Append a non-milestone JSONL record without forcing an immediate disk
/// synchronization.
///
/// The advisory lock and rotation boundary are identical to
/// [`append_jsonl_line_sync`], so records cannot cross generations. The bytes
/// are written to the kernel before the file closes, but a sudden power loss
/// may drop the most recent relaxed records. Use this only for replayable,
/// high-frequency output deltas; lifecycle, usage, gate, and terminal records
/// must use the durable function above.
pub fn append_jsonl_line_relaxed_sync(
    path: &Path,
    line: &[u8],
    max_mb: u64,
) -> std::io::Result<Option<RotationResult>> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let _lock = lock_jsonl(path)?;
    let rotation = rotate_if_needed_unlocked(path, max_mb)?;
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    file.write_all(line)?;
    if !line.ends_with(b"\n") {
        file.write_all(b"\n")?;
    }
    Ok(rotation)
}

/// Atomically retain only the newest `max_lines` complete records in a live
/// JSONL file while holding the canonical append/rotation lock.
///
/// Returns the number of records removed. A zero limit truncates the live log
/// to an empty file. An unterminated crash tail is discarded rather than
/// promoted into the compacted generation.
pub fn compact_jsonl_to_latest_sync(path: &Path, max_lines: usize) -> std::io::Result<usize> {
    let _lock = lock_jsonl(path)?;
    let contents = match std::fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(0),
        Err(error) => return Err(error),
    };
    let lines = complete_jsonl_lines(&contents);
    if lines.len() <= max_lines {
        return Ok(0);
    }

    let removed = lines.len() - max_lines;
    let retained = &lines[removed..];
    let replacement = if retained.is_empty() {
        String::new()
    } else {
        retained.join("\n") + "\n"
    };
    let tmp = path.with_extension("jsonl.compacting");
    {
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(&tmp)?;
        file.write_all(replacement.as_bytes())?;
        file.flush()?;
        file.sync_all()?;
    }
    std::fs::rename(&tmp, path)?;
    Ok(removed)
}

/// Replace the live JSONL file with `contents` and remove its timestamped
/// archive generations under the canonical lock.
///
/// This is used after a reader compacts across all generations: leaving the
/// prior archives in place would make those records reappear on the next
/// archive-aware read.
pub fn replace_jsonl_generations_sync(path: &Path, contents: &[u8]) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let _lock = lock_jsonl(path)?;
    let tmp = path.with_extension("jsonl.compacting");
    {
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(&tmp)?;
        file.write_all(contents)?;
        file.flush()?;
        file.sync_all()?;
    }
    std::fs::rename(&tmp, path)?;
    for archive in discover_archives_sync(path)? {
        match std::fs::remove_file(archive) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
    }
    Ok(())
}

fn rotate_if_needed_sync(path: &Path, max_mb: u64) -> std::io::Result<Option<RotationResult>> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let _lock = lock_jsonl(path)?;
    rotate_if_needed_unlocked(path, max_mb)
}

fn rotate_if_needed_unlocked(path: &Path, max_mb: u64) -> std::io::Result<Option<RotationResult>> {
    let meta = match std::fs::metadata(path) {
        Ok(meta) if meta.is_file() => meta,
        Ok(_) => return Ok(None),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error),
    };

    let size = meta.len();
    let threshold_bytes = max_mb.saturating_mul(1024 * 1024);

    if size < threshold_bytes {
        return Ok(None);
    }

    let archive_path = make_archive_path(path);

    // Atomic rename: moves the complete file (all JSONL lines intact).
    std::fs::rename(path, &archive_path)?;

    // Create a fresh empty live file at the original path.
    std::fs::OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(path)?;

    let archive_size = std::fs::metadata(&archive_path)?.len();

    Ok(Some(RotationResult {
        original_path: path.to_path_buf(),
        archive_path,
        original_size: size,
        archive_size,
    }))
}

/// Rotate all known JSONL log files under the given layout.
///
/// Checks each canonical JSONL path (episodes, signals, efficiency, gate
/// verdicts, events, and periodic telemetry) and rotates any that exceed
/// `max_mb`.
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
    let mut paths = BTreeSet::from([
        layout.episodes_path(),
        layout.signals_path(),
        layout.efficiency_path(),
        layout.gate_verdicts_path(),
        layout.events_jsonl_path(),
        layout.telemetry_observations_path(),
    ]);

    // Learning adds new append-only streams over time. Discover them rather
    // than requiring every producer to update this list, while excluding
    // timestamped generations (which are immutable archives, not live logs).
    if let Ok(entries) = std::fs::read_dir(layout.learn_dir()) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) == Some("jsonl")
                && path
                    .file_stem()
                    .and_then(|stem| stem.to_str())
                    .is_some_and(|stem| {
                        !stem
                            .rsplit_once('.')
                            .is_some_and(|(_, suffix)| is_timestamp_like(suffix))
                    })
            {
                paths.insert(path);
            }
        }
    }

    paths.into_iter().collect()
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
    let live_path = live_path.to_path_buf();
    tokio::task::spawn_blocking(move || discover_archives_sync(&live_path))
        .await
        .map_err(|error| std::io::Error::other(format!("archive discovery task failed: {error}")))?
}

fn discover_archives_sync(live_path: &Path) -> std::io::Result<Vec<PathBuf>> {
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
    let entries = match std::fs::read_dir(parent) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error),
    };
    for entry in entries {
        let entry = entry?;
        let name = entry.file_name();
        let name_str = match name.to_str() {
            Some(s) => s,
            None => continue,
        };

        // Match pattern: {stem}.{timestamp}.jsonl
        // The live file itself is {stem}.jsonl — skip it.
        if name_str.starts_with(&prefix)
            && name_str.ends_with(suffix)
            && name_str != format!("{stem}.jsonl")
        {
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
    let base = now.format("%Y%m%dT%H%M%S%9f").to_string();
    for collision in 0_u16..=u16::MAX {
        let timestamp = if collision == 0 {
            format!("{base}Z")
        } else {
            format!("{base}{collision:05}Z")
        };
        let candidate = parent.join(format!("{stem}.{timestamp}.jsonl"));
        if !candidate.exists() {
            return candidate;
        }
    }

    // Exhausting 65,536 same-nanosecond archive names is not realistic, but
    // keep a deterministic fallback rather than overwriting a generation.
    parent.join(format!("{stem}.{base}65536Z.jsonl"))
}

/// Basic check whether a string looks like a compact ISO-8601 timestamp.
///
/// Accepts strings like `20260811T143022Z` (16 characters, digits + T + Z).
fn is_timestamp_like(s: &str) -> bool {
    // Compact ISO-8601: 20260811T143022Z → 16 chars
    s.len() >= 15
        && s.len() <= 40
        && s.chars()
            .all(|c| c.is_ascii_digit() || c == 'T' || c == 'Z')
}

fn complete_jsonl_lines(contents: &str) -> Vec<&str> {
    contents
        .split_inclusive('\n')
        .filter_map(|record| record.strip_suffix('\n'))
        .map(|record| record.strip_suffix('\r').unwrap_or(record))
        .filter(|record| !record.trim().is_empty())
        .collect()
}

fn lock_jsonl(path: &Path) -> std::io::Result<std::fs::File> {
    let lock_path = path.with_extension("jsonl.lock");
    let lock = std::fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(lock_path)?;
    lock.lock_exclusive()?;
    Ok(lock)
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
        assert_eq!(rotation.archive_size, data.len() as u64);
    }

    #[tokio::test]
    async fn rotation_preserves_complete_jsonl_lines() {
        let tmp = TempDir::new().expect("tempdir");
        let path = tmp.path().join("engrams.jsonl");

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
    async fn discover_archives_returns_empty_for_missing_parent() {
        let tmp = TempDir::new().expect("tempdir");
        let live = tmp
            .path()
            .join("not-created")
            .join("telemetry-observations.jsonl");

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

    #[tokio::test]
    async fn rapid_rotations_never_overwrite_an_archive_generation() -> std::io::Result<()> {
        let tmp = TempDir::new().expect("tempdir");
        let path = tmp.path().join("events.jsonl");

        append_jsonl_line_sync(&path, br#"{"generation":1}"#, 100).unwrap();
        rotate_if_needed(&path, 0).await.unwrap().unwrap();
        append_jsonl_line_sync(&path, br#"{"generation":2}"#, 100).unwrap();
        rotate_if_needed(&path, 0).await.unwrap().unwrap();

        let archives = discover_archives(&path).await.unwrap();
        assert_eq!(archives.len(), 2);
        let generations = archives
            .iter()
            .map(std::fs::read_to_string)
            .collect::<Result<Vec<_>, _>>()?
            .join("");
        assert!(generations.contains("\"generation\":1"));
        assert!(generations.contains("\"generation\":2"));
        Ok(())
    }

    #[tokio::test]
    async fn rotatable_paths_discovers_additional_live_learning_logs() {
        let tmp = TempDir::new().expect("tempdir");
        let layout = RokoLayout::for_project(tmp.path());
        layout.ensure_dirs().await.unwrap();
        let live = layout.learn_dir().join("provider-outcomes.jsonl");
        let archive = layout
            .learn_dir()
            .join("provider-outcomes.20260101T000000Z.jsonl");
        tokio::fs::write(&live, "{}\n").await.unwrap();
        tokio::fs::write(&archive, "{}\n").await.unwrap();

        let paths = rotatable_jsonl_paths(&layout);
        assert!(paths.contains(&live));
        assert!(!paths.contains(&archive));
    }

    #[test]
    fn compact_jsonl_retains_only_newest_complete_records() {
        let tmp = TempDir::new().expect("tempdir");
        let path = tmp.path().join("episodes.jsonl");
        std::fs::write(&path, "{\"id\":1}\n{\"id\":2}\n{\"id\":3}\n").unwrap();

        let removed = compact_jsonl_to_latest_sync(&path, 2).unwrap();

        assert_eq!(removed, 1);
        assert_eq!(
            std::fs::read_to_string(path).unwrap(),
            "{\"id\":2}\n{\"id\":3}\n"
        );
    }

    #[tokio::test]
    async fn replace_generations_removes_archives_after_consolidation() {
        let tmp = TempDir::new().expect("tempdir");
        let live = tmp.path().join("episodes.jsonl");
        let archive = tmp.path().join("episodes.20260101T000000Z.jsonl");
        std::fs::write(&live, "live\n").unwrap();
        std::fs::write(&archive, "old\n").unwrap();

        replace_jsonl_generations_sync(&live, b"consolidated\n").unwrap();

        assert_eq!(std::fs::read_to_string(&live).unwrap(), "consolidated\n");
        assert!(discover_archives(&live).await.unwrap().is_empty());
    }

    #[test]
    fn archive_path_has_correct_format() {
        let live = PathBuf::from("/project/.roko/episodes.jsonl");
        let archive = make_archive_path(&live);

        let name = archive.file_name().unwrap().to_str().unwrap();
        assert!(name.starts_with("episodes."), "should start with stem");
        assert!(name.ends_with(".jsonl"), "should end with .jsonl");
        assert!(
            name.len() > "episodes..jsonl".len(),
            "should have timestamp"
        );

        // Extract the timestamp part.
        let middle = &name["episodes.".len()..name.len() - ".jsonl".len()];
        assert!(
            is_timestamp_like(middle),
            "middle should be a timestamp: {middle}"
        );
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
        assert!(names.contains(&"telemetry-observations.jsonl".to_string()));
    }
}
