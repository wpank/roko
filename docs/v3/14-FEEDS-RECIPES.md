# 14 -- Feeds and Recipes

> Cell-composed runtime data streams and pure-data scoring DAGs.
> Feeds connect external sources to the Bus. Recipes transform feed
> data through deterministic Score pipelines. Both compose from existing
> kernel primitives with no new types.

**Depends on**: [01-SIGNAL](01-SIGNAL.md) (Signal/Pulse, Bus topics),
[02-CELL](02-CELL.md) (Cell trait, ProtocolId), [03-GRAPH](03-GRAPH.md)
(DAG execution), [13-TELEMETRY](13-TELEMETRY.md) (Lens observation of feed
health)

**Implementation status (2026-09-15):** E27 is **complete (10/10)**.
Runtime `FeedCell` composition, bounded lifecycle supervision with
exponential-backoff reconnect, three built-in serve-time feeds,
canonical Bus Pulse routing via `FeedBusBridge`, recipe DAG
validation/evaluation, atomic TOML recipe persistence, REST
lifecycle/discovery/recipe APIs, CLI commands, and integration tests
are all shipped. Deferred product work includes on-chain ERC-8004
feed advertisement, paid remote subscription routing, recipe template
packs, backtesting, and dashboard authoring.

### Authoritative sources

| Surface | Source file |
|---|---|
| Feed descriptors and registry | `crates/roko-core/src/feed.rs` |
| Feed Cell runtime primitives | `crates/roko-core/src/feed_cell.rs` |
| Feed Bus bridge | `crates/roko-core/src/feed_bus_bridge.rs` |
| Runtime supervision registry | `crates/roko-core/src/feed_runtime.rs` |
| Recipe DAG types and evaluator | `crates/roko-core/src/recipe.rs` |
| Recipe TOML store | `crates/roko-core/src/recipe_store.rs` |
| Feed REST routes | `crates/roko-serve/src/routes/feeds.rs` |
| Recipe REST routes | `crates/roko-serve/src/routes/recipes.rs` |
| Feed CLI commands | `crates/roko-cli/src/commands/feed.rs` |
| Recipe CLI commands | `crates/roko-cli/src/commands/recipe.rs` |
| Built-in feed registration | `crates/roko-serve/src/state.rs` |

---

## 1. Feed Primitive

A **Feed** is a Cell specialization that connects to an external data
source (Connect protocol), watches for events (Trigger protocol), and
optionally persists snapshots (Store protocol). Feeds are the
"always-on" complement to one-shot Cell queries. The architecture is
domain-agnostic: the same primitives serve file-change streams, provider
health monitors, episode outcome tails, and any other continuous data
source.

**Cross-reference**: [depth/14-feeds/01-feed-sources.md](depth/14-feeds/01-feed-sources.md)

### 1.1 Kernel Decomposition

```
Feed = Cell + Connect + Trigger + Store(optional)

  Cell provides:    cell_id, cell_name, cell_version, protocols, execute
  Connect provides: connect, query, disconnect, health
  Trigger provides: listen, filter, debounce, fire
  Store provides:   put, get  (optional -- some feeds are ephemeral)
```

A `FeedCell` declares its protocol conformances at construction time:

```rust
fn protocols(&self) -> Vec<ProtocolId> {
    let mut protocols = vec![ProtocolId::Connect, ProtocolId::Trigger];
    if self.store.is_some() {
        protocols.push(ProtocolId::Store);
    }
    protocols
}
```

### 1.2 FeedCell

`FeedCell` is the runtime concrete type. It owns three object-safe trait
objects -- `ConnectorOps`, `FeedTriggerOps`, and optional `StoreOps` --
behind `Arc<dyn ...>`. This keeps the lifecycle manager generic: it
supervises heterogeneous feeds without knowing their implementation.

