//! Trait abstraction for CLI operations needed by the HTTP server.
//!
//! `roko-serve` depends on this trait rather than directly on `roko-cli`,
//! breaking the circular dependency. The CLI crate provides the concrete
//! implementation.

use std::path::PathBuf;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::bench::BenchConfigOverrides;
use crate::plan_types::{
    CreatePlanOutcome, PlanSourceDto, PlanSummaryDto, PlanTasksDto, PlanValidationDto, RevisionDto,
};
use roko_runtime::cancel::CancelToken;

/// Token usage reported by an LLM provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunResultUsage {
    /// Number of input (prompt) tokens consumed.
    pub input_tokens: u64,
    /// Number of output (completion) tokens generated.
    pub output_tokens: u64,
}

/// Result of a single `run_once()` invocation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunResult {
    /// Whether the overall run succeeded (all gates passed). With no
    /// `gate_results`, `true` only says the runtime finished: nothing
    /// verified the output, and `POST /api/run` reports the run
    /// `unverified`, not a success (G42).
    pub success: bool,
    /// Final text output produced by the run, when available.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_text: Option<String>,
    /// Real token usage from the provider, when available.
    /// Gateway falls back to a character-based heuristic when `None`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub usage: Option<RunResultUsage>,
    /// Structured gate results collected during execution. Empty means no
    /// gate checked the output.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub gate_results: Vec<RuntimeGateResult>,
}

/// Result of generating an implementation plan from a prompt.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanGenerationResult {
    /// Root directory where plan artifacts were generated.
    pub plans_root: PathBuf,
    /// Specific plan directories or files that should be executed.
    pub plan_targets: Vec<PathBuf>,
    /// Plan-related files to attach to the job submission.
    pub artifacts: Vec<PathBuf>,
}

/// Structured gate result collected while executing a plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeGateResult {
    /// Verify name.
    pub gate: String,
    /// Whether the gate passed.
    pub passed: bool,
    /// Human-readable gate detail.
    pub detail: String,
}

/// Result of executing an implementation plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanExecutionResult {
    /// Whether the execution completed successfully.
    pub success: bool,
    /// Text summary or stdout-like output.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_text: Option<String>,
    /// Structured gate results when the runtime can provide them.
    pub gate_results: Vec<RuntimeGateResult>,
}

/// Options forwarded to a plan execution initiated through the HTTP API.
///
/// All fields are optional; the default leaves every choice to the runtime,
/// mirroring a plain `roko plan run` invocation with no flags.
#[derive(Debug, Clone, Default)]
pub struct PlanRunOptions {
    /// Cancellation token observed by the executor.
    ///
    /// When `Some`, the runtime must poll or `select!` this token and abort
    /// the run when it fires, so that `POST /api/plans/{id}/cancel` can reach
    /// a run that executes inside `spawn_blocking` or a detached task.
    pub cancel: Option<CancelToken>,

    /// Start the plan from scratch, discarding any existing checkpoint.
    ///
    /// Mirrors the `--fresh` flag of `roko plan run`.
    pub fresh: bool,

    /// Resume from the last checkpoint even when a run is not currently
    /// paused, overriding the default "run only pending tasks" logic.
    ///
    /// Mirrors the `--force-resume` flag of `roko plan run`.
    pub force_resume: bool,

    /// Restrict a plan-set directory to these plan ids; others are skipped.
    ///
    /// `None` means run all plans discovered under the target.
    pub only_plans: Option<Vec<String>>,

    /// Maximum number of independent plans in a plan-set that may execute
    /// concurrently.
    ///
    /// `None` defers to the workspace `[conductor] max_parallel_plans` setting,
    /// matching the default behaviour of `roko plan run`.
    pub max_parallel_plans: Option<usize>,

    /// Which live-output level the dispatcher should use for this run.
    ///
    /// `None` falls back to `ToolSteps` (the safe default).  Set from
    /// `AppState::effective_live_agent_output()` by plan run handlers so that
    /// the trust level configured at startup flows into every server-side run.
    pub live_agent_output: Option<roko_core::config::serve::LiveAgentOutput>,

