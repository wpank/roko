//! The controller: IDLE, SEARCH and HOLD, with guided (Thompson) and Ashby
//! random moves, dwell, rollback and relaxation toward θ₀ (S06 §4.5).
//!
//! [`Controller::on_resolution`] takes one settled task resolution and
//! returns what the controller did as [`ControllerEvent`]s:
//!
//! - a holdout row only teaches the cost baseline of adaptation spend; any
//!   other row feeds the estimators, the Schmitt bands (once the window is
//!   full) and the detectors;
//! - a breach the detectors confirm opens an episode, which keeps θ as the
//!   last-known-good and makes a first move;
//! - an EV whose estimate stays outside its inner band for `relax_window`
//!   resolutions without a breach latches as a slow breach, so an EV
//!   parked between its bands still opens an episode, or keeps one moving;
//! - a change dwells `dwell_resolutions` resolutions **and**
//!   `dwell_min_secs` seconds, then is kept, or rolled back and made tabu
//!   when the drive did not fall by `improve_delta_frac` of D_pre, its
//!   value before the change, or another EV newly breached. D_pre is the
//!   window's drive, except that a breached EV whose estimate left its
//!   inner band inside the window also reads over the resolutions since,
//!   and counts at the worse reading, so a step whose onset lies inside the
//!   window is not diluted by the resolutions before it. In shadow mode a
//!   change is unevaluable: the would-be θ drops it and the move is tabu,
//!   so the next proposal differs;
//! - `recover_window` resolutions with D = 0 commit θ and close the
//!   episode; `max_changes_per_episode` changes, or adaptation spend over
//!   `adaptation_spend_max_frac` of spend, restore the last-known-good and
//!   HOLD until a person acknowledges or the S5 bounds change;
//! - after `relax_window` calm resolutions with E2 below 0.7 of its bound,
//!   one cost-raising knob steps one notch back toward θ₀, judged by the
//!   same rule.
//!
//! A move is one notch of one knob. Guided moves come from the catalog's
//! directed candidates, less those the SafetyBox refuses, the tabu ones and
//! locked knobs, by Thompson sampling. With probability `random_step_prob`
//! the move is instead a uniform step over every admissible notch (Ashby's
//! step function). Before any candidate has evidence the guided step takes
//! the highest prior mean, the catalog's most fitting candidate on a tie,
//! since a Thompson draw would be prior noise alone. The prior is the
//! catalog sign (±0.1) until M3's priors replace it ([`MovePrior`], 8121).
//! A knob whose search moves change direction twice in an episode is
//! locked for the episode.
//!
//! Model swap and convention flip show the same breach, E1 low and nothing
//! else, but have different requisite-variety rows. With the catalog-sign
//! prior both get B1 floor↑ first, which lies in model swap's row only, so
//! S06 C2 ("first move in row") holds for convention flip only once M3's
//! priors tell the two apart.
//!
//! The controller only proposes θ: it never reads the conductor's ring and
//! never restarts or fails an attempt. Its one file is its own state,
//! `.roko/learn/homeostat-state.json`; committing and restoring θ go
//! through [`super::lkg`] (8113).

use std::path::{Path, PathBuf};

use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use roko_core::config::harness_params::{Block, HarnessLadders, HarnessParams, Knob, Step};
use roko_core::config::homeostasis::{HomeostasisConfig, HomeostasisMode};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::catalog::{Effect, Move, Signature, candidates, catalog_move};
use super::detect::{Baseline, DetectorBank, DetectorTuning, Observation};
use super::ev::{
    Drive, Estimate, Ev, EvBands, EvEstimates, EvWindow, Side, drive, estimate_latency_p90_s,
    estimate_pass_rate, estimate_usd_per_verified_success,
};
use super::policy::{ChangeKind, SafetyBox, SafetyContext, Verdict, ViabilityPolicy};
use super::resolution::TaskResolution;
use crate::telemetry::AttemptOutcome;

/// The controller's state file, relative to the `.roko` directory.
pub const STATE_FILE: &str = "learn/homeostat-state.json";
/// `schema_version` of the state file.
pub const STATE_SCHEMA: &str = "roko.homeostat_state/1";
/// The mode a person set for M1's next runs through serve's admin route
/// (8131), relative to the `.roko` directory. It beats `[homeostasis] mode`;
/// M2 can still push it down to shadow.
pub const MODE_FILE: &str = "learn/homeostat-mode.json";

/// The mode a person set for M1's next runs, when one did (8131).
#[must_use]
pub fn operator_mode(roko_dir: &Path) -> Option<HomeostasisMode> {
    let text = std::fs::read_to_string(roko_dir.join(MODE_FILE)).ok()?;
    let value: Value = serde_json::from_str(&text).ok()?;
    serde_json::from_value(value.get("mode")?.clone()).ok()
}

/// Keep `mode` as the one a person set for M1's next runs (8131).
///
/// # Errors
///
/// I/O errors creating the directory or writing the file.
pub fn set_operator_mode(roko_dir: &Path, mode: HomeostasisMode) -> std::io::Result<()> {
    let path = roko_dir.join(MODE_FILE);
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let text = serde_json::json!({ "mode": mode, "actor": "human" }).to_string();
    std::fs::write(path, text)
}
/// The catalog-sign prior: the drive reduction expected from a move whose
/// catalog effects counter the breach, and its negative when they worsen it.
pub const CATALOG_PRIOR: f64 = 0.1;
/// v_base: the prior variance of a move's drive reduction.
pub const PRIOR_VARIANCE: f64 = 0.25;
/// The variance of one observed drive reduction.
pub const REWARD_VARIANCE: f64 = 0.25;
/// z of a one-sided 95% bound, for the gated variant (S09 X2's A3-gated).
pub const Z95_ONE_SIDED: f64 = 1.644_854;
/// Relaxation waits for E2 below this share of its bound.
pub const RELAX_COST_SHARE: f64 = 0.7;
/// An auxiliary signal is up when this share of the window shows it.
pub const AUX_SHARE: f64 = 0.15;
/// An auxiliary signal is also up when one of the last this many
/// resolutions shows it. The detectors can confirm a breach on the first
/// resolution after its onset, when the window still dilutes the signal.
pub const AUX_RECENT: usize = 5;
/// Retries are up when the window, or its last [`AUX_RECENT`] resolutions,
/// average more attempts than this.
pub const AUX_ATTEMPTS: f64 = 1.5;
/// The config fingerprint the SafetyBox compares when the caller gives
/// none: the controller never edits the config, so before equals after.
pub const UNCHANGED_CONFIG: &str = "unchanged";

/// Where the controller is (S06 §4.5).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    /// No confirmed breach: θ stays, or relaxes toward θ₀.
    #[default]
    Idle,
    /// An episode is open: moves are made and judged.
    Search,
    /// Nothing in the box helped: θ is the last-known-good until a person
    /// acknowledges or the S5 bounds change.
    Hold,
}

/// One notch of one knob: the unit of the posteriors and the tabu list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MoveKey {
    /// The knob.
    pub knob: Knob,
    /// The notch it moves to.
    pub direction: Step,
}

/// Why θ changed, as `param.change` rows write it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MoveReason {
    /// A guided step among the directed candidates.
    Directed,
    /// Ashby's uniform step over every admissible notch.
    Random,
    /// One notch back toward θ₀ after a calm.
    Relax,
    /// Undoing the last change.
    Rollback,
    /// Restoring the last-known-good.
    Restore,
}

/// Where a move's prior comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PriorSource {
    /// The catalog's effect signs.
    Catalog,
    /// The self-model's predicted drive change (8121).
    M3,
}

/// A move's prior over the drive reduction it brings.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct MovePriorValue {
    /// The expected drive reduction: positive helps.
    pub mean: f64,
    /// Its variance.
    pub variance: f64,
    /// Where it comes from.
    pub source: PriorSource,
}

/// A candidate move: its key, the catalog entry it comes from (none for an
/// Ashby step the catalog lacks) and the θ it leads to.
#[derive(Debug, Clone, PartialEq)]
pub struct Candidate {
    /// The move.
    pub key: MoveKey,
    /// Its catalog entry.
    pub entry: Option<Move>,
    /// θ after the move.
    pub next: HarnessParams,
}

/// The prior of a candidate move: the catalog sign until M3's priors
/// replace it (8121). A prior ranks candidates; it never adds one.
pub trait MovePrior: std::fmt::Debug + Send + Sync {
    /// The prior of `candidate` from `theta`, with `breached` the breached
    /// EVs.
    fn prior(
        &self,
        theta: &HarnessParams,
        candidate: &Candidate,
        breached: &[Ev],
    ) -> MovePriorValue;
}

/// The catalog-sign prior: +0.1 when the move's catalog effects counter
/// more breached EVs than they worsen, −0.1 for the reverse, 0 otherwise.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CatalogPrior;

impl MovePrior for CatalogPrior {
    fn prior(
        &self,
        _theta: &HarnessParams,
        candidate: &Candidate,
        breached: &[Ev],
    ) -> MovePriorValue {
        catalog_prior(candidate.entry.as_ref(), breached)
    }
}

/// The catalog-sign prior of `entry` against the `breached` EVs.
#[must_use]
pub fn catalog_prior(entry: Option<&Move>, breached: &[Ev]) -> MovePriorValue {
    let net: i32 = entry.map_or(0, |entry| {
        breached
            .iter()
            .map(|&ev| counters(entry.effect(ev), ev))
            .sum()
    });
    MovePriorValue {
        mean: CATALOG_PRIOR * f64::from(net.signum()),
        variance: PRIOR_VARIANCE,
        source: PriorSource::Catalog,
    }
}

