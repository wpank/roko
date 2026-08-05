# ADR: Wire-or-Shelve Decision for Zero-Caller Chain Modules

**Status:** Accepted
**Date:** 2026-08-03 (revised 2026-08-05)
**Epic:** E11-CHAIN-ISFR
**Task:** E11-T05
**Author:** E11-T05 (documentation/decision task)

## Context

`roko-chain` is ~23K LOC across 30+ source files backed by 13 authored Solidity contracts.
Only the **ISFR vertical** (isfr_keeper, isfr_sources, isfr_oracle_submit, isfr_bootstrap)
plus supporting plumbing (alloy_impl, block_watcher, observer, mock, types, client, wallet,
chain_profile, tools, triage) is wired into the runtime today.

Alongside the wired surface, **19 modules contain real, tested Rust logic but have zero
runtime callers** -- 16,786 LOC of shelf-ware. These modules were built during parallel
development but were never connected to any CLI command, serve route, or runner dispatch path.

The chain surface is split across two repositories:

- **roko-chain** (`crates/roko-chain/`) is the **client/runtime integration side**: signs
  transactions, reads chain state via alloy RPC, deploys contracts via forge/alloy.
- **daeji** is a **separate devnet repo** that owns node software, BFT consensus,
  precompiles, and verified-state proofs. Design-only docs live at
  `tmp/agentchain-v2/02-daeji/` in this repo. Node-side and consensus features must NOT
  be built in roko.

This decision records which modules should be wired into the runtime (with follow-up tasks)
vs shelved as Phase 2+ (with explicit blocking dependencies).

## Decision

**All 19 zero-caller modules are SHELVE (Phase 2+).**

None of the 19 modules sit on the critical path for self-hosting. The blocking dependency
for most is the daeji devnet reaching a deployable state with real contracts. The ISFR
vertical is the only chain feature needed for current runtime operation, and it is already
wired. Wiring any individual module would be a follow-up epic, not a patch.

## Per-Module Verdicts

### 1. witness.rs (305 LOC) -- SHELVE

**What it does:** Marker-transaction anchoring and verification of off-chain computation
proofs (`ChainWitnessEngine`, `witness_on_chain`, `verify_on_chain`).

**Why shelve:** Requires a daeji witness registry contract to be deployed and reachable via
RPC. Also requires `get_logs` (E11-T02) for verification queries. No runtime consumer exists
-- runner does not anchor gate verdicts or episodes on-chain.

**Wire when:** daeji mainnet launches with a witness registry contract. Follow-up: wire
`ChainWitnessEngine` into the attestation/episode flow (config-gated).

### 2. x402.rs (958 LOC) -- SHELVE

**What it does:** HTTP 402 micropayment protocol with ERC-3009 authorization, state channels,
channel lifecycle, and settlement (`X402Manager`, `StateChannel`, `PaymentRequest`).

**Why shelve:** Requires a live token contract (KORAI or DAEJI) for ERC-3009 transfers.
No runtime consumer -- agent-server `/message` does not enforce payment. Jobs use file-based
`.roko/jobs/*.json`, not on-chain payment channels.

**Wire when:** Token contract is deployed and a 402-payment middleware is needed on
agent-server routes. Follow-up epic: 18-PAYMENTS.

### 3. korai_token.rs (657 LOC) -- SHELVE

**What it does:** Rust client for a demurrage token with lazy-decay mechanics
(`KoraiToken`, `KoraiTokenConfig`).

**Why shelve:** No KORAI.sol exists -- `Deploy.s.sol` uses `MockERC20("DAEJI")` as a
placeholder. The token contract must be authored and deployed before this client has
anything to talk to.

**Wire when:** KORAI token contract is authored, deployed, and the runtime needs to
query balances or transfer tokens.

### 4. marketplace.rs (1,096 LOC) -- SHELVE

**What it does:** Spore FSM job marketplace with Vickrey, Sparrow, and Direct auction
hiring models, escrow management, and settlement (`Marketplace`, `MarketplaceJob`).

**Why shelve:** Runtime jobs use file-based `.roko/jobs/*.json` (managed by roko-cli job
subcommands), not an on-chain marketplace. The on-chain marketplace needs deployed contracts
and an indexer to be useful.

**Wire when:** On-chain job market is needed (likely post-daeji-mainnet). The marketplace
contract must be deployed and indexed.

### 5. agent_registry.rs (785 LOC) -- SHELVE

**What it does:** ERC-8004 Rust client twin for `AgentRegistry.sol` with soulbound
passport queries (`AgentRegistry`).

