//! E2 Monte Carlo simulator of false demotions under β = 0 (S03 C4; backlogs 5109, 5117).
//!
//! [`LegacyRule`] is a line-for-line copy of `PromptExperiment`'s rule as it
//! stood before backlog 5118 replaced it: UCB1 assignment, and a winner once
//! every arm has 10 trials, the leader's success rate beats the runner-up's
//! by 0.1, and either the χ² test gives p < 0.05 or, after 50 trials in all,
//! the two Wilson 95% intervals are disjoint. E2 prints its A/A false-winner
//! rate as the contrast (C4 row 6), so it must stay runnable after the fix.
//! `tests/fixtures/legacy_experiment_rule.json` holds the live rule's outcome
//! on 1,000 seeded A/A sequences; [`legacy_aa_run`] reproduces each one.

use serde::{Deserialize, Serialize};

/// Trials every arm needs before the legacy rule may conclude.
pub const LEGACY_MIN_TRIALS: u64 = 10;

/// Success-rate gap between the two leaders the legacy rule needs.
pub const LEGACY_MIN_EFFECT: f64 = 0.1;

/// Total trials after which the legacy rule's Wilson early stop applies.
pub const LEGACY_EARLY_STOP_TRIALS: u64 = 50;

/// 2^-53: scales the top 53 bits of a draw into `[0, 1)`.
const UNIT_SCALE: f64 = 1.0 / 9_007_199_254_740_992.0;

/// splitmix64, the outcome stream of the captured A/A sequences: any
/// language reproduces it exactly.
#[derive(Debug, Clone)]
pub struct SplitMix64(u64);

impl SplitMix64 {
    /// A stream seeded with `seed`.
    #[must_use]
    pub const fn new(seed: u64) -> Self {
        Self(seed)
    }

    /// The next 64 bits.
    pub const fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// The next draw in `[0, 1)`: the top 53 bits over 2^53.
    pub fn next_f64(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 * UNIT_SCALE
    }
}

/// One arm's counters.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ArmStats {
    /// Outcomes recorded.
    pub trials: u64,
    /// Successes among them.
    pub successes: u64,
}

impl ArmStats {
    /// The empirical success rate; 0 without trials.
    #[must_use]
    pub fn rate(self) -> f64 {
        if self.trials == 0 {
            0.0
        } else {
            self.successes as f64 / self.trials as f64
        }
    }

    /// The UCB1 score at `total` trials over every arm; an untried arm
    /// scores `f64::MAX`.
    fn ucb(self, total: u64) -> f64 {
        if self.trials == 0 {
            return f64::MAX;
        }
        let mean = self.successes as f64 / self.trials as f64;
        let exploration = (2.0 * (total as f64).ln() / self.trials as f64).sqrt();
        mean + exploration
    }

    /// The Wilson 95% interval of the success rate, with z = 1.96.
    fn wilson(self) -> (f64, f64) {
        if self.trials == 0 {
            return (0.0, 0.0);
        }
        let n = self.trials as f64;
        let p = self.rate();
        let z = 1.96_f64;
        let z_sq = z * z;
        let denom = 1.0 + z_sq / n;
        let center = (p + z_sq / (2.0 * n)) / denom;
        let margin = (z / denom) * ((p * (1.0 - p) / n + z_sq / (4.0 * n * n)).sqrt());
        (
            (center - margin).clamp(0.0, 1.0),
            (center + margin).clamp(0.0, 1.0),
        )
    }
}

/// `PromptExperiment`'s legacy rule over a fixed set of arms.
#[derive(Debug, Clone)]
pub struct LegacyRule {
    arms: Vec<ArmStats>,
    winner: Option<usize>,
}

impl LegacyRule {
    /// A running experiment over `arms` arms.
    #[must_use]
    pub fn new(arms: usize) -> Self {
        Self {
            arms: vec![ArmStats::default(); arms],
            winner: None,
        }
    }

