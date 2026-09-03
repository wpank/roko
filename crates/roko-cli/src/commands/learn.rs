//! learn command handlers.

use crate::*;
use std::collections::HashSet;

/// Format a cost value for human display.
/// Uses the heuristic: if cost is exactly 0.0 and both token counts are 0,
/// treat the value as unknown.
fn display_cost(cost_usd: f64, input_tokens: u64, output_tokens: u64) -> String {
    if cost_usd == 0.0 && input_tokens == 0 && output_tokens == 0 {
        "unknown".to_string()
    } else {
        let cost_usd = cost_usd.max(0.0);
        format!("${cost_usd:.2}")
    }
}

/// Format a cost value for recent-entry display with four decimal places.
fn display_cost_precise(cost_usd: f64, input_tokens: u64, output_tokens: u64) -> String {
    let display = display_cost(cost_usd, input_tokens, output_tokens);
    if display == "unknown" {
        display
    } else {
        let cost_usd = cost_usd.max(0.0);
        format!("${cost_usd:.4}")
    }
}

pub(crate) async fn dispatch_learn(cli: &Cli, cmd: LearnCmd) -> Result<i32> {
    let json = cli.json;

    // All learn subcommands are read-only inspections of `.roko/learn/`
    // files, so acquire a shared lock that can coexist with other readers.
    let wd_for_lock = match &cmd {
        LearnCmd::All { workdir }
        | LearnCmd::Route { workdir }
        | LearnCmd::Experiments { workdir, .. }
        | LearnCmd::Efficiency { workdir, .. }
        | LearnCmd::Episodes { workdir, .. }
        | LearnCmd::Reflexes { workdir }
        | LearnCmd::Gates { workdir }
        | LearnCmd::KnowledgeStats { workdir } => {
            workdir.clone().unwrap_or_else(|| resolve_workdir(cli))
        }
        LearnCmd::Inspect { subsystem } => inspect_workdir(cli, subsystem),
        LearnCmd::Tune { workdir, .. } => workdir.clone().unwrap_or_else(|| resolve_workdir(cli)),
    };
    let _lock = roko_cli::workspace_lock::acquire_workspace_lock_shared(
        &wd_for_lock.join(".roko"),
    )?;

    match cmd {
        LearnCmd::All { workdir } => {
            let wd = workdir.unwrap_or_else(|| resolve_workdir(cli));
            if json {
                cmd_learn_json(&wd, "all").await
            } else {
                cmd_learn(&wd, "all").await
            }
        }
        LearnCmd::Route { workdir } => {
            let wd = workdir.unwrap_or_else(|| resolve_workdir(cli));
            if json {
                cmd_learn_json(&wd, "router").await
            } else {
                cmd_learn(&wd, "router").await
            }
        }
        LearnCmd::Experiments { workdir, .. } => {
            let wd = workdir.unwrap_or_else(|| resolve_workdir(cli));
            if json {
                cmd_learn_json(&wd, "experiments").await
            } else {
                cmd_learn(&wd, "experiments").await
            }
        }
        LearnCmd::Efficiency { workdir, .. } => {
            let wd = workdir.unwrap_or_else(|| resolve_workdir(cli));
            if json {
                cmd_learn_json(&wd, "efficiency").await
            } else {
                cmd_learn(&wd, "efficiency").await
            }
        }
        LearnCmd::Episodes { workdir, .. } => {
            let wd = workdir.unwrap_or_else(|| resolve_workdir(cli));
            if json {
                cmd_learn_json(&wd, "episodes").await
            } else {
                cmd_learn(&wd, "episodes").await
            }
        }
        LearnCmd::Reflexes { workdir } => {
            let wd = workdir.unwrap_or_else(|| resolve_workdir(cli));
            if json {
                cmd_learn_json(&wd, "reflexes").await
            } else {
                cmd_learn_reflexes(&wd).await
            }
        }
        LearnCmd::Gates { workdir } => {
            let wd = workdir.unwrap_or_else(|| resolve_workdir(cli));
            if json {
                cmd_learn_json(&wd, "gates").await
            } else {
                cmd_learn(&wd, "gates").await
            }
        }
        LearnCmd::KnowledgeStats { workdir } => {
            let wd = workdir.unwrap_or_else(|| resolve_workdir(cli));
            if json {
                cmd_learn_json(&wd, "knowledge").await
            } else {
                cmd_learn(&wd, "knowledge").await
            }
        }
        LearnCmd::Inspect { subsystem } => {
            let wd = inspect_workdir(cli, &subsystem);
            cmd_learn_inspect(&wd, &subsystem, json).await
        }
        LearnCmd::Tune {
            subsystem,
            dry_run,
            workdir,
        } => {
            eprintln!("warning: 'roko learn tune' is deprecated, use 'roko learn inspect {subsystem}'");
            let wd = workdir.unwrap_or_else(|| resolve_workdir(cli));
            if dry_run {
                eprintln!("note: inspection is always read-only; --dry-run has no effect");
            }
            cmd_learn_inspect_legacy(&wd, &subsystem, json).await
        }
    }
}

// ── Inspect command ─────────────────────────────────────────────────

/// Extract workdir from an `InspectSubsystem` variant.
fn inspect_workdir(cli: &Cli, subsystem: &InspectSubsystem) -> PathBuf {
    match subsystem {
        InspectSubsystem::Gates { workdir }
        | InspectSubsystem::Routing { workdir }
        | InspectSubsystem::Budget { workdir } => {
            workdir.clone().unwrap_or_else(|| resolve_workdir(cli))
        }
    }
}

/// `roko learn inspect <subsystem>` — rich, read-only inspection.
#[allow(clippy::cast_precision_loss)]
async fn cmd_learn_inspect(
    workdir: &std::path::Path,
    subsystem: &InspectSubsystem,
    json: bool,
) -> Result<i32> {
    match subsystem {
        InspectSubsystem::Gates { .. } => inspect_gates(workdir, json),
        InspectSubsystem::Routing { .. } => inspect_routing(workdir, json),
        InspectSubsystem::Budget { .. } => inspect_budget(workdir, json).await,
    }
}

/// Legacy `roko learn tune <name>` now routes to the matching inspect handler.
#[allow(clippy::cast_precision_loss)]
async fn cmd_learn_inspect_legacy(
    workdir: &std::path::Path,
    subsystem: &str,
    json: bool,
) -> Result<i32> {
    match subsystem {
        "gates" => inspect_gates(workdir, json),
        "routing" => inspect_routing(workdir, json),
        "budget" => inspect_budget(workdir, json).await,
        other => anyhow::bail!(
            "unknown subsystem '{other}'. Available: gates, routing, budget"
        ),
    }
}

