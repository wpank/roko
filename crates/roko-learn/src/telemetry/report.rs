//! Read-only checks and reports over a run's attempt records (S01 P0-13,
//! §5.9, §7): what `roko learn telemetry check` and `route-report` print.
//!
//! Everything here reads files alone and writes nothing.
//! [`RunRecords::load`] parses one run directory's `attempts.jsonl`,
//! `decisions.jsonl` (route and content decisions), `exposures.jsonl` and
//! `census.json`, and [`LegacyRows::load`] finds the run's attempt keys in
//! the logs that predate S01 (`learn/efficiency.jsonl`, `learn/costs.jsonl`,
//! `episodes.jsonl`). [`check`] validates one run; [`route_report`] counts
//! routing outcomes per decision source over any number of runs; and
//! [`srm_check`] tests each randomised layer's arm split against its logged
//! propensities (S02 SC3).

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::path::Path;

use chrono::{DateTime, Utc};
use roko_fs::layout::RokoLayout;
use serde::Serialize;
use serde_json::Value;

use super::assign::{Arm, Assignment};
use super::census::{CENSUS_FILE, CensusReport};
use super::manifest::AttemptTally;
use super::records::{
    ATTEMPT_OPEN_SCHEMA, AttemptKey, AttemptOpenRecord, AttemptPredictionRecord,
    AttemptVerdictRecord, ContentDecisionPoint, ContentDecisionRecord, DECISION_SCHEMA,
    DecisionSource, EXPOSURE_SCHEMA, ExecutedModel, ExposureRecord, HARNESS_POLICY_DECISION_POINT,
    HarnessPolicyDecisionRecord, PLACEBO_DECISION_POINT, PREDICTION_SCHEMA, PlaceboDecisionRecord,
    RunFile, Stamped, VERDICT_SCHEMA,
};
use crate::error::LearnError;
use crate::loop_audit::arm_set::{
    ArmSet, FORCED_CONDITION, MAXIMIZE_CONDITION, NORMAL_CONDITION, PLACEBO_LAYER,
};
use crate::loop_audit::cs::SrmEvalue;
use crate::routing_log::{ROUTE_DECISION_POINT, RoutingDecisionLog};
use crate::section_effect::SectionDecision;

/// The source [`route_report`] files an attempt under when no route
/// decision names one: an attempt that never routed (a T0 reflex, a harness
/// failure before planning), or one from a run written before Graph
/// dispatch recorded route decisions (S01 P0-8).
pub const UNKNOWN_SOURCE: &str = "unknown";

// ── Reading a run ─────────────────────────────────────────────────────

/// The records of one run directory, `.roko/runs/<run_id>/`.
#[derive(Debug, Clone, Default)]
pub struct RunRecords {
    /// The run id: the directory's name.
    pub run_id: String,
    /// Attempt-open lines, in file order.
    pub opens: Vec<Stamped<AttemptOpenRecord>>,
    /// Verdict lines, in file order.
    pub verdicts: Vec<Stamped<AttemptVerdictRecord>>,
    /// Route decision rows, in file order.
    pub decisions: Vec<Stamped<RoutingDecisionLog>>,
    /// Content decision rows (knowledge, playbooks, sections, error
    /// patterns), in file order.
    pub content_decisions: Vec<Stamped<ContentDecisionRecord>>,
    /// Placebo decision rows (S03 §4.3), one per attempt, in file order.
    pub placebo_decisions: Vec<Stamped<PlaceboDecisionRecord>>,
    /// M1's `harness_policy` decision rows (A-DEC-H), one per attempt of a
    /// run with an M1 sink, in file order.
    pub harness_decisions: Vec<Stamped<HarnessPolicyDecisionRecord>>,
    /// Exposure rows, in file order.
    pub exposures: Vec<Stamped<ExposureRecord>>,
    /// Self-model prediction rows (S01 §5.6), in file order.
    pub predictions: Vec<Stamped<AttemptPredictionRecord>>,
    /// The run's wiring census; `None` for a run that has none.
    pub census: Option<CensusReport>,
    /// Lines that are not a valid record of their file, as
    /// `file:line: reason`.
    pub invalid: Vec<String>,
    /// The `seq` of every line that has one, with its file, in file order.
    pub seqs: Vec<(RunFile, u64)>,
}

impl RunRecords {
    /// Parse the run files in `run_dir`. Missing files hold no lines; a line
    /// that does not parse as a record of its file is kept in
    /// [`Self::invalid`].
    ///
    /// # Errors
    ///
    /// Returns an error when a run file exists but cannot be read.
    pub fn load(run_dir: &Path) -> Result<Self, LearnError> {
        let run_id = run_dir
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default()
            .to_string();
        let mut records = Self {
            run_id,
            ..Self::default()
        };
        for file in RunFile::ALL {
            for (index, line) in read_lines(&file.path_in(run_dir))?.iter().enumerate() {
                if !line.trim().is_empty() {
                    records.add_line(file, index + 1, line);
                }
            }
        }
        match CensusReport::load(run_dir) {
            Ok(census) => records.census = census,
            Err(error) => records.invalid.push(format!("{CENSUS_FILE}: {error}")),
        }
        Ok(records)
    }

    fn add_line(&mut self, file: RunFile, number: usize, line: &str) {
        let at = format!("{}:{number}", file.file_name());
        let value: Value = match serde_json::from_str(line) {
            Ok(value) => value,
            Err(error) => {
                self.invalid.push(format!("{at}: not JSON: {error}"));
                return;
            }
        };
        if let Some(seq) = value.get("seq").and_then(Value::as_u64) {
            self.seqs.push((file, seq));
        }
        let schema = value
            .get("schema_version")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        let parsed = match (file, schema.as_str()) {
            (RunFile::Attempts, ATTEMPT_OPEN_SCHEMA) => {
                serde_json::from_value(value).map(|record| self.opens.push(record))
            }
            (RunFile::Attempts, VERDICT_SCHEMA) => {
                serde_json::from_value(value).map(|record| self.verdicts.push(record))
            }
            // A route row's `decision_point` is `route`; rows written before
            // the field existed have none, and are route rows too.
            (RunFile::Decisions, DECISION_SCHEMA) if is_route_decision(&value) => {
                serde_json::from_value(value).map(|record| self.decisions.push(record))
            }
            (RunFile::Decisions, DECISION_SCHEMA) if is_placebo_decision(&value) => {
                serde_json::from_value(value).map(|record| self.placebo_decisions.push(record))
            }
            (RunFile::Decisions, DECISION_SCHEMA) if is_harness_decision(&value) => {
                serde_json::from_value(value).map(|record| self.harness_decisions.push(record))
            }
            (RunFile::Decisions, DECISION_SCHEMA) => {
                serde_json::from_value(value).map(|record| self.content_decisions.push(record))
            }
            (RunFile::Exposures, EXPOSURE_SCHEMA) => {
                serde_json::from_value(value).map(|record| self.exposures.push(record))
            }
            (RunFile::Predictions, PREDICTION_SCHEMA) => {
                serde_json::from_value(value).map(|record| self.predictions.push(record))
            }
            _ => {
                self.invalid
                    .push(format!("{at}: unexpected schema_version {schema:?}"));
                return;
            }
        };
        if let Err(error) = parsed {
            self.invalid
                .push(format!("{at}: invalid {schema}: {error}"));
        }
    }

    /// The attempts the run's `attempts.jsonl` opened, settled and abandoned.
    #[must_use]
    pub fn tally(&self) -> AttemptTally {
        let opens = self.opens.iter().map(|line| {
            let key = line.record.identity.attempt_key.as_str();
            (ATTEMPT_OPEN_SCHEMA, key)
        });
        let verdicts = self.verdicts.iter().map(|line| {
            let key = line.record.identity.attempt_key.as_str();
            (VERDICT_SCHEMA, key)
        });
        AttemptTally::from_lines(opens.chain(verdicts))
    }
}

/// Whether a decision row is a route row: its `decision_point` is `route`,
/// or it has none.
fn is_route_decision(value: &Value) -> bool {
    value
        .get("decision_point")
        .and_then(Value::as_str)
        .is_none_or(|point| point == ROUTE_DECISION_POINT)
}

/// Whether a decision row is a placebo row (S03 §4.3).
fn is_placebo_decision(value: &Value) -> bool {
    value.get("decision_point").and_then(Value::as_str) == Some(PLACEBO_DECISION_POINT)
}

/// Whether a decision row is M1's `harness_policy` row (A-DEC-H).
fn is_harness_decision(value: &Value) -> bool {
    value.get("decision_point").and_then(Value::as_str) == Some(HARNESS_POLICY_DECISION_POINT)
}

/// One run's attempt keys in the logs that predate S01, which gained an
/// `attempt_key` (S01 §5): rows whose key names another run, or that have
/// none, are left out.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LegacyRows {
    /// Keys of the `learn/efficiency.jsonl` rows (a dispatch row, and a
    /// gate-pass row when the gate passed).
    pub efficiency: Vec<String>,
    /// Keys of the `learn/costs.jsonl` rows.
    pub costs: Vec<String>,
    /// Keys of the `episodes.jsonl` rows (`extra.attempt_key`): the
    /// workspace's, and a frozen run's own `runs/<run_id>/episodes.jsonl`.
    pub episodes: Vec<String>,
}

