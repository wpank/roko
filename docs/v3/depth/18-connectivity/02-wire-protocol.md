# Depth 18-02: Wire Protocol

> Transport-neutral relay envelope and recovery contracts. Canonical types shared by the
> agent-side relay client, serve-side subscription relay, and any future transport adapter.

**Source:** `crates/roko-core/src/wire_protocol.rs`

---

## 1. WireEnvelope

The canonical envelope for all relay and WebSocket transports:

```rust
pub struct WireEnvelope {
    pub seq: u64,
    pub ts: u64,
    pub room: String,
    #[serde(rename = "type")]
    pub msg_type: String,
    pub payload: Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub publisher_id: Option<String>,
}

pub type RelayEnvelope = WireEnvelope;
```

### 1.1 Field Semantics

| Field | Semantics |
|-------|-----------|
| `seq` | Monotonic sequence number within the relay's global stream. Used for cursor tracking, gap detection, and ordered replay. |
| `ts` | Server timestamp in Unix milliseconds. Assigned by the relay, not the publisher. |
| `room` | Subscription room receiving this event. Matches `RoomPattern` constructors. |
| `msg_type` | Event discriminator. Serialized as `"type"` on the wire (serde rename). |
| `payload` | Event-specific body. Arbitrary JSON, bounded to 1 MiB. |
| `publisher_id` | Optional identity of the publishing agent. Carried by relay transports for attribution. |

### 1.2 Wire Key Convention

The `msg_type` field uses `#[serde(rename = "type")]` so that the JSON wire representation
uses the key `"type"` while the Rust struct avoids the reserved keyword. The field
`msg_type` never appears in serialized output -- only `"type"` does.

### 1.3 Validation Rules

`WireEnvelope::validate()` enforces:

| Field | Rule |
|-------|------|
| `room` | Non-empty, at most 256 bytes, no ASCII control or whitespace characters |
| `msg_type` | Non-empty, at most 128 bytes, no ASCII control or whitespace characters |
| `payload` | Serialized size at most 1 MiB (`MAX_WIRE_PAYLOAD_BYTES = 1024 * 1024`) |
| `publisher_id` | If present: 1-256 bytes, no ASCII control or whitespace characters |

All validation is checked before the envelope enters any delivery pipeline.

---

## 2. Room Patterns

`RoomPattern` provides canonical room-name constructors:

```rust
pub struct RoomPattern;

impl RoomPattern {
    pub fn agent(id: &str) -> String          // "agent:{id}"
    pub fn agent_heartbeat(id: &str) -> String // "agent:{id}:heartbeat"
    pub fn agent_output(id: &str) -> String    // "agent:{id}:output"
    pub fn plan(id: &str) -> String            // "plan:{id}"
    pub fn group(id: &str) -> String           // "group:{id}"
    pub fn chain(chain_id: u64) -> String      // "chain:{chain_id}"
    pub const fn system() -> &'static str      // "system"
    pub const fn learning() -> &'static str    // "learning"
}
```

Identifier-bearing constructors allocate because the room is assembled at runtime. The
constant rooms (`system`, `learning`) are returned as `&'static str` but callers typically
`.to_owned()` them for homogeneous subscription lists.

### 2.1 Room Name Validation

A valid room name for relay subscription must:
- Be an exact string (no glob characters `*` or `?`)
- Pass `WireEnvelope` validation when used as the `room` field
- The serve-side `is_exact_relay_room()` function enforces both checks

---

## 3. Subscription Messages

### 3.1 SubscribeMessage

```rust
pub struct SubscribeMessage {
    pub rooms: Vec<String>,
    pub last_seq: Option<u64>,
}
```

When `last_seq` is `Some(n)`, the relay replays all events with `seq > n` for the
requested rooms. When `last_seq` is `None`, the relay establishes the current head as
the baseline -- no replay occurs.

This is the mechanism for **atomic cursor restore**: the complete room set and cursor are
sent in a single message, ensuring the relay never sees a partial subscription state.

### 3.2 UnsubscribeMessage

```rust
pub struct UnsubscribeMessage {
    pub rooms: Vec<String>,
}
```

### 3.3 ResumeMessage (ReplayRequest)

```rust
pub struct ResumeMessage {
    pub last_seq: u64,
}

pub type ReplayRequest = ResumeMessage;
```

Used when a client wants to resume a stream strictly after `last_seq` without modifying
its subscription set.

### 3.4 SnapshotMessage

