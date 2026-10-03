//! M1 on the plan path (S06 §4.1; backlog 8122): [`HomeostasisSink`] folds a
//! Graph run's settled verdicts into task resolutions and hands each to the
//! ultrastable controller (`roko_learn::homeostasis`).
//!
//! - A chain resolves at its first `passed` verdict, or at the failed
//!   attempt that used up its retry budget ([`HomeostasisSink::set_retry_limits`],
//!   counted in this process as the Graph cell counts its retries), or, still
//!   open, when the run ends ([`HomeostasisSink::finish`]). A cancelled
//!   attempt ends its chain, as the cell starts no retry after one.
//! - The controller's rows go through a telemetry writer per run to the
//!   workspace's cross-run `learn/controller.jsonl` (S01 §5.10), and a θ it
//!   commits goes through the guarded store (`learn/commits/harness/`). Its
//!   state is saved to `learn/homeostat-state.json` when the run ends.
//! - The sink exists only when `[homeostasis] mode` is not off and the
//!   workspace has an S5 policy (`.roko/policy/viability.toml`): without
//!   bounds a person wrote, M1 has nothing to regulate. A frozen run
//!   registers no sink (decision 2218). In shadow mode, the default, nothing
//!   it does changes dispatch.
//! - With `[homeostasis] m3_prior`, the controller's move priors come from
//!   the run's self-model through [`SelfModelPredictor`] (8121, gap-d1ebc1).
//!
//! Per attempt, [`HomeostasisSink::decide`] draws the chain's arm on the
//! fixed `harness_policy` holdout (8118) and returns the θ the attempt runs:
//! θ₀ on the holdout and all-off arms and in shadow, the controller's θ on
//! the learned arm in `on` mode. Dispatch writes the decision row and stamps
//! the verdict with it (8123). The controller's θ lives in a
//! [`HarnessParamsHandle`], swapped after every change it makes.
//!
//! It never reads the conductor's ring. A verdict whose failure class names
//! a conductor cancel counts as a conductor restart of its chain, an
//! auxiliary signal that only ranks moves; no attempt is marked so while the
//! conductor tick is held (dec-e70592).

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use async_trait::async_trait;
use roko_core::config::harness_params::{
    HarnessLadders, HarnessParams, HarnessParamsHandle, VersionedParams,
};
use roko_core::config::homeostasis::HomeostasisMode;
use roko_core::config::schema::RokoConfig;
use roko_core::task::TaskTier;
use roko_fs::layout::RokoLayout;
use roko_learn::homeostasis::controller::{Controller, ControllerEvent};
use roko_learn::homeostasis::holdout::HarnessHoldout;
use roko_learn::homeostasis::detect::Baseline;
use roko_learn::homeostasis::ev::Ev;
use roko_learn::homeostasis::ledger::{ControllerRecord, Envelope};
use roko_learn::homeostasis::lkg::ThetaLkg;
use roko_learn::homeostasis::policy::{DEFAULT_HOLDOUT, ViabilityPolicy, non_m1_fingerprint};
use roko_learn::homeostasis::priors::{Calibration, DrivePredictor, M3Prior, PredictedLevels};
use roko_learn::homeostasis::resolution::{ResolutionFold, TaskResolution};
use roko_learn::self_model::{ArmKey, CandidateForecast};
use roko_learn::loop_audit::assign::takes_default;
use roko_learn::telemetry::records::HARNESS_POLICY_DECISION_POINT;
use roko_learn::telemetry::{
    Arm, Assignment, AttemptIdentity, AttemptKey, AttemptOutcome, AttemptVerdictRecord,
    DecisionSource, HarnessPolicyDecisionRecord, HarnessStamp, TelemetryWriter,
    TelemetryWriterConfig,
};

use super::{FeedbackEvent, FeedbackSink};
use crate::graph_task_dispatch::self_model::{SelfModelRuntime, arm_of};

