# B — Audit Chain and Taint Tracking (Docs 02, 03)

Parity of the two provenance-layer chapters: audit chain (Merkle
hash-chain of AuditEntry) and taint tracking (TaintLabel + data
sink flow matrix).

Both subsystems ship as real implementations in
`crates/roko-orchestrator/src/safety/`. The shipping AuditChain uses
a custom canonical-encoding hash (not directly SHA-256/BLAKE3 as Doc
02 describes, but functionally equivalent). TaintTracker ships a
simpler version than Doc 03's full Denning lattice / SecurityLabel
design — but the core `is_tainted / propagate / mark_tainted` flow
is real and consumes `ContentHash` from `roko-core`.

Generated: 2026-04-16.

---

## B.01 — AuditChain append-only hash-chain ships (Doc 02 §"SHA-256 / BLAKE3 Merkle hash-chain")

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 02 §"SHA-256 / BLAKE3 Merkle hash-chain" specifies an append-only chain of audit entries with content-addressed linking.
**Reality**: `crates/roko-orchestrator/src/safety/audit_chain.rs:1-565` ships:
- `AuditEntry { prev_hash: [u8; 32], kind, actor, resource, ts_ms, signature: Option<String> }` at `:37-53`
- `AuditEntry::new(prev_hash, kind, actor, resource)` builder at `:62-76`
- `AuditEntry::with_signature(signature)` at `:82-85`
- `content_hash()` at `:91+` — canonical byte encoding with `auditv1|` prefix + field tags + length-prefixed bodies (hand-rolled for stability across serde versions)
- `AuditChain` at `:136+` manages the chain (append / tip / verify)

The hash algorithm is custom canonical-encoding, not literally BLAKE3 — but the tamper-evident hash-chain property is preserved. `prev_hash: [u8; 32]` is BLAKE3-shaped.
**Fix sketch**: Doc 02 should clarify the shipping hash algorithm is custom canonical encoding (for serde-version stability), with hash-length equivalent to SHA-256 / BLAKE3.

---

## B.02 — Engram lineage DAG via `ContentHash` (Doc 02 §"Engram Lineage DAG")

**Status**: DONE
**Severity**: —
**Doc claim**: Every Engram carries its lineage via `ContentHash` references to parents.
**Reality**: `roko-core/src/provenance.rs` + `engram.rs` define provenance tracking (per grep earlier). `roko-core::ContentHash` is the canonical content-addressing primitive (imported in audit_chain.rs:31, taint_propagation.rs:30). Engram lineage is a core-crate invariant.

---

## B.03 — AuditSink trait + FileSubstrate persistence (Doc 02 §"AuditSink Trait", §"FileSubstrate Persistence")

**Status**: PARTIAL
**Severity**: LOW
**Doc claim**: An `AuditSink` trait allows pluggable persistence backends; `FileSubstrate` persists audit entries to JSONL.
**Reality**: `AuditChain` ships in-memory (`Mutex<Vec<AuditEntry>>` per `audit_chain.rs:30+`). Doc 16 §"Stage-level audit emit" mentions the dispatcher emits audit Engrams via `emit_audit()`. Whether those Engrams persist through `roko-fs::FileSubstrate` to `.roko/audit.jsonl` or similar is unverified; the plumbing exists but verification requires deeper reading.
**Fix sketch**: Doc 02 §"AuditSink Trait" should cite the shipping `AuditChain::append` path and flag the sink-to-FileSubstrate wiring as partial/unverified.

---

## B.04 — On-chain anchoring of audit entries (Doc 02 §"On-chain anchoring")

**Status**: PARTIAL
**Severity**: LOW
**Doc claim**: Audit entries can be anchored on the Korai chain for tamper-evidence at the inter-organization trust boundary.
**Reality**: Cross-ref batch 08 F.08 — `crates/roko-chain/src/witness.rs::ChainWitnessEngine` ships attestation anchoring via `witness_on_chain(attestation, wallet, client)`. This is the on-chain anchor primitive. It is not yet wired from `AuditChain` → `witness_on_chain` directly; the attestation-anchor path exists as a separate witness surface. Infrastructure ready; specific audit-anchor integration frontier.
**Fix sketch**: Doc 02 should cite `ChainWitnessEngine` as the anchoring primitive and note that audit-specific anchoring (periodic audit-chain root → on-chain witness) is a wiring step.

---

## B.05 — TaintTracker with mark/propagate/is_tainted ships (Doc 03 §"TaintLabel enum", §"TaintedString with zeroize")

**Status**: DONE (simpler than full design)
**Severity**: LOW
**Doc claim**: Doc 03 specifies `TaintLabel` enum + `TaintedString` with zeroize-on-drop. Full taint propagation algebra (Denning lattice, `SecurityLabel { confidentiality, integrity }`, join operator), FIDES integration, RTBAS dynamic taint, Prompt Flow Integrity (PFI), PCAS Datalog policy language.
**Reality**: `crates/roko-orchestrator/src/safety/taint_propagation.rs:1-409` ships:
- `TaintReason { category: String, detail: String }` at `:39-45` (with `external`, `user_input`, `propagated` constructors at `:56-70`)
- `TaintTracker { inner: Mutex<HashMap<ContentHash, TaintReason>> }` at `:92-94`
- `mark_tainted(hash, reason)` at `:105-107`
- `is_tainted(hash)` at `:111-113`
- `propagate(parents, child)` at `:85+` — child becomes tainted if any parent is tainted
- `reason(hash)` at `:117-119`
- Thread-safe via `parking_lot::Mutex`

