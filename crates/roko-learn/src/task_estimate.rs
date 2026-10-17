//! Cost and time estimates for plan tasks on the model ladder (gap-6605f7,
//! dogfood U9, spec v3 R7): what similar tasks' recent attempts used, priced
//! at the rates of each rung a task may run on.
//!
//! [`load_past_attempts`] reads a workspace's settled attempts
//! (`.roko/runs/*/attempts.jsonl`), each verdict with its open line, which
//! carries the task's role and tier. [`similar_attempts`] takes the recent
//! ones of the same role and tier, else of the same tier, else any: the median
//! tokens and time of one attempt, and how often a first attempt passed.
//! [`estimate_task`] prices a task's chain of attempts as dispatch would run
//! it ([`TaskRoute`], one rung per attempt): one attempt on its start rung;
//! the likely total, each attempt weighted by the chance that the ones before
//! it failed; and the worst case, where every attempt fails and the task
//! climbs as far as it can. [`estimate_plan`] adds the tasks up.
//!
//! It is an estimate: list prices, and the same tokens on every rung, though
//! a stronger model may need fewer.

use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

use roko_core::task::TaskTier;
use serde::{Deserialize, Serialize};

use crate::telemetry::records::{
    ATTEMPT_OPEN_SCHEMA, ATTEMPTS_FILE, AttemptOpenRecord, AttemptVerdictRecord, Stamped,
    VERDICT_SCHEMA,
};

/// Settled attempts an estimate reads: the most recent ones.
pub const RECENT: usize = 200;

/// Fewest matching attempts before an estimate takes a narrower match.
pub const MIN_SIMILAR: usize = 3;

/// Matching attempts that make an estimate's confidence high.
pub const CONFIDENT_SAMPLES: usize = 10;

/// One attempt when there is no history: tokens and seconds.
pub const DEFAULT_ATTEMPT: AttemptProfile = AttemptProfile {
    tokens_in: 8_000,
    tokens_cache_read: 0,
    tokens_out: 4_000,
    secs: 60.0,
};

/// One settled attempt, as an estimate reads it.
#[derive(Debug, Clone, PartialEq)]
pub struct PastAttempt {
    /// The role it was dispatched in, from its open line.
    pub role: Option<String>,
    /// Its task's tier, from its open line.
    pub tier: Option<TaskTier>,
    /// Its ordinal in its task's chain: 1 is the first try.
    pub attempt: u32,
    /// Whether it passed; `None` when it carries no learning label.
    pub passed: Option<bool>,
    /// Uncached input tokens.
    pub tokens_in: Option<u64>,
    /// Input tokens read from the prompt cache.
    pub tokens_cache_read: Option<u64>,
    /// Output tokens.
    pub tokens_out: Option<u64>,
    /// Seconds from the attempt's start to its verdict.
    pub secs: Option<f64>,
    /// Unix ms it settled.
    pub settled_at: Option<i64>,
}

/// What one attempt uses: tokens in each priced class, and seconds.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AttemptProfile {
    /// Uncached input tokens.
    pub tokens_in: u64,
    /// Input tokens read from the prompt cache.
    pub tokens_cache_read: u64,
    /// Output tokens.
    pub tokens_out: u64,
    /// Seconds from the attempt's start to its verdict.
    pub secs: f64,
}

/// A model's rates, in USD per 1M tokens.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Rates {
    /// Uncached input.
    pub input_per_m: f64,
    /// Input read from the prompt cache.
    pub cache_read_per_m: f64,
    /// Output.
    pub output_per_m: f64,
}

impl Rates {
    /// What `profile`'s tokens cost at these rates, in USD.
    #[must_use]
    pub fn cost(&self, profile: &AttemptProfile) -> f64 {
        (profile.tokens_in as f64 * self.input_per_m
            + profile.tokens_cache_read as f64 * self.cache_read_per_m
            + profile.tokens_out as f64 * self.output_per_m)
            / 1_000_000.0
    }
}

/// Which attempts an estimate drew on, from the closest match to the
/// weakest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EstimateBasis {
    /// Recent attempts of tasks with the same role and tier.
    SameRoleAndTier,
    /// Recent attempts of tasks with the same tier.
    SameTier,
    /// Recent attempts of any task.
    AnyTask,
    /// No attempts yet: [`DEFAULT_ATTEMPT`].
    Defaults,
}

