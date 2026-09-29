//! Auto-maintained indexes for PRDs, plans, research, and tasks.
//!
//! Every time roko creates or modifies a PRD, plan, research artifact, or task,
//! the relevant index is rebuilt. Indexes are both human-readable (markdown)
//! and machine-parseable (structured sections with consistent formatting).
//!
//! Index files serve as:
//! 1. **Discovery** — what exists, where it lives
//! 2. **Dedup** — agents read the index before creating anything new
//! 3. **Context** — injected into agent prompts so they know the full picture
//! 4. **Cross-references** — which PRDs link to which plans, etc.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use crate::orchestrator::DiscoveryError;
use crate::orchestrator::plan_discovery::{PlanDir, find_plan_dirs};
use crate::workspace_paths::{drafts_dir, ideas_path, plans_dir, prd_dir, published_dir, roko_dir};
use anyhow::{Context, Result};

// ─── Index paths ───────────────────────────────────────────────────

fn master_index_path(workdir: &Path) -> PathBuf {
    roko_dir(workdir).join("INDEX.md")
}
fn prd_index_path(workdir: &Path) -> PathBuf {
    prd_dir(workdir).join("INDEX.md")
}
fn plans_index_path(workdir: &Path) -> PathBuf {
    plans_dir(workdir).join("INDEX.md")
}
fn research_index_path(workdir: &Path) -> PathBuf {
    workdir.join(".roko").join("research").join("INDEX.md")
}

/// Append the master index to a prompt when it exists and is non-empty.
pub fn append_master_index_prompt(out: &mut String, workdir: &Path, heading: &str) {
    let master_index = std::fs::read_to_string(master_index_path(workdir)).unwrap_or_default();
    if master_index.trim().is_empty() {
        return;
    }
    let _ = writeln!(out, "{heading}\n{master_index}\n---\n");
}

// ─── PRD index ─────────────────────────────────────────────────────

/// Rebuild `.roko/prd/INDEX.md` from all published + draft PRDs.
pub fn rebuild_prd_index(workdir: &Path) -> Result<()> {
    let mut out = String::new();
    let _ = writeln!(out, "# PRD Index");
    let _ = writeln!(out, "\n> Auto-generated. Do not edit manually.");
    let _ = writeln!(out, "> Rebuilt on every `roko prd` command.\n");

    // Ideas count
    let ideas = ideas_path(workdir);
    let idea_count = std::fs::read_to_string(&ideas)
        .unwrap_or_default()
        .lines()
        .filter(|l| l.starts_with("- "))
        .count();
    let _ = writeln!(out, "**Ideas**: {idea_count} captured in `ideas.md`\n");

    // Published
    let _ = writeln!(out, "## Published\n");
    let _ = writeln!(out, "| Slug | Title | Crates | Plans | Coverage |");
    let _ = writeln!(out, "|------|-------|--------|-------|----------|");
    let published = list_md_sorted(&published_dir(workdir));
    if published.is_empty() {
        let _ = writeln!(out, "| _(none)_ | | | | |");
    }
    for path in &published {
        let slug = file_slug(path);
        let meta = read_frontmatter(path);
        let _ = writeln!(
            out,
            "| `{slug}` | {} | {} | {} | {} |",
            meta.title,
            meta.crates,
            meta.plans_generated,
            if meta.coverage > 0.0 {
                format!("{:.0}%", meta.coverage * 100.0)
            } else {
                "—".into()
            }
        );
    }

    // Drafts
    let _ = writeln!(out, "\n## Drafts\n");
    let _ = writeln!(out, "| Slug | Title | Created |");
    let _ = writeln!(out, "|------|-------|---------|");
    let drafts = list_md_sorted(&drafts_dir(workdir));
    if drafts.is_empty() {
        let _ = writeln!(out, "| _(none)_ | | |");
    }
    for path in &drafts {
        let slug = file_slug(path);
        let meta = read_frontmatter(path);
        let _ = writeln!(out, "| `{slug}` | {} | {} |", meta.title, meta.created);
    }

    // Recent ideas (last 10)
    let _ = writeln!(out, "\n## Recent Ideas\n");
    let ideas_content = std::fs::read_to_string(&ideas).unwrap_or_default();
    let ideas: Vec<&str> = ideas_content
        .lines()
        .filter(|l| l.starts_with("- "))
        .collect();
    let start = ideas.len().saturating_sub(10);
    for line in &ideas[start..] {
        let _ = writeln!(out, "{line}");
    }

    let idx = prd_index_path(workdir);
    if let Some(parent) = idx.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&idx, &out)?;
    Ok(())
}

// ─── Plans index ───────────────────────────────────────────────────

