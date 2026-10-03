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
//! `ArmSet` asks it for a loop's layer and the policy the loop executes.

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

use chrono::{DateTime, Utc};
use roko_core::config::learning::{AuditEpoch, LearningAuditConfig};

use self::arm_set::{ArmDraws, MAXIMIZE_CONDITION, NORMAL_CONDITION};
use self::assign::{EXPLORE_EPSILON, HoldoutSchedule, LoopLayer};
use self::ledger::{Ledger, LoopAuditRecord, LoopAuditRow};
use self::state::{AuditParams, ExecutedPolicy, LoopStatus};
use crate::telemetry::LayerSpec;

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

/// A loop's latest audited state and its reason.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Standing {
    state: AuditState,
    reason: Option<ReasonCode>,
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
    /// on probation without one. The auditor is broken while any loop's
    /// latest health row says the placebo moved.
    #[must_use]
    pub fn from_records(
        registry: Registry,
        config: &LearningAuditConfig,
        records: &[LoopAuditRecord],
    ) -> Self {
        let mut standings = BTreeMap::new();
        let mut placebo_ok = BTreeMap::new();
        for record in records {
            let standing = match &record.row {
                LoopAuditRow::Health(health) => {
                    placebo_ok.insert(record.loop_id.as_str(), health.placebo_ok);
                    Standing {
                        state: health.state,
                        reason: health.reason,
                    }
                }
                LoopAuditRow::Transition(transition) => Standing {
                    state: transition.to,
                    reason: transition.reason,
                },
                _ => continue,
            };
            standings.insert(LoopId::new(record.loop_id.as_str()), standing);
        }
        Self {
            registry,
            config: config.clone(),
            standings,
            audit_broken: placebo_ok.values().any(|ok| !ok),
            maximize: false,
        }
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

    /// `loop_id`'s latest standing, else probation with no reason.
    fn standing(&self, loop_id: &str) -> Standing {
        self.standings
            .get(&LoopId::new(loop_id))
            .copied()
            .unwrap_or(Standing {
                state: AuditState::Probation,
                reason: None,
            })
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
    /// g, every loop's state and the schedule.
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
}
