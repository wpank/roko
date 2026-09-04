//! Workflow outer lifecycle controller (#257).
//!
//! The [`WorkflowGraphController`] manages the full lifecycle of a
//! single-prompt workflow execution expressed as a sequence of acyclic
//! subgraph generations. It never adds a back-edge to an existing graph.
//!
//! # Controller Sequence
//!
//! 1. Shared preflight checks.
//! 2. Acquire workspace lease (#249).
//! 3. Execute generation(s) in a loop.
//! 4. Completion settlement (#253).
//! 5. Delivery / commit (#254).
//! 6. Final checkpoint.
//! 7. Release / retain workspace lease (#249).
//! 8. Return one terminal receipt.
//!
//! Gate failure creates a new autofix generation rather than adding a
//! back-edge. Review "revise" creates a new generation starting at Compose.
//! The controller enforces the configured auto-fix and review caps.

use std::fmt;

use serde::{Deserialize, Serialize};

use super::review_parser::ReviewVerdict;
use super::templates::WorkflowTemplateDescriptor;

// ---------------------------------------------------------------------------
// Idempotency key
// ---------------------------------------------------------------------------

/// Compute the canonical idempotency key for a workflow activity.
///
/// Format: `<run>/<template>@<version>/<workflow>/<generation>/<phase>/<attempt>`
///
/// This ensures repeated node IDs cannot alias prior iterations or attempts.
#[must_use]
pub fn idempotency_key(scope: &ActivityScope) -> String {
    format!(
        "{}/{}/{}@{}/{}/{}/{}/{}",
        scope.run_id,
        scope.template_name,
        scope.template_name,
        scope.template_version,
        scope.workflow_id,
        scope.generation,
        scope.phase.as_str(),
        scope.attempt,
    )
}

/// Scope identifying a single workflow Activity for idempotency.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActivityScope {
    /// Run identifier.
    pub run_id: String,
    /// Canonical template name.
    pub template_name: String,
    /// Template schema version.
    pub template_version: u32,
    /// Workflow instance identifier.
    pub workflow_id: String,
    /// Zero-based generation index.
    pub generation: u32,
    /// Phase within the generation.
    pub phase: WorkflowPhase,
    /// Zero-based attempt index within the phase.
    pub attempt: u32,
}

// ---------------------------------------------------------------------------
// Workflow phases
// ---------------------------------------------------------------------------

/// Phases within a workflow generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowPhase {
    /// Prompt composition (deterministic).
    Compose,
    /// Agent implementation dispatch (Activity).
    Implement,
    /// Gate verification pipeline (Activity).
    Gate,
    /// Autofix dispatch for gate failures (Activity).
    AutoFix,
    /// Review agent dispatch (Activity).
    Review,
    /// Git commit delivery (Activity).
    Commit,
}

impl WorkflowPhase {
    /// Returns the phase name as a static string.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Compose => "compose",
            Self::Implement => "implement",
            Self::Gate => "gate",
            Self::AutoFix => "autofix",
            Self::Review => "review",
            Self::Commit => "commit",
        }
    }
}

impl fmt::Display for WorkflowPhase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

// ---------------------------------------------------------------------------
// Controller state
// ---------------------------------------------------------------------------

/// Terminal outcome of a workflow run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum WorkflowTermination {
    /// All phases completed successfully.
    Success {
        /// Commit hash produced, if any.
        commit_hash: Option<String>,
    },
    /// Gate failures exhausted all retry/autofix caps.
    GateExhausted {
        /// Last gate failure output.
        last_failure: String,
    },
    /// Review cap exhausted without approval.
    ReviewExhausted {
        /// Accumulated review findings.
        findings: Vec<String>,
    },
    /// User or system cancellation.
    Cancelled,
    /// Preflight or system error.
    Failed {
        /// Error description.
        reason: String,
    },
    /// No changes were produced by the implementation.
    NoChanges,
    /// Workflow was skipped (e.g., by a skip flag).
    Skipped {
        /// Reason for skipping.
        reason: String,
    },
}

impl WorkflowTermination {
    /// Returns `true` if this is a successful completion.
    #[must_use]
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Success { .. })
    }
}

/// Per-phase durable receipt, recorded after each phase completes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PhaseReceipt {
    /// The activity scope (includes idempotency key components).
    pub scope: ActivityScope,
    /// Whether the phase completed successfully.
    pub success: bool,
    /// Optional output data.
    pub output: Option<String>,
    /// Optional error message.
    pub error: Option<String>,
}

