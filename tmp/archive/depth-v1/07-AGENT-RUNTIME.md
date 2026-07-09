# 07 — Agent Runtime

> Agent = Space + Extensions + Memory + adaptive clock. The 9-step pipeline IS a Graph.

**Subsumes**: AgentRuntime, TickPipeline, CorticalState, AdaptiveClock, T0/T1/T2 gating, DomainProfile, AgentMode.

**Source**: Refactored from `tmp/architecture/02-agent-runtime.md` as Graph-based.

---

## 1. Overview

An **Agent** is the most complex specialization (see [doc-04 §10](04-SPECIALIZATIONS.md)): a Space + Extensions + Memory + adaptive clock. Every agent — in-process or remote — runs the same core loop. The agent's 9-step pipeline is itself a Graph, interpreted by the same execution engine that runs all other Graphs.

### Core framing

```
Agent = Space + Extensions + Memory + adaptive clock
```

| Component | What | Where |
|---|---|---|
| **Space** | Isolation boundary + capability grants | Defines what the agent can access |
| **Extensions** | Interceptor Blocks across 8 layers | Modify agent behavior through hooks |
| **Memory** | Store-protocol Block with decay + dreams | Durable knowledge with HDC retrieval |
| **Adaptive clock** | Tick frequency control across 3 timescales | Regulates perception/planning/consolidation |

The agent does not contain special runtime machinery. It composes fundamentals: its pipeline is a Graph, its knowledge is a Memory (Store Block), its hooks are Extensions (Blocks), and its isolation is a Space.

---

## 2. The Agent Struct

```rust
pub struct Agent {
    // ── Identity ──────────────────────────────────────────
    pub id: AgentId,
    pub name: String,
    pub profile: DomainProfile,         // user-defined string (e.g., "coding", "research")
    pub mode: AgentMode,                // Ephemeral | Persistent | Reactive

    // ── Composition ───────────────────────────────────────
    pub space: Space,                   // isolation boundary + capability grants
    pub extensions: Vec<Extension>,     // interceptor chain (8 layers)
    pub memory: Memory,                 // knowledge store with decay + HDC

    // ── Runtime ───────────────────────────────────────────
    pub clock: AdaptiveClock,           // tick frequency control
    pub pipeline: NineStepGraph,        // the 9-step pipeline as a Graph
    pub cortical: CorticalState,        // working memory, goals, beliefs, attention

    // ── Communication ─────────────────────────────────────
    pub inbox: mpsc::Receiver<AgentMessage>,
    pub bus: BusHandle,                 // publish/subscribe ephemeral Signals

    // ── Lifecycle ─────────────────────────────────────────
    pub cancel: CancellationToken,
}
```

### Mapping to existing code

The `Agent` struct maps to `roko-agent::AgentRuntime`:

| Agent field | AgentRuntime field | Notes |
|---|---|---|
| `id` | `id` | Same `AgentId` |
| `name` | `name` | Same |
| `profile` | `profile` | Same `DomainProfile` |
| `mode` | `mode` | Same `AgentMode` |
| `extensions` | `extensions` | Same `Vec<Box<dyn Extension>>` |
| `clock` | `clock` | Same `AdaptiveClock` |
| `pipeline` | `pipeline` | Same `TickPipeline`, now a Graph |
| `cortical` | `cortical` | Same `CorticalState` |
| `inbox` | `inbox` | Same `mpsc::Receiver` |
| `cancel` | `cancel` | Same `CancellationToken` |

New: `space` (was implicit workspace), `memory` (was implicit neuro store), `bus` (was implicit relay).

---

## 3. The run() Loop

