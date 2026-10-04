//! DP5 (S05 §4.6, §5; 7134): an audited attempt's verified-success label,
//! the `vs.label` row, and what the self-model learns from it.
//!
//! After each `audit.result`, the worker writes the attempt's `vs.label` row,
//! `vs_source` audit, with π the result's π_i·π_B, to the vault ledger and to
//! the run's `.roko/runs/<run>/labels.jsonl`; S01's attempt key joins it to
//! the attempt. Its checks come from the audit (S05 §0.1): completion is the
//! green verdict and its result tree, visible_clean is A2, hidden is B1, and
//! integrity is G. Strict VS is 1 only when every check is known and passes,
//! so a unit phase B did not draw, which has no B1, has an unknown VS unless
//! another check failed. `prediction_id` names M3's forecast of the attempt,
//! which the lottery noted when its prediction row was logged, and is null
//! when M3 made none.
//!
//! A known VS teaches the run's self-model with weight 1/π, through S04's
//! late-label hook (6129, [`VsLearner`]): its target is P(VS), not P(gate
//! pass), and the weight undoes the lottery's tilt. A unit the lottery did not
//! select is never audited, so it teaches nothing.

use std::io::Write as _;
use std::path::{Path, PathBuf};

use roko_core::audit_types::{
    AuditLabels, HiddenResult, Integrity, LabelCosts, LabelFinding, VsLabel, VsSource,
};
use roko_gate::audit::ledger::{AuditEvent, AuditLedger};
use roko_learn::self_model::LabelSource;
use serde_json::{Map, Value};

use super::worker::AuditUnit;
use crate::graph_task_dispatch::self_model::SelfModelRuntime;

/// The file of a run's `vs.label` rows.
pub const LABELS_FILE: &str = "labels.jsonl";

/// What learns audited VS labels: the run's self-model, through S04's
/// late-label hook (6129). Its false-green risk tilts the lottery (6132).
pub trait VsLearner: Send + Sync {
    /// Teach the attempt `attempt_key`'s VS label `vs` with weight 1/π;
    /// `false` when the learner does not know the attempt.
    fn learn_vs(&self, attempt_key: &str, vs: bool, weight: f64) -> bool;

    /// r, P(false green) of the chain `chain_key`'s last pass that stood:
    /// the `risk_fg` S05's tilt draws by; `None` without one.
    fn false_green_risk(&self, _chain_key: &str) -> Option<f64> {
        None
    }
}

impl VsLearner for SelfModelRuntime {
    fn learn_vs(&self, attempt_key: &str, vs: bool, weight: f64) -> bool {
        self.observe_label(attempt_key, vs, weight, LabelSource::Vs)
    }

    fn false_green_risk(&self, chain_key: &str) -> Option<f64> {
        self.risk_fg(chain_key)
    }
}

/// What an audit appended to `audit.result` about a unit.
#[derive(Debug, Clone, Copy)]
pub struct AuditReport<'a> {
    /// Its labels.
    pub labels: AuditLabels,
    /// Each check's detail.
    pub checks: &'a Map<String, Value>,
    /// Phase A's findings, one line each.
    pub findings: &'a [String],
    /// π_i·π_B, when the unit had trees to draw phase B on.
    pub pi_eff: Option<f64>,
    /// The audit's spend.
    pub cost_usd: f64,
}

