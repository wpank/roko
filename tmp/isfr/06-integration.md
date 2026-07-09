# 06: End-to-End Integration

How all the pieces connect — startup sequence, configuration, demo flow, and the path from "no ISFR" to "rates flowing through the system." Chain-agnostic throughout; examples use mirage-rs as the default dev profile.

## Startup Sequence

```
1. Chain starts (profile-dependent)
   └── mirage-rs:  auto-deploys ISFR contracts, writes chain-profile.json
   └── daeji:      contracts already deployed, addresses from roko.toml
   └── anvil fork: auto-deploys to fork, has real DeFi protocols accessible
   └── Any EVM:    contracts at configured addresses

2. agent-relay starts
   └── Loads ChainProfile (from chain-profile.json or roko.toml)
   └── Starts chain watcher → subscribes to events at profile.rpc_url
   └── Publishes to chain:{profile.chain_id} topic
   └── Bus ready for topic pub/sub

3. roko serve starts
   └── Loads same ChainProfile
   └── Connects to relay as an agent
   └── Starts ISFRFeed (subscribes to feed:isfr:*, chain:{chain_id})
   └── Registers ISFR tools with contract addresses from profile
   └── Starts SSE/dashboard serving

4. ISFR keeper agent starts (separate process or integrated)
   └── Registers with relay (Hello + Card)
   └── Subscribes to feed:isfr:rates, feed:isfr:ranges, chain:{chain_id}
   └── Starts polling sources every 10s (mock or real, per config)
   └── Publishes rate observations to feed:isfr:rates

5. (Optional) Additional keeper agents start
   └── Same subscription pattern — all keepers on all chains use the same topics
   └── Range coordination happens through relay topics
```

## Data Flow

```
Sources → ISFRKeeper → Relay (feed:isfr:rates) → ISFRFeed → PulseBus → Dashboard/SSE
                                                → Other keepers (coordination)
                                                → Chain watcher (chain:{id} → bus)
                     → ISFROracle (on-chain)    → Chain events (RateSubmitted, etc.)
                                                → Relay (chain:{id}) → all subscribers
```

### Rate Observation Flow

1. Keeper polls configured sources (mock for dev chains, real for mainnet forks)
2. Keeper computes weighted median composite rate
3. Keeper publishes to relay topic `feed:isfr:rates`:
   ```json
   {
     "type": "publish",
     "topic": "feed:isfr:rates",
     "payload": {
       "type": "rate_observation",
       "composite_bps": 690,
       "lending_bps": 620,
       "structured_bps": 710,
       "funding_bps": 45,
       "staking_bps": 320,
       "confidence_bps": 8500,
       "timestamp": 1713960000,
       "keeper": "isfr-keeper-1"
     }
   }
   ```
4. Relay routes envelope to all `feed:isfr:rates` subscribers
5. ISFRFeed (in roko serve) receives envelope, converts to Pulse, publishes on local Bus
6. GraduationCell graduates rate Pulses to Signals (persisted in Store)
7. Dashboard/SSE receives Pulse as `EventLogEntry` event
8. Keeper periodically submits aggregate rate to ISFROracle contract (at `profile.contracts.isfr_oracle`)
9. Chain watcher sees `RateSubmitted` event, publishes to `chain:{chain_id}`
10. Subscribers (keepers, dashboard) receive chain confirmation

### Block-Range Flow

1. Keeper-1 proposes a range on `feed:isfr:ranges`:
   ```json
   { "type": "range_propose", "start": 1000, "end": 1009, "proposed_by": "keeper-1" }
   ```
2. Keeper-2 sees proposal, votes with their rate:
   ```json
   { "type": "range_vote", "start": 1000, "end": 1009, "composite_bps": 685, ... }
   ```
3. Any keeper detects quorum, submits `ISFROracle.submitRateForRange()` on-chain
4. Chain watcher sees `RangeClosed` event, publishes to `chain:{chain_id}`
5. All keepers receive confirmation, credit rewards from BountyPool

## Configuration

### Single roko.toml

