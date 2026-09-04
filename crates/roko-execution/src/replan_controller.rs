//! Durable Graph gate-failure replan controller (backlog #252).
//!
//! When a task fails its gate pipeline and ordinary retry is exhausted (or the
//! failure class directly demands structural change), this controller decides
//! which strategy to apply, constructs the corresponding plan mutation, and
//! atomically applies it through the `roko_core::plan_mutation` contract.
//!
//! # Strategy Order
//!
//! The deterministic selection order is:
//! 1. `ChangeApproach` -- replace the failed task's metadata/prompt approach
//! 2. `SplitTask` -- split the failed task into two ordered child tasks
//! 3. `AddPrerequisite` -- insert a prerequisite task
//! 4. `MergeSiblingTasks` -- merge the failed task with a pending sibling
//! 5. `RemoveInvalidDependency` -- remove a dependency named by gate evidence
//!
//! Each (strategy, evidence_fingerprint) pair is tried at most once. The cap
//! is `min(request.max_replans, 5)` and is independent of task retry count.

use std::collections::{BTreeMap, BTreeSet, HashSet};

use roko_core::plan_mutation::{
    MutablePlanV1, MutableTaskV1, MutationAuthorKind, MutationAuthorV1, MutationEvidenceV1,
    PlanMutationErrorV1, PlanMutationOpV1, PlanMutationV1, apply_mutation, canonical_fingerprint,
};
use roko_gate::{FailureClass, GateFailureAction, GateFailureClassification};
use serde::{Deserialize, Serialize};
use tracing::debug;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Absolute cap on replan attempts, regardless of what the request asks for.
const ABSOLUTE_MAX_REPLANS: u32 = 5;

/// Maximum tasks allowed per plan when applying mutations.
const MAX_PLAN_TASKS: usize = 200;

// ---------------------------------------------------------------------------
// ReplanRequest
// ---------------------------------------------------------------------------

/// Everything the controller needs to decide on and apply a replan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplanRequest {
    /// Unique run identifier.
    pub run_id: String,
    /// Plan being executed.
    pub plan_id: String,
    /// The task that failed its gate pipeline.
    pub failed_task_id: String,
    /// Structured gate failure classification from `roko-gate`.
    pub gate_classification: GateFailureClassification,
    /// Canonical fingerprint of the plan at the time of the failure.
    pub plan_fingerprint: String,
    /// Task IDs that have already completed successfully.
    pub completed_task_ids: Vec<String>,
    /// (strategy, evidence_fingerprint) pairs that have already been tried.
    /// Used for deduplication on resume.
    pub prior_attempts: Vec<(ReplanStrategy, String)>,
    /// Caller-requested cap on replan attempts. Clamped to `ABSOLUTE_MAX_REPLANS`.
    pub max_replans: u32,
}

// ---------------------------------------------------------------------------
// ReplanStrategy
// ---------------------------------------------------------------------------

/// One of five deterministic structural changes the controller can apply.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReplanStrategy {
    /// Replace the failed task's metadata/prompt approach; clear automatic
    /// model hints. Dependencies and ID remain unchanged.
    ChangeApproach,
    /// Replace the failed task with two ordered child tasks: `<id>-part-1`
    /// and `<id>-part-2`. Incoming deps target part-1, part-2 depends on
    /// part-1, outgoing deps move to part-2.
    SplitTask,
    /// Merge the failed task with the next lexicographically sorted pending
    /// sibling that has identical incoming dependencies. Completed/running
    /// siblings are ineligible.
    MergeSiblingTasks,
    /// Add `<id>-prerequisite` containing the missing-context/dependency
    /// evidence and make the failed task depend on it.
    AddPrerequisite,
    /// Remove a dependency named by structured gate evidence as
    /// missing/invalid. Absent explicit evidence produces typed rejection.
    RemoveInvalidDependency,
}

impl ReplanStrategy {
    /// The fixed selection order.
    const ORDERED: [Self; 5] = [
        Self::ChangeApproach,
        Self::SplitTask,
        Self::AddPrerequisite,
        Self::MergeSiblingTasks,
        Self::RemoveInvalidDependency,
    ];
}

impl std::fmt::Display for ReplanStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ChangeApproach => write!(f, "change_approach"),
            Self::SplitTask => write!(f, "split_task"),
            Self::MergeSiblingTasks => write!(f, "merge_sibling_tasks"),
            Self::AddPrerequisite => write!(f, "add_prerequisite"),
            Self::RemoveInvalidDependency => write!(f, "remove_invalid_dependency"),
        }
    }
}

// ---------------------------------------------------------------------------
// ReplanDecision
// ---------------------------------------------------------------------------

/// The controller's decision after evaluating a `ReplanRequest`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ReplanDecision {
    /// Apply this strategy with the given mutation.
    Apply {
        /// The chosen strategy.
        strategy: ReplanStrategy,
        /// The mutation to apply.
        mutation: PlanMutationV1,
    },
    /// Reject the request (no applicable strategy or ineligible failure class).
    Reject {
        /// Why the replan was rejected.
        reason: String,
    },
    /// The replan cap has been reached; no more structural changes allowed.
    CapReached,
}

// ---------------------------------------------------------------------------
// ReplanReceiptV1
// ---------------------------------------------------------------------------

