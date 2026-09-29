//! Single-prompt execution: the body of `roko run <prompt>` and of the
//! single-agent `roko do` routes.
//!
//! [`run_prompt`] writes the prompt as a one-task plan under
//! `.roko/runs/<run_id>/` and executes it through the Graph engine, so a
//! prompt gets the same dispatch, failover, safety, budget, gates,
//! checkpoints, and learning feedback as `roko plan run`.

use crate::config::{Config, GateConfig};
use crate::model_selection::{EffectiveModelSelection, SelectionSource, resolve_effective_model};
use crate::output_format;
use crate::state_hub::{SharedStateHub, StateHub};
use crate::task_parser::{TaskDef, TaskMeta, TasksFile, VerifyStep};
use anyhow::{Context as _, Result, anyhow, bail};
use chrono::Utc;
use roko_agent::provider::is_known_protocol_command;
use roko_core::DashboardSnapshot;
use roko_core::agent::resolve_model;
use roko_core::config::schema::RokoConfig;
use roko_learn::episode_logger::{Episode, EpisodeLogger};
use roko_learn::playbook::Playbook;
use roko_runtime::workflow_contract::{GateOutcome, WorkflowRunReport};
use roko_serve::bench::BenchStrategy;
use std::path::{Path, PathBuf};

/// Summary of a single `run` invocation.
#[derive(Debug, Clone, serde::Serialize)]
pub struct RunReport {
    /// Content hash of the episode signal emitted at the end.
    pub episode_id: String,
    /// Content hash of the assembled prompt signal.
    pub prompt_id: String,
    /// Content hash of the agent's output signal.
    pub agent_output_id: String,
    /// Whether the agent invocation succeeded (exit code 0, no timeout).
    pub agent_success: bool,
    /// Per-gate verdicts in declaration order: (gate name, passed).
    pub gate_verdicts: Vec<(String, bool)>,
    /// How many signals are now in the substrate.
    pub total_signals: usize,
    /// Final agent output text, if it was a text payload.
    pub output_text: Option<String>,
    /// Token usage reported by the agent dispatch, when available.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub usage: Option<RunUsage>,
}

/// Token usage captured from a single run.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct RunUsage {
    /// Input (prompt) tokens consumed.
    pub input_tokens: u64,
    /// Output (completion) tokens produced.
    pub output_tokens: u64,
}

impl RunReport {
    /// True if the agent succeeded and every configured gate passed.
    #[must_use]
    pub fn overall_success(&self) -> bool {
        self.agent_success && self.gate_verdicts.iter().all(|(_, ok)| *ok)
    }
}

pub fn write_shared_workflow_run(
    workdir: &std::path::Path,
    prompt: &str,
    agent: &str,
    role: &str,
    report: &WorkflowRunReport,
) -> anyhow::Result<String> {
    // Scrub secrets from user-visible text fields before persisting the transcript.
    let scrubbed_prompt = crate::share::scrub_share_text(prompt);
    let scrubbed_output = crate::share::scrub_share_text(&report.output);

    let token = roko_core::generate_share_token();
    let (report_agent, report_role) = workflow_report_agent_role(report);
    let transcript = roko_serve::routes::shared_runs::RunTranscript {
        id: token.clone(),
        agent: non_empty(agent)
            .map(ToOwned::to_owned)
            .or(report_agent)
            .unwrap_or_else(|| "workflow".to_string()),
        role: non_empty(role)
            .map(ToOwned::to_owned)
            .or(report_role)
            .unwrap_or_else(|| "workflow".to_string()),
        prompt: scrubbed_prompt,
        success: report.success,
        gates: report
            .gates
            .iter()
            .map(|gate| (gate.name.clone(), gate.passed))
            .collect(),
        output: non_empty(&scrubbed_output).map(ToOwned::to_owned),
        cost_usd: report.cost,
        // GAP: WorkflowRunReport exposes only a combined `token_usage: u64` total; the
        // workflow engine does not track input vs. output token counts separately. To
        // populate these fields the engine would need to accumulate per-turn TokenUsage
        // breakdowns and surface them on WorkflowRunReport.
        input_tokens: None,
        output_tokens: None,
        model: non_empty(&report.model).map(ToOwned::to_owned),
        duration_s: Some(report.duration_secs),
        episode_id: Some(report.run_id.clone()),
        transcript: report.events.clone(),
        timestamp: report
            .events
            .first()
            .map(|event| event.ts.to_rfc3339())
            .unwrap_or_else(|| chrono::Utc::now().to_rfc3339()),
    };
    write_shared_transcript(workdir, &transcript)
}

fn write_shared_transcript(
    workdir: &std::path::Path,
    transcript: &roko_serve::routes::shared_runs::RunTranscript,
) -> anyhow::Result<String> {
    let token = transcript.id.clone();
    let dir = workdir.join(".roko").join("shared");
    std::fs::create_dir_all(&dir)?;
    std::fs::write(
        dir.join(format!("{token}.json")),
        serde_json::to_string_pretty(&transcript)?,
    )?;

    output_format::divider();
    output_format::step("Shared", "");
    output_format::bar(&output_format::cyan(&format!(
        "http://localhost:6677/runs/{token}"
    )));
    output_format::note("run with --serve to make the URL accessible");

    Ok(token)
}

fn truncate(text: &str, max_chars: usize) -> &str {
    text.char_indices()
        .nth(max_chars)
        .map_or(text, |(idx, _)| &text[..idx])
}

fn non_empty(text: &str) -> Option<&str> {
    let trimmed = text.trim();
    (!trimmed.is_empty()).then_some(trimmed)
}

fn workflow_report_agent_role(report: &WorkflowRunReport) -> (Option<String>, Option<String>) {
    let mut first = None;
    for envelope in &report.events {
        if let roko_core::RuntimeEvent::AgentSpawned { agent_id, role, .. } = &envelope.payload {
            let values = (Some(agent_id.clone()), Some(role.clone()));
            if role == "implementer" {
                return values;
            }
            first.get_or_insert(values);
        }
    }
    first.unwrap_or((None, None))
}

pub fn workflow_report_outcome(report: &WorkflowRunReport) -> Option<roko_core::WorkflowOutcome> {
    report
        .events
        .iter()
        .rev()
        .find_map(|envelope| match &envelope.payload {
            roko_core::RuntimeEvent::WorkflowCompleted { outcome, .. } => Some(outcome.clone()),
            _ => None,
        })
}

