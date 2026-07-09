# A — Vision, Chain Spec, Token Economics, Status (Docs 00, 01, 02, 24)

Parity of the four framing / spec / status documents in topic 08. These
chapters establish what the Korai chain is supposed to be, how it
parameterizes an EVM, how its native token works, and what the 6-contract
roadmap looks like. Most entries here are explicit **Phase 2+ frontier**
work — the chain layer is Tier 6 deferred per CLAUDE.md priority item 11,
so nearly all on-chain subsystems are design-only. The interesting drift
is between (a) the doc claim that zero Solidity contracts exist and (b)
the presence of seven **different** Solidity files under `contracts/` that
do not match the Korai 6-contract roadmap.

Generated 2026-04-16.

---

## A.01 — "Blockchain as ONE domain plugin" framing is structurally honored (Doc 00 §"Abstract", §"The Ecosystem Problem")

**Status**: DONE
**Severity**: —
**Doc claim**: "The Korai chain is a domain plugin that extends this kernel with chain-specific capabilities ... one instance of the pattern `domain_specific_trait_implementations + domain_specific_configuration = domain_agent`." Nothing about the chain layer requires special treatment in the Roko kernel.
**Reality**: `crates/roko-core/src/traits.rs` defines the six canonical verb traits (`Substrate`, `Scorer`, `Gate`, `Router`, `Composer`, `Policy`) with no chain-specific leakage. `crates/roko-chain/` is a stand-alone leaf crate: its `Cargo.toml` depends only on `roko-core` (for `Attestation`, `ChainAttestation`, `ContentHash`) plus `async-trait`, `serde`, `thiserror`, `tokio`, `parking_lot`, `alloy` (feature-gated), `k256` (feature-gated). No coupling in the reverse direction — `Grep 'roko_chain|roko-chain' crates/roko-core` returns zero matches. The gates (`WalletGate`, `TxSimGate`) implement `roko_core::traits::Gate` (see `gate/wallet_gate.rs:17` and `gate/tx_sim_gate.rs:21`), and `roko_bridge` impls for `Substrate` live in the mirage-rs app, not the kernel. The framing holds.

---

## A.02 — Three-level knowledge architecture (Local → Mesh → Chain) is only 1/3 shipped (Doc 00 §"Three-Level Knowledge Architecture")

**Status**: PARTIAL
**Severity**: LOW
**Doc claim**: Agents access knowledge at three levels: (1) Local Neuro Store (per-agent JSONL + HDC, tiered decay) managed by `roko-neuro`; (2) Agent Mesh (WebSocket + Iroh P2P); (3) Korai Chain (ERC-8004 identity + HDC precompile + KORAI demurrage + pheromone contracts + reputation registry).
**Reality**: Level 1 ships: `crates/roko-neuro/` is listed as "Wired" in CLAUDE.md key-crates table (durable knowledge store, distillation, tier progression). Level 2 is partial: WebSocket is the standard `roko-agent-server` sidecar surface (`/stream` WS) and broadcast `InsightBus` / `PheromoneBus` (`apps/mirage-rs/src/roko_bridge/subscription/`), but there is no Iroh P2P integration anywhere — `Grep 'iroh' crates/ apps/` returns zero matches outside this single doc. Level 3 is entirely Phase 2+: no ERC-8004 registries (see B.01), no HDC precompile (see B.02), no KORAI demurrage code (see A.06-A.08), no pheromone contracts, no reputation registry. Doc 00 should mark Level 2 Iroh and all of Level 3 as `Design — not yet implemented`.
**Fix sketch**: Annotate Doc 00 §"Three-Level Knowledge Architecture" with a per-level shipping status column: Level 1 `Shipping`, Level 2 `Shipping (WebSocket only; Iroh planned)`, Level 3 `Design — not yet implemented`.

---

