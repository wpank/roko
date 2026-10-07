//! The S5 viability policy, which M1 only reads, and the `SafetyBox` that
//! every θ change must pass (S06 §4.6).
//!
//! [`ViabilityPolicy`] is `.roko/policy/viability.toml`: the EV bounds and
//! inner bands, the drive's weights, the tiers, the minimum verify rungs
//! and the ceiling of M1's verify-depth floor, the audit floor and ceiling,
//! and the holdout rate. A human writes it and edits it; this module opens
//! it to read and has no way to write it.
//!
//! [`SafetyBox::validate`] admits a change θ_old → θ_new only when:
//!
//! 1. no config section outside M1's own `[homeostasis]` changed: tool
//!    allowlists, sandbox, roles, providers, models, credentials and budget
//!    ceilings are not θ fields, and the fingerprint of the rest of the
//!    config is compared before and after;
//! 2. θ_new sits on its ladders, and a search move never lowers a
//!    verification floor (B3) or an audit boost (B7). The effective verify
//!    depth is the higher of S05's ladder, S5's minimum rungs and M1's
//!    floor, so M1 can withdraw only rungs it asked for;
//! 3. the task budget share never exceeds θ₀'s and a search move never
//!    raises it (B8 is decrease-only), and a search move waits while the
//!    adaptation spend is over A_max of run spend;
//! 4. the boosted audit rate stays within S5's `p_max`, and M1's
//!    verify-depth floor (B3) within S5's `verify.max_floor`;
//! 5. search, relaxation and rollback change one notch of one knob (an
//!    adjacent provider swap counts as one), and relaxation moves toward θ₀;
//! 6. a holdout row runs θ₀;
//! 7. a search move switches the knowledge section on only for a live loop;
//! 8. a search move never turns a knob that acts on no dispatch
//!    (bug-35a738): B6, since a verify run stops at its first failed step
//!    and the promise tracker never sees two low readings in a row, and a
//!    tier's floor below θ₀'s, since the floor only raises the ladder's
//!    start rung (8101).
//!
//! Restoring a last-known-good θ may change several blocks at once; every
//! other rule still applies to it.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use roko_core::config::RokoConfig;
use roko_core::config::harness_params::{
    Block, EXTRA_RUNGS_NOTCHES, HarnessLadders, HarnessParams, Knob, VerifyDepth,
};
use roko_core::config::homeostasis::HomeostasisConfig;
use serde::{Deserialize, Serialize};

use super::ev::{DrivePolicy, Ev, EvBounds};
use crate::telemetry::Arm;

/// The policy file, relative to the `.roko` directory.
pub const VIABILITY_POLICY_FILE: &str = "policy/viability.toml";

/// The holdout share when the policy names none (S06 §4.8).
pub const DEFAULT_HOLDOUT: f64 = 0.10;

/// The S5 viability policy (S06 §5). M1 reads it and never writes it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ViabilityPolicy {
    /// The policy's version, stamped on every controller record.
    pub policy_version: u32,
    /// The EV bounds and inner bands.
    pub ev: EvBounds,
    /// The drive's shape and weights; D9's when absent.
    #[serde(default)]
    pub drive: DrivePolicy,
    /// The models each tier stands for. B1 moves along `[routing.ladder]`
    /// (decision 8101), so M1 reads this map but no check needs it yet.
    #[serde(default)]
    pub tiers: BTreeMap<String, Vec<String>>,
    /// The verify rungs every task keeps, and how deep a floor M1 may ask
    /// for.
    #[serde(default)]
    pub verify: VerifyPolicy,
    /// The audit rate's floor and ceiling.
    #[serde(default)]
    pub audit: AuditPolicy,
    /// The holdout share, at most 0.5 (S06 §4.8).
    #[serde(default = "default_holdout")]
    pub holdout: f64,
}

const fn default_holdout() -> f64 {
    DEFAULT_HOLDOUT
}

/// The verify rungs every task keeps whatever M1 does, and the ceiling of
/// the verify-depth floor M1 may request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VerifyPolicy {
    /// `authored` (the task's own verify steps, always kept) and verify
    /// depths `V0`..`V4` on S05 §4.6's scale.
    pub min_rungs: Vec<String>,
    /// The deepest verify-depth floor B3 may request: S5's ceiling, as
    /// `p_max` is B7's. S05's ladder may still check deeper on its own. V4,
    /// the top of B3's ladder, when the policy names none.
    #[serde(default = "top_floor")]
    pub max_floor: VerifyDepth,
}

impl Default for VerifyPolicy {
    fn default() -> Self {
        Self {
            min_rungs: vec!["authored".to_string()],
            max_floor: top_floor(),
        }
    }
}

/// The top of B3's ladder: S5's verify-depth ceiling when it names none.
const fn top_floor() -> VerifyDepth {
    VerifyDepth::V4
}

/// The audit rate's bounds (M4).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuditPolicy {
    /// The rate never goes below this.
    pub p_floor: f64,
    /// M1's audit boost never takes the rate above this.
    pub p_max: f64,
}

impl Default for AuditPolicy {
    fn default() -> Self {
        Self {
            p_floor: 0.10,
            p_max: 0.40,
        }
    }
}