/// Render the plans index deterministically without mutating the workspace.
pub fn render_plans_index(workdir: &Path) -> Result<String> {
    let mut out = String::new();
    let _ = writeln!(out, "# Plans Index");
    let _ = writeln!(out, "\n> Auto-generated. Do not edit manually.");
    let _ = writeln!(
        out,
        "> Rebuilt after successful mutating `roko plan` commands."
    );
    if plans_dir(workdir)
        .join("_meta/IMPLEMENTATION_ORDER.md")
        .is_file()
    {
        let _ = writeln!(
            out,
            "> Execution order: [`_meta/IMPLEMENTATION_ORDER.md`](_meta/IMPLEMENTATION_ORDER.md)."
        );
    }
    let _ = writeln!(
        out,
        "> Executable totals exclude superseded, archived, and fixture plans, and plans that \
         `roko plan run` cannot load.\n"
    );

    let _ = writeln!(out, "## Executable Plans\n");
    let _ = writeln!(out, "| Plan | Tasks | Done | Ready | Status | Parallel |");
    let _ = writeln!(out, "|------|-------|------|-------|--------|----------|");

    let Some(plan_entries) = collect_plan_index_entries(workdir)? else {
        let _ = writeln!(out, "| _(no plans directory)_ | | | | | |");
        return Ok(out);
    };

    let mut total_tasks = 0u32;
    let mut total_done = 0u32;
    let mut executable_plans = 0u32;
    let mut complete_plans = 0u32;
    let mut ready_plans = 0u32;

    for entry in plan_entries.iter().filter(|entry| entry.is_executable()) {
        let _ = writeln!(
            out,
            "| `{}` | {} | {} | {} | {} | {} |",
            entry.name,
            entry.tasks,
            entry.done,
            entry.ready,
            entry.status_label(),
            entry.max_parallel
        );
        total_tasks += entry.tasks;
        total_done += entry.done;
        executable_plans += 1;
        if entry.is_complete() {
            complete_plans += 1;
        } else {
            ready_plans += 1;
        }
    }

    if executable_plans == 0 {
        let _ = writeln!(out, "| _(none)_ | | | | | |");
    }

    let remaining = total_tasks.saturating_sub(total_done);
    let _ = writeln!(
        out,
        "\n**Executable Total**: {} plans, {} tasks, {} done ({:.0}%), {} remaining",
        executable_plans,
        total_tasks,
        total_done,
        if total_tasks > 0 {
            total_done as f64 / total_tasks as f64 * 100.0
        } else {
            0.0
        },
        remaining
    );
    let _ = writeln!(out, "**Complete Plans**: {complete_plans}");
    let _ = writeln!(out, "**Ready/In-Progress Plans**: {ready_plans}");

    let inactive: Vec<&PlanIndexEntry> = plan_entries
        .iter()
        .filter(|entry| entry.is_inactive())
        .collect();
    if !inactive.is_empty() {
        let inactive_tasks: u32 = inactive.iter().map(|entry| entry.tasks).sum();
        let _ = writeln!(out, "\n## Superseded / Archived\n");
        let _ = writeln!(out, "| Plan | Tasks | Status | Replaced By |");
        let _ = writeln!(out, "|------|-------|--------|-------------|");
        for entry in &inactive {
            let _ = writeln!(
                out,
                "| `{}` | {} | {} | {} |",
                entry.name,
                entry.tasks,
                entry.status_label(),
                entry
                    .superseded_by
                    .as_deref()
                    .filter(|value| !value.trim().is_empty())
                    .unwrap_or("—")
            );
        }
        let _ = writeln!(
            out,
            "\n**Excluded**: {} plans, {} tasks",
            inactive.len(),
            inactive_tasks
        );
    }

    let fixtures: Vec<&PlanIndexEntry> = plan_entries
        .iter()
        .filter(|entry| entry.is_fixture())
        .collect();
    if !fixtures.is_empty() {
        let fixture_tasks: u32 = fixtures.iter().map(|entry| entry.tasks).sum();
        let _ = writeln!(out, "\n## Fixtures / Examples\n");
        let _ = writeln!(out, "| Plan | Tasks | Status |");
        let _ = writeln!(out, "|------|-------|--------|");
        for entry in &fixtures {
            let _ = writeln!(
                out,
                "| `{}` | {} | {} |",
                entry.name,
                entry.tasks,
                entry.status_label()
            );
        }
        let _ = writeln!(
            out,
            "\n**Fixtures excluded from backlog**: {} plans, {} tasks",
            fixtures.len(),
            fixture_tasks
        );
    }

    let not_runnable: Vec<&PlanIndexEntry> = plan_entries
        .iter()
        .filter(|entry| entry.is_not_runnable())
        .collect();
    if !not_runnable.is_empty() {
        let _ = writeln!(out, "\n## Not Runnable\n");
        let _ = writeln!(out, "| Plan | Tasks | Reason |");
        let _ = writeln!(out, "|------|-------|--------|");
        for entry in &not_runnable {
            let _ = writeln!(
                out,
                "| `{}` | {} | {} |",
                entry.name,
                if entry.has_tasks_file {
                    entry.tasks.to_string()
                } else {
                    "—".to_string()
                },
                entry.not_runnable.as_deref().unwrap_or_default()
            );
        }
        let _ = writeln!(
            out,
            "\n**Not runnable, excluded from backlog**: {} plans",
            not_runnable.len()
        );
    }

    Ok(out)
}

/// Rebuild the plans index, `INDEX.md` in the workspace plans directory, from
/// all plan directories. Creates the plans directory when it does not exist.
pub fn rebuild_plans_index(workdir: &Path) -> Result<()> {
    let path = plans_index_path(workdir);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, render_plans_index(workdir)?)?;
    Ok(())
}

/// Verify the plans index exactly matches its deterministic rendering.
///
/// This check never writes the index, including when the file is missing or
/// stale, so it is safe for validation commands and CI drift gates.
pub fn check_plans_index(workdir: &Path) -> Result<()> {
    let path = plans_index_path(workdir);
    let actual = std::fs::read_to_string(&path).map_err(|error| {
        anyhow::anyhow!("read generated plans index {}: {error}", path.display())
    })?;
    let expected = render_plans_index(workdir)?;
    if actual != expected {
        anyhow::bail!(
            "generated plans index is stale: {}; run `roko plan index --workdir {}`",
            path.display(),
            workdir.display()
        );
    }
    Ok(())
}