// ── Inspect: gates ──────────────────────────────────────────────────

/// Typed representation for gate threshold JSON output.
#[derive(serde::Serialize)]
struct InspectGatesJson {
    path: String,
    rung_count: usize,
    rungs: serde_json::Value,
}

fn inspect_gates(workdir: &std::path::Path, json: bool) -> Result<i32> {
    let path = learn_gate_thresholds_path(workdir);

    if !path.exists() {
        if json {
            let output = InspectGatesJson {
                path: path.display().to_string(),
                rung_count: 0,
                rungs: serde_json::Value::Object(Default::default()),
            };
            println!("{}", serde_json::to_string_pretty(&output)?);
        } else {
            print_no_data(&path);
        }
        return Ok(EXIT_SUCCESS);
    }

    let content = std::fs::read_to_string(&path)?;
    let thresholds: serde_json::Value = serde_json::from_str(&content)?;
    let rung_count = thresholds
        .get("rungs")
        .and_then(serde_json::Value::as_object)
        .map_or(0, |rungs| rungs.len());

    if json {
        let output = InspectGatesJson {
            path: path.display().to_string(),
            rung_count,
            rungs: thresholds
                .get("rungs")
                .cloned()
                .unwrap_or(serde_json::Value::Object(Default::default())),
        };
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        println!("Adaptive gate thresholds ({})", path.display());
        println!("  Rungs: {rung_count}");
        if let Some(rungs) = thresholds.get("rungs").and_then(|v| v.as_object()) {
            for (rung_key, rung_val) in rungs {
                let ema = rung_val
                    .get("ema_pass_rate")
                    .and_then(|v| v.as_f64())
                    .map(|v| format!("{v:.2}"))
                    .unwrap_or_else(|| "n/a".to_string());
                let count = rung_val
                    .get("observation_count")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
                println!("    rung {rung_key}: EMA pass rate={ema}, observations={count}");
            }
        }
    }

    Ok(EXIT_SUCCESS)
}

// ── Inspect: routing ────────────────────────────────────────────────

/// Typed representation for routing JSON output.
#[derive(serde::Serialize)]
struct InspectRoutingJson {
    path: String,
    total_observations: u64,
    stage: String,
    models: Vec<LearnJsonRouterModel>,
}

fn inspect_routing(workdir: &std::path::Path, json: bool) -> Result<i32> {
    let path = learn_router_path(workdir);

    if !path.exists() {
        if json {
            let output = InspectRoutingJson {
                path: path.display().to_string(),
                total_observations: 0,
                stage: "static".to_string(),
                models: Vec::new(),
            };
            println!("{}", serde_json::to_string_pretty(&output)?);
        } else {
            print_no_data(&path);
        }
        return Ok(EXIT_SUCCESS);
    }

    let content = std::fs::read_to_string(&path)?;
    let snapshot =
        serde_json::from_str::<LearnCascadeRouterSnapshot>(&content).unwrap_or_default();
    let configured_slugs = roko_core::config::loader::load_config_unified(workdir)
        .ok()
        .map(|config| {
            config
                .model_slugs_for_cascade()
                .into_iter()
                .collect::<HashSet<_>>()
        })
        .unwrap_or_default();
    let model_rows = learn_router_model_rows(&snapshot, &configured_slugs);
    let total_observations = if snapshot.total_observations > 0 {
        snapshot.total_observations
    } else {
        model_rows.iter().map(|row| row.trials).sum()
    };
    let stage = cascade_stage_for_observations(total_observations).to_string();

    if json {
        let output = InspectRoutingJson {
            path: path.display().to_string(),
            total_observations,
            stage,
            models: model_rows
                .into_iter()
                .map(|row| LearnJsonRouterModel {
                    slug: row.slug,
                    trials: row.trials,
                    successes: row.successes,
                    available: row.available,
                })
                .collect(),
        };
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        println!("Cascade router state ({})", path.display());
        println!("  Observations: {total_observations}");
        println!("  Stage: {stage}");
        if model_rows.is_empty() {
            println!("  Models: none");
        } else {
            println!("  Models:");
            for row in &model_rows {
                let suffix = if row.available { "" } else { " (unavailable)" };
                println!(
                    "    {}{}: {} trials, {} successes",
                    row.slug, suffix, row.trials, row.successes
                );
            }
        }
    }

    Ok(EXIT_SUCCESS)
}

// ── Inspect: budget ─────────────────────────────────────────────────

/// Typed representation for budget JSON output.
#[derive(serde::Serialize)]
struct InspectBudgetJson {
    config: InspectBudgetConfigJson,
    efficiency: InspectBudgetEfficiencyJson,
}

#[derive(serde::Serialize)]
struct InspectBudgetConfigJson {
    max_plan_usd: f32,
    max_task_usd: f32,
    max_turn_usd: f32,
    max_task_retry_usd: f32,
    prompt_token_budget: usize,
}

#[derive(serde::Serialize)]
struct InspectBudgetEfficiencyJson {
    path: String,
    total_events: usize,
    passed: usize,
    failed: usize,
    total_cost_usd: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    first_seen: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    last_seen: Option<String>,
}

