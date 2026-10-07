//! SASO step-response metrics and the IAE (S06 §1, §4.9;
//! `hellerstein2004feedback`): the publishable part of M1 is how fast and at
//! what cost the harness regains viability, not the controller.
//!
//! A [`Tracer`] folds one arm's resolutions into a drive trace with its own
//! window and bands, the same instrument for every arm (the static arm A0
//! has no controller). [`Recovery::measure`] reads a trace from the
//! disturbance's onset, which the evaluator supplies and the controller
//! never sees:
//!
//! - detection delay: resolutions from the onset to the first breach;
//! - settling: the first resolution from which D stays 0 for
//!   `recover_window` resolutions, counted in resolutions, dollars and
//!   seconds from the onset;
//! - overshoot: the largest D after the first return to 0 (ringing);
//! - collateral: the largest excess of each EV that never breached;
//! - steady-state error: the mean D over the last `recover_window`
//!   resolutions;
//! - IAE = Σ D(t) from the onset, H6's primary endpoint;
//! - adaptation cost: spend above the holdout arm's per resolution, net,
//!   from the onset.
//!
//! Replayed results are labelled "replayed from logged outcomes (date, n)"
//! wherever they are shown (S06 §8).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::ev::{Drive, Ev, EvBands, EvWindow, drive};
use super::ledger::RecoveryRow;
use super::policy::ViabilityPolicy;
use super::resolution::TaskResolution;

/// One resolution of a drive trace.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TracePoint {
    /// D after the resolution.
    pub drive: f64,
    /// Each EV's excess g, its distance outside the inner band over σ, in
    /// [`Ev::ALL`] order.
    pub excess: [f64; 4],
    /// The EVs breached after the resolution.
    pub breached: Vec<Ev>,
    /// The resolution's spend.
    pub usd: f64,
    /// Its wall time, in seconds.
    pub wall_s: f64,
}

impl TracePoint {
    /// The point of `resolution`, after which the drive is `shaped` and
    /// `breached` are breached.
    #[must_use]
    pub fn new(shaped: &Drive, breached: Vec<Ev>, resolution: &TaskResolution) -> Self {
        let mut excess = [0.0; 4];
        for (slot, ev) in Ev::ALL.into_iter().enumerate() {
            excess[slot] = shaped
                .excess
                .iter()
                .find(|(other, _)| *other == ev)
                .map_or(0.0, |&(_, g)| g);
        }
        Self {
            drive: shaped.value,
            excess,
            breached,
            usd: resolution.api_equiv_usd.unwrap_or(0.0),
            wall_s: resolution.wall_ms.map_or(0.0, |ms| ms as f64 / 1000.0),
        }
    }
}

/// Folds one arm's resolutions into a drive trace.
#[derive(Debug, Clone)]
pub struct Tracer {
    policy: ViabilityPolicy,
    window: EvWindow,
    bands: EvBands,
}

impl Tracer {
    /// A tracer under `policy` with windows of `window` resolutions.
    #[must_use]
    pub fn new(policy: &ViabilityPolicy, window: usize) -> Self {
        Self {
            policy: policy.clone(),
            window: EvWindow::new(window),
            bands: EvBands::new(&policy.ev),
        }
    }

    /// The trace point of the next resolution. The bands move once the
    /// window is full; a holdout row enters nothing.
    pub fn push(&mut self, resolution: &TaskResolution) -> TracePoint {
        self.window.push(resolution);
        let estimates = self.window.estimate(&self.policy.ev);
        if self.window.is_full() {
            self.bands.update(&estimates);
        }
        let shaped = drive(&estimates, &self.policy.ev, &self.policy.drive);
        TracePoint::new(&shaped, self.bands.breached(), resolution)
    }
}

