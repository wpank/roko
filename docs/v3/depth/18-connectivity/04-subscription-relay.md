# Depth 18-04: Subscription Relay (Serve-Side)

> Restart-safe relay subscription execution for `roko serve`. Durable journal, stream
> binding, intent-before-execution, atomic commit, and fail-closed reconciliation.

**Source:** `crates/roko-serve/src/subscription_relay.rs`

---

## 1. Design Invariant

The relay client emits ACK only after `TopicHandler::on_topic_message` returns success.
This module makes that success boundary durable:

1. A **processing intent** is persisted BEFORE agent execution
2. Terminal dispatch **receipts** are recorded as they complete
3. The global cursor, room cursors, and subscription cursors are committed **atomically**
   with the terminal receipts in a single journal replacement
4. An interrupted intent is NEVER replayed blindly -- it becomes an explicit
   **reconciliation requirement**

---

## 2. Journal File

The journal persists at `.roko/state/subscription-relay-journal.json`.

```rust
struct JournalFile {
    schema_version: u32,                              // Currently 2
    integrity_hash: String,                           // Content hash of journal state
    global_cursor: u64,                               // Highest committed relay sequence
    stream_binding: Option<RelayStreamBinding>,       // Bound relay identity + rooms
    room_cursors: BTreeMap<String, u64>,              // Per-room highest committed seq
    subscription_cursors: BTreeMap<String, BTreeMap<String, u64>>,  // Per-subscription per-room
    entries: VecDeque<JournalEntry>,                   // Processing/committed/failed entries
    reconciliation_required: Option<ReconciliationRecord>,
}
```

### 2.1 Bounds

| Constant | Value | Purpose |
|----------|-------|---------|
| `JOURNAL_SCHEMA_VERSION` | 2 | Current schema version |
| `JOURNAL_MAX_ENTRIES` | 4,096 | Maximum retained entries |
| `JOURNAL_MAX_BYTES` | 4 MiB | Maximum file size |
| `JOURNAL_MAX_CURSOR_KEYS` | 4,096 | Maximum room/subscription cursor keys |
| `JOURNAL_MAX_TEXT_BYTES` | 512 | Maximum text field size (room, msg_type, etc.) |

### 2.2 Integrity

Every journal write refreshes the `integrity_hash` field before persisting. The journal
is validated on open: schema version, integrity hash, bounds on entries/cursors, and
monotonic cursor ordering.

---

## 3. Journal Entry State Machine

```rust
enum JournalEntryState {
    Processing {
        expected_subscriptions: Vec<String>,
        partial_terminals: Vec<SubscriptionTerminal>,
    },
    Committed {
        terminals: Vec<SubscriptionTerminal>,
    },
    Failed {
        error: String,
        partial_terminals: Vec<SubscriptionTerminal>,
    },
    ReplayCheckpoint,
}
```

### 3.1 Processing

Created by `begin_message()`. Records the dispatch plan: which subscriptions will process
this relay message. The `expected_subscriptions` list defines the exact order in which
terminal receipts must arrive.

### 3.2 Terminal Recording

`record_terminal()` appends a `SubscriptionTerminal` to the entry's `partial_terminals`.
Terminals must arrive in plan order -- out-of-order terminals are rejected.

Two terminal types:
- `Dispatched { receipt }` -- the subscription was dispatched and has a `DispatchTerminal`
- `Suppressed { subscription_id, policy, detail }` -- the subscription was intentionally
  skipped (e.g., rate limiting, cooldown)

### 3.3 Commit

`commit_message()` transitions the entry from `Processing` to `Committed` and atomically
advances:
- `global_cursor` to the entry's `seq`
- `room_cursors[room]` to the entry's `seq`
- `subscription_cursors[subscription_id][room]` to the entry's `seq` for each terminal

The commit fails if the terminal receipt list does not exactly match the expected
subscription list.

### 3.4 Failure

`fail_message()` transitions the entry from `Processing` to `Failed` and records a
reconciliation requirement. The failed entry retains its partial terminals for forensic
inspection.

---

## 4. Stream Binding

Before any message is processed, the journal binds to a specific relay stream:

```rust
pub struct RelayStreamBinding {
    pub origin_hash: String,     // Content hash of the relay identity
    pub rooms: Vec<String>,      // Exact sorted room set
    pub room_set_hash: String,   // Hash of the room set
    pub generation: u64,         // Monotonic binding generation
}
```

### 4.1 Binding Lifecycle

- First bind: records the origin hash, rooms, and generation 1
- Subsequent binds with same origin and rooms: no-op (idempotent)
- Changed origin: forces reconciliation (cursor reuse is unsafe)
- Changed rooms after cursor > 0: forces reconciliation

### 4.2 Guard Stream Change

`guard_stream_change()` is a lighter pre-check that determines whether a proposed stream
change would be safe. It validates:
- Origin hash format (64-character hex content hash)
- Room list ordering and validity
- Existing binding compatibility

If reconciliation is already required, the guard rejects any change.

---

## 5. Reconciliation

A `ReconciliationRecord` is persisted when any of these conditions is detected:

| Trigger | Reason |
|---------|--------|
| Restart with interrupted intent | "restart observed an interrupted dispatch intent" |
| Unbound room delivery | "relay delivered unbound room '{room}'" |
| Unfinished intent redelivery | "relay redelivered an unfinished dispatch intent" |
| Dispatch failure | "dispatch failed before relay ACK: {error}" |
| Origin identity change | "relay origin identity changed; cursor reuse is unsafe" |
| Room set change after cursor > 0 | "relay room set changed after the global cursor advanced" |
| Snapshot behind cursor | Immediate `ReconciliationRequired` exit in relay client |
| Snapshot disposition | `SnapshotDisposition::ReconciliationRequired` from handler |

