# Daeji Integration -- Master Index

This document is the entry point for all documentation about integrating roko (a
self-developing AI agent toolkit) with daeji (a custom blockchain built from composable
cryptographic and consensus primitives). It serves as both a complete introduction and a
reference manual. Every concept mentioned anywhere in the 12 documents in this series is
defined here so a reader can reference back to this file at any time.

**No prior knowledge of roko, daeji, commonware, blockchain, or AI agents is assumed.**

---

## The Three Projects

### Roko -- Self-Developing AI Agent Toolkit

Roko is a Rust codebase (~177,000 lines of code across 18 crates) that orchestrates AI
coding assistants to develop software autonomously -- including developing itself. The
codebase lives at `/Users/will/dev/nunchi/roko/roko/`. The repository is at
`github.com/Nunchi-trade/roko`.

**The core mission:** Roko reads Product Requirements Documents (PRDs), generates
implementation plans, executes tasks via Claude-based AI agents, validates results with
gates (compile, test, lint, review), learns from outcomes, and persists results. The
entire cycle runs autonomously and continuously. Roko uses itself to develop itself -- its
own backlog of work items lives in its `.roko/` data directory and is executed by
`roko plan run`.

**What it does, step by step:**

1. A human writes a PRD describing a feature or fix.
2. Roko reads the PRD and generates an implementation plan -- a directed acyclic graph
   (DAG) of concrete tasks, stored as a TOML file (`tasks.toml`).
3. Roko dispatches AI agents (Claude CLI, Claude API, Ollama, Gemini, Perplexity,
   OpenAI-compatible backends) to execute each task. An agent receives a prompt assembled
   by the 9-layer SystemPromptBuilder, writes code using tools (shell, file edit, search),
   and returns results. The CascadeRouter selects which LLM model to use for each task
   based on historical performance.
4. After each task, roko runs a gate pipeline -- a sequence of validation checks called
   "rungs" -- to verify the agent's output is correct.
5. Results are recorded as episodes (structured JSON logs of every agent turn, tool call,
   and gate result) and persisted to disk.
6. Roko learns from outcomes: it adjusts model routing via the CascadeRouter, tunes gate
   thresholds via adaptive EMA, stores reusable patterns in playbooks, and archives
   durable knowledge in the neuro store.
7. If a gate fails, roko can automatically generate a revised plan and retry via the
   gate-failure replan mechanism.

**The self-hosting workflow (every step is a real CLI command):**

```bash
roko prd idea "Wire X into Y"          # 1. Capture a work item
roko prd draft new "slug-name"          # 2. Draft a PRD (agent-driven)
roko research enhance-prd slug-name     # 3. Research for context
roko prd plan slug-name                 # 4. Generate plan + tasks
roko plan run plans/                    # 5. Execute (agents -> gates -> persist)
roko plan run plans/ --resume ...       # 6. Resume if interrupted
roko dashboard                          # 7. Watch progress (ratatui TUI)
roko serve                              # 8. HTTP control plane (~85 routes)
```

#### Architecture: 1 Noun + 6 Verbs

Roko's architecture is built on a single universal data type and six verb traits that
operate on it:

**The Noun: Engram (also called Signal)**

Every event, data point, agent output, and gate verdict in roko is an Engram. Engrams are:

- **Content-addressed** -- identified by a BLAKE3 cryptographic hash of their contents
- **Decaying** -- weight fades over time according to configurable half-lives
- **Scored** -- rated on multiple dimensions: confidence, novelty, utility, reputation
- **Traced** -- linked into a lineage DAG (directed acyclic graph) so you can trace any
  engram back to its origins
- **Composable** -- can be merged under token budgets for prompt assembly

Optional fields: HDC fingerprint (10,240-bit binary vector for semantic similarity),
emotional tags (from the daimon affect engine), chain attestations (on-chain witness
records).

**The 6 Verb Traits:**

| Trait | What It Does |
|---|---|
| **Store** (also called Substrate) | Persist and query engrams. The `FileSubstrate` writes JSONL to disk. |
| **Score** (also called Scorer) | Rate engrams on multiple dimensions (confidence, novelty, utility, reputation). |
| **Gate** (also called Verify) | Validate engrams against ground truth. The gate pipeline runs compile, test, lint, diff review, and more. |
| **Route** (also called Router) | Select from candidates. The CascadeRouter picks the best LLM model; enrichment steps pick the best context. |
| **Compose** (also called Composer) | Assemble engrams under a token budget. The SystemPromptBuilder assembles 9 layers. VCG auction allocates context. |
| **React** (also called Policy) | Watch the engram stream and emit interventions. Circuit breakers, adaptive thresholds, replan triggers. |

**The Universal Loop:** `query -> score -> route -> compose -> act -> verify -> write -> react`

This loop executes at every level: per agent turn, per task, per plan.

#### The 18 Crates

| Crate | What It Does | Status |
|---|---|---|
| `roko-core` | Engram type, 6 verb traits, config, tools, errors. The kernel of the system. | Stable |
| `roko-primitives` | HDC vectors (10,240-bit binary), tier routing. | Wired |
| `roko-runtime` | ProcessSupervisor (manages agent subprocesses), event bus, cancellation tokens. | Wired |
| `roko-std` | Defaults, 19 builtin tools (shell, file edit, search, etc.), mock dispatcher. | Stable |
| `roko-fs` | FileSubstrate -- JSONL persistence for engrams, GC (garbage collection), directory layout. | Stable |
| `roko-agent` | 5+ LLM backends (Claude CLI, Claude API, Ollama, Gemini, Perplexity, OpenAI-compat), tool loops, MCP passthrough, safety layer (role-based auth, pre/post checks, tool allowlists). | Wired |
| `roko-gate` | 11 concrete gates, 7-rung pipeline, adaptive thresholds via EMA per rung. | Wired |
| `roko-compose` | Prompt assembly via 9-layer SystemPromptBuilder, 9 role templates, 6 enrichment steps, VCG auction for context allocation. | Wired |
| `roko-orchestrator` | Plan DAG parser, parallel executor, merge queue, safety constraints. | Wired |
| `roko-conductor` | 10 watchers, circuit breaker, health diagnosis. | Wired |
| `roko-agent-server` | Per-agent HTTP sidecar: 13 routes (/message, /stream WS, /predictions, /research, /tasks). | Wired |
| `roko-learn` | Episodes (JSONL turn logs), playbooks (reusable patterns), skills, CascadeRouter (model routing), efficiency events, prompt experiments (A/B), cost tracking, error patterns. | Wired |
| `roko-neuro` | Durable knowledge store with 6 knowledge kinds, 4 tiers, distillation, HDC similarity search. Queried at dispatch time and injected into system prompts. | Wired |
| `roko-dreams` | Offline consolidation: hypnagogia (recent memory), imagination (synthesis), cycle (full consolidation). | Partial |
| `roko-daimon` | Affect engine, somatic markers, dispatch modulation. Loaded per-task in orchestrate.rs. | Wired |
| `roko-index` | Parser, graph, HDC indexing for code intelligence. | Built |
| `roko-mcp-code` | Code-intelligence MCP server. | Wired |
| `roko-chain` | Chain witness primitives, AlloyChainClient, ChainWitnessEngine. Built but not yet wired into runtime. | Phase 2+ |
| `roko-cli` | Main CLI binary: all subcommands listed above, ratatui TUI with F1-F7 tabs. | Wired |
| `roko-serve` | HTTP control plane: ~85 REST routes + SSE + WebSocket on port 6677. | Wired |

