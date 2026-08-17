# Gap Analysis: Original Design Assumptions vs Daeji Reality

## Purpose of This Document

This document catalogs technical assumptions made during an earlier design phase (the "agent-chain" architecture documents) that no longer hold now that the real blockchain -- daeji -- exists and is running. For each gap, we explain what the concept is, what was assumed, what daeji actually does, why it matters, and how to fix it.

### Background for the Cold Reader

**Roko** is a Rust toolkit (18 crates, ~177K lines of code) that orchestrates AI coding agents. It reads a product requirements document (PRD), generates a task plan as a TOML file, dispatches LLM-powered agents (Claude, Codex, Gemini, Ollama, etc.) to execute those tasks, validates each task's output through a "gate pipeline" (compile, test, lint, diff review), and persists what was learned to a local knowledge store. The core execution loop lives in `crates/roko-cli/src/orchestrate.rs` (~11,000 lines). Roko is used to develop itself -- it reads its own PRDs, generates its own plans, and runs agents to implement them.

The self-hosting loop works end-to-end today: `roko prd idea` -> `roko prd draft` -> `roko prd plan` -> `roko plan run` -> gate validation -> knowledge extraction -> `roko dashboard` for monitoring. All 18 crates compile and test. The HTTP control plane (`roko serve`) exposes ~85 routes on port 6677. The interactive TUI (`roko dashboard`) has 7 tabs (F1-F7).

