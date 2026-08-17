# Required Changes to the Daeji Chain Node

## Purpose of This Document

This document specifies every modification needed to the daeji blockchain node source code (located at `/Users/will/dev/nunchi/daeji/`) for the roko agent integration. Each change includes:
- What the current behavior is and why it is problematic
- A detailed explanation of which roko subsystems depend on this change and how
- The code change
- Risk assessment and testing approach

Changes are ordered by priority. Priority 1 changes are prerequisites for deploying any Solidity contracts. Priority 2 changes enable advanced agent features. Priority 3 changes are nice-to-haves for production readiness.

---

## Background for the Cold Reader

### What Roko Is

**Roko** is a self-developing Rust toolkit (18 crates, ~177K lines of code) for building AI agents that build software autonomously. Roko develops itself: it reads PRDs (Product Requirements Documents), generates implementation plans, dispatches LLM-backed agents to execute each task, validates results through a 7-rung gate pipeline, persists episode records, and updates learning models. The entire cycle is: PRD -> plan -> agent -> gate -> persist -> learn -> iterate.

The central execution loop lives in `orchestrate.rs` (~18K lines in `crates/roko-cli/src/`). For each task in a plan DAG (directed acyclic graph), orchestrate.rs: checks dependencies, selects a model via the CascadeRouter (contextual bandit), builds a 9-layer system prompt via the SystemPromptBuilder, enriches with playbooks/neuro/research/gate feedback, dispatches an agent, runs the gate pipeline (compile -> lint -> test -> symbol -> gentest -> property -> LLM judge), records an episode, updates learning, and advances the DAG.

### What Daeji Is

**Daeji** (internal codename "Kora") is a minimal blockchain node that executes Ethereum-compatible smart contracts. It is built from scratch in Rust using composable primitives from the [commonware](https://github.com/commonwarexyz/monorepo) library. It is NOT a fork of go-ethereum (geth), Reth, or any existing Ethereum client.

Daeji runs as a multi-node system: 4 validator nodes collectively agree on blocks via a consensus protocol called "simplex BFT" (Byzantine Fault Tolerant). Each validator holds a share of a BLS12-381 threshold cryptographic key. When at least 3-of-4 validators sign a block proposal, it is finalized. Blocks finalize roughly every 400 milliseconds.

The node has three major components:
1. **Consensus layer** -- implements simplex BFT, proposes blocks, collects votes, finalizes
2. **Execution engine** -- runs smart contract bytecode using **REVM** (see below)
3. **State database** -- stores all account balances and contract data using **QMDB** (see below)

### What REVM Is

**REVM** (Rust Ethereum Virtual Machine) is a standalone Rust implementation of the Ethereum Virtual Machine. It is the same EVM engine used by Foundry (the Solidity development toolkit) and Reth (a Rust Ethereum client). REVM takes bytecode (compiled Solidity or any EVM-compatible language), executes it, and produces state changes (storage writes, balance transfers, event logs).

In daeji, REVM is configured and invoked in the file `crates/node/executor/src/revm.rs`. This file controls:
- Which precompiled contracts are available (see "precompiles" below)
- What block context values the EVM sees (`block.timestamp`, `block.number`, `block.coinbase`, etc.)
- How the EVM accesses state (reads and writes go through QMDB)
- How block hashes are resolved (the `BLOCKHASH` opcode)

### What jsonrpsee Is

**jsonrpsee** is a Rust library for building JSON-RPC servers and clients, created by the Parity team (the organization behind the Polkadot blockchain). JSON-RPC is the protocol Ethereum uses for external communication: tools like MetaMask, Foundry, and ethers.js send JSON-formatted requests (`{"method": "eth_getBalance", "params": ["0x123...", "latest"]}`) over HTTP, and the node responds with JSON results.

jsonrpsee provides:
- A `#[rpc(server)]` procedural macro that generates the JSON-RPC server boilerplate from a Rust trait definition
- WebSocket support (for push-based event subscriptions like `eth_subscribe`)
- Both HTTP and WebSocket transport on the same server

Daeji uses jsonrpsee for its RPC layer. Adding new RPC methods means defining a new trait with `#[rpc(server, namespace = "kora")]` and implementing it.

### What a Precompile Is

A **precompile** (precompiled contract) is native code that lives at a fixed EVM address. When a smart contract calls a precompile address, the EVM does NOT execute bytecode. Instead, it calls a native function (written in Rust in this case) that is compiled into the node binary.

Standard Ethereum has 9 precompiles at addresses 0x01-0x09 for expensive cryptographic operations (signature recovery, hashing, elliptic curve math). Custom precompiles require modifying the node's EVM configuration to register additional addresses and their handler functions.

In REVM, precompiles are registered as a collection: a mapping from `Address` to a function that takes raw bytes as input and returns raw bytes as output. The `build_mainnet()` convenience function loads only the standard Ethereum precompiles. To add custom ones, you need to either extend this collection or replace `build_mainnet()` with a custom builder.

### What a Precompile Registry Is

A **precompile registry** is the data structure that maps EVM addresses to precompile handler functions. When the EVM encounters a `CALL` to address 0x09 (for example), it checks the registry: "is 0x09 a precompile?" If yes, it calls the registered handler function with the call's input data and gas limit. If no, it treats the call as a normal contract call and looks up bytecode in state.

In the context of the changes below, "creating a precompile registry" means replacing daeji's current approach (calling `build_mainnet()` which returns a fixed set of standard precompiles) with a configurable approach where custom precompiles can be added alongside the standard ones.

---

## Priority 1: Critical (Phase 1)

These changes are prerequisites for deploying Solidity contracts to daeji. Without them, multiple contracts will malfunction.

### 1.1 Fix block.timestamp

**Current behavior:** When daeji constructs the `BlockContext` (the set of values that the EVM sees as `block.*` in Solidity), the `timestamp` field is set to the block height (block number). This means at block 1000, `block.timestamp` in Solidity returns `1000`, not `1714000000` (a Unix timestamp representing a real date/time).

**Why this is problematic -- the roko context:**

Roko's knowledge decay formula is `weight = initial * 2^(-elapsed / half_life)`. This formula uses wall-clock seconds throughout. The neuro store (roko's local knowledge database in `crates/roko-neuro/src/`) defines half-lives in days:

```rust
// From roko-neuro/src/lib.rs:
pub const INSIGHT_HALF_LIFE_DAYS: f64 = 30.0;      // Insights decay over 30 days
pub const HEURISTIC_HALF_LIFE_DAYS: f64 = 90.0;    // Heuristics over 90 days
pub const WARNING_HALF_LIFE_DAYS: f64 = 1.0 / 24.0; // Warnings over 1 hour
pub const CAUSAL_LINK_HALF_LIFE_DAYS: f64 = 60.0;   // CausalLinks over 60 days
pub const STRATEGY_FRAGMENT_HALF_LIFE_DAYS: f64 = 14.0; // Strategies over 14 days
```

The on-chain InsightBoard contract's `currentWeight()` function computes the same decay using `block.timestamp`:

```solidity
uint256 age = block.timestamp - entry.timestamp;
uint256 halfLifeSecs = uint256(entry.halfLifeHrs) * 3600;
uint256 halvings = age / halfLifeSecs;
uint256 weight = 1e18 >> halvings;
```

With the current bug, `age` computes elapsed *blocks*, not elapsed *seconds*. At 400ms per block, the effect is:
- An Insight with 7-day on-chain half-life (604,800 seconds = 604,800 "blocks" at timestamp=height) would actually survive for `604,800 * 0.4 = 241,920` real seconds (2.8 days instead of 7 days).
- A Warning with 3-minute on-chain half-life (180 seconds = 180 "blocks") would actually survive for `180 * 0.4 = 72` real seconds (1.2 minutes instead of 3 minutes).

Every Solidity contract in roko's repository that uses `block.timestamp` is affected: `IdentityRegistry.sol` (cooldown periods, timelocks), `ReputationRegistry.sol` (EMA decay windows), and the InsightBoard's `currentWeight()` function.

Beyond roko's contracts, this breaks any standard Solidity pattern that uses `block.timestamp` for timing. Cooldown periods like "call again after `block.timestamp + 86400`" (24 hours = 86,400 seconds) would mean "after 86,400 more blocks" which at ~400ms per block is about 9.6 hours. Timelocks like "funds locked until `block.timestamp >= 1714000000`" would mean "until block 1,714,000,000" which will never be reached.