## A.03 — Chain selection rationale is pure Phase 2+ design (Doc 00 §"Chain Selection Rationale")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Doc 00's enhancement adds "chain selection rationale (EVM vs. Move/CosmWasm/SVM)", Arbitrum Orbit L3 deployment, Stylus precompile path, EigenLayer AVS, cross-chain interop via Hyperlane ISM / IBC / intent bridges.
**Reality**: None of this has code. `Grep 'Arbitrum|Orbit|Stylus|EigenLayer|Hyperlane|IBC' crates/ apps/` returns zero matches. The entire chain-selection enhancement is a design essay pasted into a PRD — fine as long as it is explicitly marked frontier work. Today it is not banner-tagged as such.
**Fix sketch**: Add a `Design — Phase 2+` banner to Doc 00 §"Chain Selection Rationale" and §"L2/L3 Deployment" subsections introduced in the 2026-04-13 enhancement pass.

---

## A.04 — 400ms block time / 2s finality / chain parameter table is design-only (Doc 01 §"Chain Parameters")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Doc 01 tables 400ms target block time, 2s soft finality, custom `korai_*` RPC namespace, chain intelligence pipeline, three deployment modes (standalone L1, Arbitrum Orbit L3, EigenLayer AVS).
**Reality**: No Korai L1 node exists. The closest shipping surface is `apps/mirage-rs/` which runs revm in-process at a user-configurable `block_time_ms` (see `apps/mirage-rs/src/lib.rs` `MirageConfig`; Doc 18 `:84` shows `block_time_ms: 400` as an example, not a hard-coded constant). The `korai_*` RPC namespace does not exist (see F.15 — `Grep 'korai_' apps/mirage-rs/src/chain_rpc.rs` returns zero matches; the file uses `chain_*` and `eth_*` prefixes). Soft-finality, validator set, checkpointing, and the "chain intelligence pipeline" described in Doc 01 §"Chain Intelligence Pipeline" are Phase 2+.
**Fix sketch**: Doc 01 should distinguish "Chain Design Parameters" (what the Korai mainnet will eventually target) from "Current Simulator Defaults" (`mirage-rs` 400ms config). Flag the RPC-namespace mismatch (`korai_*` → `chain_*`) since the mirage simulator is the only place these methods could currently live.

---

## A.05 — Korai chain binary does not exist (Doc 01 §"Deployment Modes")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Doc 01 §"Deployment Modes" describes three deployment targets for the Korai chain itself. The doc's `--implementation Built` banner implies at least mode 1 (standalone L1) is live.
**Reality**: No Korai chain node exists in the repo. `find /Users/will/dev/nunchi/roko/roko -name 'korai*' -o -name 'Korai*' 2>&1 | grep -v target | grep -v docs` returns no chain binary — only the docs. `apps/mirage-rs/` is a *simulator*, not a chain. `apps/roko-chain-watcher/` is an *observer*, not a chain. The banner "Implementation: Built" on Doc 01 is over-claiming.
**Fix sketch**: Downgrade Doc 01's banner from "Implementation: Built" to "Implementation: Design — Phase 2+". Explicitly state in the abstract that the Korai chain is specified but not running; `mirage-rs` is the development surrogate.

---

## A.06 — KORAI token contract does not exist (Doc 02 §"Demurrage", Doc 24 §"5. KORAI Token")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: KORAI token implements ERC-20 + ERC-3009 `transferWithAuthorization` + demurrage (1% annual, per-block decay factor `(1 - 0.01)^(1/78_894_000)` at 400ms blocks). Predeployed at Korai genesis.
**Reality**: `Grep 'KORAI|demurrage|transferWithAuthorization' crates/ apps/ --include=*.rs` returns zero matches on the token contract side. The closest shipping item is `contracts/src/MockERC20.sol` (23 LOC, a plain ERC-20 used by tests), which has no ERC-3009 and no demurrage. No per-block decay factor, no `applyDemurrage(address)`, no epoch handler. `apps/mirage-rs/src/chain/mod.rs:17-22` explicitly documents that the mirage POC "collapses GNOS/KORAI to plain ETH (wei)" and defers demurrage to callers.
**Fix sketch**: Mark Doc 02 as `Design — Phase 2+` globally. The demurrage math can stay; the "predeployed at genesis" claim cannot until a Korai chain ships.

