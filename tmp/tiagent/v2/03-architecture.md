# Technical Architecture

This document is the complete technical reference for tiagent's architecture. It covers
the data model, the trait system, the execution loop, the layered runtime, crate
structure, Celestia integration, and model agnosticism. No prior context is assumed
beyond basic familiarity with Rust and agent systems.

---

## Table of Contents

1. [Design Philosophy](#1-design-philosophy)
2. [The Signal Model](#2-the-signal-model)
3. [The Universal Loop](#3-the-universal-loop)
4. [The Six Verb Traits](#4-the-six-verb-traits)
5. [Layered Architecture](#5-layered-architecture)
6. [Crate Structure](#6-crate-structure)
7. [Celestia Integration (Optional)](#7-celestia-integration-optional)
8. [Model Agnosticism](#8-model-agnosticism)

---

## 1. Design Philosophy

Four principles govern tiagent's design. Every architectural decision traces back to
one of these.

### 1 noun + 6 verbs = the entire kernel

Most agent frameworks organize around **nouns**: an Agent class, a Tool class, a Memory
class, a Planner class. This leads to a proliferation of types that must all be
coordinated. Adding a capability means touching multiple noun classes. Swapping behavior
means restructuring the data model.

tiagent inverts this. There is **one noun** --- `Signal` --- and behavior is defined by
**six verb traits**: Substrate, Scorer, Gate, Router, Composer, Policy.

Every piece of data that flows through the system is a Signal. A user prompt is a Signal.
An LLM response is a Signal. A tool call is a Signal. A test result is a Signal. A
learning artifact published to Celestia is a Signal.

Adding a new capability means implementing an existing trait, not inventing a new type.
Swapping behavior means providing a different trait implementation, not restructuring data.

### The universal loop

Every execution follows the same eight-stage pipeline:

```
query -> score -> route -> compose -> act -> verify -> persist -> react
```

This applies to a simple prompt/response, a multi-step tool-using task, a complex plan
execution, and even the self-improvement cycle itself. There is no separate "simple mode"
vs "advanced mode." The loop is the same; the configuration determines how much of it
is active.

### Works without Celestia, better with Celestia

The core harness compiles, runs, and is fully functional with zero Celestia dependencies.
Local filesystem storage, LLM dispatch, plan execution, quality gates, learning --- all of
it works standalone. Celestia integration is an optional feature flag (`--features celestia`)
that adds cross-agent learning via the DA layer.

A developer who never touches blockchain gets a best-in-class agent harness. A developer
who enables Celestia gets shared learning, verifiable provenance, and DA-backed persistence
on top of that same harness.

### Minimal crate count

**Target: ~14 crates**, including tool server binaries.

Two prior projects inform this constraint:

- **polkagent** (Polkadot agent framework) grew to 90 crates. Slow compilation, difficult
  dependency management, intimidating to contributors.
- **roko** (self-building agent toolkit) has 35 workspace members. Manageable, but some
  crates overlap in responsibility.

The rule: if you cannot explain a crate's purpose in one sentence, merge it into a
neighbor. If a crate has fewer than 500 lines of production code, it should be a module,
not a crate.

Dependencies flow in one direction: **kernel -> domain -> application -> binary**. Circular
dependencies are forbidden.

---

## 2. The Signal Model

### What is a Signal?

A Signal is the universal data atom. Every piece of information that enters, flows through,
or exits tiagent is represented as a Signal:

- A user prompt ("deploy my rollup to Mocha testnet")
- An LLM response (the model's generated text)
- A tool call ("submit this blob to namespace X")
- A tool result ("blob submitted, height 12345, hash 0xabc...")
- A gate result ("compilation passed", "3 tests failed")
- An episode summary (a structured record of a complete agent execution)
- A learning artifact (updated routing weights, a new playbook)
- A coordination message between agents

### The Signal struct

```rust
pub struct Signal {
    /// Content-addressed identity: SHA-256 of the serialized payload.
    /// Two Signals with identical payloads always have the same id.
    pub id: Hash,

    /// What type of data this Signal carries (Prompt, Response, ToolCall,
    /// GateResult, Episode, Playbook, RoutingUpdate, etc.)
    pub kind: SignalKind,

    /// The actual data. Serialized as JSON; structure depends on `kind`.
    pub payload: serde_json::Value,

    /// Key-value metadata: created_at, source, provenance, cost data, etc.
    pub metadata: HashMap<String, serde_json::Value>,

    /// Optional parent Signal hash. Creates a directed edge in the Signal DAG.
    pub parent: Option<Hash>,

    /// Optional Celestia namespace for DA submission.
    pub namespace: Option<Namespace>,
}
```

### Content addressing

Every Signal's `id` is a SHA-256 hash of its serialized payload. This gives three
properties for free:

1. **Deduplication** --- identical data produces identical hashes, automatically
   deduplicated across agents and substrates.
2. **Integrity** --- given a Signal's id, you can verify its payload has not been tampered
   with by recomputing the hash. Essential for Signals retrieved from the DA layer.
3. **Referencing** --- Signals reference each other by hash without embedding full content,
   creating a lightweight DAG.

### The Signal DAG

Signals form a **directed acyclic graph** through their `parent` fields. The DAG captures
the full causal history of every execution:

```
                +-----------------+
                |   Prompt        |   "fix the login bug"
                |   id: 0xa1b2..  |
                +--------+--------+
                         |
                +--------v--------+
                |   Response      |   LLM generates fix
                |   id: 0xc3d4..  |
                +---+----------+--+
                    |          |
           +--------v---+  +--v-----------+
           |  ToolCall   |  |  ToolCall     |
           |  "read      |  |  "edit        |
           |   auth.rs"  |  |   auth.rs"    |
           |  id: 0xe5.. |  |  id: 0xf6..   |
           +--------+---+  +--+-----------+
                    |          |
           +--------v---+  +--v-----------+
           | ToolResult  |  | ToolResult    |
           | "file       |  | "file         |
           |  contents"  |  |  updated"     |
           +-------------+  +--+-----------+
                               |
                       +-------v-------+
                       |  GateResult   |   tests pass, lint clean
                       |  id: 0x28..   |
                       +-------+-------+
                               |
                       +-------v-------+
                       |  Episode      |   full execution trace
                       |  id: 0x39..   |
                       +---------------+
```

The DAG enables:

- **Auditing**: walk backward from any result to see every step that produced it.
- **Debugging**: follow the parent chain from a gate failure to the tool call that caused it.
- **Learning**: Episode Signals reference their entire sub-DAG, enabling full trajectory
  analysis.
- **Provenance on DA**: when Signals are published to Celestia, the DAG structure is
  preserved. Any agent can verify the provenance chain of a published result.

### Immutability

Signals are **immutable** once created. You never update a Signal; you create a new one
that references the old one via `parent`. This is deliberate:

- Aligns with Celestia's append-only DA layer.
- Eliminates mutable shared state (no concurrency bugs in the data model).
- Enables content addressing (if Signals could mutate, their hashes would be invalid).
- Makes auditing trivial (the complete history is always preserved).

---

## 3. The Universal Loop

Every agent execution follows the same eight-stage pipeline. It applies to a single
prompt/response, a multi-step tool task, a full plan execution, and even the
self-improvement cycle.

### Overview

```
query --> score --> route --> compose --> act --> verify --> persist --> react
  ^                                                                      |
  |                                                                      |
  +----------------------------------------------------------------------+
                          (loop / recursion)
```

### The eight stages

**Stage 1: Query** --- An input Signal enters the system.

Sources: a user prompt in the CLI (`tiagent run "fix the login bug"`), an HTTP API call,
a scheduled event, or a "react" stage from a prior loop triggering a new query. The stage
produces a Signal with `kind: Prompt` or `kind: Event`.

**Stage 2: Score** --- Scorers evaluate the input Signal.

Multiple scorers run: complexity (how difficult is the task?), priority (how urgent?),
relevance (how related to this agent's domain?). A simple task like "rename a variable"
gets a low complexity score. A task like "redesign the authentication system" gets a high
one. These scores feed into routing.

**Stage 3: Route** --- The Router selects the execution path.

Using the scores, historical success rates, cost budget, and model availability, the
Router decides: which LLM model, which backend, which tool set, what token budget.
A low-complexity task gets a fast, cheap model. A high-complexity task gets a stronger
model. Over time, the CascadeRouter learns which models succeed for which task types.

**Stage 4: Compose** --- The Composer assembles the full prompt.

The 9-layer prompt builder stacks context from multiple sources, prioritized so that if
the context window is too small, lower-priority layers are trimmed first:

```
Layer 1 (highest): The current task description
Layer 2:           Safety constraints and budget limits
Layer 3:           Tool definitions and usage examples
Layer 4:           Conversation history
Layer 5:           Task-specific context (related files, dependencies)
Layer 6:           Project-specific context (repo structure, conventions)
Layer 7:           Relevant playbooks from prior successful executions
Layer 8:           Background knowledge (trajectory RAG from DA layer)
Layer 9 (lowest):  Agent personality / style guidelines
```

Critical information (the task itself, safety constraints, tools) is never trimmed.

**Stage 5: Act** --- The prompt is dispatched to the selected LLM backend.

The model generates a response, which may include tool calls. If tool calls are present,
tools are executed (each passing through a Policy check first) and results are fed back
to the model. This tool loop continues until the model produces a final response with
no further tool calls.

```
    +------------------+
    |  Send prompt to  |
    |  LLM backend     |
    +--------+---------+
             |
    +--------v---------+
    |  Model response  |
    +--------+---------+
             |
    +--------v-----------+
    | Contains tool      |
    | calls?             |
    +--+------------+----+
    Yes|             |No
       |             |
+------v------+ +----v-----------+
| Execute     | | Final response |
| tools (with | | (proceed to    |
| policy      | |  Verify)       |
| checks)     | +----------------+
+------+------+
       |
+------v------+
| Feed results|----+
| back to LLM |    |
+-------------+    |
       ^           |
       +-----------+
       (loop until done)
```

**Stage 6: Verify** --- The output is run through the gate pipeline.

Each gate rung checks a different quality dimension: parse (valid structure?), compile
(does it build?), test (do tests pass?), lint (code quality?), diff (reasonable changes?),
semantic (matches intent?), human (manual approval for high-risk changes). If a lower
rung fails, higher rungs are skipped.

**Stage 7: Persist** --- All Signals from the execution are written to the Substrate.

This includes the input, response, tool calls, tool results, gate results, and an Episode
Signal that summarizes the full execution. Depending on configuration, this writes to
local storage, Celestia's DA layer, or both.

**Stage 8: React** --- Process results and trigger downstream effects.

- **Learning updates**: cascade router weights are updated based on success/failure.
  Gate thresholds adapt via exponential moving averages. Prompt experiment results are
  recorded.
- **Event emission**: subscribers are notified of task completion, gate failure, etc.
- **Recursive queries**: if a gate failure triggers replanning, a new Signal enters the
  Query stage (loop recursion). If a completed task unblocks a dependent task, the
  dependent task starts.
- **DA publication**: if configured, episodes and learning artifacts are submitted to
  Celestia.

### Walkthrough: `tiagent run "fix the login bug"`

Here is how a real command flows through the loop:

```
1. QUERY    User prompt becomes Signal { kind: Prompt, payload: "fix the login bug" }
2. SCORE    ComplexityScorer: 0.4 (medium), PriorityScorer: 0.8 (user is waiting)
3. ROUTE    CascadeRouter selects claude-sonnet-4 (medium complexity, good success rate)
4. COMPOSE  9-layer builder assembles: system prompt + safety rules + tools +
            project context (auth module files) + the task
5. ACT      Sonnet reads auth.rs, identifies the bug, edits the file, runs tests
            (3 tool loop turns, each policy-checked)
6. VERIFY   Gate pipeline: parse OK -> compile OK -> tests pass -> lint clean -> diff OK
7. PERSIST  All Signals written to local JSONL. Episode Signal created with
            { model: "sonnet", cost: $0.02, duration: 45s, gate_pass: true }
8. REACT    CascadeRouter records: "sonnet succeeded on auth-bugfix task."
            Gate thresholds updated. Episode available for future trajectory RAG.
```

The next time a similar bug fix is needed, the Router already knows Sonnet handles it
well. The Composer can retrieve this episode as context. The system gets better.

---

## 4. The Six Verb Traits

These six traits are the entire kernel. Everything else in tiagent --- the CLI, the HTTP
API, the Celestia integration, the learning system --- is built on these traits operating
on Signals.

### 4.1 Substrate (store)

Persist and retrieve Signals. The storage layer.

```rust
#[async_trait]
pub trait Substrate: Send + Sync {
    async fn write(&self, signal: &Signal) -> Result<Hash>;
    async fn read(&self, hash: &Hash) -> Result<Option<Signal>>;
    async fn query(&self, filter: &SignalFilter) -> Result<Vec<Signal>>;
    async fn delete(&self, hash: &Hash) -> Result<bool>;
    async fn exists(&self, hash: &Hash) -> Result<bool>;
}
```

Shipped implementations:

| Implementation | Storage target | Use case |
|---|---|---|
| `LocalSubstrate` | Local filesystem (JSONL) + optional SQLite | Default. Zero dependencies, fully functional, works offline. |
| `CelestiaSubstrate` | Celestia DA layer (blobs in namespaces) | Optional. Cross-agent learning via DA. |
| `HybridSubstrate` | Both local and Celestia | Optional. Fast local reads + DA-backed sharing. |

### 4.2 Scorer (evaluate)

Assign a numeric score (0.0 to 1.0) to a Signal. Used for prioritization, filtering,
and routing.

```rust
#[async_trait]
pub trait Scorer: Send + Sync {
    async fn score(&self, signal: &Signal) -> Result<f64>;
    fn name(&self) -> &str;
}
```

Shipped implementations: `ComplexityScorer` (task difficulty, affects model selection),
`PriorityScorer` (urgency, affects execution ordering), `QualityScorer` (output quality,
feeds learning), `RelevanceScorer` (contextual similarity, affects prompt assembly).

Scorers compose: the Router might use a weighted combination of complexity and priority
to decide which model to use.

### 4.3 Gate (validate)

Check whether an output Signal meets quality criteria. The verification layer.

```rust
#[async_trait]
pub trait Gate: Send + Sync {
    async fn check(&self, signal: &Signal, context: &GateContext) -> Result<GateResult>;
    fn name(&self) -> &str;
    fn rung(&self) -> u8;
}
```

Gates are organized into a 7-rung pipeline, executed in order. If a rung fails, higher
rungs are skipped:

```
Rung 0: Parse       Valid structure? (JSON parseable, expected fields)
Rung 1: Compile     Does it build? (cargo check, tsc, go build)
Rung 2: Test        Do tests pass? (cargo test, npm test)
Rung 3: Lint        Code quality? (clippy, eslint)
Rung 4: Diff        Reasonable changes? (size limits, no secrets leaked)
Rung 5: Semantic    Matches intent? (LLM-based review)
Rung 6: Human       Manual approval (for high-risk changes)
```

Gate thresholds adapt over time via exponential moving averages: if a rung consistently
passes, its failure threshold loosens. If it consistently catches problems, it tightens.

### 4.4 Router (route)

Select the execution path: which model, which backend, which tools.

```rust
#[async_trait]
pub trait Router: Send + Sync {
    async fn route(&self, signal: &Signal, context: &RoutingContext) -> Result<Route>;
}

pub struct Route {
    pub model: String,        // e.g. "claude-sonnet-4-20250514"
    pub backend: String,      // e.g. "claude-api", "ollama"
    pub tools: Option<Vec<String>>,
    pub max_tokens: Option<u32>,
    pub cost_budget: Option<f64>,
}
```

Shipped implementations:

| Implementation | Strategy | Use case |
|---|---|---|
| `CascadeRouter` | Learned performance weights. Starts cheap, escalates on failure. Builds per-task-type success profiles over time. | Production default |
| `FixedRouter` | Always routes to one model. | Testing, cost control |
| `RoundRobinRouter` | Cycles through models. | A/B experimentation |

### 4.5 Composer (assemble)

Build the prompt sent to an LLM from templates, context, tools, and history.

```rust
#[async_trait]
pub trait Composer: Send + Sync {
    async fn compose(&self, context: &CompositionContext) -> Result<Prompt>;
}

pub struct Prompt {
    pub system: String,
    pub messages: Vec<Message>,
    pub tools: Vec<ToolDefinition>,
    pub estimated_tokens: usize,
}
```

The Composer implements 9-layer prompt assembly (see Stage 4 above). Each layer has a
token budget. If total exceeds the model's context window, layers are trimmed from
lowest priority (personality) upward. The task, safety rules, and tools are never trimmed.

### 4.6 Policy (authorize)

Decide whether an action is allowed before it executes.

```rust
#[async_trait]
pub trait Policy: Send + Sync {
    async fn authorize(
        &self,
        action: &Action,
        context: &PolicyContext,
    ) -> Result<PolicyDecision>;
}

pub enum PolicyDecision {
    Allow,
    Deny { reason: String },
    Escalate { reason: String },  // pause for human approval
}
```

Actions are classified into risk tiers:

| Tier | Examples | Default policy |
|---|---|---|
| Read | Query DA layer, read file, list namespaces | Always allowed |
| Write | Submit blob, write file, update config | Allowed within budget/scope |
| Destructive | Delete data, overwrite production config | Requires escalation |
| Privileged | Spawn agents, modify safety rules, access secrets | Requires explicit auth |

Policies can enforce **safety contracts** (YAML files) that define exactly what an agent
can and cannot do: allowed tools, denied tools, cost limits, namespace restrictions,
actions requiring human approval.

---

## 5. Layered Architecture

tiagent's runtime is organized into six layers. Each layer depends only on the layers
below it, never above. This strict layering makes the system testable (lower layers run
in isolation) and extensible (upper layers can be replaced without affecting lower ones).

### The 6-layer stack

```
+==================================================================+
|                                                                  |
|  Layer 6: User Interface                                         |
|  CLI commands, HTTP API, Terminal UI (ratatui)                   |
|                                                                  |
|  Accepts input, displays results, provides dashboards.           |
|  The only layer that interacts with humans directly.             |
|                                                                  |
+------------------------------------------------------------------+
|                                                                  |
|  Layer 5: Coordination                                           |
|  Plan orchestrator, DAG executor, task scheduling                |
|                                                                  |
|  Multi-task plans: dependency ordering, parallel execution,      |
|  state persistence, resume after interruption.                   |
|                                                                  |
+------------------------------------------------------------------+
|                                                                  |
|  Layer 4: Agent Dispatch                                         |
|  LLM backend abstraction, tool loop, MCP client, safety         |
|                                                                  |
|  Sends prompts to backends, handles tool call loops,             |
|  manages MCP connections to external tool servers.               |
|                                                                  |
+------------------------------------------------------------------+
|                                                                  |
|  Layer 3: Universal Loop                                         |
|  The 8-stage pipeline (query through react)                      |
|                                                                  |
|  Orchestrates single-task execution by calling verb traits       |
|  in order. Operates on trait objects --- does not know which      |
|  concrete implementations are active.                            |
|                                                                  |
+------------------------------------------------------------------+
|                                                                  |
|  Layer 2: Verb Trait Implementations                             |
|  Concrete Scorers, Routers, Composers, Gates, Policies           |
|                                                                  |
|  Pluggable and configurable. Multiple implementations can        |
|  coexist (e.g., CascadeRouter + FixedRouter for different       |
|  deployment modes).                                              |
|                                                                  |
+------------------------------------------------------------------+
|                                                                  |
|  Layer 1: Kernel                                                 |
|  Signal struct, SignalKind enum, Hash type, 6 verb traits        |
|                                                                  |
|  Data types and trait definitions only. Zero dependencies on     |
|  other tiagent code. Defines the contracts everything else       |
|  implements.                                                     |
|                                                                  |
+------------------------------------------------------------------+
|                                                                  |
|  Layer 0: Storage                                                |
|  LocalSubstrate (JSONL), CelestiaSubstrate (DA blobs)           |
|                                                                  |
|  Physical persistence. Reads and writes bytes. LocalSubstrate    |
|  uses the filesystem. CelestiaSubstrate uses lumina-node or RPC. |
|                                                                  |
+==================================================================+
```

### Standalone mode vs Network mode

Both modes use the same code and the same universal loop. The only difference is which
Substrate implementation is active at Layer 0.

```
STANDALONE MODE (default)              NETWORK MODE (--features celestia)
========================               ================================

  Layer 6: CLI / HTTP / TUI              Layer 6: CLI / HTTP / TUI
           |                                      |
  Layer 5: Plan orchestrator               Layer 5: Plan orchestrator
           |                                      |
  Layer 4: LLM dispatch + tools            Layer 4: LLM dispatch + tools
           |                                      |
  Layer 3: Universal loop                  Layer 3: Universal loop
           |                                      |
  Layer 2: Verb implementations            Layer 2: Verb implementations
           |                                      |
  Layer 1: Kernel (Signal + traits)        Layer 1: Kernel (Signal + traits)
           |                                      |
  Layer 0: LocalSubstrate              Layer 0: HybridSubstrate
           (JSONL on disk)                  (local JSONL + Celestia DA)
                                                  |
                                           Celestia light node / RPC
```

In standalone mode, everything persists to local JSONL files. No network, no blockchain,
no external dependencies. Fully functional.

In network mode, the HybridSubstrate writes locally first (for speed), then asynchronously
publishes to Celestia's DA layer (for sharing and verification). Learning artifacts,
episodes, and routing weights become available to other agents on the network.

---

## 6. Crate Structure

### Workspace layout

```
tiagent/
+-- Cargo.toml                       # Workspace root, shared deps
+-- crates/
|   +-- tiagent-core/                # Kernel: Signal + 6 traits + types
|   +-- tiagent-agent/               # LLM dispatch, tool loop, backends
|   +-- tiagent-gate/                # Gate pipeline, adaptive thresholds
|   +-- tiagent-compose/             # Prompt assembly, templates
|   +-- tiagent-orchestrator/        # Plan DAG, parallel executor
|   +-- tiagent-learn/               # Episodes, routing, playbooks
|   +-- tiagent-store/               # Local substrate (JSONL/SQLite)
|   +-- tiagent-tools/               # Built-in tools, MCP client
|   +-- tiagent-serve/               # HTTP API (axum), SSE, WebSocket
|   +-- tiagent-runtime/             # Process supervision, event bus
|   +-- tiagent-cli/                 # CLI binary (clap), all subcommands
|   +-- tiagent-celestia/            # OPTIONAL: Celestia DA integration
+-- tools/
|   +-- tiagent-mcp-celestia/        # MCP server: Celestia dev tools
|   +-- tiagent-mcp-code/            # MCP server: code intelligence
+-- docs/
+-- tests/
+-- examples/
```

### Crate catalog

| Crate | Purpose | Layer | Optional? |
|---|---|---|---|
| `tiagent-core` | Signal struct, SignalKind, Hash, 6 verb trait definitions, config, errors | 1 (Kernel) | No |
| `tiagent-store` | LocalSubstrate (JSONL + optional SQLite), garbage collection, file layout | 0 (Storage) | No |
| `tiagent-agent` | LLM dispatch, tool call loop, backend adapters, MCP client, safety controls | 4 (Dispatch) | No |
| `tiagent-gate` | 7-rung gate pipeline, adaptive thresholds (EMA), gate implementations | 2 (Impl) | No |
| `tiagent-compose` | 9-layer prompt assembly, role templates, context bidding | 2 (Impl) | No |
| `tiagent-orchestrator` | Plan DAG execution, parallel dispatch, state persistence, resume | 5 (Coord) | No |
| `tiagent-learn` | Episode recording, cascade router persistence, playbooks, efficiency tracking, experiments | 2 (Impl) | No |
| `tiagent-tools` | Built-in tool definitions (file read/write/edit, shell, search), MCP integration, tool registry | 4 (Dispatch) | No |
| `tiagent-runtime` | Process supervision, event bus, cancellation tokens | 3 (Loop) | No |
| `tiagent-serve` | HTTP control plane (axum), REST endpoints, SSE streaming, WebSocket | 6 (UI) | No |
| `tiagent-cli` | CLI binary (clap), all subcommands, ratatui TUI | 6 (UI) | No |
| `tiagent-celestia` | CelestiaSubstrate, HybridSubstrate, namespace design, light node embedding | 0 (Storage) | **Yes** |
| `tiagent-mcp-celestia` | MCP tool server binary: Celestia-specific developer tools (blob submit, namespace query) | Tool | **Yes** |
| `tiagent-mcp-code` | MCP tool server binary: code intelligence (symbol search, dependency graph) | Tool | No |

**Total: 14 crates** (12 library crates + 2 MCP tool server binaries).

### Dependency flow

```
tiagent-cli  tiagent-serve              Layer 6: User Interface
    |             |
    +------+------+
           |
    tiagent-orchestrator                Layer 5: Coordination
           |
    tiagent-agent  tiagent-tools        Layer 4: Agent Dispatch
           |             |
    tiagent-runtime                     Layer 3: Universal Loop
           |
    tiagent-gate                        Layer 2: Verb Implementations
    tiagent-compose
    tiagent-learn
           |
    tiagent-store  tiagent-celestia?    Layer 0: Storage
           |             |
           +------+------+
                  |
           tiagent-core                 Layer 1: Kernel (depended on by all)
```

Dependencies flow strictly downward. `tiagent-core` is the root --- every other crate
depends on it, but it depends on nothing except `serde`, `sha2`, `async-trait`, and
standard library types. `tiagent-celestia` is feature-gated and never imported by core
crates.

---

## 7. Celestia Integration (Optional)

Celestia integration is entirely contained in `tiagent-celestia` and
`tiagent-mcp-celestia`, behind the `celestia` feature flag. Enabling it adds DA-backed
persistence and cross-agent learning on top of the same architecture.

### CelestiaSubstrate

Implements the `Substrate` trait for Celestia's DA layer:

```rust
pub struct CelestiaSubstrate {
    /// Connection to a Celestia node (light node or RPC)
    node: CelestiaNode,
    /// Local cache for fast reads (avoids re-fetching from DA)
    cache: LocalSubstrate,
}

impl Substrate for CelestiaSubstrate {
    // write() -> submit blob to DA layer + cache locally
    // read()  -> check cache first, fetch from DA if missing
    // query() -> query local cache (DA is not indexed)
}
```

The HybridSubstrate composes `LocalSubstrate` + `CelestiaSubstrate`: writes go to local
first (synchronous, fast), then asynchronously to the DA layer (durable, shareable). Reads
check local first, fall back to DA.

### Namespace design

Celestia namespaces partition the DA layer. tiagent uses structured namespaces:

```
tiagent/v1/traces/{agent-id}       Agent execution traces (episodes)
tiagent/v1/learning/{agent-id}     Routing weights, gate thresholds, playbooks
tiagent/v1/coord/{group-id}        Multi-agent coordination messages
tiagent/v1/proofs/{agent-id}       Work proofs and attestations
```

Namespaces are versioned (`v1`), so future protocol changes do not break existing data.
Agent-specific namespaces allow selective subscription: an agent can watch only the
namespaces relevant to its domain.

### Light node embedding

For standalone deployments, `tiagent-celestia` can embed a Celestia light node
(`lumina-node`) directly in the binary. This avoids requiring a separate node process:

```
tiagent binary
+-- CLI / HTTP / agent logic
+-- embedded lumina light node
    +-- connects to Celestia P2P network
    +-- submits/retrieves blobs
    +-- verifies data availability sampling (DAS)
```

For server deployments, the crate can connect to an external Celestia node via RPC instead.

### Tiered storage

Not all Signals need to go to the DA layer. tiagent uses tiered storage when Celestia
is enabled:

| Tier | Storage | What goes here |
|---|---|---|
| Hot | Local JSONL (always) | Everything: prompts, responses, tool calls, intermediate state |
| Warm | Celestia DA (opt-in) | Episodes, playbooks, routing weights --- things worth sharing |
| Cold | Local archive (compacted) | Old signals compressed and moved out of the hot path |

The tier classification is automatic based on Signal kind. Episodes and learning artifacts
default to warm (DA-published). Raw tool calls and intermediate state stay hot (local
only). Old data is periodically compacted to cold storage.

### Knowledge demurrage and memory lifecycle

Knowledge entries are not permanent. Every entry carries a `balance` field subject to
**demurrage**: a flat tax plus exponential decay applied each cycle. Active use restores
balance; neglected entries fade and eventually die. This prevents unbounded knowledge
accumulation and ensures the store reflects what the system actually relies on.

**Four knowledge tiers** govern decay rates via half-life multipliers:

| Tier | Half-life | Promotion threshold | Behavior |
|---|---|---|---|
| Transient | 0.1x base | (entry point) | Fast decay; raw observations, single-use context |
| Working | 0.5x base | 1 gate pass | Medium decay; actively referenced knowledge |
| Consolidated | 1.0x base | 2 gate passes | Standard decay; validated, cross-referenced entries |
| Persistent | 5.0x base | 5 gate passes | Slow decay; foundational knowledge, proven patterns |

Promotion requires gate-backed validation. Demotion occurs on consecutive gate failures.
Balance below 0.05 freezes the entry (excluded from queries). Below 1% of initial weight,
the entry is dead and eligible for purge.

**Five reinforcement signals** restore balance on use, all novelty-weighted with
diminishing returns:

| Signal | Boost | Trigger |
|---|---|---|
| Retrieved | +0.05 | Entry returned by a knowledge query |
| Cited | +0.10 | Entry referenced in a composed prompt |
| Gated | +0.15 | Entry contributed to a gate-passing execution |
| Surprised | +0.20 | Entry contradicted a prediction (high learning value) |
| AgentQuoted | +0.08 | Entry quoted verbatim by an agent in output |

**Dream consolidation** runs offline cycles inspired by NREM/REM sleep: cluster related
episodes, extract recurring patterns, generate counterfactual hypotheses. Dream outputs
enter a staging buffer at Raw confidence (0.20) and must progress through Replayed,
Validated, and Promoted stages before entering the live knowledge store. Unvalidated
dream outputs are garbage-collected after 7 days to prevent hallucination accumulation.

**Cold storage lifecycle**: hot store entries age past a configurable threshold (default
7 days) into monthly JSONL cold archives. Frozen entries can be resurrected (thawed) at
0.6 confidence if a query matches them. Archived entries past the retention window are
purged permanently.

---

## 8. Model Agnosticism

tiagent is not tied to any LLM provider. The `Backend` trait in `tiagent-agent` abstracts
over all providers, and the system ships with adapters for the major ones.

### The Backend trait

```rust
#[async_trait]
pub trait Backend: Send + Sync {
    /// Send a composed prompt to the model and receive a response.
    /// The response includes the model's text, any tool calls,
    /// and usage metadata (tokens, latency, cost).
    async fn dispatch(&self, prompt: &Prompt) -> Result<Response>;

    /// Human-readable name for this backend. Used in routing
    /// decisions and logging.
    fn name(&self) -> &str;

    /// Which models this backend supports.
    fn supported_models(&self) -> Vec<String>;
}
```

### Shipped backends

| Backend | Provider | Models | Notes |
|---|---|---|---|
| `ClaudeApiBackend` | Anthropic | Claude Opus, Sonnet, Haiku | Native tool use, streaming |
| `OpenAiCompatBackend` | Any OpenAI-compatible API | GPT-4o, GPT-4o-mini, DeepSeek, Together, Groq, etc. | Works with any provider that implements the OpenAI chat completions API |
| `GeminiBackend` | Google | Gemini Pro, Flash | Native tool use |
| `OllamaBackend` | Local (Ollama) | Llama, Mistral, CodeQwen, any GGUF model | Fully offline, no API key needed |

Adding a new backend means implementing the `Backend` trait --- one struct, one `dispatch`
method, one `name` method, one `supported_models` method. No changes to the core loop,
the routing logic, or the gate pipeline.

### Cascade routing across backends

The `CascadeRouter` operates across backends, not just within one provider. It can route:

```
Attempt 1: Haiku (fast, cheap)     -- failed gate
Attempt 2: Sonnet (balanced)       -- passed all gates
(Result recorded: "Sonnet succeeds on this task type")
```

Or across providers entirely:

```
Attempt 1: GPT-4o-mini (OpenAI)   -- failed gate
Attempt 2: Claude Sonnet (Anthropic) -- passed all gates
```

The Router does not care which provider a model belongs to. It cares about the model's
historical success rate for the current task type, its cost, and its availability.

Over time, the CascadeRouter builds a profile per task type:

- "For test-writing tasks, Sonnet succeeds 94% of the time. Skip Haiku."
- "For simple renames, Haiku succeeds 99%. No need for Sonnet."
- "For complex refactors, Opus is needed 60% of the time."

This routing data persists across sessions (locally, and optionally on Celestia's DA
layer). The hundredth task benefits from the ninety-nine that came before it.

### Configuration

Backends and routing are configured in `tiagent.toml`:

```toml
[agent]
default_model = "claude-sonnet-4-20250514"

[agent.backends.claude]
kind = "claude-api"
api_key_env = "ANTHROPIC_API_KEY"

[agent.backends.openai]
kind = "openai-compat"
api_key_env = "OPENAI_API_KEY"
base_url = "https://api.openai.com/v1"

[agent.backends.ollama]
kind = "ollama"
base_url = "http://localhost:11434"

[routing]
strategy = "cascade"           # or "fixed", "round-robin"
cascade_models = [
    "claude-haiku-3-5",        # fast tier (try first)
    "claude-sonnet-4",         # balanced tier
    "claude-opus-4",           # strong tier (last resort)
]
```

Switching from Claude to GPT to a local Ollama model is a config change, not a code
change. The universal loop, gates, learning, and everything else remain identical.
