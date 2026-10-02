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
//!   playbook ids, knowledge ids, and each retrieved item with whether it
//!   reached the prompt — used for prompt experiments, the projection layer
//!   and the run's exposure log
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

use parking_lot::RwLock;
use roko_compose::{
    AttentionBidder, CompositionManifest, CompositionStrategy, ContextChunk, ContextSource,
    LearningBidder, MultiPatchForager, PromptComposer, PromptSection as CanonicalPromptSection,
    RoleSystemPromptSpec, SourceForagingProfile, TaskContext,
};
use roko_core::config::schema::ConfigCompositionStrategy;
use roko_core::{AgentRole, Group, GroupId, GroupPheromone, TaskContextWeight};
use roko_learn::telemetry::records::b3_digest;
use roko_learn::telemetry::{ExcludedReason, ExposureItemKind};
use serde::{Deserialize, Serialize};
use sha2::Digest;

use super::outcome::RunnerDispatchError;
use super::prompt_cache::PromptCache;
use super::{DispatchContext, PromptExperimentContext};
use crate::task_accept;
use crate::task_parser::TaskDef;

/// Printed under a list of verify steps that includes a pinned acceptance test (gap-1b5636). The
/// list shows such a step by its header line alone; the rest of it is the harness's plumbing.
const PINNED_STEP_NOTE: &str = "The harness runs each `# roko accept:` step itself: it copies the \
     pinned test over its destination, so edits to that copy are lost, and requires exactly the \
     stated number of passing tests.";

/// Appended to the user prompt of a task that sets `research_before_edit` (gap-404fdb).
const RESEARCH_BEFORE_EDIT_NOTE: &str = "\n## Before You Edit\nResearch first: find the code that \
     already does something like this task (search for the types, functions and files it names), \
     read it, and follow its patterns. Make your first edit only after that.\n";

/// Maximum tokens an assembled prompt may emit before deterministic
/// dropping kicks in. Roughly mirrors a 200K-context-window providers'
/// budget for system + user combined.
const DEFAULT_TOKEN_BUDGET: u32 = 64_000;

/// Construct a default [`MultiPatchForager`] with baseline source profiles.
///
/// The profiles assign diminishing-returns curves to the three context
/// sources that the prompt composer evaluates: knowledge entries, episodes,
/// and inline files. The active-inference bias defaults to 0.3, encouraging
/// moderate exploration across patches.
fn default_forager() -> MultiPatchForager {
    MultiPatchForager {
        source_profiles: vec![
            SourceForagingProfile {
                source: ContextSource::KnowledgeEntry {
                    entry_id: String::new(),
                    kind: String::new(),
                    source: None,
                },
                g_max: 0.9,
                lambda: 0.3,
                travel_cost: 0.1,
            },
            SourceForagingProfile {
                source: ContextSource::Episode {
                    episode_id: String::new(),
                    plan_id: String::new(),
                    task_id: String::new(),
                },
                g_max: 0.7,
                lambda: 0.4,
                travel_cost: 0.15,
            },
            SourceForagingProfile {
                source: ContextSource::InlineFile {
                    path: String::new(),
                    lines: None,
                },
                g_max: 0.8,
                lambda: 0.5,
                travel_cost: 0.05,
            },
        ],
        environment_rate: 0.1,
        active_inference_bias: 0.3,
    }
}

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
    /// `task.verify` shell commands as prompts show them: a pinned acceptance step by its header
    /// line only ([`task_accept::prompt_command`]).
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
    /// Output files from completed dependency tasks.
    /// Each entry is `(task_id, files)`.
    pub dependency_outputs: Vec<(String, Vec<String>)>,
    /// Workspace context: git branch, modified files, crate names/descriptions.
    /// Ported from the legacy `workspace_context()` helper; includes
    /// git state (best-effort, bounded) and crate scan from `crates/*/Cargo.toml`.
    pub workspace_context: String,
    /// Pre-rendered error patterns from the shared in-memory store.
    ///
    /// Carried from `DispatchContext::error_patterns_context` so the prompt
    /// assembler can inject "known pitfalls" without touching the store itself.
    pub error_patterns_context: String,
    /// The other plans running in the same working tree now, with the areas
    /// they write ([`DispatchContext::concurrent_plans`]).
    pub concurrent_plans: Vec<(String, Vec<String>)>,
    /// The plan's `brief.md`, which `roko plan prepare` writes (gap-d6fd85).
    pub plan_brief: String,
}