/// The seed of the controller's decisions: 0 until runs record an
/// experiment seed (S01 `experiment.seed`), as the arm-set draws do.
pub const CONTROLLER_SEED: u64 = 0;
/// The failure-class basis that marks an attempt a conductor cancel ended.
pub const CONDUCTOR_BASIS: &str = "conductor";
/// Settled tasks the self-model forecasts for a move's prior.
pub const PREDICTED_TASKS: usize = 20;

/// Folds a run's settled verdicts into resolutions for M1's controller.
#[derive(Debug)]
pub struct HomeostasisSink {
    layout: RokoLayout,
    /// The controller's θ, swapped after each change it makes.
    handle: HarnessParamsHandle,
    theta0: HarnessParams,
    mode: HomeostasisMode,
    holdout: HarnessHoldout,
    state: parking_lot::Mutex<SinkState>,
}

/// What M1 decided for one attempt (8123).
#[derive(Debug, Clone, PartialEq)]
pub struct HarnessDecision {
    /// The chain's assignment on the `harness_policy` layer.
    pub assignment: Assignment,
    /// The θ the attempt runs.
    pub applied: HarnessParams,
    /// The controller's θ with its version: the would-be θ in shadow.
    pub chosen: Arc<VersionedParams>,
    /// θ₀.
    pub default: HarnessParams,
    /// M1's mode.
    pub mode: HomeostasisMode,
}

impl HarnessDecision {
    /// What the attempt's verdict says of it.
    #[must_use]
    pub fn stamp(&self) -> HarnessStamp {
        HarnessStamp {
            arm: self.assignment.arm,
            policy_version: self.chosen.policy_version,
            params_digest: self.applied.params_digest(),
        }
    }

    /// The attempt `identity`'s `harness_policy` decision row (A-DEC-H);
    /// `pinned` when a pin chooses its model, which B1 leaves alone (8124).
    #[must_use]
    pub fn record(&self, identity: AttemptIdentity, pinned: bool) -> HarnessPolicyDecisionRecord {
        HarnessPolicyDecisionRecord {
            identity,
            decision_point: HARNESS_POLICY_DECISION_POINT.to_string(),
            assignment: self.assignment.clone(),
            arm: self.assignment.arm,
            mode: self.mode,
            policy_version: self.chosen.policy_version,
            params_digest: self.applied.params_digest(),
            chosen: self.chosen.params.clone(),
            default: self.default.clone(),
            differs: self.chosen.params != self.default,
            source: DecisionSource::Control,
            pinned,
        }
    }
}

#[derive(Debug, Default)]
struct SinkState {
    fold: ResolutionFold,
    /// Attempts settled in this process, by chain key.
    attempts: HashMap<String, u32>,
    /// Attempts a conductor cancel ended, by chain key.
    restarts: HashMap<String, u32>,
    /// Each task's retry budget, by plan and task id.
    retry_limits: HashMap<(String, String), u32>,
    /// The chains resolved so far, in order.
    resolved: Vec<String>,
    /// Each chain's last harness stamp: its arm and the θ it ran.
    stamps: HashMap<String, HarnessStamp>,
    controller: Option<Controller>,
    lkg: Option<ThetaLkg>,
    /// Each run's writer; `None` when it did not start.
    writers: HashMap<String, Option<TelemetryWriter>>,
}

impl HomeostasisSink {
    /// A sink under `workdir` that folds resolutions for `controller`, or
    /// only counts them without one.
    #[must_use]
    pub fn new(workdir: &Path, controller: Option<Controller>, lkg: Option<ThetaLkg>) -> Self {
        let (theta0, theta, mode, holdout) = match &controller {
            Some(controller) => (
                controller.state().theta0.clone(),
                controller.theta().clone(),
                controller.mode(),
                controller.policy().holdout,
            ),
            None => {
                let theta0 = HarnessParams::baseline(&RokoConfig::default());
                (theta0.clone(), theta0, HomeostasisMode::Shadow, DEFAULT_HOLDOUT)
            }
        };
        let handle = HarnessParamsHandle::new(theta0.clone());
        if theta != theta0 {
            handle.swap(theta, "lkg");
        }
        let state = SinkState {
            controller,
            lkg,
            ..SinkState::default()
        };
        Self {
            layout: RokoLayout::for_project(workdir),
            handle,
            theta0,
            mode,
            holdout: HarnessHoldout::new(holdout),
            state: parking_lot::Mutex::new(state),
        }
    }

