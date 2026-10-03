//! Policy (b): the verify-then-escalate cascade: backlog task 6119.
//!
//! Policy (b) runs on the model ladder (S04 §4.4), the rungs a task's role can run, cheapest
//! first, each with its self-model forecast. Three pure decisions:
//!
//! - **Start rung** ([`start_rung`]): the first rung j whose lower bound on P(VS) beats the
//!   break-even c_j/c_{j+1} + m_b, else the last rung. Trying rung j before rung j + 1 pays
//!   only when it passes often enough to save the dearer rung's cost.
//! - **After an agent-blamed failure** ([`after_failure`]): with the rungs re-forecast for a
//!   prior failure and its error class, the cheapest in expected cost to VS among retrying the
//!   rung, climbing one rung, and skipping one (flagged, and only where decision 6101 allows
//!   it: today it does not), within K_max climbs and the retry budget; else `refine_spec` or
//!   `abandon`. The conditional P(VS on a rung | the current one failed), when the data hold
//!   co-failures (`chen2026when`), replaces the climb target's forecast.
//! - **After a gate pass** ([`after_pass`]): with r = p_fg, request the deepest verify depth
//!   V_j where r·L_fg·d_j > c_j, on S05's V0–V4 scale; escalate the model ("pass but
//!   suspicious") only when r > r_max at the deepest depth; else accept and export r as
//!   `risk_fg`. Verification deepens before the model escalates (D11), and M3 only requests
//!   depth: S05's ladder is its single writer.
//!
//! This is the library; live use follows decision 6101 (6130 to 6132).

use serde::{Deserialize, Serialize};

use super::CandidateForecast;
use super::baselines::K_MAX;
use super::policy::{P_ABANDON, SPEC_SCORE_MIN, TARGET, expected_cost};

/// The break-even margin, m_b (S04 §4.10).
pub const BREAK_EVEN_MARGIN: f64 = 0.05;
/// The false-green risk above which a pass at the deepest depth escalates the model, r_max.
pub const FALSE_GREEN_RISK_MAX: f64 = 0.25;

/// The expected cost of reaching VS on a rung by running it until it passes: E[c]/P(VS).
#[must_use]
pub fn cost_to_vs(forecast: &CandidateForecast, p_vs: f64) -> f64 {
    if p_vs > 0.0 {
        expected_cost(forecast) / p_vs
    } else {
        f64::INFINITY
    }
}

/// The rung to start on: the first rung j with p_vs_lcb(j) > c_j/c_{j+1} + `margin`, else the
/// last rung. `None` for no rungs.
#[must_use]
pub fn start_rung(rungs: &[CandidateForecast], margin: f64) -> Option<usize> {
    let last = rungs.len().checked_sub(1)?;
    let start = rungs.windows(2).position(|pair| {
        let (here, above) = (&pair[0], &pair[1]);
        let above_cost = expected_cost(above);
        above_cost > 0.0 && here.p_vs_lcb > expected_cost(here) / above_cost + margin
    });
    Some(start.unwrap_or(last))
}

/// Where a task stands after an agent-blamed failure.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct FailureContext {
    /// The rung that failed.
    pub current: usize,
    /// The rungs climbed so far.
    pub climbs: u32,
    /// The attempts the task's retry budget has left.
    pub retries_left: u32,
    /// The spec score, when known.
    pub spec_score: Option<f64>,
    /// Decision 6101 allows skipping a rung (its B2 says no today).
    pub skip_allowed: bool,
}

/// What to do after an agent-blamed failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum StepAction {
    /// Retry the same rung.
    Retry,
    /// Climb one rung.
    Climb {
        /// The rung to climb to.
        to: usize,
    },
    /// Skip one rung, flagged.
    Skip {
        /// The rung to jump to.
        to: usize,
    },
    /// Send the spec back for refinement.
    RefineSpec,
    /// Give the task up.
    Abandon,
}