```rust
pub struct FeedCell {
    cell_id:    String,
    cell_name:  String,
    version:    CellVersion,
    connector:  Arc<dyn ConnectorOps>,
    trigger:    Arc<dyn FeedTriggerOps>,
    store:      Option<Arc<dyn StoreOps>>,
    config:     FeedCellConfig,
    status:     Arc<RwLock<FeedStatus>>,
    output:     broadcast::Sender<FeedPulse>,
    emitted:    AtomicU64,
    last_emitted_at_ms: AtomicU64,
}
```

Key methods:

| Method | What it does |
|---|---|
| `connect()` | Transitions to `Connecting`, delegates to `ConnectorOps::connect()`, transitions to `Connected` or `Degraded` |
| `listen(cancel)` | Runs the trigger loop: filters raw events, applies debounce, persists to optional store, publishes `FeedPulse` to the broadcast channel |
| `subscribe()` | Returns a `broadcast::Receiver<FeedPulse>` for downstream consumers |
| `disconnect()` | Tears down the connector and transitions to `Disconnected` |
| `health()` | Queries connector health and synchronizes the feed lifecycle state |
| `execute(input, ctx)` | Cell trait implementation: runs an on-demand `query()` against the connector |

### 1.3 Feed Lifecycle States

```
Idle --> Connecting --> Connected --> Degraded
  ^                        |             |
  |                        v             v
  +-------- Disconnected <-+-------------+
```

```rust
pub enum FeedStatus {
    Idle,          // Constructed but not connected
    Connecting,    // Connection attempt in progress
    Connected,     // Listening for source events
    Degraded,      // Running but source is unhealthy
    Disconnected,  // Explicitly stopped or post-error
}
```

### 1.4 FeedPulse

Every accepted source event becomes a `FeedPulse` -- the typed event
published by a running feed.

```rust
pub struct FeedPulse {
    pub feed_id:      String,              // Feed instance that emitted the event
    pub topic:        String,              // Source-level topic
    pub sequence:     u64,                 // Monotonic per-feed sequence
    pub payload:      serde_json::Value,   // Structured event data
    pub emitted_at_ms: i64,               // Unix timestamp in ms
    pub metadata:     BTreeMap<String, String>,  // Optional source metadata
}
```

The trigger-to-output pipeline inside `FeedCell::listen()`:

1. The trigger fires raw events through a private broadcast channel.
2. The listen loop receives each event, increments `emitted`, updates
   `last_emitted_at_ms`.
3. If a `StoreOps` is configured, the pulse payload is persisted with
   key `{feed_id}:{sequence}`.
4. The pulse is forwarded to the public `output` broadcast channel
   where subscribers and the Bus bridge receive it.

### 1.5 Feed Kinds

```rust
pub enum FeedKind {
    Raw,       // Unprocessed source data (file changes, log lines)
    Derived,   // Computed from one or more raw feeds (moving average)
    Composite, // Assembled from multiple derived feeds (portfolio risk)
    Meta,      // Metadata about other feeds (health, accuracy tracking)
}
```

### 1.6 Feed Access and Pricing

Feeds have three access levels:

```rust
pub enum FeedAccess {
    Public,   // Readable by any agent
    Private,  // Restricted to the producing agent and explicit subscribers
    Paid,     // Requires payment to access
}
```

Paid feeds carry a `FeedPricingConfig` with reputation-derived pricing
tiers:

```rust
pub struct FeedPricingConfig {
    pub tier:             PricingTier,
    pub per_request_cost: f64,
    pub session_pricing:  Option<SessionPricing>,
    pub protocol:         PaymentProtocol,
}

pub enum PricingTier {
    Free,          // multiplier: 0.0x
    Starter,       // multiplier: 0.5x
    Standard,      // multiplier: 1.0x
    Professional,  // multiplier: 1.5x
    Enterprise,    // multiplier: 2.0x
}
```

Session pricing supports metered longer-lived connections with
`base_rate_per_minute`, `burst_rate`, `max_session_cost`, and
`settlement_interval_secs`.

