# PRD: Core Agent Harness MVP

## Document Information

| Field | Value |
|-------|-------|
| **Product** | tiagent core harness |
| **One-line** | A self-improving, model-agnostic coding agent that gets better the more you use it --- with optional shared learning via Celestia DA |
| **Status** | Design phase |
| **Document** | 12 of 15 in the tiagent design suite |
| **Prerequisites** | 01-vision-and-overview.md, 02-architecture.md, 03-crate-structure.md |

---

## Table of Contents

1. [Overview](#1-overview)
2. [Problem Statement](#2-problem-statement)
3. [Target Users](#3-target-users)
4. [Comparison with Existing Tools](#4-comparison-with-existing-tools)
5. [Goals and Non-Goals](#5-goals-and-non-goals)
6. [Requirements](#6-requirements)
7. [Technical Design Summary](#7-technical-design-summary)
8. [Success Metrics](#8-success-metrics)
9. [Milestones](#9-milestones)
10. [Open Questions](#10-open-questions)
11. [Dependencies](#11-dependencies)

---

## 1. Overview

tiagent is a self-improving, model-agnostic coding agent written in Rust. It competes
directly with Claude Code, Codex, and Cursor --- but with capabilities none of them offer:
the agent learns from every session, routes tasks across multiple LLM providers automatically,
validates all output through a gate pipeline before accepting it, and executes multi-step
implementation plans from PRDs to working code.

Optionally, tiagent can publish learning artifacts to Celestia's data availability (DA) layer,
enabling shared learning across agent deployments. But Celestia is not required. The core
harness works entirely standalone --- local models via Ollama, cloud models via API, no
blockchain dependency.

This document defines the **minimum viable product (MVP)** for the core agent harness. The
MVP is the smallest useful subset of tiagent --- the set of features required to run a single
agent through a complete task, validate the result, persist the trace, and recover from
failure. It does not include full Celestia DA integration, cross-agent shared learning,
IronClaw sandboxing, or any of the interoperability protocols (A2A, AITP, x402). Those are
scoped in separate PRDs (see 13-prd-celestia-native.md for Celestia-specific features).

The MVP answers one question: **can a developer install tiagent, point it at a task, and get
validated, auditable output that improves over time?** If yes, everything else can be layered
on incrementally.

For full context on what tiagent is and why it exists, see 01-vision-and-overview.md. For the
architectural foundations this PRD builds on, see 02-architecture.md.

---

## 2. Problem Statement

### 2.1 Coding agents don't learn

Your coding agent doesn't learn. Session 1,000 is identical to session 1. Every interaction
starts from zero --- the agent has no memory of what worked before, which models handled
which tasks best, or what mistakes it made last time. There is no feedback loop between
past performance and future behavior.

### 2.2 You're locked into one model provider

Claude Code requires Anthropic. Codex requires OpenAI. Cursor has limited model options.
If a cheaper model can handle 80% of your tasks and you only need the expensive model for
the hard 20%, you have no way to express that. You pay top-tier pricing for every task,
regardless of difficulty.

### 2.3 No quality gates

The agent can generate broken code with no automated validation. It writes a function,
declares success, and you discover the compilation error manually. There is no built-in
pipeline that runs parse checks, compilation, and tests before accepting output. Quality
assurance is entirely on the developer.

### 2.4 No plan execution

Existing tools handle single tasks, not multi-step implementation plans. If you have a
feature that requires five coordinated changes across multiple files with dependency ordering,
you run five separate prompts and manage the sequencing yourself. There is no DAG-based
executor that respects task dependencies and validates each step.

### 2.5 No PRD workflow

You can't go from a spec to validated code autonomously. The workflow of "read a PRD,
generate an implementation plan, execute tasks, validate results" does not exist in any
current coding agent. Every step is manual.

### 2.6 No shared learning infrastructure

Every agent deployment is an island. When an agent learns that a particular prompt structure
produces better results, or that a specific model handles edge cases more reliably, that
knowledge stays locked in local state. There is no infrastructure for publishing learning
artifacts, discovering relevant prior trajectories, or improving from collective experience.

### 2.7 No native agent tooling for Celestia

And for developers building on Celestia --- constructing rollups, writing DA clients, or
building applications that post blobs --- there is no native agent framework at all. The
existing landscape (LangChain, CrewAI in Python; Rig in Rust; polkagent for Polkadot, ARC
for Solana) either treats blockchain as an afterthought or targets a different chain entirely.
Celestia's DA layer is uniquely well-suited for agent trace storage and shared learning, but
no framework exists to use it this way.

---

## 3. Target Users

The MVP targets four user groups, ordered by priority. The primary audience is any developer
who wants a better coding agent. Celestia integration is a differentiator, not a prerequisite.

### 3.1 Software developers who want a coding agent that improves over time

**Profile**: Any developer --- backend, frontend, infrastructure, data --- who uses AI coding
tools daily. They are frustrated that their agent never gets better, is locked to one model
provider, and produces unvalidated output.

**MVP need**: A coding agent that runs tasks with tool calling, validates output through
automated gates (compile, test), routes tasks to the best model for the job, and learns from
every session. Install, configure, run --- no blockchain knowledge required.

**Post-MVP interest**: Team-wide shared learning, custom gate rungs, plugin ecosystem.

### 3.2 Teams wanting autonomous PRD-to-code workflows

**Profile**: Engineering teams that write PRDs and want to go from spec to validated,
implemented code with minimal manual intervention. They need plan execution, not just
single-prompt chat.

**MVP need**: A plan executor that reads `tasks.toml`, respects dependency ordering, runs
each task through the agent with gate validation, and produces a complete implementation.
Crash recovery so long-running plans survive interruptions.

**Post-MVP interest**: Multi-agent coordination, parallel task execution, CI/CD integration.

### 3.3 Agent operators and research teams

**Profile**: Teams deploying autonomous agents in production, or researchers studying
self-improvement dynamics and shared learning. They care about reliability, cost control,
audit trails, and reproducibility.

**MVP need**: Crash recovery (snapshot-resume), episode logging for auditing, gate validation
for output quality, cost tracking via efficiency metrics. The ability to run controlled
experiments with different models and prompts.

**Post-MVP interest**: On-chain audit trails, HDC fingerprinting, trajectory RAG, verifiable
execution.

### 3.4 Developers building on Celestia

**Profile**: Engineers building sovereign rollups, DA clients, or Celestia ecosystem tools.
They benefit from an agent that has native Celestia tooling and can publish learning artifacts
to the DA layer.

**MVP need**: A general-purpose agent harness that works standalone, with optional Celestia
blob submit/get tools and the ability to extend via MCP.

**Post-MVP interest**: TraceCommons integration, DA-backed cross-agent learning, MCP server
mode, A2A interoperability.

---

## 4. Comparison with Existing Tools

tiagent's MVP is a standalone coding agent. Here is how it compares to the tools developers
use today:

| Feature | tiagent MVP | Claude Code | Codex | Cursor |
|---------|-------------|-------------|-------|--------|
| Self-improvement | Yes --- CascadeRouter learns model routing; adaptive gate thresholds tune over time; efficiency metrics track per-turn performance | No | No | No |
| Model agnostic | Yes --- Claude API, Ollama, OpenAI-compat, more via Backend trait | No (Anthropic only) | No (OpenAI only) | Limited |
| Plan execution | Yes --- DAG-based executor with dependency ordering and per-task gate validation | No | No | No |
| Quality gates | Yes --- parse, compile, test rungs run automatically before output is accepted | No | No | No |
| PRD workflow | Yes --- draft PRD, generate plan, execute to validated code | No | No | No |
| Crash recovery | Yes --- snapshot-resume from any interruption point | No | No | No |
| Episode logging | Yes --- structured JSONL with every turn, tool call, gate result, model, token count | No | No | No |
| MCP extensibility | Yes --- connect any MCP-compatible tool server | Yes | No | Yes |
| Open source | Yes | Partial | Yes | No |
| Celestia DA integration | Optional (P1) | No | No | No |

The key differentiator: **tiagent gets better the more you use it.** Every other tool on this
list treats session N identically to session 1.

---

## 5. Goals and Non-Goals

### 5.1 Goals

The MVP must deliver these capabilities. Each goal maps to a concrete user action that must
work end-to-end. Goals are ordered by developer impact --- the first things a user will
interact with come first.

| # | Goal | User-visible behavior |
|---|------|----------------------|
| G1 | Working CLI that runs coding tasks | `tiagent run "write a hello world"` produces correct, validated output with tool calling |
| G2 | Multiple LLM backends (model agnosticism) | User can switch between Claude API, Ollama, and at least one other backend via config --- no vendor lock-in |
| G3 | Gate pipeline for automated quality validation | Output is validated through parse, compile, and test rungs before being accepted. Broken code is caught, not shipped |
| G4 | PRD workflow | `tiagent prd draft` -> `tiagent prd plan` -> `tiagent plan run` takes a spec from draft to validated implementation |
| G5 | Plan execution with dependency-aware dispatch | Given a `tasks.toml` with ordered tasks, the executor runs each through the agent, respects dependencies, and validates each step |
| G6 | Self-improvement (cascade routing, efficiency tracking) | CascadeRouter learns which models handle which tasks best. Efficiency metrics track cost/performance per turn. The agent gets better over time |
| G7 | Episode logging for audit trail | Every agent turn, tool call, and gate result is recorded in `.tiagent/episodes.jsonl` |
| G8 | Snapshot-resume for crash recovery | An interrupted run can be resumed from `.tiagent/state/executor.json` |
| G9 | MCP client for tool extensibility | User can connect any MCP-compatible tool server and the agent discovers and calls its tools |
| G10 | Universal loop execution | Every task flows through query, score, route, compose, act, verify, persist, react |
| G11 | Local substrate for signal persistence | Signals are persisted to local JSONL files with content-addressed hashing |
| G12 | Configuration via TOML | All agent behavior is configurable through `tiagent.toml` |

### 5.2 P1 Goals (first follow-up, not MVP)

These features are important and expected shortly after MVP, but are not required for the
core coding agent to be useful.

| Item | Rationale |
|------|-----------|
| Basic Celestia integration (blob submit/get) | The core harness works standalone; Celestia tools are optional and additive. See 13-prd-celestia-native.md |
| Adaptive gate thresholds | EMA-based threshold tuning improves gates over time but is not required for initial quality validation |
| Third LLM backend | Claude API + Ollama cover MVP; additional providers (OpenAI, Gemini) follow |

### 5.3 Non-Goals for MVP

These are explicitly out of scope. They are important features, but they are not required
for the MVP to be useful. Each is tagged with where it is scoped instead.

| Item | Why deferred | Scoped in |
|------|-------------|-----------|
| Full TraceCommons integration | Requires trace quality scoring infrastructure; MVP can log traces without scoring them | 13-prd-celestia-native.md |
| IronClaw WASM sandboxing | Requires IronClaw runtime; MVP can run tools natively | 08-ironclaw-integration.md |
| TEE execution | Requires TEE hardware and attestation infrastructure | 08-ironclaw-integration.md |
| A2A protocol support | Multi-agent interop is post-MVP | 09-interop-protocols.md |
| AITP protocol support | Near ecosystem interop is post-MVP | 09-interop-protocols.md |
| x402 paid API access | Agent commerce is post-MVP | 09-interop-protocols.md |
| HTTP control plane | CLI-first for MVP; HTTP API is a separate crate | Post-MVP |
| Interactive TUI | Terminal dashboard is post-MVP | Post-MVP |
| Sleep-time consolidation | Offline learning requires a running daemon | 11-self-improving-loop.md |
| Cross-agent learning via DA | Requires DA storage patterns + TraceCommons | 13-prd-celestia-native.md |
| Multi-agent coordination | Requires coordination protocols + namespace subscriptions | Post-MVP |
| Plugin system | Extensibility via plugins is post-MVP; MCP covers tool extensibility | Post-MVP |

---

## 6. Requirements

Requirements are organized by priority. P0 items must ship in the MVP. P1 items should ship
if time allows and are expected in the first follow-up release. P2 items are desirable but
can wait.

### 6.1 P0 --- Must Have

These are blocking requirements. The MVP is not shippable without every P0 item working
end-to-end.

#### CLI and Configuration

| ID | Requirement | Acceptance Criteria |
|----|------------|---------------------|
| P0-01 | `tiagent init` creates `.tiagent/` directory and default `tiagent.toml` | Running `tiagent init` in any directory creates `.tiagent/` with `episodes.jsonl`, `state/`, and a valid `tiagent.toml` with documented defaults |
| P0-02 | `tiagent run "<prompt>"` executes a single prompt through the universal loop | Running `tiagent run "create a Rust function that adds two numbers"` dispatches to an LLM, executes tool calls, runs gate validation, persists results, and prints output |
| P0-03 | `tiagent status` reports agent state | Running `tiagent status` shows: number of signals, number of episodes, last run timestamp, active LLM backend, and any errors |
| P0-04 | `tiagent.toml` configuration file | Config supports: LLM backend selection, model specification, API key references (env vars, not plaintext), tool enable/disable, gate rung selection, MCP server list |

#### Signal and Substrate

| ID | Requirement | Acceptance Criteria |
|----|------------|---------------------|
| P0-05 | Signal type with content-addressing | `Signal` struct has: SHA-256 hash, kind enum, payload (JSON/bytes), score (f64), metadata (timestamps, source), optional DA reference |
| P0-06 | FileSubstrate (JSONL) for local persistence | Signals are written to `.tiagent/signals.jsonl` with one JSON object per line. Reads support filtering by kind, time range, and hash prefix |

#### LLM Dispatch

| ID | Requirement | Acceptance Criteria |
|----|------------|---------------------|
| P0-07 | Claude API backend | Agent can dispatch prompts to Claude (Sonnet 4 or Haiku) via Anthropic's HTTP API. Supports streaming responses, tool use, and system prompts |
| P0-08 | Ollama backend | Agent can dispatch prompts to a local Ollama instance. Supports tool use via Ollama's tool calling API. Model is configurable |
| P0-09 | Backend trait abstraction | A `Backend` trait defines the interface: `send(prompt, tools, config) -> Response`. Adding a new backend requires implementing this trait only |

#### Tool System

| ID | Requirement | Acceptance Criteria |
|----|------------|---------------------|
| P0-10 | Tool loop with structured calling | Agent sends tool definitions to the LLM, receives tool call requests, executes them, and returns results in a loop until the LLM produces a final response |
| P0-11 | Built-in tools: file read | `file_read(path) -> content` reads a file and returns its contents. Respects path restrictions if configured |
| P0-12 | Built-in tools: file write | `file_write(path, content) -> result` writes content to a file. Creates parent directories if needed |
| P0-13 | Built-in tools: bash | `bash(command) -> output` executes a shell command and returns stdout/stderr. Supports timeout and working directory configuration |
| P0-14 | Built-in tools: search | `search(pattern, path) -> matches` searches for a regex pattern in files. Returns matching lines with file paths and line numbers |
| P0-15 | MCP client integration | Agent can connect to MCP servers listed in `tiagent.toml`, discover their tools, and call them during the tool loop. Uses the `mcp-sdk` crate |

#### Gate Pipeline

| ID | Requirement | Acceptance Criteria |
|----|------------|---------------------|
| P0-16 | Parse rung | Validates that LLM output is well-formed (valid JSON for structured output, valid code for code generation). Fails fast on parse errors |
| P0-17 | Compile rung | Runs `cargo build` (or equivalent for the target language) and checks for compilation errors. Reports specific error messages on failure |
| P0-18 | Test rung | Runs `cargo test` (or equivalent) and checks for test failures. Reports which tests failed and why |
| P0-19 | Gate pipeline orchestration | Rungs execute in order (parse, compile, test). A failure at any rung stops the pipeline and reports the failing rung. Gate results are included in episode logs |

#### Logging and Recovery

| ID | Requirement | Acceptance Criteria |
|----|------------|---------------------|
| P0-20 | Episode logging | Every agent turn is recorded as a structured JSON entry in `.tiagent/episodes.jsonl`. Each entry includes: timestamp, turn number, prompt sent, response received, tool calls made, tool results, gate results, model used, token counts |
| P0-21 | Snapshot-resume | Executor state is persisted to `.tiagent/state/executor.json` after each completed step. Running `tiagent run --resume .tiagent/state/executor.json` restores state and continues from the last completed step |

### 6.2 P1 --- Should Have

These features significantly improve the MVP experience and are expected in the first
follow-up release. They should be included in the MVP if implementation time allows.

#### Plan Execution

| ID | Requirement | Acceptance Criteria |
|----|------------|---------------------|
| P1-01 | `tiagent plan run <dir>` executes multi-task plans | Given a directory containing a `tasks.toml` with ordered tasks and dependencies, the executor runs each task through the universal loop, respects dependency ordering, and reports overall plan status |
| P1-02 | `tasks.toml` format | Tasks have: id, title, description, dependencies (list of task ids), agent role, gate requirements. The format matches the structure defined in 02-architecture.md |

#### Model Routing

| ID | Requirement | Acceptance Criteria |
|----|------------|---------------------|
| P1-03 | CascadeRouter | Routes tasks through models in order of increasing cost/capability. Starts with the cheapest configured model; escalates to more expensive models on failure. Routing decisions are logged |
| P1-04 | Routing state persistence | Router weights and escalation history are persisted to `.tiagent/learn/cascade-router.json`. State survives restarts |

#### Prompt Composition

| ID | Requirement | Acceptance Criteria |
|----|------------|---------------------|
| P1-05 | Template-based prompt composer | System prompts are assembled from composable template layers (role definition, task context, tool instructions, output format, constraints). Templates are defined as text files in a configurable directory |
| P1-06 | Context enrichment | The composer can inject relevant context into the system prompt: recent episode summaries, task-specific documentation, project-level instructions |

#### Celestia Integration

| ID | Requirement | Acceptance Criteria |
|----|------------|---------------------|
| P1-07 | Blob submit tool | `celestia_blob_submit(namespace, data) -> tx_hash` submits data as a blob to Celestia. Supports Mocha testnet and mainnet. Returns transaction hash and blob commitment |
| P1-08 | Blob get tool | `celestia_blob_get(namespace, height) -> data` retrieves blobs from a Celestia namespace at a given height. Returns decoded blob data |

#### Additional Backends and Learning

| ID | Requirement | Acceptance Criteria |
|----|------------|---------------------|
| P1-09 | Third LLM backend | At least one additional backend beyond Claude API and Ollama (OpenAI API, Gemini, or another provider). Implements the same `Backend` trait |
| P1-10 | Adaptive gate thresholds | Gate pass/fail thresholds adjust over time using exponential moving averages. Thresholds are persisted to `.tiagent/learn/gate-thresholds.json` |
| P1-11 | Efficiency metrics logging | Per-turn metrics (token count, latency, cost estimate, tool call count) are logged to `.tiagent/learn/efficiency.jsonl` |

### 6.3 P2 --- Nice to Have

These features are valuable but not required for the MVP or immediate follow-up.

| ID | Requirement | Acceptance Criteria |
|----|------------|---------------------|
| P2-01 | `tiagent doctor` workspace health check | Reports: Rust toolchain version, configured backends and their reachability, MCP server connectivity, Celestia node connectivity, config validation results |
| P2-02 | OpenAI-compatible backend | A generic backend that works with any OpenAI-compatible API endpoint (Together, Fireworks, Groq, vLLM, etc.) via configurable base URL |
| P2-03 | Basic TUI for task progress | A minimal terminal display showing: current task, progress through plan, gate results, token usage. Not the full ratatui dashboard --- just enough to watch a run |
| P2-04 | MCP server mode | Expose tiagent's built-in tools (including Celestia tools) as an MCP server that other clients (Claude Desktop, Cursor) can connect to |
| P2-05 | `tiagent plan validate <dir>` | Lint a `tasks.toml` file without executing it: check for dependency cycles, missing fields, unknown gate rungs, and other structural errors |

---

## 7. Technical Design Summary

This section summarizes the technical approach. Full architectural details are in
02-architecture.md and 03-crate-structure.md.

### 7.1 Workspace Structure

The MVP workspace contains approximately 12--14 Rust crates organized in four layers:

```
tiagent/
  crates/
    tiagent-core/        # Signal, 6 verb traits, errors, config types
    tiagent-agent/       # LLM backends, tool loop, dispatch
    tiagent-gate/        # Gate pipeline, rung implementations
    tiagent-compose/     # Prompt templates, context assembly
    tiagent-learn/       # Episodes, routing state, experiments
    tiagent-tools/       # Built-in tools, MCP client
    tiagent-celestia/    # Celestia RPC, blob tools, namespace mgmt
    tiagent-fs/          # FileSubstrate, JSONL, layout
    tiagent-runtime/     # Process supervision, event bus, cancellation
    tiagent-cli/         # CLI binary, subcommands, config loading
```

Optional post-MVP crates: `tiagent-serve` (HTTP API), `tiagent-tui` (terminal UI),
`tiagent-ironclaw` (WASM/TEE), `tiagent-interop` (A2A/AITP/x402).

### 7.2 Core Pattern

The architectural foundation is **1 noun + 6 verb traits**:

- **Noun**: `Signal` --- a content-addressed, typed, scored datum. Every piece of information
  flowing through tiagent is a Signal.
- **Verbs**: `Substrate` (persist/retrieve), `Scorer` (evaluate priority), `Gate` (validate
  results), `Router` (select model/strategy), `Composer` (assemble prompts), `Policy`
  (define reactions).

Every agent execution follows the universal loop:

```
query -> score -> route -> compose -> act -> verify -> write -> react
```

### 7.3 Technology Choices

| Component | Choice | Rationale |
|-----------|--------|-----------|
| Async runtime | Tokio | Industry standard for Rust async; required by Celestia client libraries |
| CLI framework | clap (derive) | Type-safe argument parsing with subcommand support; generates shell completions |
| Configuration | TOML via `serde` + `toml` | Human-readable, well-supported in Rust ecosystem, matches Cargo.toml conventions |
| Persistence | JSONL files in `.tiagent/` | Simple, debuggable, appendable; no database dependency for MVP |
| LLM dispatch | Trait-based backends | `Backend` trait with concrete implementations per provider; new backends in ~100 lines |
| Tool schemas | JSON Schema via `schemars` | Standard format; compatible with MCP and LLM tool calling APIs |
| MCP integration | `mcp-sdk` crate | Official Rust SDK for Model Context Protocol |
| HTTP client | `reqwest` | Standard Rust HTTP client; used for LLM API calls and Celestia RPC |
| Serialization | `serde` + `serde_json` | Universal in Rust; required by every dependency |
| Hashing | SHA-256 via `sha2` | Content-addressing for Signals; standard, fast, well-supported |
| Celestia client | `celestia-types` + `celestia-rpc` | Official Celestia Rust SDK; provides typed blob/namespace operations |

### 7.4 Data Layout

The `.tiagent/` directory structure for the MVP:

```
.tiagent/
  tiagent.toml              # User configuration (also valid at project root)
  signals.jsonl             # Signal persistence (FileSubstrate)
  episodes.jsonl            # Agent turn logs (episode logger)
  state/
    executor.json           # Snapshot for resume (latest executor state)
  learn/
    cascade-router.json     # Model routing weights (P1)
    gate-thresholds.json    # Adaptive gate thresholds (P1)
    efficiency.jsonl        # Per-turn cost/performance metrics (P1)
```

---

## 8. Success Metrics

The MVP is successful when all of the following can be demonstrated in a live run (not just
unit tests). Each metric maps to a specific user workflow. Developer-facing metrics come first.

### 8.1 Core Developer Experience

| # | Metric | How to verify |
|---|--------|--------------|
| SM-1 | Single-prompt execution produces working, tested code | Run `tiagent run "write a Rust function that reverses a string"`. The output compiles, tests pass, and the function is correct |
| SM-2 | Gate pipeline catches broken code | Give the agent a task that produces code with a compilation error. The compile gate catches it and reports the error --- broken code is never silently accepted |
| SM-3 | Multiple backends work (no vendor lock-in) | Run the same task with Claude API and Ollama (configured via `tiagent.toml`). Both produce valid output that passes gates |

### 8.2 Plan Execution

| # | Metric | How to verify |
|---|--------|--------------|
| SM-4 | Multi-task plan runs to completion with gate validation | Create a `tasks.toml` with 5 dependent tasks. Run `tiagent plan run <dir>`. All 5 tasks complete with dependency ordering and gate validation at each step |
| SM-5 | Crash recovery works | Start a 5-task plan run, kill the process after task 3. Run `tiagent run --resume .tiagent/state/executor.json`. Tasks 4 and 5 execute; tasks 1-3 are skipped |

### 8.3 Self-Improvement

| # | Metric | How to verify |
|---|--------|--------------|
| SM-6 | CascadeRouter demonstrably improves model selection | After 20+ runs, the router's persisted weights in `.tiagent/learn/cascade-router.json` show differentiated routing --- not uniform. Cheaper models handle easy tasks; expensive models are reserved for hard tasks |
| SM-7 | Episode log captures all agent turns and is searchable | After a multi-turn run, `.tiagent/episodes.jsonl` contains entries for every turn, including tool calls, gate results, model used, and token counts |

### 8.4 Tool Extensibility

| # | Metric | How to verify |
|---|--------|--------------|
| SM-8 | Built-in tools work | Agent uses file_read, file_write, and bash tools during a task. All three produce correct results |
| SM-9 | MCP tool discovery works | Configure an MCP server in `tiagent.toml`. Agent discovers the server's tools and lists them. At least one tool is callable |

### 8.5 Celestia Integration (P1)

| # | Metric | How to verify |
|---|--------|--------------|
| SM-10 | Blob submit works on Mocha testnet | Agent calls `celestia_blob_submit` with test data. The blob appears on Mocha testnet and can be retrieved by namespace and height. This metric is P1, not required for MVP launch |

---

## 9. Milestones

The MVP is divided into six milestones. Each milestone produces a working (if incomplete)
system that can be tested end-to-end. Milestones are ordered by dependency: each builds on
the previous one.

### M1: Foundation

**Delivers**: Signal types, FileSubstrate, CLI scaffold, configuration.

| Task | Description | Crates touched |
|------|-------------|----------------|
| Define `Signal` struct | Content-addressed, typed, scored datum with serialization | `tiagent-core` |
| Implement `SignalKind` enum | Task, Response, ToolCall, ToolResult, Gate, Episode, Error, Meta | `tiagent-core` |
| Define 6 verb traits | `Substrate`, `Scorer`, `Gate`, `Router`, `Composer`, `Policy` with async trait methods | `tiagent-core` |
| Implement `FileSubstrate` | JSONL-based local persistence with read/write/query | `tiagent-fs` |
| Scaffold CLI | `tiagent init`, `tiagent status` with clap derive macros | `tiagent-cli` |
| Config loading | Parse `tiagent.toml`, validate, provide defaults | `tiagent-core`, `tiagent-cli` |

**Exit criterion**: `tiagent init` creates a valid workspace. `tiagent status` reports
"0 signals, 0 episodes."

### M2: LLM Dispatch and Tool Loop

**Delivers**: Two LLM backends, tool loop, built-in tools.

| Task | Description | Crates touched |
|------|-------------|----------------|
| Define `Backend` trait | `async fn send(prompt, tools, config) -> Response` | `tiagent-agent` |
| Claude API backend | HTTP client for Anthropic API with streaming, tool use, system prompts | `tiagent-agent` |
| Ollama backend | HTTP client for local Ollama with tool calling support | `tiagent-agent` |
| Tool loop implementation | Send tools to LLM, receive calls, execute, return results, repeat | `tiagent-agent` |
| Built-in tools | file_read, file_write, bash, search with typed schemas | `tiagent-tools` |
| Wire `tiagent run` | Connect CLI command to backend dispatch and tool loop | `tiagent-cli` |

**Exit criterion**: `tiagent run "write hello world to a file"` dispatches to Claude API,
uses file_write tool, and produces the file. Switching config to Ollama produces the same
result.

### M3: Gates, Episodes, and Recovery

**Delivers**: Gate pipeline, episode logging, snapshot-resume.

| Task | Description | Crates touched |
|------|-------------|----------------|
| Gate trait and pipeline | Ordered rung execution with short-circuit on failure | `tiagent-gate` |
| Parse rung | Validate output structure (JSON, code syntax) | `tiagent-gate` |
| Compile rung | Run `cargo build` and capture errors | `tiagent-gate` |
| Test rung | Run `cargo test` and capture failures | `tiagent-gate` |
| Episode logger | Append structured turn records to `.tiagent/episodes.jsonl` | `tiagent-learn` |
| Executor snapshot | Serialize executor state to `.tiagent/state/executor.json` after each step | `tiagent-runtime` |
| Resume from snapshot | `--resume` flag restores state and skips completed steps | `tiagent-cli`, `tiagent-runtime` |

**Exit criterion**: A run that produces code with a test failure reports the gate failure.
Killing a multi-step run and resuming picks up from the last completed step. Episodes are
queryable in the JSONL log.

### M4: MCP Client and Celestia Tools

**Delivers**: MCP client integration, Celestia blob submit/get tools.

| Task | Description | Crates touched |
|------|-------------|----------------|
| MCP client | Connect to MCP servers, discover tools, call tools during agent loop | `tiagent-tools` |
| MCP config in TOML | `[mcp.servers]` section in `tiagent.toml` with server name, command, args, env | `tiagent-core`, `tiagent-cli` |
| Celestia blob submit tool | `celestia_blob_submit(namespace, data)` via `celestia-rpc` | `tiagent-celestia` |
| Celestia blob get tool | `celestia_blob_get(namespace, height)` via `celestia-rpc` | `tiagent-celestia` |
| Celestia config | `[celestia]` section in `tiagent.toml` with RPC URL, auth token, default namespace | `tiagent-core` |
| Wire Celestia tools into agent | Register Celestia tools alongside built-in tools in the tool loop | `tiagent-agent`, `tiagent-celestia` |

**Exit criterion**: Agent can connect to an MCP server (e.g., filesystem MCP), discover its
tools, and use them in a task. Agent can submit a blob to Mocha testnet and retrieve it.

### M5: Routing, Plans, and Composition

**Delivers**: CascadeRouter, plan execution, prompt composer.

| Task | Description | Crates touched |
|------|-------------|----------------|
| CascadeRouter | Route tasks through models in cost order; escalate on failure | `tiagent-learn` |
| Router state persistence | Persist routing weights to `.tiagent/learn/cascade-router.json` | `tiagent-learn` |
| Plan executor | Parse `tasks.toml`, build dependency DAG, execute tasks in order | `tiagent-runtime`, `tiagent-cli` |
| Prompt composer | Template-based system prompt assembly with context injection | `tiagent-compose` |
| Adaptive gate thresholds | EMA-based threshold updates persisted to `.tiagent/learn/gate-thresholds.json` | `tiagent-gate`, `tiagent-learn` |
| Efficiency metrics | Per-turn metrics (tokens, latency, cost) logged to `.tiagent/learn/efficiency.jsonl` | `tiagent-learn` |

**Exit criterion**: A 5-task `tasks.toml` plan runs to completion with dependency ordering
respected. CascadeRouter starts with Haiku and escalates to Sonnet on failure. Gate
thresholds adjust after repeated runs.

### M6: MVP Release

**Delivers**: Polish, documentation, integration testing, release artifacts.

| Task | Description | Crates touched |
|------|-------------|----------------|
| Integration tests | End-to-end tests for each success metric (SM-1 through SM-9) | All |
| Error messages | Every error path produces a human-readable message with suggested fix | All |
| Default config | Sensible defaults that work out of the box (Ollama as default, local-only mode) | `tiagent-core` |
| Shell completions | `tiagent completions bash/zsh/fish` | `tiagent-cli` |
| Cargo publish prep | Workspace metadata, license, README, categories, keywords | All |
| Binary distribution | Prebuilt binaries for Linux (x86_64, aarch64) and macOS (aarch64) | CI |

**Exit criterion**: All success metrics (SM-1 through SM-9) pass. A new user can install
tiagent, run `tiagent init`, and execute `tiagent run` within 5 minutes.

---

## 10. Open Questions

These are decisions that need to be resolved before or during implementation. Each has a
current leaning and a rationale, but none are finalized.

### Q1: Configuration format

**Question**: TOML or YAML for `tiagent.toml`?

**Current leaning**: TOML. It matches Rust ecosystem conventions (Cargo.toml), is simpler to
parse, and avoids YAML's well-documented footguns (implicit typing, indentation sensitivity,
the Norway problem). The tradeoff is that TOML handles deeply nested structures less
gracefully than YAML.

### Q2: Default model

**Question**: What model should tiagent default to when no model is configured?

**Current leaning**: Ollama with a small open model (e.g., `llama3.1:8b`). This means a new
user can run tiagent without any API key by installing Ollama and pulling a model. Claude
Sonnet 4 is the recommended model for production use but requires an API key. The config
should make it easy to switch: `backend = "claude"` with `model = "claude-sonnet-4-20250514"`.

### Q3: Celestia namespace strategy

**Question**: Should namespaces be per-user, per-agent, or per-data-type?

**Current leaning**: Per-agent with a structured naming convention
(`tiagent/v1/<data-type>/<agent-id>`). This keeps each agent's data isolated while making it
discoverable. The namespace scheme is described in detail in 05-da-storage-patterns.md. The
question for the MVP is whether we need any namespace management at all, or if a single
hardcoded namespace is sufficient for blob submit/get tools.

### Q4: Celestia in MVP or P1?

**Question**: Should the MVP include Celestia blob submit/get, or should it be deferred to P1?

**Current leaning**: Include basic blob submit/get as P1 tools, not P0. The reasoning: the
core harness should work completely without a Celestia node (local-only mode). Celestia
integration is important for the product story but not for the "can I run an agent?" question
the MVP answers. Milestone M4 includes Celestia tools, but they are P1 requirements, not P0.

### Q5: Minimum Rust version

**Question**: What MSRV (minimum supported Rust version) should tiagent target?

**Current leaning**: Rust 1.80+. This is the version where `async fn` in traits was
stabilized, eliminating the need for the `async-trait` proc macro. The Celestia Rust SDK
(`celestia-types`, `celestia-rpc`) may impose additional version requirements that could push
this higher.

---

## 11. Dependencies

### 11.1 External Crate Dependencies

| Crate | Version | Used for | Required by |
|-------|---------|----------|-------------|
| `tokio` | 1.x | Async runtime | All async code |
| `serde` + `serde_json` | 1.x | Serialization | All structured data |
| `toml` | 0.8+ | Configuration parsing | `tiagent-core`, `tiagent-cli` |
| `clap` | 4.x | CLI argument parsing | `tiagent-cli` |
| `reqwest` | 0.12+ | HTTP client for LLM APIs and Celestia RPC | `tiagent-agent`, `tiagent-celestia` |
| `sha2` | 0.10+ | SHA-256 hashing for content addressing | `tiagent-core` |
| `schemars` | 0.8+ | JSON Schema generation for tool definitions | `tiagent-tools` |
| `mcp-sdk` | latest | MCP client implementation | `tiagent-tools` |
| `celestia-types` | latest | Celestia blob and namespace types | `tiagent-celestia` |
| `celestia-rpc` | latest | Celestia RPC client | `tiagent-celestia` |
| `tracing` | 0.1+ | Structured logging | All crates |
| `anyhow` / `thiserror` | latest | Error handling | All crates |
| `chrono` | 0.4+ | Timestamps for signals and episodes | `tiagent-core` |

### 11.2 Infrastructure Dependencies

| Dependency | Required for | MVP requirement? |
|------------|-------------|------------------|
| Anthropic API key | Claude backend | P0 (one of two required backends) |
| Ollama installation | Ollama backend | P0 (one of two required backends) |
| Celestia node (Mocha testnet) | Blob submit/get tools | P1 (not required for local-only mode) |
| Rust 1.80+ | Async trait stabilization | P0 |
| Internet connectivity | LLM API calls, Celestia RPC | P0 for cloud backends; not required for Ollama |

### 11.3 Related Documents

| Document | Relationship to this PRD |
|----------|-------------------------|
| 01-vision-and-overview.md | Defines the "why" behind every requirement in this PRD |
| 02-architecture.md | Defines the universal loop, Signal type, and 6 verb traits that this PRD's requirements implement |
| 03-crate-structure.md | Defines the workspace layout referenced in section 7.1 |
| 04-celestia-integration.md | Full design for Celestia integration; this PRD scopes only blob submit/get for MVP |
| 05-da-storage-patterns.md | Full design for DA storage; this PRD defers most patterns to post-MVP |
| 06-tool-system.md | Full tool system design; this PRD scopes built-in tools + MCP client for MVP |
| 11-self-improving-loop.md | Full self-improvement design; this PRD scopes episode logging + cascade router for MVP |
| 13-prd-celestia-native.md | Companion PRD scoping Celestia-native features beyond the core harness |

---

## Appendix A: Requirement Traceability

Every P0 requirement maps to at least one success metric and one milestone.

| Requirement | Success Metric | Milestone |
|-------------|---------------|-----------|
| P0-01 (tiagent init) | SM-1 | M1 |
| P0-02 (tiagent run) | SM-1 | M2 |
| P0-03 (tiagent status) | SM-1 | M1 |
| P0-04 (tiagent.toml) | SM-3 | M1 |
| P0-05 (Signal type) | SM-7 | M1 |
| P0-06 (FileSubstrate) | SM-7 | M1 |
| P0-07 (Claude API) | SM-3 | M2 |
| P0-08 (Ollama) | SM-3 | M2 |
| P0-09 (Backend trait) | SM-3 | M2 |
| P0-10 (Tool loop) | SM-8 | M2 |
| P0-11 (file_read) | SM-8 | M2 |
| P0-12 (file_write) | SM-8 | M2 |
| P0-13 (bash) | SM-8 | M2 |
| P0-14 (search) | SM-8 | M2 |
| P0-15 (MCP client) | SM-9 | M4 |
| P0-16 (Parse gate) | SM-2 | M3 |
| P0-17 (Compile gate) | SM-2 | M3 |
| P0-18 (Test gate) | SM-2 | M3 |
| P0-19 (Gate pipeline) | SM-2 | M3 |
| P0-20 (Episode logging) | SM-7 | M3 |
| P0-21 (Snapshot-resume) | SM-5 | M3 |