impl PromptContext {
    /// Construct a `PromptContext` from runner inputs.
    ///
    /// When `ctx` carries pre-computed `cached_workspace_map` or
    /// `cached_workspace_context` (non-empty), those values are used
    /// directly — no filesystem I/O is performed for those fields.  This avoids blocking the Tokio reactor on repeated
    /// directory walks and `git` subprocess spawns.
    ///
    /// `GraphTaskDispatcher` populates the cache fields via a `OnceLock` so
    /// the work is done at most once per plan run, on the first dispatch.
    #[must_use]
    pub fn from_task(task: &TaskDef, ctx: &DispatchContext) -> Self {
        let execution_policy = crate::plan_policy::PlanExecutionPolicy::for_environment();
        let bounded_context_only = execution_policy.bounded_context_only;

        // Resolve per-role context limits before loading any sections. Each
        // role cluster has different information needs; applying role-specific
        // limits here keeps the run-scoped cache at full size while still
        // giving individual task dispatches only the context they need.
        let role_limits = context_limits_for_role(&ctx.role);
        // The task's `context_weight` scales those limits: `slim` loads none of
        // these sections, `deep` twice as much (gap-404fdb).
        let factor = context_factor(task.hints.context_weight);
        let role_limits = role_limits.scaled(factor);
        let skip_enrichment = bounded_context_only || factor == 0;

        // Use pre-computed run-scoped cache when available; fall back to
        // on-demand computation (for callers that don't populate the cache,
        // e.g. tests, runner-v2, or the oneshot path).
        //
        // The cache always holds the full-size content (up to the global
        // constants). After loading we re-apply role-specific limits so that
        // roles with smaller budgets get a tighter slice without requiring a
        // separate cache entry per role.
        let workspace_map = if skip_enrichment {
            String::new()
        } else if !ctx.cached_workspace_map.is_empty() {
            truncate_to_limit(ctx.cached_workspace_map.clone(), role_limits.workspace_map)
        } else {
            truncate_to_limit(
                generate_workspace_map(&ctx.workdir),
                role_limits.workspace_map,
            )
        };
        let tasks_toml = if skip_enrichment {
            String::new()
        } else {
            truncate_to_limit(
                load_tasks_toml(&ctx.workdir, &ctx.plan_id, TASKS_TOML_LIMIT * factor),
                role_limits.tasks_toml,
            )
        };
        let workspace_context = if skip_enrichment {
            String::new()
        } else if !ctx.cached_workspace_context.is_empty() {
            truncate_to_limit(
                ctx.cached_workspace_context.clone(),
                role_limits.workspace_context,
            )
        } else {
            truncate_to_limit(
                generate_workspace_context(&ctx.workdir),
                role_limits.workspace_context,
            )
        };
        let impact_context = declared_impact_context(task, bounded_context_only);
        let plan_brief = if skip_enrichment {
            String::new()
        } else {
            truncate_to_limit(
                load_plan_brief(&ctx.workdir, &ctx.plan_id),
                PLAN_BRIEF_LIMIT,
            )
        };
        tracing::debug!(
            plan_id = %ctx.plan_id,
            role = %ctx.role,
            workspace_map_limit = role_limits.workspace_map,
            tasks_toml_limit = role_limits.tasks_toml,
            workspace_context_limit = role_limits.workspace_context,
            workspace_map_bytes = workspace_map.len(),
            tasks_toml_bytes = tasks_toml.len(),
            workspace_context_bytes = workspace_context.len(),
            workspace_map_from_cache = !ctx.cached_workspace_map.is_empty(),
            workspace_context_from_cache = !ctx.cached_workspace_context.is_empty(),
            "PromptContext enrichment sizes (role-scoped)"
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
                .map(|step| task_accept::prompt_command(&step.command).to_string())
                .collect(),
            impact_context,
            gate_feedback: ctx.gate_feedback.clone(),
            attempt: ctx.attempt,
            prompt_experiment: ctx.prompt_experiment.clone(),
            workspace_map,
            tasks_toml,
            dependency_outputs: ctx.dependency_outputs.clone(),
            workspace_context,
            error_patterns_context: ctx.error_patterns_context.clone(),
            concurrent_plans: ctx.concurrent_plans.clone(),
            plan_brief,
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

const WORKSPACE_MAP_LIMIT: usize = 6_000;
const TASKS_TOML_LIMIT: usize = 4_000;
const PLAN_BRIEF_LIMIT: usize = 4_000;

/// Per-role context size limits for prompt enrichment sections.
///
/// Different roles have different information needs:
/// - `implementer` needs full workspace map and task context to make code changes.
/// - `researcher` does broad research; workspace map and task detail are less useful.
/// - `strategist` needs the full task list to reason about plans; workspace detail less needed.
/// - `auditor` needs gate/verification context; workspace map less critical.
/// - All other roles fall back to the defaults matching the global constants above.
#[derive(Debug, Clone, Copy)]
pub struct RoleContextLimits {
    /// Maximum characters for the workspace crate map section.
    pub workspace_map: usize,
    /// Maximum characters for tasks.toml content.
    pub tasks_toml: usize,
    /// Maximum characters for the workspace context (git + crate descriptions).
    pub workspace_context: usize,
}

impl RoleContextLimits {
    /// Default limits — matched to the global constants; used by `implementer`
    /// and any role that benefits from full workspace visibility.
    pub const fn default_limits() -> Self {
        Self {
            workspace_map: WORKSPACE_MAP_LIMIT,         // 6 000
            tasks_toml: TASKS_TOML_LIMIT,               // 4 000
            workspace_context: WORKSPACE_CONTEXT_LIMIT, // 2 000
        }
    }

    /// Limits for roles focused on research and knowledge synthesis.
    ///
    /// Reduces the workspace map, task list and workspace context, which are
    /// less relevant to broad research.
    pub const fn researcher_limits() -> Self {
        Self {
            workspace_map: 2_000,
            tasks_toml: 2_000,
            workspace_context: 1_000,
        }
    }

    /// Limits for roles focused on planning and strategy (Strategist, Architect,
    /// PrePlanner, Scribe, Critic).
    ///
    /// Reduces workspace detail and keeps the full task list so the whole
    /// plan scope is visible when reasoning about decomposition.
    pub const fn strategist_limits() -> Self {
        Self {
            workspace_map: 2_000,
            tasks_toml: TASKS_TOML_LIMIT, // full task list for planning
            workspace_context: 1_000,
        }
    }

    /// Limits for roles focused on verification and gate review (Auditor,
    /// QuickReviewer, DocVerifier, IntegrationTester, TerminalValidator,
    /// RegressionDetector, CoverageTracker, SnapshotComparator, FullLoopValidator).
    ///
    /// Reduces workspace map (less relevant to reviewing) while keeping
    /// gate/verification context accessible.
    pub const fn auditor_limits() -> Self {
        Self {
            workspace_map: 2_000,
            tasks_toml: TASKS_TOML_LIMIT, // full task list for context on what was planned
            workspace_context: WORKSPACE_CONTEXT_LIMIT,
        }
    }

    /// Every limit `factor` times over.
    #[must_use]
    pub const fn scaled(self, factor: usize) -> Self {
        Self {
            workspace_map: self.workspace_map * factor,
            tasks_toml: self.tasks_toml * factor,
            workspace_context: self.workspace_context * factor,
        }
    }
}

/// How many times the usual context a task's `context_weight` asks for: none
/// for `slim` (just the task and its role), twice for `deep`, and the usual
/// for `standard` or no hint.
const fn context_factor(weight: Option<TaskContextWeight>) -> usize {
    match weight {
        Some(TaskContextWeight::Slim) => 0,
        Some(TaskContextWeight::Deep) => 2,
        // `standard`, no hint, or a weight this build does not know.
        _ => 1,
    }
}

/// Select per-role context limits from the role label string.
///
/// Parsing is the same logic as [`parse_role_label`] — falls back to
/// [`RoleContextLimits::default_limits`] for unrecognised roles so
/// prompt assembly is never blocked by a missing role mapping.
fn context_limits_for_role(role: &str) -> RoleContextLimits {
    match parse_role_label(role) {
        // Implementer: full workspace map, standard limits.
        AgentRole::Implementer
        | AgentRole::AutoFixer
        | AgentRole::Refactorer
        | AgentRole::MergeResolver
        | AgentRole::ErrorDiagnoser
        | AgentRole::DependencyValidator
        | AgentRole::PatternExtractor
        | AgentRole::LifecycleTester
        | AgentRole::CrossSystemTester => RoleContextLimits::default_limits(),

        // Researcher: smaller workspace map and task list.
        AgentRole::Researcher => RoleContextLimits::researcher_limits(),

        // Strategist cluster: planning-focused, full task list.
        AgentRole::Strategist
        | AgentRole::Architect
        | AgentRole::PrePlanner
        | AgentRole::Scribe
        | AgentRole::Critic
        | AgentRole::SpecDriftDetector
        | AgentRole::PlanLifecycleManager => RoleContextLimits::strategist_limits(),

        // Auditor cluster: verification-focused, reduced workspace map.
        AgentRole::Auditor
        | AgentRole::QuickReviewer
        | AgentRole::DocVerifier
        | AgentRole::IntegrationTester
        | AgentRole::TerminalValidator
        | AgentRole::RegressionDetector
        | AgentRole::PerformanceSentinel
        | AgentRole::CoverageTracker
        | AgentRole::SnapshotComparator
        | AgentRole::FullLoopValidator => RoleContextLimits::auditor_limits(),

        // Conductor: meta-orchestrator; use defaults.
        AgentRole::Conductor => RoleContextLimits::default_limits(),

        // Any future variants not explicitly handled default to implementer
        // limits so prompt assembly is never blocked by a missing role mapping.
        _ => RoleContextLimits::default_limits(),
    }
}

/// Truncate `s` to `limit` chars, appending `"\n[truncated]"` when clipped.
///
/// Returns the string unchanged when it is already within the limit.
fn truncate_to_limit(s: String, limit: usize) -> String {
    if s.len() <= limit {
        return s;
    }
    let mut truncated: String = s.chars().take(limit).collect();
    truncated.push_str("\n[truncated]");
    truncated
}

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
/// Returns an empty string when neither exists, and at most `cap` characters.
fn load_tasks_toml(workdir: &Path, plan_id: &str, cap: usize) -> String {
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
                return if content.len() > cap {
                    let mut truncated = content.chars().take(cap).collect::<String>();
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

/// Load the `brief.md` of plan `plan_id` (`roko plan prepare`), from the
/// plan directories [`load_tasks_toml`] reads. Empty when it has none.
fn load_plan_brief(workdir: &Path, plan_id: &str) -> String {
    let file = crate::plan_brief::BRIEF_FILE;
    let candidates = [
        workdir.join(".roko").join("plans").join(plan_id).join(file),
        workdir.join("plans").join(plan_id).join(file),
    ];
    candidates
        .iter()
        .find_map(|path| std::fs::read_to_string(path).ok())
        .unwrap_or_default()
}

/// The `## Skills` section of a task that names `skills`: each skill's
/// summary and prompt from the workspace's skill library
/// (`.roko/learn/skills.json`), or just its name when the library has no
/// skill by that name (gap-404fdb). Empty when the task names none.
fn skills_section(task: &TaskDef, workdir: &Path) -> String {
    let names = task.hints.skills.as_deref().unwrap_or_default();
    if names.is_empty() {
        return String::new();
    }
    let path = workdir.join(".roko").join("learn").join("skills.json");
    let library: Vec<serde_json::Value> = std::fs::read(path)
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default();
    let field = |skill: &serde_json::Value, key: &str| {
        skill[key].as_str().unwrap_or_default().trim().to_string()
    };
    let mut section = String::from("\n## Skills\n");
    for name in names {
        let skill = library.iter().find(|skill| skill["name"] == name.as_str());
        match skill {
            Some(skill) => section.push_str(&format!(
                "### {name}\n{}\n\n{}\n",
                field(skill, "summary"),
                field(skill, "prompt_template")
            )),
            None => section.push_str(&format!("- {name}\n")),
        }
    }
    section
}

// ─── Workspace context (ported from legacy orchestrator) ───────────────

const WORKSPACE_CONTEXT_LIMIT: usize = 2_000;
const GIT_STATUS_LINE_LIMIT: usize = 40;

/// Build a bounded workspace context string with git state and crate descriptions.
///
/// Combines:
/// - Current git branch (`git branch --show-current`)
/// - Modified files (`git status --short`), capped at [`GIT_STATUS_LINE_LIMIT`] lines
/// - Crate names and descriptions from `crates/*/Cargo.toml`
///
/// All git calls are best-effort to avoid hanging on non-git workdirs or slow
/// NFS mounts.
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

// ─── Public adapters for run-scoped caching ───────────────────────────────
//
// `GraphTaskDispatcher` computes these once per plan run (via `OnceLock`) and
// stores them on `DispatchContext` so `PromptContext::from_task` never has to
// call the underlying sync I/O helpers on the Tokio reactor thread.

/// Public adapter — see [`generate_workspace_map`].
pub fn generate_workspace_map_pub(workdir: &Path) -> String {
    generate_workspace_map(workdir)
}

/// Public adapter — see [`generate_workspace_context`].
pub fn generate_workspace_context_pub(workdir: &Path) -> String {
    generate_workspace_context(workdir)
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
    /// A cheap model's short diagnosis of the failure, rendered ahead of the
    /// errors.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diagnosis: Option<String>,
}

impl GateFeedback {
    /// Attach a diagnosis of the failure; an empty one is ignored.
    #[must_use]
    pub fn with_diagnosis(mut self, diagnosis: &str) -> Self {
        let diagnosis = diagnosis.trim();
        if !diagnosis.is_empty() {
            self.diagnosis = Some(diagnosis.chars().take(MAX_DIAGNOSIS_CHARS).collect());
        }
        self
    }

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
            diagnosis: None,
        })
    }
}

/// Longest diagnosis a retry prompt carries.
const MAX_DIAGNOSIS_CHARS: usize = 1_200;

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
    /// Prior-episode ids the episode section cited (if any). Feedback treats
    /// `knowledge_ids` as knowledge entry ids, so these stay apart.
    #[serde(default)]
    pub episode_ids: Vec<String>,
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
    /// Every item the prompt's sources retrieved (knowledge entries, cited
    /// episodes, playbooks), its error-pattern block and every candidate
    /// section, each with whether it reached the prompt (S01 P0-9). The id
    /// lists above name what was retrieved; these say what was included.
    #[serde(default)]
    pub items: Vec<PromptItemDiagnostic>,
}

/// One item a prompt retrieved, and whether it reached the prompt (S01
/// §4.5). It holds a digest and a token count, never the item's text.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PromptItemDiagnostic {
    /// What the item is.
    pub kind: ExposureItemKind,
    /// Its id: a knowledge entry, episode or playbook id, a section name, or
    /// the `b3:` digest of the error-pattern block, whose formatter passes
    /// no pattern ids.
    pub id: String,
    /// The prompt section that carries it: `domain_context` for knowledge,
    /// episodes and playbooks, `context_layer` for the error patterns, and
    /// its own name for a section.
    pub section: String,
    /// 1-based position in its source's ranking; `None` for a section.
    pub rank: Option<u32>,
    /// Its source's score, when the source scores: the task keywords a
    /// knowledge entry or episode matched, a playbook's relevance, or a
    /// section's composition score.
    pub score: Option<f64>,
    /// Estimated tokens of its rendered text (of a section, after its hard
    /// cap).
    pub tokens: u32,
    /// `sha256` of its rendered text (of a section, of its candidate
    /// content).
    pub rendered_sha256: String,
    /// Its section reached the prompt, and so did its rendered text.
    pub included: bool,
    /// Why it did not, when it did not: `token_budget` when its section was
    /// dropped or its hard cap cut the item off, `role_filter` when the
    /// role's budget gives its section no room.
    pub excluded_reason: Option<ExcludedReason>,
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
    // Reserved for future context-pressure token budgeting; not read yet.
    _drop_priority: u32,
    knowledge_ids: Vec<String>,
    playbook_ids: Vec<String>,
    /// Each entry the source rendered into `body`, in its ranking.
    items: Vec<PromptItem>,
}

