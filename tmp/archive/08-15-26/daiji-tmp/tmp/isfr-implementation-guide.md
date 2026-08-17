# ISFR Implementation Guide: Contracts → Relay → Feeds → UI

Everything required to deploy canonical ISFR contracts to mirage, integrate with roko + relay via data feeds, and build an engaging UI in the demo-app.

## 1. Canonical Contracts

The production v3.0 ISFR contracts live in **demo-ide**:

| Contract | Path | Lines | Purpose |
|----------|------|-------|---------|
| **ISFROracle.sol** | `roko/demo-ide/demo/contracts/src/` | 550 | Two-level four-class oracle, block-range voting, trust-weighted median, 256-epoch ring buffer |
| **ISFRBountyPool.sol** | `roko/demo-ide/demo/contracts/src/` | 182 | Per-range reward distribution, pro-rata by trust weight, async claiming |
| **IISFROracle.sol** | `roko/demo-ide/demo/contracts/src/` | 134 | Oracle interface (Epoch struct, submitRate, submitRateForRange) |
| **IISFRBountyPool.sol** | `roko/demo-ide/demo/contracts/src/` | 22 | Bounty pool interface (recordRangeReward) |

**Dependencies (also in demo-ide):**

| Contract | Lines | Purpose |
|----------|-------|---------|
| **RoleRegistry.sol** | 83 | RBAC: MANAGER_ROLE, OPERATOR_ROLE, KEEPER_ROLE |
| **WorkerRegistry.sol** | 238 | Stake bonds + EMA reputation (α=0.2) + 4 tiers + 30-day decay |
| **MockERC20.sol** | ~30 | DAEJI test token with open faucet mint() |

**Shared interface:** `IRoleRegistry.sol` in `demo/contracts/shared/interfaces/`

**Why demo-ide, not contracts-core?** contracts-core/main has only stubs (ISFRMinimal with hardcoded values). The full v3.0 implementation exists on branch `jl/isfr-port-r2-oracle` but isn't merged. Demo-ide has the same code, deployed and tested. Use demo-ide as source of truth.

### Spec Compliance

| Feature | Status |
|---------|--------|
| Two-level aggregation (fast + block-range) | ✓ |
| Four source classes (LENDING/STRUCTURED/FUNDING/STAKING) | ✓ |
| Trust-weighted median: `clamp(sqrt(bond/MIN_BOND) × reputation_bp, 0.1, 10.0)` | ✓ |
| Probation-tier exclusion (zero weight) | ✓ |
| Block-range voting with quorum + close delay | ✓ |
| 256-epoch circular ring buffer | ✓ |
| Bounty pool pro-rata reward distribution | ✓ |
| Configurable range parameters (admin-settable) | ✓ |

What it doesn't have (spec Phase 2+): validator-computed oracle, precompile 0xA01, circuit breaker, CRPS scoring. Those are consensus-layer features.

## 2. Deployment to Mirage

### What Mirage Is

Mirage (`roko/apps/mirage-rs/`) is roko's local Ethereum fork simulator — like anvil but with persistent state, ERC-8004 bootstrapping, and chain extensions (HDC, knowledge, stigmergy). Runs on `127.0.0.1:8545`.

### Start Mirage

```bash
# Option A: Via demo-ide app (auto-starts on launch)
cd /Users/will/dev/nunchi/roko/demo-ide && npm run tauri dev

# Option B: Direct binary
mirage-rs \
  --host 127.0.0.1 \
  --port 8545 \
  --chain-id 88888 \
  --block-interval-ms 1000 \
  --state-dir .roko/state
```

Mirage writes status to `/tmp/mirage-8545-status.json` when ready:
```json
{
  "status": "ready",
  "port": 8545,
  "chainId": 88888,
  "erc8004": {
    "identityRegistry": "0x8004A818BFB912233c491871b3d84c89A494BD9e",
    "reputationRegistry": "0x8004A818BFB912233c491871b3d84c89A494Bd9F",
    "validationRegistry": "0x8004a818bfb912233c491871B3D84C89A494Bda0"
  }
}
```

### Compile Contracts

```bash
cd /Users/will/dev/nunchi/roko/demo-ide/demo/contracts
forge build
```

Foundry config (`foundry.toml`): solc 0.8.26, shanghai EVM.

### Deployment Script

