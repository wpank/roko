# Integrating a Self-Developing AI Agent Toolkit with a Minimal EVM Blockchain

This document describes how two systems -- Roko (an AI agent toolkit) and Daeji (a
minimal blockchain) -- connect, and provides a complete technical guide to implementing
that connection across three tiers of integration depth. Every roko concept is explained
from first principles. No prior knowledge of either system is assumed.

---

## The Two Systems

### Roko: A Toolkit for AI Agents That Develop Themselves

Roko is a Rust toolkit (~177K lines of code, organized into 18 crates) for building AI
agents that can autonomously develop software. The system is designed to develop itself:
it reads product requirements, generates implementation plans, executes those plans via
AI agents, validates the results, learns from outcomes, and persists what was learned
for future runs.

#### The Universal Loop

Every operation in roko follows the same 8-step loop:

```
query -> score -> route -> compose -> act -> verify -> write -> react
```

1. **Query**: A task enters the system. This could be a task from a plan DAG, a PRD
   (product requirements document), or a user prompt. The system queries its knowledge
   store and signals substrate for relevant context.

2. **Score**: Available context is scored for relevance and priority. The neuro knowledge
   store uses HDC vectors (10,240-bit binary vectors) for semantic similarity search.
   Attention bidders compete for prompt token budget.

3. **Route**: The CascadeRouter selects which LLM backend to use for this task. It uses
   a LinUCB bandit algorithm trained on historical outcomes: which model performed best
   for tasks with similar characteristics.

4. **Compose**: The 9-layer SystemPromptBuilder assembles a cache-aligned system prompt
   from composable fragments (role identity, conventions, domain context, active
   signals, task details, gate feedback, tool instructions, learned techniques, and
   anti-patterns). High-scoring knowledge and playbooks from step 2 are injected here.

5. **Act**: An agent is dispatched with the composed prompt. The agent interacts with an
   LLM backend (Claude CLI, Claude API, Ollama, Gemini, Perplexity, OpenAI-compatible,
   or others) in a tool loop: the LLM receives the prompt plus tool schemas, makes
   tool calls (read files, write files, run shell commands, search code), receives
   tool results, and iterates until the task is complete.

6. **Verify**: The gate pipeline runs validation checks on the agent's output. There are
   7 rungs in the canonical pipeline (Compile, Lint, Test, Symbol, GeneratedTest,
   PropertyTest, Integration/LLMJudge) plus standalone gates (Diff, CodeExecution,
   Benchmark, Security, Format). Each rung produces a Verdict: passed or failed, with
   evidence and logs. Adaptive thresholds (EMA-based) adjust pass criteria based on
   historical gate performance.

7. **Write**: Results are persisted. Episode logs record every agent turn, gate verdict,
   model used, tokens consumed, and cost. State snapshots enable plan resumption after
   interruption. High-confidence knowledge extractions are written to the neuro store.

8. **React**: The system responds to outcomes. If a gate fails, the replan mechanism can
   generate a revised plan and retry (configurable via `learning.replan_on_gate_failure`
   in roko.toml). Error patterns are recorded for future avoidance. The CascadeRouter
   updates its model performance data.

#### The Architecture: 1 Noun + 6 Verb Traits

Roko's type system is built on one noun type and six verb traits:

**The noun: Engram** (called "Signal" in the abstract architecture). An Engram is a
typed data container with a kind (Task, Plan, Gate, Episode, Knowledge, etc.), a body,
provenance metadata, content hash, and timestamps. Everything that flows through the
system -- tasks, prompts, verdicts, knowledge entries, episodes -- is an Engram.

**The 6 verb traits:**

| Trait | What it does | Primary implementation |
|-------|-------------|----------------------|
| **Store** (Substrate) | Persist and query signals | `FileSubstrate` in roko-fs (append-only JSONL) |
| **Score** | Rank signals by relevance | HDC similarity in roko-neuro, attention bidders in roko-compose |
| **Verify** (Gate) | Validate signal correctness | 7-rung pipeline in roko-gate |
| **Route** | Select processing path | CascadeRouter in roko-learn (LinUCB bandit) |
| **Compose** | Assemble prompts from signals | SystemPromptBuilder in roko-compose (9 layers) |
| **React** (Policy) | Respond to outcomes | Replan on failure, error pattern recording |

#### The 18 Crates

| Crate | Path | What It Does |
|-------|------|-------------|
| **roko-core** | `crates/roko-core/` | The kernel. Defines Engram, the 6 verb traits, types, config schema, tool definitions, and error types. Everything depends on this crate. |
| **roko-cli** | `crates/roko-cli/` | The CLI binary. Contains `orchestrate.rs` (the main plan execution loop), all subcommands (`roko plan run`, `roko prd`, `roko research`, `roko dashboard`, etc.), and the ratatui TUI. This is the entry point for everything. |
| **roko-agent** | `crates/roko-agent/` | Manages LLM backends. Supports 5+ backends: Claude CLI (spawns `claude` subprocess), Claude API (direct HTTP), Ollama (local models), Gemini, Perplexity, and any OpenAI-compatible API. Implements the tool loop (prompt -> tool calls -> feedback -> iterate), MCP (Model Context Protocol) for external tool servers, and the safety layer (role authorization, pre/post action checks). |
| **roko-agent-server** | `crates/roko-agent-server/` | Per-agent HTTP sidecar with 13 routes. Handles `/message` (real LLM dispatch), `/stream` (WebSocket streaming), `/predictions`, `/research`, and `/tasks`. Each agent can be addressed via its own HTTP endpoint. |
| **roko-gate** | `crates/roko-gate/` | The verification stack. 7-rung pipeline: CompileGate, ClippyGate, TestGate, SymbolGate, GeneratedTestGate, PropertyTestGate, LlmJudgeGate/IntegrationGate. Plus 6 standalone gates: DiffGate, CodeExecutionGate, ShellGate, BenchmarkRegressionGate, FormatCheckGate, SecurityScanGate. Plus composition wrappers: ParallelGate, VotingGate, FallbackGate. Plus adaptive thresholds (EMA per rung). |
| **roko-chain** | `crates/roko-chain/` | Blockchain client abstractions. Defines `ChainClient` (read-only: blocks, receipts, logs, storage, eth_call) and `ChainWallet` (sign and submit transactions) traits. Ships an Alloy-backed JSON-RPC implementation and mock test doubles. Also includes: AgentRegistry (soulbound ERC-721 agent passports), ReputationRegistry (7-domain EMA scoring), KoraiToken (lazy demurrage), Marketplace (job escrow), ChainWitnessEngine (attestation anchoring), MevDetector, TraceRank (PageRank reputation), and X402 (micropayment state channels). Currently Phase 2: built but not yet wired into the main execution loop. |
| **roko-compose** | `crates/roko-compose/` | Prompt assembly. The 9-layer SystemPromptBuilder produces cache-aligned, role-specific system prompts. 9+ role templates. Attention bidder framework for competitive context allocation. Token counting and budget enforcement. |
| **roko-orchestrator** | `crates/roko-orchestrator/` | Plan DAG representation, parallel executor, merge queue, and safety constraints. Plans are directed acyclic graphs of tasks; the executor runs independent tasks in parallel and respects dependency edges. |
| **roko-learn** | `crates/roko-learn/` | Learning and feedback. EpisodeLogger (JSONL recording of every agent turn). CascadeRouter (LinUCB bandit for model selection). ExperimentStore (A/B testing of prompts). Playbook store (reusable patterns extracted from successful runs). Efficiency events (C-Factor per-turn metrics). Error patterns. Section effectiveness tracking. Skill library. |
| **roko-neuro** | `crates/roko-neuro/` | Durable knowledge store. 6 knowledge kinds (Insight, Heuristic, Warning, AntiKnowledge, CausalLink, StrategyFragment) with type-specific half-lives. 4 retention tiers (Transient -> Working -> Consolidated -> Persistent) with promotion/demotion rules. HDC vector indexing for semantic search. Confidence scoring with temporal decay. Admission gating. Tier progression engine. Dream consolidation (offline knowledge distillation). |
| **roko-runtime** | `crates/roko-runtime/` | Process supervision and async runtime primitives. ProcessSupervisor manages subprocess lifecycles (spawn, track, shutdown, kill, reap). CancelToken for cooperative cancellation. EventBus for typed broadcast. JsonlLogger for structured metrics. Workflow engine. Run ledger. |
| **roko-serve** | `crates/roko-serve/` | HTTP control plane. ~85 REST routes + SSE + WebSocket on port 6677. Exposes plan execution, agent management, learning data, knowledge queries, and PRD lifecycle to external dashboards and API consumers. |
| **roko-conductor** | `crates/roko-conductor/` | 10 watchers (file system, network, health, etc.), circuit breaker, diagnosis. Used by the executor to monitor agent health and detect degradation. |
| **roko-fs** | `crates/roko-fs/` | FileSubstrate: append-only JSONL storage with GC. The default persistence backend for signals. Layout utilities for `.roko/` directory structure. |
| **roko-std** | `crates/roko-std/` | Default configuration, 19 builtin tools (Read, Write, Bash, Grep, Glob, etc.), and mock dispatcher for testing. |
| **roko-primitives** | `crates/roko-primitives/` | HDC vectors (10,240-bit binary vectors for semantic similarity), tier routing, fingerprinting. |
| **roko-dreams** | `crates/roko-dreams/` | Offline knowledge consolidation: hypnagogia (pre-sleep synthesis), imagination (counterfactual exploration), dream cycle (consolidation loop). Triggered after plan completion. |
| **roko-daimon** | `crates/roko-daimon/` | Affect engine: somatic markers, emotional state tracking, dispatch modulation. The DaimonState is loaded per-task and influences agent behavior (e.g., increased caution after recent failures). |