---

## A.07 — Five earning + five spending mechanisms are design-only (Doc 02 §"Earning KORAI", §"Spending KORAI")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Five earning mechanisms (registration mint, validated knowledge posting, knowledge confirmation, job completion, governance participation). Five spending mechanisms (posting knowledge, querying knowledge, bidding for jobs, staking into domains, burning for governance weight).
**Reality**: `Grep 'registration_mint|validated_posting|earn_korai|spend_korai' crates/ apps/` returns zero matches. No earning / spending routines exist — no on-chain mechanism to mint, burn, or distribute KORAI. The entire 10-mechanism table is specification.
**Fix sketch**: Add a `Design — Phase 2+` banner at the top of Doc 02 §"Earning KORAI" + §"Spending KORAI". Cross-link to `roko-learn` efficiency events as the nearest shipping ancestor (learn tracks agent behaviour per turn but does not settle in a token).

---

## A.08 — Fee distribution and demurrage-to-NeuroStore relationship are frontier (Doc 02 §"Fee Distribution", §"Relationship to NeuroStore Decay")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Fee distribution model (posters / validators / protocol / burn), and the claim that demurrage decay is "isomorphic" to NeuroStore Ebbinghaus half-life decay.
**Reality**: `crates/roko-neuro/` does implement real half-life tier decay (listed as "Wired" in CLAUDE.md), so the concept it is isomorphic to exists. But there is no on-chain side to mirror: no fee distribution contract, no burn schedule, no coupling between `roko-neuro` decay ticks and any token flow. The isomorphism is a design aspiration.
**Fix sketch**: Keep the "Relationship to NeuroStore Decay" essay in Doc 02 as design rationale; mark "Fee Distribution" Phase 2+; note that the real Ebbinghaus half-life lives in `roko-neuro` and that the chain side is frontier.

---

## A.09 — Doc 24 "What Is Built" undercounts the real shipping surface (Doc 24 §"What Is Built", §"What Is Scaffolded")

**Status**: PARTIAL
**Severity**: HIGH (for doc honesty)
**Doc claim**: Doc 24's "What Is Built" table lists 10 rows (ChainClient / ChainWallet traits, types, TxSimGate stub, WalletGate stub, mocks, mirage-rs core, mirage-rs chain extensions scaffold, HDC local ops, HDC index HNSW, AgentPassport specified). Two of those are tagged "Stub" and two point at a stale crate path (`bardo-primitives` instead of `roko-primitives`).
**Reality**: Four systematic drifts:
1. **`TxSimGate stub`** and **`WalletGate stub`** — both are fully-implemented Gates (see F.09, F.10). `TxSimGate` is 448 LOC + pluggable `TxSimulator` trait + MockTxSimulator. `WalletGate` is 523 LOC + `WalletCheck` enum + balance / nonce verification. The only pending piece is Permit2 allowance in `WalletGate`.
2. **`HDC local operations | bardo-primitives/src/hdc.rs`** — wrong crate path. The file lives at `crates/roko-primitives/src/hdc.rs` (see F.01 cross-link; `bardo-primitives` no longer exists as a crate name).
3. **Missing rows**: `AlloyChainClient` (327 LOC, feature-gated live backend), `roko_bridge` `Substrate` / `Gate` impls (~1,100 LOC across `apps/mirage-rs/src/roko_bridge/`), `ChainWitnessEngine` (199 LOC in `roko-chain/src/witness.rs`), `apps/roko-chain-watcher/` (2,931 LOC observer app), the 7 Solidity files in `contracts/src/` (760 LOC; see A.11).
4. **`mirage-rs chain extensions | Scaffold`** — this row is fine directionally but the scope is mis-described; see F.12 and the scaffold inventory batches (K3). The scaffold implements a *generic* agent-coordination knowledge layer, not the Korai-specific registry emulation the doc implies.

