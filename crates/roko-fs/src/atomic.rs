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

use fs2::FileExt;
use serde::de::DeserializeOwned;
use std::ffi::OsString;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static TEMP_FILE_SEQUENCE: AtomicU64 = AtomicU64::new(0);
const MAX_STRICT_JSON_BYTES: u64 = 64 * 1024 * 1024;

/// Run one strict read/mutate/atomic-write JSON transaction while holding a
/// stable sibling advisory lock.
///
/// The lock file is derived only from `path` (for example,
/// `experiments.json.lock`) and is intentionally retained between calls. A
/// missing target starts from `T::default()`. Any other read failure, including
/// malformed JSON, is returned without modifying the target. The lock remains
/// held while `transaction` runs and until a changed replacement has been
/// atomically published and its parent directory synced. A semantic no-op does
/// not rewrite or reformat the source bytes.
///
/// # Errors
///
/// Returns filesystem/JSON errors converted through `E`, or an error returned
/// by `transaction`. A failed read or transaction leaves the target untouched.
pub fn with_locked_json_transaction<T, R, E, F>(path: &Path, transaction: F) -> Result<R, E>
where
    T: Default + DeserializeOwned + serde::Serialize,
    E: From<io::Error>,
    F: FnOnce(&mut T) -> Result<R, E>,
{
    with_locked_json_transaction_bounded(path, MAX_STRICT_JSON_BYTES, transaction)
}

/// Run a strict locked JSON transaction with a caller-supplied input bound.
///
/// This is equivalent to [`with_locked_json_transaction`] except `max_bytes`
/// constrains the persisted input read while the sibling lock is held. A zero
/// limit is rejected. Subsystems with small control ledgers should use this to
/// prevent a validly addressed but oversized file from consuming unbounded
/// memory before semantic validation.
pub fn with_locked_json_transaction_bounded<T, R, E, F>(
    path: &Path,
    max_bytes: u64,
    transaction: F,
) -> Result<R, E>
where
    T: Default + DeserializeOwned + serde::Serialize,
    E: From<io::Error>,
    F: FnOnce(&mut T) -> Result<R, E>,
{
    if max_bytes == 0 {
        return Err(E::from(io::Error::new(
            io::ErrorKind::InvalidInput,
            "JSON transaction byte limit must be positive",
        )));
    }
    let parent = parent_dir_for(path);
    std::fs::create_dir_all(parent).map_err(E::from)?;

    let lock_path = sibling_lock_path(path);
    let lock = std::fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(lock_path)
        .map_err(E::from)?;
    FileExt::lock_exclusive(&lock).map_err(E::from)?;

    let mut value = read_json_or_default_strict_bounded(path, max_bytes).map_err(E::from)?;
    let before = serde_json::to_vec(&value)
        .map_err(|error| E::from(io::Error::new(io::ErrorKind::InvalidData, error)))?;
    let result = transaction(&mut value)?;
    let after = serde_json::to_vec(&value)
        .map_err(|error| E::from(io::Error::new(io::ErrorKind::InvalidData, error)))?;
    if before != after {
        let persisted = serde_json::to_vec_pretty(&value)
            .map_err(|error| E::from(io::Error::new(io::ErrorKind::InvalidData, error)))?;
        if persisted.len() as u64 > max_bytes {
            return Err(E::from(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("JSON output exceeds the {max_bytes} byte transaction limit"),
            )));
        }
        atomic_write_bytes(path, &persisted).map_err(E::from)?;
    }
    Ok(result)
}

/// Read JSON strictly, returning `T::default()` only when `path` is absent.
///
/// Unlike a permissive "load or new" helper, malformed or unreadable files are
/// surfaced to the caller and are never silently replaced.
///
/// # Errors
///
/// Returns [`io::ErrorKind::InvalidData`] for malformed JSON, or the original
/// filesystem error for failures other than `NotFound`.
pub fn read_json_or_default_strict<T>(path: &Path) -> io::Result<T>
where
    T: Default + DeserializeOwned,
{
    read_json_or_default_strict_bounded(path, MAX_STRICT_JSON_BYTES)
}

/// Read strict JSON with a caller-supplied maximum input size.
///
/// A missing file still returns `T::default()`. The byte bound is checked from
/// metadata and enforced again while reading to handle concurrent growth.
pub fn read_json_or_default_strict_bounded<T>(path: &Path, max_bytes: u64) -> io::Result<T>
where
    T: Default + DeserializeOwned,
{
    if max_bytes == 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "JSON input byte limit must be positive",
        ));
    }
    let file = match std::fs::File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(T::default()),
        Err(error) => return Err(error),
    };
    if file.metadata()?.len() > max_bytes {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "JSON input exceeds the {} byte transaction limit",
                max_bytes
            ),
        ));
    }
    let mut bytes = Vec::new();
    file.take(max_bytes.saturating_add(1))
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > max_bytes {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "JSON input exceeds the {} byte transaction limit",
                max_bytes
            ),
        ));
    }
    serde_json::from_slice(&bytes)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
}

