# Implementation Guide: Wiring Roko Agents to Daeji

## Purpose of This Document

This is a step-by-step implementation guide for connecting roko's agent execution loop to a local daeji blockchain instance. It assumes you have never seen either codebase. Every concept, file, struct, and code path is explained from scratch.

---

## Part 1: What Exists Today

### What Roko Is

**Roko** is a self-developing Rust toolkit (18 crates, ~177K lines of code) for building AI agents that build software autonomously. Roko develops itself: it reads PRDs (Product Requirements Documents), generates implementation plans, executes tasks via LLM-backed agents, validates results through a gate pipeline, persists episode records, updates its learning models, and iterates. The entire workflow is driven by CLI commands:

```bash
roko prd idea "Wire chain witness anchoring"   # capture work item
roko prd draft new "chain-witness"             # agent drafts PRD
roko prd plan chain-witness                    # generate tasks.toml from PRD
roko plan run plans/                           # execute: agents + gates + learning
roko dashboard                                 # watch progress in ratatui TUI
```

### The orchestrate.rs Execution Loop In Detail

The central component is `orchestrate.rs` -- the main execution loop located at `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/orchestrate.rs`. This file is ~18,000 lines long and connects the CLI to every subsystem. Here is what happens for each task in the plan DAG, step by step:

**Step 1: Read Plan and Check Dependencies.**
orchestrate.rs reads a `tasks.toml` file containing a directed acyclic graph (DAG) of implementation tasks. The `ParallelExecutor` (from `roko-orchestrator`) manages the DAG state machine. It returns `ExecutorAction` values -- pure data describing what to do next. orchestrate.rs dispatches those actions to real agents, gates, and git, then feeds results back as `ExecutorEvent` values. Before dispatching a task, the executor checks that all upstream dependencies (other tasks this one depends on) have completed successfully. Tasks with no unmet dependencies can run in parallel.

**Step 2: Select Model via CascadeRouter.**
The `CascadeRouter` (from `roko-learn`) is a contextual bandit that selects which LLM model to use for each task. It considers the task's category (e.g., `Refactor`, `Implement`, `Test`, `Debug`), complexity band (e.g., `Simple`, `Medium`, `Complex`), domain (e.g., `Code`, `Chain`, `Docs`), and historical performance data (which models succeeded on similar tasks). The router persists its state to `.roko/learn/cascade-router.json` and learns from every task outcome. The function `resolve_effective_model()` in orchestrate.rs calls into the router and may also apply A/B experiment overrides from the `ModelExperimentStore`.

**Step 3: Build 9-Layer System Prompt via SystemPromptBuilder.**
The `SystemPromptBuilder` (from `roko-compose`) assembles the agent's system prompt from 9 distinct layers, each contributing context from a different subsystem:

1. **Role-specific base** -- e.g., "You are a Rust developer working on roko. Your role is Implementer." Generated from `RoleSystemPromptSpec` using templates in `crates/roko-compose/src/templates/`.
2. **Domain constraints** -- language rules, framework conventions, project-specific patterns.
3. **Tool allowlist instructions** -- which tools this agent can use (file edit, shell, search, etc.). Built by `claude_tool_allowlist_with()`.
4. **Prior experience** -- playbooks and skills from successful past episodes. Loaded from `PlaybookStore` and `SkillLibrary` by `build_task_playbook()` and `render_prior_experience()`.
5. **Current task context** -- the plan name, task description, dependencies, relevant file paths, prior task outputs. Built by `code_context_for_task()` and `load_prior_task_outputs()`.
6. **Gate feedback** -- what went wrong on previous attempts of this same task (error messages, failing tests, lint violations). Built by `with_task_failure_context()` using error pattern data from `ErrorPatternStore`.
7. **Neuro store guidance** -- relevant insights, heuristics, warnings, and anti-knowledge from the knowledge store. Queried by `query_anti_knowledge_patterns()` and `render_neuro_chunk()`, injected by `apply_neuro_gate_hints()`.
8. **Daimon somatic markers** -- the affect engine's current state (urgency level, risk tolerance, exploration vs. exploitation bias). Loaded from `DaimonState` at `.roko/daimon/affect.json`, built by `build_daimon_context_section()`.
9. **Attention allocation hints** -- from the VCG auction, signals about which context sections matter most for this task. Managed by `AttentionBidder` variants (`NeuroBidder`, `TaskBidder`, `ResearchBidder`).

**Step 4: Enrich with Playbooks, Neuro, Research, and Gate Feedback.**
The `EnrichmentPipeline` (from `roko-compose`) runs up to 6 enrichment steps in order:
- Symbol resolution (resolve file paths, function names, struct references)
- Context retrieval (fetch relevant code snippets, documentation)
- Active inference (predict what the agent will need before it asks)
- Playbook injection (inject step-by-step guidance from past successes)
- Gate feedback synthesis (summarize why previous attempts failed)
- Cost prediction (estimate token usage and execution time)

Each step is gated by a `StepSelector` that decides whether the step adds enough value to justify its cost. The pipeline is configured by `EnrichmentConfig`.

**Step 5: Dispatch Agent.**
The agent is dispatched via `dispatch_agent_with()`. This function:
- Creates an `AgentInvocationSession` from the `MultiAgentPool`
- Passes the assembled system prompt, tool schemas, and MCP configuration
- Supports multiple backends: Claude CLI, Claude API, Ollama, Codex, Cursor, Gemini, Perplexity, and OpenAI-compatible endpoints
- The agent runs a tool loop: it generates code edits, shell commands, file searches, etc. and executes them via roko's tool system
- The `SafetyLayer` applies role-based authorization, pre/post checks, and scrubs secrets from outputs
- The `ProcessSupervisor` (from `roko-runtime`) tracks the agent subprocess lifecycle

**Step 6: Agent Executes Tool Loop.**
The dispatched agent (an LLM subprocess) reads the system prompt, generates actions (code edits, shell commands, file searches), and executes them. The tool loop continues until the agent declares completion or hits a budget limit. Actions are tracked by the `CustodyLogger` for provenance auditing.

**Step 7: Run 7-Rung Gate Pipeline.**
After the agent finishes, orchestrate.rs runs the gate pipeline. The `select_rungs()` function (from `roko-gate`) determines which rungs to execute based on task domain and complexity. The 7 rungs, in order:

1. **Compile** (`CompileGate`) -- `cargo build` / `cargo check`. Must pass for code tasks.
2. **Clippy** (`ClippyGate`) -- `cargo clippy --no-deps -- -D warnings`. Lint check.
3. **Test** (`TestGate`) -- `cargo test`. Runs the test suite.
4. **Symbol** -- verifies expected symbols (functions, structs, traits) exist in the output with correct signatures and visibility.
5. **GeneratedTest** (`GeneratedTestGate`) -- generates and runs tests for the agent's changes.
6. **Property** -- property-based testing for invariants.
7. **LLMJudge** (`JudgeOracle`) -- an LLM reviews the diff against the task requirements and scores it.

