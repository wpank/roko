# E — Feedback + Calibration

Audit-corrected parity view of the feedback-loop and predictive-calibration docs in `docs/05-learning/`.

---

## What Is Already Shipped

- `LearningRuntime` is the real integration hub.
- multiple feedback loops are already wired in production.
- `prediction.rs`, `drift.rs`, and `regression.rs` all exist today.
- predictive prompt/scoring consumers already read calibration information derived from routing logs.

## What The Old Parity Material Overstated

- predictive calibration was described as if one richer direct pipeline already shipped,
- the real runtime path today is the **routing-log replay path**,
- `PredictionRecord::register/resolve` exists but is not the live source of truth,
- `DriftDetector` and `run_learning_subscriber` still need an explicit "live or dormant" decision,
- FEP / VSM / Friston framing adds little engineering value because the active-inference machinery already exists in code.

## Corrected Status

### Shipping

- `LearningRuntime`
- routed feedback loops around health, latency, skills, experiments, and local rewards
- routing-log-backed predictive calibration consumers
- existing `prediction.rs`, `drift.rs`, and `regression.rs` modules

### Ship Soon

- choose one canonical calibration path,
- expose one simple, real calibration summary on that path,
- add a typed heuristic calibration struct as a narrower follow-up than the old grand-theory docs,
- decide whether `DriftDetector` / `run_learning_subscriber` are live or dormant.

### Deferred

- universal predict-publish-correct bus architecture
- Brier / reliability / arithmetic-corrector doctrine unless actually implemented
- worldview-driven calibration layers
- replication-ledger or constitutional constraint systems

## Practical Rewrite Guidance

When touching feedback/calibration docs:

1. keep `LearningRuntime` and the routed feedback loops in present tense,
2. make the routing-log replay path the documented default unless code changes say otherwise,
3. label predictive-foraging extras and bus-centric calibration doctrine as planned.

## Batch-Ready Follow-Ups

- `L4`: canonicalize predictive calibration
- `L6`: resolve drift/subscriber ambiguity

## Source Anchors

- `crates/roko-learn/src/runtime_feedback.rs:323` — `LearningRuntime`
- `crates/roko-learn/src/runtime_feedback.rs:782` — `record_completed_run`
- `crates/roko-learn/src/prediction.rs:14` — `PredictionRecord`
- `crates/roko-learn/src/prediction.rs:125` — `CalibrationTracker`
- `crates/roko-learn/src/prediction.rs:274` — `adjust_prediction`
- `crates/roko-learn/src/drift.rs:89` — `DriftDetector`
- `crates/roko-learn/src/regression.rs:140` — `detect_regressions`
- `crates/roko-cli/src/orchestrate.rs:253` — calibration loading from workdir
- `crates/roko-cli/src/orchestrate.rs:312` — predictive policy sections

## Bottom Line

The feedback stack is already substantial. The parity refresh should stop describing the richer predictive-foraging doctrine as if it is live and instead document the narrower, real contract: routing logs feed the current calibration consumers, and the dormant modules still need an explicit decision.
