//! Safe lifecycle management for Rust build artifacts and bounded run caches.
//!
//! Cleanup is deliberately outside the build/run critical path. A scan is
//! read-only; mutation requires an explicit apply request and non-blocking
//! ownership of the cache-GC, workspace, and Cargo advisory locks.

use fs2::FileExt as _;
use serde::Serialize;
use std::collections::BTreeMap;
use std::fs::{File, OpenOptions};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

static CARGO_CLEAN_LOCK: OnceLock<tokio::sync::Mutex<()>> = OnceLock::new();

const GIB: u64 = 1024 * 1024 * 1024;
const MIB: u64 = 1024 * 1024;
const DAY_SECS: u64 = 86_400;
const MAX_PROTECTED_FINDINGS: usize = 128;

/// Metadata for a discovered `target/` directory.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TargetDir {
    /// Absolute path to the `target/` directory.
    pub path: PathBuf,
    /// Total size of the directory tree in bytes.
    pub size_bytes: u64,
    /// Last-modified time of the directory itself (mtime).
    #[serde(skip)]
    pub last_modified: SystemTime,
}

impl TargetDir {
    /// Age of this directory in whole days (rounded down).
    #[must_use]
    pub fn age_days(&self) -> u32 {
        u32::try_from(age_secs(self.last_modified) / DAY_SECS).unwrap_or(u32::MAX)
    }
}

/// Backwards-compatible summary of an age-based target cleanup pass.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct CleanupReport {
    /// Number of `target/` directories found during the scan.
    pub dirs_scanned: usize,
    /// Number of directories successfully removed.
    pub dirs_removed: usize,
    /// Total bytes freed by the removal.
    pub bytes_freed: u64,
}

/// Rebuild impact of deleting a candidate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ColdBuildRisk {
    /// No build output is affected.
    None,
    /// Incremental state is lost, while compiled dependencies remain warm.
    Low,
    /// A dead worktree's complete target is removed.
    Medium,
}

/// Kind of bounded cache entry selected for cleanup.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CacheCandidateKind {
    /// One Cargo incremental compilation partition.
    TargetIncremental,
    /// The complete target of a Git-unregistered runner worktree.
    OrphanTarget,
    /// A terminal, non-current run evidence bundle.
    EvidenceRun,
    /// An immutable rotated JSONL generation.
    LogArchive,
    /// One context-pack cache entry.
    ContextCache,
}

/// One safe, bounded cleanup candidate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CacheCleanupCandidate {
    /// Candidate category.
    pub kind: CacheCandidateKind,
    /// Absolute candidate path.
    pub path: PathBuf,
    /// Allocated size observed during the scan.
    pub size_bytes: u64,
    /// Age at scan time, rounded down.
    pub age_hours: u64,
    /// Expected effect on the next Rust build.
    pub cold_build_risk: ColdBuildRisk,
    /// Human-readable selection explanation.
    pub reason: String,
}

/// A path intentionally retained by policy or because a live owner exists.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProtectedCachePath {
    /// Absolute retained path.
    pub path: PathBuf,
    /// Policy or ownership reason for retention.
    pub reason: String,
}

/// Size, age, and revision-aware cache cleanup policy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CacheCleanupPolicy {
    /// Combined target budget across Git-authoritative worktrees.
    pub target_budget_bytes: u64,
    /// Completed evidence-run budget under `.roko/runs`.
    pub evidence_budget_bytes: u64,
    /// Context-pack cache budget.
    pub context_cache_budget_bytes: u64,
    /// Minimum age before rebuildable incremental state is eligible.
    pub min_incremental_age: Duration,
    /// Maximum age of terminal evidence runs and immutable log archives.
    pub max_evidence_age: Duration,
    /// Always retain this many newest terminal evidence runs.
    pub preserve_evidence_runs: usize,
    /// Always retain this many newest incremental partitions per target.
    pub preserve_incremental_entries: usize,
    /// Git revision whose evidence must be retained.
    pub current_revision: Option<String>,
}

impl Default for CacheCleanupPolicy {
    fn default() -> Self {
        Self {
            target_budget_bytes: 96 * GIB,
            evidence_budget_bytes: 2 * GIB,
            context_cache_budget_bytes: GIB,
            min_incremental_age: Duration::from_hours(6),
            max_evidence_age: Duration::from_secs(14 * DAY_SECS),
            preserve_evidence_runs: 10,
            preserve_incremental_entries: 8,
            current_revision: None,
        }
    }
}

/// Result of a read-only cache scan or an explicit apply pass.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct CacheCleanupReport {
    /// True when no mutation was requested.
    pub dry_run: bool,
    /// Combined size of discovered worktree targets.
    pub target_bytes: u64,
    /// Combined size of run evidence.
    pub evidence_bytes: u64,
    /// Combined size of context-pack cache entries.
    pub context_cache_bytes: u64,
    /// Combined size of immutable log archives.
    pub log_archive_bytes: u64,
    /// Ordered entries selected by the policy.
    pub candidates: Vec<CacheCleanupCandidate>,
    /// Bounded list of explicitly retained entries.
    pub protected: Vec<ProtectedCachePath>,
    /// Bytes projected to be reclaimable.
    pub eligible_bytes: u64,
    /// Number of entries removed by an apply pass.
    pub removed_count: usize,
    /// Bytes actually reclaimed by an apply pass.
    pub reclaimed_bytes: u64,
    /// Entries skipped or failed after the scan.
    pub failed_count: usize,
}

