# Depth 18-03: Supervised Relay Client

> Dedicated outbound relay bridge for presence, card hosting, and messaging. WebSocket
> connection with supervised reconnect, atomic room restore, FIFO delivery, and
> ACK-after-durable topic handling.

**Source:** `crates/roko-agent-server/src/features/relay_client.rs`,
`crates/roko-agent-server/src/features/relay_subscriber.rs`

---

## 1. Connection Entry Point

```rust
pub async fn connect(
    config: RelayClientConfig,
    state: Arc<AgentState>,
    card: AgentCard,
    topic_handler: Option<Arc<dyn TopicHandler>>,
) -> Result<RelayHandle>
```

This function:
1. Builds a `RelayIdentity` from the config (WebSocket URL, `AgentHello`, card, card URI)
2. Loads the durable cursor from the topic handler (if provided)
3. Establishes the initial WebSocket connection with a 10-second timeout
4. Spawns the supervisor task
5. Returns a `RelayHandle` for caller interaction

The `RelayClientConfig` carries the relay base URL and an optional initial room set. The
initial room set is validated and bounded by `MAX_DESIRED_ROOMS` (64).

---

## 2. RelayHandle

A cheaply clonable command sender:

```rust
pub struct RelayHandle {
    command_tx: mpsc::Sender<ClientCommand>,
    shutdown: CancellationToken,
    status_rx: watch::Receiver<RelayClientStatus>,
}
```

All commands are enqueued via `try_send` -- if the command queue (capacity 256) is full,
the operation fails immediately rather than blocking. If the relay connection has been
closed, sends return an error but never panic.

### 2.1 Command Validation

- `subscribe` and `unsubscribe` validate room names via `validate_room()`
- `register_feed` validates the feed descriptor and its topic
- `publish` constructs a temporary `RelayEnvelope` and calls `.validate()` before enqueuing

---

## 3. RelayClientStatus

Observable supervisor state:

```rust
pub enum RelayClientStatus {
    Connected {
        durable_cursor: u64,    // Highest committed sequence
        desired_rooms: usize,   // Number of rooms restored on reconnect
    },
    Disconnected {
        durable_cursor: u64,    // Cursor for next atomic restore
    },
    ReconciliationRequired {
        snapshot_seq: u64,      // Relay sequence of rejected snapshot
    },
    Superseded {
        by_instance: String,    // Instance that took ownership
    },
    Stopped,
}
```

Status is distributed via a `watch::channel`. The `subscribe_status()` method returns a
clone of the receiver for health/status surfaces.

---

## 4. Supervisor Loop

The `supervise` function is an infinite loop with three terminal exits:

### 4.1 Reconnect with Backoff

On transport error, the supervisor:
1. Reports `Disconnected` status
2. Waits for exponential backoff: `min(250ms * 2^attempt, 30s)`
3. Attempts reconnection with the same identity
4. If connected for 30+ seconds, resets the attempt counter to zero

### 4.2 Terminal Exits

Three conditions stop the supervisor permanently:
- **Shutdown:** explicit `CancellationToken` cancellation
- **Superseded:** another connection claimed the same agent ID
- **ReconciliationRequired:** a snapshot could not safely substitute for topic replay

### 4.3 Connection Run Loop

`run_connection` multiplexes five concurrent streams:

```rust
tokio::select! {
    () = shutdown.cancelled() => break RunExit::Shutdown,
    incoming = socket.next() => { /* relay frames */ },
    completion = completion_rx.recv() => { /* delivery worker */ },
    response = response_rx.recv() => { /* dispatch responses */ },
    completed = dispatch_tasks.join_next() => { /* task cleanup */ },
    command = command_rx.recv() => { /* client commands */ },
}
```

---

## 5. Atomic Room Restore

On each new connection, `restore_desired` sends a single `Subscribe` frame containing:
- ALL desired rooms (sorted)
- The durable cursor as `last_seq`

This is deliberately atomic: the relay sees the complete subscription state in one message.
The client stays in `Disconnected` status until the relay acknowledges the subscription
install with a matching `Ack` event.

