# ISFR Analysis: Current State and First Use Case

## What ISFR Is

Single number: the cost of secured funding across DeFi, measured in basis points. To DeFi what SOFR is to TradFi (~$570T notional). V3.0 two-level four-class methodology.

## Current Implementations

### 1. Full Oracle (demo-ide)

**Location:** `/Users/will/dev/nunchi/roko/demo-ide/demo/contracts/src/ISFROracle.sol` (551 lines)

Production-quality v3.0 implementation:

**Two submission paths:**
- **Fast path** (`submitRate`) — single permissioned keeper, immediate write
- **Block-range path** (`submitRateForRange`) — multi-voter consensus, buffer until quorum/closeDelay

**Trust-weighted median:**
```
trustLevel(agent) = clamp(sqrt(bond/MIN_BOND) × reputationBp, FLOOR=0.1, CEILING=10.0)
```
- Probation-tier voters get weight 0 (excluded)
- Square-root bond scaling prevents whale dominance
- Five independent medians: composite + 4 class rates

**Ring buffer:** 256-epoch circular storage, O(1) writes

**Range validation parameters (admin-settable):**
- maxRangeWidth = 10 blocks
- stalenessLimit = 300 blocks
- closeDelay = 2 blocks after rangeEnd
- closeQuorum = 5 keepers (auto-close)
- minVoters = 3 keepers

### 2. Bounty Pool (demo-ide)

**Location:** `/Users/will/dev/nunchi/roko/demo-ide/demo/contracts/src/ISFRBountyPool.sol`