    /// M1's decision for the attempt `key`, whose run draws its arms on
    /// `epoch`: its chain's arm on the fixed `harness_policy` holdout, and
    /// the θ it runs (θ₀ on the holdout and all-off arms and in shadow).
    #[must_use]
    pub fn decide(&self, key: &AttemptKey, epoch: &str) -> HarnessDecision {
        let assignment = self.holdout.assign(epoch, CONTROLLER_SEED, key);
        let chosen = self.handle.load();
        let applied = if takes_default(assignment.arm) || self.mode != HomeostasisMode::On {
            self.theta0.clone()
        } else {
            chosen.params.clone()
        };
        HarnessDecision {
            assignment,
            applied,
            chosen,
            default: self.theta0.clone(),
            mode: self.mode,
        }
    }

    /// The handle holding the controller's θ.
    #[must_use]
    pub const fn handle(&self) -> &HarnessParamsHandle {
        &self.handle
    }

    /// This sink with no holdout and no all-off draw, so every chain is on
    /// the learned arm whatever the day's draws: for tests of what θ does.
    #[cfg(test)]
    #[must_use]
    pub(crate) fn without_holdout(mut self) -> Self {
        self.holdout = HarnessHoldout { h: 0.0, g: 0.0 };
        self
    }

    /// The digest of the θ a learned-arm attempt runs now: the run
    /// manifest's `config.params_digest` at run open.
    #[must_use]
    pub fn params_digest(&self) -> String {
        if self.mode == HomeostasisMode::On {
            self.handle.load().params_digest.clone()
        } else {
            self.theta0.params_digest()
        }
    }

    /// The sink of a plan run under `workdir`: `None` when `[homeostasis]
    /// mode` is off or the workspace has no S5 policy. The controller resumes
    /// its saved state, takes the committed θ, and draws its move priors from
    /// `self_model` when `[homeostasis] m3_prior` is on.
    #[must_use]
    pub fn for_workdir(
        workdir: &Path,
        config: &RokoConfig,
        self_model: Option<&Arc<SelfModelRuntime>>,
    ) -> Option<Self> {
        if config.homeostasis.mode == HomeostasisMode::Off {
            return None;
        }
        let layout = RokoLayout::for_project(workdir);
        let policy = match ViabilityPolicy::load_optional(layout.root()) {
            Ok(Some(policy)) => policy,
            Ok(None) => {
                tracing::debug!("no .roko/policy/viability.toml: M1 has nothing to regulate");
                return None;
            }
            Err(error) => {
                tracing::warn!(%error, "M1 is off: the S5 viability policy does not load");
                return None;
            }
        };
        let theta0 = HarnessParams::baseline(config);
        let ladders = HarnessLadders::from_config(config);
        let state_path = Controller::state_path(layout.root());
        let (mut controller, restart) = Controller::load(
            &state_path,
            &config.homeostasis,
            policy.clone(),
            theta0.clone(),
            ladders.clone(),
            implied_baseline(&policy),
            CONTROLLER_SEED,
        );
        if !restart.is_empty() {
            tracing::info!(events = restart.len(), "M1 resumed: an open episode was abandoned");
        }
        controller = controller.with_config_fingerprint(non_m1_fingerprint(config));
        if config.homeostasis.m3_prior
            && let Some(runtime) = self_model
        {
            let predictor = SelfModelPredictor::new(Arc::clone(runtime), config);
            let prior = M3Prior::new(Arc::new(predictor), &policy);
            controller = controller.with_prior(Box::new(prior));
        }
        let lkg = match ThetaLkg::open(&layout.learn_dir(), theta0, ladders, policy) {
            Ok(lkg) => Some(lkg),
            Err(error) => {
                tracing::warn!(%error, "M1's guarded store does not open; θ is not committed");
                None
            }
        };
        if let Some(lkg) = &lkg
            && let Err(error) = lkg.restore_into(&mut controller)
        {
            tracing::warn!(%error, "M1's committed θ does not restore");
        }
        Some(Self::new(workdir, Some(controller), lkg))
    }