The existing `Deploy.s.sol` deploys the full contract suite but **does NOT include ISFROracle or ISFRBountyPool**. You need a custom script or extend the existing one.

**Deployment order (strict):**

```
1. RoleRegistry(deployer)
2. MockERC20("DAEJI", "DAEJI", 18)
3. WorkerRegistry(MockERC20, RoleRegistry)
4. ISFROracle(RoleRegistry, WorkerRegistry)
5. ISFRBountyPool(RoleRegistry, MockERC20, initialRate=1 ether)
```

**Post-deploy wiring:**

```
6. roleRegistry.grantRole(KEEPER_ROLE, keeper1_address)
7. roleRegistry.grantRole(KEEPER_ROLE, keeper2_address)
8. roleRegistry.grantRole(ORACLE_ROLE, ISFROracle_address)  // so oracle can call bountyPool
9. ISFROracle.setBountyPool(ISFRBountyPool_address)         // MANAGER_ROLE
10. MockERC20.approve(ISFRBountyPool, fundAmount)
11. ISFRBountyPool.fund(fundAmount)                          // seed reward pool
```

**Optional (for full demo with jobs):**

```
12. AgentRegistry()
13. BountyMarket(MockERC20, WorkerRegistry, RoleRegistry)
14. ConsortiumValidator(WorkerRegistry, BountyMarket)
15. roleRegistry.grantRole(OPERATOR_ROLE, BountyMarket)
16. roleRegistry.grantRole(OPERATOR_ROLE, ConsortiumValidator)
```

**Forge command:**

```bash
export DEPLOYER_PRIVATE_KEY="0xac0974bfc9882..."  # anvil default key 0
forge script script/Deploy.s.sol \
  --rpc-url http://127.0.0.1:8545 \
  --broadcast
```

### Register Keepers as Workers

Each keeper needs a WorkerRegistry bond to get non-zero weight in aggregation:

```bash
# Mint DAEJI tokens to keeper
cast send $MOCK_ERC20 "mint(address,uint256)" $KEEPER_ADDR 10000ether --rpc-url http://127.0.0.1:8545

# Approve WorkerRegistry to spend
cast send $MOCK_ERC20 "approve(address,uint256)" $WORKER_REGISTRY 1000ether --from $KEEPER_ADDR --rpc-url http://127.0.0.1:8545

# Register with minimum bond
cast send $WORKER_REGISTRY "register(uint256)" 1000ether --from $KEEPER_ADDR --rpc-url http://127.0.0.1:8545
```

New workers start at reputation 0.5 (Probation tier). For demo purposes, seed reputation via OPERATOR_ROLE:

```bash
# Boost keeper to Standard tier (need ~4 successful "jobs")
for i in {1..10}; do
  cast send $WORKER_REGISTRY "updateReputation(address,bool)" $KEEPER_ADDR true --from $OPERATOR --rpc-url http://127.0.0.1:8545
done
```

### ISFROracle Range Parameters

Default parameters work for ~1s block times. For mirage with `--block-interval-ms 1000`:

```
maxRangeWidth = 10        # 10 blocks = 10 seconds
stalenessLimit = 300      # 5 minutes before range is too old
closeDelay = 2            # wait 2 blocks after rangeEnd
closeQuorum = 3           # 3 voters auto-closes (lower for demo)
minVoters = 2             # minimum 2 voters (lower for demo)
```

Adjust for demo:
```bash
cast send $ISFR_ORACLE "setRangeParams(uint256,uint256,uint256,uint256,uint256,uint256)" \
  10 300 0 2 3 2 \
  --from $DEPLOYER --rpc-url http://127.0.0.1:8545
```

## 3. Relay Integration

### Architecture

```
┌──────────────┐     ┌──────────────┐     ┌──────────────┐
│  ISFR Keeper │     │  ISFR Keeper │     │  ISFR Keeper │
│  (Python)    │     │  (Rust/Roko) │     │  (any lang)  │
└──────┬───────┘     └──────┬───────┘     └──────┬───────┘
       │ WebSocket          │ WebSocket          │ WebSocket
       └───────────┬────────┴────────────┬───────┘
                   │                     │
            ┌──────┴─────────────────────┴──────┐
            │           daeji-relay              │
            │                                    │
            │  Topics:                           │
            │    feed:isfr:rates    rate obs.    │
            │    feed:isfr:ranges   range coord  │
            │    chain:nunchi       chain events │
            │                                    │
            │  Chain Watcher:                    │
            │    ISFROracle events → bus         │
            │    ISFRBountyPool events → bus     │
            │                                    │
            ├────────────────────────────────────┤
            │  Ring buffer per connection        │
            │  Resume from sequence number       │
            └──────────────┬─────────────────────┘
                           │ alloy WS
                    ┌──────┴──────┐
                    │   Mirage    │
                    │   :8545     │
                    └─────────────┘
```