/// Tracks the state of a workflow across generations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowGraphController {
    /// Run identifier.
    pub run_id: String,
    /// Template descriptor for this workflow.
    pub template: WorkflowTemplateDescriptor,
    /// Workflow instance identifier.
    pub workflow_id: String,
    /// Current generation index (0-based).
    pub current_generation: u32,
    /// Current phase within the generation.
    pub current_phase: WorkflowPhase,
    /// Current attempt within the current phase.
    pub current_attempt: u32,
    /// Total iterations used (implement -> gate cycles).
    pub iterations_used: u32,
    /// Autofix attempts used in the current iteration.
    pub autofix_attempts_used: u32,
    /// Accumulated review findings across iterations.
    pub review_findings: Vec<String>,
    /// Last gate failure output.
    pub last_gate_failure: Option<String>,
    /// Durable phase receipts.
    pub receipts: Vec<PhaseReceipt>,
    /// Terminal outcome, set when the workflow completes.
    pub termination: Option<WorkflowTermination>,
}

impl WorkflowGraphController {
    /// Create a new controller for the given template.
    #[must_use]
    pub fn new(run_id: String, template: WorkflowTemplateDescriptor, workflow_id: String) -> Self {
        Self {
            run_id,
            template,
            workflow_id,
            current_generation: 0,
            current_phase: WorkflowPhase::Compose,
            current_attempt: 0,
            iterations_used: 0,
            autofix_attempts_used: 0,
            review_findings: Vec::new(),
            last_gate_failure: None,
            receipts: Vec::new(),
            termination: None,
        }
    }

    /// Returns `true` if the workflow has terminated.
    #[must_use]
    pub fn is_terminated(&self) -> bool {
        self.termination.is_some()
    }

    /// Build the current activity scope for idempotency key generation.
    #[must_use]
    pub fn current_scope(&self) -> ActivityScope {
        ActivityScope {
            run_id: self.run_id.clone(),
            template_name: self.template.name.clone(),
            template_version: self.template.version,
            workflow_id: self.workflow_id.clone(),
            generation: self.current_generation,
            phase: self.current_phase,
            attempt: self.current_attempt,
        }
    }

    /// Compute the idempotency key for the current activity.
    #[must_use]
    pub fn current_idempotency_key(&self) -> String {
        idempotency_key(&self.current_scope())
    }

    /// Record a phase receipt and advance to the next phase.
    pub fn record_receipt(&mut self, receipt: PhaseReceipt) {
        self.receipts.push(receipt);
    }

