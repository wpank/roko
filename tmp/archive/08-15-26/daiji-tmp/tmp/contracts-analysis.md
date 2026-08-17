# Contracts-Core Analysis

## Inventory: packages/agents/src/

17 contracts deployed as a unified suite via `Deploy.s.sol`.

### Identity & Reputation (ERC-8004 overlap)

| Contract | What it does | ERC-8004 overlap | Verdict |
|----------|-------------|------------------|---------|
| **AgentRegistry** | Minimal identity: register(passportHash, capabilities), heartbeat(), 200-block liveness | Direct — soulbound identity, liveness tracking | **Replace** with vanilla ERC-8004 Identity Registry (ERC-721 passport, capability bitmask, tier-based staking, system-prompt hash, TEE attestation) |
| **WorkerRegistry** | EMA reputation (α=0.2), 5 tiers, MIN_BOND=1000, 30-day decay, slashing | Overlaps ERC-8004 Reputation Registry but single-domain | **Replace** with vanilla ERC-8004 Reputation Registry (7-domain EMA, TraceRank composite, adaptive alpha, authorized feedback sources) |
| **RoleRegistry** | MANAGER_ROLE + OPERATOR_ROLE, 2-step owner transfer | Access control pattern, not identity | **Keep** as infrastructure (simple, correct) |

**Key gap:** Current WorkerRegistry has one reputation score. ERC-8004 spec has seven independent domains (OracleResolution, RiskDetection, AnomalyFlagging, DataIntegrity, CrossAppValidation, SealedExecution, KnowledgeVerification). Current tier system (5 levels by single score) should become TraceRank composite (5 levels: Gray/Copper/Silver/Gold/Amber by weighted blend of 7 domains).

### Job Escrow (ERC-8183 overlap)

| Contract | What it does | ERC-8183 overlap | Verdict |
|----------|-------------|------------------|---------|
| **BountyMarket** | 4-state escrow: Open→Funded→Assigned→Submitted→Terminal, resolver pattern | Direct — but simpler lifecycle than spec | **Replace** with vanilla ERC-8183 BountyMarket (7-state lifecycle, 3 hiring models: VRF/Vickrey/Direct, IACPHook) |
| **MultiAgentMarket** | N-winner variant, emits roomId for chat channels | BountyMarket sibling, daeji-chat dependency | **Merge** into ERC-8183 (N-winner is just a hiring-model parameter, not a separate contract) |
| **ConsortiumValidator** | 3-of-3 committee, blockhash-seeded selection | ERC-8004 Validation Registry (reputation-based validator type) | **Replace** — blockhash randomness is manipulable; spec uses VRF |
| **DisputeResolver** | Mediator proxy, 48-hour evidence window | ERC-8183 dispute path | **Adapt** — useful pattern, wire into vanilla lifecycle |
| **CompletionProof** | 2-of-3 attestation artifact | ERC-8004 Validation Registry (work proofs) | **Replace** with ValidationRegistry WorkProof struct |

**Key gap:** Current BountyMarket has 4 states (Open→Funded→Assigned→Submitted→Terminal). Spec has 7 states (Posted→Bidding→Assigned→InProgress→Submitted→Verified→Settled + dispute path). Current hiring is direct assignment only. Spec has three models: Random VRF (commodity), Blind Vickrey (standard), Direct Hire (specialized).

### ISFR Oracle

| Contract | What it does | Spec target | Verdict |
|----------|-------------|-------------|---------|
| **IISFROracle** | Interface: ISFRSnapshot struct, currentRate/snapshotAt views | Matches ISFR spec §3.5 layout | **Keep** as reference |
| **ISFRMinimal** | Hardcoded reference values (composite_bps=690) | Test stub only | **Drop** when real oracle lands |

**Key gap:** The spec calls for a **validator-computed oracle** — every validator independently reads source protocols and submits OracleVote, aggregated via stake-weighted median in consensus. The current design uses a separate keeper-submitted oracle contract. The spec's approach eliminates the separate-operator trust model entirely.

### Post-Resolution & Auxiliary

