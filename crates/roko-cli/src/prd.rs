//! `roko prd` subcommand — PRD lifecycle management.
//!
//! Manages product requirements documents through their lifecycle:
//! idea → draft → published → plans → implemented.
//!
//! PRDs live in `.roko/prd/` with this layout:
//! ```text
//! .roko/prd/
//! ├── ideas.md              # quick captures
//! ├── drafts/               # work-in-progress PRDs
//! │   └── <slug>.md
//! └── published/            # finalized PRDs
//!     └── <slug>.md
//! ```

mod accept_blocks;
mod dry_run_fs;

use std::collections::{HashMap, HashSet};
use std::fmt::Write as _;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::time::Instant;

use crate::agent_config::command_from_config;
use crate::agent_exec::{
    AgentCrashClass, AgentExecOpts, classify_agent_crash, persist_capture_episode,
    run_agent_capture_silent_with_usage,
};
use crate::model_selection::resolve_planner_model;
use crate::plan_authoring::AuthoringSpend;
use crate::runner::tui_bridge::TuiBridge;
use crate::task_parser::{META_KEYS, TASK_KEYS, TasksFile, VERIFY_KEYS, suggest_field_correction};
use crate::workspace_paths::{
    drafts_dir, ideas_path, plans_dir as workspace_plans_dir, prd_dir, published_dir,
};
use anyhow::{Context as _, Result, anyhow};
use indexmap::IndexMap;
use roko_core::config::routing::LadderConfig;
use roko_core::config::schema::{ModelProfile, RokoConfig};
use roko_core::io::atomic_write_str;
use roko_core::{Body, Kind, Provenance, Signal, Store};
use roko_fs::FileSubstrate;
use roko_learn::episode_logger::{Episode, EpisodeLogger};
pub use roko_learn::runtime_feedback::{ArtifactValidationReport, GenerationOutcome};
use roko_runtime::event_bus::{PublishOrigin, RokoEvent, global_event_bus};

/// Typed artifact result projected from the current PRD/plan generation outcome.
#[derive(Debug, Clone, PartialEq)]
pub enum ArtifactOutcome {
    Valid {
        artifact_type: String,
        path: PathBuf,
        report: ArtifactValidationReport,
    },
    Invalid {
        artifact_type: String,
        path: Option<PathBuf>,
        report: Option<ArtifactValidationReport>,
    },
    NotProduced {
        artifact_type: String,
        reason: String,
    },
    ValidationUnavailable {
        artifact_type: String,
        path: Option<PathBuf>,
        reason: String,
    },
}

impl ArtifactOutcome {
    /// Adapt the legacy `GenerationOutcome` booleans without changing generation behavior.
    #[must_use]
    pub fn from_generation_outcome(
        artifact_type: impl Into<String>,
        path: Option<PathBuf>,
        outcome: &GenerationOutcome,
    ) -> Self {
        let artifact_type = artifact_type.into();
        if !outcome.process_success {
            return Self::NotProduced {
                artifact_type,
                reason: "generation process failed".to_string(),
            };
        }

        if !outcome.artifact_valid {
            return Self::Invalid {
                artifact_type,
                path,
                report: outcome.validation_report.clone(),
            };
        }

        let Some(path) = path else {
            return Self::NotProduced {
                artifact_type,
                reason: "generation process succeeded but no artifact path was provided"
                    .to_string(),
            };
        };

        match &outcome.validation_report {
            Some(report) => Self::Valid {
                artifact_type,
                path,
                report: report.clone(),
            },
            None => Self::ValidationUnavailable {
                artifact_type,
                path: Some(path),
                reason: "artifact validation report was not available".to_string(),
            },
        }
    }

    #[must_use]
    pub fn is_valid(&self) -> bool {
        matches!(self, Self::Valid { .. })
    }
}

fn generated_plan_stats(paths: &[PathBuf]) -> Result<(usize, String)> {
    if paths.is_empty() {
        return Ok((0, "unknown".to_string()));
    }

    let mut task_count = 0usize;
    let mut max_tier = roko_core::task::TaskTier::Mechanical;

    for path in paths {
        let tasks_file =
            TasksFile::parse(path).with_context(|| format!("parse {}", path.display()))?;
        task_count = task_count.saturating_add(tasks_file.tasks.len());
        for task in &tasks_file.tasks {
            max_tier = max_tier.max(task.tier_class());
        }
    }

    let estimated_complexity = if task_count == 0 {
        "unknown".to_string()
    } else {
        max_tier.label().to_string()
    };

    Ok((task_count, estimated_complexity))
}

fn normalize_task_title(title: &str) -> String {
    title
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { ' ' })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

fn preserve_completed_task_status(
    old_tasks: Option<&TasksFile>,
    mut regenerated: TasksFile,
    plan_dir: &Path,
) -> TasksFile {
    use crate::task_parser::PlanMutation;

    if let Some(old_tasks) = old_tasks {
        let completed: Vec<&crate::task_parser::TaskDef> = old_tasks
            .tasks
            .iter()
            .filter(|task| task.status.eq_ignore_ascii_case("done"))
            .collect();

        let mutations: Vec<PlanMutation> = regenerated
            .tasks
            .iter()
            .filter_map(|task| {
                let normalized = normalize_task_title(&task.title);
                let already_done = completed.iter().any(|old| {
                    let old_title = normalize_task_title(&old.title);
                    old.id == task.id
                        || old_title == normalized
                        || old_title.contains(&normalized)
                        || normalized.contains(&old_title)
                });
                if already_done {
                    Some(PlanMutation::MarkTaskDone {
                        task_id: task.id.clone(),
                    })
                } else {
                    None
                }
            })
            .collect();

        regenerated.apply_mutations(mutations);

        regenerated.meta.iteration = old_tasks.meta.iteration.saturating_add(1);
        if regenerated.meta.plan.trim().is_empty() {
            regenerated.meta.plan = old_tasks.meta.plan.clone();
        }
    }

    if regenerated.meta.plan.trim().is_empty() {
        regenerated.meta.plan = plan_dir
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| "unknown-plan".to_string());
    }

    // apply_mutations already calls recount_meta; call it once more to handle
    // the case where old_tasks was None (no mutations were applied).
    regenerated.recount_meta();

    regenerated
}

fn find_plan_source_document(plan_dir: &Path) -> Result<PathBuf> {
    for candidate in ["source-prd.md", "prd-extract.md", "plan.md"] {
        let path = plan_dir.join(candidate);
        if path.exists() {
            return Ok(path);
        }
    }

    Err(anyhow!(
        "no source PRD found in {} (looked for source-prd.md, prd-extract.md, and plan.md)",
        plan_dir.display()
    ))
}

fn old_format_plan_dirs(root: &Path) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Ok(entries) = std::fs::read_dir(root) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let tasks_path = path.join("tasks.toml");
            if !tasks_path.is_file() {
                continue;
            }
            if matches!(
                TasksFile::validate_modern_fields(&tasks_path),
                Ok(issues) if !issues.is_empty()
            ) {
                dirs.push(path);
            }
        }
    }
    dirs.sort();
    dirs
}

/// Regenerate the plan in `plan_dir` through the plan generator when its
/// tasks.toml lacks modern fields. Returns whether it did.
async fn regenerate_old_format_plan(
    workdir: &Path,
    model: Option<&str>,
    plan_dir: &Path,
) -> Result<bool> {
    let tasks_path = plan_dir.join("tasks.toml");
    if !tasks_path.is_file() {
        return Ok(false);
    }

    let modern_issues = TasksFile::validate_modern_fields(&tasks_path)
        .with_context(|| format!("validate modern fields at {}", tasks_path.display()))?;
    if modern_issues.is_empty() {
        return Ok(false);
    }

    let slug = plan_dir_slug(plan_dir);
    generate_plan(PlanRequest {
        model,
        ..PlanRequest::new(PlanSource::Regenerate(plan_dir), &slug, workdir)
    })
    .await?;
    Ok(true)
}

/// Regenerate, one planner call each, the plans in `workdir`'s plans
/// directory whose tasks.toml lacks modern fields. Opt-in
/// (`roko prd plan --regenerate-old`): no generate path runs it on its own.
/// Returns how many plans it regenerated.
pub async fn regenerate_old_format_plans(workdir: &Path, model: Option<&str>) -> Result<usize> {
    let mut regen_count = 0usize;
    for plan_dir in old_format_plan_dirs(&workspace_plans_dir(workdir)) {
        if regenerate_old_format_plan(workdir, model, &plan_dir).await? {
            regen_count += 1;
        }
    }
    Ok(regen_count)
}

async fn emit_prd_plan_signal(workdir: &Path, kind: Kind, body: serde_json::Value) -> Result<()> {
    let substrate = FileSubstrate::open(workdir.join(".roko"))
        .await
        .with_context(|| format!("open {}", workdir.join(".roko").display()))?;
    let signal = Signal::builder(kind)
        .body(Body::Json(body))
        .provenance(Provenance::trusted("roko.prd"))
        .build();
    substrate.put(signal).await?;
    Ok(())
}

async fn append_prd_published_episode(
    workdir: &Path,
    slug: &str,
    path: &Path,
    published_at: chrono::DateTime<chrono::Utc>,
    origin: PublishOrigin,
) -> Result<()> {
    let logger = EpisodeLogger::new(workdir.join(".roko").join("episodes.jsonl"));
    let mut episode = Episode::new("roko-cli", slug);
    episode.kind = "prd_published".to_string();
    episode.agent_template = "cli".to_string();
    episode.trigger_kind = "prd_publish".to_string();
    episode.timestamp = published_at;
    episode.started_at = published_at;
    episode.completed_at = published_at;
    episode.success = true;
    episode
        .extra
        .insert("slug".to_string(), serde_json::json!(slug));
    episode.extra.insert(
        "path".to_string(),
        serde_json::json!(path.display().to_string()),
    );
    episode.extra.insert(
        "origin".to_string(),
        serde_json::to_value(origin).unwrap_or(serde_json::Value::Null),
    );
    episode.extra.insert(
        "published_at".to_string(),
        serde_json::json!(published_at.to_rfc3339()),
    );
    logger.append(&episode).await?;
    Ok(())
}

/// Ensure the PRD directory structure exists.
pub fn ensure_dirs(workdir: &Path) -> Result<()> {
    std::fs::create_dir_all(drafts_dir(workdir))?;
    std::fs::create_dir_all(published_dir(workdir))?;
    let ideas = ideas_path(workdir);
    if !ideas.exists() {
        std::fs::write(
            &ideas,
            "# Ideas\n\nQuick captures. Run `roko prd idea \"text\"` to append.\n",
        )?;
    }
    Ok(())
}

// ─── PRD frontmatter ───────────────────────────────────────────────

/// Parsed PRD frontmatter.
#[derive(Debug, Default)]
pub struct PrdMeta {
    /// Stable PRD identifier (e.g. `prd-agent-memory`).
    pub id: String,
    /// Human-readable PRD title.
    pub title: String,
    /// Lifecycle status (`draft` or `published`).
    pub status: String,
    /// Monotonic document version number.
    pub version: u32,
    /// Creation date in `YYYY-MM-DD` format.
    pub created: String,
    /// Last update date in `YYYY-MM-DD` format.
    pub updated: String,
    /// Other PRD ids this document depends on.
    pub depends_on: Vec<String>,
    /// Crates touched by the requirements in this PRD.
    pub crates: Vec<String>,
    /// Plan ids generated from this PRD.
    pub plans_generated: Vec<String>,
    /// Coverage ratio in `[0.0, 1.0]`.
    pub coverage: f64,
    /// Free-form metadata tags.
    pub tags: Vec<String>,
    /// Optional plan generation template preset.
    pub plan_template: Option<String>,
}

impl PrdMeta {
    /// Parse frontmatter from a PRD markdown file.
    ///
    /// Extracts the YAML block between `---` markers and parses it with
    /// `serde_yaml_ng` so that values containing colons, quoted strings,
    /// and YAML list syntax are handled correctly.
    pub fn parse(content: &str) -> Option<Self> {
        let content = content.trim();
        if !content.starts_with("---") {
            return None;
        }
        let end = content[3..].find("---")?;
        let yaml_str = &content[3..3 + end];

        // Also support legacy `plan_template = "strict"` TOML-ish syntax by
        // normalizing it to valid YAML before parsing.
        let normalized: String = yaml_str
            .lines()
            .map(|line| {
                if let Some(rest) = line.strip_prefix("plan_template =") {
                    format!("plan_template: {rest}")
                } else if let Some(rest) = line.strip_prefix("plan_template=") {
                    format!("plan_template: {rest}")
                } else {
                    line.to_string()
                }
            })
            .collect::<Vec<_>>()
            .join("\n");

        let mapping: serde_yaml_ng::Mapping =
            serde_yaml_ng::from_str(&normalized).unwrap_or_default();

        if mapping.is_empty() {
            // Parsing produced nothing useful — treat as no frontmatter.
            return None;
        }

        let mut meta = Self::default();
        meta.id = yaml_get_string(&mapping, "id").unwrap_or_default();
        meta.title = yaml_get_string(&mapping, "title").unwrap_or_default();
        meta.status = yaml_get_string(&mapping, "status").unwrap_or_default();
        meta.version = yaml_get_string(&mapping, "version")
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or(1);
        meta.created = yaml_get_string(&mapping, "created").unwrap_or_default();
        meta.updated = yaml_get_string(&mapping, "updated").unwrap_or_default();
        meta.coverage = yaml_get_string(&mapping, "coverage")
            .and_then(|v| v.parse::<f64>().ok())
            .unwrap_or(0.0);
        meta.depends_on = yaml_get_string_list(&mapping, "depends_on");
        meta.crates = yaml_get_string_list(&mapping, "crates");
        meta.plans_generated = yaml_get_string_list(&mapping, "plans_generated");
        meta.tags = yaml_get_string_list(&mapping, "tags");
        meta.plan_template = yaml_get_string(&mapping, "plan_template")
            .map(|v| v.trim_matches('"').trim_matches('\'').to_string())
            .filter(|v| !v.is_empty());
        Some(meta)
    }
}

