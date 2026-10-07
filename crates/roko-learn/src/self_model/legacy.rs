//! Normalise the legacy `efficiency.jsonl` and `episodes.jsonl` logs into deduplicated
//! units with an honest data audit: backlog task 6110.
//!
//! Before S01 the only local data was two logs: `.roko/learn/efficiency.jsonl`
//! ([`AgentEfficiencyEvent`](crate::efficiency::AgentEfficiencyEvent) rows) and
//! `.roko/episodes.jsonl` ([`Episode`](crate::episode_logger::Episode) rows). Neither has a usable
//! attempt key: one `attempt_id` held 178 rows from a runaway loop, and episodes carry no gate
//! verdicts. [`normalize`] reads both leniently and [`dedupe_units`] keeps one unit per
//! (plan, task): its final labelled turn, a gate label before the agent's own claim of success
//! on the same turn. Rows sharing an `attempt_id` collapse into one attempt in the audit.
//!
//! These logs only smoke-test the pipeline (S04 §0): never present them as calibration
//! evidence. Their costs come from the old price tables, so the units carry none.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::path::Path;

use serde::Deserialize;

use super::ingest::{DataAudit, IngestReport};
use super::{ArmKey, Label, LabelSource, Unit};
use crate::telemetry::{AttemptKey, CostSource};

/// The run id of every legacy unit's synthetic attempt key: the logs have no run.
pub const LEGACY_RUN_ID: &str = "legacy";

/// One legacy row, normalised: the fields the dedupe reads.
#[derive(Debug, Clone, PartialEq)]
pub struct LegacyRow {
    /// The plan.
    pub plan_id: String,
    /// The task.
    pub task_id: String,
    /// The row's attempt id; `None` when the row has none, so it is an attempt of its own.
    pub attempt_id: Option<String>,
    /// The attempt's iteration (efficiency rows; 0 for episodes).
    pub iteration: u32,
    /// The turn within the iteration (efficiency rows; 0 for episodes).
    pub turn: u32,
    /// When the row was written (RFC 3339).
    pub timestamp: String,
    /// The model.
    pub model: String,
    /// The backend that ran it.
    pub backend: String,
    /// The role.
    pub role: String,
    /// The row's label and where it came from, when it has one.
    pub label: Option<(bool, LabelSource)>,
    /// Wall time in milliseconds; 0 when unknown.
    pub wall_time_ms: u64,
}

impl LegacyRow {
    /// The order of a (plan, task)'s labelled rows: the last turn wins, and on the same turn a
    /// gate label beats the agent's own claim, then the later row.
    fn order(&self) -> (u32, u32, bool, &str) {
        let gate = matches!(self.label, Some((_, LabelSource::GatePassed)));
        (self.iteration, self.turn, gate, self.timestamp.as_str())
    }
}

/// An `efficiency.jsonl` row, read leniently with `AgentEfficiencyEvent`'s field names.
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct EfficiencyLine {
    plan_id: String,
    task_id: String,
    attempt_id: String,
    iteration: u32,
    turn_number: u32,
    gate_passed: Option<bool>,
    outcome: Option<String>,
    model: String,
    backend: String,
    role: String,
    wall_time_ms: u64,
    timestamp: String,
}

impl EfficiencyLine {
    /// The row's label: the gate verdict, else its outcome's name.
    fn label(&self) -> Option<(bool, LabelSource)> {
        if let Some(passed) = self.gate_passed {
            return Some((passed, LabelSource::GatePassed));
        }
        match self.outcome.as_deref()? {
            "gate_pass" => Some((true, LabelSource::GatePassed)),
            "gate_failure" => Some((false, LabelSource::GatePassed)),
            "success" => Some((true, LabelSource::PreGateSuccess)),
            "failure" => Some((false, LabelSource::PreGateSuccess)),
            _ => None,
        }
    }
}

