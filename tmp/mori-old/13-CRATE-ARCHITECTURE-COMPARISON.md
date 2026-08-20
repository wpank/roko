# Crate Architecture Comparison: Bardo/Mori vs Roko

Generated: 2026-08-19

## Overview

| Metric | Bardo/Mori | Roko |
|---|---|---|
| Total Rust LOC | ~337K | ~893K |
| Workspace members (crates/) | 34 | 31 |
| Workspace members (apps/) | 6 | 3 |
| Workspace members (other) | 1 (tests/harness) | 1 (tests/) |
| **Total workspace members** | **41** | **35** |
| Minimum Rust version | 1.85 | 1.91 |
| Edition | 2024 | 2024 |
| Default members | all | roko-cli, roko-mcp-code, roko-mcp-github |

Roko is 2.65x the size of bardo by LOC. The bardo codebase includes
both the golem agent runtime (the autonomous DeFi agent) and the mori
orchestrator (the plan execution system). Roko subsumes both roles into a
single unified architecture.

---

## 1. Bardo/Mori Crate Inventory

### Layer 0 -- Zero-dependency primitives

| Crate | LOC | Purpose |
|---|---|---|
| `bardo-primitives` | 410 | 10,240-bit HDC vectors, inference tier routing, zero-dependency compute primitives |
| `bardo-inference` | 413 | Inference protocol wire types: messages, requests, responses, streaming chunks for Anthropic/OpenAI APIs |

### Layer 0 -- Core

| Crate | LOC | Purpose |
|---|---|---|
| `golem-core` | 9,025 | GolemId, PADVector, CognitiveTier, GolemConfig, CorticalState, EventFabric, TaintLabel, bump allocator, extensions |

### Layer 1 -- Runtime

| Crate | LOC | Purpose |
|---|---|---|
| `golem-runtime` | 7,847 | Cognitive state management: MindState, WorkingMemory, AttentionSalience, HabituationMask, SleepPressure, HomeostasisRegulator, CognitiveEngine, StateManager, prediction, reflexion, learning loop |

### Layer 2 -- Subsystems

| Crate | LOC | Purpose |
|---|---|---|
| `golem-grimoire` | 15,019 | LanceDB episodic store, SQLite semantic store, PLAYBOOK.md, four-factor retrieval, Ebbinghaus demurrage, A-MAC admission, Curator cycles |
| `golem-daimon` | 9,283 | ALMA three-layer EMA affect engine, OCC/Scherer appraisal, somatic markers (k-d tree), mood-congruent retrieval, mortality affect, dream scheduling, clade contagion |
| `golem-mortality` | 10,468 | Three death clocks (economic, epistemic, stochastic), VitalityState, behavioral phases, knowledge demurrage, fractal mortality, thanatopsis |
| `golem-economy` | 323 | Knowledge marketplace, commerce bazaar, Styx integration, memory demurrage, agent revenue (shell) |
| `golem-dreams` | 2,844 | Dream scheduling (NREM/REM), consolidation, Styx dream-packet submission, memetic evolution |
| `golem-context` | 3,995 | CognitiveWorkspace assembly, ContextPolicy, per-category token allocation, background fiber |

### Layer 3 -- Safety

| Crate | LOC | Purpose |
|---|---|---|
| `golem-safety` | 4,869 | Capability tokens, taint-aware ingestion, audit chains, loop detection, permits, allowlists, PolicyCage |

### Layer 4 -- External integration

| Crate | LOC | Purpose |
|---|---|---|
| `golem-inference` | 2,349 | Tier routing, inference types, gateway client (HTTP) |
| `golem-chain` | 6,206 | Alloy provider, ERC-8004, Permit2, Warden, revm simulation, block/log types, 12-chain registry |
| `golem-identity` | 7,146 | On-chain identity, reputation, ERC-8004 write ops, clade replication, x402 |
| `golem-chain-intelligence` | 9,717 | Block ingestion, chain witness, ABI decoder, indexer, protocol state, event router, streaming API |
| `golem-triage` | 8,291 | Curiosity scoring, HDC fingerprints, Bayesian surprise, ANN lookup, anomaly detection (ADWIN, BOCPD), Thompson routing |
| `golem-ta` | 11 | Technical analysis: TDA, Betti curves, regime detection (shell) |
| `golem-uniswap` | 2,118 | Uniswap V3/V4: ActionPermit pipeline, PolicyCage, WardenHandle, position management, hook ABI |
| `golem-oneirography` | 11 | Dream journal, death mask minting, SuperRare integration, lineage graph (shell) |
| `golem-tools` | 10,234 | DeFi tool library: 423+ tools, ToolExecutor, ToolRegistry, feature-gated categories (data/trading/lp/vault/safety/memory/identity) |

### Layer 5 -- Coordination

| Crate | LOC | Purpose |
|---|---|---|
| `golem-coordination` | 11 | Pheromone field client, clade sync, bloodstain ingestion, PropagationPolicy (shell) |

