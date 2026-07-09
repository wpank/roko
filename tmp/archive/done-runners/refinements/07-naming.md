# Naming Decisions

> **TL;DR**: Keep `Engram` for the durable record. For the ephemeral
> message use **`Pulse`**. Reclaim the name `Signal` only if we really
> want to — it's on the table but has downsides. Don't use `Event` alone
> (too generic). The transport trait is `Bus`. The topic identifier is
> `Topic`.

## 1. What we know for sure

**`Engram` stays.** The rename Signal → Engram is already done in code
(877 occurrences vs. 5 for Signal). The name has neuroscience
lineage (Semon 1904, Lashley, Tonegawa), and `docs/00-architecture/02-engram-data-type.md`
has a solid justification paragraph for it. Don't touch this.

**`Bus` is the transport trait.** Short, standard in systems-programming
vocabulary, mirrors `Substrate`'s shape (both are storage/transport
fabrics). Not controversial.

**`Topic` is the routing handle.** Matches pub/sub convention across
NATS, Kafka, MQTT, AMQP. Not controversial.

The only real decision is what to call the medium that flows on the
Bus.

## 2. The contenders

### 2.1 `Pulse` — recommended

**Pro**:
- Neuro-coherent with Engram. An engram is a persistent memory trace;
  a pulse is the instantaneous signal that deposits it. The cognitive
  metaphor extends naturally.
- Not currently used anywhere in the Roko codebase (grep
  `\bpulse\b` in `crates/` returns clippy lints for unrelated code,
  nothing load-bearing).
- Communicates ephemerality. "Pulse" implies something that fires
  and passes. "Event" and "Signal" don't.
- Short (5 letters). Good for common type name.
- No namespace collision with popular Rust crates.

**Con**:
- Mildly unusual for an event/message type. Developers coming from
  other pub/sub systems will wonder if it's something special.
- The word has prior art in distributed systems (Apache Pulsar,
  PulseAudio, HeartbeatPulse) but none of those are Rust-idiomatic
  collisions.

**Verdict**: Best choice. Neurocognitive coherence wins; the
novelty is actually an asset for a framework that wants a
distinctive vocabulary.

### 2.2 `Signal` (reclaiming)

**Pro**:
- The name is literally free: the rename Signal → Engram moved the
  old type out. Reclaiming `Signal` for the transient event matches
  the original *engineering* meaning of the word (Shannon
  information, POSIX signals, electrical signals).
- No new word to learn.

**Con**:
- **This is the problem.** Roko's docs and commit history still carry
  heavy "Signal = Engram" content. Reclaiming `Signal` for a
  different meaning in the same codebase will confuse readers for
  years. Every old doc that says "Signal" will now be wrong in a
  subtly different way.
- The doc 23 audit and the "Signal ↔ Engram" naming glossary at
  `docs/00-architecture/01-naming-and-glossary.md` explicitly call
  the two concepts "the same" — reclaiming Signal contradicts that
  statement without erasing it from history.
- Grep will become painful. `rg Signal` will return both old legacy
  references and new transient-event references indistinguishably.

**Verdict**: Don't. The naming-cleanup cost exceeds the benefit.

### 2.3 `Event`

**Pro**:
- Universally understood. Every pub/sub system in the world has
  `Event`.
- Easy to onboard external developers.

**Con**:
- Collides with tokio, winit, and every other Rust crate that has
  an Event type. Imports become noisy.
- Generic — doesn't carry Roko-specific meaning. The framework's
  distinctive vocabulary gets diluted.
- Doesn't pair as well with `Engram` (Engram is specific to Roko;
  Event is specific to nothing).

**Verdict**: Workable fallback if `Pulse` is vetoed, but weaker.

### 2.4 `Message`

**Pro**: Clear, neutral.
**Con**: Also highly colliding. Same problems as `Event`, with the
added issue that `Message` in LLM contexts already means a chat
message.
**Verdict**: No.

### 2.5 `Envelope`