/// Extract a scalar value from a YAML mapping as a `String`.
fn yaml_get_string(mapping: &serde_yaml_ng::Mapping, key: &str) -> Option<String> {
    let value = mapping.get(serde_yaml_ng::Value::String(key.to_string()))?;
    match value {
        serde_yaml_ng::Value::String(s) => Some(s.clone()),
        serde_yaml_ng::Value::Number(n) => Some(n.to_string()),
        serde_yaml_ng::Value::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}

/// Extract a list of strings from a YAML mapping.
///
/// Supports both inline `[a, b]` and block list syntax.
fn yaml_get_string_list(mapping: &serde_yaml_ng::Mapping, key: &str) -> Vec<String> {
    let Some(value) = mapping.get(serde_yaml_ng::Value::String(key.to_string())) else {
        return Vec::new();
    };
    match value {
        serde_yaml_ng::Value::Sequence(seq) => seq
            .iter()
            .filter_map(|v| match v {
                serde_yaml_ng::Value::String(s) => Some(s.clone()),
                serde_yaml_ng::Value::Number(n) => Some(n.to_string()),
                _ => None,
            })
            .collect(),
        serde_yaml_ng::Value::String(s) => {
            // Single scalar value — wrap in a vec.
            if s.is_empty() {
                Vec::new()
            } else {
                vec![s.clone()]
            }
        }
        _ => Vec::new(),
    }
}

// ─── List PRDs ─────────────────────────────────────────────────────

/// Return sorted markdown files (`*.md`) in `dir`.
///
/// Missing or unreadable directories are treated as empty.
pub fn list_md_files(dir: &Path) -> Vec<PathBuf> {
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

/// Entry in the PRD listing.
pub struct PrdEntry {
    /// File slug (`<slug>.md` without extension).
    pub slug: String,
    /// Display title shown in CLI output.
    pub title: String,
    /// Lifecycle status for this entry.
    pub status: String,
    /// Coverage ratio in `[0.0, 1.0]`.
    pub coverage: f64,
}

fn read_prd_entry(path: &Path) -> PrdEntry {
    let slug = path
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    let content = std::fs::read_to_string(path).unwrap_or_default();
    if let Some(meta) = PrdMeta::parse(&content) {
        PrdEntry {
            slug,
            title: meta.title,
            status: meta.status,
            coverage: meta.coverage,
        }
    } else {
        let path_str = path.to_string_lossy();
        let status = if path_str.contains("/published/") {
            "published"
        } else if path_str.contains("/drafts/") {
            "draft"
        } else {
            "unknown"
        };
        PrdEntry {
            slug: slug.clone(),
            title: slug,
            status: status.into(),
            coverage: 0.0,
        }
    }
}

// ─── Public command handlers ───────────────────────────────────────

/// `roko prd idea "text"` — append to ideas.md.
pub fn cmd_idea(workdir: &Path, text: &str, json: bool) -> Result<()> {
    ensure_dirs(workdir)?;
    let path = ideas_path(workdir);
    let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M");
    let entry = format!("- {timestamp} — {text}\n");
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .with_context(|| format!("open {}", path.display()))?;
    std::io::Write::write_all(&mut file, entry.as_bytes())?;
    if json {
        let payload = serde_json::json!({
            "status": "captured",
            "text": text,
            "timestamp": timestamp.to_string(),
            "path": path.display().to_string(),
        });
        println!("{}", serde_json::to_string_pretty(&payload)?);
    } else {
        println!("💡 Captured: {text}");
    }
    Ok(())
}

/// `roko prd list` — list all PRDs, drafts, and ideas.
pub fn cmd_list(workdir: &Path, json: bool) -> Result<()> {
    ensure_dirs(workdir)?;

    let published = list_md_files(&published_dir(workdir));
    let drafts = list_md_files(&drafts_dir(workdir));
    let ideas_file = ideas_path(workdir);
    let ideas_content = std::fs::read_to_string(&ideas_file).unwrap_or_default();
    let ideas_lines: Vec<&str> = ideas_content
        .lines()
        .filter(|l| l.starts_with("- "))
        .collect();

    if json {
        #[derive(serde::Serialize)]
        struct PrdRow {
            slug: String,
            title: String,
            status: String,
            coverage: f64,
        }
        let published_rows: Vec<PrdRow> = published
            .iter()
            .map(|p| {
                let e = read_prd_entry(p);
                PrdRow {
                    slug: e.slug,
                    title: e.title,
                    status: e.status,
                    coverage: e.coverage,
                }
            })
            .collect();
        let draft_rows: Vec<PrdRow> = drafts
            .iter()
            .map(|p| {
                let e = read_prd_entry(p);
                PrdRow {
                    slug: e.slug,
                    title: e.title,
                    status: e.status,
                    coverage: e.coverage,
                }
            })
            .collect();
        let payload = serde_json::json!({
            "published": published_rows,
            "drafts": draft_rows,
            "ideas": ideas_lines,
        });
        println!("{}", serde_json::to_string_pretty(&payload)?);
        return Ok(());
    }

    println!("═══ Published PRDs ═══");
    if published.is_empty() {
        println!("  (none)");
    } else {
        for path in &published {
            let entry = read_prd_entry(path);
            let cov = if entry.coverage > 0.0 {
                format!("coverage: {:.0}%", entry.coverage * 100.0)
            } else {
                String::new()
            };
            println!("  {:<30} slug: {:<25} {cov}", entry.title, entry.slug);
        }
    }

    println!();
    println!("═══ Drafts ═══");
    if drafts.is_empty() {
        println!("  (none)");
    } else {
        for path in &drafts {
            let entry = read_prd_entry(path);
            println!("  {:<30} slug: {}", entry.title, entry.slug);
        }
    }

    println!();
    let idea_count = ideas_lines.len();
    println!("═══ Ideas ({idea_count} captured) ═══");
    // Show last 5 ideas
    let start = ideas_lines.len().saturating_sub(5);
    for line in &ideas_lines[start..] {
        println!("  {line}");
    }
    if ideas_lines.is_empty() {
        println!("  (none)");
    }

    // Show actionable hints for ACP / CLI users.
    let has_drafts = !drafts.is_empty();
    let has_published = !published.is_empty();
    let has_ideas = idea_count > 0;
    if has_drafts || has_published || has_ideas {
        println!();
        println!("═══ Actions ═══");
        if has_ideas {
            println!("  /prd-draft <slug>         Draft a PRD from an idea");
        }
        if has_drafts {
            let first_draft = read_prd_entry(&drafts[0]);
            println!(
                "  /enhance-prd {:<12} Research & enrich a draft",
                first_draft.slug
            );
            println!(
                "  /prd-plan {:<15} Generate implementation plan from draft",
                first_draft.slug
            );
        }
        if has_published {
            let first_pub = read_prd_entry(&published[0]);
            println!(
                "  /prd-plan {:<15} Generate implementation plan from PRD",
                first_pub.slug
            );
        }
        println!("  /prd-idea \"<text>\"        Capture a new idea");
    }

    Ok(())
}

/// `roko prd status` — coverage report.
pub fn cmd_status(workdir: &Path, plans_dir: Option<&Path>, json: bool) -> Result<()> {
    ensure_dirs(workdir)?;

    let all_prds: Vec<PathBuf> = list_md_files(&published_dir(workdir))
        .into_iter()
        .chain(list_md_files(&drafts_dir(workdir)))
        .collect();

    // Build per-plan stats: (task_count, done_count, source_prd).
    // Keys are the plan directory name; source_prd is the optional explicit link.
    struct PlanStats {
        tasks: u32,
        done: u32,
        source_prd: Option<String>,
    }
    let plans_root = plans_dir.map_or_else(|| workspace_plans_dir(workdir), Path::to_path_buf);
    let mut plan_stats: HashMap<String, PlanStats> = HashMap::new();
    if plans_root.is_dir() {
        if let Ok(entries) = std::fs::read_dir(&plans_root) {
            for entry in entries.flatten() {
                let toml_path = entry.path().join("tasks.toml");
                if !toml_path.exists() {
                    continue;
                }
                let dir_name = entry.file_name().to_string_lossy().to_string();
                let content = std::fs::read_to_string(&toml_path).unwrap_or_default();
                let task_count = usize_to_u32_saturating(content.matches("status = ").count());
                let done_count =
                    usize_to_u32_saturating(content.matches("status = \"done\"").count());
                // Try to parse source_prd from the [meta] section.
                let source_prd = TasksFile::parse(&toml_path)
                    .ok()
                    .and_then(|f| f.meta.source_prd);
                plan_stats.insert(
                    dir_name,
                    PlanStats {
                        tasks: task_count,
                        done: done_count,
                        source_prd,
                    },
                );
            }
        }
    }

    // Build a reverse lookup: prd_slug -> Vec<plan_dir_name>.
    // A plan links to a PRD if (a) its directory name matches, or
    // (b) its source_prd field matches.
    let prd_slugs: Vec<String> = all_prds
        .iter()
        .filter_map(|p| p.file_stem().map(|s| s.to_string_lossy().to_string()))
        .collect();
    let mut linked_plans: HashMap<String, Vec<String>> = HashMap::new();
    let mut matched_plan_names: std::collections::HashSet<String> =
        std::collections::HashSet::new();
    for (plan_name, stats) in &plan_stats {
        // Direct name match.
        if prd_slugs.contains(plan_name) {
            linked_plans
                .entry(plan_name.clone())
                .or_default()
                .push(plan_name.clone());
            matched_plan_names.insert(plan_name.clone());
        }
        // source_prd match (secondary lookup).
        if let Some(ref src) = stats.source_prd {
            if prd_slugs.contains(src) && !matched_plan_names.contains(plan_name) {
                linked_plans
                    .entry(src.clone())
                    .or_default()
                    .push(plan_name.clone());
                matched_plan_names.insert(plan_name.clone());
            }
        }
    }

    let mut total_plans = 0u32;
    let mut total_tasks = 0u32;
    let mut total_done = 0u32;
    for stats in plan_stats.values() {
        total_plans += 1;
        total_tasks = total_tasks.saturating_add(stats.tasks);
        total_done = total_done.saturating_add(stats.done);
    }

    // Unlinked plans: plans that have no matching PRD.
    let mut unlinked: Vec<&String> = plan_stats
        .keys()
        .filter(|name| !matched_plan_names.contains(*name))
        .collect();

    let coverage_pct = if total_tasks > 0 {
        f64::from(total_done) / f64::from(total_tasks) * 100.0
    } else {
        0.0
    };

    if json {
        #[derive(serde::Serialize)]
        struct PrdStatusRow {
            slug: String,
            title: String,
            status: String,
            coverage: f64,
            linked_plans: Vec<String>,
            tasks: u32,
            done: u32,
        }
        let rows: Vec<PrdStatusRow> = all_prds
            .iter()
            .map(|path| {
                let entry = read_prd_entry(path);
                let plans = linked_plans.get(&entry.slug).cloned().unwrap_or_default();
                let (slug_tasks, slug_done) = plans.iter().fold((0u32, 0u32), |(t, d), pn| {
                    if let Some(s) = plan_stats.get(pn) {
                        (t.saturating_add(s.tasks), d.saturating_add(s.done))
                    } else {
                        (t, d)
                    }
                });
                PrdStatusRow {
                    slug: entry.slug,
                    title: entry.title,
                    status: entry.status,
                    coverage: entry.coverage,
                    linked_plans: plans,
                    tasks: slug_tasks,
                    done: slug_done,
                }
            })
            .collect();
        unlinked.sort();
        let payload = serde_json::json!({
            "prds": rows,
            "total_plans": total_plans,
            "total_tasks": total_tasks,
            "total_done": total_done,
            "coverage_pct": coverage_pct,
            "linked_plan_count": matched_plan_names.len(),
            "unlinked_plans": unlinked,
        });
        println!("{}", serde_json::to_string_pretty(&payload)?);
        return Ok(());
    }

    println!("═══ PRD Coverage Report ═══");
    println!();
    println!(
        "{:<35} {:<12} {:<6} {:<6} {:<8}",
        "PRD", "Status", "Plans", "Tasks", "Done"
    );
    println!(
        "{:<35} {:<12} {:<6} {:<6} {:<8}",
        "───", "──────", "─────", "─────", "────"
    );

    for path in &all_prds {
        let entry = read_prd_entry(path);
        if let Some(plans) = linked_plans.get(&entry.slug) {
            let mut slug_tasks = 0u32;
            let mut slug_done = 0u32;
            for pn in plans {
                if let Some(s) = plan_stats.get(pn) {
                    slug_tasks = slug_tasks.saturating_add(s.tasks);
                    slug_done = slug_done.saturating_add(s.done);
                }
            }
            println!(
                "{:<35} {:<12} {:<6} {:<6} {:<8}",
                entry.slug,
                entry.status,
                plans.len(),
                slug_tasks,
                slug_done,
            );
        } else {
            println!(
                "{:<35} {:<12} {:<6} {:<6} {:<8}",
                entry.slug, entry.status, 0, "—", "—"
            );
        }
    }

    if all_prds.is_empty() {
        println!("  (no PRDs yet — run `roko prd draft new \"title\"`)");
    }

    println!();
    println!(
        "Linked: {}  Unlinked: {}",
        matched_plan_names.len(),
        unlinked.len()
    );
    if !unlinked.is_empty() {
        unlinked.sort();
        for name in &unlinked {
            println!("  Unlinked plan: {name}");
        }
    }

    println!();
    println!(
        "Plans: {total_plans}  Tasks: {total_tasks}  Done: {total_done}  \
         Coverage: {coverage_pct:.0}%",
    );

    Ok(())
}

/// `roko prd draft promote <slug>` — move draft to published.
pub async fn cmd_promote(workdir: &Path, slug: &str, auto_execute: bool) -> Result<()> {
    ensure_dirs(workdir)?;
    let src = drafts_dir(workdir).join(format!("{slug}.md"));
    if !src.exists() {
        return Err(anyhow!("draft not found: {}", src.display()));
    }
    let dst = published_dir(workdir).join(format!("{slug}.md"));
    // §14.2: Refuse to silently overwrite an existing published PRD.
    if dst.exists() {
        return Err(anyhow!(
            "published PRD already exists at {}; remove or rename it first",
            dst.display()
        ));
    }

    let mut content = std::fs::read_to_string(&src)?;
    if !has_substantive_markdown_content(&content) {
        return Err(anyhow!(
            "draft has no substantive content; cannot promote. \
             Re-run `roko prd draft edit {slug}` to populate it first."
        ));
    }
    // §14.4: Only replace status within YAML frontmatter, not in body text.
    content = replace_in_frontmatter(&content, "status: draft", "status: published");
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    // Update the 'updated' field if present
    if content.contains("updated:") {
        let re_updated =
            regex::Regex::new(r"updated: .*").context("compile updated-field regex")?;
        content = re_updated
            .replace(&content, format!("updated: {today}"))
            .to_string();
    }
    atomic_write_str(&dst, &content)?;
    std::fs::remove_file(&src)?;
    println!("✅ Promoted: {}", dst.display());
    let published_at = chrono::Utc::now();
    if let Err(err) =
        append_prd_published_episode(workdir, slug, &dst, published_at, PublishOrigin::Cli).await
    {
        tracing::warn!(error = %err, "failed to append PRD publish audit event");
    }
    global_event_bus().emit(RokoEvent::PrdPublished {
        slug: slug.to_string(),
        path: dst.clone(),
        published_at,
        origin: PublishOrigin::Cli,
    });
    let _ = maybe_generate_plan_after_promote(workdir, slug, &dst, auto_execute).await?;
    Ok(())
}

async fn maybe_generate_plan_after_promote(
    workdir: &Path,
    slug: &str,
    prd_path: &Path,
    auto_execute: bool,
) -> Result<Option<PathBuf>> {
    maybe_generate_plan_after_promote_with(
        workdir,
        slug.to_string(),
        prd_path.to_path_buf(),
        auto_execute,
        |slug, path, dry_run| async move {
            generate_plan_from_prd_with_outcome(&slug, &path, dry_run, None, None, None).await
        },
    )
    .await
}

async fn maybe_generate_plan_after_promote_with<F, Fut>(
    workdir: &Path,
    slug: String,
    prd_path: PathBuf,
    auto_execute: bool,
    generator: F,
) -> Result<Option<PathBuf>>
where
    F: FnOnce(String, PathBuf, bool) -> Fut,
    Fut: Future<Output = Result<(PathBuf, GenerationOutcome)>>,
{
    if !auto_plan_enabled(workdir)? {
        return Ok(None);
    }

    let prd_path_display = prd_path.display().to_string();
    match generator(slug, prd_path.clone(), false).await {
        Ok((plans_root, outcome)) => {
            if outcome.fully_successful() {
                println!("Plan generated: {}", plans_root.display());
                if auto_execute {
                    run_generated_plans(workdir, &plans_root).await?;
                }
            } else if outcome.process_success {
                eprintln!(
                    "warning: plan generation completed but artifact validation failed ({})",
                    outcome.status_label()
                );
            } else {
                eprintln!(
                    "warning: plan generation reported {} for {}",
                    outcome.status_label(),
                    prd_path_display
                );
            }
            if auto_execute && !outcome.fully_successful() {
                eprintln!(
                    "warning: skipping auto-execute because generated artifact was not fully successful"
                );
            }
            Ok(Some(plans_root))
        }
        Err(err) => {
            // §14.6: Surface actionable feedback — the PRD was promoted
            // but no plan was generated. Tell the user how to recover.
            eprintln!("error: auto plan generation failed for '{prd_path_display}': {err:#}");
            eprintln!(
                "  → PRD is promoted. Run `roko prd plan <slug>` manually to generate a plan."
            );
            Ok(None)
        }
    }
}

async fn run_generated_plans(workdir: &Path, plans_root: &Path) -> Result<()> {
    // Library path (CLI promote and the serve PRD subscriber): no TUI and no
    // signal handlers of its own.
    let exit_code =
        crate::graph_execution::run_graph_plan(crate::graph_execution::GraphPlanRunParams {
            plans_dir: plans_root.to_path_buf(),
            workdir: workdir.to_path_buf(),
            quiet: false,
            json: false,
            resume_plan: None,
            fresh: false,
            force_resume: false,
            max_retries: None,
            max_tasks: 0,
            budget_override: None,
            no_budget: false,
            cli_model_override: None,
            dangerously_skip_permissions: false,
            log_file: None,
            worktree_per_task: false,
            worktree_per_task_explicit: false,
            rich_topology: false,
            promote: None,
            no_tui: true,
            state_hub: None,
            interrupt: None,
            max_parallel_plans: None,
            fail_fast: false,
            only_plans: None,
            live_agent_output: crate::graph_task_dispatch::LiveAgentOutput::ToolSteps,
            force_disk_check: false,
            effort: None,
            no_cascade: false,
            metrics: None,
        })
        .await?;
    if exit_code != crate::exit_codes::EXIT_SUCCESS {
        return Err(anyhow!(
            "generated plan execution failed (exit {exit_code})"
        ));
    }
    Ok(())
}

fn auto_plan_enabled(workdir: &Path) -> Result<bool> {
    let roko_toml = workdir.join("roko.toml");
    if roko_toml.is_file() {
        let text = std::fs::read_to_string(&roko_toml)
            .with_context(|| format!("read {}", roko_toml.display()))?;
        let raw: toml::Value =
            toml::from_str(&text).with_context(|| format!("parse {}", roko_toml.display()))?;
        if raw
            .get("prd")
            .and_then(|prd| prd.get("auto_plan"))
            .is_some()
        {
            let cfg: RokoConfig =
                toml::from_str(&text).with_context(|| format!("parse {}", roko_toml.display()))?;
            return Ok(cfg.prd.auto_plan);
        }
    }

    Ok(crate::load_resolved_config(workdir)?.config.auto_plan)
}

/// Generate implementation plans from a published PRD file.
///
/// The plan is written by the planner model ([`resolve_planner_model`]).
pub async fn generate_plan_from_prd(slug: &str, prd_path: &Path, dry_run: bool) -> Result<PathBuf> {
    let (plans_root, _) =
        generate_plan_from_prd_with_outcome(slug, prd_path, dry_run, None, None, None).await?;
    Ok(plans_root)
}

/// Generate implementation plans from a published PRD file, publishing each
/// agent call's spend on `live` when given (see
/// [`crate::plan_authoring::AuthoringSpend`]). The plan is written by the
/// planner model ([`resolve_planner_model`]).
pub async fn generate_plan_from_prd_isolated(
    slug: &str,
    prd_path: &Path,
    live: Option<TuiBridge>,
) -> Result<PathBuf> {
    let (plans_root, _) =
        generate_plan_from_prd_with_outcome(slug, prd_path, false, None, None, live).await?;
    Ok(plans_root)
}

/// Generate implementation plans from a published PRD file using an
/// explicit resolved model key from the caller.
pub async fn generate_plan_from_prd_with_model(
    slug: &str,
    prd_path: &Path,
    dry_run: bool,
    model: Option<&str>,
) -> Result<PathBuf> {
    let (plans_root, _) =
        generate_plan_from_prd_with_outcome(slug, prd_path, dry_run, None, model, None).await?;
    Ok(plans_root)
}

/// Generate implementation plans from a published PRD file with optional
/// failure context injected into the planning prompt.
pub async fn generate_plan_from_prd_with_failure_context(
    slug: &str,
    prd_path: &Path,
    dry_run: bool,
    failure_context: Option<&str>,
    model: Option<&str>,
) -> Result<PathBuf> {
    let (plans_root, _) =
        generate_plan_from_prd_with_outcome(slug, prd_path, dry_run, failure_context, model, None)
            .await?;
    Ok(plans_root)
}

/// Characters of the model's output that a failed plan generation prints.
const FAILURE_OUTPUT_CHARS: usize = 2000;
/// Characters of the model's output that a non-retriable agent error quotes.
const AGENT_ERROR_PREVIEW_CHARS: usize = 500;
/// Characters of the model's last output that a retry prompt quotes.
const RETRY_OUTPUT_CHARS: usize = 2000;

/// Default model escalation chain: haiku -> sonnet -> opus.
///
/// When the workspace configures models, chain models it does not configure
/// are skipped so we never escalate to a model it cannot run.
const DEFAULT_ESCALATION_CHAIN: &[&str] =
    &["claude-haiku-4-5", "claude-sonnet-4-6", "claude-opus-4-6"];

/// Return the next-tier model for escalation on validation failures.
///
/// Checks `tier_models` config first (keys: `"haiku"`, `"sonnet"`, `"opus"`),
/// falling back to [`DEFAULT_ESCALATION_CHAIN`]. `current` and the chain
/// entries are compared by slug, so a `[models.*]` key matches the chain entry
/// for its slug.
///
/// Escalation only moves up from `current`, which starts as the planner model.
/// It returns `None`, and the retry keeps `current`, when `current` is at the
/// top of the chain, when no configured model sits above it, or when `current`
/// is not in the chain at all: a model outside the chain, such as a newer
/// frontier planner, has no known rank, so no chain model is known to be
/// stronger.
///
/// When `models` is non-empty, only chain models it configures (by key or
/// slug) are eligible. An empty map disables filtering (backward compat).
fn next_tier_model(
    current: Option<&str>,
    tier_models: &HashMap<String, String>,
    models: &IndexMap<String, ModelProfile>,
) -> Option<String> {
    // Build the chain from config or defaults.
    let chain: Vec<&str> = if tier_models.is_empty() {
        DEFAULT_ESCALATION_CHAIN.to_vec()
    } else {
        // Config keys in escalation order.
        ["haiku", "sonnet", "opus"]
            .iter()
            .filter_map(|k| tier_models.get(*k).map(String::as_str))
            .collect()
    };

    // Find the current model's rank; a model outside the chain stays put.
    let current_slug = model_slug(models, current?);
    let pos = chain
        .iter()
        .position(|m| model_slug(models, m) == current_slug)?;

    // Candidates above the current position. When models are configured, only
    // return one the workspace configures so we don't escalate to an
    // unavailable model.
    let mut candidates = chain[pos + 1..].iter().copied();
    if models.is_empty() {
        candidates.next().map(str::to_string)
    } else {
        candidates
            .find(|m| model_is_configured(models, m))
            .map(str::to_string)
    }
}

/// The provider slug `model` names: the slug of its `[models.*]` entry when it
/// is a key there, else `model` itself.
fn model_slug<'a>(models: &'a IndexMap<String, ModelProfile>, model: &'a str) -> &'a str {
    models
        .get(model)
        .map(|profile| profile.slug.trim())
        .filter(|slug| !slug.is_empty())
        .unwrap_or(model)
}

/// Whether `model` names a `[models.*]` entry, by key or by slug.
fn model_is_configured(models: &IndexMap<String, ModelProfile>, model: &str) -> bool {
    models.contains_key(model) || models.values().any(|profile| profile.slug.trim() == model)
}

/// What a plan is generated from. Every plan-generating command runs
/// [`generate_plan`] (gap-2623b2), so planner fixes land in one place.
#[derive(Debug, Clone, Copy)]
pub enum PlanSource<'a> {
    /// A PRD file (`roko prd plan`, `roko do`'s complex band, serve). Its
    /// frontmatter picks the plan template, the plan records it as
    /// `source_prd`, and it records the plan.
    Prd(&'a Path),
    /// Text from `roko plan generate` or `roko do`'s standard band.
    Text {
        /// The text to plan from.
        text: &'a str,
        /// What the text is (`"prompt"`, `"file"`, `"notes"`).
        kind: &'a str,
    },
    /// A plan directory (`roko plan regenerate`, and the old-format plans
    /// `roko prd plan` refreshes), regenerated in place from its source
    /// document. Tasks marked done stay done.
    Regenerate(&'a Path),
}

/// One run of the plan generator ([`generate_plan`]).
pub struct PlanRequest<'a> {
    /// What the plan is generated from.
    pub source: PlanSource<'a>,
    /// The plan's slug: its `meta.plan` and its directory name.
    pub slug: &'a str,
    /// The workspace the plan is for.
    pub workdir: &'a Path,
    /// Where plan directories go. `None` is the workspace plans directory,
    /// or a regenerated plan's parent.
    pub plans_root: Option<&'a Path>,
    /// More for the planner to read after the source: `--context` files or
    /// earlier validation findings.
    pub context: Option<&'a str>,
    /// Plan in a scratch copy of the workspace, and only report.
    pub dry_run: bool,
    /// Why an earlier plan failed, for replanning.
    pub failure_context: Option<&'a str>,
    /// The planner model. `None` is `[authoring] planner_model`.
    pub model: Option<&'a str>,
    /// The planner's reasoning effort. `None` is `[agent] effort`.
    pub effort: Option<&'a str>,
    /// Where each agent call's spend is published.
    pub live: Option<TuiBridge>,
}

impl<'a> PlanRequest<'a> {
    /// A request for the plan `slug` from `source`, with every option unset.
    #[must_use]
    pub fn new(source: PlanSource<'a>, slug: &'a str, workdir: &'a Path) -> Self {
        Self {
            source,
            slug,
            workdir,
            plans_root: None,
            context: None,
            dry_run: false,
            failure_context: None,
            model: None,
            effort: None,
            live: None,
        }
    }
}

/// How much of its source a planner reads inline, and how many repository
/// files it may open, by its context window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PlannerBudget {
    /// Characters of source in the prompt.
    source_chars: usize,
    /// Repository files the planner may read.
    read_files: usize,
}

impl PlannerBudget {
    /// The old fixed caps: the budget of a planner whose window is unknown,
    /// and the floor for every planner.
    const SMALL: Self = Self {
        source_chars: 8_000,
        read_files: 5,
    };

    /// A quarter of a `window`-token context for the source (about four
    /// characters a token), and one file per 2,000 tokens of another quarter.
    fn for_context_window(window: Option<u64>) -> Self {
        let Some(quarter) = window.map(|window| usize::try_from(window / 4).unwrap_or(usize::MAX))
        else {
            return Self::SMALL;
        };
        Self {
            source_chars: quarter.saturating_mul(4).max(Self::SMALL.source_chars),
            read_files: (quarter / 2_000).max(Self::SMALL.read_files),
        }
    }
}

/// The context window a `[models.*]` entry (by key, else by slug) states for
/// `model`.
pub(crate) fn planner_context_window(
    models: &IndexMap<String, ModelProfile>,
    model: &str,
) -> Option<u64> {
    models
        .get(model)
        .or_else(|| models.values().find(|profile| profile.slug.trim() == model))
        .map(|profile| profile.context_window)
        .filter(|&window| window > 0)
}

/// A candidate plan's spec-quality scores, by the static rules
/// ([`roko_gate::spec_quality`], sq-3), as generation acts on them (3218).
#[derive(Debug, Clone)]
struct SpecScores {
    /// One `task <id> <rule>: <detail>` line per hard fail.
    hard_fails: Vec<String>,
    /// The mean task score, 0–100.
    mean: f64,
    /// The lowest task score.
    min: f64,
    /// Tasks per band.
    bands: std::collections::BTreeMap<&'static str, usize>,
    /// One line per task below `allow_threshold`, naming the rules it scored
    /// 0 on.
    weak: Vec<String>,
}

impl SpecScores {
    /// Whether generation asks the planner once more: a plan mean below
    /// `allow_threshold`, or a task below `block_threshold`.
    fn is_weak(&self, config: &roko_core::config::SpecQualityConfig) -> bool {
        self.mean < config.allow_threshold || self.min < config.block_threshold
    }

    /// The line generation prints.
    fn summary_line(&self) -> String {
        let bands: Vec<String> = ["A", "B", "C", "D"]
            .iter()
            .map(|band| format!("{band} {}", self.bands.get(band).copied().unwrap_or(0)))
            .collect();
        format!(
            "Spec quality: mean {:.1}, lowest task {:.1}; bands {}",
            self.mean,
            self.min,
            bands.join(", ")
        )
    }

    /// What [`GenerationOutcome`] records.
    fn record(&self, regenerated: bool) -> roko_learn::runtime_feedback::GenerationSpecQuality {
        roko_learn::runtime_feedback::GenerationSpecQuality {
            mean: self.mean,
            min: self.min,
            bands: self
                .bands
                .iter()
                .map(|(band, count)| ((*band).to_string(), *count))
                .collect(),
            regenerated,
        }
    }
}

/// Score a candidate `tasks.toml` against the workspace at `workdir`; `None`
/// when `[spec_quality] mode = "off"` or nothing could be scored.
fn spec_scores(
    candidate: &str,
    workdir: &Path,
    config: &roko_core::config::SpecQualityConfig,
) -> Option<SpecScores> {
    if !config.is_on() {
        return None;
    }
    let scratch = tempfile::tempdir().ok()?;
    let path = scratch.path().join("tasks.toml");
    std::fs::write(&path, candidate).ok()?;
    let report = roko_gate::spec_quality::lint_files(&[path], workdir);
    if report.tasks.is_empty() {
        return None;
    }
    let scores: Vec<f64> = report.tasks.iter().map(|record| record.score).collect();
    let mean = scores.iter().sum::<f64>() / scores.len() as f64;
    let min = scores.iter().copied().fold(f64::INFINITY, f64::min);
    let mut bands = std::collections::BTreeMap::new();
    let mut hard_fails = Vec::new();
    let mut weak = Vec::new();
    for record in &report.tasks {
        *bands.entry(record.band).or_insert(0) += 1;
        for &rule in &record.hard_fail {
            let detail = record
                .hard_fail_detail
                .get(rule)
                .filter(|details| !details.is_empty())
                .map_or_else(
                    || hard_fail_name(rule).to_string(),
                    |details| details.join("; "),
                );
            hard_fails.push(format!("task {} {rule}: {detail}", record.task_id));
        }
        if record.score < config.allow_threshold {
            let zero: Vec<String> = roko_gate::spec_quality::RULES
                .iter()
                .filter(|rule| !record.excluded.contains(&rule.id))
                .filter(|rule| record.rules.get(rule.id).is_some_and(|score| *score <= 0.0))
                .map(|rule| format!("{} ({})", rule.id, rule.name))
                .collect();
            weak.push(format!(
                "- task {} (score {:.1}): {}",
                record.task_id,
                record.score,
                if zero.is_empty() {
                    "no rule scored 0; strengthen the partial ones".to_string()
                } else {
                    format!("scored 0 on {}", zero.join(", "))
                }
            ));
        }
    }
    Some(SpecScores {
        hard_fails,
        mean,
        min,
        bands,
        weak,
    })
}

/// What a hard fail means, as `HARD_FAILS` names it.
fn hard_fail_name(rule: &str) -> &'static str {
    roko_gate::spec_quality::HARD_FAILS
        .iter()
        .find(|(id, _)| *id == rule)
        .map_or("hard fail", |(_, name)| *name)
}