/// What similar tasks' recent attempts used.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SimilarAttempts {
    /// The median of each measure over them.
    pub profile: AttemptProfile,
    /// Which attempts they are.
    pub basis: EstimateBasis,
    /// How many.
    pub samples: usize,
    /// The share of their first tries, with a verdict, that passed.
    pub first_try_rate: Option<f64>,
}

/// One attempt of a task's chain: the rung it runs on, the model dispatch runs
/// there, and that model's rates (`None`: no price is known for it).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RungPrice {
    /// The ladder rung; `None` for a pinned model or one the router picks.
    pub rung: Option<String>,
    /// The model dispatch runs.
    pub model: String,
    /// Its rates.
    pub rates: Option<Rates>,
}

/// A task as dispatch would run it: one entry per attempt of its chain in the
/// worst case, from its start rung up.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TaskRoute {
    /// The task id.
    pub task_id: String,
    /// The task title.
    pub title: String,
    /// The role it is dispatched in.
    pub role: String,
    /// Its tier.
    pub tier: TaskTier,
    /// Whether its model is pinned, so it never climbs.
    pub pinned: bool,
    /// Each attempt's rung, the first on the start rung.
    pub attempts: Vec<RungPrice>,
}

/// The estimate of one task.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TaskEstimate {
    /// The task id.
    pub task_id: String,
    /// The task title.
    pub title: String,
    /// The role it is dispatched in.
    pub role: String,
    /// Its tier.
    pub tier: String,
    /// The rung it starts on; `None` for a pinned model or the router's.
    pub start_rung: Option<String>,
    /// The model of its first attempt.
    pub model: String,
    /// Whether its model is pinned, so it never climbs.
    pub pinned: bool,
    /// One attempt on its start rung, in USD.
    pub start_cost_usd: Option<f64>,
    /// The likely total, each attempt weighted by the chance that the ones
    /// before it failed (one attempt when no first-try rate is known).
    pub likely_cost_usd: Option<f64>,
    /// The likely time, in seconds, weighted the same way.
    pub likely_duration_secs: f64,
    /// Every attempt fails, and the task climbs as far as it can.
    pub worst_cost_usd: Option<f64>,
    /// The time of every attempt.
    pub worst_duration_secs: f64,
    /// The rung (else the model) of each attempt of the worst case.
    pub worst_rungs: Vec<String>,
    /// The share of similar tasks' first tries that passed.
    pub first_try_rate: Option<f64>,
    /// Which attempts the estimate drew on.
    pub basis: EstimateBasis,
    /// How many.
    pub samples: usize,
    /// What one attempt uses.
    pub per_attempt: AttemptProfile,
    /// `likely_cost_usd`, under its older name.
    pub estimated_cost_usd: Option<f64>,
    /// `likely_duration_secs`, under its older name.
    pub estimated_duration_secs: f64,
}

/// Remaining tasks by the rung they start on.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RungShare {
    /// The start rung (else the model).
    pub rung: String,
    /// Its model.
    pub model: String,
    /// The tasks that start there.
    pub tasks: usize,
    /// Their likely total, in USD; `None` when the model has no price.
    pub likely_cost_usd: Option<f64>,
}

/// The plan's totals.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlanTotals {
    /// The likely total of the priced tasks, in USD; `None` when no task is
    /// priced.
    pub total_cost_usd: Option<f64>,
    /// The likely time of every task, in seconds, one after another.
    pub total_duration_secs: f64,
    /// The worst case of the priced tasks, in USD.
    pub worst_cost_usd: Option<f64>,
    /// The worst-case time, in seconds.
    pub worst_duration_secs: f64,
    /// The plan's spending cap, `[budget] max_plan_usd`: a run stops there.
    pub cap_usd: Option<f64>,
    /// Tasks whose models have no known price, left out of the totals.
    pub unpriced_tasks: Vec<String>,
}