Per-range reward distribution:
- `rewardPerRange = bountyRate × rangeWidth`
- Pro-rata distribution by trust weight
- Async claims (operators don't lose rewards for going offline)
- Open funding model (anyone can fund pool)
- Phase-1-only interim — spec target is validator-computed oracle with no separate bounty

### 3. Python Keeper (offchainservices-agent)

**Location:** `/Users/will/dev/nunchi/offchainservices-agent/cli/jobs/keepers/funding.py` (468 lines)

Funding rate keeper strategy:
- Samples mark vs. index premium every 30 seconds
- Computes TWAP over accumulation window
- Submits `settleFundingWindow` hourly
- Rate clamping: ±4%
- File-based state persistence for crash recovery
- Inflight recovery: detects stale in-flight transactions on restart

### 4. Gossip Aggregation (gossip-protocol)

**Location:** `/Users/will/dev/nunchi/gossip-protocol/specpool-evm/` (666+ lines)

Multi-node funding rate demo:
- BinanceFundingOracle: reads 8-hour rates, computes annualized
- Settlement epoch model with insurance fund
- Message types: FundingRateUpdate, Settlement
- Multi-node consensus via gossip mesh

### 5. Contracts-Core Stubs

**Location:** `/Users/will/dev/nunchi/contracts-core/packages/agents/src/`

- `IISFROracle.sol` — Interface matching ISFR paper §3.5 layout
- `ISFRMinimal.sol` — Hardcoded reference values (epoch=1, composite_bps=690)

## Spec Target: Validator-Computed Oracle

The agent-chainv2 spec calls for a fundamentally different architecture:

### Current → Spec Gap

| Aspect | Current (keeper-submitted) | Spec (validator-computed) |
|--------|--------------------------|--------------------------|
| Who computes | Separate keeper agents | Every validator independently |
| Trust model | Keeper reputation + bond | Validator stake (consensus) |
| Submission | Contract call (gas cost) | OracleVote in consensus |
| Aggregation | Contract-side weighted median | Consensus-side stake-weighted median |
| Publication | Contract storage | Precompile 0xA01 (constant gas reads) |
| Cadence | When keepers submit | Every 25 blocks (~10 seconds) |
| Byzantine defense | Trust-weighted median only | Two-level: intra-class + inter-validator |
| Circuit breaker | None | 4-state (Live/Degraded/Stale/Halted) |
| Confidence score | No | Yes (stake-weighted within-σ %) |

### Level 1 — Intra-class (per validator)

Four source classes with fixed weights:
- LENDING (0.60) — Aave V3, Compound V3
- STRUCTURED (0.25) — Ethena sUSDe
- FUNDING (0.10) — Hyperliquid ETH perp
- STAKING (0.05) — ETH Beacon Chain

TVL-weighted median within each class. Tolerates 49% corrupted weight per class.

### Level 2 — Inter-validator (consensus)

Stake-weighted median across all validator OracleVotes. Two-pass outlier exclusion (3σ). Tolerates 49% compromised validator stake.

**Combined:** attacker must simultaneously corrupt 50%+ source weight AND 50%+ validator stake.

## ISFR as First Use Case for Relay

The user noted: "The daeji-chat is probably okay-ish for ISFR oracle updates. That's one of the only things that should be done as a first use case."

### What ISFR Needs from a Communication Layer

1. **Rate broadcast** — keepers/validators publish computed rates
2. **Aggregation coordination** — multi-keeper agreement on range windows
3. **Liveness signals** — who is online and ready to submit
4. **Result notification** — when a range closes, notify participants

### Current Chat Layer (PR #24) for ISFR

**What works:**
- ChaCha20Poly1305 AEAD encryption — protects rate submissions from frontrunning
- Chain watcher — can detect JobFunded/JobCompleted for bounty lifecycle
- Room key derivation — deterministic per-job rooms

**What doesn't work for ISFR specifically:**
- Symphony coordination pattern assumes human-like job lifecycle (post→assign→work→submit→settle)
- ISFR is **continuous** — no discrete job start/end, just perpetual rate publication
- 64-slot pool designed for concurrent discrete jobs, not persistent data streams
- Typed message enum (Hello/Status/PartialResult/Vote/Final) doesn't fit oracle semantics
- Full mesh O(n²) is wasteful for oracle updates that are broadcast-only

### Relay Design for ISFR

ISFR maps naturally to the **feed** pattern in the relay redesign:

```
Topic: feed:isfr:rates
  - Each keeper publishes rate observations
  - Relay delivers to all subscribers

Topic: feed:isfr:ranges
  - Range-open proposals
  - Vote submissions
  - Range-close notifications

Topic: chain:nunchi
  - ISFROracle events (RateSubmitted, RangeClosed)
  - ISFRBountyPool events (RangeRewardRecorded)
```

**Why this is better:**
- No job/room lifecycle overhead — feeds are always-on
- Any language can subscribe (Python keepers, Rust validators, dashboards)
- Ring buffer handles reconnection (keeper restarts, catches up)
- Chain events arrive automatically (no separate watcher per keeper)

### However: Does ISFR Even Need Daeji-Chat?

The user also noted: "I don't think things are really needed for that."

If the spec target is **validator-computed oracle**, then:
- Validators already communicate through consensus (commonware-p2p)
- OracleVote is part of the block proposal, not a separate message channel
- No separate keeper infrastructure needed
- No relay/chat needed for the oracle itself

**The relay becomes useful for ISFR when:**
- Broadcasting finalized rates to external consumers (dashboards, other chains)
- Coordinating keeper agents during the Phase-1 interim (before validator-computed)
- Publishing chain events (circuit breaker state changes) to subscribers
- Feeding rates into yield perpetual agents that need real-time ISFR

### Recommendation

**Phase-1 (interim):** Use relay feed topics for keeper coordination. This is the simplest first use case — just `feed:isfr:*` topics with JSON envelopes. No complex group coordination needed.

**Phase-2 (validator-computed):** Oracle moves into consensus. Relay still useful for broadcasting finalized rates and chain events to external subscribers.

**Neither phase requires the PR #24 architecture** (commonware-p2p mesh, symphony coordination, slot pools). A simple relay with feed topics handles ISFR better.
