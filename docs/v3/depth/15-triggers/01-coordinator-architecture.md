# Depth: Coordinator Architecture

> Parent: [15-TRIGGERS](../../15-TRIGGERS.md) -- Section 1

---

## TriggerCoordinator Actor

The `TriggerCoordinator` is a single-task actor that owns the entire trigger
runtime's mutable state. It is spawned once by `TriggerRuntimeHandle::spawn`
and communicates exclusively through a bounded `mpsc::channel` with capacity
1,024.

### State

```rust
struct TriggerCoordinator {
    state: Weak<AppState>,                               // non-owning server ref
    sender: mpsc::Sender<Command>,                       // self-reference for spawned tasks
    bindings: HashMap<String, ActiveBinding>,             // live trigger state
    seen_traces: HashSet<(String, String)>,               // (name, trace_id) dedup
    pending_chain: HashMap<ChainLogKey, Vec<PendingChainEvent>>,
    delivered_chain: HashMap<ChainLogKey, Vec<DeliveredChainEvent>>,
    canonical_blocks: BTreeMap<u64, (String, String)>,   // block -> (hash, parent_hash)
}
```

### ActiveBinding

Each armed trigger is wrapped in an `ActiveBinding` that carries all
runtime-mutable state:

```rust
struct ActiveBinding {
    binding: TriggerBinding,                // the durable config
    signature: Vec<u8>,                     // JSON fingerprint for change detection
    armed: bool,                            // whether sources are listening
    source_cancel: Option<CancellationToken>, // kill switch for the source task
    source_generation: u64,                 // monotonic; prevents stale rearm
    running: HashMap<Uuid, RunningFlow>,    // active Flow executions
    concurrency_queue: VecDeque<TriggerEvent>,
    debounce_event: Option<TriggerEvent>,
    debounce_generation: u64,
    rate_history: VecDeque<u64>,            // timestamps of recent fires
    rate_queue: VecDeque<TriggerEvent>,
    rate_generation: u64,
    signal_history: VecDeque<(u64, String, String)>, // SignalPattern window
    consecutive_successes: u32,             // for graduation policy
}
```

### Command enum

The coordinator processes eleven command variants:

| Command | Producer | Effect |
|---|---|---|
| `Reconcile` | DiskReloader, API | Diff bindings, arm/disarm as needed |
| `Submit` | Webhook handler, API fire, inbox | Run admission pipeline, launch Flow |
| `Observe` | EventObserver | Process ServerEvent (webhooks, chain, bus projection) |
| `ObservePulse` | PulseObserver | Match Bus and SignalPattern triggers |
| `DebounceReady` | Timer task | Release debounced event if generation matches |
| `RateReady` | Timer task | Drain one rate-queued event if generation matches |
| `FlowDone` | Flow task | Record outcome, evaluate graduation, drain queue |
| `SourceStopped` | Source task | Log error, schedule 1s delayed rearm |
| `Rearm` | Timer task | Restart source if generation matches and binding enabled |
| `SourceError` | Webhook auth, external | Emit error lifecycle event |
| `Shutdown` | ShutdownObserver | Disarm all sources, cancel all Flows |

### Reconciliation algorithm

1. Identify removed bindings (present in live set but absent from incoming).
   Disarm each one.
2. For each incoming binding, compute its JSON serialization as a fingerprint.
3. If the fingerprint matches the live binding's signature, skip it.
4. If changed or new: disarm the old binding (if any), create a new
   `ActiveBinding`, arm it (if enabled), insert into the live set.

### Source lifecycle

`start_source` spawns a tokio task for each source kind that needs
background polling or watching. The task receives a `CancellationToken` and
a copy of the coordinator's `mpsc::Sender`. When the source detects its
condition, it sends a `Submit` command. When the task exits (error or
cancel), it sends `SourceStopped`.

Failed sources are automatically rearmed after a 1-second delay via the
`Rearm` command. The `source_generation` counter prevents stale rearm
attempts from resurrecting a binding that has been reconciled away.

### Background observers

| Observer | Source | Delivery |
|---|---|---|
| `start_event_observer` | `ServerEvent` broadcast receiver | `Command::Observe` |
| `start_pulse_observer` | `Bus::subscribe(TopicFilter::All)` | `Command::ObservePulse` |
| `start_disk_reloader` | 500ms tokio interval | `Command::Reconcile` |
| `start_shutdown_observer` | `CancelToken::cancelled()` | `Command::Shutdown` |

All observers hold a `Weak<AppState>` and exit gracefully when the server
drops. The event observer handles broadcast lag by logging and continuing.

**Source:** `crates/roko-serve/src/trigger_runtime.rs`