/// Format a duration for human display: "3.2s", "1m 42s", "0.8s".
fn format_duration(d: std::time::Duration) -> String {
    let secs = d.as_secs_f64();
    if secs < 60.0 {
        format!("{secs:.1}s")
    } else {
        let mins = secs as u64 / 60;
        let remaining = secs as u64 % 60;
        format!("{mins}m {remaining}s")
    }
}

// WorkflowExecutionRoute removed by #276: all callers use graph templates.
// resolve_engine_route is retained as a no-op adapter for the --engine CLI
// flag (#258). The only accepted value is "graph" (already the default).

/// Resolve the CLI `--engine` flag. After #276, graph is the only path.
///
/// Unknown values log a warning but do not change behavior. This function
/// exists so the `--engine` flag continues to parse without error.
#[must_use]
pub fn resolve_engine_flag(engine: Option<&str>) -> &'static str {
    match engine {
        None | Some("graph") | Some("graph_canary") => "graph",
        Some(other) => {
            tracing::warn!(
                engine = other,
                "unknown --engine value; graph is the only available engine after #276"
            );
            "graph"
        }
    }
}

/// CLI overrides parsed from clap args, threaded through the call chain
/// instead of re-parsing `std::env::args_os()`.
#[derive(Debug, Default, Clone)]
pub struct CliOverrides {
    pub model: Option<String>,
    pub role: Option<String>,
    pub provider: Option<String>,
    pub cascade_enabled: Option<bool>,
    pub effort: Option<String>,
}

pub(crate) fn resolve_workflow_model_selection(
    workdir: &std::path::Path,
    overrides: &CliOverrides,
) -> anyhow::Result<(Config, RokoConfig, EffectiveModelSelection)> {
    let resolved = crate::config::load_resolved_config(workdir)
        .with_context(|| format!("load config for workflow engine in {}", workdir.display()))?;
    let mut config = resolved.config;
    ensure_workflow_agent_configured(&config, resolved.sources.agent_command, overrides)?;

    if let Some(ref model) = overrides.model {
        config.agent.model = Some(model.clone());
    }
    if let Some(ref role) = overrides.role {
        config.prompt.role.clone_from(role);
    }

    let mut model_config =
        roko_core::config::loader::load_config_unified(workdir).unwrap_or_default();
    model_config.providers.extend(config.providers.clone());
    model_config.models.extend(config.models.clone());
    model_config.agent.command = Some(config.agent.command.clone());
    model_config.agent.args = Some(config.agent.args.clone());
    model_config.agent.timeout_ms = Some(config.agent.timeout_ms);
    model_config.agent.env = Some(config.agent.env.clone());
    model_config.agent.default_effort = config.agent.effort.clone();
    model_config.agent.bare_mode = config.agent.bare_mode;
    model_config.agent.fallback_model = config.agent.fallback_model.clone();
    model_config.agent.tier_models = config.agent.tier_models.clone();
    if let Some(model) = config.agent.model.clone() {
        model_config.agent.default_model = model;
    }

    if let Some(selection) =
        legacy_command_workflow_selection(&mut config, &mut model_config, overrides)
    {
        return Ok((config, model_config, selection));
    }

    let role = non_empty(&config.prompt.role).map(str::to_owned);
    let selection = resolve_effective_model(
        overrides.model.clone(),
        None,
        role,
        None,
        &model_config,
        overrides.provider.clone(),
    )
    .map_err(|error| anyhow!("resolve workflow model selection: {error}"))?;

    // Apply the resolved model back to config so downstream code sees it.
    config.agent.model = Some(selection.effective_model_key.clone());

    Ok((config, model_config, selection))
}

fn legacy_command_workflow_selection(
    config: &mut Config,
    model_config: &mut RokoConfig,
    overrides: &CliOverrides,
) -> Option<EffectiveModelSelection> {
    if overrides.provider.is_some()
        || !model_config.providers.is_empty()
        || !model_config.models.is_empty()
    {
        return None;
    }

    let command = config.agent.command.trim();
    if command.is_empty() || command == "false" {
        return None;
    }
    if is_known_protocol_command(command) {
        return None;
    }

    let model = overrides
        .model
        .clone()
        .or_else(|| config.agent.model.clone())
        .filter(|model| !model.trim().is_empty())
        .unwrap_or_else(|| command.to_string());

    model_config.agent.default_model.clone_from(&model);
    config.agent.model = Some(model.clone());

    let resolved = resolve_model(model_config, &model);
    Some(EffectiveModelSelection {
        requested_model: Some(model.clone()),
        effective_model_key: resolved.model_key,
        provider_key: format!("exec:{command}"),
        provider_kind: "exec".to_string(),
        backend_slug: resolved.slug,
        source: SelectionSource::ProjectDefault,
        reason: format!(
            "project agent command `{command}` selected generic subprocess model `{model}`"
        ),
    })
}

fn ensure_workflow_agent_configured(
    config: &Config,
    agent_command_source: crate::config::Source,
    overrides: &CliOverrides,
) -> anyhow::Result<()> {
    let has_model_override =
        config.agent.model.is_some() || overrides.model.is_some() || overrides.provider.is_some();
    if agent_command_source == crate::config::Source::Default
        && config.agent.command == "cat"
        && !has_model_override
    {
        return Err(anyhow!(
            "roko run refused to run with the default `cat` agent. Run `roko init`, configure a provider in roko.toml, or pass a model/provider override."
        ));
    }
    Ok(())
}

pub fn workflow_enabled_gate_names(gates: &[GateConfig]) -> Vec<String> {
    gates
        .iter()
        .map(|gate| match gate {
            GateConfig::Compile { .. } => "compile".to_string(),
            GateConfig::Clippy { .. } => "clippy".to_string(),
            GateConfig::Test { .. } => "test".to_string(),
            GateConfig::Shell { .. } => "shell".to_string(),
        })
        .collect()
}

