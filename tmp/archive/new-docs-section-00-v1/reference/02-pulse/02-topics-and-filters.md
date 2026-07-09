# Pulse — Topics and Filters

> Topic is a Pulse routing key. TopicFilter is a subscription matcher that selects Pulses by topic pattern.

**Status**: Specified  
**Crate**: `roko-core` (planned)  
**Depends on**: [Specification](01-specification.md)  
**Last reviewed**: 2026-04-19

> **Target state — no code yet.**

---

## TL;DR

Topics are hierarchical routing keys (e.g. `"prediction.error.high"`). Subscribers
register `TopicFilter`s that match by exact name, prefix, or wildcard. The Bus routes
each Pulse to every subscriber whose filter matches the Pulse's topic.

---

## Specification

### Topic

```rust
<!-- source: crates/roko-core/src/pulse.rs (target state) -->

/// A routing key for a Pulse.
/// Hierarchical, dot-separated. Example: "prediction.error.high"
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Topic(pub String);

impl Topic {
    /// Parse a topic string. Returns error if invalid.
    /// Valid: alphanumeric segments separated by dots. No empty segments.
    pub fn parse(s: &str) -> Result<Self, TopicError>;

    /// The top-level namespace (first segment).
    pub fn namespace(&self) -> &str;

    /// All segments.
    pub fn segments(&self) -> &[&str];
}
```

**Convention for standard topics:**

| Topic | Meaning |
|-------|---------|
| `prediction.error.high` | High prediction error from heartbeat |
| `prediction.error.low` | Low prediction error |
| `gate.pass` | Gate passed |
| `gate.fail` | Gate failed |
| `agent.output` | Agent produced output |
| `tool.complete` | Tool call completed |
| `substrate.gc` | Substrate garbage collection ran |
| `taint.propagated` | Taint propagated to N Engrams |
| `health.degraded` | Subsystem health degraded |
| `health.restored` | Subsystem health restored |

### TopicFilter

```rust
<!-- source: crates/roko-core/src/pulse.rs (target state) -->

/// A subscription predicate over Topics.
#[derive(Clone, Debug)]
pub enum TopicFilter {
    /// Match a single exact topic.
    Exact(Topic),
    /// Match all topics with this prefix.
    Prefix(String),
    /// Match all topics.
    All,
    /// Match by predicate.
    Custom(Box<dyn Fn(&Topic) -> bool + Send + Sync>),
}

impl TopicFilter {
    pub fn matches(&self, topic: &Topic) -> bool { /* ... */ }
}
```

### Subscription Registration

```rust
<!-- source: crates/roko-core/src/bus.rs (target state) -->

trait Bus {
    /// Register a subscriber for all Pulses matching `filter`.
    fn subscribe<P, F>(
        &self,
        filter: TopicFilter,
        handler: F,
    ) -> SubscriptionHandle
    where
        P: 'static,
        F: Fn(Pulse<P>) + Send + Sync + 'static;

    /// Unregister a subscription.
    fn unsubscribe(&self, handle: SubscriptionHandle);
}
```

---

## Routing Algorithm

The Bus routes a Pulse to subscribers as follows:

```
for each subscriber in subscriptions:
    if subscriber.filter.matches(pulse.topic):
        deliver pulse to subscriber.handler
```

Order of delivery is unspecified (parallel delivery may be implemented).

---

## Examples

### Subscribe to all gate events:

```rust
<!-- source: crates/roko-core/src/bus.rs (target state) -->

let handle = bus.subscribe(
    TopicFilter::Prefix("gate.".to_string()),
    |pulse: Pulse<GateEvent>| {
        if !pulse.payload.passed {
            tracing::warn!("Gate failure: {}", pulse.payload.gate_name);
        }
    },
);
```

### Subscribe to a specific topic:

```rust
let handle = bus.subscribe(
    TopicFilter::Exact(Topic::parse("prediction.error.high")?),
    |pulse: Pulse<PredictionErrorEvent>| {
        // Escalate to T2 cognitive speed
    },
);
```

---

## Invariants

1. Topic segments must be non-empty alphanumeric strings
2. `TopicFilter::Exact(t).matches(&t)` is always true
3. `TopicFilter::All.matches(_)` is always true
4. A Pulse is delivered to all matching subscribers, not just the first

---

## Open Questions

- Should topics be versioned? (e.g., `prediction.error.high/v2`)
- Should there be a dead-letter topic for unrouted Pulses?

---

## See Also

- [`01-specification.md`](01-specification.md) — Pulse struct
- [`07-open-questions.md`](07-open-questions.md) — unresolved routing questions
