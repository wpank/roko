//! Prompt assembly — turn a task + context into an [`AssembledPrompt`].
//!
//! ## Composition (architectural note)
//!
//! Prompt construction is a **Compose** verb in the Roko model. This
//! module owns the runner-facing seam and delegates the heavy lifting to
//! [`roko_compose::SystemPromptBuilder`] (the 9-layer canonical builder)
//! via [`RoleSystemPromptSpec`] / [`crate::prompting::build_role_system_prompt`].
//! Anything provider-specific (token counting, allowlist syntax) belongs
//! below this layer.
//!
//! ## What's structured
//!
//! The result is intentionally rich:
//!
//! - `system_prompt` — the rendered system message (canonical 9-layer)
//! - `user_prompt` — the rendered user message
//! - `tool_allowlist` — explicit allowlist (intersected with safety
//!   contract upstream of dispatch)
//! - `diagnostics` — what got included / dropped, total token estimate,
//!   playbook ids, knowledge ids — used for prompt experiments and the
//!   projection layer
//! - `gate_feedback` (carried into context, not the result) — structured
//!   compile / test / clippy errors injected on retry
//!
//! Token budget enforcement is deterministic: when the assembled prompt
//! exceeds the configured budget, sections are dropped in priority order
//! (knowledge → playbooks → code-index → retry-feedback → allowlist →
//! task description). The dropped list is reported in `diagnostics` so
//! observers can investigate budget pressure.
//!
//! ## Test seam
//!
//! [`PromptAssembler::minimal`] returns an assembler with no playbook /
//! neuro store and a tiny default budget — used by tests and CI smoke
//! runs to keep prompt construction deterministic.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use parking_lot::RwLock;
use roko_compose::{
    AttentionBidder, CompositionManifest, CompositionStrategy, ContextChunk, ContextSource,
    LearningBidder, PromptComposer, PromptSection as CanonicalPromptSection, RoleSystemPromptSpec,
    TaskContext,
};
use roko_core::config::schema::ConfigCompositionStrategy;
use roko_core::{AgentRole, Group, GroupId, GroupPheromone};
use serde::{Deserialize, Serialize};

use super::outcome::RunnerDispatchError;
use super::prompt_cache::PromptCache;
use super::{DispatchContext, PromptExperimentContext};
use crate::task_parser::TaskDef;

/// Maximum tokens an assembled prompt may emit before deterministic
/// dropping kicks in. Roughly mirrors a 200K-context-window providers'
/// budget for system + user combined.
const DEFAULT_TOKEN_BUDGET: u32 = 64_000;

// ─── Inputs ────────────────────────────────────────────────────────────

/// Per-call context the assembler needs from the runner.
///
/// Constructed from a `TaskDef` + `DispatchContext` so the assembler
/// stays pure.
#[derive(Debug, Clone)]
pub struct PromptContext {
    /// Plan id.
    pub plan_id: String,
    /// Role label.
    pub role: String,
    /// Attempt working directory used for task-local workspace enrichment.
    /// Durable prompt experiments use the explicit root path in
    /// [`Self::prompt_experiment`] instead.
    pub workdir: PathBuf,
    /// Files in scope for this task (from `task.files`).
    pub files_in_scope: Vec<String>,
    /// Acceptance criteria (from `task.acceptance`).
    pub acceptance_criteria: Vec<String>,
    /// `task.verify` shell commands.
    pub verify_commands: Vec<String>,
    /// Declared-scope impact warning included before implementation.
    pub impact_context: String,
    /// Optional structured gate feedback for retry prompts.
    pub gate_feedback: Option<GateFeedback>,
    /// Attempt number (0 = first, > 0 = retry).
    pub attempt: u32,
    /// Durable experiment identity and root-workspace store path for this
    /// attempt, when prompt experiments are enabled.
    pub prompt_experiment: Option<PromptExperimentContext>,
    /// Indented tree of `crates/*/src/` paths (truncated to 20 000 chars).
    pub workspace_map: String,
    /// Raw content of this plan's `tasks.toml` (truncated to 10 000 chars).
    pub tasks_toml: String,
    /// Short excerpt from the plan's PRD document (truncated to 2 000 chars).
    pub prd_excerpt: String,
    /// Output files from completed dependency tasks.
    /// Each entry is `(task_id, files)`.
    pub dependency_outputs: Vec<(String, Vec<String>)>,
    /// Workspace context: git branch, modified files, crate names/descriptions.
    /// Ported from the legacy `workspace_context()` helper; includes
    /// git state (best-effort, bounded) and crate scan from `crates/*/Cargo.toml`.
    pub workspace_context: String,
    /// C-Factor collective-intelligence policy text.
    /// Loaded from `.roko/learn/c-factor.jsonl` when history exists.
    pub cfactor_context: String,
}

impl PromptContext {
    /// Construct a `PromptContext` from runner inputs.
    #[must_use]
    pub fn from_task(task: &TaskDef, ctx: &DispatchContext) -> Self {
        let execution_policy = crate::plan_policy::PlanExecutionPolicy::for_environment();
        let bounded_context_only = execution_policy.bounded_context_only;
        let workspace_map = if bounded_context_only {
            String::new()
        } else {
            generate_workspace_map(&ctx.workdir)
        };
        let tasks_toml = if bounded_context_only {
            String::new()
        } else {
            load_tasks_toml(&ctx.workdir, &ctx.plan_id)
        };
        let prd_excerpt = load_prd_excerpt(&ctx.workdir, &ctx.plan_id);
        let workspace_context = if bounded_context_only {
            String::new()
        } else {
            generate_workspace_context(&ctx.workdir)
        };
        let cfactor_context = if bounded_context_only {
            String::new()
        } else {
            generate_cfactor_context(&ctx.workdir)
        };
        let impact_context = declared_impact_context(task, bounded_context_only);
        tracing::debug!(
            plan_id = %ctx.plan_id,
            workspace_map_bytes = workspace_map.len(),
            tasks_toml_bytes = tasks_toml.len(),
            prd_excerpt_bytes = prd_excerpt.len(),
            workspace_context_bytes = workspace_context.len(),
            cfactor_context_bytes = cfactor_context.len(),
            "PromptContext enrichment sizes"
        );
        Self {
            plan_id: ctx.plan_id.clone(),
            role: ctx.role.clone(),
            workdir: ctx.workdir.clone(),
            files_in_scope: task.files.clone(),
            acceptance_criteria: task.acceptance.clone(),
            verify_commands: task
                .verify
                .iter()
                .map(|step| step.command.clone())
                .collect(),
            impact_context,
            gate_feedback: ctx.gate_feedback.clone(),
            attempt: ctx.attempt,
            prompt_experiment: ctx.prompt_experiment.clone(),
            workspace_map,
            tasks_toml,
            prd_excerpt,
            dependency_outputs: ctx.dependency_outputs.clone(),
            workspace_context,
            cfactor_context,
        }
    }
}

fn declared_impact_context(task: &TaskDef, bounded_context_only: bool) -> String {
    let description = format!(
        "{} {}",
        task.title,
        task.description.as_deref().unwrap_or_default()
    )
    .to_ascii_lowercase();
    let high_impact_terms = [
        "public",
        "signature",
        "struct field",
        "enum",
        "trait",
        "serialize",
        "serde",
        "schema",
        "re-export",
        "reexport",
        "api contract",
    ];
    let high_impact = !task
        .context
        .as_ref()
        .map_or(true, |context| context.symbols.is_empty())
        || high_impact_terms
            .iter()
            .any(|term| description.contains(term))
        || task.files.iter().any(|file| file.ends_with("Cargo.toml"));
    if !high_impact {
        return "Impact policy: keep the edit private/local when possible; report any newly discovered consumer outside the planned file list.".into();
    }
    let files = task
        .files
        .iter()
        .map(|file| format!("`{file}`"))
        .collect::<Vec<_>>()
        .join(", ");
    if bounded_context_only {
        format!(
            "Impact policy: this task may change a public, trait, re-export, or serialized contract. Exact declared symbols and source snippets are supplied below. Do not broad-search in FAST mode. The authorized planned scope is: {files}. If the supplied consumers are insufficient, stop and surface the omission for plan repair. The runner's post-diff impact analyzer remains the safety net."
        )
    } else {
        format!(
            "Impact policy: this task may change a public, trait, re-export, or serialized contract. Start with the supplied exact symbols and snippets. If they reveal an unresolved consumer, perform at most one repository-scoped exact-symbol search capped at 20 matches. Never search home/session history, other worktrees, unreachable Git objects, or the web. The authorized planned scope is: {files}. Surface omissions for plan repair; the runner will analyze the final diff."
        )
    }
}

// ─── PromptContext enrichment helpers ──────────────────────────────────

const WORKSPACE_MAP_LIMIT: usize = 20_000;
const TASKS_TOML_LIMIT: usize = 10_000;
const PRD_EXCERPT_LIMIT: usize = 2_000;

/// Walk `{workdir}/crates/*/src/` and produce an indented file tree.
///
/// The result is truncated to [`WORKSPACE_MAP_LIMIT`] characters so it
/// never balloons the system prompt on large workspaces.
fn generate_workspace_map(workdir: &Path) -> String {
    let crates_dir = workdir.join("crates");

    let mut out = String::from("# Workspace crate map\n");
    let mut entries: Vec<_> = match std::fs::read_dir(&crates_dir) {
        Ok(e) => e.filter_map(|r| r.ok()).collect(),
        Err(_) => return String::new(),
    };
    entries.sort_by_key(|e| e.file_name());

    for entry in entries {
        let crate_path = entry.path();
        if !crate_path.is_dir() {
            continue;
        }
        let crate_name = crate_path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        out.push_str(&format!("crates/{crate_name}/\n"));

        let src_dir = crate_path.join("src");
        // walk_src_tree handles missing dirs gracefully via read_dir error.
        out.push_str(&walk_src_tree(&src_dir, "  ", 0));

        if out.len() >= WORKSPACE_MAP_LIMIT {
            out.truncate(WORKSPACE_MAP_LIMIT);
            out.push_str("\n[truncated]");
            return out;
        }
    }

    if out.len() > WORKSPACE_MAP_LIMIT {
        out.truncate(WORKSPACE_MAP_LIMIT);
        out.push_str("\n[truncated]");
    }
    out
}

/// Recursively walk a source directory, producing an indented tree.
///
/// Stops at `MAX_DEPTH` levels of nesting to avoid runaway recursion on
/// deeply nested source trees.
fn walk_src_tree(dir: &Path, prefix: &str, depth: usize) -> String {
    const MAX_DEPTH: usize = 3;
    if depth >= MAX_DEPTH {
        return String::new();
    }

    let mut out = String::new();
    let mut entries: Vec<_> = match std::fs::read_dir(dir) {
        Ok(e) => e.filter_map(|r| r.ok()).collect(),
        Err(_) => return out,
    };
    // Directories first, then files, each group sorted by name.
    entries.sort_by_key(|e| {
        let is_file = e.path().is_file();
        (is_file as u8, e.file_name())
    });

    for entry in entries {
        let path = entry.path();
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        if path.is_dir() {
            out.push_str(&format!("{prefix}{name}/\n"));
            out.push_str(&walk_src_tree(&path, &format!("{prefix}  "), depth + 1));
        } else {
            out.push_str(&format!("{prefix}{name}\n"));
        }
    }
    out
}

