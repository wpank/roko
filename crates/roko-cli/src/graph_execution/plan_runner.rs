//! Graph plan execution entry point for library-callable use.
//!
//! This module contains the primary graph plan execution function, extracted
//! from the binary-side `cmd_plan_run_engine` so that `serve_runtime` and
//! other library callers can invoke it without depending on the binary crate.

use std::io::IsTerminal as _;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use anyhow::{Context as _, anyhow};
use roko_fs::RokoLayout;

use crate::execution_control::{
    CommandAckReceiver, CommandAckStatus, ExecutionCommandKind, ExecutionCommandSender, ack_for,
};
use crate::exit_codes::{EXIT_FAILURE, EXIT_SUCCESS};

// ── Private helpers (ported from commands/plan.rs) ────────────────────────

fn graph_plan_topological_order(
    dependencies: &std::collections::BTreeMap<String, std::collections::BTreeSet<String>>,
) -> anyhow::Result<Vec<String>> {
    use std::collections::{BTreeMap, BTreeSet};

    let mut indegree = BTreeMap::new();
    let mut dependents = BTreeMap::<String, BTreeSet<String>>::new();
    for (plan_id, plan_dependencies) in dependencies {
        for dependency in plan_dependencies {
            if dependency == plan_id {
                anyhow::bail!("Graph plan '{plan_id}' cannot depend on itself");
            }
            if !dependencies.contains_key(dependency) {
                anyhow::bail!(
                    "Graph plan '{plan_id}' depends on unknown plan '{dependency}' in the selected plan set"
                );
            }
            dependents
                .entry(dependency.clone())
                .or_default()
                .insert(plan_id.clone());
        }
        indegree.insert(plan_id.clone(), plan_dependencies.len());
    }

    let mut ready = indegree
        .iter()
        .filter_map(|(plan_id, degree)| (*degree == 0).then_some(plan_id.clone()))
        .collect::<BTreeSet<_>>();
    let mut order = Vec::with_capacity(dependencies.len());
    while let Some(plan_id) = ready.iter().next().cloned() {
        ready.remove(&plan_id);
        order.push(plan_id.clone());

        if let Some(plan_dependents) = dependents.get(&plan_id) {
            for dependent in plan_dependents {
                let Some(degree) = indegree.get_mut(dependent) else {
                    anyhow::bail!("Graph plan dependency index is inconsistent for '{dependent}'");
                };
                *degree = degree.saturating_sub(1);
                if *degree == 0 {
                    ready.insert(dependent.clone());
                }
            }
        }
    }

    if order.len() != dependencies.len() {
        let cycle = indegree
            .into_iter()
            .filter_map(|(plan_id, degree)| (degree > 0).then_some(plan_id))
            .collect::<Vec<_>>();
        anyhow::bail!(
            "Graph plan dependency cycle involving: {}",
            cycle.join(", ")
        );
    }

    Ok(order)
}

fn graph_plan_execution_order(
    plans: &[crate::runner::plan_loader::Plan],
) -> anyhow::Result<(
    Vec<String>,
    std::collections::BTreeMap<String, std::collections::BTreeSet<String>>,
)> {
    use std::collections::{BTreeMap, BTreeSet};

    let mut plan_ids = BTreeSet::new();
    for plan in plans {
        if !plan_ids.insert(plan.id.as_str()) {
            anyhow::bail!(
                "Graph selected plan set contains duplicate plan ID '{}'",
                plan.id
            );
        }
    }

    let dependencies = plans
        .iter()
        .map(|plan| {
            let dependencies = plan
                .tasks
                .tasks
                .iter()
                .flat_map(|task| task.depends_on_plan.iter().cloned())
                .collect::<BTreeSet<_>>();
            (plan.id.clone(), dependencies)
        })
        .collect::<BTreeMap<_, _>>();

    let order = graph_plan_topological_order(&dependencies)?;
    Ok((order, dependencies))
}

fn unsatisfied_graph_plan_dependencies(
    plan_id: &str,
    dependencies: &std::collections::BTreeMap<String, std::collections::BTreeSet<String>>,
    outcomes: &std::collections::BTreeMap<String, bool>,
) -> Vec<String> {
    dependencies
        .get(plan_id)
        .into_iter()
        .flatten()
        .filter(|dependency| outcomes.get(*dependency) != Some(&true))
        .cloned()
        .collect()
}

/// Inline progress telemetry sink that prints per-node lifecycle events to
/// stderr and delegates to the inner (StateHub) sink. This provides real-time
/// feedback during Graph engine execution without requiring the full TUI.
struct InlineProgressTelemetrySink {
    inner: Arc<dyn roko_core::TelemetryEventSink>,
    show_progress: bool,
}

#[async_trait::async_trait]
impl roko_core::TelemetryEventSink for InlineProgressTelemetrySink {
    async fn emit(
        &self,
        event: &roko_core::ObservableEvent,
        ancestry: &[roko_core::LensScope],
    ) -> roko_core::error::Result<Vec<roko_core::Signal>> {
        if self.show_progress {
            match event {
                roko_core::ObservableEvent::CellStarted { block, .. } => {
                    eprintln!("    \u{25b8} executing node '{block}'...");
                }
                roko_core::ObservableEvent::CellCompleted {
                    block,
                    duration_ms,
                    cost_usd,
                    ..
                } => {
                    let secs = *duration_ms as f64 / 1000.0;
                    if *cost_usd > 0.0 {
                        eprintln!(
                            "    \u{2713} node '{block}' completed ({secs:.1}s, ${cost_usd:.4})"
                        );
                    } else {
                        eprintln!("    \u{2713} node '{block}' completed ({secs:.1}s)");
                    }
                }
                roko_core::ObservableEvent::CellFailed { block, error, .. } => {
                    eprintln!("    \u{2717} node '{block}' failed: {error}");
                }
                _ => {}
            }
        }
        self.inner.emit(event, ancestry).await
    }
}

fn join_approval_tui_thread(handle: Option<std::thread::JoinHandle<anyhow::Result<()>>>) {
    let Some(handle) = handle else {
        return;
    };

    match handle.join() {
        Ok(Ok(())) => {}
        Ok(Err(err)) => {
            tracing::error!(error = %err, "approval TUI exited with error");
        }
        Err(_) => {
            tracing::error!("approval TUI thread panicked");
        }
    }
}

