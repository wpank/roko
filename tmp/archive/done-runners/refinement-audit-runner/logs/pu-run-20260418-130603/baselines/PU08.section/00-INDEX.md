# 08-Chain Parity Analysis

Gap analysis of `docs/08-chain/` against the current Rust chain stack (`crates/roko-chain/`, `crates/roko-primitives/`), the mirage EVM/simulator surfaces (`apps/mirage-rs/`), the long-running observer (`apps/roko-chain-watcher/`), the demo consumer (`crates/roko-demo/`), and the shipping Solidity demo contracts at `contracts/src/`.

Generated: 2026-04-16

---

## How To Use This Batch

This batch should be treated as **Tier-6 doc honesty + scaffold inventory + status-contract cleanup**, not as a license to start implementing Korai v1 contracts, the gossip mesh, auction solvers, payment channels, privacy tiers, or on-chain markets.

- Distinguish four surfaces clearly:
  - **shipping Rust chain primitives**: `ChainClient` / `ChainWallet`, `AlloyChainClient`, `WalletGate`, `TxSimGate`, `ChainWitnessEngine`, `roko_bridge`, `roko-chain-watcher`
  - **shipping Solidity demo contracts**: the 7 files under `contracts/src/`
  - **shipping-but-different mirage chain scaffold**: `apps/mirage-rs/src/chain/*`
  - **honest Phase 2+ frontier**: Korai node, gossip mesh, marketplace v1, KORAI token, x402, ISFR solver, Valhalla, privacy, futures
- Prefer doc status accuracy, cross-links, and banner hygiene over new code.
- If a task starts requiring actual Solidity, libp2p, QP solver, TEE, ZK, or chain-runtime implementation, record the seam and stop.
- Every batch should be able to stop with a clear `PASS`, `FAIL`, or `BLOCKED` result and leave behind evidence: files changed, commands run, outputs, and explicit deferrals.

Recommended single-agent serial order inside batch `08`:

`K1 -> K2 -> K3 -> K5 -> K6 -> K4 -> K7 -> K8`

Reasoning:

- `K1` and `K2` lock in the real shipped surfaces first.
- `K3` maps the largest ambiguous scaffold before broader status rewrites.
- `K5` and `K6` resolve the two most misleading partial/adjacent surfaces after that.
- `K4` is a pure frontier/banner pass once the shipping precursor story is already explicit.
- `K7` regenerates Doc 24 from the earlier findings.
- `K8` is the final global banner/housekeeping pass.

---

## Document Index

| File | Docs Covered | Items | Status |
|------|--------------|-------|--------|
| [A-vision-and-chain-spec.md](A-vision-and-chain-spec.md) | 00, 01, 02, 24 | A.01-A.16 | 3 DONE / 3 PARTIAL / 10 NOT DONE |
| [B-identity-and-on-chain-trust.md](B-identity-and-on-chain-trust.md) | 03, 04, 05, 06 | B.01-B.13 | 2 DONE / 0 PARTIAL / 11 NOT DONE |
| [C-gossip-and-p2p-network.md](C-gossip-and-p2p-network.md) | 07, 08, 09 | C.01-C.08 | 1 DONE / 0 PARTIAL / 7 NOT DONE |
| [D-job-market-and-reputation.md](D-job-market-and-reputation.md) | 10, 11, 12, 13, 14 | D.01-D.11 | 1 DONE / 4 PARTIAL / 6 NOT DONE |
| [E-witness-triage-heartbeat.md](E-witness-triage-heartbeat.md) | 15, 16, 19 | E.01-E.13 | 3 DONE / 3 PARTIAL / 7 NOT DONE |
| [F-built-foundation.md](F-built-foundation.md) | 17, 18 | F.01-F.20 | 13 DONE / 5 PARTIAL / 2 NOT DONE |
| [G-payments-settlement-privacy.md](G-payments-settlement-privacy.md) | 20, 21, 22, 23 | G.01-G.11 | 0 DONE / 1 PARTIAL / 10 NOT DONE |
| [BATCHES.md](BATCHES.md) | — | 8 batches | Execution contract |
| [SOURCE-INDEX.md](SOURCE-INDEX.md) | — | Verified code anchors | Reference |
| [run-docs-parity.sh](run-docs-parity.sh) | — | Batch runner | Launcher |

Doc `INDEX.md` is absorbed into this file.

---

## Overall Parity: 23/92 items DONE (25%)

Topic `08` is in the posture the broader priority list already implies:

- **Tier 6 deferred**
- **mostly design**
- but **not empty**

The main problem is not missing code alone. It is that the docs currently blur:

- real Rust chain primitives,
- real but partial Solidity demos,
- a real mirage scaffold that is not the same thing as the Korai spec,
- and a large amount of honest future work.

### Tier 1 — Should Exist Now (self-hosting relevant)

None.

Nothing in topic `08` blocks the self-hosting loop directly. The highest-value work here is status clarity, not chain implementation.

### Tier 2 — Should Exist Soon (status and doc honesty)

