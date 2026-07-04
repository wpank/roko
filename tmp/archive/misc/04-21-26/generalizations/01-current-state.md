# Current State vs. Original Vision

## The Core Problem

The original Golem architecture describes **long-lived autonomous agents** with:
- A 9-step decision cycle (OBSERVE → RETRIEVE → ANALYZE → GATE → SIMULATE → VALIDATE → EXECUTE → VERIFY → REFLECT)
- Three timescales (gamma 5-15s, theta 30-120s, delta ~50 thetas)
- 20 lifecycle hooks via extensions
- Event-driven wakeup from chain events
- Type-state lifecycle (Provisioning → Active ↔ Dreaming → Terminal → Dead)
- Full cognitive subsystem access (daimon, neuro, dreams, somatic)
- Persistent chat interface for operator interaction

**What roko actually has:** Spawn-execute-die task agents. The orchestrator holds all cognitive state. Agents are dumb LLM wrappers.

## What Was Lost

### 1. The Heartbeat Loop (completely lost)
The Golem's core innovation was the **adaptive decision cycle** with cognitive gating:
- 80% of ticks are T0 (deterministic, no LLM) — pure Rust pattern matching
- 15% are T1 (cheap model) — lightweight decisions
- 5% are T2 (full reasoning) — complex strategy

This means agents are **cost-efficient by architecture**, not by prompt engineering. Currently roko always calls the full LLM (100% T2 equivalent).

### 2. The Extension System (completely lost)
28 extensions across 7 dependency layers, each implementing only the hooks it needs:
- Layer 0: Foundation (Heartbeat, Clock, CorticalState)
- Layer 1: Perception (EventFabric, Probes)
- Layer 2: Memory (Grimoire, Episodic, Semantic)
- Layer 3: Cognition (Daimon, WorkingMemory, Attention)
- Layer 4: Action (ToolDispatch, Safety, Execution)
- Layer 5: Social (Pheromones, A2A, Communication)
- Layer 6: Meta (Dreams, Consolidation, Evolution)
- Layer 7: Recovery (Compensation, Rollback, Death)

### 3. Event-Driven Agent Behavior (completely lost)
Agents were supposed to:
- Subscribe to chain events (new blocks, txs, price feeds)
- React to pheromone signals from other agents
- Wake from dreams on urgent events
- Have attention salience with exponential decay

### 4. Context Engineering as Learnable Control (mostly lost)
The original design had:
- CognitiveWorkspace: typed, budgeted context per inference call
- ContextPolicy: evolves through cybernetic feedback loops
- 4-tier caching for prefix alignment (90% cache hits)
- Complexity-based context dropping (4K for trivial → 40K for complex)
- U-shaped placement (most relevant first and last)

### 5. Type-State Lifecycle (completely lost)
Compile-time enforcement of valid agent states:
- Can't tick a dead agent (compiler error)
- Can't dream while active without transitioning
- Graceful 10-phase shutdown with genome extraction

### 6. Process Model (completely lost)
Actor model + process calculi:
- Encapsulated state (no shared mutable state)
- Mailbox semantics (messages buffered, processed sequentially)
- Supervision trees (one-for-one, rest-for-one, one-for-all restart)
- Channel mobility (agent receives event channel at spawn time)

## What Partially Exists

### Daimon
- Core ALMA affect model: **working**
- Somatic markers: **working** (queried at dispatch)
- Behavioral state: **working** (modulates routing)
- Phase 2 (contagion, mortality): **stubs only**
- Agent-side affect: **missing** (only orchestrator has affect)

### Neuro
- Knowledge store: **working**
- Strategy fragment injection: **working**
- Anti-knowledge surfacing: **working**
- Full CBR (case-based reasoning): **missing**
- Agent-side knowledge access: **missing**

### Dreams
- Core cycle (replay→imagine→rehearse→promote): **working**
- Runs at plan completion: **working**
- Hypnagogia/evolution: **stubs**
- Dream-driven behavior adaptation: **missing**
- Sleep pressure accumulation: **missing**

### Conductor
- 10 watchers: **working**
- Intervention policy: **working**
- Routing bias: **working**
- (This is the most complete subsystem)

### Event Bus
- Broadcast channel: **working**
- Dashboard consumption: **working**
- Agent subscription: **missing**
- Event-driven wakeup: **missing**

## Current Architecture (simplified)

```
Orchestrator (owns everything)
  ├── Plan DAG execution (sequential/parallel tasks)
  ├── Agent dispatch (spawn LLM → get result → kill)
  ├── Gate pipeline (compile → test → clippy)
  ├── Learning (episodes, routing, experiments)
  ├── Daimon (orchestrator-side only)
  ├── Neuro (queried at dispatch only)
  ├── Dreams (triggered at plan end only)
  └── Event bus (dashboards only)
```

## Target Architecture (from PRDs)

```
Agent Runtime
  ├── Heartbeat Loop (gamma/theta/delta)
  │   ├── OBSERVE: probes + event subscriptions
  │   ├── RETRIEVE: query neuro/grimoire
  │   ├── ANALYZE: prediction error
  │   ├── GATE: T0/T1/T2 decision
  │   └── (conditional) SIMULATE → VALIDATE → EXECUTE → VERIFY → REFLECT
  ├── Extensions (28, layered, trait-based)
  │   ├── Daimon (agent-side affect)
  │   ├── Memory (agent-side knowledge)
  │   ├── Dreams (agent-side consolidation)
  │   └── Safety (agent-side constraints)
  ├── Event Fabric (subscribe to chain/file/pheromone events)
  ├── CognitiveWorkspace (learnable context assembly)
  ├── Type-State Lifecycle (compile-time enforcement)
  └── Communication (A2A, pheromones, operator chat)
```