/// An `episodes.jsonl` row, read leniently; its plan id sits in `extra`.
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct EpisodeLine {
    id: String,
    task_id: String,
    model: String,
    backend: String,
    success: bool,
    timestamp: String,
    duration_secs: f64,
    extra: serde_json::Map<String, serde_json::Value>,
}

/// Read the two legacy logs' text into deduplicated units and their audit. Efficiency rows
/// win: an episode only adds a (plan, task) the efficiency log does not hold.
pub fn normalize(efficiency: &str, episodes: &str) -> IngestReport {
    let mut audit = DataAudit::default();
    let efficiency_rows = parse_lines(efficiency, &mut audit, efficiency_row);
    let episode_rows = parse_lines(episodes, &mut audit, episode_row);

    let attempts: BTreeSet<(&str, &str, String)> = efficiency_rows
        .iter()
        .enumerate()
        .map(|(index, row)| {
            let attempt = row
                .attempt_id
                .clone()
                .unwrap_or_else(|| format!("#row{index}"));
            (row.plan_id.as_str(), row.task_id.as_str(), attempt)
        })
        .collect();
    audit.attempts = attempts.len() + episode_rows.len();

    let efficiency_tasks: BTreeSet<(&str, &str)> = efficiency_rows
        .iter()
        .map(|row| (row.plan_id.as_str(), row.task_id.as_str()))
        .collect();
    let mut units = dedupe_units(&efficiency_rows);
    audit.unlabelled = efficiency_tasks.len() - units.len();
    let new_episodes: Vec<LegacyRow> = episode_rows
        .into_iter()
        .filter(|row| !efficiency_tasks.contains(&(row.plan_id.as_str(), row.task_id.as_str())))
        .collect();
    units.extend(dedupe_units(&new_episodes));
    for unit in &units {
        audit.count_labelled(unit);
    }
    IngestReport { units, audit }
}

/// [`normalize`] the log files; a missing file holds no rows.
pub fn read_files(efficiency: &Path, episodes: &Path) -> std::io::Result<IngestReport> {
    let read = |path: &Path| match std::fs::read_to_string(path) {
        Ok(text) => Ok(text),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(String::new()),
        Err(error) => Err(error),
    };
    Ok(normalize(&read(efficiency)?, &read(episodes)?))
}

/// The rows of `text` that `row` reads, counting what it cannot.
fn parse_lines(
    text: &str,
    audit: &mut DataAudit,
    row: fn(&str) -> Option<Option<LegacyRow>>,
) -> Vec<LegacyRow> {
    let mut rows = Vec::new();
    for line in text.lines().filter(|line| !line.trim().is_empty()) {
        audit.rows += 1;
        match row(line) {
            Some(Some(parsed)) => rows.push(parsed),
            Some(None) => audit.unkeyed += 1,
            None => audit.unreadable += 1,
        }
    }
    rows
}

/// An efficiency line: `None` when it does not parse, `Some(None)` when it has no plan or task.
fn efficiency_row(line: &str) -> Option<Option<LegacyRow>> {
    let parsed: EfficiencyLine = serde_json::from_str(line).ok()?;
    if parsed.plan_id.is_empty() || parsed.task_id.is_empty() {
        return Some(None);
    }
    let label = parsed.label();
    Some(Some(LegacyRow {
        attempt_id: Some(parsed.attempt_id).filter(|id| !id.is_empty()),
        plan_id: parsed.plan_id,
        task_id: parsed.task_id,
        iteration: parsed.iteration,
        turn: parsed.turn_number,
        timestamp: parsed.timestamp,
        model: parsed.model,
        backend: parsed.backend,
        role: parsed.role,
        label,
        wall_time_ms: parsed.wall_time_ms,
    }))
}

