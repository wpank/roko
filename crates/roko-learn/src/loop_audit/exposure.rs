//! Exposure (ε) and net-influence (ι) estimators with the A/A floor (S03 §4.4; backlog 5112).
//!
//! Per opportunity t of a loop: its arm and logged propensity, the learned
//! proposal A^L_t and the default A^D_t (both computed on both arms), a
//! second independent draw A^L'_t of a stochastic learned policy, the
//! executed action A_t with its label, what the reader loaded, and whether
//! the exposure receipt is present. [`Opportunity`] holds those, whatever
//! file they come from (the row adapter is backlog 5123's).
//!
//! - **Exposure** `E_t = read_t · 1[A_t ≡ A^L_t] · honest_t · receipt_t` on
//!   learned-arm opportunities, decomposed into ε_read (state loaded,
//!   non-empty, current) · ε_reach (executed equals learned) · ε_honest (the
//!   label matches the provider record; a masked decision fails it) ·
//!   ε_receipt. A failing decomposition maps to S03 §4.6's
//!   `dormant:{write_only, cut, stale, mask, unlogged}`.
//! - **Influence** `I_t = d(A^L_t, A^D_t)` (1[≠], or the Jaccard distance for
//!   sets), with the A/A floor `I^AA_t = d(A^L_t, A^L'_t)` (0 for a
//!   deterministic reader) and `ι_net = mean(I − I^AA)`. Never report ι
//!   without the floor: identical reruns flip 5.37% of decisions
//!   (`lee2026most`). `ι_cite`, the share of exposures whose patch cites an
//!   injected id, is a cheap proxy.
//!
//! Both carry a confidence sequence ([`super::cs::BettingCs`]): ε on
//! `[0, 1]`, ι_net on `[−1, 1]`.

use std::collections::BTreeSet;

use super::cs::BettingCs;
use super::spec::ReasonCode;

/// An executed or proposed action, with the distance influence uses.
pub trait Action: PartialEq {
    /// `d(self, other)` in `[0, 1]`: 0 for the same action.
    fn distance(&self, other: &Self) -> f64;
}

impl Action for String {
    /// 1[≠]: a model, a threshold label, a variant id.
    fn distance(&self, other: &Self) -> f64 {
        if self == other { 0.0 } else { 1.0 }
    }
}

impl Action for BTreeSet<String> {
    /// The Jaccard distance `1 − |A ∩ B| / |A ∪ B|`, 0 for two empty sets:
    /// included knowledge ids, playbooks, sections.
    fn distance(&self, other: &Self) -> f64 {
        let union = self.union(other).count();
        if union == 0 {
            return 0.0;
        }
        let shared = self.intersection(other).count();
        1.0 - shared as f64 / union as f64
    }
}

/// What the decision's reader loaded of the loop's learned state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadStatus {
    /// Loaded, non-empty and current.
    Loaded,
    /// Never loaded on the decision path (`dormant:write_only`).
    Missing,
    /// Loaded but empty (`dormant:cut`).
    Empty,
    /// Loaded at an older state version than the current one
    /// (`dormant:stale`).
    Stale,
}

/// One opportunity of a loop (S03 §4.4).
#[derive(Debug, Clone, PartialEq)]
pub struct Opportunity<A> {
    /// The unit drew the learned arm (Z_t = L).
    pub learned_arm: bool,
    /// The logged probability of the drawn arm.
    pub propensity: f64,
    /// The learned proposal A^L_t.
    pub learned: A,
    /// The default proposal A^D_t.
    pub default: A,
    /// A second, independent draw A^L'_t of a stochastic learned policy;
    /// `None` for a deterministic reader.
    pub learned_again: Option<A>,
    /// The executed action A_t, as the decision record labels it.
    pub executed: A,
    /// The label matches the provider's record of what ran.
    pub honest: bool,
    /// What the reader loaded.
    pub read: ReadStatus,
    /// The exposure receipt is present.
    pub receipt: bool,
    /// The patch or transcript cites an injected id or n-gram (ι_cite);
    /// `None` when unchecked.
    pub cites: Option<bool>,
}

/// ε and its decomposition over the learned-arm opportunities so far.
#[derive(Debug, Clone, PartialEq)]
pub struct ExposureEstimate {
    /// Learned-arm opportunities.
    pub opportunities: u64,
    /// ε̂ = mean(E_t).
    pub epsilon: f64,
    /// ε_read: the state was loaded, non-empty and current.
    pub read: f64,
    /// ε_reach: the executed action equals the learned one, given a read.
    pub reach: f64,
    /// ε_honest: the label matches the provider record, given a reach.
    pub honest: f64,
    /// ε_receipt: the receipt is present, given an honest reach.
    pub receipt: f64,
    /// The confidence sequence's interval for ε; `None` before the first
    /// opportunity.
    pub interval: Option<(f64, f64)>,
    /// The read failures: never loaded, empty, stale.
    pub read_failures: (u64, u64, u64),
}

