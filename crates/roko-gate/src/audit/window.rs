//! Window close (S05 §4.5): a window's green units, and its estimates of
//! false greens (θ), spec gaming (γ) and weak oracles (ω).
//!
//! A window holds the green units, in ledger order, whose `audit.selection`
//! records follow the last closed window's. It is due once it holds
//! `[audit] window_units` units whose selected ones have all been audited,
//! or once its first unit is `window_hours` old; a due window takes at most
//! `window_units` units, and the rest open the next one ([`due_windows`]).
//! Its id, `window:<first seq>-<last seq>`, names the selection records it
//! holds. A window is closed once its summary, the `audit.estimate` without
//! a stratum, is in the ledger ([`closed_through`]).
//!
//! The labels are phase B's sample (S05 §4.2): a selected unit whose audit
//! drew phase B counts at π_i·π_B, the result's `pi_eff`, and one whose
//! audit did not draw it is outside the sample and counts in N only. A
//! selected unit not yet audited, or audited without a label, is a null:
//! the estimate leaves it out, and its bounds read it as 0 and as 1. Every
//! interval is Wilson's at n_eff, at α = 0.05 ([`super::estimate`]).

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use roko_core::audit_types::{AuditLabels, Stratum, TrustEstimate};
use roko_core::config::audit::AuditConfig;
use serde::{Deserialize, Serialize};

use super::AuditError;
use super::estimate::{Estimate, estimate, null_bounds};
use super::ledger::{AuditEvent, LedgerRecord};

/// The prefix of a window's id.
pub const WINDOW_PREFIX: &str = "window:";

/// The α of every window interval: UCB95 is the upper end of a 95% one.
pub const WINDOW_ALPHA: f64 = 0.05;

/// One green unit of a window, with its audit once reported.
#[derive(Debug, Clone, PartialEq)]
pub struct WindowUnit {
    /// Its `audit.selection` record's seq.
    pub seq: u64,
    /// When the selection was logged.
    pub at: DateTime<Utc>,
    /// The stratum it was drawn in.
    pub stratum: Stratum,
    /// π_i.
    pub pi: f64,
    /// Whether the lottery selected it.
    pub selected: bool,
    /// Its `audit.result`, once the audit reported.
    pub audit: Option<Audited>,
}

/// What a selected unit's `audit.result` reported.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Audited {
    /// Its labels.
    pub labels: AuditLabels,
    /// Whether the audit drew phase B; `None` when it could not draw, as
    /// when the unit had no trees.
    pub phase_b: Option<bool>,
    /// π_i·π_B.
    pub pi_eff: Option<f64>,
}

impl WindowUnit {
    /// Whether the unit needs no more audit: not selected, or audited.
    #[must_use]
    pub const fn reported(&self) -> bool {
        !self.selected || self.audit.is_some()
    }

    /// The π its labels count at and the labels, or `None` outside phase
    /// B's sample. A selected unit without a usable audit is all nulls, at
    /// π_i.
    #[must_use]
    pub fn sampled(&self) -> Option<(f64, AuditLabels)> {
        if !self.selected {
            return None;
        }
        match self.audit {
            Some(audit) if audit.phase_b == Some(false) => None,
            Some(audit) if audit.phase_b == Some(true) => {
                Some((audit.pi_eff.unwrap_or(self.pi), audit.labels))
            }
            _ => Some((self.pi, AuditLabels::default())),
        }
    }

    /// The harness of the unit's arm: roko's for `prod` and the `roko_*`
    /// arms, else the arm itself.
    #[must_use]
    pub fn harness(&self) -> &str {
        let arm = self.stratum.arm.as_str();
        if arm == "prod" || arm.starts_with("roko") {
            "roko"
        } else {
            arm
        }
    }
}