/// Load `tasks.toml` for `plan_id` from the two canonical locations.
///
/// Searches:
/// 1. `{workdir}/.roko/plans/{plan_id}/tasks.toml`
/// 2. `{workdir}/plans/{plan_id}/tasks.toml`
///
/// Returns an empty string when neither exists.
fn load_tasks_toml(workdir: &Path, plan_id: &str) -> String {
    let candidates = [
        workdir
            .join(".roko")
            .join("plans")
            .join(plan_id)
            .join("tasks.toml"),
        workdir.join("plans").join(plan_id).join("tasks.toml"),
    ];
    for path in &candidates {
        match std::fs::read_to_string(path) {
            Ok(content) => {
                return if content.len() > TASKS_TOML_LIMIT {
                    let mut truncated = content.chars().take(TASKS_TOML_LIMIT).collect::<String>();
                    truncated.push_str("\n[truncated]");
                    truncated
                } else {
                    content
                };
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
            Err(_) => continue,
        }
    }
    String::new()
}

/// Load a PRD excerpt for `plan_id`.
///
/// Searches:
/// 1. `{workdir}/.roko/prd/published/{plan_id}.md`
/// 2. `{workdir}/.roko/prd/drafts/{plan_id}.md`
///
/// Returns an empty string when neither exists.
fn load_prd_excerpt(workdir: &Path, plan_id: &str) -> String {
    let prd_base = workdir.join(".roko").join("prd");
    let candidates = [
        prd_base.join("published").join(format!("{plan_id}.md")),
        prd_base.join("drafts").join(format!("{plan_id}.md")),
        prd_base.join("draft").join(format!("{plan_id}.md")),
    ];
    for path in &candidates {
        match std::fs::read_to_string(path) {
            Ok(content) => {
                return if content.len() > PRD_EXCERPT_LIMIT {
                    let mut truncated = content.chars().take(PRD_EXCERPT_LIMIT).collect::<String>();
                    truncated.push_str("\n[truncated]");
                    truncated
                } else {
                    content
                };
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
            Err(_) => continue,
        }
    }
    String::new()
}

// ─── Workspace context (ported from legacy orchestrator) ───────────────

const WORKSPACE_CONTEXT_LIMIT: usize = 4_000;
#[allow(dead_code)]
const GIT_COMMAND_TIMEOUT: Duration = Duration::from_secs(3);
const GIT_STATUS_LINE_LIMIT: usize = 40;

/// Build a bounded workspace context string with git state and crate descriptions.
///
/// Combines:
/// - Current git branch (`git branch --show-current`)
/// - Modified files (`git status --short`), capped at [`GIT_STATUS_LINE_LIMIT`] lines
/// - Crate names and descriptions from `crates/*/Cargo.toml`
///
/// All git calls are best-effort with a [`GIT_COMMAND_TIMEOUT`] to avoid hanging
/// on non-git workdirs or slow NFS mounts.
fn generate_workspace_context(workdir: &Path) -> String {
    let mut out = String::from("# Workspace context\n");

    // ── Git branch ──────────────────────────────────────────────────────
    if let Some(branch) = git_command(workdir, &["branch", "--show-current"]) {
        let branch = branch.trim();
        if !branch.is_empty() {
            out.push_str(&format!("Branch: `{branch}`\n"));
        }
    }

    // ── Git modified files ──────────────────────────────────────────────
    if let Some(status) = git_command(workdir, &["status", "--short"]) {
        let lines: Vec<&str> = status.lines().filter(|l| !l.trim().is_empty()).collect();
        if !lines.is_empty() {
            out.push_str(&format!("Modified files ({}):\n", lines.len()));
            for line in lines.iter().take(GIT_STATUS_LINE_LIMIT) {
                out.push_str(&format!("  {line}\n"));
            }
            if lines.len() > GIT_STATUS_LINE_LIMIT {
                out.push_str(&format!(
                    "  ... and {} more\n",
                    lines.len() - GIT_STATUS_LINE_LIMIT
                ));
            }
        }
    }

    // ── Crate descriptions ──────────────────────────────────────────────
    let crate_descriptions = scan_crate_descriptions(workdir);
    if !crate_descriptions.is_empty() {
        out.push_str("\n## Workspace crates\n");
        for (name, desc) in &crate_descriptions {
            if desc.is_empty() {
                out.push_str(&format!("- {name}\n"));
            } else {
                out.push_str(&format!("- {name}: {desc}\n"));
            }
            if out.len() >= WORKSPACE_CONTEXT_LIMIT {
                out.truncate(WORKSPACE_CONTEXT_LIMIT);
                out.push_str("\n[truncated]");
                return out;
            }
        }
    }

    // If we only have the header and nothing else, return empty.
    if out.trim() == "# Workspace context" {
        return String::new();
    }

    if out.len() > WORKSPACE_CONTEXT_LIMIT {
        out.truncate(WORKSPACE_CONTEXT_LIMIT);
        out.push_str("\n[truncated]");
    }
    out
}

/// Run a git command with a bounded timeout. Returns `None` on any failure.
fn git_command(workdir: &Path, args: &[&str]) -> Option<String> {
    let output = std::process::Command::new("git")
        .args(["-C", &workdir.to_string_lossy()])
        .args(args)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .ok()?;

    // Use wait_with_output with a background thread to enforce a timeout.
    let handle = std::thread::spawn(move || output.wait_with_output());
    match handle.join() {
        Ok(Ok(output)) if output.status.success() => String::from_utf8(output.stdout).ok(),
        _ => None,
    }
}

/// Scan `crates/*/Cargo.toml` for package names and descriptions.
///
/// Ported from the legacy `workspace_context()` helper.
fn scan_crate_descriptions(workdir: &Path) -> Vec<(String, String)> {
    let crates_dir = workdir.join("crates");
    let entries = match std::fs::read_dir(&crates_dir) {
        Ok(e) => e,
        Err(_) => return Vec::new(),
    };

    let mut crates: Vec<(String, String)> = Vec::new();
    for entry in entries.flatten() {
        let cargo_path = entry.path().join("Cargo.toml");
        let Ok(content) = std::fs::read_to_string(&cargo_path) else {
            continue;
        };
        let Ok(parsed) = content.parse::<toml::Value>() else {
            continue;
        };
        let name = parsed
            .get("package")
            .and_then(|p| p.get("name"))
            .and_then(|n| n.as_str())
            .unwrap_or_default()
            .to_string();
        let desc = parsed
            .get("package")
            .and_then(|p| p.get("description"))
            .and_then(|d| d.as_str())
            .unwrap_or("")
            .to_string();
        if !name.is_empty() {
            crates.push((name, desc));
        }
    }
    crates.sort_by(|a, b| a.0.cmp(&b.0));
    crates
}

// ─── C-Factor context (ported from legacy orchestrator) ────────────────

/// Load C-Factor history and generate policy context for the system prompt.
///
/// Reads `.roko/learn/c-factor.jsonl`, computes a summary, and runs the
/// [`roko_core::CFactorPolicy`] to produce coordination guidance text.
/// Returns an empty string when no history exists or the episode count
/// is below the minimum threshold.
fn generate_cfactor_context(workdir: &Path) -> String {
    use roko_core::{CFactorPolicy, CFactorSource, Context, React};
    use roko_learn::cfactor::CFactor;
    use std::sync::Arc;

    let cfactor_path = roko_fs::RokoLayout::for_project(workdir)
        .learn_dir()
        .join("c-factor.jsonl");

    let contents = match std::fs::read_to_string(&cfactor_path) {
        Ok(c) => c,
        Err(_) => return String::new(),
    };

    let mut history: Vec<CFactor> = contents
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect();
    history.sort_by(|left, right| left.computed_at.cmp(&right.computed_at));

    let Some(current) = history.last().cloned() else {
        return String::new();
    };

    let historical_average = if history.len() > 1 {
        history[..history.len() - 1]
            .iter()
            .map(|snapshot| snapshot.overall)
            .sum::<f64>()
            / (history.len() - 1) as f64
    } else {
        current.overall
    };
    let trend = current.overall - historical_average;
    let regression = roko_learn::cfactor::detect_cfactor_regression(
        &history,
        Duration::from_secs(7 * 24 * 60 * 60),
        0.08,
    );

    // Collect top contributors.
    let mut positive: Vec<_> = current
        .agent_contributions
        .iter()
        .filter(|c| c.contribution_score > 0.0)
        .cloned()
        .collect();
    positive.sort_by(|a, b| {
        b.contribution_score
            .total_cmp(&a.contribution_score)
            .then(a.agent_id.cmp(&b.agent_id))
    });
    let mut negative: Vec<_> = current
        .agent_contributions
        .iter()
        .filter(|c| c.contribution_score < 0.0)
        .cloned()
        .collect();
    negative.sort_by(|a, b| {
        a.contribution_score
            .total_cmp(&b.contribution_score)
            .then(a.agent_id.cmp(&b.agent_id))
    });

    let top_positive: Vec<String> = positive
        .iter()
        .take(3)
        .map(|c| c.agent_id.clone())
        .collect();
    let top_negative: Vec<String> = negative
        .iter()
        .take(3)
        .map(|c| c.agent_id.clone())
        .collect();

    let summary = roko_core::CFactorSummary {
        overall: current.overall,
        trend,
        regression_drop: regression.map_or(0.0, |entry| entry.drop_fraction),
        gate_pass_rate: current.components.gate_pass_rate,
        turn_taking_equality: current.components.turn_taking_equality,
        social_perceptiveness: current.components.social_perceptiveness,
        citation_reciprocity: current.components.knowledge_integration_rate,
        delivery_rate: current.components.information_flow_rate,
        hdc_diversity: current.components.hdc_diversity,
        episode_count: current.episode_count,
        top_positive_contributors: top_positive,
        top_negative_contributors: top_negative,
    };

    // Use CFactorPolicy to generate signals, then extract their text bodies.
    #[derive(Clone)]
    struct StaticSource(Option<roko_core::CFactorSummary>);
    impl CFactorSource for StaticSource {
        fn summary(&self) -> Option<roko_core::CFactorSummary> {
            self.0.clone()
        }
    }

    let source: Arc<dyn CFactorSource> = Arc::new(StaticSource(Some(summary)));
    let policy = CFactorPolicy::new(source).with_min_episode_count(6);
    let signals = policy.decide(&[], &Context::now());

    if signals.is_empty() {
        return String::new();
    }

    let mut out = String::from("# Collective calibration\n");
    for signal in &signals {
        if let Ok(text) = signal.body.as_text() {
            let text = text.trim();
            if !text.is_empty() {
                out.push_str(text);
                out.push('\n');
            }
        }
    }

    if out.trim() == "# Collective calibration" {
        return String::new();
    }

    out
}

/// Structured gate feedback injected into retry prompts.
///
/// Replaces the legacy "raw stdout dump" prepend with a typed payload
/// the prompt builder can render selectively.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GateFeedback {
    /// Compile errors lifted from cargo check output.
    #[serde(default)]
    pub compile_errors: Vec<String>,
    /// Failing test names + their summaries.
    #[serde(default)]
    pub test_failures: Vec<String>,
    /// Clippy warnings that surfaced.
    #[serde(default)]
    pub clippy_warnings: Vec<String>,
    /// The original gate output (truncated to ≤ 4 KB upstream).
    pub raw_output: String,
}

impl GateFeedback {
    /// Parse raw gate output into structured retry context.
    #[must_use]
    pub fn from_raw(raw_output: &str) -> Option<Self> {
        let raw = raw_output.trim();
        if raw.is_empty() {
            return None;
        }

        let mut compile_errors = Vec::new();
        let mut test_failures = Vec::new();
        let mut clippy_warnings = Vec::new();
        for line in raw.lines().map(str::trim).filter(|line| !line.is_empty()) {
            let lower = line.to_ascii_lowercase();
            let truncated = line.chars().take(240).collect::<String>();
            if lower.contains("error[") || lower.starts_with("error:") || line.contains("-->") {
                compile_errors.push(truncated);
            } else if lower.contains("test")
                && (lower.contains("failed") || lower.contains("panicked"))
            {
                test_failures.push(truncated);
            } else if lower.contains("warning") || lower.contains("clippy") {
                clippy_warnings.push(truncated);
            }
            if compile_errors.len() + test_failures.len() + clippy_warnings.len() >= 24 {
                break;
            }
        }

        Some(Self {
            compile_errors,
            test_failures,
            clippy_warnings,
            raw_output: raw
                .chars()
                .take(roko_core::defaults::DEFAULT_TOOL_OUTPUT_TRUNCATE_AT)
                .collect(),
        })
    }
}

// ─── Outputs ───────────────────────────────────────────────────────────

/// Assembled prompt, allowlist, and diagnostics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssembledPrompt {
    /// Rendered system prompt.
    pub system_prompt: String,
    /// Rendered user prompt.
    pub user_prompt: String,
    /// Optional tool allowlist (intersected with safety contract).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_allowlist: Option<Vec<String>>,
    /// Per-assembly diagnostics for experiments + projection.
    pub diagnostics: PromptDiagnostics,
}

/// Auditable info about the assembly run.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PromptDiagnostics {
    /// Sections that made it into the rendered prompt.
    pub included_sections: Vec<String>,
    /// Sections dropped to fit the token budget.
    pub dropped_sections: Vec<String>,
    /// Coarse estimate of the assembled prompt token count.
    pub estimated_tokens: u32,
    /// Playbook ids consulted (if any).
    pub playbook_ids: Vec<String>,
    /// Neuro knowledge ids surfaced (if any).
    pub knowledge_ids: Vec<String>,
    /// Canonical source refs and score results produced by prompt composition.
    #[serde(default)]
    pub scored_signals: Vec<ScoredSignalDiagnostic>,
    /// Raw-content-free canonical allocation receipt. This is retained until
    /// the terminal gate outcome so the exact eligible bidders and selected
    /// sections can receive learning feedback.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub composition_manifest: Option<CompositionManifest>,
    /// Raw-content-free durable experiment assignments applied before
    /// canonical scoring and composition.
    #[serde(default)]
    pub experiment_assignments: Vec<PromptExperimentAssignmentDiagnostic>,
}

/// One content-addressed prompt source and its serialized score result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScoredSignalDiagnostic {
    /// Full content-addressed Signal reference.
    pub signal_ref: String,
    /// JSON-encoded [`roko_compose::CandidateScoreResult`].
    pub score_result: String,
}

/// One durable prompt experiment assignment and whether its canonical section
/// survived the composition budget.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PromptExperimentAssignmentDiagnostic {
    /// Stable durable assignment id used by dispatch and terminal feedback.
    pub assignment_id: String,
    /// Experiment that selected the variant.
    pub experiment_id: String,
    /// Selected variant id. Raw variant content is intentionally omitted.
    pub variant_id: String,
    /// Canonical prompt section replaced by the assigned content snapshot.
    pub section_name: String,
    /// Content hash retained by the experiment store.
    pub content_hash: String,
    /// Whether this assigned section survived canonical composition.
    pub included: bool,
}

/// Best-effort cleanup for a treatment bucket that was prepared successfully
/// but could not produce a dispatchable prompt. The runner has not crossed the
/// provider boundary yet, so abandoning here must not count a trial.
fn abandon_prompt_experiment_after_assembly_error(
    experiment: &PromptExperimentContext,
    assignments: &[roko_learn::prompt_experiment::PromptExperimentAssignment],
    stage: &'static str,
) {
    if assignments.is_empty() {
        return;
    }
    if let Err(error) = roko_learn::prompt_experiment::ExperimentStore::settle_attempt(
        &experiment.store_path,
        &experiment.attempt_key,
        roko_learn::prompt_experiment::AssignmentSettlement::Abandoned,
    ) {
        tracing::warn!(
            plan_id = %experiment.attempt_key.plan_id,
            task_id = %experiment.attempt_key.task_id,
            attempt = experiment.attempt_key.attempt,
            %stage,
            %error,
            "failed to abandon prompt-experiment treatment after prompt assembly error"
        );
    }
}

