//! Policy (b): the verify-then-escalate cascade: backlog task 6119.
//!
//! Policy (b) runs on the model ladder (S04 §4.4), the rungs a task's role can run, cheapest
//! first, each with its self-model forecast. Three pure decisions:
//!
//! - **Start rung**, by `[self_model] start_rule` (gap-74893d):
//!   - `expected_total`, the default ([`lowest_total_start`]): the rung with the lowest
//!     expected total, cost plus λ·time, to a verified pass, counting the climb each failure
//!     starts ([`total_to_vs`]);
//!   - `break_even` ([`start_rung`]), S04 §4.4 as decision 6101 adopted it: the first rung j
//!     whose lower bound on P(VS) beats the break-even c_j/c_{j+1} + m_b, else the last rung.
//!     Trying rung j before rung j + 1 pays only when it passes often enough to save the
//!     dearer rung's cost.
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
use super::policy::{P_ABANDON, SPEC_SCORE_MIN, TARGET, expected_cost, expected_latency};

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
/// last rung. `None` for no rungs. This is `[self_model] start_rule = "break_even"`, S04 §4.4
/// as decision 6101 adopted it. It weighs cost alone, takes the rung above as sure to pass,
/// and falls back to the last rung even where the forecasts rank it lowest (find-816b4b), so
/// [`lowest_total_start`] is the default.
#[must_use]
pub fn start_rung(rungs: &[CandidateForecast], margin: f64) -> Option<usize> {
    let last = rungs.len().checked_sub(1)?;
    let bars = break_even_bars(rungs, margin);
    let start = rungs
        .iter()
        .zip(&bars)
        .position(|(rung, bar)| bar.is_some_and(|bar| rung.p_vs_lcb > bar));
    Some(start.unwrap_or(last))
}

/// The bar each rung's lower bound on P(VS) must beat for [`start_rung`] to start there:
/// c_j/c_{j+1} + `margin`, the break-even against the rung above. `None` for the last rung,
/// and for a rung whose next one costs nothing.
#[must_use]
pub fn break_even_bars(rungs: &[CandidateForecast], margin: f64) -> Vec<Option<f64>> {
    let mut bars: Vec<Option<f64>> = rungs
        .windows(2)
        .map(|pair| {
            let above_cost = expected_cost(&pair[1]);
            (above_cost > 0.0).then(|| expected_cost(&pair[0]) / above_cost + margin)
        })
        .collect();
    if !rungs.is_empty() {
        bars.push(None);
    }
    bars
}

/// What one try on `forecast`'s rung is expected to cost in all, in USD: its expected cost
/// plus `usd_per_s` for each second it is expected to run, g = E[c] + λ·E[t] (gap-74893d).
#[must_use]
pub fn expected_total(forecast: &CandidateForecast, usd_per_s: f64) -> f64 {
    usd_per_s.mul_add(expected_latency(forecast), expected_cost(forecast))
}

/// The expected total, [`expected_total`] at `usd_per_s`, to a verified pass for a task that
/// starts on rung `start` of `rungs`, cheapest first. Each failure climbs one rung, at most
/// [`K_MAX`] times, and the highest rung the task can reach is retried until it passes:
/// G_j = g_j + (1 − p_j)·G_{j+1}, ending in g/p, where p is each rung's forecast P(VS).
/// Infinite when that last rung never passes, or for a start past the rungs.
#[must_use]
pub fn total_to_vs(rungs: &[CandidateForecast], start: usize, usd_per_s: f64) -> f64 {
    let climbs = usize::try_from(K_MAX).unwrap_or(usize::MAX);
    let Some(top) = rungs
        .len()
        .checked_sub(1)
        .map(|last| last.min(start.saturating_add(climbs)))
        .filter(|&top| top >= start)
    else {
        return f64::INFINITY;
    };
    let retried = &rungs[top];
    let p_top = retried.p_vs.clamp(0.0, 1.0);
    let at_top = if p_top > 0.0 {
        expected_total(retried, usd_per_s) / p_top
    } else {
        f64::INFINITY
    };
    rungs[start..top]
        .iter()
        .rev()
        .fold(at_top, |above, forecast| {
            let fails = 1.0 - forecast.p_vs.clamp(0.0, 1.0);
            let here = expected_total(forecast, usd_per_s);
            if fails > 0.0 {
                fails.mul_add(above, here)
            } else {
                here
            }
        })
}