/// Why the policy cannot be used.
#[derive(Debug, thiserror::Error)]
pub enum PolicyError {
    /// The file exists but could not be read.
    #[error("read {path}: {source}")]
    Read {
        /// The policy file.
        path: String,
        /// The I/O error.
        #[source]
        source: std::io::Error,
    },
    /// The file is not a policy.
    #[error("parse {path}: {source}")]
    Parse {
        /// The policy file.
        path: String,
        /// The TOML error.
        #[source]
        source: toml::de::Error,
    },
    /// The policy parses but cannot be right.
    #[error("{path}: {problem}")]
    Invalid {
        /// The policy file.
        path: String,
        /// What is wrong.
        problem: String,
    },
}

impl ViabilityPolicy {
    /// `<roko_dir>/policy/viability.toml`.
    #[must_use]
    pub fn path_in(roko_dir: &Path) -> PathBuf {
        roko_dir.join(VIABILITY_POLICY_FILE)
    }

    /// Read and check the policy at `path`.
    ///
    /// # Errors
    ///
    /// [`PolicyError`] when the file cannot be read or parsed, or fails
    /// [`Self::problems`].
    pub fn load(path: &Path) -> Result<Self, PolicyError> {
        let shown = path.display().to_string();
        let text = std::fs::read_to_string(path).map_err(|source| PolicyError::Read {
            path: shown.clone(),
            source,
        })?;
        Self::parse(&text).map_err(|error| match error {
            PolicyError::Parse { source, .. } => PolicyError::Parse {
                path: shown.clone(),
                source,
            },
            PolicyError::Invalid { problem, .. } => PolicyError::Invalid {
                path: shown.clone(),
                problem,
            },
            other => other,
        })
    }

    /// The policy of `<roko_dir>`, or `None` when it has none: without
    /// bounds a human has written, M1 has nothing to regulate.
    ///
    /// # Errors
    ///
    /// As [`Self::load`], for a file that exists.
    pub fn load_optional(roko_dir: &Path) -> Result<Option<Self>, PolicyError> {
        let path = Self::path_in(roko_dir);
        if !path.exists() {
            return Ok(None);
        }
        Self::load(&path).map(Some)
    }

    /// Parse and check policy text.
    ///
    /// # Errors
    ///
    /// [`PolicyError::Parse`] or [`PolicyError::Invalid`], with an empty
    /// path.
    pub fn parse(text: &str) -> Result<Self, PolicyError> {
        let policy: Self = toml::from_str(text).map_err(|source| PolicyError::Parse {
            path: String::new(),
            source,
        })?;
        let problems = policy.problems();
        if problems.is_empty() {
            Ok(policy)
        } else {
            Err(PolicyError::Invalid {
                path: String::new(),
                problem: problems.join("; "),
            })
        }
    }

    /// What makes the policy unusable; empty when it is sound.
    #[must_use]
    pub fn problems(&self) -> Vec<String> {
        let mut problems = Vec::new();
        if self.policy_version == 0 {
            problems.push("policy_version must be at least 1".to_string());
        }
        let (lo, inner) = (self.ev.pass_rate.lo, self.ev.pass_rate.inner());
        if !((0.0..1.0).contains(&lo) && inner > lo && inner <= 1.0) {
            problems.push("ev.pass_rate needs 0 <= lo < inner <= 1".to_string());
        }
        for ev in [Ev::UsdPerVerifiedSuccess, Ev::FalseGreen, Ev::LatencyP90S] {
            let (outer, inner) = (self.ev.outer(ev), self.ev.inner(ev));
            if !(outer > 0.0 && (0.0..=outer).contains(&inner)) {
                problems.push(format!("ev.{} needs 0 <= inner <= hi, hi > 0", ev.name()));
            }
        }
        let weights = Ev::ALL.map(|ev| self.drive.weights.get(ev));
        let negative = weights
            .iter()
            .any(|weight| weight.is_nan() || *weight < 0.0);
        if self.drive.n == 0 || self.drive.m == 0 || negative {
            problems.push("drive needs n, m >= 1 and weights >= 0".to_string());
        }
        let audit = &self.audit;
        if !((0.0..=audit.p_max).contains(&audit.p_floor) && audit.p_max <= 1.0) {
            problems.push("audit needs 0 <= p_floor <= p_max <= 1".to_string());
        }
        if !(0.0..=0.5).contains(&self.holdout) {
            problems.push("holdout must be between 0 and 0.5".to_string());
        }
        for rung in &self.verify.min_rungs {
            if rung != "authored" && verify_depth(rung).is_none() {
                problems.push(format!("verify.min_rungs: unknown rung `{rung}`"));
            }
        }
        problems
    }

    /// The deepest verify depth S5's `min_rungs` names; V0 when it names
    /// only `authored`.
    #[must_use]
    pub fn min_verify_depth(&self) -> VerifyDepth {
        self.verify
            .min_rungs
            .iter()
            .filter_map(|rung| verify_depth(rung))
            .max()
            .unwrap_or_default()
    }

    /// The verify depth a task gets under θ: the higher of S5's minimum and
    /// M1's floor. S05's ladder raises it further and lowers it never, and
    /// the authored steps always run.
    #[must_use]
    pub fn effective_verify_depth(&self, params: &HarnessParams) -> VerifyDepth {
        self.min_verify_depth().max(params.extra_rungs)
    }
}

fn verify_depth(name: &str) -> Option<VerifyDepth> {
    EXTRA_RUNGS_NOTCHES
        .into_iter()
        .find(|depth| depth.label() == name)
}

