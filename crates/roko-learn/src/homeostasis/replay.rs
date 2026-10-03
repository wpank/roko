//! The full-information replay evaluator for arms A0 to A5, plus S09 X2's
//! A3-gated and A3-mis (S06 §4.9).
//!
//! Shadow replay cannot judge a change: there the outcomes come from θ₀. An
//! [`OutcomeTable`] knows the resolution at every stream position under any
//! θ, so each arm's θ shapes what it sees. A [`SyntheticTable`] builds one
//! over a synthetic stream; S09's Stage-A export of logged outcomes is the
//! other implementation it waits for.
//!
//! The arms ([`ReplayArm`]): A0 static θ₀; A1 naive retry (one retry more,
//! always); A2 reactive rules (a directed move on every resolution with an
//! EV past its outer bound, a step back toward θ₀ otherwise: no dwell,
//! rollback or hysteresis); A3 M1; A4 random only (Ashby); A5 M1 without
//! dwell or hysteresis; A3-gated (a guided step only when M3's predicted
//! gain has a 95% lower bound above 0); A3-mis (M3's predictions shuffled
//! within stratum). The last two read M3 through [`Predictions`]; without
//! a predictor they report "no predictions" instead of inventing any.
//!
//! Common random numbers: every arm replays the same seed, in the same
//! stream order, and the resolution at a position depends only on that
//! position's draws and the arm's θ. A [`Tracer`] measures every arm with
//! the same instrument, and A0's spend per resolution stands for the
//! holdout's. Results are replayed: label them "replayed from synthetic
//! outcomes" or "replayed from logged outcomes (date, n)" (S06 §8).

use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use roko_core::config::harness_params::{HarnessLadders, HarnessParams, Knob, Step};
use roko_core::config::homeostasis::{HomeostasisConfig, HomeostasisMode};
use serde::{Deserialize, Serialize};

use super::catalog::{
    Effect, Expected, Move, REQUISITE_VARIETY, RequisiteRow, Signature, candidates, catalog_move,
};
use super::controller::{
    Candidate, Controller, ControllerEvent, MoveKey, MovePrior, MovePriorValue, UNCHANGED_CONFIG,
};
use super::detect::Baseline;
use super::ev::{Ev, EvBounds, EvWindow, SchmittBand};
use super::ledger::RecoveryRow;
use super::policy::{ChangeKind, SafetyBox, SafetyContext, ViabilityPolicy};
use super::resolution::TaskResolution;
use super::saso::{Recovery, TracePoint, Tracer};
use super::streams::{Draws, Regime, StepKind, SyntheticStream, resolve};

/// Share of a disturbance's remaining gap each counter-move in θ restores.
pub const RESTORE_SHARE: f64 = 0.6;
/// A move that raises E2 multiplies the cost by this; one that lowers it,
/// by [`COST_DOWN`].
pub const COST_UP: f64 = 1.25;
/// See [`COST_UP`].
pub const COST_DOWN: f64 = 0.8;
/// A move that raises E4 multiplies the wall time by this; one that lowers
/// it, by [`WALL_DOWN`].
pub const WALL_UP: f64 = 1.2;
/// See [`WALL_UP`].
pub const WALL_DOWN: f64 = 0.85;
/// Resamples behind each bootstrap interval.
pub const BOOTSTRAP_RESAMPLES: usize = 2000;
/// The bootstrap's seed.
pub const BOOTSTRAP_SEED: u64 = 0x8117_b005;

/// The resolution at each stream position under any θ.
pub trait OutcomeTable {
    /// Positions in the stream, numbered from 1.
    fn len(&self) -> u64;

    /// Whether the stream is empty.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The resolution at `position` under `theta`.
    fn outcome(&self, position: u64, theta: &HarnessParams) -> TaskResolution;
}