#[allow(clippy::cast_precision_loss)]
async fn inspect_budget(workdir: &std::path::Path, json: bool) -> Result<i32> {
    // Load configured budget
    let config = roko_core::config::loader::load_config_unified(workdir)
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    let budget = &config.budget;

    // Parse efficiency log for spend summaries
    let eff_path = learn_efficiency_path(workdir);
    let text = tokio::fs::read_to_string(&eff_path).await.unwrap_or_default();

    let mut total_events = 0usize;
    let mut passed = 0usize;
    let mut failed = 0usize;
    let mut total_cost = 0.0f64;
    let mut first_seen: Option<chrono::DateTime<chrono::Utc>> = None;
    let mut last_seen: Option<chrono::DateTime<chrono::Utc>> = None;

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let Ok(event) =
            serde_json::from_str::<roko_learn::efficiency::AgentEfficiencyEvent>(trimmed)
        else {
            continue;
        };
        total_events += 1;
        total_cost += event.cost_usd;
        match event.gate_passed {
            Some(true) => passed += 1,
            Some(false) => failed += 1,
            None => {}
        }
        if let Some(ts) = parse_rfc3339_utc(&event.timestamp) {
            first_seen = Some(first_seen.map_or(ts, |c| c.min(ts)));
            last_seen = Some(last_seen.map_or(ts, |c| c.max(ts)));
        }
    }

    if json {
        let output = InspectBudgetJson {
            config: InspectBudgetConfigJson {
                max_plan_usd: budget.max_plan_usd,
                max_task_usd: budget.max_task_usd,
                max_turn_usd: budget.max_turn_usd,
                max_task_retry_usd: budget.max_task_retry_usd,
                prompt_token_budget: budget.prompt_token_budget,
            },
            efficiency: InspectBudgetEfficiencyJson {
                path: eff_path.display().to_string(),
                total_events,
                passed,
                failed,
                total_cost_usd: total_cost,
                first_seen: first_seen.map(|ts| ts.to_rfc3339()),
                last_seen: last_seen.map(|ts| ts.to_rfc3339()),
            },
        };
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        let fmt_limit = |v: f32| -> String {
            if v <= 0.0 {
                "unlimited".to_string()
            } else {
                format!("${v:.2}")
            }
        };
        println!("Budget configuration");
        println!("  max_plan_usd:       {}", fmt_limit(budget.max_plan_usd));
        println!("  max_task_usd:       {}", fmt_limit(budget.max_task_usd));
        println!("  max_turn_usd:       {}", fmt_limit(budget.max_turn_usd));
        println!(
            "  max_task_retry_usd: {}",
            fmt_limit(budget.max_task_retry_usd)
        );
        println!("  prompt_token_budget: {}", budget.prompt_token_budget);
        println!();
        println!("Spend history ({})", eff_path.display());
        println!("  Events: {total_events} ({passed} passed, {failed} failed)");
        println!("  Total cost: ${total_cost:.4}");
        println!(
            "  Range: {}",
            format_range(first_seen, last_seen)
        );
    }

    Ok(EXIT_SUCCESS)
}

/// `roko learn [what]` — display learning subsystem state.
pub(crate) async fn cmd_learn(workdir: &std::path::Path, what: &str) -> Result<i32> {
    let show_all = what == "all";

    if show_all || what == "router" {
        print_learn_router(workdir);
    }

    if show_all || what == "experiments" {
        print_learn_experiments(workdir);
    }

    if show_all || what == "efficiency" {
        print_learn_efficiency(workdir).await;
    }

    if show_all || what == "episodes" {
        print_learn_episodes(workdir).await;
    }

    if show_all || what == "reflexes" {
        print_learn_reflexes(workdir);
    }

    if show_all {
        print_learn_gate_thresholds(workdir);
        print_learn_knowledge(workdir).await;
    }

    if !show_all
        && ![
            "router",
            "experiments",
            "efficiency",
            "episodes",
            "reflexes",
        ]
        .contains(&what)
    {
        anyhow::bail!(
            "unknown learning area '{what}'. Available: router, experiments, efficiency, episodes, reflexes, all"
        );
    }

    Ok(EXIT_SUCCESS)
}

// ── JSON output ────────────────────────────────────────────────────

#[derive(serde::Serialize)]
struct LearnJsonOutput {
    #[serde(skip_serializing_if = "Option::is_none")]
    cascade_router: Option<LearnJsonRouter>,
    #[serde(skip_serializing_if = "Option::is_none")]
    experiments: Option<LearnJsonExperiments>,
    #[serde(skip_serializing_if = "Option::is_none")]
    efficiency: Option<LearnJsonEfficiency>,
    #[serde(skip_serializing_if = "Option::is_none")]
    episodes: Option<LearnJsonEpisodes>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reflexes: Option<LearnJsonReflexes>,
    #[serde(skip_serializing_if = "Option::is_none")]
    gate_thresholds: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    knowledge: Option<LearnJsonKnowledge>,
}

#[derive(serde::Serialize)]
struct LearnJsonRouter {
    total_observations: u64,
    stage: String,
    models: Vec<LearnJsonRouterModel>,
}

#[derive(serde::Serialize)]
struct LearnJsonRouterModel {
    slug: String,
    trials: u64,
    successes: u64,
    available: bool,
}

#[derive(serde::Serialize)]
struct LearnJsonExperiments {
    prompt: LearnJsonExperimentGroup,
    model: LearnJsonExperimentGroup,
}

#[derive(serde::Serialize)]
struct LearnJsonExperimentGroup {
    running: usize,
    concluded: usize,
}

#[derive(serde::Serialize)]
struct LearnJsonEfficiency {
    total_events: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    first_seen: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    last_seen: Option<String>,
    latest: Vec<LearnJsonEfficiencyEntry>,
}

#[derive(serde::Serialize)]
struct LearnJsonEfficiencyEntry {
    timestamp: String,
    model: String,
    task_id: String,
    plan_id: String,
    gate_passed: bool,
    cost_usd: f64,
    input_tokens: u64,
    output_tokens: u64,
}

#[derive(serde::Serialize)]
struct LearnJsonEpisodes {
    total: usize,
    passed: usize,
    failed: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    first_seen: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    last_seen: Option<String>,
    latest: Vec<LearnJsonEpisodeEntry>,
}

#[derive(serde::Serialize)]
struct LearnJsonEpisodeEntry {
    timestamp: String,
    model: String,
    task_id: String,
    success: bool,
    cost_usd: f64,
    input_tokens: u64,
    output_tokens: u64,
}

#[derive(serde::Serialize)]
struct LearnJsonReflexes {
    total_rules: usize,
    max_rules: usize,
    top_rules: Vec<roko_learn::reflex_store::ReflexRule>,
    recent_demotions: Vec<roko_learn::efficiency::AgentEfficiencyEvent>,
}

#[derive(serde::Serialize)]
struct LearnJsonKnowledge {
    total_entries: usize,
}

/// Maximum number of recent entries to include in JSON output.
const JSON_LATEST_LIMIT: usize = 10;

/// `roko learn [what] --json` — structured JSON output.
#[allow(clippy::cast_precision_loss)]
async fn cmd_learn_json(workdir: &std::path::Path, what: &str) -> Result<i32> {
    let show_all = what == "all";

    let cascade_router = if show_all || what == "router" {
        Some(collect_router_json(workdir))
    } else {
        None
    };

    let experiments = if show_all || what == "experiments" {
        Some(collect_experiments_json(workdir))
    } else {
        None
    };

    let efficiency = if show_all || what == "efficiency" {
        Some(collect_efficiency_json(workdir).await)
    } else {
        None
    };

    let episodes = if show_all || what == "episodes" {
        Some(collect_episodes_json(workdir).await)
    } else {
        None
    };

    let reflexes = if show_all || what == "reflexes" {
        Some(collect_reflexes_json(workdir))
    } else {
        None
    };

    let gate_thresholds = if show_all {
        collect_gate_thresholds_json(workdir)
    } else {
        None
    };

    let knowledge = if show_all {
        Some(collect_knowledge_json(workdir).await)
    } else {
        None
    };

    if !show_all
        && ![
            "router",
            "experiments",
            "efficiency",
            "episodes",
            "reflexes",
        ]
        .contains(&what)
    {
        anyhow::bail!(
            "unknown learning area '{what}'. Available: router, experiments, efficiency, episodes, reflexes, all"
        );
    }

    let output = LearnJsonOutput {
        cascade_router,
        experiments,
        efficiency,
        episodes,
        reflexes,
        gate_thresholds,
        knowledge,
    };

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(EXIT_SUCCESS)
}