#[derive(Debug, Clone)]
struct WorktreeTarget {
    checkout: PathBuf,
    target: PathBuf,
    head: Option<String>,
    git_authoritative: bool,
}

/// Scan every Git-authoritative checkout target without walking source trees.
///
/// `.roko/worktrees/*` is also inspected one level deep so dead runner
/// checkouts remain visible, but symlink entries are always ignored.
pub async fn scan_target_dirs(root: &Path) -> std::io::Result<Vec<TargetDir>> {
    let root = root.to_path_buf();
    tokio::task::spawn_blocking(move || {
        let root = canonical_real_dir(&root)?;
        let targets = discover_worktree_targets(&root)?;
        let mut results = Vec::new();
        for entry in targets {
            let Ok(metadata) = std::fs::symlink_metadata(&entry.target) else {
                continue;
            };
            if metadata.file_type().is_symlink() || !metadata.is_dir() {
                continue;
            }
            results.push(TargetDir {
                size_bytes: dir_size_bytes(&entry.target),
                last_modified: metadata.modified().unwrap_or(UNIX_EPOCH),
                path: entry.target,
            });
        }
        results.sort_by_key(|target| target.last_modified);
        Ok(results)
    })
    .await
    .map_err(|error| std::io::Error::other(format!("target scan task failed: {error}")))?
}

/// Legacy age cleanup retained for runner compatibility.
///
/// It now removes only targets belonging to Git-unregistered runner
/// worktrees. Live/current worktrees and their warm caches are never selected.
pub async fn clean_stale_targets(root: &Path, max_age_days: u32) -> std::io::Result<CleanupReport> {
    let root = root.to_path_buf();
    tokio::task::spawn_blocking(move || {
        let root = canonical_real_dir(&root)?;
        let targets = discover_worktree_targets(&root)?;
        let mut report = CleanupReport::default();
        for entry in targets {
            let Ok(metadata) = std::fs::symlink_metadata(&entry.target) else {
                continue;
            };
            if metadata.file_type().is_symlink() || !metadata.is_dir() {
                continue;
            }
            report.dirs_scanned += 1;
            if entry.git_authoritative
                || workspace_is_active(&entry.checkout)
                || age_secs(metadata.modified().unwrap_or(UNIX_EPOCH))
                    < u64::from(max_age_days) * DAY_SECS
            {
                continue;
            }
            let Some(_locks) = try_lock_target_profiles(&entry.target)? else {
                continue;
            };
            validate_removal_path(&entry.target, std::slice::from_ref(&entry.checkout))?;
            let size = dir_size_bytes(&entry.target);
            match std::fs::remove_dir_all(&entry.target) {
                Ok(()) => {
                    report.dirs_removed += 1;
                    report.bytes_freed = report.bytes_freed.saturating_add(size);
                }
                Err(error) => tracing::warn!(path = %entry.target.display(), %error, "stale target cleanup failed"),
            }
        }
        Ok(report)
    })
    .await
    .map_err(|error| std::io::Error::other(format!("target cleanup task failed: {error}")))?
}

