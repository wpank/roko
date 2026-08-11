//! Explicit cleanup for retained runner attempt worktrees and branches.

use anyhow::{Context, Result, bail};
use serde::Serialize;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::Command;

const ATTEMPT_BRANCH_PREFIX: &str = "roko/attempt/";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum AttemptPruneDisposition {
    Eligible,
    PreservedDirty,
    PreservedUnmerged,
    PreservedResumeState,
    PreservedUnmanagedPath,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct AttemptPruneEntry {
    pub id: String,
    pub branch: String,
    /// Immutable tip inspected when this entry was classified. Apply uses it
    /// as a compare-and-delete guard so a concurrently advanced sibling tip
    /// is never deleted.
    pub expected_oid: String,
    pub worktree: Option<PathBuf>,
    pub disposition: AttemptPruneDisposition,
    pub detail: String,
}

#[derive(Debug, Default, Serialize)]
pub(crate) struct AttemptPruneReport {
    pub dry_run: bool,
    pub entries: Vec<AttemptPruneEntry>,
    pub removed_worktrees: Vec<PathBuf>,
    pub deleted_branches: Vec<String>,
    pub errors: Vec<String>,
}

impl AttemptPruneReport {
    pub(crate) fn eligible_count(&self) -> usize {
        self.entries
            .iter()
            .filter(|entry| entry.disposition == AttemptPruneDisposition::Eligible)
            .count()
    }

    pub(crate) fn preserved_count(&self) -> usize {
        self.entries.len().saturating_sub(self.eligible_count())
    }
}

/// Inspect and optionally remove runner-owned attempt worktrees and branches.
///
/// The caller must hold the workspace lock for the full call. Dirty worktrees
/// are never removed. Unmerged tips require the explicit `include_unmerged`
/// opt-in, while attempts referenced by resumable state are always preserved.
pub(crate) fn prune_attempts(
    workdir: &Path,
    apply: bool,
    include_unmerged: bool,
) -> Result<AttemptPruneReport> {
    let protected = protected_resume_attempts(workdir)?;
    let linked = linked_attempt_worktrees(workdir)?;
    let branches = attempt_branches(workdir)?;
    let managed_root_raw = workdir.join(".roko").join("worktrees");
    let managed_root = std::fs::canonicalize(&managed_root_raw).unwrap_or(managed_root_raw);
    let mut report = AttemptPruneReport {
        dry_run: !apply,
        ..AttemptPruneReport::default()
    };

    for (id, (branch, expected_oid)) in branches {
        let worktree = linked.get(&branch).cloned();
        let expected_path = managed_root.join(&id);
        let (disposition, detail) = if protected.contains(&id) {
            (
                AttemptPruneDisposition::PreservedResumeState,
                "referenced by the current resumable plan snapshot".to_string(),
            )
        } else if let Some(path) = worktree.as_ref().filter(|path| *path != &expected_path) {
            (
                AttemptPruneDisposition::PreservedUnmanagedPath,
                format!(
                    "linked checkout is outside its exact managed path {}",
                    path.display()
                ),
            )
        } else if let Some(path) = worktree.as_ref() {
            let status = git_output(
                workdir,
                [
                    "-C",
                    &path.to_string_lossy(),
                    "status",
                    "--porcelain",
                    "--untracked-files=all",
                ],
            )?;
            if !status.trim().is_empty() {
                (
                    AttemptPruneDisposition::PreservedDirty,
                    format!(
                        "uncommitted evidence: {}",
                        status.lines().collect::<Vec<_>>().join("; ")
                    ),
                )
            } else {
                classify_tip(workdir, &expected_oid, include_unmerged)?
            }
        } else {
            classify_tip(workdir, &expected_oid, include_unmerged)?
        };

        report.entries.push(AttemptPruneEntry {
            id,
            branch,
            expected_oid,
            worktree,
            disposition,
            detail,
        });
    }

    if !apply {
        return Ok(report);
    }

    for entry in report
        .entries
        .iter()
        .filter(|entry| entry.disposition == AttemptPruneDisposition::Eligible)
    {
        if let Some(path) = entry.worktree.as_ref() {
            match git_status(
                workdir,
                ["worktree", "remove", "--", &path.to_string_lossy()],
            ) {
                Ok(()) => report.removed_worktrees.push(path.clone()),
                Err(error) => {
                    report.errors.push(format!(
                        "{}: could not remove worktree: {error:#}",
                        entry.id
                    ));
                    continue;
                }
            }
        }
        match delete_branch_if_unchanged(workdir, &entry.branch, &entry.expected_oid) {
            Ok(()) => report.deleted_branches.push(entry.branch.clone()),
            Err(error) => report
                .errors
                .push(format!("{}: could not delete branch: {error:#}", entry.id)),
        }
    }

    Ok(report)
}

fn classify_tip(
    workdir: &Path,
    expected_oid: &str,
    include_unmerged: bool,
) -> Result<(AttemptPruneDisposition, String)> {
    let merged = Command::new("git")
        .current_dir(workdir)
        .args(["merge-base", "--is-ancestor", expected_oid, "HEAD"])
        .status()
        .with_context(|| format!("check whether {expected_oid} is merged"))?;
    if merged.success() || include_unmerged {
        let detail = if merged.success() {
            "tip is reachable from HEAD".to_string()
        } else {
            "unmerged tip selected by --include-unmerged".to_string()
        };
        Ok((AttemptPruneDisposition::Eligible, detail))
    } else if merged.code() == Some(1) {
        Ok((
            AttemptPruneDisposition::PreservedUnmerged,
            "tip is not reachable from HEAD; pass --include-unmerged to select it".to_string(),
        ))
    } else {
        bail!("git merge-base failed while inspecting {expected_oid}")
    }
}

fn attempt_branches(workdir: &Path) -> Result<BTreeMap<String, (String, String)>> {
    let output = git_output(
        workdir,
        [
            "for-each-ref",
            "--format=%(refname:short)%00%(objectname)",
            "refs/heads/roko/attempt/",
        ],
    )?;
    let mut branches = BTreeMap::new();
    for line in output
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
    {
        let Some((branch, oid)) = line.split_once('\0') else {
            bail!("git returned an invalid attempt ref record")
        };
        let Some(id) = branch.strip_prefix(ATTEMPT_BRANCH_PREFIX) else {
            continue;
        };
        if id.starts_with("attempt-") {
            branches.insert(id.to_string(), (branch.to_string(), oid.to_string()));
        }
    }
    Ok(branches)
}

fn delete_branch_if_unchanged(workdir: &Path, branch: &str, expected_oid: &str) -> Result<()> {
    let full_ref = format!("refs/heads/{branch}");
    let output = Command::new("git")
        .current_dir(workdir)
        .args(["update-ref", "-d", &full_ref, expected_oid])
        .output()
        .context("run guarded git ref deletion")?;
    if output.status.success() {
        Ok(())
    } else {
        bail!(
            "branch tip changed after inspection; preserving it: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )
    }
}

fn linked_attempt_worktrees(workdir: &Path) -> Result<BTreeMap<String, PathBuf>> {
    let output = git_output(workdir, ["worktree", "list", "--porcelain"])?;
    let mut linked = BTreeMap::new();
    let mut path = None;
    for line in output.lines().chain(std::iter::once("")) {
        if let Some(value) = line.strip_prefix("worktree ") {
            path = Some(PathBuf::from(value));
        } else if let Some(value) = line.strip_prefix("branch refs/heads/") {
            if value.starts_with(ATTEMPT_BRANCH_PREFIX) {
                if let Some(path) = path.take() {
                    linked.insert(value.to_string(), path);
                }
            }
        } else if line.is_empty() {
            path = None;
        }
    }
    Ok(linked)
}

fn protected_resume_attempts(workdir: &Path) -> Result<BTreeSet<String>> {
    let snapshot_path = workdir.join(".roko/state/state-snapshot.json");
    if !snapshot_path.exists() {
        return Ok(BTreeSet::new());
    }
    let outer: Value = serde_json::from_slice(
        &std::fs::read(&snapshot_path)
            .with_context(|| format!("read {}", snapshot_path.display()))?,
    )
    .with_context(|| format!("parse {}", snapshot_path.display()))?;
    let executor = nested_json(&outer, "executor_json", &snapshot_path)?;
    let run_state = nested_json(&outer, "run_state_json", &snapshot_path)?;
    let attempt_namespace = run_state.get("attempt_namespace").and_then(Value::as_str);

    let mut resumable_plans = BTreeSet::new();
    let plan_states = executor
        .get("plan_states")
        .and_then(Value::as_object)
        .context("snapshot executor_json has no plan_states object")?;
    for (plan_id, state) in plan_states {
        let kind = state
            .pointer("/current_phase/kind")
            .and_then(Value::as_str)
            .context("snapshot plan state has no current_phase.kind")?;
        if !matches!(kind, "complete" | "skipped") {
            resumable_plans.insert(plan_id.as_str());
        }
    }

    let mut protected = BTreeSet::new();
    if let Some(attempts) = run_state
        .pointer("/lifecycle/task_attempts")
        .and_then(Value::as_object)
    {
        for attempt in attempts.values() {
            let Some(plan_id) = attempt.get("plan_id").and_then(Value::as_str) else {
                continue;
            };
            if !resumable_plans.contains(plan_id) {
                continue;
            }
            let Some(task_id) = attempt.get("task_id").and_then(Value::as_str) else {
                continue;
            };
            let Some(number) = attempt.get("attempt").and_then(Value::as_u64) else {
                continue;
            };
            let Ok(number) = u32::try_from(number) else {
                continue;
            };
            let id = attempt_namespace.map_or_else(
                || {
                    roko_cli::orchestrator::worktree::format_attempt_worktree_id(
                        plan_id, task_id, number,
                    )
                },
                |namespace| {
                    roko_cli::orchestrator::worktree::format_scoped_attempt_worktree_id(
                        namespace, plan_id, task_id, number,
                    )
                },
            );
            protected.insert(id);
        }
    }
    Ok(protected)
}

fn nested_json(outer: &Value, key: &str, path: &Path) -> Result<Value> {
    let encoded = outer
        .get(key)
        .and_then(Value::as_str)
        .with_context(|| format!("{} has no {key} string", path.display()))?;
    serde_json::from_str(encoded)
        .with_context(|| format!("parse {key} embedded in {}", path.display()))
}

fn git_output<const N: usize>(workdir: &Path, args: [&str; N]) -> Result<String> {
    let output = Command::new("git")
        .current_dir(workdir)
        .args(args)
        .output()
        .context("run git")?;
    if !output.status.success() {
        bail!(
            "git failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    String::from_utf8(output.stdout).context("git returned non-UTF-8 output")
}

fn git_status<const N: usize>(workdir: &Path, args: [&str; N]) -> Result<()> {
    let output = Command::new("git")
        .current_dir(workdir)
        .args(args)
        .output()
        .context("run git")?;
    if output.status.success() {
        Ok(())
    } else {
        bail!("{}", String::from_utf8_lossy(&output.stderr).trim())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn git(dir: &Path, args: &[&str]) {
        let output = Command::new("git")
            .current_dir(dir)
            .args(args)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "git {args:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    #[test]
    fn prune_defaults_to_merged_clean_attempts_and_preserves_evidence() {
        let dir = tempfile::tempdir().unwrap();
        git(dir.path(), &["init", "-q"]);
        git(
            dir.path(),
            &["config", "user.email", "test@example.invalid"],
        );
        git(dir.path(), &["config", "user.name", "Roko Test"]);
        std::fs::write(dir.path().join("base"), "base\n").unwrap();
        git(dir.path(), &["add", "base"]);
        git(dir.path(), &["commit", "-qm", "base"]);

        let root = dir.path().join(".roko/worktrees");
        std::fs::create_dir_all(&root).unwrap();
        for id in ["attempt-merged", "attempt-unmerged", "attempt-dirty"] {
            git(
                dir.path(),
                &[
                    "worktree",
                    "add",
                    "-qb",
                    &format!("roko/attempt/{id}"),
                    root.join(id).to_str().unwrap(),
                ],
            );
        }
        let unmerged = root.join("attempt-unmerged");
        std::fs::write(unmerged.join("evidence"), "committed evidence\n").unwrap();
        git(&unmerged, &["add", "evidence"]);
        git(&unmerged, &["commit", "-qm", "unmerged evidence"]);
        std::fs::write(root.join("attempt-dirty").join("notes"), "dirty evidence\n").unwrap();

        let preview = prune_attempts(dir.path(), false, false).unwrap();
        assert!(preview.dry_run);
        assert_eq!(preview.eligible_count(), 1);
        assert_eq!(preview.preserved_count(), 2);
        assert!(preview.entries.iter().any(|entry| {
            entry.id == "attempt-unmerged"
                && entry.disposition == AttemptPruneDisposition::PreservedUnmerged
        }));
        assert!(preview.entries.iter().any(|entry| {
            entry.id == "attempt-dirty"
                && entry.disposition == AttemptPruneDisposition::PreservedDirty
        }));
        assert!(root.join("attempt-merged").is_dir());

        let applied = prune_attempts(dir.path(), true, false).unwrap();
        assert!(applied.errors.is_empty(), "{applied:?}");
        assert_eq!(applied.removed_worktrees.len(), 1);
        assert_eq!(applied.deleted_branches, ["roko/attempt/attempt-merged"]);
        assert!(!root.join("attempt-merged").exists());
        assert!(root.join("attempt-unmerged").is_dir());
        assert!(root.join("attempt-dirty").is_dir());

        let destructive_preview = prune_attempts(dir.path(), false, true).unwrap();
        assert!(destructive_preview.entries.iter().any(|entry| {
            entry.id == "attempt-unmerged" && entry.disposition == AttemptPruneDisposition::Eligible
        }));
        assert!(destructive_preview.entries.iter().any(|entry| {
            entry.id == "attempt-dirty"
                && entry.disposition == AttemptPruneDisposition::PreservedDirty
        }));
    }

    #[test]
    fn guarded_deletion_preserves_a_tip_advanced_after_classification() {
        let dir = tempfile::tempdir().unwrap();
        git(dir.path(), &["init", "-q"]);
        git(
            dir.path(),
            &["config", "user.email", "test@example.invalid"],
        );
        git(dir.path(), &["config", "user.name", "Roko Test"]);
        std::fs::write(dir.path().join("base"), "base\n").unwrap();
        git(dir.path(), &["add", "base"]);
        git(dir.path(), &["commit", "-qm", "base"]);
        let branch = "roko/attempt/attempt-race";
        git(dir.path(), &["branch", branch]);
        let inspected = git_output(dir.path(), ["rev-parse", branch])
            .unwrap()
            .trim()
            .to_string();

        std::fs::write(dir.path().join("sibling"), "integrated sibling\n").unwrap();
        git(dir.path(), &["add", "sibling"]);
        git(dir.path(), &["commit", "-qm", "integrate sibling"]);
        git(dir.path(), &["branch", "-f", branch, "HEAD"]);
        let advanced = git_output(dir.path(), ["rev-parse", branch])
            .unwrap()
            .trim()
            .to_string();
        assert_ne!(inspected, advanced);

        let error = delete_branch_if_unchanged(dir.path(), branch, &inspected).unwrap_err();
        assert!(
            error.to_string().contains("branch tip changed"),
            "{error:#}"
        );
        assert_eq!(
            git_output(dir.path(), ["rev-parse", branch])
                .unwrap()
                .trim(),
            advanced
        );
    }

    #[test]
    fn completed_snapshot_does_not_protect_old_attempts() {
        let dir = tempfile::tempdir().unwrap();
        let state = dir.path().join(".roko/state");
        std::fs::create_dir_all(&state).unwrap();
        let executor =
            serde_json::json!({"plan_states":{"p":{"current_phase":{"kind":"complete"}}}});
        let run = serde_json::json!({"lifecycle":{"task_attempts":{"p:t:1":{"plan_id":"p","task_id":"t","attempt":1}}}});
        std::fs::write(
            state.join("state-snapshot.json"),
            serde_json::to_vec(&serde_json::json!({
                "executor_json": executor.to_string(), "run_state_json": run.to_string()
            }))
            .unwrap(),
        )
        .unwrap();
        assert!(protected_resume_attempts(dir.path()).unwrap().is_empty());
    }

    #[test]
    fn nonterminal_snapshot_protects_every_recorded_attempt() {
        let dir = tempfile::tempdir().unwrap();
        let state = dir.path().join(".roko/state");
        std::fs::create_dir_all(&state).unwrap();
        let executor =
            serde_json::json!({"plan_states":{"p":{"current_phase":{"kind":"implementing"}}}});
        let run = serde_json::json!({"lifecycle":{"task_attempts":{
            "p:t:1":{"plan_id":"p","task_id":"t","attempt":1},
            "p:t:2":{"plan_id":"p","task_id":"t","attempt":2}
        }}});
        std::fs::write(
            state.join("state-snapshot.json"),
            serde_json::to_vec(&serde_json::json!({
                "executor_json": executor.to_string(), "run_state_json": run.to_string()
            }))
            .unwrap(),
        )
        .unwrap();
        let protected = protected_resume_attempts(dir.path()).unwrap();
        assert_eq!(protected.len(), 2);
        assert!(
            protected.contains(
                &roko_cli::orchestrator::worktree::format_attempt_worktree_id("p", "t", 1)
            )
        );
        assert!(
            protected.contains(
                &roko_cli::orchestrator::worktree::format_attempt_worktree_id("p", "t", 2)
            )
        );
    }

    #[test]
    fn nonterminal_snapshot_protects_run_scoped_attempt() {
        let dir = tempfile::tempdir().unwrap();
        let state = dir.path().join(".roko/state");
        std::fs::create_dir_all(&state).unwrap();
        let executor =
            serde_json::json!({"plan_states":{"p":{"current_phase":{"kind":"implementing"}}}});
        let run = serde_json::json!({
            "attempt_namespace": "run-current",
            "lifecycle":{"task_attempts":{
                "p:t:1":{"plan_id":"p","task_id":"t","attempt":1}
            }}
        });
        std::fs::write(
            state.join("state-snapshot.json"),
            serde_json::to_vec(&serde_json::json!({
                "executor_json": executor.to_string(), "run_state_json": run.to_string()
            }))
            .unwrap(),
        )
        .unwrap();

        let protected = protected_resume_attempts(dir.path()).unwrap();
        assert_eq!(
            protected,
            [
                roko_cli::orchestrator::worktree::format_scoped_attempt_worktree_id(
                    "run-current",
                    "p",
                    "t",
                    1
                )
            ]
            .into_iter()
            .collect()
        );
    }

    #[test]
    fn malformed_existing_snapshot_fails_closed() {
        let dir = tempfile::tempdir().unwrap();
        let state = dir.path().join(".roko/state");
        std::fs::create_dir_all(&state).unwrap();
        std::fs::write(state.join("state-snapshot.json"), b"not json").unwrap();
        assert!(protected_resume_attempts(dir.path()).is_err());
    }
}
