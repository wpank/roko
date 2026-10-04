//! Typed done-gate contract for self-hosting Roko tasks.
//!
//! The contract describes the evidence a task was meant to produce before it
//! could be marked done. Its evaluator is retired (decision 3205): a task's
//! `acceptance_contract` is still parsed, so archived plans load, and
//! [`AcceptanceContract::validate_contract`] still checks its shape, but
//! nothing evaluates evidence against it at run time, and `roko plan validate`
//! warns when a contract is a task's only acceptance (PLAN_046). A task states
//! its acceptance criteria in `acceptance`, with verify steps that name them in
//! `covers`, or pins a planner-written test with `[task.accept]`; beside those,
//! a contract is metadata, such as an architecture packet's parity rows.
//! [`ReviewVerdictEvidence`] and [`RequiredNextAction`] remain for structured
//! review verdicts.

use serde::{Deserialize, Serialize};

/// Terminal and actionable states for a done-gate decision.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AcceptanceOutcome {
    /// All required evidence is present and passing.
    Passed,
    /// Required evidence failed or is malformed.
    Failed,
    /// Work cannot proceed with the current external state.
    Blocked,
    /// A required gate exceeded its time budget.
    TimedOut,
    /// The run was cancelled before a terminal verdict.
    Cancelled,
    /// Evidence points to a bounded retry of the same task.
    NeedsRetry,
    /// Evidence points to changing the plan before retrying.
    NeedsReplan,
    /// Evidence requires human review or approval.
    NeedsHuman,
    /// Required evidence is incomplete and the task needs more implementation work.
    NeedsWork,
}

/// A typed acceptance contract for one self-hosting task.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AcceptanceContract {
    /// Contract schema version. Only version 1 is accepted.
    pub version: u32,
    /// Compile/test/lint or custom commands that must produce gate evidence.
    #[serde(default)]
    pub gates: Vec<GateRequirement>,
    /// Requirement that production paths are not satisfied by stubs/noops.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub no_stub: Option<NoStubRequirement>,
    /// Requirement for structurally parseable agent output.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent_output: Option<StructuredAgentOutputRequirement>,
    /// Requirement for a structured reviewer verdict.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub review_verdict: Option<ReviewVerdictRequirement>,
    /// Requirement to record retry/reflection/replan signals after failures.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recovery: Option<RecoveryRequirement>,
    /// Requirement to attach doc parity evidence rows.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parity_ledger: Option<ParityLedgerRequirement>,
}

impl AcceptanceContract {
    /// Validate contract shape before any evidence is evaluated.
    #[must_use]
    pub fn validate_contract(&self) -> AcceptanceDecision {
        let mut issues = Vec::new();

        if self.version != 1 {
            issues.push(AcceptanceIssue::blocking(
                "ACCEPT_001",
                format!("unsupported acceptance contract version {}", self.version),
            ));
        }

        for gate in &self.gates {
            if gate.id.trim().is_empty() {
                issues.push(AcceptanceIssue::blocking(
                    "ACCEPT_002",
                    "gate requirement is missing id",
                ));
            }
            if gate.command.as_deref().is_none_or(str::is_empty) {
                issues.push(AcceptanceIssue::blocking(
                    "ACCEPT_003",
                    format!("gate '{}' is missing command", gate.id),
                ));
            }
        }

        if let Some(no_stub) = &self.no_stub
            && no_stub.required
            && no_stub.production_paths.is_empty()
        {
            issues.push(AcceptanceIssue::blocking(
                "ACCEPT_004",
                "no-stub requirement must name at least one production path",
            ));
        }

        if let Some(agent_output) = &self.agent_output
            && agent_output.required
            && agent_output.schema.trim().is_empty()
        {
            issues.push(AcceptanceIssue::blocking(
                "ACCEPT_005",
                "structured agent output requirement is missing schema",
            ));
        }

        if let Some(review) = &self.review_verdict {
            if review.required && review.reviewer_role_id.trim().is_empty() {
                issues.push(AcceptanceIssue::blocking(
                    "ACCEPT_006",
                    "review verdict requirement is missing reviewer_role_id",
                ));
            }
            if !(0.0..=1.0).contains(&review.min_confidence) {
                issues.push(AcceptanceIssue::blocking(
                    "ACCEPT_007",
                    "review verdict min_confidence must be in 0.0..=1.0",
                ));
            }
        }

        if let Some(recovery) = &self.recovery
            && recovery.required
            && !recovery.retry
            && !recovery.reflection
            && !recovery.replan
        {
            issues.push(AcceptanceIssue::blocking(
                "ACCEPT_008",
                "recovery requirement must require retry, reflection, or replan evidence",
            ));
        }

        if let Some(parity) = &self.parity_ledger
            && parity.required
            && parity.rows.is_empty()
        {
            issues.push(AcceptanceIssue::blocking(
                "ACCEPT_009",
                "parity ledger requirement must declare at least one row",
            ));
        }

        for row in self
            .parity_ledger
            .as_ref()
            .into_iter()
            .flat_map(|parity| &parity.rows)
        {
            if row.requirement_id.trim().is_empty() {
                issues.push(AcceptanceIssue::blocking(
                    "ACCEPT_010",
                    "parity ledger row is missing requirement_id",
                ));
            }
            if row.evidence_ref.trim().is_empty()
                && row
                    .implementation_refs
                    .iter()
                    .all(|value| value.trim().is_empty())
            {
                issues.push(AcceptanceIssue::blocking(
                    "ACCEPT_011",
                    format!(
                        "parity ledger row '{}' is missing implementation evidence",
                        row.requirement_id
                    ),
                ));
            }
            if row.source_ref.trim().is_empty() {
                issues.push(AcceptanceIssue::blocking(
                    "ACCEPT_012",
                    format!(
                        "parity ledger row '{}' is missing source_ref",
                        row.requirement_id
                    ),
                ));
            }
        }

        decision_from_issues(issues)
    }
}