/// An episode line: `None` when it does not parse, `Some(None)` when it has no task.
fn episode_row(line: &str) -> Option<Option<LegacyRow>> {
    let parsed: EpisodeLine = serde_json::from_str(line).ok()?;
    if parsed.task_id.is_empty() {
        return Some(None);
    }
    let plan_id = parsed
        .extra
        .get("plan_id")
        .and_then(serde_json::Value::as_str)
        .filter(|plan| !plan.is_empty())
        .unwrap_or("-")
        .to_string();
    Some(Some(LegacyRow {
        plan_id,
        task_id: parsed.task_id,
        attempt_id: Some(parsed.id).filter(|id| !id.is_empty()),
        iteration: 0,
        turn: 0,
        timestamp: parsed.timestamp,
        model: parsed.model,
        backend: parsed.backend,
        role: "unknown".to_string(),
        // Episodes carry no gate verdicts: their success is the agent's own.
        label: Some((parsed.success, LabelSource::PreGateSuccess)),
        wall_time_ms: (parsed.duration_secs * 1_000.0).max(0.0) as u64,
    }))
}

/// One unit per (plan, task) of `rows` that has a labelled row: its final labelled turn
/// ([`LegacyRow`]'s order). S06.T1's EV fold shares this dedupe instead of writing its own.
pub fn dedupe_units(rows: &[LegacyRow]) -> Vec<Unit> {
    let mut tasks: BTreeMap<(&str, &str), Vec<&LegacyRow>> = BTreeMap::new();
    for row in rows {
        tasks
            .entry((row.plan_id.as_str(), row.task_id.as_str()))
            .or_default()
            .push(row);
    }
    tasks
        .into_values()
        .filter_map(|task_rows| unit_of(&task_rows))
        .collect()
}

/// The unit of one (plan, task)'s rows, `None` when none is labelled.
fn unit_of(rows: &[&LegacyRow]) -> Option<Unit> {
    let labelled: Vec<&LegacyRow> = rows
        .iter()
        .copied()
        .filter(|row| row.label.is_some())
        .collect();
    let last = labelled
        .iter()
        .copied()
        .max_by(|a, b| a.order().cmp(&b.order()))?;
    let (passed, source) = last.label?;
    let attempts: BTreeSet<&str> = rows
        .iter()
        .filter_map(|row| row.attempt_id.as_deref())
        .collect();
    let unnamed = rows.iter().filter(|row| row.attempt_id.is_none()).count();
    let attempt = u32::try_from(attempts.len() + unnamed)
        .unwrap_or(u32::MAX)
        .max(1);
    let prior_failure = labelled
        .iter()
        .any(|row| matches!(row.label, Some((false, _))) && row.order() < last.order());
    let or_unknown = |value: &str| {
        if value.is_empty() {
            "unknown".to_string()
        } else {
            value.to_string()
        }
    };
    Some(Unit {
        attempt_key: AttemptKey::new(LEGACY_RUN_ID, &last.plan_id, &last.task_id, attempt),
        plan_id: last.plan_id.clone(),
        task_id: last.task_id.clone(),
        role: or_unknown(&last.role),
        tier: "unknown".to_string(),
        family: "unknown".to_string(),
        arm: ArmKey::roko(or_unknown(&last.backend), or_unknown(&last.model)),
        attempt,
        prior_failure,
        failure_class: None,
        // A pre-gate success stands in for the gate verdict; its source says so.
        label: Label {
            y_gate: Some(passed),
            y_vs: None,
            weight: 1.0,
            source,
        },
        api_equiv_usd: None,
        cost_source: CostSource::Unknown,
        latency_s: (last.wall_time_ms > 0).then_some(last.wall_time_ms as f64 / 1_000.0),
        failover: false,
    })
}

