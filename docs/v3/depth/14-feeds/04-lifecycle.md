# depth/14-feeds/04 -- Feed Lifecycle and Supervision

> Runtime supervision, exponential backoff reconnection, start/stop
> semantics, health aggregation, and HTTP/CLI lifecycle management.

**Parent**: [14-FEEDS-RECIPES.md](../../14-FEEDS-RECIPES.md) -- Sections 2.3, 7, 8

---

## 1. RuntimeRegistry

The `RuntimeRegistry` is the central supervisor for all live feed Cells.
It maintains two maps:

```rust
pub struct RuntimeRegistry {
    factories: RwLock<HashMap<String, FeedFactory>>,  // discoverable descriptors + constructors
    running:   RwLock<HashMap<String, FeedHandle>>,   // actively supervised feeds
    reconnect: ReconnectPolicy,                        // shared backoff parameters
}
```

The separation of `factories` (static registration) from `running`
(live instances) means a feed can be discoverable but not active, and
starting a feed does not remove its factory registration.

---

## 2. FeedHandle

Each running feed is represented by a `FeedHandle`:

```rust
pub struct FeedHandle {
    cell:           Arc<FeedCell>,
    cancel:         CancellationToken,
    task:           Arc<tokio::sync::Mutex<Option<JoinHandle<()>>>>,
    started_at_ms:  u64,
    error_count:    Arc<AtomicU64>,
    last_error:     Arc<RwLock<Option<String>>>,
}
```

The handle is `Clone`, so it can be shared between the registry, the
HTTP handlers, and the Bus bridge.

Key methods:

| Method | Behavior |
|---|---|
| `cell()` | Returns the underlying `Arc<FeedCell>` for subscription |
| `stop()` | Cancels the supervision token, awaits the task, disconnects the Cell |
| `status()` | Computes `FeedRuntimeStatus` from live counters |
| `error_count()` | Monotonic count of failed connect/listen cycles |
| `last_error()` | Human-readable string from the most recent failure |

---

## 3. Supervision Loop

When `RuntimeRegistry::start(cell)` is called, it spawns a tokio task
that runs the following loop:

```
delay = policy.initial  (default 1s)

loop {
    if cancelled: break

    match cell.connect() {
        Ok:
            delay = policy.initial    -- reset on success
            clear last_error
            cell.listen(cancel) -- blocks until cancel or error
            if listen error:
                error_count += 1
                last_error = error message
        Err:
            error_count += 1
            last_error = error message
    }

    cell.disconnect()

    select {
        cancelled => break
        sleep(delay) => {}
    }

    delay = min(delay * 2, policy.maximum)  -- exponential backoff
}

cell.disconnect()  -- final cleanup
```

### 3.1 Reconnect Policy

```rust
pub struct ReconnectPolicy {
    pub initial: Duration,  // First delay after failure (default: 1s)
    pub maximum: Duration,  // Ceiling for exponential backoff (default: 60s)
}
```

The backoff sequence is: 1s, 2s, 4s, 8s, 16s, 32s, 60s, 60s, ...

Each successful `connect()` resets the delay to `initial`, so a feed
that recovers from a transient failure immediately returns to fast
reconnection for the next failure.

### 3.2 Cancellation

The `CancellationToken` is checked at two points:

1. Before each connection attempt.
2. During the sleep between retries (`tokio::select!` with
   `cancel.cancelled()`).

The listen phase receives a child token, so cancellation propagates
to the trigger loop without waiting for the next poll.

---

## 4. Start Semantics

`start_registered(feed_id)`:

1. Look up the factory by ID. Fail if not found.
2. Call the factory closure to construct a new `Arc<FeedCell>`.
3. Delegate to `start(cell)`.

`start(cell)`:

1. Check if the feed ID is already in `running`. Fail if duplicate.
2. Create cancellation token, error counters, and last-error state.
3. Spawn the supervision task.
4. Insert the `FeedHandle` into `running`.
5. Return the handle.