Additional crates that exist but are not core to the integration story: `roko-index`
(code intelligence parser + graph), `roko-lang-rust/typescript/go` (language support),
`roko-mcp-code/github/slack/scripts/stdio` (MCP server implementations).

#### How Agents Work

When roko dispatches an agent to work on a task, the sequence is:

1. **Backend selection**: The CascadeRouter picks a model. Roko supports 5+ backend
   types, each with different tradeoffs:
   - **Claude CLI**: spawns a `claude` subprocess with `--print`, `--output-format json`,
     and `--allowedTools`. The subprocess manages its own tool loop. ProcessSupervisor
     tracks its lifecycle.
   - **Claude API**: direct HTTP to Anthropic's API. Roko manages the tool loop.
   - **OpenAI-compatible**: any API that speaks the OpenAI chat completions format
     (Ollama, OpenRouter, Gemini, Moonshot, Cerebras, ZhipuAI, etc.).
   - **Perplexity**: specialized for research tasks with citation support.
   - **Codex**: OpenAI's code generation backend.

2. **Prompt composition**: The SystemPromptBuilder produces a 9-layer system prompt:
   - Layer 1 (Role identity): "You are an implementer agent working on roko..."
   - Layer 2 (Conventions): "Use snake_case, thiserror for errors, no unwrap in libs..."
   - Layer 3 (Domain context): Project-specific knowledge from neuro store queries
   - Layer 3c (Active signals): Pheromone/stigmergic guidance from the substrate
   - Layer 4 (Task context): The specific task description and acceptance criteria
   - Layer 4b (Gate feedback): If retrying after a gate failure, the failure digest
   - Layer 5 (Tool instructions): Available tools, MCP servers, usage patterns
   - Layer 6 (Relevant techniques): Learned playbooks and skills
   - Layer 7 (Anti-patterns): What NOT to do, from AntiKnowledge entries

3. **Tool loop**: The agent receives the prompt plus a schema of available tools. It
   makes tool calls (Read, Write, Bash, Grep, Glob, etc.), receives results, and
   iterates until done. If MCP (Model Context Protocol) servers are configured, the
   agent can also call tools exposed by external servers.

4. **Safety layer**: Role authorization checks (what can this agent role do?), pre-action
   checks (is this tool call safe?), and post-action checks (did the tool call produce
   expected results?).

#### How Gates Work

After an agent completes a task, the gate pipeline validates its output. The pipeline
is configured per-plan based on complexity (the `RungSelector` chooses how many rungs
to run). Here is the full 7-rung pipeline:

| Rung | Index | Gate(s) | What It Checks |
|------|-------|---------|----------------|
| Compile | 0 | `CompileGate` | `cargo build` succeeds -- the code compiles |
| Lint | 1 | `ClippyGate` | `cargo clippy` produces no warnings |
| Test | 2 | `TestGate` | `cargo test` passes -- existing tests still work |
| Symbol | 3 | `SymbolGate` | Expected symbols exist in the compiled output |
| GeneratedTest | 4 | `GeneratedTestGate` + `VerifyChainGate` | Auto-generated tests pass; chain interactions verify |
| PropertyTest | 5 | `PropertyTestGate` + `FactCheckGate` | Property-based tests hold; factual claims verified |
| Integration | 6 | `LlmJudgeGate` + `IntegrationGate` | LLM reviews the diff; integration tests pass |

Plus standalone gates invoked for specific scenarios:
- **DiffGate**: git diff analysis (size, complexity, risk assessment)
- **CodeExecutionGate**: sandboxed code execution
- **ShellGate**: arbitrary shell command verification
- **BenchmarkRegressionGate**: performance regression detection
- **FormatCheckGate**: code formatting compliance
- **SecurityScanGate**: security vulnerability scanning

Each gate returns a `Verdict`: passed or failed, with evidence (what was checked) and
logs (raw output). Adaptive thresholds use exponential moving averages per rung to
adjust pass criteria based on historical performance. If rung 2 (Test) has been failing
30% of the time, the threshold for what counts as a "pass" can be adjusted.

When gates fail, the React step can trigger replanning: `learning.replan_on_gate_failure`
in roko.toml enables automatic generation of a revised plan based on the gate failure
evidence. Up to `replan_max_per_plan` retries (default 2).

#### How Learning Works

Roko has 6 learning subsystems, all in `crates/roko-learn/`:

1. **EpisodeLogger**: Records every agent turn to `.roko/episodes.jsonl`. Each episode
   includes: the task, model used, prompt sent, agent output, tool calls, gate verdicts,
   tokens consumed, cost in USD, and duration.

2. **CascadeRouter**: A LinUCB contextual bandit that selects which LLM model to use
   for each task. Context features include task complexity, domain, and historical model
   performance. Persisted to `.roko/learn/cascade-router.json`. Configured in
   `roko.toml` under `[routing]`: algorithm, discount factor, fast/standard/complex
   task models, quality/cost/latency weights.

3. **ExperimentStore**: A/B tests on system prompt variants. Randomly assigns tasks to
   variant groups, tracks outcomes, and identifies statistically significant winners.
   Persisted to `.roko/learn/experiments.json`.

4. **Playbook store**: Reusable patterns extracted from successful runs. When an agent
   successfully completes a task type multiple times with similar approaches, the
   pattern is captured as a playbook and injected into future prompts for similar tasks
   (via Layer 6 of the SystemPromptBuilder).

5. **Efficiency events**: Per-turn C-Factor metrics written to
   `.roko/learn/efficiency.jsonl`. Tracks token utilization, cost efficiency, and
   output quality ratios.

6. **Error patterns**: Recurring failure modes extracted from gate failures. Written to
   `.roko/learn/error-patterns.json`. Fed back into anti-pattern Layer 7 of the
   SystemPromptBuilder so agents learn to avoid known failure modes.

#### The Knowledge Layer (Neuro Store)

The neuro store (`crates/roko-neuro/`) is roko's durable knowledge database. It
persists learned observations across runs and injects them into agent prompts.

