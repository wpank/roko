//! `roko backlog` — batch import, listing, and reconciliation audit.
//!
//! Reads markdown specs from `tmp/backlog/<N>-*.md` files and creates PRD
//! ideas in `.roko/prd/ideas/`. Optionally chains through draft, plan, and
//! execution steps.
//!
//! The `audit` subcommand reconciles plan TOML status against durable runner
//! state, reporting drift between declared task status and actual runner
//! completion/failure records.

use anyhow::{Context, Result};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use roko_fs::RokoLayout;

use crate::{BacklogCmd, Cli, resolve_workdir};

/// Dispatch backlog subcommands.
pub(crate) async fn cmd_backlog(cli: &Cli, cmd: BacklogCmd) -> Result<i32> {
    match cmd {
        BacklogCmd::Import {
            path,
            draft,
            plan,
            execute,
            workdir,
            ..
        } => {
            let wd = workdir.unwrap_or_else(|| resolve_workdir(cli));
            cmd_backlog_import(&wd, &path, draft, plan, execute).await
        }
        BacklogCmd::List { workdir } => {
            let wd = workdir.unwrap_or_else(|| resolve_workdir(cli));
            cmd_backlog_list(&wd)
        }
        BacklogCmd::Audit { workdir, json } => {
            let wd = workdir.unwrap_or_else(|| resolve_workdir(cli));
            cmd_backlog_audit(&wd, json)
        }
    }
}

/// List backlog items and their import status.
fn cmd_backlog_list(workdir: &Path) -> Result<i32> {
    let backlog_dir = workdir.join("tmp/backlog");
    if !backlog_dir.is_dir() {
        println!("No backlog directory found at {}", backlog_dir.display());
        return Ok(0);
    }

    let mut entries: Vec<(u32, String, PathBuf)> = Vec::new();
    for entry in std::fs::read_dir(&backlog_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().is_some_and(|e| e == "md")
            && let Some(fname) = path.file_stem().and_then(|s| s.to_str())
        {
            // Skip the index file
            if fname == "00-INDEX" {
                continue;
            }
            // Parse leading number
            if let Some(num) = fname.split('-').next().and_then(|n| n.parse::<u32>().ok()) {
                entries.push((num, fname.to_string(), path.clone()));
            }
        }
    }

    entries.sort_by_key(|(num, _, _)| *num);

    // Check which have been imported
    let ideas_dir = workdir.join(".roko/prd/ideas");
    let ideas_exist = ideas_dir.is_dir();

    println!("Backlog specs ({} items):", entries.len());
    println!("{:<6} {:<50} {}", "ID", "Slug", "Status");
    println!("{}", "-".repeat(70));

    for (num, slug, _path) in &entries {
        let imported = if ideas_exist {
            // Check if an idea file references this backlog number
            has_imported_idea(&ideas_dir, *num)
        } else {
            false
        };
        let status = if imported { "imported" } else { "-" };
        println!("#{:<5} {:<50} {}", num, slug, status);
    }

    Ok(0)
}

/// Check if an idea referencing a backlog number already exists.
fn has_imported_idea(ideas_dir: &Path, backlog_num: u32) -> bool {
    if let Ok(content) = std::fs::read_to_string(ideas_dir.join("ideas.md")) {
        let marker = format!("[backlog#{}]", backlog_num);
        return content.contains(&marker);
    }
    false
}

/// Import backlog spec(s) as PRD ideas.
async fn cmd_backlog_import(
    workdir: &Path,
    path: &Path,
    draft: bool,
    plan: bool,
    execute: bool,
) -> Result<i32> {
    let files = collect_backlog_files(workdir, path)?;

    if files.is_empty() {
        println!("No backlog spec files found at {}", path.display());
        return Ok(1);
    }

    println!("Importing {} backlog spec(s)...\n", files.len());

    let mut imported = 0;
    let mut skipped = 0;

    for (num, slug, filepath) in &files {
        // Read the spec title from the first heading
        let content = std::fs::read_to_string(filepath)
            .with_context(|| format!("read {}", filepath.display()))?;
        let title = extract_title(&content).unwrap_or_else(|| slug.clone());

        // Create the PRD idea
        let idea_text = format!("[backlog#{}] {}", num, title);
        match roko_cli::prd::cmd_idea(workdir, &idea_text) {
            Ok(()) => {
                imported += 1;
                println!("  #{}: {}", num, title);
            }
            Err(e) => {
                eprintln!("  #{}: failed: {}", num, e);
                skipped += 1;
                continue;
            }
        }

        if draft || plan || execute {
            println!(
                "    note: --draft/--plan/--execute require agent dispatch; \
                 use `roko prd draft new` or `roko develop` for each imported idea"
            );
        }
    }

    println!(
        "\nImported: {}, Skipped: {}, Total: {}",
        imported,
        skipped,
        files.len()
    );

    if imported > 0 {
        crate::commands::util::print_next_step_hint(
            "Next: roko prd list (or roko develop 'your idea' to plan+execute)",
        );
    }

    Ok(0)
}