/// Inspect bounded build/evidence/log caches and optionally remove candidates.
///
/// `apply=false` is guaranteed read-only. `apply=true` acquires a global
/// cache-GC lease and then rechecks a relevant workspace/Cargo lock before
/// every removal. Lock contention is reported as protected state, never
/// waited on, so cleanup cannot join the task critical path.
pub async fn cleanup_workspace_caches(
    root: &Path,
    mut policy: CacheCleanupPolicy,
    apply: bool,
) -> std::io::Result<CacheCleanupReport> {
    let root = root.to_path_buf();
    tokio::task::spawn_blocking(move || {
        let root = canonical_real_dir(&root)?;
        let roko_dir = prepare_roko_dir(&root, apply)?;
        if policy.current_revision.is_none() {
            policy.current_revision = git_stdout(&root, &["rev-parse", "--verify", "HEAD"]);
        }

        let _gc_lock = if apply {
            Some(acquire_gc_lock(
                roko_dir
                    .as_deref()
                    .ok_or_else(|| std::io::Error::other("missing workspace state directory"))?,
            )?)
        } else {
            None
        };
        let mut report = scan_workspace_caches_sync(&root, &policy)?;
        report.dry_run = !apply;
        if !apply {
            return Ok(report);
        }

        let allowed_roots = allowed_cleanup_roots(&root, roko_dir.as_deref())?;
        let candidates = report.candidates.clone();
        for candidate in candidates {
            if candidate_owner_active(&root, &candidate) {
                push_protected(
                    &mut report,
                    candidate.path.clone(),
                    "live workspace owner acquired after scan",
                );
                report.failed_count += 1;
                continue;
            }

            let locks = match locks_for_candidate(&root, &candidate)? {
                Some(locks) => locks,
                None => {
                    push_protected(
                        &mut report,
                        candidate.path.clone(),
                        "Cargo or log writer lock is active",
                    );
                    report.failed_count += 1;
                    continue;
                }
            };
            validate_removal_path(&candidate.path, &allowed_roots)?;
            let result = match std::fs::symlink_metadata(&candidate.path) {
                Ok(metadata) if metadata.is_dir() => std::fs::remove_dir_all(&candidate.path),
                Ok(_) => std::fs::remove_file(&candidate.path),
                Err(error) => Err(error),
            };
            drop(locks);
            match result {
                Ok(()) => {
                    report.removed_count += 1;
                    report.reclaimed_bytes = report
                        .reclaimed_bytes
                        .saturating_add(candidate.size_bytes);
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => {
                    report.failed_count += 1;
                    tracing::warn!(path = %candidate.path.display(), %error, "cache candidate removal failed");
                }
            }
        }
        Ok(report)
    })
    .await
    .map_err(|error| std::io::Error::other(format!("cache cleanup task failed: {error}")))?
}

/// Run `cargo clean` only for a cold, inactive worktree target.
///
/// This compatibility function no longer destroys the target produced by the
/// task that just completed. Fresh targets are retained for subsequent tasks.
pub async fn cargo_clean(workdir: &Path) -> std::io::Result<()> {
    let explicitly_enabled = std::env::var("ROKO_EXPLICIT_CARGO_CLEAN").is_ok_and(|value| {
        matches!(
            value.trim().to_ascii_lowercase().as_str(),
            "1" | "true" | "yes" | "on"
        )
    });
    if !explicitly_enabled {
        tracing::debug!(
            workdir = %workdir.display(),
            "preserving warm target; set ROKO_EXPLICIT_CARGO_CLEAN=1 for an explicit cold clean"
        );
        return Ok(());
    }
    let target = workdir.join("target");
    let metadata = match tokio::fs::symlink_metadata(&target).await {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error),
    };
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(std::io::Error::other(
            "refusing cargo clean for non-directory target",
        ));
    }
    if age_secs(metadata.modified().unwrap_or(UNIX_EPOCH)) < DAY_SECS {
        tracing::debug!(path = %target.display(), "preserving fresh target cache");
        return Ok(());
    }

    let _clean_guard = CARGO_CLEAN_LOCK
        .get_or_init(|| tokio::sync::Mutex::new(()))
        .lock()
        .await;
    let output = tokio::process::Command::new("cargo")
        .arg("clean")
        .current_dir(workdir)
        .output()
        .await?;
    if output.status.success() {
        return Ok(());
    }
    Err(std::io::Error::other(format!(
        "cargo clean failed in {}: {}",
        workdir.display(),
        String::from_utf8_lossy(&output.stderr).trim()
    )))
}

fn scan_workspace_caches_sync(
    root: &Path,
    policy: &CacheCleanupPolicy,
) -> std::io::Result<CacheCleanupReport> {
    let mut report = CacheCleanupReport::default();
    let targets = discover_worktree_targets(root)?;
    let mut incremental = Vec::new();

    for worktree in &targets {
        let Ok(metadata) = std::fs::symlink_metadata(&worktree.target) else {
            continue;
        };
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            continue;
        }
        let target_size = dir_size_bytes(&worktree.target);
        report.target_bytes = report.target_bytes.saturating_add(target_size);

        if workspace_is_active(&worktree.checkout) {
            push_protected(
                &mut report,
                worktree.target.clone(),
                "workspace/runner lock is active",
            );
            continue;
        }
        if !worktree.git_authoritative {
            let modified = metadata.modified().unwrap_or(UNIX_EPOCH);
            if age_secs(modified) >= policy.min_incremental_age.as_secs() {
                incremental.push(CacheCleanupCandidate {
                    kind: CacheCandidateKind::OrphanTarget,
                    path: worktree.target.clone(),
                    size_bytes: target_size,
                    age_hours: age_secs(modified) / 3600,
                    cold_build_risk: ColdBuildRisk::Medium,
                    reason: "Git-unregistered runner worktree target".to_string(),
                });
            }
            continue;
        }
        scan_incremental_entries(worktree, policy, &mut incremental, &mut report)?;
    }

    incremental.sort_by_key(|candidate| std::cmp::Reverse(candidate.age_hours));
    let mut pressure = report
        .target_bytes
        .saturating_sub(policy.target_budget_bytes);
    for candidate in incremental {
        if candidate.kind == CacheCandidateKind::OrphanTarget || pressure > 0 {
            pressure = pressure.saturating_sub(candidate.size_bytes);
            report.eligible_bytes = report.eligible_bytes.saturating_add(candidate.size_bytes);
            report.candidates.push(candidate);
        }
    }

    scan_evidence_runs(root, policy, &mut report)?;
    scan_context_cache(root, policy, &mut report)?;
    scan_log_archives(root, policy, &mut report)?;
    report.candidates.sort_by(|left, right| {
        right
            .age_hours
            .cmp(&left.age_hours)
            .then_with(|| left.path.cmp(&right.path))
    });
    Ok(report)
}

