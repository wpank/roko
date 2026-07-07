---
title: "Dreams × Daimon"
section: analysis
subsection: integration-map
id: im-dreams-x-daimon
source: 24-cross-section-integration-map.md (§6.1 M18, §8.3)
missing-integration: M18
tier: 3
tags: [dreams, daimon, depotentiation, consolidation, affect-reset, PAD]
---

# Dreams × Daimon

**Direction**: 10-Dreams → 09-Daimon (depotentiation during consolidation cycles)  
**Status**: **Missing (M18)** — Tier 3, ~80 LOC. Depends on M7 (Dreams→Neuro) being implemented first.  
**Interface**: `DreamRunner` depotentiation events → `roko-daimon::AffectState` PAD update

## What Flows

During sleep/consolidation cycles, the brain (and by analogy, the Dreams subsystem) performs depotentiation — a weakening of strong emotional associations that prevents affective memory from becoming permanently calcified. In Roko, this means Dream cycles should gently nudge the PAD vector back toward neutral.

| Signal | From | To | Status |
|---|---|---|---|
| Depotentiation event | `DreamRunner` post-cycle | `roko-daimon` PAD baseline | **Missing** (M18) |
| Emotional episode reprocessing | `DreamRunner` | Affect baseline adjustment | **Missing** |

## Wiring Recipe

```rust
// In DreamRunner, after completing an NREM cycle:
let dream_cycle_result = self.run_nrem_cycle(&pending_episodes);

// Depotentiation: nudge PAD toward neutral based on resolved emotional episodes
let emotional_episodes: Vec<&Episode> = pending_episodes.iter()
    .filter(|e| e.affect_delta.is_some())
    .collect();

if !emotional_episodes.is_empty() {
    let depotentiation = compute_depotentiation(&emotional_episodes);
    daimon.apply_depotentiation(depotentiation);
}

// In roko-daimon::apply_depotentiation():
fn apply_depotentiation(&mut self, signal: DepotentiationSignal) {
    // Gentle mean reversion toward neutral
    let decay_rate = 0.1;  // 10% regression toward neutral per dream cycle
    self.pad.pleasure *= (1.0 - decay_rate);
    self.pad.arousal  *= (1.0 - decay_rate);
    self.pad.dominance *= (1.0 - decay_rate);
    self.update_behavioral_state();
}
```

Estimated LOC: ~80. Depends on M7 for `run_nrem_cycle` and cursor infrastructure.

## Theoretical Basis

Based on research on sleep and emotional memory (Walker & van der Helm 2009): REM sleep performs a form of memory reconsolidation that strips the emotional intensity from episodic memories while preserving the semantic content. The PAD drift toward neutral after a dream cycle models this depotentiation process.

## Invariants of the Interaction

1. Depotentiation is additive on top of normal PAD updates — it does not override task-driven PAD changes.
2. Depotentiation only runs after a completed dream cycle, not in partial/interrupted cycles.
3. The decay rate (10% per cycle) is configurable: `daimon.depotentiation_rate` in `roko.toml`.
4. Depotentiation cannot push PAD past neutral — it regresses toward [0,0,0] but does not overshoot.

## Failure Modes

| Failure | Consequence | Detection |
|---|---|---|
| M7 not yet implemented | No dream cycles; no depotentiation | M18 depends on M7 |
| Depotentiation too aggressive | PAD always near neutral; no behavioral variety | Tune `depotentiation_rate`; monitor PAD variance |
| Depotentiation not applied after crash | PAD stuck in high-affect state | Crash recovery: apply depotentiation at startup if last dream cycle unconfirmed |

## Open Questions

1. Should depotentiation differentiate by episode type (e.g., don't depotentiate PAD from safety violations)?
2. Should the REM imagination cycle (Pearl SCM counterfactuals) produce a different depotentiation profile than NREM replay?

## Cross-References

- Prerequisite: [dreams-x-neuro.md](./dreams-x-neuro.md) — M7 (must be implemented first)
- Cross-cut triangle: [00-overview.md](./00-overview.md) §8.3
- Readiness audit: [RA-10: Dreams](../readiness-audit/subsystem-dreams.md), [RA-09: Daimon](../readiness-audit/subsystem-daimon.md)