**6 Knowledge Kinds** (each with a different temporal half-life):

| Kind | Half-Life | Description |
|------|-----------|-------------|
| Insight | 30 days | Compact causal observations distilled from episodes |
| Heuristic | 90 days | Rules of thumb and learned tendencies |
| Warning | 1 hour | Cautionary notes about active failure modes |
| AntiKnowledge | 30 days | What to avoid; what has failed |
| CausalLink | 60 days | Causal relationships between observations |
| StrategyFragment | 14 days | Reusable approach fragments for plan composition |

Half-lives control temporal decay: an Insight created 30 days ago has half the effective
weight of a fresh one. Warnings decay very fast (1 hour) because they represent
transient conditions. Heuristics persist the longest (90 days) because behavioral
strategies that work tend to remain valid.

**4 Retention Tiers** (with promotion/demotion rules):

| Tier | Multiplier | Promotion Requirement |
|------|-----------|----------------------|
| Transient | 0.1x | Entry point for all new knowledge |
| Working | 0.5x | 2+ successful uses (confirmation_count >= 2) |
| Consolidated | 1.0x | 3+ distinct task contexts |
| Persistent | 5.0x | 5+ tier progression passes, high confidence |

Knowledge entries progress through tiers based on evidence: a Transient entry that
proves useful in 2+ tasks promotes to Working. A Working entry validated across 3+
distinct task contexts promotes to Consolidated. Demotion also occurs: entries that
stop being useful or whose confidence drops below threshold are demoted.

**HDC (Hyperdimensional Computing) vectors**: Each knowledge entry is indexed with a
10,240-bit binary vector for fast approximate semantic similarity search. When a new
task enters the system, the query is encoded as an HDC vector and compared against all
knowledge entries using Hamming distance. The most relevant entries are injected into
the agent's system prompt (Layer 3: Domain context).

**Queried at dispatch time**: Before every agent dispatch, `orchestrate.rs` queries the
neuro store for knowledge relevant to the current task. Matching entries flow into the
SystemPromptBuilder as domain context.

#### The 9-Layer System Prompt Builder

The SystemPromptBuilder in `crates/roko-compose/src/system_prompt_builder.rs` assembles
prompts in cache-layer order:

| Layer | Content | Cache Tier | Volatility |
|-------|---------|-----------|-----------|
| 1. Role identity | Who am I, what is my job | System | Stable across sessions |
| 2. Conventions | Project coding standards | System | Semi-stable |
| 3. Domain context | Neuro store knowledge, project-specific info | Session | Semi-stable |
| 3c. Active signals | Pheromone/stigmergic guidance from substrate | Session | Semi-stable |
| 4. Task context | Current task description and criteria | Task | Volatile per task |
| 4b. Gate feedback | Prior verification failure digest (on retry) | Dynamic | Only on retry |
| 5. Tool instructions | Available tools and MCP servers | System | Stable |
| 6. Relevant techniques | Learned playbooks and skills | Task | Volatile per task |
| 7. Anti-patterns | What NOT to do (from AntiKnowledge) | Task | Volatile per task |
| 8. Affect guidance | Emotional tone and focus (from DaimonState) | Dynamic | Varies |

Layers 1+2+5 form the "system" tier (prefix-cacheable, rarely changes). Layers 3+3c
form the "session" tier. Layers 4+6+7 are per-task. Layers 4b+8 are dynamic.

The builder enforces a token budget and uses attention bidders to allocate prompt space
among competing context sources (neuro knowledge, task details, playbooks, etc.).

#### Persistence (`.roko/` Directory)

All roko state lives under the `.roko/` directory in the workspace root:

| File/Directory | What It Stores |
|---------------|---------------|
| `.roko/signals.jsonl` | All signals (the substrate -- append-only log of all Engrams) |
| `.roko/episodes.jsonl` | Episode log (every agent turn with verdicts and costs) |
| `.roko/state/executor.json` | Executor snapshot (enables `--resume` after interruption) |
| `.roko/state/process-sessions.json` | ProcessSupervisor durable process state |
| `.roko/prd/` | Product requirements documents |
| `.roko/research/` | Research artifacts (topics, searches, enhancements) |
| `.roko/learn/cascade-router.json` | CascadeRouter model performance data |
| `.roko/learn/gate-thresholds.json` | Adaptive gate threshold EMA values |
| `.roko/learn/experiments.json` | A/B experiment state |
| `.roko/learn/efficiency.jsonl` | Per-turn C-Factor metrics |
| `.roko/learn/error-patterns.json` | Recurring failure patterns |
| `.roko/neuro/` | Knowledge store entries |

#### Configuration (`roko.toml`)

Roko is configured via a single TOML file at the workspace root. Key sections relevant
to chain integration:

```toml
[chain]
rpc_url = "http://127.0.0.1:8545"        # JSON-RPC endpoint of daeji devnet
chain_id = 31337                           # Must match devnet genesis chain_id
wallet_key = "0xac0974..."                 # secp256k1 signing key (hex, 32 bytes)
agent_registry = "0x9fE467..."             # Deployed AgentRegistry contract
bounty_market = "0xDc64a1..."              # Deployed BountyMarket contract

[agent]
default_model = "claude-sonnet"            # Default LLM for task dispatch
default_backend = "anthropic"              # Default provider
temperament = "balanced"                   # Affects DaimonState guidance

[gates]
clippy_enabled = true                      # Include Lint rung
skip_tests = false                         # Include Test rung
max_iterations = 3                         # Gate retry limit before failing

[routing]
algorithm = "linucb"                       # CascadeRouter algorithm
fast_task_model = "claude-haiku-4-5"       # Model for simple tasks
standard_task_model = "claude-sonnet-4-6"  # Model for typical tasks
complex_task_model = "claude-opus-4-6"     # Model for complex tasks

[learning]
replan_on_gate_failure = true              # Auto-replan when gates fail
replan_max_per_plan = 2                    # Max replans per plan run
dream_on_completion = true                 # Run dream consolidation after plans

[relay]
heartbeat_interval_secs = 30               # On-chain heartbeat frequency
```

### Daeji: A Minimal EVM Blockchain

Daeji (internally codenamed "Kora") is a minimal Ethereum Virtual Machine (EVM)
blockchain built from scratch using the commonware library suite. It is not a fork of
geth, Reth, or any existing Ethereum client -- it is constructed from first principles
using commonware's consensus, networking, storage, and cryptography primitives.

Key properties:

- **Consensus**: Simplex BFT with BLS12-381 threshold signatures. In the devnet
  configuration, 4 validators run consensus, and any 3 of them (a 3-of-4 threshold)
  can collectively produce a valid block signature.
- **Execution**: REVM, the Rust EVM implementation used by Foundry and Reth.
- **State storage**: QMDB (Quick Merkle Database), an authenticated key-value store
  where every commit produces a Merkle root that can be used to generate cryptographic
  proofs of inclusion or exclusion.
- **Chain ID**: 1337
- **Block time**: ~400ms (dependent on consensus round timing)
- **RPC interface**: Standard Ethereum JSON-RPC (`eth_*`, `net_*`, `web3_*` namespaces)
  plus a custom `kora_*` namespace for chain-specific queries.

### Why Connect Them

Today, roko agents learn and persist knowledge only within a single run on a single
machine. The neuro store (the local knowledge database) does not share data across
agent fleets or operators. There is no shared infrastructure for:

- **Witness anchoring**: creating a tamper-evident, cryptographic proof that a task
  completed and what its outcome was. Currently, episode logs are plain JSONL files
  that anyone with file access could edit. Roko already has a `ChainWitnessEngine`
  in the roko-chain crate that computes `blake3(episode_data)` and submits the hash
  as on-chain calldata -- it just needs a chain to submit to.