/// +1 when `effect` moves `ev` back toward its bound, −1 when it moves it
/// away, 0 otherwise.
const fn counters(effect: Effect, ev: Ev) -> i32 {
    match (effect, ev.side()) {
        (Effect::Raises, Side::Lower) | (Effect::Lowers, Side::Upper) => 1,
        (Effect::Raises, Side::Upper) | (Effect::Lowers, Side::Lower) => -1,
        _ => 0,
    }
}

/// A move's discounted evidence: each new episode discounts the older ones
/// by γ (`garivier2011upper`).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct MovePosterior {
    /// The move.
    pub key: MoveKey,
    /// Discounted count of its evaluations (n).
    pub weight: f64,
    /// Discounted sum of the drive reductions observed after it.
    pub reward_sum: f64,
}

impl MovePosterior {
    /// No evidence on `key`.
    #[must_use]
    pub const fn empty(key: MoveKey) -> Self {
        Self {
            key,
            weight: 0.0,
            reward_sum: 0.0,
        }
    }

    /// The posterior mean and variance over `prior` (normal-normal).
    #[must_use]
    pub fn mean_variance(&self, prior: &MovePriorValue) -> (f64, f64) {
        let prior_precision = 1.0 / prior.variance.max(f64::EPSILON);
        let precision = prior_precision + self.weight / REWARD_VARIANCE;
        let mean = (prior.mean * prior_precision + self.reward_sum / REWARD_VARIANCE) / precision;
        (mean, 1.0 / precision)
    }
}

/// An open episode (S06 §4.5).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Episode {
    /// `ep-0007`.
    pub id: String,
    /// The EVs whose confirmed breach opened it.
    pub trigger: Vec<Ev>,
    /// Search moves made so far.
    pub changes: u32,
    /// Spend over the holdout arm's per resolution (the baseline's before
    /// any holdout row), net, over the episode.
    pub adaptation_spend_usd: f64,
    /// Moves rolled back, or unevaluable in shadow, in this episode.
    pub tabu: Vec<MoveKey>,
    /// Each knob's search directions in this episode, oldest first.
    pub directions: Vec<(Knob, Vec<Step>)>,
}

impl Episode {
    /// Whether `knob`'s search moves changed direction twice.
    #[must_use]
    pub fn locked(&self, knob: Knob) -> bool {
        self.directions
            .iter()
            .any(|(turned, steps)| *turned == knob && flips(steps) >= 2)
    }

    fn record_direction(&mut self, knob: Knob, step: Step) {
        match self.directions.iter_mut().find(|(k, _)| *k == knob) {
            Some((_, steps)) => steps.push(step),
            None => self.directions.push((knob, vec![step])),
        }
    }
}

fn flips(steps: &[Step]) -> usize {
    steps.windows(2).filter(|pair| pair[0] != pair[1]).count()
}

/// The change waiting out its dwell.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PendingChange {
    /// `ch-0019`.
    pub change_id: String,
    /// A search move or a relaxation.
    pub kind: ChangeKind,
    /// The move.
    pub key: MoveKey,
    /// Why it was made.
    pub reason: MoveReason,
    /// θ before it, which a rollback returns to.
    pub previous: HarnessParams,
    /// D_pre: D over the window when it was made, or more when a breached
    /// EV's shift began inside the window.
    pub drive_before: f64,
    /// The EVs breached when it was made.
    pub breached_before: Vec<Ev>,
    /// The adaptive resolutions since.
    pub after: Vec<TaskResolution>,
}

/// One θ change, applied or (in shadow) only proposed: a `param.change`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChangeProposal {
    /// `ch-0019`.
    pub change_id: String,
    /// The episode it belongs to; `None` for a relaxation.
    pub episode_id: Option<String>,
    /// What kind of change the SafetyBox judged it as.
    pub kind: ChangeKind,
    /// Why it was made.
    pub reason: MoveReason,
    /// Whether θ is swapped (`on`) or the change is only logged (`shadow`).
    pub applied: bool,
    /// The knob that moved: the first, when a restore moves several.
    pub knob: Knob,
    /// Its block.
    pub block: Block,
    /// Its value before, as records write it.
    pub from: Value,
    /// Its value after.
    pub to: Value,
    /// The SafetyBox's verdict.
    pub verdict: Verdict,
    /// The move's prior, the drive change it predicts being `−mean`.
    pub predicted: Option<MovePriorValue>,
    /// The move lowers cost, so the audit rate on the class doubles for the
    /// next passes (§4.6.5, 8127).
    pub audit_coupled: bool,
    /// θ after the change.
    pub theta: HarnessParams,
}

/// How a change was judged after its dwell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Evaluation {
    /// The drive fell enough, with no new breach.
    Kept,
    /// It did not: θ went back and the move is tabu.
    RolledBack,
    /// Shadow mode: the outcomes came from θ₀, so nothing can be judged.
    UnevaluableInShadow,
}

/// How an episode ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EpisodeOutcome {
    /// Back in bounds under a changed θ, which was committed.
    Recovered,
    /// Back in bounds, or abandoned at a restart, with θ at the
    /// last-known-good.
    RolledBack,
    /// Nothing in the box helped: HOLD.
    Hold,
}

/// Why the controller holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HoldReason {
    /// The episode made `max_changes_per_episode` changes (N_max).
    MaxChanges,
    /// Adaptation spend passed `adaptation_spend_max_frac` of spend (A_max).
    AdaptationSpend,
    /// No move is admissible.
    NoAdmissibleMove,
}

/// What released a HOLD.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReleaseReason {
    /// A person acknowledged it.
    Ack,
    /// The S5 bounds changed.
    PolicyChanged,
}

/// What the controller did on one input; the A-CTL rows (8114) are built
/// from these.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum ControllerEvent {
    /// An EV entered its breached state (`ev.breach`).
    Breach {
        /// The EV.
        ev: Ev,
        /// Its window estimate; `None` when unmeasured.
        value: Option<f64>,
        /// Its outer bound; for a slow breach, the inner bound it stayed
        /// outside of.
        bound: f64,
        /// Resolutions since its estimate last left the inner band.
        detect_delay_resolutions: u32,
        /// The open episode, if any.
        episode_id: Option<String>,
    },
    /// A breached EV came back inside its inner band (`ev.restore`).
    Restore {
        /// The EV.
        ev: Ev,
        /// Its window estimate.
        value: Option<f64>,
        /// Its inner bound.
        bound: f64,
        /// The open episode, if any.
        episode_id: Option<String>,
    },
    /// An episode opened (`homeostasis.episode`, `open`).
    EpisodeOpen {
        /// The episode.
        episode_id: String,
        /// The EVs that opened it.
        evs: Vec<Ev>,
    },
    /// An episode closed (`homeostasis.episode`, `close`).
    EpisodeClose {
        /// The episode.
        episode_id: String,
        /// The EVs that opened it.
        evs: Vec<Ev>,
        /// Search moves it made.
        changes: u32,
        /// Its net adaptation spend.
        adaptation_spend_usd: f64,
        /// How it ended.
        outcome: EpisodeOutcome,
    },
    /// θ changed, or would have in shadow (`param.change`).
    Change(Box<ChangeProposal>),
    /// A change was judged (`param.evaluate`).
    Evaluate {
        /// The change.
        change_id: String,
        /// The judgement.
        decision: Evaluation,
        /// D over the dwell minus D_pre, D before the change.
        d_drive: f64,
    },
    /// θ is the new last-known-good: 8113 commits it through the guarded
    /// store. Only in `on` mode.
    Commit {
        /// The θ to commit.
        theta: Box<HarnessParams>,
        /// `homeostat:<episode_id>/<change_id>`.
        reason: String,
    },
    /// The controller holds and alerts (`controller.hold`, `enter`).
    Hold {
        /// Why.
        reason: HoldReason,
        /// The episode that ended in it.
        episode_id: Option<String>,
    },
    /// A HOLD was released (`controller.hold`, `release`).
    Release {
        /// What released it.
        reason: ReleaseReason,
    },
    /// A person switched the mode (`controller.mode`).
    Mode {
        /// The mode before.
        from: HomeostasisMode,
        /// The mode after.
        to: HomeostasisMode,
    },
}

/// Everything the controller keeps across restarts, in
/// `.roko/learn/homeostat-state.json`. The estimators and detectors refill
/// from the resolutions that follow a restart.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ControllerState {
    /// [`STATE_SCHEMA`].
    pub schema_version: String,
    /// `off`, `shadow` or `on`; the config's at every start.
    pub mode: HomeostasisMode,
    /// IDLE, SEARCH or HOLD.
    pub phase: Phase,
    /// θ: applied in `on` mode, the would-be θ in shadow.
    pub theta: HarnessParams,
    /// θ₀.
    pub theta0: HarnessParams,
    /// The last-known-good θ, which HOLD and a restart return to.
    pub lkg: HarnessParams,
    /// The seed of every decision's generator.
    pub seed: u64,
    /// Decisions drawn so far: the next decision's ChaCha8 stream.
    pub decisions: u64,
    /// Adaptive-arm resolutions seen.
    pub resolutions: u64,
    /// The number of the last episode.
    pub next_episode: u64,
    /// The number of the last change.
    pub next_change: u64,
    /// The moves' discounted evidence.
    pub posteriors: Vec<MovePosterior>,
    /// The open episode.
    pub episode: Option<Episode>,
    /// The change waiting out its dwell.
    pub pending: Option<PendingChange>,
    /// Resolutions left in the dwell.
    pub dwell: u32,
    /// Unix ms before which the dwell has not ended.
    pub dwell_until_ms: Option<i64>,
    /// Resolutions left in which the last episode's EVs cannot open another.
    pub refractory: u32,
    /// The last episode's EVs.
    pub refractory_evs: Vec<Ev>,
    /// Resolutions in a row with D = 0 and no breach.
    pub recover_streak: u32,
    /// Resolutions in a row with no breach since the last change.
    pub calm_streak: u32,
    /// API-equivalent spend of every resolution seen, both arms.
    pub spend_usd: f64,
    /// Spend of the holdout rows seen.
    pub holdout_spend_usd: f64,
    /// Holdout rows seen.
    pub holdout_resolutions: u64,
    /// Unix ms of the latest resolution.
    pub now_ms: Option<i64>,
}