#[derive(Debug, Clone)]
struct PlanIndexEntry {
    /// The plan directory below the plans root, `/`-separated: `<set>/<plan>`
    /// for a plan inside a plan set.
    name: String,
    tasks: u32,
    done: u32,
    ready: u32,
    max_parallel: String,
    meta_status: String,
    superseded_by: Option<String>,
    /// Whether the plan directory has a `tasks.toml` (a plan may have only a
    /// `plan.md`).
    has_tasks_file: bool,
    /// Why `roko plan run` cannot load the plan; `None` when it can.
    not_runnable: Option<String>,
}

impl PlanIndexEntry {
    fn is_inactive(&self) -> bool {
        matches!(self.meta_status.as_str(), "superseded" | "archived")
    }

    fn is_fixture(&self) -> bool {
        self.meta_status == "fixture"
    }

    /// Backlog plans: active, not fixtures, and loadable by `roko plan run`.
    fn is_executable(&self) -> bool {
        !self.is_inactive() && !self.is_fixture() && self.not_runnable.is_none()
    }

    /// Active, non-fixture plans that `roko plan run` cannot load.
    fn is_not_runnable(&self) -> bool {
        !self.is_inactive() && !self.is_fixture() && self.not_runnable.is_some()
    }

    fn is_complete(&self) -> bool {
        self.meta_status == "done" || (self.tasks > 0 && self.done == self.tasks)
    }

    fn status_label(&self) -> String {
        match self.meta_status.as_str() {
            "superseded" => "⏭ superseded".to_string(),
            "archived" => "🗄 archived".to_string(),
            "fixture" => "🧪 fixture".to_string(),
            "done" => "✅ complete".to_string(),
            status if self.is_complete() => {
                if status.is_empty() {
                    "✅ complete".to_string()
                } else {
                    format!("✅ {status}")
                }
            }
            status if self.done > 0 => {
                if status.is_empty() || status == "ready" {
                    "🔄 in progress".to_string()
                } else {
                    format!("🔄 {status}")
                }
            }
            "ready" | "" => "📋 ready".to_string(),
            status => format!("📋 {status}"),
        }
    }
}

fn collect_plan_index_entries(workdir: &Path) -> Result<Option<Vec<PlanIndexEntry>>> {
    let plans_root = plans_dir(workdir);
    // The plan directories `roko plan list` and `roko plan run` see, including
    // plans inside plan sets and plans that have only a `plan.md`.
    let plan_dirs = match find_plan_dirs(&plans_root) {
        Ok(plan_dirs) => plan_dirs,
        Err(DiscoveryError::DirMissing(_)) => return Ok(None),
        Err(error) => {
            return Err(error)
                .with_context(|| format!("read plans directory {}", plans_root.display()));
        }
    };

    let run_state_completed = load_run_state_completed(workdir)?;
    plan_dirs
        .iter()
        .map(|plan_dir| plan_index_entry(plan_dir, &run_state_completed))
        .collect::<Result<Vec<_>>>()
        .map(Some)
}

fn plan_index_entry(
    plan_dir: &PlanDir,
    run_state_completed: &std::collections::HashMap<String, Vec<String>>,
) -> Result<PlanIndexEntry> {
    let name = plan_dir.group.as_deref().map_or_else(
        || plan_dir.id.clone(),
        |group| format!("{group}/{}", plan_dir.id),
    );
    let tasks_path = plan_dir.dir.join("tasks.toml");
    if !tasks_path.is_file() {
        return Ok(PlanIndexEntry {
            name,
            tasks: 0,
            done: 0,
            ready: 0,
            max_parallel: "—".to_string(),
            meta_status: String::new(),
            superseded_by: None,
            has_tasks_file: false,
            not_runnable: Some("no `tasks.toml`".to_string()),
        });
    }

    let content = std::fs::read_to_string(&tasks_path)
        .with_context(|| format!("read plan tasks input {}", tasks_path.display()))?;
    let parsed = toml::from_str::<toml::Value>(&content)
        .context("invalid TOML")
        .with_context(|| format!("parse plan tasks input {}", tasks_path.display()))?;
    let mut counts = count_top_level_tasks(&parsed)
        .with_context(|| format!("parse plan tasks input {}", tasks_path.display()))?;

    // Overlay real completion data from run-state.json if available.
    if let Some(completed_ids) = run_state_completed.get(&plan_dir.id) {
        if !completed_ids.is_empty() {
            counts.1 = completed_ids.len() as u32;
            counts.2 = counts.0.saturating_sub(counts.1);
        }
    }

    Ok(PlanIndexEntry {
        name,
        tasks: counts.0,
        done: counts.1,
        ready: counts.2,
        max_parallel: extract_meta_value(&parsed, "max_parallel")
            .unwrap_or_else(|| "—".to_string()),
        meta_status: extract_meta_string(&parsed, "status")
            .unwrap_or_default()
            .trim()
            .to_string(),
        superseded_by: extract_meta_string(&parsed, "superseded_by"),
        has_tasks_file: true,
        not_runnable: not_runnable_reason(&parsed, &content),
    })
}

/// Why `roko plan run` cannot load a plan whose `tasks.toml` is valid TOML, or
/// `None` when it can.
///
/// The file must parse as a plan and pass the task schema, as
/// `runner::plan_loader::load_plan` requires, and hold at least one task.
/// `load_plan` also checks the plan's context against the workspace and the
/// environment; the index leaves that out so that it renders the same on every
/// machine.
fn not_runnable_reason(parsed: &toml::Value, content: &str) -> Option<String> {
    let mut problems = Vec::new();
    match (parsed.get("task"), parsed.get("tasks")) {
        (None, Some(_)) => problems.push("`[[tasks]]` instead of `[[task]]`"),
        (None, None) => problems.push("no tasks"),
        (Some(_), _) => {}
    }
    match parsed.get("meta").and_then(toml::Value::as_table) {
        None => problems.push("no `[meta]` table"),
        Some(meta) if !meta.contains_key("plan") => problems.push("no `plan` in `[meta]`"),
        Some(_) => {}
    }
    if !problems.is_empty() {
        return Some(problems.join("; "));
    }

    match crate::task_parser::TasksFile::parse_str(content) {
        Err(_) => Some("does not parse as a plan; run `roko plan validate`".to_string()),
        Ok(tasks_file) if tasks_file.tasks.is_empty() => Some("no tasks".to_string()),
        Ok(tasks_file) => {
            let issues = tasks_file.validate_against_schema().len();
            (issues > 0).then(|| format!("{issues} schema issue(s); run `roko plan validate`"))
        }
    }
}