**Why shelve:** Serve routes that interact with the agent registry use `sol!` ABI bindings
directly (alloy-generated), not this module. The Rust twin adds convenience methods but
duplicates what `sol!` already provides.

**Wire when:** Rust-native registry queries are needed that go beyond raw `sol!` calls,
or when the ERC-8004 trio is deployed and indexed with a proper event-driven data layer.

### 6. reputation_registry.rs (1,179 LOC) -- SHELVE

**What it does:** ERC-8004 Rust twin for `ReputationRegistry.sol` with 7-domain EMA
scoring and tier transitions (`ReputationRegistry`).

**Why shelve:** Same pattern as agent_registry -- serve uses `sol!` bindings, not this
module. The 7-domain scoring logic is comprehensive but has no consumer.

**Wire when:** Reputation-informed routing is implemented (CLAUDE.md item 13: neuro store
consulted for model selection in CascadeRouter). The ERC-8004 trio must be deployed first.

### 7. validation_registry.rs (456 LOC) -- SHELVE

**What it does:** ERC-8004 Rust twin for `ValidationRegistry.sol` with gate score
recording and verification results (`ValidationRegistry`).

**Why shelve:** Same pattern as agent/reputation registries. Serve uses `sol!` bindings.

**Wire when:** After ERC-8004 trio deployment and indexing.

### 8. isfr.rs / IsfrRegistry (1,277 LOC) -- SHELVE

**What it does:** 6-phase commit-reveal clearing engine for multi-party ISFR rate
consensus (`IsfrRegistry`, `IsfrConfig`).

**Why shelve:** The ISFR keeper does NOT use this module. The keeper submits rates directly
to the ISFROracle contract via `isfr_oracle_submit`. The clearing engine is a separate
multi-party protocol that has no runtime consumer.

**Wire when:** Multi-party ISFR clearing goes live (requires multiple rate submitters
coordinating via the commit-reveal protocol).

### 9. trace_rank.rs (508 LOC) -- SHELVE

**What it does:** PageRank-style reputation propagation over payment-edge graphs
(`TraceRank`). Computes composite reputation scores from multi-domain attestations.

**Why shelve:** Tested primitive with no runtime consumer. The reputation system does not
currently propagate scores across payment edges.

**Wire when:** Reputation-informed routing (CLAUDE.md item 13) is implemented, and payment
graph data is available from on-chain transactions.

### 10. collusion.rs (379 LOC) -- SHELVE

**What it does:** Clique-based collusion ring detection on assignment graphs. Identifies
groups of agents that suspiciously co-assign work.

**Why shelve:** Tested primitive with no consumer. Multi-agent marketplace (where collusion
detection matters) is not yet operational.

**Wire when:** On-chain multi-agent marketplace requires collusion detection as a safety
measure.

### 11. nelson_siegel.rs (307 LOC) -- SHELVE

**What it does:** Nelson-Siegel yield curve model for DeFi oracle rate term structure
interpolation.

**Why shelve:** Tested primitive. ISFR rate consumers do not currently need term-structure
interpolation -- the keeper publishes a single composite rate, not a yield curve.

**Wire when:** ISFR rate consumers need term-structure interpolation (e.g., computing
forward rates from a panel of maturities).

### 12. futures_market.rs (590 LOC) -- SHELVE

**What it does:** Interest rate futures market with order book, margin, and settlement
(`FuturesMarket`, `FuturesMarketConfig`).

**Why shelve:** Tested primitive. DeFi derivatives trading is not part of the current
runtime. Requires deployed contracts and a functioning rate oracle.

**Wire when:** DeFi derivatives trading is needed (significantly post-mainnet).

### 13. gate/mev_gate.rs (1,005 LOC) -- SHELVE

**What it does:** MEV detection gate that identifies sandwich bundles, frontrunning, and
other MEV patterns in mempool transactions (`MevGate`, `MevDetector`).

**Why shelve:** Not in the 7-rung gate pipeline (`runner/gate_dispatch.rs`). The current
gate pipeline covers compile, test, clippy, diff, and adaptive threshold rungs. MEV
detection is a chain-domain gate that has no runtime trigger.

**Wire when:** Chain transactions from roko agents need MEV protection (e.g., automated
on-chain trading or DeFi operations).

### 14. gate/tx_sim_gate.rs (445 LOC) -- SHELVE

**What it does:** Transaction simulation gate that pre-flights transactions against a
simulated EVM state (`TxSimGate`).

**Why shelve:** Not in the 7-rung gate pipeline. No chain transactions are submitted by
the runner that would need pre-flight simulation.

