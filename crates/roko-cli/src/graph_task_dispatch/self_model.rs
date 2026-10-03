//! M3 at dispatch (S04 T10; backlog 6128): before each routed attempt, the self-model forecasts
//! the runnable rungs of the task's ladder and logs a `roko.prediction/1` row with the rung it
//! would choose beside the ladder's own. In shadow mode that is all it does: the route and the
//! `DispatchContext` stay as they were. The forecast waits, by attempt key, for the attempt's
//! verdict, which the outcome sink teaches the model (6129).
//!
//! A pinned attempt (`--model`, a task's `model_hint`) is forecast for its pinned model and
//! marked as not routable. Without a ladder rung that can run, nothing is forecast.
//!
//! In active mode (6130), while the calibration gate holds, the rung the self-model would
//! choose becomes the chain's start rung: dispatch hands it to the router, which draws it
//! through S03's route table and logs its propensity.

use std::collections::{BTreeMap, HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use roko_core::config::schema::RokoConfig;
use roko_core::config::self_model::{SelfModelConfig, SelfModelMode, SelfModelPolicy};
use roko_core::pricing_snapshot::PriceSnapshot;
use roko_learn::self_model::cascade::{
    BREAK_EVEN_MARGIN, FailureContext, StepAction, after_failure, start_rung,
};
use roko_learn::self_model::features::TaskFeatures;
use roko_learn::self_model::gate::{CalibrationGate, CalibrationWindow, GateReport, WindowOutcome};
use roko_learn::self_model::model::{MODEL_CLASS, SelfModel, StateLoad};
use roko_learn::self_model::policy::{LcbAci, LcbAciConfig, RouteAction, expected_cost};
use roko_learn::self_model::{ArmKey, CandidateForecast, LabelSource, PredictorVersion, Unit};
use roko_learn::telemetry::AttemptIdentity;
use roko_learn::telemetry::records::{
    AttemptPredictionRecord, PredictionCandidate, PredictionDecision, PredictionPredictor,
    b3_digest,
};

use super::attempt::AttemptContext;
use super::*;
use crate::dispatch::{LadderStartRung, RoutingInputs, RoutingLadder};

/// The version of the feature schema a prediction row names (`m3-features/1`).
const FEATURES_SCHEMA: u32 = 1;

/// Settled attempts a run keeps for late VS labels (6129); older ones are let go.
const SETTLED_KEPT: usize = 4_096;

/// A plan run's self-model: loaded at plan start when `[self_model] mode` is not off, and shared
/// by dispatch, which forecasts each routed attempt, and the outcome sink, which teaches it each
/// settled verdict and saves it when the run ends (6129).
#[derive(Debug)]
pub struct SelfModelRuntime {
    settings: SelfModelConfig,
    state_path: PathBuf,
    model: parking_lot::RwLock<SelfModel>,
    /// Each open attempt's forecast, by attempt key, until its verdict settles.
    forecasts: parking_lot::Mutex<HashMap<String, AttemptForecast>>,
    /// The ladder rung each chain's last forecast attempt ran on, by chain key: where a retry
    /// failed.
    last_rung: parking_lot::Mutex<HashMap<String, usize>>,
    /// Policy (a)'s adaptive margin, when it is the policy.
    lcb: parking_lot::Mutex<LcbAci>,
    /// The calibration gate's window of settled forecasts (6122).
    window: parking_lot::Mutex<CalibrationWindow>,
    /// The attempts the run settled, for late VS labels (6129).
    settled: parking_lot::Mutex<SettledUnits>,
    /// The start rung an active self-model chose for each chain, by chain key,
    /// which the chain's retries keep (6130); `None` for a chain it left to the
    /// ladder.
    chain_starts: parking_lot::Mutex<HashMap<String, Option<usize>>>,
}

/// The units a run settled, by attempt key, oldest first; past [`SETTLED_KEPT`] the oldest go.
#[derive(Debug, Default)]
struct SettledUnits {
    order: VecDeque<String>,
    units: HashMap<String, (Unit, TaskFeatures)>,
}

impl SettledUnits {
    fn insert(&mut self, unit: Unit, features: TaskFeatures) {
        let key = unit.attempt_key.attempt_key();
        if self.units.insert(key.clone(), (unit, features)).is_none() {
            self.order.push_back(key);
        }
        while self.order.len() > SETTLED_KEPT {
            if let Some(oldest) = self.order.pop_front() {
                self.units.remove(&oldest);
            }
        }
    }
}

/// One attempt's forecast, kept until its verdict settles.
#[derive(Debug, Clone, PartialEq)]
pub struct AttemptForecast {
    /// The predictor version that made it.
    pub version: PredictorVersion,
    /// The features it forecast, which the verdict teaches.
    pub features: TaskFeatures,
    /// Each candidate's forecast, cheapest rung first; one for a pinned attempt.
    pub candidates: Vec<CandidateForecast>,
    /// The candidate the self-model would choose; `None` when it would dispatch nothing.
    pub would_choose: Option<usize>,
    /// The candidate the ladder routes the attempt to without it.
    pub default: usize,
    /// The attempt's model is pinned, so no policy could route it.
    pub pinned: bool,
    /// The self-model chose the attempt's model (active mode, 6130).
    pub routed: bool,
}

impl AttemptForecast {
    /// The forecast for `model`, the model the attempt ran.
    #[must_use]
    pub fn for_model(&self, model: &str) -> Option<&CandidateForecast> {
        self.candidates
            .iter()
            .find(|candidate| candidate.arm.model == model)
    }
}

impl SelfModelRuntime {
    /// The run's self-model when `[self_model] mode` is not off: its state from `state_path`
    /// (fresh when there is none; a saved state of another version is set aside), priced at the
    /// workspace's snapshot. `None` when the mode is off or no price snapshot loads.
    #[must_use]
    pub fn load(workdir: &Path, config: &RokoConfig) -> Option<Arc<Self>> {
        let settings = config.self_model.clone();
        if !settings.enabled() {
            return None;
        }
        let snapshot = PriceSnapshot::shared(&config.pricing, workdir)?;
        let state_path = workdir.join(&settings.state_path);
        let model = match SelfModel::load_or_new(&state_path, &snapshot) {
            Ok((model, load)) => {
                if let StateLoad::SetAside(aside) = &load {
                    tracing::info!(
                        aside = %aside.display(),
                        "self-model: the saved state is of another version; starting fresh"
                    );
                }
                model
            }
            Err(error) => {
                tracing::warn!(
                    path = %state_path.display(),
                    %error,
                    "self-model: the saved state did not load; starting fresh"
                );
                SelfModel::new(&snapshot)
            }
        };
        tracing::info!(
            mode = mode_name(settings.mode),
            version = %model.version,
            outcomes = model.outcomes,
            "self-model loaded"
        );
        Some(Arc::new(Self::new(settings, state_path, model)))
    }

    /// A runtime around `model`, saved to `state_path`.
    #[must_use]
    pub fn new(settings: SelfModelConfig, state_path: PathBuf, model: SelfModel) -> Self {
        let lcb = LcbAci::new(LcbAciConfig {
            target: settings.target,
            ..LcbAciConfig::default()
        });
        let window = CalibrationWindow::new(model.version.clone());
        Self {
            settings,
            state_path,
            model: parking_lot::RwLock::new(model),
            forecasts: parking_lot::Mutex::new(HashMap::new()),
            last_rung: parking_lot::Mutex::new(HashMap::new()),
            lcb: parking_lot::Mutex::new(lcb),
            window: parking_lot::Mutex::new(window),
            settled: parking_lot::Mutex::new(SettledUnits::default()),
            chain_starts: parking_lot::Mutex::new(HashMap::new()),
        }
    }

    /// `[self_model]` as the run loaded it.
    #[must_use]
    pub fn settings(&self) -> &SelfModelConfig {
        &self.settings
    }

    /// The predictor version forecasts are made with now.
    #[must_use]
    pub fn version(&self) -> PredictorVersion {
        self.model.read().version.clone()
    }

    /// Labelled outcomes the model has learned.
    #[must_use]
    pub fn outcomes(&self) -> usize {
        self.model.read().outcomes
    }

    /// Keep `forecast` for the attempt `attempt_key` until its verdict settles.
    pub fn remember(&self, attempt_key: impl Into<String>, forecast: AttemptForecast) {
        self.forecasts.lock().insert(attempt_key.into(), forecast);
    }

    /// The forecast of the attempt `attempt_key`, taken out of the cache.
    pub fn take_forecast(&self, attempt_key: &str) -> Option<AttemptForecast> {
        self.forecasts.lock().remove(attempt_key)
    }

    /// Teach the model `unit`, whose features were `features`, with weight `w`.
    pub fn observe(&self, unit: &Unit, features: &TaskFeatures, w: f64) {
        self.model.write().observe_with(unit, features, w);
    }

    /// Teach the model a settled attempt it forecast (6129): `unit` under the forecast's
    /// features, and the calibration window with the forecast for the model that ran. The unit
    /// stays for a late VS label.
    pub fn settle(&self, unit: Unit, forecast: AttemptForecast) {
        let w = unit.label.weight;
        self.observe(&unit, &forecast.features, w);
        if let Some(candidate) = forecast.for_model(&unit.arm.model) {
            let outcome = WindowOutcome {
                p: candidate.p_gate,
                y: unit.label.y_gate == Some(true),
                w,
                routed: forecast.routed,
            };
            self.record_outcome(&forecast.version, outcome, candidate.p_fg);
        }
        self.settled.lock().insert(unit, forecast.features);
    }

    /// A late VS label of a settled attempt (S05's `vs.label`, weight 1/π): only the
    /// false-green head learns it. `false` when the run settled no such attempt.
    pub fn observe_label(
        &self,
        attempt_key: &str,
        y_vs: bool,
        weight: f64,
        source: LabelSource,
    ) -> bool {
        let Some((mut unit, features)) = self.settled.lock().units.get(attempt_key).cloned() else {
            return false;
        };
        unit.label.y_vs = Some(y_vs);
        unit.label.weight = weight;
        unit.label.source = source;
        self.model.write().observe_vs(&unit, &features, weight);
        true
    }

    /// Add a settled forecast to the calibration gate's window, under the version that made
    /// it, and step policy (a)'s margin with the outcome.
    pub fn record_outcome(&self, version: &PredictorVersion, outcome: WindowOutcome, p_fg: f64) {
        self.window.lock().push(version, outcome);
        self.lcb.lock().update(None, outcome.y, p_fg);
    }

    /// The calibration gate's report on the window (6122).
    #[must_use]
    pub fn gate(&self) -> GateReport {
        let gate = CalibrationGate {
            target: self.settings.target,
        };
        gate.evaluate(&self.window.lock())
    }

    /// Save the model's state, atomically.
    pub fn save(&self) -> std::io::Result<()> {
        self.model.read().save(&self.state_path)
    }

    /// Forecast `candidates` for the attempt `identity`, decide as the configured policy would,
    /// keep the forecast for the attempt's verdict, and return the prediction row with the
    /// candidate the self-model would choose.
    fn predict(
        &self,
        identity: &AttemptIdentity,
        features: TaskFeatures,
        candidates: &Candidates,
        retries_left: u32,
    ) -> (AttemptPredictionRecord, Option<usize>) {
        let model = self.model.read();
        let forecasts = model.forecast(&features, &candidates.arms);
        let (would_choose, action) = self.decide(identity, &forecasts, candidates, retries_left);
        let arms: Vec<String> = forecasts
            .iter()
            .map(|forecast| forecast.arm.to_string())
            .collect();
        let decision = PredictionDecision {
            would_choose: would_choose.and_then(|index| arms.get(index).cloned()),
            default: arms.get(candidates.default).cloned(),
            action: action.to_string(),
        };
        let features_json = serde_json::to_vec(&features).unwrap_or_default();
        let predictor = PredictionPredictor {
            version: model.version.to_string(),
            class: MODEL_CLASS.to_string(),
            mode: mode_name(self.settings.mode).to_string(),
            trained_on_n: model.outcomes as u64,
            features_schema: FEATURES_SCHEMA,
            features_hash: b3_digest(&features_json),
        };
        let rows = forecasts.iter().map(prediction_candidate).collect();
        let record = AttemptPredictionRecord::new(identity.clone(), predictor, rows, decision)
            .with_price_snapshot_id(model.price_snapshot_id.clone());
        if let Some(&rung) = candidates.rungs.get(candidates.default) {
            self.last_rung
                .lock()
                .insert(identity.chain_key.clone(), rung);
        }
        let forecast = AttemptForecast {
            version: model.version.clone(),
            features,
            candidates: forecasts,
            would_choose,
            default: candidates.default,
            pinned: candidates.pinned,
            routed: false,
        };
        self.remember(identity.attempt_key.clone(), forecast);
        (record, would_choose)
    }

    /// The start rung, an index on the role's ladder, that an active self-model routes the
    /// attempt `identity` to (6130): on a chain's first attempt the candidate it would choose,
    /// when [`active_start`] lets it act, and on a retry the chain's start again, which the
    /// ladder climbs from. `None` otherwise: the ladder routes as it does by itself.
    fn active_rung(
        &self,
        identity: &AttemptIdentity,
        candidates: &Candidates,
        would_choose: Option<usize>,
    ) -> Option<usize> {
        if self.settings.mode != SelfModelMode::Active {
            return None;
        }
        if identity.attempt > 1 {
            return self
                .chain_starts
                .lock()
                .get(&identity.chain_key)
                .copied()
                .flatten();
        }
        let gate = self.gate();
        let (pinned, default) = (candidates.pinned, candidates.default);
        let start = active_start(&self.settings, &gate, pinned, would_choose, default)
            .and_then(|position| candidates.rungs.get(position).copied());
        let chain = identity.chain_key.clone();
        self.chain_starts.lock().insert(chain.clone(), start);
        if let Some(rung) = start {
            self.last_rung.lock().insert(chain, rung);
        }
        start
    }

    /// The candidate the policy would choose, and its action's name.
    fn decide(
        &self,
        identity: &AttemptIdentity,
        forecasts: &[CandidateForecast],
        candidates: &Candidates,
        retries_left: u32,
    ) -> (Option<usize>, &'static str) {
        if candidates.pinned {
            return (Some(candidates.default), "pinned");
        }
        match self.settings.policy {
            SelfModelPolicy::Static => (Some(candidates.default), "dispatch"),
            SelfModelPolicy::LcbAci => {
                let recovery = forecasts.iter().map(expected_cost).fold(0.0, f64::max);
                let choice = self.lcb.lock().choose(forecasts, None, None, recovery);
                match choice.action {
                    RouteAction::Dispatch { arm, .. } => {
                        let index = forecasts.iter().position(|forecast| forecast.arm == arm);
                        (index, "dispatch")
                    }
                    RouteAction::RefineSpec => (None, "refine_spec"),
                    RouteAction::Abandon => (None, "abandon"),
                }
            }
            SelfModelPolicy::Cascade => {
                let failed_on = self.last_rung.lock().get(&identity.chain_key).copied();
                match failed_on.filter(|_| identity.attempt > 1) {
                    None => {
                        // Decision 6101 B1 lets a task start below its tier's rung.
                        let floor = if self.settings.allow_downward_start {
                            0
                        } else {
                            candidates.default
                        };
                        let start = start_rung(forecasts, BREAK_EVEN_MARGIN);
                        (start.map(|start| start.max(floor)), "start")
                    }
                    Some(rung) => {
                        let current = candidates
                            .rungs
                            .iter()
                            .position(|&index| index == rung)
                            .unwrap_or(candidates.default);
                        let context = FailureContext {
                            current,
                            climbs: candidates.step,
                            retries_left,
                            spec_score: None,
                            skip_allowed: self.settings.allow_rung_skip,
                        };
                        match after_failure(forecasts, &context, &|_| None) {
                            StepAction::Retry => (Some(current), "retry"),
                            StepAction::Climb { to } => (Some(to), "climb"),
                            StepAction::Skip { to } => (Some(to), "skip"),
                            StepAction::RefineSpec => (None, "refine_spec"),
                            StepAction::Abandon => (None, "abandon"),
                        }
                    }
                }
            }
        }
    }
}

/// What an attempt's forecast compares: the runnable rungs of its role's ladder, cheapest first,
/// or the one model a pin names.
#[derive(Debug, Clone, PartialEq)]
struct Candidates {
    /// Each candidate's arm.
    arms: Vec<ArmKey>,
    /// Each candidate's index on its role's ladder; empty for a pinned attempt.
    rungs: Vec<usize>,
    /// The candidate the ladder routes the attempt to without the self-model.
    default: usize,
    /// Rungs the task has climbed above its start rung.
    step: u32,
    /// A pin fixes the model.
    pinned: bool,
}

impl Candidates {
    /// The candidates of the attempt `inputs` describe: its pinned model, else the runnable
    /// rungs of its role on `ladder`. `None` without either.
    fn of(
        inputs: &RoutingInputs,
        ladder: Option<&RoutingLadder>,
        config: &RokoConfig,
    ) -> Option<Self> {
        let pin = inputs
            .force_backend
            .clone()
            .or_else(|| inputs.task_model_hint.clone());
        if let Some(model) = pin {
            return Some(Self {
                arms: vec![arm_of(config, &model)],
                rungs: Vec::new(),
                default: 0,
                step: 0,
                pinned: true,
            });
        }
        let ladder = ladder?;
        let role = inputs.role.as_str();
        let first = ladder.rung_name(role, 0)?;
        let lowest = ladder.start(role, inputs.task_tier, Some(first))?;
        let above = ladder.rung_models_above(role, lowest.index);
        let rungs: Vec<LadderStartRung> = std::iter::once(lowest).chain(above).collect();
        let start = ladder.start(role, inputs.task_tier, inputs.task_rung.as_deref())?;
        let routed = ladder.climb(role, start, inputs.ladder_step);
        let default = rungs.iter().position(|rung| rung.index == routed.index)?;
        Some(Self {
            arms: rungs
                .iter()
                .map(|rung| arm_of(config, &rung.model))
                .collect(),
            rungs: rungs.iter().map(|rung| rung.index).collect(),
            default,
            step: inputs.ladder_step,
            pinned: false,
        })
    }
}

impl GraphTaskDispatcher {
    /// M3's hook before routing (6128): forecast the attempt's candidates, log the prediction
    /// with the rung the self-model would choose beside the ladder's, and keep the forecast for
    /// the attempt's verdict. It runs only when `[self_model] mode` is not off, and it never
    /// changes the route.
    pub(super) fn forecast_attempt(
        &self,
        spec: &TaskExecutionSpec,
        task: &TaskDef,
        dispatch_ctx: &DispatchContext,
        attempt: &AttemptContext,
    ) -> Option<usize> {
        let runtime = self.feedback.self_model.as_deref()?;
        let inputs = RoutingInputs::from_task(task, dispatch_ctx);
        let ladder = self.factory.dispatcher().routing_ladder();
        let Some(candidates) = Candidates::of(&inputs, ladder, &self.config) else {
            tracing::debug!(
                plan_id = %spec.plan_id,
                task_id = %task.id,
                "self-model: no ladder rung can run and no model is pinned; nothing to forecast"
            );
            return None;
        };
        let task_key = format!("{}/{}", spec.plan_id, task.id);
        let used = self.attempt_in_run(&task_key);
        let retries_left = spec.max_retries.saturating_sub(used);
        let features = task_features(task, &inputs);
        let identity = attempt.identity();
        let (prediction, would_choose) =
            runtime.predict(identity, features, &candidates, retries_left);
        attempt.record_prediction(prediction);
        runtime.active_rung(identity, &candidates, would_choose)
    }
}

/// Whether an active self-model routes an attempt, and to which candidate (6130): only in active
/// mode, for an attempt no pin fixes, while the calibration gate is eligible and its breaker has
/// not tripped. A start below the ladder's own needs decision 6101's B1.
pub(crate) fn active_start(
    settings: &SelfModelConfig,
    gate: &GateReport,
    pinned: bool,
    would_choose: Option<usize>,
    default: usize,
) -> Option<usize> {
    let acts = settings.mode == SelfModelMode::Active
        && !pinned
        && gate.eligible
        && !gate.breaker_tripped;
    let choice = would_choose.filter(|_| acts)?;
    (settings.allow_downward_start || choice >= default).then_some(choice)
}

/// What the self-model knows of `task` before its attempt runs. Plan tasks have no benchmark
/// family, so the tier stands in, as it does for the units the model learns (S04 §4.2).
fn task_features(task: &TaskDef, inputs: &RoutingInputs) -> TaskFeatures {
    TaskFeatures {
        title: task.title.clone(),
        description: task.description.clone().unwrap_or_default(),
        verify_commands: task
            .verify
            .iter()
            .map(|step| step.command.clone())
            .collect(),
        family: task.tier.clone(),
        tier: task.tier.clone(),
        role: inputs.role.clone(),
        attempt: inputs.attempt + 1,
        has_prior_failure: inputs.attempt > 0,
        error_class: None,
        spec: BTreeMap::new(),
    }
}

/// The arm that runs `model`: roko's harness on the model's `[models.*]` provider.
fn arm_of(config: &RokoConfig, model: &str) -> ArmKey {
    let models = &config.models;
    let profile = models
        .get(model)
        .or_else(|| models.values().find(|profile| profile.slug == model));
    let provider = profile.map_or("unknown", |profile| profile.provider.as_str());
    ArmKey::roko(provider, model)
}

/// A forecast as a prediction row's candidate.
fn prediction_candidate(forecast: &CandidateForecast) -> PredictionCandidate {
    PredictionCandidate {
        arm: forecast.arm.to_string(),
        p_gate: forecast.p_gate,
        p_fg: forecast.p_fg,
        p_vs: forecast.p_vs,
        p_vs_lcb: forecast.p_vs_lcb,
        sd: None,
        cost_q50: forecast.cost_q50,
        cost_q90: forecast.cost_q90,
        lat_q50_s: forecast.lat_q50_s,
        lat_q90_s: forecast.lat_q90_s,
        p_retry: None,
    }
}

/// The mode's name in `[self_model] mode`.
const fn mode_name(mode: SelfModelMode) -> &'static str {
    match mode {
        SelfModelMode::Off => "off",
        SelfModelMode::Shadow => "shadow",
        SelfModelMode::Active => "active",
    }
}