**Pro**: Matches the existing `Envelope<E>` name in `roko-runtime`.
**Con**: An Envelope *wraps* a payload. It's the container, not the
thing. If we kept this name, we'd still need a name for the payload.
**Verdict**: Keep `Envelope` as an internal implementation detail if
desired, but don't use it as the user-facing type name.

### 2.6 `Spark`

**Pro**: Also neuro/firing-coherent; evocative.
**Con**: Already famous as a product name (Apache Spark). Would
trigger unwanted associations.
**Verdict**: No.

### 2.7 `Beat`, `Chime`, `Signal`, `Ping`, `Impulse`

Other candidates considered and rejected on either the collision
or the evocation axis. `Impulse` would work but is too long and too
physics-coded; `Ping` is too network-coded; `Beat` is too music-coded.

## 3. Recommendation

- **Durable**: `Engram`
- **Ephemeral**: `Pulse`
- **Storage trait**: `Substrate`
- **Transport trait**: `Bus`
- **Routing handle**: `Topic`
- **Filter**: `TopicFilter`
- **Sequence id**: `u64` (per-bus, per-topic)
- **Either-medium enum**: `Datum<'a> { Engram(&'a Engram), Pulse(&'a Pulse) }`

The `Signal` name stays retired — it was the old name for Engram, it
stays that way, and we don't reclaim it.

## 4. Module naming

- `crates/roko-core/src/engram.rs` — exists, unchanged
- `crates/roko-core/src/pulse.rs` — new
- `crates/roko-core/src/topic.rs` — new
- `crates/roko-core/src/traits.rs` — extend with `Bus`
- `crates/roko-core/src/datum.rs` — new (defines the `Datum` enum)
- `crates/roko-core/src/topics/mod.rs` — new (canonical topic string constants)
  - `topics/orchestration.rs`
  - `topics/agent.rs`
  - `topics/gate.rs`
  - `topics/safety.rs`
  - `topics/conductor.rs`
  - `topics/substrate.rs`
  - `topics/chain.rs` (Phase 2+)
  - `topics/mesh.rs` (Phase 2+)

## 5. File name for the core docs

- `docs/00-architecture/02a-engram-durable-record.md` (rename from
  `02-engram-data-type.md`, redirect preserved)
- `docs/00-architecture/02b-pulse-ephemeral-event.md` (new)
- `docs/00-architecture/07a-substrate-storage-fabric.md` (rename
  from `07-substrate-trait.md`)
- `docs/00-architecture/07b-bus-transport-fabric.md` (new)

## 6. One-line summary that uses all the names

> Roko's kernel has two mediums (**Engram** — durable, content-addressed,
> decayed; **Pulse** — ephemeral, topic-addressed, sequenced) moving
> through two fabrics (**Substrate** — storage; **Bus** — transport),
> acted on by six operators (**Scorer**, **Gate**, **Router**,
> **Composer**, **Policy**, and the fabric traits themselves). Data
> has lineage via the Engram DAG; events have ordering via Bus
> sequence numbers. Pulses graduate to Engrams when their lineage
> matters.

Every bolded term is a type or trait in `roko-core` after Phase B.

## 7. Topic-namespace registry

The canonical topic names in `roko-core::topics`. Using the constants
(not string literals) lets the compiler catch typos and gives rustdoc
a home for each topic's documentation.

```rust
// roko-core/src/topics/mod.rs
pub mod agent;
pub mod orchestration;
pub mod gate;
pub mod safety;
pub mod conductor;
pub mod substrate;
pub mod consensus;       // from 13-collective-intelligence-c-factor.md
pub mod prediction;      // from 10-self-learning-cybernetic-loops.md
pub mod chain;           // Phase 2+, from 09-phase-2-implications.md
pub mod mesh;            // Phase 2+, from 09-phase-2-implications.md
```

Example of one submodule:

```rust
// roko-core/src/topics/gate.rs
//! Gate-related topic strings.

/// Emitted once per Gate verdict in the pipeline.
/// Body: `Body::Json({ "gate": "compile|test|clippy|...", "passed": bool, "reason": string })`.
pub const VERDICT_EMITTED: &str = "gate.verdict.emitted";

/// Emitted when a Gate pipeline fails overall.
pub const PIPELINE_FAILED: &str = "gate.pipeline.failed";

/// Emitted by a learning Policy computing rolling failure rate.
/// Body: `Body::Json({ "rate": f64, "window_ms": i64 })`.
pub const FAILURE_RATE: &str = "gate.failure.rate";

/// Emitted by a `ConsistencyGate` when an agent output looks
/// semantically disconnected from its cited supporting Engrams.
/// Body: `Body::Json({ "distance": f64, "threshold": f64 })`.
pub const HALLUCINATION_DETECTED: &str = "gate.hallucination.detected";
```

Topic constants are `pub const` strings, not typed wrappers, so they
can be used directly in the `Topic::new(...)` calls throughout the
codebase:

```rust
use roko_core::topics::gate;
bus.publish(Pulse {
    topic: Topic::new(gate::VERDICT_EMITTED),
    ...
}).await?;
```

A future `Topic` newtype could carry stronger validation (the dot-separated
lowercase convention in §2.1 of `02-engram-vs-pulse.md`). Punt that until
the registry has 100+ topics and reader confusion becomes real.

## 8. Reserved topic prefixes

Some prefixes have ecosystem-wide meaning. Plugins and third-party
extensions should avoid them unless they truly fit:

| Prefix | Owner | Meaning |
|---|---|---|
| `orchestration.*` | `roko-orchestrator` | Plan / task lifecycle |
| `agent.*` | `roko-agent`, `roko-agent-server` | Agent process, turn, token events |
| `gate.*` | `roko-gate` | Gate pipeline verdicts and stats |
| `safety.*` | `roko-agent/safety` | Permission, approval, taint, attestation |
| `conductor.*` | `roko-conductor` | Health, circuit breakers, watchers |
| `substrate.*` | Any Substrate impl | `engram.stored`, `engram.pruned`, `thaw.requested` |
| `consensus.*` | `13-collective-intelligence-c-factor.md` | HDC-bundle consensus |
| `prediction.*`, `outcome.*`, `error.*` | `10-self-learning-cybernetic-loops.md` | Active-inference triples |
| `heartbeat.*` | Clock | Three-speed ticks (gamma, theta, delta) |
| `chain.*` | `roko-chain` (Phase 2+) | On-chain event forwarding |
| `mesh.*` | `roko-mesh` (Phase 2+) | Inter-agent pub/sub |
| `plugin.*` | Plugin registry | Install, enable, disable events |
| `ui.*` | `roko-cli`, `roko-serve` | UI-local state refresh |

Third-party plugins should namespace under their own ID:
`org.example.my_gate.verdict` rather than reusing `gate.*`. This is the
same rule as Kubernetes label keys.

## 9. Deprecated / avoided names

To keep the rename audit honest, this is the list of names we
deliberately avoid using — with reasons — so future maintainers
don't accidentally resurrect them:

| Avoided | Why | Prefer |
|---|---|---|
| `Signal` for an ephemeral event | Collides with the old `Signal → Engram` rename history | `Pulse` |
| `Event` as the primary type name | Collides with tokio/winit/many crates | `Pulse` |
| `Message` | Overloaded with LLM chat messages | `Pulse` (wire), `ChatMessage` (LLM-specific) |
| `Signal` for the durable record | Legacy; renamed to Engram | `Engram` |
| `EventBus<E>` as the trait | Ad-hoc generic; not canonical | `Bus` + `Pulse` |
| `Topic<String>` | Roko uses `Topic` (wraps `String`) | `Topic` |
| `Channel` | Too generic; used by tokio for different things | `Topic` |
| `Envelope` | Sounds like the container, not the thing | `Pulse` |
| `Grimoire` | Previous project name for Neuro | `Neuro` |
| `Styx` | Previous project name for mesh | `Mesh` |
| `Clade` | Previous project name for agent roster | `Fleet` |
| `Bardo`, `Golem`, `Mori` | Previous project codename heritage | `Roko` |

`34-glossary.md` carries the same table plus definitions; this
section is the reason log.
