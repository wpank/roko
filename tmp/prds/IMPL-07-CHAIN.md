# IMPL-07: Korai chain

**Implements:** Chain infrastructure (Kauri BFT consensus, SpecPool EVM, precompiles, InsightStore, token economics)
**Status:** Draft
**Date:** 2026-04-21
**Estimated effort:** 16-24 weeks across 5 phases

---

## Context

Roko is a Rust workspace at `/Users/will/dev/nunchi/roko/roko/` with 18 crates. Korai is the companion blockchain -- a purpose-built L1 for agent coordination, knowledge sharing, and financial instruments. This document specifies every task required to build the Korai chain from the existing crate stubs to a functional testnet.

Korai is not a general-purpose EVM chain. It has two execution planes (Kernel and EVM), custom precompiles for agent operations, and a consensus layer that computes ISFR as part of block production. The chain exists to serve agents, not the other way around.

### Workspace layout

| Crate | Path | Role in Korai |
|-------|------|--------------|
| `roko-chain` | `crates/roko-chain/` | Chain client, precompile logic, token, registries, ISFR |
| `roko-primitives` | `crates/roko-primitives/` | HDC vectors, tier routing, SIMD operations |
| `roko-neuro` | `crates/roko-neuro/` | Knowledge store (off-chain counterpart to InsightStore) |
| `roko-core` | `crates/roko-core/` | BenchmarkIndex trait, shared types |

### What already exists

The `roko-chain` crate has substantial infrastructure. Read these files before writing anything:

- `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/lib.rs` -- module map: `agent_registry`, `alloy_impl`, `client`, `collusion`, `futures_market`, `gate`, `heartbeat_ext`, `identity_economy_identity`, `identity_economy_markets`, `isfr`, `korai_token`, `marketplace`, `mock`, `nelson_siegel`, `observer`, `phase2`, `reputation_registry`, `tools`, `trace_rank`, `triage`, `types`, `validation_registry`, `wallet`, `witness`, `x402`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/phase2.rs` -- Phase 2 stubs for the chain surface: `Address`, `Bytes`, `B256`, `Hash`, `u256`, `i256`, `SigningKey`, `Provider`, `MirageInstance`, `BinaryFuse8`, `AgentPassport`, `PassportTier`, many more types
- `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/agent_registry.rs` -- ERC-8004 soulbound passports, 10 capability bits, 24h timelock for prompt hash updates
- `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/reputation_registry.rs` -- 7-domain EMA scoring (coding, security, research, chain, knowledge, operations, strategy), 4 discipline states, 30-day half-life decay, 7 violation types
- `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/korai_token.rs` -- KORAI token with lazy demurrage (1% annual), 5 earning pathways, 5 spending mechanisms, `BalanceRecord` with `effective_balance()` and `materialise_demurrage()`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/client.rs` -- `ChainClient` and `ChainWallet` traits (backend-agnostic)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/alloy_impl.rs` -- Alloy-backed JSON-RPC implementation
- `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/witness.rs` -- On-chain witness engine for reasoning traces
- `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/marketplace.rs` -- Spore job marketplace with escrow and 3 hiring models
- `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/collusion.rs` -- Collusion ring detection via assignment graph clique analysis
- `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/trace_rank.rs` -- PageRank-style reputation propagation over payment edges
- `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/x402.rs` -- HTTP 402 micropayment protocol with state channels
- `/Users/will/dev/nunchi/roko/roko/crates/roko-primitives/src/` -- HDC vector operations, SIMD acceleration

**Critical:** The `phase2.rs` file contains dozens of type stubs that define the Korai type landscape. Many tasks below involve replacing stubs with real implementations. Start by reading the stubs to understand the expected type signatures.

---

## Phase 1: Consensus (Kauri BFT)

Goal: implement a pipelined BFT consensus protocol with 400ms block cadence and single-slot finality.

### Task 1.1: Define consensus types

**Read first:**
- `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/phase2.rs` (existing consensus stubs)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/types.rs` (BlockNumber, TxHash, ChainHeader)

**File to create:** `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/consensus/mod.rs`

**What to implement:**

Core consensus types for Kauri BFT:

```rust
pub struct KauriConfig {
    pub block_cadence_ms: u64,       // 400ms
    pub min_validators: u32,         // 21
    pub min_stake: u256,             // 100 ETH equivalent
    pub byzantine_threshold: f64,    // 1/3
    pub pipeline_depth: u32,         // 3 (propose, prevote, precommit overlap)
}

pub struct ValidatorSet {
    pub validators: Vec<Validator>,
    pub total_stake: u256,
    pub epoch: u64,
}

pub struct Validator {
    pub index: u32,
    pub address: Address,
    pub stake: u256,
    pub bls_pubkey: [u8; 48],
}

pub struct KauriBlock {
    pub header: KauriBlockHeader,
    pub transactions: Vec<SignedTransaction>,
    pub oracle_votes: Vec<OracleVote>,
    pub consensus_certificate: ConsensusCertificate,
}
```

**Checklist:**
- [ ] Create `crates/roko-chain/src/consensus/` module directory
- [ ] Create `mod.rs` with submodule declarations
- [ ] Define `KauriConfig` with defaults: 400ms cadence, 21 validators, 100 ETH min stake
- [ ] Define `ValidatorSet` with total stake computation
- [ ] Define `Validator` struct with BLS public key placeholder
- [ ] Define `KauriBlock` and `KauriBlockHeader` (parent_hash, state_root, tx_root, oracle_root, timestamp, height, proposer)
- [ ] Define `ConsensusCertificate` -- aggregated BLS signature from 2/3+ validators
- [ ] Define `SignedTransaction` wrapper
- [ ] Define `OracleVote` -- validator's ISFR submission bundled with the block
- [ ] Export all types from `lib.rs`
- [ ] Unit test: `ValidatorSet` rejects sets with fewer than `min_validators`
- [ ] Unit test: `ConsensusCertificate` verifies that signatures cover >2/3 stake

**Test:** `cargo test -p roko-chain -- consensus_types`

---

### Task 1.2: Implement pipelined BFT

**Read first:** Task 1.1

**File to create:** `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/consensus/kauri_bft.rs`

**What to implement:**

Kauri BFT is a pipelined variant of PBFT with O(n) message complexity (not O(n^2)). Pipelining means that while block N is in the precommit phase, block N+1 is in prevote, and block N+2 is being proposed. This achieves 400ms block cadence despite multi-round consensus.

