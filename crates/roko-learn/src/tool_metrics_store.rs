//! P4-17: Persistent tool metrics aggregation store.
//!
//! Appends `(MetricsKey, ToolMetrics)` records to `.roko/learn/tool-metrics.jsonl`
//! so that per-tool usage, success rates, and costs can be queried for
//! tool recommendation and degradation detection.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Write;
use std::path::PathBuf;

/// Key identifying a tool usage observation.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MetricsKey {
    /// Tool name (e.g., "Read", "Bash", "Edit").
    pub tool_name: String,
    /// Agent role that used the tool.
    #[serde(default)]
    pub role: String,
    /// Model that was dispatched.
    #[serde(default)]
    pub model: String,
}

/// Aggregated metrics for one tool.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ToolMetrics {
    /// Total invocations.
    pub total_calls: u64,
    /// Successful invocations (no error response).
    pub success_count: u64,
    /// Total wall-clock duration across all calls (ms).
    pub total_duration_ms: u64,
    /// Total cost attributed to this tool (USD).
    pub total_cost_usd: f64,
    /// Most recent invocation timestamp (ISO 8601).
    #[serde(default)]
    pub last_used: String,
}

impl ToolMetrics {
    /// Success rate as a fraction in [0, 1].
    #[must_use]
    pub fn success_rate(&self) -> f64 {
        if self.total_calls == 0 {
            return 0.0;
        }
        self.success_count as f64 / self.total_calls as f64
    }

    /// Average duration per call in milliseconds.
    #[must_use]
    pub fn avg_duration_ms(&self) -> f64 {
        if self.total_calls == 0 {
            return 0.0;
        }
        self.total_duration_ms as f64 / self.total_calls as f64
    }
}

/// One JSONL record persisted per tool observation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolMetricsRecord {
    /// Timestamp of the observation.
    pub timestamp: String,
    /// Tool identification key.
    pub key: MetricsKey,
    /// Whether this specific invocation succeeded.
    pub success: bool,
    /// Duration of this specific invocation (ms).
    pub duration_ms: u64,
    /// Cost of this specific invocation (USD).
    pub cost_usd: f64,
    /// Plan ID context.
    #[serde(default)]
    pub plan_id: String,
    /// Task ID context.
    #[serde(default)]
    pub task_id: String,
}

/// Persistent JSONL-backed tool metrics store.
pub struct ToolMetricsStore {
    path: PathBuf,
}

impl ToolMetricsStore {
    /// Create a store at the given JSONL path.
    #[must_use]
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// Append a single tool metrics record.
    ///
    /// # Errors
    ///
    /// Returns an error if the record cannot be serialized or appended.
    pub fn append(&self, record: &ToolMetricsRecord) -> Result<(), std::io::Error> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;
        let line = serde_json::to_string(record)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        writeln!(file, "{line}")?;
        Ok(())
    }

    /// Read all records and aggregate by tool name.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read.
    pub fn aggregate(&self) -> Result<HashMap<String, ToolMetrics>, std::io::Error> {
        let contents = match std::fs::read_to_string(&self.path) {
            Ok(c) => c,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(HashMap::new()),
            Err(e) => return Err(e),
        };

        let mut aggregates: HashMap<String, ToolMetrics> = HashMap::new();
        for line in contents.lines() {
            if let Ok(record) = serde_json::from_str::<ToolMetricsRecord>(line) {
                let metrics = aggregates
                    .entry(record.key.tool_name.clone())
                    .or_default();
                metrics.total_calls += 1;
                if record.success {
                    metrics.success_count += 1;
                }
                metrics.total_duration_ms += record.duration_ms;
                metrics.total_cost_usd += record.cost_usd;
                if record.timestamp > metrics.last_used {
                    metrics.last_used = record.timestamp;
                }
            }
        }
        Ok(aggregates)
    }

    /// Aggregate by (tool_name, role) for role-specific analysis.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read.
    pub fn aggregate_by_role(&self) -> Result<HashMap<(String, String), ToolMetrics>, std::io::Error> {
        let contents = match std::fs::read_to_string(&self.path) {
            Ok(c) => c,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(HashMap::new()),
            Err(e) => return Err(e),
        };

        let mut aggregates: HashMap<(String, String), ToolMetrics> = HashMap::new();
        for line in contents.lines() {
            if let Ok(record) = serde_json::from_str::<ToolMetricsRecord>(line) {
                let key = (record.key.tool_name.clone(), record.key.role.clone());
                let metrics = aggregates.entry(key).or_default();
                metrics.total_calls += 1;
                if record.success {
                    metrics.success_count += 1;
                }
                metrics.total_duration_ms += record.duration_ms;
                metrics.total_cost_usd += record.cost_usd;
                if record.timestamp > metrics.last_used {
                    metrics.last_used = record.timestamp;
                }
            }
        }
        Ok(aggregates)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_metrics_success_rate() {
        let mut metrics = ToolMetrics::default();
        metrics.total_calls = 10;
        metrics.success_count = 8;
        assert!((metrics.success_rate() - 0.8).abs() < 1e-10);
    }

    #[test]
    fn tool_metrics_avg_duration() {
        let mut metrics = ToolMetrics::default();
        metrics.total_calls = 4;
        metrics.total_duration_ms = 1000;
        assert!((metrics.avg_duration_ms() - 250.0).abs() < 1e-10);
    }

    #[test]
    fn store_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("tool-metrics.jsonl");
        let store = ToolMetricsStore::new(&path);

        let record = ToolMetricsRecord {
            timestamp: "2026-09-06T00:00:00Z".to_string(),
            key: MetricsKey {
                tool_name: "Bash".to_string(),
                role: "implementer".to_string(),
                model: "claude-sonnet-4".to_string(),
            },
            success: true,
            duration_ms: 500,
            cost_usd: 0.01,
            plan_id: "plan-1".to_string(),
            task_id: "task-1".to_string(),
        };
        store.append(&record).unwrap();
        store.append(&record).unwrap();

        let agg = store.aggregate().unwrap();
        assert_eq!(agg["Bash"].total_calls, 2);
        assert_eq!(agg["Bash"].success_count, 2);
    }
}
