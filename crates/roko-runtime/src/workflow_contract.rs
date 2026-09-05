//! Preserved workflow contract types (#276).
//!
//! These serializable types were originally defined in `workflow_engine.rs` and
//! `pipeline_state.rs`. They are retained as the public report/config contracts
//! consumed by `roko-cli`, `roko-serve`, `roko-acp`, and `roko-execution`.
//!
//! The `WorkflowEngine`, `PipelineStateV2`, and `EffectDriver` orchestration
//! internals that consumed these types have been deleted; the graph-based
//! workflow controller in `roko-execution::workflow` is the replacement.

use std::path::PathBuf;

use roko_core::foundation::{ModelInputMessage, ShellGateCommand};
use roko_core::runtime_event::RuntimeEventEnvelope;
use serde::{Deserialize, Serialize};

/// Canonical workflow outcome, re-exported from `roko-core`.
pub use roko_core::runtime_event::WorkflowOutcome;

// ---------------------------------------------------------------------------
// Commit outcome (from pipeline_state.rs)
// ---------------------------------------------------------------------------

/// Typed result of a commit effect.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CommitOutcome {
    /// A git commit was created.
    Created {
        /// Created commit hash.
        hash: String,
    },
    /// The commit effect found no changes to commit.
    NoChanges,
    /// Commit creation was intentionally rejected before running git commit.
    Rejected {
        /// Human-readable rejection reason.
        reason: String,
    },
    /// Commit creation failed.
    Failed {
        /// Human-readable failure details.
        error: String,
    },
}

impl CommitOutcome {
    /// Convert a legacy successful commit input into a typed outcome.
    pub fn from_commit_done(hash: impl Into<String>) -> Self {
        Self::Created { hash: hash.into() }
    }

    /// Convert a legacy failed commit input into a typed outcome.
    pub fn from_commit_failed(error: impl Into<String>) -> Self {
        Self::Failed {
            error: error.into(),
        }
    }

    /// Return the created commit hash, if this outcome actually created a commit.
    pub fn created_hash(&self) -> Option<&str> {
        match self {
            Self::Created { hash } => Some(hash),
            Self::NoChanges | Self::Rejected { .. } | Self::Failed { .. } => None,
        }
    }
}

// ---------------------------------------------------------------------------
// Workflow config (from pipeline_state.rs)
// ---------------------------------------------------------------------------

/// Configuration for the pipeline. Determines which phases are active.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct WorkflowConfig {
    /// Include a strategist phase before implementation.
    pub has_strategy: bool,
    /// Include a review phase after gates pass.
    pub has_review: bool,
    /// Maximum implement -> gate -> review iterations.
    pub max_iterations: u32,
    /// Maximum autofix attempts per gate failure.
    pub max_autofix_attempts: u32,
}

impl Default for WorkflowConfig {
    fn default() -> Self {
        Self::standard()
    }
}

impl WorkflowConfig {
    /// Express: implement -> gate -> commit.
    pub fn express() -> Self {
        Self {
            has_strategy: false,
            has_review: false,
            max_iterations: 1,
            max_autofix_attempts: 1,
        }
    }

    /// Standard: implement -> gate -> review -> commit.
    pub fn standard() -> Self {
        Self {
            has_strategy: false,
            has_review: true,
            max_iterations: 2,
            max_autofix_attempts: 2,
        }
    }

    /// Full: strategy -> implement -> gate -> review -> commit.
    pub fn full() -> Self {
        Self {
            has_strategy: true,
            has_review: true,
            max_iterations: 3,
            max_autofix_attempts: 2,
        }
    }

    /// Parse a `WorkflowConfig` from a TOML string.
    ///
    /// The string may contain a `[workflow]` table or just the bare keys. If a
    /// `template` key is present (`"express"`, `"standard"`, or `"full"`), that
    /// preset is used as the base; any additional keys override the preset values.
    ///
    /// Returns an error if the TOML is malformed or `template` is an unknown value.
    pub fn from_toml_str(s: &str) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let raw = parse_workflow_config_toml(s)?;