Three phases per block:
1. **Propose:** Leader broadcasts block proposal
2. **Prevote:** Validators validate and sign prevotes
3. **Precommit:** Validators precommit after receiving 2/3+ prevotes

O(n) complexity via tree topology: validators form a balanced binary tree. Messages aggregate up the tree rather than broadcasting to all peers.

**Checklist:**
- [ ] Implement `KauriBFT` struct holding validator set, current height, pipeline state
- [ ] Implement `propose(txs: Vec<SignedTransaction>, oracle_votes: Vec<OracleVote>) -> KauriBlock` -- leader creates block proposal
- [ ] Implement `prevote(block: &KauriBlock) -> Prevote` -- validator validates and signs
- [ ] Implement `precommit(prevotes: &[Prevote]) -> Option<Precommit>` -- only if 2/3+ prevotes received
- [ ] Implement `finalize(precommits: &[Precommit]) -> Option<ConsensusCertificate>` -- if 2/3+ precommits
- [ ] Implement pipeline: track 3 in-flight blocks at different stages simultaneously
- [ ] Implement leader rotation: round-robin among validators weighted by stake
- [ ] Implement view change: timeout after 2x block cadence triggers leader rotation
- [ ] Handle Byzantine validators: ignore votes from validators who have been slashed
- [ ] Single-slot finality: a block is final when its ConsensusCertificate is formed (no confirmation depth needed)
- [ ] Unit test: 4 validators, all honest, produce 10 blocks at 400ms cadence
- [ ] Unit test: 4 validators, 1 Byzantine (never votes), chain still progresses

**Test:** `cargo test -p roko-chain -- kauri_bft`

---

### Task 1.3: Validator set management

**Read first:** Task 1.1

**File to create:** `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/consensus/validator_set.rs`

**What to implement:**

Validator set management: staking, unstaking, slashing, and epoch transitions.

**Checklist:**
- [ ] Implement `ValidatorRegistry` struct: active validators, pending joins, pending exits
- [ ] Implement `stake(address: Address, amount: u256)` -- join the validator set (effective next epoch)
- [ ] Implement `unstake(address: Address)` -- request exit (effective after unbonding period)
- [ ] Implement `slash(address: Address, amount: u256, reason: SlashReason)` -- reduce stake, may remove from set
- [ ] Define `SlashReason` enum: `Equivocation`, `Downtime`, `InvalidOracleVote`, `ByzantineBehavior`
- [ ] Implement epoch transitions: apply pending joins/exits at epoch boundaries
- [ ] Enforce 21-validator minimum: reject unstake requests that would reduce below minimum
- [ ] Enforce 100 ETH minimum stake: reject stakes below threshold
- [ ] Unit test: validator stakes, becomes active next epoch
- [ ] Unit test: validator unstakes, removed after unbonding period
- [ ] Unit test: slash reduces stake, drops below minimum -> validator removed

**Test:** `cargo test -p roko-chain -- validator_set`

---

### Task 1.4: Testnet configuration and test

**Read first:** Tasks 1.1, 1.2, 1.3

**File to create:** `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/tests/testnet_consensus.rs`

**What to implement:**

A 3-node testnet that produces blocks at 400ms cadence and survives 1 Byzantine node.

**Checklist:**
- [ ] Create integration test that instantiates 3 validators (below the 21 minimum -- use a testnet config override)
- [ ] Run consensus for 100 blocks
- [ ] Verify block cadence: median inter-block time is approximately 400ms
- [ ] Verify single-slot finality: every block has a ConsensusCertificate
- [ ] Inject Byzantine behavior on 1 of 3 nodes (node sends conflicting prevotes)
- [ ] Verify chain progresses despite 1 Byzantine node (1/3 tolerance satisfied: 1 of 3 is at the limit)
- [ ] Verify Byzantine node is eventually slashed
- [ ] Verify no forks: all honest nodes agree on the same chain tip

**Test:** `cargo test -p roko-chain --test testnet_consensus`

---

## Phase 2: Execution (SpecPool EVM)

Goal: implement Block-STM parallel execution with MDBX storage and a dual-plane architecture.

### Task 2.1: Block-STM parallel execution

**Read first:**
- `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/phase2.rs` (execution stubs)

**File to create:** `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/execution/mod.rs`

**What to implement:**

Block-STM: optimistic parallel execution with conflict detection and re-execution.

1. All transactions in a block execute in parallel, optimistically assuming no conflicts.
2. A conflict detector checks read/write sets after execution.
3. Conflicting transactions re-execute serially in the correct order.

**Checklist:**
- [ ] Create `crates/roko-chain/src/execution/` module directory
- [ ] Define `ExecutionEngine` trait: `execute_block(block: &KauriBlock, state: &mut StateDB) -> ExecutionResult`
- [ ] Implement `BlockSTM` struct implementing `ExecutionEngine`
- [ ] Implement `ReadWriteSet` per transaction: tracks which storage slots were read and written
- [ ] Implement optimistic parallel execution: spawn N tasks (one per tx), each writes to a thread-local overlay
- [ ] Implement conflict detection: for each pair of transactions, check if tx_i's write set overlaps tx_j's read set
- [ ] Implement serial re-execution of conflicting transactions in block order
- [ ] Merge non-conflicting overlays into the final state
- [ ] Define `ExecutionResult`: `{ receipts: Vec<Receipt>, state_root: [u8; 32], gas_used: u64 }`
- [ ] Unit test: 10 non-conflicting transactions execute in parallel, all succeed
- [ ] Unit test: 2 transactions writing to the same slot -> conflict detected, re-executed serially
- [ ] Unit test: 1 tx depends on another's output -> conflict detection catches the dependency

**Test:** `cargo test -p roko-chain -- block_stm`

---

### Task 2.2: MDBX storage backend

**Read first:** Task 2.1

**File to create:** `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/execution/mdbx_store.rs`

**What to implement:**

MDBX-backed state storage. MDBX is a memory-mapped key-value store (fork of LMDB) used by Reth and Erigon for Ethereum state.

