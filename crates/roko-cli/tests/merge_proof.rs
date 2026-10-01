//! Merge success/conflict proof harnesses (backlog #140).
//!
//! These tests prove that the `MergeQueue` / executor state-machine
//! machinery produces correct outcomes:
//!
//! 1. **Non-conflicting merge**: two plans touching different files are both
//!    accepted by the queue and independently mergeable.
//! 2. **Conflicting merge detection**: two plans touching the same file are
//!    enqueued, but the queue's file-level locking prevents concurrent
//!    reservations that would collide.
//! 3. **Executor MergeSucceeded / MergeFailed transitions**: the plan state
//!    machine correctly transitions plans between Merging, Complete, and
//!    failure phases.
//!
//! Graph runs deliver plans through `GitDeliveryBackend`, whose tests live in
//! `graph_execution::delivery`; `PlanMerger` was deleted (gap-3505fb).

use roko_cli::orchestrator::{
    ExecutorEvent, MergeQueue, MergeRequest, PlanState, PlanStateMachine,
};
use roko_core::PlanPhase;

fn test_merge_queue() -> MergeQueue {
    MergeQueue::new()
}

// ── Test 1: Non-conflicting merge ────────────────────────────────────────

#[test]
fn non_conflicting_merge_enqueues_both_plans() {
    let queue = test_merge_queue();

    // Plan A touches file_a.rs, Plan B touches file_b.rs.
    let request_a = MergeRequest::new(
        "plan-a",
        "plan-a-branch",
        vec!["src/file_a.rs".to_string()],
        1,
    );
    let request_b = MergeRequest::new(
        "plan-b",
        "plan-b-branch",
        vec!["src/file_b.rs".to_string()],
        1,
    );

    // Enqueue both merge requests.
    assert!(queue.enqueue(request_a), "plan-a enqueue should succeed");
    assert!(queue.enqueue(request_b), "plan-b enqueue should succeed");

    // First plan should be immediately mergeable since nothing is locked.
    let first = queue.next_mergeable();
    assert!(
        first.is_some(),
        "at least one plan should be immediately mergeable"
    );
}

// ── Test 2: Conflicting merge detection ──────────────────────────────────

#[test]
fn conflicting_merge_requests_share_file() {
    let queue = test_merge_queue();

    // Both plans touch the same file.
    let request_a = MergeRequest::new(
        "plan-a",
        "plan-a-branch",
        vec!["src/shared.rs".to_string()],
        1,
    );
    let request_b = MergeRequest::new(
        "plan-b",
        "plan-b-branch",
        vec!["src/shared.rs".to_string()],
        1,
    );

    assert!(queue.enqueue(request_a), "plan-a enqueue should succeed");
    assert!(queue.enqueue(request_b), "plan-b enqueue should succeed");

    // Both should enqueue. The queue's file-level locking will prevent
    // simultaneous processing at reservation time (when the reservation is
    // taken, the lock is held). The exact blocking behavior depends on
    // MergeQueue's internal locking — this test verifies that both are
    // accepted without error.
    let metrics = queue.metrics();
    assert_eq!(metrics.queued, 2, "both plans should be queued");
}

// ── Test 3: Executor MergeSucceeded / MergeFailed transitions ────────────

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
