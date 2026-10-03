//! θ's last-known-good versions, committed through the guarded commit and
//! restored at start (S06 §4.5; P21).
//!
//! - [`HarnessParams`] is a [`GuardedState`]: its snapshot is θ's JSON.
//! - [`ThetaCheck`] is the `harness` store's [`CommitCheck`]. Held out: the
//!   controller's own commit condition, every EV inside its inner band for
//!   `recover_window` resolutions under the candidate. Anchors: the
//!   candidate sits on its ladders and the SafetyBox would restore it from
//!   θ₀, so it stays inside the S5 box.
//! - [`ThetaLkg`] commits the θ of a controller's [`ControllerEvent::Commit`]
//!   with provenance `ConfigSource::Evolved` and the reason
//!   `homeostat:<episode_id>/<change_id>`, and at start puts the top version
//!   back into the controller ([`ThetaLkg::restore_into`]): the restore the
//!   deleted `GuaranteedFinallyController` was to guarantee on an abnormal
//!   exit. A version the box refuses is never adopted, and the store
//!   enforces from its first version, so an invalid θ is never committed.

use std::path::Path;

use roko_core::config::harness_params::{HarnessLadders, HarnessParams};

use super::controller::{Controller, ControllerEvent, UNCHANGED_CONFIG};
use super::policy::{ChangeKind, SafetyBox, SafetyContext, ViabilityPolicy};
use crate::guarded_commit::{
    CheckOutcome, CommitCheck, CommitDecision, GuardError, GuardMode, GuardedState, GuardedStore,
    Proposer,
};

/// θ's guarded store, in `.roko/learn/commits/harness/`.
pub const STORE: &str = "harness";
/// The actor of the controller's commits.
pub const ACTOR: &str = "homeostat";

impl GuardedState for HarnessParams {
    fn snapshot(&self) -> Vec<u8> {
        serde_json::to_vec(self).expect("plain data serializes to JSON")
    }

    fn restore(&mut self, snapshot: &[u8]) -> Result<(), GuardError> {
        let theta: Self =
            serde_json::from_slice(snapshot).map_err(|error| GuardError::BadSnapshot {
                store: STORE.to_string(),
                reason: error.to_string(),
            })?;
        *self = theta;
        Ok(())
    }
}

/// The `harness` store's checks on a candidate θ.
#[derive(Debug, Clone)]
pub struct ThetaCheck {
    safety: SafetyBox,
    recovered: bool,
}

impl ThetaCheck {
    /// The checks around `theta0` on `ladders` under `policy`; `recovered`
    /// is whether the controller saw every EV inside its inner band for
    /// `recover_window` resolutions under the candidate.
    #[must_use]
    pub fn new(
        theta0: HarnessParams,
        ladders: HarnessLadders,
        policy: &ViabilityPolicy,
        recovered: bool,
    ) -> Self {
        Self {
            safety: SafetyBox::new(theta0, ladders, policy),
            recovered,
        }
    }

    /// Why `theta` is outside the S5 box, or `None` when the SafetyBox
    /// would restore it from θ₀.
    #[must_use]
    pub fn refusal(&self, theta: &HarnessParams) -> Option<String> {
        let verdict = self
            .safety
            .validate(self.safety.theta0(), theta, &restore_context());
        if verdict.passed() {
            return None;
        }
        Some(serde_json::to_string(&verdict.violations).unwrap_or_default())
    }
}

/// A restore of a whole θ: the context in which the SafetyBox judges a
/// version. The controller never edits the config, so before equals after.
fn restore_context() -> SafetyContext {
    SafetyContext {
        kind: ChangeKind::Restore,
        config_before: Some(UNCHANGED_CONFIG.to_string()),
        config_after: Some(UNCHANGED_CONFIG.to_string()),
        arm: None,
        adaptation_spend_usd: 0.0,
        run_spend_usd: 0.0,
        adaptation_spend_max_frac: 0.0,
        knowledge_loop_live: false,
    }
}

