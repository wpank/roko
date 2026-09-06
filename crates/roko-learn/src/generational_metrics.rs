//! P4-20: Generational improvement metrics.
//!
//! Tracks per-generation success rate, average cost, knowledge entry count,
//! distillation yield, and playbook size. Persisted to
//! `.roko/learn/generational-metrics.jsonl`.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::PathBuf;

/// Metrics for one generation (plan run or dream cycle).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationalMetrics {
    /// Monotonically increasing generation number.
    pub generation: u64,
    /// Timestamp when this generation completed.
    pub timestamp: DateTime<Utc>,
    /// Task success rate for this generation.
    pub success_rate: f64,
    /// Average cost per task (USD).
    pub avg_cost_usd: f64,
    /// Total cost for this generation (USD).
    pub total_cost_usd: f64,
    /// Number of knowledge entries after this generation.
    pub knowledge_entry_count: u64,
    /// Number of new knowledge entries created in this generation.
    pub new_knowledge_entries: u64,
    /// Distillation yield (entries distilled / candidates).
    pub distillation_yield: f64,
    /// Total playbook count after this generation.
    pub playbook_count: u64,
    /// Number of new playbooks created in this generation.
    pub new_playbooks: u64,
    /// Number of tasks completed.
    pub tasks_completed: u64,
    /// Number of tasks that succeeded.
    pub tasks_succeeded: u64,
    /// Plan ID associated with this generation, if any.
    #[serde(default)]
    pub plan_id: String,
}

impl GenerationalMetrics {
    /// Compute the improvement ratio vs a previous generation.
    ///
    /// Returns a map of metric name to (current, previous, improvement_ratio).
    #[must_use]
    pub fn improvement_vs(&self, previous: &GenerationalMetrics) -> Vec<MetricDelta> {
        let mut deltas = Vec::new();

        deltas.push(MetricDelta {
            name: "success_rate".to_string(),
            current: self.success_rate,
            previous: previous.success_rate,
            improvement: delta_ratio(self.success_rate, previous.success_rate),
        });

        deltas.push(MetricDelta {
            name: "avg_cost_usd".to_string(),
            current: self.avg_cost_usd,
            previous: previous.avg_cost_usd,
            // For cost, lower is better.
            improvement: if previous.avg_cost_usd > 0.0 {
                (previous.avg_cost_usd - self.avg_cost_usd) / previous.avg_cost_usd
            } else {
                0.0
            },
        });

        deltas.push(MetricDelta {
            name: "distillation_yield".to_string(),
            current: self.distillation_yield,
            previous: previous.distillation_yield,
            improvement: delta_ratio(self.distillation_yield, previous.distillation_yield),
        });

        deltas.push(MetricDelta {
            name: "knowledge_entries".to_string(),
            current: self.knowledge_entry_count as f64,
            previous: previous.knowledge_entry_count as f64,
            improvement: delta_ratio(
                self.knowledge_entry_count as f64,
                previous.knowledge_entry_count as f64,
            ),
        });

        deltas
    }
}

/// Improvement delta between two generations for one metric.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricDelta {
    /// Metric name.
    pub name: String,
    /// Current value.
    pub current: f64,
    /// Previous value.
    pub previous: f64,
    /// Improvement ratio (positive = better).
    pub improvement: f64,
}

fn delta_ratio(current: f64, previous: f64) -> f64 {
    if previous.abs() < 1e-12 {
        return 0.0;
    }
    (current - previous) / previous
}

/// Persistent store for generational metrics.
pub struct GenerationalMetricsStore {
    path: PathBuf,
}

impl GenerationalMetricsStore {
    /// Create a store at the given JSONL path.
    #[must_use]
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// Append a generation's metrics.
    ///
    /// # Errors
    ///
    /// Returns an error if the record cannot be serialized or appended.
    pub fn append(&self, metrics: &GenerationalMetrics) -> Result<(), std::io::Error> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;
        let line = serde_json::to_string(metrics)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        writeln!(file, "{line}")?;
        Ok(())
    }

    /// Read all generational metrics.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read.
    pub fn read_all(&self) -> Result<Vec<GenerationalMetrics>, std::io::Error> {
        let contents = match std::fs::read_to_string(&self.path) {
            Ok(c) => c,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(e) => return Err(e),
        };

        let mut metrics = Vec::new();
        for line in contents.lines() {
            if let Ok(m) = serde_json::from_str::<GenerationalMetrics>(line) {
                metrics.push(m);
            }
        }
        Ok(metrics)
    }

    /// Get the latest generation number.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read.
    pub fn latest_generation(&self) -> Result<u64, std::io::Error> {
        let all = self.read_all()?;
        Ok(all.last().map(|m| m.generation).unwrap_or(0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_metrics(generation: u64, success_rate: f64) -> GenerationalMetrics {
        GenerationalMetrics {
            generation,
            timestamp: Utc::now(),
            success_rate,
            avg_cost_usd: 0.05,
            total_cost_usd: 0.5,
            knowledge_entry_count: 100 + generation * 10,
            new_knowledge_entries: 10,
            distillation_yield: 0.6,
            playbook_count: 5 + generation,
            new_playbooks: 1,
            tasks_completed: 10,
            tasks_succeeded: (success_rate * 10.0) as u64,
            plan_id: format!("plan-{generation}"),
        }
    }

    #[test]
    fn improvement_tracking() {
        let gen1 = sample_metrics(1, 0.6);
        let gen2 = sample_metrics(2, 0.8);

        let deltas = gen2.improvement_vs(&gen1);
        let success_delta = deltas.iter().find(|d| d.name == "success_rate").unwrap();
        assert!(success_delta.improvement > 0.0, "success rate should improve");
    }

    #[test]
    fn store_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("generational-metrics.jsonl");
        let store = GenerationalMetricsStore::new(&path);

        store.append(&sample_metrics(1, 0.7)).unwrap();
        store.append(&sample_metrics(2, 0.85)).unwrap();

        let all = store.read_all().unwrap();
        assert_eq!(all.len(), 2);
        assert_eq!(store.latest_generation().unwrap(), 2);
    }
}
