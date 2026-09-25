# Cognitive Kernel Safety

> **v3 depth file** -- `/docs/v3/depth/12-safety/cognitive-kernel-safety.md`
> Canonical source: v1 `docs/v1/11-safety/14-cognitive-kernel-safety.md`
> Status: **Partial**. SafetyLayer (composite Policy), ToolDispatcher (syscall-style
> dispatch), ProcessSupervisor (process-level signals), event bus (signal delivery), and
> RateLimiter (resource limits) are built. Cognitive Namespaces, full Cognitive Signal
> enum, and universal Engram Syscall enforcement remain design targets.

---

## 1. OS-Level Primitives for Agents

Roko implements OS-level primitives for agents inspired by what Linux got right for
process management. Four primitives adapt kernel concepts to cognitive agent management:

| Linux primitive | Purpose | Roko equivalent | Purpose |
|---|---|---|---|
| Namespaces (PID, net, mount) | Process isolation | Cognitive Namespaces | Knowledge isolation |
| Signals (SIGTERM, SIGKILL) | Process control | Cognitive Signals | Agent behavioral control |
| Scheduler (CFS) | CPU time allocation | Cognitive Scheduling | Reasoning resource allocation |
| System calls (syscall table) | Hardware access control | Engram Syscalls | Action access control |

The analogy is structural: Roko's primitives solve the same problems for agents that
Linux kernel primitives solve for processes -- isolation, control, fair scheduling, and
mediated access.

---

## 2. Cognitive Namespaces

Cognitive Namespaces are isolated knowledge spaces with explicit, auditable
cross-namespace channels.

### 2.1 Safety properties

**Isolation guarantee.** An agent's private knowledge is isolated within its namespace.
No other agent can access it without an explicit channel. This prevents:

- Knowledge poisoning: a compromised agent cannot directly corrupt another agent's
  Neuro store.
- Information leakage: proprietary strategies stay within their namespace.
- Cross-contamination: experimental knowledge cannot accidentally pollute production
  knowledge.

**Explicit channels.** Knowledge sharing happens only through declared channels that
log every transfer:

- Audit trail: every cross-namespace transfer is recorded.
- Rate limiting: channels can limit transfer rate to prevent flooding.
- Kind filtering: only specific Signal kinds can flow through a channel.
- Directionality: channels are one-way.

### 2.2 Relation to PathPolicy

`PathPolicy` provides filesystem-level isolation. Cognitive Namespaces extend this to
knowledge-level isolation. The two compose:

- `PathPolicy` prevents reading files outside the worktree.
- Namespaces prevent reading Signals outside the namespace.
- Together they enforce both physical and logical isolation.

---

## 3. Cognitive Signals (Typed Interrupts)

Cognitive Signals are typed interrupts that alter agent behavior without killing the
process. Unlike Unix SIGKILL, no Cognitive Signal causes abrupt termination with state
loss.

```rust
pub enum CognitiveSignal {
    Pause,                       // Suspend, serialize state
    Resume,                      // Resume from serialized state
    Reprioritize(TaskId),        // Change task priority
    InjectContext(Box<Signal>),  // Add context mid-reasoning
    Escalate,                    // Switch to stronger model
    Cooldown,                    // Reduce arousal, slow down
    Explore,                     // Switch to exploratory mode
    Shutdown,                    // Graceful termination
}
```

### 3.1 Priority ordering

| Signal | Priority | Effect on current work |
|---|---|---|
| Shutdown | 1 (highest) | Complete current unit, then exit |
| Pause | 2 | Serialize state, suspend immediately |
| Escalate | 3 | Switch model tier, continue work |
| Cooldown | 4 | Modulate affect, continue work |
| Reprioritize | 5 | Reorder task queue, continue work |
| InjectContext | 6 | Add to context, continue work |
| Explore | 7 | Change exploration mode, continue work |
| Resume | 8 (lowest) | Resume from suspended state |

Higher-priority signals preempt lower-priority ones.

### 3.2 Human oversight (EU AI Act Article 14)

Cognitive Signals satisfy the requirement for human oversight of autonomous agents:

- **Pause**: stop mid-execution if behavior seems anomalous.
- **InjectContext**: provide new safety constraints without restarting.
- **Escalate**: force deeper reasoning when the agent is cutting corners.
- **Cooldown**: reduce risk-taking when conditions are volatile.
- **Shutdown**: graceful termination with full state preservation.

### 3.3 Timeout escalation

Unacknowledged signals escalate: Cooldown becomes Pause, Pause becomes Shutdown. This
prevents an agent from ignoring safety signals by simply not processing them.

### 3.4 Priority inversion prevention

When a high-priority signal depends on completion of a low-priority task (e.g., Shutdown
blocked by a task holding a file lock), priority inheritance (Sha, Rajkumar & Lehoczky,
1990) temporarily elevates the blocking task's priority.

---

## 4. Cognitive Scheduling

Cognitive Scheduling allocates reasoning resources based on priority, deadline, and
expected value:

```
cognitive_priority = task_urgency * expected_value * (1 / cognitive_cost)
```

### 4.1 Safety properties

**Starvation prevention.** Fairness properties analogous to CFS:

- No task monopolizes reasoning resources indefinitely.
- Long-waiting tasks receive priority boosts.
- Minimum time slices ensure every queued task makes progress.

**Deadline enforcement.** Tasks with deadlines use Earliest Deadline First (EDF).
Safety-critical tasks always receive deadline priority.

**Cost accounting.** Every reasoning step has a cost. When budget limits are reached:

- Soft limit: emit Warning, reduce tier (T2 -> T1 -> T0).
- Hard limit: emit Cooldown, pause non-critical work.
- Exhaustion: emit Shutdown for non-essential agents.

---

## 5. Engram Syscalls

Every meaningful agent action passes through a controlled interface. The `Policy` trait
is the enforcement mechanism:

```rust
pub trait Policy: Send + Sync {
    fn decide(&self, engrams: &[Signal]) -> Vec<Signal>;
}
```

### 5.1 Four decision modes

| Mode | What happens |
|---|---|
| Permit | Allow the action, log approval |
| Deny | Block the action, return error |
| Modify | Allow but alter (reduce scope, scrub secrets) |
| Log | Allow but create detailed audit record |

### 5.2 Composite policies

Multiple Policy implementations compose via the product rule: all policies must agree
to Permit; any single Deny blocks.

The current composite chain:

```
SafetyLayer::check_pre_execution()
  --> BashPolicy     (deny dangerous shell commands)
  --> GitPolicy      (deny force-push, protected branches)
  --> NetworkPolicy  (deny private networks, enforce HTTPS)
  --> PathPolicy     (deny paths outside worktree)
  --> RateLimiter    (deny if rate limit exceeded)
  --> SandboxPolicy  (enforce level-appropriate restrictions)

[post-execution]
  --> ScrubPolicy    (modify: redact secrets in output)
```

### 5.3 Single enforcement point

The ToolDispatcher already implements syscall-style enforcement:

```
dispatch() pipeline:
  1. validate          (schema check)
  2. tool_filter       (allowlist/denylist)
  3. permission        (role-based check)
  4. safety pre-check  (SafetyLayer)
  5. handler           (execute)
  6. truncate          (output size limit)
  7. safety post-check (ScrubPolicy)
```

Each step emits an audit Signal via `emit_audit()`.

---

## 6. Defense in Depth via Kernel Primitives

The four primitives compose into a defense-in-depth model:

```
+---------------------------------------------+
|         Engram Syscalls (outermost)          |
|  Every action passes through Policy.decide() |
|                                              |
|  +-------------------------------------+    |
|  |     Cognitive Namespaces             |    |
|  |  Knowledge isolation + channels      |    |
|  |                                      |    |
|  |  +-------------------------------+   |    |
|  |  |    Cognitive Scheduling        |   |    |
|  |  |  Fair resource allocation      |   |    |
|  |  |                                |   |    |
|  |  |  +-------------------------+   |   |    |
|  |  |  |   Cognitive Signals      |   |   |    |
|  |  |  |  Human intervention      |   |   |    |
|  |  |  +-------------------------+   |   |    |
|  |  +-------------------------------+   |    |
|  +-------------------------------------+    |
+---------------------------------------------+
```

- Engram Syscalls prevent unauthorized actions (outermost).
- Cognitive Namespaces prevent unauthorized knowledge access.
- Cognitive Scheduling prevents resource starvation and cost runaway.
- Cognitive Signals provide human override (innermost safety net).

---

## Academic References

| Paper | Contribution |
|---|---|
| Saltzer & Schroeder (1975), "The Protection of Information in Computer Systems" | Complete mediation (Engram Syscalls) |
| Dennis & Van Horn (1966), "Programming Semantics for Multiprogrammed Computations" | Capability-based access control |
| Sha, Rajkumar & Lehoczky (1990), "Priority Inheritance Protocols" | Priority inversion prevention |
| Arpaci-Dusseau & Arpaci-Dusseau (2018), "Operating Systems: Three Easy Pieces" | OS primitives |
| Sumers et al. (2023, arXiv:2309.02427), "Cognitive Architectures for Language Agents" | CoALA cognitive loop |

---

## Implementation References

| Component | Location |
|---|---|
| SafetyLayer | `crates/roko-agent/src/safety/mod.rs` |
| ToolDispatcher | `crates/roko-agent/src/dispatcher/mod.rs` |
| ProcessSupervisor | `crates/roko-runtime/src/process.rs` |
| Event bus | `crates/roko-runtime/` |
| RateLimiter | `crates/roko-agent/src/safety/rate_limit.rs` |
| PathPolicy | `crates/roko-agent/src/safety/path.rs` |
| ScrubPolicy | `crates/roko-agent/src/safety/scrub.rs` |
| SandboxPolicy | `crates/roko-agent/src/safety/sandbox.rs` |
