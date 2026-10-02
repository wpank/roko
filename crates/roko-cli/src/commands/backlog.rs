//! `roko backlog` — batch import, listing, and reconciliation audit.
//!
//! Reads markdown specs from `tmp/backlog/<N>-*.md` files and records each as
//! a PRD idea in `.roko/prd/ideas.md`. Optionally chains through draft, plan,
//! and execution steps.
//!
//! The `audit` subcommand reconciles plan TOML status against the Graph runs
//! on record (`.roko/state/graph/`), reporting each mismatch with a stable
//! code.

use anyhow::{Context, Result};
use clap::Subcommand;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use roko_cli::graph_checkpoint::{
    CheckpointInspection, GraphCheckpointStatus, canonical_checkpoint_plans,
    canonical_task_outcomes, inspect_canonical_checkpoint,
};
use roko_cli::orchestrator::plan_discovery::{DiscoveryError, find_plan_dirs};
use roko_cli::plan_generate::DEFAULT_BACKLOG_DIR;
use roko_cli::task_parser::TasksFile;
use roko_graph::cells::task_executor::TaskGateVerdict;

use crate::{Cli, resolve_workdir};

// -----------------------------------------------------------------------
// Backlog import
// -----------------------------------------------------------------------

#[derive(Debug, Subcommand)]
pub(crate) enum BacklogCmd {
    /// Import backlog spec(s) as plan artifacts with eligibility checks.
    Import {
        /// Path to a single backlog .md file or a directory containing them.
        path: PathBuf,
        /// Create/update the plan artifact without execution.
        #[arg(long)]
        draft: bool,
        /// Alias for --draft (deprecated; use --draft).
        #[arg(long)]
        plan: bool,
        /// Create then start an eligible packet (fails on blocked packets).
        #[arg(long)]
        execute: bool,
        /// Dry-run: check eligibility without side effects.
        #[arg(long)]
        check: bool,
        /// Working directory (default: cwd / --repo).
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
    /// List backlog items with their status and import state.
    List {
        /// Backlog directory (default: tmp/backlog); its archive/ is listed too.
        path: Option<PathBuf>,
        /// Working directory (default: cwd / --repo).
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
    /// Reconcile plan TOML status against the Graph runs on record.
    ///
    /// Walks every tasks.toml in the plans directory, plan sets included, and
    /// compares its task and meta statuses with the plan's Graph checkpoint in
    /// .roko/state/graph/. Reports each mismatch with a stable code, such as
    /// AUDIT_RUN_SUCCEEDED_TOML_READY, and exits 1 when any is an error.
    Audit {
        /// Working directory (default: cwd / --repo).
        #[arg(long)]
        workdir: Option<PathBuf>,
        /// Emit machine-readable JSON instead of text.
        #[arg(long)]
        json: bool,
        /// Apply deterministic mechanical repairs: remove broken plan
        /// references from the index, deduplicate IDs, and fix spec counts.
        /// Never changes semantic status (use `mark-done` for that).
        #[arg(long)]
        fix_safe: bool,
    },
    /// Mark a backlog spec as done with explicit evidence.
    ///
    /// Finds the backlog file by its numeric ID (e.g. `229`) and writes or
    /// updates the `**Status**: Done (DATE) -- EVIDENCE` line near the top.
    MarkDone {
        /// Numeric backlog ID (e.g. 229).
        id: u32,
        /// Evidence string (run-id, commit hash, PR number, etc.).
        evidence: String,
        /// Working directory (default: cwd / --repo).
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
}

/// Dispatch backlog subcommands.
pub(crate) async fn cmd_backlog(cli: &Cli, cmd: BacklogCmd) -> Result<i32> {
    match cmd {
        BacklogCmd::Import {
            path,
            draft,
            plan,
            execute,
            check,
            workdir,
        } => {
            let wd = workdir.unwrap_or_else(|| resolve_workdir(cli));
            cmd_backlog_import(&wd, &path, draft, plan, execute, check).await
        }
        BacklogCmd::List { path, workdir } => {
            let wd = workdir.unwrap_or_else(|| resolve_workdir(cli));
            cmd_backlog_list(&wd, path.as_deref())
        }
        BacklogCmd::Audit {
            workdir,
            json,
            fix_safe,
        } => {
            let wd = workdir.unwrap_or_else(|| resolve_workdir(cli));
            cmd_backlog_audit(&wd, json, fix_safe)
        }
        BacklogCmd::MarkDone {
            id,
            evidence,
            workdir,
        } => {
            let wd = workdir.unwrap_or_else(|| resolve_workdir(cli));
            cmd_backlog_mark_done(&wd, id, &evidence)
        }
    }
}

/// The backlog directory: `path` (relative to `workdir`), or
/// [`DEFAULT_BACKLOG_DIR`].
fn resolve_backlog_dir(workdir: &Path, path: Option<&Path>) -> PathBuf {
    workdir.join(path.unwrap_or(Path::new(DEFAULT_BACKLOG_DIR)))
}

/// One backlog spec as `backlog list` shows it.
#[derive(Debug)]
struct BacklogSpec {
    id: u32,
    slug: String,
    /// Whether the spec sits in the backlog's `archive/`.
    archived: bool,
    /// The spec's `**Status**:` line, as `mark-done` writes it, without its
    /// label.
    status: Option<String>,
}

/// List backlog specs with their status and import state.
fn cmd_backlog_list(workdir: &Path, path: Option<&Path>) -> Result<i32> {
    let backlog_dir = resolve_backlog_dir(workdir, path);
    if !backlog_dir.is_dir() {
        println!("No backlog directory found at {}", backlog_dir.display());
        return Ok(0);
    }

    let specs = list_backlog_specs(&backlog_dir)?;
    // `backlog import` records each spec as an idea in the PRD ideas file.
    let ideas =
        std::fs::read_to_string(roko_cli::workspace_paths::ideas_path(workdir)).unwrap_or_default();

    println!("Backlog specs ({} items):", specs.len());
    println!("{:<6} {:<50} {:<9} {}", "ID", "Slug", "Imported", "Status");
    println!("{}", "-".repeat(90));

    for spec in &specs {
        let slug = if spec.archived {
            format!("archive/{}", spec.slug)
        } else {
            spec.slug.clone()
        };
        let imported = if has_imported_idea(&ideas, spec.id) {
            "imported"
        } else {
            "-"
        };
        let status = spec.status.as_deref().unwrap_or("-");
        println!("#{:<5} {:<50} {:<9} {}", spec.id, slug, imported, status);
    }

    Ok(0)
}

/// The specs in `backlog_dir` and its `archive/`, by id: the files whose
/// name [`parse_backlog_filename`] accepts.
fn list_backlog_specs(backlog_dir: &Path) -> Result<Vec<BacklogSpec>> {
    let mut specs = Vec::new();
    for (dir, archived) in [
        (backlog_dir.to_path_buf(), false),
        (backlog_dir.join("archive"), true),
    ] {
        if !dir.is_dir() {
            continue;
        }
        for (id, slug, path) in backlog_files_in(&dir)? {
            let content = std::fs::read_to_string(&path)
                .with_context(|| format!("read {}", path.display()))?;
            specs.push(BacklogSpec {
                id,
                slug,
                archived,
                status: spec_status(&content),
            });
        }
    }
    specs.sort_by(|a, b| (a.id, a.archived, &a.slug).cmp(&(b.id, b.archived, &b.slug)));
    Ok(specs)
}

/// The text of `content`'s `**Status**:` line, the one `mark-done` writes. A
/// blockquoted status line is prose, as in [`upsert_status_line`].
fn spec_status(content: &str) -> Option<String> {
    content
        .lines()
        .find_map(|line| line.trim().strip_prefix("**Status**:"))
        .map(|status| status.trim().to_string())
        .filter(|status| !status.is_empty())
}

/// Whether `ideas`, the PRD ideas file that `backlog import` appends to,
/// holds the idea imported from backlog spec `backlog_num`.
fn has_imported_idea(ideas: &str, backlog_num: u32) -> bool {
    ideas.contains(&format!("[backlog#{backlog_num}]"))
}

/// The PRD idea `backlog import` records for spec `num`; `backlog list`
/// finds the import by its `[backlog#N]` marker.
fn backlog_idea_text(num: u32, title: &str) -> String {
    format!("[backlog#{num}] {title}")
}

/// Import backlog spec(s) as PRD ideas. With `check`, print the ideas an
/// import would record and write nothing.
async fn cmd_backlog_import(
    workdir: &Path,
    path: &Path,
    draft: bool,
    plan: bool,
    execute: bool,
    check: bool,
) -> Result<i32> {
    let files = collect_backlog_files(workdir, path)?;

    if files.is_empty() {
        println!("No backlog spec files found at {}", path.display());
        return Ok(1);
    }

    if check {
        let ideas_path = roko_cli::workspace_paths::ideas_path(workdir);
        let ideas = std::fs::read_to_string(&ideas_path).unwrap_or_default();
        println!(
            "Would import {} backlog spec(s) as ideas in {} (--check: nothing written):\n",
            files.len(),
            ideas_path.display()
        );
        for (num, slug, filepath) in &files {
            let content = std::fs::read_to_string(filepath)
                .with_context(|| format!("read {}", filepath.display()))?;
            let title = extract_title(&content).unwrap_or_else(|| slug.clone());
            let note = if has_imported_idea(&ideas, *num) {
                " (already imported)"
            } else {
                ""
            };
            println!("  #{num}: {}{note}", backlog_idea_text(*num, &title));
        }
        return Ok(0);
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
        let idea_text = backlog_idea_text(*num, &title);
        match roko_cli::prd::cmd_idea(workdir, &idea_text, false) {
            Ok(()) => {
                imported += 1;
                println!("  #{}: {}", num, title);
            }
            Err(e) => {
                tracing::error!(num, error = %e, "failed to import backlog item as PRD idea");
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

    if resolved.is_file() {
        return Ok(parse_backlog_filename(&resolved).into_iter().collect());
    }
    if resolved.is_dir() {
        return backlog_files_in(&resolved);
    }
    Ok(Vec::new())
}

/// The backlog spec files directly in `dir`, by id.
fn backlog_files_in(dir: &Path) -> Result<Vec<(u32, String, PathBuf)>> {
    let mut files = Vec::new();
    for entry in std::fs::read_dir(dir).with_context(|| format!("read {}", dir.display()))? {
        let path = entry?.path();
        if path.is_file()
            && path.extension().is_some_and(|ext| ext == "md")
            && let Some(parsed) = parse_backlog_filename(&path)
        {
            files.push(parsed);
        }
    }
    files.sort();
    Ok(files)
}

/// Parse a backlog spec's file name: a numeric id, then `-`, `_` or `.` and a
/// slug (`65-cli-verb-consolidation.md`, `01_t0-reflex-store.md`), or the id
/// alone (`65.md`). Id 0 is the prefix of the index and summary files
/// (`00-INDEX.md`, `00-STATUS-SUMMARY.md`), not a spec's id.
fn parse_backlog_filename(path: &Path) -> Option<(u32, String, PathBuf)> {
    let stem = path.file_stem()?.to_str()?;
    let digits = stem.bytes().take_while(u8::is_ascii_digit).count();
    let num: u32 = stem[..digits].parse().ok()?;
    let slug = match stem[digits..].chars().next() {
        None => "",
        Some('-' | '_' | '.') => &stem[digits + 1..],
        Some(_) => return None,
    };
    if num == 0 {
        return None;
    }
    Some((num, slug.to_string(), path.to_path_buf()))
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
// Backlog audit — plan TOML vs Graph run reconciliation
// -----------------------------------------------------------------------

/// What an audit finding reports. The codes are stable: CI and scripts match
/// on them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AuditCode {
    /// A Graph run succeeded, or recorded a passing verdict for the task, but
    /// tasks.toml still says ready or pending.
    RunSucceededTomlReady,
    /// tasks.toml says done, but no Graph run records the task.
    TaskDoneNotRecorded,
    /// The plan's tasks.toml does not parse, or has no `[meta] plan`.
    PlanSkipped,
    /// The plan's Graph checkpoint exists but cannot be read.
    CheckpointUnreadable,
    /// A Graph checkpoint whose plan is not in the plans directory.
    OrphanCheckpoint,
    /// The last Graph run failed the task and tasks.toml still says ready.
    /// The task may be retried, so this only informs.
    RunFailedTomlReady,
}

impl AuditCode {
    const fn as_str(self) -> &'static str {
        match self {
            Self::RunSucceededTomlReady => "AUDIT_RUN_SUCCEEDED_TOML_READY",
            Self::TaskDoneNotRecorded => "AUDIT_TASK_DONE_NOT_RECORDED",
            Self::PlanSkipped => "AUDIT_PLAN_SKIPPED",
            Self::CheckpointUnreadable => "AUDIT_CHECKPOINT_UNREADABLE",
            Self::OrphanCheckpoint => "AUDIT_ORPHAN_CHECKPOINT",
            Self::RunFailedTomlReady => "AUDIT_RUN_FAILED_TOML_READY",
        }
    }

    /// `error` fails the audit; `warning` and `info` do not.
    const fn severity(self) -> &'static str {
        match self {
            Self::RunSucceededTomlReady | Self::PlanSkipped | Self::CheckpointUnreadable => "error",
            Self::TaskDoneNotRecorded => "warning",
            Self::OrphanCheckpoint | Self::RunFailedTomlReady => "info",
        }
    }
}

/// One reconciliation finding.
#[derive(Debug, Clone, serde::Serialize)]
struct AuditFinding {
    /// Stable code, such as `AUDIT_RUN_SUCCEEDED_TOML_READY`.
    code: &'static str,
    /// `error`, `warning` or `info`.
    severity: &'static str,
    plan_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    task_id: Option<String>,
    /// The plan's tasks.toml, or the checkpoint, relative to the workspace.
    path: String,
    /// The Graph run the evidence comes from.
    #[serde(skip_serializing_if = "Option::is_none")]
    run_id: Option<String>,
    evidence: String,
}

impl AuditFinding {
    fn new(
        code: AuditCode,
        plan_id: &str,
        task_id: Option<&str>,
        path: &str,
        run_id: Option<&str>,
        evidence: String,
    ) -> Self {
        Self {
            code: code.as_str(),
            severity: code.severity(),
            plan_id: plan_id.to_string(),
            task_id: task_id.map(str::to_string),
            path: path.to_string(),
            run_id: run_id.map(str::to_string),
            evidence,
        }
    }
}

/// Full audit report.
#[derive(Debug, Clone, serde::Serialize)]
struct AuditReport {
    /// Plans with a tasks.toml in the plans directory, plan sets included.
    plans_on_disk: usize,
    /// Of those, the plans with a Graph checkpoint.
    plans_with_graph_runs: usize,
    findings: Vec<AuditFinding>,
}

impl AuditReport {
    fn count(&self, severity: &str) -> usize {
        self.findings
            .iter()
            .filter(|finding| finding.severity == severity)
            .count()
    }

    fn has_errors(&self) -> bool {
        self.count("error") > 0
    }
}

/// A status a task or plan carries before it has run.
fn is_unstarted_status(status: &str) -> bool {
    matches!(
        status.trim().to_ascii_lowercase().as_str(),
        "" | "ready" | "pending" | "todo"
    )
}

/// A status that claims a task or plan is finished.
fn is_done_status(status: &str) -> bool {
    matches!(
        status.trim().to_ascii_lowercase().as_str(),
        "done" | "complete" | "completed"
    )
}

/// `path` relative to `workdir`, for findings.
fn display_path(workdir: &Path, path: &Path) -> String {
    path.strip_prefix(workdir)
        .unwrap_or(path)
        .display()
        .to_string()
}

/// Whether `verdict` shows a task's work done: a pass, a pass with
/// pre-existing failures, or work already there.
fn is_passing(verdict: TaskGateVerdict) -> bool {
    matches!(
        verdict,
        TaskGateVerdict::Passed
            | TaskGateVerdict::PassedWithPreexistingFailures
            | TaskGateVerdict::AlreadySatisfied
    )
}

/// The tasks whose latest record in `run` carries a passing gate verdict. A
/// task's node is the task id, or `task.<id>.executor` in the rich topology.
fn passed_tasks(
    run: &CheckpointInspection,
    tasks_file: &TasksFile,
) -> BTreeMap<String, TaskGateVerdict> {
    let recorded = &run.recorded;
    let mut passed = BTreeMap::new();
    for task in &tasks_file.tasks {
        let rich_node = format!("task.{}.executor", task.id);
        let verdict = recorded
            .get(&task.id)
            .or_else(|| recorded.get(&rich_node))
            .copied()
            .flatten();
        if let Some(verdict) = verdict
            && is_passing(verdict)
        {
            passed.insert(task.id.clone(), verdict);
        }
    }
    passed
}

/// Compare one plan's tasks.toml statuses with its last Graph run, if any.
fn audit_plan(
    workdir: &Path,
    plan_id: &str,
    path: &str,
    tasks_file: &TasksFile,
    run: Option<&CheckpointInspection>,
    findings: &mut Vec<AuditFinding>,
) {
    let status = run.map(|run| run.manifest.status);
    let run_id = run.map(|run| run.manifest.run_id.as_str());
    let run_label = run_id.unwrap_or_default();
    // An interrupted or cancelled run never counts as done.
    let succeeded = status == Some(GraphCheckpointStatus::Succeeded);
    let passed = run.map_or_else(BTreeMap::new, |run| passed_tasks(run, tasks_file));
    let failed = if status == Some(GraphCheckpointStatus::Failed) {
        canonical_task_outcomes(workdir, plan_id)
            .map(|outcomes| outcomes.failed)
            .unwrap_or_default()
    } else {
        BTreeSet::new()
    };

    let meta_status = tasks_file.meta.status.as_str();
    if succeeded && is_unstarted_status(meta_status) {
        findings.push(AuditFinding::new(
            AuditCode::RunSucceededTomlReady,
            plan_id,
            None,
            path,
            run_id,
            format!("Graph run {run_label} succeeded, but `[meta] status` is `{meta_status}`"),
        ));
    }

    for task in &tasks_file.tasks {
        let task_status = task.status.as_str();
        let verdict = passed.get(&task.id);
        if is_unstarted_status(task_status) && (succeeded || verdict.is_some()) {
            let evidence = match verdict {
                Some(verdict) if !succeeded => format!(
                    "Graph run {run_label} recorded a `{}` verdict for the task, but its status is \
                     `{task_status}`",
                    verdict.as_str()
                ),
                _ => format!(
                    "Graph run {run_label} succeeded, but the task's status is `{task_status}`"
                ),
            };
            findings.push(AuditFinding::new(
                AuditCode::RunSucceededTomlReady,
                plan_id,
                Some(task.id.as_str()),
                path,
                run_id,
                evidence,
            ));
        } else if is_done_status(task_status) && !succeeded && verdict.is_none() {
            let evidence = match status {
                Some(status) => format!(
                    "tasks.toml says `{task_status}`, but Graph run {run_label} ({}) records no \
                     passing verdict for the task",
                    status.as_str()
                ),
                None => format!(
                    "tasks.toml says `{task_status}`, but no Graph run of the plan is recorded"
                ),
            };
            findings.push(AuditFinding::new(
                AuditCode::TaskDoneNotRecorded,
                plan_id,
                Some(task.id.as_str()),
                path,
                run_id,
                evidence,
            ));
        } else if is_unstarted_status(task_status) && failed.contains(&task.id) {
            findings.push(AuditFinding::new(
                AuditCode::RunFailedTomlReady,
                plan_id,
                Some(task.id.as_str()),
                path,
                run_id,
                format!(
                    "Graph run {run_label} failed the task; its status is still `{task_status}`"
                ),
            ));
        }
    }
}

/// Build the full reconciliation report: every plan in the workspace plans
/// directory, plan sets included, against its Graph checkpoint.
fn build_audit_report(workdir: &Path) -> Result<AuditReport> {
    let plans_root = roko_cli::plan::plans_dir(workdir);
    let plan_dirs = match find_plan_dirs(&plans_root) {
        Ok(plan_dirs) => plan_dirs,
        Err(DiscoveryError::DirMissing(_)) => Vec::new(),
        Err(error) => {
            return Err(error)
                .with_context(|| format!("discover plans in {}", plans_root.display()));
        }
    };

    let mut findings = Vec::new();
    let mut plans_on_disk = 0;
    let mut plans_with_graph_runs = 0;
    for plan_dir in plan_dirs.iter().filter(|plan_dir| plan_dir.has_tasks()) {
        plans_on_disk += 1;
        let tasks_path = plan_dir.dir.join("tasks.toml");
        let path = display_path(workdir, &tasks_path);
        let skipped = |evidence: String| {
            AuditFinding::new(
                AuditCode::PlanSkipped,
                &plan_dir.id,
                None,
                &path,
                None,
                evidence,
            )
        };
        let tasks_file = match TasksFile::parse(&tasks_path) {
            Ok(tasks_file) => tasks_file,
            Err(error) => {
                findings.push(skipped(format!("tasks.toml does not parse: {error:#}")));
                continue;
            }
        };
        if tasks_file.meta.plan.trim().is_empty() {
            findings.push(skipped("tasks.toml has no `plan` in `[meta]`".to_string()));
            continue;
        }

        let run = match inspect_canonical_checkpoint(workdir, &plan_dir.id) {
            Ok(run) => run,
            Err(error) => {
                findings.push(AuditFinding::new(
                    AuditCode::CheckpointUnreadable,
                    &plan_dir.id,
                    None,
                    &path,
                    None,
                    format!("{error:#}"),
                ));
                continue;
            }
        };
        if run.is_some() {
            plans_with_graph_runs += 1;
        }
        // Superseded, archived and fixture plans are not on the backlog.
        if matches!(
            tasks_file.meta.status.as_str(),
            "superseded" | "archived" | "fixture"
        ) {
            continue;
        }
        audit_plan(
            workdir,
            &plan_dir.id,
            &path,
            &tasks_file,
            run.as_ref(),
            &mut findings,
        );
    }

    let disk_ids: BTreeSet<&str> = plan_dirs
        .iter()
        .map(|plan_dir| plan_dir.id.as_str())
        .collect();
    for (plan_id, manifest) in canonical_checkpoint_plans(workdir) {
        if !disk_ids.contains(plan_id.as_str()) {
            findings.push(AuditFinding::new(
                AuditCode::OrphanCheckpoint,
                &plan_id,
                None,
                &display_path(workdir, &manifest),
                None,
                format!("a Graph checkpoint, but no plan `{plan_id}` in the plans directory"),
            ));
        }
    }

    findings
        .sort_by(|a, b| (a.code, &a.plan_id, &a.task_id).cmp(&(b.code, &b.plan_id, &b.task_id)));
    Ok(AuditReport {
        plans_on_disk,
        plans_with_graph_runs,
        findings,
    })
}

/// How many findings of a warning or info code the text report lists; JSON
/// lists them all.
const AUDIT_TEXT_LIMIT: usize = 10;

/// Print the audit report in human-readable text format.
fn print_audit_report(report: &AuditReport) {
    println!("Backlog/Plan State Reconciliation Audit");
    println!("{}", "=".repeat(50));
    println!();
    println!(
        "Plans on disk: {}    With a Graph run: {}",
        report.plans_on_disk, report.plans_with_graph_runs
    );
    println!();

    if report.findings.is_empty() {
        println!("No drift: tasks.toml statuses agree with the Graph runs on record.");
        return;
    }

    // Findings are sorted by code: print them a code at a time.
    let mut rest = report.findings.as_slice();
    while let Some(first) = rest.first() {
        let count = rest
            .iter()
            .take_while(|finding| finding.code == first.code)
            .count();
        let (group, tail) = rest.split_at(count);
        rest = tail;
        let (code, severity) = (first.code, first.severity);
        println!("{code} ({severity}, {count}):");
        let shown = if severity == "error" {
            count
        } else {
            AUDIT_TEXT_LIMIT
        };
        for finding in group.iter().take(shown) {
            let target = match &finding.task_id {
                Some(task_id) => format!("{}:{task_id}", finding.plan_id),
                None => finding.plan_id.clone(),
            };
            println!("  [{target}] {} ({})", finding.evidence, finding.path);
        }
        if count > shown {
            let more = count - shown;
            println!("  ... and {more} more (--json lists them all)");
        }
        println!();
    }

    let errors = report.count("error");
    let warnings = report.count("warning");
    let infos = report.count("info");
    println!("Findings: {errors} error(s), {warnings} warning(s), {infos} info");
}

/// `roko backlog audit` entry point. Exits 1 when any finding is an error, so
/// CI can run it read-only.
fn cmd_backlog_audit(workdir: &Path, json: bool, fix_safe: bool) -> Result<i32> {
    let plans_dir = roko_cli::plan::plans_dir(workdir);
    if !plans_dir.is_dir() {
        println!("No plans directory found at {}", plans_dir.display());
        return Ok(0);
    }

    let report = build_audit_report(workdir)?;

    if json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        print_audit_report(&report);
    }

    if fix_safe {
        let fixes = apply_safe_fixes(workdir)?;
        if fixes == 0 {
            println!("\n--fix-safe: no mechanical repairs needed.");
        } else {
            println!("\n--fix-safe: applied {} mechanical repair(s).", fixes);
        }
    }

    Ok(i32::from(report.has_errors()))
}

// -----------------------------------------------------------------------
// --fix-safe: deterministic mechanical repairs
// -----------------------------------------------------------------------

/// A single safe-fix action that was applied.
#[derive(Debug)]
struct SafeFix {
    description: String,
}

/// Apply deterministic mechanical repairs. Returns the count of fixes applied.
///
/// Safe fixes are limited to:
/// 1. Broken plan references in the index (links to files that don't exist)
/// 2. Duplicate IDs in the index
///
/// This never changes semantic status (done/ready/etc.).
fn apply_safe_fixes(workdir: &Path) -> Result<usize> {
    let mut fixes: Vec<SafeFix> = Vec::new();

    // --- Fix broken index references and duplicate IDs ---
    let index_path = workdir.join("tmp/backlog/00-INDEX.md");
    if index_path.is_file() {
        let content = std::fs::read_to_string(&index_path).context("read 00-INDEX.md")?;
        let (new_content, index_fixes) = fix_broken_index_references(workdir, &content);
        fixes.extend(index_fixes);

        let (new_content, dedup_fixes) = fix_duplicate_index_ids(&new_content);
        fixes.extend(dedup_fixes);

        if !fixes.is_empty() {
            std::fs::write(&index_path, new_content).context("write repaired 00-INDEX.md")?;
        }
    }

    for fix in &fixes {
        println!("  fix: {}", fix.description);
    }

    Ok(fixes.len())
}

/// Scan the index for markdown links like `[#NNN](NNN-slug.md)` that point to
/// files which no longer exist in `tmp/backlog/` (and are not in `archive/`
/// either). Remove such broken references from the table rows.
fn fix_broken_index_references(workdir: &Path, content: &str) -> (String, Vec<SafeFix>) {
    let backlog_dir = workdir.join("tmp/backlog");
    let archive_dir = backlog_dir.join("archive");
    let mut fixes = Vec::new();
    let link_re = regex::Regex::new(r"\[#(\d+)\]\(([^)]+\.md)\)").unwrap();

    let mut result = String::with_capacity(content.len());
    for line in content.lines() {
        // Only inspect table rows (lines starting with `|`)
        if line.trim_start().starts_with('|') {
            let mut broken_in_line = false;
            for cap in link_re.captures_iter(line) {
                let file_ref = &cap[2];
                let full_path = backlog_dir.join(file_ref);
                let archive_path = archive_dir.join(file_ref);
                if !full_path.is_file() && !archive_path.is_file() {
                    broken_in_line = true;
                    fixes.push(SafeFix {
                        description: format!(
                            "removed broken index reference [#{}]({}) -- file not found",
                            &cap[1], file_ref
                        ),
                    });
                }
            }
            if broken_in_line {
                // Drop the entire table row containing the broken reference.
                continue;
            }
        }
        result.push_str(line);
        result.push('\n');
    }

    // Trim trailing newline duplication but ensure file ends with newline.
    let result = result.trim_end().to_string() + "\n";
    (result, fixes)
}

/// Detect duplicate backlog IDs in the index (same `#NNN` appearing in
/// multiple table rows) and remove the later occurrences.
fn fix_duplicate_index_ids(content: &str) -> (String, Vec<SafeFix>) {
    let link_re = regex::Regex::new(r"\[#(\d+)\]").unwrap();
    let mut seen_ids: BTreeSet<u32> = BTreeSet::new();
    let mut fixes = Vec::new();

    let mut result = String::with_capacity(content.len());
    for line in content.lines() {
        if line.trim_start().starts_with('|') {
            // Extract all IDs from table row.
            let ids_in_row: Vec<u32> = link_re
                .captures_iter(line)
                .filter_map(|cap| cap[1].parse::<u32>().ok())
                .collect();

            let mut is_dup = false;
            for id in &ids_in_row {
                if !seen_ids.insert(*id) {
                    is_dup = true;
                    fixes.push(SafeFix {
                        description: format!(
                            "removed duplicate index row for #{} (already listed above)",
                            id
                        ),
                    });
                }
            }
            if is_dup {
                continue;
            }
        }
        result.push_str(line);
        result.push('\n');
    }

    let result = result.trim_end().to_string() + "\n";
    (result, fixes)
}

// -----------------------------------------------------------------------
// mark-done: semantic status update with explicit evidence
// -----------------------------------------------------------------------

/// `roko backlog mark-done <id> <evidence>` entry point.
fn cmd_backlog_mark_done(workdir: &Path, id: u32, evidence: &str) -> Result<i32> {
    let backlog_dir = resolve_backlog_dir(workdir, None);
    if !backlog_dir.is_dir() {
        anyhow::bail!("No backlog directory found at {}", backlog_dir.display());
    }

    // Find the spec file matching this ID.
    let spec_path = find_backlog_spec(&backlog_dir, id)?;
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let status_line = format!("**Status**: Done ({}) -- {}", today, evidence);

    let content = std::fs::read_to_string(&spec_path)
        .with_context(|| format!("read {}", spec_path.display()))?;

    let new_content = upsert_status_line(&content, &status_line);

    std::fs::write(&spec_path, &new_content)
        .with_context(|| format!("write {}", spec_path.display()))?;

    println!("Marked #{} as done in {}", id, spec_path.display());
    println!("  {}", status_line);

    Ok(0)
}

/// Find the backlog spec file for a given numeric ID, with the file-name
/// grammar of [`parse_backlog_filename`].
fn find_backlog_spec(backlog_dir: &Path, id: u32) -> Result<PathBuf> {
    // Search root backlog dir first, then archive/.
    for dir in [backlog_dir.to_path_buf(), backlog_dir.join("archive")] {
        if !dir.is_dir() {
            continue;
        }
        if let Some((_, _, path)) = backlog_files_in(&dir)?
            .into_iter()
            .find(|(num, _, _)| *num == id)
        {
            return Ok(path);
        }
    }

    anyhow::bail!(
        "no backlog spec found for #{} in {} or {}/archive/",
        id,
        backlog_dir.display(),
        backlog_dir.display()
    )
}

/// Insert or update the `**Status**: ...` line in a backlog spec.
///
/// If an existing `**Status**:` line exists (not inside a blockquote), replace
/// it. Otherwise insert the new status line after the first heading.
fn upsert_status_line(content: &str, status_line: &str) -> String {
    let lines: Vec<&str> = content.lines().collect();
    let mut result = Vec::with_capacity(lines.len() + 1);
    let mut replaced = false;

    for line in &lines {
        let trimmed = line.trim();
        // Match a non-blockquoted **Status**: line.
        if !replaced && trimmed.starts_with("**Status**:") && !trimmed.starts_with('>') {
            result.push(status_line.to_string());
            replaced = true;
            continue;
        }
        result.push(line.to_string());
    }

    if !replaced {
        // Insert after the first `# ...` heading line.
        let mut inserted = false;
        let mut final_result = Vec::with_capacity(result.len() + 2);
        for line in &result {
            final_result.push(line.clone());
            if !inserted && line.trim().starts_with("# ") {
                // Insert a blank line then the status line after the heading.
                final_result.push(String::new());
                final_result.push(status_line.to_string());
                inserted = true;
            }
        }
        if !inserted {
            // No heading found; prepend.
            final_result.insert(0, status_line.to_string());
            final_result.insert(1, String::new());
        }
        result = final_result;
    }

    let mut out = result.join("\n");
    if !out.ends_with('\n') {
        out.push('\n');
    }
    out
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

    // ── List tests ───────────────────────────────────────────────────

    /// bug-053644: `backlog list` reads the backlog and its `archive/`, takes
    /// ids with leading zeros or a `_`, leaves out `00-` index files and
    /// other notes, and shows the status line `mark-done` writes.
    #[test]
    fn backlog_list_reads_archive_and_status_lines() {
        let tmp = tempfile::tempdir().unwrap();
        let backlog_dir = tmp.path().join("tmp/backlog");
        std::fs::create_dir_all(backlog_dir.join("archive")).unwrap();
        std::fs::create_dir_all(backlog_dir.join("_archive")).unwrap();
        for (path, content) in [
            ("00-INDEX.md", "# Index\n"),
            ("00-STATUS-SUMMARY.md", "# Summary\n"),
            ("README.md", "# Backlog\n"),
            ("58-perf.md", "# 58\n**Status**: Done (2026-09-03) -- abc\n"),
            ("7_tiers.md", "# 7\n> **Status**: quoted prose\n"),
            ("archive/01-t0-reflex.md", "# 1\n"),
            ("_archive/README.md", "# Old\n"),
        ] {
            std::fs::write(backlog_dir.join(path), content).unwrap();
        }

        let specs = list_backlog_specs(&backlog_dir).unwrap();
        let slugs: Vec<&str> = specs.iter().map(|spec| spec.slug.as_str()).collect();
        assert_eq!(slugs, ["t0-reflex", "tiers", "perf"]);
        let rows: Vec<_> = specs
            .iter()
            .map(|spec| (spec.id, spec.archived, spec.status.as_deref()))
            .collect();
        assert_eq!(
            rows,
            [
                (1, true, None),
                (7, false, None),
                (58, false, Some("Done (2026-09-03) -- abc")),
            ]
        );
        assert_eq!(
            find_backlog_spec(&backlog_dir, 1).unwrap(),
            backlog_dir.join("archive/01-t0-reflex.md")
        );
    }

    /// bug-053644: `backlog list` finds an import where `backlog import`
    /// writes it, the PRD ideas file.
    #[tokio::test]
    async fn backlog_list_sees_ideas_written_by_import() {
        let tmp = tempfile::tempdir().unwrap();
        let backlog_dir = tmp.path().join("tmp/backlog");
        std::fs::create_dir_all(&backlog_dir).unwrap();
        std::fs::write(backlog_dir.join("58-perf.md"), "# Perf hot path\n").unwrap();

        let path = Path::new("tmp/backlog");
        let code = cmd_backlog_import(tmp.path(), path, false, false, false, false)
            .await
            .unwrap();
        assert_eq!(code, 0);

        let ideas =
            std::fs::read_to_string(roko_cli::workspace_paths::ideas_path(tmp.path())).unwrap();
        assert!(has_imported_idea(&ideas, 58), "{ideas}");
        assert!(!has_imported_idea(&ideas, 5), "{ideas}");
    }

    // ── Import tests ─────────────────────────────────────────────────

    /// bug-255765: `backlog import --check` reports what it would import and
    /// changes no file.
    #[tokio::test]
    async fn backlog_import_check_writes_nothing() {
        let tmp = tempfile::tempdir().unwrap();
        let backlog_dir = tmp.path().join("tmp/backlog");
        std::fs::create_dir_all(&backlog_dir).unwrap();
        std::fs::write(backlog_dir.join("58-perf.md"), "# Perf hot path\n").unwrap();
        let before = tree_snapshot(tmp.path());

        let path = Path::new("tmp/backlog");
        let code = cmd_backlog_import(tmp.path(), path, false, false, false, true)
            .await
            .unwrap();

        assert_eq!(code, 0);
        assert_eq!(tree_snapshot(tmp.path()), before);
        assert!(!tmp.path().join(".roko").exists());
    }

    /// Every file under `root` with its contents, by path.
    fn tree_snapshot(root: &Path) -> Vec<(PathBuf, Vec<u8>)> {
        let mut files = Vec::new();
        let mut dirs = vec![root.to_path_buf()];
        while let Some(dir) = dirs.pop() {
            for entry in std::fs::read_dir(&dir).unwrap() {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    dirs.push(path);
                } else {
                    let content = std::fs::read(&path).unwrap();
                    files.push((path, content));
                }
            }
        }
        files.sort();
        files
    }

    // ── Audit tests ──────────────────────────────────────────────────

    /// Write `plans/<rel_dir>/tasks.toml`.
    fn write_plan(workdir: &Path, rel_dir: &str, tasks_toml: &str) {
        let dir = workdir.join("plans").join(rel_dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("tasks.toml"), tasks_toml).unwrap();
    }

    /// Write the canonical Graph checkpoint of `plan_id`, run `run-1`, and
    /// return its directory.
    fn write_checkpoint(workdir: &Path, plan_id: &str, status: &str) -> PathBuf {
        let dir = workdir.join(".roko/state/graph").join(plan_id);
        std::fs::create_dir_all(&dir).unwrap();
        let manifest = serde_json::json!({
            "schema_version": 3,
            "plan_id": plan_id,
            "graph_fingerprint": "fingerprint",
            "run_id": "run-1",
            "activity_log": "activities.jsonl",
            "status": status,
            "updated_at_ms": 0,
        });
        std::fs::write(dir.join("checkpoint.json"), manifest.to_string()).unwrap();
        dir
    }

    /// Record, in run `run-1`, an output of `node_id` stamped `verdict`.
    fn record_verdict(checkpoint_dir: &Path, node_id: &str, verdict: TaskGateVerdict) {
        let mut signals = vec![
            roko_core::Signal::builder(roko_core::Kind::AgentOutput)
                .body(roko_core::Body::text("done"))
                .build(),
        ];
        verdict.stamp(&mut signals);
        let record = serde_json::json!({
            "graph_id": "graph",
            "run_id": "run-1",
            "node_id": node_id,
            "tick": 0,
            "signals": signals,
        });
        let log = checkpoint_dir.join("activities.jsonl");
        std::fs::write(log, format!("{record}\n")).unwrap();
    }

    fn codes(report: &AuditReport) -> Vec<(&str, Option<&str>)> {
        report
            .findings
            .iter()
            .map(|finding| (finding.code, finding.task_id.as_deref()))
            .collect()
    }

    const READY_PLAN: &str = "[meta]\nplan = \"01-x\"\nstatus = \"ready\"\n\n\
        [[task]]\nid = \"T1\"\ntitle = \"First\"\nstatus = \"ready\"\n\n\
        [[task]]\nid = \"T2\"\ntitle = \"Second\"\nstatus = \"ready\"\n";

    #[test]
    fn audit_empty_workspace_reports_no_findings() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join("plans")).unwrap();

        let report = build_audit_report(tmp.path()).unwrap();
        assert_eq!(report.plans_on_disk, 0);
        assert_eq!(report.plans_with_graph_runs, 0);
        assert!(report.findings.is_empty());
        assert!(!report.has_errors());
    }