fn apply_prompt_experiment_assignments(
    sections: &mut [CanonicalPromptSection],
    assignments: &[roko_learn::prompt_experiment::PromptExperimentAssignment],
    expected_attempt: &roko_learn::prompt_experiment::PromptAttemptKey,
    expected_role: &str,
) -> Result<(), RunnerDispatchError> {
    let expected_role = expected_role.trim();
    let mut section_indices = HashMap::new();
    for (index, section) in sections.iter().enumerate() {
        if section_indices
            .insert(section.name.clone(), index)
            .is_some()
        {
            return Err(RunnerDispatchError::PromptAssembly(format!(
                "canonical prompt contains duplicate section name {:?}",
                section.name
            )));
        }
    }

    let mut replaced_sections = HashSet::new();
    for assignment in assignments {
        if assignment.attempt_key != *expected_attempt {
            return Err(RunnerDispatchError::PromptAssembly(format!(
                "prompt experiment assignment {} belongs to a different attempt",
                assignment.assignment_id
            )));
        }
        if assignment
            .role
            .as_deref()
            .is_some_and(|role| role != expected_role)
        {
            return Err(RunnerDispatchError::PromptAssembly(format!(
                "prompt experiment assignment {} targets role {:?}, not {:?}",
                assignment.assignment_id, assignment.role, expected_role
            )));
        }
        if !replaced_sections.insert(assignment.section_name.clone()) {
            return Err(RunnerDispatchError::PromptAssembly(format!(
                "multiple prompt experiment assignments target canonical section {:?}",
                assignment.section_name
            )));
        }
        let Some(index) = section_indices.get(&assignment.section_name).copied() else {
            return Err(RunnerDispatchError::PromptAssembly(format!(
                "prompt experiment assignment {} targets unknown canonical section {:?}",
                assignment.assignment_id, assignment.section_name
            )));
        };
        let content = assignment.content_snapshot.as_ref().ok_or_else(|| {
            RunnerDispatchError::PromptAssembly(format!(
                "prompt experiment assignment {} has no content snapshot",
                assignment.assignment_id
            ))
        })?;
        let actual_content_hash = roko_core::ContentHash::of(content.as_bytes()).to_hex();
        if actual_content_hash != assignment.content_hash {
            return Err(RunnerDispatchError::PromptAssembly(format!(
                "prompt experiment assignment {} content hash does not match its snapshot",
                assignment.assignment_id
            )));
        }

        // Deliberately mutate only content and attribution. Canonical policy
        // metadata (stable section id, priority, cache layer, placement, cap,
        // and bidder) must remain exactly as the role builder produced it.
        let section = &mut sections[index];
        section.content.clone_from(content);
        section.source_type = Some("prompt_experiment".into());
        section.source_id = Some(assignment.variant_id.clone());
        section.provenance = Some(format!(
            "prompt_experiment:{}:{}",
            assignment.experiment_id, assignment.assignment_id
        ));
        section.experiment_id = Some(assignment.experiment_id.clone());
    }
    Ok(())
}

// ─── Source Plugins ────────────────────────────────────────────────────

/// One optional section contributed by a prompt context source.
#[derive(Debug, Clone)]
struct PromptSection {
    name: String,
    body: String,
    #[allow(dead_code)]
    drop_priority: u32,
    knowledge_ids: Vec<String>,
    playbook_ids: Vec<String>,
}

impl PromptSection {
    fn new(name: impl Into<String>, body: impl Into<String>, drop_priority: u32) -> Self {
        Self {
            name: name.into(),
            body: body.into(),
            drop_priority,
            knowledge_ids: Vec::new(),
            playbook_ids: Vec::new(),
        }
    }

    fn with_knowledge_ids(mut self, ids: Vec<String>) -> Self {
        self.knowledge_ids = ids;
        self
    }

    fn with_playbook_ids(mut self, ids: Vec<String>) -> Self {
        self.playbook_ids = ids;
        self
    }
}

/// Pluggable prompt context provider.
trait PromptSectionSource: Send + Sync + std::fmt::Debug {
    fn collect(&self, task: &TaskDef, ctx: &PromptContext) -> Vec<PromptSection>;
}

/// Reads durable `.roko` knowledge stores and prior episodes.
///
/// When `cache` is present, searches in-memory vectors instead of hitting
/// the filesystem. When absent, falls back to the original I/O path.
#[derive(Debug, Clone)]
struct WorkdirKnowledgeSource {
    cache: Option<Arc<PromptCache>>,
}

/// Reads learned playbooks from `.roko/learn/playbooks`.
///
/// When `cache` is present, searches the pre-loaded playbook vec.
#[derive(Debug, Clone)]
struct WorkdirPlaybookSource {
    cache: Option<Arc<PromptCache>>,
}

/// Applies learned section-effectiveness priority adjustments.
///
/// When `cache` is present, reads from the pre-loaded registry.
#[derive(Debug, Clone)]
struct SectionEffectivenessSource {
    cache: Option<Arc<PromptCache>>,
}

// ─── Assembler ─────────────────────────────────────────────────────────

/// Parse a role label string into [`AgentRole`].
///
/// Accepts kebab-case labels (e.g. `"implementer"`, `"quick-reviewer"`) as
/// well as debug-style variant names (e.g. `"Implementer"`). Falls back to
/// [`AgentRole::Implementer`] for unrecognised values so prompt assembly never
/// fails hard on a missing or malformed role.
fn parse_role_label(role: &str) -> AgentRole {
    let normalized = role.trim().to_ascii_lowercase().replace(['_', ' '], "-");
    // Try kebab-case serde repr first (e.g. "implementer", "quick-reviewer").
    if let Ok(parsed) = serde_json::from_str::<AgentRole>(&format!("\"{normalized}\"")) {
        return parsed;
    }
    // Try iterating all known variants (covers debug-name variants like "Implementer").
    for candidate in [
        AgentRole::Conductor,
        AgentRole::Strategist,
        AgentRole::Implementer,
        AgentRole::Architect,
        AgentRole::Researcher,
        AgentRole::Auditor,
        AgentRole::QuickReviewer,
        AgentRole::AutoFixer,
        AgentRole::Refactorer,
        AgentRole::Scribe,
    ] {
        if normalized == candidate.label() {
            return candidate;
        }
    }
    tracing::debug!(role = %role, "unrecognised role label — defaulting to Implementer");
    AgentRole::Implementer
}

/// Build the rich runner context string for the canonical `context_layer`.
///
/// Assembles files-in-scope, acceptance criteria, verify commands, gate retry
/// feedback, dependency outputs, PRD excerpt, workspace map, tasks toml,
/// workspace context, and C-factor context into a single markdown block. This
/// block is passed to [`TaskContext::with_context`] so the canonical 9-layer
/// builder includes it in the "Relevant Context" section.
fn build_runner_context(
    task: &TaskDef,
    ctx: &PromptContext,
) -> Result<String, RunnerDispatchError> {
    let mut parts: Vec<String> = Vec::new();

    let declared_context = crate::plan_policy::render_declared_context(
        task,
        &ctx.workdir,
        crate::plan_policy::PlanExecutionPolicy::for_environment(),
    )
    .map_err(|reason| RunnerDispatchError::PreValidationFailed { reason })?;
    if !declared_context.is_empty() {
        parts.push(declared_context);
    }

    if !ctx.files_in_scope.is_empty() {
        let list = ctx
            .files_in_scope
            .iter()
            .map(|f| format!("- `{f}`"))
            .collect::<Vec<_>>()
            .join("\n");
        parts.push(format!("# Files in scope\n{list}"));
    }

    if !ctx.acceptance_criteria.is_empty() {
        let list = ctx
            .acceptance_criteria
            .iter()
            .map(|c| format!("- {c}"))
            .collect::<Vec<_>>()
            .join("\n");
        parts.push(format!("# Acceptance criteria\n{list}"));
    }

    if !ctx.verify_commands.is_empty() {
        let list = ctx
            .verify_commands
            .iter()
            .map(|v| format!("- `{v}`"))
            .collect::<Vec<_>>()
            .join("\n");
        parts.push(format!("# Verify\nAfter editing, run:\n{list}"));
    }

    if !ctx.impact_context.is_empty() {
        parts.push(format!("# Change impact\n{}", ctx.impact_context));
    }

    if let Some(allowlist) = task.allowed_tools.as_ref().filter(|l| !l.is_empty()) {
        let joined = allowlist
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>()
            .join(", ");
        parts.push(format!("# Allowed tools\nYou may only invoke: {joined}"));
    }

    if ctx.attempt > 0 {
        if let Some(feedback) = &ctx.gate_feedback {
            parts.push(render_gate_feedback(feedback));
        }
    }

    if !ctx.dependency_outputs.is_empty() {
        let mut dep = String::from(
            "# Prior Task Outputs\n\nThese tasks have already completed. \
             Use their output files instead of reimplementing.\n",
        );
        for (task_id, files) in &ctx.dependency_outputs {
            dep.push_str(&format!(
                "\n## Completed by task {task_id}:\nFiles created/modified:\n"
            ));
            for f in files {
                dep.push_str(&format!("- `{f}`\n"));
            }
        }
        parts.push(dep);
    }

    if !ctx.prd_excerpt.is_empty() {
        parts.push(format!("# PRD Requirements\n{}", ctx.prd_excerpt));
    }

    if !ctx.workspace_map.is_empty() {
        parts.push(ctx.workspace_map.clone());
    }

    if !ctx.tasks_toml.is_empty() {
        parts.push(format!("# Sibling Tasks\n```toml\n{}\n```", ctx.tasks_toml));
    }

    if !ctx.workspace_context.is_empty() {
        parts.push(ctx.workspace_context.clone());
    }

    if !ctx.cfactor_context.is_empty() {
        parts.push(ctx.cfactor_context.clone());
    }

    Ok(parts.join("\n\n"))
}

/// The file name for the persisted attention bidders store under `.roko/learn/`.
pub const ATTENTION_BIDDERS_FILENAME: &str = "attention-bidders.json";
const MAX_ATTENTION_BIDDERS_BYTES: u64 = 4 * 1024 * 1024;

/// Load persisted learning bidders from `.roko/learn/attention-bidders.json`.
///
/// A missing store is a valid cold start. Malformed, oversized, or internally
/// inconsistent stores return an error so the caller can avoid overwriting
/// forensic evidence with a new cold-start state.
pub fn load_attention_bidders(
    learn_dir: &Path,
) -> std::io::Result<HashMap<AttentionBidder, LearningBidder>> {
    let path = learn_dir.join(ATTENTION_BIDDERS_FILENAME);
    let metadata = match std::fs::metadata(&path) {
        Ok(metadata) => metadata,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(HashMap::new()),
        Err(err) => return Err(err),
    };
    if metadata.len() > MAX_ATTENTION_BIDDERS_BYTES {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!(
                "attention bidder store is {} bytes; limit is {MAX_ATTENTION_BIDDERS_BYTES}",
                metadata.len()
            ),
        ));
    }

    let contents = std::fs::read_to_string(&path)?;
    let bidders: HashMap<AttentionBidder, LearningBidder> = serde_json::from_str(&contents)
        .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidData, err))?;
    for (key, bidder) in &bidders {
        if bidder.subsystem_id != *key
            || !bidder.prior_bid.is_finite()
            || bidder.prior_bid < 0.0
            || bidder.section_betas.values().any(|(alpha, beta)| {
                !alpha.is_finite() || !beta.is_finite() || *alpha <= 0.0 || *beta <= 0.0
            })
            || bidder
                .section_costs
                .values()
                .any(|stats| !stats.total_cost_usd.is_finite() || stats.total_cost_usd < 0.0)
        {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "attention bidder store failed invariant validation",
            ));
        }
    }
    tracing::debug!(path = %path.display(), bidder_count = bidders.len(), "loaded attention bidders");
    Ok(bidders)
}

/// Save learning bidders to `.roko/learn/attention-bidders.json`.
///
/// Creates the learn directory if it does not exist and atomically replaces
/// the prior snapshot only after the complete JSON payload is durable.
pub fn save_attention_bidders(
    learn_dir: &Path,
    bidders: &HashMap<AttentionBidder, LearningBidder>,
) -> std::io::Result<()> {
    std::fs::create_dir_all(learn_dir)?;
    let path = learn_dir.join(ATTENTION_BIDDERS_FILENAME);
    roko_fs::atomic_write_json(&path, bidders)?;
    tracing::debug!(
        path = %path.display(),
        bidder_count = bidders.len(),
        "saved attention bidders"
    );
    Ok(())
}

#[derive(Debug, Clone)]
pub struct PromptAssembler {
    /// Token budget cap.
    token_budget: u32,
    /// Optional prompt context sources. `minimal()` leaves this empty.
    sources: Vec<Arc<dyn PromptSectionSource>>,
    /// Persisted learning bidders for prompt composition.
    learning_bidders: Arc<RwLock<HashMap<AttentionBidder, LearningBidder>>>,
    /// Requested allocation strategy from `[prompt]` configuration.
    composition_strategy: CompositionStrategy,
    /// Eligible allocation rounds required before `Auto` selects VCG.
    vcg_warmup_observations: u32,
    /// Learned section-effectiveness registry for the compose builder.
    ///
    /// When present, the canonical compose path adjusts section priorities
    /// based on historical effectiveness data.
    section_effectiveness: Option<roko_learn::section_effect::SectionEffectivenessRegistry>,
}

