//! Loop layers over S01's one assignment function (S03 §4.3).
//!
//! [`assign`](fn@crate::telemetry::assign) draws one unit on one layer with a
//! fixed holdout rate h and all-off rate g. This module adds what the loop
//! audit needs on top of it, with no second hash:
//!
//! - [`LoopLayer::layer_spec`]: a loop's layer for an epoch, with h from its
//!   registry entry and audit state ([`HoldoutSchedule`]): 0.20 on probation,
//!   0.05 once live and 0.50 while flagged or demoted, never below the floor
//!   of 0.02. L-M1's fixed 0.10 and the placebo's 0.5 replace the schedule,
//!   as does a bench campaign's fixed rate, and maximize mode
//!   (`--no-holdout`, D7) withholds nothing. S02.P1-14's `ArmSet` builds its
//!   layers here.
//! - [`assign_nested`]: a nested loop (`route.dream_bias`) is drawn only in
//!   its parent's learned arm, and otherwise takes π⁰ with its parent.
//! - The all-learning-off arm is the chain's draw on the `global` layer, so
//!   it switches every layer to π⁰ at once ([`takes_default`]), and each
//!   layer logs P(π⁰) = g + (1 − g)·h.
//! - The placebo, L-placebo, is defined once, in the registry: its own
//!   layer, a fixed h of 0.5 and two identical arms (receipt `sham`), so its
//!   true effect is 0.
//! - [`route_propensity`]: the one composed propensity P(a) of a route
//!   decision, over the all-off, holdout, nested and ε draws.

use std::iter;

use serde::{Deserialize, Serialize};

use super::spec::{AuditState, LoopSpec};
use crate::telemetry::{Arm, Assignment, AttemptKey, LayerSpec, assign};

/// ε of the per-attempt `route.explore` draw (D12, confirmed by decision
/// 2203): only in the route layer's learned arm, and only with no pin.
pub const EXPLORE_EPSILON: f64 = 0.05;
/// The cap on ε: only S06's ladder raises it (S02 SC7).
pub const EXPLORE_EPSILON_CAP: f64 = 0.10;
/// g, the all-learning-off rate (D10).
pub const GLOBAL_OFF_RATE: f64 = 0.03;
/// A bench campaign's fixed h, recorded in its prereg lock (S09, D10).
pub const CAMPAIGN_HOLDOUT: f64 = 0.10;
/// The per-attempt exploration layer of the route decision (S01 §4.6).
pub const ROUTE_EXPLORE_LAYER: &str = "route.explore";

/// S03 §4.3's holdout schedule (`[learning.audit] h`): h by audit state, and
/// the floor that no randomized loop goes below.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HoldoutSchedule {
    /// h on probation.
    pub probation: f64,
    /// h once live.
    pub live: f64,
    /// The floor h_min: never 0, so that every loop stays identifiable.
    pub min: f64,
    /// h_suspect, while flagged or demoted. The executed decision barely
    /// differs between the arms then, so the holdout costs almost nothing.
    pub suspect: f64,
}

impl HoldoutSchedule {
    /// S03 §4.10's defaults, confirmed with D10 (decision 5101).
    pub const S03: Self = Self {
        probation: 0.20,
        live: 0.05,
        min: 0.02,
        suspect: 0.50,
    };

    /// h for a loop in `state`, before the floor. A demoted loop keeps
    /// h_suspect too: while enforcement is off, half of its units see the
    /// harmful policy instead of most of them.
    #[must_use]
    pub const fn for_state(&self, state: AuditState) -> f64 {
        match state {
            AuditState::Probation => self.probation,
            AuditState::Live => self.live,
            AuditState::Flagged | AuditState::Demoted => self.suspect,
        }
    }
}

impl Default for HoldoutSchedule {
    fn default() -> Self {
        Self::S03
    }
}

/// Builds each loop's [`LayerSpec`] for an epoch: S03's schedule, the floor,
/// and the run-level overrides. S02.P1-14's `ArmSet` and the loop auditor
/// call [`Self::layer_spec`].
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct LoopLayer {
    /// h by audit state, and the floor.
    pub schedule: HoldoutSchedule,
    /// A bench campaign's fixed h ([`CAMPAIGN_HOLDOUT`]), which replaces the
    /// schedule for every loop without a fixed holdout of its own.
    pub campaign_h: Option<f64>,
    /// Maximize mode (`--no-holdout`, D7): g = h = 0, so nothing is
    /// withheld, though decisions are still logged.
    pub maximize: bool,
}

