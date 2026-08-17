# Open Questions

Ten unresolved design decisions that must be made before implementation proceeds. Each
question includes full context on why it matters, what the options are, and what the
trade-offs look like. This document is written for someone with no prior knowledge of the
project.

---

## Background: The Systems Involved

**Roko** is a Rust toolkit (~177,000 lines of code, 18 crates) that orchestrates AI coding
agents to execute software engineering tasks. The core loop: an agent reads a Product
Requirements Document (PRD), generates a task plan as a TOML file, dispatches LLM-powered
agents (Claude, Codex, Gemini, Ollama, etc.) to execute those tasks, validates each task's
output through a "gate pipeline" (compile, test, lint, diff review), and persists what was
learned to a local knowledge store. Roko is used to develop itself -- it reads its own PRDs,
generates its own plans, and executes them via AI agents.

The core execution loop lives in `crates/roko-cli/src/orchestrate.rs` (~11,000 lines). The
knowledge subsystem lives in `crates/roko-neuro/`. The chain integration lives in
`crates/roko-chain/` (built but not yet connected to the runtime). All 18 crates compile and
test. The self-hosting loop works end-to-end.

**Daeji** is a minimal EVM-compatible blockchain built from scratch using the commonware
library (github.com/commonwarexyz/monorepo). "EVM-compatible" means it runs standard
Ethereum smart contracts (written in Solidity) and speaks the standard `eth_` JSON-RPC
protocol. It uses Simplex BFT consensus with BLS12-381 threshold signatures, REVM for
execution, and QMDB for state storage. Chain ID: 1337. Block time: ~400ms.

**The integration goal** is to wire roko agents to daeji so that: (a) agent knowledge is
shared via an on-chain ledger, (b) task outcomes are tamper-evident via on-chain witness
hashes, and (c) novel cryptographic features (verifiable randomness, sealed commitments,
cross-chain certificates) become available.

---

## 1. Token Economics

### Why This Matters

The original design documents proposed a token called GNOS -- an ERC-20 token (Ethereum's
standard for fungible tokens) with 1% annual demurrage (balances decay over time,
incentivizing activity over hoarding). The token would be used for:
- Paying a fee to post knowledge entries (~1 GNOS per post)
- Receiving rewards when other agents confirm your entries (~0.1 GNOS per confirmation)
- Staking when challenging entries you believe are wrong (~5 GNOS stake)
- Staking for reputation in the agent registry

Whether to implement a token, and how to implement it, affects the entire incentive
structure of the knowledge sharing system.

### What Roko Already Does

Roko already tracks costs, but in USD, not tokens. The orchestrator in `orchestrate.rs`
enforces per-plan and per-task budgets via a `BudgetConfig` struct
(`crates/roko-cli/src/config.rs`):

```rust
pub struct BudgetConfig {
    pub max_plan_usd: f64,   // default $25, set in roko.toml [budget]
    pub max_task_usd: f64,   // default $2
    pub max_session_usd: f64,
    pub warn_at_percent: u32,
}
```

The orchestrator maintains two `HashMap`s -- `plan_costs` and `task_costs` -- that accumulate
USD spend per LLM API call. When a plan exceeds its budget, the orchestrator aborts with
`"failure budget exhausted"`. Cost data flows to `learn/costs.jsonl` for post-run analysis.

This is the existing "energy pool" system. Every plan starts with a USD budget (currently
$25 in `roko.toml`), and each agent dispatch deducts from it. The connection to GNOS is
direct: the GNOS token is an on-chain version of this same concept, with demurrage replacing
the current hard cutoff.

Roko also already has an in-memory KORAI token implementation in
`crates/roko-chain/src/korai_token.rs` with full lazy demurrage:

```rust
pub fn effective_balance(&self, now: u64, annual_rate: f64) -> u256 {
    let elapsed = (now - self.last_update) as f64;
    let decay_factor = (1.0 - annual_rate).powf(elapsed / SECONDS_PER_YEAR);
    (self.stored_balance as f64 * decay_factor) as u256
}
```

This Rust token has mint, burn, transfer, 5 earning pathways (TaskCompletion,
KnowledgeContribution, ValidationParticipation, ReputationStaking, MarketplaceFees), and
5 spending mechanisms. It also includes an `EmissionSchedule` with halving epochs and a
terminal emission rate. But it is purely in-memory -- used only in unit tests, never deployed
as a Solidity contract.

The Solidity side has `contracts/src/MockERC20.sol` -- a plain ERC-20 called "DAEJI" with
no demurrage. The `InsightBoard.sol` contract already uses an ERC-20 for confirmation
rewards (`REWARD_PER_CONFIRM = 1 ether`).

### Decisions Needed

**Do we want a token at all for the development network?** The alternative is to use plain
ETH (the native currency of any EVM chain, available on daeji for free via a faucet) for
gas (transaction fees) and staking. A custom token adds complexity: deployment, minting
policy, decay math, approval flows (ERC-20 requires a two-step approve-then-transfer
pattern). ETH is simpler and already works. The argument for a token is that it enables
demurrage and fine-grained economic incentives that plain ETH does not support.

**If yes, what is the minting policy?** The original design specified: 100 GNOS on agent
registration, 10-100 GNOS per knowledge entry post (depending on entry type), 250 GNOS per
block as a validator reward. These numbers were designed for a public network with many
independent operators. For a private development network where all agents are controlled by
the same operator, minting can be much simpler: a faucet that grants tokens on demand +
fixed rewards for specific actions. The risk of over-simple minting: if tokens are free and
unlimited, the economic incentives (pay to post, earn for confirmations) become meaningless.