### What the Relay Needs

The relay needs to be built or the roko relay needs to be extended. Minimum for ISFR:

**New in relay (bus.rs, ~150 lines):**
- Topic pub/sub with ring buffer per connection
- Subscribe/Publish/Envelope frame types
- Resume from sequence number on reconnect

**New in relay (chain.rs, ~100 lines):**
- Subscribe to ISFROracle events: `RateSubmitted`, `SubmissionAccepted`, `RangeClosed`
- Subscribe to ISFRBountyPool events: `RangeRewardRecorded`
- Publish each event as an envelope on `chain:nunchi` topic

**Extended in relay protocol:**
```json
// New frame types (add to existing Hello/Card/Response/Error/Ping):
{ "type": "subscribe", "topics": ["feed:isfr:rates", "chain:nunchi"] }
{ "type": "publish", "topic": "feed:isfr:rates", "payload": { ... } }
{ "type": "envelope", "seq": 123, "ts": 1713960000, "topic": "feed:isfr:rates",
  "from": "keeper-1", "payload": { ... } }
```

### Keeper → Relay → Chain Flow

**Rate observation (continuous):**
```
Keeper reads source protocols (Aave, Compound, Ethena, Hyperliquid, ETH staking)
  ↓
Keeper publishes to relay:
  topic: "feed:isfr:rates"
  payload: {
    composite_bps: 690,
    lending_bps: 620,
    structured_bps: 710,
    funding_bps: 45,
    staking_bps: 32,
    confidence_bps: 9000,
    timestamp: 1713960000
  }
  ↓
All subscribers receive the envelope (other keepers, dashboards, UI)
```

**Block-range coordination:**
```
Keeper-1 proposes:
  topic: "feed:isfr:ranges"
  payload: { type: "range_propose", start: 1000, end: 1009 }
  ↓
Keeper-2 votes:
  topic: "feed:isfr:ranges"
  payload: { type: "range_vote", start: 1000, end: 1009,
             composite_bps: 685, components: [620, 710, 45, 32] }
  ↓
Keeper-3 votes (quorum reached):
  topic: "feed:isfr:ranges"
  payload: { type: "range_vote", ... }
  ↓
Any keeper submits on-chain:
  ISFROracle.submitRateForRange(1000, 1009, rate, components, confidence)
  ↓
Chain watcher detects RangeClosed event → publishes on chain:nunchi:
  { type: "isfr.range_closed", range_start: 1000, range_end: 1009,
    composite_bps: 690, voter_count: 3 }
  ↓
UI receives chain event and updates display
```

### Roko Agent as ISFR Keeper

A roko agent acting as an ISFR keeper needs:

1. **Relay client upgrade** — add subscribe/publish to existing `relay_client.rs` (~50 lines)
2. **Source protocol readers** — functions that call Aave/Compound/etc. via mirage RPC
3. **Rate computation** — TVL-weighted median per class, composite from class weights
4. **Range coordination logic** — propose/vote/submit state machine
5. **Chain submission** — call ISFROracle via alloy/ethers

Integration point in roko:
```
roko/crates/roko-agent-server/src/features/relay_client.rs
  → Add: subscribe(topics: &[&str])
  → Add: publish(topic: &str, payload: Value)
  → Add: on_envelope(callback) for incoming envelopes
```

### Python Keeper Alternative

The existing Python keeper (`offchainservices-agent/cli/jobs/keepers/funding.py`) can be adapted with ~30 lines:

```python
import websockets, json

async def connect_to_relay():
    async with websockets.connect("ws://localhost:9011/ws") as ws:
        await ws.send(json.dumps({
            "type": "hello",
            "agent_id": "isfr-keeper-py"
        }))
        await ws.send(json.dumps({
            "type": "subscribe",
            "topics": ["feed:isfr:rates", "feed:isfr:ranges", "chain:nunchi"]
        }))
        return ws
```

## 4. Agent Discovery

### How Keepers Become Visible