fn scan_incremental_entries(
    worktree: &WorktreeTarget,
    policy: &CacheCleanupPolicy,
    output: &mut Vec<CacheCleanupCandidate>,
    report: &mut CacheCleanupReport,
) -> std::io::Result<()> {
    let mut entries = Vec::new();
    for profile in real_child_dirs(&worktree.target)? {
        let incremental_root = profile.join("incremental");
        for path in real_child_dirs(&incremental_root).unwrap_or_default() {
            let metadata = std::fs::symlink_metadata(&path)?;
            let modified = metadata.modified().unwrap_or(UNIX_EPOCH);
            entries.push((modified, path));
        }
    }
    entries.sort_by_key(|(modified, _)| std::cmp::Reverse(*modified));
    for (index, (modified, path)) in entries.into_iter().enumerate() {
        if index < policy.preserve_incremental_entries {
            push_protected(report, path, "newest incremental partition retained");
            continue;
        }
        let age = age_secs(modified);
        if age < policy.min_incremental_age.as_secs() {
            push_protected(report, path, "incremental partition is inside minimum age");
            continue;
        }
        output.push(CacheCleanupCandidate {
            kind: CacheCandidateKind::TargetIncremental,
            size_bytes: dir_size_bytes(&path),
            path,
            age_hours: age / 3600,
            cold_build_risk: ColdBuildRisk::Low,
            reason: format!(
                "target budget {} MiB exceeded; compiled dependencies retained (HEAD {})",
                policy.target_budget_bytes / MIB,
                worktree.head.as_deref().unwrap_or("unknown")
            ),
        });
    }
    Ok(())
}

fn scan_evidence_runs(
    root: &Path,
    policy: &CacheCleanupPolicy,
    report: &mut CacheCleanupReport,
) -> std::io::Result<()> {
    let runs = root.join(".roko/runs");
    let mut terminal = Vec::new();
    for path in real_child_dirs(&runs).unwrap_or_default() {
        let metadata = std::fs::symlink_metadata(&path)?;
        let size = dir_size_bytes(&path);
        report.evidence_bytes = report.evidence_bytes.saturating_add(size);
        let modified = metadata.modified().unwrap_or(UNIX_EPOCH);
        let summary = read_small_json(&path.join("summary.json"));
        let is_terminal = summary
            .as_ref()
            .and_then(|value| value.get("terminal"))
            .and_then(serde_json::Value::as_bool)
            == Some(true);
        if !is_terminal {
            push_protected(report, path, "run has no durable terminal summary");
            continue;
        }
        let revision_matches = summary.as_ref().is_some_and(|value| {
            let git = value.get("git");
            ["head_before", "head_after"].into_iter().any(|field| {
                git.and_then(|value| value.get(field))
                    .and_then(serde_json::Value::as_str)
                    .is_some_and(|revision| Some(revision) == policy.current_revision.as_deref())
            })
        });
        terminal.push((modified, path, size, revision_matches));
    }
    terminal.sort_by_key(|(modified, _, _, _)| std::cmp::Reverse(*modified));
    let mut pressure = report
        .evidence_bytes
        .saturating_sub(policy.evidence_budget_bytes);
    for (index, (modified, path, size, revision_matches)) in terminal.into_iter().enumerate() {
        if index < policy.preserve_evidence_runs || revision_matches {
            push_protected(
                report,
                path,
                if revision_matches {
                    "evidence belongs to current Git revision"
                } else {
                    "newest terminal evidence retained"
                },
            );
            continue;
        }
        let age = age_secs(modified);
        if pressure == 0 && age < policy.max_evidence_age.as_secs() {
            continue;
        }
        pressure = pressure.saturating_sub(size);
        report.eligible_bytes = report.eligible_bytes.saturating_add(size);
        report.candidates.push(CacheCleanupCandidate {
            kind: CacheCandidateKind::EvidenceRun,
            path,
            size_bytes: size,
            age_hours: age / 3600,
            cold_build_risk: ColdBuildRisk::None,
            reason: "terminal evidence exceeds age or size retention".to_string(),
        });
    }
    Ok(())
}

fn scan_context_cache(
    root: &Path,
    policy: &CacheCleanupPolicy,
    report: &mut CacheCleanupReport,
) -> std::io::Result<()> {
    let cache = root.join(".roko/cache/context-pack-cache");
    let mut entries = Vec::new();
    for path in real_child_entries(&cache).unwrap_or_default() {
        let metadata = std::fs::symlink_metadata(&path)?;
        let size = if metadata.is_dir() {
            dir_size_bytes(&path)
        } else {
            metadata.len()
        };
        report.context_cache_bytes = report.context_cache_bytes.saturating_add(size);
        entries.push((metadata.modified().unwrap_or(UNIX_EPOCH), path, size));
    }
    entries.sort_by_key(|(modified, _, _)| std::cmp::Reverse(*modified));
    let mut pressure = report
        .context_cache_bytes
        .saturating_sub(policy.context_cache_budget_bytes);
    for (index, (modified, path, size)) in entries.into_iter().enumerate() {
        if index < 32 || pressure == 0 {
            continue;
        }
        pressure = pressure.saturating_sub(size);
        report.eligible_bytes = report.eligible_bytes.saturating_add(size);
        report.candidates.push(CacheCleanupCandidate {
            kind: CacheCandidateKind::ContextCache,
            path,
            size_bytes: size,
            age_hours: age_secs(modified) / 3600,
            cold_build_risk: ColdBuildRisk::None,
            reason: "context-pack cache exceeds size budget".to_string(),
        });
    }
    Ok(())
}