Doc 24 is the canonical status doc; its drift propagates into any reader who uses it as the source of truth. High severity for *readability*, not runtime.
**Fix sketch**: Re-generate Doc 24 §"What Is Built" from SOURCE-INDEX.md rows. Each row should cite a current path and LOC count. Add a §"What Is Scaffolded In mirage-rs chain/" subsection listing the 9 scaffold modules with a one-liner each.

---

## A.10 — "Zero Solidity contracts" is false; seven contracts exist under `contracts/` (Doc 24 §"Six Planned Solidity Contracts")

**Status**: PARTIAL
**Severity**: HIGH (for doc honesty)
**Doc claim**: Doc 24 §"Six Planned Solidity Contracts" lists Agent Registry (0xA100), Reputation Registry (0xA200), Marketplace (Spore), Escrow, KORAI Token, Validation Registry (0xA300) as "All are Tier 6 deferred — blocked by Tier 5 completion." The implication throughout Docs 00/24 is that no Solidity has been written yet.
**Reality**: `contracts/src/` ships **7 Solidity files, 760 LOC**, with matching test files at `contracts/test/*.t.sol`:

| File | LOC | What |
|---|---|---|
| `contracts/src/AgentRegistry.sol` | 73 | Minimal ERC-8004-style identity (address → capabilities + passportHash + lastHeartbeat). `LIVENESS_WINDOW = 200` blocks. Events: `AgentRegistered`, `AgentHeartbeat`, `AgentCapabilitiesUpdated`. |
| `contracts/src/WorkerRegistry.sol` | 233 | Worker registration with capability + stake + slash primitives. |
| `contracts/src/BountyMarket.sol` | 136 | Bounty / job posting with claim + settle flow. |
| `contracts/src/InsightBoard.sol` | 78 | On-chain insight posting surface. |
| `contracts/src/ConsortiumValidator.sol` | 114 | Consortium-style validation committee. |
| `contracts/src/FeeDistributor.sol` | 103 | Fee split contract. |
| `contracts/src/MockERC20.sol` | 23 | Plain ERC-20 test fixture. |

None of these map 1-to-1 to the six Korai contracts. The names and surfaces are closer to a prior POC / bardo-heritage set. CLAUDE.md confirms: `roko-demo chain integration | crates/roko-demo/ | Built | Full chain wallet + client demo via alloy-backend feature`. So `contracts/` is the demo contract set, not the canonical Korai v1.
**Fix sketch**: Doc 24 should add a §"Currently-shipping demo contracts" subsection listing the 7 files above with a one-liner each and an explicit callout that these are **not** the six planned Korai v1 contracts. Clarify whether Korai v1 will (a) live downstream in a different repo, (b) re-use / extend the demo contract set, or (c) replace it entirely. Without this clarification the reader cannot tell whether the demo set is progress toward Korai v1 or unrelated.

---

## A.11 — Soulbound ERC-721 passport is absent in shipping Solidity (Doc 24 §"1. Agent Registry")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Agent Registry at `0xA100` manages Korai Passports — **soulbound ERC-721 NFTs** serving as agent identity.
**Reality**: `contracts/src/AgentRegistry.sol:9-73` is **not** an ERC-721 contract at all — it is a plain `mapping(address => Agent)` with `register`, `heartbeat`, `updateCapabilities`, `isActive`, `getAgent`, `registeredCount`, `registeredAt(index)`. No `tokenId`, no `ownerOf`, no non-transferability enforcement, no soulbound event gates, no tier bitmask. A soulbound ERC-721 Passport contract has not been written.
**Fix sketch**: See B.02 for the Passport design-vs-reality entry. For Doc 24, add a callout that the shipping `contracts/src/AgentRegistry.sol` is a **minimal identity ledger**, not a soulbound Passport, and the latter is Phase 2+ Tier-6 deferred work.