/// The estimate of a plan's remaining tasks.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlanEstimate {
    /// The plan id.
    pub plan_id: String,
    /// Its tasks.
    pub total_tasks: usize,
    /// Those already done.
    pub completed_tasks: usize,
    /// Those the estimate covers.
    pub remaining_tasks: usize,
    /// The totals.
    pub estimate: PlanTotals,
    /// The remaining tasks by start rung.
    pub by_rung: Vec<RungShare>,
    /// Each remaining task.
    pub per_task: Vec<TaskEstimate>,
    /// `high`, `medium` or `low`, by how much history the estimate has.
    pub confidence: String,
    /// What the estimate rests on, in a sentence.
    pub note: String,
    /// The price snapshot behind the rates, when one was read.
    pub price_snapshot: Option<String>,
}

/// The fields of an attempt line an estimate needs, whichever its schema.
#[derive(Deserialize)]
struct LineProbe {
    #[serde(default)]
    schema_version: Option<String>,
}

/// Settled attempts from every run under `runs_dir` (`.roko/runs`), the
/// [`RECENT`] most recent first. Each takes its role and tier from its open
/// line. Unreadable files and lines are skipped.
#[must_use]
pub fn load_past_attempts(runs_dir: &Path) -> Vec<PastAttempt> {
    let Ok(runs) = std::fs::read_dir(runs_dir) else {
        return Vec::new();
    };
    let mut attempts = Vec::new();
    for run in runs.flatten() {
        let Ok(file) = File::open(run.path().join(ATTEMPTS_FILE)) else {
            continue;
        };
        let mut opened: HashMap<String, AttemptOpenRecord> = HashMap::new();
        let mut verdicts = Vec::new();
        for line in BufReader::new(file).lines().map_while(Result::ok) {
            let Ok(probe) = serde_json::from_str::<LineProbe>(&line) else {
                continue;
            };
            match probe.schema_version.as_deref() {
                Some(ATTEMPT_OPEN_SCHEMA) => {
                    if let Ok(open) = serde_json::from_str::<Stamped<AttemptOpenRecord>>(&line) {
                        let key = open.record.identity.attempt_key.clone();
                        opened.insert(key, open.record);
                    }
                }
                Some(VERDICT_SCHEMA) => {
                    if let Ok(verdict) =
                        serde_json::from_str::<Stamped<AttemptVerdictRecord>>(&line)
                    {
                        verdicts.push(verdict.record);
                    }
                }
                _ => {}
            }
        }
        for verdict in verdicts {
            let open = opened.get(&verdict.identity.attempt_key);
            let timing = &verdict.timing;
            let started = timing
                .attempt_started_at
                .or_else(|| open.and_then(|open| open.attempt_started_at));
            let secs = started
                .zip(timing.settled_at)
                .and_then(|(start, end)| end.checked_sub(start))
                .filter(|ms| *ms >= 0)
                .map(|ms| ms as f64 / 1_000.0);
            attempts.push(PastAttempt {
                role: open.and_then(|open| open.role.clone()),
                tier: open
                    .and_then(|open| open.tier.as_deref())
                    .and_then(TaskTier::parse),
                attempt: verdict.identity.attempt,
                passed: verdict.learning_label.map(|label| label == 1),
                tokens_in: verdict.usage.tokens_in,
                tokens_cache_read: verdict.usage.tokens_cache_read,
                tokens_out: verdict.usage.tokens_out,
                secs,
                settled_at: timing.settled_at,
            });
        }
    }
    attempts.sort_by_key(|attempt| std::cmp::Reverse(attempt.settled_at));
    attempts.truncate(RECENT);
    attempts
}

