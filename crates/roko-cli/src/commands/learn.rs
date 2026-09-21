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
        | LearnCmd::Efficiency { workdir, .. }
        | LearnCmd::Episodes { workdir, .. }
        | LearnCmd::Reflexes { workdir }
        | LearnCmd::Gates { workdir }
        | LearnCmd::KnowledgeStats { workdir }
        | LearnCmd::Playbooks { workdir }
        | LearnCmd::Sections { workdir }
        | LearnCmd::Reflections { workdir, .. }
        | LearnCmd::Tools { workdir }
        | LearnCmd::FeedbackProof { workdir }
        | LearnCmd::RoleCosts { workdir }
        | LearnCmd::Graduation { workdir } => {
            workdir.clone().unwrap_or_else(|| resolve_workdir(cli))
        }
        LearnCmd::Experiments { workdir, cmd: sub } => {
            // Use workdir from outer flag, or from the inner subcommand, or cwd.
            let outer = workdir.clone();
            let inner = sub.as_ref().and_then(|s| match s {
                ExperimentsSubCmd::List { workdir, .. }
                | ExperimentsSubCmd::Create { workdir, .. }
                | ExperimentsSubCmd::Conclude { workdir, .. }
                | ExperimentsSubCmd::Report { workdir, .. } => workdir.clone(),
            });
            outer.or(inner).unwrap_or_else(|| resolve_workdir(cli))
        }
        LearnCmd::Inspect { subsystem } => inspect_workdir(cli, subsystem),
        LearnCmd::Tune { workdir, .. } => workdir.clone().unwrap_or_else(|| resolve_workdir(cli)),
    };
    let _lock =
        roko_cli::workspace_lock::acquire_workspace_lock_shared(&wd_for_lock.join(".roko"))?;

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
        LearnCmd::Experiments { workdir, cmd: sub } => {
            let outer_wd = workdir;
            match sub {
                // No subcommand → default to list (backward-compatible).
                None => {
                    let wd = outer_wd.unwrap_or_else(|| resolve_workdir(cli));
                    if json {
                        cmd_learn_json(&wd, "experiments").await
                    } else {
                        cmd_experiments_list(&wd, None).await
                    }
                }
                Some(ExperimentsSubCmd::List { workdir, limit }) => {
                    let wd = outer_wd.or(workdir).unwrap_or_else(|| resolve_workdir(cli));
                    cmd_experiments_list(&wd, limit).await
                }
                Some(ExperimentsSubCmd::Create {
                    workdir,
                    name,
                    section,
                    variants,
                }) => {
                    let wd = outer_wd.or(workdir).unwrap_or_else(|| resolve_workdir(cli));
                    cmd_experiments_create(&wd, &name, &section, &variants)
                }
                Some(ExperimentsSubCmd::Conclude { workdir, name }) => {
                    let wd = outer_wd.or(workdir).unwrap_or_else(|| resolve_workdir(cli));
                    cmd_experiments_conclude(&wd, &name)
                }
                Some(ExperimentsSubCmd::Report { workdir, name }) => {
                    let wd = outer_wd.or(workdir).unwrap_or_else(|| resolve_workdir(cli));
                    cmd_experiments_report(&wd, &name)
                }
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
            // Route directly to inspect_gates so the output includes
            // per-rung EMA pass rates and observation counts, not just
            // the entry count printed by print_learn_gate_thresholds.
            inspect_gates(&wd, json)
        }
        LearnCmd::KnowledgeStats { workdir } => {
            let wd = workdir.unwrap_or_else(|| resolve_workdir(cli));
            if json {
                cmd_learn_json(&wd, "knowledge").await
            } else {
                cmd_learn(&wd, "knowledge").await
            }
        }
        LearnCmd::Playbooks { workdir } => {
            let wd = workdir.unwrap_or_else(|| resolve_workdir(cli));
            cmd_learn_playbooks(&wd).await
        }
        LearnCmd::Sections { workdir } => {
            let wd = workdir.unwrap_or_else(|| resolve_workdir(cli));
            cmd_learn_sections(&wd).await
        }
        LearnCmd::Reflections { workdir, limit } => {
            let wd = workdir.unwrap_or_else(|| resolve_workdir(cli));
            cmd_learn_reflections(&wd, limit).await
        }
        LearnCmd::Tools { workdir } => {
            let wd = workdir.unwrap_or_else(|| resolve_workdir(cli));
            cmd_learn_tools(&wd, json).await
        }
        LearnCmd::FeedbackProof { workdir } => {
            let wd = workdir.unwrap_or_else(|| resolve_workdir(cli));
            cmd_learn_feedback_proof(&wd, json).await
        }
        LearnCmd::RoleCosts { workdir } => {
            let wd = workdir.unwrap_or_else(|| resolve_workdir(cli));
            cmd_learn_role_costs(&wd, json).await
        }
        LearnCmd::Graduation { workdir } => {
            let wd = workdir.unwrap_or_else(|| resolve_workdir(cli));
            cmd_learn_graduation(&wd, json).await
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
            tracing::warn!(%subsystem, "'roko learn tune' is deprecated; use 'roko learn inspect <subsystem>'");
            let wd = workdir.unwrap_or_else(|| resolve_workdir(cli));
            if dry_run {
                tracing::info!("note: inspection is always read-only; --dry-run has no effect");
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
        other => anyhow::bail!("unknown subsystem '{other}'. Available: gates, routing, budget"),
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

    // Audit #80: load from disk if present, then fill in defaults for all 7
    // canonical rungs so the display always shows a complete picture.
    let mut gate_thresholds = if path.exists() {
        let content = std::fs::read_to_string(&path)?;
        serde_json::from_str::<roko_cli::runner::persist::GateThresholds>(&content)
            .unwrap_or_default()
    } else {
        roko_cli::runner::persist::GateThresholds::default()
    };
    gate_thresholds.fill_default_rungs();

    let rung_count = gate_thresholds.rungs.len();
    let file_exists = path.exists();

    // Serialize the filled thresholds back to a JSON value for display.
    let rungs_json = serde_json::to_value(&gate_thresholds.rungs)?;

    if json {
        let output = InspectGatesJson {
            path: path.display().to_string(),
            rung_count,
            rungs: rungs_json,
        };
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        let reporter = roko_cli::cli_reporter::CliReporter::new(false, false);
        if file_exists {
            reporter.step("Adaptive gate thresholds", &path.display().to_string());
        } else {
            reporter.step(
                "Adaptive gate thresholds",
                "defaults — no observations yet; file will be created on first plan run",
            );
            reporter.note(&format!("  ({})", path.display()));
        }
        reporter.note(&format!("  Rungs: {rung_count} (all canonical rungs shown)"));
        // Sort by rung index for stable output then emit as a table.
        let mut rung_pairs: Vec<(u32, &roko_cli::runner::persist::GateThresholdStats)> =
            gate_thresholds.rungs.iter().map(|(k, v)| (*k, v)).collect();
        rung_pairs.sort_by_key(|(idx, _)| *idx);
        let rows: Vec<Vec<String>> = rung_pairs
            .iter()
            .map(|(rung_idx, stats)| {
                let display_name = roko_gate::Rung::from_index(*rung_idx)
                    .map(|r| r.label())
                    .unwrap_or("unknown");
                let obs_label = if stats.total_count == 0 {
                    "0 (default prior)".to_string()
                } else {
                    stats.total_count.to_string()
                };
                vec![
                    display_name.to_string(),
                    rung_idx.to_string(),
                    format!("{:.2}", stats.ema_pass_rate),
                    obs_label,
                ]
            })
            .collect();
        reporter.table(&["RUNG", "IDX", "EMA PASS RATE", "OBSERVATIONS"], &rows);
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
    let snapshot = serde_json::from_str::<LearnCascadeRouterSnapshot>(&content).unwrap_or_default();
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
    max_daily_usd: f32,
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
    let text = tokio::fs::read_to_string(&eff_path)
        .await
        .unwrap_or_default();

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
                max_daily_usd: budget.max_daily_usd,
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
        println!("  max_daily_usd:      {}", fmt_limit(budget.max_daily_usd));
        println!("  prompt_token_budget: {}", budget.prompt_token_budget);
        println!();
        println!("Spend history ({})", eff_path.display());
        println!("  Events: {total_events} ({passed} passed, {failed} failed)");
        println!("  Total cost: ${total_cost:.4}");
        println!("  Range: {}", format_range(first_seen, last_seen));
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

    if show_all || what == "gates" {
        print_learn_gate_thresholds(workdir);
    }

    if show_all {
        print_learn_knowledge(workdir).await;
    }

    if !show_all
        && ![
            "router",
            "experiments",
            "efficiency",
            "episodes",
            "reflexes",
            "gates",
        ]
        .contains(&what)
    {
        anyhow::bail!(
            "unknown learning area '{what}'. Available: router, experiments, efficiency, episodes, reflexes, gates, all"
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

    let gate_thresholds = if show_all || what == "gates" {
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
            "gates",
        ]
        .contains(&what)
    {
        anyhow::bail!(
            "unknown learning area '{what}'. Available: router, experiments, efficiency, episodes, reflexes, gates, all"
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

// ── Experiments subcommands ──────────────────────────────────────────

/// `roko learn experiments list` — tabular view of all prompt experiments.
#[allow(clippy::cast_precision_loss)]
async fn cmd_experiments_list(workdir: &std::path::Path, limit: Option<u32>) -> Result<i32> {
    let prompt_path = learn_root(workdir).join("experiments.json");
    let store = ExperimentStore::load_or_new(&prompt_path);

    let mut experiments: Vec<_> = store.iter().collect();
    // Sort by experiment_id for deterministic output.
    experiments.sort_by(|a, b| a.experiment_id.cmp(&b.experiment_id));
    if let Some(n) = limit {
        experiments.truncate(n as usize);
    }

    if experiments.is_empty() {
        println!("No experiments found at {}", prompt_path.display());
        return Ok(EXIT_SUCCESS);
    }

    // Column widths — derive from data.
    let name_w = experiments
        .iter()
        .map(|e| e.experiment_id.len())
        .max()
        .unwrap_or(4)
        .max(4);
    let section_w = experiments
        .iter()
        .map(|e| e.section_name.len())
        .max()
        .unwrap_or(7)
        .max(7);

    println!(
        "{:<name_w$}  {:<section_w$}  {:>8}  {:>12}  {:>12}  {:>8}  {:>10}",
        "Name",
        "Section",
        "Status",
        "Variants",
        "Observations",
        "Best",
        "Win Rate",
        name_w = name_w,
        section_w = section_w,
    );
    println!(
        "{:-<name_w$}  {:-<section_w$}  {:->8}  {:->12}  {:->12}  {:->8}  {:->10}",
        "",
        "",
        "",
        "",
        "",
        "",
        "",
        name_w = name_w,
        section_w = section_w,
    );

    for exp in &experiments {
        let status = match exp.status {
            roko_learn::prompt_experiment::ExperimentStatus::Running => "running",
            roko_learn::prompt_experiment::ExperimentStatus::Concluded => "concluded",
        };
        let variants_count = exp.variants.len();
        let total_obs: u64 = exp.stats.values().map(|s| s.trials).sum();
        let best_variant = exp
            .winner_id
            .as_deref()
            .or_else(|| {
                exp.stats
                    .iter()
                    .max_by(|a, b| {
                        a.1.success_rate()
                            .partial_cmp(&b.1.success_rate())
                            .unwrap_or(std::cmp::Ordering::Equal)
                    })
                    .map(|(id, _)| id.as_str())
            })
            .unwrap_or("-");
        let win_rate = exp
            .winner_id
            .as_deref()
            .and_then(|id| exp.stats.get(id))
            .map(|s| format!("{:.1}%", s.success_rate() * 100.0))
            .unwrap_or_else(|| "-".to_string());

        println!(
            "{:<name_w$}  {:<section_w$}  {:>8}  {:>12}  {:>12}  {:>8}  {:>10}",
            exp.experiment_id,
            exp.section_name,
            status,
            variants_count,
            total_obs,
            best_variant,
            win_rate,
            name_w = name_w,
            section_w = section_w,
        );
    }

    Ok(EXIT_SUCCESS)
}

/// `roko learn experiments create` — register a new prompt experiment.
fn cmd_experiments_create(
    workdir: &std::path::Path,
    name: &str,
    section: &str,
    variants_csv: &str,
) -> Result<i32> {
    use roko_learn::prompt_experiment::{PromptExperiment, PromptVariant};

    let variant_ids: Vec<&str> = variants_csv.split(',').map(str::trim).collect();
    if variant_ids.len() < 2 {
        anyhow::bail!("at least two comma-separated variants are required (got: {variants_csv:?})");
    }
    if variant_ids.iter().any(|id| id.is_empty()) {
        anyhow::bail!("variant ids must not be empty (got: {variants_csv:?})");
    }

    let variants: Vec<PromptVariant> = variant_ids
        .iter()
        .map(|id| PromptVariant {
            id: id.to_string(),
            name: id.to_string(),
            section_name: section.to_string(),
            content: String::new(),
            slug: None,
            active: true,
        })
        .collect();

    let experiment = PromptExperiment::new(name, section, variants);

    let prompt_path = learn_root(workdir).join("experiments.json");

    // Ensure parent directory exists.
    if let Some(parent) = prompt_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    ExperimentStore::transaction(&prompt_path, |store| {
        if store.get(name).is_some() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                format!("experiment '{name}' already exists"),
            ));
        }
        store.register(experiment.clone());
        Ok(())
    })
    .map_err(|e| anyhow::anyhow!("{e}"))?;

    println!(
        "Created experiment '{}' for section '{}' with variants: {}",
        name,
        section,
        variant_ids.join(", ")
    );

    Ok(EXIT_SUCCESS)
}

/// `roko learn experiments conclude` — force-conclude an experiment by picking the best variant.
fn cmd_experiments_conclude(workdir: &std::path::Path, name: &str) -> Result<i32> {
    let prompt_path = learn_root(workdir).join("experiments.json");

    let winner_id = ExperimentStore::transaction(&prompt_path, |store| store.force_conclude(name))
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    println!("Concluded experiment '{name}' — winner: {winner_id}");

    Ok(EXIT_SUCCESS)
}

/// `roko learn experiments report` — detailed statistical report for one experiment.
#[allow(clippy::cast_precision_loss)]
fn cmd_experiments_report(workdir: &std::path::Path, name: &str) -> Result<i32> {
    use roko_learn::prompt_experiment::{ExperimentStatus, chi_squared_test};

    let prompt_path = learn_root(workdir).join("experiments.json");
    let store = ExperimentStore::load_or_new(&prompt_path);

    let exp = store.get(name).ok_or_else(|| {
        anyhow::anyhow!("experiment '{name}' not found at {}", prompt_path.display())
    })?;

    println!("Experiment: {}", exp.experiment_id);
    println!("  Section:  {}", exp.section_name);
    println!(
        "  Status:   {:?}",
        match exp.status {
            ExperimentStatus::Running => "running",
            ExperimentStatus::Concluded => "concluded",
        }
    );
    if let Some(winner) = &exp.winner_id {
        println!("  Winner:   {winner}");
    }

    let total_trials: u64 = exp.stats.values().map(|s| s.trials).sum();
    println!("  Total observations: {total_trials}");
    println!();

    // Sort variants by success rate descending.
    let mut rows: Vec<_> = exp
        .variants
        .iter()
        .map(|v| {
            let stats = exp.stats.get(&v.id).cloned().unwrap_or_default();
            (v, stats)
        })
        .collect();
    rows.sort_by(|a, b| {
        b.1.success_rate()
            .partial_cmp(&a.1.success_rate())
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // Header
    println!(
        "  {:<20}  {:>8}  {:>9}  {:>8}  {:>18}",
        "Variant", "Trials", "Successes", "Win Rate", "95% CI (Wilson)"
    );
    println!(
        "  {:-<20}  {:->8}  {:->9}  {:->8}  {:->18}",
        "", "", "", "", ""
    );

    for (variant, stats) in &rows {
        let rate = stats.success_rate();
        let (ci_lo, ci_hi) = {
            if stats.trials == 0 {
                (0.0_f64, 0.0_f64)
            } else {
                let n = stats.trials as f64;
                let p = rate;
                let z = 1.96_f64;
                let z_sq = z * z;
                let denom = 1.0 + z_sq / n;
                let center = (p + z_sq / (2.0 * n)) / denom;
                let margin = (z / denom) * ((p * (1.0 - p) / n + z_sq / (4.0 * n * n)).sqrt());
                (
                    (center - margin).clamp(0.0, 1.0),
                    (center + margin).clamp(0.0, 1.0),
                )
            }
        };
        let winner_marker = if exp.winner_id.as_deref() == Some(&variant.id) {
            " *"
        } else {
            ""
        };
        println!(
            "  {:<20}  {:>8}  {:>9}  {:>7.1}%  [{:.3}, {:.3}]{}",
            variant.id,
            stats.trials,
            stats.successes,
            rate * 100.0,
            ci_lo,
            ci_hi,
            winner_marker,
        );
    }

    // Chi-squared test between the top two variants, if enough data.
    if rows.len() >= 2 && total_trials >= 10 {
        println!();
        let (v1, s1) = &rows[0];
        let (v2, s2) = &rows[1];
        let (chi_sq, p_value) = chi_squared_test(s1, s2);
        let effect_size = (s1.success_rate() - s2.success_rate()).abs();
        // Cohen's h for two proportions.
        let cohens_h =
            2.0 * (s1.success_rate().sqrt().asin() - s2.success_rate().sqrt().asin()).abs();
        println!("  Chi-squared test ({} vs {}):", v1.id, v2.id);
        println!("    statistic = {chi_sq:.4}");
        println!(
            "    p-value   = {p_value:.4} {}",
            if p_value < 0.05 {
                "(significant at alpha=0.05)"
            } else {
                "(not significant)"
            }
        );
        println!("    effect size (|Δ win rate|) = {effect_size:.4}");
        println!("    Cohen's h = {cohens_h:.4}");
    } else if rows.len() < 2 {
        println!();
        println!("  (only one variant — chi-squared test not applicable)");
    } else {
        println!();
        println!("  (fewer than 10 total observations — statistical test not yet reliable)");
    }

    // Archived stats, if concluded.
    if let Some(archive) = &exp.archive {
        println!();
        println!("  Concluded at: {}", archive.concluded_at.to_rfc3339());
        println!("  Archive p-value:    {:.4}", archive.p_value);
        println!("  Archive effect size: {:.4}", archive.effect_size);
    }

    Ok(EXIT_SUCCESS)
}

#[allow(clippy::cast_precision_loss)]
pub(crate) async fn print_learn_efficiency(workdir: &std::path::Path) {
    let path = learn_efficiency_path(workdir);
    print_checked_path(&path);
    if !path.exists() {
        print_no_data(&path);
        return;
    }

    // Audit #80 Step 6: one-shot migration — rewrite pre-P0-GA-1 entries
    // that erroneously stored gate_passed=false for successful tasks.
    // Safe to call repeatedly; already-patched entries are left unchanged.
    match roko_learn::efficiency::null_ambiguous_gate_failed_entries(&path) {
        Ok(0) => {}
        Ok(n) => tracing::info!(
            patched = n,
            "efficiency.jsonl: nulled {n} ambiguous gate_passed=false entries (audit #80)"
        ),
        Err(err) => tracing::warn!(
            error = %err,
            "efficiency.jsonl: gate_passed migration failed (non-fatal)"
        ),
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

fn learn_playbooks_dir(workdir: &std::path::Path) -> std::path::PathBuf {
    learn_root(workdir).join("playbooks")
}

fn learn_section_outcomes_path(workdir: &std::path::Path) -> std::path::PathBuf {
    learn_root(workdir).join("section-outcomes.jsonl")
}

fn learn_post_gate_reflections_path(workdir: &std::path::Path) -> std::path::PathBuf {
    learn_root(workdir).join("post-gate-reflections.json")
}

// ── P2-11: Playbooks ─────────────────────────────────────────────────

/// `roko learn playbooks` — list all learned playbooks.
async fn cmd_learn_playbooks(workdir: &std::path::Path) -> Result<i32> {
    let dir = learn_playbooks_dir(workdir);
    let store = roko_learn::playbook::PlaybookStore::new(&dir);
    let mut playbooks = store.list().await.unwrap_or_default();

    if playbooks.is_empty() {
        println!("Playbook store: empty ({})", dir.display());
        return Ok(EXIT_SUCCESS);
    }

    // Sort by success rate descending, then name ascending for determinism.
    playbooks.sort_by(|a, b| {
        let ra = a.success_rate().unwrap_or(0.0);
        let rb = b.success_rate().unwrap_or(0.0);
        rb.total_cmp(&ra).then_with(|| a.name.cmp(&b.name))
    });

    let name_w = playbooks
        .iter()
        .map(|pb| pb.name.len().min(40))
        .max()
        .unwrap_or(4)
        .max(4);
    let pattern_w = playbooks
        .iter()
        .map(|pb| pb.when_pattern.as_deref().unwrap_or("-").len().min(30))
        .max()
        .unwrap_or(7)
        .max(7);

    println!(
        "Playbook store: {} entries ({})",
        playbooks.len(),
        dir.display()
    );
    println!();
    println!(
        "{:<name_w$}  {:<pattern_w$}  {:>4}  {:>4}  {:>8}  {:>4}",
        "Name",
        "Trigger",
        "Succ",
        "Fail",
        "Rate",
        "Deps",
        name_w = name_w,
        pattern_w = pattern_w,
    );
    println!(
        "{:-<name_w$}  {:-<pattern_w$}  {:->4}  {:->4}  {:->8}  {:->4}",
        "",
        "",
        "",
        "",
        "",
        "",
        name_w = name_w,
        pattern_w = pattern_w,
    );

    for pb in &playbooks {
        let name = truncate_str(&pb.name, 40);
        let pattern = truncate_str(pb.when_pattern.as_deref().unwrap_or("-"), 30);
        let rate = pb
            .success_rate()
            .map(|r| format!("{:.0}%", r * 100.0))
            .unwrap_or_else(|| "-".to_string());
        let steps = pb.steps.len();
        println!(
            "{:<name_w$}  {:<pattern_w$}  {:>4}  {:>4}  {:>8}  {:>4}",
            name,
            pattern,
            pb.success_count,
            pb.failure_count,
            rate,
            steps,
            name_w = name_w,
            pattern_w = pattern_w,
        );
    }

    Ok(EXIT_SUCCESS)
}

// ── P2-12: Sections ──────────────────────────────────────────────────

/// `roko learn sections` — show per-section pass rates (worst first).
async fn cmd_learn_sections(workdir: &std::path::Path) -> Result<i32> {
    let path = learn_section_outcomes_path(workdir);
    let text = tokio::fs::read_to_string(&path).await.unwrap_or_default();

    // Accumulate (passed, total, total_cost) per section name.
    let mut stats: std::collections::HashMap<String, (u64, u64, f64)> =
        std::collections::HashMap::new();

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let Ok(record) =
            serde_json::from_str::<roko_learn::section_outcome::SectionOutcomeRecord>(trimmed)
        else {
            continue;
        };

        let entry = stats.entry(record.section_name.clone()).or_default();
        entry.1 += 1; // total trials
        if record.status == roko_learn::section_outcome::SectionOutcomeStatus::Passed {
            entry.0 += 1; // passed
        }
        // Cost is not directly on the record; skip for now.
    }

    if stats.is_empty() {
        println!("Section outcomes: no data at {}", path.display());
        return Ok(EXIT_SUCCESS);
    }

    // Sort by pass rate ascending (worst first), then name for determinism.
    let mut rows: Vec<(String, u64, u64)> = stats
        .into_iter()
        .map(|(name, (passed, total, _cost))| (name, passed, total))
        .collect();
    rows.sort_by(|(na, pa, ta), (nb, pb, tb)| {
        let ra = if *ta == 0 {
            0.0f64
        } else {
            *pa as f64 / *ta as f64
        };
        let rb = if *tb == 0 {
            0.0f64
        } else {
            *pb as f64 / *tb as f64
        };
        ra.total_cmp(&rb).then_with(|| na.cmp(nb))
    });

    let name_w = rows
        .iter()
        .map(|(n, _, _)| n.len().min(50))
        .max()
        .unwrap_or(7)
        .max(7);

    println!(
        "Section outcomes: {} sections ({})",
        rows.len(),
        path.display()
    );
    println!();
    println!(
        "{:<name_w$}  {:>6}  {:>9}",
        "Section Name",
        "Trials",
        "Pass Rate",
        name_w = name_w,
    );
    println!("{:-<name_w$}  {:->6}  {:->9}", "", "", "", name_w = name_w);
    #[allow(clippy::cast_precision_loss)]
    for (name, passed, total) in &rows {
        let rate = if *total == 0 {
            "-".to_string()
        } else {
            format!("{:.1}%", (*passed as f64 / *total as f64) * 100.0)
        };
        let display_name = truncate_str(name, 50);
        println!(
            "{:<name_w$}  {:>6}  {:>9}",
            display_name,
            total,
            rate,
            name_w = name_w,
        );
    }

    Ok(EXIT_SUCCESS)
}

// ── P2-13: Reflections ───────────────────────────────────────────────

/// `roko learn reflections` — show recent post-gate reflections.
fn cmd_learn_reflections(
    workdir: &std::path::Path,
    limit: usize,
) -> impl std::future::Future<Output = Result<i32>> {
    let workdir = workdir.to_path_buf();
    async move {
        let path = learn_post_gate_reflections_path(&workdir);
        let store = roko_learn::post_gate_reflection::PostGateReflectionStore::load(&path);
        let records = &store.records;

        if records.is_empty() {
            println!("Post-gate reflections: no data at {}", path.display());
            return Ok(EXIT_SUCCESS);
        }

        // Show the most recent `limit` records (records are in append order).
        let display_records: Vec<_> = records.iter().rev().take(limit).collect();

        println!(
            "Post-gate reflections: {} total, showing {} ({})",
            records.len(),
            display_records.len(),
            path.display()
        );
        println!();

        for record in display_records {
            let ts = record.created_at.format("%Y-%m-%d %H:%M:%S").to_string();
            let task = record.task_id.as_deref().unwrap_or("-");
            let verdict = match record.outcome {
                roko_learn::post_gate_reflection::ReflectionGateOutcome::Passed => "pass",
                roko_learn::post_gate_reflection::ReflectionGateOutcome::Failed => "fail",
            };
            let lesson = truncate_str(&record.proposed_lesson, 120);
            println!(
                "  [{}] gate={} verdict={} task={}",
                ts, record.trigger_gate, verdict, task
            );
            println!("    {}", lesson);
            println!();
        }

        Ok(EXIT_SUCCESS)
    }
}

// ── Tool usage statistics ──────────────────────────────────────────────

/// `roko learn tools` — show tool usage statistics from the tool audit log.
#[allow(clippy::cast_precision_loss)]
async fn cmd_learn_tools(workdir: &std::path::Path, json: bool) -> Result<i32> {
    let path = workdir.join(".roko").join("tool_audit.jsonl");
    if !path.exists() {
        if json {
            println!("{{\"tools\":[]}}");
        } else {
            println!("Tool audit: no data at {}", path.display());
        }
        return Ok(EXIT_SUCCESS);
    }

    let content = tokio::fs::read_to_string(&path).await.unwrap_or_default();

    // Accumulate per-tool stats from audit lines.
    let mut stats: std::collections::HashMap<String, ToolStats> = std::collections::HashMap::new();

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Ok(entry) = serde_json::from_str::<roko_fs::tool_audit::AuditLine>(line) else {
            continue;
        };
        match entry {
            roko_fs::tool_audit::AuditLine::Admit {
                call_name, ts_ms, ..
            } => {
                let tool = stats.entry(call_name).or_default();
                tool.calls += 1;
                if ts_ms > tool.last_seen_ms {
                    tool.last_seen_ms = ts_ms;
                }
            }
            roko_fs::tool_audit::AuditLine::Result {
                call_name,
                ok,
                ts_ms,
                ..
            } => {
                let tool = stats.entry(call_name).or_default();
                tool.results += 1;
                if ok {
                    tool.successes += 1;
                }
                if ts_ms > tool.last_seen_ms {
                    tool.last_seen_ms = ts_ms;
                }
            }
        }
    }

    // Sort by call count descending.
    let mut rows: Vec<_> = stats.into_iter().collect();
    rows.sort_by_key(|b| std::cmp::Reverse(b.1.calls));

    if json {
        let json_rows: Vec<serde_json::Value> = rows
            .iter()
            .map(|(name, s)| {
                let rate = if s.results > 0 {
                    s.successes as f64 / s.results as f64
                } else {
                    0.0
                };
                serde_json::json!({
                    "name": name,
                    "calls": s.calls,
                    "results": s.results,
                    "successes": s.successes,
                    "success_rate": (rate * 100.0).round() / 100.0,
                })
            })
            .collect();
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({ "tools": json_rows }))?
        );
    } else {
        println!("Tool usage statistics ({})", path.display());
        println!();
        if rows.is_empty() {
            println!("  (no tool calls recorded)");
        } else {
            println!(
                "  {:<30} {:>6} {:>8} {:>8} {:>10}",
                "Tool", "Calls", "Results", "Success", "Rate"
            );
            println!("  {}", "-".repeat(66));
            for (name, s) in &rows {
                let rate = if s.results > 0 {
                    format!("{:.0}%", s.successes as f64 / s.results as f64 * 100.0)
                } else {
                    "n/a".to_string()
                };
                println!(
                    "  {:<30} {:>6} {:>8} {:>8} {:>10}",
                    truncate_str(name, 30),
                    s.calls,
                    s.results,
                    s.successes,
                    rate
                );
            }
            println!();
            let total_calls: u64 = rows.iter().map(|(_, s)| s.calls).sum();
            let total_results: u64 = rows.iter().map(|(_, s)| s.results).sum();
            let total_successes: u64 = rows.iter().map(|(_, s)| s.successes).sum();
            let overall_rate = if total_results > 0 {
                format!(
                    "{:.0}%",
                    total_successes as f64 / total_results as f64 * 100.0
                )
            } else {
                "n/a".to_string()
            };
            println!(
                "  {:<30} {:>6} {:>8} {:>8} {:>10}",
                "TOTAL", total_calls, total_results, total_successes, overall_rate
            );
        }
    }

    Ok(EXIT_SUCCESS)
}

#[derive(Default)]
struct ToolStats {
    calls: u64,
    results: u64,
    successes: u64,
    last_seen_ms: i64,
}

// ── Shared helpers ────────────────────────────────────────────────────

/// Truncate a string to at most `max_chars` characters, appending `...` when cut.
fn truncate_str(text: &str, max_chars: usize) -> String {
    let text = text.trim();
    if max_chars == 0 {
        return String::new();
    }
    let chars: Vec<char> = text.chars().collect();
    if chars.len() <= max_chars {
        chars.into_iter().collect()
    } else {
        let truncated: String = chars[..max_chars.saturating_sub(3)].iter().collect();
        format!("{truncated}...")
    }
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

// ── P2-35: End-to-end feedback loop proof query ──────────────────────

/// `roko learn feedback-proof` -- trace a closed feedback loop.
#[allow(clippy::cast_precision_loss)]
async fn cmd_learn_feedback_proof(workdir: &std::path::Path, json: bool) -> Result<i32> {
    // Read episodes for knowledge_ids_injected.
    let episodes_path = learn_episodes_path(workdir);
    let episodes_text = tokio::fs::read_to_string(&episodes_path)
        .await
        .unwrap_or_default();

    let mut injected_ids: HashSet<String> = HashSet::new();
    let mut passed_with_knowledge: Vec<(String, Vec<String>)> = Vec::new();

    for line in episodes_text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let Ok(episode) = serde_json::from_str::<serde_json::Value>(trimmed) else {
            continue;
        };
        let ids: Vec<String> = episode
            .get("knowledge_ids_injected")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default();
        if ids.is_empty() {
            continue;
        }
        for id in &ids {
            injected_ids.insert(id.clone());
        }
        let success = episode
            .get("success")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        if success {
            let task_id = episode
                .get("task_id")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string();
            passed_with_knowledge.push((task_id, ids));
        }
    }

    // Read knowledge store for confirmation counts.
    let knowledge_path = learn_knowledge_path(workdir);
    let knowledge_text = tokio::fs::read_to_string(&knowledge_path)
        .await
        .unwrap_or_default();

    let mut confirmed_ids: HashSet<String> = HashSet::new();
    for line in knowledge_text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let Ok(entry) = serde_json::from_str::<serde_json::Value>(trimmed) else {
            continue;
        };
        let id = entry.get("id").and_then(|v| v.as_str()).unwrap_or_default();
        let confirmations = entry
            .get("confirmation_count")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        if confirmations > 0 && injected_ids.contains(id) {
            confirmed_ids.insert(id.to_string());
        }
    }

    // Build closed-loop chains.
    let mut closed_loops: Vec<serde_json::Value> = Vec::new();
    for (task_id, ids) in &passed_with_knowledge {
        for id in ids {
            if confirmed_ids.contains(id) {
                closed_loops.push(serde_json::json!({
                    "knowledge_id": id,
                    "task_id": task_id,
                    "chain": "ingested -> injected -> gate_pass -> confirmed",
                }));
            }
        }
    }

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "injected_knowledge_ids": injected_ids.len(),
                "passed_with_knowledge": passed_with_knowledge.len(),
                "confirmed_ids": confirmed_ids.len(),
                "closed_loops": closed_loops.len(),
                "loops": closed_loops,
            }))?
        );
    } else {
        println!("Feedback loop proof");
        println!(
            "  Knowledge IDs injected into prompts: {}",
            injected_ids.len()
        );
        println!(
            "  Tasks passed with knowledge: {}",
            passed_with_knowledge.len()
        );
        println!(
            "  Confirmed knowledge entries used: {}",
            confirmed_ids.len()
        );
        println!("  Closed loops found: {}", closed_loops.len());
        if closed_loops.is_empty() {
            println!();
            println!("  No closed feedback loop found yet.");
            println!("  A closed loop requires: knowledge entry ingested from an episode,");
            println!(
                "  injected into a prompt, contributed to a passing gate, and received confirmation."
            );
        } else {
            println!();
            for loop_entry in &closed_loops {
                let kid = loop_entry
                    .get("knowledge_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("?");
                let tid = loop_entry
                    .get("task_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("?");
                println!("  * {kid} -> task {tid}: ingested -> injected -> gate_pass -> confirmed");
            }
        }
    }

    Ok(EXIT_SUCCESS)
}

// ── P2-17: Per-role cost profiles ────────────────────────────────────

/// `roko learn role-costs` -- show per-role cost profiles from efficiency events.
#[allow(clippy::cast_precision_loss)]
async fn cmd_learn_role_costs(workdir: &std::path::Path, json: bool) -> Result<i32> {
    let eff_path = learn_efficiency_path(workdir);
    let text = tokio::fs::read_to_string(&eff_path)
        .await
        .unwrap_or_default();

    let events: Vec<roko_learn::efficiency::AgentEfficiencyEvent> = text
        .lines()
        .filter_map(|line| serde_json::from_str(line.trim()).ok())
        .collect();

    if events.is_empty() {
        if json {
            println!("{{\"profiles\":[]}}");
        } else {
            println!(
                "Role cost profiles: no efficiency data at {}",
                eff_path.display()
            );
        }
        return Ok(EXIT_SUCCESS);
    }

    let profiles = roko_learn::efficiency::compute_role_profiles(&events);

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "profiles": profiles,
            }))?
        );
    } else {
        println!("Per-role cost profiles ({})", eff_path.display());
        println!();
        println!(
            "  {:<16} {:>5} {:>10} {:>10} {:>8} {:>10} {:>8}",
            "Role", "Obs", "Avg Cost", "P95 Cost", "Pass%", "Cost/Pass", "Avg Wall"
        );
        println!("  {}", "-".repeat(76));
        for p in &profiles {
            let pass_pct = format!("{:.0}%", p.pass_rate * 100.0);
            let cost_per_pass = if p.cost_per_successful_task().is_finite() {
                format!("${:.4}", p.cost_per_successful_task())
            } else {
                "inf".to_string()
            };
            println!(
                "  {:<16} {:>5} {:>10} {:>10} {:>8} {:>10} {:>8}",
                truncate_str(&p.role, 16),
                p.observations,
                format!("${:.4}", p.avg_cost_usd),
                format!("${:.4}", p.p95_cost_usd),
                pass_pct,
                cost_per_pass,
                format!("{:.0}ms", p.avg_wall_time_ms),
            );
        }
    }

    Ok(EXIT_SUCCESS)
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
                max_daily_usd: 0.0,
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