/// A full-information table over a synthetic stream. How θ changes a
/// regime is a model ([`SyntheticTable::regime`]), not logged data.
#[derive(Debug, Clone)]
pub struct SyntheticTable {
    stream: SyntheticStream,
    length: u64,
    theta0: HarnessParams,
    ladders: HarnessLadders,
}

impl SyntheticTable {
    /// The table of `length` positions of `stream`, around `theta0`.
    #[must_use]
    pub fn new(
        stream: SyntheticStream,
        length: u64,
        theta0: HarnessParams,
        ladders: HarnessLadders,
    ) -> Self {
        Self {
            stream,
            length,
            theta0,
            ladders,
        }
    }

    /// The stream.
    #[must_use]
    pub const fn stream(&self) -> &SyntheticStream {
        &self.stream
    }

    /// The regime at `position` under `theta`. Each knob θ moved off θ₀
    /// counts as one move, in its direction from θ₀:
    ///
    /// - after the onset, each move in the disturbance's requisite-variety
    ///   row restores [`RESTORE_SHARE`] of what is left of the gap in pass
    ///   rate, cost and wall time, and with one the failures stop being
    ///   provider errors or budget ends; a kind M1 cannot regulate
    ///   (`flaky_verify`, every provider slowed) has no such move;
    /// - every move scales cost and wall time by its catalog effect on E2
    ///   and E4 ([`COST_UP`], [`COST_DOWN`], [`WALL_UP`], [`WALL_DOWN`]).
    #[must_use]
    pub fn regime(&self, position: u64, theta: &HarnessParams) -> Regime {
        let base = self.stream.regime(position);
        let moves = self.moves(theta);
        let countered = match self.row() {
            Some(row) if position > self.stream.onset && row.expected == Expected::Regulate => {
                moves.iter().filter(|entry| row.counters_with(entry)).count()
            }
            _ => 0,
        };
        let left = (1.0 - RESTORE_SHARE).powi(i32::try_from(countered).unwrap_or(i32::MAX));
        let full = Regime::IN_CONTROL;
        let mut regime = Regime {
            pass_rate: full.pass_rate - (full.pass_rate - base.pass_rate) * left,
            cost_factor: full.cost_factor + (base.cost_factor - full.cost_factor) * left,
            wall_factor: full.wall_factor + (base.wall_factor - full.wall_factor) * left,
            provider_errors: base.provider_errors && countered == 0,
            budget_failures: base.budget_failures && countered == 0,
            attempts: base.attempts,
        };
        for entry in &moves {
            let cost = scale(entry.effect(Ev::UsdPerVerifiedSuccess), COST_UP, COST_DOWN);
            regime.cost_factor *= cost;
            regime.wall_factor *= scale(entry.effect(Ev::LatencyP90S), WALL_UP, WALL_DOWN);
        }
        regime
    }

    /// The requisite-variety row of the stream's disturbance.
    fn row(&self) -> Option<&'static RequisiteRow> {
        let step = self.stream.step;
        let rows: &'static [RequisiteRow] = &REQUISITE_VARIETY;
        rows.iter()
            .find(|row| row.kind == step.kind && row.all_providers == step.all_providers)
    }

    /// θ's moves off θ₀: one per knob, in its direction from θ₀, when the
    /// catalog has that move.
    fn moves(&self, theta: &HarnessParams) -> Vec<Move> {
        theta
            .changed_knobs(&self.theta0)
            .into_iter()
            .filter_map(|knob| {
                let here = theta.notch(knob, &self.ladders).ok()?;
                let home = self.theta0.notch(knob, &self.ladders).ok()?;
                let direction = if here > home { Step::Up } else { Step::Down };
                catalog_move(knob.kind(), direction)
            })
            .collect()
    }
}

fn scale(effect: Effect, up: f64, down: f64) -> f64 {
    match effect {
        Effect::Raises => up,
        Effect::Lowers => down,
        Effect::None | Effect::Varies => 1.0,
    }
}

impl OutcomeTable for SyntheticTable {
    fn len(&self) -> u64 {
        self.length
    }

