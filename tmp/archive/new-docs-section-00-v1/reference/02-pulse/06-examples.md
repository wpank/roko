# Pulse — Examples

> Worked examples of Pulse emission, routing, graduation, and the EventBus today.

**Status**: Specified (Pulse) / Shipping (EventBus)  
**Crate**: `roko-core` (planned Pulse), `roko-runtime` (EventBus)  
**Last reviewed**: 2026-04-19

---

## Example 1: Emit and Subscribe with EventBus (Today — Shipping)

```rust
<!-- source: crates/roko-runtime/src/event_bus.rs -->

let bus: EventBus<GateEvent> = EventBus::new();

// Subscribe
let id = bus.subscribe(|event: GateEvent| {
    if !event.passed {
        tracing::warn!("Gate failed: {}", event.gate_name);
    }
});

// Emit
bus.emit(GateEvent {
    passed: false,
    gate_name: "syntax_check".to_string(),
    confidence: 0.1,
});
```

---

## Example 2: Target-State Pulse Emission

```rust
<!-- source: crates/roko-core/src/pulse.rs (target state) -->

let pulse = Pulse {
    topic: Topic::parse("gate.fail")?,
    source: PulseSource::Subsystem { name: "gate-pipeline".to_string() },
    payload: GateFailedPayload {
        gate_name: "syntax_check".to_string(),
        confidence: 0.1,
        rung: 1,
    },
    emitted_at_ms: now_ms(),
    correlation_id: Some(CorrelationId::new()),
};

bus.emit(pulse);
```

---

## Example 3: Subscribe with TopicFilter (Target State)

```rust
<!-- source: crates/roko-core/src/bus.rs (target state) -->

// Subscribe to all gate events
let handle = bus.subscribe(
    TopicFilter::Prefix("gate.".to_string()),
    |pulse: Pulse<GateEvent>| {
        tracing::info!("Gate event on topic {}: passed={}", pulse.topic.0, pulse.payload.passed);
    },
);
```

---

## Example 4: Graduation Pattern (Target State)

```rust
<!-- source: crates/roko-core/src/graduation.rs (target state) -->

let handle = bus.subscribe(
    TopicFilter::Exact(Topic::parse("prediction.error.high")?),
    |pulse: Pulse<PredictionErrorEvent>| {
        // Graduate to Engram for durable storage
        let engram = EngramBuilder::new()
            .kind(Kind::Observation)
            .body(Body::Observation(ObservationBody {
                source: pulse.source.name(),
                content_json: serde_json::to_string(&pulse.payload).unwrap(),
                was_expected: false,
            }))
            .tag("pulse.topic", &pulse.topic.0)
            .tag("pulse.correlation_id", 
                pulse.correlation_id.map(|c| c.to_hex()).unwrap_or_default())
            .build()
            .unwrap();
        substrate.insert(engram).unwrap();
    },
);
```

---

## See Also

- [`01-specification.md`](01-specification.md) — Pulse struct
- [`03-graduation-rules.md`](03-graduation-rules.md) — when to graduate
- [`05-today-vs-planned.md`](05-today-vs-planned.md) — EventBus today vs. Pulse target