**Checklist:**
- [ ] Add `libmdbx` dependency to `roko-chain/Cargo.toml` (behind a feature flag `mdbx-backend`)
- [ ] Implement `StateDB` trait: `get(key: &[u8]) -> Option<Vec<u8>>`, `put(key: &[u8], value: &[u8])`, `delete(key: &[u8])`, `commit() -> [u8; 32]` (returns state root)
- [ ] Implement `MDBXStateDB` struct implementing `StateDB`
- [ ] Implement account storage: `account_key(address) -> (nonce, balance, code_hash, storage_root)`
- [ ] Implement contract storage: `storage_key(address, slot) -> value`
- [ ] Implement state root computation: Merkle Patricia Trie root over all accounts
- [ ] Implement snapshotting: create read-only snapshot for parallel execution overlays
- [ ] Handle overlay merge: apply overlay writes to the canonical state after conflict resolution
- [ ] Unit test (feature-gated): write 100 key-value pairs, read them back, verify state root is deterministic
- [ ] Unit test: two overlays with non-conflicting writes merge correctly
- [ ] Benchmark: 10,000 reads in <10ms

**Test:** `cargo test -p roko-chain --features mdbx-backend -- mdbx_store`

---

### Task 2.3: Dual-plane architecture

**Read first:** Tasks 2.1, 2.2

**File to create:** `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/execution/dual_plane.rs`

**What to implement:**

Two execution planes:
1. **Kernel Plane** -- deterministic operations only. Precompile calls, ISFR computation, clearing engine, agent passport management. No arbitrary EVM bytecode. Executes in O(1) per operation. State transitions are fully specified by the protocol.
2. **EVM Plane** -- general-purpose EVM execution. Arbitrary smart contracts. Standard gas metering. Block-STM parallel execution from Task 2.1.

Transactions are routed to the correct plane based on their target address:
- Addresses 0xA00-0xAFF -> Kernel Plane (precompile range)
- All other addresses -> EVM Plane

**Checklist:**
- [ ] Define `ExecutionPlane` enum: `Kernel`, `EVM`
- [ ] Implement `route_transaction(tx: &SignedTransaction) -> ExecutionPlane`
- [ ] Implement `KernelPlaneExecutor`: processes precompile calls directly without EVM overhead
- [ ] Implement `EVMPlaneExecutor`: wraps Block-STM for general EVM execution
- [ ] Implement `DualPlaneExecutor` that routes transactions and aggregates results
- [ ] Kernel Plane transactions have zero gas cost (covered by protocol)
- [ ] EVM Plane transactions use standard EVM gas metering
- [ ] Both planes share the same state DB but Kernel Plane state is write-protected from EVM Plane
- [ ] Unit test: precompile call routes to Kernel Plane
- [ ] Unit test: regular contract call routes to EVM Plane
- [ ] Unit test: EVM contract cannot write to Kernel Plane state (address range protection)

**Test:** `cargo test -p roko-chain -- dual_plane`

---

### Task 2.4: Parallel execution integration test

**Read first:** Tasks 2.1, 2.2, 2.3

**File to create:** `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/tests/parallel_execution.rs`

**What to implement:**

End-to-end test: a block with mixed Kernel and EVM transactions executes correctly with parallel execution.

**Checklist:**
- [ ] Create a block with 20 transactions: 5 Kernel Plane (precompile calls) + 15 EVM Plane (contract interactions)
- [ ] Include 2 conflicting EVM transactions that write to the same storage slot
- [ ] Execute the block with `DualPlaneExecutor`
- [ ] Verify: all 5 Kernel transactions succeed with zero gas
- [ ] Verify: 13 non-conflicting EVM transactions execute in parallel
- [ ] Verify: 2 conflicting EVM transactions re-execute serially in correct order
- [ ] Verify: final state root is deterministic (same result on repeated execution)

**Test:** `cargo test -p roko-chain --test parallel_execution`

---

## Phase 3: Precompiles

Goal: implement 6 precompiles at addresses 0xA01-0xA0C that provide native agent operations on the Kernel Plane.

### Task 3.1: AgentPassport precompile (0xA01)

**Read first:**
- `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/agent_registry.rs` (existing passport logic: soulbound ERC-721, 10 capability bits, 24h timelock)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/phase2.rs` (`AgentPassport`, `PassportTier`)

**File to create:** `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/precompiles/agent_passport.rs`

**What to implement:**

Wrap the existing `AgentRegistry` as an EVM precompile at 0xA01. Functions:
- `register(capabilities, promptHash, stake)` -- mint soulbound passport
- `getPassport(address)` -- return passport data
- `updatePromptHash(newHash)` -- submit with 24h timelock
- `getReputation(address, domain)` -- query 7-domain reputation

**Checklist:**
- [ ] Create `crates/roko-chain/src/precompiles/` module (if not already created in IMPL-06)
- [ ] Implement `AgentPassportPrecompile` at address `0xA01`
- [ ] Implement ABI encoding/decoding for each function
- [ ] Wire to existing `AgentRegistry` for passport CRUD
- [ ] Wire to existing `ReputationRegistry` for reputation queries
- [ ] Enforce soulbound: `transfer()` always reverts
- [ ] Enforce 24h timelock on prompt hash updates
- [ ] Enforce capability bits: validate against `CAP_*` constants
- [ ] Unit test: register passport, query it back, verify fields match
- [ ] Unit test: attempt transfer -> revert
- [ ] Unit test: update prompt hash, verify 24h lock before it takes effect

**Test:** `cargo test -p roko-chain -- agent_passport_precompile`

---

### Task 3.2: nCLOB precompile (0xA02)

**Read first:**
- IMPL-06 Phase 6 (cooperative clearing engine)

**File to create:** `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/precompiles/nclob.rs`

**What to implement:**

The native Central Limit Order Book for yield perpetuals, exposed as a precompile at 0xA02. Functions:
- `submitOrder(side, limitBps, notional, partialFill)` -- add order to current batch
- `cancelOrder(orderId)` -- remove from pending batch
- `getOrderBook()` -- return current book state (buy/sell depth)
- `getPosition(account, market)` -- return current position

**Checklist:**
- [ ] Implement `NCLOBPrecompile` at address `0xA02`
- [ ] Wire to `BatchAccumulator` from IMPL-06 Task 6.1 for order submission
- [ ] Wire to `YieldPerpMarket` from IMPL-06 Task 4.1 for position queries
- [ ] Implement order ID generation: `keccak256(account, side, limit, notional, nonce)`
- [ ] Implement order cancellation (only possible while in pending batch, not after close)
- [ ] Implement order book depth query: aggregate buy and sell side by price level
- [ ] Unit test: submit buy order, query book, verify order appears
- [ ] Unit test: submit order then cancel, verify removed from batch
- [ ] Unit test: query position after clearing round settles

**Test:** `cargo test -p roko-chain -- nclob_precompile`

---

### Task 3.3: INTENT precompile (0xA03)

**Read first:**
- `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/phase2.rs` (intent and delegation stubs)

**File to create:** `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/precompiles/intent.rs`

**What to implement:**

Signed agent intents with delegation caveats. The INTENT precompile enforces hard limits on agent behavior at the chain level.

```rust
pub struct AgentIntent {
    pub agent: Address,           // agent's passport address
    pub delegator: Address,       // user who delegated authority
    pub action: IntentAction,     // what the agent intends to do
    pub caveats: Vec<Caveat>,     // hard limits
    pub signature: [u8; 65],      // EIP-712 signature from delegator
    pub nonce: u64,
    pub expiry: u64,
}