    /// The run id the caller returned to its client: the `id` of the 202
    /// from `POST /api/plans/{id}/execute` or `POST /api/plans/execute`.
    ///
    /// The runtime runs under it: the run's events, its status and, for a
    /// fresh single plan, its checkpoint take this id. `None` lets the runtime
    /// mint its own.
    pub run_id: Option<String>,
}

/// Options for a prompt run through [`CliRuntime::run_prompt_plan`].
#[derive(Debug, Clone, Default)]
pub struct PromptPlanOptions {
    /// The run id the caller returned to its client: the one-task plan and
    /// its Graph run take it. `None` lets the runtime mint its own.
    pub run_id: Option<String>,
    /// Cancellation token the run observes, so the caller can stop it.
    pub cancel: Option<CancelToken>,
    /// The task's work domain; `None` leaves it unset.
    pub domain: Option<roko_core::TaskDomain>,
    /// A hard cap on what the run may spend, in USD, as `roko plan run
    /// --budget-override` sets one; `None` keeps the configured ceiling.
    pub max_usd: Option<f64>,
}

/// How a prompt run through [`CliRuntime::run_prompt_plan`] ended.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptPlanResult {
    /// The run id the run ran under.
    pub run_id: String,
    /// The run's verdict: `succeeded`, `failed`, `unverified` or `cancelled`.
    pub verdict: crate::state::RunState,
    /// Whether the run succeeded: its verdict is `succeeded`.
    pub success: bool,
    /// The task's final output, when it has one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_text: Option<String>,
    /// What the run cost, when known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost_usd: Option<f64>,
}

/// Summary info for a configured repository, used to give agents
/// cross-repo context during dispatch.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoInfo {
    /// Human-readable repo name (matches the `[[repos]]` entry name).
    pub name: String,
    /// Filesystem path to the repository root.
    pub path: PathBuf,
    /// Branch tracked for this repo.
    pub branch: String,
}

/// Options for starting a SWE-bench run via the HTTP API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SweBenchRunOptions {
    /// Optional path to a local JSONL dataset.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dataset_path: Option<PathBuf>,
    /// Agent mode (e.g. "gold", "empty", "command").
    #[serde(default = "default_agent_mode")]
    pub agent_mode: String,
    /// Maximum number of instances to run.
    #[serde(default = "default_batch_size")]
    pub batch_size: usize,
    /// Offset into the dataset.
    #[serde(default)]
    pub offset: usize,
    /// Whether to record learning episodes.
    #[serde(default)]
    pub record_learning: bool,
}

fn default_agent_mode() -> String {
    "gold".to_string()
}

fn default_batch_size() -> usize {
    10
}

/// Per-instance result from a SWE-bench run.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(clippy::struct_excessive_bools)]
pub struct SweBenchInstanceResult {
    /// SWE-bench instance id.
    pub instance_id: String,
    /// Repository label.
    #[serde(default)]
    pub repo: String,
    /// Whether the patch was a valid unified diff.
    #[serde(default)]
    pub format_valid: bool,
    /// Whether `git apply --check` accepted the patch.
    #[serde(default)]
    pub apply_check: bool,
    /// Whether the test command passed.
    #[serde(default)]
    pub tests_passed: bool,
    /// Final proxy outcome.
    #[serde(default)]
    pub resolved: bool,
    /// Patch size in bytes.
    #[serde(default)]
    pub patch_bytes: usize,
    /// Wall-clock runtime in milliseconds.
    #[serde(default)]
    pub duration_ms: u64,
    /// Short failure reason.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failure_reason: Option<String>,
}