/// What `history`'s attempts of tasks like this one used: those of the same
/// role and tier when there are [`MIN_SIMILAR`], else those of the same tier,
/// else any; [`DEFAULT_ATTEMPT`] when there are none.
#[must_use]
pub fn similar_attempts(history: &[PastAttempt], role: &str, tier: TaskTier) -> SimilarAttempts {
    let same_role = |attempt: &&PastAttempt| {
        attempt
            .role
            .as_deref()
            .is_some_and(|other| other.trim().eq_ignore_ascii_case(role.trim()))
    };
    let same_tier = |attempt: &&PastAttempt| attempt.tier == Some(tier);
    let both: Vec<&PastAttempt> = history.iter().filter(same_tier).filter(same_role).collect();
    let tiered: Vec<&PastAttempt> = history.iter().filter(same_tier).collect();
    let (basis, matched) = if both.len() >= MIN_SIMILAR {
        (EstimateBasis::SameRoleAndTier, both)
    } else if tiered.len() >= MIN_SIMILAR {
        (EstimateBasis::SameTier, tiered)
    } else if !history.is_empty() {
        (EstimateBasis::AnyTask, history.iter().collect())
    } else {
        return SimilarAttempts {
            profile: DEFAULT_ATTEMPT,
            basis: EstimateBasis::Defaults,
            samples: 0,
            first_try_rate: None,
        };
    };
    let medians = |measure: fn(&PastAttempt) -> Option<f64>| -> Option<f64> {
        median(
            matched
                .iter()
                .filter_map(|attempt| measure(attempt))
                .collect(),
        )
    };
    let tokens =
        |value: Option<f64>, default: u64| value.map_or(default, |value| value.round() as u64);
    let profile = AttemptProfile {
        tokens_in: tokens(
            medians(|attempt| attempt.tokens_in.map(|tokens| tokens as f64)),
            DEFAULT_ATTEMPT.tokens_in,
        ),
        tokens_cache_read: tokens(
            medians(|attempt| attempt.tokens_cache_read.map(|tokens| tokens as f64)),
            DEFAULT_ATTEMPT.tokens_cache_read,
        ),
        tokens_out: tokens(
            medians(|attempt| attempt.tokens_out.map(|tokens| tokens as f64)),
            DEFAULT_ATTEMPT.tokens_out,
        ),
        secs: medians(|attempt| attempt.secs).unwrap_or(DEFAULT_ATTEMPT.secs),
    };
    let first_tries: Vec<bool> = matched
        .iter()
        .filter(|attempt| attempt.attempt == 1)
        .filter_map(|attempt| attempt.passed)
        .collect();
    let first_try_rate = (!first_tries.is_empty()).then(|| {
        first_tries.iter().filter(|passed| **passed).count() as f64 / first_tries.len() as f64
    });
    SimilarAttempts {
        profile,
        basis,
        samples: matched.len(),
        first_try_rate,
    }
}

/// The likely and worst totals of a chain of attempts whose `k`-th attempt
/// costs `each[k]`, when each attempt passes with probability `pass`: likely
/// weights each attempt by the chance that the ones before it failed, and
/// worst is every attempt. With `pass` unknown the first attempt is likely.
#[must_use]
pub fn chain_totals(each: &[f64], pass: Option<f64>) -> (f64, f64) {
    let worst = each.iter().sum();
    let likely = match pass {
        None => each.first().copied().unwrap_or(0.0),
        Some(pass) => {
            let fail = 1.0 - pass.clamp(0.0, 1.0);
            let mut reach = 1.0;
            let mut total = 0.0;
            for value in each {
                total += reach * value;
                reach *= fail;
            }
            total
        }
    };
    (likely, worst)
}

/// The estimate of `route`, from what similar tasks in `history` used.
#[must_use]
pub fn estimate_task(route: &TaskRoute, history: &[PastAttempt]) -> TaskEstimate {
    let similar = similar_attempts(history, &route.role, route.tier);
    let profile = similar.profile;
    let costs: Option<Vec<f64>> = route
        .attempts
        .iter()
        .map(|attempt| attempt.rates.map(|rates| rates.cost(&profile)))
        .collect();
    let first = route.attempts.first();
    let start_cost_usd = first
        .and_then(|attempt| attempt.rates)
        .map(|rates| round_usd(rates.cost(&profile)));
    let pass = similar.first_try_rate;
    let (likely_cost_usd, worst_cost_usd) = match &costs {
        Some(costs) => {
            let (likely, worst) = chain_totals(costs, pass);
            (Some(round_usd(likely)), Some(round_usd(worst)))
        }
        // A rung of the climb has no price: the likely cost is still known
        // when the start rung's is and no retry is likely.
        None => (start_cost_usd.filter(|_| pass.is_none()), None),
    };
    let times = vec![profile.secs; route.attempts.len().max(1)];
    let (likely_secs, worst_secs) = chain_totals(&times, pass);
    TaskEstimate {
        task_id: route.task_id.clone(),
        title: route.title.clone(),
        role: route.role.clone(),
        tier: route.tier.label().to_string(),
        start_rung: first.and_then(|attempt| attempt.rung.clone()),
        model: first
            .map(|attempt| attempt.model.clone())
            .unwrap_or_default(),
        pinned: route.pinned,
        start_cost_usd,
        likely_cost_usd,
        likely_duration_secs: likely_secs.round(),
        worst_cost_usd,
        worst_duration_secs: worst_secs.round(),
        worst_rungs: route
            .attempts
            .iter()
            .map(|attempt| {
                attempt
                    .rung
                    .clone()
                    .unwrap_or_else(|| attempt.model.clone())
            })
            .collect(),
        first_try_rate: similar
            .first_try_rate
            .map(|rate| (rate * 100.0).round() / 100.0),
        basis: similar.basis,
        samples: similar.samples,
        per_attempt: profile,
        estimated_cost_usd: likely_cost_usd,
        estimated_duration_secs: likely_secs.round(),
    }
}

