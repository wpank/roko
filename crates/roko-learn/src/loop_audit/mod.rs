//! The M2 loop-liveness audit (S03). Every learning loop registers a
//! contract, and the auditor measures whether the loop's learned state
//! reaches the executed decision (exposure ε), changes it (net influence ι)
//! and pays for itself (benefit β) against a randomized default policy π⁰.
//!
//! - [`spec`]: the contract ([`LoopSpec`]), the closed reason codes and the
//!   registry: the embedded `loops.toml`, merged by loop id with a
//!   workspace's `.roko/learn/loop-registry.toml`.
//! - [`assign`]: loop layers, holdout schedules, nesting, the all-off arm
//!   and composed propensities over S01's one assignment function.
//! - [`arm_set`]: S02.P1-14's per-chain arm set, drawn once over those
//!   layers at attempt open and inherited by the chain's retries.
//!
//! The other modules hold one later piece of S03 each, so that they can be
//! built in parallel: [`exposure`] (ε, ι), [`estimators`] (β), [`cs`]
//! (confidence sequences), [`state`] (the state machine), [`ledger`] (the
//! A-LOOP rows), [`census`], [`canary`], [`faults`] and [`sim`].
//!
//! Per-run records (decisions, faults) stay in the run directory; the
//! cross-run loop-audit rows go in `.roko/learn` (decision 2201).
//!
//! [`LoopAuditor`] is the facade (S03 §4.1): the registry, each loop's
//! latest audited state and `[learning.audit]`. At attempt open, S02.P1-14's
//! `ArmSet` asks it for a loop's layer and the policy the loop executes. At
//! each plan run's close, [`LoopAuditor::observe_run`] is the audit tick
//! (backlog 5126): it evaluates every measured loop and appends the
//! `loop.health` and `loop.transition` rows. A tripped audit stays tripped
//! until a person clears it ([`LoopAuditor::clear_trip`]).

pub mod arm_set;
pub mod assign;
pub mod canary;
pub mod census;
pub mod cs;
pub mod estimators;
pub mod exposure;
pub mod faults;
pub mod ledger;
pub mod sim;
pub mod spec;
pub mod state;

pub use spec::{
    AuditState, Layer, Lifecycle, LoopId, LoopSpec, Qualifier, ReasonCode, ReceiptKind, Registry,
    RegistryError, StaticFinding,
};

use std::collections::BTreeMap;
use std::path::Path;

use chrono::{DateTime, SecondsFormat, Utc};
use roko_core::config::learning::{AuditEpoch, LearningAuditConfig};

use self::arm_set::{ArmDraws, MAXIMIZE_CONDITION, NORMAL_CONDITION, PLACEBO_LAYER};
use self::assign::{EXPLORE_EPSILON, HoldoutSchedule, LoopLayer};
use self::census::{CensusState, LoopMeasurement, MeasuredLoop};
use self::exposure::InfluenceEstimate;
use self::ledger::{
    AuditClearedRow, BetaFields, HealthRow, IotaFields, LOOP_AUDIT_SCHEMA, Ledger, LoopAuditRecord,
    LoopAuditRow, TransitionRow,
};
use self::state::{
    Actor, AuditParams, Auditor, AuditorSignals, ExecutedPolicy, LoopEvidence, LoopStatus,
    Structural,
};
use crate::telemetry::LayerSpec;
use crate::telemetry::RunProvenanceManifest;
use crate::telemetry::records::b3_digest;
use crate::telemetry::report::RunRecords;

/// A `loop.health` row's kind.
const HEALTH_KIND: &str = "loop.health";
/// A `loop.transition` row's kind.
const TRANSITION_KIND: &str = "loop.transition";
/// A `loop.audit_cleared` row's kind.
const CLEARED_KIND: &str = "loop.audit_cleared";
/// The `loop_id` of a row about the whole audit rather than one loop.
pub const ALL_LOOPS: &str = "*";
/// Where an audit tick's health rows come from: the runs' decision rows.
const MEASURED_EVIDENCE: &str = "measured";

/// The loop auditor's facade (S03 §4.1): the registry, each loop's latest
/// audited state from the loop-audit ledger, and `[learning.audit]`.
///
/// At attempt open, S02.P1-14's `ArmSet` asks it for a loop's layer
/// ([`Self::layer_spec`], or every layer at once with [`Self::arm_draws`])
/// and for the policy the loop executes ([`Self::executed_policy`]).
#[derive(Debug, Clone)]
pub struct LoopAuditor {
    registry: Registry,
    config: LearningAuditConfig,
    standings: BTreeMap<LoopId, Standing>,
    audit_broken: bool,
    maximize: bool,
}

/// A loop's latest audited state and its reason, and the dwell its last move
/// started.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Standing {
    state: AuditState,
    reason: Option<ReasonCode>,
    /// When it last moved (unix seconds): its latest transition row's `ts`.
    moved_at: Option<i64>,
    /// Its opportunities then: the `n_opp` of its latest health row before
    /// that transition row, which an audit tick writes first.
    moved_opps: u64,
    /// Its latest health row's `n_opp`.
    opps: u64,
}