**Note on payment protocols**: `FeedPricingConfig` declares both `X402`
and `Mpp` as payment protocol variants. The x402 blockchain settlement
path is deprecated at the transport level; pricing tiers and
per-request/session cost fields remain as the commercial vocabulary for
feed access control regardless of settlement mechanism.

---

## 2. Feed Registry and Discovery

Two registries serve different purposes:

| Registry | Purpose | Location |
|---|---|---|
| `FeedRegistry` | Static descriptor catalog (serializable metadata) | `roko-core/src/feed.rs` |
| `RuntimeRegistry` | Live supervision of running feed Cells | `roko-core/src/feed_runtime.rs` |

### 2.1 Static FeedRegistry

The `FeedRegistry` is an in-memory vector of `FeedInfo` entries with a
monotonic ID counter. It supports:

- `register(info) -> String` -- assigns a unique `feed-{n}` ID
- `unregister(id) -> bool` -- removes by ID
- `get(id) -> Option<&FeedInfo>` -- lookup
- `list() -> &[FeedInfo]` -- all feeds
- `list_by_kind(kind)` -- filtered by `FeedKind`
- `list_by_agent(agent_id)` -- filtered by producing agent
- `search(query)` -- substring match across name and description

### 2.2 RuntimeRegistry

The `RuntimeRegistry` manages live feed Cells with bounded reconnect
supervision. It provides:

- **Factory registration**: `register(info, build_fn)` stores a
  descriptor alongside a closure that constructs the feed Cell.
- **Discovery**: `discover()` returns all registered descriptors in
  stable ID order. `search(query)` filters by ID, name, description,
  or kind.
- **Lifecycle**: `start_registered(id)` constructs and supervises a
  feed. `stop(id)` cancels and awaits teardown. `stop_all()` attempts
  cooperative shutdown of every active feed.
- **Health**: `health()` returns `FeedRuntimeStatus` for all
  discoverable feeds, including stopped ones.

### 2.3 Supervised Reconnection

Each running feed gets a tokio task with exponential backoff:

```rust
pub struct ReconnectPolicy {
    pub initial: Duration,  // default: 1s
    pub maximum: Duration,  // default: 60s
}
```

The supervision loop:

1. If cancelled, break.
2. Call `connect()`. On success, reset delay to `initial`; on failure,
   record error.
3. Call `listen(cancel)` until it returns or fails.
4. Call `disconnect()`.
5. Sleep for `delay`, then double it up to `maximum`.

This ensures feeds recover from transient source failures without
manual intervention and without unbounded retry storms.

**Cross-reference**: [depth/14-feeds/04-lifecycle.md](depth/14-feeds/04-lifecycle.md)

---

## 3. Bus Bridging

**Cross-reference**: [depth/14-feeds/02-bus-bridge.md](depth/14-feeds/02-bus-bridge.md)

The `FeedBusBridge` routes every `FeedPulse` from a feed Cell's broadcast
channel into the canonical Bus as a typed Pulse.

### 3.1 Topic Convention

Every feed pulse is published to:

```
feed:{feed_id}:data
```

For example, `file-watch-roko-dir` publishes to
`feed:file-watch-roko-dir:data`.

### 3.2 Bridge Implementation

```rust
pub struct FeedBusBridge<B: Bus + 'static> {
    bus:      Arc<B>,
    counters: Arc<RwLock<HashMap<String, Arc<FeedRouteStats>>>>,
}
```

The `route_pulse` method:

1. Looks up or creates per-feed `FeedRouteStats`.
2. Constructs a `Pulse` with `Kind::Custom("feed.data")`.
3. Tags the Pulse with `feed_id`, `source_topic`, and
   `source_sequence`.
4. Publishes via `bus.publish(pulse)`.
5. Increments `pulses_routed` and `bytes_routed` counters.

### 3.3 Forwarding Loop

`FeedBusBridge::spawn(receiver)` starts a tokio task that:

- Receives `FeedPulse` events from a `broadcast::Receiver`.
- Routes each through `route_pulse`.
- On `RecvError::Lagged`, skips lost messages (bounded degradation).
- On `RecvError::Closed`, exits cleanly.