impl ControllerState {
    fn fresh(mode: HomeostasisMode, theta0: HarnessParams, seed: u64) -> Self {
        Self {
            schema_version: STATE_SCHEMA.to_string(),
            mode,
            phase: Phase::Idle,
            theta: theta0.clone(),
            lkg: theta0.clone(),
            theta0,
            seed,
            decisions: 0,
            resolutions: 0,
            next_episode: 0,
            next_change: 0,
            posteriors: Vec::new(),
            episode: None,
            pending: None,
            dwell: 0,
            dwell_until_ms: None,
            refractory: 0,
            refractory_evs: Vec::new(),
            recover_streak: 0,
            calm_streak: 0,
            spend_usd: 0.0,
            holdout_spend_usd: 0.0,
            holdout_resolutions: 0,
            now_ms: None,
        }
    }
}

/// A search move, why it was chosen, and its prior when it was guided.
type Choice = (Candidate, MoveReason, Option<MovePriorValue>);

/// M1's second-order loop over one stream of task resolutions (S06 §4.5).
#[derive(Debug)]
pub struct Controller {
    config: HomeostasisConfig,
    policy: ViabilityPolicy,
    safety: SafetyBox,
    ladders: HarnessLadders,
    baseline: Baseline,
    tuning: DetectorTuning,
    config_fingerprint: Option<String>,
    knowledge_loop_live: bool,
    guided_needs_lcb: bool,
    prior: Box<dyn MovePrior>,
    window: EvWindow,
    bands: EvBands,
    detectors: DetectorBank,
    latched: Vec<Ev>,
    left_inner: [Option<u64>; 4],
    state: ControllerState,
}

impl Controller {
    /// A fresh controller in `config`'s mode around `theta0` on `ladders`,
    /// with the S5 `policy`'s bounds, detectors watching for shifts from
    /// `baseline`, and every decision drawn from `seed`.
    #[must_use]
    pub fn new(
        config: &HomeostasisConfig,
        policy: ViabilityPolicy,
        theta0: HarnessParams,
        ladders: HarnessLadders,
        baseline: Baseline,
        seed: u64,
    ) -> Self {
        let state = ControllerState::fresh(config.mode, theta0, seed);
        Self::with_state(config, policy, ladders, baseline, state)
    }

    fn with_state(
        config: &HomeostasisConfig,
        policy: ViabilityPolicy,
        ladders: HarnessLadders,
        baseline: Baseline,
        state: ControllerState,
    ) -> Self {
        let tuning = DetectorTuning {
            confirm_k: config.confirm_k,
            ..DetectorTuning::default()
        };
        Self {
            config: config.clone(),
            safety: SafetyBox::new(state.theta0.clone(), ladders.clone(), &policy),
            ladders,
            baseline,
            tuning,
            config_fingerprint: Some(UNCHANGED_CONFIG.to_string()),
            knowledge_loop_live: false,
            guided_needs_lcb: false,
            prior: Box::new(CatalogPrior),
            window: EvWindow::new(config.window as usize),
            bands: EvBands::new(&policy.ev),
            detectors: DetectorBank::new(baseline, &policy.ev, &tuning),
            latched: Vec::new(),
            left_inner: [None; 4],
            policy,
            state,
        }
    }

    /// The controller with `prior` in place of the catalog-sign prior.
    #[must_use]
    pub fn with_prior(mut self, prior: Box<dyn MovePrior>) -> Self {
        self.prior = prior;
        self
    }

    /// The controller comparing `fingerprint`, the run config's
    /// `policy::non_m1_fingerprint`, before and after every change. `None`
    /// makes the SafetyBox refuse every change.
    #[must_use]
    pub fn with_config_fingerprint(mut self, fingerprint: Option<String>) -> Self {
        self.config_fingerprint = fingerprint;
        self
    }

    /// Whether M2 rates the knowledge loop live, so that a search may
    /// switch the knowledge section on (§4.6.8).
    #[must_use]
    pub fn with_knowledge_loop_live(mut self, live: bool) -> Self {
        self.knowledge_loop_live = live;
        self
    }

    /// Take a guided step only when the move's prior has a one-sided 95%
    /// lower bound above 0, an Ashby step otherwise: the replay's A3-gated
    /// variant (S09 X2).
    #[must_use]
    pub fn with_guided_needs_lcb(mut self, gated: bool) -> Self {
        self.guided_needs_lcb = gated;
        self
    }

    /// `<roko_dir>/learn/homeostat-state.json`.
    #[must_use]
    pub fn state_path(roko_dir: &Path) -> PathBuf {
        roko_dir.join(STATE_FILE)
    }

    /// The controller over the state saved at `path`, with what its restart
    /// did. With no usable state (none, unreadable, another θ₀, or a θ off
    /// the ladders) it is a fresh controller and nothing happened. The mode
    /// is `config`'s. An open episode or a pending change is abandoned and θ
    /// returns to the last-known-good; 8113 restores the committed θ over it.
    #[must_use]
    pub fn load(
        path: &Path,
        config: &HomeostasisConfig,
        policy: ViabilityPolicy,
        theta0: HarnessParams,
        ladders: HarnessLadders,
        baseline: Baseline,
        seed: u64,
    ) -> (Self, Vec<ControllerEvent>) {
        let saved = std::fs::read_to_string(path)
            .ok()
            .and_then(|text| serde_json::from_str::<ControllerState>(&text).ok())
            .filter(|state| {
                state.schema_version == STATE_SCHEMA
                    && state.theta0 == theta0
                    && state.theta.validate(&ladders).is_ok()
                    && state.lkg.validate(&ladders).is_ok()
            });
        let Some(mut state) = saved else {
            let fresh = Self::new(config, policy, theta0, ladders, baseline, seed);
            return (fresh, Vec::new());
        };
        state.mode = config.mode;
        let mut controller = Self::with_state(config, policy, ladders, baseline, state);
        let events = controller.resume();
        (controller, events)
    }