pub enum Caveat {
    MaxPositionSize(u256),        // cannot exceed this notional
    ApprovedProtocols(Vec<Address>), // only interact with these contracts
    SpendingCap(u256),            // maximum total spending
    ForbiddenActions(Vec<IntentAction>), // explicitly blocked actions
    StopLoss(u256),               // must exit if loss exceeds this
}
```

**Checklist:**
- [ ] Implement `INTENTPrecompile` at address `0xA03`
- [ ] Implement `AgentIntent` struct
- [ ] Implement `Caveat` enum with 5 caveat types
- [ ] Implement `submitIntent(intent)` -- validate signature, verify caveats, execute if compliant
- [ ] Implement `validateCaveats(intent) -> bool` -- check all caveats before execution
- [ ] Implement `revokeDelegation(agent)` -- delegator can revoke at any time
- [ ] Implement EIP-712 structured data signing for intents
- [ ] Caveat enforcement: if ANY caveat is violated, the entire intent reverts
- [ ] Unit test: valid intent with all caveats satisfied -> executes
- [ ] Unit test: intent violating MaxPositionSize -> reverts
- [ ] Unit test: intent targeting a non-approved protocol -> reverts
- [ ] Unit test: delegator revokes, then agent submits intent -> reverts

**Test:** `cargo test -p roko-chain -- intent_precompile`

---

### Task 3.4: PROOF_LOG precompile (0xA04)

**Read first:**
- `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/witness.rs` (existing witness engine)

**File to create:** `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/precompiles/proof_log.rs`

**What to implement:**

On-chain commitment of reasoning traces. Agents commit the hash of their reasoning process before acting. This creates a verifiable audit trail.

Functions:
- `commitTrace(traceHash, blockHeight)` -- commit a reasoning trace hash
- `verifyTrace(agent, blockHeight, traceHash)` -- verify a previously committed trace
- `getTraceHistory(agent, fromBlock, toBlock)` -- return trace commitments in a range

**Checklist:**
- [ ] Implement `ProofLogPrecompile` at address `0xA04`
- [ ] Store trace commitments: `mapping(address => mapping(uint64 => bytes32))`
- [ ] Implement `commitTrace`: store `keccak256(agent, blockHeight, traceHash)` as the commitment
- [ ] Implement `verifyTrace`: compare stored commitment with provided inputs
- [ ] Implement `getTraceHistory`: return all commitments for an agent in a block range
- [ ] Wire to existing `ChainWitnessEngine` from `witness.rs`
- [ ] Unit test: commit a trace, verify it, get history
- [ ] Unit test: attempt to verify a non-committed trace -> returns false
- [ ] Unit test: commit at block 100, query range 50-150, verify included

**Test:** `cargo test -p roko-chain -- proof_log_precompile`

---

### Task 3.5: AGENT_REASON precompile (0xA05)

**Read first:** Task 3.4

**File to create:** `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/precompiles/agent_reason.rs`

**What to implement:**

Structured reasoning output publication. Unlike PROOF_LOG (which stores hashes), AGENT_REASON stores structured reasoning summaries on-chain.

```rust
pub struct ReasoningOutput {
    pub agent: Address,
    pub block_height: u64,
    pub observation_hash: [u8; 32],  // what the agent observed
    pub reasoning_summary: Vec<u8>,  // compressed reasoning (max 1024 bytes)
    pub action_taken: [u8; 32],      // hash of the action
    pub confidence: u16,             // 0-10000 (basis points)
}
```

**Checklist:**
- [ ] Implement `AgentReasonPrecompile` at address `0xA05`
- [ ] Implement `publishReasoning(output: ReasoningOutput)`
- [ ] Implement `getReasoning(agent, blockHeight) -> Option<ReasoningOutput>`
- [ ] Enforce max reasoning_summary size: 1024 bytes
- [ ] Only the agent itself (or its delegator) can publish reasoning
- [ ] Unit test: publish reasoning, query it back
- [ ] Unit test: exceed 1024 byte limit -> revert
- [ ] Unit test: non-agent tries to publish -> revert

**Test:** `cargo test -p roko-chain -- agent_reason_precompile`

---

### Task 3.6: HTC precompile (0xA0C)

**Read first:**
- `/Users/will/dev/nunchi/roko/roko/crates/roko-primitives/src/` (HDC vector operations, SIMD)

**File to create:** `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/precompiles/htc.rs`

**What to implement:**

Hyperdimensional Computing similarity search at consensus speed. The HTC precompile computes cosine similarity between HDC vectors using SIMD acceleration. Target: ~170us at 10K entries.

Functions:
- `similarity(vectorA, vectorB)` -- cosine similarity between two HDC vectors
- `nearestK(queryVector, k)` -- find k nearest vectors in the on-chain index
- `insert(key, vector)` -- add a vector to the on-chain index
- `delete(key)` -- remove a vector

**Checklist:**
- [ ] Implement `HTCPrecompile` at address `0xA0C`
- [ ] Wire to `roko-primitives` HDC vector operations
- [ ] Implement cosine similarity using SIMD: dot product / (norm_a * norm_b)
- [ ] Implement brute-force nearest-K search (scan all entries, keep top K by similarity)
- [ ] Implement vector storage: `mapping(bytes32 => Vec<f32>)` (or binary HDC vectors)
- [ ] Gas metering: O(n) for nearest-K where n = number of stored vectors
- [ ] Unit test: insert 100 vectors, query nearest-5, verify results are correct
- [ ] Unit test: similarity between identical vectors -> 1.0
- [ ] Unit test: similarity between orthogonal vectors -> ~0.0
- [ ] Benchmark: 10K vectors, nearest-10 query, verify <200us (SIMD-accelerated)

**Test:** `cargo test -p roko-chain -- htc_precompile`

---

### Task 3.7: Per-precompile integration tests

**Read first:** Tasks 3.1-3.6

**File to create:** `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/tests/precompile_suite.rs`

**What to implement:**

Integration test that calls each precompile from simulated EVM context.

**Checklist:**
- [ ] Test AgentPassport (0xA01): register -> query -> updatePromptHash
- [ ] Test nCLOB (0xA02): submitOrder -> getOrderBook -> getPosition
- [ ] Test INTENT (0xA03): submitIntent with valid caveats -> executes, with invalid caveats -> reverts
- [ ] Test PROOF_LOG (0xA04): commitTrace -> verifyTrace
- [ ] Test AGENT_REASON (0xA05): publishReasoning -> getReasoning
- [ ] Test HTC (0xA0C): insert 10 vectors -> nearestK(query, 3) -> verify top 3
- [ ] Each test simulates an EVM CALL to the precompile address
- [ ] Verify correct ABI encoding/decoding for all function signatures

**Test:** `cargo test -p roko-chain --test precompile_suite`

---

## Phase 4: InsightStore

Goal: implement the on-chain knowledge store with 6 entry types, pheromone-weighted relevance, and HDC similarity queries.

### Task 4.1: Define entry types

**Read first:**
- `/Users/will/dev/nunchi/roko/roko/crates/roko-neuro/src/` (off-chain KnowledgeStore for reference)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/phase2.rs` (InsightStore stubs)