### 3.4 Per-Feed Routing Metrics

```rust
pub struct FeedRouteStats {
    pub pulses_routed:     AtomicU64,
    pub bytes_routed:      AtomicU64,
    pub last_routed_at_ms: AtomicU64,
}
```

These counters are queryable per feed via `bridge.stats(feed_id)`.

---

## 4. Built-In Source Feeds

Roko ships three serve-time built-in feeds registered during `AppState`
construction. All three are `Private` access and produced by the
`"system"` agent.

**Cross-reference**: [depth/14-feeds/01-feed-sources.md](depth/14-feeds/01-feed-sources.md)

### 4.1 file-watch-roko-dir

| Property | Value |
|---|---|
| **ID** | `file-watch-roko-dir` |
| **Kind** | `Raw` |
| **Topic** | `fs.changed` |
| **Description** | Debounced changes below `.roko`, excluding partial writes |

Uses a `notify::RecommendedWatcher` to detect file system events under
the workspace `.roko/` directory. Raw events are filtered by a predicate
that ignores partial writes and temporary files, then debounced before
emission. The connector is a `NoopConnector` (the file-system watcher
does not require a network handshake).

### 4.2 provider-health-feed

| Property | Value |
|---|---|
| **ID** | `provider-health-feed` |
| **Kind** | `Meta` |
| **Topic** | `provider.health` |
| **Description** | Canonical model provider circuit health |

Reuses the observed circuit-breaker health snapshot from the provider
health registry. Each polling interval, it reads the latest circuit
state for all configured LLM providers and emits a summary pulse. This
feed deliberately does **not** create shadow HTTP probes -- it reads
existing health observations.

### 4.3 episode-outcome-feed

| Property | Value |
|---|---|
| **ID** | `episode-outcome-feed` |
| **Kind** | `Raw` |
| **Topic** | `episode.outcome` |
| **Description** | New append-only agent episode outcomes |

Tails the canonical `.roko/episodes.jsonl` log. Each new line appended
to the log is parsed and emitted as a feed pulse. This feed exists so
downstream consumers (dashboards, meta-feeds, recipe pipelines) can
react to agent episode completions in near-real-time without polling
the JSONL file directly.

---

## 5. Connector and Trigger Contracts

### 5.1 ConnectorOps

The object-safe async trait for external source management:

```rust
#[async_trait]
pub trait ConnectorOps: Send + Sync {
    async fn connect(&self) -> Result<()>;
    async fn query(&self, query: &str) -> Result<Value>;
    async fn disconnect(&self) -> Result<()>;
    async fn health(&self) -> ConnectorStatus;
}
```

Built-in implementations:

| Struct | Use case |
|---|---|
| `NoopConnector` | In-memory or adaptor feeds with no external handshake |

### 5.2 FeedTriggerOps

The object-safe async trait for source event production:

```rust
#[async_trait]
pub trait FeedTriggerOps: Send + Sync {
    async fn listen(
        &self,
        feed_id: &str,
        output: broadcast::Sender<FeedPulse>,
        cancel: CancellationToken,
    ) -> Result<()>;

    fn filter(&self, event: &Value) -> bool;
    fn debounce_ms(&self) -> u64;
    fn fire(&self, feed_id: &str, sequence: u64, event: Value) -> FeedPulse;
}
```

Built-in implementations:

| Struct | Use case |
|---|---|
| `FeedTrigger` | Channel-backed trigger: bounded `mpsc` receiver with configurable filter and debounce. Useful for adapters, tests, and webhook-like feeds |
| `UnavailableTrigger` | Descriptor-only feeds that cannot start (returns a clear error on `listen`) |

### 5.3 StoreOps

Optional async persistence backend:

```rust
#[async_trait]
pub trait StoreOps: Send + Sync {
    async fn put(&self, key: &str, value: Value) -> Result<()>;
    async fn get(&self, key: &str) -> Result<Option<Value>>;
}
```

Built-in implementations:

| Struct | Use case |
|---|---|
| `MemoryFeedStore` | Bounded in-memory `HashMap<String, Value>` for tests and transient local feeds |

---

## 6. Recipes

A **Recipe** is a directed acyclic graph of deterministic Score
operations that transforms feed data into derived values. Recipes
involve no LLM inference and no agent dispatch -- they are pure data
pipelines.

**Cross-reference**: [depth/14-feeds/03-recipe-dags.md](depth/14-feeds/03-recipe-dags.md)

### 6.1 Why Recipes Are Distinct

| Concept | What It Is | Involves LLM? | Involves Agents? |
|---|---|---|---|
| **Plan** | Agent task DAG (tasks with dependencies, assigned to agents) | Yes | Yes |
| **Compose** | Prompt assembly for LLM calls (VCG auction, section effects) | Yes | Yes |
| **Recipe** | Pure data pipeline: Feed values through Score Cells | No | No |

Recipes make indicator chains, attribution pipelines, HDC encoding, and
scoring logic into composable, versionable, testable objects.

### 6.2 Core Types

```rust
pub struct Recipe {
    pub id:            String,
    pub name:          String,
    pub version:       u32,
    pub nodes:         Vec<RecipeNode>,
    pub edges:         Vec<RecipeEdge>,
    pub input_feeds:   Vec<String>,
    pub output_schema: Option<Value>,
}

pub struct RecipeNode {
    pub id:        String,
    pub operation: ScoreOp,
    pub params:    HashMap<String, f64>,
}

pub struct RecipeEdge {
    pub from:  String,   // Input feed or node identifier
    pub to:    String,   // Destination node identifier
    pub field: String,   // Optional field extracted from source value
}
```

### 6.3 Score Operations

Six built-in deterministic transforms plus a `Custom` escape hatch
(which fails closed -- the evaluator rejects unregistered custom
operations):

| Operation | Parameters | Description |
|---|---|---|
| `WeightedAverage` | Weight per source ID (default 1.0) | Weighted mean of incoming values |
| `Normalize` | `min`, `max` (default 0.0, 1.0) | Map input range to [0, 1] |
| `Threshold` | `threshold` (default 0.5) | Emit 1.0 if input >= threshold, else 0.0 |
| `ZScore` | `mean`, `stddev` (default 0.0, 1.0) | Standardize: `(value - mean) / stddev` |
| `Clamp` | `min`, `max` (default 0.0, 1.0) | Clamp value to range |
| `Rescale` | `from_min`, `from_max`, `to_min`, `to_max` | Linear mapping between ranges |
| `Custom(name)` | -- | Rejected at evaluation time with a clear error |

### 6.4 DAG Validation

`Recipe::validate()` checks:

1. **Non-empty ID** -- the recipe must have an identifier.
2. **Unique node IDs** -- no duplicate node identifiers.
3. **Unique input feeds** -- no duplicate input feed identifiers.
4. **Finite parameters** -- all numeric parameters must be finite.
5. **Edge reference integrity** -- every edge `from` must reference
   an existing node or input feed; every edge `to` must reference an
   existing node.
6. **Acyclicity** -- Kahn's algorithm (topological sort) must succeed.

Validation returns a `Vec<String>` of human-readable errors. An empty
vector means the recipe is valid.

### 6.5 Topological Evaluation

`Recipe::evaluate(inputs)` runs the DAG synchronously:

1. Validate the recipe (fail if invalid).
2. Verify all declared `input_feeds` are present in the `inputs` map.
3. Compute topological order via Kahn's algorithm.
4. For each node in order:
   - Gather incoming edge values (applying optional `field` extraction).
   - Convert each to `f64` (non-numeric values fail).
   - Execute the node's `ScoreOp` with its parameters.
   - Verify the output is finite (non-finite results fail).
   - Store the output for downstream nodes.
5. Identify sink nodes (nodes with no outgoing edges).
6. If one sink: return its value. If multiple sinks: return a JSON
   object keyed by sink ID.

