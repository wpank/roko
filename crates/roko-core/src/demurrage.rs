//! Time-decay tax on stored value (Gesellian demurrage).
//!
//! Implementors must be refreshed (validated, used) to maintain their balance.
//! Neglected items naturally fade, ensuring active validation of knowledge.
//!
//! This trait formalizes the demurrage pattern already present on
//! [`PlaybookRules::Rule`] and the `DemurrageConfig` in `roko.toml`.
//!
//! The spec-compliant path uses [`demurrage_tick`] with a [`DemurrageConfig`],
//! which implements a 3-term novelty-weighted rate law:
//!
//! ```text
//! d(balance)/dt = -flat_tax - beta*balance + reinforcement*novelty
//! ```
//!
//! The legacy [`Demurrage`] trait remains for backward compatibility with
//! simpler single-rate exponential decay.

use crate::Engram;
use serde::{Deserialize, Serialize};

/// Configuration for the novelty-weighted demurrage tick.
///
/// Controls the three terms of the rate law:
/// ```text
/// new_balance = balance
///             - flat_tax*(elapsed/24)
///             - exp_decay*balance*(elapsed/24)
///             + novelty_bonus*novelty*(elapsed/24)
/// ```
///
/// The result is clamped to `[0.0, 1.0]`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DemurrageConfig {
    /// Flat daily tax applied regardless of balance (default 0.01).
    ///
    /// This is the "circulation incentive" — every signal costs something
    /// just to exist in the store.
    #[serde(default = "default_flat_tax")]
    pub flat_tax_per_day: f64,

    /// Exponential decay rate per day — the beta term (default 0.05).
    ///
    /// Scales with the current balance: higher-balance signals decay faster
    /// in absolute terms.
    #[serde(default = "default_exp_decay")]
    pub exp_decay_per_day: f64,

    /// Novelty reinforcement multiplier (default 0.1).
    ///
    /// Novel signals decay slower — high-novelty input partially offsets
    /// the flat and exponential terms.
    #[serde(default = "default_novelty_bonus")]
    pub novelty_bonus: f64,
}

impl Default for DemurrageConfig {
    fn default() -> Self {
        Self {
            flat_tax_per_day: default_flat_tax(),
            exp_decay_per_day: default_exp_decay(),
            novelty_bonus: default_novelty_bonus(),
        }
    }
}

fn default_flat_tax() -> f64 {
    0.01
}

fn default_exp_decay() -> f64 {
    0.05
}

fn default_novelty_bonus() -> f64 {
    0.1
}

/// Apply a novelty-weighted demurrage tick to an [`Engram`].
///
/// Implements the 3-term rate law from the v2 spec (01-SIGNAL.md section 6):
///
/// ```text
/// new_balance = balance
///             - flat_tax*(elapsed/24)
///             - exp_decay*balance*(elapsed/24)
///             + novelty_bonus*novelty*(elapsed/24)
/// balance = new_balance.clamp(0.0, 1.0)
/// ```
///
/// The amount lost (before clamping) is accumulated in `engram.demurrage_paid`,
/// which is monotonically increasing — it never decreases.
///
/// # Parameters
///
/// - `engram`: the signal whose balance will be decayed in-place
/// - `config`: tunable rate parameters
/// - `elapsed_hours`: time since last tick (must be non-negative)
/// - `novelty`: the signal's current novelty in `[0..1]`, from [`Score::novelty`]
pub fn demurrage_tick(
    engram: &mut Engram,
    config: &DemurrageConfig,
    elapsed_hours: f64,
    novelty: f32,
) {
    if elapsed_hours <= 0.0 {
        return;
    }

    let elapsed_days = elapsed_hours / 24.0;
    let flat_loss = config.flat_tax_per_day * elapsed_days;
    let exp_loss = config.exp_decay_per_day * engram.balance * elapsed_days;
    let novelty_gain = config.novelty_bonus * f64::from(novelty) * elapsed_days;

    let total_loss = flat_loss + exp_loss - novelty_gain;
    let new_balance = (engram.balance - total_loss).clamp(0.0, 1.0);

    // Track cumulative tax paid (monotonically increasing).
    let actual_loss = engram.balance - new_balance;
    if actual_loss > 0.0 {
        engram.demurrage_paid += actual_loss;
    }

    engram.balance = new_balance;
}

/// Time-decay tax on stored value -- ensures active validation.
///
/// Implementors must be refreshed (validated, used) to maintain
/// their balance. Neglected items naturally fade.
///
/// The decay formula is: `balance *= (1 - rate)^elapsed_hours`
///
/// For the full spec-compliant 3-term formula, use [`demurrage_tick`] instead.
pub trait Demurrage {
    /// Current attention/value balance in `[0.0, 1.0]`.
    fn balance(&self) -> f64;

    /// Hourly decay rate in `[0.0, 1.0]`.
    fn demurrage_rate(&self) -> f64;

    /// Apply time-based decay for `elapsed_hours` of inactivity.
    fn tick(&mut self, elapsed_hours: f64);