### Layer 6 -- Presentation

| Crate | LOC | Purpose |
|---|---|---|
| `golem-surfaces` | 2,869 | WebSocket, SSE, Telegram, Discord connector surfaces for state streaming |
| `golem-creature` | 11 | Creature visual state, evolution forms (Egg->Hatchling->Mature->Weathered->Transcendent) (shell) |
| `golem-engagement` | 11 | Achievement engine, death recap, graveyard, toast/notification events (shell) |
| `golem-sonification` | 5,641 | Modular synth driven by CorticalState: real-time audio, CV mapper, rack processor, cpal output |

### Layer 6.5 -- HTTP/WS API

| Crate | LOC | Purpose |
|---|---|---|
| `golem-api` | 2,244 | Health probes, REST snapshots, WebSocket event stream (62 EventPayload variants across 16 subsystems), SSE, Prometheus metrics, admin on :8402 |

### Layer 7 -- Binary

| Crate | LOC | Purpose |
|---|---|---|
| `golem-binary` | 3,617 | The `bardo-golem` binary: CLI entry point wiring all golem crates |

### Evaluation

| Crate | LOC | Purpose |
|---|---|---|
| `golem-eval` | 2,179 | MVP gate: end-to-end thesis validation for the mortality thesis |

### Mori subsystem (within bardo workspace)

| Crate | LOC | Purpose |
|---|---|---|
| `mori-index` | 5,605 | Incremental Rust code intelligence: tree-sitter parsing, PageRank, HDC fingerprints, hybrid search |
| `mori-context` | 702 | Context assembly from code search results to structured markdown blocks |
| `mori-mcp` | 3,331 | MCP server + CLI for code intelligence: search, navigation, context tools over stdio JSON-RPC |

### Protocol/Payment

| Crate | LOC | Purpose |
|---|---|---|
| `mpp` | 988 | Machine Payment Protocol: HTTP 402 types, ERC-3009 verification, session management, USDC settlement |

### Apps (binaries)

| App | LOC | Purpose |
|---|---|---|
| `mori` | 107,915 | **The orchestrator**: plan execution, agent spawning, TUI, git integration, conductor, deployment, server, state management. By far the largest component |
| `mori-service` | 7,742 | HTTP service: API endpoints, proposal system, GitHub/Twitter integrations, SQLite state |
| `bardo-gateway` | 23,115 | LLM inference proxy: three-layer caching, multi-provider routing, cost tracking, USDC micropayments |
| `bardo-styx` | 209 | Styx relay: clade sync, knowledge exchange, pheromone field semantics |
| `bardo-terminal` | 35,995 | Ratatui TUI for live golem observation: cortical state, grimoire, heartbeat, chain activity |
| `bardo-compute` | 30 | Compute node binary (shell) |
| `mirage-rs` | 14,129 | In-process Ethereum fork simulator: lazy upstream, copy-on-write branching, JSON-RPC |

### Test harness

| Crate | LOC | Purpose |
|---|---|---|
| `tests/harness` | -- | Shared integration test utilities |

---

## 2. Roko Crate Inventory

### Layer 0 -- Primitives

| Crate | LOC | Purpose |
|---|---|---|
| `roko-primitives` | 4,833 | 10,240-bit HDC vectors, inference tier routing (forked from bardo-primitives, extended) |

### Layer 0/1 -- Kernel

| Crate | LOC | Purpose |
|---|---|---|
| `roko-core` | 75,122 | Signal/Engram type, 12 kernel traits (Store, ColdStore, Score, Verify, Route, Compose, React, Bus, Observe, Connect, Trigger, Substrate), types, config, tools, errors, safety contracts, trust lattice, immune graph, taint tracking |
| `roko-runtime` | 33,505 | ProcessSupervisor, typed event bus, cancellation, relay client, worker auth, disk admission |

### Layer 2 -- Trait implementations and subsystems