### 6.6 Recipe TOML Persistence

The `RecipeStore` persists one TOML file per recipe under
`.roko/recipes/`:

```
.roko/recipes/
  blend.toml
  quality-score.toml
  risk-index.toml
```

Operations:

| Method | Behavior |
|---|---|
| `load(id)` | Read and parse `{id}.toml` |
| `save(recipe)` | Validate, auto-increment version if overwriting, atomic write |
| `list()` | Return sorted recipe IDs from directory listing |
| `delete(id)` | Remove the TOML file, return whether it existed |

**Path safety**: Recipe IDs are restricted to ASCII alphanumeric
characters, hyphens, and underscores (max 128 chars). Path traversal
attempts (e.g., `../secret`) are rejected.

### 6.7 Example Recipe

A weighted blend of two input feeds:

```toml
id = "blend"
name = "Blend"
version = 1
input_feeds = ["left", "right"]

[[nodes]]
id = "score"
operation = "WeightedAverage"

[nodes.params]
left = 1.0
right = 3.0

[[edges]]
from = "left"
to = "score"

[[edges]]
from = "right"
to = "score"
```

Evaluation with `left=2, right=6`:

```
score = (2 * 1.0 + 6 * 3.0) / (1.0 + 3.0) = 20/4 = 5.0
```

---

## 7. HTTP Routes

### 7.1 Feed Routes

All feed routes are under `/api/feeds` on the `roko serve` control
plane (default `:6677`).

#### Descriptor CRUD (static registry)

| Method | Path | Description |
|---|---|---|
| `GET` | `/api/feeds` | List feeds. Optional `?kind=` and `?agent_id=` query filters |
| `POST` | `/api/feeds` | Register a new feed descriptor |
| `GET` | `/api/feeds/{id}` | Get feed detail (enforces payment gate for paid feeds) |
| `DELETE` | `/api/feeds/{id}` | Unregister a feed descriptor |
| `GET` | `/api/feeds/catalog` | List built-in feed agents and descriptors |

#### Runtime lifecycle

| Method | Path | Description |
|---|---|---|
| `GET` | `/api/feeds/runtime` | List all runtime feeds with status |
| `GET` | `/api/feeds/runtime/{id}` | Detailed runtime status for one feed |
| `GET` | `/api/feeds/discover` | List every runnable built-in feed descriptor |
| `GET` | `/api/feeds/search?q=` | Search runtime feed metadata by substring |
| `GET` | `/api/feeds/health` | Aggregate runtime health including stopped feeds |
| `POST` | `/api/feeds/start/{id}` | Start a registered runtime feed and bridge it to Bus |
| `POST` | `/api/feeds/stop/{id}` | Cooperatively stop a running feed |

#### Paid feed gate

When `GET /api/feeds/{id}` targets a feed with `FeedAccess::Paid`, the
middleware checks for an `x-payment-authorization` header. Missing or
insufficient payment returns `402 Payment Required` with an
`x-payment-request` header containing a `PaymentRequest` JSON body.
Public and private feeds bypass the payment gate.

### 7.2 Recipe Routes

All recipe routes are under `/api/recipes`.

| Method | Path | Description |
|---|---|---|
| `GET` | `/api/recipes` | List persisted recipe IDs |
| `POST` | `/api/recipes` | Save a recipe (validates, auto-increments version) |
| `GET` | `/api/recipes/{id}` | Load a recipe definition |
| `DELETE` | `/api/recipes/{id}` | Delete a persisted recipe |
| `POST` | `/api/recipes/{id}/evaluate` | Evaluate a recipe with JSON input map |

#### Evaluate endpoint

`POST /api/recipes/{id}/evaluate` accepts a JSON body mapping input
feed names to numeric values:

```json
{
  "left": 2.0,
  "right": 6.0
}
```

Returns the recipe output value (scalar or keyed object for multi-sink
recipes).

---

## 8. CLI Commands

### 8.1 Feed Commands

All feed commands query a running `roko serve` instance. If the server
is not running, they report the error gracefully.