- **Cross-agent knowledge sharing**: insights discovered by one agent fleet being made
  available to another. The neuro store has 6 knowledge kinds and 4 retention tiers,
  but everything stays local. An on-chain InsightBoard contract provides the shared
  ledger.
- **Novel cryptographic primitives**: verifiable randomness (VRF), sealed commitments,
  compact finality certificates -- none of which are available from local file storage.
  The CascadeRouter and ExperimentStore currently use local PRNG for randomization;
  on-chain VRF makes selection verifiable by third parties.

Daeji provides all of these. The integration connects roko's execution loop to a
shared, cryptographically authenticated ledger.

---

## Commonware: The Library Suite Daeji Is Built On

Commonware is a collection of independently-usable Rust crates for building distributed
systems. Understanding these crates is essential because roko can use them either
indirectly through daeji (via network calls) or directly as Cargo dependencies
(importing them into roko crates).

### commonware-cryptography

Provides three cryptographic primitives:

**Ed25519 signatures** -- fast elliptic-curve signatures used for peer identity in the
P2P overlay network. Each node (and, in the deepest integration tier, each roko agent)
has an Ed25519 keypair. Ed25519 keys are 32 bytes and signing/verification is very fast.

**BLS12-381 threshold signatures** -- the consensus signing scheme used by daeji
validators. With a 3-of-4 threshold, any 3 of the 4 validators can collectively produce
a signature that is indistinguishable from a single-signer signature. The group public
key (48 bytes) never changes even when the validator set is reshared. This is important
because it means a single compact public key can verify any block, regardless of which
specific 3 validators signed it.

**VRF (Verifiable Random Function)** -- the threshold signature over a consensus view
number serves as a deterministic, unpredictable, bias-resistant random number. Because
it requires threshold participation (3 of 4 validators), no single validator can
influence the output. Daeji exposes this as the `prevrandao` field in block headers
(the standard Ethereum field used for on-chain randomness).

Key types:
```rust
// Ed25519 identity keys
use commonware_cryptography::ed25519::{PrivateKey, PublicKey, Signature};

// BLS12-381 threshold signatures
use commonware_cryptography::bls12381::{
    PrivateKey, PublicKey,           // individual validator key
    Partial, Threshold,              // partial and combined threshold sigs
    dkg::{JointFeldman, TrustedDealer},  // DKG ceremony types
};
```

### commonware-p2p

Authenticated, encrypted peer-to-peer networking. Every peer is identified by its
Ed25519 public key. Connections are authenticated via the identity key -- there is no
unauthenticated path into the network overlay.

The `authenticated` module provides a `Network` type that manages connections to a set
of known peers (listed in a `peers.json` config file), delivers messages to
application-level handlers identified by peer public key, and handles reconnection,
backpressure, and message ordering.

The `simulated` module replaces real TCP with a deterministic in-process network, used
in tests. The same application code runs against it with no sockets required.

### commonware-storage

Two storage backends:

**QMDB (Quick Merkle Database)** -- an authenticated key-value store where every commit
produces a Merkle root. It supports `get(key)` / `put(key, value)` / `delete(key)`
operations, a `root()` call that returns the current Merkle root (included in every
block header), and `prove(key)` which generates a Merkle inclusion/exclusion proof valid
against a given root. Historical roots are retained, enabling proofs against any
finalized block.

**MMR (Merkle Mountain Range)** -- an append-only authenticated log. New elements are
appended; the current peak is a cryptographic commitment to all prior entries. Elements
are never modified after insertion. Well-suited to tamper-evident logs like episode
histories.

### commonware-runtime

Two runtime backends with the same interface:

**`runtime::deterministic`** -- runs async tasks on a single thread with a seeded PRNG
for all concurrency decisions and random number generation. Given the same seed, every
run produces identical behavior. Used in tests and for reproducible simulation.

**`runtime::tokio`** -- the production runtime. Same application code, real async I/O on
top of the Tokio async runtime.

The interface unification means: write application code once, test it deterministically
with a fixed seed, deploy it on Tokio in production.

### commonware-codec

A `#[derive(Encode, Decode)]` codec for binary wire formats. Used throughout daeji and
commonware for P2P messages, block encoding, and certificate serialization. Produces
significantly smaller output than JSON or protobuf.

### commonware-broadcast

Ordered broadcast for multi-sequencer scenarios (DSMR -- Decoupled State Machine
Replication). Multiple sequencers broadcast messages concurrently; validators finalize
them by referencing each sequencer's certified tip. This is how daeji can scale
throughput beyond what a single block-proposer allows. Relevant to roko in Phase 3+,
when agents could potentially act as sequencers for their own message streams.

### commonware-resolver

A pluggable content-addressed storage layer for large data that should not be stored
inline in chain state. Clients request data by hash; resolvers fetch it from wherever it
lives (local disk, IPFS, a peer). Used in daeji's ordered broadcast for block data
availability.

---

## Three Integration Tiers

The integration is structured into three tiers, ordered by increasing depth. All three
tiers can coexist -- a roko deployment can start with Tier 1 (simple RPC calls) on day
one and add Tier 2 and Tier 3 incrementally without replacing anything.

---

## Tier 1: JSON-RPC Client

**What it is**: Roko agents talk to daeji over standard Ethereum JSON-RPC, the same way
any wallet or dApp talks to any Ethereum-compatible chain. Roko submits transactions,
reads state, and queries event logs using the Alloy library (the standard Rust Ethereum
client library, successor to ethers-rs).

**What it requires**: No new crate dependencies beyond Alloy, which is already present
in the `roko-chain` crate. The only configuration change is pointing the chain client
at daeji's endpoint via the `[chain]` section of roko.toml.

### Where Chain Data Enters the Universal Loop

Here is how daeji data flows into each step of roko's universal loop at Tier 1:

| Loop Step | Without Chain | With Chain (Tier 1) |
|-----------|--------------|-------------------|
| **Query** | Query local neuro store only | Also query InsightBoard contract for knowledge entries from other agent fleets |
| **Score** | Score local knowledge by HDC similarity | Also score on-chain entries, weighted by confirmation count and chain confidence |
| **Route** | CascadeRouter uses local PRNG for randomization | Can use `prevrandao` from latest finalized block as VRF seed for verifiable model selection |
| **Compose** | 9-layer SystemPromptBuilder with local data | Layer 3 (Domain context) includes on-chain knowledge; Layer 3c includes chain status signals |
| **Act** | Agent interacts with LLM via tool loop | Agent can additionally submit chain transactions via chain tools |
| **Verify** | 7-rung gate pipeline with local checks | New ChainWitness rung anchors episode hash on-chain; new ChainSimulate rung tests chain interactions |
| **Write** | Episode to JSONL, knowledge to neuro store | Episode includes `ChainAttestation {chain_id, tx_hash, block_number}`; high-confidence knowledge posted to InsightBoard |
| **React** | Replan on failure, update error patterns | Also update on-chain reputation scores based on gate outcomes |

### Available RPC Methods

**Standard Ethereum namespace (`eth_*`):**

| Method | Description |
|---|---|
| `eth_sendRawTransaction` | Submit a signed EIP-1559 transaction |
| `eth_call` | Read-only EVM execution (no state change) |
| `eth_estimateGas` | Estimate gas for a transaction |
| `eth_getBalance` | Get the ETH balance of an address |
| `eth_getTransactionCount` | Get the nonce (transaction count) for an address |
| `eth_getCode` | Get the bytecode deployed at a contract address |
| `eth_getStorageAt` | Read a specific storage slot from a contract |
| `eth_getBlockByNumber` | Fetch a block by number (includes `mixHash` = VRF seed) |
| `eth_getBlockByHash` | Fetch a block by hash |
| `eth_getTransactionByHash` | Fetch a transaction by hash |
| `eth_getTransactionReceipt` | Fetch the receipt of a mined transaction |
| `eth_getLogs` | Query event logs with topic and address filters |
| `eth_chainId` | Returns `0x539` (1337 decimal) |
| `eth_blockNumber` | Returns the latest finalized block number |
| `eth_gasPrice` | Returns current base fee |

