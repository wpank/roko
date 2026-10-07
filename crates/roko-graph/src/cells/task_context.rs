//! TaskContextCell -- collects task metadata and prior attempt state for plan execution.
//!
//! Replaces the `PassthroughCell` stub for `"plan.task-context"`. This cell is the
//! root of each per-task subgraph in the production plan topology. It:
//!
//! 1. Reads task metadata from its TOML node config (populated by
//!    [`ProductionPlanTopology::build_context_config`]).
//! 2. Extracts prior attempt context from upstream input signals (predecessor
//!    task success boundaries feed into this cell's inputs).
//! 3. Outputs a single `Kind::Task` signal whose `Body::Json` payload contains
//!    the assembled task context for the downstream compose cell.

use std::time::Duration;

use async_trait::async_trait;
use roko_core::{Body, Kind, ProtocolId, Signal, error::Result};
use serde::{Deserialize, Serialize};

use crate::cell::{Cell, CellContext, CellVersion};

/// Configuration parsed from the node's TOML config table.
///
/// These fields mirror what [`ProductionPlanTopology::build_context_config`]
/// writes into the node config.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TaskContextConfig {
    /// Enclosing plan identifier.
    #[serde(default)]
    pub plan_id: String,
    /// Source plan directory path.
    #[serde(default)]
    pub plan_dir: String,
    /// Task identifier (unique within the plan).
    #[serde(default)]
    pub task_id: String,
    /// Human-readable task title.
    #[serde(default)]
    pub title: String,
    /// Detailed task description.
    #[serde(default)]
    pub description: Option<String>,
    /// Requested agent role.
    #[serde(default)]
    pub role: Option<String>,
    /// Complexity tier label.
    #[serde(default)]
    pub tier: String,
    /// Model hint for dispatch.
    #[serde(default)]
    pub model_hint: Option<String>,
    /// Work domain.
    #[serde(default)]
    pub domain: Option<String>,
    /// Per-task timeout in seconds.
    #[serde(default)]
    pub timeout_secs: u64,
    /// Definition order index.
    #[serde(default)]
    pub sequence: i64,
    /// Files expected to be in scope.
    #[serde(default)]
    pub files: Vec<String>,
}

impl TaskContextConfig {
    /// Parse from a TOML config value.
    ///
    /// Falls back to defaults for missing or non-table values.
    pub fn from_toml(config: &toml::Value) -> Self {
        // Try deserializing the TOML value directly. If the shape doesn't
        // match, fall back to field-by-field extraction for resilience.
        if let Ok(parsed) = config.clone().try_into::<Self>() {
            return parsed;
        }

        let table = match config.as_table() {
            Some(t) => t,
            None => return Self::default(),
        };

        let str_field = |key: &str| -> String {
            table
                .get(key)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string()
        };

        let opt_str = |key: &str| -> Option<String> {
            table.get(key).and_then(|v| v.as_str()).map(String::from)
        };

        let files = table
            .get("files")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        Self {
            plan_id: str_field("plan_id"),
            plan_dir: str_field("plan_dir"),
            task_id: str_field("task_id"),
            title: str_field("title"),
            description: opt_str("description"),
            role: opt_str("role"),
            tier: str_field("tier"),
            model_hint: opt_str("model_hint"),
            domain: opt_str("domain"),
            timeout_secs: table
                .get("timeout_secs")
                .and_then(|v| v.as_integer())
                .unwrap_or(0) as u64,
            sequence: table
                .get("sequence")
                .and_then(|v| v.as_integer())
                .unwrap_or(0),
            files,
        }
    }
}

/// TaskContextCell: assembles task metadata and prior attempt state into a
/// structured signal for the downstream compose cell.
///
/// This is the first cell in each per-task subgraph. Its output feeds the
/// compose cell, which turns it into the task's prompt.
pub struct TaskContextCell {
    /// Parsed task configuration from the node's TOML config.
    config: TaskContextConfig,
}