    /// The arm UCB1 assigns next (the first arm wins a tie), or the winner
    /// once concluded.
    #[must_use]
    pub fn assign(&self) -> usize {
        if let Some(winner) = self.winner {
            return winner;
        }
        let total: u64 = self.arms.iter().map(|arm| arm.trials).sum();
        let mut best = 0;
        let mut best_score = f64::NEG_INFINITY;
        for (index, arm) in self.arms.iter().enumerate() {
            let score = arm.ucb(total);
            if score > best_score {
                best_score = score;
                best = index;
            }
        }
        best
    }

    /// Record an outcome of `arm`. Returns whether the rule concluded now.
    pub fn record(&mut self, arm: usize, success: bool) -> bool {
        if let Some(stats) = self.arms.get_mut(arm) {
            stats.trials += 1;
            stats.successes += u64::from(success);
        }
        if self.winner.is_none()
            && let Some(winner) = self.conclusion()
        {
            self.winner = Some(winner);
            return true;
        }
        false
    }

    /// The concluded winner, if any.
    #[must_use]
    pub const fn winner(&self) -> Option<usize> {
        self.winner
    }

    /// The arms by success rate, best first; a tie keeps arm order.
    fn ranked(&self) -> Vec<usize> {
        let mut ranked: Vec<usize> = (0..self.arms.len()).collect();
        ranked.sort_by(|a, b| self.arms[*b].rate().total_cmp(&self.arms[*a].rate()));
        ranked
    }

    /// `check_conclusion`: the leader, once every arm has its trials, the
    /// gap reaches the effect, and χ² or the early stop says so.
    fn conclusion(&self) -> Option<usize> {
        if self.arms.len() < 2 {
            return self.arms.first().map(|_| 0);
        }
        if self.arms.iter().any(|arm| arm.trials < LEGACY_MIN_TRIALS) {
            return None;
        }
        let ranked = self.ranked();
        let (best, second) = (self.arms[ranked[0]], self.arms[ranked[1]]);
        let significant = chi_squared_p(best, second) < 0.05;
        let early = self.early_stop() == Some(ranked[0]);
        let gap = best.rate() - second.rate();
        (gap >= LEGACY_MIN_EFFECT && (significant || early)).then_some(ranked[0])
    }

    /// `early_stopping_check`: the leader, when after 50 trials its Wilson
    /// interval lies above the runner-up's.
    fn early_stop(&self) -> Option<usize> {
        let total: u64 = self.arms.iter().map(|arm| arm.trials).sum();
        if self.arms.len() < 2 || total < LEGACY_EARLY_STOP_TRIALS {
            return None;
        }
        let ranked = self.ranked();
        let best = self.arms[ranked[0]].wilson();
        let second = self.arms[ranked[1]].wilson();
        (best.0 > second.1).then_some(ranked[0])
    }
}

/// `chi_squared_test`'s p-value: Pearson's χ² on the 2×2 table, one degree
/// of freedom, with a degenerate table non-significant.
fn chi_squared_p(a: ArmStats, b: ArmStats) -> f64 {
    if a.trials == 0 || b.trials == 0 {
        return 1.0;
    }
    let a_success = a.successes.min(a.trials) as f64;
    let b_success = b.successes.min(b.trials) as f64;
    let a_total = a.trials as f64;
    let b_total = b.trials as f64;
    let successes = a_success + b_success;
    let total = a_total + b_total;
    let failures = total - successes;
    if successes <= f64::EPSILON || failures <= f64::EPSILON {
        return 1.0;
    }
    let expected_a_success = a_total * successes / total;
    let expected_b_success = b_total * successes / total;
    let cells = [
        (a_success, expected_a_success),
        (a_total - a_success, a_total - expected_a_success),
        (b_success, expected_b_success),
        (b_total - b_success, b_total - expected_b_success),
    ];
    let statistic = cells
        .iter()
        .filter(|(_, expected)| *expected > f64::EPSILON)
        .map(|(observed, expected)| (observed - expected).powi(2) / expected)
        .sum::<f64>();
    erfc((statistic / 2.0).sqrt()).clamp(0.0, 1.0)
}

