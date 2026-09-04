//! Gate input fingerprinting — deterministic identity of a task worktree's
//! base commit plus all tracked and untracked owned bytes.
//!
//! Extracted from `gate_dispatch.rs` to isolate filesystem hashing from the
//! gate execution pipeline.

use std::fs::OpenOptions;
use std::io::Read;
use std::path::Path;

use sha2::{Digest, Sha256};
use tokio::process::Command;
use tokio::time::{Duration, timeout};

pub(super) const MAX_UNTRACKED_FILES: usize = 1024;
pub(super) const MAX_UNTRACKED_FILE_BYTES: u64 = 8 * 1024 * 1024;
pub(super) const MAX_GATE_INPUT_BYTES: u64 = 32 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct GateInputSnapshot(
    /// Base commit OID.
    pub(super) String,
    /// SHA-256 digest of the owned diff.
    pub(super) [u8; 32],
    /// Whether the worktree has any owned diff (tracked changes or untracked files).
    pub(super) bool,
);

fn hash_part(hasher: &mut Sha256, bytes: &[u8]) {
    hasher.update((bytes.len() as u64).to_le_bytes());
    hasher.update(bytes);
}

#[cfg(unix)]
pub(super) fn metadata_unchanged(
    before: &std::fs::Metadata,
    after: &std::fs::Metadata,
) -> bool {
    use std::os::unix::fs::MetadataExt;
    before.file_type() == after.file_type()
        && before.len() == after.len()
        && before.modified().ok() == after.modified().ok()
        && before.dev() == after.dev()
        && before.ino() == after.ino()
        && before.mode() == after.mode()
}

#[cfg(unix)]
fn metadata_mode(metadata: &std::fs::Metadata) -> u32 {
    use std::os::unix::fs::MetadataExt;
    metadata.mode()
}

#[cfg(not(unix))]
fn metadata_mode(_metadata: &std::fs::Metadata) -> u32 {
    0
}

pub(super) fn gate_input_snapshot_blocking(
    workdir: &Path,
) -> Result<GateInputSnapshot, String> {
    #[cfg(not(unix))]
    return Err("stable gate input identity is unavailable on this platform".into());
    let git = |args: &[&str]| {
        let output = std::process::Command::new("git")
            .args(args)
            .current_dir(workdir)
            .env("GIT_TERMINAL_PROMPT", "0")
            .output()
            .map_err(|error| error.to_string())?;
        if output.status.success() {
            Ok(output.stdout)
        } else {
            Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
        }
    };
    let base_commit = String::from_utf8_lossy(&git(&["rev-parse", "HEAD"])?)
        .trim()
        .to_string();
    let diff = git(&["diff", "--binary", "HEAD", "--"])?;
    if diff.len() as u64 > MAX_GATE_INPUT_BYTES {
        return Err("tracked diff exceeds gate input byte limit".into());
    }
    let status = git(&[
        "status",
        "--porcelain=v1",
        "-z",
        "--ignored=matching",
        "-uall",
    ])?;
    crate::orchestrator::worktree::validate_workspace_file_kinds(workdir, &status)
        .map_err(|error| error.to_string())?;
    let untracked = git(&["ls-files", "--others", "--exclude-standard", "-z"])?;
    let mut hasher = Sha256::new();
    hash_part(&mut hasher, base_commit.as_bytes());
    hash_part(&mut hasher, &diff);
    let mut total_bytes = diff.len() as u64;
    for (index, raw_path) in untracked
        .split(|byte| *byte == 0)
        .filter(|path| !path.is_empty())
        .enumerate()
    {
        if index >= MAX_UNTRACKED_FILES {
            return Err("untracked file count exceeds input limit".into());
        }
        let relative = std::str::from_utf8(raw_path).map_err(|error| error.to_string())?;
        let path = workdir.join(relative);
        let before = std::fs::symlink_metadata(&path).map_err(|error| error.to_string())?;
        hash_part(&mut hasher, raw_path);
        hasher.update(metadata_mode(&before).to_le_bytes());
        if before.file_type().is_symlink() {
            let target_path = std::fs::read_link(&path).map_err(|error| error.to_string())?;
            let target = target_path.as_os_str().as_encoded_bytes();
            total_bytes = total_bytes.saturating_add(target.len() as u64);
            if target.len() as u64 > MAX_UNTRACKED_FILE_BYTES || total_bytes > MAX_GATE_INPUT_BYTES
            {
                return Err("untracked symlink exceeds input limit".into());
            }
            hasher.update([b'l']);
            hash_part(&mut hasher, target);
            if std::fs::read_link(&path).ok().as_ref() != Some(&target_path) {
                return Err("untracked symlink changed while hashing".into());
            }
        } else if before.is_file() {
            if before.len() > MAX_UNTRACKED_FILE_BYTES
                || total_bytes.saturating_add(before.len()) > MAX_GATE_INPUT_BYTES
            {
                return Err("untracked file exceeds input limit".into());
            }
            let mut options = OpenOptions::new();
            options.read(true);
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
            }
            let mut file = options.open(&path).map_err(|error| error.to_string())?;
            let opened = file.metadata().map_err(|error| error.to_string())?;
            #[cfg(unix)]
            if !opened.is_file() || !metadata_unchanged(&before, &opened) {
                return Err("untracked file changed before hashing".into());
            }
            hasher.update([b'f']);
            hasher.update(before.len().to_le_bytes());
            let read_bytes = std::io::copy(&mut (&mut file).take(before.len() + 1), &mut hasher)
                .map_err(|error| error.to_string())?;
            let after = std::fs::symlink_metadata(&path).map_err(|error| error.to_string())?;
            #[cfg(unix)]
            if read_bytes != before.len() || !metadata_unchanged(&before, &after) {
                return Err("untracked file changed while hashing".into());
            }
            let _ = read_bytes;
            let _ = after;
            total_bytes += before.len();
        } else {
            return Err("untracked path is not a regular file or symlink".into());
        }
    }
    let owned_diff: [u8; 32] = hasher.finalize().into();
    let has_owned_diff = !diff.is_empty() || !untracked.is_empty();
    Ok(GateInputSnapshot(base_commit, owned_diff, has_owned_diff))
}