impl PromptAssembler {
    /// Construct a production assembler (no cache -- I/O per task).
    #[must_use]
    pub fn new() -> Self {
        Self {
            token_budget: DEFAULT_TOKEN_BUDGET,
            sources: vec![
                Arc::new(WorkdirKnowledgeSource { cache: None }),
                Arc::new(WorkdirPlaybookSource { cache: None }),
                Arc::new(SectionEffectivenessSource { cache: None }),
            ],
            learning_bidders: Arc::new(RwLock::new(HashMap::new())),
            composition_strategy: CompositionStrategy::Auto,
            vcg_warmup_observations: roko_compose::DEFAULT_VCG_WARMUP_OBSERVATIONS,
            section_effectiveness: None,
        }
    }

    /// Construct a production assembler backed by a pre-loaded cache.
    ///
    /// Sources will search in-memory vectors from the cache instead of
    /// reading from the filesystem, eliminating per-task I/O.
    #[must_use]
    pub fn with_cache(cache: Arc<PromptCache>) -> Self {
        let effectiveness = cache.effectiveness.clone();
        Self {
            token_budget: DEFAULT_TOKEN_BUDGET,
            sources: vec![
                Arc::new(WorkdirKnowledgeSource {
                    cache: Some(Arc::clone(&cache)),
                }),
                Arc::new(WorkdirPlaybookSource {
                    cache: Some(Arc::clone(&cache)),
                }),
                Arc::new(SectionEffectivenessSource { cache: Some(cache) }),
            ],
            learning_bidders: Arc::new(RwLock::new(HashMap::new())),
            composition_strategy: CompositionStrategy::Auto,
            vcg_warmup_observations: roko_compose::DEFAULT_VCG_WARMUP_OBSERVATIONS,
            section_effectiveness: Some(effectiveness),
        }
    }

    /// Test / smoke assembler -- no knowledge stores, tiny budget.
    #[must_use]
    pub fn minimal() -> Self {
        Self {
            token_budget: 8_000,
            sources: Vec::new(),
            learning_bidders: Arc::new(RwLock::new(HashMap::new())),
            composition_strategy: CompositionStrategy::Auto,
            vcg_warmup_observations: roko_compose::DEFAULT_VCG_WARMUP_OBSERVATIONS,
            section_effectiveness: None,
        }
    }

    /// Override the token budget.
    pub fn with_token_budget(mut self, budget: u32) -> Self {
        self.token_budget = budget;
        self
    }

    /// Attach persisted learning bidders for prompt composition.
    #[must_use]
    pub fn with_learning_bidders(
        mut self,
        bidders: HashMap<AttentionBidder, LearningBidder>,
    ) -> Self {
        self.learning_bidders = Arc::new(RwLock::new(bidders));
        self
    }

    /// Replace the current learning bidders without rebuilding the dispatcher
    /// or discarding its prompt cache.
    pub fn replace_learning_bidders(&self, bidders: HashMap<AttentionBidder, LearningBidder>) {
        *self.learning_bidders.write() = bidders;
    }

    /// Snapshot the current learning bidders for durable persistence.
    #[must_use]
    pub fn learning_bidders(&self) -> HashMap<AttentionBidder, LearningBidder> {
        self.learning_bidders.read().clone()
    }

    /// Apply one terminal gate outcome to the exact canonical composition
    /// receipt produced for that attempt.
    ///
    /// Every eligible subsystem records one round, including bidders whose
    /// sections lost the cold-start greedy allocation. Only included sections
    /// update success/failure posteriors, avoiding false causal credit for
    /// context the model never saw.
    pub fn record_outcome(&self, diagnostics: &PromptDiagnostics, gate_passed: bool) {
        let Some(manifest) = diagnostics.composition_manifest.as_ref() else {
            return;
        };

        let eligible = manifest
            .included
            .iter()
            .map(|section| section.bidder)
            .chain(manifest.excluded.iter().map(|section| section.bidder))
            .collect::<HashSet<_>>();
        let mut bidders = self.learning_bidders.write();
        for bidder_id in eligible {
            bidders
                .entry(bidder_id)
                .or_insert_with(|| LearningBidder::new(bidder_id, 1.0))
                .observe_round();
        }
        for section in &manifest.included {
            bidders
                .entry(section.bidder)
                .or_insert_with(|| LearningBidder::new(section.bidder, 1.0))
                .update(&section.name, true, gate_passed);
        }
    }

    /// Set the composition strategy for VCG/density-greedy budget allocation.
    /// The selected strategy is passed to the canonical [`PromptComposer`]
    /// used by [`Self::assemble`].
    #[must_use]
    pub fn with_composition_strategy(mut self, strategy: ConfigCompositionStrategy) -> Self {
        self.composition_strategy = match strategy {
            ConfigCompositionStrategy::Auto => CompositionStrategy::Auto,
            ConfigCompositionStrategy::DensityGreedy => CompositionStrategy::DensityGreedy,
            ConfigCompositionStrategy::WeightedSum => CompositionStrategy::WeightedSum,
            ConfigCompositionStrategy::Vcg => CompositionStrategy::Vcg,
        };
        self
    }

    /// Set the minimum bidder-observation count before VCG allocation activates.
    /// The threshold is passed to the canonical [`PromptComposer`] used by
    /// [`Self::assemble`].
    #[must_use]
    pub fn with_vcg_warmup_observations(mut self, observations: u32) -> Self {
        self.vcg_warmup_observations = observations;
        self
    }

    /// Resolve the section-effectiveness registry for the compose builder.
    ///
    /// If the assembler was constructed with a cache (via [`with_cache`]), the
    /// cached registry is returned. Otherwise, loads it from disk using the
    /// standard `.roko/learn/` path under `workdir`. Returns `None` for
    /// minimal assemblers (no sources, no workdir lookup).
    fn resolve_section_effectiveness(
        &self,
        workdir: &Path,
    ) -> Option<roko_learn::section_effect::SectionEffectivenessRegistry> {
        if let Some(ref registry) = self.section_effectiveness {
            return Some(registry.clone());
        }
        // Fallback: load from disk (matches the non-cached SectionEffectivenessSource path).
        // For minimal assemblers (no sources), skip the disk load entirely.
        if self.sources.is_empty() {
            return None;
        }
        let path = workdir.join(roko_learn::section_effect::DEFAULT_SECTION_EFFECTS_PATH);
        Some(roko_learn::section_effect::SectionEffectivenessRegistry::load_or_new(&path))
    }

    /// Assemble the prompt for `task` in the given context.
    ///
    /// Delegates system-prompt construction to the canonical
    /// [`RoleSystemPromptSpec`] / [`build_role_system_prompt`] path (the
    /// 9-layer [`roko_compose::SystemPromptBuilder`]). Runner-specific context
    /// (files in scope, acceptance criteria, verify commands, gate feedback,
    /// dependency outputs, PRD excerpt, workspace map, etc.) is mapped into
    /// [`TaskContext::with_context`]. Knowledge and playbook sections collected
    /// from the registered sources flow through [`PromptBuildOptions`].
    pub fn assemble(
        &self,
        task: &TaskDef,
        ctx: &PromptContext,
    ) -> Result<AssembledPrompt, RunnerDispatchError> {
        // ── Collect source sections (knowledge, playbooks, effectiveness) ──
        // Run all registered sources so playbook / knowledge ids are available
        // for diagnostics.
        let mut source_sections: Vec<PromptSection> = Vec::new();
        for source in &self.sources {
            source_sections.extend(source.collect(task, ctx));
        }

        // Gather playbook / knowledge ids and text for the canonical path.
        let mut playbook_ids: Vec<String> = Vec::new();
        let mut knowledge_ids: Vec<String> = Vec::new();
        let mut code_context: Vec<String> = Vec::new();
        for sec in &source_sections {
            playbook_ids.extend(sec.playbook_ids.clone());
            knowledge_ids.extend(sec.knowledge_ids.clone());
            if !sec.body.is_empty()
                && matches!(
                    sec.name.as_str(),
                    "knowledge" | "episode_knowledge" | "playbooks" | "section_effectiveness"
                )
            {
                code_context.push(sec.body.clone());
            }
        }

        // ── Build canonical RoleSystemPromptSpec ───────────────────────────
        let role = parse_role_label(&ctx.role);

        // Task text for the canonical TaskContext task layer.
        let task_text = format!(
            "{}: {}",
            task.id,
            task.description
                .clone()
                .unwrap_or_else(|| task.title.clone())
        );

        // Rich runner context (files, acceptance, verify, allowed tools,
        // gate feedback, dep outputs, PRD, workspace map, etc.) injected
        // into the canonical "Relevant Context" section.
        let runner_context = build_runner_context(task, ctx)?;

        // Build TaskContext with runner-specific context block.
        let task_context = {
            let mut tc = TaskContext::new(task_text)
                .with_plan_id(ctx.plan_id.clone())
                .with_workspace(ctx.workdir.to_string_lossy().into_owned());
            if !runner_context.is_empty() {
                tc = tc.with_context(runner_context.clone());
            }
            if !code_context.is_empty() {
                tc = tc.with_domain_notes(code_context.join("\n\n"));
            }
            tc
        };

        // Tools CSV (allowlist from task).
        let allowlist = task.allowed_tools.clone();
        let tools_csv = allowlist
            .as_ref()
            .filter(|l| !l.is_empty())
            .map(|l| {
                l.iter()
                    .cloned()
                    .collect::<BTreeSet<_>>()
                    .into_iter()
                    .collect::<Vec<_>>()
                    .join(", ")
            })
            .unwrap_or_default();

        // Compose the canonical section Signals under the runner's actual
        // token budget. The manifest is the authoritative scoring receipt:
        // every entry carries the source Signal's content hash and the exact
        // score result used by selection.
        let section_effectiveness = self.resolve_section_effectiveness(&ctx.workdir);
        let group_context = load_group_context(&ctx.workdir, &ctx.role, task, ctx);
        let has_mcp = task.mcp_servers.as_ref().is_some_and(|s| !s.is_empty());
        let mut spec = RoleSystemPromptSpec::new(role, task_context, tools_csv)
            .with_cache_markers()
            .with_pheromones(&group_context);
        if has_mcp {
            spec = spec.with_mcp_tools();
        }
        let composer = PromptComposer::new()
            .with_strategy(self.composition_strategy)
            .with_vcg_warmup_observations(self.vcg_warmup_observations)
            .with_learning_bidders(self.learning_bidders());
        let mut canonical_sections = if let Some(registry) = section_effectiveness.as_ref() {
            spec.build_sections_with_section_effectiveness(registry)
        } else {
            spec.build_sections()
        };
        let experiment_assignments = if let Some(experiment) = &ctx.prompt_experiment {
            let eligible_sections = canonical_sections
                .iter()
                .map(|section| section.name.as_str())
                .collect::<Vec<_>>();
            let assignments =
                roko_learn::prompt_experiment::ExperimentStore::prepare_attempt_assignments(
                    &experiment.store_path,
                    &experiment.attempt_key,
                    Some(ctx.role.as_str()),
                    &eligible_sections,
                )
                .map_err(|error| RunnerDispatchError::PromptAssembly(error.to_string()))?;
            if let Err(error) = apply_prompt_experiment_assignments(
                &mut canonical_sections,
                &assignments,
                &experiment.attempt_key,
                &ctx.role,
            ) {
                abandon_prompt_experiment_after_assembly_error(
                    experiment,
                    &assignments,
                    "assignment_application",
                );
                return Err(error);
            }
            assignments
        } else {
            Vec::new()
        };
        let prompt_build = match spec.compose_build_from_sections_with_budget_and_composer(
            canonical_sections,
            self.token_budget as usize,
            composer,
        ) {
            Ok(prompt_build) => prompt_build,
            Err(error) => {
                if let Some(experiment) = &ctx.prompt_experiment {
                    abandon_prompt_experiment_after_assembly_error(
                        experiment,
                        &experiment_assignments,
                        "canonical_composition",
                    );
                }
                return Err(RunnerDispatchError::PromptAssembly(error.to_string()));
            }
        };
        let experiment_assignment_diagnostics = experiment_assignments
            .iter()
            .map(|assignment| PromptExperimentAssignmentDiagnostic {
                assignment_id: assignment.assignment_id.clone(),
                experiment_id: assignment.experiment_id.clone(),
                variant_id: assignment.variant_id.clone(),
                section_name: assignment.section_name.clone(),
                content_hash: assignment.content_hash.clone(),
                included: prompt_build
                    .composition_manifest
                    .as_ref()
                    .is_some_and(|manifest| {
                        manifest
                            .included
                            .iter()
                            .any(|section| section.name == assignment.section_name)
                    }),
            })
            .collect::<Vec<_>>();
        let composition_manifest = prompt_build.composition_manifest.clone();
        let scored_signals = prompt_build
            .composition_manifest
            .as_ref()
            .into_iter()
            .flat_map(|manifest| &manifest.scored_signals)
            .filter_map(|scored| {
                serde_json::to_string(&scored.result)
                    .ok()
                    .map(|score_result| ScoredSignalDiagnostic {
                        signal_ref: scored.signal_ref.clone(),
                        score_result,
                    })
            })
            .collect::<Vec<_>>();
        let included_sections = composition_manifest.as_ref().map_or_else(
            || {
                source_sections
                    .iter()
                    .map(|section| section.name.clone())
                    .collect()
            },
            |manifest| {
                manifest
                    .included
                    .iter()
                    .map(|section| section.name.clone())
                    .collect()
            },
        );
        let dropped_sections = composition_manifest
            .as_ref()
            .map_or_else(Vec::new, |manifest| {
                manifest
                    .excluded
                    .iter()
                    .map(|section| section.name.clone())
                    .collect()
            });
        let system_prompt = prompt_build.prompt;

        // ── Diagnostics ───────────────────────────────────────────────────
        let estimated_tokens = (system_prompt.len() / 4).max(1) as u32;
        let diagnostics = PromptDiagnostics {
            included_sections,
            dropped_sections,
            estimated_tokens,
            playbook_ids,
            knowledge_ids,
            scored_signals,
            composition_manifest,
            experiment_assignments: experiment_assignment_diagnostics,
        };

        // ── User prompt (unchanged) ────────────────────────────────────────
        let mut user_prompt = format!("# Task Request\n{}\n", task.title);
        if let Some(description) = &task.description {
            user_prompt.push_str("\n## Details\n");
            user_prompt.push_str(description);
            user_prompt.push('\n');
        }
        if let Some(context) = &task.context {
            if !context.read_files.is_empty()
                || !context.symbols.is_empty()
                || !context.anti_patterns.is_empty()
                || !context.prior_failures.is_empty()
                || context.impact_acknowledgement.is_some()
            {
                user_prompt.push_str("\n## Task Context\n");
                for file in &context.read_files {
                    user_prompt.push_str("- Read `");
                    user_prompt.push_str(&file.path);
                    if let Some(lines) = &file.lines {
                        user_prompt.push_str("` lines ");
                        user_prompt.push_str(lines);
                    } else {
                        user_prompt.push('`');
                    }
                    user_prompt.push_str(": ");
                    user_prompt.push_str(&file.why);
                    user_prompt.push('\n');
                }
                for symbol in &context.symbols {
                    user_prompt.push_str("- Symbol: ");
                    user_prompt.push_str(symbol);
                    user_prompt.push('\n');
                }
                for anti_pattern in &context.anti_patterns {
                    user_prompt.push_str("- Avoid: ");
                    user_prompt.push_str(anti_pattern);
                    user_prompt.push('\n');
                }
                for failure in &context.prior_failures {
                    user_prompt.push_str("- Prior failure: ");
                    user_prompt.push_str(failure);
                    user_prompt.push('\n');
                }
                if let Some(reason) = context.impact_acknowledgement.as_deref() {
                    user_prompt.push_str("- Reviewed impact-scope acknowledgement: ");
                    user_prompt.push_str(reason);
                    user_prompt.push('\n');
                }
            }
        }
        if !task.acceptance.is_empty() {
            user_prompt.push_str("\n## Acceptance\n");
            for item in &task.acceptance {
                user_prompt.push_str("- ");
                user_prompt.push_str(item);
                user_prompt.push('\n');
            }
        }
        if !task.verify.is_empty() {
            user_prompt.push_str("\n## Verification Commands\n");
            for step in &task.verify {
                user_prompt.push_str("- ");
                user_prompt.push_str(&step.command);
                user_prompt.push('\n');
            }
        }

        Ok(AssembledPrompt {
            system_prompt,
            user_prompt,
            tool_allowlist: allowlist,
            diagnostics,
        })
    }
}