/// Resolve the effective per-plan USD ceiling from CLI flags and config.
///
/// Priority order (highest to lowest):
/// 1. `no_budget = true` → `0.0` (unlimited — no enforcement)
/// 2. `budget_override = Some(amount)` → `amount.max(0.0)` (explicit CLI ceiling)
/// 3. `config_max_plan_usd` (from `roko.toml [budget].max_plan_usd`)
///
/// Returns `(effective_ceiling, bypass_block)` where `bypass_block` is `true`
/// when the caller explicitly provided a ceiling via the CLI (so the runner
/// warns on overage instead of hard-blocking).
pub fn resolve_budget_ceiling(
    budget_override: Option<f64>,
    no_budget: bool,
    config_max_plan_usd: f64,
) -> (f64, bool) {
    if no_budget {
        (0.0, true)
    } else if let Some(ceiling) = budget_override {
        (ceiling.max(0.0), true)
    } else {
        (config_max_plan_usd, false)
    }
}

/// Check that the configured provider for a model is reachable.
///
/// Ported from `commands::util::preflight_provider_for_model` so library
/// callers do not need to depend on the binary-only `commands` module.
fn preflight_provider_for_model(
    config: &roko_core::config::schema::RokoConfig,
    model_key: &str,
) -> anyhow::Result<()> {
    let model = config.models.get(model_key);
    if model.is_none() {
        if let Some(builtin) = roko_core::config::model_registry::builtin_model(model_key) {
            if std::env::var(builtin.api_key_env).is_ok() {
                return Ok(());
            }
            anyhow::bail!(
                "model '{}' requires {} but it is not set.\n  hint: export {}=<your-key>",
                model_key,
                builtin.api_key_env,
                builtin.api_key_env
            );
        }
        anyhow::bail!(
            "model '{}' not found in config.\n  hint: run `roko config models list` to see configured models, or add a [[models]] entry in roko.toml",
            model_key
        );
    }
    let model = model.expect("model should be Some after config lookup loop");
    let provider_name = &model.provider;
    let provider = config.providers.get(provider_name).ok_or_else(|| {
        anyhow!(
            "provider '{}' (for model '{}') not found in config",
            provider_name,
            model_key
        )
    })?;

    if let Some(ref env_var) = provider.api_key_env
        && !env_var.trim().is_empty()
    {
        match std::env::var(env_var) {
            Ok(val) if val.is_empty() => {
                anyhow::bail!(
                    "provider '{}' requires {} but it is empty.\n  hint: export {}=<your-key>",
                    provider_name,
                    env_var,
                    env_var
                );
            }
            Err(_) => {
                anyhow::bail!(
                    "provider '{}' requires {} but it is not set.\n  hint: export {}=<your-key>",
                    provider_name,
                    env_var,
                    env_var
                );
            }
            Ok(_) => {}
        }
    }

    Ok(())
}

// ── Public API ────────────────────────────────────────────────────────────

/// Parameters for running plans through the Graph Engine.
///
/// Mirrors the arguments of the binary-side `cmd_plan_run_engine`, but
/// replaces the `cli: &Cli` reference with plain boolean fields for the
/// flags the engine actually inspects.
#[derive(Debug)]
pub struct GraphPlanRunParams {
    pub plans_dir: PathBuf,
    pub workdir: PathBuf,
    /// Suppress all non-error output.
    pub quiet: bool,
    /// Emit structured JSON output instead of human-readable text.
    pub json: bool,
    pub resume_plan: Option<PathBuf>,
    pub fresh: bool,
    pub force_resume: bool,
    pub max_retries: Option<u32>,
    pub max_tasks: usize,
    pub budget_override: Option<f64>,
    pub no_budget: bool,
    pub cli_model_override: Option<String>,
    pub dangerously_skip_permissions: bool,
    pub log_file: Option<PathBuf>,
    pub worktree_per_task: bool,
    pub rich_topology: bool,
    pub no_tui: bool,
}