| Crate | LOC | Purpose |
|---|---|---|
| `roko-std` | 10,951 | Standard trait implementations: 35 tool definitions, in-memory substrate, NoOp defaults, composable scorers, MCP clients/resolvers |
| `roko-fs` | 8,609 | Filesystem-backed Substrate: JSONL append-only persistence with in-memory index, GC, disk layout |
| `roko-compose` | 29,022 | 9-layer SystemPromptBuilder, 11 role templates, prompt assembly with token budgets, context packs, attention bidders |
| `roko-plugin` | 5,792 | Plugin SDK: manifests, declarative tools, tier/capability policy, semver resolution, WASM hooks, signed dependency graphs, registry install/publish |
| `roko-learn` | 65,725 | Episodes, playbooks, bandits, model routing (CascadeRouter), experiments (A/B), efficiency tracking, HDC consolidation, hindsight, c-factor governance, significance/early stopping, Variance Inequality, autocatalytic metrics, when/then playbooks |
| `roko-neuro` | 20,555 | Durable knowledge store, distillation, tier progression (Transient->Working->Long-term), balance demurrage/reinforcement, falsifiers, streaming HDC lookup, temporal query/GC, cross-domain transfer |
| `roko-dreams` | 14,286 | Offline consolidation: hypnagogia, imagination, dream cycles, adaptive scheduling, cron triggers, checkpoint restore |
| `roko-daimon` | 8,364 | Affect engine, somatic markers, dispatch modulation, PAD vectors, ALMA-like three-layer model |
| `roko-chain` | 29,296 | Chain client/wallet traits, mock implementations, optional alloy backend, local registry/marketplace/arena/DeFi state machines |
| `roko-graph` | 12,979 | Graph execution engine: DAG types, TOML loader, CellRegistry, seven cognitive Cells, five Verify Cells, immune decision Graph, bounded parallel waves, provider dispatch, budget/cost tracking |
| `roko-index` | 4,500 | Code intelligence: source parsing, symbol graphs, PageRank scoring, HDC fingerprints, multi-language |
| `roko-lang-rust` | 1,390 | Rust/Cargo language provider |
| `roko-lang-typescript` | 938 | TypeScript/JavaScript language provider |
| `roko-lang-go` | 673 | Go language provider |

### Layer 3 -- Mid-level services

| Crate | LOC | Purpose |
|---|---|---|
| `roko-gate` | 23,263 | 19 gates, 7-rung pipeline, adaptive thresholds, oracle rungs 4-6 |
| `roko-agent` | 104,426 | 11 LLM provider kinds (Anthropic, Claude CLI, OpenAI-compat, Cursor ACP/CLI, Perplexity, Gemini API/CLI, Cerebras, Hermes, OpenClaw), pools, MCP, tool loop, safety layer, provider health/rate-limiting |
| `roko-gateway` | 4,812 | Centralized inference pipeline: 9-stage routing/fallback, semantic+exact caches, tool/output/thinking controls, convergence, cost accounting, key rotation, backpressure, batches |
| `roko-conductor` | 11,062 | 12 watchers, circuit breaker, diagnosis, intervention policies |

### Layer 4 -- Application surfaces

| Crate | LOC | Purpose |
|---|---|---|
| `roko-cli` | 234,797 | **The CLI binary**: plan DAG/runner-v2, merge queue, worktree manager, ratatui TUI (F1-F10 tabs), PRD lifecycle, research agent, chat, status, doctor, config, learning commands, dashboard, GitHub integration |
| `roko-serve` | 98,819 | HTTP control plane: ~317 REST routes + SSE + WebSocket on :6677, triggers, feeds, recipes, payments, named surfaces, telemetry lens, cold archival |
| `roko-agent-server` | 5,368 | Per-agent HTTP sidecar: /message (real LLM dispatch), /stream WS, /predictions, /research, /tasks, bearer auth |
| `roko-acp` | 21,579 | ACP (Agent Client Protocol) server for Cursor/external agent integration: mutation consent, experiments, capabilities, budgets, sandboxing |

### MCP servers

| Crate | LOC | Purpose |
|---|---|---|
| `roko-mcp-stdio` | 251 | Shared stdio JSON-RPC transport for MCP servers |
| `roko-mcp-code` | 1,987 | Code intelligence MCP server backed by roko-index |
| `roko-mcp-github` | 4,323 | GitHub API as MCP tools |
| `roko-mcp-slack` | 1,114 | Slack Web API as MCP tools |
| `roko-mcp-scripts` | 766 | Wraps arbitrary scripts as MCP tools |

### Demo/utility

| Crate | LOC | Purpose |
|---|---|---|
| `roko-demo` | 5,860 | Manifest-driven deploy + fixtures + agent-spawn orchestrator for demo environments |

### Apps (binaries)

| App | LOC | Purpose |
|---|---|---|
| `agent-relay` | 6,207 | In-memory relay for agent WebSocket presence, cards, message forwarding, room subscription |
| `mirage-rs` | 38,381 | In-process Ethereum fork simulator (enhanced from bardo version) |
| `roko-chain-watcher` | 2,931 | Long-running agent: subscribes to mirage chain, posts insights via HTTP |

### Tests

| Crate | LOC | Purpose |
|---|---|---|
| `tests/` | -- | End-to-end integration tests |

---

## 3. Crate Mapping Table

### Direct mappings (mori crate -> roko crate)

