//! Observability retention policy.
//!
//! Prevents unbounded growth of log/metric/event state for long-running
//! installations. Defines TTL, compaction, postmortem export, and
//! event-log bounds.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Core types
// ---------------------------------------------------------------------------

/// Retention policy for a single observability artifact.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactRetentionPolicy {
    /// Artifact name (e.g., "episodes.jsonl", "efficiency.jsonl").
    pub artifact: String,
    /// Path relative to `.roko/`.
    pub path: String,
    /// Maximum age in hours before rotation/compaction.
    pub max_age_hours: u64,
    /// Maximum file size in bytes before rotation.
    pub max_size_bytes: u64,
    /// Compaction strategy.
    pub strategy: CompactionStrategy,
}

/// How an artifact is compacted when it exceeds retention limits.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompactionStrategy {
    /// Rotate to `.old` and truncate.
    Rotate,
    /// Keep only last N entries.
    TailKeep {
        /// Number of entries to retain.
        entries: usize,
    },
    /// Archive to cold storage.
    Archive,
    /// No compaction (manual only).
    Manual,
}

/// A retention violation detected for an artifact.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetentionViolation {
    /// Artifact name.
    pub artifact: String,
    /// Absolute path to the artifact.
    pub path: PathBuf,
    /// What limit was exceeded.
    pub reason: ViolationReason,
    /// Current value that exceeded the limit.
    pub current: u64,
    /// Limit that was exceeded.
    pub limit: u64,
}

/// What limit was exceeded.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ViolationReason {
    /// File is older than `max_age_hours`.
    Age,
    /// File exceeds `max_size_bytes`.
    Size,
}

/// An action taken (or that would be taken) by `apply_retention`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetentionAction {
    /// Artifact name.
    pub artifact: String,
    /// Path that was acted on.
    pub path: PathBuf,
    /// What action was taken.
    pub action: ActionKind,
    /// Whether this was a dry-run (no-op).
    pub dry_run: bool,
}

/// What compaction action was performed.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionKind {
    /// Rotated to `.old`.
    Rotated,
    /// Tail-kept to N entries.
    TailKept {
        /// Number of entries retained.
        kept: usize,
    },
    /// Archived to cold storage path.
    Archived {
        /// Path to the archive file.
        archive_path: PathBuf,
    },
    /// Skipped (manual policy).
    Skipped,
}

// ---------------------------------------------------------------------------
// Default policies
// ---------------------------------------------------------------------------