**Wire when:** Automated chain spending needs pre-flight transaction simulation for
safety.

### 15. gate/wallet_gate.rs (579 LOC) -- SHELVE

**What it does:** Wallet health and balance gate that checks whether the signing wallet
has sufficient funds before submitting transactions (`WalletGate`).

**Why shelve:** Not in the 7-rung gate pipeline. The only chain transactions today are
ISFR rate submissions, which are low-value and do not require a balance guard.

**Wire when:** Automated chain spending needs balance guards (e.g., contract deployments,
marketplace escrow funding).

### 16. heartbeat_ext.rs (475 LOC) -- SHELVE

**What it does:** Policy-cage extension for chain heartbeat monitoring with sleepwalker
detection and simulation-based pre-act checks (`ChainHeartbeatExtension`, `PolicyCageConfig`).

**Why shelve:** No runtime consumer. The agent lifecycle (ProcessSupervisor in roko-runtime)
does not use chain-aware heartbeat monitoring.

**Wire when:** Chain-aware agent lifecycle management is needed (e.g., agents that must
maintain on-chain liveness proofs).

### 17. phase2.rs (2,312 LOC) -- SHELVE

**What it does:** Phase 2 chain-layer stubs derived from `docs/08-chain`. Defines type aliases
(`Address`, `u256`, `i256`, `B256`), placeholder signing keys, RPC provider descriptors,
and deferred chain surface types (auction types, hiring models, staking, governance, DeFi
bridge stubs). Used as an internal dependency by other shelved modules (`marketplace.rs`,
`trace_rank.rs`, `identity_economy_markets.rs`, `korai_token.rs`, etc.) via `crate::phase2::*`.

**Why shelve:** Zero external callers. All external-crate references to `phase2::` are
`roko_dreams::phase2`, not `roko_chain::phase2`. The `pub use phase2::*` re-export in `lib.rs`
makes types available, but no downstream crate imports them. The module exists so internal
shelved modules can share type definitions without pulling in real alloy types.

**Wire when:** Never as a standalone module. Its types graduate to real alloy/chain types when
the modules that depend on them (marketplace, registries, etc.) are wired.

### 18. identity_economy_identity.rs (2,154 LOC) -- SHELVE

**What it does:** Phase 2+ identity stubs from `docs/14-identity-economy`. Defines placeholder
types for the deferred identity, reputation, passport, and attestation surfaces. Includes
`AgentId`, `Signature`, `ChainGateVerdict`, `GateType`, `X402Receipt` and supporting
identity-economy types.

**Why shelve:** Zero external callers. Only referenced internally by
`identity_economy_markets.rs` and `marketplace.rs`. The identity-economy docs describe a
post-mainnet identity layer that does not exist yet.

**Wire when:** The identity-economy layer is implemented, requiring deployed identity
contracts and an indexer.

### 19. identity_economy_markets.rs (1,319 LOC) -- SHELVE

**What it does:** Phase 2+ job-market, settlement, futures, and compliance stubs from
`docs/14-identity-economy`. Covers compliance policy enforcement, job-market clearing,
settlement protocols, and regulatory compliance types. References both
`identity_economy_identity` and `phase2` types internally.

**Why shelve:** Zero external callers. Only referenced by `marketplace.rs` internally. The
GAPS.md entry for E03-T07 already tracks the duplicate `Engram` struct at line 653 of this
file.

**Wire when:** The on-chain job market with compliance enforcement is needed
(post-daeji-mainnet + deployed marketplace + compliance contracts).

## Summary Table