| Bardo/Mori Crate | Roko Crate | Status | Notes |
|---|---|---|---|
| `bardo-primitives` | `roko-primitives` | **Forked + extended** | 410 -> 4,833 LOC; same HDC vector foundation, roko adds benchmarks and property tests |
| `bardo-inference` | _(absorbed into roko-agent)_ | **Absorbed** | Wire types moved into roko-agent's provider-specific request/response modules |
| `golem-core` | `roko-core` | **Redesigned** | 9K -> 75K LOC. bardo had GolemId/PADVector/EventFabric/extensions; roko has Signal/Engram + 12 kernel traits, trust lattice, immune graph. Fundamentally different architecture |
| `golem-runtime` | `roko-runtime` | **Redesigned** | 8K -> 34K LOC. bardo had cognitive engine/mind state/habituation; roko has ProcessSupervisor, event bus, cancellation, relay client. Different purpose |
| `golem-grimoire` | `roko-neuro` | **Redesigned** | 15K -> 21K LOC. bardo used LanceDB + SQLite + PLAYBOOK.md; roko uses JSONL-backed durable knowledge with tier progression, HDC lookup, distillation. Same concept (knowledge store), different implementation |
| `golem-daimon` | `roko-daimon` | **Ported + simplified** | 9K -> 8K LOC. Same ALMA/PAD/somatic marker foundation. Roko removed mortality-specific and clade contagion features, kept affect engine and dispatch modulation |
| `golem-mortality` | _(removed)_ | **Removed** | Death concepts deliberately removed in roko (naming decision: death/mortality -> removed). Vitality/behavioral phase concepts absorbed into roko-core's CorticalState |
| `golem-economy` | _(absorbed into roko-chain)_ | **Absorbed** | Marketplace/bazaar concepts moved into roko-chain's local state machines |
| `golem-dreams` | `roko-dreams` | **Ported + extended** | 3K -> 14K LOC. Roko adds adaptive scheduling, cron triggers, checkpoint restore; removes Styx WebSocket submission |
| `golem-context` | `roko-compose` | **Redesigned** | 4K -> 29K LOC. bardo had CognitiveWorkspace + ContextPolicy; roko has 9-layer SystemPromptBuilder + 11 role templates + attention bidders. Same concept (context assembly), richer implementation |
| `golem-safety` | _(absorbed into roko-core + roko-agent)_ | **Absorbed + extended** | bardo: 5K LOC standalone crate. roko: safety is distributed across roko-core (trust lattice, taint tracker, immune graph, corrigibility) and roko-agent (provider-level safety). Much more comprehensive in roko |
| `golem-inference` | `roko-gateway` | **Redesigned** | 2K -> 5K LOC. bardo had tier routing + gateway HTTP client; roko has full 9-stage inference pipeline with caching, controls, backpressure. Plus roko-agent subsumes the client role |
| `golem-chain` | `roko-chain` | **Redesigned** | 6K -> 29K LOC. bardo: alloy provider, ERC-8004, Permit2, Warden, revm sim. roko: trait abstractions, optional alloy backend, local registry/marketplace/arena/DeFi state machines |
| `golem-identity` | _(absorbed into roko-chain)_ | **Absorbed** | ERC-8004, reputation, x402 absorbed into roko-chain |
| `golem-chain-intelligence` | _(absorbed into roko-chain + roko-conductor)_ | **Absorbed** | Block ingestion -> roko-chain; watchers/circuit breaker -> roko-conductor |
| `golem-triage` | _(absorbed into roko-conductor)_ | **Partially absorbed** | Curiosity scoring, anomaly detection concepts are in roko-conductor's watchers. Thompson routing -> roko-learn's CascadeRouter |
| `golem-ta` | _(removed)_ | **Removed** | Technical analysis shell was empty, not needed for agent orchestration |
| `golem-uniswap` | _(removed)_ | **Removed** | DeFi-specific tool removed; generic tool system in roko-std/roko-agent |
| `golem-oneirography` | _(removed)_ | **Removed** | Dream journal/NFT minting shell removed |
| `golem-tools` | `roko-std` + `roko-agent` | **Split + redesigned** | bardo: 423 DeFi tools in one crate. roko: 35 tool definitions in roko-std (general-purpose), tool loop execution in roko-agent. DeFi-specific tools dropped |
| `golem-coordination` | _(absorbed into roko-runtime)_ | **Absorbed** | Pheromone/clade coordination shell -> roko-runtime's event bus and relay client |
| `golem-surfaces` | _(absorbed into roko-serve)_ | **Absorbed** | WebSocket/SSE/Telegram/Discord surfaces -> roko-serve's ~317 routes. Named surfaces system (Workbench/Inbox/Canvas/Minimap/Autonomy) is richer |
| `golem-creature` | _(removed)_ | **Removed** | Visual creature lifecycle not needed |
| `golem-engagement` | _(removed)_ | **Removed** | Achievement engine not needed |
| `golem-sonification` | _(removed)_ | **Removed** | Audio synthesis engine removed (5.6K LOC). No audio in roko |
| `golem-api` | `roko-serve` | **Absorbed + extended** | 2K -> 99K LOC. bardo: health + WebSocket + SSE + Prometheus on :8402. roko: full ~317-route HTTP control plane on :6677 with triggers, feeds, recipes, payments, telemetry |
| `golem-binary` | `roko-cli` | **Redesigned** | 4K -> 235K LOC. bardo: simple CLI wiring all golem crates. roko: full plan DAG runner, TUI, PRD lifecycle, research, config management, learning commands. Also subsumes mori app |
| `golem-eval` | _(absorbed into roko-gate)_ | **Absorbed** | bardo: mortality thesis validation. roko: 19-gate 7-rung pipeline in roko-gate |
| `mori-index` | `roko-index` | **Forked** | 6K -> 5K LOC. Same tree-sitter + PageRank + HDC fingerprint foundation. Roko adds multi-language support via separate language crates |
| `mori-context` | _(absorbed into roko-compose)_ | **Absorbed** | Context assembly from code search -> roko-compose's broader prompt assembly system |
| `mori-mcp` | `roko-mcp-code` | **Forked + split** | 3K -> 2K (code) + 251 (stdio) + 4K (github) + 1K (slack) + 1K (scripts). Roko split MCP into 5 purpose-specific servers |
| `mpp` | _(absorbed into roko-serve)_ | **Absorbed** | HTTP 402 payment types -> roko-serve's payments routes (x402 batching, MPP sessions) |
| `mori` (app) | `roko-cli` | **Absorbed + extended** | 108K -> 235K LOC. The entire mori orchestrator (agent spawning, plan execution, TUI, git integration, conductor, server, state) is now inside roko-cli plus extracted crates |
| `mori-service` (app) | `roko-serve` | **Absorbed** | 8K -> 99K LOC. mori-service API/proposals/integrations -> roko-serve's comprehensive HTTP control plane |
| `bardo-gateway` (app) | `roko-gateway` | **Redesigned** | 23K -> 5K LOC. bardo: standalone LLM inference proxy binary. roko: library crate with 9-stage pipeline, integrated into roko-serve. Provider routing now in roko-agent |
| `bardo-styx` (app) | _(removed)_ | **Removed** | Styx relay (clade sync, pheromone fields) removed. Agent relay in roko is a separate simpler design |
| `bardo-terminal` (app) | _(absorbed into roko-cli)_ | **Absorbed** | 36K LOC TUI -> roko-cli's built-in ratatui TUI (F1-F10 tabs via `roko dashboard`) |
| `bardo-compute` (app) | _(removed)_ | **Removed** | 30 LOC compute node shell removed |
| `mirage-rs` (app) | `mirage-rs` (app) | **Preserved + extended** | 14K -> 38K LOC. Same in-process EVM fork simulator, extended with more features |
| `tests/harness` | `tests/` | **Preserved** | Test infrastructure carried forward |