**Daeji-specific extensions (`kora_*`):**

| Method | Description | Return Fields |
|---|---|---|
| `kora_nodeStatus` | Consensus health and peer info | `currentView`, `finalizedCount`, `nullifiedCount`, `peerCount`, `isLeader` |

Additional `kora_*` methods are planned for Phase 3:
- `kora_activeAgents` -- list agents registered in the AgentRegistry contract
- `kora_recentKnowledge` -- knowledge entries posted since a given block
- `kora_vrfSeed` -- the VRF seed (= `mixHash`) for a given finalized block

### What Roko Agents Can Do at Tier 1

1. **Deploy Solidity contracts** -- standard `eth_sendRawTransaction` with contract
   creation bytecode.
2. **Post knowledge entries** -- call `InsightBoard.post(contentHash, entryType,
   halfLifeHrs, content)` to share an insight on-chain.
3. **Confirm knowledge entries** -- call `InsightBoard.confirm(contentHash)` when an
   entry from another agent proved useful.
4. **Anchor episode witnesses** -- submit a transaction carrying
   `blake3(episode_data)` as calldata, creating a tamper-evident on-chain record of
   what happened. This is implemented in `roko-chain/src/witness.rs`: the
   `ChainWitnessEngine` constructs a `TxRequest` with the witness marker prefix
   `b"roko.attestation.witness:"` followed by the 32-byte blake3 hash, sends it to
   address `0x00...c0`, and records the `ChainAttestation` (chain_id, tx_hash,
   block_number) on the episode's `Attestation` struct.
5. **Register agent identity** -- call `AgentRegistry.register(ed25519Pubkey,
   capabilities)` to announce the agent's presence.
6. **Send heartbeats** -- call `AgentRegistry.heartbeat()` periodically (configured
   via `relay.heartbeat_interval_secs` in roko.toml, default 30 seconds) to prove the
   agent is still alive.
7. **Read chain state** -- `eth_call` into any contract for balances, reputation
   scores, knowledge weights.
8. **Watch events** -- `eth_getLogs` for `InsightPosted`, `InsightConfirmed`,
   `AgentRegistered` events.
9. **Check consensus health** -- `kora_nodeStatus` to decide whether to wait before
   submitting a transaction (e.g., if too many blocks have been nullified recently,
   consensus may be degraded).

### Configuration

The `[chain]` section of `roko.toml` configures the Tier 1 connection:

```toml
[chain]
# The JSON-RPC endpoint of a running daeji node.
# Use a validator endpoint for read+write, or a secondary endpoint for read-only.
rpc_url = "http://localhost:8545"

# The Ethereum chain identifier. Must match the devnet's genesis chain_id.
# 1337 for standard daeji devnet; 31337 for the current roko.toml default.
chain_id = 1337

# Agent's transaction signing key -- a secp256k1 private key (hex, 32 bytes).
# This is NOT the Ed25519 consensus key used by validators.
# For local dev, generate with: cast wallet new
# The address derived from this key must have ETH balance in the genesis allocations.
wallet_key = "${DAEJI_AGENT_KEY}"

# Contract addresses, populated after deploying with Foundry:
#   forge script contracts/script/Deploy.s.sol --rpc-url http://localhost:8545 --broadcast
agent_registry = "0x..."
bounty_market  = "0x..."
```

The `roko-chain` crate reads this config to instantiate its two key types:

```rust
// AlloyChainClient -- read-only operations (implements ChainClient trait)
// Methods: block_number(), get_block_header(), get_receipt(), get_logs(),
//          get_storage_at(), eth_call(), get_balance(), chain_id(), name()
let chain_client = AlloyChainClient::http(&config.chain.rpc_url)?;

// AlloyChainWallet -- sign and submit transactions (implements ChainWallet trait)
// Methods: address(), balance(), nonce(), sign_and_submit(), wait_for_receipt(), name()
let chain_wallet = AlloyChainWallet::from_hex_key(
    &config.chain.rpc_url,
    &config.chain.wallet_key,
    config.chain.chain_id,
)?;
```

Both traits are defined in `roko-chain/src/client.rs` and `roko-chain/src/wallet.rs`,
with mock implementations in `roko-chain/src/mock.rs` for testing without a real chain.

### Witness Anchoring (Tier 1 Core Feature)

Witness anchoring is the primary Tier 1 feature. After a task completes and all gate
rungs pass, the system creates an on-chain record of what happened. The implementation
lives in `roko-chain/src/witness.rs`:

```rust
// After gate_pipeline succeeds for a task:

// 1. The ChainWitnessEngine constructs a witness transaction:
//    - Destination: 0x00000000000000000000000000000000000000c0 (static sink)
//    - Calldata: b"roko.attestation.witness:" + blake3(episode_json) (32 bytes)
//    - Gas limit: 50,000

// 2. The wallet signs and submits the transaction:
let tx_hash = wallet.sign_and_submit(witness_tx).await?;

// 3. Wait for the receipt (30-second timeout):
let receipt = wallet.wait_for_receipt(&tx_hash, 30_000).await?;

// 4. Record chain attestation on the episode:
attestation.chain_attestation = Some(ChainAttestation {
    chain_id: client.chain_id().await?,
    tx_hash: tx_hash_bytes,
    block_number: receipt.block_number,
});
```

Verification (by anyone, at any time):

```rust
// verify_on_chain() checks:
// 1. Chain attestation exists on the episode
// 2. Receipt exists for the stored tx_hash
// 3. Chain ID matches
// 4. Receipt status is success
// 5. Block number matches
// 6. Receipt logs contain the witness topic and witness hash
let is_valid = ChainWitnessEngine::new()
    .verify_on_chain(&attestation, &client)
    .await?;
```

This creates a tamper-evident record: anyone with the episode data can recompute the
blake3 hash and verify it against the on-chain transaction calldata. The block timestamp
provides a lower bound on when the episode occurred, and the consensus signature on the
block proves the record was accepted by a majority of validators.

---

## Tier 2: Secondary Peer

**What it is**: A roko-controlled process runs as a daeji secondary peer -- it
authenticates to the P2P network, replicates all blocks in real time, but never
participates in consensus votes. There is zero consensus risk: a secondary peer cannot
influence or disrupt the chain.

### What a Secondary Peer Is

In commonware's network model, there are two participant types:

**Validators** -- hold DKG (Distributed Key Generation) key shares, participate in BFT
voting, propose and sign blocks. If a validator misbehaves or goes offline, it affects
chain liveness. In the 4-validator devnet, losing 2 validators would halt the chain
(since 3-of-4 are needed for threshold signatures).

**Secondary peers** -- hold an Ed25519 keypair (required for P2P authentication so the
network knows who they are), receive all block data and finality messages, but do not
sign anything consensus-related. They are read-only observers with cryptographic proof
of what they observe.

A secondary peer's Ed25519 public key must be listed in the network's `peers.json`
configuration before the devnet starts (or added via a network config update).

### Why Tier 2 Matters Beyond Tier 1

| Capability | Tier 1 (RPC Polling) | Tier 2 (Secondary Peer) |
|---|---|---|
| Learn about new blocks | Poll `eth_blockNumber` every N seconds | Push notification via P2P when block finalizes |
| Finalization latency | 1-10 seconds (poll interval) | Milliseconds (P2P message delivery) |
| State access | JSON-RPC overhead + serialization/deserialization | Direct QMDB read with Merkle proofs |
| Trust model | Trust the RPC node to return correct data | Verify blocks locally using the same code as validators |
| Bandwidth | Pull only what you request | Receive all block data (can archive the full chain history) |