/// A single required verification command.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GateRequirement {
    /// Stable identifier used by evidence packets.
    pub id: String,
    /// The kind of gate this command represents.
    pub kind: GateRequirementKind,
    /// Shell command or named gate invocation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    /// Whether this gate blocks completion.
    #[serde(default = "default_true")]
    pub required: bool,
}

/// Verify categories understood by the done-gate contract.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GateRequirementKind {
    /// A compile/build gate such as `cargo check`.
    Compile,
    /// A test gate such as `cargo test`.
    Test,
    /// A lint/static-analysis gate such as `cargo clippy`.
    Lint,
    /// A structured review gate.
    Review,
    /// A caller-defined gate category.
    Custom,
}

/// No-stub production path requirement.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NoStubRequirement {
    /// Whether no-stub evidence blocks completion.
    #[serde(default = "default_true")]
    pub required: bool,
    /// Paths that must be covered by non-stub evidence.
    #[serde(default)]
    pub production_paths: Vec<String>,
}

/// Structured output requirement for the implementing agent.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StructuredAgentOutputRequirement {
    /// Whether output parsing blocks completion.
    #[serde(default = "default_true")]
    pub required: bool,
    /// Schema id, JSON schema path, or manifest id.
    pub schema: String,
}

/// Structured review verdict requirement.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ReviewVerdictRequirement {
    /// Whether review verdict evidence blocks completion.
    #[serde(default = "default_true")]
    pub required: bool,
    /// Reviewer role/profile id expected to produce the verdict.
    pub reviewer_role_id: String,
    /// Minimum accepted confidence in `[0.0, 1.0]`.
    #[serde(default)]
    pub min_confidence: f32,
}

/// Required recovery signals after failed gates.
#[allow(clippy::struct_excessive_bools)]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RecoveryRequirement {
    /// Whether recovery evidence blocks completion.
    #[serde(default = "default_true")]
    pub required: bool,
    /// Require a retry signal.
    #[serde(default)]
    pub retry: bool,
    /// Require a reflection signal.
    #[serde(default)]
    pub reflection: bool,
    /// Require a replan signal.
    #[serde(default)]
    pub replan: bool,
}

/// Parity ledger evidence requirement.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ParityLedgerRequirement {
    /// Whether parity ledger rows block completion.
    #[serde(default = "default_true")]
    pub required: bool,
    /// Required ledger rows for implemented doc requirements.
    #[serde(default)]
    pub rows: Vec<ParityLedgerRequirementRow>,
}

/// A required parity ledger row.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ParityLedgerRequirementRow {
    /// Stable doc requirement id.
    pub requirement_id: String,
    /// Source document path or requirement reference.
    pub source_ref: String,
    /// Legacy implementation evidence artifact path or structured ledger key.
    #[serde(default)]
    pub evidence_ref: String,
    /// Implementation evidence artifact paths or structured ledger keys.
    #[serde(default)]
    pub implementation_refs: Vec<String>,
    /// Declared test or gate evidence refs for this requirement, when known at plan time.
    #[serde(default)]
    pub test_evidence_refs: Vec<String>,
}