#[cfg(test)]
mod tests {
    use roko_core::config::routing::LadderRung;

    use super::*;
    use crate::graph_task_dispatch::tests::{
        VERIFY_PROVIDER, jsonl_rows_where, make_spec, make_test_dispatcher, model, no_auto_fix,
    };

    const RUN: &str = "graph-self-model-run";

    /// Unpinned tasks the shadow test dispatches, one attempt each, before one pinned task.
    const TASKS: usize = 20;

    /// The pinned task: the fixture's own, whose `model_hint` names `stream-model`.
    const PINNED: &str = "T-STREAM";

    /// Two ladder rungs on the scripted provider, and `[self_model] mode`.
    fn ladder(mode: SelfModelMode) -> impl FnOnce(&mut RokoConfig) {
        move |config| {
            no_auto_fix(config);
            let models = [
                ("cheap-model", "claude-haiku-4-5"),
                ("stream-model", "claude-sonnet-4-6"),
            ];
            for (key, slug) in models {
                config
                    .models
                    .insert(key.to_string(), model("stream-cli", slug, None));
            }
            let rung = |name: &str, model: &str| LadderRung {
                name: name.to_string(),
                model: model.to_string(),
            };
            config.routing.ladder.rungs =
                vec![rung("cheap", "cheap-model"), rung("strong", "stream-model")];
            config.self_model.mode = mode;
        }
    }