fn collect_router_json(workdir: &std::path::Path) -> LearnJsonRouter {
    let path = learn_router_path(workdir);
    let snapshot = std::fs::read_to_string(&path)
        .ok()
        .and_then(|c| serde_json::from_str::<LearnCascadeRouterSnapshot>(&c).ok())
        .unwrap_or_default();

    let configured_slugs = roko_core::config::loader::load_config_unified(workdir)
        .ok()
        .map(|config| {
            config
                .model_slugs_for_cascade()
                .into_iter()
                .collect::<HashSet<_>>()
        })
        .unwrap_or_default();

    let model_rows = learn_router_model_rows(&snapshot, &configured_slugs);
    let total_observations = if snapshot.total_observations > 0 {
        snapshot.total_observations
    } else {
        model_rows.iter().map(|row| row.trials).sum()
    };

    LearnJsonRouter {
        total_observations,
        stage: cascade_stage_for_observations(total_observations).to_string(),
        models: model_rows
            .into_iter()
            .map(|row| LearnJsonRouterModel {
                slug: row.slug,
                trials: row.trials,
                successes: row.successes,
                available: row.available,
            })
            .collect(),
    }
}

fn collect_experiments_json(workdir: &std::path::Path) -> LearnJsonExperiments {
    let prompt_path = learn_root(workdir).join("experiments.json");
    let prompt_store = ExperimentStore::load_or_new(&prompt_path);

    let model_path = learn_root(workdir).join("model-experiments.json");
    let model_store = roko_learn::model_experiment::ModelExperimentStore::load_or_new(&model_path);

    LearnJsonExperiments {
        prompt: LearnJsonExperimentGroup {
            running: prompt_store.running_count(),
            concluded: prompt_store.concluded_count(),
        },
        model: LearnJsonExperimentGroup {
            running: model_store.running_count(),
            concluded: model_store.concluded_experiments().len(),
        },
    }
}

async fn collect_efficiency_json(workdir: &std::path::Path) -> LearnJsonEfficiency {
    let path = learn_efficiency_path(workdir);
    let text = tokio::fs::read_to_string(&path).await.unwrap_or_default();

    let mut count = 0usize;
    let mut first_seen: Option<chrono::DateTime<chrono::Utc>> = None;
    let mut last_seen: Option<chrono::DateTime<chrono::Utc>> = None;
    let mut tail: Vec<LearnJsonEfficiencyEntry> = Vec::new();

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let Ok(event) =
            serde_json::from_str::<roko_learn::efficiency::AgentEfficiencyEvent>(trimmed)
        else {
            continue;
        };

        count += 1;
        if let Some(ts) = parse_rfc3339_utc(&event.timestamp) {
            first_seen = Some(first_seen.map_or(ts, |c| c.min(ts)));
            last_seen = Some(last_seen.map_or(ts, |c| c.max(ts)));
        }

        let model = efficiency_model_label(&event).to_string();
        tail.push(LearnJsonEfficiencyEntry {
            timestamp: event.timestamp.clone(),
            model,
            task_id: event.task_id.clone(),
            plan_id: event.plan_id.clone(),
            gate_passed: event.gate_passed.unwrap_or(false),
            cost_usd: event.cost_usd,
            input_tokens: event.input_tokens,
            output_tokens: event.output_tokens,
        });
        if tail.len() > JSON_LATEST_LIMIT {
            tail.remove(0);
        }
    }

    LearnJsonEfficiency {
        total_events: count,
        first_seen: first_seen.map(|ts| ts.to_rfc3339()),
        last_seen: last_seen.map(|ts| ts.to_rfc3339()),
        latest: tail,
    }
}

async fn collect_episodes_json(workdir: &std::path::Path) -> LearnJsonEpisodes {
    let path = roko_learn::runtime_feedback::resolve_project_episode_path(workdir);
    let text = tokio::fs::read_to_string(&path).await.unwrap_or_default();

    let mut total = 0usize;
    let mut passed = 0usize;
    let mut failed = 0usize;
    let mut first_seen: Option<chrono::DateTime<chrono::Utc>> = None;
    let mut last_seen: Option<chrono::DateTime<chrono::Utc>> = None;
    let mut tail: Vec<LearnJsonEpisodeEntry> = Vec::new();

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let Ok(episode) = serde_json::from_str::<roko_learn::episode_logger::Episode>(trimmed)
        else {
            continue;
        };

        total += 1;
        if episode.success {
            passed += 1;
        } else {
            failed += 1;
        }

        let ts = episode.timestamp;
        first_seen = Some(first_seen.map_or(ts, |c| c.min(ts)));
        last_seen = Some(last_seen.map_or(ts, |c| c.max(ts)));

        tail.push(LearnJsonEpisodeEntry {
            timestamp: episode.timestamp.to_rfc3339(),
            model: episode.model.clone(),
            task_id: episode.task_id.clone(),
            success: episode.success,
            cost_usd: episode.usage.cost_usd,
            input_tokens: episode.usage.input_tokens,
            output_tokens: episode.usage.output_tokens,
        });
        if tail.len() > JSON_LATEST_LIMIT {
            tail.remove(0);
        }
    }

    LearnJsonEpisodes {
        total,
        passed,
        failed,
        first_seen: first_seen.map(|ts| ts.to_rfc3339()),
        last_seen: last_seen.map(|ts| ts.to_rfc3339()),
        latest: tail,
    }
}

fn collect_reflexes_json(workdir: &std::path::Path) -> LearnJsonReflexes {
    let mut rules = reflex_store_snapshot(workdir);
    let total_rules = rules.len();
    rules.truncate(REFLEX_DISPLAY_LIMIT);
    LearnJsonReflexes {
        total_rules,
        max_rules: roko_learn::reflex_store::MAX_RULES,
        top_rules: rules,
        recent_demotions: recent_reflex_demotions(workdir),
    }
}

fn collect_gate_thresholds_json(workdir: &std::path::Path) -> Option<serde_json::Value> {
    let path = learn_gate_thresholds_path(workdir);
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|c| serde_json::from_str(&c).ok())
}