/// The `vs.label` row of `unit`'s audit (S05 §5), `vs_source` audit.
#[must_use]
pub fn vs_label(unit: &AuditUnit, report: &AuditReport<'_>) -> VsLabel {
    let checks = report.checks;
    let visible_clean = checks
        .get("a2")
        .and_then(|a2| a2["y"].as_bool())
        .map(|failed| !failed);
    let hidden = checks.get("b1").and_then(|b1| {
        let failed = b1["y"].as_bool()?;
        Some(HiddenResult {
            suite: b1["suite_id"].as_str()?.to_string(),
            n: 1,
            failed: u32::from(failed),
            n_known: Some(false),
        })
    });
    let integrity = Integrity {
        g: report.labels.g.map(u8::from),
        findings: report.findings.iter().map(|line| finding(line)).collect(),
    };
    let completion = unit.result_tree.as_ref().map(|_| true);
    let parts = [
        completion,
        visible_clean,
        hidden.as_ref().map(|hidden| hidden.failed == 0),
        integrity.g.map(|g| g == 0),
    ];
    let vs = parts.iter().all(|part| *part == Some(true));
    let lenient = parts[..3].iter().all(|part| *part == Some(true));
    VsLabel {
        ev: VsLabel::EV.to_string(),
        run_id: unit.run_id.clone(),
        attempt_key: unit.attempt_key.clone(),
        task_id: unit.task_id.clone(),
        seed: None,
        arm: "prod".to_string(),
        prediction_id: unit.prediction_id.clone(),
        vs_source: VsSource::Audit,
        pi: report.pi_eff.unwrap_or(unit.pi),
        verdict: None,
        final_commit: unit.result_tree.clone(),
        completion,
        visible_clean,
        hidden,
        integrity,
        honeypot: None,
        vs: u8::from(vs),
        vs_lenient: u8::from(lenient),
        unknown: parts.contains(&None),
        cost_usd: LabelCosts {
            audit: Some(report.cost_usd),
            ..LabelCosts::default()
        },
        label_rule: VsLabel::LABEL_RULE.to_string(),
        gold: None,
        battery: None,
    }
}

/// The VS a row settles: 0 once a check failed, 1 once every check passed,
/// and `None` while a check it still needs could not run.
#[must_use]
pub fn known_vs(row: &VsLabel) -> Option<bool> {
    if row.vs == 1 {
        return Some(true);
    }
    let failed = [
        row.completion,
        row.visible_clean,
        row.hidden.as_ref().map(|hidden| hidden.failed == 0),
        row.integrity.g.map(|g| g == 0),
    ]
    .contains(&Some(false));
    failed.then_some(false)
}

/// One A1 finding line, `<kind> `<path>`: <detail>`, as a label finding.
fn finding(line: &str) -> LabelFinding {
    let kind = line.split_whitespace().next().unwrap_or_default();
    let mut detail = Map::new();
    if let Some((_, rest)) = line.split_once('`')
        && let Some((path, _)) = rest.split_once('`')
    {
        detail.insert("path".to_string(), Value::from(path));
    }
    detail.insert("finding".to_string(), Value::from(line));
    LabelFinding {
        kind: kind.to_string(),
        detail,
    }
}

/// The run's `vs.label` file in the workspace at `workdir`.
#[must_use]
pub fn labels_path(workdir: &Path, run_id: &str) -> PathBuf {
    roko_fs::RokoLayout::for_project(workdir)
        .run_dir(run_id)
        .join(LABELS_FILE)
}

/// DP5: append `row` to `ledger` and to its run's labels file in `workdir`,
/// and teach `learner` its VS, when known, with weight 1/π. Returns whether
/// the learner learned it.
///
/// # Errors
///
/// The ledger or the labels file cannot be written.
pub fn record_label(
    ledger: &mut AuditLedger,
    workdir: &Path,
    row: &VsLabel,
    learner: Option<&dyn VsLearner>,
) -> std::io::Result<bool> {
    ledger.append(AuditEvent::VsLabel {
        attempt_key: row.attempt_key.clone(),
        row: Box::new(row.clone()),
    })?;
    let path = labels_path(workdir, &row.run_id);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut line = serde_json::to_string(row)?;
    line.push('\n');
    std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)?
        .write_all(line.as_bytes())?;
    let (Some(learner), Some(vs)) = (learner, known_vs(row)) else {
        return Ok(false);
    };
    Ok(learner.learn_vs(&row.attempt_key, vs, 1.0 / row.pi))
}

#[cfg(test)]
mod tests {
    use roko_core::audit_home::AuditVault;
    use roko_gate::audit::ledger::records;
    use serde_json::json;

    use super::*;
    use crate::audit::worker::AuditTask;

    /// A fake self-model that keeps every label it is taught.
    #[derive(Default)]
    struct Learner {
        taught: parking_lot::Mutex<Vec<(String, bool, f64)>>,
    }

    impl VsLearner for Learner {
        fn learn_vs(&self, attempt_key: &str, vs: bool, weight: f64) -> bool {
            let taught = (attempt_key.to_string(), vs, weight);
            self.taught.lock().push(taught);
            true
        }
    }