Three discovery layers, all working together:

**Layer 1: Relay presence**
- Keeper connects to relay via WebSocket, sends Hello with agent_id, name, capabilities
- `GET /relay/agents` lists all connected agents
- Relay broadcasts `AgentConnected` event on `/relay/events/ws`

**Layer 2: Roko-serve registration**
- Keeper POSTs to `POST /api/agents/register`:
  ```json
  {
    "agent_id": "isfr-keeper-1",
    "label": "ISFR Keeper (Lending)",
    "capabilities": ["isfr", "oracle"],
    "domain_tags": ["isfr", "defi"],
    "endpoints": {
      "rest": "http://localhost:8081",
      "websocket": "ws://localhost:8081/stream"
    },
    "tier": "Standard",
    "reputation": 65
  }
  ```
- `GET /api/managed-agents` returns full dashboard payload with heartbeat, performance, costs

**Layer 3: Agent card**
- Keeper builds `AgentCard` with name, capabilities, endpoints, domain_tags
- Published to relay via Card frame → available at `GET /relay/cards/{id}`
- Or via data URI (base64-encoded card JSON)

**Layer 4 (optional): On-chain identity**
- Register on AgentRegistry: `register(capabilities, passportHash)`
- Heartbeat: `heartbeat()` every ~15 minutes
- Links on-chain identity to relay presence

### What the Frontend Queries

```
GET /api/managed-agents
  → Full agent roster with status dots, reputation, capabilities, heartbeat age

GET /relay/agents
  → Relay-connected agents (lighter, real-time)

GET /relay/cards/{agent_id}
  → Full agent card JSON

GET /api/agents/{id}
  → Single agent detail view
```

## 5. Demo-App UI Integration

### What Exists

The demo-ide (`roko/demo-ide/`) is a **Tauri + React 19 + TypeScript** app with:

- **Tile-based dashboard** (React Grid Layout) — draggable, resizable tiles
- **Widget system** — declarative data visualization (metric, chart, list, sparkline, gauge, etc.)
- **IsfrSymphonyTile** — existing ISFR demo execution UI showing agent proposals, composite rate, settlement state
- **AgentChat** — right sidebar with slash commands (`/isfr-demo` already exists)
- **PulseView** — health monitoring for roko/mirage/relay
- **AgentsView** — fleet management with status, reputation, capabilities
- **TradingTiles** — market chart, order book, trades tape (good reference)
- **Workspace templates** — pre-configured tile layouts

**Design system:** ROSEDUST dark theme with CSS variables (`--rd-rose`, `--rd-success`, `--rd-warning`, `--rd-error`).

### ISFR Feed UI: What to Build

#### New Tile: IsfrFeedTile

A real-time ISFR feed monitor tile showing live rate observations from keepers:

```
┌─────────────────────────────────────────────────────────────┐
│  ISFR Live Feed                              [⟳ Connected]  │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  Composite: 6.90%    Confidence: 90%    Epoch: 1,247       │
│  ─────────────────────────────────────────────────────────  │
│                                                             │
│  ┌─────────┬─────────┬─────────┬──────────┐                │
│  │ LENDING │ STRUCT  │ FUNDING │ STAKING  │                │
│  │  6.20%  │  7.10%  │  0.45%  │  0.32%   │                │
│  │  w=0.60 │  w=0.25 │  w=0.10 │  w=0.05  │                │
│  └─────────┴─────────┴─────────┴──────────┘                │
│                                                             │
│  ┌─ Rate History (last 30 epochs) ─────────────────────┐   │
│  │  7.5% ┤                                              │   │
│  │  7.0% ┤    ╱╲    ╱╲                                  │   │
│  │  6.5% ┤╱╲╱  ╲╱╲╱  ╲╱╲╱╲                             │   │
│  │  6.0% ┤                  ╲╱╲╱╲╱╲                      │   │
│  │  5.5% ┤                                              │   │
│  └──────────────────────────────────────────────────────┘   │
│                                                             │
│  ┌─ Active Keepers ─────────────────────────────────────┐   │
│  │  keeper-1  │ Standard │ rep: 0.65 │ 12 proposals     │   │
│  │  keeper-2  │ Trusted  │ rep: 0.78 │ 11 proposals     │   │
│  │  keeper-3  │ Elite    │ rep: 0.92 │ 14 proposals     │   │
│  └──────────────────────────────────────────────────────┘   │
│                                                             │
│  ┌─ Range Activity ─────────────────────────────────────┐   │
│  │  Range 1000-1009  CLOSED  3 voters  composite: 690   │   │
│  │  Range 1010-1019  VOTING  2/3 votes                  │   │
│  │  Range 1020-1029  PROPOSED by keeper-1               │   │
│  └──────────────────────────────────────────────────────┘   │
│                                                             │
│  ┌─ Chain Events ───────────────────────────────────────┐   │
│  │  12:34:01  RangeClosed  range=1000-1009  rate=690    │   │
│  │  12:34:05  RewardRecorded  range=1000-1009  3 voters │   │
│  │  12:34:12  SubmissionAccepted  keeper-2  range=1010  │   │
│  └──────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

#### Data Sources for the UI

The UI needs these endpoints. Some exist, some need to be added:

**Already available (roko-serve):**
```
GET /api/managed-agents         → agent roster (filter by domain_tags: ["isfr"])
GET /relay/agents               → connected agents
GET /relay/cards/{id}           → agent cards
```

**Need to add (roko-serve or relay HTTP API):**
```
GET /api/isfr/current           → { composite_bps, lending_bps, structured_bps,
                                    funding_bps, staking_bps, confidence_bps,
                                    epoch_id, timestamp }