    /// gap-759041: a plan in a plan set whose Graph run succeeded, while its
    /// tasks.toml still says `ready`, is an error.
    #[test]
    fn audit_reports_graph_completed_plan_left_ready() {
        let tmp = tempfile::tempdir().unwrap();
        write_plan(tmp.path(), "set/01-x", READY_PLAN);
        write_checkpoint(tmp.path(), "01-x", "succeeded");

        let report = build_audit_report(tmp.path()).unwrap();
        assert_eq!(report.plans_on_disk, 1);
        assert_eq!(report.plans_with_graph_runs, 1);
        let ready = "AUDIT_RUN_SUCCEEDED_TOML_READY";
        assert_eq!(
            codes(&report),
            [(ready, None), (ready, Some("T1")), (ready, Some("T2"))]
        );
        let finding = &report.findings[1];
        assert_eq!(finding.plan_id, "01-x");
        assert_eq!(finding.path, "plans/set/01-x/tasks.toml");
        assert_eq!(finding.run_id.as_deref(), Some("run-1"));
        assert!(report.has_errors());
    }

    /// An interrupted run never makes its plan done; only a task whose
    /// output carries a passing verdict counts.
    #[test]
    fn audit_counts_only_passed_tasks_of_an_interrupted_run() {
        let tmp = tempfile::tempdir().unwrap();
        write_plan(tmp.path(), "01-x", READY_PLAN);
        let checkpoint_dir = write_checkpoint(tmp.path(), "01-x", "interrupted");
        record_verdict(&checkpoint_dir, "T1", TaskGateVerdict::Passed);

        let report = build_audit_report(tmp.path()).unwrap();
        assert_eq!(
            codes(&report),
            [("AUDIT_RUN_SUCCEEDED_TOML_READY", Some("T1"))]
        );
        assert!(report.findings[0].evidence.contains("`passed` verdict"));
    }