Each rung produces a `GateResult` with a pass/fail verdict and evidence. The `enrich_rung_config()` function injects rung-specific oracle configurations (rungs 4-6 get special oracle enrichment). The `AdaptiveThresholds` system (from `roko-gate`) adjusts pass/fail thresholds per rung based on historical data, persisted to `.roko/learn/gate-thresholds.json`.

**Step 8: Record Episode.**
The `EpisodeLogger` (from `roko-learn`) writes a complete episode record to `.roko/episodes.jsonl`. Each episode is a JSON object with these fields:

```rust
pub struct Episode {
    pub kind: String,              // "agent_turn", "gate", "replan"
    pub id: String,                // hash-derived stable identifier
    pub timestamp: DateTime<Utc>,  // wall-clock time
    pub agent_id: String,          // e.g., "claude-implementer"
    pub task_id: String,           // which task in the plan
    pub episode_id: String,        // stable episode record ID
    pub agent_template: String,    // role name used for dispatch
    pub model: String,             // model slug (e.g., "claude-sonnet-4-20250514")
    pub backend: String,           // provider slug (e.g., "claude-cli")
    pub trigger_kind: String,      // what caused this dispatch
    pub started_at: DateTime<Utc>, // dispatch start time
    pub completed_at: DateTime<Utc>, // dispatch end time
    pub duration_secs: f64,        // execution duration
    pub gate_verdicts: Vec<GateVerdict>, // per-rung pass/fail + evidence
    pub usage: Usage,              // prompt_tokens, completion_tokens, cost_usd
    pub success: bool,             // overall success
    pub turns: u64,                // number of agent tool-loop turns
    pub tokens_used: u64,          // total tokens consumed
    pub failure_reason: Option<String>, // hashed failure reason
    pub reflection: Option<String>,     // post-gate reflection for learning
    pub hdc_fingerprint: Option<String>, // 10,240-bit HDC vector of the episode
    pub emotional_tag: Option<EmotionalTag>, // affect state at completion
    pub prompt_composition: Option<Value>,   // which prompt sections were included
    pub extra: HashMap<String, Value>,       // forward-compat extension bag
}
```

The `hdc_fingerprint` is computed by `fingerprint_episode()` (from `roko-learn/src/hdc_fingerprint.rs`), which encodes the episode's prompt, outcome, and metadata as a 10,240-bit binary vector. This fingerprint enables similarity search over past episodes.

**Step 9: Update Learning.**
After persisting the episode, orchestrate.rs updates multiple learning subsystems:
- **CascadeRouter feedback**: records which model was used and whether it succeeded, updating the contextual bandit
- **Efficiency event**: emits an `AgentEfficiencyEvent` to `.roko/learn/efficiency.jsonl` with per-turn cost, latency, and token usage data
- **Error pattern store**: if the task failed, `learned_error_signature()` extracts the error pattern and stores it for future avoidance
- **Skill library**: if the task succeeded, `SkillLibrary` may extract a reusable skill from the episode
- **Section effectiveness**: records which prompt sections correlated with success via `SectionEffectivenessRegistry`
- **Conductor bandit**: updates the retry strategy bandit (`ConductorBandit`) based on outcome

**Step 10: Gate Failure Replan.**
If the gate pipeline failed and `learning_config.replan_on_gate_failure` is enabled in `roko.toml`, orchestrate.rs calls `build_gate_failure_plan_revision()`. This generates a new task plan specifically targeting the failure. The replan includes the error message, failing test names, and the original task context. The `ReplanLedger` tracks replans to prevent infinite retry loops.

**Step 11: Advance DAG.**
The `ParallelExecutor` advances the DAG state: marks the current task as completed (or failed), checks which downstream tasks are now unblocked, and returns the next batch of ready tasks. The executor snapshot is periodically persisted to `.roko/state/executor.json` for crash recovery (resumable via `roko plan run plans/ --resume .roko/state/executor.json`).

**This is the integration point.** Chain interactions insert into this flow at specific moments:
- After Step 7 (gate passes) and before Step 9 (learning update): anchor a witness hash on-chain
- Before Step 5 (dispatch): query chain for knowledge from other agents
- Continuously in background: send heartbeat transactions to prove liveness

### The roko-chain Crate

The `roko-chain` crate (located at `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/`) is a substantial module with blockchain interaction code. Here is what it provides:

**Core traits:**

`ChainClient` trait -- an async interface for reading blockchain state:
```rust
#[async_trait]
pub trait ChainClient: Send + Sync {
    async fn block_number(&self) -> ChainResult<BlockNumber>;
    async fn get_block_header(&self, number: BlockNumber) -> ChainResult<ChainHeader>;
    async fn get_transaction_receipt(&self, hash: TxHash) -> ChainResult<Option<Receipt>>;
    async fn get_logs(&self, filter: Filter) -> ChainResult<Vec<LogEntry>>;
    async fn get_balance(&self, address: Address) -> ChainResult<U256>;
    async fn eth_call(&self, tx: TxRequest) -> ChainResult<CallResult>;
    async fn chain_id(&self) -> ChainResult<u64>;
}
```

`ChainWallet` trait -- an async interface for signing and submitting transactions.

**Implementations:**

- `AlloyChainClient` -- uses the Alloy Rust library for Ethereum JSON-RPC. Behind the `alloy-backend` Cargo feature flag. Already imported in orchestrate.rs on line 34: `use roko_chain::alloy_impl::{AlloyChainClient, AlloyChainWallet};`
- `AlloyChainWallet` -- wraps a signing key and RPC client for transaction submission
- `MockChainClient` / `MockChainWallet` -- test doubles via `paired_mocks()`

**Witness engine:**

- `ChainWitnessEngine` -- anchors attestation hashes on-chain. Takes a blake3 hash and submits it as transaction calldata. Exported as `witness_on_chain` and `verify_on_chain` free functions.

**Domain-specific modules (built but not all connected):**