    /// Dispatch [`TASKS`] unpinned tasks and the pinned one with `[self_model] mode = mode`;
    /// each task's dispatched model, and the run's prediction rows.
    async fn run(mode: SelfModelMode) -> (BTreeMap<String, String>, Vec<serde_json::Value>) {
        let temp = tempfile::tempdir().expect("tempdir");
        let roko = temp.path().join(".roko");
        let runtime = (mode != SelfModelMode::Off).then(|| {
            let settings = SelfModelConfig {
                mode,
                ..SelfModelConfig::default()
            };
            let snapshot = PriceSnapshot::builtin().expect("the built-in snapshot");
            let state = roko.join("learn/self-model/state-v1.json");
            Arc::new(SelfModelRuntime::new(
                settings,
                state,
                SelfModel::new(&snapshot),
            ))
        });
        let feedback = GraphFeedbackContext {
            runs_dir: Some(roko.join("runs")),
            self_model: runtime.clone(),
            ..GraphFeedbackContext::default()
        };
        let (dispatcher, pinned) =
            make_test_dispatcher(&temp, VERIFY_PROVIDER, ladder(mode), feedback).await;
        let ctx = CellContext::new().with_run_id(RUN.to_string());
        let mut tasks: Vec<TaskDef> = (1..=TASKS)
            .map(|index| TaskDef {
                id: format!("T{index:02}"),
                model_hint: None,
                ..pinned.clone()
            })
            .collect();
        tasks.push(pinned);
        for task in &tasks {
            dispatcher
                .dispatch(&make_spec(task), Vec::new(), &ctx)
                .await
                .expect("the attempt completes");
        }
        drop(dispatcher);

        let run_dir = roko.join("runs").join(RUN);
        let verdicts = jsonl_rows_where(&run_dir.join("attempts.jsonl"), tasks.len(), |row| {
            row["schema_version"] == "roko.verdict/1"
        })
        .await;
        let models = verdicts
            .iter()
            .map(|row| {
                let task = row["task_id"].as_str().unwrap_or_default().to_string();
                // The route's model key, as arms name it (`model_dispatched`
                // is the provider slug it resolves to).
                let model = &row["executed"]["model_requested"];
                (task, model.as_str().unwrap_or_default().to_string())
            })
            .collect();
        let predictions_path = run_dir.join("predictions.jsonl");
        let Some(runtime) = runtime else {
            assert!(!predictions_path.exists(), "mode = off forecasts nothing");
            return (models, Vec::new());
        };
        let expected = (tasks.len() * 95).div_ceil(100);
        let predictions = jsonl_rows_where(&predictions_path, expected, |row| {
            row["schema_version"] == "roko.prediction/1"
        })
        .await;
        // Each forecast waits, by attempt key, for its attempt's verdict (6129).
        for row in &predictions {
            let key = row["attempt_key"].as_str().expect("an attempt key");
            assert!(runtime.take_forecast(key).is_some(), "{key}");
        }
        (models, predictions)
    }

