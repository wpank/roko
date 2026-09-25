# Depth 17-02: Coordination Patterns

> Four coordination modes detailed: Stigmergic, Pipeline, Broadcast,
> and LeaderFollower. Assignment strategies, task lifecycle, and
> mode-specific runtime behavior.

**Parent**: [17-GROUPS](../../17-GROUPS.md) -- Section 3

---

## 1. CoordinationMode Enum

Every group selects exactly one coordination mode at creation time.
The mode determines how agents within the group discover, claim, and
complete work items.

**Source**: `crates/roko-core/src/groups.rs`

```rust
pub enum CoordinationMode {
    Stigmergic,       // Pheromone-guided indirect coordination
    Pipeline,         // Ordered stage-to-stage handoff
    Broadcast,        // All members receive every message
    LeaderFollower,   // Central leader assigns tasks to followers
}
```

All four values use stable `snake_case` serde representation.
Parsing is case-insensitive and normalizes hyphens to underscores:

```rust
impl FromStr for CoordinationMode {
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().replace('-', "_").as_str() {
            "stigmergic"      => Ok(Self::Stigmergic),
            "pipeline"        => Ok(Self::Pipeline),
            "broadcast"       => Ok(Self::Broadcast),
            "leader_follower" => Ok(Self::LeaderFollower),
            _ => Err(format!("unknown coordination mode '{value}'")),
        }
    }
}
```

---

## 2. Stigmergic Mode (Default)

Stigmergic coordination uses indirect communication through shared
pheromone deposits. Agents do not send direct messages or receive
explicit assignments. Instead, they observe the pheromone field and
act on the strongest signals.

### 2.1 How It Works

1. Agent A completes work and deposits a pheromone:
   `POST /api/groups/{id}/pheromones`
   with `signal_type: "task-complete"` and metadata describing results
2. Agent B queries the pheromone field:
   `GET /api/groups/{id}/pheromones`
3. The returned pheromones are sorted by balance (highest first)
4. Agent B sees A's deposit and decides what to do next

### 2.2 Pheromone Decay

Pheromones decay over time using configurable exponential decay:

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

- `BASE_PHEROMONE_DECAY_PER_DAY`: 0.01 (1% per day baseline)
- `pheromone_decay_rate`: per-group config multiplier (default 1.0)
- Balance starts at 1.0 on deposit and decays toward 0.0
- Refreshing a pheromone resets its balance to 1.0

### 2.3 Use Cases

- Swarm exploration: agents mark explored areas
- Emergent task allocation: strongest pheromone attracts next agent
- Feedback loops: successful paths accumulate stronger signals

---

## 3. Pipeline Mode

Pipeline coordination orders agents into sequential stages. Work
flows from one stage to the next in a defined sequence.

### 3.1 Stage Flow

```
Agent A (stage 1) --> Agent B (stage 2) --> Agent C (stage 3)
   research            implementation         verification
```

Pipeline mode uses the same Bus event infrastructure as other modes
but imposes ordering through `ClusterStarted` and `ClusterCompleted`
events:

```rust
GroupEvent::ClusterStarted {
    cluster_id: String,
    pipeline: serde_json::Value,
    agents: Vec<String>,
}

GroupEvent::ClusterCompleted {
    cluster_id: String,
    outcome: String,
    duration_secs: u64,
}
```

### 3.2 Stage Handoff

When a stage completes, the completing agent publishes a
`ClusterCompleted` event to the group's Bus room. The next stage
agent subscribes to these events and begins work when it sees the
predecessor complete.

---

## 4. Broadcast Mode

Broadcast mode delivers every group message to every member. There
is no filtering, no routing, and no stage ordering.

### 4.1 Message Delivery

All events published to the group's root Bus room
(`group:{group_id}`) reach every member with read permission.
The `GroupEvent::Message` variant carries the content:

```rust
GroupEvent::Message {
    from: String,
    content: String,
    tags: Vec<String>,
}
```

