# StateHub Rearchitecture

> **TL;DR**: Today's "StateHub" is a TUI-specific subscription
> mechanism — it pipes runtime state to the ratatui renderer.
> But what the system actually needs is a *universal projection
> layer*: any consumer (TUI, web UI, external dashboard, audit
> log, analytics backend) can subscribe to a typed, query-able,
> live-updating view over the Bus + Substrate. This doc proposes
> promoting StateHub from a TUI helper to a kernel subsystem with
> projection contracts, subscription filters, and a stable wire
> format. After this refactor, the TUI is just one client of
> many.

> **For first-time readers**: A "projection" in this doc is a named,
> typed, live-updating view over the Bus + Substrate. Think:
> `cohort_health` (gauge numbers + roster), `active_tasks` (live list
> with progress), `gate_pipeline` (pass/fail counts per rung). Each
> projection has a canonical name, a `State` type (full shape), a
> `Delta` type (incremental update), and a folding function. Any
> consumer — TUI, web UI, Slack bot, Grafana — subscribes to the same
> projections over the same wire protocol. Read 03 (Bus) and 27
> (realtime surface) first; StateHub sits between them.

## 1. What StateHub is today

Located in `crates/roko-cli/src/tui/`. Function: maintain a
cached, react-style state tree that the TUI reads from on each
frame. Fed by events from the Bus and the Substrate, debounced,
formatted for display.

Observations:

- **TUI-coupled**: its data shapes reflect what the TUI wants to
  show, not what's semantically true about the system.
- **In-process**: lives in the same binary as the TUI; no way for
  an external consumer to use the same projections.
- **Ad-hoc formats**: fields added as the TUI needs them.
- **Single consumer**: assumes one reader.
- **No filter language**: consumers get everything or nothing.

None of these are bugs — they're appropriate for the TUI. But
they foreclose a general pattern the system needs.

## 2. What StateHub should become

A kernel subsystem (new crate: `roko-statehub`) with these
properties:

1. **Multi-consumer**: many clients subscribe simultaneously;
   deliveries are back-pressured independently.
2. **Typed projections**: named views that are strongly typed on
   both wire and reader side.
3. **Filterable**: a subscription can scope to a topic, a
   lineage, a role, a user, a time range.
4. **Queryable**: a client can request the current state *and*
   subscribe to updates (the classic `query + subscribe`
   pattern).
5. **Transport-agnostic**: the same projections flow over
   in-process channels, WebSocket, SSE, gRPC — whatever the
   consumer needs.
6. **Replayable**: a consumer can reconnect and catch up from a
   known position without losing events.
7. **Auditable**: every projection update is trace-linked to the
   Engram/Pulse that caused it.

This is MVC's model layer done well, where the M is a living
projection over durable + ephemeral fabrics.

## 3. Projection as a first-class type

```rust
pub trait Projection: Send + Sync + 'static {
    /// Unique identifier for the projection.
    const NAME: &'static str;

    /// The type clients receive.
    type State: Serialize + DeserializeOwned + Clone + Send + 'static;

    /// The type clients receive as an incremental update.
    type Delta: Serialize + DeserializeOwned + Clone + Send + 'static;

    /// Fold a Delta into a State.
    fn apply(state: &mut Self::State, delta: Self::Delta);

    /// Which Bus topics this projection cares about.
    fn topics() -> &'static [&'static str];

    /// Compute the initial State from historical Engrams + recent Pulses.
    async fn hydrate(ctx: &ProjectionContext) -> Result<Self::State>;

    /// Compute a Delta from an event.
    fn reduce(event: &Event) -> Option<Self::Delta>;
}
```

A `Projection` is defined once. Any client can subscribe by name
and receive a `State` followed by a stream of `Delta`. The
projection is computed once per running Roko instance and fanned
out; clients don't each have to re-derive.

## 4. Canonical projections

Ship at least these with the kernel:

| Name | State shape | Use |
|---|---|---|
| `cohort_health` | c-factor, agent roster, turn stats | dashboards |
| `active_tasks` | running tasks, progress, ETAs | live status |
| `gate_pipeline` | rung status, pass/fail counts | CI-like view |
| `recent_episodes` | last N episodes with summary | TUI list |
| `heuristic_library` | calibration histogram, top hits | beliefs view |
| `cost_meter` | spend per model per role, live | budget dashboards |
| `bus_stats` | pulses/sec by topic, delivery rate | ops |
| `substrate_stats` | balance histogram, tier sizes | memory health |
| `agent_trails` | per-agent timeline, current action | chat/trace UI |
| `replication_ledger` | claim status table | research |

Each projection's State and Delta types are in `roko-statehub`
so all consumers share vocabulary. New projections can be added
later; names are namespaced (`org.example.custom_view`).

## 5. Subscription filters

Different consumers want different slices:

```rust
pub struct Subscription {
    pub projection: ProjectionName,
    pub filter: ProjectionFilter,
    pub cursor: Option<Cursor>,
    pub delivery: DeliveryMode,
}

pub enum ProjectionFilter {
    All,
    User(PrincipalId),
    Tenant(TenantId),
    Role(Role),
    Lineage(EngramHash),
    Topic(TopicPattern),
    TimeRange { start: Timestamp, end: Option<Timestamp> },
    Custom(Box<dyn FilterFn>),
}

pub enum DeliveryMode {
    AtMostOnce,       // lossy; fine for UIs
    AtLeastOnce,      // retry on ack timeout
    Exactly(Cursor),  // resume from a position
}
```

Filters execute server-side. A web client subscribing to just its
user's episodes doesn't pay the bandwidth cost of other users'.

## 6. Queryable APIs

Every projection also exposes a one-shot query for the current
state:

```
GET /projections/cohort_health
GET /projections/cohort_health?filter=tenant:acme
GET /projections/recent_episodes?filter=user:me&limit=20
```

And a subscription:

```
GET /projections/cohort_health/stream
  (WebSocket or SSE upgrade)
```

The `query + stream` split maps to whatever the client prefers.
REST-shaped clients get REST. Event-shaped clients get the
stream. A React app usually uses both: query on mount, subscribe
after.

## 7. Wire format

JSON by default. Protobuf/MessagePack/CBOR as opt-ins. The
message envelope:

```json
{
  "projection": "cohort_health",
  "cursor": "0x1a2b...",
  "kind": "state" | "delta",
  "timestamp": "2026-04-16T12:00:00Z",
  "payload": { /* State or Delta */ }
}
```

Every message carries its cursor so a reconnecting client can
resume. Cursors are monotonic per-projection.

## 8. The local-first path

In-process consumers (TUI) should not pay serialization cost.
The API should offer a typed in-process subscription that
returns typed State/Delta directly:

```rust
let mut sub = statehub.subscribe::<CohortHealth>(Filter::all()).await?;
let state: CohortHealthState = sub.initial().await?;
while let Some(delta) = sub.next().await {
    // ...
}
```

The out-of-process path serializes; the in-process path doesn't.
Same contract.

## 9. StateHub and the Bus

StateHub is *not* the Bus. They have different jobs:

- **Bus**: raw event transport. Low-level. Many topics.
- **StateHub**: typed, folded, filterable views on top.

StateHub *subscribes* to Bus topics and *publishes* projection
events. An external consumer rarely wants raw Bus pulses; they
want "the current state of X, with updates." StateHub is the
abstraction that serves that want.

In a large deployment, one Roko instance computes the
projections; the Bus can remain in-process or cluster-backed
depending on scale. StateHub cleanly sits in-between.

## 10. Access control

StateHub subscriptions respect tenant and role:

- A tenant sees only their own projections (filter
  automatically scoped).