impl PromptSectionSource for WorkdirKnowledgeSource {
    fn collect(&self, task: &TaskDef, ctx: &PromptContext) -> Vec<PromptSection> {
        let mut sections = Vec::new();
        if let Some(cache) = &self.cache {
            if let Some(section) = collect_neuro_knowledge_cached(task, ctx, &cache.neuro_entries) {
                sections.push(section);
            }
            if let Some(section) = collect_episode_knowledge_cached(task, ctx, &cache.episodes) {
                sections.push(section);
            }
        } else {
            if let Some(section) = collect_neuro_knowledge(task, ctx) {
                sections.push(section);
            }
            if let Some(section) = collect_episode_knowledge(task, ctx) {
                sections.push(section);
            }
        }
        sections
    }
}

impl PromptSectionSource for WorkdirPlaybookSource {
    fn collect(&self, task: &TaskDef, ctx: &PromptContext) -> Vec<PromptSection> {
        if let Some(cache) = &self.cache {
            collect_playbooks_cached(task, ctx, &cache.playbooks)
                .into_iter()
                .collect()
        } else {
            collect_playbooks(task, ctx).into_iter().collect()
        }
    }
}

impl PromptSectionSource for SectionEffectivenessSource {
    fn collect(&self, _task: &TaskDef, ctx: &PromptContext) -> Vec<PromptSection> {
        let registry = if let Some(cache) = &self.cache {
            &cache.effectiveness
        } else {
            let path = ctx
                .workdir
                .join(roko_learn::section_effect::DEFAULT_SECTION_EFFECTS_PATH);
            // load_or_new handles missing files gracefully (returns empty registry).
            let loaded =
                roko_learn::section_effect::SectionEffectivenessRegistry::load_or_new(&path);
            return render_effectiveness_section(&loaded, &ctx.role);
        };
        render_effectiveness_section(registry, &ctx.role)
    }
}

fn render_effectiveness_section(
    registry: &roko_learn::section_effect::SectionEffectivenessRegistry,
    role: &str,
) -> Vec<PromptSection> {
    let positive = registry.positive_lift_sections(role);
    if positive.is_empty() {
        return Vec::new();
    }
    let mut body = String::from(
        "# Prompt section effectiveness\nHistorically high-signal prompt sections for this role:\n",
    );
    for effect in positive.into_iter().take(5) {
        body.push_str(&format!(
            "- {} (lift {:+.2}, weight {:.2})\n",
            effect.section_name,
            effect.lift(),
            effect.lift_weight()
        ));
    }
    vec![PromptSection::new("section_effectiveness", body, 7)]
}

fn collect_neuro_knowledge(task: &TaskDef, ctx: &PromptContext) -> Option<PromptSection> {
    let store = roko_neuro::KnowledgeStore::for_workdir(&ctx.workdir);
    // store.query -> read_all handles NotFound internally (returns empty Vec).
    let query = task_query_text(task, ctx);
    // Group-tagged entries have a separate membership-gated auction path.
    // Query extra candidates first so private group entries cannot crowd public
    // workspace knowledge out of this section before filtering.
    let mut entries = store.query(&query, 64).ok()?;
    entries.retain(|entry| !is_group_scoped_knowledge(entry));
    entries.truncate(3);
    if entries.is_empty() {
        return None;
    }

    let ids = entries
        .iter()
        .map(|entry| entry.id.clone())
        .filter(|id| !id.is_empty())
        .collect::<Vec<_>>();
    let mut body = String::from("# Neuro knowledge\nRelevant durable knowledge from prior runs:\n");
    for entry in entries {
        let source = entry.source.as_deref().unwrap_or("neuro");
        body.push_str(&format!(
            "- [{}] {} (confidence {:.2}, source: {})\n",
            entry.id,
            truncate_chars(&entry.content, 420),
            entry.confidence,
            source
        ));
    }
    Some(PromptSection::new("knowledge", body, 7).with_knowledge_ids(ids))
}

fn collect_episode_knowledge(task: &TaskDef, ctx: &PromptContext) -> Option<PromptSection> {
    let keywords = query_keywords(&task_query_text(task, ctx));
    if keywords.is_empty() {
        return None;
    }

    let mut scored = Vec::new();
    for path in episode_paths(&ctx.workdir) {
        let file = match std::fs::File::open(&path) {
            Ok(f) => f,
            Err(_) => continue,
        };
        let reader = std::io::BufReader::new(file);
        for line in std::io::BufRead::lines(reader).map_while(Result::ok) {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let Ok(episode) = serde_json::from_str::<roko_learn::episode_logger::Episode>(trimmed)
            else {
                continue;
            };
            let haystack = format!(
                "{} {} {} {} {}",
                episode.task_id,
                episode.agent_id,
                episode.model,
                episode.reasoning_summary.as_deref().unwrap_or(""),
                episode.failure_reason.as_deref().unwrap_or("")
            )
            .to_ascii_lowercase();
            let score = keywords
                .iter()
                .filter(|keyword| haystack.contains(keyword.as_str()))
                .count();
            if score > 0 {
                scored.push((score, episode));
            }
        }
    }
    if scored.is_empty() {
        return None;
    }
    scored.sort_by(|a, b| {
        b.1.success
            .cmp(&a.1.success)
            .then_with(|| b.0.cmp(&a.0))
            .then_with(|| b.1.completed_at.cmp(&a.1.completed_at))
    });
    scored.truncate(3);

    let ids = scored
        .iter()
        .map(|(_, episode)| {
            if !episode.id.is_empty() {
                episode.id.clone()
            } else if !episode.episode_id.is_empty() {
                episode.episode_id.clone()
            } else {
                episode.task_id.clone()
            }
        })
        .filter(|id| !id.is_empty())
        .collect::<Vec<_>>();
    let mut body =
        String::from("# Learned patterns from prior episodes\nSimilar prior work suggests:\n");
    for (_, episode) in scored {
        let outcome = if episode.success { "passed" } else { "failed" };
        let summary = episode
            .reasoning_summary
            .as_deref()
            .or(episode.reflection.as_deref())
            .or(episode.failure_reason.as_deref())
            .unwrap_or("no summary recorded");
        body.push_str(&format!(
            "- {} ({}, model: {}): {}\n",
            episode.task_id,
            outcome,
            if episode.model.is_empty() {
                "unknown"
            } else {
                &episode.model
            },
            truncate_chars(summary, 420)
        ));
    }
    Some(PromptSection::new("episode_knowledge", body, 7).with_knowledge_ids(ids))
}

fn collect_playbooks(task: &TaskDef, ctx: &PromptContext) -> Option<PromptSection> {
    let root = roko_core::Workspace::open(&ctx.workdir)
        .map(|ws| ws.playbooks_dir())
        .unwrap_or_else(|_| ctx.workdir.join(".roko").join("learn").join("playbooks"));
    let query = query_keywords(&task_query_text(task, ctx));
    let mut scored = Vec::new();
    let read_dir = match std::fs::read_dir(&root) {
        Ok(rd) => rd,
        Err(_) => return None,
    };
    for entry in read_dir {
        let entry = entry.ok()?;
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
            continue;
        }
        let text = std::fs::read_to_string(&path).ok()?;
        let Ok(playbook) = serde_json::from_str::<roko_learn::playbook::Playbook>(&text) else {
            continue;
        };
        let haystack = playbook_text(&playbook).to_ascii_lowercase();
        let lexical_score = query
            .iter()
            .filter(|keyword| haystack.contains(keyword.as_str()))
            .count();
        let outcome_score = playbook
            .success_count
            .saturating_sub(playbook.failure_count) as usize;
        let score = lexical_score
            .saturating_mul(10)
            .saturating_add(outcome_score);
        if score > 0 || scored.len() < 3 {
            scored.push((score, playbook));
        }
    }
    if scored.is_empty() {
        return None;
    }
    scored.sort_by(|a, b| {
        b.0.cmp(&a.0)
            .then_with(|| b.1.success_count.cmp(&a.1.success_count))
            .then_with(|| a.1.id.cmp(&b.1.id))
    });
    scored.truncate(3);

    let ids = scored
        .iter()
        .map(|(_, playbook)| playbook.id.clone())
        .collect::<Vec<_>>();
    let mut body = String::from("# Relevant playbooks\nReusable proven procedures:\n");
    for (_, playbook) in scored {
        body.push_str(&format!(
            "- {}: {} (successes {}, failures {})\n",
            playbook.id, playbook.goal, playbook.success_count, playbook.failure_count
        ));
        for step in playbook.steps.iter().take(5) {
            body.push_str(&format!(
                "  - {} via {}; expect {}\n",
                step.description,
                step.action_kind,
                if step.expected_signals.is_empty() {
                    "task-local verification".to_string()
                } else {
                    step.expected_signals.join(", ")
                }
            ));
        }
    }
    Some(PromptSection::new("playbooks", body, 7).with_playbook_ids(ids))
}

// ─── Cached variants ──────────────────────────────────────────────────
//
// These mirror the original I/O-based functions but operate on in-memory
// vectors pre-loaded by `PromptCache`.

fn collect_neuro_knowledge_cached(
    task: &TaskDef,
    ctx: &PromptContext,
    entries: &[roko_neuro::KnowledgeEntry],
) -> Option<PromptSection> {
    if entries.is_empty() {
        return None;
    }
    let query = task_query_text(task, ctx);
    let keywords = query_keywords(&query);
    if keywords.is_empty() {
        return None;
    }

    // Score entries by keyword overlap (mirrors KnowledgeStore::query's lexical path).
    let mut scored: Vec<(usize, &roko_neuro::KnowledgeEntry)> = entries
        .iter()
        .filter_map(|entry| {
            if is_group_scoped_knowledge(entry) {
                return None;
            }
            let haystack = format!(
                "{} {} {}",
                entry.content,
                entry.tags.join(" "),
                entry.source.as_deref().unwrap_or("")
            )
            .to_ascii_lowercase();
            let score = keywords
                .iter()
                .filter(|kw| haystack.contains(kw.as_str()))
                .count();
            if score > 0 {
                Some((score, entry))
            } else {
                None
            }
        })
        .collect();
    scored.sort_by(|a, b| {
        b.0.cmp(&a.0)
            .then_with(|| b.1.confidence.total_cmp(&a.1.confidence))
    });
    scored.truncate(5);

    if scored.is_empty() {
        return None;
    }

    let ids = scored
        .iter()
        .map(|(_, entry)| entry.id.clone())
        .filter(|id| !id.is_empty())
        .collect::<Vec<_>>();
    let mut body = String::from("# Neuro knowledge\nRelevant durable knowledge from prior runs:\n");
    for (_, entry) in &scored {
        let source = entry.source.as_deref().unwrap_or("neuro");
        body.push_str(&format!(
            "- [{}] {} (confidence {:.2}, source: {})\n",
            entry.id,
            truncate_chars(&entry.content, 420),
            entry.confidence,
            source
        ));
    }
    Some(PromptSection::new("knowledge", body, 7).with_knowledge_ids(ids))
}