impl CommitCheck for ThetaCheck {
    fn held_out(&self, _candidate: &[u8], _lkg: Option<&[u8]>) -> CheckOutcome {
        if self.recovered {
            CheckOutcome::pass(
                "held_out",
                "every EV stayed inside its inner band for recover_window resolutions",
            )
        } else {
            CheckOutcome::fail(
                "held_out",
                "the controller has not seen recover_window calm resolutions under it",
            )
        }
    }

    fn anchors(&self, candidate: &[u8]) -> CheckOutcome {
        let mut theta = self.safety.theta0().clone();
        if let Err(error) = theta.restore(candidate) {
            return CheckOutcome::fail("anchors", error.to_string());
        }
        match self.refusal(&theta) {
            None => CheckOutcome::pass("anchors", "inside the S5 box"),
            Some(reason) => CheckOutcome::fail("anchors", reason),
        }
    }
}

/// θ's guarded store: the controller's commits, and the top version at
/// start.
#[derive(Debug, Clone)]
pub struct ThetaLkg {
    store: GuardedStore,
    theta0: HarnessParams,
    ladders: HarnessLadders,
    policy: ViabilityPolicy,
}

impl ThetaLkg {
    /// Open the `harness` store under `learn_dir`, in enforce mode:
    /// decision 8103's observe period is for the router and knowledge
    /// stores, and S06 never lets an unvalidated θ survive.
    ///
    /// # Errors
    ///
    /// The store's I/O and row errors.
    pub fn open(
        learn_dir: &Path,
        theta0: HarnessParams,
        ladders: HarnessLadders,
        policy: ViabilityPolicy,
    ) -> Result<Self, GuardError> {
        let store = GuardedStore::open(learn_dir, STORE, GuardMode::Enforce)?;
        Ok(Self {
            store,
            theta0,
            ladders,
            policy,
        })
    }

    /// The store, for its versions and commit rows.
    #[must_use]
    pub const fn store(&self) -> &GuardedStore {
        &self.store
    }

    /// Propose `theta` as the next version with `reason`; `recovered` is
    /// the controller's commit condition. A failed check keeps the current
    /// version (`rolled_back`).
    ///
    /// # Errors
    ///
    /// [`GuardError::NothingToRestore`] when the first proposal fails a
    /// check (nothing is committed), and the store's I/O errors.
    pub fn commit(
        &mut self,
        theta: &HarnessParams,
        reason: &str,
        recovered: bool,
    ) -> Result<CommitDecision, GuardError> {
        let check = ThetaCheck::new(
            self.theta0.clone(),
            self.ladders.clone(),
            &self.policy,
            recovered,
        );
        let proposer = Proposer::evolved(ACTOR, STORE, reason);
        let mut candidate = theta.clone();
        self.store.propose(&mut candidate, &check, &proposer)
    }

    /// Commit the θ of a controller's [`ControllerEvent::Commit`], which it
    /// emits only once its commit condition held; `None` for any other
    /// event.
    ///
    /// # Errors
    ///
    /// As [`Self::commit`].
    pub fn commit_event(
        &mut self,
        event: &ControllerEvent,
    ) -> Result<Option<CommitDecision>, GuardError> {
        let ControllerEvent::Commit { theta, reason } = event else {
            return Ok(None);
        };
        self.commit(theta, reason, true).map(Some)
    }

    /// The top version and its θ; `None` before the first.
    ///
    /// # Errors
    ///
    /// A snapshot that is not a θ, and the store's I/O errors.
    pub fn top(&self) -> Result<Option<(u64, HarnessParams)>, GuardError> {
        let mut theta = self.theta0.clone();
        let restored = self.store.restore_top(&mut theta)?;
        Ok(restored.map(|version| (version, theta)))
    }

