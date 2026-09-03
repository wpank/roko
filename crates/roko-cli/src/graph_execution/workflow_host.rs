//! CLI host adapter for workflow graph controller (#257).
//!
//! Bridges the [`WorkflowGraphController`] from `roko-execution` to the
//! CLI-layer services (workspace management, gate dispatch, event bus)
//! without introducing a reverse dependency from `roko-execution` to
//! `roko-cli`.
//!
//! The host adapter translates controller actions into CLI-layer effects
//! and feeds controller inputs back from the execution results.

use roko_execution::workflow::{
    ControllerAction, PhaseInput, PhaseReceipt, WorkflowGraphController,
    WorkflowPhase, WorkflowTermination,
};
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Host adapter types
// ---------------------------------------------------------------------------

/// Configuration for the CLI workflow host.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowHostConfig {
    /// User prompt for the workflow.
    pub prompt: String,
    /// Template name (canonical or alias).
    pub template_name: String,
    /// Whether git commits are enabled.
    pub commit_enabled: bool,
    /// Enabled gate names.
    pub enabled_gates: Vec<String>,
    /// Commit message prefix.
    pub commit_prefix: Option<String>,
}

/// Outcome of a CLI workflow host execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowHostOutcome {
    /// Run identifier.
    pub run_id: String,
    /// Terminal outcome.
    pub termination: WorkflowTermination,
    /// Number of generations executed.
    pub generations_executed: u32,
    /// Phase receipts collected during execution.
    pub receipts: Vec<PhaseReceipt>,
}

impl WorkflowHostOutcome {
    /// Returns `true` if the workflow completed successfully.
    #[must_use]
    pub fn is_success(&self) -> bool {
        self.termination.is_success()
    }
}

// ---------------------------------------------------------------------------
// Phase outcome for host -> controller feedback
// ---------------------------------------------------------------------------

/// Represents the outcome of executing a single phase, to be fed back
/// to the controller as a [`PhaseInput`].
#[derive(Debug, Clone)]
pub enum PhaseOutcome {
    /// Compose phase completed.
    ComposeCompleted,
    /// Implement phase completed with file change count.
    ImplementCompleted {
        /// Number of files changed.
        files_changed: u32,
    },
    /// Gate pipeline passed.
    GatePassed,
    /// Gate pipeline failed.
    GateFailed {
        /// Failure output.
        output: String,
    },
    /// Autofix phase completed.
    AutoFixCompleted,
    /// Review completed with structured verdict.
    ReviewCompleted {
        /// Parsed review verdict.
        verdict: roko_execution::workflow::ReviewVerdict,
    },
    /// Commit completed with hash.
    CommitCompleted {
        /// Git commit hash.
        hash: String,
    },
    /// Commit produced no changes.
    CommitNoChanges,
    /// Error occurred during phase execution.
    Error {
        /// Error description.
        reason: String,
    },
}

impl PhaseOutcome {
    /// Convert this outcome into a [`PhaseInput`] for the controller.
    #[must_use]
    pub fn into_phase_input(self) -> PhaseInput {
        match self {
            Self::ComposeCompleted => PhaseInput::ComposeCompleted,
            Self::ImplementCompleted { files_changed } => {
                PhaseInput::ImplementCompleted { files_changed }
            }
            Self::GatePassed => PhaseInput::GatePassed,
            Self::GateFailed { output } => PhaseInput::GateFailed { output },
            Self::AutoFixCompleted => PhaseInput::AutoFixCompleted,
            Self::ReviewCompleted { verdict } => PhaseInput::ReviewCompleted { verdict },
            Self::CommitCompleted { hash } => PhaseInput::CommitCompleted { hash },
            Self::CommitNoChanges => PhaseInput::CommitNoChanges,
            Self::Error { reason } => PhaseInput::Error { reason },
        }
    }
}

// ---------------------------------------------------------------------------
// Controller wrapper
// ---------------------------------------------------------------------------

/// CLI host wrapper around the `WorkflowGraphController`.
///
/// This type orchestrates the interaction between the pure controller
/// state machine and the CLI-layer effect services. The actual effect
/// execution (agent dispatch, gate running, git commit) is performed
/// by the caller through the [`ControllerAction`] enum.
///
/// # Usage
///
/// ```ignore
/// let mut host = WorkflowHost::new(config)?;
///
/// let mut action = host.start();
/// loop {
///     match action {
///         ControllerAction::Terminal(termination) => {
///             return Ok(host.into_outcome());
///         }
///         ControllerAction::RunCompose { .. } => {
///             // Execute compose phase...
///             let outcome = PhaseOutcome::ComposeCompleted;
///             action = host.advance(outcome);
///         }
///         // ... other phases
///     }
/// }
/// ```
pub struct WorkflowHost {
    /// Inner controller.
    controller: WorkflowGraphController,
    /// Host configuration.
    config: WorkflowHostConfig,
}

impl WorkflowHost {
    /// Create a new workflow host from the given configuration.
    ///
    /// # Errors
    ///
    /// Returns an error if the template name cannot be resolved.
    pub fn new(
        config: WorkflowHostConfig,
    ) -> Result<Self, roko_execution::workflow::TemplateResolutionError> {
        let mut template =
            roko_execution::workflow::resolve_template(&config.template_name)?;

        if !config.commit_enabled {
            template = template.with_commit_disabled();
        }

        let run_id = uuid::Uuid::new_v4().to_string();
        let workflow_id = format!("workflow-{run_id}");

        let controller = WorkflowGraphController::new(run_id, template, workflow_id);

        Ok(Self { controller, config })
    }