    /// Record the retry budgets of `plan_id`'s tasks: a failed attempt that
    /// is the last its task's budget allows resolves the chain.
    pub fn set_retry_limits(&self, plan_id: &str, limits: impl IntoIterator<Item = (String, u32)>) {
        let mut state = self.state.lock();
        for (task_id, max_retries) in limits {
            state
                .retry_limits
                .insert((plan_id.to_string(), task_id), max_retries);
        }
    }

    /// The chains resolved so far, in order.
    #[must_use]
    pub fn resolved_chains(&self) -> Vec<String> {
        self.state.lock().resolved.clone()
    }

    /// θ as the controller holds it; `None` without a controller.
    #[must_use]
    pub fn theta(&self) -> Option<HarnessParams> {
        let state = self.state.lock();
        state.controller.as_ref().map(|controller| controller.theta().clone())
    }

    /// The run ended: resolve the chains still open, save the controller's
    /// state, and close the writers.
    pub fn finish(&self) {
        let mut state = self.state.lock();
        let open = state.fold.close();
        for resolution in open {
            let run_id = run_of(&resolution.chain_key).to_string();
            self.resolve(&mut state, resolution, &run_id);
        }
        if let Some(controller) = &state.controller {
            let path = Controller::state_path(self.layout.root());
            if let Err(error) = controller.save(&path) {
                tracing::warn!(%error, "failed to save M1's state (non-fatal)");
            }
        }
        for writer in state.writers.drain().filter_map(|(_, writer)| writer) {
            writer.close();
        }
    }

    /// Fold one settled verdict.
    fn settle(&self, verdict: &AttemptVerdictRecord) {
        let mut state = self.state.lock();
        let identity = &verdict.identity;
        let chain = identity.chain_key.clone();
        let attempts = {
            let count = state.attempts.entry(chain.clone()).or_default();
            *count += 1;
            *count
        };
        let conductor = verdict
            .failure_class
            .as_ref()
            .is_some_and(|class| class.basis.as_deref() == Some(CONDUCTOR_BASIS));
        if conductor {
            *state.restarts.entry(chain.clone()).or_default() += 1;
        }
        if let Some(stamp) = &verdict.harness {
            state.stamps.insert(chain.clone(), stamp.clone());
        }
        let task = (identity.plan_id.clone(), identity.task_id.clone());
        let last = verdict.outcome == AttemptOutcome::Cancelled
            || state
                .retry_limits
                .get(&task)
                .is_some_and(|&retries| attempts > retries);
        let mut resolution = state.fold.push(verdict);
        if resolution.is_none() && last {
            resolution = state.fold.resolve(&chain);
        }
        if let Some(resolution) = resolution {
            self.resolve(&mut state, resolution, &identity.run_id);
        }
    }