/// The prompt that asks the planner once more for a weak plan (3218): the
/// original request, the plan it wrote, and each weak task's rules that
/// scored 0.
fn spec_regeneration_prompt(task_prompt: &str, plan: &str, scores: &SpecScores) -> String {
    format!(
        "{task_prompt}\n\n---\n\nYour plan below scored low on spec quality (mean {:.1}, \
         lowest task {:.1}). Each weak task lists the spec rules it scored 0 on:\n{}\n\n\
         ```toml\n{plan}\n```\n\n\
         Rewrite the whole plan so every weak task meets those rules, and keep what already \
         works. Output the complete plan as a single ```toml fenced block, followed only by \
         the ```accept:accept/<file> blocks of the tests it pins.",
        scores.mean,
        scores.min,
        scores.weak.join("\n")
    )
}

/// A plan source, read for the planner.
struct ReadSource<'a> {
    origin: PlanSource<'a>,
    /// What the planner plans from.
    content: String,
    /// What the prompt calls it (`PRD`, `prompt`, `plan`, ...).
    kind: String,
    /// The file the content came from, which the planner need not reopen.
    path: Option<PathBuf>,
    /// Keywords for the repository context, beside the slug's.
    title: String,
    template: crate::plan_generate::PlanTemplateKind,
    /// The plan being regenerated, for [`PlanSource::Regenerate`].
    regeneration: Option<Regeneration>,
}

/// A plan being regenerated in place: its directory and current tasks.toml.
struct Regeneration {
    plan_dir: PathBuf,
    existing_toml: String,
    existing: Option<TasksFile>,
}

impl<'a> ReadSource<'a> {
    fn read(origin: PlanSource<'a>) -> Result<Self> {
        let read = |path: &Path| {
            std::fs::read_to_string(path).with_context(|| format!("read {}", path.display()))
        };
        Ok(match origin {
            PlanSource::Prd(path) => {
                let content = read(path)?;
                let meta = PrdMeta::parse(&content).unwrap_or_default();
                Self {
                    origin,
                    kind: "PRD".to_string(),
                    path: Some(path.to_path_buf()),
                    title: meta.title,
                    template: crate::plan_generate::PlanTemplateKind::resolve(
                        meta.plan_template.as_deref(),
                    ),
                    content,
                    regeneration: None,
                }
            }
            PlanSource::Text { text, kind } => Self {
                origin,
                content: text.to_string(),
                kind: kind.to_string(),
                path: None,
                title: String::new(),
                template: crate::plan_generate::PlanTemplateKind::resolve(None),
                regeneration: None,
            },
            PlanSource::Regenerate(plan_dir) => {
                let existing_toml = read(&plan_dir.join("tasks.toml"))?;
                let path = find_plan_source_document(plan_dir)?;
                Self {
                    origin,
                    content: read(&path)?,
                    kind: "plan".to_string(),
                    path: Some(path),
                    title: plan_dir
                        .file_name()
                        .map(|name| name.to_string_lossy().into_owned())
                        .unwrap_or_default(),
                    template: crate::plan_generate::PlanTemplateKind::resolve(None),
                    regeneration: Some(Regeneration {
                        plan_dir: plan_dir.to_path_buf(),
                        existing: TasksFile::parse_str(&existing_toml).ok(),
                        existing_toml,
                    }),
                }
            }
        })
    }

    /// The `task_id` and task kind of the planner call's episode.
    fn episode(&self, slug: &str) -> (String, &'static str) {
        match self.origin {
            PlanSource::Prd(_) => (format!("prd:plan:{slug}"), "prd-plan-generate"),
            PlanSource::Text { .. } => (format!("plan:generate:{slug}"), "plan-generate"),
            PlanSource::Regenerate(_) => (format!("plan:regenerate:{slug}"), "plan-regenerate"),
        }
    }
}

/// The planner's task prompt: `source` within `budget`, what to output, and
/// the checks the output must pass. `extra` follows the source.
fn plan_task_prompt(
    source: &ReadSource<'_>,
    slug: &str,
    budget: PlannerBudget,
    template_guidance: &str,
    extra: &str,
) -> String {
    let kind = source.kind.as_str();
    let content = if source.content.len() > budget.source_chars {
        let boundary = source.content.floor_char_boundary(budget.source_chars);
        format!(
            "{}\n\n[{kind} content truncated at {} chars]",
            &source.content[..boundary],
            budget.source_chars
        )
    } else {
        source.content.clone()
    };
    let (task, shape) = match &source.regeneration {
        Some(regeneration) => (
            "Regenerate the plan below from its source document, with full modern metadata."
                .to_string(),
            format!(
                "Keep each task's id where the task still applies; tasks already marked done \
                 stay done.\n\n## Existing tasks.toml\n\n```toml\n{}\n```",
                regeneration.existing_toml
            ),
        ),
        None => (
            format!("Generate an implementation plan from the {kind} below."),
            "Each requirement (REQ-XXX in a PRD) becomes one or more tasks. Each acceptance \
             criterion becomes a task verification command."
                .to_string(),
        ),
    };
    let reopen = source.path.as_ref().map_or_else(String::new, |path| {
        format!(" — do NOT read {} again", path.display())
    });
    format!(
        "{task}\n\n\
         Plan slug (use exactly in meta.plan): {slug}\n\n\
         IMPORTANT: The {kind} content is included inline{reopen}. You may read up to \
         {read_files} codebase files to understand existing structure, but then you MUST \
         produce your output.\n\n\
         {shape}\n\n\
         Do NOT create files directly. Instead, output the plan content \
         as follows:\n\n\
         1. Output a fenced block tagged `toml` containing the tasks.toml content.\n\
         2. Optionally output a fenced block tagged `plan.md` containing the plan narrative.\n\n\
         TOML quality checklist (every task MUST pass all of these):\n\
         - `meta.plan` matches the slug exactly: {slug} (use `plan =`, NOT `name =`)\n\
         - Every task has `id`, `title`, `description`, `status = \"ready\"`, `role`, and `tier`\n\
         - `files` lists only real paths that exist in the codebase (no placeholders)\n\
         - `depends_on` only references task ids defined in this same plan\n\
         - No `model_hint` field: `tier` and `role` pick each task's model; add `rung` \
           (for example `rung = \"strong\"`) only when a task needs more than its tier's \
           start rung\n\
         - No `mcp_servers` field unless the task genuinely requires an MCP server\n\
         - Every `[[task.verify]]` entry has `phase` and `command`\n\
         - Output ONLY a fenced ```toml block followed optionally by a fenced \
           ```plan.md block — no prose, no explanation outside those blocks\n\n\
         {template_guidance}\n\
         {kind} content:\n{content}{extra}",
        read_files = budget.read_files,
    )
}

/// The slug of the plan in `plan_dir`: its `meta.plan`, else the directory's
/// name.
#[must_use]
pub fn plan_dir_slug(plan_dir: &Path) -> String {
    TasksFile::parse(&plan_dir.join("tasks.toml"))
        .ok()
        .map(|tasks| tasks.meta.plan)
        .filter(|plan| !plan.trim().is_empty())
        .unwrap_or_else(|| {
            plan_dir.file_name().map_or_else(
                || "unknown-plan".to_string(),
                |name| name.to_string_lossy().into_owned(),
            )
        })
}

/// Write `validated_toml`, a regeneration of `regeneration`'s plan, into
/// `plan_dir`, keeping done tasks done and the plan's source PRD.
fn write_regenerated_plan(
    regeneration: &Regeneration,
    plan_dir: &Path,
    validated_toml: &str,
) -> Result<()> {
    let regenerated = TasksFile::parse_str(validated_toml)?;
    let mut merged =
        preserve_completed_task_status(regeneration.existing.as_ref(), regenerated, plan_dir);
    if merged.meta.source_prd.is_none() {
        merged.meta.source_prd = regeneration
            .existing
            .as_ref()
            .and_then(|tasks| tasks.meta.source_prd.clone());
    }
    let rendered = toml::to_string_pretty(&merged).context("serialize regenerated tasks.toml")?;
    atomic_write_str(&plan_dir.join("tasks.toml"), &rendered)
        .with_context(|| format!("write tasks.toml to {}", plan_dir.display()))?;
    println!("📋 Regenerated tasks.toml in {}", plan_dir.display());
    Ok(())
}

/// Generate the plan for the PRD at `prd_path`. Other plans are left alone:
/// refreshing old-format plans is opt-in ([`regenerate_old_format_plans`]).
async fn generate_plan_from_prd_with_outcome(
    slug: &str,
    prd_path: &Path,
    dry_run: bool,
    failure_context: Option<&str>,
    model: Option<&str>,
    live: Option<TuiBridge>,
) -> Result<(PathBuf, GenerationOutcome)> {
    let workdir = prd_workdir(prd_path)?;
    generate_plan(PlanRequest {
        dry_run,
        failure_context,
        model,
        live,
        ..PlanRequest::new(PlanSource::Prd(prd_path), slug, &workdir)
    })
    .await
}

