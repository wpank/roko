//! Forecast scores: Brier, Brier skill, ECE over equal-mass bins, calibration-in-the-large,
//! AUROC, the Murphy decomposition and reliability bins: backlog task 6111.
//!
//! Pure functions over weighted forecasts ([`Scored`]): a probability, its 0/1 outcome and an
//! importance weight, 1/π for an audit-only label (S05 DP5). A forecast of weight w counts like
//! w copies of it. [`mean_square`] is the crate's one Brier formula:
//! [`CalibrationTracker::brier_score`](crate::prediction::CalibrationTracker::brier_score) calls
//! it too.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// The number of equal-mass bins ECE and the reliability diagram use (S04 §4.7).
pub const ECE_BINS: usize = 10;

/// One forecast and its outcome.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Scored {
    /// The forecast probability of the event.
    pub p: f64,
    /// Whether the event happened.
    pub y: bool,
    /// The importance weight.
    pub w: f64,
}

impl Scored {
    /// A forecast of weight 1.
    #[must_use]
    pub const fn new(p: f64, y: bool) -> Self {
        Self { p, y, w: 1.0 }
    }

    /// A forecast of weight `w`.
    #[must_use]
    pub const fn weighted(p: f64, y: bool, w: f64) -> Self {
        Self { p, y, w }
    }

    /// The outcome as 0 or 1.
    const fn outcome(self) -> f64 {
        if self.y { 1.0 } else { 0.0 }
    }
}

/// The weighted mean of squared errors, from `(error, weight)` pairs: the formula behind every
/// Brier score. `None` when the weights sum to nothing.
pub fn mean_square(errors: impl IntoIterator<Item = (f64, f64)>) -> Option<f64> {
    let (sum, weight) = errors
        .into_iter()
        .fold((0.0, 0.0), |(sum, weight), (error, w)| {
            (sum + w * error * error, weight + w)
        });
    (weight > 0.0).then_some(sum / weight)
}

/// The weights' sum.
fn total_weight(forecasts: &[Scored]) -> f64 {
    forecasts.iter().map(|forecast| forecast.w).sum()
}

/// The Brier score: the weighted mean of (p − y)².
pub fn brier(forecasts: &[Scored]) -> Option<f64> {
    mean_square(
        forecasts
            .iter()
            .map(|forecast| (forecast.p - forecast.outcome(), forecast.w)),
    )
}

/// The weighted rate of the event.
pub fn base_rate(forecasts: &[Scored]) -> Option<f64> {
    let weight = total_weight(forecasts);
    let events: f64 = forecasts
        .iter()
        .map(|forecast| forecast.w * forecast.outcome())
        .sum();
    (weight > 0.0).then_some(events / weight)
}

/// The Brier skill score against the base rate: 1 − Brier / (ȳ(1 − ȳ)). `None` when every
/// outcome is the same, so no forecast can show skill.
pub fn brier_skill(forecasts: &[Scored]) -> Option<f64> {
    let brier = brier(forecasts)?;
    let rate = base_rate(forecasts)?;
    let uncertainty = rate * (1.0 - rate);
    (uncertainty > 0.0).then_some(1.0 - brier / uncertainty)
}

/// Calibration-in-the-large: the mean forecast minus the base rate.
pub fn calibration_in_the_large(forecasts: &[Scored]) -> Option<f64> {
    let weight = total_weight(forecasts);
    if weight <= 0.0 {
        return None;
    }
    let mean_p = forecasts
        .iter()
        .map(|forecast| forecast.w * forecast.p)
        .sum::<f64>()
        / weight;
    Some(mean_p - base_rate(forecasts)?)
}

/// One bin of a reliability diagram.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct ReliabilityBin {
    /// The bin's mean forecast.
    pub mean_p: f64,
    /// The bin's event rate.
    pub rate: f64,
    /// The bin's total weight.
    pub weight: f64,
    /// The forecasts in the bin.
    pub count: usize,
}

