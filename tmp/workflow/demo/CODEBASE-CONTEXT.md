# Codebase Context: Nunchi / Roko

**Purpose**: Complete technical reference for the Nunchi codebase — every crate, every file, every API endpoint, every CLI command, the build pipeline, and how everything connects. Written for someone with zero prior context about the project, Rust, React, or any tool used here.

**Date**: April 2026

---

## 0. Glossary

Before anything else, here are terms used throughout this document and the rest of the demo planning docs.

| Term | What It Means |
|------|---------------|
| **Nunchi** | The company and product name. Two components: the Roko runtime (open source) and the Nunchi blockchain (purpose-built L1 chain). |
| **Roko** | The open-source Rust runtime for building and coordinating AI agents. 18 crates, ~177K lines of code. Apache 2.0 license. This is the code in the repository. |
| **Crate** | Rust's unit of compilation — similar to a "package" in npm or a "module" in Python. Each crate lives in its own directory under `crates/` and has its own `Cargo.toml` manifest. |
| **Cargo** | Rust's build system and package manager (like npm for JavaScript or pip for Python). `cargo build` compiles, `cargo test` runs tests, `cargo clippy` runs the linter. |
| **Cargo.toml** | The manifest file for a Rust crate or workspace. Lists dependencies, features, metadata. Equivalent to `package.json` in Node.js. |
| **Workspace** | A Cargo workspace is a collection of crates that share dependencies and build together. The root `Cargo.toml` at the repo root lists all workspace members. |
| **Axum** | The Rust web framework used for the HTTP server (`roko-serve`). Similar to Express.js in Node.js. Runs on Tokio (async runtime). |
| **Tokio** | Rust's async runtime — provides the event loop, timers, networking. Equivalent to Node.js's event loop but explicit. |
| **ArcSwap** | A lock-free atomic pointer swap. Used for hot-reloading configuration without stopping the server. |
| **rust-embed** | A Rust crate that bakes static files (like a compiled React SPA) into the binary at compile time. The binary is fully self-contained — no external file dependencies at runtime. |
| **build.rs** | A Rust build script that runs before compilation. Used here to trigger `npm run build` so the React app is compiled before rust-embed bakes it into the binary. |
| **SPA** | Single Page Application — a web app that loads once and navigates via JavaScript, not full page reloads. The React app is an SPA. |
| **Vite** | A fast JavaScript build tool. Compiles React/TypeScript into optimized static files in a `dist/` directory. Also provides a dev server with Hot Module Replacement (HMR). |
| **React Router** | Client-side routing for React SPAs. Maps URL paths to React components. The server must serve `index.html` for all paths (SPA fallback) so React Router can handle navigation. |
| **xterm.js** | A browser-based terminal emulator. Renders a terminal in a `<canvas>` element. Connected to a real shell via WebSocket. |
| **PTY** | Pseudo-Terminal — a Unix mechanism that creates a fake terminal for programs to run in. `portable-pty` (a Rust crate) creates PTYs that xterm.js connects to via WebSocket. |
| **WebSocket** | A persistent bidirectional connection between browser and server. Used for terminal I/O (keystrokes sent to server, output streamed back) and real-time event streaming. |
| **SSE** | Server-Sent Events — a one-way stream from server to browser. Used for dashboard event updates. |
| **Gate** | A validation step that checks agent output. Examples: does the code compile? Do tests pass? Does clippy (Rust linter) find warnings? Does a semantic review pass? Gates run after every agent execution. |
| **Gate Pipeline** | The ordered sequence of gates. 11 gates across 7 rungs (levels). Each rung can have adaptive thresholds that adjust based on historical pass rates. |
| **CascadeRouter** | The model routing system. Uses Thompson sampling and LinUCB bandits to route each task to the cheapest model that can handle it. Saves 3-5x on model costs. |
| **NeuroStore** | The durable knowledge store. Agents deposit facts (knowledge entries) and query them. Entries have confidence scores, timestamps, and citation counts. Unused entries decay via Ebbinghaus forgetting curve (demurrage). |
| **HDC** | Hyperdimensional Computing — uses 10,240-bit binary vectors for similarity search, fingerprinting, and routing. Much cheaper than embedding-based similarity (~400 gas on-chain vs 100,000+ for conventional approaches). |
| **ERC-8004** | An Ethereum token standard for agent identities. Each agent has a verifiable on-chain identity with 7-domain reputation (code quality, resource efficiency, latency, cost optimization, safety, gate compliance, collaboration). |
| **ZK-HDC** | Zero-Knowledge proofs over HDC vectors. Proves that an agent's computation matches its claims without revealing the underlying data. |
| **Signal** | Roko's universal data unit. Everything is a Signal: prompts, agent outputs, gate verdicts, episodes, knowledge entries. Signals are content-addressed (hashed) and stored in `.roko/engrams.jsonl`. |
| **Episode** | A record of one agent execution: what prompt was given, what output was produced, which gates passed/failed, cost, tokens used. Stored in `.roko/episodes.jsonl`. |
| **mirage-rs** | A local EVM fork simulator. Creates a realistic Ethereum-like blockchain environment without connecting to any real chain. Used for demos and testing. |
| **ROSEDUST** | The current design system for the web dashboard: dark purple-black backgrounds (#060608), dusty rose accents (#AA7088), bone off-white (#C8B890), Instrument Serif + JetBrains Mono fonts, CRT scan-line overlay, film grain texture. |
| **Tokyo Night** | A code editor color theme — electric blue (#7AA2F7), purple (#BB9AF7), dark blue-black background (#1A1B26). Proposed replacement for ROSEDUST in the demo. |
| **Clack-style** | A CLI output formatting convention using Unicode symbols (◆ ◇ │ └ ✔ ✖) instead of plain text or emoji. Named after the `@clack/prompts` npm package. |
| **ACP** | Agent Client Protocol — a JSON-RPC 2.0 protocol over stdio that lets editors (VS Code, JetBrains, Zed, Neovim) use Roko as a coding agent backend. Similar to LSP but for AI agents. |
| **ISFR** | Internet Secured Funding Rate — an on-chain benchmark rate. Future expansion product, not current. The agent coordination plane's equivalent of SOFR (the rate that underpins $668T in interest rate derivatives). |

---

## 1. Repository Structure

The repository lives at `/Users/will/dev/nunchi/roko/roko/`. Here is its top-level layout:

```
roko/
├── Cargo.toml                    # Workspace root — lists all 33 crate members
├── Cargo.lock                    # Locked dependency versions
├── CLAUDE.md                     # Project instructions for AI agents
├── .roko/                        # Runtime data directory (created by `roko init`)
│   ├── engrams.jsonl             # Signal store (all signals as JSONL)
│   ├── episodes.jsonl            # Episode log (agent execution records)
│   ├── events.jsonl              # Event log
│   ├── state/                    # Executor snapshots for resume
│   ├── learn/                    # Learning state (router, experiments, thresholds, efficiency)
│   ├── prd/                      # Product requirement documents
│   ├── research/                 # Research artifacts
│   └── acp/                      # ACP session persistence
│       └── sessions/             # Persisted ACP sessions as JSON
├── crates/                       # All 33 Rust crates
│   ├── roko-cli/                 # Main binary — CLI commands, TUI, orchestrator
│   ├── roko-serve/               # HTTP server — ~115 API routes, WebSocket, SSE
│   ├── roko-agent/               # Agent dispatch — 5+ LLM backends, MCP, tool loop
│   ├── roko-agent-server/        # Per-agent HTTP sidecar (13 routes)
│   ├── roko-acp/                 # Agent Client Protocol (JSON-RPC 2.0 over stdio)
│   ├── roko-core/                # Signal + 6 verb traits, types, config
│   ├── roko-std/                 # Default implementations, 19 built-in tools
│   ├── roko-gate/                # 11 gates, 7-rung pipeline, adaptive thresholds
│   ├── roko-compose/             # Prompt assembly, 9 templates
│   ├── roko-orchestrator/        # Plan DAG, parallel executor, merge queue
│   ├── roko-learn/               # Episodes, playbooks, bandits, cascade router
│   ├── roko-neuro/               # Durable knowledge store, distillation
│   ├── roko-fs/                  # FileSubstrate — JSONL signal store
│   ├── roko-runtime/             # ProcessSupervisor, event bus, cancellation
│   ├── roko-primitives/          # HDC vectors, tier routing
│   ├── roko-conductor/           # 10 watchers, circuit breaker, diagnosis
│   ├── roko-plugin/              # EventSource, FeedbackCollector plugin SDK
│   ├── roko-chain/               # Chain client + wallet (alloy backend)
│   ├── roko-dreams/              # Offline consolidation (hypnagogia, imagination)
│   ├── roko-daimon/              # Affect engine, somatic markers
│   ├── roko-demo/                # Demo environment orchestrator
│   ├── roko-index/               # Code parser, symbol graph, HDC fingerprints
│   ├── roko-lang-rust/           # Rust language support for gates
│   ├── roko-lang-typescript/     # TypeScript language support
│   ├── roko-lang-go/             # Go language support
│   ├── roko-mcp-stdio/           # MCP server (stdio transport)
│   ├── roko-mcp-code/            # Code-intelligence MCP server
│   ├── roko-mcp-github/          # GitHub MCP integration
│   ├── roko-mcp-slack/           # Slack MCP integration
│   ├── roko-mcp-scripts/         # Script-based MCP server
│   └── ...
├── apps/                         # Standalone applications
│   ├── mirage-rs/                # In-process EVM fork simulator
│   ├── agent-relay/              # Agent relay proxy
│   └── roko-chain-watcher/       # Long-running chain observer
├── demo/
│   ├── demo-app/                 # React + Vite SPA (the web dashboard)
│   │   ├── package.json
│   │   ├── vite.config.ts
│   │   ├── tsconfig.json
│   │   ├── index.html
│   │   ├── dist/                 # Build output (baked into binary via rust-embed)
│   │   └── src/                  # React source code
│   └── demo-web/                 # Legacy vanilla HTML files (7 pages, being replaced)
└── tmp/                          # Planning and reference documents (not shipped)
    ├── workflow/demo/            # These demo planning documents
    ├── unified/                  # 29 v3.0 specification documents
    ├── unified-depth/            # 128 depth documents across 22 directories
    ├── research/                 # 15 research documents
    └── learnings2/               # 15 distilled briefing documents
```

---

## 2. The Workspace: All 33 Crates

The workspace `Cargo.toml` at the repo root lists all crates:

### Core Library Crates (what they do)

| Crate | Path | What It Does |
|-------|------|-------------|
| `roko-primitives` | `crates/roko-primitives/` | 10,240-bit HDC (hyperdimensional computing) binary vectors. Similarity search, fingerprinting, tier-based model routing. The mathematical foundation for efficient similarity comparison. |
| `roko-runtime` | `crates/roko-runtime/` | `ProcessSupervisor` manages agent subprocess lifecycles (start, stop, health check). Event bus for publish/subscribe. `CancelToken` for cooperative cancellation. |
| `roko-core` | `crates/roko-core/` | The kernel. Defines `Signal` (universal data unit) + 6 verb traits: `Substrate` (storage), `Scorer` (ranking), `Gate` (validation), `Router` (model selection), `Composer` (prompt assembly), `Policy` (enforcement). Also: `RokoConfig` schema (the `roko.toml` structure), tool definitions, error types. |
| `roko-std` | `crates/roko-std/` | Default implementations of the 6 traits. 19 built-in tools (file read/write, shell exec, web search, etc.). Mock dispatcher for testing. |
| `roko-gate` | `crates/roko-gate/` | 11 gate implementations: `CompileGate`, `TestGate`, `ClippyGate`, `ShellGate`, `DiffReviewGate`, `LlmJudgeGate`, `IntegrationTestGate`, `RegressionGate`, `SecurityScanGate`, `CoverageGate`, `PerformanceGate`. 7-rung pipeline orders them from cheapest (compile) to most expensive (LLM judge). Adaptive thresholds adjust pass criteria based on historical data. |
| `roko-fs` | `crates/roko-fs/` | `FileSubstrate` — stores signals as newline-delimited JSON (JSONL) in `.roko/engrams.jsonl`. Handles atomic writes, garbage collection, and layout (directory structure for `.roko/`). |
| `roko-compose` | `crates/roko-compose/` | `PromptComposer` assembles prompts from sections with priority and placement. `SystemPromptBuilder` builds 9-layer system prompts from role templates, knowledge hints, context bidders, and tool descriptions. 9 built-in templates for different agent roles. |
| `roko-plugin` | `crates/roko-plugin/` | SDK for event sources (cron, file watch, webhook) and feedback collectors. Plugins extend Roko's event-driven architecture. |
| `roko-agent` | `crates/roko-agent/` | Agent dispatch to 8+ LLM backends: Claude CLI (subprocess), Anthropic API (direct HTTP), Ollama (local models), Codex, OpenAI-compatible, Gemini, Perplexity, generic subprocess. MCP (Model Context Protocol) configuration passthrough. Tool loop: agent calls tools → tools execute → results fed back to agent. Safety layer: role-based authorization, pre/post execution checks. |
| `roko-orchestrator` | `crates/roko-orchestrator/` | Plan DAG (Directed Acyclic Graph) executor. Reads `tasks.toml` files that define task dependencies. Runs independent tasks in parallel. Merge queue for coordinating results. Safety checks before and after task execution. |
| `roko-chain` | `crates/roko-chain/` | Chain client (reads) and wallet (writes) using the `alloy` Rust Ethereum library. Interacts with the Nunchi L1 chain or local mirage-rs simulator. ERC-8004 identity registration, knowledge publication, ZK proof anchoring. |
| `roko-conductor` | `crates/roko-conductor/` | 10 health watchers that monitor different subsystems. Circuit breaker patterns (stops retrying when a service is consistently failing). Diagnosis engine that identifies root causes of failures. |
| `roko-learn` | `crates/roko-learn/` | Episode storage and querying. Playbook extraction (recurring patterns from episode history). Thompson sampling / LinUCB bandits for the CascadeRouter. A/B experiment framework for prompt variations. Efficiency event tracking (cost, tokens, latency per turn). |
| `roko-neuro` | `crates/roko-neuro/` | Durable knowledge store (NeuroStore). Knowledge entries with confidence scores, timestamps, source attribution. Ebbinghaus decay curve: unused knowledge fades, cited knowledge strengthens. Tier progression: entries graduate from ephemeral → working → consolidated → crystallized based on usage. HDC fingerprinting for similarity-based retrieval. |
| `roko-dreams` | `crates/roko-dreams/` | Offline consolidation cycles. Hypnagogia: generates novel connections between unrelated knowledge entries. Imagination: explores hypothetical scenarios. Cycle: scheduled consolidation (like sleep for the knowledge store). Currently built but not triggered at runtime (no cron job wired). |
| `roko-daimon` | `crates/roko-daimon/` | Affect engine using a PAD (Pleasure-Arousal-Dominance) vector model. Somatic markers influence dispatch decisions: high frustration → route to more capable model, low arousal → maintain current routing. Modulates agent behavior based on accumulated experience. |
| `roko-demo` | `crates/roko-demo/` | Demo environment orchestrator. Sets up pre-seeded data, agent configurations, and scenarios for demo purposes. |
| `roko-index` | `crates/roko-index/` | Code intelligence: parses source files (Rust, TypeScript, Go), builds symbol graphs, computes PageRank for importance, generates HDC fingerprints for code similarity. Used by the MCP code intelligence server. |
| `roko-lang-rust` | `crates/roko-lang-rust/` | Rust-specific gate implementations (compile with `cargo build`, test with `cargo test`, lint with `cargo clippy`). |
| `roko-lang-typescript` | `crates/roko-lang-typescript/` | TypeScript-specific gate implementations. |
| `roko-lang-go` | `crates/roko-lang-go/` | Go-specific gate implementations. |
| `roko-mcp-code` | `crates/roko-mcp-code/` | MCP (Model Context Protocol) server that provides code intelligence tools: symbol lookup, definition finding, reference search, code search. |
| `roko-mcp-github` | `crates/roko-mcp-github/` | MCP server for GitHub: PR management, issue queries, code search. |
| `roko-mcp-slack` | `crates/roko-mcp-slack/` | MCP server for Slack: message posting, channel queries. |
| `roko-mcp-scripts` | `crates/roko-mcp-scripts/` | MCP server that exposes shell scripts as tools. |
| `roko-mcp-stdio` | `crates/roko-mcp-stdio/` | Generic MCP server transport over stdio. |

### Application Crates (the binaries)

| Crate | Path | What It Produces |
|-------|------|-----------------|
| `roko-cli` | `crates/roko-cli/` | **The main binary**: `roko`. All CLI subcommands (run, plan, prd, agent, serve, chat, dashboard, etc.). Ratatui TUI with F1-F7 tabs. The orchestrator (`orchestrate.rs`) that wires everything together. |
| `roko-serve` | `crates/roko-serve/` | **The HTTP server**: `roko serve`. ~115 API routes, WebSocket, SSE, embedded React SPA. Runs on port 6677 by default. |
| `roko-agent-server` | `crates/roko-agent-server/` | **Per-agent sidecar**: 13 HTTP routes for a single agent (message, stream, predictions, research, tasks). |
| `roko-acp` | `crates/roko-acp/` | **ACP server**: JSON-RPC 2.0 over stdio for editor integrations. |
| `mirage-rs` | `apps/mirage-rs/` | **EVM simulator**: local Ethereum fork for testing chain interactions. |
| `agent-relay` | `apps/agent-relay/` | **Relay proxy**: routes messages between distributed agents. |
| `roko-chain-watcher` | `apps/roko-chain-watcher/` | **Chain observer**: watches for on-chain events (knowledge publications, identity changes). |

### Key Workspace Dependencies (versions)

| Dependency | Version | What It Does |
|-----------|---------|-------------|
| `tokio` | 1.50 | Async runtime (event loop, networking, timers) |
| `axum` | 0.8 | Web framework with WebSocket support |
| `reqwest` | 0.12 | HTTP client |
| `serde` | 1.0 | Serialization/deserialization (JSON, TOML, etc.) |
| `alloy` | 1 | Ethereum library (transactions, ABI, providers) |
| `revm` | 36.0 | Rust EVM implementation (for mirage-rs) |
| `tower-http` | 0.6 | HTTP middleware (CORS, tracing) |
| `tracing` | 0.1 | Structured logging |
| `uuid` | 1 | UUID generation |
| `chrono` | 0.4 | Date/time handling |
| `blake3` | 1 | Fast cryptographic hashing (for content addressing) |
| `rust-embed` | 8 | Static file embedding (SPA into binary) |
| `portable-pty` | 0.9 | Cross-platform pseudo-terminal creation |
| `utoipa` | 5 | OpenAPI spec generation from Rust types |
| `jsonwebtoken` | 9 | JWT verification (for Privy auth) |

---

## 3. The `roko run` Universal Loop (How Agent Execution Works)

**File**: `crates/roko-cli/src/run.rs`

This is the core execution path. Every `roko run "<prompt>"` invocation follows these exact steps:

### Step-by-step flow

```
User types: roko run "Fix the failing test in src/auth.rs"
                │
                ▼
Step 1: OPEN SUBSTRATE
  Opens .roko/engrams.jsonl (the signal store)
  Creates it if it doesn't exist
                │
                ▼
Step 2: CREATE EVENT HUB
  If called from HTTP (/api/run), uses the server's shared StateHub
  If called from CLI, creates a local hub with .roko/events.jsonl
                │
                ▼
Step 3: BUILD PROMPT SECTIONS
  Creates sections with priority and placement:
    - Role section (from roko.toml config) → Critical priority, placed at Start
    - File sections (from roko.toml [[prompt.files]]) → loaded from disk
    - Task section (the user's prompt text) → Critical priority, placed at End
  Each section is persisted as a Signal to the substrate
                │
                ▼
Step 4: COMPOSE PROMPT
  PromptComposer::compose() assembles sections into a single prompt
  Respects token budget, priority ordering, and placement rules
  Persists the composed prompt as a Signal
                │
                ▼
Step 5: DISPATCH AGENT
  The routing logic tries backends in this priority order:

  1. If roko.toml has providers/models config → CascadeRouter
     - Uses Thompson sampling / LinUCB bandit to pick the best model
     - Builds a 9-layer system prompt via SystemPromptBuilder
     - Spawns a scoped agent process

  2. If `claude` binary exists + ANTHROPIC_API_KEY env var → Anthropic API
     - Direct HTTP to Anthropic's API
     - Full tool loop with 19 built-in tools

  3. If `claude` binary exists (no API key) → Claude CLI subprocess
     - Runs: claude --output-format stream-json --system "..." --allowedTools "..."
     - Reads streaming JSON output
     - Supports --resume and --fallback-model

  4. If `ollama` binary exists → Ollama local inference
     - Uses OllamaLlmBackend
     - Full tool loop

  5. Known protocol (openai, gemini, etc.) → synthesized config

  6. Any other command → generic subprocess

  The agent runs to completion or timeout.
                │
                ▼
Step 6: CLEAN OUTPUT (optional)
  If config.agent.clean_output = true:
    - Strips ANSI escape codes
    - Strips reasoning-model thinking traces
  Raw output preserved as an AgentMessage trace signal
                │
                ▼
Step 7: PERSIST OUTPUT
  Writes agent_result.output as a Signal
  Writes all trace signals (tool calls, intermediate steps)
                │
                ▼
Step 8: PUBLISH DASHBOARD EVENTS
  Emits to the event hub:
    - PlanStarted, TaskStarted
    - AgentSpawned, AgentOutput, TaskOutputAppended
  These events drive the TUI dashboard and web dashboard via SSE/WebSocket
                │
                ▼
Step 9: RUN GATE PIPELINE
  For each [[gates]] entry in roko.toml (in order):
    Supported types: Shell, Compile, Clippy, Test
    Runs the gate → gets GateVerdict (pass/fail)
    Persists each verdict as a Signal
                │
                ▼
Step 10: PUBLISH GATE EVENTS
  Emits GateResult for each verdict
                │
                ▼
Step 11: RECORD EPISODE
  Creates an Episode signal with:
    - Prompt ID, output ID
    - Agent success flag
    - Gate verdicts
    - Cost, tokens, latency
  Persists to substrate
                │
                ▼
Step 12: APPEND EPISODE LOG
  Writes to .roko/episodes.jsonl (consumed by learning subsystems)
                │
                ▼
Step 13: PUBLISH COMPLETION EVENTS
  Emits TaskCompleted, PlanCompleted, EpisodeRecorded
  Emits 3 EfficiencyEvent records (input tokens, output tokens, cost USD)
                │
                ▼
Step 14: RETURN RunReport
  {
    episode_id: "content hash of Episode",
    prompt_id: "content hash of Prompt",
    agent_output_id: "content hash of output",
    agent_success: bool,
    gate_verdicts: [("compile", true), ("test", true), ("clippy", true)],
    total_signals: N,
    output_text: "the agent's text output"
  }

  overall_success() = agent_success && all_gates_passed
```

---

## 4. The HTTP Server (`roko-serve`)

**File**: `crates/roko-serve/src/lib.rs` + `crates/roko-serve/src/routes/mod.rs`

### Server startup sequence

When you run `roko serve`, the following happens in order:

1. **Parse config**: Load `roko.toml`, resolve port (explicit arg → `$PORT` env → config → default 6677)
2. **Build AppState**: Construct the shared state struct (see below)
3. **Restore snapshot**: Load persisted server state from disk (running runs, plans, operations)
4. **Start dispatch loop**: Background task that processes agent dispatch requests from a queue
5. **Start event sources**: Register cron, file watch, and webhook event sources from config
6. **Start config watcher**: File watcher on `roko.toml` — hot-reloads config via ArcSwap
7. **Start PRD publisher**: Subscribes to PRD publish events → auto-triggers plan generation
8. **Start feedback loop**: Adaptive gate thresholds + cascade router learning
9. **Start StateHub bridge**: Bridges EventBus events into the shared StateHub (feeds TUI, SSE, WS)
10. **Start snapshot saver**: Periodically saves AppState to disk for crash recovery
11. **Start job runner**: Background execution of marketplace jobs
12. **Start cold archival**: Scheduled cold storage for old signals
13. **Load deployments**: Restore cloud deployment records from disk
14. **Prime JWKS cache**: If Privy auth configured, pre-fetch JWT verification keys
15. **Register with relay**: Announce workspace to the relay network
16. **Build router**: Construct the full Axum router with all routes and middleware
17. **Bind TCP listener**: Start accepting HTTP connections
18. **Start chain watcher**: If `chain.rpc_url` is set, spawn chain observer subprocess
19. **Serve with graceful shutdown**: Run until cancel token is triggered

### AppState (the shared server state)

Every route handler receives `AppState` via Axum's `State` extractor. Key fields:

| Field | Type | What It Holds |
|-------|------|--------------|
| `workdir` | `PathBuf` | Project working directory |
| `layout` | `RokoLayout` | Helper for `.roko/` directory paths |
| `signal_store` | `SignalStore` | Lazy JSONL writer for signals |
| `cancel` | `CancelToken` | Graceful shutdown trigger |
| `started_at` | `Instant` | Server start time (for uptime) |
| `metrics` | `Arc<MetricRegistry>` | Prometheus-style counters |
| `supervisor` | `Arc<ProcessSupervisor>` | Agent process lifecycle manager |
| `event_bus` | `EventBus<ServerEvent>` | Broadcast channel for SSE |
| `state_hub` | `SharedStateHub` | Dashboard snapshot + event streaming |
| `roko_config` | `ArcSwap<RokoConfig>` | Lock-free hot-reload config |
| `active_runs` | `RwLock<HashMap<String, RunHandle>>` | In-flight one-shot runs |
| `active_plans` | `RwLock<HashMap<String, PlanHandle>>` | In-flight plan executions |
| `terminal_sessions` | `SessionManager` | PTY sessions for web terminal |
| `chain_client` | `Option<Arc<AlloyChainClient>>` | On-chain reads (if configured) |
| `chain_wallet` | `Option<Arc<AlloyChainWallet>>` | On-chain writes (if configured) |
| `cascade_router` | `RwLock<Option<CascadeRouter>>` | Cached model router state |

### Middleware stack (outermost to innermost)

1. **CORS** — configurable origins from `roko.toml`
2. **TraceLayer** — structured request/response logging
3. **scrub_secrets** — scans JSON responses and redacts API keys
4. **require_api_key** — `Authorization: Bearer <key>` (when auth enabled)
5. **require_scope** — JWT scope validation (when auth enabled)

### Complete API route catalog

**Total**: ~115 routes organized into functional groups.

#### Top-level (no `/api` prefix)

| Method | Path | What It Does |
|--------|------|-------------|
| GET | `/health` | Load balancer health check → `{"status":"ok"}` |
| POST | `/webhooks/github` | GitHub webhook receiver |
| POST | `/webhooks/slack` | Slack webhook receiver |
| POST | `/webhooks/generic` | Generic webhook receiver |
| GET | `/runs/{id}` | Shareable run page (HTML) |
| GET | `/ws/terminal/{id}` | WebSocket for terminal PTY I/O |
| GET | `/ws` or `/roko-ws` | WebSocket for dashboard events |
| GET | `/api/events` or `/api/sse` | SSE stream for ServerEvent |
| `*` | `/*` (fallback) | Serves React SPA via rust-embed |

#### Terminal sessions

| Method | Path | What It Does |
|--------|------|-------------|
| POST | `/api/terminal/sessions` | Create PTY session (body: `{session_id, cols, rows}`) |
| GET | `/api/terminal/sessions` | List active sessions |
| DELETE | `/api/terminal/sessions/{id}` | Destroy session |
| POST | `/api/terminal/sessions/{id}/input` | Send text input to PTY |

#### Status, health, metrics

| Method | Path | What It Returns |
|--------|------|----------------|
| GET | `/api/health` | Detailed health: uptime, version, active plans/agents/runs, provider health |
| GET | `/api/status` | Dashboard session status |
| GET | `/api/episodes` | All agent episode records |
| GET | `/api/signals` | Signal store contents |
| GET | `/api/dashboard` | Full dashboard snapshot |
| GET | `/api/metrics` | All raw metrics |
| GET | `/api/metrics/summary` | Aggregated metrics |
| GET | `/api/metrics/c_factor` | C-factor efficiency composite |
| GET | `/api/metrics/prometheus` | Prometheus scrape endpoint |
| GET | `/api/statehub/snapshot` | Current StateHub snapshot |
| GET | `/api/statehub/events` | StateHub event log |
| GET | `/api/gates/summary` | Gate pass/fail summary |
| GET | `/api/gates/history` | Gate history |
| GET | `/api/truth_map` | Causal truth map |

#### Run (single-task execution)

| Method | Path | What It Does |
|--------|------|-------------|
| POST | `/api/run` | Start a `roko run` (body: `{prompt, workdir?}`) → returns `{run_id}` |
| GET | `/api/run/{id}/status` | Poll run status → `{status, result?, error?}` |

#### Plans (multi-task execution)

| Method | Path | What It Does |
|--------|------|-------------|
| GET/POST | `/api/plans` | List / create plans |
| GET | `/api/plans/{id}` | Get plan details |
| GET | `/api/plans/{id}/tasks` | Plan's task list |
| POST | `/api/plans/{id}/execute` | Start plan execution |
| GET | `/api/plans/{id}/status` | Execution status |
| POST | `/api/plans/{id}/pause` | Pause execution |
| POST | `/api/plans/{id}/resume` | Resume execution |
| POST | `/api/plans/generate` | Generate plan from prompt |

#### PRDs (Product Requirement Documents)

| Method | Path | What It Does |
|--------|------|-------------|
| GET | `/api/prds` | List all PRDs |
| POST | `/api/prds/ideas` | Capture work item idea |
| GET | `/api/prds/{slug}` | Get PRD |
| POST | `/api/prds/{slug}/draft` | Draft PRD (agent-driven) |
| POST | `/api/prds/{slug}/promote` | Promote draft to published |
| POST | `/api/prds/{slug}/plan` | Generate implementation plan |

#### Agents

| Method | Path | What It Does |
|--------|------|-------------|
| GET | `/api/agents` | List all agents (aggregated from sidecars) |
| POST | `/api/agents/create` | Create + start agent |
| GET | `/api/agents/{id}` | Get agent info |
| POST | `/api/agents/{id}/stop` | Stop agent |
| POST | `/api/agents/{id}/message` | Send message to agent |
| GET | `/api/agents/topology` | Agent dependency graph |

#### Learning

| Method | Path | What It Does |
|--------|------|-------------|
| GET | `/api/learn/efficiency` | Efficiency events (cost/tokens/latency per turn) |
| GET | `/api/learn/cascade-router` | Cascade router snapshot (model weights, trial counts) |
| GET | `/api/learn/experiments` | A/B experiment state |
| GET | `/api/learn/adaptive-thresholds` | Gate threshold EMA state |
| GET | `/api/learn/costs` | Cost breakdown |

#### Knowledge

| Method | Path | What It Does |
|--------|------|-------------|
| GET | `/api/knowledge/entries` | Knowledge store entries |
| GET | `/api/knowledge/edges` | Knowledge graph edges |
| GET | `/api/knowledge/search` | Knowledge search |
| POST | `/api/neuro/query` | Query neuro knowledge store |

#### Research

| Method | Path | What It Does |
|--------|------|-------------|
| POST | `/api/research/topic` | Deep research on topic |
| POST | `/api/research/enhance-prd/{slug}` | Enhance PRD with research |

#### Config

| Method | Path | What It Does |
|--------|------|-------------|
| GET/PUT | `/api/config` | Get / update `roko.toml` |
| POST | `/api/config/reload` | Hot-reload config from disk |

#### Providers and models

| Method | Path | What It Does |
|--------|------|-------------|
| GET | `/api/providers/` | List configured LLM providers |
| GET | `/api/models/` | List available models |
| GET | `/api/routing/explain` | Explain model routing decision |

#### Gateway / Inference

| Method | Path | What It Does |
|--------|------|-------------|
| POST | `/api/inference/complete` | Single LLM completion (through cascade router) |
| GET | `/api/gateway/stats` | Per-model token/cost counters |

#### Chain

| Method | Path | What It Does |
|--------|------|-------------|
| GET | `/api/chain/agents` | On-chain agent registry |
| GET | `/api/chain/bounties` | On-chain bounty list |
| GET | `/api/chain/status` | Chain connection status |

#### Jobs, deployments, templates, subscriptions, integrations, secrets, team

Each has its own CRUD routes (GET/POST/PUT/DELETE patterns). Total across all groups: ~115 routes.

---

## 5. The React SPA (`demo/demo-app/`)

### Project config

**package.json dependencies**:
- `react` ^19.1.0, `react-dom` ^19.1.0 — React 19
- `react-router` ^7.6.0 — Client-side routing
- `@xterm/xterm` ^5.5.0, `@xterm/addon-fit` ^0.10.0 — Browser terminal emulator
- `vite` ^6.3.4, `@vitejs/plugin-react` ^4.4.1 — Build tool
- `typescript` ^5.8.3 — Type checking

**No charting library.** All charts are hand-drawn with Canvas 2D.

**vite.config.ts**: Dev server proxies `/api`, `/ws`, and `/health` to `localhost:6677`. Build output goes to `dist/`.

### File inventory (43 source files, ~3,200 lines)

| File | Lines | Purpose |
|------|-------|---------|
| `src/main.tsx` | 10 | Entry point: renders `<App />` in StrictMode |
| `src/App.tsx` | 27 | BrowserRouter with 7 routes under `<Layout />` |
| `src/vite-env.d.ts` | 6 | TypeScript declarations for Vite and CSS modules |

#### Styles (3 files)

| File | Lines | Purpose |
|------|-------|---------|
| `src/styles/rosedust.css` | 35 | ROSEDUST design tokens — 34 CSS custom properties |
| `src/styles/fonts.css` | 2 | Font imports: Instrument Serif, JetBrains Mono, General Sans |
| `src/styles/global.css` | 40 | Reset, scrollbar, CRT scan-line overlay, film grain texture |

#### Design tokens (ROSEDUST — current, to be replaced)

```
Background layers:
  --void:        #060608  (deepest background)
  --raised:      #0C0A0E  (card surfaces)
  --mid:         #080810  (mid-level)
  --surface:     #100E14  (elevated)

Rose palette:
  --rose:        #AA7088  (primary accent)
  --rose-bright: #CC90A8  (active/highlighted)
  --rose-dim:    #7A5060  (muted/borders)
  --rose-deep:   #3A2030  (active backgrounds)
  --rose-ember:  #482838

Accent colors:
  --bone:        #C8B890  (primary value text, warm off-white)
  --sage:        #70887A  (pass/success, muted green)

Status:
  --fail:        #C36E55  (error, reddish-orange)
  --warn:        #AA8855  (warning, amber)
  --pass:        #6A9870  (pass, green)

Ethereum:
  --eth-blue:    #627EEA

Glass morphism:
  --glass-bg:    rgba(255,255,255,0.03)
  --glass-border: rgba(255,255,255,0.07)

Text:
  --text:        #B0A0B0  (default, mauve-gray)
  --text-dim:    #706070  (secondary)
  --text-ghost:  #302830  (barely visible)

Typography:
  --font-serif:  'Instrument Serif', Georgia, serif
  --font-sans:   'General Sans', 'Inter', system-ui, sans-serif
  --font-mono:   'JetBrains Mono', 'Geist Mono', 'SF Mono', monospace

Effects:
  --bone-bloom:  0 0 12px rgba(200,184,144,0.4), 0 0 32px rgba(200,184,144,0.15)
  --rose-glow:   0 0 12px rgba(170,112,136,0.3), 0 0 24px rgba(170,112,136,0.1)
  --ease:        cubic-bezier(0.22, 1, 0.36, 1)
```

#### Lib files (3 files)

| File | Lines | Purpose |
|------|-------|---------|
| `src/lib/serve-url.ts` | 11 | Resolves `SERVE_URL` and `WS_BASE` at runtime. In dev: uses localhost:6677. In production: uses the current origin. |
| `src/lib/rosedust-theme.ts` | 26 | xterm.js theme object mapping ROSEDUST colors to ANSI palette. Background #0e0c10, foreground #a58e9e, cursor #b97894. |
| `src/lib/demo-scenarios.ts` | 123 | 7 hardcoded demo scenarios with step definitions and pane configurations. ALL data is static — no API calls. |

#### Hooks (3 files)

| File | Lines | Purpose |
|------|-------|---------|
| `src/hooks/useApi.ts` | 23 | Fetch wrapper: `get(path)` and `post(path, body)` with `SERVE_URL` base. Returns parsed JSON. |
| `src/hooks/useServerHealth.ts` | 26 | Polls `GET /health` every 5s. Returns `'connected' | 'disconnected' | 'checking'`. |
| `src/hooks/useTerminal.ts` | 122 | Full xterm.js lifecycle: creates Terminal → POST to create PTY session → WebSocket connection → bidirectional I/O → ResizeObserver for responsive fit. |

#### Components (8 components + CSS)

| Component | Lines | Props | Purpose |
|-----------|-------|-------|---------|
| `Layout.tsx` | 46 | (none) | Shell: topbar with 7 nav tabs + status dot + `<Outlet />` |
| `StatCard.tsx` | 18 | `{label, value, sub?, color?}` | Single metric card with large monospace value |
| `GateBar.tsx` | 30 | `{gates: {name, status}[]}` | Horizontal pass/fail indicators (✓ ✗ ○ –) |
| `CommandLog.tsx` | 33 | `{entries: {ts, text, type?}[]}` | Auto-scrolling log (**unused — not imported anywhere**) |
| `Timeline.tsx` | 27 | `{steps: {label, status, detail?}[]}` | Vertical step timeline (done/active/pending) |
| `BarChart.tsx` | 91 | `{data: {label, value, color?}[]}` | Canvas 2D horizontal bar chart, DPR-aware |
| `CostChart.tsx` | 110 | `{data: {label, value}[]}` | Canvas 2D cumulative line chart, DPR-aware |
| `TerminalPane.tsx` | 23 | `{sessionId, label?}` | Single xterm pane with status header |
| `TerminalGrid.tsx` | 19 | `{sessions: {id, label?}[]}` | Multi-column grid of TerminalPanes |

#### Pages (7 pages + CSS)

| Page | Lines | Data Source | Real vs Fake |
|------|-------|------------|-------------|
| `Home.tsx` | 71 | Hardcoded links | Fake (static nav page) except health dot |
| `Demo.tsx` | 134 | `demo-scenarios.ts` | **Mostly fake**: terminals are real (WebSocket PTY), but stats (model, cost, tokens, time) are randomly generated. Canvas viz is an empty placeholder. |
| `Terminal.tsx` | 45 | User interaction | **Real**: creates real PTY sessions via WebSocket |
| `Builder.tsx` | 129 | `POST /api/run` | **Mixed**: API call is real, but gate results are hardcoded (always all-pass on success, all-fail on error). No polling for completion. |
| `Explorer.tsx` | 235 | `GET /api/health`, `/api/status`, `/api/episodes`, `/api/statehub/events` | **Real**: all 4 endpoints return live data |
| `Bench.tsx` | 293 | `GET /api/learn/*`, `POST /api/run`, `GET /api/run/{id}/status` | **Real**: submits runs, polls status, reads efficiency/router data |
| `BenchLive.tsx` | 181 | `GET /api/learn/efficiency`, `/api/learn/cascade-router` | **Mixed**: polls real data every 5s, but runs a parallel simulation (75% pass rate, random costs) that overwrites when real data is unavailable |

### Router structure

```
Path           → Component   → Nav label
/              → Home        → Home (0)
/demo          → Demo        → Demo (1)
/terminal      → Terminal    → Terminal (2)
/builder       → Builder     → Builder (3)
/explorer      → Explorer    → Explorer (4)
/bench         → Bench       → Bench (5)
/bench-live    → BenchLive   → Live (6)
```

### API endpoints actually used by the frontend

Only 11 of the ~115 backend endpoints are currently used:

| Endpoint | Used By | Method |
|----------|---------|--------|
| `/health` | `useServerHealth` (Layout, Home) | GET (polled every 5s) |
| `/api/health` | Explorer | GET |
| `/api/status` | Explorer | GET |
| `/api/episodes` | Explorer | GET |
| `/api/statehub/events` | Explorer | GET |
| `/api/learn/cascade-router` | Bench, BenchLive | GET |
| `/api/learn/efficiency` | Bench, BenchLive | GET |
| `/api/run` | Bench, Builder | POST |
| `/api/run/{id}/status` | Bench | GET (polled every 2s) |
| `/api/terminal/sessions` | `useTerminal` | POST |
| `/ws/terminal/{id}` | `useTerminal` | WebSocket |

---

## 6. The Build Pipeline (How React Becomes Part of the Rust Binary)

### Development workflow

```
Terminal 1: roko serve                          # Rust backend on :6677
Terminal 2: cd demo/demo-app && npm run dev     # Vite dev server on :5173

Browser → http://localhost:5173
  → React app loads from Vite
  → /api/* proxied to localhost:6677 (via vite.config.ts proxy)
  → /ws/* proxied via WebSocket
  → Hot Module Replacement: edit React → instant browser update
```

### Production build

```
cargo build -p roko-serve
  │
  ├── build.rs runs (crates/roko-serve/build.rs):
  │   │
  │   ├── Checks: does demo/demo-app/package.json exist?
  │   │   No → skip (allows building without Node.js)
  │   │   Yes → continue
  │   │
  │   ├── Checks: is SKIP_FRONTEND_BUILD env var set?
  │   │   Yes → skip
  │   │   No → continue
  │   │
  │   ├── Checks: does demo/demo-app/node_modules/ exist?
  │   │   No → runs `npm install` in demo/demo-app/
  │   │   Yes → skip install
  │   │
  │   ├── Runs `npm run build` in demo/demo-app/
  │   │   (which runs: tsc -b && vite build)
  │   │   Output: demo/demo-app/dist/ (optimized static files)
  │   │
  │   └── Declares rerun-if-changed triggers:
  │       - demo/demo-app/src/
  │       - demo/demo-app/index.html
  │       - demo/demo-app/package.json
  │       - demo/demo-app/vite.config.ts
  │       - demo/demo-app/tsconfig.json
  │
  └── Rust compilation:
      │
      └── embedded.rs uses rust-embed:
          #[derive(rust_embed::Embed)]
          #[folder = "../../demo/demo-app/dist/"]
          struct DemoAssets;

          At compile time, rust-embed reads every file in dist/
          and bakes them into the binary as byte arrays.

          At runtime, serve_embedded() handles all requests:
          1. Try exact file match (e.g., /assets/index-abc123.js)
          2. If no match → serve index.html (SPA fallback)
          3. MIME type from file extension
          4. Cache-Control:
             - /assets/* → immutable, 1 year (hashed filenames)
             - everything else → no-cache
```

### Result

```
cargo run -p roko-cli -- serve
  → Single binary serves:
    - React SPA at / (from embedded assets)
    - ~115 API routes at /api/*
    - WebSocket terminals at /ws/terminal/*
    - SSE events at /api/events
    - No Node.js, no npm, no external files needed
```

---

## 7. The ACP Protocol (Editor Integration)

**Crate**: `crates/roko-acp/`

ACP (Agent Client Protocol) enables editors like VS Code, JetBrains, Zed, and Neovim to use Roko as a coding agent backend. Communication is JSON-RPC 2.0 over stdin/stdout.

### How it works

```
Editor (VS Code / JetBrains / Zed)
    │ Spawns: roko acp --workdir /path/to/project
    │ Communicates via: stdin/stdout (newline-delimited JSON-RPC)
    ▼
roko-acp handler
    │
    ├── initialize → handshake, returns capabilities
    ├── session/new → creates a session (UUID)
    ├── session/prompt → submits user message
    │       │
    │       ├── Runs pipeline state machine:
    │       │   Express: implementer only
    │       │   Standard: implementer + gates
    │       │   Full: strategist → implementer → gates → reviewer → commit
    │       │
    │       └── Streams session/update notifications:
    │           - agent_message_chunk (text output)
    │           - agent_thought_chunk (reasoning trace)
    │           - tool_call (file edit, terminal command, etc.)
    │           - tool_call_update (status changes)
    │           - plan (ordered task list)
    │           - usage_update (tokens/cost)
    │
    ├── session/cancel → cooperative cancellation
    ├── session/list → all sessions
    └── session/load → resume persisted session
```

### Pipeline phases

```
Pending → Strategizing → Implementing → AutoFixing → Gating → Reviewing → Committing → Complete
                                                                                      → Halted
                                                                                      → Cancelled
```

Sessions persist to `.roko/acp/sessions/{id}.json`. Old sessions are garbage collected after 7 days.

---

## 8. The `roko chat` Command (Interactive REPL)

**File**: `crates/roko-cli/src/chat_inline.rs`

A Claude Code-like interactive chat REPL using ratatui for the terminal UI.

### Features
- 40 slash commands with fuzzy autocomplete
- Ctrl+R reverse search through input history
- Ctrl+K command palette (fuzzy search over commands and descriptions)
- Running token/cost meter
- Streaming token display
- Mid-session model switching (`/model`, `/switch`)
- Falls back to line-oriented REPL when stdout is not a TTY

### Session phases
```
Input → Thinking → Streaming → [Error → retry/switch/quit] → Done
```

---

## 9. How Everything Connects

```
┌─────────────────────────────────────────────────────────────────┐
│                        USER INTERFACES                          │
├─────────┬──────────┬─────────────┬──────────────┬──────────────┤
│  CLI    │  TUI     │  Web SPA    │  Editor/ACP  │  HTTP API    │
│ roko run│ roko dash│  React app  │  VS Code etc │  curl/SDK    │
│ roko chat│         │  (embedded) │  JSON-RPC    │              │
└────┬────┴────┬─────┴──────┬──────┴──────┬───────┴──────┬───────┘
     │         │            │             │              │
     ▼         ▼            ▼             ▼              ▼
┌─────────────────────────────────────────────────────────────────┐
│                      RUNTIME LAYER                              │
│                                                                 │
│  ┌──────────────┐  ┌──────────────┐  ┌────────────────────────┐│
│  │ PromptComposer│  │ CascadeRouter│  │ ProcessSupervisor     ││
│  │ 9-layer build │  │ Thompson/    │  │ agent lifecycle mgmt  ││
│  │ sections,     │  │ LinUCB bandit│  │ start/stop/health     ││
│  │ templates     │  │ model routing│  │                       ││
│  └──────┬───────┘  └──────┬───────┘  └────────┬──────────────┘│
│         │                 │                     │               │
│         ▼                 ▼                     ▼               │
│  ┌────────────────────────────────────────────────────────────┐│
│  │                   AGENT DISPATCH                           ││
│  │  8+ backends: Claude CLI, Anthropic API, Ollama, Codex,   ││
│  │  OpenAI-compat, Gemini, Perplexity, generic subprocess    ││
│  │  + MCP config passthrough + Tool loop (19 built-in tools) ││
│  └────────────────────────┬───────────────────────────────────┘│
│                           │                                     │
│                           ▼                                     │
│  ┌────────────────────────────────────────────────────────────┐│
│  │                   GATE PIPELINE                            ││
│  │  11 gates across 7 rungs:                                  ││
│  │  compile → test → clippy → shell → diff review →          ││
│  │  LLM judge → integration test → regression                ││
│  │  Adaptive thresholds (EMA-based)                           ││
│  └────────────────────────┬───────────────────────────────────┘│
│                           │                                     │
│                           ▼                                     │
│  ┌──────────────┐  ┌──────────────┐  ┌────────────────────────┐│
│  │ EpisodeLogger│  │ NeuroStore   │  │ Daimon (Affect Engine) ││
│  │ .roko/       │  │ knowledge    │  │ PAD vectors modulate   ││
│  │ episodes.jsonl│  │ with decay   │  │ dispatch decisions     ││
│  └──────────────┘  └──────────────┘  └────────────────────────┘│
│                                                                 │
│  ┌────────────────────────────────────────────────────────────┐│
│  │                  FILE SUBSTRATE                            ││
│  │  .roko/engrams.jsonl — all signals as content-addressed    ││
│  │  JSONL. Atomic writes. GC. Blake3 hashing.                 ││
│  └────────────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────────────┘
                           │
                           ▼
┌─────────────────────────────────────────────────────────────────┐
│                     CHAIN LAYER (optional)                       │
│  Nunchi L1 chain or mirage-rs local simulator                   │
│  ERC-8004 identities, knowledge publications, ZK-HDC proofs    │
│  alloy Rust Ethereum client                                     │
└─────────────────────────────────────────────────────────────────┘
```

---

## 10. Configuration (`roko.toml`)

The runtime is configured via `roko.toml` in the project root. Key sections:

```toml
[prompt]
role = "You are a senior Rust engineer..."
files = [{ path = "CONTEXT.md", priority = "high" }]

[[gates]]
type = "compile"
build_system = "cargo"

[[gates]]
type = "test"
build_system = "cargo"

[[gates]]
type = "clippy"
build_system = "cargo"

[agent]
clean_output = true
mcp_config = ".roko/mcp.json"

[serve]
port = 6677
cors_origins = ["http://localhost:5173"]

[serve.auth]
enabled = false

[chain]
rpc_url = "http://localhost:8545"  # mirage-rs or real chain

[learning_config]
replan_on_gate_failure = true
```

---

## 11. CLI Command Reference

The `roko` binary (from `roko-cli`) has these subcommands:

| Command | What It Does |
|---------|-------------|
| `roko init` | Create `.roko/` directory and default `roko.toml` |
| `roko run "<prompt>"` | Execute the universal loop: compose → agent → gate → persist |
| `roko status` | Show workspace status (signals, episodes, active plans) |
| `roko doctor` | Diagnose workspace health |
| `roko serve` | Start HTTP server on :6677 |
| `roko chat` | Interactive Claude Code-like REPL |
| `roko dashboard` | Ratatui TUI (F1-F7 tabs) |
| `roko plan list/show/create/run/generate/validate` | Plan management and execution |
| `roko prd idea/list/draft/plan/consolidate` | PRD lifecycle |
| `roko agent create/start/stop/list/status/serve/chat` | Agent management |
| `roko research topic/search/enhance-prd/plan/tasks/analyze` | Research tools |
| `roko knowledge query/stats/gc/backup/restore/sync/dream` | Knowledge store |
| `roko learn all/router/experiments/efficiency/episodes` | Learning inspection |
| `roko config init/show/validate/set/providers/models/subscriptions` | Configuration |
| `roko deploy railway/fly/docker` | Cloud deployment |
| `roko replay <hash>` | Walk signal DAG |
| `roko index build/search/stats` | Code intelligence |

---

*This document provides the complete technical context needed to understand the Nunchi codebase. Cross-references: DEMO-STRATEGY.md, DEMO-VISUAL-SPEC.md, DEMO-FLOW.md, DEMO-COMPETITIVE.md, DEMO-BUILD.md.*