impl fmt::Display for DataAudit {
    /// The raw and deduplicated counts and the label-source histogram, for the CLI (6124).
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "rows read: {} ({} unreadable, {} without a plan or task)",
            self.rows, self.unreadable, self.unkeyed
        )?;
        writeln!(
            f,
            "attempts: {} ({} abandoned)",
            self.attempts, self.abandoned
        )?;
        writeln!(
            f,
            "units: {} labelled, {} unlabelled, {} failover substitutes",
            self.labelled, self.unlabelled, self.failover
        )?;
        let histogram = |counts: &BTreeMap<String, usize>| {
            counts
                .iter()
                .map(|(name, count)| format!("{name} {count}"))
                .collect::<Vec<_>>()
                .join(", ")
        };
        writeln!(f, "label sources: {}", histogram(&self.label_sources))?;
        write!(f, "cost sources: {}", histogram(&self.cost_sources))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(name: &str) -> String {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/self_model/legacy")
            .join(name);
        std::fs::read_to_string(path).expect("read the legacy fixture")
    }

    /// The fixture is a scrubbed subset of the two live logs as of 2026-10-02 (ids, labels,
    /// costs and iterations only), pinned by hash: seven (plan, task)s of efficiency rows,
    /// three rows with no plan, and five episodes, one of whose (plan, task)s the efficiency
    /// log already holds.
    #[test]
    fn legacy_dedupe_reproduces_the_pinned_audit() {
        let efficiency = fixture("efficiency.jsonl");
        let episodes = fixture("episodes.jsonl");
        assert_eq!(
            blake3::hash(efficiency.as_bytes()).to_hex().as_str(),
            "27ee43041d1097a2bb56034e8885cdccc1cb8fceb69448632c4c51f2d00d75c0"
        );
        assert_eq!(
            blake3::hash(episodes.as_bytes()).to_hex().as_str(),
            "911ee25b78a9c1f5c918041aecba6c8f0fd60d38beb32513defe92760049ea17"
        );

        let report = normalize(&efficiency, &episodes);
        let audit = &report.audit;
        assert_eq!(audit.rows, 27);
        assert_eq!(audit.unreadable, 0);
        assert_eq!(audit.unkeyed, 3);
        assert_eq!(audit.attempts, 22);
        assert_eq!(audit.labelled, 9);
        assert_eq!(audit.unlabelled, 1);
        assert_eq!(audit.label_sources.get("gate_passed"), Some(&4));
        assert_eq!(audit.label_sources.get("pre_gate_success"), Some(&5));
        assert_eq!(audit.cost_sources.get("unknown"), Some(&9));

        let unit = |plan: &str, task: &str| {
            report
                .units
                .iter()
                .find(|unit| unit.plan_id == plan && unit.task_id == task)
                .unwrap_or_else(|| panic!("no unit {plan}/{task}"))
        };
        // The gate row beats the agent's own claim on the same final turn.
        let polished = unit("08f-final-polish", "T01");
        assert_eq!(polished.label.source, LabelSource::GatePassed);
        assert_eq!(polished.label.y_gate, Some(true));
        let failed = unit("02-backend-plan-execution", "T08");
        assert_eq!(failed.label.y_gate, Some(false));
        assert!(failed.prior_failure);
        assert_eq!(
            unit("E01-execution-engine", "E01-T01").label.y_gate,
            Some(false)
        );
        assert_eq!(
            unit("cli-ux-consistency", "T02").label.source,
            LabelSource::PreGateSuccess,
            "the efficiency row, not the episode"
        );
        let unpriced = report.units.iter().all(|unit| unit.api_equiv_usd.is_none());
        assert!(unpriced, "legacy costs are not snapshot prices");

        let printed = audit.to_string();
        assert!(printed.contains("rows read: 27"), "{printed}");
        let sources = "label sources: gate_passed 4, pre_gate_success 5";
        assert!(printed.contains(sources), "{printed}");
    }

    #[test]
    fn missing_logs_hold_no_rows() {
        let empty = tempfile::tempdir().expect("tempdir");
        let report = read_files(
            &empty.path().join("efficiency.jsonl"),
            &empty.path().join("episodes.jsonl"),
        )
        .expect("missing files");
        assert_eq!(report, IngestReport::default());
    }
}
