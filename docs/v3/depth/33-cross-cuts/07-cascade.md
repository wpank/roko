# 33-07 -- Gate-Failure Cascade

> **Parent**: [33-CROSS-CUTS](../../33-CROSS-CUTS.md)
>
> When a verification gate fails, the natural transformations fire in a coordinated
> sequence that weakens responsible knowledge, shifts the affect state, and
> conditionally triggers a delta dream. This depth file covers the cascade protocol,
> the evidence trail, and the non-blocking runner integration.

**Authority**: `crates/roko-compose/src/natural_transforms.rs` (`run_gate_failure_cascade`)

---

## 1. Purpose

The gate-failure cascade exists because a failed gate is not just a verdict -- it is a
learning event that should propagate through all three behavioral cross-cuts:

| System | What it learns |
|--------|----------------|
| Memory | "The knowledge entries in context were not helpful for this task" |
| Daimon | "This outcome was negative; update PAD accordingly" |
| Dreams | "This episode has high prediction error; prioritize it for replay" |

Without the cascade, these updates would happen independently, potentially with
inconsistent timing and ordering. The cascade ensures they happen atomically (the
synchronous portion) or under a consistent snapshot (the asynchronous delta dream).

---

## 2. The Seven Steps

### Step 1: VERIFY Emits Verdict

A gate failure produces a `Kind::GateVerdict` Signal with `passed: false`. This
is the entry point.

### Step 2: Memory Weakening

The cascade receives a `MemoryOutcome` identifying the affected knowledge entries.
Two store mutations run:

1. `score_prediction_utility(entry_ids, correct=false, accuracy=0.0)` -- reduces
   confidence based on prediction failure.
2. `batch_record_usage([(id, false), ...])` -- records unsuccessful usage, feeding
   the demurrage/GC pipeline.

After these mutations, the affected entries have lower balance and confidence,
making them less likely to be retrieved in future queries.

### Step 3: eta_MN (Memory -> Daimon)

The `MemoryOutcome` is transformed into an `AffectEvent::GateResult`:

```rust
AffectEvent::GateResult {
    plan_id: outcome.plan_id,
    task_id: outcome.task_id,
    passed: false,
    rung: outcome.rung,
}
```

This event is applied to the live `DaimonState` via `appraise()`, which shifts the
PAD vector -- typically decreasing pleasure and increasing arousal.

### Step 4: eta_NM (Daimon -> Memory)

The updated Daimon state is snapshotted into a `DaimonAssessment` and transformed
into a `KnowledgeEntry` at `Working` tier. This entry records:

```
PAD assessment p=-0.600 a=0.800 d=-0.500 state=Struggling
```

The entry is ingested into the KnowledgeStore, making the affect state queryable
by future Memory retrievals.

### Step 5: eta_MD + eta_ND (Both Paths to Dreams)

Two paths produce `DreamConsolidationInput`:

1. **Memory path**: `eta_MD(eta_NM(assessment))` -- the stored knowledge entry is
   converted to a replay input.
2. **Direct path**: `eta_ND(assessment)` -- the Daimon assessment is directly
   converted to a replay input.

The commuting triangle guarantees these produce identical output:

```rust
debug_assert_eq!(memory_path.episode_ids, direct_path.episode_ids);
debug_assert_eq!(memory_path.priority, direct_path.priority);
debug_assert_eq!(memory_path.trigger_delta, direct_path.trigger_delta);
```

### Step 6: Conditional Delta Dream

If the Daimon assessment shows `BehavioralState::Struggling`, the `trigger_delta`
flag is true. The runner's async worker then runs a delta dream cycle, which produces
a `DreamCycleReport`.

### Step 7: Dream Publication (eta_DM + eta_DN)

The completed dream report is consumed by `DreamOutputConsumer`:

- **eta_DM**: consolidated knowledge entries are ingested into the store
- **eta_DN**: affect depotentiation runs, reducing negative emotional charge

After Step 7, the system has:
- Weakened the responsible knowledge
- Shifted the affect state (twice: once from the failure, once from depotentiation)
- Stored both the affect assessment and the dream output as durable knowledge
- Reduced the emotional charge of the failure experience

---

## 3. The Evidence Struct

```rust
pub struct GateFailureCascade {
    /// Memory outcome that initiated the cascade.
    pub memory_outcome: MemoryOutcome,
    /// PAD after applying eta_MN.
    pub updated_pad: PadVector,
    /// Daimon snapshot offered to Memory and Dreams.
    pub daimon_assessment: DaimonAssessment,
    /// NREM input produced via Daimon -> Memory -> Dreams.
    pub memory_path: DreamConsolidationInput,
    /// NREM input produced directly via Daimon -> Dreams.
    pub direct_path: DreamConsolidationInput,
}
```

This struct captures the full audit trail of the synchronous cascade. The caller
can inspect it to verify commutativity and log the cascade outcome.

---

## 4. Consolidation Priority

The replay priority for dream inputs is computed from the Daimon assessment:

```rust
fn consolidation_priority(assessment: &DaimonAssessment) -> f64 {
    (0.35
        + assessment.pad.arousal.max(0.0) * 0.25
        + (-assessment.pad.pleasure).max(0.0) * 0.20
        + (1.0 - assessment.confidence.clamp(0.0, 1.0)) * 0.20)
        .clamp(0.0, 1.0)
}
```

The formula produces higher priority when:
- Arousal is high (the agent is activated by the failure)
- Pleasure is low (the outcome was negative)
- Confidence is low (the agent is uncertain about the situation)

The base value of 0.35 ensures even mild failures get some replay priority.

---

## 5. Non-Blocking Runner Integration

The cascade must not block the runner event loop. The production wiring:

1. A gate failure completion is detected in the graph task dispatch.
2. `tokio::spawn` creates a new task.
3. Inside the task, `spawn_blocking` runs the synchronous
   `run_gate_failure_cascade()`.
4. If `trigger_delta` is true, the blocking worker runs the delta dream.
5. The dream report is consumed via `DreamOutputConsumer::consume()`.
6. Any error in the cascade is logged (via `tracing`) without changing the gate
   result or blocking the event loop.

This design ensures that cascade failures are observable but not fatal. The gate
result stands regardless of whether the cascade succeeds.

---

## 6. Key Tests

| Test | What it verifies |
|------|-----------------|
| `gate_failure_cascade_weakens_memory_and_produces_commuting_replay_inputs` | Real store weakening, commuting triangle assertion, end-to-end cascade |
| `daimon_memory_dreams_triangle_commutes` | Both paths produce identical episode IDs, priority, and trigger |
| `eta_mn_preserves_gate_identity_and_outcome` | Gate result faithfully converted to affect event |

---

## References

- See [07-GATES](../../07-GATES.md) for the gate pipeline specification.
- See [08-LEARNING](../../08-LEARNING.md) for the broader learning loop.
- See [01-functor-model.md](01-functor-model.md) for the commuting triangle proof.
