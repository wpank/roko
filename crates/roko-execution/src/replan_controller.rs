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
use roko_graph::plan_mutation::{build_merge_with_rewiring, build_split_with_rewiring};
use roko_graph::snapshot::{CheckpointExtension, EXT_REPLAN};
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
// Checkpoint state (durable extension `roko.replan@1`)
// ---------------------------------------------------------------------------

/// Durable checkpoint state for replan history.
///
/// Stored as the `roko.replan@1` extension in the snapshot ledger. On resume,
/// this state is read to reconstruct `prior_attempts` and ordinal so the
/// controller never repeats a (strategy, evidence_fingerprint) pair.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ReplanCheckpointState {
    /// All receipts produced so far, in ordinal order.
    pub receipts: Vec<ReplanReceiptV1>,
    /// Current generation (next ordinal to assign).
    pub generation: u32,
    /// Whether the replan cap has been reached for this run.
    pub cap_reached: bool,
}

impl ReplanCheckpointState {
    /// Create a new empty checkpoint state.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Record an applied replan receipt.
    pub fn record(&mut self, receipt: ReplanReceiptV1) {
        self.generation = receipt.ordinal + 1;
        self.receipts.push(receipt);
    }

    /// Mark that the replan cap has been reached.
    pub fn mark_cap_reached(&mut self) {
        self.cap_reached = true;
    }

    /// Reconstruct `prior_attempts` from the checkpoint for feeding into a
    /// [`ReplanRequest`].
    #[must_use]
    pub fn prior_attempts(&self) -> Vec<(ReplanStrategy, String)> {
        self.receipts
            .iter()
            .map(|r| (r.strategy.clone(), r.evidence_fingerprint.clone()))
            .collect()
    }

    /// Build a [`CheckpointExtension`] for storage in the snapshot extension ledger.
    ///
    /// The extension uses namespace `roko.replan@1` (required). The fingerprint
    /// is a BLAKE3 hash of the canonical JSON so idempotent re-registration
    /// with the same content succeeds.
    #[must_use]
    pub fn to_extension(&self) -> CheckpointExtension {
        let value = serde_json::to_value(self)
            .unwrap_or_else(|_| serde_json::json!({"error": "serialization_failed"}));
        let fingerprint =
            blake3::hash(serde_json::to_string(&value).unwrap_or_default().as_bytes());

        // Parse the namespace and version from the constant.
        let parts: Vec<&str> = EXT_REPLAN.split('@').collect();
        let namespace = parts[0].to_string();
        let schema_version: u32 = parts.get(1).and_then(|v| v.parse().ok()).unwrap_or(1);

        CheckpointExtension {
            namespace,
            schema_version,
            required: true,
            fingerprint: fingerprint.to_hex()[..16].to_string(),
            value,
        }
    }

    /// Restore checkpoint state from a [`CheckpointExtension`] value.
    ///
    /// Returns `None` if deserialization fails, allowing the caller to start
    /// fresh (which is safe because the controller is conservative -- starting
    /// fresh just means it may re-try strategies that already failed, which
    /// the plan mutation layer will reject if the plan fingerprint changed).
    #[must_use]
    pub fn from_extension_value(value: &serde_json::Value) -> Option<Self> {
        serde_json::from_value(value.clone()).ok()
    }
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
    ///
    /// Without the plan, no dependency edges can be rewired: split parts start
    /// with no dependencies and merges are unavailable. Use
    /// [`Self::decide_with_plan`] when the plan is known.
    #[must_use]
    pub fn decide(request: &ReplanRequest) -> ReplanDecision {
        Self::decide_inner(request, None)
    }