**File to create:** `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/insight_store/mod.rs`

**What to implement:**

6 entry types per the knowledge architecture:

```rust
pub enum InsightType {
    Insight,          // factual observation with evidence
    Heuristic,        // learned pattern or rule of thumb
    Warning,          // risk signal or anomaly detection
    CausalLink,       // observed cause-effect relationship
    StrategyFragment, // partial strategy or tactic
    AntiKnowledge,    // "this is wrong" -- explicit falsification
}

pub struct InsightEntry {
    pub id: [u8; 32],
    pub entry_type: InsightType,
    pub content_hash: [u8; 32],      // hash of off-chain content
    pub hdc_fingerprint: Vec<f32>,    // HDC vector for similarity search
    pub domain: String,
    pub confidence: f64,              // 0.0 to 1.0
    pub contributor: Address,         // agent passport address
    pub pheromone_weight: f64,        // access-weighted relevance
    pub created_at: u64,
    pub last_accessed: u64,
    pub confirmation_count: u32,      // independent confirmations
    pub half_life_secs: u64,          // demurrage on relevance
}
```

**Checklist:**
- [ ] Create `crates/roko-chain/src/insight_store/` module directory
- [ ] Define `InsightType` enum with 6 variants
- [ ] Define `InsightEntry` struct with all fields
- [ ] Implement `InsightEntry::new()` with defaults: pheromone_weight = 1.0, half_life = 30 days
- [ ] Define `InsightQuery` struct: `{ query_vector: Vec<f32>, domain: Option<String>, min_confidence: f64, limit: usize }`
- [ ] Export from `lib.rs`
- [ ] Unit test: create each of the 6 entry types, verify serialization round-trip

**Test:** `cargo test -p roko-chain -- insight_types`

---

### Task 4.2: Implement pheromone weight with demurrage

**Read first:** Task 4.1

**File to modify:** `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/insight_store/mod.rs`

**What to implement:**

Pheromone weight decays over time (demurrage) unless the entry is accessed or confirmed:
- Each access increases pheromone weight
- Each independent confirmation extends the half-life
- Weight decays with the configured half-life

```
effective_weight = pheromone_weight * 0.5^((now - last_accessed) / half_life)
```

**Checklist:**
- [ ] Implement `effective_weight(now: u64) -> f64` on `InsightEntry`
- [ ] Implement `record_access(now: u64)` -- reset last_accessed, boost pheromone by 0.1
- [ ] Implement `record_confirmation(confirmer: Address, now: u64)` -- increment confirmation_count, extend half_life by 25%
- [ ] Cap pheromone_weight at 10.0 to prevent runaway accumulation
- [ ] Cap half_life extension at 180 days (6 months maximum relevance)
- [ ] Unit test: entry decays to 50% weight after one half-life
- [ ] Unit test: access resets decay timer, weight increases
- [ ] Unit test: 3 confirmations extend half_life from 30 days to ~58 days (30 * 1.25^3)

**Test:** `cargo test -p roko-chain -- pheromone_weight`

---

### Task 4.3: Implement reputation-weighted quality scoring

**Read first:**
- Task 4.1
- `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/reputation_registry.rs` (7-domain EMA scores)

**File to modify:** `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/insight_store/mod.rs`

**What to implement:**

Entries from higher-reputation agents start with higher initial quality scores. Quality score combines:
- Contributor reputation in the relevant domain
- Confidence declared by the contributor
- Confirmation count
- Pheromone weight

```
quality = (contributor_reputation * 0.4) + (confidence * 0.3) + (confirmation_score * 0.2) + (pheromone_normalized * 0.1)
```

where `confirmation_score = min(confirmation_count / 5, 1.0)`.

**Checklist:**
- [ ] Implement `quality_score(reputation: f64, now: u64) -> f64` on `InsightEntry`
- [ ] Wire reputation lookup: given contributor address and domain, look up reputation score
- [ ] Implement confirmation score: 5 confirmations = maximum contribution
- [ ] Normalize pheromone weight against a reference (e.g., median pheromone across all entries)
- [ ] Unit test: high-reputation contributor (0.9) with high confidence (0.9) and 5 confirmations -> quality ~0.9
- [ ] Unit test: low-reputation contributor (0.2) with low confidence (0.3) and 0 confirmations -> quality ~0.17

**Test:** `cargo test -p roko-chain -- insight_quality`

---

### Task 4.4: Implement HDC similarity queries

