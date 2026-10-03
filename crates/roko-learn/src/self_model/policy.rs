//! Policy (a): the cheapest arm meeting an adaptive-conformal target: backlog task 6118.
//!
//! `lcb_aci` (S04 §4.3) takes the eligible arms the caller's guards leave (configured,
//! healthy, tool-capable, not trust-excluded) and keeps those whose lower bound on P(VS) clears
//! π* + τ and whose 90th-percentile cost fits the remaining budget:
//!
//! ```text
//! F = {a ∈ A : p_vs_lcb(a) ≥ π* + τ and cost_q90(a) ≤ B_remaining}
//! if F ≠ ∅:  a* = argmin_{a∈F} E[c_a] + (1 − p_vs(a))·C_rec
//! elif spec_score < s_min and no cheap arm's p_vs_lcb reaches π*:  refine_spec
//! elif max_a p_vs(a) < p_abandon:  abandon
//! else:  a* = argmax_a p_vs(a), flagged below_target
//! ```
//!
//! τ follows the adaptive-conformal update on self-routed attempts:
//! τ ← clip(τ + γ·((1 − ỹ) − (1 − π*)), τ_min, τ_max), with ỹ the VS label when audited, else
//! the gate label times (1 − p_fg). The clip means ACI's long-run guarantee does not carry over;
//! S06 watches `ev.m3.route_pass` instead. The policy draws nothing at random: holdout and
//! exploration belong to S03's route table, and the caller logs the composed propensity.

use serde::{Deserialize, Serialize};

use super::prior::normal_quantile;
use super::{ArmKey, CandidateForecast};

/// The target P(VS), π* (D11, S04 §4.10).
pub const TARGET: f64 = 0.80;
/// The margin's step size, γ.
pub const GAMMA: f64 = 0.02;
/// The margin's floor.
pub const TAU_MIN: f64 = -0.10;
/// The margin's ceiling.
pub const TAU_MAX: f64 = 0.15;
/// The spec score below which a task whose cheap arms all fall short is sent back for
/// refinement, s_min.
pub const SPEC_SCORE_MIN: f64 = 0.5;
/// The P(VS) below which a task is abandoned, p_abandon.
pub const P_ABANDON: f64 = 0.15;

/// Policy (a)'s parameters.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct LcbAciConfig {
    /// π*.
    pub target: f64,
    /// γ.
    pub gamma: f64,
    /// The margin's floor.
    pub tau_min: f64,
    /// The margin's ceiling.
    pub tau_max: f64,
    /// s_min.
    pub spec_score_min: f64,
    /// p_abandon.
    pub p_abandon: f64,
}

impl Default for LcbAciConfig {
    fn default() -> Self {
        Self {
            target: TARGET,
            gamma: GAMMA,
            tau_min: TAU_MIN,
            tau_max: TAU_MAX,
            spec_score_min: SPEC_SCORE_MIN,
            p_abandon: P_ABANDON,
        }
    }
}

/// What policy (a) decided for one attempt.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum RouteAction {
    /// Run the attempt on `arm`. With `below_target`, no arm met the target and this one is
    /// the likeliest to pass.
    Dispatch {
        /// The chosen arm.
        arm: ArmKey,
        /// No arm's lower bound met the target.
        below_target: bool,
    },
    /// Send the spec back for refinement first.
    RefineSpec,
    /// Give the task up: no arm is likely enough to pass.
    Abandon,
}

/// Policy (a)'s choice, with what it chose among.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RouteChoice {
    /// The action.
    pub action: RouteAction,
    /// The threshold π* + τ the lower bounds had to clear.
    pub threshold: f64,
    /// The arms in F: they cleared it and fit the budget.
    pub feasible: Vec<ArmKey>,
}

/// An arm's expected cost from its median and 90th percentile, taking the cost as
/// log-normal: E[c] = q50·exp(σ²/2), σ = ln(q90/q50)/Φ⁻¹(0.9).
#[must_use]
pub fn expected_cost(forecast: &CandidateForecast) -> f64 {
    let (q50, q90) = (forecast.cost_q50, forecast.cost_q90);
    let sigma = if q90 > q50 && q50 > 0.0 {
        (q90 / q50).ln() / normal_quantile(0.9)
    } else {
        0.0
    };
    q50 * (sigma * sigma / 2.0).exp()
}