impl LoopLayer {
    /// The holdout rate h of `spec` in `state`.
    ///
    /// It is 0 in maximize mode and for a loop that is not randomized
    /// (observe-only or retired). Otherwise it is, in order of precedence,
    /// the loop's fixed holdout (L-M1, the placebo), a campaign's fixed rate,
    /// or the schedule's rate for `state`, and never below the floor.
    ///
    /// The all-learning-off draw is the run's, not the loop's: in that arm
    /// an observe-only loop takes π⁰ like every other loop (S03 §4.3).
    #[must_use]
    pub fn h(&self, spec: &LoopSpec, state: AuditState) -> f64 {
        if self.maximize || !spec.lifecycle.is_randomized() {
            return 0.0;
        }
        let h = spec
            .fixed_holdout
            .or(self.campaign_h)
            .unwrap_or_else(|| self.schedule.for_state(state));
        h.max(self.schedule.min).min(1.0)
    }

    /// `spec`'s layer for an epoch: where its arm is drawn (`route.dream_bias`
    /// for a loop nested in L-route), its unit, its h, and g, which maximize
    /// mode turns off.
    #[must_use]
    pub fn layer_spec(
        &self,
        spec: &LoopSpec,
        state: AuditState,
        epoch: &str,
        run_seed: u64,
        g: f64,
    ) -> LayerSpec {
        LayerSpec {
            run_seed,
            layer: spec.assignment_layer().to_string(),
            epoch: epoch.to_string(),
            unit: spec.unit,
            h: self.h(spec, state),
            g: if self.maximize { 0.0 } else { g },
        }
    }
}

/// Whether a loop in `arm` runs its default policy π⁰: in the default arm,
/// and in the all-learning-off arm, which switches every layer at once.
#[must_use]
pub const fn takes_default(arm: Arm) -> bool {
    matches!(arm, Arm::Default | Arm::GlobalOff)
}

/// Assign `key` on a nested loop's layer, `child`, inside its parent's
/// assignment (S03 §4.3, row 3).
///
/// The nested arm is drawn only in the parent's learned arm, and its
/// propensity multiplies in the parent's (S01 §4.6). In the parent's default
/// or all-off arm the nested loop takes π⁰ with its parent: it inherits the
/// parent's arm and propensity, and keeps its own layer and draw on record.
#[must_use]
pub fn assign_nested(parent: &Assignment, child: &LayerSpec, key: &AttemptKey) -> Assignment {
    let own = assign(child, key);
    if parent.arm != Arm::Learned {
        return Assignment {
            arm: parent.arm,
            propensity: parent.propensity,
            g: parent.g,
            ..own
        };
    }
    let (arm, conditional) = if own.u < own.h {
        (Arm::Default, own.h)
    } else {
        (Arm::Learned, 1.0 - own.h)
    };
    Assignment {
        arm,
        propensity: parent.propensity * conditional,
        g: parent.g,
        ..own
    }
}

/// Every combination of arms of the nested loops whose holdout rates are
/// `holdouts`, with its probability P(n) inside the parent's learned arm:
/// the n of [`route_propensity`]. One nested loop at h = 0.2 gives
/// `[Default]` at 0.2 and `[Learned]` at 0.8; no nested loop gives one empty
/// combination at probability 1.
#[must_use]
pub fn nested_arm_combinations(holdouts: &[f64]) -> Vec<(Vec<Arm>, f64)> {
    let mut combinations = vec![(Vec::new(), 1.0)];
    for &h in holdouts {
        let h = unit_interval(h);
        combinations = combinations
            .into_iter()
            .flat_map(|(arms, p)| {
                let mut held_out = arms.clone();
                held_out.push(Arm::Default);
                let mut learned = arms;
                learned.push(Arm::Learned);
                [(held_out, p * h), (learned, p * (1.0 - h))]
            })
            .collect();
    }
    combinations
}

/// The learned pick under one combination n of nested arms, with the
/// combination's probability P(n).
#[derive(Debug, Clone, PartialEq)]
pub struct NestedPick<T> {
    /// a^L(n): the learned policy's pick under the combination.
    pub pick: T,
    /// P(n), from [`nested_arm_combinations`].
    pub probability: f64,
}