        let mut config = match raw.template.as_deref() {
            Some("express") => Self::express(),
            Some("standard") | None => Self::standard(),
            Some("full") => Self::full(),
            Some(template) => {
                return Err(config_parse_error(format!(
                    "unknown workflow template: {template}"
                )));
            }
        };

        if let Some(has_strategy) = raw.has_strategy {
            config.has_strategy = has_strategy;
        }
        if let Some(has_review) = raw.has_review {
            config.has_review = has_review;
        }
        if let Some(max_iterations) = raw.max_iterations {
            config.max_iterations = max_iterations;
        }
        if let Some(max_autofix_attempts) = raw.max_autofix_attempts {
            config.max_autofix_attempts = max_autofix_attempts;
        }

        Ok(config)
    }

    /// Load a `WorkflowConfig` from a TOML file on disk.
    pub fn from_toml(
        path: &std::path::Path,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let contents = std::fs::read_to_string(path)?;
        Self::from_toml_str(&contents)
    }
}

// ---------------------------------------------------------------------------
// TOML parsing helpers (from pipeline_state.rs)
// ---------------------------------------------------------------------------

#[derive(Debug, serde::Deserialize)]
struct WorkflowConfigToml {
    template: Option<String>,
    has_strategy: Option<bool>,
    has_review: Option<bool>,
    max_iterations: Option<u32>,
    max_autofix_attempts: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WorkflowTomlScope {
    Root,
    Workflow,
    WorkflowStep,
    Other,
}

fn parse_workflow_config_toml(
    s: &str,
) -> Result<WorkflowConfigToml, Box<dyn std::error::Error + Send + Sync>> {
    let mut workflow = WorkflowConfigToml {
        template: None,
        has_strategy: None,
        has_review: None,
        max_iterations: None,
        max_autofix_attempts: None,
    };
    let mut scope = WorkflowTomlScope::Root;
    let mut saw_workflow_table = false;
    let mut saw_workflow_steps = false;
    let mut steps_have_strategy = false;
    let mut steps_have_review = false;

    for (idx, raw_line) in s.lines().enumerate() {
        let line_number = idx + 1;
        let line = strip_toml_comment(raw_line)
            .map_err(|err| config_parse_error(format!("line {line_number}: {err}")))?;
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        if line.starts_with("[[") || line.starts_with('[') {
            scope = parse_workflow_toml_scope(line)
                .map_err(|err| config_parse_error(format!("line {line_number}: {err}")))?;
            if scope == WorkflowTomlScope::Workflow {
                saw_workflow_table = true;
            } else if scope == WorkflowTomlScope::WorkflowStep {
                saw_workflow_steps = true;
            }
            continue;
        }

        let target_scope = if saw_workflow_table {
            WorkflowTomlScope::Workflow
        } else {
            WorkflowTomlScope::Root
        };

        if scope == target_scope || scope == WorkflowTomlScope::Root {
            if let Some((key, value)) = parse_workflow_kv(line) {
                match key {
                    "template" => workflow.template = Some(unquote(&value)),
                    "has_strategy" => workflow.has_strategy = parse_bool_value(&value),
                    "has_review" => workflow.has_review = parse_bool_value(&value),
                    "max_iterations" => workflow.max_iterations = value.parse().ok(),
                    "max_autofix_attempts" => workflow.max_autofix_attempts = value.parse().ok(),
                    _ => {}
                }
            }
        } else if scope == WorkflowTomlScope::WorkflowStep
            && let Some((key, value)) = parse_workflow_kv(line)
        {
            let name = unquote(&value);
            if key == "name" || key == "type" {
                if name == "strategy" || name == "strategist" {
                    steps_have_strategy = true;
                } else if name == "review" || name == "reviewer" {
                    steps_have_review = true;
                }
            }
        }
    }

    if saw_workflow_steps {
        if workflow.has_strategy.is_none() {
            workflow.has_strategy = Some(steps_have_strategy);
        }
        if workflow.has_review.is_none() {
            workflow.has_review = Some(steps_have_review);
        }
    }

    Ok(workflow)
}

fn parse_workflow_toml_scope(
    line: &str,
) -> Result<WorkflowTomlScope, Box<dyn std::error::Error + Send + Sync>> {
    let header = line.trim_start_matches('[').trim_end_matches(']').trim();
    Ok(match header {
        "workflow" => WorkflowTomlScope::Workflow,
        "workflow.steps" | "workflow.step" => WorkflowTomlScope::WorkflowStep,
        _ => WorkflowTomlScope::Other,
    })
}

fn strip_toml_comment(line: &str) -> Result<&str, Box<dyn std::error::Error + Send + Sync>> {
    let mut in_string = false;
    let mut prev_char = '\0';
    for (i, ch) in line.char_indices() {
        match ch {
            '"' if prev_char != '\\' => in_string = !in_string,
            '#' if !in_string => return Ok(&line[..i]),
            _ => {}
        }
        prev_char = ch;
    }
    Ok(line)
}

fn parse_workflow_kv(line: &str) -> Option<(&str, String)> {
    let (key, rest) = line.split_once('=')?;
    Some((key.trim(), rest.trim().to_string()))
}

fn unquote(s: &str) -> String {
    s.trim_matches('"').trim_matches('\'').to_string()
}

fn parse_bool_value(s: &str) -> Option<bool> {
    match s.trim() {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    }
}

fn config_parse_error(msg: String) -> Box<dyn std::error::Error + Send + Sync> {
    Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, msg))
}