```rust
impl Agent {
    pub async fn run(mut self) -> AgentResult {
        // Announce presence on the Bus
        self.bus.publish("agent:presence", Signal::presence(
            &self.id, PresenceEvent::Join, &self.profile
        )).await;

        loop {
            tokio::select! {
                // Graceful shutdown
                _ = self.cancel.cancelled() => break,

                // Clock tick → execute pipeline
                _ = self.clock.tick() => {
                    let result = self.pipeline.execute_tick(
                        &mut self.cortical,
                        &self.extensions,
                        &self.memory,
                        &self.space,
                    ).await;

                    // Publish heartbeat
                    self.bus.publish(
                        &format!("agent:{}:heartbeat", self.id),
                        Signal::heartbeat(&self.id, &result),
                    ).await;

                    // Check stop condition (Ephemeral mode)
                    if result.should_stop() {
                        break;
                    }
                }

                // Inbound message → handle
                msg = self.inbox.recv() => {
                    if let Some(msg) = msg {
                        self.handle_message(msg).await;
                    }
                }
            }
        }

        // Announce departure
        self.bus.publish("agent:presence", Signal::presence(
            &self.id, PresenceEvent::Leave, &self.profile
        )).await;

        // Persist final cortical state
        self.cortical.persist(&self.id).await;

        self.cortical.into_result()
    }
}
```

The `run()` loop is the same for all three modes. The mode affects when `should_stop()` returns true:

- **Ephemeral**: stops when the task completes (all goals resolved)
- **Persistent**: never stops (runs until cancelled)
- **Reactive**: sleeps between triggers (zero CPU), wakes on trigger fire

---

## 4. The 9-Step Pipeline

The agent's internal pipeline is a Graph with 9 nodes. Each tick executes these steps in order. Extensions can intercept at each step.

```
Step  Name       What Happens                                      Extension Layer
────  ────       ─────────────                                     ───────────────
 1    Observe    Read inbox, check triggers, scan environment      L1 (Perception)
 2    Retrieve   Query Memory, load relevant context               L2 (Memory)
 3    Analyze    Score observations, compute prediction error      L3 (Cognition)
 4    Gate       T0/T1/T2 decision (PE threshold)                  L3 (Cognition)
 5    Simulate   Generate candidate actions, evaluate outcomes     L3 (Cognition)
 6    Validate   Safety checks, capability verification, budget    L4 (Action)
 7    Execute    Dispatch action (LLM call, tool use, message)     L4 (Action)
 8    Verify     Check result against predictions                  L3 (Cognition)
 9    Reflect    Update cortical state, log episode, adjust clock  L6 (Meta)
```

### Pipeline as Graph

The 9-step pipeline is defined as a Graph with conditional edges based on the T0/T1/T2 gating decision:

```
        ┌─────────┐
        │ Observe  │ ──── Step 1
        └────┬─────┘
             │
        ┌────▼─────┐
        │ Retrieve  │ ──── Step 2
        └────┬─────┘
             │
        ┌────▼─────┐
        │ Analyze   │ ──── Step 3 (computes PE)
        └────┬─────┘
             │
        ┌────▼─────┐
        │   Gate    │ ──── Step 4 (T0/T1/T2 decision)
        └──┬──┬──┬─┘
           │  │  │
     T0 ───┘  │  └─── T2
              T1
           │  │  │
     ┌─────┘  │  └─────┐
     │        │         │
     │   ┌────▼─────┐   │
     │   │ Simulate  │   │ ──── Step 5 (T1/T2 only)
     │   └────┬─────┘   │
     │        │         │
     │   ┌────▼─────┐   │
     │   │ Validate  │   │ ──── Step 6 (T1/T2 only)
     │   └────┬─────┘   │
     │        │         │
     └───┐    │    ┌────┘
         │    │    │
        ┌▼────▼────▼┐
        │  Execute   │ ──── Step 7 (all tiers, different depth)
        └────┬──────┘
             │
        ┌────▼─────┐
        │  Verify   │ ──── Step 8
        └────┬─────┘
             │
        ┌────▼─────┐
        │  Reflect  │ ──── Step 9
        └──────────┘
```

T0 skips steps 5-6 (Simulate, Validate) and goes directly to Execute with a cached reflex action. T1 runs the full pipeline with a lightweight model. T2 runs the full pipeline with the most capable model.

---

## 5. Three Modes

```rust
pub enum AgentMode {
    /// Runs until task completes, then stops.
    Ephemeral,
    /// Runs continuously until manually stopped.
    Persistent,
    /// Sleeps until a trigger fires, wakes, works, sleeps again.
    Reactive,
}
```