impl LegacyRows {
    /// Find run `run_id`'s rows in the logs of `layout`. A missing log holds
    /// none.
    ///
    /// # Errors
    ///
    /// Returns an error when a log exists but cannot be read.
    pub fn load(layout: &RokoLayout, run_id: &str) -> Result<Self, LearnError> {
        let learn_dir = layout.learn_dir();
        // A frozen run writes its episodes to its own run directory, not to
        // the workspace's log (gap-127263).
        let mut episodes = run_keys(
            &layout.root_episodes_path(),
            run_id,
            &["extra", "attempt_key"],
        )?;
        episodes.extend(run_keys(
            &layout.run_dir(run_id).join("episodes.jsonl"),
            run_id,
            &["extra", "attempt_key"],
        )?);
        Ok(Self {
            efficiency: run_keys(
                &learn_dir.join("efficiency.jsonl"),
                run_id,
                &["attempt_key"],
            )?,
            costs: run_keys(&learn_dir.join("costs.jsonl"), run_id, &["attempt_key"])?,
            episodes,
        })
    }
}

/// The attempt keys at `field_path` of the rows of `path` whose key names
/// run `run_id`.
fn run_keys(path: &Path, run_id: &str, field_path: &[&str]) -> Result<Vec<String>, LearnError> {
    let needle = format!("\"{run_id}:");
    let mut keys = Vec::new();
    for line in read_lines(path)? {
        if !line.contains(&needle) {
            continue;
        }
        let Ok(row) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        let key = field_path
            .iter()
            .try_fold(&row, |value, field| value.get(*field))
            .and_then(Value::as_str);
        if let Some(key) = key
            && AttemptKey::parse(key).is_some_and(|key| key.run_id == run_id)
        {
            keys.push(key.to_string());
        }
    }
    Ok(keys)
}

/// The lines of `path`; none when it does not exist.
fn read_lines(path: &Path) -> Result<Vec<String>, LearnError> {
    let io_error = |source: io::Error| LearnError::Io {
        path: path.display().to_string(),
        source,
    };
    let file = match File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(io_error(error)),
    };
    BufReader::new(file)
        .lines()
        .collect::<Result<_, _>>()
        .map_err(io_error)
}

// ── check ─────────────────────────────────────────────────────────────

/// How one legacy log joins a run's verdicts (S01 SC2 wants 100%).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct JoinCoverage {
    /// The log, e.g. `learn/costs.jsonl`.
    pub file: &'static str,
    /// Settled attempts of the run.
    pub verdicts: usize,
    /// Settled attempts with at least one row in the log.
    pub joined: usize,
    /// Settled attempts with no row in the log.
    pub missing: Vec<String>,
    /// Keys of rows that name the run but no settled attempt of it.
    pub orphans: Vec<String>,
}

/// What `roko learn telemetry check` found in one run.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct CheckReport {
    /// The run checked.
    pub run_id: String,
    /// Attempts opened, settled and abandoned (S01 SC1).
    pub attempts_opened: u64,
    /// See [`AttemptTally::settled`].
    pub attempts_settled: u64,
    /// See [`AttemptTally::abandoned`].
    pub attempts_abandoned: u64,
    /// Route decision rows.
    pub decisions: usize,
    /// Content decision rows.
    pub content_decisions: usize,
    /// Exposure rows.
    pub exposures: usize,
    /// Lines that are not valid records of their file.
    pub invalid_lines: Vec<String>,
    /// `settlement_id`s that settle more than once.
    pub duplicate_settlements: Vec<String>,
    /// Attempts with a verdict but no attempt-open line.
    pub unopened_verdicts: Vec<String>,
    /// Attempt keys of exposure rows whose attempt the run never opened.
    pub unopened_exposures: Vec<String>,
    /// `seq` ordering violations: repeated or decreasing `seq`, and a verdict
    /// that precedes its own attempt's open line, decisions or exposures.
    pub seq_violations: Vec<String>,
    /// The learning components the run's census lists as unwired; `None`
    /// when the run has no census.
    pub unwired_components: Option<Vec<String>>,
    /// What is worth knowing but fails nothing, e.g. a run with no census.
    pub warnings: Vec<String>,
    /// Join coverage of the efficiency, cost and episode logs.
    pub coverage: Vec<JoinCoverage>,
    /// Settled attempts per `cost.source`.
    pub cost_sources: BTreeMap<String, usize>,
}

impl CheckReport {
    /// Every failure the check found; empty when the run passes.
    #[must_use]
    pub fn failures(&self) -> Vec<String> {
        let mut failures: Vec<String> = self
            .invalid_lines
            .iter()
            .map(|line| format!("invalid line {line}"))
            .collect();
        failures.extend(
            self.duplicate_settlements
                .iter()
                .map(|id| format!("settled more than once: {id}")),
        );
        failures.extend(
            self.unopened_verdicts
                .iter()
                .map(|key| format!("verdict without an attempt-open line: {key}")),
        );
        failures.extend(
            self.unopened_exposures
                .iter()
                .map(|key| format!("exposure without an attempt-open line: {key}")),
        );
        failures.extend(self.seq_violations.iter().cloned());
        for coverage in &self.coverage {
            if !coverage.missing.is_empty() {
                failures.push(format!(
                    "{}: {} of {} settled attempts have no row",
                    coverage.file,
                    coverage.missing.len(),
                    coverage.verdicts
                ));
            }
            if !coverage.orphans.is_empty() {
                failures.push(format!(
                    "{}: {} row(s) name no settled attempt of the run",
                    coverage.file,
                    coverage.orphans.len()
                ));
            }
        }
        failures
    }

    /// Whether the run passed every check.
    #[must_use]
    pub fn passed(&self) -> bool {
        self.failures().is_empty()
    }
}

/// Check one run's records: schema validity, one settlement per attempt,
/// exposures that join an opened attempt, `seq` ordering, join coverage of
/// the legacy logs and the cost-source mix (S01 §7 criterion 2). It lists
/// the components the run's census has unwired, and warns of a run with no
/// census, since runs before it have none.
#[must_use]
pub fn check(records: &RunRecords, legacy: &LegacyRows) -> CheckReport {
    let tally = records.tally();
    let opened: BTreeSet<&str> = records
        .opens
        .iter()
        .map(|line| line.record.identity.attempt_key.as_str())
        .collect();
    let settled: BTreeSet<&str> = records
        .verdicts
        .iter()
        .map(|line| line.record.identity.attempt_key.as_str())
        .collect();

    let mut settlements: BTreeMap<&str, usize> = BTreeMap::new();
    for line in &records.verdicts {
        *settlements
            .entry(line.record.settlement_id.as_str())
            .or_default() += 1;
    }
    let duplicate_settlements = settlements
        .into_iter()
        .filter(|(_, count)| *count > 1)
        .map(|(id, _)| id.to_string())
        .collect();
    let unopened_verdicts = settled
        .difference(&opened)
        .map(|key| (*key).to_string())
        .collect();
    let unopened_exposures = records
        .exposures
        .iter()
        .map(|line| line.record.identity.attempt_key.as_str())
        .filter(|key| !opened.contains(key))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .map(str::to_string)
        .collect();

    let mut cost_sources = BTreeMap::new();
    for line in &records.verdicts {
        let source = serde_json::to_value(line.record.cost.source)
            .ok()
            .and_then(|source| source.as_str().map(str::to_string))
            .unwrap_or_else(|| UNKNOWN_SOURCE.to_string());
        *cost_sources.entry(source).or_default() += 1;
    }

    let coverage = [
        ("learn/efficiency.jsonl", &legacy.efficiency),
        ("learn/costs.jsonl", &legacy.costs),
        ("episodes.jsonl", &legacy.episodes),
    ]
    .into_iter()
    .map(|(file, keys)| join_coverage(file, &settled, keys))
    .collect();

    let unwired_components = records
        .census
        .as_ref()
        .map(|census| census.unwired().into_iter().map(str::to_string).collect());
    let mut warnings = Vec::new();
    if records.census.is_none() {
        warnings.push(format!(
            "no {CENSUS_FILE}: the run predates the wiring census"
        ));
    }

    CheckReport {
        run_id: records.run_id.clone(),
        attempts_opened: tally.opened,
        attempts_settled: tally.settled,
        attempts_abandoned: tally.abandoned,
        decisions: records.decisions.len(),
        content_decisions: records.content_decisions.len(),
        exposures: records.exposures.len(),
        invalid_lines: records.invalid.clone(),
        duplicate_settlements,
        unopened_verdicts,
        unopened_exposures,
        seq_violations: seq_violations(records),
        unwired_components,
        warnings,
        coverage,
        cost_sources,
    }
}

fn join_coverage(file: &'static str, settled: &BTreeSet<&str>, keys: &[String]) -> JoinCoverage {
    let rows: BTreeSet<&str> = keys.iter().map(String::as_str).collect();
    JoinCoverage {
        file,
        verdicts: settled.len(),
        joined: settled.intersection(&rows).count(),
        missing: settled
            .difference(&rows)
            .map(|key| (*key).to_string())
            .collect(),
        orphans: rows
            .difference(settled)
            .map(|key| (*key).to_string())
            .collect(),
    }
}

