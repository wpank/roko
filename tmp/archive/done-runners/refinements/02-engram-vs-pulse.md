# Two Mediums: Engram (Durable) and Pulse (Ephemeral)

> **TL;DR**: Keep the Engram exactly as it is — the content-addressed,
> lineage-bearing, decayed, provenance-stamped record. Introduce a sibling
> type, **Pulse**, for the in-flight message. Define a clean conversion
> law so Pulses can graduate into Engrams when their lineage matters.

> **For first-time readers**: An **Engram** today is a Roko Rust struct —
> hashed by BLAKE3 over its `kind`/`body`/`author`/`tags`, decayed on a schedule,
> scored along 7 axes, chained by lineage to parent Engrams. It is Roko's one
> existing data type. A **Pulse** (this proposal) is its sibling: a typed,
> sequence-numbered, brief message that lives in an event-bus ring buffer
> and delivers once. This doc names the Pulse, lists which fields it has and
> doesn't, and defines when a Pulse should "graduate" into an Engram.

## 1. The split

| Property | Engram | Pulse |
|---|---|---|
| Identity | `ContentHash` (BLAKE3 over kind + body + author + tags) | `(topic, seq)` within a Bus; no global hash |
| Durability | Persisted in a `Substrate` | Lives in a Bus ring buffer; drops when ring wraps |
| Lineage | `Vec<ContentHash>` — audit DAG | Optional `lineage_hint: Option<ContentHash>` pointing at an Engram |
| Decay | `Decay` enum (HalfLife, Ttl, Ebbinghaus, None) | N/A — Pulses are instantaneous |
| Score | `Score` (7-axis appraisal) | N/A — Pulses may be scored in flight but don't carry a score |
| Provenance | Full `Provenance` (author, trust, taint, attestation) | Lightweight: `source: String`, topic implies author class |
| Attestation | Optional Ed25519 / chain attestation | None |
| Typical rate | 1 Hz – 1 kHz (plans, tasks, verdicts, episodes) | 1 Hz – 1 MHz (heartbeats, tokens, stream chunks) |
| Typical consumer | Scorer, Gate, Composer, Substrate | Policy, TUI, HTTP subscribers, sidecar, dashboards |
| Examples | Plan, Task, AgentOutput (final), GateVerdict, Episode, Playbook, Insight, Heuristic, Pheromone, Prediction, Attestation | ProcessSpawn, ProcessExit, AgentMessage chunk, TokenUsage tick, ApprovalRequested, GateVerdictInFlight, ContextUpdated, HeartbeatTick, CancellationRequested, UiRefresh |

The split is *not* between important and unimportant data. It's between
**data that needs to be auditable forever** and **data that needs to be
delivered right now and maybe remembered briefly**.

## 2. The Pulse type

Proposed shape, mirroring `Envelope<E>` in `roko-runtime` but canonicalized:

```rust
/// An in-flight event traveling on a Bus.
///
/// Pulses are typed, sequence-numbered, timestamped messages. They are
/// not content-addressed and are not persisted by default. A Pulse may
/// carry an optional lineage hint pointing at an Engram whose
/// ContentHash contextualizes it.
///
/// Pulses may be "graduated" to Engrams by a Policy via
/// `Pulse::graduate(provenance, decay) -> Engram`. This is the ONLY
/// path from transport into audit-DAG.
#[derive(Clone, Debug)]
pub struct Pulse {
    /// Topic-local monotonic sequence. Unique per (bus, topic) pair.
    pub seq: u64,
    /// Topic string, e.g. "gate.verdict" or "agent.msg.chunk".
    pub topic: Topic,
    /// Kind — reused from Engram. Same taxonomy.
    pub kind: Kind,
    /// Payload — reused from Engram. Same taxonomy.
    pub body: Body,
    /// Unix milliseconds when the Pulse was published.
    pub emitted_at_ms: i64,
    /// Lightweight source attribution (component name, agent id, etc.).
    pub source: PulseSource,
    /// Optional Engram reference that gives context for this Pulse.
    /// E.g. an AgentMessage Pulse may reference the Task Engram it
    /// belongs to.
    pub lineage_hint: Option<ContentHash>,
    /// Optional trace id for distributed tracing. Not part of identity.
    pub trace_id: Option<TraceId>,
}
```