/// The rung to start on by default, `[self_model] start_rule = "expected_total"`
/// (gap-74893d): the one with the lowest [`total_to_vs`] at `usd_per_s`, ties going to the
/// higher rung, and the last rung when no total is finite. `None` for no rungs. It weighs
/// time, counts every rung the climb may need, and has no fallback to choose (find-816b4b).
#[must_use]
pub fn lowest_total_start(rungs: &[CandidateForecast], usd_per_s: f64) -> Option<usize> {
    let last = rungs.len().checked_sub(1)?;
    let mut best = (last, f64::INFINITY);
    for start in (0..=last).rev() {
        let total = total_to_vs(rungs, start, usd_per_s);
        if total < best.1 {
            best = (start, total);
        }
    }
    Some(best.0)
}

/// Both start rules on a task's rungs, cheapest first, with the numbers behind each pick
/// (gap-74893d): the rung each would start on, whichever `[self_model] start_rule` routes, so
/// a run's predictions show what the other rule would have done.
#[derive(Debug, Clone, PartialEq)]
pub struct StartRules {
    /// S04 §4.4's pick, [`start_rung`] at [`BREAK_EVEN_MARGIN`], raised to the floor.
    pub break_even: Option<usize>,
    /// Each rung's break-even bar ([`break_even_bars`]).
    pub bars: Vec<Option<f64>>,
    /// The pick with the lowest expected total, [`lowest_total_start`] among the rungs from the
    /// floor up.
    pub expected_total: Option<usize>,
    /// For a start on each rung, its expected total to a verified pass ([`total_to_vs`]);
    /// `None` where that is infinite.
    pub totals: Vec<Option<f64>>,
}

