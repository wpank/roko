# 06 — Roko Agent Runtime

## 1. What Roko Is

Roko is a Rust toolkit for building AI agents that build themselves. It is approximately 177,000 lines of code organized across 18 crates in a single Cargo workspace. The system's defining property is self-hosting: Roko is the tool that develops Roko. It reads product requirement documents, generates implementation plans, dispatches LLM-backed agents to execute tasks, validates results through automated gate pipelines, persists outcomes, and learns from each cycle to improve the next one.

The toolkit is not a single monolithic application. It is a layered runtime composed of independent crates that handle planning, agent dispatch, prompt construction, gate validation, learning, knowledge storage, process supervision, and chain integration. Each crate exposes Rust traits and types that compose into the full self-hosting loop. The CLI binary (`roko-cli`) is the primary entry point, but the same crate APIs back an HTTP control plane (~85 REST routes on port 6677), a per-agent HTTP sidecar, and an interactive TUI dashboard built on ratatui.

Roko replaces and succeeds an earlier system called Mori, a 108K-LOC orchestrator in the same problem space. The migration preserved Mori's architecture and patterns while consolidating into a cleaner crate graph and adding subsystems (the learning layer, the knowledge store, the affect engine) that Mori lacked.

---

## 2. The Self-Hosting Loop

Roko develops itself through a six-phase cycle. Each phase is a CLI command that exists today:

**Phase 1 — Capture.** A work item enters the system as a PRD (product requirement document). The operator runs `roko prd idea "Wire the cascade router into dispatch"` to create a raw idea, then `roko prd draft new "cascade-router-wiring"` to produce a structured draft. An LLM agent writes the draft, guided by a system prompt that includes project conventions, codebase context, and anti-patterns.

**Phase 2 — Research.** Before planning, the system can enrich the PRD with external research: `roko research enhance-prd cascade-router-wiring`. This runs a research agent (backed by Perplexity Sonar or equivalent) that gathers citations, API documentation, and related patterns, then appends structured context to the PRD.

**Phase 3 — Plan.** The command `roko prd plan cascade-router-wiring` dispatches a Strategist agent to read the PRD and generate a `tasks.toml` file. This file defines a directed acyclic graph (DAG) of tasks with dependencies, roles, domains, complexity bands, and acceptance criteria. The plan becomes the executable specification for the next phase.

**Phase 4 — Execute.** `roko plan run plans/` is the core orchestration loop. The `ParallelExecutor` reads the DAG, identifies tasks whose dependencies are satisfied, and dispatches agents in parallel. Each task is assigned to an agent role (Implementer, Auditor, Researcher, etc.), which determines the LLM backend, model tier, tool permissions, and dollar budget for that turn. Agents run as subprocesses (Claude CLI, Codex CLI, Cursor ACP, or HTTP API calls to OpenAI-compatible endpoints). Their output flows back into the executor as events.

**Phase 5 — Gate.** After each task completes, the gate pipeline validates the result. The pipeline selects a set of rungs (verification levels) based on the plan's complexity band. Rungs range from basic compilation checks (rung 1) through clippy linting, test execution, diff review, LLM-judge review, symbol verification, integration tests, and property-based tests (rungs 2-7). Each rung produces a verdict (pass/fail/skip). Gate failures can trigger automatic replanning: the system builds a revised plan that addresses the failure and re-dispatches.

**Phase 6 — Persist and Learn.** Results, episodes, and metrics are written to disk. The episode logger records every agent turn with token counts, cost, duration, model used, and gate verdict. The cascade router updates its bandit weights for model selection. Prompt experiments record A/B outcomes. Adaptive gate thresholds adjust based on exponential moving averages of pass rates per rung. The knowledge store (neuro) distills durable insights from episodes. The system is strictly better the next time it encounters a similar task.

The loop supports resumption. If interrupted, `roko plan run plans/ --resume .roko/state/executor.json` reloads the executor snapshot and continues from the last checkpoint. State is auto-saved every 5 actions.

---

## 3. Architecture: 1 Noun + 6 Verbs

Roko's type system is organized around one universal data type and six verb traits that operate on it.

### The Noun: Signal (Engram)

Every piece of data flowing through the system is an `Engram` (also referred to as a Signal in the architectural model). An Engram has:

- A **Kind** — what type of data it represents (observation, prediction, action, knowledge, episode, etc.)
- A **Body** — the payload (text, structured JSON, binary)
- A **Provenance** — where and when it was created
- A **ContentHash** — a deterministic hash of the body for deduplication and integrity
- **Tags** — arbitrary key-value metadata for filtering and routing
- A **Lineage** — links to parent Engrams, forming a DAG of derivations
- A **Decay** — a time-based relevance function that reduces weight over time