    /// Write the state to `path`, through a temp file renamed into place.
    ///
    /// # Errors
    ///
    /// I/O errors creating the directory or writing the file.
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let text = serde_json::to_string_pretty(&self.state).map_err(std::io::Error::other)?;
        let temp = path.with_extension("json.tmp");
        std::fs::write(&temp, text)?;
        std::fs::rename(&temp, path)
    }

    /// The state a restart keeps.
    #[must_use]
    pub const fn state(&self) -> &ControllerState {
        &self.state
    }

    /// θ: applied in `on` mode, the would-be θ in shadow.
    #[must_use]
    pub const fn theta(&self) -> &HarnessParams {
        &self.state.theta
    }

    /// IDLE, SEARCH or HOLD.
    #[must_use]
    pub const fn phase(&self) -> Phase {
        self.state.phase
    }

    /// `off`, `shadow` or `on`.
    #[must_use]
    pub const fn mode(&self) -> HomeostasisMode {
        self.state.mode
    }

    /// The open episode.
    #[must_use]
    pub const fn episode(&self) -> Option<&Episode> {
        self.state.episode.as_ref()
    }

    /// The S5 policy the controller reads.
    #[must_use]
    pub const fn policy(&self) -> &ViabilityPolicy {
        &self.policy
    }

    /// Every EV's estimate over the window, E3 judged against the S5
    /// bounds, as `ev.update` events show it (8130).
    #[must_use]
    pub fn estimates(&self) -> EvEstimates {
        self.window.estimate(&self.policy.ev)
    }

    /// The breached EVs, E1 first: past the outer bound and not yet back
    /// in the inner band, by the window's band or a confirmed detector, or
    /// latched as a slow breach.
    #[must_use]
    pub fn breached(&self) -> Vec<Ev> {
        Ev::ALL
            .into_iter()
            .filter(|&ev| self.bands.get(ev).breached || self.latched.contains(&ev))
            .collect()
    }

    /// Make `theta` both θ and the last-known-good, as a start does after
    /// restoring a committed θ (8113).
    ///
    /// # Errors
    ///
    /// The SafetyBox's verdict when it would not restore `theta` from θ₀
    /// (off the ladders, a budget share above θ₀'s, an audit rate above
    /// S5's ceiling); θ then stays.
    pub fn adopt(&mut self, theta: HarnessParams) -> Result<(), Verdict> {
        let context = self.context(ChangeKind::Restore);
        let verdict = self.safety.validate(&self.state.theta0, &theta, &context);
        if !verdict.passed() {
            return Err(verdict);
        }
        self.state.lkg = theta.clone();
        self.state.theta = theta;
        Ok(())
    }

    /// A person acknowledged the HOLD: back to IDLE, with the refractory
    /// period running.
    pub fn ack(&mut self) -> Vec<ControllerEvent> {
        self.release(ReleaseReason::Ack)
    }

    /// The S5 policy changed: new bands and detectors, each EV's time
    /// outside its inner band counted afresh, and a HOLD is released.
    pub fn policy_changed(&mut self, policy: ViabilityPolicy) -> Vec<ControllerEvent> {
        self.safety = SafetyBox::new(self.state.theta0.clone(), self.ladders.clone(), &policy);
        self.bands = EvBands::new(&policy.ev);
        self.detectors = DetectorBank::new(self.baseline, &policy.ev, &self.tuning);
        self.latched.clear();
        self.left_inner = [None; 4];
        self.policy = policy;
        self.release(ReleaseReason::PolicyChanged)
    }

    /// Swap the detectors' tuning, as the feedforward pre-arm does (8121):
    /// the detectors restart under it.
    pub fn set_detector_tuning(&mut self, tuning: DetectorTuning) {
        self.tuning = tuning;
        self.detectors = DetectorBank::new(self.baseline, &self.policy.ev, &tuning);
    }

    /// A person switched the mode; M1 never switches its own.
    pub fn set_mode(&mut self, mode: HomeostasisMode) -> Vec<ControllerEvent> {
        let from = self.state.mode;
        if from == mode {
            return Vec::new();
        }
        self.state.mode = mode;
        vec![ControllerEvent::Mode { from, to: mode }]
    }

    /// Take one settled resolution and return what the controller did.
    pub fn on_resolution(&mut self, resolution: &TaskResolution) -> Vec<ControllerEvent> {
        let mut events = Vec::new();
        if self.state.mode == HomeostasisMode::Off {
            return events;
        }
        let cost = resolution.api_equiv_usd.unwrap_or(0.0);
        self.state.spend_usd += cost;
        if resolution.holdout() {
            self.state.holdout_spend_usd += cost;
            self.state.holdout_resolutions += 1;
            return events;
        }
        self.state.resolutions += 1;
        if let Some(at) = resolution.resolved_at {
            self.state.now_ms = Some(at);
        }
        let estimates = self.observe(resolution, &mut events);
        let current = drive(&estimates, &self.policy.ev, &self.policy.drive);
        self.track(cost, &current);
        if let Some(pending) = self.state.pending.as_mut() {
            pending.after.push(resolution.clone());
        }
        if !self.dwell_over() {
            return events;
        }
        match self.state.phase {
            Phase::Idle => self.idle(&estimates, &current, &mut events),
            Phase::Search => self.search(&estimates, &current, &mut events),
            Phase::Hold => {}
        }
        events
    }

    /// Feed the window, the bands and the detectors; latch confirmed
    /// breaches, and record each EV that enters or leaves its breached
    /// state.
    fn observe(
        &mut self,
        resolution: &TaskResolution,
        events: &mut Vec<ControllerEvent>,
    ) -> EvEstimates {
        let before = self.breached();
        self.window.push(resolution);
        let estimates = self.window.estimate(&self.policy.ev);
        let false_green = Some(&estimates.false_green).filter(|e| e.is_measured());
        let observation = Observation::of(resolution);
        let report = self.detectors.observe(&observation, false_green);
        for ev in report.confirmed {
            if !self.latched.contains(&ev) {
                self.latched.push(ev);
            }
        }
        if self.window.is_full() {
            self.bands.update(&estimates);
            self.track_inner(&estimates);
        }
        let after = self.breached();
        let episode_id = self.episode().map(|e| e.id.clone());
        for ev in Ev::ALL {
            let value = estimates.get(ev).value;
            if after.contains(&ev) && !before.contains(&ev) {
                events.push(ControllerEvent::Breach {
                    ev,
                    value,
                    bound: self.policy.ev.outer(ev),
                    detect_delay_resolutions: self.delay(ev),
                    episode_id: episode_id.clone(),
                });
            } else if before.contains(&ev) && !after.contains(&ev) {
                events.push(ControllerEvent::Restore {
                    ev,
                    value,
                    bound: self.policy.ev.inner(ev),
                    episode_id: episode_id.clone(),
                });
            }
        }
        estimates
    }

    /// Clear a latched breach once its EV is back in the inner band, and
    /// note when each measured EV leaves the band.
    fn track_inner(&mut self, estimates: &EvEstimates) {
        for ev in Ev::ALL {
            let estimate = estimates.get(ev);
            if self.bands.get(ev).within_inner(estimate) {
                self.latched.retain(|&latched| latched != ev);
                self.left_inner[slot(ev)] = None;
            } else if estimate.is_measured() && self.left_inner[slot(ev)].is_none() {
                self.left_inner[slot(ev)] = Some(self.state.resolutions);
            }
        }
    }

    /// Resolutions since `ev`'s estimate last left its inner band.
    fn delay(&self, ev: Ev) -> u32 {
        let Some(left) = self.left_inner[slot(ev)] else {
            return 0;
        };
        u32::try_from(self.state.resolutions.saturating_sub(left) + 1).unwrap_or(u32::MAX)
    }

    /// Latch every EV whose estimate has stayed outside its inner band for
    /// `relax_window` resolutions without a breach: a slow breach, whose
    /// `ev.breach` names the inner bound. Such an EV keeps D above 0, yet
    /// its band breaches only past the outer bound and its detector only on
    /// a shift beyond the drift it allows, so M1 would wait forever: in IDLE
    /// for a breach, in SEARCH for a recovery (gap-d1ebc1). The wait is
    /// W_relax, as long as a relaxation waits: two windows of evidence,
    /// which an in-bounds stream almost never shows.
    fn latch_slow(&mut self, estimates: &EvEstimates, events: &mut Vec<ControllerEvent>) {
        let patience = self.config.relax_window.max(1);
        let breached = self.breached();
        for ev in Ev::ALL {
            let estimate = estimates.get(ev);
            let outside = estimate.is_measured() && !self.bands.get(ev).within_inner(estimate);
            if !outside || breached.contains(&ev) || self.delay(ev) < patience {
                continue;
            }
            self.latched.push(ev);
            self.state.calm_streak = 0;
            events.push(ControllerEvent::Breach {
                ev,
                value: estimate.value,
                bound: self.policy.ev.inner(ev),
                detect_delay_resolutions: self.delay(ev),
                episode_id: self.episode().map(|e| e.id.clone()),
            });
        }
    }

    /// Count the resolution toward the episode's adaptation spend, the
    /// refractory period and the calm and recovery streaks.
    fn track(&mut self, cost: f64, current: &Drive) {
        let excess = cost - self.cost_baseline();
        if let Some(episode) = self.state.episode.as_mut() {
            episode.adaptation_spend_usd += excess;
        }
        self.state.refractory = self.state.refractory.saturating_sub(1);
        let calm = self.breached().is_empty();
        let measured = current.unmeasured.len() < Ev::ALL.len();
        let recovered = calm && measured && current.value <= 0.0;
        self.state.calm_streak = if calm { self.state.calm_streak + 1 } else { 0 };
        self.state.recover_streak = if recovered {
            self.state.recover_streak + 1
        } else {
            0
        };
    }

    /// The holdout arm's mean spend per resolution; the baseline's before
    /// any holdout row.
    fn cost_baseline(&self) -> f64 {
        if self.state.holdout_resolutions == 0 {
            return self.baseline.usd_per_resolution;
        }
        self.state.holdout_spend_usd / self.state.holdout_resolutions as f64
    }

    /// Count one resolution against the dwell; true once it has run out in
    /// resolutions and in seconds.
    fn dwell_over(&mut self) -> bool {
        self.state.dwell = self.state.dwell.saturating_sub(1);
        if self.state.dwell > 0 {
            return false;
        }
        match (self.state.dwell_until_ms, self.state.now_ms) {
            (Some(until), Some(now)) => now >= until,
            _ => true,
        }
    }

    /// IDLE: judge a relaxation whose dwell ended and latch slow breaches,
    /// then open an episode on a latched breach outside the refractory
    /// period, or relax.
    fn idle(
        &mut self,
        estimates: &EvEstimates,
        current: &Drive,
        events: &mut Vec<ControllerEvent>,
    ) {
        if let Some(pending) = self.state.pending.take()
            && self.evaluate(pending, estimates, events)
        {
            self.commit(events);
        }
        self.latch_slow(estimates, events);
        let refractory = self.state.refractory;
        let triggers: Vec<Ev> = self
            .latched
            .iter()
            .copied()
            .filter(|ev| refractory == 0 || !self.state.refractory_evs.contains(ev))
            .collect();
        if !triggers.is_empty() {
            self.open_episode(triggers, current, events);
        } else if self.relax_due(estimates) {
            self.relax(current, events);
        }
    }

    /// Open an episode on `trigger`: discount older evidence by γ, keep θ
    /// as the last-known-good and make the first move.
    fn open_episode(
        &mut self,
        trigger: Vec<Ev>,
        current: &Drive,
        events: &mut Vec<ControllerEvent>,
    ) {
        let discount = self.config.posterior_discount;
        for posterior in &mut self.state.posteriors {
            posterior.weight *= discount;
            posterior.reward_sum *= discount;
        }
        self.state.next_episode += 1;
        let id = format!("ep-{:04}", self.state.next_episode);
        events.push(ControllerEvent::EpisodeOpen {
            episode_id: id.clone(),
            evs: trigger.clone(),
        });
        self.state.lkg = self.state.theta.clone();
        self.state.episode = Some(Episode {
            id,
            trigger,
            changes: 0,
            adaptation_spend_usd: 0.0,
            tabu: Vec::new(),
            directions: Vec::new(),
        });
        self.state.phase = Phase::Search;
        self.state.recover_streak = 0;
        self.make_move(current, events);
    }

    /// SEARCH: judge the last change and latch slow breaches, then commit
    /// on recovery, wait while nothing is breached, HOLD at N_max or A_max,
    /// or move again.
    fn search(
        &mut self,
        estimates: &EvEstimates,
        current: &Drive,
        events: &mut Vec<ControllerEvent>,
    ) {
        if let Some(pending) = self.state.pending.take() {
            self.evaluate(pending, estimates, events);
        }
        self.latch_slow(estimates, events);
        if self.state.recover_streak >= self.config.recover_window {
            self.close_recovered(events);
        } else if self.breached().is_empty() {
            self.state.dwell = self.config.dwell_resolutions;
            self.state.dwell_until_ms = None;
        } else if let Some(reason) = self.hold_reason() {
            self.hold(reason, events);
        } else {
            self.make_move(current, events);
        }
    }

    /// Why the episode must HOLD: N_max changes, or adaptation spend over
    /// A_max of spend.
    fn hold_reason(&self) -> Option<HoldReason> {
        let episode = self.episode()?;
        if episode.changes >= self.config.max_changes_per_episode {
            return Some(HoldReason::MaxChanges);
        }
        let cap = self.config.adaptation_spend_max_frac * self.state.spend_usd;
        (episode.adaptation_spend_usd > cap).then_some(HoldReason::AdaptationSpend)
    }

    /// Make a search move, or HOLD when none is admissible.
    fn make_move(&mut self, current: &Drive, events: &mut Vec<ControllerEvent>) {
        let breached = self.breached();
        match self.choose_move(&breached) {
            Some((candidate, reason, prior)) => self.apply(
                candidate,
                ChangeKind::Search,
                reason,
                prior,
                current,
                &breached,
                events,
            ),
            None => self.hold(HoldReason::NoAdmissibleMove, events),
        }
    }

    /// The next search move: a guided one with probability
    /// 1 − `random_step_prob` when a directed candidate is admissible, an
    /// Ashby step otherwise; `None` when nothing is admissible.
    fn choose_move(&mut self, breached: &[Ev]) -> Option<Choice> {
        let directed = self.directed_candidates(breached);
        let mut rng = self.next_rng();
        let draw: f64 = rng.r#gen();
        if draw >= self.config.random_step_prob
            && let Some((candidate, prior)) = self.guided(&directed, breached, &mut rng)
        {
            return Some((candidate, MoveReason::Directed, Some(prior)));
        }
        let mut steps = self.ashby_candidates();
        if steps.is_empty() {
            return None;
        }
        let pick = rng.gen_range(0..steps.len());
        Some((steps.swap_remove(pick), MoveReason::Random, None))
    }

    /// The generator of the next decision: ChaCha8 from the seed, on stream
    /// number `decisions`, so a decision depends only on the seed and its
    /// index, across restarts too.
    fn next_rng(&mut self) -> ChaCha8Rng {
        let mut rng = ChaCha8Rng::seed_from_u64(self.state.seed);
        rng.set_stream(self.state.decisions);
        self.state.decisions += 1;
        rng
    }

    /// The catalog's moves for `breached`, one per knob each may turn, in
    /// the catalog's order, less those that are not admissible.
    fn directed_candidates(&self, breached: &[Ev]) -> Vec<Candidate> {
        let signature = self.signature(breached);
        let mut found = Vec::new();
        for entry in candidates(&signature) {
            for knob in entry.knobs(&self.state.theta) {
                let key = MoveKey {
                    knob,
                    direction: entry.direction,
                };
                found.extend(self.admissible(key, Some(entry)));
            }
        }
        found
    }

    /// Every admissible single-notch step of θ: Ashby's step function.
    fn ashby_candidates(&self) -> Vec<Candidate> {
        let mut found = Vec::new();
        for knob in self.state.theta.knobs() {
            for direction in [Step::Up, Step::Down] {
                let key = MoveKey { knob, direction };
                found.extend(self.admissible(key, catalog_move(knob.kind(), direction)));
            }
        }
        found
    }

    /// The candidate of `key` when its notch exists, it is neither tabu nor
    /// on a locked knob, and the SafetyBox admits it as a search move.
    fn admissible(&self, key: MoveKey, entry: Option<Move>) -> Option<Candidate> {
        if let Some(episode) = self.episode()
            && (episode.tabu.contains(&key) || episode.locked(key.knob))
        {
            return None;
        }
        let theta = &self.state.theta;
        let next = theta.step(key.knob, key.direction, &self.ladders).ok()?;
        if !self.validate(&next, ChangeKind::Search).passed() {
            return None;
        }
        Some(Candidate { key, entry, next })
    }

    /// Thompson sampling over `directed`: each candidate's drive reduction
    /// is drawn from its posterior and the largest wins. While no candidate
    /// has evidence the highest prior mean wins, the first on a tie.
    fn guided(
        &self,
        directed: &[Candidate],
        breached: &[Ev],
        rng: &mut ChaCha8Rng,
    ) -> Option<(Candidate, MovePriorValue)> {
        let evidence = directed
            .iter()
            .any(|candidate| self.posterior(candidate.key).weight > 0.0);
        let mut best: Option<(f64, usize, MovePriorValue)> = None;
        for (index, candidate) in directed.iter().enumerate() {
            let prior = self.prior.prior(&self.state.theta, candidate, breached);
            let lower = prior.mean - Z95_ONE_SIDED * prior.variance.sqrt();
            if self.guided_needs_lcb && lower <= 0.0 {
                continue;
            }
            let (mean, variance) = self.posterior(candidate.key).mean_variance(&prior);
            let score = if evidence {
                mean + variance.sqrt() * gaussian(rng)
            } else {
                mean
            };
            if best.is_none_or(|(top, _, _)| score > top) {
                best = Some((score, index, prior));
            }
        }
        best.map(|(_, index, prior)| (directed[index].clone(), prior))
    }

    fn posterior(&self, key: MoveKey) -> MovePosterior {
        self.state
            .posteriors
            .iter()
            .find(|posterior| posterior.key == key)
            .copied()
            .unwrap_or(MovePosterior::empty(key))
    }

    /// Add one observed drive reduction to `key`'s evidence.
    fn learn(&mut self, key: MoveKey, reward: f64) {
        match self.state.posteriors.iter_mut().find(|p| p.key == key) {
            Some(posterior) => {
                posterior.weight += 1.0;
                posterior.reward_sum += reward;
            }
            None => self.state.posteriors.push(MovePosterior {
                key,
                weight: 1.0,
                reward_sum: reward,
            }),
        }
    }

    /// The breach signature, with the auxiliary signals: each is up when
    /// [`AUX_SHARE`] of the window shows it or one of the last
    /// [`AUX_RECENT`] resolutions does, so a breach confirmed one resolution
    /// into a provider fault already reads as an outage.
    fn signature(&self, breached: &[Ev]) -> Signature {
        let window = self.window.resolutions();
        let recent = &window[window.len().saturating_sub(AUX_RECENT)..];
        let up = |hit: &dyn Fn(&TaskResolution) -> bool| {
            share(window, hit) >= AUX_SHARE || recent.iter().any(hit)
        };
        let retries_up =
            mean_attempts(window) > AUX_ATTEMPTS || mean_attempts(recent) > AUX_ATTEMPTS;
        Signature {
            provider_errors: up(&|r| r.provider_errors > 0),
            budget_exhausted: up(&|r| r.final_verdict == AttemptOutcome::BudgetExhausted),
            retries_up,
            ..Signature::of(breached)
        }
    }

    fn context(&self, kind: ChangeKind) -> SafetyContext {
        SafetyContext {
            kind,
            config_before: self.config_fingerprint.clone(),
            config_after: self.config_fingerprint.clone(),
            arm: None,
            adaptation_spend_usd: self.adaptation_spend(),
            run_spend_usd: self.state.spend_usd,
            adaptation_spend_max_frac: self.config.adaptation_spend_max_frac,
            knowledge_loop_live: self.knowledge_loop_live,
        }
    }

    fn adaptation_spend(&self) -> f64 {
        self.episode().map_or(0.0, |e| e.adaptation_spend_usd)
    }

    /// The SafetyBox's verdict on moving θ to `next` as a `kind` change.
    fn validate(&self, next: &HarnessParams, kind: ChangeKind) -> Verdict {
        self.safety
            .validate(&self.state.theta, next, &self.context(kind))
    }

    /// Apply a search move or a relaxation: log it, move θ (only the
    /// would-be θ in shadow), restart the detectors in `on` mode, and dwell.
    fn apply(
        &mut self,
        candidate: Candidate,
        kind: ChangeKind,
        reason: MoveReason,
        prior: Option<MovePriorValue>,
        current: &Drive,
        breached: &[Ev],
        events: &mut Vec<ControllerEvent>,
    ) {
        let change_id = self.next_change_id();
        let mut proposal = self.proposal(
            change_id.clone(),
            kind,
            reason,
            candidate.key.knob,
            &candidate.next,
        );
        proposal.predicted = prior;
        proposal.audit_coupled = candidate.entry.is_some_and(|entry| entry.audit_coupled());
        events.push(ControllerEvent::Change(Box::new(proposal)));
        if kind == ChangeKind::Search
            && let Some(episode) = self.state.episode.as_mut()
        {
            episode.changes += 1;
            episode.record_direction(candidate.key.knob, candidate.key.direction);
        }
        self.state.pending = Some(PendingChange {
            change_id,
            kind,
            key: candidate.key,
            reason,
            previous: self.state.theta.clone(),
            drive_before: self.drive_before(current, breached),
            breached_before: breached.to_vec(),
            after: Vec::new(),
        });
        self.state.theta = candidate.next;
        self.start_dwell();
        self.state.calm_streak = 0;
        if self.state.mode == HomeostasisMode::On {
            self.detectors.reset_all();
        }
    }

    /// The `param.change` of moving θ to `next` by `knob`, with the
    /// SafetyBox's verdict.
    fn proposal(
        &self,
        change_id: String,
        kind: ChangeKind,
        reason: MoveReason,
        knob: Knob,
        next: &HarnessParams,
    ) -> ChangeProposal {
        ChangeProposal {
            change_id,
            episode_id: self.episode().map(|e| e.id.clone()),
            kind,
            reason,
            applied: self.state.mode == HomeostasisMode::On,
            knob,
            block: knob.block(),
            from: self.state.theta.value(knob).unwrap_or(Value::Null),
            to: next.value(knob).unwrap_or(Value::Null),
            verdict: self.validate(next, kind),
            predicted: None,
            audit_coupled: false,
            theta: next.clone(),
        }
    }

    fn start_dwell(&mut self) {
        self.state.dwell = self.config.dwell_resolutions;
        let wait_ms = self.config.dwell_min_secs.saturating_mul(1000);
        let wait_ms = i64::try_from(wait_ms).unwrap_or(i64::MAX);
        self.state.dwell_until_ms = self.state.now_ms.map(|now| now.saturating_add(wait_ms));
    }

    fn next_change_id(&mut self) -> String {
        self.state.next_change += 1;
        format!("ch-{:04}", self.state.next_change)
    }

    /// Judge the pending change after its dwell: kept, or rolled back and
    /// made tabu; unevaluable in shadow, where the would-be θ drops it and
    /// the move is tabu. Returns whether it was kept.
    fn evaluate(
        &mut self,
        pending: PendingChange,
        estimates: &EvEstimates,
        events: &mut Vec<ControllerEvent>,
    ) -> bool {
        let after = self.drive_over(&pending.after, estimates);
        let before = pending.drive_before;
        let breached = self.breached();
        let collateral = breached
            .iter()
            .any(|ev| !pending.breached_before.contains(ev));
        let improved = after <= before - self.config.improve_delta_frac * before && !collateral;
        let decision = if self.state.mode != HomeostasisMode::On {
            Evaluation::UnevaluableInShadow
        } else if improved {
            Evaluation::Kept
        } else {
            Evaluation::RolledBack
        };
        events.push(ControllerEvent::Evaluate {
            change_id: pending.change_id.clone(),
            decision,
            d_drive: after - before,
        });
        match decision {
            Evaluation::Kept => self.learn(pending.key, before - after),
            Evaluation::RolledBack => {
                self.learn(pending.key, before - after);
                self.make_tabu(pending.key);
                let back = pending.previous;
                self.change_to(back, ChangeKind::Rollback, MoveReason::Rollback, events);
            }
            Evaluation::UnevaluableInShadow => {
                self.make_tabu(pending.key);
                self.state.theta = pending.previous;
            }
        }
        decision == Evaluation::Kept
    }

    /// D over `resolutions` alone, with E3 from the window.
    fn drive_over(&self, resolutions: &[TaskResolution], estimates: &EvEstimates) -> f64 {
        let false_green = estimates.false_green;
        let dwell = EvEstimates {
            pass_rate: estimate_pass_rate(resolutions),
            usd_per_verified_success: estimate_usd_per_verified_success(
                resolutions,
                false_green.value,
            ),
            false_green,
            latency_p90_s: estimate_latency_p90_s(resolutions),
        };
        drive(&dwell, &self.policy.ev, &self.policy.drive).value
    }

    /// D_pre, the drive a change made now is judged against: the window's,
    /// except that each breached EV whose estimate left its inner band
    /// inside the window also reads over the resolutions since, at the end
    /// of their 90% interval nearer the band, and counts at the worse of
    /// its two readings. When a step's onset lies inside the window, the
    /// window averages in the resolutions before it and understates the
    /// shift, so a good first move could show too little gain and be rolled
    /// back (gap-d1ebc1); the interval's end keeps a short, unlucky stretch
    /// from overstating it instead.
    fn drive_before(&self, current: &Drive, breached: &[Ev]) -> f64 {
        let mut adjusted: Option<EvEstimates> = None;
        for &ev in breached {
            let Some(since) = self.since_left_inner(ev) else {
                continue;
            };
            let estimates = adjusted.get_or_insert_with(|| self.estimates());
            let false_green = estimates.false_green.value;
            let (reading, entry) = match ev {
                Ev::PassRate => (estimate_pass_rate(since), &mut estimates.pass_rate),
                Ev::UsdPerVerifiedSuccess => (
                    estimate_usd_per_verified_success(since, false_green),
                    &mut estimates.usd_per_verified_success,
                ),
                Ev::LatencyP90S => (estimate_latency_p90_s(since), &mut estimates.latency_p90_s),
                Ev::FalseGreen => continue,
            };
            *entry = worse(*entry, favourable(reading));
        }
        adjusted.map_or(current.value, |estimates| {
            drive(&estimates, &self.policy.ev, &self.policy.drive).value
        })
    }

    /// The window's resolutions since `ev`'s estimate left its inner band,
    /// when it left inside the window, after its first resolution.
    fn since_left_inner(&self, ev: Ev) -> Option<&[TaskResolution]> {
        let window = self.window.resolutions();
        let left = self.left_inner[slot(ev)]?;
        let held = u64::try_from(window.len()).ok()?;
        let first = (self.state.resolutions + 1).checked_sub(held)?;
        let start = usize::try_from(left.checked_sub(first)?).ok()?;
        window
            .get(start..)
            .filter(|since| start > 0 && !since.is_empty())
    }

    fn make_tabu(&mut self, key: MoveKey) {
        if let Some(episode) = self.state.episode.as_mut()
            && !episode.tabu.contains(&key)
        {
            episode.tabu.push(key);
        }
    }

    /// Move θ to `target` by a rollback or a restore, logged as a change.
    fn change_to(
        &mut self,
        target: HarnessParams,
        kind: ChangeKind,
        reason: MoveReason,
        events: &mut Vec<ControllerEvent>,
    ) {
        let Some(&knob) = self.state.theta.changed_knobs(&target).first() else {
            return;
        };
        let change_id = self.next_change_id();
        let proposal = self.proposal(change_id, kind, reason, knob, &target);
        events.push(ControllerEvent::Change(Box::new(proposal)));
        self.state.theta = target;
        if self.state.mode == HomeostasisMode::On {
            self.detectors.reset_all();
        }
    }

    /// Restore the last-known-good, HOLD and alert, closing the episode.
    fn hold(&mut self, reason: HoldReason, events: &mut Vec<ControllerEvent>) {
        let lkg = self.state.lkg.clone();
        self.change_to(lkg, ChangeKind::Restore, MoveReason::Restore, events);
        self.state.pending = None;
        let episode_id = self.episode().map(|e| e.id.clone());
        events.push(ControllerEvent::Hold { reason, episode_id });
        self.close_episode(EpisodeOutcome::Hold, events);
        self.state.phase = Phase::Hold;
    }

    fn close_episode(&mut self, outcome: EpisodeOutcome, events: &mut Vec<ControllerEvent>) {
        let Some(episode) = self.state.episode.take() else {
            return;
        };
        self.state.refractory = self.config.refractory;
        self.state.refractory_evs = episode.trigger.clone();
        events.push(ControllerEvent::EpisodeClose {
            episode_id: episode.id,
            evs: episode.trigger,
            changes: episode.changes,
            adaptation_spend_usd: episode.adaptation_spend_usd,
            outcome,
        });
    }

    /// Back in bounds for `recover_window` resolutions: commit a changed θ
    /// and close the episode.
    fn close_recovered(&mut self, events: &mut Vec<ControllerEvent>) {
        let outcome = if self.state.theta == self.state.lkg {
            EpisodeOutcome::RolledBack
        } else {
            self.commit(events);
            EpisodeOutcome::Recovered
        };
        self.close_episode(outcome, events);
        self.state.phase = Phase::Idle;
    }

    /// θ is the last-known-good; in `on` mode 8113 commits it through the
    /// guarded store, with reason `homeostat:<episode_id>/<change_id>`.
    fn commit(&mut self, events: &mut Vec<ControllerEvent>) {
        self.state.lkg = self.state.theta.clone();
        if self.state.mode != HomeostasisMode::On {
            return;
        }
        let episode = self.episode().map_or("relax", |e| e.id.as_str());
        let reason = format!("homeostat:{episode}/ch-{:04}", self.state.next_change);
        events.push(ControllerEvent::Commit {
            theta: Box::new(self.state.theta.clone()),
            reason,
        });
    }

    /// Whether to relax: θ is off θ₀, `relax_window` calm resolutions have
    /// passed since the last change, and E2 is below 0.7 of its bound.
    fn relax_due(&self, estimates: &EvEstimates) -> bool {
        let cheap = self.policy.ev.outer(Ev::UsdPerVerifiedSuccess) * RELAX_COST_SHARE;
        let cost = estimates.usd_per_verified_success.value;
        let calm = self.state.calm_streak >= self.config.relax_window;
        self.state.theta != self.state.theta0 && calm && cost.is_some_and(|usd| usd < cheap)
    }

    /// Step one cost-raising knob one notch back toward θ₀, judged like a
    /// search move.
    fn relax(&mut self, current: &Drive, events: &mut Vec<ControllerEvent>) {
        match self.relaxation() {
            Some(candidate) => self.apply(
                candidate,
                ChangeKind::Relax,
                MoveReason::Relax,
                None,
                current,
                &[],
                events,
            ),
            None => self.state.calm_streak = 0,
        }
    }

    /// The first knob of θ off θ₀ whose move away from θ₀ raises cost,
    /// stepped one notch back, when the SafetyBox admits it.
    fn relaxation(&self) -> Option<Candidate> {
        let theta = &self.state.theta;
        let theta0 = &self.state.theta0;
        theta.changed_knobs(theta0).into_iter().find_map(|knob| {
            let here = theta.notch(knob, &self.ladders).ok()?;
            let home = theta0.notch(knob, &self.ladders).ok()?;
            let (direction, away) = if here > home {
                (Step::Down, Step::Up)
            } else {
                (Step::Up, Step::Down)
            };
            let entry = catalog_move(knob.kind(), away)?;
            if entry.effect(Ev::UsdPerVerifiedSuccess) != Effect::Raises {
                return None;
            }
            let next = theta.step(knob, direction, &self.ladders).ok()?;
            if !self.validate(&next, ChangeKind::Relax).passed() {
                return None;
            }
            Some(Candidate {
                key: MoveKey { knob, direction },
                entry: catalog_move(knob.kind(), direction),
                next,
            })
        })
    }

    fn release(&mut self, reason: ReleaseReason) -> Vec<ControllerEvent> {
        if self.state.phase != Phase::Hold {
            return Vec::new();
        }
        self.state.phase = Phase::Idle;
        self.latched.clear();
        self.detectors.reset_all();
        vec![ControllerEvent::Release { reason }]
    }

    /// After a restart: an open episode or a pending change is abandoned
    /// and θ returns to the last-known-good.
    fn resume(&mut self) -> Vec<ControllerEvent> {
        let mut events = Vec::new();
        self.state.pending = None;
        self.state.dwell = 0;
        self.state.dwell_until_ms = None;
        let lkg = self.state.lkg.clone();
        self.change_to(lkg, ChangeKind::Restore, MoveReason::Restore, &mut events);
        self.close_episode(EpisodeOutcome::RolledBack, &mut events);
        if self.state.phase == Phase::Search {
            self.state.phase = Phase::Idle;
        }
        events
    }
}