**Read first:**
- Task 3.6 (HTC precompile)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-primitives/src/` (HDC vector ops)

**File to modify:** `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/insight_store/mod.rs`

**What to implement:**

Query InsightStore entries by HDC vector similarity, optionally filtered by domain, type, and minimum confidence.

**Checklist:**
- [ ] Implement `InsightStore` struct holding all entries
- [ ] Implement `insert(entry: InsightEntry)` -- add to store and index
- [ ] Implement `query(q: InsightQuery) -> Vec<(InsightEntry, f64)>` -- returns entries with similarity scores
- [ ] Query logic: compute cosine similarity between query vector and each entry's hdc_fingerprint, apply filters, sort by similarity * quality, return top `limit` results
- [ ] Use the HTC precompile (or its underlying SIMD operations) for similarity computation
- [ ] Implement `get(id: &[u8; 32]) -> Option<&InsightEntry>` -- direct lookup by ID
- [ ] Implement `remove(id: &[u8; 32])` -- mark as removed (soft delete)
- [ ] Unit test: insert 100 entries in 3 domains, query with domain filter, verify only matching domain returns
- [ ] Unit test: insert 5 entries with known HDC vectors, query with a vector close to entry 3, verify entry 3 ranks first
- [ ] Unit test: query with min_confidence=0.8 excludes low-confidence entries

**Test:** `cargo test -p roko-chain -- insight_store_query`

---

### Task 4.5: InsightStore integration test

**Read first:** Tasks 4.1-4.4

**File to create:** `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/tests/insight_store.rs`

**What to implement:**

End-to-end test: submit entries, query by HDC similarity, verify relevance ranking.

**Checklist:**
- [ ] Create 20 entries across 3 domains with diverse HDC fingerprints
- [ ] Submit all 20 via `InsightStore::insert()`
- [ ] Query with a vector similar to entry #7 (blockchain domain)
- [ ] Verify entry #7 ranks first or second
- [ ] Simulate access on entry #7 (3 accesses), verify its pheromone weight increases
- [ ] Simulate passage of 60 days without access on entry #1, verify its effective weight has decayed
- [ ] Add 3 confirmations to entry #12, verify half-life extended
- [ ] Query again with domain filter "blockchain", verify only blockchain entries returned
- [ ] Verify all operations complete in <50ms for 20 entries

**Test:** `cargo test -p roko-chain --test insight_store`

---

## Phase 5: Token economics

Goal: implement KORAI token mechanics -- demurrage, staking exemption, emission schedule, and reputation multiplier.

### Task 5.1: Extend KORAI token implementation

**Read first:**
- `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/korai_token.rs` (existing: `KoraiToken`, `BalanceRecord`, lazy demurrage, earning/spending pathways)

**File to modify:** `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/korai_token.rs`

**What to implement:**

The existing `KoraiToken` has basic balance tracking with demurrage. Extend with:
1. Staking exemption: staked balances do not decay
2. Weekly emission cap with halving schedule
3. Reputation multiplier for earning

**Checklist:**
- [ ] Add `staked_balance: u256` field to `BalanceRecord`
- [ ] Modify `effective_balance()`: only apply demurrage to `stored_balance - staked_balance` (staked portion is exempt)
- [ ] Implement `stake(amount: u256)`: move tokens from available to staked
- [ ] Implement `unstake(amount: u256)`: move tokens from staked to available (with unbonding delay)
- [ ] Add `unbonding_period_secs: u64` to `KoraiTokenConfig` (default: 7 days)
- [ ] Track pending unstakes with release timestamps
- [ ] Unit test: stake 1000, wait 1 year, verify staked portion has not decayed
- [ ] Unit test: unstaked portion decays by 1% per year

**Test:** `cargo test -p roko-chain -- korai_staking`

---

### Task 5.2: Implement emission schedule

**Read first:** Task 5.1

**File to modify:** `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/korai_token.rs`

**What to implement:**

Weekly emission cap with halving:
- Initial weekly emission: configurable (e.g., 100,000 KORAI/week)
- Halving period: every 52 weeks (1 year)
- Emission distributed to: validators (40%), task completers (30%), knowledge contributors (20%), insurance fund (10%)

**Checklist:**
- [ ] Add `EmissionSchedule` struct: `{ initial_weekly: u256, halving_period_weeks: u32, current_epoch_week: u32, total_emitted: u256 }`
- [ ] Implement `weekly_cap(week: u32) -> u256`: `initial_weekly / 2^(week / halving_period_weeks)`
- [ ] Implement `distribute_emission(week: u32, validators: &[Address], task_completers: &[Address], knowledge_contributors: &[Address])` -- split according to percentages
- [ ] Track total emitted to enforce global supply cap (if configured)
- [ ] Unit test: week 0 emission = initial_weekly
- [ ] Unit test: week 52 emission = initial_weekly / 2
- [ ] Unit test: week 104 emission = initial_weekly / 4
- [ ] Unit test: distribution splits 40/30/20/10 correctly

**Test:** `cargo test -p roko-chain -- emission_schedule`

---

### Task 5.3: Implement reputation multiplier

**Read first:**
- Task 5.1
- `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/reputation_registry.rs` (7-domain EMA)

**File to modify:** `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/korai_token.rs`

**What to implement:**

Earnings are multiplied by a reputation-based factor:

```
multiplier = 0.1 + 2.9 * R^1.7
```

Where R is the agent's composite reputation (0.0 to 1.0). This means:
- R = 0.0: multiplier = 0.1 (10% of base earnings)
- R = 0.5: multiplier = 0.1 + 2.9 * 0.5^1.7 = 0.1 + 2.9 * 0.308 = ~0.99
- R = 1.0: multiplier = 0.1 + 2.9 * 1.0 = 3.0 (3x base earnings)

**Checklist:**
- [ ] Implement `reputation_multiplier(reputation: f64) -> f64`: `0.1 + 2.9 * reputation.powf(1.7)`
- [ ] Clamp input reputation to [0.0, 1.0]
- [ ] Wire into earning pathway: `actual_earning = base_earning * reputation_multiplier(R)`
- [ ] Define composite reputation: weighted average across the agent's active domains
- [ ] Unit test: R=0.0 -> multiplier = 0.1
- [ ] Unit test: R=0.5 -> multiplier approximately 1.0
- [ ] Unit test: R=1.0 -> multiplier = 3.0
- [ ] Unit test: R=0.8 -> multiplier approximately 2.18

**Test:** `cargo test -p roko-chain -- reputation_multiplier`

---

### Task 5.4: Token economics integration test

**Read first:** Tasks 5.1-5.3

**File to create:** `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/tests/token_economics.rs`

**What to implement:**

End-to-end test: mint, earn (with reputation multiplier), stake (exempt from demurrage), unstake (resumes demurrage), emission halving.

**Checklist:**
- [ ] Create 3 agents with reputations 0.2, 0.5, 0.9
- [ ] Distribute emission for week 0: verify each agent earns base * reputation_multiplier
- [ ] Agent 2 stakes half their balance, wait 1 year (simulated), verify staked portion did not decay
- [ ] Agent 1 does not stake, wait 1 year, verify 1% demurrage applied to full balance
- [ ] Advance to week 52: verify emission cap halved
- [ ] Agent 2 unstakes, verify unbonding period enforced
- [ ] After unbonding, verify tokens are available and now subject to demurrage

**Test:** `cargo test -p roko-chain --test token_economics`

---

## Phase 6: HDC precompile implementation

**Goal**: Implement the 4 HDC precompile functions at address 0xA0C that provide native hyperdimensional computing operations on the Kernel Plane. These operations underpin the InsightStore similarity search, knowledge sharing, and privacy-preserving publication.

### Task 6.1: Implement `hdc_similarity` precompile (Hamming distance)

**File to modify:** `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/precompiles/htc.rs`

**Read first:**
- Task 3.6 (HTC precompile skeleton)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-primitives/src/hdc.rs` -- `HdcVector`, `HDC_BITS`, `similarity()`, SIMD acceleration