/// Abramowitz and Stegun 7.1.26, as the legacy rule computes it.
fn erfc(value: f64) -> f64 {
    let x = value.abs();
    let t = 1.0 / (1.0 + 0.327_591_1 * x);
    let polynomial =
        (((((1.061_405_429 * t - 1.453_152_027) * t) + 1.421_413_741) * t - 0.284_496_736) * t
            + 0.254_829_592)
            * t;
    let erf = 1.0 - polynomial * (-x * x).exp();
    if value >= 0.0 { 1.0 - erf } else { 1.0 + erf }
}

/// The legacy rule on one A/A sequence: `arms` arms that all succeed with
/// probability `p`, outcome t a success when `SplitMix64::new(seed)`'s t-th
/// draw is below `p`. Returns the outcomes recorded when it concluded and
/// the winning arm, or `None` within `horizon` outcomes.
#[must_use]
pub fn legacy_aa_run(seed: u64, arms: usize, p: f64, horizon: u64) -> Option<(u64, usize)> {
    let mut rule = LegacyRule::new(arms);
    let mut outcomes = SplitMix64::new(seed);
    for trial in 1..=horizon {
        let arm = rule.assign();
        if rule.record(arm, outcomes.next_f64() < p) {
            return rule.winner().map(|winner| (trial, winner));
        }
    }
    None
}

/// `tests/fixtures/legacy_experiment_rule.json`: the live rule's outcome on
/// the captured A/A sequences.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LegacyFixture {
    /// Outcomes per sequence before it counts as no winner.
    pub horizon: u64,
    /// Share of the sequences that declared a winner.
    pub false_winner_rate: f64,
    /// The sequences.
    pub sequences: Vec<LegacySequence>,
}

/// One captured A/A sequence.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LegacySequence {
    /// The splitmix64 seed of its outcomes.
    pub seed: u64,
    /// How many arms it ran; arm i is variant `v{i}`.
    pub arms: usize,
    /// Every arm's success probability.
    pub p: f64,
    /// Outcomes recorded when the rule concluded, if it did.
    pub concluded_at: Option<u64>,
    /// The winning variant, if any.
    pub winner: Option<String>,
}

impl LegacySequence {
    /// The captured outcome as `(concluded_at, winner)`.
    #[must_use]
    pub fn outcome(&self) -> Option<(u64, String)> {
        self.concluded_at.zip(self.winner.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The captured fixture.
    fn fixture() -> LegacyFixture {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/legacy_experiment_rule.json");
        let text = std::fs::read_to_string(path).expect("the legacy-rule fixture");
        serde_json::from_str(&text).expect("parse the legacy-rule fixture")
    }

    /// Backlog 5109: `LegacyRule` reproduces every A/A sequence the live
    /// `PromptExperiment` produced, winner and conclusion trial alike, and
    /// the rule's A/A false-winner rate is printed for C4's contrast.
    #[test]
    fn legacy_rule_matches_prompt_experiment() {
        let fixture = fixture();
        assert_eq!(fixture.sequences.len(), 1_000);
        let mut winners = 0_u32;
        for sequence in &fixture.sequences {
            let run = legacy_aa_run(sequence.seed, sequence.arms, sequence.p, fixture.horizon);
            let got = run.map(|(trial, arm)| (trial, format!("v{arm}")));
            assert_eq!(got, sequence.outcome(), "seed {}", sequence.seed);
            winners += u32::from(got.is_some());
        }
        let rate = f64::from(winners) / fixture.sequences.len() as f64;
        println!(
            "legacy experiment rule: A/A false-winner rate {rate:.3} over {} sequences",
            fixture.sequences.len()
        );
        assert!((rate - fixture.false_winner_rate).abs() < 1e-12);
    }

    /// The splitmix64 reference values for seed 0.
    #[test]
    fn splitmix64_matches_its_reference() {
        let mut rng = SplitMix64::new(0);
        assert_eq!(rng.next_u64(), 0xE220_A839_7B1D_CDAF);
        assert_eq!(rng.next_u64(), 0x6E78_9E6A_A1B9_65F4);
    }
}