GET /api/isfr/history?limit=30  → [{ epoch_id, composite_bps, lending_bps, ..., timestamp }]

GET /api/isfr/keepers           → [{ agent_id, reputation, tier, bond, proposals_count,
                                     last_submission_ts, trust_weight }]

GET /api/isfr/ranges            → [{ range_id, start, end, status, voter_count,
                                     composite_bps, closed_at }]

GET /api/isfr/contracts         → { ISFROracle: "0x...", ISFRBountyPool: "0x...",
                                    WorkerRegistry: "0x...", RoleRegistry: "0x..." }
```

**Implementation options for these endpoints:**

1. **Direct chain reads** — roko-serve calls mirage RPC to read ISFROracle state:
   ```typescript
   // Read current epoch from ISFROracle
   const current = await publicClient.readContract({
     address: ISFR_ORACLE,
     abi: isfrOracleAbi,
     functionName: 'currentRate'
   });
   ```

2. **Relay feed subscription** — roko-serve subscribes to relay feed topics and caches latest data. Lower latency for real-time display.

3. **Hybrid** — chain reads for authoritative state, relay feeds for real-time updates.

#### WebSocket Feed for Real-Time Updates

The UI can connect directly to the relay's event stream for live updates:

```typescript
// In IsfrFeedTile.tsx
const ws = new WebSocket("ws://localhost:9011/ws");

ws.send(JSON.stringify({
  type: "hello",
  agent_id: "demo-ui"
}));

ws.send(JSON.stringify({
  type: "subscribe",
  topics: ["feed:isfr:rates", "feed:isfr:ranges", "chain:nunchi"]
}));