    fn outcome(&self, position: u64, theta: &HarnessParams) -> TaskResolution {
        let draws = Draws::at(self.stream.seed, position);
        resolve(position, &self.regime(position, theta), &draws)
    }
}

/// An arm of the replay (S06 §4.9; S09 X2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ReplayArm {
    /// Static θ₀.
    A0,
    /// Naive retry: one retry more, always.
    A1,
    /// Reactive rules: no dwell, rollback or hysteresis.
    A2,
    /// M1.
    A3,
    /// Random moves only (Ashby).
    A4,
    /// M1 without dwell or hysteresis.
    A5,
    /// M1 taking a guided step only when M3's predicted gain has a 95%
    /// lower bound above 0.
    #[serde(rename = "A3-gated")]
    A3Gated,
    /// M1 with M3's predictions shuffled within stratum.
    #[serde(rename = "A3-mis")]
    A3Mis,
}

impl ReplayArm {
    /// Every arm.
    pub const ALL: [Self; 8] = [
        Self::A0,
        Self::A1,
        Self::A2,
        Self::A3,
        Self::A4,
        Self::A5,
        Self::A3Gated,
        Self::A3Mis,
    ];

    /// `A0` to `A5`, `A3-gated`, `A3-mis`.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::A0 => "A0",
            Self::A1 => "A1",
            Self::A2 => "A2",
            Self::A3 => "A3",
            Self::A4 => "A4",
            Self::A5 => "A5",
            Self::A3Gated => "A3-gated",
            Self::A3Mis => "A3-mis",
        }
    }
}

/// M3's move priors for A3-gated and A3-mis. Until the self-model's
/// predictor feeds them (S04.T04, 8121) there are none.
pub trait Predictions: std::fmt::Debug + Send + Sync {
    /// M3's priors as the controller reads them.
    fn prior(&self) -> Box<dyn MovePrior>;
}

/// A prior whose predictions are shuffled within stratum, the block: a
/// candidate gets the inner prior of the move on another knob of its block,
/// the knobs rotated by an offset the seed fixes. A block of one knob keeps
/// its own. S09 X2's A3-mis.
#[derive(Debug)]
pub struct ShuffledPrior {
    inner: Box<dyn MovePrior>,
    ladders: HarnessLadders,
    seed: u64,
}

impl ShuffledPrior {
    /// `inner`'s predictions shuffled by `seed`.
    #[must_use]
    pub fn new(inner: Box<dyn MovePrior>, ladders: HarnessLadders, seed: u64) -> Self {
        Self {
            inner,
            ladders,
            seed,
        }
    }
}

impl MovePrior for ShuffledPrior {
    fn prior(
        &self,
        theta: &HarnessParams,
        candidate: &Candidate,
        breached: &[Ev],
    ) -> MovePriorValue {
        let block = candidate.key.knob.block();
        let peers: Vec<Knob> = theta
            .knobs()
            .into_iter()
            .filter(|knob| knob.block() == block)
            .collect();
        let own = peers
            .iter()
            .position(|&knob| knob == candidate.key.knob)
            .unwrap_or(0);
        let offset = usize::try_from(self.seed % 7 + 1).unwrap_or(1);
        let knob = peers
            .get((own + offset) % peers.len().max(1))
            .copied()
            .unwrap_or(candidate.key.knob);
        let direction = candidate.key.direction;
        let Ok(next) = theta.step(knob, direction, &self.ladders) else {
            return self.inner.prior(theta, candidate, breached);
        };
        let peer = Candidate {
            key: MoveKey { knob, direction },
            entry: catalog_move(knob.kind(), direction),
            next,
        };
        self.inner.prior(theta, &peer, breached)
    }
}

/// The reason A3-gated and A3-mis give without a predictor.
pub const NO_PREDICTIONS: &str = "no predictions";