---

## A.12 — Reputation Registry at 0xA200 is absent (Doc 24 §"2. Reputation Registry")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Reputation Registry at `0xA200` stores per-domain reputation scores + manages feedback authorization. Functions: `submitFeedback`, `applyDecayTick`, `slash`, `getReputation`, `isAuthorizedFeedbackSource`, `addFeedbackSource`.
**Reality**: `Grep 'Reputation|ReputationRegistry|reputation_score' contracts/src/` returns zero matches. `ConsortiumValidator.sol` touches on validation but not reputation. None of the Rust side implements the 7-domain EMA reputation either (see D.05). Doc 24 §"2. Reputation Registry" is pure design.
**Fix sketch**: Mark Doc 24 §"2. Reputation Registry" with `Design — Phase 2+ Tier 6`. Link to `docs/08-chain/14-reputation-system-7-domain.md` for the full spec. Cross-ref D.05 for the 7-domain EMA parity entry.

---

## A.13 — Marketplace (Spore), Escrow, Validation Registry are absent (Doc 24 §§"3", "4", "6")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Marketplace (Spore) implements job lifecycle (postJob → submitBid → revealBid → acceptDirectHire → submitDeliverables → dispute / resolve). Escrow holds budgets in escrow with 2% escrow fee + 3% marketplace fee. Validation Registry at `0xA300` records work proofs.
**Reality**: None of these contracts exist. `contracts/src/BountyMarket.sol:1-136` is the nearest shipping piece — it posts + claims + settles **bounties** but is not the Spore auction / sealed-bid / direct-hire lifecycle. No `postJob`, `revealBid`, `acceptDirectHire`. No escrow-fee split. No `submitWorkProof(...)`, `verifyWork(jobHash)`, `getGatePassRate(passportId, domain)` on any Validation contract.
**Fix sketch**: In Doc 24, mark §§3-6 all `Design — Phase 2+`. Add a callout that `contracts/src/BountyMarket.sol` is the closest existing analogue and may (or may not) evolve into the Spore Marketplace — needs product decision.

---

## A.14 — 76-item chain-layer implementation plan is still valid (Doc 24 §"Implementation Status")

**Status**: DONE (reference)
**Severity**: —
**Doc claim**: Doc 24 cites `roko/tmp/implementation-plans/12b-chain-layer.md` with 76 items across 11 sections (A Identity, B Gossip, C Job Market, H ChainWitness, K Reputation, L Payments, M Safety, N ISFR, O Clearing, P Privacy, Q mirage-rs, R Crate Architecture).
**Reality**: The plan file exists at `tmp/implementation-plans/12b-chain-layer.md` per CLAUDE.md and is the plan-of-record for chain-layer execution. Nothing in this parity audit invalidates it — the 76 items remain the right bounded work, and most correspond 1-to-1 to individual entries in this batch's sections A-G. The parity audit should feed back into that plan (e.g., remove §R1/§R5/§Q4a from "pending" because they already ship; see F.07, F.09, F.10, F.13).
**Fix sketch**: After Doc 24 is reconciled with the findings here, run `tmp/implementation-plans/12b-chain-layer.md` through the same lens and mark items that are now DONE / PARTIAL.

---

## A.15 — Tier 6 dependency graph is accurate (Doc 24 §"Tier 6 Dependencies")

**Status**: DONE
**Severity**: —
**Doc claim**: Tier 6 chain layer is blocked by Tier 5 (self-hosting loop). Tier 5 blockers: Interactive TUI, automatic plan generation, feedback loop. Recommended Solidity build order: KORAI Token → Agent Registry → Reputation Registry → Validation Registry → Escrow → Marketplace.
**Reality**: CLAUDE.md "What to work on" priority list confirms: items 10-11 ("automatic plan generation" and "feedback loop") remain open; items 1-9 (TUI, SystemPromptBuilder, EpisodeLogger, ProcessSupervisor, MCP, learning/feedback instrumentation, TUI T1-T19 parity merged, per-agent sidecar, HTTP control plane) are Done. So Tier 5 is most of the way closed but items 10-11 are the canonical blockers that remain. The dependency claim is accurate today. The build-order graph is a product decision with no code dependency.