    /// At start, after an abnormal exit or not: make the top version the
    /// controller's θ and last-known-good. Returns the version adopted;
    /// `None` when there is none, or when the SafetyBox refuses it and θ
    /// stays.
    ///
    /// # Errors
    ///
    /// As [`Self::top`].
    pub fn restore_into(&self, controller: &mut Controller) -> Result<Option<u64>, GuardError> {
        let Some((version, theta)) = self.top()? else {
            return Ok(None);
        };
        Ok(controller.adopt(theta).ok().map(|()| version))
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use roko_core::config::harness_params::{Knob, Step};
    use roko_core::config::homeostasis::{HomeostasisConfig, HomeostasisMode};
    use roko_core::config::{ConfigSource, ProviderConfig, RokoConfig};
    use roko_core::task::TaskTier;

    use super::*;
    use crate::homeostasis::controller::{EpisodeOutcome, Phase, ReleaseReason};
    use crate::homeostasis::detect::Baseline;
    use crate::homeostasis::ev::{DrivePolicy, EvBounds};
    use crate::homeostasis::policy::{AuditPolicy, VerifyPolicy};
    use crate::homeostasis::resolution::{CostSourceMix, TaskResolution};
    use crate::telemetry::{AttemptOutcome, CostSource};

    /// In control: 80% pass, $0.05 and five minutes per resolution.
    const BASELINE: Baseline = Baseline {
        pass_rate: 0.80,
        usd_per_resolution: 0.05,
        wall_ms: 300_000.0,
    };

    /// What a controller and the store are built from.
    struct Fixture {
        settings: HomeostasisConfig,
        policy: ViabilityPolicy,
        theta0: HarnessParams,
        ladders: HarnessLadders,
    }

    impl Fixture {
        fn new() -> Self {
            let mut config = RokoConfig::default();
            for name in ["alpha", "beta"] {
                config
                    .providers
                    .insert(name.to_string(), ProviderConfig::default());
            }
            Self {
                settings: HomeostasisConfig {
                    mode: HomeostasisMode::On,
                    random_step_prob: 0.0,
                    ..HomeostasisConfig::default()
                },
                policy: ViabilityPolicy {
                    policy_version: 1,
                    ev: EvBounds::s06_example(),
                    drive: DrivePolicy::default(),
                    tiers: BTreeMap::new(),
                    verify: VerifyPolicy::default(),
                    audit: AuditPolicy::default(),
                    holdout: 0.10,
                },
                theta0: HarnessParams::baseline(&config),
                ladders: HarnessLadders::from_config(&config),
            }
        }

        fn controller(&self) -> Controller {
            Controller::new(
                &self.settings,
                self.policy.clone(),
                self.theta0.clone(),
                self.ladders.clone(),
                BASELINE,
                7,
            )
        }

        fn load(&self, path: &Path) -> (Controller, Vec<ControllerEvent>) {
            Controller::load(
                path,
                &self.settings,
                self.policy.clone(),
                self.theta0.clone(),
                self.ladders.clone(),
                BASELINE,
                7,
            )
        }

        fn lkg(&self, learn_dir: &Path) -> ThetaLkg {
            ThetaLkg::open(
                learn_dir,
                self.theta0.clone(),
                self.ladders.clone(),
                self.policy.clone(),
            )
            .expect("open the harness store")
        }
    }

    /// Resolution `index`, a minute after the one before.
    fn resolution(index: u64, passed: bool) -> TaskResolution {
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
            wall_ms: Some(300_000),
            provider_errors: 0,
            conductor_restarts: 0,
            params_digest: None,
            arm: None,
            audited: false,
            audit_false_green: None,
            pre_instrumentation: false,
            resolved_at: i64::try_from(index * 60_000).ok(),
        }
    }