**What to do:**

1. Implement the `hdc_similarity` function within the HTC precompile:

```rust
/// Compute Hamming-based similarity between two binary HDC vectors.
///
/// Input ABI: (bytes vectorA, bytes vectorB)
/// Output ABI: (uint256 similarity_bps) -- similarity in basis points (0-10000)
///
/// Gas cost: O(1) -- fixed for 10,240-bit vectors
/// Target latency: ~5us per pair
pub fn hdc_similarity(input: &[u8]) -> PrecompileResult {
    // Decode two 1,280-byte vectors from ABI-encoded input
    let (vec_a, vec_b) = decode_two_vectors(input)?;

    // Compute Hamming distance using SIMD (64-bit popcount)
    let hamming = vec_a.hamming_distance(&vec_b);
    let similarity = 1.0 - (hamming as f64 / HDC_BITS as f64);
    let similarity_bps = (similarity * 10000.0) as u32;

    // ABI-encode the result
    Ok(encode_u256(similarity_bps))
}
```

2. Implement `decode_two_vectors(input: &[u8]) -> Result<(HdcVector, HdcVector)>`: ABI-decode two `bytes` arguments into `HdcVector` instances.
3. Wire into the HTC precompile dispatch: function selector `0x01` maps to `hdc_similarity`.

**Test:**
- Two identical vectors -> similarity = 10000 bps (1.0).
- Two orthogonal vectors (random, expected ~5000 bps) -> similarity ~5000 bps.
- Two opposite vectors (one is bitwise NOT of the other) -> similarity = 0 bps.
- Verify result matches `HdcVector::similarity()` from roko-primitives.
- Benchmark: <10us per call.

- [ ] `hdc_similarity` computes Hamming-based similarity
- [ ] ABI-encoded input/output
- [ ] Result matches off-chain `HdcVector::similarity()`
- [ ] Latency <10us

---

### Task 6.2: Implement `hdc_topk` precompile (top-k similarity search)

**File to modify:** `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/precompiles/htc.rs`

**Read first:**
- Task 6.1 output
- Task 3.6 (original HTC spec)

**What to do:**

1. Implement the `hdc_topk` function:

```rust
/// Find the k most similar vectors in the on-chain index.
///
/// Input ABI: (bytes queryVector, uint256 k)
/// Output ABI: (bytes32[] keys, uint256[] similarities)
///
/// Gas cost: O(n) where n = number of stored vectors
/// Target latency: ~170us at 10K entries
pub fn hdc_topk(input: &[u8], store: &VectorStore) -> PrecompileResult {
    let (query, k) = decode_query_and_k(input)?;
    let k = k.min(100);  // cap at 100 results

    // Brute-force scan with SIMD-accelerated similarity
    let mut results: Vec<(Similarity, Key)> = store.entries()
        .map(|(key, vec)| {
            let sim = query.hamming_similarity(vec);
            (sim, key)
        })
        .collect();

    // Partial sort: only need top-k, not full sort
    results.select_nth_unstable_by(k.min(results.len()) - 1, |a, b| {
        b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal)
    });
    results.truncate(k);
    results.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

    encode_topk_result(&results)
}
```

2. Implement `VectorStore`: an in-memory store mapping `[u8; 32]` keys to `HdcVector` values. This is the on-chain HDC index.
3. Use `select_nth_unstable_by` for O(n) partial sort instead of O(n log n) full sort.

**Test:**
- Insert 100 vectors. Query nearest 5. Assert top-1 is the most similar.
- Insert 10,000 vectors. Query nearest 10. Assert latency <200us.
- Query with k > stored count: return all stored vectors (no panic).
- Query with k = 0: return empty result.

- [ ] `hdc_topk` finds k most similar vectors via brute-force scan
- [ ] Partial sort (O(n)) for efficiency
- [ ] Latency <170us at 10K entries
- [ ] k capped at 100

---

### Task 6.3: Implement `hdc_bind` precompile (XOR binding)

**File to modify:** `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/precompiles/htc.rs`

**Read first:**
- `/Users/will/dev/nunchi/roko/roko/crates/roko-primitives/src/hdc.rs` -- `HdcVector::bind()` (XOR)
- Task 6.1 output

**What to do:**

1. Implement the `hdc_bind` function:

```rust
/// Bind two HDC vectors via element-wise XOR.
///
/// Binding represents association: bind(dog, chases) = "dog-chases" role-filler pair.
/// XOR is its own inverse (involution): bind(bind(A, B), B) = A.
///
/// Input ABI: (bytes vectorA, bytes vectorB)
/// Output ABI: (bytes result)
///
/// Gas cost: O(1) -- fixed for 10,240-bit vectors
/// Target latency: ~2us
pub fn hdc_bind(input: &[u8]) -> PrecompileResult {
    let (vec_a, vec_b) = decode_two_vectors(input)?;
    let result = vec_a.bind(&vec_b);
    Ok(encode_vector(&result))
}
```

2. Wire into HTC precompile dispatch: function selector `0x03` maps to `hdc_bind`.

**Test:**
- `bind(A, B)` produces a vector different from both A and B.
- `bind(bind(A, B), B)` == A (involution property).
- `bind(A, A)` == zero vector (self-binding cancels).
- Latency <5us.

- [ ] `hdc_bind` implements element-wise XOR
- [ ] Involution property verified: `bind(bind(A, B), B) == A`
- [ ] Latency <5us

---

### Task 6.4: Implement `hdc_bundle` precompile (majority vote)

**File to modify:** `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/precompiles/htc.rs`

**Read first:**
- `/Users/will/dev/nunchi/roko/roko/crates/roko-primitives/src/hdc.rs` -- `HdcVector::bundle()`, `BundleAccumulator`
- Task 6.1 output

**What to do:**

1. Implement the `hdc_bundle` function:

```rust
/// Bundle multiple HDC vectors via element-wise majority vote.
///
/// Bundling represents superposition: bundle([dog, cat, bird]) = "animals" set.
/// The result is similar to all inputs.
///
/// Input ABI: (bytes[] vectors)
/// Output ABI: (bytes result)
///
/// Gas cost: O(n) where n = number of input vectors
/// Target latency: ~50us for 100 vectors
pub fn hdc_bundle(input: &[u8]) -> PrecompileResult {
    let vectors = decode_vector_array(input)?;
    if vectors.is_empty() {
        return Err(PrecompileError::InvalidInput("empty vector array".into()));
    }

    let mut accumulator = BundleAccumulator::new();
    for vec in &vectors {
        accumulator.add(vec);
    }
    let result = accumulator.finish();

    Ok(encode_vector(&result))
}
```

2. Implement `decode_vector_array(input: &[u8]) -> Result<Vec<HdcVector>>`: ABI-decode a dynamic array of `bytes` into a Vec of HdcVector.
3. Wire into HTC precompile dispatch: function selector `0x04` maps to `hdc_bundle`.

**Test:**
- `bundle([A])` == A (single vector bundle is identity).
- `bundle([A, A, B])` is more similar to A than to B (majority vote).
- `bundle([A, B])` is equally similar to A and B (tie-breaking is deterministic).
- 100 vectors: latency <50us.
- Result is similar to all input vectors (similarity > 0.5 for each).

- [ ] `hdc_bundle` implements majority vote over n vectors
- [ ] Result similar to all inputs
- [ ] Majority inputs dominate the result
- [ ] Latency <50us for 100 vectors

---

### Task 6.5: HDC precompile integration test

**File to create:** `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/tests/hdc_precompile.rs`

**Read first:**
- Tasks 6.1 through 6.4

**Do:**

1. **Scenario A: Full HDC workflow from EVM**
   - Call `hdc_bind(dog_vec, chases_vec)` -> dog_chases
   - Call `hdc_bind(cat_vec, chases_vec)` -> cat_chases
   - Call `hdc_bundle([dog_chases, cat_chases])` -> animals_chase
   - Call `hdc_similarity(animals_chase, dog_chases)` -> high similarity
   - Call `hdc_similarity(animals_chase, fish_vec)` -> low similarity
   - Assert: animals_chase is more similar to dog_chases than to fish_vec

2. **Scenario B: InsightStore query via HTC precompile**
   - Insert 50 vectors via the VectorStore
   - Call `hdc_topk(query, 5)` -> top 5 results
   - Verify top-1 result matches the manually computed nearest neighbor
   - Verify results are sorted by decreasing similarity

3. **Scenario C: Privacy-preserving publish via HDC**
   - Call `hdc_bind(knowledge_vec, agent_role_vec)` -> bound
   - Call `hdc_bind(bound, agent_role_vec)` -> unbound (should equal knowledge_vec)
   - Assert: unbound == knowledge_vec (involution)
   - This verifies the role unbinding used in IMPL-04 publishing works at the precompile level

4. **Scenario D: Performance at scale**
   - Insert 10,000 vectors
   - Run `hdc_topk(query, 10)` 100 times
   - Assert: mean latency <200us
   - Assert: all results correct (verify against brute-force reference)

5. Run: `cargo test -p roko-chain --test hdc_precompile`

- [ ] Full HDC workflow (bind, bundle, similarity) produces correct results
- [ ] InsightStore query via topk returns correct nearest neighbors
- [ ] Role unbinding involution verified at precompile level
- [ ] Performance: <200us for 10K-entry topk search
- [ ] All scenarios pass

---

## Acceptance criteria

- [ ] Testnet runs at 400ms blocks with single-slot finality (Task 1.4)
- [ ] Pipelined BFT achieves O(n) message complexity (Task 1.2)
- [ ] 1 Byzantine node out of 3 does not halt the chain (Task 1.4)
- [ ] Block-STM parallel execution handles conflicts correctly (Task 2.4)
- [ ] Dual-plane separates Kernel operations from general EVM (Task 2.3)
- [ ] EVM plane cannot write to Kernel Plane state (Task 2.3)
- [ ] All 6 precompiles are callable and return correct results (Task 3.7)
- [ ] INTENT precompile enforces delegation caveats (Task 3.3)
- [ ] HTC precompile computes similarity in <200us at 10K vectors (Task 3.6)
- [ ] InsightStore stores and retrieves entries by HDC similarity (Task 4.5)
- [ ] Pheromone weight decays with demurrage and recovers with access (Task 4.2)
- [ ] Quality scoring weights reputation, confidence, confirmations (Task 4.3)
- [ ] Token demurrage applies only to unstaked balances (Task 5.1)
- [ ] Emission halves yearly (Task 5.2)
- [ ] Reputation multiplier ranges from 0.1x to 3.0x (Task 5.3)
- [ ] All tests pass: `cargo test -p roko-chain`
- [ ] Clippy clean: `cargo clippy -p roko-chain --no-deps -- -D warnings`

---

## Dependencies

| This phase | Depends on | Reason |
|-----------|------------|--------|
| Phase 2 | Phase 1 | Execution runs inside consensus blocks |
| Phase 3 | Phase 2 | Precompiles execute on the Kernel Plane |
| Phase 4 | Phase 3 (Task 3.6) | InsightStore uses HTC precompile for similarity |
| Phase 5 | Phase 1 (Task 1.3) | Token staking interacts with validator set |
| IMPL-06 Phase 2 | This Phase 3 | ISFR precompile lives in the precompile system |

Phase 4 and Phase 5 can be developed in parallel once Phase 3 is complete.

---

## Build and test commands

```bash
# Build the chain crate
cargo build -p roko-chain

# Run consensus tests
cargo test -p roko-chain -- consensus

# Run execution tests
cargo test -p roko-chain -- block_stm
cargo test -p roko-chain -- dual_plane

# Run precompile tests
cargo test -p roko-chain -- precompile

# Run InsightStore tests
cargo test -p roko-chain -- insight_store

# Run token economics tests
cargo test -p roko-chain -- korai

# Run full integration suite
cargo test -p roko-chain --test testnet_consensus
cargo test -p roko-chain --test parallel_execution
cargo test -p roko-chain --test precompile_suite
cargo test -p roko-chain --test insight_store
cargo test -p roko-chain --test token_economics

# Lint
cargo clippy -p roko-chain --no-deps -- -D warnings

# With MDBX backend
cargo test -p roko-chain --features mdbx-backend -- mdbx
```
