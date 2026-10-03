//! Off-policy evaluation of the policies from logged propensities: backlog task 6121.
//!
//! Once route decisions carry the composed propensity of the arm they chose, the value of a
//! routing policy that did not run can be estimated from the log (S04 §4.6). [`logged_decisions`]
//! turns decision rows ([`RoutingDecisionLog`]) into (context, action, propensity, reward)
//! samples, the reward being the joined verdict's label or its cost. [`ips`] gives the inverse
//! propensity estimate of a target policy's value with a 95% interval. The doubly robust
//! estimate of the self-model's benefit over the default arm is S03.T5's AIPW estimator in
//! [`crate::loop_audit::estimators`], which [`dr_benefit`] calls rather than writing a second one;
//! its interval is that estimator's empirical-Bernstein confidence sequence, valid at any
//! stopping time.

use serde::{Deserialize, Serialize};

use super::prior::normal_quantile;
use crate::loop_audit::estimators::{
    BenefitConfig, BenefitEstimator, BenefitRow, ChainOutcome, Estimate, PreAssignment,
};
use crate::routing_log::RoutingDecisionLog;
use crate::telemetry::GateVerdictTag;

/// What a logged decision's reward measures.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Reward {
    /// 1 when the joined verdict passed, else 0.
    Success,
    /// The attempt's cost, in USD.
    Cost,
}

/// One logged route decision as an off-policy sample.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LoggedDecision {
    /// The decision's context: role, complexity and category.
    pub context: String,
    /// The arm (model) it chose.
    pub action: String,
    /// The default arm of the decision, when logged.
    pub default_action: Option<String>,
    /// The composed probability with which the chosen arm was drawn, μ(a|x).
    pub propensity: f64,
    /// The reward.
    pub reward: f64,
}

/// The decision rows of `rows` that carry a usable propensity and the reward `reward`, as
/// samples, and how many rows were left out.
#[must_use]
pub fn logged_decisions(
    rows: &[RoutingDecisionLog],
    reward: Reward,
) -> (Vec<LoggedDecision>, usize) {
    let mut skipped = 0;
    let mut decisions = Vec::with_capacity(rows.len());
    for row in rows {
        let value = match reward {
            Reward::Success => row
                .outcome_success
                .map(|passed| f64::from(u8::from(passed))),
            Reward::Cost => row.outcome_cost_usd,
        };
        let propensity = row
            .propensity
            .filter(|propensity| *propensity > 0.0 && *propensity <= 1.0);
        let (Some(value), Some(propensity)) = (value, propensity) else {
            skipped += 1;
            continue;
        };
        decisions.push(LoggedDecision {
            context: format!("{}|{}|{}", row.role, row.task_complexity, row.task_category),
            action: row.selected_model.clone(),
            default_action: row.default_model.clone(),
            propensity,
            reward: value,
        });
    }
    (decisions, skipped)
}

/// An off-policy estimate and its 95% interval.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct OffPolicyEstimate {
    /// The estimated value.
    pub value: f64,
    /// Its 95% interval.
    pub interval: (f64, f64),
    /// The samples behind it.
    pub n: usize,
}

/// The inverse propensity estimate of the value of a target policy, which plays action `a` in
/// context `x` with probability `target(x, a)`: (1/n) Σ target(x_i, a_i)/μ_i · r_i, with a
/// normal 95% interval. `None` for fewer than two samples.
pub fn ips(
    decisions: &[LoggedDecision],
    target: impl Fn(&str, &str) -> f64,
) -> Option<OffPolicyEstimate> {
    if decisions.len() < 2 {
        return None;
    }
    let terms: Vec<f64> = decisions
        .iter()
        .map(|decision| {
            target(&decision.context, &decision.action) / decision.propensity * decision.reward
        })
        .collect();
    let n = terms.len() as f64;
    let mean = terms.iter().sum::<f64>() / n;
    let variance = terms.iter().map(|term| (term - mean).powi(2)).sum::<f64>() / (n - 1.0);
    let half = normal_quantile(0.975) * (variance / n).sqrt();
    Some(OffPolicyEstimate {
        value: mean,
        interval: (mean - half, mean + half),
        n: terms.len(),
    })
}

/// One decision between the self-model's arm and the default arm, for the doubly robust
/// benefit.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct DrSample {
    /// The self-model's arm ran, not the default.
    pub learned: bool,
    /// The logged probability of the default arm.
    pub p_default: f64,
    /// The attempt passed.
    pub passed: bool,
    /// The outcome model's predictions for the self-model's arm and the default.
    pub predictions: (f64, f64),
}