/// The forecasts sorted by probability and cut into `bins` bins of equal weight. A bin closes
/// once it reaches its share of the total weight, so ties may leave a bin heavier.
pub fn reliability_bins(forecasts: &[Scored], bins: usize) -> Vec<ReliabilityBin> {
    let mut sorted: Vec<&Scored> = forecasts
        .iter()
        .filter(|forecast| forecast.w > 0.0)
        .collect();
    sorted.sort_by(|a, b| a.p.total_cmp(&b.p));
    let total: f64 = sorted.iter().map(|forecast| forecast.w).sum();
    if total <= 0.0 || bins == 0 {
        return Vec::new();
    }
    let finish = |(sum_p, sum_y, weight, count): (f64, f64, f64, usize)| ReliabilityBin {
        mean_p: sum_p / weight,
        rate: sum_y / weight,
        weight,
        count,
    };
    let mut out = Vec::with_capacity(bins);
    let mut current = (0.0, 0.0, 0.0, 0_usize);
    let mut closed = 1;
    let mut seen = 0.0;
    for forecast in sorted {
        current.0 += forecast.w * forecast.p;
        current.1 += forecast.w * forecast.outcome();
        current.2 += forecast.w;
        current.3 += 1;
        seen += forecast.w;
        if closed < bins && seen >= total * closed as f64 / bins as f64 - 1e-12 {
            out.push(finish(current));
            current = (0.0, 0.0, 0.0, 0);
            closed += 1;
        }
    }
    if current.3 > 0 {
        out.push(finish(current));
    }
    out
}

/// The expected calibration error over `bins` equal-mass bins: Σ_b (W_b/W)·|ȳ_b − p̄_b|.
pub fn ece(forecasts: &[Scored], bins: usize) -> Option<f64> {
    let bins = reliability_bins(forecasts, bins);
    let total: f64 = bins.iter().map(|bin| bin.weight).sum();
    let gap: f64 = bins
        .iter()
        .map(|bin| bin.weight * (bin.rate - bin.mean_p).abs())
        .sum();
    (total > 0.0).then_some(gap / total)
}

/// The Murphy decomposition of the Brier score.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Murphy {
    /// How far each forecast value is from its event rate (lower is better).
    pub reliability: f64,
    /// How far the forecast values' event rates spread from the base rate (higher is better).
    pub resolution: f64,
    /// The base rate's own variance, ȳ(1 − ȳ).
    pub uncertainty: f64,
}

impl Murphy {
    /// Reliability − resolution + uncertainty: the Brier score.
    #[must_use]
    pub fn brier(&self) -> f64 {
        self.reliability - self.resolution + self.uncertainty
    }
}

/// The Murphy decomposition over the distinct forecast values, so that its parts sum to
/// [`brier`] exactly.
pub fn murphy(forecasts: &[Scored]) -> Option<Murphy> {
    let total = total_weight(forecasts);
    let rate = base_rate(forecasts)?;
    let mut values: BTreeMap<u64, (f64, f64, f64)> = BTreeMap::new();
    for forecast in forecasts {
        let value = values
            .entry(forecast.p.to_bits())
            .or_insert((forecast.p, 0.0, 0.0));
        value.1 += forecast.w;
        value.2 += forecast.w * forecast.outcome();
    }
    let (mut reliability, mut resolution) = (0.0, 0.0);
    for (p, weight, events) in values.into_values() {
        if weight <= 0.0 {
            continue;
        }
        let value_rate = events / weight;
        reliability += weight / total * (p - value_rate).powi(2);
        resolution += weight / total * (value_rate - rate).powi(2);
    }
    Some(Murphy {
        reliability,
        resolution,
        uncertainty: rate * (1.0 - rate),
    })
}

