//! P4-07: Cross-session plan cost aggregation.
//!
//! Loads per-plan cost snapshots and aggregates them into a cross-session
//! view showing total spend, cost trend, and budget utilization.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Aggregated cost information for a single plan across sessions.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PlanCostSummary {
    /// Plan identifier.
    pub plan_id: String,
    /// Total cost in USD.
    pub total_cost_usd: f64,
    /// Number of sessions that contributed cost.
    pub session_count: u64,
    /// Cost per session over time (most recent last).
    pub session_costs: Vec<f64>,
    /// Total tasks completed.
    pub total_tasks: u64,
    /// Average cost per task.
    pub avg_cost_per_task: f64,
    /// Configured budget, if known.
    pub budget_usd: Option<f64>,
}

impl PlanCostSummary {
    /// Budget utilization as a fraction in [0, 1], if budget is set.
    #[must_use]
    pub fn budget_utilization(&self) -> Option<f64> {
        self.budget_usd
            .map(|budget| (self.total_cost_usd / budget).clamp(0.0, 10.0))
    }

    /// Whether the plan is over budget.
    #[must_use]
    pub fn is_over_budget(&self) -> bool {
        self.budget_utilization().is_some_and(|u| u > 1.0)
    }

    /// Cost trend: positive = costs increasing, negative = decreasing.
    #[must_use]
    pub fn cost_trend(&self) -> f64 {
        if self.session_costs.len() < 2 {
            return 0.0;
        }
        let n = self.session_costs.len();
        let recent = &self.session_costs[n / 2..];
        let early = &self.session_costs[..n / 2];
        let recent_avg = recent.iter().sum::<f64>() / recent.len().max(1) as f64;
        let early_avg = early.iter().sum::<f64>() / early.len().max(1) as f64;
        if early_avg.abs() < 1e-12 {
            return 0.0;
        }
        (recent_avg - early_avg) / early_avg
    }
}

/// Aggregated cross-session cost view.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CrossSessionCostReport {
    /// Per-plan summaries.
    pub plans: HashMap<String, PlanCostSummary>,
    /// Total spend across all plans.
    pub total_spend_usd: f64,
    /// Number of unique plans.
    pub plan_count: u64,
    /// Number of plans over budget.
    pub over_budget_count: u64,
}

/// Individual session cost record (input to aggregation).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionCostRecord {
    /// Plan identifier.
    pub plan_id: String,
    /// Session identifier.
    pub session_id: String,
    /// Cost incurred in this session.
    pub cost_usd: f64,
    /// Tasks completed in this session.
    pub tasks_completed: u64,
    /// Budget configured for the plan.
    #[serde(default)]
    pub budget_usd: Option<f64>,
}

impl CrossSessionCostReport {
    /// Aggregate costs from a set of session records.
    #[must_use]
    pub fn aggregate(records: &[SessionCostRecord]) -> Self {
        let mut plans: HashMap<String, PlanCostSummary> = HashMap::new();
        let mut total_spend = 0.0;

        for record in records {
            total_spend += record.cost_usd;
            let summary = plans
                .entry(record.plan_id.clone())
                .or_insert_with(|| PlanCostSummary {
                    plan_id: record.plan_id.clone(),
                    ..Default::default()
                });
            summary.total_cost_usd += record.cost_usd;
            summary.session_count += 1;
            summary.session_costs.push(record.cost_usd);
            summary.total_tasks += record.tasks_completed;
            if let Some(budget) = record.budget_usd {
                summary.budget_usd = Some(budget);
            }
        }

        // Compute per-task averages.
        for summary in plans.values_mut() {
            if summary.total_tasks > 0 {
                summary.avg_cost_per_task = summary.total_cost_usd / summary.total_tasks as f64;
            }
        }

        let over_budget_count = plans.values().filter(|s| s.is_over_budget()).count() as u64;

        CrossSessionCostReport {
            plan_count: plans.len() as u64,
            total_spend_usd: total_spend,
            over_budget_count,
            plans,
        }
    }

    /// Load session cost records from a JSONL file and aggregate.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read.
    pub fn from_jsonl(path: &Path) -> Result<Self, std::io::Error> {
        let contents = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Self::default());
            }
            Err(e) => return Err(e),
        };

        let records: Vec<SessionCostRecord> = contents
            .lines()
            .filter_map(|line| serde_json::from_str(line).ok())
            .collect();

        Ok(Self::aggregate(&records))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aggregation_computes_totals() {
        let records = vec![
            SessionCostRecord {
                plan_id: "plan-1".into(),
                session_id: "s1".into(),
                cost_usd: 0.5,
                tasks_completed: 5,
                budget_usd: Some(2.0),
            },
            SessionCostRecord {
                plan_id: "plan-1".into(),
                session_id: "s2".into(),
                cost_usd: 0.3,
                tasks_completed: 3,
                budget_usd: Some(2.0),
            },
            SessionCostRecord {
                plan_id: "plan-2".into(),
                session_id: "s3".into(),
                cost_usd: 1.0,
                tasks_completed: 10,
                budget_usd: None,
            },
        ];

        let report = CrossSessionCostReport::aggregate(&records);
        assert_eq!(report.plan_count, 2);
        assert!((report.total_spend_usd - 1.8).abs() < 1e-10);
        assert_eq!(report.plans["plan-1"].session_count, 2);
        assert!((report.plans["plan-1"].total_cost_usd - 0.8).abs() < 1e-10);
    }

    #[test]
    fn budget_tracking() {
        let records = vec![SessionCostRecord {
            plan_id: "over".into(),
            session_id: "s1".into(),
            cost_usd: 3.0,
            tasks_completed: 5,
            budget_usd: Some(2.0),
        }];

        let report = CrossSessionCostReport::aggregate(&records);
        assert!(report.plans["over"].is_over_budget());
        assert_eq!(report.over_budget_count, 1);
    }
}