#### How Agents Work

- **Backends:** Claude CLI (invokes the `claude` command), Claude API (direct Anthropic
  HTTP), Ollama (local inference), OpenAI-compat (OpenRouter, etc.), Perplexity (web
  search). Each backend implements the same dispatch trait.
- **Tool loop:** Agent receives prompt + tool schema. Calls tools (shell commands, file
  edits, code search). Results feed back as context. Iterates until done or max turns.
- **Safety layer:** Role-based authorization, pre-execution checks, post-execution checks,
  per-role tool allowlists. Implemented in `roko-agent/src/safety/`.
- **MCP (Model Context Protocol):** Configured per-agent in `roko.toml`. Supports stdio,
  HTTP, and SSE transports. Allows agents to use external tool servers.

#### How the Gate Pipeline Works (7-Rung Pipeline)

After every agent task, the gate pipeline runs these validation rungs in sequence:

| Rung | What It Checks |
|---|---|
| 1. Compile | `cargo build` -- does the code compile? Always runs. |
| 2. Clippy/Lint | `cargo clippy -- -D warnings` -- any lint violations? |
| 3. Test | `cargo test` -- do the tests pass? |
| 4. Symbol verification | Do expected symbols exist in the compiled output? |
| 5. Generated tests | Run tests that were auto-generated for the changed code. |
| 6. Property tests | Run property-based tests if configured. |
| 7. LLM judge review | An LLM reviews the diff for correctness and style. |

Plus standalone gates: DiffGate (reviews the git diff), CodeExecutionGate (runs the code),
BenchmarkRegressionGate (checks performance), SecurityScanGate (scans for vulnerabilities).

**Adaptive thresholds:** Each rung tracks its historical pass rate via exponential moving
average (EMA). Thresholds adjust over time so that a rung that consistently passes at 95%
does not block on a rare 94% score. Stored in `.roko/learn/gate-thresholds.json`.

A task only passes if ALL rungs pass. If any rung fails, roko can automatically generate a
revised plan via the gate-failure replan mechanism.

#### The 9-Layer System Prompt Builder

Before dispatching an agent, the SystemPromptBuilder (in `roko-compose`) assembles a prompt
from 9 layers:

| Layer | What It Contains |
|---|---|
| 1. Role-specific base | Template for the agent's role (implementer, reviewer, researcher, etc.) |
| 2. Domain constraints | Rules specific to the domain (Rust conventions, project-specific patterns) |
| 3. Tool allowlist | Which tools this agent is permitted to use |
| 4. Prior experience (playbooks) | Reusable patterns from successful past episodes |
| 5. Task context | The specific task description, plan context, dependencies |
| 6. Gate feedback | Results from previous gate runs (if this is a retry) |
| 7. Neuro store guidance | Knowledge entries from the neuro store relevant to this task |
| 8. Daimon somatic markers | Emotional/affect signals from the daimon engine |
| 9. Attention hints (VCG) | Context items selected by VCG auction under token budget |

Layers 4 and 7 inject learned knowledge. Layer 9 uses a VCG (Vickrey-Clarke-Groves)
auction mechanism to allocate limited context window space among competing context bidders
(Neuro, Task, and Research attention bidders).

#### Learning Subsystems

Roko has multiple interconnected learning mechanisms:

- **Episodes** -- JSONL record of every agent turn + gate result. Fields include: task ID,
  model used, tokens consumed, latency, gate verdict, cost, HDC fingerprint. Stored at
  `.roko/episodes.jsonl`.
- **Playbooks** -- Reusable patterns extracted from successful episodes. Injected into
  layer 4 of the system prompt. Stored at `.roko/learn/playbooks.json`.
- **Skills** -- Structured agent capabilities linked to gate results.
- **CascadeRouter** -- Multi-stage model routing: tries a fast/cheap model first, escalates
  to a more capable model based on confidence thresholds. Uses UCB (Upper Confidence
  Bound) bandit algorithm. Learns from gate verdicts. Stored at
  `.roko/learn/cascade-router.json`.
- **Efficiency events** -- Per-turn metrics for C-Factor (a composite score of cost,
  quality, and latency). Stored at `.roko/learn/efficiency.jsonl`.
- **Prompt experiments** -- A/B testing of prompt variants. The ExperimentStore assigns
  tasks to variant groups and tracks which variant produces better gate outcomes. Stored
  at `.roko/learn/experiments.json`.
- **Error patterns** -- Gate-failure signatures for pre-remediation (recognizing failure
  modes before they happen).

#### Knowledge Layer (Neuro Store)

The neuro store (`roko-neuro`) is roko's durable, local knowledge database. It stores
knowledge entries with semantic similarity search and temporal decay.

**6 Knowledge Kinds with Half-Lives:**

| Kind | Description | Default Half-Life |
|---|---|---|
| Insight | Factual observation | 30 days |
| Heuristic | Behavioral rule | 90 days |
| Warning | Urgent condition | 1 hour |
| AntiKnowledge | Something explicitly wrong -- "do NOT do X" | 90 days |
| CausalLink | Cause-effect relationship | 60 days |
| StrategyFragment | Reusable plan or pattern | 14 days |

**4 Knowledge Tiers (weight multipliers):**

| Tier | Weight Multiplier | Description |
|---|---|---|
| Transient | 0.1x | Just observed, unconfirmed |
| Working | 0.5x | Used once successfully |
| Consolidated | 1.0x | Confirmed by multiple successful uses |
| Persistent | 5.0x | Repeatedly validated, high confidence |

