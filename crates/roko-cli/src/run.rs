//! The universal loop: prompt → compose → agent → gate → persist → policy.
//!
//! This is the body of `roko run <prompt>`. It reads [`Config`], opens a
//! [`FileSubstrate`] under `.roko/`, seeds prompt sections, composes them
//! into a single Prompt signal, invokes the configured agent backend, runs
//! each configured gate on the working directory, and emits an Episode.

use crate::config::{Config, GateConfig, PromptFile};
use crate::model_selection::{EffectiveModelSelection, SelectionSource, resolve_effective_model};
use crate::output_format;
use crate::state_hub::{StateHub, StateHubSender};
use anyhow::{Context as _, Result, anyhow};
use chrono::Utc;
use roko_agent::provider::is_known_protocol_command;
use roko_core::AgentRole;
use roko_core::agent::resolve_model;
use roko_core::config::schema::RokoConfig;
use roko_core::dashboard_snapshot::DashboardEvent;
use roko_core::foundation::{
    EventConsumer as WorkflowEventConsumer, ShellGateCommand as CoreShellGateCommand,
};
use roko_gate::BuildSystem;
use roko_learn::episode_logger::{Episode, EpisodeLogger};
use roko_learn::playbook::Playbook;
use roko_runtime::effect_driver::EffectServices;
use roko_runtime::pipeline_state::WorkflowConfig;
use roko_runtime::workflow_engine::{WorkflowEngine, WorkflowRunConfig, WorkflowRunReport};
use roko_serve::bench::BenchStrategy;
use roko_serve::{ServiceConfig, ServiceFactory};
use std::path::{Path, PathBuf};
use std::sync::Arc;

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

    /// Return the first gate that failed, if any.
    #[must_use]
    #[allow(dead_code)]
    pub(crate) fn first_failed_gate(&self) -> Option<&str> {
        self.gate_verdicts
            .iter()
            .find_map(|(gate, passed)| (!*passed).then_some(gate.as_str()))
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

/// Bridges WorkflowEngine lifecycle events to the StateHub for TUI/SSE/WS
/// consumption.
struct StateHubBridge {
    sender: StateHubSender,
}

impl WorkflowEventConsumer for StateHubBridge {
    fn consume(&self, event: &roko_core::RuntimeEvent) {
        match event {
            roko_core::RuntimeEvent::WorkflowStarted {
                run_id,
                template,
                prompt,
            } => {
                self.sender.publish(DashboardEvent::PlanStarted {
                    plan_id: run_id.clone(),
                    tasks_total: 0,
                });
                self.sender.publish(DashboardEvent::TaskStarted {
                    plan_id: run_id.clone(),
                    task_id: "workflow".to_string(),
                    title: truncate(prompt, 60).to_string(),
                    phase: format!("starting ({template})"),
                });
            }
            roko_core::RuntimeEvent::PhaseTransition { run_id, from, to } => {
                self.sender.publish(DashboardEvent::PhaseTransition {
                    plan_id: run_id.clone(),
                    from: from.clone(),
                    to: to.clone(),
                });
            }
            roko_core::RuntimeEvent::WorkflowCompleted { run_id, outcome } => {
                let success = matches!(outcome, roko_core::WorkflowOutcome::Success { .. });
                self.sender.publish(DashboardEvent::TaskCompleted {
                    plan_id: run_id.clone(),
                    task_id: "workflow".to_string(),
                    outcome: format!("{outcome:?}"),
                });
                self.sender.publish(DashboardEvent::PlanCompleted {
                    plan_id: run_id.clone(),
                    success,
                });
            }
            _ => {}
        }
    }
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

/// CLI overrides parsed from clap args, threaded through the call chain
/// instead of re-parsing `std::env::args_os()`.
#[derive(Debug, Default, Clone)]
pub struct CliOverrides {
    pub model: Option<String>,
    pub role: Option<String>,
    pub provider: Option<String>,
    pub cascade_enabled: Option<bool>,
    /// Reasoning effort level override (e.g. "low", "medium", "high", "max").
    pub effort: Option<String>,
}

fn resolve_workflow_model_selection(
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
    if let Some(ref effort) = overrides.effort {
        config.agent.effort.clone_from(effort);
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
            "WorkflowEngine refused to run with the default `cat` agent. Run `roko init`, configure a provider in roko.toml, or pass a model/provider override."
        ));
    }
    Ok(())
}

