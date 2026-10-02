//! Structured run-metrics persistence for plan runs.

use serde::{Deserialize, Serialize};
use std::path::Path;

/// A structured record of a completed plan run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunMetricsRecord {
    /// Unique identifier for this run. For a single plan it is the plan's
    /// Graph checkpoint run, which its attempt keys carry; a run of several
    /// plans names each plan's run in [`PlanMetrics::run_id`].
    pub run_id: String,
    /// ISO 8601 timestamp of when the record was captured.
    pub timestamp: String,
    /// Total wall-clock duration in milliseconds.
    pub duration_ms: u64,
    /// Total number of tasks in the run.
    pub total_tasks: usize,
    /// Number of tasks that passed every verify step.
    pub tasks_completed: usize,
    /// Number of tasks whose work was already there: the attempt changed
    /// nothing and every verify step passed on the tree as it was.
    #[serde(default)]
    pub tasks_already_satisfied: usize,
    /// Number of tasks that failed.
    pub tasks_failed: usize,
    /// Number of tasks that completed without running a verify step.
    #[serde(default)]
    pub tasks_unverified: usize,
    /// Number of tasks that never ran.
    #[serde(default)]
    pub tasks_skipped: usize,
    /// Total cost in USD across all providers.
    pub total_cost_usd: f64,
    /// Total input tokens consumed.
    pub total_tokens_in: u64,
    /// Total output tokens produced.
    pub total_tokens_out: u64,
    /// Total number of agent dispatch calls.
    pub total_agent_calls: usize,
    /// Whether the run was halted due to budget exhaustion.
    pub budget_exhausted: bool,
    /// Per-plan breakdown.
    pub plans: Vec<PlanMetrics>,
}

/// Per-plan metrics within a run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanMetrics {
    /// Plan identifier.
    pub plan_id: String,
    /// Whether the plan succeeded: every task passed its verify steps.
    pub completed: bool,
    /// Number of tasks in this plan that passed every verify step.
    pub tasks_completed: usize,
    /// Number of tasks in this plan whose work was already there: the
    /// attempt changed nothing and every verify step passed on the tree as it
    /// was.
    #[serde(default)]
    pub tasks_already_satisfied: usize,
    /// Number of tasks that failed in this plan.
    pub tasks_failed: usize,
    /// Number of tasks in this plan that completed without running a verify
    /// step.
    #[serde(default)]
    pub tasks_unverified: usize,
    /// Number of tasks in this plan that never ran: blocked by a failed task,
    /// or not started.
    #[serde(default)]
    pub tasks_skipped: usize,
    /// The plan's Graph checkpoint run, which its attempt keys carry.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run_id: Option<String>,
}

/// Append a single JSON line to the given path (creates file if not exists).
pub fn append_run_metrics(path: &Path, record: &RunMetricsRecord) -> std::io::Result<()> {
    roko_core::io::append_jsonl(path, record)
}