// ---------------------------------------------------------------------------
// Pipeline phase (from pipeline_state.rs, used by RunLedger)
// ---------------------------------------------------------------------------

/// Pipeline execution phase.
///
/// Retained from `pipeline_state.rs` because [`RunLedger`](crate::run_ledger::RunLedger)
/// records phase transitions using this type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Phase {
    /// Pipeline has been created but not started.
    Pending,
    /// Strategy phase is running.
    Strategizing,
    /// Implementation phase is running.
    Implementing,
    /// Verification gates are running.
    Gating,
    /// Autofix phase is running after a gate failure.
    AutoFixing,
    /// Review phase is running after gates pass.
    Reviewing,
    /// Commit creation is running.
    Committing,
    /// Workflow completed successfully.
    Complete,
    /// Workflow halted before completion.
    Halted {
        /// Human-readable halt reason.
        reason: String,
    },
    /// Workflow was cancelled by the user.
    Cancelled,
}

impl Phase {
    /// Returns true when no further state transitions should be accepted.
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Complete | Self::Halted { .. } | Self::Cancelled)
    }

    /// Returns a static label for the phase.
    pub fn label(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Strategizing => "strategizing",
            Self::Implementing => "implementing",
            Self::Gating => "gating",
            Self::AutoFixing => "auto_fixing",
            Self::Reviewing => "reviewing",
            Self::Committing => "committing",
            Self::Complete => "complete",
            Self::Halted { .. } => "halted",
            Self::Cancelled => "cancelled",
        }
    }
}

// ---------------------------------------------------------------------------
// Workflow run config (from workflow_engine.rs)
// ---------------------------------------------------------------------------

/// Configuration for a workflow run.
#[derive(Debug, Clone)]
pub struct WorkflowRunConfig {
    /// User prompt.
    pub prompt: String,
    /// Ordered structured input used for multimodal provider dispatch.
    pub input_messages: Vec<ModelInputMessage>,
    /// Working directory.
    pub workdir: PathBuf,
    /// Workflow configuration (express/standard/full).
    pub workflow: WorkflowConfig,
    /// Which gates to run.
    pub enabled_gates: Vec<String>,
    /// Shell command configs for shell/custom:shell gate entries.
    pub shell_gates: Vec<ShellGateCommand>,
    /// Commit message prefix.
    pub commit_prefix: Option<String>,
}