**Invariant**: At most one supervision task runs for any given feed ID
at any time.

---

## 5. Stop Semantics

`stop(feed_id)`:

1. Remove the handle from `running`. Fail if not found.
2. Call `handle.stop()`:
   a. Cancel the supervision token.
   b. Await the supervision task (via `task.lock().take()`).
   c. Call `cell.disconnect()`.
3. Return success.

`stop_all()`:

1. Collect all running feed IDs.
2. Call `stop(id)` for each, in sequence.
3. If any stop fails, continue with the rest and return the first error.

---

## 6. Health Aggregation

`health()` returns `Vec<FeedRuntimeStatus>` for all discoverable feeds
(not just running ones):

```rust
pub struct FeedRuntimeStatus {
    pub id:              String,
    pub topic:           String,
    pub kind:            String,
    pub connected:       bool,
    pub rate_hz:         f64,
    pub pulses_produced: u64,
    pub last_update_ms:  Option<u64>,
    pub error:           Option<String>,
}
```

For running feeds, the status is computed from live `FeedHandle`
counters. For stopped feeds, the status is synthesized from the
descriptor with `connected: false` and zeroed counters.

Rate calculation: `pulses_produced * 1000.0 / elapsed_ms`. This is an
average rate since the feed was started, not an instantaneous rate.

---

## 7. HTTP Lifecycle Endpoints

### 7.1 Start a Feed

```
POST /api/feeds/start/{id}
```

**Handler behavior**:

1. Call `runtime_feeds.start_registered(id)`.
2. Subscribe to the new handle's broadcast channel.
3. Spawn a `FeedBusBridge` forwarding loop.
4. Return `202 Accepted` with `FeedRuntimeStatus`.

**Error**: 400 if the feed ID is unknown or already running.

### 7.2 Stop a Feed

```
POST /api/feeds/stop/{id}
```

**Handler behavior**:

1. Look up the running handle.
2. Call `runtime_feeds.stop(id)`.
3. Return `200 OK` with the final `FeedRuntimeStatus`.

**Error**: 404 if the feed is not running.

### 7.3 Health

```
GET /api/feeds/health
```

Returns `Vec<FeedRuntimeStatus>` for all discoverable feeds.

### 7.4 Discovery and Search

```
GET /api/feeds/discover        -- all registered feed descriptors
GET /api/feeds/search?q=term   -- filtered by id, name, description, kind
```

---

## 8. CLI Lifecycle Commands

### 8.1 Start and Stop

```bash
roko feed start file-watch-roko-dir
# Output: started feed 'file-watch-roko-dir' (fs.changed)

roko feed stop file-watch-roko-dir
# Output: stopped feed 'file-watch-roko-dir' (fs.changed)
```

Both commands POST to the serve layer and print the resulting status.
If `--json` is passed, raw `FeedRuntimeStatus` JSON is printed instead.

### 8.2 Server Availability

All feed CLI commands require a running `roko serve` instance. If the
server is not reachable, the command prints a diagnostic:

```
(roko serve is not running; no live feed data available)
Start the server with: roko serve
```

With `--json`, an error object is returned:

```json
{"error": "roko serve is not running"}
```

The base URL defaults to `http://localhost:6677` and can be overridden
via the `ROKO_SERVE_URL` environment variable.

---

## 9. Source References

| File | What it contains |
|---|---|
| `crates/roko-core/src/feed_runtime.rs` | `RuntimeRegistry`, `FeedHandle`, `ReconnectPolicy`, supervision loop, health aggregation |
| `crates/roko-core/src/feed.rs` | `FeedRuntimeStatus` |
| `crates/roko-serve/src/routes/feeds.rs` | HTTP start/stop/health/discover/search handlers |
| `crates/roko-cli/src/commands/feed.rs` | CLI start/stop/list/status/health/discover/search commands |
