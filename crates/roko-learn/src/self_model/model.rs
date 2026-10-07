//! The `SelfModel` handle: forecast, observe, a versioned state file and a cold start:
//! backlog task 6116.
//!
//! One object wires L0 ([`HierarchicalBeta`]), L1 ([`GateForecaster`]), the cost model
//! ([`CostModel`]) and recalibration ([`Recalibrator`]) together, so the version, cold-start
//! and state rules live in one place for every consumer (S04 §5). Units carry one gate
//! verdict, so L1 learns a single `gate` rung until units carry per-rung outcomes. The model
//! counts gate labels toward its cold start and shadow period: S05's VS labels join later
//! (decision 6102, 1b).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use roko_core::pricing_snapshot::PriceSnapshot;
use serde::{Deserialize, Serialize};

use super::cost::CostModel;
use super::features::{
    FEATURE_SCHEMA, FeatureVector, PassEma, TaskFeatures, feature_vector, price_tier,
};
use super::logit::{FALSE_GREEN_PRIOR, GateForecaster};
use super::prior::HierarchicalBeta;
use super::recal::Recalibrator;
use super::{ArmKey, CandidateForecast, PredictorVersion, Unit};

/// The predictor's class, which its version hashes.
pub const MODEL_CLASS: &str = "m3-l1";
/// `schema_version` of the state file.
pub const STATE_SCHEMA: &str = "roko.self_model_state/1";
/// The state file, under `.roko/`.
pub const STATE_FILE: &str = "learn/self-model/state-v1.json";
/// Below this many labelled outcomes, forecasts use L0 alone (S04 §4.10).
pub const N_MIN: usize = 30;
/// The outcomes a new version spends in shadow before its forecasts may count (S04 §4.2).
pub const SHADOW_OUTCOMES: usize = 30;
/// The rung L1 learns while units carry one gate verdict.
const GATE_RUNG: &str = "gate";

/// The version of a predictor of `class` over features `schema`, priced at `snapshot_id`:
/// `m3-l1-` and eight hex digits of their blake3 hash.
#[must_use]
pub fn predictor_version(class: &str, schema: &str, snapshot_id: &str) -> PredictorVersion {
    let digest = blake3::hash(format!("{class}\n{schema}\n{snapshot_id}").as_bytes());
    PredictorVersion(format!("{class}-{}", &digest.to_hex()[..8]))
}

/// The self-model: every part, and what it has learned under its version.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SelfModel {
    /// The predictor version: forecasts are never compared across versions.
    pub version: PredictorVersion,
    /// The price snapshot the costs are priced at.
    pub price_snapshot_id: String,
    /// Each priced model's tier at that snapshot.
    price_tiers: BTreeMap<String, String>,
    /// L0.
    prior: HierarchicalBeta,
    /// L1 and the false-green head.
    gates: GateForecaster,
    /// The cost and latency regressions.
    costs: CostModel,
    /// The recalibration of `p_gate`.
    recal: Recalibrator,
    /// The (family, model) pass EMA.
    history: PassEma,
    /// Labelled outcomes learned under this version.
    pub outcomes: usize,
}

/// What [`SelfModel::load_or_new`] found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StateLoad {
    /// No state file: a fresh model.
    Fresh,
    /// The saved model, of the current version.
    Loaded,
    /// A file of another version, or one that did not parse, set aside here; a fresh model.
    SetAside(PathBuf),
}

/// The state file's contents.
#[derive(Serialize, Deserialize)]
struct StateFile {
    schema_version: String,
    model: SelfModel,
}

