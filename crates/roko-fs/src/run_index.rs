//! Safe filesystem names for per-run append-only indexes.
//!
//! Run identifiers are never interpolated into paths. The validated identifier
//! is hashed and the digest is used as the only variable path component. This
//! keeps the writer and read-only HTTP surfaces in agreement without exposing
//! identifiers in directory listings or permitting traversal.

use std::fmt::Write as _;
use std::fs::{File, OpenOptions};
use std::io;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

/// Maximum accepted UTF-8 byte length for a run or task identifier.
pub const MAX_SCOPED_ID_BYTES: usize = 128;

/// Validate an identifier used to select run-scoped observability data.
///
/// IDs are deliberately narrower than a general filesystem segment. Existing
/// UUID, slug, plan, and task identifiers fit this grammar; control characters,
/// whitespace, separators, and shell punctuation do not.
pub fn validate_scoped_id(value: &str) -> Result<(), &'static str> {
    if value.is_empty() {
        return Err("must not be empty");
    }
    if value.len() > MAX_SCOPED_ID_BYTES {
        return Err("is too long");
    }
    if !value
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':'))
    {
        return Err("contains unsupported characters");
    }
    if value == "." || value == ".." || value.contains("..") {
        return Err("contains a traversal marker");
    }
    Ok(())
}

/// Resolve the per-run JSONL index beside a global JSONL log.
///
/// For `.roko/events.jsonl`, this returns
/// `.roko/events-by-run/<sha256(run_id)>.jsonl`. The same rule maps
/// `runtime-events.jsonl` to `runtime-events-by-run/`.
pub fn run_index_path(global_log: &Path, run_id: &str) -> Result<PathBuf, &'static str> {
    validate_scoped_id(run_id)?;

    let stem = global_log
        .file_stem()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
        .ok_or("global log has no UTF-8 file stem")?;
    let parent = global_log.parent().ok_or("global log has no parent")?;
    let digest = Sha256::digest(run_id.as_bytes());
    let mut encoded = String::with_capacity(digest.len() * 2);
    for byte in digest {
        let _ = write!(&mut encoded, "{byte:02x}");
    }

    Ok(parent
        .join(format!("{stem}-by-run"))
        .join(format!("{encoded}.jsonl")))
}

/// Open a derived run index for append without following a replaced directory
/// or file symlink. The global log's parent must already be a real directory.
pub fn open_run_index_append(global_log: &Path, run_id: &str) -> io::Result<(PathBuf, File)> {
    let path = run_index_path(global_log, run_id).map_err(io::Error::other)?;
    let parent = prepare_index_parent(global_log, &path, true)?;
    reject_index_file_if_present(&path, &parent)?;
    let mut options = OpenOptions::new();
    options.create(true).append(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt as _;
        options.custom_flags(libc::O_NOFOLLOW);
    }
    let file = options.open(&path)?;
    let metadata = file.metadata()?;
    if !metadata.is_file() {
        return Err(io::Error::other(format!(
            "run index is not a regular file: {}",
            path.display()
        )));
    }
    Ok((path, file))
}

/// Open an existing derived run index without following directory/file
/// symlinks. `NotFound` remains distinguishable to callers.
pub fn open_existing_run_index(path: &Path) -> io::Result<File> {
    let parent = path
        .parent()
        .ok_or_else(|| io::Error::other("run index has no parent"))?;
    let parent_metadata = std::fs::symlink_metadata(parent)?;
    if parent_metadata.file_type().is_symlink() || !parent_metadata.is_dir() {
        return Err(io::Error::other(format!(
            "run index parent must be a real directory: {}",
            parent.display()
        )));
    }
    let canonical_parent = std::fs::canonicalize(parent)?;
    reject_index_file_if_present(path, &canonical_parent)?;
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt as _;
        options.custom_flags(libc::O_NOFOLLOW);
    }
    let file = options.open(path)?;
    if !file.metadata()?.is_file() {
        return Err(io::Error::other(format!(
            "run index is not a regular file: {}",
            path.display()
        )));
    }
    Ok(file)
}

fn prepare_index_parent(global_log: &Path, path: &Path, create: bool) -> io::Result<PathBuf> {
    let global_parent = global_log
        .parent()
        .ok_or_else(|| io::Error::other("global log has no parent"))?;
    let metadata = std::fs::symlink_metadata(global_parent)?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(io::Error::other(format!(
            "global log parent must be a real directory: {}",
            global_parent.display()
        )));
    }
    let canonical_global_parent = std::fs::canonicalize(global_parent)?;
    let parent = path
        .parent()
        .ok_or_else(|| io::Error::other("run index has no parent"))?;
    match std::fs::symlink_metadata(parent) {
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_dir() => {
            return Err(io::Error::other(format!(
                "run index parent must be a real directory: {}",
                parent.display()
            )));
        }
        Ok(_) => {}
        Err(error) if error.kind() == io::ErrorKind::NotFound && create => {
            std::fs::create_dir(parent)?;
        }
        Err(error) => return Err(error),
    }
    let canonical_parent = std::fs::canonicalize(parent)?;
    if canonical_parent == canonical_global_parent
        || !canonical_parent.starts_with(&canonical_global_parent)
    {
        return Err(io::Error::other(format!(
            "run index parent escapes global log directory: {}",
            parent.display()
        )));
    }
    Ok(canonical_parent)
}

fn reject_index_file_if_present(path: &Path, canonical_parent: &Path) -> io::Result<()> {
    match std::fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_file() => {
            return Err(io::Error::other(format!(
                "run index must be a real regular file: {}",
                path.display()
            )));
        }
        Ok(_) => {
            let canonical = std::fs::canonicalize(path)?;
            if canonical.parent() != Some(canonical_parent) {
                return Err(io::Error::other(format!(
                    "run index escapes expected directory: {}",
                    path.display()
                )));
            }
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(error),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn path_uses_digest_not_identifier() {
        let path = run_index_path(Path::new("/tmp/.roko/events.jsonl"), "run-123")
            .expect("valid path");
        assert_eq!(path.parent().and_then(Path::file_name), Some("events-by-run".as_ref()));
        assert!(!path.to_string_lossy().contains("run-123"));
        assert_eq!(path.extension().and_then(|value| value.to_str()), Some("jsonl"));
    }

    #[test]
    fn rejects_unsafe_or_unbounded_ids() {
        for value in ["", "../escape", "a/b", "a b", "line\nbreak"] {
            assert!(validate_scoped_id(value).is_err(), "accepted {value:?}");
        }
        assert!(validate_scoped_id(&"a".repeat(MAX_SCOPED_ID_BYTES + 1)).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn append_rejects_symlinked_index_directory_and_file() {
        use std::os::unix::fs::symlink;
        let root = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let global = root.path().join("events.jsonl");
        std::fs::write(&global, b"").unwrap();
        symlink(outside.path(), root.path().join("events-by-run")).unwrap();
        assert!(open_run_index_append(&global, "run-1").is_err());

        std::fs::remove_file(root.path().join("events-by-run")).unwrap();
        std::fs::create_dir(root.path().join("events-by-run")).unwrap();
        let path = run_index_path(&global, "run-1").unwrap();
        let outside_file = outside.path().join("outside.jsonl");
        std::fs::write(&outside_file, b"keep").unwrap();
        symlink(&outside_file, &path).unwrap();
        assert!(open_run_index_append(&global, "run-1").is_err());
        assert_eq!(std::fs::read(&outside_file).unwrap(), b"keep");
    }
}
