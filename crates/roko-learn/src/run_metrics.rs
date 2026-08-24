//! Structured run-metrics persistence for plan runs.

use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::Path;

/// A structured record of a completed plan run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunMetricsRecord {
    /// Unique identifier for this run.
    pub run_id: String,
    /// ISO 8601 timestamp of when the record was captured.
    pub timestamp: String,
    /// Total wall-clock duration in milliseconds.
    pub duration_ms: u64,
    /// Total number of tasks in the run.
    pub total_tasks: usize,
    /// Number of tasks that completed successfully.
    pub tasks_completed: usize,
    /// Number of tasks that failed.
    pub tasks_failed: usize,
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
    /// Whether the plan completed successfully.
    pub completed: bool,
    /// Number of tasks that completed in this plan.
    pub tasks_completed: usize,
    /// Number of tasks that failed in this plan.
    pub tasks_failed: usize,
}

/// Append a single JSON line to the given path (creates file if not exists).
pub fn append_run_metrics(path: &Path, record: &RunMetricsRecord) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    let line = serde_json::to_string(record).map_err(std::io::Error::other)?;
    writeln!(file, "{line}")?;
    Ok(())
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
            tasks_failed: 1,
            total_cost_usd: 0.35,
            total_tokens_in: 10_000,
            total_tokens_out: 3_000,
            total_agent_calls: 5,
            budget_exhausted: false,
            plans: vec![PlanMetrics {
                plan_id: "plan-1".into(),
                completed: true,
                tasks_completed: 4,
                tasks_failed: 1,
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
            tasks_failed: 0,
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
}