    #[test]
    fn audit_reports_done_task_without_a_run_as_a_warning() {
        let tmp = tempfile::tempdir().unwrap();
        write_plan(
            tmp.path(),
            "01-x",
            "[meta]\nplan = \"01-x\"\n\n\
             [[task]]\nid = \"T1\"\ntitle = \"First\"\nstatus = \"done\"\n",
        );

        let report = build_audit_report(tmp.path()).unwrap();
        assert_eq!(
            codes(&report),
            [("AUDIT_TASK_DONE_NOT_RECORDED", Some("T1"))]
        );
        assert!(!report.has_errors());
    }

    #[test]
    fn audit_reports_unparseable_and_planless_tasks_toml() {
        let tmp = tempfile::tempdir().unwrap();
        write_plan(tmp.path(), "broken", "[meta\nplan = ");
        write_plan(
            tmp.path(),
            "no-plan",
            "[meta]\n\n[[task]]\nid = \"T1\"\ntitle = \"x\"\n",
        );

        let report = build_audit_report(tmp.path()).unwrap();
        let plans: Vec<(&str, &str)> = report
            .findings
            .iter()
            .map(|finding| (finding.code, finding.plan_id.as_str()))
            .collect();
        assert_eq!(
            plans,
            [
                ("AUDIT_PLAN_SKIPPED", "broken"),
                ("AUDIT_PLAN_SKIPPED", "no-plan"),
            ]
        );
        assert!(report.has_errors());
    }

