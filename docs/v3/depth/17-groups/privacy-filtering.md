# Depth 17-04: Privacy Filtering

> GroupContextBidder, privacy-scoped prompt injection, permission
> checks, and the attention allocation protocol for group context.

**Parent**: [17-GROUPS](../../17-GROUPS.md) -- Section 6

---

## 1. Privacy Problem

When an agent belongs to multiple groups, each group's shared context
(knowledge, pheromones, coordination state) competes for limited
prompt space. Injecting everything would:

- Exceed context window limits
- Leak group-private information across boundaries
- Dilute task-relevant context with irrelevant group chatter

The privacy filtering system addresses all three concerns through
scoped bidding and permission-gated access.

---

## 2. GroupContextBidder (roko-core)

The `GroupContextBidder` struct in `crates/roko-core/src/groups.rs`
computes attention bids for group context:

```rust
pub struct GroupContextBidder {
    pub group_id: GroupId,
    pub pheromone_weight: f64,
    pub knowledge_weight: f64,
    pub coordination_weight: f64,
}
```

### 2.1 Bid Calculation

The bid value combines three normalized signals with configurable
weights:

```rust
pub fn bid_value(
    &self,
    pheromone_intensity: f64,
    knowledge_recency: f64,
    coordination_urgency: f64,
) -> f64 {
    let weighted =
        self.pheromone_weight.max(0.0)
            * pheromone_intensity.clamp(0.0, 1.0)
        + self.knowledge_weight.max(0.0)
            * knowledge_recency.clamp(0.0, 1.0)
        + self.coordination_weight.max(0.0)
            * coordination_urgency.clamp(0.0, 1.0);
    if weighted.is_finite() { weighted } else { 0.0 }
}
```

### 2.2 Safety Properties

The bid function enforces defensive invariants:

| Input | Handling |
|-------|----------|
| Negative weights | Clamped to 0.0 via `.max(0.0)` |
| Out-of-range signals | Clamped to [0.0, 1.0] |
| Infinite result | Returns 0.0 |
| NaN result | Returns 0.0 (not finite) |

This prevents adversarial or buggy inputs from producing unbounded
bids that would dominate the attention allocation.

### 2.3 Test Coverage

```rust
fn bidder_clamps_adversarial_inputs_and_rejects_non_finite_output() {
    let bidder = GroupContextBidder {
        group_id: GroupId::new("g"),
        pheromone_weight: 2.0,
        knowledge_weight: -10.0,   // Negative: clamped to 0.0
        coordination_weight: f64::INFINITY,  // Infinite weight
    };
    assert_eq!(bidder.bid_value(2.0, 1.0, 1.0), 0.0);  // Non-finite
    let bidder = GroupContextBidder {
        coordination_weight: 3.0,
        ..bidder
    };
    assert_eq!(bidder.bid_value(2.0, 1.0, 0.5), 3.5);  // Normal
}
```

---

## 3. GroupContextBidder (roko-compose)

The `GroupContextBidder` in `crates/roko-compose/src/group_context_bidder.rs`
implements the `ContextBidder` trait to inject group context into
agent prompts during dispatch.

### 3.1 Entry Model

```rust
pub struct GroupContextEntry {
    pub group_id: String,
    pub content: String,
    pub priority: SectionPriority,
    pub source: String,    // "pheromone", "knowledge", "convention"
}
```

### 3.2 ContextBidder Implementation

```rust
impl ContextBidder for GroupContextBidder {
    fn bidder_id(&self) -> &'static str {
        "group-context"
    }

    fn propose_context(
        &self,
        _provider: &ContextProvider,
        _request: &ContextRequest,
    ) -> Vec<ContextCandidate> {
        self.entries
            .iter()
            .map(|entry| {
                let section = ContextSection::scoped(
                    PromptSection::new(
                        format!("group:{}", entry.group_id),
                        entry.content.clone(),
                    )
                    .with_priority(entry.priority),
                    ContextSource::Pheromone {
                        kind: entry.source.clone(),
                        source: entry.group_id.clone(),
                    },
                    ContextPurpose::TaskGuidance,
                    ContextScope::Global { reason: ... },
                    format!("via group-context (group {}, ...)", ...),
                );
                ContextCandidate {
                    section,
                    relevance: 0.7,
                    bidder: AttentionBidder::Neuro,
                }
            })
            .collect()
    }
}
```

### 3.3 Key Design Choices