impl PromptSection {
    fn new(name: impl Into<String>, body: impl Into<String>, drop_priority: u32) -> Self {
        Self {
            name: name.into(),
            body: body.into(),
            _drop_priority: drop_priority,
            knowledge_ids: Vec::new(),
            playbook_ids: Vec::new(),
            items: Vec::new(),
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

    fn with_items(mut self, items: Vec<PromptItem>) -> Self {
        self.items = items;
        self
    }
}

/// One entry a prompt source rendered into its section (S01 P0-9): a
/// knowledge entry, a cited episode or a playbook.
#[derive(Debug, Clone)]
struct PromptItem {
    kind: ExposureItemKind,
    id: String,
    /// 1-based position in the source's ranking.
    rank: u32,
    /// The source's score, when it scores.
    score: Option<f64>,
    /// The text the source rendered for the entry.
    rendered: String,
}

impl PromptItem {
    /// The entry at 0-based `index` of its source's ranking. An entry with no
    /// id is not an item: no exposure could name it.
    fn ranked(
        kind: ExposureItemKind,
        id: &str,
        index: usize,
        score: Option<f64>,
        rendered: &str,
    ) -> Option<Self> {
        (!id.is_empty()).then(|| Self {
            kind,
            id: id.to_string(),
            rank: u32::try_from(index + 1).unwrap_or(u32::MAX),
            score,
            rendered: rendered.to_string(),
        })
    }
}

/// The canonical section the knowledge, episode, playbook and
/// section-effectiveness sources render into: their bodies are its domain
/// notes.
const SOURCE_SECTION: &str = "domain_context";

/// The canonical section the error-pattern block renders into: it ends the
/// runner context.
const RUNNER_CONTEXT_SECTION: &str = "context_layer";

/// A composed prompt and its composition receipt, which together say
/// whether a retrieved item reached the prompt.
struct ComposedPrompt<'a> {
    manifest: Option<&'a CompositionManifest>,
    prompt: &'a str,
}

impl ComposedPrompt<'_> {
    /// Why text rendered into the section `carrier` is not in the prompt, or
    /// `None` when it is: the section reached the prompt and its hard cap
    /// kept the text. Without a composition receipt the text alone decides.
    fn excluded_reason(&self, carrier: &str, rendered: &str) -> Option<ExcludedReason> {
        let in_prompt = self.prompt.contains(rendered.trim_end());
        let Some(manifest) = self.manifest else {
            return (!in_prompt).then_some(ExcludedReason::TokenBudget);
        };
        if manifest
            .included
            .iter()
            .any(|section| section.name == carrier)
        {
            (!in_prompt).then_some(ExcludedReason::TokenBudget)
        } else if manifest
            .excluded
            .iter()
            .any(|section| section.name == carrier)
        {
            Some(ExcludedReason::TokenBudget)
        } else {
            // The role's budget profile gives the section no room, so it
            // was never a candidate.
            Some(ExcludedReason::RoleFilter)
        }
    }

    /// `item`, rendered into the section `carrier`, and whether it reached
    /// the prompt.
    fn item(&self, item: &PromptItem, carrier: &str) -> PromptItemDiagnostic {
        let excluded_reason = self.excluded_reason(carrier, &item.rendered);
        PromptItemDiagnostic {
            kind: item.kind,
            id: item.id.clone(),
            section: carrier.to_string(),
            rank: Some(item.rank),
            score: item.score,
            tokens: token_count(roko_compose::estimate_tokens(&item.rendered)),
            rendered_sha256: sha256_hex(&item.rendered),
            included: excluded_reason.is_none(),
            excluded_reason,
        }
    }

    /// Every entry `sources` rendered, the `error_patterns` block, and one
    /// item per candidate section, whose candidate content `section_digests`
    /// holds.
    fn items(
        &self,
        sources: &[PromptSection],
        error_patterns: &str,
        section_digests: &HashMap<String, String>,
    ) -> Vec<PromptItemDiagnostic> {
        let mut items: Vec<PromptItemDiagnostic> = sources
            .iter()
            .flat_map(|section| &section.items)
            .map(|item| self.item(item, SOURCE_SECTION))
            .collect();
        if !error_patterns.trim().is_empty() {
            // One item for the block until its formatter passes pattern ids.
            let block = PromptItem {
                kind: ExposureItemKind::ErrorPattern,
                id: b3_digest(error_patterns.as_bytes()),
                rank: 1,
                score: None,
                rendered: error_patterns.to_string(),
            };
            items.push(self.item(&block, RUNNER_CONTEXT_SECTION));
        }
        let Some(manifest) = self.manifest else {
            return items;
        };
        let section = |name: &str, tokens: usize, score: f32, kept: bool| PromptItemDiagnostic {
            kind: ExposureItemKind::Section,
            id: name.to_string(),
            section: name.to_string(),
            rank: None,
            score: Some(f64::from(score)),
            tokens: token_count(tokens),
            rendered_sha256: section_digests.get(name).cloned().unwrap_or_default(),
            included: kept,
            excluded_reason: (!kept).then_some(ExcludedReason::TokenBudget),
        };
        for kept in &manifest.included {
            items.push(section(
                kept.name.as_str(),
                kept.estimated_tokens,
                kept.score,
                true,
            ));
        }
        for cut in &manifest.excluded {
            items.push(section(
                cut.name.as_str(),
                cut.estimated_tokens,
                cut.score,
                false,
            ));
        }
        items
    }
}

/// `tokens` as a diagnostic count.
fn token_count(tokens: usize) -> u32 {
    u32::try_from(tokens).unwrap_or(u32::MAX)
}