---

## 4. Crates in Roko with No Mori/Bardo Equivalent

These are entirely new in roko -- the cybernetic features that distinguish it
from the original golem agent architecture:

| Roko Crate | LOC | What it adds |
|---|---|---|
| `roko-learn` | 65,725 | **Entire learning subsystem**: episode logging, playbook discovery, cascade model routing, A/B experiments, efficiency tracking, HDC consolidation, hindsight adjustments, c-factor governance, autocatalytic metrics. Nothing like this existed in bardo |
| `roko-agent` | 104,426 | **Multi-provider agent dispatch**: 11 LLM provider kinds, connection pools, MCP tool loop, safety layer, health/rate-limiting. bardo had inference but not provider-neutral agent management |
| `roko-compose` | 29,022 | **9-layer prompt builder**: SystemPromptBuilder with role templates, attention bidders, context enrichment. bardo had context assembly but not structured multi-layer prompt construction |
| `roko-gate` | 23,263 | **19-gate 7-rung pipeline**: compile, test, clippy, diff, shell gates with adaptive thresholds. bardo had golem-eval for thesis validation but not a general-purpose gate system |
| `roko-graph` | 12,979 | **Graph execution engine**: DAG-based cell orchestration, fan-out/fan-in, seven cognitive cells, immune decision graph, provider dispatch within graph nodes |
| `roko-conductor` | 11,062 | **Reactive intelligence**: 12 watchers, circuit breaker, diagnosis. Loosely related to bardo's golem-triage but purpose-built for orchestration |
| `roko-acp` | 21,579 | **Agent Client Protocol**: Cursor/external editor integration, mutation consent, experiments, capabilities, budgets, sandboxing. No bardo equivalent |
| `roko-agent-server` | 5,368 | **Per-agent HTTP sidecar**: /message dispatch, /stream WS, /predictions, bearer auth. bardo agents used different connection model |
| `roko-plugin` | 5,792 | **Plugin SDK**: manifests, WASM hooks, signed dependencies, registry install/publish. bardo had no plugin system |
| `agent-relay` (app) | 6,207 | **WebSocket relay**: agent presence, cards, message forwarding, room subscription. Replaces bardo-styx's pheromone-based approach |
| `roko-chain-watcher` (app) | 2,931 | **Chain observation agent**: subscribes to mirage chain, posts insights |
| `roko-demo` | 5,860 | **Demo orchestrator**: manifest-driven deploy + fixtures + agent spawning |
| `roko-lang-rust` | 1,390 | Language-specific providers (bardo only had Rust via tree-sitter in mori-index) |
| `roko-lang-typescript` | 938 | TypeScript/JavaScript language provider |
| `roko-lang-go` | 673 | Go language provider |
| `roko-mcp-github` | 4,323 | GitHub MCP (bardo had no GitHub tool integration) |
| `roko-mcp-slack` | 1,114 | Slack MCP |
| `roko-mcp-scripts` | 766 | Script-wrapping MCP |