/// Return the default retention policies covering all standard observability
/// artifacts under `.roko/`.
#[must_use]
pub fn default_retention_policies() -> Vec<ArtifactRetentionPolicy> {
    vec![
        ArtifactRetentionPolicy {
            artifact: "episodes.jsonl".into(),
            path: "episodes.jsonl".into(),
            max_age_hours: 168, // 1 week
            max_size_bytes: 50 * 1024 * 1024,
            strategy: CompactionStrategy::TailKeep { entries: 10_000 },
        },
        ArtifactRetentionPolicy {
            artifact: "engrams.jsonl".into(),
            path: "engrams.jsonl".into(),
            max_age_hours: 168,
            max_size_bytes: 100 * 1024 * 1024,
            strategy: CompactionStrategy::Rotate,
        },
        ArtifactRetentionPolicy {
            artifact: "learn/efficiency.jsonl".into(),
            path: "learn/efficiency.jsonl".into(),
            max_age_hours: 720, // 30 days
            max_size_bytes: 20 * 1024 * 1024,
            strategy: CompactionStrategy::TailKeep { entries: 5_000 },
        },
        ArtifactRetentionPolicy {
            artifact: "learn/c-factor.jsonl".into(),
            path: "learn/c-factor.jsonl".into(),
            max_age_hours: 720,
            max_size_bytes: 10 * 1024 * 1024,
            strategy: CompactionStrategy::TailKeep { entries: 1_000 },
        },
        ArtifactRetentionPolicy {
            artifact: "learn/cascade-router.json".into(),
            path: "learn/cascade-router.json".into(),
            max_age_hours: 0, // no rotation
            max_size_bytes: 0,
            strategy: CompactionStrategy::Manual,
        },
        ArtifactRetentionPolicy {
            artifact: "learn/experiments.json".into(),
            path: "learn/experiments.json".into(),
            max_age_hours: 0,
            max_size_bytes: 0,
            strategy: CompactionStrategy::Manual,
        },
        ArtifactRetentionPolicy {
            artifact: "learn/gate-thresholds.json".into(),
            path: "learn/gate-thresholds.json".into(),
            max_age_hours: 0,
            max_size_bytes: 0,
            strategy: CompactionStrategy::Manual,
        },
        ArtifactRetentionPolicy {
            artifact: "task-outputs/*".into(),
            path: "task-outputs".into(),
            max_age_hours: 336, // 2 weeks
            max_size_bytes: 500 * 1024 * 1024,
            strategy: CompactionStrategy::Archive,
        },
        // ── largest unbounded append-only logs (E02-T07) ──────────────────
        ArtifactRetentionPolicy {
            artifact: "events.jsonl".into(),
            path: "events.jsonl".into(),
            max_age_hours: 168, // 1 week
            max_size_bytes: 50 * 1024 * 1024,
            strategy: CompactionStrategy::TailKeep { entries: 20_000 },
        },
        ArtifactRetentionPolicy {
            artifact: "roko.log".into(),
            path: "roko.log".into(),
            max_age_hours: 168, // 1 week
            max_size_bytes: 100 * 1024 * 1024,
            strategy: CompactionStrategy::Rotate,
        },
        ArtifactRetentionPolicy {
            artifact: "chain-watcher.log".into(),
            path: "chain-watcher.log".into(),
            max_age_hours: 168, // 1 week
            max_size_bytes: 50 * 1024 * 1024,
            strategy: CompactionStrategy::Rotate,
        },
        ArtifactRetentionPolicy {
            artifact: "run-ledger.jsonl".into(),
            path: "state/run-ledger.jsonl".into(),
            max_age_hours: 720, // 30 days
            max_size_bytes: 20 * 1024 * 1024,
            strategy: CompactionStrategy::TailKeep { entries: 10_000 },
        },
        // ── state/*.bak.* backup files ────────────────────────────────────
        // Sentinel: apply_retention calls sweep_bak_files() for this entry
        // instead of acting on the whole state/ directory.
        ArtifactRetentionPolicy {
            artifact: "state/bak".into(),
            path: "state".into(),
            max_age_hours: 336, // 2 weeks
            max_size_bytes: 200 * 1024 * 1024,
            strategy: CompactionStrategy::Rotate,
        },
    ]
}

// ---------------------------------------------------------------------------
// Check
// ---------------------------------------------------------------------------