impl ExposureEstimate {
    /// The dormant reason when exposure stays below `epsilon_min` (the
    /// interval's upper end, else ε̂): the code of the decomposition stage
    /// that loses the most opportunities, the earlier stage on a tie. A read
    /// that never loads is `write_only`, an empty one `cut`, an old one
    /// `stale`; a reach or honesty failure is `mask`, a missing receipt
    /// `unlogged`. `None` while exposure may still reach `epsilon_min`.
    #[must_use]
    pub fn dormant_reason(&self, epsilon_min: f64) -> Option<ReasonCode> {
        let upper = self.interval.map_or(self.epsilon, |(_, high)| high);
        if self.opportunities == 0 || upper >= epsilon_min {
            return None;
        }
        let stages = [
            (self.read, self.read_reason()),
            (self.reach, ReasonCode::Mask),
            (self.honest, ReasonCode::Mask),
            (self.receipt, ReasonCode::Unlogged),
        ];
        let mut worst = stages[0];
        for stage in &stages[1..] {
            if stage.0 < worst.0 {
                worst = *stage;
            }
        }
        Some(worst.1)
    }

    /// The reason of the most frequent read failure; precedence on a tie.
    fn read_reason(&self) -> ReasonCode {
        let (missing, empty, stale) = self.read_failures;
        let failures = [
            (missing, ReasonCode::WriteOnly),
            (empty, ReasonCode::Cut),
            (stale, ReasonCode::Stale),
        ];
        let mut worst = failures[0];
        for failure in &failures[1..] {
            if failure.0 > worst.0 {
                worst = *failure;
            }
        }
        worst.1
    }
}

/// Accumulates exposure over a loop's opportunities.
#[derive(Debug, Clone)]
pub struct ExposureEstimator {
    opportunities: u64,
    read_failures: (u64, u64, u64),
    read: u64,
    reach: u64,
    honest: u64,
    exposed: u64,
    cs: BettingCs,
}

impl ExposureEstimator {
    /// An estimator whose confidence sequence has level `alpha`.
    #[must_use]
    pub fn new(alpha: f64) -> Self {
        Self {
            opportunities: 0,
            read_failures: (0, 0, 0),
            read: 0,
            reach: 0,
            honest: 0,
            exposed: 0,
            cs: BettingCs::new(alpha, 0.0, 1.0),
        }
    }

    /// Take one opportunity; a default-arm opportunity carries no exposure
    /// and is skipped.
    pub fn push<A: Action>(&mut self, opportunity: &Opportunity<A>) {
        if !opportunity.learned_arm {
            return;
        }
        self.opportunities += 1;
        match opportunity.read {
            ReadStatus::Loaded => {}
            ReadStatus::Missing => self.read_failures.0 += 1,
            ReadStatus::Empty => self.read_failures.1 += 1,
            ReadStatus::Stale => self.read_failures.2 += 1,
        }
        let read = opportunity.read == ReadStatus::Loaded;
        let reach = read && opportunity.executed.distance(&opportunity.learned) == 0.0;
        let honest = reach && opportunity.honest;
        let exposed = honest && opportunity.receipt;
        self.read += u64::from(read);
        self.reach += u64::from(reach);
        self.honest += u64::from(honest);
        self.exposed += u64::from(exposed);
        self.cs.push(f64::from(u8::from(exposed)), 1.0);
    }

    /// ε and its decomposition so far.
    #[must_use]
    pub fn estimate(&self) -> ExposureEstimate {
        ExposureEstimate {
            opportunities: self.opportunities,
            epsilon: share(self.exposed, self.opportunities),
            read: share(self.read, self.opportunities),
            reach: share(self.reach, self.read),
            honest: share(self.honest, self.reach),
            receipt: share(self.exposed, self.honest),
            interval: (self.opportunities > 0)
                .then(|| self.cs.interval())
                .flatten(),
            read_failures: self.read_failures,
        }
    }
}