| Decision | Rationale |
|----------|-----------|
| Fixed relevance 0.7 | Group context is important but not dominant |
| `AttentionBidder::Neuro` | Routes through the knowledge attention system |
| `ContextSource::Pheromone` | Tags provenance for filtering and auditing |
| `ContextPurpose::TaskGuidance` | Groups provide operational guidance |

---

## 4. Permission-Gated Access

Every group operation that exposes group state checks permissions
before returning data.

### 4.1 Visibility Check

Groups use a `can_view` function to determine if a caller may see
the group:

- **Public groups**: visible to all authenticated callers
- **Private groups**: visible only to the owner and members whose
  `owner` matches the caller

### 4.2 Read Permission

Member-level read access is checked via `Group::can_read`:

```rust
pub fn can_read(&self, agent_id: &str) -> bool {
    self.member(agent_id)
        .is_some_and(|member| member.permissions.read)
}
```

### 4.3 Write Permission

Member-level write access is checked via `Group::can_write`:

```rust
pub fn can_write(&self, agent_id: &str) -> bool {
    self.member(agent_id)
        .is_some_and(|member| member.permissions.write)
}
```

### 4.4 Permission Levels

```rust
pub struct MemberPermissions {
    pub read: bool,
    pub write: bool,
    pub execute: bool,
}

impl MemberPermissions {
    pub const FULL: Self = Self {
        read: true, write: true, execute: true,
    };
    pub const READ_ONLY: Self = Self {
        read: true, write: false, execute: false,
    };
}
```

### 4.5 Role-Permission Interaction

Observers are restricted regardless of their explicit permissions:

```rust
// In deposit_pheromone:
if !member.permissions.write || member.role == MemberRole::Observer {
    return Err(GroupRuntimeError::Forbidden(
        "member lacks group write permission".to_string(),
    ));
}
```

---

## 5. Cross-Group Privacy Boundary

When an agent belongs to groups A and B:

1. The `GroupContextBidder` generates separate entries per group
2. Each entry carries its `group_id` in the `ContextSource`
3. The attention system selects entries based on relevance to the
   current task
4. Group A's knowledge entries do not appear in Group B's Bus room

### 5.1 Bus Room Isolation

Each group's events are partitioned into separate Bus rooms:

| Room pattern | Content |
|-------------|---------|
| `group:{id}` | General group events |
| `group:{id}:knowledge` | Knowledge publish/validate |
| `group:{id}:pheromones` | Pheromone deposit/decay |
| `group:{id}:coordination` | Task assign/complete |

An agent subscribed to Group A's rooms does not receive Group B's
events. The Bus subscription is scoped to the agent's memberships.

### 5.2 Pheromone Field Isolation

Pheromone queries are scoped by `group_id`. The `pheromones` method
requires passing a `group_id`, and the runtime validates the caller
has view permission for that specific group. There is no cross-group
pheromone query.

---

## 6. Event Visibility

The `events` method on `GroupRuntime` checks permissions before
returning durable group events:

```rust
pub async fn events(
    &self,
    group_id: &GroupId,
    actor: &str,
    after_seq: Option<u64>,
    limit: usize,
) -> Result<Vec<GroupEventRecord>, GroupRuntimeError> {
    let state = self.state.lock().await;
    let group = require_group(&state, group_id)?;
    require_view(group, actor)?;   // Privacy gate
    // ... return filtered events
}
```

The `limit` parameter is clamped to `1..=500` to prevent unbounded
responses.

---

## 7. Source References

| File | What it contains |
|------|------------------|
| `crates/roko-core/src/groups.rs` | `GroupContextBidder` (bid calculation), `MemberPermissions`, `MemberRole`, `Group::can_read`, `Group::can_write` |
| `crates/roko-compose/src/group_context_bidder.rs` | `GroupContextBidder` (prompt injection), `GroupContextEntry`, `ContextBidder` impl |
| `crates/roko-serve/src/group_runtime.rs` | `require_view`, `require_owner`, permission checks on all mutation and query paths |

---

## Verification

```bash
# Bidder safety test
cargo test --package roko-core -- \
  bidder_clamps_adversarial_inputs_and_rejects_non_finite_output

# GroupContextBidder in compose
cargo test --package roko-compose -- group_context

# Permission structs
grep -n 'pub const FULL\|pub const READ_ONLY' \
  crates/roko-core/src/groups.rs

# Runtime permission checks
grep -n 'require_view\|require_owner\|Forbidden' \
  crates/roko-serve/src/group_runtime.rs | head -20
```