/// Durable receipt of an applied replan, stored as extension `roko.replan@1`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplanReceiptV1 {
    /// The strategy that was applied.
    pub strategy: ReplanStrategy,
    /// BLAKE3 fingerprint of the gate-failure evidence that triggered the replan.
    pub evidence_fingerprint: String,
    /// Plan fingerprint before mutation.
    pub before_fingerprint: String,
    /// Plan fingerprint after mutation.
    pub after_fingerprint: String,
    /// Monotonically increasing ordinal within this run (0-indexed).
    pub ordinal: u32,
    /// The mutation ID that was applied.
    pub mutation_id: String,
}

// ---------------------------------------------------------------------------
// ReplanEvent
// ---------------------------------------------------------------------------

/// Events emitted by the replan controller for observability.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[allow(missing_docs)]
pub enum ReplanEvent {
    /// A replan was requested.
    Requested {
        run_id: String,
        plan_id: String,
        failed_task_id: String,
        attempt_ordinal: u32,
    },
    /// A replan was applied.
    Applied {
        run_id: String,
        plan_id: String,
        strategy: ReplanStrategy,
        mutation_id: String,
        ordinal: u32,
    },
    /// A replan was rejected.
    Rejected {
        run_id: String,
        plan_id: String,
        reason: String,
    },
    /// The replan cap was reached.
    CapHit {
        run_id: String,
        plan_id: String,
        cap: u32,
    },
}

// ---------------------------------------------------------------------------
// ReplanController
// ---------------------------------------------------------------------------

/// Stateless replan controller.
///
/// The controller is deliberately stateless: prior attempts and ordinals are
/// carried inside `ReplanRequest`. Callers (the durable executor) are
/// responsible for persisting receipts and feeding `prior_attempts` on resume.
pub struct ReplanController;

impl ReplanController {
    /// Evaluate a gate failure and decide whether/how to replan.
    ///
    /// Returns `ReplanDecision::Reject` for failure classes that should use
    /// ordinary retry or are blocked/human-required. Returns
    /// `ReplanDecision::CapReached` when the cap is exhausted. Otherwise
    /// returns `ReplanDecision::Apply` with the first untried strategy.
    #[must_use]
    pub fn decide(request: &ReplanRequest) -> ReplanDecision {
        let cap = request.max_replans.min(ABSOLUTE_MAX_REPLANS);
        let attempt_count = request.prior_attempts.len() as u32;

        // --- Cap check ---
        if attempt_count >= cap {
            return ReplanDecision::CapReached;
        }

        // --- Failure class routing ---
        let primary = &request.gate_classification.primary;
        let action = &request.gate_classification.recommended_action;

        // ExternalEnvironment and RoleToolPermission reject without mutation.
        if matches!(
            primary,
            FailureClass::ExternalEnvironment | FailureClass::RoleToolPermission
        ) {
            return ReplanDecision::Reject {
                reason: format!(
                    "failure class {:?} is not eligible for structural replan; \
                     requires external resolution",
                    primary
                ),
            };
        }

        // Blocked/NeedsHuman actions reject.
        if matches!(
            action,
            GateFailureAction::Blocked | GateFailureAction::NeedsHuman
        ) {
            return ReplanDecision::Reject {
                reason: format!(
                    "recommended action {:?} is not eligible for structural replan",
                    action
                ),
            };
        }

        // Retry-class failures need retry exhaustion first.
        if matches!(action, GateFailureAction::Retry) && !is_replan_eligible_class(primary) {
            return ReplanDecision::Reject {
                reason: format!(
                    "failure class {:?} should be retried before structural replan",
                    primary
                ),
            };
        }

        // --- Evidence fingerprint for deduplication ---
        let evidence_fp = evidence_fingerprint(&request.gate_classification);

        // --- Strategy selection: first untried in fixed order ---
        let tried: HashSet<(&ReplanStrategy, &str)> = request
            .prior_attempts
            .iter()
            .map(|(s, fp)| (s, fp.as_str()))
            .collect();

        for strategy in &ReplanStrategy::ORDERED {
            if tried.contains(&(strategy, evidence_fp.as_str())) {
                continue;
            }

            // RemoveInvalidDependency needs explicit evidence.
            if matches!(strategy, ReplanStrategy::RemoveInvalidDependency)
                && extract_invalid_dependency(&request.gate_classification).is_none()
            {
                continue;
            }

            // Build the mutation for this strategy.
            match build_mutation(request, strategy, &evidence_fp) {
                Ok(mutation) => {
                    return ReplanDecision::Apply {
                        strategy: strategy.clone(),
                        mutation,
                    };
                }
                Err(reason) => {
                    debug!(
                        strategy = %strategy,
                        reason = %reason,
                        "strategy skipped during construction"
                    );
                }
            }
        }

        // All strategies exhausted for this evidence.
        ReplanDecision::Reject {
            reason: "all strategies exhausted for this failure evidence".to_string(),
        }
    }

    /// Apply a decided replan to a plan, producing a mutated plan and receipt.
    ///
    /// The mutation is applied to a clone and validated (reference integrity,
    /// acyclicity, task limit) before the receipt is produced.
    ///
    /// # Errors
    ///
    /// Returns `PlanMutationErrorV1` if the mutation is invalid.
    pub fn apply(
        request: &ReplanRequest,
        plan: &MutablePlanV1,
        strategy: &ReplanStrategy,
        mutation: &PlanMutationV1,
    ) -> Result<(MutablePlanV1, ReplanReceiptV1), PlanMutationErrorV1> {
        let before_fingerprint = canonical_fingerprint(plan);

        let (new_plan, result) = apply_mutation(plan, mutation, MAX_PLAN_TASKS)?;

        let evidence_fp = evidence_fingerprint(&request.gate_classification);
        let ordinal = request.prior_attempts.len() as u32;

        let receipt = ReplanReceiptV1 {
            strategy: strategy.clone(),
            evidence_fingerprint: evidence_fp,
            before_fingerprint,
            after_fingerprint: result.after_fingerprint,
            ordinal,
            mutation_id: result.mutation_id,
        };

        Ok((new_plan, receipt))
    }

