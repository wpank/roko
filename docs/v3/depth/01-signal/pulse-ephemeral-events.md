# Pulse: Ephemeral Events

> Pulses are the live-wire counterpart to Signals. They flow through the Bus
> without persistence. Graduation converts a Pulse into a durable Signal.

**Source**: `crates/roko-core/src/pulse.rs`

---

## 1. Signal vs Pulse

| Property | Signal | Pulse |
|---|---|---|
| **Medium** | Store (durable) | Bus (ephemeral) |
| **Identity** | Content-hashed (`ContentHash`) | Sequence-numbered (`u64`) |
| **Persistence** | Written to `.roko/signals.jsonl` | Never persisted directly |
| **Score** | 7-axis `Score` | None |
| **Decay** | `Decay` function | None (no lifespan) |
| **Provenance** | Full `Provenance` record | None (inherits on graduation) |
| **Lineage** | `Vec<ContentHash>` parent chain | Optional `lineage_hint` back-reference |
| **Routing** | Kind-based dispatch | Topic-based subscription |
| **Lifecycle** | 4-tier graduation | Fire-and-forget or graduate |

Signals are the nouns of the system -- they accumulate over time, are
scored, decay, form audit DAGs, and survive restarts. Pulses are the
verbs -- they notify subscribers of real-time events and are discarded
after delivery unless explicitly graduated.

---

## 2. The Pulse Struct

```rust
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Pulse {
    pub seq:            u64,
    pub topic:          Topic,
    pub kind:           Kind,
    pub body:           Body,
    pub created_at_ms:  i64,
    pub tags:           BTreeMap<String, String>,
    pub lineage_hint:   Option<ContentHash>,
}
```

### Field Details

| Field | Type | Purpose |
|---|---|---|
| `seq` | `u64` | Monotonic sequence number assigned by the Bus. Ordering guarantee within a single Bus instance |
| `topic` | `Topic` | Dotted hierarchical routing key (e.g. `"gate.verdict.emitted"`, `"heartbeat.gamma.tick"`) |
| `kind` | `Kind` | Same `Kind` enum as Signal -- determines body interpretation |
| `body` | `Body` | Same `Body` enum as Signal -- the payload |
| `created_at_ms` | `i64` | Unix milliseconds. Defaults to now; can be pinned for tests |
| `tags` | `BTreeMap<String, String>` | Arbitrary metadata for filtering (ordered for stable serialization) |
| `lineage_hint` | `Option<ContentHash>` | Optional back-reference to a Signal. Set when projected via `Signal::to_pulse()` |

---

## 3. Topic and TopicFilter

### Topic

A `Topic` is a newtype over `String` with hierarchical matching:

```rust
pub struct Topic(pub String);

impl Topic {
    pub fn new(s: impl Into<String>) -> Self;
    pub fn starts_with(&self, prefix: &str) -> bool;
}
```

Topics follow a dotted hierarchy: `"gate.verdict.emitted"`,
`"heartbeat.gamma.tick"`, `"episode.logged"`. Subscribers filter by
topic prefix.

### TopicFilter

Subscription filters for Bus consumers. Six variants forming a
Boolean algebra:

```rust
pub enum TopicFilter {
    Exact(Topic),              // match one specific topic
    Prefix(String),            // match all topics with this prefix
    All,                       // match everything
    And(Vec<TopicFilter>),     // conjunction (all must match)
    Or(Vec<TopicFilter>),      // disjunction (any must match)
    Not(Box<TopicFilter>),     // negation
}
```

Edge cases: `And(vec![])` is vacuous truth (matches everything).
`Or(vec![])` is false (matches nothing).

---

## 4. Construction

### Direct

```rust
let p = Pulse::new(1, Topic::new("gate.verdict"), Kind::GateVerdict, Body::text("pass"));
```

### Builder

```rust
let p = Pulse::builder(10, Topic::new("gate.compile"), Kind::GateVerdict)
    .body(Body::text("pass"))
    .created_at_ms(12345)
    .tag("plan_id", "plan-1")
    .tag("gate", "compile")
    .lineage_hint(signal_hash)
    .build();
```