/// Collect backlog files from a path (single file or directory).
fn collect_backlog_files(workdir: &Path, path: &Path) -> Result<Vec<(u32, String, PathBuf)>> {
    let resolved = if path.is_relative() {
        workdir.join(path)
    } else {
        path.to_path_buf()
    };

    let mut files = Vec::new();

    if resolved.is_file() {
        if let Some(parsed) = parse_backlog_filename(&resolved) {
            files.push(parsed);
        }
    } else if resolved.is_dir() {
        for entry in std::fs::read_dir(&resolved)? {
            let entry = entry?;
            let p = entry.path();
            if p.extension().is_some_and(|e| e == "md")
                && let Some(parsed) = parse_backlog_filename(&p)
            {
                files.push(parsed);
            }
        }
        files.sort_by_key(|(num, _, _)| *num);
    }

    Ok(files)
}

/// Parse a backlog filename like `65-cli-verb-consolidation.md` -> (65, "cli-verb-consolidation", path).
fn parse_backlog_filename(path: &Path) -> Option<(u32, String, PathBuf)> {
    let stem = path.file_stem()?.to_str()?;
    if stem == "00-INDEX" {
        return None;
    }
    let dash_pos = stem.find('-')?;
    let num: u32 = stem[..dash_pos].parse().ok()?;
    let slug = stem[dash_pos + 1..].to_string();
    Some((num, slug, path.to_path_buf()))
}

/// Extract the title from a markdown file (first # heading).
fn extract_title(content: &str) -> Option<String> {
    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(heading) = trimmed.strip_prefix("# ") {
            return Some(heading.trim().to_string());
        }
    }
    None
}

// -----------------------------------------------------------------------
// Backlog audit — plan TOML vs runner state reconciliation
// -----------------------------------------------------------------------

/// Per-task reconciliation finding.
#[derive(Debug, Clone, serde::Serialize)]
struct TaskDrift {
    plan_id: String,
    task_id: String,
    toml_status: String,
    runner_status: String,
    drift: String,
}

/// Per-plan reconciliation finding.
#[derive(Debug, Clone, serde::Serialize)]
struct PlanDrift {
    plan_id: String,
    meta_status: String,
    executor_phase: String,
    drift: String,
}

/// Full audit report.
#[derive(Debug, Clone, serde::Serialize)]
struct AuditReport {
    plans_on_disk: usize,
    plans_in_executor: usize,
    plan_drifts: Vec<PlanDrift>,
    task_drifts: Vec<TaskDrift>,
    orphaned_executor_plans: Vec<String>,
    orphaned_disk_plans: Vec<String>,
}

impl AuditReport {
    fn total_issues(&self) -> usize {
        self.plan_drifts.len()
            + self.task_drifts.len()
            + self.orphaned_executor_plans.len()
            + self.orphaned_disk_plans.len()
    }
}

/// Read per-plan task status from all `plans/*/tasks.toml` files.
///
/// Returns a map of `plan_id -> (meta_status, vec of (task_id, task_status))`.
fn read_toml_plan_statuses(workdir: &Path) -> BTreeMap<String, (String, Vec<(String, String)>)> {
    let plans_dir = workdir.join("plans");
    let mut result = BTreeMap::new();

    let Ok(entries) = std::fs::read_dir(&plans_dir) else {
        return result;
    };

    for entry in entries.flatten() {
        let dir = entry.path();
        if !dir.is_dir() {
            continue;
        }
        let tasks_path = dir.join("tasks.toml");
        if !tasks_path.is_file() {
            continue;
        }
        let Ok(tasks_file) = roko_cli::task_parser::TasksFile::parse(&tasks_path) else {
            continue;
        };
        let plan_id = tasks_file.meta.plan.clone();
        let meta_status = tasks_file.meta.status.clone();
        let task_statuses: Vec<(String, String)> = tasks_file
            .tasks
            .iter()
            .map(|t| (t.id.clone(), t.status.clone()))
            .collect();
        result.insert(plan_id, (meta_status, task_statuses));
    }

    result
}