| # | Module | LOC | Verdict | Blocking Dependency |
|---|--------|-----|---------|---------------------|
| 1 | `witness.rs` | 305 | SHELVE | daeji witness registry + `get_logs` |
| 2 | `x402.rs` | 958 | SHELVE | Live token contract + 402 middleware |
| 3 | `korai_token.rs` | 657 | SHELVE | KORAI.sol (not authored) |
| 4 | `marketplace.rs` | 1,096 | SHELVE | On-chain marketplace contracts + indexer |
| 5 | `agent_registry.rs` | 785 | SHELVE | ERC-8004 trio deployment + indexer |
| 6 | `reputation_registry.rs` | 1,179 | SHELVE | ERC-8004 trio + reputation routing |
| 7 | `validation_registry.rs` | 456 | SHELVE | ERC-8004 trio deployment |
| 8 | `isfr.rs` (IsfrRegistry) | 1,277 | SHELVE | Multi-party clearing protocol |
| 9 | `trace_rank.rs` | 508 | SHELVE | Reputation routing + payment graph |
| 10 | `collusion.rs` | 379 | SHELVE | On-chain marketplace |
| 11 | `nelson_siegel.rs` | 307 | SHELVE | ISFR term-structure consumers |
| 12 | `futures_market.rs` | 590 | SHELVE | DeFi derivatives (post-mainnet) |
| 13 | `gate/mev_gate.rs` | 1,005 | SHELVE | Chain tx MEV protection |
| 14 | `gate/tx_sim_gate.rs` | 445 | SHELVE | Pre-flight tx simulation |
| 15 | `gate/wallet_gate.rs` | 579 | SHELVE | Automated chain spending |
| 16 | `heartbeat_ext.rs` | 475 | SHELVE | Chain-aware agent lifecycle |
| 17 | `phase2.rs` | 2,312 | SHELVE | Internal type provider for other shelved modules |
| 18 | `identity_economy_identity.rs` | 2,154 | SHELVE | Identity-economy contracts (post-mainnet) |
| 19 | `identity_economy_markets.rs` | 1,319 | SHELVE | Marketplace + compliance contracts (post-mainnet) |
| | **Total shelf-ware** | **16,786** | | |

## daeji Boundary Fence

The following responsibilities belong to the **daeji devnet** (separate repo) and must
NOT be built in roko-chain:

- **Node software** (validator/full node binary)
- **BFT consensus** (PBFT/HotStuff variant)
- **Precompiles** (HDC similarity, ZK verification, custom opcodes)
- **Verified-state proofs** (QMDB proofs, BLS aggregate signatures)
- **Genesis configuration** and chain parameters

Design-only documentation for daeji lives at `tmp/agentchain-v2/02-daeji/` in this repo
for cross-reference. That directory is informational, not executable.

roko-chain's responsibility is limited to:

- **Signing and submitting transactions** (via alloy provider + wallet)
- **Reading chain state** (via alloy RPC: `get_block`, `get_receipt`, `get_logs`, etc.)
- **Deploying contracts** (via forge scripts and alloy-based bootstrap)
- **ISFR rate management** (sources, keeper, oracle submit, bootstrap)
- **Client-side domain logic** for chain features (registries, witness, marketplace, etc.)

## Wired Surface (for reference)

The following modules ARE wired into the runtime today:

| Module | Runtime Caller |
|--------|---------------|
| `alloy_impl.rs` | `roko-serve` state, `isfr_bootstrap` |
| `isfr_keeper.rs` | `roko-serve` lib, CLI `commands/isfr.rs` |
| `isfr_sources/` | `isfr_keeper` |
| `isfr_oracle_submit.rs` | `isfr_keeper` |
| `isfr_bootstrap.rs` | `roko-serve` lib |
| `block_watcher.rs` | `roko-serve` lib |
| `observer.rs` | `roko-serve`, `job_runner` |
| `mock.rs` / `types.rs` / `client.rs` / `wallet.rs` | plumbing (traits, test doubles) |
| `chain_profile.rs` / `tools.rs` / `triage.rs` | plumbing (config, tool defs, event enrichment) |

## Cross-References

- **`.roko/GAPS.md`**: Contains the per-module WIRE/SHELVE table (canonical gap tracker).
- **`CLAUDE.md`**: roko-chain row updated to reflect ISFR-wired reality + daeji fence.
- **`docs/v2/22-REGISTRIES.md`**: Maturity note at top (lines 20-41) clarifies that
  registries, witness, gossip, and marketplace sections are spec-level designs, not
  runtime-live code.
- **`tmp/status-quo/backlog/epics/E11-CHAIN-ISFR.md`**: Epic with full module status table
  and finding analysis.

## Consequences

1. No runtime wiring work is needed for any of the 19 modules in the current development
   phase. Self-hosting does not depend on any of them.

2. The 16,786 LOC of shelf-ware remains compiled and tested but not called. This adds to
   compile time but does not affect runtime behavior.

3. Each module's "wire when" condition is documented. When a blocking dependency is resolved
   (e.g., daeji mainnet, KORAI token contract), the corresponding module can be wired as a
   follow-up epic with a clear scope.

4. The daeji fence prevents accidental scope creep into node-side features that belong in
   the separate devnet repo.

5. Three modules (17-19) form an internal dependency chain: `phase2.rs` provides shared type
   aliases consumed by `identity_economy_identity.rs` and `identity_economy_markets.rs`.
   None of the three have any external callers. They exist to match the documented type
   landscape and will graduate to real implementations when their downstream modules are wired.