impl Standing {
    /// A loop without a row: on probation, and never moved.
    const NEW: Self = Self {
        state: AuditState::Probation,
        reason: None,
        moved_at: None,
        moved_opps: 0,
        opps: 0,
    };
}

/// The common fields of an audit tick's rows (S01 §5.10).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RowOrigin {
    /// The run whose close the tick follows; `None` for a scheduled pass,
    /// whose rows say `run_id: null`.
    pub run_id: Option<String>,
    /// The harness commit.
    pub harness_sha: Option<String>,
    /// The config fingerprint.
    pub config_hash: Option<String>,
}

impl RowOrigin {
    /// The origin of the tick after run `run_id` under `runs_dir`: the run,
    /// with the harness commit and the config hash its manifest records.
    #[must_use]
    pub fn of_run(runs_dir: &Path, run_id: &str) -> Self {
        let manifest = RunProvenanceManifest::load(&runs_dir.join(run_id))
            .ok()
            .flatten();
        let recorded = |value: &str| (!value.is_empty()).then(|| value.to_string());
        Self {
            run_id: Some(run_id.to_string()),
            harness_sha: manifest
                .as_ref()
                .and_then(|manifest| recorded(&manifest.harness.sha)),
            config_hash: manifest
                .as_ref()
                .and_then(|manifest| recorded(&manifest.config.hash)),
        }
    }
}

/// One loop's evaluation at an audit tick (S03 §5; backlog 5126).
#[derive(Debug, Clone, PartialEq)]
pub struct LoopObservation {
    /// Its `loop.health` row.
    pub health: LoopAuditRecord,
    /// The qualifiers its evidence carries, which the row has no field for.
    pub qualifiers: Vec<Qualifier>,
    /// Its `loop.transition` row, when it moved.
    pub transition: Option<LoopAuditRecord>,
}

impl LoopObservation {
    /// Its rows in ledger order: the health row, then the transition, whose
    /// opportunities a reader takes from the health row.
    pub fn records(&self) -> impl Iterator<Item = &LoopAuditRecord> {
        std::iter::once(&self.health).chain(self.transition.as_ref())
    }
}

/// What the rows of one audit tick share: their origin, epoch and time.
struct Stamp<'a> {
    origin: &'a RowOrigin,
    epoch: String,
    ts: String,
}

impl Stamp<'_> {
    /// A `kind` row of `loop_id`, its id the digest of its kind, loop, run
    /// and time.
    fn record(&self, kind: &str, loop_id: &str, row: LoopAuditRow) -> LoopAuditRecord {
        let run_id = self.origin.run_id.as_deref().unwrap_or_default();
        let parts = [LOOP_AUDIT_SCHEMA, kind, loop_id, run_id, self.ts.as_str()];
        LoopAuditRecord {
            schema_version: LOOP_AUDIT_SCHEMA.to_string(),
            record_id: Some(b3_digest(parts.join("|").as_bytes())),
            ts: Some(self.ts.clone()),
            loop_id: loop_id.to_string(),
            harness_sha: self.origin.harness_sha.clone(),
            config_hash: self.origin.config_hash.clone(),
            audit_epoch: Some(self.epoch.clone()),
            run_id: Some(self.origin.run_id.clone()),
            row,
        }
    }
}

impl LoopAuditor {
    /// The auditor of the workspace `workdir`: its registry
    /// ([`Registry::load`]) and each loop's state from its latest health or
    /// transition row in `.roko/learn/loop-audit.jsonl`.
    ///
    /// # Errors
    ///
    /// The registry's error. An unreadable ledger leaves every loop on
    /// probation.
    pub fn load(workdir: &Path, config: &LearningAuditConfig) -> Result<Self, RegistryError> {
        let registry = Registry::load(workdir)?;
        let learn_dir = roko_fs::RokoLayout::for_project(workdir).learn_dir();
        let records = Ledger::in_learn_dir(&learn_dir)
            .read()
            .unwrap_or_else(|error| {
                tracing::warn!(%error, "loop-audit ledger unreadable; every loop is on probation");
                Vec::new()
            });
        Ok(Self::from_records(registry, config, &records))
    }

