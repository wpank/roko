---
title: "Technical Analysis × Heartbeat"
section: analysis
subsection: integration-map
id: im-tech-analysis-x-heartbeat
source: 24-cross-section-integration-map.md (§6.1 M17)
missing-integration: M17
tier: 4
tags: [technical-analysis, heartbeat, oracle, predictions, prediction-loop, cognitive-clock]
---

# Technical Analysis × Heartbeat

**Direction**: 20-Technical Analysis → 16-Heartbeat (prediction signals into the cognitive clock)  
**Status**: **Missing (M17)** — Tier 4, ~150 LOC. Blocked on Oracle trait definition (Readiness Audit G21) and Heartbeat implementation (G24).  
**Interface**: `roko-oracle::Prediction` Signals → `roko-heartbeat::CorticalState` tick scheduling

## What Flows

The Technical Analysis subsystem generates domain predictions (market, code, research) with confidence intervals and time horizons. The Heartbeat subsystem manages the cognitive clock — when to run Gamma, Theta, and Delta ticks. High-confidence predictions with short time horizons should increase Heartbeat tick frequency.

| Signal | From | To | Status |
|---|---|---|---|
| `Kind::Prediction` Signals | `roko-oracle` (planned) | `CorticalState` tick scheduler | **Missing** (M17) |
| Prediction urgency scores | Oracle prediction horizon | Heartbeat tick frequency | **Missing** |
| Prediction resolution feedback | Heartbeat tick outcomes | Oracle calibration | **Missing** |

## Reality Check

Both ends of this integration are unimplemented:
- `roko-oracle` does not exist in any crate (G21)
- `CorticalState` / Heartbeat implementation is spec-only (G24)
- Oracle trait does not exist

This is a Phase 2+ integration — design specification only at this stage.

## Design Intent

When a prediction says "compiler error expected in next 2 minutes" (short horizon, high confidence), the Heartbeat should increase Gamma tick frequency to respond quickly. When predictions have long horizons and low confidence, Gamma ticks can be less frequent to conserve compute.

## Open Questions

1. Should prediction urgency modulate all three speeds (Gamma/Theta/Delta) or only Gamma?
2. How does prediction-driven Heartbeat modulation interact with affect-driven scheduling (M1)?
3. Are there domain-specific prediction types that require specific Heartbeat responses?

## Cross-References

- Oracle gap: Readiness Audit G21 (Oracle trait definition)
- Heartbeat gap: Readiness Audit G24 (CorticalState implementation)
- Learning feedback: [learning-x-verification.md](./learning-x-verification.md) — prediction calibration feedback loops
- Readiness audit: [RA-20: Technical Analysis](../readiness-audit/subsystem-technical-analysis.md)