/// Return the stable sibling advisory-lock path for `target`.
#[must_use]
pub fn sibling_lock_path(target: &Path) -> PathBuf {
    let mut name = target
        .file_name()
        .map(OsString::from)
        .unwrap_or_else(|| OsString::from("artifact"));
    name.push(".lock");
    target.with_file_name(name)
}

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
    let parent = parent_dir_for(path);
    std::fs::create_dir_all(parent)?;

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

fn parent_dir_for(path: &Path) -> &Path {
    path.parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."))
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
fn sync_parent_dir(parent: &Path) -> io::Result<()> {
    std::fs::File::open(parent)?.sync_all()
}

#[cfg(not(unix))]
fn sync_parent_dir(_parent: &Path) -> io::Result<()> {
    // Opening a directory as a file is not portable. The data file has still
    // been synced above; platforms without directory fsync use rename's
    // native durability guarantees.
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};
    use tempfile::TempDir;

    #[derive(Debug, Default, Serialize, Deserialize)]
    struct Counter {
        value: u64,
    }

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
    fn bare_relative_target_syncs_current_directory() {
        assert_eq!(parent_dir_for(Path::new("state.json")), Path::new("."));
        assert_eq!(
            parent_dir_for(Path::new("nested/state.json")),
            Path::new("nested")
        );
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
                .expect("temporary path has a file name")
                .to_string_lossy()
                .starts_with("router.json.tmp.")
        );
        assert_ne!(first, second);
    }

    #[test]
    fn locked_json_transaction_defaults_only_when_missing() {
        let dir = TempDir::new().expect("tempdir");
        let path = dir.path().join("state.json");

        let result = with_locked_json_transaction::<Counter, _, io::Error, _>(&path, |counter| {
            assert_eq!(counter.value, 0);
            counter.value = 7;
            Ok(counter.value)
        })
        .expect("transaction");

        assert_eq!(result, 7);
        assert_eq!(
            read_json_or_default_strict::<Counter>(&path)
                .expect("strict load")
                .value,
            7
        );
        assert_eq!(sibling_lock_path(&path), dir.path().join("state.json.lock"));
        assert!(sibling_lock_path(&path).exists());
    }

    #[test]
    fn locked_json_transaction_preserves_malformed_source() {
        let dir = TempDir::new().expect("tempdir");
        let path = dir.path().join("state.json");
        let malformed = b"{ definitely-not-json";
        std::fs::write(&path, malformed).expect("seed malformed source");

        let error = with_locked_json_transaction::<Counter, _, io::Error, _>(&path, |counter| {
            counter.value = 99;
            Ok(())
        })
        .expect_err("malformed JSON must fail closed");

        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
        assert_eq!(std::fs::read(path).expect("preserved source"), malformed);
    }

    #[test]
    fn no_op_locked_transaction_preserves_exact_source_bytes() {
        let dir = TempDir::new().expect("tempdir");
        let path = dir.path().join("state.json");
        let compact = br#"{"value":7}"#;
        std::fs::write(&path, compact).expect("seed compact JSON");

        with_locked_json_transaction::<Counter, _, io::Error, _>(&path, |_counter| Ok(()))
            .expect("no-op transaction");

        assert_eq!(std::fs::read(path).expect("preserved source"), compact);
    }

    #[test]
    fn strict_json_read_rejects_oversized_input_before_allocation() {
        let dir = TempDir::new().expect("tempdir");
        let path = dir.path().join("oversized.json");
        let file = std::fs::File::create(&path).expect("create sparse input");
        file.set_len(MAX_STRICT_JSON_BYTES + 1)
            .expect("extend sparse input");

        let error = read_json_or_default_strict::<Counter>(&path)
            .expect_err("oversized JSON must fail closed");

        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
        assert_eq!(
            std::fs::metadata(path)
                .expect("oversized input remains present")
                .len(),
            MAX_STRICT_JSON_BYTES + 1
        );
    }

    #[test]
    fn bounded_transaction_rejects_oversized_output_before_publish() {
        let dir = TempDir::new().expect("tempdir");
        let path = dir.path().join("bounded.json");

        let error =
            with_locked_json_transaction_bounded::<String, _, io::Error, _>(&path, 32, |value| {
                *value = "x".repeat(64);
                Ok(())
            })
            .expect_err("oversized output must fail closed");

        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
        assert!(!path.exists());
    }

    #[test]
    fn concurrent_locked_json_transactions_do_not_lose_updates() {
        let dir = TempDir::new().expect("tempdir");
        let path = std::sync::Arc::new(dir.path().join("counter.json"));
        let workers = (0..8)
            .map(|_| {
                let path = std::sync::Arc::clone(&path);
                std::thread::spawn(move || {
                    for _ in 0..25 {
                        with_locked_json_transaction::<Counter, _, io::Error, _>(
                            &path,
                            |counter| {
                                counter.value += 1;
                                Ok(())
                            },
                        )?;
                    }
                    Ok::<_, io::Error>(())
                })
            })
            .collect::<Vec<_>>();
        for worker in workers {
            worker.join().expect("worker thread").expect("transactions");
        }

        assert_eq!(
            read_json_or_default_strict::<Counter>(&path)
                .expect("strict load")
                .value,
            200
        );
    }
}
