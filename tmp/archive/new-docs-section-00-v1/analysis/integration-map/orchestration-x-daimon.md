---
title: "Orchestration × Daimon"
section: analysis
subsection: integration-map
id: im-orchestration-x-daimon
source: 24-cross-section-integration-map.md (§6.1 M11)
missing-integration: M11
tier: 2
tags: [orchestration, daimon, PAD, task-outcomes, affect-update, emotional-feedback]
---

# Orchestration × Daimon

**Direction**: 01-Orchestration → 09-Daimon (task outcomes update affect state)  
**Status**: **Missing (M11)** — Tier 2, ~40 LOC. Depends on M1 being implemented first.  
**Interface**: `roko-orchestrator` task completion events → `roko-daimon::AffectState` PAD update

## What Flows

The orchestrator knows whether tasks succeed or fail, their cost, and their latency. These outcomes should update the Daimon's PAD vector — success increases Pleasure and Dominance; repeated failure decreases both, triggering behavioral state changes.

| Signal | From | To | Status |
|---|---|---|---|
| Task completion outcome (success/fail, cost, latency) | `PlanRunner` | `roko-daimon` PAD update | **Missing** (M11) |
| Repeated failure signal | Orchestrator failure accumulator | Daimon `Struggling` state | **Missing** |
| Task complexity estimate | Orchestrator | Dominance axis update | **Missing** |

## Wiring Recipe

```rust
// In orchestrate.rs, after task completion:
let outcome = TaskOutcome {
    success: task_result.is_ok(),
    cost_usd: task_result.cost(),
    latency_ms: elapsed.as_millis() as u64,
    complexity: task.complexity_estimate,
};

// Update Daimon PAD based on outcome
daimon.update_from_task_outcome(&outcome);

// In roko-daimon::update_from_task_outcome():
fn update_from_task_outcome(&mut self, outcome: &TaskOutcome) {
    let delta_pleasure = if outcome.success { 0.05 } else { -0.1 };
    let delta_dominance = if outcome.success { 0.03 } else { -0.07 };
    let delta_arousal = if outcome.latency_ms > 30_000 { 0.05 } else { -0.02 };
    
    self.pad.pleasure = (self.pad.pleasure + delta_pleasure).clamp(-1.0, 1.0);
    self.pad.dominance = (self.pad.dominance + delta_dominance).clamp(-1.0, 1.0);
    self.pad.arousal = (self.pad.arousal + delta_arousal).clamp(-1.0, 1.0);
    self.update_behavioral_state();
}
```

Estimated LOC: ~40.

**Dependency**: M11 should be implemented after M1 (Daimon→Orchestration) to complete the bidirectional loop.

## Invariants of the Interaction

1. PAD updates are bounded: each update is a small delta (≤ 0.1 per axis per task), preventing dramatic swings.
2. The Daimon's behavioral state is recomputed after each PAD update.
3. Orchestrator calls `daimon.update_from_task_outcome()` asynchronously — it must not block the next task dispatch.
4. Task cost and latency are included to enable affect modulation of resource usage.

## Failure Modes

| Failure | Consequence | Detection |
|---|---|---|
| Daimon unavailable | PAD not updated; behavioral state stagnates | Log warn; continue |
| PAD drift (continuous failures) | Agent stuck in `Struggling` state indefinitely | Baseline recovery: slowly trend toward neutral when idle |
| High-cost tasks always trigger negative affect | Distorted behavior for legitimate expensive tasks | Weight affect update by task importance, not just cost |

## Observed Metrics

Expected after implementation:
- PAD vector time series per session
- Behavioral state distribution (% time in each state)
- Correlation between task outcome stream and behavioral state transitions

## Open Questions

1. Should task complexity feed into the Dominance update (harder task succeeded → larger Dominance gain)?
2. Is there a need for a "mood baseline recovery" mechanism — PAD trending toward neutral when no tasks are running?
3. Should M3 (Failure→Replanning) and M11 be implemented as a compound "failure cascade" rather than two separate integrations?

## Cross-References

- Reverse direction: [daimon-x-orchestration.md](./daimon-x-orchestration.md) — M1 (prerequisite)
- Affect context: [daimon-x-composition.md](./daimon-x-composition.md) — M2 (PAD also modulates composition)
- Failure cascade: [verification-x-orchestration.md](./verification-x-orchestration.md) — M3 (failures should also feed M11)
- Readiness audit: [RA-01: Orchestration](../readiness-audit/subsystem-orchestration.md), [RA-09: Daimon](../readiness-audit/subsystem-daimon.md)