    /// Hand `resolution` to the controller, and write what it did.
    fn resolve(&self, state: &mut SinkState, mut resolution: TaskResolution, run_id: &str) {
        resolution.conductor_restarts = state
            .restarts
            .get(&resolution.chain_key)
            .copied()
            .unwrap_or(0);
        if let Some(stamp) = state.stamps.get(&resolution.chain_key) {
            resolution.arm = Some(stamp.arm);
            resolution.params_digest = Some(stamp.params_digest.clone());
        }
        state.resolved.push(resolution.chain_key.clone());
        let SinkState {
            controller,
            lkg,
            writers,
            ..
        } = state;
        let Some(controller) = controller.as_mut() else {
            return;
        };
        let events = controller.on_resolution(&resolution);
        let envelope = Envelope {
            ts: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            run_id: Some(run_id.to_string()),
            policy_version: controller.policy().policy_version,
            arm: Arm::Learned,
            seq: controller.state().resolutions,
        };
        let mut swap_reason = None;
        for event in &events {
            if let ControllerEvent::Change(change) = event {
                let episode = change.episode_id.as_deref().unwrap_or("relax");
                swap_reason = Some(format!("homeostat:{episode}/{}", change.change_id));
            }
            if let Some(lkg) = lkg.as_mut()
                && let Err(error) = lkg.commit_event(event)
            {
                tracing::warn!(%error, "M1 could not commit θ");
            }
            let Some(record) = ControllerRecord::from_event(&envelope, event) else {
                continue;
            };
            if let Some(writer) = writer(writers, &self.layout, run_id) {
                writer.submit(record);
            }
        }
        if let Some(reason) = swap_reason {
            self.handle.swap(controller.theta().clone(), reason);
        }
        if events
            .iter()
            .any(|event| matches!(event, ControllerEvent::Hold { .. }))
        {
            tracing::warn!(run_id, "M1 holds: nothing in its box helped; a person decides");
        }
    }
}

/// The writer of `run_id`, started on its first row.
fn writer<'a>(
    writers: &'a mut HashMap<String, Option<TelemetryWriter>>,
    layout: &RokoLayout,
    run_id: &str,
) -> Option<&'a TelemetryWriter> {
    writers
        .entry(run_id.to_string())
        .or_insert_with(|| {
            let config = TelemetryWriterConfig::default();
            match TelemetryWriter::for_run(layout, run_id, config) {
                Ok(writer) => Some(writer),
                Err(error) => {
                    tracing::warn!(run_id, %error, "M1's rows have no writer");
                    None
                }
            }
        })
        .as_ref()
}

/// The run of a chain key, `run:plan:task`.
fn run_of(chain_key: &str) -> &str {
    chain_key
        .split_once(':')
        .map_or(chain_key, |(run, _)| run)
}

/// The in-control levels the S5 policy's bounds were calibrated from, by
/// S06 §4.2's rules run backwards: L₁ = p* − 0.15, U₂ = 2 × the in-control
/// cost per verified success, and U₄ = 2 × the in-control p90 wall time.
#[must_use]
pub fn implied_baseline(policy: &ViabilityPolicy) -> Baseline {
    let pass_rate = (policy.ev.pass_rate.lo + 0.15).min(0.95);
    let usd_per_success = policy.ev.outer(Ev::UsdPerVerifiedSuccess) / 2.0;
    let wall_s = policy.ev.outer(Ev::LatencyP90S) / 2.0;
    Baseline {
        pass_rate,
        usd_per_resolution: usd_per_success * pass_rate,
        wall_ms: wall_s * 1000.0,
    }
}

#[async_trait]
impl FeedbackSink for HomeostasisSink {
    fn name(&self) -> &'static str {
        "homeostasis"
    }

    /// Settled attempts only: every attempt settles once (S01 §4.3).
    fn interested(&self, event: &FeedbackEvent) -> bool {
        matches!(event, FeedbackEvent::AttemptSettled(_))
    }

    async fn on_event(&self, event: &FeedbackEvent) -> Result<(), anyhow::Error> {
        if let FeedbackEvent::AttemptSettled(verdict) = event {
            self.settle(verdict);
        }
        Ok(())
    }
}

/// M3's drive predictions for the controller's move priors (8121;
/// gap-d1ebc1): the run's most recently settled tasks, each forecast on the
/// rung θ starts its tier on, averaged. With the ladder off, or before any
/// task settles, it predicts nothing and the catalog-sign prior stands.
#[derive(Debug)]
pub struct SelfModelPredictor {
    runtime: Arc<SelfModelRuntime>,
    /// Each `[routing.ladder]` rung's arm, by rung name.
    arms: HashMap<String, ArmKey>,
}