/// The estimate of plan `plan_id`, whose remaining tasks run as `routes`
/// (`completed` of its tasks are done), from `history`. `cap_usd` is the
/// plan's spending cap and `price_snapshot` the snapshot behind the rates.
#[must_use]
pub fn estimate_plan(
    plan_id: &str,
    routes: &[TaskRoute],
    completed: usize,
    history: &[PastAttempt],
    cap_usd: Option<f64>,
    price_snapshot: Option<String>,
) -> PlanEstimate {
    let per_task: Vec<TaskEstimate> = routes
        .iter()
        .map(|route| estimate_task(route, history))
        .collect();
    let unpriced_tasks: Vec<String> = per_task
        .iter()
        .filter(|task| task.likely_cost_usd.is_none())
        .map(|task| task.task_id.clone())
        .collect();
    let sum = |value: fn(&TaskEstimate) -> Option<f64>| -> Option<f64> {
        let known: Vec<f64> = per_task.iter().filter_map(value).collect();
        (!known.is_empty()).then(|| round_usd(known.iter().sum()))
    };
    let estimate = PlanTotals {
        total_cost_usd: sum(|task| task.likely_cost_usd),
        total_duration_secs: per_task.iter().map(|task| task.likely_duration_secs).sum(),
        worst_cost_usd: sum(|task| task.worst_cost_usd),
        worst_duration_secs: per_task.iter().map(|task| task.worst_duration_secs).sum(),
        cap_usd: cap_usd.filter(|cap| *cap > 0.0),
        unpriced_tasks,
    };
    let mut by_rung: Vec<RungShare> = Vec::new();
    for task in &per_task {
        let rung = task
            .start_rung
            .clone()
            .unwrap_or_else(|| task.model.clone());
        match by_rung.iter_mut().find(|share| share.rung == rung) {
            Some(share) => {
                share.tasks += 1;
                share.likely_cost_usd = share
                    .likely_cost_usd
                    .zip(task.likely_cost_usd)
                    .map(|(sum, cost)| round_usd(sum + cost));
            }
            None => by_rung.push(RungShare {
                rung,
                model: task.model.clone(),
                tasks: 1,
                likely_cost_usd: task.likely_cost_usd,
            }),
        }
    }
    let confidence = confidence(&per_task, &estimate);
    let note = note(&per_task, &estimate);
    PlanEstimate {
        plan_id: plan_id.to_string(),
        total_tasks: completed + per_task.len(),
        completed_tasks: completed,
        remaining_tasks: per_task.len(),
        estimate,
        by_rung,
        per_task,
        confidence: confidence.to_string(),
        note,
        price_snapshot,
    }
}

/// `high` when every task has [`CONFIDENT_SAMPLES`] attempts of its role and
/// tier and a price; `low` when a task has no history or no price; `medium`
/// otherwise.
fn confidence(tasks: &[TaskEstimate], totals: &PlanTotals) -> &'static str {
    if tasks
        .iter()
        .any(|task| task.basis >= EstimateBasis::AnyTask)
        || !totals.unpriced_tasks.is_empty()
    {
        "low"
    } else if tasks.iter().all(|task| {
        task.basis == EstimateBasis::SameRoleAndTier && task.samples >= CONFIDENT_SAMPLES
    }) {
        "high"
    } else {
        "medium"
    }
}