/// Budget admission for `roko run`, checked before anything is dispatched.
///
/// Two guards mirror the plan runner so `roko run` respects the same spend
/// ceilings that `roko plan run` enforces:
///
/// 1. Plan ceiling as a daily guard (`max_plan_usd`): read today's total
///    from the costs JSONL log; reject the dispatch if today's accumulated
///    spend already meets or exceeds the plan ceiling.
/// 2. Turn ceiling (`max_turn_usd`): load the learned `BudgetPredictor` and
///    compare the predicted token cost against the per-turn USD cap. The
///    predictor provides a best-effort estimate; if no history is available
///    the fallback token count is used. A conservative average price of
///    $15 / million tokens is applied (sonnet-class output side).
///
/// A cap of `0.0` means no cap, as in the core `[budget]` section, and skips
/// its guard. Both checks are soft-fail on I/O errors (best-effort).
///
/// # Errors
///
/// Fails when today's spend has reached `max_plan_usd`, or when the
/// predicted turn cost exceeds `max_turn_usd`.
pub async fn check_budget_admission(workdir: &Path, config: &Config) -> Result<()> {
    let learn_dir = workdir.join(".roko").join("learn");
    let budget = &config.budget;

    // Guard 1: plan ceiling as a daily spend guard.
    let max_plan = budget.max_plan_usd;
    if max_plan > 0.0 {
        let costs_path = learn_dir.join("costs.jsonl");
        let costs_log = roko_learn::costs_log::CostsLog::at(&costs_path);
        match costs_log.cost_today().await {
            Ok(today_usd) if today_usd >= max_plan => {
                bail!(
                    "daily budget exhausted: spent ${today_usd:.4} of ${max_plan:.2} today \
                     (max_plan_usd = {max_plan}). \
                     Increase [budget].max_plan_usd in roko.toml or wait until tomorrow."
                );
            }
            Ok(today_usd) => {
                tracing::debug!(
                    today_usd,
                    max_plan_usd = max_plan,
                    "daily budget admission: ok"
                );
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                tracing::debug!("costs log not found; skipping daily budget check");
            }
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    "could not read costs log for daily budget check; proceeding"
                );
            }
        }
    }

    // Guard 2: per-turn ceiling via BudgetPredictor.
    let max_turn = budget.max_turn_usd;
    if max_turn > 0.0 {
        // Load the predictor. When no budget-predictor.json exists yet,
        // calibrate from efficiency.jsonl so historical cost data is used
        // even on a fresh workspace (P2-LRN-2).
        let predictor = match roko_compose::budget_predictor::load_or_calibrate(&learn_dir) {
            Ok(p) => p,
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    "could not load or calibrate budget predictor; using defaults"
                );
                roko_compose::BudgetPredictor::new()
            }
        };

        // Derive task features from config: role, complexity, domain.
        let role = if config.prompt.role.trim().is_empty() {
            "workflow".to_string()
        } else {
            config.prompt.role.trim().to_string()
        };
        let features = roko_compose::TaskFeatures::new(role, "standard", "code");
        let predicted_tokens = predictor.predict(&features);

        // Conservative price: $15 / million tokens (sonnet output tier).
        // This errs on the side of caution so the cap is enforced before
        // committing to a potentially over-budget dispatch.
        const USD_PER_TOKEN: f64 = 15.0 / 1_000_000.0;
        #[allow(clippy::cast_precision_loss)]
        let predicted_usd = predicted_tokens as f64 * USD_PER_TOKEN;

        if predicted_usd > max_turn {
            bail!(
                "predicted turn cost ${predicted_usd:.4} exceeds max_turn_usd ${max_turn:.4} \
                 (estimated {predicted_tokens} tokens at $15/MTok). \
                 Increase [budget].max_turn_usd in roko.toml or use a simpler prompt."
            );
        }

        tracing::debug!(
            predicted_tokens,
            predicted_usd,
            max_turn_usd = max_turn,
            "turn budget admission: ok"
        );
    }

    Ok(())
}

/// A prompt to execute through the Graph engine (see [`run_prompt`]).
pub struct PromptRun<'a> {
    /// The prompt; it becomes the task description verbatim.
    pub prompt: &'a str,
    /// Workspace root.
    pub workdir: &'a Path,
    /// Task tier: `mechanical`, `focused`, `integrative`, or `architectural`.
    pub tier: &'a str,
    /// `--model`, `--provider`, and `--role` overrides.
    pub overrides: &'a CliOverrides,
    /// Retries after a failed attempt; `None` keeps the plan default.
    pub max_retries: Option<u32>,
    /// Suppress the Graph engine's inline progress and summary line.
    pub quiet: bool,
    /// Hub that receives the run's dashboard events (`roko run --serve`
    /// passes the server's); `None` uses a private hub.
    pub state_hub: Option<SharedStateHub>,
}