impl TaskContextCell {
    /// Create a new `TaskContextCell` from a TOML config value.
    pub fn new(config: &toml::Value) -> Self {
        Self {
            config: TaskContextConfig::from_toml(config),
        }
    }
}

#[async_trait]
impl Cell for TaskContextCell {
    fn cell_id(&self) -> &'static str {
        "plan.task-context"
    }

    fn cell_name(&self) -> &'static str {
        "TaskContextCell"
    }

    fn cell_version(&self) -> CellVersion {
        (0, 2, 0)
    }

    fn protocols(&self) -> Vec<ProtocolId> {
        Vec::new()
    }

    fn estimated_cost(&self) -> Option<f64> {
        Some(0.0) // Pure metadata assembly, no LLM cost.
    }

    fn estimated_duration(&self) -> Option<Duration> {
        Some(Duration::from_millis(1))
    }

    async fn execute(&self, input: Vec<Signal>, ctx: &CellContext) -> Result<Vec<Signal>> {
        // Collect predecessor context from input signals (these come from
        // upstream success-boundary nodes of tasks we depend on).
        let predecessor_count = input.len();
        let predecessor_tasks: Vec<serde_json::Value> = input
            .iter()
            .map(|signal| {
                // Try to extract task_id from predecessor signals' tags.
                let task_id = signal.tag("task_id").unwrap_or("unknown");
                let status = signal.tag("status").unwrap_or("completed");
                serde_json::json!({
                    "task_id": task_id,
                    "status": status,
                })
            })
            .collect();

        // Assemble the task context payload.
        let context_payload = serde_json::json!({
            "plan_id": self.config.plan_id,
            "plan_dir": self.config.plan_dir,
            "task_id": self.config.task_id,
            "title": self.config.title,
            "description": self.config.description,
            "role": self.config.role,
            "tier": self.config.tier,
            "model_hint": self.config.model_hint,
            "domain": self.config.domain,
            "timeout_secs": self.config.timeout_secs,
            "sequence": self.config.sequence,
            "files": self.config.files,
            "predecessors": {
                "count": predecessor_count,
                "tasks": predecessor_tasks,
            },
            "run_id": ctx.run_id,
            "budget_remaining": ctx.budget_remaining,
        });

        tracing::info!(
            task_id = %self.config.task_id,
            plan_id = %self.config.plan_id,
            tier = %self.config.tier,
            predecessor_count,
            "TaskContextCell assembled context for task '{}'",
            self.config.task_id,
        );

        let signal = Signal::builder(Kind::Task)
            .body(Body::Json(context_payload))
            .tag("task_id", &self.config.task_id)
            .tag("plan_id", &self.config.plan_id)
            .tag("tier", &self.config.tier)
            .tag("cell", "task-context")
            .build();

        Ok(vec![signal])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_config() -> toml::Value {
        toml::from_str(
            r#"
plan_id = "test-plan"
plan_dir = "/tmp/plans/test"
task_id = "T1"
title = "Implement feature X"
description = "Build the feature with tests"
role = "implementer"
tier = "focused"
model_hint = "claude-sonnet-4-20250514"
domain = "coding"
timeout_secs = 600
sequence = 0
files = ["src/lib.rs", "src/main.rs"]
"#,
        )
        .unwrap()
    }

    #[test]
    fn config_parses_from_toml() {
        let toml_val = make_config();
        let config = TaskContextConfig::from_toml(&toml_val);
        assert_eq!(config.plan_id, "test-plan");
        assert_eq!(config.task_id, "T1");
        assert_eq!(config.title, "Implement feature X");
        assert_eq!(
            config.description.as_deref(),
            Some("Build the feature with tests")
        );
        assert_eq!(config.role.as_deref(), Some("implementer"));
        assert_eq!(config.tier, "focused");
        assert_eq!(
            config.model_hint.as_deref(),
            Some("claude-sonnet-4-20250514")
        );
        assert_eq!(config.domain.as_deref(), Some("coding"));
        assert_eq!(config.timeout_secs, 600);
        assert_eq!(config.sequence, 0);
        assert_eq!(config.files, vec!["src/lib.rs", "src/main.rs"]);
    }

    #[test]
    fn config_defaults_on_empty() {
        let toml_val = toml::Value::Table(toml::map::Map::new());
        let config = TaskContextConfig::from_toml(&toml_val);
        assert_eq!(config.plan_id, "");
        assert_eq!(config.task_id, "");
        assert!(config.description.is_none());
    }

    #[test]
    fn config_handles_non_table() {
        let toml_val = toml::Value::String("not a table".into());
        let config = TaskContextConfig::from_toml(&toml_val);
        assert_eq!(config.plan_id, "");
    }

    #[tokio::test]
    async fn execute_produces_task_signal() {
        let toml_val = make_config();
        let cell = TaskContextCell::new(&toml_val);
        let ctx = CellContext::new().with_run_id("run-1".into());

        let output = cell.execute(vec![], &ctx).await.unwrap();
        assert_eq!(output.len(), 1);

        let signal = &output[0];
        assert_eq!(signal.kind, Kind::Task);
        assert_eq!(signal.tag("task_id"), Some("T1"));
        assert_eq!(signal.tag("plan_id"), Some("test-plan"));
        assert_eq!(signal.tag("tier"), Some("focused"));
        assert_eq!(signal.tag("cell"), Some("task-context"));

        // Verify JSON body contents.
        let body_json = match &signal.body {
            Body::Json(v) => v,
            other => panic!("expected Body::Json, got {other:?}"),
        };
        assert_eq!(body_json["task_id"], "T1");
        assert_eq!(body_json["title"], "Implement feature X");
        assert_eq!(body_json["description"], "Build the feature with tests");
        assert_eq!(body_json["role"], "implementer");
        assert_eq!(body_json["predecessors"]["count"], 0);
        assert_eq!(body_json["run_id"], "run-1");
    }

    #[tokio::test]
    async fn execute_includes_predecessor_info() {
        let toml_val = make_config();
        let cell = TaskContextCell::new(&toml_val);
        let ctx = CellContext::new();

        // Simulate predecessor signals from upstream success boundaries.
        let pred_signal = Signal::builder(Kind::Task)
            .body(Body::text("predecessor completed"))
            .tag("task_id", "T0")
            .tag("status", "completed")
            .build();

        let output = cell.execute(vec![pred_signal], &ctx).await.unwrap();
        assert_eq!(output.len(), 1);

        let body_json = match &output[0].body {
            Body::Json(v) => v,
            other => panic!("expected Body::Json, got {other:?}"),
        };
        assert_eq!(body_json["predecessors"]["count"], 1);
        let tasks = body_json["predecessors"]["tasks"].as_array().unwrap();
        assert_eq!(tasks[0]["task_id"], "T0");
        assert_eq!(tasks[0]["status"], "completed");
    }

    #[test]
    fn cell_identity() {
        let toml_val = make_config();
        let cell = TaskContextCell::new(&toml_val);
        assert_eq!(cell.cell_id(), "plan.task-context");
        assert_eq!(cell.cell_name(), "TaskContextCell");
        assert_eq!(cell.cell_version(), (0, 2, 0));
        assert!(!cell.is_stub());
        assert_eq!(cell.estimated_cost(), Some(0.0));
    }

    #[test]
    fn config_partial_fields() {
        // Only some fields present -- the rest should default.
        let toml_val: toml::Value = toml::from_str(
            r#"
plan_id = "p1"
task_id = "T5"
tier = "mechanical"
"#,
        )
        .unwrap();
        let config = TaskContextConfig::from_toml(&toml_val);
        assert_eq!(config.plan_id, "p1");
        assert_eq!(config.task_id, "T5");
        assert_eq!(config.tier, "mechanical");
        assert_eq!(config.title, "");
        assert!(config.description.is_none());
        assert!(config.role.is_none());
        assert!(config.files.is_empty());
    }
}