```toml
# ── Chain ────────────────────────────────────────────────────
# Swap this section to point to a different chain.
# Everything else (relay topics, tools, feeds) adapts automatically.

[chain]
profile = "mirage"                    # "mirage" | "daeji" | "custom"
# chain_id = "mirage"                # auto-derived from profile name
# rpc_url = "ws://localhost:8545"     # auto-derived from profile

# For pre-deployed chains, specify addresses:
# [chain.contracts]
# isfr_oracle = "0x..."
# bounty_pool = "0x..."
# worker_registry = "0x..."
# role_registry = "0x..."

# ── Relay ────────────────────────────────────────────────────
[relay]
url = "ws://localhost:9011/relay/agents/ws"

# ── ISFR ─────────────────────────────────────────────────────
[isfr]
enabled = true
epoch_duration_secs = 28800
poll_interval_secs = 10
min_submissions = 2
outlier_sigma = 3.0

# Source configuration — mock for dev, real for mainnet forks
[[isfr.sources]]
name = "mock-aave-v3"
kind = "mock"
weight = 0.30
class = "lending"
rate_bps = 620
jitter_bps = 15

[[isfr.sources]]
name = "mock-compound-v3"
kind = "mock"
weight = 0.25
class = "lending"
rate_bps = 580
jitter_bps = 20

[[isfr.sources]]
name = "mock-ethena-susde"
kind = "mock"
weight = 0.25
class = "structured"
rate_bps = 710
jitter_bps = 30

[[isfr.sources]]
name = "mock-beacon-staking"
kind = "mock"
weight = 0.20
class = "staking"
rate_bps = 320
jitter_bps = 5

# ── Feeds ────────────────────────────────────────────────────
[feeds]
# ISFRFeed relay topics — chain topic auto-derived from [chain].chain_id
isfr_relay_topics = ["feed:isfr:rates", "feed:isfr:ranges"]

# ── Graduation ───────────────────────────────────────────────
[[graduation.policies]]
watch = { Prefix = "isfr.rates" }
always = true

[[graduation.policies]]
watch = { Prefix = "isfr.ranges" }
sample_every = 10
```

### Switching Chains

To switch from mirage to daeji, only the `[chain]` section changes:

```toml
[chain]
profile = "daeji"
rpc_url = "ws://kora.nunchi.dev:8545"

[chain.contracts]
isfr_oracle = "0x3456..."
bounty_pool = "0x4567..."
worker_registry = "0x2345..."
role_registry = "0x1234..."
```

Everything else — relay topics, feed subscriptions, keeper logic, tools — adapts automatically because they reference `chain_id` from the profile, not hardcoded values.

## Integration Points

### 1. ChainProfile → Everything

The profile flows through the system:

```rust
let profile = ChainProfile::from_config(&config.chain);

// Relay chain watcher
let watcher = ChainWatcher::new(bus.clone(), profile.rpc_url.clone(), &profile.chain_id);

// ISFRFeed
let isfr_feed = ISFRFeed::new(&config.relay.url, &profile.chain_id);

// ISFRKeeper
let keeper = ISFRKeeper::new(sources, relay, "keeper-1", isfr_config, &profile.chain_id);

// ISFRToolHandler
let tool_handler = ISFRToolHandler::new(
    provider,
    profile.contracts.isfr_oracle.expect("ISFROracle address required"),
    profile.contracts.bounty_pool.expect("BountyPool address required"),
    profile.contracts.worker_registry.expect("WorkerRegistry address required"),
);
```

### 2. relay_client.rs ↔ Bus

```
Relay WebSocket ──[subscribe]──→ relay_client ──[envelope]──→ ISFRFeed ──[Pulse]──→ PulseBus
                                                                                      │
                                                                              GraduationCell
                                                                                      │
                                                                                   Store
```

### 3. Tool Registry ↔ ISFRToolHandler

ISFR tools register alongside existing chain tools:

```rust
// Existing chain tools
for tool in CHAIN_DOMAIN_TOOLS.iter() {
    registry.register(tool.clone(), chain_handler.clone());
}

// ISFR tools — same pattern, uses profile.contracts for addresses
for tool in ISFR_DOMAIN_TOOLS.iter() {
    registry.register(tool.clone(), isfr_handler.clone());
}
```

### 4. Chain Watcher ↔ Contract Events

The chain watcher decodes events using the same ABIs regardless of chain:

```rust
fn decode_isfr_event(log: &Log) -> Option<(&str, Value)> {
    match log.topic0()? {
        RATE_SUBMITTED_SIG => Some(("isfr.rate_submitted", decode_rate_submitted(log))),
        RANGE_CLOSED_SIG => Some(("isfr.range_closed", decode_range_closed(log))),
        EPOCH_ADVANCED_SIG => Some(("isfr.epoch_advanced", decode_epoch_advanced(log))),
        BOUNTY_DEPOSITED_SIG => Some(("isfr.bounty_deposited", decode_bounty_deposited(log))),
        REWARD_CLAIMED_SIG => Some(("isfr.reward_claimed", decode_reward_claimed(log))),
        WORKER_REGISTERED_SIG => Some(("isfr.worker_registered", decode_worker_registered(log))),
        REPUTATION_UPDATED_SIG => Some(("isfr.reputation_updated", decode_reputation_updated(log))),
        _ => None,
    }
}
```