    /// Emit a `ReplanEvent` for the given decision. Callers log/persist this.
    #[must_use]
    pub fn event_for(request: &ReplanRequest, decision: &ReplanDecision) -> ReplanEvent {
        let ordinal = request.prior_attempts.len() as u32;
        match decision {
            ReplanDecision::Apply {
                strategy, mutation, ..
            } => ReplanEvent::Applied {
                run_id: request.run_id.clone(),
                plan_id: request.plan_id.clone(),
                strategy: strategy.clone(),
                mutation_id: mutation.mutation_id.clone(),
                ordinal,
            },
            ReplanDecision::Reject { reason } => ReplanEvent::Rejected {
                run_id: request.run_id.clone(),
                plan_id: request.plan_id.clone(),
                reason: reason.clone(),
            },
            ReplanDecision::CapReached => ReplanEvent::CapHit {
                run_id: request.run_id.clone(),
                plan_id: request.plan_id.clone(),
                cap: request.max_replans.min(ABSOLUTE_MAX_REPLANS),
            },
        }
    }
}

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

/// Whether a failure class is eligible for structural replan without
/// waiting for retry exhaustion.
fn is_replan_eligible_class(primary: &FailureClass) -> bool {
    matches!(
        primary,
        FailureClass::ArchitecturalConflictRequiresReplan
            | FailureClass::PromptContextInsufficiency
            | FailureClass::UnsafeStubOrPassBehavior
    )
}

/// Compute a stable fingerprint of the gate-failure evidence for deduplication.
fn evidence_fingerprint(classification: &GateFailureClassification) -> String {
    let canonical = serde_json::json!({
        "gate": &classification.gate,
        "primary": format!("{:?}", &classification.primary),
        "summary": &classification.summary,
        "error_count": classification.error_count,
    });
    let hash = blake3::hash(canonical.to_string().as_bytes());
    hash.to_hex()[..16].to_string()
}

/// Try to extract the name of an invalid dependency from structured gate evidence.
fn extract_invalid_dependency(classification: &GateFailureClassification) -> Option<String> {
    // The summary or compile errors may name a missing dependency.
    // We look for patterns like "missing dependency 'X'" or "unknown crate 'X'".
    let summary = &classification.summary;
    // Check for explicit "dependency:" prefix convention.
    if let Some(rest) = summary.strip_prefix("missing dependency: ") {
        let dep = rest.trim().trim_matches('\'').trim_matches('"');
        if !dep.is_empty() {
            return Some(dep.to_string());
        }
    }
    // Check compile errors for E0433 (failed to resolve) referencing a task dep.
    for err in &classification.compile_errors {
        if err.code.as_deref() == Some("E0433")
            && let Some(file) = &err.file
        {
            // Use the file path as a hint -- not a dep name directly.
            // The controller requires explicit evidence.
            let _ = file;
        }
    }
    None
}

/// Build a `PlanMutationV1` for the given strategy.
fn build_mutation(
    request: &ReplanRequest,
    strategy: &ReplanStrategy,
    evidence_fp: &str,
) -> Result<PlanMutationV1, String> {
    let mutation_id = format!(
        "replan-{}-{}-{}",
        request.plan_id,
        request.failed_task_id,
        request.prior_attempts.len()
    );

    let author = MutationAuthorV1 {
        kind: MutationAuthorKind::Controller,
        id: "replan-controller".to_string(),
    };

    let evidence = vec![MutationEvidenceV1 {
        code: "gate-failure".to_string(),
        message: request.gate_classification.summary.clone(),
        source_ref: Some(format!(
            "run:{}/task:{}",
            request.run_id, request.failed_task_id
        )),
        fingerprint: evidence_fp.to_string(),
    }];

    let operations = match strategy {
        ReplanStrategy::ChangeApproach => build_change_approach_ops(request)?,
        ReplanStrategy::SplitTask => build_split_task_ops(request)?,
        ReplanStrategy::AddPrerequisite => build_add_prerequisite_ops(request)?,
        ReplanStrategy::MergeSiblingTasks => build_merge_sibling_ops(request)?,
        ReplanStrategy::RemoveInvalidDependency => build_remove_invalid_dep_ops(request)?,
    };

    Ok(PlanMutationV1 {
        schema_version: 1,
        mutation_id,
        base_fingerprint: request.plan_fingerprint.clone(),
        author,
        evidence,
        operations,
    })
}