/// What the estimate rests on, in a sentence that names its weakest basis
/// and how many tasks rest on it (bug-d88298): the attempts of tasks with
/// each task's role and tier, of its tier, of any task, or none.
fn note(tasks: &[TaskEstimate], totals: &PlanTotals) -> String {
    let samples = tasks.iter().map(|task| task.samples).max().unwrap_or(0);
    let Some(weakest) = tasks.iter().map(|task| task.basis).max() else {
        return "Every task is done.".to_string();
    };
    let resting = tasks.iter().filter(|task| task.basis == weakest).count();
    let share = format!("{resting} of {}", tasks.len());
    let mut note = match weakest {
        EstimateBasis::SameRoleAndTier => format!(
            "Each task priced on its start rung from up to {samples} recent attempts of tasks \
             with its role and tier"
        ),
        EstimateBasis::SameTier => format!(
            "Each task priced on its start rung from up to {samples} recent attempts of tasks \
             like it; {share} tasks have none with their role, so attempts of their tier stand \
             in"
        ),
        EstimateBasis::AnyTask => format!(
            "Low confidence: {share} tasks have no recent attempt of their tier, so up to \
             {samples} attempts of any task stand in; each is priced on its start rung"
        ),
        EstimateBasis::Defaults if samples == 0 => format!(
            "Low confidence: no attempts yet, so each task is one attempt of {} tokens in and {} \
             out, priced on its start rung",
            DEFAULT_ATTEMPT.tokens_in, DEFAULT_ATTEMPT.tokens_out
        ),
        EstimateBasis::Defaults => format!(
            "Low confidence: {share} tasks have no recorded attempt, so each is one attempt of {} \
             tokens in and {} out; the rest are priced from up to {samples} recent attempts",
            DEFAULT_ATTEMPT.tokens_in, DEFAULT_ATTEMPT.tokens_out
        ),
    };
    if samples > 0 {
        note.push_str("; the worst case is every attempt failing and climbing the ladder");
    }
    if !totals.unpriced_tasks.is_empty() {
        note.push_str(&format!(
            "; {} left out of the totals: no price for their models",
            totals.unpriced_tasks.join(", ")
        ));
    }
    note.push('.');
    note
}

/// The median of `values`; `None` when there are none.
fn median(mut values: Vec<f64>) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    values.sort_by(f64::total_cmp);
    let middle = values.len() / 2;
    Some(if values.len().is_multiple_of(2) {
        f64::midpoint(values[middle - 1], values[middle])
    } else {
        values[middle]
    })
}