/// The doubly robust (AIPW) estimate of the self-model's benefit over the default arm, from
/// S03.T5's estimator. Route decisions carry no audit draw, so outcomes lie in [0, 1] and the
/// estimator's audit floor is 1.
#[must_use]
pub fn dr_benefit(samples: &[DrSample]) -> Estimate {
    let config = BenefitConfig {
        audit_floor: 1.0,
        ..BenefitConfig::default()
    };
    let mut estimator = BenefitEstimator::new(config);
    for sample in samples {
        let outcome = if sample.passed {
            ChainOutcome::Tagged(GateVerdictTag::Passed)
        } else {
            ChainOutcome::Failed
        };
        estimator.push(&BenefitRow {
            pre: PreAssignment {
                predictions: Some(sample.predictions),
                ..PreAssignment::default()
            },
            learned_arm: sample.learned,
            global_off: false,
            logged_default: sample.p_default,
            global_rate: 0.0,
            outcome,
            audit: None,
            cost: None,
        });
    }
    estimator.aipw()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::self_model::TestRng;

    #[test]
    fn ips_and_dr_cover_the_known_value() {
        // Four contexts with known pass rates for the learned and the default arm, and a
        // logged default propensity per context.
        let learned_rate = [0.8_f64, 0.6, 0.5, 0.3];
        let default_rate = [0.6_f64, 0.6, 0.4, 0.5];
        let p_default = [0.3_f64, 0.5, 0.6, 0.7];
        let value = learned_rate.iter().sum::<f64>() / 4.0;
        let benefit = value - default_rate.iter().sum::<f64>() / 4.0;
        let mut rng = TestRng::new(13);
        let (mut ips_covered, mut dr_covered) = (0_u32, 0_u32);
        for _ in 0..200 {
            let mut decisions = Vec::with_capacity(1_000);
            let mut samples = Vec::with_capacity(1_000);
            for _ in 0..1_000 {
                let k = ((rng.uniform() * 4.0) as usize).min(3);
                let learned = !rng.bernoulli(p_default[k]);
                let rate = if learned {
                    learned_rate[k]
                } else {
                    default_rate[k]
                };
                let passed = rng.bernoulli(rate);
                decisions.push(LoggedDecision {
                    context: format!("s{k}"),
                    action: if learned { "learned" } else { "default" }.to_string(),
                    default_action: Some("default".to_string()),
                    propensity: if learned {
                        1.0 - p_default[k]
                    } else {
                        p_default[k]
                    },
                    reward: f64::from(u8::from(passed)),
                });
                // A biased outcome model: DR stays unbiased with known propensities.
                let predictions = (
                    (learned_rate[k] + 0.1).min(1.0),
                    (default_rate[k] - 0.05).max(0.0),
                );
                samples.push(DrSample {
                    learned,
                    p_default: p_default[k],
                    passed,
                    predictions,
                });
            }
            let always_learned = |_: &str, action: &str| f64::from(u8::from(action == "learned"));
            let ips = ips(&decisions, always_learned).expect("an estimate");
            ips_covered += u32::from(ips.interval.0 <= value && value <= ips.interval.1);
            let (low, high) = dr_benefit(&samples).interval.expect("an interval");
            dr_covered += u32::from(low <= benefit && benefit <= high);
        }
        assert!(ips_covered >= 186, "IPS covered {ips_covered} of 200");
        assert!(dr_covered >= 186, "DR covered {dr_covered} of 200");
    }

    #[test]
    fn decision_rows_become_samples() {
        let row = |propensity: Option<f64>, passed: Option<bool>| {
            let row = serde_json::json!({
                "task_id": "T1",
                "role": "implementer",
                "task_complexity": "standard",
                "task_category": "implementation",
                "selected_model": "glm-4.7",
                "default_model": "gpt-oss-120b",
                "candidates": [],
                "outcome_success": passed,
                "outcome_cost_usd": 0.02,
                "propensity": propensity,
            });
            serde_json::from_value::<RoutingDecisionLog>(row).expect("a decision row")
        };
        let rows = [
            row(Some(0.8), Some(true)),
            row(None, Some(true)),
            row(Some(0.0), Some(false)),
            row(Some(0.2), None),
        ];
        let (decisions, skipped) = logged_decisions(&rows, Reward::Success);
        assert_eq!(skipped, 3);
        assert_eq!(decisions.len(), 1);
        assert_eq!(decisions[0].action, "glm-4.7");
        assert_eq!(decisions[0].default_action.as_deref(), Some("gpt-oss-120b"));
        assert_eq!(decisions[0].context, "implementer|standard|implementation");
        assert_eq!(decisions[0].reward, 1.0);
        let (costs, skipped) = logged_decisions(&rows, Reward::Cost);
        assert_eq!((costs.len(), skipped), (2, 2));
        assert_eq!(costs[1].reward, 0.02);
    }
}
