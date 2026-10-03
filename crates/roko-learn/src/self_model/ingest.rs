//! Read S01 run records (`.roko/runs/<run_id>/attempts.jsonl`) into labelled self-model
//! units: backlog task 6109.
//!
//! Every Graph attempt writes a `roko.attempt_open/1` line before its prompt is assembled and a
//! `roko.verdict/1` line when it settles. [`read_runs`] joins the two by attempt key. An open
//! line with no verdict is an abandoned attempt: it is counted and skipped. The label is the
//! verdict's `learning_label` (S01 §4.1): `1` a pass, `0` an agent failure, and none for an
//! unverified attempt or a provider or harness outcome. An attempt a provider failover
//! substituted is marked and kept out of training, as `RoutingObservationSink` does.
//!
//! Read-only: nothing here writes under `.roko/`.

use std::collections::{BTreeMap, HashMap};
use std::path::Path;

use serde::Serialize;

use super::{ArmKey, Label, LabelSource, Unit};
use crate::telemetry::records::{ATTEMPT_OPEN_SCHEMA, VERDICT_SCHEMA};
use crate::telemetry::{AttemptKey, AttemptOpenRecord, AttemptOutcome, AttemptVerdictRecord};

/// What a read of run records or legacy logs found: the labelled units, and an audit of
/// everything read, so that local data is never mistaken for evidence (S04 §0).
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct IngestReport {
    /// The units, labelled or not, in a stable order.
    pub units: Vec<Unit>,
    /// The counts behind them.
    pub audit: DataAudit,
}

impl IngestReport {
    /// The units the self-model may train on: labelled, and run on the arm that was asked for.
    pub fn training_units(&self) -> impl Iterator<Item = &Unit> {
        self.units
            .iter()
            .filter(|unit| !unit.failover && unit.label.y_gate.is_some())
    }
}

/// The counts of an [`IngestReport`].
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct DataAudit {
    /// Lines or rows read.
    pub rows: usize,
    /// Lines or rows that did not parse.
    pub unreadable: usize,
    /// Attempts found.
    pub attempts: usize,
    /// Attempts opened and never settled, skipped.
    pub abandoned: usize,
    /// Units with a gate label.
    pub labelled: usize,
    /// Settled attempts with no label: unverified, provider or harness outcomes.
    pub unlabelled: usize,
    /// Labelled units a provider failover substituted, kept out of training.
    pub failover: usize,
    /// Labelled units per label source (`pre_gate_success`, `gate_passed`, `vs`).
    pub label_sources: BTreeMap<String, usize>,
    /// Labelled units per cost source (S01 §4.4).
    pub cost_sources: BTreeMap<String, usize>,
}

impl DataAudit {
    /// Count `unit`, which carries a label.
    pub(crate) fn count_labelled(&mut self, unit: &Unit) {
        self.labelled += 1;
        if unit.failover {
            self.failover += 1;
        }
        *self
            .label_sources
            .entry(snake_name(&unit.label.source))
            .or_default() += 1;
        *self
            .cost_sources
            .entry(snake_name(&unit.cost_source))
            .or_default() += 1;
    }
}

/// The snake_case name `value` serialises to.
pub(crate) fn snake_name<T: Serialize>(value: &T) -> String {
    serde_json::to_value(value)
        .ok()
        .and_then(|value| value.as_str().map(str::to_string))
        .unwrap_or_default()
}

/// One run's attempt lines, joined by attempt key.
#[derive(Default)]
struct RunLines {
    opens: HashMap<String, AttemptOpenRecord>,
    verdicts: HashMap<String, AttemptVerdictRecord>,
}

/// Read every run directory under `runs_dir` (`.roko/runs`) into self-model units. A missing
/// directory holds no runs.
pub fn read_runs(runs_dir: &Path) -> std::io::Result<IngestReport> {
    let mut report = IngestReport::default();
    let mut run_dirs: Vec<_> = match std::fs::read_dir(runs_dir) {
        Ok(entries) => entries
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.is_dir())
            .collect(),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(report),
        Err(error) => return Err(error),
    };
    run_dirs.sort();
    for run_dir in run_dirs {
        let path = run_dir.join("attempts.jsonl");
        let text = match std::fs::read_to_string(&path) {
            Ok(text) => text,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => return Err(error),
        };
        read_run(&text, &mut report);
    }
    Ok(report)
}