| ID | Title | Status | Severity |
|----|-------|--------|----------|
| A.09 | Doc 24 undercounts the real shipping chain surface | PARTIAL | HIGH |
| A.10 | 7 shipping Solidity demo contracts exist but the status doc effectively hides them | PARTIAL | HIGH |
| F.12 | mirage chain scaffold is real but mis-scoped relative to Doc 18 | PARTIAL | HIGH |
| G.04 | Doc 21 “Built” framing overstates a proxy-only shipping surface | PARTIAL | HIGH |
| F.07 | `AlloyChainClient` ships but Doc 24 still treats it as not built | DONE (doc stale) | MEDIUM |
| F.09 | `WalletGate` ships but Doc 24 still treats it as a stub | DONE (doc stale) | MEDIUM |
| F.10 | `TxSimGate` ships but Doc 24 still treats it as a stub | DONE (doc stale) | MEDIUM |
| F.13 | `roko_bridge` trait impls ship but Doc 24 still treats them as not built | DONE (doc stale) | MEDIUM |
| F.14 | `MirageChainClient` really is absent and should stay the explicit remaining integration gap | NOT DONE | MEDIUM |
| D.01 | `BountyMarket.sol` is a real partial marketplace precursor, not “nothing” | PARTIAL | MEDIUM |
| D.05 | `WorkerRegistry.sol` is a real partial reputation precursor, not the 7-domain design | PARTIAL | MEDIUM |
| E.13 | `ChainWitnessEngine` naming collision remains easy to misread without cross-links | DONE (docs unclear) | LOW |

### Tier 3 — Future / Phase 2+ Frontier

| ID | Title | Status | Severity |
|----|-------|--------|----------|
| A.03-A.08, A.11-A.13, A.16 | Korai node, token economics, registry contracts, canonical addresses | NOT DONE | LOW |
| B.03-B.13 | on-chain identity, Passport, Ventriloquist, TEE, ERC-8004 registries | NOT DONE | LOW |
| C.01-C.07 | gossip mesh, topics, peer scoring, Sybil resistance | NOT DONE | LOW |
| D.02-D.04, D.07-D.09 | Sparrow, hiring models, Vickrey, gaming resistance, C-factor, fee split | NOT DONE | LOW |
| E.01-E.03, E.05, E.07-E.10 | Binary Fuse witness engine, MIDAS-R, curiosity scoring, PolicyCage | NOT DONE | LOW |
| G.01-G.11 | x402, ISFR solver, KKT, Valhalla, privacy tiers, futures | NOT DONE | LOW |

### Already Shipped

| ID | Title | Status |
|----|-------|--------|
| F.01-F.11, F.13, F.16, F.19-F.20 | `roko-chain`, alloy backend, gates, witness anchoring, mirage simulator, bridge impls, watcher app, demo consumer | DONE |
| B.01-B.02 | canonical 10,240-bit HDC primitive plus mirage wrappers | DONE |
| A.01, A.14, A.15 | domain-plugin framing and Tier-6 implementation-plan/dependency framing | DONE |
| C.08 | local `InsightBus` / `PheromoneBus` pub/sub precursor | DONE |
| D.11 | `ConsortiumValidator.sol` exists and is tested, though under-documented by the PRDs | DONE |
| E.04, E.12, E.13 | chain watcher, REFLECT path, attestation witness anchoring | DONE |

---

## Execution Boundaries

These are valid findings, but they should usually be handled outside batch `08`:

| Item | Better Home | Why |
|------|-------------|-----|
| Korai v1 Solidity contracts | post-self-hosting Tier-6 execution pass | not doc-audit work |
| libp2p / iroh / GossipSub | later P2P pass | no runtime owner yet |
| ISFR solver + KKT verifier | later settlement pass | current repo only ships a proxy |
| TEE / ZK / Binius privacy stack | later privacy pass | Phase 2+ only |
| `MirageChainClient` implementation | later mirage integration pass | real code gap, but not status-audit critical path |
| renaming `ChainWitnessEngine` in Rust | optional cleanup pass | docs can disambiguate first |
| demo-contract migration to Korai v1 | later Solidity pass | not a doc-parity batch job |

Batch `08` should usually produce:

- an accurate Doc 24,
- explicit acknowledgment of shipping demo contracts,
- a canonical inventory of the mirage chain scaffold,
- honest frontier banners on the major theory docs,
- a clear answer to the `ChainWitnessEngine` naming confusion,
- and no accidental drift into chain implementation work.

---

## Critical Chain-Layer Issues

1. **Doc 24 is the canonical status doc, but it still understates shipping code.**
2. **The shipping Solidity demo surface is real, tested, and still mostly invisible from the PRDs.**
3. **The mirage `chain/` scaffold is real, but the docs still imply it is a narrower Korai-registry emulation surface than it actually is.**
4. **Doc 21’s top-level framing is still the most misleading banner in topic `08`.**
5. **`ChainWitnessEngine` currently names two semantically different surfaces in a way that invites confusion.**

---

## Key Insight

The chain batch does **not** mainly need new code.

It needs a calibrated separation between:

- the **shipped Rust chain primitives**,
- the **shipping Solidity demos**,
- the **shipping-but-different mirage scaffold**,
- and the **honest Phase 2+ frontier**.

That means the highest-value work here is usually:

1. make the shipped surfaces visible,
2. make the partial/demo surfaces legible as partial/demo,
3. make the frontier surfaces unmistakably frontier,
4. leave later agents with one obvious status document and one obvious source index.

---

## Batch 08 Success Definition

Batch `08` is successful when:

- later agents can tell, from the first screen, whether a chain PRD is shipping, partial, or frontier,
- Doc 24 is accurate enough to use as the canonical status summary,
- the 7 demo Solidity contracts are visible and correctly scoped,
- the mirage scaffold has one honest inventory,
- Doc 21 no longer reads like the ISFR solver is in this repo,
- and `BATCHES.md` plus the context pack are sufficient for an unattended overnight docs pass.