    /// The auditor over `registry` and the ledger rows `records`, in order:
    /// a loop takes the state of its latest health or transition row, and is
    /// on probation without one. Its latest transition row starts its dwell:
    /// the row's time, and the opportunities of the health row before it.
    /// The auditor is broken while any loop's latest health row since the
    /// last `loop.audit_cleared` row has `placebo_ok` false.
    #[must_use]
    pub fn from_records(
        registry: Registry,
        config: &LearningAuditConfig,
        records: &[LoopAuditRecord],
    ) -> Self {
        let mut standings: BTreeMap<LoopId, Standing> = BTreeMap::new();
        for record in records {
            let loop_id = record.loop_id.as_str();
            match &record.row {
                LoopAuditRow::Health(health) => {
                    let standing = standings
                        .entry(LoopId::new(loop_id))
                        .or_insert(Standing::NEW);
                    standing.state = health.state;
                    standing.reason = health.reason;
                    standing.opps = health.n_opp;
                }
                LoopAuditRow::Transition(transition) => {
                    let standing = standings
                        .entry(LoopId::new(loop_id))
                        .or_insert(Standing::NEW);
                    standing.state = transition.to;
                    standing.reason = transition.reason;
                    let moved_at = record.ts.as_deref().and_then(unix_seconds);
                    standing.moved_at = moved_at.or(standing.moved_at);
                    standing.moved_opps = standing.opps;
                }
                _ => {}
            }
        }
        Self {
            registry,
            config: config.clone(),
            standings,
            audit_broken: !tripped_loops(records).is_empty(),
            maximize: false,
        }
    }

