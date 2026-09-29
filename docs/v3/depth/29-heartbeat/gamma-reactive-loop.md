# Gamma: The Reactive Loop (~5-15s)

> v3 depth file for chapter 29 (Heartbeat and Cognitive Loop).
> Source: v1/16-heartbeat/04-gamma-reactive-loop.md.
> Parent: `docs/v3/29-HEARTBEAT.md` SS5.

---

## 1. Abstract

Gamma is the heartbeat. Every 5-15 seconds, a Roko agent executes one complete
pass through the canonical seven-step universal loop. Gamma is where things
actually happen: probes fire, observations are scored, the tier gate decides
whether to invoke an LLM, and actions are taken if warranted.

The name "Gamma" comes from EEG frequency bands. Biological gamma oscillations
(30-100 Hz) are associated with sensory processing, attention binding, and active
perception (Buzsaki 2006). Roko's gamma loop serves the same function at a longer
timescale: it is the agent's fast reactive perception, the constant sweep of its
environment.

The critical property: **~80% of gamma ticks cost nothing.** The 16 T0 probes run
as pure functions with zero LLM cost. Only when probes detect an anomaly exceeding
the adaptive threshold does the tick escalate to T1 or T2.

---

## 2. Full Tick Execution

Steps 1-3 (SENSE, ASSESS, COMPOSE) always execute. Steps 4-7 are conditional on
the tier gate decision:

1. **SENSE**: Run T0 probes, read Substrate, drain Bus topics, detect regime changes,
   read coordination signals.
2. **ASSESS**: Score retrieved Signals by relevance, recency, emotional congruence,
   confidence. Compute prediction error. Route to T0/T1/T2.
3. **COMPOSE** _(T1/T2 only)_: Assemble context via VCG auction (T2) or fixed-
   template (T1). Token budget: ~4,000 (T1) or ~32,000 (T2).
4. **ACT** _(T1/T2 only)_: Call the LLM through CascadeRouter model selection.
5. **VERIFY** _(if acted)_: Gate pipeline with ratcheting and adaptive thresholds.
6. **PERSIST + BROADCAST**: Store Signal with lineage. Publish Pulse.
7. **REACT**: Episode log, Daimon PAD update, Router feedback, prediction calibration.

### 2.1 SENSE -- `Substrate.query()` + `Bus.subscribe()`

```rust
async fn perceive(
    substrate: &dyn Substrate,
    probes: &[Box<dyn Probe>],
    state: &AgentState,
) -> Result<Observation> {
    let probe_results: Vec<ProbeResult> = probes
        .iter()
        .map(|p| p.evaluate(&state.engine_state))
        .collect();

    let domain_obs = substrate.query(
        &Query::current_state(),
        &state.context,
    ).await?;

    let regime = detect_regime(&probe_results, &state.previous_regime);

    Ok(Observation {
        tick: state.current_tick,
        probes: probe_results,
        domain_state: domain_obs,
        regime,
        anomalies: probe_results.iter()
            .filter(|p| p.is_anomalous())
            .cloned()
            .collect(),
    })
}
```

### 2.2 ASSESS -- Scoring and routing

The scoring function combines four factors (following Bower 1981):

```
score = w_recency * recency(Ebbinghaus)
      + w_importance * quality(confidence * validation_ratio)
      + w_relevance * cosine_similarity(query, entry)
      + w_emotional * PAD_cosine(current_mood, entry_affect)
```

Every 100 ticks, **contrarian retrieval** forces mood-opposite entries to prevent
rumination loops.

---

## 3. Adaptive Gamma Interval

Gamma does not tick at a fixed rate. The interval adapts to environmental volatility
following Friston's (2010) active sampling:

```rust
fn compute_gamma_interval(violations: &[Violation]) -> Duration {
    Duration::from_secs(15)
        .mul_f64(1.0 / (1.0 + violations.len() as f64 * 0.3))
        .max(Duration::from_secs(5))
}
```

| Anomaly Count | Interval | Ticks/Hour |
|---|---|---|
| 0 | 15.0s | 240 |
| 1 | ~11.5s | 313 |
| 3 | ~7.9s | 456 |
| 7+ | 5.0s (floor) | 720 |

---

## 4. The DecisionCycleRecord

Every gamma tick produces a typed, self-contained record serving as: the unit of
dream replay; the unit of credit assignment; the unit of resource accounting; the
source of event fabric events.