impl StartRules {
    /// Both rules on `rungs` for a task that may start no lower than `floor` (0 when decision
    /// 6101 B1 lets it start below its tier's rung), the expected total pricing time at
    /// `usd_per_s`.
    #[must_use]
    pub fn of(rungs: &[CandidateForecast], floor: usize, usd_per_s: f64) -> Self {
        let totals = (0..rungs.len())
            .map(|start| {
                Some(total_to_vs(rungs, start, usd_per_s)).filter(|total| total.is_finite())
            })
            .collect();
        let expected_total = rungs
            .get(floor..)
            .and_then(|above| lowest_total_start(above, usd_per_s))
            .map(|start| start + floor);
        Self {
            break_even: start_rung(rungs, BREAK_EVEN_MARGIN).map(|start| start.max(floor)),
            bars: break_even_bars(rungs, BREAK_EVEN_MARGIN),
            expected_total,
            totals,
        }
    }
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
    // The prior must agree before L1 alone gives a task up (gap-d91691).
    let best_prior = options
        .iter()
        .map(|&(_, rung)| rungs[rung].prior_p_vs)
        .fold(f64::NEG_INFINITY, f64::max);
    if options.is_empty() || (best_p < P_ABANDON && best_prior < P_ABANDON) {
        return StepAction::Abandon;
    }
    if context
        .spec_score
        .is_some_and(|score| score < SPEC_SCORE_MIN)
        && best_p < TARGET
    {
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
            basis: crate::telemetry::ForecastBasis::Logit,
            prior_p_vs: p,
            cost_prior: false,
            latency_prior: false,
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

    /// One of Undertow's logged forecasts (round v7, `predictions.jsonl`): P(VS) `p` and its
    /// lower bound `lcb`, and the cost (USD) and latency (seconds) quantiles, q50 then q90.
    fn logged(
        model: &str,
        p: f64,
        lcb: f64,
        cost: (f64, f64),
        latency: (f64, f64),
    ) -> CandidateForecast {
        CandidateForecast {
            arm: ArmKey::roko("p", model),
            p_vs_lcb: lcb,
            cost_q90: cost.1,
            lat_q50_s: latency.0,
            lat_q90_s: latency.1,
            ..rung(0, p, cost.0)
        }
    }

    /// $3 an hour, the default time value, per second.
    const THREE_AN_HOUR: f64 = 3.0 / 3_600.0;

    /// find-816b4b: on phase 0 run 2's forecasts for T1 (Undertow run e578cf71, after six
    /// outcomes), no rung clears S04's break-even, so `break_even` falls back to strong, the
    /// rung its own forecast ranks lowest (P(VS) 0.11). The expected-total rule has no
    /// fallback. By cost alone it starts on cheap: $0.33 to a verified pass, against mid's
    /// $0.50 and strong's $0.61. At $3 an hour it starts on strong: $1.28, against cheap's
    /// $1.63 and mid's $1.81, since about nine 1.5-minute tries beat two 19-minute ones.
    #[test]
    fn start_rung_fallback_with_no_rung_at_break_even() {
        let t1 = [
            logged(
                "gpt-oss-120b",
                0.37444,
                0.17594,
                (0.01275, 0.05221),
                (311.0, 1273.5),
            ),
            logged(
                "glm-4.7",
                0.45351,
                0.30977,
                (0.12232, 0.31865),
                (746.1, 2417.0),
            ),
            logged(
                "gpt-5.4-mini",
                0.11206,
                0.0173,
                (0.04908, 0.14011),
                (54.1, 196.0),
            ),
        ];
        assert_eq!(start_rung(&t1, BREAK_EVEN_MARGIN), Some(2));
        assert_eq!(lowest_total_start(&t1, 0.0), Some(0));
        assert_eq!(lowest_total_start(&t1, THREE_AN_HOUR), Some(2));
        let cents = |start: usize, usd_per_s: f64| {
            (total_to_vs(&t1, start, usd_per_s) * 100.0).round() / 100.0
        };
        assert_eq!(
            [cents(0, 0.0), cents(1, 0.0), cents(2, 0.0)],
            [0.33, 0.50, 0.61]
        );
        let timed = [
            cents(0, THREE_AN_HOUR),
            cents(1, THREE_AN_HOUR),
            cents(2, THREE_AN_HOUR),
        ];
        assert_eq!(timed, [1.63, 1.81, 1.28]);
    }

    /// gap-74893d: on phase 1's forecasts for T3's first try (Undertow run 0bed5c58), cost
    /// alone starts on cheap, under either rule. At $3 an hour the expected-total rule starts
    /// on strong: $0.99 to a verified pass, against cheap's $1.08 and mid's $1.12. A strong
    /// try is forecast at about $0.08 and 1.4 minutes, a mid one at $0.18 and 12 minutes.
    #[test]
    fn the_start_rule_weighs_time_on_phase_one_forecasts() {
        let t3 = [
            logged(
                "gpt-oss-120b",
                0.51381,
                0.31364,
                (0.01275, 0.05221),
                (332.6, 1361.7),
            ),
            logged(
                "glm-4.7",
                0.67248,
                0.56486,
                (0.12498, 0.36864),
                (440.3, 1635.1),
            ),
            logged(
                "gpt-5.4-mini",
                0.15377,
                0.03864,
                (0.05053, 0.1772),
                (42.0, 192.5),
            ),
        ];
        assert_eq!(start_rung(&t3, BREAK_EVEN_MARGIN), Some(0));
        assert_eq!(lowest_total_start(&t3, 0.0), Some(0));
        assert_eq!(lowest_total_start(&t3, THREE_AN_HOUR), Some(2));
        let totals: Vec<f64> = (0..3)
            .map(|start| (total_to_vs(&t3, start, THREE_AN_HOUR) * 100.0).round() / 100.0)
            .collect();
        assert_eq!(totals, [1.08, 1.12, 0.99]);
    }

    /// gap-74893d: a start's total counts only the rungs its K_max climbs can reach, the
    /// highest of them retried until it passes; a rung that always passes ends the climb; and
    /// with no rung that can pass, the last rung stands.
    #[test]
    fn a_starts_total_counts_the_rungs_its_climbs_reach() {
        let rungs = [
            rung(0, 0.1, 0.01),
            rung(1, 0.1, 0.02),
            rung(2, 0.1, 0.03),
            rung(3, 1.0, 0.04),
        ];
        // From rung 0 two climbs reach rung 2, retried: 0.01 + 0.9·(0.02 + 0.9·0.03/0.1).
        let from_the_bottom = 0.9_f64.mul_add(0.9_f64.mul_add(0.3, 0.02), 0.01);
        assert!((total_to_vs(&rungs, 0, 0.0) - from_the_bottom).abs() < 1e-12);
        // From rung 2 the climb reaches rung 3, which always passes: 0.03 + 0.9·0.04.
        let from_rung_two = 0.9_f64.mul_add(0.04, 0.03);
        assert!((total_to_vs(&rungs, 2, 0.0) - from_rung_two).abs() < 1e-12);
        assert_eq!(lowest_total_start(&rungs, 0.0), Some(3));
        assert!(total_to_vs(&rungs, 4, 0.0).is_infinite());
        // A rung that always passes ends the climb: 0.01 + 0.9·0.02, the dear top unreached.
        let sure = [rung(0, 0.1, 0.01), rung(1, 1.0, 0.02), rung(2, 0.1, 0.5)];
        let before_the_sure_rung = 0.9_f64.mul_add(0.02, 0.01);
        assert!((total_to_vs(&sure, 0, 0.0) - before_the_sure_rung).abs() < 1e-12);
        assert_eq!(lowest_total_start(&sure, 0.0), Some(1));
        let never = [rung(0, 0.0, 0.01), rung(1, 0.0, 0.02)];
        assert!(total_to_vs(&never, 0, 0.0).is_infinite());
        assert_eq!(lowest_total_start(&never, 0.0), Some(1));
        assert_eq!(lowest_total_start(&[], 0.0), None);
    }

    /// gap-74893d: both rules' picks with their numbers, for the prediction record: on phase
    /// 1's T3 forecasts break-even starts on cheap, whose bound 0.31 beats its bar of about
    /// 0.18, and the expected total on strong. A floor (6101 B1 off) raises the break-even pick
    /// and limits the expected total to the rungs from the floor up; an infinite total is
    /// `None`.
    #[test]
    fn start_rules_record_both_picks_and_their_numbers() {
        let t3 = [
            logged(
                "gpt-oss-120b",
                0.51381,
                0.31364,
                (0.01275, 0.05221),
                (332.6, 1361.7),
            ),
            logged(
                "glm-4.7",
                0.67248,
                0.56486,
                (0.12498, 0.36864),
                (440.3, 1635.1),
            ),
            logged(
                "gpt-5.4-mini",
                0.15377,
                0.03864,
                (0.05053, 0.1772),
                (42.0, 192.5),
            ),
        ];
        let rules = StartRules::of(&t3, 0, THREE_AN_HOUR);
        assert_eq!((rules.break_even, rules.expected_total), (Some(0), Some(2)));
        let cheap_bar = rules.bars[0].expect("a bar below the top");
        assert!((0.17..0.19).contains(&cheap_bar), "{cheap_bar}");
        assert!(rules.bars[1].is_some_and(|bar| bar > 1.0) && rules.bars[2].is_none());
        let cents: Vec<f64> = rules
            .totals
            .iter()
            .map(|total| (total.expect("finite") * 100.0).round() / 100.0)
            .collect();
        assert_eq!(cents, [1.08, 1.12, 0.99]);

        let floored = StartRules::of(&t3, 1, 0.0);
        assert_eq!(
            (floored.break_even, floored.expected_total),
            (Some(1), Some(1))
        );
        let never = [rung(0, 0.0, 0.01), rung(1, 0.0, 0.02)];
        assert_eq!(StartRules::of(&never, 0, 0.0).totals, [None, None]);
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