impl SelfModel {
    /// A fresh model priced at `snapshot`.
    #[must_use]
    pub fn new(snapshot: &PriceSnapshot) -> Self {
        let price_tiers = snapshot
            .rows()
            .iter()
            .filter_map(|row| {
                let tier = price_tier(snapshot, &row.slug)?;
                Some((row.slug.clone(), tier.to_string()))
            })
            .collect();
        Self {
            version: predictor_version(MODEL_CLASS, FEATURE_SCHEMA, snapshot.id()),
            price_snapshot_id: snapshot.id().to_string(),
            price_tiers,
            prior: HierarchicalBeta::new(),
            gates: GateForecaster::default(),
            costs: CostModel::default(),
            recal: Recalibrator::default(),
            history: PassEma::default(),
            outcomes: 0,
        }
    }

    /// Whether forecasts still come from L0 alone.
    #[must_use]
    pub const fn cold_start(&self) -> bool {
        self.outcomes < N_MIN
    }

    /// Whether this version is still in its shadow period.
    #[must_use]
    pub const fn in_shadow_period(&self) -> bool {
        self.outcomes < SHADOW_OUTCOMES
    }

    /// The forecast for each of `arms` on `task`.
    #[must_use]
    pub fn forecast(&self, task: &TaskFeatures, arms: &[ArmKey]) -> Vec<CandidateForecast> {
        arms.iter()
            .map(|arm| self.forecast_arm(task, arm))
            .collect()
    }

    /// The forecast for `arm` on `task`.
    fn forecast_arm(&self, task: &TaskFeatures, arm: &ArmKey) -> CandidateForecast {
        let l0 = self.prior.forecast(&arm.model, &task.family, &task.role);
        let cold_start = self.cold_start();
        let (p_gate, p_gate_lcb, p_fg) = if cold_start {
            (l0.p, l0.lcb, FALSE_GREEN_PRIOR)
        } else {
            let x = self.features(task, arm);
            let gate = self.gates.forecast(&x, l0.p, &[GATE_RUNG]);
            let p_gate = self.recal.apply(gate.p_gate);
            (p_gate, gate.p_gate_lcb.min(p_gate), gate.p_fg)
        };
        let costs = self.costs.forecast(&cost_features(task, arm));
        CandidateForecast {
            arm: arm.clone(),
            p_gate,
            p_fg,
            p_vs: p_gate * (1.0 - p_fg),
            p_vs_lcb: p_gate_lcb * (1.0 - p_fg),
            cost_q50: costs.cost_q50,
            cost_q90: costs.cost_q90,
            lat_q50_s: costs.lat_q50_s,
            lat_q90_s: costs.lat_q90_s,
            cold_start,
        }
    }

    /// L1's feature vector for `arm` on `task`.
    fn features(&self, task: &TaskFeatures, arm: &ArmKey) -> FeatureVector {
        let tier = self.price_tiers.get(&arm.model).map(String::as_str);
        feature_vector(task, arm, tier, &self.history)
    }

    /// Learn `unit` with weight `w`, from the task features the unit itself carries.
    pub fn observe(&mut self, unit: &Unit, w: f64) {
        self.observe_with(unit, &TaskFeatures::from(unit), w);
    }

    /// Learn `unit` with weight `w` (1/π for an audit-only label), whose task's features are
    /// `task`: L0, L1, the cost model and the history, and the recalibration on its schedule.
    /// A unit with no gate label, or one a failover substituted, teaches nothing.
    pub fn observe_with(&mut self, unit: &Unit, task: &TaskFeatures, w: f64) {
        let Some(passed) = unit.label.y_gate.filter(|_| !unit.failover) else {
            return;
        };
        let arm = &unit.arm;
        let l0 = self.prior.forecast(&arm.model, &task.family, &task.role);
        let x = self.features(task, arm);
        let raw = self.gates.forecast(&x, l0.p, &[GATE_RUNG]).p_gate;
        self.recal.observe(raw, passed, w);
        let rungs = [(GATE_RUNG, Some(passed))];
        self.gates.observe(&x, l0.p, &rungs, unit.label.y_vs, w);
        self.prior
            .observe(&arm.model, &task.family, &task.role, passed, w);
        let cost_x = cost_features(task, arm);
        self.costs.observe(
            &cost_x,
            unit.api_equiv_usd,
            unit.cost_source,
            unit.latency_s,
            w,
        );
        self.history.observe(&task.family, &arm.model, passed);
        self.outcomes += 1;
    }