/// `ev`'s slot in per-EV arrays, in [`Ev::ALL`] order.
const fn slot(ev: Ev) -> usize {
    match ev {
        Ev::PassRate => 0,
        Ev::UsdPerVerifiedSuccess => 1,
        Ev::FalseGreen => 2,
        Ev::LatencyP90S => 3,
    }
}

/// The share of `window` for which `hit` holds; 0 for an empty window.
fn share(window: &[TaskResolution], hit: impl Fn(&TaskResolution) -> bool) -> f64 {
    let hits = window.iter().filter(|r| hit(r)).count();
    hits as f64 / window.len().max(1) as f64
}

/// The mean attempts per resolution of `resolutions`; 0 when there are none.
fn mean_attempts(resolutions: &[TaskResolution]) -> f64 {
    let attempts: u32 = resolutions.iter().map(|r| r.attempts).sum();
    f64::from(attempts) / resolutions.len().max(1) as f64
}

/// `estimate` read at the end of its 90% interval nearer the inner band:
/// the least shift its resolutions allow. Without an interval it has no
/// reading.
fn favourable(estimate: Estimate) -> Estimate {
    let value = estimate
        .interval
        .map(|(low, high)| match estimate.ev.side() {
            Side::Lower => high,
            Side::Upper => low,
        });
    Estimate { value, ..estimate }
}