/// Every green unit the ledger's `records` hold, in order, each with its
/// audit's report.
#[must_use]
pub fn units(records: &[LedgerRecord]) -> Vec<WindowUnit> {
    let mut audits: BTreeMap<&str, Audited> = BTreeMap::new();
    for record in records {
        if let AuditEvent::Result {
            sel_id,
            labels,
            checks,
            pi_eff,
            ..
        } = &record.event
        {
            let phase_b = checks["phase_b"]["drawn"].as_bool();
            let audited = Audited {
                labels: *labels,
                phase_b,
                pi_eff: *pi_eff,
            };
            audits.insert(sel_id.as_str(), audited);
        }
    }
    records
        .iter()
        .filter_map(|record| {
            let AuditEvent::Selection {
                sel_id,
                stratum,
                pi,
                selected,
                ..
            } = &record.event
            else {
                return None;
            };
            let at = DateTime::parse_from_rfc3339(&record.at).ok()?;
            Some(WindowUnit {
                seq: record.seq,
                at: at.with_timezone(&Utc),
                stratum: stratum.clone(),
                pi: *pi,
                selected: *selected,
                audit: audits.get(sel_id.as_str()).copied(),
            })
        })
        .collect()
}

/// The `(first, last)` selection seqs a window id names.
#[must_use]
pub fn window_span(id: &str) -> Option<(u64, u64)> {
    let (first, last) = id.strip_prefix(WINDOW_PREFIX)?.split_once('-')?;
    Some((first.parse().ok()?, last.parse().ok()?))
}

/// The last selection seq a closed window holds, 0 before the first.
#[must_use]
pub fn closed_through(records: &[LedgerRecord]) -> u64 {
    records
        .iter()
        .filter_map(|record| match &record.event {
            AuditEvent::Estimate {
                window,
                stratum: None,
                ..
            } => window_span(window).map(|(_, last)| last),
            _ => None,
        })
        .max()
        .unwrap_or(0)
}

/// A window's green units.
#[derive(Debug, Clone, PartialEq)]
pub struct Window {
    /// `window:<first seq>-<last seq>`.
    pub id: String,
    /// Its units, in ledger order; never empty.
    pub units: Vec<WindowUnit>,
}

/// The windows due to close at `now`, in order, after the last closed one
/// in `records`.
#[must_use]
pub fn due_windows(
    records: &[LedgerRecord],
    config: &AuditConfig,
    now: DateTime<Utc>,
) -> Vec<Window> {
    let after = closed_through(records);
    let mut open: Vec<WindowUnit> = units(records)
        .into_iter()
        .filter(|unit| unit.seq > after)
        .collect();
    let size = usize::try_from(config.window_units.max(1)).unwrap_or(usize::MAX);
    let span = chrono::Duration::hours(i64::from(config.window_hours));
    let mut due = Vec::new();
    while let Some(first) = open.first() {
        let take = open.len().min(size);
        let full = open.len() >= size && open[..take].iter().all(WindowUnit::reported);
        let old = now.signed_duration_since(first.at) >= span;
        if !full && !old {
            break;
        }
        let rest = open.split_off(take);
        let units = std::mem::replace(&mut open, rest);
        let id = format!("{WINDOW_PREFIX}{}-{}", units[0].seq, units[take - 1].seq);
        due.push(Window { id, units });
    }
    due
}

/// One label's estimate over a group of units.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LabelEstimate {
    /// Over the sampled units whose label is known.
    pub estimate: Estimate,
    /// Sampled units whose label is null.
    pub nulls: u64,
    /// θ̂_H with every null read as 0, and as 1; `None` without nulls.
    pub bounds: Option<(f64, f64)>,
}

impl LabelEstimate {
    /// UCB95: the upper end of the interval, `None` without a known label.
    #[must_use]
    pub fn ucb(&self) -> Option<f64> {
        self.estimate.theta_hajek.map(|_| self.estimate.ci.1)
    }
}

/// θ̂, γ̂ and ω̂ over a group of a window's units: a stratum, a task type,
/// a (model, harness) pair or the whole window.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GroupEstimate {
    /// N, its green units.
    pub n_green: u64,
    /// The units the lottery selected.
    pub n_selected: u64,
    /// θ̂, false greens.
    pub theta: LabelEstimate,
    /// γ̂, spec gaming.
    pub gamma: LabelEstimate,
    /// ω̂, weak oracles.
    pub omega: LabelEstimate,
}