/// Execute one prompt through the Graph engine.
///
/// The prompt becomes a one-task plan in `.roko/runs/<run_id>/` (never under
/// `plans/`): an `implementer` task whose verify steps are the workspace
/// gates (see [`prompt_verify_steps`]). [`run_graph_plan`] executes it with
/// the provider dispatch, failover, safety contracts, budget, checkpoints,
/// and per-task episodes, efficiency, and cost records of `roko plan run`.
/// The run then settles one `workflow_complete` episode carrying the gate
/// verdicts.
///
/// [`run_graph_plan`]: crate::graph_execution::run_graph_plan
///
/// # Errors
///
/// Fails before any provider call when no agent is configured, the role
/// override is not a plan task role, or no gate can verify the change; and
/// when the Graph engine cannot start the run.
pub async fn run_prompt(run: PromptRun<'_>) -> Result<WorkflowRunReport> {
    use crate::graph_execution::plan_runner::{
        PlanRunInterruptHandle, install_plan_run_signal_handlers,
    };
    use crate::graph_execution::{GraphPlanRunParams, run_graph_plan};

    let (_config, model_config, selection) =
        resolve_workflow_model_selection(run.workdir, run.overrides)?;
    if !run.quiet {
        selection.print_stderr();
    }
    let role = match run.overrides.role.as_deref() {
        None => "implementer",
        Some(role) if crate::task_parser::PLAN_TASK_ROLES.contains(&role) => role,
        Some(role) => bail!(
            "unknown role `{role}` for roko run (valid: {})",
            crate::task_parser::PLAN_TASK_ROLES.join(", ")
        ),
    };
    let verify = prompt_verify_steps(run.workdir, &model_config.gates);
    if verify.is_empty() {
        bail!(
            "no gate can verify this change: declare the project's build or test command \
             in roko.toml as a `[[gates.rungs]]` entry (`name`, `command`)"
        );
    }

    let layout = roko_fs::RokoLayout::for_project(run.workdir);
    let run_id = format!("run-{}", Utc::now().format("%Y%m%d-%H%M%S-%3f"));
    let run_dir = layout.run_dir(&run_id);
    std::fs::create_dir_all(&run_dir)
        .with_context(|| format!("create run directory {}", run_dir.display()))?;
    prompt_tasks_file(&run_id, run.prompt, run.tier, role, verify, run.workdir)
        .write(&run_dir.join("tasks.toml"))?;

    // A model or provider override pins the task's model; otherwise the
    // Graph engine routes by tier, as for authored plans.
    let cli_model_override = run.overrides.model.clone().or_else(|| {
        run.overrides
            .provider
            .as_ref()
            .map(|_| selection.effective_model_key.clone())
    });
    let episodes_path = layout.root_episodes_path();
    let episodes_offset = std::fs::metadata(&episodes_path).map_or(0, |meta| meta.len());
    let hub = run
        .state_hub
        .unwrap_or_else(crate::state_hub::shared_state_hub);

    // SIGINT/SIGTERM stop the run gracefully for as long as the guard lives.
    let interrupt = PlanRunInterruptHandle::default();
    let _signals = install_plan_run_signal_handlers(interrupt.clone())?;
    let started = std::time::Instant::now();
    let exit_code = run_graph_plan(GraphPlanRunParams {
        plans_dir: run_dir.clone(),
        workdir: run.workdir.to_path_buf(),
        quiet: run.quiet,
        json: false,
        resume_plan: None,
        fresh: false,
        force_resume: false,
        max_retries: run.max_retries,
        max_tasks: 0,
        budget_override: None,
        no_budget: false,
        cli_model_override,
        dangerously_skip_permissions: false,
        log_file: None,
        worktree_per_task: false,
        rich_topology: false,
        no_tui: true,
        state_hub: Some(hub.clone()),
        interrupt: Some(interrupt),
        max_parallel_plans: None,
        fail_fast: false,
        only_plans: None,
        live_agent_output: crate::graph_task_dispatch::LiveAgentOutput::ToolSteps,
    })
    .await?;
    let duration = started.elapsed();

    let snapshot = hub.current_snapshot();
    let episodes = task_episodes_since(&episodes_path, episodes_offset, &run_id);
    let success = exit_code == crate::exit_codes::EXIT_SUCCESS;
    let last = episodes.last();
    let report = WorkflowRunReport {
        run_id: run_id.clone(),
        success,
        model: last.map_or_else(
            || selection.effective_model_key.clone(),
            |e| e.model.clone(),
        ),
        provider: last
            .map(|episode| episode.backend.clone())
            .filter(|backend| !backend.is_empty())
            .or(Some(selection.provider_key)),
        prompt_summary: truncate(run.prompt, 120).to_string(),
        output: task_output(&snapshot, &run_id),
        agent_turns: u32::try_from(episodes.iter().map(|e| e.turns).sum::<u64>())
            .unwrap_or(u32::MAX),
        token_usage: episodes
            .iter()
            .map(|e| e.usage.input_tokens + e.usage.output_tokens)
            .sum(),
        input_tokens: episodes.iter().map(|e| e.usage.input_tokens).sum(),
        output_tokens: episodes.iter().map(|e| e.usage.output_tokens).sum(),
        cache_read_tokens: episodes.iter().map(|e| e.usage.cache_read_tokens).sum(),
        cost: Some(episodes.iter().map(|e| e.usage.cost_usd).sum()),
        duration_secs: duration.as_secs_f64(),
        gates: gate_outcomes(&snapshot, &run_id),
        events: Vec::new(),
        checkpoint_path: None,
    };
    let outcome = if success {
        "success".to_string()
    } else {
        episodes
            .iter()
            .rev()
            .find_map(|episode| episode.failure_reason.clone())
            .unwrap_or_else(|| format!("Graph engine exited with code {exit_code}"))
    };
    record_workflow_feedback(layout.root(), &report, outcome, duration).await;
    Ok(report)
}

/// Verify steps for a prompt run: the workspace's declared gate rungs
/// (`[[gates.rungs]]`, which legacy `[[gate]]` entries migrate into), else
/// the compile check of a Cargo or Go workspace. Empty when neither exists.
fn prompt_verify_steps(workdir: &Path, gates: &roko_core::config::GatesConfig) -> Vec<VerifyStep> {
    if gates.has_custom_rungs() {
        return gates
            .effective_rungs()
            .into_iter()
            .filter(|rung| rung.required && !rung.command.trim().is_empty())
            .map(|rung| VerifyStep {
                phase: rung.name,
                command: rung.command,
                fail_msg: None,
                timeout_ms: rung.timeout_secs.saturating_mul(1_000),
            })
            .collect();
    }
    let compile = if workdir.join("Cargo.toml").is_file() {
        "cargo check --workspace"
    } else if workdir.join("go.mod").is_file() {
        "go build ./..."
    } else {
        return Vec::new();
    };
    vec![VerifyStep {
        phase: "compile".to_string(),
        command: compile.to_string(),
        fail_msg: None,
        timeout_ms: roko_core::config::TimeoutConfig::default()
            .gate_test()
            .as_secs()
            .saturating_mul(1_000),
    }]
}

/// The one-task plan that runs `prompt`.
fn prompt_tasks_file(
    run_id: &str,
    prompt: &str,
    tier: &str,
    role: &str,
    verify: Vec<VerifyStep>,
    workdir: &Path,
) -> TasksFile {
    let title = prompt.lines().next().unwrap_or(prompt).trim();
    TasksFile {
        meta: TaskMeta {
            plan: run_id.to_string(),
            iteration: 1,
            total: 1,
            done: 0,
            status: "ready".to_string(),
            superseded_by: None,
            max_parallel: 1,
            estimated_total_minutes: 0,
            // The prompt is the whole task definition.
            skip_enrichment: true,
            source_prd: None,
            failure_policy: None,
        },
        tasks: vec![TaskDef {
            id: "T1".to_string(),
            title: truncate(title, 80).to_string(),
            description: Some(prompt.to_string()),
            role: Some(role.to_string()),
            status: "ready".to_string(),
            tier: tier.to_string(),
            frequency: None,
            model_hint: None,
            replan_strategy: None,
            max_loc: None,
            files: workspace_scope(workdir),
            allowed_tools: None,
            denied_tools: None,
            mcp_servers: None,
            depends_on: Vec::new(),
            depends_on_plan: Vec::new(),
            split_into: None,
            context: None,
            verify,
            timeout_secs: 0,
            max_retries: crate::task_parser::default_max_retries(),
            acceptance: Vec::new(),
            acceptance_contract: None,
            domain: None,
            estimated_minutes: None,
            crates_touched: None,
            sequence: 0,
        }],
    }
}