### 2.1 Topics

Topics are strings with a recommended hierarchy, like OpenTelemetry
span names:

```
orchestration.plan.started
orchestration.task.ready
agent.msg.chunk
agent.process.spawned
agent.process.exited
agent.tokens.used
gate.verdict.emitted
gate.pipeline.failed
safety.approval.requested
safety.taint.propagated
conductor.circuit.tripped
conductor.health.degraded
ui.refresh.requested
chain.transaction.confirmed    (Phase 2+)
mesh.pheromone.deposited       (Phase 2+)
```

Topics are the contract surface between subsystems. They replace today's
ad-hoc `OrchestrationEvent`, `AgentEvent`, `UiEvent` enums with a single
string-namespaced taxonomy.

### 2.2 Why reuse `Kind` and `Body`?

The Engram's `Kind` enum in `crates/roko-core/src/kind.rs` already
enumerates the semantic categories of the system (ProcessSpawn,
AgentMessage, GateVerdict, TokenUsage, …). Reusing it for Pulses means a
Pulse and an Engram that describe the same event have the same `kind`
and `body`, which makes graduation trivially an identity function plus
some extra fields.

This also means existing code that dispatches on `Kind` continues to
work: a Policy that reacts to `Kind::GateVerdict` Pulses is the
*same* Policy that reads `Kind::GateVerdict` Engrams from storage
during replay.

## 3. The conversion law

Graduation is the well-defined path from Pulse to Engram:

```rust
impl Pulse {
    /// Graduate this Pulse into an Engram suitable for Substrate storage.
    ///
    /// The caller supplies provenance (author, trust, taint chain) and
    /// a decay policy. Lineage is carried forward from `lineage_hint`.
    ///
    /// The resulting Engram's ContentHash is computed from
    /// (kind, body, provenance.author, tags), as usual. If two Pulses
    /// graduate with identical content they produce identical
    /// Engrams — deduplication is automatic.
    pub fn graduate(
        &self,
        provenance: Provenance,
        decay: Decay,
        score: Score,
        tags: BTreeMap<String, String>,
    ) -> Engram {
        let lineage = self.lineage_hint.clone().into_iter().collect();
        EngramBuilder::new(self.kind.clone(), self.body.clone())
            .created_at_ms(self.emitted_at_ms)
            .provenance(provenance)
            .decay(decay)
            .score(score)
            .lineage(lineage)
            .tags(tags)
            .build()
    }
}
```

The reverse — Engram to Pulse — is a lossy projection (loses score,
decay, lineage vector):

```rust
impl Engram {
    /// Project this Engram onto a Pulse for broadcast.
    ///
    /// Used when a stored Engram needs to be announced to live
    /// subscribers (e.g. replay-on-resume, or dashboard updates).
    pub fn to_pulse(&self, topic: Topic, seq: u64, source: PulseSource) -> Pulse {
        Pulse {
            seq,
            topic,
            kind: self.kind.clone(),
            body: self.body.clone(),
            emitted_at_ms: self.created_at_ms,
            source,
            lineage_hint: Some(self.id.clone()),
            trace_id: None,
        }
    }
}
```

### 3.1 Graduation policy

**Not every Pulse should graduate.** Heartbeat ticks, UI refresh
requests, and intermediate token-usage samples have no lineage value
and should die in the ring buffer. Good defaults:

| Pulse topic | Graduate? | Reason |
|---|---|---|
| `orchestration.plan.started` | Yes | Plan lifecycle belongs in DAG |
| `orchestration.task.ready` | No | Redundant with Task Engram already in Substrate |
| `agent.msg.chunk` | Batch-graduate on stream close | Individual chunks are noise |
| `agent.process.spawned/exited` | Yes | Process lifecycle is forensic |
| `agent.tokens.used` | Aggregate then graduate | Per-chunk is noise; per-turn is useful |
| `gate.verdict.emitted` | Yes | Verdicts are the core audit record |
| `safety.approval.requested` | Yes | Safety events must be auditable |
| `conductor.circuit.tripped` | Yes | Health events are forensic |
| `ui.refresh.requested` | No | UI-local |
| `heartbeat.tick` | No | Clock pulses are infrastructure |

This table itself should live in `roko-core` as a `GraduationPolicy`
default implementation, overridable via config.

## 4. Why this split is safer than it looks

### 4.1 It matches what the code already does

`roko-agent-server` already publishes WebSocket token-chunk events that
are not Engrams. `roko-orchestrator` already emits `OrchestrationEvent`
on a bus. `roko-runtime` already defines `Envelope<E>`. We are
*naming* what exists, not inventing a second type system.

### 4.2 It preserves the Engram invariants

The forensic AI capability and the content-addressed DAG depend on
Engrams being *exactly* what they are today: hashed, lineage-bearing,
decayed. Pulses don't weaken this — they just stop forcing ephemeral
events into the hashed DAG when they don't need to be there.

### 4.3 It lets the Substrate remain the only persistence surface

Pulses aren't persisted. If a Pulse matters long-term, it graduates to
an Engram and goes to the Substrate. There is still exactly one
storage surface, with one audit model. We added a transport surface
alongside it, which is what we have been calling the event bus all
along.

### 4.4 It doesn't break Policy

Policy's signature changes from:

```rust
fn decide(&self, stream: &[Engram], ctx: &Context) -> Vec<Engram>;
```

to:

```rust
fn decide(&self, stream: &[Pulse], ctx: &Context) -> PolicyOutputs;
// where PolicyOutputs = { pulses: Vec<Pulse>, engrams: Vec<Engram> }
```

Existing Policy implementations that were doing
`Policy::decide(&[], ctx)` with synthetic empty Engram streams
(doc 23 calls this "awkward but functional") can now emit their
metric Pulses cleanly. Policies that want to react to stored Engrams
can still do so — they subscribe to a `substrate.*` topic that the
Substrate emits when Engrams land.

## 5. Worked example — agent turn

Current state, one-noun model:

1. Agent subprocess spawns → ad-hoc `AgentEvent::ProcessSpawned` on bus.
2. Token chunks arrive → ad-hoc `AgentEvent::StreamChunk` on bus.
3. Turn completes → `Engram { kind: AgentOutput, body: Text(...) }`
   written to Substrate.
4. GatePipeline verifies the Engram → `Engram { kind: GateVerdict }`
   written to Substrate.
5. TUI polls Substrate for new Engrams (doc 24 flags this as the
   P0 "polling-vs-streaming" bug).

Two-medium model:

1. Agent subprocess spawns → publish `Pulse { topic:
   "agent.process.spawned", kind: ProcessSpawn, ... }`. Policy
   graduates it to an Engram (process lifecycle is forensic).
2. Token chunks arrive → publish `Pulse { topic: "agent.msg.chunk", ... }`
   at 10–100 Hz. Not graduated. TUI subscribes, renders incrementally.
3. Turn completes → publish `Pulse { topic: "agent.turn.completed", ... }`
   AND graduate to `Engram { kind: AgentOutput }` written to
   Substrate.
4. GatePipeline runs → publishes `Pulse { topic:
   "gate.verdict.emitted", ... }` AND graduates to
   `Engram { kind: GateVerdict }`.
5. TUI never polls — it subscribes to `gate.verdict.*`,
   `agent.msg.*`, `orchestration.*` topics. The P0 bug dissolves.

The agent turn is the same turn. We just stopped forcing every
heartbeat and token into the hashed DAG, and we stopped making the TUI
poll a database.

