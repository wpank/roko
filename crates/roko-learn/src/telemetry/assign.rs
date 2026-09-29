//! The one assignment function (S01 §4.6). S02, S03, S04 and S06 call it
//! and never re-implement it.
//!
//! ```text
//! key(layer, epoch) = blake3::derive_key("roko.assign/1", seed ‖ 0x00 ‖ layer ‖ 0x00 ‖ epoch)
//! u(layer, unit)    = u64_le(blake3::keyed_hash(key(layer, epoch), unit_key)[0..8]) / 2^64
//! arm               = global_off if u("global", chain_key) < g,
//!                     else default if u(layer, unit) < h, else learned
//! propensity        = g | (1 − g)·h | (1 − g)·(1 − h)
//! ```
//!
//! The seed enters the key material as its decimal digits. Keys are never
//! logged: the logged `salt_id` is `"{layer}@{epoch}"`. Unlike `std`'s
//! `DefaultHasher`, whose algorithm is unspecified, these draws are stable
//! across Rust releases and reproducible outside Rust.

use serde::{Deserialize, Serialize};

use super::records::AttemptKey;

/// `blake3::derive_key` context of the layer keys.
pub const ASSIGN_CONTEXT: &str = "roko.assign/1";
/// Layer of the all-learning-off draw (S03).
pub const GLOBAL_LAYER: &str = "global";

/// 2^-53: scales the top 53 bits of a draw into `[0, 1)`.
const UNIT_SCALE: f64 = 1.0 / 9_007_199_254_740_992.0;

/// The arm a unit lands in (S01 §4.6), one vocabulary for every spec.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Arm {
    /// The loop's learned policy (S03's L).
    Learned,
    /// The default policy π⁰ (S03's D).
    Default,
    /// Every loop takes π⁰, and M1 runs θ₀.
    GlobalOff,
    /// S02's ε draw inside the learned arm of the route decision. [`assign`]
    /// never returns it.
    Explore,
}

/// What a layer randomizes (`unit_key`, S01 §4.6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssignmentUnit {
    /// The chain key, so every retry of a task stays in one arm. The
    /// default.
    Chain,
    /// `run_id:plan_id`, for the knowledge and playbook writers.
    Plan,
    /// The attempt key, for the per-attempt `route.explore` draw.
    Attempt,
}

impl AssignmentUnit {
    /// The unit key of `key` for this unit.
    #[must_use]
    pub fn unit_key(self, key: &AttemptKey) -> String {
        match self {
            Self::Chain => key.chain_key(),
            Self::Plan => format!("{}:{}", key.run_id, key.plan_id),
            Self::Attempt => key.attempt_key(),
        }
    }
}

/// One layer's assignment inputs for an epoch.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LayerSpec {
    /// The run's `experiment.seed`.
    pub run_seed: u64,
    /// Layer name, one per decision point: e.g. `route` or `route.explore`.
    pub layer: String,
    /// The UTC day in production, the bench-run id in campaigns.
    pub epoch: String,
    /// What the layer randomizes.
    pub unit: AssignmentUnit,
    /// P(default arm | not global_off).
    pub h: f64,
    /// P(global_off).
    pub g: f64,
}

/// The logged `assignment` object (S01 §5.3):
/// `{unit, layer, salt_id, u, h, g, arm, propensity}`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Assignment {
    /// What the layer randomized.
    pub unit: AssignmentUnit,
    /// Layer name.
    pub layer: String,
    /// `"{layer}@{epoch}"`, logged in place of the key.
    pub salt_id: String,
    /// The unit's draw on this layer, in `[0, 1)`.
    pub u: f64,
    /// P(default arm | not global_off), clamped to `[0, 1]`.
    pub h: f64,
    /// P(global_off), clamped to `[0, 1]`.
    pub g: f64,
    /// The realized arm.
    pub arm: Arm,
    /// P(realized arm): `g`, `(1 − g)·h` or `(1 − g)·(1 − h)`.
    pub propensity: f64,
}

impl Assignment {
    /// P(the loop takes the default policy π⁰) = `g + (1 − g)·h`.
    #[must_use]
    pub fn default_policy_probability(&self) -> f64 {
        self.g + (1.0 - self.g) * self.h
    }
}