**Daeji** (internal codename "Kora") is a minimal blockchain node that executes Ethereum-compatible smart contracts. It is built from scratch using composable Rust primitives from the [commonware](https://github.com/commonwarexyz/monorepo) library, not forked from any existing Ethereum client. It runs real multi-node consensus (called "simplex BFT") with BLS12-381 threshold signatures across 4 validator nodes.

**The integration goal** is to wire roko's agent execution loop to daeji so that: (1) agent knowledge can be shared across agent fleets via on-chain contracts, (2) completed work is tamper-evident via on-chain hash anchoring, and (3) novel cryptographic features (verifiable randomness, sealed commitments) become available to agents.

**The roko-chain crate** (`crates/roko-chain/`) is roko's existing chain integration layer. It contains `ChainClient` (read-only RPC trait), `ChainWallet` (transaction signing trait), `AlloyChainClient` and `AlloyChainWallet` (alloy-backed JSON-RPC implementations), `ChainWitnessEngine` (attestation anchoring), plus in-memory implementations of agent registry, reputation registry, KORAI token with demurrage, job marketplace, and more. All of this code compiles and passes unit tests, but nothing in the main execution loop (`orchestrate.rs`) instantiates or calls it. The chain config exists in `roko.toml`:

```toml
[chain]
rpc_url = "http://127.0.0.1:8545"
chain_id = 31337
wallet_key = "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80"
agent_registry = "0x9fE46736679d2D9a65F0992F2272dE9f3c7fa6e0"
bounty_market = "0xDc64a140Aa3E981100a9becA4E685f962f0cF6C9"
```

**The agent-chain documents** were a set of design specifications written before daeji existed. They described a hypothetical custom blockchain for agent knowledge sharing, explicitly choosing commonware simplex as the consensus layer. Daeji implements the base layer of that vision -- but not everything maps cleanly. This document catalogs every assumption that broke.

---

## Background Concepts

Before reading the gap analysis, you need to understand several blockchain concepts that the original design documents relied on. Each is explained here from scratch.

### Merkle Patricia Trie (MPT)

Standard Ethereum stores all state (every account balance, every contract's storage, every piece of deployed code) in a data structure called a **Merkle Patricia Trie** (MPT). A trie is a tree-shaped data structure where data is stored at leaf nodes and each interior node's hash is computed from its children's hashes. This means the single hash at the root of the tree (the "state root") is a cryptographic commitment to the entire state -- if any value anywhere in the tree changes, the root hash changes too.

The critical capability this enables is **Merkle proofs**: given a key-value pair (say, "account 0xABC has balance 100 ETH"), you can produce a proof -- roughly 1KB of intermediate hash values from the leaf to the root -- that demonstrates this key-value pair is in the tree. The verifier only needs the root hash (32 bytes, included in every block header) and the proof. They do not need the rest of the tree. The standard Ethereum JSON-RPC method `eth_getProof` generates these proofs.

### QMDB Transition Hash

Daeji does not use MPT. Its state database is called **QMDB** (Quick Merkle Database), from the commonware library. QMDB produces a "state root" at every block, but it computes it differently:

```
keccak256(b"_KORA_STATE_TRANSITION_ROOT" + parent_root + serialized_changes)
```

This formula takes the parent block's root and the serialized list of state changes in the current block, and hashes them together. The result is deterministic (two nodes processing the same blocks in the same order get the same root) and fast to compute. But it is NOT a Merkle tree root. You cannot produce a Merkle proof from it. There is no `eth_getProof`. You cannot prove "key K had value V at block N" to someone who only has the root hash.

### EIP-1559

**EIP-1559** is Ethereum's fee market mechanism, introduced in August 2021. Before EIP-1559, users bid a gas price and miners chose the highest-paying transactions. EIP-1559 replaced this with a two-part fee:

- **`base_fee`**: A per-gas price that adjusts automatically based on how full recent blocks are. If blocks are more than 50% full, the base fee increases; if less than 50% full, it decreases. The base fee is **burned** (destroyed), not paid to validators.
- **`priority_fee`** (also called "tip"): An optional per-gas tip paid to the block validator to incentivize inclusion.

The economic effect is spam prevention: submitting transactions costs real money (the base fee), and the cost rises automatically when the network is congested.

### block.timestamp

In standard Ethereum, `block.timestamp` is a Unix timestamp -- the number of seconds since January 1, 1970 (UTC). Smart contracts use it for time-dependent logic: cooldown periods ("you can call this function again after 24 hours"), timelocks ("funds are locked until January 2027"), and decay calculations ("this entry's weight halves every 72 hours").

### BLOCKHASH Opcode

The EVM (Ethereum Virtual Machine) has an instruction called `BLOCKHASH` that returns the hash of a recent block. Given a block number N (up to 256 blocks in the past), `BLOCKHASH(N)` returns the 32-byte hash of block N. Smart contracts use this for:
- On-chain randomness derivation (hashing the block hash with other values)
- Verifying that a transaction was included in a recent block
- Various DeFi protocols that reference recent chain history

### Precompile

A **precompile** (short for "precompiled contract") is native code that lives at a fixed EVM address. When a smart contract calls a precompile address, the EVM does not execute bytecode -- instead, it runs a native function (written in Rust, Go, C, etc.) that is compiled into the node software.

Standard Ethereum has precompiles at addresses 0x01 through 0x09 for expensive cryptographic operations:
- 0x01: `ecrecover` (recover signer from ECDSA signature)
- 0x02: `sha256` hash
- 0x03: `ripemd160` hash
- 0x04: `identity` (copy input to output)
- 0x05: `modexp` (modular exponentiation)
- 0x06-0x08: BN256 elliptic curve operations
- 0x09: Blake2 hash

Adding custom precompiles requires modifying the blockchain node's EVM implementation -- you are adding native code to the execution engine, which means all validators must run the same modified code.

### REVM

**REVM** (Rust Ethereum Virtual Machine) is a standalone Rust implementation of the Ethereum Virtual Machine. It is the same EVM engine used by Foundry (the Solidity development toolkit) and Reth (a Rust Ethereum client). Daeji uses REVM to execute smart contract bytecode. When we talk about "modifying the executor" or "registering precompiles," we mean changing how daeji configures and invokes REVM.

---

## Gap 1: State Root Model (No Merkle Proofs)

### What Was Assumed

The original design specified an Ethereum-style Merkle Patricia Trie (MPT) state root. It also envisioned a separate "Superposition Memory root" (`sm_root`) -- a BLAKE3 Merkle root over all knowledge entries, sorted by entry ID. This root would be included in every block header.

The design assumed that HDC search results (HDC = Hyperdimensional Computing, a technique for encoding text as large binary vectors for fast similarity search) would include Merkle inclusion proofs against `sm_root`, allowing any observer to cryptographically verify that a search result was genuine.

### What Daeji Actually Does

Daeji uses QMDB, which computes state roots via a transition hash:

```rust
keccak256(b"_KORA_STATE_TRANSITION_ROOT" + parent_root + serialized_changes)
```

This is NOT a trie root. Two nodes with different physical states but the same change history produce the same root. There is no `eth_getProof` RPC method. You cannot generate Merkle inclusion proofs against this root.

### What Roko Code Is Affected

Two roko workflows would benefit from Merkle proofs:

1. **Knowledge verification across fleets.** When roko fleet A sends a knowledge entry to fleet B, fleet B currently has no way to verify the entry was actually posted to the chain (other than querying the RPC node, which requires trusting it). With Merkle proofs, fleet B could verify the entry exists in the chain state using only the block header's state root + a proof. The `roko-neuro` knowledge store (`crates/roko-neuro/src/knowledge_store.rs`) manages local entries but has no verification of chain-sourced entries.

2. **Cross-chain trust via certificates.** The `ChainWitnessEngine` in `crates/roko-chain/src/witness.rs` anchors episode hashes on-chain. To prove these anchors to an external chain (e.g., Ethereum L1), you need a Merkle proof that the anchor exists in daeji's state at a specific block. Without proofs, the certificate only proves the block was finalized -- not what state it contained.

**What roko currently does**: nothing. No chain verification exists in the runtime. The `AlloyChainClient` in `crates/roko-chain/src/alloy_impl.rs` does RPC calls but never generates or verifies proofs. The witness engine trusts the RPC response.

**Impact on the self-hosting loop**: low for now. The self-hosting loop (PRD -> plan -> agent -> gate -> persist) is entirely local. Chain verification only matters when multiple roko instances need to trust each other's chain claims.

### Fix Options

| Option | Description | Trade-offs |
|---|---|---|
| **A. Accept the limitation** | Skip Merkle proofs entirely. Trust RPC results. | Fine for a local devnet where you control all validators. Breaks cross-chain verification and any trust-minimized audit path. |
| **B. Deploy an on-chain MMR (Merkle Mountain Range)** | Deploy a smart contract on daeji that maintains an append-only Merkle tree of entry hashes. Proofs come from the MMR, not from QMDB state roots. Commonware ships `commonware-storage::mmr` which could serve as the reference implementation. | Adds gas cost per entry (one append per knowledge post). Proofs work but only cover entries explicitly tracked by the MMR, not arbitrary state. |
| **C. Replace QMDB state root with MPT** | Major daeji modification to swap the state backend. | Extremely high effort. Not recommended unless full Ethereum state proof compatibility is a hard requirement. |

### Recommendation

**Option B** -- deploy an MMR contract for knowledge proofs, accept the QMDB transition hash for everything else. This gives cryptographic proofs where they matter (knowledge entries, witness anchors) without requiring invasive chain modifications.

---

## Gap 2: Block Header Extension

### What Was Assumed

The original design specified extended block headers containing:

```
sm_root:        [u8; 32]   -- BLAKE3 Merkle root over all knowledge entries
active_golems:  u32        -- number of agents with a heartbeat in the last 21,600 blocks
insight_count:  u64        -- cumulative count of knowledge entries posted
```

These fields would be available to any node processing the block and to smart contracts via system-level reads.

### What Daeji Actually Does

Daeji's block structure is minimal:

```rust
pub struct Block {
    pub parent: BlockId,
    pub height: u64,
    pub prevrandao: B256,     // VRF seed from threshold consensus
    pub state_root: StateRoot,
    pub txs: Vec<Tx>,
}
```

No extension fields. Adding fields to this struct requires changing:
- The binary codec (how blocks are serialized for P2P transmission)
- The consensus protocol (all validators must agree on the encoding)
- The P2P wire format (message sizes change)
- The RPC layer (new fields must be returned)
- The indexer (if one exists)

### What Roko Code Is Affected

Roko would use these header fields for two purposes:

1. **Episode summaries in headers.** The orchestrator in `orchestrate.rs` writes episode data to `.roko/episodes.jsonl` after each task. If episode summaries (gate pass/fail, entry count produced) were in block headers, the TUI dashboard (`roko dashboard`, 7 tabs, ratatui-based) could display chain-level activity metrics directly. Currently the TUI reads local files.

2. **Fleet metadata.** The `roko serve` HTTP control plane exposes status endpoints. If `active_agents` were in the block header, the `/agents` endpoint could return chain-verified agent counts. Currently it returns local process counts from `ProcessSupervisor`.

**Is this critical?** No. All of this data can be queried from contract state via standard `eth_call`. The header extension is a convenience optimization, not a functional requirement. The TUI can query the `AgentRegistry.sol` contract for agent counts just as easily as it could read a header field.

**Impact on the self-hosting loop**: none. The self-hosting loop does not depend on any block header data.

### Fix Options

| Option | Description | Trade-offs |
|---|---|---|
| **A. Track in contract state** | Deploy a smart contract that maintains `active_agents`, `insight_count`, and `sm_root` as storage variables, updated per transaction. Query via standard `eth_getStorageAt` or `eth_call`. | No chain changes needed. Slightly higher gas cost (storage writes). Data is one transaction behind (updated when someone calls the contract, not automatically per block). |
| **B. Extend the Block struct** | Add optional extension fields to daeji's `Block` type. | Requires daeji source changes, updated codec, updated P2P wire format, updated RPC. Medium-high effort. |
| **C. System transaction per block** | Inject a system transaction at the end of each block that calls a fixed-address contract with block-level statistics. | Requires modifying the executor (block production logic). The contract always has fresh data. Medium effort. |

### Recommendation

**Option A** -- contract state. Simple, no chain changes required, queryable via standard Ethereum tooling. The one-transaction lag is acceptable for the agent use case.

---

## Gap 3: Custom Precompiles

### What Was Assumed

Three custom precompiles at fixed EVM addresses:
- **0x08** -- GolemRegistry: agent identity lookup, heartbeat tracking, capability queries
- **0x09** -- HDC similarity search: find knowledge entries similar to a 10,240-bit query vector
- **0x0A** -- InsightLedger: post, confirm, and challenge knowledge entries

These were designed as precompiles (native code) rather than smart contracts for performance: HDC vector math is prohibitively expensive in Solidity, and registry lookups need to be fast for real-time agent coordination.

### What Daeji Actually Does

The executor calls `ctx.build_mainnet()`, which configures REVM with only the standard Ethereum precompiles (0x01-0x09: ecrecover, sha256, ripemd160, identity, modexp, BN256 operations, Blake2). There is no mechanism to register custom precompiles without modifying daeji's source code -- specifically, the file `crates/node/executor/src/revm.rs`.

To add custom precompiles, you would need to replace `ctx.build_mainnet()` with a custom builder that includes your precompile functions in the precompile registry (a HashMap from address to function pointer in REVM).

### What Roko Code Is Affected

The only operation that truly needs native code is HDC search. The `roko-primitives` crate (`crates/roko-primitives/src/hdc.rs`) implements `HdcVector` as `[u64; 160]` (10,240 bits = 1,280 bytes) with Hamming distance, bundling, binding, and encoding operations. The `roko-neuro` crate's knowledge store uses HDC for similarity search when the `hdc` feature is enabled.

The registry and ledger are simple CRUD operations. Roko already has Solidity contracts that implement them:

- `AgentRegistry.sol`: minimal agent identity with capabilities, heartbeat, liveness tracking
- `IdentityRegistry.sol`: full ERC-8004 soulbound passport with 4 tiers, staking, timelocks
- `InsightBoard.sol`: knowledge posting with pheromone curation (contentHash + uri + confirmations)

These work fine as contracts. The 10,240-bit Hamming distance computation is the only operation where Solidity cannot keep up: scanning 10,000 entries at ~3,200 gas per comparison = 32,000,000 gas, which exceeds the 30M block gas limit. As a native Rust precompile with SIMD, the same operation takes ~17 microseconds.

**Impact on the self-hosting loop**: none directly. The self-hosting loop uses the local `KnowledgeStore` in roko-neuro for all knowledge queries. On-chain HDC search is a multi-fleet feature, not a self-hosting requirement.

### Fix Options

| Option | Description | Trade-offs |
|---|---|---|
| **A. Deploy as standard Solidity contracts** | Implement all three as normal smart contracts. HDC search becomes tag-based or hash-based (no vector similarity). | No chain changes. HDC similarity search is impossible in Solidity at meaningful scale (10,240-bit vector math costs billions of gas). Registry and ledger work fine as contracts. |
| **B. Modify daeji executor** | Replace `build_mainnet()` with a custom precompile registry in `revm.rs`. Register Rust functions at addresses 0x08-0x0A. | Requires daeji source changes. Straightforward in REVM (it is designed for this). All validators must run the modified code. |
| **C. Hybrid** | Deploy registry and ledger as contracts (simple CRUD, reasonable gas cost). Implement HDC search as a precompile only when the entry count exceeds ~1000 (the threshold where tag-based search breaks down). | Minimal chain changes (only the HDC precompile). Pragmatic phasing. |

### Recommendation

**Option C** -- contracts for the registry and ledger (they are simple storage operations that Solidity handles well), precompile for HDC search only when the entry count justifies the chain modification effort.

---

## Gap 4: Block Timing

### What Was Assumed

A 400ms block time. All time-based constants in the design were calibrated to this:
- Warning half-life: 450 blocks = 3 minutes
- Agent heartbeat interval: every 2,160 blocks = ~14.4 minutes
- 216,000 blocks per day

### What Daeji Actually Does

Block time depends on the simplex consensus protocol. The leader timeout is configurable (default 500ms), but actual block time includes execution time and network latency. On a local devnet with 4 validators, expect ~500ms-1s per block. On a network with real latency, it is higher.

More critically: **`block.timestamp` is set to `block.height`**, not to wall-clock Unix time. This means that in daeji, `block.timestamp` at block 1000 is `1000`, not `1714000000` (a Unix timestamp). This is either a deliberate simplification or a bug in the initial implementation.

### What Roko Code Is Affected

Roko has **8 Solidity contracts** that use `block.timestamp`. All of them break on daeji:

| Contract | How It Uses `block.timestamp` | Impact |
|---|---|---|
| `IdentityRegistry.sol` | `PROMPT_UPDATE_DELAY = 1 days` (86,400 seconds), `WITHDRAW_COOLDOWN = 7 days`, checks `block.timestamp < stakeData.cooldownEndsAt` | Cooldowns never expire (height increments by 1, not 86,400) |
| `ReputationRegistry.sol` | `halvings = (block.timestamp - lastUpdate) / DECAY_PERIOD` for EMA decay | Decay never happens (elapsed "time" is tiny) |
| `BountyMarket.sol` | `if (deadline <= block.timestamp) revert DeadlinePassed()` | Deadlines fire at the wrong time or never |
| `InsightBoard.sol` | `postedAt: uint64(block.timestamp)` | Timestamps are block heights, not dates |
| `WorkerRegistry.sol` | `lastUpdated: uint64(block.timestamp)`, liveness checks | Liveness windows are wrong |
| `ValidationRegistry.sol` | `timestamp: uint64(block.timestamp)` on work proofs | Proofs have meaningless timestamps |
| `ConsortiumValidator.sol` | `seed = bytes32(uint256(jobId + block.timestamp))` randomness fallback | Weak randomness (predictable low values) |

The roko-neuro knowledge store also has time-based decay, but that runs locally in Rust using `chrono::Utc::now()`, not `block.timestamp`. The on-chain counterpart would use the on-chain half-life constants defined in `crates/roko-neuro/src/lib.rs`:

```rust
pub const INSIGHT_HALF_LIFE_BLOCKS: u64 = 7 * BLOCKS_PER_DAY;     // ~7 days
pub const WARNING_HALF_LIFE_BLOCKS: u64 = 90;                       // ~3 minutes at 2s/block
pub const HEURISTIC_HALF_LIFE_BLOCKS: u64 = 15 * BLOCKS_PER_DAY;  // ~15 days
```

These block-based constants are calibrated for 2 seconds per block (BLOCKS_PER_DAY = 43,200). If daeji's block time is ~500ms, they would need recalibration.

The local off-chain decay formula is:
`weight = initial_weight * 0.5^(age_days / (half_life_days * tier_multiplier))`

The on-chain equivalent would need to use `block.timestamp` for the "age" calculation -- which is broken until the timestamp bug is fixed.

**Impact on the self-hosting loop**: none directly (the loop uses local time). But any chain-based knowledge sharing or witness verification that depends on timestamps will produce nonsensical results.

### Fix Options

| Option | Description | Trade-offs |
|---|---|---|
| **A. Fix daeji timestamps** | Change `BlockContext` construction in `crates/node/consensus/src/app.rs` to use `SystemTime::now().duration_since(UNIX_EPOCH).as_secs()` instead of `height`. One-line change. | Validators may have slightly different wall clocks. Simplex consensus tolerates this -- the timestamp is a field in the proposed block payload, not used for consensus decisions. Verifiers should accept timestamps within a reasonable window (e.g., plus or minus 30 seconds of local time). |
| **B. Calibrate constants per deployment** | Store `BLOCK_TIME_MS` as a contract constant. Compute all durations from it. Do not use `block.timestamp` -- use `block.number` for everything. | Works around the bug but forces all contracts to avoid a core Solidity feature. Any imported library or standard that uses `block.timestamp` is still broken. |
| **C. Use block numbers for all timing** | Interpret all durations as block counts. Half-lives, cooldowns, and timeouts are expressed in blocks, not seconds. | Simple and deterministic, but requires knowing the block time to convert to human-meaningful durations. Breaks Solidity conventions. |

### Recommendation

**Option A** (fix daeji timestamps -- it is a one-line change with low risk) combined with **Option B** (calibrate constants per deployment to handle variable block times). The timestamp should be wall-clock time for compatibility with the Solidity ecosystem.

---

## Gap 5: Gas Economics

### What Was Assumed

Meaningful gas costs that serve as anti-spam mechanisms:
- Posting a knowledge entry costs 0.5 GNOS tokens (burned)
- Burst spam incurs quadratic fee increases
- Gas-based economic disincentives prevent flooding

### What Daeji Actually Does

- `gasPrice` and `maxPriorityFeePerGas` are hardcoded to 1 gwei (0.000000001 ETH)
- `base_fee_per_gas` is always 0
- Gas is effectively free -- transactions cost nothing meaningful

There is no EIP-1559 dynamic fee market. The base fee never adjusts based on block fullness. There is no economic cost to submitting arbitrarily many transactions.

### Does Roko Need Gas Metering?

For devnet: **no.** Roko's current cost accounting is entirely off-chain. The orchestrator tracks LLM API costs in USD via `BudgetConfig`:

```rust
pub struct BudgetConfig {
    pub max_plan_usd: f64,     // $25 default (roko.toml [budget])
    pub max_task_usd: f64,     // $2 default
    pub max_session_usd: f64,
    pub warn_at_percent: u32,
}
```

These are real dollars spent on Claude/Codex API calls, not chain gas. Chain transactions (witness anchoring, knowledge posting) are a rounding error compared to LLM costs. A single Claude Opus turn costs ~$0.05-0.50. A chain transaction costs ~0.000001 ETH at daeji's gas prices. The on-chain costs are irrelevant.

For production multi-operator networks: **yes.** If independent operators run roko instances and share a chain, free gas means anyone can flood the chain. Gas metering or contract-level rate limits become necessary.

The connection to the GNOS/KORAI token concept: the in-memory `KoraiToken` in `crates/roko-chain/src/korai_token.rs` models exactly this -- 5 spending mechanisms including `ComputePurchase` and `KnowledgeAccess`. If deployed as a Solidity contract on daeji, it could enforce posting costs at the application layer even without gas-level enforcement.

**Impact on the self-hosting loop**: none. Free gas is actually beneficial for devnet -- it means chain operations add zero friction to the self-hosting cycle.

### Fix Options

| Option | Description | Trade-offs |
|---|---|---|
| **A. Accept free gas for devnet** | Implement spam prevention via contract-level logic: cooldown periods, staking requirements, rate limits (e.g., max 10 posts per 100 blocks per address). | No chain changes. Sufficient for a controlled devnet. Not viable for a public network where arbitrary addresses can submit transactions. |
| **B. Implement real EIP-1559 base fee** | Modify daeji to compute a dynamic base fee from parent block gas usage, adjusting up when blocks are full and down when they are empty. | Requires daeji changes to the executor and block construction logic. Medium effort. Provides genuine economic spam resistance. |
| **C. Token-based posting costs** | Ignore gas pricing entirely. Require agents to hold and spend DAEJI tokens (via the InsightBoard contract) to post knowledge entries. The economic barrier is at the application layer, not the protocol layer. | No chain changes. The token contract becomes the spam gate. Does not prevent non-knowledge-posting spam (random value transfers, contract calls). |

### Recommendation

**Option A** for devnet (free gas with contract-level rate limits). **Option C** for any multi-operator deployment (token-based posting costs provide an economic barrier without requiring chain modifications).

---

## Gap 6: BLOCKHASH Opcode

### What Was Assumed

A working `BLOCKHASH` opcode. The original design used block hashes for VRF seed derivation, on-chain randomness, and block membership verification.

### What Daeji Actually Does

The `block_hash_ref` closure in daeji's REVM configuration always returns `B256::ZERO` -- a 32-byte zero value. The `BLOCKHASH` opcode returns zero for all inputs, regardless of the block number requested.

### Does Any Roko Code Use BLOCKHASH?

Two use cases:

1. **Randomness seed for VRF fallback.** `ConsortiumValidator.sol` uses `block.timestamp` (not BLOCKHASH directly) for a randomness fallback: `seed = bytes32(uint256(jobId + block.timestamp))`. However, the agent-chain design docs specified using `BLOCKHASH` for VRF seed derivation in cases where the threshold VRF output was not available. Daeji already provides `prevrandao` (the threshold consensus VRF output) in every block, which is a superior randomness source. BLOCKHASH-based randomness is a fallback for the fallback.

2. **Audit trail and block membership verification.** The `ChainWitnessEngine` in `crates/roko-chain/src/witness.rs` stores the block number of each witness anchor (`receipt.block_number`). A future audit tool might want to verify "was this witness really included in block N?" by checking the block hash. With BLOCKHASH returning zero, this verification path is broken.

**Impact on the self-hosting loop**: moderate. The witness engine works (it gets receipts via RPC), but the audit verification path that would use BLOCKHASH for cross-referencing is broken. The VRF fallback randomness is a minor concern since daeji's `prevrandao` is available.

### Fix Options

| Option | Description | Trade-offs |
|---|---|---|
| **A. Block hash ring buffer** | Store the most recent 256 block hashes in a data structure within the executor. The `block_hash_ref` closure looks up actual hashes instead of returning zero. | Requires modifying `revm.rs` in daeji. Low risk -- the ring buffer is simple and the data is already available (the executor knows the hash of every block it processes). |
| **B. System contract** | Deploy a contract at a fixed address that stores recent block hashes, updated per block via a system transaction. `BLOCKHASH` opcode calls this contract. | Requires executor changes (inject system transaction) and a deployed contract. More complex than option A but keeps state in EVM storage. |

### Recommendation

**Option A** -- add a ring buffer in the executor. This is the standard approach used by other minimal EVM chains and is straightforward to implement.

---

## Gap 7: Coinbase / Beneficiary

### What Was Assumed

Validator block rewards: 250 GNOS tokens per block paid to the block proposer's Ethereum address. Contracts could identify the current block proposer via `block.coinbase`.

### What Daeji Actually Does

`beneficiary` (the `coinbase` field in the EVM block context) is always set to `Address::ZERO` -- the zero address. There are no block rewards. `block.coinbase` in Solidity returns `0x0000...0000`.

### What Roko Would Use Coinbase For

Block producer identification. In a multi-validator production network, contracts might
need to know which validator proposed the current block for:
- Distributing block rewards (the `KoraiToken::EmissionSchedule` in roko-chain models
  100 KORAI/block initial emission with halving epochs)
- Validator attribution in audit logs
- Validator-specific rate limits or permissions

**Impact on the self-hosting loop**: low. The self-hosting loop does not interact with block producers. Coinbase is only relevant for validator economics, which is a production concern.

### Fix Options

| Option | Description | Trade-offs |
|---|---|---|
| **A. Map validator keys to Ethereum addresses** | Set `beneficiary` to an Ethereum address derived from the proposing validator's Ed25519 public key: `Address::from_slice(&keccak256(validator_pubkey)[12..])`. | Requires a small change in `crates/node/consensus/src/app.rs`. The derivation is deterministic. Validators do not need to manage a separate Ethereum key. |
| **B. Lookup table in genesis config** | Add a `"validator_addresses"` field to `genesis.json` mapping Ed25519 public keys to Ethereum addresses. | More flexible (validators choose their own Ethereum address) but requires config management. |

### Recommendation

**Option A** for devnet (deterministic derivation, zero config). Option B when validators need to control which Ethereum address receives rewards.

---

## Gap 8: WebSocket Subscriptions

### What Was Assumed

Real-time event streaming for agents watching chain state: new blocks, new knowledge entries, challenge windows opening, etc. Standard Ethereum clients expose `eth_subscribe` and `eth_unsubscribe` over WebSocket connections.

### What Daeji Actually Does

Daeji exposes only HTTP JSON-RPC. There is no WebSocket endpoint, no `eth_subscribe`, no `eth_unsubscribe`. Agents can only learn about new events by polling -- repeatedly calling `eth_blockNumber` and `eth_getLogs` at regular intervals.

### What Roko Components Need Real-Time Chain Events

Three roko subsystems would benefit from real-time chain event subscriptions:

1. **TUI dashboard (`roko dashboard`).** The ratatui-based TUI already has a file watcher (`crates/roko-cli/src/tui/fs_watch.rs`) using `notify::RecommendedWatcher` for local state changes. A chain events tab could display new blocks, knowledge postings, and witness anchors in real time. Currently the TUI only watches local files.

2. **Conductor watchers.** The `roko-conductor` crate has 10 watchers and a circuit breaker for runtime health monitoring. If any of these watchers need chain state (agent heartbeat liveness, reputation changes), they currently must poll.

3. **Agent heartbeat monitor.** The `HeartbeatMonitor` contract (if deployed) expects agents to post heartbeats periodically. The `heartbeat_ext` module in roko-chain models chain heartbeat integration. Real-time notification of missed heartbeats would be useful for the conductor's circuit breaker.

**What roko already has for real-time events**: `roko serve` runs on port 6677 with SSE (Server-Sent Events) and WebSocket endpoints for its own event streams. The architecture for push-based event delivery exists in roko -- it just is not wired to chain events because daeji does not push chain events.

**Impact on the self-hosting loop**: low. The self-hosting loop is sequential (dispatch task -> wait for gate -> persist -> next task). Real-time chain events would enhance the dashboard experience but do not affect the core loop's functionality.

### Fix Options

| Option | Description | Trade-offs |
|---|---|---|
| **A. Poll-based (current)** | Agent polls `eth_blockNumber` and `eth_getLogs` every N seconds. | Works today with no changes. High latency, high RPC load. Functional but inelegant. |
| **B. LedgerEvent channel (Tier 2)** | Run a daeji secondary peer (a read-only node that replicates all blocks via the P2P network). Subscribe to its `LedgerEvent::SnapshotPersisted` stream to learn about new finalized blocks in milliseconds. | Requires running a secondary peer process. Much lower latency than polling. But requires in-process or IPC access to the peer's event stream -- not accessible over a network API. |
| **C. Add WebSocket subscriptions to daeji RPC** | The RPC server library daeji uses (`jsonrpsee` -- a Rust JSON-RPC server library from the Parity team) natively supports WebSocket subscriptions. Wire `SubscriptionSink` for `newHeads`, `logs`, and `newPendingTransactions`. | Requires daeji source changes. Medium effort but well-supported by the library. The standard approach for EVM-compatible chains. |

### Recommendation

**Option A** for Phase 1 (polling works, no changes needed). **Option B** for Phase 2 if a secondary peer is already running (lowest latency). **Option C** as a daeji enhancement for full Ethereum tooling compatibility.

---

## Gap 9: Validator Set Size

### What Was Assumed

Up to 100 validators. Dynamic validator sets with staking (locking tokens to become a validator), slashing (penalizing misbehaving validators by destroying their staked tokens), and jailing (temporarily removing validators who fail to participate).

### What Daeji Actually Does

Leader election is hardcoded: `(view % 4) as u32 == validator_index`. The number 4 is a constant. The DKG ceremony (the process where validators collectively generate their cryptographic keys) is performed once at setup time and produces a fixed set of 4 key shares. There is no mechanism for:
- Adding or removing validators after setup
- Changing the threshold
- Staking or slashing
- Rotating the validator set while keeping the group public key constant ("resharing")

### Does Roko Need Dynamic Validators?

For devnet and the self-hosting loop: **no.** All 4 validators run on the developer's machine. The self-hosting loop does not interact with the validator set at all. Agent processes are managed by `ProcessSupervisor` in `roko-runtime`, which tracks Claude CLI subprocesses -- completely separate from chain validators.

For production multi-operator networks: **yes.** If independent operators run roko instances and want to validate transactions on a shared chain, dynamic validator membership is essential. The `IdentityRegistry.sol` contract already models 4 tiers (Protocol, Sovereign, Worker, Edge) with staking thresholds (25,000 DAEJI for Sovereign, 5,000 for Worker), which could gate validator admission.

**Impact on the self-hosting loop**: none. This is entirely a production operations concern.

### Fix Options

| Option | Description | Trade-offs |
|---|---|---|
| **A. Accept fixed validator set for devnet** | 4 validators is sufficient for development and testing. Dynamic sets are not needed until production. | Zero effort. Fine for now. |
| **B. Make validator count configurable** | Change the hardcoded `4` to read from configuration. Trivial code change. Does not add dynamic membership, but allows deploying with different set sizes. | Very low effort. Still requires DKG re-run to change the set. |
| **C. Implement resharing** | Build the resharing protocol in daeji using commonware's existing resharing primitives. Allows the validator set to change while the group public key stays constant. | Significant effort. Phase 3+ concern. |

### Recommendation

**Option A** for devnet. This is a non-issue for local development. Revisit when preparing for multi-operator deployment.

---

## Gap 10: Token Identity (GNOS vs KORAI vs DAEJI)

### What Was Assumed

A GNOS token: an ERC-20 token with 1% annual demurrage (the balance decays over time if not actively used, encouraging circulation). Lazy-evaluated -- the decay is not applied per block but computed on-read using the formula `balance * 2^(-elapsed_time / half_life)`.

### What Exists Today

Three different token implementations, none of which match the full GNOS specification:

1. **`roko-chain/korai_token.rs`** (Rust, in-memory): An in-memory KORAI token with 1% demurrage. Rust-only, not a deployed smart contract. Used in unit tests. Full implementation with 5 earning pathways (TaskCompletion, KnowledgeContribution, ValidationParticipation, ReputationStaking, MarketplaceFees), 5 spending mechanisms, and an `EmissionSchedule` with halving epochs (100 KORAI/block initial, halving per epoch, 1 KORAI/block terminal, 1B max supply).

2. **`contracts/MockERC20.sol`** (Solidity, deployed): A simple `DAEJI` test token. Standard ERC-20 with no demurrage. Deployed against local test environments (Anvil, mirage-rs).

3. **The GNOS spec from the agent-chain docs** (not implemented): Full specification with demurrage, minting events, quadratic posting costs, and reputation tier gating.

No token with demurrage is deployed on any chain.

### Connection to Roko's Energy Pools

Roko already has an economic system: the USD budget pools in `orchestrate.rs`. Every plan gets a budget (default $25 from `roko.toml [budget] max_plan_usd`). Every agent dispatch deducts from it. The orchestrator tracks costs per plan and per task in `HashMap<String, f64>` accumulators. When the budget is exhausted, the plan aborts.

A deployed token would be the on-chain analog of this off-chain budget system. The demurrage mechanic maps naturally to roko's cost pressure: tokens you do not spend decay, just as unused budget in a plan expires when the plan completes.

The `InsightBoard.sol` contract already integrates with an ERC-20 for confirmation rewards (`REWARD_PER_CONFIRM = 1 ether`). If KORAI (with demurrage) were deployed instead of MockERC20, the knowledge sharing economics would have a natural decay pressure -- old, unconfirmed entries' posting costs would effectively vanish as the poster's balance decays.

**Impact on the self-hosting loop**: low. The loop works entirely with USD budget accounting. Token economics are a multi-fleet production concern.

### Fix Options

| Option | Description | Trade-offs |
|---|---|---|
| **A. Use plain ETH for devnet** | No token overhead. Gas pays for transactions. | Simplest. But gas is free on daeji (see Gap 5), so there is no economic cost. |
| **B. Deploy MockERC20 (DAEJI)** | Simple ERC-20, no demurrage. Use for staking and posting fees in the InsightBoard contract. | Low effort. Functional for testing the economic flows without the complexity of demurrage. |
| **C. Deploy KORAI with demurrage** | Port the Rust demurrage logic from `korai_token.rs` to Solidity. Use `wadPow` (a fixed-point exponentiation function) for lazy decay computation. | Medium effort. Matches the spirit of the GNOS spec. Demurrage logic in Solidity is non-trivial but well-understood. |
| **D. Implement the full GNOS spec** | Minting events, quadratic posting costs, reputation tiers, the works. | High effort. Premature for a devnet. |

### Recommendation

**Option B** for Phase 1 (simple token for testing economic flows). **Option C** for Phase 2 (demurrage token for realistic agent economics).

---

## Gap 11: HDC Vector Storage

### What Was Assumed

HDC (Hyperdimensional Computing) vectors -- 10,240-bit binary vectors (1,280 bytes each) -- stored inline in EVM contract state. A precompile at address 0x09 performs brute-force Hamming distance similarity search over all stored vectors to find knowledge entries relevant to a given query.

### Where HDC Vectors Live Today

HDC vectors live in two places in roko, both off-chain:

1. **`roko-primitives::HdcVector`** (`crates/roko-primitives/src/hdc.rs`): The core type. Represented as `[u64; 160]` (10,240 bits = 1,280 bytes). Full operations: encoding text to vectors via random projection, Hamming distance computation, majority-vote bundling, XOR binding, serialization. Also includes `ItemMemory` (a collection of labeled vectors for nearest-neighbor lookup), `BundleAccumulator`, and `DecayingBundleAccumulator`.

2. **`roko-neuro::hdc`** (`crates/roko-neuro/src/hdc.rs`): A `KnowledgeHdcEncoder` that converts `KnowledgeEntry` objects into `HdcVector` representations. The `KnowledgeStore` uses this (behind the `hdc` feature flag) for semantic similarity search during knowledge queries. The HDC similarity score contributes 40% of the context assembly weight (per the spec in `ContextAssemblyWeights`).

Additionally, `roko-primitives` has a `Codebook` module for deterministic symbol allocation, role-filler binding, pattern storage, and cross-domain resonance detection.

The on-chain contract `InsightBoard.sol` stores only `contentHash: bytes32` (32 bytes) and `uri: string`. **No HDC vector is stored on-chain.**

### What Daeji Reality Means for HDC Storage

Storing 1,280 bytes per entry in EVM storage is expensive. Each EVM `SSTORE` operation writes 32 bytes and costs ~20,000 gas (for a fresh slot). A full HDC vector requires 40 SSTORE operations = ~800,000 gas per vector (at minimum, potentially higher). At daeji's current free gas pricing this technically works, but it means each knowledge entry consumes 40 storage slots in the state database, which scales poorly.

More importantly, there is no way to search these vectors on-chain. Solidity cannot efficiently compute Hamming distance over 10,240-bit vectors. Even a simple comparison loop over 160 uint64 values would cost millions of gas. The precompile (0x09) was designed to solve this, but it requires daeji source modifications (see Gap 3).

### Impact on the Integration

On-chain HDC storage is gas-prohibitive at scale. Off-chain storage breaks the precompile search model (the precompile cannot search vectors that are not in EVM state). The original design's "search on-chain, prove on-chain" model does not work without both the storage and the precompile.

**Impact on the self-hosting loop**: none. The self-hosting loop uses `KnowledgeStore::query()` locally, which does HDC search in-process via Rust. On-chain HDC is for cross-fleet knowledge discovery.

### Fix Options

| Option | Description | Trade-offs |
|---|---|---|
| **A. Off-chain vectors, on-chain hashes** | Store `blake3(vector)` on-chain (32 bytes, cheap). Maintain a separate vector index off-chain in roko's neuro store or a sidecar process. Search happens off-chain. | No chain changes. No on-chain proof that a search result is genuine (you trust the searching agent). Pragmatic and functional. |
| **B. Precompile with sidecar state** | Implement the HDC precompile (0x09) but have it read from a separate state store, not from EVM storage. Vectors are stored in a dedicated QMDB partition or in-memory index maintained by the executor. | Requires daeji source changes. Cleanest architecture. The precompile indexes vectors per finalized block, so the search is always consistent with chain state. |
| **C. Calldata-only** | Post vectors as transaction calldata (much cheaper than storage -- calldata costs 4 gas per zero byte, 16 per non-zero byte). A sidecar indexes all calldata. Search results reference transaction hashes. | Cheap on-chain footprint. But calldata is not in contract state, so contracts cannot read it. Requires off-chain indexing. |

### Recommendation

**Option A** for the contract-only approach (no chain changes needed, pragmatic). **Option B** when entry count exceeds ~1000 and tag-based search is insufficient, justifying the daeji source modification.

---

## Gap 12: Existing Roko Contracts vs Agent-Chain Contracts

### What Was Assumed

The agent-chain documents specified three on-chain components:
- **GolemRegistry** at precompile 0x08: agent identity, heartbeat, capability hashing
- **InsightLedger** at precompile 0x0A: knowledge entries with posting, confirmation, challenge, decay, and HDC search
- **GNOSToken**: ERC-20 with demurrage

### What Already Exists in Roko's Solidity Contracts

Roko has 10 Solidity contracts in `contracts/src/`:

| Contract | What It Does | Lines | Uses `block.timestamp`? |
|---|---|---|---|
| `IdentityRegistry.sol` | Full ERC-8004 soulbound identity passport. 4 tiers (Protocol/Sovereign/Worker/Edge), staking (25K DAEJI for Sovereign, 5K for Worker), timelocks, capability bitmasks, system prompt hash, TEE attestation. | ~430 | Yes: cooldowns, update delays |
| `AgentRegistry.sol` | Minimal agent identity: name, capabilities (as string array), heartbeat, liveness tracking. | ~120 | Yes: heartbeat timestamps |
| `InsightBoard.sol` | Knowledge with pheromone curation: contentHash + uri, confirm/claim, ERC-20 rewards (1 token per confirmation). | ~78 | Yes: `postedAt` |
| `ReputationRegistry.sol` | 7-domain EMA reputation with decay. Domains: code_quality, task_completion, reliability, collaboration, knowledge, security, efficiency. | ~270 | Yes: decay calculation |
| `ValidationRegistry.sol` | Work proof + validator attestation. Records task completions with proof hashes. | ~130 | Yes: proof timestamps |
| `BountyMarket.sol` | Bounty marketplace with DAEJI token. Post bounties, assign workers, submit/approve work. | ~110 | Yes: deadline checks |
| `WorkerRegistry.sol` | Worker staking and liveness tracking. Stake to register, heartbeat to stay active. | ~230 | Yes: liveness windows |
| `ConsortiumValidator.sol` | Consortium-based validation with weighted voting. | ~80 | Yes: randomness fallback |
| `FeeDistributor.sol` | Fee distribution across registered recipients. | ~50 | No |
| `MockERC20.sol` | DAEJI test token (simple ERC-20, no demurrage). Standard mint/transfer. | ~30 | No |

### Gap Between Existing and Specified Contracts

- `AgentRegistry.sol` + `IdentityRegistry.sol` together cover the GolemRegistry spec (identity + heartbeat + capabilities). `IdentityRegistry` is actually richer than the spec (4 tiers, staking, TEE attestation, soulbound passport).
- `InsightBoard.sol` covers the basics of the InsightLedger (post + confirm + pheromone) but is simpler: no decay computation, no challenge mechanism, no HDC vectors. No knowledge kind classification, no half-life. The gap between `InsightBoard.sol` and the local `KnowledgeEntry` struct is large.
- No deployed token has demurrage. `MockERC20` is a plain ERC-20. The Rust `KoraiToken` has demurrage but is not deployable.
- No PredictionClaim contract exists (though `roko-chain` has a `FuturesMarket` module in Rust).

### Compatibility Issues with Daeji

All existing contracts were deployed against local Anvil (the Foundry test chain) or mirage-rs (roko's in-process EVM fork simulator, chain ID 88888). They need redeployment to daeji (chain ID 1337). No Solidity code changes are needed -- just rerun the deployment script against the daeji RPC endpoint.

However, specific daeji issues affect contract behavior:
- **`IdentityRegistry.sol`**: Uses `block.timestamp` for registration timelocks (`PROMPT_UPDATE_DELAY = 1 days`) and cooldowns (`WITHDRAW_COOLDOWN = 7 days`). **Broken on daeji** until the timestamp bug is fixed (see Gap 4). A 7-day cooldown = 604,800 seconds. With `block.timestamp = block.height`, the cooldown would "expire" at block 604,800 -- but relative to the current block, not relative to when the cooldown started.
- **`ReputationRegistry.sol`**: Uses `block.timestamp` for decay calculations (`halvings = (block.timestamp - lastUpdate) / DECAY_PERIOD`). **Broken on daeji** -- the "elapsed time" between block 1000 and block 2000 would be 1000 (seconds? blocks? neither -- it is a meaningless number when timestamp = height).
- **`InsightBoard.sol`**: Uses `msg.sender` and basic ERC-20 transfers. The `postedAt` timestamp is cosmetic (used for display, not logic). **Mostly compatible** -- functional but with wrong timestamps.
- **`BountyMarket.sol`**: Deadline enforcement uses `block.timestamp`. **Broken** -- deadlines set as Unix timestamps will never match block heights.
- **All contracts using `block.timestamp`**: Will not work correctly until the one-line daeji fix is applied.

**Impact on the self-hosting loop**: none. The self-hosting loop does not deploy or call any Solidity contracts. The orchestrator, gate pipeline, knowledge store, and episode logger all run locally in Rust.

### Recommendation

Fix daeji timestamps first (Gap 4, one-line change). Then redeploy the existing contracts to daeji without modification. Extend `InsightBoard.sol` with decay and challenge logic in Phase 2.

---

## Severity Ranking

| Gap | Issue | Severity | Effort to Fix | Recommended Phase | Self-Hosting Impact |
|---|---|---|---|---|---|
| 4 | `block.timestamp = height` (not wall-clock time) | **Critical** | Low (one-line daeji change) | Phase 1 | None (local loop uses chrono) |
| 6 | BLOCKHASH opcode returns zero for all inputs | **High** | Medium (ring buffer in executor) | Phase 1 | Moderate (audit verification) |
| 3 | No custom precompiles (0x08-0x0A) | **High** | Medium (executor refactor) | Phase 2 | None (local HDC search works) |
| 8 | No WebSocket subscriptions (`eth_subscribe`) | **Medium** | Medium (jsonrpsee supports it natively) | Phase 2 | None (dashboard uses local files) |
| 1 | No Merkle proofs / `eth_getProof` | **Medium** | High (architectural change or MMR workaround) | Phase 3+ | None (single-fleet) |
| 11 | HDC vector storage cost (1,280 bytes per entry) | **Medium** | Medium (precompile with sidecar state) | Phase 2 | None (local vectors work) |
| 5 | Free gas / no economic spam prevention | **Low** (devnet) | High (full EIP-1559 implementation) | Phase 3 | None (free gas helps devnet) |
| 7 | Coinbase/beneficiary is always zero address | **Low** | Low (key derivation mapping) | Phase 2 | None |
| 9 | Hardcoded 4-validator set | **Low** (devnet) | N/A for devnet | Phase 3 | None |
| 10 | Token identity confusion (GNOS/KORAI/DAEJI) | **Low** | Low (deploy MockERC20 for now) | Phase 1 | None (USD budgets work) |
| 12 | Existing contracts need redeployment + timestamp fix | **Medium** | Low (redeploy after timestamp fix) | Phase 1 | None |
| 2 | No block header extension fields | **Low** | Low (use contract state instead) | Phase 2 | None |
