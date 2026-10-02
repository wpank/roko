//! Read-only checks and reports over a run's attempt records (S01 P0-13,
//! §5.9, §7): what `roko learn telemetry check` and `route-report` print.
//!
//! Everything here reads files alone and writes nothing.
//! [`RunRecords::load`] parses one run directory's `attempts.jsonl`,
//! `decisions.jsonl` (route and content decisions), `exposures.jsonl` and
//! `census.json`, and [`LegacyRows::load`] finds the run's attempt keys in
//! the logs that predate S01 (`learn/efficiency.jsonl`, `learn/costs.jsonl`,
//! `episodes.jsonl`). [`check`] validates one run; [`route_report`] counts
//! routing outcomes per decision source over any number of runs.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::path::Path;

use chrono::{DateTime, Utc};
use roko_fs::layout::RokoLayout;
use serde::Serialize;
use serde_json::Value;

use super::census::{CENSUS_FILE, CensusReport};
use super::manifest::AttemptTally;
use super::records::{
    ATTEMPT_OPEN_SCHEMA, AttemptKey, AttemptOpenRecord, AttemptVerdictRecord, ContentDecisionRecord,
    DECISION_SCHEMA, DecisionSource, EXPOSURE_SCHEMA, ExecutedModel, ExposureRecord, RunFile,
    Stamped, VERDICT_SCHEMA,
};
use crate::error::LearnError;
use crate::routing_log::{ROUTE_DECISION_POINT, RoutingDecisionLog};

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
    /// Exposure rows, in file order.
    pub exposures: Vec<Stamped<ExposureRecord>>,
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
            (RunFile::Decisions, DECISION_SCHEMA) => {
                serde_json::from_value(value).map(|record| self.content_decisions.push(record))
            }
            (RunFile::Exposures, EXPOSURE_SCHEMA) => {
                serde_json::from_value(value).map(|record| self.exposures.push(record))
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
    /// Keys of the `episodes.jsonl` rows (`extra.attempt_key`).
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
        Ok(Self {
            efficiency: run_keys(
                &learn_dir.join("efficiency.jsonl"),
                run_id,
                &["attempt_key"],
            )?,
            costs: run_keys(&learn_dir.join("costs.jsonl"), run_id, &["attempt_key"])?,
            episodes: run_keys(
                &layout.root_episodes_path(),
                run_id,
                &["extra", "attempt_key"],
            )?,
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
fn undated(model: &str) -> &str {
    match model.rsplit_once('-') {
        Some((base, date)) if date.len() == 8 && date.bytes().all(|b| b.is_ascii_digit()) => base,
        _ => model,
    }
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;
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
}
