# 05-agent/agent-pools -- Agent Pools

> The removed AgentPool (sequential, single-role) and MultiAgentPool (parallel, multi-role):
> lifecycle states, warm pre-spawning, concurrency limits, fallback retry, and
> integration status.

**Parent:** [05-AGENT](../../05-AGENT.md)

**Source (removed 2026-10-01, gap-ee8dc0):** `crates/roko-agent/src/pool.rs`, `crates/roko-agent/src/multi_pool.rs`

---

## 1. Status

> **Removed.** `AgentPool` and `MultiAgentPool` never ran outside their own
> tests, and were deleted on 2026-10-01 (gap-ee8dc0) together with the TUI
> roster modal built for them (bug-2f33d6). Agents are constructed on demand
> via `create_agent_for_model()` and tracked by `ProcessSupervisor`; the only
> pool is `WarmPool` (`crates/roko-cli/src/dispatch/warm_pool.rs`), a per-role
> LRU of warm agent handles. The rest of this page records the removed design.

---

## 2. Two Pool Types

Roko provides two pool implementations:

1. **`AgentPool`** -- Manages a queue of tasks for a single role. Tasks execute
   sequentially. If the primary agent fails, the pool retries with a fallback
   (different model).

2. **`MultiAgentPool`** -- Manages multiple pools across roles for concurrent
   execution. Supports warm pre-spawning so agents are ready without cold-start
   latency.

---

## 3. AgentInstanceId

Every agent instance gets a unique identifier:

```rust
pub struct AgentInstanceId {
    pub role: AgentRole,
    pub instance: String,  // e.g., "plan42-task3"
}
```

The `key()` method produces a string like `"implementer-plan42-task3"` for logs,
metrics, and TUI display. The `matches()` method supports plan-based filtering
for bulk operations (e.g., kill all agents working on plan 42).

---

## 4. Instance Lifecycle

```rust
pub enum InstanceStatus {
    Warm,       // Pre-spawned, waiting for work
    Pending,    // Queued, waiting its turn
    Running,    // Currently executing
    Completed,  // Finished successfully
    Failed,     // Finished with error
    Killed,     // Terminated externally
}
```

State transitions:

```
Warm --work-arrives--> Pending --turn-comes--> Running
                                                  |
                                          +-------+-------+
                                          v               v
                                      Completed        Failed
                                                         |
                                                    +----+----+
                                                    v         v
                                              TryFallback   Killed
```

---

## 5. AgentPool

The single-role sequential pool:

```rust
pub struct AgentPool {
    role: AgentRole,
    primary: Arc<dyn Agent>,
    fallback: Option<Arc<dyn Agent>>,
    pending: VecDeque<AgentTask>,
    statuses: Vec<(AgentInstanceId, InstanceStatus)>,
    completed: VecDeque<TaskOutcome>,
    active_task: Option<AgentInstanceId>,
}
```

### Fallback retry

When an agent fails, the pool checks for a configured fallback:

```
Primary (Opus) fails -> Fallback (Sonnet) retries -> Final result
```

This provides automatic model tier de-escalation: if the expensive model fails
(rate limit, timeout, context overflow), the cheaper model gets a chance before
the task is marked as failed.

### AgentTask

```rust
pub struct AgentTask {
    pub id: AgentInstanceId,
    pub prompt: Signal,
    pub context: Context,
    pub priority: u32,
}
```

The `priority` field enables scheduling: higher-priority tasks (e.g., gate
validation blocking the merge queue) preempt lower-priority tasks.

### TaskOutcome

```rust
pub enum TaskOutcome {
    Success(AgentResult),
    Failed(AgentResult),
    Cancelled,
}
```

The `AgentResult` inside `Failed` still contains the agent's output -- even
failed runs produce diagnostic information for logging and retry decisions.

---

## 6. MultiAgentPool

The multi-role concurrent pool:

```rust
pub struct MultiAgentPool {
    active: HashMap<AgentInstanceId, ActiveEntry>,
    warm: HashMap<(AgentRole, String), WarmEntry>,
    fallbacks: HashMap<AgentRole, Arc<dyn Agent>>,
    concurrency_limits: HashMap<AgentRole, usize>,
    default_concurrency: usize,  // Default: 4
}
```

### Warm pool

Agents are constructed and held in memory before work arrives:

```rust
struct WarmEntry {
    agent: Arc<dyn Agent>,
    spawned_at: Instant,
}
```

When a task arrives for a role with a warm agent available, the pool promotes
the warm agent to active status instead of constructing a new one, eliminating
cold-start latency.

`evict_stale_warm()` removes entries idle longer than a configurable timeout
(default: 5 minutes) to prevent memory waste.

### Concurrency control

Each role can have its own limit:

```rust
pool.set_concurrency_limit(AgentRole::Architect, 1);    // Serial
pool.set_concurrency_limit(AgentRole::Implementer, 4);  // Parallel
pool.set_concurrency_limit(AgentRole::SnapshotComparator, 8); // High parallelism
```

When a role hits its limit, new tasks queue in `Pending` status until a running
instance completes.

### Bulk operations

| Method | Purpose |
|--------|---------|
| `kill_all()` | Terminate all active instances (plan completion, Ctrl-C) |
| `kill_by_plan(plan_id)` | Terminate instances matching a plan (plan failure) |
| `kill_by_role(role)` | Terminate all instances of a specific role |

These work through the `ProcessSupervisor` for subprocess-based agents -- the
supervisor sends SIGTERM and waits for graceful shutdown before SIGKILL.

---

## 7. Relationship to ProcessSupervisor

The pool layer sits above the supervisor:

```
MultiAgentPool
    |
    +-- AgentPool (per role)
    |   +-- AgentInstanceId + status tracking
    |   +-- fallback retry logic
    |
    v
create_agent_for_model() -> Box<dyn Agent>
    |
    v
Agent::run() -> AgentResult
    |
    +-- ClaudeCliAgent -> ProcessSupervisor (subprocess)
    +-- AnthropicApiAgent -> HTTP client (no supervisor)
    +-- GeminiApiAgent -> HTTP client (no supervisor)
```

The pool decides *when* to spawn and *which model* to use. The supervisor
handles *how* the subprocess runs: spawning, monitoring exit codes, and
graceful shutdown.

---

## 8. TUI Integration

The TUI roster modal (`tui/modals/agent_pool_modal.rs`, removed with the
pools) showed every pool entry:

| Column | Content |
|--------|---------|
| Role | AgentRole label |
| Model | Backend model slug |
| Task | Current task ID |
| Tokens | Input/output token counts |
| Cost | Accumulated USD |
| State | InstanceStatus |
| Context% | Context window utilization |

---

## 9. Future Architecture

The intended flow when pools are wired into the Graph engine:

```
Current:
  Graph engine -> create_agent_for_model() -> Agent::run()

Future:
  Graph engine -> MultiAgentPool.submit(role, task)
      -> warm-pool promotion or cold-start construction
      -> create_agent_for_model() via provider adapter
      -> execution with timeout + cancellation
      -> fallback retry on failure
      -> lifecycle state tracking
      -> bulk kill on plan completion
```

---

## 10. Citations

1. `crates/roko-agent/src/pool.rs` (removed) -- AgentPool, AgentInstanceId,
   InstanceStatus, AgentTask, TaskOutcome.
2. `crates/roko-agent/src/multi_pool.rs` (removed) -- MultiAgentPool, WarmEntry,
   ActiveEntry, concurrency control.
3. `crates/roko-runtime/src/supervisor.rs` -- ProcessSupervisor for subprocess
   lifecycle.
4. `crates/roko-cli/src/tui/modals/agent_pool_modal.rs` (removed) -- TUI pool view.