/// The SASO scorecard of one trace from a disturbance's onset.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Recovery {
    /// Resolutions from the onset to the first breach, the onset counting
    /// as 1; `None` when nothing breached.
    pub detection_delay: Option<u32>,
    /// Resolutions from the onset until D stays 0; `None` if it never does.
    pub settling_resolutions: Option<u32>,
    /// Spend over those resolutions.
    pub settling_usd: Option<f64>,
    /// Wall seconds over those resolutions.
    pub settling_secs: Option<f64>,
    /// The largest D after the first return to 0.
    pub overshoot: f64,
    /// The largest excess of each EV that never breached, when above 0.
    pub collateral_max: BTreeMap<String, f64>,
    /// The mean D over the last `recover_window` resolutions.
    pub steady_state_error: f64,
    /// IAE = Σ D(t) from the onset.
    pub iae: f64,
    /// Spend above the holdout arm's per resolution, net, from the onset.
    pub adaptation_cost_usd: f64,
}

impl Recovery {
    /// Measure `trace` from the resolution at index `onset`, the holdout
    /// arm spending `holdout_usd` per resolution, with D required to stay 0
    /// for `recover_window` resolutions to count as settled.
    #[must_use]
    pub fn measure(
        trace: &[TracePoint],
        onset: usize,
        holdout_usd: f64,
        recover_window: usize,
    ) -> Self {
        let after = trace.get(onset..).unwrap_or_default();
        let detection_delay = after
            .iter()
            .position(|point| !point.breached.is_empty())
            .map(|index| count(index + 1));
        let run = recover_window.max(1);
        let settled = (0..after.len()).find(|&start| {
            after.len() - start >= run && after[start..start + run].iter().all(calm)
        });
        let rising = after.iter().position(|point| !calm(point));
        let first_zero =
            rising.and_then(|start| after[start..].iter().position(calm).map(|i| start + i));
        let overshoot = first_zero.map_or(0.0, |zero| {
            after[zero..]
                .iter()
                .map(|point| point.drive)
                .fold(0.0, f64::max)
        });
        let tail = &after[after.len().saturating_sub(run)..];
        let steady_state_error = if tail.is_empty() {
            0.0
        } else {
            tail.iter().map(|point| point.drive).sum::<f64>() / tail.len() as f64
        };
        Self {
            detection_delay,
            settling_resolutions: settled.map(count),
            settling_usd: settled.map(|end| after[..end].iter().map(|point| point.usd).sum()),
            settling_secs: settled.map(|end| after[..end].iter().map(|point| point.wall_s).sum()),
            overshoot,
            collateral_max: collateral(after),
            steady_state_error,
            iae: after.iter().map(|point| point.drive).sum(),
            adaptation_cost_usd: after.iter().map(|point| point.usd - holdout_usd).sum(),
        }
    }

    /// The ledger's `recovery` row of `episode_id` on `ev`.
    #[must_use]
    pub fn row(&self, episode_id: &str, ev: Ev) -> RecoveryRow {
        RecoveryRow {
            episode_id: episode_id.to_string(),
            ev,
            settling_resolutions: self.settling_resolutions,
            settling_usd: self.settling_usd,
            overshoot: self.overshoot,
            collateral_max: self.collateral_max.clone(),
            steady_state_error: self.steady_state_error,
            iae: self.iae,
            adaptation_cost_usd: self.adaptation_cost_usd,
            detection_delay_resolutions: self.detection_delay,
            settling_secs: self.settling_secs,
        }
    }
}

fn calm(point: &TracePoint) -> bool {
    point.drive <= 0.0
}

fn count(resolutions: usize) -> u32 {
    u32::try_from(resolutions).unwrap_or(u32::MAX)
}

/// The largest excess of each EV that never breached in `after`.
fn collateral(after: &[TracePoint]) -> BTreeMap<String, f64> {
    let mut largest = BTreeMap::new();
    for (slot, ev) in Ev::ALL.into_iter().enumerate() {
        if after.iter().any(|point| point.breached.contains(&ev)) {
            continue;
        }
        let peak = after
            .iter()
            .map(|point| point.excess[slot])
            .fold(0.0, f64::max);
        if peak > 0.0 {
            largest.insert(ev.name().to_string(), peak);
        }
    }
    largest
}

#[cfg(test)]
mod tests {
    use serde_json::Value;

    use super::*;
    use crate::homeostasis::ledger::ControllerRow;

    fn close(actual: f64, expected: f64) -> bool {
        (actual - expected).abs() < 1e-9
    }