/// What one arm saw and did over one stream.
#[derive(Debug, Clone, PartialEq)]
pub struct ArmRun {
    /// The arm.
    pub arm: ReplayArm,
    /// The θ in force at each position.
    pub thetas: Vec<HarnessParams>,
    /// The resolution at each position, under that θ.
    pub resolutions: Vec<TaskResolution>,
    /// The drive trace the tracer measured.
    pub trace: Vec<TracePoint>,
    /// Positions after which θ changed.
    pub changes: usize,
    /// HOLDs the arm's controller entered.
    pub holds: usize,
}

/// Why an arm cannot run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArmUnavailable {
    /// The arm.
    pub arm: ReplayArm,
    /// Why: [`NO_PREDICTIONS`] until M3's predictor feeds the replay.
    pub reason: &'static str,
}

/// A mean over seeds with its percentile bootstrap 95% interval.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct IaeSummary {
    /// The mean.
    pub mean: f64,
    /// The interval's lower end.
    pub low: f64,
    /// The interval's upper end.
    pub high: f64,
    /// Values behind it.
    pub n: usize,
}

/// The mean of `values` with a percentile bootstrap 95% interval; `None`
/// when there are none.
#[must_use]
pub fn bootstrap_mean(values: &[f64]) -> Option<IaeSummary> {
    if values.is_empty() {
        return None;
    }
    let n = values.len();
    let mean = values.iter().sum::<f64>() / n as f64;
    let mut rng = ChaCha8Rng::seed_from_u64(BOOTSTRAP_SEED);
    let mut means = Vec::with_capacity(BOOTSTRAP_RESAMPLES);
    for _ in 0..BOOTSTRAP_RESAMPLES {
        let mut sum = 0.0;
        for _ in 0..n {
            sum += values[rng.gen_range(0..n)];
        }
        means.push(sum / n as f64);
    }
    means.sort_unstable_by(f64::total_cmp);
    let last = BOOTSTRAP_RESAMPLES - 1;
    let at = |q: f64| means[((q * BOOTSTRAP_RESAMPLES as f64) as usize).min(last)];
    Some(IaeSummary {
        mean,
        low: at(0.025),
        high: at(0.975),
        n,
    })
}

/// One arm × disturbance over the seeds.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArmReport {
    /// The arm.
    pub arm: ReplayArm,
    /// The disturbance's name.
    pub disturbance: String,
    /// The first seed's scorecard.
    pub recovery: Option<Recovery>,
    /// IAE over the seeds.
    pub iae: Option<IaeSummary>,
    /// θ changes per stream, on average.
    pub changes: f64,
    /// HOLDs per stream, on average.
    pub holds: f64,
    /// Why the arm did not run.
    pub note: Option<String>,
}

impl ArmReport {
    /// The `recovery` row on `ev`: the first seed's scorecard, with the
    /// mean IAE over the seeds.
    #[must_use]
    pub fn row(&self, ev: Ev) -> Option<RecoveryRow> {
        let mut recovery = self.recovery.clone()?;
        if let Some(iae) = self.iae {
            recovery.iae = iae.mean;
        }
        let episode = format!("replay:{}:{}", self.arm.label(), self.disturbance);
        Some(recovery.row(&episode, ev))
    }
}

/// `policy` with every inner band at its outer bound, so an EV recovers
/// where it breaches: A5's bounds.
#[must_use]
pub fn without_hysteresis(policy: &ViabilityPolicy) -> ViabilityPolicy {
    let mut flat = policy.clone();
    flat.ev.pass_rate.inner = Some(flat.ev.pass_rate.lo);
    let uppers = [
        &mut flat.ev.usd_per_verified_success,
        &mut flat.ev.false_green,
        &mut flat.ev.latency_p90_s,
    ];
    for bound in uppers {
        bound.inner = Some(bound.outer());
    }
    flat
}

/// Runs the arms over outcome tables.
#[derive(Debug)]
pub struct Evaluator {
    config: HomeostasisConfig,
    policy: ViabilityPolicy,
    theta0: HarnessParams,
    ladders: HarnessLadders,
    baseline: Baseline,
    predictions: Option<Box<dyn Predictions>>,
}

