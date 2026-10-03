//! A window's false-green estimates from its audited units (S05 §4.5), and
//! the live betting sequence. The Python reference is
//! `benchmarks/viabilitybench/audit/estimate.py`.
//!
//! For a window of N green units, s the audited ones, π_i their inclusion
//! probabilities, w_i = 1/π_i and Y_i ∈ {0, 1} their labels (γ̂ and ω̂ take
//! the gaming and weak-oracle labels the same way):
//!
//! - θ̂_HT = (1/N) Σ_s Y_i/π_i, Horvitz–Thompson, unbiased since every
//!   π_i ≥ ε_floor;
//! - θ̂_H = Σ_s w_i Y_i / N̂ with N̂ = Σ_s w_i, Hájek, the reported estimate;
//! - v̂ = Σ_s (1 − π_i) w_i² (Y_i − θ̂_H)² / N̂², its variance;
//! - n_eff = N̂² / Σ_s w_i², Kish's effective sample size;
//! - the interval is Wilson's at n_eff in every cell ([`wilson`]). Will
//!   decided this on 2026-10-02 (gap-a499aa): S05 §4.5's first rule, Wald
//!   on v̂ once n_eff ≥ 30 with 5 events, missed SC1's 0.93 coverage.
//!
//! [`betting_cs`] runs the hedged capital process of Waudby-Smith and
//! Ramdas (2024) on Z_i = ε_floor·S_i·Y_i/π_i ([`audit_z`]), over every green
//! unit in stream order: a confidence sequence for θ that holds at every
//! stopping time.

use serde::{Deserialize, Serialize};

use super::policy::EPS_FLOOR;
use super::{AuditError, strictly_within, within};

/// z = Φ⁻¹(0.975), as scipy prints it and the Python reference uses it.
pub const Z95: f64 = 1.959963984540054;
/// The one interval method: Wilson at Kish n_eff.
pub const CI_METHOD: &str = "wilson_eff";
/// The bet truncation c: a capital factor never drops below 1 − c.
pub const CS_C: f64 = 0.5;
/// The weight of the upward capital process; the downward one gets the rest.
pub const CS_HEDGE: f64 = 0.5;
/// The default grid of [`betting_cs`].
pub const CS_GRID: usize = 1000;

/// One window's estimate, named as `audit.estimate` records it (S05 §5).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Estimate {
    /// N, the window's green units.
    pub n_green: u64,
    /// The audited units.
    pub n_audited: u64,
    /// The audited units labelled 1.
    pub events: u64,
    /// N̂ = Σ_s 1/π_i.
    pub n_hat: f64,
    /// θ̂_HT.
    pub theta_ht: f64,
    /// θ̂_H; `None` for an empty sample.
    pub theta_hajek: Option<f64>,
    /// v̂(θ̂_H); `None` for an empty sample.
    pub variance: Option<f64>,
    /// Kish's n_eff.
    pub n_eff: f64,
    /// The interval, [0, 1] for an empty sample.
    pub ci: (f64, f64),
    /// [`CI_METHOD`].
    pub ci_method: String,
    /// The interval's α.
    pub alpha: f64,
}

/// z = Φ⁻¹(1 − α/2) for the levels audits use: α = 0.05 ([`Z95`]), 0.10
/// and 0.01.
///
/// # Errors
///
/// Any other α.
pub fn z_value(alpha: f64) -> Result<f64, AuditError> {
    const LEVELS: [(f64, f64); 3] = [
        (0.05, Z95),
        (0.10, 1.6448536269514715),
        (0.01, 2.5758293035489),
    ];
    LEVELS
        .iter()
        .find(|(level, _)| (level - alpha).abs() < 1e-12)
        .map(|(_, z)| *z)
        .ok_or_else(|| AuditError::Invalid(format!("alpha {alpha} is not 0.05, 0.10 or 0.01")))
}