/// The cheap arms of `eligible`: those whose expected cost is at most the median expected
/// cost among them.
fn cheap_arms(eligible: &[CandidateForecast]) -> Vec<&CandidateForecast> {
    let mut costs: Vec<f64> = eligible.iter().map(expected_cost).collect();
    costs.sort_by(f64::total_cmp);
    let Some(&median) = costs.get(costs.len().saturating_sub(1) / 2) else {
        return Vec::new();
    };
    eligible
        .iter()
        .filter(|forecast| expected_cost(forecast) <= median)
        .collect()
}

/// Policy (a), `lcb_aci`, with its adaptive margin τ.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LcbAci {
    /// The parameters.
    pub config: LcbAciConfig,
    /// The margin τ, which starts at 0.
    pub tau: f64,
}

impl LcbAci {
    /// The policy with `config` and τ = 0.
    #[must_use]
    pub const fn new(config: LcbAciConfig) -> Self {
        Self { config, tau: 0.0 }
    }

    /// The threshold π* + τ.
    #[must_use]
    pub fn threshold(&self) -> f64 {
        self.config.target + self.tau
    }

    /// Choose among the `eligible` arms' forecasts, given the task's remaining budget, its spec
    /// score when known, and the cost `recovery_cost` of continuing after a failure (C_rec).
    #[must_use]
    pub fn choose(
        &self,
        eligible: &[CandidateForecast],
        budget_remaining: Option<f64>,
        spec_score: Option<f64>,
        recovery_cost: f64,
    ) -> RouteChoice {
        let threshold = self.threshold();
        let feasible: Vec<&CandidateForecast> = eligible
            .iter()
            .filter(|forecast| forecast.p_vs_lcb >= threshold)
            .filter(|forecast| budget_remaining.is_none_or(|budget| forecast.cost_q90 <= budget))
            .collect();
        let objective = |forecast: &CandidateForecast| {
            expected_cost(forecast) + (1.0 - forecast.p_vs) * recovery_cost
        };
        let best = feasible
            .iter()
            .min_by(|a, b| objective(a).total_cmp(&objective(b)));
        let action = if let Some(best) = best {
            RouteAction::Dispatch {
                arm: best.arm.clone(),
                below_target: false,
            }
        } else if spec_score.is_some_and(|score| score < self.config.spec_score_min)
            && cheap_arms(eligible)
                .iter()
                .all(|forecast| forecast.p_vs_lcb < self.config.target)
        {
            RouteAction::RefineSpec
        } else {
            // The likeliest arm, the first of equals.
            let likeliest = eligible
                .iter()
                .min_by(|a, b| b.p_vs.total_cmp(&a.p_vs))
                .filter(|forecast| forecast.p_vs >= self.config.p_abandon);
            match likeliest {
                Some(forecast) => RouteAction::Dispatch {
                    arm: forecast.arm.clone(),
                    below_target: true,
                },
                None => RouteAction::Abandon,
            }
        };
        RouteChoice {
            action,
            threshold,
            feasible: feasible
                .iter()
                .map(|forecast| forecast.arm.clone())
                .collect(),
        }
    }