    /// Start the workflow, returning the first action.
    #[must_use]
    pub fn start(&mut self) -> ControllerAction {
        self.controller.step(PhaseInput::Start)
    }

    /// Advance the controller with a phase outcome and return the next action.
    pub fn advance(&mut self, outcome: PhaseOutcome) -> ControllerAction {
        // Record a receipt for the completed phase.
        let scope = self.controller.current_scope();
        let success = !matches!(outcome, PhaseOutcome::Error { .. } | PhaseOutcome::GateFailed { .. });
        let receipt = PhaseReceipt {
            scope,
            success,
            output: None,
            error: match &outcome {
                PhaseOutcome::Error { reason } => Some(reason.clone()),
                PhaseOutcome::GateFailed { output } => Some(output.clone()),
                _ => None,
            },
        };
        self.controller.record_receipt(receipt);

        let input = outcome.into_phase_input();
        self.controller.step(input)
    }

    /// Cancel the workflow.
    pub fn cancel(&mut self) -> ControllerAction {
        self.controller.step(PhaseInput::Cancel)
    }

    /// Returns the current controller state for inspection.
    #[must_use]
    pub fn controller(&self) -> &WorkflowGraphController {
        &self.controller
    }

    /// Returns the host configuration.
    #[must_use]
    pub fn config(&self) -> &WorkflowHostConfig {
        &self.config
    }

    /// Consume the host and return the outcome.
    ///
    /// Should only be called after the controller has terminated.
    #[must_use]
    pub fn into_outcome(self) -> WorkflowHostOutcome {
        WorkflowHostOutcome {
            run_id: self.controller.run_id.clone(),
            termination: self.controller.termination.clone().unwrap_or(
                WorkflowTermination::Failed {
                    reason: "controller not terminated".to_string(),
                },
            ),
            generations_executed: self.controller.current_generation + 1,
            receipts: self.controller.receipts.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_config() -> WorkflowHostConfig {
        WorkflowHostConfig {
            prompt: "Fix the bug".to_string(),
            template_name: "mechanical".to_string(),
            commit_enabled: true,
            enabled_gates: vec!["compile".to_string(), "test".to_string()],
            commit_prefix: None,
        }
    }

    #[test]
    fn host_resolves_template() {
        let config = make_config();
        let host = WorkflowHost::new(config).unwrap();
        assert_eq!(host.controller().template.name, "mechanical");
    }

    #[test]
    fn host_resolves_alias() {
        let mut config = make_config();
        config.template_name = "express".to_string();
        let host = WorkflowHost::new(config).unwrap();
        assert_eq!(host.controller().template.name, "mechanical");
    }

    #[test]
    fn host_unknown_template_errors() {
        let mut config = make_config();
        config.template_name = "nonexistent".to_string();
        let result = WorkflowHost::new(config);
        assert!(result.is_err());
    }

    #[test]
    fn host_no_commit_mode() {
        let mut config = make_config();
        config.commit_enabled = false;
        let host = WorkflowHost::new(config).unwrap();
        assert!(!host.controller().template.commit_enabled);
    }

    #[test]
    fn host_happy_path() {
        let config = make_config();
        let mut host = WorkflowHost::new(config).unwrap();

        let action = host.start();
        assert!(matches!(action, ControllerAction::RunCompose { .. }));

        let action = host.advance(PhaseOutcome::ComposeCompleted);
        assert!(matches!(action, ControllerAction::RunImplement { .. }));

        let action = host.advance(PhaseOutcome::ImplementCompleted { files_changed: 3 });
        assert!(matches!(action, ControllerAction::RunGate { .. }));

        let action = host.advance(PhaseOutcome::GatePassed);
        assert!(matches!(action, ControllerAction::RunCommit { .. }));

        let action = host.advance(PhaseOutcome::CommitCompleted {
            hash: "abc123".to_string(),
        });
        assert!(action.is_terminal());

        let outcome = host.into_outcome();
        assert!(outcome.is_success());
        assert!(!outcome.receipts.is_empty());
    }

    #[test]
    fn host_cancel() {
        let config = make_config();
        let mut host = WorkflowHost::new(config).unwrap();

        host.start();
        let action = host.cancel();
        assert!(action.is_terminal());

        let outcome = host.into_outcome();
        assert!(!outcome.is_success());
        assert!(matches!(
            outcome.termination,
            WorkflowTermination::Cancelled
        ));
    }

    #[test]
    fn host_gate_failure_records_receipt() {
        let config = make_config();
        let mut host = WorkflowHost::new(config).unwrap();

        host.start();
        host.advance(PhaseOutcome::ComposeCompleted);
        host.advance(PhaseOutcome::ImplementCompleted { files_changed: 1 });
        host.advance(PhaseOutcome::GateFailed {
            output: "error[E0308]".to_string(),
        });

        // The gate failure receipt should be recorded.
        let receipts = &host.controller().receipts;
        let gate_receipt = receipts.iter().find(|r| r.scope.phase == WorkflowPhase::Gate);
        assert!(gate_receipt.is_some());
        assert!(!gate_receipt.unwrap().success);
    }

    #[test]
    fn host_full_template_alias() {
        let mut config = make_config();
        config.template_name = "full".to_string();
        let host = WorkflowHost::new(config).unwrap();
        assert_eq!(host.controller().template.name, "architectural");
        assert!(host.controller().template.has_review);
        assert!(host.controller().template.has_strategy);
    }
}