This is a **simpler but functional** taint tracker: it is boolean-tainted + reason annotation, not the full Denning lattice with `SecurityLabel { confidentiality, integrity }` + join operator. The shipping design is sufficient for operational refuse-at-sink semantics (`is_tainted` check before git commit / network egress / signal emit per module doc at `:1-7`).
**Fix sketch**: Doc 03 §"Taint propagation algebra" should mark the full Denning lattice as `Design — Phase 2+`. Cite `TaintTracker` as the shipping minimal taint surface. Note that `TaintedString with zeroize` is separate from `TaintTracker` — `ScrubPolicy` in `roko-agent/src/safety/scrub.rs` handles the zeroize-style secret scrubbing (see C.05).

---

## B.06 — 4-stage ingestion pipeline (Doc 03 §"4-stage ingestion pipeline")

**Status**: PARTIAL
**Severity**: LOW
**Doc claim**: Doc 03 describes a 4-stage ingestion pipeline: canonicalize → classify → tag → tag-propagate.
**Reality**: The 7-stage ToolDispatcher pipeline (Doc 16 §"The dispatch method runs a 7-stage pipeline") covers a different axis: validate → filter → permission → safety pre-exec → handler → truncate → safety scrub. The 4-stage *ingestion* pipeline Doc 03 describes is a separate flow for external inputs. Partially covered by the broader dispatcher checks; specific ingest-classify-tag flow is frontier.

---

## B.07 — Bloom Oracle, causal rollback (Doc 03 §"Bloom Oracle", §"Causal Rollback")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Bloom Oracle as a probabilistic membership test for taint; causal rollback to unwind tainted propagation.
**Reality**: `Grep 'BloomOracle\|bloom_oracle\|causal_rollback' crates/ --include=*.rs` returns zero matches. Frontier.

---

## B.08 — FIDES, RTBAS, PFI, PCAS Datalog are frontier (Doc 03 §"FIDES integration", §"RTBAS dynamic taint tracking", §"Prompt Flow Integrity", §"PCAS Datalog")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Doc 03's 2025-04 enhancement adds FIDES, RTBAS, PFI, and PCAS Datalog policy language.
**Reality**: `Grep 'FIDES\|RTBAS\|PFI\|PCAS\|Datalog' crates/ --include=*.rs` returns zero matches. These are academic enhancements introduced in the enhancement pass; all frontier.

---

## B.09 — DataSink flow matrix (Doc 03 §"DataSink Flow Matrix")

**Status**: PARTIAL
**Severity**: LOW
**Doc claim**: Matrix of tainted-data behaviors per sink (git / network / signal / exec refuse tainted; log / audit accept tainted with warning).
**Reality**: The module doc at `taint_propagation.rs:1-7` explicitly cites the refuse-at-sink semantics: "Sinks that need to refuse tainted data (git commits, network egress, signal emits) consult `TaintTracker::is_tainted` before proceeding." The `is_tainted` check primitive ships; whether it is called from every sink (git / network / signal emit) in the dispatcher pipeline is unverified.
**Fix sketch**: Verify via grep whether `is_tainted` is called from `roko-agent/src/safety/{git, network}.rs`. If so, update Doc 03 to cite those call sites.

---

## Section Summary

| Status | Count |
|--------|-------|
| DONE | 3 (B.01 AuditChain, B.02 ContentHash lineage, B.05 TaintTracker) |
| PARTIAL | 4 (B.03 AuditSink/FileSubstrate wiring, B.04 on-chain anchoring, B.06 4-stage ingest, B.09 DataSink flow matrix) |
| NOT DONE | 2 (B.07 Bloom Oracle, B.08 FIDES/RTBAS/PFI/PCAS) |

Section B has **two substantial shipping surfaces** (AuditChain at
565 LOC, TaintTracker at 409 LOC) that are **not flagged as shipping
by Doc 16**. Doc 16's focus is the 6-guard `SafetyLayer` in the
agent crate; it does not enumerate the orchestrator-layer audit +
taint + capability + loop-guard + permit modules.

## Agent Execution Notes

### B.01 / B.05 — Regenerate Doc 02 / Doc 03 status

Doc 02 should cite `roko-orchestrator/src/safety/audit_chain.rs:1-565`
as the shipping AuditChain. Doc 03 should cite
`roko-orchestrator/src/safety/taint_propagation.rs:1-409` as the
shipping TaintTracker.

### B.09 — DataSink refuse-at-sink verification

One-line grep verifies whether `is_tainted` is actually called at
git / network / signal sinks. Worth doing in K3 (runtime guards).

Acceptance criteria:

- Doc 02 cites shipping AuditChain with hash-algorithm note,
- Doc 03 cites shipping TaintTracker + marks full Denning / FIDES / PCAS frontier,
- cross-link B.04 to `ChainWitnessEngine` as anchoring primitive.