    /// Determine the next controller action based on the current phase outcome.
    ///
    /// Returns the next [`ControllerAction`] the host should execute.
    #[must_use]
    #[allow(clippy::too_many_lines)]
    pub fn step(&mut self, input: PhaseInput) -> ControllerAction {
        if self.is_terminated() {
            return ControllerAction::Terminal(
                self.termination
                    .clone()
                    .expect("terminated but no termination set"),
            );
        }

        match input {
            PhaseInput::Start => {
                self.current_phase = WorkflowPhase::Compose;
                self.current_generation = 0;
                self.iterations_used = 0;
                ControllerAction::RunCompose {
                    generation: self.current_generation,
                }
            }

            PhaseInput::ComposeCompleted => {
                self.current_phase = WorkflowPhase::Implement;
                self.iterations_used += 1;
                ControllerAction::RunImplement {
                    generation: self.current_generation,
                }
            }

            PhaseInput::ImplementCompleted { files_changed } => {
                if files_changed == 0 {
                    self.termination = Some(WorkflowTermination::NoChanges);
                    return ControllerAction::Terminal(WorkflowTermination::NoChanges);
                }
                self.current_phase = WorkflowPhase::Gate;
                ControllerAction::RunGate {
                    generation: self.current_generation,
                }
            }

            PhaseInput::GatePassed => {
                self.autofix_attempts_used = 0;

                if self.template.has_review {
                    self.current_phase = WorkflowPhase::Review;
                    ControllerAction::RunReview {
                        generation: self.current_generation,
                    }
                } else if self.template.commit_enabled {
                    self.current_phase = WorkflowPhase::Commit;
                    ControllerAction::RunCommit {
                        generation: self.current_generation,
                    }
                } else {
                    let term = WorkflowTermination::Success { commit_hash: None };
                    self.termination = Some(term.clone());
                    ControllerAction::Terminal(term)
                }
            }

            PhaseInput::GateFailed { output } => {
                self.last_gate_failure = Some(output.clone());

                if self.autofix_attempts_used < self.template.max_autofix_attempts {
                    self.autofix_attempts_used += 1;
                    self.current_generation += 1;
                    self.current_phase = WorkflowPhase::AutoFix;
                    ControllerAction::RunAutoFix {
                        generation: self.current_generation,
                        failure_output: output,
                    }
                } else {
                    let term = WorkflowTermination::GateExhausted {
                        last_failure: output,
                    };
                    self.termination = Some(term.clone());
                    ControllerAction::Terminal(term)
                }
            }

            PhaseInput::AutoFixCompleted => {
                self.current_phase = WorkflowPhase::Gate;
                ControllerAction::RunGate {
                    generation: self.current_generation,
                }
            }

            PhaseInput::ReviewCompleted { verdict } => match verdict {
                ReviewVerdict::Approved => {
                    if self.template.commit_enabled {
                        self.current_phase = WorkflowPhase::Commit;
                        ControllerAction::RunCommit {
                            generation: self.current_generation,
                        }
                    } else {
                        let term = WorkflowTermination::Success { commit_hash: None };
                        self.termination = Some(term.clone());
                        ControllerAction::Terminal(term)
                    }
                }
                ReviewVerdict::Revise { findings } => {
                    self.review_findings.extend(findings);

                    if self.iterations_used < self.template.max_iterations {
                        self.current_generation += 1;
                        self.current_phase = WorkflowPhase::Compose;
                        self.autofix_attempts_used = 0;
                        ControllerAction::RunCompose {
                            generation: self.current_generation,
                        }
                    } else {
                        let term = WorkflowTermination::ReviewExhausted {
                            findings: self.review_findings.clone(),
                        };
                        self.termination = Some(term.clone());
                        ControllerAction::Terminal(term)
                    }
                }
                ReviewVerdict::Rejected { reason } => {
                    let term = WorkflowTermination::Failed { reason };
                    self.termination = Some(term.clone());
                    ControllerAction::Terminal(term)
                }
                ReviewVerdict::Unclear { summary } => {
                    // Unclear is treated as a revision request with the summary.
                    self.review_findings.push(summary);

                    if self.iterations_used < self.template.max_iterations {
                        self.current_generation += 1;
                        self.current_phase = WorkflowPhase::Compose;
                        self.autofix_attempts_used = 0;
                        ControllerAction::RunCompose {
                            generation: self.current_generation,
                        }
                    } else {
                        let term = WorkflowTermination::ReviewExhausted {
                            findings: self.review_findings.clone(),
                        };
                        self.termination = Some(term.clone());
                        ControllerAction::Terminal(term)
                    }
                }
            },

            PhaseInput::CommitCompleted { hash } => {
                let term = WorkflowTermination::Success {
                    commit_hash: Some(hash),
                };
                self.termination = Some(term.clone());
                ControllerAction::Terminal(term)
            }

            PhaseInput::CommitNoChanges => {
                let term = WorkflowTermination::Success { commit_hash: None };
                self.termination = Some(term.clone());
                ControllerAction::Terminal(term)
            }

            PhaseInput::Cancel => {
                self.termination = Some(WorkflowTermination::Cancelled);
                ControllerAction::Terminal(WorkflowTermination::Cancelled)
            }

            PhaseInput::Error { reason } => {
                let term = WorkflowTermination::Failed {
                    reason: reason.clone(),
                };
                self.termination = Some(term.clone());
                ControllerAction::Terminal(term)
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Controller I/O types
// ---------------------------------------------------------------------------

/// Input to a controller step, representing the outcome of the previous phase.
#[derive(Debug, Clone)]
pub enum PhaseInput {
    /// Start the workflow.
    Start,
    /// Compose phase completed.
    ComposeCompleted,
    /// Implement phase completed.
    ImplementCompleted {
        /// Number of files changed by the implementation.
        files_changed: u32,
    },
    /// Gate pipeline passed.
    GatePassed,
    /// Gate pipeline failed.
    GateFailed {
        /// Gate failure output.
        output: String,
    },
    /// Autofix phase completed.
    AutoFixCompleted,
    /// Review phase completed.
    ReviewCompleted {
        /// Structured review verdict.
        verdict: ReviewVerdict,
    },
    /// Commit completed with a hash.
    CommitCompleted {
        /// Git commit hash.
        hash: String,
    },
    /// Commit produced no changes.
    CommitNoChanges,
    /// User or system cancellation.
    Cancel,
    /// System error.
    Error {
        /// Error description.
        reason: String,
    },
}

/// Action the host should execute next.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ControllerAction {
    /// Run the compose phase for the given generation.
    RunCompose {
        /// Generation index.
        generation: u32,
    },
    /// Run the implement phase for the given generation.
    RunImplement {
        /// Generation index.
        generation: u32,
    },
    /// Run the gate phase for the given generation.
    RunGate {
        /// Generation index.
        generation: u32,
    },
    /// Run the autofix phase for the given generation.
    RunAutoFix {
        /// Generation index.
        generation: u32,
        /// Gate failure output to fix.
        failure_output: String,
    },
    /// Run the review phase for the given generation.
    RunReview {
        /// Generation index.
        generation: u32,
    },
    /// Run the commit phase for the given generation.
    RunCommit {
        /// Generation index.
        generation: u32,
    },
    /// Workflow has terminated.
    Terminal(WorkflowTermination),
}

impl ControllerAction {
    /// Returns `true` if this is a terminal action.
    #[must_use]
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Terminal(_))
    }
}

#[cfg(test)]
#[allow(unused_must_use)] // Many ctrl.step() calls are for setup, not assertion.
mod tests {
    use super::*;
    use crate::workflow::templates::WorkflowTemplateDescriptor;

