# E — Chain-Domain Safety (Docs 10, 11, 12, 13)

Parity of four chain-specific safety chapters: MEV protection, temporal
logic (LTL Büchi automata, CTL plan verification, 40 DeFi patterns),
Witness DAG (BLAKE3 content-addressed DAG, ZK proofs, SQLite storage),
formal verification pipeline (Heimdall-rs / Slither / Echidna / hevm /
Certora / Kontrol).

Almost all of Section E is **Tier-6 chain-deferred** per batch 08. The
chain layer itself is mostly Phase 2+, so chain-specific safety work
follows. A few shipping cross-references: `WalletGate` + `TxSimGate`
from batch 08 F.09-F.10 are the shipping chain-safety gates; the
`ChainWitnessEngine` (batch 08 F.08) is the attestation anchor.

Generated: 2026-04-16.

---

## E.01 — MEV taxonomy + detection (Doc 10 §"MEV Taxonomy", §"Detection Algorithms")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Doc 10 enumerates MEV types (sandwich, front-run, back-run, JIT, arbitrage) with detection algorithms and protection strategies.
**Reality**: `Grep 'MEV\|sandwich_attack\|frontrun\|backrun' crates/ --include=*.rs` returns zero matches. Cross-ref batch 08 — MEV protection is Tier-6 chain deferred. The closest shipping surface is `TxSimGate` at `roko-chain/src/gate/tx_sim_gate.rs:1-448` (batch 08 F.10) which simulates transactions — it could be extended with MEV detection heuristics when the chain layer activates.

---

## E.02 — MEV as intelligence signal + Gate pipeline integration (Doc 10 §"MEV as Intelligence Signal")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: MEV detection outputs feed the Gate pipeline + learning system.
**Reality**: Frontier. `TxSimGate` can hold an MEV-detector plugin once implemented; currently no MEV-aware verdict in the gate pipeline.

---

## E.03 — LTL Büchi automata runtime monitoring (Doc 11 §"LTL Büchi Automata")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Doc 11 (1,157 lines) describes Linear Temporal Logic with Büchi automata for runtime monitoring, plus CTL for pre-execution plan verification.
**Reality**: `Grep 'LTL\|Buchi\|temporal_logic\|TemporalMonitor' crates/ --include=*.rs` returns zero matches. Pure frontier. The academic design is extensive (safety / liveness / fairness properties, 40 DeFi patterns, category-theoretic composition, past-time LTL, 3-tier temporal attack detection) but none of it ships.

---

## E.04 — TemporalMonitor as Policy (Doc 11 §"TemporalMonitor as Policy")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: `TemporalMonitor` implements `Policy` — integrates LTL monitoring into the conductor watcher ensemble.
**Reality**: Absent. The shipping watcher ensemble has 10 watchers (batch 07 A.03); none are TemporalMonitor. Frontier.

---

## E.05 — 40 DeFi temporal patterns + boiling-frog / slow-escalation detectors (Doc 11 §"40 DeFi Temporal Patterns")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Doc 11 tables 40 DeFi-specific temporal patterns + boiling-frog / slow-escalation detectors.
**Reality**: Informational taxonomy. No pattern library exists.

---

## E.06 — 11 code-agent + 3 multi-agent temporal patterns (Doc 11 §"Extended Temporal Pattern Library")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Doc 11's 2025-04 enhancement adds 11 code-agent + 3 multi-agent temporal patterns.
**Reality**: Informational enhancement.

---

## E.07 — BLAKE3 content-addressed Witness DAG (Doc 12 §"BLAKE3 content-addressed DAG")

**Status**: PARTIAL
**Severity**: LOW
**Doc claim**: Doc 12 (1,544 lines) specifies a BLAKE3-addressed DAG with 5 vertex types (Observation, Prediction, Decision, Resolution, NeuroEntry), ZK proofs, SQLite storage, on-chain anchoring.
**Reality**: `roko-core::ContentHash` provides the BLAKE3-shaped primitive (cross-ref B.02). `Engram` carries parent hashes (batch 08 F.08 + B.02). The DAG shape is present implicitly via Engram lineage. What does NOT ship:
- Explicit 5-vertex-type enum (`Observation | Prediction | Decision | Resolution | NeuroEntry`)
- SQLite-backed DAG storage (Engrams use JSONL via `roko-fs::FileSubstrate`)
- DAG query language (6 query types)
- Datalog provenance queries via Datafrog
- ZK proofs for strategy auditing
- Safety-specific query patterns (TOCTOU, escalation chain, exfiltration, circular reasoning)
**Fix sketch**: Doc 12 should mark itself `Implementation: Partial (content-addressing + lineage ship via Engram; explicit vertex types + SQLite + Datalog + ZK frontier)`.