    /// Learn a late VS label of `unit` (S05's audits, weight 1/π) whose gate label the model has
    /// already learned: only the false-green head moves, and only for a pass.
    pub fn observe_vs(&mut self, unit: &Unit, task: &TaskFeatures, w: f64) {
        let (Some(true), Some(verified)) = (unit.label.y_gate, unit.label.y_vs) else {
            return;
        };
        if unit.failover {
            return;
        }
        let x = self.features(task, &unit.arm);
        self.gates.observe_vs(&x, verified, w);
    }

    /// The state file under the `.roko` directory `roko_dir`.
    #[must_use]
    pub fn state_path(roko_dir: &Path) -> PathBuf {
        roko_dir.join(STATE_FILE)
    }

    /// Save the model to `path` atomically: a temporary file, then a rename.
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let state = StateFile {
            schema_version: STATE_SCHEMA.to_string(),
            model: self.clone(),
        };
        let json = serde_json::to_vec_pretty(&state).map_err(std::io::Error::other)?;
        let temporary = path.with_extension("json.tmp");
        std::fs::write(&temporary, json)?;
        std::fs::rename(&temporary, path)
    }

    /// The model saved at `path`, when it is of the version a fresh model at `snapshot` has.
    /// A missing file gives a fresh model. A file of another version, or one that does not
    /// parse, is set aside (renamed with its version) rather than merged, and a fresh model
    /// starts.
    pub fn load_or_new(
        path: &Path,
        snapshot: &PriceSnapshot,
    ) -> std::io::Result<(Self, StateLoad)> {
        let fresh = Self::new(snapshot);
        let text = match std::fs::read_to_string(path) {
            Ok(text) => text,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok((fresh, StateLoad::Fresh));
            }
            Err(error) => return Err(error),
        };
        let saved = serde_json::from_str::<StateFile>(&text)
            .ok()
            .filter(|state| state.schema_version == STATE_SCHEMA);
        match saved {
            Some(state) if state.model.version == fresh.version => {
                Ok((state.model, StateLoad::Loaded))
            }
            other => {
                let label =
                    other.map_or_else(|| "unreadable".to_string(), |state| state.model.version.0);
                let aside = path.with_extension(format!("json.{label}"));
                std::fs::rename(path, &aside)?;
                Ok((fresh, StateLoad::SetAside(aside)))
            }
        }
    }
}

/// The cost model's features for `arm` on `task`.
fn cost_features(task: &TaskFeatures, arm: &ArmKey) -> FeatureVector {
    let spec_tokens = task.spec.get("spec_tokens").copied();
    CostModel::features(arm, &task.family, spec_tokens, None)
}