## 6. Things this does not answer

- **Does Pulse need a hash at all?** For replay determinism across a
  restart, maybe a lightweight non-content-addressed id helps. Or we
  accept that replay is best-effort until a Pulse graduates. See
  `09-phase-2-implications.md` for the chain-replay implications.
- **What about backpressure?** Pulses are broadcast, not queued. If a
  subscriber is slow, it misses. This is the same model `tokio::sync::broadcast`
  uses. For critical subscribers, they can graduate the Pulse and
  subscribe to the Substrate.
- **Who owns topic names?** They need a registry. A `roko-core::topics`
  module with `const TOPIC_AGENT_MSG_CHUNK: &str = "agent.msg.chunk"`
  declarations is the minimum; a richer `Topic` newtype with validation
  is better. `07-naming.md` §4 proposes the file layout for this.
- **How large should Pulse bodies get?** Small is good: Pulses fan out
  to all subscribers, so cost is *O(body × subscribers)*. A 5 MB token
  stream slice on a topic with 50 subscribers is 250 MB of copies per
  publish. Rule of thumb: bodies under 64 KB for hot topics (`agent.msg.chunk`),
  under 1 MB for structural topics (`orchestration.plan.started`). If it
  has to be bigger, put the payload in a Substrate Engram and let the
  Pulse carry only a `lineage_hint` pointing at it.

## 7. What else changes if Pulse lands

Adopting Pulse ripples through four other proposals in this folder. Keep
these in mind when reviewing:

- **`10-self-learning-cybernetic-loops.md`** — every operator gets a
  prediction/outcome Pulse pair, which only works if Pulses are a
  first-class type. Without Pulse, active inference has nowhere to publish
  the prediction-error signal.
- **`12-knowledge-demurrage.md`** §2 — the `ReinforceKind::Retrieved` and
  `ReinforceKind::Surprised` signals ride on Pulses. Demurrage without
  Pulse either forces every read to write a new Engram (expensive) or
  skips reinforcement entirely (misses the point).
- **`13-collective-intelligence-c-factor.md`** §2.2 — every c-factor
  metric is computable from Bus statistics. Authorship entropy, delivery
  rate, peer-prediction accuracy — all are Pulse-level observations.
- **`26-statehub-rearchitecture.md`** — StateHub projections consume Bus
  subscriptions to build their live views. Without a typed Pulse the
  projection layer has to carry subsystem-specific enums through a
  generic bus, which is what `Envelope<E>` does today and why it's ad hoc.

## 8. Before / after cheat sheet

The same five lines of code, once without Pulse and once with:

```rust
// Before: ad hoc, per-crate enum lives on a generic broadcast channel.
use roko_orchestrator::OrchestrationEvent;
tx.send(OrchestrationEvent::PlanStarted { plan_id, started_at_ms });
```

```rust
// After: one Pulse shape, topic-addressed, contextually graduatable.
use roko_core::{Pulse, Topic, Kind, Body, PulseSource};
bus.publish(Pulse {
    seq: 0,                              // filled by bus at publish time
    topic: Topic::new("orchestration.plan.started"),
    kind: Kind::Plan,
    body: Body::Json(json!({ "plan_id": plan_id, "started_at_ms": started_at_ms })),
    emitted_at_ms: now_ms(),
    source: PulseSource { component: "roko-orchestrator".into(), agent_id: None },
    lineage_hint: Some(plan_hash),
    trace_id: None,
}).await?;
```

The "after" form is longer — but the verbosity is paying for four concrete
things: every subsystem in the workspace speaks the same vocabulary; the
TUI/web/Slack bot can subscribe without importing the orchestrator; demurrage
can reinforce the referenced plan_hash; and graduation is a two-line call
away when the Pulse should become a durable record.

See `07-naming.md` for whether to call this type `Pulse`, `Event`, or
reclaim `Signal`. See `08-code-sketches.md` for the `Pulse::graduate`
implementation and a full end-to-end test.