async fn collect_knowledge_json(workdir: &std::path::Path) -> LearnJsonKnowledge {
    let path = learn_knowledge_path(workdir);
    let count = tokio::fs::read_to_string(&path)
        .await
        .map(|content| {
            content
                .lines()
                .filter(|line| !line.trim().is_empty())
                .filter(|line| serde_json::from_str::<serde_json::Value>(line).is_ok())
                .count()
        })
        .unwrap_or(0);

    LearnJsonKnowledge {
        total_entries: count,
    }
}

pub(crate) fn print_learn_router(workdir: &std::path::Path) {
    let path = learn_router_path(workdir);
    print_checked_path(&path);
    if !path.exists() {
        print_no_data(&path);
        return;
    }
    let Ok(content) = std::fs::read_to_string(&path) else {
        println!("Cascade router: 0 entries at {}", path.display());
        return;
    };
    let snapshot = serde_json::from_str::<LearnCascadeRouterSnapshot>(&content).unwrap_or_default();
    // Compare against the runtime wire slugs, not the config map keys.
    let configured_slugs = roko_core::config::loader::load_config_unified(workdir)
        .map_err(|e| anyhow::anyhow!("{e}"))
        .ok()
        .map(|config| {
            config
                .model_slugs_for_cascade()
                .into_iter()
                .collect::<HashSet<_>>()
        })
        .unwrap_or_default();
    let model_rows = learn_router_model_rows(&snapshot, &configured_slugs);
    let total_observations = if snapshot.total_observations > 0 {
        snapshot.total_observations
    } else {
        model_rows.iter().map(|row| row.trials).sum()
    };

    let mut first_seen: Option<chrono::DateTime<chrono::Utc>> = None;
    let mut last_seen: Option<chrono::DateTime<chrono::Utc>> = None;
    for transition in &snapshot.stage_transitions {
        first_seen = Some(match first_seen {
            Some(current) => current.min(transition.timestamp.clone()),
            None => transition.timestamp.clone(),
        });
        last_seen = Some(match last_seen {
            Some(current) => current.max(transition.timestamp.clone()),
            None => transition.timestamp.clone(),
        });
    }

    let latest = snapshot
        .stage_transitions
        .last()
        .map(|transition| {
            format!(
                "{} {} -> {} after {} observations",
                transition.timestamp.to_rfc3339(),
                transition.from,
                transition.to,
                transition.observations
            )
        })
        .unwrap_or_else(|| {
            format!(
                "snapshot stage={} total_observations={}",
                cascade_stage_for_observations(total_observations),
                total_observations
            )
        });

    if total_observations == 0 && model_rows.is_empty() {
        println!("Cascade router: 0 entries at {}", path.display());
        return;
    }

    println!(
        "Cascade router: {} observations, {} models at {}",
        total_observations,
        model_rows.len(),
        path.display()
    );
    println!("  Range: {}", format_range(first_seen, last_seen));
    println!("  Latest: {}", latest);

    if !model_rows.is_empty() {
        println!("  Models:");
        for row in model_rows {
            let suffix = if row.available { "" } else { " (unavailable)" };
            println!(
                "    {}{}: {} obs, {} successes",
                row.slug, suffix, row.trials, row.successes
            );
        }
    }
}

pub(crate) fn print_learn_experiments(workdir: &std::path::Path) {
    // Prompt experiments
    let prompt_path = learn_root(workdir).join("experiments.json");
    print_checked_path(&prompt_path);
    let prompt_store = ExperimentStore::load_or_new(&prompt_path);
    let running = prompt_store.running_count();
    let concluded = prompt_store.concluded_count();
    if running > 0 || concluded > 0 {
        println!(
            "Prompt experiments: {} running, {} concluded",
            running, concluded
        );
    } else if prompt_path.exists() {
        println!("Prompt experiments: 0 entries at {}", prompt_path.display());
    } else {
        println!("Prompt experiments: none");
    }

    // Model experiments
    let model_path = learn_root(workdir).join("model-experiments.json");
    print_checked_path(&model_path);
    let model_store = roko_learn::model_experiment::ModelExperimentStore::load_or_new(&model_path);
    let model_running = model_store.running_count();
    let model_concluded = model_store.concluded_experiments().len();
    if model_running > 0 || model_concluded > 0 {
        println!(
            "Model experiments: {} running, {} concluded",
            model_running, model_concluded
        );
        for exp in model_store.iter() {
            println!(
                "  {} [{:?}] role={} variants={} winner={}",
                exp.experiment_id,
                exp.status,
                exp.role.as_deref().unwrap_or("any"),
                exp.variants.len(),
                exp.winner_id.as_deref().unwrap_or("-"),
            );
        }
    } else if model_path.exists() {
        println!("Model experiments: 0 entries at {}", model_path.display());
    } else {
        println!("Model experiments: none");
    }
}

#[allow(clippy::cast_precision_loss)]
pub(crate) async fn print_learn_efficiency(workdir: &std::path::Path) {
    let path = learn_efficiency_path(workdir);
    print_checked_path(&path);
    if !path.exists() {
        print_no_data(&path);
        return;
    }

    let Ok(text) = tokio::fs::read_to_string(&path).await else {
        println!("Efficiency: 0 entries at {}", path.display());
        return;
    };

    let mut count = 0usize;
    let mut first_seen: Option<chrono::DateTime<chrono::Utc>> = None;
    let mut last_seen: Option<chrono::DateTime<chrono::Utc>> = None;
    let mut latest: Option<String> = None;
    let mut events = Vec::new();

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let Ok(event) =
            serde_json::from_str::<roko_learn::efficiency::AgentEfficiencyEvent>(trimmed)
        else {
            continue;
        };

        count += 1;
        let parsed_timestamp = parse_rfc3339_utc(&event.timestamp);
        if let Some(timestamp) = parsed_timestamp {
            first_seen = Some(match first_seen {
                Some(current) => current.min(timestamp),
                None => timestamp,
            });
            last_seen = Some(match last_seen {
                Some(current) => current.max(timestamp),
                None => timestamp,
            });
        }

        let timestamp = parsed_timestamp
            .map(|ts| ts.to_rfc3339())
            .unwrap_or_else(|| event.timestamp.clone());
        let model = efficiency_model_label(&event);
        let task_id = non_empty_or_unknown(&event.task_id);
        let plan_id = non_empty_or_unknown(&event.plan_id);
        let status = match event.gate_passed {
            Some(true) => "pass",
            Some(false) => "fail",
            None => "?",
        };
        latest = Some(format!(
            "{timestamp} model={model} task={task_id} plan={plan_id} {status} cost={}",
            display_cost_precise(event.cost_usd, event.input_tokens, event.output_tokens)
        ));
        events.push(event);
    }

    if count == 0 {
        println!("Efficiency: 0 entries at {}", path.display());
    } else {
        println!("Efficiency: {} events at {}", count, path.display());
    }
    println!("  Range: {}", format_range(first_seen, last_seen));
    println!("  Latest: {}", latest.unwrap_or_else(|| "none".to_string()));
    if let Some(summary) = attempt_correlation_summary(&events) {
        println!("{summary}");
    }
}

