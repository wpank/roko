//! `POST /api/plans/{id}/estimate` (gap-6605f7, dogfood U9, spec v3 R7):
//! each remaining task priced where dispatch would run it.
//!
//! A task's chain of attempts follows the model ladder as dispatch binds it
//! ([`RoutingLadder`]). It starts on its tier's start rung (or the rung its
//! `rung` hint names), gets [`FAILURES_PER_RUNG`] attempts on each rung and
//! climbs at most [`MAX_ESCALATIONS`] runnable rungs, within its retry budget:
//! its authored `max_retries`, else at least [`LADDER_MIN_RETRIES`]. A model
//! that `model_hint` or `preferred_model` pins stays put, as does the
//! router's pick for a tier when the ladder routes nothing. Each model is
//! priced from the workspace's price snapshot, else its `[models.*]` rates,
//! else built-in pricing. [`roko_learn::task_estimate`] prices the chains from
//! what similar tasks' recent attempts used.

use std::collections::BTreeSet;
use std::path::Path;

use anyhow::{Context as _, Result};
use roko_core::config::model_registry::builtin_pricing;
use roko_core::config::schema::RokoConfig;
use roko_core::pricing_snapshot::PriceSnapshot;
use roko_core::task::{TaskComplexityBand, TaskTier};
use roko_fs::RokoLayout;
use roko_learn::task_estimate::{
    PlanEstimate, Rates, RungPrice, TaskRoute, estimate_plan, load_past_attempts,
};

use crate::dispatch::RoutingLadder;
use crate::graph_task_dispatch::{FAILURES_PER_RUNG, LADDER_MIN_RETRIES, MAX_ESCALATIONS};
use crate::task_parser::{TaskDef, TasksFile};

/// The estimate of plan `plan_id`'s remaining tasks, from its `tasks.toml`
/// text, in the workspace at `workdir` that `config` configures.
///
/// # Errors
///
/// When `tasks_toml` does not parse.
pub fn estimate_plan_tasks(
    workdir: &Path,
    config: &RokoConfig,
    plan_id: &str,
    tasks_toml: &str,
) -> Result<PlanEstimate> {
    let ladder = RoutingLadder::from_config(config);
    estimate_with(workdir, config, ladder.as_ref(), plan_id, tasks_toml)
}

/// [`estimate_plan_tasks`] on the ladder `ladder`.
pub(crate) fn estimate_with(
    workdir: &Path,
    config: &RokoConfig,
    ladder: Option<&RoutingLadder>,
    plan_id: &str,
    tasks_toml: &str,
) -> Result<PlanEstimate> {
    let tasks = TasksFile::parse_str(tasks_toml)
        .with_context(|| format!("parse the tasks of plan {plan_id}"))?;
    let authored = authored_max_retries(tasks_toml);
    let snapshot = PriceSnapshot::shared(&config.pricing, workdir);
    let history = load_past_attempts(&RokoLayout::for_project(workdir).runs_dir());
    let (done, remaining): (Vec<&TaskDef>, Vec<&TaskDef>) = tasks
        .tasks
        .iter()
        .partition(|task| crate::plan::task_status_is_complete(&task.status));
    let routes: Vec<TaskRoute> = remaining
        .into_iter()
        .map(|task| {
            let authored = authored.contains(&task.id);
            route(task, config, ladder, snapshot.as_deref(), authored)
        })
        .collect();
    let cap = f64::from(config.budget.max_plan_usd);
    Ok(estimate_plan(
        plan_id,
        &routes,
        done.len(),
        &history,
        (cap > 0.0).then_some(cap),
        snapshot.map(|snapshot| snapshot.id().to_string()),
    ))
}

