//! Offline replay of the policies over a recorded matrix: backlog task 6120.
//!
//! H4 (S09 R-H4) and S09.E6's `vb replay` replay the routing policies prequentially over the
//! benchmark's log-once matrix. [`Matrix`] reads `vb.run_record/1` lines into one cell per
//! (task, arm, model): the outcome and cost at the lowest recorded seed. An arm that ran several
//! models (S09's LOG1 block A runs `roko_fixed` on four) becomes one arm per model,
//! [`arm_label`], so no model's runs stand in for another's; econ.py's join labels them the same
//! way. An arm whose cost source is `unknown` is left out and counted. [`replay`] runs a fresh
//! policy over each of
//! several seeded orderings of the tasks with bandit feedback: only the chosen arm's cell is
//! revealed, and the matrix counts what it reveals. Each (ordering, task) writes one trace
//! line: the arms tried, the outcome and the cost, which the economics report (6123) reads.
//! [`cross_fit`] fits a policy's parameters on every family but one held-out family.
//!
//! Policies (a) and (b) replay through [`LcbAciPolicy`] and [`CascadePolicy`], each with its own
//! [`SelfModel`] that learns every outcome it is shown. Replayed escalation differs from live
//! escalation, whose retries carry gate errors, so a report shows both (S04 §8).

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::io::Write;
use std::path::Path;

use roko_core::pricing_snapshot::PriceSnapshot;
use serde::{Deserialize, Serialize};

use super::baselines::{AttemptResult, K_MAX, MAX_ATTEMPTS, RouteTask, RoutingPolicy};
use super::cascade::{BREAK_EVEN_MARGIN, FailureContext, StepAction, after_failure, start_rung};
use super::features::TaskFeatures;
use super::model::SelfModel;
use super::policy::{LcbAci, LcbAciConfig, RouteAction, expected_cost};
use super::{ArmKey, CandidateForecast, Effort, Label, LabelSource, ROKO_HARNESS, Unit};
use crate::telemetry::{AttemptKey, CostSource};

/// `schema_version` of the run records the matrix reads.
pub const RUN_RECORD_SCHEMA: &str = "vb.run_record/1";
/// The orderings a replay runs by default (S09 R-H4).
pub const ORDERINGS: u32 = 50;