```
roko feed list        List all runtime feeds with their topics and status
roko feed status ID   Show detailed status for a specific feed
roko feed start ID    Start a discoverable runtime feed
roko feed stop ID     Stop a running feed
roko feed health      Show aggregate health for all discoverable feeds
roko feed discover    List feed types available to start
roko feed search Q    Search available feeds by id, name, description, or kind
```

**Environment variable**: `ROKO_SERVE_URL` overrides the default
`http://localhost:6677` base URL.

Example output from `roko feed list`:

```
ID                       TOPIC                            KIND       CONNECTED
--------------------------------------------------------------------------------
file-watch-roko-dir      fs.changed                       Raw        yes
provider-health-feed     provider.health                  Meta       yes
episode-outcome-feed     episode.outcome                  Raw        yes
```

Example output from `roko feed health`:

```
FEED                         STATUS            PULSES       RATE
------------------------------------------------------------------
file-watch-roko-dir          connected             42     0.07 Hz
provider-health-feed         connected             18     0.03 Hz
episode-outcome-feed         stopped                0     0.00 Hz
```

### 8.2 Recipe Commands

Recipe commands operate directly on the local `.roko/recipes/` directory
and do not require a running server.

```
roko recipe list                    List persisted recipe IDs
roko recipe show ID                 Show a recipe definition (TOML or JSON)
roko recipe validate ID             Validate a recipe DAG
roko recipe run ID --input K=V ...  Evaluate a recipe with literal inputs
```

Example:

```bash
$ roko recipe validate blend
recipe 'blend' is valid

$ roko recipe run blend --input left=2 --input right=6
5.0

$ roko recipe run blend --input left=2 --input right=6 --json
5.0
```

Input values are parsed as JSON first; if parsing fails, they are
treated as strings. This allows both `--input score=0.75` (parsed as
number) and `--input label=good` (kept as string).

---

## 9. Feed Data Flow

The complete data path from source to subscriber:

```
External Source           ConnectorOps              FeedTriggerOps
      |                       |                          |
      +-- connect() --------> |                          |
      |                       |                          |
      +-- raw events -------> | filter() + debounce ----> |
                              |                          |
                              |     FeedPulse            |
                              |       |                  |
                              |       v                  |
                         FeedCell::listen()              |
                              |                          |
                    +---------+---------+                |
                    |                   |                |
              StoreOps::put()    broadcast::send()       |
              (if configured)          |                 |
                                       v                 |
                              FeedBusBridge::spawn()     |
                                       |                 |
                                  Bus::publish()         |
                                       |                 |
                           +-----------+-----------+
                           |                       |
                     Local subscribers      SSE/WS routes
                     (in-process agents,    (remote dashboards,
                      recipe pipelines)      external consumers)
```

Key constraint: feed data flows exclusively through the broadcast
channel and Bus. There are no hidden staging maps or side channels.

---

## 10. Feed Composition

Feeds compose into value chains where each layer adds computation.

### 10.1 Type Hierarchy

```
Raw feeds (direct source ingestion)
  +-> Derived feeds (computed from one or more raw feeds)
       +-> Composite feeds (assembled from multiple derived feeds)
            +-> Meta feeds (metadata about other feeds)
```

### 10.2 Composition Example

```
file-watch-roko-dir (Raw, system, Private)
  +-> State change detector (Derived)
       +-> Workspace health composite (Composite)
            +-> System meta-feed (Meta)

provider-health-feed (Meta, system, Private)
  +-> Routing decisions (consumed by CascadeRouter)

episode-outcome-feed (Raw, system, Private)
  +-> Learning pipeline (consumed by PlaybookStore, EpisodeLogger)
```

### 10.3 Pricing Chains

For paid feeds, economics stack along the composition chain. A mid-chain
agent pays for input feeds and charges for output. With N subscribers:

```
Revenue = per_request_cost * tier_multiplier * N
Cost    = sum of input feed costs
Margin  = Revenue - Cost
```

---