    /// 6128: in shadow mode at least 95% of the attempts get a prediction row naming both rungs
    /// and the ladder's own pick, a pinned one names its model alone, and every attempt runs the
    /// model it runs with the self-model off.
    #[tokio::test]
    async fn self_model_shadow_writes_predictions_and_keeps_routing() {
        let (off, none) = run(SelfModelMode::Off).await;
        assert!(none.is_empty());
        let (shadow, predictions) = run(SelfModelMode::Shadow).await;
        assert_eq!(off.len(), TASKS + 1, "{off:?}");
        assert_eq!(shadow, off, "shadow mode never changes the route");

        assert!(predictions.len() * 100 >= (TASKS + 1) * 95);
        for row in &predictions {
            assert_eq!(row["precedes"], "route");
            assert_eq!(row["predictor"]["mode"], "shadow");
            let task = row["task_id"].as_str().expect("a task id");
            let candidates = row["candidates"].as_array().expect("candidates");
            let pinned = task == PINNED;
            assert_eq!(candidates.len(), if pinned { 1 } else { 2 }, "{row}");
            assert_eq!(row["decision"]["action"] == "pinned", pinned, "{row}");
            // An arm reads `roko/<provider>/<model>@<effort>#V<depth>`.
            let default = row["decision"]["default"].as_str().expect("a default arm");
            assert!(
                default.contains(&format!("/{}@", off[task])),
                "{default} vs {}",
                off[task]
            );
        }
    }
}