pub(crate) async fn print_learn_episodes(workdir: &std::path::Path) {
    let exact_path = learn_episodes_path(workdir);
    let path = roko_learn::runtime_feedback::resolve_project_episode_path(workdir);
    print_checked_path(&exact_path);
    if path != exact_path && path.exists() {
        println!("  legacy fallback: {}", path.display());
    }
    if !path.exists() {
        print_no_data(&path);
        return;
    }

    let Ok(text) = tokio::fs::read_to_string(&path).await else {
        println!("Episodes: 0 entries at {}", path.display());
        return;
    };

    let mut count = 0usize;
    let mut first_seen: Option<chrono::DateTime<chrono::Utc>> = None;
    let mut last_seen: Option<chrono::DateTime<chrono::Utc>> = None;
    let mut latest: Option<String> = None;

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let Ok(episode) = serde_json::from_str::<roko_learn::episode_logger::Episode>(trimmed)
        else {
            continue;
        };

        count += 1;
        first_seen = Some(match first_seen {
            Some(current) => current.min(episode.timestamp.clone()),
            None => episode.timestamp.clone(),
        });
        last_seen = Some(match last_seen {
            Some(current) => current.max(episode.timestamp.clone()),
            None => episode.timestamp.clone(),
        });

        let status = if episode.success { "pass" } else { "fail" };
        let model = non_empty_or_unknown(&episode.model);
        let task_id = non_empty_or_unknown(&episode.task_id);
        latest = Some(format!(
            "{} model={model} task={task_id} {status} cost={}",
            episode.timestamp.to_rfc3339(),
            display_cost_precise(
                episode.usage.cost_usd,
                episode.usage.input_tokens,
                episode.usage.output_tokens
            )
        ));
    }

    if count == 0 {
        println!("Episodes: 0 entries at {}", path.display());
    } else {
        println!("Episodes: {} entries at {}", count, path.display());
    }
    println!("  Range: {}", format_range(first_seen, last_seen));
    println!("  Latest: {}", latest.unwrap_or_else(|| "none".to_string()));
}

pub(crate) fn print_learn_gate_thresholds(workdir: &std::path::Path) {
    let path = learn_gate_thresholds_path(workdir);
    print_checked_path(&path);
    if !path.exists() {
        println!("Gate thresholds: 0 entries at {}", path.display());
        return;
    }
    let Ok(content) = std::fs::read_to_string(&path) else {
        println!("Gate thresholds: 0 entries at {}", path.display());
        return;
    };
    let count = count_gate_threshold_entries(&content);
    println!("Gate thresholds: {} entries at {}", count, path.display());
}

pub(crate) async fn print_learn_knowledge(workdir: &std::path::Path) {
    let path = learn_knowledge_path(workdir);
    print_checked_path(&path);
    if !path.exists() {
        print_no_data(&path);
        return;
    }
    let Ok(content) = tokio::fs::read_to_string(&path).await else {
        println!("Knowledge: 0 entries at {}", path.display());
        return;
    };
    let count = content
        .lines()
        .filter(|line| !line.trim().is_empty())
        .filter(|line| serde_json::from_str::<serde_json::Value>(line).is_ok())
        .count();
    if count == 0 {
        println!("Knowledge: 0 entries at {}", path.display());
    } else {
        println!("Knowledge: {} durable entries at {}", count, path.display());
    }
}

async fn cmd_learn_reflexes(workdir: &std::path::Path) -> Result<i32> {
    print_learn_reflexes(workdir);
    Ok(EXIT_SUCCESS)
}

const REFLEX_DISPLAY_LIMIT: usize = 5;

fn print_learn_reflexes(workdir: &std::path::Path) {
    let rules = reflex_store_snapshot(workdir);
    let demotions = recent_reflex_demotions(workdir);
    print!("{}", format_reflexes_human(&rules, &demotions));
}

fn reflex_store_snapshot(workdir: &std::path::Path) -> Vec<roko_learn::reflex_store::ReflexRule> {
    let path = learn_root(workdir).join("reflexes.jsonl");
    roko_learn::reflex_store::ReflexStore::open(path).snapshot()
}

fn recent_reflex_demotions(
    workdir: &std::path::Path,
) -> Vec<roko_learn::efficiency::AgentEfficiencyEvent> {
    let Ok(text) = std::fs::read_to_string(learn_efficiency_path(workdir)) else {
        return Vec::new();
    };
    text.lines()
        .rev()
        .filter_map(|line| serde_json::from_str(line.trim()).ok())
        .filter(|event: &roko_learn::efficiency::AgentEfficiencyEvent| {
            event.outcome == "reflex_demoted"
        })
        .take(REFLEX_DISPLAY_LIMIT)
        .collect()
}

fn format_reflexes_human(
    rules: &[roko_learn::reflex_store::ReflexRule],
    demotions: &[roko_learn::efficiency::AgentEfficiencyEvent],
) -> String {
    use std::fmt::Write as _;

    let mut output = String::new();
    let _ = writeln!(
        output,
        "T0 Reflex Store — {} rules (max {})",
        rules.len(),
        roko_learn::reflex_store::MAX_RULES,
    );

    if rules.is_empty() {
        let _ = writeln!(
            output,
            "  (no rules yet; run tasks to build reflex history)"
        );
    } else {
        let _ = writeln!(output, "\nTop rules by hit count:");
        for (index, rule) in rules.iter().take(REFLEX_DISPLAY_LIMIT).enumerate() {
            let _ = writeln!(
                output,
                "  {}. [{:.0}% conf, {} hits] {:?} → {} {}",
                index + 1,
                rule.confidence * 100.0,
                rule.hit_count,
                rule.condition.tool.as_deref().unwrap_or("*"),
                rule.action.tool,
                rule.action.args,
            );
        }
    }

    let _ = writeln!(output, "\nRecent demotions:");
    if demotions.is_empty() {
        let _ = writeln!(output, "  (none)");
    } else {
        for event in demotions {
            let _ = writeln!(
                output,
                "  {} plan={} task={} attempt={}",
                event.timestamp,
                non_empty_or_unknown(&event.plan_id),
                non_empty_or_unknown(&event.task_id),
                non_empty_or_unknown(&event.attempt_id),
            );
        }
    }
    output
}