/// Repeated `seq`s across the run's files, a `seq` that does not grow along
/// its file, and a verdict whose `seq` does not follow its attempt's open
/// line, decisions and exposures.
fn seq_violations(records: &RunRecords) -> Vec<String> {
    let mut violations = Vec::new();
    let mut seen = BTreeSet::new();
    let mut last: HashMap<RunFile, u64> = HashMap::new();
    for (file, seq) in &records.seqs {
        if !seen.insert(*seq) {
            violations.push(format!("seq {seq} appears more than once"));
        }
        if let Some(previous) = last.insert(*file, *seq)
            && *seq <= previous
        {
            violations.push(format!(
                "{}: seq {seq} follows seq {previous}",
                file.file_name()
            ));
        }
    }
    let mut preceding: HashMap<&str, Vec<(&str, u64)>> = HashMap::new();
    for line in &records.opens {
        let key = line.record.identity.attempt_key.as_str();
        preceding
            .entry(key)
            .or_default()
            .push(("attempt-open line", line.seq));
    }
    for line in &records.decisions {
        if let Some(key) = &line.record.attempt_key {
            preceding
                .entry(key)
                .or_default()
                .push(("route decision", line.seq));
        }
    }
    for line in &records.content_decisions {
        let key = line.record.identity.attempt_key.as_str();
        preceding
            .entry(key)
            .or_default()
            .push(("content decision", line.seq));
    }
    for line in &records.exposures {
        let key = line.record.identity.attempt_key.as_str();
        preceding
            .entry(key)
            .or_default()
            .push(("exposure", line.seq));
    }
    for line in &records.verdicts {
        let key = line.record.identity.attempt_key.as_str();
        for (what, seq) in preceding.get(key).into_iter().flatten() {
            if *seq >= line.seq {
                violations.push(format!(
                    "{key}: verdict (seq {}) does not follow its {what} (seq {seq})",
                    line.seq
                ));
            }
        }
    }
    violations
}

// ── route-report ──────────────────────────────────────────────────────

/// Routing outcomes of the attempts of one decision source (S01 §7
/// criterion 4). Each list holds attempt keys, so every number traces to
/// its records (`--explain`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct RouteRow {
    /// The route decision's `source` (`router`, `task_hint`, `fallback`,
    /// ...), or [`UNKNOWN_SOURCE`].
    pub source: String,
    /// Settled attempts.
    pub attempts: Vec<String>,
    /// Attempts with learning label 1.
    pub passed: Vec<String>,
    /// Attempts with learning label 0.
    pub failed: Vec<String>,
    /// Attempts with a null learning label (unverified, infra and harness
    /// outcomes).
    pub unlabeled: Vec<String>,
    /// Attempts whose served model differs from the one requested, date
    /// suffixes aside.
    pub model_mismatch: Vec<String>,
    /// Attempts whose route decision names the default model.
    pub with_default: Vec<String>,
    /// Those of them whose pick differs from the default (ι).
    pub pick_not_default: Vec<String>,
    /// Attempts whose route decision records the router's own proposal
    /// (`proposals.learned`).
    pub with_learned: Vec<String>,
    /// Masked routes (`source = router` while the pick differs from the
    /// router's own proposal): `None` until a decision of this source
    /// records that proposal.
    pub masked: Option<Vec<String>>,
    /// Router-sourced attempts whose executed model is the router's own
    /// proposal (ε_honest's numerator): `None` for other sources and until a
    /// decision records that proposal.
    pub executed_learned: Option<Vec<String>>,
}

impl RouteRow {
    /// Share of the attempts with learning label 1; `None` without attempts.
    #[must_use]
    pub fn pass_rate(&self) -> Option<f64> {
        ratio(self.passed.len(), self.attempts.len())
    }

    /// ι: share of the attempts whose decision names a default that picked
    /// another model; `None` when no decision names one.
    #[must_use]
    pub fn iota(&self) -> Option<f64> {
        ratio(self.pick_not_default.len(), self.with_default.len())
    }

    /// ε_honest: share of the router-sourced attempts recording the router's
    /// own proposal whose executed model is that proposal; `None` for other
    /// sources and when no decision records one.
    #[must_use]
    pub fn eps_honest(&self) -> Option<f64> {
        let honest = self.executed_learned.as_ref()?;
        ratio(honest.len(), self.with_learned.len())
    }
}

/// `part / whole`, or `None` when `whole` is 0.
fn ratio(part: usize, whole: usize) -> Option<f64> {
    (whole > 0).then(|| part as f64 / whole as f64)
}

/// What `roko learn telemetry route-report` prints.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct RouteReport {
    /// The runs read.
    pub runs: Vec<String>,
    /// Route decision rows joined to a counted attempt.
    pub decisions: usize,
    /// One row per decision source, by source name.
    pub rows: Vec<RouteRow>,
}

impl RouteReport {
    /// Settled attempts counted over every source.
    #[must_use]
    pub fn attempts(&self) -> usize {
        self.rows.iter().map(|row| row.attempts.len()).sum()
    }
}

/// Count the settled attempts of `runs` per route decision source: how many
/// passed, failed or carry no label, how often the served model differed
/// from the requested one, ι where decisions name a default, and the masked
/// routes and ε_honest where they record the router's own proposal. With
/// `since`, only verdicts written at or after it count.
#[must_use]
pub fn route_report(runs: &[RunRecords], since: Option<DateTime<Utc>>) -> RouteReport {
    let decisions: HashMap<&str, &RoutingDecisionLog> = runs
        .iter()
        .flat_map(|run| &run.decisions)
        .filter_map(|line| Some((line.record.attempt_key.as_deref()?, &line.record)))
        .collect();
    let mut rows: BTreeMap<String, RouteRow> = BTreeMap::new();
    let mut joined_decisions = 0;
    for line in runs.iter().flat_map(|run| &run.verdicts) {
        if let Some(since) = since
            && !DateTime::parse_from_rfc3339(&line.ts).is_ok_and(|ts| ts >= since)
        {
            continue;
        }
        let verdict = &line.record;
        let key = verdict.identity.attempt_key.clone();
        let decision = decisions.get(key.as_str()).copied();
        joined_decisions += usize::from(decision.is_some());
        let source = decision
            .and_then(|decision| decision.source)
            .and_then(|source| serde_json::to_value(source).ok())
            .and_then(|source| source.as_str().map(str::to_string))
            .unwrap_or_else(|| UNKNOWN_SOURCE.to_string());
        let row = rows.entry(source.clone()).or_insert_with(|| RouteRow {
            source,
            ..RouteRow::default()
        });
        match verdict.learning_label {
            Some(1) => row.passed.push(key.clone()),
            Some(_) => row.failed.push(key.clone()),
            None => row.unlabeled.push(key.clone()),
        }
        let executed = &verdict.executed;
        if let (Some(requested), Some(reported)) =
            (&executed.model_requested, &executed.model_reported)
            && undated(requested) != undated(reported)
        {
            row.model_mismatch.push(key.clone());
        }
        if let Some(decision) = decision
            && let Some(default) = &decision.default_model
        {
            row.with_default.push(key.clone());
            if decision.selected_model != *default {
                row.pick_not_default.push(key.clone());
            }
        }
        if let Some(decision) = decision
            && let Some(learned) = &decision.proposals.learned
        {
            row.with_learned.push(key.clone());
            let masked = row.masked.get_or_insert_with(Vec::new);
            if decision.source == Some(DecisionSource::Router) {
                if decision.selected_model != *learned {
                    masked.push(key.clone());
                }
                let honest = row.executed_learned.get_or_insert_with(Vec::new);
                if executed_model(executed).is_some_and(|ran| undated(ran) == undated(learned)) {
                    honest.push(key.clone());
                }
            }
        }
        row.attempts.push(key);
    }
    RouteReport {
        runs: runs.iter().map(|run| run.run_id.clone()).collect(),
        decisions: joined_decisions,
        rows: rows.into_values().collect(),
    }
}

/// The model an attempt ran on: the one the provider reported, else the one
/// the provider bridge launched, else the one dispatch requested.
fn executed_model(executed: &ExecutedModel) -> Option<&str> {
    executed
        .model_reported
        .as_deref()
        .or(executed.model_dispatched.as_deref())
        .or(executed.model_requested.as_deref())
}

/// `model` without a trailing `-YYYYMMDD` release date.
pub(crate) fn undated(model: &str) -> &str {
    match model.rsplit_once('-') {
        Some((base, date)) if date.len() == 8 && date.bytes().all(|b| b.is_ascii_digit()) => base,
        _ => model,
    }
}

// ── srm ───────────────────────────────────────────────────────────────

/// α of the sample-ratio-mismatch check (S02 SC3).
pub const SRM_ALPHA: f64 = 0.001;
/// Units a layer needs before S02 SC3 judges it.
pub const SRM_MIN_UNITS: usize = 200;
/// The arms an assignment can realise, in the order the e-value counts
/// them. [`Arm::Explore`] is S02's ε draw inside the route decision, never
/// an assignment's arm.
pub const SRM_ARMS: [Arm; 3] = [Arm::Learned, Arm::Default, Arm::GlobalOff];
/// How far a logged propensity may sit from the one its h and g give.
const PROPENSITY_TOLERANCE: f64 = 1e-9;
/// A unit that took [`Arm::Explore`].
const EXPLORE_ARM: &str = "took the explore arm, which no assignment draws";
/// A unit that took an arm of probability 0.
const NO_CHANCE: &str = "took an arm their h and g give no chance";
/// A unit whose logged propensity is not its arm's probability.
const WRONG_PROPENSITY: &str = "log a propensity their h and g do not give";
/// A unit whose rows carry two different draws.
const REDRAWN: &str = "carry another draw in a later row: a retry redrew";

/// One arm of a layer in the sample-ratio check.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SrmArm {
    /// The arm.
    pub arm: Arm,
    /// Units that took it.
    pub units: usize,
    /// Its mean probability over the layer's units, from their logged h and
    /// g.
    pub expected: f64,
    /// Its share of the layer's units.
    pub realised: f64,
}

