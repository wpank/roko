# Roko as WASM: Universal Agent Runtime, Composable Framework, and Breakthrough Vision

> **Goal**: Make roko the most modular, composable, and universally deployable agent framework
> — running natively on CLI/server, in browsers via WASM, on edge via CDN workers, and
> composing with other agents via open protocols. Design for exponential synergy: the more
> instances run, the smarter all instances become.

---

## Table of Contents

1. [The Vision: Agents That Run Anywhere](#1-the-vision)
2. [WASM Compilation: What Works, What Doesn't](#2-wasm-compilation)
3. [The Architecture: Isomorphic Agent Core](#3-the-architecture)
4. [WASM Component Model for Agent Modularity](#4-wasm-component-model)
5. [Browser-Native Agent Runtime](#5-browser-native)
6. [Edge-Cloud Hybrid Architecture](#6-edge-cloud-hybrid)
7. [Progressive Enhancement Tiers](#7-progressive-enhancement)
8. [Distributed Learning: Merkle-CRDTs for Shared Intelligence](#8-distributed-learning)
9. [Protocol Stack: MCP + A2A + ACP](#9-protocol-stack)
10. [Portable Agent State: The "Brain Dump" Format](#10-portable-state)
11. [Breakthrough Use Cases](#11-breakthrough-use-cases)
12. [Self-Modifying Agent Architecture](#12-self-modifying)
13. [Agent Marketplace and Skills Economy](#13-marketplace)
14. [What Needs to Change in Roko](#14-what-needs-to-change)
15. [Implementation Roadmap](#15-implementation-roadmap)
16. [Research Sources](#16-sources)

---

## 1. The Vision

Roko should be three things simultaneously:

1. **A CLI agent** (what it is today) — runs plans, gates, learns, self-improves
2. **An embeddable WASM component** — runs in browsers, edge workers, notebooks, CI pipelines, other agents
3. **A composable node in a distributed agent network** — discovers, delegates, and learns from other agents

The key insight: these aren't three separate products. They're the same core compiled to three targets, with the same learning state synchronized across all instances via CRDTs. An agent that learns a pattern in CI teaches the same pattern to the agent running in your IDE, which teaches it to the agent embedded in your documentation.

**What no one else has built**: A self-improving agent framework where the learning compounds across deployment targets. Browser instances, CLI instances, edge instances, and CI instances all contribute observations to a shared knowledge base, and the CascadeRouter, skill library, and playbook rules improve for ALL instances simultaneously.

---

## 2. WASM Compilation: What Works, What Doesn't

### Fully Compatible (zero changes needed)

| Component | Crate | WASM Status |
|---|---|---|
| Config parsing | `serde`, `serde_json`, `toml` | Works perfectly |
| Model routing | `cascade_router.rs` (pure math) | Works perfectly |
| Learning algorithms | LinUCB, Thompson, UCB1 | Works perfectly — pure computation |
| Cost normalization | `cost_table.rs` | Works perfectly |
| Prompt assembly | `system_prompt_builder.rs` | Works perfectly |
| Token counting | `tiktoken-wasm` | Dedicated WASM build exists |
| Episode/signal types | All roko-core types | Works perfectly |

### Compatible with Platform Abstraction

| Component | Native | WASM Browser | WASM Edge (WASI) |
|---|---|---|---|
| Async runtime | `tokio` | `wasm-bindgen-futures::spawn_local` | `tokio_wasi` (WasmEdge) |
| HTTP client | `reqwest` (native TLS) | `reqwest` (Fetch API — built-in!) | `reqwest` (WASI sockets) |
| Persistence | `std::fs` (JSONL files) | IndexedDB / OPFS / SQLite WASM | WASI filesystem |
| Sync primitives | `parking_lot::Mutex` | `RefCell` (single-threaded) | `parking_lot` (works) |
| Random | `rand` | `rand` (with `js` feature) | `rand` (with `wasi` feature) |
| Time | `chrono` | `js_sys::Date` | WASI clocks |

### Hard Blockers (require architectural change)

| Component | Why It Fails | Solution |
|---|---|---|
| **Agent dispatch (subprocess)** | `std::process::Command` doesn't exist in WASM | Replace CLI spawn with direct HTTP API calls |
| **Gate pipeline (cargo/clippy)** | Can't run compilers in browser | Delegate to server-side gate runner |
| **MCP subprocess servers** | Can't spawn MCP stdio processes | Use HTTP MCP transport instead |
| **File watcher** | No inotify/FSEvents in WASM | Use polling or IndexedDB change events |

### Binary Size Expectations

| Config | Size (uncompressed) | Size (gzipped) |
|---|---|---|
| Full agent (all features) | ~3-5 MB | ~800 KB - 1.5 MB |
| Core only (routing + learning) | ~500 KB - 1 MB | ~150-300 KB |
| Minimal (heuristic-only, no LLM) | ~100-200 KB | ~40-80 KB |

Optimization: `opt-level = "z"`, `lto = true`, `codegen-units = 1`, `panic = "abort"`, `strip = true`, then `wasm-opt -Oz`.

---

## 3. The Architecture: Isomorphic Agent Core

### The Layer Split

```
┌─────────────────────────────────────────────────────────┐
│                    roko-core (pure Rust)                  │
│  Signal, Score, ToolDef, ChatRequest, ProviderConfig     │
│  NO I/O, NO async, NO platform deps                      │
│  Compiles to: native + wasm32-unknown-unknown + wasip2   │
└──────────────────────────┬──────────────────────────────┘
                           │
        ┌──────────────────┼──────────────────┐
        │                  │                  │
┌───────┴───────┐  ┌───────┴───────┐  ┌───────┴───────┐
│  roko-native  │  │  roko-wasm    │  │  roko-edge    │
│  (CLI/server) │  │  (browser)    │  │  (WASI/CDN)   │
│               │  │               │  │               │
│ tokio         │  │ spawn_local   │  │ tokio_wasi    │
│ std::fs       │  │ IndexedDB     │  │ WASI FS       │
│ reqwest(tls)  │  │ reqwest(fetch)│  │ reqwest(wasi) │
│ subprocess    │  │ HTTP API only │  │ HTTP API only │
│ full gates    │  │ remote gates  │  │ remote gates  │
└───────────────┘  └───────────────┘  └───────────────┘
```

### The Abstraction Traits

```rust
/// Platform-agnostic I/O — the bridge between core and platform adapters.
/// Lives in roko-core. Implemented per platform.

#[async_trait(?Send)]  // ?Send for WASM compatibility
pub trait StateStore {
    async fn read(&self, key: &str) -> Option<Vec<u8>>;
    async fn write(&self, key: &str, data: &[u8]) -> Result<()>;
    async fn append(&self, key: &str, line: &[u8]) -> Result<()>;
    async fn list_keys(&self, prefix: &str) -> Vec<String>;
}

#[async_trait(?Send)]
pub trait LlmClient {
    async fn chat_completion(&self, request: &ChatRequest) -> Result<ChatResponse>;
    async fn stream_completion(&self, request: &ChatRequest)
        -> Result<Pin<Box<dyn Stream<Item = StreamChunk>>>>;
}

#[async_trait(?Send)]
pub trait GateRunner {
    async fn run_gate(&self, gate: &str, workdir: &str) -> Result<GateResult>;
}

#[async_trait(?Send)]
pub trait EventSink {
    fn publish(&self, event: AgentEvent);
}
```

Note: `?Send` bound is critical — WASM futures are `!Send` (single-threaded). Using `?Send` makes these traits compatible with both native (`Send` futures) and WASM (`!Send` futures).

### Platform Implementations

**Native** (`roko-native`):
```rust
struct NativeStateStore { base_dir: PathBuf }
impl StateStore for NativeStateStore {
    async fn read(&self, key: &str) -> Option<Vec<u8>> {
        tokio::fs::read(self.base_dir.join(key)).await.ok()
    }
    // ... std::fs for everything
}
```

**Browser** (`roko-wasm`):
```rust
struct BrowserStateStore { db: indexed_db::Database }
impl StateStore for BrowserStateStore {
    async fn read(&self, key: &str) -> Option<Vec<u8>> {
        self.db.transaction("state").object_store("state")
            .get(key).await.ok().flatten()
    }
    // ... IndexedDB for everything
}
```

**Edge** (`roko-edge`):
```rust
struct WasiStateStore; // Uses WASI filesystem
struct CloudflareKvStore { kv: worker::kv::KvStore }
struct SpinKvStore { store: spin_sdk::key_value::Store }
```

---

## 4. WASM Component Model for Agent Modularity

### Why Components

The WASM Component Model lets you build the agent as **composable, sandboxed WASM components** that plug together via typed interfaces (WIT). Each component:
- Runs in isolated memory (no shared state — security by default)
- Can be written in different languages (Rust, Python, Go, TypeScript)
- Composes at the binary level (`wasm-tools compose`)
- Can be independently versioned, tested, deployed, and replaced

### Agent Components

```wit
// scoring.wit — signal evaluation
interface scoring {
    record signal-score {
        confidence: f32,
        novelty: f32,
        utility: f32,
    }
    score: func(signal-body: string, signal-kind: string) -> signal-score;
}

// routing.wit — model selection
interface routing {
    record route-decision {
        model-slug: string,
        provider: string,
        confidence: f32,
        stage: string,
    }
    route: func(role: string, task-category: string, complexity: string) -> route-decision;
    observe: func(model-slug: string, reward: f32, success: bool);
}

// gating.wit — quality verification
interface gating {
    record gate-result {
        gate-name: string,
        passed: bool,
        score: f32,
        detail: string,
    }
    run-gate: func(gate-name: string, content: string) -> gate-result;
}

// learning.wit — knowledge accumulation
interface learning {
    record skill {
        name: string,
        precondition: string,
        procedure: string,
        confidence: f32,
    }
    query-skills: func(task-category: string, top-k: u32) -> list<skill>;
    record-episode: func(episode-json: string);
    get-routing-weights: func() -> list<tuple<string, f32>>;
}

// The composed agent world
world roko-agent {
    import scoring;
    import routing;
    import gating;
    import learning;

    export handle-prompt: func(prompt: string) -> string;
    export handle-acp-message: func(method: string, params: string) -> string;
}
```

### Composition Example

```bash
# Build each component independently
cargo component build -p roko-scoring --release
cargo component build -p roko-routing --release
cargo component build -p roko-gating --release
cargo component build -p roko-learning --release
cargo component build -p roko-agent-core --release

# Compose into a single agent binary
wac plug target/wasm32-wasip2/release/roko_agent_core.wasm \
    --plug target/wasm32-wasip2/release/roko_scoring.wasm \
    --plug target/wasm32-wasip2/release/roko_routing.wasm \
    --plug target/wasm32-wasip2/release/roko_gating.wasm \
    --plug target/wasm32-wasip2/release/roko_learning.wasm \
    -o roko-agent-composed.wasm
```

### What This Enables

- **Swap the router**: Replace LinUCB with Thompson Sampling by plugging a different routing component
- **Custom gates**: A Python gate component that validates scientific claims alongside the Rust compile gate
- **Third-party skills**: A community-contributed learning component trained on web development patterns
- **Sandboxed tool execution**: Each tool runs in its own WASM instance (Agentor pattern: 1.2ms overhead, total blast radius containment)

---

## 5. Browser-Native Agent Runtime

### What Runs in the Browser

```
Browser Tab
├── roko-wasm (compiled to wasm32-unknown-unknown)
│   ├── CascadeRouter (pure computation — runs locally)
│   ├── SkillLibrary (loaded from IndexedDB)
│   ├── PlaybookRules (loaded from IndexedDB)
│   ├── PromptAssembler (pure computation)
│   ├── TokenCounter (tiktoken-wasm)
│   ├── CostTable (pure computation)
│   ├── AnomalyDetector (pure computation)
│   └── EpisodeLogger (writes to IndexedDB)
│
├── HTTP Calls (via browser Fetch API / reqwest)
│   ├── LLM Provider APIs (GLM, Kimi, OpenRouter, etc.)
│   ├── Gate Runner API (server-side compile/test)
│   └── Sync API (CRDT merge with other instances)
│
├── Persistent Storage
│   ├── IndexedDB: episodes, signals, config
│   ├── OPFS: routing weights, gate thresholds (via SQLite WASM)
│   └── Cache API: LLM response cache
│
└── UI (DOM via wasm-bindgen or framework)
    ├── Chat interface
    ├── Dashboard (plan progress, gate results, learning metrics)
    └── Diff viewer (file edit approval)
```

### What CANNOT Run in the Browser

- **Compilation gates** (cargo check, cargo test, clippy) — must delegate to server
- **Subprocess agents** (Claude CLI, Cursor ACP as subprocess) — must use HTTP APIs directly
- **MCP stdio servers** — must use HTTP/SSE MCP transport instead
- **File system access** to real project files — must use browser FS APIs or server proxy

### The Thin Client Pattern

The browser roko is a "smart thin client":
- **All intelligence runs locally**: Routing decisions, prompt assembly, cost tracking, anomaly detection, learning updates
- **Only I/O goes to the network**: LLM API calls, gate execution, file syncs
- **Learning state persists locally**: IndexedDB/OPFS survives page reloads, accumulates across sessions
- **Periodically syncs with server**: CRDT merge uploads local learnings, downloads others' learnings

This means the browser agent gets **smarter over time** even without a server — it just misses the collective learning until it syncs.

### Existing Precedent

**Bolt.new** (StackBlitz): Runs a full Node.js development environment in the browser via WebContainers. Claude Sonnet has full control over filesystem, server, package manager, terminal. Zero server-side execution during development.

**Mozilla AI WASM Agents Blueprint**: Runs the OpenAI Agents SDK in WASM via Pyodide. Agents as plain HTML files. The 3W stack (WebLLM + WASM + WebWorkers) for fully client-side execution.

**Notion**: Migrated to WASM SQLite + OPFS for browser persistence, with major speed improvements.

---

## 6. Edge-Cloud Hybrid Architecture

### Three Tiers

```
┌──────────────────────────────────────────────┐
│              Tier 1: Edge (WASM)              │
│  Browser tab / CDN worker / Edge function     │
│                                               │
│  Responsibilities:                            │
│    - Intent classification (<10ms)            │
│    - Routing decision (cached weights)        │
│    - Cost estimation                          │
│    - Cache lookup (response dedup)            │
│    - Input validation                         │
│                                               │
│  Runtime: Cloudflare Workers / Fermyon Spin   │
│  Memory: 128MB / Cold start: <1ms             │
└──────────────────┬───────────────────────────┘
                   │ A2A delegation
┌──────────────────┴───────────────────────────┐
│           Tier 2: Regional (WASM/native)      │
│  Cloud function / container                   │
│                                               │
│  Responsibilities:                            │
│    - Prompt assembly (full context)           │
│    - Tool dispatch (Read, Edit, Search)       │
│    - Gate validation (compile, test)          │
│    - Learning updates                         │
│                                               │
│  Runtime: Wasmtime / native binary            │
│  Latency: 50-200ms                            │
└──────────────────┬───────────────────────────┘
                   │ A2A delegation
┌──────────────────┴───────────────────────────┐
│            Tier 3: Central (native)           │
│  Full server / GPU-equipped                   │
│                                               │
│  Responsibilities:                            │
│    - LLM inference coordination               │
│    - Full state management                    │
│    - Experiment tracking                      │
│    - CRDT merge point (sync all instances)    │
│    - Heavy computation (HDC, embeddings)      │
│                                               │
│  Runtime: Native Rust binary                  │
│  Latency: 200ms-30s (LLM dependent)          │
└──────────────────────────────────────────────┘
```

### How Tiers Communicate

Each tier publishes an **A2A Agent Card** describing its capabilities. When the edge agent receives a request it can't handle (needs compilation, needs full LLM), it discovers and delegates to a regional or central agent via A2A's task API.

---

## 7. Progressive Enhancement Tiers

| Tier | Capabilities | What Works | What Doesn't |
|---|---|---|---|
| **0: Heuristic** | No LLM, no network | Cached routing tables, playbook replay, static gates | No adaptation, no new learning |
| **1: Local LLM** | Small model via WASI-NN or WebLLM | Basic classification, simple code edits, local learning | No frontier model quality |
| **2: Cloud LLM** | Full API access, single instance | Complete agent loop, all gates, experiment participation | No collective learning |
| **3: Distributed** | Multiple instances, CRDT sync | Shared routing weights, collective skill library, fleet experiments | Requires network for sync |

Degradation is automatic:
```rust
async fn route(&self, input: &str) -> RouteDecision {
    if self.capabilities.has_cloud_llm() {
        self.cascade_router.route(input).await     // Tier 2-3
    } else if self.capabilities.has_local_llm() {
        self.local_classifier.classify(input).await // Tier 1
    } else {
        self.cached_routes.lookup(input)            // Tier 0
    }
}
```

---

## 8. Distributed Learning: Merkle-CRDTs for Shared Intelligence

### The Exponential Insight

Every roko instance (browser, CLI, edge, CI) produces learning data:
- Routing observations (model X succeeded/failed on task type Y)
- Gate threshold updates (EMA pass rates per rung)
- Skill discoveries (successful tool-use patterns)
- Cost data (actual costs per model per provider)

If N instances share this data, each instance learns N times faster. With 1000 users, a new user's agent starts with the collective experience of all 1000.

### How It Works

**Merkle-CRDTs** combine:
1. **CRDTs** (Conflict-free Replicated Data Types): Guarantee eventual consistency without coordination. Any replica can process writes independently. Merges are commutative, associative, and idempotent.
2. **Merkle DAGs**: Enable efficient pair-wise reconciliation. Instead of syncing full state, nodes exchange tree hashes to identify divergences.

### Mapping to Roko's Learning State

| Data | CRDT Type | Merge Strategy |
|---|---|---|
| Routing observations | G-Counter per (model, category) | Sum observations across instances |
| Gate thresholds (EMA) | LWW-Register with Lamport timestamps | Latest writer wins |
| Skill library | G-Set (add-only) | Union of all discovered skills |
| Experiment results | G-Set of (variant_id, outcome) | Union of all observations |
| Cost records | G-Set | Union (dedup by record_id) |
| Playbook rules | OR-Set (add + remove) | Merge with tombstones |
| Provider health | LWW-Map | Latest observation per provider |

### Sync Protocol

```
Instance A                              Instance B
    |                                       |
    |--- Merkle root hash ----------------->|
    |<-- Merkle root hash ------------------|
    |                                       |
    |  (Roots differ: need sync)            |
    |                                       |
    |--- Divergent subtree hashes --------->|
    |<-- Missing entries from B ------------|
    |--- Missing entries from A ----------->|
    |                                       |
    | (Both merge. Roots now match.)        |
```

### Privacy-Preserving Mode

For sensitive codebases, sync only aggregate statistics:
- Routing: "(model X, task_type Y) had 82% pass rate across 203 observations" — no code content
- Skills: "Read-before-Edit pattern succeeds 94% of the time" — no specific files
- Costs: "$0.19 average for GLM-5.1 on implementation tasks" — no prompts

This is differential privacy at the application level: share the learned patterns, not the learning data.

---

## 9. Protocol Stack: MCP + A2A + ACP

### The Three-Layer Stack

```
Layer 3: User Interface    ACP (Agent Client Protocol)
         Agent ↔ IDE       roko ↔ VS Code / Zed / JetBrains

Layer 2: Agent Coordination A2A (Agent-to-Agent Protocol)
         Agent ↔ Agent     roko edge ↔ roko cloud ↔ third-party agents

Layer 1: Tool Access       MCP (Model Context Protocol)
         Agent ↔ Tools     roko ↔ code search / databases / APIs
```

### How Roko Uses Each

**MCP** (already partially implemented):
- Roko consumes MCP tools for code search, documentation, external APIs
- Roko PROVIDES MCP tools (plan execution, research, status) — other agents can use roko as a tool
- `roko serve --mcp` exposes roko's capabilities as MCP tools

**A2A** (new):
- Roko publishes an Agent Card describing its capabilities (plan execution, gate validation, learning)
- Browser roko delegates compilation to server roko via A2A task API
- Multiple roko instances discover each other for CRDT sync
- Third-party agents (LangGraph, CrewAI) can delegate tasks to roko

**ACP** (from ide.md):
- `roko acp` starts roko as an ACP agent for IDE integration
- Streams plan progress, gate results, learning metrics to the IDE
- Receives prompts and approval decisions from the IDE

### Roko's Agent Card (A2A)

```json
{
  "name": "roko",
  "description": "Self-improving coding agent with plan execution, gate verification, and learning",
  "url": "https://roko.local:9090/a2a",
  "version": "0.1.0",
  "capabilities": {
    "streaming": true,
    "pushNotifications": true,
    "stateTransitionHistory": true
  },
  "skills": [
    {
      "id": "plan-execute",
      "name": "Execute coding plan with gate verification",
      "description": "Run a multi-task plan through DAG executor with compile/test/clippy gates",
      "inputModes": ["text/plain", "application/json"],
      "outputModes": ["text/plain", "application/json"]
    },
    {
      "id": "code-review",
      "name": "Multi-reviewer code review",
      "description": "Architect + Auditor + Scribe review with structured feedback",
      "inputModes": ["text/plain"],
      "outputModes": ["application/json"]
    },
    {
      "id": "research",
      "name": "Deep research with citations",
      "description": "Research a topic and produce a cited report",
      "inputModes": ["text/plain"],
      "outputModes": ["text/markdown"]
    }
  ]
}
```

---

## 10. Portable Agent State: The "Brain Dump" Format

### What Gets Exported

A roko "brain dump" packages all learned state into a portable artifact:

```
roko-brain-v1/
├── manifest.json              # Version, source instance, timestamp, capabilities
├── routing/
│   ├── cascade-router.json    # Confidence stats + LinUCB arm state
│   ├── thompson-arms.json     # Thompson Sampling beta distributions
│   └── static-table.json     # Cold-start routing defaults
├── learning/
│   ├── gate-thresholds.json   # EMA pass rates per rung
│   ├── efficiency-summary.json # Aggregated (not raw) efficiency metrics
│   ├── cost-table.json        # Observed costs per model
│   └── latency-stats.json     # Observed latency per provider
├── skills/
│   ├── skill-library.json     # Discovered skills with confidence
│   └── playbook-rules.json   # Validated playbook entries
├── heuristics/
│   ├── patterns.json          # Mined patterns from episodes
│   └── section-effects.json   # Prompt section effectiveness data
├── experiments/
│   ├── prompt-experiments.json # A/B test results
│   └── model-experiments.json # Model comparison results
└── config/
    ├── providers.json         # Provider configurations (keys stripped)
    └── models.json            # Model profiles
```

### Import/Export Commands

```bash
# Export learned state
roko brain export --output roko-brain-v1.tar.gz

# Import into a new instance (merges via CRDT, doesn't overwrite)
roko brain import roko-brain-v1.tar.gz

# Share with the collective (privacy-preserving aggregates only)
roko brain share --mode aggregate --endpoint https://roko-collective.example.com/sync
```

### What This Enables

- **Onboarding**: A new team member imports the team's collective brain. Their agent instantly has good routing weights, proven skills, and calibrated thresholds.
- **Cross-project transfer**: Skills learned on project A (e.g., "always run tests after editing lib.rs") transfer to project B.
- **Agent marketplace**: Publish your agent's brain as a product. Buy specialized brains trained on specific domains (web dev, systems programming, data science).

---

## 11. Breakthrough Use Cases

### Use Case 1: Agents in Documentation

An agent embedded in your docs page (via roko-wasm) that:
- Answers questions about the code by reading the actual source (via MCP)
- Detects when docs are outdated (compares doc content to code)
- Proposes and applies doc updates (via A2A delegation to a server-side agent with write access)
- Learns which questions are asked most often and pre-generates answers

### Use Case 2: CI Pipeline Agent

A WASM roko component in GitHub Actions that:
- Runs in the CI environment (no Docker, just WASM)
- Reviews PRs with the collective learning from all team members' agents
- Fixes failing tests autonomously (delegates to cloud roko via A2A)
- Updates routing weights based on CI outcomes (which models produce mergeable code?)
- Reports gate results back to the PR as structured comments

### Use Case 3: Self-Improving Documentation Bot

A roko agent on your docs site that:
- Runs entirely in the visitor's browser (roko-wasm)
- Routes questions to the cheapest adequate model (CascadeRouter with learned weights)
- Accumulates question patterns in IndexedDB
- Syncs patterns to the server (CRDT merge)
- The server agent generates new FAQ entries from common patterns
- The generated entries are PR'd to the docs repo (A2A delegation)

### Use Case 4: Distributed Code Review Fleet

Multiple roko instances (one per developer) that:
- Each learn which patterns cause PR rejections in THEIR codebase
- Sync pattern discoveries via Merkle-CRDT
- Pre-flight check code BEFORE pushing (local roko runs gates)
- Share skill libraries (team-wide "don't do X in this codebase" rules)
- The collective improves faster than any individual instance

### Use Case 5: Notebook Agent-Researcher

A roko-wasm component embedded in Jupyter/Observable that:
- Proposes experiments based on learned patterns (CellVoyager pattern)
- Executes code cells and interprets results
- Routes computation: simple stats locally (WASM), ML training to cloud (A2A)
- Accumulates domain-specific skills in the notebook's local storage
- Shares skills across notebooks via brain export/import

### Use Case 6: Agent-Native IDE (long-term)

Not a fork of VS Code, but a new interface paradigm:
- The agent is the primary developer; the human reviews
- Plan DAG is the primary view (not file explorer)
- Multiple agents work in parallel on different tasks
- Gate results stream in real-time
- Learning metrics visible as a live dashboard
- Built on ACP for agent communication, MCP for tools, A2A for delegation
- Roko-wasm runs the UI; roko-native runs the agents

---

## 12. Self-Modifying Agent Architecture

### The State of the Art

**HyperAgents** (Meta, ICLR 2026): Merges the task agent and meta-agent into a single self-modifiable codebase. The agent improves how it improves. 3x improvement on coding benchmarks through self-modification alone.

**Darwin Godel Machine** (Sakana AI): Darwinian evolution + Godelian self-improvement. Maintains expanding lineage of agent variants. SWE-bench 20% → 50%.

**A-Evolve** (open source): Five-stage evolutionary loop: Solve → Observe → Evolve → Gate → Reload. Git-tagged mutations with automatic rollback on regression.

### How This Maps to Roko

Roko already has the foundation:
- **Solve**: Agent executes tasks via plan runner
- **Observe**: EpisodeLogger + EfficiencyEvents capture execution data
- **Evolve**: roko-neuro distiller extracts heuristics (WIP)
- **Gate**: 6-rung gate pipeline validates changes
- **Reload**: `--resume` reloads state; playbook rules are injected into prompts

What's missing for full self-modification:
1. **Prompt evolution**: GEPA-style reflective prompt mutation using execution traces (deferred to Phase 5+)
2. **Framework code modification**: Agent modifying its own WIT interfaces or Rust code (HyperAgents pattern — very advanced)
3. **Evolutionary history**: Git-tagged mutations of the agent's configuration (A-Evolve pattern)

### The Safe Self-Modification Path

```
1. Agent runs task → gate fails
2. roko-neuro distiller analyzes failure → extracts heuristic
3. Heuristic is tested on next similar task → tracked as experiment
4. If heuristic improves pass rate by >5% with p<0.05 → promote to playbook
5. Playbook rule is injected into all future prompts
6. Agent behavior has been modified WITHOUT changing code

This is self-modification at the prompt level, not the code level.
It's safe because the gate pipeline validates every modification.
```

---

## 13. Agent Marketplace and Skills Economy

### What's Real Today

**Agensi** (agensi.io): Marketplace for AI agent skills. Buy once, install instantly, own forever. Works across Claude Code, Codex CLI, Cursor, and 20+ agents.

**Agent Skills** (Anthropic, December 2025): Open standard adopted by OpenAI, Microsoft, Google, Cursor, GitHub, Figma. Skills are SKILL.md files with YAML frontmatter. 1,600+ security-vetted skills indexed.

### Roko's Marketplace Opportunity

Roko could trade not just skills, but **learned state**:

| Product Type | What It Contains | Example |
|---|---|---|
| **Skill Pack** | SKILL.md files + playbook rules | "Rust Error Handling Best Practices" |
| **Brain** | Full learned state (routing + skills + thresholds) | "Senior Rust Developer Brain" |
| **Routing Profile** | Trained CascadeRouter weights | "Cost-Optimized GLM+Kimi Router" |
| **Gate Config** | Custom gate pipeline + thresholds | "High-Security Gate Pipeline" |
| **Template Pack** | Plan + task templates for specific workflows | "Microservice Migration Kit" |

The unique value: roko's brains include **empirically validated** routing weights and skills, not just static instructions. A brain that says "GLM-5.1 passes 82% on implementation tasks at $0.19/task" is worth more than a skill that says "use GLM-5.1 for implementation."

---

## 14. What Needs to Change in Roko

### Tier 1: Platform Abstraction (enables WASM compilation)

| Change | What | Effort |
|---|---|---|
| Abstract async runtime | Trait for `spawn`, `sleep`, `timeout` | Medium |
| Abstract state storage | `StateStore` trait replacing `std::fs` | Medium |
| Abstract agent dispatch | HTTP API calls replacing subprocess spawn | **High** |
| Feature-gate CLI-only code | `#[cfg(not(target_arch = "wasm32"))]` | Low |
| Add wasm32 target | `Cargo.toml` with `[target.'cfg(target_arch = "wasm32")'.dependencies]` | Low |

### Tier 2: Component Model (enables modularity)

| Change | What | Effort |
|---|---|---|
| Define WIT interfaces | `scoring.wit`, `routing.wit`, `gating.wit`, `learning.wit` | Medium |
| Split crates along WIT boundaries | Each WIT world = one crate | Medium |
| Add `cargo component build` | Build scripts for WASM components | Low |
| Component composition pipeline | `wac plug` commands in Makefile | Low |

### Tier 3: Distribution (enables collective learning)

| Change | What | Effort |
|---|---|---|
| CRDT types for learning state | G-Counter, LWW-Register, G-Set per data type | High |
| Merkle tree for reconciliation | Hash tree over CRDT state | High |
| Sync protocol | HTTP endpoint for Merkle-CRDT exchange | Medium |
| A2A Agent Card | Publish roko's capabilities | Low |
| Brain export/import | Serialize/deserialize learned state | Medium |

### Tier 4: Protocol (enables integration)

| Change | What | Effort |
|---|---|---|
| `roko acp` subcommand | ACP agent over stdio (from ide.md) | Medium |
| `roko serve --mcp` | MCP tool server | Low |
| A2A task handler | Accept delegated tasks from other agents | Medium |
| A2A task delegation | Delegate to other agents (browser→server) | Medium |

---

## 15. Implementation Roadmap

### Phase 1: Platform Abstraction (1-2 months)
Make roko-core compile to WASM. No new features, just abstractions.
- Define `StateStore`, `LlmClient`, `GateRunner`, `EventSink` traits
- Implement native versions (wrapping existing code)
- Feature-gate subprocess and filesystem code
- Verify: `cargo build --target wasm32-unknown-unknown -p roko-core`

### Phase 2: Browser Prototype (1 month)
Minimal roko-wasm that runs CascadeRouter + PromptAssembler in a browser.
- Implement `BrowserStateStore` (IndexedDB)
- Implement `BrowserLlmClient` (reqwest with Fetch)
- Build a simple chat UI with wasm-bindgen
- Verify: Open in browser, send prompt, get routed response

### Phase 3: ACP + A2A Integration (1-2 months)
From ide.md: `roko acp` subcommand + A2A Agent Card.
- Implement ACP over stdio
- Publish A2A Agent Card at `.well-known/agent-card.json`
- Browser roko delegates gates to server roko via A2A

### Phase 4: WASM Component Model (2-3 months)
Split agent into composable WASM components.
- Define WIT interfaces for scoring, routing, gating, learning
- Build component pipeline with `cargo component` and `wac`
- Verify: Swap routing component without rebuilding agent

### Phase 5: Distributed Learning (2-3 months)
CRDT-based shared learning across instances.
- Implement CRDT types for each learning data structure
- Build Merkle tree reconciliation
- Brain export/import commands
- Privacy-preserving aggregate sync mode

### Phase 6: Edge Deployment (1-2 months)
Run roko on Cloudflare Workers / Fermyon Spin.
- Implement `EdgeStateStore` (Cloudflare KV / Spin KV)
- Deploy routing component as edge function
- Measure cold start, memory, latency

---

## 16. Research Sources

### WASM Compilation
- [wasm32-wasip2 Rust book](https://doc.rust-lang.org/nightly/rustc/platform-support/wasm32-wasip2.html)
- [WASI 0.2 Launched](https://bytecodealliance.org/articles/WASI-0.2)
- [WASI Interfaces](https://wasi.dev/interfaces)
- [Looking Ahead to WASIp3](https://www.fermyon.com/blog/looking-ahead-to-wasip3)
- [WASM Component Model](https://component-model.bytecodealliance.org/)
- [tokio-with-wasm](https://github.com/cunarist/tokio-with-wasm)
- [reqwest WASM Streaming](https://parsec.cloud/en/how-the-reqwest-http-client-streams-responses-in-a-web-context/)
- [tiktoken-wasm](https://lib.rs/crates/tiktoken-wasm)
- [Porting Tokenizers to WASM](https://blog.mithrilsecurity.io/porting-tokenizers-to-wasm/)
- [Shrinking WASM Size](https://rustwasm.github.io/book/reference/code-size.html)
- [WASM Threads](https://web.dev/articles/webassembly-threads)

### Browser Agents
- [Bolt.new](https://github.com/stackblitz/bolt.new)
- [WebContainers](https://webcontainers.io/)
- [Mozilla AI WASM Agents](https://blog.mozilla.ai/wasm-agents-ai-agents-running-in-your-browser/)
- [Mozilla 3W Stack](https://blog.mozilla.ai/3w-for-in-browser-ai-webllm-wasm-webworkers/)
- [Notion WASM SQLite](https://www.notion.com/blog/how-we-sped-up-notion-in-the-browser-with-wasm-sqlite)
- [SQLite WASM + OPFS](https://www.powersync.com/blog/sqlite-persistence-on-the-web)

### Edge/Server WASM
- [Cloudflare Workers Rust](https://developers.cloudflare.com/workers/languages/rust/)
- [Fermyon Spin](https://www.fermyon.com/spin)
- [Wasmtime](https://github.com/bytecodealliance/wasmtime)
- [WasmEdge](https://wasmedge.org/)
- [WASI-NN Edge AI](https://dev.to/vaib/revolutionizing-edge-ai-deploying-models-with-webassembly-and-wasi-nn-d0h)

### Composability
- [WASM Component Composition](https://component-model.bytecodealliance.org/composing-and-distributing/composing.html)
- [Composable WASM Microservices in Rust](https://www.essamamdani.com/blog/the-post-container-era-building-composable-wasm-microservices-with-rust-273326)
- [Agentor: Secure Agents in Rust](https://www.xcapit.com/en/blog/from-openclaw-to-agentor-building-secure-ai-agents-in-rust)
- [SymmACP Composable Agents](https://smallcultfollowing.com/babysteps/blog/2025/10/08/symmacp/)

### Protocols
- [A2A Protocol](https://a2a-protocol.org/latest/)
- [MCP Protocol](https://modelcontextprotocol.io/)
- [ACP Protocol](https://agentclientprotocol.com)
- [Agent Protocol Stack 2026](https://subhadipmitra.com/blog/2026/agent-protocol-stack/)
- [Agentic AI Foundation (Linux Foundation)](https://www.linuxfoundation.org/press/linux-foundation-announces-the-formation-of-the-agentic-ai-foundation)

### Distributed Learning
- [Merkle-CRDTs](https://arxiv.org/pdf/2004.00107)
- [go-ds-crdt (IPFS)](https://github.com/ipfs/go-ds-crdt)
- [Confluent: Event-Driven Multi-Agent Patterns](https://www.confluent.io/blog/event-driven-multi-agent-systems/)
- [Agentic AI with Kafka + A2A + MCP](https://www.kai-waehner.de/blog/2025/05/26/agentic-ai-with-the-agent2agent-protocol-a2a-and-mcp-using-apache-kafka-as-event-broker/)

### Portable State
- [Agent File (.af) by Letta](https://github.com/letta-ai/agent-file)
- [Open Agent Specification (Oracle)](https://arxiv.org/html/2510.04173v1)
- [Agent Skills Specification](https://agentskills.io/home)

### Self-Modification
- [HyperAgents (Meta, ICLR 2026)](https://arxiv.org/abs/2603.19461)
- [Darwin Godel Machine (Sakana AI)](https://arxiv.org/abs/2505.22954)
- [Karpathy AutoResearch](https://github.com/karpathy/autoresearch)
- [A-Evolve](https://github.com/A-EVO-Lab/a-evolve)

### Marketplace
- [Agensi Marketplace](https://www.agensi.io/)
- [Agent Skills (Anthropic)](https://www.anthropic.com/engineering/equipping-agents-for-the-real-world-with-agent-skills)

### Novel Use Cases
- [CellVoyager (Nature Methods 2026)](https://www.nature.com/articles/s41592-026-03029-6)
- [GitHub Agentic Workflows](https://github.blog/ai-and-ml/automate-repository-tasks-with-github-agentic-workflows/)
- [Cursor 3 Agent-First Interface](https://cursor.com/blog/cursor-3)
- [Google Antigravity](https://developers.googleblog.com/build-with-google-antigravity-our-new-agentic-development-platform/)
- [Runcell Notebook Agent](https://www.runcell.dev)
