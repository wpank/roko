---
title: "Verification × Orchestration"
section: analysis
subsection: integration-map
id: im-verification-x-orchestration
source: 24-cross-section-integration-map.md (§6.2 M3, §3.1)
missing-integration: M3
tier: 1
tags: [verification, orchestration, failure, replanning, gate-verdict, feedback-loop]
---

# Verification × Orchestration

**Direction**: 04-Verification → 01-Orchestration (bidirectional — verdict flows downstream, plan drives verification)  
**Status**: **Partially Wired** — `GateVerdict` flows to orchestrator for pass/fail decisions; **Missing**: N-consecutive-failure triggers replanning (M3 gap)  
**Interface**: `roko-gate::GateVerdict` → `roko-orchestrator::PlanRunner` retry/replan logic

## What Flows

| Signal | From | To | Status |
|---|---|---|---|
| `Kind::GateVerdict` Engrams | `roko-gate` | `roko-orchestrator` | **Wired** — pass/fail used in task retry |
| Plan DAG | `roko-orchestrator` | `roko-gate` (via agent execution) | **Wired** |
| N-consecutive-failure signal | `roko-gate` failure accumulator | `roko-orchestrator` replanning trigger | **Missing** (M3) |
| Failure pattern analysis | (new code) | Plan generator redecomposition | **Missing** |

## Wiring Recipe (M3 Gap)

```rust
// In roko-orchestrator's task runner:
// Count consecutive gate failures for a given task
if consecutive_failures >= gates.max_iterations {
    // Analyze failure patterns
    let pattern = failure_analyzer.summarize(&recent_verdicts);
    
    // Emit replanning event
    let replan_signal = Signal::builder()
        .kind(Kind::ReplanRequest)
        .body(Body::Json(json!({
            "task_id": task.id,
            "failure_count": consecutive_failures,
            "pattern": pattern,
        })))
        .build();
    
    // Feed to orchestrator's plan regeneration
    plan_runner.request_replan(&replan_signal).await?;
}
```

See also: 05-learning/13-8-missing-feedback-loops.md, Loop 4 for full wiring recipe (~80 LOC total).

**Key enhancement beyond Loop 4**: The replanning signal should also feed into Daimon (M11), creating a bidirectional loop: repeated failures lower Pleasure and Dominance, which causes the Daimon to shift to `Struggling` state, which causes more cautious model selection via the CascadeRouter.

## Invariants of the Interaction

1. Pass verdict → orchestrator marks task complete; no side effects.
2. Fail verdict → orchestrator increments failure counter; retries if within `gates.max_iterations`.
3. `gates.max_iterations` exhausted → **M3**: replanning trigger fires; task counter resets.
4. Replanning must not create an infinite loop — replanning depth is bounded by a separate counter.
5. The analysis of failure patterns must be bounded in compute (no full scan of all episodes).

## Failure Modes

| Failure | Consequence | Detection |
|---|---|---|
| Failure counter not persisted | Counter resets on restart → infinite retries on same task | Persist counter to `.roko/orchestrator/` |
| Replan loop (replan → same failure → replan) | Infinite thrashing | Max-replan-depth counter; emit alert after N replans |
| Failure analysis OOM | Crash in pattern analysis | Bound recent_verdicts window (last N verdicts only) |
| Gate verdict not emitted (gate panics) | Failure not counted | Default to failure on any non-success gate outcome |

## Observed Metrics

Currently observable:
- Gate pass rate per task type (visible in `roko dashboard`)
- Number of retries per task (visible in orchestrator logs)

Expected after M3 wiring:
- Replan frequency per plan
- Failure pattern categories (timeout vs quality vs safety vs...)
- Time-to-resolution comparison: retry-only vs replan strategy

## Open Questions

1. What constitutes a "different" replan? Should it require a different task decomposition, or just a different model/agent assignment?
2. Should failure patterns be stored as `Kind::Warning` Engrams in NeuroStore for future use?
3. How does M3 interact with M11 (Orchestration→Daimon)? Should they be implemented together?

## Cross-References

- Complement: [orchestration-x-daimon.md](./orchestration-x-daimon.md) — M11 (failure→Daimon PAD update)
- Learning side: [learning-x-verification.md](./learning-x-verification.md) — verdict Engrams also flow to Learning (wired)
- Finding: [AA-08 Proposal 2](../architectural-analysis/08-novel-proposals.md) — gradient gate feedback (continuous learning signal)
- Readiness audit: [RA-04: Verification](../readiness-audit/subsystem-verification.md), [RA-01: Orchestration](../readiness-audit/subsystem-orchestration.md)