---

## 5. Bardo Crates with No Roko Equivalent

These bardo concepts were **deliberately removed** during the roko migration:

| Bardo Crate | LOC | Why removed |
|---|---|---|
| `golem-mortality` | 10,468 | Death concepts removed by design decision. Vitality/phase concepts absorbed into roko-core |
| `golem-sonification` | 5,641 | Modular synthesis / audio output not relevant to agent orchestration |
| `golem-creature` | 11 | Visual creature evolution forms not relevant |
| `golem-engagement` | 11 | Achievement / graveyard / gamification not relevant |
| `golem-oneirography` | 11 | Dream journal / NFT minting not relevant |
| `golem-ta` | 11 | Technical analysis (TDA, Betti curves) shell -- DeFi-specific, not general |
| `golem-uniswap` | 2,118 | Uniswap V3/V4 specific tools |
| `golem-coordination` | 11 | Pheromone field / clade coordination shell -- replaced by event bus + relay |
| `golem-economy` | 323 | Knowledge marketplace shell -- absorbed into roko-chain |
| `golem-eval` | 2,179 | Mortality thesis validation -- replaced by general-purpose gate system |
| `bardo-styx` (app) | 209 | Styx relay (clade sync, knowledge exchange) -- replaced by agent-relay |
| `bardo-terminal` (app) | 35,995 | Standalone TUI binary -- absorbed into roko-cli's dashboard |
| `bardo-compute` (app) | 30 | Compute node shell |
| `bardo-gateway` (app) | 23,115 | Standalone inference proxy -- functionality absorbed into roko-gateway + roko-agent |
| `mori-service` (app) | 7,742 | Standalone HTTP service -- absorbed into roko-serve |
| `bardo-inference` | 413 | Wire types -- absorbed into roko-agent |
| `mpp` | 988 | Payment protocol -- absorbed into roko-serve |

---

## 6. Dependency Graph Comparison

### Bardo/Mori dependency layers (from workspace Cargo.toml)

```
Layer 0:  bardo-primitives, bardo-inference
Layer 0:  golem-core
Layer 1:  golem-runtime -> golem-core, golem-daimon, golem-grimoire, golem-inference, golem-chain, golem-mortality
Layer 2:  golem-heartbeat, golem-grimoire, golem-daimon, golem-mortality, golem-economy, golem-dreams, golem-context
Layer 3:  golem-safety -> golem-core
Layer 4:  golem-inference, golem-chain, golem-identity, golem-chain-intelligence, golem-triage, golem-ta, golem-uniswap, golem-oneirography, golem-tools
Layer 5:  golem-coordination
Layer 6:  golem-surfaces, golem-creature, golem-engagement, golem-sonification
Layer 6.5: golem-api
Layer 7:  golem-binary

Mori (separate stack):
  mori-index -> bardo-primitives
  mori-context -> mori-index
  mori-mcp -> mori-index, mori-context
  mori (app) -> mori-index, bardo-gateway
  mori-service -> bardo-gateway, bardo-primitives
```

Key observation: In bardo, there are **two parallel stacks**:
1. The golem agent stack (golem-core -> golem-runtime -> golem-heartbeat -> ... -> golem-binary)
2. The mori orchestrator stack (mori-index -> mori-context -> mori-mcp, plus the mori app)

The mori app is a monolith at 108K LOC that depends on bardo-gateway but
does NOT depend on any golem-* crate (it uses its own agent spawning,
plan execution, and state management).

### Roko dependency layers

```
Layer 0:  roko-primitives (no deps)
Layer 0/1: roko-core -> roko-primitives
Layer 1:  roko-runtime -> roko-core, roko-primitives

Layer 2 (subsystems, all depend on roko-core):
  roko-fs -> roko-core
  roko-daimon -> roko-core
  roko-chain -> roko-core
  roko-graph -> roko-core
  roko-lang-{rust,typescript,go} -> roko-core
  roko-index -> roko-core, roko-primitives, roko-lang-*
  roko-std -> roko-core [optional: roko-chain]
  roko-plugin -> roko-core
  roko-learn -> roko-core, roko-agent, roko-daimon, roko-fs, roko-primitives
  roko-neuro -> roko-core, roko-fs, roko-agent, roko-learn
  roko-dreams -> roko-core, roko-neuro, roko-learn, roko-agent
  roko-compose -> roko-agent, roko-core, roko-daimon, roko-dreams, roko-learn, roko-neuro

Layer 3:
  roko-agent -> roko-core, roko-fs, roko-graph, roko-std
  roko-gate -> roko-core, roko-agent
  roko-gateway -> roko-core, roko-agent, roko-learn
  roko-conductor -> roko-core, roko-learn

Layer 4 (application surfaces):
  roko-agent-server -> roko-agent, roko-chain, roko-core, roko-learn, roko-neuro
  roko-acp -> roko-core, roko-runtime, roko-agent, roko-gate, roko-compose, roko-serve, roko-learn, roko-dreams, roko-neuro, roko-std
  roko-serve -> (most crates)
  roko-cli -> (most crates)

MCP (independent stack):
  roko-mcp-stdio (standalone)
  roko-mcp-code -> roko-core, roko-index, roko-mcp-stdio
  roko-mcp-github (standalone HTTP)
  roko-mcp-slack -> roko-mcp-stdio
  roko-mcp-scripts -> roko-mcp-stdio
```