/// Result of a workflow run.
#[derive(Debug, Clone)]
pub struct WorkflowResult {
    /// Workflow run id.
    pub run_id: String,
    /// Final workflow outcome.
    pub outcome: WorkflowOutcome,
    /// Number of implementation iterations used.
    pub iterations: u32,
}

/// Per-gate result included in a workflow run report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GateOutcome {
    /// Gate name.
    pub name: String,
    /// Whether the gate passed.
    pub passed: bool,
    /// Optional gate output or failure details.
    pub output: Option<String>,
    /// Gate runtime in milliseconds.
    pub duration_ms: u64,
}

/// Summary returned after a workflow run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowRunReport {
    /// Workflow run id.
    pub run_id: String,
    /// Whether the workflow completed successfully.
    pub success: bool,
    /// Primary model used by the workflow.
    pub model: String,
    /// Provider used for the primary model, when known.
    pub provider: Option<String>,
    /// Short summary of the prompt.
    pub prompt_summary: String,
    /// Final workflow output.
    pub output: String,
    /// Number of agent turns used.
    pub agent_turns: u32,
    /// Total tokens used.
    pub token_usage: u64,
    /// Total cost, when known.
    pub cost: Option<f64>,
    /// Total runtime in seconds.
    pub duration_secs: f64,
    /// Gate outcomes collected during the run.
    pub gates: Vec<GateOutcome>,
    /// Runtime events emitted during the run.
    pub events: Vec<RuntimeEventEnvelope>,
    /// Last checkpoint path, when one was written.
    pub checkpoint_path: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workflow_config_express() {
        let c = WorkflowConfig::express();
        assert!(!c.has_strategy);
        assert!(!c.has_review);
        assert_eq!(c.max_iterations, 1);
    }

    #[test]
    fn workflow_config_standard() {
        let c = WorkflowConfig::standard();
        assert!(!c.has_strategy);
        assert!(c.has_review);
        assert_eq!(c.max_iterations, 2);
    }

    #[test]
    fn workflow_config_full() {
        let c = WorkflowConfig::full();
        assert!(c.has_strategy);
        assert!(c.has_review);
        assert_eq!(c.max_iterations, 3);
    }

    #[test]
    fn workflow_config_from_toml_template() {
        let c = WorkflowConfig::from_toml_str("template = \"express\"").unwrap();
        assert!(!c.has_strategy);
        assert!(!c.has_review);
        assert_eq!(c.max_iterations, 1);
    }

    #[test]
    fn workflow_config_from_toml_overrides() {
        let c = WorkflowConfig::from_toml_str("template = \"express\"\nhas_review = true").unwrap();
        assert!(c.has_review);
    }

    #[test]
    fn commit_outcome_roundtrip() {
        let done = CommitOutcome::from_commit_done("abc123");
        assert_eq!(done.created_hash(), Some("abc123"));
        let failed = CommitOutcome::from_commit_failed("nope");
        assert_eq!(failed.created_hash(), None);
    }

    #[test]
    fn gate_outcome_serde() {
        let gate = GateOutcome {
            name: "compile".to_string(),
            passed: true,
            output: None,
            duration_ms: 42,
        };
        let json = serde_json::to_string(&gate).unwrap();
        let back: GateOutcome = serde_json::from_str(&json).unwrap();
        assert_eq!(back.name, "compile");
        assert!(back.passed);
    }

    #[test]
    fn workflow_run_report_serde() {
        let report = WorkflowRunReport {
            run_id: "run-1".to_string(),
            success: true,
            model: "claude".to_string(),
            provider: Some("anthropic".to_string()),
            prompt_summary: "fix bug".to_string(),
            output: "done".to_string(),
            agent_turns: 2,
            token_usage: 1000,
            cost: Some(0.05),
            duration_secs: 12.5,
            gates: vec![],
            events: vec![],
            checkpoint_path: None,
        };
        let json = serde_json::to_string(&report).unwrap();
        let back: WorkflowRunReport = serde_json::from_str(&json).unwrap();
        assert_eq!(back.run_id, "run-1");
        assert!(back.success);
    }
}