impl From<&Unit> for TaskFeatures {
    /// The features a unit carries itself: no title, description or verify commands.
    fn from(unit: &Unit) -> Self {
        Self {
            family: unit.family.clone(),
            tier: unit.tier.clone(),
            role: unit.role.clone(),
            attempt: unit.attempt,
            has_prior_failure: unit.prior_failure,
            error_class: unit.failure_class.clone(),
            ..Self::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::self_model::{Label, LabelSource};
    use crate::telemetry::{AttemptKey, CostSource};

    fn snapshot() -> PriceSnapshot {
        PriceSnapshot::builtin().expect("the built-in snapshot")
    }

    /// A gate-labelled unit of `model`'s attempt `attempt` at task `task`.
    fn unit(model: &str, task: &str, attempt: u32, passed: bool) -> Unit {
        Unit {
            attempt_key: AttemptKey::new("run", "plan", task, attempt),
            plan_id: "plan".to_string(),
            task_id: task.to_string(),
            role: "implementer".to_string(),
            tier: "focused".to_string(),
            family: "focused".to_string(),
            arm: ArmKey::roko("cerebras", model),
            attempt,
            prior_failure: false,
            failure_class: None,
            label: Label {
                y_gate: Some(passed),
                y_vs: None,
                weight: 1.0,
                source: LabelSource::GatePassed,
            },
            api_equiv_usd: Some(0.015),
            cost_source: CostSource::ProviderUsage,
            latency_s: Some(80.0),
            failover: false,
        }
    }

    fn task() -> TaskFeatures {
        TaskFeatures {
            title: "Fix the parser".to_string(),
            family: "focused".to_string(),
            tier: "focused".to_string(),
            role: "implementer".to_string(),
            attempt: 1,
            ..TaskFeatures::default()
        }
    }

    #[test]
    fn state_round_trips_and_version_tracks_schema() {
        let snapshot = snapshot();
        let mut model = SelfModel::new(&snapshot);
        for i in 0..40 {
            model.observe_with(&unit("gpt-oss-120b", "T1", 1, i % 4 != 0), &task(), 1.0);
        }
        let dir = tempfile::tempdir().expect("tempdir");
        let path = SelfModel::state_path(dir.path());
        model.save(&path).expect("save the state");
        let (loaded, load) = SelfModel::load_or_new(&path, &snapshot).expect("load the state");
        assert_eq!(load, StateLoad::Loaded);
        assert_eq!(loaded.version, model.version);
        assert_eq!(loaded.outcomes, model.outcomes);
        assert!(
            !path.with_extension("json.tmp").exists(),
            "the write was atomic"
        );
        // JSON keeps every float to within a rounding step, so the forecasts agree.
        let arms = [
            ArmKey::roko("cerebras", "gpt-oss-120b"),
            ArmKey::roko("zai", "glm-4.7"),
        ];
        let (before, after) = (
            model.forecast(&task(), &arms),
            loaded.forecast(&task(), &arms),
        );
        for (a, b) in before.iter().zip(&after) {
            assert!((a.p_vs - b.p_vs).abs() < 1e-9, "{a:?} against {b:?}");
            assert!((a.cost_q90 / b.cost_q90 - 1.0).abs() < 1e-9, "{a:?}");
        }

        let version = predictor_version(MODEL_CLASS, FEATURE_SCHEMA, snapshot.id());
        assert_eq!(model.version, version);
        assert!(model.version.0.starts_with("m3-l1-"));
        let next_schema = predictor_version(MODEL_CLASS, "m3-features/2", snapshot.id());
        assert_ne!(
            next_schema, version,
            "a new feature schema is a new predictor"
        );
    }

    #[test]
    fn observe_moves_the_forecast_in_the_right_direction() {
        let arms = [ArmKey::roko("cerebras", "gpt-oss-120b")];
        let mut passing = SelfModel::new(&snapshot());
        let before = passing.forecast(&task(), &arms)[0].p_vs;
        for _ in 0..40 {
            passing.observe_with(&unit("gpt-oss-120b", "T1", 1, true), &task(), 1.0);
        }
        let after_passes = passing.forecast(&task(), &arms)[0].clone();
        assert!(after_passes.p_vs > before, "{before} then {after_passes:?}");
        assert!(after_passes.p_vs_lcb <= after_passes.p_vs);
        let mut failing = SelfModel::new(&snapshot());
        for _ in 0..40 {
            failing.observe_with(&unit("gpt-oss-120b", "T1", 1, false), &task(), 1.0);
        }
        let after_failures = failing.forecast(&task(), &arms)[0].p_vs;
        assert!(after_failures < before, "{before} then {after_failures}");
        // The cost model learned the attempts' cost.
        let cost = after_passes.cost_q50;
        assert!((cost - 0.015).abs() < 0.002, "{cost}");
    }

    #[test]
    fn the_cold_start_flag_clears_at_n_min() {
        let arms = [ArmKey::roko("cerebras", "gpt-oss-120b")];
        let mut model = SelfModel::new(&snapshot());
        for _ in 0..(N_MIN - 1) {
            model.observe(&unit("gpt-oss-120b", "T1", 1, true), 1.0);
        }
        assert!(model.forecast(&task(), &arms)[0].cold_start);
        assert!(model.in_shadow_period());
        model.observe(&unit("gpt-oss-120b", "T1", 1, true), 1.0);
        assert!(!model.forecast(&task(), &arms)[0].cold_start);
        // A failover substitute or an unlabelled unit teaches nothing.
        let mut substitute = unit("gpt-oss-120b", "T2", 1, true);
        substitute.failover = true;
        model.observe(&substitute, 1.0);
        let mut unlabelled = unit("gpt-oss-120b", "T3", 1, true);
        unlabelled.label.y_gate = None;
        model.observe(&unlabelled, 1.0);
        assert_eq!(model.outcomes, N_MIN);
    }

    /// 6129: a late VS label of a pass moves the false-green forecast and nothing else; one of
    /// a failure, or of a failover substitute, teaches nothing.
    #[test]
    fn a_late_vs_label_moves_only_the_false_green_head() {
        let arms = [ArmKey::roko("cerebras", "gpt-oss-120b")];
        let mut model = SelfModel::new(&snapshot());
        for _ in 0..N_MIN {
            model.observe_with(&unit("gpt-oss-120b", "T1", 1, true), &task(), 1.0);
        }
        let before = model.forecast(&task(), &arms)[0].clone();
        let mut failed = unit("gpt-oss-120b", "T2", 1, false);
        failed.label.y_vs = Some(false);
        let mut substitute = unit("gpt-oss-120b", "T3", 1, true);
        substitute.label.y_vs = Some(false);
        substitute.failover = true;
        model.observe_vs(&failed, &task(), 5.0);
        model.observe_vs(&substitute, &task(), 5.0);
        assert_eq!(model.forecast(&task(), &arms)[0], before);

        let mut audited = unit("gpt-oss-120b", "T1", 1, true);
        audited.label.y_vs = Some(false);
        audited.label.source = LabelSource::Vs;
        for _ in 0..10 {
            model.observe_vs(&audited, &task(), 5.0);
        }
        let after = model.forecast(&task(), &arms)[0].clone();
        assert!(after.p_fg > before.p_fg, "{before:?} then {after:?}");
        assert_eq!(after.p_gate, before.p_gate);
        assert_eq!(model.outcomes, N_MIN);
    }

    #[test]
    fn the_version_changes_with_the_snapshot_id() {
        let old = snapshot();
        let newer_text = include_str!("../../../../config/prices/2026-09-28.toml")
            .replace("id = \"prices-2026-09-28\"", "id = \"prices-2026-10-01\"");
        let newer = PriceSnapshot::from_toml(&newer_text, "a newer snapshot").expect("snapshot");
        let model = SelfModel::new(&old);
        assert_ne!(model.version, SelfModel::new(&newer).version);

        let dir = tempfile::tempdir().expect("tempdir");
        let path = SelfModel::state_path(dir.path());
        model.save(&path).expect("save the state");
        let (fresh, load) = SelfModel::load_or_new(&path, &newer).expect("load the state");
        let StateLoad::SetAside(aside) = load else {
            panic!("the old version's state is set aside: {load:?}");
        };
        assert!(aside.exists() && !path.exists(), "{}", aside.display());
        assert_eq!(fresh.price_snapshot_id, "prices-2026-10-01");
        assert_eq!(fresh.outcomes, 0);
        let (_, missing) = SelfModel::load_or_new(&path, &newer).expect("no state");
        assert_eq!(missing, StateLoad::Fresh);
    }
}