/// HT, Hájek, v̂, n_eff and the Wilson interval from the audited units'
/// (π, Y), for a window of `n_green` green units.
///
/// # Errors
///
/// A π outside (0, 1], a label other than 0 or 1, more audited units than
/// green ones, or an α [`z_value`] refuses.
pub fn estimate(audited: &[(f64, u8)], n_green: u64, alpha: f64) -> Result<Estimate, AuditError> {
    for &(pi, y) in audited {
        strictly_within("pi", pi, 0.0, f64::INFINITY)?;
        within("pi", pi, 0.0, 1.0)?;
        if y > 1 {
            return Err(AuditError::Invalid(format!("a label is 0 or 1, not {y}")));
        }
    }
    let n_audited = audited.len() as u64;
    if n_green < n_audited.max(1) {
        return Err(AuditError::Invalid(format!(
            "a window of {n_green} green units cannot hold {n_audited} audited ones"
        )));
    }
    let z = z_value(alpha)?;
    let ci_method = CI_METHOD.to_string();
    if audited.is_empty() {
        return Ok(Estimate {
            n_green,
            n_audited: 0,
            events: 0,
            n_hat: 0.0,
            theta_ht: 0.0,
            theta_hajek: None,
            variance: None,
            n_eff: 0.0,
            ci: (0.0, 1.0),
            ci_method,
            alpha,
        });
    }
    let events = audited.iter().map(|&(_, y)| u64::from(y)).sum();
    let n_hat: f64 = audited.iter().map(|&(pi, _)| 1.0 / pi).sum();
    let weighted: f64 = audited.iter().map(|&(pi, y)| f64::from(y) / pi).sum();
    let theta = weighted / n_hat;
    let spread: f64 = audited
        .iter()
        .map(|&(pi, y)| (1.0 - pi) * (f64::from(y) - theta).powi(2) / pi.powi(2))
        .sum();
    let squares: f64 = audited.iter().map(|&(pi, _)| 1.0 / pi.powi(2)).sum();
    let n_eff = n_hat.powi(2) / squares;
    Ok(Estimate {
        n_green,
        n_audited,
        events,
        n_hat,
        theta_ht: weighted / n_green as f64,
        theta_hajek: Some(theta),
        variance: Some(spread / n_hat.powi(2)),
        n_eff,
        ci: wilson(theta, n_eff, z)?,
        ci_method,
        alpha,
    })
}

/// The estimate with every null label read as 0, and again as 1 (S05
/// §4.5).
///
/// # Errors
///
/// As [`estimate`].
pub fn null_bounds(
    audited: &[(f64, Option<u8>)],
    n_green: u64,
    alpha: f64,
) -> Result<(Estimate, Estimate), AuditError> {
    let read = |missing: u8| -> Vec<(f64, u8)> {
        audited
            .iter()
            .map(|&(pi, y)| (pi, y.unwrap_or(missing)))
            .collect()
    };
    Ok((
        estimate(&read(0), n_green, alpha)?,
        estimate(&read(1), n_green, alpha)?,
    ))
}

/// The Wilson score interval for a proportion `p` at a possibly fractional
/// sample size `n`; [0, 1] when `n` is 0. The ends are exactly 0 and 1 at
/// p = 0 and p = 1, as in the reference.
///
/// # Errors
///
/// A `p` outside [0, 1].
#[allow(clippy::float_cmp)] // the reference compares p with 0 and 1 exactly
pub fn wilson(p: f64, n: f64, z: f64) -> Result<(f64, f64), AuditError> {
    within("p", p, 0.0, 1.0)?;
    if n <= 0.0 {
        return Ok((0.0, 1.0));
    }
    let z2 = z * z;
    let denom = 1.0 + z2 / n;
    let center = (p + z2 / (2.0 * n)) / denom;
    let half = z * (p * (1.0 - p) / n + z2 / (4.0 * n * n)).sqrt() / denom;
    let low = if p == 0.0 {
        0.0
    } else {
        (center - half).max(0.0)
    };
    let high = if p == 1.0 {
        1.0
    } else {
        (center + half).min(1.0)
    };
    Ok((low, high))
}