/// Generate one plan from `request`'s source. The planner model writes a
/// tasks.toml, which is repaired, validated and checked against the
/// generated-plan policy (retrying, and escalating the model, when it fails)
/// before it is written under the plans root.
pub async fn generate_plan(request: PlanRequest<'_>) -> Result<(PathBuf, GenerationOutcome)> {
    let PlanRequest {
        source,
        slug,
        workdir,
        plans_root: requested_plans_root,
        context,
        dry_run,
        failure_context,
        model,
        effort,
        live,
    } = request;
    let requested_plans_root = requested_plans_root.or(match source {
        PlanSource::Regenerate(plan_dir) => plan_dir.parent(),
        _ => None,
    });
    let workdir = workdir.to_path_buf();
    let result = async {
        let t_total = Instant::now();
        let t_phase = Instant::now();
        let source = ReadSource::read(source)?;
        let template_kind = source.template;
        let template_guidance = crate::plan_generate::render_plan_template_guidance(template_kind);
        println!("📋 Generating a plan from the {}: {slug}", source.kind);

        let dry_run_workdir = if dry_run {
            Some(dry_run_fs::DryRunWorkspace::new(&workdir)?)
        } else {
            None
        };
        let workdir_ref = dry_run_workdir
            .as_ref()
            .map_or(workdir.as_path(), |temp| temp.path());
        // Every agent call below is recorded against the plan as it returns,
        // the same way task dispatch records its spend.
        let spend = AuthoringSpend::generation(workdir_ref, slug, live);

        let resolved = crate::load_resolved_config(workdir_ref)?;
        // Generated `rung` hints must name a rung of this ladder.
        let ladder = crate::plan_validate::workspace_ladder(workdir_ref);
        // Callers that pass no model (the serve runtime, auto-plan on
        // promote) plan with the planner model.
        let planner_model = match model {
            Some(model) => model.to_string(),
            None => resolve_planner_model(workdir_ref, None, "plan generation")?,
        };
        let system = augment_generator_system_prompt(
            crate::plan_generate::build_generator_system_prompt(workdir_ref),
            failure_context,
        );
        // A requested plans root in the workspace follows a dry run into its
        // scratch copy.
        let plans_root = requested_plans_root.map_or_else(
            || workspace_plans_dir(workdir_ref),
            |root| {
                root.strip_prefix(&workdir)
                    .map_or_else(|_| root.to_path_buf(), |relative| workdir_ref.join(relative))
            },
        );
        // A regenerated plan keeps its directory, whatever its slug.
        let plan_dir = match &source.regeneration {
            Some(regeneration) => {
                plans_root.join(regeneration.plan_dir.file_name().unwrap_or_default())
            }
            None => plans_root.join(slug),
        };
        let planner_effort = effort.unwrap_or(resolved.config.agent.effort.as_str());
        let tasks_before = dry_run_fs::snapshot_tasks_files(&plans_root);
        let init_ms = t_phase.elapsed().as_millis();

        // Build repo context to ground the planning agent in actual repository
        // structure. Keywords come from the PRD slug and title.
        let t_phase = Instant::now();
        let prd_title = source.title.as_str();
        let mut prd_feature_keywords: Vec<String> = slug
            .split(|c: char| c == '-' || c == '_' || c.is_whitespace())
            .chain(prd_title.split(|c: char| c == '-' || c == '_' || c.is_whitespace()))
            .filter(|w| w.len() >= 3)
            .map(|w| w.to_lowercase())
            .collect();
        prd_feature_keywords.sort_unstable();
        prd_feature_keywords.dedup();
        prd_feature_keywords.truncate(10);
        let prd_keyword_refs: Vec<&str> = prd_feature_keywords.iter().map(String::as_str).collect();
        // Skip repo context scanning for workspaces without source code
        // (e.g. freshly-initialized workspaces from `roko init`).
        let has_source_code = workdir_ref.join("src").is_dir()
            || workdir_ref.join("crates").is_dir()
            || workdir_ref.join("lib").is_dir()
            || workdir_ref.join("Cargo.toml").is_file()
            || workdir_ref.join("package.json").is_file();
        let repo_context_section: Option<String> = if has_source_code {
            match crate::repo_context::build_repo_context(workdir_ref, &prd_keyword_refs).await {
                Ok(repo_context) => {
                    if !repo_context.context_root_verified {
                        eprintln!(
                            "warning: repository context not verified for keywords {:?}; \
                             generated plan may reference nonexistent code.",
                            prd_feature_keywords
                        );
                    }
                    Some(repo_context.to_prompt_section())
                }
                Err(err) => {
                    eprintln!(
                        "warning: repository context unavailable for keywords {:?}: {err}",
                        prd_feature_keywords
                    );
                    None
                }
            }
        } else {
            None // Empty workspace — skip context scanning
        };
        let context_ms = t_phase.elapsed().as_millis();
        let t_phase = Instant::now();
        let mut extra = String::new();
        if let Some(context) = context.map(str::trim).filter(|context| !context.is_empty()) {
            extra.push_str("\n\n");
            extra.push_str(context);
        }
        if let Some(repo_context) = &repo_context_section {
            extra.push_str("\n\n---\n\n");
            extra.push_str(repo_context);
        }

        // A planner with a large context window sees the whole source
        // (gap-2623b2); the old fixed caps are the floor.
        let budget = PlannerBudget::for_context_window(planner_context_window(
            &resolved.config.models,
            &planner_model,
        ));
        let task_prompt = plan_task_prompt(&source, slug, budget, &template_guidance, &extra);

        let prompt_ms = t_phase.elapsed().as_millis();
        let t_phase = Instant::now();
        let (task_id, task_kind) = source.episode(slug);
        let effective_model = Some(planner_model.as_str());
        let plan_agent_command =
            command_from_config(workdir_ref).unwrap_or_else(|| "claude".to_string());
        let plan_started = Instant::now();
        eprintln!("  Generating plan from PRD: {slug}");
        let call = run_agent_capture_silent_with_usage(AgentExecOpts {
            prompt: &task_prompt,
            workdir: workdir_ref,
            model: effective_model,
            effort: Some(planner_effort),
            system_prompt: Some(&system),
            resume_session: None,
            env_vars: &resolved.config.agent.env,
            role: Some("strategist"),
            allowed_tools: Some("Read,Grep,Glob"),
        })
        .await?;
        spend.record(&call).await;
        let (exit_code, output) = (call.exit_code, call.output);
        let agent_ms = t_phase.elapsed().as_millis();
        let t_phase = Instant::now();
        if exit_code == 0 {
            eprintln!("  ✓ Plan generated for: {slug}");
        } else {
            eprintln!("  ✗ Plan generation failed");
        }
        tracing::info!(
            exit_code,
            output_len = output.len(),
            output_trimmed_len = output.trim().len(),
            "prd plan: agent returned"
        );
        // ── Crash classification and retry for non-zero exit codes ──────
        let (_exit_code, output) = if exit_code != 0 {
            let crash_class = classify_agent_crash(&output);
            let _ = persist_capture_episode(
                workdir_ref,
                &plan_agent_command,
                effective_model,
                task_kind,
                &task_id,
                &task_prompt,
                &output,
                false,
                plan_started.elapsed().as_millis() as u64,
                None,
            )
            .await;

            // Auth errors: fail fast with actionable message.
            if matches!(crash_class, AgentCrashClass::AuthenticationError) {
                return Err(anyhow!(
                    "plan generation agent auth failure (exit code {exit_code}): {}\n\
                     Hint: Check ANTHROPIC_API_KEY or your provider config in roko.toml",
                    crash_class.recovery_hint()
                ));
            }

            // Non-retriable errors (model not found, context overflow): fail
            // immediately because a retry will hit the same permanent error.
            if !crash_class.is_retriable() {
                let preview = crate::run::truncate(&output, AGENT_ERROR_PREVIEW_CHARS);
                return Err(anyhow!(
                    "plan generation agent failed (exit code {exit_code}, {crash_class:?}): \
                     {preview}\nHint: {}",
                    crash_class.recovery_hint()
                ));
            }

            // Retriable errors (rate limit, network, unknown): retry with
            // exponential back-off. Unknown crashes (signal/OOM/transient)
            // get 2 attempts; rate-limit / network get 3.
            let max_crash_retries: u32 = if matches!(crash_class, AgentCrashClass::Unknown) {
                2
            } else {
                3
            };
            let mut last_exit_code = exit_code;
            let mut last_output = output;

            for attempt in 1..=max_crash_retries {
                let retry_class = classify_agent_crash(&last_output);
                // Exponential back-off: 1 s, 2 s, 4 s …
                let backoff_secs = 1u64 << (attempt - 1).min(3);
                eprintln!(
                    "  plan generation agent {} (attempt {}/{}) \u{2014} \
                     waiting {backoff_secs}s then retrying\u{2026}",
                    retry_class.recovery_hint(),
                    attempt,
                    max_crash_retries + 1,
                );
                tokio::time::sleep(std::time::Duration::from_secs(backoff_secs)).await;
                let retry = run_agent_capture_silent_with_usage(AgentExecOpts {
                    prompt: &task_prompt,
                    workdir: workdir_ref,
                    model: effective_model,
                    effort: Some(planner_effort),
                    system_prompt: Some(&system),
                    resume_session: None,
                    env_vars: &resolved.config.agent.env,
                    role: Some("strategist"),
                    allowed_tools: Some("Read,Grep,Glob"),
                })
                .await?;
                spend.record(&retry).await;
                last_exit_code = retry.exit_code;
                last_output = retry.output;
                if last_exit_code == 0 {
                    eprintln!("  \u{2713} Retry {attempt} succeeded");
                    break;
                }
            }

            if last_exit_code != 0 {
                let final_class = classify_agent_crash(&last_output);
                return Err(anyhow!(
                    "plan generation agent failed after {} attempts \
                     (last exit code {last_exit_code}): {}",
                    max_crash_retries + 1,
                    final_class.recovery_hint()
                ));
            }

            (last_exit_code, last_output)
        } else {
            (exit_code, output)
        };
        if output.trim().is_empty() {
            let _ = persist_capture_episode(
                workdir_ref,
                &plan_agent_command,
                effective_model,
                task_kind,
                &task_id,
                &task_prompt,
                &output,
                false,
                plan_started.elapsed().as_millis() as u64,
                None,
            )
            .await;
            return Err(anyhow!(
                "plan generation agent returned empty output for {slug} — \
                 the model may not support the required output format"
            ));
        }

        // Write files from agent output (strategist can't write files directly).
        // Try fenced ```toml block first, then ```tasks.toml, then unfenced TOML.
        //
        // If extraction or validation fails, retry up to 2 times with a
        // stricter prompt requesting only the TOML block.
        let extract_and_validate =
            |raw: &str| -> std::result::Result<String, String> {
                let toml_content = extract_fenced_block(raw, "toml")
                    .or_else(|| extract_fenced_block(raw, "tasks.toml"))
                    .or_else(|| extract_toml_content_fallback(raw));
                tracing::info!(
                    has_toml_block = toml_content.is_some(),
                    toml_block_len = toml_content.map(|s| s.len()).unwrap_or(0),
                    "prd plan: fenced block extraction"
                );
                let toml_content = toml_content
                    .ok_or_else(|| "no TOML block found in agent output".to_string())?;

                // Fast structural pre-check: verify required sections are present
                // before attempting a full TOML parse.
                if !toml_content.contains("[meta]") {
                    return Err(
                        "TOML block is missing the required [meta] section".to_string(),
                    );
                }
                if !toml_content.contains("[[task]]") {
                    return Err(
                        "TOML block is missing required [[task]] entries".to_string(),
                    );
                }

                let validated = validate_and_fix_generated_plan(
                    toml_content,
                    slug,
                    &resolved.config.models,
                    resolved.config.agent.model.as_deref(),
                    &ladder,
                )
                .map_err(|e| format!("{e:#}"))?;
                let parsed = TasksFile::parse_str(&validated).map_err(|error| {
                    format!("generated plan failed runtime parsing after repair: {error:#}")
                })?;
                let policy = crate::plan_policy::PlanExecutionPolicy::generated_for_environment(
                    template_kind.max_task_count(),
                );
                let policy_issues = crate::plan_policy::validate_plan_budgets(&parsed, policy);
                if !policy_issues.is_empty() {
                    return Err(format!(
                        "generated plan violates the `{}` structural budget:\n{}",
                        template_kind.label(),
                        policy_issues
                            .iter()
                            .map(|issue| format!("  - {issue}"))
                            .collect::<Vec<_>>()
                            .join("\n")
                    ));
                }

                // Run the same context check that plan_loader.rs enforces at
                // load time so the written tasks.toml is guaranteed to pass.
                // PLAN_ARTIFACT_MISSING is suppressed because tasks.toml has
                // not been written yet; PLAN_SOURCE_PRD_MISSING is suppressed
                // because source_prd is injected after this closure returns.
                let mut ctx_violations = crate::plan_policy::validate_plan_context(
                    &parsed,
                    workdir_ref,
                    &plan_dir,
                    crate::plan_policy::PlanExecutionPolicy::for_environment(),
                );
                ctx_violations.retain(|v| {
                    !matches!(v.code, "PLAN_ARTIFACT_MISSING" | "PLAN_SOURCE_PRD_MISSING")
                });

                if ctx_violations.is_empty() {
                    return Ok(validated);
                }

                // Collect paths flagged as PLAN_CONTEXT_MISSING and drop them
                // so the written tasks.toml always passes the loader's check.
                let missing_paths: HashSet<String> = ctx_violations
                    .iter()
                    .filter(|v| v.code == "PLAN_CONTEXT_MISSING")
                    .filter_map(|v| {
                        v.message
                            .strip_prefix("declared context file `")
                            .and_then(|s| s.find('`').map(|end| s[..end].to_string()))
                    })
                    .collect();

                if missing_paths.is_empty() {
                    // Violations are not PLAN_CONTEXT_MISSING — cannot auto-fix.
                    return Err(format!(
                        "generated plan violates execution context policy:\n{}",
                        ctx_violations
                            .iter()
                            .map(|v| format!("  - {v}"))
                            .collect::<Vec<_>>()
                            .join("\n")
                    ));
                }

                // Drop the unreachable entries and re-serialize.
                let mut fixed = parsed.clone();
                for task in &mut fixed.tasks {
                    if let Some(ctx) = task.context.as_mut() {
                        ctx.read_files.retain(|f| !missing_paths.contains(&f.path));
                    }
                }
                let re_serialized = toml::to_string_pretty(&fixed).map_err(|e| {
                    format!("re-serialize after dropping missing read_files: {e}")
                })?;
                let re_parsed = TasksFile::parse_str(&re_serialized).map_err(|e| {
                    format!("re-parse after dropping missing read_files: {e}")
                })?;

                // Second context check — must be clean now.
                let mut remaining = crate::plan_policy::validate_plan_context(
                    &re_parsed,
                    workdir_ref,
                    &plan_dir,
                    crate::plan_policy::PlanExecutionPolicy::for_environment(),
                );
                remaining.retain(|v| {
                    !matches!(v.code, "PLAN_ARTIFACT_MISSING" | "PLAN_SOURCE_PRD_MISSING")
                });

                if remaining.is_empty() {
                    Ok(re_serialized)
                } else {
                    Err(format!(
                        "generated plan still violates execution context policy \
                         after dropping {} missing read_files:\n{}",
                        missing_paths.len(),
                        remaining
                            .iter()
                            .map(|v| format!("  - {v}"))
                            .collect::<Vec<_>>()
                            .join("\n")
                    ))
                }
            };

        // 3218: a spec hard fail (a verify step that can never fail, a missing
        // context file, ...) is a validation failure, so the retries below
        // name the task, the rule and the detail.
        //
        // 3221: the planner may write acceptance tests as `accept:<path>`
        // blocks; every `[task.accept]` src must be one of them (or a test
        // the plan already has), and the tests of the plan that is written
        // go beside its tasks.toml.
        let spec_config = resolved.config.spec_quality.clone();
        // A Mutex, not a RefCell: the closure below lives across awaits, and
        // the serve route needs this future to be Send.
        let accept_for: std::sync::Mutex<HashMap<String, Vec<accept_blocks::AcceptBlock>>> =
            std::sync::Mutex::default();
        let try_extract_and_validate = |raw: &str| -> std::result::Result<String, String> {
            let validated = extract_and_validate(raw)?;
            let accept = accept_blocks::extract(raw)?;
            accept_blocks::check_sources(&validated, &accept, &plan_dir)?;
            match spec_scores(&validated, workdir_ref, &spec_config) {
                Some(scores) if !scores.hard_fails.is_empty() => Err(format!(
                    "generated plan has spec hard fails; every check must be able to fail \
                     and every context file must exist:\n{}",
                    scores
                        .hard_fails
                        .iter()
                        .map(|line| format!("  - {line}"))
                        .collect::<Vec<_>>()
                        .join("\n")
                )),
                _ => {
                    accept_for
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner)
                        .insert(validated.clone(), accept);
                    Ok(validated)
                }
            }
        };

        // First attempt uses the output we already have.
        let mut validated_toml = try_extract_and_validate(&output);

        // Retry up to 2 times on extraction/validation failure, escalating
        // to a higher-tier model when TOML validation keeps failing.
        if validated_toml.is_err() {
            let max_retries = 2u32;
            let mut escalated_model: Option<String> = None;
            let mut last_output = output.clone();

            for attempt in 1..=max_retries {
                let t_retry = Instant::now();

                // Escalate model on format/validation failures (not auth/network),
                // only ever upward from the planner model, and only to a model
                // this workspace configures.
                let current_model = escalated_model
                    .as_deref()
                    .or(effective_model);
                if resolved.config.agent.escalation.escalate_model {
                    if let Some(next) = next_tier_model(
                        current_model,
                        &resolved.config.agent.tier_models,
                        &resolved.config.models,
                    ) {
                        tracing::info!(
                            from = current_model.unwrap_or("<default>"),
                            to = next.as_str(),
                            attempt,
                            "prd plan: escalating model tier on validation failure"
                        );
                        eprintln!(
                            "⬆️  Escalating model: {} → {next}",
                            current_model.unwrap_or("<default>"),
                        );
                        escalated_model = Some(next);
                    }
                }
                let retry_model = escalated_model.as_deref().or(effective_model);

                tracing::warn!(
                    attempt,
                    max_retries,
                    model = retry_model.unwrap_or("<default>"),
                    err = validated_toml.as_ref().unwrap_err().as_str(),
                    "prd plan: TOML extraction/validation failed, retrying"
                );
                eprintln!(
                    "⚠️  Plan TOML extraction failed (attempt {}/{}), retrying with stricter prompt…",
                    attempt,
                    max_retries + 1,
                );
                let error = validated_toml.as_ref().unwrap_err();
                let head = crate::run::truncate(&last_output, RETRY_OUTPUT_CHARS);
                let truncated_output = if head.len() < last_output.len() {
                    format!("{head}…(truncated)")
                } else {
                    last_output.clone()
                };
                let retry_prompt = format!(
                    "Previous attempt produced invalid TOML. Error: {error}\n\n\
                     Invalid output (truncated):\n```\n{truncated_output}\n```\n\n\
                     Please regenerate a valid tasks.toml. Your response must be a single \
                     ```toml fenced block, followed only by an ```accept:accept/<file> block for \
                     each test its [task.accept] entries name.\n\n\
                     MINIMUM REQUIRED STRUCTURE:\n\
                     ```toml\n\
                     [meta]\n\
                     plan = \"{slug}\"\n\
                     total = 1\n\
                     done = 0\n\
                     status = \"ready\"\n\
                     # max_parallel is omitted: tasks that do not depend on each other run together\n\n\
                     [[task]]\n\
                     id = \"T1\"\n\
                     title = \"Task title\"\n\
                     description = \"What this task does.\"\n\
                     status = \"ready\"\n\
                     tier = \"focused\"\n\
                     max_loc = 50\n\
                     files = [\"crates/roko-core/src/lib.rs\"]\n\
                     allowed_tools = [\"read_file\", \"grep\"]\n\
                     denied_tools = []\n\
                     depends_on = []\n\
                     role = \"implementer\"\n\
                     ```\n\n\
                     Do NOT include Rust code, markdown prose, or explanations outside the TOML block.\n\
                     Note: the meta field is `plan`, not `name`."
                );
                let retry_result = run_agent_capture_silent_with_usage(AgentExecOpts {
                    prompt: &retry_prompt,
                    workdir: workdir_ref,
                    model: retry_model,
                    effort: Some(planner_effort),
                    system_prompt: Some(&system),
                    resume_session: None,
                    env_vars: &resolved.config.agent.env,
                    role: Some("strategist"),
                    allowed_tools: Some("Read,Grep,Glob"),
                })
                .await;
                if let Ok(retry) = &retry_result {
                    spend.record(retry).await;
                }

                match retry_result.map(|retry| (retry.exit_code, retry.output)) {
                    Ok((0, retry_output)) if !retry_output.trim().is_empty() => {
                        last_output = retry_output.clone();
                        validated_toml = try_extract_and_validate(&retry_output);
                        if validated_toml.is_ok() {
                            tracing::info!(
                                attempt,
                                model = retry_model.unwrap_or("<default>"),
                                retry_ms = t_retry.elapsed().as_millis() as u64,
                                "prd plan: retry succeeded"
                            );
                            eprintln!("✅ Retry {attempt} succeeded");
                            break;
                        }
                    }
                    // Non-zero exit (auth/network errors) — do NOT escalate model,
                    // just log and retry with the same model.
                    Ok((code, _)) => {
                        tracing::warn!(
                            attempt,
                            code,
                            retry_ms = t_retry.elapsed().as_millis() as u64,
                            "prd plan: retry agent failed (not escalating — agent error)"
                        );
                        // Revert escalation: auth/network failures won't be fixed
                        // by a different model.
                        escalated_model = None;
                    }
                    Err(err) => {
                        tracing::warn!(
                            attempt,
                            %err,
                            retry_ms = t_retry.elapsed().as_millis() as u64,
                            "prd plan: retry agent error (not escalating — transport error)"
                        );
                        escalated_model = None;
                    }
                }
            }
        }

        // 3218: score the plan. A weak one (a mean below `[spec_quality]
        // allow_threshold`, or a task below `block_threshold`) is regenerated
        // once with each weak task's missing rules, and the better plan is
        // kept: no hard fail and a higher mean.
        let mut spec_quality = None;
        if let Ok(candidate) = &validated_toml
            && let Some(scores) = spec_scores(candidate, workdir_ref, &spec_config)
        {
            let mut kept = (candidate.clone(), scores);
            let weak = kept.1.is_weak(&spec_config);
            if weak {
                eprintln!(
                    "  Spec quality is low (mean {:.1}, lowest task {:.1}); asking the planner \
                     once more",
                    kept.1.mean, kept.1.min
                );
                let prompt = spec_regeneration_prompt(&task_prompt, &kept.0, &kept.1);
                let retry = run_agent_capture_silent_with_usage(AgentExecOpts {
                    prompt: &prompt,
                    workdir: workdir_ref,
                    model: effective_model,
                    effort: Some(planner_effort),
                    system_prompt: Some(&system),
                    resume_session: None,
                    env_vars: &resolved.config.agent.env,
                    role: Some("strategist"),
                    allowed_tools: Some("Read,Grep,Glob"),
                })
                .await;
                if let Ok(retry) = &retry {
                    spend.record(retry).await;
                }
                if let Ok(retry) = retry
                    && retry.exit_code == 0
                    && let Ok(better) = try_extract_and_validate(&retry.output)
                    && let Some(better_scores) = spec_scores(&better, workdir_ref, &spec_config)
                    && better_scores.mean > kept.1.mean
                {
                    kept = (better, better_scores);
                }
            }
            println!("📋 {}", kept.1.summary_line());
            spec_quality = Some(kept.1.record(weak));
            validated_toml = Ok(kept.0);
        }

        if let Ok(validated_toml) = validated_toml {
            let accept = accept_for
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .remove(&validated_toml)
                .unwrap_or_default();
            if let Some(regeneration) = &source.regeneration {
                write_regenerated_plan(regeneration, &plan_dir, &validated_toml)?;
            } else {
                // Inject source_prd into the [meta] section so cmd_status can
                // link plans back to their originating PRD by slug.
                let validated_toml = if !matches!(source.origin, PlanSource::Prd(_))
                    || validated_toml.contains("source_prd")
                {
                    validated_toml
                } else {
                    validated_toml.replacen(
                        "[meta]",
                        &format!("[meta]\nsource_prd = \"{slug}\""),
                        1,
                    )
                };
                std::fs::create_dir_all(&plan_dir)
                    .with_context(|| format!("create plan dir {}", plan_dir.display()))?;
                atomic_write_str(&plan_dir.join("tasks.toml"), &validated_toml)
                    .with_context(|| format!("write tasks.toml to {}", plan_dir.display()))?;
                println!(
                    "📋 Wrote tasks.toml ({} bytes) to {}",
                    validated_toml.len(),
                    plan_dir.display()
                );
                let plan_md_content = extract_fenced_block(&output, "plan.md")
                    .or_else(|| extract_fenced_block(&output, "markdown"))
                    .or_else(|| extract_fenced_block(&output, "md"));
                if let Some(plan_md) = plan_md_content {
                    atomic_write_str(&plan_dir.join("plan.md"), &plan_md)
                        .with_context(|| format!("write plan.md to {}", plan_dir.display()))?;
                    println!(
                        "📋 Wrote plan.md ({} bytes) to {}",
                        plan_md.len(),
                        plan_dir.display()
                    );
                } else {
                    // Write minimal plan.md so plan discovery tools can find
                    // this directory.
                    let minimal_plan_md = format!(
                        "---\nplan: {slug}\ntitle: {slug}\n---\n\n# {slug}\n\nGenerated plan.\n"
                    );
                    atomic_write_str(&plan_dir.join("plan.md"), &minimal_plan_md)
                        .with_context(|| format!("write plan.md to {}", plan_dir.display()))?;
                }
            }

            // 3221: the planner-written tests beside tasks.toml; a test the
            // plan already had stays unless the planner wrote it again.
            accept_blocks::write(&plan_dir, &accept)
                .with_context(|| format!("write the accept tests to {}", plan_dir.display()))?;

            // Update PRD frontmatter: record the generated plan slug.
            if let PlanSource::Prd(prd_path) = source.origin {
                if let Err(err) = update_prd_plans_generated(prd_path, slug) {
                    tracing::warn!(
                        slug = %slug,
                        error = %err,
                        "failed to update PRD plans_generated field"
                    );
                } else {
                    tracing::info!(slug = %slug, "updated PRD plans_generated field");
                }
            }
        } else {
            // All attempts (initial + retries) failed to produce valid TOML.
            let final_err = validated_toml.unwrap_err();
            let _preview: String = output.chars().take(500).collect();
            let has_toml_like = output.contains("[meta]") || output.contains("[[task]]");
            let toml_hint = if has_toml_like {
                "\nhint: The model output TOML without proper fencing. \
                 Try a more capable model or check the plan_generate system prompt."
            } else {
                ""
            };
            let _ = persist_capture_episode(
                workdir_ref,
                &plan_agent_command,
                effective_model,
                task_kind,
                &task_id,
                &task_prompt,
                &output,
                false,
                plan_started.elapsed().as_millis() as u64,
                None,
            )
            .await;
            eprintln!("--- Raw model output (first {FAILURE_OUTPUT_CHARS} chars) ---");
            eprintln!("{}", crate::run::truncate(&output, FAILURE_OUTPUT_CHARS));
            eprintln!("--- End raw model output ---");
            return Err(anyhow!(
                "Plan generation failed after retries: no valid tasks.toml was produced.\n\
                 Last error: {final_err}\n\
                 The agent output ({} bytes) did not contain a parseable ```toml block.\n\
                 hint: Try again, or create plans/{slug}/tasks.toml manually.{toml_hint}",
                output.len()
            ));
        }

        let extraction_ms = t_phase.elapsed().as_millis();
        tracing::info!(
            extraction_and_write_ms = extraction_ms,
            "prd plan: TOML extracted and written"
        );
        let t_phase = Instant::now();
        let generated_changed = dry_run_fs::changed_tasks_files(&plans_root, &tasks_before);

        let changed = dry_run_fs::changed_tasks_files(&plans_root, &tasks_before);
        let mut artifact_valid = true;
        let mut validation_report: Option<ArtifactValidationReport> = None;

        if dry_run {
            if changed.is_empty() {
                artifact_valid = false;
                eprintln!("warning: dry-run plan generation did not produce any tasks.toml files");
            } else {
                for path in &changed {
                    if let Err(err) = dry_run_fs::validate_and_print_preview(path) {
                        artifact_valid = false;
                        eprintln!(
                            "warning: dry-run validation failed for {}: {err:#}",
                            path.display()
                        );
                    }
                }
            }
        } else {
            dry_run_fs::warn_on_new_or_updated_tasks(&plans_root, &tasks_before);
        }

        let (task_count, estimated_complexity) = generated_plan_stats(&generated_changed)?;
        let max_tasks = crate::plan_policy::effective_generated_task_limit(
            template_kind.max_task_count(),
        );
        if task_count > max_tasks {
            eprintln!(
                "⚠️  Generated {task_count} tasks, which exceeds the `{}` template budget of {}",
                template_kind.label(),
                max_tasks
            );
        }

        // Validate the plan this call wrote, not the whole plans root: a broken
        // plan beside it must not fail the generation (bug-2d06bf).
        match crate::plan_validate::validate_plans_dir_with_workdir(
            &plan_dir,
            None,
            Some(workdir_ref),
        ) {
            Ok(report) => {
                if report.totals.errors > 0 {
                    artifact_valid = false;
                    eprintln!(
                        "warning: artifact validation found {} error(s) and {} warning(s) for {}",
                        report.totals.errors, report.totals.warnings, slug
                    );
                }
                validation_report = serde_json::to_value(&report).ok();
                if validation_report.is_none() {
                    artifact_valid = false;
                }
            }
            Err(err) => {
                artifact_valid = false;
                eprintln!(
                    "warning: artifact validation could not be completed for {}: {err:#}",
                    slug
                );
            }
        }

        let post_ms = t_phase.elapsed().as_millis();
        let total_ms = t_total.elapsed().as_millis();
        tracing::info!(
            init_ms,
            context_ms,
            prompt_ms,
            agent_ms,
            post_ms,
            total_ms,
            "prd plan generate: phase timing"
        );

        let outcome = GenerationOutcome {
            process_success: true,
            artifact_valid,
            validation_report,
            spec_quality,
        };

        let _ = persist_capture_episode(
            workdir_ref,
            &plan_agent_command,
            effective_model,
            task_kind,
            &task_id,
            &task_prompt,
            &output,
            outcome.fully_successful(),
            plan_started.elapsed().as_millis() as u64,
            None,
        )
        .await;

        Ok((
            requested_plans_root
                .map_or_else(|| workspace_plans_dir(&workdir), Path::to_path_buf),
            task_count,
            estimated_complexity,
            outcome,
        ))
    }
    .await;

    match result {
        Ok((plans_root, task_count, estimated_complexity, outcome)) => {
            if !dry_run && matches!(source, PlanSource::Prd(_)) {
                let signal_kind = if outcome.fully_successful() {
                    Some(Kind::Custom("prd:plan:generated".into()))
                } else if outcome.process_success {
                    Some(Kind::Custom("prd:plan:partial_success".into()))
                } else {
                    Some(Kind::Custom("prd:plan:failed".into()))
                };

                if let Some(kind) = signal_kind
                    && let Err(err) = emit_prd_plan_signal(
                        &workdir,
                        kind,
                        serde_json::json!({
                            "plan_path": plans_root.display().to_string(),
                            "task_count": task_count,
                            "estimated_complexity": estimated_complexity,
                            "status": outcome.status_label(),
                            "process_success": outcome.process_success,
                            "artifact_valid": outcome.artifact_valid,
                            "validation_report": outcome.validation_report,
                        }),
                    )
                    .await
                {
                    tracing::warn!("[prd] failed to emit plan signal: {err}");
                }
            }
            Ok((plans_root, outcome))
        }
        Err(err) => {
            if !dry_run
                && matches!(source, PlanSource::Prd(_))
                && let Err(signal_err) = emit_prd_plan_signal(
                    &workdir,
                    Kind::Custom("prd:plan:failed".into()),
                    serde_json::json!({
                        "plan_path": workspace_plans_dir(&workdir).display().to_string(),
                        "error": format!("{err:#}"),
                    }),
                )
                .await
            {
                tracing::warn!("[prd] failed to emit failed-plan signal: {signal_err}");
            }
            Err(err)
        }
    }
}

pub(crate) fn augment_generator_system_prompt(
    mut system_prompt: String,
    failure_context: Option<&str>,
) -> String {
    let Some(failure_context) = failure_context.map(str::trim).filter(|ctx| !ctx.is_empty()) else {
        return system_prompt;
    };

    system_prompt.push_str("\n\n## Failure context for replanning\n");
    system_prompt.push_str(failure_context);
    system_prompt.push_str(
        "\n\nUse this failure context to revise the plan first. Do not reproduce the same task shape.\n",
    );
    system_prompt
}

/// Build the system prompt for agent-assisted PRD commands.
///
/// Combines the PRD quality system prompt (from [`crate::prd_prompt`]) with
/// context about existing PRDs and the specific task.
/// Default limit for how many PRDs to include in the agent system prompt.
const PRD_CONTEXT_LIMIT: usize = 20;

pub fn prd_agent_prompt(workdir: &Path, task: &str) -> String {
    prd_agent_prompt_with_limit(workdir, task, PRD_CONTEXT_LIMIT)
}

pub fn prd_agent_prompt_with_limit(workdir: &Path, task: &str, prd_limit: usize) -> String {
    let mut prompt = String::new();

    // Include the PRD quality standards as the foundation
    let _ = writeln!(prompt, "{}", crate::prd_prompt::PRD_SYSTEM_PROMPT);
    let _ = writeln!(prompt, "\n---\n");
    let _ = writeln!(prompt, "## Project workspace: {}\n", workdir.display());

    // Include the master index so the agent knows everything that exists
    crate::index::append_master_index_prompt(
        &mut prompt,
        workdir,
        "## Master Index (what already exists — do NOT duplicate)",
    );

    // Include the PRD index for detailed cross-references
    let prd_index = std::fs::read_to_string(prd_dir(workdir).join("INDEX.md")).unwrap_or_default();
    if !prd_index.is_empty() {
        let _ = writeln!(prompt, "## PRD Index\n{prd_index}\n---\n");
    }

    // Gather existing PRD context (most recent first, limited to prd_limit)
    let _ = writeln!(
        prompt,
        "## Existing PRDs (for cross-references and consistency)\n"
    );
    let mut all_prd_files: Vec<PathBuf> = Vec::new();
    for dir in [&published_dir(workdir), &drafts_dir(workdir)] {
        all_prd_files.extend(list_md_files(dir));
    }
    // Sort by modification time descending so newest PRDs come first
    all_prd_files.sort_by(|a, b| {
        let mtime = |p: &Path| {
            std::fs::metadata(p)
                .and_then(|m| m.modified())
                .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
        };
        mtime(b).cmp(&mtime(a))
    });
    let total = all_prd_files.len();
    for path in all_prd_files.into_iter().take(prd_limit) {
        if let Ok(content) = std::fs::read_to_string(&path) {
            // Include just the frontmatter + first section as context
            let truncated: String = content.lines().take(30).collect::<Vec<_>>().join("\n");
            let _ = writeln!(prompt, "### {}\n{truncated}\n---\n", path.display());
        }
    }
    if total > prd_limit {
        let _ = writeln!(
            prompt,
            "_({} older PRDs omitted — see .roko/prd/ for full list)_\n",
            total - prd_limit,
        );
    }

    // Ideas
    let ideas = std::fs::read_to_string(ideas_path(workdir)).unwrap_or_default();
    if !ideas.is_empty() {
        let _ = writeln!(prompt, "## Recent ideas\n{ideas}\n");
    }

    let _ = writeln!(prompt, "## Your task\n{task}");
    let _ = writeln!(prompt, "\n{}", crate::prd_prompt::PRD_QUALITY_CHECKLIST);
    prompt
}

/// Generate the YAML frontmatter for a new draft.
pub fn new_draft_frontmatter(slug: &str, title: &str) -> String {
    let today = chrono::Local::now().format("%Y-%m-%d");
    format!(
        "---\n\
         id: prd-{slug}\n\
         title: {title}\n\
         status: draft\n\
         version: 1\n\
         created: {today}\n\
         updated: {today}\n\
         depends_on: []\n\
         crates: []\n\
         plans_generated: []\n\
         coverage: 0\n\
         tags: []\n\
         ---\n\n"
    )
}