/// ι, its A/A floor and ι_net over the opportunities so far.
#[derive(Debug, Clone, PartialEq)]
pub struct InfluenceEstimate {
    /// Opportunities, on both arms.
    pub opportunities: u64,
    /// mean(I_t) = mean d(A^L, A^D).
    pub iota: f64,
    /// The A/A floor mean(I^AA_t) = mean d(A^L, A^L').
    pub aa_floor: f64,
    /// ι_net = ι − the floor.
    pub iota_net: f64,
    /// The confidence sequence's interval for ι_net; `None` before the
    /// first opportunity.
    pub interval: Option<(f64, f64)>,
    /// ι_cite: the share of checked exposures that cite an injected id;
    /// `None` when none was checked.
    pub iota_cite: Option<f64>,
}

/// Accumulates net influence over a loop's opportunities.
#[derive(Debug, Clone)]
pub struct InfluenceEstimator {
    opportunities: u64,
    influence: f64,
    floor: f64,
    checked: u64,
    cited: u64,
    cs: BettingCs,
}

impl InfluenceEstimator {
    /// An estimator whose confidence sequence has level `alpha`.
    #[must_use]
    pub fn new(alpha: f64) -> Self {
        Self {
            opportunities: 0,
            influence: 0.0,
            floor: 0.0,
            checked: 0,
            cited: 0,
            cs: BettingCs::new(alpha, -1.0, 1.0),
        }
    }

    /// Take one opportunity, on either arm.
    pub fn push<A: Action>(&mut self, opportunity: &Opportunity<A>) {
        let influence = opportunity.learned.distance(&opportunity.default);
        let floor = opportunity
            .learned_again
            .as_ref()
            .map_or(0.0, |again| opportunity.learned.distance(again));
        self.opportunities += 1;
        self.influence += influence;
        self.floor += floor;
        if let Some(cites) = opportunity.cites {
            self.checked += 1;
            self.cited += u64::from(cites);
        }
        self.cs.push(influence - floor, 1.0);
    }

    /// ι, the floor and ι_net so far.
    #[must_use]
    pub fn estimate(&self) -> InfluenceEstimate {
        let n = self.opportunities.max(1) as f64;
        let (iota, aa_floor) = (self.influence / n, self.floor / n);
        InfluenceEstimate {
            opportunities: self.opportunities,
            iota,
            aa_floor,
            iota_net: iota - aa_floor,
            interval: (self.opportunities > 0)
                .then(|| self.cs.interval())
                .flatten(),
            iota_cite: (self.checked > 0).then(|| share(self.cited, self.checked)),
        }
    }
}

