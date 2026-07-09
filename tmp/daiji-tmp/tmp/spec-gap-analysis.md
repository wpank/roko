# Agent-ChainV2 Spec Gap Analysis

How far current implementations are from the agent-chainv2 target spec.

## Spec Overview

The agent-chainv2 spec at `/Users/will/dev/nunchi/collaboration/workspace/wp/agent-chainv2/` defines the complete Nunchi blockchain architecture across four sections:

- **01-roko/** — Agent runtime (5 primitives, 9 protocols, 4 patterns)
- **02-daeji/** — Chain architecture (consensus, precompiles, contracts, registries)
- **03-isfr/** — ISFR oracle, yield perpetuals, benchmarks
- **04-markets/** — Business strategy and competitive landscape

## ERC-8004: Identity & Reputation

### Spec Target

Three registries:

1. **Identity Registry** — Soulbound ERC-721 passport, capability bitmask (64-bit, 14 defined), 4 tiers (Protocol/Sovereign/Worker/Edge by stake), system-prompt hash, TEE attestation hash, Agent Card URI

2. **Reputation Registry** — 7-domain EMA (OracleResolution, RiskDetection, AnomalyFlagging, DataIntegrity, CrossAppValidation, SealedExecution, KnowledgeVerification), TraceRank composite (25% consistency + 15% breadth + 25% depth + 20% recency + 15% collaboration), 5 tiers (Gray/Copper/Silver/Gold/Amber), decay toward 0.5 with 30-day half-life, authorized feedback sources only

3. **Validation Registry** — WorkProof struct, 4 validator types (reputation-based, stake-secured re-execution, zkML, TEE oracle)

### Current State

| Feature | Spec | Current | Gap |
|---------|------|---------|-----|
| Identity model | Soulbound ERC-721 | Simple mapping (address→Agent struct) | **Major** — no NFT, no soulbound semantics |
| Capability system | 64-bit bitmask, O(1) filter | String field ("capabilities") | **Major** — no structured capability matching |
| Tier system | 4 tiers by stake (100k/25k/5k/0) | 5 tiers by single reputation score | **Medium** — different axis (stake vs reputation) |
| System-prompt hash | SHA-256 commitment | Not present | **Missing** |
| TEE attestation | Optional, expiring hash | Not present | **Missing** |
| Agent Card URI | JSON document pointer | capabilities string with `endpoint=URL` | **Medium** — ad hoc vs structured |
| Reputation domains | 7 independent domains | 1 single score | **Major** — 1/7th of target model |
| TraceRank | Weighted composite of 5 factors | Not present | **Missing** |
| Reputation decay | Toward 0.5, 30-day half-life | Toward 0.5, 30-day half-life | **Done** ✓ |
| Authorized feedback | Contract-level authorization | OPERATOR_ROLE only | **Medium** — correct pattern, needs extension |
| Validation Registry | 4 validator types, WorkProof struct | CompletionProof (2-of-3 attestation only) | **Major** — 1/4 validator types |

### Effort Estimate

**Identity Registry:** ~200 lines new contract (ERC-721 + soulbound + capability bitmask + tier staking)
**Reputation Registry:** ~300 lines new contract (7-domain EMA + TraceRank + authorized sources)
**Validation Registry:** ~200 lines new contract (WorkProof struct + 4 validator types)

## ERC-8183: Job Marketplace

### Spec Target

7-state lifecycle: Posted→Bidding→Assigned→InProgress→Submitted→Verified→Settled (+ dispute path). Three hiring models: Random VRF (commodity), Blind Vickrey (standard), Direct Hire (specialized). IACPHook mechanism for cross-contract integration.

### Current State

| Feature | Spec | Current | Gap |
|---------|------|---------|-----|
| Lifecycle states | 7 states | 4 states (Open/Funded/Assigned/Submitted/Terminal) | **Medium** — missing Bidding, InProgress, Verified |
| Hiring models | 3 (VRF, Vickrey, Direct) | 1 (direct assignment only) | **Major** — no auction/VRF |
| IACPHook | Cross-contract integration | Not present | **Missing** |
| N-winner jobs | Hiring parameter | Separate contract (MultiAgentMarket) | **Medium** — should be unified |
| Dispute resolution | Built-in dispute path | Separate DisputeResolver contract | **Medium** — correct pattern, needs integration |
| Escrow | Token-based | Token-based | **Done** ✓ |
| Resolver pattern | Configurable | Configurable (setResolver) | **Done** ✓ |

### Effort Estimate

**BountyMarket rewrite:** ~400 lines (7-state machine + 3 hiring models + IACPHook)

## ISFR Oracle

### Spec Target

Validator-computed oracle: every validator independently reads source protocols (4 classes: LENDING 0.60, STRUCTURED 0.25, FUNDING 0.10, STAKING 0.05), two-level aggregation (intra-class TVL-weighted median + inter-validator stake-weighted median), published at precompile 0xA01, 4-state circuit breaker (Live/Degraded/Stale/Halted), CRPS scoring, confidence metric.

### Current State

| Feature | Spec | Current | Gap |
|---------|------|---------|-----|
| Computation model | Validator-computed in consensus | Keeper-submitted to contract | **Fundamental** — different architecture |
| Publication | Precompile 0xA01, constant gas | Contract storage | **Major** — needs precompile implementation |
| Source classes | 4 classes with fixed weights | 4 classes (in ISFROracle.sol) | **Done** ✓ (weights match) |
| Aggregation | Two-level (intra-class + inter-validator) | Trust-weighted median (single level) | **Major** — missing intra-class aggregation in validators |
| Circuit breaker | 4-state with hysteresis | Not present | **Missing** |
| Confidence score | Stake-weighted within-σ % | confidenceBps field exists but not computed | **Medium** |
| CRPS scoring | Epistemic reputation from predictions | Not present | **Missing** |
| Outlier exclusion | Two-pass 3σ filter | Not present | **Missing** |
| Cadence | Every 25 blocks (~10s) | When keepers submit | **Medium** |

### Effort Estimate

**Phase-1 interim:** Keep keeper-submitted model, add circuit breaker + confidence computation (~100 lines)
**Phase-2 validator-computed:** Requires consensus-layer changes in daeji (significant — source reading in validators, OracleVote in block proposal, precompile 0xA01)

## Knowledge Layer (InsightBoard)

### Spec Target

Hybrid architecture: on-chain anchoring (71 bytes per entry) + event logs (full text) + off-chain (HDC vectors, metadata). Six knowledge kinds with half-lives (Insight/Heuristic/Warning/AntiKnowledge/CausalLink/StrategyFragment). Four retention tiers with decay multipliers. NeuroChainSync bidirectional protocol (push when confidence≥0.70 + 3 distinct contexts; pull from events). AntiKnowledge conflict detection via HDC similarity.

### Current State

| Feature | Spec | Current | Gap |
|---------|------|---------|-----|
| On-chain anchoring | 71 bytes (hash, poster, timestamp, pheromone, type, half-life) | contentHash, poster, timestamp, pheromone, uri | **Medium** — missing type + half-life fields |
| Knowledge kinds | 6 kinds with different half-lives | Undifferentiated insights | **Major** |
| Retention tiers | 4 tiers (Transient/Working/Consolidated/Persistent) | Flat pheromone counter | **Major** |
| Decay | Per-kind half-lives + tier multipliers | No decay | **Major** |
| NeuroChainSync | Bidirectional push/pull with conditions | No sync protocol | **Missing** |
| AntiKnowledge | Conflict detection via HDC similarity | Not present | **Missing** |
| Confirmation rewards | REWARD_PER_CONFIRM | REWARD_PER_CONFIRM = 1 ether | **Done** ✓ |
| HDC vectors | 10,240-bit, precompile 0x09 | Not present | **Missing** (needs precompile) |

### Effort Estimate

**InsightBoard v2:** ~200 lines contract (add kind, half-life, tier fields)
**NeuroChainSync:** Agent-runtime feature, not chain-level
**HDC precompile:** Consensus-layer implementation (separate track)

## Precompiles

### Spec Target

| Address | Name | Status |
|---------|------|--------|
| 0x09 | HDC Similarity Search | Not implemented |
| 0x0B | QMDB Historical State Proofs | Not implemented |
| 0x0C | BTLE Encryption/Decryption | Not implemented |
| 0xA01 | ISFR Oracle | Not implemented |
| 0xA10-0xA1F | Agent Namespace | Not implemented |

### Current State

**None of the spec precompiles are implemented.** This is expected — they require consensus-layer changes in daeji, which is Phase 2 work.

### Dependencies

- 0x09 (HDC) requires: knowledge layer redesign, POPCOUNT/SIMD-optimized Hamming distance
- 0x0B (QMDB proofs) requires: QMDB state store integration (partially done in daeji)
- 0x0C (BTLE) requires: DKG ceremony, threshold VRF in consensus
- 0xA01 (ISFR) requires: validator-computed oracle in consensus
- 0xA10-0xA1F requires: ERC-8004 contracts deployed

## TEE Clearing Engine

### Spec Target

Commit-reveal-clear cycle with QP optimization, KKT verification on-chain, AWS Nitro enclaves, 37 clearing rounds verified.

### Current State

**Not started.** Depends on:
- ERC-8183 marketplace (for order flow)
- ISFR oracle (for mark price in yield perpetuals)
- ERC-8004 identity (for agent credentials in sealed bids)

### Effort Estimate

Significant standalone project. The spec describes it in detail but implementation requires:
- Nitro enclave setup
- QP solver implementation
- KKT verifier contract (~200 lines)
- Clearing protocol integration with marketplace

## Daeji Chain Architecture

### Spec Target

- Block time ~400ms, single-slot finality
- Simplex BFT with BLS12-381 threshold signatures
- QMDB authenticated KV store
- Phase-ordered blocks: ORACLE→ACCRUAL→LIQUIDATION→MATCHING
- Finality certificates (~240 bytes)
- (T,N) = (3,4) validator set

### Current State

| Feature | Status |
|---------|--------|
| Simplex BFT consensus | **Done** ✓ |
| BLS12-381 threshold signatures | **Done** ✓ |
| QMDB state store | **Done** ✓ (recent PR: state transition root computation) |
| ~400ms block time | **Done** ✓ |
| Single-slot finality | **Done** ✓ |
| Phase-ordered blocks | **Not implemented** |
| `block.timestamp` (wall-clock) | **Not implemented** (uses block height) |
| `BLOCKHASH` ring buffer | **Not implemented** |
| Finality certificate export | **Not implemented** |

Phase-1 blockers from spec's roadmap: `block.timestamp` fix and `BLOCKHASH` ring buffer must land before contract suite deploys.

## Communication Layer (Relay)

### Spec Target (from roko v2)

WebSocket relay implementing Bus fabric:
- Dynamic topic pub/sub
- Ring buffer with resume protocol
- Signal/Pulse duality
- Feed registration and discovery
- Group lifecycle driven by chain events
- 4 coordination modes
- NAT-friendly (outbound WebSocket)
- Language-agnostic (any agent that speaks WS+JSON)

### Current State (PR #24)

commonware-p2p mesh:
- 64 pre-allocated channels
- Rust-only
- NAT-hostile
- Typed message enum (one coordination pattern)
- Embedded in kora
- No reconnection/resume
- No feeds
- No chain-event delivery

**Gap: Fundamental architectural mismatch.** See pr24-review/ docs for detailed analysis.

## Summary: Distance from Spec

### Done (can ship)
- Simplex BFT consensus
- BLS12-381 threshold signatures
- QMDB state store
- ~400ms blocks with single-slot finality
- ISFR source class weights and structure
- EMA reputation decay model (30-day half-life toward 0.5)
- InsightBoard pheromone confirmation rewards
- Escrow + resolver pattern for jobs

### Close (contract rewrites, ~2 weeks)
- ERC-8004 Identity Registry (soulbound ERC-721 + capability bitmask)
- ERC-8183 BountyMarket (7-state lifecycle + 3 hiring models)
- InsightBoard v2 (knowledge kinds + retention tiers)
- `block.timestamp` and `BLOCKHASH` fixes
- Relay v1 (WebSocket + topic pub/sub + ring buffer)

### Medium-term (1-2 months)
- ERC-8004 Reputation Registry (7-domain EMA + TraceRank)
- ERC-8004 Validation Registry (4 validator types)
- Relay v2 (feeds + groups + chain-event delivery)
- ISFR circuit breaker + confidence scoring
- Finality certificate export

### Long-term (3-6 months)
- Validator-computed ISFR oracle (consensus-layer)
- HDC precompile (0x09)
- QMDB proof precompile (0x0B)
- BTLE precompile (0x0C)
- TEE clearing engine
- Phase-ordered blocks
- DKG ceremony tooling

### Not started, aspirational
- zkML proof integration
- Cross-chain light client
- Multi-operator validator resharing
- Yield perpetual instrument
- ClearingProfile intents
