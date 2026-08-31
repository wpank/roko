//! Safe filesystem names for per-run append-only indexes.
//!
//! Run identifiers are never interpolated into paths. The validated identifier
//! is hashed and the digest is used as the only variable path component. This
//! keeps the writer and read-only HTTP surfaces in agreement without exposing
//! identifiers in directory listings or permitting traversal.

use std::fmt::Write as _;
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
}