```rust
pub struct ReconciliationRecord {
    pub seq: u64,              // Relay sequence at which reconciliation was detected
    pub reason: String,        // Human-readable explanation
    pub detected_at: u64,      // Unix timestamp
    pub evidence_hash: String, // Content hash of the evidence (payload, binding, etc.)
}
```

Once reconciliation is required, all further `begin_message()` calls are rejected with an
error. The operator must inspect the journal and resolve the inconsistency before
processing can resume.

---

## 6. Remote Subscription Planning

`remote_subscription_plan()` scans the `SubscriptionRegistry` to build the relay room set:

```rust
pub fn remote_subscription_plan(
    registry: &SubscriptionRegistry,
    room_capacity: usize,
) -> RemoteSubscriptionPlan {
    // ...
}

pub struct RemoteSubscriptionPlan {
    pub rooms: Vec<String>,
    pub unsupported_triggers: Vec<String>,
    pub capacity_rejected_rooms: Vec<String>,
}
```

### 6.1 Eligibility Criteria

A subscription is eligible for remote relay execution when:
1. `enabled` is true
2. Not a local-only trigger (cron, file watch)
3. Trigger is an exact relay room (no globs)
4. Room count does not exceed capacity

### 6.2 Exact Room Validation

`is_exact_relay_room()` checks two things:
1. No glob characters (`*`, `?`)
2. The trigger string passes `WireEnvelope::validate()` when used as a room name

---

## 7. TopicHandler Implementation

The serve-side `TopicHandler` implementation connects the relay client to the journal:

### 7.1 on_topic_message Flow

1. Hash the payload for evidence
2. Find matching subscriptions for this room
3. Call `journal.begin_message()` -- persists processing intent
4. For each matched subscription:
   a. Check admission (permit, cooldown, debounce)
   b. Dispatch or suppress
   c. Call `journal.record_terminal()` -- persists each result
5. Call `journal.commit_message()` -- atomically commits everything
6. Return `Ok(())` -- relay client emits ACK

If any step fails, `journal.fail_message()` is called and the handler returns an error.
The relay client does NOT emit ACK. On reconnect, the durable cursor is loaded from the
journal, and the relay replays from that point.

### 7.2 on_snapshot Flow

1. Record the snapshot in the journal
2. Determine disposition:
   - If the snapshot state can be proven equivalent to missed messages: `AppliedEquivalent`
   - Otherwise: `ReconciliationRequired`

### 7.3 durable_cursor

Returns `journal.durable_cursor()` -- the `global_cursor` from the journal file.

---

## 8. SubscriptionRelayRuntime

Shared serve-side runtime:

```rust
pub struct SubscriptionRelayRuntime {
    journal: SubscriptionRelayJournal,
    connection: RwLock<ServeRelayConnectionStatus>,
    unsupported_triggers: RwLock<Vec<String>>,
    capacity_rejected_rooms: RwLock<Vec<String>>,
}
```

### 8.1 Status Reporting

`status()` returns a `SubscriptionRelayStatus` combining journal state and connection
state:

```rust
pub struct SubscriptionRelayStatus {
    pub connection: ServeRelayConnectionStatus,
    pub schema_version: u32,
    pub global_cursor: u64,
    pub stream_binding: Option<RelayStreamBinding>,
    pub room_cursors: BTreeMap<String, u64>,
    pub subscription_cursors: BTreeMap<String, BTreeMap<String, u64>>,
    pub retained_entries: usize,
    pub processing_entries: usize,
    pub failed_entries: usize,
    pub reconciliation_required: Option<ReconciliationRecord>,
    pub unsupported_triggers: Vec<String>,
    pub capacity_rejected_rooms: Vec<String>,
}
```

Exposed via `GET /api/subscriptions/relay/status`.

### 8.2 ServeRelayConnectionStatus

```rust
pub enum ServeRelayConnectionStatus {
    Unconfigured,
    Connecting,
    Connected { durable_cursor: u64, desired_rooms: usize },
    Disconnected { durable_cursor: u64, reason: Option<String> },
    ReconciliationRequired { snapshot_seq: u64 },
    Superseded { by_instance: String },
    Stopped,
}
```

This is a superset of the agent-side `RelayClientStatus` with an additional `Unconfigured`
state for when no relay URL is configured.

---

## 9. Persistence Strategy

Every journal mutation follows the same pattern:

1. Clone the current `JournalFile` into a candidate
2. Apply the mutation to the candidate
3. Refresh the integrity hash
4. Persist the candidate to disk (atomic write via temp file + rename)
5. Replace the in-memory state with the candidate

The clone-before-mutate pattern ensures that a failed persistence never corrupts the
in-memory state. The Mutex guard is held across both mutation and persistence to prevent
concurrent writes.

---

## Verification

```bash
# Subscription relay journal: open, bind, begin, record, commit, fail, reconcile
cargo test -p roko-serve --lib subscription_relay

# Remote subscription planning
cargo test -p roko-serve --lib subscription_relay::remote_subscription_plan

# Serve-side relay connection status mapping
cargo test -p roko-serve --lib subscription_relay::tests
```