/// Z_i = ε_floor·S_i·Y_i/π_i for one green unit; `y` may be `None` only
/// for a unit the lottery did not select.
///
/// # Errors
///
/// A `pi` outside [ε_floor, 1], or a selected unit without a 0 or 1 label.
pub fn audit_z(selected: bool, y: Option<u8>, pi: f64) -> Result<f64, AuditError> {
    within("pi", pi, EPS_FLOOR, 1.0)?;
    if !selected {
        return Ok(0.0);
    }
    match y {
        Some(y @ (0 | 1)) => Ok(EPS_FLOOR * f64::from(y) / pi),
        _ => Err(AuditError::Invalid(format!(
            "an audited unit needs a label of 0 or 1, not {y:?}"
        ))),
    }
}

/// K_t^±(m) = max(½·K_t^+(m), ½·K_t^−(m)) after each x_t ∈ [0, 1]: the
/// hedged capital of a test of mean `m` (Waudby-Smith and Ramdas 2024,
/// Theorem 3), with their predictable plug-in bets.
///
/// # Errors
///
/// An `m` or an x_t outside [0, 1], or an `alpha` outside (0, 1).
pub fn hedged_capital(xs: &[f64], m: f64, alpha: f64) -> Result<Vec<f64>, AuditError> {
    within("m", m, 0.0, 1.0)?;
    let mut process = Capital::new(alpha)?;
    let (mut up, mut down) = (1.0_f64, 1.0_f64);
    let mut out = Vec::with_capacity(xs.len());
    for &x in xs {
        within("x", x, 0.0, 1.0)?;
        let bet = process.bet();
        up *= 1.0 + up_bet(bet, m) * (x - m);
        down *= 1.0 - down_bet(bet, m) * (x - m);
        out.push((CS_HEDGE * up).max((1.0 - CS_HEDGE) * down));
        process.observe(x);
    }
    Ok(out)
}

/// The confidence sequence for θ after each unit: the running intersection
/// of {θ : K_t^±(ε_floor·θ/z_max) < 1/α} over a grid of step 1/`grid`. Each
/// entry is the hull of the grid points still inside, widened by a step on
/// each side and clipped to [0, 1]; `None` once no point is left.
///
/// # Errors
///
/// A `z_max` outside [ε_floor, 1], a z outside [0, z_max], an `alpha`
/// outside (0, 1), or a `grid` of 0.
pub fn betting_cs(
    z: &[f64],
    alpha: f64,
    z_max: f64,
    grid: usize,
) -> Result<Vec<Option<(f64, f64)>>, AuditError> {
    let xs = scaled(z, z_max)?;
    if grid == 0 {
        return Err(AuditError::Invalid(
            "the grid needs at least one step".into(),
        ));
    }
    let steps = grid as f64;
    let means: Vec<f64> = (0..=grid)
        .map(|j| EPS_FLOOR * (j as f64 / steps) / z_max)
        .collect();
    let mut up = vec![1.0_f64; grid + 1];
    let mut down = vec![1.0_f64; grid + 1];
    let mut inside = vec![true; grid + 1];
    let threshold = 1.0 / alpha;
    let mut process = Capital::new(alpha)?;
    let mut out = Vec::with_capacity(xs.len());
    for x in xs {
        let bet = process.bet();
        for (j, &m) in means.iter().enumerate() {
            if !inside[j] {
                continue;
            }
            up[j] *= 1.0 + up_bet(bet, m) * (x - m);
            down[j] *= 1.0 - down_bet(bet, m) * (x - m);
            if (CS_HEDGE * up[j]).max((1.0 - CS_HEDGE) * down[j]) >= threshold {
                inside[j] = false;
            }
        }
        process.observe(x);
        let first = inside.iter().position(|&kept| kept);
        let last = inside.iter().rposition(|&kept| kept);
        out.push(first.zip(last).map(|(first, last)| {
            let low = first.saturating_sub(1) as f64 / steps;
            (low, (last + 1).min(grid) as f64 / steps)
        }));
    }
    Ok(out)
}

/// Whether the sequence covers `theta` at every time: K_t^±(ε_floor·θ/z_max)
/// stays below 1/α throughout.
///
/// # Errors
///
/// As [`betting_cs`], and a `theta` outside [0, 1].
pub fn sequence_holds(z: &[f64], theta: f64, alpha: f64, z_max: f64) -> Result<bool, AuditError> {
    within("theta", theta, 0.0, 1.0)?;
    let threshold = 1.0 / alpha;
    let capital = hedged_capital(&scaled(z, z_max)?, EPS_FLOOR * theta / z_max, alpha)?;
    Ok(capital.iter().all(|&value| value < threshold))
}