Entries are queried at dispatch time using HDC similarity search (compare the task
description's HDC vector against all stored entry vectors via Hamming distance) and
injected into layer 7 of the 9-layer system prompt.

#### Key Data Persistence (all under .roko/)

| Path | What It Stores |
|---|---|
| `signals.jsonl` | Engrams with decay scores |
| `episodes.jsonl` | Agent turns + gate results |
| `state/executor.json` | Plan execution snapshots (for `--resume`) |
| `prd/` | PRD lifecycle documents |
| `learn/cascade-router.json` | Model routing state (CascadeRouter) |
| `learn/gate-thresholds.json` | Adaptive gate thresholds (EMA per rung) |
| `learn/playbooks.json` | Reusable patterns from successful episodes |
| `learn/experiments.json` | Prompt A/B experiment state |
| `learn/efficiency.jsonl` | Per-turn cost/quality/latency metrics |
| `neuro/` | Knowledge store entries |

#### Configuration (roko.toml)

The main configuration file (`roko.toml` in the project root) controls all subsystems:

- **Agent:** `default_model`, `backend`, `temperament`, `context_limit`
- **Gates:** `clippy` enabled/disabled, `tests` enabled/disabled, `max_iterations`
- **Routing:** `algorithm` (linucb for CascadeRouter), model tiers, quality/cost/latency
  weights
- **Learning:** `replan_on_gate_failure`, `dream_on_completion`, `auto_playbook_refresh`
- **Chain:** `rpc_url`, `chain_id`, `wallet_key`, `agent_registry` address

---

### Commonware -- Composable Blockchain Primitives

Commonware is a Rust library of independent, composable blockchain building blocks
created by Patrick O'Grady (formerly of Ava Labs, the company behind the Avalanche
blockchain). It is explicitly an "anti-framework": rather than providing a monolithic
blockchain node that you fork and customize, it provides 17 independent crates -- each
implementing one primitive -- that you compose however you like.

**Key primitives used in this project:**

| Crate | What It Provides |
|---|---|
| `commonware-cryptography` | Ed25519 digital signatures (for P2P node identity), BLS12-381 threshold signatures (for consensus finalization), Verifiable Random Functions (VRF, for bias-resistant randomness) |
| `commonware-p2p` | Two P2P networking implementations sharing the same trait: `authenticated` (real TCP with Ed25519 handshakes, for production) and `simulated` (in-process message bus, for deterministic testing) |
| `commonware-consensus` | Simplex BFT -- a Byzantine fault tolerant consensus protocol |
| `commonware-storage` | QMDB (Quick Merkle Database, for authenticated key-value state storage) and MMR (Merkle Mountain Range, an append-only cryptographic audit log) |
| `commonware-runtime` | Two async runtime implementations sharing the same trait: `tokio` (production, real I/O) and `deterministic` (single-threaded simulator, controllable time, for testing) |
| `commonware-codec` | Wire format for encoding and decoding consensus messages |
| `commonware-broadcast` | Ordered broadcast for multi-sequencer scenarios (DSMR) |
| `commonware-resolver` | Pluggable content-addressed storage for large data |

**Philosophy:** Every crate is independent. You can use the consensus without the P2P.
You can use the P2P without the storage. There is no "commonware node" binary -- you
build your own node by composing the pieces you need.

**Repository:** `github.com/commonwarexyz/monorepo`
**Version used here:** 2026.4.0
**License:** Dual MIT / Apache-2.0

### Daeji (Codename "Kora") -- Minimal EVM Blockchain

Daeji is a concrete blockchain node built from commonware primitives. It is the
specific chain that roko agents will interact with.

**What it is, concretely:**

- A Rust binary (`kora`) that runs as a blockchain validator node
- Uses commonware's simplex BFT consensus to agree on blocks across multiple nodes
- Uses REVM (Rust Ethereum Virtual Machine) to execute EVM-compatible smart contracts
  -- standard Ethereum bytecode and Solidity contracts work without modification
- Uses QMDB to store the blockchain state (account balances, contract storage) in a
  Merkle-authenticated database
- Uses BLS12-381 threshold cryptography: the validator set collectively holds a shared
  signing key, and any 3-of-4 validators can produce a valid block finalization
  signature, but no individual validator (and no outside observer) ever knows the
  complete private key
- Exposes standard Ethereum JSON-RPC (`eth_sendRawTransaction`, `eth_call`,
  `eth_getBalance`, etc.) so any Ethereum tooling works against it: ethers-rs, alloy,
  Foundry, MetaMask, Hardhat, etc.

**Daeji is NOT a fork of go-ethereum or reth.** It is built from scratch using three
components: commonware consensus + REVM execution + QMDB storage.

**Standard local devnet configuration:**

- 4 validator nodes, each holding a BLS12-381 threshold signature share
- 1 secondary (follower) node that replicates blocks but does not vote
- Threshold: 3-of-4 (any 3 validators can finalize a block; 1 can be offline)
- Chain ID: 1337
- Block time: ~400ms
- Gas limit: 30,000,000 per block
- Transaction type: EIP-1559

**Repository:** `github.com/Nunchi-trade/daeji`

---

## The Integration Goal

The purpose of connecting roko to daeji is to give AI agents access to a real
blockchain with unique cryptographic properties. There are four objectives:

### 1. Shared Agent Knowledge