    /// Clear the tripped audit of the workspace `workdir` (S03 §4.6's
    /// `audit_broken`), which only a person may do: append a
    /// `loop.audit_cleared` row at `now` naming `by`, `reason` and the loops
    /// whose latest health row tripped it. The next audit tick audits
    /// afresh, and trips it again while the cause remains. `None`, with no
    /// row, when the audit is not tripped.
    ///
    /// # Errors
    ///
    /// An empty `by` or `reason`, or the ledger's read or write error.
    pub fn clear_trip(
        workdir: &Path,
        by: &str,
        reason: &str,
        now: DateTime<Utc>,
    ) -> std::io::Result<Option<LoopAuditRecord>> {
        if by.trim().is_empty() || reason.trim().is_empty() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "clearing the loop audit needs who clears it and why",
            ));
        }
        let learn_dir = roko_fs::RokoLayout::for_project(workdir).learn_dir();
        let ledger = Ledger::in_learn_dir(&learn_dir);
        let tripped = tripped_loops(&ledger.read()?);
        if tripped.is_empty() {
            return Ok(None);
        }
        let origin = RowOrigin::default();
        let stamp = Stamp {
            origin: &origin,
            epoch: now.format("%Y-%m-%d").to_string(),
            ts: now.to_rfc3339_opts(SecondsFormat::Secs, true),
        };
        let row = AuditClearedRow {
            by: by.trim().to_string(),
            reason: reason.trim().to_string(),
            tripped,
            actor: Actor::Human,
        };
        let record = stamp.record(CLEARED_KIND, ALL_LOOPS, LoopAuditRow::AuditCleared(row));
        ledger.append(&record)?;
        Ok(Some(record))
    }

    /// This auditor in maximize mode or not (`--no-holdout`, D7). Maximize
    /// mode withholds nothing (g = h = ε = 0), enforces nothing, and its
    /// chains record [`MAXIMIZE_CONDITION`].
    #[must_use]
    pub fn maximize(mut self, on: bool) -> Self {
        self.maximize = on;
        self
    }

    /// The registry it audits.
    #[must_use]
    pub const fn registry(&self) -> &Registry {
        &self.registry
    }

    /// Its `[learning.audit]` section.
    #[must_use]
    pub const fn config(&self) -> &LearningAuditConfig {
        &self.config
    }

    /// Whether the placebo moved, which freezes every transition and every
    /// enforcement.
    #[must_use]
    pub const fn audit_broken(&self) -> bool {
        self.audit_broken
    }

    /// The condition a chain records: [`MAXIMIZE_CONDITION`] in maximize
    /// mode, else [`NORMAL_CONDITION`].
    #[must_use]
    pub const fn condition_id(&self) -> &'static str {
        if self.maximize {
            MAXIMIZE_CONDITION
        } else {
            NORMAL_CONDITION
        }
    }

    /// `loop_id`'s audit state: its latest row's, else probation.
    #[must_use]
    pub fn state(&self, loop_id: &str) -> AuditState {
        self.standing(loop_id).state
    }

    /// `loop_id`'s audit reason: its latest row's, if any.
    #[must_use]
    pub fn reason(&self, loop_id: &str) -> Option<ReasonCode> {
        self.standing(loop_id).reason
    }

    /// `loop_id`'s latest standing, else probation with no reason.
    fn standing(&self, loop_id: &str) -> Standing {
        self.standings
            .get(&LoopId::new(loop_id))
            .copied()
            .unwrap_or(Standing::NEW)
    }

    /// The holdout schedule `[learning.audit] h` sets, with maximize mode.
    fn layers(&self) -> LoopLayer {
        let h = self.config.h;
        LoopLayer {
            schedule: HoldoutSchedule {
                probation: h.probation,
                live: h.live,
                min: h.min,
                suspect: h.suspect,
            },
            campaign_h: None,
            maximize: self.maximize,
        }
    }

    /// g, the all-learning-off rate: `global_off`, or 0 in maximize mode.
    #[must_use]
    pub fn global_off(&self) -> f64 {
        if self.maximize {
            0.0
        } else {
            self.config.global_off
        }
    }

    /// ε of the route's per-attempt explore draw: 0 in maximize mode.
    #[must_use]
    pub const fn explore_epsilon(&self) -> f64 {
        if self.maximize { 0.0 } else { EXPLORE_EPSILON }
    }

    /// `loop_id`'s layer for `epoch` and `run_seed` (S03 §4.3): h from its
    /// audit state's schedule, while L-M1 and the placebo keep their fixed
    /// rates, and g. Maximize mode sets both to 0. `None` for a loop the
    /// registry lacks.
    #[must_use]
    pub fn layer_spec(&self, loop_id: &str, epoch: &str, run_seed: u64) -> Option<LayerSpec> {
        let spec = self.registry.get(loop_id)?;
        let state = self.state(loop_id);
        let g = self.global_off();
        Some(self.layers().layer_spec(spec, state, epoch, run_seed, g))
    }

    /// What `ArmSet` draws a chain's arms from for `epoch` and `run_seed`:
    /// g, every loop's state, the loops whose enforced demotion runs π⁰
    /// ([`Self::executed_policy`]) and the schedule.
    #[must_use]
    pub fn arm_draws(&self, run_seed: u64, epoch: impl Into<String>) -> ArmDraws {
        ArmDraws {
            run_seed,
            epoch: epoch.into(),
            g: self.global_off(),
            states: self
                .standings
                .iter()
                .map(|(id, standing)| (id.clone(), standing.state))
                .collect(),
            enforced: self
                .registry
                .loops()
                .iter()
                .filter(|spec| self.executed_policy(spec.id.as_str()) == ExecutedPolicy::Default)
                .map(|spec| spec.id.clone())
                .collect(),
            layers: self.layers(),
        }
    }

    /// The policy `loop_id` executes (S03 §4.6). It is π⁰ only for an
    /// enforced demotion (demoted, `inert` or `null`), which needs
    /// `[learning.audit] enforce` and the loop's own `enforce`, no exemption
    /// (the registry's or the section's) and an auditor that is not broken.
    /// Otherwise, and in maximize mode, it is the learned policy.
    #[must_use]
    pub fn executed_policy(&self, loop_id: &str) -> ExecutedPolicy {
        let Some(spec) = self.registry.get(loop_id) else {
            return ExecutedPolicy::Learned;
        };
        let standing = self.standing(loop_id);
        let enforce = self.config.enforce && spec.enforce && !self.audit_broken;
        let status = LoopStatus {
            enforce: enforce && !self.maximize,
            exempt: spec.exempt || self.config.exempts(loop_id),
            ..LoopStatus::registered(false, false, false)
        };
        state::executed_policy(&status, standing.state, standing.reason)
    }

    /// The assignment epoch `[learning.audit] epoch` names for a run: the
    /// UTC day of `now`, or the run id, as a bench campaign uses.
    #[must_use]
    pub fn epoch(&self, run_id: &str, now: DateTime<Utc>) -> String {
        match self.config.epoch {
            AuditEpoch::UtcDay => now.format("%Y-%m-%d").to_string(),
            AuditEpoch::Run => run_id.to_string(),
        }
    }

    /// The state machine's parameters (S03 §4.10) from the section, with α
    /// split over the randomized loops.
    #[must_use]
    pub fn params(&self) -> AuditParams {
        let config = &self.config;
        let randomized = self
            .registry
            .loops()
            .iter()
            .filter(|spec| spec.lifecycle.is_randomized())
            .count();
        AuditParams {
            alpha: config.alpha,
            enforced_loops: randomized.max(1),
            n_eps: config.n_eps,
            eps_min: config.eps_min,
            n_iota: config.n_iota,
            iota_min: config.iota_min,
            n_beta: config.n_beta,
            n_null: config.n_null,
            delta_min: config.delta_min,
            delta_ni: config.delta_ni,
            dwell_secs: i64::from(config.dwell.hours) * 3_600,
            dwell_opportunities: config.dwell.opps,
            reprobation_secs: i64::from(config.reprobation_days) * 86_400,
            spibb_n: config.spibb_n,
            h_probation: config.h.probation,
            h_live: config.h.live,
            h_suspect: config.h.suspect,
            ..AuditParams::default()
        }
    }

    /// The audit tick after run `run_id` of the workspace `workdir` closed
    /// (S03 §5; backlog 5126): fold into the census the tick keeps
    /// ([`CensusState`], in `.roko/learn`) the rows under `.roko/runs` no
    /// tick has folded, [`Self::observe_census`] at `now`, and append the
    /// rows to the loop-audit ledger in order. It reads the run that closed
    /// and the runs not read to their end, never the whole history
    /// (gap-addf2a), and writes nothing but the census and the ledger.
    ///
    /// # Errors
    ///
    /// The census's or the ledger's write error. The census is kept before
    /// any row is appended; the rows before the one that failed are
    /// appended.
    pub fn observe_run(
        &mut self,
        workdir: &Path,
        run_id: &str,
        now: DateTime<Utc>,
    ) -> std::io::Result<Vec<LoopObservation>> {
        let layout = roko_fs::RokoLayout::for_project(workdir);
        let runs_dir = layout.runs_dir();
        let origin = RowOrigin::of_run(&runs_dir, run_id);
        let learn_dir = layout.learn_dir();
        let kept = learn_dir.join(census::CENSUS_STATE_FILE);
        let mut census = CensusState::load(&kept, self.params().loop_alpha());
        census.fold_pending(&runs_dir, run_id, now);
        census.save(&kept)?;
        let observations = self.observe_census(&census, &origin, now);
        let ledger = Ledger::in_learn_dir(&learn_dir);
        for record in observations.iter().flat_map(LoopObservation::records) {
            ledger.append(record)?;
        }
        Ok(observations)
    }

    /// Evaluate each loop the decision rows of `runs` measure, at `now`
    /// (S03 §4.6), and keep the result as its standing: one fold of every
    /// row of `runs` into a fresh census, then [`Self::observe_census`].
    pub fn observe(
        &mut self,
        runs: &[RunRecords],
        origin: &RowOrigin,
        now: DateTime<Utc>,
    ) -> Vec<LoopObservation> {
        let mut census = CensusState::new(self.params().loop_alpha());
        for run in runs {
            census.fold(run, u64::MAX);
        }
        self.observe_census(&census, origin, now)
    }

    /// Evaluate each loop `census` measures, at `now` (S03 §4.6), and keep
    /// the result as its standing.
    ///
    /// Every registered loop that is not retired and has decision rows with
    /// S03's fields gets a `loop.health` row: its opportunities, ε and ι_net
    /// over every row folded, at α/K ([`CensusState::measurements`]), its
    /// layer's SRM e-value, and the state the state machine leaves it in,
    /// with the dwell its ledger rows carry across runs. A loop that moves
    /// gets a `loop.transition` row too. β is not estimated yet, so no loop
    /// goes live, `null` or `harm` here. An SRM alarm, or an auditor the
    /// ledger says is broken, freezes every transition, and the health rows
    /// say `placebo_ok: false`. Nothing here changes what a loop executes.
    pub fn observe_census(
        &mut self,
        census: &CensusState,
        origin: &RowOrigin,
        now: DateTime<Utc>,
    ) -> Vec<LoopObservation> {
        let params = self.params();
        let measured = census.measurements(&params);
        let signals = AuditorSignals {
            srm_alarm: census.srm_alarm(),
            ..AuditorSignals::default()
        };
        let mut auditor = Auditor::new(params);
        if self.audit_broken {
            auditor = auditor.tripped("the loop-audit ledger says the audit is broken");
        }
        let stamp = Stamp {
            origin,
            epoch: self.epoch(origin.run_id.as_deref().unwrap_or_default(), now),
            ts: now.to_rfc3339_opts(SecondsFormat::Secs, true),
        };
        let mut observations = Vec::new();
        let mut standings = Vec::new();
        for spec in self.registry.loops() {
            let id = spec.id.as_str();
            let measurement = measured.get(id).filter(|measurement| measurement.rows > 0);
            let retired = matches!(spec.lifecycle, Lifecycle::Retired { .. });
            let Some(measurement) = measurement.filter(|_| !retired) else {
                continue;
            };
            let standing = self.standing(id);
            let evidence = loop_evidence(measurement, &params, now.timestamp());
            let evaluation = auditor.evaluate(&self.status(spec, standing), &evidence, &signals);
            let n_opp = measurement.measured.n_opp;
            let health = HealthRow {
                state: evaluation.state,
                reason: evaluation.reason,
                h: self.layers().h(spec, evaluation.state),
                n_opp,
                n_learned: measurement.measured.n_learned,
                n_default: measurement.measured.n_default,
                eps: measurement.measured.eps,
                iota: iota_fields(&measurement.influence),
                beta: beta_fields(&measurement.measured, &params),
                srm_evalue: census.srm_evalue(spec.assignment_layer().as_str()),
                placebo_ok: !evaluation.audit_broken,
                evidence: MEASURED_EVIDENCE.to_string(),
            };
            let transition = evaluation.transition.as_ref().map(|moved| {
                let row = TransitionRow {
                    from: moved.from,
                    to: moved.to,
                    reason: moved.reason,
                    rule: moved.rule.clone(),
                    evidence: moved.evidence.clone(),
                    actor: moved.actor,
                    repair: None,
                };
                stamp.record(TRANSITION_KIND, id, LoopAuditRow::Transition(row))
            });
            let mut next = Standing {
                state: evaluation.state,
                reason: evaluation.reason,
                opps: n_opp,
                ..standing
            };
            if let Some(moved) = &evaluation.transition {
                next.moved_at = Some(moved.at);
                next.moved_opps = n_opp;
            }
            standings.push((spec.id.clone(), next));
            observations.push(LoopObservation {
                health: stamp.record(HEALTH_KIND, id, LoopAuditRow::Health(health)),
                qualifiers: evaluation.qualifiers,
                transition,
            });
        }
        self.standings.extend(standings);
        self.audit_broken = auditor.broken().is_some();
        observations
    }

    /// `spec`'s standing as the state machine reads it: its state and the
    /// dwell its last move started, with the switches it is audited under.
    fn status(&self, spec: &LoopSpec, standing: Standing) -> LoopStatus {
        LoopStatus {
            state: standing.state,
            reason: standing.reason,
            last_transition_at: standing.moved_at,
            opportunities_at_transition: standing.moved_opps,
            enforce: self.config.enforce && spec.enforce && !self.maximize,
            exempt: spec.exempt || self.config.exempts(spec.id.as_str()),
            placebo: spec.layer.as_str() == PLACEBO_LAYER,
        }
    }
}