impl Evaluator {
    /// An evaluator measuring against `policy` around `theta0` on
    /// `ladders`, its controllers in `on` mode with `[homeostasis]`'s
    /// defaults and detectors at `baseline`.
    #[must_use]
    pub fn new(
        policy: ViabilityPolicy,
        theta0: HarnessParams,
        ladders: HarnessLadders,
        baseline: Baseline,
    ) -> Self {
        Self {
            config: HomeostasisConfig {
                mode: HomeostasisMode::On,
                ..HomeostasisConfig::default()
            },
            policy,
            theta0,
            ladders,
            baseline,
            predictions: None,
        }
    }

    /// The evaluator with M3's predictions, for A3-gated and A3-mis.
    #[must_use]
    pub fn with_predictions(mut self, predictions: Box<dyn Predictions>) -> Self {
        self.predictions = Some(predictions);
        self
    }

    /// The evaluator with `config`'s controller constants; the mode stays
    /// `on`.
    #[must_use]
    pub fn with_config(mut self, config: &HomeostasisConfig) -> Self {
        self.config = HomeostasisConfig {
            mode: HomeostasisMode::On,
            ..config.clone()
        };
        self
    }

    /// The table of `step` over `length` positions, stepped after `onset`,
    /// drawn from `seed`.
    #[must_use]
    pub fn table(&self, step: StepKind, onset: u64, length: u64, seed: u64) -> SyntheticTable {
        let stream = SyntheticStream { step, onset, seed };
        SyntheticTable::new(stream, length, self.theta0.clone(), self.ladders.clone())
    }

    /// Run `arm` over `table`, its controller's decisions drawn from `seed`.
    ///
    /// # Errors
    ///
    /// [`ArmUnavailable`] for A3-gated and A3-mis without predictions.
    pub fn run(
        &self,
        arm: ReplayArm,
        table: &dyn OutcomeTable,
        seed: u64,
    ) -> Result<ArmRun, ArmUnavailable> {
        let mut chosen = self.arm_policy(arm, seed)?;
        let mut tracer = Tracer::new(&self.policy, self.config.window as usize);
        let mut run = ArmRun {
            arm,
            thetas: Vec::new(),
            resolutions: Vec::new(),
            trace: Vec::new(),
            changes: 0,
            holds: 0,
        };
        for position in 1..=table.len() {
            let theta = chosen.theta().clone();
            let resolution = table.outcome(position, &theta);
            run.trace.push(tracer.push(&resolution));
            run.holds += chosen.observe(&resolution);
            if *chosen.theta() != theta {
                run.changes += 1;
            }
            run.thetas.push(theta);
            run.resolutions.push(resolution);
        }
        Ok(run)
    }