- A role sees only projections permitted for it (a low-privilege
  role can't see cost data).
- External clients authenticate with API keys or OIDC.
- Subscriptions have per-tenant rate limits.

This fits the multi-tenancy story in `24` §8.

## 11. Migration from current TUI StateHub

The TUI-specific StateHub becomes a thin client of the new
kernel subsystem.

1. Build new `roko-statehub` crate with the `Projection` trait
   and in-process API.
2. Re-home the TUI's current state tree as a set of
   `Projection` impls.
3. Wire the TUI to subscribe in-process.
4. Add the HTTP/WS/SSE transport layer in `roko-serve`.
5. Deprecate the old ad-hoc StateHub.

Two weeks of work for steps 1–4. Step 5 can take longer; keep
the old shim until all consumers have migrated.

## 12. Extension points

Third parties can define new projections via the plugin system
(`17`):

```rust
pub struct MyProjection;

impl Projection for MyProjection {
    const NAME: &'static str = "org.example.my_projection";
    // ...
}

roko_statehub::register::<MyProjection>();
```

This lands at tier 4 (native) with an eventual tier-5 (WASM)
path. Plugin projections respect the same access control and
appear in the registry.

## 13. Snapshot and replay

A projection's State is effectively a CRDT-like snapshot. For
debugging and testing:

- `statehub.snapshot(projection)` writes the current State to
  an Engram.
- `statehub.restore(projection, engram)` rebuilds the projection
  from the snapshot and catches up from the Bus.
- `statehub.replay(projection, from=cursor, to=cursor)` replays
  the projection over a historical range.

This turns "what was the state of X at time T?" into a one-liner.
Valuable for postmortems, audits, and tutorials.

## 14. Performance considerations

- **Delta coalescing**: if a projection emits deltas faster than
  consumers can drink, server-side coalesce them.
- **Selective materialization**: projections that nobody is
  currently subscribing to stop computing after a grace period
  and rehydrate on demand.
- **Shared computation**: two subscribers to the same projection
  with the same filter share the computed stream.
- **Incremental hydration**: `hydrate()` should return quickly by
  using indexed summaries rather than full rescans.

None of these are premature; at a real deployment scale they
matter.

## 15. The shape of what this enables

After StateHub exists:

- A web UI is a set of projection subscriptions plus views.
- An external dashboard is the same, plus auth.
- A Slack bot subscribing to `gate_pipeline` can post when a
  gate fails — in 30 lines of Go.
- An audit log is a projection persisted continuously to S3.
- A Grafana data source is a StateHub adapter.

All of these exist without core changes because the subscription
contract is stable. The ecosystem expands without kernel
involvement.

## 16. Why this is the key refactor for UX

Every UX improvement in docs 23, 27, 28, 29, 30 depends on
real-time, consistent, typed state reaching the UI. Without
StateHub rearchitecture, each UX surface reinvents the state
pipeline. With it, they share.

This is the architectural linchpin for the second half of the
UX story. Building TUIs, Web UIs, and external integrations
without it produces five parallel reimplementations of the same
plumbing. Building it once produces a platform.

## 17. Worked example: `cohort_health` end to end

The `cohort_health` projection exposes c-factor (13) and agent
activity. Implementation sketch:

```rust
// roko-statehub/src/projections/cohort_health.rs
use roko_core::{Pulse, Topic};

pub struct CohortHealth;

#[derive(Clone, Serialize, Deserialize)]
pub struct CohortHealthState {
    pub c_factor: f64,
    pub agent_roster: Vec<AgentSummary>,
    pub turn_taking_entropy: f64,
    pub peer_prediction_accuracy: f64,
    pub citation_reciprocity: f64,
    pub delivery_rate: f64,
    pub hdc_diversity: f64,
    pub window_ms: i64,
}

#[derive(Clone, Serialize, Deserialize)]
pub enum CohortHealthDelta {
    AgentJoined(AgentSummary),
    AgentLeft(String),
    MetricsUpdated {
        c_factor: f64,
        turn_taking_entropy: f64,
        peer_prediction_accuracy: f64,
        citation_reciprocity: f64,
        delivery_rate: f64,
        hdc_diversity: f64,
    },
}

impl Projection for CohortHealth {
    const NAME: &'static str = "cohort_health";
    type State = CohortHealthState;
    type Delta = CohortHealthDelta;

    fn topics() -> &'static [&'static str] {
        &[
            "cohort.metrics.updated",
            "agent.process.spawned",
            "agent.process.exited",
        ]
    }

    fn apply(state: &mut Self::State, delta: Self::Delta) {
        match delta {
            CohortHealthDelta::AgentJoined(a) => state.agent_roster.push(a),
            CohortHealthDelta::AgentLeft(id) => {
                state.agent_roster.retain(|a| a.id != id);
            }
            CohortHealthDelta::MetricsUpdated {
                c_factor, turn_taking_entropy, peer_prediction_accuracy,
                citation_reciprocity, delivery_rate, hdc_diversity,
            } => {
                state.c_factor = c_factor;
                state.turn_taking_entropy = turn_taking_entropy;
                state.peer_prediction_accuracy = peer_prediction_accuracy;
                state.citation_reciprocity = citation_reciprocity;
                state.delivery_rate = delivery_rate;
                state.hdc_diversity = hdc_diversity;
            }
        }
    }

    async fn hydrate(ctx: &ProjectionContext) -> Result<Self::State> {
        // Read the latest cohort metrics Engram from Substrate;
        // compute initial roster by scanning agent process Engrams.
        let latest = ctx.substrate
            .query(Predicate::kind(Kind::CohortMetrics).limit(1))
            .await?;
        let roster = ctx.substrate
            .query(Predicate::kind(Kind::ProcessSpawn).since(now_ms() - 3600_000))
            .await?;
        Ok(CohortHealthState {
            c_factor: extract_c_factor(&latest),
            agent_roster: build_roster(&roster),
            ..Default::default()
        })
    }

    fn reduce(event: &Event) -> Option<Self::Delta> {
        match event {
            Event::Pulse(p) if p.topic.as_str() == "cohort.metrics.updated" => {
                Some(CohortHealthDelta::MetricsUpdated { /* parse body */ })
            }
            Event::Pulse(p) if p.topic.as_str() == "agent.process.spawned" => {
                Some(CohortHealthDelta::AgentJoined(parse_agent(p)))
            }
            Event::Pulse(p) if p.topic.as_str() == "agent.process.exited" => {
                Some(CohortHealthDelta::AgentLeft(parse_id(p)))
            }
            _ => None,
        }
    }
}
```

A consumer subscribes:

```rust
let mut sub = statehub.subscribe::<CohortHealth>(ProjectionFilter::All).await?;
let state: CohortHealthState = sub.initial().await?;
println!("c-factor: {}", state.c_factor);
while let Some(delta) = sub.next().await {
    CohortHealth::apply(&mut state, delta);
    render_dashboard(&state);
}
```

Every consumer — TUI, web, Slack — uses the same `CohortHealth::apply`
function. No duplication.

## 18. Projection lifecycle and testing

Each projection needs three lifecycle validations:

1. **Hydration test**: given a frozen Substrate snapshot, the
   hydrated State matches the expected snapshot.
2. **Delta-fold equivalence**: given an initial State + a sequence
   of Pulses, folding deltas produces the same State as rehydrating
   after the Pulses were all persisted.
3. **Cursor resumption**: disconnect mid-stream, reconnect with
   cursor, the resulting State matches continuous subscription.

Framework: `roko-statehub::testing` ships fixtures for each. Custom
projections inherit the same three tests via a macro.

## 19. Cross-references

- Bus trait this consumes: `03-bus-as-first-class.md`.
- Realtime wire format that serializes Delta/State:
  `27-realtime-event-surface.md`.
- Web UI that consumes projections:
  `29-web-ui-architecture.md`.
- TUI's migration from ad-hoc StateHub to projection client:
  `23-user-ux-running-agents.md` §5 (TUI becomes interactive).
- Third-party projections as plugins:
  `17-plugin-extension-architecture.md` §2.4.
- c-factor measurement feeding `cohort_health`:
  `13-collective-intelligence-c-factor.md`.
- Snapshot + replay depends on demurrage's thaw mechanism:
  `12-knowledge-demurrage.md` §7.