    /// Learn the outcome of a self-routed attempt: its VS label when audited, its gate label,
    /// and the forecast P(false green) of a pass.
    pub fn update(&mut self, y_vs: Option<bool>, y_gate: bool, p_fg: f64) {
        let observed = match y_vs {
            Some(verified) => f64::from(u8::from(verified)),
            None if y_gate => 1.0 - p_fg,
            None => 0.0,
        };
        let target = self.config.target;
        let step = self.config.gamma * ((1.0 - observed) - (1.0 - target));
        self.tau = (self.tau + step).clamp(self.config.tau_min, self.config.tau_max);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::self_model::TestRng;

    /// A calibrated forecast for model `m{index}`: P(VS) `p`, its lower bound 0.05 below, and
    /// a median cost `q50` with the 90th percentile at twice it.
    fn candidate(index: usize, p: f64, q50: f64) -> CandidateForecast {
        CandidateForecast {
            arm: ArmKey::roko("p", format!("m{index}")),
            p_gate: p,
            p_fg: 0.0,
            p_vs: p,
            p_vs_lcb: p - 0.05,
            cost_q50: q50,
            cost_q90: 2.0 * q50,
            lat_q50_s: 60.0,
            lat_q90_s: 120.0,
            cold_start: false,
        }
    }

    fn ladder(ps: &[f64]) -> Vec<CandidateForecast> {
        let costs = [0.01, 0.02, 0.04, 0.08, 0.16];
        ps.iter()
            .zip(costs)
            .enumerate()
            .map(|(index, (p, q50))| candidate(index, *p, q50))
            .collect()
    }

    #[test]
    fn lcb_aci_failure_rate_tracks_target() {
        let truth = [0.55, 0.7, 0.8, 0.9, 0.97];
        let arms = ladder(&truth);
        let mut policy = LcbAci::new(LcbAciConfig::default());
        let mut rng = TestRng::new(21);
        let mut failures = 0_u32;
        for _ in 0..1_000 {
            let choice = policy.choose(&arms, None, None, 0.05);
            let RouteAction::Dispatch { arm, .. } = choice.action else {
                panic!("a calibrated ladder always dispatches: {choice:?}");
            };
            let index = arms
                .iter()
                .position(|forecast| forecast.arm == arm)
                .expect("a candidate");
            let passed = rng.bernoulli(truth[index]);
            failures += u32::from(!passed);
            policy.update(Some(passed), passed, 0.0);
        }
        let rate = f64::from(failures) / 1_000.0;
        assert!((rate - 0.20).abs() <= 0.05, "{rate}");
        assert!((TAU_MIN..=TAU_MAX).contains(&policy.tau));
    }

    #[test]
    fn each_action_fires_under_its_condition() {
        let policy = LcbAci::new(LcbAciConfig::default());
        // F holds the 0.9 and 0.97 arms: the cheaper one wins.
        let arms = ladder(&[0.55, 0.7, 0.8, 0.9, 0.97]);
        let choice = policy.choose(&arms, None, None, 0.05);
        assert_eq!(
            choice.action,
            RouteAction::Dispatch {
                arm: arms[3].arm.clone(),
                below_target: false,
            }
        );
        assert_eq!(choice.feasible, [arms[3].arm.clone(), arms[4].arm.clone()]);
        // A budget the 0.9 arm's 90th percentile overruns leaves no arm in F.
        let tight = policy.choose(&arms, Some(0.15), None, 0.05);
        assert_eq!(
            tight.action,
            RouteAction::Dispatch {
                arm: arms[4].arm.clone(),
                below_target: true,
            }
        );
        // No arm reaches the target: a weak spec goes back for refinement, a good one runs on
        // the likeliest arm, and a hopeless task is abandoned.
        let short = ladder(&[0.4, 0.5, 0.6]);
        let refine = policy.choose(&short, None, Some(0.3), 0.05);
        assert_eq!(refine.action, RouteAction::RefineSpec);
        let below = policy.choose(&short, None, Some(0.9), 0.05);
        assert_eq!(
            below.action,
            RouteAction::Dispatch {
                arm: short[2].arm.clone(),
                below_target: true,
            }
        );
        let hopeless = ladder(&[0.05, 0.1]);
        let abandon = policy.choose(&hopeless, None, None, 0.05);
        assert_eq!(abandon.action, RouteAction::Abandon);
        let nothing = policy.choose(&[], None, None, 0.05);
        assert_eq!(nothing.action, RouteAction::Abandon);
    }

    #[test]
    fn the_policy_is_deterministic_and_its_margin_moves_with_failures() {
        let arms = ladder(&[0.55, 0.7, 0.8, 0.9, 0.97]);
        let policy = LcbAci::new(LcbAciConfig::default());
        assert_eq!(
            policy.choose(&arms, None, None, 0.05),
            policy.choose(&arms, None, None, 0.05)
        );
        let mut failing = LcbAci::new(LcbAciConfig::default());
        failing.update(None, false, 0.1);
        // A failure raises τ by γ·(1 − 0.2).
        assert!((failing.tau - 0.016).abs() < 1e-12, "{}", failing.tau);
        let mut audited = LcbAci::new(LcbAciConfig::default());
        audited.update(None, true, 0.5);
        // A pass with p_fg = 0.5 counts half: τ moves by γ·(0.5 − 0.2).
        assert!((audited.tau - 0.006).abs() < 1e-12, "{}", audited.tau);
        for _ in 0..100 {
            failing.update(Some(false), true, 0.0);
        }
        assert_eq!(failing.tau, TAU_MAX, "the clip holds");
    }
}