fn scan_log_archives(
    root: &Path,
    policy: &CacheCleanupPolicy,
    report: &mut CacheCleanupReport,
) -> std::io::Result<()> {
    let layout = crate::RokoLayout::for_project(root);
    for live in crate::log_rotation::rotatable_jsonl_paths(&layout) {
        let Some(parent) = live.parent() else {
            continue;
        };
        let Some(stem) = live.file_stem().and_then(|stem| stem.to_str()) else {
            continue;
        };
        let mut archives = Vec::new();
        for path in real_child_files(parent).unwrap_or_default() {
            let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
                continue;
            };
            if !name.starts_with(&format!("{stem}."))
                || !Path::new(name)
                    .extension()
                    .is_some_and(|extension| extension.eq_ignore_ascii_case("jsonl"))
                || !archive_name_has_timestamp(stem, name)
            {
                continue;
            }
            let metadata = std::fs::symlink_metadata(&path)?;
            report.log_archive_bytes = report.log_archive_bytes.saturating_add(metadata.len());
            archives.push((
                metadata.modified().unwrap_or(UNIX_EPOCH),
                path,
                metadata.len(),
            ));
        }
        archives.sort_by_key(|(modified, _, _)| std::cmp::Reverse(*modified));
        for (index, (modified, path, size)) in archives.into_iter().enumerate() {
            let age = age_secs(modified);
            if index == 0 || age < policy.max_evidence_age.as_secs() {
                continue;
            }
            report.eligible_bytes = report.eligible_bytes.saturating_add(size);
            report.candidates.push(CacheCleanupCandidate {
                kind: CacheCandidateKind::LogArchive,
                path,
                size_bytes: size,
                age_hours: age / 3600,
                cold_build_risk: ColdBuildRisk::None,
                reason: "immutable JSONL generation exceeds retention age".to_string(),
            });
        }
    }
    Ok(())
}

fn discover_worktree_targets(root: &Path) -> std::io::Result<Vec<WorktreeTarget>> {
    let mut by_checkout: BTreeMap<PathBuf, WorktreeTarget> = BTreeMap::new();
    let output = Command::new("git")
        .args(["-C"])
        .arg(root)
        .args(["worktree", "list", "--porcelain"])
        .output();
    if let Ok(output) = output
        && output.status.success()
    {
        let text = String::from_utf8_lossy(&output.stdout);
        let mut checkout = None;
        let mut head = None;
        for line in text.lines().chain(std::iter::once("")) {
            if let Some(value) = line.strip_prefix("worktree ") {
                if let Some(previous) = checkout.take() {
                    insert_worktree_target(&mut by_checkout, previous, head.take(), true);
                }
                checkout = Some(PathBuf::from(value));
            } else if let Some(value) = line.strip_prefix("HEAD ") {
                head = Some(value.to_string());
            } else if line.is_empty()
                && let Some(previous) = checkout.take()
            {
                insert_worktree_target(&mut by_checkout, previous, head.take(), true);
            }
        }
    }
    if by_checkout.is_empty() {
        insert_worktree_target(&mut by_checkout, root.to_path_buf(), None, true);
    }

    for checkout in real_child_dirs(&root.join(".roko/worktrees")).unwrap_or_default() {
        if !by_checkout.contains_key(&checkout) {
            insert_worktree_target(&mut by_checkout, checkout, None, false);
        }
    }
    Ok(by_checkout.into_values().collect())
}

fn insert_worktree_target(
    targets: &mut BTreeMap<PathBuf, WorktreeTarget>,
    checkout: PathBuf,
    head: Option<String>,
    git_authoritative: bool,
) {
    let Ok(checkout) = canonical_real_dir(&checkout) else {
        return;
    };
    targets
        .entry(checkout.clone())
        .or_insert_with(|| WorktreeTarget {
            target: checkout.join("target"),
            checkout,
            head,
            git_authoritative,
        });
}

fn allowed_cleanup_roots(root: &Path, roko_dir: Option<&Path>) -> std::io::Result<Vec<PathBuf>> {
    let mut allowed = discover_worktree_targets(root)?
        .into_iter()
        .map(|entry| entry.checkout)
        .collect::<Vec<_>>();
    if let Some(roko_dir) = roko_dir {
        allowed.push(roko_dir.to_path_buf());
    }
    Ok(allowed)
}