    fn make_controller(template: WorkflowTemplateDescriptor) -> WorkflowGraphController {
        WorkflowGraphController::new("run-001".to_string(), template, "wf-001".to_string())
    }

    // ── Happy path: mechanical (no review) ─────────────────────────────

    #[test]
    fn mechanical_happy_path() {
        let mut ctrl = make_controller(WorkflowTemplateDescriptor::mechanical());

        let action = ctrl.step(PhaseInput::Start);
        assert_eq!(action, ControllerAction::RunCompose { generation: 0 });

        let action = ctrl.step(PhaseInput::ComposeCompleted);
        assert_eq!(action, ControllerAction::RunImplement { generation: 0 });

        let action = ctrl.step(PhaseInput::ImplementCompleted { files_changed: 3 });
        assert_eq!(action, ControllerAction::RunGate { generation: 0 });

        let action = ctrl.step(PhaseInput::GatePassed);
        assert_eq!(action, ControllerAction::RunCommit { generation: 0 });

        let action = ctrl.step(PhaseInput::CommitCompleted {
            hash: "abc123".to_string(),
        });
        assert!(action.is_terminal());
        if let ControllerAction::Terminal(WorkflowTermination::Success { commit_hash }) = action {
            assert_eq!(commit_hash, Some("abc123".to_string()));
        } else {
            panic!("expected success terminal");
        }
    }

    // ── Happy path: focused (with review) ──────────────────────────────

    #[test]
    fn focused_happy_path_with_review() {
        let mut ctrl = make_controller(WorkflowTemplateDescriptor::focused());

        ctrl.step(PhaseInput::Start);
        ctrl.step(PhaseInput::ComposeCompleted);
        ctrl.step(PhaseInput::ImplementCompleted { files_changed: 2 });

        let action = ctrl.step(PhaseInput::GatePassed);
        assert_eq!(action, ControllerAction::RunReview { generation: 0 });

        let action = ctrl.step(PhaseInput::ReviewCompleted {
            verdict: ReviewVerdict::Approved,
        });
        assert_eq!(action, ControllerAction::RunCommit { generation: 0 });
    }

    // ── No-commit mode ─────────────────────────────────────────────────

    #[test]
    fn no_commit_mode() {
        let template = WorkflowTemplateDescriptor::mechanical().with_commit_disabled();
        let mut ctrl = make_controller(template);

        ctrl.step(PhaseInput::Start);
        ctrl.step(PhaseInput::ComposeCompleted);
        ctrl.step(PhaseInput::ImplementCompleted { files_changed: 1 });

        let action = ctrl.step(PhaseInput::GatePassed);
        assert!(action.is_terminal());
        if let ControllerAction::Terminal(WorkflowTermination::Success { commit_hash }) = action {
            assert!(commit_hash.is_none());
        }
    }

    // ── No changes ─────────────────────────────────────────────────────

    #[test]
    fn no_changes_terminates() {
        let mut ctrl = make_controller(WorkflowTemplateDescriptor::mechanical());

        ctrl.step(PhaseInput::Start);
        ctrl.step(PhaseInput::ComposeCompleted);

        let action = ctrl.step(PhaseInput::ImplementCompleted { files_changed: 0 });
        assert!(action.is_terminal());
        assert!(matches!(
            action,
            ControllerAction::Terminal(WorkflowTermination::NoChanges)
        ));
    }