ws.onmessage = (event) => {
  const envelope = JSON.parse(event.data);

  switch (envelope.topic) {
    case "feed:isfr:rates":
      updateCompositeDisplay(envelope.payload);
      appendToHistory(envelope.payload);
      break;
    case "feed:isfr:ranges":
      updateRangeActivity(envelope.payload);
      break;
    case "chain:nunchi":
      appendChainEvent(envelope.payload);
      if (envelope.payload.type === "isfr.range_closed") {
        updateLatestClosedRange(envelope.payload);
      }
      break;
  }
};
```

#### Widget-Based Alternative (Zero Custom Code)

Using the existing declarative widget system, agents can create ISFR tiles without new components:

```typescript
// Agent requests tile creation via chat or API:
requestTileCreate({
  type: "widget",
  spec: {
    kind: "metric-grid",
    label: "ISFR Live",
    source: "http://localhost:5678/api/isfr/current",
    refreshMs: 5000,
    cells: [
      { label: "Composite", path: "composite_bps", format: "number", suffix: " bps" },
      { label: "Lending", path: "lending_bps", format: "number", suffix: " bps" },
      { label: "Structured", path: "structured_bps", format: "number", suffix: " bps" },
      { label: "Funding", path: "funding_bps", format: "number", suffix: " bps" },
      { label: "Staking", path: "staking_bps", format: "number", suffix: " bps" },
      { label: "Confidence", path: "confidence_bps", format: "number", suffix: " bps" }
    ]
  }
});
```

And a chart tile:
```typescript
requestTileCreate({
  type: "widget",
  spec: {
    kind: "chart",
    label: "ISFR History",
    source: "http://localhost:5678/api/isfr/history?limit=50",
    refreshMs: 10000,
    datasets: [
      { label: "Composite", path: "history[*].composite_bps", color: "#c08394" },
      { label: "Lending", path: "history[*].lending_bps", color: "#7a9b86" },
      { label: "Structured", path: "history[*].structured_bps", color: "#c39b5f" }
    ]
  }
});
```

#### Workspace Template

Add an ISFR workspace template to `workspace-templates.ts`:

```typescript
{
  id: "isfr-feed",
  name: "ISFR Feed Monitor",
  description: "Live ISFR rates, keeper activity, range coordination, chain events",
  tiles: [
    { slot: "composite", type: "isfr-feed", col: 0, row: 0, w: 3, h: 4 },
    { slot: "agents", type: "widget", col: 3, row: 0, w: 1, h: 2 },
    { slot: "terminal", type: "terminal", col: 3, row: 2, w: 1, h: 2 }
  ]
}
```

#### Slash Command

Add `/isfr-feed` to AgentChat that:
1. Ensures mirage is running
2. Deploys ISFR contracts if not already deployed
3. Starts keeper agents
4. Opens the ISFR Feed Monitor workspace
5. Connects UI to relay feed topics

## 6. End-to-End Implementation Checklist

### Phase 1: Contracts on Mirage

```
[ ] Start mirage (via demo-ide or direct binary)
[ ] Add ISFROracle + ISFRBountyPool to Deploy.s.sol (or create ISFRDeploy.s.sol)
[ ] Deploy: forge script --broadcast --rpc-url http://127.0.0.1:8545
[ ] Wire roles: KEEPER_ROLE to keeper addresses, ORACLE_ROLE to ISFROracle
[ ] Set bounty pool: ISFROracle.setBountyPool(ISFRBountyPool)
[ ] Fund bounty pool with DAEJI tokens
[ ] Register keepers in WorkerRegistry with MIN_BOND
[ ] Seed keeper reputation to Standard+ tier (for non-zero weight)
[ ] Verify: cast call $ISFR_ORACLE "currentRate()" --rpc-url http://127.0.0.1:8545
```

### Phase 2: Relay with Topic Pub/Sub

```
[ ] Add bus.rs to roko relay (or create daeji-relay): topic pub/sub + ring buffer (~150 lines)
[ ] Add Subscribe/Publish/Envelope frame types to protocol.rs (~40 lines)
[ ] Wire frames into WebSocket handler (~60 lines)
[ ] Add chain.rs: subscribe to ISFROracle/ISFRBountyPool events via alloy (~100 lines)
[ ] Test: connect WebSocket client, subscribe to feed:isfr:rates, verify envelopes
```

### Phase 3: Keeper Agents

```
[ ] Add subscribe/publish to roko relay_client.rs (~50 lines)
[ ] Create ISFR keeper agent (Rust or Python):
    - Connect to relay
    - Subscribe to feed:isfr:rates, feed:isfr:ranges, chain:nunchi
    - Read source protocols (mock data for demo: Aave, Compound via mirage)
    - Publish rate observations
    - Coordinate on ranges
    - Submit on-chain when quorum reached
[ ] Register keeper agents with roko-serve for discovery
[ ] Verify: multiple keepers publishing rates visible on relay topics
```

### Phase 4: API Endpoints

```
[ ] Add /api/isfr/current to roko-serve (reads ISFROracle via RPC)
[ ] Add /api/isfr/history to roko-serve (reads ring buffer)
[ ] Add /api/isfr/keepers to roko-serve (reads WorkerRegistry + relay presence)
[ ] Add /api/isfr/ranges to roko-serve (reads oracle range state + relay feed)
[ ] Add /api/isfr/contracts to roko-serve (returns deployed addresses)
[ ] Verify: curl http://localhost:5678/api/isfr/current returns live data
```

### Phase 5: Frontend

```
[ ] Create IsfrFeedTile.tsx component
    - WebSocket connection to relay for real-time feed data
    - Composite rate display (large number + 4 class breakdown)
    - Rate history sparkline/chart
    - Active keepers table (from /api/isfr/keepers)
    - Range activity list (from relay feed:isfr:ranges)
    - Chain event log (from relay chain:nunchi)