    #[test]
    fn audit_reports_orphan_checkpoint_as_info() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join("plans")).unwrap();
        write_checkpoint(tmp.path(), "gone", "succeeded");

        let report = build_audit_report(tmp.path()).unwrap();
        assert_eq!(codes(&report), [("AUDIT_ORPHAN_CHECKPOINT", None)]);
        assert_eq!(report.findings[0].plan_id, "gone");
        assert!(!report.has_errors());
    }

    #[test]
    fn audit_json_lists_findings() {
        let tmp = tempfile::tempdir().unwrap();
        write_plan(tmp.path(), "01-x", READY_PLAN);
        write_checkpoint(tmp.path(), "01-x", "succeeded");

        let report = build_audit_report(tmp.path()).unwrap();
        let json = serde_json::to_value(&report).unwrap();
        assert_eq!(json["plans_on_disk"], 1);
        assert_eq!(json["plans_with_graph_runs"], 1);
        let finding = &json["findings"][0];
        assert_eq!(finding["code"], "AUDIT_RUN_SUCCEEDED_TOML_READY");
        assert_eq!(finding["severity"], "error");
        assert_eq!(finding["run_id"], "run-1");
        assert!(finding.get("task_id").is_none(), "{finding}");
    }

    // ── fix-safe tests ──────────────────────────────────────────────

    #[test]
    fn fix_safe_removes_broken_index_references() {
        let content = "# Index\n\
                        \n\
                        | # | Title | Size |\n\
                        |---|---|---|\n\
                        | 10 | [#10](10-exists.md) | S |\n\
                        | 20 | [#20](20-gone.md) | M |\n\
                        | 30 | [#30](30-also-exists.md) | S |\n";

        let tmp = tempfile::tempdir().unwrap();
        let backlog_dir = tmp.path().join("tmp/backlog");
        std::fs::create_dir_all(&backlog_dir).unwrap();
        // Create only the files that should exist.
        std::fs::write(backlog_dir.join("10-exists.md"), "# Exists").unwrap();
        std::fs::write(backlog_dir.join("30-also-exists.md"), "# Also").unwrap();

        let (result, fixes) = fix_broken_index_references(tmp.path(), content);
        assert_eq!(fixes.len(), 1);
        assert!(fixes[0].description.contains("#20"));
        assert!(!result.contains("20-gone.md"));
        assert!(result.contains("10-exists.md"));
        assert!(result.contains("30-also-exists.md"));
    }

    #[test]
    fn fix_safe_keeps_archived_references() {
        let content = "| 10 | [#10](10-archived.md) | S |\n";

        let tmp = tempfile::tempdir().unwrap();
        let backlog_dir = tmp.path().join("tmp/backlog");
        let archive_dir = backlog_dir.join("archive");
        std::fs::create_dir_all(&archive_dir).unwrap();
        // File exists in archive/, not root.
        std::fs::write(archive_dir.join("10-archived.md"), "# Archived").unwrap();

        let (result, fixes) = fix_broken_index_references(tmp.path(), content);
        assert_eq!(fixes.len(), 0);
        assert!(result.contains("10-archived.md"));
    }

    #[test]
    fn fix_safe_removes_duplicate_ids() {
        let content = "# Index\n\
                        \n\
                        | # | Title | Size |\n\
                        |---|---|---|\n\
                        | 10 | [#10](10-first.md) | S |\n\
                        | 20 | [#20](20-item.md) | M |\n\
                        | 10 | [#10](10-first.md) | S |\n";

        let (result, fixes) = fix_duplicate_index_ids(content);
        assert_eq!(fixes.len(), 1);
        assert!(fixes[0].description.contains("#10"));
        // First occurrence kept, second removed.
        assert!(result.contains("10-first.md"));
        assert!(result.contains("20-item.md"));
        // Only one occurrence of the table row for #10.
        assert_eq!(result.matches("10-first.md").count(), 1);
    }

    #[test]
    fn fix_safe_idempotent() {
        let tmp = tempfile::tempdir().unwrap();
        let backlog_dir = tmp.path().join("tmp/backlog");
        std::fs::create_dir_all(&backlog_dir).unwrap();
        std::fs::write(backlog_dir.join("10-item.md"), "# Item 10").unwrap();

        let content = "# Index\n\
                        \n\
                        | # | Title | Size |\n\
                        |---|---|---|\n\
                        | 10 | [#10](10-item.md) | S |\n";

        let (result, fixes) = fix_broken_index_references(tmp.path(), content);
        assert_eq!(fixes.len(), 0);
        let (result2, fixes2) = fix_duplicate_index_ids(&result);
        assert_eq!(fixes2.len(), 0);
        // Content is unchanged after a second pass.
        let (result3, fixes3) = fix_broken_index_references(tmp.path(), &result2);
        assert_eq!(fixes3.len(), 0);
        assert_eq!(result2, result3);
    }

    // ── mark-done tests ─────────────────────────────────────────────

    #[test]
    fn upsert_status_replaces_existing_line() {
        let content = "# 229 -- Some Feature\n\
                        \n\
                        **Status**: Not started\n\
                        **Priority**: P1\n\
                        **Size**: M\n";

        let result = upsert_status_line(content, "**Status**: Done (2026-09-07) -- abc123");
        assert!(result.contains("**Status**: Done (2026-09-07) -- abc123"));
        assert!(!result.contains("Not started"));
        // Preserves other lines.
        assert!(result.contains("**Priority**: P1"));
    }

    #[test]
    fn upsert_status_inserts_after_heading_when_absent() {
        let content = "# 100 -- No Status Line\n\
                        \n\
                        **Priority**: P2\n";

        let result = upsert_status_line(content, "**Status**: Done (2026-09-07) -- xyz");
        assert!(result.contains("**Status**: Done (2026-09-07) -- xyz"));
        // Should appear after the heading.
        let heading_pos = result.find("# 100").unwrap();
        let status_pos = result.find("**Status**:").unwrap();
        assert!(status_pos > heading_pos);
    }

    #[test]
    fn upsert_status_preserves_blockquoted_status() {
        let content = "# 212 -- Some Feature\n\
                        \n\
                        > **Status: SOURCE-DONE** old blockquote\n\
                        \n\
                        **Status**: Verified (2026-09-03) -- evidence\n\
                        **Priority**: P2\n";

        let result = upsert_status_line(content, "**Status**: Done (2026-09-07) -- new-evidence");
        // Blockquoted status line should be preserved.
        assert!(result.contains("> **Status: SOURCE-DONE** old blockquote"));
        // Non-blockquoted line should be replaced.
        assert!(result.contains("**Status**: Done (2026-09-07) -- new-evidence"));
        assert!(!result.contains("Verified (2026-09-03)"));
    }

    #[test]
    fn find_backlog_spec_finds_by_id() {
        let tmp = tempfile::tempdir().unwrap();
        let backlog_dir = tmp.path().join("tmp/backlog");
        std::fs::create_dir_all(&backlog_dir).unwrap();
        std::fs::write(
            backlog_dir.join("229-backlog-plan-state-reconciliation.md"),
            "# 229\n",
        )
        .unwrap();

        let found = find_backlog_spec(&backlog_dir, 229).unwrap();
        assert!(
            found
                .to_str()
                .unwrap()
                .contains("229-backlog-plan-state-reconciliation.md")
        );
    }

    #[test]
    fn find_backlog_spec_checks_archive() {
        let tmp = tempfile::tempdir().unwrap();
        let backlog_dir = tmp.path().join("tmp/backlog");
        let archive_dir = backlog_dir.join("archive");
        std::fs::create_dir_all(&archive_dir).unwrap();
        std::fs::write(archive_dir.join("50-old-item.md"), "# 50\n").unwrap();

        let found = find_backlog_spec(&backlog_dir, 50).unwrap();
        assert!(found.to_str().unwrap().contains("50-old-item.md"));
    }

    #[test]
    fn find_backlog_spec_returns_error_for_missing() {
        let tmp = tempfile::tempdir().unwrap();
        let backlog_dir = tmp.path().join("tmp/backlog");
        std::fs::create_dir_all(&backlog_dir).unwrap();

        let result = find_backlog_spec(&backlog_dir, 999);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("#999"));
    }

    #[test]
    fn mark_done_writes_status_to_spec_file() {
        let tmp = tempfile::tempdir().unwrap();
        let backlog_dir = tmp.path().join("tmp/backlog");
        std::fs::create_dir_all(&backlog_dir).unwrap();
        std::fs::write(
            backlog_dir.join("42-test-item.md"),
            "# 42 -- Test Item\n\n**Priority**: P1\n**Size**: S\n",
        )
        .unwrap();

        let result = cmd_backlog_mark_done(tmp.path(), 42, "commit abc123");
        assert!(result.is_ok());

        let content = std::fs::read_to_string(backlog_dir.join("42-test-item.md")).unwrap();
        assert!(content.contains("**Status**: Done ("));
        assert!(content.contains("commit abc123"));
        // Priority line preserved.
        assert!(content.contains("**Priority**: P1"));
    }
}