/// The fingerprint of every config section but M1's own `[homeostasis]`:
/// what no θ change may alter (§4.6.1). `None` when the config cannot be
/// fingerprinted, which [`SafetyBox::validate`] treats as a change.
#[must_use]
pub fn non_m1_fingerprint(config: &RokoConfig) -> Option<String> {
    let mut others = config.clone();
    others.homeostasis = HomeostasisConfig::default();
    roko_core::config::fingerprint(&others)
        .ok()
        .map(|fingerprint| fingerprint.hash)
}

// ── The SafetyBox ─────────────────────────────────────────────────────

/// Why θ changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChangeKind {
    /// A guided or random search move.
    Search,
    /// One notch back toward θ₀ after a long calm (anti-ratchet).
    Relax,
    /// Undoing the last change.
    Rollback,
    /// Restoring a last-known-good θ, which may differ in several blocks.
    Restore,
}

/// What [`SafetyBox::validate`] knows about a change besides the two θs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SafetyContext {
    /// Why θ changes.
    pub kind: ChangeKind,
    /// [`non_m1_fingerprint`] before the change.
    pub config_before: Option<String>,
    /// [`non_m1_fingerprint`] after the change.
    pub config_after: Option<String>,
    /// The arm of the dispatch θ is for; `None` for M1's own θ.
    pub arm: Option<Arm>,
    /// What adaptation has cost so far: spend beyond the holdout arm's.
    pub adaptation_spend_usd: f64,
    /// The run's spend so far.
    pub run_spend_usd: f64,
    /// A_max, the share of run spend adaptation may take (`[homeostasis]
    /// adaptation_spend_max_frac`).
    pub adaptation_spend_max_frac: f64,
    /// Whether M2 rates the knowledge loop live (S03).
    pub knowledge_loop_live: bool,
}

/// One broken rule.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "rule", rename_all = "snake_case")]
pub enum Violation {
    /// A config section outside `[homeostasis]` changed, or could not be
    /// fingerprinted (§4.6.1).
    ConfigChanged,
    /// θ_new does not sit on its ladders.
    Inadmissible {
        /// Why.
        reason: String,
    },
    /// A search move lowered an add-only knob: a verification floor (B3)
    /// or an audit boost (B7) (§4.6.2, §4.6.5).
    AddOnlyLowered {
        /// The knob.
        knob: Knob,
    },
    /// The task budget share went above θ₀'s, or a search move raised it
    /// (§4.6.3).
    BudgetRaised {
        /// The share before.
        from: f64,
        /// The share after.
        to: f64,
    },
    /// A search move while adaptation spend is over A_max (§4.6.3).
    AdaptationSpendExhausted {
        /// Adaptation spend so far.
        spend_usd: f64,
        /// A_max of run spend.
        cap_usd: f64,
    },
    /// The boosted audit rate passes S5's ceiling.
    AuditAboveMax {
        /// The boosted rate.
        rate: f64,
        /// S5's `p_max`.
        max: f64,
    },
    /// M1's verify-depth floor (B3) passes S5's ceiling.
    FloorAboveMax {
        /// The floor θ requests.
        floor: VerifyDepth,
        /// S5's `verify.max_floor`.
        max: VerifyDepth,
    },
    /// The change spans several blocks.
    SeveralBlocks {
        /// The blocks.
        blocks: Vec<Block>,
    },
    /// The change is not one notch of one knob.
    NotOneNotch {
        /// The knobs that changed.
        knobs: Vec<Knob>,
    },
    /// The change changes nothing.
    NoChange,
    /// A relaxation moved a knob away from θ₀.
    RelaxAwayFromTheta0 {
        /// The knob.
        knob: Knob,
    },
    /// A holdout row's θ is not θ₀ (§4.6.7).
    HoldoutNotTheta0,
    /// A search move switched the knowledge section on while the loop is
    /// not live (§4.6.8).
    SectionOnForInactiveLoop,
    /// A search move turned a knob that acts on no dispatch: B6, or a tier's
    /// floor below θ₀'s (bug-35a738).
    InertMove {
        /// The knob.
        knob: Knob,
    },
}

/// The validator's answer.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Verdict {
    /// Every rule the change breaks; empty when it passes.
    pub violations: Vec<Violation>,
}

impl Verdict {
    /// Whether the change passes.
    #[must_use]
    pub fn passed(&self) -> bool {
        self.violations.is_empty()
    }

    /// `pass` or `fail`, as `param.change` rows write it.
    #[must_use]
    pub fn label(&self) -> &'static str {
        if self.passed() { "pass" } else { "fail" }
    }
}

/// The hard bounds of every θ change (S06 §4.6). They are not part of the
/// search space: the controller asks, the box answers.
#[derive(Debug, Clone, PartialEq)]
pub struct SafetyBox {
    theta0: HarnessParams,
    ladders: HarnessLadders,
    audit: AuditPolicy,
    /// S5's ceiling of B3's verify-depth floor.
    max_floor: VerifyDepth,
}

impl SafetyBox {
    /// The box around `theta0`, on `ladders`, under `policy`.
    #[must_use]
    pub fn new(theta0: HarnessParams, ladders: HarnessLadders, policy: &ViabilityPolicy) -> Self {
        Self {
            theta0,
            ladders,
            audit: policy.audit,
            max_floor: policy.verify.max_floor,
        }
    }

    /// θ₀, which the holdout arm always runs.
    #[must_use]
    pub const fn theta0(&self) -> &HarnessParams {
        &self.theta0
    }