    // ── Gate failure -> autofix ─────────────────────────────────────────

    #[test]
    fn gate_failure_triggers_autofix() {
        let mut ctrl = make_controller(WorkflowTemplateDescriptor::mechanical());

        ctrl.step(PhaseInput::Start);
        ctrl.step(PhaseInput::ComposeCompleted);
        ctrl.step(PhaseInput::ImplementCompleted { files_changed: 1 });

        let action = ctrl.step(PhaseInput::GateFailed {
            output: "error[E0308]".to_string(),
        });
        assert!(matches!(
            action,
            ControllerAction::RunAutoFix { generation: 1, .. }
        ));

        let action = ctrl.step(PhaseInput::AutoFixCompleted);
        assert_eq!(action, ControllerAction::RunGate { generation: 1 });
    }

    // ── Gate exhaustion ────────────────────────────────────────────────

    #[test]
    fn gate_exhaustion() {
        let mut ctrl = make_controller(WorkflowTemplateDescriptor::mechanical());

        ctrl.step(PhaseInput::Start);
        ctrl.step(PhaseInput::ComposeCompleted);
        ctrl.step(PhaseInput::ImplementCompleted { files_changed: 1 });

        // First gate failure -> autofix.
        ctrl.step(PhaseInput::GateFailed {
            output: "error 1".to_string(),
        });
        ctrl.step(PhaseInput::AutoFixCompleted);

        // Second gate failure -> exhausted (mechanical has max_autofix_attempts=1).
        let action = ctrl.step(PhaseInput::GateFailed {
            output: "error 2".to_string(),
        });
        assert!(action.is_terminal());
        assert!(matches!(
            action,
            ControllerAction::Terminal(WorkflowTermination::GateExhausted { .. })
        ));
    }

    // ── Review revise -> new iteration ─────────────────────────────────

    #[test]
    fn review_revise_creates_new_generation() {
        let mut ctrl = make_controller(WorkflowTemplateDescriptor::focused());

        ctrl.step(PhaseInput::Start);
        ctrl.step(PhaseInput::ComposeCompleted);
        ctrl.step(PhaseInput::ImplementCompleted { files_changed: 2 });
        ctrl.step(PhaseInput::GatePassed);

        let action = ctrl.step(PhaseInput::ReviewCompleted {
            verdict: ReviewVerdict::Revise {
                findings: vec!["add tests".to_string()],
            },
        });
        // Focused has max_iterations=2, iterations_used=1, so we get a new gen.
        assert_eq!(action, ControllerAction::RunCompose { generation: 1 });
        assert_eq!(ctrl.review_findings, vec!["add tests"]);
    }

    // ── Review exhaustion ──────────────────────────────────────────────

    #[test]
    fn review_exhaustion() {
        let mut ctrl = make_controller(WorkflowTemplateDescriptor::focused());

        // Iteration 1.
        ctrl.step(PhaseInput::Start);
        ctrl.step(PhaseInput::ComposeCompleted);
        ctrl.step(PhaseInput::ImplementCompleted { files_changed: 1 });
        ctrl.step(PhaseInput::GatePassed);
        ctrl.step(PhaseInput::ReviewCompleted {
            verdict: ReviewVerdict::Revise {
                findings: vec!["first".to_string()],
            },
        });

        // Iteration 2.
        ctrl.step(PhaseInput::ComposeCompleted);
        ctrl.step(PhaseInput::ImplementCompleted { files_changed: 1 });
        ctrl.step(PhaseInput::GatePassed);

        let action = ctrl.step(PhaseInput::ReviewCompleted {
            verdict: ReviewVerdict::Revise {
                findings: vec!["second".to_string()],
            },
        });

        assert!(action.is_terminal());
        if let ControllerAction::Terminal(WorkflowTermination::ReviewExhausted { findings }) =
            action
        {
            assert_eq!(findings.len(), 2);
        } else {
            panic!("expected ReviewExhausted");
        }
    }

    // ── Cancellation ───────────────────────────────────────────────────

    #[test]
    fn cancel_from_any_phase() {
        let mut ctrl = make_controller(WorkflowTemplateDescriptor::mechanical());

        ctrl.step(PhaseInput::Start);
        ctrl.step(PhaseInput::ComposeCompleted);

        let action = ctrl.step(PhaseInput::Cancel);
        assert!(action.is_terminal());
        assert!(matches!(
            action,
            ControllerAction::Terminal(WorkflowTermination::Cancelled)
        ));
    }

