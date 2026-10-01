# Crate Dependency Graph

> **Verified against codebase: 2026-09-15.**
> Generated from `cargo metadata --no-deps`. Shows only intra-workspace
> dependencies (external crates like tokio, serde, axum are omitted).
> 39 workspace members. 9 tiers. ~130 intra-workspace edges.

---

## Table of Contents

1. [Tier Architecture](#1-tier-architecture)
2. [Full Dependency Edges](#2-full-dependency-edges)
3. [Dependency Graph (ASCII)](#3-dependency-graph-ascii)
4. [Compilation Order](#4-compilation-order)
5. [Default Build Targets](#5-default-build-targets)
6. [Fan-in and Fan-out Analysis](#6-fan-in-and-fan-out-analysis)
7. [Compilation Islands](#7-compilation-islands)
8. [Incremental Build Impact](#8-incremental-build-impact)
9. [Key Metrics](#9-key-metrics)
10. [Design Notes](#10-design-notes)

---

## 1. Tier Architecture

The 39 workspace crates are organized into 9 tiers. Lower tiers are foundations;
higher tiers are user-facing entry points. Dependencies flow strictly upward: a
crate in tier N may depend on crates in tiers 1 through N-1, but never on a crate
in a higher tier. Circular dependencies are impossible within the tier system.

```
Tier 9: Apps & Tests
  speed-test, roko-demo, roko-chain-watcher, roko-tests, mirage-rs

Tier 8: Chain & Economy
  roko-chain

Tier 7: MCP, Plugin & Gateway
  roko-mcp-code, roko-mcp-github, roko-mcp-slack, roko-mcp-scripts
  roko-mcp-stdio, roko-plugin, roko-gateway, roko-eval

Tier 6: Code Intelligence
  roko-index, roko-lang-rust, roko-lang-typescript, roko-lang-go

Tier 5: External Interfaces (user-facing binaries)
  roko-cli, roko-serve, roko-acp, roko-agent-server, agent-relay

Tier 4: Learning & Memory
  roko-learn, roko-neuro, roko-dreams, roko-daimon

Tier 3: Agent, Composition & Verification
  roko-agent, roko-compose, roko-gate, roko-conductor

Tier 2: Execution Engine
  roko-graph, roko-execution, roko-runtime

Tier 1: Core Kernel
  roko-primitives, roko-core, roko-fs, roko-std
```

### Tier descriptions

**Tier 1: Core Kernel** -- Defines the vocabulary. Signal type, 12 protocol traits,
config schema, error types, HDC vectors, file substrate, and default trait
implementations. Everything else depends on this tier.

**Tier 2: Execution Engine** -- The machinery that runs task DAGs. The Graph engine
is the sole executor. roko-execution provides a shared service facade so CLI, serve,
and ACP do not duplicate setup code. roko-runtime provides the process supervisor,
event bus, and cancellation tokens.

**Tier 3: Agent, Composition & Verification** -- The operational core. LLM dispatch
(12 backends), prompt assembly (9 layers), gate verification (19 gates in 7 rungs),
and orchestration monitoring (12 watchers with circuit breaker).

**Tier 4: Learning & Memory** -- The feedback loop. Episodes, model routing, playbooks,
experiments (roko-learn). Durable knowledge with tier progression (roko-neuro). Offline
consolidation via dream cycles (roko-dreams). Affect-aware dispatch modulation
(roko-daimon).

**Tier 5: External Interfaces** -- User-facing binaries that wire internal crates
together. The CLI (roko-cli), HTTP control plane (roko-serve), editor protocol
(roko-acp), per-agent sidecar (roko-agent-server), and relay server (agent-relay).
These contain minimal domain logic -- they are integration points.

**Tier 6: Code Intelligence** -- Static analysis and language support. Tree-sitter
parsing for Rust, TypeScript, and Go. Symbol graph, PageRank scoring, HDC fingerprints.

**Tier 7: MCP, Plugin & Gateway** -- Tool ecosystem. MCP servers for code intelligence,
GitHub, Slack, and scripts. Plugin SDK with signed dependency graphs and WASM hooks.
Nine-stage inference gateway.

**Tier 8: Chain & Economy** -- Local state machines for registry, marketplace, arena,
and DeFi primitives. Tested but production transport/persistence adapters are Phase 2+.

**Tier 9: Apps & Tests** -- Standalone binaries, demos, and end-to-end integration tests.

---

## 2. Full Dependency Edges

Each entry shows `crate -> [workspace dependencies]`. Crates with no intra-workspace
dependencies are leaf nodes. All edges are verified from `cargo metadata --no-deps`.

### Tier 1: Core Kernel

```
roko-primitives -> (none)
    Leaf node. HDC vectors (10,240-bit), PAD affect vectors, tier routing math.
    No dependencies on any other workspace crate. Compiles first.

roko-core -> roko-primitives
    The kernel. Signal type, 12 protocol traits, Cell trait, config schema,
    tool system, error types, Kind enum, Body, Decay, Score, Context, Query.
    Nearly everything depends on this.

roko-fs -> roko-core, roko-primitives
    JSONL substrate on disk. FileSubstrate, GC, workspace layout (RokoLayout).

roko-std -> roko-core, roko-chain
    Default trait implementations: SumScorer, StaticToolRegistry, MemorySubstrate.
    The roko-chain dep is for ChainSubstrate stubs (cross-tier, but the dep is
    minimal -- just type re-exports).
```

### Tier 2: Execution Engine

```
roko-graph -> roko-core
    Sole execution engine since #260/#276. DAG cells, topological wave execution,
    conditional routing, cost enforcement, checkpoints, immune decision graph,
    graph fingerprinting, budget.rs, snapshot.rs, topo.rs.

roko-runtime -> roko-core, roko-primitives, roko-compose, roko-gate, roko-learn
    ProcessSupervisor, event bus, cancellation tokens, workflow contract types.
    Cross-cuts composition and gating for PipelineStateV2.
    NOTE: the deps on compose/gate/learn are for pipeline state types, not
    for calling their logic directly.

roko-execution -> roko-core, roko-agent, roko-compose, roko-fs,
                  roko-gate, roko-graph, roko-learn, roko-neuro, roko-runtime
    Shared RuntimeServices builder (#243). Gives CLI, serve, and ACP a
    common layer for safety, budget, routing, and feedback setup.
    Wide dependency fan-out is intentional: this is the assembly point.
```

### Tier 3: Agent, Composition & Verification

```
roko-agent -> roko-core, roko-fs, roko-graph, roko-std
    12 LLM provider backends (AnthropicApi, ClaudeCli, CodexCli, OpenAiCompat,
    CursorAcp, CursorCli, PerplexityApi, GeminiApi, GeminiCli, CerebrasApi,
    Hermes, OpenClaw). Agent pools, MCP tool integration, tool dispatch loop,
    safety layer with role-based access, immune boundary. Largest non-CLI crate.

roko-compose -> roko-core, roko-primitives, roko-agent, roko-daimon,
                roko-dreams, roko-graph, roko-learn, roko-neuro, roko-std
    9-layer SystemPromptBuilder, 11 role templates (implementer, reviewer,
    researcher, refactorer, strategist, conductor, integration, scribe, quick,
    task_impl, common). Enrichment pipeline, context assembly, symbol resolution.
    Wide fan-out because it injects playbooks (learn), knowledge (neuro),
    affect guidance (daimon), and dream insights (dreams) into prompts.

roko-gate -> roko-core, roko-agent, roko-std
    19 gate types in a 7-rung pipeline. Adaptive EMA thresholds. CompileGate,
    ClippyGate, TestGate, SymbolGate, GeneratedTestGate, PropertyTestGate,
    IntegrationGate, LlmJudgeGate, DiffGate, FactCheckGate, VerifyChainGate,
    CodeExecutionGate, ShellGate, BenchmarkRegressionGate, FormatCheckGate,
    SecurityScanGate, plus gate_pipeline, rung_dispatch, rung_selector.
    Uses roko-agent for LlmJudgeGate (LLM evaluates code quality).

roko-conductor -> roko-core, roko-learn
    12 reactive watchers, circuit breaker, diagnosis engine.
    Monitors for: stuck agents, budget exhaustion, quality degradation,
    latency spikes, cost anomalies, and other operational anomalies.
```

### Tier 4: Learning & Memory

```
roko-daimon -> roko-core
    Affect engine. PAD state (pleasure/arousal/dominance), somatic markers,
    dispatch modulation (temperature, turn budget, exploration rate).
    Minimal dependencies -- pure emotional computation. Also includes
    GoalTree, life review, mortality emotions, and prospect theory valuation.

roko-learn -> roko-core, roko-primitives, roko-agent, roko-daimon, roko-fs
    Episodes, when/then playbooks, multi-armed bandits, cascade model routing
    (Static -> Confidence -> UCB), prompt A/B experiments, efficiency tracking,
    HDC clustering, hindsight adjustments, c-factor governance, Variance
    Inequality, attention fit, anomaly detection, cross-session costs,
    cross-workspace transfer, curriculum learning. ~60 source files.

roko-neuro -> roko-core, roko-primitives, roko-agent, roko-fs, roko-learn
    Durable knowledge store. Entry types (fact, insight, heuristic, procedure,
    constraint, anti-knowledge), half-lives, tier progression (Transient ->
    Working -> Reference), distillation, context assembly, HDC lookup,
    temporal queries, admission control, sync protocol, episode completion.

roko-dreams -> roko-core, roko-primitives, roko-agent, roko-learn, roko-neuro
    Offline consolidation. DreamCycle with 7 phases: staging, hypnagogia
    (loosened association discovery), imagination (counterfactual analysis),
    rehearsal (threat replay), distillation, playbook promotion, routing advice.
    Adaptive idle scheduling for daemon mode.
```

### Tier 5: External Interfaces

```
roko-cli -> roko-core, roko-agent, roko-agent-server, roko-acp,
            roko-chain, roko-compose, roko-conductor, roko-daimon,
            roko-dreams, roko-execution, roko-fs, roko-gate, roko-graph,
            roko-index, roko-learn, roko-mcp-github, roko-neuro,
            roko-plugin, roko-runtime, roko-serve, roko-std, agent-relay
    Main binary. 85+ CLI subcommands, plan runner (event_loop, plan_dag,
    gate_dispatch, resume, merge), worktree manager, ratatui TUI (10 tabs,
    F1-F10), chat, PRD lifecycle, research, knowledge, learning inspection,
    config management, graph/feed/recipe/trigger commands.
    Depends on 22 workspace crates -- the widest fan-out in the workspace.

roko-serve -> roko-core, roko-agent, roko-agent-server, roko-chain,
              roko-compose, roko-daimon, roko-dreams, roko-execution,
              roko-fs, roko-gate, roko-gateway, roko-graph, roko-learn,
              roko-neuro, roko-plugin, roko-primitives, roko-runtime, roko-std
    HTTP control plane. REST routes (counts in tools/http_route_inventory.snapshot.json) +
    SSE + WebSocket on port 6677. StateHub push-based dashboard. PeriodicObserver
    for telemetry sampling. Relay subscription execution. Arena/meta-agent services.
    Depends on 18 workspace crates.

roko-acp -> roko-core, roko-agent, roko-compose, roko-dreams,
            roko-execution, roko-gate, roko-learn, roko-neuro,
            roko-runtime, roko-serve
    Agent Client Protocol server for editor integration (Cursor, etc.).
    Mutation consent, budget enforcement, health-aware provider selection,
    experiment assignment. 180 tests. Depends on roko-serve for shared
    route infrastructure.

roko-agent-server -> roko-core, roko-agent, roko-chain, roko-fs,
                     roko-learn, roko-neuro, agent-relay
    Per-agent HTTP sidecar. 14 routes: /message (real LLM dispatch),
    /stream (WebSocket), /predictions, /research, /tasks, and more.

agent-relay -> roko-core, roko-plugin
    Bounded canonical-envelope relay server with atomic cursor restore,
    fail-closed reconciliation, and ACK-after-durable exact-room
    subscription terminalization.
```

### Tier 6: Code Intelligence

```
roko-lang-rust -> roko-core
roko-lang-typescript -> roko-core
roko-lang-go -> roko-core
    Language providers. Tree-sitter parsing for each language.
    Each depends only on roko-core. Independent of each other.

roko-index -> roko-core, roko-primitives, roko-lang-rust,
              roko-lang-typescript, roko-lang-go
    Source code parser, symbol graph, PageRank scoring, HDC fingerprints.
    Aggregates all three language providers.
```

### Tier 7: MCP, Plugin & Gateway

```
roko-mcp-stdio -> (none)
    Leaf node. Shared JSON-RPC 2.0 transport for all MCP servers.
    No workspace dependencies at all.

roko-mcp-github -> roko-mcp-stdio
roko-mcp-slack -> roko-mcp-stdio
roko-mcp-scripts -> roko-mcp-stdio
    MCP server binaries. Each depends only on the shared transport.
    Form a self-contained compilation island.

roko-mcp-code -> roko-core, roko-index, roko-mcp-stdio
    Code-intelligence MCP server. Bridges the MCP island into the
    index/language crate subtree.

roko-plugin -> roko-core
    Plugin SDK. Signed dependency graphs, WASM hooks, strict admission,
    kernel confinement, verified registry install/publish.

roko-gateway -> roko-core, roko-agent, roko-graph, roko-learn
    Nine-stage inference gateway: routing/fallback, exact and semantic
    caches, tool/output/thinking controls, convergence, cost accounting,
    key rotation, three-level backpressure, handles, batches, events.

roko-eval -> roko-core
    Evaluation framework. Evidence collector, criterion, profile traits.
```

### Tier 8: Chain & Economy

```
roko-chain -> roko-core
    Local registry, marketplace, arena, and DeFi state machines. X402
    payments, identity, witness. Production transport/persistence
    adapters remain Phase 2+. The deprecated rate-oracle vertical
    has been removed.
```

### Tier 9: Apps & Tests

```
mirage-rs -> roko-core, roko-primitives, roko-runtime
    In-process EVM fork simulator.

roko-chain-watcher -> (none)
    Long-running chain observation agent. Standalone binary.
    Leaf node -- no workspace dependencies.

roko-demo -> (none)
    Demo/example binary. Currently disconnected. Leaf node.

speed-test -> (none)
    Benchmark crate. Leaf node.

roko-tests -> roko-core, roko-agent, roko-compose, roko-fs, roko-gate, roko-std
    End-to-end integration test suite. Depends on 6 workspace crates.
```

---

## 3. Dependency Graph (ASCII)

This diagram shows the primary dependency edges. Minor edges are omitted for
clarity. Read bottom-to-top: foundations at the bottom, user-facing at the top.

```
                              roko-cli (22 deps)
                           /   |    |   \
                     roko-acp  |  roko-serve (18 deps)  roko-agent-server
                       |       |     |    \        |
                       +---+---+     |     +-------+
                           |         |         |
                     roko-execution  |     agent-relay
                      /  |  |  \     |
            roko-gate |  |  |  roko-runtime
               |      |  |  |     |
               +------+--+--+-----+
                      |
              +-------+--------+
              |                |
         roko-compose    roko-conductor
          / |  |  \           |
         /  |  |   \          |
        /   |  |    \         |
       /    |  |     \        |
  roko-   roko- roko-  roko-dreams
  graph   learn neuro      |
    |      / |    |         |
    |     /  |    |         |
    |    /   |    +---------+
    |   /    |         |
    |  /  roko-daimon  |
    | /      |         |
    |/       |         |
  roko-agent |         |
    |  \     |         |
    |   \    |         |
    |    roko-fs       |
    |      |           |
    |      |           |
  roko-std |           |
    |      |           |
    +------+--------+--+
           |        |
         roko-core  |
           |        |
      roko-primitives


   Independent tracks (no cross-deps to main tree):

   roko-mcp-stdio (leaf)
     |
     +-- roko-mcp-github
     +-- roko-mcp-slack
     +-- roko-mcp-scripts

   roko-mcp-code -> roko-index -> roko-lang-{rust,typescript,go} -> roko-core

   roko-chain -> roko-core (standalone)
   roko-plugin -> roko-core (standalone)
   roko-eval -> roko-core (standalone)
   roko-gateway -> roko-core + roko-agent + roko-graph + roko-learn
```

---

## 4. Compilation Order

Based on the dependency graph, crates compile in this approximate order. Crates at
the same level can compile in parallel. The total depth is 13 levels.

```
Level 1 (leaves -- compile first, no workspace deps):
  roko-primitives, roko-mcp-stdio, speed-test, roko-demo,
  roko-chain-watcher

Level 2 (depends only on Level 1):
  roko-core

Level 3 (depends on Levels 1-2):
  roko-fs, roko-graph, roko-chain, roko-plugin, roko-eval,
  roko-daimon, roko-lang-rust, roko-lang-typescript, roko-lang-go,
  roko-mcp-github, roko-mcp-slack, roko-mcp-scripts

Level 4 (depends on Levels 1-3):
  roko-std, roko-agent, roko-index

Level 5 (depends on Levels 1-4):
  roko-gate, roko-learn, roko-mcp-code, agent-relay, mirage-rs

Level 6 (depends on Levels 1-5):
  roko-neuro, roko-conductor, roko-gateway

Level 7 (depends on Levels 1-6):
  roko-compose, roko-dreams

Level 8 (depends on Levels 1-7):
  roko-runtime

Level 9 (depends on Levels 1-8):
  roko-execution

Level 10 (depends on Levels 1-9):
  roko-agent-server, roko-tests

Level 11 (depends on Levels 1-10):
  roko-serve

Level 12 (depends on Levels 1-11):
  roko-acp

Level 13 (last -- depends on nearly everything):
  roko-cli
```

### Parallel compilation breakdown

At each level, all listed crates can compile simultaneously on separate cores:

- Level 1: 5 crates in parallel
- Level 3: 12 crates in parallel (the widest wave)
- Level 4: 3 crates in parallel
- Levels 5-7: 2-5 crates each
- Levels 8-13: 1-2 crates each (serial bottleneck at the top)

The CLI compiles last because it depends on 22 other workspace crates. This is
by design -- it is the integration point, not a library.

---

## 5. Default Build Targets

The workspace defines three default members. A plain `cargo build` compiles only
these three binaries (and their transitive dependencies):

```toml
default-members = [
    "crates/roko-cli",       # main binary: roko
    "crates/roko-mcp-code",  # code-intelligence MCP server
    "crates/roko-mcp-github" # GitHub MCP server
]
```

To build the full workspace: `cargo build --workspace`.

To build only the CLI: `cargo build -p roko-cli`.

To run tests for a single crate: `cargo test -p roko-gate`.

---

## 6. Fan-in and Fan-out Analysis

### Highest fan-in (most depended-upon)

These crates are depended on by the most other workspace crates. Changes to these
trigger the widest recompilation.

| Crate | Depended on by | Impact |
|-------|---------------|--------|
| **roko-core** | ~36 crates | Nearly universal. Any change recompiles almost everything. |
| **roko-primitives** | ~12 crates | Wide but less than core. HDC/PAD math. |
| **roko-agent** | ~10 crates | All interface crates + compose + gate + learn. |
| **roko-learn** | ~9 crates | Compose, conductor, runtime, execution, plus interfaces. |
| **roko-fs** | ~7 crates | Agent, learn, neuro, execution, agent-server, CLI, serve. |
| **roko-core** | ~36 crates | |

### Highest fan-out (most dependencies)

These crates depend on the most other workspace crates. They are integration points.

| Crate | Depends on | Role |
|-------|-----------|------|
| **roko-cli** | 22 crates | Main binary. Wires everything together. |
| **roko-serve** | 18 crates | HTTP control plane. |
| **roko-acp** | 10 crates | Editor integration. |
| **roko-execution** | 9 crates | Shared service facade. |
| **roko-compose** | 9 crates | Prompt assembly (needs learn, neuro, daimon, dreams). |

---

## 7. Compilation Islands

The workspace has several independent compilation subtrees that share no edges with
each other (except through roko-core):

### Island 1: MCP servers

```
roko-mcp-stdio (leaf, no workspace deps)
  +-- roko-mcp-github
  +-- roko-mcp-slack
  +-- roko-mcp-scripts
```

These four crates form a completely independent island. Changes to the main tree
do not affect them, and vice versa. They can be built and tested in isolation.

### Island 2: Code intelligence

```
roko-mcp-code -> roko-index -> roko-lang-{rust,typescript,go} -> roko-core
```

This subtree is only reachable from the main tree via roko-cli (which depends on
roko-index). Changes to language providers do not affect agent dispatch, gating,
or learning.

### Island 3: Chain economy

```
roko-chain -> roko-core (only)
```

roko-chain is a standalone subtree. It is depended on by roko-std (for ChainSubstrate
stubs), roko-cli, roko-serve, and roko-agent-server, but changes to roko-chain do not
propagate into the core execution path.

### Island 4: Standalone apps

```
roko-chain-watcher -> (none)
roko-demo -> (none)
speed-test -> (none)
```

These three leaf crates have zero workspace dependencies and zero dependents.

---

## 8. Incremental Build Impact

When you change a file, only the changed crate and its dependents recompile. Here
is the impact of changes to key crates:

| Changed crate | Recompiles | Approximate count |
|--------------|-----------|-------------------|
| roko-primitives | roko-core + everything above | ~36 crates |
| roko-core | Everything except roko-primitives and leaf crates | ~35 crates |
| roko-daimon | roko-learn, roko-compose, roko-serve, roko-cli, etc. | ~8 crates |
| roko-graph | roko-agent, roko-compose, roko-gateway, roko-execution, interfaces | ~10 crates |
| roko-gate | roko-runtime, roko-execution, roko-tests, interfaces | ~7 crates |
| roko-learn | roko-neuro, roko-compose, roko-conductor, roko-runtime, etc. | ~10 crates |
| roko-agent | roko-gate, roko-learn, roko-compose, roko-dreams, etc. | ~12 crates |
| roko-cli | Only roko-cli itself | 1 crate |
| roko-mcp-github | Only roko-mcp-github and roko-cli | 2 crates |
| roko-lang-rust | roko-index, roko-mcp-code, roko-cli | 3 crates |

**Key insight**: Changes to leaf crates (roko-daimon, roko-plugin, roko-eval) are
cheap. Changes to roko-core or roko-primitives are expensive. This is intentional:
the kernel changes rarely, and when it does, you want a full rebuild to catch
breakage.

---

## 9. Key Metrics

| Metric | Value |
|--------|-------|
| Workspace members | 38 |
| Intra-workspace dependency edges | ~130 |
| Maximum dependency chain depth | 13 levels |
| Leaf crates (no workspace deps) | 5 (roko-primitives, roko-mcp-stdio, speed-test, roko-demo, roko-chain-watcher) |
| Heaviest dependency fan-in | roko-core (depended on by ~36 crates) |
| Heaviest dependency fan-out | roko-cli (depends on 22 workspace crates) |
| Widest parallel compilation wave | Level 3 (12 crates) |
| Independent compilation islands | 4 |
| Minimum Rust version | 1.91 (alloy deps) |
| Edition | 2024 |

---

## 10. Design Notes

- **roko-core is the universal dependency.** Nearly every crate depends on it. Changes
  to roko-core trigger the widest recompilation. This is intentional: the kernel
  defines the vocabulary (Signal, traits, config) that all other crates speak. It
  changes rarely.

- **Tier 5 crates are integration points, not business logic.** roko-cli, roko-serve,
  and roko-acp wire internal crates together and expose them to users. They contain
  minimal domain logic of their own. Their wide dependency fan-out is by design.

- **The MCP track is nearly independent.** roko-mcp-stdio and the three MCP server
  crates form a separate compilation island with minimal coupling to the main tree.
  Only roko-mcp-code bridges into the index/language crates.

- **roko-execution is the shared service facade.** Introduced by PR #243 to give
  CLI, serve, and ACP a common runtime services layer, reducing duplicated setup code.
  Its wide fan-out (9 deps) is intentional: it assembles all the services that the
  interface crates need.

- **roko-compose has cross-tier dependencies.** It depends on roko-daimon (Tier 4) and
  roko-dreams (Tier 4) for injecting affect guidance and dream insights into prompts.
  This means roko-compose is technically Tier 4+ even though its primary role
  (prompt assembly) is Tier 3 work. The cross-tier deps are deliberate: prompts need
  to include information from the learning/memory stack.

- **roko-std has a cross-tier dep on roko-chain.** The roko-std crate depends on
  roko-chain for ChainSubstrate type stubs. This is a minimal dependency (just type
  re-exports) that does not introduce significant coupling.

- **The Graph engine (roko-graph) depends only on roko-core.** This is a key design
  property: the execution engine is independent of agent dispatch, gating, learning,
  and composition. It operates purely on the Cell/Graph abstraction. The concrete
  cells that wrap agents, gates, etc. are defined in their respective crates and
  registered at runtime.