---

## A.16 — 6 canonical precompile / registry addresses are unused today (Doc 24 §"6 Contracts", Doc 18 §"Emulated Registries")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Addresses baked into the design: `0xA01` (HDC precompile), `0xA100` (Agent Registry), `0xA200` (Reputation Registry), `0xA300` (Validation Registry).
**Reality**: `Grep '0xA01\b|0xA100\b|0xA200\b|0xA300\b' crates/ apps/ contracts/` returns zero hits. These addresses are referenced ONLY in the docs. `crates/roko-chain/src/witness.rs:13` uses `0x00000000000000000000000000000000000000c0` as a witness sink — unrelated. `roko-chain/src/types.rs` stores all addresses as `String` (see F.04) — no shared constant table.
**Fix sketch**: When a `roko-chain-addresses` module is eventually introduced, wire the four addresses as shared constants (`pub const HDC_PRECOMPILE: &str = "0xa01";` etc.) and cross-link them to the ERC-8004 registry spec.

---

## Section Summary

| Status | Count |
|--------|-------|
| DONE | 3 (A.01 domain-plugin framing, A.14 76-item plan reference, A.15 Tier-6 dependency graph) |
| PARTIAL | 3 (A.02 three-level knowledge 1/3 shipped, A.09 Doc 24 undercounts, A.10 7 demo Solidity contracts exist) |
| NOT DONE | 10 (A.03 chain selection, A.04 chain params, A.05 Korai node, A.06 KORAI token, A.07 earn/spend, A.08 fee/demurrage isomorphism, A.11 soulbound passport, A.12 reputation registry, A.13 marketplace/escrow/validation, A.16 canonical addresses) |

Docs 00 / 01 / 02 / 24 are the chain-layer's framing + spec chapters.
They mostly describe Tier-6 design work and behave honestly about that
deferral — the abstracts clearly mark "Implementation: Built" when Built
is the kernel hooks (A.01) and "Implementation: Design" would be more
accurate elsewhere. The two MEDIUM-weight doc-drift items in this section
(A.09, A.10) are both in the **status** doc (Doc 24), which is the place
most readers turn to first. A.09 understates what the code ships; A.10
hides the fact that 7 Solidity files already exist in `contracts/`. Fixing
those two alone would make the chain-layer parity story legible without
any code changes.

## Agent Execution Notes

### A.09 / A.10 — Canonical Status Doc Honesty (1 pass)

Best use of this section in batch `08`:

1. regenerate Doc 24 §"What Is Built" from real paths + LOC (pull from F's entries and SOURCE-INDEX.md)
2. add §"Currently-shipping demo contracts" subsection pointing at `contracts/src/*.sol` with a one-liner each
3. add the explicit callout that the 7 existing Solidity files are not the 6 planned Korai contracts, and decide whether the two sets will converge

### A.03 / A.04 / A.05-A.08 / A.11-A.13 / A.16 — Frontier Banner Pass

All of these are `Design — Phase 2+`. Apply a single banner tag to each
subsection; do not expand into any code work.

### A.01 / A.14 / A.15 — Already Accurate

Leave alone. The framing + Tier-6 dependency claims hold.

Acceptance criteria for this section:

- Doc 24 reads as an accurate audit of both the Rust chain code and the existing `contracts/` Solidity,
- readers can tell from Doc 00 / 01 / 02 that nearly everything not in F is Phase 2+,
- later batches can stop re-verifying the "is the KORAI token built yet" question — the answer is captured here.