/// Replace `from` with `to` only within the YAML frontmatter (between first
/// pair of `---` delimiters). Body content is left untouched.
fn replace_in_frontmatter(content: &str, from: &str, to: &str) -> String {
    // Frontmatter is the region between the first `---\n` and the next `---\n`.
    if let Some(start) = content.find("---") {
        let after_first = start + 3;
        if let Some(end_offset) = content[after_first..].find("---") {
            let end = after_first + end_offset + 3;
            let frontmatter = &content[..end];
            let body = &content[end..];
            return format!("{}{}", frontmatter.replace(from, to), body);
        }
    }
    // No frontmatter delimiters found — fall back to global replace.
    content.replace(from, to)
}

/// Returns true if a PRD markdown string contains substantive body content.
#[must_use]
pub fn has_substantive_markdown_content(content: &str) -> bool {
    let mut in_frontmatter = false;
    let mut saw_frontmatter = false;

    content.lines().any(|line| {
        let trimmed = line.trim();
        if trimmed == "---" {
            if !saw_frontmatter {
                saw_frontmatter = true;
                in_frontmatter = true;
                return false;
            }
            if in_frontmatter {
                in_frontmatter = false;
                return false;
            }
        }

        if in_frontmatter {
            return false;
        }

        !trimmed.is_empty() && !trimmed.starts_with('#')
    })
}

/// Normalize markdown emitted by an agent and optionally prepend a scaffold.
///
/// If the model returns fenced markdown, the outer code fence is stripped.
/// When `scaffold` is provided and the returned markdown lacks YAML frontmatter,
/// the scaffold is prepended so draft creation can still recover a full PRD file.
#[must_use]
pub fn materialize_agent_markdown_output(output: &str, scaffold: Option<&str>) -> Option<String> {
    let trimmed = output.trim();
    if trimmed.is_empty() {
        return None;
    }

    let normalized = strip_markdown_code_fence(trimmed).trim();
    if normalized.is_empty() {
        return None;
    }

    if let Some(scaffold) = scaffold
        && !normalized.starts_with("---")
    {
        return Some(format!("{scaffold}\n{normalized}"));
    }

    Some(normalized.to_string())
}

fn strip_markdown_code_fence(output: &str) -> &str {
    let trimmed = output.trim();
    if !trimmed.starts_with("```") {
        return trimmed;
    }

    let Some(first_newline) = trimmed.find('\n') else {
        return trimmed;
    };
    let inner = &trimmed[first_newline + 1..];
    let Some(closing) = inner.rfind("\n```") else {
        return trimmed;
    };
    &inner[..closing]
}

/// Extract the contents of a fenced code block tagged with `tag` from agent output.
///
/// Looks for `` ```tag `` or `` ```<tag> `` and returns the inner content.
/// Handles nested fences by matching the closing `` ``` `` that sits alone
/// on a line (possibly with trailing whitespace).
pub(crate) fn extract_fenced_block<'a>(text: &'a str, tag: &str) -> Option<&'a str> {
    let fence_plain = format!("```{tag}");
    let fence_angle = format!("```<{tag}>");
    let start = text
        .find(&fence_plain)
        .or_else(|| text.find(&fence_angle))?;
    let after_fence = &text[start..];
    let newline = after_fence.find('\n')? + 1;
    let inner = &after_fence[newline..];

    // Find a closing ``` that is alone on a line (not followed by more text
    // like ```toml or ```python — those are nested openers).
    let mut search_from = 0;
    loop {
        let candidate = inner[search_from..].find("\n```")?;
        let abs = search_from + candidate;
        let after_ticks = abs + 4; // position after \n```
        // Closing fence: either end-of-string, or next char is \n or whitespace-then-\n
        let rest = &inner[after_ticks..];
        if rest.is_empty()
            || rest.starts_with('\n')
            || rest.trim_start().starts_with('\n')
            || rest.trim_start().is_empty()
        {
            let content = inner[..abs].trim();
            return if content.is_empty() {
                None
            } else {
                Some(&inner[..abs])
            };
        }
        search_from = after_ticks;
    }
}

/// Fallback TOML extraction for agent output that lacks fenced code blocks.
///
/// Scans for a `[meta]` section header and returns everything from that point
/// up to the end of the TOML content, provided it also contains at least one
/// `[[task]]`.  Trailing explanatory text (markdown headings, horizontal rules,
/// prose paragraphs after a blank line) is trimmed so that `toml::from_str`
/// doesn't choke on non-TOML content the LLM appended after the plan.
pub(crate) fn extract_toml_content_fallback(output: &str) -> Option<&str> {
    let meta_start = output.find("[meta]")?;
    // Find the start of the line containing [meta]
    let line_start = output[..meta_start].rfind('\n').map(|i| i + 1).unwrap_or(0);
    let candidate = &output[line_start..];
    if !candidate.contains("[[task]]") {
        return None;
    }

    // Walk lines and find the last one that looks like TOML content.
    // Stop when we hit lines that are clearly markdown/prose trailing text.
    let mut last_toml_end = 0; // byte offset into `candidate`
    let mut saw_blank = false;
    for line in candidate.lines() {
        let trimmed = line.trim();

        if trimmed.is_empty() {
            saw_blank = true;
            // A blank line inside TOML is fine (between sections), so we don't
            // stop yet — only if the *next* non-blank line looks non-TOML.
            last_toml_end += line.len() + 1; // +1 for '\n'
            continue;
        }

        // Lines that are clearly not TOML — stop here.
        if trimmed.starts_with("## ")
            || trimmed.starts_with("---")
            || trimmed.starts_with("**")
            || trimmed.starts_with("> ")
            || trimmed.starts_with("Note:")
            || trimmed.starts_with("NOTE:")
        {
            break;
        }

        // After a blank line, the next non-blank line must still look like TOML
        // (contains `=`, starts with `[`, or is a `"""` / `'''` continuation).
        if saw_blank
            && !trimmed.contains('=')
            && !trimmed.starts_with('[')
            && !trimmed.starts_with('"')
            && !trimmed.starts_with('\'')
            && !trimmed.starts_with('#')
        // TOML comment
        {
            break;
        }

        saw_blank = false;
        last_toml_end += line.len() + 1;
    }

    let result = candidate[..last_toml_end].trim();
    if result.is_empty() || !result.contains("[[task]]") {
        None
    } else {
        Some(result)
    }
}

/// Update the PRD frontmatter to record that a plan was generated.
fn update_prd_plans_generated(prd_path: &std::path::Path, plan_slug: &str) -> anyhow::Result<()> {
    let content = std::fs::read_to_string(prd_path)?;

    let updated = if content.contains("plans_generated: []") {
        content.replace(
            "plans_generated: []",
            &format!("plans_generated: [\"{plan_slug}\"]"),
        )
    } else if let Some(pos) = content.find("plans_generated: [") {
        let after_bracket = pos + "plans_generated: [".len();
        if let Some(close) = content[after_bracket..].find(']') {
            let close_pos = after_bracket + close;
            let existing = content[after_bracket..close_pos].trim();
            if existing.is_empty() {
                format!(
                    "{}\"{plan_slug}\"{}",
                    &content[..after_bracket],
                    &content[close_pos..]
                )
            } else if existing.contains(plan_slug) {
                // Already listed
                return Ok(());
            } else {
                format!(
                    "{}, \"{plan_slug}\"{}",
                    &content[..close_pos],
                    &content[close_pos..]
                )
            }
        } else {
            return Ok(()); // Malformed, skip
        }
    } else {
        // No plans_generated field found -- don't modify
        return Ok(());
    };

    atomic_write_str(prd_path, &updated)?;
    Ok(())
}

// ---- post-generation plan TOML validation --------------------------------

/// Required field names for the `[meta]` section.
const REQUIRED_META_FIELDS: &[&str] = &["plan", "total", "status"];

/// Required field names for each `[[task]]`.
const REQUIRED_TASK_FIELDS: &[&str] = &["id", "title", "status", "role", "tier"];

/// Required field names for each `[[task.verify]]` entry.
const REQUIRED_VERIFY_FIELDS: &[&str] = &["phase", "command"];

