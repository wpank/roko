# 08-Chain Parity Refresh

Audit-aligned refresh of `docs/08-chain/` parity materials.

Generated: 2026-04-18

---

## Post-Audit Posture

Topic `08` is now a **doc-honesty / Phase 2+** pack, not a chain-delivery plan.

Current posture for this section:

- The shipped chain story is **narrow**: `ChainClient`, `ChainWallet`, `AlloyChainClient`, `WalletGate`, `TxSimGate`, `ChainWitnessEngine`, the minimal witness helpers around it, and the Solidity demo contracts.
- The Solidity contracts are **real demo precursors**, not proof that Korai v1 is implemented.
- Anything that reads like a Korai node, KORAI token, full ERC-8004 suite, gossip mesh, settlement solver, privacy runtime, or futures market should be treated as **DEFERRED / Phase 2+**.
- This pack should stop using adjacent repo surfaces to imply more chain delivery than actually ships.

---

## Section Index

| File | Docs Covered | Audit Posture | What Changed |
|---|---|---|---|
| [A-vision-and-chain-spec.md](A-vision-and-chain-spec.md) | 00, 01, 02, 24 | `rewrite` + `defer` | Keep only the small shipped precursor story; mark Korai chain, token, and planned contract suite as Phase 2+ |
| [B-identity-and-on-chain-trust.md](B-identity-and-on-chain-trust.md) | 03, 04, 05, 06 | `defer` | Keep `AgentRegistry.sol` as the visible demo precursor; defer the rest of the identity / registry stack |
| [C-gossip-and-p2p-network.md](C-gossip-and-p2p-network.md) | 07, 08, 09 | `defer` | No shipped gossip runtime belongs in the core parity story here |
| [D-job-market-and-reputation.md](D-job-market-and-reputation.md) | 10, 11, 12, 13, 14 | `defer` | Keep demo contracts visible; do not describe marketplace / reputation theory as near-term runtime |
| [E-witness-triage-heartbeat.md](E-witness-triage-heartbeat.md) | 15, 16, 19 | `narrow` | Separate the shipped attestation witness helper from the larger deferred witness-observer design |
| [F-built-foundation.md](F-built-foundation.md) | 17, 18 | `rewrite` | Treat the foundation as the real traits/backend/gates/witness helper plus demo contracts, not as a broader chain-runtime inventory |
| [G-payments-settlement-privacy.md](G-payments-settlement-privacy.md) | 20, 21, 22, 23 | `defer` | Settlement / privacy / futures remain Phase 2+; do not use proxy or theory surfaces as built-chain evidence |
| [BATCHES.md](BATCHES.md) | — | `rewrite` | Docs-only PU08 execution contract with most items deferred |
| [SOURCE-INDEX.md](SOURCE-INDEX.md) | — | `rewrite` | Source anchors reduced to the shipped surfaces this pack is allowed to lean on |

---

## Gap Picture After The Audit

### What Is Shipped

- `ChainClient` and `ChainWallet` define the live narrow chain interface.
- `AlloyChainClient` is a real JSON-RPC backend.
- `WalletGate` and `TxSimGate` are real gate surfaces for preflight checks.
- `ChainWitnessEngine` is a real attestation witness helper.
- The witness path has real minimal primitives: static witness marker/topic/target plus request/verify helpers.
- A seven-contract demo suite is clearly wired into the current demo path, and
  `contracts/src/` also contains additional registry-oriented Solidity files.

### What Must Be Treated As Deferred

- Korai node/runtime
- canonical Korai predeploy set
- KORAI token and demurrage runtime
- Passport / full ERC-8004 registry suite
- gossip / p2p mesh
- market-clearing / ISFR / KKT settlement logic
- privacy tiers, TEE / ZK execution, knowledge futures

---

## Working Rules For This Pack

1. Use present tense only for the shipped surfaces listed above.
2. Call the Solidity contracts `demo`, `precursor`, or `partial` surfaces, not Korai v1.
3. Treat `ChainWitnessEngine` as an attestation anchor helper only.
4. If a claim needs a broader chain runtime to be true, mark it `DEFERRED` instead of widening the shipped story.
5. Keep `SOURCE-INDEX.md` limited to the same narrow anchor set.

---

## Success Definition

This parity pack is correct when:

- a reader immediately sees that topic `08` is mostly Phase 2+,
- the shipped chain story is limited to the core Rust traits/backend/gates/witness helper plus the Solidity demos,
- the witness docs do not blur the attestation helper with the frontier observer design,
- and the source index no longer uses wider repo surfaces to overstate chain delivery.