/// The loops whose latest health row since the last `loop.audit_cleared`
/// row has `placebo_ok` false: the audit is tripped while there is one.
fn tripped_loops(records: &[LoopAuditRecord]) -> Vec<String> {
    let mut placebo_ok = BTreeMap::new();
    for record in records {
        match &record.row {
            LoopAuditRow::Health(health) => {
                placebo_ok.insert(record.loop_id.as_str(), health.placebo_ok);
            }
            LoopAuditRow::AuditCleared(_) => placebo_ok.clear(),
            _ => {}
        }
    }
    placebo_ok
        .into_iter()
        .filter(|(_, ok)| !ok)
        .map(|(loop_id, _)| loop_id.to_string())
        .collect()
}

/// The unix seconds of the RFC 3339 time `ts`, if it parses.
fn unix_seconds(ts: &str) -> Option<i64> {
    DateTime::parse_from_rfc3339(ts)
        .ok()
        .map(|ts| ts.timestamp())
}

/// What the state machine knows of a loop from its `measurement` at `now`
/// (unix seconds): the structural pre-checks its rows show (an arm assigned
/// at or after its decision; N_ε rows and not one opportunity), ε and ι_net.
/// Rows from before S03's fields add the `pre_instrumentation` qualifier.
fn loop_evidence(measurement: &LoopMeasurement, params: &AuditParams, now: i64) -> LoopEvidence {
    let n_opp = measurement.measured.n_opp;
    let mut qualifiers = Vec::new();
    if measurement.measured.pre_instrumentation > 0 {
        qualifiers.push(Qualifier::PreInstrumentation);
    }
    LoopEvidence {
        now,
        opportunities: n_opp,
        structural: Structural {
            ordering_violated: measurement.ordering_violations > 0,
            no_opportunity: n_opp == 0 && measurement.rows >= params.n_eps,
            ..Structural::default()
        },
        exposure: Some(measurement.exposure.clone()),
        influence: Some(measurement.influence.clone()),
        qualifiers,
        ..LoopEvidence::default()
    }
}