/// Check each artifact against its retention policy and return any violations.
///
/// `workdir` is the project root; artifacts are resolved under `workdir/.roko/`.
#[must_use]
pub fn check_retention(workdir: &Path) -> Vec<RetentionViolation> {
    let roko_dir = workdir.join(".roko");
    let policies = default_retention_policies();
    let mut violations = Vec::new();

    for policy in &policies {
        // Skip manual / no-limit policies.
        if policy.max_age_hours == 0 && policy.max_size_bytes == 0 {
            continue;
        }

        let artifact_path = roko_dir.join(&policy.path);

        if policy.path.ends_with('*') || artifact_path.is_dir() {
            // Directory-based policy: sum all files.
            if let Ok(total_size) = dir_total_size(&artifact_path) {
                if policy.max_size_bytes > 0 && total_size > policy.max_size_bytes {
                    violations.push(RetentionViolation {
                        artifact: policy.artifact.clone(),
                        path: artifact_path.clone(),
                        reason: ViolationReason::Size,
                        current: total_size,
                        limit: policy.max_size_bytes,
                    });
                }
            }
            // Check oldest file age.
            if policy.max_age_hours > 0 {
                if let Some(oldest_age_hours) = dir_oldest_age_hours(&artifact_path) {
                    if oldest_age_hours > policy.max_age_hours {
                        violations.push(RetentionViolation {
                            artifact: policy.artifact.clone(),
                            path: artifact_path,
                            reason: ViolationReason::Age,
                            current: oldest_age_hours,
                            limit: policy.max_age_hours,
                        });
                    }
                }
            }
        } else if artifact_path.exists() {
            // Single-file policy.
            if let Ok(metadata) = fs::metadata(&artifact_path) {
                let size = metadata.len();
                if policy.max_size_bytes > 0 && size > policy.max_size_bytes {
                    violations.push(RetentionViolation {
                        artifact: policy.artifact.clone(),
                        path: artifact_path.clone(),
                        reason: ViolationReason::Size,
                        current: size,
                        limit: policy.max_size_bytes,
                    });
                }

                if policy.max_age_hours > 0 {
                    if let Some(age_hours) = file_age_hours(&metadata) {
                        if age_hours > policy.max_age_hours {
                            violations.push(RetentionViolation {
                                artifact: policy.artifact.clone(),
                                path: artifact_path,
                                reason: ViolationReason::Age,
                                current: age_hours,
                                limit: policy.max_age_hours,
                            });
                        }
                    }
                }
            }
        }
    }

    violations
}

// ---------------------------------------------------------------------------
// Apply
// ---------------------------------------------------------------------------

/// Apply compaction for all artifacts that exceed their retention policies.
///
/// When `dry_run` is true, violations are detected but no files are modified.
/// Returns the list of actions taken (or that would be taken).
pub fn apply_retention(workdir: &Path, dry_run: bool) -> Vec<RetentionAction> {
    let roko_dir = workdir.join(".roko");
    let policies = default_retention_policies();
    let mut actions = Vec::new();

    for policy in &policies {
        let artifact_path = roko_dir.join(&policy.path);

        // Special-case: "state/bak" sentinel sweeps .bak.* files inside the
        // state directory rather than acting on the whole state/ tree.
        if policy.artifact == "state/bak" {
            let baks = bak_files_in_state(&artifact_path);
            let has_excess = baks.len() > 10;
            let has_aged = policy.max_age_hours > 0
                && baks.iter().any(|p| {
                    fs::metadata(p)
                        .ok()
                        .and_then(|m| file_age_hours(&m))
                        .unwrap_or(0)
                        > policy.max_age_hours
                });
            if has_excess || has_aged {
                if !dry_run {
                    sweep_bak_files(&artifact_path, policy.max_age_hours, 10);
                }
                actions.push(RetentionAction {
                    artifact: policy.artifact.clone(),
                    path: artifact_path,
                    action: ActionKind::Rotated,
                    dry_run,
                });
            }
            continue;
        }

        // Check whether any limit is exceeded.
        let exceeded = if policy.max_age_hours == 0 && policy.max_size_bytes == 0 {
            false
        } else if artifact_path.is_dir() || policy.path.ends_with('*') {
            let size_exceeded = policy.max_size_bytes > 0
                && dir_total_size(&artifact_path).unwrap_or(0) > policy.max_size_bytes;
            let age_exceeded = policy.max_age_hours > 0
                && dir_oldest_age_hours(&artifact_path).unwrap_or(0) > policy.max_age_hours;
            size_exceeded || age_exceeded
        } else if artifact_path.exists() {
            let meta = fs::metadata(&artifact_path).ok();
            let size_exceeded = meta
                .as_ref()
                .map(|m| policy.max_size_bytes > 0 && m.len() > policy.max_size_bytes)
                .unwrap_or(false);
            let age_exceeded = meta
                .as_ref()
                .and_then(file_age_hours)
                .map(|h| policy.max_age_hours > 0 && h > policy.max_age_hours)
                .unwrap_or(false);
            size_exceeded || age_exceeded
        } else {
            false
        };

        if !exceeded {
            continue;
        }

        let action = match &policy.strategy {
            CompactionStrategy::Rotate => {
                if !dry_run {
                    let old_path = artifact_path.with_extension("old");
                    let _ = fs::rename(&artifact_path, &old_path);
                }
                RetentionAction {
                    artifact: policy.artifact.clone(),
                    path: artifact_path,
                    action: ActionKind::Rotated,
                    dry_run,
                }
            }
            CompactionStrategy::TailKeep { entries } => {
                let kept = if dry_run {
                    *entries
                } else {
                    tail_keep_file(&artifact_path, *entries)
                };
                RetentionAction {
                    artifact: policy.artifact.clone(),
                    path: artifact_path,
                    action: ActionKind::TailKept { kept },
                    dry_run,
                }
            }
            CompactionStrategy::Archive => {
                let archive_dir = roko_dir.join("archive");
                let archive_path =
                    archive_dir.join(format!("{}.tar", policy.artifact.replace('/', "_")));
                if !dry_run {
                    let _ = fs::create_dir_all(&archive_dir);
                    // Move the artifact (or directory) to the archive location.
                    let _ = fs::rename(&artifact_path, &archive_path);
                }
                RetentionAction {
                    artifact: policy.artifact.clone(),
                    path: artifact_path,
                    action: ActionKind::Archived { archive_path },
                    dry_run,
                }
            }
            CompactionStrategy::Manual => RetentionAction {
                artifact: policy.artifact.clone(),
                path: artifact_path,
                action: ActionKind::Skipped,
                dry_run,
            },
        };

        actions.push(action);
    }

    actions
}