/// The bets' shared state: the plug-in bet depends on the past values only,
/// never on the mean under test.
struct Capital {
    log_term: f64,
    t: u64,
    /// 1/2 + Σ x_i, the regularised running mean's numerator.
    total: f64,
    /// 1/4 + Σ (x_i − μ̂_i)², which is σ̂²_{t−1}·t.
    squares: f64,
}

impl Capital {
    fn new(alpha: f64) -> Result<Self, AuditError> {
        strictly_within("alpha", alpha, 0.0, 1.0)?;
        Ok(Self {
            log_term: 2.0 * (2.0 / alpha).ln(),
            t: 1,
            total: 0.5,
            squares: 0.25,
        })
    }

    fn bet(&self) -> f64 {
        (self.log_term / (self.squares * (self.t as f64 + 1.0).ln())).sqrt()
    }

    fn observe(&mut self, x: f64) {
        self.total += x;
        self.squares += (x - self.total / (self.t as f64 + 1.0)).powi(2);
        self.t += 1;
    }
}

fn up_bet(bet: f64, m: f64) -> f64 {
    if m == 0.0 { bet } else { bet.min(CS_C / m) }
}

#[allow(clippy::float_cmp)] // the reference bets without a cap at m = 1 exactly
fn down_bet(bet: f64, m: f64) -> f64 {
    if m == 1.0 {
        bet
    } else {
        bet.min(CS_C / (1.0 - m))
    }
}

fn scaled(z: &[f64], z_max: f64) -> Result<Vec<f64>, AuditError> {
    within("z_max", z_max, EPS_FLOOR, 1.0)?;
    z.iter()
        .map(|&value| within("z", value, 0.0, z_max).map(|value| value / z_max))
        .collect()
}

#[cfg(test)]
mod tests {
    use serde_json::Value;

    use super::super::{fixture, number};
    use super::*;

    /// Within 1e-9, relative for numbers above 1 (capital grows fast).
    fn close(left: f64, right: f64) -> bool {
        (left - right).abs() <= 1e-9 * right.abs().max(1.0)
    }

    /// A two-number array, such as an interval.
    fn pair(value: &Value) -> Option<(f64, f64)> {
        let ends = value.as_array()?;
        Some((ends.first()?.as_f64()?, ends.get(1)?.as_f64()?))
    }

    fn units(entry: &Value) -> Vec<(f64, Option<u8>)> {
        let pairs = entry["units"].as_array().expect("units");
        pairs
            .iter()
            .map(|pair| {
                let label = pair[1].as_u64().map(|y| u8::try_from(y).expect("a label"));
                (pair[0].as_f64().expect("pi"), label)
            })
            .collect()
    }

    fn assert_estimate(actual: &Estimate, expected: &Value) {
        let ints = [
            ("n_green", actual.n_green),
            ("n_audited", actual.n_audited),
            ("events", actual.events),
        ];
        for (name, value) in ints {
            assert_eq!(Some(value), expected[name].as_u64(), "{name}: {expected}");
        }
        let floats = [
            ("n_hat", actual.n_hat),
            ("theta_ht", actual.theta_ht),
            ("n_eff", actual.n_eff),
            ("alpha", actual.alpha),
        ];
        for (name, value) in floats {
            assert!(close(value, number(expected, name)), "{name}: {expected}");
        }
        for (name, value) in [
            ("theta_hajek", actual.theta_hajek),
            ("variance", actual.variance),
        ] {
            match (value, expected[name].as_f64()) {
                (Some(value), Some(want)) => assert!(close(value, want), "{name}: {expected}"),
                (None, None) => {}
                other => panic!("{name}: {other:?} in {expected}"),
            }
        }
        let (low, high) = pair(&expected["ci"]).expect("ci");
        assert!(close(actual.ci.0, low), "ci: {expected}");
        assert!(close(actual.ci.1, high), "ci: {expected}");
        let method = expected["ci_method"].as_str();
        assert_eq!(Some(actual.ci_method.as_str()), method);
    }

