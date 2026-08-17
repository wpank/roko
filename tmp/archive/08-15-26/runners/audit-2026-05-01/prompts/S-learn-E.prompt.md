# S-learn-E: Cascade router stage progression integration test

## Task
Add an integration test that drives the cascade router from `LearningStageKind::ConfidenceOnly` to `Contextual` after threshold observations with full `RoutingContext`. Catches regressions where T4-30's plumbing breaks the contextual update path.

## Runner Context
Runner audit-2026-05-01, group S. Depends on T4-30 + S-learn-A. Wave 3.

## Source plan
`tmp/subsystem-audits/implementation-plans/25-learning-feedback-completion.md` § Phase F.

## Exact changes

`crates/roko-learn/tests/cascade_progression.rs` (new):

```rust
use roko_learn::cascade_router::{CascadeRouter, LearningStageKind, RoutingContext, RoutingOutcome};

#[tokio::test]
async fn router_progresses_to_contextual_after_threshold() {
    let r = CascadeRouter::new(vec!["model-a".into(), "model-b".into()]);
    assert_eq!(r.learning_stage().stage, LearningStageKind::ConfidenceOnly);

    // Feed N contextual observations.
    for i in 0..60 {
        let ctx = RoutingContext {
            model: "model-a".into(),
            // Fill realistic feature values
            ..Default::default()
        };
        r.observe_multi_objective(&ctx, RoutingOutcome {
            success: i % 3 != 0,
            // ...
        }).await;
    }

    let stage = r.learning_stage();
    assert_eq!(stage.stage, LearningStageKind::Contextual,
        "expected Contextual after {} observations; got {:?}",
        stage.observations, stage.stage,
    );
}

#[tokio::test]
async fn confidence_only_observations_do_not_trigger_contextual() {
    let r = CascadeRouter::new(vec!["model-a".into()]);
    for _ in 0..100 {
        r.record_confidence_outcome("model-a", true);
    }
    // Confidence-only updates should not advance to Contextual.
    assert_eq!(r.learning_stage().stage, LearningStageKind::ConfidenceOnly);
}
```

If `RoutingContext::default()` doesn't expose enough fields for a meaningful test, build a `test_context()` helper.

## Write Scope
- `crates/roko-learn/tests/cascade_progression.rs` (new)
- `crates/roko-learn/src/cascade_router.rs` (only if missing test helpers)

## Verify

```bash
ls crates/roko-learn/tests/cascade_progression.rs

rg 'router_progresses_to_contextual_after_threshold' crates/roko-learn/tests/
# Expect: 1 hit
```

## Do NOT

- Do NOT skip the second test (confidence-only). It catches the inverse regression.
- Do NOT bundle with other S-learn batches.
- Do NOT change the router internals just to pass the test.
- Do NOT use `#[ignore]`.