    #[test]
    fn lkg_restored_after_abnormal_exit() {
        let dir = tempfile::tempdir().expect("tempdir");
        let learn = dir.path().join("learn");
        let state_path = Controller::state_path(dir.path());
        let fixture = Fixture::new();

        // θ₁, the mechanical floor one rung up, committed through the guard.
        let theta1 = fixture
            .theta0
            .step(Knob::TierFloor(TaskTier::Mechanical), Step::Up, &fixture.ladders)
            .expect("cheap -> mid");
        let mut lkg = fixture.lkg(&learn);
        let decision = lkg
            .commit(&theta1, "homeostat:ep-0001/ch-0003", true)
            .expect("commit θ₁");
        assert_eq!(decision, CommitDecision::Committed);
        let rows = lkg.store().rows().expect("commit rows");
        assert_eq!(rows[0].source, ConfigSource::Evolved);
        assert_eq!(rows[0].reason.as_deref(), Some("homeostat:ep-0001/ch-0003"));
        assert_eq!(rows[0].actor, ACTOR);

        // A start adopts θ₁. A breach then opens an episode whose first move
        // takes θ past θ₁, the state is saved mid-episode, and the process
        // dies.
        let mut controller = fixture.controller();
        assert_eq!(lkg.restore_into(&mut controller).expect("restore"), Some(1));
        assert_eq!(controller.theta(), &theta1);
        for index in 1..=24 {
            let passed = index <= 20 && index % 5 != 1;
            controller.on_resolution(&resolution(index, passed));
        }
        assert_eq!(controller.phase(), Phase::Search);
        assert_ne!(controller.theta(), &theta1);
        controller.save(&state_path).expect("save mid-episode");
        drop(controller);
        drop(lkg);

        // The restart abandons the episode and gets θ₁ back.
        let (mut restarted, events) = fixture.load(&state_path);
        assert!(events.iter().any(|event| matches!(
            event,
            ControllerEvent::EpisodeClose {
                outcome: EpisodeOutcome::RolledBack,
                ..
            }
        )));
        let mut lkg = fixture.lkg(&learn);
        assert_eq!(lkg.restore_into(&mut restarted).expect("restore"), Some(1));
        assert_eq!(restarted.theta(), &theta1);
        assert_eq!(restarted.phase(), Phase::Idle);
        assert!(restarted.episode().is_none());

        // With the state lost too, the fresh controller still gets θ₁ back.
        std::fs::remove_file(&state_path).expect("lose the state");
        let (mut fresh, events) = fixture.load(&state_path);
        assert!(events.is_empty());
        assert_eq!(fresh.theta(), &fixture.theta0);
        assert_eq!(lkg.restore_into(&mut fresh).expect("restore"), Some(1));
        assert_eq!(fresh.theta(), &theta1);

        // A θ outside the box is never committed, nor one the controller has
        // not seen recover.
        let mut raised = theta1.clone();
        raised.task_budget_scale = 1.25;
        let decision = lkg
            .commit(&raised, "homeostat:ep-0002/ch-0001", true)
            .expect("judged");
        assert_eq!(decision, CommitDecision::RolledBack);
        let theta2 = theta1
            .step(Knob::RetryDelta, Step::Up, &fixture.ladders)
            .expect("one more retry");
        let decision = lkg
            .commit(&theta2, "homeostat:ep-0002/ch-0002", false)
            .expect("judged");
        assert_eq!(decision, CommitDecision::RolledBack);
        assert_eq!(lkg.store().current(), Some(1));
        assert_eq!(lkg.top().expect("top"), Some((1, theta1.clone())));

        // A controller's commit event commits; any other event does not.
        let commit = ControllerEvent::Commit {
            theta: Box::new(theta2.clone()),
            reason: "homeostat:ep-0002/ch-0003".to_string(),
        };
        assert_eq!(
            lkg.commit_event(&commit).expect("commit"),
            Some(CommitDecision::Committed)
        );
        let release = ControllerEvent::Release {
            reason: ReleaseReason::Ack,
        };
        assert_eq!(lkg.commit_event(&release).expect("ignored"), None);
        assert_eq!(lkg.store().current(), Some(2));

        // A top version tampered outside the box is refused at start, and θ
        // stays.
        let tampered = serde_json::to_vec(&raised).expect("θ to JSON");
        std::fs::write(lkg.store().dir().join("v2.snap"), tampered).expect("tamper with v2");
        let mut guarded = fixture.controller();
        assert_eq!(lkg.restore_into(&mut guarded).expect("refused"), None);
        assert_eq!(guarded.theta(), &fixture.theta0);
    }
}