    fn arm_policy(&self, arm: ReplayArm, seed: u64) -> Result<ArmPolicy, ArmUnavailable> {
        let controller = |config: &HomeostasisConfig, policy: ViabilityPolicy| {
            Controller::new(
                config,
                policy,
                self.theta0.clone(),
                self.ladders.clone(),
                self.baseline,
                seed,
            )
        };
        let unavailable = ArmUnavailable {
            arm,
            reason: NO_PREDICTIONS,
        };
        let chosen = match arm {
            ReplayArm::A0 => ArmPolicy::Static(self.theta0.clone()),
            ReplayArm::A1 => {
                let retry = self.theta0.step(Knob::RetryDelta, Step::Up, &self.ladders);
                ArmPolicy::Static(retry.unwrap_or_else(|_| self.theta0.clone()))
            }
            ReplayArm::A2 => ArmPolicy::Reactive(Box::new(Reactive::new(self))),
            ReplayArm::A3 => {
                let plain = controller(&self.config, self.policy.clone());
                ArmPolicy::Controlled(Box::new(plain))
            }
            ReplayArm::A4 => {
                let config = HomeostasisConfig {
                    random_step_prob: 1.0,
                    ..self.config.clone()
                };
                ArmPolicy::Controlled(Box::new(controller(&config, self.policy.clone())))
            }
            ReplayArm::A5 => {
                let config = HomeostasisConfig {
                    dwell_resolutions: 1,
                    dwell_min_secs: 0,
                    ..self.config.clone()
                };
                let flat = without_hysteresis(&self.policy);
                ArmPolicy::Controlled(Box::new(controller(&config, flat)))
            }
            ReplayArm::A3Gated => {
                let predictions = self.predictions.as_ref().ok_or(unavailable)?;
                let gated = controller(&self.config, self.policy.clone())
                    .with_prior(predictions.prior())
                    .with_guided_needs_lcb(true);
                ArmPolicy::Controlled(Box::new(gated))
            }
            ReplayArm::A3Mis => {
                let predictions = self.predictions.as_ref().ok_or(unavailable)?;
                let shuffled = ShuffledPrior::new(predictions.prior(), self.ladders.clone(), seed);
                let mis =
                    controller(&self.config, self.policy.clone()).with_prior(Box::new(shuffled));
                ArmPolicy::Controlled(Box::new(mis))
            }
        };
        Ok(chosen)
    }

    /// One report per arm × disturbance: each stream steps after `onset`,
    /// runs `length` positions and is replayed once per seed, A0 standing
    /// for the holdout's spend.
    #[must_use]
    pub fn evaluate(
        &self,
        steps: &[StepKind],
        arms: &[ReplayArm],
        onset: u64,
        length: u64,
        seeds: &[u64],
    ) -> Vec<ArmReport> {
        let start = usize::try_from(onset).unwrap_or(usize::MAX);
        let mut reports = Vec::new();
        for &step in steps {
            let tables: Vec<SyntheticTable> = seeds
                .iter()
                .map(|&seed| self.table(step, onset, length, seed))
                .collect();
            let holdout: Vec<f64> = tables
                .iter()
                .zip(seeds)
                .map(|(table, &seed)| self.static_spend(table, seed, start))
                .collect();
            for &arm in arms {
                reports.push(self.report(arm, step, &tables, seeds, &holdout, start));
            }
        }
        reports
    }

    /// A0's spend per resolution from `start`.
    fn static_spend(&self, table: &SyntheticTable, seed: u64, start: usize) -> f64 {
        let Ok(run) = self.run(ReplayArm::A0, table, seed) else {
            return 0.0;
        };
        let after = run.trace.get(start..).unwrap_or_default();
        after.iter().map(|point| point.usd).sum::<f64>() / after.len().max(1) as f64
    }

    fn report(
        &self,
        arm: ReplayArm,
        step: StepKind,
        tables: &[SyntheticTable],
        seeds: &[u64],
        holdout: &[f64],
        start: usize,
    ) -> ArmReport {
        let window = self.config.recover_window as usize;
        let mut recoveries = Vec::new();
        let (mut changes, mut holds) = (0_usize, 0_usize);
        let mut note = None;
        for ((table, &seed), &spend) in tables.iter().zip(seeds).zip(holdout) {
            match self.run(arm, table, seed) {
                Ok(run) => {
                    recoveries.push(Recovery::measure(&run.trace, start, spend, window));
                    changes += run.changes;
                    holds += run.holds;
                }
                Err(unavailable) => {
                    note = Some(unavailable.reason.to_string());
                    break;
                }
            }
        }
        let runs = recoveries.len().max(1) as f64;
        let iaes: Vec<f64> = recoveries.iter().map(|recovery| recovery.iae).collect();
        ArmReport {
            arm,
            disturbance: step.name().to_string(),
            recovery: recoveries.first().cloned(),
            iae: bootstrap_mean(&iaes),
            changes: changes as f64 / runs,
            holds: holds as f64 / runs,
            note,
        }
    }
}

