# Gap Analysis: What's Lost and What Needs Rebuilding

## Critical Gaps (Ordered by Impact)

### Gap 1: Agent Runtime (Agent as Process, not Function Call)

**Original:** Agents are long-lived processes with heartbeat loops, event subscriptions,
and full cognitive subsystem access.

**Current:** Agents are `spawn_process() → wait_for_output() → kill()` — a function call
to an LLM with extra steps.

**Impact:** Without this, nothing else works. Blockchain agents can't subscribe to blocks.
Research agents can't accumulate knowledge across sessions. Coding agents can't be
cost-efficient (no T0/T1 gating).

**What to build:**
```rust
pub trait AgentRuntime: Send + Sync {
    /// The heartbeat — called on every tick (gamma/theta/delta)
    async fn tick(&mut self, frequency: Frequency) -> TickOutcome;

    /// Event subscription — agent receives events from the fabric
    async fn on_event(&mut self, event: RuntimeEvent) -> EventResponse;

    /// Operator message — persistent chat input
    async fn on_message(&mut self, msg: OperatorMessage) -> Response;

    /// Lifecycle management
    fn state(&self) -> AgentState; // Active, Dreaming, Terminal
    async fn dream(&mut self) -> DreamOutcome;
    async fn shutdown(&mut self) -> GenomeExtract;
}
```

### Gap 2: Extension System (Composable Agent Behavior)

**Original:** 28 extensions across 7 layers, implementing 20 lifecycle hooks.
Extensions compose: Heartbeat reads from Daimon's affect state, Memory reads from
Heartbeat's probes, Dreams read from both.

**Current:** Everything is hardcoded in `orchestrate.rs` (19K lines). No composition,
no hooks, no pluggable behavior. Adding a new cognitive capability means editing
a monolith.

**Impact:** Users can't extend agent behavior. Can't add new subsystems. Can't
compose behaviors declaratively.

**What to build:**
```rust
pub trait Extension: Send + Sync {
    fn name(&self) -> &str;
    fn layer(&self) -> u8; // 0-7, determines init order
    fn hooks(&self) -> HookMask; // which hooks this extension implements

    // 20 hooks (see 02-golem-vision.md)
    async fn on_tick(&mut self, ctx: &mut TickContext) -> Result<()> { Ok(()) }
    async fn assemble_context(&mut self, ws: &mut CognitiveWorkspace) -> Result<()> { Ok(()) }
    async fn before_tool_call(&mut self, call: &mut ToolCall) -> Result<ToolDecision> { Ok(ToolDecision::Allow) }
    // ...
}
```

### Gap 3: Cognitive Gating (T0/T1/T2 Decision)

**Original:** 80% of ticks are T0 (no LLM, $0). Agent uses pattern matching,
habituation, and prediction error to decide whether to escalate.

**Current:** Every agent dispatch calls the full LLM. 100% T2. Maximally expensive.

**Impact:** Running agents continuously would bankrupt the budget instantly.
The whole economic viability of autonomous agents depends on gating.

**What to build:**
```rust
pub enum CognitiveTier {
    T0, // Deterministic: pattern match against known situations
    T1, // Lightweight: cheap model (Haiku) for simple decisions
    T2, // Full reasoning: expensive model (Opus) for novel situations
}

pub trait CognitiveGate: Send + Sync {
    /// Given current state + stimulus, decide what tier of cognition to use
    fn gate(&self, state: &CorticalState, stimulus: &Stimulus) -> CognitiveTier;
}
```

### Gap 4: Event Fabric (Agent-Side Subscriptions)

**Original:** tokio::broadcast ring buffer (10K events). Agents subscribe to
event categories. Chain events wake sleeping agents.

**Current:** Event bus exists but only dashboards consume. Agents don't subscribe.

**Impact:** No event-driven behavior. Blockchain agents can't react to blocks.
No inter-agent communication via events.

**What to build:**
- Agent receives `EventReceiver` at spawn time
- Filter by event category (chain, file, pheromone, heartbeat)
- `tokio::select!` in agent loop: next tick OR next relevant event

### Gap 5: Context Engineering as Control System

**Original:** CognitiveWorkspace with typed sections, priority-based assembly,
learnable allocation, 4-tier caching, U-shaped placement.

**Current:** System prompt builder exists (9 layers) but:
- No learnable allocation (static budgets)
- No workspace delta tracking
- No feedback loop from outcomes to context decisions
- No complexity-based dropping

**Impact:** Agents get too much irrelevant context (wasted tokens/cost) or
miss critical context (bad decisions).

### Gap 6: Agent Persistence & State

**Original:** Agents have continuous state across ticks. Grimoire persists.
Type-state lifecycle prevents invalid transitions at compile time.

**Current:** Agents are stateless. Each dispatch starts fresh. No memory
across invocations beyond what the orchestrator tracks.

**Impact:** Agents can't learn within a session. Can't maintain working memory.
Can't build up context over time.

### Gap 7: Process Model (Actor + Supervision)

**Original:** Actor model with mailboxes, supervision trees (one-for-one restart),
channel mobility, fault isolation.

**Current:** Flat process spawning. No supervision. No fault isolation.
No restart strategies.

**Impact:** Agent crashes take down the whole plan. No graceful degradation.
No resilience.

### Gap 8: A2A / Inter-Agent Communication

**Original:** JSON-RPC 2.0 Agent-to-Agent protocol. Pheromone field for
implicit coordination. Delegation DAG (max depth 3).

**Current:** No inter-agent communication at all. Agents are isolated.
Coordination happens only via shared files (stigmergic, accidental).

## What CAN Be Preserved

| Subsystem | Current State | Reusable? |
|---|---|---|
| Core traits (6 verbs) | Fully generic | YES — foundation stays |
| Signal (Engram) | Domain-agnostic | YES — universal data type |
| Daimon (affect) | Working, orchestrator-side | YES — move into extension |
| Neuro (knowledge) | Working, partial | YES — expose to agents |
| Dreams (consolidation) | Working, basic cycle | YES — trigger from sleep pressure |
| Conductor (10 watchers) | Fully wired | YES — becomes an extension |
| Gate pipeline | Working | YES — domain-aware already |
| Learning (episodes) | Working | YES — domain-agnostic |
| CascadeRouter | Working | YES — model routing stays |
| Tool registry | Working | YES — tools are generic |
| MCP integration | Working | YES — extensible |

## What Needs Rewriting

| Component | Why |
|---|---|
| Agent dispatch | Replace spawn-die with persistent runtime |
| orchestrate.rs monolith | Break into extensions + runtime + lifecycle |
| Event bus consumption | Add agent-side subscription |
| Context assembly | Replace static builder with learnable workspace |
| Heartbeat/frequency | Make it the actual tick driver, not metadata |
| Process management | Add supervision trees, fault isolation |

## Incremental Path

**Phase 1:** Extract AgentRuntime trait + extension system from orchestrate.rs.
Keep existing task-based flow working but now agents are Runtime instances
that handle tasks via `on_message()`.

**Phase 2:** Add heartbeat loop. Agents tick on gamma/theta/delta.
Existing task dispatch becomes "inject task as a theta-level stimulus."
T0 gating means idle agents are nearly free.

**Phase 3:** Wire event fabric to agents. Chain subscriptions, file watchers,
pheromones all become event sources that agents receive.

**Phase 4:** Implement CognitiveWorkspace with learnable allocation.
Track workspace deltas, feed outcomes back to allocation policy.

**Phase 5:** Type-state lifecycle. Compile-time enforcement of valid
state transitions. Genome extraction on death.