impl SelfModelPredictor {
    /// The predictions of `runtime`, the run's self-model, over `config`'s
    /// ladder.
    #[must_use]
    pub fn new(runtime: Arc<SelfModelRuntime>, config: &RokoConfig) -> Self {
        let arms = config
            .routing
            .ladder
            .rungs
            .iter()
            .map(|rung| (rung.name.clone(), arm_of(config, &rung.model)))
            .collect();
        Self { runtime, arms }
    }
}

impl DrivePredictor for SelfModelPredictor {
    fn predict(&self, theta: &HarnessParams) -> Option<PredictedLevels> {
        let mut forecasts: Vec<CandidateForecast> = Vec::new();
        for features in self.runtime.recent_features(PREDICTED_TASKS) {
            let tier = TaskTier::parse(&features.tier).unwrap_or_default();
            let arm = theta
                .tier_floor
                .get(&tier)
                .and_then(|rung| self.arms.get(rung));
            if let Some(arm) = arm {
                forecasts.extend(self.runtime.forecast(&features, std::slice::from_ref(arm)));
            }
        }
        if forecasts.is_empty() {
            return None;
        }
        let n = forecasts.len() as f64;
        let mean = |value: fn(&CandidateForecast) -> f64| -> f64 {
            forecasts.iter().map(value).sum::<f64>() / n
        };
        Some(PredictedLevels {
            pass_rate: mean(|forecast| forecast.p_vs),
            cost_p50_usd: mean(|forecast| forecast.cost_q50),
            latency_p90_s: mean(|forecast| forecast.lat_q90_s),
        })
    }

    fn calibration(&self) -> Calibration {
        let report = self.runtime.gate();
        Calibration {
            ece: report.ece,
            drift: report.breaker_tripped,
        }
    }
}

#[cfg(test)]
mod tests {
    use roko_learn::telemetry::{AttemptIdentity, AttemptKey, CostSource};

    use super::*;

    /// The loop-census plan (S01 P0-12): T4 is routed first, T1 passes, T2
    /// fails both its attempts (one retry), T3 has no verify step.
    const PLAN: &str = "loop-census";
    const RUN: &str = "gr-census";

    fn verdict(task: &str, attempt: u32, outcome: AttemptOutcome) -> AttemptVerdictRecord {
        let key = AttemptKey::new(RUN, PLAN, task, attempt);
        let mut record = AttemptVerdictRecord::settle(AttemptIdentity::new(&key), outcome, false);
        record.cost.api_equiv_usd = Some(0.01);
        record.cost.source = CostSource::ProviderUsage;
        let started = i64::from(attempt) * 10_000;
        record.timing.attempt_started_at = Some(started);
        record.timing.settled_at = Some(started + 4_000);
        record
    }

    fn census() -> Vec<AttemptVerdictRecord> {
        vec![
            verdict("T4", 1, AttemptOutcome::Passed),
            verdict("T1", 1, AttemptOutcome::Passed),
            verdict("T2", 1, AttemptOutcome::GateFailed),
            verdict("T2", 2, AttemptOutcome::GateFailed),
            verdict("T3", 1, AttemptOutcome::Unverified),
        ]
    }

    fn limits() -> Vec<(String, u32)> {
        [("T1", 0), ("T2", 1), ("T3", 0), ("T4", 0)]
            .into_iter()
            .map(|(task, retries)| (task.to_string(), retries))
            .collect()
    }

    async fn settle_all(sink: &HomeostasisSink, verdicts: &[AttemptVerdictRecord]) {
        for verdict in verdicts {
            let event = FeedbackEvent::AttemptSettled(Arc::new(verdict.clone()));
            assert!(sink.interested(&event));
            sink.on_event(&event).await.expect("the sink takes the verdict");
        }
    }