/// Read the executor plan_states from the durable state snapshot.
///
/// Returns a map of `plan_id -> phase_kind`.
fn read_executor_plan_phases(workdir: &Path) -> BTreeMap<String, String> {
    let layout = RokoLayout::for_project(workdir);
    let snapshot_path = layout.state_dir().join("state-snapshot.json");

    let contents = match std::fs::read_to_string(&snapshot_path) {
        Ok(c) => c,
        Err(_) => {
            // Fall back to executor.json for older workspaces.
            let executor_path = layout.executor_snapshot();
            match std::fs::read_to_string(&executor_path) {
                Ok(c) => c,
                Err(_) => return BTreeMap::new(),
            }
        }
    };

    let Ok(value) = serde_json::from_str::<serde_json::Value>(&contents) else {
        return BTreeMap::new();
    };

    // state-snapshot.json wraps executor state inside `executor_json` as a
    // stringified JSON field; executor.json has plan_states at the top level.
    let executor_value = if let Some(inner) = value.get("executor_json").and_then(|v| v.as_str()) {
        serde_json::from_str::<serde_json::Value>(inner).unwrap_or(value.clone())
    } else {
        value
    };

    let Some(plan_states) = executor_value
        .get("plan_states")
        .and_then(serde_json::Value::as_object)
    else {
        return BTreeMap::new();
    };

    plan_states
        .iter()
        .map(|(id, state)| {
            let phase = state
                .get("current_phase")
                .and_then(|p| p.get("kind"))
                .and_then(serde_json::Value::as_str)
                .unwrap_or("unknown")
                .to_string();
            (id.clone(), phase)
        })
        .collect()
}

/// Read run-state task terminal sets from the durable state snapshot.
///
/// Returns `(completed, failed, skipped)` maps of `plan_id -> set of task_ids`.
fn read_run_state_task_terminals(
    workdir: &Path,
) -> (
    BTreeMap<String, BTreeSet<String>>,
    BTreeMap<String, BTreeSet<String>>,
    BTreeMap<String, BTreeSet<String>>,
) {
    let layout = RokoLayout::for_project(workdir);
    let snapshot_path = layout.state_dir().join("state-snapshot.json");
    let run_state_path = layout.run_state_path();

    // Try state-snapshot.json first (has run_state_json embedded), fall back
    // to standalone run-state.json.
    let run_value = if let Ok(contents) = std::fs::read_to_string(&snapshot_path) {
        let snap: serde_json::Value =
            serde_json::from_str(&contents).unwrap_or(serde_json::Value::Null);
        if let Some(inner) = snap.get("run_state_json").and_then(|v| v.as_str()) {
            serde_json::from_str(inner).unwrap_or(serde_json::Value::Null)
        } else {
            serde_json::Value::Null
        }
    } else if let Ok(contents) = std::fs::read_to_string(&run_state_path) {
        serde_json::from_str(&contents).unwrap_or(serde_json::Value::Null)
    } else {
        serde_json::Value::Null
    };

    let extract = |key: &str| -> BTreeMap<String, BTreeSet<String>> {
        let Some(obj) = run_value.get(key).and_then(serde_json::Value::as_object) else {
            return BTreeMap::new();
        };
        obj.iter()
            .map(|(plan_id, tasks)| {
                let ids: BTreeSet<String> = tasks
                    .as_array()
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default();
                (plan_id.clone(), ids)
            })
            .collect()
    };

    (
        extract("completed_tasks"),
        extract("failed_tasks"),
        extract("skipped_tasks"),
    )
}