    /// Check the change `old` → `new`.
    #[must_use]
    pub fn validate(
        &self,
        old: &HarnessParams,
        new: &HarnessParams,
        context: &SafetyContext,
    ) -> Verdict {
        let mut violations = Vec::new();
        if context.config_before.is_none() || context.config_before != context.config_after {
            violations.push(Violation::ConfigChanged);
        }
        if let Err(error) = new.validate(&self.ladders) {
            violations.push(Violation::Inadmissible {
                reason: error.to_string(),
            });
        }
        let search = context.kind == ChangeKind::Search;
        if search {
            check_add_only(old, new, &mut violations);
            self.check_inert(old, new, &mut violations);
        }
        self.check_budget(old, new, context, &mut violations);
        self.check_audit(new, &mut violations);
        self.check_floor(new, &mut violations);
        if context.kind != ChangeKind::Restore {
            self.check_one_notch(old, new, context.kind, &mut violations);
        }
        if context.arm.is_some_and(is_holdout) && *new != self.theta0 {
            violations.push(Violation::HoldoutNotTheta0);
        }
        if search && new.knowledge_section && !old.knowledge_section && !context.knowledge_loop_live
        {
            violations.push(Violation::SectionOnForInactiveLoop);
        }
        Verdict { violations }
    }

    /// The per-dispatch assertion: a holdout row runs θ₀ (§4.6.7).
    #[must_use]
    pub fn check_dispatch(&self, arm: Option<Arm>, params: &HarnessParams) -> Verdict {
        let mut violations = Vec::new();
        if arm.is_some_and(is_holdout) && *params != self.theta0 {
            violations.push(Violation::HoldoutNotTheta0);
        }
        Verdict { violations }
    }

    fn check_budget(
        &self,
        old: &HarnessParams,
        new: &HarnessParams,
        context: &SafetyContext,
        violations: &mut Vec<Violation>,
    ) {
        let raised_by_search =
            context.kind == ChangeKind::Search && new.task_budget_scale > old.task_budget_scale;
        if new.task_budget_scale > self.theta0.task_budget_scale || raised_by_search {
            violations.push(Violation::BudgetRaised {
                from: old.task_budget_scale,
                to: new.task_budget_scale,
            });
        }
        let cap_usd = context.adaptation_spend_max_frac * context.run_spend_usd;
        let spend = context.adaptation_spend_usd;
        if context.kind == ChangeKind::Search && (spend.is_nan() || spend > cap_usd) {
            violations.push(Violation::AdaptationSpendExhausted {
                spend_usd: spend,
                cap_usd,
            });
        }
    }

    fn check_audit(&self, new: &HarnessParams, violations: &mut Vec<Violation>) {
        let rate = self.audit.p_floor * f64::from(new.audit_boost);
        if new.audit_boost > 1 && rate > self.audit.p_max {
            violations.push(Violation::AuditAboveMax {
                rate,
                max: self.audit.p_max,
            });
        }
    }

    fn check_floor(&self, new: &HarnessParams, violations: &mut Vec<Violation>) {
        if new.extra_rungs > self.max_floor {
            violations.push(Violation::FloorAboveMax {
                floor: new.extra_rungs,
                max: self.max_floor,
            });
        }
    }

    /// A search move never turns a knob that acts on no dispatch
    /// (bug-35a738). B6's promise tracker reads each step of a verify run,
    /// and the run stops at its first failed step, so the tracker never sees
    /// two low readings in a row and no B6 notch changes a run. A tier's
    /// floor only raises the ladder's start rung (8101), so below θ₀'s it
    /// binds nothing. Rollback, relaxation and restore may still undo either.
    fn check_inert(
        &self,
        old: &HarnessParams,
        new: &HarnessParams,
        violations: &mut Vec<Violation>,
    ) {
        for knob in old.changed_knobs(new) {
            let inert = match knob {
                Knob::PromiseMin | Knob::PromiseConsecutive => true,
                Knob::TierFloor(_) => self.below_theta0(new, knob),
                _ => false,
            };
            if inert {
                violations.push(Violation::InertMove { knob });
            }
        }
    }

    /// Whether `new` puts `knob` on a lower notch than θ₀ does.
    fn below_theta0(&self, new: &HarnessParams, knob: Knob) -> bool {
        let notch = |params: &HarnessParams| params.notch(knob, &self.ladders).ok();
        match (notch(new), notch(&self.theta0)) {
            (Some(after), Some(home)) => after < home,
            _ => false,
        }
    }

    fn check_one_notch(
        &self,
        old: &HarnessParams,
        new: &HarnessParams,
        kind: ChangeKind,
        violations: &mut Vec<Violation>,
    ) {
        let knobs = old.changed_knobs(new);
        if knobs.is_empty() {
            violations.push(Violation::NoChange);
            return;
        }
        let blocks = old.changed_blocks(new);
        if blocks.len() > 1 {
            violations.push(Violation::SeveralBlocks {
                blocks: blocks.into_iter().collect(),
            });
        }
        let distances: Vec<Option<usize>> = knobs
            .iter()
            .map(|&knob| self.notch_distance(old, new, knob))
            .collect();
        let one_step = distances.iter().all(|distance| *distance == Some(1));
        let swap = knobs.len() == 2
            && knobs
                .iter()
                .all(|knob| matches!(knob, Knob::ProviderRank(_)));
        if !one_step || (knobs.len() > 1 && !swap) {
            violations.push(Violation::NotOneNotch {
                knobs: knobs.clone(),
            });
        }
        if kind == ChangeKind::Relax {
            for &knob in &knobs {
                if !self.closer_to_theta0(old, new, knob) {
                    violations.push(Violation::RelaxAwayFromTheta0 { knob });
                }
            }
        }
    }

