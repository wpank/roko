# Phase 3A: Feeds — Continuous Data Streams as Cells

## What Is a Feed?

A Feed is a Cell that:
1. **Connects** to an external data source (Connect protocol)
2. **Watches** for new data (Trigger protocol)
3. **Stores** snapshots when needed (Store protocol)
4. **Publishes** data as Pulses on the Bus

Feeds are a specialization, not a new primitive. They compose existing protocols.

## Why Feeds?

Without Feeds, external data integration is ad-hoc. Each agent that needs external
data manages its own HTTP client, polling loop, error handling, etc.

With Feeds:
- External data enters the system through a uniform interface
- Agents subscribe to Bus topics, not HTTP endpoints
- Rate limiting, caching, error handling are in the Feed, not the consumer
- Feeds are composable (derived feeds transform raw feeds)

## Feed Trait

```rust
/// A continuous data stream that publishes to the Bus.
///
/// Feeds implement Cell (they have identity, cost, execute) and additionally
/// provide lifecycle methods for subscription management.
#[async_trait]
pub trait Feed: Cell {
    /// The Bus topic this feed publishes to.
    fn topic(&self) -> &Topic;

    /// Kind of feed: Raw, Derived, Composite, Meta.
    fn feed_kind(&self) -> FeedKind;

    /// Start producing data. Called by Engine when feed is activated.
    async fn start(&self, ctx: &CellContext) -> Result<()>;

    /// Stop producing data.
    async fn stop(&self) -> Result<()>;

    /// Poll for latest value (no subscription needed).
    async fn poll(&self) -> Result<Option<Signal>>;

    /// Current status.
    async fn status(&self) -> Result<FeedStatus>;
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum FeedKind {
    Raw,       // Direct from external source
    Derived,   // Computed from one or more raw feeds
    Composite, // Cross-domain aggregation
    Meta,      // Feed about feeds (health monitoring)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedStatus {
    pub connected: bool,
    pub rate_hz: f64,
    pub last_update_ms: Option<i64>,
    pub error: Option<String>,
    pub signals_produced: u64,
}
```

## Example Implementations

### 1. File Watcher Feed (simplest, wire first)

```rust
struct FileWatchFeed {
    path: PathBuf,
    topic: Topic,
    // ...
}

impl Feed for FileWatchFeed {
    fn topic(&self) -> &Topic { &self.topic }
    fn feed_kind(&self) -> FeedKind { FeedKind::Raw }

    async fn start(&self, ctx: &CellContext) -> Result<()> {
        // Use notify::RecommendedWatcher (already in roko-cli TUI)
        // Publish Pulse on each file change
    }
}
```

**Wire target**: Replace the TUI file watcher with a FileWatchFeed.

### 2. Provider Health Feed (useful, demonstrates derived feed)

```rust
struct ProviderHealthFeed {
    providers: Vec<ProviderConfig>,
    topic: Topic,
    poll_interval: Duration,
}

impl Feed for ProviderHealthFeed {
    fn topic(&self) -> &Topic { &self.topic }
    fn feed_kind(&self) -> FeedKind { FeedKind::Meta }

    async fn start(&self, ctx: &CellContext) -> Result<()> {
        // Poll each provider on interval
        // Publish health status as Pulses
    }
}
```

**Wire target**: `roko serve` health endpoint subscribes to this feed.

## Wiring Plan

### Step 1: Add Feed trait to roko-core (1 hour)

Add to `traits.rs` alongside the other protocols.

### Step 2: Implement FileWatchFeed (2-3 hours)

The TUI already uses `notify::RecommendedWatcher`. Extract the watcher logic into
a Feed implementation.

### Step 3: Wire into `roko serve` (1-2 hours)

Register feeds with the Engine. The serve runtime subscribes to feed topics and
exposes them via SSE/WebSocket.

### Step 4: Add CLI command `roko feed list/status` (1 hour)

Show registered feeds and their status.

## What NOT to Do

- Don't build paid feeds or marketplace integration. That's Phase 5+.
- Don't build blockchain feeds. That requires chain runtime (Phase 2+).
- Don't over-abstract: start with 2 concrete feeds, extract patterns later.