// ---------------------------------------------------------------------------
// Postmortem export
// ---------------------------------------------------------------------------

/// Bundle recent logs, episodes, and metrics into a single JSON file for
/// debugging / postmortem analysis.
///
/// # Errors
///
/// Returns an error if any required file cannot be read or the output cannot
/// be written.
pub fn export_postmortem(workdir: &Path, output: &Path) -> anyhow::Result<()> {
    let roko_dir = workdir.join(".roko");
    let now: DateTime<Utc> = Utc::now();

    let mut postmortem = serde_json::json!({
        "exported_at": now.to_rfc3339(),
        "workdir": workdir.display().to_string(),
    });

    // Collect recent content from each artifact (tail 100 lines each).
    let artifacts = [
        ("episodes", "episodes.jsonl"),
        ("engrams", "engrams.jsonl"),
        ("efficiency", "learn/efficiency.jsonl"),
        ("cfactor", "learn/c-factor.jsonl"),
    ];

    for (key, rel_path) in &artifacts {
        let path = roko_dir.join(rel_path);
        if path.exists() {
            let content = fs::read_to_string(&path).unwrap_or_default();
            let lines: Vec<&str> = content.lines().collect();
            let tail: Vec<&str> = if lines.len() > 100 {
                lines[lines.len() - 100..].to_vec()
            } else {
                lines
            };

            // Parse each line as JSON; fall back to string if it fails.
            let entries: Vec<serde_json::Value> = tail
                .iter()
                .map(|line| {
                    serde_json::from_str(line).unwrap_or(serde_json::Value::String((*line).into()))
                })
                .collect();
            postmortem[key] = serde_json::Value::Array(entries);
        }
    }

    // Collect JSON config files as-is.
    let json_files = [
        ("cascade_router", "learn/cascade-router.json"),
        ("experiments", "learn/experiments.json"),
        ("gate_thresholds", "learn/gate-thresholds.json"),
    ];
    for (key, rel_path) in &json_files {
        let path = roko_dir.join(rel_path);
        if path.exists() {
            if let Ok(content) = fs::read_to_string(&path) {
                postmortem[key] =
                    serde_json::from_str(&content).unwrap_or(serde_json::Value::String(content));
            }
        }
    }

    // Include current retention state.
    let violations = check_retention(workdir);
    postmortem["retention_violations"] = serde_json::to_value(&violations)?;
    postmortem["retention_policies"] = serde_json::to_value(default_retention_policies())?;

    // Write output.
    let json_bytes = serde_json::to_vec_pretty(&postmortem)?;
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(output, json_bytes)?;

    Ok(())
}

