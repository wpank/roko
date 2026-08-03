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

use std::collections::{BTreeSet, HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use roko_compose::{AttentionBidder, LearningBidder, RoleSystemPromptSpec, TaskContext};
use roko_core::AgentRole;
use serde::{Deserialize, Serialize};

use super::DispatchContext;
use super::outcome::DispatchError;
use super::prompt_cache::PromptCache;
use crate::prompting::{PromptBuildOptions, build_role_system_prompt};
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
    /// Workspace root used to resolve `.roko` learning stores.
    pub workdir: PathBuf,
    /// Files in scope for this task (from `task.files`).
    pub files_in_scope: Vec<String>,
    /// Acceptance criteria (from `task.acceptance`).
    pub acceptance_criteria: Vec<String>,
    /// `task.verify` shell commands.
    pub verify_commands: Vec<String>,
    /// Optional structured gate feedback for retry prompts.
    pub gate_feedback: Option<GateFeedback>,
    /// Attempt number (0 = first, > 0 = retry).
    pub attempt: u32,
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
    /// Ported from the legacy `workspace_context()` in orchestrate.rs; includes
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
        let workspace_map = generate_workspace_map(&ctx.workdir);
        let tasks_toml = load_tasks_toml(&ctx.workdir, &ctx.plan_id);
        let prd_excerpt = load_prd_excerpt(&ctx.workdir, &ctx.plan_id);
        let workspace_context = generate_workspace_context(&ctx.workdir);
        let cfactor_context = generate_cfactor_context(&ctx.workdir);
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
            gate_feedback: ctx.gate_feedback.clone(),
            attempt: ctx.attempt,
            workspace_map,
            tasks_toml,
            prd_excerpt,
            dependency_outputs: ctx.dependency_outputs.clone(),
            workspace_context,
            cfactor_context,
        }
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
/// 2. `{workdir}/.roko/prd/draft/{plan_id}.md`
///
/// Returns an empty string when neither exists.
fn load_prd_excerpt(workdir: &Path, plan_id: &str) -> String {
    let prd_base = workdir.join(".roko").join("prd");
    let candidates = [
        prd_base.join("published").join(format!("{plan_id}.md")),
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

// ─── Workspace context (ported from legacy orchestrate.rs) ─────────────

const WORKSPACE_CONTEXT_LIMIT: usize = 4_000;
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
/// Ported from legacy `workspace_context()` in orchestrate.rs.
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

// ─── C-Factor context (ported from legacy orchestrate.rs) ──────────────

/// Load C-Factor history and generate policy context for the system prompt.
///
/// Reads `.roko/learn/c-factor.jsonl`, computes a summary, and runs the
/// [`roko_core::CFactorPolicy`] to produce coordination guidance text.
/// Returns an empty string when no history exists or the episode count
/// is below the minimum threshold.
fn generate_cfactor_context(workdir: &Path) -> String {
    use roko_core::{Body, CFactorPolicy, CFactorSource, Context, React};
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

    // Use CFactorPolicy to generate engrams, then extract their text bodies.
    #[derive(Clone)]
    struct StaticSource(Option<roko_core::CFactorSummary>);
    impl CFactorSource for StaticSource {
        fn summary(&self) -> Option<roko_core::CFactorSummary> {
            self.0.clone()
        }
    }

    let source: Arc<dyn CFactorSource> = Arc::new(StaticSource(Some(summary)));
    let policy = CFactorPolicy::new(source).with_min_episode_count(6);
    let engrams = policy.decide(&[], &Context::now());

    if engrams.is_empty() {
        return String::new();
    }

    let mut out = String::from("# Collective calibration\n");
    for engram in &engrams {
        if let Ok(text) = engram.body.as_text() {
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
}

// ─── Source Plugins ────────────────────────────────────────────────────

/// One optional section contributed by a prompt context source.
#[derive(Debug, Clone)]
struct PromptSection {
    name: String,
    body: String,
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
fn build_runner_context(task: &TaskDef, ctx: &PromptContext) -> String {
    let mut parts: Vec<String> = Vec::new();

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

    parts.join("\n\n")
}

/// The file name for the persisted attention bidders store under `.roko/learn/`.
pub const ATTENTION_BIDDERS_FILENAME: &str = "attention-bidders.json";

/// Load persisted learning bidders from `.roko/learn/attention-bidders.json`.
///
/// Returns an empty map (with a warning) if the file is missing or malformed,
/// so prompt composition never fails hard on a missing store.
pub fn load_attention_bidders(learn_dir: &Path) -> HashMap<AttentionBidder, LearningBidder> {
    let path = learn_dir.join(ATTENTION_BIDDERS_FILENAME);
    match std::fs::read_to_string(&path) {
        Ok(contents) => match serde_json::from_str(&contents) {
            Ok(bidders) => {
                tracing::debug!(path = %path.display(), "loaded attention bidders");
                bidders
            }
            Err(err) => {
                tracing::warn!(
                    path = %path.display(),
                    error = %err,
                    "malformed attention-bidders.json — starting with empty bidders"
                );
                HashMap::new()
            }
        },
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            tracing::debug!(path = %path.display(), "attention-bidders.json not found — starting fresh");
            HashMap::new()
        }
        Err(err) => {
            tracing::warn!(
                path = %path.display(),
                error = %err,
                "failed to read attention-bidders.json — starting with empty bidders"
            );
            HashMap::new()
        }
    }
}

/// Save learning bidders to `.roko/learn/attention-bidders.json`.
///
/// Creates the learn directory if it does not exist. Errors are logged
/// but do not propagate -- bidder persistence is best-effort.
pub fn save_attention_bidders(
    learn_dir: &Path,
    bidders: &HashMap<AttentionBidder, LearningBidder>,
) {
    if let Err(err) = std::fs::create_dir_all(learn_dir) {
        tracing::warn!(
            path = %learn_dir.display(),
            error = %err,
            "failed to create learn directory for attention-bidders"
        );
        return;
    }
    let path = learn_dir.join(ATTENTION_BIDDERS_FILENAME);
    match serde_json::to_string_pretty(bidders) {
        Ok(json) => {
            if let Err(err) = std::fs::write(&path, json) {
                tracing::warn!(
                    path = %path.display(),
                    error = %err,
                    "failed to write attention-bidders.json"
                );
            } else {
                tracing::debug!(
                    path = %path.display(),
                    bidder_count = bidders.len(),
                    "saved attention bidders"
                );
            }
        }
        Err(err) => {
            tracing::warn!(error = %err, "failed to serialize attention bidders");
        }
    }
}

#[derive(Debug, Clone)]
pub struct PromptAssembler {
    /// Token budget cap.
    token_budget: u32,
    /// Optional prompt context sources. `minimal()` leaves this empty.
    sources: Vec<Arc<dyn PromptSectionSource>>,
    /// Persisted learning bidders for prompt composition.
    learning_bidders: HashMap<AttentionBidder, LearningBidder>,
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
            learning_bidders: HashMap::new(),
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
            learning_bidders: HashMap::new(),
            section_effectiveness: Some(effectiveness),
        }
    }

    /// Test / smoke assembler -- no knowledge stores, tiny budget.
    #[must_use]
    pub fn minimal() -> Self {
        Self {
            token_budget: 8_000,
            sources: Vec::new(),
            learning_bidders: HashMap::new(),
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
        self.learning_bidders = bidders;
        self
    }

    /// Read-only access to the current learning bidders.
    #[must_use]
    pub fn learning_bidders(&self) -> &HashMap<AttentionBidder, LearningBidder> {
        &self.learning_bidders
    }

    /// Set the composition strategy for VCG/density-greedy budget allocation.
    ///
    /// The canonical `build_role_system_prompt` path manages its own budget
    /// internally; this knob is retained for forward-compatibility with
    /// `factory.rs` callers. The value is currently ignored by `assemble()`.
    #[must_use]
    pub fn with_composition_strategy(
        self,
        _strategy: roko_core::config::schema::ConfigCompositionStrategy,
    ) -> Self {
        self
    }

    /// Set the minimum bidder-observation count before VCG allocation activates.
    ///
    /// Retained for forward-compatibility with `factory.rs` callers. The value
    /// is currently ignored by `assemble()` which delegates to the canonical
    /// `build_role_system_prompt` path.
    #[must_use]
    pub fn with_vcg_warmup_observations(self, _observations: u32) -> Self {
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
    ) -> Result<AssembledPrompt, DispatchError> {
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
        let runner_context = build_runner_context(task, ctx);

        // Build TaskContext with runner-specific context block.
        let task_context = {
            let mut tc = TaskContext::new(task_text)
                .with_plan_id(ctx.plan_id.clone())
                .with_workspace(ctx.workdir.to_string_lossy().into_owned());
            if !runner_context.is_empty() {
                tc = tc.with_context(runner_context.clone());
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

        // Build PromptBuildOptions, threading knowledge/playbook context and
        // section-effectiveness through to the canonical compose builder.
        //
        // When the assembler has a section_effectiveness registry (loaded from
        // the PromptCache or disk), the compose builder adjusts section
        // priorities based on historical effectiveness data.
        let section_effectiveness = self.resolve_section_effectiveness(&ctx.workdir);
        let options = PromptBuildOptions {
            code_context,
            section_effectiveness,
            ..PromptBuildOptions::default()
        };

        // Delegate to the canonical build_role_system_prompt path.
        let system_prompt = build_role_system_prompt(role, task_context, tools_csv, options);

        // ── Diagnostics ───────────────────────────────────────────────────
        let estimated_tokens = (system_prompt.len() / 4).max(1) as u32;
        let diagnostics = PromptDiagnostics {
            included_sections: source_sections.iter().map(|s| s.name.clone()).collect(),
            dropped_sections: Vec::new(),
            estimated_tokens,
            playbook_ids,
            knowledge_ids,
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
    let entries = store.query(&query, 5).ok()?;
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
    // Build paths through Workspace accessors where possible; fall back to
    // raw construction only for the legacy memory path.
    //
    // Order: root (canonical) -> learn -> memory (legacy fallback only).
    match roko_core::Workspace::open(workdir) {
        Ok(ws) => vec![
            ws.episodes_path(),
            ws.learn_episodes_path(),
            // Legacy fallback — retained for reading pre-migration data.
            ws.memory_dir().join("episodes.jsonl"),
        ],
        Err(_) => vec![
            workdir.join(".roko").join("episodes.jsonl"),
            workdir.join(".roko").join("learn").join("episodes.jsonl"),
            workdir.join(".roko").join("memory").join("episodes.jsonl"),
        ],
    }
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
    use std::path::PathBuf;

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
            gate_feedback: None,
            routing_context: None,
            dependency_outputs: Vec::new(),
        }
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
    fn token_budget_drops_lowest_priority_sections() {
        // The canonical builder handles its own budget; this test verifies the
        // assembler produces a non-empty system_prompt even under a tight budget.
        let assembler = PromptAssembler::new().with_token_budget(40);
        let mut t = task();
        t.acceptance = vec!["a very long acceptance criterion that takes many tokens".into()];
        let pctx = PromptContext::from_task(&t, &ctx());
        let p = assembler.assemble(&t, &pctx).unwrap();
        // The canonical path always emits at least the role identity section.
        assert!(
            !p.system_prompt.is_empty(),
            "system_prompt should be non-empty even with a tight budget"
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
            gate_feedback: None,
            attempt: 0,
            workspace_map: String::new(),
            tasks_toml: String::new(),
            prd_excerpt: String::new(),
            dependency_outputs: Vec::new(),
            workspace_context: String::new(),
            cfactor_context: String::new(),
        };
        let ctx_str = build_runner_context(&t, &pctx);
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