/// Determine the effective runner status for a given plan+task from the
/// terminal sets.
fn runner_task_status(
    plan_id: &str,
    task_id: &str,
    completed: &BTreeMap<String, BTreeSet<String>>,
    failed: &BTreeMap<String, BTreeSet<String>>,
    skipped: &BTreeMap<String, BTreeSet<String>>,
) -> &'static str {
    if completed.get(plan_id).is_some_and(|s| s.contains(task_id)) {
        return "completed";
    }
    if failed.get(plan_id).is_some_and(|s| s.contains(task_id)) {
        return "failed";
    }
    if skipped.get(plan_id).is_some_and(|s| s.contains(task_id)) {
        return "skipped";
    }
    "none"
}

/// Build the full reconciliation report.
fn build_audit_report(workdir: &Path) -> AuditReport {
    let toml_plans = read_toml_plan_statuses(workdir);
    let executor_phases = read_executor_plan_phases(workdir);
    let (completed, failed, skipped) = read_run_state_task_terminals(workdir);

    let disk_ids: BTreeSet<&str> = toml_plans.keys().map(String::as_str).collect();
    let executor_ids: BTreeSet<&str> = executor_phases.keys().map(String::as_str).collect();

    // Orphaned plans: in executor but not on disk, or on disk but not in executor.
    let orphaned_executor: Vec<String> = executor_ids
        .difference(&disk_ids)
        .map(|s| (*s).to_string())
        .collect();
    let orphaned_disk: Vec<String> = disk_ids
        .difference(&executor_ids)
        .map(|s| (*s).to_string())
        .collect();

    let mut plan_drifts = Vec::new();
    let mut task_drifts = Vec::new();

    for (plan_id, (meta_status, task_statuses)) in &toml_plans {
        let executor_phase = executor_phases
            .get(plan_id)
            .cloned()
            .unwrap_or_else(|| "absent".to_string());

        // Plan-level drift: meta says "complete" but executor phase is not.
        let meta_lower = meta_status.to_ascii_lowercase();
        let phase_lower = executor_phase.to_ascii_lowercase();
        if (meta_lower == "complete" || meta_lower == "completed" || meta_lower == "done")
            && phase_lower != "complete"
            && phase_lower != "completed"
            && phase_lower != "done"
            && phase_lower != "absent"
        {
            plan_drifts.push(PlanDrift {
                plan_id: plan_id.clone(),
                meta_status: meta_status.clone(),
                executor_phase: executor_phase.clone(),
                drift: format!(
                    "TOML meta.status=\"{}\" but executor phase=\"{}\"",
                    meta_status, executor_phase
                ),
            });
        }
        // Executor says complete/done but TOML meta does not.
        if (phase_lower == "complete" || phase_lower == "completed" || phase_lower == "done")
            && meta_lower != "complete"
            && meta_lower != "completed"
            && meta_lower != "done"
            && meta_lower != "fixture"
            && meta_lower != "superseded"
            && meta_lower != "archived"
        {
            plan_drifts.push(PlanDrift {
                plan_id: plan_id.clone(),
                meta_status: meta_status.clone(),
                executor_phase: executor_phase.clone(),
                drift: format!(
                    "executor phase=\"{}\" but TOML meta.status=\"{}\"",
                    executor_phase, meta_status
                ),
            });
        }

        // Task-level drift.
        for (task_id, toml_status) in task_statuses {
            let runner = runner_task_status(plan_id, task_id, &completed, &failed, &skipped);
            let toml_lower = toml_status.to_ascii_lowercase();

            // TOML says done/complete but runner has no completion record.
            if (toml_lower == "done" || toml_lower == "complete" || toml_lower == "completed")
                && runner != "completed"
            {
                task_drifts.push(TaskDrift {
                    plan_id: plan_id.clone(),
                    task_id: task_id.clone(),
                    toml_status: toml_status.clone(),
                    runner_status: runner.to_string(),
                    drift: format!(
                        "TOML status=\"{}\" but runner has no completion record (runner={})",
                        toml_status, runner
                    ),
                });
            }

            // Runner completed but TOML still says ready/pending.
            if runner == "completed"
                && toml_lower != "done"
                && toml_lower != "complete"
                && toml_lower != "completed"
            {
                task_drifts.push(TaskDrift {
                    plan_id: plan_id.clone(),
                    task_id: task_id.clone(),
                    toml_status: toml_status.clone(),
                    runner_status: runner.to_string(),
                    drift: format!("runner completed but TOML status=\"{}\"", toml_status),
                });
            }

            // Runner failed but TOML still says ready (informational, not
            // necessarily wrong since tasks can be retried, but worth
            // surfacing).
            if runner == "failed" && toml_lower == "ready" {
                task_drifts.push(TaskDrift {
                    plan_id: plan_id.clone(),
                    task_id: task_id.clone(),
                    toml_status: toml_status.clone(),
                    runner_status: runner.to_string(),
                    drift: format!(
                        "runner recorded failure but TOML status still \"{}\"",
                        toml_status
                    ),
                });
            }
        }
    }

    AuditReport {
        plans_on_disk: toml_plans.len(),
        plans_in_executor: executor_phases.len(),
        plan_drifts,
        task_drifts,
        orphaned_executor_plans: orphaned_executor,
        orphaned_disk_plans: orphaned_disk,
    }
}