Engrams are the universal currency. Agent outputs, gate verdicts, knowledge entries, episodes, routing decisions, and prompt sections are all Engrams or are derived from Engrams.

### The 6 Verb Traits

| Trait | Role | Example Implementation |
|:---|:---|:---|
| **Substrate** | Storage and retrieval. Reads/writes Engrams to a backing store. | `FileSubstrate` — JSONL files on disk with GC and layout management |
| **Scorer** | Evaluates relevance or quality. Takes Engrams and a Context, returns scores. | `SumScorer` — sums component scores; `CatalystScorer` — factors in reuse, confirmation, and downstream impact |
| **Gate** | Binary verification. Takes an Engram and decides pass/fail with evidence. | `CompileGate`, `TestGate`, `ClippyGate`, `LlmJudgeGate`, `SymbolGate` |
| **Router** | Selects where work goes. Routes tasks to models, backends, or agents. | `CascadeRouter` — LinUCB bandit over model tiers; `select_model_for_task` — scores models against task requirements |
| **Composer** | Assembles context. Constructs prompts, combines sections, manages token budgets. | `SystemPromptBuilder` — 9-layer prompt constructor; `PromptComposer` — attention-weighted section assembly |
| **Policy** | Enforces constraints. Guards actions, budgets, and permissions. | `AgentContract` — YAML-defined constraints; `SafetyLayer` — pre/post execution checks; `BudgetGuardrail` — per-turn dollar ceilings |

### The Universal Loop

These traits compose into a single execution pattern that repeats at every level of the system:

```
query -> score -> route -> compose -> act -> verify -> write -> react
```

1. **Query**: Read relevant Engrams from the Substrate (prior outputs, knowledge entries, episodes).
2. **Score**: Rank them by relevance to the current task using a Scorer.
3. **Route**: Select the model, backend, and role using the Router.
4. **Compose**: Build the system prompt using the Composer (9-layer builder + attention bidding).
5. **Act**: Dispatch the agent to the selected backend; execute tool calls through the ToolDispatcher.
6. **Verify**: Run the gate pipeline on the output.
7. **Write**: Persist the result as new Engrams in the Substrate.
8. **React**: Update learning state (episode logger, cascade router, prompt experiments, adaptive thresholds) and potentially trigger replanning.

---

## 4. Agent Dispatch

### Backends

Roko supports seven LLM backends, each mapping to a different subprocess protocol or HTTP API:

| Backend | Protocol | Typical Models |
|:---|:---|:---|
| **Claude** | `claude` CLI subprocess (stream-JSON protocol) | claude-sonnet-4-5, claude-opus-4 |
| **Codex** | OpenAI `codex` CLI (JSON-RPC) | gpt-5, o3 |
| **Cursor** | Cursor Agent Client Protocol (ACP JSON-RPC) | composer-*, cursor-*, gemini-* |
| **Ollama** | OpenAI-compatible HTTP to local server | llama3.2, ollama/* |
| **OpenAI** | OpenAI chat completions HTTP API | gpt-5 (direct API) |
| **Perplexity** | Perplexity Sonar HTTP API | sonar-*, perplexity/* |
| **Cerebras** | OpenAI-compatible HTTP (ultra-fast inference) | cerebras/* |

Backend selection is automatic. When a task specifies a model slug (e.g., `claude-sonnet-4-5`), `AgentBackend::from_model()` infers the correct backend from the slug prefix. Slugs starting with `claude-` route to the Claude CLI; `ollama/` routes to Ollama; `sonar*` routes to Perplexity; and so on.

### Provider Resolution

The `resolve_model` function looks up a model key against the project's `roko.toml` configuration in three passes:

1. **Direct key match**: The model key matches a `[models.<key>]` entry exactly.
2. **Slug match**: The model key matches a configured model's `slug` field.
3. **Prefix match**: The model key is a prefix of a slug, separated by `-`, `.`, or `_` (so `claude-opus-4` matches `claude-opus-4-6`).

Each resolved model carries a `ProviderKind` (which protocol family), an optional `ProviderConfig` (API base URL, API key reference, headers), and an optional `ModelProfile` (context window size, supported capabilities, cost per million tokens).

### Model Tier Routing

Every `AgentRole` has a default `ModelTier`:

- **Fast** (Haiku-class): Cheap, fast inference for classification, watchers, orchestration overhead.
- **Standard** (Sonnet-class): The workhorse for implementation and review.
- **Premium** (Opus/GPT-5-class): Reserved for architecture, hard debugging, and complex reasoning.

The cascade router (`CascadeRouter` in `roko-learn`) refines this default using a LinUCB contextual bandit. It observes task outcomes (gate pass/fail, token usage, cost, latency) and learns which models perform best for which task categories and domains. Over time, the router shifts traffic toward models that produce higher gate-pass rates at lower cost.

### Task Requirements Matching

The `score_model_for_task` function evaluates model profiles against structured `TaskRequirements`:

- `needs_web_search` — requires grounding/retrieval (filters to Perplexity, Gemini with grounding)
- `needs_code_execution` — requires native code sandbox
- `needs_thinking` — benefits from extended reasoning (chain-of-thought)
- `needs_vision` — requires image analysis
- `needs_structured_output` — requires tool use or partial JSON output
- `min_context_window` — minimum token window
- `max_cost_output_per_m` — cost ceiling per million output tokens
- `max_latency_ms` — latency ceiling

Models that fail any hard requirement are filtered out. Surviving candidates are scored on a weighted combination of capability match, context window headroom, cost efficiency, and latency profile. A learned bonus from the cascade router is added before final ranking.

### Reasoning Effort

Each dispatch carries a `ReasoningEffort` hint: `Low`, `Medium`, `High`, or `Max`. Backends that support it (Claude's `--effort` flag, Codex's `reasoning_effort` parameter) pass it through; others ignore it. Conductor and watcher roles default to `Low`; Implementer defaults to `Medium`; Architect and Critic default to `High`.

---

## 5. The 28-Role Taxonomy

Roko defines 28 agent roles organized by responsibility. Each role carries defaults for backend, model tier, turn budget (in USD), and tool permissions. These defaults are starting points; any plan or policy can override them.

**Orchestration roles** manage the execution loop itself:
- **Conductor** — meta-orchestrator that watches all other agents and intervenes when stuck
- **PlanLifecycleManager** — manages plan state transitions
- **PrePlanner** — validates pre-plan artifacts before expensive enrichment

**Planning roles** design work:
- **Strategist** — writes the plan brief, decomposes PRDs into tasks
- **Architect** — reviews architecture before implementation

**Implementation roles** write code:
- **Implementer** — the main coding agent (full read/write/exec/git permissions)
- **AutoFixer** — lightweight patcher used after gate failure in express mode
- **Refactorer** — structural rewrite without behavior change
- **MergeResolver** — resolves merge conflicts across parallel workstreams

**Review roles** assess quality:
- **Auditor** — post-implementation review for correctness and safety
- **Critic** — devil's advocate / alternative-approach reviewer
- **QuickReviewer** — single-pass reviewer for Standard-complexity plans
- **Scribe** — drafts documentation

**Research roles** gather information:
- **Researcher** — broad research reader (docs, code, external; has network permissions)
- **PatternExtractor** — extracts reusable patterns from completed work
- **ErrorDiagnoser** — diagnoses errors into actionable root causes

**Validation roles** verify correctness:
- **IntegrationTester**, **TerminalValidator**, **GolemLifecycleTester**, **CrossSystemTester**, **FullLoopValidator** — various levels of end-to-end testing
- **SnapshotComparator** — compares snapshots across runs for drift
- **DocVerifier** — verifies docs still match code after edits
- **DependencyValidator** — validates dependency additions/upgrades
- **RegressionDetector** — watches for regression in test-pass rate and cost
- **SpecDriftDetector** — detects divergence between PRD and implementation

**Observability roles** track system health:
- **PerformanceSentinel** — tracks performance metrics across runs
- **CoverageTracker** — tracks coverage/rung satisfaction

### Tool Permissions

Each role operates within a permission matrix:

| Permission Level | Read | Write | Exec | Git | Network |
|:---|:---:|:---:|:---:|:---:|:---:|
| **Full** (Implementer, Refactorer) | Yes | Yes | Yes | Yes | No |
| **Read-only** (Auditor, Critic) | Yes | No | No | No | No |
| **Read+Exec** (validators, testers) | Yes | No | Yes | No | No |
| **Networked** (Researcher) | Yes | No | Yes | No | Yes |

### Turn Budgets

Per-turn spend ceilings range from $0.10 (Conductor, operating at Haiku-class) to $3.00 (Architect, operating at Opus-class). A `multiplier` field adjusts the budget for tier escalation: Opus dispatch multiplies by 2.0x, Haiku by 0.6x.

---

## 6. The Orchestration System

### Plan Structure

A plan is a `tasks.toml` file that defines a DAG of tasks. Each task specifies:

- An **id** and human-readable **title**
- **Dependencies** (which task IDs must complete before this one starts)
- A **role** (kebab-case label that resolves to an `AgentRole`)
- A **domain** (`Code`, `Chain`, `Config`, `Docs`, etc.)
- A **complexity band** (`Trivial`, `Simple`, `Standard`, `Complex`, `Heroic`)
- **Acceptance criteria** (free-text description of what "done" means)
- Optional **model override**, **tool allowlist/denylist**, and **gate configuration**

### The ParallelExecutor

The `ParallelExecutor` in `roko-orchestrator` is a pure state machine. It never performs I/O. It takes a `UnifiedTaskDag` (the parsed plan), tracks task states (pending, ready, running, passed, failed, skipped), and emits `ExecutorAction` values that describe what should happen next:

- `DispatchAgent { task_id, role, prompt, ... }` — run an agent on this task
- `RunGate { task_id, rung, ... }` — validate this task's output
- `PersistState { snapshot }` — save a checkpoint
- `NotifyCompletion { task_id, verdict }` — a task finished

The orchestration harness in `orchestrate.rs` (the CLI's runtime module) receives these actions and performs the actual I/O: spawning agent subprocesses, running gate commands, writing files, updating the TUI. Results flow back as `ExecutorEvent` values that advance the state machine.

This separation means the executor logic is fully testable without any I/O. The harness is the adapter layer between the pure state machine and the messy real world.

### Parallel Execution

Tasks whose dependencies are all satisfied run concurrently. The executor maintains a set of "ready" tasks and dispatches them in parallel via `JoinSet` (Tokio). A configurable concurrency limit caps the number of simultaneous agent subprocesses. When a task completes and its gate pipeline passes, the executor checks whether any blocked tasks are now unblocked and adds them to the ready set.

### Worktree Isolation

For code-domain tasks, each parallel task can run in its own git worktree. The `WorktreeManager` in `roko-orchestrator` creates branches, checks out worktrees, and tracks their health. After a task passes gates, its worktree is merged back to the plan branch. An idle TTL (default 30 minutes) reclaims stale worktrees.

### Resumption and Snapshots

The executor periodically serializes its state to `.roko/state/executor.json`. This snapshot includes the task DAG, per-task status, gate results, timestamps, circuit-breaker state, and the current event log. On resume, the executor reloads the snapshot and picks up where it left off, skipping already-completed tasks.

---

## 7. System Prompt Construction

### The 9-Layer SystemPromptBuilder

The `SystemPromptBuilder` in `roko-compose` constructs system prompts from nine composable layers, each targeting a different cache-stability tier:

| Layer | Content | Cache Tier |
|:---|:---|:---|
| **1. Role identity** | Who the agent is and what its job is | System (stable) |
| **2. Conventions** | Project coding standards, naming rules | System (semi-stable) |
| **3. Domain context** | Project-specific knowledge | Session (semi-stable) |
| **3c. Active signals** | Pheromone / stigmergic guidance from other agents | Session (semi-stable) |
| **4. Task context** | Current task details, acceptance criteria | Task (volatile) |
| **4b. Gate feedback** | Prior verification failure digest (for retries) | Dynamic |
| **5. Tool instructions** | Available tools and usage guidance | System (stable) |
| **6. Relevant techniques** | Learned playbooks and skills from prior tasks | Task (volatile) |
| **7. Anti-patterns** | What NOT to do (learned from past failures) | Task (volatile) |
| **8. Affect guidance** | Emotional tone and focus from the affect engine | Dynamic |

The builder uses a fluent API:

```rust
let prompt = SystemPromptBuilder::new("You are an implementer...")
    .with_conventions("Use snake_case, thiserror for errors")
    .with_domain("DeFi protocol context: ...")
    .with_task("Implement the rate limiter in crates/roko-core")
    .with_tools("MCP tools available: Read, Write, Bash")
    .with_anti_patterns(vec!["Never call unwrap in library crates"])
    .build();
```

### Cache Alignment

Layers 1 + 2 + 5 form the prefix-cacheable "system" tier and rarely change between tasks. Layers 3 and 3c form the "session" tier and change per-plan or per-agent-session. Layers 4 + 6 + 7 are per-task. Layers 4b and 8 are dynamic. The builder can insert cache alignment markers between tiers so that LLM providers that support prompt caching (Anthropic's cache_control) get maximum cache hit rates.

### Attention Bidding and Section Scoring

When the assembled prompt exceeds the token budget, the `PromptComposer` uses attention bidding to decide what stays and what gets cut. Each `PromptSection` carries:

- A **priority** (Critical, High, Normal, Low, Optional)
- A **placement** (Top, Middle, Bottom)
- A **hard cap** in tokens
- An **AttentionBidder** variant (Task, Research, Neuro, Oracles, Pheromone)

Bidders compete for token budget. The composer scores each section using a `SectionScorer` (which can incorporate learned section-effectiveness data from prior runs), then fills the prompt from highest-scoring sections down until the budget is exhausted.

### Section Effectiveness Learning

The `SectionEffectivenessRegistry` in `roko-learn` tracks the causal impact of each prompt section on gate outcomes. For each section, it records how many times the section was included vs. excluded, and the pass rate in each case. The "lift" (pass rate with section minus pass rate without) determines whether a section gets promoted, demoted, or dropped in future prompts. This is Roko's version of prompt A/B testing at the section level.

---

## 8. Safety Layer

### Tool Dispatch Pipeline

Every tool call passes through a six-stage safety funnel in the `ToolDispatcher`:

1. **Validate** — arguments are checked against the tool's JSON schema.
2. **Resolve** — the `ToolDef` for the canonical tool name is looked up in the registry.
3. **Authorize** — the tool's required permissions (read/write/exec/git/network) are compared against the role's granted permissions. A Reviewer role with `write=false` cannot call the Write tool.
4. **Safety checks** — if a `SafetyLayer` is attached, pre-execution policies run. These include secret scrubbing (detecting API keys, credentials, private keys in tool arguments), provenance logging (recording a custody chain for each tool invocation), and contract enforcement.
5. **Hook chain** — an optional sequence of `SafetyHook` implementations runs in order. Each hook can allow, modify (rewrite arguments), or reject the call. The first rejection short-circuits the chain. All decisions are emitted as audit Engrams.
6. **Execute with timeout and cancellation** — the handler runs with a configurable timeout and a cancellation token. Results exceeding `max_result_bytes` (default 16KB) are truncated at UTF-8 character boundaries.

### Agent Contracts

`AgentContract` defines per-role constraints in YAML. Contracts can restrict which tools a role may use, set cost ceilings, require specific output formats, or enforce domain-specific invariants. When a contract YAML is missing, the system falls back to a permissive default (this is a known gap tracked for tightening).

### Batch Dispatch

`ToolDispatcher::dispatch_batch` groups calls by concurrency mode. Tools marked `Parallel` (read-only operations like Read, Glob, Grep) run concurrently via `join_all`. Tools marked `Serial` (shell commands, file writes) run sequentially to avoid write-write races and preserve shell state ordering.

### Secret Scrubbing

The `ScrubPolicy` in `roko-agent/src/safety/scrub.rs` runs on both inputs and outputs, detecting and redacting patterns that look like API keys, bearer tokens, private keys, and other sensitive material. Scrubbing is applied before tool execution (to prevent agents from passing secrets to tools) and after (to prevent tool outputs from leaking secrets into prompts).

### Custody Logging

Every tool invocation is recorded in a custody log at `.roko/custody/audit.jsonl`. Each entry captures the tool name, arguments (post-scrubbing), result summary, timestamp, agent role, and trace ID. This provides a complete audit trail of what every agent did, when, and why.

---

## 9. Learning Subsystems

### Episode Logger

Every agent turn is recorded as an `Episode` in `.roko/episodes.jsonl`. An episode captures:

- Task ID, role, model used, backend
- Input token count, output token count, total cost in USD
- Duration (wall-clock time for the turn)
- Gate verdict (pass/fail/skip, with the rung that was applied)
- An HDC fingerprint — a high-dimensional binary vector encoding the episode's semantic content for fast similarity search

Episodes are the raw data that feeds every other learning subsystem.

### Cascade Router

The `CascadeRouter` persists to `.roko/learn/cascade-router.json`. It implements a LinUCB contextual bandit that selects models based on:

- Task category (code, research, review, validation)
- Task domain (DeFi, infrastructure, testing)
- Complexity band
- Historical pass rate per model for this category/domain
- Cost per successful turn

The router explores initially (trying different models) and exploits as it accumulates data (shifting traffic to the best-performing model). An exploration parameter controls the explore/exploit tradeoff.

### Prompt Experiments

The `ExperimentStore` at `.roko/learn/experiments.json` manages A/B experiments over prompt variants. An experiment defines a control prompt and a variant prompt, assigns incoming tasks to one of the two arms, and tracks gate outcomes per arm. When statistical significance is reached, the winning variant becomes the new default.

### Adaptive Gate Thresholds

Gate pass/fail thresholds are not static. The `AdaptiveThresholds` system maintains an exponential moving average (EMA) of pass rates per rung. If a rung consistently passes at 98%+, its threshold tightens (expecting higher quality). If it consistently fails at 30%, the threshold relaxes (avoiding blocking all work on a noisy gate). Thresholds persist to `.roko/learn/gate-thresholds.json`.

### Efficiency Events

Each agent turn emits an `AgentEfficiencyEvent` to `.roko/learn/efficiency.jsonl`, recording per-turn metrics: tokens used, cost, time, prompt section metadata (which sections were included and their token counts), and gate outcome. These events feed the fleet-wide C-Factor (collective intelligence factor), a composite metric that tracks gate pass rate, turn-taking equality, knowledge integration rate, information flow, and HDC diversity across all agents.

### Playbook Store

The `PlaybookStore` extracts reusable task sequences from completed work. When a Strategist encounters a task similar to one that previously succeeded, the playbook is injected into the system prompt's layer 6 (Relevant Techniques) so the agent can follow a proven approach.

### Skill Library

The `SkillLibrary` stores fine-grained skills extracted from successful agent turns. Skills are smaller than playbooks — a single technique or pattern rather than a full task sequence. They are matched to tasks by domain and category, and injected into layer 6.

### Error Pattern Store

The `ErrorPatternStore` at `.roko/learn/discovered-patterns.json` catalogs recurring gate failure patterns. When a gate failure matches a known pattern, the system can inject the pattern's known fix as an anti-pattern (layer 7) or skip straight to an AutoFixer with a targeted prompt.

---

## 10. Key Crates

| Crate | What It Does |
|:---|:---|
| **roko-core** | The kernel. Defines `Engram`, the 6 verb traits, `AgentRole` (28 variants), `ModelTier`, `TurnBudget`, `ToolPermissions`, configuration schema, tool registry, and error types. Everything depends on this crate; it depends on almost nothing. |
| **roko-agent** | Agent dispatch and tool execution. Houses the `ToolDispatcher` (6-stage safety pipeline), 7 LLM backend adapters, agent pool management, MCP config passthrough, invocation session tracking, warm-reuse policies, and the safety layer (scrubbing, contracts, provenance, hooks). |
| **roko-agent-server** | Per-agent HTTP sidecar. Exposes 13 routes including `/message` (real LLM dispatch), `/stream` (WebSocket), `/predictions`, `/research`, and `/tasks`. Allows external systems to interact with individual agents over HTTP. |
| **roko-serve** | HTTP control plane. Exposes ~85 REST routes plus SSE and WebSocket endpoints on port 6677. Covers plans, tasks, agents, PRDs, knowledge, learning state, configuration, and health monitoring. Powers external dashboards and API consumers. |
| **roko-orchestrator** | Plan DAG management. Contains the `ParallelExecutor` (pure state machine), `UnifiedTaskDag`, worktree management, merge queue, coordination primitives (pheromones), recovery engine, and snapshot serialization. No I/O — only state transitions and actions. |
| **roko-gate** | Verification pipeline. 11+ gate implementations (Compile, Test, Clippy, Diff, LLM Judge, Symbol, Generated Test, Integration, Property Test, Benchmark, Format Check, Security Scan, Fact Check, Verify Chain) organized into a 7-rung pipeline with adaptive thresholds, ratcheting, and failure classification. |
| **roko-compose** | Prompt assembly. The 9-layer `SystemPromptBuilder`, role templates, `PromptComposer` (attention-weighted section assembly with token budget management), enrichment pipeline, context bidders, and PAD (Pleasure-Arousal-Dominance) affect state rendering. |
| **roko-conductor** | Runtime supervision. 10 watchers (health monitor, stuck detector, circuit breaker, anomaly detector), diagnosis engine, and system snapshot facilities. The Conductor role uses this crate to watch over all other agents and intervene when things go wrong. |
| **roko-learn** | Learning infrastructure. Episode logger, cascade router (LinUCB bandit), prompt experiments (A/B), adaptive gate thresholds (EMA), efficiency events, playbook store, skill library, error pattern store, C-Factor computation, curriculum scheduler, routing log, latency registry, cost tracking, and section effectiveness learning. |
| **roko-cli** | The CLI binary. All subcommands (`plan`, `prd`, `research`, `agent`, `knowledge`, `learn`, `serve`, `dashboard`, `status`, `doctor`, etc.), the orchestration harness (`orchestrate.rs`), the ratatui TUI (F1-F7 tabs with file watcher), agent spawn configuration, and dispatch helpers. This is the main entry point for all of Roko. |
| **roko-fs** | File-backed storage. `FileSubstrate` (JSONL-based Engram storage with garbage collection), `RokoLayout` (standard directory structure for `.roko/`), and observability sinks. |
| **roko-std** | Standard library. 19 built-in tool handlers (Read, Write, Edit, Bash, Glob, Grep, Git operations, etc.), `StaticToolRegistry`, `SumScorer`, and a mock dispatcher for testing. |
| **roko-runtime** | Process lifecycle. `ProcessSupervisor` (tracks agent subprocesses, handles graceful shutdown with drain grace periods), runtime event bus (typed events for plan revision, gate verdicts, agent lifecycle), and cancellation tokens. |
| **roko-primitives** | Low-level primitives. Hyperdimensional computing (HDC) vectors for semantic fingerprinting, tier routing algorithms, and binary vector operations. HDC fingerprints are computed per-episode and used for fast similarity search in the knowledge store. |
| **roko-neuro** | Durable knowledge store. `NeuroStore` with tiered knowledge (Ephemeral, Working, Consolidated, Archival), admission gating, distillation (compressing many observations into durable insights), tier progression (promoting knowledge based on confirmation and utility), context assembly for prompt injection, and emotional provenance tracking. |
| **roko-dreams** | Offline consolidation. The dream engine runs during delta cycles (slow, reflective periods) to consolidate raw observations into durable knowledge. Three phases: hypnagogia (loose association), imagination (hypothesis generation), and full cycle (verification and commitment). Currently triggered from the orchestrator but lacks a standalone cron trigger. |
| **roko-daimon** | Affect engine. Models agent emotional state using the PAD (Pleasure-Arousal-Dominance) framework. Somatic markers (gut feelings about task difficulty, code quality, risk) modulate dispatch parameters: a high-arousal, low-pleasure state (frustration) might trigger model escalation or reduced turn budget. The `DaimonState` is loaded and applied per-task during orchestration. |
| **roko-chain** | Chain integration. Provides `ChainClient` and `ChainWallet` traits with an `AlloyChainClient` implementation for EVM interaction. Used for knowledge posting, task attestation, and witness anchoring. Currently Phase 2+ status — the traits are defined and the Alloy implementation exists, but full runtime integration awaits the Korai chain's testnet. |

---

## 11. Integration with the Korai Chain

Roko agents are the off-chain compute layer for the Korai decentralized context engineering platform. The integration operates across several surfaces:

### Knowledge Posting

When a Roko agent produces a durable insight (a knowledge entry that survives distillation in the neuro store and reaches the Consolidated tier), it can post that insight to the Korai chain's Knowledge layer (L2) as a blob. The blob carries:

- The insight's content (text payload)
- Schema type and metadata tags for retrieval
- A `CompletionProof` — cryptographic evidence that the insight was produced by a valid agent run (TEE attestation hash, system prompt hash for the Ventriloquist defense, reasoning trace commitment)
- Provenance chain back to the source episodes

Posted knowledge becomes available to all agents on the Korai network via the stigmergy retrieval layer (L2.5). Other agents query posted knowledge before assembling their LLM prompts, creating a collectively-curated context that improves every time any agent learns something.

### Knowledge Retrieval and Context Assembly

Before dispatching an agent, the orchestrator can query the Korai network's InsightStore for relevant prior knowledge. Retrieved entries are scored by relevance, filtered by trust tier and domain track, and injected into the system prompt's layer 3 (Domain Context) or layer 3c (Active Signals / Pheromones). This means an agent working on a DeFi rate monitoring task will see insights posted by other agents who previously worked on related problems — even agents operated by different teams.

The neuro store (`roko-neuro`) serves as the local cache and staging area for chain-sourced knowledge. Entries flow through a tiered progression: Ephemeral (just retrieved, unverified) to Working (confirmed useful in a local context) to Consolidated (durably valuable, candidate for on-chain posting) to Archival (cold storage for long-term retention).

### ISFR Consumption

Agents executing yield-related tasks can read the ISFR (Internet Secured Funding Rate) benchmark from on-chain verified state. ISFR is a confidence-weighted median interest rate computed across DeFi lending venues (Aave, Compound, Hyperliquid, dYdX). The agent's scaffolding layer queries the ISFR oracle, and the result is injected as a verified score into the prompt context. The agent can then reason about rate divergence, hedge recommendations, and clearing profile adjustments with a trusted, manipulation-resistant reference rate.

### Task Execution and Job Marketplace

The Korai chain hosts a job marketplace where agents bid on tasks (X402 agentic jobs). When a Roko agent accepts a job from the marketplace:

1. The job specification is parsed into a task definition compatible with Roko's `tasks.toml` format.
2. The orchestrator dispatches the task through the standard loop (compose prompt, dispatch agent, run gates, persist results).
3. On completion, the agent submits a sealed result with a `CompletionProof` to the chain.
4. A VRF-assigned worker panel (2-of-3 or 3-of-5) verifies the submission's schema conformance, proof integrity, and provenance.
5. Upon verification, the agent earns the job fee and its domain reputation score is updated.

### Agent Identity and Reputation

Each Roko agent registers on-chain via the GolemRegistry (an extension of ERC-8004 Agent Passports). The registration includes:

- Capability lists (which domains and job types the agent can handle)
- System prompt hash (Ventriloquist defense — proves the agent is running the code it claims)
- Staking tier (bonded GNOS tokens, slashable for misbehavior)
- Liveness heartbeat transactions

The agent's on-chain reputation is tracked independently across seven domain tracks (Oracle Resolution, Risk Detection, Anomaly Flagging, Data Integrity, Cross-App Validation, Sealed Execution, Knowledge Verification). Reputation follows an EMA with 30-day half-life decay, converging toward a neutral midpoint when idle. The reputation multiplier ($0.1\times$ at probation to $3.0\times$ at elite) directly affects job priority, fee rates, and knowledge entry weighting in the stigmergy layer.

### Chain Client Integration

The `roko-chain` crate provides the Rust-side integration. `ChainClient` is an async trait with methods for reading state, submitting transactions, and querying knowledge. `ChainWallet` handles key management and transaction signing. The `AlloyChainClient` implements these traits using the Alloy Ethereum SDK, targeting the Korai EVM. The orchestration harness in `orchestrate.rs` imports and wires the chain client, making chain operations available as part of the standard dispatch loop. A chain-aware handler resolver (`chain_aware_resolver`) adds chain-specific tool handlers (e.g., `post_knowledge`, `query_insight_store`, `submit_completion_proof`) alongside the standard 19 built-in tools.

---

## 12. Data Flow Summary

The following shows the complete data flow for a single task execution:

```
roko.toml (config)
  |
  v
tasks.toml (plan DAG)
  |
  v
ParallelExecutor (pure state machine)
  |
  | emits ExecutorAction::DispatchAgent
  v
orchestrate.rs (I/O harness)
  |
  |-- resolve_model() --> ModelSpec (slug + backend + effort)
  |-- score_model_for_task() --> best model for TaskRequirements
  |-- CascadeRouter.select() --> learned bandit adjustment
  |-- SystemPromptBuilder
  |     |-- Layer 1: Role identity (from AgentRole)
  |     |-- Layer 2: Conventions (from roko.toml)
  |     |-- Layer 3: Domain context (from neuro store + chain knowledge)
  |     |-- Layer 3c: Pheromones (from other agents' signals)
  |     |-- Layer 4: Task context (from tasks.toml entry)
  |     |-- Layer 4b: Gate feedback (from prior failures)
  |     |-- Layer 5: Tool instructions (from ToolRegistry)
  |     |-- Layer 6: Playbooks + Skills (from roko-learn)
  |     |-- Layer 7: Anti-patterns (from error pattern store)
  |     |-- Layer 8: Affect guidance (from daimon engine)
  |     v
  |   Assembled system prompt (token-budgeted, cache-aligned)
  |
  v
Agent subprocess (Claude CLI / Codex / Cursor / HTTP API)
  |
  |-- Tool calls --> ToolDispatcher
  |     |-- Validate args
  |     |-- Resolve ToolDef
  |     |-- Authorize (role permissions)
  |     |-- SafetyLayer (pre-checks, contract, scrubbing)
  |     |-- Hook chain (sequential safety hooks)
  |     |-- Execute handler (with timeout + cancellation)
  |     |-- Truncate result
  |     |-- Post-execution scrubbing
  |     v
  |   ToolResult --> back to agent
  |
  v
Agent output
  |
  v
Gate Pipeline (rung selection by complexity band)
  |-- Rung 1: CompileGate
  |-- Rung 2: ClippyGate
  |-- Rung 3: TestGate
  |-- Rung 4: DiffGate + LlmJudgeGate
  |-- Rung 5: SymbolGate + IntegrationGate
  |-- Rung 6: PropertyTestGate + SecurityScanGate
  |-- Rung 7: FullLoopValidator
  v
Gate Verdict (pass / fail / skip)
  |
  |-- If pass: persist output, advance DAG, merge worktree
  |-- If fail: classify failure, emit feedback, optionally replan
  |
  v
Learning updates (all in parallel):
  |-- EpisodeLogger --> .roko/episodes.jsonl
  |-- CascadeRouter --> .roko/learn/cascade-router.json
  |-- AdaptiveThresholds --> .roko/learn/gate-thresholds.json
  |-- EfficiencyEvent --> .roko/learn/efficiency.jsonl
  |-- SectionEffectiveness --> .roko/learn/section-effects.json
  |-- PlaybookStore --> .roko/learn/playbooks.json
  |-- SkillLibrary --> .roko/learn/skills.json
  |-- ErrorPatternStore --> .roko/learn/discovered-patterns.json
  |-- CustodyLog --> .roko/custody/audit.jsonl
  |-- (optional) ChainClient.post_knowledge() --> Korai L2
  |
  v
ExecutorEvent --> ParallelExecutor (advance state, unblock next tasks)
```

This cycle repeats for every task in the DAG until all tasks are complete, all tasks have failed, or the circuit breaker trips (too many consecutive failures). The system learns from every iteration, making each subsequent run measurably better than the last.