/// Add the attempts of one run's `attempts.jsonl` text to `report`.
fn read_run(text: &str, report: &mut IngestReport) {
    let mut lines = RunLines::default();
    for line in text.lines().filter(|line| !line.trim().is_empty()) {
        report.audit.rows += 1;
        if !lines.add(line) {
            report.audit.unreadable += 1;
        }
    }
    let attempt_keys: Vec<&String> = {
        let mut keys: Vec<&String> = lines.opens.keys().chain(lines.verdicts.keys()).collect();
        keys.sort();
        keys.dedup();
        keys
    };
    report.audit.attempts += attempt_keys.len();
    report.audit.abandoned += attempt_keys
        .iter()
        .filter(|key| !lines.verdicts.contains_key(key.as_str()))
        .count();

    // Each chain's settled attempts in ordinal order, so an attempt can see the one before.
    let mut chains: BTreeMap<String, Vec<&AttemptVerdictRecord>> = BTreeMap::new();
    for verdict in lines.verdicts.values() {
        chains
            .entry(verdict.identity.chain_key.clone())
            .or_default()
            .push(verdict);
    }
    for verdicts in chains.values_mut() {
        verdicts.sort_by_key(|verdict| verdict.identity.attempt);
        let mut previous: Option<&AttemptVerdictRecord> = None;
        for &verdict in verdicts.iter() {
            let open = lines.opens.get(&verdict.identity.attempt_key);
            match unit_of(verdict, open, previous) {
                Some(unit) => {
                    report.audit.count_labelled(&unit);
                    report.units.push(unit);
                }
                None => report.audit.unlabelled += 1,
            }
            previous = Some(verdict);
        }
    }
}

impl RunLines {
    /// Keep `line`'s record; `false` when it does not parse.
    fn add(&mut self, line: &str) -> bool {
        let Ok(value) = serde_json::from_str::<serde_json::Value>(line) else {
            return false;
        };
        match value.get("schema_version").and_then(serde_json::Value::as_str) {
            Some(ATTEMPT_OPEN_SCHEMA) => {
                let Ok(open) = serde_json::from_value::<AttemptOpenRecord>(value) else {
                    return false;
                };
                self.opens.insert(open.identity.attempt_key.clone(), open);
                true
            }
            Some(VERDICT_SCHEMA) => {
                let Ok(verdict) = serde_json::from_value::<AttemptVerdictRecord>(value) else {
                    return false;
                };
                self.verdicts
                    .insert(verdict.identity.attempt_key.clone(), verdict);
                true
            }
            // Decisions and other records share no line with attempts; skip them.
            Some(_) => true,
            None => false,
        }
    }
}

/// The unit of `verdict`, whose open line is `open` and whose chain's previous attempt is
/// `previous`; `None` when the verdict carries no label.
fn unit_of(
    verdict: &AttemptVerdictRecord,
    open: Option<&AttemptOpenRecord>,
    previous: Option<&AttemptVerdictRecord>,
) -> Option<Unit> {
    let passed = verdict.learning_success()?;
    let identity = &verdict.identity;
    let executed = &verdict.executed;
    let model = executed
        .model_dispatched
        .clone()
        .or_else(|| executed.model_requested.clone())
        .unwrap_or_else(|| "unknown".to_string());
    let provider = executed
        .provider
        .clone()
        .unwrap_or_else(|| "unknown".to_string());
    let tier = open
        .and_then(|open| open.tier.clone())
        .unwrap_or_else(|| "unknown".to_string());
    let failed_before = previous.filter(|previous| !previous_passed(previous));
    Some(Unit {
        attempt_key: AttemptKey::new(
            identity.run_id.clone(),
            identity.plan_id.clone(),
            identity.task_id.clone(),
            identity.attempt,
        ),
        plan_id: identity.plan_id.clone(),
        task_id: identity.task_id.clone(),
        role: open
            .and_then(|open| open.role.clone())
            .unwrap_or_else(|| "unknown".to_string()),
        // Plan tasks have no benchmark family: the tier stands in (S04 §4.2).
        family: tier.clone(),
        tier,
        arm: ArmKey::roko(provider, model),
        attempt: identity.attempt,
        prior_failure: failed_before.is_some(),
        failure_class: failed_before.and_then(failure_class_of),
        label: Label {
            y_gate: Some(passed),
            y_vs: None,
            weight: 1.0,
            source: LabelSource::GatePassed,
        },
        api_equiv_usd: verdict.cost.api_equiv_usd,
        cost_source: verdict.cost.source,
        latency_s: latency_s(verdict),
        failover: !executed.failover_chain.is_empty(),
    })
}