fn load_run_state_completed(
    workdir: &Path,
) -> Result<std::collections::HashMap<String, Vec<String>>> {
    // tasks.toml is never updated by plan run; completion state lives in
    // run-state.json. This is RunStateSnapshot, not executor.json.
    let run_state_path = workdir.join(".roko/state/run-state.json");
    let content = match std::fs::read_to_string(&run_state_path) {
        Ok(content) => content,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(std::collections::HashMap::new());
        }
        Err(error) => {
            return Err(error)
                .with_context(|| format!("read run-state input {}", run_state_path.display()));
        }
    };
    let value: serde_json::Value = serde_json::from_str(&content)
        .with_context(|| format!("parse run-state input {}", run_state_path.display()))?;
    let completed = value
        .get("completed_tasks")
        .context("run-state input is missing completed_tasks")?;
    serde_json::from_value(completed.clone())
        .with_context(|| format!("parse completed_tasks in {}", run_state_path.display()))
}

/// Count a plan's tasks, done tasks and ready tasks. Tasks written as
/// `[[tasks]]` are counted too, so the index shows what such a plan holds,
/// although `roko plan run` reads only `[[task]]` (see
/// [`not_runnable_reason`]).
fn count_top_level_tasks(parsed: &toml::Value) -> Result<(u32, u32, u32)> {
    let tasks = match parsed.get("task") {
        Some(tasks) => tasks
            .as_array()
            .context("task must be an array of tables")?,
        None => match parsed.get("tasks").and_then(toml::Value::as_array) {
            Some(tasks) => tasks,
            None => return Ok((0, 0, 0)),
        },
    };

    let mut done = 0u32;
    let mut ready = 0u32;
    for task in tasks {
        let table = task.as_table().context("task entry must be a table")?;
        let status = match table.get("status") {
            Some(status) => Some(
                status
                    .as_str()
                    .context("task status must be a string when present")?,
            ),
            None => None,
        };
        match status {
            Some("done") => done += 1,
            Some("ready") | None => ready += 1,
            _ => {}
        }
    }

    Ok((tasks.len() as u32, done, ready))
}

// ─── Research index ────────────────────────────────────────────────

/// Rebuild `.roko/research/INDEX.md` from all research artifacts.
pub fn rebuild_research_index(workdir: &Path) -> Result<()> {
    let mut out = String::new();
    let _ = writeln!(out, "# Research Index");
    let _ = writeln!(out, "\n> Auto-generated. Do not edit manually.");
    let _ = writeln!(out, "> Rebuilt on every `roko research` command.\n");

    let _ = writeln!(out, "| Artifact | Size | Modified |");
    let _ = writeln!(out, "|----------|------|----------|");

    let research_dir = workdir.join(".roko/research");
    let files = list_md_sorted(&research_dir);

    for path in &files {
        let name = file_slug(path);
        if name == "INDEX" {
            continue;
        }
        let size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
        let modified = std::fs::metadata(path)
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| {
                let dt: chrono::DateTime<chrono::Local> = t.into();
                Some(dt.format("%Y-%m-%d %H:%M").to_string())
            })
            .unwrap_or_else(|| "—".into());
        let _ = writeln!(out, "| `{name}` | {size} bytes | {modified} |");
    }

    if files.is_empty() || (files.len() == 1 && file_slug(&files[0]) == "INDEX") {
        let _ = writeln!(out, "| _(none)_ | | |");
    }

    let idx = research_index_path(workdir);
    if let Some(parent) = idx.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&idx, &out)?;
    Ok(())
}

// ─── Master index ──────────────────────────────────────────────────