    fn decide_inner(request: &ReplanRequest, plan: Option<&MutablePlanV1>) -> ReplanDecision {
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
            match build_mutation(request, strategy, &evidence_fp, plan) {
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

    /// Full decide-apply-checkpoint cycle.
    ///
    /// Given a gate failure, checkpoint state, and the current plan:
    /// 1. Builds a `ReplanRequest` from the checkpoint's prior attempts.
    /// 2. Calls `decide_with_plan` to find an applicable strategy.
    /// 3. On `Apply`, applies the mutation and records the receipt.
    /// 4. Returns the decision, the mutated plan (if any), and updated checkpoint.
    ///
    /// The caller is responsible for persisting the checkpoint state to the
    /// extension ledger after a successful apply.
    pub fn decide_apply_checkpoint(
        run_id: &str,
        plan: &MutablePlanV1,
        failed_task_id: &str,
        gate_classification: GateFailureClassification,
        completed_task_ids: &[String],
        max_replans: u32,
        checkpoint: &mut ReplanCheckpointState,
    ) -> Result<(ReplanDecision, Option<MutablePlanV1>), PlanMutationErrorV1> {
        if checkpoint.cap_reached {
            return Ok((ReplanDecision::CapReached, None));
        }

        let request = ReplanRequest {
            run_id: run_id.to_string(),
            plan_id: plan.plan_id.clone(),
            failed_task_id: failed_task_id.to_string(),
            gate_classification,
            plan_fingerprint: canonical_fingerprint(plan),
            completed_task_ids: completed_task_ids.to_vec(),
            prior_attempts: checkpoint.prior_attempts(),
            max_replans,
        };

        let decision = Self::decide_with_plan(&request, plan);

        match &decision {
            ReplanDecision::Apply { strategy, mutation } => {
                let (new_plan, receipt) = Self::apply(&request, plan, strategy, mutation)?;
                checkpoint.record(receipt);
                Ok((decision, Some(new_plan)))
            }
            ReplanDecision::CapReached => {
                checkpoint.mark_cap_reached();
                Ok((decision, None))
            }
            ReplanDecision::Reject { .. } => Ok((decision, None)),
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
    plan: Option<&MutablePlanV1>,
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
        ReplanStrategy::ChangeApproach => build_change_approach_ops(request, plan)?,
        ReplanStrategy::SplitTask => build_split_task_ops(request, plan)?,
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
fn build_change_approach_ops(
    request: &ReplanRequest,
    plan: Option<&MutablePlanV1>,
) -> Result<Vec<PlanMutationOpV1>, String> {
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
        // Keep the task's place in the DAG when the plan is known.
        dependencies: plan
            .and_then(|plan| plan.tasks.get(task_id))
            .map(|task| task.dependencies.clone())
            .unwrap_or_default(),
        metadata,
        completed: false,
    };

    Ok(vec![PlanMutationOpV1::ReplaceTask {
        task_id: task_id.clone(),
        replacement,
    }])
}

/// SplitTask: replace the failed task with `<id>-part-1` and `<id>-part-2`.
///
/// With the plan, part 1 inherits the failed task's dependencies and its
/// dependents move to part 2.
fn build_split_task_ops(
    request: &ReplanRequest,
    plan: Option<&MutablePlanV1>,
) -> Result<Vec<PlanMutationOpV1>, String> {
    let task_id = &request.failed_task_id;
    let summary = &request.gate_classification.summary;

    let part1_title = format!("[split 1/2] {}", task_id);
    let part1_description = format!(
        "First part of split task after gate failure: {}\n\n\
         Focus on the foundational/setup work.",
        summary
    );
    let part2_title = format!("[split 2/2] {}", task_id);
    let part2_description = format!(
        "Second part of split task after gate failure: {}\n\n\
         Build on the foundation from part 1.",
        summary
    );

    if let Some(plan) = plan {
        return build_split_with_rewiring(
            plan,
            task_id,
            &part1_title,
            &part1_description,
            &part2_title,
            &part2_description,
        )
        .map(|(operations, _, _)| operations)
        .ok_or_else(|| format!("task '{task_id}' is not pending in the plan"));
    }

    let part1_id = format!("{}-part-1", task_id);
    let part2_id = format!("{}-part-2", task_id);

    let part1 = MutableTaskV1 {
        id: part1_id.clone(),
        title: part1_title,
        description: part1_description,
        dependencies: BTreeSet::new(),
        metadata: BTreeMap::from([
            ("split_source".to_string(), task_id.clone()),
            ("split_ordinal".to_string(), "1".to_string()),
        ]),
        completed: false,
    };

    // Part 2 depends on part 1.
    let part2 = MutableTaskV1 {
        id: part2_id,
        title: part2_title,
        description: part2_description,
        dependencies: BTreeSet::from([part1_id]),
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
        // First try the normal path, wiring edges from the plan topology.
        let decision = Self::decide_inner(request, Some(plan));

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

        // Build merge mutation; dependents of either source move to the merged task.
        let Some(operations) = build_merge_with_rewiring(
            plan,
            &request.failed_task_id,
            sibling_id,
            &request.failed_task_id,
            &format!("[merged] {} + {}", request.failed_task_id, sibling_id),
            &format!(
                "Merged task after gate failure.\n\n\
                 Original: {}\n\
                 Merged with: {}\n\n\
                 Failure: {}",
                failed_task.description, sibling.description, request.gate_classification.summary,
            ),
        ) else {
            return decision;
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
            operations,
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

        // t2 sits between t1 and t3, so the split needs the plan to rewire.
        let decision = ReplanController::decide_with_plan(&request, &plan);
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

                // Part 1 inherits t2's dependencies; part 2 depends on part 1.
                let part1 = &new_plan.tasks["t2-part-1"];
                assert!(part1.dependencies.contains("t1"));
                let part2 = &new_plan.tasks["t2-part-2"];
                assert!(part2.dependencies.contains("t2-part-1"));
                // t2's dependents now wait for the whole split.
                assert_eq!(
                    new_plan.tasks["t3"].dependencies,
                    BTreeSet::from(["t2-part-2".to_string()])
                );
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
                // t3's edge to t2 moves to the merged task.
                assert_eq!(
                    new_plan.tasks["t3"].dependencies,
                    BTreeSet::from(["t1".to_string()])
                );
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

    // ── Checkpoint state ──────────────────────────────────────────────

    #[test]
    fn checkpoint_state_new_is_empty() {
        let state = ReplanCheckpointState::new();
        assert!(state.receipts.is_empty());
        assert_eq!(state.generation, 0);
        assert!(!state.cap_reached);
        assert!(state.prior_attempts().is_empty());
    }

    #[test]
    fn checkpoint_state_records_receipt() {
        let mut state = ReplanCheckpointState::new();

        let receipt = ReplanReceiptV1 {
            strategy: ReplanStrategy::ChangeApproach,
            evidence_fingerprint: "fp1".to_string(),
            before_fingerprint: "before".to_string(),
            after_fingerprint: "after".to_string(),
            ordinal: 0,
            mutation_id: "mut-0".to_string(),
        };

        state.record(receipt);

        assert_eq!(state.receipts.len(), 1);
        assert_eq!(state.generation, 1);
        assert!(!state.cap_reached);

        let attempts = state.prior_attempts();
        assert_eq!(attempts.len(), 1);
        assert_eq!(attempts[0].0, ReplanStrategy::ChangeApproach);
        assert_eq!(attempts[0].1, "fp1");
    }

    #[test]
    fn checkpoint_state_mark_cap_reached() {
        let mut state = ReplanCheckpointState::new();
        assert!(!state.cap_reached);
        state.mark_cap_reached();
        assert!(state.cap_reached);
    }

    #[test]
    fn checkpoint_extension_roundtrip() {
        let mut state = ReplanCheckpointState::new();
        state.record(ReplanReceiptV1 {
            strategy: ReplanStrategy::SplitTask,
            evidence_fingerprint: "fp-split".to_string(),
            before_fingerprint: "before-split".to_string(),
            after_fingerprint: "after-split".to_string(),
            ordinal: 0,
            mutation_id: "mut-split".to_string(),
        });
        state.record(ReplanReceiptV1 {
            strategy: ReplanStrategy::AddPrerequisite,
            evidence_fingerprint: "fp-prereq".to_string(),
            before_fingerprint: "before-prereq".to_string(),
            after_fingerprint: "after-prereq".to_string(),
            ordinal: 1,
            mutation_id: "mut-prereq".to_string(),
        });

        let ext = state.to_extension();

        // Verify extension metadata.
        assert_eq!(ext.namespace, "roko.replan");
        assert_eq!(ext.schema_version, 1);
        assert!(ext.required);
        assert!(!ext.fingerprint.is_empty());

        // Roundtrip via JSON (simulating snapshot persistence).
        let restored = ReplanCheckpointState::from_extension_value(&ext.value);
        assert!(restored.is_some(), "should deserialize successfully");

        let restored = restored.unwrap();
        assert_eq!(restored.receipts.len(), 2);
        assert_eq!(restored.generation, 2);
        assert!(!restored.cap_reached);

        let attempts = restored.prior_attempts();
        assert_eq!(attempts.len(), 2);
        assert_eq!(attempts[0].0, ReplanStrategy::SplitTask);
        assert_eq!(attempts[1].0, ReplanStrategy::AddPrerequisite);
    }

    #[test]
    fn checkpoint_extension_idempotent_fingerprint() {
        let mut state = ReplanCheckpointState::new();
        state.record(ReplanReceiptV1 {
            strategy: ReplanStrategy::ChangeApproach,
            evidence_fingerprint: "fp1".to_string(),
            before_fingerprint: "b".to_string(),
            after_fingerprint: "a".to_string(),
            ordinal: 0,
            mutation_id: "m".to_string(),
        });

        let ext1 = state.to_extension();
        let ext2 = state.to_extension();

        // Same state should produce the same fingerprint.
        assert_eq!(ext1.fingerprint, ext2.fingerprint);
    }

    #[test]
    fn checkpoint_extension_different_state_different_fingerprint() {
        let mut state1 = ReplanCheckpointState::new();
        state1.record(ReplanReceiptV1 {
            strategy: ReplanStrategy::ChangeApproach,
            evidence_fingerprint: "fp1".to_string(),
            before_fingerprint: "b".to_string(),
            after_fingerprint: "a".to_string(),
            ordinal: 0,
            mutation_id: "m1".to_string(),
        });

        let mut state2 = ReplanCheckpointState::new();
        state2.record(ReplanReceiptV1 {
            strategy: ReplanStrategy::SplitTask,
            evidence_fingerprint: "fp2".to_string(),
            before_fingerprint: "b2".to_string(),
            after_fingerprint: "a2".to_string(),
            ordinal: 0,
            mutation_id: "m2".to_string(),
        });

        let ext1 = state1.to_extension();
        let ext2 = state2.to_extension();

        assert_ne!(ext1.fingerprint, ext2.fingerprint);
    }

    #[test]
    fn checkpoint_from_invalid_value_returns_none() {
        let invalid = serde_json::json!({"not": "a checkpoint"});
        assert!(ReplanCheckpointState::from_extension_value(&invalid).is_none());
    }

    #[test]
    fn checkpoint_cap_reached_roundtrips() {
        let mut state = ReplanCheckpointState::new();
        state.mark_cap_reached();

        let ext = state.to_extension();
        let restored = ReplanCheckpointState::from_extension_value(&ext.value).unwrap();
        assert!(restored.cap_reached);
    }

    // ── Checkpoint-driven decide-apply cycle ──────────────────────────

    #[test]
    fn decide_apply_checkpoint_cycle() {
        let plan = test_plan(&[("t1", &[]), ("t2", &["t1"])]);
        let mut checkpoint = ReplanCheckpointState::new();
        let classification = test_classification(
            FailureClass::ArchitecturalConflictRequiresReplan,
            GateFailureAction::NeedsReplan,
            "test failure for checkpoint cycle",
        );

        // First replan: should pick ChangeApproach.
        let (decision, new_plan) = ReplanController::decide_apply_checkpoint(
            "run-cp",
            &plan,
            "t2",
            classification.clone(),
            &[],
            3,
            &mut checkpoint,
        )
        .unwrap();

        match &decision {
            ReplanDecision::Apply { strategy, .. } => {
                assert_eq!(strategy, &ReplanStrategy::ChangeApproach);
            }
            other => panic!("expected Apply(ChangeApproach), got: {:?}", other),
        }
        assert!(new_plan.is_some());
        assert_eq!(checkpoint.receipts.len(), 1);
        assert_eq!(checkpoint.generation, 1);

        let plan_after_first = new_plan.unwrap();

        // Second replan on the mutated plan: should pick SplitTask.
        let (decision2, new_plan2) = ReplanController::decide_apply_checkpoint(
            "run-cp",
            &plan_after_first,
            "t2",
            classification.clone(),
            &[],
            3,
            &mut checkpoint,
        )
        .unwrap();

        match &decision2 {
            ReplanDecision::Apply { strategy, .. } => {
                assert_eq!(strategy, &ReplanStrategy::SplitTask);
            }
            other => panic!("expected Apply(SplitTask), got: {:?}", other),
        }
        assert!(new_plan2.is_some());
        assert_eq!(checkpoint.receipts.len(), 2);
        assert_eq!(checkpoint.generation, 2);

        let plan_after_second = new_plan2.unwrap();

        // Third replan: the split removed t2, so the same failure now lands
        // on its first part; ChangeApproach and SplitTask are spent for this
        // evidence, so AddPrerequisite is next.
        let (decision3, _) = ReplanController::decide_apply_checkpoint(
            "run-cp",
            &plan_after_second,
            "t2-part-1",
            classification,
            &[],
            3,
            &mut checkpoint,
        )
        .unwrap();

        match &decision3 {
            ReplanDecision::Apply { strategy, .. } => {
                assert_eq!(strategy, &ReplanStrategy::AddPrerequisite);
            }
            other => panic!("expected Apply(AddPrerequisite), got: {:?}", other),
        }
        assert_eq!(checkpoint.receipts.len(), 3);
        assert_eq!(checkpoint.generation, 3);
    }

    #[test]
    fn decide_apply_checkpoint_respects_prior_cap_reached() {
        let plan = test_plan(&[("t1", &[])]);
        let mut checkpoint = ReplanCheckpointState::new();
        checkpoint.mark_cap_reached();

        let classification = test_classification(
            FailureClass::ArchitecturalConflictRequiresReplan,
            GateFailureAction::NeedsReplan,
            "should be blocked by cap",
        );

        let (decision, new_plan) = ReplanController::decide_apply_checkpoint(
            "run-cap",
            &plan,
            "t1",
            classification,
            &[],
            5,
            &mut checkpoint,
        )
        .unwrap();

        assert!(matches!(decision, ReplanDecision::CapReached));
        assert!(new_plan.is_none());
    }

    #[test]
    fn checkpoint_resume_skips_tried_strategies() {
        // Simulate a checkpoint with one receipt from a previous run.
        let mut checkpoint = ReplanCheckpointState::new();
        checkpoint.record(ReplanReceiptV1 {
            strategy: ReplanStrategy::ChangeApproach,
            evidence_fingerprint: "fp-resume".to_string(),
            before_fingerprint: "b".to_string(),
            after_fingerprint: "a".to_string(),
            ordinal: 0,
            mutation_id: "m-resume".to_string(),
        });

        // Persist and restore via extension.
        let ext = checkpoint.to_extension();
        let restored = ReplanCheckpointState::from_extension_value(&ext.value).unwrap();
        let attempts = restored.prior_attempts();

        // Build a request with the restored attempts.
        let plan = test_plan(&[("t1", &[])]);
        let request = ReplanRequest {
            run_id: "run-resume".to_string(),
            plan_id: plan.plan_id.clone(),
            failed_task_id: "t1".to_string(),
            gate_classification: test_classification(
                FailureClass::ArchitecturalConflictRequiresReplan,
                GateFailureAction::NeedsReplan,
                "test failure",
            ),
            plan_fingerprint: canonical_fingerprint(&plan),
            completed_task_ids: vec![],
            prior_attempts: attempts,
            max_replans: 5,
        };

        // The checkpoint has "fp-resume" but the new evidence_fp is computed
        // from the test classification. They differ, so ChangeApproach will
        // be tried again (correct -- different evidence means it hasn't been
        // tried for THIS failure).
        let decision = ReplanController::decide(&request);
        match &decision {
            ReplanDecision::Apply { strategy, .. } => {
                assert_eq!(strategy, &ReplanStrategy::ChangeApproach);
            }
            other => panic!("expected Apply(ChangeApproach), got: {:?}", other),
        }

        // Now test with matching fingerprints.
        let evidence_fp = evidence_fingerprint(&request.gate_classification);
        let mut request2 = request.clone();
        request2.prior_attempts = vec![(ReplanStrategy::ChangeApproach, evidence_fp)];

        let decision2 = ReplanController::decide(&request2);
        match &decision2 {
            ReplanDecision::Apply { strategy, .. } => {
                assert_eq!(strategy, &ReplanStrategy::SplitTask);
            }
            other => panic!("expected Apply(SplitTask), got: {:?}", other),
        }
    }

    #[test]
    fn checkpoint_cap_exhaustion_terminal() {
        let plan = test_plan(&[("t1", &[])]);
        let mut checkpoint = ReplanCheckpointState::new();
        let classification = test_classification(
            FailureClass::ArchitecturalConflictRequiresReplan,
            GateFailureAction::NeedsReplan,
            "cap test",
        );

        // Set max_replans to 1 so we hit the cap after one attempt.
        let (decision, _) = ReplanController::decide_apply_checkpoint(
            "run-cap-test",
            &plan,
            "t1",
            classification.clone(),
            &[],
            1,
            &mut checkpoint,
        )
        .unwrap();

        assert!(matches!(decision, ReplanDecision::Apply { .. }));
        assert_eq!(checkpoint.receipts.len(), 1);

        // Second attempt should hit cap.
        let (decision2, _) = ReplanController::decide_apply_checkpoint(
            "run-cap-test",
            &plan,
            "t1",
            classification,
            &[],
            1,
            &mut checkpoint,
        )
        .unwrap();

        assert!(matches!(decision2, ReplanDecision::CapReached));
        assert!(checkpoint.cap_reached);

        // Verify the cap state persists through checkpoint.
        let ext = checkpoint.to_extension();
        let restored = ReplanCheckpointState::from_extension_value(&ext.value).unwrap();
        assert!(restored.cap_reached);
    }

    #[test]
    fn checkpoint_preserves_completed_task_ids() {
        let mut plan = test_plan(&[("t1", &[]), ("t2", &["t1"]), ("t3", &["t2"])]);
        // Mark t1 as completed.
        plan.tasks.get_mut("t1").unwrap().completed = true;

        let mut checkpoint = ReplanCheckpointState::new();
        let classification = test_classification(
            FailureClass::ArchitecturalConflictRequiresReplan,
            GateFailureAction::NeedsReplan,
            "test with completed tasks",
        );

        let (decision, new_plan) = ReplanController::decide_apply_checkpoint(
            "run-completed",
            &plan,
            "t2",
            classification,
            &["t1".to_string()],
            3,
            &mut checkpoint,
        )
        .unwrap();

        assert!(matches!(decision, ReplanDecision::Apply { .. }));
        let new_plan = new_plan.unwrap();

        // t1 should still be present and completed.
        assert!(new_plan.tasks.contains_key("t1"));
    }

    #[test]
    fn checkpoint_extension_with_register_extension() {
        use roko_graph::snapshot::register_extension;
        use std::collections::BTreeMap as BTreeMapStd;

        let mut state = ReplanCheckpointState::new();
        state.record(ReplanReceiptV1 {
            strategy: ReplanStrategy::ChangeApproach,
            evidence_fingerprint: "fp".to_string(),
            before_fingerprint: "b".to_string(),
            after_fingerprint: "a".to_string(),
            ordinal: 0,
            mutation_id: "m".to_string(),
        });

        let ext = state.to_extension();

        // Register into a BTreeMap the same way the snapshot ledger does.
        let mut extensions = BTreeMapStd::new();
        register_extension(&mut extensions, ext.clone())
            .expect("first registration should succeed");

        // Idempotent re-registration with the same content should succeed.
        register_extension(&mut extensions, ext).expect("idempotent registration should succeed");

        assert_eq!(extensions.len(), 1);

        // The key should be "roko.replan@1".
        assert!(extensions.contains_key("roko.replan@1"));

        // Restore from the ledger entry.
        let ledger_entry = &extensions["roko.replan@1"];
        let restored = ReplanCheckpointState::from_extension_value(&ledger_entry.value).unwrap();
        assert_eq!(restored.receipts.len(), 1);
        assert_eq!(
            restored.receipts[0].strategy,
            ReplanStrategy::ChangeApproach
        );
    }
}