/// How an arm sets θ.
#[derive(Debug)]
enum ArmPolicy {
    Static(HarnessParams),
    Reactive(Box<Reactive>),
    Controlled(Box<Controller>),
}

impl ArmPolicy {
    fn theta(&self) -> &HarnessParams {
        match self {
            Self::Static(theta) => theta,
            Self::Reactive(reactive) => &reactive.theta,
            Self::Controlled(controller) => controller.theta(),
        }
    }

    /// Show the arm one resolution; returns the HOLDs it entered.
    fn observe(&mut self, resolution: &TaskResolution) -> usize {
        match self {
            Self::Static(_) => 0,
            Self::Reactive(reactive) => {
                reactive.observe(resolution);
                0
            }
            Self::Controlled(controller) => controller
                .on_resolution(resolution)
                .iter()
                .filter(|event| matches!(event, ControllerEvent::Hold { .. }))
                .count(),
        }
    }
}

/// A2's reactive rules.
#[derive(Debug, Clone)]
struct Reactive {
    theta: HarnessParams,
    theta0: HarnessParams,
    ladders: HarnessLadders,
    safety: SafetyBox,
    bounds: EvBounds,
    window: EvWindow,
}

impl Reactive {
    fn new(evaluator: &Evaluator) -> Self {
        let theta0 = evaluator.theta0.clone();
        let ladders = evaluator.ladders.clone();
        Self {
            theta: theta0.clone(),
            safety: SafetyBox::new(theta0.clone(), ladders.clone(), &evaluator.policy),
            theta0,
            ladders,
            bounds: evaluator.policy.ev,
            window: EvWindow::new(evaluator.config.window as usize),
        }
    }

    /// While an EV is past its outer bound, take the first admissible
    /// directed move; once none is, step one knob back toward θ₀.
    fn observe(&mut self, resolution: &TaskResolution) {
        self.window.push(resolution);
        if !self.window.is_full() {
            return;
        }
        let estimates = self.window.estimate(&self.bounds);
        let breached: Vec<Ev> = Ev::ALL
            .into_iter()
            .filter(|&ev| SchmittBand::new(ev, &self.bounds).beyond_outer(estimates.get(ev)))
            .collect();
        let next = if breached.is_empty() {
            self.toward_theta0()
        } else {
            self.directed(&breached)
        };
        if let Some(next) = next {
            self.theta = next;
        }
    }

    fn directed(&self, breached: &[Ev]) -> Option<HarnessParams> {
        candidates(&Signature::of(breached))
            .into_iter()
            .find_map(|entry| {
                entry.knobs(&self.theta).into_iter().find_map(|knob| {
                    let next = self.theta.step(knob, entry.direction, &self.ladders).ok()?;
                    self.admits(&next, ChangeKind::Search).then_some(next)
                })
            })
    }

    fn toward_theta0(&self) -> Option<HarnessParams> {
        self.theta
            .changed_knobs(&self.theta0)
            .into_iter()
            .find_map(|knob| {
                let here = self.theta.notch(knob, &self.ladders).ok()?;
                let home = self.theta0.notch(knob, &self.ladders).ok()?;
                let step = if here > home { Step::Down } else { Step::Up };
                let next = self.theta.step(knob, step, &self.ladders).ok()?;
                self.admits(&next, ChangeKind::Relax).then_some(next)
            })
    }