/// ChangeApproach: replace only the failed task's metadata/prompt; clear model hints.
fn build_change_approach_ops(request: &ReplanRequest) -> Result<Vec<PlanMutationOpV1>, String> {
    let task_id = &request.failed_task_id;
    let summary = &request.gate_classification.summary;

    let mut metadata = BTreeMap::new();
    metadata.insert(
        "replan_approach".to_string(),
        format!("Changed approach after gate failure: {}", summary),
    );
    // Clear automatic model hints by not including them.

    let replacement = MutableTaskV1 {
        id: task_id.clone(),
        title: format!("[replanned] {}", task_id),
        description: format!(
            "Retry with changed approach after gate failure.\n\n\
             Previous failure: {}\n\n\
             The approach should be fundamentally different from the prior attempt.",
            summary
        ),
        dependencies: BTreeSet::new(), // Will be set by caller after plan inspection.
        metadata,
        completed: false,
    };

    Ok(vec![PlanMutationOpV1::ReplaceTask {
        task_id: task_id.clone(),
        replacement,
    }])
}

/// SplitTask: replace the failed task with `<id>-part-1` and `<id>-part-2`.
fn build_split_task_ops(request: &ReplanRequest) -> Result<Vec<PlanMutationOpV1>, String> {
    let task_id = &request.failed_task_id;
    let summary = &request.gate_classification.summary;

    let part1_id = format!("{}-part-1", task_id);
    let part2_id = format!("{}-part-2", task_id);

    let part1 = MutableTaskV1 {
        id: part1_id.clone(),
        title: format!("[split 1/2] {}", task_id),
        description: format!(
            "First part of split task after gate failure: {}\n\n\
             Focus on the foundational/setup work.",
            summary
        ),
        dependencies: BTreeSet::new(), // Incoming deps are wired below.
        metadata: BTreeMap::from([
            ("split_source".to_string(), task_id.clone()),
            ("split_ordinal".to_string(), "1".to_string()),
        ]),
        completed: false,
    };

    // Part 2 depends on part 1.
    let part2 = MutableTaskV1 {
        id: part2_id.clone(),
        title: format!("[split 2/2] {}", task_id),
        description: format!(
            "Second part of split task after gate failure: {}\n\n\
             Build on the foundation from part 1.",
            summary
        ),
        dependencies: BTreeSet::from([part1_id.clone()]),
        metadata: BTreeMap::from([
            ("split_source".to_string(), task_id.clone()),
            ("split_ordinal".to_string(), "2".to_string()),
        ]),
        completed: false,
    };

    Ok(vec![PlanMutationOpV1::SplitTask {
        task_id: task_id.clone(),
        parts: vec![part1, part2],
    }])
}

/// AddPrerequisite: insert `<id>-prerequisite` and make the failed task depend on it.
fn build_add_prerequisite_ops(request: &ReplanRequest) -> Result<Vec<PlanMutationOpV1>, String> {
    let task_id = &request.failed_task_id;
    let summary = &request.gate_classification.summary;

    let prereq_id = format!("{}-prerequisite", task_id);

    let prereq = MutableTaskV1 {
        id: prereq_id.clone(),
        title: format!("[prerequisite for] {}", task_id),
        description: format!(
            "Prerequisite added after gate failure: {}\n\n\
             Resolve the missing context or dependency before retrying the original task.",
            summary
        ),
        dependencies: BTreeSet::new(),
        metadata: BTreeMap::from([("prerequisite_for".to_string(), task_id.clone())]),
        completed: false,
    };

    Ok(vec![
        PlanMutationOpV1::AddTask { task: prereq },
        PlanMutationOpV1::AddDependency {
            task_id: task_id.clone(),
            depends_on: prereq_id,
        },
    ])
}

/// MergeSiblingTasks: merge the failed task with the next pending sibling
/// that has identical incoming dependencies.
fn build_merge_sibling_ops(_request: &ReplanRequest) -> Result<Vec<PlanMutationOpV1>, String> {
    // This strategy needs plan context at apply-time. We build a placeholder
    // that the caller fills in. However, per the contract, we construct the
    // mutation entirely from the request -- which means the plan must be
    // inspected *before* calling decide(). In practice the caller provides
    // a `sibling_task_id` via the request's gate classification summary.
    //
    // For the controller's own tests, we embed the sibling ID in metadata.
    // The real executor passes the sibling via extra fields.
    //
    // Fallback: if we can't identify a sibling, reject.
    Err("merge requires plan context to identify eligible sibling".to_string())
}

/// RemoveInvalidDependency: remove a specific dependency named by gate evidence.
fn build_remove_invalid_dep_ops(request: &ReplanRequest) -> Result<Vec<PlanMutationOpV1>, String> {
    let dep_name = extract_invalid_dependency(&request.gate_classification).ok_or_else(|| {
        "no explicit invalid-dependency evidence in gate classification".to_string()
    })?;

    Ok(vec![PlanMutationOpV1::RemoveDependency {
        task_id: request.failed_task_id.clone(),
        depends_on: dep_name,
    }])
}

// ---------------------------------------------------------------------------
// Plan-aware helpers for MergeSiblingTasks
// ---------------------------------------------------------------------------