## 11. Verification

### 11.1 Feed Tests

Feed infrastructure is verified by tests in:

- `crates/roko-core/src/feed.rs` -- registry operations, serde
  roundtrips, pricing tier multipliers, legacy descriptor compatibility
- `crates/roko-core/src/feed_cell.rs` -- channel trigger
  filter/debounce, protocol declaration
- `crates/roko-core/src/feed_bus_bridge.rs` -- Bus routing and byte
  accounting
- `crates/roko-core/src/feed_runtime.rs` -- supervised reconnection,
  bounded backoff, stop semantics
- `crates/roko-serve/src/routes/feeds.rs` -- REST CRUD, kind/agent
  filters, paid-feed payment gate (402 challenge/accept), runtime
  lifecycle (start/stop/health/search)

### 11.2 Recipe Tests

- `crates/roko-core/src/recipe.rs` -- weighted average evaluation,
  cycle detection, validation errors
- `crates/roko-core/src/recipe_store.rs` -- TOML round-trip, version
  auto-increment, path traversal rejection
- `crates/roko-serve/src/routes/recipes.rs` -- empty store listing
- `crates/roko-cli/src/commands/recipe.rs` -- input parsing
  (JSON literals vs. strings)

### 11.3 Test Counts

The E27 epic shipped with integration tests covering the full
lifecycle: feed registration, runtime start/stop, Bus bridging, recipe
CRUD, and recipe evaluation. All tests pass in the workspace CI gate
(`cargo test --workspace`).

---

## 12. Deferred Product Work

The following items from the broader feed/recipe roadmap are explicitly
deferred and not claimed by the E27 acceptance:

| Item | Status | Notes |
|---|---|---|
| On-chain ERC-8004 feed advertisement | Deferred | Requires deployed contracts and chain client |
| Paid remote subscription routing | Deferred | Relay transport + session management |
| Recipe template packs | Deferred | Reusable building blocks exist in `roko-learn` and `roko-primitives` |
| Recipe backtesting | Deferred | Requires Store historical replay |
| Recipe dashboard authoring UI | Deferred | Depends on named surfaces (E37) |
| Feed agent subscriptions in TOML | Deferred | `[[agent.feed_subscriptions]]` config parsing |
| WebSocket subscription routing | Deferred | Relay wire protocol scope |
| Cursor-based feed list pagination | Deferred | Current list is unbounded |

---

## 13. Crate Mapping

| Component | Crate | Status |
|---|---|---|
| Feed descriptors, registry, pricing (`FeedInfo`, `FeedRegistry`, `PricingTier`) | `roko-core` | Shipped (E27) |
| Feed Cell runtime (`FeedCell`, `ConnectorOps`, `FeedTriggerOps`, `StoreOps`) | `roko-core` | Shipped (E27) |
| Feed Bus bridge (`FeedBusBridge`, `FeedRouteStats`) | `roko-core` | Shipped (E27) |
| Runtime supervision (`RuntimeRegistry`, `FeedHandle`, `ReconnectPolicy`) | `roko-core` | Shipped (E27) |
| Recipe DAG (`Recipe`, `RecipeNode`, `RecipeEdge`, `ScoreOp`) | `roko-core` | Shipped (E27) |
| Recipe TOML store (`RecipeStore`) | `roko-core` | Shipped (E27) |
| Feed REST routes (14 endpoints) | `roko-serve` | Shipped (E27) |
| Recipe REST routes (5 endpoints) | `roko-serve` | Shipped (E27) |
| Feed CLI commands (7 subcommands) | `roko-cli` | Shipped (E27) |
| Recipe CLI commands (4 subcommands) | `roko-cli` | Shipped (E27) |
| Built-in feed registration (3 feeds) | `roko-serve` (state) | Shipped (E27) |
| Feed on-chain advertisement (ERC-8004) | `roko-chain` | Deferred |
| Recipe template pack | `roko-learn`, `roko-primitives` | Deferred |
| Recipe authoring UI | Dashboard | Deferred |
