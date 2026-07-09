# Carry-Forward Map — Batch 07

Use this when an item appears during batch `07` work but is better executed elsewhere.

| Item | Better Home | Keep In 07 As | Why |
|------|-------------|---------------|-----|
| Yerkes-Dodson pressure dial + flow detection (Doc 12) | later learning / pressure-tuning pass | design note | 919-line doc is theory-heavy; current loop works without `PressureBandit` / `FlowDetector` / `ModelPressureProfile` |
| Typed `CognitiveSignal` enum (Doc 09) | later signal-channel redesign | design note | current `ConductorDecision` 3-state enum covers the interventions that actually fire |
| Good Regulator Brier / Kalman / forward predictor (Doc 08) | later self-model pass | design note | add after `HealthMonitor` / `StuckDetector` wire-ups prove the base loop |
| Federated multi-level conductor + `SelfHealingConductor` (Doc 15) | later multi-agent governance pass | roadmap note | batch 07 should focus on single-plan infrastructure first |
| Watcher composition + `OnlineIsolationForest` / CUSUM (Doc 01 §advanced) | later anomaly research pass | design note | 10 base watchers + EWMA `AnomalyDetector` already meet production need |
| Triple-loop learning / `ConductorLevel` hierarchy (Doc 15) | later meta-learning pass | roadmap note | requires F.22 / F.25 groundwork first |
| Unified algedonic channel (Doc 07) | later signal-channel redesign | design note | current alert/decision emissions (`conductor:alert:<watcher>`, `conductor.decision`) work for now |
| Linux cgroup CPU/memory/IO limits (Doc 13) | later deployment-hardening pass | roadmap note | `ResourceAccount` in-process budgets ship; kernel-level limits can wait |

When deferring, record:

1. the exact file or gap id,
2. why it is out of scope,
3. the batch or pass that should own it,
4. the minimal contract batch `07` still needs to leave behind.
