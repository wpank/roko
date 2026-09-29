# Depth 17-03: Knowledge and Pheromone Flows

> Knowledge publishing and validation within groups, pheromone deposit
> and decay mechanics, aggregated fields, and the interaction between
> knowledge policy and pheromone signals.

**Parent**: [17-GROUPS](../../17-GROUPS.md) -- Sections 4, 5

---

## 1. Group Knowledge Policy

Each group configures how agents contribute to the shared knowledge
partition via the `KnowledgePolicy` enum:

```rust
pub enum KnowledgePolicy {
    Open,           // Any member with write permission can publish
    WriteLeader,    // Only the group leader can publish
    Curated,        // Members propose; leader approves
}
```

**Source**: `crates/roko-core/src/groups.rs`

### 1.1 Policy Effects

| Policy | Who publishes | Validation |
|--------|--------------|------------|
| `Open` | Any member with `write = true` | None required |
| `WriteLeader` | Only `MemberRole::Leader` | Role check at publish time |
| `Curated` | Members propose, leader approves | Two-step lifecycle |

### 1.2 Configuration

```toml
[[groups]]
name = "research-team"
knowledge_policy = "curated"
```

Parsing is case-insensitive with hyphen-to-underscore normalization:

```rust
impl FromStr for KnowledgePolicy {
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase()
            .replace('-', "_").as_str()
        {
            "open" => Ok(Self::Open),
            "write_leader" => Ok(Self::WriteLeader),
            "curated" => Ok(Self::Curated),
            _ => Err(format!("unknown knowledge policy '{value}'")),
        }
    }
}
```

---

## 2. Knowledge Events

Two typed events track knowledge flow within a group:

### 2.1 KnowledgePublished

Emitted when an agent publishes a knowledge entry to the group:

```rust
GroupEvent::KnowledgePublished {
    entry_id: String,    // Stable knowledge entry identifier
    author: String,      // Agent that authored the entry
    topic: String,       // Subject classification
}
```

Routed to Bus room `group:{group_id}:knowledge`.

### 2.2 KnowledgeValidated

Emitted when a peer agent validates or endorses a knowledge entry:

```rust
GroupEvent::KnowledgeValidated {
    entry_id: String,    // Entry being validated
    validator: String,   // Agent performing validation
}
```

Also routed to `group:{group_id}:knowledge`.

Knowledge events are separated into their own Bus room so agents
that only care about pheromone or coordination events are not
burdened with knowledge traffic.

---

## 3. Pheromone Data Model

### 3.1 GroupPheromone

The persistent pheromone record:

```rust
pub struct GroupPheromone {
    pub group_id: GroupId,
    pub depositor: String,           // Agent that deposited
    pub signal_type: String,         // Classification (e.g., "task-found")
    pub position_hint: Option<String>,  // Spatial/contextual hint
    pub metadata: serde_json::Value, // Arbitrary payload
    pub deposited_at: DateTime<Utc>,
}
```

### 3.2 StoredPheromone (Internal)

The runtime wraps each pheromone with balance and timestamp:

```rust
struct StoredPheromone {
    id: String,                      // "phr-{uuid}"
    pheromone: GroupPheromone,
    balance: f64,                    // Current strength [0.0, 1.0]
    last_touched_at: DateTime<Utc>,  // Last deposit/refresh
}
```

### 3.3 PheromoneView (API Response)

The public view returned to callers:

```rust
pub struct PheromoneView {
    pub id: String,
    pub pheromone: GroupPheromone,    // Flattened via serde
    pub balance: f64,                // Demurrage-adjusted balance
    pub last_touched_at: DateTime<Utc>,
}
```

---

## 4. Pheromone Deposit

### 4.1 Deposit Flow

```
POST /api/groups/{id}/pheromones
{
    "depositor": "scout-agent",
    "signal_type": "resource-found",
    "metadata": { "path": "/data/results" }
}
```

The runtime enforces:

1. **Membership**: depositor must be a group member
2. **Ownership**: caller must own the depositing agent (or be
   the group owner)
3. **Write permission**: member must have `write = true` and must
   not be an Observer
4. **Signal type format**: 1..=128 ASCII alphanumeric chars plus
   `.`, `_`, `-`

### 4.2 Refresh vs Create

If the depositor already has a pheromone of the same `signal_type`,
the deposit refreshes the existing entry:

```rust
if let Some(existing) = field.iter_mut().find(|candidate| {
    candidate.pheromone.depositor == depositor
        && candidate.pheromone.signal_type == signal_type.trim()
}) {
    existing.balance = 1.0;          // Reset to full strength
    existing.last_touched_at = now;
    existing.pheromone.metadata = metadata;
    existing.pheromone.position_hint = position_hint;
    existing.pheromone.deposited_at = now;
    existing.clone()
} else {
    // Create new with balance = 1.0
}
```

This prevents unbounded pheromone accumulation: each
(depositor, signal_type) pair maps to at most one active entry.

### 4.3 Deposit Event

Every deposit emits a `PheromoneDeposited` event:

```rust
GroupEvent::PheromoneDeposited {
    depositor: String,
    signal_type: String,
    intensity: f64,      // Current balance at deposit time
}
```