/// Assign `key` on `spec`'s layer. Pure: the same inputs always give the
/// same assignment. The all-off draw uses the chain key on
/// [`GLOBAL_LAYER`]; the arm draw uses the key of `spec.unit`.
#[must_use]
pub fn assign(spec: &LayerSpec, key: &AttemptKey) -> Assignment {
    let h = probability(spec.h);
    let g = probability(spec.g);
    let global_u = draw(spec.run_seed, GLOBAL_LAYER, &spec.epoch, &key.chain_key());
    let unit_key = spec.unit.unit_key(key);
    let u = draw(spec.run_seed, &spec.layer, &spec.epoch, &unit_key);
    let (arm, propensity) = if global_u < g {
        (Arm::GlobalOff, g)
    } else if u < h {
        (Arm::Default, (1.0 - g) * h)
    } else {
        (Arm::Learned, (1.0 - g) * (1.0 - h))
    };
    Assignment {
        unit: spec.unit,
        layer: spec.layer.clone(),
        salt_id: salt_id(&spec.layer, &spec.epoch),
        u,
        h,
        g,
        arm,
        propensity,
    }
}

/// `"{layer}@{epoch}"`: names a layer key without revealing it.
#[must_use]
pub fn salt_id(layer: &str, epoch: &str) -> String {
    format!("{layer}@{epoch}")
}

/// `u(layer, unit)`: the unit's draw on a layer, in `[0, 1)`.
///
/// It is the first 8 digest bytes read as a little-endian `u64` over 2^64,
/// truncated to the 53 bits an `f64` holds so that it never rounds up to 1.
#[must_use]
pub fn draw(run_seed: u64, layer: &str, epoch: &str, unit_key: &str) -> f64 {
    let digest = blake3::keyed_hash(&layer_key(run_seed, layer, epoch), unit_key.as_bytes());
    let mut head = [0_u8; 8];
    head.copy_from_slice(&digest.as_bytes()[..8]);
    (u64::from_le_bytes(head) >> 11) as f64 * UNIT_SCALE
}

/// `key(layer, epoch)`: 32 bytes, never logged.
fn layer_key(run_seed: u64, layer: &str, epoch: &str) -> [u8; 32] {
    let seed = run_seed.to_string();
    let mut material = Vec::with_capacity(seed.len() + layer.len() + epoch.len() + 2);
    material.extend_from_slice(seed.as_bytes());
    material.push(0);
    material.extend_from_slice(layer.as_bytes());
    material.push(0);
    material.extend_from_slice(epoch.as_bytes());
    blake3::derive_key(ASSIGN_CONTEXT, &material)
}

/// Clamp a probability to `[0, 1]`; NaN counts as 0.
fn probability(p: f64) -> f64 {
    if p.is_nan() { 0.0 } else { p.clamp(0.0, 1.0) }
}

#[cfg(test)]
mod tests {
    use super::{Arm, Assignment, AssignmentUnit, LayerSpec, assign, draw, salt_id};
    use crate::telemetry::records::AttemptKey;

    const SEED: u64 = 1234;
    const EPOCH: &str = "2026-10-02";

    fn route_layer(h: f64, g: f64) -> LayerSpec {
        LayerSpec {
            run_seed: SEED,
            layer: "route".to_string(),
            epoch: EPOCH.to_string(),
            unit: AssignmentUnit::Chain,
            h,
            g,
        }
    }

    fn pearson(xs: &[f64], ys: &[f64]) -> f64 {
        let n = xs.len() as f64;
        let mean_x = xs.iter().sum::<f64>() / n;
        let mean_y = ys.iter().sum::<f64>() / n;
        let (mut sxy, mut sxx, mut syy) = (0.0, 0.0, 0.0);
        for (x, y) in xs.iter().zip(ys) {
            sxy += (x - mean_x) * (y - mean_y);
            sxx += (x - mean_x).powi(2);
            syy += (y - mean_y).powi(2);
        }
        sxy / (sxx * syy).sqrt()
    }

    #[test]
    fn assign_draw_matches_golden_vector() {
        // Pinned with the reference `blake3` Python package. A change here
        // would silently reshuffle every running experiment.
        const GOLDEN: f64 = 0.196_341_219_703_467;
        let u = draw(SEED, "route", EPOCH, "gr-7f3c2a91:loop-census:T2");
        assert!((u - GOLDEN).abs() < 1e-12, "u = {u}");
    }

    #[test]
    fn assign_is_deterministic_and_logs_the_salt_not_the_key() {
        let key = AttemptKey::new("gr-7f3c2a91", "loop-census", "T2", 2);
        let spec = route_layer(0.5, 0.1);
        let first = assign(&spec, &key);
        assert_eq!(first, assign(&spec, &key));
        assert_eq!(first.salt_id, "route@2026-10-02");
        assert_eq!(salt_id("route", EPOCH), first.salt_id);
        assert!((0.0..1.0).contains(&first.u));

        // Retries of one task share the chain, so they share the arm.
        let retry = AttemptKey::new("gr-7f3c2a91", "loop-census", "T2", 3);
        assert_eq!(assign(&spec, &retry).arm, first.arm);

        let json = serde_json::to_value(&first).expect("serialize assignment");
        assert_eq!(json["unit"], "chain");
        assert_eq!(json["salt_id"], "route@2026-10-02");
        let back: Assignment = serde_json::from_value(json).expect("parse assignment");
        assert_eq!(back, first);
    }