/// How dispatch would run `task`: one entry per attempt of its chain in the
/// worst case.
fn route(
    task: &TaskDef,
    config: &RokoConfig,
    ladder: Option<&RoutingLadder>,
    snapshot: Option<&PriceSnapshot>,
    authored: bool,
) -> TaskRoute {
    let role = task
        .role
        .clone()
        .unwrap_or_else(|| "implementer".to_string());
    let tier = task.tier_class();
    let price = |rung: Option<String>, model: String| RungPrice {
        rates: rates_for(config, snapshot, &model),
        rung,
        model,
    };
    let pin = task
        .model_hint
        .clone()
        .or_else(|| task.hints.preferred_model.clone())
        .filter(|model| !model.trim().is_empty());
    let pinned = pin.is_some();
    let start = ladder.filter(|_| !pinned).and_then(|ladder| {
        let start = ladder.start(&role, tier, task.hints.rung.as_deref())?;
        Some((ladder, start))
    });
    let attempts = match (pin, start) {
        (Some(model), _) => vec![price(None, model); attempts(task, authored, false)],
        (None, Some((ladder, start))) => {
            let mut rungs = ladder.rung_models_above(&role, start.index);
            rungs.truncate(usize::try_from(MAX_ESCALATIONS).unwrap_or(usize::MAX));
            rungs.insert(0, start);
            let per_rung = usize::try_from(FAILURES_PER_RUNG).unwrap_or(1).max(1);
            (0..attempts(task, authored, true))
                .map(|attempt| {
                    let rung = &rungs[(attempt / per_rung).min(rungs.len() - 1)];
                    price(Some(rung.name.clone()), rung.model.clone())
                })
                .collect()
        }
        (None, None) => {
            vec![price(None, router_model(config, tier)); attempts(task, authored, false)]
        }
    };
    TaskRoute {
        task_id: task.id.clone(),
        title: task.title.clone(),
        role,
        tier,
        pinned,
        attempts,
    }
}

/// Attempts in `task`'s chain: its retry budget plus the first. The budget is
/// its authored `max_retries`, else its default, raised to what climbing the
/// ladder takes when the ladder routes it.
fn attempts(task: &TaskDef, authored: bool, laddered: bool) -> usize {
    let retries = if authored || !laddered {
        task.max_retries
    } else {
        task.max_retries.max(LADDER_MIN_RETRIES)
    };
    usize::try_from(retries).unwrap_or(usize::MAX - 1) + 1
}

/// The model the router picks for a tier when the ladder routes nothing: the
/// tier band's `[routing]` model, else `[agent] default_model`.
fn router_model(config: &RokoConfig, tier: TaskTier) -> String {
    let routing = &config.routing;
    let model = match tier.complexity_band() {
        TaskComplexityBand::Fast => &routing.fast_task_model,
        TaskComplexityBand::Standard => &routing.standard_task_model,
        _ => &routing.complex_task_model,
    };
    if model.trim().is_empty() {
        config.agent.default_model.clone()
    } else {
        model.clone()
    }
}

/// The rates of `model` (a `[models.*]` key or a slug): the price snapshot's
/// row for its slug, else its `[models.*]` rates, else its built-in pricing;
/// `None` when nothing prices it.
fn rates_for(config: &RokoConfig, snapshot: Option<&PriceSnapshot>, model: &str) -> Option<Rates> {
    let resolved = roko_core::agent::resolve_model(config, model);
    let slug = if resolved.slug.trim().is_empty() {
        model
    } else {
        resolved.slug.as_str()
    };
    if let Some(row) = snapshot.and_then(|snapshot| snapshot.row(slug)) {
        return Some(Rates {
            input_per_m: row.input,
            cache_read_per_m: row.cache_read,
            output_per_m: row.output,
        });
    }
    if let Some(profile) = resolved
        .profile
        .as_ref()
        .filter(|profile| profile.cost_input_per_m.is_some() || profile.cost_output_per_m.is_some())
    {
        let input = profile.cost_input_per_m.unwrap_or(0.0);
        return Some(Rates {
            input_per_m: input,
            cache_read_per_m: profile.cost_cache_read_per_m.unwrap_or(input),
            output_per_m: profile.cost_output_per_m.unwrap_or(0.0),
        });
    }
    builtin_pricing(slug).map(|pricing| Rates {
        input_per_m: pricing.input_per_m,
        cache_read_per_m: pricing.cache_read_per_m,
        output_per_m: pricing.output_per_m,
    })
}