/// One randomised layer's sample-ratio check (S02 SC3).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SrmLayer {
    /// The layer, e.g. `knowledge`.
    pub layer: String,
    /// Units counted, once each: chains, for a per-chain layer.
    pub units: usize,
    /// Each arm, in [`SRM_ARMS`] order.
    pub arms: Vec<SrmArm>,
    /// The sequential multinomial SRM e-value (S03 §4.5) of the units' arms
    /// against their logged propensities, capped at `f64::MAX`.
    pub e_value: f64,
    /// Whether the e-value reached 1/[`SRM_ALPHA`], or a unit took an arm
    /// its h and g give no chance.
    pub mismatch: bool,
    /// Whether the layer has the [`SRM_MIN_UNITS`] units S02 SC3 asks for.
    /// The e-value is valid at any count, so a mismatch fails either way.
    pub enough_units: bool,
    /// Units whose rows break the assignment's rules, one line per rule.
    pub problems: Vec<String>,
}

/// One droppable section's bandit draws in the sample-ratio check (S02
/// L9): how often the chains that drew it left it out, against the
/// exclusion probabilities they logged.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SrmSection {
    /// The section, e.g. `conventions`.
    pub section: String,
    /// Chains that drew it, once each, with their first draw.
    pub units: usize,
    /// Of those, the draws that left it out.
    pub excluded: usize,
    /// The exclusions the draws' logged probabilities expect: Σ p_ex.
    pub expected_excluded: f64,
    /// The sequential SRM e-value (S03 §4.5) of the draws against their
    /// p_ex, capped at `f64::MAX`.
    pub e_value: f64,
    /// Whether the e-value reached 1/[`SRM_ALPHA`], or a draw took an
    /// outcome its p_ex gives no chance.
    pub mismatch: bool,
}

/// What `roko learn telemetry check --srm` prints.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct SrmReport {
    /// The runs read.
    pub runs: Vec<String>,
    /// α of the check ([`SRM_ALPHA`]).
    pub alpha: f64,
    /// One entry per layer, by layer name.
    pub layers: Vec<SrmLayer>,
    /// One entry per section the section bandit drew, by section name.
    pub sections: Vec<SrmSection>,
    /// Chains left out, by condition: `maximize` withholds nothing and
    /// `forced` pins arms, so neither draws as its h and g say.
    pub excluded: BTreeMap<String, usize>,
    /// Decision rows that carry no arms: rows written before S02.P1-14, or
    /// outside an attempt.
    pub unassigned: usize,
    /// Rows with arms whose attempt key does not parse, as `run: "key"`.
    pub unreadable: Vec<String>,
}

impl SrmReport {
    /// Every failure: a layer whose arms do not split as logged, a row that
    /// breaks the assignment's rules, arms without a readable attempt key.
    /// Empty when the check passes.
    #[must_use]
    pub fn failures(&self) -> Vec<String> {
        let mut failures = Vec::new();
        for layer in &self.layers {
            if layer.mismatch {
                failures.push(format!(
                    "{}: sample-ratio mismatch over {} unit(s) (e-value {:.3e})",
                    layer.layer, layer.units, layer.e_value
                ));
            }
            for problem in &layer.problems {
                failures.push(format!("{}: {problem}", layer.layer));
            }
        }
        for section in self.sections.iter().filter(|section| section.mismatch) {
            failures.push(format!(
                "section `{}`: the bandit left it out in {} of {} draw(s), {:.1} expected \
                 (e-value {:.3e})",
                section.section,
                section.excluded,
                section.units,
                section.expected_excluded,
                section.e_value
            ));
        }
        for row in &self.unreadable {
            failures.push(format!("arms without a readable attempt key: {row}"));
        }
        failures
    }

    /// Whether every layer splits as logged.
    #[must_use]
    pub fn passed(&self) -> bool {
        self.failures().is_empty()
    }
}

/// Check that every randomised layer of `runs` splits its units across
/// arms as their logged propensities say (S02 SC3, S02.P1-16), with S03's
/// sequential multinomial e-value at [`SRM_ALPHA`].
///
/// Every decision row of an attempt carries its chain's arm set. A layer's
/// units are the chains with a decision row at the layer's own decision
/// point: the content rows whose `decision_point` names the layer
/// (`knowledge`, `playbooks`, `sections`, ...) and the placebo rows for
/// `placebo`. A layer that records no decisions of its own (`global`, the
/// all-off draw, and `prompt_variant`) counts every chain whose rows carry
/// arms. Each unit counts once, however many of its attempts wrote rows,
/// with its first draw; a unit whose rows carry another draw is a problem,
/// since a retry must keep its chain's arms. Chains in the `maximize` and
/// `forced` conditions are left out; a placebo row takes its chain's
/// condition from the run's other rows.
///
/// The section bandit's draws (S02 L9), which the `sections` rows carry,
/// are checked section by section: each chain's first draw of a section
/// against its logged exclusion probability (bug-2410e1).
#[must_use]
pub fn srm_check(runs: &[RunRecords]) -> SrmReport {
    let mut tally = SrmTally::default();
    for run in runs {
        tally.read(run);
    }
    let layers = tally
        .draws
        .iter()
        .map(|(layer, draws)| srm_layer(layer, draws, tally.redrawn.get(layer)))
        .collect();
    let sections = tally
        .sections
        .iter()
        .map(|(section, draws)| srm_section(section, draws))
        .collect();
    SrmReport {
        runs: runs.iter().map(|run| run.run_id.clone()).collect(),
        alpha: SRM_ALPHA,
        layers,
        sections,
        excluded: tally
            .excluded
            .into_iter()
            .map(|(condition, chains)| (condition, chains.len()))
            .collect(),
        unassigned: tally.unassigned,
        unreadable: tally.unreadable,
    }
}

/// The draws [`srm_check`] gathers from its runs' decision rows.
#[derive(Default)]
struct SrmTally {
    /// Each unit's first draw, by layer and unit key.
    draws: BTreeMap<String, BTreeMap<String, Assignment>>,
    /// Units whose rows carry another draw, by layer.
    redrawn: BTreeMap<String, BTreeSet<String>>,
    /// Each chain's first draw of a section by the section bandit, by
    /// section and chain key.
    sections: BTreeMap<String, BTreeMap<String, SectionDecision>>,
    /// Chains left out, by condition.
    excluded: BTreeMap<String, BTreeSet<String>>,
    /// Decision rows without arms.
    unassigned: usize,
    /// Rows with arms whose attempt key does not parse.
    unreadable: Vec<String>,
}

impl SrmTally {
    /// Gather the draws of `run`'s decision rows: route and content rows
    /// carry their chain's whole arm set, placebo rows the placebo's draw.
    fn read(&mut self, run: &RunRecords) {
        let mut rows: Vec<(&str, &ArmSet, Option<&str>)> = Vec::new();
        for line in &run.decisions {
            match &line.record.arm_set {
                Some(arms) => {
                    let key = line.record.attempt_key.as_deref().unwrap_or_default();
                    rows.push((key, arms, None));
                }
                None => self.unassigned += 1,
            }
        }
        for line in &run.content_decisions {
            let record = &line.record;
            match &record.arm_set {
                Some(arms) => {
                    let point = record.decision_point.as_str();
                    rows.push((record.identity.attempt_key.as_str(), arms, Some(point)));
                }
                None => self.unassigned += 1,
            }
            for draw in &record.section_draws {
                self.sections
                    .entry(draw.section.clone())
                    .or_default()
                    .entry(record.identity.chain_key.clone())
                    .or_insert_with(|| draw.clone());
            }
        }
        // A run draws every chain's arms in one mode, so a placebo row whose
        // chain wrote no other row takes the condition the run's arms share.
        let conditions: HashMap<&str, &str> = rows
            .iter()
            .map(|&(_, arms, _)| (arms.chain_key.as_str(), arms.condition_id.as_str()))
            .collect();
        let shared: BTreeSet<&str> = conditions.values().copied().collect();
        let run_condition = match shared.first() {
            Some(condition) if shared.len() == 1 => *condition,
            _ => NORMAL_CONDITION,
        };
        for (key, arms, point) in rows {
            if self.leaves_out(&arms.condition_id, &arms.chain_key) {
                continue;
            }
            let Some(key) = AttemptKey::parse(key) else {
                self.unreadable.push(format!("{}: {key:?}", run.run_id));
                continue;
            };
            for assignment in arms.arms.values() {
                let layer = assignment.layer.as_str();
                // A layer with decision rows of its own counts only those.
                if point == Some(layer) || !has_decision_point(layer) {
                    self.observe(&key, assignment);
                }
            }
        }
        for line in &run.placebo_decisions {
            let record = &line.record;
            let chain = record.identity.chain_key.as_str();
            let condition = conditions.get(chain).copied().unwrap_or(run_condition);
            if self.leaves_out(condition, chain) {
                continue;
            }
            let key = &record.identity.attempt_key;
            match AttemptKey::parse(key) {
                Some(key) => self.observe(&key, &record.assignment),
                None => self.unreadable.push(format!("{}: {key:?}", run.run_id)),
            }
        }
    }

    /// Whether `condition` leaves `chain` out of the check; a chain left out
    /// is counted under its condition.
    fn leaves_out(&mut self, condition: &str, chain: &str) -> bool {
        let left_out = condition == MAXIMIZE_CONDITION || condition == FORCED_CONDITION;
        if left_out {
            self.excluded
                .entry(condition.to_string())
                .or_default()
                .insert(chain.to_string());
        }
        left_out
    }