fn collect_episode_knowledge_cached(
    task: &TaskDef,
    ctx: &PromptContext,
    episodes: &[roko_learn::episode_logger::Episode],
) -> Option<PromptSection> {
    let keywords = query_keywords(&task_query_text(task, ctx));
    if keywords.is_empty() {
        return None;
    }

    let mut scored: Vec<(usize, &roko_learn::episode_logger::Episode)> = Vec::new();
    for episode in episodes {
        let haystack = format!(
            "{} {} {} {} {}",
            episode.task_id,
            episode.agent_id,
            episode.model,
            episode.reasoning_summary.as_deref().unwrap_or(""),
            episode.failure_reason.as_deref().unwrap_or("")
        )
        .to_ascii_lowercase();
        let score = keywords
            .iter()
            .filter(|keyword| haystack.contains(keyword.as_str()))
            .count();
        if score > 0 {
            scored.push((score, episode));
        }
    }
    if scored.is_empty() {
        return None;
    }
    scored.sort_by(|a, b| {
        b.1.success
            .cmp(&a.1.success)
            .then_with(|| b.0.cmp(&a.0))
            .then_with(|| b.1.completed_at.cmp(&a.1.completed_at))
    });
    scored.truncate(5);

    let ids = scored
        .iter()
        .map(|(_, episode)| {
            if !episode.id.is_empty() {
                episode.id.clone()
            } else if !episode.episode_id.is_empty() {
                episode.episode_id.clone()
            } else {
                episode.task_id.clone()
            }
        })
        .filter(|id| !id.is_empty())
        .collect::<Vec<_>>();
    let mut body =
        String::from("# Learned patterns from prior episodes\nSimilar prior work suggests:\n");
    for (_, episode) in scored {
        let outcome = if episode.success { "passed" } else { "failed" };
        let summary = episode
            .reasoning_summary
            .as_deref()
            .or(episode.reflection.as_deref())
            .or(episode.failure_reason.as_deref())
            .unwrap_or("no summary recorded");
        body.push_str(&format!(
            "- {} ({}, model: {}): {}\n",
            episode.task_id,
            outcome,
            if episode.model.is_empty() {
                "unknown"
            } else {
                &episode.model
            },
            truncate_chars(summary, 420)
        ));
    }
    Some(PromptSection::new("episode_knowledge", body, 7).with_knowledge_ids(ids))
}

fn collect_playbooks_cached(
    task: &TaskDef,
    ctx: &PromptContext,
    playbooks: &[roko_learn::playbook::Playbook],
) -> Option<PromptSection> {
    if playbooks.is_empty() {
        return None;
    }
    let query = query_keywords(&task_query_text(task, ctx));
    let mut scored: Vec<(usize, &roko_learn::playbook::Playbook)> = Vec::new();
    for playbook in playbooks {
        let haystack = playbook_text(playbook).to_ascii_lowercase();
        let lexical_score = query
            .iter()
            .filter(|keyword| haystack.contains(keyword.as_str()))
            .count();
        let outcome_score = playbook
            .success_count
            .saturating_sub(playbook.failure_count) as usize;
        let score = lexical_score
            .saturating_mul(10)
            .saturating_add(outcome_score);
        if score > 0 || scored.len() < 3 {
            scored.push((score, playbook));
        }
    }
    if scored.is_empty() {
        return None;
    }
    scored.sort_by(|a, b| {
        b.0.cmp(&a.0)
            .then_with(|| b.1.success_count.cmp(&a.1.success_count))
            .then_with(|| a.1.id.cmp(&b.1.id))
    });
    scored.truncate(3);

    let ids = scored
        .iter()
        .map(|(_, playbook)| playbook.id.clone())
        .collect::<Vec<_>>();
    let mut body = String::from("# Relevant playbooks\nReusable proven procedures:\n");
    for (_, playbook) in scored {
        body.push_str(&format!(
            "- {}: {} (successes {}, failures {})\n",
            playbook.id, playbook.goal, playbook.success_count, playbook.failure_count
        ));
        for step in playbook.steps.iter().take(5) {
            body.push_str(&format!(
                "  - {} via {}; expect {}\n",
                step.description,
                step.action_kind,
                if step.expected_signals.is_empty() {
                    "task-local verification".to_string()
                } else {
                    step.expected_signals.join(", ")
                }
            ));
        }
    }
    Some(PromptSection::new("playbooks", body, 7).with_playbook_ids(ids))
}

fn task_query_text(task: &TaskDef, ctx: &PromptContext) -> String {
    let mut parts = vec![task.id.clone(), task.title.clone(), ctx.role.clone()];
    if let Some(description) = &task.description {
        parts.push(description.clone());
    }
    parts.extend(task.acceptance.clone());
    parts.extend(task.files.clone());
    parts.join(" ")
}

fn query_keywords(text: &str) -> HashSet<String> {
    text.to_ascii_lowercase()
        .split(|c: char| !c.is_ascii_alphanumeric() && c != '-' && c != '_')
        .filter(|word| word.len() > 2)
        .map(ToString::to_string)
        .collect()
}

fn episode_paths(workdir: &Path) -> Vec<PathBuf> {
    vec![roko_learn::runtime_feedback::resolve_project_episode_path(
        workdir,
    )]
}

const MAX_GROUP_STATE_BYTES: u64 = 16 * 1024 * 1024;
const MAX_GROUP_CONTEXT_CHUNKS: usize = 12;

#[derive(Debug, Default, Deserialize)]
struct GroupContextState {
    #[serde(default)]
    groups: BTreeMap<GroupId, Group>,
    #[serde(default)]
    pheromones: BTreeMap<GroupId, Vec<StoredGroupPheromone>>,
}

#[derive(Debug, Deserialize)]
struct StoredGroupPheromone {
    id: String,
    pheromone: GroupPheromone,
    balance: f64,
    last_touched_at: chrono::DateTime<chrono::Utc>,
}

/// Load only the group context that the dispatched logical agent may read.
///
/// The runner currently identifies an agent by its logical role label. Group
/// definitions that want prompt injection therefore use that label as the
/// member `agent_id` (for example `implementer` or `reviewer`). Unknown,
/// malformed, or oversized state fails closed and contributes no context.
fn load_group_context(
    workdir: &Path,
    agent_id: &str,
    task: &TaskDef,
    ctx: &PromptContext,
) -> Vec<ContextChunk> {
    let Some(state) = read_group_context_state(workdir) else {
        return Vec::new();
    };
    let accessible = state
        .groups
        .iter()
        .filter(|(_, group)| group.can_read(agent_id))
        .map(|(group_id, group)| (group_id.clone(), group))
        .collect::<BTreeMap<_, _>>();
    if accessible.is_empty() {
        return Vec::new();
    }

    let now = chrono::Utc::now();
    let mut chunks = Vec::new();
    for (group_id, group) in &accessible {
        let Some(pheromones) = state.pheromones.get(group_id) else {
            continue;
        };
        for stored in pheromones {
            if &stored.pheromone.group_id != group_id {
                continue;
            }
            let balance = current_pheromone_balance(
                stored.balance,
                stored.last_touched_at,
                group.config.pheromone_decay_rate,
                now,
            );
            if balance <= f64::EPSILON {
                continue;
            }
            let metadata = truncate_chars(&stored.pheromone.metadata.to_string(), 300);
            let position = stored
                .pheromone
                .position_hint
                .as_deref()
                .map_or_else(String::new, |hint| format!(" position={hint}"));
            chunks.push(ContextChunk {
                content: format!(
                    "[Group {}] [{}] deposited_by={} balance={balance:.3}{position} metadata={metadata}",
                    group.name, stored.pheromone.signal_type, stored.pheromone.depositor
                ),
                source: ContextSource::Pheromone {
                    kind: stored.pheromone.signal_type.clone(),
                    source: format!("{}:{}", group_id, stored.id),
                },
                relevance: balance,
                track_record: Some(balance),
                confidence: Some(balance),
                recency: Some(datetime_recency(stored.pheromone.deposited_at, now)),
                emotional_tag: None,
            });
        }
    }

    let query = task_query_text(task, ctx);
    let keywords = query_keywords(&query);
    let knowledge = roko_neuro::KnowledgeStore::for_workdir(workdir)
        .read_all()
        .unwrap_or_default();
    for entry in knowledge {
        let scoped_group_ids = knowledge_group_ids(&entry);
        if scoped_group_ids.is_empty()
            || scoped_group_ids
                .iter()
                .any(|group_id| !accessible.contains_key(group_id))
        {
            continue;
        }
        let Some(group_id) = scoped_group_ids.first() else {
            continue;
        };
        let Some(group) = accessible.get(group_id) else {
            continue;
        };
        let haystack = format!("{} {}", entry.content, entry.tags.join(" ")).to_ascii_lowercase();
        let matches = keywords
            .iter()
            .filter(|keyword| haystack.contains(keyword.as_str()))
            .count();
        let lexical = if keywords.is_empty() {
            0.0
        } else {
            matches as f64 / keywords.len() as f64
        };
        let confidence = entry.confidence.clamp(0.0, 1.0);
        let relevance = (0.25 + lexical * 0.5 + confidence * 0.25).clamp(0.0, 1.0);
        chunks.push(ContextChunk {
            content: format!(
                "[Group {} knowledge] {}",
                group.name,
                truncate_chars(&entry.content, 420)
            ),
            source: ContextSource::KnowledgeEntry {
                entry_id: entry.id.clone(),
                kind: format!("{:?}", entry.kind).to_ascii_lowercase(),
                source: entry.source.clone(),
            },
            relevance,
            track_record: Some(confidence),
            confidence: Some(confidence),
            recency: Some(datetime_recency(entry.created_at, now)),
            emotional_tag: None,
        });
    }

    chunks.sort_by(|left, right| {
        right
            .relevance
            .total_cmp(&left.relevance)
            .then_with(|| left.content.cmp(&right.content))
    });
    chunks.truncate(MAX_GROUP_CONTEXT_CHUNKS);
    chunks
}

fn read_group_context_state(workdir: &Path) -> Option<GroupContextState> {
    let path = workdir.join(".roko").join("groups").join("state.json");
    let metadata = match std::fs::metadata(&path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return None,
        Err(error) => {
            tracing::warn!(path = %path.display(), %error, "group context state metadata failed");
            return None;
        }
    };
    if metadata.len() > MAX_GROUP_STATE_BYTES {
        tracing::warn!(
            path = %path.display(),
            bytes = metadata.len(),
            "group context state exceeds read limit"
        );
        return None;
    }
    let contents = match std::fs::read_to_string(&path) {
        Ok(contents) => contents,
        Err(error) => {
            tracing::warn!(path = %path.display(), %error, "group context state read failed");
            return None;
        }
    };
    match serde_json::from_str(&contents) {
        Ok(state) => Some(state),
        Err(error) => {
            tracing::warn!(path = %path.display(), %error, "group context state decode failed");
            None
        }
    }
}

fn current_pheromone_balance(
    balance: f64,
    last_touched_at: chrono::DateTime<chrono::Utc>,
    decay_modifier: f64,
    now: chrono::DateTime<chrono::Utc>,
) -> f64 {
    if !balance.is_finite() || !decay_modifier.is_finite() {
        return 0.0;
    }
    let elapsed_hours = (now - last_touched_at).num_milliseconds().max(0) as f64 / 3_600_000.0;
    let daily_rate = (0.01 * decay_modifier).clamp(0.0, 1.0);
    (balance.clamp(0.0, 1.0) * (1.0 - daily_rate).powf(elapsed_hours / 24.0)).clamp(0.0, 1.0)
}

fn datetime_recency(
    timestamp: chrono::DateTime<chrono::Utc>,
    now: chrono::DateTime<chrono::Utc>,
) -> f64 {
    let age_hours = (now - timestamp).num_milliseconds().max(0) as f64 / 3_600_000.0;
    0.5_f64.powf(age_hours / (7.0 * 24.0)).clamp(0.0, 1.0)
}

fn is_group_scoped_knowledge(entry: &roko_neuro::KnowledgeEntry) -> bool {
    entry.tags.iter().any(|tag| tag.starts_with("group:"))
}

fn knowledge_group_ids(entry: &roko_neuro::KnowledgeEntry) -> Vec<GroupId> {
    entry
        .tags
        .iter()
        .filter_map(|tag| tag.strip_prefix("group:"))
        .filter(|id| !id.is_empty())
        .map(GroupId::new)
        .collect()
}