**How should demurrage be implemented?** Demurrage means token balances shrink over time.
Two approaches:

- **Lazy (compute on read)**: balances are stored as-is. When `balanceOf(address)` is
  called, the contract computes the decayed balance based on when the address last
  transacted. Cheaper (no gas spent on decay computation until someone queries), but makes
  `balanceOf` a non-trivial computation instead of a simple storage read. External tools
  (block explorers, wallets) that cache balances may show stale values.

- **Eager (decay every block or transfer)**: on every transfer, compute decay for both
  sender and receiver. Simpler mental model (balance is always "current"), but costs gas on
  every transfer because two decay computations run on every token movement.

### What This Means for Roko

The practical question is whether the on-chain token should mirror the existing USD budget
system or replace it. Today, `orchestrate.rs` checks `BudgetConfig::max_plan_usd` before
every agent dispatch. If GNOS were wired in, the orchestrator could instead check an
on-chain balance and deduct tokens per dispatch. The demurrage mechanic would naturally
incentivize agents to spend their budget promptly rather than hoarding tokens across plans
-- mirroring the real-world cost pressure that makes roko's existing budget guardrails
necessary.

### Recommendation (not yet decided)

For the development network, skip the token entirely and use ETH. Add the token in Phase 4
when/if the network opens to multiple independent operators and real economic incentives
matter.

---

## 2. Knowledge Entry Storage

### Why This Matters

Knowledge entries are the core data structure of the shared intelligence system. An entry
is a piece of text (100-2000 bytes) that an agent learned during task execution. Entries
are posted to the InsightBoard smart contract on daeji. How they are stored determines gas
costs, state bloat, search capabilities, and how other agents discover and retrieve them.

### What Roko Already Does

Roko's local knowledge store is fully wired and actively used. The `roko-neuro` crate
stores knowledge entries as append-only JSONL at `.roko/neuro/knowledge.jsonl`. Each entry
is a `KnowledgeEntry` struct with rich metadata:

```rust
pub struct KnowledgeEntry {
    pub id: String,
    pub kind: KnowledgeKind,      // Insight, Heuristic, Warning, AntiKnowledge, CausalLink, StrategyFragment
    pub source: Option<String>,
    pub content: String,           // the actual learned text
    pub confidence: f64,           // 0.0..=1.0
    pub source_episodes: Vec<String>,
    pub tags: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub half_life_days: f64,       // exponential decay half-life
    pub tier: KnowledgeTier,       // Transient (0.1x), Working (0.5x), Consolidated (1.0x), Persistent (5.0x)
    pub emotional_tag: Option<EmotionalTag>,
    // ... additional fields
}
```

There are 6 knowledge kinds, each with a different default half-life:

| Kind | Default Half-Life | Purpose |
|---|---|---|
| Insight | 30 days | Compact causal observation distilled from episodes |
| Heuristic | 90 days | Lightweight rule of thumb |
| Warning | 1 hour | Cautionary note about failure modes |
| CausalLink | 60 days | Relationship between two observations |
| StrategyFragment | 14 days | Reusable approach fragment |
| AntiKnowledge | 30 days | What to avoid; what has failed |

The 4-tier retention system multiplies the base half-life:
- Transient: 0.1x (entries decay 10x faster)
- Working: 0.5x
- Consolidated: 1.0x (base rate)
- Persistent: 5.0x (entries last 5x longer)

Entries decay via the formula:
`weight = initial_weight * 0.5^(age_days / (half_life_days * tier_multiplier))`

Entries confirmed by multiple independent episodes get a boost:
`weight *= (1 + confirmations * 0.1)`

The `KnowledgeStore` supports query by tags and keywords, HDC vector similarity search
(when the `hdc` feature is enabled), anti-knowledge conflict detection, garbage collection
of dead entries (below 1% of initial weight), and cross-confirmation tracking between
independent episodes.

Currently, all of this is local. Nothing goes on-chain. The question is how much of this
rich local structure should be reflected on-chain, and how.

The existing on-chain contract `InsightBoard.sol` is much simpler:

```solidity
struct Insight {
    address poster;
    bytes32 contentHash;   // only the hash, not full content
    string uri;            // off-chain reference
    uint64 postedAt;
    uint64 pheromone;      // confirmation count
}
```

No kind classification, no half-life, no tier, no HDC vector. The gap between the local
store and the on-chain store is large.

### Decisions Needed

**Inline vs external content.** The original design stored full entry content (the actual
text) directly in the smart contract's storage. This is simple: one contract call retrieves
everything. But at EVM gas costs (~600 gas per byte for storage writes), a 1000-byte entry
costs ~600,000 gas just for the content. At scale, this bloats the chain's state (every
validator and secondary peer must store all entries forever).