The most impactful difference is trust: at Tier 1, roko trusts whatever the RPC
endpoint returns. At Tier 2, roko independently verifies every block by checking the
consensus signatures and replaying transactions against its local QMDB state copy.

### How Tier 2 Integrates with Roko's ProcessSupervisor

The secondary peer is a subprocess managed by roko's `ProcessSupervisor` (defined in
`crates/roko-runtime/src/process.rs`). ProcessSupervisor already manages Claude CLI
agent processes during plan execution -- adding a secondary peer process uses the same
infrastructure:

```rust
// During plan runner initialization in orchestrate.rs:

// 1. Spawn the secondary peer as a managed child process
let secondary_pid = supervisor.spawn(SpawnConfig {
    program: "kora",
    args: &["secondary",
            "--data-dir", ".roko/state/daeji-secondary",
            "--peers", "/path/to/peers.json",
            "--chain-id", "1337"],
    working_dir: Some(workspace_root),
    env: &[],
}).await?;

// 2. The supervisor tracks the process with a ProcessId
// 3. When the plan completes (or CancelToken fires):
supervisor.shutdown(secondary_pid).await;
// This sends SIGTERM, waits the grace period (5s), then SIGKILL if needed
```

The secondary peer starts when the plan executor starts and stops when it stops.
ProcessSupervisor's drop guard ensures cleanup even on unexpected termination.

### Running a Secondary Peer

```bash
# Generate a secondary peer keypair (run once, store the key)
cargo run --release --bin keygen -- secondary \
  --output-dir .roko/state/daeji-secondary

# The above writes:
#   .roko/state/daeji-secondary/identity.key   (Ed25519 private key)
#   .roko/state/daeji-secondary/identity.pub   (Ed25519 public key -- add to peers.json)

# Start the secondary peer (managed by ProcessSupervisor in practice)
cargo run --release --bin kora -- secondary \
  --data-dir .roko/state/daeji-secondary \
  --peers /path/to/peers.json \
  --chain-id 1337
```

The secondary peer exposes a local event stream. Roko subscribes to this stream to
receive `LedgerEvent::SnapshotPersisted` notifications when a block finalizes, instead
of polling the RPC endpoint.

### What Tier 2 Enables in Roko's Subsystems

- **Gate pipeline (verify step)**: Merkle proofs -- prove that key K had value V at
  block N, without trusting any RPC node. The proof is a path in the QMDB Merkle tree,
  verifiable against the certified block's state root. This strengthens the
  ChainWitness gate rung.
- **Learning (CascadeRouter)**: Finalization latency measurement -- know actual
  finalization time in milliseconds. Useful for configuring adaptive gate timeouts.
- **Episodes (write step)**: Offline verification -- replay blocks locally to detect
  validator misbehavior. Episode chain attestations can be verified without network
  access.
- **Knowledge (neuro store)**: Direct state reads -- query QMDB directly without
  JSON-RPC serialization overhead. Relevant when scanning all InsightBoard entries to
  seed a new agent's local knowledge store.
- **Archival**: Store all block data locally. Historical state queries need no external
  node.

---

## Tier 3: Commonware Crate Dependencies

**What it is**: Roko crates add `commonware-*` as direct Cargo dependencies and use
their primitives natively, without going through daeji or the network at all. This is
the deepest integration -- roko adopts the same cryptographic and storage primitives
that daeji is built on.

### What Gets Integrated Directly

| Commonware Crate | Use in Roko | Which Roko Subsystem | Benefit |
|---|---|---|---|
| `commonware-cryptography` | Ed25519 keypair per agent for P2P identity | roko-agent, roko-chain | Native key management; no separate wallet library needed |
| `commonware-cryptography` | BLS12-381 multi-agent attestation (Phase 3) | roko-chain | Threshold signatures for collective agent decisions |
| `commonware-storage::qmdb` | Agent state persistence with Merkle proofs | roko-fs, roko-neuro | Prove what an agent knew at a given point in time; authenticated episode logs |
| `commonware-storage::mmr` | Append-only tamper-evident episode log | roko-learn (EpisodeLogger) | `episodes.jsonl` becomes cryptographically auditable without the chain |
| `commonware-runtime::deterministic` | Agent integration tests | roko-gate | Reproducible tests with a fixed seed; same code runs in production via Tokio |
| `commonware-p2p::authenticated` | Direct agent-to-agent encrypted messaging | roko-agent | Bypass the orchestrator hub; direct P2P between agents in a fleet |
| `commonware-codec` | Wire format for cross-agent messages | roko-agent, roko-chain | Consistent compact serialization across chain and off-chain |
| `commonware-broadcast` | Agent-as-sequencer (Phase 3+) | roko-agent | Each agent broadcasts its own certified event stream |

### Ed25519 Agent Identity

Every roko agent gets an Ed25519 keypair. This key serves as the agent's identity
across all layers: P2P authentication, chain transaction signing (via a derived
secp256k1 key), and knowledge entry attribution.

```rust
use commonware_cryptography::ed25519::{PrivateKey, PublicKey};
use rand::rngs::OsRng;

// Generate once, persist in .roko/state/agent-{name}/identity.key
let private_key = PrivateKey::new(&mut OsRng);
let public_key = private_key.public_key();

// Register on chain -- AgentRegistry stores the Ed25519 pubkey alongside the
// Ethereum address derived from the secp256k1 transaction signing key
```

### QMDB for Authenticated Episode Logs

Currently, episode logs are written to `episodes.jsonl` via the `EpisodeLogger` in
roko-learn -- an append-only file with no cryptographic authentication. Anyone with
file access could modify or delete entries. With `commonware-storage::qmdb`, each
episode entry produces a Merkle root. This enables proofs of the form: "episode E
existed in state root R at time T."

```toml
# Cargo.toml for roko-chain
[dependencies]
commonware-storage = { version = "0.x", features = ["qmdb"] }
```

```rust
use commonware_storage::qmdb::{Database, Config};

// Open (or create) the authenticated episode store
let db = Database::open(Config {
    path: ".roko/state/episodes.qmdb",
    ..Default::default()
})?;

// Write an episode entry
let key = episode_id.as_bytes();
let value = serde_json::to_vec(&episode)?;
let mut batch = db.batch();
batch.put(key, &value);
let root = batch.commit()?;
// `root` is the new Merkle root -- include in the on-chain witness transaction
```

This directly strengthens the Verify step of the universal loop: the ChainWitness gate
rung can now include the QMDB Merkle root in its witness transaction, proving not just
the individual episode hash but the entire episode history up to that point.

### MMR for the Neuro Knowledge Store

The neuro store's 6 knowledge kinds and 4 retention tiers create a rich local knowledge
graph. With `commonware-storage::mmr`, each knowledge entry addition produces an
MMR peak commitment. This creates an auditable append-only history of all knowledge
entries, including their tier progression (Transient -> Working -> Consolidated ->
Persistent). Other agents or auditors can verify the complete knowledge history without
trusting the local filesystem.

### Deterministic Runtime for Agent Tests

Writing integration tests for agent behavior is hard because agents interact with
external services (LLMs, chains, file system) concurrently, and non-deterministic
scheduling can cause flaky tests. The deterministic runtime makes these tests
reproducible:

```rust
use commonware_runtime::deterministic::Executor;

#[test]
fn agent_submits_knowledge_after_task_success() {
    let mut executor = Executor::seeded(42); // fixed seed = deterministic behavior
    executor.start(async {
        // Spin up a simulated daeji network in-process (no real TCP, no real I/O)
        let harness = TestHarness::new(4, 3).await; // 4 validators, threshold 3
        let agent = TestAgent::new(harness.rpc_url()).await;

        agent.run_task("implement feature X").await;

        // Assert knowledge was posted to the simulated chain
        let entries = harness.insight_board_entries().await;
        assert!(entries.iter().any(|e| e.content.contains("feature X")));
    });
}
```