| Contract | What it does | Verdict |
|----------|-------------|---------|
| **FeeDistributor** | 40/30/20/10 split (validators/provider/agent/treasury) | **Adapt** — useful pattern, but fee split should be configurable |
| **InsightBoard** | On-chain knowledge with pheromone confirmations, REWARD_PER_CONFIRM | **Adapt** — spec adds 6 knowledge kinds, 4 retention tiers, HDC vectors, NeuroChainSync, AntiKnowledge conflict detection |
| **JobTypeRegistry** | bytes32→JobTemplate mapping, MANAGER gated | **Drop** — ERC-8183 capability bitmask + hiring model replaces typed templates |
| **NotificationRegistry** | On-chain subscription preferences (webhook/email/in-app) | **Drop** — relay handles event delivery; on-chain notification prefs are over-engineered |

### Job Wrappers

| Contract | What it does | Verdict |
|----------|-------------|---------|
| **FundingRateKeeperJob** | Typed wrapper for funding-window settlement jobs | **Drop** — ERC-8183 specHash encoding handles this generically |
| **OracleUpdaterJob** | Typed wrapper for oracle-update keeper jobs | **Drop** — same |
| **PerpsLiquidatorJob** | Typed wrapper for perps-liquidation keeper jobs | **Drop** — same |

### Test Infrastructure

| Contract | Verdict |
|----------|---------|
| **MockERC20** | **Keep** — always need test tokens |

## Dependency Graph

```
RoleRegistry (access control)
  ↓
WorkerRegistry (stake + reputation)  ←  IERC20
  ↓
BountyMarket (job escrow)  ←  IERC20, IRoleRegistry
  ↓
ConsortiumValidator (2-of-3 resolver)  →  BountyMarket.resolve()
DisputeResolver (mediator resolver)    →  BountyMarket.resolve()
  ↓
FeeDistributor (payment split)  ←  IERC20
CompletionProof (attestation)   ←  ConsortiumValidator
```

## What Vanilla ERC-8004 + ERC-8183 Replaces

### ERC-8004 replaces 4 contracts:

| Current | Vanilla replacement |
|---------|-------------------|
| AgentRegistry | Identity Registry (ERC-721 passport, capability bitmask, tier staking, heartbeat) |
| WorkerRegistry | Reputation Registry (7-domain EMA, TraceRank, decay, authorized feedback) |
| ConsortiumValidator | Validation Registry (4 validator types: reputation, re-execution, zkML, TEE) |
| CompletionProof | Validation Registry WorkProof struct |

### ERC-8183 replaces 5 contracts:

| Current | Vanilla replacement |
|---------|-------------------|
| BountyMarket | BountyMarket (7-state lifecycle, 3 hiring models, IACPHook) |
| MultiAgentMarket | BountyMarket with N-winner hiring parameter |
| FundingRateKeeperJob | Generic specHash encoding |
| OracleUpdaterJob | Generic specHash encoding |
| PerpsLiquidatorJob | Generic specHash encoding |

### Remaining after replacement: 6 contracts

1. **RoleRegistry** — keep as access control infrastructure
2. **FeeDistributor** — adapt with configurable splits
3. **InsightBoard** — adapt to spec's knowledge layer
4. **DisputeResolver** — adapt into ERC-8183 dispute path
5. **MockERC20** — test infrastructure
6. **IISFROracle** — interface reference

## PR Landscape (Key PRs)

### ISFR Stack (#102-#106, all OPEN)
Five stacked PRs porting ISFR from standalone repo to contracts-core. Key decisions:
- No parallel stake/reputation stack — uses WorkerRegistry
- Access control via RoleRegistry
- Trust-weighted median preserved
- Admin-settable range parameters
- Bounty pool is Phase-1-only interim

### MultiAgentMarket (#111 MERGED, #113/#116/#117/#118 OPEN)
N-winner sibling of BountyMarket with roomId derivation for daeji-chat. The daeji-chat PR #24 depends on this for JobAwarded events.

### Audit PRs (#88/#91/#110)
JaeLeex review on #100 found 15 MUST-FIX findings. These likely need re-review if contracts are replaced with vanilla ERC specs.