/// The candidates of a route decision that is an opportunity.
#[derive(Debug, Clone, PartialEq)]
pub struct RouteDraw<T> {
    /// a⁰, π⁰'s pick: `default_slug` in L-route epochs, S04's static
    /// allocation in L-M3 epochs.
    pub default: T,
    /// E, the eligible set the ε draw is uniform over.
    pub eligible: Vec<T>,
    /// The learned pick under each combination of nested arms; their
    /// probabilities sum to 1. With no nested loop, the one learned pick at
    /// probability 1.
    pub nested: Vec<NestedPick<T>>,
}

/// One route decision, for its composed propensity (S03 §4.3, the single
/// table).
#[derive(Debug, Clone, PartialEq)]
pub enum RouteDecision<T> {
    /// A pin (`model_hint`, `force_backend`) is no opportunity: there is no
    /// draw, and the pinned model has P = 1 (source `override` or
    /// `task_hint`).
    Pinned(T),
    /// A fallback with no eligible candidate: P = 1 (source `fallback`).
    Fallback(T),
    /// An opportunity, drawn in order: all-off, holdout, the nested arms,
    /// then ε.
    Drawn(RouteDraw<T>),
}

impl<T: PartialEq> RouteDecision<T> {
    /// Every action the decision can take, once each: a⁰, the learned picks,
    /// then E. Their propensities sum to 1.
    #[must_use]
    pub fn candidates(&self) -> Vec<&T> {
        let draw = match self {
            Self::Pinned(choice) | Self::Fallback(choice) => return vec![choice],
            Self::Drawn(draw) => draw,
        };
        let picks = draw.nested.iter().map(|nested| &nested.pick);
        let mut candidates: Vec<&T> = Vec::new();
        for candidate in iter::once(&draw.default).chain(picks).chain(&draw.eligible) {
            if !candidates.contains(&candidate) {
                candidates.push(candidate);
            }
        }
        candidates
    }
}

/// P(a), the composed propensity of `candidate` in a route decision (S03
/// §4.3):
///
/// ```text
/// P(a) = g·1[a = a⁰] + (1 − g)·( h·1[a = a⁰]
///        + (1 − h)·( (1 − ε)·Σ_n P(n)·1[a = a^L(n)] + ε·1[a ∈ E]/k ) )
/// ```
///
/// with k = |E|, g the all-off rate, h the route layer's holdout rate and ε
/// the `route.explore` rate ([`EXPLORE_EPSILON`]). The propensities of the
/// decision's [`candidates`](RouteDecision::candidates) sum to 1. A pin or a
/// fallback is certain, and with E empty nothing is explored. Each rate is
/// clamped to `[0, 1]`, with NaN as 0, as `assign` clamps h and g.
#[must_use]
pub fn route_propensity<T: PartialEq>(
    g: f64,
    h: f64,
    eps: f64,
    decision: &RouteDecision<T>,
    candidate: &T,
) -> f64 {
    let draw = match decision {
        RouteDecision::Pinned(choice) | RouteDecision::Fallback(choice) => {
            return indicator(choice == candidate);
        }
        RouteDecision::Drawn(draw) => draw,
    };
    let (g, h) = (unit_interval(g), unit_interval(h));
    let k = distinct_count(&draw.eligible);
    let eps = if k == 0 { 0.0 } else { unit_interval(eps) };
    let is_default = indicator(draw.default == *candidate);
    let learned: f64 = draw
        .nested
        .iter()
        .filter(|nested| nested.pick == *candidate)
        .map(|nested| nested.probability)
        .sum();
    let explored = if draw.eligible.contains(candidate) {
        eps / k as f64
    } else {
        0.0
    };
    g * is_default + (1.0 - g) * (h * is_default + (1.0 - h) * ((1.0 - eps) * learned + explored))
}

fn indicator(condition: bool) -> f64 {
    if condition { 1.0 } else { 0.0 }
}

/// The number of distinct items in `items`.
fn distinct_count<T: PartialEq>(items: &[T]) -> usize {
    items
        .iter()
        .enumerate()
        .filter(|(index, item)| !items[..*index].contains(item))
        .count()
}

/// A probability clamped to `[0, 1]`, NaN counting as 0, as `assign` does.
fn unit_interval(p: f64) -> f64 {
    if p.is_nan() { 0.0 } else { p.clamp(0.0, 1.0) }
}

