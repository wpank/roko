# Depth 17-05: Bus Publication

> Room topology, 16 typed group events, Pulse delivery protocol,
> event recording, and the durable event outbox.

**Parent**: [17-GROUPS](../../17-GROUPS.md) -- Section 7

---

## 1. Room Topology

Every group event is published to a Bus room determined by the
event type. The `GroupEvent::room` method resolves the target room:

```rust
impl GroupEvent {
    pub fn room(&self, group_id: &GroupId) -> String {
        match self {
            Self::Created { .. } | Self::Deleted { .. }
                => "system".to_string(),
            Self::KnowledgePublished { .. } | Self::KnowledgeValidated { .. }
                => format!("group:{group_id}:knowledge"),
            Self::PheromoneDeposited { .. } | Self::PheromoneDecayed { .. }
                => format!("group:{group_id}:pheromones"),
            Self::TaskAssigned { .. } | Self::TaskCompleted { .. }
                => format!("group:{group_id}:coordination"),
            _ => group_id.room(),  // "group:{group_id}"
        }
    }
}
```

### 1.1 Room Hierarchy

```
system
  |-- group.created
  |-- group.deleted
  |
group:{id}                        (root room)
  |-- group.updated
  |-- group.member_invited
  |-- group.member_joined
  |-- group.member_left
  |-- group.member_updated
  |-- group.message
  |-- group.cluster_started
  |-- group.cluster_completed
  |
  +-- group:{id}:knowledge        (knowledge sub-room)
  |     |-- group.knowledge_published
  |     |-- group.knowledge_validated
  |
  +-- group:{id}:pheromones       (pheromone sub-room)
  |     |-- group.pheromone_deposited
  |     |-- group.pheromone_decayed
  |
  +-- group:{id}:coordination     (coordination sub-room)
        |-- group.task_assigned
        |-- group.task_completed
```

### 1.2 Room Purpose

| Room | Events | Subscribers |
|------|--------|-------------|
| `system` | Created, Deleted | Global watchers, admin dashboards |
| `group:{id}` | Membership, messages, clusters | All group members |
| `group:{id}:knowledge` | Knowledge lifecycle | Knowledge-interested agents |
| `group:{id}:pheromones` | Pheromone deposits and decay | Stigmergic agents |
| `group:{id}:coordination` | Task assignment and completion | Leader/follower agents |

Agents subscribe to the rooms relevant to their coordination mode
and role. An Observer might subscribe only to the root room, while
a stigmergic agent subscribes to the pheromone sub-room.

---

## 2. The 16 GroupEvent Variants

The `GroupEvent` enum defines 16 typed events organized into five
categories:

### 2.1 Lifecycle Events (3)

| Variant | Fields | Room |
|---------|--------|------|
| `Created` | group_id, name, owner | `system` |
| `Updated` | group_id, changes (JSON) | `group:{id}` |
| `Deleted` | group_id, owner | `system` |

### 2.2 Membership Events (4)

| Variant | Fields | Room |
|---------|--------|------|
| `MemberInvited` | agent_id, invited_by, role | `group:{id}` |
| `MemberJoined` | agent_id, owner, role | `group:{id}` |
| `MemberLeft` | agent_id, reason | `group:{id}` |
| `MemberUpdated` | agent_id, changes (JSON) | `group:{id}` |

### 2.3 Communication Events (3)

| Variant | Fields | Room |
|---------|--------|------|
| `Message` | from, content, tags | `group:{id}` |
| `ClusterStarted` | cluster_id, pipeline (JSON), agents | `group:{id}` |
| `ClusterCompleted` | cluster_id, outcome, duration_secs | `group:{id}` |

### 2.4 Knowledge Events (2)

| Variant | Fields | Room |
|---------|--------|------|
| `KnowledgePublished` | entry_id, author, topic | `group:{id}:knowledge` |
| `KnowledgeValidated` | entry_id, validator | `group:{id}:knowledge` |

### 2.5 Pheromone Events (2)

| Variant | Fields | Room |
|---------|--------|------|
| `PheromoneDeposited` | depositor, signal_type, intensity | `group:{id}:pheromones` |
| `PheromoneDecayed` | count_removed, threshold | `group:{id}:pheromones` |

### 2.6 Coordination Events (2)

| Variant | Fields | Room |
|---------|--------|------|
| `TaskAssigned` | task_id, assigned_to, assigned_by | `group:{id}:coordination` |
| `TaskCompleted` | task_id, completed_by, result (JSON) | `group:{id}:coordination` |

---

## 3. Event Type Strings

Every variant has a stable string identifier returned by
`event_type()`:

```rust
impl GroupEvent {
    pub const fn event_type(&self) -> &'static str {
        match self {
            Self::Created { .. }           => "group.created",
            Self::Updated { .. }           => "group.updated",
            Self::Deleted { .. }           => "group.deleted",
            Self::MemberInvited { .. }     => "group.member_invited",
            Self::MemberJoined { .. }      => "group.member_joined",
            Self::MemberLeft { .. }        => "group.member_left",
            Self::MemberUpdated { .. }     => "group.member_updated",
            Self::Message { .. }           => "group.message",
            Self::ClusterStarted { .. }    => "group.cluster_started",
            Self::ClusterCompleted { .. }  => "group.cluster_completed",
            Self::KnowledgePublished { .. } => "group.knowledge_published",
            Self::KnowledgeValidated { .. } => "group.knowledge_validated",
            Self::PheromoneDeposited { .. } => "group.pheromone_deposited",
            Self::PheromoneDecayed { .. }  => "group.pheromone_decayed",
            Self::TaskAssigned { .. }      => "group.task_assigned",
            Self::TaskCompleted { .. }     => "group.task_completed",
        }
    }
}
```

