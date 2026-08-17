# RPC Enablement — What New Capabilities Unlock

## Current State (What Works Now)

The kora RPC layer exposes ~25 methods. All are **poll-based** — the client asks, the server answers.
No push. No streaming. No subscriptions.

**Available now:**
- Block queries (`eth_getBlockByNumber/Hash`)
- State reads (`eth_getBalance`, `eth_getCode`, `eth_getStorageAt`)
- Transaction lifecycle (`eth_sendRawTransaction`, `eth_getTransactionByHash`, `eth_getTransactionReceipt`)
- Execution simulation (`eth_call`, `eth_estimateGas`)
- Fee data (`eth_gasPrice`, `eth_feeHistory`)
- Logs (`eth_getLogs` with block range filters)

**What we can build with polling alone:**
- Block-by-block visualization (poll `eth_blockNumber` at 1s)
- Historical state exploration (read any past block)
- Account balance tracking over time
- Gas/fee waveforms from `eth_feeHistory`
- Log/event search across block ranges
- Transaction detail views

**Limitations of polling:**
- 1-second minimum latency (block time = poll interval)
- No sub-block visibility (can't see pending txns)
- No instant event notification (must poll `eth_getLogs` to detect new events)
- Client does redundant work checking "anything new?" every second
- Scales poorly — 100 browser tabs = 100x the RPC load

---

## Tier 1: `eth_subscribe` (Highest Impact)

### What it is
WebSocket push. Client subscribes once, server pushes events as they happen.
The jsonrpsee crate already supports this via `#[subscription]` — the transport is wired, the semantics aren't.

### Subscription types to implement

#### `newHeads` — New block headers
```json
{"jsonrpc":"2.0","method":"eth_subscribe","params":["newHeads"]}
```
**Pushes:** Full block header the instant it's finalized.
**Explorer unlock:** Zero-latency block arrival. The terrain tile / waterfall block / constellation pulse triggers the instant the block exists — not 0-1000ms later when a poll happens to fire. This is the difference between "live" and "laggy."

#### `logs` — Filtered event stream
```json
{"jsonrpc":"2.0","method":"eth_subscribe","params":["logs", {"address":"0x...", "topics":["0x..."]}]}
```
**Pushes:** Log entries matching the filter as blocks finalize.
**Explorer unlock:** Real-time event particle effects. Contract emits a Transfer → a light arc traces sender→receiver *instantly*. No polling delay. Can filter to specific contracts/events for focused visualizations.

#### `newPendingTransactions` — Mempool visibility
```json
{"jsonrpc":"2.0","method":"eth_subscribe","params":["newPendingTransactions"]}
```
**Pushes:** Transaction hash (or full tx) when it enters the mempool.
**Explorer unlock:** **This is the big one for visuals.** Right now, transactions appear fully formed in a block. With pending tx streaming, you see the *lifecycle*:

1. Transaction enters mempool → particle spawns at sender address, glowing, unresolved
2. Transaction sits pending → particle orbits, pulsing, waiting
3. Transaction included in block → particle *snaps* into the block, crystallizes
4. Transaction reverted → particle shatters, red flash

This gives the explorer **anticipation** — you see activity *before* it resolves. The chain feels alive between blocks, not just at block boundaries.

#### `syncing` — Sync status changes
```json
{"jsonrpc":"2.0","method":"eth_subscribe","params":["syncing"]}
```
**Pushes:** Sync progress updates.
**Explorer unlock:** Node health heartbeat. Not critical for visuals but completes the picture.

### Implementation in kora

The infrastructure exists. `jsonrpsee` subscription support, `tokio::broadcast` channels, and `LedgerEvent` notifications from block finalization are all in place. Rough shape:

```rust
// In eth.rs trait
#[subscription(name = "subscribe" unsubscribe = "unsubscribe", item = Value)]
async fn subscribe(&self, kind: String, params: Option<Value>) -> SubscriptionResult;

// In EthApiImpl — wire to broadcast channel
// LedgerService already emits events on finalization
// BlockIndex updates could trigger broadcasts
```

Estimated effort: medium. The hard part is routing finalization events to per-subscription broadcast channels with correct filtering (especially for `logs`).

---

## Tier 2: Filter API (Moderate Impact)

### What it is
Server-side filters for HTTP clients (no WebSocket required).

```
eth_newBlockFilter        → returns filter ID
eth_newPendingTransactionFilter
eth_newFilter             → custom log filter
eth_getFilterChanges      → poll for new matches since last call
eth_uninstallFilter       → cleanup
```

### Explorer unlock
Allows efficient polling without WebSocket. Client creates a filter, then polls `eth_getFilterChanges` to get only *new* items since last poll. Much cheaper than re-scanning block ranges with `eth_getLogs`.

**Useful for:** HTTP-only environments, server-side indexers, fallback when WS isn't available.

**Less exciting than subscriptions** — still polling, still has latency — but more compatible.

---

## Tier 3: Debug/Trace APIs (Deep Exploration)

### `debug_traceTransaction` / `debug_traceBlockByNumber`
**What it does:** Returns the full EVM execution trace — every opcode, every state change, every internal call.

**Explorer unlock:** *Transaction X-ray.* Click a transaction → see the internal call tree rendered as a branching visualization:
- Each internal CALL is a branch from the trunk
- SSTORE operations flash as state mutations
- REVERT paths render as broken/red branches
- Gas consumption shown as thickness — fat branches burned more gas
- Stack depth as vertical position

This turns opaque "tx succeeded" into a visible execution story. Especially powerful for complex DeFi interactions where one user action triggers 15 internal calls across 8 contracts.

### `debug_storageRangeAt`
**What it does:** Dumps a range of storage slots for a contract at a given block.

**Explorer unlock:** *Contract memory visualizer.* Render a contract's storage as a grid/heatmap where each slot is a cell. Color by recency of change, brightness by value magnitude. Watch storage evolve over time — you can literally see where a contract "thinks."

### `trace_filter` / `trace_block`
**What it does:** OpenEthereum-style traces — lighter weight than debug, focused on call trees and state diffs.

**Explorer unlock:** *Block anatomy.* Instead of "block has 5 transactions," see the full internal call graph for the entire block rendered as a circuit diagram. Who called whom, what changed, where value flowed.

### Implementation in kora
Requires REVM tracing hooks. REVM supports inspector-based tracing — you'd implement a custom `Inspector` that records operations during re-execution. More work than subscriptions but the REVM integration point exists.

---

## Tier 4: Extended State APIs (Completeness)

### `eth_getBlockReceipts`
**What it does:** Returns all receipts for a block in one call (vs N individual `eth_getTransactionReceipt` calls).

**Explorer unlock:** Batch efficiency. Render block detail views without N+1 queries. Also gives you gas usage distribution across all txns in one shot — good for the gas waveform visual.

### `eth_getProof`
**What it does:** Returns Merkle proofs for account state (balance, nonce, code hash, storage slots).

**Explorer unlock:** *Merkle tree visualization.* Render the actual proof path from state root to a specific value. Each proof node is a visual element — you see exactly how the trie branches to reach a specific account. Powerful educational tool.

### `eth_createAccessList`
**What it does:** Simulates a transaction and returns the set of addresses/storage keys it would access.

**Explorer unlock:** *Dependency graph.* Before a transaction executes, show exactly which accounts and storage slots it will touch. Render as a connection diagram — "this transaction will read from contracts A, B, C and write to D."

---

## Tier 5: Kora-Specific Methods (Unique Differentiator)

These don't exist in standard Ethereum and would be unique to kora.

### `kora_consensusState`
Expose BLS threshold consensus state — which validators signed, round progress, vote tallies.

**Explorer unlock:** *Consensus constellation.* Each validator is a node in a ring. As a round progresses, signed validators glow (rose). Threshold line is visible. When threshold crosses → block crystallizes. You literally watch consensus happen in real-time.

### `kora_mempoolSnapshot`
Expose the current mempool state — pending transactions, their priority, age.

**Explorer unlock:** *Mempool pressure gauge.* Render pending txns as particles in a chamber. Pressure (count) drives visual density. Priority drives vertical position. As blocks include txns, particles are "consumed" upward. Backlog = visual pressure building.

### `kora_stateMetrics`
Expose QMDB metrics — tree depth, page count, compaction state.

**Explorer unlock:** *Storage health.* Render the state database as a living structure — depth, density, hotspots. Operational insight unique to kora.

### `kora_subscribe` (extended subscriptions)
Push events that go beyond Ethereum standard:
- Consensus round events (pre-vote, pre-commit, finalize)
- Mempool events (tx added, tx evicted, tx promoted)
- Peer connection events
- State compaction events

**Explorer unlock:** Full real-time visibility into the node as a living system, not just the chain it produces.

---

## Priority Order for Explorer Impact

| Priority | Method | Visual Impact | Effort |
|----------|--------|---------------|--------|
| **P0** | `eth_subscribe("newHeads")` | Live block arrival, zero latency | Low-Medium |
| **P0** | `eth_subscribe("logs")` | Real-time event particles | Medium |
| **P1** | `eth_subscribe("newPendingTransactions")` | Mempool lifecycle animation | Medium |
| **P1** | `eth_getBlockReceipts` | Batch block detail rendering | Low |
| **P2** | `debug_traceTransaction` | Transaction X-ray / call tree | High |
| **P2** | `kora_consensusState` | Consensus visualization | Medium |
| **P3** | `eth_getProof` | Merkle tree visualization | Medium |
| **P3** | `debug_storageRangeAt` | Contract storage heatmap | High |
| **P3** | `kora_subscribe` (extended) | Full node lifecycle streaming | High |

The single highest-impact addition is **`eth_subscribe("newPendingTransactions")`** — it transforms the explorer from "a thing that shows you blocks after they happen" into "a thing that shows you the chain *living*."
