//! Public-contract tests for the Lens overhead circuit breaker.

use roko_core::lens_circuit_breaker::{LensBreakerAction, LensBreakerStage, LensCircuitBreaker};

#[test]
fn lens_circuit_public_api_progresses_through_all_stages_and_recovers() {
    let mut breaker = LensCircuitBreaker::new(0.01);

    assert_eq!(breaker.check(10, 1_000), LensBreakerAction::Allow);
    assert_eq!(breaker.check(11, 1_000), LensBreakerAction::Allow);
    assert_eq!(breaker.check(11, 1_000), LensBreakerAction::Allow);
    assert_eq!(breaker.check(11, 1_000), LensBreakerAction::Skip);
    assert_eq!(breaker.stage(), LensBreakerStage::Sampled);
    assert!(breaker.should_invoke());

    for _ in 3..9 {
        assert_eq!(breaker.check(11, 1_000), LensBreakerAction::Skip);
    }
    assert_eq!(breaker.check(11, 1_000), LensBreakerAction::Disable);
    assert_eq!(breaker.stage(), LensBreakerStage::Disabled);
    assert!(!breaker.should_invoke());
    assert_eq!(breaker.total_violations(), 10);
    assert_eq!(breaker.check(0, 1_000), LensBreakerAction::Disable);

    breaker.reset();
    assert_eq!(breaker.stage(), LensBreakerStage::Sampled);
    assert!(breaker.should_invoke());
    assert_eq!(breaker.check(0, 1_000), LensBreakerAction::Skip);
}

#[test]
fn lens_circuit_passing_check_breaks_a_consecutive_violation_streak() {
    let mut breaker = LensCircuitBreaker::default().with_thresholds(3, 10);

    assert_eq!(breaker.check(11, 1_000), LensBreakerAction::Allow);
    assert_eq!(breaker.check(11, 1_000), LensBreakerAction::Allow);
    assert_eq!(breaker.check(10, 1_000), LensBreakerAction::Allow);
    assert_eq!(breaker.check(11, 1_000), LensBreakerAction::Allow);
    assert_eq!(breaker.check(11, 1_000), LensBreakerAction::Allow);
    assert_eq!(breaker.stage(), LensBreakerStage::Active);
    assert_eq!(breaker.total_violations(), 4);
}

#[test]
fn lens_circuit_zero_duration_cell_has_a_well_defined_budget_boundary() {
    let mut breaker = LensCircuitBreaker::default().with_thresholds(1, 2);

    assert_eq!(breaker.check(0, 0), LensBreakerAction::Allow);
    assert_eq!(breaker.check(1, 0), LensBreakerAction::Skip);
    assert_eq!(breaker.check(1, 0), LensBreakerAction::Disable);
}