    fn unit(n: u32, pi: f64) -> AuditUnit {
        AuditUnit {
            sel_id: format!("sel-{n}"),
            attempt_key: format!("run-1:plan:t{n}:1"),
            run_id: "run-1".to_string(),
            plan_id: "plan".to_string(),
            task_id: format!("t{n}"),
            pi,
            base_tree: Some("base".to_string()),
            result_tree: Some("result".to_string()),
            model: "glm-4.7".to_string(),
            prediction_id: None,
            task: AuditTask::default(),
        }
    }

    /// A unit audited with A2 and, when phase B drew it, B1.
    fn checks(a2_failed: bool, b1_failed: Option<bool>) -> Map<String, Value> {
        let mut checks = Map::new();
        checks.insert("a2".into(), json!({ "y": a2_failed }));
        if let Some(failed) = b1_failed {
            checks.insert("b1".into(), json!({ "suite_id": "hs-1", "y": failed }));
        }
        checks
    }

    /// DP5: of a run's six green units, the lottery selects four. Each
    /// audit writes its `vs.label` row to the ledger and the run's labels
    /// file, and a known VS teaches the self-model with weight 1/π, so the
    /// weights undo the lottery's tilt; a unit whose VS stays unknown
    /// teaches nothing, and so do the two the lottery left alone.
    #[test]
    fn audit_labels_correct_the_self_model_with_ipw_weights() {
        let temp = tempfile::tempdir().expect("tempdir");
        let workspace = temp.path().join("repo");
        std::fs::create_dir_all(&workspace).expect("mkdir");
        let home = temp.path().join("vault");
        let vault = AuditVault::resolve_with(&workspace, Some(&home), None).expect("a vault");
        let mut ledger = AuditLedger::open(&vault).expect("a ledger");
        let learner = Learner::default();
        let clean = AuditLabels {
            y: Some(false),
            g: Some(false),
            w: None,
        };
        let gamed = AuditLabels {
            g: Some(true),
            ..clean
        };
        // Units 1 to 4 are selected: 1 passes every check, 2 passes A2 but
        // phase B did not draw it, 3 games its tests, and 4 fails B1. Units
        // 5 and 6 were not selected, so no audit reports on them.
        let audits = [
            (unit(1, 0.25), clean, checks(false, Some(false)), Some(0.25)),
            (unit(2, 0.25), clean, checks(false, None), Some(0.125)),
            (unit(3, 0.5), gamed, checks(false, None), Some(0.5)),
            (unit(4, 1.0), clean, checks(false, Some(true)), Some(1.0)),
        ];
        let mut learned = Vec::new();
        for (unit, labels, checks, pi_eff) in &audits {
            let report = AuditReport {
                labels: *labels,
                checks,
                findings: &[],
                pi_eff: *pi_eff,
                cost_usd: 0.01,
            };
            let row = vs_label(unit, &report);
            assert_eq!(row.vs_source, VsSource::Audit);
            let taught = record_label(&mut ledger, &workspace, &row, Some(&learner));
            learned.push(taught.expect("the label is written"));
        }
        assert_eq!(learned, [true, false, true, true]);
        let taught = learner.taught.lock().clone();
        assert_eq!(
            taught,
            [
                ("run-1:plan:t1:1".to_string(), true, 4.0),
                ("run-1:plan:t3:1".to_string(), false, 2.0),
                ("run-1:plan:t4:1".to_string(), false, 1.0),
            ]
        );

        // Every audited unit has its row in the vault ledger and the run's
        // labels file, unit 2's with VS unknown.
        let rows: Vec<VsLabel> = records(ledger.dir())
            .expect("the ledger")
            .into_iter()
            .filter_map(|record| match record.event {
                AuditEvent::VsLabel { row, .. } => Some(*row),
                _ => None,
            })
            .collect();
        assert_eq!(rows.len(), 4);
        assert!(rows[1].unknown && rows[1].vs == 0, "{:?}", rows[1]);
        assert_eq!(known_vs(&rows[1]), None);
        assert!((rows[1].pi - 0.125).abs() < 1e-12);
        let text = std::fs::read_to_string(labels_path(&workspace, "run-1")).expect("the file");
        let file: Vec<VsLabel> = text
            .lines()
            .map(|line| serde_json::from_str(line).expect("a vs.label row"))
            .collect();
        assert_eq!(file, rows);
    }
}