[ ] Register in tile library (tiles.ts)
[ ] Add "isfr-feed" workspace template
[ ] Add /isfr-feed slash command in AgentChat
[ ] Test: full flow from keeper submission to UI update
```

## 7. File Locations Summary

### Contracts (source of truth)
```
/Users/will/dev/nunchi/roko/demo-ide/demo/contracts/src/ISFROracle.sol
/Users/will/dev/nunchi/roko/demo-ide/demo/contracts/src/ISFRBountyPool.sol
/Users/will/dev/nunchi/roko/demo-ide/demo/contracts/src/IISFROracle.sol
/Users/will/dev/nunchi/roko/demo-ide/demo/contracts/src/IISFRBountyPool.sol
/Users/will/dev/nunchi/roko/demo-ide/demo/contracts/src/WorkerRegistry.sol
/Users/will/dev/nunchi/roko/demo-ide/demo/contracts/src/RoleRegistry.sol
/Users/will/dev/nunchi/roko/demo-ide/demo/contracts/src/MockERC20.sol
/Users/will/dev/nunchi/roko/demo-ide/demo/contracts/shared/interfaces/IRoleRegistry.sol
/Users/will/dev/nunchi/roko/demo-ide/demo/contracts/script/Deploy.s.sol
/Users/will/dev/nunchi/roko/demo-ide/demo/contracts/foundry.toml
```

### Relay (to build/extend)
```
/Users/will/dev/nunchi/roko/roko/apps/agent-relay/src/lib.rs        — existing relay
/Users/will/dev/nunchi/roko/roko/apps/agent-relay/src/protocol.rs   — add frame types
/Users/will/dev/nunchi/roko/roko/apps/agent-relay/src/state.rs      — add topic state
                                                    src/bus.rs      — NEW: pub/sub
                                                    src/chain.rs    — NEW: chain watcher
```

### Roko Agent Integration
```
/Users/will/dev/nunchi/roko/roko/crates/roko-agent-server/src/features/relay_client.rs  — add subscribe/publish
/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/                          — add /api/isfr/* routes
```

### Frontend
```
/Users/will/dev/nunchi/roko/demo-ide/src/components/IsfrFeedTile.tsx     — NEW: feed tile
/Users/will/dev/nunchi/roko/demo-ide/src/components/IsfrSymphonyTile.tsx — existing demo tile (reference)
/Users/will/dev/nunchi/roko/demo-ide/src/lib/workspace-templates.ts     — add isfr-feed template
/Users/will/dev/nunchi/roko/demo-ide/src/lib/tiles.ts                   — register tile
/Users/will/dev/nunchi/roko/demo-ide/src/components/AgentChat.tsx        — add /isfr-feed command
```

### Mirage
```
/Users/will/dev/nunchi/roko/roko/apps/mirage-rs/src/main.rs   — server startup
Binary: mirage-rs (in target/release/)
Status: /tmp/mirage-8545-status.json
State: .roko/state/mirage-snapshot.json
```

## 8. What Makes It Engaging

The difference between a demo and an engaging demo:

**Real-time data flow visible end-to-end:**
- Keeper reads price → publishes to relay → UI updates within 100ms
- Range proposed → votes arrive → quorum → on-chain → chain event → UI
- The user sees the full pipeline live, not just final numbers

**Multiple keepers with different behaviors:**
- Keeper-1: conservative (reads Aave + Compound only, high confidence)
- Keeper-2: aggressive (reads all sources, lower confidence)
- Keeper-3: lagging (submits late, gets lower weight due to reputation decay)
- Their proposals diverge slightly, weighted median resolves

**Agent cards with personality:**
- Each keeper has an agent card with name, capabilities, domain_tags
- Cards visible in the UI via relay cards endpoint
- Click a keeper → see their card, reputation, submission history

**Chain events as live feed:**
- RangeClosed events show the trust-weighted median in action
- RewardRecorded events show bounty distribution
- SubmissionAccepted shows each vote arriving

**Interactive controls:**
- Start/stop individual keepers
- Adjust range parameters via UI
- Fund/drain the bounty pool
- Force-decay a keeper's reputation to show weight changes

**Dashboard composition:**
- ISFR feed tile + trading tile (yield perp mock) + agent roster + terminal
- User can drag, resize, rearrange
- Multiple workspaces for different views