/// The one of two readings of an EV farther past its bound's side: the
/// lower for E1, the higher for the others. A reading without a value
/// loses.
fn worse(a: Estimate, b: Estimate) -> Estimate {
    let badness = |estimate: &Estimate| {
        estimate.value.map(|value| match estimate.ev.side() {
            Side::Lower => -value,
            Side::Upper => value,
        })
    };
    if badness(&b) > badness(&a) { b } else { a }
}

/// One standard normal draw (Box–Muller).
fn gaussian(rng: &mut ChaCha8Rng) -> f64 {
    let u1: f64 = rng.gen_range(f64::MIN_POSITIVE..1.0);
    let u2: f64 = rng.gen_range(0.0..1.0);
    (-2.0 * u1.ln()).sqrt() * (std::f64::consts::TAU * u2).cos()
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use roko_core::config::{ProviderConfig, RokoConfig};
    use roko_core::task::TaskTier;

    use super::*;
    use crate::homeostasis::ev::{DrivePolicy, EvBounds};
    use crate::homeostasis::policy::{AuditPolicy, VerifyPolicy};
    use crate::homeostasis::resolution::CostSourceMix;
    use crate::telemetry::CostSource;

    /// In control: 80% pass, $0.05 and five minutes per resolution.
    const BASELINE: Baseline = Baseline {
        pass_rate: 0.80,
        usd_per_resolution: 0.05,
        wall_ms: 300_000.0,
    };

    /// S06 §5's example policy: E1 ≥ 0.70 (inner 0.75), E2 ≤ $0.12, E4 ≤
    /// 900 s (inner 810 s).
    fn policy() -> ViabilityPolicy {
        ViabilityPolicy {
            policy_version: 1,
            ev: EvBounds::s06_example(),
            drive: DrivePolicy::default(),
            tiers: BTreeMap::new(),
            verify: VerifyPolicy::default(),
            audit: AuditPolicy::default(),
            holdout: 0.10,
        }
    }

    /// A controller in `mode` over two providers, making guided moves only.
    fn controller(mode: HomeostasisMode) -> Controller {
        let mut config = RokoConfig::default();
        for name in ["alpha", "beta"] {
            config
                .providers
                .insert(name.to_string(), ProviderConfig::default());
        }
        let settings = HomeostasisConfig {
            mode,
            random_step_prob: 0.0,
            ..HomeostasisConfig::default()
        };
        Controller::new(
            &settings,
            policy(),
            HarnessParams::baseline(&config),
            HarnessLadders::from_config(&config),
            BASELINE,
            7,
        )
    }

    /// Resolution `index`, settled `index × spacing_s` seconds in: a pass or
    /// a failure that took `wall_s` seconds, at the baseline's cost.
    fn resolution(index: u64, passed: bool, wall_s: u64, spacing_s: u64) -> TaskResolution {
        let mut cost_source_mix = CostSourceMix::default();
        cost_source_mix.add(CostSource::ProviderUsage);
        TaskResolution {
            chain_key: format!("run:plan:t{index}"),
            final_verdict: if passed {
                AttemptOutcome::Passed
            } else {
                AttemptOutcome::GateFailed
            },
            attempts: 1,
            api_equiv_usd: Some(0.05),
            cost_source_mix,
            wall_ms: Some(wall_s * 1000),
            provider_errors: 0,
            conductor_restarts: 0,
            params_digest: None,
            arm: None,
            audited: false,
            audit_false_green: None,
            pre_instrumentation: false,
            resolved_at: i64::try_from(index * spacing_s * 1000).ok(),
        }
    }

    /// Twenty in-bounds resolutions (E1 = 0.8, failing at 1, 6, 11 and 16),
    /// then failures from 21: the detectors confirm E1's breach at 24, which
    /// opens an episode with its first move. The events of each resolution.
    fn breach(controller: &mut Controller, spacing_s: u64) -> Vec<Vec<ControllerEvent>> {
        (1..=24)
            .map(|index| {
                let passed = index <= 20 && index % 5 != 1;
                controller.on_resolution(&resolution(index, passed, 300, spacing_s))
            })
            .collect()
    }

    fn changes(events: &[ControllerEvent]) -> Vec<ChangeProposal> {
        events
            .iter()
            .filter_map(|event| match event {
                ControllerEvent::Change(change) => Some(change.as_ref().clone()),
                _ => None,
            })
            .collect()
    }

    fn evaluations(events: &[ControllerEvent]) -> Vec<Evaluation> {
        events
            .iter()
            .filter_map(|event| match event {
                ControllerEvent::Evaluate { decision, .. } => Some(*decision),
                _ => None,
            })
            .collect()
    }

    fn floor_up(tier: TaskTier) -> MoveKey {
        MoveKey {
            knob: Knob::TierFloor(tier),
            direction: Step::Up,
        }
    }

    /// The wall time of resolution `index`: 850 s for three in twenty, 300 s
    /// otherwise. The window's p90 is then 850 s, between E4's inner band
    /// (810 s) and its bound (900 s), while E4's detector, which reads each
    /// resolution's log-ratio to the 300 s baseline, sees lone slow tasks
    /// and never alarms.
    fn slow_tail(index: u64) -> u64 {
        if matches!(index % 20, 0 | 7 | 14) {
            850
        } else {
            300
        }
    }

    /// `events` hold E4's slow breach: outside its inner bound, 810 s, for
    /// `delay` resolutions.
    fn assert_slow_breach(events: &[ControllerEvent], delay: u32) {
        let found = events.iter().find_map(|event| match event {
            ControllerEvent::Breach {
                ev,
                bound,
                detect_delay_resolutions,
                ..
            } => Some((*ev, *bound, *detect_delay_resolutions)),
            _ => None,
        });
        assert_eq!(found, Some((Ev::LatencyP90S, 810.0, delay)));
    }

    #[test]
    fn no_move_within_dwell() {
        let mut on = controller(HomeostasisMode::On);
        let events = breach(&mut on, 60);
        assert!(events[..23].iter().all(|step| changes(step).is_empty()));
        // The breach directs B1 floor↑ first (S06 A1).
        let first = changes(&events[23]);
        assert_eq!(first.len(), 1, "{:?}", events[23]);
        assert_eq!(first[0].knob, Knob::TierFloor(TaskTier::Mechanical));
        assert_eq!(first[0].reason, MoveReason::Directed);
        assert_eq!(first[0].kind, ChangeKind::Search);
        assert!(first[0].applied && first[0].verdict.passed());
        assert_eq!(on.phase(), Phase::Search);

        // Failures go on. For seven resolutions nothing moves or is judged;
        // the eighth judges the change, rolls it back and moves again.
        for index in 25..=31 {
            let events = on.on_resolution(&resolution(index, false, 300, 60));
            assert!(changes(&events).is_empty(), "{index}: {events:?}");
            assert!(evaluations(&events).is_empty(), "{index}: {events:?}");
        }
        let events = on.on_resolution(&resolution(32, false, 300, 60));
        assert_eq!(evaluations(&events), [Evaluation::RolledBack]);
        let moved = changes(&events);
        assert_eq!(moved.len(), 2, "{events:?}");
        assert_eq!(moved[0].kind, ChangeKind::Rollback);
        assert_eq!(moved[1].knob, Knob::TierFloor(TaskTier::Focused));

        // Ten seconds apart, eight resolutions take 80 s: the change also
        // waits out its 120 s, until resolution 36.
        let mut quick = controller(HomeostasisMode::On);
        let events = breach(&mut quick, 10);
        assert_eq!(changes(&events[23]).len(), 1);
        for index in 25..=35 {
            let events = quick.on_resolution(&resolution(index, false, 300, 10));
            assert!(changes(&events).is_empty(), "{index}: {events:?}");
            assert!(evaluations(&events).is_empty(), "{index}: {events:?}");
        }
        let events = quick.on_resolution(&resolution(36, false, 300, 10));
        assert_eq!(evaluations(&events), [Evaluation::RolledBack]);
    }

    #[test]
    fn rollback_on_collateral() {
        let mut on = controller(HomeostasisMode::On);
        let theta0 = on.theta().clone();
        breach(&mut on, 60);
        assert_eq!(on.theta().tier_floor[&TaskTier::Mechanical], "mid");

        // After the move every task passes but takes 950 s: E1 recovers
        // while E4, which was in bounds when the move was made, breaches.
        let mut during = Vec::new();
        for index in 25..=31 {
            during.extend(on.on_resolution(&resolution(index, true, 950, 60)));
        }
        assert!(changes(&during).is_empty() && evaluations(&during).is_empty());
        assert!(during.iter().any(|event| matches!(
            event,
            ControllerEvent::Breach {
                ev: Ev::LatencyP90S,
                ..
            }
        )));
        assert!(during.iter().any(|event| matches!(
            event,
            ControllerEvent::Restore {
                ev: Ev::PassRate,
                ..
            }
        )));

        // The drive fell from about 2.3 (D_pre) to about 0.12, so only the
        // collateral breach rolls the move back.
        let events = on.on_resolution(&resolution(32, true, 950, 60));
        let judged = events.iter().find_map(|event| match event {
            ControllerEvent::Evaluate {
                decision, d_drive, ..
            } => Some((*decision, *d_drive)),
            _ => None,
        });
        let (decision, d_drive) = judged.expect("the change is judged");
        assert_eq!(decision, Evaluation::RolledBack);
        assert!(d_drive < -0.5, "{d_drive}");
        let moved = changes(&events);
        assert_eq!(moved[0].kind, ChangeKind::Rollback);
        assert_eq!(moved[0].theta, theta0);
        let episode = on.episode().expect("the episode stays open");
        assert_eq!(episode.tabu, [floor_up(TaskTier::Mechanical)]);

        // The next move goes after latency. It lowers cost, so it is
        // audit-coupled.
        assert_eq!(moved.len(), 2, "{events:?}");
        assert_eq!(moved[1].knob, Knob::TurnCapMult);
        assert_eq!(moved[1].to, Value::from(0.75));
        assert!(moved[1].audit_coupled);
    }

    #[test]
    fn hold_after_n_max() {
        let mut on = controller(HomeostasisMode::On);
        let theta0 = on.theta().clone();
        let mut events: Vec<ControllerEvent> = breach(&mut on, 60).into_iter().flatten().collect();
        for index in 25..=80 {
            events.extend(on.on_resolution(&resolution(index, false, 300, 60)));
        }

        // Nothing helps: six distinct moves, each rolled back after its
        // dwell, in the order of the catalog-sign prior.
        let searched: Vec<Knob> = changes(&events)
            .iter()
            .filter(|change| change.kind == ChangeKind::Search)
            .map(|change| change.knob)
            .collect();
        assert_eq!(
            searched,
            [
                Knob::TierFloor(TaskTier::Mechanical),
                Knob::TierFloor(TaskTier::Focused),
                Knob::TierFloor(TaskTier::Integrative),
                Knob::RetryDelta,
                Knob::TurnCapMult,
                Knob::ExtraRungs
            ]
        );
        assert_eq!(evaluations(&events), [Evaluation::RolledBack; 6]);

        // After the sixth, N_max: HOLD, θ at the last-known-good (θ₀).
        let holds: Vec<HoldReason> = events
            .iter()
            .filter_map(|event| match event {
                ControllerEvent::Hold { reason, .. } => Some(*reason),
                _ => None,
            })
            .collect();
        assert_eq!(holds, [HoldReason::MaxChanges]);
        assert!(events.iter().any(|event| matches!(
            event,
            ControllerEvent::EpisodeClose {
                outcome: EpisodeOutcome::Hold,
                changes: 6,
                ..
            }
        )));
        assert_eq!(on.phase(), Phase::Hold);
        assert_eq!(on.theta(), &theta0);
        assert!(on.episode().is_none());

        // While held nothing moves; an acknowledgement releases the HOLD.
        for index in 81..=100 {
            let events = on.on_resolution(&resolution(index, false, 300, 60));
            assert!(changes(&events).is_empty(), "{index}: {events:?}");
        }
        assert_eq!(
            on.ack(),
            [ControllerEvent::Release {
                reason: ReleaseReason::Ack
            }]
        );
        assert_eq!(on.phase(), Phase::Idle);
    }

    #[test]
    fn shadow_proposals_are_unevaluable() {
        let mut shadow = controller(HomeostasisMode::Shadow);
        let events: Vec<ControllerEvent> = breach(&mut shadow, 60).into_iter().flatten().collect();
        let first = changes(&events);
        assert_eq!(first.len(), 1);
        assert!(!first[0].applied && first[0].verdict.passed());

        // The dwell ends unjudged: the would-be θ drops the change without a
        // rollback row, and the next proposal is another move.
        let mut later = Vec::new();
        for index in 25..=32 {
            later.extend(shadow.on_resolution(&resolution(index, false, 300, 60)));
        }
        assert_eq!(evaluations(&later), [Evaluation::UnevaluableInShadow]);
        let moved = changes(&later);
        assert_eq!(moved.len(), 1, "{later:?}");
        assert_eq!(moved[0].knob, Knob::TierFloor(TaskTier::Focused));
        assert_eq!(moved[0].from, Value::from("cheap"));
        assert!(!moved[0].applied);
        assert_eq!(shadow.theta().tier_floor[&TaskTier::Mechanical], "cheap");
        let episode = shadow.episode().expect("the episode stays open");
        assert!(episode.tabu.contains(&floor_up(TaskTier::Mechanical)));
    }

    #[test]
    fn ev_between_inner_and_outer_bands_eventually_acts() {
        // IDLE: every task passes, and from resolution 20, the first full
        // window, E4 sits between its bands with D above 0. Nothing
        // breaches, so nothing happens until E4 has been outside its inner
        // band for relax_window (40) resolutions: at 59 it latches as a slow
        // breach and opens an episode, which moves.
        let mut idle = controller(HomeostasisMode::On);
        for index in 1..=58 {
            let events = idle.on_resolution(&resolution(index, true, slow_tail(index), 60));
            assert!(events.is_empty(), "{index}: {events:?}");
        }
        let events = idle.on_resolution(&resolution(59, true, slow_tail(59), 60));
        assert_slow_breach(&events, 40);
        let opened = events.iter().find_map(|event| match event {
            ControllerEvent::EpisodeOpen { evs, .. } => Some(evs.clone()),
            _ => None,
        });
        assert_eq!(opened, Some(vec![Ev::LatencyP90S]));
        let first = changes(&events);
        assert_eq!(first.len(), 1, "{events:?}");
        assert_eq!(first[0].kind, ChangeKind::Search);
        assert_eq!(first[0].knob, Knob::TurnCapMult);
        assert_eq!(idle.phase(), Phase::Search);

        // SEARCH: E1's breach opens an episode at 24, and its first move is
        // kept at 32 once E1 is back in its inner band. E4 has sat between
        // its bands since 20: nothing is breached and D stays above 0, so
        // the episode waits, dwell after dwell, until the decision at 64
        // finds E4 forty resolutions outside its band and moves on it.
        let mut search = controller(HomeostasisMode::On);
        for index in 1..=24 {
            let passed = index <= 20 && index % 5 != 1;
            search.on_resolution(&resolution(index, passed, slow_tail(index), 60));
        }
        assert_eq!(search.phase(), Phase::Search);
        let mut waiting = Vec::new();
        for index in 25..=63 {
            waiting.extend(search.on_resolution(&resolution(index, true, slow_tail(index), 60)));
        }
        assert_eq!(evaluations(&waiting), [Evaluation::Kept]);
        assert!(changes(&waiting).is_empty(), "{waiting:?}");
        let events = search.on_resolution(&resolution(64, true, slow_tail(64), 60));
        assert_slow_breach(&events, 45);
        let next = changes(&events);
        assert_eq!(next.len(), 1, "{events:?}");
        assert_eq!(next[0].kind, ChangeKind::Search);
        assert_eq!(next[0].knob, Knob::TurnCapMult);
    }

    #[test]
    fn onset_inside_the_window_keeps_a_good_first_move() {
        // The breach's onset, resolution 21, lies inside the window of the
        // first move, 5 to 24, whose D of 1.0 averages in sixteen
        // resolutions from before it. Read over 23 and 24, both failed,
        // since E1 left its inner band, at the top of their 90% interval,
        // E1 is about 0.575 and D_pre about 2.3.
        let mut on = controller(HomeostasisMode::On);
        breach(&mut on, 60);

        // The move helps: five of the eight tasks in its dwell pass, and D
        // over the dwell is about 1.4. Against the window's 1.0 the move
        // would be rolled back and made tabu; against D_pre it is kept.
        let mut dwell = Vec::new();
        for index in 25..=32 {
            let passed = !matches!(index, 27 | 29 | 32);
            dwell.extend(on.on_resolution(&resolution(index, passed, 300, 60)));
        }
        let judged = dwell.iter().find_map(|event| match event {
            ControllerEvent::Evaluate {
                decision, d_drive, ..
            } => Some((*decision, *d_drive)),
            _ => None,
        });
        let (decision, d_drive) = judged.expect("the change is judged");
        assert_eq!(decision, Evaluation::Kept);
        assert!((-1.0..-0.8).contains(&d_drive), "{d_drive}");
        let episode = on.episode().expect("the episode stays open");
        assert!(episode.tabu.is_empty(), "{:?}", episode.tabu);
        // E1 is still breached, so the search makes its next move.
        let kinds: Vec<ChangeKind> = changes(&dwell).iter().map(|change| change.kind).collect();
        assert_eq!(kinds, [ChangeKind::Search]);
    }
}