Key observation: Roko has a **single unified stack**. There is no separate
orchestrator vs agent division. roko-cli is the top-level binary that wires
everything together, and roko-serve provides the HTTP surface. The golem
agent runtime and mori orchestrator are fused into one coherent architecture.

---

## 7. Public API Boundaries

### Bardo/Mori API surface

- **golem-core**: GolemId, CorticalState, EventFabric, Extension trait, config types
- **golem-runtime**: CognitiveEngine, StateManager, GolemState, PredictionStore, RuntimeLoop
- **golem-grimoire**: GrimoireReader/GrimoireWriter traits, GrimoireStore
- **golem-safety**: Capability, AllowList, AuditChain, PermitState
- **golem-inference**: InferenceClient trait, TierRouter, request/response types
- **golem-chain**: ChainProvider, Erc8004Registry, RevmSimulator, Warden
- **golem-tools**: ToolExecutor, ToolRegistry, ToolDef, PolicyCage
- **golem-api**: Axum router, WebSocket handler, SSE handler
- **mori-index**: CodeIndex (tree-sitter + PageRank + search)
- **mori app**: CLI subcommands (implicit, not a library)

### Roko API surface

- **roko-core**: Signal/Engram type, 12 kernel traits (Substrate, Score, Verify, Route, Compose, React, Bus, Observe, Connect, Trigger, Store, ColdStore), config, safety contracts, taint tracking, trust lattice
- **roko-agent**: ProviderKind enum (11 kinds), ProviderPool, Agent trait, tool loop, MCP client/resolver, safety layer
- **roko-gate**: Gate trait, 19 concrete gate implementations, 7-rung pipeline, adaptive thresholds
- **roko-graph**: CellRegistry, GraphDef, DAG execution with parallel waves, immune graph
- **roko-learn**: EpisodeLogger, CascadeRouter, Experiment, PlaybookStore, EfficiencyTracker
- **roko-neuro**: KnowledgeStore, DistillationEngine, TierProgression
- **roko-compose**: SystemPromptBuilder (9-layer), RoleSystemPromptSpec, AttentionBidder
- **roko-serve**: ~317 Axum routes, SSE, WebSocket, triggers, feeds, telemetry lens
- **roko-cli**: `roko` binary with all subcommands (plan, prd, run, status, doctor, dashboard, agent, research, knowledge, learn, config, serve, ...)

---

## 8. Architectural Philosophy Differences

### Bardo: Agent-centric, golem lifecycle

The bardo architecture is organized around the **lifecycle of an autonomous
DeFi agent** (the "golem"). The layering goes from identity/state through
cognition, inference, chain interaction, and surfaces. The mori orchestrator
is a separate bolt-on system that manages plan execution but does not deeply
integrate with the golem lifecycle.

- Primary noun: **GolemId** (agent identity)
- Primary loop: **Heartbeat tick** (observe -> retrieve -> analyze -> gate -> simulate -> validate -> execute -> verify -> reflect)
- Subsystem organization: by **cognitive function** (grimoire=memory, daimon=affect, mortality=lifecycle, dreams=consolidation)
- Safety model: **Capability tokens** with compile-time enforcement
- Knowledge: **LanceDB + SQLite** episodic/semantic stores
- Communication: **EventFabric** broadcast channel + pheromone fields

### Roko: Signal-centric, self-developing toolkit

The roko architecture is organized around the **Signal** as the universal
protocol noun, with 12 kernel traits that any subsystem can implement. The
primary loop is plan execution (prompt -> compose -> agent -> gate -> persist),
not agent heartbeat ticks. Everything is designed for self-development.

- Primary noun: **Signal** (content + metadata, backed by Engram struct)
- Primary loop: **query -> score -> route -> compose -> act -> verify -> write -> react**
- Subsystem organization: by **architectural function** (agent=dispatch, gate=validation, compose=prompts, learn=adaptation, neuro=knowledge)
- Safety model: **Trust-origin lattice** + taint tracking + immune graph + five-head corrigibility
- Knowledge: **JSONL-backed** durable stores with tier progression
- Communication: **Event bus** + relay + SSE/WebSocket

### Key structural differences