```rust
pub struct SnapshotMessage {
    pub seq: u64,
    pub state: Value,
}
```

Sent by the relay when the requested replay gap exceeds retained history. The `state`
contains a full snapshot of the current relay state for the client's subscriptions. The
client must durably inspect the snapshot and determine whether it is equivalent to the
missed topic messages before accepting it.

---

## 4. Recovery Types

### 4.1 ReconnectionState

Client-side connection/recovery state:

```rust
pub enum ReconnectionState {
    Disconnected,
    Connecting,
    Resuming { last_seq: u64 },
    Connected,
    Superseded { by_instance: String },
}
```

### 4.2 GapAction (RelayRecoveryPolicy)

Recovery selected after comparing a resume cursor with retained history:

```rust
pub enum GapAction {
    Replay { from_seq: u64, to_seq: u64 },
    Snapshot,
}

pub type RelayRecoveryPolicy = GapAction;
```

`GapAction::validate()` rejects inverted replay ranges (`from_seq > to_seq`).

### 4.3 SupersededNotice

```rust
pub struct SupersededNotice {
    pub agent_id: String,
    #[serde(rename = "by", alias = "by_instance")]
    pub by_instance: String,
}
```

Sent to the older instance when a new connection claims the same agent ID. The
`by_instance` field is serialized as `"by"` on the wire but accepts both `"by"` and
`"by_instance"` on deserialization for backward compatibility.

---

## 5. Backpressure Strategies

Per-event overload policies:

```rust
pub enum BackpressureStrategy {
    Coalesce { interval_ms: u64 },
    DropOldest { ring_size: usize },
    Lossless,
    Sample { every_nth: u64 },
}

pub type RelayBackpressure = BackpressureStrategy;
```

### 5.1 EventBackpressureConfig

```rust
pub struct EventBackpressureConfig {
    pub event_type: String,
    pub strategy: BackpressureStrategy,
}
```

Binds a backpressure strategy to a specific wire event discriminator. Validation requires
a non-empty `event_type` and a valid strategy (no zero-capacity parameters).

---

## 6. Workspace Protocol Types

### 6.1 ExoskeletonStatus

```rust
pub struct ExoskeletonStatus {
    pub mcp: bool,
    pub a2a: bool,
    pub erc8004_chain_id: Option<u64>,
    pub x402: bool,
}
```

Declares which exoskeleton protocol bindings are available from this workspace.

### 6.2 WorkspaceHello

Registration sent by `roko serve` when joining a relay:

```rust
pub struct WorkspaceHello {
    pub workspace_id: String,
    pub name: String,
    pub url: Option<String>,
    pub version: String,
    pub capabilities: Vec<String>,
    pub owner_wallet: Option<String>,
    pub agents_count: u32,
    pub uptime_secs: u64,
    pub exoskeleton: ExoskeletonStatus,
}
```

Validation: `workspace_id`, `name`, and `version` must be non-empty.

### 6.3 WorkspaceInfo

Directory entry returned by a relay:

```rust
pub struct WorkspaceInfo {
    pub workspace_id: String,
    pub name: String,
    pub url: Option<String>,
    pub owner_wallet: Option<String>,
    pub agents_count: u32,
    pub online: bool,
    pub last_seen_ms: u64,
}
```

### 6.4 WorkspaceEvent

Presence transitions:

```rust
pub enum WorkspaceEvent {
    #[serde(rename = "workspace_connected")]
    Connected { workspace_id: String, url: Option<String> },
    #[serde(rename = "workspace_disconnected")]
    Disconnected { workspace_id: String },
}
```

Wire discriminators are unambiguous: `"workspace_connected"` and
`"workspace_disconnected"`.

---

## Verification

```bash
# Envelope wire key convention
cargo test -p roko-core envelope_uses_reserved_type_wire_key

# Room constructors
cargo test -p roko-core room_constructors_match_the_protocol

# Recovery and backpressure serde round-trip
cargo test -p roko-core recovery_and_backpressure_variants_round_trip

# Adversarial validation
cargo test -p roko-core adversarial_zero_capacity_policies_and_inverted_gaps_fail_closed

# Supersession wire key
cargo test -p roko-core supersession_uses_the_documented_by_key

# Workspace event discriminators
cargo test -p roko-core workspace_events_have_unambiguous_wire_discriminators

# Subscription, resume, snapshot, workspace round-trip
cargo test -p roko-core subscription_resume_snapshot_and_workspace_contracts_round_trip
```