pub(super) async fn gate_input_snapshot(
    workdir: std::path::PathBuf,
) -> Result<GateInputSnapshot, String> {
    tokio::task::spawn_blocking(move || gate_input_snapshot_blocking(&workdir))
        .await
        .map_err(|error| error.to_string())?
}

/// Fetch the `git diff HEAD` output for the LlmJudge gate.
///
/// Runs `git diff HEAD -- .` in a blocking task with a bounded 5 s timeout.
/// Returns `None` on any error or timeout so the caller can fall back to
/// description-only evaluation.
pub(super) async fn fetch_git_diff(workdir: &Path) -> Option<String> {
    let workdir = workdir.to_path_buf();
    tokio::time::timeout(
        Duration::from_secs(5),
        tokio::task::spawn_blocking(move || {
            std::process::Command::new("git")
                .args(["diff", "HEAD", "--", "."])
                .current_dir(&workdir)
                .env("GIT_TERMINAL_PROMPT", "0")
                .output()
                .ok()
                .and_then(|o| {
                    if o.status.success() {
                        String::from_utf8(o.stdout).ok()
                    } else {
                        None
                    }
                })
        }),
    )
    .await
    .ok()
    .and_then(|r| r.ok())
    .flatten()
}

/// Stable identity of a task worktree's base commit plus all tracked and
/// untracked owned bytes. Reflex promotion reuses the same attribution proof
/// as the gate so an isolated replay can be compared with the Premium source
/// attempt without inventing a weaker diff format.
pub(crate) async fn reflex_input_fingerprint(
    workdir: std::path::PathBuf,
) -> Result<(String, [u8; 32], bool), String> {
    let GateInputSnapshot(base, digest, has_owned_diff) = gate_input_snapshot(workdir).await?;
    Ok((base, digest, has_owned_diff))
}

pub(super) fn gate_input_fingerprint_id(snapshot: &GateInputSnapshot) -> String {
    let mut identity = Sha256::new();
    hash_part(&mut identity, snapshot.0.as_bytes());
    identity.update(snapshot.1);
    identity.update([u8::from(snapshot.2)]);
    format!("{:x}", identity.finalize())
}

/// Combined identity of the immutable base plus every tracked/untracked byte
/// and mode in a task checkout.
pub(crate) async fn owned_input_fingerprint_id(
    workdir: std::path::PathBuf,
) -> Result<String, String> {
    let snapshot = gate_input_snapshot(workdir).await?;
    snapshot
        .2
        .then(|| gate_input_fingerprint_id(&snapshot))
        .ok_or_else(|| "worktree has no owned diff to fingerprint".to_string())
}

pub(super) async fn accepted_input_snapshot(
    workdir: std::path::PathBuf,
    expected_oid: &str,
) -> Result<GateInputSnapshot, String> {
    let snapshot = gate_input_snapshot(workdir).await?;
    (snapshot.0 == expected_oid && !snapshot.2)
        .then_some(snapshot)
        .ok_or_else(|| "accepted plan input differs from immutable commit".into())
}