/// A free-form prompt may touch any file, so its task's scope is the
/// workspace's top-level entries (the plan contract requires a non-empty,
/// glob-free list of repository-relative paths).
fn workspace_scope(workdir: &Path) -> Vec<String> {
    let mut entries: Vec<String> = std::fs::read_dir(workdir)
        .into_iter()
        .flatten()
        .filter_map(|entry| entry.ok()?.file_name().into_string().ok())
        .filter(|name| {
            !name.starts_with('.')
                && !matches!(name.as_str(), "target" | "node_modules")
                && !name.contains(['*', '?', '[', ']', '\\'])
        })
        .collect();
    entries.sort();
    entries.truncate(crate::plan_policy::PlanExecutionPolicy::for_environment().max_files_per_task);
    if entries.is_empty() {
        entries.push(".roko".to_string());
    }
    entries
}

/// Episodes for `plan_id`'s tasks appended to the log at `path` after byte
/// `offset` (the whole log when it rotated meanwhile).
fn task_episodes_since(path: &Path, offset: u64, plan_id: &str) -> Vec<Episode> {
    use std::io::{Read as _, Seek as _, SeekFrom};

    let mut appended = Vec::new();
    if let Ok(mut file) = std::fs::File::open(path) {
        let len = file.metadata().map_or(0, |meta| meta.len());
        let start = if offset <= len { offset } else { 0 };
        if file.seek(SeekFrom::Start(start)).is_ok() {
            let _ = file.read_to_end(&mut appended);
        }
    }
    String::from_utf8_lossy(&appended)
        .lines()
        .filter_map(|line| serde_json::from_str::<Episode>(line).ok())
        .filter(|episode| {
            episode
                .extra
                .get("plan_id")
                .and_then(serde_json::Value::as_str)
                == Some(plan_id)
        })
        .collect()
}

/// Final verdict of each verify step of `plan_id`, in the order they ran.
fn gate_outcomes(snapshot: &DashboardSnapshot, plan_id: &str) -> Vec<GateOutcome> {
    let mut gates: Vec<GateOutcome> = Vec::new();
    for verdict in snapshot.gates.iter().filter(|v| v.plan_id == plan_id) {
        // A retried step reports again; its last verdict stands.
        match gates.iter_mut().find(|gate| gate.name == verdict.gate) {
            Some(gate) => gate.passed = verdict.passed,
            None => gates.push(GateOutcome {
                name: verdict.gate.clone(),
                passed: verdict.passed,
                output: None,
                duration_ms: 0,
            }),
        }
    }
    gates
}