/// The area under the ROC curve: the weighted chance that an event's forecast is higher than
/// a non-event's, a tie counting half. `None` without both outcomes.
pub fn auroc(forecasts: &[Scored]) -> Option<f64> {
    let mut sorted: Vec<&Scored> = forecasts
        .iter()
        .filter(|forecast| forecast.w > 0.0)
        .collect();
    sorted.sort_by(|a, b| a.p.total_cmp(&b.p));
    let (mut negatives_below, mut positives, mut concordant) = (0.0, 0.0, 0.0);
    for tie in sorted.chunk_by(|a, b| a.p == b.p) {
        let positive: f64 = tie
            .iter()
            .filter(|forecast| forecast.y)
            .map(|forecast| forecast.w)
            .sum();
        let negative: f64 = tie
            .iter()
            .filter(|forecast| !forecast.y)
            .map(|forecast| forecast.w)
            .sum();
        concordant += positive * (negatives_below + 0.5 * negative);
        negatives_below += negative;
        positives += positive;
    }
    (positives > 0.0 && negatives_below > 0.0).then_some(concordant / (positives * negatives_below))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::self_model::TestRng;

    /// Five forecasts whose scores are worked by hand below.
    fn five() -> Vec<Scored> {
        vec![
            Scored::new(0.9, true),
            Scored::new(0.7, true),
            Scored::new(0.6, false),
            Scored::new(0.2, false),
            Scored::new(0.4, true),
        ]
    }

    #[test]
    fn murphy_decomposition_matches_brier() {
        let golden = [
            Scored::new(0.1, false),
            Scored::new(0.1, true),
            Scored::weighted(0.1, false, 2.0),
            Scored::new(0.4, true),
            Scored::new(0.4, false),
            Scored::weighted(0.8, true, 3.0),
            Scored::new(0.8, false),
            Scored::new(0.95, true),
        ];
        let murphy = murphy(&golden).expect("decomposition");
        let brier = brier(&golden).expect("brier");
        assert!(
            (murphy.brier() - brier).abs() < 1e-9,
            "{murphy:?} against {brier}"
        );
        assert!(murphy.reliability >= 0.0 && murphy.resolution >= 0.0);
        // Weight 11, six of it events: the base rate is 6/11.
        assert!((murphy.uncertainty - 30.0 / 121.0).abs() < 1e-12);
    }

    #[test]
    fn brier_skill_and_auroc_match_hand_computed_values() {
        let five = five();
        // (0.01 + 0.09 + 0.36 + 0.04 + 0.36) / 5.
        let brier = brier(&five).expect("brier");
        assert!((brier - 0.172).abs() < 1e-12, "{brier}");
        // Base rate 0.6, so uncertainty 0.24 and skill 1 − 0.172 / 0.24.
        let skill = brier_skill(&five).expect("skill");
        assert!((skill - (1.0 - 0.172 / 0.24)).abs() < 1e-12, "{skill}");
        // Of the six event/non-event pairs, only (0.4, 0.6) is out of order.
        let area = auroc(&five).expect("auroc");
        assert!((area - 5.0 / 6.0).abs() < 1e-12, "{area}");
        let tie = [Scored::new(0.5, true), Scored::new(0.5, false)];
        assert_eq!(auroc(&tie), Some(0.5), "a tie counts half");
        // Mean forecast 0.56 against a base rate of 0.6.
        let large = calibration_in_the_large(&five).expect("calibration-in-the-large");
        assert!((large + 0.04).abs() < 1e-12, "{large}");
        assert_eq!(brier_skill(&[Scored::new(0.3, true)]), None);
        assert_eq!(auroc(&[Scored::new(0.3, true)]), None);
    }

    #[test]
    fn ece_is_small_on_a_calibrated_stream() {
        let mut rng = TestRng::new(7);
        let stream: Vec<Scored> = (0..20_000)
            .map(|_| {
                let p = rng.uniform();
                Scored::new(p, rng.bernoulli(p))
            })
            .collect();
        let ece = ece(&stream, ECE_BINS).expect("ece");
        assert!(ece < 0.02, "{ece}");
        let bins = reliability_bins(&stream, ECE_BINS);
        assert_eq!(bins.len(), ECE_BINS);
        assert!(bins.iter().all(|bin| bin.count == 2_000), "{bins:?}");
    }

    #[test]
    fn weights_behave_as_importance_weights() {
        let weighted = [
            Scored::weighted(0.3, true, 2.0),
            Scored::new(0.8, false),
            Scored::new(0.6, true),
        ];
        let copied = [
            Scored::new(0.3, true),
            Scored::new(0.3, true),
            Scored::new(0.8, false),
            Scored::new(0.6, true),
        ];
        let close = |a: Option<f64>, b: Option<f64>| {
            let (a, b) = (a.expect("a score"), b.expect("a score"));
            assert!((a - b).abs() < 1e-12, "{a} against {b}");
        };
        close(brier(&weighted), brier(&copied));
        close(brier_skill(&weighted), brier_skill(&copied));
        close(auroc(&weighted), auroc(&copied));
        close(ece(&weighted, 2), ece(&copied, 2));
        close(
            calibration_in_the_large(&weighted),
            calibration_in_the_large(&copied),
        );
        let (a, b) = (murphy(&weighted), murphy(&copied));
        close(a.map(|m| m.reliability), b.map(|m| m.reliability));
        close(a.map(|m| m.resolution), b.map(|m| m.resolution));
    }
}
