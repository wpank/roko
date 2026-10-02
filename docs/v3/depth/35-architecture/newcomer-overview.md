# Newcomer Overview

> **Verified against codebase: 2026-09-15.**
> This document introduces Roko to someone with zero prior context.
> It covers what Roko is, why it exists, how it works at every level,
> and how to get productive in your first 10 minutes.
> For the condensed architecture overview, see `docs/v3/35-ARCHITECTURE.md`.

---

## Table of Contents

1. [What is Roko?](#1-what-is-roko)
2. [Why does Roko exist?](#2-why-does-roko-exist)
3. [The self-hosting idea](#3-the-self-hosting-idea)
4. [How the core loop works](#4-how-the-core-loop-works)
5. [The three pillars: Signal, Cell, Graph](#5-the-three-pillars-signal-cell-graph)
6. [Crate map for the impatient](#6-crate-map-for-the-impatient)
7. [Your first 10 minutes](#7-your-first-10-minutes)
8. [Project structure walkthrough](#8-project-structure-walkthrough)
9. [Reading the source code](#9-reading-the-source-code)
10. [Common workflows](#10-common-workflows)
11. [The learning loop explained](#11-the-learning-loop-explained)
12. [Configuration in depth](#12-configuration-in-depth)
13. [Troubleshooting your first run](#13-troubleshooting-your-first-run)
14. [Common commands reference](#14-common-commands-reference)
15. [Frequently asked questions](#15-frequently-asked-questions)
16. [Where to go next](#16-where-to-go-next)

---

## 1. What is Roko?

Roko is a command-line toolkit, written in Rust, that turns LLM-powered AI agents
into a repeatable software engineering pipeline. You describe what you want in plain
English. Roko breaks it into tasks, dispatches an LLM agent for each task, verifies
the output (does it compile? do tests pass? does the linter agree?), records what
happened, and uses that record to do better next time.

The unusual part: Roko uses this same pipeline to develop *itself*. The codebase you
are looking at was largely built by Roko's own agents, verified by Roko's own gates,
and improved by Roko's own learning loops. This property -- the system being its own
primary user -- is called **self-hosting**, and it is the reason every component
exists.

### The elevator pitch

```
You: "Add rate limiting to the API endpoints"
     |
Roko: breaks it into tasks -> dispatches Claude/GPT/Gemini
     -> verifies with cargo check/test/clippy
     -> records the outcome
     -> learns which model works best for which kind of task
     -> retries failures with adjusted parameters
     |
You: reviewed, tested code in a branch, ready for PR
```

### What Roko is NOT

- **Not an IDE plugin.** Roko is a CLI tool and HTTP server. It integrates with
  editors via ACP (Agent Client Protocol), but the primary interface is `roko <command>`.
- **Not a single-model wrapper.** Roko supports 12 LLM provider backends and learns
  which model to use for which task.
- **Not a chatbot.** While `roko chat` exists for interactive use, the core value is
  the automated plan-execute-verify loop, not conversation.
- **Not a cloud service.** Everything runs on your machine. State lives in the project
  directory. No account required.

---

## 2. Why does Roko exist?

The problem: AI coding assistants produce unreliable output. Sometimes the code
compiles, sometimes it does not. Sometimes it passes tests, sometimes it breaks them.
There is no feedback loop -- each interaction starts from scratch. Every prompt is a
roll of the dice.

Roko solves this with three ideas:

**1. Verification gates.** Every agent output passes through a pipeline of up to 19
checks -- compile, lint, test, diff analysis, LLM judge, and more -- organized into 7
rungs of increasing cost. Bad code is rejected and the agent retries with the error
message included in the next attempt. The cheapest gates (does it compile?) run first
so you waste no money on expensive checks when the code is obviously broken.

**2. Persistent learning.** Every outcome (success, failure, cost, latency, model
used, tokens consumed) is recorded to JSONL files. Over time, Roko learns:
- Which models work best for which tasks (cascade model routing).
- Which prompting patterns produce better results (playbook extraction).
- Which mistakes to warn about in future prompts (anti-pattern injection).
- How much to explore vs. exploit (contextual bandit with UCB).

**3. Plan-based execution.** Instead of one-shot prompts, Roko breaks work into a DAG
of tasks with dependencies. Tasks execute in topological order, with parallel waves
for independent work. State checkpoints after each task so you can resume after
interruption. If a task fails all retries, the system can generate a revised plan and
try a different approach.

---

## 3. The self-hosting idea

Every architectural decision in Roko exists because the self-hosting loop needed it.
A plan is the unit of work. Here is the complete self-hosting workflow as CLI commands:

```
    roko plan generate "..."     Write a plan from what you want to build:
         |                        plans/<slug>/ (tasks.toml + plan.md)
         v
    roko research enhance-plan   Research-backed improvements to the plan (optional)
         |
         v
    review / edit tasks.toml     Read and adjust the plan; roko plan validate lints it
         |
         v
    roko run plans/<slug>        Execute the plan through the Graph engine
         |                        - each task dispatches an LLM agent
         |                        - each agent output runs through gates
         |                        - state checkpoints after each task
         |
         +---> gate fails?        Retry the task with the gate's feedback
         |                        (up to max_retries)
         |
         +---> interrupted?       roko plan run plans/<slug> --resume-plan
         |
         v
    roko dashboard               Watch progress in real time (TUI)
         |
         v
    roko status                  Inspect the final state
```

`roko run --plan "..."` writes the plan, shows it, and asks before running it, all in
one command.

When you run a plan (`roko run plans/<slug>` or `roko plan run`), the system:
1. Parses your plan (a `tasks.toml` file with task descriptions and dependencies).
2. Converts the task DAG into a Graph of Cells.
3. Topologically sorts the graph and identifies parallel waves.
4. For each task: selects a model, builds a prompt, dispatches an agent, runs gates,
   records the outcome.
5. Checkpoints after each task so a crash is never more than one task's worth of work.

For a single quick task that does not need the full planning pipeline:

```bash
roko run "add a health check endpoint to the API"
```

This is the same pipeline compressed into a single step. Internally it still composes a
prompt, dispatches an agent, runs gates, and persists the result.

---

## 4. How the core loop works

Everything in Roko follows one loop. Here it is, annotated with the responsible crate
at each step:

```
Step  Name       Crate             What happens
----  ---------  ----------------  -----------------------------------------------
  1   ROUTE      roko-learn        CascadeRouter selects a model. Three stages:
                                     Static (< 50 obs) -> Confidence (50-200) -> UCB (200+)
                                   Unhealthy providers filtered out.

  2   COMPOSE    roko-compose      SystemPromptBuilder assembles 9 layers:
                                     L1 Role ("You are a senior Rust engineer...")
                                     L2 Conventions (style, imports, naming rules)
                                     L3 Domain (crate map, project context)
                                     L3c Active signals (pheromone/stigmergic guidance)
                                     L4 Task (the actual work to do)
                                     L4b Feedback (errors from prior attempts)
                                     L5 Tools (available: Read, Write, Bash...)
                                     L6 Playbooks (successful patterns from history)
                                     L7 Anti-patterns (known mistakes to avoid)
                                     L8 Affect (adjust tone based on recent outcomes)

  3   MODULATE   roko-daimon       DaimonState reads PAD affect state (pleasure/
                                   arousal/dominance). Computes DispatchModulation:
                                   temperature delta, turn budget, exploration rate.
                                   After failures: lower temp, less exploration.

  4   ACT        roko-agent        AgentDispatcher sends the prompt to one of 12
                                   LLM provider backends. Agent runs in a tool loop:
                                   read files, write code, run commands. Safety layer
                                   enforces tool policies and role-based access.

  5   VERIFY     roko-gate         GatePipeline runs 7 rungs of verification:
                                     R0 Compile (cargo check / tsc / go build)
                                     R1 Lint (cargo clippy / eslint)
                                     R2 Test (cargo test)
                                     R3 Symbol (public API breakage)
                                     R4 GeneratedTest (agent-written tests)
                                     R5 PropertyTest (proptest + fact check)
                                     R6 Integration (full scenarios + LLM judge)
                                   Thresholds adapt via EMA -- auto-tighten/relax.

  6   PERSIST    roko-learn        EpisodeLogger writes to .roko/episodes.jsonl.
                 roko-fs           Agent output Signal stored in .roko/engrams.jsonl.
                 roko-graph        Graph engine checkpoints to .roko/state/graph/.

  7   LEARN      roko-learn        CascadeRouter.feedback() updates model arm.
                 roko-daimon       DaimonState.on_outcome() updates affect.
                 roko-neuro        KnowledgeStore considers creating entry.
                                   Gate thresholds updated. Efficiency event written.

  8   REACT      roko-cli/runner   If gates failed: GateFailureReplan builds revision.
                 roko-conductor    12 watchers monitor for stuck agents, budget
                                   exhaustion, quality degradation, latency spikes.
```

This loop runs for every task in a plan. When all tasks complete, the plan is done.

---

## 5. The three pillars: Signal, Cell, Graph

Three concepts form the backbone of Roko's design. Understanding these three gives
you a mental model for everything else.

### Signal: the universal data type

Everything in Roko is a **Signal**. An agent's output is a Signal. A gate verdict is a
Signal. A knowledge entry is a Signal. A task definition is a Signal.

A Signal is like a Git commit for a piece of information:

- **Content-addressed**: its identity is a BLAKE3 hash of its kind, body, author,
  taint status, lineage, and tags. Two Signals with identical content always have the
  same ID, regardless of when or where they were created.
- **Immutable**: once created, a Signal does not change. You create new Signals that
  reference old ones.
- **Traceable**: it carries parent pointers (`lineage` field) that form a DAG. You can
  always walk backward to see why an agent made a decision.
- **Decaying**: knowledge has a half-life. A `Decay::HalfLife` Signal fades over time.
  Facts persist for months; transient observations decay in minutes.
- **Scored**: multi-dimensional quality assessment (confidence, novelty, utility,
  reputation).
- **Tiered**: lifecycle goes Transient -> Working -> Consolidated -> Persistent.
  Only durable tiers survive across sessions.

The Rust struct is `Signal` in `roko-core/src/signal.rs`, with a backward-compatible
alias `pub type Engram = Signal` in `roko-core/src/engram.rs`. You will see both names
in the codebase; they refer to the same thing. `Signal` is the canonical name.

```rust
// Creating a Signal
let signal = Signal::builder(Kind::Task)
    .body(Body::text("implement login"))
    .tag("priority", "high")
    .decay(Decay::HalfLife { half_life_ms: 86_400_000 })
    .build();

// Identity is content-based
assert_eq!(signal.id, signal.content_hash());
```

Signal kinds include: `AgentOutput`, `GateVerdict`, `Task`, `Plan`, `Episode`,
`PlaybookRule`, `Pheromone`, `Insight`, `RouterChoice`, `Metric`, and more. The `Kind`
enum is `#[non_exhaustive]` with a `Custom(String)` escape hatch for extensions.

### Cell: a unit of computation

A **Cell** is any component that can participate in execution. Cells implement one or
more of 12 protocol traits. The `Cell` trait provides identity and metadata:

```rust
pub trait Cell: Send + Sync + 'static {
    fn cell_id(&self) -> &str;
    fn cell_name(&self) -> &str;
    fn protocols(&self) -> &[ProtocolId];   // Which of the 12 protocols this implements
    fn capabilities(&self) -> Capabilities; // What runtime permissions it needs
    fn estimated_cost(&self) -> Option<f64>;
}
```

The 12 protocol traits are:

| # | Trait | What it does | Example implementations |
|---|-------|-------------|------------------------|
| 1 | **Store** | Persist/retrieve Signals | FileSubstrate, MemorySubstrate |
| 2 | **ColdStore** | Archive aged-out Signals | ArchiveColdSubstrate |
| 3 | **Score** | Rate a Signal on dimensions | RelevanceScorer, RecencyScorer |
| 4 | **Verify** | Check against ground truth | CompileGate, TestGate, ClippyGate |
| 5 | **Route** | Pick one from many candidates | CascadeRouter, LinUCBRouter |
| 6 | **Compose** | Combine Signals into a prompt | SystemPromptBuilder, PromptComposer |
| 7 | **React** | Watch streams, emit interventions | Conductor watchers, circuit breaker |
| 8 | **Bus** | Publish/subscribe ephemeral Pulses | PulseBus |
| 9 | **Observe** | Import data from external sources | Chain observers, file watchers |
| 10 | **Connect** | Manage network connections | Relay connections |
| 11 | **Trigger** | Arm/disarm scheduled actions | Cron triggers, event triggers |
| 12 | **Substrate** | Low-level storage backend | Alias for Store |

Every capability in the system -- agent dispatch, gate verification, prompt assembly,
model routing, knowledge retrieval, affect modulation -- is an implementation of one
of these protocols operating on Signals.

### Graph: the execution engine

A **Graph** is a directed acyclic graph (DAG) of Cells. When you run a plan, each task
becomes a subgraph. The Graph engine executes these in topological order, running
independent branches in parallel.

Key properties:
- **Parallel waves**: independent cells execute concurrently within a wave.
- **Conditional routing**: cells can route to different downstream paths based on output.
- **Cost enforcement**: per-plan and per-task USD budgets are enforced atomically.
- **Checkpoint/resume**: state is durably checkpointed after each task completion.
- **Immune Graph**: a five-stage verification pipeline screens all outputs before they
  propagate downstream.

The Graph engine is the **sole executor** since PR #260/#276. The older WorkflowEngine
was retired. Runner-v2 is retained as `--engine legacy` for one deprecation cycle.

---

## 6. Crate map for the impatient

The 39 workspace crates organized by what you need to know first. Start with Tier 1
and work down only when you need to.

### Must know (used in every execution)

| Crate | One-liner |
|-------|-----------|
| **roko-core** | The kernel: Signal type, 12 traits, config schema, errors |
| **roko-graph** | The sole execution engine: DAG cells, parallel waves, checkpoints |
| **roko-agent** | 12 LLM backends, tool loop, safety layer |
| **roko-gate** | 19 gates, 7-rung pipeline, adaptive thresholds |
| **roko-compose** | 9-layer prompt builder, 11 role templates |
| **roko-cli** | The `roko` binary: all commands, plan runner, TUI dashboard |

### Important for understanding behavior

| Crate | One-liner |
|-------|-----------|
| **roko-learn** | Episodes, cascade routing, playbooks, experiments, efficiency |
| **roko-neuro** | Durable knowledge store with tier progression |
| **roko-daimon** | Affect engine: PAD state adjusts dispatch parameters |
| **roko-dreams** | Offline consolidation: episodes -> knowledge -> playbooks |
| **roko-conductor** | 12 reactive watchers, circuit breaker, diagnosis |
| **roko-fs** | Append-only JSONL substrate on disk, GC, workspace layout |

### Important for external integration

| Crate | One-liner |
|-------|-----------|
| **roko-serve** | HTTP control plane: REST routes (counts in `tools/http_route_inventory.snapshot.json`) + SSE + WebSocket on :6677 |
| **roko-acp** | Editor integration protocol (Cursor, etc.) |
| **roko-agent-server** | Per-agent HTTP sidecar: 14 routes |
| **roko-execution** | Shared RuntimeServices builder for CLI/serve/ACP |

### Can learn later

Everything else: language providers (roko-lang-*), MCP servers (roko-mcp-*),
plugin SDK (roko-plugin), inference gateway (roko-gateway), chain primitives
(roko-chain), evaluation framework (roko-eval), and standalone apps.

---

## 7. Your first 10 minutes

### Prerequisites

- Rust 1.91 or newer (`rustup update stable`)
- An LLM API key (Anthropic, OpenAI, Google, or Cerebras)
- A project you want to work on (or use Roko's own codebase)

### Step 1: Build Roko (2 minutes)

```bash
git clone https://github.com/nunchi/roko
cd roko

# Build the CLI binary (release mode for speed)
cargo build -p roko-cli --release

# Verify it works
./target/release/roko --version
./target/release/roko doctor
```

Alternatively, install to your PATH:

```bash
cargo install --path crates/roko-cli
roko --version
```

The full workspace build (`cargo build --workspace`) compiles all 38 crates. For your
first run, just the CLI is enough.

### Step 2: Initialize a workspace (30 seconds)

Navigate to any project directory and run:

```bash
cd /path/to/your/project
roko init
```

This creates:
- `.roko/` directory -- all runtime state goes here
- `roko.toml` -- per-project configuration file

The `.roko/` directory stores everything: episodes, signals, knowledge, learning data,
checkpoints, research artifacts. Plans live in `plans/` at the workspace root. `.roko/`
is gitignored by default.

### Step 3: Configure a provider (1 minute)

The simplest way -- interactive setup:

```bash
roko setup
```

Or manually edit `roko.toml`:

```toml
[providers.anthropic]
kind = "anthropic_api"
api_key_env = "ANTHROPIC_API_KEY"
default_model = "claude-sonnet-4-20250514"
```

Make sure the environment variable is set:

```bash
export ANTHROPIC_API_KEY="sk-ant-..."
```

Verify your provider works:

```bash
roko config providers health
```

You should see output indicating the provider is reachable. If not, check your API key
and network connectivity.

### Step 4: Run your first task (2 minutes)

The simplest way to use Roko is a one-shot command:

```bash
roko run "add a health check endpoint that returns JSON with status and uptime"
```

This triggers the full loop: compose a prompt from your project context, dispatch an
agent, verify the output with gates, and persist the result. Watch the output to see
each step happening.

What you will see:
1. Model selection (which LLM was chosen and why)
2. Prompt assembly (the 9 layers being composed)
3. Agent execution (the LLM writing code, using tools)
4. Gate results (compile, lint, test -- each with pass/fail)
5. Episode recording (the outcome persisted for learning)

### Step 5: Try the planning pipeline (3 minutes)

For larger work, use the plan-based flow:

```bash
# Write a plan, show it, and run it after you approve
roko run --plan "Add rate limiting to the API endpoints"

# Or in steps: write the plan (plans/<slug>/), review or edit it, then run it
roko plan generate "Add rate limiting to the API endpoints"
roko run plans/<slug>

# Watch progress in the dashboard
roko dashboard
```

The dashboard is a full ratatui TUI with 10 tabs (F1-F10): dashboard, plans, agents,
git, logs, config, inspect, marketplace, learning, and providers.

### Step 6: Explore the results (1 minute)

```bash
# What happened?
roko status

# What did it learn?
roko learn all

# What does it know?
roko knowledge stats

# Inspect costs
roko show costs

# View episode history
roko show history
```

---

## 8. Project structure walkthrough

```
roko/
  Cargo.toml           Workspace root (38 members)
  roko.toml            Per-project configuration
  CLAUDE.md            Project instructions for AI agents (the canonical context file)
  README.md            Quick start and overview

  .roko/               All runtime state (gitignored except config)
  |  engrams.jsonl       Signal log (every Signal ever created, append-only JSONL)
  |  episodes.jsonl      Episode log (one record per agent task execution)
  |  GAPS.md             Canonical gap tracker -- check before starting any work
  |
  |  state/
  |  |  graph/           Graph engine checkpoints (per-plan, per-run)
  |  |  state-snapshot.json  Legacy Runner-v2 snapshot (deprecated)
  |
  |  learn/
  |  |  cascade-router.json  Model routing state (arm statistics, pass rates)
  |  |  gate-thresholds.json Adaptive gate thresholds (EMA per rung)
  |  |  efficiency.jsonl     Per-turn cost/latency/token events
  |  |  experiments/         A/B experiment assignments and results
  |  |  dream-routing.json   Dream-generated model routing advice
  |
  |  knowledge/
  |  |  entries/            Durable knowledge entries by type
  |  |  hdc/                HDC fingerprint index for semantic similarity
  |
  |  research/              Research artifacts and citations
  |  archive/               Cold storage for aged-out signals

  crates/               All 38 Rust crates (see crate map above)
  apps/                 Standalone applications
  |  agent-relay/         Bounded canonical-envelope relay server
  |  mirage-rs/           In-process EVM fork simulator
  |  roko-chain-watcher/  Chain observation agent
  docs/                 Documentation
  |  v3/                  Current documentation version
  |  |  depth/            Detailed depth files per chapter (you are here)
  plans/                Implementation plans (tasks.toml files)
  tests/                End-to-end integration test suite
  tools/                Developer utilities (route inventory, etc.)
```

---

## 9. Reading the source code

If you want to understand how the system works by reading code, here is the recommended
order. Each file is self-contained enough to understand without reading everything else.

### Start here (the data model)

1. **`crates/roko-core/src/signal.rs`** -- The Signal struct. This is the universal
   data type. Read the struct definition and the `content_hash()` method.

2. **`crates/roko-core/src/kind.rs`** -- Signal kinds. Scan the enum variants to see
   what kinds of Signals exist (AgentOutput, GateVerdict, Task, Episode, etc.).

3. **`crates/roko-core/src/traits.rs`** -- The 12 kernel traits. Read the doc comments
   on Store, Score, Verify, Route, Compose, React. These define the entire operational
   surface.

4. **`crates/roko-core/src/cell.rs`** -- The Cell trait. See how Capabilities, CostEstimate,
   and PredictionRecord work. Understand that every computation unit is identifiable
   and composable.

### Then trace an execution path

5. **`crates/roko-cli/src/runner/event_loop.rs`** -- The plan runner's event loop.
   This is where tasks are dispatched and results are processed. Follow how a single
   task moves through the system.

6. **`crates/roko-compose/src/system_prompt_builder.rs`** -- The 9-layer prompt builder.
   See how context, task, feedback, playbooks, and affect guidance are assembled into
   a single prompt.

7. **`crates/roko-agent/src/dispatcher/mod.rs`** -- The agent dispatcher. See how the
   system selects a provider, sends the prompt, and runs the tool loop.

8. **`crates/roko-gate/src/gate_pipeline.rs`** -- The gate pipeline. See how the 7 rungs
   are executed in sequence and how adaptive thresholds work.

### Then understand the feedback loop

9. **`crates/roko-learn/src/cascade_router.rs`** -- The cascade model router. See the
   three stages (Static -> Confidence -> UCB) and how feedback updates arm statistics.

10. **`crates/roko-learn/src/episode_logger.rs`** -- Episode recording. See what data is
    captured per agent execution.

11. **`crates/roko-daimon/src/lib.rs`** -- The affect engine. See how PAD state
    (pleasure/arousal/dominance) is tracked across three timescales and translated into
    dispatch modulation.

---

## 10. Common workflows

### "I want to fix a specific bug"

```bash
roko run "Fix the off-by-one error in the pagination logic in src/api/list.rs"
```

One-shot command. The agent reads the file, identifies the bug, fixes it, and gates
verify the fix compiles and passes tests.

### "I want to build a feature with multiple parts"

```bash
roko run --plan "Add WebSocket support for real-time updates"
```

The plan-based flow breaks the feature into tasks (add dependencies, create handler,
wire routes, write tests) and executes them in dependency order.

### "I want to ask a question without changing code"

```bash
roko think "How does the authentication middleware work in this codebase?"
```

The `think` command researches without executing agents or modifying files.

### "I want to research a topic for context"

```bash
roko research topic "rate limiting best practices in Rust"
roko research search "tokio rate limit middleware"
```

Research produces grounded, cited artifacts that can feed into plans (pass them to
`roko plan generate --context <path>`).

### "A plan failed and I want to see why"

```bash
roko plan status plans/
roko diagnose <plan-id>
```

The `diagnose` command prints a report explaining what failed, at which verify step, with
what error message, and how to resume. Add `--json` for the structured JSON report.

### "I want to resume after a crash"

```bash
roko plan run plans/ --resume-plan
```

The Graph engine checkpoints after every task. Resume picks up exactly where you left
off, skipping completed tasks.

---

## 11. The learning loop explained

Roko gets better over time through several interconnected feedback mechanisms:

### Online learning (happens during execution)

- **CascadeRouter feedback**: after each task, the router updates its statistics for
  the selected model. Over time, it learns which models work best for which task types.
  State persists to `.roko/learn/cascade-router.json`.

- **Adaptive gate thresholds**: each gate rung maintains an EMA (exponential moving
  average) of its pass rate. When a gate consistently passes, its threshold tightens
  (higher standards). When it consistently fails, it relaxes (avoid infinite retries).
  State persists to `.roko/learn/gate-thresholds.json`.

- **Affect state updates**: `DaimonState.on_outcome()` updates the PAD vector after
  each task. Consecutive failures produce "frustration" (lower pleasure, higher
  arousal), which causes the system to lower temperature and escalate to more capable
  models.

- **Efficiency tracking**: every turn records cost, latency, tokens, and model used.
  Written to `.roko/learn/efficiency.jsonl`.

### Offline learning (happens between sessions)

- **Dream consolidation** (`roko knowledge dream run`): batches completed episodes,
  clusters them by task shape using HDC fingerprints, and extracts durable knowledge:
  facts, insights, heuristics, procedures, constraints, and anti-knowledge.

- **Playbook promotion**: when the same approach succeeds repeatedly for a task shape,
  it is promoted to a "when/then" playbook. Playbooks are injected into future prompts
  (Layer 6 of the system prompt).

- **Knowledge tier progression**: Transient knowledge that gets confirmed by gate
  results and accumulates enough evidence is promoted to Working, then Reference.
  Reference-tier knowledge persists permanently and feeds future prompts.

---

## 12. Configuration in depth

The `roko.toml` file controls all behavior. Here is a minimal working configuration:

```toml
# Provider configuration -- at least one required
[providers.anthropic]
kind = "anthropic_api"
api_key_env = "ANTHROPIC_API_KEY"
default_model = "claude-sonnet-4-6"

# Optional: model routing tiers
[models.routing]
tier0 = "claude-haiku-4-5"     # Fast, cheap tasks
tier1 = "claude-sonnet-4-6"    # Default complexity
tier2 = "claude-opus-4-6"      # Hard tasks

# Optional: gate configuration
[gates]
max_rung = 2     # Only run compile + lint + test (skip expensive gates)
adaptive = true  # Enable adaptive threshold learning

# Optional: learning configuration
[learning]
replan_on_gate_failure = true  # Auto-generate revised plans on failure
```

Key configuration sections:
- `[providers.*]` -- LLM provider backends (API keys, models, endpoints)
- `[models.*]` -- Model routing and tier configuration
- `[gates]` -- Gate pipeline settings (which rungs, thresholds)
- `[learning]` -- Learning loop settings (replan, efficiency, experiments)
- `[knowledge]` -- Knowledge store settings (admission threshold, decay)
- `[budget]` -- Cost limits (per-plan, per-task USD caps)
- `[workspace]` -- Workspace settings (worktree management, disk limits)

Use `roko config show` to inspect the current configuration with all defaults applied.
Use `roko config validate` to check for schema errors.

---

## 13. Troubleshooting your first run

### "roko doctor says no providers found"

You need at least one LLM provider configured. Run `roko setup` for interactive
configuration, or add a `[providers.*]` section to `roko.toml`.

### "cargo build fails with version errors"

Roko requires Rust 1.91 or newer (the alloy EVM dependencies set this floor):
```bash
rustup update stable
rustc --version  # should show 1.91+
```

### "the agent runs but gates always fail"

Check which gates are running with `roko learn gates`. For a new project, start with
a low `max_rung` (0 or 1) in the `[gates]` config to only run compile and lint checks.
Higher rungs (tests, integration) require a project with an existing test suite.

### "roko plan run says 'no tasks found'"

Make sure your plan directory contains a `tasks.toml` file. Check with
`roko plan validate plans/` to lint the task definitions.

### "costs seem high"

Check `roko show costs` for a breakdown. The cascade router starts in Static mode
(using config defaults) and needs ~50 tasks before it begins optimizing. For cost
control during early runs, configure `[budget]` limits in `roko.toml`.

### "I see 'Engram' and 'Signal' in the code -- what is the difference?"

They are the same thing. `Signal` is the canonical name. `Engram` is a backward-
compatible type alias (`pub type Engram = Signal`). You may see either in logs, code,
and file names (e.g., `engrams.jsonl` is the Signal log).

---

## 14. Common commands reference

| Task | Command |
|------|---------|
| Initialize workspace | `roko init` |
| Interactive setup | `roko setup` |
| System health check | `roko doctor` |
| Disk health | `roko doctor disk` |
| Provider health | `roko config providers health` |
| One-shot task | `roko run "do something"` |
| Plan, review, run | `roko run --plan "feature description"` |
| Write a plan only | `roko plan generate "feature description"` |
| Execute plan | `roko run plans/<slug>` or `roko plan run plans/` |
| Resume interrupted plan | `roko plan run plans/ --resume-plan` |
| Validate plan | `roko plan validate plans/` |
| Plan status | `roko plan status plans/` |
| Diagnose failure | `roko diagnose <plan-id>` |
| Interactive dashboard | `roko dashboard` |
| View costs | `roko show costs` |
| View learning state | `roko learn all` |
| View gate thresholds | `roko learn gates` |
| Search knowledge | `roko knowledge query "topic"` |
| Knowledge stats | `roko knowledge stats` |
| Dream consolidation | `roko knowledge dream run` |
| Research a topic | `roko research topic "question"` |
| Think without acting | `roko think "question"` |
| Quick note | `roko note "observation"` |
| Chat with agent | `roko chat` |
| Start HTTP server | `roko serve` |
| Config inspection | `roko config show` |
| View all commands | `roko --help` |

---

## 15. Frequently asked questions

### How big is the codebase?

Approximately 1 million lines of Rust across 39 workspace crates, with 10,300+ tests.
48 out of 48 planned epics are accepted.

### Which LLM providers are supported?

Twelve: AnthropicApi, ClaudeCli, CodexCli, OpenAiCompat, CursorAcp, CursorCli,
PerplexityApi, GeminiApi, GeminiCli, CerebrasApi, Hermes, OpenClaw. The cascade
router learns which to use for which tasks.

### Can I use Roko without an LLM?

No. The core value proposition requires an LLM for code generation. However, the gate
pipeline, knowledge store, and learning loops work independently and could be reused
without the agent dispatch layer.

### What Rust version do I need?

1.91 or newer. The alloy dependencies (EVM stack) set this floor. Formatting requires
nightly (`cargo +nightly fmt --all`).

### Where is all the state stored?

In the `.roko/` directory inside your project workspace. There is no global database.
Each project is self-contained. Copy the directory and you bring the full history.

### What is the Graph engine?

The sole execution engine for plans. It runs task DAGs as parallel waves of cells with
cost enforcement, conditional routing, and durable checkpoints. The older Runner-v2
is retained as `--engine legacy` for one deprecation cycle.

### How does the cascade router cold-start?

Three stages:
- **Static** (0-49 observations): uses a hardcoded config table mapping task role to model.
- **Confidence** (50-199 observations): uses empirical pass rates with confidence intervals.
- **UCB** (200+ observations): full LinUCB contextual bandit with features (role, complexity,
  domain, cost).

### Is the state format stable?

JSONL files are append-only and forward-compatible. New fields are added with
`#[serde(default)]` so old files remain readable. Checkpoint format includes a schema
version for migration.

### How do I contribute?

1. Read `CLAUDE.md` and `.roko/GAPS.md`.
2. Run pre-commit checks: `cargo +nightly fmt --all`, `cargo clippy --workspace --no-deps -- -D warnings`, `cargo test --workspace`.
3. Search before writing -- this codebase has duplicate implementations from parallel development.
4. Wire existing code before building new code. The most common pattern is "built but never connected."

---

## 16. Where to go next

Read these documents in order based on what you want to understand:

1. **Architecture Guide** (`docs/v3/35-ARCHITECTURE.md`) -- the full system overview
   with diagrams, the complete crate map (all 9 tiers), and key design decisions.

2. **Data Flow Diagrams** (`docs/v3/depth/35-architecture/data-flow-diagrams.md`) --
   ASCII diagrams showing how data moves through the system for plan execution, agent
   dispatch, gate validation, knowledge query, and dream consolidation.

3. **Design Decisions** (`docs/v3/depth/35-architecture/design-decisions-and-rationale.md`)
   -- every major architectural WHY: everything-is-a-Signal, Graph sole engine, 19
   gates, cascade routing, affect dispatch, JSONL state, workspace-local, 12 traits,
   CoALA mapping.

4. **Crate Dependency Graph** (`docs/v3/depth/35-architecture/crate-dependency-graph.md`)
   -- which crate depends on what, compilation order, tier layers, key metrics.

5. **CLAUDE.md** (workspace root) -- the canonical project context file. This is what
   AI agents read when working on the codebase. It contains the most up-to-date status
   information, CLI command reference, absolute paths, and build instructions.

6. **Gap Tracker** (`.roko/GAPS.md`) -- what is not yet done. Always check this before
   starting new work to avoid duplicating effort.

7. **Source code** -- follow the reading order in Section 9: start with signal.rs and
   traits.rs, then trace an execution path through the runner, compose, agent, and gate
   crates.