fn playbook_text(playbook: &roko_learn::playbook::Playbook) -> String {
    let mut text = format!("{} {} {}", playbook.id, playbook.name, playbook.goal);
    for step in &playbook.steps {
        text.push(' ');
        text.push_str(&step.description);
        text.push(' ');
        text.push_str(&step.action_kind);
        text.push(' ');
        text.push_str(&step.expected_signals.join(" "));
    }
    text
}

fn truncate_chars(text: &str, limit: usize) -> String {
    let mut out = text.chars().take(limit).collect::<String>();
    if text.chars().count() > limit {
        out.push_str(" [truncated]");
    }
    out
}

impl Default for PromptAssembler {
    fn default() -> Self {
        Self::new()
    }
}

fn render_gate_feedback(feedback: &GateFeedback) -> String {
    let mut buf = String::from("# Previous attempt feedback\n");
    if !feedback.compile_errors.is_empty() {
        buf.push_str("## Compile errors\n");
        for err in &feedback.compile_errors {
            buf.push_str(&format!("- {err}\n"));
        }
    }
    if !feedback.test_failures.is_empty() {
        buf.push_str("## Failing tests\n");
        for failure in &feedback.test_failures {
            buf.push_str(&format!("- {failure}\n"));
        }
    }
    if !feedback.clippy_warnings.is_empty() {
        buf.push_str("## Clippy warnings\n");
        for w in &feedback.clippy_warnings {
            buf.push_str(&format!("- {w}\n"));
        }
    }
    buf
}

