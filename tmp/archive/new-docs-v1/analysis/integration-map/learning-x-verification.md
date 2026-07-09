---
title: "Learning × Verification"
section: analysis
subsection: integration-map
id: im-learning-x-verification
source: 24-cross-section-integration-map.md (§3.1, §3.3, §4.1)
tags: [learning, verification, gate-verdict, episodes, cascade-router, wired]
---

# Learning × Verification

**Direction**: 04-Verification → 05-Learning (verdict feedback into learning runtime)  
**Status**: **Wired** — `GateVerdict` Engrams flow into `LearningRuntime.record_completed_run()`  
**Interface**: `roko-gate::GateVerdict` → `roko-learn::LearningRuntime`

## What Flows

| Signal | From | To | Status |
|---|---|---|---|
| `Kind::GateVerdict` (pass/fail, score, gate name) | `roko-gate` | `LearningRuntime::record_completed_run()` | **Wired** |
| `AgentEfficiencyEvent` (28-field composite) | Verification + orchestration | Learning runtime | **Wired** |
| Episode log entry | Learning runtime | `episodes.jsonl` | **Wired** |
| CascadeRouter arm update | Learning runtime (feedback) | `CascadeRouter` | **Wired** |
| Adaptive threshold update | Gate verdict history | `AdaptiveThresholds` | **Wired** |

## Invariants of the Interaction

1. Every gate verdict is forwarded to the learning runtime — no silent discarding.
2. The `AgentEfficiencyEvent` is the richest data structure in the system (28 fields); all learning subsystems consume it.
3. The learning runtime does not modify gate pipeline behavior directly — it updates router arms and threshold parameters that the gate reads on the next tick.
4. Verdict Engrams have content-addressed identity — the same verdict cannot be recorded twice.

## Enhancement Opportunities

- [../architectural-analysis/08-novel-proposals.md](../architectural-analysis/08-novel-proposals.md) — Proposal 2: use continuous `verdict.score` as learning signal (currently binary)
- [neuro-x-verification.md](./neuro-x-verification.md) — M14: knowledge-informed thresholds (feedback loop from learning → knowledge → gate)

## Cross-References

- Orchestration feedback: [verification-x-orchestration.md](./verification-x-orchestration.md) — M3 (failure→replanning gap)
- Neuro persistence: [neuro-x-learning.md](./neuro-x-learning.md) — learning outputs are also fed to NeuroStore
- Readiness audit: [RA-04: Verification](../readiness-audit/subsystem-verification.md), [RA-05: Learning](../readiness-audit/subsystem-learning.md)
