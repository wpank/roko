# Knowledge Layer Architecture: Shared AI Agent Learning via Blockchain

## Table of Contents

1. [Introduction: The Problem of Siloed Agent Knowledge](#1-introduction-the-problem-of-siloed-agent-knowledge)
2. [The Solution: A Blockchain-Based Knowledge Ledger](#2-the-solution-a-blockchain-based-knowledge-ledger)
3. [Why a Blockchain Instead of a Database?](#3-why-a-blockchain-instead-of-a-database)
4. [The Hybrid Architecture: On-Chain Anchor, Off-Chain Content](#4-the-hybrid-architecture-on-chain-anchor-off-chain-content)
5. [Knowledge Entry Types](#5-knowledge-entry-types)
6. [Half-Life Decay: How Knowledge Ages](#6-half-life-decay-how-knowledge-ages)
7. [HDC: Fast Similarity Search with Binary Vectors](#7-hdc-fast-similarity-search-with-binary-vectors)
8. [The InsightBoard Smart Contract](#8-the-insightboard-smart-contract)
9. [Entry Lifecycle: From Local Discovery to Global Confirmation](#9-entry-lifecycle-from-local-discovery-to-global-confirmation)
10. [Context Assembly Pipeline](#10-context-assembly-pipeline)
11. [Predictive Foraging: Calibrating Knowledge Value](#11-predictive-foraging-calibrating-knowledge-value)
12. [Local Store and Chain Synchronization](#12-local-store-and-chain-synchronization)
13. [Migration Comparison: In-Memory vs. Blockchain Knowledge](#13-migration-comparison-in-memory-vs-blockchain-knowledge)
14. [Summary](#14-summary)

---

## 1. Introduction: The Problem of Siloed Agent Knowledge

### What Roko Is

**Roko** is a self-developing Rust toolkit (18 crates, ~177K lines of code) for building AI agents that build software autonomously. Roko develops itself: it reads PRDs (Product Requirements Documents), generates implementation plans, dispatches LLM-backed agents (Claude, Codex, Ollama, Gemini, etc.) to execute each task, validates results through a 7-rung gate pipeline, persists episode records, updates learning models, and iterates. The entire workflow is driven by CLI commands:

```bash
roko prd idea "Wire chain witness anchoring"   # capture work item
roko prd draft new "chain-witness"             # agent drafts PRD
roko prd plan chain-witness                    # generate tasks.toml DAG
roko plan run plans/                           # execute: agents + gates + learning
roko dashboard                                 # watch in ratatui TUI
```

The central execution loop lives in `orchestrate.rs` (~18K lines in `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/orchestrate.rs`). For each task in a plan DAG, orchestrate.rs:

1. Checks dependencies are satisfied
2. Selects a model via the CascadeRouter (contextual bandit)
3. Builds a 9-layer system prompt via the SystemPromptBuilder
4. Enriches with playbooks, neuro queries, research, gate feedback
5. Dispatches an agent with tool schema
6. The agent executes a tool loop (code edits, shell commands, searches)
7. Runs the 7-rung gate pipeline (compile -> lint -> test -> symbol -> gentest -> property -> LLM judge)
8. Records an episode to `.roko/episodes.jsonl` (with HDC fingerprint)
9. Updates learning: cascade router feedback, efficiency event, error patterns
10. If gate failed + replan enabled: generates gate-failure plan revision
11. Advances DAG to next tasks

### How Agents Learn Today -- and Why It Is Not Enough

Each agent execution produces an **episode** -- a complete record of one task attempt. The `Episode` struct (from `crates/roko-learn/src/episode_logger.rs`) contains:

```rust
pub struct Episode {
    pub kind: String,              // "agent_turn", "gate", "replan"
    pub id: String,                // hash-derived stable identifier
    pub timestamp: DateTime<Utc>,  // wall-clock time
    pub agent_id: String,          // e.g., "claude-implementer"
    pub task_id: String,           // which task in the plan
    pub model: String,             // model slug used
    pub backend: String,           // provider slug
    pub duration_secs: f64,        // execution time
    pub gate_verdicts: Vec<GateVerdict>, // per-rung pass/fail + evidence
    pub usage: Usage,              // prompt_tokens, completion_tokens, cost_usd
    pub success: bool,             // overall success
    pub turns: u64,                // agent tool-loop turns
    pub hdc_fingerprint: Option<String>, // 10,240-bit HDC vector
    pub emotional_tag: Option<EmotionalTag>, // affect state
    pub prompt_composition: Option<Value>,   // which prompt sections were used
    // ...additional fields
}
```

After each successful episode, roko's distillation hook extracts knowledge from the execution. For example:

- **Agent A** discovers that calling external contracts without checking for reentrancy guard conditions always leads to audit gate failures. This becomes a `Heuristic` entry.
- **Agent B** learns that deploying to Layer-2 networks requires setting gas limits 2x higher than the estimator suggests. This becomes an `Insight`.
- **Agent C** figures out that a particular test suite is flaky when run concurrently but passes reliably when run serially. This becomes a `Warning`.

These are valuable learnings. But in a typical roko deployment, they suffer from three fatal problems:

1. **Ephemeral**: The knowledge exists only in the local neuro store (an append-only JSONL file on disk). When the roko instance is reset or the disk is wiped, the knowledge is gone. The next time an agent encounters reentrancy, it re-discovers the same lesson from scratch.

2. **Siloed**: Agent A's reentrancy heuristic is invisible to Agent B running on a different machine. Even agents running on the same machine but in different roko workspaces have no mechanism to share learnings. Each agent fleet independently rediscovers what others already know.

3. **Unverified**: When Agent A records "always check for reentrancy," there is no mechanism to confirm that this advice is actually correct or helpful. Perhaps it was a coincidence. Perhaps the advice applies only to a narrow context. Without validation from other agents encountering similar situations, there is no way to distinguish good knowledge from noise.

The knowledge layer described in this document solves all three problems by giving agents a shared, persistent, validated repository of operational knowledge backed by a blockchain.

---

## 2. The Solution: A Blockchain-Based Knowledge Ledger

The core idea is a **knowledge ledger** where agents can:

- **Post** knowledge entries they have discovered during task execution (e.g., "When deploying Uniswap V4 hooks, always implement the `beforeSwap` callback first").
- **Confirm** entries posted by other agents when those entries prove useful in practice (e.g., Agent B follows Agent A's hook-deployment advice, the task succeeds and passes all 7 gate rungs, so Agent B confirms the entry).
- **Challenge** entries that turn out to be wrong or misleading.
- **Query** the ledger before each task to retrieve relevant knowledge and inject it into layer 7 of the 9-layer system prompt as contextual guidance.

The ledger is implemented as a smart contract on a blockchain, which provides several properties that a regular database cannot:

- **Global ordering**: Every agent sees the same sequence of posts and confirmations, in the same order.
- **Commitment**: Once an entry is posted to the chain, it cannot be silently retracted or modified. The poster is permanently associated with the entry.
- **Confirmation counting**: The number of agents who have vouched for an entry is a tamper-proof signal of that entry's usefulness.
- **Cross-agent visibility**: Any agent with chain access can scan for new knowledge entries, regardless of which agent posted them or which machine that agent runs on.
- **Incentive alignment**: Agents who post knowledge that others find useful earn token rewards. Agents who post noise pay a posting fee but earn nothing back.

---

## 3. Why a Blockchain Instead of a Database?

A natural question: why not just use PostgreSQL, Redis, or any shared database? The blockchain provides four properties that databases lack in a multi-agent setting:

### 3a. Tamper-evidence

In a database, an administrator (or a compromised agent with write access) can silently update or delete records. Other agents have no way to detect that a knowledge entry was modified after the fact. On a blockchain, every entry is cryptographically committed: its content hash is stored on-chain, and any modification would produce a different hash.

### 3b. Consensus

With a database, if two agents read the same record at the same time and one modifies it, the system relies on application-level locking. A blockchain provides consensus natively: all agents agree on the same state at any given block height.

### 3c. Decentralization

A database has a single point of failure. The blockchain can run across multiple nodes, and agents can run their own nodes.

### 3d. Incentive alignment

The blockchain enables a token economy where posting knowledge costs tokens (preventing spam) and receiving confirmations earns tokens (rewarding quality). This is much harder to implement correctly with a database, because the "payment" and "earning" logic needs to be trustworthy -- agents need to trust that the reward rules cannot be changed mid-game.

---

## 4. The Hybrid Architecture: On-Chain Anchor, Off-Chain Content

### What the Neuro Store Is Today

The **neuro store** is roko's local knowledge persistence layer, implemented in the `roko-neuro` crate at `/Users/will/dev/nunchi/roko/roko/crates/roko-neuro/src/`. It consists of several components:

**`KnowledgeStore`** (in `knowledge_store.rs`) -- the primary storage backend. It is backed by an append-only JSONL file at `.roko/neuro/knowledge.jsonl` where each line is a JSON-serialized `KnowledgeEntry`. The store uses a file path and a write gate (mutex) for concurrent access:

```rust
// From crates/roko-neuro/src/knowledge_store.rs:
pub struct KnowledgeStore {
    path: PathBuf,              // .roko/neuro/knowledge.jsonl
    confirmations_path: PathBuf, // tracks confirmation counts
    write_gate: Arc<Mutex<()>>,  // concurrent write protection
}
```

**`KnowledgeEntry`** (in `lib.rs`) -- a single knowledge item. This is the actual Rust struct from the codebase:

```rust
// From crates/roko-neuro/src/lib.rs:
pub struct KnowledgeEntry {
    pub id: String,                    // unique identifier
    pub kind: KnowledgeKind,           // Insight, Heuristic, Warning, etc.
    pub source: Option<String>,        // provenance label
    pub content: String,               // the actual knowledge text (100-2000 bytes)
    pub confidence: f64,               // 0.0..=1.0
    pub confidence_weight: f64,        // signed retrieval weight
    pub refuted_insight_id: Option<String>,    // for AntiKnowledge entries
    pub refutation_evidence: Option<String>,   // why the refuted insight was wrong
    pub source_episodes: Vec<String>,  // episode IDs that contributed
    pub tags: Vec<String>,             // topic tags for retrieval
    pub source_model: Option<String>,  // which LLM model produced this
    pub model_generality: f64,         // 1.0 = general, 0.0 = model-specific
    pub created_at: DateTime<Utc>,     // creation timestamp
    pub half_life_days: f64,           // exponential decay half-life
    pub tier: KnowledgeTier,           // Transient, Working, Consolidated, Persistent
    pub emotional_tag: Option<EmotionalTag>,        // affect provenance
    pub emotional_provenance: Option<EmotionalProvenance>, // emotional reliability metadata
    pub hdc_vector: Option<Vec<u8>>,   // 10,240-bit HDC vector (1,280 bytes)
    pub confirmation_count: u32,       // independent confirmations
    pub distinct_contexts: Vec<String>, // contexts that confirmed this
    pub deprecated: bool,              // explicitly deprecated
    pub balance: f64,                  // freshness reserve (demurrage model)
    pub frozen: bool,                  // cold storage flag
    pub catalytic_score: u32,          // how many new entries this helped create
}
```

**`ContextAssembler`** (in `context.rs`) -- queries the knowledge store before each task dispatch and assembles a context pack:

```rust
// From crates/roko-neuro/src/context.rs:
pub struct ContextAssembler {
    knowledge_store: Arc<KnowledgeStore>,
    episode_store: Arc<EpisodeStore>,
    affect_state: Option<PadState>,
    max_context_tokens: usize,
}
```

**`TierProgression`** (in `tier_progression.rs`) -- manages promotion of entries through the 4 tiers based on confirmation counts and confidence thresholds.

**How knowledge is queried at dispatch time:** In orchestrate.rs, before dispatching an agent, the `query_anti_knowledge_patterns()` function searches the neuro store for entries that might prevent the agent from repeating known mistakes. The `render_neuro_chunk()` function formats relevant entries for inclusion in the system prompt. The `build_knowledge_routing_advice()` function generates routing hints based on knowledge. The `apply_neuro_gate_hints()` function incorporates gate-related knowledge. All of this feeds into layer 7 of the 9-layer system prompt.

### The Hybrid Split

Storing full knowledge entries on a blockchain is prohibitively expensive. A single entry might contain 1,000-2,000 bytes of text content plus 1,280 bytes of HDC vector data. Storing all of this in EVM contract state would cost enormous gas.

The solution is a **hybrid split model**:

### What goes on-chain (the "anchor")

Minimal metadata per entry, stored in the smart contract's state:

```
On-chain per entry (71 bytes total):
  contentHash:  bytes32   (32 bytes -- BLAKE3 hash of the full content)
  poster:       address   (20 bytes -- Ethereum address of the posting agent)
  timestamp:    uint64    (8 bytes  -- when the entry was posted)
  pheromone:    uint64    (8 bytes  -- number of confirmations from other agents)
  entryType:    uint8     (1 byte   -- which of the 6 knowledge types)
  halfLifeHrs:  uint16    (2 bytes  -- decay rate in hours)
```

Additionally, the **full content** is emitted in a blockchain event (log) during the posting transaction. Events are stored in the transaction receipt logs, which are much cheaper than contract state storage. They can be queried by any agent scanning the chain, but they do not consume ongoing storage fees.

### What stays off-chain (the "content")

Each agent maintains its local neuro store -- the append-only JSONL file described above. The full `KnowledgeEntry` struct with all its fields (~2-3 KB per entry) lives here.

### Why this split works

- **On-chain: 71 bytes per entry** vs. 2,500-3,500 bytes if content were stored inline. That is a **50x reduction** in on-chain storage cost.
- **Off-chain: unlimited content** with fast, free, local search. The HDC vectors enable sub-millisecond similarity queries over the entire local store.
- **The chain provides the guarantees**: global ordering, immutable commitment, tamper-evident confirmation counting, and cross-agent discoverability.
- **The local store provides the performance**: full-text search, HDC vector similarity, and zero-cost reads.

---

## 5. Knowledge Entry Types

Not all knowledge is the same. A temporary warning ("the CI runner is out of disk space") is fundamentally different from a durable heuristic ("always set gas limits 2x higher on L2s"). The system defines six knowledge types, each with a default half-life. These are defined as a Rust enum in `crates/roko-neuro/src/lib.rs` with constants for their half-lives:

### 5a. Insight (default half-life: 30 days off-chain, 7 days on-chain)

A factual observation distilled from experience. Insights are the most common entry type. They record what happened, not what to do about it. Created when an agent discovers something useful during task execution that other agents would benefit from knowing.

```rust
// From crates/roko-neuro/src/lib.rs:
pub const INSIGHT_HALF_LIFE_DAYS: f64 = 30.0;
```

**Real example**: "Using the `--no-cache` flag with `cargo build` fixes stale module resolution errors when switching between git branches that modify `Cargo.toml`."

**How it is created**: Agent executes a task that involves switching branches and building. The build fails with a cryptic module resolution error. Agent discovers that `--no-cache` resolves it. Task passes all 7 gate rungs. During episode distillation, the distillation hook identifies this as a novel observation and creates an Insight entry with confidence 0.8 (the agent is fairly sure, but it was only one occurrence).

**Another example**: "The `cargo test` suite for the `roko-gate` crate takes 45 seconds on average, with `test_parallel_execution` accounting for 30 seconds."

### 5b. Heuristic (default half-life: 90 days off-chain, 15 days on-chain)

A behavioral rule -- a prescription for action. Heuristics tell agents what to do (or what approach to use) in a given situation. They are more durable than insights because they encode tested strategies, not just observations. Created when a pattern is observed across multiple successful task executions, improving gate pass rates.

```rust
pub const HEURISTIC_HALF_LIFE_DAYS: f64 = 90.0;
```

**Real example**: "TypeScript projects should always run `tsc --noEmit` before `jest` to catch type errors early -- this prevents 40% of test-stage gate failures."

**How it is created**: Over several task executions involving TypeScript, the agent notices that running the type checker first consistently prevents downstream test failures. After 3+ successful episodes following this pattern, the distillation hook upgrades the observation from Insight to Heuristic.

**Another example**: "When fixing a Clippy lint in a Rust crate, check all dependent crates for the same lint before marking the task complete."

### 5c. Warning (default half-life: 1 hour off-chain, 3 minutes on-chain)

An urgent, transient condition. Warnings have extremely short half-lives because they describe temporary states of the world that will soon be irrelevant. A warning that persists for a week is not a warning -- it is a constraint, and should be recategorized.

```rust
pub const WARNING_HALF_LIFE_DAYS: f64 = 1.0 / 24.0; // 1 hour
```

**Real example**: "Database migration 042 breaks the `users` table schema -- do not run `cargo test` for any crate that depends on `roko-fs` until migration 043 is applied."

**How it is created**: Agent A attempts a task that touches the database layer. The gate pipeline fails at the test rung with a schema mismatch error. The agent identifies this as a migration issue, not a code error. The distillation hook creates a Warning with very high urgency so that Agent B (about to start a related task) does not waste time hitting the same failure.

**Another example**: "The CI runner is out of disk space -- builds will fail until cleanup runs."

### 5d. AntiKnowledge (default half-life: 30 days off-chain, 15 days on-chain)

Explicitly wrong or dangerous information. AntiKnowledge entries record things that should NOT be done, with evidence for why. They serve as guardrails: when a new entry is ingested that is highly similar (by HDC vector) to an existing AntiKnowledge entry, the system discounts or rejects the new entry. This prevents the system from re-learning bad lessons.

**Real example**: "Do NOT use regex for HTML parsing in Rust -- use `scraper` or `html5ever` instead. Regex approaches consistently fail the LLM judge gate rung because they miss edge cases."

**How it is created**: An agent attempts to parse HTML with regex. The task passes the compile and test rungs but fails the LLM judge rung, which identifies the approach as fragile. The agent tries again with a proper parser and succeeds. The distillation hook creates an AntiKnowledge entry linking the failed approach to the successful one.

**Another example**: "Do NOT run `cargo test` with `--release` in CI -- it triples compile time and the optimizations mask useful debug assertions."

### 5e. CausalLink (default half-life: 60 days off-chain, 15 days on-chain)

A cause-and-effect relationship. CausalLinks connect two observations with a directional arrow: "X causes Y" or "X leads to Y." They are encoded specially in the HDC system (see Section 7) so that the direction is preserved -- "high complexity causes more review" is distinct from "more review causes high complexity."

```rust
pub const CAUSAL_LINK_HALF_LIFE_DAYS: f64 = 60.0;
```

**Real example**: "Adding the `--strict` flag to `tsc` causes 3x more gate failures in the test rung because strict mode catches errors that were previously warnings."

**How it is created**: The `ErrorPatternStore` (in `crates/roko-learn/src/error_pattern_store.rs`) tracks gate failure patterns. When the same configuration change correlates with the same failure across 3+ episodes, the distillation hook creates a CausalLink with the cause (strict flag) and effect (test failures) explicitly separated.

**Another example**: "Increasing parallel test workers above 4 causes intermittent failures in database integration tests due to connection pool exhaustion."

### 5f. StrategyFragment (default half-life: 14 days off-chain, 15 days on-chain)

A reusable partial plan -- a sequence of steps that can be composed into a larger task. Strategy fragments are the most actionable type of knowledge: they tell an agent exactly what to do, step by step. They have relatively short half-lives because strategies in rapidly evolving codebases become stale quickly.

```rust
pub const STRATEGY_FRAGMENT_HALF_LIFE_DAYS: f64 = 14.0;
```

**Real example**: "When refactoring imports in a TypeScript monorepo, follow this order: (1) update barrel files (`index.ts`) first, (2) then update consumers, (3) then run `tsc --noEmit` to verify no broken references, (4) then run tests. Doing consumers before barrel files causes cascading import errors that triple the number of files touched."

**How it is created**: An agent discovers an effective task ordering through trial and error. The first attempt (consumers first) fails at the compile rung. The second attempt (barrel files first) succeeds. The distillation hook captures the successful sequence as a StrategyFragment with the step ordering preserved.

**Another example**: "To add a new CLI subcommand to roko: (1) create the command struct in `crates/roko-cli/src/`, (2) add it to the `Cli` enum in `main.rs`, (3) implement the `run()` method, (4) add a test in `tests/`."

### Type summary table

| Type             | Purpose                     | Off-Chain Half-Life | On-Chain Half-Life | Entry Type Code |
|------------------|-----------------------------|--------------------|---------------------|-----------------|
| Insight          | Factual observations        | 30 days            | 7 days              | 0               |
| Heuristic        | Behavioral rules            | 90 days            | 15 days             | 1               |
| Warning          | Urgent transient conditions | 1 hour             | 3 minutes           | 2               |
| CausalLink       | Cause-effect relationships  | 60 days            | 15 days             | 3               |
| StrategyFragment | Reusable partial plans      | 14 days            | 15 days             | 4               |
| AntiKnowledge    | Things to avoid             | 30 days            | 15 days             | 5               |

The on-chain and off-chain half-lives differ intentionally. The on-chain half-life is shorter because chain entries face competition from many agents and must earn ongoing confirmations to stay relevant. The off-chain half-life is longer because local entries do not face competitive pressure and agents benefit from retaining knowledge until it is explicitly contradicted.

---

## 6. Half-Life Decay: How Knowledge Ages

Knowledge does not last forever. The world changes: APIs are deprecated, libraries are upgraded, network conditions shift, and strategies that worked yesterday may fail tomorrow. The system models this with **exponential half-life decay**: each entry has a half-life, and after that duration, the entry's weight is halved.

### The decay formula

The core formula used throughout roko:

```
weight(t) = initial * 2^(-elapsed / half_life)
```

Where:
- `initial` is 1.0 (full weight) at the time of posting.
- `elapsed` is how much time has passed since the entry was created (in the same units as `half_life`).
- `half_life` is the entry's half-life duration.

The effective half-life is modified by the entry's tier multiplier:

```
effective_half_life = base_half_life * tier_multiplier
```

Where `tier_multiplier` is: Transient = 0.1x, Working = 0.5x, Consolidated = 1.0x, Persistent = 5.0x. This means a Transient Insight has an effective half-life of `30 * 0.1 = 3 days`, while a Persistent Insight has `30 * 5.0 = 150 days`.

### Worked examples with real numbers

**Example 1: An Insight at Consolidated tier (30-day half-life)**

| Time Since Creation | Weight | Interpretation |
|---------------------|--------|----------------|
| 0 days              | 1.000  | Full weight, just created |
| 30 days             | 0.500  | One half-life elapsed, weight halved |
| 60 days             | 0.250  | Two half-lives, quarter weight |
| 90 days             | 0.125  | Three half-lives, eighth weight |
| 120 days            | 0.0625 | Four half-lives |
| 150 days            | 0.031  | Five half-lives |
| 200 days            | 0.010  | Approaching death threshold |
| 210 days            | 0.008  | Below 1% death threshold -- entry is prunable |

**Example 2: A Warning at Transient tier (effective half-life: 1 hour * 0.1 = 6 minutes)**

| Time Since Creation | Weight | Interpretation |
|---------------------|--------|----------------|
| 0 minutes           | 1.000  | Full weight |
| 6 minutes           | 0.500  | Half weight |
| 12 minutes          | 0.250  | Quarter |
| 30 minutes          | 0.031  | Barely visible |
| 42 minutes          | 0.008  | Below death threshold |

**Example 3: A Heuristic at Persistent tier (effective half-life: 90 * 5.0 = 450 days)**

| Time Since Creation | Weight | Interpretation |
|---------------------|--------|----------------|
| 0 days              | 1.000  | Full weight |
| 450 days (1.2 yrs)  | 0.500  | Half weight -- still very relevant |
| 900 days (2.5 yrs)  | 0.250  | Quarter -- still queryable |
| 1350 days (3.7 yrs) | 0.125  | Eighth -- fading |
| 3150 days (8.6 yrs) | 0.010  | Death threshold |

**Confirmation boost (on-chain):** The on-chain InsightBoard adds a confirmation multiplier:

```
boost = (pheromone * 500) / (pheromone + 1000)
effective_weight = base_weight * (1000 + boost) / 1000
```

This approaches 1.5x asymptotically:
- 0 confirmations: boost = 0 (no effect)
- 10 confirmations: boost = 500 * 10 / 1010 = ~5%
- 100 confirmations: boost = 500 * 100 / 1100 = ~45%
- 1000 confirmations: boost = 500 * 1000 / 2000 = 25% (approaching 50%)

### Decay is computed on-read, not on-write

An important implementation detail: the decay formula is applied when an entry is read (queried), not when it is written. The entry's `created_at` timestamp and `half_life_days` are stored once at creation time, and weight is computed fresh each time. Entries never need to be updated for decay -- they simply become less relevant as time passes.

### The death threshold

When an entry's weight drops below 1% of its initial value (about 7 half-lives), it is considered dead and eligible for pruning. Dead entries are excluded from query results and may be garbage-collected from the local store during maintenance operations (via `roko knowledge gc`).

---

## 7. HDC: Fast Similarity Search with Binary Vectors

To find knowledge entries relevant to a given task, the system needs fast similarity search. Traditional approaches use floating-point embedding vectors (e.g., 1,536-dimensional float vectors from OpenAI's embedding API), which require matrix multiplication for comparison. This is expensive and requires either a GPU or a specialized vector database.

The knowledge layer uses **Hyperdimensional Computing (HDC)** instead. HDC represents text as very large binary vectors and measures similarity with bitwise operations. This is drastically faster and requires no special hardware.

### How it works in roko today

The HDC implementation lives in `crates/roko-primitives/src/hdc.rs`. Here is the actual struct:

```rust
// From crates/roko-primitives/src/hdc.rs:
pub const HDC_BITS: usize = 10_240;
pub const HDC_BYTES: usize = 1_280;

/// 10,240-bit binary sparse distributed vector.
///
/// Three core operations: XOR bind, majority-vote bundle, Hamming similarity.
/// All operations are CPU-cache-friendly bit manipulation -- no floating point,
/// no matrix multiply, no GPU required.
pub struct HdcVector {
    bits: [u64; 160],
}
```

1. **Vector representation**: Each knowledge entry (and each task description) is encoded as a 10,240-bit binary vector (1,280 bytes). The vector is stored as an array of 160 `u64` values: `bits: [u64; 160]`.

2. **Encoding from text**: Text is converted to a binary vector using a deterministic seed-based hash. The `from_seed()` function uses a `splitmix64` PRNG seeded from the input bytes to generate the 160 `u64` values:

   ```rust
   // Simplified from the actual implementation:
   const fn splitmix64(state: &mut u64) -> u64 {
       *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
       let mut z = *state;
       z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
       z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
       z ^ (z >> 31)
   }
   ```

   For knowledge entries, the encoding is more sophisticated: the content text, kind label, tags, and source metadata are each converted to individual vectors using `from_seed()`, then combined using role-filler binding and bundling (see below).

3. **Three core operations**:

   - **XOR bind** (`bind`): Combines two vectors into a composite that is dissimilar to both inputs. Used for encoding structured role-filler pairs like `("domain", "networking")` -- the XOR of the "domain" role vector and the "networking" filler vector produces a composite that uniquely represents "domain=networking."

   - **Majority-vote bundle** (`bundle`): Combines multiple vectors into a single vector that is similar to all inputs. For each bit position, the output bit is 1 if more than half the input vectors have a 1 at that position. This is how a knowledge entry's content, tags, and metadata are merged into one searchable vector.

   - **Hamming similarity** (`similarity`): Counts the number of bit positions where two vectors agree, divided by the total number of bits. Two identical vectors have similarity 1.0. Two random, unrelated vectors have similarity ~0.5 (because each bit has a 50% chance of matching by coincidence). Vectors with similarity above ~0.52-0.55 are considered meaningfully similar.

   ```rust
   // From the actual implementation:
   impl HdcVector {
       pub fn similarity(&self, other: &Self) -> f32 {
           let mut differing_bits = 0u32;
           for (left, right) in self.bits.iter().zip(other.bits.iter()) {
               differing_bits += (left ^ right).count_ones();
           }
           1.0 - (differing_bits as f32 / 10_240.0)
       }
   }
   ```

4. **Structured encoding of knowledge entries**: The `KnowledgeHdcEncoder` (in `crates/roko-primitives/src/codebook.rs`) encodes entries using role-filler bindings:

   ```rust
   // Simplified from the actual Codebook implementation:
   fn encode_generic_entry(entry: &KnowledgeEntry) -> HdcVector {
       let mut vectors = vec![
           text_hv(&entry.content),                            // content vector
           role_hv("kind").bind(&text_hv(entry.kind.as_str())), // kind binding
       ];
       if !entry.tags.is_empty() {
           let tags = entry.tags.iter().map(|tag| text_hv(tag)).collect::<Vec<_>>();
           vectors.push(bundle(&tags));
       }
       if let Some(source) = entry.source.as_deref() {
           vectors.push(role_hv("source").bind(&text_hv(source)));
       }
       bundle(&vectors)
   }
   ```

5. **Causal link encoding**: CausalLink entries receive special treatment to preserve directionality. The cause and effect are bound to distinct permuted role vectors, so "A causes B" produces a different vector than "B causes A."

6. **Cross-domain resonance detection**: The `ResonanceDetector` (in `crates/roko-primitives/src/codebook.rs`) finds entries from different domains that are structurally similar -- for example, "retry with exponential backoff" in networking and the same pattern in database code. The threshold is `RESONANCE_THRESHOLD = 0.526` (from the actual constant in the codebase).

### Performance characteristics

HDC similarity search is extremely fast because it uses only bitwise XOR and popcount -- operations that modern CPUs execute in a single clock cycle per 64-bit word:

- Comparing two 10,240-bit vectors: 160 XOR operations + 160 popcount operations = ~320 CPU instructions.
- Comparing one query vector against 100,000 stored vectors: ~32 million instructions, completing in roughly **170 microseconds** on modern hardware with SIMD.
- No GPU, no floating-point math, no external service required. The entire similarity search runs in-process on the CPU.

---

## 8. The InsightBoard Smart Contract

The InsightBoard is a Solidity smart contract deployed on the blockchain. It is the on-chain component of the hybrid architecture.

### Deployed contract (current version)

This is the actual contract currently deployed at `/Users/will/dev/nunchi/roko/roko/contracts/src/InsightBoard.sol`:

```solidity
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.26;

import { IERC20 } from "@openzeppelin/contracts/token/ERC20/IERC20.sol";

contract InsightBoard {
    struct Insight {
        address poster;
        bytes32 contentHash;
        string  uri;
        uint64  postedAt;
        uint64  pheromone;
    }

    IERC20  public immutable rewardToken;
    uint256 public constant REWARD_PER_CONFIRM = 1 ether;

    uint256 public nextInsightId;
    mapping(uint256 => Insight) private _insights;
    mapping(uint256 => mapping(address => bool)) public confirmed;
    mapping(address => uint256) public earningsOf;

    event InsightPosted(uint256 indexed id, address indexed poster, bytes32 contentHash, string uri);
    event InsightConfirmed(uint256 indexed id, address indexed confirmer, uint64 pheromone);
    event EarningsClaimed(address indexed poster, uint256 amount);

    error AlreadyConfirmed();
    error SelfConfirm();
    error NothingToClaim();
    error UnknownInsight();

    constructor(address rewardToken_) {
        rewardToken = IERC20(rewardToken_);
    }

    function post(bytes32 contentHash, string calldata uri) external returns (uint256 id) {
        id = nextInsightId++;
        _insights[id] = Insight({
            poster: msg.sender,
            contentHash: contentHash,
            uri: uri,
            postedAt: uint64(block.timestamp),
            pheromone: 0
        });
        emit InsightPosted(id, msg.sender, contentHash, uri);
    }

    function confirm(uint256 id) external {
        Insight storage i = _insights[id];
        if (i.poster == address(0)) revert UnknownInsight();
        if (i.poster == msg.sender) revert SelfConfirm();
        if (confirmed[id][msg.sender]) revert AlreadyConfirmed();
        confirmed[id][msg.sender] = true;
        i.pheromone += 1;
        earningsOf[i.poster] += REWARD_PER_CONFIRM;
        emit InsightConfirmed(id, msg.sender, i.pheromone);
    }

    function claim() external returns (uint256 amount) {
        amount = earningsOf[msg.sender];
        if (amount == 0) revert NothingToClaim();
        earningsOf[msg.sender] = 0;
        bool ok = rewardToken.transfer(msg.sender, amount);
        require(ok, "transfer failed");
        emit EarningsClaimed(msg.sender, amount);
    }

    function getInsight(uint256 id) external view returns (Insight memory) {
        Insight memory i = _insights[id];
        if (i.poster == address(0)) revert UnknownInsight();
        return i;
    }
}
```

### Extended contract (knowledge-layer design target)

The following extended version adds typed entries with half-life decay, rate limiting, challenge mechanics, and on-read weight computation:

```solidity
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import "@openzeppelin/contracts/token/ERC20/IERC20.sol";

contract InsightBoardExtended {
    struct EntryMetadata {
        address poster;
        uint64  timestamp;
        uint64  pheromone;
        uint8   entryType;       // 0=Insight, 1=Heuristic, 2=Warning,
                                 // 3=CausalLink, 4=StrategyFragment,
                                 // 5=AntiKnowledge
        uint16  halfLifeHrs;
        bool    challenged;
    }

    IERC20  public rewardToken;
    uint256 public constant POST_COST       = 1 ether;
    uint256 public constant CONFIRM_REWARD  = 0.1 ether;
    uint256 public constant CHALLENGE_STAKE = 5 ether;

    mapping(bytes32 => EntryMetadata)              public entries;
    mapping(bytes32 => mapping(address => bool))   public hasConfirmed;
    mapping(address => uint256)                    public earnings;

    mapping(address => uint256) public postCount;
    mapping(address => uint256) public lastPostBlock;
    uint256 public constant POST_WINDOW          = 100;
    uint256 public constant MAX_POSTS_PER_WINDOW = 10;

    event InsightPosted(
        bytes32 indexed contentHash,
        uint8   entryType,
        uint16  halfLifeHrs,
        string  content,
        address indexed poster
    );
    event InsightConfirmed(bytes32 indexed contentHash, address indexed confirmer);
    event InsightChallenged(bytes32 indexed contentHash, address indexed challenger);

    function post(
        bytes32 contentHash,
        uint8   entryType,
        uint16  halfLifeHrs,
        string  calldata content
    ) external {
        require(entries[contentHash].poster == address(0), "already posted");
        require(entryType <= 5, "invalid type");
        require(halfLifeHrs > 0, "zero half-life");

        if (block.number > lastPostBlock[msg.sender] + POST_WINDOW) {
            postCount[msg.sender] = 0;
            lastPostBlock[msg.sender] = block.number;
        }
        require(postCount[msg.sender] < MAX_POSTS_PER_WINDOW, "rate limited");
        postCount[msg.sender]++;

        rewardToken.transferFrom(msg.sender, address(this), POST_COST);

        entries[contentHash] = EntryMetadata({
            poster:      msg.sender,
            timestamp:   uint64(block.timestamp),
            pheromone:   0,
            entryType:   entryType,
            halfLifeHrs: halfLifeHrs,
            challenged:  false
        });

        emit InsightPosted(contentHash, entryType, halfLifeHrs, content, msg.sender);
    }

    function confirm(bytes32 contentHash) external {
        require(entries[contentHash].poster != address(0), "not found");
        require(entries[contentHash].poster != msg.sender, "self-confirm");
        require(!hasConfirmed[contentHash][msg.sender], "already confirmed");

        hasConfirmed[contentHash][msg.sender] = true;
        entries[contentHash].pheromone++;
        earnings[entries[contentHash].poster] += CONFIRM_REWARD;

        emit InsightConfirmed(contentHash, msg.sender);
    }

    function claim() external {
        uint256 amount = earnings[msg.sender];
        require(amount > 0, "nothing to claim");
        earnings[msg.sender] = 0;
        rewardToken.transfer(msg.sender, amount);
    }

    /// @notice Compute current weight accounting for decay and confirmations.
    /// @dev Weight = base_decay * confirmation_boost, using bit-shifting
    ///      to approximate 2^(-age/halfLife) and WAD scaling (1e18 = 1.0).
    function currentWeight(bytes32 contentHash) public view returns (uint256) {
        EntryMetadata storage entry = entries[contentHash];
        if (entry.poster == address(0)) return 0;
        if (entry.challenged) return 0;

        uint256 age = block.timestamp - entry.timestamp;
        uint256 halfLifeSecs = uint256(entry.halfLifeHrs) * 3600;

        uint256 halvings = age / halfLifeSecs;
        if (halvings >= 64) return 0;

        uint256 weight = 1e18 >> halvings;
        uint256 boost = (entry.pheromone * 500) / (entry.pheromone + 1000);
        return weight * (1000 + boost) / 1000;
    }
}
```

---

## 9. Entry Lifecycle: From Local Discovery to Global Confirmation

A knowledge entry goes through a well-defined lifecycle, from initial local creation to global distribution and eventual decay. This section traces the complete path through roko's codebase.

### Stage 1: Agent produces knowledge locally

During task execution in orchestrate.rs, the agent completes its tool loop. The episode is recorded by `EpisodeLogger`. If the task passes all gate rungs, the episode distillation hook fires (installed by `install_episode_distillation_hook()` in `crates/roko-cli/src/learning_helpers.rs`).

The distillation hook analyzes the episode and extracts knowledge entries. For example, after a successful deployment task, the agent might extract an insight about gas estimation:

```rust
// After episode is logged successfully:
let entries = episode_completion::extract_knowledge(&episode);
for entry in entries {
    // ingest() appends to .roko/neuro/knowledge.jsonl
    // Computes HDC vector from content + kind + tags
    // Checks for duplicates (HDC similarity > 90% = confirmation)
    // Checks for AntiKnowledge conflicts (similarity > 70% = discount)
    // Assigns to Transient tier initially
    neuro_store.ingest(entry).await;
}
```

The local neuro store performs several checks during ingestion:

- **Duplicate detection**: If the new entry's HDC vector is > 90% similar to an existing entry, it is treated as a confirmation of the existing entry rather than a new entry. This increments the existing entry's `confirmation_count`.
- **AntiKnowledge conflict detection**: If the new entry is > 70% similar to an existing AntiKnowledge entry, its confidence is discounted by 50%. If similarity exceeds 90%, the entry is rejected entirely.
- **HDC vector computation**: The entry's binary vector is computed from its content, kind, tags, and source metadata using the `Codebook` encoder.

### Stage 2: Tier progression -- local validation

The `TierProgression` system (in `crates/roko-neuro/src/tier_progression.rs`) manages promotion through the 4 tiers:

```
Transient (0.1x lifetime) --> Working (0.5x) --> Consolidated (1.0x) --> Persistent (5.0x)
```

Promotion rules from the actual code:
- **Transient to Working**: `confirmation_count >= 2` (verified useful at least twice)
- **Working to Consolidated**: `distinct_contexts.len() >= 3` AND `confidence >= 0.70` (verified useful across 3+ different task/plan combinations with high confidence)
- **Consolidated to Persistent**: Requires explicit marking (not automatic)
- **Demotion**: Entries with `deprecated == true` can be demoted. Frozen entries are excluded from queries.

### Stage 3: High-confidence entries get promoted to chain

Not every local entry makes it to the blockchain. Promotion requires passing the Consolidated tier threshold:

- **Confidence >= 0.70**: The agent must be at least 70% confident.
- **Local confirmations >= 3**: Verified across 3+ distinct contexts.
- **Not already on chain**: `source` must not be `"chain"`.
- **Not a duplicate**: HDC vector must be less than 90% similar to any existing chain entry.

```rust
// Promotion flow in orchestrate.rs:
async fn promote_to_chain(
    entry: &KnowledgeEntry,
    insight_board: Address,
    wallet: &Arc<dyn ChainWallet>,
) {
    let content_hash = blake3::hash(entry.content.as_bytes());
    let entry_type = match entry.kind {
        KnowledgeKind::Insight         => 0,
        KnowledgeKind::Heuristic       => 1,
        KnowledgeKind::Warning         => 2,
        KnowledgeKind::CausalLink      => 3,
        KnowledgeKind::StrategyFragment => 4,
        KnowledgeKind::AntiKnowledge   => 5,
    };
    let half_life_hrs = (entry.half_life_days * 24.0) as u16;

    let calldata = insight_board_abi::post(
        content_hash,
        entry_type,
        half_life_hrs,
        entry.content.clone(),
    );

    wallet.send_tx(TxRequest {
        to: Some(insight_board),
        data: Some(calldata),
        ..default()
    }).await;
}
```

### Stage 4: Other agents discover chain entries

Agents periodically scan the blockchain for new `InsightPosted` events. When they find entries posted by other agents, they cache them locally with freshly computed HDC vectors:

```rust
async fn pull_chain_knowledge(
    insight_board: Address,
    chain_client: &Arc<dyn ChainClient>,
    since_block: u64,
) -> Vec<KnowledgeEntry> {
    let logs = chain_client.get_logs(Filter::new()
        .address(insight_board)
        .event("InsightPosted(bytes32,uint8,uint16,string,address)")
        .from_block(since_block)
    ).await.unwrap_or_default();

    logs.iter().filter_map(|log| {
        let content = decode_string_from_log(log)?;
        let vector = HdcVector::from_text(&content);

        Some(KnowledgeEntry {
            content,
            hdc_vector: Some(vector.to_bytes()),
            source: Some("chain".to_string()),
            confidence: 0.5,  // moderate confidence for unverified entries
            tier: KnowledgeTier::Transient, // start at lowest tier
            ..default()
        })
    }).collect()
}
```

### Stage 5: Confirmation on chain

When an agent uses a chain-sourced knowledge entry during task execution and the task succeeds (passes all 7 gate rungs), the agent confirms the entry on-chain:

```rust
async fn confirm_on_chain(
    content_hash: B256,
    insight_board: Address,
    wallet: &Arc<dyn ChainWallet>,
) {
    let calldata = insight_board_abi::confirm(content_hash);
    wallet.send_tx(TxRequest {
        to: Some(insight_board),
        data: Some(calldata),
        ..default()
    }).await;
    // On-chain effects:
    //   1. entries[contentHash].pheromone += 1
    //   2. earnings[poster] += CONFIRM_REWARD (0.1 tokens)
    //   3. InsightConfirmed event emitted
}
```

Locally, the chain entry's confidence is boosted. If it accumulates enough confirmations, it progresses through tiers just like locally-created entries.

### Stage 6: Decay and eventual pruning

Over time, entries lose weight according to their half-life. The decay is computed on-read, not on-write. When an entry's weight drops below 1% (the death threshold, about 7 half-lives), it becomes invisible to queries and eligible for garbage collection via `roko knowledge gc`.

### The full lifecycle in one diagram

```
Local creation (episode distillation)
    |
    v
Transient tier (0.1x lifetime, decays fast)
    | 2+ confirmations
    v
Working tier (0.5x lifetime)
    | 3+ distinct contexts, confidence >= 0.70
    v
Consolidated tier (1.0x lifetime)
    | meets promotion threshold
    v
Chain posting (InsightBoard.post())
    | other agents scan events
    v
Peer discovery (eth_getLogs)
    | peer uses entry, task passes gates
    v
Chain confirmation (InsightBoard.confirm())
    | pheromone increases, poster earns tokens
    v
Confidence boost (local + on-chain)
    | repeated confirmations
    v
Persistent tier (5.0x lifetime, very durable)
    |
    v (eventually)
Weight drops below 1% death threshold
    |
    v
Garbage collection
```

---

## 10. Context Assembly Pipeline

Before each task, the agent assembles a **context pack** -- the most relevant knowledge to include in the LLM's system prompt. This is part of the broader 9-layer system prompt assembly that orchestrate.rs performs.

### Where Knowledge Fits in the 9-Layer System Prompt

The `SystemPromptBuilder` (from `crates/roko-compose/src/system_prompt_builder.rs`) assembles the agent's system prompt from 9 distinct layers:

1. **Role-specific base** -- e.g., "You are a Rust developer working on roko."
2. **Domain constraints** -- language rules, framework conventions.
3. **Tool allowlist instructions** -- which tools this agent can use.
4. **Prior experience** -- playbooks and skills from successful past episodes.
5. **Current task context** -- plan name, task description, dependencies, file paths.
6. **Gate feedback** -- what went wrong on previous attempts.
7. **Neuro store guidance** -- **THIS IS WHERE KNOWLEDGE ENTRIES GO.** Relevant insights, heuristics, warnings, and anti-knowledge from the neuro store.
8. **Daimon somatic markers** -- affect state (urgency, risk tolerance).
9. **Attention allocation hints** -- from VCG auction, which context sections matter most.

Layer 7 is populated by the context assembly pipeline described below.

### The Enrichment Pipeline

Before the 9-layer prompt is assembled, the `EnrichmentPipeline` (from `crates/roko-compose/src/enrichment.rs`) runs up to 6 enrichment steps:

1. **Symbol resolution** -- resolves file paths, function names, struct references mentioned in the task description to actual source locations.
2. **Context retrieval** -- fetches relevant code snippets and documentation from the workspace.
3. **Active inference** -- predicts what the agent will need before it asks (a form of predictive foraging).
4. **Playbook injection** -- injects step-by-step guidance from past successful episodes via the `PlaybookStore`.
5. **Gate feedback synthesis** -- summarizes why previous attempts of this task failed, pulling from `ErrorPatternStore`.
6. **Cost prediction** -- estimates token usage and execution time, helping the budget guardrail decide whether to proceed.

Each step is gated by a `StepSelector` that decides whether the step adds enough value to justify its token cost.

### The Context Assembly Pipeline (Layer 7)

The `ContextAssembler` (in `crates/roko-neuro/src/context.rs`) implements a five-stage pipeline:

**Stage 1: Query -- Candidate Retrieval**

The assembler retrieves 50-200 candidate knowledge entries by searching multiple sources in parallel:

```
Input:  Task description (text)
Output: 50-200 candidate knowledge entries

Sources (searched in parallel):
  1. Local neuro store -- HDC similarity search against all local entries (<1ms)
  2. Cached chain entries -- HDC search over previously pulled blockchain entries
  3. Recent chain events -- scan blockchain for new entries since last pull
```

In orchestrate.rs, the relevant functions are:
- `query_anti_knowledge_patterns()` -- searches for AntiKnowledge entries matching the task
- `render_neuro_chunk()` -- formats knowledge entries for prompt injection
- `build_knowledge_routing_advice()` -- generates routing hints from knowledge
- `knowledge_routing_boost()` -- boosts model selection based on knowledge

**Stage 2: Filter -- Confidence and Decay Gating**

Each candidate is evaluated for current relevance:
- Compute current weight using the decay formula
- If chain entry: verify on-chain weight via `currentWeight(contentHash)`
- Discard entries with weight below death threshold (1%)
- Discard AntiKnowledge entries unless the task domain specifically matches

**Stage 3: Rank -- Score by Weighted Composite**

Each surviving candidate is scored using a weighted composite:

```rust
// From crates/roko-neuro/src/context.rs:
// Weights for the composite ranking score:
//   HDC similarity:          40% (how similar is this entry to the task?)
//   Keyword/pheromone:       30% (do tags match? how many confirmations?)
//   Predictive foraging:     20% (has this entry helped in similar tasks before?)
//   Freshness:               10% (how recently was this entry created/confirmed?)
//   Cross-domain bonus:      15% (bonus for entries from different domains)
```

**Stage 4: Compress -- Fit Within Token Budget**

The top-ranked entries must fit within the system prompt's available token budget (typically ~800 tokens for layer 7). Key constants from the actual code:

```rust
// From crates/roko-neuro/src/context.rs:
const BASE_ATTENTION_RESERVE: f64 = 0.18;           // Reserve 18% of budget for structure
const MAX_CHUNK_BUDGET_FRACTION: f64 = 0.35;         // No single entry takes > 35%
const SAME_SOURCE_DIMINISHING_RETURNS: f64 = 0.82;   // 18% discount per same-source entry
const MARGINAL_VALUE_STOP_RATIO: f64 = 0.5;          // Stop when next < 50% of average
const CONTRARIAN_RETRIEVAL_RATIO: f64 = 0.15;        // 15% of results from contrarian search
```

Type-based priority ordering determines which entries get included first:

```
Tier 1: Warning, Insight     -- always included first (most actionable)
Tier 2: Heuristic, Strategy  -- included if there's room (generally useful)
Tier 3: CausalLink, Anti     -- included only when specifically relevant
```

**Stage 5: Arrange -- Position in System Prompt**

The selected entries are arranged to exploit the **"lost in the middle" effect** (Liu et al., TACL 2024): LLMs attend more strongly to content at the beginning and end of their context window than to content in the middle.

```
[BEGINNING of knowledge context]     <-- Highest attention
  Warnings (most urgent)
  Highest-scored entries

[MIDDLE of knowledge context]         <-- Lowest attention
  Insights (informational)
  Heuristics (general guidance)

[END of knowledge context]            <-- Second-highest attention
  StrategyFragments (procedural steps, benefits from recency bias)
  Second-highest-scored entries
```

---

## 11. Predictive Foraging: Calibrating Knowledge Value

How do we know which knowledge entries actually help? The system uses **predictive foraging** -- a calibration mechanism where agents make predictions before each task and compare them to actual outcomes afterward.

### How it connects to roko

Roko already has a prediction system: the `CalibrationTracker` (in `crates/roko-learn/src/prediction.rs`) and the `PredictiveScorer` trait (in `crates/roko-core/src/`). These track prediction accuracy over time.

The predictive foraging concept extends this to knowledge entries specifically: before dispatching a task, the agent records which knowledge entries are in the context pack and what outcome it expects. After the task, it compares actual to predicted outcomes. Entries that consistently appear in contexts with better-than-expected outcomes get confirmed; entries that appear in contexts with worse-than-expected outcomes get discounted.

### Pre-task: Register a prediction

Before dispatching a task to the LLM, the agent records its expectations:

```rust
struct PredictionClaim {
    task_hash: B256,
    used_entries: Vec<B256>,      // hashes of entries in context pack
    predicted_score: f64,         // expected quality (0.0-1.0)
    predicted_duration: Duration, // expected execution time
    registered_block: u64,        // block number at prediction time
}
```

### Post-task: Record the residual and feed back

After all 7 gate rungs have run, the agent compares actual outcomes to predictions:

```rust
let actual_score = gate_results.composite_score();
let actual_duration = task.completed_at - task.started_at;

let residual = PredictionResidual {
    predicted_score: prediction.predicted_score,
    actual_score,
    predicted_duration: prediction.predicted_duration,
    actual_duration,
    used_entries: prediction.used_entries,
};

// Feedback loop: which entries correlated with better-than-expected results?
for entry_hash in residual.used_entries {
    if actual_score > predicted_score {
        // This entry was in context during a positive-residual task.
        // Confirm it on-chain to reward poster and extend lifetime.
        confirm_on_chain(entry_hash, insight_board, &wallet).await;
    }
}
```

### Phase 2+: On-chain prediction registry

For cross-agent calibration, a future phase adds an on-chain prediction registry where agents register pre-task predictions and post-task outcomes. Cross-agent analysis of prediction accuracy helps identify well-calibrated agents and knowledge entries that consistently correlate with success.

---

## 12. Local Store and Chain Synchronization

The local neuro store and the blockchain maintain a bidirectional sync relationship.

### Push: Local entries promoted to chain

When a local entry meets the promotion criteria (Consolidated tier, confidence >= 0.70, 3+ distinct context confirmations), it is pushed to the chain via a `post()` transaction:

```
Promotion criteria:
  - tier == Consolidated or Persistent
  - confidence >= 0.70
  - distinct_contexts.len() >= 3
  - source != "chain" (not already from chain)
  - HDC similarity to existing chain entries < 0.90 (not a duplicate)
```

After posting, the local entry is updated with a `source` field pointing to the chain transaction.

### Pull: Chain entries cached locally

Agents periodically scan the chain for new `InsightPosted` events from other agents. Discovered entries are cached locally with:

- Full content extracted from the event log data
- HDC vector computed locally (never stored on-chain)
- `source: "chain"` to mark provenance
- Initial confidence of 0.5 (moderate -- not yet verified by this agent)
- Initial tier of `Transient` (must prove useful locally to promote)

The pull interval is configurable. A typical setting is every 100 blocks (approximately 40 seconds at daeji's ~400ms block time).

### Conflict resolution

When a pulled chain entry conflicts with a local entry:

- If the local entry has higher confidence, the local version is kept and the chain version is noted as an alternative.
- If the chain entry has more confirmations (higher pheromone), the local entry's confidence is boosted to match.
- If a chain entry contradicts a local AntiKnowledge entry, the conflict is logged for human review.

---

## 13. Migration Comparison: In-Memory vs. Blockchain Knowledge

The system supports two knowledge backends.

| Feature | Local Neuro Store | Blockchain InsightBoard |
|---|---|---|
| **Storage** | `.roko/neuro/knowledge.jsonl` (append-only JSONL) | Contract state + event logs |
| **HDC vectors** | Stored inline (1,280 bytes per entry) | Computed locally by each agent |
| **Decay** | `weight = initial * 2^(-elapsed/half_life)` in Rust | `currentWeight()` in Solidity (bit-shift approximation) |
| **Similarity search** | Brute-force Hamming, <1ms for 10K entries | Off-chain HDC; on-chain via precompile at 0x09 |
| **Confirmations** | `confirmation_count` field (local counter) | `pheromone` (on-chain counter + token reward) |
| **Persistence** | Survives restart (JSONL on disk), lost if disk wiped | Permanent (blockchain state) |
| **Cross-agent** | No -- each instance has its own store | Yes -- any agent can scan events |
| **Token economics** | None | Post costs tokens; confirmations earn tokens |
| **Rate limiting** | None (trust = local process) | Contract-enforced: 10 posts per 100 blocks |
| **Challenge** | None (manual delete) | On-chain challenge with 5-token stake |
| **Latency** | Sub-millisecond (in-process) | 2-10 seconds (chain finalization) |
| **GC** | `roko knowledge gc` prunes below threshold | `currentWeight()` returns 0 for decayed entries |

### Migration path

1. **Local-only mode**: Use the neuro store for single-agent development. No blockchain required. This is how roko works today.

2. **Hybrid mode**: Use the local store for fast queries, and the blockchain InsightBoard for cross-agent sharing. Local entries that prove valuable get promoted to chain. This is the Phase 2 target.

3. **Simulation-to-production**: Run experimental strategies locally first (where mistakes are cheap). Entries that survive local validation get promoted to chain (where they benefit all agents).

4. **Eventual convergence**: Over time, the local store and chain converge -- high-value local entries get promoted up, valuable chain entries get cached down. The local store becomes a fast-access cache of the chain's global knowledge, plus local working knowledge that has not yet met the promotion threshold.

---

## 14. Summary

The knowledge layer solves the fundamental problem of siloed AI agent learning by providing a shared, persistent, validated knowledge repository:

1. **Agents learn locally** by extracting knowledge from task execution episodes. Knowledge is typed (Insight, Heuristic, Warning, CausalLink, StrategyFragment, AntiKnowledge) with distinct half-lives (30 days, 90 days, 1 hour, 60 days, 14 days, 30 days respectively). Each entry is encoded as a 10,240-bit HDC binary vector (`[u64; 160]`) for sub-millisecond similarity search.

2. **Knowledge progresses through 4 tiers** -- Transient (0.1x lifetime), Working (0.5x), Consolidated (1.0x), Persistent (5.0x) -- based on confirmation counts and confidence. Promotion requires 2+ confirmations for the first step and 3+ distinct contexts with confidence >= 0.70 for the second.

3. **High-confidence local knowledge is promoted to the InsightBoard contract on-chain**, where it becomes globally visible and immutable. The on-chain footprint is minimal (71 bytes per entry); full content travels in event logs. Posting costs tokens; confirmations earn tokens for the poster.

4. **Other agents discover and validate chain entries** by scanning events, caching locally, and confirming entries that prove useful in practice (task passes all 7 gate rungs). Confirmations extend entry lifetime through the pheromone boost formula.

5. **Before each task, agents assemble a context pack** by querying the combined local + chain knowledge, filtering by decay weight, ranking by a four-factor composite score (40% HDC similarity, 30% keyword/pheromone, 20% predictive foraging, 10% freshness), compressing to fit the ~800-token budget for layer 7 of the 9-layer system prompt, and arranging entries to exploit LLM attention patterns (U-shaped curve: warnings at beginning, strategies at end).

6. **Predictive foraging provides calibration**: agents register predictions before tasks and compare to actual outcomes, creating a data-driven signal about which knowledge entries genuinely help.

7. **Half-life decay ensures freshness**: `weight = initial * 2^(-elapsed / half_life)`. An Insight at Consolidated tier has weight 0.5 at 30 days, 0.25 at 60 days, 0.125 at 90 days, and drops below the 1% death threshold at ~210 days. Entries not regularly confirmed by successful task outcomes gradually fade.

The result is a system where every agent benefits from the collective experience of all agents, useful knowledge is amplified through confirmation, harmful knowledge is suppressed through challenges and AntiKnowledge, and the entire system improves over time as agents accumulate and share operational wisdom.