Daeji's `crates/e2e/` crate provides a `TestHarness` that uses
`commonware_p2p::simulated` to run a full validator network in a single process. Adding
it as a dev-dependency in `roko-gate` enables a "ChainSimulate" gate rung that tests
chain interactions in-process before sending them to the real network. This integrates
directly into the Verify step of the universal loop at rung 4 (GeneratedTest) via the
`VerifyChainGate`.

---

## Architecture Diagram

```
  +====================================================================+
  |                        roko agent process                          |
  |                                                                    |
  |  +-------------------+  +------------------+  +------------------+ |
  |  |  orchestrate.rs   |  |  roko-learn      |  |   roko-neuro     | |
  |  |  (plan executor:  |  |  (CascadeRouter: |  |   (knowledge     | |
  |  |   dispatches agent|  |   LinUCB bandit   |  |    store: 6 kinds| |
  |  |   tasks, runs the |  |   model selection;|  |    Insight/30d,  | |
  |  |   7-rung gate     |  |   ExperimentStore:|  |    Heuristic/90d,| |
  |  |   pipeline,       |  |   A/B prompts;   |  |    Warning/1h,   | |
  |  |   records episodes|  |   EpisodeLogger: |  |    AntiKnowledge,| |
  |  |   with chain      |  |   turn recording;|  |    CausalLink/   | |
  |  |   attestation)    |  |   Playbooks:     |  |    60d, Strategy/| |
  |  |                   |  |   reusable        |  |    14d. 4 tiers: | |
  |  |                   |  |   patterns)       |  |    T->W->C->P.  | |
  |  |                   |  |                   |  |    HDC vectors.) | |
  |  +---------+---------+  +--------+---------+  +--------+---------+ |
  |            |                     |                     |           |
  |  +---------v---------------------v---------------------v---------+ |
  |  |                        roko-chain                             | |
  |  |                                                               | |
  |  |  ChainClient trait     ChainWallet trait   ChainWitnessEngine | |
  |  |  (block_number,        (address, balance,  (witness_on_chain, | |
  |  |   get_block_header,     nonce,              verify_on_chain)  | |
  |  |   get_receipt,          sign_and_submit,                      | |
  |  |   get_logs,             wait_for_receipt)  AgentRegistry      | |
  |  |   get_storage_at,                          ReputationRegistry | |
  |  |   eth_call,            AlloyChainWallet    InsightBoard       | |
  |  |   get_balance,         (Alloy JSON-RPC     Marketplace       | |
  |  |   chain_id)             implementation)    KoraiToken         | |
  |  |                                                               | |
  |  |  AlloyChainClient                                             | |
  |  |  (Alloy JSON-RPC)     MockChainClient + MockChainWallet      | |
  |  +----------------------------+----------------------------------+ |
  |                               |                                    |
  |      +------------------------+---------------------------+        |
  |      |  Tier 3                |  Tier 2          Tier 1   |        |
  |      |  (Cargo deps)         |  (secondary      (JSON-   |        |
  |      |                       |   peer process)   RPC)     |        |
  +======|=======================|================|=========|==========+
         |                       |                |         |
  +------v-----------+    +------v------+   +-----v---------v----+
  |  commonware-*    |    |  secondary  |   |    daeji devnet    |
  |  crates used     |    |  peer proc  |   |                    |
  |  directly:       |    |  (P2P       |   |  4 validators      |
  |  - cryptography  |    |   replicat.,|   |  Simplex BFT       |
  |  - storage/qmdb  |    |   QMDB read,|   |  REVM execution    |
  |  - storage/mmr   |    |   no votes) |   |  QMDB state        |
  |  - runtime/det.  |    |  Managed by |   |  chain_id = 1337   |
  |  - p2p           |    |  Process-   |   |  ~400ms blocks     |
  |  - codec         |    |  Supervisor |   |  Managed by        |
  +------------------+    +-------------+   |  ProcessSupervisor |
                                            +--------------------+
```

---

## Roko Components That Need Chain Integration

This section maps every major roko subsystem to its specific chain integration points,
explaining both the code changes needed and where in the universal loop they take effect.

### orchestrate.rs -- Main Execution Loop (All Loop Steps)

This file (`crates/roko-cli/src/orchestrate.rs`) is where tasks are dispatched to LLM
agents, the gate pipeline runs, and episodes are recorded. It is the integration point
for all chain features because it is where the universal loop executes:

1. **Initialization** (before the loop): Instantiate `AlloyChainClient` and
   `AlloyChainWallet` from `roko.toml` config. Optionally, have ProcessSupervisor
   start the daeji devnet (keygen setup, DKG, 4 validators).

2. **Query step** (pre-task dispatch): Query `InsightBoard` via `eth_getLogs` for
   relevant knowledge entries from other agents. Inject them into the neuro store
   context used by the SystemPromptBuilder.

3. **Route step** (model selection): Read `prevrandao` (VRF output) from the latest
   finalized block. Use it as the randomization seed for the CascadeRouter's model
   selection, making the choice verifiable.

4. **Compose step** (prompt assembly): The SystemPromptBuilder includes chain-sourced
   knowledge in Layer 3 (Domain context) and chain status in Layer 3c (Active signals).

5. **Act step** (agent execution): Register agent in `AgentRegistry` if not already
   registered. Send periodic heartbeat transactions.

6. **Verify step** (gate pipeline): After the standard rungs pass, the ChainWitness
   rung anchors the episode hash via `ChainWitnessEngine::witness_on_chain()`.

7. **Write step** (persistence): Episode's `Attestation` struct gains
   `chain_attestation: Some(ChainAttestation { chain_id, tx_hash, block_number })`.
   High-confidence knowledge extractions are posted to `InsightBoard`.

8. **React step** (feedback): Update on-chain reputation scores based on gate outcomes.

### Gate Pipeline -- Verification (Verify Step)

The gate pipeline (`crates/roko-gate/src/`) runs a series of validation checks ("rungs")
after each task. The current 7-rung pipeline and standalone gates are described above.
Two new rungs become available with daeji:

```rust
// At rung 4 (GeneratedTest), VerifyChainGate already exists in the pipeline:
// crates/roko-gate/src/verify_chain_gate.rs

// New ChainWitness capability (post-pipeline):
// After all rungs pass, anchor the episode hash on-chain
let engine = ChainWitnessEngine::new();
let tx_hash = engine.witness_on_chain(&mut attestation, &wallet, &client).await?;

// New ChainSimulate capability (pre-pipeline or at rung 4):
// Spin up an in-process simulated daeji network using commonware deterministic
// runtime, replay chain interactions, verify outcomes before touching real chain
let harness = TestHarness::new(HarnessConfig {
    validators: 4,
    threshold: 3,
    chain_id: 1337,
    seed: blake3::hash(task_id.as_bytes()),
}).await;
// ... deploy contracts, replay transactions, assert outcomes ...
```

### ProcessSupervisor -- Agent Lifecycle (Act Step)

The `ProcessSupervisor` (`crates/roko-runtime/src/process.rs`) manages subprocess
lifecycle for LLM agent processes. With Tier 2 integration, it additionally manages:

- The secondary peer subprocess: start it before the first task, stop it after the last
  task completes, track its health via `kora_nodeStatus` RPC calls.
- Optionally, the daeji validator processes themselves (for programmatic devnet
  bootstrap): 4 `kora validator` processes started before plan execution and stopped
  after.

All child processes are tied to the plan run's `CancelToken`. When the token fires
(plan completes, user interrupts, timeout expires), ProcessSupervisor's
`shutdown_all()` sends SIGTERM to every managed child, waits 5 seconds, then SIGKILL.

### CascadeRouter -- Model Selection (Route Step)

The `CascadeRouter` (`crates/roko-learn/src/`) selects which LLM model to use for each
task based on historical performance data and task category. Configured in
`roko.toml` under `[routing]`:

```toml
[routing]
algorithm = "linucb"            # Contextual bandit algorithm
discount_factor = 0.99          # Reward decay
fast_task_model = "claude-haiku-4-5"
standard_task_model = "claude-sonnet-4-6"
complex_task_model = "claude-opus-4-6"

[routing.weights]
quality = 0.5                   # Weight for output quality
cost = 0.3                      # Weight for token cost
latency = 0.2                   # Weight for response time
```

With daeji:

- **Verifiable routing**: Read `prevrandao` (the VRF output) from the latest finalized
  block; use it as the seed for weighted random model selection. Any observer can verify
  the selection was not biased by the agent operator.

```rust
// In CascadeRouter::select_model()
let block = chain_client.get_block_header(chain_client.block_number().await?).await?;
let vrf_seed = block.mix_hash; // prevrandao = VRF output from threshold consensus
let model_index = hash(vrf_seed, task_id, experiment_id) % models.len();
```

- **Cross-fleet routing data**: Read routing outcomes posted by other agent fleets from
  the on-chain `ReputationRegistry`. Benefit from collective model performance data
  rather than only local history.

### ExperimentStore -- A/B Testing (Route Step)

The `ExperimentStore` (`crates/roko-learn/src/`) runs A/B tests on system prompts by
assigning tasks to variant groups. Currently uses local random number generation. With
daeji, the assignment uses on-chain VRF:

```rust
let seed = chain_client.get_block_header(latest).await?.mix_hash;
let variant = hash(seed, agent_id, experiment_id) % num_variants;
```

Any observer can verify that variant assignment was fair -- the VRF seed is public and
the assignment formula is deterministic. This matters when multiple parties want to
trust that A/B test results were not manipulated.

### EpisodeLogger -- Turn Recording (Write Step)

The `EpisodeLogger` (`crates/roko-learn/src/`) records every agent turn to
`.roko/episodes.jsonl`. Each episode includes: task ID, model used, prompt tokens,
completion tokens, cost in USD, tool calls, and gate verdicts. With daeji:

- Compute `blake3(episode_json)` after each episode completes.
- Anchor the hash on-chain via `ChainWitnessEngine` (the implementation in
  `roko-chain/src/witness.rs` constructs a `TxRequest` to address `0x00...c0` with
  calldata `b"roko.attestation.witness:" + hash`).
- The `Attestation` struct gains a `chain_attestation` field:
  `ChainAttestation { chain_id: u64, tx_hash: [u8; 32], block_number: u64 }`.
- Anyone with the raw episode data can recompute the hash and verify it matches the
  on-chain record using `ChainWitnessEngine::verify_on_chain()`.

### Neuro Store -- Knowledge Persistence (Query + Write Steps)

The neuro store (`crates/roko-neuro/src/`) is an in-process knowledge database. Its
entries are:

- **6 kinds**: Insight (30d half-life), Heuristic (90d), Warning (1h), AntiKnowledge
  (30d), CausalLink (60d), StrategyFragment (14d).
- **4 tiers**: Transient (0.1x weight) -> Working (0.5x, promoted after 2+ uses) ->
  Consolidated (1.0x, promoted after 3+ distinct contexts) -> Persistent (5.0x,
  promoted after 5+ tier passes).
- **Indexed with HDC vectors** (10,240-bit binary) for semantic similarity search.
- **Queried at dispatch time** and injected into the SystemPromptBuilder's Layer 3.

With daeji, the neuro store gains a two-way sync:

**Push (local to chain)**: When a neuro entry reaches `confidence >= 0.70` and has been
locally confirmed 3+ times (confirmation_count >= 3, meaning it was successfully used
in 3+ tasks), promote it to the on-chain `InsightBoard` contract. The half-life is
converted from days to on-chain blocks using the constants in roko-neuro: e.g.,
`INSIGHT_HALF_LIFE_BLOCKS = 7 * 43,200 = 302,400 blocks` (at 2s/block).

**Pull (chain to local)**: Periodically scan `InsightPosted` events from other agents.
Fetch full content from the event data. Store locally with `source: "chain"` and initial
`confidence = 0.5` at `KnowledgeTier::Transient`. Apply normal neuro store admission
gating (the entry still has to prove itself useful locally -- 2+ confirmations to
promote to Working, 3+ distinct contexts for Consolidated).

This creates a flywheel: agents learn locally, promote high-confidence knowledge to the
shared chain, other agents pull that knowledge, use it, confirm it when it helps, it
gains weight on-chain, and more agents pull it.

### SystemPromptBuilder -- Prompt Assembly (Compose Step)

The 9-layer SystemPromptBuilder (`crates/roko-compose/src/system_prompt_builder.rs`)
gains chain-sourced content in two layers:

- **Layer 3 (Domain context)**: On-chain knowledge entries pulled from InsightBoard are
  merged with local neuro store results. The attention bidder framework allocates prompt
  token budget between local and chain-sourced knowledge based on relevance scores.

- **Layer 3c (Active signals)**: Chain health status from `kora_nodeStatus` (current
  view, nullification count, peer count) is included as contextual guidance. If the
  chain is experiencing consensus degradation, the agent can adjust its behavior (e.g.,
  batch multiple witness transactions instead of sending one per task).

### DaimonState -- Affect Engine (Compose Step, Layer 8)

The `DaimonState` (`crates/roko-daimon/`) tracks emotional state and somatic markers
that modulate agent dispatch. With daeji, on-chain reputation scores feed back into
the affect engine: a recent string of gate failures (reflected in ReputationRegistry
scores) increases the caution somatic marker, causing Layer 8 of the SystemPromptBuilder
to inject more conservative guidance.

---

## Integration Priority

### Phase 1 (Tier 1, no blockchain modifications required)

1. Verify daeji devnet is running and accessible at `localhost:8545`.
2. Deploy the existing Solidity contracts (`AgentRegistry`, `InsightBoard`,
   `BountyMarket`) to daeji using Foundry:
   `forge script contracts/script/Deploy.s.sol --rpc-url http://localhost:8545 --broadcast`.
3. Instantiate `AlloyChainClient` and `AlloyChainWallet` in `orchestrate.rs` from
   `roko.toml` config (the `[chain]` section).
4. Wire `ChainWitnessEngine::witness_on_chain()` into the post-task success path in
   `orchestrate.rs` (the Verify/Write steps of the universal loop).
5. Add `AgentRegistry.heartbeat()` call to the `ProcessSupervisor`'s periodic task
   loop (the Act step), using the interval from `relay.heartbeat_interval_secs`.

### Phase 2 (Knowledge layer, still Tier 1)

6. Wire pre-task `eth_getLogs` query against `InsightBoard` into system prompt assembly
   in `orchestrate.rs` (the Query/Score/Compose steps).
7. Wire post-task `InsightBoard.post()` for high-confidence knowledge extractions
   (the Write step, triggered when neuro entry confidence >= 0.70 and
   confirmation_count >= 3).
8. Implement `NeuroChainSync` push/pull loop in `roko-neuro`, converting half-lives
   between days (local) and blocks (on-chain) using the constants:
   `INSIGHT_HALF_LIFE_BLOCKS`, `HEURISTIC_HALF_LIFE_BLOCKS`, etc.
9. Wire `CascadeRouter` to use `prevrandao` for verifiable model selection (the Route
   step).

### Phase 3 (Tier 2 + Tier 3, advanced)

10. Generate secondary peer keypair; add to devnet `peers.json`; start secondary peer
    via `ProcessSupervisor`.
11. Subscribe to finalization events from secondary peer; remove RPC polling for block
    progression.
12. Add `commonware-storage::mmr` to `roko-chain` for authenticated episode log
    (replacing plain JSONL in the EpisodeLogger).
13. Add `ChainSimulate` gate rung using daeji's `TestHarness` library (at rung 4,
    alongside the existing `VerifyChainGate`).
14. Add custom `kora_*` RPC methods to daeji for agent-specific queries.