    /// Replenish the balance (capped at 1.0) after active validation.
    fn replenish(&mut self, amount: f64);

    /// Returns `true` when the balance has fallen below the usable threshold.
    fn is_depleted(&self) -> bool {
        self.balance() < 0.1
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Body, Kind};

    struct TestItem {
        balance: f64,
        rate: f64,
    }

    impl Demurrage for TestItem {
        fn balance(&self) -> f64 {
            self.balance
        }

        fn demurrage_rate(&self) -> f64 {
            self.rate
        }

        fn tick(&mut self, elapsed_hours: f64) {
            if elapsed_hours > 0.0 {
                self.balance *= (1.0 - self.rate).powf(elapsed_hours);
            }
        }

        fn replenish(&mut self, amount: f64) {
            self.balance = (self.balance + amount).min(1.0);
        }
    }

    #[test]
    fn tick_decays_balance() {
        let mut item = TestItem {
            balance: 1.0,
            rate: 0.01,
        };
        item.tick(100.0);
        assert!(item.balance() < 1.0);
        assert!(item.balance() > 0.0);
    }

    #[test]
    fn replenish_caps_at_one() {
        let mut item = TestItem {
            balance: 0.5,
            rate: 0.01,
        };
        item.replenish(0.8);
        assert!((item.balance() - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn is_depleted_when_low() {
        let item = TestItem {
            balance: 0.05,
            rate: 0.01,
        };
        assert!(item.is_depleted());
    }

    #[test]
    fn not_depleted_when_healthy() {
        let item = TestItem {
            balance: 0.5,
            rate: 0.01,
        };
        assert!(!item.is_depleted());
    }

    #[test]
    fn zero_elapsed_does_not_decay() {
        let mut item = TestItem {
            balance: 1.0,
            rate: 0.01,
        };
        item.tick(0.0);
        assert!((item.balance() - 1.0).abs() < f64::EPSILON);
    }

    // ─── DemurrageConfig + demurrage_tick tests ─────────────────────────────

    fn make_engram(balance: f64) -> Engram {
        Engram::builder(Kind::Task)
            .body(Body::text("test signal"))
            .balance(balance)
            .build()
    }

    #[test]
    fn demurrage_config_defaults() {
        let cfg = DemurrageConfig::default();
        assert!((cfg.flat_tax_per_day - 0.01).abs() < f64::EPSILON);
        assert!((cfg.exp_decay_per_day - 0.05).abs() < f64::EPSILON);
        assert!((cfg.novelty_bonus - 0.1).abs() < f64::EPSILON);
    }

    #[test]
    fn demurrage_tick_zero_elapsed_is_noop() {
        let mut e = make_engram(0.8);
        let cfg = DemurrageConfig::default();
        demurrage_tick(&mut e, &cfg, 0.0, 0.0);
        assert!((e.balance - 0.8).abs() < f64::EPSILON);
        assert!((e.demurrage_paid - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn demurrage_tick_negative_elapsed_is_noop() {
        let mut e = make_engram(0.8);
        let cfg = DemurrageConfig::default();
        demurrage_tick(&mut e, &cfg, -1.0, 0.0);
        assert!((e.balance - 0.8).abs() < f64::EPSILON);
    }

    #[test]
    fn demurrage_tick_reduces_balance() {
        let mut e = make_engram(1.0);
        let cfg = DemurrageConfig::default();
        demurrage_tick(&mut e, &cfg, 24.0, 0.0);
        assert!(e.balance < 1.0);
        assert!(e.balance > 0.0);
    }

    #[test]
    fn demurrage_tick_novelty_slows_decay() {
        let mut no_novelty = make_engram(1.0);
        let mut high_novelty = make_engram(1.0);
        let cfg = DemurrageConfig::default();

        demurrage_tick(&mut no_novelty, &cfg, 24.0, 0.0);
        demurrage_tick(&mut high_novelty, &cfg, 24.0, 1.0);

        assert!(high_novelty.balance > no_novelty.balance);
    }

    #[test]
    fn demurrage_tick_clamps_to_zero() {
        let mut e = make_engram(0.001);
        let cfg = DemurrageConfig {
            flat_tax_per_day: 10.0,
            exp_decay_per_day: 5.0,
            novelty_bonus: 0.0,
        };
        demurrage_tick(&mut e, &cfg, 24.0, 0.0);
        assert!((e.balance - 0.0).abs() < f64::EPSILON);
        assert!((e.demurrage_paid - 0.001).abs() < 1e-10);
    }

    #[test]
    fn demurrage_paid_accumulates_monotonically() {
        let mut e = make_engram(1.0);
        let cfg = DemurrageConfig::default();

        demurrage_tick(&mut e, &cfg, 24.0, 0.0);
        let paid_after_first = e.demurrage_paid;
        assert!(paid_after_first > 0.0);

        demurrage_tick(&mut e, &cfg, 24.0, 0.0);
        let paid_after_second = e.demurrage_paid;
        assert!(paid_after_second >= paid_after_first);
    }
}
