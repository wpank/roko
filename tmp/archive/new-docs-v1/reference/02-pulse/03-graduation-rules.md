# Pulse — Graduation Rules

> Graduation is the process by which a Pulse becomes an Engram. This page specifies when and how graduation occurs.

**Status**: Specified  
**Crate**: `roko-core` (planned)  
**Depends on**: [Specification](01-specification.md), [Engram Builder](../01-engram/07-builder-pattern.md)  
**Last reviewed**: 2026-04-19

> **Target state — no code yet.**

---

## TL;DR

Not every Pulse becomes an Engram. Graduation is a conscious decision by a subscriber:
"this event is significant enough to persist." The Bus provides no automatic graduation.
Graduation rules are configured per subscriber and per topic. After graduation, the
Engram's `tags` carry the Pulse's `correlation_id` for trace correlation.

---

## The Idea

If every Pulse became an Engram, the Substrate would fill with transient noise. Most
heartbeat ticks, most probe readings, most routine gate events are not worth persisting.
But some are: a prediction-error spike that triggers speed escalation is worth recording.
A gate failure that caused a retry is worth recording. The difference is significance.

Graduation gives subscribers the choice. A subscriber that monitors prediction errors
can decide: "if prediction error > 0.8, graduate to an Engram." A subscriber that simply
logs gate events discards the Pulse after logging.

---

## Graduation Process

### Step 1: Subscriber Decides

```rust
<!-- source: crates/roko-core/src/graduation.rs (target state) -->

let handle = bus.subscribe(
    TopicFilter::Prefix("prediction.error.".to_string()),
    |pulse: Pulse<PredictionErrorEvent>| {
        if pulse.payload.error > 0.8 {
            // Graduate: this error is significant
            let engram = EngramBuilder::new()
                .kind(Kind::Observation)
                .body(Body::Observation(ObservationBody {
                    source: "prediction-error-monitor".to_string(),
                    content_json: serde_json::to_string(&pulse.payload).unwrap(),
                    was_expected: false,
                }))
                .tag("correlation_id", pulse.correlation_id
                    .map(|c| c.to_hex())
                    .unwrap_or_default())
                .tag("pulse_topic", pulse.topic.0.clone())
                .provenance(Provenance {
                    author: pulse.source.name().to_string(),
                    trust: TrustLevel::LocalAgent,
                    tainted: false,
                    custody: vec![],
                })
                .build()
                .expect("graduation build failed");
            substrate.insert(engram).expect("graduation insert failed");
        }
        // else: discard
    },
);
```

### Step 2: Engram Tags Preserve Correlation

The graduated Engram's `tags` must carry at minimum:

| Tag key | Value |
|---------|-------|
| `pulse.correlation_id` | `correlation_id.to_hex()` if present |
| `pulse.topic` | `topic.0` |
| `pulse.source` | `source.name()` |

This allows log correlation: given an Engram, find the original Pulse that produced it.

---

## Standard Graduation Predicates

The following predicates are commonly used in graduation decisions:

| Predicate | Typical topics |
|-----------|---------------|
| `error_rate > threshold` | `prediction.error.*` |
| `gate failed` | `gate.fail` |
| `tool exited with error` | `tool.complete` |
| `health degraded` | `health.degraded` |
| `anomaly detected` | `substrate.gc` |

---

## Anti-patterns

**Do not graduate every Pulse.** High-frequency topics (heartbeat ticks, metric samples)
would flood the Substrate. Use step-down graduation: only graduate if the value is outside
normal bounds.

**Do not forget correlation_id.** Without it, the Engram cannot be traced back to the
event that produced it.

---

## Invariants

1. A graduated Engram is a valid Engram (passes `EngramBuilder::build()`)
2. The Pulse is not modified during graduation (the Bus receives the original)
3. Graduation is always the subscriber's responsibility; the Bus does not auto-graduate

---

## Open Questions

- Should the Bus support declarative graduation rules configured externally?
- Should graduation produce a `Pulse` → `Engram` link in the custody record?

---

## See Also

- [`01-specification.md`](01-specification.md) — Pulse lifecycle
- [`../01-engram/07-builder-pattern.md`](../01-engram/07-builder-pattern.md) — how to build the graduated Engram
- [`07-open-questions.md`](07-open-questions.md)