fn build_workflow_effect_services(
    workdir: &std::path::Path,
    config: &Config,
    mut model_config: RokoConfig,
    selection: &EffectiveModelSelection,
    cascade_enabled: bool,
) -> anyhow::Result<EffectServices> {
    model_config.agent.default_model = selection.effective_model_key.clone();

    let services = ServiceFactory::build(ServiceConfig {
        workdir: workdir.to_path_buf(),
        roko_dir: workdir.join(".roko"),
        workspace_config: model_config,
        model_key: Some(selection.effective_model_key.clone()),
        mcp_config: config.agent.mcp_config.clone(),
        feedback_enabled: true,
        affect_enabled: true,
        cascade_enabled,
        run_id: Some(format!("cli_workflow_{}", Utc::now().timestamp_millis())),
        inference_observer: Some(Arc::new(
            crate::inference_observer::RuntimeEventInferenceObserver::new(),
        )),
        metrics: None,
    })
    .map_err(|error| anyhow!("build workflow services: {error}"))?;

    Ok(services.effect_services())
}

fn workflow_config_for_template(workflow_template: &str) -> WorkflowConfig {
    match workflow_template {
        "express" => WorkflowConfig::express(),
        "full" => WorkflowConfig::full(),
        _ => WorkflowConfig::standard(),
    }
}

