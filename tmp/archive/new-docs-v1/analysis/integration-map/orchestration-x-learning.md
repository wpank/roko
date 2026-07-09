---
title: "Orchestration × Learning"
section: analysis
subsection: integration-map
id: im-orchestration-x-learning
source: 24-cross-section-integration-map.md (§3.1, §4.1)
tags: [orchestration, learning, episodes, cascade-router, wired]
---

# Orchestration × Learning

**Direction**: Bidirectional — Orchestration generates task outcomes; Learning drives model routing  
**Status**: **Wired**  
**Interface**: `roko-orchestrator` task completion → `roko-learn::LearningRuntime`; `roko-learn::CascadeRouter` → `roko-agent` model selection

## What Flows

| Signal | From | To | Status |
|---|---|---|---|
| Task dispatch events | `roko-orchestrator` | `LearningRuntime` | **Wired** |
| `AgentEfficiencyEvent` | Orchestrator + gate | Learning runtime | **Wired** |
| `Kind::Episode` (per completed run) | Learning runtime | `episodes.jsonl` | **Wired** |
| CascadeRouter decision (model selection) | `roko-learn::CascadeRouter` | `roko-agent` via orchestrator | **Wired** |
| `Kind::Playbook` (proven plans) | Learning runtime | `roko-compose` | **Partial** |

## Invariants of the Interaction

1. Every task completion (success or failure) generates an episode record.
2. CascadeRouter decisions are made with the current episode history and health metrics.
3. The orchestrator does not dictate which model to use — it defers to CascadeRouter.

## Enhancement Opportunities

- [conductor-x-routing.md](./conductor-x-routing.md) — M9: system load also biases routing
- [learning-x-routing.md](./learning-x-routing.md) — M6: budget state also biases routing
- [daimon-x-orchestration.md](./daimon-x-orchestration.md) — M1: affect state should modulate scheduling

## Cross-References

- Readiness audit: [RA-01: Orchestration](../readiness-audit/subsystem-orchestration.md), [RA-05: Learning](../readiness-audit/subsystem-learning.md)