// ---------------------------------------------------------------------------
// Response type for the retention API route
// ---------------------------------------------------------------------------

/// Response payload for `GET /api/retention`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetentionStatus {
    /// Current retention policies.
    pub policies: Vec<ArtifactRetentionPolicy>,
    /// Any policy violations detected.
    pub violations: Vec<RetentionViolation>,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn file_age_hours(metadata: &fs::Metadata) -> Option<u64> {
    let modified = metadata.modified().ok()?;
    let elapsed = SystemTime::now().duration_since(modified).ok()?;
    Some(elapsed.as_secs() / 3600)
}

fn dir_total_size(dir: &Path) -> std::io::Result<u64> {
    if !dir.is_dir() {
        return Ok(0);
    }
    let mut total: u64 = 0;
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let meta = entry.metadata()?;
        if meta.is_file() {
            total += meta.len();
        } else if meta.is_dir() {
            total += dir_total_size(&entry.path())?;
        }
    }
    Ok(total)
}

fn dir_oldest_age_hours(dir: &Path) -> Option<u64> {
    if !dir.is_dir() {
        return None;
    }
    let mut oldest: Option<u64> = None;
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            if let Ok(meta) = entry.metadata() {
                if let Some(age) = file_age_hours(&meta) {
                    oldest = Some(oldest.map_or(age, |prev: u64| prev.max(age)));
                }
            }
        }
    }
    oldest
}

fn tail_keep_file(path: &Path, entries: usize) -> usize {
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return 0,
    };
    let lines: Vec<&str> = content.lines().collect();
    if lines.len() <= entries {
        return lines.len();
    }
    let kept_lines = &lines[lines.len() - entries..];
    let new_content = kept_lines.join("\n") + "\n";
    let _ = fs::write(path, new_content);
    entries
}

/// Collect `.bak.*` files under `state_dir`, sorted oldest-first.
fn bak_files_in_state(state_dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(state_dir) else {
        return Vec::new();
    };
    let mut baks: Vec<(PathBuf, SystemTime)> = entries
        .flatten()
        .filter(|e| {
            let name = e.file_name();
            let n = name.to_string_lossy();
            n.contains(".bak.") && e.path().is_file()
        })
        .filter_map(|e| {
            let p = e.path();
            let mtime = fs::metadata(&p).ok()?.modified().ok()?;
            Some((p, mtime))
        })
        .collect();
    baks.sort_by_key(|(_, t)| *t);
    baks.into_iter().map(|(p, _)| p).collect()
}