/// `part / whole`, 0 when `whole` is 0.
fn share(part: u64, whole: u64) -> f64 {
    if whole == 0 {
        0.0
    } else {
        part as f64 / whole as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A learned-arm routing opportunity whose reader loaded current state,
    /// executed the learned model honestly and left its receipt.
    fn exposed() -> Opportunity<String> {
        Opportunity {
            learned_arm: true,
            propensity: 0.8,
            learned: "model-l".to_string(),
            default: "model-d".to_string(),
            learned_again: None,
            executed: "model-l".to_string(),
            honest: true,
            read: ReadStatus::Loaded,
            receipt: true,
            cites: None,
        }
    }

    /// ε over `opportunities`.
    fn exposure(opportunities: &[Opportunity<String>]) -> ExposureEstimate {
        let mut estimator = ExposureEstimator::new(0.05);
        for opportunity in opportunities {
            estimator.push(opportunity);
        }
        estimator.estimate()
    }

    /// S03 §4.4: ε is the product ε_read · ε_reach · ε_honest · ε_receipt
    /// over learned-arm opportunities, default-arm ones are skipped, and each
    /// failing stage maps to its §4.6 reason: an empty read is `cut`, a
    /// masked label fails ε_honest (`mask`), a dropped receipt is
    /// `unlogged`, a state never loaded is `write_only`.
    #[test]
    fn exposure_decomposes_into_read_reach_honest_receipt() {
        let mut mixed = Vec::new();
        for i in 0..100 {
            let mut opportunity = exposed();
            match i % 10 {
                0 => opportunity.read = ReadStatus::Empty,
                1 => opportunity.executed = "model-d".to_string(),
                2 => opportunity.honest = false,
                3 => opportunity.receipt = false,
                _ => {}
            }
            mixed.push(opportunity);
        }
        let mut default_arm = exposed();
        default_arm.learned_arm = false;
        default_arm.read = ReadStatus::Missing;
        mixed.push(default_arm);
        let estimate = exposure(&mixed);
        assert_eq!(estimate.opportunities, 100, "the default arm is skipped");
        let product = estimate.read * estimate.reach * estimate.honest * estimate.receipt;
        assert!((estimate.epsilon - product).abs() < 1e-12, "{estimate:?}");
        assert!((estimate.epsilon - 0.6).abs() < 1e-12, "{estimate:?}");
        assert!((estimate.read - 0.9).abs() < 1e-12);

        let all = |change: fn(&mut Opportunity<String>)| {
            let opportunities: Vec<Opportunity<String>> = (0..200)
                .map(|_| {
                    let mut opportunity = exposed();
                    change(&mut opportunity);
                    opportunity
                })
                .collect();
            exposure(&opportunities)
        };
        let cut = all(|o| o.read = ReadStatus::Empty);
        assert_eq!(cut.read, 0.0);
        assert_eq!(cut.dormant_reason(0.5), Some(ReasonCode::Cut));
        let masked = all(|o| o.honest = false);
        assert_eq!((masked.read, masked.reach, masked.honest), (1.0, 1.0, 0.0));
        assert_eq!(masked.dormant_reason(0.5), Some(ReasonCode::Mask));
        let unlogged = all(|o| o.receipt = false);
        assert_eq!(unlogged.dormant_reason(0.5), Some(ReasonCode::Unlogged));
        let write_only = all(|o| o.read = ReadStatus::Missing);
        assert_eq!(write_only.dormant_reason(0.5), Some(ReasonCode::WriteOnly));
        let stale = all(|o| o.read = ReadStatus::Stale);
        assert_eq!(stale.dormant_reason(0.5), Some(ReasonCode::Stale));
        let live = all(|_| {});
        assert_eq!(live.epsilon, 1.0);
        assert_eq!(live.dormant_reason(0.5), None);
        let (low, high) = live.interval.expect("an interval");
        assert!(low > 0.5 && high <= 1.0, "[{low}, {high}]");
    }

    /// S03 §4.4: ι_net subtracts the A/A floor of a stochastic reader from
    /// its influence, a deterministic reader has no floor, a reader whose
    /// learned draws differ as often as learned and default has ι_net ≈ 0,
    /// and sets use the Jaccard distance.
    #[test]
    fn net_influence_subtracts_aa_floor() {
        let influence = |opportunities: &[Opportunity<String>]| {
            let mut estimator = InfluenceEstimator::new(0.05);
            for opportunity in opportunities {
                estimator.push(opportunity);
            }
            estimator.estimate()
        };
        // 30 of 100 learned proposals differ from the default; a second draw
        // differs from the first on 10.
        let stochastic: Vec<Opportunity<String>> = (0..100)
            .map(|i| {
                let mut opportunity = exposed();
                if i >= 30 {
                    opportunity.default = opportunity.learned.clone();
                }
                let again = if i % 10 == 0 { "model-x" } else { "model-l" };
                opportunity.learned_again = Some(again.to_string());
                opportunity
            })
            .collect();
        let estimate = influence(&stochastic);
        assert!((estimate.iota - 0.3).abs() < 1e-12, "{estimate:?}");
        assert!((estimate.aa_floor - 0.1).abs() < 1e-12, "{estimate:?}");
        assert!((estimate.iota_net - 0.2).abs() < 1e-12, "{estimate:?}");

        let deterministic: Vec<Opportunity<String>> = stochastic
            .iter()
            .cloned()
            .map(|mut opportunity| {
                opportunity.learned_again = None;
                opportunity
            })
            .collect();
        let estimate = influence(&deterministic);
        assert_eq!(estimate.aa_floor, 0.0);
        assert!((estimate.iota_net - 0.3).abs() < 1e-12);

        // A reader as noisy against itself as against the default: the
        // floor absorbs all of its influence.
        let noise: Vec<Opportunity<String>> = (0..400)
            .map(|i| {
                let mut opportunity = exposed();
                if i % 2 == 0 {
                    opportunity.default = opportunity.learned.clone();
                    opportunity.learned_again = Some("model-x".to_string());
                } else {
                    opportunity.learned_again = Some(opportunity.learned.clone());
                }
                opportunity
            })
            .collect();
        let estimate = influence(&noise);
        assert!(estimate.iota_net.abs() < 1e-12, "{estimate:?}");
        let (low, high) = estimate.interval.expect("an interval");
        assert!(low <= 0.0 && 0.0 <= high, "[{low}, {high}]");

        let set = |ids: &[&str]| ids.iter().map(|id| (*id).to_string()).collect::<BTreeSet<_>>();
        assert!((set(&["a", "b"]).distance(&set(&["b", "c"])) - 2.0 / 3.0).abs() < 1e-12);
        assert_eq!(set(&[]).distance(&set(&[])), 0.0);
    }
}