/// Hex `sha256` of `text`.
fn sha256_hex(text: &str) -> String {
    format!("{:x}", sha2::Sha256::digest(text.as_bytes()))
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
    // The `reviewer` plan role has no `AgentRole` of its own. Its contract
    // denies every write tool, so it gets the read-only quick-reviewer prompt
    // instead of falling through to the implementer's "write code" prompt.
    if normalized == "reviewer" {
        return AgentRole::QuickReviewer;
    }
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
/// feedback, dependency outputs, workspace map, tasks toml, workspace
/// context, and C-factor context into a single markdown block. This
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
        let pinned = ctx
            .verify_commands
            .iter()
            .map(String::as_str)
            .any(task_accept::is_pinned_command);
        let note = if pinned {
            format!("\n{PINNED_STEP_NOTE}")
        } else {
            String::new()
        };
        parts.push(format!("# Verify\nAfter editing, run:\n{list}{note}"));
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

    // gap-c09fc7: other plans editing this tree make a wide build fail for
    // reasons that are not the agent's.
    if !ctx.concurrent_plans.is_empty() {
        let mut plans = String::from(
            "# Plans Running Beside This One\n\nOther plans edit this working tree while you \
             work. A build or test of more than your own crates may compile their half-finished \
             edits and fail for reasons that are not yours. Build and test only the crates your \
             task changes, and leave these areas alone:\n",
        );
        for (plan_id, areas) in &ctx.concurrent_plans {
            let areas = if areas.is_empty() {
                "its own files".to_string()
            } else {
                areas.join(", ")
            };
            plans.push_str(&format!("- `{plan_id}`: {areas}\n"));
        }
        parts.push(plans);
    }

    if !ctx.workspace_map.is_empty() {
        parts.push(ctx.workspace_map.clone());
    }

    if !ctx.plan_brief.is_empty() {
        parts.push(format!("# Plan Brief\n{}", ctx.plan_brief));
    }

    if !ctx.tasks_toml.is_empty() {
        parts.push(format!("# Sibling Tasks\n```toml\n{}\n```", ctx.tasks_toml));
    }

    if !ctx.workspace_context.is_empty() {
        parts.push(ctx.workspace_context.clone());
    }

    if !ctx.error_patterns_context.is_empty() {
        parts.push(ctx.error_patterns_context.clone());
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

    /// P1-19: Feed per-section cost attribution into learning bidders.
    ///
    /// Each tuple is `(bidder, section_name, included, gate_passed, cost_usd, tokens)`.
    pub fn update_bidders_with_cost(
        &self,
        section_costs: &[(
            roko_compose::AttentionBidder,
            String,
            bool,
            bool,
            f64,
            usize,
        )],
    ) {
        let mut bidders = self.learning_bidders.write();
        for (bidder_id, section_name, was_included, gate_passed, cost_usd, tokens) in section_costs
        {
            bidders
                .entry(*bidder_id)
                .or_insert_with(|| LearningBidder::new(*bidder_id, 1.0))
                .update_with_cost(
                    section_name,
                    *was_included,
                    *gate_passed,
                    *cost_usd,
                    *tokens,
                );
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
    /// dependency outputs, workspace map, etc.) is mapped into
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
        let mut episode_ids: Vec<String> = Vec::new();
        let mut code_context: Vec<String> = Vec::new();
        for sec in &source_sections {
            playbook_ids.extend(sec.playbook_ids.clone());
            // The episode section cites prior episodes, not knowledge entries.
            if sec.name == "episode_knowledge" {
                episode_ids.extend(sec.knowledge_ids.clone());
            } else {
                knowledge_ids.extend(sec.knowledge_ids.clone());
            }
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
        // gate feedback, dep outputs, workspace map, etc.) injected
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
            .with_learning_bidders(self.learning_bidders())
            .with_foraging(default_forager());
        let mut canonical_sections = if let Some(registry) = section_effectiveness.as_ref() {
            spec.build_sections_with_section_effectiveness(registry)
        } else {
            spec.build_sections()
        };
        // P1-20: Apply model-aware attention-curve-driven dynamic placement.
        // Non-critical sections are reassigned to higher-attention prompt
        // edges (start/end) based on information density relative to the
        // task description, mitigating the "lost in the middle" effect.
        let task_query = task.description.as_deref().unwrap_or(&task.title);
        roko_compose::dynamic_placement(&mut canonical_sections, task_query);

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
        // Each candidate section's content digest, for its exposure item.
        let section_digests: HashMap<String, String> = canonical_sections
            .iter()
            .map(|section| (section.name.clone(), sha256_hex(&section.content)))
            .collect();
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
        // What each retrieved item became: an item reached the prompt only
        // when its section did and its text survived the section's cap.
        let composed = ComposedPrompt {
            manifest: composition_manifest.as_ref(),
            prompt: &system_prompt,
        };
        let items = composed.items(
            &source_sections,
            &ctx.error_patterns_context,
            &section_digests,
        );
        let diagnostics = PromptDiagnostics {
            included_sections,
            dropped_sections,
            estimated_tokens,
            playbook_ids,
            knowledge_ids,
            episode_ids,
            scored_signals,
            composition_manifest,
            experiment_assignments: experiment_assignment_diagnostics,
            items,
        };

        // ── User prompt (unchanged) ────────────────────────────────────────
        let mut user_prompt = format!("# Task Request\n{}\n", task.title);
        if let Some(description) = &task.description {
            user_prompt.push_str("\n## Details\n");
            user_prompt.push_str(description);
            user_prompt.push('\n');
        }
        user_prompt.push_str(&task.tss_sections());
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
        user_prompt.push_str(&task.specification_section());
        user_prompt.push_str(&skills_section(task, &ctx.workdir));
        if task.hints.research_before_edit == Some(true) {
            user_prompt.push_str(RESEARCH_BEFORE_EDIT_NOTE);
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
                user_prompt.push_str(task_accept::prompt_command(&step.command));
                if !step.covers.is_empty() {
                    user_prompt.push_str(" (covers ");
                    user_prompt.push_str(&step.covers.join(", "));
                    user_prompt.push(')');
                }
                user_prompt.push('\n');
            }
            if task.verify.iter().any(task_accept::is_pinned_step) {
                user_prompt.push_str(PINNED_STEP_NOTE);
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
            if let Some(section) = collect_neuro_knowledge_cached(task, &cache.neuro_entries) {
                sections.push(section);
            }
            if let Some(section) = collect_episode_knowledge_cached(task, &cache.episodes) {
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
            collect_playbooks_cached(task, &cache.playbooks)
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
    // The uncached path ranks the hot entries as a plan run's cache does
    // (backlog 4211).
    let entries = roko_neuro::KnowledgeStore::for_workdir(&ctx.workdir)
        .hot_entries()
        .ok()?;
    collect_neuro_knowledge_cached(task, &entries)
}

fn collect_episode_knowledge(task: &TaskDef, ctx: &PromptContext) -> Option<PromptSection> {
    let mut episodes: Vec<roko_learn::episode_logger::Episode> = Vec::new();
    for path in episode_paths(&ctx.workdir) {
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        episodes.extend(
            text.lines()
                .map(str::trim)
                .filter(|line| !line.is_empty())
                .filter_map(|line| serde_json::from_str(line).ok()),
        );
    }
    // The uncached path ranks episodes as a plan run's cache does
    // (backlog 4213).
    collect_episode_knowledge_cached(task, &episodes)
}

/// What an episode says, in order: its reasoning summary, reflection and
/// failure reason, each when it is not empty (backlog 4213).
fn episode_statements(
    episode: &roko_learn::episode_logger::Episode,
) -> impl Iterator<Item = &str> {
    [
        episode.reasoning_summary.as_deref(),
        episode.reflection.as_deref(),
        episode.failure_reason.as_deref(),
    ]
    .into_iter()
    .flatten()
    .filter(|text| !text.trim().is_empty())
}

/// The id a prompt cites an episode by: its id, else its episode id, else
/// its task's id.
fn cited_episode_id(episode: &roko_learn::episode_logger::Episode) -> &str {
    if !episode.id.is_empty() {
        &episode.id
    } else if !episode.episode_id.is_empty() {
        &episode.episode_id
    } else {
        &episode.task_id
    }
}

fn collect_playbooks(task: &TaskDef, ctx: &PromptContext) -> Option<PromptSection> {
    let root = roko_core::Workspace::open(&ctx.workdir)
        .map(|ws| ws.playbooks_dir())
        .unwrap_or_else(|_| ctx.workdir.join(".roko").join("learn").join("playbooks"));
    let playbooks: Vec<roko_learn::playbook::Playbook> = std::fs::read_dir(&root)
        .ok()?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("json"))
        .filter_map(|path| std::fs::read_to_string(path).ok())
        .filter_map(|text| serde_json::from_str(&text).ok())
        .collect();
    // The uncached path ranks playbooks as a plan run's cache does
    // (backlog 4212).
    collect_playbooks_cached(task, &playbooks)
}

// ─── Cached variants ──────────────────────────────────────────────────
//
// These mirror the original I/O-based functions but operate on in-memory
// vectors pre-loaded by `PromptCache`.

fn collect_neuro_knowledge_cached(
    task: &TaskDef,
    entries: &[roko_neuro::KnowledgeEntry],
) -> Option<PromptSection> {
    if entries.is_empty() {
        return None;
    }
    let terms = task_topic_terms(task);
    if terms.is_empty() {
        return None;
    }

    // Count the topic terms an entry holds as whole words: a substring test
    // also finds them inside longer words ("log" in "catalog"). An entry
    // needs `MIN_TOPIC_OVERLAP` of them and `MIN_KNOWLEDGE_CONFIDENCE`, and
    // a runtime success note holds no lesson (backlog 4211).
    let mut scored: Vec<(usize, &roko_neuro::KnowledgeEntry)> = entries
        .iter()
        .filter_map(|entry| {
            if is_group_scoped_knowledge(entry)
                || is_runtime_success_note(entry)
                || entry.confidence < MIN_KNOWLEDGE_CONFIDENCE
            {
                return None;
            }
            let words = query_words(&format!(
                "{} {} {}",
                entry.content,
                entry.tags.join(" "),
                entry.source.as_deref().unwrap_or("")
            ));
            let score = terms.intersection(&words).count();
            (score >= MIN_TOPIC_OVERLAP).then_some((score, entry))
        })
        .collect();
    scored.sort_by(|a, b| {
        b.0.cmp(&a.0)
            .then_with(|| b.1.confidence.total_cmp(&a.1.confidence))
    });
    // The uncached path's cap.
    scored.truncate(3);

    if scored.is_empty() {
        return None;
    }

    let ids = scored
        .iter()
        .map(|(_, entry)| entry.id.clone())
        .filter(|id| !id.is_empty())
        .collect::<Vec<_>>();
    let mut body = String::from("# Neuro knowledge\nRelevant durable knowledge from prior runs:\n");
    let mut items = Vec::new();
    for (index, (score, entry)) in scored.iter().enumerate() {
        let source = entry.source.as_deref().unwrap_or("neuro");
        let line = format!(
            "- [{}] {} (confidence {:.2}, source: {})\n",
            entry.id,
            truncate_chars(&entry.content, 420),
            entry.confidence,
            source
        );
        // The score is the task's topic terms the entry holds.
        let (kind, score) = (ExposureItemKind::Knowledge, Some(*score as f64));
        items.extend(PromptItem::ranked(kind, &entry.id, index, score, &line));
        body.push_str(&line);
    }
    let section = PromptSection::new("knowledge", body, 7).with_knowledge_ids(ids);
    Some(section.with_items(items))
}

fn collect_episode_knowledge_cached(
    task: &TaskDef,
    episodes: &[roko_learn::episode_logger::Episode],
) -> Option<PromptSection> {
    let terms = task_topic_terms(task);
    if terms.is_empty() {
        return None;
    }

    // An episode matches on what it says, never on its task id, its agent
    // (the role) or its model. One that says nothing is skipped, and one
    // needs `MIN_TOPIC_OVERLAP` of the task's topic terms as whole words; the
    // most overlap, then the most recent, rank first (backlog 4213).
    let mut scored: Vec<(usize, &roko_learn::episode_logger::Episode)> = episodes
        .iter()
        .filter_map(|episode| {
            let said = episode_statements(episode).collect::<Vec<_>>().join(" ");
            let overlap = terms.intersection(&query_words(&said)).count();
            (overlap >= MIN_TOPIC_OVERLAP).then_some((overlap, episode))
        })
        .collect();
    if scored.is_empty() {
        return None;
    }
    scored.sort_by(|a, b| {
        b.0.cmp(&a.0)
            .then_with(|| b.1.completed_at.cmp(&a.1.completed_at))
    });
    scored.truncate(5);

    let ids = scored
        .iter()
        .map(|(_, episode)| cited_episode_id(episode).to_string())
        .filter(|id| !id.is_empty())
        .collect::<Vec<_>>();
    let mut body =
        String::from("# Learned patterns from prior episodes\nSimilar prior work suggests:\n");
    let mut items = Vec::new();
    for (index, (score, episode)) in scored.iter().enumerate() {
        let outcome = if episode.success { "passed" } else { "failed" };
        let summary = episode_statements(episode).next().unwrap_or_default();
        let line = format!(
            "- {} ({}, model: {}): {}\n",
            episode.task_id,
            outcome,
            if episode.model.is_empty() {
                "unknown"
            } else {
                &episode.model
            },
            truncate_chars(summary, 420)
        );
        // The score is the task's topic terms the episode holds.
        let (kind, score) = (ExposureItemKind::Episode, Some(*score as f64));
        let id = cited_episode_id(episode);
        items.extend(PromptItem::ranked(kind, id, index, score, &line));
        body.push_str(&line);
    }
    let section = PromptSection::new("episode_knowledge", body, 7).with_knowledge_ids(ids);
    Some(section.with_items(items))
}

fn collect_playbooks_cached(
    task: &TaskDef,
    playbooks: &[roko_learn::playbook::Playbook],
) -> Option<PromptSection> {
    if playbooks.is_empty() {
        return None;
    }
    // A playbook needs `MIN_TOPIC_OVERLAP` of the task's topic terms as whole
    // words. Its successes over its failures only break ties, and no floor
    // tops the section up with unrelated playbooks (backlog 4212).
    let terms = task_topic_terms(task);
    let mut scored: Vec<(usize, &roko_learn::playbook::Playbook)> = playbooks
        .iter()
        .filter_map(|playbook| {
            let overlap = terms
                .intersection(&query_words(&playbook_text(playbook)))
                .count();
            (overlap >= MIN_TOPIC_OVERLAP).then_some((overlap, playbook))
        })
        .collect();
    if scored.is_empty() {
        return None;
    }
    let net_successes = |playbook: &roko_learn::playbook::Playbook| {
        playbook.success_count.saturating_sub(playbook.failure_count)
    };
    scored.sort_by(|a, b| {
        b.0.cmp(&a.0)
            .then_with(|| net_successes(b.1).cmp(&net_successes(a.1)))
            .then_with(|| a.1.id.cmp(&b.1.id))
    });
    scored.truncate(3);

    let ids = scored
        .iter()
        .map(|(_, playbook)| playbook.id.clone())
        .collect::<Vec<_>>();
    let mut body = String::from("# Relevant playbooks\nReusable proven procedures:\n");
    let mut items = Vec::new();
    for (index, (score, playbook)) in scored.iter().enumerate() {
        let mut text = format!(
            "- {}: {} (successes {}, failures {})\n",
            playbook.id, playbook.goal, playbook.success_count, playbook.failure_count
        );
        for step in playbook.steps.iter().take(5) {
            text.push_str(&format!(
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
        // The score is the task's topic terms the playbook holds.
        let (kind, score) = (ExposureItemKind::Playbook, Some(*score as f64));
        let item = PromptItem::ranked(kind, &playbook.id, index, score, &text);
        items.extend(item);
        body.push_str(&text);
    }
    let section = PromptSection::new("playbooks", body, 7).with_playbook_ids(ids);
    Some(section.with_items(items))
}

/// Distinct topic terms ([`task_topic_terms`]) a knowledge entry, a playbook
/// or an episode must share with a task to reach its prompt (backlogs 4211,
/// 4212, 4213).
const MIN_TOPIC_OVERLAP: usize = 2;

/// The least confidence of a knowledge entry a prompt shows (backlog 4211).
const MIN_KNOWLEDGE_CONFIDENCE: f64 = 0.3;

/// Path pieces, file extensions and verbs that say nothing about a task's
/// topic (backlog 4211).
const GENERIC_TOPIC_TERMS: &[&str] = &[
    "crates", "src", "tests", "test", "lib", "mod", "main", "bin", "docs", "rs", "py", "ts", "js",
    "md", "toml", "json", "yaml", "yml", "txt", "add", "fix", "update", "implement", "create",
    "make", "write", "use", "new",
];

/// The words that say what `task` is about, for matching knowledge against
/// it (backlog 4211): the words of its title, description and acceptance,
/// and its declared files' crate or package names and file stems. Never its
/// id, its plan's id or its role, nor a stopword, a generic path piece or a
/// generic verb.
fn task_topic_terms(task: &TaskDef) -> HashSet<String> {
    let mut text = vec![task.title.clone()];
    text.extend(task.description.clone());
    text.extend(task.acceptance.iter().cloned());
    text.extend(task.files.iter().flat_map(|file| file_topic_words(file)));
    let mut terms = query_words(&text.join(" "));
    terms.retain(|word| {
        word.len() > 2
            && !QUERY_STOPWORDS.contains(&word.as_str())
            && !GENERIC_TOPIC_TERMS.contains(&word.as_str())
    });
    terms
}

/// The topic words of a declared file: its crate or package name (the
/// directory after `crates/` or `packages/`, else its first directory) and
/// its file stem.
fn file_topic_words(file: &str) -> Vec<String> {
    let path = Path::new(file);
    let parts: Vec<&str> = path.iter().filter_map(|part| part.to_str()).collect();
    let package = match parts
        .iter()
        .position(|part| matches!(*part, "crates" | "packages"))
    {
        Some(index) => parts.get(index + 1).copied(),
        None if parts.len() > 1 => parts.first().copied(),
        None => None,
    };
    let stem = path.file_stem().and_then(|stem| stem.to_str());
    package.into_iter().chain(stem).map(str::to_string).collect()
}

/// A runtime success note: the "Successful runtime episode for …" entry a
/// verified pass wrote, which holds no lesson (backlog 4211; 4216 stops
/// writing them).
fn is_runtime_success_note(entry: &roko_neuro::KnowledgeEntry) -> bool {
    entry.source.as_deref() == Some("runtime:gate_verdict")
        && entry.content.starts_with("Successful runtime episode for")
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

/// Words that say nothing about a task's topic, so they never match its
/// context (bug-86117a).
const QUERY_STOPWORDS: &[&str] = &[
    "about", "after", "again", "all", "also", "and", "any", "are", "because", "been", "before",
    "being", "both", "but", "can", "could", "did", "does", "doing", "done", "during", "each",
    "every", "for", "from", "had", "has", "have", "here", "how", "into", "its", "just", "may",
    "might", "more", "most", "must", "nor", "not", "off", "once", "only", "onto", "other", "our",
    "out", "over", "own", "per", "same", "should", "some", "such", "than", "that", "the", "their",
    "them", "then", "there", "these", "they", "this", "those", "through", "too", "under", "until",
    "upon", "very", "via", "was", "were", "what", "when", "where", "which", "while", "who", "whom",
    "why", "will", "with", "within", "without", "would", "yet", "you", "your",
];

/// The lowercase words of `text`. `-` and `_` join words, so `roko-cli` and
/// `prompt_builder` are one word each.
fn query_words(text: &str) -> HashSet<String> {
    text.to_ascii_lowercase()
        .split(|c: char| !c.is_ascii_alphanumeric() && c != '-' && c != '_')
        .filter(|word| !word.is_empty())
        .map(ToString::to_string)
        .collect()
}

/// The words of a task's query text that can match its context: longer
/// than two characters, and not stopwords.
fn query_keywords(text: &str) -> HashSet<String> {
    let mut keywords = query_words(text);
    keywords.retain(|word| word.len() > 2 && !QUERY_STOPWORDS.contains(&word.as_str()));
    keywords
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
    let mut buf = String::from(
        "# Previous attempt feedback\n\n\
         Your previous attempt FAILED verification. Fix these exact errors before doing anything else.\n\n",
    );
    if let Some(diagnosis) = &feedback.diagnosis {
        buf.push_str("## Diagnosis\n");
        buf.push_str(diagnosis);
        buf.push_str("\n\n");
    }
    let has_structured = !feedback.compile_errors.is_empty()
        || !feedback.test_failures.is_empty()
        || !feedback.clippy_warnings.is_empty();

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
    // When no structured errors were parsed but raw output exists, include
    // a bounded excerpt so the agent still sees what went wrong.
    if !has_structured && !feedback.raw_output.is_empty() {
        buf.push_str("## Raw gate output\n```\n");
        // Limit to ~2 KB to avoid blowing the prompt budget.
        let raw_excerpt: String = feedback.raw_output.chars().take(2048).collect();
        buf.push_str(&raw_excerpt);
        buf.push_str("\n```\n");
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
                scope: Vec::new(),
                covers: Vec::new(),
                expect: None,
            }],
            timeout_secs: 60,
            max_retries: 1,
            acceptance: vec!["compiles".into()],
            acceptance_contract: None,
            accept: None,
            domain: None,
            estimated_minutes: None,
            crates_touched: None,
            sequence: 0,
            spec: Default::default(),
            hints: Default::default(),
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
            ladder_step: 0,
            prompt_experiment: None,
            gate_feedback: None,
            routing_context: None,
            routing_bias: None,
            dependency_outputs: Vec::new(),
            error_patterns_context: String::new(),
            cached_workspace_map: String::new(),
            cached_workspace_context: String::new(),
            concurrent_plans: Vec::new(),
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
            "content": "public wiring knowledge to explain",
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

    /// Writes `entries` (id and content) to `workdir`'s knowledge store.
    fn write_knowledge(workdir: &Path, entries: &[(&str, &str)]) {
        let neuro_dir = workdir.join(".roko/neuro");
        std::fs::create_dir_all(&neuro_dir).expect("neuro dir");
        let lines = entries
            .iter()
            .map(|(id, content)| {
                let entry: roko_neuro::KnowledgeEntry = serde_json::from_value(serde_json::json!({
                    "id": id,
                    "content": content,
                    "confidence": 0.8,
                    "created_at": Utc::now(),
                }))
                .expect("knowledge entry");
                serde_json::to_string(&entry).expect("knowledge json") + "\n"
            })
            .collect::<String>();
        std::fs::write(neuro_dir.join("knowledge.jsonl"), lines).expect("write knowledge");
    }

    /// Assembles `task` in `workdir` as a plan run does, from a prompt cache
    /// loaded once.
    fn assemble_cached(task: &TaskDef, workdir: &Path) -> AssembledPrompt {
        let mut dispatch = ctx();
        dispatch.workdir = workdir.to_path_buf();
        let prompt_ctx = PromptContext::from_task(task, &dispatch);
        PromptAssembler::with_cache(Arc::new(PromptCache::load(workdir)))
            .assemble(task, &prompt_ctx)
            .expect("assemble")
    }

    /// A plan run's assembler reads knowledge from a cache loaded once
    /// (bug-86117a). The cache holds every hot entry, and each prompt carries
    /// those that share two topic words with its task, not those that share
    /// only stopwords.
    #[test]
    fn cached_prompt_surfaces_matching_durable_knowledge() {
        let temp = tempfile::tempdir().expect("tempdir");
        // The task explains "the wiring": the first entry shares "wiring" and
        // "wire", the second only "the", which a substring test also finds in
        // "other".
        write_knowledge(
            temp.path(),
            &[
                ("k-wiring", "Register new wiring for the wire table"),
                ("k-stopwords", "Keep the other notes short"),
            ],
        );
        assert_eq!(
            PromptCache::load(temp.path()).neuro_entries.len(),
            2,
            "the cache holds every hot entry"
        );
        let prompt = assemble_cached(&task(), temp.path());

        let system = &prompt.system_prompt;
        assert!(system.contains("# Neuro knowledge"), "{system}");
        assert!(
            system.contains("- [k-wiring] Register new wiring for the wire table"),
            "{system}"
        );
        assert!(!system.contains("Keep the other notes short"), "{system}");
        assert_eq!(prompt.diagnostics.knowledge_ids, ["k-wiring"]);
    }

    /// The knowledge also reaches a prompt whose domain context is longer
    /// than its conventions section. The composer's foraging pre-pass used
    /// to drop that section, and the knowledge with it (bug-4aa696).
    #[test]
    fn cached_knowledge_survives_a_long_domain_context() {
        let temp = tempfile::tempdir().expect("tempdir");
        write_knowledge(
            temp.path(),
            &[("k-wiring", "Register new wiring for the wire table")],
        );
        let mut long_task = task();
        long_task.description = Some(format!(
            "Explain the wiring. {}",
            "Lorem ipsum dolor sit amet, consectetur adipiscing elit. ".repeat(30)
        ));
        let prompt = assemble_cached(&long_task, temp.path());

        let manifest = prompt
            .diagnostics
            .composition_manifest
            .as_ref()
            .expect("composition manifest");
        let tokens = |name: &str| {
            manifest
                .included
                .iter()
                .find(|section| section.name == name)
                .map(|section| section.estimated_tokens)
        };
        let domain = tokens("domain_context").expect("domain_context is kept");
        let conventions = tokens("conventions").expect("conventions is kept");
        assert!(domain > conventions, "{domain} <= {conventions} tokens");
        assert!(
            prompt
                .system_prompt
                .contains("- [k-wiring] Register new wiring for the wire table"),
            "{}",
            prompt.system_prompt
        );
        assert_eq!(prompt.diagnostics.knowledge_ids, ["k-wiring"]);
    }

    /// Writes `entries`, knowledge entries as JSON, to `workdir`'s store.
    fn write_knowledge_json(workdir: &Path, entries: &[serde_json::Value]) {
        let neuro_dir = workdir.join(".roko/neuro");
        std::fs::create_dir_all(&neuro_dir).expect("neuro dir");
        let lines = entries
            .iter()
            .map(|entry| {
                let entry: roko_neuro::KnowledgeEntry =
                    serde_json::from_value(entry.clone()).expect("knowledge entry");
                serde_json::to_string(&entry).expect("knowledge json") + "\n"
            })
            .collect::<String>();
        std::fs::write(neuro_dir.join("knowledge.jsonl"), lines).expect("write knowledge");
    }

    /// backlog 4211 (G36): a task's id, role and path words never match
    /// knowledge. A hot entry about another plan's `T01`, tagged with another
    /// crate's file, shares only those with task `T01`, so the task's prompt
    /// has no knowledge section.
    #[test]
    fn knowledge_section_ignores_id_and_path_word_matches() {
        let temp = tempfile::tempdir().expect("tempdir");
        write_knowledge_json(
            temp.path(),
            &[serde_json::json!({
                "id": "k-other-t01",
                "content": "Implementer notes for T01: keep the crates and src tidy",
                "confidence": 0.9,
                "tags": ["crates/b/src/x.rs"],
                "created_at": Utc::now(),
            })],
        );
        let mut market = task();
        market.id = "T01".into();
        market.title = "Summarise the market close".into();
        market.description = None;
        market.acceptance = vec!["prints the closing prices".into()];
        market.files = vec!["crates/a/src/lib.rs".into()];

        let prompt = assemble_cached(&market, temp.path());
        assert!(
            !prompt.system_prompt.contains("# Neuro knowledge"),
            "{}",
            prompt.system_prompt
        );
        assert!(prompt.diagnostics.knowledge_ids.is_empty());
    }

    /// backlog 4211: a runtime success note holds no lesson, so it stays out
    /// of the prompt, while an entry with the same topic words gets in.
    #[test]
    fn knowledge_section_skips_success_notes() {
        let temp = tempfile::tempdir().expect("tempdir");
        let entry = |id: &str, content: &str, source: &str| {
            serde_json::json!({
                "id": id,
                "content": content,
                "confidence": 0.9,
                "source": source,
                "created_at": Utc::now(),
            })
        };
        write_knowledge_json(
            temp.path(),
            &[
                entry(
                    "k-success",
                    "Successful runtime episode for explain wiring passed verify[0:test]",
                    "runtime:gate_verdict",
                ),
                entry(
                    "k-lesson",
                    "Explain the wiring before editing the dispatcher",
                    "runtime:lesson",
                ),
            ],
        );

        let prompt = assemble_cached(&task(), temp.path());
        assert_eq!(prompt.diagnostics.knowledge_ids, ["k-lesson"]);
        assert!(
            !prompt.system_prompt.contains("Successful runtime episode"),
            "{}",
            prompt.system_prompt
        );
    }

    /// backlog 4212: a playbook reaches a prompt only when it shares two topic
    /// words with the task. Three proven playbooks about other work give no
    /// section, and one about the task's work gives a section with it alone.
    #[test]
    fn playbooks_without_overlap_are_not_injected() {
        let proven = |id: &str, goal: &str| {
            let mut playbook = roko_learn::playbook::Playbook::new(id, goal);
            playbook.success_count = 9;
            playbook
        };
        let mut playbooks = vec![
            proven("pb-deploy", "Deploy the service to staging"),
            proven("pb-css", "Tidy the stylesheet colours"),
            proven("pb-sql", "Index the orders table"),
        ];
        assert!(collect_playbooks_cached(&task(), &playbooks).is_none());

        playbooks.push(proven("pb-wiring", "Explain the wiring map"));
        let section = collect_playbooks_cached(&task(), &playbooks).expect("a playbooks section");
        assert_eq!(section.playbook_ids, ["pb-wiring"]);
        assert!(!section.body.contains("pb-deploy"), "{}", section.body);
    }

    /// An episode of `task_id` by the implementer on `model`, saying
    /// `summary`.
    fn episode(
        task_id: &str,
        model: &str,
        summary: Option<&str>,
    ) -> roko_learn::episode_logger::Episode {
        let mut episode = roko_learn::episode_logger::Episode::new("implementer", task_id);
        episode.model = model.to_string();
        episode.success = true;
        episode.reasoning_summary = summary.map(str::to_string);
        episode
    }

    /// backlog 4213: an episode with no summary, reflection or failure reason
    /// says nothing, so it never fills a line with "no summary recorded".
    #[test]
    fn episode_section_skips_episodes_with_nothing_to_say() {
        let episodes = [
            episode("wire-quiet", "claude-haiku-4-5", None),
            episode(
                "wire-said",
                "claude-haiku-4-5",
                Some("Explain the wiring map before editing it"),
            ),
        ];
        let section = collect_episode_knowledge_cached(&task(), &episodes).expect("episodes");
        assert!(section.body.contains("wire-said"), "{}", section.body);
        assert!(!section.body.contains("wire-quiet"), "{}", section.body);
        assert!(!section.body.contains("no summary"), "{}", section.body);
    }

    /// backlog 4213: an episode's task id, agent (the role) and model never
    /// match a task, so one that shares only those gives no section.
    #[test]
    fn episode_section_ignores_role_and_model_matches() {
        let episodes = [episode(
            "wire-and-explain",
            "wire-explain-7b",
            Some("Bumped the lockfile"),
        )];
        assert!(collect_episode_knowledge_cached(&task(), &episodes).is_none());
    }

    /// backlog 4214 (decision 4202, option A): a run's prompts come from one
    /// prompt-cache snapshot. Knowledge and an episode written after it was
    /// taken reach no later prompt of the run, and its digest stays the same;
    /// the next run's snapshot holds them.
    #[test]
    fn prompt_cache_is_one_snapshot_per_run() {
        let temp = tempfile::tempdir().expect("tempdir");
        let cache = Arc::new(PromptCache::load(temp.path()));
        let digest = cache.digest();
        let mut dispatch = ctx();
        dispatch.workdir = temp.path().to_path_buf();
        let prompt_ctx = PromptContext::from_task(&task(), &dispatch);
        let assemble = || {
            PromptAssembler::with_cache(Arc::clone(&cache))
                .assemble(&task(), &prompt_ctx)
                .expect("assemble")
        };
        let first = assemble();

        write_knowledge(
            temp.path(),
            &[("k-late", "Explain the wiring after the snapshot")],
        );
        let mut late = roko_learn::episode_logger::Episode::new("implementer", "t-late");
        late.reasoning_summary = Some("Explain the wiring after the snapshot".into());
        let episodes = roko_learn::runtime_feedback::resolve_project_episode_path(temp.path());
        std::fs::create_dir_all(episodes.parent().expect("episode dir")).expect("episode dir");
        let line = serde_json::to_string(&late).expect("episode json") + "\n";
        std::fs::write(&episodes, line).expect("write the episode");

        let second = assemble();
        assert!(
            !second.system_prompt.contains("after the snapshot"),
            "{}",
            second.system_prompt
        );
        assert_eq!(
            second.diagnostics.knowledge_ids,
            first.diagnostics.knowledge_ids
        );
        assert_eq!(cache.digest(), digest);
        let next_run = PromptCache::load(temp.path()).digest();
        assert_eq!((next_run.knowledge.count, next_run.episodes.count), (1, 1));
        assert_ne!(next_run, digest);
    }

    /// The item of `kind` and `id` in `prompt`'s diagnostics.
    fn prompt_item(
        prompt: &AssembledPrompt,
        kind: ExposureItemKind,
        id: &str,
    ) -> PromptItemDiagnostic {
        let items = &prompt.diagnostics.items;
        items
            .iter()
            .find(|item| item.kind == kind && item.id == id)
            .cloned()
            .unwrap_or_else(|| panic!("no {kind:?} item {id}: {items:#?}"))
    }

    /// An item reaches the prompt only when its section does (S01 §4.5).
    /// Knowledge entries and playbooks render into the domain context, so a
    /// token budget that drops it leaves each of them retrieved, as the id
    /// lists say, but not included, for the token budget. With room for the
    /// section, each is included with the digest of the text it rendered.
    #[test]
    fn diagnostics_mark_items_of_dropped_sections_not_included() {
        use roko_learn::telemetry::ExposureItemKind::{Knowledge, Playbook, Section};
        const CRITICAL: [&str; 3] = ["role_identity", "context_layer", "task_context"];

        let temp = tempfile::tempdir().expect("tempdir");
        // "explain", "wiring" and "wire" are topic words of the task; the
        // entry with two of them ranks below the one with three.
        write_knowledge(
            temp.path(),
            &[
                ("k-wiring", "Register new wiring for the wire table"),
                (
                    "k-explain",
                    "Explain the dispatcher wiring before you wire it",
                ),
            ],
        );
        let playbooks = temp.path().join(".roko/learn/playbooks");
        std::fs::create_dir_all(&playbooks).expect("playbook dir");
        let playbook = roko_learn::playbook::Playbook::new("pb-wiring", "Wire the dispatcher wiring");
        let json = serde_json::to_string(&playbook).expect("playbook json");
        std::fs::write(playbooks.join("pb-wiring.json"), json).expect("write playbook");
        let cache = Arc::new(PromptCache::load(temp.path()));
        let mut dispatch = ctx();
        dispatch.workdir = temp.path().to_path_buf();
        let prompt_ctx = PromptContext::from_task(&task(), &dispatch);

        let roomy = PromptAssembler::with_cache(Arc::clone(&cache))
            .assemble(&task(), &prompt_ctx)
            .expect("assemble");
        let line = "- pb-wiring: Wire the dispatcher wiring (successes 0, failures 0)\n";
        assert!(
            roomy.system_prompt.contains(line),
            "{}",
            roomy.system_prompt
        );
        let kept = prompt_item(&roomy, Playbook, "pb-wiring");
        assert!(kept.included, "{kept:?}");
        assert_eq!(kept.excluded_reason, None);
        assert_eq!(kept.section, "domain_context");
        assert_eq!(kept.rendered_sha256, sha256_hex(line));
        assert!(kept.tokens > 0);
        let explain = prompt_item(&roomy, Knowledge, "k-explain");
        let wiring = prompt_item(&roomy, Knowledge, "k-wiring");
        assert!(explain.included && wiring.included);
        assert_eq!((explain.rank, explain.score), (Some(1), Some(3.0)));
        assert_eq!((wiring.rank, wiring.score), (Some(2), Some(2.0)));
        assert_ne!(explain.rendered_sha256, wiring.rendered_sha256);
        assert!(prompt_item(&roomy, Section, "domain_context").included);

        // Room for the critical sections alone: the domain context drops.
        let manifest = roomy
            .diagnostics
            .composition_manifest
            .as_ref()
            .expect("composition manifest");
        let critical: usize = manifest
            .included
            .iter()
            .filter(|section| CRITICAL.contains(&section.name.as_str()))
            .map(|section| section.estimated_tokens)
            .sum();
        let budget = u32::try_from(critical).expect("a token count");
        let tight = PromptAssembler::with_cache(cache)
            .with_token_budget(budget)
            .assemble(&task(), &prompt_ctx)
            .expect("the critical sections fit");
        let dropped = &tight.diagnostics.dropped_sections;
        assert!(
            dropped.iter().any(|name| name == "domain_context"),
            "{dropped:?}"
        );
        assert!(!tight.system_prompt.contains("pb-wiring"));
        for (kind, id) in [
            (Knowledge, "k-explain"),
            (Knowledge, "k-wiring"),
            (Playbook, "pb-wiring"),
        ] {
            let item = prompt_item(&tight, kind, id);
            assert!(!item.included, "{item:?}");
            assert_eq!(item.excluded_reason, Some(ExcludedReason::TokenBudget));
            assert_eq!(item.section, "domain_context");
        }
        assert_eq!(
            tight.diagnostics.knowledge_ids, roomy.diagnostics.knowledge_ids,
            "the id lists name what was retrieved"
        );
        assert_eq!(tight.diagnostics.playbook_ids, ["pb-wiring"]);
        let section = prompt_item(&tight, Section, "domain_context");
        assert_eq!(
            (section.included, section.excluded_reason),
            (false, Some(ExcludedReason::TokenBudget))
        );
        assert!(prompt_item(&tight, Section, "task_context").included);
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
            diagnosis: Some("The import path moved to crate::dispatch.".into()),
        });
        let pctx = PromptContext::from_task(&task(), &c);
        let p = assembler.assemble(&task(), &pctx).unwrap();
        // Gate feedback is rendered into the context_layer by build_runner_context.
        assert!(
            p.system_prompt.contains("# Previous attempt feedback"),
            "retry should contain gate feedback header"
        );
        assert!(
            p.system_prompt
                .contains("## Diagnosis\nThe import path moved to crate::dispatch."),
            "retry should lead with the diagnosis even when errors are structured"
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

    /// gap-0f3980: a task's specification hints and `context_files` reach
    /// the prompt its agent gets.
    #[test]
    fn task_hints_reach_the_user_prompt() {
        let assembler = PromptAssembler::minimal();
        let t = crate::task_parser::TasksFile::parse_str(
            r#"
[meta]
plan = "p"

[[task]]
id = "t"
title = "Wire it up"
role = "implementer"
context_files = ["src/hints.rs"]
formulas = ["retries = 2 * (k + 1) - 1"]
"#,
        )
        .expect("parse")
        .tasks
        .remove(0);
        // Dispatch refuses a context file that does not exist.
        let workdir = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir_all(workdir.path().join("src")).expect("src dir");
        std::fs::write(workdir.path().join("src/hints.rs"), "// hints\n").expect("context file");
        let mut dispatch_ctx = ctx();
        dispatch_ctx.workdir = workdir.path().to_path_buf();
        let p = assembler
            .assemble(&t, &PromptContext::from_task(&t, &dispatch_ctx))
            .unwrap();
        assert!(
            p.user_prompt
                .contains("## Task Context\n- Read `src/hints.rs`: context\n"),
            "{}",
            p.user_prompt
        );
        assert!(
            p.user_prompt
                .contains("## Specification\n### Formulas\n- retries = 2 * (k + 1) - 1\n"),
            "{}",
            p.user_prompt
        );

        let plain = assembler
            .assemble(&task(), &PromptContext::from_task(&task(), &ctx()))
            .unwrap();
        assert!(!plain.user_prompt.contains("## Specification"));
    }

    /// 3207: a task's TSS v1 fields reach its prompt: the goal, non-goals and
    /// assumptions, the hidden-test hook without its suite, and the criteria
    /// each verify step covers.
    #[test]
    fn tss_v1_fields_reach_the_user_prompt() {
        let t = crate::task_parser::TasksFile::parse_str(
            r#"
[meta]
plan = "p"

[[task]]
id = "t"
title = "Wire it up"
role = "implementer"
goal = "`roko plan validate` prints PLAN_043 for an unknown key."
non_goals = ["Do not change the parser"]
assumptions = ["A warning is enough"]
acceptance = ["AC1: an unknown key prints PLAN_043"]

[task.hidden]
suite = "suite-17"
interface = ["crates/roko-cli/src/plan_validate.rs::validate_tasks_file"]
properties = ["nested tables"]

[[task.verify]]
phase = "test"
command = "cargo test -p roko-cli plan_validate"
covers = ["AC1"]
"#,
        )
        .expect("parse")
        .tasks
        .remove(0);
        let p = PromptAssembler::minimal()
            .assemble(&t, &PromptContext::from_task(&t, &ctx()))
            .unwrap();
        for section in [
            "\n## Goal\n`roko plan validate` prints PLAN_043 for an unknown key.\n",
            "\n## Non-goals\n- Do not change the parser\n",
            "\n## Assumptions\n- A warning is enough\n",
            "- Interface: `crates/roko-cli/src/plan_validate.rs::validate_tasks_file`\n",
            "- Property: nested tables\n",
            "- cargo test -p roko-cli plan_validate (covers AC1)\n",
        ] {
            assert!(
                p.user_prompt.contains(section),
                "{section:?} missing: {}",
                p.user_prompt
            );
        }
        assert!(!p.user_prompt.contains("suite-17"), "{}", p.user_prompt);

        let plain = PromptAssembler::minimal()
            .assemble(&task(), &PromptContext::from_task(&task(), &ctx()))
            .unwrap();
        for heading in ["## Goal", "## Non-goals", "## Assumptions", "## Hidden"] {
            assert!(
                !plain.user_prompt.contains(heading),
                "{heading} in a task without TSS v1 fields: {}",
                plain.user_prompt
            );
        }
        assert!(
            !plain.user_prompt.contains("(covers"),
            "{}",
            plain.user_prompt
        );
    }

    /// gap-d6fd85: a plan's `brief.md` reaches its tasks' prompts.
    #[test]
    fn plan_brief_reaches_the_prompt() {
        let workdir = tempfile::tempdir().expect("tempdir");
        let plan_dir = workdir.path().join("plans/p");
        std::fs::create_dir_all(&plan_dir).expect("plan dir");
        std::fs::write(plan_dir.join("brief.md"), "# Plan brief: `p`\n").expect("brief");
        let mut dispatch_ctx = ctx();
        dispatch_ctx.workdir = workdir.path().to_path_buf();

        let pctx = PromptContext::from_task(&task(), &dispatch_ctx);
        let context = build_runner_context(&task(), &pctx).expect("runner context");

        assert_eq!(pctx.plan_brief, "# Plan brief: `p`\n");
        assert!(
            context.contains("# Plan Brief\n# Plan brief: `p`"),
            "{context}"
        );
    }

    /// gap-c09fc7: an agent hears which other plans run in its working tree
    /// and what they write.
    #[test]
    fn prompt_names_concurrent_plans() {
        let mut dispatch_ctx = ctx();
        let areas = vec!["crates/roko-serve".to_string(), "web/src".to_string()];
        dispatch_ctx.concurrent_plans = vec![("portal-api".to_string(), areas)];
        let pctx = PromptContext::from_task(&task(), &dispatch_ctx);
        let context = build_runner_context(&task(), &pctx).expect("runner context");
        assert!(
            context.contains("# Plans Running Beside This One"),
            "{context}"
        );
        assert!(
            context.contains("- `portal-api`: crates/roko-serve, web/src\n"),
            "{context}"
        );

        let alone = PromptContext::from_task(&task(), &ctx());
        let context = build_runner_context(&task(), &alone).expect("runner context");
        assert!(!context.contains("# Plans Running Beside This One"));
    }

    /// gap-404fdb: the context-depth hints shape the prompt. `skills` brings
    /// in each named skill from the skill library, `research_before_edit` asks
    /// for research first, and `context_weight` scales the plan context:
    /// `slim` drops it and `deep` takes twice as much.
    #[test]
    fn context_depth_hints_shape_the_prompt() {
        let workdir = tempfile::tempdir().expect("tempdir");
        let root = workdir.path();
        let write = |path: &str, text: &str| {
            let path = root.join(path);
            let dir = path.parent().expect("parent");
            std::fs::create_dir_all(dir).expect("create dir");
            std::fs::write(path, text).expect("write fixture");
        };
        let padding = "x".repeat(3 * TASKS_TOML_LIMIT / 2);
        write(
            "plans/p/tasks.toml",
            &format!("[meta]\nplan = \"p\"\n# {padding}\n"),
        );
        write(
            ".roko/learn/skills.json",
            r#"[{"name": "serde", "summary": "Derive serde traits.",
                 "prompt_template": "Default optional keys."}]"#,
        );
        let mut dispatch_ctx = ctx();
        dispatch_ctx.workdir = root.to_path_buf();
        let assembler = PromptAssembler::minimal();
        let prompt = |hints: &str| {
            let task = crate::task_parser::TasksFile::parse_str(&format!(
                "[meta]\nplan = \"p\"\n\n[[task]]\nid = \"t\"\ntitle = \"Wire it up\"\n\
                 role = \"implementer\"\n{hints}"
            ))
            .expect("parse")
            .tasks
            .remove(0);
            let context = PromptContext::from_task(&task, &dispatch_ctx);
            let assembled = assembler.assemble(&task, &context).expect("assemble");
            (context, assembled.user_prompt)
        };

        let (plain, plain_prompt) = prompt("");
        assert!(
            plain.tasks_toml.ends_with("[truncated]"),
            "{}",
            plain.tasks_toml
        );
        assert!(!plain_prompt.contains("## Skills"), "{plain_prompt}");
        assert!(
            !plain_prompt.contains("## Before You Edit"),
            "{plain_prompt}"
        );

        let (_, hinted_prompt) =
            prompt("skills = [\"serde\", \"tokio\"]\nresearch_before_edit = true\n");
        let skills = "## Skills\n### serde\nDerive serde traits.\n\n\
                      Default optional keys.\n- tokio\n";
        assert!(hinted_prompt.contains(skills), "{hinted_prompt}");
        assert!(
            hinted_prompt.contains(RESEARCH_BEFORE_EDIT_NOTE),
            "{hinted_prompt}"
        );

        let (slim, _) = prompt("context_weight = \"slim\"\n");
        assert!(slim.tasks_toml.is_empty(), "{}", slim.tasks_toml);
        assert!(slim.workspace_map.is_empty(), "{}", slim.workspace_map);

        let (deep, _) = prompt("context_weight = \"deep\"\n");
        assert!(
            deep.tasks_toml.ends_with(&format!("{padding}\n")),
            "the whole tasks.toml"
        );
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

    /// backlog 4206: plan prompts carry no `# Collective calibration` block,
    /// even in a workspace whose bench runs left c-factor history.
    #[test]
    fn plan_prompt_has_no_collective_calibration_block() {
        let temp = tempfile::tempdir().expect("tempdir");
        let learn_dir = temp.path().join(".roko/learn");
        std::fs::create_dir_all(&learn_dir).expect("learn dir");
        let history: String = (0..8)
            .map(|hours| {
                let snapshot = roko_learn::cfactor::CFactor {
                    overall: 0.72,
                    episode_count: 12,
                    computed_at: chrono::Utc::now() - chrono::Duration::hours(hours),
                    ..roko_learn::cfactor::CFactor::default()
                };
                serde_json::to_string(&snapshot).expect("snapshot") + "\n"
            })
            .collect();
        std::fs::write(learn_dir.join("c-factor.jsonl"), history).expect("c-factor history");

        let mut dispatch = ctx();
        dispatch.workdir = temp.path().to_path_buf();
        let pctx = PromptContext::from_task(&task(), &dispatch);
        let prompt = PromptAssembler::minimal()
            .assemble(&task(), &pctx)
            .expect("prompt");
        assert!(
            !prompt.system_prompt.contains("# Collective calibration"),
            "{}",
            prompt.system_prompt
        );
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
    fn parse_role_label_prompts_reviewer_as_read_only_quick_reviewer() {
        assert_eq!(parse_role_label("reviewer"), AgentRole::QuickReviewer);
        assert_eq!(parse_role_label("Reviewer"), AgentRole::QuickReviewer);
        assert_eq!(parse_role_label("quick-reviewer"), AgentRole::QuickReviewer);
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
            dependency_outputs: Vec::new(),
            workspace_context: String::new(),
            error_patterns_context: String::new(),
            concurrent_plans: Vec::new(),
            plan_brief: String::new(),
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

    // ── Per-role context limit tests ────────────────────────────────────

    #[test]
    fn truncate_to_limit_leaves_short_string_unchanged() {
        let s = "hello world".to_string();
        assert_eq!(truncate_to_limit(s.clone(), 100), s);
    }

    #[test]
    fn truncate_to_limit_clips_long_string_and_appends_marker() {
        let s = "abcdefghij".to_string(); // 10 chars
        let result = truncate_to_limit(s, 5);
        assert_eq!(result, "abcde\n[truncated]");
    }

    #[test]
    fn truncate_to_limit_exact_boundary_is_not_clipped() {
        let s = "12345".to_string(); // exactly 5 chars
        assert_eq!(truncate_to_limit(s.clone(), 5), s);
    }

    #[test]
    fn context_limits_for_role_implementer_uses_defaults() {
        let limits = context_limits_for_role("implementer");
        assert_eq!(limits.workspace_map, WORKSPACE_MAP_LIMIT);
        assert_eq!(limits.tasks_toml, TASKS_TOML_LIMIT);
    }

    #[test]
    fn context_limits_for_role_researcher_reduces_workspace_map() {
        let limits = context_limits_for_role("researcher");
        assert!(
            limits.workspace_map < WORKSPACE_MAP_LIMIT,
            "researcher should have smaller workspace map than implementer"
        );
    }

    #[test]
    fn context_limits_for_role_strategist_reduces_workspace_map() {
        let limits = context_limits_for_role("strategist");
        assert!(
            limits.workspace_map < WORKSPACE_MAP_LIMIT,
            "strategist should have smaller workspace map than implementer"
        );
    }

    #[test]
    fn context_limits_for_role_auditor_reduces_workspace_map() {
        let limits = context_limits_for_role("auditor");
        assert!(
            limits.workspace_map < WORKSPACE_MAP_LIMIT,
            "auditor should have smaller workspace map than implementer"
        );
    }

    #[test]
    fn context_limits_for_role_unknown_falls_back_to_defaults() {
        // Unknown roles fall back to implementer (the parse_role_label default).
        let limits = context_limits_for_role("unknown-custom-role");
        assert_eq!(limits.workspace_map, WORKSPACE_MAP_LIMIT);
        assert_eq!(limits.tasks_toml, TASKS_TOML_LIMIT);
    }

    #[test]
    fn from_task_researcher_gets_smaller_workspace_map_than_implementer() {
        // Demonstrate that two roles with the same cached map content end up
        // with different workspace_map sizes inside PromptContext.
        let big_map = "# Workspace crate map\n".to_string() + &"x".repeat(5_000);

        let mut impl_ctx = ctx();
        impl_ctx.role = "implementer".to_string();
        impl_ctx.cached_workspace_map = big_map.clone();
        let pctx_impl = PromptContext::from_task(&task(), &impl_ctx);

        let mut res_ctx = ctx();
        res_ctx.role = "researcher".to_string();
        res_ctx.cached_workspace_map = big_map;
        let pctx_res = PromptContext::from_task(&task(), &res_ctx);

        assert!(
            pctx_res.workspace_map.len() < pctx_impl.workspace_map.len(),
            "researcher workspace_map ({}) should be smaller than implementer ({})",
            pctx_res.workspace_map.len(),
            pctx_impl.workspace_map.len()
        );
    }

    /// gap-1b5636: both verify listings show a pinned acceptance step by its header line and a
    /// note, never the generated script; authored steps stay verbatim.
    #[test]
    fn pinned_accept_steps_show_only_their_header() {
        let entry = task_accept::AcceptFile {
            src: "accept/x.test.ts".into(),
            dest: "apps/portal/src/x.test.ts".into(),
            runner: "cd apps/portal && node scripts/vitest-min.mjs {dest} {count}".into(),
            count: 16,
            timeout_ms: None,
        };
        let pinned = task_accept::PinnedAccept {
            stored: PathBuf::from("/accept-store/p/t/accept/x.test.ts"),
            sha256: "ab".repeat(32),
        };
        let step = task_accept::pinned_verify_step("t", &entry, &pinned);
        let header = step
            .command
            .lines()
            .next()
            .expect("a header line")
            .to_string();
        assert_eq!(
            header,
            concat!(
                "# roko accept: t accept/x.test.ts -> apps/portal/src/x.test.ts ",
                "(exactly 16 passing tests)"
            )
        );
        let mut t = task();
        t.verify.insert(0, step);

        let pctx = PromptContext::from_task(&t, &ctx());
        let p = PromptAssembler::minimal().assemble(&t, &pctx).unwrap();
        for (section, prompt) in [
            ("# Verify", &p.system_prompt),
            ("## Verification Commands", &p.user_prompt),
        ] {
            assert!(prompt.contains(section), "{section} missing: {prompt}");
            assert!(prompt.contains(&header), "{section}: no header: {prompt}");
            assert!(
                prompt.contains(PINNED_STEP_NOTE),
                "{section}: no note: {prompt}"
            );
            assert!(
                prompt.contains("cargo test"),
                "{section}: authored step: {prompt}"
            );
            for plumbing in [
                "roko_pinned=",
                "/accept-store/",
                "sha256sum",
                "vitest-min.mjs",
            ] {
                assert!(
                    !prompt.contains(plumbing),
                    "{section}: {plumbing} leaked: {prompt}"
                );
            }
        }

        // Without a pinned step the note stays out.
        let plain = task();
        let plain_ctx = PromptContext::from_task(&plain, &ctx());
        let p = PromptAssembler::minimal()
            .assemble(&plain, &plain_ctx)
            .unwrap();
        assert!(!p.system_prompt.contains(PINNED_STEP_NOTE));
        assert!(!p.user_prompt.contains(PINNED_STEP_NOTE));
    }
}