- `AgentRegistry` -- soulbound ERC-721 passport system for agent identity (4 tiers, staking, timelocks)
- `KoraiToken` / `KoraiTokenConfig` -- KORAI token with lazy demurrage
- `Marketplace` -- job marketplace with escrow and 3 hiring models
- `ReputationRegistry` -- 7-domain exponential moving average reputation scores
- `ValidationRegistry` -- work proof + validator attestation
- `ChainWitnessEngine` -- witness anchoring (the Phase 1 integration point)
- `ChainHeartbeatExtension` -- agent heartbeat with policy cage and sleepwalker detection
- `BlockObserver` -- event observation with address filtering
- `MevGate` / `TxSimGate` -- MEV detection and transaction simulation gates
- `X402Manager` -- HTTP 402 micropayment protocol with state channels
- `FuturesMarket` -- prediction market for agent performance
- `IsfrRegistry` -- inter-system flow rate tracking
- `TraceRank` -- PageRank-style reputation propagation over payment edges
- `collusion` -- collusion ring detection via assignment graph clique analysis
- `nelson_siegel` -- yield curve model for DeFi oracle rate term structure
- `tools` -- 10 chain domain DeFi tool definitions

**The critical problem:** orchestrate.rs already imports `AlloyChainClient` and `AlloyChainWallet` and instantiates them conditionally based on `roko_config.chain.rpc_url`. The `chain_client` and `chain_wallet` fields exist on the `PlanRunner` struct. But the witness engine is never called in the task completion flow. The chain client is wired for initialization but not used for knowledge queries or witness anchoring in the per-task loop.

### How roko-chain Is Already Wired in orchestrate.rs

orchestrate.rs already contains this initialization code (around line 4353):

```rust
let chain_client: Option<Arc<dyn ChainClient>> = match roko_config.chain.rpc_url.as_deref() {
    Some(url) => match AlloyChainClient::http(url) {
        Ok(c) => {
            tracing::info!(rpc_url = url, "chain client initialized");
            Some(Arc::new(c))
        }
        Err(e) => {
            tracing::warn!(error = %e, "chain client init failed; chain features disabled");
            None
        }
    },
    None => None,
};

let chain_wallet: Option<Arc<dyn ChainWallet>> = match (
    roko_config.chain.rpc_url.as_deref(),
    roko_config.chain.wallet_key.as_deref(),
) {
    (Some(url), Some(key)) => {
        let chain_id = roko_config.chain.chain_id.unwrap_or(1);
        match AlloyChainWallet::from_hex_key(url, key, chain_id) {
            Ok(w) => Some(Arc::new(w)),
            Err(e) => {
                tracing::warn!(error = %e, "wallet_key invalid; chain signing disabled");
                None
            }
        }
    }
    _ => None,
};
```

The `PlanRunner` struct stores these as fields:
```rust
chain_client: Option<Arc<dyn ChainClient>>,
chain_wallet: Option<Arc<dyn ChainWallet>>,
```

What is missing: calling `witness_on_chain()` after gate passes, querying chain knowledge before dispatch, and starting heartbeat loops.

### The mirage-rs Component

**mirage-rs** is an in-process EVM fork simulator that roko uses for a different purpose: forking the state of a live Ethereum network (e.g., mainnet) at a specific block and running transactions against that forked state locally. It has chain extensions for agent-specific operations: an HDC index (for searching 10,240-bit binary vectors), a knowledge store, a pheromone field (a stigmergy-inspired mechanism where agents leave "scent" on knowledge entries they find useful), and an agent registry.

mirage-rs has `Substrate` and `Gate` trait implementations that bridge to roko-core. It assumes in-process execution -- it cannot talk to an external blockchain over the network.

mirage-rs and daeji serve completely different purposes. mirage-rs answers "what would happen if I executed this transaction on the current Ethereum mainnet state?" Daeji answers "record this agent's knowledge and work output on a shared ledger with real consensus finality." They coexist and agents can talk to both. The existing contract deployment flow uses mirage-rs for simulation first, then deploys to the real target chain.

### The Solidity Contracts

Roko has 10 Solidity smart contracts in `/Users/will/dev/nunchi/roko/roko/contracts/src/`:

| Contract | Purpose |
|---|---|
| `IdentityRegistry.sol` | Soulbound identity passport -- 4 tiers, staking, timelocks |
| `AgentRegistry.sol` | Agent identity -- capabilities, heartbeat, liveness tracking |
| `InsightBoard.sol` | Knowledge curation -- post content hashes, confirm entries, pheromone weighting |
| `ReputationRegistry.sol` | 7-domain exponential moving average reputation scores |
| `ValidationRegistry.sol` | Work proof + validator attestation |
| `BountyMarket.sol` | Bounty marketplace for cross-agent task exchange |
| `WorkerRegistry.sol` | Worker staking |
| `ConsortiumValidator.sol` | Consortium validation |
| `FeeDistributor.sol` | Fee distribution |
| `MockERC20.sol` | Simple DAEJI test token (standard ERC-20, no special features) |

