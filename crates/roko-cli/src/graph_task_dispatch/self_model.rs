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
//!
//! After a gate pass (6132), policy (b) weighs the attempt's false-green risk r = p_fg: it asks
//! DP3 (`verify_depth`) for the deepest verify depth d* whose check pays, r·L_fg·d_j > c_j, and
//! only at the deepest depth with r above r_max rejects the pass so that a stronger model
//! retries the task. An active self-model acts on the chains it started; in shadow mode the
//! step is only logged. A pass that stands exports r as `risk_fg` for S05's audit tilt.
//!
//! A refine-spec or abandon forecast (6133) becomes a dashboard diagnosis and an event-log
//! entry, and a refine request also a `spec.refine_requested` record in the run's spec ledger
//! for S07. The self-model never edits a spec or drops a task: the attempt still runs on the
//! ladder's choice. S07's plan-load gate reads the record in later runs (gap-2b0575), and under
//! `enforce` stops the task until its spec is refined when the request came from a self-model
//! that routes.

use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use roko_core::audit_types::VerifyDepth;
use roko_core::config::schema::RokoConfig;
use roko_core::config::self_model::{SelfModelConfig, SelfModelMode, SelfModelPolicy};
use roko_core::dashboard_snapshot::{DiagnosisSeverity, DiagnosisSummary};
use roko_core::pricing_snapshot::PriceSnapshot;
use roko_learn::self_model::baselines::K_MAX;
use roko_learn::self_model::cascade::{
    BREAK_EVEN_MARGIN, DepthOption, FALSE_GREEN_RISK_MAX, FailureContext, PassAction, StepAction,
    after_failure, after_pass, start_rung,
};
use roko_learn::self_model::features::TaskFeatures;
use roko_learn::self_model::gate::{CalibrationGate, CalibrationWindow, GateReport, WindowOutcome};
use roko_learn::self_model::logit::FALSE_GREEN_PRIOR;
use roko_learn::self_model::model::{MODEL_CLASS, SelfModel, StateLoad};
use roko_learn::self_model::policy::{LcbAci, LcbAciConfig, P_ABANDON, RouteAction, expected_cost};
use roko_learn::self_model::spec_features::{SPEC_RECORDS_FILE, SpecFeatureIndex, SpecVector};
use roko_learn::self_model::{ArmKey, CandidateForecast, LabelSource, PredictorVersion, Unit};
use roko_learn::telemetry::records::{
    AttemptPredictionRecord, PredictionCandidate, PredictionDecision, PredictionPredictor,
    b3_digest,
};
use roko_learn::telemetry::{AttemptIdentity, AttemptVerdictRecord};

use super::attempt::AttemptContext;
use super::*;
use crate::dispatch::{LadderStartRung, RoutingInputs, RoutingLadder};

/// The version of the feature schema a prediction row names (`m3-features/1`).
const FEATURES_SCHEMA: u32 = 1;

/// Settled attempts a run keeps for late VS labels (6129); older ones are let go.
const SETTLED_KEPT: usize = 4_096;

/// The verify depths policy (b) may ask DP3 for after a pass (6132), each with its catch rate
/// d_j, the prior 0.5 until S05's audits measure it, and its cost c_j in USD: V1's and V2's
/// checks take machine time only, V3 adds a hidden suite a model writes, and V4 mutation and a
/// review by a model of another family.
const DEPTH_OPTIONS: [DepthOption; 4] = [
    DepthOption {
        depth: 1,
        catch_rate: 0.5,
        cost_usd: 0.005,
    },
    DepthOption {
        depth: 2,
        catch_rate: 0.5,
        cost_usd: 0.02,
    },
    DepthOption {
        depth: 3,
        catch_rate: 0.5,
        cost_usd: 0.08,
    },
    DepthOption {
        depth: 4,
        catch_rate: 0.5,
        cost_usd: 0.20,
    },
];

/// A false green's loss L_fg, in the attempt's expected cost on the model that ran: S04
/// §4.10's five task costs.
const FALSE_GREEN_LOSS: f64 = 5.0;

/// The spec feature that holds S07's score of the task's spec over 1 (3240), which both
/// policies compare with s_min.
const SPEC_SCORE: &str = "spec_score";

/// The action a policy names when the task's spec should be refined before it runs (6133).
const REFINE_SPEC: &str = "refine_spec";

/// The action a policy names when no rung is likely to pass the task (6133).
const ABANDON: &str = "abandon";

/// The event that hands a refine request to S07's plan-load gate, in the run's spec ledger
/// and its event log (6133).
const SPEC_REFINE_EVENT: &str = roko_gate::spec_quality::REFINE_REQUESTED;

/// A plan run's self-model: loaded at plan start when `[self_model] mode` is not off, and shared
/// by dispatch, which forecasts each routed attempt, and the outcome sink, which teaches it each
/// settled verdict and saves it when the run ends (6129).
#[derive(Debug)]
pub struct SelfModelRuntime {
    /// `[self_model]` as the plan run loaded it, fixed for the runtime's life: nothing in roko
    /// writes the section, and a person's change to the mode takes effect at the next run
    /// (bug-78e5ce). The calibration gate's breaker is what each step re-checks live.
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
    /// Each chain's candidates and features as its last forecast saw them, for
    /// the re-forecast after a failure (6131).
    chain_plans: parking_lot::Mutex<HashMap<String, (Candidates, TaskFeatures)>>,
    /// Whether the self-model made each chain's last climb (6131), so the next
    /// attempt's ladder record can say so.
    early_climbs: parking_lot::Mutex<HashMap<String, bool>>,
    /// Each run's spec records (3240), by run id, with the size of the file read.
    spec_indexes: parking_lot::Mutex<HashMap<String, (u64, SpecFeatureIndex)>>,
    /// P(false green) of each chain's last pass that stood, by chain key: `risk_fg` for S05's
    /// audit tilt (6132).
    risks: parking_lot::Mutex<HashMap<String, f64>>,
    /// The attempts whose pass an active self-model rejected as suspicious at the deepest
    /// depth, by attempt key, so that their failure climbs a rung (6132).
    suspicious: parking_lot::Mutex<HashSet<String>>,
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
            chain_plans: parking_lot::Mutex::new(HashMap::new()),
            early_climbs: parking_lot::Mutex::new(HashMap::new()),
            spec_indexes: parking_lot::Mutex::new(HashMap::new()),
            risks: parking_lot::Mutex::new(HashMap::new()),
            suspicious: parking_lot::Mutex::new(HashSet::new()),
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