    fn notch_distance(
        &self,
        old: &HarnessParams,
        new: &HarnessParams,
        knob: Knob,
    ) -> Option<usize> {
        let before = old.notch(knob, &self.ladders).ok()?;
        let after = new.notch(knob, &self.ladders).ok()?;
        Some(before.abs_diff(after))
    }

    fn closer_to_theta0(&self, old: &HarnessParams, new: &HarnessParams, knob: Knob) -> bool {
        let notch = |params: &HarnessParams| params.notch(knob, &self.ladders).ok();
        match (notch(old), notch(new), notch(&self.theta0)) {
            (Some(before), Some(after), Some(home)) => after.abs_diff(home) < before.abs_diff(home),
            _ => false,
        }
    }
}

fn check_add_only(old: &HarnessParams, new: &HarnessParams, violations: &mut Vec<Violation>) {
    if new.extra_rungs < old.extra_rungs {
        violations.push(Violation::AddOnlyLowered {
            knob: Knob::ExtraRungs,
        });
    }
    if new.audit_boost < old.audit_boost {
        violations.push(Violation::AddOnlyLowered {
            knob: Knob::AuditBoost,
        });
    }
}

const fn is_holdout(arm: Arm) -> bool {
    matches!(arm, Arm::Default | Arm::GlobalOff)
}

#[cfg(test)]
mod tests {
    use rand::{Rng, SeedableRng};
    use rand_chacha::ChaCha8Rng;
    use roko_core::config::ProviderConfig;
    use roko_core::config::harness_params::Step;
    use roko_core::task::TaskTier;

    use super::*;

    /// S06 §5's example policy, with a tighter audit ceiling so boosts can
    /// break it, a V1 verify minimum and a V3 ceiling on M1's floor.
    const POLICY: &str = r#"
policy_version = 1
ev.pass_rate = { lo = 0.70, inner = 0.75 }
ev.usd_per_verified_success = { hi = 0.12, inner = 0.108, abs_cap = 0.50 }
ev.false_green = { hi = 0.10 }
ev.latency_p90_s = { hi = 900, inner = 810 }
drive = { n = 3, m = 2, weights = { pass_rate = 1.0, usd_per_verified_success = 1.0, false_green = 2.0, latency_p90_s = 0.5 } }
tiers = { cheap = ["gpt-oss-120b"], mid = ["glm-4.7"], strong = ["kimi-k2.6"] }
verify = { min_rungs = ["authored", "V1"], max_floor = "V3" }
audit = { p_floor = 0.10, p_max = 0.25 }
"#;

    fn config() -> RokoConfig {
        let mut config = RokoConfig::default();
        for name in ["alpha", "beta", "gamma"] {
            config
                .providers
                .insert(name.to_string(), ProviderConfig::default());
        }
        config
    }

    fn fixture() -> (SafetyBox, HarnessParams, HarnessLadders, ViabilityPolicy) {
        let config = config();
        let theta0 = HarnessParams::baseline(&config);
        let ladders = HarnessLadders::from_config(&config);
        let policy = ViabilityPolicy::parse(POLICY).expect("the example policy parses");
        let safety = SafetyBox::new(theta0.clone(), ladders.clone(), &policy);
        (safety, theta0, ladders, policy)
    }

    fn context(kind: ChangeKind) -> SafetyContext {
        SafetyContext {
            kind,
            config_before: Some("b3:config".to_string()),
            config_after: Some("b3:config".to_string()),
            arm: Some(Arm::Learned),
            adaptation_spend_usd: 0.0,
            run_spend_usd: 10.0,
            adaptation_spend_max_frac: 0.15,
            knowledge_loop_live: false,
        }
    }

    #[test]
    fn policy_parses_read_only_and_rejects_bad_bounds() {
        let policy = ViabilityPolicy::parse(POLICY).expect("parse");
        assert_eq!(policy.policy_version, 1);
        assert_eq!(policy.ev, EvBounds::s06_example());
        assert_eq!(policy.drive, DrivePolicy::default());
        assert_eq!(policy.tiers.len(), 3);
        assert_eq!(policy.holdout, DEFAULT_HOLDOUT);
        assert_eq!(policy.min_verify_depth(), VerifyDepth::V1);
        assert_eq!(
            policy.effective_verify_depth(&HarnessParams::baseline(&config())),
            VerifyDepth::V1
        );

        // Loaded from a read-only file: M1 only ever reads it.
        let dir = tempfile::tempdir().expect("tempdir");
        assert!(
            ViabilityPolicy::load_optional(dir.path())
                .expect("no policy")
                .is_none()
        );
        let path = ViabilityPolicy::path_in(dir.path());
        std::fs::create_dir_all(path.parent().expect("policy dir")).expect("mkdir");
        std::fs::write(&path, POLICY).expect("write the policy");
        let mut permissions = std::fs::metadata(&path).expect("stat").permissions();
        permissions.set_readonly(true);
        std::fs::set_permissions(&path, permissions).expect("make it read-only");
        let loaded = ViabilityPolicy::load_optional(dir.path())
            .expect("a read-only policy loads")
            .expect("a policy");
        assert_eq!(loaded, policy);
        assert_eq!(std::fs::read_to_string(&path).expect("read back"), POLICY);

        // Bounds that cannot be right, unknown rungs and unknown keys fail.
        let bad = POLICY.replace("p_max = 0.25", "p_max = 0.05");
        assert!(matches!(
            ViabilityPolicy::parse(&bad),
            Err(PolicyError::Invalid { .. })
        ));
        let bad = POLICY.replace("\"V1\"", "\"V9\"");
        assert!(ViabilityPolicy::parse(&bad).is_err());
        let bad = POLICY.replace("lo = 0.70, inner = 0.75", "lo = 0.80, inner = 0.75");
        assert!(ViabilityPolicy::parse(&bad).is_err());
        let bad = format!("{POLICY}allowed_tools = []\n");
        assert!(matches!(
            ViabilityPolicy::parse(&bad),
            Err(PolicyError::Parse { .. })
        ));
    }