/// Whether an earlier attempt of the chain passed.
fn previous_passed(verdict: &AttemptVerdictRecord) -> bool {
    matches!(
        verdict.outcome,
        AttemptOutcome::Passed | AttemptOutcome::AlreadySatisfied
    )
}

/// A failed attempt's error class: its failing rung, else its outcome.
fn failure_class_of(verdict: &AttemptVerdictRecord) -> Option<String> {
    let rung = verdict
        .failure_class
        .as_ref()
        .and_then(|class| class.rung.clone());
    Some(rung.unwrap_or_else(|| snake_name(&verdict.outcome)))
}

/// The attempt's wall time in seconds: open to settle, else the provider call.
fn latency_s(verdict: &AttemptVerdictRecord) -> Option<f64> {
    let timing = &verdict.timing;
    let span = timing
        .attempt_started_at
        .zip(timing.settled_at)
        .or_else(|| timing.dispatch_started_at.zip(timing.dispatch_ended_at))?;
    let millis = span.1.checked_sub(span.0).filter(|millis| *millis >= 0)?;
    Some(millis as f64 / 1_000.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::telemetry::CostSource;

    fn fixture_runs() -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/self_model/runs")
    }

    /// The fixture holds two runs. `run-a`: T1 passes, T2 fails at its test rung, T3 is
    /// unverified, a provider failover substituted T4's model, and T5 was opened and never
    /// settled. `run-b` was resumed: T1's first attempt failed in invocation 1 and its second
    /// passed in invocation 2.
    #[test]
    fn ingest_joins_open_and_verdict_lines_by_attempt_key() {
        let report = read_runs(&fixture_runs()).expect("read the fixture runs");
        let audit = &report.audit;
        assert_eq!(audit.rows, 13);
        assert_eq!(audit.unreadable, 0);
        assert_eq!(audit.attempts, 7);
        assert_eq!(audit.abandoned, 1);
        assert_eq!(audit.labelled, 5);
        assert_eq!(audit.unlabelled, 1);
        assert_eq!(audit.failover, 1);
        assert_eq!(audit.label_sources.get("gate_passed"), Some(&5));
        assert_eq!(audit.cost_sources.get("provider_usage"), Some(&3));
        assert_eq!(audit.cost_sources.get("cli_usage"), Some(&2));

        let unit = |key: &str| {
            report
                .units
                .iter()
                .find(|unit| unit.attempt_key.attempt_key() == key)
                .unwrap_or_else(|| panic!("no unit {key}"))
        };
        let pass = unit("run-a:plan-1:T1:1");
        assert_eq!(pass.label.y_gate, Some(true));
        assert_eq!(pass.label.source, LabelSource::GatePassed);
        assert_eq!(pass.arm.to_string(), "roko/cerebras/gpt-oss-120b@default#V0");
        assert_eq!(pass.role, "implementer");
        assert_eq!(pass.tier, "focused");
        assert_eq!(pass.family, "focused");
        assert_eq!(pass.api_equiv_usd, Some(0.0155));
        assert_eq!(pass.cost_source, CostSource::ProviderUsage);
        assert_eq!(pass.latency_s, Some(60.0));
        assert!(!pass.prior_failure);

        assert_eq!(unit("run-a:plan-1:T2:1").label.y_gate, Some(false));
        assert!(unit("run-a:plan-1:T4:1").failover);
        assert!(
            report
                .units
                .iter()
                .all(|unit| unit.task_id != "T3" && unit.task_id != "T5"),
            "unlabelled and abandoned attempts are not units"
        );

        let retry = unit("run-b:plan-2:T1:2");
        assert_eq!(retry.label.y_gate, Some(true));
        assert!(retry.prior_failure);
        assert_eq!(retry.failure_class.as_deref(), Some("verify:0/test"));
        assert_eq!(retry.cost_source, CostSource::CliUsage);

        assert_eq!(report.training_units().count(), 4, "the failover unit is left out");
    }

    #[test]
    fn a_missing_runs_directory_holds_no_runs() {
        let empty = tempfile::tempdir().expect("tempdir");
        let report = read_runs(&empty.path().join("runs")).expect("no runs");
        assert_eq!(report, IngestReport::default());
    }
}