/// The ids of the tasks that author `max_retries`, whose budget dispatch
/// keeps exactly (`graph_task_dispatch::retry_budget`).
fn authored_max_retries(tasks_toml: &str) -> BTreeSet<String> {
    roko_graph::AuthoredPlan::from_tasks_toml(tasks_toml)
        .map(|plan| {
            plan.tasks
                .into_iter()
                .filter(|(_, task)| task.get("max_retries").is_some())
                .map(|(id, _)| id)
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use roko_learn::task_estimate::EstimateBasis;
    use roko_learn::telemetry::records::{
        ATTEMPT_OPEN_SCHEMA, ATTEMPTS_FILE, AttemptIdentity, AttemptKey, AttemptOpenRecord,
        AttemptOutcome, AttemptVerdictRecord, Stamped, VERDICT_SCHEMA,
    };

    use super::*;

    /// Rungs of a ladder whose models are priced in the built-in snapshot.
    const CONFIG: &str = r#"
[models.cheap-model]
provider = "p"
slug = "gpt-oss-120b"

[models.mid-model]
provider = "p"
slug = "glm-4.7"

[models.strong-model]
provider = "p"
slug = "gpt-5.4-mini"

[routing.ladder]
rungs = [
    { name = "cheap", model = "cheap-model" },
    { name = "mid", model = "mid-model" },
    { name = "strong", model = "strong-model" },
]

[routing.ladder.start]
mechanical = "cheap"
focused = "cheap"
integrative = "mid"
architectural = "strong"
"#;

    const TASKS: &str = r#"
[meta]
plan = "synth"

[[task]]
id = "OSC"
title = "Oscillator"
role = "implementer"
tier = "focused"
files = ["osc.py"]

[[task]]
id = "MIX"
title = "Mixer"
tier = "integrative"
files = ["mix.py"]

[[task]]
id = "PIN"
title = "Pinned"
tier = "focused"
model_hint = "claude-haiku-4-5"
max_retries = 1
files = ["pin.py"]

[[task]]
id = "DONE"
title = "Done already"
status = "done"
files = ["done.py"]
"#;

    fn line<T: serde::Serialize>(schema: &str, record: T) -> String {
        serde_json::to_string(&Stamped {
            schema_version: schema.to_string(),
            record_id: String::new(),
            seq: 0,
            ts: String::new(),
            record,
        })
        .expect("serialize line")
    }

    fn close(actual: Option<f64>, expected: f64) -> bool {
        actual.is_some_and(|actual| (actual - expected).abs() < 1e-4)
    }

    /// gap-6605f7: each remaining task is priced on the rung dispatch starts
    /// it on, from what similar tasks' recent attempts used: one attempt
    /// there, the likely chain (weighted by the first-try rate) and the worst
    /// case, two attempts on each rung of its climb. A pinned model stays put
    /// for its authored budget, and a done task is not estimated.
    #[test]
    fn plan_estimate_prices_each_task_on_its_start_rung_and_its_climbs() {
        let temp = tempfile::tempdir().expect("tempdir");
        let workdir = temp.path();
        let run = RokoLayout::for_project(workdir).runs_dir().join("run-1");
        std::fs::create_dir_all(&run).expect("run dir");
        // Three first tries of focused implementer tasks: two passed.
        let mut lines = Vec::new();
        let outcomes = [
            (AttemptOutcome::Passed, 100_000),
            (AttemptOutcome::Passed, 200_000),
            (AttemptOutcome::GateFailed, 300_000),
        ];
        for (index, (outcome, settled)) in outcomes.into_iter().enumerate() {
            let key = AttemptKey::new("run-1", "past", &format!("T{index}"), 1);
            let mut open = AttemptOpenRecord::new(AttemptIdentity::new(&key), settled - 30_000);
            open.role = Some("implementer".to_string());
            open.tier = Some("focused".to_string());
            let mut verdict =
                AttemptVerdictRecord::settle(AttemptIdentity::new(&key), outcome, true);
            verdict.usage.tokens_in = Some(20_000);
            verdict.usage.tokens_cache_read = Some(80_000);
            verdict.usage.tokens_out = Some(9_000);
            verdict.timing.attempt_started_at = Some(settled - 30_000);
            verdict.timing.settled_at = Some(settled);
            lines.push(line(ATTEMPT_OPEN_SCHEMA, open));
            lines.push(line(VERDICT_SCHEMA, verdict));
        }
        std::fs::write(run.join(ATTEMPTS_FILE), lines.join("\n")).expect("attempts");
        let config: RokoConfig = toml::from_str(CONFIG).expect("config");
        let ladder = RoutingLadder::from_config_with(&config, |_| true).expect("the ladder runs");

        let estimate =
            estimate_with(workdir, &config, Some(&ladder), "synth", TASKS).expect("estimate");

        assert_eq!(
            (
                estimate.total_tasks,
                estimate.completed_tasks,
                estimate.remaining_tasks
            ),
            (4, 1, 3)
        );
        // Per attempt: 20,000 tokens in, 80,000 from the cache, 9,000 out.
        let cheap = 0.04175; // gpt-oss-120b: $0.35 in and cached, $0.75 out
        let mid = 0.0406; // glm-4.7: $0.60 in, $0.11 cached, $2.20 out
        let strong = 0.0615; // gpt-5.4-mini: $0.75 in, $0.075 cached, $4.50 out
        let haiku = 0.073; // claude-haiku-4-5: $1 in, $0.10 cached, $5 out
        let task = |id: &str| {
            estimate
                .per_task
                .iter()
                .find(|task| task.task_id == id)
                .unwrap_or_else(|| panic!("task {id}"))
        };

        let osc = task("OSC");
        assert_eq!(osc.start_rung.as_deref(), Some("cheap"));
        assert_eq!(osc.basis, EstimateBasis::SameRoleAndTier);
        assert_eq!(osc.samples, 3);
        assert_eq!(osc.first_try_rate, Some(0.67));
        assert!(close(osc.start_cost_usd, cheap), "{osc:?}");
        // Each attempt weighted by the chance (1/3 each) that those before it failed.
        let likely = cheap + cheap / 3.0 + mid / 9.0 + mid / 27.0 + strong / 81.0 + strong / 243.0;
        assert!(close(osc.likely_cost_usd, likely), "{osc:?}");
        assert!(
            close(osc.worst_cost_usd, 2.0 * (cheap + mid + strong)),
            "{osc:?}"
        );
        assert_eq!(
            osc.worst_rungs,
            ["cheap", "cheap", "mid", "mid", "strong", "strong"]
        );

        // Integrative starts on mid, with one rung to climb; no history of its
        // own role and tier, so it reads any task's.
        let mix = task("MIX");
        assert_eq!(mix.start_rung.as_deref(), Some("mid"));
        assert_eq!(mix.basis, EstimateBasis::AnyTask);
        assert!(
            close(mix.worst_cost_usd, 2.0 * mid + 4.0 * strong),
            "{mix:?}"
        );

        // A pinned model never climbs, within its authored budget.
        let pin = task("PIN");
        assert!(pin.pinned);
        assert_eq!(pin.start_rung, None);
        assert_eq!(pin.model, "claude-haiku-4-5");
        assert!(close(pin.worst_cost_usd, 2.0 * haiku), "{pin:?}");
        assert!(estimate.per_task.iter().all(|task| task.task_id != "DONE"));

        assert!(estimate.estimate.unpriced_tasks.is_empty());
        let total: f64 = estimate
            .per_task
            .iter()
            .filter_map(|task| task.likely_cost_usd)
            .sum();
        assert!(close(estimate.estimate.total_cost_usd, total));
        assert_eq!(estimate.confidence, "medium");
        assert_eq!(
            estimate.price_snapshot.as_deref(),
            Some(roko_core::pricing_snapshot::BUILTIN_SNAPSHOT_ID)
        );
        let rungs: Vec<(&str, usize)> = estimate
            .by_rung
            .iter()
            .map(|share| (share.rung.as_str(), share.tasks))
            .collect();
        assert_eq!(rungs, [("cheap", 1), ("mid", 1), ("claude-haiku-4-5", 1)]);
    }
}