    #[test]
    fn safety_box_admits_one_notch_and_rejects_each_rule() {
        let (safety, theta0, ladders, _) = fixture();
        let search = context(ChangeKind::Search);
        let raised = theta0
            .step(Knob::ExtraRungs, Step::Up, &ladders)
            .expect("V0 -> V1");
        assert!(safety.validate(&theta0, &raised, &search).passed());
        assert_eq!(safety.validate(&theta0, &raised, &search).label(), "pass");

        let rules = |old: &HarnessParams, new: &HarnessParams, context: &SafetyContext| {
            safety.validate(old, new, context).violations
        };
        // A search never lowers a verification floor; a rollback may.
        assert_eq!(
            rules(&raised, &theta0, &search),
            [Violation::AddOnlyLowered {
                knob: Knob::ExtraRungs
            }]
        );
        assert!(rules(&raised, &theta0, &context(ChangeKind::Rollback)).is_empty());
        // The budget share never rises above θ₀'s, and a search never
        // raises it.
        let cut = theta0
            .step(Knob::TaskBudgetScale, Step::Up, &ladders)
            .expect("1.0 -> 0.75");
        assert!(rules(&theta0, &cut, &search).is_empty());
        assert!(matches!(
            rules(&cut, &theta0, &search)[..],
            [Violation::BudgetRaised { .. }]
        ));
        assert!(rules(&cut, &theta0, &context(ChangeKind::Relax)).is_empty());
        let mut above = theta0.clone();
        above.task_budget_scale = 1.25;
        assert!(
            rules(&theta0, &above, &context(ChangeKind::Restore))
                .iter()
                .any(|violation| matches!(violation, Violation::BudgetRaised { .. }))
        );
        // A changed config section is a widening.
        let mut widened = search.clone();
        widened.config_after = Some("b3:other".to_string());
        assert_eq!(
            rules(&theta0, &raised, &widened),
            [Violation::ConfigChanged]
        );
        // Two blocks, or two notches, are not one move.
        let both = raised
            .step(Knob::RetryDelta, Step::Up, &ladders)
            .expect("retry +1");
        assert!(
            rules(&theta0, &both, &search)
                .iter()
                .any(|violation| matches!(violation, Violation::SeveralBlocks { .. }))
        );
        let two = raised
            .step(Knob::ExtraRungs, Step::Up, &ladders)
            .expect("V1 -> V2");
        assert!(matches!(
            rules(&theta0, &two, &search)[..],
            [Violation::NotOneNotch { .. }]
        ));
        assert!(rules(&theta0, &two, &context(ChangeKind::Restore)).is_empty());
        // An adjacent provider swap is one move.
        let swapped = theta0
            .step(Knob::ProviderRank(0), Step::Up, &ladders)
            .expect("swap");
        assert!(rules(&theta0, &swapped, &search).is_empty());
        // The audit boost stays within p_max: 2 × 0.10 passes, 4 × 0.10 not.
        let boosted = theta0
            .step(Knob::AuditBoost, Step::Up, &ladders)
            .expect("1x -> 2x");
        assert!(rules(&theta0, &boosted, &search).is_empty());
        let over = boosted
            .step(Knob::AuditBoost, Step::Up, &ladders)
            .expect("2x -> 4x");
        assert!(matches!(
            rules(&boosted, &over, &search)[..],
            [Violation::AuditAboveMax { .. }]
        ));
        // Spend over A_max stops search moves only.
        let mut spent = search.clone();
        spent.adaptation_spend_usd = 2.0;
        assert!(matches!(
            rules(&theta0, &raised, &spent)[..],
            [Violation::AdaptationSpendExhausted { .. }]
        ));
        spent.kind = ChangeKind::Rollback;
        assert!(rules(&raised, &theta0, &spent).is_empty());
        // A holdout row runs θ₀.
        let mut holdout = search.clone();
        holdout.arm = Some(Arm::Default);
        assert!(rules(&theta0, &raised, &holdout).contains(&Violation::HoldoutNotTheta0));
        assert!(safety.check_dispatch(Some(Arm::Default), &theta0).passed());
        assert!(!safety.check_dispatch(Some(Arm::Default), &raised).passed());
        assert!(safety.check_dispatch(Some(Arm::Learned), &raised).passed());
        // The knowledge section comes back on only for a live loop.
        let off = theta0
            .step(Knob::KnowledgeSection, Step::Down, &ladders)
            .expect("on -> off");
        assert!(rules(&theta0, &off, &search).is_empty());
        assert_eq!(
            rules(&off, &theta0, &search),
            [Violation::SectionOnForInactiveLoop]
        );
        let mut live = search.clone();
        live.knowledge_loop_live = true;
        assert!(rules(&off, &theta0, &live).is_empty());
        // Relaxation moves toward θ₀ only.
        assert!(matches!(
            rules(&theta0, &raised, &context(ChangeKind::Relax))[..],
            [Violation::RelaxAwayFromTheta0 { .. }]
        ));
        assert_eq!(rules(&theta0, &theta0, &search), [Violation::NoChange]);
        // An unfingerprintable config counts as changed.
        let mut unknown = search;
        unknown.config_before = None;
        unknown.config_after = None;
        assert_eq!(
            rules(&theta0, &raised, &unknown),
            [Violation::ConfigChanged]
        );
        // The non-M1 fingerprint ignores [homeostasis] and sees the rest.
        let base = config();
        let mut shadow_off = base.clone();
        shadow_off.homeostasis.mode = roko_core::config::homeostasis::HomeostasisMode::Off;
        assert_eq!(non_m1_fingerprint(&base), non_m1_fingerprint(&shadow_off));
        let mut widened_config = base.clone();
        widened_config.runner.dangerously_skip_permissions = true;
        assert_ne!(
            non_m1_fingerprint(&base),
            non_m1_fingerprint(&widened_config)
        );
    }