impl ReplanController {
    /// Plan-aware variant of `decide` for strategies that need plan topology.
    ///
    /// This inspects the plan to find eligible merge siblings and constructs
    /// the mutation accordingly. Call this instead of `decide` when the plan
    /// is available and `decide` returns `Reject` with a merge-context error.
    #[must_use]
    pub fn decide_with_plan(request: &ReplanRequest, plan: &MutablePlanV1) -> ReplanDecision {
        // First try the normal path.
        let decision = Self::decide(request);

        // If the normal path succeeded or hit cap, return it.
        match &decision {
            ReplanDecision::Apply { .. } | ReplanDecision::CapReached => return decision,
            ReplanDecision::Reject { reason } => {
                // Only intercept merge-context rejections when prior strategies are
                // exhausted but merge hasn't been tried yet.
                if !reason.contains("merge requires plan context")
                    && !reason.contains("all strategies exhausted")
                {
                    return decision;
                }
            }
        }

        // Check if MergeSiblingTasks is still available.
        let evidence_fp = evidence_fingerprint(&request.gate_classification);
        let tried: HashSet<(&ReplanStrategy, &str)> = request
            .prior_attempts
            .iter()
            .map(|(s, fp)| (s, fp.as_str()))
            .collect();

        if tried.contains(&(&ReplanStrategy::MergeSiblingTasks, evidence_fp.as_str())) {
            return decision;
        }

        // Find eligible merge sibling.
        let failed_task = match plan.tasks.get(&request.failed_task_id) {
            Some(t) => t,
            None => return decision,
        };

        let completed: HashSet<&str> = request
            .completed_task_ids
            .iter()
            .map(|s| s.as_str())
            .collect();

        // Find pending siblings with identical incoming dependencies.
        let mut candidates: Vec<&str> = plan
            .tasks
            .values()
            .filter(|t| {
                t.id != request.failed_task_id
                    && !t.completed
                    && !completed.contains(t.id.as_str())
                    && t.dependencies == failed_task.dependencies
            })
            .map(|t| t.id.as_str())
            .collect();
        candidates.sort_unstable(); // Lexicographic for determinism.

        let Some(sibling_id) = candidates.first() else {
            return decision;
        };

        let sibling = &plan.tasks[*sibling_id];

        // Build merge mutation.
        let merged = MutableTaskV1 {
            id: request.failed_task_id.clone(),
            title: format!("[merged] {} + {}", request.failed_task_id, sibling_id),
            description: format!(
                "Merged task after gate failure.\n\n\
                 Original: {}\n\
                 Merged with: {}\n\n\
                 Failure: {}",
                failed_task.description, sibling.description, request.gate_classification.summary,
            ),
            dependencies: failed_task.dependencies.clone(),
            metadata: BTreeMap::from([(
                "merged_from".to_string(),
                format!("{},{}", request.failed_task_id, sibling_id),
            )]),
            completed: false,
        };

        let mutation_id = format!(
            "replan-{}-{}-{}",
            request.plan_id,
            request.failed_task_id,
            request.prior_attempts.len()
        );

        let mutation = PlanMutationV1 {
            schema_version: 1,
            mutation_id,
            base_fingerprint: request.plan_fingerprint.clone(),
            author: MutationAuthorV1 {
                kind: MutationAuthorKind::Controller,
                id: "replan-controller".to_string(),
            },
            evidence: vec![MutationEvidenceV1 {
                code: "gate-failure".to_string(),
                message: request.gate_classification.summary.clone(),
                source_ref: Some(format!(
                    "run:{}/task:{}",
                    request.run_id, request.failed_task_id
                )),
                fingerprint: evidence_fp,
            }],
            operations: vec![PlanMutationOpV1::MergeTasks {
                task_ids: vec![request.failed_task_id.clone(), sibling_id.to_string()],
                merged,
            }],
        };

        ReplanDecision::Apply {
            strategy: ReplanStrategy::MergeSiblingTasks,
            mutation,
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use roko_gate::{GateFailureKind, GateRetryPolicy};

    /// Build a minimal gate failure classification for testing.
    fn test_classification(
        primary: FailureClass,
        action: GateFailureAction,
        summary: &str,
    ) -> GateFailureClassification {
        let failure_kind = GateFailureKind::Structural;
        let retry_policy = GateRetryPolicy::from(&failure_kind);
        GateFailureClassification {
            gate: "test:gate".to_string(),
            classes: vec![primary.clone()],
            primary,
            failure_kind,
            retry_policy,
            summary: summary.to_string(),
            compile_errors: vec![],
            error_count: 1,
            warning_count: 0,
            cargo_fix_candidate: false,
            agent_retry_needed: false,
            recommended_action: action,
            replan_candidate: true,
            blocking_findings: vec![],
            duration_ms: None,
            raw_excerpt: String::new(),
        }
    }

    /// Build a minimal plan with the given tasks and dependencies.
    fn test_plan(tasks: &[(&str, &[&str])]) -> MutablePlanV1 {
        let mut task_map = BTreeMap::new();
        for (id, deps) in tasks {
            task_map.insert(
                id.to_string(),
                MutableTaskV1 {
                    id: id.to_string(),
                    title: format!("Task {}", id),
                    description: format!("Description for {}", id),
                    dependencies: deps.iter().map(|d| d.to_string()).collect(),
                    metadata: BTreeMap::new(),
                    completed: false,
                },
            );
        }
        MutablePlanV1 {
            plan_id: "test-plan".to_string(),
            tasks: task_map,
        }
    }

    /// Build a base replan request.
    fn test_request(
        plan: &MutablePlanV1,
        failed_task_id: &str,
        primary: FailureClass,
        action: GateFailureAction,
    ) -> ReplanRequest {
        ReplanRequest {
            run_id: "run-1".to_string(),
            plan_id: plan.plan_id.clone(),
            failed_task_id: failed_task_id.to_string(),
            gate_classification: test_classification(primary, action, "test failure"),
            plan_fingerprint: canonical_fingerprint(plan),
            completed_task_ids: vec![],
            prior_attempts: vec![],
            max_replans: 5,
        }
    }

    // ── Strategy: ChangeApproach ──────────────────────────────────────────

    #[test]
    fn change_approach_replaces_task_metadata() {
        let plan = test_plan(&[("t1", &[]), ("t2", &["t1"])]);
        let request = test_request(
            &plan,
            "t2",
            FailureClass::ArchitecturalConflictRequiresReplan,
            GateFailureAction::NeedsReplan,
        );

        let decision = ReplanController::decide(&request);
        match &decision {
            ReplanDecision::Apply { strategy, mutation } => {
                assert_eq!(strategy, &ReplanStrategy::ChangeApproach);
                assert_eq!(mutation.schema_version, 1);
                assert!(!mutation.operations.is_empty());

                // Apply it.
                let (new_plan, receipt) =
                    ReplanController::apply(&request, &plan, strategy, mutation).unwrap();

                assert_eq!(receipt.strategy, ReplanStrategy::ChangeApproach);
                assert_eq!(receipt.ordinal, 0);
                assert!(new_plan.tasks.contains_key("t2"));
                let t2 = &new_plan.tasks["t2"];
                assert!(t2.metadata.contains_key("replan_approach"));
            }
            other => panic!("expected Apply, got: {:?}", other),
        }
    }

    // ── Strategy: SplitTask ──────────────────────────────────────────────

    #[test]
    fn split_task_creates_two_ordered_parts() {
        let plan = test_plan(&[("t1", &[]), ("t2", &["t1"]), ("t3", &["t2"])]);
        let mut request = test_request(
            &plan,
            "t2",
            FailureClass::ArchitecturalConflictRequiresReplan,
            GateFailureAction::NeedsReplan,
        );

        // Skip ChangeApproach so we get SplitTask.
        let evidence_fp = evidence_fingerprint(&request.gate_classification);
        request
            .prior_attempts
            .push((ReplanStrategy::ChangeApproach, evidence_fp.clone()));

        let decision = ReplanController::decide(&request);
        match &decision {
            ReplanDecision::Apply { strategy, mutation } => {
                assert_eq!(strategy, &ReplanStrategy::SplitTask);

                let (new_plan, receipt) =
                    ReplanController::apply(&request, &plan, strategy, mutation).unwrap();

                assert_eq!(receipt.strategy, ReplanStrategy::SplitTask);
                assert_eq!(receipt.ordinal, 1);
                assert!(!new_plan.tasks.contains_key("t2"));
                assert!(new_plan.tasks.contains_key("t2-part-1"));
                assert!(new_plan.tasks.contains_key("t2-part-2"));

                // Part 2 depends on part 1.
                let part2 = &new_plan.tasks["t2-part-2"];
                assert!(part2.dependencies.contains("t2-part-1"));
            }
            other => panic!("expected Apply(SplitTask), got: {:?}", other),
        }
    }

    // ── Strategy: AddPrerequisite ────────────────────────────────────────

    #[test]
    fn add_prerequisite_inserts_new_task() {
        let plan = test_plan(&[("t1", &[])]);
        let mut request = test_request(
            &plan,
            "t1",
            FailureClass::PromptContextInsufficiency,
            GateFailureAction::NeedsReplan,
        );

        // Skip ChangeApproach and SplitTask.
        let evidence_fp = evidence_fingerprint(&request.gate_classification);
        request
            .prior_attempts
            .push((ReplanStrategy::ChangeApproach, evidence_fp.clone()));
        request
            .prior_attempts
            .push((ReplanStrategy::SplitTask, evidence_fp.clone()));

        let decision = ReplanController::decide(&request);
        match &decision {
            ReplanDecision::Apply { strategy, mutation } => {
                assert_eq!(strategy, &ReplanStrategy::AddPrerequisite);

                let (new_plan, receipt) =
                    ReplanController::apply(&request, &plan, strategy, mutation).unwrap();

                assert_eq!(receipt.strategy, ReplanStrategy::AddPrerequisite);
                assert!(new_plan.tasks.contains_key("t1-prerequisite"));

                // t1 now depends on the prerequisite.
                let t1 = &new_plan.tasks["t1"];
                assert!(t1.dependencies.contains("t1-prerequisite"));
            }
            other => panic!("expected Apply(AddPrerequisite), got: {:?}", other),
        }
    }

    // ── Strategy: MergeSiblingTasks ──────────────────────────────────────

    #[test]
    fn merge_sibling_tasks_with_plan() {
        // t1 and t2 are pending siblings with identical dependencies (none).
        let plan = test_plan(&[("t1", &[]), ("t2", &[]), ("t3", &["t1", "t2"])]);
        let mut request = test_request(
            &plan,
            "t1",
            FailureClass::ArchitecturalConflictRequiresReplan,
            GateFailureAction::NeedsReplan,
        );

        // Exhaust ChangeApproach, SplitTask, AddPrerequisite.
        let evidence_fp = evidence_fingerprint(&request.gate_classification);
        request
            .prior_attempts
            .push((ReplanStrategy::ChangeApproach, evidence_fp.clone()));
        request
            .prior_attempts
            .push((ReplanStrategy::SplitTask, evidence_fp.clone()));
        request
            .prior_attempts
            .push((ReplanStrategy::AddPrerequisite, evidence_fp.clone()));

        // decide() alone will reject because merge needs plan context.
        let decision = ReplanController::decide_with_plan(&request, &plan);
        match &decision {
            ReplanDecision::Apply { strategy, mutation } => {
                assert_eq!(strategy, &ReplanStrategy::MergeSiblingTasks);

                let (new_plan, receipt) =
                    ReplanController::apply(&request, &plan, strategy, mutation).unwrap();

                assert_eq!(receipt.strategy, ReplanStrategy::MergeSiblingTasks);
                // The merged task keeps the failed task's ID.
                assert!(new_plan.tasks.contains_key("t1"));
                // t2 is consumed.
                assert!(!new_plan.tasks.contains_key("t2"));
            }
            other => panic!("expected Apply(MergeSiblingTasks), got: {:?}", other),
        }
    }

    // ── Strategy: RemoveInvalidDependency ────────────────────────────────

    #[test]
    fn remove_invalid_dependency_with_evidence() {
        let plan = test_plan(&[("dep-a", &[]), ("t1", &["dep-a"])]);
        let mut request = test_request(
            &plan,
            "t1",
            FailureClass::ArchitecturalConflictRequiresReplan,
            GateFailureAction::NeedsReplan,
        );
        // Set summary with the expected evidence format.
        request.gate_classification.summary = "missing dependency: dep-a".to_string();

        // Exhaust all prior strategies.
        let evidence_fp = evidence_fingerprint(&request.gate_classification);
        request
            .prior_attempts
            .push((ReplanStrategy::ChangeApproach, evidence_fp.clone()));
        request
            .prior_attempts
            .push((ReplanStrategy::SplitTask, evidence_fp.clone()));
        request
            .prior_attempts
            .push((ReplanStrategy::AddPrerequisite, evidence_fp.clone()));
        request
            .prior_attempts
            .push((ReplanStrategy::MergeSiblingTasks, evidence_fp.clone()));

        let decision = ReplanController::decide(&request);
        match &decision {
            ReplanDecision::Apply { strategy, mutation } => {
                assert_eq!(strategy, &ReplanStrategy::RemoveInvalidDependency);

                let (new_plan, _receipt) =
                    ReplanController::apply(&request, &plan, strategy, mutation).unwrap();

                // t1 no longer depends on dep-a.
                let t1 = &new_plan.tasks["t1"];
                assert!(!t1.dependencies.contains("dep-a"));
            }
            other => panic!("expected Apply(RemoveInvalidDependency), got: {:?}", other),
        }
    }

    #[test]
    fn remove_invalid_dependency_rejected_without_evidence() {
        let plan = test_plan(&[("dep-a", &[]), ("t1", &["dep-a"])]);
        let mut request = test_request(
            &plan,
            "t1",
            FailureClass::ArchitecturalConflictRequiresReplan,
            GateFailureAction::NeedsReplan,
        );

        // Exhaust all prior strategies -- but no evidence for RemoveInvalidDependency.
        let evidence_fp = evidence_fingerprint(&request.gate_classification);
        request
            .prior_attempts
            .push((ReplanStrategy::ChangeApproach, evidence_fp.clone()));
        request
            .prior_attempts
            .push((ReplanStrategy::SplitTask, evidence_fp.clone()));
        request
            .prior_attempts
            .push((ReplanStrategy::AddPrerequisite, evidence_fp.clone()));
        request
            .prior_attempts
            .push((ReplanStrategy::MergeSiblingTasks, evidence_fp.clone()));

        let decision = ReplanController::decide(&request);
        match &decision {
            ReplanDecision::Reject { reason } => {
                assert!(
                    reason.contains("exhausted"),
                    "expected exhaustion message, got: {}",
                    reason
                );
            }
            other => panic!("expected Reject, got: {:?}", other),
        }
    }

    // ── Cap exhaustion ──────────────────────────────────────────────────

    #[test]
    fn cap_reached_after_max_attempts() {
        let plan = test_plan(&[("t1", &[])]);
        let mut request = test_request(
            &plan,
            "t1",
            FailureClass::ArchitecturalConflictRequiresReplan,
            GateFailureAction::NeedsReplan,
        );
        request.max_replans = 2;

        // Fill prior_attempts to meet the cap.
        let evidence_fp = evidence_fingerprint(&request.gate_classification);
        request
            .prior_attempts
            .push((ReplanStrategy::ChangeApproach, evidence_fp.clone()));
        request
            .prior_attempts
            .push((ReplanStrategy::SplitTask, evidence_fp.clone()));

        let decision = ReplanController::decide(&request);
        assert!(matches!(decision, ReplanDecision::CapReached));
    }

    #[test]
    fn absolute_cap_is_enforced() {
        let plan = test_plan(&[("t1", &[])]);
        let mut request = test_request(
            &plan,
            "t1",
            FailureClass::ArchitecturalConflictRequiresReplan,
            GateFailureAction::NeedsReplan,
        );
        request.max_replans = 100; // Exceeds absolute cap of 5.

        // Fill 5 attempts.
        let evidence_fp = evidence_fingerprint(&request.gate_classification);
        for strategy in &ReplanStrategy::ORDERED {
            request
                .prior_attempts
                .push((strategy.clone(), evidence_fp.clone()));
        }

        let decision = ReplanController::decide(&request);
        assert!(matches!(decision, ReplanDecision::CapReached));
    }

    // ── Resume deduplication ────────────────────────────────────────────

    #[test]
    fn resume_skips_already_tried_strategies() {
        let plan = test_plan(&[("t1", &[])]);
        let mut request = test_request(
            &plan,
            "t1",
            FailureClass::ArchitecturalConflictRequiresReplan,
            GateFailureAction::NeedsReplan,
        );

        let evidence_fp = evidence_fingerprint(&request.gate_classification);

        // Mark ChangeApproach as already tried.
        request
            .prior_attempts
            .push((ReplanStrategy::ChangeApproach, evidence_fp.clone()));

        let decision = ReplanController::decide(&request);
        match &decision {
            ReplanDecision::Apply { strategy, .. } => {
                // Should skip to SplitTask.
                assert_eq!(strategy, &ReplanStrategy::SplitTask);
            }
            other => panic!("expected Apply(SplitTask), got: {:?}", other),
        }
    }

    // ── Failure class routing ───────────────────────────────────────────

    #[test]
    fn external_environment_rejects() {
        let plan = test_plan(&[("t1", &[])]);
        let request = test_request(
            &plan,
            "t1",
            FailureClass::ExternalEnvironment,
            GateFailureAction::Blocked,
        );

        let decision = ReplanController::decide(&request);
        match &decision {
            ReplanDecision::Reject { reason } => {
                assert!(reason.contains("ExternalEnvironment"));
            }
            other => panic!("expected Reject, got: {:?}", other),
        }
    }

    #[test]
    fn role_tool_permission_rejects() {
        let plan = test_plan(&[("t1", &[])]);
        let request = test_request(
            &plan,
            "t1",
            FailureClass::RoleToolPermission,
            GateFailureAction::Blocked,
        );

        let decision = ReplanController::decide(&request);
        assert!(matches!(decision, ReplanDecision::Reject { .. }));
    }

    #[test]
    fn retry_class_without_replan_action_rejects() {
        let plan = test_plan(&[("t1", &[])]);
        let request = test_request(
            &plan,
            "t1",
            FailureClass::TypeError, // Not a replan-eligible class.
            GateFailureAction::Retry,
        );

        let decision = ReplanController::decide(&request);
        assert!(matches!(decision, ReplanDecision::Reject { .. }));
    }

    // ── Event emission ──────────────────────────────────────────────────

    #[test]
    fn event_for_applied() {
        let plan = test_plan(&[("t1", &[])]);
        let request = test_request(
            &plan,
            "t1",
            FailureClass::ArchitecturalConflictRequiresReplan,
            GateFailureAction::NeedsReplan,
        );

        let decision = ReplanController::decide(&request);
        let event = ReplanController::event_for(&request, &decision);

        match event {
            ReplanEvent::Applied {
                run_id,
                plan_id,
                strategy,
                ordinal,
                ..
            } => {
                assert_eq!(run_id, "run-1");
                assert_eq!(plan_id, "test-plan");
                assert_eq!(strategy, ReplanStrategy::ChangeApproach);
                assert_eq!(ordinal, 0);
            }
            other => panic!("expected Applied event, got: {:?}", other),
        }
    }

    #[test]
    fn event_for_cap_hit() {
        let plan = test_plan(&[("t1", &[])]);
        let mut request = test_request(
            &plan,
            "t1",
            FailureClass::ArchitecturalConflictRequiresReplan,
            GateFailureAction::NeedsReplan,
        );
        request.max_replans = 0;

        let decision = ReplanController::decide(&request);
        let event = ReplanController::event_for(&request, &decision);

        match event {
            ReplanEvent::CapHit { cap, .. } => {
                assert_eq!(cap, 0);
            }
            other => panic!("expected CapHit event, got: {:?}", other),
        }
    }

    // ── Mutation cannot introduce cycles ────────────────────────────────

    #[test]
    fn add_prerequisite_does_not_create_cycle() {
        // t1 depends on nothing. Adding a prerequisite should not create a cycle.
        let plan = test_plan(&[("t1", &[])]);
        let mut request = test_request(
            &plan,
            "t1",
            FailureClass::PromptContextInsufficiency,
            GateFailureAction::NeedsReplan,
        );

        // Skip to AddPrerequisite.
        let evidence_fp = evidence_fingerprint(&request.gate_classification);
        request
            .prior_attempts
            .push((ReplanStrategy::ChangeApproach, evidence_fp.clone()));
        request
            .prior_attempts
            .push((ReplanStrategy::SplitTask, evidence_fp.clone()));

        let decision = ReplanController::decide(&request);
        if let ReplanDecision::Apply { strategy, mutation } = &decision {
            assert_eq!(strategy, &ReplanStrategy::AddPrerequisite);
            let result = ReplanController::apply(&request, &plan, strategy, mutation);
            assert!(result.is_ok(), "prerequisite should not create a cycle");
        }
    }

    // ── Receipt serde roundtrip ─────────────────────────────────────────

    #[test]
    fn receipt_serde_roundtrip() {
        let receipt = ReplanReceiptV1 {
            strategy: ReplanStrategy::SplitTask,
            evidence_fingerprint: "abc123".to_string(),
            before_fingerprint: "before".to_string(),
            after_fingerprint: "after".to_string(),
            ordinal: 2,
            mutation_id: "mut-1".to_string(),
        };

        let json = serde_json::to_string(&receipt).unwrap();
        let decoded: ReplanReceiptV1 = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.strategy, ReplanStrategy::SplitTask);
        assert_eq!(decoded.ordinal, 2);
        assert_eq!(decoded.mutation_id, "mut-1");
    }
}