// ── Graduation command (#334) ───────────────────────────────────────

/// JSON output for `roko learn graduation --json`.
#[derive(serde::Serialize)]
struct GraduationJson {
    policy_count: usize,
    always_topics: Vec<String>,
    never_topics: Vec<String>,
    sample_topics: Vec<GraduationSampleEntry>,
}

#[derive(serde::Serialize)]
struct GraduationSampleEntry {
    topic: String,
    sample_every: usize,
}

/// `roko learn graduation` -- show configured graduation policies and
/// their evaluation semantics.
async fn cmd_learn_graduation(workdir: &std::path::Path, json: bool) -> Result<i32> {
    let config: roko_core::config::schema::RokoConfig =
        roko_core::config::loader::load_config_unified(workdir).unwrap_or_default();

    let grad_config = &config.graduation;
    let policies = &grad_config.policies;

    if json {
        let mut always_topics = Vec::new();
        let mut never_topics = Vec::new();
        let mut sample_topics = Vec::new();

        for policy in policies {
            let topic_desc = format!("{:?}", policy.watch);
            if policy.never {
                never_topics.push(topic_desc);
            } else if policy.always {
                always_topics.push(topic_desc);
            } else {
                sample_topics.push(GraduationSampleEntry {
                    topic: topic_desc,
                    sample_every: policy.sample_every,
                });
            }
        }

        let output = GraduationJson {
            policy_count: policies.len(),
            always_topics,
            never_topics,
            sample_topics,
        };
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        println!("Graduation policies ({} configured)", policies.len());
        println!();

        if policies.is_empty() {
            println!("  No graduation policies configured.");
            println!("  All Pulses will remain ephemeral (default: do not graduate).");
        } else {
            println!("  Precedence: never > always > sample_every > default (skip)");
            println!();

            for (i, policy) in policies.iter().enumerate() {
                let mode = if policy.never {
                    "NEVER"
                } else if policy.always {
                    "ALWAYS"
                } else {
                    "SAMPLE"
                };

                let sample_note = if !policy.always && !policy.never && policy.sample_every > 1 {
                    format!(" (every {})", policy.sample_every)
                } else {
                    String::new()
                };

                println!(
                    "  [{}] {:<8} {:?}{}",
                    i + 1,
                    mode,
                    policy.watch,
                    sample_note,
                );
            }
        }
    }

    Ok(EXIT_SUCCESS)
}