    #[test]
    fn estimators_match_the_python_fixtures() {
        let fixture = fixture();
        assert!(close(number(&fixture["constants"], "z95"), Z95));
        assert!(close(number(&fixture["constants"], "cs_c"), CS_C));
        for entry in fixture["wilson"].as_array().expect("wilson") {
            let (p, n, z) = (number(entry, "p"), number(entry, "n"), number(entry, "z"));
            let (low, high) = wilson(p, n, z).expect("an interval");
            let (want_low, want_high) = pair(&entry["ci"]).expect("ci");
            assert!(close(low, want_low) && close(high, want_high), "{entry}");
        }
        for entry in fixture["estimates"].as_array().expect("estimates") {
            let audited: Vec<(f64, u8)> = units(entry)
                .into_iter()
                .map(|(pi, y)| (pi, y.expect("a label")))
                .collect();
            let n_green = entry["n_green"].as_u64().expect("n_green");
            let actual = estimate(&audited, n_green, number(entry, "alpha")).expect("an estimate");
            assert_estimate(&actual, &entry["expected"]);
        }
        for entry in fixture["null_bounds"].as_array().expect("null_bounds") {
            let n_green = entry["n_green"].as_u64().expect("n_green");
            let (low, high) = null_bounds(&units(entry), n_green, 0.05).expect("bounds");
            assert_estimate(&low, &entry["missing_0"]);
            assert_estimate(&high, &entry["missing_1"]);
        }
        for entry in fixture["betting"].as_array().expect("betting") {
            let z: Vec<f64> = entry["z"]
                .as_array()
                .expect("z")
                .iter()
                .map(|value| value.as_f64().expect("a z"))
                .collect();
            let (alpha, z_max) = (number(entry, "alpha"), number(entry, "z_max"));
            let xs: Vec<f64> = z.iter().map(|value| value / z_max).collect();
            for (theta, expected) in entry["capital"].as_object().expect("capital") {
                let theta: f64 = theta.parse().expect("a theta");
                let capital =
                    hedged_capital(&xs, EPS_FLOOR * theta / z_max, alpha).expect("the capital");
                let expected = expected.as_array().expect("values");
                assert_eq!(capital.len(), expected.len());
                for (value, want) in capital.iter().zip(expected) {
                    let want = want.as_f64().expect("a value");
                    assert!(close(*value, want), "{theta}: {value} vs {want}");
                }
            }
            let grid = usize::try_from(entry["grid"].as_u64().expect("grid")).expect("usize");
            let sequence = betting_cs(&z, alpha, z_max, grid).expect("the sequence");
            let expected = entry["cs"].as_array().expect("cs");
            assert_eq!(sequence.len(), expected.len());
            for (step, want) in sequence.iter().zip(expected) {
                match (step, pair(want)) {
                    (Some(step), Some(want)) => {
                        assert!(close(step.0, want.0) && close(step.1, want.1), "{step:?}");
                    }
                    (None, None) => {}
                    other => panic!("{other:?}"),
                }
            }
        }
    }

    #[test]
    fn estimators_refuse_bad_input_and_bound_null_labels() {
        assert!(estimate(&[(0.0, 1)], 10, 0.05).is_err());
        assert!(estimate(&[(0.5, 2)], 10, 0.05).is_err());
        assert!(estimate(&[(0.5, 1); 3], 2, 0.05).is_err());
        assert!(estimate(&[], 10, 0.2).is_err());
        let (low, high) = null_bounds(&[(0.5, None), (0.5, Some(1))], 10, 0.05).expect("bounds");
        assert!(low.theta_hajek < high.theta_hajek);
        assert_eq!(audit_z(false, None, 0.1), Ok(0.0));
        assert!(audit_z(true, None, 0.1).is_err());
        assert!(close(audit_z(true, Some(1), 0.5).expect("z"), 0.1));
        assert!(sequence_holds(&[0.0; 20], 0.0, 0.05, 1.0).expect("holds"));
        assert!(betting_cs(&[0.5], 0.05, 1.0, 0).is_err());
    }
}