### 5.1 Ephemeral

The default for task-oriented work. The agent receives a task, executes it through the pipeline, and shuts down when done.

- **Stop condition**: All goals in `cortical.goals` are resolved (completed or abandoned)
- **Timeout**: 30 minutes of no goal completion triggers warning and stop (configurable via `agent.ephemeral_timeout_secs`, default 1800)
- **Use cases**: Coding tasks, one-off research, PR review, plan execution

### 5.2 Persistent

The agent runs its tick loop indefinitely. It processes messages from its inbox, monitors its environment, and maintains long-running state.

- **Stop condition**: External cancellation only (`roko agent stop --name X`)
- **Use cases**: Chain monitoring, continuous integration watchers, team coordinators

### 5.3 Reactive

The agent registers Triggers and sleeps. When a Trigger fires, the runtime wakes the agent, it processes the event through the full pipeline, then sleeps again. Zero compute cost while sleeping.

- **Stop condition**: External cancellation
- **Wake latency**: Webhook Trigger wakes within 100ms. Cron Trigger fires on schedule.
- **Status display**: `roko agent status --name X` shows `sleeping` between triggers
- **Use cases**: PR reviewer, scheduled jobs, event-driven automation

```toml
# roko.toml — reactive agent example
[[agents]]
name = "pr-reviewer"
profile = "coding"
mode = "reactive"
triggers = [
    { type = "webhook", path = "/hooks/github-pr" },
    { type = "cron", schedule = "0 9 * * MON" },   # Monday morning sweep
]
```

---

## 6. Three Timescales

The adaptive clock operates at three frequencies, inspired by neural oscillation bands:

| Timescale | Name | Frequency Range | Purpose |
|---|---|---|---|
| **Gamma** | Fast perception | 100ms – 2s | Reflex responses, environment scanning, heartbeat |
| **Theta** | Reflective planning | 750ms – 16s | Reasoning, strategy adjustment, context retrieval |
| **Delta** | Deep consolidation | 60s – 10m | Memory consolidation, model updates, knowledge distillation |

### Gamma ticks

Every gamma tick executes the 9-step pipeline. This is the agent's heartbeat — the fastest it can perceive and react. At minimum (Crisis regime), gamma ticks fire every 125ms. At maximum (Calm regime), every 2000ms.

### Theta ticks

Every N gamma ticks, the agent performs a theta-level operation:
- Persist cortical state to disk
- Run deeper memory retrieval (cross-domain HDC search)
- Evaluate strategic goals and adjust priorities
- Update the cascade router with recent episode data

### Delta ticks

Triggered by inactivity or episode accumulation (not periodic):
- **Idle trigger**: 60s of no observation activity (no new messages, no tool results)
- **Episode trigger**: 20 episodes accumulated since last delta tick

Delta operations:
- Memory consolidation (dream cycle: NREM replay → REM imagination → Integration)
- Reflex store pruning and promotion
- Knowledge tier progression evaluation
- Long-horizon trend analysis

---

## 7. T0/T1/T2 Gating

Each tick, the Gate step (step 4) decides how much reasoning to apply based on prediction error (PE), budget, and urgency.

```rust
pub fn decide_tier(pe: f64, budget_remaining: f64, urgency: f64) -> Tier {
    if budget_remaining <= 0.0 {
        return Tier::Sleepwalk;
    }
    if pe < 0.15 && urgency < 0.3 {
        return Tier::T0;
    }
    if pe < 0.40 && urgency < 0.7 {
        return Tier::T1;
    }
    Tier::T2
}
```

| Tier | Condition | Cost | Action |
|---|---|---|---|
| **T0** (reflex) | PE < 0.15, no urgency | ~0 tokens | Execute cached reflex rule. Skip steps 5-6. |
| **T1** (reflective) | PE 0.15–0.40, moderate urgency | ~500 tokens | Full pipeline with lightweight model (Haiku). |
| **T2** (deliberate) | PE > 0.40, high urgency, or novel situation | ~2000–8000 tokens | Full pipeline with capable model (Sonnet/Opus). |
| **Sleepwalk** | Budget exhausted or externally throttled | 0 tokens | Steps 1, 9 only (Observe + Reflect). |