    #[test]
    fn saso_matches_hand_computed_fixture() {
        // 40 resolutions, the onset at index 10. D rises to 2 with E1
        // breached from 13 to 17, returns to 0 at 18, rings to 0.3 at 19,
        // and stays 0 from 20. E4 never breaches but its excess reaches
        // 0.12 at 16. Spend is $0.05 before the onset, $0.10 until 20 and
        // $0.06 after; every resolution takes 300 s.
        let drives = [
            0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, // 0-9
            0.0, 0.5, 1.0, 2.0, 2.0, 1.5, 1.0, 0.5, 0.0, 0.3, // 10-19
        ];
        let trace: Vec<TracePoint> = (0..40)
            .map(|index| {
                let mut excess = [0.0; 4];
                if index == 16 {
                    excess[3] = 0.12;
                }
                if index == 17 {
                    excess[3] = 0.05;
                }
                let breached = if (13..=17).contains(&index) {
                    vec![Ev::PassRate]
                } else {
                    Vec::new()
                };
                let usd = match index {
                    0..=9 => 0.05,
                    10..=19 => 0.10,
                    _ => 0.06,
                };
                TracePoint {
                    drive: drives.get(index).copied().unwrap_or(0.0),
                    excess,
                    breached,
                    usd,
                    wall_s: 300.0,
                }
            })
            .collect();
        let recovery = Recovery::measure(&trace, 10, 0.05, 10);

        // E1 breaches at 13: the fourth resolution from the onset.
        assert_eq!(recovery.detection_delay, Some(4));
        // D stays 0 from 20, ten resolutions after the onset; those cost
        // ten times $0.10 and 300 s.
        assert_eq!(recovery.settling_resolutions, Some(10));
        assert!(close(recovery.settling_usd.expect("settled"), 1.0));
        assert!(close(recovery.settling_secs.expect("settled"), 3000.0));
        // After the first return to 0 (at 18) D rings to 0.3.
        assert!(close(recovery.overshoot, 0.3));
        // Only E4 shows collateral; E1 breached, E2 and E3 never moved.
        assert_eq!(recovery.collateral_max.len(), 1);
        assert!(close(recovery.collateral_max["latency_p90_s"], 0.12));
        assert!(close(recovery.steady_state_error, 0.0));
        // IAE = 0.5 + 1 + 2 + 2 + 1.5 + 1 + 0.5 + 0.3.
        assert!(close(recovery.iae, 8.8), "{}", recovery.iae);
        // Ten resolutions $0.05 over the holdout, twenty $0.01 over it.
        assert!(close(recovery.adaptation_cost_usd, 0.7));

        // A trace that never settles has no settling numbers, and its
        // steady-state error is the drive left at the end.
        let stuck: Vec<TracePoint> = trace
            .iter()
            .cloned()
            .map(|mut point| {
                point.drive += 1.0;
                point
            })
            .collect();
        let unsettled = Recovery::measure(&stuck, 10, 0.05, 10);
        assert_eq!(unsettled.settling_resolutions, None);
        assert_eq!(unsettled.settling_usd, None);
        assert!(close(unsettled.steady_state_error, 1.0));

        // The row serializes as S01 v1.3 §5.10's recovery example.
        let example = Recovery {
            detection_delay: None,
            settling_resolutions: Some(27),
            settling_usd: Some(2.1),
            settling_secs: None,
            overshoot: 0.04,
            collateral_max: BTreeMap::from([("latency_p90_s".to_string(), 0.12)]),
            steady_state_error: 0.02,
            iae: 3.7,
            adaptation_cost_usd: 0.61,
        };
        let expected: Value = serde_json::from_str(
            r#"{"kind":"recovery","episode_id":"ep-0007","ev":"pass_rate","settling_resolutions":27,"settling_usd":2.1,"overshoot":0.04,"collateral_max":{"latency_p90_s":0.12},"steady_state_error":0.02,"iae":3.7,"adaptation_cost_usd":0.61}"#,
        )
        .expect("the example parses");
        let row = ControllerRow::Recovery(Box::new(example.row("ep-0007", Ev::PassRate)));
        assert_eq!(serde_json::to_value(&row).expect("serialize"), expected);
    }
}