/// Result of a SWE-bench run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SweBenchRunResult {
    /// Stable run id.
    pub run_id: String,
    /// Dataset label.
    #[serde(default)]
    pub dataset: String,
    /// Agent mode used.
    #[serde(default)]
    pub agent_mode: String,
    /// Number of instances evaluated.
    pub total: usize,
    /// Number of instances resolved.
    pub resolved: usize,
    /// Pass rate (resolved / total).
    pub pass_rate: f64,
    /// Per-instance results.
    #[serde(default)]
    pub instances: Vec<SweBenchInstanceResult>,
}

/// Snapshot of session status (mirrors `roko_cli::SessionStatus` fields).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionStatusInfo {
    /// Active session identifier when a daemon-backed session exists.
    pub session_id: Option<String>,
    /// Repository working directory used to resolve local `.roko/` state.
    pub workdir: PathBuf,
    /// Whether the background daemon is currently running.
    pub daemon_running: bool,
    /// Number of known signals, if available from the runtime implementation.
    pub signal_count: Option<usize>,
    /// Number of recorded episodes, if available from the runtime implementation.
    pub episode_count: Option<usize>,
    /// Whether the latest episode passed, if the runtime can determine it.
    pub last_episode_passed: Option<bool>,
}

/// Opaque dashboard payload rendered by the CLI dashboard scaffold.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardInfo {
    /// Pre-rendered textual dashboard output.
    pub rendered: String,
}

/// Capability boundary resolved for a trigger-spawned Graph execution.
#[derive(Debug, Clone, Default)]
pub struct TriggerExecutionScope {
    /// Space in which the Flow executes, when scoped.
    pub space_id: Option<String>,
    /// Effective Graph/Space capability intersection. `None` means the legacy
    /// unscoped workspace boundary applies.
    pub capabilities: Option<roko_core::CapabilitySet>,
}

/// No-op runtime used in tests.
#[cfg(test)]
pub struct NoOpRuntime;

#[cfg(test)]
#[async_trait]
impl CliRuntime for NoOpRuntime {
    async fn run_once(
        &self,
        _workdir: &std::path::Path,
        _prompt: &str,
    ) -> anyhow::Result<RunResult> {
        Ok(RunResult {
            success: true,
            output_text: None,
            usage: None,
            gate_results: Vec::new(),
        })
    }

    /// Returns `Ok(None)` for every id so that handlers that delegate to this
    /// method produce a clean 404 rather than a 500 in unit tests.
    async fn load_plan_summary(
        &self,
        _workdir: &std::path::Path,
        _plan_id: &str,
    ) -> anyhow::Result<Option<PlanSummaryDto>> {
        Ok(None)
    }

    fn session_status(&self, workdir: PathBuf) -> SessionStatusInfo {
        SessionStatusInfo {
            session_id: None,
            workdir,
            daemon_running: false,
            signal_count: None,
            episode_count: None,
            last_episode_passed: None,
        }
    }

    fn dashboard_scaffold(&self, _workdir: &std::path::Path) -> DashboardInfo {
        DashboardInfo {
            rendered: String::new(),
        }
    }
}