    /// Count `assignment`, the draw of `key`'s unit on its layer, unless the
    /// unit has a draw already; a different one is a redraw.
    fn observe(&mut self, key: &AttemptKey, assignment: &Assignment) {
        let unit = assignment.unit.unit_key(key);
        let layer = self.draws.entry(assignment.layer.clone()).or_default();
        let first = layer
            .entry(unit.clone())
            .or_insert_with(|| assignment.clone());
        if *first != *assignment {
            self.redrawn
                .entry(assignment.layer.clone())
                .or_default()
                .insert(unit);
        }
    }
}

/// The check of `section`'s bandit draws, one per chain: a two-outcome
/// e-value of each draw, kept or left out, against its logged p_ex.
fn srm_section(section: &str, draws: &BTreeMap<String, SectionDecision>) -> SrmSection {
    let mut srm = SrmEvalue::new(2);
    let (mut excluded, mut expected_excluded) = (0, 0.0);
    let mut impossible = false;
    for draw in draws.values() {
        let p_ex = draw.p_exclude.clamp(0.0, 1.0);
        let chance = if draw.excluded { p_ex } else { 1.0 - p_ex };
        impossible |= chance <= 0.0;
        srm.push(usize::from(draw.excluded), &[1.0 - p_ex, p_ex]);
        excluded += usize::from(draw.excluded);
        expected_excluded += p_ex;
    }
    SrmSection {
        section: section.to_string(),
        units: draws.len(),
        excluded,
        expected_excluded,
        e_value: srm.e_value().min(f64::MAX),
        mismatch: impossible || srm.rejects(SRM_ALPHA),
    }
}

/// Whether `layer` records decisions in rows of its own: the placebo, and
/// each content decision point (`knowledge`, `playbooks`, `sections`, ...).
fn has_decision_point(layer: &str) -> bool {
    let point = serde_json::from_value::<ContentDecisionPoint>(Value::from(layer));
    layer == PLACEBO_LAYER || point.is_ok()
}

/// Units that broke each assignment rule: how many, and the first.
type Problems = BTreeMap<&'static str, (usize, String)>;

/// Note `unit` under `rule`.
fn note(problems: &mut Problems, rule: &'static str, unit: &str) {
    let entry = problems.entry(rule).or_default();
    if entry.0 == 0 {
        entry.1 = unit.to_string();
    }
    entry.0 += 1;
}

/// P(learned), P(default) and P(all-off) under a draw's h and g (S01 §4.6),
/// in [`SRM_ARMS`] order.
fn arm_probabilities(assignment: &Assignment) -> [f64; 3] {
    let (h, g) = (assignment.h, assignment.g);
    [(1.0 - g) * (1.0 - h), (1.0 - g) * h, g]
}