fn prepare_roko_dir(root: &Path, create: bool) -> std::io::Result<Option<PathBuf>> {
    let roko_dir = root.join(".roko");
    match std::fs::symlink_metadata(&roko_dir) {
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_dir() => {
            return Err(std::io::Error::other(format!(
                "workspace state must be a real directory: {}",
                roko_dir.display()
            )));
        }
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound && create => {
            std::fs::create_dir(&roko_dir)?;
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error),
    }
    let canonical = canonical_real_dir(&roko_dir)?;
    if canonical == root || !canonical.starts_with(root) {
        return Err(std::io::Error::other(format!(
            "workspace state escapes cleanup root: {}",
            roko_dir.display()
        )));
    }
    Ok(Some(canonical))
}

fn real_child_dirs(path: &Path) -> std::io::Result<Vec<PathBuf>> {
    real_children(path, Some(true))
}

fn real_child_files(path: &Path) -> std::io::Result<Vec<PathBuf>> {
    real_children(path, Some(false))
}

fn real_child_entries(path: &Path) -> std::io::Result<Vec<PathBuf>> {
    real_children(path, None)
}

fn real_children(path: &Path, directory: Option<bool>) -> std::io::Result<Vec<PathBuf>> {
    let metadata = match std::fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error),
    };
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Ok(Vec::new());
    }
    let mut paths = Vec::new();
    for entry in std::fs::read_dir(path)? {
        let entry = entry?;
        let metadata = std::fs::symlink_metadata(entry.path())?;
        if metadata.file_type().is_symlink() {
            continue;
        }
        if directory.is_none_or(|expected| metadata.is_dir() == expected) {
            paths.push(entry.path());
        }
    }
    Ok(paths)
}

fn canonical_real_dir(path: &Path) -> std::io::Result<PathBuf> {
    let metadata = std::fs::symlink_metadata(path)?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(std::io::Error::other(format!(
            "cleanup root must be a real directory: {}",
            path.display()
        )));
    }
    std::fs::canonicalize(path)
}

fn validate_removal_path(path: &Path, allowed_roots: &[PathBuf]) -> std::io::Result<()> {
    let metadata = std::fs::symlink_metadata(path)?;
    if metadata.file_type().is_symlink() {
        return Err(std::io::Error::other(format!(
            "refusing symlink cleanup candidate: {}",
            path.display()
        )));
    }
    let canonical = std::fs::canonicalize(path)?;
    if !allowed_roots
        .iter()
        .any(|root| canonical.starts_with(root) && canonical != *root)
    {
        return Err(std::io::Error::other(format!(
            "cleanup candidate escapes allowed roots: {}",
            path.display()
        )));
    }
    Ok(())
}

fn dir_size_bytes(path: &Path) -> u64 {
    if let Ok(output) = Command::new("du").arg("-sk").arg(path).output()
        && output.status.success()
        && let Some(kib) = String::from_utf8_lossy(&output.stdout)
            .split_whitespace()
            .next()
            .and_then(|value| value.parse::<u64>().ok())
    {
        return kib.saturating_mul(1024);
    }
    let mut total = 0_u64;
    let mut stack = vec![path.to_path_buf()];
    while let Some(directory) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(directory) else {
            continue;
        };
        for entry in entries.flatten() {
            let Ok(metadata) = std::fs::symlink_metadata(entry.path()) else {
                continue;
            };
            if metadata.file_type().is_symlink() {
                continue;
            }
            if metadata.is_dir() {
                stack.push(entry.path());
            } else {
                total = total.saturating_add(metadata.len());
            }
        }
    }
    total
}

fn age_secs(modified: SystemTime) -> u64 {
    SystemTime::now()
        .duration_since(modified)
        .unwrap_or(Duration::ZERO)
        .as_secs()
}

fn git_stdout(root: &Path, args: &[&str]) -> Option<String> {
    let output = Command::new("git")
        .args(["-C"])
        .arg(root)
        .args(args)
        .output()
        .ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_string())
        .filter(|value| !value.is_empty())
}

fn read_small_json(path: &Path) -> Option<serde_json::Value> {
    let metadata = std::fs::symlink_metadata(path).ok()?;
    if metadata.file_type().is_symlink() || !metadata.is_file() || metadata.len() > 256 * 1024 {
        return None;
    }
    serde_json::from_slice(&std::fs::read(path).ok()?).ok()
}

fn archive_name_has_timestamp(stem: &str, name: &str) -> bool {
    let middle = name
        .strip_prefix(&format!("{stem}."))
        .and_then(|name| name.strip_suffix(".jsonl"));
    middle.is_some_and(|timestamp| {
        (15..=40).contains(&timestamp.len())
            && timestamp
                .chars()
                .all(|character| character.is_ascii_digit() || character == 'T' || character == 'Z')
    })
}

fn try_lock_existing(path: &Path) -> std::io::Result<Option<File>> {
    match std::fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_file() => {
            return Err(std::io::Error::other(format!(
                "refusing non-regular lock file: {}",
                path.display()
            )));
        }
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error),
    }
    let file = match OpenOptions::new().read(true).write(true).open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(path)?,
        Err(error) => return Err(error),
    };
    match file.try_lock_exclusive() {
        Ok(()) => Ok(Some(file)),
        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => Ok(None),
        Err(error) => Err(error),
    }
}