Builder defaults: `Body::Empty`, current time, empty tags, no lineage hint.

---

## 5. Graduation: Pulse to Signal

`Pulse::graduate()` is the deliberate promotion path -- the only way to get
a Pulse into the durable Store with a full audit trail.

```rust
pub fn graduate(
    &self,
    provenance: Provenance,
    initial_balance: f64,
    score: Score,
    tags: Vec<String>,
) -> Signal;
```

### What graduation preserves and adds

| Preserved from Pulse | Added by graduation |
|---|---|
| `kind` | `provenance` (caller-supplied) |
| `body` | `score` (caller-supplied) |
| `created_at_ms` | `balance` (caller-supplied) |
| All existing `tags` | `status = Working` (already past Transient) |
| | Tag `"pulse_topic"` = topic string |
| | Tag `"pulse_seq"` = sequence number |
| | Extra label tags from `tags` param as `key = "true"` |

### Graduation vs synthetic promotion

There are two ways to convert a Pulse to a Signal:

1. **`Pulse::graduate()`** -- full audit trail with explicit provenance,
   score, balance, and audit tags. Content hash differs from synthetic
   due to the extra audit tags. This is the production path.

2. **`Signal::from_pulse_synthetic()`** -- lossy helper for quick scoring.
   No audit tags, no explicit provenance/score/balance. Marked as
   `"pulse_promotion"` provenance with `Decay::None`. Used for internal
   scoring operations that need a Signal temporarily.

---

## 6. Projection: Signal to Pulse

`Signal::to_pulse()` projects a durable Signal into an ephemeral Pulse
for Bus broadcast. This is the inverse direction.

```rust
pub fn to_pulse(&self, topic: Topic, seq: u64) -> Pulse;
```

The projection intentionally drops durable-only metadata: score, balance,
decay, fingerprint, attestation, and the full lineage DAG. Only the
essential content (kind, body, tags) and a `lineage_hint` back-reference
cross over. The Signal's author is added as a `"signal_author"` tag.

---

## 7. PolicyOutputs

The explicit dual-channel output from a `React` policy's `decide()` call:

```rust
pub struct PolicyOutputs {
    pub pulses: Vec<Pulse>,    // publish on Bus for immediate reactions
    pub signals: Vec<Signal>,  // persist via Store for durability
}
```

Methods: `empty()`, `with_pulse()`, `with_signal()`, `is_empty()`, `len()`.
`Default` returns empty. This struct makes the two output channels
explicit rather than conflating them.

---

## 8. Bus Implementations

| Bus | Behavior | Use case |
|---|---|---|
| `BroadcastBus` | Live-only delivery, no replay | Production runtime |
| `MemoryBus` | Bounded replay buffer | Testing, catch-up subscriptions |
| `MultiBus` | Fans out through a `MemoryBus` primary | Composed delivery |

All Bus implementations deliver Pulses by topic filter. The Bus trait
is defined in `crates/roko-core/src/traits.rs`.

---

## 9. Verification Commands

```bash
# Run Pulse tests
cargo test -p roko-core pulse -- --nocapture

# Run graduation tests
cargo test -p roko-core graduation -- --nocapture

# Run TopicFilter tests
cargo test -p roko-core filter -- --nocapture
```

---

## 10. Source Files

| File | Contents |
|---|---|
| `crates/roko-core/src/pulse.rs` | Pulse, PulseBuilder, Topic, TopicFilter, PolicyOutputs |
| `crates/roko-core/src/signal.rs` | `Signal::to_pulse()`, `Signal::from_pulse_synthetic()`, `Signal::from_pulses()` |
| `crates/roko-core/src/traits.rs` | Bus trait (`subscribe`, `publish`) |
| `crates/roko-core/src/bus_backends.rs` | BroadcastBus, MemoryBus, MultiBus, BusErased |