#[cfg(test)]
mod tests {
    use rand::{Rng, SeedableRng};
    use rand_chacha::ChaCha8Rng;

    use super::{
        CAMPAIGN_HOLDOUT, EXPLORE_EPSILON, EXPLORE_EPSILON_CAP, GLOBAL_OFF_RATE, HoldoutSchedule,
        LoopLayer, NestedPick, RouteDecision, RouteDraw, assign_nested, nested_arm_combinations,
        route_propensity, takes_default,
    };
    use crate::loop_audit::spec::{AuditState, Lifecycle, LoopSpec, ReceiptKind, Registry};
    use crate::telemetry::{Arm, AssignmentUnit, AttemptKey, LayerSpec, assign};

    const SEED: u64 = 1234;
    const EPOCH: &str = "2026-10-02";
    const NEXT_EPOCH: &str = "2026-10-03";
    const G: f64 = GLOBAL_OFF_RATE;

    fn registry() -> Registry {
        Registry::embedded().expect("embedded loop registry")
    }

    fn spec<'a>(registry: &'a Registry, id: &str) -> &'a LoopSpec {
        registry.get(id).expect("registered loop")
    }

    /// `id`'s layer in `state` under S03's defaults.
    fn layer_of(registry: &Registry, id: &str, state: AuditState) -> LayerSpec {
        LoopLayer::default().layer_spec(spec(registry, id), state, EPOCH, SEED, G)
    }

    fn chain(index: usize) -> AttemptKey {
        AttemptKey::new("gr-m2", "loop-audit", format!("t{index}"), 1)
    }

    /// The arms of chains `0..chains` on `layer`.
    fn arms_on(layer: &LayerSpec, chains: usize) -> Vec<Arm> {
        (0..chains)
            .map(|index| assign(layer, &chain(index)).arm)
            .collect()
    }

    /// Where `arm` goes in a `[global_off, default, learned]` tally.
    fn slot(arm: Arm) -> usize {
        match arm {
            Arm::GlobalOff => 0,
            Arm::Default => 1,
            Arm::Learned => 2,
            Arm::Explore => unreachable!("assign never draws explore"),
        }
    }

    /// Pearson's χ² of a 2×2 table, with one degree of freedom.
    fn chi_square_2x2(table: [[u32; 2]; 2]) -> f64 {
        let [[a, b], [c, d]] = table.map(|row| row.map(f64::from));
        (a + b + c + d) * (a * d - b * c).powi(2) / ((a + b) * (c + d) * (a + c) * (b + d))
    }

    #[test]
    fn assignment_uniform_sticky_and_independent_across_layers() {
        const CHAINS: usize = 100_000;
        // χ² at α = 0.001 with two degrees of freedom (a layer's arm shares)
        // and one (a pair of layers). This fixed sample scores at most 1.5
        // and 4.1.
        const CRITICAL_ARMS: f64 = 13.816;
        const CRITICAL_PAIR: f64 = 10.828;
        let registry = registry();
        // One loop per independent layer: five at the probation h of 0.2,
        // and the placebo at its fixed 0.5.
        let loops = [
            "L-route",
            "L-know",
            "L-play",
            "L-sec",
            "L-prompt-exp",
            "L-placebo",
        ];
        let specs: Vec<LayerSpec> = loops
            .iter()
            .map(|id| layer_of(&registry, id, AuditState::Probation))
            .collect();
        let names: Vec<&str> = specs.iter().map(|layer| layer.layer.as_str()).collect();
        let expected = [
            "route",
            "knowledge",
            "playbooks",
            "sections",
            "prompt_variant",
            "placebo",
        ];
        assert_eq!(names, expected);
        let arms: Vec<Vec<Arm>> = specs.iter().map(|layer| arms_on(layer, CHAINS)).collect();

        // Uniform: each layer's arm shares follow g and its h.
        for (layer, layer_arms) in specs.iter().zip(&arms) {
            let mut counts = [0_u32; 3];
            for &arm in layer_arms {
                counts[slot(arm)] += 1;
            }
            let (g, h) = (layer.g, layer.h);
            let shares = [g, (1.0 - g) * h, (1.0 - g) * (1.0 - h)];
            let chi_square: f64 = counts
                .iter()
                .zip(shares)
                .map(|(&count, share)| {
                    let expected = share * CHAINS as f64;
                    (f64::from(count) - expected).powi(2) / expected
                })
                .sum();
            let name = &layer.layer;
            assert!(chi_square < CRITICAL_ARMS, "{name}: {chi_square}");
        }

        // The all-off draw is the chain's, shared by every layer.
        let global_off: Vec<bool> = arms[0].iter().map(|&arm| arm == Arm::GlobalOff).collect();
        for layer_arms in &arms {
            let off = layer_arms.iter().map(|&arm| arm == Arm::GlobalOff);
            assert!(off.eq(global_off.iter().copied()));
        }

        // Independent: no pair of layers is associated outside that arm.
        let kept: Vec<usize> = (0..CHAINS).filter(|&index| !global_off[index]).collect();
        for a in 0..specs.len() {
            for b in a + 1..specs.len() {
                let mut table = [[0_u32; 2]; 2];
                for &index in &kept {
                    let row = usize::from(arms[a][index] == Arm::Default);
                    let column = usize::from(arms[b][index] == Arm::Default);
                    table[row][column] += 1;
                }
                let chi_square = chi_square_2x2(table);
                let pair = format!("{} × {}", names[a], names[b]);
                assert!(chi_square < CRITICAL_PAIR, "{pair}: {chi_square}");
            }
        }

        // Sticky: a retry stays in its chain's arm on every layer.
        for index in 0..1_000 {
            let retry = AttemptKey::new("gr-m2", "loop-audit", format!("t{index}"), 3);
            for (layer, layer_arms) in specs.iter().zip(&arms) {
                assert_eq!(assign(layer, &retry).arm, layer_arms[index]);
            }
        }

        // A new epoch redraws: P(change) = 1 − Σ p² over the three arms.
        let next = LayerSpec {
            epoch: NEXT_EPOCH.to_string(),
            ..specs[0].clone()
        };
        let changed = arms_on(&next, CHAINS)
            .iter()
            .zip(&arms[0])
            .filter(|(now, before)| now != before)
            .count();
        let shares = [G, (1.0 - G) * 0.2, (1.0 - G) * 0.8];
        let expected = 1.0 - shares.iter().map(|share| share * share).sum::<f64>();
        let share = changed as f64 / CHAINS as f64;
        assert!((share - expected).abs() < 0.01, "{share} vs {expected}");
    }

    #[test]
    fn composed_route_propensity_sums_to_one() {
        // A worked case: g = 0.03, h = 0.2, ε = 0.05 and four eligible
        // models; π⁰ picks a and the learned policy b.
        let worked = RouteDecision::Drawn(RouteDraw {
            default: "a",
            eligible: vec!["a", "b", "c", "d"],
            nested: vec![NestedPick {
                pick: "b",
                probability: 1.0,
            }],
        });
        let p = |candidate| route_propensity(0.03, 0.2, EXPLORE_EPSILON, &worked, &candidate);
        let explored = EXPLORE_EPSILON / 4.0;
        assert!((p("a") - (0.03 + 0.97 * (0.2 + 0.8 * explored))).abs() < 1e-12);
        assert!((p("b") - 0.97 * 0.8 * (1.0 - EXPLORE_EPSILON + explored)).abs() < 1e-12);
        assert!((p("c") - 0.97 * 0.8 * explored).abs() < 1e-12);
        assert_eq!(p("z"), 0.0);
        assert_eq!(worked.candidates(), [&"a", &"b", &"c", &"d"]);
        // Maximize mode (g = h = ε = 0) draws nothing: the learned pick runs.
        assert_eq!(route_propensity(0.0, 0.0, 0.0, &worked, &"b"), 1.0);

        // A pin or a fallback is certain.
        let certain = [RouteDecision::Pinned("m"), RouteDecision::Fallback("m")];
        for decision in certain {
            assert_eq!(route_propensity(0.03, 0.2, 0.05, &decision, &"m"), 1.0);
            assert_eq!(route_propensity(0.03, 0.2, 0.05, &decision, &"n"), 0.0);
            assert_eq!(decision.candidates(), [&"m"]);
        }

        // The nested-arm combinations and their probabilities.
        assert_eq!(nested_arm_combinations(&[]), [(Vec::new(), 1.0)]);
        let expected = [(vec![Arm::Default], 0.2), (vec![Arm::Learned], 0.8)];
        assert_eq!(nested_arm_combinations(&[0.2]), expected);

        // Random configurations: the candidates' propensities sum to 1.
        let models = ["m0", "m1", "m2", "m3", "m4", "m5"];
        let mut rng = ChaCha8Rng::seed_from_u64(2203);
        for _ in 0..10_000 {
            let (g, h) = (rng.gen_range(0.0..0.1), rng.gen_range(0.02..0.5));
            let eps = rng.gen_range(0.0..EXPLORE_EPSILON_CAP);
            let default = models[rng.gen_range(0..models.len())];
            let eligible: Vec<&str> = models.into_iter().filter(|_| rng.gen_bool(0.6)).collect();
            let holdouts: Vec<f64> = (0..rng.gen_range(0..3))
                .map(|_| rng.gen_range(0.02..0.5))
                .collect();
            let nested = nested_arm_combinations(&holdouts)
                .into_iter()
                .map(|(_, probability)| NestedPick {
                    pick: models[rng.gen_range(0..models.len())],
                    probability,
                })
                .collect();
            let decision = RouteDecision::Drawn(RouteDraw {
                default,
                eligible,
                nested,
            });
            let total: f64 = decision
                .candidates()
                .into_iter()
                .map(|candidate| route_propensity(g, h, eps, &decision, candidate))
                .sum();
            assert!((total - 1.0).abs() < 1e-9, "{decision:?}: {total}");
        }
    }

    #[test]
    fn nested_loop_draws_only_inside_the_learned_parent() {
        const CHAINS: usize = 20_000;
        let registry = registry();
        let route = layer_of(&registry, "L-route", AuditState::Probation);
        let bias = layer_of(&registry, "L-dream-bias", AuditState::Probation);
        assert_eq!(bias.layer, "route.dream_bias");
        let (mut learned_parents, mut held_out) = (0_u32, 0_u32);
        for index in 0..CHAINS {
            let key = chain(index);
            let parent = assign(&route, &key);
            let nested = assign_nested(&parent, &bias, &key);
            assert_eq!(nested.layer, "route.dream_bias");
            assert_eq!(nested.unit, AssignmentUnit::Chain);
            if parent.arm != Arm::Learned {
                // π⁰ with the parent: its arm and its propensity.
                assert_eq!(nested.arm, parent.arm);
                assert_eq!(nested.propensity, parent.propensity);
                continue;
            }
            learned_parents += 1;
            let conditional = if nested.arm == Arm::Default {
                held_out += 1;
                bias.h
            } else {
                assert_eq!(nested.arm, Arm::Learned);
                1.0 - bias.h
            };
            assert!((nested.propensity - parent.propensity * conditional).abs() < 1e-12);
        }
        let share = f64::from(held_out) / f64::from(learned_parents);
        assert!((share - bias.h).abs() < 0.02, "nested default share {share}");
    }

    #[test]
    fn all_off_arm_takes_default_on_every_layer() {
        const CHAINS: usize = 20_000;
        let registry = registry();
        let layers = LoopLayer::default();
        // Every randomized loop that is not nested, at its probation or
        // fixed h.
        let specs: Vec<LayerSpec> = registry
            .loops()
            .iter()
            .filter(|entry| entry.lifecycle.is_randomized() && entry.nested_in.is_none())
            .map(|entry| layers.layer_spec(entry, AuditState::Probation, EPOCH, SEED, G))
            .collect();
        let mut all_off = 0_u32;
        for index in 0..CHAINS {
            let key = chain(index);
            let assignments: Vec<_> = specs.iter().map(|layer| assign(layer, &key)).collect();
            let off = assignments[0].arm == Arm::GlobalOff;
            all_off += u32::from(off);
            for (layer, assignment) in specs.iter().zip(&assignments) {
                // Every layer logs P(π⁰) = g + (1 − g)·h.
                let logged = assignment.default_policy_probability();
                assert!((logged - (layer.g + (1.0 - layer.g) * layer.h)).abs() < 1e-12);
                let arm = assignment.arm;
                assert_eq!(arm == Arm::GlobalOff, off, "{}", layer.layer);
                assert_eq!(takes_default(arm), arm != Arm::Learned);
                if off {
                    assert_eq!(assignment.propensity, layer.g);
                }
            }
        }
        let share = f64::from(all_off) / CHAINS as f64;
        assert!((share - G).abs() < 0.005, "all-off share {share}");
    }

    #[test]
    fn holdout_schedule_follows_audit_state_with_floor_and_overrides() {
        use AuditState::{Demoted, Flagged, Live, Probation};

        let registry = registry();
        let know = spec(&registry, "L-know");
        let layers = LoopLayer::default();
        let schedule = [
            (Probation, 0.20),
            (Live, 0.05),
            (Flagged, 0.50),
            (Demoted, 0.50),
        ];
        for (state, h) in schedule {
            assert_eq!(layers.h(know, state), h, "{state:?}");
        }

        // The floor: no schedule takes a randomized loop below h_min.
        let low = LoopLayer {
            schedule: HoldoutSchedule {
                live: 0.01,
                ..HoldoutSchedule::S03
            },
            ..LoopLayer::default()
        };
        assert_eq!(low.h(know, Live), 0.02);

        // L-M1's static 10% and the placebo's 0.5 replace the schedule and
        // win over a campaign's rate, which replaces it for the other loops.
        // Observe-only loops are never randomized.
        let campaign = LoopLayer {
            campaign_h: Some(CAMPAIGN_HOLDOUT),
            ..LoopLayer::default()
        };
        let (m1, placebo) = (spec(&registry, "L-M1"), spec(&registry, "L-placebo"));
        let observe_only: Vec<&LoopSpec> = registry
            .loops()
            .iter()
            .filter(|entry| entry.lifecycle == Lifecycle::ObserveOnly)
            .collect();
        assert_eq!(observe_only.len(), 4);
        for state in [Probation, Live, Flagged, Demoted] {
            assert_eq!(layers.h(m1, state), 0.10);
            assert_eq!(campaign.h(m1, state), 0.10);
            assert_eq!(layers.h(placebo, state), 0.5);
            assert_eq!(campaign.h(placebo, state), 0.5);
            assert_eq!(campaign.h(know, state), CAMPAIGN_HOLDOUT);
            for entry in &observe_only {
                assert_eq!(layers.h(entry, state), 0.0, "{}", entry.id);
            }
        }

        // A retired loop is not randomized either.
        let mut retired = know.clone();
        retired.lifecycle = Lifecycle::Retired {
            by: "S02.P1-7".to_string(),
        };
        assert_eq!(layers.h(&retired, Live), 0.0);

        // Maximize mode (--no-holdout, D7) withholds nothing: g = h = 0.
        let maximize = LoopLayer {
            maximize: true,
            ..LoopLayer::default()
        };
        let layer = maximize.layer_spec(placebo, Probation, EPOCH, SEED, G);
        assert_eq!((layer.h, layer.g), (0.0, 0.0));

        // A nested loop draws on its own layer, with its unit and h.
        let bias = layers.layer_spec(spec(&registry, "L-dream-bias"), Live, EPOCH, SEED, G);
        assert_eq!(bias.layer, "route.dream_bias");
        assert_eq!((bias.h, bias.g, bias.run_seed), (0.05, G, SEED));
        assert_eq!(bias.unit, AssignmentUnit::Chain);
        assert_eq!(bias.epoch, EPOCH);
    }

    #[test]
    fn placebo_is_its_own_layer_with_half_held_out() {
        const CHAINS: usize = 20_000;
        let registry = registry();
        let placebo = spec(&registry, "L-placebo");
        // One definition: its own layer, identical arms, nothing to prove.
        assert_eq!(placebo.receipt, ReceiptKind::Sham);
        assert!(placebo.nested_in.is_none() && placebo.lifecycle.is_randomized());
        let sharing = registry
            .loops()
            .iter()
            .filter(|entry| entry.layer == placebo.layer)
            .count();
        assert_eq!(sharing, 1);

        let layer = layer_of(&registry, "L-placebo", AuditState::Flagged);
        assert_eq!((layer.layer.as_str(), layer.h), ("placebo", 0.5));
        let arms = arms_on(&layer, CHAINS);
        let held_out = arms.iter().filter(|&&arm| arm == Arm::Default).count();
        let drawn = arms.iter().filter(|&&arm| arm != Arm::GlobalOff).count();
        let share = held_out as f64 / drawn as f64;
        assert!((share - 0.5).abs() < 0.02, "placebo default share {share}");
    }
}