```rust
pub struct DecisionCycleRecord {
    // Identity
    pub tick: u64,
    pub timestamp: SystemTime,
    pub agent_id: AgentId,

    // SENSE
    pub observation: Observation,
    pub regime: Regime,
    pub probe_results: Vec<ProbeResult>,

    // ASSESS
    pub prediction_error: f32,
    pub deliberation_threshold: f32,
    pub tier: InferenceTier,

    // COMPOSE
    pub context_bundle_summary: ContextSummary,
    pub retrieved_entries: Vec<SignalSummary>,

    // ACT + VERIFY
    pub deliberation: Option<DeliberationRecord>,
    pub actions: Vec<ActionRecord>,
    pub outcome: Option<OutcomeRecord>,

    // REACT
    pub pad_before: PadVector,
    pub pad_after: PadVector,
    pub primary_emotion: PlutchikLabel,

    // Cost
    pub inference_cost: f64,
    pub total_cost: f64,
}
```

---

## 5. Ten Cognitive Mechanisms

Ten runtime mechanisms modulate how each gamma tick behaves. They are concurrent
processes that inject into gamma, not gamma steps:

| Mechanism | Where in Gamma | Function |
|---|---|---|
| AttentionSalience | SENSE | Priority queue with decay for observation items |
| HabituationMask | SENSE | Per-pattern exposure attenuation |
| EventDrivenWakeup | Before SENSE | Condition-based clock interrupts |
| SleepPressure | ASSESS | Accumulate toward delta threshold |
| HomeostasisRegulator | ASSESS | Proportional control for signal stability |
| ContextDelta | COMPOSE | I-frame/P-frame context compression |
| EpisodicReplay | COMPOSE | Case-based reasoning injection |
| CompensationChain | ACT-PERSIST | Saga-pattern rollback tracking |
| StateSnapshot | REACT | Periodic content-addressed checkpoint |
| MetricsEmitter | Every step | Wide-event telemetry (OTel gen_ai spans) |

---

## 6. Daily Cost Model

| Regime | Gamma Interval | Ticks/Day | T0 Rate | Estimated Daily Cost |
|---|---|---|---|---|
| Calm | ~15s | ~5,760 | ~90% | ~$1.00 (with context eng.) |
| Normal | ~10s | ~8,640 | ~80% | ~$2.50 (with context eng.) |
| Volatile | ~5s | ~17,280 | ~60% | ~$8.00 (with context eng.) |

Without tier gating (every tick at T2): 17,280 * $0.10 = **$1,728/day**. With
gating: ~$8/day in the volatile case. Context engineering provides an additional
~6x reduction.

---

## 7. OODA Loop Correspondence

| OODA Phase | Gamma Step(s) | What Happens |
|---|---|---|
| **Observe** | SENSE | Probes + coordination signals |
| **Orient** | ASSESS | Scoring + prediction error |
| **Decide** | ASSESS + COMPOSE | Tier gate + context assembly |
| **Act** | ACT + VERIFY + PERSIST | Execution with verification |
| _(missing)_ | REACT | Episode recording + affect update |

---

## 8. References

- **Buzsaki 2006** -- "Rhythms of the Brain" (Oxford University Press).
- **Friston 2010** -- "The Free-Energy Principle" (Nature Reviews Neuroscience 11(2)).
- **Kahneman 2011** -- "Thinking, Fast and Slow".
- **Bower 1981** -- "Mood and Memory" (American Psychologist 36(2)).
- **Barrett 2017** -- "How Emotions Are Made" (Houghton Mifflin).
- **Sumers et al. 2023** -- CoALA framework (arXiv:2309.02427).
- **Chen et al. 2023** -- FrugalGPT (arXiv:2305.05176).
- **Baddeley 2000** -- "The episodic buffer" (Trends in Cognitive Sciences 4(11)).
- **Mattar & Daw 2018** -- "Prioritized memory access" (Nature Neuroscience 21).

---

## Cross-References

- `docs/v3/depth/29-heartbeat/three-cognitive-speeds-t0-t1-t2.md` -- Hierarchy
- `docs/v3/depth/29-heartbeat/theta-reflective-loop.md` -- Reflective loop
- `docs/v3/depth/29-heartbeat/dual-process-t0-t1-t2.md` -- Tier gating
- `docs/v3/depth/29-heartbeat/16-t0-probes.md` -- 16 probes
- `docs/v3/depth/29-heartbeat/attention-auction-and-gating.md` -- VCG auction
- `docs/v3/29-HEARTBEAT.md` -- Parent chapter