/// Review verdict evidence with enough structure for orchestration.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ReviewVerdictEvidence {
    /// Stable verdict id.
    pub verdict_id: String,
    /// Batch or task family id.
    pub batch_id: String,
    /// Task id.
    pub task_id: String,
    /// Reviewer role/profile id.
    pub reviewer_role_id: String,
    /// Structured review outcome.
    pub status: AcceptanceOutcome,
    /// Reviewer confidence in `[0.0, 1.0]`.
    pub confidence: f32,
    /// Blocking findings.
    #[serde(default)]
    pub blocking_findings: Vec<String>,
    /// Non-blocking findings.
    #[serde(default)]
    pub non_blocking_findings: Vec<String>,
    /// Required next action for the orchestrator.
    pub required_next_action: RequiredNextAction,
    /// Evidence paths or artifact ids.
    #[serde(default)]
    pub evidence_refs: Vec<String>,
    /// Raw reviewer output path or artifact id.
    pub raw_output_ref: String,
    /// Creation timestamp supplied by the caller.
    pub created_at: String,
}

/// Required next action from a structured review verdict.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RequiredNextAction {
    /// No follow-up action is required.
    None,
    /// Retry the task with bounded new evidence.
    Retry,
    /// Record reflection before continuing.
    Reflect,
    /// Replan before retrying.
    Replan,
    /// Escalate to a human reviewer.
    Human,
}

/// Done-gate validation decision.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AcceptanceDecision {
    /// Final outcome after fail-closed validation.
    pub outcome: AcceptanceOutcome,
    /// Structured validation issues.
    #[serde(default)]
    pub issues: Vec<AcceptanceIssue>,
}

impl AcceptanceDecision {
    /// True only when the decision has no blocking issue and the outcome passed.
    #[must_use]
    pub fn passed(&self) -> bool {
        self.outcome == AcceptanceOutcome::Passed && self.issues.iter().all(|issue| !issue.blocking)
    }
}

/// Validation issue for a contract or evidence packet.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AcceptanceIssue {
    /// Stable machine-readable code.
    pub code: String,
    /// Human-readable diagnostic.
    pub message: String,
    /// Whether this issue blocks completion.
    pub blocking: bool,
}

impl AcceptanceIssue {
    fn blocking(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            blocking: true,
        }
    }
}

fn decision_from_issues(issues: Vec<AcceptanceIssue>) -> AcceptanceDecision {
    let outcome = if issues.iter().any(|issue| issue.blocking) {
        AcceptanceOutcome::Failed
    } else {
        AcceptanceOutcome::Passed
    };
    AcceptanceDecision { outcome, issues }
}

const fn default_true() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn full_contract() -> AcceptanceContract {
        AcceptanceContract {
            version: 1,
            gates: vec![
                GateRequirement {
                    id: "compile".to_string(),
                    kind: GateRequirementKind::Compile,
                    command: Some("cargo check -p roko-gate".to_string()),
                    required: true,
                },
                GateRequirement {
                    id: "test".to_string(),
                    kind: GateRequirementKind::Test,
                    command: Some("cargo test -p roko-gate --lib --no-run".to_string()),
                    required: true,
                },
            ],
            no_stub: Some(NoStubRequirement {
                required: true,
                production_paths: vec!["crates/roko-gate/src".to_string()],
            }),
            agent_output: Some(StructuredAgentOutputRequirement {
                required: true,
                schema: "roko.acceptance.agent_output.v1".to_string(),
            }),
            review_verdict: Some(ReviewVerdictRequirement {
                required: true,
                reviewer_role_id: "quick-reviewer".to_string(),
                min_confidence: 0.6,
            }),
            recovery: Some(RecoveryRequirement {
                required: true,
                retry: true,
                reflection: true,
                replan: true,
            }),
            parity_ledger: Some(ParityLedgerRequirement {
                required: true,
                rows: vec![ParityLedgerRequirementRow {
                    requirement_id: "RT00.done-gate".to_string(),
                    source_ref: "tmp/architecture-plans/08-end-to-end-acceptance.md".to_string(),
                    evidence_ref: "crates/roko-gate/src/acceptance_contract.rs".to_string(),
                    implementation_refs: Vec::new(),
                    test_evidence_refs: Vec::new(),
                }],
            }),
        }
    }

    #[test]
    fn malformed_contract_fails_closed() {
        let mut contract = full_contract();
        contract.version = 99;

        let decision = contract.validate_contract();

        assert_eq!(decision.outcome, AcceptanceOutcome::Failed);
        assert!(
            decision
                .issues
                .iter()
                .any(|issue| issue.code == "ACCEPT_001")
        );
    }

    #[test]
    fn parity_requirement_accepts_structured_implementation_refs() {
        let mut contract = full_contract();
        let row = &mut contract
            .parity_ledger
            .as_mut()
            .expect("parity requirement")
            .rows[0];
        row.evidence_ref.clear();
        row.implementation_refs = vec!["crates/roko-gate/src/acceptance_contract.rs".to_string()];

        let decision = contract.validate_contract();

        assert!(decision.passed(), "{decision:?}");
    }
}