/// Execute plans via the Graph Engine path.
///
/// Loads plans using the Runner v2 plan_loader, converts each to a Graph
/// via `roko_graph::convert::plan_to_graph` (default) or
/// `roko_graph::topology::ProductionPlanTopology` (when `rich_topology` is
/// true), and runs them through the GraphEngine with the default cell registry.
pub async fn run_graph_plan(params: GraphPlanRunParams) -> anyhow::Result<i32> {
    use roko_graph::cell::CellContext;
    use roko_graph::cells::{TaskDispatcher, TaskExecutorCell};
    use roko_graph::convert::{PlanTaskInfo, plan_to_graph};
    use roko_graph::engine::GraphEngine;

    let GraphPlanRunParams {
        plans_dir,
        workdir,
        quiet,
        json,
        resume_plan,
        fresh,
        force_resume,
        max_retries,
        max_tasks,
        budget_override,
        no_budget,
        cli_model_override,
        dangerously_skip_permissions,
        log_file,
        worktree_per_task,
        rich_topology,
        no_tui,
    } = params;

    let plans_dir: &Path = &plans_dir;
    let workdir: &Path = &workdir;
    let resume_plan: Option<PathBuf> = resume_plan;
    let log_file: Option<PathBuf> = log_file;

    let run_start = std::time::Instant::now();
    let plans = crate::runner::plan_loader::load_plans(plans_dir)?;
    // Validate the complete selected set before initializing extensions or
    // launching a provider. This makes missing and cyclic cross-plan
    // dependencies fail closed without partially executing the batch.
    let (plan_execution_order, plan_dependencies) = graph_plan_execution_order(&plans)?;

    // Build the same provider/config/extension foundation used by runner-v2.
    // Graph tasks are Activities, but they must still share rate limits,
    // health state, prompt context, MCP/plugin handlers, and safety contracts.
    let mut roko_config = roko_core::config::loader::load_config_validated(workdir)
        .map_err(|error| anyhow!("load Graph runtime config: {error}"))?
        .into_config();
    roko_core::config::loader::normalize_and_validate_dispatch_models(&mut roko_config)
        .context("validate model configuration before Graph dispatch")?;

    // Merge CLI flag with config (same logic as runner-v2).
    let dangerously_skip_permissions =
        dangerously_skip_permissions || roko_config.runner.dangerously_skip_permissions;

    let (plan_budget_ceiling, budget_override_active) = resolve_budget_ceiling(
        budget_override,
        no_budget,
        f64::from(roko_config.budget.max_plan_usd),
    );
    if !roko_config.agent.default_model.trim().is_empty() {
        preflight_provider_for_model(&roko_config, &roko_config.agent.default_model)?;
    }
    let graph_run_config = crate::runner::RunConfig::from_roko_config(
        workdir.to_path_buf(),
        plans_dir.to_path_buf(),
        roko_config.clone(),
    );
    crate::runner::extension_loader::initialize_extensions(
        graph_run_config.extension_chain.as_ref(),
    )
    .await?;

    let roko_config = Arc::new(roko_config);
    let prompt_cache = Arc::new(crate::dispatch::PromptCache::load(workdir));
    let mut shared_factory = crate::dispatch::SharedAgentFactory::new(
        Arc::clone(&roko_config),
        roko_config.agent.mcp_config.as_ref(),
        graph_run_config.cascade_router.clone(),
        Some(prompt_cache),
    )
    .await
    .with_health_registry(Arc::new(
        roko_learn::provider_health::ProviderHealthRegistry::load_or_new(
            &RokoLayout::for_project(workdir)
                .learn_dir()
                .join("provider-health.json"),
        ),
    ))
    .with_error_patterns_from_disk(workdir);
    let plugin_catalog = crate::runner::extension_loader::resolve_plugin_tool_catalog(
        workdir,
        &roko_config.agent.extensions,
        &[],
    )?;
    if !plugin_catalog.plugin_tools().is_empty() {
        shared_factory = shared_factory.with_local_tool_runtime(plugin_catalog.local_runtime());
    }
    let shared_factory = Arc::new(shared_factory);
    if dangerously_skip_permissions {
        tracing::warn!(
            "running Graph Engine with --dangerously-skip-permissions: agents will execute tools without approval"
        );
    }

    // ── Learning/feedback subsystem wiring ─────────────────────────────
    //
    // Build the same feedback infrastructure that Runner-v2 uses, so Graph
    // engine runs produce episodes, efficiency events, playbook outcomes,
    // routing observations, experiment settlements, and daimon feedback.
    let graph_layout = RokoLayout::for_project(workdir);
    let graph_learn_dir = graph_layout.learn_dir();
    let _ = std::fs::create_dir_all(&graph_learn_dir);

    // ── #144: Construct daimon state early so it can be shared between
    //    the feedback facade (for plan-completion persistence) and the
    //    GraphFeedbackContext (for dispatch-time affect modulation). ───────
    let affect_path = workdir.join(".roko").join("daimon").join("affect.json");
    let shared_daimon_state: Option<std::sync::Arc<std::sync::Mutex<roko_daimon::DaimonState>>> = {
        let dims_vec = &roko_config.daimon.strategy_space.dimensions;
        if dims_vec.len() == 8 {
            // SAFETY: len == 8 is checked above, so try_into() is infallible here.
            let dims: [String; 8] = dims_vec
                .clone()
                .try_into()
                .expect("dims_vec has exactly 8 elements (checked above)");
            let def = roko_daimon::StrategySpaceDefinition {
                domain: roko_config.daimon.strategy_space.domain.clone(),
                dimensions: dims,
            };
            let mut s = roko_daimon::DaimonState::load_or_new(&affect_path);
            let _ = s.configure_strategy_space(def);
            Some(std::sync::Arc::new(std::sync::Mutex::new(s)))
        } else {
            tracing::warn!(
                dims = dims_vec.len(),
                "daimon strategy_space.dimensions must have exactly 8 entries; skipping"
            );
            None
        }
    };

    let graph_episodes_path = graph_layout.root_episodes_path();
    let graph_feedback_facade = {
        let mut facade =
            crate::runtime_feedback::FeedbackFacade::new().with_sink(std::sync::Arc::new(
                crate::runtime_feedback::EpisodeSink::at(&graph_episodes_path),
            ));
        if let Some(cascade) = &graph_run_config.cascade_router {
            facade = facade.with_sink(std::sync::Arc::new(
                crate::runtime_feedback::RoutingObservationSink::new(cascade.clone()),
            ));
        }

        // ── #143: Dream consolidation trigger on plan completion ────────
        facade = facade.with_sink(std::sync::Arc::new(
            crate::runtime_feedback::DreamConsolidationSink::new(
                workdir.to_path_buf(),
                roko_config.learning.dream_on_completion,
                roko_config.learning.dreams.trigger_on_plan_complete,
            ),
        ));

        // ── #144: Daimon affect persistence on plan completion ──────────
        if let Some(ref daimon) = shared_daimon_state {
            facade = facade.with_sink(std::sync::Arc::new(
                crate::runtime_feedback::DaimonPersistenceSink::new(
                    affect_path.clone(),
                    std::sync::Arc::clone(daimon),
                ),
            ));
        }

        // ── Theta reflection on plan completion ─────────────────────────
        //
        // Runs a five-phase reflective cycle (gamma summary, affect update,
        // calibration check, progress assessment, meta-cognition) after each
        // plan completes. Lightweight and synchronous (no LLM calls).
        let shared_cortical =
            std::sync::Arc::new(roko_runtime::heartbeat::CorticalState::default());
        let shared_theta = std::sync::Arc::new(std::sync::Mutex::new(
            roko_runtime::theta_consumer::ThetaConsumer::default(),
        ));
        facade = facade.with_sink(std::sync::Arc::new(
            crate::runtime_feedback::ThetaReflectionSink::new(
                std::sync::Arc::clone(&shared_theta),
                std::sync::Arc::clone(&shared_cortical),
            ),
        ));

        // ── Delta consolidation on plan completion ──────────────────────
        //
        // Tracks episode counts and checks trigger conditions for a dream
        // consolidation cycle (NREM replay, REM imagination, integration).
        // Bridges roko_runtime::delta_consumer into the feedback pipeline.
        let shared_delta = std::sync::Arc::new(std::sync::Mutex::new(
            roko_runtime::delta_consumer::DeltaConsumer::default(),
        ));
        facade = facade.with_sink(std::sync::Arc::new(
            crate::runtime_feedback::DeltaConsolidationSink::new(
                std::sync::Arc::clone(&shared_delta),
                std::sync::Arc::clone(&shared_cortical),
            ),
        ));

        std::sync::Arc::new(facade)
    };

    // ── P0-04: CodingOracle ─────────────────────────────────────────────
    //
    // Persists across the plan run, accumulating build/test observations
    // for predictive gate feedback. Mirrors Runner-v2's CodingOracle.
    let coding_oracle = std::sync::Arc::new(roko_learn::oracles::coding::CodingOracle::new());

    // ── P1-01: GateGamingDetector ────────────────────────────────────
    //
    // Flags when agents game the gate system by passing gates at an
    // increasing rate while delivering lower-quality outputs. Alerts are
    // appended to a JSONL file on disk.
    let gate_gaming_detector = std::sync::Arc::new(tokio::sync::Mutex::new(
        roko_learn::GateGamingDetector::new(graph_learn_dir.join("gate-gaming-alerts.jsonl")),
    ));

    // ── P1-04: HoldoutExperiment ─────────────────────────────────────
    //
    // Deterministic 80/20 train/holdout split for detecting overfitting
    // in learned routing. Learning updates are gated behind the holdout
    // partition check.
    let holdout_experiment = std::sync::Arc::new(tokio::sync::Mutex::new(
        roko_learn::HoldoutExperiment::load_or_new(
            graph_learn_dir.join("holdout-state.json"),
        )
        .unwrap_or_else(|err| {
            tracing::warn!(error = %err, "failed to load holdout experiment state; starting fresh");
            roko_learn::HoldoutExperiment::new(
                graph_learn_dir.join("holdout-state.json"),
            )
        }),
    ));

    // ── P2-01: ShadowRunner ─────────────────────────────────────────
    //
    // Records shadow dispatch decisions (infrastructure-only; no actual
    // shadow task spawn). Uses the configured default model as the
    // shadow alternative.
    let shadow_runner = std::sync::Arc::new(roko_learn::shadow::ShadowRunner::new(
        roko_learn::shadow::ShadowConfig {
            model_slug: roko_config.agent.default_model.clone(),
            prompt_variant: None,
            label: "graph-shadow".to_string(),
        },
        graph_learn_dir.join("shadow-results.jsonl"),
    ));

    let graph_feedback = crate::graph_task_dispatch::GraphFeedbackContext {
        feedback_facade: Some(graph_feedback_facade),
        efficiency_path: Some(graph_learn_dir.join("efficiency.jsonl")),
        costs_path: Some(graph_learn_dir.join("costs.jsonl")),
        playbook_dir: Some(graph_learn_dir.join("playbooks")),
        // Reuse the daimon state constructed above so the feedback facade
        // persistence sink and dispatch-time modulation share the same
        // mutable state (#144).
        daimon_state: shared_daimon_state,
        experiment_store_path: Some(graph_learn_dir.join("experiments.json")),
        gate_failures_path: Some(graph_layout.gate_failures_path()),
        post_gate_reflection_path: Some(graph_learn_dir.join("post-gate-reflections.json")),
        replan_on_gate_failure: roko_config.learning.replan_on_gate_failure,
        coding_oracle: Some(coding_oracle),
        gate_gaming_detector: Some(gate_gaming_detector),
        holdout_experiment: Some(holdout_experiment.clone()),
        shadow_runner: Some(shadow_runner),
        eval_generation_enabled: true,
        // P2-LRN-6 Loop 1: Gate threshold EMA updates after each task's
        // verify sequence. Uses the canonical workspace path so the TUI,
        // serve, and `roko learn gates` all read from the same file.
        gate_thresholds_path: Some(graph_layout.gate_thresholds_path()),
        // RAG-10: retrieval outcome JSONL for gate-pass correlation telemetry.
        retrieval_outcomes_path: Some(graph_learn_dir.join("retrieval-outcomes.jsonl")),
    };

    // ── TUI vs inline progress decision ──────────────────────────────
    //
    // Auto-enable the interactive TUI dashboard when stdout is an
    // interactive terminal, unless the user explicitly opted out with
    // --no-tui, --quiet, or --json. This mirrors the runner-v2 approval
    // TUI logic (line ~470).
    let launch_tui = !no_tui && !quiet && !json && std::io::stdout().is_terminal();

    // Keep the full SharedStateHub alive so the TUI can subscribe to the
    // live event stream. Previously this path only extracted sender().
    let state_hub = crate::state_hub::shared_state_hub();
    let state_hub_sender = state_hub.sender();
    let state_hub_sink: Arc<dyn roko_core::TelemetryEventSink> = Arc::new(
        crate::runner::graph_tui_bridge::StateHubTelemetrySink::new(state_hub_sender.clone()),
    );

    // Inline progress display: print per-node start/complete/fail to stderr
    // so the user can see what the Graph engine is doing in real time.
    // Disabled when the TUI is active — events flow through the dashboard
    // instead of being printed inline.
    let show_progress = !quiet && !json && !launch_tui;
    let graph_telemetry: Arc<dyn roko_core::TelemetryEventSink> =
        Arc::new(InlineProgressTelemetrySink {
            inner: state_hub_sink,
            show_progress,
        });

    // Wire graph engine execution into the TUI dashboard event stream.
    // Create separate TUI bridges for the task dispatcher (agent output
    // streaming) and the graph lifecycle bridge (plan/node events).
    let dispatcher_tui_bridge = crate::runner::tui_bridge::TuiBridge::new(state_hub_sender.clone());
    let graph_tui_bridge = crate::runner::graph_tui_bridge::GraphTuiBridge::new(
        crate::runner::tui_bridge::TuiBridge::new(state_hub_sender),
    );

    // ── T0 reflex store ───────────────────────────────────────────────
    //
    // Open the persisted reflex store so the dispatcher can check for
    // deterministic condition-action rules before invoking the LLM.
    // The store is cheap to open (reads one JSONL file); rules are
    // matched in-memory at sub-millisecond latency.
    let reflex_store_path = graph_learn_dir.join("reflexes.jsonl");
    let reflex_store = roko_learn::reflex_store::ReflexStore::open(&reflex_store_path);
    tracing::debug!(
        path = %reflex_store_path.display(),
        rules = reflex_store.len(),
        "T0 reflex store opened for graph plan run"
    );

    let mut dispatcher_builder = crate::graph_task_dispatch::GraphTaskDispatcher::new(
        Arc::clone(&shared_factory),
        Arc::clone(&roko_config),
        workdir.to_path_buf(),
    )
    .with_plan_budget(
        plan_budget_ceiling,
        f64::from(roko_config.budget.max_turn_usd),
        budget_override_active,
    )
    .with_cli_model_override(cli_model_override)
    .with_dangerously_skip_permissions(dangerously_skip_permissions)
    .with_feedback(graph_feedback)
    .with_reflex_store(reflex_store)
    .with_tui_bridge(dispatcher_tui_bridge);

    // ── Per-task worktree isolation (opt-in via --worktree-per-task) ──
    if worktree_per_task {
        use crate::orchestrator::worktree::{WorktreeConfig, WorktreeManager};
        let worktree_manager = WorktreeManager::new(WorktreeConfig {
            repo_root: workdir.to_path_buf(),
            base_branch: "HEAD".to_string(),
            worktrees_root: workdir.join(".roko").join("worktrees"),
            max_live: None,
            idle_ttl: std::time::Duration::from_hours(1),
        });
        let workspace_provider = Arc::new(
            crate::graph_execution::WorktreeExecutionWorkspaceProvider::new(worktree_manager),
        );
        if !quiet && !json {
            tracing::info!("per-task worktree isolation enabled (--worktree-per-task)");
        }
        dispatcher_builder = dispatcher_builder.with_workspace_provider(workspace_provider);
    }

    let graph_task_dispatcher = Arc::new(dispatcher_builder);
    let task_dispatcher: Arc<dyn TaskDispatcher> = graph_task_dispatcher.clone();

    // ── TUI execution command channel (P2-TUI-3) ─────────────────────
    //
    // Create the bi-directional command/ack channel before spawning the TUI
    // so we can wire both ends: sender+ack_rx go to the TUI, cmd_rx+ack_tx
    // stay in the async execution path. The run_id "graph-engine" is a
    // placeholder; per-plan run IDs are substituted when each plan begins.
    // Using tokio::sync::mpsc directly here because ExecutionCommandSender
    // wraps a bounded Tokio sender and we need the raw receiver.
    let (tui_cmd_sender, mut exec_cmd_rx, tui_ack_tx, tui_ack_rx) =
        ExecutionCommandSender::channel("graph-engine");
    let tui_ack_receiver = CommandAckReceiver::new(tui_ack_rx);

    // Shared pause flag: set/cleared by Pause/Resume commands from the TUI.
    // Wired into each CellContext so the task executor cell can check it
    // between agent turns (cells check this flag between turns; a paused
    // cell waits until the flag is cleared).
    let shared_pause_flag: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));

    // ── Spawn interactive TUI thread ─────────────────────────────────
    //
    // Replicates the runner-v2 approval TUI pattern: spawn the App on a
    // dedicated OS thread so it owns the terminal while the async engine
    // drives execution on the current task.
    let mut tui_handle: Option<std::thread::JoinHandle<anyhow::Result<()>>> = None;
    if launch_tui {
        // Redirect stderr to a log file so tracing output does not
        // corrupt the TUI's raw terminal display.
        let layout = RokoLayout::for_project(workdir);
        let stderr_log_path = layout.runner_stderr_log();
        let _ = std::fs::create_dir_all(stderr_log_path.parent().unwrap_or(workdir));
        #[cfg(unix)]
        if let Ok(log_file) = std::fs::File::create(&stderr_log_path) {
            use std::os::unix::io::AsRawFd;
            #[allow(unsafe_code)]
            unsafe {
                libc::dup2(log_file.as_raw_fd(), 2);
            }
        }

        let state_hub_for_tui = state_hub.clone();
        let workdir_for_tui = workdir.to_path_buf();
        let handle = std::thread::Builder::new()
            .name("roko-graph-engine-tui".to_string())
            .spawn(move || {
                let app = crate::tui::App::new_connected_with_page(
                    &workdir_for_tui,
                    None, // Default page = Tab::Dashboard
                    &state_hub_for_tui,
                )
                .without_mouse_capture()
                .with_exit_on_plan_completion()
                // P2-TUI-3: Wire the execution command sender so TUI recovery
                // keybindings (s=soft-retry, S=repair, c=reverify, F=force-advance,
                // V=reverify-plan, p=pause/resume) forward commands to this loop.
                .with_execution_command_sender(tui_cmd_sender, tui_ack_receiver);
                app.run()
            })
            .context("spawn Graph Engine TUI thread")?;
        tui_handle = Some(handle);
    }

    // ── Canonical --log-file recorder for Graph Engine (#115) ──
    let graph_event_logger: Option<Arc<dyn roko_graph::events::GraphEventSink>> =
        match log_file.as_deref() {
            Some(path) => {
                let resolved = if path.is_absolute() {
                    path.to_path_buf()
                } else {
                    workdir.join(path)
                };
                let logger = crate::runner::structured_log::GraphEventLogger::open(&resolved)
                    .map_err(|e| anyhow!("open --log-file {}: {e}", resolved.display()))?;
                Some(Arc::new(logger))
            }
            None => None,
        };

    let total_tasks: usize = plans.iter().map(|p| p.tasks.tasks.len()).sum();
    let plan_count = plans.len();

    if !quiet && !json && !launch_tui {
        let plan_names: Vec<&str> = plan_execution_order.iter().map(String::as_str).collect();
        tracing::info!(
            plan_count,
            total_tasks,
            plans = plan_names.join(", "),
            "running plans via Graph Engine"
        );
    }

    let mut all_succeeded = true;
    let mut total_output_count = 0usize;
    let mut plan_outcomes = std::collections::BTreeMap::<String, bool>::new();

    for plan_id in &plan_execution_order {
        let plan = plans
            .iter()
            .find(|plan| &plan.id == plan_id)
            .ok_or_else(|| anyhow!("Graph execution order references unloaded plan '{plan_id}'"))?;
        let unsatisfied =
            unsatisfied_graph_plan_dependencies(&plan.id, &plan_dependencies, &plan_outcomes);
        if !unsatisfied.is_empty() {
            tracing::warn!(
                plan_id = %plan.id,
                prerequisites = unsatisfied.join(", "),
                "plan blocked: prerequisite plan(s) did not succeed"
            );
            graph_tui_bridge.log_event(
                "graph.plan_blocked",
                &format!(
                    "plan '{}' blocked: prerequisites {}",
                    plan.id,
                    unsatisfied.join(", ")
                ),
            );
            plan_outcomes.insert(plan.id.clone(), false);
            all_succeeded = false;
            continue;
        }

        if !quiet && !json && !launch_tui {
            tracing::info!(
                plan_id = %plan.id,
                task_count = plan.tasks.tasks.len(),
                "running plan via Graph Engine"
            );
        }

        // Convert Runner v2 tasks into PlanTaskInfo for the converter.
        let tasks: Vec<(String, PlanTaskInfo)> = plan
            .tasks
            .tasks
            .iter()
            .map(|t| {
                let info = PlanTaskInfo {
                    title: t.title.clone(),
                    description: t.description.clone(),
                    role: t.role.clone(),
                    tier: t.tier.clone(),
                    model_hint: t.model_hint.clone(),
                    files: t.files.clone(),
                    depends_on: t.depends_on.clone(),
                    depends_on_plan: t.depends_on_plan.clone(),
                    timeout_secs: t.timeout_secs,
                    max_retries: max_retries.unwrap_or(t.max_retries),
                    domain: t.domain.as_ref().map(|d| format!("{d:?}")),
                    sequence: t.sequence,
                    full_config_json: serde_json::to_value(t).unwrap_or_default(),
                };
                (t.id.clone(), info)
            })
            .collect();

        let max_parallel = if max_tasks > 0 {
            u32::try_from(max_tasks).unwrap_or(u32::MAX)
        } else {
            plan.tasks.meta.max_parallel
        };
        let max_parallel_usize = usize::try_from(max_parallel.max(1)).unwrap_or(usize::MAX);
        let plan_dir_str = plan.dir.display().to_string();

        let (graph, registry) = if rich_topology {
            // ── Rich 11-node-per-task production topology ──────────────────
            // Warn: enricher cells are currently PassthroughCell stubs and do
            // not yet add runtime value. The richer topology is available for
            // incremental implementation of each enricher cell type.
            if !quiet && !json {
                tracing::info!(
                    "--rich-topology is active; enricher cells (knowledge, \
                     episodes, playbook, modulation, safety, experiment) are \
                     currently passthrough stubs"
                );
            }
            let topo = roko_graph::ProductionPlanTopology::new(
                &plan.id,
                &plan_dir_str,
                max_parallel_usize,
            );
            // Convert PlanTaskInfo -> TopologyTaskInfo.
            // Note: info.max_retries is already resolved (max_retries.unwrap_or(t.max_retries))
            // during the PlanTaskInfo construction above, so we use it directly.
            let topo_tasks: Vec<roko_graph::TopologyTaskInfo> = tasks
                .iter()
                .map(|(id, info)| roko_graph::TopologyTaskInfo {
                    task_id: id.clone(),
                    title: info.title.clone(),
                    description: info.description.clone(),
                    role: info.role.clone(),
                    tier: info.tier.clone(),
                    model_hint: info.model_hint.clone(),
                    files: info.files.clone(),
                    depends_on: info.depends_on.clone(),
                    timeout_secs: info.timeout_secs,
                    max_retries: info.max_retries,
                    domain: info.domain.clone(),
                    sequence: info.sequence,
                    full_config_json: info.full_config_json.clone(),
                })
                .collect();
            match topo.build(&topo_tasks) {
                Ok((g, _report)) => {
                    let mut reg = roko_graph::default_registry();
                    // Register stub passthrough cells for all topology node types.
                    roko_graph::register_topology_cells(&mut reg);
                    let plan_dispatcher = Arc::clone(&task_dispatcher);
                    reg.register("task-executor", move |config| {
                        Box::new(TaskExecutorCell::live(config, Arc::clone(&plan_dispatcher)))
                    });
                    (g, reg)
                }
                Err(e) => {
                    graph_tui_bridge.error(&format!(
                        "failed to build rich topology for plan '{}': {e}",
                        plan.id
                    ));
                    tracing::error!(plan_id = %plan.id, error = %e, "failed to build rich topology for plan");
                    plan_outcomes.insert(plan.id.clone(), false);
                    all_succeeded = false;
                    continue;
                }
            }
        } else {
            // ── Simple single-Activity-per-task converter (default) ─────────
            match plan_to_graph(&plan.id, &plan_dir_str, &tasks, max_parallel) {
                Ok(g) => {
                    let mut reg = roko_graph::default_registry();
                    let plan_dispatcher = Arc::clone(&task_dispatcher);
                    reg.register("task-executor", move |config| {
                        Box::new(TaskExecutorCell::live(config, Arc::clone(&plan_dispatcher)))
                    });
                    (g, reg)
                }
                Err(e) => {
                    graph_tui_bridge.error(&format!(
                        "failed to convert plan '{}' to graph: {e}",
                        plan.id
                    ));
                    tracing::error!(plan_id = %plan.id, error = %e, "failed to convert plan to graph");
                    plan_outcomes.insert(plan.id.clone(), false);
                    all_succeeded = false;
                    continue;
                }
            }
        };
        let mut checkpoint = crate::graph_checkpoint::prepare_graph_checkpoint(
            workdir,
            resume_plan.as_deref(),
            &plan.id,
            plan_count,
            &graph,
            fresh,
            force_resume,
        )?;
        let run_id = checkpoint.run_id().to_string();
        let replayed_entries = checkpoint.replayed_entries();
        graph_task_dispatcher
            .attach_plan_budget_checkpoint(&plan.id, checkpoint.take_cost_ledger())?;
        let mut engine = GraphEngine::new(graph, registry)
            .with_recorder(checkpoint.take_recorder())
            .with_telemetry(Arc::clone(&graph_telemetry))
            // Allow stub cells when using the rich topology. Enricher cells are
            // PassthroughCell stubs; without this the engine rejects the graph
            // at validate_for_start time.
            .with_allow_test_stubs(rich_topology);
        // Wire canonical --log-file recorder (#115): attach the event sink
        // so every GraphExecutionEvent is written to JSONL.
        if let Some(ref sink) = graph_event_logger {
            engine = engine.with_event_sink(Arc::clone(sink));
        }
        if let Some(replayer) = checkpoint.take_replayer() {
            engine = engine.with_replayer(replayer);
        }
        // P2-TUI-3: Wire the shared pause flag into CellContext so cells can
        // check it between turns and yield when the TUI sends Pause.
        let ctx = CellContext::new()
            .with_run_id(run_id.clone())
            .with_pause_flag(Arc::clone(&shared_pause_flag));

        // Validate before running.
        let issues = engine.validate();
        if !issues.is_empty() {
            graph_tui_bridge.error(&format!(
                "plan '{}' has {} validation error{}",
                plan.id,
                issues.len(),
                if issues.len() == 1 { "" } else { "s" },
            ));
            tracing::error!(plan_id = %plan.id, error_count = issues.len(), "plan has validation errors");
            for issue in &issues {
                tracing::error!(plan_id = %plan.id, issue, "validation error");
            }
            plan_outcomes.insert(plan.id.clone(), false);
            all_succeeded = false;
            checkpoint.finish(false)?;
            continue;
        }

        if replayed_entries > 0 && !quiet && !json {
            tracing::info!(
                plan_id = %plan.id,
                replayed_entries,
                manifest = %checkpoint.paths().manifest.display(),
                "resuming plan with replayed task outputs"
            );
        }

        // ── Graph TUI bridge: emit PlanStarted + per-node TaskStarted ──
        let plan_task_count = tasks.len();
        graph_tui_bridge.plan_started(&plan.id, plan_task_count);
        graph_tui_bridge.log_event(
            "graph.plan_executing",
            &format!(
                "plan '{}': {} task{}, engine=graph",
                plan.id,
                plan_task_count,
                if plan_task_count == 1 { "" } else { "s" },
            ),
        );
        // Pre-populate the TUI plan tree with all nodes.
        for (task_id, info) in &tasks {
            graph_tui_bridge.node_started(&plan.id, task_id, &info.title);
        }

        // P2-TUI-3: Use engine.start() instead of engine.execute() so we
        // can interleave command processing with graph execution. The
        // FlowHandle's cancel token is wired to Cancel commands from the TUI.
        let flow_handle = engine.start(ctx);
        let mut was_cancelled_by_tui = false;

        // ── Command polling loop ─────────────────────────────────────
        //
        // Poll TUI commands every 100 ms while the plan is running.
        // Cancel: requests early termination via FlowHandle.
        // Pause: sets shared_pause_flag; cells check this between turns.
        // Resume: clears shared_pause_flag.
        // Other commands (SoftRetry, Repair, ReverifyGates, etc.) are
        // post-execution requests; ack them and skip (they are effective
        // when re-invoked after the plan completes or restarts).
        loop {
            if !flow_handle.is_running() {
                break;
            }
            // Non-blocking drain of pending TUI commands.
            loop {
                match exec_cmd_rx.try_recv() {
                    Ok(cmd) => {
                        let ack_status = match &cmd.kind {
                            ExecutionCommandKind::Cancel => {
                                tracing::info!(
                                    plan_id = %plan.id,
                                    command_id = %cmd.command_id,
                                    "TUI cancel: requesting flow cancellation"
                                );
                                flow_handle.cancel();
                                was_cancelled_by_tui = true;
                                CommandAckStatus::Completed
                            }
                            ExecutionCommandKind::Pause => {
                                shared_pause_flag.store(true, Ordering::Release);
                                tracing::info!(
                                    plan_id = %plan.id,
                                    command_id = %cmd.command_id,
                                    "TUI pause: execution paused after current task"
                                );
                                CommandAckStatus::Completed
                            }
                            ExecutionCommandKind::Resume => {
                                shared_pause_flag.store(false, Ordering::Release);
                                tracing::info!(
                                    plan_id = %plan.id,
                                    command_id = %cmd.command_id,
                                    "TUI resume: execution resumed"
                                );
                                CommandAckStatus::Completed
                            }
                            // Post-execution commands: ack as accepted; the TUI
                            // can re-send after plan completion.
                            ExecutionCommandKind::SoftRetry
                            | ExecutionCommandKind::Repair { .. }
                            | ExecutionCommandKind::ReverifyGates
                            | ExecutionCommandKind::Skip
                            | ExecutionCommandKind::Approve { .. }
                            | ExecutionCommandKind::RejectApproval { .. }
                            | ExecutionCommandKind::Reset => {
                                tracing::debug!(
                                    plan_id = %plan.id,
                                    command_id = %cmd.command_id,
                                    kind = %cmd.kind,
                                    "TUI command queued (post-execution; plan still running)"
                                );
                                CommandAckStatus::Accepted
                            }
                        };
                        let ack = ack_for(&cmd, ack_status, None);
                        let _ = tui_ack_tx.try_send(ack);
                    }
                    Err(tokio::sync::mpsc::error::TryRecvError::Empty) => break,
                    Err(tokio::sync::mpsc::error::TryRecvError::Disconnected) => break,
                }
            }
            // Yield for 100 ms before the next command poll so we don't spin.
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }

        // Collect the final result from the background task.
        let flow_result = flow_handle.await_completion().await;
        // Clear any residual pause after the plan exits.
        shared_pause_flag.store(false, Ordering::Release);

        // Drain any commands that arrived after the flow finished.
        while let Ok(cmd) = exec_cmd_rx.try_recv() {
            let ack = ack_for(
                &cmd,
                CommandAckStatus::Accepted,
                Some("plan finished — re-run to apply".into()),
            );
            let _ = tui_ack_tx.try_send(ack);
        }

        match flow_result {
            Some(output) => {
                let output_count = output
                    .node_results
                    .iter()
                    .map(|r| r.output_count)
                    .sum::<usize>();
                total_output_count += output_count;
                let budget = graph_task_dispatcher.plan_budget_snapshot(&plan.id);
                let execution_succeeded =
                    output.success && !budget.dispatch_blocked && !was_cancelled_by_tui;

                // ── Graph TUI bridge: emit per-node completions + PlanCompleted ──
                crate::runner::graph_tui_bridge::emit_plan_lifecycle(
                    &graph_tui_bridge,
                    &plan.id,
                    plan_task_count,
                    &output,
                    execution_succeeded,
                );

                if !quiet && !json {
                    if was_cancelled_by_tui {
                        tracing::warn!(
                            plan_id = %plan.id,
                            node_count = output.node_results.len(),
                            "plan cancelled by user"
                        );
                    } else if execution_succeeded {
                        tracing::info!(
                            plan_id = %plan.id,
                            node_count = output.node_results.len(),
                            output_count,
                            "plan completed: SUCCESS"
                        );
                    } else {
                        tracing::warn!(
                            plan_id = %plan.id,
                            node_count = output.node_results.len(),
                            output_count,
                            "plan completed: FAILED"
                        );
                    }
                    // Log per-node error details so failures are not silent.
                    if !execution_succeeded {
                        for result in &output.node_results {
                            if let Some(error) = &result.error {
                                tracing::warn!(
                                    node_id = %result.node_id,
                                    status = ?result.status,
                                    %error,
                                    "node failed"
                                );
                            }
                        }
                    }
                    if budget.exhausted {
                        let ceiling = budget.ceiling_usd.unwrap_or_default();
                        if budget.dispatch_blocked {
                            tracing::warn!(
                                plan_id = %plan.id,
                                spent_usd = budget.spent_usd,
                                ceiling_usd = ceiling,
                                "plan budget exhausted"
                            );
                        } else {
                            tracing::warn!(
                                plan_id = %plan.id,
                                spent_usd = budget.spent_usd,
                                ceiling_usd = ceiling,
                                "plan budget exhausted; explicit override active, continuing"
                            );
                        }
                    }
                }
                if !execution_succeeded {
                    all_succeeded = false;
                }
                plan_outcomes.insert(plan.id.clone(), execution_succeeded);
                checkpoint.finish(execution_succeeded)?;
            }
            None => {
                // Flow was cancelled before producing a result (e.g. validation
                // failure inside start(), or the task panicked).
                let cancelled_msg = if was_cancelled_by_tui {
                    format!("plan '{}' cancelled by user", plan.id)
                } else {
                    format!("plan '{}' execution failed (no output)", plan.id)
                };
                graph_tui_bridge.error(&cancelled_msg);
                graph_tui_bridge.plan_completed(&plan.id, false);

                tracing::error!(plan_id = %plan.id, "plan execution failed: no output");
                plan_outcomes.insert(plan.id.clone(), false);
                all_succeeded = false;
                checkpoint.finish(false)?;
            }
        }
    }

    let plan_budgets = plans
        .iter()
        .map(|plan| {
            let budget = graph_task_dispatcher.plan_budget_snapshot(&plan.id);
            serde_json::json!({
                "plan_id": plan.id,
                "spent_usd": budget.spent_usd,
                "reserved_usd": budget.reserved_usd,
                "ceiling_usd": budget.ceiling_usd,
                "exhausted": budget.exhausted,
                "dispatch_blocked": budget.dispatch_blocked,
            })
        })
        .collect::<Vec<_>>();
    let total_cost_usd = plans
        .iter()
        .map(|plan| {
            graph_task_dispatcher
                .plan_budget_snapshot(&plan.id)
                .spent_usd
        })
        .sum::<f64>();

    if let Some(extension_chain) = graph_run_config.extension_chain.as_ref() {
        let mut chain = extension_chain.lock().await;
        for (name, error) in chain.shutdown_all().await {
            tracing::warn!(extension = %name, %error, "extension shutdown failed");
            all_succeeded = false;
        }
    }

    // ── Wait for TUI to exit ───────────────────────────────────────
    // The TUI is operator-owned: keep the final state visible until the
    // operator explicitly quits (same semantics as the runner-v2 path).
    if !all_succeeded {
        state_hub
            .sender()
            .publish(roko_core::DashboardEvent::Error {
                message: format!(
                    "Graph Engine: {plan_count} plan(s), {} succeeded, {} failed",
                    plan_outcomes.values().filter(|v| **v).count(),
                    plan_outcomes.values().filter(|v| !**v).count(),
                ),
            });
    }
    // ── Persist cascade router observations (UX34) ─────────────────
    //
    // Save learned routing state (confidence stats, LinUCB weights, Pareto
    // frontier) so that force_backend override outcomes and all other
    // routing observations survive across runs. Without this, in-memory
    // learning accumulated during plan execution was lost on exit.
    if let Some(cascade) = &graph_run_config.cascade_router {
        let cascade_path = graph_layout.cascade_router_path();
        if let Err(err) = cascade.save(&cascade_path) {
            tracing::warn!(
                path = %cascade_path.display(),
                error = %err,
                "failed to persist cascade router state (non-fatal)"
            );
        }
    }

    // ── Persist holdout experiment state ────────────────────────────
    //
    // Save holdout state so overfitting detection survives across runs
    // and partition assignments remain stable. Mirrors Runner-v2 cleanup.
    if let Ok(exp) = holdout_experiment.try_lock() {
        if let Err(err) = exp.save() {
            tracing::warn!(error = %err, "failed to persist holdout experiment state (non-fatal)");
        }
    }

    // ── Persist run metrics (backlog #169) ──────────────────────────
    //
    // Collect task counts and cost from the just-completed plan loop and
    // append a structured RunMetricsRecord to `.roko/learn/run-metrics.jsonl`.
    // The write is fire-and-forget on a background task so it never blocks
    // the TUI exit path.
    {
        let duration_ms = run_start.elapsed().as_millis() as u64;
        let per_plan: Vec<roko_learn::run_metrics::PlanMetrics> = plan_outcomes
            .iter()
            .map(|(id, succeeded)| {
                let plan_tasks = plans
                    .iter()
                    .find(|p| &p.id == id)
                    .map_or(0, |p| p.tasks.tasks.len());
                let completed = if *succeeded { plan_tasks } else { 0 };
                roko_learn::run_metrics::PlanMetrics {
                    plan_id: id.clone(),
                    completed: *succeeded,
                    tasks_completed: completed,
                    tasks_failed: plan_tasks.saturating_sub(completed),
                }
            })
            .collect();
        let tasks_completed: usize = per_plan.iter().map(|p| p.tasks_completed).sum();
        let tasks_failed: usize = per_plan.iter().map(|p| p.tasks_failed).sum();
        let any_budget_exhausted = plans
            .iter()
            .any(|p| graph_task_dispatcher.plan_budget_snapshot(&p.id).exhausted);
        let (agg_tokens_in, agg_tokens_out, agg_dispatch_count) =
            graph_task_dispatcher.run_aggregate_stats();
        let record = roko_learn::run_metrics::RunMetricsRecord {
            run_id: format!("graph-run-{}", chrono::Utc::now().timestamp_millis().max(0)),
            timestamp: chrono::Utc::now().to_rfc3339(),
            duration_ms,
            total_tasks,
            tasks_completed,
            tasks_failed,
            total_cost_usd,
            total_tokens_in: agg_tokens_in,
            total_tokens_out: agg_tokens_out,
            total_agent_calls: agg_dispatch_count as usize,
            budget_exhausted: any_budget_exhausted,
            plans: per_plan,
        };
        let metrics_path = graph_learn_dir.join("run-metrics.jsonl");
        tokio::spawn(async move {
            if let Err(err) = roko_learn::run_metrics::append_run_metrics(&metrics_path, &record) {
                tracing::warn!(error = %err, "failed to persist run metrics (non-fatal)");
            }
        });
    }

    join_approval_tui_thread(tui_handle.take());

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "engine": "graph",
                "dry_run": false,
                "succeeded": all_succeeded,
                "plan_count": plan_count,
                "total_tasks": total_tasks,
                "total_outputs": total_output_count,
                "total_cost_usd": total_cost_usd,
                "plan_budgets": plan_budgets,
            }))
            .unwrap_or_default()
        );
    } else if !quiet {
        // Always print a human-readable summary to stdout so `--no-tui` and
        // piped invocations produce visible output.  When the TUI was active
        // the user already saw interactive progress, but one final summary
        // line is still useful (and harmless) after the terminal is restored.
        println!(
            "Graph Engine complete: {} plan(s), {} task(s), ${:.2}",
            plan_count, total_tasks, total_cost_usd,
        );
    }

    Ok(if all_succeeded {
        EXIT_SUCCESS
    } else {
        EXIT_FAILURE
    })
}