/// After an agent-blamed failure on `context.current`, the next step, with the `rungs`
/// re-forecast for the failure. `conditional(k)` gives P(VS on rung k | the current rung
/// failed) when the data hold it.
#[must_use]
pub fn after_failure(
    rungs: &[CandidateForecast],
    context: &FailureContext,
    conditional: &dyn Fn(usize) -> Option<f64>,
) -> StepAction {
    let current = context.current;
    let can_attempt = context.retries_left > 0;
    let can_climb = can_attempt && context.climbs < K_MAX;
    let mut options: Vec<(StepAction, usize)> = Vec::new();
    if can_attempt && current < rungs.len() {
        options.push((StepAction::Retry, current));
    }
    if can_climb && current + 1 < rungs.len() {
        options.push((StepAction::Climb { to: current + 1 }, current + 1));
    }
    if can_climb && context.skip_allowed && current + 2 < rungs.len() {
        options.push((StepAction::Skip { to: current + 2 }, current + 2));
    }
    let p_vs = |rung: usize| {
        let forecast = rungs[rung].p_vs;
        if rung == current {
            forecast
        } else {
            conditional(rung).unwrap_or(forecast)
        }
    };
    let best_p = options
        .iter()
        .map(|&(_, rung)| p_vs(rung))
        .fold(f64::NEG_INFINITY, f64::max);
    if options.is_empty() || best_p < P_ABANDON {
        return StepAction::Abandon;
    }
    if context.spec_score.is_some_and(|score| score < SPEC_SCORE_MIN) && best_p < TARGET {
        return StepAction::RefineSpec;
    }
    options
        .iter()
        .min_by(|a, b| {
            let cost = |&(_, rung): &(StepAction, usize)| cost_to_vs(&rungs[rung], p_vs(rung));
            cost(a).total_cmp(&cost(b))
        })
        .map_or(StepAction::Abandon, |&(action, _)| action)
}

/// One verify depth S05's ladder can run after a pass.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct DepthOption {
    /// The depth, V0 to V4.
    pub depth: u8,
    /// The chance it catches a false green, d_j.
    pub catch_rate: f64,
    /// What running it costs, c_j.
    pub cost_usd: f64,
}

/// What to do after a gate pass.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum PassAction {
    /// Accept the pass.
    Accept {
        /// P(false green), exported to S05 as the audit-tilt risk.
        risk_fg: f64,
    },
    /// Request a deeper verify depth first.
    Deepen {
        /// The depth requested, d*.
        depth: u8,
        /// P(false green).
        risk_fg: f64,
    },
    /// Escalate the model: the pass is suspicious even at the deepest depth.
    Escalate {
        /// P(false green).
        risk_fg: f64,
    },
}