/// The last `n` records appended to `path`, newest first (`roko show costs`).
/// Lossy: a line that does not parse is skipped, and a missing file holds
/// none.
///
/// # Errors
///
/// Returns an error when the file exists but cannot be read.
pub fn read_recent(path: &Path, n: usize) -> std::io::Result<Vec<RunMetricsRecord>> {
    let content = match std::fs::read_to_string(path) {
        Ok(content) => content,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error),
    };
    Ok(content
        .lines()
        .rev()
        .filter_map(|line| serde_json::from_str(line).ok())
        .take(n)
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_serialization() {
        let record = RunMetricsRecord {
            run_id: "run-abc123".into(),
            timestamp: "2026-08-24T12:00:00Z".into(),
            duration_ms: 45_000,
            total_tasks: 5,
            tasks_completed: 4,
            tasks_already_satisfied: 0,
            tasks_failed: 1,
            tasks_unverified: 0,
            tasks_skipped: 0,
            total_cost_usd: 0.35,
            total_tokens_in: 10_000,
            total_tokens_out: 3_000,
            total_agent_calls: 5,
            budget_exhausted: false,
            plans: vec![PlanMetrics {
                plan_id: "plan-1".into(),
                completed: true,
                tasks_completed: 4,
                tasks_already_satisfied: 0,
                tasks_failed: 1,
                tasks_unverified: 0,
                tasks_skipped: 0,
                run_id: Some("graph-plan-1-run".into()),
            }],
        };

        let json = serde_json::to_string(&record).unwrap();
        let deser: RunMetricsRecord = serde_json::from_str(&json).unwrap();

        assert_eq!(deser.run_id, "run-abc123");
        assert_eq!(deser.duration_ms, 45_000);
        assert_eq!(deser.total_tasks, 5);
        assert_eq!(deser.tasks_completed, 4);
        assert_eq!(deser.tasks_failed, 1);
        assert_eq!(deser.total_cost_usd, 0.35);
        assert_eq!(deser.plans.len(), 1);
        assert!(deser.plans[0].completed);
        assert_eq!(deser.plans[0].run_id.as_deref(), Some("graph-plan-1-run"));
    }

    #[test]
    fn append_creates_file_and_writes_lines() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("metrics.jsonl");

        let record = RunMetricsRecord {
            run_id: "r1".into(),
            timestamp: "2026-08-24T00:00:00Z".into(),
            duration_ms: 1_000,
            total_tasks: 1,
            tasks_completed: 1,
            tasks_already_satisfied: 0,
            tasks_failed: 0,
            tasks_unverified: 0,
            tasks_skipped: 0,
            total_cost_usd: 0.01,
            total_tokens_in: 500,
            total_tokens_out: 200,
            total_agent_calls: 1,
            budget_exhausted: false,
            plans: vec![],
        };

        append_run_metrics(&path, &record).unwrap();
        append_run_metrics(&path, &record).unwrap();

        let contents = std::fs::read_to_string(&path).unwrap();
        let lines: Vec<&str> = contents.lines().collect();
        assert_eq!(lines.len(), 2);

        let parsed: RunMetricsRecord = serde_json::from_str(lines[0]).unwrap();
        assert_eq!(parsed.run_id, "r1");
    }

    /// `read_recent` returns the last rows first, at most `n`, skipping a
    /// line that does not parse; a missing file holds none.
    #[test]
    fn read_recent_returns_the_newest_rows_first() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("run-metrics.jsonl");
        assert!(read_recent(&path, 10).unwrap().is_empty());

        for run_id in ["r1", "r2", "r3"] {
            let record = RunMetricsRecord {
                run_id: run_id.into(),
                timestamp: "2026-10-02T00:00:00Z".into(),
                duration_ms: 1_000,
                total_tasks: 1,
                tasks_completed: 1,
                tasks_already_satisfied: 0,
                tasks_failed: 0,
                tasks_unverified: 0,
                tasks_skipped: 0,
                total_cost_usd: 0.01,
                total_tokens_in: 500,
                total_tokens_out: 200,
                total_agent_calls: 1,
                budget_exhausted: false,
                plans: vec![],
            };
            append_run_metrics(&path, &record).unwrap();
            if run_id == "r2" {
                roko_core::io::append_jsonl(&path, &serde_json::json!({"torn": true})).unwrap();
            }
        }

        let ids = |n| -> Vec<String> {
            read_recent(&path, n)
                .unwrap()
                .into_iter()
                .map(|record| record.run_id)
                .collect()
        };
        assert_eq!(ids(10), ["r3", "r2", "r1"]);
        assert_eq!(ids(2), ["r3", "r2"]);
    }

    /// Rows written before the unverified and skipped counts existed still
    /// parse, with both counts zero.
    #[test]
    fn rows_without_unverified_or_skipped_counts_still_parse() {
        let row = r#"{"run_id":"graph-run-1","timestamp":"2026-09-28T12:00:00Z","duration_ms":1000,"total_tasks":2,"tasks_completed":2,"tasks_failed":0,"total_cost_usd":0.5,"total_tokens_in":10,"total_tokens_out":5,"total_agent_calls":2,"budget_exhausted":false,"plans":[{"plan_id":"p1","completed":true,"tasks_completed":2,"tasks_failed":0}]}"#;

        let parsed: RunMetricsRecord = serde_json::from_str(row).unwrap();

        assert_eq!(parsed.tasks_completed, 2);
        assert_eq!(parsed.tasks_unverified, 0);
        assert_eq!(parsed.tasks_skipped, 0);
        assert_eq!(parsed.plans[0].tasks_unverified, 0);
        assert_eq!(parsed.plans[0].tasks_skipped, 0);
        assert_eq!(parsed.tasks_already_satisfied, 0);
        assert_eq!(parsed.plans[0].tasks_already_satisfied, 0);
    }
}