fn workspace_is_active(checkout: &Path) -> bool {
    let lock = checkout.join(".roko/runtime/roko.lock");
    if !lock.exists() {
        return false;
    }
    match try_lock_existing(&lock) {
        Ok(Some(file)) => {
            let _ = file.unlock();
            false
        }
        Ok(None) | Err(_) => true,
    }
}

fn acquire_gc_lock(roko_dir: &Path) -> std::io::Result<File> {
    let runtime = roko_dir.join("runtime");
    match std::fs::symlink_metadata(&runtime) {
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_dir() => {
            return Err(std::io::Error::other(format!(
                "runtime state must be a real directory: {}",
                runtime.display()
            )));
        }
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            std::fs::create_dir(&runtime)?;
        }
        Err(error) => return Err(error),
    }
    let canonical_runtime = canonical_real_dir(&runtime)?;
    if !canonical_runtime.starts_with(roko_dir) || canonical_runtime == roko_dir {
        return Err(std::io::Error::other(
            "runtime state escapes workspace state",
        ));
    }
    try_lock_existing(&canonical_runtime.join("cache-gc.lock"))?
        .ok_or_else(|| std::io::Error::other("another cache cleanup pass is already active"))
}

fn try_lock_target_profiles(target: &Path) -> std::io::Result<Option<Vec<File>>> {
    let mut locks = Vec::new();
    for profile in real_child_dirs(target)? {
        let cargo_lock = profile.join(".cargo-lock");
        let Some(lock) = try_lock_existing(&cargo_lock)? else {
            return Ok(None);
        };
        locks.push(lock);
    }
    Ok(Some(locks))
}

fn locks_for_candidate(
    root: &Path,
    candidate: &CacheCleanupCandidate,
) -> std::io::Result<Option<Vec<File>>> {
    let mut workspace_locks = Vec::new();
    let checkout = match candidate.kind {
        CacheCandidateKind::EvidenceRun
        | CacheCandidateKind::LogArchive
        | CacheCandidateKind::ContextCache => Some(root),
        CacheCandidateKind::TargetIncremental | CacheCandidateKind::OrphanTarget => candidate
            .path
            .ancestors()
            .find(|path| path.join(".git").exists()),
    };
    if let Some(checkout) = checkout {
        let runtime = checkout.join(".roko/runtime");
        if runtime.is_dir() {
            let Some(lock) = try_lock_existing(&runtime.join("roko.lock"))? else {
                return Ok(None);
            };
            workspace_locks.push(lock);
        }
    }

    match candidate.kind {
        CacheCandidateKind::TargetIncremental => {
            let Some(profile) = candidate.path.parent().and_then(Path::parent) else {
                return Ok(None);
            };
            let lock = profile.join(".cargo-lock");
            let Some(lock) = try_lock_existing(&lock)? else {
                return Ok(None);
            };
            workspace_locks.push(lock);
            Ok(Some(workspace_locks))
        }
        CacheCandidateKind::OrphanTarget => {
            let Some(mut cargo_locks) = try_lock_target_profiles(&candidate.path)? else {
                return Ok(None);
            };
            workspace_locks.append(&mut cargo_locks);
            Ok(Some(workspace_locks))
        }
        CacheCandidateKind::LogArchive => {
            let Some(parent) = candidate.path.parent() else {
                return Ok(None);
            };
            let Some(name) = candidate.path.file_name().and_then(|name| name.to_str()) else {
                return Ok(None);
            };
            let Some((stem, _)) = name.split_once('.') else {
                return Ok(None);
            };
            let lock = parent.join(format!("{stem}.jsonl.lock"));
            let Some(lock) = try_lock_existing(&lock)? else {
                return Ok(None);
            };
            workspace_locks.push(lock);
            Ok(Some(workspace_locks))
        }
        CacheCandidateKind::EvidenceRun | CacheCandidateKind::ContextCache => {
            Ok(Some(workspace_locks))
        }
    }
}

fn candidate_owner_active(root: &Path, candidate: &CacheCleanupCandidate) -> bool {
    match candidate.kind {
        CacheCandidateKind::EvidenceRun
        | CacheCandidateKind::LogArchive
        | CacheCandidateKind::ContextCache => workspace_is_active(root),
        CacheCandidateKind::TargetIncremental | CacheCandidateKind::OrphanTarget => candidate
            .path
            .ancestors()
            .find(|path| path.join(".git").exists())
            .is_some_and(workspace_is_active),
    }
}