/// After a pass verified at `depth_run`, with P(false green) `p_fg` and a false green's loss
/// `loss_fg` (L_fg), the next step over the `depths` S05 offers.
#[must_use]
pub fn after_pass(p_fg: f64, depth_run: u8, depths: &[DepthOption], loss_fg: f64) -> PassAction {
    let risk_fg = p_fg;
    let deepest = depths
        .iter()
        .filter(|option| option.depth > depth_run)
        .filter(|option| risk_fg * loss_fg * option.catch_rate > option.cost_usd)
        .max_by_key(|option| option.depth);
    if let Some(option) = deepest {
        return PassAction::Deepen {
            depth: option.depth,
            risk_fg,
        };
    }
    let max_depth = depths
        .iter()
        .map(|option| option.depth)
        .max()
        .unwrap_or(depth_run);
    if risk_fg > FALSE_GREEN_RISK_MAX && depth_run >= max_depth {
        PassAction::Escalate { risk_fg }
    } else {
        PassAction::Accept { risk_fg }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::self_model::ArmKey;

    /// A rung forecast: P(VS) `p` with its lower bound 0.05 below, at a fixed median cost
    /// `cost` (its 90th percentile equal, so the expected cost is the median).
    fn rung(index: usize, p: f64, cost: f64) -> CandidateForecast {
        CandidateForecast {
            arm: ArmKey::roko("p", format!("r{index}")),
            p_gate: p,
            p_fg: 0.05,
            p_vs: p,
            p_vs_lcb: p - 0.05,
            cost_q50: cost,
            cost_q90: cost,
            lat_q50_s: 60.0,
            lat_q90_s: 90.0,
            cold_start: false,
        }
    }

    fn no_cofailure(_: usize) -> Option<f64> {
        None
    }

    #[test]
    fn predicted_start_rung_skips_a_rung_below_break_even() {
        // Rung 0 must beat 0.01/0.02 + 0.05 = 0.55 but its bound is 0.30; rung 1 must beat
        // 0.02/0.20 + 0.05 = 0.15 and its bound is 0.85.
        let rungs = [rung(0, 0.35, 0.01), rung(1, 0.9, 0.02), rung(2, 0.95, 0.20)];
        assert_eq!(start_rung(&rungs, BREAK_EVEN_MARGIN), Some(1));
        // A cheap rung that passes often enough starts the task.
        let cheap_wins = [rung(0, 0.8, 0.01), rung(1, 0.9, 0.02)];
        assert_eq!(start_rung(&cheap_wins, BREAK_EVEN_MARGIN), Some(0));
        // No rung qualifies: start on the last.
        let weak = [rung(0, 0.2, 0.01), rung(1, 0.3, 0.012), rung(2, 0.4, 0.013)];
        assert_eq!(start_rung(&weak, BREAK_EVEN_MARGIN), Some(2));
        assert_eq!(start_rung(&[], BREAK_EVEN_MARGIN), None);
    }

    #[test]
    fn climbs_never_exceed_k_max() {
        let rungs = [rung(0, 0.2, 0.01), rung(1, 0.6, 0.02), rung(2, 0.9, 0.04)];
        let mut context = FailureContext {
            current: 0,
            climbs: 0,
            retries_left: 5,
            spec_score: None,
            skip_allowed: false,
        };
        // Rung 1 reaches VS at 0.02/0.6 against rung 0's 0.01/0.2: climb.
        let step = after_failure(&rungs, &context, &no_cofailure);
        assert_eq!(step, StepAction::Climb { to: 1 });
        // With K_max climbs spent it retries instead, however good the rung above.
        context = FailureContext {
            current: 1,
            climbs: K_MAX,
            ..context
        };
        let step = after_failure(&rungs, &context, &no_cofailure);
        assert_eq!(step, StepAction::Retry);
        // With no retries left it abandons.
        context.retries_left = 0;
        let step = after_failure(&rungs, &context, &no_cofailure);
        assert_eq!(step, StepAction::Abandon);
        // Skipping a rung needs decision 6101's leave; a co-failure can make it worth it.
        let skippable = FailureContext {
            current: 0,
            climbs: 0,
            retries_left: 5,
            spec_score: None,
            skip_allowed: true,
        };
        let correlated = |rung: usize| (rung == 1).then_some(0.05);
        let step = after_failure(&rungs, &skippable, &correlated);
        assert_eq!(step, StepAction::Skip { to: 2 });
        let forbidden = FailureContext {
            skip_allowed: false,
            ..skippable
        };
        let step = after_failure(&rungs, &forbidden, &correlated);
        assert!(!matches!(step, StepAction::Skip { .. }), "{step:?}");
        // A weak spec whose rungs all fall short goes back for refinement.
        let weak_spec = FailureContext {
            spec_score: Some(0.2),
            ..forbidden
        };
        let short = [rung(0, 0.3, 0.01), rung(1, 0.5, 0.02)];
        let step = after_failure(&short, &weak_spec, &no_cofailure);
        assert_eq!(step, StepAction::RefineSpec);
    }

    #[test]
    fn a_suspicious_pass_deepens_verification_before_it_escalates_the_model() {
        let depths = [
            DepthOption {
                depth: 1,
                catch_rate: 0.3,
                cost_usd: 0.01,
            },
            DepthOption {
                depth: 2,
                catch_rate: 0.5,
                cost_usd: 0.05,
            },
            DepthOption {
                depth: 4,
                catch_rate: 0.9,
                cost_usd: 0.40,
            },
        ];
        // r = 0.5 and L_fg = 1: V1 pays 0.15 > 0.01 and V2 0.25 > 0.05, V4 0.45 > 0.40, so
        // the deepest, V4, is requested before any model escalation.
        let step = after_pass(0.5, 0, &depths, 1.0);
        assert_eq!(
            step,
            PassAction::Deepen {
                depth: 4,
                risk_fg: 0.5,
            }
        );
        // Run at the deepest depth and still suspicious: escalate the model.
        let step = after_pass(0.5, 4, &depths, 1.0);
        assert_eq!(step, PassAction::Escalate { risk_fg: 0.5 });
        // A low risk is accepted and exported.
        let step = after_pass(0.1, 4, &depths, 1.0);
        assert_eq!(step, PassAction::Accept { risk_fg: 0.1 });
        // Suspicious, but no deeper depth is worth its cost below the deepest: accept.
        let step = after_pass(0.3, 1, &depths, 0.01);
        assert_eq!(step, PassAction::Accept { risk_fg: 0.3 });
    }
}
