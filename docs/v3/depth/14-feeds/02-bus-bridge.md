# depth/14-feeds/02 -- Feed Bus Bridge

> How feed Cell outputs are routed into the canonical Bus for
> downstream consumption by subscribers, dashboards, and recipe
> pipelines.

**Parent**: [14-FEEDS-RECIPES.md](../../14-FEEDS-RECIPES.md) -- Section 3

---

## 1. Purpose

The `FeedBusBridge` is a generic adapter that converts `FeedPulse`
events from any feed Cell's `broadcast::Receiver` into canonical `Pulse`
objects published on the workspace Bus. Without the bridge, feed outputs
remain confined to their local broadcast channel. With the bridge, every
feed's data becomes available to any Bus subscriber through the
standard topic-based pub/sub mechanism.

---

## 2. Bridge Architecture

```
FeedCell                  FeedBusBridge              Bus
   |                          |                       |
   | broadcast::send(         |                       |
   |   FeedPulse { ... })     |                       |
   | -----------------------> |                       |
   |                          | route_pulse():        |
   |                          |   1. Get/create stats |
   |                          |   2. Build Pulse      |
   |                          |   3. Tag with feed_id |
   |                          |   4. bus.publish()    |
   |                          | --------------------> |
   |                          |   5. Increment stats  |
   |                          |                       |
   |                          |                       +--- subscribers
   |                          |                       +--- SSE routes
   |                          |                       +--- recipe eval
```

---

## 3. Topic Convention

Every `FeedPulse` is routed to a Bus topic following the pattern:

```
feed:{feed_id}:data
```

Examples:

| Feed ID | Bus Topic |
|---|---|
| `file-watch-roko-dir` | `feed:file-watch-roko-dir:data` |
| `provider-health-feed` | `feed:provider-health-feed:data` |
| `episode-outcome-feed` | `feed:episode-outcome-feed:data` |

---

## 4. Pulse Construction

The bridge constructs each Bus Pulse with:

| Pulse field | Value |
|---|---|
| `sequence` | Source pulse sequence number |
| `topic` | `feed:{feed_id}:data` |
| `kind` | `Kind::Custom("feed.data")` |
| `body` | `Body::Json(pulse.payload)` |
| `created_at_ms` | Source pulse `emitted_at_ms` |
| **Tags** | |
| `feed_id` | Source feed identifier |
| `source_topic` | Original feed-level topic (e.g., `fs.changed`) |
| `source_sequence` | Original sequence number (string) |

The tags preserve provenance so downstream consumers can trace any Bus
Pulse back to its originating feed and sequence.

---

## 5. Forwarding Loop Semantics

`FeedBusBridge::spawn(receiver)` starts a tokio task with the
following behavior:

1. **Receive**: Block on `receiver.recv()`.
2. **Route**: On success, call `route_pulse(pulse)`. Errors from the
   Bus publish are silently dropped (the bridge is best-effort -- it
   must not crash the feed supervision loop).
3. **Lag**: On `RecvError::Lagged(n)`, skip the lost messages and
   continue. This provides bounded degradation: if the bridge is slow,
   the feed Cell is not blocked, and the bridge catches up from the
   next available message.
4. **Closed**: On `RecvError::Closed`, exit cleanly. This happens when
   the feed Cell is stopped and the broadcast channel is dropped.

The bridge task runs for the lifetime of the feed. When the feed is
stopped, the broadcast channel closes, the bridge exits, and the
`JoinHandle` can be awaited for cleanup.

---

## 6. Per-Feed Routing Metrics

The bridge maintains per-feed counters in a `HashMap<String, Arc<FeedRouteStats>>`:

```rust
pub struct FeedRouteStats {
    pub pulses_routed:     AtomicU64,
    pub bytes_routed:      AtomicU64,
    pub last_routed_at_ms: AtomicU64,
}
```

- `pulses_routed`: Monotonic count of successfully published Bus Pulses.
- `bytes_routed`: Approximate total serialized payload bytes published
  (computed via `serde_json::to_vec(&payload).len()`).
- `last_routed_at_ms`: Unix timestamp of the most recently routed
  source pulse.

Counters are queryable via `bridge.stats(feed_id)`.

---

## 7. Lifecycle Integration

The bridge is started alongside each feed in the serve layer:

```rust
// In POST /api/feeds/start/{id} handler:
let handle = state.runtime_feeds.start_registered(&id)?;
let _bridge = state.feed_bus_bridge.spawn(handle.cell().subscribe());
```

The `_bridge` JoinHandle is not explicitly tracked because the bridge
exits naturally when the feed's broadcast channel closes. Stopping
the feed via `runtime_feeds.stop(id)` causes the Cell to disconnect
and drop the broadcast sender, which closes the bridge loop.

---

## 8. Bus Type Genericity

`FeedBusBridge` is generic over the Bus implementation:

```rust
pub struct FeedBusBridge<B: Bus + 'static> { ... }
```

In production, `B` is the workspace's `MemoryBus`. In tests, any
`Bus` implementation (including a test spy) can be injected. The bridge
test in `feed_bus_bridge.rs` uses `MemoryBus::new(16)`.

---

## 9. Source References

| File | What it contains |
|---|---|
| `crates/roko-core/src/feed_bus_bridge.rs` | `FeedBusBridge`, `FeedRouteStats`, forwarding loop, test |
| `crates/roko-serve/src/routes/feeds.rs` (line ~313) | Bridge spawned in `start_feed` handler |
| `crates/roko-core/src/pulse.rs` | `Pulse::builder`, `Topic`, `Kind`, `Body` |