/// Trait that roko-serve calls for operations that live in roko-cli.
///
/// The HTTP server holds an `Arc<dyn CliRuntime>` and delegates to it
/// whenever a handler needs to invoke the CLI's universal loop, query
/// session status, or render the dashboard scaffold.
#[async_trait]
pub trait CliRuntime: Send + Sync + 'static {
    /// Run a single prompt through the universal loop.
    async fn run_once(&self, workdir: &std::path::Path, prompt: &str) -> anyhow::Result<RunResult>;

    /// Run a single prompt with bench config overrides.
    ///
    /// The default ignores the overrides and delegates to `run_once`.
    /// The real CLI runtime applies model/backend overrides and can inspect
    /// the bench strategy hint carried in those overrides.
    async fn run_once_with_config(
        &self,
        workdir: &std::path::Path,
        prompt: &str,
        _overrides: &BenchConfigOverrides,
    ) -> anyhow::Result<RunResult> {
        self.run_once(workdir, prompt).await
    }

    /// Run `prompt` as a gated one-task plan through the Graph engine, as
    /// `roko run` does, under `options.run_id` when it is set: the route of
    /// `POST /api/run` (9113). Text generation uses [`Self::run_once`].
    ///
    /// The default bails: the runtime cannot run prompt plans.
    async fn run_prompt_plan(
        &self,
        workdir: &std::path::Path,
        prompt: &str,
        options: PromptPlanOptions,
    ) -> anyhow::Result<PromptPlanResult> {
        let _ = (workdir, prompt, options);
        anyhow::bail!("runtime does not support prompt plan runs")
    }

    /// Generate the plan `slug` from a request's text (a prompt, or a
    /// written spec) with the plan generator.
    ///
    /// Runtime implementations that know the real CLI internals should
    /// override this. The default is explicit so callers can fall back to a
    /// local synthetic plan without assuming every runtime can plan.
    async fn generate_plan_from_prompt(
        &self,
        workdir: &std::path::Path,
        slug: &str,
        prompt: &str,
    ) -> anyhow::Result<PlanGenerationResult> {
        let _ = (workdir, slug, prompt);
        anyhow::bail!("runtime does not support plan generation")
    }

    /// Execute a plan target.
    ///
    /// The default delegates to `run_once()` with a plan-execution prompt so
    /// lightweight test runtimes and remote runtimes still have a functional
    /// path. The real CLI runtime overrides this with `PlanRunner`.
    async fn run_plan(
        &self,
        workdir: &std::path::Path,
        plan_target: &std::path::Path,
    ) -> anyhow::Result<PlanExecutionResult> {
        let prompt = format!(
            "Execute the implementation plan at {} in the current workspace. \
             Run the relevant gates and include changed files plus gate results in the response.",
            plan_target.display()
        );
        let result = self.run_once(workdir, &prompt).await?;
        Ok(PlanExecutionResult {
            success: result.success,
            output_text: result.output_text,
            gate_results: Vec::new(),
        })
    }

    /// Execute a plan target with caller-supplied options.
    ///
    /// Implementations should honour `options.cancel` so that the HTTP
    /// cancel handler can interrupt a run that lives inside `spawn_blocking`
    /// or a detached task — dropping the future of `run_plan` is not enough
    /// when the executor does not observe the token.
    ///
    /// The default ignores every option and delegates to `run_plan`, keeping
    /// every existing runtime (including test stubs) compilable without changes.
    async fn run_plan_with_options(
        &self,
        workdir: &std::path::Path,
        plan_target: &std::path::Path,
        _options: PlanRunOptions,
    ) -> anyhow::Result<PlanExecutionResult> {
        self.run_plan(workdir, plan_target).await
    }

    /// The tasks of the plan at `plan_dir` whose recorded outputs a resumed
    /// run would replay instead of running, read from its checkpoint without
    /// changing it, as a server resume runs it (`force_resume`). `POST
    /// /api/plans/{id}/execute` with `{ "resume": true }` reports them before
    /// the run starts (gap-b07969).
    ///
    /// The default returns `Ok(None)`: the runtime cannot tell.
    async fn resume_skippable_tasks(
        &self,
        _workdir: &std::path::Path,
        _plan_dir: &std::path::Path,
    ) -> anyhow::Result<Option<Vec<String>>> {
        Ok(None)
    }

    /// Return the ordered list of plan ids that would be executed for
    /// `plan_target`, respecting `only_plans` when provided.
    ///
    /// Callers use this to populate `PlanHandle::members` before a run starts,
    /// so that cancel / pause / status can resolve a plan-set member id to its
    /// active run key.
    ///
    /// The default bails so that callers can detect unsupported runtimes and
    /// fall back gracefully (e.g. treat the target itself as the only member).
    async fn plan_run_order(
        &self,
        _workdir: &std::path::Path,
        _plan_target: &std::path::Path,
        _only_plans: Option<Vec<String>>,
    ) -> anyhow::Result<Vec<String>> {
        anyhow::bail!("runtime does not support plan run order")
    }

    /// Check the plans a run of `plan_target` would start, as `roko plan run`
    /// checks them before it starts any agent; `only_plans` limits a plan-set
    /// directory to the plans the run names.
    ///
    /// Returns the validation report when an error stops the run, and `None`
    /// when the run may start. The default admits every run: a runtime that
    /// cannot validate plans leaves that to the run itself.
    async fn validate_plan_run(
        &self,
        _workdir: &std::path::Path,
        _plan_target: &std::path::Path,
        _only_plans: Option<&[String]>,
    ) -> anyhow::Result<Option<PlanValidationDto>> {
        Ok(None)
    }

    /// Execute the graph attached to a trigger firing.
    ///
    /// The default preserves compatibility with runtimes that only implement
    /// plan execution. Implementations may override this method to expose the
    /// trigger payload and trace directly to a graph-native executor.
    async fn run_trigger_graph(
        &self,
        workdir: &std::path::Path,
        graph: &std::path::Path,
        _event: &roko_core::trigger::TriggerEvent,
    ) -> anyhow::Result<PlanExecutionResult> {
        self.run_plan(workdir, graph).await
    }

    /// Execute a trigger Graph with an explicitly resolved Space boundary.
    ///
    /// The default keeps lightweight runtimes compatible while concrete
    /// runtimes can propagate the effective grants into Cell execution.
    async fn run_trigger_graph_scoped(
        &self,
        workdir: &std::path::Path,
        graph: &std::path::Path,
        event: &roko_core::trigger::TriggerEvent,
        scope: &TriggerExecutionScope,
    ) -> anyhow::Result<PlanExecutionResult> {
        let _ = scope;
        self.run_trigger_graph(workdir, graph, event).await
    }

    /// Return current session status for the given workdir.
    fn session_status(&self, workdir: PathBuf) -> SessionStatusInfo;

    /// Return a dashboard scaffold rendering.
    fn dashboard_scaffold(&self, workdir: &std::path::Path) -> DashboardInfo;

    /// Return the live extension chain for a workspace when the runtime owns
    /// one. The HTTP status API uses this same chain as plan execution, so
    /// circuit-breaker state is reported rather than reconstructed.
    fn extension_chain(
        &self,
        _workdir: &std::path::Path,
    ) -> Option<std::sync::Arc<tokio::sync::Mutex<roko_core::extension::ExtensionChain>>> {
        None
    }

    /// Resolve the working directory for a repo identified by its full name
    /// (e.g. from a webhook `repository.full_name`). Returns `None` when the
    /// repo is not configured.
    fn resolve_repo_workdir(&self, repo_full_name: &str) -> Option<PathBuf> {
        let _ = repo_full_name;
        None
    }

    /// Return the merged `RokoConfig` for a named repo, applying per-repo
    /// overrides on top of the global config. Returns `None` when the repo
    /// is not configured.
    fn repo_roko_config(&self, _repo_name: &str) -> Option<roko_core::config::schema::RokoConfig> {
        None
    }

    /// Return a list of all configured repositories. Used to inject
    /// cross-repo context into agent system prompts during dispatch.
    fn list_repos(&self) -> Vec<RepoInfo> {
        Vec::new()
    }

    /// List all plans available in the given working directory.
    ///
    /// Runtime implementations that know the real CLI internals should
    /// override this. The default is explicit so callers can detect
    /// unsupported runtimes without assuming plan discovery is available.
    async fn list_plans(&self, workdir: &std::path::Path) -> anyhow::Result<Vec<PlanSummaryDto>> {
        let _ = workdir;
        anyhow::bail!("runtime does not support plan discovery")
    }

    /// Load a summary for a single plan identified by `plan_id`.
    ///
    /// Returns `Ok(None)` when the plan does not exist. The default returns
    /// an explicit error so callers can detect unsupported runtimes.
    async fn load_plan_summary(
        &self,
        workdir: &std::path::Path,
        plan_id: &str,
    ) -> anyhow::Result<Option<PlanSummaryDto>> {
        let _ = (workdir, plan_id);
        anyhow::bail!("runtime does not support plan discovery")
    }

    /// Load the task list for a single plan identified by `plan_id`.
    ///
    /// Returns `Ok(None)` when the plan does not exist. The default returns
    /// an explicit error so callers can detect unsupported runtimes.
    async fn load_plan_tasks(
        &self,
        workdir: &std::path::Path,
        plan_id: &str,
    ) -> anyhow::Result<Option<PlanTasksDto>> {
        let _ = (workdir, plan_id);
        anyhow::bail!("runtime does not support plan discovery")
    }

    /// Return the raw TOML source of a plan's `tasks.toml`.
    ///
    /// Returns `Ok(None)` when no plan with `plan_id` is found in `workdir`.
    async fn plan_source(
        &self,
        workdir: &std::path::Path,
        plan_id: &str,
    ) -> anyhow::Result<Option<PlanSourceDto>> {
        let _ = (workdir, plan_id);
        anyhow::bail!("runtime does not support plan source access")
    }

    /// Validate a plan source text.
    ///
    /// - `toml = None` validates the plan file currently on disk.
    /// - `toml = Some(text)` validates that text without writing anything.
    ///
    /// Returns `Ok(None)` when no plan with `plan_id` is found in `workdir`.
    async fn validate_plan_source(
        &self,
        workdir: &std::path::Path,
        plan_id: &str,
        toml: Option<String>,
    ) -> anyhow::Result<Option<PlanValidationDto>> {
        let _ = (workdir, plan_id, toml);
        anyhow::bail!("runtime does not support plan source validation")
    }

    /// Validate and save a plan source text.
    ///
    /// Saves only when validation succeeds; on failure the file on disk is
    /// left completely untouched and the returned report contains the errors.
    ///
    /// Returns `Ok(None)` when no plan with `plan_id` is found in `workdir`.
    async fn save_plan_source(
        &self,
        workdir: &std::path::Path,
        plan_id: &str,
        toml: String,
    ) -> anyhow::Result<Option<PlanValidationDto>> {
        let _ = (workdir, plan_id, toml);
        anyhow::bail!("runtime does not support plan source saving")
    }

    /// Revise a plan's `tasks.toml` using an LLM and caller-supplied feedback.
    ///
    /// The runtime calls the plan-authoring agent with the current source and
    /// `feedback`, validates the result, and writes it only when it passes.
    ///
    /// Returns `Ok(None)` when no plan with `plan_id` is found in `workdir`.
    async fn revise_plan(
        &self,
        workdir: &std::path::Path,
        plan_id: &str,
        feedback: &str,
    ) -> anyhow::Result<Option<RevisionDto>> {
        let _ = (workdir, plan_id, feedback);
        anyhow::bail!("runtime does not support plan revision")
    }

    /// Create a new plan with the given slug and title.
    ///
    /// Writes `<plans_dir>/<slug>/tasks.toml` from a generated starter source
    /// and a sibling `plan.md` carrying the title.  The starter source is
    /// validated before any file is written; a failed validation returns
    /// [`CreatePlanOutcome::Rejected`] rather than an error.
    async fn create_plan(
        &self,
        workdir: &std::path::Path,
        slug: &str,
        title: &str,
    ) -> anyhow::Result<CreatePlanOutcome> {
        let _ = (workdir, slug, title);
        anyhow::bail!("runtime does not support plan creation")
    }

    /// Run a SWE-bench evaluation. Returns per-instance results.
    ///
    /// The default returns an error since not all runtimes support SWE-bench.
    async fn run_swe_bench(
        &self,
        _workdir: &std::path::Path,
        _options: SweBenchRunOptions,
    ) -> anyhow::Result<SweBenchRunResult> {
        anyhow::bail!("runtime does not support SWE-bench")
    }
}
