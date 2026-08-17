# tiagent: Rust Workspace Layout and Crate Dependencies

This document defines the crate structure for the tiagent workspace --- which crates exist,
what each one does, what it depends on, and why the boundaries are drawn where they are. It
is written for a reader encountering the project for the first time, with no assumed context
beyond basic Rust knowledge (workspaces, crates, features, traits).

If you have not read the preceding documents:

- **01-vision-and-overview.md** explains what tiagent is: an open-source Rust harness for
  building coding agents, with optional Celestia DA integration for on-chain persistence.
- **02-architecture.md** explains the core abstractions: one noun (Signal), six verb traits
  (Substrate, Scorer, Gate, Router, Composer, Policy), and a universal loop
  (query, score, route, compose, act, verify, write, react).

This document covers the physical code organization that implements those abstractions.

---

## Table of Contents

1. [Design Principles](#1-design-principles)
2. [Workspace Layout](#2-workspace-layout)
3. [Crate Catalog](#3-crate-catalog)
4. [Dependency Graph](#4-dependency-graph)
5. [Feature Flags](#5-feature-flags)
6. [Key External Dependencies](#6-key-external-dependencies)
7. [Comparison with Prior Art](#7-comparison-with-prior-art)
8. [Decision Log](#8-decision-log)

---

## 1. Design Principles

Five rules govern how crates are organized. Every crate boundary decision should be
evaluated against these rules.

### 1.1 Minimal crate count

**Target: 12--15 crates, including tool server binaries.**

Two prior projects inform this constraint:

- **polkagent** (a Rust agent framework for Polkadot) grew to 90 crates. The result was
  slow compilation, difficult dependency management, and a workspace that intimidated new
  contributors. Adding a feature meant touching 5--10 Cargo.toml files.
- **roko** (a Rust toolkit for self-building agents) has 18 crates. This is manageable but
  some crates overlap in responsibility, and a few contain fewer than 500 lines of code that
  could have been modules within a larger crate.

The test: if you cannot explain a crate's purpose in one sentence, it should be merged into
another crate. If a crate has fewer than 500 lines of production code, it should probably be
a module within a neighbor.

### 1.2 Clear dependency flow

Dependencies flow in one direction: **kernel -> domain -> application -> binary**.

- Kernel crates (`tiagent-core`) depend only on `std`, `serde`, and other foundational
  crates. They never depend on domain crates like `tiagent-celestia` or `tiagent-agent`.
- Domain crates (`tiagent-agent`, `tiagent-gate`, etc.) depend on `tiagent-core` and on
  third-party libraries. They never depend on application crates like `tiagent-cli`.
- Application crates (`tiagent-cli`, `tiagent-serve`) depend on domain crates. They are the
  leaves of the dependency tree.
- Circular dependencies are forbidden. If crate A depends on crate B, crate B must not
  depend on crate A, directly or transitively.

### 1.3 Feature-gated optionality

Heavy dependencies should be behind Cargo feature flags so that users who do not need them
do not pay the compilation cost.

Examples of heavy dependencies that must be feature-gated:

| Dependency | Feature flag | Why gated |
|------------|-------------|-----------|
| `celestia-rpc` | `celestia-rpc` | Not needed for local-only development |
| `lumina-node` | `light-node` | Embeds a full Celestia light node; large binary size |
| LLM provider SDKs | `claude-api`, `openai`, `ollama` | Each adds HTTP client + serialization code |
| `axum` + tower stack | `serve` | Not needed for CLI-only usage |
| `ratatui` | `tui` | Not needed for headless/server usage |

The default feature set should include enough to run a basic agent with local storage and
one LLM backend. Everything else is opt-in.

### 1.4 Celestia-optional builds

**The core harness compiles and works without any Celestia dependencies.**

The following crates form the core harness and must never depend on Celestia types,
RPC clients, or DA-layer concepts: `tiagent-core`, `tiagent-agent`, `tiagent-gate`,
`tiagent-compose`, `tiagent-orchestrator`, `tiagent-learn`, `tiagent-store` (default),
`tiagent-tools`, `tiagent-cli` (default), `tiagent-runtime`.

`tiagent-celestia` is an optional leaf crate behind a feature flag. A developer can
`cargo build -p tiagent-cli` and get a fully functional coding agent harness --- local
storage, LLM dispatch, plan execution, gates, learning --- with zero Celestia code compiled.
Celestia integration is enabled explicitly via `--features celestia`.

### 1.5 Testable in isolation

Every crate must have standalone unit tests that pass without starting external services
(no Celestia node, no LLM API, no HTTP server). Integration tests that require external
services are allowed but must be behind `#[cfg(feature = "integration")]` or in a separate
test binary.

This means:

- Traits must have mock implementations shipped in the same crate (or in `tiagent-core`).
- Network-dependent code must accept injected clients, not construct them internally.
- File-system-dependent code must accept a root path, not hardcode one.

---

## 2. Workspace Layout

```
tiagent/
├── Cargo.toml                          # Workspace root, shared dependencies
├── crates/
│   ├── tiagent-core/                   # Signal, 6 verb traits, types, errors, config
│   ├── tiagent-agent/                  # LLM dispatch, tool loop, backends, safety
│   ├── tiagent-celestia/               # Optional: Celestia DA integration (feature-gated)
│   ├── tiagent-gate/                   # Gate pipeline (7 rungs), adaptive thresholds
│   ├── tiagent-compose/                # Prompt assembly, templates, context bidding
│   ├── tiagent-orchestrator/           # Plan DAG, task execution, parallel dispatch
│   ├── tiagent-learn/                  # Episodes, cascade router, efficiency, playbooks
│   ├── tiagent-store/                  # Local substrate (JSONL/SQLite), GC, layout
│   ├── tiagent-tools/                  # Built-in tools, MCP client, tool registry
│   ├── tiagent-serve/                  # HTTP API (axum), SSE, WebSocket
│   ├── tiagent-runtime/                # Process supervision, event bus, cancellation
│   └── tiagent-cli/                    # CLI binary (clap), all subcommands
├── tools/
│   ├── tiagent-mcp-celestia/           # MCP server: Celestia-specific developer tools
│   └── tiagent-mcp-code/              # MCP server: code intelligence tools
├── docs/                               # Design documents (this file lives here)
├── tests/                              # Workspace-level integration tests
└── examples/                           # Runnable example configurations
```

**Total: 14 crates** (12 library crates + 2 MCP tool server binaries).

The `tools/` directory is separate from `crates/` because MCP tool servers are standalone
binaries that communicate over stdio or HTTP. They depend on `tiagent-core` for types but
do not participate in the core dependency graph.

---

## 3. Crate Catalog

### 3.1 tiagent-core

**Purpose**: The kernel crate. Defines the Signal data type, the six verb traits, shared
error types, configuration structures, and foundational utilities. Every other crate in
the workspace depends on `tiagent-core`.

**Key types**:

| Type | Kind | Description |
|------|------|-------------|
| `Signal` | struct | The universal data atom: content-addressed, typed, scored |
| `SignalKind` | enum | Categorizes signals (Prompt, Response, ToolCall, Episode, etc.) |
| `Hash` | newtype | SHA-256 content address wrapping `[u8; 32]` |
| `Namespace` | struct | Logical namespace identifier (version + data type + agent/group) |
| `SignalFilter` | struct | Query predicate for filtering signals by kind, time, metadata |
| `Substrate` | trait | Persist and retrieve signals |
| `Scorer` | trait | Assign numeric scores to signals |
| `Gate` | trait | Validate signals against quality criteria |
| `Router` | trait | Select model, backend, and execution path |
| `Composer` | trait | Assemble prompts from templates, context, and tools |
| `Policy` | trait | Authorize actions based on safety rules and role permissions |
| `GateResult` | struct | Output of a gate check (passed, summary, details, duration) |
| `Route` | struct | Routing decision (model, backend, tools, budget) |
| `TiagentError` | enum | Unified error type for the workspace |
| `Config` | struct | Top-level configuration (parsed from `tiagent.toml`) |

**tiagent crate dependencies**: None. This is the root of the dependency tree.

**External dependencies**: `serde`, `serde_json`, `sha2`, `thiserror`, `async-trait`,
`chrono`, `uuid`.

**Feature flags**: None. This crate is always fully compiled.

---

### 3.2 tiagent-agent

**Purpose**: LLM dispatch and the agent tool loop. Contains backend adapters for multiple
LLM providers, the tool call execution loop (send prompt, receive response, execute tool
calls, repeat), and safety controls for tool authorization.

**Key types**:

| Type | Kind | Description |
|------|------|-------------|
| `AgentDispatcher` | struct | Top-level dispatcher: accepts a prompt + route, returns a result |
| `Backend` | trait | Interface for LLM providers (send messages, receive responses) |
| `ClaudeApiBackend` | struct | Backend for Anthropic's Claude API |
| `OpenAiCompatBackend` | struct | Backend for any OpenAI-compatible API (GPT, Together, Groq) |
| `OllamaBackend` | struct | Backend for local Ollama inference |
| `CliBackend` | struct | Backend that shells out to Claude CLI or Codex CLI |
| `ToolLoop` | struct | Executes the tool call cycle until the model stops calling tools |
| `AgentContract` | struct | Safety contract defining allowed tools, rate limits, role permissions |
| `BackendPool` | struct | Manages a pool of backend instances with health checks |

**tiagent crate dependencies**: `tiagent-core`

**External dependencies**: `reqwest` (HTTP client for API calls), `tokio` (async runtime),
`tracing` (structured logging), `serde_json`.

**Feature flags**:

| Flag | What it enables |
|------|----------------|
| `claude-api` | `ClaudeApiBackend` (requires `reqwest`) |
| `openai` | `OpenAiCompatBackend` (requires `reqwest`) |
| `ollama` | `OllamaBackend` (requires `reqwest`) |
| `cli-backends` | `CliBackend` (requires `tokio::process`) |

Default: `claude-api` + `openai`.

---

### 3.3 tiagent-celestia

**This crate is optional.** The core harness works without it. Enable via `--features celestia`
on `tiagent-cli`, or add `celestia = ["dep:tiagent-celestia"]` to the workspace `[features]`
in the root `Cargo.toml`.

**Purpose**: Celestia DA layer integration. Implements the `Substrate` trait for Celestia
blob storage, provides namespace management utilities, fee estimation, blob querying, and
optionally embeds a Celestia light node for direct DA access.

**Key types**:

| Type | Kind | Description |
|------|------|-------------|
| `CelestiaSubstrate` | struct | `Substrate` implementation that reads/writes blobs to Celestia DA |
| `NamespaceManager` | struct | Creates, organizes, and queries namespaces following the `tiagent/v1/...` schema |
| `BlobBuilder` | struct | Constructs Celestia blobs from signals with proper encoding and commitment |
| `FeeEstimator` | struct | Estimates gas and fee costs before blob submission |
| `LightNodeClient` | struct | Interface to an embedded or external Celestia light node |
| `DaRef` | struct | Reference to a blob on the DA layer (height, namespace, commitment) |
| `BlobQuery` | struct | Query parameters for retrieving historical blobs |

**tiagent crate dependencies**: `tiagent-core`

**External dependencies**: `celestia-types` (blob/namespace types), `celestia-rpc` (RPC
client), `nmt-rs` (Namespaced Merkle Tree proofs), `tokio`, `tracing`.

**Feature flags**:

| Flag | What it enables |
|------|----------------|
| `celestia-rpc` | RPC client for remote Celestia node interaction |
| `light-node` | Embeds `lumina-node` for direct light node DA access |
| `mocha` | Mocha testnet configuration defaults |
| `arabica` | Arabica testnet configuration defaults |

Default: `celestia-rpc`. The `light-node` feature is opt-in due to binary size impact.

---

### 3.4 tiagent-gate

**Purpose**: The gate pipeline --- a sequence of validation checks (called "rungs") that
verify agent outputs before they are accepted. Gates check compilation, test passage,
linting, diff quality, semantic correctness, and (optionally) human approval. Gate
thresholds adapt over time using exponential moving averages.

**Key types**:

| Type | Kind | Description |
|------|------|-------------|
| `GatePipeline` | struct | Runs an ordered sequence of gates, short-circuiting on failure |
| `CompileGate` | struct | Rung 1: checks whether generated code compiles |
| `TestGate` | struct | Rung 2: runs existing tests and checks for regressions |
| `LintGate` | struct | Rung 3: runs linters (clippy, eslint, etc.) |
| `DiffGate` | struct | Rung 4: validates diff size, no accidental deletions, no secrets |
| `SemanticGate` | struct | Rung 5: LLM-based review of output against intent |
| `HumanGate` | struct | Rung 6: manual approval for high-risk changes |
| `AdaptiveThreshold` | struct | EMA-based threshold that adjusts per gate per task type |

**tiagent crate dependencies**: `tiagent-core`

**External dependencies**: `tokio`, `tracing`, `serde_json`.

**Feature flags**: None. All gate types are cheap to compile.

---

### 3.5 tiagent-compose

**Purpose**: Prompt assembly. Takes a task description, context signals, tool definitions,
and template configuration, and produces the final system prompt + user message that gets
sent to an LLM. Implements the `Composer` trait. Includes a context bidding system where
different context sources (knowledge store, task history, research) compete for limited
context window space.

**Key types**:

| Type | Kind | Description |
|------|------|-------------|
| `SystemPromptBuilder` | struct | Multi-layer prompt builder (role, domain, tools, history, constraints) |
| `PromptTemplate` | struct | A named, parameterized prompt template |
| `ContextBidder` | trait | Interface for context sources that bid for inclusion in the prompt |
| `TaskBidder` | struct | Bids task-specific context (prior attempts, related tasks) |
| `KnowledgeBidder` | struct | Bids relevant knowledge from the local or DA-backed store |
| `ResearchBidder` | struct | Bids research artifacts relevant to the current task |
| `ContextBudget` | struct | Manages the token budget and allocates space to bidders |

**tiagent crate dependencies**: `tiagent-core`

**External dependencies**: `serde`, `serde_json`, `handlebars` (template rendering),
`tiktoken-rs` (token counting).

**Feature flags**: None.

---

### 3.6 tiagent-orchestrator

**Purpose**: Plan execution. Parses task plans (TOML files describing a DAG of tasks),
resolves dependencies, dispatches tasks to agents in parallel where possible, handles
retries and gate failures, and persists execution state for resume-after-interruption.

**Key types**:

| Type | Kind | Description |
|------|------|-------------|
| `PlanRunner` | struct | Top-level plan executor: loads plan, runs tasks, persists state |
| `TaskDag` | struct | Directed acyclic graph of tasks with dependency edges |
| `TaskNode` | struct | A single task in the DAG: description, dependencies, gate config |
| `ExecutorState` | struct | Serializable snapshot of execution progress (for resume) |
| `DispatchResult` | enum | Outcome of dispatching a task: success, gate failure, error |
| `ReplanStrategy` | struct | How to revise a plan when a gate fails (retry, decompose, escalate) |

**tiagent crate dependencies**: `tiagent-core`, `tiagent-agent`, `tiagent-gate`,
`tiagent-compose`, `tiagent-learn`

**External dependencies**: `tokio` (async parallelism), `toml` (plan parsing), `tracing`,
`serde_json`.

**Feature flags**: None.

---

### 3.7 tiagent-learn

**Purpose**: The learning system. Records episodes (structured traces of agent executions),
maintains cascade router weights for model selection, runs prompt experiments (A/B testing),
tracks efficiency metrics, and extracts playbooks from successful episodes. Learning
artifacts are stored locally by default; they can optionally be published to the DA layer
for cross-agent sharing when the `celestia` feature is enabled.

**Key types**:

| Type | Kind | Description |
|------|------|-------------|
| `EpisodeLogger` | struct | Records agent turns, tool calls, and outcomes into episodes |
| `Episode` | struct | A complete structured trace of one agent execution |
| `CascadeRouter` | struct | `Router` implementation that selects models based on learned weights |
| `ExperimentStore` | struct | Manages A/B experiments on prompts, models, and strategies |
| `EfficiencyTracker` | struct | Records per-turn cost, latency, and token usage |
| `PlaybookExtractor` | struct | Identifies reusable patterns from high-scoring episodes |
| `Playbook` | struct | A reusable strategy template extracted from successful runs |
| `LearningState` | struct | Serializable snapshot of all learning data (for persistence + DA) |

**tiagent crate dependencies**: `tiagent-core`

**External dependencies**: `serde`, `serde_json`, `chrono`, `rand` (for experiment
randomization), `tracing`.

**Feature flags**: None.

---

### 3.8 tiagent-store

**Purpose**: The default storage substrate. This is what all developers use. Implements the
`Substrate` trait for local filesystem storage (JSONL files, optionally SQLite), provides
garbage collection, and manages the data directory layout (`.tiagent/`). No blockchain, no
network --- just files on disk. Works out of the box.

When the optional `hybrid` feature is enabled, this crate also provides `HybridSubstrate`,
which combines local and Celestia DA storage. But the default build uses only local storage.

**Key types**:

| Type | Kind | Description |
|------|------|-------------|
| `FileSubstrate` | struct | `Substrate` implementation backed by JSONL files on disk |
| `SqliteSubstrate` | struct | `Substrate` implementation backed by SQLite |
| `HybridSubstrate` | struct | Combines local + DA substrates with configurable sync policy |
| `DataLayout` | struct | Manages `.tiagent/` directory structure and file paths |
| `GarbageCollector` | struct | Removes expired or low-value local signals based on policy |

**tiagent crate dependencies**: `tiagent-core`, `tiagent-celestia` (optional, for
`HybridSubstrate`)

**External dependencies**: `serde_json`, `tokio` (async file I/O), `tracing`.

**Feature flags**:

| Flag | What it enables |
|------|----------------|
| `sqlite` | `SqliteSubstrate` (requires `rusqlite`) |
| `hybrid` | `HybridSubstrate` (requires `tiagent-celestia`; opt-in) |

Default: none. Local JSONL storage works with zero feature flags. The `hybrid` feature is
opt-in for users who want combined local + Celestia DA storage.

---

### 3.9 tiagent-tools

**Purpose**: Tool definitions and the MCP integration layer. Provides the built-in tool
registry, typed tool schemas, the MCP client (for connecting to external MCP servers), and
the MCP server scaffolding (for exposing tiagent tools to external MCP clients).
Celestia-specific tools (blob submission, namespace querying, fee estimation) live in the
separate `tiagent-mcp-celestia` binary, not here.

**Key types**:

| Type | Kind | Description |
|------|------|-------------|
| `ToolRegistry` | struct | Registry of available tools with schema validation |
| `ToolDefinition` | struct | A tool's name, description, JSON Schema, and risk classification |
| `ToolResult` | struct | The output of a tool invocation (success/error + structured data) |
| `McpClient` | struct | Connects to external MCP servers and imports their tool definitions |
| `McpServer` | struct | Exposes tiagent tools as an MCP server (stdio or HTTP transport) |
| `RiskLevel` | enum | Tool risk classification: `ReadOnly`, `Write`, `Destructive` |

**tiagent crate dependencies**: `tiagent-core`

**External dependencies**: `serde_json`, `schemars` (JSON Schema generation), `tokio`,
`tracing`.

**Feature flags**:

| Flag | What it enables |
|------|----------------|
| `mcp-client` | MCP client for connecting to external tool servers |
| `mcp-server` | MCP server for exposing tools to external clients |

Default: `mcp-client` + `mcp-server`.

---

### 3.10 tiagent-serve

**Purpose**: HTTP API server. Exposes tiagent's capabilities over REST, SSE (server-sent
events), and WebSocket endpoints. Built on `axum`. Provides endpoints for submitting tasks,
querying state, streaming agent output, and managing configuration. This crate is optional
--- tiagent can run as a pure CLI tool without the HTTP layer.

**Key types**:

| Type | Kind | Description |
|------|------|-------------|
| `Server` | struct | The axum HTTP server with route configuration |
| `AppState` | struct | Shared application state passed to route handlers |
| `SseStream` | struct | Server-sent event stream for real-time agent output |
| `WsHandler` | struct | WebSocket handler for bidirectional agent communication |

**tiagent crate dependencies**: `tiagent-core`, `tiagent-agent`, `tiagent-orchestrator`,
`tiagent-learn`, `tiagent-store`

**External dependencies**: `axum`, `tower`, `tower-http`, `tokio`, `tracing`,
`serde_json`.

**Feature flags**: None. This entire crate is opt-in (only compiled when the `serve`
feature is enabled on `tiagent-cli` or when built independently).

---

### 3.11 tiagent-runtime

**Purpose**: Process lifecycle management and infrastructure. Provides a process supervisor
(tracks spawned agent processes, handles graceful shutdown), an in-process event bus (for
decoupled component communication), and cancellation token management.

**Key types**:

| Type | Kind | Description |
|------|------|-------------|
| `ProcessSupervisor` | struct | Tracks child processes, enforces timeouts, handles shutdown |
| `EventBus` | struct | Typed publish/subscribe event system for internal communication |
| `CancellationToken` | struct | Cooperative cancellation for async task trees |
| `ShutdownGuard` | struct | RAII guard that ensures cleanup runs on drop |

**tiagent crate dependencies**: `tiagent-core`

**External dependencies**: `tokio`, `tracing`.

**Feature flags**: None.

---

### 3.12 tiagent-cli

**Purpose**: The CLI binary. This is the main entry point for tiagent. It parses command-line
arguments (via `clap`), loads configuration, initializes the runtime, and dispatches to
subcommands. It depends on nearly every other crate, assembling them into a usable
application.

**Key subcommands**:

| Command | What it does |
|---------|-------------|
| `tiagent init` | Create `.tiagent/` directory and `tiagent.toml` config |
| `tiagent run "<prompt>"` | Execute a single prompt through the universal loop |
| `tiagent plan run <dir>` | Execute a task plan (the main orchestration loop) |
| `tiagent plan run <dir> --resume <snapshot>` | Resume an interrupted plan execution |
| `tiagent status` | Query and display current agent state |
| `tiagent doctor` | Diagnose workspace health (config, backends, connectivity) |
| `tiagent serve` | Start the HTTP API server |
| `tiagent config show/set/validate` | Configuration management |
| `tiagent learn show/tune` | Inspect and tune learning state |

**tiagent crate dependencies**: `tiagent-core`, `tiagent-agent`, `tiagent-gate`,
`tiagent-compose`, `tiagent-orchestrator`, `tiagent-learn`, `tiagent-store`,
`tiagent-tools`, `tiagent-runtime`, `tiagent-serve` (optional),
`tiagent-celestia` (optional, behind `celestia` feature)

**External dependencies**: `clap` (argument parsing), `tokio` (async runtime), `tracing`,
`tracing-subscriber`, `toml`, `serde_json`, `directories` (platform-specific paths).

**Feature flags**:

| Flag | What it enables |
|------|----------------|
| `serve` | HTTP server subcommand (pulls in `tiagent-serve` + `axum`) |
| `tui` | Terminal UI dashboard (pulls in `ratatui`) |
| `celestia` | Celestia DA integration (pulls in `tiagent-celestia`) |
| `light-node` | Embedded Celestia light node (pulls in `lumina-node` via `tiagent-celestia`) |

Default: `serve`. Note: `tiagent-celestia` is NOT a default dependency.

---

### 3.13 tiagent-mcp-celestia (in tools/)

**Purpose**: A standalone MCP tool server binary that exposes Celestia-specific developer
tools. Can be connected to any MCP client (Claude Desktop, Cursor, VS Code, or tiagent
itself). Provides tools for blob submission, namespace management, fee estimation, block
queries, and light node status.

**Key tools exposed**:

| Tool | Description |
|------|-------------|
| `celestia_submit_blob` | Submit a blob to a Celestia namespace |
| `celestia_get_blob` | Retrieve a blob by height and namespace |
| `celestia_list_namespaces` | List namespaces matching a prefix |
| `celestia_estimate_fee` | Estimate gas cost for a blob of given size |
| `celestia_node_status` | Check light node sync status and connectivity |
| `celestia_query_blobs` | Query blobs by namespace and height range |

**tiagent crate dependencies**: `tiagent-core`, `tiagent-celestia`, `tiagent-tools`

**External dependencies**: `clap`, `tokio`, `serde_json`, `tracing`.

---

### 3.14 tiagent-mcp-code (in tools/)

**Purpose**: A standalone MCP tool server binary for code intelligence. Provides tools for
searching code, analyzing dependencies, understanding project structure, and navigating
large codebases. Language-agnostic where possible, with deeper support for Rust.

**Key tools exposed**:

| Tool | Description |
|------|-------------|
| `code_search` | Search code by pattern (regex) across a workspace |
| `code_symbols` | List symbols (functions, structs, traits) in a file or directory |
| `code_dependencies` | Analyze dependency relationships between modules/crates |
| `code_references` | Find all references to a symbol |
| `code_structure` | Summarize the structure of a file or module |

**tiagent crate dependencies**: `tiagent-core`, `tiagent-tools`

**External dependencies**: `clap`, `tokio`, `tree-sitter` (parsing), `serde_json`,
`tracing`.

---

## 4. Dependency Graph

The following ASCII diagram shows which crates depend on which. Arrows point from dependent
to dependency (A --> B means "A depends on B").

```
                         ┌──────────────────┐
                         │   tiagent-core   │
                         │                  │
                         │  Signal, traits, │
                         │  types, errors   │
                         └────────┬─────────┘
                                  │
              ┌───────────────────┼───────────────────────────────┐
              │                   │                               │
              │         ┌────────┼──────────┐                    │
              ▼         ▼        ▼          ▼                    ▼
     ┌────────────┐ ┌────────┐ ┌──────┐ ┌────────┐      ┌──────────────┐
     │  tiagent-  │ │tiagent-│ │tiag- │ │tiagent-│      │   tiagent-   │
     │   agent    │ │ tools  │ │ent-  │ │compose │      │   runtime    │
     │            │ │        │ │gate  │ │        │      │              │
     │ LLM       │ │Registry│ │      │ │Prompt  │      │ Supervisor,  │
     │ dispatch   │ │MCP,    │ │Valid- │ │build,  │      │ event bus    │
     └───┬────────┘ │schemas │ │ation │ │context │      └──────────────┘
         │          └──┬─────┘ └──┬───┘ │bidding │
         │             │          │     └───┬────┘
         │             │          │         │
         │             │          │         │       ┌──────────────┐
     ┌───▼─────────┐   │          │         │       │   tiagent-   │
     │  tiagent-   │   │          │         │       │    learn     │
     │   store     │   │          │         │       │              │
     │             │   │          │         │       │ Episodes,    │
     │ Local FS    │◄──┼──────────┼─────────┤       │ router, A/B  │
     │ (default)   │   │          │         │       └──────┬───────┘
     └─────────────┘   │          │         │              │
                       │          │         │              │
            │    ┌─────▼──────────▼─────────▼──────────────▼───┐
            │    │           tiagent-orchestrator               │
            │    │                                              │
            │    │  Plan DAG, task dispatch, state persistence  │
            │    └──────────────────┬───────────────────────────┘
            │                      │
            │              ┌───────▼──────────┐
            │              │   tiagent-serve   │
            │              │                   │
            │              │  HTTP API, SSE,   │
            │              │  WebSocket        │
            │              └───────┬───────────┘
            │                      │
            ├──────────────────────┤
            │                      │
     ┌──────▼──────────────────────▼────┐
     │           tiagent-cli            │
     │                                  │
     │  CLI binary: all subcommands,    │
     │  config loading, runtime init    │
     └──────────────────────────────────┘

     ════════════════════════════════════════════════════════
     Everything above is the CORE HARNESS.
     No Celestia dependencies. Builds with: cargo build -p tiagent-cli
     ════════════════════════════════════════════════════════

     OPTIONAL: Celestia integration (--features celestia)

     ┌──────────────────────────┐
     │     tiagent-celestia     │  ◄── LEAF: nothing in the core depends on this
     │                          │
     │  DA substrate,           │
     │  namespaces, light node  │
     │                          │
     │  Depends on:             │
     │   - tiagent-core         │
     │                          │
     │  Depended on by:         │
     │   - tiagent-store        │
     │     (optional "hybrid"   │
     │      feature only)       │
     │   - tiagent-mcp-celestia │
     └──────────────────────────┘


     MCP Tool Servers (standalone binaries):

     ┌─────────────────────┐     ┌─────────────────────┐
     │ tiagent-mcp-celestia│     │  tiagent-mcp-code   │
     │                     │     │                     │
     │ Depends on:         │     │ Depends on:         │
     │  - tiagent-core     │     │  - tiagent-core     │
     │  - tiagent-celestia │     │  - tiagent-tools    │
     │  - tiagent-tools    │     │                     │
     │  (requires celestia │     │  (no Celestia deps) │
     │   feature)          │     │                     │
     └─────────────────────┘     └─────────────────────┘
```

**Key point**: `tiagent-celestia` is a leaf dependency. No core crate depends on it.
The only optional reference is from `tiagent-store`'s `hybrid` feature flag, which is
not enabled by default. The core harness is a complete, self-contained coding agent
toolkit.

### Dependency summary table

| Crate | Depends on (tiagent crates) | Core harness? |
|-------|---------------------------|---------------|
| `tiagent-core` | (none) | Yes |
| `tiagent-agent` | `core` | Yes |
| `tiagent-gate` | `core` | Yes |
| `tiagent-compose` | `core` | Yes |
| `tiagent-learn` | `core` | Yes |
| `tiagent-runtime` | `core` | Yes |
| `tiagent-tools` | `core` | Yes |
| `tiagent-store` | `core`, `celestia` (optional, via `hybrid` feature) | Yes (default: local-only) |
| `tiagent-orchestrator` | `core`, `agent`, `gate`, `compose`, `learn` | Yes |
| `tiagent-serve` | `core`, `agent`, `orchestrator`, `learn`, `store` | Yes (feature-gated) |
| `tiagent-cli` | all core crates above | Yes |
| `tiagent-celestia` | `core` | **No** --- optional leaf |
| `tiagent-mcp-celestia` | `core`, `celestia`, `tools` | **No** --- requires celestia |
| `tiagent-mcp-code` | `core`, `tools` | Yes |

### Maximum dependency depth

The longest chain is 4 levels:

```
tiagent-core (0)
  -> tiagent-agent (1)
    -> tiagent-orchestrator (2)
      -> tiagent-serve (3)
        -> tiagent-cli (4)
```

This is shallow enough that changes to `tiagent-core` do not trigger a cascade of slow
recompilations through many intermediate crates.

---

## 5. Feature Flags

### Workspace-level features (set on tiagent-cli)

| Feature | Enables | Default |
|---------|---------|---------|
| `serve` | HTTP API server (`tiagent-serve`) | Yes |
| `tui` | Terminal dashboard (`ratatui`) | No |
| `celestia` | Celestia DA integration (`tiagent-celestia`) | No |
| `light-node` | Embedded Celestia light node (implies `celestia`) | No |
| `sqlite` | SQLite-backed local storage | No |
| `full` | All optional features | No |

### Building without Celestia

The default build includes zero Celestia dependencies. This is the recommended starting
point for most developers.

```bash
# Build core harness only (no Celestia deps)
cargo build -p tiagent-cli

# Build with Celestia integration
cargo build -p tiagent-cli --features celestia

# Build with Celestia + embedded light node
cargo build -p tiagent-cli --features "celestia,light-node"
```

The first command gives you a fully functional coding agent: LLM dispatch, plan execution,
gate validation, local storage, learning, MCP tools. No Celestia node needed, no DA layer
configuration, no blockchain dependencies in your compile graph.

### Building with specific features

```bash
# Minimal build (CLI only, no server, no TUI)
cargo build -p tiagent-cli --no-default-features

# Default build (CLI + HTTP server, no Celestia)
cargo build -p tiagent-cli

# Full build (everything including Celestia)
cargo build -p tiagent-cli --features full

# Development build (CLI backends only, no server, no Celestia)
cargo build -p tiagent-cli --no-default-features --features "cli-backends"
```

---

## 6. Key External Dependencies

This section lists the major third-party crates used across the workspace, grouped by
purpose. Minor utilities (`thiserror`, `chrono`, `uuid`, etc.) are omitted.

### Celestia ecosystem

| Crate | Version | Used by | Purpose |
|-------|---------|---------|---------|
| `celestia-types` | 1.x | `tiagent-celestia` | Blob, namespace, and commitment types |
| `celestia-rpc` | 1.x | `tiagent-celestia` | RPC client for Celestia node communication |
| `lumina-node` | latest | `tiagent-celestia` | Embedded light node (feature-gated) |
| `nmt-rs` | latest | `tiagent-celestia` | Namespaced Merkle Tree proof verification |

### Web and networking

| Crate | Version | Used by | Purpose |
|-------|---------|---------|---------|
| `axum` | 0.7+ | `tiagent-serve` | HTTP framework |
| `tower` / `tower-http` | 0.4+ | `tiagent-serve` | Middleware (CORS, compression, tracing) |
| `reqwest` | 0.12+ | `tiagent-agent` | HTTP client for LLM API calls |
| `tokio` | 1.x | everywhere | Async runtime |

### Serialization and schemas

| Crate | Version | Used by | Purpose |
|-------|---------|---------|---------|
| `serde` / `serde_json` | 1.x | everywhere | Serialization/deserialization |
| `toml` | 0.8+ | `tiagent-orchestrator`, `tiagent-cli` | Plan and config file parsing |
| `schemars` | 0.8+ | `tiagent-tools` | JSON Schema generation for tool definitions |

### CLI and user interface

| Crate | Version | Used by | Purpose |
|-------|---------|---------|---------|
| `clap` | 4.x | `tiagent-cli`, MCP binaries | Command-line argument parsing |
| `ratatui` | 0.28+ | `tiagent-cli` | Terminal UI (feature-gated) |
| `tracing` / `tracing-subscriber` | 0.1+ / 0.3+ | everywhere | Structured logging |

### Other

| Crate | Version | Used by | Purpose |
|-------|---------|---------|---------|
| `sha2` | 0.10+ | `tiagent-core` | SHA-256 for content addressing |
| `async-trait` | 0.1+ | `tiagent-core` | Async trait support |
| `handlebars` | 5.x | `tiagent-compose` | Prompt template rendering |
| `tiktoken-rs` | 0.5+ | `tiagent-compose` | Token counting for context budgeting |
| `tree-sitter` | 0.22+ | `tiagent-mcp-code` | Code parsing for symbol extraction |
| `rand` | 0.8+ | `tiagent-learn` | Experiment randomization |

---

## 7. Comparison with Prior Art

### 7.1 Crate count comparison

| Project | Crate count | LOC (approx.) | Assessment |
|---------|-------------|---------------|------------|
| **polkagent** | 90 | ~200K | Excessive. Many crates have <200 LOC. Compile times are painful. New contributors struggle to find where code lives. Adding a feature touches 5--10 Cargo.toml files. |
| **roko** | 18 | ~177K | Manageable but some overlap. `roko-fs` (local storage) and `roko-neuro` (knowledge store) both implement storage patterns. `roko-primitives` could be part of `roko-core`. `roko-std` (defaults + built-in tools) straddles two responsibilities. |
| **tiagent** | 14 | target ~50K | Sweet spot. Every crate has a clear, non-overlapping responsibility. Compile times should remain fast. A new contributor can understand the workspace structure in 15 minutes. |

### 7.2 Comparison with agent harnesses

tiagent occupies the same space as existing coding agent tools, but as an open-source,
extensible Rust crate workspace rather than a closed-source product.

| Dimension | Claude Code | Codex CLI | Cursor | tiagent |
|-----------|-------------|-----------|--------|---------|
| **Source** | Closed | Closed (CLI open, engine closed) | Closed | Open-source (Apache-2.0) |
| **Extensibility** | MCP servers only | MCP servers only | Extensions + MCP | Rust crates + MCP servers + feature flags |
| **Gate pipeline** | None (manual review) | None | None | 7-rung adaptive pipeline (compile, test, lint, diff, semantic, human) |
| **Plan execution** | Single prompt | Single prompt | Single prompt | DAG-based multi-task orchestration with resume |
| **Learning** | None | None | None | Episode logging, cascade routing, prompt A/B, playbook extraction |
| **Storage** | Local only | Local only | Local only | Local (default) + Celestia DA (optional) |
| **Multi-backend** | Claude only | GPT only | Multiple (closed) | Claude, OpenAI-compat, Ollama, CLI backends (open, pluggable) |
| **Self-hosting** | No | No | No | Yes --- tiagent can read PRDs, generate plans, execute them, and improve itself |

The key difference: Claude Code, Codex, and Cursor are products. tiagent is a toolkit.
You can swap backends, add gates, write custom tools as crates, extend the orchestrator,
and ship a custom agent binary --- all with `cargo build`.

### 7.3 Structural differences (vs. Rust agent workspaces)

| Dimension | polkagent | roko | tiagent |
|-----------|-----------|------|---------|
| **Kernel crate** | `polkagent-core` (types + traits) | `roko-core` (signal + traits + config + tools + errors) | `tiagent-core` (signal + traits + errors + config). Focused: no tool defs, no built-in implementations. |
| **Storage** | Off-chain DB adapter crates | `roko-fs` (JSONL), `roko-neuro` (knowledge store) | `tiagent-store` (JSONL + SQLite) + `tiagent-celestia` (DA). Clear split: local vs. on-chain. |
| **Tool system** | Scattered across chain-specific crates | `roko-std` (built-in tools) + `roko-agent` (MCP client) | `tiagent-tools` (registry + MCP + schemas). One crate for everything tool-related. |
| **Learning** | None | `roko-learn` (episodes, router, experiments, efficiency) | `tiagent-learn` (same scope). Proven design, carried forward. |
| **Chain integration** | 20+ crates for Polkadot pallets, XCM, staking | `roko-chain` (alloy RPC, partial) | `tiagent-celestia` (one crate, focused on DA). |
| **MCP servers** | None | `roko-mcp-code`, `roko-mcp-github`, etc. | `tools/tiagent-mcp-celestia`, `tools/tiagent-mcp-code`. Separate directory, clear binary boundary. |

### 7.4 Lessons applied

| Lesson | Source | How tiagent applies it |
|--------|--------|----------------------|
| Too many crates slow everything down | polkagent (90 crates) | Hard cap at 15 crates. Merge small crates into neighbors. |
| Kernel crate must be zero-opinion | roko (`roko-core` includes config specifics) | `tiagent-core` defines traits and types only. No default implementations, no config file parsing. |
| Storage should have one crate per backend, not one crate per pattern | roko (`roko-fs` + `roko-neuro` overlap) | `tiagent-store` handles all local storage. `tiagent-celestia` handles all DA storage. No overlap. |
| Tools, MCP client, and MCP server belong together | roko (tools in `roko-std`, MCP in `roko-agent`) | `tiagent-tools` is the single home for tool schemas, MCP client, and MCP server scaffolding. |
| MCP servers are standalone binaries, not library crates | roko (MCP servers as library crates) | `tools/` directory with binary crates that communicate over stdio. |
| Feature flags prevent dependency bloat | Both projects | Heavy deps (`lumina-node`, `axum`, `ratatui`) are feature-gated. Default build is lean. |

---

## 8. Decision Log

This section records key decisions about crate structure and the rationale behind them.
Future contributors should read this before proposing structural changes.

### D1: Why not merge tiagent-celestia into tiagent-store?

Both are about storage, but they serve different roles. `tiagent-store` manages the local
`.tiagent/` directory and provides fast, always-available local storage. `tiagent-celestia`
manages blob submission, namespace organization, and light node interaction --- concerns
specific to Celestia that would pollute a generic storage crate. The `HybridSubstrate`
(which combines both) lives in `tiagent-store` and depends on `tiagent-celestia` via an
optional feature flag, keeping the two concerns cleanly separated.

### D2: Why is tiagent-tools separate from tiagent-agent?

The agent crate handles LLM dispatch and the tool call loop. The tools crate handles tool
definitions, schemas, and the MCP integration layer. Separating them means you can use the
tool registry and MCP infrastructure without pulling in LLM backend code, and vice versa.
This matters for the MCP server binaries in `tools/`, which need tool schemas but do not
need LLM dispatch.

### D3: Why are MCP servers in tools/ instead of crates/?

MCP servers are standalone binaries, not libraries. They communicate with the rest of
tiagent over stdio or HTTP, not through Rust function calls. Placing them in a separate
`tools/` directory makes this architectural boundary visible in the filesystem and prevents
accidental tight coupling.

### D4: Why is tiagent-runtime separate from tiagent-orchestrator?

The orchestrator manages plan execution (task DAGs, dependencies, dispatch). The runtime
manages process lifecycle (spawning, supervision, shutdown, event bus). These are different
concerns. A process supervisor is useful even without plan orchestration (for example, when
running a single long-lived agent). Keeping them separate allows `tiagent-runtime` to
remain a low-dependency utility crate.

### D5: Why 14 crates and not 8?

The original target was 8--12 crates. The final count of 14 (12 libraries + 2 binaries)
reflects a pragmatic tradeoff: merging further would create crates with too many
responsibilities. For example, merging `tiagent-gate` into `tiagent-orchestrator` would
make the orchestrator responsible for both execution scheduling and validation logic,
violating single-responsibility. The two MCP server binaries in `tools/` are unavoidable ---
they are separate processes by design.

### D6: Why feature-gate the HTTP server?

Not every deployment needs an HTTP API. A CI/CD pipeline agent, a cron-triggered automation
agent, or a developer running `tiagent run "..."` from the command line all work fine
without `axum` and its tower middleware stack. Feature-gating `tiagent-serve` saves about
15 seconds of compile time and ~5MB of binary size for users who do not need it.