/// Rebuild `.roko/INDEX.md` — the master index linking everything.
pub fn rebuild_master_index(workdir: &Path) -> Result<()> {
    let mut out = String::new();
    let _ = writeln!(out, "# Roko Master Index");
    let _ = writeln!(out, "\n> Auto-generated. Do not edit manually.");
    let _ = writeln!(out, "> Single entry point for all roko artifacts.\n");

    // PRD summary
    let published_count = list_md_sorted(&published_dir(workdir)).len();
    let drafts_count = list_md_sorted(&drafts_dir(workdir)).len();
    let ideas_count = std::fs::read_to_string(ideas_path(workdir))
        .unwrap_or_default()
        .lines()
        .filter(|l| l.starts_with("- "))
        .count();
    let _ = writeln!(
        out,
        "## PRDs ({published_count} published, {drafts_count} drafts, {ideas_count} ideas)"
    );
    let _ = writeln!(out, "→ [Full index](.roko/prd/INDEX.md)\n");

    // Plans summary
    let plan_entries = collect_plan_index_entries(workdir)?.unwrap_or_default();
    let executable_entries: Vec<&PlanIndexEntry> = plan_entries
        .iter()
        .filter(|entry| entry.is_executable())
        .collect();
    let plan_count = executable_entries.len() as u32;
    let task_count: u32 = executable_entries.iter().map(|entry| entry.tasks).sum();
    let done_count: u32 = executable_entries.iter().map(|entry| entry.done).sum();
    let complete_count = executable_entries
        .iter()
        .filter(|entry| entry.is_complete())
        .count();
    let superseded_count = plan_entries
        .iter()
        .filter(|entry| entry.is_inactive())
        .count();
    let remaining_count = task_count.saturating_sub(done_count);
    let _ = writeln!(
        out,
        "## Plans ({plan_count} executable, {complete_count} complete, {remaining_count} tasks remaining, {superseded_count} superseded)"
    );
    let plans_index = plans_index_path(workdir);
    let _ = writeln!(
        out,
        "→ [Full index]({})\n",
        plans_index
            .strip_prefix(workdir)
            .unwrap_or(&plans_index)
            .display()
    );

    // Research summary
    let research_count = list_md_sorted(&workdir.join(".roko/research"))
        .iter()
        .filter(|p| file_slug(p) != "INDEX")
        .count();
    let _ = writeln!(out, "## Research ({research_count} artifacts)");
    let _ = writeln!(out, "→ [Full index](.roko/research/INDEX.md)\n");

    // Episodes summary
    let episodes_path = workdir.join(".roko/episodes.jsonl");
    let episode_count = if episodes_path.exists() {
        std::fs::read_to_string(&episodes_path)
            .unwrap_or_default()
            .lines()
            .count()
    } else {
        0
    };
    let _ = writeln!(out, "## Episodes ({episode_count} recorded)");
    let _ = writeln!(out, "→ `.roko/episodes.jsonl`\n");

    // Config
    let config_exists = workdir.join("roko.toml").exists();
    let _ = writeln!(out, "## Config");
    let _ = writeln!(
        out,
        "- `roko.toml`: {}",
        if config_exists {
            "✅ present"
        } else {
            "❌ missing"
        }
    );

    std::fs::write(master_index_path(workdir), &out)?;
    Ok(())
}

/// Rebuild ALL indexes. Call this after any mutation.
///
/// The plans index is rebuilt only when the workspace plans directory exists:
/// a PRD or research command must not create `plans/` in a workspace that has
/// no plans yet.
pub fn rebuild_all(workdir: &Path) -> Result<()> {
    rebuild_prd_index(workdir)?;
    if plans_dir(workdir).is_dir() {
        rebuild_plans_index(workdir)?;
    }
    rebuild_research_index(workdir)?;
    rebuild_master_index(workdir)?;
    Ok(())
}

// ─── Helpers ───────────────────────────────────────────────────────

fn list_md_sorted(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "md") {
                files.push(path);
            }
        }
    }
    files.sort();
    files
}

fn file_slug(path: &Path) -> String {
    path.file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string()
}

struct FrontmatterBrief {
    title: String,
    created: String,
    crates: String,
    plans_generated: String,
    coverage: f64,
}

fn read_frontmatter(path: &Path) -> FrontmatterBrief {
    let content = std::fs::read_to_string(path).unwrap_or_default();
    let slug = file_slug(path);
    let mut brief = FrontmatterBrief {
        title: slug,
        created: "—".into(),
        crates: "—".into(),
        plans_generated: "—".into(),
        coverage: 0.0,
    };
    for line in content.lines() {
        let line = line.trim();
        if let Some(val) = line.strip_prefix("title:") {
            brief.title = val.trim().trim_matches('"').to_string();
        } else if let Some(val) = line.strip_prefix("created:") {
            brief.created = val.trim().to_string();
        } else if let Some(val) = line.strip_prefix("crates:") {
            brief.crates = val.trim().to_string();
        } else if let Some(val) = line.strip_prefix("plans_generated:") {
            brief.plans_generated = val.trim().to_string();
        } else if let Some(val) = line.strip_prefix("coverage:") {
            brief.coverage = val.trim().parse().unwrap_or(0.0);
        }
    }
    brief
}

fn extract_meta_string(parsed: &toml::Value, key: &str) -> Option<String> {
    extract_meta_value(parsed, key).map(|value| value.trim_matches('"').to_string())
}

fn extract_meta_value(parsed: &toml::Value, key: &str) -> Option<String> {
    let value = parsed
        .get("meta")
        .and_then(toml::Value::as_table)?
        .get(key)?;

    match value {
        toml::Value::String(s) => Some(s.clone()),
        toml::Value::Integer(i) => Some(i.to_string()),
        toml::Value::Float(f) => Some(f.to_string()),
        toml::Value::Boolean(b) => Some(b.to_string()),
        _ => Some(value.to_string()),
    }
}