The alternative: store only the content hash (32 bytes, a blake3 cryptographic hash that
uniquely identifies the content) in contract storage, and emit the full content in the
event log (events are part of the transaction receipt, accessible via `eth_getLogs`, but not
part of the EVM's mutable state). Off-chain indexers (including the agents themselves) read
events to get full content. This is ~50x cheaper per entry (71 bytes in storage vs 3,500).
The downside: full content is not available via a simple `eth_call` (read-only contract
call) -- you must query event logs, which requires knowing the block range to search.

**HDC vectors: on-chain or off-chain?** Each knowledge entry has an associated HDC vector
(10,240 bits = 1,280 bytes) used for semantic similarity search. `roko-primitives::HdcVector`
represents this as `[u64; 160]`. Storing the vector on-chain enables a future precompile
(native chain code at address 0x09) to search directly from EVM state. Storing it off-chain
(computed locally by each agent from the entry's text content) reduces state size
dramatically but means the precompile would need to maintain a separate index. The cost of
on-chain vectors: ~770,000 gas per entry (1,280 bytes * ~600 gas/byte). At daeji's 30M gas
block limit, this allows at most ~39 new entries per block.

**Entry pruning.** Roko's local store already has a death threshold: entries below 1% of
initial weight (`DEATH_THRESHOLD = 0.01`) are eligible for GC. On-chain, entries never
leave state. Options:

- **No pruning**: entries remain forever. State grows monotonically. Simple but unbounded.
- **Deterministic pruning**: a consensus operation where all validators prune entries whose
  weight has been zero for N blocks. Must be implemented identically across all validators
  to maintain state root agreement. Complex to implement correctly.
- **Lazy pruning**: individual nodes prune their local index (used for search) but chain
  state retains everything. The cheapest approach but means state still grows.

### Recommendation (not yet decided)

Lean toward: content hash on-chain, full content in events, HDC vectors off-chain, no
pruning for now (monitor state growth and revisit). This minimizes per-entry cost to ~71
bytes of contract storage and aligns with what `InsightBoard.sol` already does (stores
`contentHash` + `uri`, not full content).

---

## 3. Agent Identity Model

### Why This Matters

Agents need cryptographic identities for two purposes: (1) signing Ethereum transactions
to interact with daeji smart contracts (submit knowledge entries, register, heartbeat), and
(2) authenticating to the peer-to-peer network (if the agent runs as a secondary peer or
uses commonware-p2p directly for agent-to-agent messaging).

Daeji uses two different cryptographic systems for these purposes. The question is whether
agents should use one key for both or maintain separate keys.

### How Roko Currently Identifies Agents

Roko agents today have **no cryptographic identity**. An agent is identified by a string
name configured in `roko.toml`:

```toml
[agent]
default_model = "claude-sonnet"
default_backend = "anthropic"
default_effort = "medium"
```

The orchestrator dispatches agents by name. The `ProcessSupervisor` in `roko-runtime`
tracks them by PID. Nothing is signed, nothing is authenticated. Agent identity is purely
a local configuration concept.

The chain config in `roko.toml` does hold a single wallet key:

```toml
[chain]
rpc_url = "http://127.0.0.1:8545"
chain_id = 31337
wallet_key = "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80"
agent_registry = "0x9fE46736679d2D9a65F0992F2272dE9f3c7fa6e0"
bounty_market = "0xDc64a140Aa3E981100a9becA4E685f962f0cF6C9"
```

This key is a secp256k1 private key for EVM transaction signing, consumed by
`AlloyChainWallet::from_hex_key()` in `crates/roko-chain/src/alloy_impl.rs`. Currently one
key is shared across all agents in a roko instance -- there is no per-agent key.

The `IdentityRegistry.sol` contract already supports per-agent identity with 4 tiers
(Protocol, Sovereign, Worker, Edge), staking requirements, a soulbound ERC-721 passport,
capability bitmasks, and system prompt hash verification. But no roko code calls it yet.

Where cryptographic keys would live in roko's architecture:
- **secp256k1 (EVM signing)**: stored in `roko.toml [chain] wallet_key`, loaded by
  `AlloyChainWallet`
- **Ed25519 (P2P identity)**: does not exist yet in roko. Would need a new config field,
  a key generation command, and storage in `.roko/keys/` or `roko.toml`

### Technical Background

**Ed25519** -- an elliptic curve signature scheme used by commonware for peer-to-peer
identity. Ed25519 keys are 32 bytes (private) / 32 bytes (public). It is fast (tens of
thousands of signatures per second), has small signatures (64 bytes), and is widely used in
non-Ethereum systems (SSH, TLS, Cosmos chains, Solana). Daeji validators and secondary
peers are identified by their Ed25519 public key.

**secp256k1** -- the elliptic curve used by Ethereum for transaction signing. secp256k1
keys are 32 bytes (private) / 33 bytes (compressed public) / 65 bytes (uncompressed
public). Ethereum addresses are derived from the public key: `keccak256(uncompressed_pubkey)[12..]`
(the last 20 bytes of a Keccak-256 hash). All EVM transaction signatures use secp256k1.

These are fundamentally different curves. You cannot use an Ed25519 key to sign an Ethereum
transaction, and you cannot use a secp256k1 key to authenticate to the commonware P2P
overlay.

### Options

**Option A: One Ed25519 key, derive secp256k1 from it.** The agent has a single Ed25519
keypair as its canonical identity. A secp256k1 signing key is deterministically derived
from the Ed25519 private key (e.g., via HKDF key derivation). Pros: single key to manage,
single identity concept. Cons: the derivation is non-standard (no existing library does
Ed25519 -> secp256k1 derivation), it creates a dependency between the two key types (if
the Ed25519 key is compromised, the derived secp256k1 key is also compromised), and it may
confuse developers who expect Ethereum-standard key management.

**Option B: Separate keys for P2P and EVM.** The agent holds two independent keypairs:
Ed25519 for P2P identity (connecting to the daeji overlay, authenticating to other peers)
and secp256k1 for EVM transactions (signing transactions, deriving Ethereum address). Pros:
standard tooling works for both (commonware key tools for Ed25519, `cast wallet new` for
secp256k1), no non-standard derivation. Cons: two keys to manage, two identities to link
(the AgentRegistry contract would store both: `ed25519Pubkey` and `msg.sender` address).

**Option C: secp256k1 for everything.** Use only secp256k1. For P2P, wrap the secp256k1
key to work with commonware-p2p (which expects Ed25519). This would require modifying
commonware's P2P layer or adding an adapter. Pros: single key type, standard Ethereum
tooling. Cons: requires non-trivial commonware modifications, goes against the library's
design.

### What This Means for Roko

The practical impact is on `roko.toml` and the `roko agent create` command. Today, creating
an agent is purely local config. With cryptographic identity, `roko agent create` would
need to generate keys, register on-chain via `IdentityRegistry.sol`, and store the keys
somewhere persistent. The `AlloyChainWallet` in `roko-chain` already handles secp256k1 --
Option B would add an Ed25519 key alongside it, with the `AgentRegistry.sol` contract
linking the two on-chain.

### Recommendation (not yet decided)

Lean toward Option B (separate keys). It is the most standard approach, uses each library's
native key type, and avoids non-standard derivation schemes. The AgentRegistry contract
links the two identities on-chain.

---

## 4. Secondary Peer vs RPC Client

### Why This Matters

Roko agents can connect to daeji in two ways: as an **RPC client** (sends JSON-RPC
requests to a daeji node's HTTP endpoint, like any Ethereum application) or as a
**secondary peer** (joins the P2P network as a read-only participant, replicating all
blocks locally). The choice affects latency, trust assumptions, resource usage, and what
capabilities are available.

### What Roko's Orchestration Loop Tolerates

The key timing question is: how fast does roko need to learn about chain events?

Roko's orchestration loop is not latency-sensitive in most cases. A single task dispatch
takes 30 seconds to 5 minutes (agent thinks, writes code, runs tests). The gate pipeline
(compile + test + lint + diff) takes another 10-30 seconds. Total per-task cycle: 1-6
minutes. Within this cycle, chain interactions happen at two points:

1. **Pre-dispatch**: query the neuro store / chain for relevant knowledge. Latency budget:
   ~1 second is fine (already dominated by system prompt assembly).
2. **Post-gate**: anchor the episode witness on-chain. Latency budget: 5-10 seconds is fine
   (this is a background operation after the task is validated).

Neither of these requires sub-second finality notification. RPC polling at 2-second
intervals would be invisible within roko's task cycle.

Where sub-second matters: **real-time agent coordination** -- if two agents need to
synchronize state (e.g., "agent A posted a knowledge entry, agent B should see it before
its next dispatch"), the 1-10 second polling delay adds up. The `ProcessSupervisor` in
`roko-runtime` manages agent lifecycles but does not do real-time cross-agent coordination
today. `PlanRunner` tracks processes via PIDs and handles graceful shutdown with
cancellation tokens -- none of this needs chain awareness.

### What a Secondary Peer Is

In commonware's network model, there are two participant types:

- **Validators**: hold DKG key shares (shares of the collective signing key, generated
  during the Distributed Key Generation ceremony), participate in BFT voting, propose and
  sign blocks. Disrupting a validator affects chain liveness.

- **Secondary peers**: hold an Ed25519 keypair for P2P authentication, receive all block
  data and finality messages, but never vote or sign anything. They are read-only observers
  with cryptographic proof of what they observe. A secondary peer cannot disrupt the chain.

### Trade-offs

| Concern | RPC Client | Secondary Peer |
|---|---|---|
| Setup | Point at a URL; no additional process | Run a subprocess, manage its lifecycle |
| Resource usage | Negligible (HTTP requests) | ~500MB-1GB RAM + CPU for block verification |
| Block discovery latency | Poll-based: 1-10 seconds depending on poll interval | Push-based: milliseconds (P2P message delivery) |
| Trust model | Trust the RPC node to return correct data | Verify blocks locally (same verification code as validators) |
| Merkle proofs | Not directly available (would need a custom RPC method) | Direct QMDB access enables proofs against any historical state root |
| State queries | JSON-RPC serialization overhead per query | Direct in-process state reads (no network round-trip) |

### Decisions Needed

**When is a secondary peer worth the overhead?** For roko's current single-operator devnet
workflow, RPC is sufficient. A secondary peer would matter when: (a) multiple roko instances
on different machines need to verify each other's chain claims without trusting a shared RPC
node, or (b) Merkle proofs are needed for cross-chain certificate verification.

**Resource budget.** The `ProcessSupervisor` in `roko-runtime` already manages child
processes (Claude CLI subprocesses). Adding a secondary peer as another managed subprocess
is architecturally natural -- `ProcessSupervisor::spawn()` handles lifecycle, cancellation,
and cleanup. But 500MB-1GB RAM per peer is significant when roko might be running 5+ agent
processes simultaneously.

**Phase recommendation.** Start with RPC client (Phase 1 and 2). Add secondary peer in
Phase 3 only for specific use cases (Merkle proof generation, sub-second finalization
notification for time-sensitive agent coordination).

---

## 5. Precompile vs Contract Boundary

### Why This Matters

A **smart contract** is Solidity code deployed to the EVM, executed as interpreted bytecode,
and metered by gas (a unit of computational cost -- each EVM opcode has a gas price, and
transactions specify a gas limit). A **precompile** is native Rust code registered at a
fixed EVM address, called like a contract but executing as compiled machine code. Precompiles
bypass the EVM interpreter entirely.

The question is: which operations should be precompiles (fast, native, but require modifying
the daeji source code and redeploying the chain) vs contracts (slower, gas-metered, but
deployable by anyone at any time without chain modifications)?

### The HDC Search Case

The HDC (Hyperdimensional Computing) precompile is the reason this question exists.
`roko-primitives::HdcVector` stores 10,240 bits as `[u64; 160]` (1,280 bytes). The core
operation is Hamming distance -- counting differing bits between two vectors. This is the
basis for semantic similarity search in roko's neuro store.

Performance comparison:

- **In Solidity**: each POPCNT-equivalent operation on 256-bit words requires ~10 EVM
  opcodes. A single 10,240-bit comparison = 40 words = ~400 opcodes = ~3,200 gas. Scanning
  10,000 entries = 32,000,000 gas (exceeds the 30M block gas limit).
- **As a precompile**: native Rust with AVX-512 SIMD compares all 10,000 entries in ~17
  microseconds. Fixed gas cost: 50,000 (a policy number, not reflecting actual computation).

Everything else in roko's chain integration works fine as standard Solidity contracts. Agent
registration, knowledge posting, heartbeats, reputation scoring, witness anchoring -- all of
these are simple CRUD operations that the EVM handles efficiently. The HDC search is the
only operation where the EVM's computational limits are a hard blocker.

However, the precompile is also the most complex chain modification -- it requires
maintaining a separate in-memory index rebuilt from contract state at every finalized block.

### Decisions Needed

**At what entry count does the contract approach break?** Estimate: fewer than ~1,000
entries can be searched via tag-matching (not HDC, just category-based filtering) in a
contract at reasonable gas cost. Above ~10,000 entries, even off-chain HDC search with
event log scanning becomes slow (must download and decode thousands of events). The
precompile is needed somewhere in the 1,000-10,000 range.

**Timeline to 10,000 entries.** If the system will not reach 10,000 knowledge entries for
months, defer the precompile and build the contract version first. The entry generation
rate depends on how many agents are running and how frequently they produce knowledge -- a
single developer running 5 agents might generate 10-50 entries per day.

**Upgrade path.** Can the system deploy a contract first and swap to a precompile later
without changing the caller interface? Yes, if either: (a) a proxy pattern is used (the
contract delegates to an upgradeable implementation), or (b) the precompile is deployed at
the same address the contract occupied (requires a chain hard fork -- all validators
upgrade simultaneously). The proxy pattern is simpler and does not require a hard fork.

---

## 6. Devnet vs Testnet vs Production

### Why This Matters

The chain environment determines trust assumptions, persistence requirements, and
operational complexity. The progression from devnet to production is not just "deploy more
nodes" -- it changes the DKG ceremony type, state persistence strategy, and who controls
the validator set.

### How Roko Currently Deploys

Roko has a mature deployment story that does not involve daeji at all today:

- **`roko serve`**: starts the HTTP control plane on port 6677 with ~85 REST routes + SSE +
  WebSocket. This is the main runtime for dashboards and external callers.
- **`roko daemon start/stop/status/logs/install`**: manages roko as a background daemon.
- **`roko deploy railway/fly/docker`**: cloud deployment to Railway, Fly.io, or Docker. The
  worker image is `ghcr.io/nunchi-trade/roko-worker:latest`.

Daeji would add another process to manage alongside these. In devnet mode, the typical
setup would be: roko serve (HTTP API) + 4 daeji validator nodes (Docker containers) + roko
agents (Claude CLI subprocesses). All on one machine.

### Definitions

- **Devnet**: all nodes run on one developer's machine (Docker containers or native
  processes). State is ephemeral -- `just devnet-reset` wipes everything and starts fresh.
  DKG uses trusted-dealer mode (one process generates the signing key and distributes
  shares). 4 validators, all controlled by one operator.

- **Testnet**: nodes distributed across multiple machines (cloud instances or physical
  servers), but still controlled by the development team. State should persist across
  restarts. DKG must be interactive (Joint-Feldman protocol, where each validator
  contributes randomness and verifies others' contributions -- no single party controls the
  key). Multiple operators may participate.

- **Production**: open validator set where independent operators run validators with real
  economic stake. Requires: slashing (validators who misbehave lose staked tokens), key
  rotation, monitoring, and incident response.

### Decisions Needed

**Who runs validators?** Currently devnet only (all local). When does the testnet need
distributed validators? This determines when the interactive DKG ceremony must be wired
(it exists in commonware but requires coordinated startup across machines).

**When does the devnet need persistent state?** Currently, every `devnet-reset` wipes all
chain history: deployed contracts, posted knowledge entries, witness anchors, agent
registrations. This is fine during initial development. At some point, accumulated knowledge
has enough value that wiping it is costly. When that point arrives, the devnet needs
durable state across restarts (persist QMDB to disk, preserve DKG key shares across
restarts). The threshold is roughly: when the neuro store has hundreds of confirmed entries
that took real agent-hours to produce.

**DKG ceremony for testnet.** The trusted-dealer mode is convenient for local dev but
inappropriate for a distributed testnet because it requires trusting the dealer process
(whoever runs it knows the full signing key). Interactive DKG (Joint-Feldman) eliminates
this trust requirement but needs all validators to be online simultaneously during the
ceremony. Options: use `commonware-deployer` (an AWS EC2 orchestration tool that
automates distributed startup) or manual coordination via SSH.

---

## 7. Chain Modifications to Daeji

### Why This Matters

Some features require changing daeji's source code (the blockchain node binary), not just
deploying contracts on top of it. Each source modification increases maintenance burden,
creates a divergence from upstream commonware examples, and requires all validators to
upgrade simultaneously (a "hard fork").

### What Roko Workflows Depend On

Several roko contracts use `block.timestamp` directly:

- `IdentityRegistry.sol`: `PROMPT_UPDATE_DELAY = 1 days`, `WITHDRAW_COOLDOWN = 7 days`,
  stake cooldown checks (`block.timestamp < stakeData.cooldownEndsAt`)
- `ReputationRegistry.sol`: decay calculations
  (`halvings = (block.timestamp - lastUpdate) / DECAY_PERIOD`)
- `BountyMarket.sol`: deadline enforcement (`if (deadline <= block.timestamp) revert`)
- `InsightBoard.sol`: posting timestamp (`postedAt: uint64(block.timestamp)`)
- `WorkerRegistry.sol`: liveness tracking (`lastUpdated: uint64(block.timestamp)`)
- `ValidationRegistry.sol`: work proof timestamps
- `ConsortiumValidator.sol`: randomness seed fallback

Every one of these breaks if `block.timestamp` is not wall-clock time (see Gap Analysis
doc 08 for the daeji bug where timestamp equals block height).

### Specific Modifications Under Consideration

**Custom precompiles** -- modifying `RevmExecutor` (daeji's REVM integration layer, the
code that connects the EVM execution engine to daeji's consensus and state) to register
new precompile functions at fixed addresses. This is the mechanism for adding HDC search
(0x09), QMDB proofs (0x0B), and BTLE encryption (0x0C).

**Extended block header** -- the original design documents proposed adding custom fields to
each block's header: `sm_root` (a separate Merkle root over all knowledge entries),
`active_agents` (count of registered agents with recent heartbeats), `insight_count` (total
knowledge entries posted). The alternative is tracking these values in smart contract state,
which is already Merkle-authenticated by QMDB and queryable via `eth_call`. Adding custom
header fields breaks compatibility with standard Ethereum tooling that expects the standard
header format.

**BTLE support** -- requires: exporting the VRF seed (threshold signature output) for each
finalized view in a way that client-side encryption code can use, and optionally an
encryption/decryption precompile (0x0C) for in-EVM BTLE operations.

**Historical state proofs** -- a new RPC method that queries QMDB for a Merkle
inclusion/exclusion proof at a specified historical block number. Standard `eth_` RPC does
not support this -- `eth_getProof` exists but only for the latest state, not historical.

**Custom kora_ RPC methods** -- adding agent-specific query methods beyond the existing
`kora_nodeStatus`: `kora_activeAgents`, `kora_recentKnowledge`, `kora_vrfSeed`. These are
convenience methods that wrap complex `eth_call` and `eth_getLogs` patterns into single
RPC calls.

### The Fork vs Upstream Question

Should modifications live in a fork of daeji (a separate branch with custom changes), or
be designed for upstream contribution (submitted back to the main daeji repository)?

If daeji is our own repository (which it is -- `github.com/Nunchi-trade/daeji`), the
distinction is about code organization, not contribution policy:

- **Fork approach**: modifications are interleaved with base chain code. Faster to
  implement. Harder to pull upstream commonware updates. Risk of the base chain and agent
  extensions becoming entangled.

- **Clean separation**: agent-specific code lives in clearly delineated modules
  (`crates/node/executor/src/precompiles/`, `crates/node/rpc/src/kora_agent.rs`). The base
  chain code remains unmodified. Upstream commonware updates can be integrated by bumping
  dependency versions. Slower to implement initially, but much easier to maintain long-term.

### Recommendation (not yet decided)

Lean toward clean separation. Keep all agent-specific extensions in dedicated modules.
Defer header modifications entirely (use contract state instead). Add precompiles and RPC
methods only when contract-based alternatives are measurably insufficient.

---

## 8. Cross-Chain Certificate Usage

### Why This Matters

Daeji's consensus produces ~240-byte finality certificates: a 48-byte BLS12-381 group
public key + a 96-byte threshold signature over the block hash + metadata (chain ID, block
number, state root). Any system that knows daeji's 48-byte group public key can verify
that a specific block was finalized by checking the threshold signature. This is
dramatically smaller than other chains' finality proofs (Ethereum sync committees: ~100KB;
Cosmos IBC: full light client state).

### What Roko Would Certify Cross-Chain

The two primary cross-chain payloads are:

1. **Gate verdicts**: the output of roko's gate pipeline (compile pass/fail, test count,
   lint warnings, diff review score). Today these are stored locally in episodes. If
   anchored on daeji and certified cross-chain, an external system could verify "agent X
   passed all gates for task Y" without trusting roko's operator. The witness engine in
   `crates/roko-chain/src/witness.rs` already anchors `blake3(episode_data)` as a 32-byte
   hash on-chain.

2. **Knowledge entries**: a confirmed neuro store entry could be certified to another chain,
   enabling cross-fleet knowledge sharing where fleet B verifies that fleet A's knowledge
   was genuinely produced and confirmed, not fabricated.

### Decisions Needed

**Who verifies certificates?** Two consumers:

- **Client-side verification (Rust code in roko)**: an agent running locally verifies
  a certificate using the BLS12-381 pairing check. This is straightforward -- the
  commonware-cryptography crate provides the verification function. Use case: a roko agent
  on one machine verifies that something happened on daeji without trusting any RPC node.

- **On-chain verification (Solidity contract on another EVM chain)**: a smart contract on
  Ethereum L1 (or another chain) verifies the certificate. This requires BLS12-381 pairing
  operations in the EVM. EIP-2537 is an Ethereum improvement proposal that adds BLS12-381
  precompiles to the EVM, making verification gas-efficient. Some testnets support it;
  mainnet support is pending. Without EIP-2537, BLS verification in pure Solidity is
  extremely expensive (~2M+ gas per verification).

**Certificate relay.** Who moves certificates from daeji to the target chain? Options:

- A dedicated relayer process that watches daeji for new finalized blocks and submits
  certificates to the verifier contract on the target chain. Simplest operationally.
- Part of the roko agent -- the agent itself relays certificates when it needs to prove
  something cross-chain. Simplest from a dependency perspective.
- Part of the orchestrator -- certificates are relayed as part of the post-task flow
  (after anchoring an episode witness on daeji, relay the certificate to L1 for external
  verifiability).

**Which chains to target?** Candidates:

- **Ethereum L1**: the most valuable target (proving daeji state to Ethereum). Requires
  EIP-2537 support for gas-efficient verification.
- **Other commonware chains**: native BLS12-381 verification (same crypto stack). Trivial
  to support but requires another commonware chain to exist and be useful.
- **Mirage-rs (chain ID 88888)**: the in-process EVM fork simulator already used by roko.
  Useful for testing the certificate flow without deploying to a real L1. Does not require
  EIP-2537 because mirage-rs can add custom precompiles.

---

## 9. Relationship to roko-chain Crate

### Why This Matters

The `roko-chain` crate is the existing Rust crate in roko's workspace that contains chain
client traits, an alloy-backed implementation, and witness anchoring logic. Its current
status is "built but not wired into the runtime" -- the code exists, passes unit tests, but
nothing in the main execution loop (`orchestrate.rs`) instantiates or calls it.

How this crate is scoped determines whether the chain integration is portable (works with
any EVM chain) or daeji-specific (tightly coupled to daeji's features).

### What roko-chain Actually Contains Today

The crate is substantial. Here is the actual module list from
`crates/roko-chain/src/lib.rs`:

| Module | Purpose |
|---|---|
| `client` | `ChainClient` trait: read-only RPC (block_number, get_block_header, get_receipt, get_logs, get_storage_at, eth_call, get_balance, chain_id) |
| `wallet` | `ChainWallet` trait: sign_and_submit, wait_for_receipt, nonce, balance |
| `alloy_impl` | `AlloyChainClient` (wraps alloy `DynProvider`) + `AlloyChainWallet` (wraps alloy `PrivateKeySigner`). Works with any JSON-RPC endpoint. |
| `mock` | `MockChainClient` + `MockChainWallet` -- in-memory test doubles |
| `witness` | `ChainWitnessEngine` -- anchors `blake3(episode)` on-chain, verifies receipts |
| `agent_registry` | Agent Registry with soulbound ERC-721 passports |
| `reputation_registry` | 7-domain EMA reputation scoring |
| `validation_registry` | Work proof + validator attestation |
| `korai_token` | KORAI token with lazy demurrage + emission schedule |
| `marketplace` | Spore job marketplace with escrow and 3 hiring models |
| `futures_market` | Prediction market primitives |
| `heartbeat_ext` | Chain heartbeat extension, policy cage |
| `observer` | Block observer, event filtering |
| `collusion` | Collusion ring detection via graph clique analysis |
| `trace_rank` | PageRank-style reputation propagation |
| `nelson_siegel` | Yield curve model for DeFi oracle rates |
| `isfr` | ISFR registry |
| `gate` | MEV detection gate, wallet gate, tx simulation gate |
| `triage` | Event triage pipeline |
| `tools` | 10 chain domain DeFi tool definitions |
| `x402` | HTTP 402 micropayment protocol with state channels |
| `phase2` | Phase 2+ types |

The key concrete types:

```rust
// AlloyChainClient -- works with ANY EVM chain (daeji, Anvil, Ethereum)
pub struct AlloyChainClient {
    provider: Arc<DynProvider>,  // alloy's universal provider
    name: String,
}

// AlloyChainWallet -- holds a secp256k1 private key
pub struct AlloyChainWallet {
    provider: Arc<DynProvider>,
    address: Address,
    chain_id: u64,
    name: String,
}

// ChainWitnessEngine -- anchors attestation hashes
pub struct ChainWitnessEngine;
// witness_tx_request sends: "roko.attestation.witness:" + blake3(episode) to address 0x...c0
```

The witness anchoring flow:
1. Compute `blake3(episode_data)` as a 32-byte hash
2. Build a transaction: `to = 0x...c0`, `data = "roko.attestation.witness:" + hash`
3. `wallet.sign_and_submit(tx)` broadcasts it
4. `wallet.wait_for_receipt(tx_hash, 30_000ms)` confirms inclusion
5. Receipt metadata (chain_id, tx_hash, block_number) is stored on the attestation

### Technical Background

**Alloy** is the Rust library for interacting with Ethereum-compatible chains. It provides
type-safe bindings for JSON-RPC methods, transaction construction, ABI encoding/decoding,
and contract interaction. It is the successor to **ethers-rs**, which is in maintenance
mode. Roko already depends on alloy for contract deployment scripts.

The distinction matters because: if roko-chain uses only standard alloy types (providers,
signers, contract calls), it works with any EVM chain. If it uses daeji-specific features
(kora_ RPC methods, commonware types), it becomes daeji-coupled.

### Options

**Option A: Generic chain client.** `roko-chain` defines traits (`ChainClient`,
`ChainWallet`) and provides alloy-backed implementations that work with any EVM chain
(daeji, Ethereum, Anvil, Hardhat, any node). Daeji-specific features (kora_ RPC methods,
secondary peer management) live in a separate `roko-chain-daeji` crate or a feature-gated
module. Pros: portable, testable against local Anvil. Cons: daeji-specific features require
a second crate or module.

**Option B: Daeji-specific crate.** `roko-chain` is explicitly the daeji integration
crate. It uses daeji-specific RPC methods, manages the secondary peer lifecycle, and
depends on commonware types. Pros: simpler code organization (one crate, one purpose).
Cons: cannot be tested against non-daeji chains, couples roko to daeji.

**Option C: Generic trait + daeji implementation.** `roko-chain` defines generic traits.
An `AlloyChainClient` implementation (already exists) works with any EVM chain. A
`DaejiChainClient` implementation extends it with kora_ methods and secondary peer support.
The orchestrator uses the trait; the concrete implementation is chosen by config. Pros: best
of both worlds. Cons: most code to write and maintain.

### Related: Witness Anchoring Format

What exactly gets anchored on-chain? The `ChainWitnessEngine` currently anchors
`blake3(episode_data)` as a 32-byte hash in transaction calldata. Should it also anchor:
- Gate results (compile success/failure, test pass count, lint warnings)?
- Knowledge entry hashes (which entries were produced by this task)?
- Plan metadata (which plan, which task index, which agent)?

More data per anchor = richer verifiability but higher gas cost per transaction.

### Recommendation (not yet decided)

Lean toward Option C (generic trait + daeji implementation). The `AlloyChainClient` already
exists and implements generic traits. Add a `DaejiClient` that wraps it with kora_-specific
methods. For witness anchoring, start with episode hash only (minimal gas cost, maximum
simplicity) and expand later.

---

## 10. Commonware Version Tracking

### Why This Matters

Daeji depends on `commonware-*` crates (cryptography, consensus, P2P, storage, runtime,
codec) at version `2026.4.0`. If roko adds its own direct dependencies on commonware crates
(for Ed25519 agent identity, QMDB local storage, deterministic test runtime), both
repositories depend on the same libraries. Version mismatches between the two repositories
can cause type incompatibilities (e.g., roko generates an Ed25519 key with commonware
v2026.5.0, but daeji at v2026.4.0 cannot deserialize it because the wire format changed).

### How Roko Manages Dependencies Today

Roko uses a standard Cargo workspace (`Cargo.toml` at the workspace root) with 18 crate
members plus several apps (`mirage-rs`, `agent-relay`, `roko-chain-watcher`). Dependencies
are pinned via `Cargo.lock`. The workspace includes `roko-chain` as a member:

```toml
members = [
    # ...
    "crates/roko-chain",
    # ...
    "apps/roko-chain-watcher",
]
```

Currently, roko does not depend on any `commonware-*` crate directly. All chain interaction
goes through alloy (standard Ethereum JSON-RPC). If roko adds commonware dependencies for
Ed25519 keys or secondary peer integration, those would be added to `roko-chain/Cargo.toml`
and pinned in the workspace `Cargo.lock`.

The risk: when commonware releases a new version, daeji and roko must update in lockstep
if they share types that cross the boundary (keys, codec formats, proofs). With separate
repos and separate `Cargo.lock` files, nothing enforces this.

### Technical Background

**Commonware** is a Rust library of composable blockchain primitives. It is explicitly
alpha-stability: the API may change between versions. Version numbers follow a
`YYYY.MINOR.PATCH` scheme. The crates are: `commonware-cryptography` (Ed25519, BLS12-381,
VRF), `commonware-p2p` (authenticated networking), `commonware-consensus` (Simplex BFT),
`commonware-storage` (QMDB, MMR), `commonware-runtime` (tokio + deterministic runtimes),
`commonware-codec` (binary serialization), and others.

### Decisions Needed

**Must roko and daeji use the exact same commonware version?** For types that cross the
boundary (Ed25519 public keys registered on-chain, codec-encoded messages exchanged over
P2P, QMDB proofs verified client-side), yes -- the wire format must match. For types used
only internally (e.g., roko uses `commonware-runtime::deterministic` for tests but never
exchanges runtime types with daeji), version alignment is less critical.

**Should daeji become a workspace member of the roko monorepo?** Currently, daeji is a
separate repository (`github.com/Nunchi-trade/daeji`). Options:

- **Separate repos, version-aligned deps**: both repos pin the same commonware version in
  their respective `Cargo.toml`. Coordination is manual (update both when bumping
  commonware). Risk: version drift if someone updates one repo but not the other.

- **Monorepo (daeji as workspace member)**: both live in one Cargo workspace. A single
  `Cargo.lock` ensures version alignment. Pros: impossible to drift. Cons: daeji becomes
  coupled to roko's build system, CI, and release cycle. Harder to develop daeji
  independently.

- **Git submodule / subtree**: daeji is pulled into roko's repo as a submodule or subtree.
  A middle ground: daeji can be developed independently, but roko pins a specific commit.
  Pros: explicit version pinning. Cons: submodule workflows are notoriously error-prone.

**How to handle commonware breaking changes?** Since commonware is alpha-stability, breaking
API changes are expected. When commonware releases v2026.5.0 with breaking changes:

- Both repos must be updated together if they share types across the boundary.
- If only one repo is updated, the other's types become incompatible (e.g., daeji produces
  blocks with new codec format, roko's secondary peer cannot deserialize them).
- This argues for either: a monorepo (single update), or a disciplined version-bump
  protocol (update both repos in the same PR/day, with CI that tests them together).

### Recommendation (not yet decided)

Lean toward separate repos with a shared CI job that tests them together at the pinned
commonware version. Add a `version-check` CI step to both repos that fails if the
commonware version drifts. Avoid monorepo coupling -- daeji should be usable independently
of roko.