These contracts were deployed against a local Anvil instance (Foundry's built-in single-node test chain) or mirage-rs. They have never been deployed to daeji. No Solidity code changes are needed -- the contracts are chain-agnostic. They just need to be redeployed to daeji's RPC endpoint.

### The Daeji Side

**Daeji** (internal codename "Kora") is a minimal EVM-compatible blockchain node. "EVM-compatible" means it executes the same bytecode that Ethereum does (via REVM -- the Rust Ethereum Virtual Machine, the same engine Foundry and Reth use), so any Ethereum tooling (Foundry, MetaMask, Alloy, ethers.js) works against it without modification.

Daeji is built from the commonware library suite: simplex BFT consensus (a Byzantine Fault Tolerant protocol using BLS12-381 threshold signatures), the commonware P2P authenticated overlay for networking, and QMDB (Quick Merkle Database) for state storage. It runs as 4 validator nodes that collectively finalize blocks every ~400ms. Chain ID: 1337. It exposes standard `eth_*` JSON-RPC methods plus a custom `kora_*` namespace.

### The Neuro Store

The **neuro store** is roko's local knowledge persistence layer, implemented in the `roko-neuro` crate (`/Users/will/dev/nunchi/roko/roko/crates/roko-neuro/src/`). It consists of two primary structures:

**`KnowledgeStore`** -- persists to `.roko/neuro/knowledge.jsonl`, an append-only JSONL file where each line is a JSON-serialized `KnowledgeEntry`. The store is backed by a file path and a write gate (mutex) for concurrent access. It provides methods for ingestion, querying by HDC similarity, tag-based filtering, and garbage collection of decayed entries.

**`KnowledgeEntry`** -- a single knowledge item with these fields (from the actual Rust struct):

```rust
pub struct KnowledgeEntry {
    pub id: String,                    // unique identifier
    pub kind: KnowledgeKind,           // Insight, Heuristic, Warning, etc.
    pub source: Option<String>,        // provenance label
    pub content: String,               // the actual knowledge text
    pub confidence: f64,               // 0.0..=1.0
    pub confidence_weight: f64,        // signed retrieval weight
    pub refuted_insight_id: Option<String>,    // for AntiKnowledge
    pub refutation_evidence: Option<String>,   // why the refuted insight was wrong
    pub source_episodes: Vec<String>,  // episode IDs that contributed
    pub tags: Vec<String>,             // topic tags for retrieval
    pub source_model: Option<String>,  // which model produced this
    pub model_generality: f64,         // 1.0 = fully general, 0.0 = model-specific
    pub created_at: DateTime<Utc>,     // creation timestamp
    pub half_life_days: f64,           // exponential decay half-life
    pub tier: KnowledgeTier,           // Transient, Working, Consolidated, Persistent
    pub emotional_tag: Option<EmotionalTag>,        // affect provenance
    pub emotional_provenance: Option<EmotionalProvenance>, // emotional reliability
    pub hdc_vector: Option<Vec<u8>>,   // 10,240-bit HDC vector (1,280 bytes)
    pub confirmation_count: u32,       // independent confirmations
    pub distinct_contexts: Vec<String>, // contexts that confirmed this entry
    pub deprecated: bool,              // explicitly deprecated
    pub balance: f64,                  // freshness reserve (demurrage model)
    pub frozen: bool,                  // cold storage flag
    pub catalytic_score: u32,          // how many new entries this helped create
}
```

**6 knowledge kinds**, each with a default half-life:
- `Insight` (30 days) -- factual observations, e.g., "Using --no-cache flag fixes stale module resolution"
- `Heuristic` (90 days) -- behavioral rules, e.g., "TypeScript projects should run tsc before jest"
- `Warning` (1 hour) -- urgent transient conditions, e.g., "Database migration 042 breaks the users table schema"
- `AntiKnowledge` (30 days) -- what failed, e.g., "Do NOT use regex for HTML parsing"
- `CausalLink` (60 days) -- observed causation, e.g., "Adding --strict to tsc causes 3x more gate failures"
- `StrategyFragment` (14 days) -- reusable approaches, e.g., "When refactoring imports, update barrel files first"

**4 retention tiers** with lifetime multipliers:
- `Transient` (0.1x) -- short-lived, decays aggressively
- `Working` (0.5x) -- active working memory
- `Consolidated` (1.0x) -- validated, decays at base rate
- `Persistent` (5.0x) -- highly durable, decays very slowly

Tier promotion rules: 2+ confirmations for Transient to Working; 3+ distinct contexts and confidence >= 0.70 for Working to Consolidated.

**`ContextAssembler`** -- queries the knowledge store before each task dispatch. Located in `crates/roko-neuro/src/context.rs`. It retrieves candidate entries by HDC similarity, filters by decay weight and confidence, ranks by a composite score (HDC similarity 40%, keyword relevance 30%, predictive foraging utility 20%, freshness 10%), and compresses to fit within a token budget. Key constants:

```rust
const BASE_ATTENTION_RESERVE: f64 = 0.18;       // reserve 18% for structure
const MAX_CHUNK_BUDGET_FRACTION: f64 = 0.35;     // no single entry > 35% of budget
const SAME_SOURCE_DIMINISHING_RETURNS: f64 = 0.82; // 18% discount per same-source
const MARGINAL_VALUE_STOP_RATIO: f64 = 0.5;      // stop when next < 50% of average
const CONTRARIAN_RETRIEVAL_RATIO: f64 = 0.15;    // 15% of results from contrarian search
```

The neuro store is local to a single roko instance. It does not persist across agent fleets or operators. One of the key goals of the daeji integration is to make high-confidence neuro store entries shareable across agents via the on-chain InsightBoard contract.

---

## Part 2: What "Witness Anchoring" Means

When we say "anchor a witness on-chain," we mean this: after an agent completes a task and the task passes all gate rungs (compile, test, lint, diff), roko computes a cryptographic hash of the complete episode data (the full JSON record of every agent turn, tool call, and gate result for that task). It then submits a transaction to the daeji chain where the transaction's calldata (the data payload of the transaction) contains this hash.

Once the transaction is mined and finalized, anyone who has the original episode data can:
1. Recompute the hash: `blake3(episode_json_bytes)`
2. Look up the transaction on daeji by its hash
3. Verify that the hash in the transaction's calldata matches

If the hashes match, the episode data has not been tampered with since it was anchored. The on-chain transaction is the witness -- it proves that at block N, this specific hash was committed. The chain's consensus guarantees that this record cannot be altered retroactively.

This is the simplest possible on-chain integration: post a 32-byte hash as proof of work. No smart contract needed (just send to a sink address). No reads required. It is the foundation that Phase 1 builds on.

---

## Part 3: Phase 1 -- Basic Chain Connection

Phase 1 connects roko's main execution loop to daeji over JSON-RPC. No daeji source modifications are required (except the critical one-line timestamp fix). By the end of Phase 1:
- Existing Solidity contracts are deployed to daeji
- Every task completion anchors a witness hash on-chain
- Agents register themselves and send periodic heartbeats

### Step 1: Fix Daeji's block.timestamp

**Why this comes first:** Multiple Solidity contracts (IdentityRegistry, ReputationRegistry, and the upgraded InsightBoard) use `block.timestamp` for timing logic. Roko's knowledge decay formula is `weight = initial * 2^(-elapsed/half_life)`, where `elapsed` is in wall-clock seconds. The InsightBoard's `currentWeight()` function computes `age = block.timestamp - entry.timestamp` to determine how many half-lives have passed. In daeji, `block.timestamp` is currently set to the block height (block number), not to wall-clock Unix time. This means `block.timestamp` at block 1000 is `1000`, not `1714000000`. Every contract that computes elapsed time in seconds is broken -- the decay math produces wildly wrong values (computing elapsed blocks instead of elapsed seconds).

**The fix:** In the daeji codebase, find where `BlockContext` is constructed for block proposals (in `crates/node/consensus/src/app.rs` or the equivalent file). Change the timestamp from `height` to actual wall-clock time:

```rust
use std::time::{SystemTime, UNIX_EPOCH};

// Change from:
//   timestamp: height
// To:
let timestamp = SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .unwrap()
    .as_secs();
```

This is a one-line change. The risk is low: validators may have slightly different wall clocks, but simplex consensus tolerates this because the timestamp is just a field in the proposed block payload, not used for consensus decisions. Verifiers should accept timestamps within a reasonable window (e.g., plus or minus 30 seconds of local time).

### Step 2: Fix BLOCKHASH Opcode

**Why this matters:** The `BLOCKHASH` EVM opcode currently returns zero for all inputs in daeji. Roko agents need recent block hashes for: VRF seed derivation (combining `prevrandao` with recent block hashes for additional entropy), commit-reveal schemes (anchoring commitments against specific blocks), and audit trail references (proving an event happened relative to a specific block). While less critical than the timestamp fix, it is an EVM compliance issue that breaks existing Solidity libraries and patterns.

**The fix:** Add a ring buffer data structure to the executor that stores the hash of each finalized block (up to the most recent 256):

```rust
// Add to the executor state (in crates/node/executor/src/revm.rs):
use std::collections::VecDeque;

pub struct BlockHashCache {
    hashes: VecDeque<(u64, B256)>,  // (block_number, block_hash) pairs
    max_depth: usize,                // 256, matching the EVM spec
}

impl BlockHashCache {
    pub fn new() -> Self {
        Self {
            hashes: VecDeque::with_capacity(256),
            max_depth: 256,
        }
    }

    pub fn get(&self, number: u64) -> B256 {
        self.hashes
            .iter()
            .find(|(n, _)| *n == number)
            .map(|(_, h)| *h)
            .unwrap_or(B256::ZERO)
    }

    pub fn push(&mut self, number: u64, hash: B256) {
        self.hashes.push_back((number, hash));
        while self.hashes.len() > self.max_depth {
            self.hashes.pop_front();
        }
    }
}
```

In the EVM execution configuration, replace the `block_hash_ref` closure that returns `B256::ZERO` with one that queries this cache:

```rust
let block_hash_cache = block_hash_cache.clone(); // Arc<RwLock<BlockHashCache>> or similar
let block_hash_ref = move |number: u64| -> B256 {
    block_hash_cache.get(number)
};
```

After each block is finalized, push its hash into the cache.

### Step 3: Deploy Existing Contracts to Daeji

No new contracts need to be written. Deploy the contracts that already exist in roko's repository.

**Prerequisites:**
- Daeji devnet is running (`cd /path/to/daeji && just trusted-devnet`)
- Foundry is installed (`curl -L https://foundry.paradigm.xyz | bash && foundryup`)

**Deploy:**

```bash
cd /Users/will/dev/nunchi/roko/roko

# Roko already has deployment scripts. The existing flow uses mirage-rs for
# simulation first: forge script runs against a forked state to verify the
# deployment sequence, then broadcasts to the real target.

# Deploy all contracts to daeji's RPC endpoint.
# --rpc-url: the JSON-RPC endpoint of daeji node0
# --chain-id: daeji's chain ID (1337)
# --broadcast: actually submit transactions (without this, it is a dry run)
# --slow: wait for each transaction to be mined before sending the next
# --legacy: use legacy transaction format (some minimal chains handle this better)
forge script contracts/script/Deploy.s.sol \
  --rpc-url http://localhost:8545 \
  --chain-id 1337 \
  --broadcast \
  --slow --legacy
```

This deploys: MockERC20 (the DAEJI test token), AgentRegistry, WorkerRegistry, BountyMarket, ConsortiumValidator, and InsightBoard. Foundry's `forge script` broadcasts the deployment transactions and reports the deployed contract addresses.

**Record the deployed addresses** in roko's configuration file (`roko.toml`, the TOML configuration file in the project root). The `[chain]` section already exists and is parsed by `roko_core::config::schema::RokoConfig`:

```toml
# roko.toml -- the [chain] section configures blockchain connectivity.
# These fields are already defined in the config schema.
[chain]
rpc_url = "http://localhost:8545"
chain_id = 1337
# The agent's Ethereum transaction signing key (secp256k1 private key, hex-encoded).
# This is NOT the Ed25519 key used by daeji validators for consensus.
# Generate one with: cast wallet new
wallet_key = "${DAEJI_AGENT_KEY}"

# Contract addresses (new fields to add to the config schema)
[chain.contracts]
daeji_token = "0x..."       # Address of the deployed MockERC20
agent_registry = "0x..."    # Address of the deployed AgentRegistry
insight_board = "0x..."     # Address of the deployed InsightBoard
bounty_market = "0x..."     # Address of the deployed BountyMarket
```

The `${DAEJI_AGENT_KEY}` syntax means the value is read from an environment variable. The private key should never be stored directly in a configuration file that might be committed to version control.

### Step 4: Instantiate AlloyChainClient in orchestrate.rs

The `AlloyChainClient` already exists and is already instantiated in orchestrate.rs. The code at line 4353 already creates `chain_client` and `chain_wallet` from the config. The `PlanRunner` struct already stores them. What remains is to use them in the per-task loop.

The chain client and wallet are created during `PlanRunner::new()`:

```rust
// This code already exists in orchestrate.rs around line 4353.
// It reads [chain] from roko.toml and creates the client if configured.
let chain_client: Option<Arc<dyn ChainClient>> = match roko_config.chain.rpc_url.as_deref() {
    Some(url) => match AlloyChainClient::http(url) {
        Ok(c) => {
            tracing::info!(rpc_url = url, "chain client initialized");
            Some(Arc::new(c))
        }
        Err(e) => {
            tracing::warn!(error = %e, "chain client init failed; chain features disabled");
            None
        }
    },
    None => None,
};

let chain_wallet: Option<Arc<dyn ChainWallet>> = match (
    roko_config.chain.rpc_url.as_deref(),
    roko_config.chain.wallet_key.as_deref(),
) {
    (Some(url), Some(key)) => {
        let chain_id = roko_config.chain.chain_id.unwrap_or(1);
        match AlloyChainWallet::from_hex_key(url, key, chain_id) {
            Ok(w) => Some(Arc::new(w)),
            Err(e) => {
                tracing::warn!(error = %e, "wallet_key invalid; chain signing disabled");
                None
            }
        }
    }
    _ => None,
};
```

### Step 5: Wire Witness Anchoring Into the Task Completion Flow

This is the core Phase 1 feature. In orchestrate.rs, after the gate pipeline succeeds for a task (between Step 7 and Step 9 in the flow described above), add the witness anchoring call:

```rust
// In orchestrate.rs -- after gate_pipeline succeeds, before learning update:
use roko_chain::{witness_on_chain, ChainWitnessEngine};

if let (Some(ref client), Some(ref wallet)) = (&self.chain_client, &self.chain_wallet) {
    // Serialize the complete episode record to JSON bytes
    let episode_bytes = serde_json::to_vec(&episode)?;
    // Compute a blake3 hash (32 bytes) of the episode data
    let episode_hash = blake3::hash(&episode_bytes);

    // Submit a transaction to the chain.
    // The transaction's calldata is: b"roko.attestation.witness:" + episode_hash
    // The transaction goes to a fixed sink address (0x00...c0).
    match witness_on_chain(client.as_ref(), wallet.as_ref(), &episode_hash.as_bytes()).await {
        Ok(receipt) => {
            tracing::info!(
                tx_hash = %receipt.tx_hash,
                block = receipt.block_number,
                "episode witness anchored on-chain"
            );
            // Store the chain attestation alongside the episode
            episode.extra.insert(
                "chain_attestation".to_string(),
                serde_json::json!({
                    "chain_id": self.roko_config.chain.chain_id.unwrap_or(1),
                    "tx_hash": format!("{}", receipt.tx_hash),
                    "block_number": receipt.block_number,
                }),
            );
        }
        Err(e) => {
            // Witness failure is non-fatal -- log and continue
            tracing::warn!(error = %e, "witness anchoring failed");
        }
    }
}
```

**What this achieves:** Every completed task now has an on-chain fingerprint. Anyone with the episode JSON file can verify it against the chain: recompute the hash, look up the transaction, compare. The chain record cannot be retroactively altered.

### Step 6: Wire Agent Heartbeats

The `AgentRegistry` contract tracks which agents are alive via periodic heartbeat transactions. The `ChainHeartbeatExtension` in roko-chain already implements the heartbeat logic with policy cage and sleepwalker detection. Wire it into the `ProcessSupervisor`:

```rust
// In orchestrate.rs or in the ProcessSupervisor (the component that manages
// agent subprocess lifecycle, located in crates/roko-runtime/src/).
// Start a background task that sends a heartbeat every ~15 minutes.

use std::time::Duration;

async fn heartbeat_loop(
    registry_address: Address,
    wallet: Arc<dyn ChainWallet>,
) {
    loop {
        // Encode the function call: AgentRegistry.heartbeat()
        let calldata = agent_registry_abi::heartbeat();

        let result = wallet.send_tx(TxRequest {
            to: Some(registry_address),
            data: Some(calldata),
            ..Default::default()
        }).await;

        if let Err(e) = result {
            tracing::warn!("heartbeat tx failed: {e}");
        }

        // Wait 15 minutes before the next heartbeat
        tokio::time::sleep(Duration::from_secs(900)).await;
    }
}

// Spawn this as a background task when the plan runner starts:
if let Some(ref wallet) = chain_wallet {
    if let Some(registry_addr) = config.chain.contracts.get("agent_registry") {
        let addr = registry_addr.parse::<Address>()?;
        tokio::spawn(heartbeat_loop(addr, wallet.clone()));
    }
}
```

---

## Part 4: Phase 2 -- Knowledge Layer

Phase 2 connects the neuro store (roko's local knowledge database) to the InsightBoard contract on daeji. Knowledge flows in two directions: high-confidence local entries are promoted to the chain (so other agents can discover them), and new chain entries from other agents are pulled into the local store.

### How Knowledge Enters the Neuro Store Today

Before understanding chain integration, it helps to understand the existing local knowledge flow in detail:

1. An agent completes a task and produces an episode.
2. If the task passes gates, the episode is logged to `.roko/episodes.jsonl`.
3. The `install_episode_distillation_hook()` function (from `crate::learning_helpers`) registers a callback that fires after each successful episode.
4. The distillation hook extracts knowledge from the episode: factual observations become Insights, behavioral patterns become Heuristics, failures become AntiKnowledge.
5. Each extracted entry is stored in the neuro store via `KnowledgeStore::ingest()`, which:
   - Computes an HDC vector from the entry's content, kind, and tags
   - Checks for duplicates (HDC similarity > 90% = confirmation of existing entry)
   - Checks for AntiKnowledge conflicts (similarity > 70% to existing AntiKnowledge = discount)
   - Assigns the entry to the `Transient` tier initially
6. At dispatch time, `ContextAssembler` queries the store for entries relevant to the current task.
7. The `TierProgression` system (from `roko-neuro/src/tier_progression.rs`) promotes entries through tiers as they accumulate confirmations.
8. Layer 7 of the 9-layer system prompt injects the selected neuro entries as guidance.

### InsightBoard Contract: What It Does

The `InsightBoard.sol` contract stores knowledge entry metadata on-chain. Each entry consists of:
- `contentHash` (bytes32): the blake3 hash of the full text content
- `poster` (address): the Ethereum address of the agent that posted it
- `timestamp` (uint64): when it was posted (Unix seconds)
- `pheromone` (uint64): how many other agents have confirmed this entry as useful
- `entryType` (uint8): what kind of knowledge (0=Insight, 1=Heuristic, 2=Warning, etc.)
- `halfLifeHrs` (uint16): how quickly the entry decays in relevance (in hours)

The full text content is NOT stored in contract state (that would be prohibitively expensive at ~20,000 gas per 32 bytes). Instead, the full content is emitted in the `InsightPosted` event log. Ethereum event logs are cheap to write but cannot be read by contracts -- they are only readable by off-chain clients via `eth_getLogs`. This is the standard pattern for on-chain data availability at low cost.

### Post-Task Knowledge Commit Flow

When a task completes and passes gates, the flow for promoting an entry to the chain:

```
Task completes --> Gate passes --> Episode logged --> Distillation hook fires
    --> Extract learnings from episode
    --> Ingest into local neuro store (with HDC vector + tier assignment)
    --> Filter by confidence (>= 0.70) and confirmation count (>= 3)
    --> Post to InsightBoard (on-chain: content hash + metadata)
    --> Full content emitted in event log (readable by other agents via eth_getLogs)
    --> HDC vector NOT posted on-chain (computed locally by each agent from the text)
```

```rust
// In orchestrate.rs -- after successful task completion and witness anchoring:

async fn post_knowledge_to_chain(
    entry: &KnowledgeEntry,
    insight_board: Address,
    wallet: &Arc<dyn ChainWallet>,
) -> Result<()> {
    let content_hash = blake3::hash(entry.content.as_bytes());

    let entry_type: u8 = match entry.kind {
        KnowledgeKind::Insight => 0,
        KnowledgeKind::Heuristic => 1,
        KnowledgeKind::Warning => 2,
        KnowledgeKind::CausalLink => 3,
        KnowledgeKind::StrategyFragment => 4,
        KnowledgeKind::AntiKnowledge => 5,
    };

    // Convert half-life from days (roko's internal unit) to hours (the contract's unit)
    let half_life_hrs = (entry.half_life_days * 24.0) as u16;

    let calldata = insight_board_abi::post(
        content_hash.as_bytes().into(),
        entry_type,
        half_life_hrs,
        entry.content.clone(),
    );

    wallet.send_tx(TxRequest {
        to: Some(insight_board),
        data: Some(calldata),
        ..Default::default()
    }).await?;

    Ok(())
}

// Call for each eligible entry from the neuro store:
if let Some(ref neuro) = self.neuro_store {
    let entries = neuro.entries_above_threshold(0.70, 3);
    for entry in entries {
        if entry.source.as_deref() != Some("chain") {
            post_knowledge_to_chain(&entry, insight_board_addr, &wallet).await?;
        }
    }
}
```

### Pre-Task Knowledge Query Flow

Before dispatching an agent on a task, orchestrate.rs already queries the local neuro store (via `query_anti_knowledge_patterns()`, `render_neuro_chunk()`, and `build_knowledge_routing_advice()`). With daeji, this expands to include chain knowledge:

```
New task --> Encode task description to HDC vector (for local search)
    --> Search local neuro store (fast, in-process, < 1ms)
    --> Search daeji InsightBoard events via eth_getLogs (cross-agent knowledge)
    --> Merge and rank results by similarity + weight + trust
    --> Compress into system prompt context (target: ~800 tokens)
    --> Inject into layer 7 of the 9-layer SystemPromptBuilder
```

```rust
// In orchestrate.rs -- before task dispatch, extending existing knowledge query:

async fn query_chain_knowledge(
    task: &TaskDef,
    insight_board: Address,
    chain_client: &Arc<dyn ChainClient>,
) -> Vec<KnowledgeEntry> {
    let logs = chain_client.get_logs(Filter::new()
        .address(insight_board)
        .event("InsightPosted(bytes32,uint8,uint16,string,address)")
        .from_block(BlockNumberOrTag::Earliest)
    ).await.unwrap_or_default();

    logs.iter().filter_map(|log| {
        let content = decode_string_from_log(log)?;
        let vector = HdcVector::from_text(&content);
        Some(KnowledgeEntry {
            content,
            hdc_vector: Some(vector.to_bytes()),
            source: Some("chain".to_string()),
            confidence: 0.5, // start at moderate confidence for chain entries
            ..Default::default()
        })
    }).collect()
}
```

### Confirmation Flow

When an agent uses a chain knowledge entry during a task and the task succeeds, the agent confirms the entry on-chain. This increments the entry's pheromone count and credits the original poster:

```rust
// In orchestrate.rs -- after task succeeds and used chain knowledge:

async fn confirm_useful_entry(
    content_hash: B256,
    insight_board: Address,
    wallet: &Arc<dyn ChainWallet>,
) -> Result<()> {
    let calldata = insight_board_abi::confirm(content_hash);
    wallet.send_tx(TxRequest {
        to: Some(insight_board),
        data: Some(calldata),
        ..Default::default()
    }).await?;
    Ok(())
}
```

### NeuroChainSync: Bidirectional Sync

The `NeuroChainSync` component (to be added in `roko-chain` or `roko-neuro`) manages the bidirectional flow between the local neuro store and the on-chain InsightBoard:

**Push (local to chain):** Periodically scan the local neuro store for entries that meet the promotion criteria (confidence >= 0.70, confirmed 3+ times locally, not already on chain, not a near-duplicate of an existing chain entry). Post them to the InsightBoard.

**Pull (chain to local):** Periodically scan `InsightPosted` events on daeji for entries posted by other agents since the last sync. Fetch the full content from the event data. Compute an HDC vector locally. Store in the neuro store with `source: "chain"` and initial confidence 0.5. Apply normal neuro store admission gating (reject duplicates, reject entries below a relevance threshold).

```rust
pub struct NeuroChainSync {
    neuro: Arc<KnowledgeStore>,
    chain_client: Arc<dyn ChainClient>,
    wallet: Arc<dyn ChainWallet>,
    insight_board: Address,
    push_threshold: f64,         // default: 0.70
    min_confirmations: u32,      // default: 3
    last_synced_block: u64,
}

impl NeuroChainSync {
    pub async fn push_eligible_entries(&self) -> Result<()> {
        let entries = self.neuro.entries_above_threshold(
            self.push_threshold,
            self.min_confirmations,
        );
        for entry in entries {
            if entry.source.as_deref() != Some("chain")
                && !self.already_on_chain(&entry).await?
            {
                post_knowledge_to_chain(&entry, self.insight_board, &self.wallet).await?;
            }
        }
        Ok(())
    }

    pub async fn pull_new_entries(&mut self) -> Result<()> {
        let current_block = self.chain_client.block_number().await?;
        let logs = self.chain_client.get_logs(Filter::new()
            .address(self.insight_board)
            .event("InsightPosted(bytes32,uint8,uint16,string,address)")
            .from_block(self.last_synced_block + 1)
            .to_block(current_block)
        ).await?;

        for log in logs {
            let entry = decode_insight_from_log(&log);
            self.neuro.ingest(entry).await;
        }

        self.last_synced_block = current_block;
        Ok(())
    }
}
```

---

## Part 5: Phase 3 -- Daeji Source Changes

Phase 3 requires modifying the daeji node source code at `/Users/will/dev/nunchi/daeji/`. These changes add agent-specific capabilities that cannot be achieved through smart contracts alone.

### Custom RPC Methods

Add agent-relevant query methods to the `kora` namespace. Daeji's RPC server uses `jsonrpsee`. Adding new RPC methods means defining a trait with the `#[rpc(server)]` attribute and implementing it.

```rust
// daeji: crates/node/rpc/src/kora.rs

use jsonrpsee::proc_macros::rpc;

#[rpc(server, namespace = "kora")]
pub trait KoraAgentApi {
    /// Get active agents from the AgentRegistry.
    /// Active = sent heartbeat within last N blocks.
    #[method(name = "activeAgents")]
    async fn active_agents(&self) -> RpcResult<Vec<AgentInfo>>;

    /// Get knowledge entries from InsightBoard since a given block.
    /// Returns decoded event data, saving clients from log decoding.
    #[method(name = "recentKnowledge")]
    async fn recent_knowledge(
        &self,
        since_block: u64,
        limit: u64,
    ) -> RpcResult<Vec<KnowledgeEntry>>;

    /// Get the VRF seed for a finalized block.
    /// This is the prevrandao value from BLS12-381 threshold consensus.
    #[method(name = "vrfSeed")]
    async fn vrf_seed(&self, block: u64) -> RpcResult<B256>;
}
```

These methods are convenience wrappers. Everything they return is already accessible via standard `eth_*` calls, but the `kora_*` methods pre-decode the data and present it in a cleaner format.

### HDC Search Precompile

The HDC system in roko uses 10,240-bit binary vectors (`[u64; 160]`) for semantic similarity search. The core operation is Hamming distance: XOR two vectors, count the differing bits via POPCNT. Comparing two vectors takes ~320 CPU instructions (160 XOR + 160 POPCNT). Searching 100,000 entries takes ~170 microseconds in native Rust.

In Solidity, the same operation would cost ~320 XOR+POPCNT operations per comparison, each consuming gas. For 100 comparisons: ~3.2M gas. For 100,000 comparisons: impossible within a single transaction. The HDC search precompile runs this as native Rust code at EVM address 0x09:

```rust
// daeji: crates/node/executor/src/precompiles/hdc.rs

use revm::precompile::{Precompile, PrecompileResult, PrecompileOutput};
use std::sync::{Arc, RwLock};

pub const HDC_SEARCH_ADDRESS: Address = address!("0000000000000000000000000000000000000009");

pub struct HdcSearchPrecompile {
    index: Arc<RwLock<HdcIndex>>,
}

impl Precompile for HdcSearchPrecompile {
    fn run(&self, input: &[u8], _gas_limit: u64) -> PrecompileResult {
        // Input format:
        //   [0..1280]:    query vector (10,240 bits = 1,280 bytes)
        //   [1280]:       top_k (number of results, 1-255)
        if input.len() < 1281 {
            return Err(PrecompileError::Other("input too short".into()));
        }

        let query = HdcVector::from_bytes(&input[..1280]);
        let top_k = input[1280] as usize;

        // Brute-force Hamming distance search.
        // ~170 microseconds for 100K entries with SIMD POPCNT.
        let results = self.index.read().unwrap().search(&query, top_k);

        let encoded = encode_search_results(&results);

        // Fixed gas cost of 50,000 regardless of entry count
        Ok(PrecompileOutput { gas_used: 50_000, bytes: encoded })
    }
}
```

Register the precompile in the executor:

```rust
// daeji: crates/node/executor/src/revm.rs
let mut handler = ctx.build_mainnet();
handler.pre_execution.load_precompiles = Arc::new(move |precompiles| {
    precompiles.extend([(
        HDC_SEARCH_ADDRESS,
        hdc_precompile.clone(),
    )]);
});
```

The HDC index must be updated after each finalized block. The executor scans for InsightBoard events in the newly finalized block, extracts any posted knowledge entry content, computes the HDC vector from the content, and inserts it into the index.

---

## Part 6: Integration with Existing Roko Subsystems

### CascadeRouter (Model Selection)

The CascadeRouter (in `crates/roko-learn/src/`) selects which LLM model to use for each task based on historical performance and task category. With daeji, model selection can use the chain's VRF output for verifiable randomness:

```rust
// In CascadeRouter::select_model()
let block = chain_client
    .get_block_header(chain_client.block_number().await?)
    .await?;
// prevrandao is the BLS12-381 threshold signature output -- cryptographically
// unpredictable and bias-resistant
let vrf_seed = block.mix_hash;

// Use for weighted random model selection
let model_index = hash(vrf_seed, task_id, experiment_id) % models.len();
```

This makes model selection verifiable: anyone with the VRF seed (public in every block header) and the selection formula can verify the routing decision.

### ExperimentStore (A/B Testing)

The ExperimentStore (in `crates/roko-learn/src/`) runs A/B tests on system prompts. Currently it uses local random number generation for variant assignment. With daeji, assignment uses on-chain VRF for verifiable fairness.

### Gate Pipeline (Validation)

The gate pipeline (in `crates/roko-gate/src/`) runs validation rungs after each task. Two new rungs become available with daeji:

- **ChainWitness rung**: After all other rungs pass, anchor the episode hash on daeji and verify the transaction was mined.
- **ChainSimulate rung**: Before committing to the real chain, simulate the chain interactions using the commonware deterministic runtime (same code as production, single process, controlled timing).

### EpisodeLogger (Turn Recording)

The EpisodeLogger (in `crates/roko-learn/src/`) records every agent turn to `.roko/episodes.jsonl`. With daeji:
1. Compute `blake3(episode_json)` after each episode completes
2. Anchor the hash on-chain via `witness_on_chain()` (Phase 1, Step 5)
3. The episode gains a `chain_attestation` in its `extra` field: `{chain_id: 1337, tx_hash: "0x...", block_number: 12345}`
4. The episode is now tamper-evident: anyone with the raw JSON can verify it against the chain

### Neuro Store (Knowledge Persistence)

The neuro store (in `crates/roko-neuro/src/`) gains bidirectional sync as described in Part 4. The existing local knowledge flow (episode distillation -> ingest -> tier progression -> context assembly) is unchanged. The chain integration adds a new dimension:

**Local to chain**: Entries that reach `Consolidated` tier (confidence >= 0.70, 3+ distinct context confirmations) get promoted to InsightBoard.

**Chain to local**: Entries from other agents get ingested at `Transient` tier with initial confidence 0.5. If they prove useful in local task execution, they accumulate confirmations and promote through tiers normally.

This creates a flywheel: agents learn locally, promote to chain, other agents pull and use, successful usage triggers confirmation, confirmation increases chain weight, higher-weight entries are pulled by more agents.

---

## Part 7: File Layout

### New and Modified Files in Roko

```
crates/roko-chain/src/
  lib.rs              -- Add daeji module, re-export DaejiConfig
  daeji/
    mod.rs            -- Daeji-specific client wrapper (wraps AlloyChainClient with
                         contract-specific methods)
    config.rs         -- DaejiConfig: rpc_url, chain_id, wallet_key, contract addresses
                         Parsed from [chain] in roko.toml (extending existing schema)
    contracts.rs      -- ABI bindings for deployed contracts (generated by alloy-sol-types
                         or forge bind). InsightBoard, AgentRegistry, etc.
    sync.rs           -- NeuroChainSync: push/pull sync between neuro store and InsightBoard
  alloy_impl.rs       -- Already exists. Already instantiated in orchestrate.rs.
  witness.rs          -- Already exists. Wire witness_on_chain into task completion flow.

crates/roko-cli/src/
  orchestrate.rs      -- Wire witness anchoring after gate pass, heartbeat loop,
                         pre-task chain knowledge query, post-task knowledge commit.
                         All integration points are in the per-task dispatch loop.
  config.rs           -- Already parses [chain] section. Add contract addresses.
```

### Modified Files in Daeji (Phase 3 Only)

```
crates/node/executor/src/
  revm.rs             -- Custom precompile registration (replace build_mainnet()
                         with custom builder that includes HDC precompile)
  precompiles/
    mod.rs            -- Precompile registry module
    hdc.rs            -- HDC similarity search precompile at address 0x09

crates/node/consensus/src/
  app.rs              -- Fix block.timestamp to use wall-clock time instead of block height

crates/node/rpc/src/
  kora.rs             -- Add kora_activeAgents, kora_recentKnowledge, kora_vrfSeed methods
```