These strings appear in:
- Bus event metadata
- API responses
- Event subscriptions

---

## 4. Serde Wire Format

The enum uses `#[serde(tag = "type", rename_all = "snake_case")]`,
producing internally tagged JSON:

```json
{
    "type": "member_joined",
    "agent_id": "researcher-01",
    "owner": "alice",
    "role": "member"
}
```

The `type` field matches the variant name in snake_case, not the
`event_type()` dotted string. Both formats are stable.

---

## 5. Durable Event Outbox

### 5.1 GroupEventRecord

Every mutation wraps its event in a durable record:

```rust
pub struct GroupEventRecord {
    pub seq: u64,              // Monotonic sequence number
    pub group_id: GroupId,     // Originating group
    pub room: String,          // Target Bus room
    pub event: GroupEvent,     // Typed event payload
    pub occurred_at: DateTime<Utc>,
}
```

### 5.2 Sequence Assignment

Events receive monotonically increasing sequence numbers from
the persisted `next_event_seq` counter:

```rust
fn append_event(
    state: &mut PersistedGroupState,
    group_id: &GroupId,
    event: GroupEvent,
) -> GroupEventRecord {
    let room = event.room(group_id);
    let record = GroupEventRecord {
        seq: state.next_event_seq,
        group_id: group_id.clone(),
        room,
        event,
        occurred_at: Utc::now(),
    };
    state.next_event_seq += 1;
    state.events.push(record.clone());
    // Prune oldest if buffer exceeds MAX_GROUP_EVENTS (4,096)
    while state.events.len() > MAX_GROUP_EVENTS {
        state.events.remove(0);
    }
    record
}
```

### 5.3 Buffer Limits

The event buffer is bounded at `MAX_GROUP_EVENTS = 4,096`. When
exceeded, the oldest events are pruned on insertion. This prevents
unbounded memory growth while preserving recent history.

### 5.4 Event Query API

```
GET /api/groups/{id}/events?after_seq=42&limit=100
```

Parameters:
- `after_seq`: return events with `seq > after_seq` (cursor-based)
- `limit`: clamped to `1..=500`

The caller must have view permission for the group (checked via
`require_view`).

---

## 6. GroupMutation Protocol

Every state-changing operation returns a `GroupMutation<T>` that
bundles the result with its event:

```rust
pub struct GroupMutation<T> {
    pub value: T,             // The created/updated entity
    pub event: GroupEventRecord,  // Durable event for Bus delivery
}
```

This ensures that:
1. The mutation is committed atomically (persisted state + event)
2. The caller receives the event for immediate Bus publication
3. The event is available in the durable outbox for replay

### 6.1 Atomic Commit

All mutations follow the pattern:

```
1. Lock state
2. Clone state into `next`
3. Apply mutation to `next`
4. Append event to `next`
5. Serialize `next` to JSON
6. Atomic file write (temp + rename)
7. Replace live state with `next`
```

If the write fails at step 6, the live state is unchanged and the
caller receives an error. No partial mutations are possible.

---

## 7. Pulse Delivery

Group events are delivered as Pulses through the workspace Bus.
Each `GroupEventRecord` is serialized and published to its resolved
room. Subscribers receive the event as a standard Bus message with:

- The `room` field from the record
- The `event_type()` string in metadata
- The serialized `GroupEvent` as the message body

### 7.1 Publication Flow

```
GroupRuntime.invite()
  |
  +-> GroupMutation { value: InviteResponse, event: GroupEventRecord }
  |
  +-> Route handler receives GroupMutation
  |
  +-> Publish event.event to Bus room event.room
  |
  +-> Subscribers (agents, SSE, WebSocket) receive the Pulse
```

### 7.2 Delivery Guarantees

Bus publication is at-most-once from the in-memory event. The
durable outbox provides the basis for at-least-once replay if
needed. Currently, replay is manual via the events query API.

---

## 8. Source References

| File | What it contains |
|------|------------------|
| `crates/roko-core/src/groups.rs` | `GroupEvent` (16 variants), `room()`, `event_type()`, serde tagging |
| `crates/roko-serve/src/group_runtime.rs` | `GroupEventRecord`, `GroupMutation`, `append_event`, `events()`, `record_event()`, `MAX_GROUP_EVENTS` |

---

## Verification

```bash
# Room routing test
cargo test --package roko-core -- \
  group_event_routes_to_its_space_partition

# Count 16 variants
grep -c 'Self::Created\|Self::Updated\|Self::Deleted\|Self::MemberInvited\|Self::MemberJoined\|Self::MemberLeft\|Self::MemberUpdated\|Self::Message\|Self::ClusterStarted\|Self::ClusterCompleted\|Self::KnowledgePublished\|Self::KnowledgeValidated\|Self::PheromoneDeposited\|Self::PheromoneDecayed\|Self::TaskAssigned\|Self::TaskCompleted' \
  crates/roko-core/src/groups.rs

# Event buffer limit
grep -n 'MAX_GROUP_EVENTS' crates/roko-serve/src/group_runtime.rs

# Stable event type strings
grep -n 'group\.' crates/roko-core/src/groups.rs | grep event_type
```