    /// gap-86286e: S5's `verify.max_floor` bounds B3 as `p_max` bounds B7.
    /// Under the policy's V3 ceiling a search raises M1's floor from V2 to
    /// V3 but not to V4, and no restore reaches V4 either. A policy that
    /// names no ceiling keeps B3's ladder up to V4, and an unknown depth
    /// does not parse.
    #[test]
    fn b3_is_bounded_by_the_s5_verify_depth_ceiling() {
        let (safety, theta0, ladders, policy) = fixture();
        assert_eq!(policy.verify.max_floor, VerifyDepth::V3);
        let search = context(ChangeKind::Search);
        let floor = |extra_rungs| HarnessParams {
            extra_rungs,
            ..theta0.clone()
        };
        let (v2, v3, v4) = (
            floor(VerifyDepth::V2),
            floor(VerifyDepth::V3),
            floor(VerifyDepth::V4),
        );
        assert!(safety.validate(&v2, &v3, &search).passed());
        let above = Violation::FloorAboveMax {
            floor: VerifyDepth::V4,
            max: VerifyDepth::V3,
        };
        let raised = safety.validate(&v3, &v4, &search).violations;
        assert_eq!(raised, [above.clone()]);
        let restored = safety.validate(&theta0, &v4, &context(ChangeKind::Restore));
        assert_eq!(restored.violations, [above]);

        let text = POLICY.replace(", max_floor = \"V3\"", "");
        let uncapped = ViabilityPolicy::parse(&text).expect("a policy without a ceiling");
        assert_eq!(uncapped.verify.max_floor, VerifyDepth::V4);
        let open = SafetyBox::new(theta0, ladders, &uncapped);
        assert!(open.validate(&v3, &v4, &search).passed());
        let bad = POLICY.replace("max_floor = \"V3\"", "max_floor = \"V9\"");
        assert!(ViabilityPolicy::parse(&bad).is_err());
    }

    /// bug-35a738: a search never turns a knob that acts on no dispatch. B6
    /// changes no verify run, since the run stops at its first failed step,
    /// and a floor below θ₀'s binds nothing, since the floor only raises the
    /// ladder's start rung (8101). A floor M1 raised still steps back down
    /// to θ₀'s, and rollback and relaxation undo either.
    #[test]
    fn search_never_turns_an_inert_knob() {
        let (safety, theta0, ladders, _) = fixture();
        let search = context(ChangeKind::Search);
        let rules = |old: &HarnessParams, new: &HarnessParams, context: &SafetyContext| {
            safety.validate(old, new, context).violations
        };
        let integrative = Knob::TierFloor(TaskTier::Integrative);
        assert_eq!(theta0.tier_floor[&TaskTier::Integrative], "mid");
        let below = theta0
            .step(integrative, Step::Down, &ladders)
            .expect("mid -> cheap");
        assert_eq!(
            rules(&theta0, &below, &search),
            [Violation::InertMove { knob: integrative }]
        );
        assert!(rules(&below, &theta0, &context(ChangeKind::Rollback)).is_empty());
        let raised = theta0
            .step(integrative, Step::Up, &ladders)
            .expect("mid -> strong");
        assert!(rules(&theta0, &raised, &search).is_empty());
        assert!(rules(&raised, &theta0, &search).is_empty());

        let promise = theta0
            .step(Knob::PromiseMin, Step::Up, &ladders)
            .expect("0.2 -> 0.3");
        assert_eq!(
            rules(&theta0, &promise, &search),
            [Violation::InertMove {
                knob: Knob::PromiseMin
            }]
        );
        assert!(rules(&promise, &theta0, &context(ChangeKind::Relax)).is_empty());
        let fewer = theta0
            .step(Knob::PromiseConsecutive, Step::Down, &ladders)
            .expect("2 -> 3");
        assert_eq!(
            rules(&theta0, &fewer, &search),
            [Violation::InertMove {
                knob: Knob::PromiseConsecutive
            }]
        );
    }