### Key properties

- **No hysteresis** on tier decisions — evaluated fresh each tick (hysteresis is on clock regime only)
- **PE is the primary input** — prediction error measures how much the environment surprised the agent
- **Budget is a hard constraint** — zero budget forces Sleepwalk regardless of PE
- **Urgency modulates thresholds** — an urgent message can push a low-PE tick to T1 or T2

---

## 8. T0 Reflex Execution

T0 skips inference entirely. Instead, the Execute step runs a rule engine over a local reflex store.

### Reflex store

Location: `.roko/learn/reflexes.jsonl`. Each line is a condition-action pair learned from previous T2 sessions:

```json
{
  "condition": {
    "tool": "bash",
    "args_pattern": "cargo test.*",
    "context": "gate_check"
  },
  "action": {
    "tool": "bash",
    "args": "cargo test --workspace"
  },
  "confidence": 0.97,
  "source_episode": "ep_a1b2c3",
  "promoted_at": "2026-04-20T14:30:00Z"
}
```

### Execution flow

```
Observation arrives
       │
       ▼
Match against reflexes.jsonl (linear scan, conditions checked in order)
       │
  match found ────────► Execute action directly (no LLM)
       │                       │
  no match                     ▼
       │               Record outcome, update confidence
       ▼
  Escalate to T1
```

### Promotion criteria

A T2 decision becomes a T0 reflex when:

1. The same observation pattern triggers the same action **3+ times**
2. Every execution passed its gate (**zero failures**)
3. Confidence > **0.90** (computed as `success_count / total_count`)

### Demotion criteria

If a reflex action fails a gate:
- Confidence is **halved**
- Below **0.50** → rule is deleted, future matches escalate to T1

### Store limits

- **Max 200 rules** — evict lowest confidence when full
- **Persists across restarts** — `.roko/learn/reflexes.jsonl` is append-only with periodic compaction

---

## 9. Adaptive Clock Algorithm

The clock adjusts tick frequency based on the agent's operating regime.

### Gamma interval

```
gamma_interval = base_interval * regime_factor

base_interval = 500ms (configurable via agent.clock_base_ms in roko.toml)
```

| Regime | Factor | Gamma interval (at 500ms base) |
|---|---|---|
| Calm | 4.0x | 2000ms |
| Normal | 1.0x | 500ms |
| Volatile | 0.5x | 250ms |
| Crisis | 0.25x | 125ms |

### Theta interval

```
theta_interval = N * gamma_interval
```

| Regime | N | Theta interval (at 500ms base) |
|---|---|---|
| Calm | 8 | 16000ms (16s) |
| Normal | 5 | 2500ms (2.5s) |
| Volatile | 3 | 750ms |
| Crisis | 2 | 250ms |

### Delta interval

Not periodic. Triggers on whichever comes first:
- `idle_timeout`: 60s of no observation activity
- `episode_threshold`: 20 episodes accumulated since last delta tick

### Regime detection with 3-tick hysteresis

Regimes transition based on prediction error (PE) and error rate, with a 3-tick hysteresis window to prevent oscillation:

```
                   ┌──────────────────────────────────────────┐
                   │                                          │
                   ▼                                          │
              ┌─────────┐   PE > 0.40 for 3 ticks       ┌────┴────┐
     ┌───────►│  Calm    │──────────────────────────────►│ Normal   │
     │        └─────────┘                                └────┬────┘
     │             ▲                                          │
     │   PE < 0.10 │ for 3 ticks               PE > 0.60     │ for 3 ticks
     │             │                            for 3 ticks   │
     │             │                                          ▼
     │        ┌────┴────┐                                ┌─────────┐
     │        │ Normal   │◄──────────────────────────────│ Volatile │
     │        └─────────┘   PE < 0.30 for 3 ticks       └────┬────┘
     │                                                        │
     │                                          error_rate    │ > 0.5
     │                                          for 3 ticks   │
     │                                                        ▼
     │                                                   ┌─────────┐
     └───────────────────────────────────────────────────│ Crisis   │
                  error_rate < 0.1 for 3 ticks           └─────────┘
```

