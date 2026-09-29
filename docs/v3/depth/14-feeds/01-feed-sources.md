# depth/14-feeds/01 -- Feed Sources and Built-In Feeds

> Detail on the three built-in serve-time feeds, the ConnectorOps/
> FeedTriggerOps contracts, and how new source feeds are constructed.

**Parent**: [14-FEEDS-RECIPES.md](../../14-FEEDS-RECIPES.md) -- Sections 1, 4, 5

---

## 1. Built-In Feed Registration

The three built-in feeds are registered during `AppState::new()` in
`crates/roko-serve/src/state.rs`. Each registration pairs a `FeedInfo`
descriptor with a factory closure that constructs the `FeedCell` on
demand.

### 1.1 file-watch-roko-dir

**Purpose**: Emit debounced file-system change events from the
workspace `.roko/` directory.

**Implementation details**:

- The connector is a `NoopConnector` -- no network handshake is needed
  because the source is the local file system.
- The trigger wraps a `notify::RecommendedWatcher` (platform-native
  file-system notification API).
- A filter predicate ignores partial writes and temporary files
  (editor swap files, `.tmp` suffixes).
- Debounce is configured at the trigger level to avoid flooding
  downstream consumers during bulk state writes.

**Downstream consumers**: TUI file watcher (`crates/roko-cli/src/tui/fs_watch.rs`),
state reload triggers.

### 1.2 provider-health-feed

**Purpose**: Emit periodic snapshots of model provider circuit-breaker
health.

**Implementation details**:

- The connector returns a clone of the shared `ProviderHealthSnapshot`
  from the `AppState` on each `query()` call.
- The trigger fires on a polling interval (configurable, default 30s).
- The feed does **not** create its own HTTP health probes. It reads the
  existing observed circuit registry maintained by the provider dispatch
  infrastructure.
- Each pulse payload is a JSON object keyed by provider ID, with
  circuit state (`closed`, `half_open`, `open`), success/failure
  counts, and last-observed timestamps.

**Downstream consumers**: CascadeRouter health-aware selection,
dashboard provider status panel.

### 1.3 episode-outcome-feed

**Purpose**: Tail the canonical `.roko/episodes.jsonl` log and emit
each new episode as a feed pulse.

**Implementation details**:

- The connector opens the episodes file and seeks to the end.
- The trigger polls for new lines at the configured interval.
- Each new line is parsed as a JSON `Episode` record and emitted as a
  pulse payload.
- If parsing fails (corrupted line), the line is skipped with a warning
  -- the feed does not crash on malformed input.

**Design note**: This feed deliberately does **not** reconstruct any
rate-oracle or ISFR (Interblock Stale Feed Rate) vertical. That
vertical was deprecated on 2026-08-13. The episode feed is a
domain-neutral tail of the append-only episode log.

**Downstream consumers**: Learning pipeline, efficiency tracker,
playbook enrichment.

---

## 2. Constructing a Custom Feed

To add a new feed source:

### 2.1 Implement ConnectorOps

```rust
pub struct MyConnector { /* connection state */ }

#[async_trait]
impl ConnectorOps for MyConnector {
    async fn connect(&self) -> Result<()> {
        // Establish connection to external source
    }
    async fn query(&self, query: &str) -> Result<Value> {
        // On-demand query against the source
    }
    async fn disconnect(&self) -> Result<()> {
        // Tear down connection
    }
    async fn health(&self) -> ConnectorStatus {
        // Return current health
    }
}
```

For feeds that do not require an external connection (in-memory
adapters, file watchers), use `NoopConnector`.

### 2.2 Implement FeedTriggerOps (or use FeedTrigger)

For most cases, the built-in `FeedTrigger` channel-based trigger is
sufficient:

```rust
let (trigger, sender) = FeedTrigger::channel(
    "my.topic",    // source topic
    100,           // debounce_ms
    256,           // channel capacity
    |event| {      // filter predicate
        event.get("relevant").and_then(Value::as_bool) == Some(true)
    },
);
```

The `sender` handle is given to whatever produces raw events (a polling
loop, a webhook handler, a file watcher callback). The trigger takes
care of filtering and debounce.

For custom trigger logic, implement `FeedTriggerOps` directly.

### 2.3 Construct and Register

```rust
let cell = Arc::new(FeedCell::new(
    "my-feed-id",
    "My Feed",
    (1, 0, 0),
    Arc::new(my_connector),
    Arc::new(trigger),
    Some(Arc::new(MemoryFeedStore::default())),  // or None
    FeedCellConfig {
        kind: FeedKind::Derived,
        access: FeedAccess::Public,
        polling_interval: Duration::from_secs(10),
        topic: "my.topic".to_string(),
    },
));

runtime_registry.register(
    FeedInfo { /* descriptor fields */ },
    move || Arc::clone(&cell),
);
```

### 2.4 Start and Bridge

```rust
let handle = runtime_registry.start_registered("my-feed-id")?;
let _bridge = feed_bus_bridge.spawn(handle.cell().subscribe());
```

The feed is now supervised (auto-reconnect on failure) and its pulses
are routed to Bus topic `feed:my-feed-id:data`.

---

## 3. Feed Kind Selection Guide

| Kind | When to use | Examples |
|---|---|---|
| `Raw` | Direct source ingestion with no transformation | File changes, episode log tail, webhook events |
| `Derived` | Computed from one or more raw feeds | Moving averages, health summaries, filtered event streams |
| `Composite` | Cross-domain assembly from multiple derived feeds | Portfolio risk aggregation, workspace health index |
| `Meta` | Metadata about other feeds or the feed system itself | Provider health, feed accuracy tracking, system heartbeat |

---

## 4. Source References

| File | What it contains |
|---|---|
| `crates/roko-core/src/feed_cell.rs` | `FeedCell`, `ConnectorOps`, `FeedTriggerOps`, `StoreOps`, `NoopConnector`, `MemoryFeedStore`, `FeedTrigger`, `UnavailableTrigger` |
| `crates/roko-serve/src/state.rs` (lines ~965-1030) | Built-in feed registration (`file-watch-roko-dir`, `provider-health-feed`, `episode-outcome-feed`) |
| `crates/roko-cli/src/tui/fs_watch.rs` | TUI file watcher consuming `file-watch-roko-dir` events |