    /// A candidate θ: usually one notch, sometimes several knobs or
    /// notches, sometimes a raw value off its ladder, sometimes nothing.
    fn propose(
        rng: &mut ChaCha8Rng,
        theta: &HarnessParams,
        ladders: &HarnessLadders,
    ) -> HarnessParams {
        let knobs = theta.knobs();
        let knob = knobs[rng.gen_range(0..knobs.len())];
        let step = if rng.gen_bool(0.5) {
            Step::Up
        } else {
            Step::Down
        };
        match rng.gen_range(0..6) {
            0 | 1 => theta
                .step(knob, step, ladders)
                .unwrap_or_else(|_| theta.clone()),
            2 => {
                let notches = HarnessParams::notch_count(knob, ladders);
                theta
                    .with_notch(knob, rng.gen_range(0..notches), ladders)
                    .unwrap_or_else(|_| theta.clone())
            }
            3 => {
                let other = knobs[rng.gen_range(0..knobs.len())];
                theta
                    .step(knob, step, ladders)
                    .and_then(|next| next.step(other, step, ladders))
                    .unwrap_or_else(|_| theta.clone())
            }
            4 => {
                let mut next = theta.clone();
                match rng.gen_range(0..5) {
                    0 => next.task_budget_scale = 1.25,
                    1 => next.extra_rungs = VerifyDepth::V0,
                    2 => next.audit_boost = 8,
                    3 => next.turn_cap_mult = 3.0,
                    _ => next.provider_order.reverse(),
                }
                next
            }
            _ => theta.clone(),
        }
    }

    fn random_context(rng: &mut ChaCha8Rng) -> SafetyContext {
        let kinds = [
            ChangeKind::Search,
            ChangeKind::Relax,
            ChangeKind::Rollback,
            ChangeKind::Restore,
        ];
        let mut drawn = context(kinds[rng.gen_range(0..kinds.len())]);
        if rng.gen_bool(0.05) {
            drawn.config_after = Some("b3:widened".to_string());
        }
        if rng.gen_bool(0.05) {
            drawn.arm = Some(Arm::Default);
        }
        drawn.adaptation_spend_usd = rng.gen_range(0.0..2.0);
        drawn.knowledge_loop_live = rng.gen_bool(0.5);
        drawn
    }

    /// Why an admitted change is unsafe, judged from the two θs and the
    /// context alone; `None` when it is safe.
    fn escape(
        old: &HarnessParams,
        new: &HarnessParams,
        context: &SafetyContext,
        theta0: &HarnessParams,
        ladders: &HarnessLadders,
        policy: &ViabilityPolicy,
    ) -> Option<&'static str> {
        let search = context.kind == ChangeKind::Search;
        if context.config_before != context.config_after {
            return Some("a config section changed: a widening");
        }
        if new.validate(ladders).is_err() {
            return Some("θ off its ladders");
        }
        if policy.effective_verify_depth(new) < policy.min_verify_depth() {
            return Some("verification below S5's minimum");
        }
        if search && (new.extra_rungs < old.extra_rungs || new.audit_boost < old.audit_boost) {
            return Some("a search removed a verification floor or an audit boost");
        }
        if new.task_budget_scale > theta0.task_budget_scale {
            return Some("a task budget above its ceiling");
        }
        if search && new.task_budget_scale > old.task_budget_scale {
            return Some("a search raised the task budget");
        }
        if f64::from(new.audit_boost) * policy.audit.p_floor > policy.audit.p_max {
            return Some("an audit rate above p_max");
        }
        if new.extra_rungs > policy.verify.max_floor {
            return Some("a verification floor above S5's ceiling");
        }
        let cap_usd = context.adaptation_spend_max_frac * context.run_spend_usd;
        if search && context.adaptation_spend_usd > cap_usd {
            return Some("a search past the adaptation budget");
        }
        if context.arm == Some(Arm::Default) && new != theta0 {
            return Some("a holdout row off θ₀");
        }
        if context.kind != ChangeKind::Restore && old.changed_blocks(new).len() > 1 {
            return Some("several blocks in one move");
        }
        None
    }

    #[test]
    fn validator_never_admits_widening_removal_or_ceiling_raise() {
        // 10⁵ random sequences of one to three proposals from θ₀; each
        // admitted proposal becomes the next θ.
        let (safety, theta0, ladders, policy) = fixture();
        let mut rng = ChaCha8Rng::seed_from_u64(0x5afe_b0c5);
        let (mut admitted, mut rejected) = (0_u64, 0_u64);
        let mut escapes = Vec::new();
        for _ in 0..100_000 {
            let mut theta = theta0.clone();
            for _ in 0..rng.gen_range(1..=3) {
                let candidate = propose(&mut rng, &theta, &ladders);
                let context = random_context(&mut rng);
                if !safety.validate(&theta, &candidate, &context).passed() {
                    rejected += 1;
                    continue;
                }
                admitted += 1;
                if let Some(reason) =
                    escape(&theta, &candidate, &context, &theta0, &ladders, &policy)
                {
                    escapes.push((reason, theta.clone(), candidate.clone(), context));
                }
                theta = candidate;
            }
        }
        assert!(
            escapes.is_empty(),
            "{} escapes; the first: {:?}",
            escapes.len(),
            escapes.first()
        );
        assert!(
            admitted > 10_000 && rejected > 10_000,
            "{admitted} admitted, {rejected} rejected"
        );
    }
}