// ─── Tests ─────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workspace_paths::{drafts_dir, ideas_path, plans_dir, published_dir};

    /// A `tasks.toml` that `roko plan run` can load: `meta` lines, then one
    /// task per status.
    fn runnable_tasks(meta: &str, statuses: &[&str]) -> String {
        let mut toml = format!("[meta]\n{meta}\n");
        for (index, status) in statuses.iter().enumerate() {
            toml.push_str(&format!(
                "\n[[task]]\nid = \"T{n}\"\ntitle = \"Task {n}\"\nstatus = \"{status}\"\n\
                 files = [\"src/lib.rs\"]\nverify = [{{ command = \"true\" }}]\n",
                n = index + 1
            ));
        }
        toml
    }

    #[test]
    fn rebuild_all_empty() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(published_dir(tmp.path())).unwrap();
        std::fs::create_dir_all(drafts_dir(tmp.path())).unwrap();
        std::fs::create_dir_all(tmp.path().join(".roko/research")).unwrap();
        std::fs::write(ideas_path(tmp.path()), "# Ideas\n").unwrap();
        rebuild_all(tmp.path()).unwrap();
        assert!(master_index_path(tmp.path()).exists());
        assert!(prd_index_path(tmp.path()).exists());
        assert!(research_index_path(tmp.path()).exists());
    }

    #[test]
    fn rebuild_all_propagates_index_write_failures() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join(".roko"), b"not a directory").unwrap();

        let error = rebuild_all(tmp.path()).unwrap_err();

        assert!(
            error.to_string().contains("Not a directory")
                || error.to_string().contains("not a directory"),
            "unexpected error: {error:#}"
        );
    }

    #[test]
    fn prd_index_includes_drafts() {
        let tmp = tempfile::tempdir().unwrap();
        let drafts = drafts_dir(tmp.path());
        std::fs::create_dir_all(&drafts).unwrap();
        std::fs::create_dir_all(published_dir(tmp.path())).unwrap();
        std::fs::write(ideas_path(tmp.path()), "# Ideas\n").unwrap();
        std::fs::write(
            drafts.join("test-prd.md"),
            "---\ntitle: Test PRD\nstatus: draft\ncreated: 2026-04-08\n---\n# Test\n",
        )
        .unwrap();
        rebuild_prd_index(tmp.path()).unwrap();
        let content = std::fs::read_to_string(prd_index_path(tmp.path())).unwrap();
        assert!(content.contains("test-prd"));
        assert!(content.contains("Test PRD"));
    }

    #[test]
    fn plans_index_counts_tasks() {
        let tmp = tempfile::tempdir().unwrap();
        let plan = plans_dir(tmp.path()).join("test-plan");
        std::fs::create_dir_all(&plan).unwrap();
        std::fs::write(
            plan.join("tasks.toml"),
            runnable_tasks("plan = \"test\"\nmax_parallel = 2", &["done", "ready"]),
        )
        .unwrap();
        rebuild_plans_index(tmp.path()).unwrap();
        let content = std::fs::read_to_string(plans_index_path(tmp.path())).unwrap();
        assert!(
            content.contains("| `test-plan` | 2 | 1 | 1 | 🔄 in progress | 2 |"),
            "{content}"
        );
    }

    #[test]
    fn plans_index_ignores_nested_acceptance_contract_ids() {
        let tmp = tempfile::tempdir().unwrap();
        let plan = plans_dir(tmp.path()).join("contract-plan");
        std::fs::create_dir_all(&plan).unwrap();
        std::fs::write(
            plan.join("tasks.toml"),
            "[meta]\nplan = \"contract\"\nmax_parallel = 1\n\n\
             [[task]]\nid = \"T1\"\ntitle = \"Task\"\nstatus = \"ready\"\n\
             files = [\"src/lib.rs\"]\nverify = [{ command = \"true\" }]\n\n\
             [task.acceptance_contract]\nversion = 1\n\n\
             [[task.acceptance_contract.gates]]\nid = \"compile\"\nkind = \"compile\"\ncommand = \"cargo check\"\n",
        )
        .unwrap();

        rebuild_plans_index(tmp.path()).unwrap();

        let content = std::fs::read_to_string(plans_index_path(tmp.path())).unwrap();
        assert!(
            content.contains("| `contract-plan` | 1 | 0 | 1 |"),
            "nested gate id/status changed plan counts: {content}"
        );
    }

    #[test]
    fn plans_index_excludes_superseded_from_executable_totals() {
        let tmp = tempfile::tempdir().unwrap();
        let active = plans_dir(tmp.path()).join("active-plan");
        let old = plans_dir(tmp.path()).join("old-plan");
        std::fs::create_dir_all(&active).unwrap();
        std::fs::create_dir_all(&old).unwrap();
        std::fs::write(
            active.join("tasks.toml"),
            runnable_tasks(
                "plan = \"active\"\nstatus = \"ready\"\nmax_parallel = 1",
                &["done", "ready"],
            ),
        )
        .unwrap();
        std::fs::write(
            old.join("tasks.toml"),
            "[meta]\nplan = \"old\"\nstatus = \"superseded\"\n\
             superseded_by = \"active-plan\"\nmax_parallel = 1\n\n\
             [[task]]\nid = \"T1\"\nstatus = \"ready\"\n\n\
             [[task]]\nid = \"T2\"\nstatus = \"ready\"\n",
        )
        .unwrap();

        rebuild_plans_index(tmp.path()).unwrap();

        let content = std::fs::read_to_string(plans_index_path(tmp.path())).unwrap();
        assert!(content.contains("| `active-plan` | 2 | 1 | 1 |"));
        assert!(content.contains("## Superseded / Archived"));
        assert!(content.contains("| `old-plan` | 2 | ⏭ superseded | active-plan |"));
        assert!(content.contains("**Executable Total**: 1 plans, 2 tasks, 1 done"));
        assert!(content.contains("**Excluded**: 1 plans, 2 tasks"));
    }

    #[test]
    fn plans_index_separates_rerunnable_fixtures_from_backlog() {
        let tmp = tempfile::tempdir().unwrap();
        let active = plans_dir(tmp.path()).join("active-plan");
        let fixture = plans_dir(tmp.path()).join("demo-example");
        std::fs::create_dir_all(&active).unwrap();
        std::fs::create_dir_all(&fixture).unwrap();
        std::fs::write(
            active.join("tasks.toml"),
            runnable_tasks(
                "plan = \"active\"\nstatus = \"ready\"\nmax_parallel = 1",
                &["ready"],
            ),
        )
        .unwrap();
        std::fs::write(
            fixture.join("tasks.toml"),
            "[meta]\nplan = \"demo\"\nstatus = \"fixture\"\nmax_parallel = 1\n\n\
             [[task]]\nid = \"D1\"\nstatus = \"ready\"\n\n\
             [[task]]\nid = \"D2\"\nstatus = \"ready\"\n",
        )
        .unwrap();

        rebuild_plans_index(tmp.path()).unwrap();

        let content = std::fs::read_to_string(plans_index_path(tmp.path())).unwrap();
        assert!(content.contains("**Executable Total**: 1 plans, 1 tasks, 0 done"));
        assert!(content.contains("## Fixtures / Examples"));
        assert!(content.contains("| `demo-example` | 2 | 🧪 fixture |"));
        assert!(content.contains("**Fixtures excluded from backlog**: 1 plans, 2 tasks"));
    }

    #[test]
    fn plans_index_lists_plans_inside_plan_sets() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("plans");
        for dir in [
            "solo",
            "programme/01-first",
            "programme/02-second",
            "archive/old",
        ] {
            std::fs::create_dir_all(root.join(dir)).unwrap();
            std::fs::write(
                root.join(dir).join("tasks.toml"),
                runnable_tasks("plan = \"p\"\nmax_parallel = 1", &["done", "ready"]),
            )
            .unwrap();
        }

        let content = render_plans_index(tmp.path()).unwrap();

        for row in [
            "| `programme/01-first` | 2 | 1 | 1 |",
            "| `programme/02-second` | 2 | 1 | 1 |",
            "| `solo` | 2 | 1 | 1 |",
        ] {
            assert!(content.contains(row), "missing {row}:\n{content}");
        }
        assert!(!content.contains("`archive/old`") && !content.contains("`old`"));
        assert!(
            content.contains("**Executable Total**: 3 plans, 6 tasks, 3 done"),
            "{content}"
        );
    }

    #[test]
    fn plans_index_lists_unrunnable_plans_apart_from_the_backlog() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("plans");
        let write = |dir: &str, file: &str, contents: &str| {
            std::fs::create_dir_all(root.join(dir)).unwrap();
            std::fs::write(root.join(dir).join(file), contents).unwrap();
        };
        write(
            "runnable",
            "tasks.toml",
            &runnable_tasks("plan = \"runnable\"", &["ready"]),
        );
        write(
            "tasks-spelling",
            "tasks.toml",
            "[meta]\nplan = \"t\"\n\n[[tasks]]\nid = \"T01\"\ntitle = \"Hello\"\n",
        );
        write(
            "no-meta",
            "tasks.toml",
            "[[tasks]]\nid = \"T01\"\n\n[[tasks]]\nid = \"T02\"\n",
        );
        write(
            "empty",
            "tasks.toml",
            "task = []\n\n[meta]\nplan = \"empty\"\n",
        );
        write("unplanned", "plan.md", "# Only a narrative\n");
        write(
            "schema",
            "tasks.toml",
            "[meta]\nplan = \"s\"\n\n[[task]]\nid = \"T1\"\ntitle = \"No files or verify\"\n",
        );

        let content = render_plans_index(tmp.path()).unwrap();

        assert!(
            content.contains("| `runnable` | 1 | 0 | 1 |")
                && content.contains("**Executable Total**: 1 plans, 1 tasks, 0 done"),
            "{content}"
        );
        for row in [
            "| `empty` | 0 | no tasks |",
            "| `no-meta` | 2 | `[[tasks]]` instead of `[[task]]`; no `[meta]` table |",
            "| `schema` | 1 | 2 schema issue(s); run `roko plan validate` |",
            "| `tasks-spelling` | 1 | `[[tasks]]` instead of `[[task]]` |",
            "| `unplanned` | — | no `tasks.toml` |",
        ] {
            assert!(content.contains(row), "missing {row}:\n{content}");
        }
        assert!(
            content.contains("**Not runnable, excluded from backlog**: 5 plans"),
            "{content}"
        );

        std::fs::create_dir_all(roko_dir(tmp.path())).unwrap();
        rebuild_master_index(tmp.path()).unwrap();
        let master = std::fs::read_to_string(master_index_path(tmp.path())).unwrap();
        assert!(master.contains("## Plans (1 executable,"), "{master}");
    }

    #[test]
    fn plans_index_writes_into_the_workspace_plans_dir() {
        let tmp = tempfile::tempdir().unwrap();

        rebuild_plans_index(tmp.path()).unwrap();

        assert!(tmp.path().join("plans/INDEX.md").is_file());
    }

    #[test]
    fn plans_index_stays_in_a_legacy_plans_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let legacy = tmp.path().join(".roko/plans");
        std::fs::create_dir_all(legacy.join("old-plan")).unwrap();
        std::fs::write(
            legacy.join("old-plan/tasks.toml"),
            "[meta]\nplan = \"old\"\n\n[[task]]\nid = \"T1\"\n",
        )
        .unwrap();

        rebuild_all(tmp.path()).unwrap();

        assert!(legacy.join("INDEX.md").is_file());
        assert!(!tmp.path().join("plans").exists());
        let master = std::fs::read_to_string(master_index_path(tmp.path())).unwrap();
        assert!(master.contains("[Full index](.roko/plans/INDEX.md)"));
    }

    #[test]
    fn rebuild_all_does_not_create_plans_dir_in_a_workspace_without_plans() {
        let tmp = tempfile::tempdir().unwrap();
        // `roko init` leaves an empty `.roko/plans/`.
        std::fs::create_dir_all(tmp.path().join(".roko/plans")).unwrap();

        rebuild_all(tmp.path()).unwrap();

        assert!(!tmp.path().join("plans").exists());
        assert!(!tmp.path().join(".roko/plans/INDEX.md").exists());
        let master = std::fs::read_to_string(master_index_path(tmp.path())).unwrap();
        assert!(master.contains("## Plans (0 executable"));
        assert!(master.contains("[Full index](plans/INDEX.md)"));
    }

    #[test]
    fn plans_index_render_is_deterministic_and_non_mutating() {
        let tmp = tempfile::tempdir().unwrap();
        let plan = plans_dir(tmp.path()).join("deterministic-plan");
        std::fs::create_dir_all(&plan).unwrap();
        std::fs::write(
            plan.join("tasks.toml"),
            runnable_tasks(
                "plan = \"deterministic\"\nstatus = \"ready\"\nmax_parallel = 1",
                &["ready"],
            ),
        )
        .unwrap();

        let first = render_plans_index(tmp.path()).unwrap();
        let second = render_plans_index(tmp.path()).unwrap();

        assert_eq!(first, second);
        assert!(first.contains("| `deterministic-plan` | 1 | 0 | 1 |"));
        assert!(!plans_index_path(tmp.path()).exists());
    }

    #[test]
    fn plans_index_render_fails_closed_on_unreadable_plans_directory_shape() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(roko_dir(tmp.path())).unwrap();
        std::fs::write(plans_dir(tmp.path()), b"not a directory").unwrap();

        let error = render_plans_index(tmp.path()).unwrap_err();

        assert!(error.to_string().contains("read plans directory"));
    }

    #[test]
    fn plans_index_render_fails_closed_on_malformed_tasks_toml() {
        let tmp = tempfile::tempdir().unwrap();
        let plan = plans_dir(tmp.path()).join("malformed-plan");
        std::fs::create_dir_all(&plan).unwrap();
        std::fs::write(plan.join("tasks.toml"), b"[[task]\nid = \"T1\"\n").unwrap();

        let error = render_plans_index(tmp.path()).unwrap_err();

        assert!(error.to_string().contains("parse plan tasks input"));
    }

    #[test]
    fn plans_index_render_fails_closed_on_malformed_run_state() {
        let tmp = tempfile::tempdir().unwrap();
        let plan = plans_dir(tmp.path()).join("valid-plan");
        std::fs::create_dir_all(&plan).unwrap();
        std::fs::write(
            plan.join("tasks.toml"),
            "[meta]\nstatus = \"ready\"\n\n[[task]]\nid = \"T1\"\n",
        )
        .unwrap();
        let state = tmp.path().join(".roko/state");
        std::fs::create_dir_all(&state).unwrap();
        std::fs::write(state.join("run-state.json"), b"{not-json").unwrap();

        let error = render_plans_index(tmp.path()).unwrap_err();

        assert!(error.to_string().contains("parse run-state input"));
    }

    #[test]
    fn plans_index_check_detects_stale_content_without_rewriting() {
        let tmp = tempfile::tempdir().unwrap();
        let plan = plans_dir(tmp.path()).join("stale-plan");
        std::fs::create_dir_all(&plan).unwrap();
        std::fs::write(
            plan.join("tasks.toml"),
            "[meta]\nplan = \"stale\"\nstatus = \"ready\"\nmax_parallel = 1\n\n\
             [[task]]\nid = \"T1\"\nstatus = \"ready\"\n",
        )
        .unwrap();
        rebuild_plans_index(tmp.path()).unwrap();
        let index_path = plans_index_path(tmp.path());
        std::fs::write(&index_path, b"stale bytes\n").unwrap();
        let before = std::fs::read(&index_path).unwrap();

        let error = check_plans_index(tmp.path()).unwrap_err();

        assert!(error.to_string().contains("generated plans index is stale"));
        assert_eq!(std::fs::read(&index_path).unwrap(), before);
        rebuild_plans_index(tmp.path()).unwrap();
        check_plans_index(tmp.path()).unwrap();
    }

    #[test]
    fn plans_index_check_does_not_create_a_missing_index() {
        let tmp = tempfile::tempdir().unwrap();

        let error = check_plans_index(tmp.path()).unwrap_err();

        assert!(error.to_string().contains("read generated plans index"));
        assert!(!plans_index_path(tmp.path()).exists());
    }

    #[test]
    fn master_index_has_all_sections() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(published_dir(tmp.path())).unwrap();
        std::fs::create_dir_all(drafts_dir(tmp.path())).unwrap();
        std::fs::create_dir_all(tmp.path().join(".roko/research")).unwrap();
        std::fs::write(ideas_path(tmp.path()), "# Ideas\n").unwrap();
        rebuild_all(tmp.path()).unwrap();
        let content = std::fs::read_to_string(master_index_path(tmp.path())).unwrap();
        assert!(content.contains("## PRDs"));
        assert!(content.contains("## Plans"));
        assert!(content.contains("## Research"));
        assert!(content.contains("## Episodes"));
        assert!(content.contains("## Config"));
    }

    #[test]
    fn append_master_index_prompt_skips_empty_index() {
        let tmp = tempfile::tempdir().unwrap();
        let mut prompt = String::from("prefix\n");

        append_master_index_prompt(&mut prompt, tmp.path(), "## Existing");

        assert_eq!(prompt, "prefix\n");
    }

    #[test]
    fn append_master_index_prompt_includes_heading_and_separator() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join(".roko")).unwrap();
        std::fs::write(master_index_path(tmp.path()), "# Master\n").unwrap();

        let mut prompt = String::new();
        append_master_index_prompt(&mut prompt, tmp.path(), "## Existing");

        assert!(prompt.contains("## Existing"));
        assert!(prompt.contains("# Master"));
        assert!(prompt.ends_with("---\n\n"));
    }
}