/// The estimates of `units`, a non-empty group.
///
/// # Errors
///
/// An empty group, or a π the estimators refuse.
pub fn group_estimate(units: &[&WindowUnit]) -> Result<GroupEstimate, AuditError> {
    let n_green = units.len() as u64;
    let sample: Vec<(f64, AuditLabels)> = units.iter().filter_map(|unit| unit.sampled()).collect();
    Ok(GroupEstimate {
        n_green,
        n_selected: units.iter().filter(|unit| unit.selected).count() as u64,
        theta: label_estimate(&sample, |labels| labels.y, n_green)?,
        gamma: label_estimate(&sample, |labels| labels.g, n_green)?,
        omega: label_estimate(&sample, |labels| labels.w, n_green)?,
    })
}

/// One label's estimate over `sample`, from a group of `n_green` units.
fn label_estimate(
    sample: &[(f64, AuditLabels)],
    label: fn(&AuditLabels) -> Option<bool>,
    n_green: u64,
) -> Result<LabelEstimate, AuditError> {
    let labelled: Vec<(f64, Option<u8>)> = sample
        .iter()
        .map(|(pi, labels)| (*pi, label(labels).map(u8::from)))
        .collect();
    let known: Vec<(f64, u8)> = labelled
        .iter()
        .filter_map(|&(pi, y)| y.map(|y| (pi, y)))
        .collect();
    let nulls = (labelled.len() - known.len()) as u64;
    let bounds = if nulls == 0 {
        None
    } else {
        let (low, high) = null_bounds(&labelled, n_green, WINDOW_ALPHA)?;
        Some((
            low.theta_hajek.unwrap_or(0.0),
            high.theta_hajek.unwrap_or(1.0),
        ))
    };
    Ok(LabelEstimate {
        estimate: estimate(&known, n_green, WINDOW_ALPHA)?,
        nulls,
        bounds,
    })
}

/// What closing a window found.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WindowClose {
    /// The window.
    pub id: String,
    /// When its first unit was logged.
    pub opened_at: DateTime<Utc>,
    /// When its last unit was logged.
    pub last_at: DateTime<Utc>,
    /// The whole window.
    pub overall: GroupEstimate,
    /// Each stratum, in order.
    pub strata: Vec<(Stratum, GroupEstimate)>,
    /// Each task type, for the strictness ladder (DP3).
    pub task_types: BTreeMap<String, GroupEstimate>,
    /// Each (model, harness) pair with a known θ̂, for routing trust (DP4).
    pub trust: Vec<TrustEstimate>,
}