### Hysteresis rules

- A regime must persist for **3 consecutive qualifying gamma ticks** before the clock adjusts
- During the hysteresis window, the clock uses the **previous regime's** intervals
- **Non-qualifying ticks reset the counter** — oscillating PE (e.g., 0.10, 0.20, 0.10) does NOT cause regime change
- This prevents a single anomalous tick from thrashing clock speeds

---

## 10. Cortical State Persistence

Cortical state is the agent's working memory: goals, beliefs, attention, and prediction error history. It is serialized to `.roko/agents/{id}/cortical.json` on every **theta tick** (not gamma).

```json
{
  "agent_id": "coder-1",
  "snapshot_at": "2026-04-24T14:32:10Z",
  "working_memory": [
    { "item": "implement auth middleware", "salience": 0.82, "added_at": "..." },
    { "item": "tests passing on main", "salience": 0.45, "added_at": "..." }
  ],
  "goals": [
    { "description": "complete PR #42", "status": "active", "progress": 0.6 }
  ],
  "beliefs": {
    "codebase_stable": 0.85,
    "tests_passing": 0.92,
    "deadline_pressure": 0.30
  },
  "attention": {
    "focus": "implement auth middleware",
    "salience": 0.82
  },
  "regime": "normal",
  "prediction_error_ema": 0.27,
  "episode_count": 142
}
```

### Restart behavior

On agent startup, the runtime checks for existing cortical state:

| Condition | Action |
|---|---|
| Snapshot exists, < 1 hour old | Load and resume from saved state |
| Snapshot exists, >= 1 hour old | Discard — stale beliefs hurt more than cold start |
| No snapshot file | Start fresh (`CorticalState::default()`) |

The 1-hour staleness threshold exists because goals, beliefs, and attention weights drift out of alignment with the actual environment. A 2-hour-old belief that "tests are passing" may be actively wrong if someone pushed breaking changes.

### Working memory limits

Working memory is capped at **50 items** with LRU eviction. Items with higher salience survive longer, but even high-salience items are eventually evicted if working memory is full and newer items arrive.

---

## 11. Domain Profiles

Domain profiles are **user-defined strings**, not enums. A profile is a label that maps to a default set of Extensions and tools. Roko ships built-in profiles as a convenience, but users create their own.

```rust
/// A domain profile is a user-defined string, not an enum.
pub struct DomainProfile(pub String);
```

### Built-in profiles

| Profile | Default Extensions | Default Tools |
|---|---|---|
| `coding` | git, compiler, test-runner, lsp | bash, file_edit, git, grep |
| `research` | web-search, citation, summarizer | web_search, pdf_read, cite |
| `chain` | chain-reader, tx-builder, feed-publisher | eth_call, send_tx, subscribe_events |

### Custom profiles

Any string is a valid profile. Profiles with no built-in defaults start with an empty Extension chain — the user specifies everything explicitly:

```toml
[[agents]]
name = "security-auditor"
profile = "security"          # user-defined, not in any enum
mode = "reactive"
extensions = ["code-scanner", "vuln-db", "report-writer"]
tools = ["grep", "ast_query", "file_read", "web_search"]
triggers = [{ type = "webhook", path = "/hooks/github-pr" }]
```

### Shareable profiles

Users can publish profiles as TOML configs:

```toml
# ~/.roko/profiles/defi-trader.toml
[profile]
name = "defi-trader"
description = "DeFi trading agent with risk management"
extensions = ["chain-reader", "tx-builder", "risk-engine", "pnl-tracker"]
tools = ["eth_call", "send_tx", "subscribe_events", "query_pool", "swap"]
default_mode = "persistent"
default_budget = { daily_limit_usd = 50.0 }
```

Then reference it:

```toml
[[agents]]
name = "my-trader"
profile = "defi-trader"      # loads from ~/.roko/profiles/defi-trader.toml
mode = "persistent"
```

---

## 12. Extension Integration

