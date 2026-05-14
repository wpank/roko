# From Original Vision to Working System: The Agent-Chain Mapping

This document explains where the daeji integration came from, what it provides today, and
what remains to be built. It is written for someone who has never seen any prior design
documents and has no access to any codebase.

---

## Part 1: The Original Vision (Background)

Before daeji existed, a set of approximately 27 design documents (internally called "the
agent-chain docs") specified a custom blockchain purpose-built as a shared knowledge ledger
for AI agents. These documents were written as a speculative design exercise -- they
described a system that did not exist yet.

### The Problem the Design Addressed

AI agents that execute software engineering tasks produce enormous amounts of operational
knowledge during task execution: what works, what fails, which approaches are effective for
which types of problems, which tools produce better results in which contexts. In a typical
setup, this knowledge is **siloed** (trapped inside one agent's memory or log files) and
**ephemeral** (lost when the agent process ends or the conversation resets).

The proposed solution was a blockchain where agents post, confirm, and challenge knowledge
entries, creating a self-curating collective intelligence. The chain would serve as a
permanent, tamper-evident, shared memory that any agent could read from and contribute to.

### Key Concepts from the Original Design

The original documents used a naming scheme that has since been replaced. Here is every
major concept, what it meant, and what it is called today:

**Golem** -- the original term for an AI agent. A golem was any LLM-powered process that
could read code, write code, run tools, and interact with the blockchain. In the current
system, this is simply called a **roko agent**. What a roko agent actually looks like today
is described in detail in Part 3 below.

**Grimoire** -- each agent's private local knowledge store. A grimoire held text entries
indexed by HDC vectors (explained below), with confidence scores and decay rates. In the
current system, this is the **neuro store**, implemented in the `roko-neuro` crate. What the
neuro store actually does today is described in detail in Part 3 below.

**Clade** -- a group of agents under one operator. If a single developer runs five agents
against the same codebase, those five agents form a clade. In the current system, this is
called a **fleet**.

**GNOS** -- a proposed ERC-20 token (an Ethereum-compatible fungible token standard) with
1% annual demurrage. Demurrage means that token balances decay over time -- if you hold
100 GNOS and do nothing, you will have 99 GNOS after one year. The purpose was to
incentivize activity: agents must keep earning tokens to maintain their balance, which
discourages hoarding and encourages participation. Whether to implement a token at all is
an open question (see `07-open-questions.md`).

**InsightEntry** -- a unit of shared knowledge posted to the blockchain. The original design
specified six types, each with different decay rates:

| Type | Description | Default Half-Life |
|---|---|---|
| Insight | A factual observation ("Rust's borrow checker rejects this pattern") | 7 days |
| Heuristic | A behavioral rule ("Always run clippy before committing") | 15 days |
| Warning | An urgent condition ("This API is rate-limited to 10 req/s") | 3 minutes |
| CausalLink | A cause-effect relationship ("Increasing batch size reduces latency by 40%") | 15 days |
| StrategyFragment | A reusable plan ("To migrate a database: 1. backup, 2. schema change, 3. data migration, 4. verify") | 15 days |
| AntiKnowledge | Something that is explicitly wrong ("Do NOT use `unsafe` blocks to bypass the borrow checker for this case -- it causes UB") | 15 days |

Half-life means the entry's weight (importance score) halves over that period. A 7-day
half-life entry posted today will have half its original weight in 7 days, a quarter in 14
days, and so on. This ensures knowledge that is not re-confirmed by subsequent successful
uses gradually fades away, preventing stale information from dominating.

**Superposition Memory (sm_root)** -- a separate Merkle root in each block header over all
knowledge entries. A Merkle root is a single cryptographic hash that commits to an entire
dataset -- you can prove that any specific entry is included (or excluded) from the dataset
by providing a short proof path (typically a few hundred bytes) that chains up to the root.
The sm_root would have let anyone prove "this knowledge entry existed at block N" without
downloading the full dataset. In daeji's implementation, this role is served by the
standard state root that QMDB (Quick Merkle Database, daeji's state storage engine)
already computes for every block.

**HDC (Hyperdimensional Computing)** -- a technique for representing text as very long
binary vectors (10,240 bits = 1,280 bytes per vector). Two pieces of text with similar
meaning produce vectors with similar bit patterns. Similarity is measured by Hamming
distance (counting the number of bits that differ). This is extremely fast because modern
CPUs have a hardware instruction (POPCNT) that counts set bits in a single clock cycle.
With SIMD (Single Instruction, Multiple Data) extensions like AVX-512, a single CPU can
compare a query vector against 100,000 stored vectors in approximately 170 microseconds.
This is how agents find relevant knowledge entries for a given task -- encode the task
description as an HDC vector, then find the stored entries whose vectors are most similar.

**Predictive Foraging (PF)** -- every knowledge retrieval is framed as a falsifiable
prediction registered BEFORE task execution begins. The agent says "I predict that using
knowledge entries A, B, and C will help me complete this task with a score of 0.8 in 45
minutes." After execution, an external verifier (the compiler, the test suite, a linter --
never the LLM itself) determines the actual outcome. The residual (difference between
predicted and actual) calibrates future knowledge retrieval. Entries that consistently
appear in successful predictions gain weight; entries that appear in failed predictions
lose weight. The key principle: no LLM ever grades its own work.

**Three precompiles** -- native chain operations compiled directly into the blockchain node
at fixed EVM (Ethereum Virtual Machine) addresses. The EVM is the runtime environment that
executes smart contract bytecode on Ethereum-compatible chains. A precompile is a piece of
native code (Rust, in daeji's case) that the EVM can call at a hardcoded address, bypassing
the normal interpreted bytecode execution. This matters because some operations (like
searching 100,000 binary vectors) would be astronomically expensive as interpreted bytecode
but are trivially cheap as native code.

The original design specified:
- **GolemRegistry at address 0x08** -- agent identity management (register, heartbeat,
  capability announcement, reputation staking)
- **HDC Search at address 0x09** -- hypervector similarity search over all knowledge entries.
  This cannot be done efficiently in Solidity (the EVM's programming language) because
  10,240-bit binary vector math at scale would cost billions of gas units. As a precompile
  (native Rust code), it takes microseconds.
- **InsightLedger at address 0x0A** -- knowledge entry management (post, confirm, challenge,
  decay computation)

**Stigmergy** -- the coordination model, borrowed from biology (specifically ant colonies).
Ants never communicate directly -- they read and write pheromone trails in their
environment, and the environment's state guides behavior. Similarly, the original design
specified that agents would never communicate directly with each other. Instead, they
read knowledge entries from the chain, use them during task execution, and write new
knowledge entries back. The chain's state -- which entries have high confirmation counts,
which are decaying, which have been challenged -- guides agent behavior without any
agent-to-agent messaging protocol.

**Context Assembly Pipeline** -- a 5-stage process that runs before every task to assemble
relevant knowledge into the agent's system prompt (the instruction text that precedes the
actual task):

1. **Query** -- retrieve 50-200 candidate entries via HDC similarity search against the
   task description
2. **Filter** -- remove entries below minimum confidence/weight thresholds, apply active
   inference (Bayesian filtering based on the agent's current task domain)
3. **Rank** -- score remaining entries by: (HDC similarity * 0.5) + (decay-adjusted weight
   * 0.3) + (poster reputation * 0.2)
4. **Compress** -- select the top entries that fit within a token budget (~800 tokens).
   Apply compression techniques if needed (summarization, deduplication)
5. **Arrange** -- position entries in the system prompt following U-shaped attention
   research: the most important entries go at the very beginning and very end (where LLMs
   pay most attention), while less critical entries go in the middle

---

## Part 2: What Daeji Provides Today

Daeji is the blockchain that implements the base layer described in the original vision. It
is a minimal EVM-compatible chain built entirely from composable Rust primitives provided by
the commonware library (github.com/commonwarexyz/monorepo). It is NOT a fork of any existing
Ethereum client -- it was written from scratch.

"EVM-compatible" means daeji executes standard Ethereum bytecode and exposes the standard
`eth_` JSON-RPC interface, so any Ethereum tooling (Foundry, MetaMask, alloy, ethers-rs)
works against it without modification.

Here is everything daeji provides out of the box today:

| Capability | Implementation | What This Means |
|---|---|---|
| BFT consensus | Simplex algorithm with BLS12-381 threshold signatures | Blocks finalize in ~400ms. Up to 1/3 of validators can be faulty or offline and the chain still makes progress. BFT stands for Byzantine Fault Tolerant -- the system functions correctly even when some participants behave maliciously. BLS12-381 is a pairing-friendly elliptic curve used for threshold signatures, where a group of N signers can collectively produce a signature that any 3rd party can verify using only the 48-byte group public key. |
| EVM execution | REVM (the Rust EVM used by Foundry and Reth) | Any Solidity smart contract can be deployed and executed. Standard EIP-1559 transactions (Ethereum's fee market mechanism where a base fee adjusts per block and users specify a max fee and priority tip). |
| Authenticated state storage | QMDB (Quick Merkle Database) | Every key-value write produces a Merkle root -- a single hash that cryptographically commits to the entire state. This means you can prove any value at any historical block with a short proof (~500 bytes) without downloading the full state. |
| Distributed key generation (DKG) | Both interactive (Joint-Feldman) and trusted-dealer modes | DKG is a protocol where N parties collectively generate a shared cryptographic key without any single party learning the full secret. Joint-Feldman is an interactive protocol where each party contributes randomness and all parties verify each other's contributions -- suitable for production use where parties may not trust each other. Trusted-dealer is a simpler protocol where one party generates the full key and distributes shares to each validator -- faster but requires trusting the dealer. Used for development. |
| Peer-to-peer networking | commonware-p2p authenticated overlay | Every network peer is identified by an Ed25519 public key (Ed25519 is an elliptic curve signature scheme known for speed and small key sizes). All connections are mutually authenticated -- peers prove their identity on connection. There is no unauthenticated path into the network. |
| Secondary (follower) peers | Read-only P2P participants | A node can replicate all block data without participating in consensus voting. Zero consensus risk -- a secondary peer cannot disrupt the chain. Useful for roko agents that need to observe the chain. |
| JSON-RPC interface | Standard `eth_*` namespace + custom `kora_*` extensions | JSON-RPC is the standard API protocol for Ethereum nodes. Standard methods include `eth_sendRawTransaction` (submit signed transactions), `eth_call` (read-only execution), `eth_getLogs` (query event logs). Daeji adds `kora_nodeStatus` for consensus health. |
| Verifiable random numbers (VRF) | Derived from threshold consensus signatures | Every finalized block produces a bias-resistant random number. VRF stands for Verifiable Random Function -- it produces output that is deterministic (same input = same output), unpredictable (cannot guess without the key), and verifiable (anyone can check it was computed correctly). In daeji, the VRF output is the threshold BLS12-381 signature over the view number, stored as the block's `prevrandao`/`mixHash` field. No oracle needed. |
| Devnet tooling | Docker Compose, `just` task runner, load generator | `just trusted-devnet` starts 4 validators + 1 secondary peer locally. `just loadgen` sends test transactions. `just devnet-reset` wipes state and starts fresh. |
| End-to-end test harness | In-process simulated network | Runs a full multi-validator network in a single process with a deterministic random seed. Same code as production, but using a simulated P2P layer instead of real TCP. Given the same seed, every run produces identical behavior. Useful for reproducible tests. |

### Devnet Configuration

The standard local development network runs:
- 4 validator nodes, each holding a BLS12-381 threshold signature share
- 1 secondary (read-only) peer
- Threshold: 3-of-4 (any 3 validators can finalize a block)
- Chain ID: 1337
- Block time: ~400ms

---

## Part 3: Mapping Every Original Concept to Its Current Equivalent

This section maps every concept from the original agent-chain design documents to what
exists today. Because the reader has no prior knowledge of roko, each mapping includes a
detailed description of the current roko implementation -- what it is, what it does, and
how it works.

### What Roko Is

Roko is a self-developing Rust toolkit: 18 crates, approximately 177,000 lines of code. Its
purpose is to orchestrate AI coding assistants (Claude, Codex, Cursor, Ollama, Gemini,
Perplexity, Cerebras, and any OpenAI-compatible API) to execute implementation tasks from
structured plans. The core loop is: read a PRD (Product Requirements Document), generate a
plan of tasks, dispatch agents to execute each task, verify the output through a gate
pipeline, persist the results, and learn from the outcome. Roko's distinguishing property is
that it uses this loop to develop itself -- it reads PRDs describing its own features,
generates plans for implementing them, and executes those plans using LLM agents.

### How the Universal Loop Works

Every operation in roko follows one abstract shape, built from a single data type (the
**Engram**) and six verb traits:

| Trait | Purpose |
|---|---|
| **Store** | Store and query engrams (the substrate -- where data lives) |
| **Score** | Rate engrams along multi-dimensional axes (relevance, quality, cost) |
| **Verify** (Gate) | Verify engrams against ground truth (compiler, tests, linters) |
| **Route** | Select one engram from many candidates (model selection, task routing) |
| **Compose** | Combine engrams into a new engram under a budget (prompt assembly) |
| **React** (Policy) | Watch engram streams and emit new engrams (interventions, replans) |

An **Engram** is the universal data envelope: it wraps any piece of information flowing
through the system with metadata (kind, provenance, timestamp, hash, optional HDC
fingerprint). Every concrete operation -- spawning a coding agent, verifying code compiles,
assembling a system prompt, selecting which LLM to use, retrieving knowledge, posting to
chain -- is one of these six verbs operating on Engrams.

The concrete loop each task follows in the orchestrator (`crates/roko-cli/src/orchestrate.rs`):

```
Read plan -> Score task complexity -> Route to model -> Compose system prompt
   -> Dispatch agent -> Agent executes (tool loop) -> Gate pipeline verifies
   -> Write results to substrate -> React (learn, replan on failure)
```

---

### Golem -> Roko Agent

**Original concept.** A golem was any LLM-powered process that could read code, write code,
run tools, and interact with the blockchain.

**What a roko agent is today.** A roko agent is an LLM-powered subprocess managed by the
`roko-agent` crate. It consists of several layers:

**Dispatcher with 7 LLM backends.** The agent dispatcher supports seven backend types, each
representing a different LLM provider protocol:

| Backend | Protocol | Example Models |
|---|---|---|
| `Claude` | Anthropic `claude` CLI subprocess (stream-json) | Claude Opus, Sonnet, Haiku |
| `Codex` | OpenAI `codex` CLI (JSON-RPC app-server) | Codex |
| `Cursor` | Cursor Agent Client Protocol (ACP JSON-RPC) | Cursor |
| `Ollama` | Local HTTP server (OpenAI-compatible) | Any Ollama model |
| `OpenAi` | Raw OpenAI HTTP API | GPT-4, GPT-4o, o1/o3 |
| `Perplexity` | Perplexity Sonar HTTP API | Sonar Pro, Sonar |
| `Cerebras` | Cerebras Inference API (OpenAI-compatible) | Cerebras models |

Each backend is selected automatically based on the model slug (the model name string like
`"claude-sonnet-4-20250514"` or `"gpt-4o"`) or from the agent role's default. The backend
handles the protocol-specific details of sending prompts, streaming responses, and parsing
tool calls.

**Tool loop with safety.** When an agent receives a task, it enters a tool loop: the LLM
generates a response, which may include tool calls (read a file, write a file, run a shell
command, search code). The `ToolDispatcher` processes each call through a pipeline:

1. **Validate** arguments against the tool's JSON schema
2. **Authorize** the call against the agent's role permissions (an Implementer can write
   files; a Reviewer cannot)
3. **Resolve** the handler (19 built-in tools in `roko-std`, plus dynamic MCP tools)
4. **Execute** with timeout and cancellation support
5. **Truncate** oversized results to 16KB, preserving UTF-8 boundaries

Tools are categorized by concurrency: parallel-safe tools (read, search) run concurrently
via `join_all`; serial tools (shell commands, file writes) run sequentially to avoid
write-write races. An optional safety hook chain runs pre-execution checks, and a result
cache avoids redundant calls for deterministic tools.

**MCP (Model Context Protocol) for external tools.** Beyond the 19 built-in tools, agents
can access external tool servers via MCP. MCP is an open protocol where a tool server
exposes capabilities (read a database, call an API, search the web) that the agent invokes
through a standardized JSON-RPC interface. MCP servers are configured in `roko.toml` under
`agent.mcp_config` and passed to the agent subprocess at spawn time.

**ProcessSupervisor for lifecycle management.** The `roko-runtime` crate provides
`ProcessSupervisor`, which tracks every spawned agent process. It handles:
- Spawning agent subprocesses with the correct environment, model, and MCP configuration
- Monitoring agent health via periodic heartbeats
- Graceful shutdown: when a plan completes or is cancelled, the supervisor sends termination
  signals and waits for agent processes to exit
- Resource tracking: monitoring which agents are active, which have completed, and which
  have failed

The supervisor integrates with the event bus (`roko-runtime::event_bus`), broadcasting
lifecycle events (spawned, completed, failed, timed out) that the TUI, HTTP API, and
learning subsystems can observe.

**28 agent roles.** Each agent is assigned a role from a taxonomy of 28 roles (Implementer,
Reviewer, Planner, Researcher, Debugger, Architect, Tester, and others). Each role carries
defaults for: which backend to use, which model tier to target, a per-turn dollar budget,
and which tool permissions to grant (read-only, read-write, or full execution).

**What agents can do with daeji.** Agents can submit transactions to daeji and read chain
state via the alloy RPC client (`roko-chain/src/alloy_impl.rs`). This client implements
`ChainClient` (read-only: block headers, receipts, logs, storage, eth_call) and
`ChainWallet` (sign and submit transactions).

---

### Grimoire -> Neuro Store

**Original concept.** Each agent's private local knowledge store. A grimoire held text
entries indexed by HDC vectors with confidence scores and decay rates.

**What the neuro store is today.** The neuro store is implemented in the `roko-neuro` crate
as an append-only JSONL file at `.roko/neuro/knowledge.jsonl`. It stores `KnowledgeEntry`
records -- each entry is a single line of JSON containing the knowledge content and rich
metadata.

**Six knowledge kinds with different half-lives.** The neuro store implements the same six
knowledge types from the original design, but with deliberately longer half-lives for local
storage (since local knowledge is not subject to on-chain demurrage competition):

| Kind | What It Captures | Local Half-Life | On-Chain Half-Life |
|---|---|---|---|
| `Insight` | Factual observations ("the borrow checker rejects this pattern") | 30 days | 7 days |
| `Heuristic` | Behavioral rules ("always run clippy before committing") | 90 days | 15 days |
| `Warning` | Urgent, transient conditions ("this API is rate-limited") | 1 hour | 3 minutes |
| `CausalLink` | Cause-effect relationships ("increasing batch size reduces latency") | 60 days | 15 days |
| `StrategyFragment` | Reusable plan fragments | 14 days | 15 days |
| `AntiKnowledge` | Explicitly wrong approaches ("do NOT use unsafe here") | 30 days | 15 days |

**Four retention tiers.** Each entry is assigned to one of four tiers that multiply the
effective half-life:

| Tier | Multiplier | How Entries Get Here |
|---|---|---|
| `Transient` | 0.1x | New entries start here |
| `Working` | 0.5x | 2+ independent confirmations from different episodes |
| `Consolidated` | 1.0x | 3+ confirmations from distinct contexts (different plan/task combos) |
| `Persistent` | 5.0x | Manually promoted or extremely well-confirmed entries |

A `Transient` Insight (30-day base half-life * 0.1 multiplier = effective 3-day half-life)
decays rapidly unless confirmed. A `Persistent` Insight (30 * 5.0 = effective 150-day
half-life) endures for months. This creates natural selection pressure: knowledge that
keeps proving useful in different contexts climbs the tiers and persists; knowledge that
was a one-off observation fades.

**Each entry's full structure.** A `KnowledgeEntry` contains:
- `id`, `kind`, `content`: The core data
- `confidence` (0.0 to 1.0): How reliable the knowledge is
- `confidence_weight`: Signed retrieval weight (can go negative for anti-knowledge)
- `half_life_days`: Base decay rate
- `tier`: Retention tier (Transient/Working/Consolidated/Persistent)
- `hdc_vector`: Optional 1,280-byte HDC fingerprint for similarity search
- `confirmation_count`: How many independent episodes have confirmed this entry
- `distinct_contexts`: Which plan/task combos confirmed it (for tier promotion)
- `source_episodes`: Which episode IDs produced this knowledge
- `source_model`: Which LLM model generated the original supporting episodes
- `model_generality` (0.0 to 1.0): Whether the knowledge is model-specific or general
- `emotional_tag` and `emotional_provenance`: Affect metadata from the Daimon system
  (PAD vectors -- Pleasure/Arousal/Dominance -- that record the emotional context
  under which the knowledge was discovered)
- `balance` (default 1.0): Freshness reserve that decreases via demurrage and increases
  via reinforcement signals (Retrieved, Cited, Gated, Surprised, AgentQuoted)
- `frozen`: Whether the entry is in cold storage (excluded from queries but retaining
  its identity)
- `catalytic_score`: How many new knowledge entries this entry helped create (when the
  average exceeds 1.5, the knowledge network is autocatalytic -- self-sustaining growth)
- `deprecated`: Whether explicitly marked as no longer valid

**How knowledge is queried.** The `KnowledgeStore` implements a scoring pipeline with
context assembly weights (configurable, defaults shown):

| Scoring Component | Default Weight | What It Measures |
|---|---|---|
| HDC similarity | 40% | How semantically close the entry is to the query, measured by Hamming distance on 10,240-bit vectors |
| Keyword/pheromone relevance | 30% | Tag overlap and content keyword matching between query and entry |
| Predictive foraging utility | 20% | Historical usefulness (proxied by `confidence_weight` -- entries that have been useful in past tasks get higher weights) |
| Freshness/recency | 10% | Exponential decay based on age and effective half-life |

Entries also receive a cross-domain diversity bonus (15% boost for entries from a different
domain than the query), and three-tier injection ordering (Warnings and Insights get
priority; CausalLinks and AntiKnowledge are included on demand).

AntiKnowledge entries have special handling during ingestion: new entries are compared
against existing AntiKnowledge via HDC similarity. At similarity > 0.5, a warning is logged.
At > 0.7, the new entry's confidence is halved. At > 0.9, the new entry is rejected
entirely. This prevents the store from accepting knowledge that has been explicitly refuted.

**How knowledge enters the store.** Two primary paths:

1. **Episode distillation.** The `Distiller` in `roko-neuro/src/distiller.rs` batches
   stored episodes (records of what agents did), sends them to a small model (Claude Haiku
   by default), and asks it to extract reusable insights, heuristics, warnings, causal
   links, and strategy fragments. The structured response is normalized into
   `KnowledgeEntry` records and ingested into the store.

2. **Tier progression.** The `tier_progression` module in `roko-neuro` compresses the
   episode log in three stages: D1 (raw episodes to insights, requiring 3+ supporting
   episodes), D2 (insights with 5+ episodes become heuristics), D3 (top heuristics are
   written to a `PLAYBOOK.md`). Each stage uses pattern mining on the episode log to find
   recurring antecedent-consequent sequences.

**How knowledge is stored on disk.** All data lives under `.roko/neuro/`:
- `knowledge.jsonl`: The main knowledge store (append-only JSONL)
- `knowledge-confirmations.jsonl`: Records of when existing entries are independently
  confirmed by new episodes (feeds C-Factor metrics)
- `heuristics.jsonl`: Falsifiable heuristic snapshots from tier progression
- `heuristic-observations.jsonl`: Evidence receipts for or against heuristics

Maintenance operations (decay and garbage collection) atomically rewrite the JSONL file
through a temporary sibling, with a process-wide mutex preventing interleaved rewrites.

**Chain sync is not yet wired.** The neuro store is entirely local today. Pushing
high-confidence entries to daeji's InsightBoard contract, and pulling entries from other
agents' chain posts into the local store, is designed but not yet wired into the runtime.

---

### HDC Vectors: The Similarity Engine

**Original concept.** 10,240-bit binary vectors for semantic similarity search over
knowledge entries.

**What HDC vectors are in roko today.** The `roko-primitives` crate (`crates/roko-primitives/src/hdc.rs`) implements `HdcVector`:

```
pub struct HdcVector {
    bits: [u64; 160],  // 160 * 64 = 10,240 bits = 1,280 bytes
}
```

Three core operations, all pure CPU bit manipulation (no floating point, no GPU):

1. **XOR bind** (`bind`): Combines two vectors. Involution: `bind(bind(a, b), b) == a`.
   Used to create role-filler pairs (binding a "concept" vector to a "property" vector).
2. **Majority-vote bundle** (`BundleAccumulator`): Merges N vectors into one by taking the
   majority bit at each position. The bundled vector is "similar to all inputs" -- it acts
   as a centroid.
3. **Hamming similarity**: Count matching bits between two vectors (via POPCNT hardware
   instruction). Two random vectors have ~50% similarity by chance. Meaningful similarity
   starts at ~55-60%.

Additional infrastructure in `roko-primitives`:
- `Codebook`: Deterministic symbol allocation for encoding text tokens into HDC vectors
- `PatternStore`: Stores and matches learned patterns
- `DecayingBundleAccumulator`: Like BundleAccumulator but older contributions fade
- `ItemMemory`: Named vector storage for retrieval by label

**How HDC vectors are used today.**

1. **Episode fingerprints.** Every agent episode (a record of one task execution) gets an
   HDC fingerprint computed from the prompt/outcome pair
   (`roko-learn/src/hdc_fingerprint.rs`). This fingerprint is stored in the `Episode.hdc_fingerprint` field. It enables finding similar past episodes without full-text
   search: compare the current task's fingerprint against stored episode fingerprints using
   Hamming distance.

2. **Knowledge entry similarity.** When the `hdc` feature is enabled, each `KnowledgeEntry`
   stores an optional `hdc_vector` field (1,280 bytes). The `KnowledgeHdcEncoder` in
   `roko-neuro/src/hdc.rs` encodes entry content into vectors. During queries, the HDC
   similarity score contributes 40% of the total ranking weight.

3. **AntiKnowledge conflict detection.** During ingestion, new entries are compared against
   existing AntiKnowledge entries via HDC similarity. High similarity (> 0.9) means the new
   entry contradicts established anti-knowledge and is rejected.

4. **Pattern discovery.** The `hdc_clustering` module in `roko-learn` uses HDC vectors to
   cluster similar episodes and discover recurring patterns.

**Performance.** Scanning 100,000 vectors takes approximately 170 microseconds on a modern
CPU with AVX-512 SIMD. Each comparison is 160 u64-word XOR + POPCNT operations. The entire
computation fits in L1 cache.

---

### Clade -> Fleet

**Original concept.** A group of agents under one operator.

**What a fleet is today.** A fleet is a configuration concept: all agents under one operator
share a `roko.toml` config file. The config defines: which models are available, which
providers to use, MCP server configurations, learning parameters, and gate thresholds. All
agents spawned by a single `roko plan run` invocation are implicitly in the same fleet.

No on-chain fleet registry exists yet. Fleet membership is purely local.

---

### InsightEntry -> Knowledge Entry (and the Full Learning System)

**Original concept.** A unit of shared knowledge posted to the blockchain, with six types,
decay rates, and confirmation counting.

**What the learning system looks like in roko today.** The learning system spans three
crates (`roko-learn`, `roko-neuro`, `roko-primitives`) and is fully wired into the
orchestrator. It operates in several interconnected subsystems:

**1. Episodes (every agent turn is recorded).** The `EpisodeLogger` in
`roko-learn/src/episode_logger.rs` writes one JSON line per agent turn to
`.roko/episodes.jsonl`. Each `Episode` record contains:
- Agent ID and task ID
- The prompt sent and outcome received
- Tool calls made (name, duration, success/failure)
- Gate verdicts (which gates ran, which passed/failed)
- HDC fingerprint (10,240-bit vector of the prompt/outcome pair)
- Timestamps, model used, token counts
- Optional emotional tag (PAD vector from the Daimon affect engine)
- Arbitrary `extra` metadata (capped at 16KB to prevent runaway growth)

The episode log is append-only and never modified in place. Concurrent writers are
serialized through a process-wide mutex.

**2. Playbooks (patterns extracted from successful episodes).** The `PlaybookStore` in
`roko-learn/src/playbook.rs` stores reusable sequences of actions that have historically led
to success. A `Playbook` has a goal ("Resolve Send+Sync errors"), an ordered list of
`PlaybookStep`s (each with an action kind like `"shell"` or `"edit_file"`, a description,
and expected success signals), and success/failure counters.

Playbooks are extracted from episodes after successful tasks: the tool calls from the
episode become playbook steps. When a new playbook is similar to an existing one (> 80%
step overlap), they merge rather than creating duplicates. Playbooks are queried at dispatch
time and injected into the system prompt (layer 6 of the 9-layer builder -- see below).

**3. CascadeRouter (bandit-based model routing).** The `CascadeRouter` in
`roko-learn/src/cascade_router.rs` decides which LLM model to use for each task. It
progresses through three stages as it accumulates observations:

| Stage | Observations | Strategy |
|---|---|---|
| Static | < 50 | Hardcoded role-to-model table (e.g., Implementer -> Claude Sonnet) |
| Confidence | 50-200 | Empirical pass rates with confidence intervals per model |
| UCB1 | > 200 | Full LinUCB contextual bandit with 8-dimensional feature vectors |

The routing context includes: task category (Implementation, Research, Review, etc.),
complexity band (Simple/Standard/Complex), iteration count (first attempt vs retry), agent
role, crate familiarity score, whether there was a prior failure, and conductor load.

The router persists its state to `.roko/learn/cascade-router.json`: per-model statistics,
stage transitions, confidence stats, and the Pareto frontier (cost-quality tradeoff). It
also tracks per-model latency EMAs and maintains a Pareto frontier to down-weight dominated
models (high cost, low quality) during UCB selection.

**4. Efficiency events (C-Factor).** After each agent turn, an `AgentEfficiencyEvent` is
computed and written to `.roko/learn/efficiency.jsonl`. It includes: token counts, cost
estimate, prompt section attributions (which sections consumed how many tokens), tool call
metadata, and the overall C-Factor summary.

The C-Factor is a composite quality metric measuring: gate pass rate, cost efficiency,
token waste, knowledge integration rate (how often new knowledge confirms existing
entries), and convergence velocity (how fast knowledge is stabilizing).

**5. Prompt experiments (A/B testing).** The `ExperimentStore` at
`.roko/learn/experiments.json` supports A/B experiments across prompt variants, model
choices, or gate thresholds. Each experiment tracks variant assignments, outcome metrics,
and statistical significance.

**6. Adaptive gate thresholds (EMA-based).** Gate pass/fail rates are tracked per rung
(Compile, Lint, Test, etc.) and stored as exponential moving averages in
`.roko/learn/gate-thresholds.json`. When a gate consistently passes, the threshold tightens
slightly. When it consistently fails, the threshold loosens. This creates a self-tuning
quality floor.

**Chain integration status.** Knowledge entries are entirely local today. Posting to
daeji's InsightBoard, pulling from other agents, and cross-agent confirmation counting
are designed but not yet wired.

---

### Context Assembly Pipeline -> 9-Layer System Prompt Builder

**Original concept.** A 5-stage process that assembles relevant knowledge into the agent's
system prompt before each task.

**What the system prompt builder is today.** The `SystemPromptBuilder` in
`roko-compose/src/system_prompt_builder.rs` implements a 9-layer composable prompt
assembly system. Each layer targets a different cache stability tier (so LLM providers
that support prompt caching can reuse stable prefixes):

| Layer | Content | Cache Tier | What It Contains |
|---|---|---|---|
| 1. Role identity | Who the agent is | System (stable) | "You are a Rust implementer specializing in async concurrency..." |
| 2. Conventions | Coding standards | System (semi-stable) | "Use snake_case, thiserror for errors, deny unsafe..." |
| 3. Domain context | Project knowledge | Session (semi-stable) | Project-specific architectural context |
| 3c. Active signals | Pheromone/stigmergic guidance | Session (semi-stable) | Context chunks from active signal monitoring |
| 4. Task context | Current task details | Task (volatile) | The specific task description, file paths, acceptance criteria |
| 4b. Gate feedback | Prior failure digest | Dynamic | "Previous attempt failed: test `test_rate_limiter` panicked..." |
| 5. Tool instructions | Available tools | System (stable) | MCP tools, built-in tools, usage instructions |
| 6. Relevant techniques | Learned playbooks and skills | Task (volatile) | Playbook steps from similar past successes, skill definitions |
| 7. Anti-patterns | What NOT to do | Task (volatile) | "Never call unwrap in library crates", "Never use unsafe to bypass borrow checker" |
| 8. Affect guidance | Emotional tone | Dynamic | PAD-derived focus hints from the Daimon engine (e.g., "approach with caution, prior failure context") |

The builder uses the `PromptComposer` from `roko-compose` to manage token budgets: each
section has a priority (0 = highest, 255 = lowest), and when the total prompt would exceed
the token budget, lower-priority sections are truncated or dropped. A
`SectionEffectivenessRegistry` tracks which sections actually correlate with task success,
allowing learned priority adjustments over time.

**How neuro store knowledge enters the prompt.** At dispatch time, the orchestrator queries
the neuro store for entries relevant to the current task. Matching knowledge entries become
"context chunks" (layer 3c) or "anti-patterns" (layer 7, for AntiKnowledge entries). The
query uses the weighted scoring described above (40% HDC similarity, 30% keyword relevance,
20% utility, 10% freshness). Playbooks from the PlaybookStore are injected at layer 6.

**How the orchestrator builds and uses this.** The orchestrator (`orchestrate.rs`) constructs
a `RoleSystemPromptSpec` for each task dispatch. This spec combines the builder output with
enrichment data: neuro store queries, playbook matches, daimon affect state, gate feedback
from prior failures, and attention hints from context bidders (Neuro, Task, and Research
bidders that compete for prompt space via an attention allocation mechanism).

---

### Stigmergy -> InsightBoard Confirmation Counting

**Original concept.** Agents coordinate through shared environmental state rather than
direct communication.

**What exists today.** The stigmergic pattern is partially realized:

- **Locally**, the neuro store's confirmation counting works: when a knowledge entry is used
  during a task that succeeds, the entry's `confirmation_count` increments and its
  `distinct_contexts` list grows. Entries with more confirmations from more contexts climb
  the tier ladder (Transient -> Working -> Consolidated -> Persistent), gaining longer
  effective half-lives.

- **On-chain**, the InsightBoard contract exists with a `confirm()` method that increments a
  pheromone counter per entry. But agents do not yet automatically call `confirm()` after
  successful tasks -- this is designed but not wired into the agent's task execution loop.

---

### The Gate Pipeline (How Agent Output Is Verified)

This was not a named concept in the original agent-chain documents, but it is central to
understanding how roko validates knowledge before it enters the store.

The `roko-gate` crate implements a 7-rung verification pipeline. Every agent's output passes
through as many rungs as the plan's complexity requires:

| Rung | Index | Gates | What It Checks |
|---|---|---|---|
| Compile | 0 | `CompileGate` | Does the code compile? (`cargo build`) |
| Lint | 1 | `ClippyGate` | Does it pass linting? (`cargo clippy -- -D warnings`) |
| Test | 2 | `TestGate` | Do existing tests pass? (`cargo test`) |
| Symbol | 3 | `SymbolGate` | Do all referenced symbols exist and resolve? |
| GeneratedTest | 4 | `GeneratedTestGate` + `VerifyChainGate` | Do AI-generated tests pass? Is the chain witness valid? |
| PropertyTest | 5 | `PropertyTestGate` + `FactCheckGate` | Do property-based tests pass? Are claimed facts accurate? |
| Integration | 6 | `LlmJudgeGate` + `IntegrationGate` | Does an LLM judge approve? Do integration tests pass? |

Additionally, 6 standalone gates run outside the rung pipeline: `DiffGate` (post-task diff
review), `CodeExecutionGate`, `ShellGate`, `BenchmarkRegressionGate`, `FormatCheckGate`,
and `SecurityScanGate`.

Gate thresholds are adaptive: per-rung exponential moving averages track pass rates. When
a rung consistently passes, the system can raise the bar slightly; when it consistently
fails, the system can relax. This prevents the quality floor from being either too loose
(everything passes trivially) or too strict (nothing ever passes).

Gate verdicts feed back into the learning system: each verdict is recorded in the episode
log, and repeated failures at a specific rung trigger replanning (the
`build_gate_failure_plan_revision` function in `orchestrate.rs` generates a revised task
that addresses the specific failure).

---

### Predictive Foraging -> Prediction Store (Planned)

**Original concept.** Every knowledge retrieval is framed as a falsifiable prediction
registered before task execution.

**What exists today.** The `roko-learn` crate has a `prediction` module and a
`calibration_policy` module that implement the local prediction-publish-correct loop. The
full on-chain `PredictionRegistry` contract is a Phase 2 item.

---

### GolemRegistry Precompile -> AgentRegistry Contract

**Original concept.** Native precompile at address 0x08 for agent identity management.

**What exists today.** The `AgentRegistry.sol` contract in the `contracts/` directory
provides standard Solidity CRUD operations for agent registration. It is deployed as a
normal smart contract, not a precompile -- registry operations (register, heartbeat, lookup)
are simple storage reads and writes that Solidity handles at acceptable gas costs.

**What the contract would connect to in roko.** When wired, the registry would be called:
- At agent startup, to call `AgentRegistry.register()` with the agent's Ed25519 public key
  and capability list
- Every ~15 minutes, to call `AgentRegistry.heartbeat()` (wired into
  `ProcessSupervisor`'s periodic task loop)
- At task dispatch, to look up available agents and their capabilities

---

### HDC Search Precompile -> Off-Chain Neuro Store Search (Phase 3 for On-Chain)

**Original concept.** Precompile at address 0x09 for on-chain hypervector similarity search.

**What exists today.** HDC similarity search runs entirely off-chain in the local neuro
store. The `roko-neuro/src/hdc.rs` module provides `KnowledgeHdcEncoder` for encoding
entries, and the `KnowledgeStore::query()` method performs brute-force Hamming distance
search over all local entries.

An on-chain precompile becomes necessary only when the total knowledge entry count across
all agents exceeds approximately 10,000 entries and off-chain per-agent search is
insufficient because agents need to search entries posted by other agents. Below that
threshold, each agent searching its own local store (which includes entries pulled from
chain via the sync mechanism) is sufficient.

---

### InsightLedger Precompile -> InsightBoard Contract

**Original concept.** Precompile at address 0x0A for knowledge entry management.

**What exists today.** The `InsightBoard.sol` contract exists in the `contracts/` directory.
It is deployed as a normal smart contract. The full 6-type system with half-life decay on
read, confirmation counting, challenge mechanism, and rate limiting is designed but the
deployed contract is simplified.

---

### Styx / iroh P2P -> commonware-p2p

**Original concept.** Various proposed P2P networking layers.

**What exists today.** Daeji uses commonware-p2p, which provides authenticated, encrypted
peer-to-peer networking with Ed25519 identity. This is built into daeji and works today.

---

### Full Concept Mapping Table

| Original Name | Current Name | Where It Lives | Status |
|---|---|---|---|
| Golem | Roko agent | `roko-agent` crate: 7 backends, ToolDispatcher with 19 built-in tools + MCP, safety layer, 28 roles | Working. Agents can submit transactions to daeji and read chain state via the alloy RPC client. |
| Grimoire | Neuro store | `roko-neuro` crate: append-only JSONL at `.roko/neuro/knowledge.jsonl`, 6 kinds, 4 tiers, HDC similarity, demurrage balance, emotional provenance | Working locally. Chain sync designed but not wired. |
| Clade | Fleet | Configuration concept (shared `roko.toml`) | Naming adopted. No on-chain fleet registry yet. |
| GNOS token | DAEJI token (name TBD) | `contracts/` directory: MockERC20 deployed against local test chains | Simple ERC-20. Demurrage not implemented. |
| InsightEntry (6 types) | Knowledge entry | On-chain: `InsightBoard` Solidity contract. Off-chain: neuro store entries with full 6-kind taxonomy, 4-tier progression, confirmation counting, and catalytic scoring. | Contract deployed locally. Off-chain store fully operational with tier progression and HDC search. |
| Superposition Memory (sm_root) | Block state root | QMDB computes Merkle root per block in block header | Natively provided by daeji. |
| HDC vectors + similarity search | Neuro store similarity search + episode fingerprints | `roko-primitives/src/hdc.rs`: 10,240-bit vectors, `roko-neuro/src/hdc.rs`: knowledge encoding, `roko-learn/src/hdc_fingerprint.rs`: episode fingerprinting | Working locally with 40% query weight. On-chain precompile is Phase 3. |
| Predictive Foraging (PF) | Prediction + calibration modules | `roko-learn/src/prediction.rs`, `roko-learn/src/calibration_policy.rs` | Local prediction loop exists. On-chain `PredictionRegistry` is Phase 2. |
| GolemRegistry precompile (0x08) | AgentRegistry contract | `contracts/AgentRegistry.sol`: standard Solidity CRUD | Deployed as contract. Wiring into `ProcessSupervisor` heartbeats is Phase 1. |
| HDC Search precompile (0x09) | Off-chain neuro store search | `roko-neuro` brute-force Hamming search | Working locally. On-chain precompile when entry count exceeds ~10K. |
| InsightLedger precompile (0x0A) | InsightBoard contract | `contracts/InsightBoard.sol` | Deployed as contract. Full 6-type + decay on read is Phase 2. |
| Styx / iroh P2P | commonware-p2p | Built into daeji | Working. |
| Stigmergy (pheromone coordination) | Local confirmation counting + InsightBoard.confirm() | Neuro store `confirmation_count` + `distinct_contexts` (local, working). Contract `confirm()` (exists, not wired into agent loop). | Local stigmergy works. On-chain not yet wired. |
| Context Assembly Pipeline (5 stages) | 9-layer SystemPromptBuilder + ContextAssemblyWeights scoring | `roko-compose/src/system_prompt_builder.rs` (9 layers), `roko-neuro/src/knowledge_store.rs` (scoring weights: 40% HDC, 30% keyword, 20% PF, 10% freshness) | 9-layer builder fully wired. Knowledge query scoring operational with weighted composite. Chain-sourced entries not yet in pipeline. |
| Block-STM parallel execution | (not building) | -- | Premature optimization. |
| Sentinel agents | (not building) | -- | Phase 4+ concern. |

---

## Part 4: What Needs to Be Built on Top of Daeji

Everything below runs on daeji as it exists today (no chain source modifications required)
unless explicitly noted otherwise.

### Layer 1: Smart Contracts (standard Solidity, deployed to daeji's EVM)

These are standard smart contracts written in Solidity (Ethereum's high-level programming
language) and deployed to daeji like to any other EVM chain.

| Contract | What It Does | Original Spec Name | Priority | Status |
|---|---|---|---|---|
| AgentRegistry | Tracks agent identity, capabilities, heartbeats, reputation stake | GolemRegistry | High | Contract exists. Needs deployment to daeji and wiring into orchestrator (ProcessSupervisor heartbeats, dispatch-time capability lookup). |
| InsightBoard | Knowledge entries: post, confirm, challenge, decay. The core of the shared knowledge system. | InsightLedger | High | Simplified contract exists. Needs: 6 entry types, half-life decay on read, rate limiting, challenge mechanism. Must integrate with neuro store's existing tier system. |
| DAEJI Token | ERC-20 fungible token. Optionally with demurrage (balance decay). Used for posting fees, confirmation rewards, challenge stakes. | GNOS | Medium | MockERC20 exists. Demurrage is a separate design decision. |
| PredictionRegistry | Records falsifiable predictions before task execution and outcomes after. Enables cross-agent calibration. | PredictionClaim | Medium | New contract. Would connect to `roko-learn`'s existing prediction and calibration_policy modules. |
| BountyMarket | Cross-agent task exchange with escrow. Agent A posts a task + reward; agent B bids, completes, and collects. | BountyMarket | Low | Contract exists. Not a priority until multiple independent operators exist. |

The original docs specified the first three as precompiles (native chain code at fixed
addresses) for performance. The correct approach: **deploy as contracts first, migrate to
precompiles only if gas costs or latency become measurable bottlenecks.** Contract
deployment requires zero chain modifications and can be tested immediately.

### Layer 2: Custom Precompiles (requires daeji source code modifications)

These are Rust functions compiled into the daeji node binary and registered at fixed EVM
addresses. They bypass the EVM interpreter entirely, running native machine code. This is
necessary when an operation is either impossible in Solidity or prohibitively expensive.

| Precompile | Address | What It Does | Why It Cannot Be a Contract |
|---|---|---|---|
| HDC Search | 0x09 | Binary hypervector similarity search over all active InsightBoard entries | 10,240-bit vector math at scale is impossible in Solidity. Scanning 10K entries would cost billions of gas units. Native Rust with AVX-512 SIMD instructions: ~170 microseconds for 100K entries. |
| QMDB State Proofs | 0x0B | Generate Merkle inclusion/exclusion proofs for any key at any finalized block | Needs direct access to QMDB internals (the Merkle tree structure, historical roots). This data is not accessible from the EVM execution context -- the EVM only sees contract storage, not the underlying database. |
| BTLE Encryption | 0x0C | Encrypt data targeting a future block number; decrypt automatically when that block's VRF output becomes available | BLS12-381 elliptic curve pairing operations cost 100K+ gas in Solidity. Native Rust with optimized curve implementations: microseconds. |

### Layer 3: Off-Chain Agent Infrastructure (roko code changes, no chain modifications)

| Component | What It Does | Where in Roko | What Already Exists |
|---|---|---|---|
| Pre-task chain knowledge query | Before dispatching an agent to a task, scan InsightBoard events for relevant entries from other agents. Merge with local neuro store results. Run the scoring pipeline (40% HDC, 30% keyword, 20% PF, 10% freshness). Inject into the 9-layer system prompt at layers 3c and 7. | `orchestrate.rs`, `roko-compose`, `roko-neuro` | The neuro store query path works locally. The SystemPromptBuilder accepts context chunks and anti-patterns. Only the chain event source is missing. |
| Post-task chain knowledge commit | After a task succeeds, extract learnings from the episode via the Distiller. Post entries meeting promotion criteria (confidence >= 0.70, locally confirmed 3+ times, not a near-duplicate based on HDC similarity > 0.90) to InsightBoard. | `orchestrate.rs`, `roko-neuro` | Episode distillation works. Tier progression works. The chain write path is missing. |
| Neuro-chain sync | Bidirectional: push local entries at Consolidated+ tier to chain; pull chain entries from other agents into local store with initial `confidence = 0.5`, `source: "chain"` tag, and `Transient` tier (requiring local re-confirmation to climb). | `roko-neuro` | The KnowledgeStore supports the `source` tag and tier system. The sync protocol is missing. |
| Prediction calibration loop | Register predictions before tasks (which knowledge was used, expected score, expected duration). Record actual outcomes after gates. Feed residuals back to adjust knowledge retrieval weights. | `roko-learn` | The `prediction` and `calibration_policy` modules exist. The loop connecting them to the orchestrator dispatch path is partially wired. |
| Episode witness anchoring | After each task passes the gate pipeline, compute `blake3(episode_data)` and submit the hash as a transaction to daeji. Creates a tamper-evident record. | `roko-chain` | `ChainWitnessEngine` exists in `roko-chain/src/witness.rs`. Needs wiring into the post-gate success path in `orchestrate.rs`. |
| Agent heartbeats | Periodic transactions to AgentRegistry proving the agent process is alive. One heartbeat every ~15 minutes. | `roko-runtime` | `ProcessSupervisor` already has a periodic task loop. The chain call needs to be added. |

---

## Part 5: Phased Build Plan

### Phase 1: Foundation (enables basic chain interaction)

No daeji source changes required. All work is deploying existing contracts and wiring
existing roko code.

**1. Deploy existing contracts to daeji.** The contracts (AgentRegistry, InsightBoard,
MockERC20) already exist in the `contracts/` directory. Deploy them using Foundry (the
standard Solidity development toolkit):
```
forge script contracts/script/Deploy.s.sol --rpc-url http://localhost:8550 --broadcast
```
Record deployed addresses in `roko.toml` under `[chain.daeji.contracts]`.

**2. Wire the RPC client into the orchestrator.** The Rust RPC client (`AlloyChainClient`)
that talks to daeji over JSON-RPC already exists in `roko-chain/src/alloy_impl.rs`. It
implements `ChainClient` (read-only operations: block headers, receipts, logs, storage,
eth_call) and `ChainWallet` (sign and submit transactions). Currently, nothing instantiates
it. Wire it into `orchestrate.rs` by reading the `[chain.daeji]` config section during plan
runner initialization.

**3. Wire episode witness anchoring.** After each task passes all gate rungs, compute
`blake3(episode_data)` and submit a transaction to daeji containing the hash. The
`ChainWitnessEngine` in `roko-chain/src/witness.rs` already implements this protocol. It
needs to be called from the post-task success path in `orchestrate.rs`.

**4. Wire agent registration and heartbeats.** On agent startup, call
`AgentRegistry.register()` with the agent's Ed25519 public key and capability list.
Every ~15 minutes, call `AgentRegistry.heartbeat()` to prove the agent process is alive.
Wire the heartbeat into `ProcessSupervisor`'s periodic task loop.

**Outcome of Phase 1:** Every completed task leaves a tamper-evident hash on daeji. Agent
identity is on-chain with liveness tracking. The chain client is initialized and available
for subsequent phases.

### Phase 2: Knowledge Layer (enables shared learning across agents)

No daeji source changes required. Extends Phase 1 with knowledge read/write.

**5. Extend InsightBoard contract.** Add the full 6 entry types (matching `KnowledgeKind`
in `roko-neuro`: Insight, Heuristic, Warning, CausalLink, StrategyFragment, AntiKnowledge),
half-life decay computed on read (not stored -- weight is a function of age, half-life, and
confirmation count), confirmation counting with poster rewards, challenge mechanism, and
rate limiting (max 10 posts per 100 blocks per address to prevent spam).

**6. Pre-task knowledge query.** Before dispatching an agent to a task, scan InsightBoard
events via `eth_getLogs` (the standard Ethereum method for querying event logs) for entries
relevant to the task. Merge with local neuro store results using the existing
`ContextAssemblyWeights` scoring (40% HDC, 30% keyword, 20% PF, 10% freshness). Inject
the resulting knowledge pack into the 9-layer system prompt at layers 3c (domain context)
and 7 (anti-patterns for AntiKnowledge entries).

**7. Post-task knowledge commit.** After a task succeeds, extract learnings from the
episode using the existing `Distiller` (which uses Claude Haiku to extract knowledge
candidates). Entries that meet promotion criteria (confidence >= 0.70, locally confirmed
3+ times by appearing in successful tasks, not already on chain, not a near-duplicate of
an existing chain entry based on HDC similarity > 0.90) get posted to InsightBoard.

**8. Neuro-chain sync.** Implement `NeuroChainSync`: push eligible local entries
(Consolidated or Persistent tier) to chain, pull new entries from other agents into local
store with initial `confidence = 0.5`, `source: "chain"` tag, and `Transient` tier. This
creates a flywheel: agents learn locally, promote to chain, other agents pull, use, confirm,
weight increases via local tier progression, more agents pull.

**9. Confirmation flow.** When an agent uses a chain-sourced entry during a task and the
task succeeds (all gate rungs pass), automatically call `InsightBoard.confirm(contentHash)`
to increment the entry's pheromone count and credit the original poster. This closes the
stigmergic loop: agents read chain entries, use them, and their success or failure feeds
back into chain-level entry weights.

**10. Verifiable model routing.** Use daeji's VRF output (the `prevrandao` field from the
latest finalized block) as the seed for weighted random model selection in the
CascadeRouter. Assignment formula: `hash(prevrandao, task_id) % models.len()`. This makes
the CascadeRouter's stage-1 (Static) routing verifiable: any observer can confirm the
model selection was not biased. The router's stage-2 (Confidence) and stage-3 (UCB1) stages
would continue to use local observation data.

**Outcome of Phase 2:** Agents share knowledge through the chain. Knowledge that helps
multiple agents gains weight via confirmations. Knowledge that is not useful decays via
half-life. The system self-curates without central coordination.

### Phase 3: Advanced Features (requires daeji source changes or significant new infrastructure)

**11. Custom kora_ RPC methods.** Add agent-relevant RPC methods to daeji's custom
namespace: `kora_activeAgents` (list registered agents), `kora_recentKnowledge` (entries
posted since a given block), `kora_vrfSeed` (VRF output for a given block). These are
convenience methods that avoid complex log filtering on the client side.

**12. HDC search precompile (0x09).** When knowledge entry count exceeds ~10,000, deploy a
native Rust precompile in daeji for on-chain HDC similarity search. Below that threshold,
off-chain search via the local neuro store is sufficient. The precompile reads from the same
contract state that InsightBoard writes to -- it maintains a separate in-memory index that
is rebuilt per finalized block from InsightBoard events.

**13. QMDB proof precompile (0x0B).** Enable Merkle inclusion/exclusion proofs for any key
at any historical block. Use cases: cross-chain state verification, audit trails ("prove
what the chain knew when agent X made decision Y"), and dispute resolution.

**14. BTLE precompile (0x0C).** Binding Timelock Encryption: an agent encrypts data
targeting a future view number; the ciphertext is posted on-chain; when that view
finalizes, the VRF output becomes the decryption key. Use cases: sealed-bid task auctions,
commit-reveal for multi-agent coordination (prevent agents from copying each other's
approach), tamper-proof governance votes.

**15. Cross-chain certificates.** Daeji produces ~240-byte finality certificates from its
threshold signatures: 48-byte BLS12-381 group public key + 96-byte threshold signature +
metadata. Any external system that knows the 48-byte group key can verify daeji state.
Build: certificate export RPC method, verifier contract for target chains (Ethereum L1 or
others), certificate relay process.

**16. PredictionRegistry contract.** On-chain version of predictive foraging: register
predictions before tasks, record outcomes after. Enables cross-agent calibration data --
all agents can see each other's prediction accuracy. Connects to `roko-learn`'s existing
prediction and calibration_policy modules.

### Phase 4: Economic Layer (if/when needed)

**17. Token with demurrage.** An ERC-20 token where inactive balances decay (1% annually,
computed lazily on transfer). Note: the neuro store's `KnowledgeEntry` already has a
`balance` field with a demurrage model (freshness reserve that decreases over time and
increases via reinforcement signals). The on-chain token demurrage would mirror this
local pattern.

**18. Bounty marketplace.** Cross-agent task exchange. An agent posts a task description +
reward in tokens. Other agents bid. The winning agent completes the task; the contract
verifies via the gate pipeline oracle and releases the reward.

**19. Data marketplace.** Monetize high-confidence knowledge. An agent exports a curated
set of InsightBoard entries as a versioned pack. Other agents pay tokens to receive the
pack. Revenue flows to original posters via on-chain confirmation tracking.

---

## Part 6: What NOT to Build

These are ideas from the original design documents that should be deliberately skipped,
with explanations of why:

**Block-STM parallel execution** -- the original docs specified running EVM transactions in
parallel using Software Transactional Memory (STM), a technique where multiple transactions
execute concurrently and conflicts are detected and resolved automatically. This is how
some high-throughput chains (Aptos, Monad) achieve millions of transactions per second. For
daeji, this is premature optimization. A knowledge ledger for a development team's agents
does not approach Ethereum mainnet transaction volumes. Sequential execution handles current
throughput easily. Adding Block-STM would introduce significant complexity (speculative
execution, conflict detection, rollback logic) for no measurable benefit at this scale.

**Sentinel agents** -- the original docs described adversarial monitoring agents that
continuously probe the system for inconsistencies, test knowledge entries for accuracy, and
challenge suspicious entries, earning rewards for successful challenges. This is valuable
for a public network with untrusted participants where bad actors might post deliberately
misleading knowledge. For a private development network where all agents are controlled by
the same operator, the threat model does not justify the complexity. Revisit if/when the
network opens to external participants.

**OaaS (Orchestration-as-a-Service)** -- decomposing roko's orchestrator into paid MCP
(Model Context Protocol) services that external consumers can call. MCP is a protocol for
tool invocation between AI agents and external services. This is a business model decision,
not a technical architecture decision. The infrastructure (HTTP control plane with ~85
routes on port 6677, per-agent sidecar with 13 routes) already exists. Whether to monetize
it requires a go-to-market decision, not more engineering.

**x402 micropayments** -- a proposed micropayment standard (based on HTTP status code 402
"Payment Required") for paying per-API-call between agents. This requires a live economic
system with real tokens, price discovery, and payment rails. Until the token economics
question is resolved and multiple independent operators exist, this is premature.

**Validator slashing** -- in production proof-of-stake chains, validators who misbehave
(double-signing, extended downtime) lose staked tokens as punishment. The development
network has 4 validators all run by the same operator. Slashing mechanics add complexity
with zero benefit when the operator can simply restart a misbehaving validator. Implement
only when moving to a distributed validator set with independent operators.

**Extended block headers** -- the original docs proposed adding custom fields to the block
header (sm_root, active_agents, insight_count). This requires forking the block format,
which breaks compatibility with standard Ethereum tooling (block explorers, indexers, RPC
clients all expect the standard header format). Instead, track these values in contract
state, which is already Merkle-authenticated by QMDB. Querying a contract costs a single
`eth_call` -- negligible overhead compared to the cost of maintaining a non-standard block
format.

**Inline HDC vectors in EVM state** -- the original docs proposed storing the full 1,280-byte
HDC vector alongside each knowledge entry in the InsightBoard contract's storage. At the
EVM's gas cost for storage writes (~600 gas per byte), each entry's vector alone would cost
~770,000 gas. At daeji's 30M gas block limit, this means at most ~39 new knowledge entries
per block. Instead, keep vectors off-chain: agents compute HDC vectors locally from the
entry content (which is available in the event log) and maintain their own local search
indexes using `roko-neuro`'s `KnowledgeHdcEncoder`. The on-chain contract stores only the
content hash (32 bytes).