/// The check of `layer` over its units' first draws; the `redrawn` units
/// carry another draw too.
fn srm_layer(
    layer: &str,
    draws: &BTreeMap<String, Assignment>,
    redrawn: Option<&BTreeSet<String>>,
) -> SrmLayer {
    let mut srm = SrmEvalue::new(SRM_ARMS.len());
    let mut counts = [0_usize; SRM_ARMS.len()];
    let mut probability_sums = [0.0_f64; SRM_ARMS.len()];
    let mut problems = Problems::new();
    let mut impossible = false;
    for (unit, assignment) in draws {
        let Some(index) = SRM_ARMS.iter().position(|arm| *arm == assignment.arm) else {
            note(&mut problems, EXPLORE_ARM, unit);
            continue;
        };
        let probabilities = arm_probabilities(assignment);
        if probabilities[index] <= 0.0 {
            impossible = true;
            note(&mut problems, NO_CHANCE, unit);
        } else if (assignment.propensity - probabilities[index]).abs() > PROPENSITY_TOLERANCE {
            let (logged, given) = (assignment.propensity, probabilities[index]);
            let example = format!("{unit} logs {logged:.4}, h and g give {given:.4}");
            note(&mut problems, WRONG_PROPENSITY, &example);
        }
        srm.push(index, &probabilities);
        counts[index] += 1;
        for (sum, probability) in probability_sums.iter_mut().zip(probabilities) {
            *sum += probability;
        }
    }
    for unit in redrawn.into_iter().flatten() {
        note(&mut problems, REDRAWN, unit);
    }
    let units: usize = counts.iter().sum();
    let whole = units.max(1) as f64;
    let arms = SRM_ARMS
        .into_iter()
        .enumerate()
        .map(|(index, arm)| SrmArm {
            arm,
            units: counts[index],
            expected: probability_sums[index] / whole,
            realised: counts[index] as f64 / whole,
        })
        .collect();
    let problems = problems
        .into_iter()
        .map(|(rule, (count, first))| format!("{count} unit(s) {rule}, e.g. {first}"))
        .collect();
    SrmLayer {
        layer: layer.to_string(),
        units,
        arms,
        e_value: srm.e_value().min(f64::MAX),
        mismatch: impossible || srm.rejects(SRM_ALPHA),
        enough_units: units >= SRM_MIN_UNITS,
        problems,
    }
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;
    use crate::telemetry::assign::AssignmentUnit;
    use crate::telemetry::census::CensusComponent;
    use crate::telemetry::records::{
        AttemptIdentity, AttemptOutcome, ContentCandidate, ContentDecisionPoint, CostSource,
        DecisionSource, ExcludedReason, ExposureItemKind, TelemetryRecord,
    };
    use crate::telemetry::writer::{TelemetryWriter, TelemetryWriterConfig};

    const RUN: &str = "gr-7f3c2a91";

    /// A route decision row with no attempt key yet.
    const ROUTE_DECISION: &str = r#"{
        "timestamp": "2026-10-02T14:03:11.402Z",
        "trace_id": "trace-1",
        "task_id": "T1",
        "requested_model": "gpt-oss-120b",
        "role": "implementer",
        "task_complexity": "focused",
        "selected_provider": "cerebras",
        "selected_model": "gpt-oss-120b",
        "routing_stage": "ucb",
        "routing_reason": "highest_ucb_score",
        "candidates": []
    }"#;

    fn identity(task: &str, attempt: u32) -> AttemptIdentity {
        AttemptIdentity::new(&AttemptKey::new(RUN, "loop-census", task, attempt))
    }

    fn open(task: &str, attempt: u32) -> AttemptOpenRecord {
        AttemptOpenRecord::new(identity(task, attempt), 0)
    }

    fn verdict(task: &str, attempt: u32, outcome: AttemptOutcome) -> AttemptVerdictRecord {
        let mut verdict = AttemptVerdictRecord::settle(identity(task, attempt), outcome, true);
        verdict.executed.model_requested = Some("gpt-oss-120b".to_string());
        verdict.executed.model_reported = Some("gpt-oss-120b".to_string());
        verdict.cost.source = CostSource::ProviderUsage;
        verdict
    }

    fn decision(
        task: &str,
        attempt: u32,
        source: DecisionSource,
        selected: &str,
    ) -> RoutingDecisionLog {
        let mut decision: RoutingDecisionLog =
            serde_json::from_str(ROUTE_DECISION).expect("parse decision");
        decision.attempt_key = Some(identity(task, attempt).attempt_key);
        decision.source = Some(source);
        decision.default_model = Some("gpt-oss-120b".to_string());
        decision.selected_model = selected.to_string();
        decision
    }

    /// The loop-census fixture's shape: T1 passes, T2 fails twice, T3 is
    /// unverified, T4's pinned model is replaced by a guard.
    fn census_run(dir: &TempDir) -> RunRecords {
        let writer = TelemetryWriter::spawn(dir.path(), TelemetryWriterConfig::default())
            .expect("spawn writer");
        let attempts = [
            ("T1", 1, AttemptOutcome::Passed),
            ("T2", 1, AttemptOutcome::GateFailed),
            ("T2", 2, AttemptOutcome::GateFailed),
            ("T3", 1, AttemptOutcome::Unverified),
            ("T4", 1, AttemptOutcome::Passed),
        ];
        for (task, attempt, outcome) in attempts {
            assert!(writer.submit(open(task, attempt)));
            let source = match task {
                "T1" | "T2" => Some((DecisionSource::Router, "gpt-oss-120b")),
                "T4" => Some((DecisionSource::Fallback, "glm-4.6")),
                _ => None,
            };
            if let Some((source, selected)) = source {
                assert!(writer.submit(decision(task, attempt, source, selected)));
            }
            let mut verdict = verdict(task, attempt, outcome);
            if task == "T4" {
                verdict.executed.model_requested = Some("glm-4.6".to_string());
                verdict.executed.model_reported = Some("glm-4.6-20250101".to_string());
            }
            if task == "T3" {
                verdict.executed.model_reported = Some("gpt-oss-20b".to_string());
            }
            assert!(writer.submit(verdict));
        }
        assert_eq!(writer.close().written, 14);
        RunRecords::load(dir.path()).expect("load the run")
    }

    #[test]
    fn route_report_counts_labels_by_source() {
        let dir = TempDir::new().expect("tempdir");
        let run = census_run(&dir);
        assert!(run.invalid.is_empty(), "{:?}", run.invalid);

        let report = route_report(std::slice::from_ref(&run), None);
        assert_eq!(report.attempts(), 5);
        assert_eq!(report.decisions, 4);
        let key = |task: &str, attempt: u32| identity(task, attempt).attempt_key;
        let sources: Vec<&str> = report.rows.iter().map(|row| row.source.as_str()).collect();
        assert_eq!(sources, ["fallback", "router", UNKNOWN_SOURCE]);

        let fallback = &report.rows[0];
        assert_eq!(fallback.passed, [key("T4", 1)]);
        assert_eq!(fallback.pick_not_default, [key("T4", 1)]);
        assert_eq!(fallback.iota(), Some(1.0));
        assert!(
            fallback.model_mismatch.is_empty(),
            "a date suffix is no mismatch"
        );

        let router = &report.rows[1];
        assert_eq!(router.attempts.len(), 3);
        assert_eq!(router.passed, [key("T1", 1)]);
        assert_eq!(router.failed, [key("T2", 1), key("T2", 2)]);
        assert!(router.unlabeled.is_empty());
        assert_eq!(router.pass_rate(), Some(1.0 / 3.0));
        assert_eq!(router.iota(), Some(0.0));
        assert_eq!(
            router.masked, None,
            "masked needs the router's own proposal"
        );

        let unknown = &report.rows[2];
        assert_eq!(unknown.unlabeled, [key("T3", 1)], "T3 is unverified");
        assert_eq!(unknown.model_mismatch, [key("T3", 1)]);
        assert_eq!(unknown.iota(), None, "no decision names a default");

        // Verdicts written before `since` do not count.
        let later = Utc::now() + chrono::Duration::hours(1);
        assert_eq!(route_report(&[run], Some(later)).attempts(), 0);
    }

    #[test]
    fn route_report_counts_masked_and_honest_routes() {
        use DecisionSource as S;
        const GLM: &str = "glm-4.6";
        const KIMI: &str = "kimi-k2";
        const OSS: &str = "gpt-oss-120b";

        let dir = TempDir::new().expect("tempdir");
        let writer = TelemetryWriter::spawn(dir.path(), TelemetryWriterConfig::default())
            .expect("spawn writer");
        // (task, attempt, source, router's proposal, pick, model the provider served)
        let attempts = [
            ("T1", 1, S::Router, Some(GLM), GLM, "glm-4.6-20250101"),
            ("T2", 1, S::Router, Some(KIMI), OSS, OSS),
            ("T2", 2, S::Router, Some(GLM), GLM, OSS),
            ("T3", 1, S::Fallback, Some(KIMI), OSS, OSS),
            ("T4", 1, S::TaskHint, None, GLM, GLM),
        ];
        for (task, attempt, source, learned, pick, served) in attempts {
            assert!(writer.submit(open(task, attempt)));
            let mut decision = decision(task, attempt, source, pick);
            decision.proposals.learned = learned.map(str::to_string);
            assert!(writer.submit(decision));
            let mut verdict = verdict(task, attempt, AttemptOutcome::Passed);
            verdict.executed.model_requested = Some(pick.to_string());
            verdict.executed.model_reported = Some(served.to_string());
            assert!(writer.submit(verdict));
        }
        assert_eq!(writer.close().written, 15);
        let run = RunRecords::load(dir.path()).expect("load the run");
        assert!(run.invalid.is_empty(), "{:?}", run.invalid);

        let report = route_report(std::slice::from_ref(&run), None);
        let key = |task: &str, attempt: u32| identity(task, attempt).attempt_key;
        let sources: Vec<&str> = report.rows.iter().map(|row| row.source.as_str()).collect();
        assert_eq!(sources, ["fallback", "router", "task_hint"]);

        // T2's first pick was rewritten while labelled `router`: masked. Its
        // second failed over to another model: honestly labelled, but not
        // what the router proposed. T1 ran a dated snapshot of its pick.
        let router = &report.rows[1];
        assert_eq!(
            router.with_learned,
            [key("T1", 1), key("T2", 1), key("T2", 2)]
        );
        assert_eq!(router.masked, Some(vec![key("T2", 1)]));
        assert_eq!(router.executed_learned, Some(vec![key("T1", 1)]));
        assert_eq!(router.eps_honest(), Some(1.0 / 3.0));

        // A fallback labelled as one is not masked, and ε_honest is the
        // router's alone.
        let fallback = &report.rows[0];
        assert_eq!(fallback.with_learned, [key("T3", 1)]);
        assert_eq!(fallback.masked, Some(Vec::new()));
        assert_eq!(fallback.eps_honest(), None);

        let hint = &report.rows[2];
        assert_eq!(hint.masked, None, "no decision records a proposal");
        assert_eq!(hint.eps_honest(), None);
    }

    #[test]
    fn check_passes_a_joined_run_and_flags_a_duplicate_settlement() {
        let dir = TempDir::new().expect("tempdir");
        let run = census_run(&dir);
        let keys: Vec<String> = run
            .verdicts
            .iter()
            .map(|line| line.record.identity.attempt_key.clone())
            .collect();
        let legacy = LegacyRows {
            efficiency: keys.clone(),
            costs: keys.clone(),
            episodes: keys,
        };
        let report = check(&run, &legacy);
        assert_eq!(report.failures(), Vec::<String>::new());
        assert_eq!(
            (
                report.attempts_opened,
                report.attempts_settled,
                report.attempts_abandoned
            ),
            (5, 5, 0)
        );
        assert_eq!(report.cost_sources["provider_usage"], 5);

        // A second process settles T1 again: the writer's dedupe cannot see
        // it, so check must.
        let mut again = run.verdicts[0].clone();
        again.seq = 15;
        roko_core::io::append_jsonl(&RunFile::Attempts.path_in(dir.path()), &again)
            .expect("append");
        let run = RunRecords::load(dir.path()).expect("reload");
        let mut legacy = legacy;
        legacy.costs.pop();
        let report = check(&run, &legacy);
        assert_eq!(
            report.duplicate_settlements,
            [again.record.settlement_id.clone()]
        );
        assert!(!report.passed());
        let costs = &report.coverage[1];
        assert_eq!(
            (costs.file, costs.joined, costs.verdicts),
            ("learn/costs.jsonl", 4, 5)
        );
        assert_eq!(costs.missing.len(), 1);
        assert!(
            report.seq_violations.is_empty(),
            "{:?}",
            report.seq_violations
        );
        assert_eq!(again.record.record_id(), run.verdicts[0].record_id);
    }

    #[test]
    fn check_flags_invalid_lines_and_seq_disorder() {
        let dir = TempDir::new().expect("tempdir");
        let path = RunFile::Attempts.path_in(dir.path());
        let open_line = |seq: u64| {
            let record = open("T1", 1);
            serde_json::json!({
                "schema_version": ATTEMPT_OPEN_SCHEMA,
                "record_id": record.record_id(),
                "seq": seq,
                "ts": "2026-10-02T14:03:11.402Z",
                "run_id": RUN,
                "plan_id": "loop-census",
                "task_id": "T1",
                "attempt": 1,
                "attempt_key": record.identity.attempt_key,
                "chain_key": record.identity.chain_key,
            })
            .to_string()
        };
        let text = format!(
            "{}\n{}\nnot json\n{{\"schema_version\":\"roko.other/1\",\"seq\":1}}\n",
            open_line(3),
            open_line(2)
        );
        std::fs::write(&path, text).expect("write attempts.jsonl");

        let report = check(
            &RunRecords::load(dir.path()).expect("load"),
            &LegacyRows::default(),
        );
        assert_eq!(report.invalid_lines.len(), 2, "{:?}", report.invalid_lines);
        assert!(report.invalid_lines[0].starts_with("attempts.jsonl:3: not JSON"));
        assert!(report.invalid_lines[1].contains("unexpected schema_version"));
        let violations = &report.seq_violations;
        assert!(
            violations.iter().any(|v| v.contains("seq 2 follows seq 3")),
            "{violations:?}"
        );
        assert!(
            violations.iter().any(|v| v.contains("seq 1 follows seq 2")),
            "{violations:?}"
        );
        assert_eq!(report.attempts_abandoned, 1);
        assert!(!report.passed());
    }

    #[test]
    fn legacy_rows_keep_only_the_runs_keys() {
        let dir = TempDir::new().expect("tempdir");
        let layout = RokoLayout::new(dir.path().join(".roko"));
        let learn = layout.learn_dir();
        std::fs::create_dir_all(&learn).expect("learn dir");
        let ours = identity("T1", 1).attempt_key;
        let theirs = AttemptKey::new("other-run", "p", "T1", 1).attempt_key();
        std::fs::write(
            learn.join("costs.jsonl"),
            format!(
                "{{\"attempt_key\":\"{ours}\",\"model\":\"m\"}}\n{{\"attempt_key\":\"{theirs}\"}}\n{{\"model\":\"pre-s01\"}}\n"
            ),
        )
        .expect("write costs");
        std::fs::write(
            layout.root_episodes_path(),
            format!("{{\"extra\":{{\"attempt_key\":\"{ours}\"}}}}\n"),
        )
        .expect("write episodes");

        let rows = LegacyRows::load(&layout, RUN).expect("load legacy rows");
        assert_eq!(rows.costs, [ours.clone()]);
        assert_eq!(rows.episodes, [ours]);
        assert!(rows.efficiency.is_empty());
    }

    /// gap-127263: a frozen run writes its episodes to its own run
    /// directory, where the check joins them as it joins the workspace's.
    #[test]
    fn legacy_rows_read_a_frozen_runs_own_episodes() {
        let dir = TempDir::new().expect("tempdir");
        let layout = RokoLayout::new(dir.path().join(".roko"));
        let ours = identity("T1", 1).attempt_key;
        std::fs::create_dir_all(layout.run_dir(RUN)).expect("run dir");
        std::fs::write(
            layout.run_dir(RUN).join("episodes.jsonl"),
            format!("{{\"extra\":{{\"attempt_key\":\"{ours}\"}}}}\n"),
        )
        .expect("write the run's episodes");

        let rows = LegacyRows::load(&layout, RUN).expect("load legacy rows");
        assert_eq!(rows.episodes, [ours]);
        assert!(!layout.root_episodes_path().exists());
    }

    /// `check` lists what the run's census has unwired, and only warns of a
    /// run with no census, since runs before the census have none.
    #[test]
    fn check_lists_the_census_unwired_components() {
        let dir = TempDir::new().expect("tempdir");
        let legacy = LegacyRows::default();
        let report = check(&census_run(&dir), &legacy);
        assert_eq!(report.unwired_components, None);
        assert_eq!(report.warnings.len(), 1, "{:?}", report.warnings);
        assert!(report.warnings[0].starts_with("no census.json"));

        let component = |id: &str, wired: bool| CensusComponent {
            id: id.to_string(),
            kind: "sink".to_string(),
            wired,
            detail: None,
        };
        let components = vec![
            component("sink.routing", true),
            component("sink.section_effect", false),
        ];
        let census = CensusReport::new(RUN, "725f21e05", false, components);
        census.store(dir.path()).expect("store the census");
        let run = RunRecords::load(dir.path()).expect("reload the run");
        assert_eq!(run.census.as_ref(), Some(&census));
        let report = check(&run, &legacy);
        let unwired = vec!["sink.section_effect".to_string()];
        assert_eq!(report.unwired_components, Some(unwired));
        assert!(report.warnings.is_empty(), "{:?}", report.warnings);
    }

    /// The knowledge decision of attempt `task`:`attempt`: two retrieved
    /// entries, the first included.
    fn knowledge_decision(task: &str, attempt: u32) -> ContentDecisionRecord {
        let candidate = |id: &str, rank: u32, p: f64| ContentCandidate {
            id: id.to_string(),
            rank: Some(rank),
            score: None,
            eligible: true,
            p: Some(p),
        };
        ContentDecisionRecord {
            identity: identity(task, attempt),
            decision_point: ContentDecisionPoint::Knowledge,
            policy: "keyword_overlap_top3".to_string(),
            candidates: vec![candidate("kn-1", 1, 1.0), candidate("kn-2", 2, 0.0)],
            chosen: vec!["kn-1".to_string()],
            chosen_propensity: Some(1.0),
            source: Some(DecisionSource::Default),
            state: None,
            thresholds_digest: None,
            arm_set: None,
            section_draws: Vec::new(),
            proposals: None,
            audit: Default::default(),
        }
    }

    /// A run's `decisions.jsonl` holds route and content rows, told apart by
    /// `decision_point` (a route row written before that field has none),
    /// and its `exposures.jsonl` holds one row per retrieved item. `check`
    /// validates both schemas and flags an exposure of an attempt the run
    /// never opened; `route_report` counts the route rows alone.
    #[test]
    fn report_reads_route_and_content_decisions() {
        let dir = TempDir::new().expect("tempdir");
        // A route row from before `decision_point`, written first.
        let route = decision("T1", 1, DecisionSource::Router, "gpt-oss-120b");
        let line = Stamped {
            schema_version: DECISION_SCHEMA.to_string(),
            record_id: route.record_id(),
            seq: 1,
            ts: "2026-10-02T14:03:11.402Z".to_string(),
            record: route,
        };
        let mut legacy = serde_json::to_value(&line).expect("serialize the route row");
        let fields = legacy.as_object_mut().expect("a JSON object");
        assert!(fields.remove("decision_point").is_some());
        let decisions_path = RunFile::Decisions.path_in(dir.path());
        std::fs::write(&decisions_path, format!("{legacy}\n")).expect("write decisions.jsonl");

        let writer = TelemetryWriter::spawn(dir.path(), TelemetryWriterConfig::default())
            .expect("spawn writer");
        assert!(writer.submit(open("T1", 1)));
        assert!(writer.submit(knowledge_decision("T1", 1)));
        let exposure = |task: &str, id: &str, included: bool| {
            let mut row = ExposureRecord::new(identity(task, 1), ExposureItemKind::Knowledge, id);
            row.included = included;
            row.excluded_reason = (!included).then_some(ExcludedReason::TokenBudget);
            row.section_id = Some("domain_context".to_string());
            row
        };
        assert!(writer.submit(exposure("T1", "kn-1", true)));
        assert!(writer.submit(exposure("T1", "kn-2", false)));
        // T9 never opened an attempt in this run.
        assert!(writer.submit(exposure("T9", "kn-1", true)));
        assert!(writer.submit(verdict("T1", 1, AttemptOutcome::Passed)));
        assert_eq!(writer.close().written, 6);

        let run = RunRecords::load(dir.path()).expect("load the run");
        assert!(run.invalid.is_empty(), "{:?}", run.invalid);
        assert_eq!(run.decisions.len(), 1, "the legacy route row");
        let route = &run.decisions[0].record;
        assert_eq!(route.decision_point, ROUTE_DECISION_POINT);
        assert_eq!(run.content_decisions.len(), 1);
        let content = &run.content_decisions[0].record;
        assert_eq!(content, &knowledge_decision("T1", 1));
        let included: Vec<(&str, bool)> = run
            .exposures
            .iter()
            .map(|line| (line.record.item_id.as_str(), line.record.included))
            .collect();
        assert_eq!(included, [("kn-1", true), ("kn-2", false), ("kn-1", true)]);

        // Content rows are not route decisions.
        let report = route_report(std::slice::from_ref(&run), None);
        assert_eq!((report.attempts(), report.decisions), (1, 1));
        let sources: Vec<&str> = report.rows.iter().map(|row| row.source.as_str()).collect();
        assert_eq!(sources, ["router"]);

        let t1 = identity("T1", 1).attempt_key;
        let legacy_rows = LegacyRows {
            efficiency: vec![t1.clone()],
            costs: vec![t1.clone()],
            episodes: vec![t1],
        };
        let checked = check(&run, &legacy_rows);
        assert_eq!((checked.content_decisions, checked.exposures), (1, 3));
        let t9 = identity("T9", 1).attempt_key;
        assert_eq!(checked.unopened_exposures, [t9.clone()]);
        assert!(
            checked.seq_violations.is_empty(),
            "{:?}",
            checked.seq_violations
        );
        let unopened = format!("exposure without an attempt-open line: {t9}");
        assert_eq!(checked.failures(), [unopened]);

        // A content row that does not match its schema is an invalid line.
        let malformed = serde_json::json!({
            "schema_version": DECISION_SCHEMA,
            "decision_point": "knowledge",
            "seq": 9,
        });
        roko_core::io::append_jsonl(&decisions_path, &malformed).expect("append");
        let run = RunRecords::load(dir.path()).expect("reload");
        assert_eq!(run.invalid.len(), 1, "{:?}", run.invalid);
        let invalid = &run.invalid[0];
        assert!(
            invalid.starts_with("decisions.jsonl:3: invalid roko.decision/1"),
            "{invalid}"
        );
        assert_eq!(run.content_decisions.len(), 1);
    }

    /// A chain's draw on `layer` at h, with g = 0: the default arm when
    /// `default`, else the learned arm, at its propensity.
    fn chain_draw(layer: &str, h: f64, default: bool) -> Assignment {
        let (arm, propensity, u) = if default {
            (Arm::Default, h, h / 2.0)
        } else {
            (Arm::Learned, 1.0 - h, (1.0 + h) / 2.0)
        };
        Assignment {
            unit: AssignmentUnit::Chain,
            layer: layer.to_string(),
            salt_id: format!("{layer}@2026-10-03"),
            u,
            h,
            g: 0.0,
            arm,
            propensity,
        }
    }

    /// The arms of chain `chain_key`.
    fn arm_set(chain_key: &str, draws: &[Assignment], condition: &str) -> ArmSet {
        ArmSet {
            chain_key: chain_key.to_string(),
            arms: draws
                .iter()
                .map(|draw| (draw.layer.clone(), draw.clone()))
                .collect(),
            condition_id: condition.to_string(),
        }
    }

    /// `record` as a line of its run file.
    fn stamped<T: TelemetryRecord>(record: T) -> Stamped<T> {
        Stamped {
            schema_version: T::SCHEMA.to_string(),
            record_id: record.record_id(),
            seq: 1,
            ts: "2026-10-03T09:00:00Z".to_string(),
            record,
        }
    }

    /// The knowledge row of attempt `task`:`attempt`, carrying its chain's
    /// `draws` in `condition`.
    fn knowledge_row(
        task: &str,
        attempt: u32,
        draws: &[Assignment],
        condition: &str,
    ) -> Stamped<ContentDecisionRecord> {
        let mut record = knowledge_decision(task, attempt);
        record.arm_set = Some(arm_set(&record.identity.chain_key, draws, condition));
        stamped(record)
    }

    /// A run with no rows yet.
    fn empty_run() -> RunRecords {
        RunRecords {
            run_id: RUN.to_string(),
            ..RunRecords::default()
        }
    }

    /// S02 SC3 (backlog 4130): 1,000 chains whose knowledge draw at h = 0.5
    /// splits 50/50 pass, each counted once however many attempts it
    /// wrote; a 60/40 split is a sample-ratio mismatch.
    #[test]
    fn srm_check_flags_a_skewed_layer() {
        // The first `defaults` of 1,000 chains take the default arm. Each
        // chain writes a row on its first attempt and on a retry, which
        // keeps the chain's draw.
        let stream = |defaults: usize| {
            let mut run = empty_run();
            for index in 0..1000 {
                let draw = chain_draw("knowledge", 0.5, index < defaults);
                let task = format!("T{index}");
                for attempt in 1..=2 {
                    let row = knowledge_row(&task, attempt, &[draw.clone()], NORMAL_CONDITION);
                    run.content_decisions.push(row);
                }
            }
            run
        };

        let even = srm_check(&[stream(500)]);
        assert_eq!(even.failures(), Vec::<String>::new());
        assert_eq!(even.layers.len(), 1, "{:?}", even.layers);
        let layer = &even.layers[0];
        assert_eq!((layer.layer.as_str(), layer.units), ("knowledge", 1000));
        assert!(layer.enough_units && !layer.mismatch, "{layer:?}");
        assert!(layer.e_value < 1.0, "{layer:?}");
        let arms: Vec<(Arm, usize, f64, f64)> = layer
            .arms
            .iter()
            .map(|arm| (arm.arm, arm.units, arm.expected, arm.realised))
            .collect();
        let split = [
            (Arm::Learned, 500, 0.5, 0.5),
            (Arm::Default, 500, 0.5, 0.5),
            (Arm::GlobalOff, 0, 0.0, 0.0),
        ];
        assert_eq!(arms, split);

        let skewed = srm_check(&[stream(400)]);
        let layer = &skewed.layers[0];
        assert_eq!((layer.units, layer.arms[0].units), (1000, 600));
        assert!(layer.mismatch, "{layer:?}");
        assert!(layer.e_value >= 1.0 / SRM_ALPHA, "{layer:?}");
        let failures = skewed.failures();
        assert_eq!(failures.len(), 1, "{failures:?}");
        assert!(
            failures[0].starts_with("knowledge: sample-ratio mismatch over 1000 unit(s)"),
            "{failures:?}"
        );
    }

    /// Each layer counts at its own decision point: knowledge rows for
    /// `knowledge`, placebo rows for `placebo`, and every row with arms for
    /// the all-off draw, which has none. Maximize and forced chains are
    /// left out, and a placebo row takes its chain's condition.
    #[test]
    fn srm_check_reads_each_layer_at_its_decision_point() {
        let mut global = chain_draw("global", 0.0, false);
        global.g = 0.05;
        global.propensity = 0.95;
        let mut run = empty_run();
        // T0-T3 write a knowledge row each, with a playbooks draw that no
        // playbooks row backs; T0 and T1 write placebo rows too.
        for index in 0..4 {
            let task = format!("T{index}");
            let draws = [
                global.clone(),
                chain_draw("knowledge", 0.5, index % 2 == 0),
                chain_draw("playbooks", 0.5, false),
            ];
            let row = knowledge_row(&task, 1, &draws, NORMAL_CONDITION);
            run.content_decisions.push(row);
            if index < 2 {
                let draw = chain_draw(PLACEBO_LAYER, 0.5, false);
                let row = PlaceboDecisionRecord::new(identity(&task, 1), draw);
                run.placebo_decisions.push(stamped(row));
            }
        }
        // T4 writes a route row alone, which counts for the all-off draw.
        let mut route = decision("T4", 1, DecisionSource::Router, "gpt-oss-120b");
        let draws = [global.clone(), chain_draw("knowledge", 0.5, true)];
        let arms = arm_set(&identity("T4", 1).chain_key, &draws, NORMAL_CONDITION);
        route.arm_set = Some(arms);
        run.decisions.push(stamped(route));
        // T5 runs in maximize mode and T6 with a forced arm: both are left
        // out, and so is T5's placebo row.
        let maximize = [chain_draw("knowledge", 0.0, false)];
        let row = knowledge_row("T5", 1, &maximize, MAXIMIZE_CONDITION);
        run.content_decisions.push(row);
        let draw = chain_draw(PLACEBO_LAYER, 0.0, false);
        let row = PlaceboDecisionRecord::new(identity("T5", 1), draw);
        run.placebo_decisions.push(stamped(row));
        let mut forced = chain_draw("knowledge", 0.5, true);
        forced.propensity = 1.0;
        let row = knowledge_row("T6", 1, &[forced], FORCED_CONDITION);
        run.content_decisions.push(row);
        // T7's row predates arm sets.
        let row = stamped(knowledge_decision("T7", 1));
        run.content_decisions.push(row);

        let report = srm_check(&[run]);
        assert_eq!(report.failures(), Vec::<String>::new());
        let units: Vec<(&str, usize)> = report
            .layers
            .iter()
            .map(|layer| (layer.layer.as_str(), layer.units))
            .collect();
        assert_eq!(units, [("global", 5), ("knowledge", 4), ("placebo", 2)]);
        let knowledge = &report.layers[1];
        assert_eq!((knowledge.arms[0].units, knowledge.arms[1].units), (2, 2));
        assert!(!knowledge.enough_units, "{knowledge:?}");
        let excluded: Vec<(&str, usize)> = report
            .excluded
            .iter()
            .map(|(condition, chains)| (condition.as_str(), *chains))
            .collect();
        assert_eq!(excluded, [(FORCED_CONDITION, 1), (MAXIMIZE_CONDITION, 1)]);
        assert_eq!(report.unassigned, 1);
    }

    /// bug-2410e1 (S02 L9): `--srm` checks each section the section bandit
    /// drew, one draw per chain, against the exclusion probabilities the
    /// sections rows logged. 400 chains that leave `conventions` out at
    /// p_ex = 0.2 pass with 80 exclusions, and 160 is a mismatch; a retry's
    /// row with the same draw counts once.
    #[test]
    fn srm_check_reports_each_sections_bandit_draws() {
        let draw = |excluded: bool| SectionDecision {
            section: "conventions".to_string(),
            p_exclude: 0.2,
            excluded,
            propensity: if excluded { 0.2 } else { 0.8 },
        };
        let stream = |left_out: usize| {
            let mut run = empty_run();
            for index in 0..400 {
                for attempt in 1..=2 {
                    let mut record = knowledge_decision(&format!("T{index}"), attempt);
                    record.decision_point = ContentDecisionPoint::Sections;
                    record.section_draws = vec![draw(index < left_out)];
                    run.content_decisions.push(stamped(record));
                }
            }
            run
        };

        let even = srm_check(&[stream(80)]);
        assert_eq!(even.failures(), Vec::<String>::new());
        let rows: Vec<(&str, usize, usize)> = even
            .sections
            .iter()
            .map(|row| (row.section.as_str(), row.units, row.excluded))
            .collect();
        assert_eq!(rows, [("conventions", 400, 80)]);
        let section = &even.sections[0];
        assert!((section.expected_excluded - 80.0).abs() < 1e-9, "{section:?}");
        assert!(!section.mismatch && section.e_value < 1.0, "{section:?}");

        let skewed = srm_check(&[stream(160)]);
        let section = &skewed.sections[0];
        assert!(section.mismatch, "{section:?}");
        let failures = skewed.failures();
        assert_eq!(failures.len(), 1, "{failures:?}");
        assert!(
            failures[0].starts_with("section `conventions`: the bandit left it out in 160 of 400"),
            "{failures:?}"
        );
    }

    /// A retry that redraws, a propensity its h and g do not give, an arm
    /// they give no chance, and arms without a readable attempt key all
    /// fail the check.
    #[test]
    fn srm_check_flags_redraws_and_impossible_arms() {
        let mut run = empty_run();
        // T0's retry draws another arm.
        for (attempt, default) in [(1, false), (2, true)] {
            let draw = chain_draw("knowledge", 0.5, default);
            let row = knowledge_row("T0", attempt, &[draw], NORMAL_CONDITION);
            run.content_decisions.push(row);
        }
        // T1 takes the default arm at h = 0; T2 logs propensity 1 at h = 0.5.
        let impossible = chain_draw("knowledge", 0.0, true);
        let row = knowledge_row("T1", 1, &[impossible], NORMAL_CONDITION);
        run.content_decisions.push(row);
        let mut mislogged = chain_draw("knowledge", 0.5, false);
        mislogged.propensity = 1.0;
        let row = knowledge_row("T2", 1, &[mislogged], NORMAL_CONDITION);
        run.content_decisions.push(row);
        // T3's placebo row names no readable attempt.
        let draw = chain_draw(PLACEBO_LAYER, 0.5, false);
        let mut placebo = PlaceboDecisionRecord::new(identity("T3", 1), draw);
        placebo.identity.attempt_key = "not-an-attempt-key".to_string();
        run.placebo_decisions.push(stamped(placebo));

        let report = srm_check(&[run]);
        assert_eq!(report.layers.len(), 1, "{:?}", report.layers);
        let layer = &report.layers[0];
        assert_eq!((layer.layer.as_str(), layer.units), ("knowledge", 3));
        assert!(
            layer.mismatch,
            "an arm with no chance is a mismatch: {layer:?}"
        );
        let key = |task: &str| identity(task, 1).chain_key;
        let line = |rule: &str, example: &str| format!("1 unit(s) {rule}, e.g. {example}");
        let mislogged = format!("{} logs 1.0000, h and g give 0.5000", key("T2"));
        let problems = [
            line(REDRAWN, &key("T0")),
            line(WRONG_PROPENSITY, &mislogged),
            line(NO_CHANCE, &key("T1")),
        ];
        assert_eq!(layer.problems, problems);
        let unreadable = format!("{RUN}: \"not-an-attempt-key\"");
        assert_eq!(report.unreadable, [unreadable]);
        let failures = report.failures();
        assert_eq!(failures.len(), 5, "{failures:?}");
        assert!(
            failures[0].starts_with("knowledge: sample-ratio mismatch over 3 unit(s)"),
            "{failures:?}"
        );
        assert!(
            failures[4].starts_with("arms without a readable attempt key"),
            "{failures:?}"
        );
    }
}