/// The fields of a run record the matrix reads.
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct RunRecordLine {
    schema_version: String,
    arm: String,
    seed: u64,
    task: RecordTask,
    vs: RecordVs,
    costs: RecordCosts,
    execution: RecordExecution,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct RecordTask {
    family: String,
    instance_id: String,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct RecordVs {
    label: Option<u8>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct RecordCosts {
    api_equiv_usd: Option<f64>,
    source: Option<CostSource>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct RecordExecution {
    attempts: Vec<RecordAttempt>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct RecordAttempt {
    model_requested: Option<String>,
}

/// One cell of the matrix: an arm's outcome on a task at its recorded seed.
#[derive(Debug, Clone, PartialEq)]
pub struct Cell {
    /// The run passed its VS check.
    pub passed: bool,
    /// Its cost, in USD at the price snapshot.
    pub cost_usd: f64,
    /// The seed the cell comes from.
    pub seed: u64,
    /// The model the run's first attempt asked for, when recorded.
    pub model_requested: Option<String>,
}

/// The (task, arm) matrix of a benchmark's run records, an arm that ran several models split
/// into one arm per model ([`arm_label`], gap-b10978).
#[derive(Debug, Default)]
pub struct Matrix {
    cells: BTreeMap<(String, String), Cell>,
    families: BTreeMap<String, String>,
    reads: RefCell<BTreeSet<(String, String)>>,
    /// Records left out because their cost source is `unknown`, or their cost is missing.
    pub excluded_unknown_cost: usize,
    /// Lines that are not run records, or carry no VS label.
    pub unreadable: usize,
}

impl Matrix {
    /// The matrix of the run records in `text`, one JSON object a line. Each (task, arm, model)
    /// keeps its lowest seed: an arm whose runs asked for more than one model has one arm per
    /// model ([`arm_label`]), an arm with one model keeps its own name.
    #[must_use]
    pub fn from_records(text: &str) -> Self {
        let mut matrix = Self::default();
        let mut runs = Vec::new();
        for line in text.lines().filter(|line| !line.trim().is_empty()) {
            let Ok(record) = serde_json::from_str::<RunRecordLine>(line) else {
                matrix.unreadable += 1;
                continue;
            };
            let Some(label) = record
                .vs
                .label
                .filter(|_| record.schema_version == RUN_RECORD_SCHEMA)
            else {
                matrix.unreadable += 1;
                continue;
            };
            let cost = record.costs.api_equiv_usd;
            let Some(cost) = cost.filter(|_| record.costs.source != Some(CostSource::Unknown))
            else {
                matrix.excluded_unknown_cost += 1;
                continue;
            };
            let task = record.task.instance_id;
            matrix
                .families
                .insert(task.clone(), record.task.family.clone());
            let cell = Cell {
                passed: label == 1,
                cost_usd: cost,
                seed: record.seed,
                model_requested: record
                    .execution
                    .attempts
                    .first()
                    .and_then(|attempt| attempt.model_requested.clone()),
            };
            runs.push((task, record.arm, cell));
        }
        let mut models: BTreeMap<&str, BTreeSet<Option<&str>>> = BTreeMap::new();
        for (_, arm, cell) in &runs {
            let model = cell.model_requested.as_deref();
            models.entry(arm.as_str()).or_default().insert(model);
        }
        let split: BTreeSet<String> = models
            .into_iter()
            .filter(|(_, models)| models.len() > 1)
            .map(|(arm, _)| arm.to_string())
            .collect();
        for (task, arm, cell) in runs {
            let arm = if split.contains(&arm) {
                arm_label(&arm, cell.model_requested.as_deref())
            } else {
                arm
            };
            let key = (task, arm);
            let earlier = matrix
                .cells
                .get(&key)
                .is_some_and(|kept| kept.seed <= cell.seed);
            if !earlier {
                matrix.cells.insert(key, cell);
            }
        }
        matrix
    }

    /// Each arm's model: the one its cells' runs asked for, the most frequent, ties by name. An
    /// arm none of whose runs recorded a model is left out.
    #[must_use]
    pub fn arm_models(&self) -> BTreeMap<String, String> {
        let mut counts: BTreeMap<&str, BTreeMap<&str, usize>> = BTreeMap::new();
        for ((_, arm), cell) in &self.cells {
            if let Some(model) = cell.model_requested.as_deref() {
                *counts.entry(arm).or_default().entry(model).or_default() += 1;
            }
        }
        counts
            .into_iter()
            .filter_map(|(arm, models)| {
                let model = models
                    .into_iter()
                    .max_by(|a, b| a.1.cmp(&b.1).then_with(|| b.0.cmp(a.0)))?;
                Some((arm.to_string(), model.0.to_string()))
            })
            .collect()
    }

    /// The matrix of every `*.jsonl` file in `dir`, in name order.
    pub fn read_dir(dir: &Path) -> std::io::Result<Self> {
        let mut paths: Vec<_> = std::fs::read_dir(dir)?
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| {
                path.extension()
                    .is_some_and(|extension| extension == "jsonl")
            })
            .collect();
        paths.sort();
        let mut text = String::new();
        for path in paths {
            text.push_str(&std::fs::read_to_string(path)?);
            text.push('\n');
        }
        Ok(Self::from_records(&text))
    }

    /// The tasks, by instance id.
    #[must_use]
    pub fn tasks(&self) -> Vec<RouteTask> {
        self.families
            .iter()
            .map(|(task, family)| RouteTask {
                task_id: task.clone(),
                family: family.clone(),
                role: "implementer".to_string(),
                tier: "focused".to_string(),
                rung_hint: None,
            })
            .collect()
    }

    /// The arms, by id.
    #[must_use]
    pub fn arms(&self) -> Vec<String> {
        let arms: BTreeSet<&String> = self.cells.keys().map(|(_, arm)| arm).collect();
        arms.into_iter().cloned().collect()
    }

    /// Reveal `arm`'s outcome on `task`, counting the read; `None` for no cell.
    pub fn reveal(&self, task: &str, arm: &str) -> Option<AttemptResult> {
        let key = (task.to_string(), arm.to_string());
        let cell = self.cells.get(&key)?;
        self.reads.borrow_mut().insert(key);
        Some(AttemptResult {
            passed: cell.passed,
            agent_blamed: !cell.passed,
            cost_usd: cell.cost_usd,
        })
    }

    /// The cells revealed so far.
    #[must_use]
    pub fn reads(&self) -> BTreeSet<(String, String)> {
        self.reads.borrow().clone()
    }

    /// Every cell as an outcome, for the oracle, which may read them all.
    #[must_use]
    pub fn outcomes(&self) -> BTreeMap<(String, String), AttemptResult> {
        self.cells
            .iter()
            .map(|(key, cell)| {
                let result = AttemptResult {
                    passed: cell.passed,
                    agent_blamed: !cell.passed,
                    cost_usd: cell.cost_usd,
                };
                (key.clone(), result)
            })
            .collect()
    }

    /// The matrix oracle's cost per resolved task: each solvable task at its cheapest passing
    /// arm. `None` when no task is solvable.
    #[must_use]
    pub fn oracle_cpr(&self) -> Option<f64> {
        let mut cheapest: BTreeMap<&str, f64> = BTreeMap::new();
        for ((task, _), cell) in self.cells.iter().filter(|(_, cell)| cell.passed) {
            let entry = cheapest.entry(task.as_str()).or_insert(f64::INFINITY);
            *entry = entry.min(cell.cost_usd);
        }
        let total: u64 = cheapest.values().map(|cost| nano_usd(*cost)).sum();
        cpr(total, cheapest.len())
    }

    /// The sub-matrix of the tasks whose family `keep` accepts.
    #[must_use]
    pub fn restricted(&self, keep: impl Fn(&str) -> bool) -> Self {
        let families: BTreeMap<String, String> = self
            .families
            .iter()
            .filter(|(_, family)| keep(family))
            .map(|(task, family)| (task.clone(), family.clone()))
            .collect();
        Self {
            cells: self
                .cells
                .iter()
                .filter(|((task, _), _)| families.contains_key(task))
                .map(|(key, cell)| (key.clone(), cell.clone()))
                .collect(),
            families,
            ..Self::default()
        }
    }
}

/// A cost in whole nano-dollars, so totals do not depend on the order they are summed in.
fn nano_usd(cost: f64) -> u64 {
    (cost.max(0.0) * 1e9).round() as u64
}

/// Cost per resolved task from a total in nano-dollars.
fn cpr(total_nano_usd: u64, resolved: usize) -> Option<f64> {
    // Dividing the exact integers first keeps proportional totals equal to the bit.
    (resolved > 0).then_some(total_nano_usd as f64 / resolved as f64 / 1e9)
}

/// One trace line: a policy's run of one task in one ordering.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TraceLine {
    /// The policy.
    pub policy: String,
    /// The ordering, from 0.
    pub ordering: u32,
    /// The task.
    pub task: String,
    /// The task's family.
    pub family: String,
    /// The arms tried, in order.
    pub arms: Vec<String>,
    /// The task resolved: its last attempt passed.
    pub passed: bool,
    /// What its attempts cost.
    pub cost_usd: f64,
}

/// A policy's totals over a replay.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReplaySummary {
    /// The policy.
    pub policy: String,
    /// The orderings replayed.
    pub orderings: u32,
    /// Task runs, over every ordering.
    pub runs: usize,
    /// Of those, the resolved.
    pub resolved: usize,
    /// Attempts made.
    pub attempts: usize,
    /// Cost per resolved task; `None` when none resolved.
    pub cpr: Option<f64>,
}

/// A small deterministic generator for the orderings (SplitMix64).
struct Shuffler(u64);

impl Shuffler {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Shuffle `items` in place (Fisher-Yates).
    fn shuffle<T>(&mut self, items: &mut [T]) {
        for i in (1..items.len()).rev() {
            let j = usize::try_from(self.next() % (i as u64 + 1)).unwrap_or(0);
            items.swap(i, j);
        }
    }
}

/// Replay a fresh policy from `make` over `orderings` seeded shuffles of the matrix's tasks
/// with bandit feedback, writing one [`TraceLine`] a task to `out`.
pub fn replay(
    matrix: &Matrix,
    make: &dyn Fn() -> Box<dyn RoutingPolicy>,
    orderings: u32,
    seed: u64,
    out: &mut dyn Write,
) -> std::io::Result<ReplaySummary> {
    let mut summary = ReplaySummary {
        policy: make().name().to_string(),
        orderings,
        runs: 0,
        resolved: 0,
        attempts: 0,
        cpr: None,
    };
    let mut total_nano_usd = 0_u64;
    for ordering in 0..orderings {
        let mut policy = make();
        let mut tasks = matrix.tasks();
        let stream = u64::from(ordering).wrapping_mul(0xD1B5_4A32_D192_ED03);
        Shuffler(seed ^ stream).shuffle(&mut tasks);
        for task in &tasks {
            let line = run_task(matrix, policy.as_mut(), task, ordering);
            summary.runs += 1;
            summary.resolved += usize::from(line.passed);
            summary.attempts += line.arms.len();
            total_nano_usd += nano_usd(line.cost_usd);
            serde_json::to_writer(&mut *out, &line)?;
            out.write_all(b"\n")?;
        }
    }
    summary.cpr = cpr(total_nano_usd, summary.resolved);
    Ok(summary)
}

/// One task's run under `policy`, revealing only the cells it chooses.
fn run_task(
    matrix: &Matrix,
    policy: &mut dyn RoutingPolicy,
    task: &RouteTask,
    ordering: u32,
) -> TraceLine {
    let mut line = TraceLine {
        policy: policy.name().to_string(),
        ordering,
        task: task.task_id.clone(),
        family: task.family.clone(),
        arms: Vec::new(),
        passed: false,
        cost_usd: 0.0,
    };
    let mut arm = policy.start(task);
    while let Some(current) = arm.take() {
        if line.arms.len() >= MAX_ATTEMPTS as usize {
            break;
        }
        let Some(result) = matrix.reveal(&task.task_id, &current) else {
            break;
        };
        line.cost_usd += result.cost_usd;
        line.passed = result.passed;
        arm = policy.next(task, &current, &result);
        line.arms.push(current);
    }
    line
}

/// The parameters in `grid` that `score` (lower is better) rates best on every family but
/// `held_out`. `score` sees only the training tasks' sub-matrix, so the held-out family never
/// shapes the fit. `None` for an empty grid.
pub fn cross_fit<P: Clone>(
    matrix: &Matrix,
    held_out: &str,
    grid: &[P],
    mut score: impl FnMut(&P, &Matrix) -> f64,
) -> Option<P> {
    let training = matrix.restricted(|family| family != held_out);
    grid.iter()
        .map(|parameters| (parameters, score(parameters, &training)))
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(parameters, _)| parameters.clone())
}

/// The matrix arm of `arm`'s runs of `model` when the arm ran several models: `<arm>[<model>]`,
/// or `<arm>[?]` for a run that recorded no model (gap-b10978). econ.py's `_matrix` labels them
/// the same way, so its rows join the replay's traces.
#[must_use]
pub fn arm_label(arm: &str, model: Option<&str>) -> String {
    format!("{arm}[{}]", model.unwrap_or("?"))
}

/// The arm key the self-model reads for a matrix arm: the arm itself when it is a key, else a
/// Roko stand-in named after it.
#[must_use]
pub fn arm_key(arm: &str) -> ArmKey {
    arm.parse().unwrap_or_else(|_| ArmKey {
        harness: ROKO_HARNESS.to_string(),
        provider: "replay".to_string(),
        model: arm.to_string(),
        effort: Effort::Default,
        depth: Some(0),
    })
}

/// The features of attempt `attempt` of `task`.
fn features(task: &RouteTask, attempt: u32) -> TaskFeatures {
    TaskFeatures {
        family: task.family.clone(),
        tier: task.tier.clone(),
        role: task.role.clone(),
        attempt,
        has_prior_failure: attempt > 1,
        ..TaskFeatures::default()
    }
}

/// The self-model's unit of a replayed attempt, VS-labelled.
fn replayed_unit(task: &RouteTask, arm: &ArmKey, attempt: u32, result: &AttemptResult) -> Unit {
    Unit {
        attempt_key: AttemptKey::new("replay", &task.family, &task.task_id, attempt),
        plan_id: task.family.clone(),
        task_id: task.task_id.clone(),
        role: task.role.clone(),
        tier: task.tier.clone(),
        family: task.family.clone(),
        arm: arm.clone(),
        attempt,
        prior_failure: attempt > 1,
        failure_class: None,
        label: Label {
            y_gate: Some(result.passed),
            y_vs: Some(result.passed),
            weight: 1.0,
            source: LabelSource::Vs,
        },
        api_equiv_usd: Some(result.cost_usd),
        cost_source: CostSource::ProviderUsage,
        latency_s: None,
        failover: false,
    }
}

/// Policy (a) for replay: its own self-model forecasts every arm, learning each outcome as it
/// is revealed, and [`LcbAci`] chooses, at most [`K_MAX`] retries a task.
pub struct LcbAciPolicy {
    model: SelfModel,
    policy: LcbAci,
    arms: Vec<String>,
    keys: Vec<ArmKey>,
    attempt: u32,
    last_p_fg: f64,
}

impl LcbAciPolicy {
    /// The policy over `arms`, with a fresh self-model priced at `snapshot`.
    #[must_use]
    pub fn new(arms: Vec<String>, config: LcbAciConfig, snapshot: &PriceSnapshot) -> Self {
        let keys = arms.iter().map(|arm| arm_key(arm)).collect();
        Self {
            model: SelfModel::new(snapshot),
            policy: LcbAci::new(config),
            arms,
            keys,
            attempt: 0,
            last_p_fg: 0.0,
        }
    }

    /// Choose an arm for attempt `self.attempt` of `task`.
    fn choose(&mut self, task: &RouteTask) -> Option<String> {
        let forecasts = self
            .model
            .forecast(&features(task, self.attempt), &self.keys);
        let recovery = forecasts.iter().map(expected_cost).fold(0.0, f64::max);
        let choice = self.policy.choose(&forecasts, None, None, recovery);
        let RouteAction::Dispatch { arm, .. } = choice.action else {
            return None;
        };
        let index = self.keys.iter().position(|key| *key == arm)?;
        self.last_p_fg = forecasts[index].p_fg;
        Some(self.arms[index].clone())
    }
}

impl RoutingPolicy for LcbAciPolicy {
    fn name(&self) -> &str {
        "lcb_aci"
    }

    fn start(&mut self, task: &RouteTask) -> Option<String> {
        self.attempt = 1;
        self.choose(task)
    }

    fn next(&mut self, task: &RouteTask, arm: &str, result: &AttemptResult) -> Option<String> {
        let key = arm_key(arm);
        let unit = replayed_unit(task, &key, self.attempt, result);
        self.model
            .observe_with(&unit, &features(task, self.attempt), 1.0);
        self.policy
            .update(Some(result.passed), result.passed, self.last_p_fg);
        if result.passed || self.attempt > K_MAX {
            return None;
        }
        self.attempt += 1;
        self.choose(task)
    }
}

/// Policy (b) for replay on `rungs`, cheapest first: its own self-model forecasts the rungs,
/// [`start_rung`] picks the start and [`after_failure`] each next step.
pub struct CascadePolicy {
    model: SelfModel,
    rungs: Vec<String>,
    keys: Vec<ArmKey>,
    current: usize,
    climbs: u32,
    attempt: u32,
}

impl CascadePolicy {
    /// The policy over `rungs`, cheapest first, with a fresh self-model priced at `snapshot`.
    #[must_use]
    pub fn new(rungs: Vec<String>, snapshot: &PriceSnapshot) -> Self {
        let keys = rungs.iter().map(|arm| arm_key(arm)).collect();
        Self {
            model: SelfModel::new(snapshot),
            rungs,
            keys,
            current: 0,
            climbs: 0,
            attempt: 0,
        }
    }

    fn forecasts(&self, task: &RouteTask) -> Vec<CandidateForecast> {
        self.model
            .forecast(&features(task, self.attempt), &self.keys)
    }
}

impl RoutingPolicy for CascadePolicy {
    fn name(&self) -> &str {
        "cascade"
    }

    fn start(&mut self, task: &RouteTask) -> Option<String> {
        self.attempt = 1;
        self.climbs = 0;
        self.current = start_rung(&self.forecasts(task), BREAK_EVEN_MARGIN)?;
        self.rungs.get(self.current).cloned()
    }

    fn next(&mut self, task: &RouteTask, arm: &str, result: &AttemptResult) -> Option<String> {
        let key = arm_key(arm);
        let unit = replayed_unit(task, &key, self.attempt, result);
        self.model
            .observe_with(&unit, &features(task, self.attempt), 1.0);
        if result.passed || self.attempt >= MAX_ATTEMPTS {
            return None;
        }
        self.attempt += 1;
        let context = FailureContext {
            current: self.current,
            climbs: self.climbs,
            retries_left: MAX_ATTEMPTS - (self.attempt - 1),
            spec_score: None,
            skip_allowed: false,
        };
        match after_failure(&self.forecasts(task), &context, &|_| None) {
            StepAction::Retry => {}
            StepAction::Climb { to } | StepAction::Skip { to } => {
                self.current = to;
                self.climbs += 1;
            }
            StepAction::RefineSpec | StepAction::Abandon => return None,
        }
        self.rungs.get(self.current).cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::self_model::baselines::{FrugalCascade, Oracle, StaticArm};

    fn fixture() -> Matrix {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/self_model/matrix");
        Matrix::read_dir(&dir).expect("read the matrix fixture")
    }

    fn rungs() -> Vec<String> {
        ["cheap_direct", "mid_direct", "top_direct"]
            .map(str::to_string)
            .to_vec()
    }

    #[test]
    fn the_matrix_keeps_one_seed_and_drops_unknown_costs() {
        let matrix = fixture();
        assert_eq!(matrix.tasks().len(), 12);
        assert_eq!(matrix.arms(), rungs());
        assert_eq!(matrix.excluded_unknown_cost, 2);
        assert_eq!(matrix.unreadable, 0);
        // F1-03's cheap arm failed at seed 1 and passed at seed 2: seed 1 stands.
        let cell = &matrix.cells[&("F1-03".to_string(), "cheap_direct".to_string())];
        assert!(!cell.passed && cell.seed == 1);
    }

    /// gap-b10978: a (task, arm) recorded under two models keeps one cell per model, each an
    /// arm of its own (`roko_fixed[glm-4.7]`) at its lowest seed, while an arm that ran one
    /// model keeps its name.
    #[test]
    fn matrix_keeps_one_cell_per_task_arm_and_model() {
        fn cell<'a>(matrix: &'a Matrix, arm: &str) -> &'a Cell {
            &matrix.cells[&("F1-01".to_string(), arm.to_string())]
        }
        let record = |arm: &str, model: &str, seed: u64, label: u8, cost: f64| {
            serde_json::json!({
                "schema_version": RUN_RECORD_SCHEMA,
                "arm": arm,
                "seed": seed,
                "task": { "family": "F1", "instance_id": "F1-01" },
                "vs": { "label": label },
                "costs": { "api_equiv_usd": cost, "source": "provider_usage" },
                "execution": { "attempts": [{ "model_requested": model }] },
            })
            .to_string()
        };
        let text = [
            record("roko_fixed", "glm-4.7", 1, 1, 0.02),
            record("roko_fixed", "gpt-oss-120b", 1, 0, 0.01),
            record("roko_fixed", "glm-4.7", 2, 0, 0.03),
            record("cheap_direct", "gpt-oss-120b", 1, 1, 0.005),
        ]
        .join("\n");
        let matrix = Matrix::from_records(&text);
        let glm = arm_label("roko_fixed", Some("glm-4.7"));
        let oss = arm_label("roko_fixed", Some("gpt-oss-120b"));
        assert_eq!(glm, "roko_fixed[glm-4.7]");
        assert_eq!(oss, "roko_fixed[gpt-oss-120b]");
        assert_eq!(matrix.arms(), ["cheap_direct", glm.as_str(), oss.as_str()]);

        // Each model keeps its own outcome, at its lowest seed.
        assert!(cell(&matrix, &glm).passed && cell(&matrix, &glm).seed == 1);
        let oss_cell = cell(&matrix, &oss);
        assert!(!oss_cell.passed && (oss_cell.cost_usd - 0.01).abs() < 1e-12);
        assert!(cell(&matrix, "cheap_direct").passed);
        let revealed = matrix.reveal("F1-01", &oss).expect("the gpt-oss-120b cell");
        assert!(!revealed.passed);
        assert!(
            matrix.reveal("F1-01", "roko_fixed").is_none(),
            "a split arm has no cell under its own name"
        );
        let models = matrix.arm_models();
        assert_eq!(models[&glm], "glm-4.7");
        assert_eq!(models[&oss], "gpt-oss-120b");
        assert_eq!(models["cheap_direct"], "gpt-oss-120b");
    }

    #[test]
    fn replaying_the_oracle_reproduces_matrix_oracle_cpr() {
        let matrix = fixture();
        let outcomes = matrix.outcomes();
        let make = || -> Box<dyn RoutingPolicy> { Box::new(Oracle::new(&outcomes)) };
        let mut traces = Vec::new();
        let summary = replay(&matrix, &make, ORDERINGS, 7, &mut traces).expect("replay");
        // Ten solvable tasks: four at $0.012, three at $0.045 and three at $0.31.
        let expected = (4.0 * 0.012 + 3.0 * 0.045 + 3.0 * 0.31) / 10.0;
        let oracle = matrix.oracle_cpr().expect("solvable tasks");
        assert!((oracle - expected).abs() < 1e-12, "{oracle}");
        assert_eq!(summary.cpr, Some(oracle));
        assert_eq!(summary.runs, 12 * ORDERINGS as usize);
        assert_eq!(summary.resolved, 10 * ORDERINGS as usize);
    }

    #[test]
    fn two_runs_with_seed_7_write_identical_bytes() {
        let matrix = fixture();
        let snapshot = PriceSnapshot::builtin().expect("the built-in snapshot");
        let make =
            || -> Box<dyn RoutingPolicy> { Box::new(CascadePolicy::new(rungs(), &snapshot)) };
        let (mut first, mut second) = (Vec::new(), Vec::new());
        let a = replay(&matrix, &make, 3, 7, &mut first).expect("replay");
        let b = replay(&matrix, &make, 3, 7, &mut second).expect("replay");
        assert_eq!(first, second);
        assert_eq!(a, b);
        let mut other = Vec::new();
        replay(&matrix, &make, 3, 8, &mut other).expect("replay");
        assert_ne!(first, other, "another seed orders the tasks otherwise");
        let lines = String::from_utf8(first).expect("utf-8");
        assert_eq!(lines.lines().count(), 3 * 12);
    }

    #[test]
    fn bandit_feedback_reads_only_the_chosen_cells() {
        let matrix = fixture();
        let make = || -> Box<dyn RoutingPolicy> { Box::new(FrugalCascade::new(rungs())) };
        let mut traces = Vec::new();
        replay(&matrix, &make, 2, 7, &mut traces).expect("replay");
        let chosen: BTreeSet<(String, String)> = String::from_utf8(traces)
            .expect("utf-8")
            .lines()
            .map(|line| serde_json::from_str::<TraceLine>(line).expect("a trace line"))
            .flat_map(|trace| {
                let task = trace.task.clone();
                trace.arms.into_iter().map(move |arm| (task.clone(), arm))
            })
            .collect();
        assert_eq!(matrix.reads(), chosen);
        // F1-01 passes on the cheap arm, so its dearer cells were never read.
        let unread = ("F1-01".to_string(), "top_direct".to_string());
        assert!(!matrix.reads().contains(&unread));
    }

    #[test]
    fn the_held_out_family_is_never_used_for_fitting() {
        let matrix = fixture();
        let mut seen = BTreeSet::new();
        let grid = [0.75_f64, 0.80, 0.85];
        let best = cross_fit(&matrix, "F2", &grid, |target, training| {
            seen.extend(training.tasks().into_iter().map(|task| task.family));
            let arm = if *target < 0.8 {
                "cheap_direct"
            } else {
                "mid_direct"
            };
            let make = || -> Box<dyn RoutingPolicy> { Box::new(StaticArm::new(arm)) };
            replay(training, &make, 1, 7, &mut std::io::sink()).map_or(f64::INFINITY, |summary| {
                summary.cpr.unwrap_or(f64::INFINITY)
            })
        });
        assert!(best.is_some());
        assert!(!seen.contains("F2"), "{seen:?}");
        assert!(seen.contains("F1") && seen.contains("F3"));
    }

    #[test]
    fn the_self_model_policies_replay() {
        let matrix = fixture();
        let snapshot = PriceSnapshot::builtin().expect("the built-in snapshot");
        let make = || -> Box<dyn RoutingPolicy> {
            Box::new(LcbAciPolicy::new(
                rungs(),
                LcbAciConfig::default(),
                &snapshot,
            ))
        };
        let summary = replay(&matrix, &make, 2, 7, &mut std::io::sink()).expect("replay");
        assert_eq!(summary.policy, "lcb_aci");
        assert_eq!(summary.runs, 24);
        assert!(summary.attempts > 0, "{summary:?}");
        assert!(
            summary.resolved <= 20,
            "two tasks are unsolvable: {summary:?}"
        );
    }
}