---

## E.08 — Five vertex types + ZK proofs + Datalog (Doc 12 §"Five Vertex Types", §"Datalog")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Five-vertex enum + ZK proof commitments + Datalog provenance queries.
**Reality**: Follows from E.07 — frontier.

---

## E.09 — Five-stage formal verification pipeline (Doc 13 §"Five-Stage Verification Pipeline")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Doc 13 (1,310 lines) chains Heimdall-rs → Slither → Echidna → hevm → Certora/Kontrol for chain contracts.
**Reality**: `Grep 'Heimdall\|Slither\|Echidna\|hevm\|Certora\|Kontrol' crates/ --include=*.rs` returns zero matches. Tier-6 chain deferred. The closest shipping formal-verification-adjacent surface is `TxSimGate` (batch 08 F.10) which runs pre-flight simulation — a lightweight analogue to hevm symbolic execution. Full pipeline integration is frontier.

---

## E.10 — 17 host-agent + 14 task-lifecycle verification properties (Doc 13 §"Verification-Guided Agent Design")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Doc 13 §"Verification-Guided Agent Design" enumerates 17 host-agent + 14 task-lifecycle properties for formal verification.
**Reality**: Informational property catalogs. No formal-verification framework to check them against.

---

## E.11 — Tool behavioral contracts (pre/post/invariant) (Doc 13 §"Tool Behavioral Contracts")

**Status**: PARTIAL
**Severity**: LOW
**Doc claim**: Tools declare pre-conditions, post-conditions, invariants.
**Reality**: `crates/roko-agent/src/safety/contract.rs:1-173` ships a contract module. Details not verified, but the scaffold for tool contracts exists at real LOC.
**Fix sketch**: Read `contract.rs` to confirm its shape; cite in Doc 13.

---

## E.12 — VeriGuard dual-stage verification + ContractEnforcingDispatcher (Doc 13 §"VeriGuard", §"ContractEnforcingDispatcher")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: VeriGuard does static + dynamic verification; ContractEnforcingDispatcher enforces tool contracts at runtime.
**Reality**: `Grep 'VeriGuard\|ContractEnforcingDispatcher' crates/ --include=*.rs` returns zero matches. The shipping `ToolDispatcher` with the 7-stage pipeline (Doc 16) covers some dispatcher-level enforcement but is not the VeriGuard/ContractEnforcingDispatcher design specifically.

---

## Section Summary

| Status | Count |
|--------|-------|
| DONE | 0 |
| PARTIAL | 2 (E.07 Engram lineage + ContentHash ship, E.11 contract.rs module shell exists) |
| NOT DONE | 10 (E.01-E.06, E.08-E.10, E.12) |

Section E is **almost entirely Tier-6 chain-deferred frontier**. It
is 5,042 lines of academic specification (MEV, LTL, Büchi, DeFi
patterns, Witness DAG queries, ZK proofs, Heimdall/Slither/Echidna/
hevm/Certora/Kontrol pipeline) that has no corresponding
implementation in this repo. The chain layer itself is Tier-6
(batch 08), so chain-specific safety is correspondingly frontier.

## Agent Execution Notes

### E.07 — Partial via Engram lineage

Doc 12 is closer to partial than pure frontier because Engram
lineage + ContentHash + on-chain witnessing all ship. The formal
Witness DAG with explicit vertex types + SQLite + Datalog queries
is the add-on that does not ship.

### E.11 — contract.rs exists

Worth a read pass in K5 to confirm what `contract.rs` actually does.

### All other E — frontier banner pass

Apply `Design — Phase 2+ Tier 6` banners to Docs 10-13. These are
all bounded by the chain layer's Tier-6 status.

Acceptance criteria:

- Docs 10-13 carry Phase 2+ Tier 6 banners,
- Doc 12 §"Witness DAG" cross-links to shipping Engram lineage as the minimal precursor,
- `contract.rs` read-through results noted.