/// Validate and fix a generated plan TOML string.
///
/// Checks:
/// 1. TOML syntax.
/// 2. Required fields in `[meta]` and `[[task]]`.
/// 3. Unknown / misspelled fields (with suggested corrections applied).
/// 4. `model_hint` values stripped, and `rung` hints kept only when they name
///    one of the task's `ladder` rungs.
/// 5. `meta.plan` matched against the expected slug.
///
/// On fixable issues the TOML is patched and a warning is logged to stderr.
/// On unfixable issues an error is returned.
fn validate_and_fix_generated_plan(
    toml_str: &str,
    slug: &str,
    _models: &IndexMap<String, roko_core::config::schema::ModelProfile>,
    _default_model: Option<&str>,
    ladder: &LadderConfig,
) -> Result<String> {
    // 0. Deterministic repair before parsing.
    let repaired = crate::task_parser::repair_toml(toml_str);
    // Also fix the common LLM mistake of using `name = ` instead of `plan = `
    // in the [meta] section. repair_toml leaves it alone because it parses
    // fine as TOML; we catch it here at the semantic level.
    let repaired = crate::task_parser::fix_meta_name_to_plan(&repaired);
    if repaired != toml_str {
        tracing::info!("prd plan: applied deterministic TOML repair");
    }

    // 1. Parse syntax.
    let mut root: toml::Value =
        toml::from_str(&repaired).map_err(|e| anyhow!("generated plan has invalid TOML: {e}"))?;

    let root_table = root
        .as_table_mut()
        .ok_or_else(|| anyhow!("generated plan TOML root is not a table"))?;

    let mut errors: Vec<String> = Vec::new();

    // -- [meta] validation ---------------------------------------------------
    if let Some(meta_val) = root_table.get_mut("meta") {
        if let Some(meta) = meta_val.as_table_mut() {
            // Flag unknown meta fields.
            let meta_keys: Vec<String> = meta.keys().cloned().collect();
            for key in &meta_keys {
                if !META_KEYS.contains(&key.as_str()) {
                    if let Some(correction) = suggest_field_correction(key, META_KEYS) {
                        if let Some(value) = meta.remove(key.as_str()) {
                            tracing::warn!(
                                "prd plan: [meta] field '{key}' is unknown; \
                                 corrected to '{correction}'"
                            );
                            meta.insert(correction, value);
                        }
                    } else {
                        tracing::warn!("prd plan: [meta] has unknown field '{key}'");
                    }
                }
            }
            // Check required meta fields.
            for &required in REQUIRED_META_FIELDS {
                match meta.get(required) {
                    None => errors.push(format!("[meta] is missing required field '{required}'")),
                    Some(v) if v.as_str().is_some_and(|s| s.trim().is_empty()) => {
                        errors.push(format!("[meta].{required} is empty"));
                    }
                    _ => {}
                }
            }
            // Fix meta.plan if truncated or wrong.
            if let Some(plan_val) = meta.get("plan") {
                if let Some(plan_str) = plan_val.as_str() {
                    if plan_str != slug {
                        if slug.starts_with(plan_str) {
                            tracing::warn!(
                                "prd plan: meta.plan '{plan_str}' appears truncated; \
                                 corrected to '{slug}'"
                            );
                        } else {
                            tracing::warn!(
                                "prd plan: meta.plan '{plan_str}' does not match \
                                 expected slug '{slug}'; corrected"
                            );
                        }
                        meta.insert("plan".to_string(), toml::Value::String(slug.to_string()));
                    }
                }
            }
        }
    } else {
        errors.push("[meta] section is missing".to_string());
    }

    // -- [[task]] validation --------------------------------------------------
    if let Some(tasks_val) = root_table.get_mut("task") {
        if let Some(tasks) = tasks_val.as_array_mut() {
            if tasks.is_empty() {
                errors.push("[[task]] array is present but empty".to_string());
            }
            for (i, task_val) in tasks.iter_mut().enumerate() {
                if let Some(task) = task_val.as_table_mut() {
                    let task_id_label: String = task
                        .get("id")
                        .and_then(toml::Value::as_str)
                        .map(String::from)
                        .unwrap_or_else(|| format!("task #{}", i + 1));

                    // Flag unknown task fields.
                    let task_keys: Vec<String> = task.keys().cloned().collect();
                    for key in &task_keys {
                        if !TASK_KEYS.contains(&key.as_str()) {
                            if let Some(correction) = suggest_field_correction(key, TASK_KEYS) {
                                if let Some(value) = task.remove(key.as_str()) {
                                    tracing::warn!(
                                        "prd plan: {task_id_label}: field '{key}' is unknown; \
                                         corrected to '{correction}'"
                                    );
                                    task.insert(correction, value);
                                }
                            } else {
                                tracing::warn!("prd plan: {task_id_label}: unknown field '{key}'");
                            }
                        }
                    }

                    // Check required task fields.
                    for &required in REQUIRED_TASK_FIELDS {
                        match task.get(required) {
                            None => errors.push(format!(
                                "{task_id_label} is missing required field '{required}'"
                            )),
                            Some(v) if v.as_str().is_some_and(|s| s.trim().is_empty()) => {
                                errors
                                    .push(format!("{task_id_label}: field '{required}' is empty"));
                            }
                            _ => {}
                        }
                    }

                    // Validate status value.
                    if let Some(status_val) = task.get("status").cloned() {
                        if let Some(s) = status_val.as_str() {
                            const VALID_STATUSES: &[&str] = &[
                                "ready",
                                "pending",
                                "blocked",
                                "in_progress",
                                "done",
                                "skipped",
                            ];
                            if !VALID_STATUSES.contains(&s) {
                                tracing::warn!(
                                    "prd plan: {task_id_label}: status '{s}' is invalid; \
                                     defaulting to 'ready'"
                                );
                                task.insert(
                                    "status".to_string(),
                                    toml::Value::String("ready".to_string()),
                                );
                            }
                        }
                    }

                    // Validate role value.
                    if let Some(role_val) = task.get("role").cloned() {
                        if let Some(r) = role_val.as_str() {
                            const VALID_ROLES: &[&str] = crate::task_parser::PLAN_TASK_ROLES;
                            if !VALID_ROLES.contains(&r) {
                                tracing::warn!(
                                    "prd plan: {task_id_label}: role '{r}' is invalid; \
                                     defaulting to 'implementer'"
                                );
                                task.insert(
                                    "role".to_string(),
                                    toml::Value::String("implementer".to_string()),
                                );
                            }
                        }
                    }

                    // Always strip model_hint from generated plans: a model
                    // name ties the plan to one provider, while the tier and
                    // role pick a model on this workspace's routing ladder.
                    if let Some(hint_val) = task.remove("model_hint") {
                        let hint = hint_val.as_str().unwrap_or("<unknown>");
                        tracing::info!(
                            "prd plan: {task_id_label}: removing model_hint '{hint}' \
                             (tier and role pick the model; a task that needs a \
                             stronger one names a `rung`)"
                        );
                    }

                    // gap-dbf2a6: keep a `rung` hint that names one of the
                    // task's ladder rungs; drop any other.
                    if let Some(rung) = crate::plan_validate::drop_unknown_rung(task, ladder) {
                        tracing::warn!(
                            "prd plan: {task_id_label}: removing rung {rung}: no rung of the \
                             routing ladder has that name"
                        );
                    }

                    // Validate [[task.verify]] sub-entries.
                    if let Some(verify_val) = task.get_mut("verify") {
                        if let Some(steps) = verify_val.as_array_mut() {
                            for (si, step_val) in steps.iter_mut().enumerate() {
                                if let Some(step) = step_val.as_table_mut() {
                                    let step_keys: Vec<String> = step.keys().cloned().collect();
                                    for key in &step_keys {
                                        if !VERIFY_KEYS.contains(&key.as_str()) {
                                            if let Some(correction) =
                                                suggest_field_correction(key, VERIFY_KEYS)
                                            {
                                                if let Some(value) = step.remove(key.as_str()) {
                                                    tracing::warn!(
                                                        "prd plan: {task_id_label} verify[{si}]: \
                                                         field '{key}' corrected to '{correction}'"
                                                    );
                                                    step.insert(correction, value);
                                                }
                                            } else {
                                                tracing::warn!(
                                                    "prd plan: {task_id_label} verify[{si}]: \
                                                     unknown field '{key}'"
                                                );
                                            }
                                        }
                                    }

                                    // Check required verify fields.
                                    for &required in REQUIRED_VERIFY_FIELDS {
                                        if step
                                            .get(required)
                                            .and_then(toml::Value::as_str)
                                            .is_none_or(|s| s.trim().is_empty())
                                        {
                                            errors.push(format!(
                                                "{task_id_label} verify[{si}]: \
                                                 missing required field '{required}'"
                                            ));
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // Auto-add verify entries for implementer tasks with files
                    // but no verify block. Researchers/strategists/scribes skip.
                    if task.get("verify").is_none() {
                        let role = task
                            .get("role")
                            .and_then(toml::Value::as_str)
                            .unwrap_or("implementer");
                        let files: Vec<String> = task
                            .get("files")
                            .and_then(toml::Value::as_array)
                            .map(|arr| {
                                arr.iter()
                                    .filter_map(toml::Value::as_str)
                                    .map(String::from)
                                    .collect()
                            })
                            .unwrap_or_default();

                        if role == "implementer" && !files.is_empty() {
                            // Infer crate name from file paths for compile check
                            let crate_name = infer_crate_from_paths(&files);
                            let compile_cmd = match &crate_name {
                                Some(c) => format!("cargo check -p {c}"),
                                None => "cargo check --workspace".to_string(),
                            };
                            let auto_verify = vec![make_verify_entry(
                                "compile",
                                &compile_cmd,
                                &format!(
                                    "{} must compile",
                                    crate_name.as_deref().unwrap_or("workspace"),
                                ),
                            )];

                            task.insert("verify".to_string(), toml::Value::Array(auto_verify));
                            tracing::info!(
                                "prd plan: {task_id_label}: auto-added one focused compile verify"
                            );
                        }
                    }
                }
            }
        }
    } else {
        errors.push("[[task]] array is missing".to_string());
    }

    if !errors.is_empty() {
        let joined = errors.join("\n  - ");
        return Err(anyhow!(
            "generated plan TOML has {n} validation error(s):\n  - {joined}",
            n = errors.len()
        ));
    }

    // Serialize the (possibly patched) TOML back to a string.
    let mut serialized = toml::to_string_pretty(&root)
        .map_err(|e| anyhow!("failed to re-serialize fixed plan TOML: {e}"))?;

    // -- Angle-bracket placeholder replacement --------------------------------
    // LLMs sometimes emit literal `<relevant-lib>`, `<crate>`, `<path>` etc.
    // instead of concrete values. Replace known placeholders with slug-derived
    // defaults so downstream tooling doesn't choke on them.
    let path_default = format!("crates/{slug}/src/lib.rs");
    let replacements: &[(&str, &str)] = &[
        ("<relevant-lib>", slug),
        ("<binary-crate>", slug),
        ("<crate>", slug),
        ("<module>", "lib"),
        ("<path>", &path_default),
        ("<file>", &path_default),
        ("<test_name>", "test_placeholder"),
    ];
    for &(placeholder, replacement) in replacements {
        if serialized.contains(placeholder) {
            tracing::info!(
                "prd plan: replaced placeholder '{}' with '{}'",
                placeholder,
                replacement
            );
            serialized = serialized.replace(placeholder, replacement);
        }
    }

    // If we did any replacements, verify the TOML still parses.
    if replacements.iter().any(|(ph, _)| toml_str.contains(*ph)) {
        let _: toml::Value = toml::from_str(&serialized)
            .map_err(|e| anyhow!("TOML became invalid after placeholder replacement: {e}"))?;
    }

    Ok(serialized)
}

/// Infer the crate name from a list of file paths.
/// Looks for `crates/<name>/` pattern. Returns `None` if no crate path found.
fn infer_crate_from_paths(files: &[String]) -> Option<String> {
    for f in files {
        // Match patterns like "crates/roko-cli/src/foo.rs"
        if let Some(rest) = f.strip_prefix("crates/") {
            if let Some(slash) = rest.find('/') {
                return Some(rest[..slash].to_string());
            }
        }
    }
    None
}

/// Build a `[[task.verify]]` TOML table value.
fn make_verify_entry(phase: &str, command: &str, fail_msg: &str) -> toml::Value {
    let mut table = toml::value::Table::new();
    table.insert("phase".to_string(), toml::Value::String(phase.to_string()));
    table.insert(
        "command".to_string(),
        toml::Value::String(command.to_string()),
    );
    table.insert(
        "fail_msg".to_string(),
        toml::Value::String(fail_msg.to_string()),
    );
    toml::Value::Table(table)
}

/// Slugify a title.
///
/// Output is capped at 200 bytes, truncated at the last hyphen before the
/// limit (word boundary) to avoid filesystem path-length errors.
pub fn slugify(title: &str) -> String {
    let slug: String = title
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-");

    const MAX_LEN: usize = 200;
    if slug.len() <= MAX_LEN {
        return slug;
    }

    // Truncate at the last hyphen before the limit (word boundary).
    if let Some(pos) = slug[..MAX_LEN].rfind('-') {
        slug[..pos].to_string()
    } else {
        slug[..MAX_LEN].to_string()
    }
}

#[must_use]
fn usize_to_u32_saturating(value: usize) -> u32 {
    u32::try_from(value).unwrap_or(u32::MAX)
}

fn prd_workdir(prd_path: &Path) -> Result<PathBuf> {
    prd_path
        .ancestors()
        .nth(4)
        .map(Path::to_path_buf)
        .ok_or_else(|| {
            anyhow!(
                "could not derive workdir from PRD path: {}",
                prd_path.display()
            )
        })
}

// ─── PRD artifact validation ───────────────────────────────────────

/// Severity of a PRD validation issue.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrdValidationSeverity {
    /// Blocking: `artifact_valid` is set to `false`.
    Error,
    /// Non-blocking: printed as a warning, does not affect `artifact_valid`.
    Warning,
}

/// A single validation issue found in a generated PRD.
#[derive(Debug, Clone)]
pub struct PrdValidationIssue {
    pub severity: PrdValidationSeverity,
    pub category: &'static str,
    pub message: String,
}

/// Outcome of post-generation PRD artifact validation.
///
/// `artifact_valid = false` means the PRD should not be accepted as a
/// successful generation outcome — learning gates should withhold rewards.
#[derive(Debug, Clone)]
pub struct PrdArtifactReport {
    /// Slug of the PRD being validated.
    pub slug: String,
    /// Whether the underlying agent process succeeded (exit 0).
    pub process_success: bool,
    /// Whether the artifact itself passes all blocking checks.
    ///
    /// Set to `false` when any [`PrdValidationSeverity::Error`] issue is found.
    pub artifact_valid: bool,
    /// All issues found during validation (errors + warnings).
    pub issues: Vec<PrdValidationIssue>,
}

impl PrdArtifactReport {
    fn new(slug: &str, process_success: bool) -> Self {
        Self {
            slug: slug.to_string(),
            process_success,
            artifact_valid: true,
            issues: Vec::new(),
        }
    }

    fn push(&mut self, issue: PrdValidationIssue) {
        if issue.severity == PrdValidationSeverity::Error {
            self.artifact_valid = false;
        }
        self.issues.push(issue);
    }

    /// Print all issues to stderr and a summary to stdout.
    pub fn print_summary(&self) {
        for issue in &self.issues {
            let label = match issue.severity {
                PrdValidationSeverity::Error => "ERROR",
                PrdValidationSeverity::Warning => "WARNING",
            };
            eprintln!("[{}] {}: {}", label, issue.category, issue.message);
        }
        let errors = self
            .issues
            .iter()
            .filter(|i| i.severity == PrdValidationSeverity::Error)
            .count();
        let warnings = self
            .issues
            .iter()
            .filter(|i| i.severity == PrdValidationSeverity::Warning)
            .count();
        if self.artifact_valid {
            println!("PRD artifact validation: PASSED ({warnings} warnings)");
        } else {
            println!("PRD artifact validation: FAILED ({errors} errors, {warnings} warnings)");
        }
    }
}

/// Extract a `## <heading>` markdown section body (case-insensitive).
///
/// Returns the lines between the matched heading and the next `##`-level
/// heading, joined as a single string. Returns `None` when the heading is
/// not found or the matched section is empty.
fn extract_prd_section(content: &str, heading: &str) -> Option<String> {
    let heading_lower = heading.to_lowercase();
    let mut in_section = false;
    let mut lines: Vec<&str> = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();
        let trimmed_lower = trimmed.to_lowercase();
        if trimmed_lower.starts_with("## ") {
            if in_section {
                // Reached the next ## heading — stop collecting.
                break;
            }
            let heading_text = trimmed_lower.trim_start_matches("## ").trim();
            if heading_text.starts_with(heading_lower.as_str()) {
                in_section = true;
                continue;
            }
        } else if in_section {
            lines.push(line);
        }
    }

    if lines.is_empty() {
        None
    } else {
        Some(lines.join("\n"))
    }
}

/// Extract all relative file paths referenced in the grounding section text.
///
/// A path is recognised when it starts with `crates/`, `src/`, `tests/`,
/// `plans/`, `apps/`, or `docs/` — i.e. plausible workspace-relative paths.
fn extract_referenced_paths(grounding_text: &str) -> Vec<String> {
    let prefixes = ["crates/", "src/", "tests/", "plans/", "apps/", "docs/"];
    let mut paths: Vec<String> = Vec::new();
    for line in grounding_text.lines() {
        for word in line.split_whitespace() {
            // Strip leading punctuation like `-`, `*`, `` ` ``, `(`.
            let word = word
                .trim_start_matches(['-', '*', '`', '(', '['])
                .trim_end_matches(['`', ')', ']', ',', '.']);
            if prefixes.iter().any(|p| word.starts_with(p)) {
                paths.push(word.to_string());
            }
        }
    }
    paths.sort();
    paths.dedup();
    paths
}

/// Check that a generated PRD contains the required `## Repository Grounding`
/// section.
///
/// **R4_B02**: This check is **blocking** — a missing section sets
/// `artifact_valid = false` on the returned report, which prevents learning
/// gates from treating the generation as successful.
///
/// Returns a [`PrdArtifactReport`] whose `artifact_valid` field reflects
/// whether the section was found.
#[must_use]
pub fn check_grounding_section(
    prd_content: &str,
    slug: &str,
    process_success: bool,
) -> PrdArtifactReport {
    let mut report = PrdArtifactReport::new(slug, process_success);
    let has_section = prd_content.lines().any(|line| {
        line.trim()
            .to_lowercase()
            .starts_with("## repository grounding")
    });
    if !has_section {
        report.push(PrdValidationIssue {
            severity: PrdValidationSeverity::Error,
            category: "missing_section",
            message: format!(
                "PRD '{}' is missing required '## Repository Grounding' section — PRD rejected",
                slug
            ),
        });
    }
    report
}

/// Validate the `## Repository Grounding` section of a generated PRD.
///
/// **R4_B02**: Missing grounding section is an `Error` (blocking).
/// **R4_B03**: Referenced source files that don't exist on disk are `Error`
///             (blocking). Duplicate crate proposals are also `Error`.
///
/// `workdir` is the workspace root (used to resolve relative source paths).
/// `workspace_members` is the list of crate directory names under `crates/`.
#[must_use]
pub fn validate_prd_grounding(
    prd_content: &str,
    slug: &str,
    workdir: &Path,
    workspace_members: &[String],
    process_success: bool,
) -> PrdArtifactReport {
    // Start with the blocking grounding-section check (R4_B02).
    let mut report = check_grounding_section(prd_content, slug, process_success);

    let Some(grounding_text) = extract_prd_section(prd_content, "repository grounding") else {
        // Section missing — already recorded as Error above; nothing more to validate.
        return report;
    };

    let text_lower = grounding_text.to_lowercase();

    // R4_B03a: "no existing crates" claim is suspicious when the workspace has members.
    if (text_lower.contains("no existing crates") || text_lower.contains("no relevant crates"))
        && !workspace_members.is_empty()
    {
        report.push(PrdValidationIssue {
            severity: PrdValidationSeverity::Warning,
            category: "false_negative",
            message: format!(
                "PRD claims no existing crates but workspace has {} crate(s): {}",
                workspace_members.len(),
                workspace_members
                    .iter()
                    .take(5)
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        });
    }

    // R4_B03b: "new crate X" proposals that duplicate existing workspace members.
    let new_crate_patterns = ["new crate: ", "new crate `", "create crate ", "add crate "];
    for line in grounding_text.lines() {
        let line_lower = line.to_lowercase();
        for pat in &new_crate_patterns {
            if let Some(after_offset) = line_lower.find(pat) {
                let rest = &line[after_offset + pat.len()..];
                let proposed = rest
                    .trim()
                    .trim_start_matches('`')
                    .trim_start_matches('"')
                    .split(|c: char| {
                        c.is_whitespace() || c == '`' || c == '"' || c == ',' || c == ')'
                    })
                    .next()
                    .unwrap_or("")
                    .trim();
                if !proposed.is_empty() && proposed.starts_with("roko-") {
                    if workspace_members
                        .iter()
                        .any(|m| m.to_lowercase() == proposed.to_lowercase())
                    {
                        report.push(PrdValidationIssue {
                            severity: PrdValidationSeverity::Error,
                            category: "duplicate_crate",
                            message: format!(
                                "PRD proposes creating crate '{}' which already exists in the workspace",
                                proposed
                            ),
                        });
                    }
                }
            }
        }
    }

    // R4_B03c: Referenced source files must exist in the workspace.
    let referenced_paths = extract_referenced_paths(&grounding_text);
    for rel_path in &referenced_paths {
        let abs_path = workdir.join(rel_path);
        if !abs_path.exists() {
            report.push(PrdValidationIssue {
                severity: PrdValidationSeverity::Error,
                category: "missing_file_ref",
                message: format!(
                    "PRD references '{}' in Repository Grounding but that path does not exist in the workspace",
                    rel_path
                ),
            });
        }
    }

    report
}

// ─── Tests ─────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// bug-d3c72e: a failed plan generation prints the head of the model's
    /// output. Cutting it at byte 2000 panicked when that byte fell inside a
    /// multi-byte character; the cut now falls on a char boundary.
    #[test]
    fn prd_failure_output_cuts_at_a_char_boundary() {
        let output = format!("{}é and more", "x".repeat(FAILURE_OUTPUT_CHARS - 1));
        assert!(!output.is_char_boundary(FAILURE_OUTPUT_CHARS));

        let printed = crate::run::truncate(&output, FAILURE_OUTPUT_CHARS);

        assert_eq!(printed.chars().count(), FAILURE_OUTPUT_CHARS);
        assert!(printed.ends_with('é'));
        let preview = crate::run::truncate(&output, AGENT_ERROR_PREVIEW_CHARS);
        assert_eq!(preview, "x".repeat(AGENT_ERROR_PREVIEW_CHARS));
        assert_eq!(crate::run::truncate(&output, RETRY_OUTPUT_CHARS), printed);
        assert_eq!(crate::run::truncate("short", FAILURE_OUTPUT_CHARS), "short");
    }

    #[test]
    fn slugify_basic() {
        assert_eq!(slugify("Agent Self-Improvement"), "agent-self-improvement");
        assert_eq!(slugify("  foo  BAR  "), "foo-bar");
        assert_eq!(slugify("hello"), "hello");
    }

    #[test]
    fn slugify_long_input_capped_at_200() {
        // Build a title that would produce a slug longer than 200 bytes.
        let long_title = "word ".repeat(100); // 500 chars → ~400+ byte slug
        let slug = slugify(&long_title);
        assert!(slug.len() <= 200, "slug len {} > 200", slug.len());
        // Should end at a word boundary (no trailing hyphen).
        assert!(!slug.ends_with('-'));
        assert!(!slug.is_empty());
    }

    #[test]
    fn slugify_exactly_200_not_truncated() {
        // Build a slug that's exactly 200 bytes — should not be truncated.
        let word = "abcdefghij"; // 10 chars
        // 19 words of 10 + 18 hyphens = 208; 18 words = 198; need filler
        let title = format!("{} ab", vec![word; 19].join(" ")); // yields 19*10+18+3 = 211 → truncated
        let slug = slugify(&title);
        assert!(slug.len() <= 200, "slug len {} > 200", slug.len());
    }

    #[test]
    fn slugify_short_input_unchanged() {
        let slug = slugify("small title");
        assert_eq!(slug, "small-title");
        assert!(slug.len() < 200);
    }

    #[test]
    fn parse_frontmatter() {
        let content = "---\nid: prd-test\ntitle: Test PRD\nstatus: draft\nversion: 2\ncoverage: 0.5\nplan_template = \"strict\"\n---\n\n# Test\n";
        let meta = PrdMeta::parse(content).unwrap();
        assert_eq!(meta.id, "prd-test");
        assert_eq!(meta.title, "Test PRD");
        assert_eq!(meta.status, "draft");
        assert_eq!(meta.version, 2);
        assert!((meta.coverage - 0.5).abs() < f64::EPSILON);
        assert_eq!(meta.plan_template.as_deref(), Some("strict"));
    }

    #[test]
    fn parse_frontmatter_colons_in_values() {
        let content =
            "---\nid: prd-wire\ntitle: \"Wire: the thing\"\nstatus: draft\n---\n\n# Body\n";
        let meta = PrdMeta::parse(content).unwrap();
        assert_eq!(meta.id, "prd-wire");
        assert_eq!(meta.title, "Wire: the thing");
        assert_eq!(meta.status, "draft");
    }

    #[test]
    fn parse_frontmatter_yaml_lists() {
        let content = "---\nid: prd-lists\ntitle: List Test\ntags: [\"infra\", \"prd\"]\ndepends_on:\n  - prd-alpha\n  - prd-beta\n---\n\n# Body\n";
        let meta = PrdMeta::parse(content).unwrap();
        assert_eq!(meta.tags, vec!["infra", "prd"]);
        assert_eq!(meta.depends_on, vec!["prd-alpha", "prd-beta"]);
    }

    #[test]
    fn parse_no_frontmatter() {
        assert!(PrdMeta::parse("# Just a heading").is_none());
    }

    #[test]
    fn generation_outcome_labels_distinguish_process_from_artifact() {
        let success = GenerationOutcome {
            process_success: true,
            artifact_valid: true,
            validation_report: None,
            spec_quality: None,
        };
        let partial = GenerationOutcome {
            process_success: true,
            artifact_valid: false,
            validation_report: None,
            spec_quality: None,
        };
        let failure = GenerationOutcome {
            process_success: false,
            artifact_valid: true,
            validation_report: None,
            spec_quality: None,
        };

        assert!(success.fully_successful());
        assert_eq!(success.status_label(), "success");
        assert!(!partial.fully_successful());
        assert_eq!(partial.status_label(), "partial_success");
        assert!(!failure.fully_successful());
        assert_eq!(failure.status_label(), "failure");
    }

    #[test]
    fn artifact_outcome_valid_requires_process_artifact_path_and_report() {
        let outcome = GenerationOutcome {
            process_success: true,
            artifact_valid: true,
            validation_report: Some(serde_json::json!({"totals": {"errors": 0}})),
            spec_quality: None,
        };
        let path = PathBuf::from(".roko/plans/demo");

        let artifact =
            ArtifactOutcome::from_generation_outcome("prd-plan", Some(path.clone()), &outcome);

        assert_eq!(
            artifact,
            ArtifactOutcome::Valid {
                artifact_type: "prd-plan".to_string(),
                path,
                report: serde_json::json!({"totals": {"errors": 0}}),
            }
        );
        assert!(artifact.is_valid());
    }

    #[test]
    fn artifact_outcome_invalid_is_not_success() {
        let outcome = GenerationOutcome {
            process_success: true,
            artifact_valid: false,
            validation_report: Some(serde_json::json!({"totals": {"errors": 2}})),
            spec_quality: None,
        };
        let path = PathBuf::from(".roko/plans/demo");

        let artifact =
            ArtifactOutcome::from_generation_outcome("prd-plan", Some(path.clone()), &outcome);

        assert_eq!(
            artifact,
            ArtifactOutcome::Invalid {
                artifact_type: "prd-plan".to_string(),
                path: Some(path),
                report: Some(serde_json::json!({"totals": {"errors": 2}})),
            }
        );
        assert!(!artifact.is_valid());
    }

    #[test]
    fn artifact_outcome_process_failure_is_not_produced() {
        let outcome = GenerationOutcome {
            process_success: false,
            artifact_valid: true,
            validation_report: Some(serde_json::json!({"ignored": true})),
            spec_quality: None,
        };

        let artifact = ArtifactOutcome::from_generation_outcome(
            "prd-plan",
            Some(PathBuf::from(".roko/plans/demo")),
            &outcome,
        );

        assert_eq!(
            artifact,
            ArtifactOutcome::NotProduced {
                artifact_type: "prd-plan".to_string(),
                reason: "generation process failed".to_string(),
            }
        );
        assert!(!artifact.is_valid());
    }

    #[test]
    fn artifact_outcome_validation_unavailable_is_not_success() {
        let outcome = GenerationOutcome {
            process_success: true,
            artifact_valid: true,
            validation_report: None,
            spec_quality: None,
        };
        let path = PathBuf::from(".roko/plans/demo");

        let artifact =
            ArtifactOutcome::from_generation_outcome("prd-plan", Some(path.clone()), &outcome);

        assert_eq!(
            artifact,
            ArtifactOutcome::ValidationUnavailable {
                artifact_type: "prd-plan".to_string(),
                path: Some(path),
                reason: "artifact validation report was not available".to_string(),
            }
        );
        assert!(!artifact.is_valid());
    }

    #[test]
    fn idea_appends() {
        let tmp = tempfile::tempdir().unwrap();
        ensure_dirs(tmp.path()).unwrap();
        cmd_idea(tmp.path(), "test idea 1", false).unwrap();
        cmd_idea(tmp.path(), "test idea 2", false).unwrap();
        let content = std::fs::read_to_string(ideas_path(tmp.path())).unwrap();
        assert!(content.contains("test idea 1"));
        assert!(content.contains("test idea 2"));
    }

    #[test]
    fn list_empty() {
        let tmp = tempfile::tempdir().unwrap();
        ensure_dirs(tmp.path()).unwrap();
        // Should not panic
        cmd_list(tmp.path(), false).unwrap();
    }

    #[tokio::test]
    async fn promote_moves_file() {
        let tmp = tempfile::tempdir().unwrap();
        ensure_dirs(tmp.path()).unwrap();
        let draft = drafts_dir(tmp.path()).join("test.md");
        std::fs::write(
            &draft,
            "---\nstatus: draft\nupdated: 2020-01-01\n---\n# Test\n\nThis PRD describes a real feature with substantive content.\n",
        )
        .unwrap();
        cmd_promote(tmp.path(), "test", false).await.unwrap();
        assert!(!draft.exists());
        let published = published_dir(tmp.path()).join("test.md");
        assert!(published.exists());
        let content = std::fs::read_to_string(&published).unwrap();
        assert!(content.contains("status: published"));
    }

    #[tokio::test]
    async fn promote_follow_on_generation_failure_is_non_fatal() {
        let tmp = tempfile::tempdir().unwrap();
        ensure_dirs(tmp.path()).unwrap();
        std::fs::write(tmp.path().join("roko.toml"), "[prd]\nauto_plan = true\n").unwrap();
        let prd_path = published_dir(tmp.path()).join("test.md");

        let outcome = maybe_generate_plan_after_promote_with(
            tmp.path(),
            "test".to_string(),
            prd_path.clone(),
            false,
            |_slug, _path, _dry_run| async move { Err(anyhow!("synthetic generation failure")) },
        )
        .await
        .unwrap();

        assert!(outcome.is_none());
    }

    #[tokio::test]
    async fn promote_rejects_empty_draft() {
        let tmp = tempfile::tempdir().unwrap();
        ensure_dirs(tmp.path()).unwrap();
        let draft = drafts_dir(tmp.path()).join("empty.md");
        std::fs::write(&draft, "---\nstatus: draft\n---\n# Empty\n").unwrap();
        let err = cmd_promote(tmp.path(), "empty", false).await.unwrap_err();
        assert!(
            err.to_string().contains("no substantive content"),
            "got: {err}"
        );
        assert!(draft.exists(), "draft should not be deleted on reject");
    }

    #[test]
    fn extract_fenced_block_finds_toml() {
        let text = "Some text\n```toml\n[tasks]\nname = \"test\"\n```\nMore text";
        let block = extract_fenced_block(text, "toml").unwrap();
        assert!(block.contains("[tasks]"));
        assert!(block.contains("name = \"test\""));
    }

    #[test]
    fn extract_fenced_block_returns_none_for_missing() {
        assert!(extract_fenced_block("no blocks here", "toml").is_none());
    }

    #[test]
    fn extract_fenced_block_skips_nested_fences() {
        // Agent output might include code samples with their own fences
        let text = "Here is the plan:\n```toml\n[[tasks]]\nid = \"T1\"\n\
                    # Example bash:\n```bash\necho hello\n```\n\
                    verify = \"cargo test\"\n```\nDone.";
        let block = extract_fenced_block(text, "toml").unwrap();
        assert!(block.contains("id = \"T1\""), "should contain task");
        assert!(block.contains("```bash"), "should include the nested fence");
    }

    #[test]
    fn extract_fenced_block_handles_angle_bracket_tag() {
        let text = "Output:\n```<plan.md>\n# My Plan\n\nSteps here.\n```\n";
        let block = extract_fenced_block(text, "plan.md").unwrap();
        assert!(block.contains("# My Plan"));
    }

    #[test]
    fn extract_fenced_block_returns_none_for_empty_block() {
        let text = "```toml\n\n```\n";
        assert!(extract_fenced_block(text, "toml").is_none());
    }

    #[test]
    fn extract_fenced_block_multiple_blocks_gets_first() {
        let text = "```toml\nfirst = true\n```\n\n```toml\nsecond = true\n```\n";
        let block = extract_fenced_block(text, "toml").unwrap();
        assert!(block.contains("first = true"));
        assert!(!block.contains("second = true"));
    }

    #[test]
    fn fallback_extracts_clean_toml() {
        let text = "[meta]\nplan = \"test\"\ntotal = 2\nstatus = \"ready\"\n\n\
                    [[task]]\nid = \"T1\"\ntitle = \"First\"\n";
        let block = extract_toml_content_fallback(text).unwrap();
        assert!(block.contains("[meta]"));
        assert!(block.contains("[[task]]"));
    }

    #[test]
    fn fallback_trims_trailing_markdown() {
        let text = "Here is your plan:\n\n\
                    [meta]\nplan = \"test\"\ntotal = 1\nstatus = \"ready\"\n\n\
                    [[task]]\nid = \"T1\"\ntitle = \"Do stuff\"\n\n\
                    ## Notes\n\nThis plan covers the main requirements.\n";
        let block = extract_toml_content_fallback(text).unwrap();
        assert!(block.contains("[meta]"));
        assert!(block.contains("[[task]]"));
        assert!(
            !block.contains("## Notes"),
            "should trim trailing markdown heading"
        );
        assert!(
            !block.contains("This plan covers"),
            "should trim trailing prose"
        );
    }

    #[test]
    fn fallback_trims_trailing_horizontal_rule() {
        let text = "[meta]\nplan = \"x\"\ntotal = 1\nstatus = \"ready\"\n\n\
                    [[task]]\nid = \"T1\"\ntitle = \"A\"\n\n\
                    ---\n\nSome explanation here.\n";
        let block = extract_toml_content_fallback(text).unwrap();
        assert!(!block.contains("---"), "should trim trailing hr");
        assert!(!block.contains("explanation"));
    }

    #[test]
    fn fallback_trims_trailing_prose_after_blank() {
        let text = "[meta]\nplan = \"x\"\ntotal = 1\nstatus = \"ready\"\n\n\
                    [[task]]\nid = \"T1\"\ntitle = \"A\"\n\n\
                    This plan implements the feature as described in the PRD.\n";
        let block = extract_toml_content_fallback(text).unwrap();
        assert!(
            !block.contains("This plan implements"),
            "should trim trailing prose paragraph"
        );
    }

    #[test]
    fn fallback_preserves_toml_comments() {
        let text = "[meta]\nplan = \"x\"\ntotal = 1\nstatus = \"ready\"\n\n\
                    # Task section\n[[task]]\nid = \"T1\"\ntitle = \"A\"\n";
        let block = extract_toml_content_fallback(text).unwrap();
        assert!(
            block.contains("# Task section"),
            "TOML comments should be kept"
        );
    }

    #[test]
    fn fallback_returns_none_without_task() {
        let text = "[meta]\nplan = \"x\"\n";
        assert!(extract_toml_content_fallback(text).is_none());
    }

    #[test]
    fn augment_generator_system_prompt_skips_empty_context() {
        let prompt = augment_generator_system_prompt("base prompt".to_string(), Some("   "));
        assert_eq!(prompt, "base prompt");
    }

    /// gap-2623b2: a planner with a large context window reads a
    /// 20,000-character PRD whole, with a file-read budget to match; a
    /// planner whose window is unknown keeps the old 8,000-character and
    /// 5-file caps, which are also every planner's floor.
    #[test]
    fn generator_prompt_keeps_a_long_prd_whole() {
        let temp = tempfile::tempdir().expect("tempdir");
        let mut prd = String::from("# Parser\n\n");
        while prd.len() < 20_000 {
            prd.push_str("REQ-001: the parser keeps every line of its input.\n");
        }
        prd.push_str("END-OF-PRD\n");
        let prd_path = temp.path().join("parser.md");
        std::fs::write(&prd_path, &prd).expect("write PRD");
        let source = ReadSource::read(PlanSource::Prd(&prd_path)).expect("read PRD");

        let mut models = IndexMap::new();
        models.insert(
            "frontier".to_string(),
            ModelProfile {
                slug: "frontier-1".to_string(),
                context_window: 200_000,
                ..ModelProfile::default()
            },
        );
        let prompt = |model: &str| {
            let budget = PlannerBudget::for_context_window(planner_context_window(&models, model));
            plan_task_prompt(&source, "parser", budget, "", "")
        };

        let frontier = prompt("frontier");
        assert!(frontier.contains(&prd), "the whole PRD reaches the planner");
        assert!(!frontier.contains("content truncated"));
        assert!(frontier.contains("read up to 25 codebase files"));
        assert_eq!(prompt("frontier-1"), frontier, "a slug finds its entry");

        let unknown = prompt("some-other-model");
        assert!(unknown.contains("[PRD content truncated at 8000 chars]"));
        assert!(!unknown.contains("END-OF-PRD"));
        assert!(unknown.contains("read up to 5 codebase files"));
        assert_eq!(
            PlannerBudget::for_context_window(Some(8_000)),
            PlannerBudget::SMALL
        );
    }

    /// Write the published PRD `widget` in `workdir`, and return its path.
    #[cfg(unix)]
    fn write_widget_prd(workdir: &Path) -> PathBuf {
        ensure_dirs(workdir).expect("PRD directories");
        let prd_path = published_dir(workdir).join("widget.md");
        std::fs::write(
            &prd_path,
            "---\nid: prd-widget\ntitle: Widget\nstatus: published\n---\n\n\
             # Widget\n\nAdd a widget module.\n",
        )
        .expect("write PRD");
        prd_path
    }

    /// The plan the fake planner answers with in the tests below.
    #[cfg(unix)]
    const WIDGET_PLAN: &str = "\
        [meta]\nplan = \"widget\"\ntotal = 1\ndone = 0\nstatus = \"ready\"\n\n\
        [[task]]\nid = \"T1\"\ntitle = \"Add the widget module\"\n\
        description = \"Create src/widget.rs.\"\nstatus = \"ready\"\n\
        role = \"implementer\"\ntier = \"focused\"\nfiles = [\"src/widget.rs\"]\n\
        depends_on = []\n\n[task.context]\nread_files = []\n\n\
        [[task.verify]]\nphase = \"check\"\ncommand = \"test -f src/widget.rs\"\n";

    /// Make `workdir`'s `planner` model a fake `claude_cli` script in `bin`
    /// that answers every call with `plan_toml` and logs it, and return the
    /// call log. The plan is never regenerated for its spec score.
    #[cfg(unix)]
    fn write_fake_planner(workdir: &Path, bin: &Path, plan_toml: &str) -> PathBuf {
        write_scripted_planner(
            workdir,
            bin,
            &[plan_toml],
            "allow_threshold = 0.0\nblock_threshold = 0.0\n",
        )
    }

    /// Make `workdir`'s `planner` model a fake `claude_cli` script in `bin`
    /// that answers its n-th call with `plans[n]` (the last plan after that;
    /// a plan that starts with a fence is the whole reply),
    /// keeps each call's arguments and input as `prompt-<n>.txt`, and logs
    /// it; return the call log. `spec_quality` is the body of the
    /// workspace's `[spec_quality]` section.
    #[cfg(unix)]
    fn write_scripted_planner(
        workdir: &Path,
        bin: &Path,
        plans: &[&str],
        spec_quality: &str,
    ) -> PathBuf {
        use std::os::unix::fs::PermissionsExt as _;

        for (index, plan_toml) in plans.iter().enumerate() {
            let text = if plan_toml.starts_with("```") {
                (*plan_toml).to_string()
            } else {
                format!("```toml\n{plan_toml}```\n")
            };
            std::fs::write(
                bin.join(format!("reply-{index}.jsonl")),
                format!(
                    "{}\n{}\n",
                    serde_json::json!({
                        "type": "content_block_delta",
                        "delta": {"text": text},
                    }),
                    serde_json::json!({
                        "type": "result",
                        "session_id": "planner",
                        "model": "claude-sonnet-4-6",
                        "total_cost_usd": 0.0,
                        "usage": {"input_tokens": 1, "output_tokens": 1},
                        "is_error": false,
                    }),
                ),
            )
            .expect("planner reply");
        }
        let last = bin.join(format!("reply-{}.jsonl", plans.len().saturating_sub(1)));
        // Work after a call (episode distillation) runs on the default model,
        // a second fake that answers nothing, so the log counts planner calls.
        let calls = bin.join("calls.log");
        let planner = bin.join("planner.sh");
        let background = bin.join("background.sh");
        let planner_body = format!(
            "n=$(cat '{calls}' 2>/dev/null | wc -l | tr -d ' ')\n\
             printf '%s\\n' \"$@\" > '{bin}/prompt-'\"$n\"'.txt'\n\
             cat >> '{bin}/prompt-'\"$n\"'.txt'\n\
             echo call >> '{calls}'\n\
             reply='{bin}/reply-'\"$n\"'.jsonl'\n\
             [ -f \"$reply\" ] || reply='{last}'\n\
             cat \"$reply\"",
            calls = calls.display(),
            bin = bin.display(),
            last = last.display()
        );
        for (script, body) in [
            (&planner, planner_body),
            (
                &background,
                format!("cat >/dev/null\nsed -n 2p '{}'", last.display()),
            ),
        ] {
            std::fs::write(script, format!("#!/bin/sh\nset -eu\n{body}\n"))
                .expect("fake provider script");
            std::fs::set_permissions(script, std::fs::Permissions::from_mode(0o755))
                .expect("make the script executable");
        }
        std::fs::write(
            workdir.join("roko.toml"),
            format!(
                "[agent]\ndefault_model = \"background\"\ncommand = {background:?}\n\
                 bare_mode = false\n\n\
                 [providers.fake]\nkind = \"claude_cli\"\ncommand = {planner:?}\n\n\
                 [providers.quiet]\nkind = \"claude_cli\"\ncommand = {background:?}\n\n\
                 [models.planner]\nprovider = \"fake\"\nslug = \"claude-sonnet-4-6\"\n\
                 context_window = 200000\n\n\
                 [models.background]\nprovider = \"quiet\"\nslug = \"claude-sonnet-4-6\"\n\n\
                 [spec_quality]\n{spec_quality}",
                planner = planner.display().to_string(),
                background = background.display().to_string()
            ),
        )
        .expect("roko.toml");
        calls
    }

    /// 3218: generation scores its plan. A stub planner whose first plan has
    /// a verify step that can never fail and whose second is clean writes
    /// the clean plan after exactly two calls, and the retry names the hard
    /// fail. A stub whose plans stay weak is asked once more and no further,
    /// and the outcome records the scores.
    #[cfg(unix)]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn generation_regenerates_once_on_a_spec_hard_fail() {
        let vacuous = WIDGET_PLAN.replace(
            "\"test -f src/widget.rs\"",
            "\"test -f src/widget.rs || true\"",
        );
        assert_ne!(vacuous, WIDGET_PLAN);

        let temp = tempfile::tempdir().expect("tempdir");
        let workdir = temp.path();
        let prd_path = write_widget_prd(workdir);
        let bin = tempfile::tempdir().expect("tempdir");
        let calls = write_scripted_planner(
            workdir,
            bin.path(),
            &[&vacuous, WIDGET_PLAN],
            "allow_threshold = 0.0\nblock_threshold = 0.0\n",
        );
        let request = PlanRequest {
            model: Some("planner"),
            ..PlanRequest::new(PlanSource::Prd(&prd_path), "widget", workdir)
        };
        let (_, outcome) = generate_plan(request)
            .await
            .expect("generate the widget plan");
        let written = std::fs::read_to_string(workdir.join("plans/widget/tasks.toml"))
            .expect("the widget plan");
        assert!(!written.contains("|| true"), "{written}");
        let log = std::fs::read_to_string(&calls).expect("planner call log");
        assert_eq!(
            log.lines().count(),
            2,
            "the vacuous plan, then the clean one"
        );
        let retry = std::fs::read_to_string(bin.path().join("prompt-1.txt")).expect("retry");
        assert!(
            retry.contains("task T1 HF2: step 1: ends in `|| true`"),
            "{retry}"
        );
        let scored = outcome.spec_quality.expect("the plan was scored");
        assert!(!scored.regenerated, "{scored:?}");
        assert_eq!(scored.bands.values().sum::<usize>(), 1, "{scored:?}");

        let temp = tempfile::tempdir().expect("tempdir");
        let workdir = temp.path();
        let prd_path = write_widget_prd(workdir);
        let bin = tempfile::tempdir().expect("tempdir");
        let calls = write_scripted_planner(
            workdir,
            bin.path(),
            &[WIDGET_PLAN],
            "allow_threshold = 100.0\n",
        );
        let request = PlanRequest {
            model: Some("planner"),
            ..PlanRequest::new(PlanSource::Prd(&prd_path), "widget", workdir)
        };
        let (_, outcome) = generate_plan(request)
            .await
            .expect("generate the widget plan");
        let log = std::fs::read_to_string(&calls).expect("planner call log");
        assert_eq!(log.lines().count(), 2, "one plan, then one regeneration");
        let regenerate = std::fs::read_to_string(bin.path().join("prompt-1.txt")).expect("ask");
        assert!(
            regenerate.contains("scored low on spec quality"),
            "{regenerate}"
        );
        let scored = outcome.spec_quality.expect("the plan was scored");
        assert!(scored.regenerated, "{scored:?}");
        assert!(
            scored.min <= scored.mean && scored.mean < 100.0,
            "{scored:?}"
        );
    }

    /// 3221: the planner can write its acceptance tests into the plan. A
    /// reply with a tasks.toml block and an `accept:accept/test_slug.py`
    /// block writes both; a `[task.accept]` src that the reply does not
    /// emit sends the plan back to the planner, which emits it next time.
    #[cfg(unix)]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn generation_writes_planner_accept_tests_into_the_plan() {
        const ACCEPT_PLAN: &str = "\
            [meta]\nplan = \"widget\"\ntotal = 1\ndone = 0\nstatus = \"ready\"\n\n\
            [[task]]\nid = \"T1\"\ntitle = \"Add the slug helper\"\n\
            description = \"Create src/slug.py with slugify().\"\nstatus = \"ready\"\n\
            role = \"implementer\"\ntier = \"focused\"\nfiles = [\"src/slug.py\"]\n\
            depends_on = []\n\n[task.context]\nread_files = []\n\n\
            [task.accept]\nfiles = [{ src = \"accept/test_slug.py\", \
            dest = \"tests/test_slug.py\", runner = \"python3 -m unittest tests.test_slug\", \
            count = 1 }]\n\n\
            [[task.verify]]\nphase = \"check\"\ncommand = \"test -f src/slug.py\"\n";
        const TEST: &str = "import unittest\n\n\
            class Slug(unittest.TestCase):\n    def test_slug(self):\n        \
            from src.slug import slugify\n        \
            self.assertEqual(slugify(\"A b\"), \"a-b\")\n";
        let with_test =
            format!("```toml\n{ACCEPT_PLAN}```\n\n```accept:accept/test_slug.py\n{TEST}```\n");

        for replies in [
            vec![with_test.as_str()],
            vec![ACCEPT_PLAN, with_test.as_str()],
        ] {
            let temp = tempfile::tempdir().expect("tempdir");
            let workdir = temp.path();
            let prd_path = write_widget_prd(workdir);
            let bin = tempfile::tempdir().expect("tempdir");
            let calls = write_scripted_planner(
                workdir,
                bin.path(),
                &replies,
                "allow_threshold = 0.0\nblock_threshold = 0.0\n",
            );
            let request = PlanRequest {
                model: Some("planner"),
                ..PlanRequest::new(PlanSource::Prd(&prd_path), "widget", workdir)
            };
            generate_plan(request)
                .await
                .expect("generate the widget plan");
            let plan_dir = workdir.join("plans/widget");
            let tasks = std::fs::read_to_string(plan_dir.join("tasks.toml")).expect("tasks.toml");
            assert!(tasks.contains("accept/test_slug.py"), "{tasks}");
            let test = std::fs::read_to_string(plan_dir.join("accept/test_slug.py"))
                .expect("the accept test beside tasks.toml");
            assert_eq!(test, TEST);
            let log = std::fs::read_to_string(&calls).expect("planner call log");
            assert_eq!(log.lines().count(), replies.len(), "{replies:?}");
            if replies.len() == 2 {
                let retry = std::fs::read_to_string(bin.path().join("prompt-1.txt"))
                    .expect("the retry prompt");
                assert!(
                    retry.contains("src `accept/test_slug.py` is missing"),
                    "{retry}"
                );
            }
        }
    }

    /// bug-a5cd6b: `roko prd plan` writes the plan it was asked for and no
    /// other. An old-format plan and a generated plan, which names no model,
    /// keep their tasks.toml byte for byte, and the planner is called once.
    #[cfg(unix)]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_prd_plan_does_not_regenerate_other_plans() {
        let temp = tempfile::tempdir().expect("tempdir");
        let workdir = temp.path();
        let prd_path = write_widget_prd(workdir);

        // Two plans are already in plans/: one in the old format, and one as
        // the generator writes it, without a model_hint. Each has a plan.md a
        // regeneration could start from.
        let plans = workdir.join("plans");
        let old_toml = "[meta]\nplan = \"old\"\ntotal = 1\nstatus = \"ready\"\n\n\
                        [[task]]\nid = \"T1\"\ntitle = \"An old task\"\nstatus = \"ready\"\n";
        let hintless_toml = "[meta]\nplan = \"hintless\"\ntotal = 1\nstatus = \"ready\"\n\n\
             [[task]]\nid = \"T1\"\ntitle = \"A generated task\"\nstatus = \"ready\"\n\
             role = \"implementer\"\ntier = \"focused\"\nfiles = [\"src/hintless.rs\"]\n\
             depends_on = []\n\n[task.context]\nread_files = []\n\n\
             [[task.verify]]\nphase = \"check\"\ncommand = \"test -f src/hintless.rs\"\n";
        for (name, tasks) in [("old", old_toml), ("hintless", hintless_toml)] {
            let dir = plans.join(name);
            std::fs::create_dir_all(&dir).expect("plan directory");
            std::fs::write(dir.join("tasks.toml"), tasks).expect("tasks.toml");
            std::fs::write(
                dir.join("plan.md"),
                format!("---\nplan: {name}\n---\n# {name}\n"),
            )
            .expect("plan.md");
        }

        let bin = tempfile::tempdir().expect("tempdir");
        let calls = write_fake_planner(workdir, bin.path(), WIDGET_PLAN);

        generate_plan_from_prd_with_model("widget", &prd_path, false, Some("planner"))
            .await
            .expect("prd plan writes the widget plan");

        let widget = std::fs::read_to_string(plans.join("widget").join("tasks.toml"))
            .expect("the widget plan");
        let widget = TasksFile::parse_str(&widget).expect("parse the widget plan");
        assert_eq!(widget.tasks.len(), 1);
        for (name, tasks) in [("old", old_toml), ("hintless", hintless_toml)] {
            let after =
                std::fs::read_to_string(plans.join(name).join("tasks.toml")).expect("tasks.toml");
            assert_eq!(after, tasks, "plans/{name} was rewritten");
        }
        let calls = std::fs::read_to_string(&calls).expect("planner call log");
        assert_eq!(
            calls.lines().count(),
            1,
            "one planner call: the widget plan"
        );
        // A plan that names no model is modern: only `old` counts as old.
        assert_eq!(old_format_plan_dirs(&plans), [plans.join("old")]);
    }

    /// bug-2d06bf: a broken plan beside the generated one does not fail the
    /// generation: generate_plan validates only the plan it wrote.
    #[cfg(unix)]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn generate_plan_validates_only_the_plan_it_wrote() {
        let temp = tempfile::tempdir().expect("tempdir");
        let workdir = temp.path();
        let prd_path = write_widget_prd(workdir);
        let broken = workdir.join("plans").join("broken");
        std::fs::create_dir_all(&broken).expect("plan directory");
        std::fs::write(broken.join("tasks.toml"), "[meta\nplan = ").expect("tasks.toml");
        let bin = tempfile::tempdir().expect("tempdir");
        write_fake_planner(workdir, bin.path(), WIDGET_PLAN);

        let request = PlanRequest {
            model: Some("planner"),
            ..PlanRequest::new(PlanSource::Prd(&prd_path), "widget", workdir)
        };
        let (_, outcome) = generate_plan(request)
            .await
            .expect("generate the widget plan");

        let report = outcome.validation_report.expect("validation report");
        assert_eq!(report["totals"]["plans_checked"], 1, "{report}");
        // The report lists only plans with findings: never the broken sibling.
        let plans = report["plans"].as_array().expect("plans");
        assert!(
            plans.iter().all(|plan| plan["plan_id"] == "widget"),
            "{report}"
        );
        assert!(!report.to_string().contains("PLAN_001"), "{report}");
    }

    #[test]
    fn augment_generator_system_prompt_includes_failure_context() {
        let prompt = augment_generator_system_prompt(
            "base prompt".to_string(),
            Some("task_id = \"demo\"\nreason = \"gate failure\""),
        );
        assert!(prompt.starts_with("base prompt"));
        assert!(prompt.contains("## Failure context for replanning"));
        assert!(prompt.contains("task_id = \"demo\""));
        assert!(prompt.contains("gate failure"));
        assert!(prompt.contains("Do not reproduce the same task shape."));
    }

    #[test]
    fn new_draft_frontmatter_valid() {
        let fm = new_draft_frontmatter("test-prd", "Test PRD");
        assert!(fm.starts_with("---\n"));
        assert!(fm.contains("id: prd-test-prd"));
        assert!(fm.contains("title: Test PRD"));
        assert!(fm.contains("status: draft"));
    }

    #[test]
    fn replace_in_frontmatter_only_affects_frontmatter() {
        let content = "---\nstatus: draft\ntitle: Demo\n---\n\n# Body\n\nThe status: draft is mentioned here too.\n";
        let result = replace_in_frontmatter(content, "status: draft", "status: published");
        assert!(result.contains("status: published"));
        assert!(
            result.contains("The status: draft is mentioned here too."),
            "body should be untouched"
        );
    }

    #[test]
    fn has_substantive_markdown_content_ignores_headers_only() {
        let content = "---\nid: demo\n---\n# Title\n\n## Overview\n";
        assert!(!has_substantive_markdown_content(content));
    }

    #[test]
    fn has_substantive_markdown_content_detects_body_text() {
        let content = "---\nid: demo\n---\n# Title\n\nActual requirement text.\n";
        assert!(has_substantive_markdown_content(content));
    }

    #[test]
    fn materialize_agent_markdown_output_strips_fences() {
        let output = "```markdown\n---\nid: demo\n---\n# Demo\n\nBody\n```";
        let rendered = materialize_agent_markdown_output(output, None).expect("rendered");
        assert!(rendered.starts_with("---"));
        assert!(rendered.contains("Body"));
        assert!(!rendered.contains("```"));
    }

    #[test]
    fn materialize_agent_markdown_output_prepends_scaffold_when_frontmatter_missing() {
        let rendered = materialize_agent_markdown_output("Body only", Some("---\nid: demo\n---"))
            .expect("rendered");
        assert!(rendered.starts_with("---\nid: demo\n---"));
        assert!(rendered.contains("Body only"));
    }

    // ─── R4_B02 / R4_B03 validation tests ─────────────────────────

    #[test]
    fn check_grounding_section_rejects_missing_section() {
        let content = "---\nid: prd-demo\n---\n# Demo\n\n## Requirements\n\nSome req.\n";
        let report = check_grounding_section(content, "demo", true);
        assert!(
            !report.artifact_valid,
            "missing section must set artifact_valid=false"
        );
        assert!(
            report
                .issues
                .iter()
                .any(|i| i.severity == PrdValidationSeverity::Error
                    && i.category == "missing_section"),
            "expected missing_section error"
        );
    }

    #[test]
    fn check_grounding_section_accepts_present_section() {
        let content = "---\nid: prd-demo\n---\n# Demo\n\n## Repository Grounding\n\nExisting crates: roko-core.\n";
        let report = check_grounding_section(content, "demo", true);
        assert!(report.artifact_valid, "present section must pass");
        assert!(report.issues.is_empty(), "no issues expected");
    }

    #[test]
    fn check_grounding_section_case_insensitive() {
        let content = "# Demo\n\n## REPOSITORY GROUNDING\n\nContent here.\n";
        let report = check_grounding_section(content, "demo", true);
        assert!(report.artifact_valid, "case-insensitive match must pass");
    }

    #[test]
    fn validate_prd_grounding_flags_no_existing_crates_claim() {
        let content = "# Demo\n\n## Repository Grounding\n\nNo existing crates are relevant.\n";
        let members = vec!["roko-core".to_string(), "roko-agent".to_string()];
        let report = validate_prd_grounding(content, "demo", Path::new("/tmp"), &members, true);
        assert!(
            report.issues.iter().any(|i| i.category == "false_negative"),
            "expected false_negative warning"
        );
        // Warning only — still valid
        assert!(report.artifact_valid);
    }

    #[test]
    fn validate_prd_grounding_blocks_duplicate_crate_proposal() {
        let content = "# Demo\n\n## Repository Grounding\n\nnew crate: roko-core\n";
        let members = vec!["roko-core".to_string()];
        let report = validate_prd_grounding(content, "demo", Path::new("/tmp"), &members, true);
        assert!(
            !report.artifact_valid,
            "duplicate crate must set artifact_valid=false"
        );
        assert!(
            report
                .issues
                .iter()
                .any(|i| i.category == "duplicate_crate"
                    && i.severity == PrdValidationSeverity::Error)
        );
    }

    #[test]
    fn validate_prd_grounding_blocks_nonexistent_file_reference() {
        let tmp = tempfile::tempdir().unwrap();
        let content = "# Demo\n\n## Repository Grounding\n\n**Source files**:\n- crates/roko-cli/src/no_such_file.rs — does not exist\n";
        let report = validate_prd_grounding(content, "demo", tmp.path(), &[], true);
        assert!(
            !report.artifact_valid,
            "nonexistent file ref must set artifact_valid=false"
        );
        assert!(report.issues.iter().any(
            |i| i.category == "missing_file_ref" && i.severity == PrdValidationSeverity::Error
        ));
    }

    #[test]
    fn validate_prd_grounding_allows_existing_file_reference() {
        let tmp = tempfile::tempdir().unwrap();
        // Create the file so it "exists"
        let file_path = tmp.path().join("crates").join("roko-cli").join("src");
        std::fs::create_dir_all(&file_path).unwrap();
        std::fs::write(file_path.join("prd.rs"), "// prd").unwrap();
        let content = "# Demo\n\n## Repository Grounding\n\n**Source files**:\n- crates/roko-cli/src/prd.rs — PRD logic\n";
        let report = validate_prd_grounding(content, "demo", tmp.path(), &[], true);
        assert!(report.artifact_valid, "existing file ref must pass");
        assert!(
            !report
                .issues
                .iter()
                .any(|i| i.category == "missing_file_ref"),
            "no missing_file_ref issues expected"
        );
    }

    #[test]
    fn extract_prd_section_returns_none_when_missing() {
        let content = "# Title\n\n## Overview\n\nSome text.\n";
        assert!(extract_prd_section(content, "repository grounding").is_none());
    }

    #[test]
    fn extract_prd_section_extracts_body() {
        let content =
            "# Title\n\n## Repository Grounding\n\nCrates: roko-core.\n\n## References\n\nRef 1.\n";
        let body = extract_prd_section(content, "repository grounding").unwrap();
        assert!(body.contains("Crates: roko-core."), "body: {body}");
        assert!(!body.contains("## References"), "must stop at next heading");
    }

    #[test]
    fn extract_referenced_paths_finds_crate_paths() {
        let text = "- crates/roko-cli/src/prd.rs — PRD logic\n- crates/roko-core/src/lib.rs";
        let paths = extract_referenced_paths(text);
        assert!(paths.contains(&"crates/roko-cli/src/prd.rs".to_string()));
        assert!(paths.contains(&"crates/roko-core/src/lib.rs".to_string()));
    }

    // ---- validate_and_fix_generated_plan tests -------------------------------

    fn empty_models() -> IndexMap<String, roko_core::config::schema::ModelProfile> {
        IndexMap::new()
    }

    fn sample_models() -> IndexMap<String, roko_core::config::schema::ModelProfile> {
        // Build from TOML to avoid enumerating every ModelProfile field.
        let toml_str = r#"
[models.claude-sonnet-4-6]
provider = "anthropic"
slug = "claude-sonnet-4-6"

[models.claude-haiku-4-5]
provider = "anthropic"
slug = "claude-haiku-4-5"
"#;
        let cfg: roko_core::config::schema::RokoConfig =
            roko_core::config::schema::RokoConfig::from_toml(toml_str).unwrap();
        cfg.models
    }

    #[test]
    fn validate_valid_plan_passes() {
        let toml = r#"
[meta]
plan = "my-plan"
total = 2
status = "pending"

[[task]]
id = "T1"
title = "First task"
status = "pending"
role = "implementer"
tier = "focused"

[[task.verify]]
phase = "test"
command = "cargo test"

[[task]]
id = "T2"
title = "Second task"
status = "pending"
role = "implementer"
tier = "mechanical"
depends_on = ["T1"]
"#;
        let result = validate_and_fix_generated_plan(
            toml,
            "my-plan",
            &empty_models(),
            None,
            &LadderConfig::default(),
        );
        assert!(result.is_ok(), "expected Ok, got: {result:?}");
    }

    #[test]
    fn validate_fixes_truncated_slug() {
        let toml = r#"
[meta]
plan = "my-pl"
total = 1
status = "pending"

[[task]]
id = "T1"
title = "Do thing"
status = "pending"
role = "implementer"
tier = "focused"
"#;
        let result = validate_and_fix_generated_plan(
            toml,
            "my-plan",
            &empty_models(),
            None,
            &LadderConfig::default(),
        )
        .unwrap();
        let parsed: toml::Value = toml::from_str(&result).unwrap();
        assert_eq!(
            parsed["meta"]["plan"].as_str().unwrap(),
            "my-plan",
            "slug should be corrected"
        );
    }

    #[test]
    fn validate_fixes_wrong_slug() {
        let toml = r#"
[meta]
plan = "wrong-slug"
total = 1
status = "pending"

[[task]]
id = "T1"
title = "Do thing"
status = "pending"
role = "implementer"
tier = "focused"
"#;
        let result = validate_and_fix_generated_plan(
            toml,
            "correct-slug",
            &empty_models(),
            None,
            &LadderConfig::default(),
        )
        .unwrap();
        let parsed: toml::Value = toml::from_str(&result).unwrap();
        assert_eq!(parsed["meta"]["plan"].as_str().unwrap(), "correct-slug");
    }

    #[test]
    fn validate_fixes_verify_field_typo() {
        let toml = r#"
[meta]
plan = "test"
total = 1
status = "pending"

[[task]]
id = "T1"
title = "Do thing"
status = "pending"
role = "implementer"
tier = "focused"

[[task.verify]]
pha = "test"
command = "cargo test"
"#;
        let result = validate_and_fix_generated_plan(
            toml,
            "test",
            &empty_models(),
            None,
            &LadderConfig::default(),
        )
        .unwrap();
        let parsed: toml::Value = toml::from_str(&result).unwrap();
        let verify = parsed["task"][0]["verify"][0].as_table().unwrap();
        assert!(
            verify.contains_key("phase"),
            "pha should be corrected to phase"
        );
        assert!(!verify.contains_key("pha"), "pha should be removed");
    }

    #[test]
    fn validate_removes_unknown_model_hint() {
        let toml = r#"
[meta]
plan = "test"
total = 1
status = "pending"

[[task]]
id = "T1"
title = "Do thing"
status = "pending"
role = "implementer"
tier = "focused"
model_hint = "gpt-nonexistent"
"#;
        let models = sample_models();
        let result = validate_and_fix_generated_plan(
            toml,
            "test",
            &models,
            Some("claude-sonnet-4-6"),
            &LadderConfig::default(),
        )
        .unwrap();
        let parsed: toml::Value = toml::from_str(&result).unwrap();
        assert!(
            parsed["task"][0].get("model_hint").is_none(),
            "unknown model_hint should be removed so runtime selects"
        );
    }

    #[test]
    fn validate_removes_model_alias_hint() {
        let toml = r#"
[meta]
plan = "test"
total = 1
status = "pending"

[[task]]
id = "T1"
title = "Do thing"
status = "pending"
role = "implementer"
tier = "focused"
model_hint = "haiku"
"#;
        let models = sample_models();
        let result =
            validate_and_fix_generated_plan(toml, "test", &models, None, &LadderConfig::default())
                .unwrap();
        let parsed: toml::Value = toml::from_str(&result).unwrap();
        assert!(
            parsed["task"][0].get("model_hint").is_none(),
            "model_hint aliases should be removed so runtime selects"
        );
    }

    /// gap-dbf2a6: a generated plan keeps a `rung` hint that names one of
    /// the task's ladder rungs, drops any other, and still loses
    /// `model_hint`.
    #[test]
    fn generated_plan_keeps_its_rung_hint() {
        let toml = r#"
[meta]
plan = "test"
total = 3
status = "pending"

[[task]]
id = "T1"
title = "Rework the parser's error recovery"
status = "pending"
role = "implementer"
tier = "mechanical"
rung = "strong"
model_hint = "claude-opus-4-6"

[[task]]
id = "T2"
title = "Rename a helper"
status = "pending"
role = "implementer"
tier = "mechanical"
rung = "stronk"

[[task]]
id = "T3"
title = "Document the parser"
status = "pending"
role = "scribe"
tier = "focused"
rung = "docs"
"#;
        let mut ladder = LadderConfig::default();
        ladder
            .roles
            .push(roko_core::config::routing::LadderRoleConfig {
                role: "scribe".to_string(),
                rungs: Some(vec![roko_core::config::routing::LadderRung {
                    name: "docs".to_string(),
                    model: "gpt-4o-mini".to_string(),
                }]),
                ..Default::default()
            });
        let result =
            validate_and_fix_generated_plan(toml, "test", &empty_models(), None, &ladder).unwrap();
        let plan = TasksFile::parse_str(&result).expect("the fixed plan parses");
        let rungs: Vec<Option<&str>> = plan
            .tasks
            .iter()
            .map(|task| task.hints.rung.as_deref())
            .collect();
        assert_eq!(rungs, [Some("strong"), None, Some("docs")]);
        assert_eq!(plan.tasks[0].model_hint, None);
    }

    #[test]
    fn validate_rejects_missing_meta() {
        let toml = r#"
[[task]]
id = "T1"
title = "Do thing"
status = "pending"
role = "implementer"
tier = "focused"
"#;
        let result = validate_and_fix_generated_plan(
            toml,
            "test",
            &empty_models(),
            None,
            &LadderConfig::default(),
        );
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("[meta] section is missing"), "msg: {msg}");
    }

    #[test]
    fn validate_rejects_missing_task_array() {
        let toml = r#"
[meta]
plan = "test"
total = 0
status = "pending"
"#;
        let result = validate_and_fix_generated_plan(
            toml,
            "test",
            &empty_models(),
            None,
            &LadderConfig::default(),
        );
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("[[task]] array is missing"), "msg: {msg}");
    }

    #[test]
    fn validate_rejects_missing_required_task_fields() {
        let toml = r#"
[meta]
plan = "test"
total = 1
status = "pending"

[[task]]
id = "T1"
"#;
        let result = validate_and_fix_generated_plan(
            toml,
            "test",
            &empty_models(),
            None,
            &LadderConfig::default(),
        );
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("missing required field 'title'"), "msg: {msg}");
        assert!(
            msg.contains("missing required field 'status'"),
            "msg: {msg}"
        );
        assert!(msg.contains("missing required field 'role'"), "msg: {msg}");
        assert!(msg.contains("missing required field 'tier'"), "msg: {msg}");
    }

    #[test]
    fn validate_rejects_invalid_toml_syntax() {
        let toml = "this is not valid toml {{{}}}";
        let result = validate_and_fix_generated_plan(
            toml,
            "test",
            &empty_models(),
            None,
            &LadderConfig::default(),
        );
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("invalid TOML"), "msg: {msg}");
    }

    #[test]
    fn validate_fixes_task_field_typo() {
        let toml = r#"
[meta]
plan = "test"
total = 1
status = "pending"

[[task]]
id = "T1"
title = "Do thing"
stat = "pending"
role = "implementer"
tier = "focused"
"#;
        let result = validate_and_fix_generated_plan(
            toml,
            "test",
            &empty_models(),
            None,
            &LadderConfig::default(),
        )
        .unwrap();
        let parsed: toml::Value = toml::from_str(&result).unwrap();
        let task = parsed["task"][0].as_table().unwrap();
        assert!(
            task.contains_key("status"),
            "stat should be corrected to status"
        );
        assert!(!task.contains_key("stat"), "stat should be removed");
    }

    #[test]
    fn validate_fixes_placeholder_crate_names() {
        let toml = r#"
[meta]
plan = "btc-funding-alert-cli"
total = 1
status = "pending"

[[task]]
id = "T1"
title = "Set up crate structure"
status = "pending"
role = "implementer"
tier = "focused"
files = ["crates/<relevant-lib>/src/lib.rs"]

[[task.verify]]
phase = "build"
command = "cargo check -p <crate>"
"#;
        let result = validate_and_fix_generated_plan(
            toml,
            "btc-funding-alert-cli",
            &empty_models(),
            None,
            &LadderConfig::default(),
        )
        .unwrap();
        assert!(
            !result.contains("<relevant-lib>"),
            "placeholder <relevant-lib> should be replaced"
        );
        assert!(
            !result.contains("<crate>"),
            "placeholder <crate> should be replaced"
        );
        assert!(
            result.contains("btc-funding-alert-cli"),
            "slug should appear in output"
        );
        // Verify it's still valid TOML.
        let _parsed: toml::Value = toml::from_str(&result).unwrap();
    }

    #[test]
    fn validate_fixes_placeholder_in_verify_command() {
        let toml = r#"
[meta]
plan = "my-cool-tool"
total = 1
status = "pending"

[[task]]
id = "T1"
title = "Implement module"
status = "pending"
role = "implementer"
tier = "focused"
files = ["crates/<binary-crate>/src/<module>.rs"]

[[task.verify]]
phase = "build"
command = "cargo check -p <binary-crate>"

[[task.verify]]
phase = "test"
command = "cargo test -p <crate> -- <test_name>"
"#;
        let result = validate_and_fix_generated_plan(
            toml,
            "my-cool-tool",
            &empty_models(),
            None,
            &LadderConfig::default(),
        )
        .unwrap();
        // <binary-crate> and <crate> replaced with slug.
        assert!(
            !result.contains("<binary-crate>"),
            "placeholder <binary-crate> should be replaced"
        );
        assert!(
            !result.contains("<crate>"),
            "placeholder <crate> should be replaced"
        );
        assert!(
            !result.contains("<module>"),
            "placeholder <module> should be replaced"
        );
        assert!(
            !result.contains("<test_name>"),
            "placeholder <test_name> should be replaced"
        );
        assert!(
            result.contains("cargo check -p my-cool-tool"),
            "verify command should contain slug: {result}"
        );
        assert!(
            result.contains("cargo test -p my-cool-tool"),
            "verify command should contain slug: {result}"
        );
        // Verify it's still valid TOML.
        let _parsed: toml::Value = toml::from_str(&result).unwrap();
    }

    // ---- next_tier_model tests ----

    /// `[models.*]` entries from `(key, slug)` pairs.
    fn models_with(entries: &[(&str, &str)]) -> IndexMap<String, ModelProfile> {
        entries
            .iter()
            .map(|(key, slug)| {
                let profile = ModelProfile {
                    slug: (*slug).to_string(),
                    ..ModelProfile::default()
                };
                ((*key).to_string(), profile)
            })
            .collect()
    }

    /// `[models.*]` entries keyed by their own slug.
    fn models_for(slugs: &[&str]) -> IndexMap<String, ModelProfile> {
        let entries: Vec<(&str, &str)> = slugs.iter().map(|slug| (*slug, *slug)).collect();
        models_with(&entries)
    }

    #[test]
    fn next_tier_model_escalates_with_empty_configured_set() {
        // No configured models = no filtering (backward compat).
        let empty_tier = HashMap::new();
        let no_models = IndexMap::new();
        assert_eq!(
            next_tier_model(Some("claude-haiku-4-5"), &empty_tier, &no_models),
            Some("claude-sonnet-4-6".to_string()),
        );
        assert_eq!(
            next_tier_model(Some("claude-sonnet-4-6"), &empty_tier, &no_models),
            Some("claude-opus-4-6".to_string()),
        );
    }

    #[test]
    fn next_tier_model_skips_unconfigured() {
        let empty_tier = HashMap::new();
        // Only sonnet is configured — opus should be skipped.
        let configured = models_for(&["claude-sonnet-4-6"]);
        assert_eq!(
            next_tier_model(Some("claude-haiku-4-5"), &empty_tier, &configured),
            Some("claude-sonnet-4-6".to_string()),
        );
        // Sonnet tries to escalate to opus, but opus isn't configured → None.
        assert_eq!(
            next_tier_model(Some("claude-sonnet-4-6"), &empty_tier, &configured),
            None,
        );
    }

    #[test]
    fn next_tier_model_skips_to_higher_configured() {
        let empty_tier = HashMap::new();
        // Only opus is configured — should skip sonnet and land on opus.
        let configured = models_for(&["claude-opus-4-6"]);
        assert_eq!(
            next_tier_model(Some("claude-haiku-4-5"), &empty_tier, &configured),
            Some("claude-opus-4-6".to_string()),
        );
    }

    #[test]
    fn next_tier_model_none_when_no_configured_above() {
        let empty_tier = HashMap::new();
        // Only haiku is configured; already at haiku → nothing above.
        let configured = models_for(&["claude-haiku-4-5"]);
        assert_eq!(
            next_tier_model(Some("claude-haiku-4-5"), &empty_tier, &configured),
            None,
        );
    }

    #[test]
    fn next_tier_model_at_top_returns_none() {
        let empty_tier = HashMap::new();
        let no_models = IndexMap::new();
        // Already at the highest tier — no escalation possible.
        assert_eq!(
            next_tier_model(Some("claude-opus-4-6"), &empty_tier, &no_models),
            None,
        );
    }

    /// bug-477ede: a planner outside the chain used to be "escalated" to the
    /// cheapest configured chain model. The retry now keeps it.
    #[test]
    fn next_tier_model_never_downgrades_an_unknown_model() {
        let empty_tier = HashMap::new();
        // A frontier planner outside the chain, with Haiku configured.
        let configured = models_for(&["claude-opus-5-5", "claude-haiku-4-5"]);
        assert_eq!(
            next_tier_model(Some("claude-opus-5-5"), &empty_tier, &configured),
            None,
        );
        assert_eq!(
            next_tier_model(Some("some-random-model"), &empty_tier, &configured),
            None,
        );
        // Unfiltered (nothing configured), an unknown model stays put too.
        assert_eq!(
            next_tier_model(Some("claude-opus-5-5"), &empty_tier, &IndexMap::new()),
            None,
        );
        // With no current model there is no known rank either.
        assert_eq!(next_tier_model(None, &empty_tier, &configured), None);
    }

    /// Keys that are not slugs, as in this repo's roko.toml: the key
    /// `claude-sonnet` is in the chain through its slug, so the retry moves up
    /// to Opus, never down to Haiku.
    #[test]
    fn next_tier_model_matches_a_model_key_by_its_slug() {
        let empty_tier = HashMap::new();
        let configured = models_with(&[
            ("claude-haiku", "claude-haiku-4-5"),
            ("claude-sonnet", "claude-sonnet-4-6"),
            ("claude-opus", "claude-opus-4-6"),
        ]);
        assert_eq!(
            next_tier_model(Some("claude-sonnet"), &empty_tier, &configured),
            Some("claude-opus-4-6".to_string()),
        );
        // Without Opus configured, the retry stays on Sonnet.
        let configured = models_with(&[
            ("claude-haiku", "claude-haiku-4-5"),
            ("claude-sonnet", "claude-sonnet-4-6"),
        ]);
        assert_eq!(
            next_tier_model(Some("claude-sonnet"), &empty_tier, &configured),
            None,
        );
    }

    #[test]
    fn next_tier_model_follows_tier_models_named_by_key() {
        let tiers = HashMap::from([
            ("haiku".to_string(), "fast".to_string()),
            ("sonnet".to_string(), "mid".to_string()),
            ("opus".to_string(), "deep".to_string()),
        ]);
        let configured = models_with(&[
            ("fast", "claude-haiku-4-5"),
            ("mid", "claude-sonnet-4-6"),
            ("deep", "claude-opus-4-6"),
        ]);
        // A current model named by slug finds its tier entry, named by key.
        assert_eq!(
            next_tier_model(Some("claude-sonnet-4-6"), &tiers, &configured),
            Some("deep".to_string()),
        );
        assert_eq!(next_tier_model(Some("deep"), &tiers, &configured), None);
    }

    #[test]
    fn next_tier_model_none_configured_returns_none() {
        let empty_tier = HashMap::new();
        // No chain model is configured → None.
        let configured = models_for(&["totally-different-model"]);
        assert_eq!(
            next_tier_model(Some("claude-haiku-4-5"), &empty_tier, &configured),
            None,
        );
    }

    // ── context-check integration tests ──────────────────────────────────────

    /// Verify that `validate_plan_context` produces a `PLAN_CONTEXT_MISSING`
    /// violation whose message is parseable by the path-extraction code in the
    /// `try_extract_and_validate` closure.  This test pins the message format so
    /// a future change to `plan_policy` cannot silently break the auto-fix path.
    #[test]
    fn plan_context_missing_message_is_parseable() {
        use crate::plan_policy::{PlanExecutionPolicy, validate_plan_context};
        use tempfile::tempdir;

        let dir = tempdir().unwrap();
        let plan_dir = dir.path().join("my-plan");
        std::fs::create_dir_all(&plan_dir).unwrap();
        // Write tasks.toml so PLAN_ARTIFACT_MISSING doesn't fire.
        std::fs::write(plan_dir.join("tasks.toml"), "[meta]\n").unwrap();

        // Write only "src/lib.rs"; "src/missing.rs" is intentionally absent.
        std::fs::create_dir(dir.path().join("src")).unwrap();
        std::fs::write(dir.path().join("src/lib.rs"), "").unwrap();

        // Build a minimal TasksFile via TOML so we don't need to construct
        // every optional field of TaskDef by hand.
        let toml_src = r#"
[meta]
plan = "my-plan"
total = 1
done = 0
status = "ready"
max_parallel = 1

[[task]]
id = "T1"
title = "test"
role = "implementer"
status = "ready"
tier = "focused"
files = ["src/lib.rs"]

[task.context]
[[task.context.read_files]]
path = "src/lib.rs"
why = "exists"

[[task.context.read_files]]
path = "src/missing.rs"
why = "does not exist on disk"

[[task.verify]]
phase = "structural"
command = "true"
"#;
        let tasks_file =
            crate::task_parser::TasksFile::parse_str(toml_src).expect("test TOML must parse");

        let violations = validate_plan_context(
            &tasks_file,
            dir.path(),
            &plan_dir,
            PlanExecutionPolicy::normal(),
        );

        let missing: Vec<_> = violations
            .iter()
            .filter(|v| v.code == "PLAN_CONTEXT_MISSING")
            .collect();
        assert!(
            !missing.is_empty(),
            "expected a PLAN_CONTEXT_MISSING violation"
        );

        // This is the exact extraction logic used in try_extract_and_validate.
        let path = missing[0]
            .message
            .strip_prefix("declared context file `")
            .and_then(|s| s.find('`').map(|end| s[..end].to_string()))
            .expect("PLAN_CONTEXT_MISSING message format must be parseable");
        assert_eq!(path, "src/missing.rs");
    }

    /// Verify that after dropping PLAN_CONTEXT_MISSING paths from a TasksFile
    /// and re-validating, no further PLAN_CONTEXT_MISSING violations remain.
    #[test]
    fn dropping_missing_read_files_clears_context_violations() {
        use crate::plan_policy::{PlanExecutionPolicy, validate_plan_context};
        use tempfile::tempdir;

        let dir = tempdir().unwrap();
        let plan_dir = dir.path().join("my-plan");
        std::fs::create_dir_all(&plan_dir).unwrap();
        std::fs::write(plan_dir.join("tasks.toml"), "[meta]\n").unwrap();
        std::fs::create_dir(dir.path().join("src")).unwrap();
        std::fs::write(dir.path().join("src/lib.rs"), "").unwrap();

        let toml_src = r#"
[meta]
plan = "my-plan"
total = 1
done = 0
status = "ready"
max_parallel = 1

[[task]]
id = "T1"
title = "test"
role = "implementer"
status = "ready"
tier = "focused"
files = ["src/lib.rs"]

[task.context]
[[task.context.read_files]]
path = "src/lib.rs"
why = "exists"

[[task.context.read_files]]
path = "src/missing.rs"
why = "does not exist on disk"

[[task.verify]]
phase = "structural"
command = "true"
"#;
        let mut tasks_file =
            crate::task_parser::TasksFile::parse_str(toml_src).expect("test TOML must parse");

        // Collect the missing paths.
        let violations = validate_plan_context(
            &tasks_file,
            dir.path(),
            &plan_dir,
            PlanExecutionPolicy::normal(),
        );
        let missing_paths: std::collections::HashSet<String> = violations
            .iter()
            .filter(|v| v.code == "PLAN_CONTEXT_MISSING")
            .filter_map(|v| {
                v.message
                    .strip_prefix("declared context file `")
                    .and_then(|s| s.find('`').map(|end| s[..end].to_string()))
            })
            .collect();
        assert!(!missing_paths.is_empty());

        // Drop the missing paths (mirrors the logic in try_extract_and_validate).
        for task in &mut tasks_file.tasks {
            if let Some(ctx) = task.context.as_mut() {
                ctx.read_files.retain(|f| !missing_paths.contains(&f.path));
            }
        }

        // Re-validate — no PLAN_CONTEXT_MISSING should remain.
        let remaining = validate_plan_context(
            &tasks_file,
            dir.path(),
            &plan_dir,
            PlanExecutionPolicy::normal(),
        );
        let still_missing: Vec<_> = remaining
            .iter()
            .filter(|v| v.code == "PLAN_CONTEXT_MISSING")
            .collect();
        assert!(
            still_missing.is_empty(),
            "no PLAN_CONTEXT_MISSING should remain after dropping: {still_missing:?}"
        );
    }
}