1. **No more golem-runtime cognitive engine**: bardo's CognitiveEngine/MindState/AttentionSalience
   are replaced by roko's ProcessSupervisor and event bus. Roko agents are spawned processes,
   not cognitive state machines.

2. **Learning is first-class**: bardo had no learning subsystem. roko-learn (66K LOC) is the
   second-largest crate, with episode logging, cascade routing, experiments, and playbooks.

3. **Gate system replaces eval**: bardo had a single golem-eval crate for thesis validation.
   Roko has a 19-gate 7-rung pipeline that validates every task execution.

4. **Graph execution**: roko-graph provides DAG-based cell orchestration that bardo lacked
   entirely. This enables complex multi-step agent workflows.

5. **Provider plurality**: bardo used one LLM provider (via bardo-gateway). Roko supports
   11 different provider kinds with health-aware routing and automatic failover.

6. **MCP ecosystem**: bardo had one MCP server (mori-mcp). Roko has 5 specialized MCP
   servers plus retained HTTP MCP clients at runtime.

7. **Mori absorbed, not separate**: The biggest structural change is that the mori orchestrator
   (108K LOC standalone app) is no longer a separate system. Its functionality is distributed
   across roko-cli (runner-v2, TUI, git integration), roko-serve (HTTP API), and the various
   subsystem crates.

---

## 9. Size Evolution by Functional Area

| Functional area | Bardo LOC | Roko LOC | Growth |
|---|---|---|---|
| Core types/kernel | 9,025 (golem-core) | 75,122 (roko-core) | 8.3x |
| Runtime/process | 7,847 (golem-runtime) | 33,505 (roko-runtime) | 4.3x |
| Knowledge/memory | 15,019 (golem-grimoire) | 20,555 (roko-neuro) | 1.4x |
| Affect/daimon | 9,283 (golem-daimon) | 8,364 (roko-daimon) | 0.9x |
| Dreams | 2,844 (golem-dreams) | 14,286 (roko-dreams) | 5.0x |
| Safety | 4,869 (golem-safety) | _(in roko-core + roko-agent)_ | distributed |
| Inference/gateway | 25,464 (golem-inference + bardo-gateway) | 109,238 (roko-agent + roko-gateway) | 4.3x |
| Chain | 23,069 (golem-chain + golem-identity + golem-chain-intelligence) | 29,296 (roko-chain) | 1.3x |
| Tools | 10,234 (golem-tools) | 10,951 (roko-std) | 1.1x |
| Surfaces/API/HTTP | 7,357 (golem-api + golem-surfaces + mori-service) | 104,187 (roko-serve + roko-agent-server) | 14.2x |
| TUI/terminal | 35,995 (bardo-terminal) | _(in roko-cli)_ | absorbed |
| Main binary | 111,532 (golem-binary + mori) | 234,797 (roko-cli) | 2.1x |
| Code intelligence | 9,638 (mori-index + mori-context + mori-mcp) | 10,402 (roko-index + roko-mcp-* + roko-lang-*) | 1.1x |
| Learning | 0 | 65,725 (roko-learn) | **new** |
| Gates | 2,179 (golem-eval) | 23,263 (roko-gate) | 10.7x |
| Graph execution | 0 | 12,979 (roko-graph) | **new** |
| Prompt composition | 3,995 (golem-context) | 29,022 (roko-compose) | 7.3x |
| ACP | 0 | 21,579 (roko-acp) | **new** |
| Conductor | 0 | 11,062 (roko-conductor) | **new** |
| Plugins | 0 | 5,792 (roko-plugin) | **new** |
| EVM simulator | 14,129 (mirage-rs) | 38,381 (mirage-rs) | 2.7x |

---

## 10. Summary

The roko crate architecture represents a deliberate re-architecture rather than
a port. The key structural decisions were:

1. **Merge the two stacks**: bardo's golem agent runtime and mori orchestrator
   are unified into one coherent system. There is no more agent-vs-orchestrator split.

2. **Signal as universal noun**: Instead of golem-core's GolemId/EventFabric/Extension
   model, roko uses Signal/Engram with 12 kernel traits. This is more flexible and
   composable.

3. **Remove domain-specific crates**: DeFi-specific (golem-uniswap, golem-tools),
   mortality (golem-mortality), presentation (golem-creature, golem-sonification,
   golem-engagement, golem-oneirography) crates are removed. Roko is domain-agnostic.

4. **Add cybernetic subsystems**: Learning (roko-learn), graph execution (roko-graph),
   conductor (roko-conductor), plugins (roko-plugin), and ACP (roko-acp) are
   entirely new systems that make roko self-developing.

5. **Extract services**: Instead of bardo's monolithic mori app (108K LOC), roko
   distributes functionality across purpose-built crates (roko-cli, roko-serve,
   roko-agent-server, roko-acp) with cleaner boundaries.

The result is 2.65x larger (337K -> 893K LOC) but architecturally cleaner:
35 workspace members vs 41, with no empty shells and clearer layering.