Routed to `group:{group_id}:pheromones`.

---

## 5. Pheromone Decay

Pheromone balance decays exponentially over time. The decay is
computed lazily on read, not by a background timer.

### 5.1 Decay Formula

```rust
fn balance_at(&self, decay_modifier: f64, now: DateTime<Utc>) -> f64 {
    let elapsed_hours = (now - self.last_touched_at)
        .num_milliseconds().max(0) as f64 / 3_600_000.0;
    let daily_rate = (BASE_PHEROMONE_DECAY_PER_DAY * decay_modifier)
        .clamp(0.0, 1.0);
    let retention = (1.0 - daily_rate).powf(elapsed_hours / 24.0);
    (self.balance * retention).clamp(0.0, 1.0)
}
```

| Parameter | Value | Meaning |
|-----------|-------|---------|
| `BASE_PHEROMONE_DECAY_PER_DAY` | 0.01 | 1% daily baseline decay |
| `pheromone_decay_rate` | Group config (default 1.0) | Multiplier on base rate |

### 5.2 Decay Examples

With default settings (decay_modifier = 1.0):

| Hours elapsed | Balance |
|---------------|---------|
| 0 | 1.000 |
| 24 | 0.990 |
| 168 (1 week) | 0.932 |
| 720 (30 days) | 0.740 |

With fast decay (decay_modifier = 10.0):

| Hours elapsed | Balance |
|---------------|---------|
| 0 | 1.000 |
| 24 | 0.905 |
| 168 (1 week) | 0.488 |

### 5.3 Decay Event

When pheromones are pruned below threshold:

```rust
GroupEvent::PheromoneDecayed {
    count_removed: usize,
    threshold: f64,
}
```

Routed to `group:{group_id}:pheromones`.

---

## 6. Pheromone Queries

### 6.1 Raw Query

```
GET /api/groups/{id}/pheromones?signal_type=resource-found&min_balance=0.5
```

Parameters:
- `signal_type`: optional filter by exact type
- `min_balance`: optional minimum balance threshold (0.0 to 1.0)

Results are sorted by balance descending, then by `last_touched_at`
descending.

### 6.2 Aggregated Query

```
GET /api/groups/{id}/pheromones/aggregated
```

Collapses all depositors of the same `signal_type` into a single
entry. The aggregate balance is the sum of individual balances,
capped at 1.0:

```rust
pub async fn aggregated_pheromones(&self, ...) -> ... {
    let raw = self.pheromones(group_id, actor, None, None).await?;
    let mut aggregated: BTreeMap<String, PheromoneView> = BTreeMap::new();
    for view in raw {
        let key = view.pheromone.signal_type.clone();
        aggregated
            .entry(key)
            .and_modify(|existing| {
                existing.balance = (existing.balance + view.balance).min(1.0);
                if view.last_touched_at > existing.last_touched_at {
                    existing.last_touched_at = view.last_touched_at;
                    existing.pheromone.metadata = view.pheromone.metadata.clone();
                }
            })
            .or_insert(view);
    }
    Ok(aggregated.into_values().collect())
}
```

---

## 7. Pheromone Field Summary

For efficient polling, the API provides a lightweight summary:

```rust
pub struct PheromoneFieldSummary {
    pub group_id: GroupId,
    pub count: usize,
    pub types: Vec<String>,
}
```

---

## 8. Knowledge-Pheromone Interaction

Knowledge and pheromones complement each other:

- **Knowledge**: durable facts that persist and progress through tiers
- **Pheromones**: ephemeral signals that decay, guiding short-term behavior

An agent discovering useful knowledge might:
1. Publish the knowledge entry (durable, via knowledge policy)
2. Deposit a pheromone (ephemeral, signals other agents to attend)

The `GroupContextBidder` combines both signals when computing
attention allocation for prompt context (see
[privacy-filtering.md](./privacy-filtering.md)).

---

## 9. Source References

| File | What it contains |
|------|------------------|
| `crates/roko-core/src/groups.rs` | `GroupPheromone`, `PheromoneDeposit`, `PheromoneQuery`, `PheromoneFieldSummary`, `KnowledgePolicy`, `GroupEvent::KnowledgePublished`, `GroupEvent::KnowledgeValidated`, `GroupEvent::PheromoneDeposited`, `GroupEvent::PheromoneDecayed` |
| `crates/roko-serve/src/group_runtime.rs` | `StoredPheromone`, `PheromoneView`, `deposit_pheromone`, `pheromones`, `aggregated_pheromones`, decay calculation |

---

## Verification

```bash
# Pheromone types exist
grep -n 'pub struct GroupPheromone\|pub struct PheromoneView' \
  crates/roko-core/src/groups.rs crates/roko-serve/src/group_runtime.rs

# Decay constant
grep -n 'BASE_PHEROMONE_DECAY_PER_DAY' \
  crates/roko-serve/src/group_runtime.rs

# Knowledge policy enum
grep -n 'pub enum KnowledgePolicy' crates/roko-core/src/groups.rs

# Group runtime tests
cargo test --package roko-serve -- group
```