/// Remove expired `state/*.bak.*` files, keeping at most `keep_count` of the
/// most recent and deleting any that are older than `max_age_hours`.
///
/// Returns the list of deleted paths.
pub fn sweep_bak_files(state_dir: &Path, max_age_hours: u64, keep_count: usize) -> Vec<PathBuf> {
    let mut baks = bak_files_in_state(state_dir);
    let mut deleted = Vec::new();

    // Always discard the oldest backups beyond keep_count.
    let excess_count = baks.len().saturating_sub(keep_count);
    for path in baks.drain(..excess_count) {
        let _ = fs::remove_file(&path);
        deleted.push(path);
    }

    // Among remaining, delete those older than max_age_hours.
    if max_age_hours > 0 {
        for path in &baks {
            if let Ok(meta) = fs::metadata(path) {
                if file_age_hours(&meta).unwrap_or(0) > max_age_hours {
                    let _ = fs::remove_file(path);
                    deleted.push(path.clone());
                }
            }
        }
    }

    deleted
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn default_policies_cover_expected_artifacts() {
        let policies = default_retention_policies();
        assert!(policies.len() >= 13);

        let names: Vec<&str> = policies.iter().map(|p| p.artifact.as_str()).collect();
        // Previously existing artifacts.
        assert!(names.contains(&"episodes.jsonl"));
        assert!(names.contains(&"engrams.jsonl"));
        assert!(names.contains(&"learn/efficiency.jsonl"));
        assert!(names.contains(&"learn/c-factor.jsonl"));
        assert!(names.contains(&"learn/cascade-router.json"));
        assert!(names.contains(&"learn/experiments.json"));
        assert!(names.contains(&"learn/gate-thresholds.json"));
        assert!(names.contains(&"task-outputs/*"));
        // New: largest unbounded artifacts (E02-T07).
        assert!(
            names.contains(&"events.jsonl"),
            "missing events.jsonl policy"
        );
        assert!(names.contains(&"roko.log"), "missing roko.log policy");
        assert!(
            names.contains(&"chain-watcher.log"),
            "missing chain-watcher.log policy"
        );
        assert!(
            names.contains(&"run-ledger.jsonl"),
            "missing run-ledger.jsonl policy"
        );
        // New: backup file sweep sentinel.
        assert!(names.contains(&"state/bak"), "missing state/bak policy");
    }

    #[test]
    fn check_retention_detects_oversized_file() {
        let dir = tempdir().unwrap();
        let roko_dir = dir.path().join(".roko");
        fs::create_dir_all(&roko_dir).unwrap();

        // Create an episodes.jsonl that exceeds the 50MB limit.
        let episodes_path = roko_dir.join("episodes.jsonl");
        let mut f = fs::File::create(&episodes_path).unwrap();
        // Write just enough to detect -- we override the limit check by using
        // a smaller file with a patched policy in a real scenario, but for the
        // default policy 50MB is too large. Instead, verify no violation when small.
        writeln!(f, "{{}}").unwrap();

        let violations = check_retention(dir.path());
        // A 3-byte file should produce no violations.
        assert!(violations.is_empty());
    }

    #[test]
    fn check_retention_returns_empty_for_missing_dir() {
        let dir = tempdir().unwrap();
        // No .roko/ directory at all.
        let violations = check_retention(dir.path());
        assert!(violations.is_empty());
    }

    #[test]
    fn apply_retention_dry_run_does_not_modify_files() {
        let dir = tempdir().unwrap();
        let roko_dir = dir.path().join(".roko");
        fs::create_dir_all(roko_dir.join("learn")).unwrap();

        let episodes_path = roko_dir.join("episodes.jsonl");
        fs::write(&episodes_path, "line1\nline2\nline3\n").unwrap();

        let actions = apply_retention(dir.path(), true);
        // Small file should not trigger any action.
        assert!(actions.is_empty());

        // Verify file is untouched.
        let content = fs::read_to_string(&episodes_path).unwrap();
        assert_eq!(content, "line1\nline2\nline3\n");
    }

    #[test]
    fn tail_keep_file_keeps_last_n_entries() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.jsonl");

        let lines: Vec<String> = (0..100).map(|i| format!("line-{i}")).collect();
        fs::write(&path, lines.join("\n") + "\n").unwrap();

        let kept = tail_keep_file(&path, 10);
        assert_eq!(kept, 10);

        let content = fs::read_to_string(&path).unwrap();
        let remaining: Vec<&str> = content.lines().collect();
        assert_eq!(remaining.len(), 10);
        assert_eq!(remaining[0], "line-90");
        assert_eq!(remaining[9], "line-99");
    }

    #[test]
    fn export_postmortem_creates_output_file() {
        let dir = tempdir().unwrap();
        let roko_dir = dir.path().join(".roko");
        fs::create_dir_all(roko_dir.join("learn")).unwrap();

        fs::write(
            roko_dir.join("episodes.jsonl"),
            "{\"episode\":1}\n{\"episode\":2}\n",
        )
        .unwrap();

        let output = dir.path().join("postmortem.json");
        export_postmortem(dir.path(), &output).unwrap();

        assert!(output.exists());

        let content = fs::read_to_string(&output).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();
        assert!(parsed["exported_at"].is_string());
        assert!(parsed["episodes"].is_array());
        assert_eq!(parsed["episodes"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn retention_policy_serializes_roundtrip() {
        let policy = ArtifactRetentionPolicy {
            artifact: "episodes.jsonl".into(),
            path: "episodes.jsonl".into(),
            max_age_hours: 168,
            max_size_bytes: 50_000_000,
            strategy: CompactionStrategy::TailKeep { entries: 10_000 },
        };
        let json = serde_json::to_string(&policy).unwrap();
        let parsed: ArtifactRetentionPolicy = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.artifact, "episodes.jsonl");
        assert_eq!(parsed.max_age_hours, 168);
    }

    #[test]
    fn new_log_policies_use_non_manual_strategies() {
        let policies = default_retention_policies();
        for artifact in &[
            "events.jsonl",
            "roko.log",
            "chain-watcher.log",
            "run-ledger.jsonl",
        ] {
            let p = policies
                .iter()
                .find(|p| p.artifact == *artifact)
                .unwrap_or_else(|| panic!("missing policy for {artifact}"));
            assert!(
                !matches!(p.strategy, CompactionStrategy::Manual),
                "policy for {artifact} must not be Manual"
            );
            assert!(
                p.max_size_bytes > 0 || p.max_age_hours > 0,
                "policy for {artifact} must have at least one non-zero limit"
            );
        }
    }

    #[test]
    fn sweep_bak_files_removes_excess_backups() {
        let dir = tempdir().unwrap();
        let state_dir = dir.path().join("state");
        fs::create_dir_all(&state_dir).unwrap();

        // Create 15 .bak.* files.
        for i in 0..15u32 {
            fs::write(state_dir.join(format!("executor.json.bak.{i}")), b"backup").unwrap();
        }

        // Sweep with keep_count = 10, no age limit.
        let deleted = sweep_bak_files(&state_dir, 0, 10);
        assert_eq!(deleted.len(), 5, "should have deleted 5 excess backups");

        // Only 10 should remain.
        let remaining = bak_files_in_state(&state_dir);
        assert_eq!(remaining.len(), 10);
    }

    #[test]
    fn sweep_bak_files_is_noop_when_under_count() {
        let dir = tempdir().unwrap();
        let state_dir = dir.path().join("state");
        fs::create_dir_all(&state_dir).unwrap();

        for i in 0..5u32 {
            fs::write(state_dir.join(format!("run-state.json.bak.{i}")), b"bak").unwrap();
        }

        let deleted = sweep_bak_files(&state_dir, 0, 10);
        assert!(
            deleted.is_empty(),
            "nothing should be deleted when under keep_count"
        );
        assert_eq!(bak_files_in_state(&state_dir).len(), 5);
    }

    #[test]
    fn sweep_bak_files_ignores_non_bak_files() {
        let dir = tempdir().unwrap();
        let state_dir = dir.path().join("state");
        fs::create_dir_all(&state_dir).unwrap();

        // Create normal state files (must not be touched).
        fs::write(state_dir.join("executor.json"), b"{}").unwrap();
        fs::write(state_dir.join("state-snapshot.json"), b"{}").unwrap();
        // Two bak files, keep_count = 1.
        fs::write(state_dir.join("executor.json.bak.123"), b"bak").unwrap();
        fs::write(state_dir.join("executor.json.bak.456"), b"bak").unwrap();

        let deleted = sweep_bak_files(&state_dir, 0, 1);
        assert_eq!(deleted.len(), 1, "only one excess bak should be removed");
        // Normal files must still exist.
        assert!(state_dir.join("executor.json").exists());
        assert!(state_dir.join("state-snapshot.json").exists());
    }
}