    // ── Review rejection ───────────────────────────────────────────────

    #[test]
    fn review_rejected_terminates() {
        let mut ctrl = make_controller(WorkflowTemplateDescriptor::focused());

        ctrl.step(PhaseInput::Start);
        ctrl.step(PhaseInput::ComposeCompleted);
        ctrl.step(PhaseInput::ImplementCompleted { files_changed: 1 });
        ctrl.step(PhaseInput::GatePassed);

        let action = ctrl.step(PhaseInput::ReviewCompleted {
            verdict: ReviewVerdict::Rejected {
                reason: "security issue".to_string(),
            },
        });
        assert!(action.is_terminal());
        assert!(matches!(
            action,
            ControllerAction::Terminal(WorkflowTermination::Failed { .. })
        ));
    }

    // ── Idempotency key ────────────────────────────────────────────────

    #[test]
    fn idempotency_key_format() {
        let scope = ActivityScope {
            run_id: "run-001".to_string(),
            template_name: "mechanical".to_string(),
            template_version: 1,
            workflow_id: "wf-001".to_string(),
            generation: 0,
            phase: WorkflowPhase::Implement,
            attempt: 0,
        };
        let key = idempotency_key(&scope);
        assert_eq!(key, "run-001/mechanical/mechanical@1/wf-001/0/implement/0");
    }

    #[test]
    fn different_generations_produce_different_keys() {
        let mut ctrl = make_controller(WorkflowTemplateDescriptor::mechanical());

        ctrl.step(PhaseInput::Start);
        let key0 = ctrl.current_idempotency_key();

        ctrl.step(PhaseInput::ComposeCompleted);
        ctrl.step(PhaseInput::ImplementCompleted { files_changed: 1 });
        ctrl.step(PhaseInput::GateFailed {
            output: "error".to_string(),
        });
        let key1 = ctrl.current_idempotency_key();

        assert_ne!(key0, key1);
    }

    // ── Unclear review -> treated as revise ────────────────────────────

    #[test]
    fn unclear_review_treated_as_revise() {
        let mut ctrl = make_controller(WorkflowTemplateDescriptor::focused());

        ctrl.step(PhaseInput::Start);
        ctrl.step(PhaseInput::ComposeCompleted);
        ctrl.step(PhaseInput::ImplementCompleted { files_changed: 1 });
        ctrl.step(PhaseInput::GatePassed);

        let action = ctrl.step(PhaseInput::ReviewCompleted {
            verdict: ReviewVerdict::Unclear {
                summary: "ambiguous output".to_string(),
            },
        });
        // Should get a new compose generation, not a terminal.
        assert_eq!(action, ControllerAction::RunCompose { generation: 1 });
    }

    // ── Terminated controller returns terminal ─────────────────────────

    #[test]
    fn terminated_controller_stays_terminated() {
        let mut ctrl = make_controller(WorkflowTemplateDescriptor::mechanical());

        ctrl.step(PhaseInput::Start);
        ctrl.step(PhaseInput::Cancel);
        assert!(ctrl.is_terminated());

        // Further steps still return Terminal.
        let action = ctrl.step(PhaseInput::Start);
        assert!(action.is_terminal());
    }

    // ── Serde round-trip ───────────────────────────────────────────────

    #[test]
    fn controller_serde_roundtrip() {
        let mut ctrl = make_controller(WorkflowTemplateDescriptor::focused());
        ctrl.step(PhaseInput::Start);
        ctrl.step(PhaseInput::ComposeCompleted);

        let json = serde_json::to_string(&ctrl).unwrap();
        let back: WorkflowGraphController = serde_json::from_str(&json).unwrap();
        assert_eq!(back.run_id, ctrl.run_id);
        assert_eq!(back.current_phase, ctrl.current_phase);
        assert_eq!(back.iterations_used, ctrl.iterations_used);
    }

    #[test]
    fn phase_receipt_records() {
        let mut ctrl = make_controller(WorkflowTemplateDescriptor::mechanical());
        ctrl.step(PhaseInput::Start);

        let receipt = PhaseReceipt {
            scope: ctrl.current_scope(),
            success: true,
            output: Some("prompt built".to_string()),
            error: None,
        };
        ctrl.record_receipt(receipt);

        assert_eq!(ctrl.receipts.len(), 1);
        assert!(ctrl.receipts[0].success);
    }
}