    /// The model's forecast of `arms` for a task with `features`.
    #[must_use]
    pub fn forecast(&self, features: &TaskFeatures, arms: &[ArmKey]) -> Vec<CandidateForecast> {
        self.model.read().forecast(features, arms)
    }

    /// The features of the run's most recently settled attempts, at most `n`, oldest first:
    /// the recent task mix M1's move priors forecast (8122).
    #[must_use]
    pub fn recent_features(&self, n: usize) -> Vec<TaskFeatures> {
        let settled = self.settled.lock();
        let skip = settled.order.len().saturating_sub(n);
        settled
            .order
            .iter()
            .skip(skip)
            .filter_map(|key| settled.units.get(key))
            .map(|(_, features)| features.clone())
            .collect()
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

    /// Whether the self-model routes now: active, with its calibration gate eligible and its
    /// breaker untripped, as [`active_start`] needs.
    fn routes(&self) -> bool {
        let gate = self.gate();
        self.settings.mode == SelfModelMode::Active && gate.eligible && !gate.breaker_tripped
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
        let spec_score = features.spec.get(SPEC_SCORE).copied();
        let (would_choose, action) =
            self.decide(identity, &forecasts, candidates, retries_left, spec_score);
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
        let plan = (candidates.clone(), features.clone());
        self.chain_plans
            .lock()
            .insert(identity.chain_key.clone(), plan);
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

    /// The spec vector of `task_id` in `plan_id` from the spec records of the run `run_id` in
    /// `run_dir` (3240): read again whenever the file has grown, since each plan of a run
    /// appends its records before its first task starts. `None` without a record.
    fn spec_vector(
        &self,
        run_dir: &Path,
        run_id: &str,
        plan_id: &str,
        task_id: &str,
    ) -> Option<SpecVector> {
        let metadata = std::fs::metadata(run_dir.join(SPEC_RECORDS_FILE)).ok()?;
        let size = metadata.len();
        let mut indexes = self.spec_indexes.lock();
        let stale = indexes.get(run_id).is_none_or(|(read, _)| *read != size);
        if stale {
            let index = SpecFeatureIndex::read_run(run_dir).ok()?;
            indexes.insert(run_id.to_string(), (size, index));
        }
        indexes.get(run_id)?.1.get(plan_id, task_id).cloned()
    }

    /// The self-model's step after an agent-blamed failure on ladder rung `rung` of a chain it
    /// started (6131): it re-forecasts the chain's rungs knowing the attempt failed, with the
    /// failure's class, and takes the cheapest step to VS (S04 §4.4), at most `K_MAX` climbs.
    /// `None` outside active mode, for a chain the ladder started by itself (held out, or never
    /// the self-model's), and once the breaker has tripped: the two-failure rule stands. The
    /// mode is the run's, fixed at its start; the breaker is re-checked here, live.
    fn post_failure_step(
        &self,
        chain_key: &str,
        rung: usize,
        climbs: u32,
        retries_left: u32,
        error_class: Option<String>,
    ) -> Option<StepAction> {
        if self.settings.mode != SelfModelMode::Active || self.gate().breaker_tripped {
            return None;
        }
        // Only a chain whose start rung the self-model chose.
        self.chain_starts.lock().get(chain_key).copied().flatten()?;
        let (candidates, mut features) = self.chain_plans.lock().get(chain_key).cloned()?;
        let current = candidates.rungs.iter().position(|&index| index == rung)?;
        features.attempt = features.attempt.saturating_add(1);
        features.has_prior_failure = true;
        features.error_class = error_class;
        let forecasts = self.model.read().forecast(&features, &candidates.arms);
        let context = FailureContext {
            current,
            climbs,
            retries_left,
            spec_score: features.spec.get(SPEC_SCORE).copied(),
            skip_allowed: self.settings.allow_rung_skip,
        };
        Some(after_failure(&forecasts, &context, &|_| None))
    }

    /// P(false green) of the chain `chain_key`'s last pass that stood, exported as `risk_fg` for
    /// S05's audit tilt (6132), which S05 keeps at 0 until M3 has 50 audited labels.
    #[must_use]
    pub fn risk_fg(&self, chain_key: &str) -> Option<f64> {
        self.risks.lock().get(chain_key).copied()
    }

    /// The candidates' forecasts of the open attempt `attempt_key`, whose forecast stays cached
    /// for its verdict.
    fn open_candidates(&self, attempt_key: &str) -> Option<Vec<CandidateForecast>> {
        self.forecasts
            .lock()
            .get(attempt_key)
            .map(|forecast| forecast.candidates.clone())
    }

    /// Whether the self-model's step after a pass acts on the chain `chain_key` (6132): policy
    /// (b) in active mode, on a chain whose start rung it chose, until the breaker trips.
    /// Otherwise the step is only logged.
    fn acts_after_pass(&self, chain_key: &str) -> bool {
        let started = self.chain_starts.lock().get(chain_key).copied().flatten();
        self.settings.mode == SelfModelMode::Active
            && self.settings.policy == SelfModelPolicy::Cascade
            && started.is_some()
            && !self.gate().breaker_tripped
    }

    /// Whether the chain `chain_key` can climb a rung from its candidate `current` (6132): a
    /// rung above it, fewer than `K_MAX` climbs so far, and an attempt left.
    fn can_climb(&self, chain_key: &str, current: usize, retries_left: u32) -> bool {
        let plans = self.chain_plans.lock();
        plans.get(chain_key).is_some_and(|(candidates, _)| {
            !candidates.pinned
                && current + 1 < candidates.rungs.len()
                && candidates.step < K_MAX
                && retries_left > 0
        })
    }

    /// The climb the failure of the attempt `attempt_key` on ladder rung `rung` of the chain
    /// `chain_key` earns when the self-model rejected its pass as suspicious (6132); `None` for
    /// any other failure.
    fn escalation_step(
        &self,
        attempt_key: &str,
        chain_key: &str,
        rung: usize,
    ) -> Option<StepAction> {
        if !self.suspicious.lock().remove(attempt_key) {
            return None;
        }
        let plans = self.chain_plans.lock();
        let (candidates, _) = plans.get(chain_key)?;
        let current = candidates.rungs.iter().position(|&index| index == rung)?;
        let to = current + 1;
        (to < candidates.rungs.len()).then_some(StepAction::Climb { to })
    }

    /// The candidate the policy would choose, and its action's name. `spec_score`, S07's score
    /// of the task's spec over 1, lets either policy ask for a clearer spec (6133).
    fn decide(
        &self,
        identity: &AttemptIdentity,
        forecasts: &[CandidateForecast],
        candidates: &Candidates,
        retries_left: u32,
        spec_score: Option<f64>,
    ) -> (Option<usize>, &'static str) {
        if candidates.pinned {
            return (Some(candidates.default), "pinned");
        }
        match self.settings.policy {
            SelfModelPolicy::Static => (Some(candidates.default), "dispatch"),
            SelfModelPolicy::LcbAci => {
                let recovery = forecasts.iter().map(expected_cost).fold(0.0, f64::max);
                let policy = self.lcb.lock();
                let choice = policy.choose(forecasts, None, spec_score, recovery);
                match choice.action {
                    RouteAction::Dispatch { arm, .. } => {
                        let index = forecasts.iter().position(|forecast| forecast.arm == arm);
                        (index, "dispatch")
                    }
                    RouteAction::RefineSpec => (None, REFINE_SPEC),
                    RouteAction::Abandon => (None, ABANDON),
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
                            spec_score,
                            skip_allowed: self.settings.allow_rung_skip,
                        };
                        match after_failure(forecasts, &context, &|_| None) {
                            StepAction::Retry => (Some(current), "retry"),
                            StepAction::Climb { to } => (Some(to), "climb"),
                            StepAction::Skip { to } => (Some(to), "skip"),
                            StepAction::RefineSpec => (None, REFINE_SPEC),
                            StepAction::Abandon => (None, ABANDON),
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

/// What the self-model makes of one attempt's gate pass (6132).
struct PostPass<'a> {
    /// The run's self-model.
    runtime: &'a SelfModelRuntime,
    /// The attempt's chain key.
    chain: String,
    /// The candidate that ran, by its place among the chain's candidates.
    current: usize,
    /// r, P(false green) of the pass.
    risk_fg: f64,
    /// Policy (b)'s step after the pass.
    action: PassAction,
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
        let identity = attempt.identity();
        let mut features = task_features(task, &inputs);
        // S07's spec features join the attempt by plan and task (3240).
        if let Some(runs) = &self.feedback.runs_dir {
            let run_dir = runs.join(&identity.run_id);
            let vector = runtime.spec_vector(&run_dir, &identity.run_id, &spec.plan_id, &task.id);
            features.spec = vector.unwrap_or_default();
        }
        let spec_score = features.spec.get(SPEC_SCORE).copied();
        let (prediction, would_choose) =
            runtime.predict(identity, features, &candidates, retries_left);
        // M3 never edits a spec or drops a task: a person or S07 acts on its request (6133).
        self.report_self_model_action(spec, task, identity, &prediction, spec_score);
        attempt.record_prediction(prediction);
        runtime.active_rung(identity, &candidates, would_choose)
    }

    /// Surface a refine-spec or abandon forecast for the attempt `identity` of `task` (6133),
    /// as the ladder surfaces `ladder_exhausted`: a dashboard diagnosis and an event-log entry,
    /// and for a refine request also a `spec.refine_requested` record in the run's spec ledger,
    /// for S07's plan-load gate. Any other action reports nothing.
    fn report_self_model_action(
        &self,
        spec: &TaskExecutionSpec,
        task: &TaskDef,
        identity: &AttemptIdentity,
        prediction: &AttemptPredictionRecord,
        spec_score: Option<f64>,
    ) {
        let refine = match prediction.decision.action.as_str() {
            REFINE_SPEC => true,
            ABANDON => false,
            _ => return,
        };
        let best = prediction
            .candidates
            .iter()
            .map(|candidate| candidate.p_vs)
            .fold(0.0, f64::max);
        let score = spec_score.map_or_else(|| "unknown".to_string(), |score| format!("{score:.2}"));
        let (plan_id, task_id) = (&spec.plan_id, &task.id);
        let (kind, event, subject, detail, suggested_action, message) = if refine {
            self.record_refine_request(identity, spec_score, best);
            (
                "self_model_refine",
                SPEC_REFINE_EVENT,
                format!("{task_id} needs a clearer spec"),
                format!(
                    "The self-model forecasts that no cheap rung reaches the success target on \
                     task `{task_id}` of plan `{plan_id}` with its spec as written (spec score \
                     {score}, best P(verified success) {best:.2}), and asks for the spec to be \
                     refined (refine_spec). The attempt runs on the ladder's choice."
                ),
                "Refine the task's spec: its acceptance criteria, verify steps and files to read.",
                format!("spec score {score}, best P(VS) {best:.2}: refine the spec"),
            )
        } else {
            (
                "self_model_abandon",
                "self_model.abandon_flagged",
                format!("{task_id} is unlikely to pass on any rung"),
                format!(
                    "The self-model forecasts at most {best:.2} P(verified success) for task \
                     `{task_id}` of plan `{plan_id}` on every rung, below p_abandon \
                     {P_ABANDON} (abandon). The attempt runs on the ladder's choice."
                ),
                "Split the task, replan it, or drop it.",
                format!("best P(VS) {best:.2} below p_abandon {P_ABANDON}: the task is flagged"),
            )
        };
        tracing::warn!(
            plan_id = %plan_id,
            task_id = %task_id,
            attempt_key = %identity.attempt_key,
            action = %prediction.decision.action,
            spec_score = ?spec_score,
            best_p_vs = best,
            "self-model: {subject}; the attempt runs on the ladder's choice"
        );
        let Some(tui) = &self.tui_bridge else {
            return;
        };
        tui.diagnosis(DiagnosisSummary {
            id: format!("{kind}:{plan_id}/{task_id}"),
            severity: DiagnosisSeverity::Warn,
            subject,
            detail,
            suggested_action: Some(suggested_action.to_string()),
            ..DiagnosisSummary::default()
        });
        let timestamp_ms = u64::try_from(chrono::Utc::now().timestamp_millis()).unwrap_or_default();
        tui.publish_event(roko_core::DashboardEvent::EventLogEntry {
            timestamp_ms,
            event_type: event.to_string(),
            plan_id: plan_id.clone(),
            task_id: task_id.clone(),
            message,
        });
    }

    /// Append the refine request of the attempt `identity` to its run's spec ledger, for S07's
    /// plan-load gate (6133). A write failure is logged: the record is telemetry.
    fn record_refine_request(
        &self,
        identity: &AttemptIdentity,
        spec_score: Option<f64>,
        best: f64,
    ) {
        use std::io::Write as _;

        let Some(runs) = &self.feedback.runs_dir else {
            return;
        };
        // S07 stops a task only on the request of a self-model that routes (gap-2b0575); a
        // request in shadow mode, or before the calibration gate holds, is advice.
        let runtime = self.feedback.self_model.as_deref();
        let mode = runtime.map_or("off", |runtime| mode_name(runtime.settings().mode));
        let acting = runtime.is_some_and(SelfModelRuntime::routes);
        let record = serde_json::json!({
            "ev": SPEC_REFINE_EVENT,
            "run_id": identity.run_id,
            "plan_id": identity.plan_id,
            "task_id": identity.task_id,
            "attempt_key": identity.attempt_key,
            "source": "self_model",
            "mode": mode,
            "acting": acting,
            "spec_score": spec_score,
            "p_vs_max": best,
            "recorded_at_ms": chrono::Utc::now().timestamp_millis(),
        });
        let run_dir = runs.join(&identity.run_id);
        let written = std::fs::create_dir_all(&run_dir).and_then(|()| {
            let mut ledger = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(run_dir.join(SPEC_RECORDS_FILE))?;
            writeln!(ledger, "{record}")
        });
        if let Err(error) = written {
            tracing::warn!(
                %error,
                run = %identity.run_id,
                "self-model: cannot write the refine request to the run's spec ledger"
            );
        }
    }
}

impl GraphTaskDispatcher {
    /// The self-model's step after `verdict`, an agent-blamed failure on ladder rung `rung`
    /// after `climbs` climbs (6131); `None` when the self-model does not route the chain. A
    /// failure that rejected a suspicious pass climbs a rung (6132).
    pub(super) fn self_model_step(
        &self,
        spec: &TaskExecutionSpec,
        task: &TaskDef,
        verdict: &AttemptVerdictRecord,
        rung: usize,
        climbs: u32,
    ) -> Option<StepAction> {
        let runtime = self.feedback.self_model.as_deref()?;
        let chain = &verdict.identity.chain_key;
        if let Some(step) = runtime.escalation_step(&verdict.identity.attempt_key, chain, rung) {
            return Some(step);
        }
        let used = self.attempt_in_run(&format!("{}/{}", spec.plan_id, task.id));
        let retries_left = spec.max_retries.saturating_sub(used);
        let error_class = verdict
            .failure_class
            .as_ref()
            .and_then(|class| class.rung.clone())
            .or_else(|| {
                serde_json::to_value(verdict.outcome)
                    .ok()
                    .and_then(|value| value.as_str().map(str::to_string))
            });
        runtime.post_failure_step(chain, rung, climbs, retries_left, error_class)
    }

    /// The verify depth DP3 checks the attempt `attempt_key`, which ran `executor`, at (6132):
    /// `depth`, its task type's ladder level or M1's floor, raised to the self-model's request
    /// d* when an active self-model acts on the chain. In shadow mode the request is only
    /// logged. Depth never decreases.
    pub(super) fn self_model_depth(
        &self,
        spec: &TaskExecutionSpec,
        task: &TaskDef,
        attempt_key: &str,
        executor: &str,
        depth: VerifyDepth,
    ) -> VerifyDepth {
        let Some(pass) = self.post_pass(attempt_key, executor, depth) else {
            return depth;
        };
        let PassAction::Deepen { depth: level, .. } = pass.action else {
            return depth;
        };
        let requested = depth_at(level).max(depth);
        let applied = pass.runtime.acts_after_pass(&pass.chain);
        tracing::info!(
            plan_id = %spec.plan_id,
            task_id = %task.id,
            depth = ?depth,
            requested = ?requested,
            risk_fg = pass.risk_fg,
            applied,
            "self-model: a low-confidence pass asks for a deeper verify depth"
        );
        if applied { requested } else { depth }
    }

    /// The self-model's step once the attempt `attempt_key`, which ran `executor`, passed every
    /// check of verify depth `depth` (6132): the failure that escalates the model, when at the
    /// deepest depth P(false green) is still above r_max and an active self-model can climb the
    /// chain a rung; that failure climbs it. Otherwise `None`: the pass stands and exports its
    /// risk as `risk_fg`, and in shadow mode an escalation is only logged.
    pub(super) fn self_model_after_pass(
        &self,
        spec: &TaskExecutionSpec,
        task: &TaskDef,
        attempt_key: &str,
        executor: &str,
        depth: VerifyDepth,
    ) -> Option<String> {
        let pass = self.post_pass(attempt_key, executor, depth)?;
        let runtime = pass.runtime;
        if !matches!(pass.action, PassAction::Escalate { .. }) {
            runtime.risks.lock().insert(pass.chain, pass.risk_fg);
            return None;
        }
        let used = self.attempt_in_run(&format!("{}/{}", spec.plan_id, task.id));
        let retries_left = spec.max_retries.saturating_sub(used);
        let applied = runtime.acts_after_pass(&pass.chain)
            && runtime.can_climb(&pass.chain, pass.current, retries_left);
        tracing::warn!(
            plan_id = %spec.plan_id,
            task_id = %task.id,
            depth = ?depth,
            risk_fg = pass.risk_fg,
            r_max = FALSE_GREEN_RISK_MAX,
            applied,
            "self-model: the pass looks like a false green at the deepest verify depth \
             (pass but suspicious); a stronger model should retry the task"
        );
        if !applied {
            runtime.risks.lock().insert(pass.chain, pass.risk_fg);
            return None;
        }
        runtime.suspicious.lock().insert(attempt_key.to_string());
        Some(format!(
            "self_model:pass_but_suspicious: the self-model puts P(false green) at {:.2}, above \
             r_max {FALSE_GREEN_RISK_MAX}, after every check of verify depth {depth:?}, the \
             deepest; a stronger model retries the task",
            pass.risk_fg
        ))
    }

    /// What the self-model makes of the pass of the attempt `attempt_key`, which ran
    /// `executor`, at verify depth `depth` (6132); `None` without a self-model, a forecast of
    /// the attempt for the model that ran, or a false-green risk that has learned.
    fn post_pass(
        &self,
        attempt_key: &str,
        executor: &str,
        depth: VerifyDepth,
    ) -> Option<PostPass<'_>> {
        let runtime = self.feedback.self_model.as_deref()?;
        let candidates = runtime.open_candidates(attempt_key)?;
        let (current, ran) = candidates
            .iter()
            .enumerate()
            .find(|(_, candidate)| self.ran_model(&candidate.arm.model, executor))?;
        let action = post_pass_action(ran, depth)?;
        let chain = roko_learn::telemetry::AttemptKey::parse(attempt_key)?.chain_key();
        Some(PostPass {
            runtime,
            chain,
            current,
            risk_fg: ran.p_fg,
            action,
        })
    }

    /// Whether `model`, an arm's model, is the one `executor` names: the same name, or the
    /// `[models.*]` key of its slug.
    fn ran_model(&self, model: &str, executor: &str) -> bool {
        let profile = self.config.models.get(model);
        model == executor || profile.is_some_and(|profile| profile.slug == executor)
    }

    /// Note whether the self-model made the climb the chain `chain_key` just took (6131).
    pub(super) fn note_self_model_climb(&self, chain_key: &str, early: bool) {
        if let Some(runtime) = self.feedback.self_model.as_deref() {
            runtime
                .early_climbs
                .lock()
                .insert(chain_key.to_string(), early);
        }
    }

    /// Whether the self-model made the last climb of the chain `chain_key` (6131).
    pub(super) fn self_model_climbed(&self, chain_key: &str) -> bool {
        let Some(runtime) = self.feedback.self_model.as_deref() else {
            return false;
        };
        let climbs = runtime.early_climbs.lock();
        climbs.get(chain_key).copied().unwrap_or(false)
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
    let acts =
        settings.mode == SelfModelMode::Active && !pinned && gate.eligible && !gate.breaker_tripped;
    let choice = would_choose.filter(|_| acts)?;
    (settings.allow_downward_start || choice >= default).then_some(choice)
}

/// Policy (b)'s step after a gate pass at verify depth `depth` (S04 §4.4, 6132), from `ran`, the
/// attempt's forecast for the model that ran: ask for a deeper depth d*, escalate the model, or
/// accept and export r = P(false green). `None` while r is the false-green head's prior: no VS
/// label from S05's audits has taught it, so it says nothing about this pass.
fn post_pass_action(ran: &CandidateForecast, depth: VerifyDepth) -> Option<PassAction> {
    if (ran.p_fg - FALSE_GREEN_PRIOR).abs() < 1e-9 {
        return None;
    }
    let loss_fg = FALSE_GREEN_LOSS * expected_cost(ran);
    Some(after_pass(
        ran.p_fg,
        depth_level(depth),
        &DEPTH_OPTIONS,
        loss_fg,
    ))
}

/// `depth`'s level on S05's V0–V4 scale.
const fn depth_level(depth: VerifyDepth) -> u8 {
    match depth {
        VerifyDepth::V0 => 0,
        VerifyDepth::V1 => 1,
        VerifyDepth::V2 => 2,
        VerifyDepth::V3 => 3,
        VerifyDepth::V4 => 4,
    }
}

/// The verify depth at `level` on S05's scale; V4 above it.
const fn depth_at(level: u8) -> VerifyDepth {
    match level {
        0 => VerifyDepth::V0,
        1 => VerifyDepth::V1,
        2 => VerifyDepth::V2,
        3 => VerifyDepth::V3,
        _ => VerifyDepth::V4,
    }
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
pub(crate) fn arm_of(config: &RokoConfig, model: &str) -> ArmKey {
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
        VERIFY_PROVIDER, jsonl_rows_where, make_spec, make_test_dispatcher,
        make_test_dispatcher_with, model, no_auto_fix,
    };
    use crate::state_hub::StateHub;

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

    /// 6131: after one agent-blamed failure of a chain an active self-model started, a
    /// re-forecast that favours the next rung climbs at once, at most twice (K_max), and the
    /// next attempt's ladder record names the self-model; a chain the self-model did not start
    /// keeps the two-failure rule.
    #[tokio::test]
    async fn self_model_climbs_early_within_k_max() {
        use roko_learn::self_model::Label;
        use roko_learn::telemetry::{
            AttemptKey, AttemptLadder, AttemptOutcome, CostSource, LadderReason,
        };

        let temp = tempfile::tempdir().expect("tempdir");
        let snapshot = PriceSnapshot::builtin().expect("the built-in snapshot");
        let settings = SelfModelConfig {
            mode: SelfModelMode::Active,
            ..SelfModelConfig::default()
        };
        let state = temp.path().join(".roko/learn/self-model/state-v1.json");
        let fresh = SelfModel::new(&snapshot);
        let runtime = Arc::new(SelfModelRuntime::new(settings, state, fresh));
        let rungs = [
            ("cheap", "cheap-model", "claude-haiku-4-5"),
            ("mid", "mid-model", "glm-4.7"),
            ("strong", "stream-model", "claude-sonnet-4-6"),
        ];
        let configure = move |config: &mut RokoConfig| {
            no_auto_fix(config);
            for (_, key, slug) in rungs {
                config
                    .models
                    .insert(key.to_string(), model("stream-cli", slug, None));
            }
            config.routing.ladder.rungs = rungs
                .iter()
                .map(|&(name, key, _)| LadderRung {
                    name: name.to_string(),
                    model: key.to_string(),
                })
                .collect();
        };
        let feedback = GraphFeedbackContext {
            self_model: Some(Arc::clone(&runtime)),
            ..GraphFeedbackContext::default()
        };
        let (dispatcher, mut task) =
            make_test_dispatcher(&temp, VERIFY_PROVIDER, configure, feedback).await;
        task.id = "T-M3".to_string();
        task.model_hint = None;
        let mut spec = make_spec(&task);
        spec.max_retries = 5;

        // What a retry after a gate failure does on each rung: the cheap rung fails it, the
        // mid rung passes about half of the time, and the strong rung passes it. The rungs take
        // turns for nine rounds, under N_MIN outcomes, so the forecasts are L0's (cheap 0.02,
        // mid 0.55, strong 0.98). L1's per-feature steps swing the mid rung's forecast between
        // about 0.03 and 0.97 with each of its outcomes, so no step can be tested on them.
        let retry = TaskFeatures {
            title: "Fix the parser".to_string(),
            family: "focused".to_string(),
            tier: "focused".to_string(),
            role: "implementer".to_string(),
            attempt: 2,
            has_prior_failure: true,
            error_class: Some("gate_failed".to_string()),
            ..TaskFeatures::default()
        };
        let training = [
            ("claude-haiku-4-5", 0.01),
            ("glm-4.7", 0.02),
            ("claude-sonnet-4-6", 0.02),
        ];
        for index in 0..9_u32 {
            for (slug, cost) in training {
                let passed = match slug {
                    "claude-haiku-4-5" => false,
                    "glm-4.7" => index % 2 == 0,
                    _ => true,
                };
                let unit = Unit {
                    attempt_key: AttemptKey::new("train", "plan", format!("T{index}"), 2),
                    plan_id: "plan".to_string(),
                    task_id: format!("T{index}"),
                    role: "implementer".to_string(),
                    tier: "focused".to_string(),
                    family: "focused".to_string(),
                    arm: ArmKey::roko("stream-cli", slug),
                    attempt: 2,
                    prior_failure: true,
                    failure_class: Some("gate_failed".to_string()),
                    label: Label {
                        y_gate: Some(passed),
                        y_vs: None,
                        weight: 1.0,
                        source: LabelSource::GatePassed,
                    },
                    api_equiv_usd: Some(cost),
                    cost_source: CostSource::ProviderUsage,
                    latency_s: Some(60.0),
                    failover: false,
                };
                runtime.observe(&unit, &retry, 1.0);
            }
        }
        assert!(runtime.model.read().cold_start(), "L0 alone forecasts");

        // The self-model started T-M3's chain on the cheap rung.
        let arms = rungs
            .iter()
            .map(|&(_, _, slug)| ArmKey::roko("stream-cli", slug))
            .collect();
        let candidates = Candidates {
            arms,
            rungs: vec![0, 1, 2],
            default: 0,
            step: 0,
            pinned: false,
        };
        let first = TaskFeatures {
            attempt: 1,
            has_prior_failure: false,
            error_class: None,
            ..retry.clone()
        };
        let chain = AttemptKey::new(RUN, "stream-plan", "T-M3", 1).chain_key();
        runtime.chain_starts.lock().insert(chain.clone(), Some(0));
        let plan = (candidates, first);
        runtime.chain_plans.lock().insert(chain.clone(), plan);

        let failure = |task: &TaskDef, attempt: u32, rung: &str, index: u32, step: u32| {
            let key = AttemptKey::new(RUN, "stream-plan", task.id.as_str(), attempt);
            let identity = AttemptIdentity::new(&key);
            let outcome = AttemptOutcome::GateFailed;
            let mut verdict = AttemptVerdictRecord::settle(identity, outcome, true);
            verdict.ladder = Some(AttemptLadder {
                rung: Some(rung.to_string()),
                index: Some(index),
                step,
                reason: LadderReason::SelfModel,
                exhausted: false,
                router_pick: None,
            });
            SettledAttempt {
                verdict: Arc::new(verdict),
                failure_reason: None,
                reflex_rule: None,
                live_tool_calls: LiveToolCalls::default(),
                harness: None,
            }
        };

        // One failure on the cheap rung climbs at once, and so does one on the mid rung.
        dispatcher.note_ladder_outcome(&spec, &task, &failure(&task, 1, "cheap", 0, 0));
        assert_eq!(dispatcher.ladder_step(&spec, &task), 1);
        assert!(dispatcher.self_model_climbed(&chain));
        dispatcher.note_ladder_outcome(&spec, &task, &failure(&task, 2, "mid", 1, 1));
        assert_eq!(dispatcher.ladder_step(&spec, &task), 2);
        // Failures on the strong rung never climb a third time.
        for attempt in 3..6 {
            let settled = failure(&task, attempt, "strong", 2, 2);
            dispatcher.note_ladder_outcome(&spec, &task, &settled);
        }
        assert_eq!(dispatcher.ladder_step(&spec, &task), 2);

        // A chain the self-model did not start keeps the two-failure rule.
        let mut held = task.clone();
        held.id = "T-HOLD".to_string();
        let held_spec = make_spec(&held);
        dispatcher.note_ladder_outcome(&held_spec, &held, &failure(&held, 1, "cheap", 0, 0));
        assert_eq!(dispatcher.ladder_step(&held_spec, &held), 0);
        dispatcher.note_ladder_outcome(&held_spec, &held, &failure(&held, 2, "cheap", 0, 0));
        assert_eq!(dispatcher.ladder_step(&held_spec, &held), 1);
        let held_chain = AttemptKey::new(RUN, "stream-plan", "T-HOLD", 1).chain_key();
        assert!(!dispatcher.self_model_climbed(&held_chain));
    }

    /// 6132: after a pass whose learned false-green risk is high, an active self-model asks DP3
    /// for a deeper verify depth while one below the deepest pays, and keeps the model; after
    /// the deepest depth it rejects the pass, and that one failure climbs the chain a rung. In
    /// shadow mode it only logs. A pass that stands exports its risk.
    #[tokio::test]
    async fn low_confidence_pass_requests_depth_before_model() {
        use roko_learn::telemetry::{AttemptKey, AttemptLadder, AttemptOutcome, LadderReason};

        for mode in [SelfModelMode::Active, SelfModelMode::Shadow] {
            let temp = tempfile::tempdir().expect("tempdir");
            let snapshot = PriceSnapshot::builtin().expect("the built-in snapshot");
            let settings = SelfModelConfig {
                mode,
                ..SelfModelConfig::default()
            };
            let state = temp.path().join(".roko/learn/self-model/state-v1.json");
            let fresh = SelfModel::new(&snapshot);
            let runtime = Arc::new(SelfModelRuntime::new(settings, state, fresh));
            let feedback = GraphFeedbackContext {
                self_model: Some(Arc::clone(&runtime)),
                ..GraphFeedbackContext::default()
            };
            let (dispatcher, mut task) =
                make_test_dispatcher(&temp, VERIFY_PROVIDER, ladder(mode), feedback).await;
            task.id = "T-PASS".to_string();
            task.model_hint = None;
            let mut spec = make_spec(&task);
            spec.max_retries = 5;

            // The self-model started the chain on the cheap rung, and puts the chance that a
            // pass there is a false green at 60%, at $0.10 an attempt.
            let key = AttemptKey::new(RUN, "stream-plan", "T-PASS", 1);
            let (attempt_key, chain) = (key.attempt_key(), key.chain_key());
            let arms: Vec<ArmKey> = ["cheap-model", "stream-model"]
                .into_iter()
                .map(|model| ArmKey::roko("stream-cli", model))
                .collect();
            let suspicious = |arm: &ArmKey| CandidateForecast {
                arm: arm.clone(),
                p_gate: 0.9,
                p_fg: 0.6,
                p_vs: 0.36,
                p_vs_lcb: 0.3,
                cost_q50: 0.10,
                cost_q90: 0.10,
                lat_q50_s: 60.0,
                lat_q90_s: 60.0,
                cold_start: false,
            };
            let candidates = Candidates {
                arms: arms.clone(),
                rungs: vec![0, 1],
                default: 0,
                step: 0,
                pinned: false,
            };
            runtime.chain_starts.lock().insert(chain.clone(), Some(0));
            let plan = (candidates, TaskFeatures::default());
            runtime.chain_plans.lock().insert(chain.clone(), plan);
            let forecast = AttemptForecast {
                version: runtime.version(),
                features: TaskFeatures::default(),
                candidates: arms.iter().map(suspicious).collect(),
                would_choose: Some(0),
                default: 0,
                pinned: false,
                routed: true,
            };
            runtime.remember(attempt_key.clone(), forecast);
            let (active, cheap) = (mode == SelfModelMode::Active, "claude-haiku-4-5");

            // r·L_fg·d_j = 0.6 × (5 × $0.10) × 0.5 = $0.15 pays for V3's $0.08 but not for V4's
            // $0.20: below the deepest depth the pass asks for V3, never for a new model.
            let depth =
                dispatcher.self_model_depth(&spec, &task, &attempt_key, cheap, VerifyDepth::V0);
            let deeper = if active {
                VerifyDepth::V3
            } else {
                VerifyDepth::V0
            };
            assert_eq!(depth, deeper, "{mode:?}");
            let step = dispatcher.self_model_after_pass(&spec, &task, &attempt_key, cheap, depth);
            assert_eq!(step, None, "{mode:?}");
            assert_eq!(runtime.risk_fg(&chain), Some(0.6));

            // After V4's checks r is still above r_max: an active self-model rejects the pass.
            let depth =
                dispatcher.self_model_depth(&spec, &task, &attempt_key, cheap, VerifyDepth::V4);
            assert_eq!(depth, VerifyDepth::V4);
            let step = dispatcher.self_model_after_pass(&spec, &task, &attempt_key, cheap, depth);
            assert_eq!(step.is_some(), active, "{mode:?}: {step:?}");

            // That one failure climbs the chain a rung; in shadow mode nothing moves.
            let identity = AttemptIdentity::new(&key);
            let outcome = AttemptOutcome::GateFailed;
            let mut verdict = AttemptVerdictRecord::settle(identity, outcome, true);
            verdict.ladder = Some(AttemptLadder {
                rung: Some("cheap".to_string()),
                index: Some(0),
                step: 0,
                reason: LadderReason::SelfModel,
                exhausted: false,
                router_pick: None,
            });
            let settled = SettledAttempt {
                verdict: Arc::new(verdict),
                failure_reason: None,
                reflex_rule: None,
                live_tool_calls: LiveToolCalls::default(),
                harness: None,
            };
            dispatcher.note_ladder_outcome(&spec, &task, &settled);
            assert_eq!(dispatcher.ladder_step(&spec, &task), u32::from(active));
            assert_eq!(dispatcher.self_model_climbed(&chain), active, "{mode:?}");
        }
    }

    /// 6133: a refine-spec forecast writes its action into the prediction row, and publishes a
    /// `self_model_refine` diagnosis and a `spec.refine_requested` event, in the run's event log
    /// and its spec ledger; nothing else changes, and the attempt runs on the ladder's choice.
    #[tokio::test]
    async fn refine_spec_action_emits_event_and_keeps_the_ladder_default() {
        let temp = tempfile::tempdir().expect("tempdir");
        let roko = temp.path().join(".roko");
        let runs = roko.join("runs");
        let run_dir = runs.join(RUN);
        // S07 scored the task's spec 30 of 100, below s_min.
        std::fs::create_dir_all(&run_dir).expect("the run's directory");
        let quality = serde_json::json!({
            "ev": "spec.quality",
            "plan_id": "stream-plan",
            "task_id": "T-REFINE",
            "score": 30.0,
        });
        std::fs::write(run_dir.join(SPEC_RECORDS_FILE), format!("{quality}\n"))
            .expect("write the spec record");
        // Policy (a) on a fresh model: no rung's lower bound on P(VS) meets π*.
        let snapshot = PriceSnapshot::builtin().expect("the built-in snapshot");
        let settings = SelfModelConfig {
            mode: SelfModelMode::Active,
            policy: SelfModelPolicy::LcbAci,
            ..SelfModelConfig::default()
        };
        let state = roko.join("learn/self-model/state-v1.json");
        let fresh = SelfModel::new(&snapshot);
        let runtime = Arc::new(SelfModelRuntime::new(settings, state, fresh));
        let feedback = GraphFeedbackContext {
            runs_dir: Some(runs),
            self_model: Some(runtime),
            ..GraphFeedbackContext::default()
        };
        let hub = StateHub::new(64);
        let bridge = TuiBridge::new(hub.sender());
        let (dispatcher, mut task) = make_test_dispatcher_with(
            &temp,
            VERIFY_PROVIDER,
            ladder(SelfModelMode::Active),
            feedback,
            |dispatcher| dispatcher.with_tui_bridge(bridge),
        )
        .await;
        task.id = "T-REFINE".to_string();
        task.model_hint = None;
        let ctx = CellContext::new().with_run_id(RUN.to_string());
        dispatcher
            .dispatch(&make_spec(&task), Vec::new(), &ctx)
            .await
            .expect("the attempt completes");
        drop(dispatcher);

        // The prediction row names the action beside the ladder's own pick, which the attempt
        // ran.
        let predictions = jsonl_rows_where(&run_dir.join("predictions.jsonl"), 1, |row| {
            row["schema_version"] == "roko.prediction/1"
        })
        .await;
        let decision = &predictions[0]["decision"];
        assert_eq!(decision["action"], REFINE_SPEC, "{decision}");
        assert!(decision["would_choose"].is_null(), "{decision}");
        let default = decision["default"].as_str().expect("the ladder's arm");
        let verdicts = jsonl_rows_where(&run_dir.join("attempts.jsonl"), 1, |row| {
            row["schema_version"] == "roko.verdict/1"
        })
        .await;
        let model = verdicts[0]["executed"]["model_requested"]
            .as_str()
            .unwrap_or_default();
        assert!(
            default.contains(&format!("/{model}@")),
            "{default} vs {model}"
        );

        // A diagnosis and an event-log entry tell a person; the spec ledger tells S07.
        let snapshot = hub.current_snapshot();
        let diagnosis = "self_model_refine:stream-plan/T-REFINE";
        let diagnosed = snapshot.diagnoses.iter().any(|row| row.id == diagnosis);
        assert!(diagnosed, "{:?}", snapshot.diagnoses);
        let logged = snapshot
            .event_log
            .iter()
            .any(|entry| entry.event_type == SPEC_REFINE_EVENT && entry.task_id == "T-REFINE");
        assert!(logged, "{:?}", snapshot.event_log);
        let ledger = run_dir.join(SPEC_RECORDS_FILE);
        let ledger = std::fs::read_to_string(&ledger).expect("the spec ledger");
        let requests: Vec<serde_json::Value> = ledger
            .lines()
            .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
            .filter(|record| record["ev"] == SPEC_REFINE_EVENT)
            .collect();
        assert_eq!(requests.len(), 1, "{ledger}");
        assert_eq!(requests[0]["task_id"], "T-REFINE");
        assert_eq!(requests[0]["spec_score"], 0.3);
        assert_eq!(requests[0]["mode"], "active");
        // A fresh model's calibration gate does not hold yet, so S07 takes the request as
        // advice (gap-2b0575); its gate reads the record as written here.
        assert_eq!(requests[0]["acting"], false);
        let read = roko_gate::spec_quality::refine_requests(&ledger);
        assert_eq!(read.len(), 1, "{ledger}");
        assert_eq!(read[0].task_id, "T-REFINE");
        assert!(!read[0].acting, "{read:?}");
    }
}