### 4.2 Use Cases

- Shared awareness: all agents see all events
- Consensus: agents vote on proposals
- Simple teams: small groups where all context is shared

---

## 5. LeaderFollower Mode

LeaderFollower mode designates one agent as the leader. The leader
assigns tasks to follower agents and tracks their completion.

### 5.1 Leader Configuration

```rust
pub struct LeaderConfig {
    pub leader_agent: String,
    pub assignment_strategy: AssignmentStrategy,
    pub max_concurrent_tasks: usize,
}
```

The configured leader must be a member of the group. This is
validated at reconciliation time:

```rust
if coordination == CoordinationMode::LeaderFollower {
    let leader = definition.leader.as_deref().ok_or_else(|| {
        GroupRuntimeError::Invalid(format!(
            "configured leader_follower group '{}' requires a leader",
            definition.name
        ))
    })?;
    if !definition.members.iter().any(|member| member == leader) {
        return Err(GroupRuntimeError::Invalid(format!(
            "configured leader '{}' is not a member of group '{}'",
            leader, definition.name
        )));
    }
}
```

### 5.2 Assignment Strategies

```rust
pub enum AssignmentStrategy {
    RoundRobin,         // Cycle through followers
    CapabilityMatch,    // Match task requirements to agent skills
    LoadBalanced,       // Assign to least-loaded follower
    CascadeRouter,      // Use the learned cascade routing table
}
```

### 5.3 Task Lifecycle

Task assignment and completion flow through typed events:

```rust
pub struct TaskAssignment {
    pub task_id: String,
    pub assigned_to: String,
    pub assigned_by: String,
    pub description: String,
    pub deadline: Option<DateTime<Utc>>,
}

pub struct TaskCompletion {
    pub task_id: String,
    pub completed_by: String,
    pub result_id: Option<String>,
    pub duration_secs: u64,
}
```

These are published as `GroupEvent::TaskAssigned` and
`GroupEvent::TaskCompleted` to the `group:{id}:coordination` room.

---

## 6. TOML Configuration

Groups can be declared in `roko.toml`:

```toml
[[groups]]
name = "research-team"
description = "Parallel research agents"
coordination = "leader_follower"
leader = "lead-researcher"
assignment_strategy = "capability_match"
members = ["lead-researcher", "web-searcher", "analyzer"]
max_members = 8
knowledge_policy = "curated"
pheromone_decay_rate = 0.5
```

The `reconcile_definitions` function in `group_runtime.rs` processes
these entries at startup, creating or updating groups to match the
TOML declarations. Declarative groups use `owner = "local"` and
`auto_accept = true`.

---

## 7. Mode Selection Guidelines

| Mode | Best for | Agents | Communication |
|------|----------|--------|---------------|
| Stigmergic | Swarm exploration, emergent behavior | Many, homogeneous | Indirect (pheromones) |
| Pipeline | Sequential workflows | Few, specialized | Stage handoff |
| Broadcast | Shared awareness, consensus | Small teams | All-to-all |
| LeaderFollower | Managed teams, task delegation | Hierarchical | Leader directs |

---

## 8. Source References

| File | What it contains |
|------|------------------|
| `crates/roko-core/src/groups.rs` | `CoordinationMode`, `AssignmentStrategy`, `LeaderConfig`, `TaskAssignment`, `TaskCompletion` |
| `crates/roko-serve/src/group_runtime.rs` | Mode validation, leader configuration checks, reconciliation |

---

## Verification

```bash
# Enum round-trips
cargo test --package roko-core -- enums_use_stable_snake_case_wire_names

# Reconciliation validates leader presence
grep -n 'leader_follower.*requires.*leader' \
  crates/roko-serve/src/group_runtime.rs

# Assignment strategy parsing
grep -n 'impl FromStr for AssignmentStrategy' \
  crates/roko-core/src/groups.rs
```
