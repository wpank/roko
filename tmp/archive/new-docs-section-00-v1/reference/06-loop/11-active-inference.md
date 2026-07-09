# Active Inference in the Cognitive Loop

> How the loop predicts, acts, and corrects — the architectural impact of
> active inference on tick-by-tick operation.

**Status**: Shipping
**Crate**: `roko-agent`
**Depends on**: [loop\_tick()](09-loop-tick-code.md), [REACT stage](08-stage-react.md),
[Pulse](../02-pulse/README.md)
**Used by**: [Dual-Process](10-dual-process.md),
[Neuro cross-cut](../09-cross-cuts/01-neuro.md)
**Last reviewed**: 2026-04-19

---

## TL;DR

Before each tick, the loop publishes a **prediction Pulse** — a forward model of what
the tick expects to do and learn. After the tick's PERSIST stage completes, REACT
publishes a **prediction.error Pulse** comparing the prediction to the actual outcome.
These two Pulses together implement the Free Energy Principle's predict/update cycle
at the architectural level, driving online adaptation of routing priors and context
assembly heuristics.

> **Research foundations**: The full theoretical treatment — Friston's Free Energy
> Principle, predictive coding, and the neuroscience grounding — lives in
> [`research/foundations/active-inference.md`](../../research/foundations/active-inference.md)
> (created in Cluster G). This page documents **architectural impact only**.

---

## The Idea

Active inference frames cognition as a continuous process of predicting the world and
minimizing the difference between predictions and observations (prediction error, or
"free energy"). The system does not simply react to stimuli — it anticipates them, and
the *surprise* of being wrong is the primary learning signal.

Roko implements this cycle pragmatically:

1. **Predict**: before the tick runs, emit a structured prediction of the tick's
   expected routing target, context size, and verification outcome.
2. **Act**: run the tick normally.
3. **Correct**: after PERSIST, compute the difference between prediction and reality,
   emit a `predict.error` Pulse, and update the routing and scoring priors.

This is a lightweight implementation of predictive coding that does not require
a separate neural process — it uses the existing loop infrastructure.

---

## The Two Pulses

### prediction Pulse (emitted before QUERY)

```rust
// source: crates/roko-agent/src/inference/predict.rs
pub struct PredictionPulse {
    pub tick_id:             TickId,
    pub expected_target:     RouteTarget,
    pub expected_confidence: f32,
    pub expected_verify:     PredictedVerdict,
    pub expected_tokens:     usize,
    pub model:               WorldModelSnapshot,
}
```

The `WorldModelSnapshot` is a compact summary of the agent's current beliefs: which
routes have been reliable for this stimulus type, what verification failure rate has
been seen recently, and what context size has led to VERIFY passes.

### predict.error Pulse (emitted by REACT)

```rust
// source: crates/roko-agent/src/inference/error.rs
pub struct PredictionErrorPulse {
    pub tick_id:          TickId,
    pub actual_target:    RouteTarget,
    pub actual_verify:    Verdict,
    pub actual_tokens:    usize,
    pub route_error:      f32,  // |expected_target ≠ actual_target|
    pub verify_error:     f32,  // |expected_verify ≠ actual_verify|
    pub total_free_energy: f32, // scalar summary of all errors
}
```

`total_free_energy` is the sum of all normalized per-dimension errors. When this
value consistently trends upward, the agent's world model is degrading — a signal to
trigger T2 consolidation.

---

## How Prediction Errors Drive Adaptation

The `predict.error` Pulse is consumed by two subsystems:

### 1. Routing prior update

The `CascadeRouter` maintains a running estimate of confidence for each
(stimulus_type, route_target) pair. A large `route_error` decreases the confidence
estimate for that pair. Over time, routes that repeatedly surprise the prediction model
will shift from T0 (high confidence) to T1 (requires deliberation).

```rust
// source: crates/roko-agent/src/loop/route/cascade.rs
fn update_priors(&mut self, error: &PredictionErrorPulse) {
    let key = RouteKey::from(error.actual_target);
    let alpha = self.learning_rate;
    self.confidence_ema[key] = (1.0 - alpha) * self.confidence_ema[key]
        + alpha * (1.0 - error.route_error);
}
```

This is an exponential moving average. `learning_rate` defaults to 0.05 — slow enough
that a single surprise doesn't destabilize routing, fast enough to adapt over dozens
of ticks.

### 2. Free Energy threshold for T2 consolidation

The scheduler tracks a rolling average of `total_free_energy`. When it exceeds the
configured `consolidation_threshold`, the scheduler triggers a T2 Delta tick:

```toml
[active_inference]
learning_rate               = 0.05
consolidation_threshold     = 0.35   # free_energy rolling average above this → T2
free_energy_window_ticks    = 50     # rolling window size
```

T2 consolidation (Dreams cross-cut) reorganizes the Substrate to reduce future
prediction errors. See [Dreams cross-cut](../09-cross-cuts/03-dreams.md).

---

## Relationship to the World Model

The agent's "world model" is not a separate neural network — it is the routing prior
table plus the scoring weight history. These are persisted as a special class of
Engram (`Kind::ModelState`) and updated in the PERSIST stage.

This means the world model is:
- **Durable**: survives agent restarts
- **Auditable**: each update is a versioned Engram
- **Introspectable**: the prior table can be queried via the Substrate API

---

## Invariants

1. Every tick produces exactly one `predict.error` Pulse.
2. The `prediction` Pulse is published before QUERY runs, never after.
3. `total_free_energy` is always in [0.0, ∞). A value of 0.0 means perfect prediction.
4. Routing prior updates are always EMA-based — no single tick causes a step change.

---

## Open Questions

- Should the world model include an explicit generative model (a small neural net)
  rather than a tabular prior? This would enable generalization across novel stimulus
  types but adds deployment complexity.
- What is the right `consolidation_threshold`? The current default (0.35) is
  empirically motivated; a principled derivation from information-theoretic bounds
  would be stronger.

See also [Open Questions](16-open-questions.md) for loop-level open items.

---

## See also

- [`research/foundations/active-inference.md`](../../research/foundations/active-inference.md) — theoretical foundations (Cluster G)
- [Dual-Process](10-dual-process.md) — how prediction errors drive T0→T1→T2 escalation
- [REACT stage](08-stage-react.md) — where prediction.error is published
- [Dreams cross-cut](../09-cross-cuts/03-dreams.md) — offline consolidation triggered by high free energy
- [Neuro cross-cut](../09-cross-cuts/01-neuro.md) — Neuro maintains the world model Engrams