    #[test]
    fn assign_draws_are_uniform_by_chi_square() {
        const DRAWS: usize = 100_000;
        const BINS: usize = 100;
        // χ²(99) at α = 0.001. This fixed sample scores about 96.9.
        const CRITICAL: f64 = 148.23;
        let mut counts = [0_u32; BINS];
        for index in 0..DRAWS {
            let unit_key = format!("gr-chi2:loop-census:t{index}");
            let u = draw(SEED, "route", EPOCH, &unit_key);
            assert!((0.0..1.0).contains(&u), "draw out of range: {u}");
            counts[(u * BINS as f64) as usize] += 1;
        }
        let expected = DRAWS as f64 / BINS as f64;
        let chi_square: f64 = counts
            .iter()
            .map(|&count| (f64::from(count) - expected).powi(2) / expected)
            .sum();
        assert!(chi_square < CRITICAL, "chi-square {chi_square}");
    }

    #[test]
    fn assign_layers_and_epochs_draw_independently() {
        const UNITS: usize = 100_000;
        let unit_keys: Vec<String> = (0..UNITS)
            .map(|index| format!("gr-indep:loop-census:t{index}"))
            .collect();
        let draws = |layer: &str, epoch: &str| -> Vec<f64> {
            unit_keys
                .iter()
                .map(|unit| draw(SEED, layer, epoch, unit))
                .collect()
        };
        let route = draws("route", EPOCH);
        for other in [
            draws("knowledge", EPOCH),
            draws("global", EPOCH),
            draws("route", "2026-10-03"),
        ] {
            let r = pearson(&route, &other);
            assert!(r.abs() < 0.02, "correlation {r}");
        }
    }

    #[test]
    fn assign_arms_follow_h_and_g() {
        const CHAINS: u32 = 20_000;
        let spec = route_layer(0.3, 0.1);
        let mut counts = [0_u32; 3];
        for index in 0..CHAINS {
            let key = AttemptKey::new("gr-arms", "loop-census", format!("t{index}"), 1);
            let assignment = assign(&spec, &key);
            let (slot, propensity) = match assignment.arm {
                Arm::GlobalOff => (0, 0.1),
                Arm::Default => (1, 0.9 * 0.3),
                Arm::Learned => (2, 0.9 * 0.7),
                Arm::Explore => unreachable!("assign never returns explore"),
            };
            counts[slot] += 1;
            assert!((assignment.propensity - propensity).abs() < 1e-12);
            let p_default = assignment.default_policy_probability();
            assert!((p_default - 0.37).abs() < 1e-12);
        }
        let fractions = counts.map(|count| f64::from(count) / f64::from(CHAINS));
        for (fraction, expected) in fractions.into_iter().zip([0.1, 0.27, 0.63]) {
            assert!((fraction - expected).abs() < 0.02, "{fractions:?}");
        }
    }

    #[test]
    fn assign_degenerate_probabilities_pick_one_arm_with_certainty() {
        let key = AttemptKey::new("gr-edge", "loop-census", "T1", 1);
        let cases = [
            (0.0, 0.0, Arm::Learned),
            (1.0, 0.0, Arm::Default),
            (0.0, 1.0, Arm::GlobalOff),
            (f64::NAN, 2.0, Arm::GlobalOff),
        ];
        for (h, g, arm) in cases {
            let assignment = assign(&route_layer(h, g), &key);
            assert_eq!(assignment.arm, arm, "h={h} g={g}");
            assert_eq!(assignment.propensity, 1.0);
        }
    }

    #[test]
    fn assignment_units_key_the_chain_the_plan_and_the_attempt() {
        let key = AttemptKey::new("gr-1", "plan-a", "T2", 3);
        assert_eq!(AssignmentUnit::Chain.unit_key(&key), "gr-1:plan-a:T2");
        assert_eq!(AssignmentUnit::Plan.unit_key(&key), "gr-1:plan-a");
        assert_eq!(AssignmentUnit::Attempt.unit_key(&key), "gr-1:plan-a:T2:3");
        let arm = serde_json::to_value(Arm::GlobalOff).expect("serialize arm");
        assert_eq!(arm, "global_off");
    }
}