/// `usd` to a hundredth of a cent.
fn round_usd(usd: f64) -> f64 {
    (usd * 10_000.0).round() / 10_000.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::telemetry::records::{AttemptIdentity, AttemptKey, AttemptOutcome};

    fn line<T: Serialize>(schema: &str, record: T) -> String {
        serde_json::to_string(&Stamped {
            schema_version: schema.to_string(),
            record_id: String::new(),
            seq: 0,
            ts: String::new(),
            record,
        })
        .expect("serialize line")
    }

    /// The open line and verdict of attempt `attempt` of task `task`.
    fn attempt_lines(
        task: &str,
        attempt: u32,
        role: &str,
        tier: &str,
        outcome: AttemptOutcome,
        tokens: (u64, u64, u64),
        settled_at: i64,
    ) -> [String; 2] {
        let key = AttemptKey::new("run-1", "plan", task, attempt);
        let mut open = AttemptOpenRecord::new(AttemptIdentity::new(&key), settled_at - 30_000);
        open.role = Some(role.to_string());
        open.tier = Some(tier.to_string());
        let mut verdict = AttemptVerdictRecord::settle(AttemptIdentity::new(&key), outcome, true);
        verdict.usage.tokens_in = Some(tokens.0);
        verdict.usage.tokens_cache_read = Some(tokens.1);
        verdict.usage.tokens_out = Some(tokens.2);
        verdict.timing.attempt_started_at = Some(settled_at - 30_000);
        verdict.timing.settled_at = Some(settled_at);
        [
            line(ATTEMPT_OPEN_SCHEMA, open),
            line(VERDICT_SCHEMA, verdict),
        ]
    }

    #[test]
    fn chain_totals_weight_each_attempt_by_the_failures_before_it() {
        let each = [1.0, 1.0, 2.0, 2.0];
        assert_eq!(chain_totals(&each, None), (1.0, 6.0));
        assert_eq!(chain_totals(&each, Some(1.0)), (1.0, 6.0));
        let (likely, worst) = chain_totals(&each, Some(0.5));
        assert!((likely - (1.0 + 0.5 + 0.5 + 0.25)).abs() < 1e-9);
        assert_eq!(worst, 6.0);
        assert_eq!(chain_totals(&[], None), (0.0, 0.0));
    }

    /// The loader joins each verdict with its open line, and the profile takes
    /// the same role and tier's medians and first-try rate, falling back to
    /// the tier, then any task, then the defaults.
    #[test]
    fn past_attempts_load_and_match_by_role_and_tier() {
        let dir = tempfile::tempdir().expect("tempdir");
        let run = dir.path().join("run-1");
        std::fs::create_dir_all(&run).expect("run dir");
        let lines: Vec<String> = [
            attempt_lines(
                "A",
                1,
                "implementer",
                "focused",
                AttemptOutcome::Passed,
                (10_000, 50_000, 5_000),
                100_000,
            ),
            attempt_lines(
                "B",
                1,
                "implementer",
                "focused",
                AttemptOutcome::GateFailed,
                (20_000, 60_000, 7_000),
                200_000,
            ),
            attempt_lines(
                "B",
                2,
                "implementer",
                "focused",
                AttemptOutcome::Passed,
                (30_000, 70_000, 9_000),
                300_000,
            ),
            attempt_lines(
                "C",
                1,
                "scribe",
                "mechanical",
                AttemptOutcome::Passed,
                (1_000, 0, 500),
                400_000,
            ),
        ]
        .into_iter()
        .flatten()
        .chain(["not a record".to_string()])
        .collect();
        std::fs::write(run.join(ATTEMPTS_FILE), lines.join("\n")).expect("attempts");

        let history = load_past_attempts(dir.path());
        assert_eq!(history.len(), 4);
        assert_eq!(history[0].role.as_deref(), Some("scribe"), "newest first");
        assert_eq!(history[0].secs, Some(30.0));

        let focused = similar_attempts(&history, "implementer", TaskTier::Focused);
        assert_eq!(focused.basis, EstimateBasis::SameRoleAndTier);
        assert_eq!(focused.samples, 3);
        assert_eq!(focused.profile.tokens_in, 20_000);
        assert_eq!(focused.profile.tokens_cache_read, 60_000);
        assert_eq!(focused.profile.tokens_out, 7_000);
        assert_eq!(focused.first_try_rate, Some(0.5));
        let reviewer = similar_attempts(&history, "quick-reviewer", TaskTier::Focused);
        assert_eq!(reviewer.basis, EstimateBasis::SameTier);
        let architectural = similar_attempts(&history, "implementer", TaskTier::Architectural);
        assert_eq!(architectural.basis, EstimateBasis::AnyTask);
        assert_eq!(architectural.samples, 4);
        let none = similar_attempts(&[], "implementer", TaskTier::Focused);
        assert_eq!(none.basis, EstimateBasis::Defaults);
        assert_eq!(none.profile, DEFAULT_ATTEMPT);
    }

    #[test]
    fn a_plan_estimate_adds_tasks_and_groups_them_by_start_rung() {
        let rates = |input, output| Rates {
            input_per_m: input,
            cache_read_per_m: input,
            output_per_m: output,
        };
        let rung = |name: &str, model: &str, rates: Option<Rates>| RungPrice {
            rung: Some(name.to_string()),
            model: model.to_string(),
            rates,
        };
        let cheap = rung("cheap", "m-cheap", Some(rates(1.0, 2.0)));
        let mid = rung("mid", "m-mid", Some(rates(2.0, 4.0)));
        let route = |id: &str, attempts: Vec<RungPrice>| TaskRoute {
            task_id: id.to_string(),
            title: id.to_string(),
            role: "implementer".to_string(),
            tier: TaskTier::Focused,
            pinned: false,
            attempts,
        };
        let routes = [
            route(
                "T1",
                vec![cheap.clone(), cheap.clone(), mid.clone(), mid.clone()],
            ),
            route("T2", vec![cheap.clone(), cheap]),
            route("T3", vec![rung("top", "unpriced", None)]),
        ];
        let plan = estimate_plan(
            "p",
            &routes,
            1,
            &[],
            Some(0.5),
            Some("prices-x".to_string()),
        );

        // No history: one attempt of 8,000 tokens in and 4,000 out per task.
        let one_cheap = (8_000.0 * 1.0 + 4_000.0 * 2.0) / 1_000_000.0;
        let one_mid = (8_000.0 * 2.0 + 4_000.0 * 4.0) / 1_000_000.0;
        let t1 = &plan.per_task[0];
        assert_eq!(t1.start_rung.as_deref(), Some("cheap"));
        assert_eq!(t1.start_cost_usd, Some(round_usd(one_cheap)));
        assert_eq!(t1.likely_cost_usd, Some(round_usd(one_cheap)));
        assert_eq!(
            t1.worst_cost_usd,
            Some(round_usd(2.0 * one_cheap + 2.0 * one_mid))
        );
        assert_eq!(t1.worst_rungs, ["cheap", "cheap", "mid", "mid"]);
        assert_eq!(t1.basis, EstimateBasis::Defaults);
        assert_eq!(plan.estimate.unpriced_tasks, ["T3"]);
        assert_eq!(
            plan.estimate.total_cost_usd,
            Some(round_usd(2.0 * one_cheap))
        );
        assert_eq!(plan.estimate.cap_usd, Some(0.5));
        assert_eq!(plan.total_tasks, 4);
        assert_eq!(plan.remaining_tasks, 3);
        assert_eq!(plan.confidence, "low");
        assert!(plan.note.contains("T3 left out"), "{}", plan.note);
        let shares: Vec<(&str, usize)> = plan
            .by_rung
            .iter()
            .map(|share| (share.rung.as_str(), share.tasks))
            .collect();
        assert_eq!(shares, [("cheap", 2), ("top", 1)]);
    }

    /// bug-d88298: the note names the weakest basis it rests on and how many
    /// tasks rest on it, and attempts of any task make the confidence low.
    #[test]
    fn the_estimate_note_names_its_basis() {
        let rung = RungPrice {
            rung: Some("cheap".to_string()),
            model: "m-cheap".to_string(),
            rates: Some(Rates {
                input_per_m: 1.0,
                cache_read_per_m: 1.0,
                output_per_m: 2.0,
            }),
        };
        let route = |id: &str, tier: TaskTier| TaskRoute {
            task_id: id.to_string(),
            title: id.to_string(),
            role: "implementer".to_string(),
            tier,
            pinned: false,
            attempts: vec![rung.clone()],
        };
        let attempt = |tier: TaskTier| PastAttempt {
            role: Some("implementer".to_string()),
            tier: Some(tier),
            attempt: 1,
            passed: Some(true),
            tokens_in: Some(1_000),
            tokens_cache_read: Some(0),
            tokens_out: Some(500),
            secs: Some(20.0),
            settled_at: Some(1),
        };
        let history = vec![attempt(TaskTier::Focused); 3];
        let estimate = |routes: &[TaskRoute]| estimate_plan("p", routes, 0, &history, None, None);

        let alike = estimate(&[route("T1", TaskTier::Focused)]);
        assert_eq!(alike.per_task[0].basis, EstimateBasis::SameRoleAndTier);
        assert_eq!(alike.confidence, "medium");
        assert!(
            alike
                .note
                .starts_with("Each task priced on its start rung from up to 3 recent attempts of tasks with its role and tier;"),
            "{}",
            alike.note
        );
        assert!(!alike.note.contains("similar"), "{}", alike.note);

        let mixed = estimate(&[
            route("T1", TaskTier::Focused),
            route("T2", TaskTier::Integrative),
        ]);
        assert_eq!(mixed.per_task[1].basis, EstimateBasis::AnyTask);
        assert_eq!(mixed.confidence, "low");
        assert!(
            mixed.note.starts_with(
                "Low confidence: 1 of 2 tasks have no recent attempt of their tier, so up to 3 \
                 attempts of any task stand in"
            ),
            "{}",
            mixed.note
        );
    }
}
