# 04 — Agent Surface

How agents use verified chain state: tool handlers, sidecar routes, MCP tools,
IDE tiles, coordination bus, and foreign runtime access.

---

## 1. Wiring chain tool handlers

roko-chain defines 17 `ToolDef` entries with no dispatch handlers. Wire them:

| Existing ToolDef | Handler wiring | Needs |
|-----------------|----------------|-------|
| `chain.balance` | `client.get_balance(addr, block)` | ChainClient (or VerifiedChainClient) |
| `chain.transfer` | `wallet.sign_and_submit(tx)` | ChainWallet |
| `chain.gas_estimate` | `client.eth_call(tx, block)` | ChainClient |
| `chain.simulate_tx` | `client.eth_call(tx, block)` | ChainClient |
| `chain.approve` | ERC-20 approve via `wallet.sign_and_submit` | ChainWallet + ABI |
| `chain.swap` | DEX router call | ChainWallet + ABI |
| `chain.add_liquidity` | Pool contract call | ChainWallet + ABI |
| `chain.remove_liquidity` | Pool contract call | ChainWallet + ABI |
| `chain.get_pool_info` | `client.eth_call` with pool ABI | ChainClient |
| `chain.get_position` | `client.eth_call` with position ABI | ChainClient |
| `chain.wallet_create` | Local key generation | None (local) |
| `chain.wallet_list` | Read config | None |
| `chain.wallet_info` | `client.get_balance` + `wallet.nonce` | ChainClient + ChainWallet |
| `chain.wallet_export_address` | `wallet.address()` | ChainWallet |
| `chain.post_insight` | InsightBoard contract call | ChainWallet + ABI |
| `chain.search_insights` | InsightBoard query | ChainClient + ABI |
| `chain.confirm_insight` | InsightBoard confirm call | ChainWallet + ABI |

**New tool defs to add:**

| New ToolDef | What | Handler |
|-------------|------|---------|
| `chain.verified_balance` | Get LC-verified balance | VerifiedChainClient |
| `chain.verified_storage` | Get LC-verified storage slot | VerifiedChainClient |
| `chain.verify_transfer` | Verify a transfer happened | VerifiedChainClient + receipt proof |
| `chain.head` | Latest verified block header | ConsensusVerifier |
| `chain.backends` | List configured chain backends | Config |
| `chain.mpp_pay` | MPP one-time payment + verification | MppClient |
| `chain.mpp_session` | MPP session payment | MppClient |
| `chain.subscribe_events` | Subscribe to verified chain events | ChainWatcherTask |

### Handler registration

```rust
// In agent startup or sidecar construction
let chain_handler = ChainToolHandler::new(
    verified_client,  // Arc<VerifiedChainClient>
    wallet,           // Option<Arc<dyn ChainWallet>>
    mpp_client,       // Option<Arc<MppClient>>
);

// Register with ToolDispatcher
dispatcher.register_domain("chain", chain_handler);
```

The `ToolDispatcher` already supports domain-based dispatch. Chain tools are the
`"chain"` domain.

---

## 2. Sidecar routes

`AgentState.chain_client` exists but is unused at runtime. Activate it:

```
POST   /chain/query            → verified chain query (balance, storage, logs)
POST   /chain/verify-transfer  → verify a specific transfer
GET    /chain/head/:network    → latest verified head
GET    /chain/backends         → configured backends + health
POST   /chain/mpp/pay          → MPP payment + verification
POST   /chain/subscribe        → start event subscription (WebSocket upgrade)
```

These are new routes in `roko-agent-server/src/features/chain.rs`, following the
existing pattern in `features/research.rs`, `features/tasks.rs`, etc.

### Request/response examples

```json
// POST /chain/query
{
  "network": "tempo-mainnet",
  "query": "balance",
  "address": "0xABC...DEF"
}

// Response
{
  "data": "1500000000",
  "chain_id": 4217,
  "network": "tempo-mainnet",
  "block_number": 1000042,
  "trust_level": "cryptographic",
  "consensus_mechanism": "threshold_bls",
  "verified_at": 1717200504042
}
```

```json
// POST /chain/mpp/pay
{
  "service_url": "https://api.example.com/data",
  "amount": "1.50",
  "token": "0xUSDC...",
  "mode": "one_time"
}

// Response
{
  "service_response": { /* whatever the service returned */ },
  "settlement": {
    "tx_hash": "0x...",
    "amount": "1500000",
    "trust_level": "cryptographic",
    "block_number": 1000043,
    "memo": "service:api.example.com,request:abc123"
  }
}
```

---

## 3. MCP tools

The sidecar routes are also exposed as MCP tools via `nunchi-mcp` (the standalone
stdio MCP server from nunchi-desktop). Any MCP-aware agent (Roko, Claude, GPT,
Cursor) can call them.

MCP tool catalog (chain subset):

```json
[
  { "name": "chain.query", "description": "Query verified blockchain state" },
  { "name": "chain.verify_transfer", "description": "Verify a transfer with cryptographic proof" },
  { "name": "chain.head", "description": "Latest verified block header" },
  { "name": "chain.backends", "description": "List chain backends + health" },
  { "name": "chain.mpp_pay", "description": "Pay for a service via Tempo MPP + verify settlement" }
]
```

---

## 4. IDE tiles (nunchi-desktop)

The widget tile system (17 kinds) from nunchi-desktop naturally renders chain data.
Agents create chain tiles via `nunchi.tiles.create`:

```json
{
  "type": "widget",
  "title": "Tempo USDC Balance",
  "initialState": {
    "kind": "metric",
    "source": "mcp://chain.query",
    "sourceArgs": { "network": "tempo-mainnet", "query": "balance", "address": "0xABC" },
    "path": "data",
    "format": "currency_usd",
    "refreshMs": 6000,
    "badge": {
      "field": "trust_level",
      "map": {
        "cryptographic": { "text": "Verified", "color": "green" },
        "rpc_trusted": { "text": "RPC", "color": "yellow" },
        "playback": { "text": "Demo", "color": "gray" }
      }
    }
  }
}
```

Other useful tile kinds for chain data:

| Kind | Chain use |
|------|----------|
| `metric` | Single verified value (balance, block number) |
| `metric-grid` | Multi-chain overview (balances across Tempo + Ethereum + daeji) |
| `status` | Backend health ("tempo-mainnet: threshold_bls, verified, block 1M") |
| `log` | Live verified event stream (header advances, transfer proofs) |
| `sparkline` | Verified balance over time |
| `kv` | Chain backend details (chain ID, trust level, latest block, peer count) |

The trust badge is driven by the `trust_level` field in `VerifiedState`. The tile
code is chain-agnostic — it reads the field and renders the appropriate badge.

---

## 5. Coordination bus integration

Verified chain events feed into the existing coordination channels:

```
ChainWatcherTask
  │ new verified block / event
  │
  ├──► KnowledgeStore.ingest()
  │     kind: Insight
  │     confidence: 0.99 (cryptographic) / 0.80 (rpc_trusted)
  │     source: "chain:tempo-mainnet:block:1000042"
  │
  ├──► roko-serve SSE /api/events
  │     event: "chain.tempo.transfer"
  │     data: VerifiedState<TransferEvent>
  │
  ├──► TriagePipeline
  │     (existing: rule filter → anomaly scoring → enrichment → action)
  │     IngestKnowledge | AlertConductor | MarketplaceHandler | Drop
  │
  └──► Conductor watchers (future)
        (e.g., a "ChainAnomalyWatcher" that detects unusual on-chain patterns)
```

The `KnowledgeStore` integration is the key novel piece: verified chain observations
become high-confidence knowledge entries with cryptographic provenance. The confidence
maps from `TrustLevel`:
- Cryptographic → 0.99
- RPC trusted → 0.80
- Playback → 0.0 (not ingested)

Half-life decay still applies — even verified chain state becomes stale as new blocks
arrive. But the initial confidence is higher than agent self-reports.

---

## 6. Foreign runtime access

From the generalized agentic stack (PR #146), external agents join via MCP, SDK, or
HTTP. Chain tools are accessible through all paths:

| Join path | Chain tool access |
|-----------|-------------------|
| MCP gateway | `chain.query`, `chain.mpp_pay`, etc. as MCP tools |
| SDK shim | `agent.chain.query()` typed API (Python/TypeScript) |
| HTTP gateway | `POST /api/chain/query` REST endpoint |
| Runtime adapter (Roko) | Direct `ChainToolHandler` in tool loop |

A LangChain agent that joins via MCP can query verified Tempo state and make MPP
payments without understanding BLS signatures or MPT proofs. The roko runtime does
the verification; the foreign agent consumes `VerifiedState<T>`.

---

## 7. On-chain anchoring

Verified external state feeds into Nunchi's on-chain contracts:

| Contract | What verified chain state provides |
|----------|-----------------------------------|
| `InsightBoard.sol` | Verified chain observations as high-confidence insights (with proof hash) |
| `BountyMarket.sol` | Cross-chain payment proofs for settlement (MPP receipt + LC proof) |
| `ReputationRegistry.sol` | Proof-backed reputation updates (verified payment, verified execution) |
| `ValidationRegistry.sol` | Chain proofs as challenge evidence (deterministic, reproducible) |

The key integration: when a bounty settles via Tempo MPP, the agent submits the
LC-verified payment receipt to `BountyMarket`. The receipt includes the consensus
proof hash and state proof hash. Anyone can re-verify by fetching the same block
from Tempo and checking the proofs.

---

## 8. Novel workflows enabled

### Trustless data marketplace

```
Buyer agent:
  1. Find data provider via InsightBoard reputation
  2. Pay via MPP (one-time mode) → service returns data
  3. Verify payment settled via LC → store verified receipt
  4. Evaluate data quality (gate pipeline)
  5. If good: post to InsightBoard with verified provenance
     "Data from provider 0xDEF, paid 1.50 USDC (Tempo block N, verified)"
  6. Other agents see high-confidence insight with payment proof
```

### Multi-chain position monitoring

```
Watcher agent:
  1. Subscribe to Tempo events (DeFi positions) via ChainWatcherTask
  2. Subscribe to Ethereum events (L1 collateral) via same interface
  3. Cross-chain risk calculation: Tempo position + Ethereum collateral
  4. Both inputs cryptographically verified
  5. Publish risk assessment to InsightBoard with dual-chain proofs
```

### Agent-to-agent service mesh

```
Agent A needs inference from Agent B:
  1. Agent A discovers Agent B via ERC-8004 passport (capabilities + reputation)
  2. Agent A pays Agent B via MPP session (pay-per-request)
  3. Agent B delivers inference results
  4. Agent A verifies each payment via LC
  5. Session closes → final settlement verified via LC
  6. Both agents' reputations updated with verified payment/delivery proofs
```

### Self-improving data quality

```
Research agent:
  1. Queries InsightBoard for data on topic X
  2. Multiple entries exist with different confidence levels
  3. Entries with chain-verified provenance ("paid $Y for this from provider Z")
     rank higher than unverified claims
  4. Agent uses verified entries, publishes improved insight
  5. New insight cites sources with on-chain proof chain
  6. Knowledge store naturally surfaces the best-verified information
```