When a subscribe or unsubscribe command changes the desired room set:
- The command handler returns an error ("subscription set changed; reconnecting with
  atomic cursor")
- This causes `run_connection` to exit with `RunExit::Disconnected`
- The supervisor reconnects with the updated set and existing cursor
- The subscription install is atomic on the new connection

This design prevents the relay from seeing a cursor that is inconsistent with the
subscription set.

---

## 6. Delivery Worker

The delivery worker is a dedicated tokio task that processes three kinds of work in FIFO
order:

### 6.1 Topic Messages

For each topic message with `seq > durable_cursor`:
1. The handler's `on_topic_message` is called
2. On success, a `Committed { cursor: seq, ack }` completion is sent
3. The supervisor advances the durable cursor and sends the ACK frame to the relay

### 6.2 Snapshots

For snapshots with `seq > durable_cursor`:
1. The handler's `on_snapshot` is called
2. If `AppliedEquivalent`, a committed completion advances the cursor
3. If `ReconciliationRequired`, the delivery worker signals reconciliation

For snapshots with `seq < durable_cursor`:
- The supervisor immediately exits with `ReconciliationRequired`
- A snapshot behind the cursor indicates an inconsistent relay state

### 6.3 Replay Checkpoints

For `ReplayComplete { from_seq, to_seq }` with `to_seq > durable_cursor`:
1. The handler's `on_replay_complete` is called
2. On success, a committed completion advances the cursor

### 6.4 Cursor Ordering

The global cursor never advances past unfinished lower-sequence work from another room.
The delivery worker runs as a single FIFO task, not as concurrent per-room workers. This
single-writer ordering eliminates cursor gaps from concurrent room processing.

---

## 7. Message Dispatch

Relay `Message` frames (direct agent-to-agent messages, not topic messages) are dispatched
concurrently through a semaphore-bounded task set:

- Semaphore capacity: 16 (`MAX_MESSAGE_DISPATCHES`)
- Each dispatch task calls `dispatch_relay_message(&state, message)`
- Results are sent back to the relay as `Response` or `Error` frames
- Permit exhaustion causes the relay to receive an error rather than blocking

---

## 8. TopicHandler Contract

```rust
#[async_trait]
pub trait TopicHandler: Send + Sync + 'static {
    async fn durable_cursor(&self) -> Result<u64>;
    async fn on_topic_message(
        &self, topic: &str, msg_type: &str, payload: Value,
        publisher_id: Option<&str>, seq: u64,
    ) -> Result<()>;
    async fn on_snapshot(&self, snapshot: &SnapshotMessage) -> Result<SnapshotDisposition>;
    async fn on_replay_complete(&self, from_seq: u64, to_seq: u64) -> Result<()>;
}
```

The contract:
- `durable_cursor()` loads the last committed cursor from persistent storage
- `on_topic_message()` must durably commit the message before returning `Ok`
- `on_snapshot()` must determine whether the snapshot safely replaces missed messages
- `on_replay_complete()` must persist the replay watermark

Returning `Ok` from `on_topic_message` is the sole trigger for the relay ACK. No other
code path advances the cursor.

---

## 9. RelaySubscriber (High-Level API)

`RelaySubscriber` wraps a `RelayHandle` with channel-based message delivery:

```rust
pub struct RelaySubscriber {
    handle: RelayHandle,
}
```

### 9.1 Handler Creation

```rust
let (handler, mut rx) = RelaySubscriber::make_handler();
// handler: Arc<dyn TopicHandler>
// rx: mpsc::Receiver<TopicMessage>
```

The internal `ChannelTopicHandler` forwards each topic message to the channel and waits
for the consumer to call `commit()` or `reject()` via a oneshot channel.

### 9.2 TopicMessage

```rust
pub struct TopicMessage {
    pub topic: String,
    pub msg_type: String,
    pub payload: Value,
    pub publisher_id: Option<String>,
    pub seq: u64,
    commit_tx: Option<oneshot::Sender<Result<()>>>,
}
```

- `commit()` -- signals durable success; the relay client emits ACK
- `reject(error)` -- signals failure; the client reconnects from its previous cursor

Each message can only be committed or rejected once. Attempting to commit/reject twice
returns an error.

### 9.3 Bounded Delivery

`make_handler_with_capacity(capacity)` creates a handler with an explicit bounded delivery
capacity. The capacity is clamped to `[1, 4096]`.

---

## Verification

```bash
# Full integration: relay registration, card hosting, message round-trip
cargo test -p roko-agent-server --test relay_registration \
    wallet_free_relay_registration_hosts_card_and_keeps_direct_routes_working

# Wallet-backed registration with on-chain identity update
cargo test -p roko-agent-server --test relay_registration \
    wallet_backed_relay_registration_submits_target_abi_with_relay_card_uri
```