/// A health row's ι fields: ι, its A/A floor, ι_net and the lower end of
/// ι_net's sequence.
fn iota_fields(influence: &InfluenceEstimate) -> IotaFields {
    IotaFields {
        act: influence.iota,
        aa: influence.aa_floor,
        net: influence.iota_net,
        lcb: influence
            .interval
            .map_or(influence.iota_net, |(low, _)| low),
    }
}

/// A health row's β fields: no estimate yet, and why. Guard 2 comes first:
/// β waits for ε̂ ≥ ε_min, then for N_β opportunities.
fn beta_fields(measured: &MeasuredLoop, params: &AuditParams) -> BetaFields {
    let reason = if measured.eps.est < params.eps_min {
        "eps_below_min"
    } else if measured.n_opp < params.n_beta {
        "n_below_n_beta"
    } else {
        "not_estimated"
    };
    BetaFields {
        est: None,
        lcb: None,
        ucb: None,
        reason: Some(reason.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::ledger::{
        BetaFields, EpsilonFields, HealthRow, IotaFields, LOOP_AUDIT_SCHEMA, TransitionRow,
    };
    use super::state::Actor;
    use super::*;

    /// The epoch the tests draw in.
    const EPOCH: &str = "2026-10-03";

    /// A ledger row of `loop_id` with `row`.
    fn record(loop_id: &str, row: LoopAuditRow) -> LoopAuditRecord {
        LoopAuditRecord {
            schema_version: LOOP_AUDIT_SCHEMA.to_string(),
            record_id: None,
            ts: None,
            loop_id: loop_id.to_string(),
            harness_sha: None,
            config_hash: None,
            audit_epoch: None,
            run_id: None,
            row,
        }
    }

    /// A health row in `state` with `reason`, whose placebo is `placebo_ok`.
    fn health(
        loop_id: &str,
        state: AuditState,
        reason: Option<ReasonCode>,
        placebo_ok: bool,
    ) -> LoopAuditRecord {
        let row = HealthRow {
            state,
            reason,
            h: 0.2,
            n_opp: 0,
            n_learned: 0,
            n_default: 0,
            eps: EpsilonFields {
                est: 1.0,
                ucb: 1.0,
                read: 1.0,
                reach: 1.0,
                honest: 1.0,
                receipt: 1.0,
            },
            iota: IotaFields {
                act: 0.0,
                aa: 0.0,
                net: 0.0,
                lcb: 0.0,
            },
            beta: BetaFields {
                est: None,
                lcb: None,
                ucb: None,
                reason: None,
            },
            srm_evalue: 1.0,
            placebo_ok,
            evidence: "log".to_string(),
        };
        record(loop_id, LoopAuditRow::Health(row))
    }

    /// A transition of `loop_id` from probation to `to` with `reason`.
    fn transition(loop_id: &str, to: AuditState, reason: Option<ReasonCode>) -> LoopAuditRecord {
        let row = TransitionRow {
            from: AuditState::Probation,
            to,
            reason,
            rule: "test".to_string(),
            evidence: "test".to_string(),
            actor: Actor::Auditor,
            repair: None,
        };
        record(loop_id, LoopAuditRow::Transition(row))
    }

    /// S03 §4.1 and §4.3: the auditor serves each loop's holdout from its
    /// latest ledger state, keeps L-M1's fixed rate and leaves observe-only
    /// loops unrandomized. It executes π⁰ only for an enforced demotion, with
    /// both switches on, never for an exempt loop or under a broken auditor,
    /// and maximize mode withholds and enforces nothing.
    #[test]
    fn auditor_serves_schedule_and_honours_enforce() {
        let dir = tempfile::tempdir().expect("temp dir");
        let learn = dir.path().join(".roko/learn");
        std::fs::create_dir_all(&learn).expect("the learn dir");
        // This workspace's registry lets L-play and L-sec enforce.
        std::fs::write(
            dir.path().join(spec::REGISTRY_OVERRIDE_PATH),
            "[[loop]]\nid = \"L-play\"\nenforce = true\n\n\
             [[loop]]\nid = \"L-sec\"\nenforce = true\n",
        )
        .expect("write the registry override");
        let ledger = Ledger::in_learn_dir(&learn);
        for row in [
            health("L-know", AuditState::Live, None, true),
            health("L-play", AuditState::Probation, None, true),
            transition("L-play", AuditState::Demoted, Some(ReasonCode::Harm)),
            health("L-sec", AuditState::Flagged, Some(ReasonCode::Inert), true),
        ] {
            ledger.append(&row).expect("append a ledger row");
        }

        // The schedule: h by the latest state, L-M1's fixed rate, and no
        // holdout for an observe-only loop.
        let config = LearningAuditConfig::default();
        let auditor = LoopAuditor::load(dir.path(), &config).expect("load the auditor");
        assert!(!auditor.audit_broken());
        let h = |auditor: &LoopAuditor, loop_id: &str| {
            auditor
                .layer_spec(loop_id, EPOCH, 7)
                .expect("a registered loop")
                .h
        };
        assert_eq!(auditor.state("L-know"), AuditState::Live);
        assert_eq!(h(&auditor, "L-know"), 0.05);
        assert_eq!(h(&auditor, "L-play"), 0.50, "demoted runs at h_suspect");
        assert_eq!(h(&auditor, "L-route"), 0.20, "no row: probation");
        assert_eq!(h(&auditor, "L-M1"), 0.10, "L-M1 keeps its fixed holdout");
        assert_eq!(h(&auditor, "L-err"), 0.0, "observe-only: not randomized");
        let know = auditor.layer_spec("L-know", EPOCH, 7).expect("L-know");
        assert_eq!(know.layer, "knowledge");
        assert_eq!((know.g, know.run_seed), (0.03, 7));
        assert!(auditor.layer_spec("L-missing", EPOCH, 7).is_none());
        let draws = auditor.arm_draws(7, EPOCH);
        assert_eq!(
            draws.states.get(&LoopId::new("L-play")),
            Some(&AuditState::Demoted)
        );
        assert_eq!(auditor.params().alpha, 0.05);

        // enforce = false: every loop runs its learned policy.
        for loop_id in ["L-know", "L-play", "L-sec", "L-route"] {
            let policy = auditor.executed_policy(loop_id);
            assert_eq!(policy, ExecutedPolicy::Learned, "{loop_id}");
        }

        // Both switches on: the demotion and the inert flag run π⁰, and the
        // live L-know stays learned.
        let enforcing = LearningAuditConfig {
            enforce: true,
            ..config.clone()
        };
        let auditor = LoopAuditor::load(dir.path(), &enforcing).expect("load the auditor");
        assert_eq!(auditor.executed_policy("L-play"), ExecutedPolicy::Default);
        assert_eq!(auditor.executed_policy("L-sec"), ExecutedPolicy::Default);
        assert_eq!(auditor.executed_policy("L-know"), ExecutedPolicy::Learned);

        // An exempt loop is never demoted.
        let exempt = LearningAuditConfig {
            exempt: vec!["L-play".to_string()],
            ..enforcing.clone()
        };
        let auditor = LoopAuditor::load(dir.path(), &exempt).expect("load the auditor");
        assert_eq!(auditor.executed_policy("L-play"), ExecutedPolicy::Learned);
        assert_eq!(auditor.executed_policy("L-sec"), ExecutedPolicy::Default);

        // Maximize mode withholds nothing and enforces nothing.
        let maximize = LoopAuditor::load(dir.path(), &enforcing)
            .expect("load the auditor")
            .maximize(true);
        let layer = maximize.layer_spec("L-know", EPOCH, 7).expect("L-know");
        assert_eq!((layer.h, layer.g), (0.0, 0.0));
        assert_eq!(maximize.explore_epsilon(), 0.0);
        assert_eq!(maximize.condition_id(), "maximize");
        assert_eq!(maximize.executed_policy("L-play"), ExecutedPolicy::Learned);

        // A placebo that moved breaks the auditor, which freezes enforcement.
        let moved = health("L-placebo", AuditState::Probation, None, false);
        ledger.append(&moved).expect("append the placebo's row");
        let broken = LoopAuditor::load(dir.path(), &enforcing).expect("load the auditor");
        assert!(broken.audit_broken());
        assert_eq!(broken.executed_policy("L-play"), ExecutedPolicy::Learned);

        // The epoch is the UTC day, or the run id in a campaign.
        let now = Utc
            .with_ymd_and_hms(2026, 10, 3, 23, 59, 0)
            .single()
            .expect("a valid time");
        assert_eq!(auditor.epoch("gr-1", now), EPOCH);
        let campaign = LearningAuditConfig {
            epoch: AuditEpoch::Run,
            ..config
        };
        let auditor = LoopAuditor::load(dir.path(), &campaign).expect("load the auditor");
        assert_eq!(auditor.epoch("gr-1", now), "gr-1");
    }

    /// gap-1cf555: a tripped audit stays tripped until a person clears it.
    /// The clear is a `loop.audit_cleared` row naming who cleared it, why
    /// and the loops that had tripped it; the auditor then loads unbroken,
    /// and a later trip breaks it again. A clear needs a name and a reason,
    /// and an audit that is not tripped writes nothing.
    #[test]
    fn operator_clears_a_tripped_loop_audit() {
        let dir = tempfile::tempdir().expect("temp dir");
        let learn = dir.path().join(".roko/learn");
        std::fs::create_dir_all(&learn).expect("the learn dir");
        let ledger = Ledger::in_learn_dir(&learn);
        let config = LearningAuditConfig::default();
        let broken = || {
            LoopAuditor::load(dir.path(), &config)
                .expect("load the auditor")
                .audit_broken()
        };
        let now = Utc
            .with_ymd_and_hms(2026, 10, 4, 12, 0, 0)
            .single()
            .expect("a valid time");
        let clear = |by: &str, reason: &str| LoopAuditor::clear_trip(dir.path(), by, reason, now);

        let live = health("L-know", AuditState::Live, None, true);
        ledger.append(&live).expect("append a health row");
        let none = clear("will", "nothing tripped").expect("the ledger");
        assert!(none.is_none(), "{none:?}");

        // A moved placebo trips the audit, and each loop's next row says so.
        for loop_id in ["L-placebo", "L-know"] {
            let tripped = health(loop_id, AuditState::Probation, None, false);
            ledger.append(&tripped).expect("append a tripped row");
        }
        assert!(broken());
        assert!(clear("", "a reason").is_err(), "no one cleared it");
        assert!(clear("will", " ").is_err(), "and why");

        let why = "the SRM alarm came from a test fixture";
        let record = clear("will", why)
            .expect("the clear is written")
            .expect("a tripped audit to clear");
        assert_eq!(record.loop_id, ALL_LOOPS);
        let LoopAuditRow::AuditCleared(row) = &record.row else {
            panic!("expected a cleared row, got {record:?}");
        };
        assert_eq!((row.by.as_str(), row.reason.as_str()), ("will", why));
        assert_eq!(row.tripped, ["L-know", "L-placebo"]);
        assert_eq!(row.actor, Actor::Human);
        let rows = ledger.read().expect("read the ledger");
        assert_eq!(rows.last(), Some(&record), "the clear is on the record");
        assert!(!broken(), "the audit runs again");

        // A later trip breaks it again.
        let again = health("L-placebo", AuditState::Probation, None, false);
        ledger.append(&again).expect("append a tripped row");
        assert!(broken());
    }
}