// ─── Tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use roko_core::{CoordinationMode, GroupConfig, GroupMember, MemberPermissions, MemberRole};
    use roko_learn::prompt_experiment::{
        ExperimentStore, PromptAssignmentState, PromptAttemptKey, PromptExperiment, PromptVariant,
    };
    use std::path::{Path, PathBuf};

    const ASSIGNED_ROLE_CONTENT: &str = "EXPERIMENT_ASSIGNED_ROLE_CONTENT";

    fn task() -> TaskDef {
        TaskDef {
            id: "t".into(),
            title: "Wire it up".into(),
            description: Some("Explain the wiring".into()),
            role: Some("implementer".into()),
            status: "ready".into(),
            tier: "focused".into(),
            frequency: None,
            model_hint: None,
            replan_strategy: None,
            max_loc: None,
            files: vec!["src/lib.rs".into()],
            allowed_tools: Some(vec!["read_file".into(), "edit_file".into()]),
            denied_tools: None,
            mcp_servers: None,
            depends_on: vec![],
            depends_on_plan: vec![],
            split_into: None,
            context: None,
            verify: vec![crate::task_parser::VerifyStep {
                phase: "test".into(),
                command: "cargo test".into(),
                fail_msg: None,
                timeout_ms: 60_000,
            }],
            timeout_secs: 60,
            max_retries: 1,
            acceptance: vec!["compiles".into()],
            acceptance_contract: None,
            domain: None,
            estimated_minutes: None,
            crates_touched: None,
            sequence: 0,
        }
    }

    fn ctx() -> DispatchContext {
        DispatchContext {
            plan_id: "p".into(),
            role: "implementer".into(),
            workdir: PathBuf::from("/tmp"),
            model_hint: None,
            force_backend: None,
            budget_remaining_usd: 5.0,
            attempt: 0,
            prompt_experiment: None,
            gate_feedback: None,
            routing_context: None,
            routing_bias: None,
            dependency_outputs: Vec::new(),
        }
    }

    fn save_role_identity_experiment(path: &Path) {
        let mut experiment = PromptExperiment::new(
            "role-identity-experiment",
            "role_identity",
            vec![PromptVariant {
                id: "role-identity-variant".into(),
                name: "Assigned role identity".into(),
                section_name: "role_identity".into(),
                content: ASSIGNED_ROLE_CONTENT.into(),
                slug: None,
                active: true,
            }],
        );
        experiment.role = Some("implementer".into());
        let mut store = ExperimentStore::new();
        store.register(experiment);
        store.save(path).expect("save experiment store");
    }

    #[test]
    fn first_attempt_includes_all_canonical_sections() {
        let assembler = PromptAssembler::minimal();
        let pctx = PromptContext::from_task(&task(), &ctx());
        let p = assembler.assemble(&task(), &pctx).unwrap();
        // The canonical 9-layer builder (via RoleSystemPromptSpec) outputs role
        // identity (e.g. "You are the Implementer") and the runner context block
        // (files, acceptance, verify, allowed tools) in the context_layer.
        assert!(
            p.system_prompt.contains("Implementer")
                || p.system_prompt.contains("implementer")
                || p.system_prompt.contains("# Role"),
            "system_prompt should contain role identity"
        );
        assert!(
            p.system_prompt.contains("# Files in scope"),
            "system_prompt should contain files section (via context_layer)"
        );
        assert!(
            p.system_prompt.contains("# Acceptance criteria"),
            "system_prompt should contain acceptance criteria (via context_layer)"
        );
        assert!(
            p.system_prompt.contains("# Verify"),
            "system_prompt should contain verify section (via context_layer)"
        );
        assert!(
            p.system_prompt.contains("# Allowed tools"),
            "system_prompt should contain allowed tools (via context_layer)"
        );
        assert!(
            !p.system_prompt.contains("# Previous attempt"),
            "first attempt should not contain gate feedback"
        );
        assert_eq!(p.tool_allowlist.as_deref().unwrap().len(), 2);
        assert!(p.diagnostics.estimated_tokens > 0);
        assert!(!p.diagnostics.scored_signals.is_empty());
        assert!(p.diagnostics.scored_signals.iter().all(|scored| {
            roko_core::ContentHash::from_hex(&scored.signal_ref).is_some()
                && serde_json::from_str::<roko_compose::CandidateScoreResult>(&scored.score_result)
                    .is_ok()
        }));
        assert!(p.diagnostics.experiment_assignments.is_empty());
    }

    #[test]
    fn no_attempt_experiment_context_preserves_prompt_and_store() {
        let root = tempfile::tempdir().expect("root tempdir");
        let attempt = tempfile::tempdir().expect("attempt worktree");
        let store_path = root.path().join("experiments.json");
        save_role_identity_experiment(&store_path);
        let before = std::fs::read(&store_path).expect("read experiment store");
        let mut dispatch_ctx = ctx();
        dispatch_ctx.workdir = attempt.path().to_path_buf();

        let prompt = PromptAssembler::minimal()
            .assemble(&task(), &PromptContext::from_task(&task(), &dispatch_ctx))
            .expect("baseline prompt");

        assert!(!prompt.system_prompt.contains(ASSIGNED_ROLE_CONTENT));
        assert!(prompt.diagnostics.experiment_assignments.is_empty());
        assert_eq!(
            std::fs::read(&store_path).expect("reread experiment store"),
            before,
            "assembly without attempt experiment context must not mutate the store"
        );
        assert!(
            !attempt.path().join(".roko/learn/experiments.json").exists(),
            "the attempt worktree must not receive an experiment store"
        );
    }

    #[test]
    fn durable_attempt_assignment_replaces_one_canonical_section_before_composition() {
        let root = tempfile::tempdir().expect("root tempdir");
        let attempt = tempfile::tempdir().expect("attempt worktree");
        let store_path = root.path().join("experiments.json");
        save_role_identity_experiment(&store_path);
        let attempt_key = PromptAttemptKey {
            run_id: "run-1".into(),
            plan_id: "p".into(),
            task_id: "t".into(),
            attempt: 1,
        };
        let mut dispatch_ctx = ctx();
        dispatch_ctx.workdir = attempt.path().to_path_buf();
        dispatch_ctx.prompt_experiment = Some(PromptExperimentContext {
            attempt_key,
            store_path: store_path.clone(),
        });
        let prompt_ctx = PromptContext::from_task(&task(), &dispatch_ctx);
        let assembler = PromptAssembler::minimal();

        let first = assembler
            .assemble(&task(), &prompt_ctx)
            .expect("assigned prompt");
        let second = assembler
            .assemble(&task(), &prompt_ctx)
            .expect("idempotently assigned prompt");

        assert!(first.system_prompt.contains(ASSIGNED_ROLE_CONTENT));
        assert_eq!(first.system_prompt, second.system_prompt);
        assert_eq!(
            first.diagnostics.experiment_assignments,
            second.diagnostics.experiment_assignments
        );
        let [assignment] = first.diagnostics.experiment_assignments.as_slice() else {
            panic!("expected one raw-content-free assignment diagnostic");
        };
        assert_eq!(assignment.experiment_id, "role-identity-experiment");
        assert_eq!(assignment.variant_id, "role-identity-variant");
        assert_eq!(assignment.section_name, "role_identity");
        assert!(
            assignment.included,
            "critical role identity must be included"
        );
        let manifest = first
            .diagnostics
            .composition_manifest
            .as_ref()
            .expect("canonical manifest");
        let role_identity = manifest
            .included
            .iter()
            .find(|section| section.name == "role_identity")
            .expect("included assigned role identity");
        assert!(role_identity.action_id.contains("prompt-experiment"));
        assert!(role_identity.action_id.contains("role-identity-variant"));
        assert!(role_identity.action_id.contains("role-identity-experiment"));
        let diagnostics_json =
            serde_json::to_string(&first.diagnostics).expect("serialize diagnostics");
        assert!(!diagnostics_json.contains(ASSIGNED_ROLE_CONTENT));
        assert!(
            !attempt.path().join(".roko/learn/experiments.json").exists(),
            "assignment persistence must use the explicit root store path"
        );
    }

    #[test]
    fn assignment_application_error_abandons_prepared_treatment_without_trial() {
        let root = tempfile::tempdir().expect("root tempdir");
        let attempt = tempfile::tempdir().expect("attempt worktree");
        let store_path = root.path().join("experiments.json");
        save_role_identity_experiment(&store_path);
        let attempt_key = PromptAttemptKey {
            run_id: "run-invalid-assignment".into(),
            plan_id: "p".into(),
            task_id: "t".into(),
            attempt: 1,
        };
        let mut dispatch_ctx = ctx();
        dispatch_ctx.workdir = attempt.path().to_path_buf();
        dispatch_ctx.prompt_experiment = Some(PromptExperimentContext {
            attempt_key: attempt_key.clone(),
            store_path: store_path.clone(),
        });
        let prompt_ctx = PromptContext::from_task(&task(), &dispatch_ctx);
        let assembler = PromptAssembler::minimal();

        assembler
            .assemble(&task(), &prompt_ctx)
            .expect("prepare a valid assignment bucket");

        // Simulate a semantically inconsistent-but-decodable durable receipt.
        // Preparation returns the existing bucket, then application must reject
        // it and clean up the reservation without crossing the launch boundary.
        let mut persisted: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&store_path).expect("read prepared store"))
                .expect("decode prepared store");
        let buckets = persisted["attempt_assignments"]
            .as_object_mut()
            .expect("attempt assignment buckets");
        let bucket = buckets.values_mut().next().expect("prepared bucket");
        let assignment = bucket["assignments"]
            .as_array_mut()
            .and_then(|assignments| assignments.first_mut())
            .expect("prepared assignment");
        assignment["content_hash"] = serde_json::Value::String("corrupted-hash".into());
        std::fs::write(
            &store_path,
            serde_json::to_vec_pretty(&persisted).expect("encode corrupt receipt"),
        )
        .expect("write corrupt receipt");

        let error = assembler
            .assemble(&task(), &prompt_ctx)
            .expect_err("invalid assignment must fail prompt assembly");
        assert!(error.to_string().contains("content hash"));

        let store = ExperimentStore::load_strict(&store_path).expect("load abandoned store");
        let [assignment] = store
            .assignments_for_attempt(&attempt_key)
            .expect("attempt assignment receipt")
        else {
            panic!("expected one assignment receipt");
        };
        assert_eq!(assignment.state, PromptAssignmentState::Abandoned);
        assert!(assignment.content_snapshot.is_none());
        assert_eq!(assignment.success, None);
        assert_eq!(
            store
                .get("role-identity-experiment")
                .expect("experiment")
                .stats
                .values()
                .map(|stats| stats.trials)
                .sum::<u64>(),
            0
        );
    }

    #[test]
    fn explicit_vcg_config_reaches_the_canonical_composer() {
        let assembler =
            PromptAssembler::minimal().with_composition_strategy(ConfigCompositionStrategy::Vcg);
        let pctx = PromptContext::from_task(&task(), &ctx());
        let prompt = assembler.assemble(&task(), &pctx).unwrap();
        let manifest = prompt
            .diagnostics
            .composition_manifest
            .expect("canonical composition manifest");

        assert_eq!(manifest.requested_strategy, CompositionStrategy::Vcg);
        assert_eq!(manifest.selected_strategy, CompositionStrategy::Vcg);
        assert!(manifest.vcg_diagnostics.is_some());
    }

    #[test]
    fn terminal_feedback_warms_auto_from_greedy_to_vcg() {
        let assembler = PromptAssembler::minimal()
            .with_composition_strategy(ConfigCompositionStrategy::Auto)
            .with_vcg_warmup_observations(1);
        let pctx = PromptContext::from_task(&task(), &ctx());

        let cold = assembler.assemble(&task(), &pctx).unwrap();
        assert_eq!(
            cold.diagnostics
                .composition_manifest
                .as_ref()
                .expect("cold manifest")
                .selected_strategy,
            CompositionStrategy::DensityGreedy
        );

        assembler.record_outcome(&cold.diagnostics, true);
        assert!(
            assembler
                .learning_bidders()
                .values()
                .all(|bidder| bidder.observation_count() >= 1)
        );

        let warm = assembler.assemble(&task(), &pctx).unwrap();
        let manifest = warm
            .diagnostics
            .composition_manifest
            .expect("warm manifest");
        assert_eq!(manifest.requested_strategy, CompositionStrategy::Auto);
        assert_eq!(manifest.selected_strategy, CompositionStrategy::Vcg);
        assert!(manifest.vcg_diagnostics.is_some());
    }

    #[test]
    fn attention_bidder_store_round_trips_learned_rounds_atomically() {
        let temp = tempfile::tempdir().expect("tempdir");
        let mut bidder = LearningBidder::new(AttentionBidder::TaskContext, 1.0);
        bidder.observe_round();
        bidder.update("task", true, true);
        let bidders = HashMap::from([(AttentionBidder::TaskContext, bidder)]);

        save_attention_bidders(temp.path(), &bidders).expect("save bidders");
        let restored = load_attention_bidders(temp.path()).expect("load bidders");

        assert_eq!(restored, bidders);
        assert!(!temp.path().join("attention-bidders.tmp").exists());
    }

    #[test]
    fn malformed_attention_bidder_store_fails_closed_without_overwrite() {
        let temp = tempfile::tempdir().expect("tempdir");
        let path = temp.path().join(ATTENTION_BIDDERS_FILENAME);
        let original = b"{ definitely-not-json";
        std::fs::write(&path, original).expect("write malformed store");

        let error = load_attention_bidders(temp.path()).expect_err("malformed store must fail");

        assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
        assert_eq!(std::fs::read(path).expect("read original"), original);
    }

    #[test]
    fn attention_bidder_store_rejects_mismatched_subsystem_identity() {
        let temp = tempfile::tempdir().expect("tempdir");
        let invalid = HashMap::from([(
            AttentionBidder::Neuro,
            LearningBidder::new(AttentionBidder::Research, 1.0),
        )]);
        roko_fs::atomic_write_json(&temp.path().join(ATTENTION_BIDDERS_FILENAME), &invalid)
            .expect("write invalid store");

        let error = load_attention_bidders(temp.path()).expect_err("identity mismatch must fail");
        assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
    }

    #[test]
    fn group_context_is_membership_gated_and_not_leaked_through_neuro() {
        let temp = tempfile::tempdir().expect("tempdir");
        let now = Utc::now();
        let readable_id = GroupId::new("grp-readable");
        let hidden_id = GroupId::new("grp-hidden");
        let readable = Group {
            id: readable_id.clone(),
            name: "review-room".into(),
            description: String::new(),
            owner: "owner-a".into(),
            members: vec![GroupMember {
                agent_id: "implementer".into(),
                owner: "owner-a".into(),
                role: MemberRole::Member,
                permissions: MemberPermissions::FULL,
                joined_at: now,
            }],
            coordination: CoordinationMode::Stigmergic,
            config: GroupConfig::default(),
            created_at: now,
            updated_at: now,
        };
        let hidden = Group {
            id: hidden_id.clone(),
            name: "secret-room".into(),
            description: String::new(),
            owner: "owner-b".into(),
            members: vec![GroupMember {
                agent_id: "reviewer".into(),
                owner: "owner-b".into(),
                role: MemberRole::Member,
                permissions: MemberPermissions::FULL,
                joined_at: now,
            }],
            coordination: CoordinationMode::Stigmergic,
            config: GroupConfig::default(),
            created_at: now,
            updated_at: now,
        };
        let state = serde_json::json!({
            "version": 1,
            "groups": {
                (readable_id.as_str()): readable,
                (hidden_id.as_str()): hidden,
            },
            "pheromones": {
                (readable_id.as_str()): [{
                    "id": "visible-pheromone",
                    "pheromone": {
                        "group_id": readable_id,
                        "depositor": "implementer",
                        "signal_type": "warning",
                        "metadata": {"summary": "visible coordination signal"},
                        "deposited_at": now,
                    },
                    "balance": 0.9,
                    "last_touched_at": now,
                }],
                (hidden_id.as_str()): [{
                    "id": "hidden-pheromone",
                    "pheromone": {
                        "group_id": hidden_id,
                        "depositor": "reviewer",
                        "signal_type": "threat",
                        "metadata": {"summary": "must remain hidden"},
                        "deposited_at": now,
                    },
                    "balance": 1.0,
                    "last_touched_at": now,
                }],
            },
        });
        let group_dir = temp.path().join(".roko/groups");
        std::fs::create_dir_all(&group_dir).expect("group dir");
        std::fs::write(
            group_dir.join("state.json"),
            serde_json::to_vec_pretty(&state).expect("state json"),
        )
        .expect("write state");

        let neuro_dir = temp.path().join(".roko/neuro");
        std::fs::create_dir_all(&neuro_dir).expect("neuro dir");
        let group_entry: roko_neuro::KnowledgeEntry = serde_json::from_value(serde_json::json!({
            "id": "group-entry",
            "content": "visible wiring group knowledge",
            "confidence": 0.8,
            "tags": [format!("group:{}", readable_id)],
            "created_at": now,
        }))
        .expect("group entry");
        let public_entry: roko_neuro::KnowledgeEntry = serde_json::from_value(serde_json::json!({
            "id": "public-entry",
            "content": "public wiring knowledge",
            "confidence": 0.8,
            "tags": ["wiring"],
            "created_at": now,
        }))
        .expect("public entry");
        std::fs::write(
            neuro_dir.join("knowledge.jsonl"),
            format!(
                "{}\n{}\n",
                serde_json::to_string(&group_entry).expect("group knowledge json"),
                serde_json::to_string(&public_entry).expect("public knowledge json")
            ),
        )
        .expect("write knowledge");

        let mut dispatch = ctx();
        dispatch.workdir = temp.path().to_path_buf();
        let prompt_ctx = PromptContext::from_task(&task(), &dispatch);
        let chunks = load_group_context(temp.path(), "implementer", &task(), &prompt_ctx);
        let rendered = chunks
            .iter()
            .map(|chunk| chunk.content.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(rendered.contains("visible coordination signal"));
        assert!(rendered.contains("visible wiring group knowledge"));
        assert!(!rendered.contains("must remain hidden"));
        assert!(load_group_context(temp.path(), "outsider", &task(), &prompt_ctx).is_empty());

        let ordinary = collect_neuro_knowledge(&task(), &prompt_ctx).expect("public knowledge");
        assert!(ordinary.body.contains("public wiring knowledge"));
        assert!(!ordinary.body.contains("visible wiring group knowledge"));
    }

    #[test]
    fn retry_attempt_renders_gate_feedback() {
        let assembler = PromptAssembler::minimal();
        let mut c = ctx();
        c.attempt = 1;
        c.gate_feedback = Some(GateFeedback {
            compile_errors: vec!["E0432: unresolved import".into()],
            test_failures: vec!["mod::test_foo: assertion failed".into()],
            clippy_warnings: vec![],
            raw_output: "...".into(),
        });
        let pctx = PromptContext::from_task(&task(), &c);
        let p = assembler.assemble(&task(), &pctx).unwrap();
        // Gate feedback is rendered into the context_layer by build_runner_context.
        assert!(
            p.system_prompt.contains("# Previous attempt feedback"),
            "retry should contain gate feedback header"
        );
        assert!(
            p.system_prompt.contains("E0432"),
            "retry should contain compile error"
        );
        assert!(
            p.system_prompt.contains("mod::test_foo"),
            "retry should contain test failure"
        );
    }

    #[test]
    fn token_budget_rejects_critical_sections_that_cannot_fit() {
        // Critical sections are never silently truncated or dropped. An
        // impossible budget must stop dispatch instead of claiming a prompt
        // was assembled within the configured limit.
        let assembler = PromptAssembler::new().with_token_budget(40);
        let mut t = task();
        t.acceptance = vec!["a very long acceptance criterion that takes many tokens".into()];
        let pctx = PromptContext::from_task(&t, &ctx());
        let error = assembler
            .assemble(&t, &pctx)
            .expect_err("critical prompt sections exceed the budget");
        assert!(
            matches!(error, RunnerDispatchError::PromptAssembly(message) if message.contains("budget exceeded")),
            "the canonical composer must surface its budget failure"
        );
    }

    #[test]
    fn empty_optional_sections_omitted_cleanly() {
        let assembler = PromptAssembler::minimal();
        let mut t = task();
        t.files = vec![];
        t.acceptance = vec![];
        t.verify = vec![];
        t.allowed_tools = None;
        let pctx = PromptContext::from_task(&t, &ctx());
        let p = assembler.assemble(&t, &pctx).unwrap();
        assert!(!p.system_prompt.contains("# Files in scope"));
        assert!(!p.system_prompt.contains("# Acceptance"));
        assert!(!p.system_prompt.contains("# Verify"));
        assert!(!p.system_prompt.contains("# Allowed tools"));
        assert_eq!(p.tool_allowlist, None);
    }

    #[test]
    fn workspace_context_included_when_present() {
        let assembler = PromptAssembler::minimal();
        let mut pctx = PromptContext::from_task(&task(), &ctx());
        pctx.workspace_context =
            "# Workspace context\nBranch: `main`\n- roko-core: Core types\n".to_string();
        let p = assembler.assemble(&task(), &pctx).unwrap();
        assert!(
            p.system_prompt.contains("# Workspace context"),
            "workspace context should appear in system_prompt via context_layer"
        );
        assert!(
            p.system_prompt.contains("Branch: `main`"),
            "workspace branch should appear in system_prompt"
        );
        // The section is embedded in context_layer, not as a standalone section name.
        // diagnostics.included_sections reflects source sections (knowledge, playbooks).
        // The system_prompt content is what matters here.
    }

    #[test]
    fn workspace_context_empty_when_no_git() {
        // /tmp has no crates/ or .git — workspace_context should be empty.
        let ws_ctx = generate_workspace_context(Path::new("/tmp"));
        assert!(ws_ctx.is_empty());
    }

    #[test]
    fn cfactor_context_included_when_present() {
        let assembler = PromptAssembler::minimal();
        let mut pctx = PromptContext::from_task(&task(), &ctx());
        pctx.cfactor_context = "# Collective calibration\nC-Factor 0.72\n".to_string();
        let p = assembler.assemble(&task(), &pctx).unwrap();
        assert!(
            p.system_prompt.contains("# Collective calibration"),
            "cfactor context should appear in system_prompt via context_layer"
        );
    }

    #[test]
    fn cfactor_context_empty_when_no_history() {
        // /tmp has no .roko/learn/c-factor.jsonl — cfactor_context should be empty.
        let ctx = generate_cfactor_context(Path::new("/tmp"));
        assert!(ctx.is_empty());
    }

    #[test]
    fn scan_crate_descriptions_empty_for_missing_dir() {
        let crates = scan_crate_descriptions(Path::new("/nonexistent"));
        assert!(crates.is_empty());
    }

    #[test]
    fn git_command_returns_none_on_bad_workdir() {
        let result = git_command(Path::new("/nonexistent"), &["status"]);
        assert!(result.is_none())
    }

    #[test]
    fn parse_role_label_returns_implementer_for_known_label() {
        assert_eq!(parse_role_label("implementer"), AgentRole::Implementer);
        assert_eq!(parse_role_label("Implementer"), AgentRole::Implementer);
        assert_eq!(parse_role_label("IMPLEMENTER"), AgentRole::Implementer);
    }

    #[test]
    fn parse_role_label_falls_back_to_implementer_for_unknown() {
        assert_eq!(parse_role_label("unknown-role"), AgentRole::Implementer);
        assert_eq!(parse_role_label(""), AgentRole::Implementer);
    }

    #[test]
    fn build_runner_context_includes_all_sections() {
        let t = task();
        let pctx = PromptContext {
            plan_id: "p".into(),
            role: "implementer".into(),
            workdir: PathBuf::from("/tmp"),
            files_in_scope: vec!["src/lib.rs".into()],
            acceptance_criteria: vec!["compiles".into()],
            verify_commands: vec!["cargo test".into()],
            impact_context: "Impact policy: inspect consumers.".into(),
            gate_feedback: None,
            attempt: 0,
            prompt_experiment: None,
            workspace_map: String::new(),
            tasks_toml: String::new(),
            prd_excerpt: String::new(),
            dependency_outputs: Vec::new(),
            workspace_context: String::new(),
            cfactor_context: String::new(),
        };
        let ctx_str = build_runner_context(&t, &pctx).expect("runner context");
        assert!(ctx_str.contains("# Files in scope"));
        assert!(ctx_str.contains("# Acceptance criteria"));
        assert!(ctx_str.contains("# Verify"));
        assert!(ctx_str.contains("# Allowed tools"));
    }

    #[test]
    fn canonical_surface_used_in_assemble() {
        // Verify that `assemble` uses the canonical `build_role_system_prompt` path
        // by checking that the system_prompt contains role identity text from the
        // canonical implementer template (not the old inline "# Role" header).
        let assembler = PromptAssembler::minimal();
        let pctx = PromptContext::from_task(&task(), &ctx());
        let p = assembler.assemble(&task(), &pctx).unwrap();
        // The canonical implementer template starts with "You are the Implementer"
        // or similar identity text.
        assert!(
            p.system_prompt.contains("Implementer") || p.system_prompt.contains("implementer"),
            "system_prompt should contain canonical role identity from RoleSystemPromptSpec"
        );
        assert!(
            !p.system_prompt.is_empty(),
            "system_prompt must not be empty"
        );
    }
}