/// Print the audit report in human-readable text format.
fn print_audit_report(report: &AuditReport) {
    println!("Backlog/Plan State Reconciliation Audit");
    println!("{}", "=".repeat(50));
    println!();
    println!(
        "Plans on disk: {}    Plans in executor state: {}",
        report.plans_on_disk, report.plans_in_executor
    );
    println!();

    if !report.orphaned_executor_plans.is_empty() {
        println!(
            "Orphaned executor plans ({} — in runner state but no tasks.toml on disk):",
            report.orphaned_executor_plans.len()
        );
        for id in &report.orphaned_executor_plans {
            println!("  - {}", id);
        }
        println!();
    }

    if !report.orphaned_disk_plans.is_empty() {
        println!(
            "Orphaned disk plans ({} — tasks.toml on disk but absent from runner state):",
            report.orphaned_disk_plans.len()
        );
        for id in &report.orphaned_disk_plans {
            println!("  - {}", id);
        }
        println!();
    }

    if !report.plan_drifts.is_empty() {
        println!(
            "Plan-level drift ({} issue{}):",
            report.plan_drifts.len(),
            if report.plan_drifts.len() == 1 {
                ""
            } else {
                "s"
            }
        );
        for d in &report.plan_drifts {
            println!("  [{}] {}", d.plan_id, d.drift);
        }
        println!();
    }

    if !report.task_drifts.is_empty() {
        println!(
            "Task-level drift ({} issue{}):",
            report.task_drifts.len(),
            if report.task_drifts.len() == 1 {
                ""
            } else {
                "s"
            }
        );
        for d in &report.task_drifts {
            println!("  [{}:{}] {}", d.plan_id, d.task_id, d.drift);
        }
        println!();
    }

    let total = report.total_issues();
    if total == 0 {
        println!("No drift detected. Plan TOML and runner state are consistent.");
    } else {
        println!(
            "Total issues: {} (plan-level: {}, task-level: {}, orphaned: {})",
            total,
            report.plan_drifts.len(),
            report.task_drifts.len(),
            report.orphaned_executor_plans.len() + report.orphaned_disk_plans.len()
        );
    }
}