Event signature hashes are computed from ABIs and are the same across all chains — the contracts have the same interface everywhere.

### 5. Dashboard / SSE

ISFRFeed Pulses flow to the dashboard through the existing SSE layer:

```
ISFRFeed → PulseBus → StateHub → SSE /api/events → Dashboard
```

### 6. CLI

```bash
# List all feeds including ISFR
roko feed list
# ID                       TOPIC                            KIND       CONNECTED
# isfr-feed                isfr.rates                       Derived    yes

# Read current rates (calls ISFROracle on whichever chain is configured)
roko tool call isfr.read_rates
# { "composite_bps": 690, "lending_bps": 620, ... }

# Check which chain profile is active
roko chain status
# Profile: mirage
# Chain ID: mirage
# RPC: ws://localhost:8545
# ISFROracle: 0x3456...
# BountyPool: 0x4567...
```

## Demo Flow (Mirage Profile)

```bash
# Terminal 1: Start mirage (auto-deploys contracts)
cd apps/mirage-rs && cargo run
# → Chain running on :8545
# → ISFR contracts deployed
# → Chain profile written to data/chain-profile.json

# Terminal 2: Start relay
cd apps/agent-relay && cargo run
# → Loads chain profile
# → Chain watcher subscribing to events
# → Relay running on :9011

# Terminal 3: Start roko serve
cargo run -p roko-cli -- serve
# → Connected to relay
# → ISFRFeed started
# → ISFR tools registered
# → Dashboard at http://localhost:6677

# Terminal 4: Start ISFR keeper
cargo run -p roko-cli -- isfr start
# → Polling mock sources every 10s
# → Publishing rate observations...

# Terminal 5: Interact
roko feed list       # → isfr-feed connected
roko tool call isfr.read_rates  # → latest composite rate
```

To switch to daeji: change `[chain]` in roko.toml, restart. Same terminals, same commands.

## What's Elegantly Abstracted

1. **ChainProfile** — single config object flows everywhere. Swap the profile, everything adapts. No scattered URL/address constants.

2. **ISFRSource trait** — adding a new rate source is one struct. No changes to keeper, relay, or tools.

3. **RelayFeedCodec** — adding a new relay-backed feed is one codec. ISFRFeed, PriceFeed, GasFeed all share the same `RelayFeed<C>` base.

4. **ToolDef pattern** — adding new ISFR tools follows the exact same `LazyLock` + `ToolDef` pattern as chain tools.

5. **Topic hierarchy** — `feed:isfr:rates`, `feed:isfr:ranges`, `chain:{chain_id}` — topics are strings parameterized by chain_id. No enum variants, no protocol coupling.

6. **Chain watcher** — same event decoder ABIs work on every chain. Adding new contract events is adding signature matches.

7. **Graduation policies** — configuring which ISFR events persist is TOML, not code.

## Dependency Graph

```
                  ChainProfile
                  (roko.toml)
                       │
          ┌────────────┼────────────┐
          │            │            │
     Chain (any)   agent-relay   roko-chain
     (EVM RPC)        │         (ABI bindings)
          │       bus + chain.rs     │
          │            │       ┌─────┴──────┐
          │            │       │            │
          │      relay_client  ISFRSource  ISFRTools
          │            │       trait        (ToolDef)
          │            │         │            │
          │       ISFRFeed    ISFRKeeper  ISFRToolHandler
          │      (Feed trait) (orchestrator)  │
          │            │         │            │
          │       PulseBus   relay pub/sub  ChainClient
          │            │         │            │
          │       Graduation  On-chain     alloy calls
          │            │      submission       │
          │         Store         │       provider at
          │                ISFROracle     profile.rpc_url
          └────────────────────┘
```

## Total Line Count Estimate

| Component | From Doc | Lines |
|-----------|----------|-------|
| Relay upgrade (bus, protocol, wiring, chain watcher) | 01 | ~490 |
| ISFRFeed + RelayFeed base | 02 | ~200 |
| ISFRSource trait + mock sources + ISFRKeeper | 03 | ~400 |
| ISFR tool definitions + handlers | 04 | ~350 |
| ChainProfile + bootstrap + ABI bindings | 05 | ~300 |
| Config + wiring + startup | 06 | ~100 |
| **Total** | | **~1,840** |

For comparison:
- PR #24 (daeji-chat): ~2,000 lines, Rust-only, can't serve ISFR, single-chain embedded
- This plan: ~1,840 lines, language-agnostic, fully functional ISFR pipeline, runs on any EVM chain