Extensions are Blocks that intercept the agent's pipeline. They fire in layer order (L0 → L7), and within a layer, in config order. See [doc-08 (Extension System)](08-EXTENSION-SYSTEM.md) for the full specification.

### How Extensions hook into the pipeline

| Pipeline Step | Extension Layer | Hooks |
|---|---|---|
| 1. Observe | L1 (Perception) | `on_observe`, `filter_input` |
| 2. Retrieve | L2 (Memory) | `on_retrieve`, `on_store` |
| 3. Analyze | L3 (Cognition) | `pre_inference` |
| 4. Gate | L3 (Cognition) | `on_gate` |
| 5. Simulate | L3 (Cognition) | `post_inference` |
| 6. Validate | L4 (Action) | `pre_action` |
| 7. Execute | L4 (Action) | `post_action`, `on_tool_call` |
| 8. Verify | L3 (Cognition) | (none — verification is internal) |
| 9. Reflect | L6 (Meta) | `on_reflect`, `on_cost_update` |

Additional hooks not tied to specific steps:
- **L0 (Foundation)**: `on_init`, `on_shutdown` — lifecycle
- **L5 (Social)**: `on_message_send`, `on_message_receive` — communication
- **L7 (Recovery)**: `on_error`, `on_budget_exceeded` — error handling

### Fault isolation

If one Extension hook errors, the runtime logs the error and continues to the next Extension. An optional Extension that crashes cannot take down the agent. Required Extensions (marked `optional = false`) cause the agent to stop if they fail to load.

---

## 13. Memory Integration

The agent's Memory is a Store-protocol Block (see [doc-04 §7](04-SPECIALIZATIONS.md)) with:

- **HDC-based retrieval** — 10,240-bit binary vectors for similarity search
- **Decay** — Ebbinghaus forgetting curve with tier multipliers
- **Tier progression** — Transient → Working → Consolidated → Persistent
- **Anti-knowledge** — known-bad information repels similar entries
- **Dream consolidation** — offline NREM/REM/Integration cycle on delta ticks

### Retrieval scoring

When the agent queries Memory (step 2, Retrieve):

```
final_score = hdc_similarity × 0.40
            + keyword_relevance × 0.30
            + utility × 0.20
            + freshness × 0.10
            + (cross_domain ? 0.15 : 0.0)
```

Cross-domain matches get a 15% bonus — a retry pattern from networking might transfer to database operations.

### Memory writes

The agent writes to Memory at two points:
- **Step 7 (Execute)**: Tool results and LLM outputs persisted as Signals
- **Step 9 (Reflect)**: Insights, heuristics, and episode Signals written

New Signals enter at Transient tier with kind-appropriate decay (see [doc-01 §3](01-SIGNAL.md)).

---

## 14. Agent Configuration

Complete TOML schema for agent definition:

```toml
[[agents]]
name = "coder-1"
profile = "coding"
mode = "ephemeral"

# ── Clock ────────────────────────────────────────────
clock_base_ms = 500                    # base gamma interval
ephemeral_timeout_secs = 1800          # timeout for ephemeral mode

# ── Extensions ───────────────────────────────────────
extensions = ["git", "compiler", "test-runner"]

# ── Tools ────────────────────────────────────────────
tools = ["bash", "file_edit", "git", "grep"]

# ── MCP ──────────────────────────────────────────────
mcp_config = ".mcp.json"              # MCP server config passthrough

# ── Models ───────────────────────────────────────────
[agents.models]
t1 = "claude-haiku-4-5"              # lightweight model for T1
t2 = "claude-sonnet-4-6"             # capable model for T2
force_backend = ""                     # override all routing (empty = use router)

# ── Budget ───────────────────────────────────────────
[agents.budget]
max_usd = 10.0                        # per-task budget
daily_limit_usd = 100.0               # rolling 24h cap
warn_at_pct = 80

# ── Triggers (reactive mode) ────────────────────────
[[agents.triggers]]
type = "webhook"
path = "/hooks/github-pr"

[[agents.triggers]]
type = "cron"
schedule = "0 9 * * MON"
```

---

## 15. Acceptance Criteria

### AgentMode lifecycle