    #[tokio::test]
    async fn homeostasis_sink_receives_one_resolution_per_chain() {
        let temp = tempfile::tempdir().expect("tempdir");
        let chain = |task: &str| format!("{RUN}:{PLAN}:{task}");

        // With the plan's retry budgets, every chain of the loop-census run
        // resolves once, as it settles: T2 at its second failure, its last.
        let sink = HomeostasisSink::new(temp.path(), None, None);
        sink.set_retry_limits(PLAN, limits());
        settle_all(&sink, &census()).await;
        let expected = [chain("T4"), chain("T1"), chain("T2"), chain("T3")];
        assert_eq!(sink.resolved_chains(), expected);
        // A repeated settlement resolves nothing more, nor does the run's end.
        settle_all(&sink, &census()[..2]).await;
        sink.finish();
        assert_eq!(sink.resolved_chains(), expected);

        // Without budgets a failed chain stays open until the run ends, and
        // resolves then, once.
        let open = HomeostasisSink::new(temp.path(), None, None);
        settle_all(&open, &census()[2..3]).await;
        assert!(open.resolved_chains().is_empty());
        open.finish();
        assert_eq!(open.resolved_chains(), [chain("T2")]);

        // A cancelled attempt ends its chain whatever its budget.
        let cancelled = HomeostasisSink::new(temp.path(), None, None);
        cancelled.set_retry_limits(PLAN, limits());
        settle_all(&cancelled, &[verdict("T2", 1, AttemptOutcome::Cancelled)]).await;
        assert_eq!(cancelled.resolved_chains(), [chain("T2")]);
    }

    /// S06 §5's example policy.
    const POLICY: &str = r#"
policy_version = 1
ev.pass_rate = { lo = 0.70, inner = 0.75 }
ev.usd_per_verified_success = { hi = 0.12, inner = 0.108, abs_cap = 0.50 }
ev.false_green = { hi = 0.10 }
ev.latency_p90_s = { hi = 900, inner = 810 }
"#;

    #[tokio::test]
    async fn homeostasis_sink_needs_a_mode_and_a_policy() {
        let temp = tempfile::tempdir().expect("tempdir");
        let config = RokoConfig::default();
        assert_eq!(config.homeostasis.mode, HomeostasisMode::Shadow);
        // No policy: nothing to regulate.
        assert!(HomeostasisSink::for_workdir(temp.path(), &config, None).is_none());

        let policy_path = ViabilityPolicy::path_in(&temp.path().join(".roko"));
        std::fs::create_dir_all(policy_path.parent().expect("policy dir")).expect("mkdir");
        std::fs::write(&policy_path, POLICY).expect("write the policy");
        let mut off = config.clone();
        off.homeostasis.mode = HomeostasisMode::Off;
        assert!(HomeostasisSink::for_workdir(temp.path(), &off, None).is_none());

        // In shadow the sink runs the controller, keeps θ₀, and saves the
        // controller's state when the run ends.
        let sink = HomeostasisSink::for_workdir(temp.path(), &config, None).expect("a sink");
        sink.set_retry_limits(PLAN, limits());
        settle_all(&sink, &census()).await;
        assert_eq!(sink.resolved_chains().len(), 4);
        assert_eq!(sink.theta(), Some(HarnessParams::baseline(&config)));
        sink.finish();
        let state = Controller::state_path(&temp.path().join(".roko"));
        assert!(state.is_file(), "{}", state.display());

        // The policy's bounds imply the detectors' in-control levels.
        let policy = ViabilityPolicy::parse(POLICY).expect("the policy parses");
        let baseline = implied_baseline(&policy);
        assert!((baseline.pass_rate - 0.85).abs() < 1e-9);
        assert!((baseline.usd_per_resolution - 0.06 * 0.85).abs() < 1e-9);
        assert!((baseline.wall_ms - 450_000.0).abs() < 1e-6);
    }
}