fn push_protected(report: &mut CacheCleanupReport, path: PathBuf, reason: &str) {
    if report.protected.len() < MAX_PROTECTED_FINDINGS {
        report.protected.push(ProtectedCachePath {
            path,
            reason: reason.to_string(),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn policy() -> CacheCleanupPolicy {
        CacheCleanupPolicy {
            target_budget_bytes: 0,
            evidence_budget_bytes: 0,
            context_cache_budget_bytes: 0,
            min_incremental_age: Duration::ZERO,
            max_evidence_age: Duration::ZERO,
            preserve_evidence_runs: 0,
            preserve_incremental_entries: 0,
            current_revision: Some("current".to_string()),
        }
    }

    #[tokio::test]
    async fn dry_run_never_removes_incremental_state() {
        let root = TempDir::new().unwrap();
        std::fs::create_dir(root.path().join(".git")).unwrap();
        let incremental = root.path().join("target/debug/incremental/old-unit");
        std::fs::create_dir_all(&incremental).unwrap();
        std::fs::write(incremental.join("artifact"), b"data").unwrap();

        let report = cleanup_workspace_caches(root.path(), policy(), false)
            .await
            .unwrap();

        assert!(incremental.exists());
        assert!(report.dry_run);
        assert!(
            report
                .candidates
                .iter()
                .any(|candidate| candidate.path == incremental)
        );
    }

    #[tokio::test]
    async fn apply_prunes_incremental_but_retains_compiled_dependencies() {
        let root = TempDir::new().unwrap();
        std::fs::create_dir(root.path().join(".git")).unwrap();
        let incremental = root.path().join("target/debug/incremental/old-unit");
        let dependency = root.path().join("target/debug/deps/keep.rlib");
        std::fs::create_dir_all(&incremental).unwrap();
        std::fs::create_dir_all(dependency.parent().unwrap()).unwrap();
        std::fs::write(incremental.join("artifact"), b"data").unwrap();
        std::fs::write(&dependency, b"warm").unwrap();

        let report = cleanup_workspace_caches(root.path(), policy(), true)
            .await
            .unwrap();

        assert!(!incremental.exists());
        assert!(dependency.exists());
        assert_eq!(report.removed_count, 1);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn target_symlink_is_never_scanned_or_removed() {
        use std::os::unix::fs::symlink;
        let root = TempDir::new().unwrap();
        let outside = TempDir::new().unwrap();
        std::fs::create_dir(root.path().join(".git")).unwrap();
        std::fs::write(outside.path().join("important"), b"keep").unwrap();
        symlink(outside.path(), root.path().join("target")).unwrap();

        let report = cleanup_workspace_caches(root.path(), policy(), true)
            .await
            .unwrap();

        assert!(report.candidates.is_empty());
        assert!(outside.path().join("important").exists());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn workspace_state_symlink_is_rejected_before_scan_or_apply() {
        use std::os::unix::fs::symlink;
        let root = TempDir::new().unwrap();
        let outside = TempDir::new().unwrap();
        std::fs::create_dir(root.path().join(".git")).unwrap();
        std::fs::write(outside.path().join("important"), b"keep").unwrap();
        symlink(outside.path(), root.path().join(".roko")).unwrap();

        let error = cleanup_workspace_caches(root.path(), policy(), true)
            .await
            .unwrap_err();

        assert!(
            error
                .to_string()
                .contains("workspace state must be a real directory")
        );
        assert!(outside.path().join("important").exists());
    }

    #[tokio::test]
    async fn nonterminal_and_current_revision_evidence_are_preserved() {
        let root = TempDir::new().unwrap();
        std::fs::create_dir(root.path().join(".git")).unwrap();
        let active = root.path().join(".roko/runs/active");
        let current = root.path().join(".roko/runs/current");
        std::fs::create_dir_all(&active).unwrap();
        std::fs::create_dir_all(&current).unwrap();
        std::fs::write(active.join("status.jsonl"), b"started\n").unwrap();
        std::fs::write(
            current.join("summary.json"),
            br#"{"terminal":true,"git":{"head_before":"current"}}"#,
        )
        .unwrap();

        let report = cleanup_workspace_caches(root.path(), policy(), true)
            .await
            .unwrap();

        assert!(active.exists());
        assert!(current.exists());
        assert!(report.protected.iter().any(|entry| entry.path == active));
        assert!(report.protected.iter().any(|entry| entry.path == current));
    }

    #[tokio::test]
    async fn active_workspace_lock_prevents_pruning() {
        let root = TempDir::new().unwrap();
        std::fs::create_dir(root.path().join(".git")).unwrap();
        let incremental = root.path().join("target/debug/incremental/old-unit");
        std::fs::create_dir_all(&incremental).unwrap();
        std::fs::write(incremental.join("artifact"), b"data").unwrap();
        let runtime = root.path().join(".roko/runtime");
        std::fs::create_dir_all(&runtime).unwrap();
        let lock = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(runtime.join("roko.lock"))
            .unwrap();
        lock.lock_exclusive().unwrap();

        let report = cleanup_workspace_caches(root.path(), policy(), true)
            .await
            .unwrap();

        assert!(incremental.exists());
        assert!(
            report
                .protected
                .iter()
                .any(|entry| entry.path == root.path().join("target"))
        );
    }

    #[test]
    fn target_age_is_reported_in_whole_days() {
        let target = TargetDir {
            path: PathBuf::from("/tmp/target"),
            size_bytes: 0,
            last_modified: SystemTime::now() - Duration::from_secs(10 * DAY_SECS),
        };
        assert_eq!(target.age_days(), 10);
    }
}
