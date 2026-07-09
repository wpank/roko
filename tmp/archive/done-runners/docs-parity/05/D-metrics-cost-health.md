# D — Metrics + Cost + Health

Audit-corrected parity view of the metrics, cost, and health docs in `docs/05-learning/`.

---

## What Is Already Shipped

- `AgentEfficiencyEvent` is real.
- task metrics, baselines, and regression reporting are real.
- cost normalization is real.
- budget guardrails, provider health, latency tracking, and anomaly detection are real.

This area is no longer a "missing observability stack" problem. It is a **doc accuracy and output-quality problem**.

## What The Old Parity Material Overstated

- the docs make regression output sound more complete than the current runtime path,
- per-slice baselines already exist, but the main regression alerts still lag behind that model,
- cost normalization and efficiency events already exist and should be described in present tense,
- advanced drift-detection theory should not be implied as shipped.

## Corrected Status

### Shipping

- efficiency events
- task metrics and baseline computation
- normalized model-cost handling
- provider health and latency registries
- budget guardrails
- anomaly detection

### Ship Soon

- activate iteration-based regression checks everywhere the public threshold type already promises them,
- emit slice-aware regressions that match the current baseline model,
- make budget and experiment outcomes easier for operators to inspect.

### Deferred

- advanced drift detectors
- dashboard redesign
- governance-heavy regression policy layers

## Practical Rewrite Guidance

When touching metrics/cost docs:

1. keep `AgentEfficiencyEvent` and cost normalization in present tense,
2. describe the main runtime gap as **slice-aware regression output**, not missing metrics collection,
3. keep advanced detectors and policy automation under future work.

## Batch-Ready Follow-Ups

- `L3`: make regression output match the slice-aware docs
- `L5`: improve the operator-facing budget and experiment story without changing router families

## Source Anchors

- `crates/roko-learn/src/efficiency.rs:80` — `AgentEfficiencyEvent`
- `crates/roko-learn/src/regression.rs:69` — `RegressionAlert`
- `crates/roko-learn/src/regression.rs:140` — `detect_regressions`
- `crates/roko-learn/src/budget.rs:8` — `BudgetGuardrail`
- `crates/roko-learn/src/budget.rs:24` — `BudgetAction`
- `crates/roko-learn/src/provider_health.rs` — provider health / circuit breaker
- `crates/roko-learn/src/latency.rs` — latency registry

## Bottom Line

The metrics, cost, and health layer already exists. The parity refresh should stop framing it as absent and instead narrow the work to the remaining output gaps: slice-aware regressions, iteration thresholds, and clearer operator-facing summaries.