impl Window {
    /// The window's estimates: for the whole window, each stratum, each
    /// task type, and the trust in each (model, harness) pair.
    ///
    /// # Errors
    ///
    /// A π the estimators refuse.
    pub fn close(&self) -> Result<WindowClose, AuditError> {
        type Key = (String, String, String, String);
        let mut strata: BTreeMap<Key, Vec<&WindowUnit>> = BTreeMap::new();
        let mut task_types: BTreeMap<String, Vec<&WindowUnit>> = BTreeMap::new();
        let mut pairs: BTreeMap<(String, String), Vec<&WindowUnit>> = BTreeMap::new();
        for unit in &self.units {
            let stratum = &unit.stratum;
            let key = (
                stratum.task_type.clone(),
                stratum.model.clone(),
                stratum.arm.clone(),
                stratum.verdict.clone(),
            );
            strata.entry(key).or_default().push(unit);
            let task_type = stratum.task_type.clone();
            task_types.entry(task_type).or_default().push(unit);
            let pair = (stratum.model.clone(), unit.harness().to_string());
            pairs.entry(pair).or_default().push(unit);
        }
        let mut by_stratum = Vec::with_capacity(strata.len());
        for units in strata.into_values() {
            by_stratum.push((units[0].stratum.clone(), group_estimate(&units)?));
        }
        let mut by_task_type = BTreeMap::new();
        for (task_type, units) in task_types {
            by_task_type.insert(task_type, group_estimate(&units)?);
        }
        let mut trust = Vec::new();
        for ((model, harness), units) in &pairs {
            let theta = group_estimate(units)?.theta.estimate;
            if let Some(rate) = theta.theta_hajek {
                trust.push(TrustEstimate::from_estimate(
                    model,
                    harness,
                    rate,
                    theta.n_eff,
                ));
            }
        }
        let all: Vec<&WindowUnit> = self.units.iter().collect();
        Ok(WindowClose {
            id: self.id.clone(),
            opened_at: all.first().map_or_else(Utc::now, |unit| unit.at),
            last_at: all.last().map_or_else(Utc::now, |unit| unit.at),
            overall: group_estimate(&all)?,
            strata: by_stratum,
            task_types: by_task_type,
            trust,
        })
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    /// A selection record at `seq`, logged `minutes` after 10:00, of a
    /// `task_type` unit by `model`.
    fn selection(
        seq: u64,
        minutes: i64,
        task_type: &str,
        model: &str,
        pi: f64,
        selected: bool,
    ) -> LedgerRecord {
        let at = DateTime::parse_from_rfc3339("2026-10-03T10:00:00Z").expect("a time")
            + chrono::Duration::minutes(minutes);
        record(
            seq,
            &at.to_rfc3339(),
            AuditEvent::Selection {
                sel_id: format!("sel-{seq}"),
                attempt_key: format!("run:plan:t{seq}:1"),
                run_id: "run".to_string(),
                task_id: format!("t{seq}"),
                stratum: Stratum {
                    task_type: task_type.to_string(),
                    model: model.to_string(),
                    arm: "prod".to_string(),
                    verdict: "passed".to_string(),
                },
                pi,
                prf_u: "0x0000000000000000".to_string(),
                selected,
                base_tree: Some("base".to_string()),
                result_tree: Some("result".to_string()),
                risk_r: None,
            },
        )
    }

    /// The `audit.result` of selection `of`, at `seq`, phase B drawn at
    /// `pi_eff`.
    fn result(seq: u64, of: u64, found: AuditLabels, pi_eff: f64) -> LedgerRecord {
        record(
            seq,
            "2026-10-03T12:00:00Z",
            AuditEvent::Result {
                sel_id: format!("sel-{of}"),
                res_id: format!("res-{of}"),
                attempt_key: format!("run:plan:t{of}:1"),
                labels: found,
                findings: Vec::new(),
                checks: json!({ "phase_b": { "pi_b": 1.0, "drawn": true } }),
                cost_usd: Some(0.0),
                cpu_secs: Some(1.0),
                pi_eff: Some(pi_eff),
            },
        )
    }

    fn record(seq: u64, at: &str, event: AuditEvent) -> LedgerRecord {
        LedgerRecord {
            schema: "roko.audit/1".to_string(),
            record_id: format!("id-{seq}"),
            seq,
            at: at.to_string(),
            prev_hash: String::new(),
            record_hash: String::new(),
            event,
        }
    }

    const fn labels(y: Option<bool>, g: Option<bool>) -> AuditLabels {
        AuditLabels { y, g, w: None }
    }

    #[test]
    fn a_window_closes_at_its_unit_count_and_estimates_each_stratum() {
        let config = AuditConfig {
            window_units: 4,
            window_hours: 24,
            ..AuditConfig::default()
        };
        let now = DateTime::parse_from_rfc3339("2026-10-03T13:00:00Z")
            .expect("a time")
            .with_timezone(&Utc);
        let mut records = vec![
            selection(1, 0, "implementer", "glm-4.7", 0.5, true),
            selection(2, 1, "implementer", "glm-4.7", 0.5, false),
            selection(3, 2, "docs", "kimi-k2", 1.0, true),
            selection(4, 3, "implementer", "glm-4.7", 0.5, true),
            selection(5, 4, "implementer", "glm-4.7", 0.5, true),
        ];
        // Unit 4's audit has not reported: the full window waits for it.
        records.push(result(6, 1, labels(Some(true), Some(false)), 0.5));
        records.push(result(7, 3, labels(Some(false), Some(false)), 1.0));
        assert!(due_windows(&records, &config, now).is_empty());

        records.push(result(8, 4, labels(None, Some(true)), 0.5));
        let due = due_windows(&records, &config, now);
        assert_eq!(due.len(), 1, "unit 5 opens the next window");
        let window = &due[0];
        assert_eq!(window.id, "window:1-4");
        assert_eq!(window_span(&window.id), Some((1, 4)));
        let closed = window.close().expect("estimates");
        assert_eq!(closed.overall.n_green, 4);
        assert_eq!(closed.overall.n_selected, 3);
        // θ: units 1 (Y = 1 at 0.5) and 3 (Y = 0 at 1.0) are known; unit 4
        // is a null.
        let theta = &closed.overall.theta;
        assert_eq!(theta.estimate.n_audited, 2);
        let hajek = theta.estimate.theta_hajek.expect("a Hájek estimate");
        assert!((hajek - 2.0 / 3.0).abs() < 1e-12, "{hajek}");
        assert_eq!(theta.nulls, 1);
        let (low, high) = theta.bounds.expect("bounds");
        assert!(
            (low - 0.4).abs() < 1e-12 && (high - 0.8).abs() < 1e-12,
            "{low} {high}"
        );
        assert!(theta.ucb().is_some_and(|ucb| ucb > hajek));
        assert_eq!(closed.overall.gamma.estimate.events, 1);
        assert_eq!(closed.overall.omega.ucb(), None, "no W label is known");

        assert_eq!(closed.strata.len(), 2, "docs and implementer");
        assert_eq!(closed.strata[0].0.task_type, "docs");
        assert_eq!(closed.task_types["implementer"].n_green, 3);
        let models: Vec<&str> = closed
            .trust
            .iter()
            .map(|trust| trust.model.as_str())
            .collect();
        assert_eq!(models, ["glm-4.7", "kimi-k2"]);
        assert_eq!(closed.trust[0].harness, "roko");

        // Once the window's summary is in the ledger, unit 5 waits alone,
        // until the window is a day old.
        records.push(record(
            9,
            "2026-10-03T13:00:00Z",
            AuditEvent::Estimate {
                window: window.id.clone(),
                stratum: None,
                estimate: json!({}),
            },
        ));
        assert_eq!(closed_through(&records), 4);
        assert!(due_windows(&records, &config, now).is_empty());
        let later = now + chrono::Duration::hours(24);
        let due = due_windows(&records, &config, later);
        assert_eq!(due.len(), 1);
        assert_eq!(due[0].id, "window:5-5");
    }

    #[test]
    fn units_outside_phase_b_count_in_n_only() {
        let mut records = vec![
            selection(1, 0, "implementer", "glm-4.7", 0.5, true),
            selection(2, 1, "implementer", "glm-4.7", 0.5, true),
        ];
        records.push(result(3, 1, labels(Some(false), Some(false)), 0.25));
        let mut undrawn = result(4, 2, labels(Some(false), Some(false)), 0.25);
        if let AuditEvent::Result { checks, .. } = &mut undrawn.event {
            *checks = json!({ "phase_b": { "pi_b": 0.5, "drawn": false } });
        }
        records.push(undrawn);
        let all = units(&records);
        assert_eq!(
            all[0].sampled(),
            Some((0.25, labels(Some(false), Some(false))))
        );
        assert_eq!(all[1].sampled(), None, "phase B did not draw it");
        let group: Vec<&WindowUnit> = all.iter().collect();
        let estimate = group_estimate(&group).expect("an estimate");
        assert_eq!(estimate.n_green, 2);
        assert_eq!(estimate.theta.estimate.n_audited, 1);
        assert_eq!(estimate.theta.nulls, 0);
    }
}