/// Convert a `PipelineBandConfig` from `roko.toml` into a `WorkflowConfig` for the V2 engine.
fn workflow_config_from_band(band: &roko_core::config::PipelineBandConfig) -> WorkflowConfig {
    WorkflowConfig {
        has_strategy: band.strategist,
        has_review: band.reviewers,
        max_iterations: band.max_iterations,
        // When reviewers are disabled, one autofix attempt is enough.
        // When reviewers are enabled, allow two rounds.
        max_autofix_attempts: if band.reviewers { 2 } else { 1 },
    }
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

pub fn workflow_shell_gate_commands(gates: &[GateConfig]) -> Vec<CoreShellGateCommand> {
    gates
        .iter()
        .filter_map(|gate| match gate {
            GateConfig::Shell {
                program,
                args,
                timeout_ms,
            } => Some(CoreShellGateCommand {
                program: program.clone(),
                args: args.clone(),
                timeout_ms: *timeout_ms,
            }),
            _ => None,
        })
        .collect()
}

pub async fn run_workflow_engine_report_with_hub(
    prompt: &str,
    workdir: &std::path::Path,
    workflow_template: &str,
    enabled_gates: Vec<String>,
    shell_gates: Vec<CoreShellGateCommand>,
    external_hub: Option<&StateHub>,
    overrides: &CliOverrides,
) -> anyhow::Result<WorkflowRunReport> {
    let (config, model_config, selection) = resolve_workflow_model_selection(workdir, overrides)?;
    selection.print_stderr();
    let workflow_prompt = workflow_prompt_with_config_files(workdir, &config, prompt)?;
    let pipeline_config = model_config.pipeline.clone();
    let services = build_workflow_effect_services(
        workdir,
        &config,
        model_config,
        &selection,
        overrides.cascade_enabled.unwrap_or(true),
    )?;

    let workflow = match workflow_template {
        "express" | "mechanical" => workflow_config_from_band(&pipeline_config.mechanical),
        "focused" => workflow_config_from_band(&pipeline_config.focused),
        "integrative" => workflow_config_from_band(&pipeline_config.integrative),
        "full" | "architectural" => workflow_config_from_band(&pipeline_config.architectural),
        "standard" => workflow_config_from_band(&pipeline_config.mechanical),
        _ => workflow_config_for_template(workflow_template),
    };

    run_workflow_engine_with_services(
        &workflow_prompt,
        workdir,
        workflow,
        enabled_gates,
        shell_gates,
        external_hub,
        services,
        selection.provider_key,
    )
    .await
}

fn workflow_prompt_with_config_files(
    workdir: &Path,
    config: &Config,
    prompt: &str,
) -> anyhow::Result<String> {
    if config.prompt.files.is_empty() {
        return Ok(prompt.to_string());
    }

    let mut enriched = prompt.to_string();
    enriched.push_str("\n\n## Prompt Files\n");
    for file in &config.prompt.files {
        enriched.push_str("\n");
        enriched.push_str(&render_prompt_file_for_workflow(workdir, file)?);
    }
    Ok(enriched)
}

fn render_prompt_file_for_workflow(workdir: &Path, spec: &PromptFile) -> anyhow::Result<String> {
    let full_path = if spec.path.is_absolute() {
        spec.path.clone()
    } else {
        workdir.join(&spec.path)
    };
    let contents = std::fs::read_to_string(&full_path)
        .with_context(|| format!("read prompt file {}", full_path.display()))?;
    let label = spec
        .name
        .as_deref()
        .unwrap_or_else(|| spec.path.to_str().unwrap_or("prompt_file"));
    Ok(format!(
        "### {label}\nPath: `{}`\n\n{}",
        spec.path.display(),
        contents
    ))
}

async fn run_workflow_engine_with_services(
    prompt: &str,
    workdir: &std::path::Path,
    workflow: WorkflowConfig,
    enabled_gates: Vec<String>,
    shell_gates: Vec<CoreShellGateCommand>,
    external_hub: Option<&StateHub>,
    services: EffectServices,
    provider_key: String,
) -> anyhow::Result<WorkflowRunReport> {
    use roko_runtime::effect_driver::RuntimeEvent;
    use roko_runtime::jsonl_logger::{EventConsumer as RuntimeEventConsumer, JsonlLogger};

    struct JsonlWorkflowConsumer {
        logger: JsonlLogger,
    }

    impl RuntimeEventConsumer for JsonlWorkflowConsumer {
        fn consume(&self, event: &RuntimeEvent) {
            self.logger.consume(event);
        }

        fn consume_with_cursor(&self, event: &RuntimeEvent) -> Option<u64> {
            self.logger.consume_with_run_cursor(event)
        }
    }

    let config = WorkflowRunConfig {
        prompt: prompt.to_string(),
        input_messages: Vec::new(),
        workdir: workdir.to_path_buf(),
        workflow,
        enabled_gates,
        shell_gates,
        commit_prefix: Some("feat".to_string()),
    };

    let mut engine = WorkflowEngine::new(services);
    let roko_dir = workdir.join(".roko");
    let logger = JsonlLogger::from_roko_dir(&roko_dir);
    let consumer = Arc::new(JsonlWorkflowConsumer { logger });
    engine.add_consumer(consumer);

    // Bridge workflow events to the StateHub for TUI/SSE/WS consumers.
    if let Some(hub) = external_hub {
        let bridge = Arc::new(StateHubBridge {
            sender: hub.sender(),
        });
        engine.add_consumer(bridge);
    }

    let mut result = engine
        .run(config)
        .await
        .map_err(|error| anyhow!("workflow engine failed: {error}"))?;

    result.provider = Some(provider_key);

    Ok(result)
}

pub fn print_workflow_run_report(
    prompt: &str,
    workflow_template: &str,
    report: &WorkflowRunReport,
) {
    output_format::intro("roko run");
    output_format::step("prompt", &output_format::dim(&truncate(prompt, 60)));
    output_format::step("workflow", workflow_template);
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
        output_format::branch("gates      (none configured)");
    } else {
        for gate in &report.gates {
            let marker = if gate.passed { "PASS" } else { "FAIL" };
            output_format::branch(&format!("gate       [{marker}] {}", gate.name));
        }
    }
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

#[allow(dead_code)]
fn parse_agent_role(role: &str) -> Option<AgentRole> {
    let normalized = role.trim().to_ascii_lowercase();
    let normalized = normalized
        .strip_prefix("agentrole::")
        .unwrap_or(&normalized)
        .replace(['_', ' '], "-");
    Some(match normalized.as_str() {
        "conductor" => AgentRole::Conductor,
        "strategist" => AgentRole::Strategist,
        "implementer" | "engineer" | "coder" => AgentRole::Implementer,
        "architect" => AgentRole::Architect,
        "researcher" => AgentRole::Researcher,
        "auditor" => AgentRole::Auditor,
        "quick-reviewer" | "quickreviewer" => AgentRole::QuickReviewer,
        "scribe" => AgentRole::Scribe,
        "critic" => AgentRole::Critic,
        "auto-fixer" | "autofixer" => AgentRole::AutoFixer,
        "refactorer" => AgentRole::Refactorer,
        "pre-planner" | "preplanner" => AgentRole::PrePlanner,
        "doc-verifier" | "docverifier" => AgentRole::DocVerifier,
        "integration-tester" | "integrationtester" => AgentRole::IntegrationTester,
        "merge-resolver" | "mergeresolver" => AgentRole::MergeResolver,
        "terminal-validator" | "terminalvalidator" => AgentRole::TerminalValidator,
        "golem-lifecycle-tester" | "golemlifecycletester" => AgentRole::GolemLifecycleTester,
        "spec-drift-detector" | "specdriftdetector" => AgentRole::SpecDriftDetector,
        "regression-detector" | "regressiondetector" => AgentRole::RegressionDetector,
        "performance-sentinel" | "performancesentinel" => AgentRole::PerformanceSentinel,
        "coverage-tracker" | "coveragetracker" => AgentRole::CoverageTracker,
        "plan-lifecycle-mgr" | "plan-lifecycle-manager" | "planlifecyclemanager" => {
            AgentRole::PlanLifecycleManager
        }
        "cross-system-tester" | "crosssystemtester" => AgentRole::CrossSystemTester,
        "error-diagnoser" | "errordiagnoser" => AgentRole::ErrorDiagnoser,
        "dep-validator" | "dependency-validator" | "dependencyvalidator" => {
            AgentRole::DependencyValidator
        }
        "pattern-extractor" | "patternextractor" => AgentRole::PatternExtractor,
        "snapshot-comparator" | "snapshotcomparator" => AgentRole::SnapshotComparator,
        "full-loop-validator" | "fullloopvalidator" => AgentRole::FullLoopValidator,
        _ => return None,
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

#[allow(dead_code)]
fn resolved_model(config: &Config) -> String {
    if let Some(model) = &config.agent.model {
        return model.clone();
    }
    // Check routing config for configured default model before returning an empty model for
    // non-Claude commands.
    if let Ok(rc) = roko_core::config::loader::load_config_unified(std::path::Path::new(".")) {
        if !rc.agent.default_model.is_empty() {
            return rc.agent.default_model;
        }
    }
    if config.agent.command.trim().eq_ignore_ascii_case("claude") {
        "claude-sonnet-4-6".to_string()
    } else {
        String::new()
    }
}

#[allow(dead_code)]
fn dashboard_agent_model(config: &Config) -> String {
    let model = resolved_model(config);
    if !model.is_empty() {
        return model;
    }

    let command = config.agent.command.trim();
    command.to_string()
}

#[allow(dead_code)]
fn infer_provider(config: &Config) -> String {
    let command = config.agent.command.trim();
    let model = resolved_model(config).to_ascii_lowercase();
    if command.eq_ignore_ascii_case("claude") || model.starts_with("claude") {
        "anthropic".to_string()
    } else if command.eq_ignore_ascii_case("codex")
        || command.eq_ignore_ascii_case("openai")
        || model.starts_with("gpt-")
        || model.starts_with("o1")
        || model.starts_with("o3")
        || model.starts_with("o4")
    {
        "openai".to_string()
    } else if command.eq_ignore_ascii_case("ollama") || model.starts_with("ollama/") {
        "ollama".to_string()
    } else {
        command.to_string()
    }
}

#[allow(dead_code)]
fn normalized_role_label(role: &str) -> String {
    parse_agent_role(role).map_or_else(
        || role.trim().to_string(),
        |parsed| parsed.label().to_string(),
    )
}

#[allow(dead_code)]
fn role_allows_dangerous_skip_permissions(role: &str) -> bool {
    parse_agent_role(role).is_none_or(|parsed| {
        let perms = parsed.tool_permissions();
        perms.write || perms.exec || perms.git || perms.network
    })
}

#[allow(dead_code)]
fn optional_resume_session_id(config: &Config, resume_from_args: Option<String>) -> Option<String> {
    resume_from_args.or_else(|| {
        config
            .agent
            .env
            .iter()
            .find_map(|(k, v)| is_resume_env_key(k).then_some(v.trim()))
            .filter(|v| !v.is_empty())
            .map(ToOwned::to_owned)
    })
}

#[allow(dead_code)]
fn is_resume_env_key(key: &str) -> bool {
    key.eq_ignore_ascii_case("ROKO_RESUME")
        || key.eq_ignore_ascii_case("ROKO_SESSION_ID")
        || key.eq_ignore_ascii_case("CLAUDE_RESUME")
        || key.eq_ignore_ascii_case("CLAUDE_SESSION_ID")
}

#[allow(dead_code)]
fn split_resume_arg(args: &[String]) -> (Vec<String>, Option<String>) {
    let mut cleaned = Vec::with_capacity(args.len());
    let mut resume = None;
    let mut idx = 0;
    while let Some(arg) = args.get(idx) {
        if let Some(value) = arg.strip_prefix("--resume=") {
            if resume.is_none() {
                let trimmed = value.trim();
                if !trimmed.is_empty() {
                    resume = Some(trimmed.to_string());
                }
            }
            idx += 1;
            continue;
        }
        if arg == "--resume" {
            if resume.is_none()
                && let Some(value) = args
                    .get(idx + 1)
                    .map(|v| v.trim())
                    .filter(|v| !v.is_empty() && !v.starts_with('-'))
            {
                resume = Some(value.to_string());
                idx += 2;
                continue;
            }
            idx += 1;
            continue;
        }
        cleaned.push(arg.clone());
        idx += 1;
    }
    (cleaned, resume)
}

#[allow(dead_code)]
fn parse_build_system(s: &str) -> Result<BuildSystem, String> {
    match s.to_ascii_lowercase().as_str() {
        "cargo" => Ok(BuildSystem::Cargo),
        "npm" => Ok(BuildSystem::Npm),
        "go" => Ok(BuildSystem::Go),
        "python" | "py" => Ok(BuildSystem::Python),
        "forge" => Ok(BuildSystem::Forge),
        "make" => Ok(BuildSystem::Make),
        other => Err(format!("unknown build_system: {other}")),
    }
}

/// Extract model keys from the project's `roko.toml` for cascade router
/// initialization. Returns an empty vec if the config is missing or has
/// no models.
#[allow(dead_code)]
fn load_roko_config_models(workdir: &Path) -> Vec<String> {
    let config = match roko_core::config::loader::load_config_unified(workdir) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };
    config.model_slugs_for_cascade()
}

#[cfg(test)]
mod tests {
    use super::*;
    use roko_core::foundation::{
        FeedbackEvent, FeedbackSink, GateClassification, GateConfig as WorkflowGateConfig,
        GateReport, GateRunner, GateVerdict, ModelCallRequest, ModelCallResponse, ModelCaller,
        PromptAssembler, PromptSpec, TokenUsage,
    };
    use tempfile::TempDir;
    use tokio::sync::Mutex as TokioMutex;

    struct ShareMockModelCaller;

    #[async_trait::async_trait]
    impl ModelCaller for ShareMockModelCaller {
        async fn call(&self, req: ModelCallRequest) -> roko_core::Result<ModelCallResponse> {
            assert_eq!(req.model, "share-mock-model");
            let role = req.role.as_deref().unwrap_or("unknown");
            let content = format!("mock response from {role}");
            Ok(ModelCallResponse {
                content,
                model: req.model,
                usage: TokenUsage {
                    input_tokens: 11,
                    output_tokens: 7,
                    total_tokens: 18,
                    cost_usd: 0.001,
                },
                stop_reason: Some("stop".to_string()),
                request_id: Some("share-mock-request".to_string()),
            })
        }
    }

    struct ShareMockPromptAssembler {
        assembled: TokioMutex<Vec<String>>,
    }

    #[async_trait::async_trait]
    impl PromptAssembler for ShareMockPromptAssembler {
        async fn assemble(&self, spec: PromptSpec) -> roko_core::Result<String> {
            let role = spec.role.unwrap_or_else(|| "unknown".to_string());
            let task = spec.task.unwrap_or_else(|| "missing task".to_string());
            let prompt = format!("assembled prompt for {role}: {task}");
            self.assembled.lock().await.push(prompt.clone());
            Ok(prompt)
        }

        fn last_prompt_section_ids(&self) -> Vec<String> {
            vec!["share_test_section".to_string()]
        }

        fn last_knowledge_ids(&self) -> Vec<String> {
            vec!["share_test_knowledge".to_string()]
        }
    }

    struct ShareMockFeedbackSink {
        events: TokioMutex<Vec<FeedbackEvent>>,
        flushes: TokioMutex<u32>,
    }

    #[async_trait::async_trait]
    impl FeedbackSink for ShareMockFeedbackSink {
        async fn record(&self, event: FeedbackEvent) -> roko_core::Result<()> {
            self.events.lock().await.push(event);
            Ok(())
        }

        async fn flush(&self) -> roko_core::Result<()> {
            *self.flushes.lock().await += 1;
            Ok(())
        }
    }

    struct ShareMockGateRunner;

    #[async_trait::async_trait]
    impl GateRunner for ShareMockGateRunner {
        async fn run_gates(&self, config: WorkflowGateConfig) -> roko_core::Result<GateReport> {
            if config.enabled_gates.is_empty() {
                return Err(roko_core::RokoError::invalid(
                    "share test expected at least one configured gate",
                ));
            }

            Ok(GateReport {
                verdicts: config
                    .enabled_gates
                    .into_iter()
                    .map(|gate_name| GateVerdict {
                        gate_name,
                        classification: GateClassification::default(),
                        passed: true,
                        skipped: false,
                        skip_reason: None,
                        output: "mock gate passed".to_string(),
                        duration_ms: 5,
                    })
                    .collect(),
            })
        }
    }

    #[test]
    fn parse_agent_role_accepts_known_labels_and_aliases() {
        assert_eq!(
            parse_agent_role("implementer"),
            Some(AgentRole::Implementer)
        );
        assert_eq!(
            parse_agent_role("quick-reviewer"),
            Some(AgentRole::QuickReviewer)
        );
        assert_eq!(parse_agent_role("engineer"), Some(AgentRole::Implementer));
        assert_eq!(parse_agent_role("unknown-role"), None);
    }

    #[test]
    fn parse_build_system_accepts_known_names() {
        assert!(matches!(
            parse_build_system("cargo"),
            Ok(BuildSystem::Cargo)
        ));
        assert!(matches!(parse_build_system("NPM"), Ok(BuildSystem::Npm)));
        assert!(matches!(parse_build_system("py"), Ok(BuildSystem::Python)));
        assert!(parse_build_system("bazel").is_err());
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

    #[test]
    fn run_report_first_failed_gate_returns_first_failure() {
        let r = RunReport {
            episode_id: "a".into(),
            prompt_id: "b".into(),
            agent_output_id: "c".into(),
            agent_success: true,
            gate_verdicts: vec![
                ("compile".into(), true),
                ("clippy".into(), false),
                ("test".into(), false),
            ],
            total_signals: 5,
            output_text: Some("done".into()),
            usage: None,
        };

        assert_eq!(r.first_failed_gate(), Some("clippy"));
    }

    #[tokio::test]
    async fn test_v2_share_produces_real_transcript() {
        let tempdir = TempDir::new().expect("tempdir");
        init_git_workdir(tempdir.path());
        std::fs::write(
            tempdir.path().join("change.txt"),
            "share transcript change\n",
        )
        .expect("write test change");

        let prompt = "produce a share transcript with real data";
        let role = "implementer";
        let agent = "share-mock-provider";
        let prompt_assembler = Arc::new(ShareMockPromptAssembler {
            assembled: TokioMutex::new(Vec::new()),
        });
        let services = EffectServices {
            default_model: "share-mock-model".to_string(),
            model_caller: Arc::new(ShareMockModelCaller),
            prompt_assembler: prompt_assembler.clone(),
            feedback_sink: Arc::new(ShareMockFeedbackSink {
                events: TokioMutex::new(Vec::new()),
                flushes: TokioMutex::new(0),
            }),
            gate_runner: Arc::new(ShareMockGateRunner),
            affect_policy: None,
        };
        let engine = WorkflowEngine::new(services);
        let report = engine
            .run(WorkflowRunConfig {
                prompt: prompt.to_string(),
                input_messages: Vec::new(),
                workdir: tempdir.path().to_path_buf(),
                workflow: WorkflowConfig::express(),
                enabled_gates: vec!["compile".to_string()],
                shell_gates: Vec::new(),
                commit_prefix: Some("test".to_string()),
            })
            .await
            .expect("workflow run succeeds");

        let token = write_shared_workflow_run(tempdir.path(), prompt, agent, role, &report)
            .expect("shared transcript is written");
        let path = tempdir
            .path()
            .join(".roko")
            .join("shared")
            .join(format!("{token}.json"));
        let transcript: roko_serve::routes::shared_runs::RunTranscript =
            serde_json::from_str(&std::fs::read_to_string(path).expect("read transcript"))
                .expect("parse transcript");
        let assembled_prompts = prompt_assembler.assembled.lock().await;

        assert!(!transcript.agent.trim().is_empty());
        assert_ne!(transcript.agent, "unknown");
        assert_eq!(transcript.agent, agent);
        assert!(!transcript.role.trim().is_empty());
        assert_eq!(transcript.role, role);
        assert_eq!(
            assembled_prompts.first().map(String::as_str),
            Some("assembled prompt for implementer: produce a share transcript with real data")
        );
        assert_eq!(transcript.prompt, prompt);
        assert_eq!(transcript.model.as_deref(), Some("share-mock-model"));
        assert_eq!(
            transcript.output.as_deref(),
            Some("mock response from implementer")
        );
        assert!(transcript.success);
        // Gate verdicts are surfaced through RuntimeEvent::GatePassed in the
        // effect driver, so the mock "compile" gate appears in the transcript.
        assert!(!transcript.gates.is_empty());
        assert_eq!(transcript.cost_usd, Some(0.001));
        assert_eq!(
            transcript.episode_id.as_deref(),
            Some(report.run_id.as_str())
        );
        assert!(report.events.iter().any(|event| matches!(
            event.payload,
            roko_core::RuntimeEvent::AgentSpawned { ref agent_id, ref role, ref model, .. }
                if !agent_id.trim().is_empty()
                    && role == "implementer"
                    && model == "share-mock-model"
        )));
    }

    #[test]
    fn role_permissions_drive_skip_permissions_flag() {
        assert!(role_allows_dangerous_skip_permissions("implementer"));
        assert!(role_allows_dangerous_skip_permissions("researcher"));
        assert!(!role_allows_dangerous_skip_permissions("architect"));
        assert!(!role_allows_dangerous_skip_permissions("auditor"));
        assert!(role_allows_dangerous_skip_permissions("custom-role"));
    }

    #[test]
    fn split_resume_arg_extracts_and_strips_resume_flags() {
        let args = vec![
            "--foo".to_string(),
            "--resume".to_string(),
            "sess-1".to_string(),
            "--bar".to_string(),
            "--resume=sess-2".to_string(),
        ];
        let (cleaned, resume) = split_resume_arg(&args);
        assert_eq!(resume.as_deref(), Some("sess-1"));
        assert_eq!(cleaned, vec!["--foo", "--bar"]);
    }

    #[test]
    fn optional_resume_prefers_args_then_env() {
        let mut cfg = Config::default();
        cfg.agent
            .env
            .push(("ROKO_SESSION_ID".to_string(), "env-sess".to_string()));
        assert_eq!(
            optional_resume_session_id(&cfg, Some("arg-sess".to_string())).as_deref(),
            Some("arg-sess")
        );
        assert_eq!(
            optional_resume_session_id(&cfg, None).as_deref(),
            Some("env-sess")
        );
    }

    #[test]
    fn dashboard_agent_model_is_never_empty_for_run_events() {
        let mut cfg = Config::default();
        cfg.agent.command = "codex".to_string();
        cfg.agent.model = None;

        assert!(!dashboard_agent_model(&cfg).trim().is_empty());

        cfg.agent.model = Some("gpt-5.4".to_string());
        assert_eq!(dashboard_agent_model(&cfg), "gpt-5.4");
    }

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
    fn workflow_config_from_band_maps_pipeline_fields() {
        let mechanical = roko_core::config::PipelineBandConfig {
            strategist: false,
            reviewers: false,
            reviewer_mode: roko_core::config::PipelineReviewerMode::Quick,
            max_iterations: 1,
        };
        let workflow = workflow_config_from_band(&mechanical);
        assert!(!workflow.has_strategy);
        assert!(!workflow.has_review);
        assert_eq!(workflow.max_iterations, 1);
        assert_eq!(workflow.max_autofix_attempts, 1);

        let architectural = roko_core::config::PipelineBandConfig {
            strategist: true,
            reviewers: true,
            reviewer_mode: roko_core::config::PipelineReviewerMode::Full,
            max_iterations: 3,
        };
        let workflow = workflow_config_from_band(&architectural);
        assert!(workflow.has_strategy);
        assert!(workflow.has_review);
        assert_eq!(workflow.max_iterations, 3);
        assert_eq!(workflow.max_autofix_attempts, 2);
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

    fn init_git_workdir(workdir: &std::path::Path) {
        run_git(workdir, &["init"]);
        run_git(workdir, &["config", "user.email", "test@example.com"]);
        run_git(workdir, &["config", "user.name", "Roko Test"]);
    }

    fn run_git(workdir: &std::path::Path, args: &[&str]) {
        let output = std::process::Command::new("git")
            .args(args)
            .current_dir(workdir)
            .output()
            .expect("run git command");

        assert!(
            output.status.success(),
            "git {:?} failed: {}",
            args,
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