fn learn_root(workdir: &std::path::Path) -> std::path::PathBuf {
    workdir.join(".roko").join("learn")
}

fn learn_gate_thresholds_path(workdir: &std::path::Path) -> std::path::PathBuf {
    learn_root(workdir).join("gate-thresholds.json")
}

fn learn_router_path(workdir: &std::path::Path) -> std::path::PathBuf {
    learn_root(workdir).join("cascade-router.json")
}

fn learn_efficiency_path(workdir: &std::path::Path) -> std::path::PathBuf {
    learn_root(workdir).join("efficiency.jsonl")
}

fn learn_episodes_path(workdir: &std::path::Path) -> std::path::PathBuf {
    workdir.join(".roko").join("episodes.jsonl")
}

fn learn_knowledge_path(workdir: &std::path::Path) -> std::path::PathBuf {
    workdir.join(".roko").join("neuro").join("knowledge.jsonl")
}

fn count_gate_threshold_entries(content: &str) -> usize {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(content) else {
        return 0;
    };

    value
        .get("rungs")
        .and_then(serde_json::Value::as_object)
        .map_or(0, |rungs| rungs.len())
}

fn print_checked_path(path: &std::path::Path) {
    println!("  path: {}", path.display());
}

fn print_no_data(path: &std::path::Path) {
    println!("No data at {}", path.display());
}

fn parse_rfc3339_utc(timestamp: &str) -> Option<chrono::DateTime<chrono::Utc>> {
    chrono::DateTime::parse_from_rfc3339(timestamp)
        .ok()
        .map(|parsed| parsed.with_timezone(&chrono::Utc))
}

fn format_range(
    first_seen: Option<chrono::DateTime<chrono::Utc>>,
    last_seen: Option<chrono::DateTime<chrono::Utc>>,
) -> String {
    match (first_seen, last_seen) {
        (Some(first_seen), Some(last_seen)) => {
            format!("{} .. {}", first_seen.to_rfc3339(), last_seen.to_rfc3339())
        }
        _ => "n/a".to_string(),
    }
}

fn non_empty_or_unknown(value: &str) -> &str {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        "unknown"
    } else {
        trimmed
    }
}

fn efficiency_model_label(event: &roko_learn::efficiency::AgentEfficiencyEvent) -> &str {
    let model_used = event.model_used.trim();
    if model_used.is_empty() {
        non_empty_or_unknown(&event.model)
    } else {
        model_used
    }
}

fn attempt_correlation_summary(
    events: &[roko_learn::efficiency::AgentEfficiencyEvent],
) -> Option<String> {
    let events_with_task_id = events
        .iter()
        .filter(|event| !event.task_id.is_empty())
        .count();
    if events_with_task_id == 0 {
        return None;
    }

    let linked_gate_failures = events
        .iter()
        .filter(|event| !event.task_id.is_empty() && event.gate_passed != Some(true))
        .count();

    Some(format!(
        "  Attempt correlation: {} events with task_id, {} gate failures linked",
        events_with_task_id, linked_gate_failures
    ))
}

fn cascade_stage_for_observations(observations: u64) -> &'static str {
    if observations >= 200 {
        "ucb"
    } else if observations >= 50 {
        "confidence"
    } else {
        "static"
    }
}

#[derive(Default, serde::Deserialize)]
struct LearnCascadeRouterSnapshot {
    #[serde(default)]
    model_slugs: Vec<String>,
    #[serde(default)]
    confidence_stats: std::collections::HashMap<String, LearnCascadeRouterModelStats>,
    #[serde(default)]
    total_observations: u64,
    #[serde(default)]
    stage_transitions: Vec<roko_learn::cascade::StageTransition>,
}