/// `roko backlog audit` entry point.
fn cmd_backlog_audit(workdir: &Path, json: bool) -> Result<i32> {
    let plans_dir = workdir.join("plans");
    if !plans_dir.is_dir() {
        println!("No plans/ directory found at {}", plans_dir.display());
        return Ok(0);
    }

    let report = build_audit_report(workdir);

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&report).unwrap_or_else(|_| "{}".to_string())
        );
    } else {
        print_audit_report(&report);
    }

    Ok(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_backlog_filename() {
        let path = PathBuf::from("tmp/backlog/65-cli-verb-consolidation.md");
        let (num, slug, _) = parse_backlog_filename(&path).unwrap();
        assert_eq!(num, 65);
        assert_eq!(slug, "cli-verb-consolidation");
    }

    #[test]
    fn test_parse_index_file_returns_none() {
        let path = PathBuf::from("tmp/backlog/00-INDEX.md");
        assert!(parse_backlog_filename(&path).is_none());
    }

    #[test]
    fn test_extract_title() {
        let content = "# CLI Verb Consolidation\n\nReduce verb sprawl...";
        assert_eq!(
            extract_title(content),
            Some("CLI Verb Consolidation".to_string())
        );
    }

    #[test]
    fn test_extract_title_no_heading() {
        let content = "No heading here\nJust text";
        assert_eq!(extract_title(content), None);
    }

    // ── Audit tests ──────────────────────────────────────────────────

    #[test]
    fn audit_empty_workspace_reports_zero_issues() {
        let tmp = tempfile::tempdir().unwrap();
        let plans_dir = tmp.path().join("plans");
        std::fs::create_dir_all(&plans_dir).unwrap();

        let report = build_audit_report(tmp.path());
        assert_eq!(report.plans_on_disk, 0);
        assert_eq!(report.plans_in_executor, 0);
        assert_eq!(report.total_issues(), 0);
    }

    #[test]
    fn audit_detects_orphaned_disk_plan() {
        let tmp = tempfile::tempdir().unwrap();
        let plan_dir = tmp.path().join("plans/my-plan");
        std::fs::create_dir_all(&plan_dir).unwrap();
        std::fs::write(
            plan_dir.join("tasks.toml"),
            "[meta]\nplan = \"my-plan\"\nmax_parallel = 1\n\n[[task]]\nid = \"T01\"\ntitle = \"Do something\"\n",
        )
        .unwrap();

        let report = build_audit_report(tmp.path());
        assert_eq!(report.plans_on_disk, 1);
        assert_eq!(report.plans_in_executor, 0);
        assert!(report.orphaned_disk_plans.contains(&"my-plan".to_string()));
    }

    #[test]
    fn audit_detects_orphaned_executor_plan() {
        let tmp = tempfile::tempdir().unwrap();
        let plans_dir = tmp.path().join("plans");
        std::fs::create_dir_all(&plans_dir).unwrap();

        // Write an executor snapshot with a plan that has no disk counterpart.
        let state_dir = tmp.path().join(".roko/state");
        std::fs::create_dir_all(&state_dir).unwrap();
        let executor_json = serde_json::json!({
            "plan_states": {
                "ghost-plan": {
                    "current_phase": { "kind": "implementing" }
                }
            }
        });
        let snapshot = serde_json::json!({
            "version": 1,
            "executor_json": serde_json::to_string(&executor_json).unwrap(),
            "run_state_json": "{}",
        });
        std::fs::write(
            state_dir.join("state-snapshot.json"),
            serde_json::to_string(&snapshot).unwrap(),
        )
        .unwrap();

        let report = build_audit_report(tmp.path());
        assert!(
            report
                .orphaned_executor_plans
                .contains(&"ghost-plan".to_string())
        );
    }

    #[test]
    fn audit_detects_runner_failed_but_toml_ready() {
        let tmp = tempfile::tempdir().unwrap();
        let plan_dir = tmp.path().join("plans/my-plan");
        std::fs::create_dir_all(&plan_dir).unwrap();
        std::fs::write(
            plan_dir.join("tasks.toml"),
            "[meta]\nplan = \"my-plan\"\nmax_parallel = 1\n\n[[task]]\nid = \"T01\"\ntitle = \"Task\"\nstatus = \"ready\"\n",
        )
        .unwrap();

        let state_dir = tmp.path().join(".roko/state");
        std::fs::create_dir_all(&state_dir).unwrap();
        let executor_json = serde_json::json!({
            "plan_states": {
                "my-plan": {
                    "current_phase": { "kind": "queued" }
                }
            }
        });
        let run_state = serde_json::json!({
            "failed_tasks": { "my-plan": ["T01"] },
            "completed_tasks": {},
            "skipped_tasks": {}
        });
        let snapshot = serde_json::json!({
            "version": 1,
            "executor_json": serde_json::to_string(&executor_json).unwrap(),
            "run_state_json": serde_json::to_string(&run_state).unwrap(),
        });
        std::fs::write(
            state_dir.join("state-snapshot.json"),
            serde_json::to_string(&snapshot).unwrap(),
        )
        .unwrap();

        let report = build_audit_report(tmp.path());
        assert_eq!(report.task_drifts.len(), 1);
        assert_eq!(report.task_drifts[0].plan_id, "my-plan");
        assert_eq!(report.task_drifts[0].task_id, "T01");
        assert!(report.task_drifts[0].drift.contains("failure"));
    }

    #[test]
    fn audit_detects_meta_complete_executor_implementing() {
        let tmp = tempfile::tempdir().unwrap();
        let plan_dir = tmp.path().join("plans/drift-plan");
        std::fs::create_dir_all(&plan_dir).unwrap();
        std::fs::write(
            plan_dir.join("tasks.toml"),
            "[meta]\nplan = \"drift-plan\"\nstatus = \"complete\"\nmax_parallel = 1\n\n[[task]]\nid = \"T01\"\ntitle = \"Task\"\n",
        )
        .unwrap();

        let state_dir = tmp.path().join(".roko/state");
        std::fs::create_dir_all(&state_dir).unwrap();
        let executor_json = serde_json::json!({
            "plan_states": {
                "drift-plan": {
                    "current_phase": { "kind": "implementing" }
                }
            }
        });
        let snapshot = serde_json::json!({
            "version": 1,
            "executor_json": serde_json::to_string(&executor_json).unwrap(),
            "run_state_json": "{}",
        });
        std::fs::write(
            state_dir.join("state-snapshot.json"),
            serde_json::to_string(&snapshot).unwrap(),
        )
        .unwrap();

        let report = build_audit_report(tmp.path());
        assert!(!report.plan_drifts.is_empty());
        assert!(
            report.plan_drifts[0]
                .drift
                .contains("meta.status=\"complete\"")
        );
        assert!(
            report.plan_drifts[0]
                .drift
                .contains("executor phase=\"implementing\"")
        );
    }

    #[test]
    fn audit_no_drift_when_consistent() {
        let tmp = tempfile::tempdir().unwrap();
        let plan_dir = tmp.path().join("plans/ok-plan");
        std::fs::create_dir_all(&plan_dir).unwrap();
        std::fs::write(
            plan_dir.join("tasks.toml"),
            "[meta]\nplan = \"ok-plan\"\nstatus = \"ready\"\nmax_parallel = 1\n\n[[task]]\nid = \"T01\"\ntitle = \"Task\"\nstatus = \"ready\"\n",
        )
        .unwrap();

        let state_dir = tmp.path().join(".roko/state");
        std::fs::create_dir_all(&state_dir).unwrap();
        let executor_json = serde_json::json!({
            "plan_states": {
                "ok-plan": {
                    "current_phase": { "kind": "implementing" }
                }
            }
        });
        let run_state = serde_json::json!({
            "completed_tasks": {},
            "failed_tasks": {},
            "skipped_tasks": {}
        });
        let snapshot = serde_json::json!({
            "version": 1,
            "executor_json": serde_json::to_string(&executor_json).unwrap(),
            "run_state_json": serde_json::to_string(&run_state).unwrap(),
        });
        std::fs::write(
            state_dir.join("state-snapshot.json"),
            serde_json::to_string(&snapshot).unwrap(),
        )
        .unwrap();

        let report = build_audit_report(tmp.path());
        assert_eq!(report.plan_drifts.len(), 0);
        assert_eq!(report.task_drifts.len(), 0);
        assert!(report.orphaned_disk_plans.is_empty());
        assert!(report.orphaned_executor_plans.is_empty());
    }

    #[test]
    fn runner_task_status_returns_correct_terminal() {
        let mut completed = BTreeMap::new();
        let mut failed = BTreeMap::new();
        let skipped = BTreeMap::new();

        completed.insert("p1".to_string(), BTreeSet::from(["T01".to_string()]));
        failed.insert("p1".to_string(), BTreeSet::from(["T02".to_string()]));

        assert_eq!(
            runner_task_status("p1", "T01", &completed, &failed, &skipped),
            "completed"
        );
        assert_eq!(
            runner_task_status("p1", "T02", &completed, &failed, &skipped),
            "failed"
        );
        assert_eq!(
            runner_task_status("p1", "T03", &completed, &failed, &skipped),
            "none"
        );
        assert_eq!(
            runner_task_status("p2", "T01", &completed, &failed, &skipped),
            "none"
        );
    }

    #[test]
    fn audit_json_output_is_valid() {
        let tmp = tempfile::tempdir().unwrap();
        let plans_dir = tmp.path().join("plans");
        std::fs::create_dir_all(&plans_dir).unwrap();

        let report = build_audit_report(tmp.path());
        let json_str = serde_json::to_string_pretty(&report).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
        assert!(parsed.get("plans_on_disk").is_some());
        assert!(parsed.get("plans_in_executor").is_some());
        assert!(parsed.get("plan_drifts").is_some());
        assert!(parsed.get("task_drifts").is_some());
        assert!(parsed.get("orphaned_executor_plans").is_some());
        assert!(parsed.get("orphaned_disk_plans").is_some());
    }
}
