//! Shared atomic-write helpers.
//!
//! Many subsystems persist JSON (or raw bytes) by writing to a temporary file
//! in the same directory and then renaming over the target path. This module
//! consolidates that pattern into two reusable functions so that each call-site
//! doesn't reinvent the same write-tmp-rename dance.
//!
//! Each write uses a uniquely named, exclusively-created sibling staging file.
//! Keeping it on the same filesystem makes the final rename atomic on POSIX,
//! while the unique name prevents concurrent writers from sharing staging
//! state.

use std::ffi::OsString;
use std::io::{self, Write};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

static TEMP_FILE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

/// Atomically persist a `serde::Serialize` value as pretty-printed JSON.
///
/// 1. Creates the parent directory if it does not exist.
/// 2. Serializes `value` to pretty-printed JSON.
/// 3. Writes and syncs the JSON to a unique sibling temporary file.
/// 4. Renames (atomic on POSIX) the temp file over `path` and syncs the parent.
///
/// # Errors
///
/// Returns [`std::io::Error`] if directory creation, serialization, write, or
/// rename fails.
pub fn atomic_write_json<T: serde::Serialize>(path: &Path, value: &T) -> io::Result<()> {
    let json = serde_json::to_string_pretty(value)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    atomic_write_bytes(path, json.as_bytes())
}

/// Atomically persist raw bytes to `path`.
///
/// 1. Creates the parent directory if it does not exist.
/// 2. Writes and syncs `data` to a unique sibling staging file.
/// 3. Renames the staging file over `path` and syncs the parent directory.
///
/// # Errors
///
/// Returns [`std::io::Error`] if directory creation, write, or rename fails.
pub fn atomic_write_bytes(path: &Path, data: &[u8]) -> io::Result<()> {
    let parent = path.parent().filter(|p| !p.as_os_str().is_empty());
    if let Some(parent) = parent {
        std::fs::create_dir_all(parent)?;
    }

    // A unique sibling prevents concurrent writers from truncating or
    // renaming each other's staging file. `create_new` also makes ownership
    // explicit, which lets recovery code distinguish our debris safely.
    let (tmp, mut file) = create_staging_file(path)?;
    if let Err(error) = file.write_all(data).and_then(|()| file.sync_all()) {
        drop(file);
        let _ = std::fs::remove_file(&tmp);
        return Err(error);
    }
    drop(file);

    // Rename is atomic on the same filesystem (POSIX guarantee).
    // If rename fails, clean up the temp file best-effort.
    if let Err(e) = std::fs::rename(&tmp, path) {
        let _ = std::fs::remove_file(&tmp);
        return Err(e);
    }

    // The file sync makes the contents durable; syncing the containing
    // directory makes the rename itself durable across a power loss.
    sync_parent_dir(parent)?;

    Ok(())
}

fn create_staging_file(path: &Path) -> io::Result<(std::path::PathBuf, std::fs::File)> {
    // Process IDs can be reused after a crash, so an owned-looking debris file
    // may already occupy the first sequence. Never truncate it: advance to a
    // fresh name and leave recovery/quarantine policy to the owning subsystem.
    for _ in 0..64 {
        let tmp = tmp_path_for(path);
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&tmp)
        {
            Ok(file) => return Ok((tmp, file)),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(error),
        }
    }
    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "could not allocate a unique atomic-write staging file",
    ))
}

/// Derive a unique, process-owned sibling staging path from `target`.
fn tmp_path_for(target: &Path) -> std::path::PathBuf {
    let sequence = TEMP_FILE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let mut name = target
        .file_name()
        .map(OsString::from)
        .unwrap_or_else(|| OsString::from("artifact"));
    name.push(format!(".tmp.{}.{sequence}", std::process::id()));
    target.with_file_name(name)
}

#[cfg(unix)]
fn sync_parent_dir(parent: Option<&Path>) -> io::Result<()> {
    if let Some(parent) = parent {
        std::fs::File::open(parent)?.sync_all()?;
    }
    Ok(())
}

#[cfg(not(unix))]
fn sync_parent_dir(_parent: Option<&Path>) -> io::Result<()> {
    // Opening a directory as a file is not portable. The data file has still
    // been synced above; platforms without directory fsync use rename's
    // native durability guarantees.
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn atomic_write_json_round_trip() {
        let dir = TempDir::new().expect("tempdir");
        let path = dir.path().join("data.json");

        let value = serde_json::json!({"key": "value", "num": 42});
        atomic_write_json(&path, &value).expect("write");

        let read_back: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&path).expect("read")).expect("parse");
        assert_eq!(read_back, value);

        // Temp file should not linger.
        assert_eq!(
            std::fs::read_dir(dir.path())
                .expect("read tempdir")
                .filter_map(Result::ok)
                .filter(|entry| entry.file_name().to_string_lossy().contains(".tmp."))
                .count(),
            0
        );
    }

    #[test]
    fn atomic_write_bytes_creates_parent_dirs() {
        let dir = TempDir::new().expect("tempdir");
        let path = dir.path().join("deep/nested/dir/output.bin");

        atomic_write_bytes(&path, b"hello world").expect("write");
        assert_eq!(std::fs::read(&path).expect("read"), b"hello world");
    }

    #[test]
    fn atomic_write_bytes_overwrites_existing() {
        let dir = TempDir::new().expect("tempdir");
        let path = dir.path().join("data.bin");

        atomic_write_bytes(&path, b"first").expect("write 1");
        atomic_write_bytes(&path, b"second").expect("write 2");
        assert_eq!(std::fs::read(&path).expect("read"), b"second");
    }

    #[test]
    fn atomic_write_does_not_overwrite_existing_staging_debris() {
        let dir = TempDir::new().expect("tempdir");
        let path = dir.path().join("data.bin");
        let sequence = TEMP_FILE_SEQUENCE.load(Ordering::Relaxed);
        let debris = path.with_file_name(format!("data.bin.tmp.{}.{sequence}", std::process::id()));
        std::fs::write(&debris, b"forensic debris").expect("seed debris");

        atomic_write_bytes(&path, b"new state").expect("write around debris");

        assert_eq!(std::fs::read(&path).expect("target"), b"new state");
        assert_eq!(
            std::fs::read(&debris).expect("debris preserved"),
            b"forensic debris"
        );
    }

    #[test]
    fn concurrent_atomic_writers_never_publish_partial_content() {
        let dir = TempDir::new().expect("tempdir");
        let path = std::sync::Arc::new(dir.path().join("data.bin"));
        let values = (0_u8..8)
            .map(|byte| vec![byte; 32 * 1024])
            .collect::<Vec<_>>();
        let writers = values
            .iter()
            .cloned()
            .map(|value| {
                let path = std::sync::Arc::clone(&path);
                std::thread::spawn(move || atomic_write_bytes(&path, &value))
            })
            .collect::<Vec<_>>();
        for writer in writers {
            writer.join().expect("writer thread").expect("atomic write");
        }

        let published = std::fs::read(&*path).expect("published target");
        assert!(values.contains(&published));
        let staging_count = std::fs::read_dir(dir.path())
            .expect("read dir")
            .filter_map(Result::ok)
            .filter(|entry| entry.file_name().to_string_lossy().contains(".tmp."))
            .count();
        assert_eq!(staging_count, 0);
    }

    #[test]
    fn tmp_path_for_creates_unique_owned_sibling() {
        let p = Path::new("/a/b/router.json");
        let first = tmp_path_for(p);
        let second = tmp_path_for(p);
        assert_eq!(first.parent(), p.parent());
        assert!(
            first
                .file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("router.json.tmp.")
        );
        assert_ne!(first, second);
    }
}