#[derive(Debug, Clone, Default, serde::Deserialize)]
struct LearnCascadeRouterModelStats {
    #[serde(default)]
    trials: u64,
    #[serde(default)]
    successes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct LearnCascadeRouterModelRow {
    slug: String,
    trials: u64,
    successes: u64,
    available: bool,
}

fn learn_router_model_rows(
    snapshot: &LearnCascadeRouterSnapshot,
    configured_slugs: &HashSet<String>,
) -> Vec<LearnCascadeRouterModelRow> {
    let mut slugs = Vec::new();
    let mut seen = HashSet::new();

    for slug in &snapshot.model_slugs {
        if seen.insert(slug.clone()) {
            slugs.push(slug.clone());
        }
    }
    for slug in snapshot.confidence_stats.keys() {
        if seen.insert(slug.clone()) {
            slugs.push(slug.clone());
        }
    }

    let mut rows = slugs
        .into_iter()
        .map(|slug| {
            let stats = snapshot.confidence_stats.get(&slug);
            let trials = stats.map_or(0, |entry| entry.trials);
            let successes = stats.map_or(0, |entry| entry.successes);
            let available = configured_slugs.contains(slug.as_str()) || successes > 0;
            LearnCascadeRouterModelRow {
                slug,
                trials,
                successes,
                available,
            }
        })
        .collect::<Vec<_>>();

    rows.sort_by(|left, right| left.slug.cmp(&right.slug));
    rows
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_cost_uses_unknown_for_zero_usage() {
        assert_eq!(display_cost(0.0, 0, 0), "unknown");
    }

    #[test]
    fn display_cost_shows_zero_for_reported_free_usage() {
        assert_eq!(display_cost(0.0, 1, 0), "$0.00");
        assert_eq!(display_cost(0.0, 0, 1), "$0.00");
    }

    #[test]
    fn display_cost_shows_formatted_value() {
        assert_eq!(display_cost(1.42, 0, 0), "$1.42");
    }

    #[test]
    fn display_cost_precise_uses_unknown_for_zero_usage() {
        assert_eq!(display_cost_precise(0.0, 0, 0), "unknown");
    }

    #[test]
    fn display_cost_precise_shows_zero_for_reported_free_usage() {
        assert_eq!(display_cost_precise(0.0, 2, 3), "$0.0000");
    }

    #[test]
    fn display_cost_precise_shows_formatted_value() {
        assert_eq!(display_cost_precise(1.42, 7, 9), "$1.4200");
    }

    #[test]
    fn attempt_correlation_summary_counts_only_attempted_events() {
        let mut success = roko_learn::efficiency::AgentEfficiencyEvent::default();
        success.task_id = "task-1".into();
        success.gate_passed = Some(true);

        let mut failure = roko_learn::efficiency::AgentEfficiencyEvent::default();
        failure.task_id = "task-2".into();
        failure.gate_passed = Some(false);

        let mut unlabeled = roko_learn::efficiency::AgentEfficiencyEvent::default();
        unlabeled.gate_passed = None;

        let events = vec![success, failure, unlabeled];
        let summary = attempt_correlation_summary(&events);

        assert_eq!(
            summary.as_deref(),
            Some("  Attempt correlation: 2 events with task_id, 1 gate failures linked")
        );
    }

    #[test]
    fn attempt_correlation_summary_skips_empty_attempt_ids() {
        let mut unlabeled = roko_learn::efficiency::AgentEfficiencyEvent::default();
        unlabeled.gate_passed = None;

        assert!(attempt_correlation_summary(&[unlabeled]).is_none());
    }

    #[test]
    fn learn_episodes_path_targets_root_log() {
        let workdir = std::path::Path::new("/tmp/workdir");
        assert_eq!(
            learn_episodes_path(workdir),
            workdir.join(".roko").join("episodes.jsonl")
        );
    }

    #[test]
    fn count_gate_threshold_entries_uses_rungs_map() {
        let content = r#"{"rungs":{"1":{"ema_pass_rate":0.5},"2":{"ema_pass_rate":0.75}}}"#;
        assert_eq!(count_gate_threshold_entries(content), 2);
    }

    #[test]
    fn learn_router_model_rows_mark_configured_and_successful_models_available() {
        let snapshot = LearnCascadeRouterSnapshot {
            model_slugs: vec!["configured".into(), "history".into()],
            confidence_stats: std::collections::HashMap::from([
                (
                    "configured".into(),
                    LearnCascadeRouterModelStats {
                        trials: 10,
                        successes: 0,
                    },
                ),
                (
                    "history".into(),
                    LearnCascadeRouterModelStats {
                        trials: 4,
                        successes: 2,
                    },
                ),
                (
                    "legacy".into(),
                    LearnCascadeRouterModelStats {
                        trials: 3,
                        successes: 0,
                    },
                ),
            ]),
            total_observations: 17,
            stage_transitions: Vec::new(),
        };
        let configured = ["configured".to_string()]
            .into_iter()
            .collect::<HashSet<_>>();

        let rows = learn_router_model_rows(&snapshot, &configured);
        let availability = rows
            .iter()
            .map(|row| (row.slug.as_str(), row.available))
            .collect::<std::collections::HashMap<_, _>>();

        assert_eq!(rows.len(), 3);
        assert!(availability["configured"]);
        assert!(availability["history"]);
        assert!(!availability["legacy"]);
    }

    // ── Inspect tests ──────────────────────────────────────────────

    #[test]
    fn inspect_gates_handles_missing_file() {
        let dir = tempfile::tempdir().unwrap();
        let workdir = dir.path();
        // No gate-thresholds.json exists; should succeed with "no data" message.
        let result = inspect_gates(workdir, false);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), EXIT_SUCCESS);
    }

    #[test]
    fn inspect_gates_json_handles_missing_file() {
        let dir = tempfile::tempdir().unwrap();
        let workdir = dir.path();
        let result = inspect_gates(workdir, true);
        assert!(result.is_ok());
    }

    #[test]
    fn inspect_gates_parses_threshold_file() {
        let dir = tempfile::tempdir().unwrap();
        let workdir = dir.path();
        let learn_dir = workdir.join(".roko").join("learn");
        std::fs::create_dir_all(&learn_dir).unwrap();
        std::fs::write(
            learn_dir.join("gate-thresholds.json"),
            r#"{"rungs":{"1":{"ema_pass_rate":0.85,"observation_count":12},"2":{"ema_pass_rate":0.70,"observation_count":5}}}"#,
        )
        .unwrap();

        let result = inspect_gates(workdir, false);
        assert!(result.is_ok());
    }

    #[test]
    fn inspect_gates_json_produces_valid_json() {
        let dir = tempfile::tempdir().unwrap();
        let workdir = dir.path();
        let learn_dir = workdir.join(".roko").join("learn");
        std::fs::create_dir_all(&learn_dir).unwrap();
        std::fs::write(
            learn_dir.join("gate-thresholds.json"),
            r#"{"rungs":{"1":{"ema_pass_rate":0.5}}}"#,
        )
        .unwrap();

        let result = inspect_gates(workdir, true);
        assert!(result.is_ok());
    }

    #[test]
    fn inspect_routing_handles_missing_file() {
        let dir = tempfile::tempdir().unwrap();
        let result = inspect_routing(dir.path(), false);
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn inspect_budget_handles_missing_files() {
        let dir = tempfile::tempdir().unwrap();
        let workdir = dir.path();
        // Create minimal roko.toml so config loads.
        std::fs::write(workdir.join("roko.toml"), "schema_version = 2\n").unwrap();
        let result = inspect_budget(workdir, false).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn inspect_budget_json_reports_configured_limits() {
        let dir = tempfile::tempdir().unwrap();
        let workdir = dir.path();
        std::fs::write(
            workdir.join("roko.toml"),
            "schema_version = 2\n[budget]\nmax_plan_usd = 25.0\nmax_turn_usd = 2.5\n",
        )
        .unwrap();
        let result = inspect_budget(workdir, true).await;
        assert!(result.is_ok());
    }

    #[test]
    fn inspect_gates_json_struct_serializes() {
        let output = InspectGatesJson {
            path: "/tmp/test".into(),
            rung_count: 2,
            rungs: serde_json::json!({"1": {"ema_pass_rate": 0.5}}),
        };
        let json = serde_json::to_string(&output).unwrap();
        assert!(json.contains("\"rung_count\":2"));
    }

    #[test]
    fn inspect_routing_json_struct_serializes() {
        let output = InspectRoutingJson {
            path: "/tmp/test".into(),
            total_observations: 100,
            stage: "confidence".into(),
            models: vec![],
        };
        let json = serde_json::to_string(&output).unwrap();
        assert!(json.contains("\"total_observations\":100"));
        assert!(json.contains("\"stage\":\"confidence\""));
    }

    #[test]
    fn inspect_budget_json_struct_serializes() {
        let output = InspectBudgetJson {
            config: InspectBudgetConfigJson {
                max_plan_usd: 25.0,
                max_task_usd: 0.0,
                max_turn_usd: 2.5,
                max_task_retry_usd: 0.0,
                prompt_token_budget: 10_000,
            },
            efficiency: InspectBudgetEfficiencyJson {
                path: "/tmp/test".into(),
                total_events: 42,
                passed: 30,
                failed: 12,
                total_cost_usd: 3.14,
                first_seen: None,
                last_seen: None,
            },
        };
        let json = serde_json::to_string(&output).unwrap();
        assert!(json.contains("\"max_plan_usd\":25.0"));
        assert!(json.contains("\"total_events\":42"));
        assert!(json.contains("\"passed\":30"));
    }
}
