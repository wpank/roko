---
title: "Daimon × Learning"
section: analysis
subsection: integration-map
id: im-daimon-x-learning
source: 24-cross-section-integration-map.md (§3.2, §4.1)
tags: [daimon, learning, affect-routing, cascade-router, behavioral-state, wired]
---

# Daimon × Learning

**Direction**: 09-Daimon → 05-Learning (PAD state modulates model-tier selection via CascadeRouter)  
**Status**: **Wired** — CascadeRouter reads live Daimon behavioral state  
**Interface**: `roko-daimon::AffectState` → `roko-learn::CascadeRouter`

## What Flows

| Signal | From | To | Status |
|---|---|---|---|
| `BehavioralState` (Struggling/Focused/etc.) | `roko-daimon::AffectState` | `CascadeRouter` tier bias | **Wired** |
| PAD vector (continuous) | `roko-daimon` | CascadeRouter urgency weight | **Wired** |

## What Is Wired

CascadeRouter reads live Daimon behavioral state and biases model-tier selection:
- `Struggling` state → prefers cheaper, faster models (less capacity to iterate with slow models)
- `Focused` state → allows premium models (productive state can leverage quality)
- `Coasting` state → standard tier selection

## Gaps

- `DaimonPolicy` is wired but `AffectRouter` (PAD-biased routing) is still listed as Missing in source file 24 — the routing is real but the full PAD-parameterized routing (proportional to all 3 axes) is not yet implemented.

## Enhancement Opportunities

- [daimon-x-orchestration.md](./daimon-x-orchestration.md) — M1: affect should also modulate scheduling cadence
- [daimon-x-composition.md](./daimon-x-composition.md) — M2: affect should also modulate composition weights

## Cross-References

- Readiness audit: [RA-09: Daimon](../readiness-audit/subsystem-daimon.md), [RA-05: Learning](../readiness-audit/subsystem-learning.md)
