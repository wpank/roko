# Phase 3B: Graduation — Pulse → Signal Promotion

## What Is Graduation?

Pulses are ephemeral events on the Bus. Signals are durable data in the Store.
Graduation is the process of promoting a Pulse to a Signal — the only way ephemeral
data enters the audit trail.

## Why It Matters

Currently, the system either:
- Logs everything (wasteful, noisy)
- Logs nothing (loses important events)
- Ad-hoc decides per callsite what to persist

Graduation policies formalize this: define rules for which Pulses get promoted,
and the system applies them consistently.

## Graduation Policy

```rust
/// A policy that watches Bus topics and promotes matching Pulses to Signals.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraduationPolicy {
    /// Bus topics to watch.
    pub watch: TopicFilter,
    /// Criteria for promotion.
    pub criteria: GraduationCriteria,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraduationCriteria {
    /// Always graduate pulses matching these topics.
    pub always: Vec<TopicFilter>,
    /// Never graduate pulses matching these topics.
    pub never: Vec<TopicFilter>,
    /// Sample rate: graduate 1 in N pulses for non-always topics.
    pub sample_rate: Option<usize>,
}
```

## Default Policies

These topics should always graduate (per v2 spec):

| Topic | Why |
|-------|-----|
| `gate.verdict.*` | Audit trail for verification decisions |
| `agent.*.turn.completed` | Episode record for learning |
| `safety.approval.*` | Safety audit trail |
| `conductor.circuit.*` | Health incident records |
| `cost.charged` | Billing/budget audit |

These should never graduate:

| Topic | Why |
|-------|-----|
| `heartbeat.*` | Too frequent, no durable value |
| `agent.*.output` | Streaming tokens — already captured in turn record |

## Implementation

### GraduationCell — A React Cell that promotes Pulses

```rust
struct GraduationCell {
    policies: Vec<GraduationPolicy>,
    store: Arc<dyn Store>,
}

impl Cell for GraduationCell {
    fn cell_id(&self) -> &str { "graduation-policy" }
    fn cell_name(&self) -> &str { "Graduation Policy" }
    fn protocols(&self) -> &[&str] { &["React"] }

    async fn execute(&self, input: Vec<Signal>, ctx: &CellContext) -> Result<Vec<Signal>> {
        // Subscribe to Bus, check policies, promote matching Pulses
        let mut graduated = Vec::new();
        // ... filtering logic ...
        for signal in &graduated {
            ctx.store.put(signal.clone()).await?;
        }
        Ok(graduated)
    }
}
```

### Pulse::graduate() method

```rust
impl Pulse {
    /// Promote this Pulse to a durable Signal.
    pub fn graduate(&self, provenance: Provenance, score: Score) -> Signal {
        Signal {
            id: ContentHash::from_pulse(self),
            kind: self.kind.clone(),
            body: self.body.clone(),
            score,
            balance: 1.0,
            created_at_ms: self.emitted_at_ms,
            provenance,
            lineage: self.lineage_hint.iter().cloned().collect(),
            ..Default::default()
        }
    }
}
```

## Wiring Plan

### Step 1: Add Pulse::graduate() (30 minutes)

Add to `roko-core/src/pulse.rs`. Pure function, no external dependencies.

### Step 2: Add GraduationPolicy config to roko.toml (1 hour)

```toml
[[graduation]]
watch = "gate.verdict.*"
always = true

[[graduation]]
watch = "agent.*.turn.completed"
always = true

[[graduation]]
watch = "heartbeat.*"
never = true
```

### Step 3: Implement GraduationCell (2-3 hours)

A React Cell that subscribes to the Bus and promotes matching Pulses.

### Step 4: Wire into Engine startup (1 hour)

When Engine starts, register GraduationCell as a background React Cell that
watches all Bus traffic.

### Step 5: Add `roko learn graduation` subcommand (30 minutes)

Show graduation statistics: how many pulses promoted, by topic, over time.

## Predict-Publish-Correct

Graduation enables the v2 learning structure:

```
1. Cell publishes prediction (Pulse: prediction.{cell_id})
2. Reality publishes outcome (Pulse: outcome.{cell_id})
3. CalibrationPolicy joins by lineage, computes error
4. Error graduates to Signal (durable learning record)
5. Cell subscribes to its calibration topic, updates model
```

This is already partially built in `roko-learn/src/calibration_policy.rs` (0 callers).
Graduation + Bus wiring makes it structural.

### Wire target for predict-publish-correct

The CascadeRouter already makes predictions (model selection). Wire it to:
1. Publish a prediction Pulse when selecting a model
2. After agent turn, publish an outcome Pulse with actual performance
3. CalibrationPolicy joins them and updates router confidence

This closes the gap identified in QW-8 (03-QUICK-WINS.md).