**The fix conceptually:** Set `timestamp` to the current wall-clock time (Unix seconds) instead of the block height.

**The code change:**

File: `crates/node/consensus/src/app.rs` (or wherever `BlockContext` is constructed for block proposals)

```rust
use std::time::{SystemTime, UNIX_EPOCH};

// In the BlockContext construction, change from:
//   timestamp: height
// To:
let timestamp = SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .unwrap()
    .as_secs();
```

**Risk assessment:** Low. Validators may have slightly different wall clocks (a few seconds of drift is normal). Simplex consensus tolerates this because the timestamp is a field in the proposed block payload, not a value used for consensus decisions. The proposing validator sets the timestamp; other validators accept it as long as it is within a reasonable window.

**Recommended validation:** Verifiers should reject proposed blocks with timestamps more than 30 seconds in the future relative to their own clock. This prevents a malicious proposer from setting far-future timestamps.

**Testing approach:**
1. Apply the change locally.
2. Start the 4-validator devnet.
3. Deploy a simple test contract that stores `block.timestamp` in a public variable.
4. Read the stored value via `eth_call` and verify it is a Unix timestamp (roughly `SystemTime::now()` in seconds), not a block number.
5. Wait a few blocks and read again -- verify the timestamps are monotonically increasing and separated by approximately the block time (400-1000ms).

---

### 1.2 Fix BLOCKHASH Opcode

**Current behavior:** The `block_hash_ref` closure in daeji's REVM configuration always returns `B256::ZERO` (32 bytes of zeros). When Solidity code calls `blockhash(blockNumber)` for any recent block number, it gets zero.

**Why this is problematic -- the roko context:**

Roko agents need recent block hashes for three purposes:

1. **VRF seed derivation.** The CascadeRouter (roko's model selection bandit in `crates/roko-learn/src/`) can use the chain's VRF output (`prevrandao` from BLS12-381 threshold consensus) for verifiable random model selection. Some VRF derivation schemes combine `prevrandao` with recent block hashes for additional entropy: `seed = hash(prevrandao, blockhash(block.number - 1), task_id)`. With BLOCKHASH returning zero, the hash component contributes nothing.

2. **Commit-reveal schemes.** Roko's prediction system (the `CalibrationTracker` in `crates/roko-learn/src/prediction.rs`) registers predictions before task execution. A commit-reveal pattern anchors the commitment against a specific block hash: the agent commits `hash(prediction, blockhash(N))` at block N, then reveals the prediction after the task completes. This proves the prediction was made before the outcome was known. With BLOCKHASH returning zero, the commitment is trivially forgeable.

3. **Audit trail references.** The `CustodyLogger` (in `crates/roko-agent/src/safety/provenance.rs`) records provenance chains for agent actions. Block hashes serve as anchor points: "this action was logged relative to block N with hash H." With zero hashes, the reference is meaningless.

Additionally, daeji has a superior randomness source (the VRF output from threshold consensus, available as `prevrandao`/`mixHash`), so randomness derivation specifically is better served by `block.prevrandao`. But `BLOCKHASH` returning zero is still an EVM compliance issue that breaks existing Solidity libraries.

**The fix conceptually:** Maintain a ring buffer (a fixed-size circular data structure) of the most recent 256 block hashes. After each block is finalized, push its hash into the buffer. When the EVM requests a block hash, look it up in the buffer.

**The code change:**

File: `crates/node/executor/src/revm.rs`

```rust
use alloy_primitives::B256;
use std::collections::VecDeque;

/// A ring buffer that stores the hashes of the most recent N blocks.
/// The EVM spec requires BLOCKHASH to return hashes for the last 256 blocks.
pub struct BlockHashCache {
    /// Ordered pairs of (block_number, block_hash).
    /// Newest entries are at the back; oldest at the front.
    hashes: VecDeque<(u64, B256)>,
    /// Maximum number of entries to retain (256 per EVM spec).
    max_depth: usize,
}

impl BlockHashCache {
    pub fn new() -> Self {
        Self {
            hashes: VecDeque::with_capacity(256),
            max_depth: 256,
        }
    }

    /// Look up the hash for a given block number.
    /// Returns B256::ZERO if the block is not in the cache (too old or unknown).
    pub fn get(&self, number: u64) -> B256 {
        self.hashes
            .iter()
            .find(|(n, _)| *n == number)
            .map(|(_, h)| *h)
            .unwrap_or(B256::ZERO)
    }

    /// Record a newly finalized block's hash.
    /// Call this after each block is finalized, before processing the next block.
    pub fn push(&mut self, number: u64, hash: B256) {
        self.hashes.push_back((number, hash));
        while self.hashes.len() > self.max_depth {
            self.hashes.pop_front();
        }
    }
}
```

In the EVM construction code, replace the closure that returns `B256::ZERO` with one that queries the cache:

```rust
// Where the EVM block context is configured:
let block_hash_cache = block_hash_cache.clone(); // Shared via Arc<RwLock<...>> or similar
let block_hash_ref = move |number: u64| -> B256 {
    block_hash_cache.read().unwrap().get(number)
};
```

After each block is finalized:
```rust
// In the finalization handler:
block_hash_cache.write().unwrap().push(block_height, block_hash);
```

**Risk assessment:** Low. The ring buffer is a simple, well-understood data structure. The data it stores (block hashes) is already known to the executor. The change is additive -- it does not modify any existing data flows.

**Testing approach:**
1. Apply the change.
2. Start the devnet and wait for at least 5 blocks to finalize.
3. Deploy a test contract:
   ```solidity
   contract BlockHashTest {
       function getRecentBlockHash() public view returns (bytes32) {
           return blockhash(block.number - 1);
       }
   }
   ```
4. Call `getRecentBlockHash()` via `eth_call`. Verify the result is non-zero.
5. Call it repeatedly over several blocks. Verify each result is different (each block has a unique hash).
6. Test the boundary: request a block hash for a block more than 256 blocks ago. Verify it returns zero (per spec).

---

## Priority 2: Important (Phase 2)

These changes enable advanced agent features. They are not required for basic contract deployment but unlock significant capabilities.

### 2.1 Custom Precompile Registration

**Current behavior:** Daeji's executor calls `ctx.build_mainnet()` to configure REVM. This function loads only the 9 standard Ethereum precompiles (ecrecover, sha256, ripemd160, etc.) and returns a handler that cannot be extended. There is no way to add custom precompile addresses without modifying this call.

**Why this is problematic -- the roko context:**

The agent integration requires at least one custom precompile: the HDC (Hyperdimensional Computing) similarity search at address 0x09. Here is why it must be a precompile rather than a Solidity implementation:

Roko's HDC system (in `crates/roko-primitives/src/hdc.rs`) uses 10,240-bit binary vectors stored as `[u64; 160]`. Similarity is computed via Hamming distance: XOR two vectors word-by-word, then count the differing bits via hardware POPCNT. The core struct:

```rust
// From roko-primitives/src/hdc.rs:
pub const HDC_BITS: usize = 10_240;
pub const HDC_BYTES: usize = 1_280;

pub struct HdcVector {
    bits: [u64; 160],
}
```

A single comparison takes 160 XOR operations + 160 POPCNT operations = ~320 CPU instructions. Searching 100,000 entries (a realistic knowledge store size for a fleet of agents) takes ~32 million instructions, completing in approximately **170 microseconds** on modern hardware with SIMD.

In Solidity, the same operation would be devastatingly expensive:
- Each `XOR` on a `uint256` costs 3 gas. 10,240 bits = 40 `uint256` words = 40 XOR operations = 120 gas per comparison.
- Each `POPCNT` in Solidity requires a bit-counting loop (~50 gas per word). 40 words = ~2,000 gas per comparison.
- Total per comparison: ~2,120 gas.
- For 100 comparisons: ~212,000 gas. Feasible but slow.
- For 10,000 comparisons: ~21.2M gas. Approaches the block gas limit.
- For 100,000 comparisons: ~212M gas. Impossible in a single transaction.

The precompile achieves the same result for a flat 50,000 gas regardless of entry count, because the actual computation happens in native Rust outside the EVM gas metering. Without modifying the executor to support custom precompiles, this operation cannot be registered.

**The fix conceptually:** Create a configurable precompile registry that includes both the standard Ethereum precompiles and any custom ones. Replace `build_mainnet()` with a custom builder that accepts additional precompile registrations.

**The code change:**

File: `crates/node/executor/src/revm.rs`

```rust
use revm::handler::precompile::PrecompileSet;

/// A precompile registry that combines standard Ethereum precompiles
/// with custom agent-specific precompiles.
pub struct KoraPrecompiles {
    standard: EthPrecompiles,
    custom: Vec<(Address, Box<dyn Precompile>)>,
}

impl PrecompileSet for KoraPrecompiles {
    fn run(
        &self,
        address: &Address,
        input: &[u8],
        gas_limit: u64,
    ) -> Option<PrecompileResult> {
        // Check custom precompiles first
        for (addr, precompile) in &self.custom {
            if addr == address {
                return Some(precompile.run(input, gas_limit));
            }
        }
        // Fall through to standard Ethereum precompiles
        self.standard.run(address, input, gas_limit)
    }
}
```

Register custom precompiles in the executor construction:

```rust
// Replace:
//   let handler = ctx.build_mainnet();
// With:
let kora_precompiles = KoraPrecompiles {
    standard: EthPrecompiles::default(),
    custom: vec![
        (HDC_SEARCH_ADDRESS, Box::new(hdc_precompile)),
    ],
};

let handler = ctx.build_with_precompiles(kora_precompiles);
```

The registry should be configurable per-node so that deployments without agent precompiles can run a vanilla EVM.

**Risk assessment:** Medium. Changing how the EVM is constructed affects all transaction execution. However, REVM is explicitly designed for precompile extensibility -- this is a supported use case. The standard precompiles are unchanged; we are only adding to the set.

**Testing approach:**
1. Apply the change with no custom precompiles registered. Run the full e2e test suite. Verify nothing breaks (this confirms the refactor is transparent).
2. Register a trivial custom precompile (e.g., one that returns its input reversed) at a test address.
3. Deploy a contract that calls this address and verify the precompile is invoked.
4. Run the standard e2e tests again to confirm standard precompiles still work.

---

### 2.2 Add WebSocket Subscription Support

**Current behavior:** Daeji exposes only HTTP JSON-RPC. There is no WebSocket endpoint. Clients that want to learn about new blocks or new events must poll `eth_blockNumber` and `eth_getLogs` at regular intervals.

**Why this is problematic -- the roko context:**

Roko has extensive real-time infrastructure that needs push-based event delivery from the chain:

1. **roko-serve (HTTP control plane).** The `roko serve` command starts an HTTP server on port 6677 with ~85 REST routes plus SSE (Server-Sent Events) and WebSocket endpoints. The SSE stream pushes live updates to connected dashboards: task progress, gate results, agent status, knowledge changes. Chain events (new blocks, knowledge posts, heartbeat confirmations) need to flow into this stream.

2. **ratatui TUI (interactive dashboard).** The `roko dashboard` command starts a terminal UI built with ratatui. It uses `notify::RecommendedWatcher` (a file system watcher in `crates/roko-cli/src/tui/fs_watch.rs`) to react to changes in `.roko/` files. Chain events need to appear in the TUI's event feed without manual refresh.

3. **roko-conductor (10 watchers + circuit breaker).** The `Conductor` (in `crates/roko-conductor/src/`) manages 10 concurrent watchers that monitor agent health, stuck detection, heartbeat timeouts, and anomaly detection. The `HealthMonitor` checks agent liveness. With chain heartbeats, the conductor needs to react when an agent misses its heartbeat window -- waiting for the next poll cycle (1-10 seconds) adds unnecessary latency.

4. **Knowledge sync.** The `NeuroChainSync` component (described in the integration docs) needs to know when new `InsightPosted` events appear on-chain. Polling every N blocks means agents do not learn about new knowledge until the next poll. WebSocket subscriptions with `eth_subscribe("logs", filter)` deliver events within milliseconds of block finalization.

Without WebSocket support, all of these systems must poll. At daeji's ~400ms block time, polling every second wastes ~60% of requests (no new data). Polling every 5 seconds means agents are 2.5 seconds behind on average.

**The fix conceptually:** jsonrpsee (the RPC library daeji already uses) natively supports WebSocket subscriptions. The same server that handles HTTP JSON-RPC can simultaneously handle WebSocket connections. Subscription methods use the `#[subscription]` attribute instead of `#[method]`, and the server pushes events to connected clients via a `SubscriptionSink`.

**The code change:**

File: `crates/node/rpc/src/server.rs` (new file, or extend the existing RPC module)

```rust
use jsonrpsee::proc_macros::rpc;
use jsonrpsee::types::SubscriptionResult;
use jsonrpsee::PendingSubscriptionSink;

#[rpc(server, namespace = "eth")]
pub trait EthSubscriptionApi {
    /// Subscribe to real-time events.
    /// `kind` specifies what to subscribe to:
    ///   - "newHeads": notification each time a new block is finalized
    ///   - "logs": event log notifications matching a filter
    ///   - "newPendingTransactions": notification when new txs enter the mempool
    #[subscription(
        name = "subscribe" => "subscription",
        unsubscribe = "unsubscribe",
        item = serde_json::Value
    )]
    async fn subscribe(
        &self,
        kind: String,
        params: Option<serde_json::Value>,
    ) -> SubscriptionResult;
}

impl EthSubscriptionApiServer for RpcServerImpl {
    async fn subscribe(
        &self,
        pending: PendingSubscriptionSink,
        kind: String,
        params: Option<serde_json::Value>,
    ) -> SubscriptionResult {
        match kind.as_str() {
            "newHeads" => {
                let sink = pending.accept().await?;
                let mut rx = self.ledger.subscribe();

                while let Ok(event) = rx.recv().await {
                    if let LedgerEvent::SnapshotPersisted(digest) = event {
                        let block = self.get_block_for_digest(digest).await;
                        let header = serialize_block_header(&block);
                        if sink.send(header).await.is_err() {
                            break; // Client disconnected
                        }
                    }
                }
                Ok(())
            }
            "logs" => {
                let sink = pending.accept().await?;
                let filter = parse_log_filter(params)?;
                let mut rx = self.ledger.subscribe();
                while let Ok(event) = rx.recv().await {
                    if let LedgerEvent::SnapshotPersisted(digest) = event {
                        let logs = self.get_logs_for_digest(digest, &filter).await;
                        for log in logs {
                            if sink.send(serde_json::to_value(&log)?).await.is_err() {
                                break;
                            }
                        }
                    }
                }
                Ok(())
            }
            _ => {
                pending.reject(jsonrpsee::types::ErrorCode::InvalidParams).await;
                Ok(())
            }
        }
    }
}
```

**Risk assessment:** Low-medium. The HTTP JSON-RPC server is unchanged. WebSocket support is additive. The main risk is resource management: each WebSocket subscription holds a connection and a broadcast receiver. Need to limit the number of concurrent subscriptions and clean up on disconnect.

**Testing approach:**
1. Apply the change.
2. Start the devnet.
3. Use `websocat` (a CLI WebSocket client) or a simple script to connect and subscribe to `newHeads`.
4. Verify that notifications arrive each time a block finalizes (~400ms apart).
5. Subscribe to `logs` with a filter matching the InsightBoard's `InsightPosted` event. Post a knowledge entry. Verify the log notification arrives.

---

### 2.3 Extend kora_ RPC Namespace

**Current behavior:** The `kora_` namespace has only one method: `kora_nodeStatus`, which returns consensus health metrics (current view, finalized block count, nullification count, peer count, whether this node is the current leader).

**Why more methods are needed -- the roko context:**

Roko agents making decisions based on chain state currently must make multiple standard `eth_*` calls and decode the results themselves. Custom `kora_*` methods provide pre-decoded, agent-relevant data:

1. **`kora_activeAgents`**: Returns agents from the AgentRegistry whose `lastSeen` is within the heartbeat window. Without this, an agent must call `eth_call` to query `AgentRegistry.agentCount()`, then loop through each agent ID calling `AgentRegistry.agents(id)`, then filter by `lastSeen`. This is N+1 RPC calls for N agents.

2. **`kora_recentKnowledge`**: Returns decoded `InsightPosted` events since a given block. Without this, agents must call `eth_getLogs` with the correct topic hash, then ABI-decode each log's data field manually. The `kora_recentKnowledge` method does this server-side and returns clean JSON.

3. **`kora_vrfSeed`**: Returns the VRF seed (`prevrandao`) for a specific block. The CascadeRouter and ExperimentStore (roko's model selection and A/B testing systems in `crates/roko-learn/src/`) can use this for verifiable random decisions: `model_index = hash(vrf_seed, task_id) % models.len()`. Without this method, agents must call `eth_getBlockByNumber` and extract the `mixHash` field -- doable but less ergonomic.

4. **`kora_agentReputation`** (future): Query an agent's reputation scores across the 7 domains tracked by `ReputationRegistry.sol`. Roko's CascadeRouter could factor reputation into model selection -- preferring models that agents with high reputation scores tend to use.

5. **`kora_knowledgeBySimilarity`** (future, requires precompile): Submit an HDC query vector and get the top-K most similar knowledge entries. This wraps the HDC precompile in a convenient RPC call.

**The code change:**

File: `crates/node/rpc/src/kora.rs`

```rust
use jsonrpsee::proc_macros::rpc;

/// Extended kora_ namespace with agent-relevant query methods.
#[rpc(server, namespace = "kora")]
pub trait KoraExtendedApi {
    /// Return the VRF seed (prevrandao) for a specific finalized block.
    ///
    /// The VRF seed is the output of the BLS12-381 threshold signature over
    /// the view number. It is deterministic, unpredictable, and bias-resistant.
    #[method(name = "vrfSeed")]
    async fn vrf_seed(&self, block_number: u64) -> RpcResult<B256>;

    /// Return recent finalized blocks with full detail.
    #[method(name = "recentBlocks")]
    async fn recent_blocks(&self, count: u64) -> RpcResult<Vec<BlockDetail>>;

    /// Return consensus health metrics (superset of kora_nodeStatus).
    #[method(name = "consensusHealth")]
    async fn consensus_health(&self) -> RpcResult<ConsensusHealth>;
}

#[derive(Serialize, Deserialize)]
pub struct BlockDetail {
    pub number: u64,
    pub hash: B256,
    pub vrf_seed: B256,
    pub tx_count: usize,
    pub gas_used: u64,
    pub timestamp: u64,
}

#[derive(Serialize, Deserialize)]
pub struct ConsensusHealth {
    pub current_view: u64,
    pub finalized_count: u64,
    pub nullified_count: u64,
    pub peer_count: usize,
    pub is_leader: bool,
    pub avg_block_time_ms: f64,
    pub participation_rate: f64,
}
```

**Risk assessment:** Very low. These are read-only methods that query existing data. They do not modify state or consensus behavior.

**Testing approach:**
1. Apply the change.
2. Start the devnet and wait for several blocks.
3. Call `kora_vrfSeed` with a recent block number. Verify the result is non-zero and 32 bytes.
4. Call `kora_recentBlocks` with count=5. Verify 5 blocks are returned with increasing block numbers.
5. Call `kora_consensusHealth`. Verify `finalized_count > 0` and `participation_rate > 0`.

---

### 2.4 Set Non-Zero Coinbase/Beneficiary

**Current behavior:** The `beneficiary` field in the EVM block context (exposed to Solidity as `block.coinbase`) is always set to `Address::ZERO` -- the all-zeros Ethereum address. There are no block rewards.

**Why this is problematic -- the roko context:**

In production with the GNOS token, `block.coinbase` identifies the block proposer. This is relevant for:

1. **Gas cost attribution.** Roko tracks execution costs via `CostsLog` and `CostRecord` (in `crates/roko-learn/src/costs_db.rs`). Each episode records `cost_usd` in its `Usage` struct. When agents pay gas for chain transactions (witness anchoring, knowledge posts, heartbeats), the gas cost needs to be attributed to the correct validator. `block.coinbase` identifies who earned that gas.

2. **Validator-specific logic.** The `AgentRegistry.sol` contract could restrict certain operations to the current block proposer (e.g., "only the proposer can finalize a batch of confirmations per block"). With zero coinbase, all proposer-specific logic is broken.

3. **Fee distribution.** The `FeeDistributor.sol` contract distributes fees to validators. It needs to know which validator proposed each block.

While no current roko contracts strictly depend on coinbase, it is an EVM compliance issue and a prerequisite for production token economics.

**The fix conceptually:** Set `beneficiary` to an Ethereum address derived from the proposing validator's identity. Daeji validators use Ed25519 keys for P2P identity (not Ethereum's secp256k1). We need a deterministic mapping from Ed25519 public key to Ethereum address.

**The code change:**

File: `crates/node/consensus/src/app.rs`

```rust
// Deterministic derivation: keccak256 of Ed25519 public key, last 20 bytes.
let beneficiary = Address::from_slice(
    &keccak256(validator_pubkey.as_bytes())[12..]
);
```

Alternative: a lookup table in the genesis configuration mapping Ed25519 keys to chosen Ethereum addresses.

**Risk assessment:** Very low. The beneficiary address is informational -- it does not affect consensus or block validity.

**Testing approach:**
1. Apply the change.
2. Start the devnet.
3. Deploy a test contract that returns `block.coinbase`.
4. Call it via `eth_call`. Verify the result is non-zero and consistent for the same proposer.
5. Wait for a leader rotation and call again. Verify a different address is returned.

---

## Priority 3: Nice to Have (Phase 3)

These changes improve production readiness but are not required for development or testing.

### 3.1 Variable Validator Set Size

**Current behavior:** Leader election uses the formula `(view % 4) as u32 == validator_index`. The number 4 is hardcoded.

**Why this matters (eventually):** A production deployment may want more (or fewer) than 4 validators. The agent-chain vision includes dynamic validator sets where agents earn validator slots based on reputation -- this is impossible without making the set size configurable.

**The code change:**

```rust
// Change from:
//   is_leader: (view % 4) as u32 == validator_index
// To:
is_leader: (view % self.validator_count) as u32 == self.validator_index
```

**Risk assessment:** Very low for the configurable constant. Dynamic sets require the resharing protocol.

---

### 3.2 Real EIP-1559 Base Fee

**Current behavior:** `gasPrice` and `maxPriorityFeePerGas` are hardcoded to 1 gwei. `base_fee_per_gas` is always 0. Gas is effectively free.

**Why this matters (eventually):** Without gas costs, there is no economic disincentive to spam the chain. For a controlled devnet this is fine, but for multi-operator deployments, gas economics provide a fundamental spam barrier. Roko's `CostRecord` (in `crates/roko-learn/src/costs_db.rs`) already tracks per-episode costs; real gas costs would flow into the same accounting pipeline.

**The code change:** Implement the EIP-1559 base fee algorithm: track `gas_used` per block, compute next-block base fee (increase if >50% utilization, decrease if <50%, 12.5% adjustment factor), enforce `maxFeePerGas >= base_fee` for transaction acceptance.

**Risk assessment:** Medium. Gas pricing affects transaction acceptance.

---

### 3.3 EIP-4844 Blob Sidecar Support

**Current behavior:** Daeji decodes EIP-4844 blob-carrying transactions at the RPC layer, but blob sidecars are not stored or served.

**Why this matters (eventually):** EIP-4844 is Ethereum's mechanism for cheap data availability. Not needed until daeji supports data availability for external systems.

**Risk assessment:** Low. Additive change. Not needed for agent integration.

---

## Changes That Should NOT Be Made

The following changes were proposed in the original agent-chain design documents but are being deferred or rejected.

### Block-STM Parallel Execution

**What was proposed:** Replace sequential EVM execution with Block-STM (Software Transactional Memory).

**Why it should be deferred:** Premature optimization. Daeji's sequential execution handles the expected load. Block-STM introduces significant complexity. Revisit if block utilization consistently exceeds 50%.

### Extended Block Header (sm_root, active_golems, insight_count)

**What was proposed:** Add custom fields to the block header.

**Why it should be deferred:** These values can be tracked in smart contract state instead, with far less engineering effort and no impact on the consensus protocol, codec, or P2P wire format.

### Validator Slashing and Jailing

**What was proposed:** Penalize misbehaving validators by destroying staked tokens.

**Why it should be deferred:** The devnet has 4 trusted validators. There is no adversarial environment and no staked value. This is a production-scale concern.

### Dynamic Validator Set

**What was proposed:** Allow validators to join and leave at runtime.

**Why it should be deferred:** Requires the resharing protocol from commonware, which is a significant undertaking. Phase 3+ work.

### GNOS Minting in Block Rewards

**What was proposed:** Mint GNOS tokens as block rewards.

**Why it should be deferred:** Token minting should be a smart contract feature, not a protocol-level feature. This keeps the protocol simple and economic policy flexible.