When one roko agent learns something useful while executing a task (e.g., "this API
requires pagination" or "this function has an off-by-one edge case"), it posts a hash
and summary of that knowledge to a smart contract (InsightBoard) on daeji. Other
agents query that contract before starting their own tasks and inject relevant prior
knowledge into their system prompt via layer 7 of the SystemPromptBuilder. This creates
a shared, tamper-evident knowledge layer across all agents -- extending the local neuro
store into a cross-agent knowledge network.

### 2. Tamper-Evident Work Products

After each task passes roko's gate pipeline, roko posts a cryptographic hash of the
episode log (the full structured record of every agent turn, tool call, and
validation result) to a witness contract on daeji. Anyone can later verify that a
given episode log has not been altered by checking its hash against the on-chain
record. This provides an immutable audit trail of all agent work.

### 3. Novel Cryptographic Features

Because daeji uses commonware (not a standard Ethereum stack), it provides
capabilities that are not available on typical EVM chains:

- **Threshold VRF (Verifiable Random Function):** Every block produces a
  bias-resistant random number derived from the BLS12-381 threshold signature. No
  single validator can predict or manipulate it. Useful for fair task assignment,
  verifiable model routing in the CascadeRouter, and unbiased A/B experiment
  assignment in the ExperimentStore.
- **BTLE (Binding Timelock Encryption):** An agent can encrypt a commitment that can
  only be decrypted at a specific future block height. The decryption key is derived
  from the threshold signature at that block. Useful for sealed-bid auctions, commit-
  reveal schemes, and time-delayed reveals.
- **Compact Finality Certificates:** A single BLS12-381 threshold signature (96 bytes)
  plus a block header (144 bytes) = 240 bytes total, verifiable with only the 48-byte
  group public key. Useful for lightweight cross-chain proofs.
- **Deterministic Simulation:** The entire consensus network can be run in-process
  with controllable time and deterministic message delivery (via commonware's
  simulated P2P and deterministic runtime). This allows roko's gate pipeline to spin
  up a complete blockchain (via the daeji `TestHarness`), replay agent interactions,
  and verify outcomes -- all within a single test process. This powers the proposed
  ChainSimulate gate rung.

### 4. Autonomous Knowledge Economy (Future)

Agents pay for high-quality knowledge entries, bid for task bounties, and build
on-chain reputations, all mediated by smart contracts on daeji. This is the long-term
vision; the immediate work focuses on objectives 1-3.

---

## The Two Chains: Mirage-rs and Daeji

Roko already uses a chain-adjacent component called mirage-rs. The two chains serve
entirely different purposes and coexist:

**Mirage-rs** (chain ID 88888) is an in-process EVM fork simulator. It forks the state
of a live Ethereum mainnet (or testnet) at a given block number, runs transactions
against that forked state in a copy-on-write manner, and discards or keeps changes as
needed. It has no validators, no P2P network, and no consensus -- blocks are mined
instantly by the single process. Its purpose is DeFi simulation: "what would happen if
I executed this swap against the current Ethereum mainnet state?" and testing Solidity
contracts against real production data without spending real gas.

**Daeji** (chain ID 1337) is a real consensus chain with multiple validator nodes. Its
purpose is the agent knowledge ledger and witness anchoring described above.

| Concern | Mirage-rs | Daeji |
|---|---|---|
| Chain type | EVM fork simulator (single process) | Real consensus chain (multi-node) |
| Consensus | None (instant auto-mining) | Simplex BFT with threshold signatures |
| Chain ID | 88888 | 1337 |
| Primary use | Fork mainnet state, simulate DeFi | Agent knowledge ledger, witness anchoring |
| Block time | Configurable, 50ms-1s | ~400ms (consensus-dependent) |
| State origin | Copy-on-write over a forked upstream chain | Fresh genesis, QMDB Merkle tree |
| Deployment | Single container (e.g., Railway) | Docker Compose (4+ containers) or native |
| Unique capability | Access to real Ethereum mainnet state | Real finality, BLS VRF, BTLE, compact certs |

Roko agents can talk to both chains simultaneously. The roko configuration file
(`roko.toml`) distinguishes them:

```toml
[chain.mirage]
rpc_url = "http://localhost:8545"
chain_id = 88888
purpose = "fork_simulation"

[chain.daeji]
rpc_url = "http://localhost:8550"   # shifted to avoid port collision with mirage on 8545
chain_id = 1337
purpose = "agent_ledger"
agent_key = "${DAEJI_AGENT_KEY}"    # env var: secp256k1 private key for signing txs
```

### Port Allocation

All services are designed to coexist on a single development machine without port
conflicts:

| Service | Port | Notes |
|---|---|---|
| mirage-rs RPC | 8545 | Default Ethereum RPC port, unchanged |
| mirage-rs agent-relay WebSocket | 9011 | Mirage sidecar for live event streaming |
| daeji node0 JSON-RPC | 8550 | Primary RPC endpoint; shifted to avoid collision |
| daeji node1 JSON-RPC | 8551 | |
| daeji node2 JSON-RPC | 8552 | |
| daeji node3 JSON-RPC | 8553 | |
| daeji secondary JSON-RPC | 8554 | Read-only follower node |
| daeji P2P overlay (validators) | 30400-30403 | Commonware authenticated P2P |
| daeji P2P (secondary) | 30500 | Secondary follower P2P port |
| Prometheus metrics (per validator) | 9000-9003 | Optional observability |
| Prometheus UI | 9090 | Optional (enabled via compose profile) |
| Grafana dashboard | 3000 | Optional; default login: admin/admin |
| Roko HTTP control plane | 6677 | Roko's own REST API (~85 routes), unrelated to chains |

### Key Identity

A roko agent holds two separate cryptographic keys:

- **Ed25519 key** -- the commonware P2P identity key. Used for authenticating to the
  daeji P2P network. Only relevant if the agent runs as a secondary peer (see
  `02-roko-integration.md`).
- **secp256k1 key** -- the Ethereum transaction signing key. Used for signing
  transactions on both mirage-rs and daeji. This is a standard Ethereum private key.

---

## Integration Phases

- **Phase 1** -- Wire `AlloyChainClient` into roko's orchestrator so it is called
  during normal plan execution. Deploy existing Solidity contracts to daeji. After
  each task passes gates, post a hash of the episode log to the chain witness
  contract. This gives roko tamper-evident audit trails with minimal code changes.

- **Phase 2** -- Implement the InsightBoard knowledge layer. After tasks complete,
  agents post knowledge entries (hash + metadata) to the InsightBoard contract on
  daeji. Before tasks start, agents query the InsightBoard for relevant prior
  knowledge and inject it into their system prompt via layer 7 of the
  SystemPromptBuilder. Sync roko's local neuro store with the on-chain index via the
  NeuroChainSync push/pull loop.

- **Phase 3** -- Modify the daeji source code itself: fix `block.timestamp` (currently
  set to block height instead of wall-clock time), fix `BLOCKHASH` opcode (currently
  returns zero for all inputs), add custom `kora_` RPC methods (`kora_vrfSeed` for the
  current VRF output, `kora_activeAgents` for registered agents, `kora_recentKnowledge`
  for recent knowledge entries), add WebSocket subscription support via jsonrpsee, and
  implement custom EVM precompiles (HDC similarity search at 0x09, QMDB state proofs at
  0x0B, BTLE encryption at 0x0C).

---

## Roko Components That Integrate with Daeji

This section summarizes which roko subsystems are affected by the chain integration and
what changes each requires. Full details are in `02-roko-integration.md` and
`09-native-design.md`.

### orchestrate.rs -- Main Execution Loop

The central integration point. Located at `crates/roko-cli/src/orchestrate.rs`. Changes:
initialization of `AlloyChainClient` and `AlloyChainWallet` from config, pre-task
InsightBoard query for cross-agent knowledge, post-task witness anchoring via
`ChainWitnessEngine`, post-task knowledge promotion to InsightBoard, agent registration
and periodic heartbeats to AgentRegistry.

### roko-chain -- Blockchain Client

Contains `AlloyChainClient` (read-only RPC), `AlloyChainWallet` (sign + submit
transactions), and `ChainWitnessEngine` (episode hash anchoring). Currently built but not
wired into the runtime. Phase 1 wires it into orchestrate.rs.

### roko-gate -- Gate Pipeline

Gains two new rungs with daeji: `ChainWitness` (anchor episode hash, confirm tx mined)
and `ChainSimulate` (run chain interactions against an in-process simulated daeji network
using commonware's deterministic runtime before committing to the real chain).

### roko-neuro -- Knowledge Store

Gains bidirectional sync with the on-chain InsightBoard via `NeuroChainSync`: push
high-confidence local entries to chain (so other agents discover them), pull new chain
entries from other agents into local store (so this agent benefits from collective
knowledge).

### roko-learn -- CascadeRouter and ExperimentStore

CascadeRouter gains verifiable model routing using daeji's VRF output (`prevrandao`) as
the seed for weighted random model selection. ExperimentStore gains verifiable A/B
experiment variant assignment using the same VRF seed.

### roko-runtime -- ProcessSupervisor

Gains management of the daeji secondary peer subprocess (start before first task, stop
after last task, health monitoring). Also runs the periodic AgentRegistry heartbeat loop.

### roko-learn -- EpisodeLogger

Episode records gain a `chain_attestation` field (`{chain_id, tx_hash, block_number}`)
after witness anchoring, making them tamper-evident.

---

## Document Index

### Setup and Architecture

| File | Description |
|---|---|
| [`01-local-infra.md`](01-local-infra.md) | Complete guide to running a daeji devnet locally. Covers what daeji is, what a DKG ceremony is (both trusted-dealer and interactive Joint-Feldman modes), Docker Compose and native cargo setup paths, the e2e TestHarness (in-process deterministic testing using commonware's simulated P2P and deterministic runtime), node configuration reference (config.toml, genesis.json, peers.json), load testing with the `loadgen` binary, and programmatic devnet bootstrap from roko's ProcessSupervisor. |
| [`02-roko-integration.md`](02-roko-integration.md) | How roko agents connect to daeji in three progressively deeper tiers: (1) JSON-RPC client -- agents send transactions via standard Ethereum RPC using AlloyChainClient and AlloyChainWallet, (2) secondary follower peer -- a roko process joins the daeji P2P overlay as a non-voting node (managed by ProcessSupervisor) and receives blocks in real time via LedgerEvent stream instead of polling, (3) direct commonware crate dependencies -- roko imports commonware crates as Rust library dependencies for in-process access to Ed25519 agent identity, QMDB authenticated episode storage, deterministic runtime for agent tests, and authenticated P2P for direct agent-to-agent messaging. Includes architecture diagram and detailed code paths for witness anchoring, pre-task knowledge query, and post-task knowledge commit. |
| [`06-coexistence.md`](06-coexistence.md) | Detailed guide for running daeji alongside mirage-rs on the same machine. Covers port allocation, configuration patterns in roko.toml, when to use which chain, interaction patterns (simulate on mirage-rs then commit to daeji, shared agent identity across both chains, daeji as witness for mirage-rs simulations, gate pipeline using both chains with SimulationGate on mirage-rs and ChainWitnessGate on daeji), and a migration analysis explaining why both chains coexist indefinitely. |

### Design and Vision

| File | Description |
|---|---|
| [`03-agent-chain-mapping.md`](03-agent-chain-mapping.md) | Maps the original agent-chain architecture vision (27 design documents written before daeji existed) to what daeji concretely provides today. Traces the complete naming evolution: Golem -> roko agent, Grimoire -> neuro store, Clade -> fleet, GNOS -> DAEJI token. Maps every original concept: InsightEntry types, Superposition Memory (now served by QMDB state root), HDC precompile, Predictive Foraging, stigmergy (pheromone coordination), the 5-stage context assembly pipeline (now the SystemPromptBuilder). Identifies what is directly supported, what requires adaptation, and what is deliberately deferred (Block-STM, sentinel agents, OaaS, x402 micropayments). Includes a phased build plan with 4 phases and 19 numbered items. |
| [`04-novel-features.md`](04-novel-features.md) | Eight innovations uniquely enabled by daeji's use of commonware rather than a standard Ethereum stack: (1) BTLE sealed commitments using IBE (Identity-Based Encryption) from BLS12-381 pairings, (2) on-chain threshold VRF for bias-resistant randomness usable by CascadeRouter and ExperimentStore, (3) 240-byte cross-chain finality certificates verifiable with only a 48-byte group public key, (4) deterministic simulation for testing via commonware's deterministic runtime and simulated P2P (powers the ChainSimulate gate rung), (5) key resharing (rotating the validator set without changing the group public key), (6) QMDB historical state proofs (prove any value at any finalized block), (7) agent P2P mesh via commonware's authenticated overlay for direct agent-to-agent encrypted messaging, (8) ordered broadcast (DSMR) for fair message sequencing where agents could act as sequencers. |
| [`05-precompiles.md`](05-precompiles.md) | Design for three custom EVM precompiles -- native Rust functions at reserved Ethereum addresses that execute inside the REVM interpreter: HDC similarity search at address 0x09 (brute-force Hamming distance over 10,240-bit vectors, ~170 microseconds for 100K entries vs billions of gas in Solidity, fixed 50,000 gas cost), QMDB state proofs at address 0x0B (Merkle inclusion/exclusion proofs against any finalized block's state root, 30,000 gas), and BTLE encryption at address 0x0C (BLS12-381 pairing-based IBE encrypt/decrypt targeting future view numbers, 80,000 gas). Each section explains why the operation cannot be done in Solidity, provides the ABI interface, implementation approach with Rust code, dependencies, and a contract-only alternative for phased deployment. |

### Gap Analysis and Implementation

| File | Description |
|---|---|
| [`08-what-breaks.md`](08-what-breaks.md) | Twelve assumptions from the original agent-chain design that do not hold with the actual daeji codebase. Each gap includes: current behavior, why it breaks, impact on integration, fix options (ranked), and a recommendation. Critical: `block.timestamp` is set to block height not wall-clock Unix time (breaks all Solidity timing logic including IdentityRegistry, ReputationRegistry, InsightBoard decay), `BLOCKHASH` opcode returns zero for all inputs. High: no custom precompile registration (only standard 0x01-0x09), no WebSocket subscriptions (`eth_subscribe`). Medium: QMDB uses transition hashes not Merkle Patricia Trie roots (no `eth_getProof`), HDC vector storage costs ~770,000 gas per entry. Low: free gas (no EIP-1559 base fee), coinbase always zero, hardcoded 4-validator set, token identity confusion. Severity ranking table at the end. |
| [`09-native-design.md`](09-native-design.md) | Step-by-step implementation guide for wiring roko to daeji in three phases. Phase 1: fix daeji timestamps (one-line change), fix BLOCKHASH (ring buffer), deploy existing Solidity contracts (AgentRegistry, InsightBoard, MockERC20) via Foundry, instantiate AlloyChainClient in orchestrate.rs, wire ChainWitnessEngine for episode hash anchoring, wire AgentRegistry heartbeats via ProcessSupervisor. Phase 2: InsightBoard knowledge layer with post-task knowledge commit (entries with confidence >= 0.70 and 3+ local confirmations get promoted), pre-task knowledge query (merge local neuro store HDC search with on-chain `eth_getLogs` scan), confirmation flow (auto-confirm entries that helped a task succeed), NeuroChainSync bidirectional push/pull loop. Phase 3: custom kora_ RPC methods via jsonrpsee, HDC search precompile at 0x09, CascadeRouter VRF integration, ExperimentStore VRF integration. Includes function signatures, crate dependency changes, file layout, and Rust code for every component. |
| [`10-daeji-changes.md`](10-daeji-changes.md) | Required source code changes to the daeji repository (`/Users/will/dev/nunchi/daeji/`). Priority 1 (Phase 1): fix `block.timestamp` to use `SystemTime::now()` instead of block height (one-line change in `app.rs`), add `BlockHashCache` ring buffer for the `BLOCKHASH` opcode (new struct in `revm.rs`). Priority 2 (Phase 2): create `KoraPrecompiles` registry that extends standard Ethereum precompiles with custom agent precompiles, add WebSocket subscription support via jsonrpsee `#[subscription]` attribute for `newHeads` and `logs`, extend kora_ RPC namespace with `kora_vrfSeed`, `kora_recentBlocks`, `kora_consensusHealth`, set non-zero `beneficiary` (coinbase) by deriving Ethereum address from validator Ed25519 key. Priority 3 (Phase 3): variable validator set size, real EIP-1559 base fee, EIP-4844 blob sidecar support. Each change includes: current behavior, why it is problematic, conceptual fix, code change with Rust snippets, risk assessment, and testing approach. Also lists changes that should NOT be made (Block-STM, extended block headers, validator slashing, dynamic validator set, GNOS minting in block rewards). |
| [`11-knowledge-layer-redesign.md`](11-knowledge-layer-redesign.md) | Comprehensive hybrid on-chain/off-chain knowledge architecture. Content hashes and metadata (32 bytes + entry type + half-life + poster + pheromone count) are stored on-chain in the InsightBoard contract (small, tamper-evident, ~71 bytes per entry). Full content (the actual knowledge text, potentially 100-2000 bytes) is emitted in Ethereum event logs (cheap to write, readable via `eth_getLogs`, not in mutable state). HDC vectors (1,280 bytes each) are computed locally by each agent from the entry content -- never stored on-chain. Defines: the bidirectional sync protocol (NeuroChainSync: push entries with confidence >= 0.70 and 3+ local confirmations to chain, pull entries from other agents into local neuro store at initial confidence 0.5), the 5-stage context assembly pipeline (query -> filter -> rank -> compress -> arrange with U-shaped attention positioning), the confirmation flow (pheromone counter incremented on-chain when an entry proves useful to another agent), predictive foraging (falsifiable predictions registered before task execution, outcomes measured after gates, residuals calibrate future knowledge retrieval), and the complete entry lifecycle from local discovery through chain promotion to cross-agent confirmation. |

### Open Items

| File | Description |
|---|---|
| [`07-open-questions.md`](07-open-questions.md) | Ten unresolved design decisions: (1) token economics (GNOS demurrage vs plain ETH, minting policy, lazy vs eager decay), (2) knowledge entry storage (inline vs content-hash-only, HDC vectors on-chain vs off-chain, pruning strategy), (3) agent identity model (single Ed25519 key with derived secp256k1 vs separate keys vs secp256k1-only), (4) secondary peer vs RPC client (when to run a full secondary peer vs simple HTTP polling), (5) precompile vs contract boundary (at what entry count does HDC search need a precompile), (6) devnet vs testnet vs production (DKG ceremony mode, state persistence, validator trust model), (7) chain modifications to daeji (fork approach vs clean separation, which changes to make), (8) cross-chain certificate usage (client-side vs on-chain verification, certificate relay, target chains), (9) relationship to roko-chain crate (generic trait vs daeji-specific, witness anchoring format), (10) commonware version tracking (must roko and daeji use the same commonware version, monorepo vs separate repos). |

---

## Key Facts Reference

### Daeji Chain Specification

| Property | Value |
|---|---|
| Repository | `github.com/Nunchi-trade/daeji` |
| Consensus algorithm | Simplex BFT (from commonware) |
| Signature scheme | BLS12-381 threshold signatures |
| EVM execution engine | REVM (Rust EVM) |
| State database | QMDB (Quick Merkle Database) |
| Chain ID | 1337 |
| Block time | ~400ms |
| Gas limit per block | 30,000,000 (configurable) |
| Transaction type | EIP-1559 |
| RPC namespaces | `eth_`, `net_`, `web3_`, `kora_` |
| Toolchain | Rust nightly, `just` task runner |

### Roko's Chain-Related Code (Current State)

| Crate | Location | Status |
|---|---|---|
| `roko-chain` | `crates/roko-chain/` | Contains `AlloyChainClient` (Ethereum RPC client using the alloy library) and `ChainWitnessEngine` (logic for posting episode hashes to chain). Built but not yet wired into the runtime -- the code exists but is never called during normal operation. |
| `roko-neuro` | `crates/roko-neuro/` | Local knowledge store with 6 knowledge kinds, 4 tiers, HDC similarity search, half-life decay. Sync-to-chain (NeuroChainSync push/pull) is not yet implemented -- knowledge stays local only. |
| Solidity contracts | `contracts/` | AgentRegistry, InsightBoard, BountyMarket, IdentityRegistry, ReputationRegistry, and others. Built and deployed against local Anvil/mirage-rs for testing. Not yet deployed to daeji. |

---

## Glossary

Every technical term used in any of the 12 documents is defined here.

### Roko Concepts

**Agent (roko agent)** -- An LLM-powered process that reads task descriptions, writes
code, runs tools, and returns results. Formerly called "golem" in the original
agent-chain documents. Managed by the ProcessSupervisor in `roko-runtime`.

**Alloy** -- The standard Rust library for interacting with Ethereum-compatible chains.
Successor to ethers-rs. Provides JSON-RPC clients, transaction signing, ABI
encoding/decoding, and contract interaction. Used by `AlloyChainClient` and
`AlloyChainWallet` in `roko-chain`.

**Attestation** -- A signed record linking an engram to its verification. After witness
anchoring, an attestation gains a `chain_attestation` field with `{chain_id, tx_hash,
block_number}`.

**C-Factor** -- A composite efficiency metric computed from cost, quality, and latency
per agent turn. Tracked in `.roko/learn/efficiency.jsonl`.

**CascadeRouter** -- The model selection component in `roko-learn`. Uses a multi-stage
approach: tries a fast/cheap model first, escalates to a more capable model based on
confidence thresholds. Learns from gate verdicts using UCB (Upper Confidence Bound)
bandit algorithm. Persisted at `.roko/learn/cascade-router.json`. With daeji, can use
on-chain VRF for verifiable model selection.

**ChainWitnessEngine** -- A component in `roko-chain` that anchors engram hashes
on-chain. Submits a transaction with `b"roko.attestation.witness:" + episode_hash` as
calldata to a sink address. Creates a tamper-evident on-chain record.

**Context Assembly Pipeline** -- A 5-stage process (query -> filter -> rank -> compress
-> arrange) that selects and positions knowledge entries in the system prompt. Stage 5
uses U-shaped attention positioning (most important entries at beginning and end).

**DaimonState** -- The affect engine from `roko-daimon`. Produces somatic markers
(emotional signals) that modulate agent dispatch. Loaded per-task in orchestrate.rs.
Injected into layer 8 of the SystemPromptBuilder.

**Engram (also called Signal)** -- The universal datum in roko. Every event, data
point, agent output, and gate verdict is an Engram. Content-addressed via BLAKE3 hash,
decaying weight, scored on multiple dimensions. See the architecture section above for
full details.

**Enrichment** -- The process of adding context to an agent's prompt before dispatch.
The SystemPromptBuilder performs 6 enrichment steps to assemble the 9 layers.

**Episode** -- A structured JSON record of everything that happened during one agent
task execution: every prompt sent to the AI model, every response received, every tool
call made, every gate result, timestamps, token counts, cost, HDC fingerprint, and the
final outcome (pass/fail). Episodes are roko's primary audit trail. Stored at
`.roko/episodes.jsonl`.

**EpisodeLogger** -- The component in `roko-learn` that records episodes. With daeji
integration, episode records gain a `chain_attestation` field after witness anchoring.

**ExperimentStore** -- Runs A/B tests on system prompt variants. Assigns tasks to
variant groups and tracks which variant produces better gate outcomes. With daeji, uses
VRF for verifiable variant assignment. Stored at `.roko/learn/experiments.json`.

**Fleet** -- A group of agents under one operator. Formerly called "clade" in the
original agent-chain documents.

**Gate pipeline (gate rungs)** -- Roko's validation system. After an agent produces
output for a task, roko runs 7 sequential rungs (compile, lint, test, symbol verify,
generated tests, property tests, LLM review) plus standalone gates (diff, code
execution, benchmark, security). A task only passes if all configured rungs pass. Gate
thresholds are adaptive via EMA.

**Gate-failure replan** -- When a task fails a gate, roko can automatically generate a
revised plan addressing the failure via `build_gate_failure_plan_revision` in
orchestrate.rs.

**HDC (Hyperdimensional Computing)** -- A computational paradigm that represents
concepts as high-dimensional binary vectors (10,240 bits = 1,280 bytes per vector in
roko). Similarity between concepts is measured by Hamming distance (how many bits
differ). With SIMD (AVX-512), a single CPU can compare a query vector against 100,000
stored vectors in approximately 170 microseconds. Roko uses HDC vectors as compact
"fingerprints" for knowledge entries, code structures, and agent episodes. Implemented
in `roko-primitives` as `HdcVector = [u64; 160]`.

**MCP (Model Context Protocol)** -- A protocol for tool invocation between AI agents
and external services. Configured per-agent in `roko.toml`. Supports stdio, HTTP, and
SSE transports. Roko has MCP servers for code intelligence (`roko-mcp-code`), GitHub,
Slack, and scripts.

**Neuro store** -- Roko's durable, local, embedding-indexed knowledge database. Stores
knowledge entries with 6 kinds, 4 tiers, HDC similarity search, and half-life decay.
Implemented in `roko-neuro`. Currently local-only; the daeji integration adds on-chain
hash anchoring and cross-agent sync via NeuroChainSync.

**NeuroChainSync** -- The bidirectional sync component between the local neuro store
and the on-chain InsightBoard. Push: promote entries with confidence >= 0.70 and 3+
local confirmations to chain. Pull: scan InsightPosted events from other agents, ingest
into local store at initial confidence 0.5.

**orchestrate.rs** -- The main execution loop. Located at
`crates/roko-cli/src/orchestrate.rs`. Dispatches agents, runs gates, records episodes,
handles replanning. The central integration point for chain features.

**Playbook** -- A reusable pattern extracted from a successful episode. Stored at
`.roko/learn/playbooks.json`. Injected into layer 4 of the SystemPromptBuilder so
agents benefit from proven approaches.

**ProcessSupervisor (PlanRunner)** -- The component in `roko-runtime` that manages
subprocess lifecycles: starting, stopping, and health-checking LLM agent processes.
With daeji Tier 2 integration, also manages the secondary peer subprocess.

**PRD (Product Requirements Document)** -- A structured document describing a feature
or fix. Roko reads PRDs and generates implementation plans (tasks.toml) from them.
Stored in `.roko/prd/`.

**SystemPromptBuilder** -- The 9-layer prompt assembly pipeline in `roko-compose`. See
the SystemPromptBuilder section above for the complete layer breakdown. Uses VCG
auction for context allocation in layer 9.

**VCG Auction (Vickrey-Clarke-Groves)** -- A mechanism used in layer 9 of the
SystemPromptBuilder to allocate limited context window space among competing context
bidders (Neuro, Task, and Research attention bidders). Each bidder "bids" for token
budget; the VCG mechanism determines the efficient allocation. Currently built and
exported (`vcg_allocate`) but the greedy path dominates at runtime.

### Blockchain and Cryptography Concepts

**BLS12-381** -- A pairing-friendly elliptic curve used for digital signatures.
Supports signature aggregation (combine many signatures into one) and threshold
signatures (T-of-N parties can produce a valid signature, but fewer than T cannot).
The "12-381" refers to the curve parameters. Used by Ethereum 2.0, Zcash, and many
other blockchain systems. Daeji uses it for consensus finalization.

**BTLE (Binding Timelock Encryption)** -- A cryptographic scheme where a message is
encrypted such that it can only be decrypted after a specific future event (in this
case, the production of a specific block). The decryption key is derived from the
BLS12-381 threshold signature at that block. "Binding" means the ciphertext commits to
the plaintext: you cannot later claim it encrypted something different. Implemented via
Identity-Based Encryption (IBE) using BLS12-381 pairings.

**Coinbase/Beneficiary** -- The `block.coinbase` Solidity field that identifies the
block proposer's Ethereum address. In daeji, currently always `Address::ZERO` (a known
gap). The fix derives an Ethereum address from the proposing validator's Ed25519 key.

**DKG (Distributed Key Generation)** -- A multi-party cryptographic protocol where N
participants collectively generate a shared secret key without any single participant
ever learning the complete key. Each participant ends up with a "share" of the key.
Any T-of-N shares can reconstruct the key's signing capability, but fewer than T
cannot. Two modes: interactive Joint-Feldman (secure, multi-round) and trusted-dealer
(fast, single process generates all shares).

**DSMR (Decoupled State Machine Replication)** -- Commonware's ordered broadcast
protocol where multiple sequencers broadcast messages concurrently and validators
finalize them by referencing each sequencer's certified tip. Relevant to Phase 3+
where agents could act as sequencers.

**Ed25519** -- An elliptic curve signature scheme used for peer identity in the
commonware P2P overlay network. Keys are 32 bytes private / 32 bytes public. Fast
signing and verification. Used by daeji validators and secondary peers for P2P
identity. Separate from the BLS12-381 consensus keys and the secp256k1 transaction
signing keys.

**EIP-1559** -- Ethereum's fee market mechanism with a dynamic base fee that adjusts
based on block utilization, plus an optional priority fee (tip). The base fee is
burned. Daeji currently has a hardcoded 1 gwei gas price with no dynamic adjustment
(a known gap).

**Episode hash** -- The BLAKE3 hash of a serialized episode JSON record. This 32-byte
value is what gets anchored on-chain during witness anchoring.

**EVM (Ethereum Virtual Machine)** -- A stack-based virtual machine that executes smart
contract bytecode. Ethereum, daeji, and many other chains use it. Smart contracts
written in Solidity are compiled to EVM bytecode.

**Finality certificate** -- A compact proof that a block was finalized: a BLS12-381
threshold signature (96 bytes) over the block hash, plus metadata. Verifiable with
only the 48-byte group public key. Daeji's certificates are ~240 bytes total.

**Gas** -- The unit of computational cost in the EVM. Each opcode has a gas cost.
Transactions specify a gas limit. If execution exceeds the limit, the transaction
reverts.

**Group public key** -- The single BLS12-381 public key (48 bytes) that corresponds to
the threshold secret shared among validators. Anyone can verify signatures made by the
group using only this key. Remains constant even across validator set resharing.

**IBE (Identity-Based Encryption)** -- A public-key encryption scheme where the
"public key" can be any arbitrary string (like a future block number). Used in BTLE:
encrypt to identity = "view 1000", decrypt when view 1000's VRF output becomes
available.

**InsightBoard** -- The Solidity smart contract on daeji that stores knowledge entry
metadata on-chain: content hash (32 bytes), entry type, half-life, poster address,
pheromone count. Full content is emitted in event logs, not stored in contract state.

**jsonrpsee** -- A Rust library for building JSON-RPC servers and clients, created by
the Parity team. Supports both HTTP and WebSocket transports. Used by daeji for its RPC
layer. Natively supports `#[subscription]` for push-based event streaming.

**Merkle proof (inclusion/exclusion)** -- A compact cryptographic proof that a specific
key-value pair is (or is not) part of a Merkle tree, without revealing the entire tree.
The proof is a path of sibling hashes from leaf to root. Size is logarithmic in the
number of entries (~20 hashes for 1 million entries). Daeji's QMDB can generate
historical proofs against any finalized block's state root.

**Merkle Mountain Range (MMR)** -- An append-only authenticated log from
commonware-storage. Elements are appended and never modified. The current peak is a
cryptographic commitment to all prior entries. Proposed for tamper-evident episode logs.

**Pheromone count** -- The number of times other agents have confirmed a knowledge entry
as useful. Inspired by stigmergy (ant colony coordination). Higher pheromone count =
more agents found this entry helpful.

**Precompile (precompiled contract)** -- Native code at a reserved EVM address. Called
like a regular contract but executes compiled Rust instead of interpreted EVM bytecode.
Standard Ethereum has 9 at addresses 0x01-0x09. Daeji adds custom precompiles for HDC
search (0x09), QMDB proofs (0x0B), and BTLE (0x0C).

**Predictive Foraging** -- A knowledge calibration mechanism where agents register
falsifiable predictions before task execution ("I predict using entries A, B, C will
help me score 0.8 in 45 minutes"). After execution, an external verifier (compiler,
test suite -- never the LLM itself) determines the actual outcome. The residual
calibrates future knowledge retrieval.

**prevrandao / mixHash** -- The block header field that contains the VRF output. In
standard Ethereum, this was the PoW mix hash; post-merge, it carries the RANDAO value.
In daeji, it carries the threshold BLS12-381 VRF output.

**QMDB (Quick Merkle Database)** -- A storage engine from commonware that provides
authenticated key-value storage with O(1) SSD I/O per state update and in-memory
Merkleization. Every commit produces a Merkle root. Used by daeji to store EVM state.
Note: daeji's current state root uses a transition hash formula, not a standard Merkle
Patricia Trie root, which means `eth_getProof` is not available (a known gap).

**REVM (Rust Ethereum Virtual Machine)** -- A Rust implementation of the Ethereum
Virtual Machine. Executes EVM bytecode and produces the same results as go-ethereum's
EVM. Used by daeji for smart contract execution, by Foundry for local testing, and by
Reth as a production execution client. Modular: parameterized over a `Database` trait
and a precompile set, making it straightforward to add custom precompiles.

**Secondary peer (follower)** -- A daeji node that connects to the P2P network using
an Ed25519 identity key and receives all finalized blocks, but does not participate in
consensus (no voting, no block proposals). Useful as a read-only RPC endpoint. A roko
agent can run as a secondary peer (Tier 2 integration) for sub-millisecond block
finalization notification and direct QMDB state access.

**secp256k1** -- The elliptic curve used by Ethereum for transaction signing. Ethereum
addresses are derived from secp256k1 public keys. Separate from Ed25519 (P2P identity)
and BLS12-381 (consensus signatures).

**Simplex BFT** -- A Byzantine Fault Tolerant consensus protocol implemented in
commonware. A block is proposed, voted on, and finalized in 3 network message hops with
single-slot finality (a finalized block will never be reverted). Uses BLS12-381
threshold signatures for voting. "Byzantine Fault Tolerant" means the protocol produces
correct results even if some participants are malicious (up to T-1 out of N
participants, where T is the threshold).

**Stigmergy** -- A coordination model from biology (ant colonies). Agents never
communicate directly -- they read and write signals (pheromones / knowledge entries) in
a shared environment, and the environment's state guides behavior. The InsightBoard's
pheromone counting implements this: agents post entries, other agents confirm entries
that helped them, high-confirmation entries become more prominent.

**Threshold signature** -- A cryptographic signature scheme where N parties each hold a
"share" of a secret key, and any T-of-N parties can combine their shares to produce a
valid signature, but fewer than T parties cannot. No individual party ever holds the
complete key. In daeji's devnet: N=4 validators, T=3 (any 3 can finalize a block).

**VRF (Verifiable Random Function)** -- A function that produces a random output along
with a proof that the output was correctly computed. In daeji, the VRF output at each
block is derived from the BLS12-381 threshold signature over the view number. Because
the threshold signature requires T-of-N validators, no single validator can predict or
manipulate the random output, making it bias-resistant. Roko uses VRF for verifiable
model routing (CascadeRouter) and A/B experiment assignment (ExperimentStore).

**Witness anchoring** -- The process of posting a cryptographic hash of an episode
record to the daeji chain as transaction calldata. Creates a tamper-evident record:
anyone with the original episode data can recompute the hash and verify it against the
on-chain record. The block timestamp provides a lower bound on when the episode
occurred. Implemented by `ChainWitnessEngine` in `roko-chain`.

---

## Repository Locations

| Project | Repository | Language | Description |
|---|---|---|---|
| Roko | `github.com/Nunchi-trade/roko` | Rust | AI agent toolkit (18 crates, ~177K LOC) |
| Daeji | `github.com/Nunchi-trade/daeji` | Rust | Minimal EVM blockchain on commonware primitives |
| Commonware | `github.com/commonwarexyz/monorepo` | Rust | 17 composable blockchain primitives |