    fn admits(&self, next: &HarnessParams, kind: ChangeKind) -> bool {
        let context = SafetyContext {
            kind,
            config_before: Some(UNCHANGED_CONFIG.to_string()),
            config_after: Some(UNCHANGED_CONFIG.to_string()),
            arm: None,
            adaptation_spend_usd: 0.0,
            run_spend_usd: 0.0,
            adaptation_spend_max_frac: 0.0,
            knowledge_loop_live: false,
        };
        self.safety.validate(&self.theta, next, &context).passed()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::homeostasis::catalog::DisturbanceKind;
    use crate::homeostasis::streams::{IN_CONTROL, replay_theta0};

    #[test]
    fn replay_arms_share_common_random_numbers() {
        let stream = SyntheticStream {
            step: StepKind::of(DisturbanceKind::ModelSwap),
            onset: 20,
            seed: 11,
        };
        let (theta0, ladders) = replay_theta0();
        let evaluator =
            Evaluator::new(stream.policy(), theta0.clone(), ladders.clone(), IN_CONTROL);
        let table = SyntheticTable::new(stream, 100, theta0.clone(), ladders);
        let arms = [
            ReplayArm::A0,
            ReplayArm::A1,
            ReplayArm::A2,
            ReplayArm::A3,
            ReplayArm::A4,
            ReplayArm::A5,
        ];
        let runs: Vec<ArmRun> = arms
            .iter()
            .map(|&arm| evaluator.run(arm, &table, 3).expect("the arm runs"))
            .collect();
        let a0 = &runs[0];

        // Every arm replays the same positions in the same order, and its
        // resolution at a position is the table's outcome under its θ
        // there: one draw per position, whatever the arm.
        for run in &runs {
            assert_eq!(run.resolutions.len(), 100);
            for (index, resolution) in run.resolutions.iter().enumerate() {
                let position = index as u64 + 1;
                assert_eq!(*resolution, table.outcome(position, &run.thetas[index]));
                assert_eq!(resolution.resolved_at, a0.resolutions[index].resolved_at);
            }
            // Until an arm first moves θ, it sees exactly what A0 sees.
            let same = run.thetas.iter().take_while(|theta| **theta == theta0).count();
            assert_eq!(run.resolutions[..same], a0.resolutions[..same]);
        }
        // A1 runs one retry more from the start; M1 waits for a breach.
        assert_ne!(runs[1].thetas[0], theta0);
        assert_eq!(runs[3].thetas[0], theta0);

        // A0 is the θ₀ stream, and its IAE is that stream's, traced by hand.
        assert_eq!(a0.resolutions, stream.resolutions(100));
        assert_eq!(a0.changes, 0);
        let mut tracer = Tracer::new(&stream.policy(), 20);
        let by_hand: f64 = stream
            .resolutions(100)
            .iter()
            .map(|resolution| tracer.push(resolution).drive)
            .skip(20)
            .sum();
        let iae = Recovery::measure(&a0.trace, 20, 0.0, 10).iae;
        assert!((iae - by_hand).abs() < 1e-12, "{iae} vs {by_hand}");
        assert!(iae > 0.0, "the step drives D above 0");

        // Without a predictor A3-gated and A3-mis say so.
        let gated = evaluator.run(ReplayArm::A3Gated, &table, 3);
        let expected = ArmUnavailable {
            arm: ReplayArm::A3Gated,
            reason: NO_PREDICTIONS,
        };
        assert_eq!(gated.err(), Some(expected));

        // The report: one per arm, IAE over the seeds with its interval.
        let reports = evaluator.evaluate(&[stream.step], &ReplayArm::ALL, 20, 100, &[1, 2, 3]);
        assert_eq!(reports.len(), ReplayArm::ALL.len());
        for report in &reports {
            if matches!(report.arm, ReplayArm::A3Gated | ReplayArm::A3Mis) {
                assert_eq!(report.note.as_deref(), Some(NO_PREDICTIONS));
                assert!(report.iae.is_none());
                continue;
            }
            let summary = report.iae.expect("an IAE over the seeds");
            assert_eq!(summary.n, 3);
            assert!(summary.low <= summary.mean && summary.mean <= summary.high);
            assert!(report.row(Ev::PassRate).is_some());
        }
        let spent_over_holdout = reports[0]
            .recovery
            .as_ref()
            .map(|recovery| recovery.adaptation_cost_usd);
        assert!(spent_over_holdout.is_some_and(|usd| usd.abs() < 1e-9));
    }
}
