//! Merge transition proofs (backlog #140): the plan state machine moves a
//! merging plan to Complete on MergeSucceeded, and never on MergeFailed.
//!
//! Graph runs deliver plans through `GitDeliveryBackend`, whose tests live in
//! `graph_execution::delivery`; `PlanMerger` (gap-3505fb) and the merge queue
//! it wrapped (9205) were deleted.

use roko_cli::orchestrator::{ExecutorEvent, PlanState, PlanStateMachine};
use roko_core::PlanPhase;

// ── Executor MergeSucceeded / MergeFailed transitions ────────────────────

#[test]
fn executor_merge_succeeded_transitions_plan() {
    let plan_state = PlanState {
        plan_id: "merge-test-plan".to_string(),
        current_phase: PlanPhase::Merging,
        ..Default::default()
    };

    // MergeSucceeded moves a merging plan to Complete.
    let phase = PlanStateMachine::transition(&plan_state, &ExecutorEvent::MergeSucceeded)
        .expect("MergeSucceeded should be a valid transition from Merging");
    assert_eq!(
        phase,
        PlanPhase::Complete,
        "MergeSucceeded from Merging should yield Complete"
    );
}

#[test]
fn executor_merge_failed_does_not_mark_success() {
    let plan_state = PlanState {
        plan_id: "merge-fail-plan".to_string(),
        current_phase: PlanPhase::Merging,
        ..Default::default()
    };

    // MergeFailed is a legal transition, and never to Complete.
    let phase = PlanStateMachine::transition(&plan_state, &ExecutorEvent::MergeFailed)
        .expect("MergeFailed should be a valid transition (not a panic)");
    assert_ne!(
        phase,
        PlanPhase::Complete,
        "MergeFailed must not produce a Complete phase"
    );
}