/// The agent text the dashboard retained for `plan_id`'s tasks (stream
/// records other than text, such as tool calls, are left out).
fn task_output(snapshot: &DashboardSnapshot, plan_id: &str) -> String {
    let mut tasks: Vec<&str> = snapshot
        .agents
        .values()
        .filter(|agent| agent.current_plan == plan_id)
        .map(|agent| agent.current_task.as_str())
        .collect();
    tasks.sort_unstable();
    tasks.dedup();
    tasks
        .iter()
        .filter_map(|task| snapshot.task_outputs.get(*task))
        .flatten()
        .filter_map(|line| {
            let Some(record) = line.strip_prefix(crate::runner::tui_bridge::STREAM_RECORD_PREFIX)
            else {
                return Some(line.clone());
            };
            let record = serde_json::from_str::<serde_json::Value>(record).ok()?;
            (record["kind"] == "text")
                .then(|| record["payload"]["text"].as_str().map(str::to_owned))
                .flatten()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Record the run's gate verdicts and completion through the learning
/// feedback service, which appends them to `learn/efficiency.jsonl` and one
/// `workflow_complete` episode to the root episode log.
async fn record_workflow_feedback(
    roko_dir: &Path,
    report: &WorkflowRunReport,
    outcome: String,
    duration: std::time::Duration,
) {
    use roko_core::foundation::{FeedbackEvent, FeedbackSink as _};

    let feedback =
        roko_learn::feedback_service::FeedbackService::from_roko_dir_with_episodes(roko_dir);
    let gate_events = report.gates.iter().map(|gate| FeedbackEvent::GateResult {
        run_id: report.run_id.clone(),
        gate_name: gate.name.clone(),
        passed: gate.passed,
        duration_ms: gate.duration_ms,
    });
    let completion = FeedbackEvent::WorkflowComplete {
        event_type: "workflow_completed".to_string(),
        run_id: report.run_id.clone(),
        model: Some(report.model.clone()),
        success: report.success,
        outcome,
        total_cost_usd: report.cost.unwrap_or_default(),
        total_tokens: report.token_usage,
        duration_ms: u64::try_from(duration.as_millis()).unwrap_or(u64::MAX),
    };
    for event in gate_events.chain(std::iter::once(completion)) {
        if let Err(error) = feedback.record(event).await {
            tracing::warn!(%error, run_id = %report.run_id, "workflow feedback record failed");
        }
    }
    if let Err(error) = feedback.flush_async().await {
        tracing::warn!(%error, run_id = %report.run_id, "workflow feedback flush failed");
    }
}

/// Print a [`run_prompt`] result: what ran, the outcome, cost, gate
/// verdicts, and where the run's plan and records are.
pub fn print_workflow_run_report(prompt: &str, tier: &str, report: &WorkflowRunReport) {
    output_format::intro("roko run");
    output_format::step("prompt", &output_format::dim(&truncate(prompt, 60)));
    output_format::step("engine", &format!("graph, one {tier} task"));
    output_format::step("model", &report.model);
    output_format::divider();

    if report.success {
        output_format::success(&format!(
            "workflow completed ({} agent turn{})",
            report.agent_turns,
            if report.agent_turns == 1 { "" } else { "s" },
        ));
    } else {
        output_format::error("workflow failed");
    }

    if !report.output.trim().is_empty() {
        output_format::bar(&truncate(&report.output, 200));
    }

    output_format::divider();
    output_format::step("Summary", "");
    output_format::branch(&format!(
        "duration   {}",
        output_format::cyan(&format_duration(std::time::Duration::from_secs_f64(
            report.duration_secs,
        ))),
    ));
    output_format::branch(&format!(
        "tokens     {}",
        output_format::cyan(&report.token_usage.to_string()),
    ));
    if let Some(cost) = report.cost {
        output_format::branch(&format!(
            "cost       {}",
            output_format::cyan(&format!("{:.4}", cost.max(0.0)))
        ));
    }
    if report.gates.is_empty() {
        output_format::branch("gates      (none ran)");
    } else {
        for gate in &report.gates {
            let marker = if gate.passed { "PASS" } else { "FAIL" };
            output_format::branch(&format!("gate       [{marker}] {}", gate.name));
        }
    }
    output_format::branch(&format!(
        "plan       {}",
        output_format::dim(&format!(".roko/runs/{}/tasks.toml", report.run_id)),
    ));
    output_format::branch(&format!(
        "episodes   {}",
        output_format::dim(".roko/episodes.jsonl"),
    ));
    output_format::end(&output_format::dim(&report.run_id));
}

/// Single-prompt execution via the `ModelCallService` path.
///
/// Uses `dispatch_bench_prompt()` infrastructure from `serve_runtime.rs`,
/// wrapping the result into a [`RunReport`] for backward compatibility with
/// existing callers (`demo_cmd`, `worker`, `run_inline`, `commands/job`).
pub async fn run_once(
    workdir: &Path,
    config: &Config,
    prompt_text: &str,
    _strategy: Option<BenchStrategy>,
    _external_hub: Option<&StateHub>,
) -> Result<RunReport> {
    let result =
        crate::serve_runtime::dispatch_bench_prompt(workdir, config, prompt_text, None).await?;

    let content_hash = roko_core::ContentHash::of(result.text.as_bytes());
    let prompt_hash = roko_core::ContentHash::of(prompt_text.as_bytes());

    Ok(RunReport {
        episode_id: content_hash.to_hex(),
        prompt_id: prompt_hash.to_hex(),
        agent_output_id: content_hash.to_hex(),
        agent_success: true,
        gate_verdicts: Vec::new(),
        total_signals: 0,
        output_text: Some(result.text),
        usage: Some(RunUsage {
            input_tokens: result.input_tokens,
            output_tokens: result.output_tokens,
        }),
    })
}

/// Extract a playbook for a successful bench run, using structured output
/// when available and otherwise falling back to the latest episode log entry.
pub(crate) async fn extract_bench_playbook(
    workdir: &Path,
    prompt: &str,
    output_text: Option<&str>,
) -> Result<Option<Playbook>> {
    if let Some(playbook) = extract_playbook_from_output_text(prompt, output_text) {
        return Ok(Some(playbook));
    }

    let Some(episode) = latest_learning_episode(workdir).await? else {
        return Ok(None);
    };
    let tool_calls = roko_learn::playbook::extract_tool_calls_from_episode(&episode);
    if tool_calls.is_empty() {
        return Ok(None);
    }

    let task_id = non_empty(&episode.task_id).unwrap_or("bench-episode");
    Ok(roko_learn::playbook::extract_playbook_from_episode(
        task_id,
        prompt,
        &tool_calls,
    ))
}

fn extract_playbook_from_output_text(prompt: &str, output_text: Option<&str>) -> Option<Playbook> {
    let tool_calls = extract_tool_calls_from_output_text(output_text)?;
    roko_learn::playbook::extract_playbook_from_episode("bench-output", prompt, &tool_calls)
}

fn extract_tool_calls_from_output_text(output_text: Option<&str>) -> Option<Vec<(String, String)>> {
    let text = non_empty(output_text?)?;
    let value = serde_json::from_str::<serde_json::Value>(text).ok()?;
    if !value.is_array() && !value.is_object() {
        return None;
    }

    let mut episode = Episode::new("bench-output", "bench-output");
    episode.extra.insert("tool_calls".to_string(), value);
    let tool_calls = roko_learn::playbook::extract_tool_calls_from_episode(&episode);
    (!tool_calls.is_empty()).then_some(tool_calls)
}

async fn latest_learning_episode(workdir: &Path) -> Result<Option<Episode>> {
    let mut last_error: Option<anyhow::Error> = None;
    for path in learning_episode_paths(workdir) {
        match EpisodeLogger::read_all_lossy(&path).await {
            Ok(episodes) => {
                if let Some(episode) = episodes.last().cloned() {
                    return Ok(Some(episode));
                }
            }
            Err(err) => {
                last_error = Some(anyhow!("read {}: {err}", path.display()));
            }
        }
    }

    if let Some(err) = last_error {
        Err(err)
    } else {
        Ok(None)
    }
}

fn learning_episode_paths(workdir: &Path) -> Vec<PathBuf> {
    let roko = workdir.join(".roko");
    // Prefer the canonical root log; keep pre-V3 locations as fallbacks.
    vec![
        roko.join("episodes.jsonl"),
        roko.join("learn").join("episodes.jsonl"),
        roko.join("memory").join("episodes.jsonl"),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use roko_runtime::workflow_contract::WorkflowConfig;
    use tempfile::TempDir;

    #[test]
    fn prompt_verify_steps_prefer_declared_rungs_then_workspace_kind() {
        let tmp = TempDir::new().unwrap();
        let mut gates = roko_core::config::GatesConfig::default();
        assert!(prompt_verify_steps(tmp.path(), &gates).is_empty());

        std::fs::write(tmp.path().join("Cargo.toml"), "").unwrap();
        let steps = prompt_verify_steps(tmp.path(), &gates);
        assert_eq!(steps.len(), 1);
        assert_eq!(steps[0].command, "cargo check --workspace");

        let rung = |name: &str, command: &str, required| roko_core::config::GateRungConfig {
            name: name.to_string(),
            command: command.to_string(),
            timeout_secs: 30,
            required,
            parallel_with: Vec::new(),
        };
        gates.custom_rungs = vec![
            rung("check", "make check", true),
            rung("lint", "make lint", false),
        ];
        let steps = prompt_verify_steps(tmp.path(), &gates);
        assert_eq!(steps.len(), 1, "optional rungs do not gate the task");
        assert_eq!(steps[0].phase, "check");
        assert_eq!(steps[0].command, "make check");
        assert_eq!(steps[0].timeout_ms, 30_000);
    }

    #[test]
    fn prompt_plan_loads_as_one_verified_task_carrying_the_prompt() {
        let tmp = TempDir::new().unwrap();
        std::fs::create_dir_all(tmp.path().join("src")).unwrap();
        std::fs::write(tmp.path().join("roko.toml"), "").unwrap();
        let run_dir = tmp.path().join(".roko").join("runs").join("run-1");
        std::fs::create_dir_all(&run_dir).unwrap();
        let prompt = "write a \"hello\" function\nwith a doc comment";
        let verify = vec![VerifyStep {
            phase: "check".to_string(),
            command: "true".to_string(),
            fail_msg: None,
            timeout_ms: 5_000,
        }];
        prompt_tasks_file(
            "run-1",
            prompt,
            "focused",
            "implementer",
            verify,
            tmp.path(),
        )
        .write(&run_dir.join("tasks.toml"))
        .unwrap();

        let plan = crate::runner::plan_loader::load_plan(&run_dir).unwrap();
        assert_eq!(plan.id, "run-1");
        let [task] = plan.tasks.tasks.as_slice() else {
            panic!("expected one task, got {:?}", plan.tasks.tasks);
        };
        assert_eq!(task.description.as_deref(), Some(prompt));
        assert_eq!(task.title, "write a \"hello\" function");
        assert_eq!(task.role.as_deref(), Some("implementer"));
        assert_eq!(task.tier, "focused");
        assert_eq!(task.files, ["roko.toml", "src"]);
        assert_eq!(task.verify.len(), 1);
        assert_eq!(task.verify[0].command, "true");
    }

    #[test]
    fn run_report_overall_success_requires_all_gates() {
        let r = RunReport {
            episode_id: "a".into(),
            prompt_id: "b".into(),
            agent_output_id: "c".into(),
            agent_success: true,
            gate_verdicts: vec![("g1".into(), true), ("g2".into(), true)],
            total_signals: 5,
            output_text: Some("done".into()),
            usage: None,
        };
        assert!(r.overall_success());

        let r = RunReport {
            gate_verdicts: vec![("g1".into(), true), ("g2".into(), false)],
            ..r
        };
        assert!(!r.overall_success());
    }

    // test_v2_share_produces_real_transcript removed: it referenced
    // EffectServices and WorkflowEngine which were retired by #276.
    // Share transcript coverage is provided by the graph workflow path.

    #[test]
    fn engine_flag_express_selects_express_config() {
        let workflow = match "express" {
            "express" => WorkflowConfig::express(),
            "full" => WorkflowConfig::full(),
            _ => WorkflowConfig::standard(),
        };

        assert!(!workflow.has_strategy);
        assert!(!workflow.has_review);
        assert_eq!(workflow.max_iterations, 1);
    }

    #[test]
    fn engine_flag_full_selects_full_config() {
        let workflow = match "full" {
            "express" => WorkflowConfig::express(),
            "full" => WorkflowConfig::full(),
            _ => WorkflowConfig::standard(),
        };

        assert!(workflow.has_strategy);
        assert!(workflow.has_review);
        assert_eq!(workflow.max_iterations, 3);
    }

    #[test]
    fn engine_flag_legacy_and_unknown_select_standard_config() {
        for workflow_template in ["legacy", "v2", "standard", "unknown"] {
            let workflow = match workflow_template {
                "express" => WorkflowConfig::express(),
                "full" => WorkflowConfig::full(),
                _ => WorkflowConfig::standard(),
            };

            assert!(
                !workflow.has_strategy,
                "{workflow_template} should not enable strategy"
            );
            assert!(
                workflow.has_review,
                "{workflow_template} should enable review"
            );
            assert_eq!(workflow.max_iterations, 2);
        }
    }

    #[test]
    fn workflow_report_outcome_reads_terminal_event() {
        let report = WorkflowRunReport {
            run_id: "run-1".to_string(),
            success: false,
            model: "test-model".to_string(),
            provider: None,
            prompt_summary: "prompt".to_string(),
            output: "output".to_string(),
            agent_turns: 0,
            token_usage: 0,
            input_tokens: 0,
            output_tokens: 0,
            cache_read_tokens: 0,
            cost: None,
            duration_secs: 0.0,
            gates: Vec::new(),
            events: vec![roko_core::runtime_event::RuntimeEventEnvelope::new(
                "run-1",
                1,
                "workflow_engine",
                roko_core::RuntimeEvent::WorkflowCompleted {
                    run_id: "run-1".to_string(),
                    outcome: roko_core::WorkflowOutcome::Halted {
                        reason: "missing API key".to_string(),
                    },
                },
            )],
            checkpoint_path: None,
        };

        assert!(matches!(
            workflow_report_outcome(&report),
            Some(roko_core::WorkflowOutcome::Halted { ref reason })
                if reason == "missing API key"
        ));
    }

    #[test]
    fn write_shared_workflow_run_scrubs_secrets() {
        let dir = tempfile::tempdir().expect("tempdir");
        let workdir = dir.path();

        // A secret that must never appear in the on-disk JSON.
        let secret = "sk-ant-secret0123456789abcdef0123456789";
        let report = WorkflowRunReport {
            run_id: "scrub-test-run".to_string(),
            success: true,
            model: "test-model".to_string(),
            provider: None,
            prompt_summary: "summary".to_string(),
            output: format!("Agent found token={secret} in env"),
            agent_turns: 1,
            token_usage: 100,
            input_tokens: 60,
            output_tokens: 40,
            cache_read_tokens: 0,
            cost: None,
            duration_secs: 1.0,
            gates: Vec::new(),
            events: Vec::new(),
            checkpoint_path: None,
        };

        let _token = write_shared_workflow_run(
            workdir,
            &format!("Run with ANTHROPIC_API_KEY={secret}"),
            "implementer",
            "coder",
            &report,
        )
        .expect("write_shared_workflow_run");

        // Find the written JSON file in .roko/shared/
        let shared_dir = workdir.join(".roko").join("shared");
        let entry = std::fs::read_dir(&shared_dir)
            .expect("read shared dir")
            .next()
            .expect("at least one entry")
            .expect("valid entry");
        let json = std::fs::read_to_string(entry.path()).expect("read json");

        assert!(
            !json.contains(secret),
            "secret leaked into shared transcript"
        );
        assert!(json.contains("[REDACTED]"), "no [REDACTED] marker found");
    }

    // ── resolve_engine_flag tests (#300) ──────────────────────────────

    #[test]
    fn resolve_engine_flag_default_is_graph() {
        assert_eq!(resolve_engine_flag(None), "graph");
    }

    #[test]
    fn resolve_engine_flag_accepts_graph_values() {
        assert_eq!(resolve_engine_flag(Some("graph")), "graph");
        assert_eq!(resolve_engine_flag(Some("graph_canary")), "graph");
    }

    #[test]
    fn resolve_engine_flag_unknown_falls_back_to_graph() {
        // Unknown values warn but do not fail.
        assert_eq!(resolve_engine_flag(Some("runner-v2")), "graph");
        assert_eq!(resolve_engine_flag(Some("legacy")), "graph");
        assert_eq!(resolve_engine_flag(Some("unknown")), "graph");
    }

    #[test]
    fn cli_overrides_has_effort_field() {
        let overrides = CliOverrides {
            model: None,
            role: None,
            provider: None,
            cascade_enabled: None,
            effort: Some("high".to_string()),
        };
        assert_eq!(overrides.effort.as_deref(), Some("high"));
    }

    // ─── #245 conformance tests ─────────────────────────────────────────

    #[test]
    fn workflow_profile_validates_via_non_plan_services() {
        let overrides = roko_execution::overrides_for_workflow(
            Some("sonnet".to_string()),
            None,
            None,
            Some(true),
            None,
        );
        let request = roko_execution::NonPlanServiceRequest::new(
            roko_execution::profiles::RuntimeProfile::Workflow,
            PathBuf::from("/tmp/test"),
            overrides,
        );
        let handle = roko_execution::build_non_plan_services(&request).unwrap();
        assert_eq!(
            handle.profile(),
            roko_execution::profiles::RuntimeProfile::Workflow
        );
        assert!(handle.cascade_enabled());
        assert!(handle.feedback_enabled());
    }

    #[test]
    fn workflow_profile_rejects_plan_profile() {
        let overrides = roko_execution::overrides_for_workflow(None, None, None, None, None);
        let request = roko_execution::NonPlanServiceRequest::new(
            roko_execution::profiles::RuntimeProfile::FullPlan,
            PathBuf::from("/tmp/test"),
            overrides,
        );
        assert!(
            roko_execution::build_non_plan_services(&request).is_err(),
            "FullPlan must be rejected by non-plan service builder"
        );
    }

    #[test]
    fn workflow_runtime_services_builder_constructs_for_workflow() {
        let tmp = TempDir::new().unwrap();
        std::fs::create_dir_all(tmp.path().join(".roko")).unwrap();
        let overrides = roko_execution::overrides::ExecutionOverrides {
            model: Some("test-model".to_string()),
            ..Default::default()
        };
        let services = roko_execution::RuntimeServicesBuilder::new(
            roko_execution::profiles::RuntimeProfile::Workflow,
            overrides,
        )
        .build(tmp.path())
        .unwrap();
        assert_eq!(
            services.profile,
            roko_execution::profiles::RuntimeProfile::Workflow
        );
    }

    #[test]
    fn chat_profile_validates_via_non_plan_services() {
        let overrides = roko_execution::overrides_for_chat(None, None);
        let request = roko_execution::NonPlanServiceRequest::new(
            roko_execution::profiles::RuntimeProfile::ChatLight,
            PathBuf::from("/tmp/test"),
            overrides,
        );
        let handle = roko_execution::build_non_plan_services(&request).unwrap();
        assert_eq!(
            handle.profile(),
            roko_execution::profiles::RuntimeProfile::ChatLight
        );
        // Chat disables affect by default
        assert!(!handle.overrides().affect_enabled.unwrap_or(true));
    }

    #[test]
    fn acp_profile_validates_via_non_plan_services() {
        let overrides = roko_execution::overrides_for_acp("test-session", None, None);
        let request = roko_execution::NonPlanServiceRequest::new(
            roko_execution::profiles::RuntimeProfile::AgentServer,
            PathBuf::from("/tmp/test"),
            overrides,
        );
        let handle = roko_execution::build_non_plan_services(&request).unwrap();
        assert_eq!(
            handle.profile(),
            roko_execution::profiles::RuntimeProfile::AgentServer
        );
        assert_eq!(handle.instance_id(), "acp_workflow_test-session",);
        assert!(handle.cascade_enabled());
    }

    /// bug-5c25e1: the CLI config replaced the core `[budget]` with its own
    /// legacy defaults, so a fresh workspace (`max_turn_usd = 0.0`, no cap)
    /// failed admission against a $1.00 turn cap and the $1.50 fallback
    /// estimate.
    #[tokio::test]
    async fn fresh_workspace_run_passes_budget_admission() {
        let tmp = TempDir::new().unwrap();
        crate::init::write_init_config(tmp.path(), false, crate::init::InitProvider::ClaudeCli)
            .expect("roko init writes roko.toml");
        let mut config = crate::config::load_resolved_config(tmp.path())
            .expect("load the fresh workspace config")
            .config;
        assert_eq!(
            config.budget.max_turn_usd, 0.0,
            "roko init sets no turn cap"
        );

        check_budget_admission(tmp.path(), &config)
            .await
            .expect("a fresh workspace passes budget admission");

        // A real turn cap below the fallback estimate still refuses the run.
        config.budget.max_turn_usd = 0.01;
        let err = check_budget_admission(tmp.path(), &config)
            .await
            .expect_err("a turn cap below the estimate refuses the run");
        assert!(err.to_string().contains("exceeds max_turn_usd"), "{err}");
    }
}