| # | Criterion | Verification |
|---|---|---|
| 1 | Ephemeral agent stops after full task-gate-persist cycle completes (not on first response) | Integration test: task with 3 steps, agent runs all 3 |
| 2 | Ephemeral timeout: 30 minutes of no completion triggers warning and stop | Test with configurable `ephemeral_timeout_secs` |
| 3 | Persistent agent runs tick loop indefinitely until manually stopped | Test: agent runs >100 ticks, only stops on cancel |
| 4 | Reactive agent sleeps between triggers (zero CPU) | Test: webhook Trigger wakes within 100ms |
| 5 | `roko agent status --name x` shows `sleeping` for reactive agents between triggers | CLI output check |

### T0/T1/T2 gating

| # | Criterion | Verification |
|---|---|---|
| 6 | `decide_tier(0.10, 1000.0, 0.1)` returns T0 | Unit test |
| 7 | `decide_tier(0.25, 1000.0, 0.5)` returns T1 | Unit test |
| 8 | `decide_tier(0.50, 1000.0, 0.8)` returns T2 | Unit test |
| 9 | `decide_tier(0.50, 0.0, 0.8)` returns Sleepwalk | Unit test |
| 10 | No hysteresis on tier decisions — evaluated fresh each tick | Test: alternate PE, verify tier changes immediately |

### T0 reflex store

| # | Criterion | Verification |
|---|---|---|
| 11 | Reflex rule created after 3 identical T2 successes with zero gate failures | Integration test |
| 12 | T0 path matches rule and executes action without LLM call | Verify zero token usage on T0 tick |
| 13 | Gate failure halves reflex confidence | Test: fail gate, check confidence |
| 14 | Rule deleted when confidence < 0.50 | Test: two failures, verify deletion |
| 15 | Max 200 rules, evict lowest confidence when full | Test: insert 201, verify eviction |
| 16 | `.roko/learn/reflexes.jsonl` persists across restarts | Restart test |

### Adaptive clock

| # | Criterion | Verification |
|---|---|---|
| 17 | Regime changes only after 3 consecutive qualifying ticks | Test: 2 qualifying + 1 non-qualifying resets counter |
| 18 | Oscillating PE does NOT cause regime change | Test: PE oscillates, verify regime stays |
| 19 | Gamma interval = base * regime_factor | Test all four regimes |
| 20 | Delta tick fires on 60s idle | Test: idle agent, verify delta fires |
| 21 | Delta tick fires on 20 episodes accumulated | Test: force 20 episodes, verify delta fires |
| 22 | `clock_base_ms` configurable via roko.toml | Config test |

### Cortical state persistence

| # | Criterion | Verification |
|---|---|---|
| 23 | Serialized to `.roko/agents/{id}/cortical.json` on every theta tick | File existence check after theta |
| 24 | Snapshot < 1 hour old loaded on restart | Restart test with recent snapshot |
| 25 | Snapshot >= 1 hour old discarded | Restart test with old snapshot |
| 26 | Working memory capped at 50 items (LRU eviction) | Test: insert 60 items, verify 50 remain |

### Domain profiles

| # | Criterion | Verification |
|---|---|---|
| 27 | Built-in profile loads default Extensions | Test: `profile = "coding"` loads git, compiler, test-runner |
| 28 | Custom profile string accepted without error | Test: `profile = "my-custom"` starts successfully |
| 29 | Shareable profile loaded from `~/.roko/profiles/` | Test: create profile TOML, reference in agent config |

---

## 16. Open Questions

- **Multi-agent coordination**: How do agents in a Space share beliefs and coordinate goals? Currently each agent has independent cortical state. Fleet-level coordination is deferred.
- **Cortical state migration**: When agent config changes (new Extensions, different profile), should cortical state be migrated or reset? Currently reset. May want selective migration.
- **T0 reflex sharing**: Should reflex stores be shared across agents with the same profile? Currently per-agent. Sharing would accelerate learning but risks cross-contamination.
- **Dream scheduling**: Delta ticks trigger dream consolidation, but there is no cron-like scheduler for deep offline dreams. A dedicated `roko knowledge dream schedule` exists but is not yet wired to the agent runtime.
