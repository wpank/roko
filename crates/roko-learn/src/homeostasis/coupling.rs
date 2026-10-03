//! Audit coupling with M4: after a cost- or verification-reducing move the
//! audit rate on that class doubles for the next passes (S06 §4.6.5, 8127).
//!
//! The coupling is automatic, not a move. An applied `param.change` whose
//! catalog move is cost-reducing or decrease-only carries an
//! [`AuditCoupling`]: its class (a tier for a B1 rung, `all` for a global
//! knob) and [`AUDIT_BOOST_PASSES`]. [`AuditBoosts`] arms it, and each green
//! attempt of the class counts one pass of it. A change shadow mode only
//! logs arms nothing.
//!
//! [`audit_rate`] is the rate M4's lottery draws a green attempt at under
//! M1: its own ρ, times θ's audit boost (B7) and the coupling's doubling,
//! within S5's `[p_floor, p_max]` and never below ρ, so M1 can raise M4's
//! rate but never lower it. M4's spend cap still binds: its audit worker
//! stops when the budget runs out.
//!
//! [`AuditCoupling`]: super::ledger::AuditCoupling
//! [`AUDIT_BOOST_PASSES`]: super::ledger::AUDIT_BOOST_PASSES

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::ledger::ParamChange;
use super::policy::AuditPolicy;

/// The class a coupling of a global knob applies to: every task's.
pub const ALL_CLASSES: &str = "all";

/// The couplings that run: each class's passes left at a doubled rate.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditBoosts {
    passes_left: BTreeMap<String, u32>,
}

impl AuditBoosts {
    /// Arm the coupling `change` carries, when it was applied: its class's
    /// rate stays doubled for its passes from now, or longer while an
    /// earlier coupling on the class still runs.
    pub fn arm(&mut self, change: &ParamChange) {
        let Some(coupling) = change.audit_coupling.as_ref().filter(|_| change.applied) else {
            return;
        };
        let left = self.passes_left.entry(coupling.class.clone()).or_default();
        *left = (*left).max(coupling.boost_until_passes);
    }

    /// 2 while a coupling on `class`, or on every class, runs; else 1.
    #[must_use]
    pub fn factor(&self, class: &str) -> u32 {
        let runs = |key: &str| self.passes_left.get(key).is_some_and(|&left| left > 0);
        if runs(class) || runs(ALL_CLASSES) {
            2
        } else {
            1
        }
    }

    /// Count one green attempt of `class` against the couplings it is
    /// under.
    pub fn pass(&mut self, class: &str) {
        let mut count = |key: &str| {
            if let Some(left) = self.passes_left.get_mut(key) {
                *left = left.saturating_sub(1);
            }
        };
        count(class);
        if class != ALL_CLASSES {
            count(ALL_CLASSES);
        }
        self.passes_left.retain(|_, left| *left > 0);
    }
}

/// The rate M4's lottery draws a green attempt at, from its own `rho`, θ's
/// audit `boost` (B7) and the coupling `factor`: their product within S5's
/// `[p_floor, p_max]`, and never below `rho`. A boost and a factor of 1,
/// θ₀'s without a coupling, leave `rho` as it is.
#[must_use]
pub fn audit_rate(rho: f64, boost: u32, factor: u32, policy: &AuditPolicy) -> f64 {
    let multiple = boost.max(1).saturating_mul(factor.max(1));
    if multiple <= 1 {
        return rho;
    }
    let raised = (rho * f64::from(multiple))
        .max(policy.p_floor)
        .min(policy.p_max);
    raised.max(rho)
}

#[cfg(test)]
mod tests {
    use roko_core::config::RokoConfig;
    use roko_core::config::harness_params::{HarnessLadders, HarnessParams, Knob, Step};
    use roko_core::task::TaskTier;
    use serde_json::Value;

    use super::*;
    use crate::homeostasis::catalog::catalog_move;
    use crate::homeostasis::controller::{ChangeProposal, MoveReason};
    use crate::homeostasis::ledger::AUDIT_BOOST_PASSES;
    use crate::homeostasis::policy::{ChangeKind, Verdict};

    /// The `param.change` of the catalog move of `knob` one notch in
    /// `direction` from θ₀, as the controller writes it, `applied` in `on`
    /// mode.
    fn change(knob: Knob, direction: Step, applied: bool) -> ParamChange {
        let config = RokoConfig::default();
        let theta0 = HarnessParams::baseline(&config);
        let ladders = HarnessLadders::from_config(&config);
        let theta = theta0
            .step(knob, direction, &ladders)
            .expect("a notch from θ₀");
        let entry = catalog_move(knob.kind(), direction).expect("a catalog move");
        let proposal = ChangeProposal {
            change_id: "ch-0001".to_string(),
            episode_id: Some("ep-0001".to_string()),
            kind: ChangeKind::Search,
            reason: MoveReason::Directed,
            applied,
            knob,
            block: knob.block(),
            from: theta0.value(knob).unwrap_or(Value::Null),
            to: theta.value(knob).unwrap_or(Value::Null),
            verdict: Verdict::default(),
            predicted: None,
            audit_coupled: entry.audit_coupled(),
            theta,
        };
        ParamChange::from(&proposal)
    }

    fn close(left: f64, right: f64) -> bool {
        (left - right).abs() < 1e-9
    }

    /// S06 §4.6.5 (8127): an applied cost-reducing move, a lower tier cap,
    /// doubles the audit rate on its tier for the next 20 passes, within
    /// S5's `[p_floor, p_max]`, and a global cost cut doubles every class's.
    /// A free move, or a cut shadow mode only logs, arms nothing, and
    /// without a coupling M1 leaves M4's rate alone.
    #[test]
    fn audit_coupling_doubles_rate_after_cost_reducing_move() {
        let policy = AuditPolicy {
            p_floor: 0.10,
            p_max: 0.40,
        };
        let mut boosts = AuditBoosts::default();
        let focused_cap = Knob::TierCap(TaskTier::Focused);
        boosts.arm(&change(Knob::TierFloor(TaskTier::Focused), Step::Up, true));
        boosts.arm(&change(focused_cap, Step::Down, false));
        assert_eq!(boosts.factor("focused"), 1);
        let rate = audit_rate(0.10, 1, boosts.factor("focused"), &policy);
        assert!(close(rate, 0.10), "{rate}");

        // The applied cut doubles the focused tier's rate for 20 passes.
        boosts.arm(&change(focused_cap, Step::Down, true));
        for pass in 0..AUDIT_BOOST_PASSES {
            assert_eq!(boosts.factor("focused"), 2, "pass {pass}");
            assert_eq!(boosts.factor("mechanical"), 1, "pass {pass}");
            let rate = audit_rate(0.10, 1, boosts.factor("focused"), &policy);
            assert!(close(rate, 0.20), "pass {pass}: {rate}");
            boosts.pass("focused");
        }
        assert_eq!(boosts.factor("focused"), 1);

        // Fewer retries cut cost on every class.
        boosts.arm(&change(Knob::RetryDelta, Step::Down, true));
        assert_eq!(
            (boosts.factor("focused"), boosts.factor("mechanical")),
            (2, 2)
        );

        // Within S5's bounds, and never below M4's own rate.
        for (rho, boost, factor, expected) in [
            (0.05, 1, 2, 0.10),
            (0.25, 2, 2, 0.40),
            (0.45, 1, 2, 0.45),
            (0.05, 1, 1, 0.05),
        ] {
            let rate = audit_rate(rho, boost, factor, &policy);
            assert!(
                close(rate, expected),
                "{rho} x {boost} x {factor}: {rate}"
            );
        }
    }
}
