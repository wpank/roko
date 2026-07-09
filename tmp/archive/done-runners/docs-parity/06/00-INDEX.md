# 06-Neuro Parity Refresh

Post-audit refresh for `docs/06-neuro/`.

Generated: 2026-04-18

## Scope

This PU06 pass is a **docs-only parity refresh** for the neuro topic. It is not
a license to implement the larger memory roadmap described across docs `06`.

The audit conclusion is simple:

- `roko-neuro` is already a real subsystem with **7 source files** covering
  storage, distillation, tier progression, HDC encoding, and context assembly.
- `HdcVector` is already real in `roko-primitives` as a **345 LOC**, tested,
  10,240-bit implementation.
- the highest-value next neuro change is still **adding an HDC fingerprint field
  to `Engram`** in `roko-core/src/engram.rs`.
- cross-domain resonance, Library of Babel exchange, demurrage economics,
  worldview organization, and mesh backup/publish flows are **not** shipped and
  must be labeled as deferred or target-state.

## Ship / Partial / Deferred

### Shipping

- `KnowledgeEntry`, `KnowledgeKind`, `KnowledgeTier`, and `KnowledgeStore`
- `Distiller` and `TierProgression`
- HDC primitives in `roko-primitives`
- HDC-backed encoding inside `roko-neuro`
- feature-gated HDC indexing on the knowledge store

### Partial

- `ContextAssembler` exists, but this batch treats its production wiring as a
  separate code-execution follow-up
- HDC is available in neuro and learning, but not yet universal on `Engram`
- query APIs exist in `roko-neuro`, but `Substrate` does **not** expose
  `query_similar()`

### Deferred / Target-State

- cross-domain resonance and analogy APIs from doc `08`
- demurrage / balance freshness as a memory model
- worldview clustering and worldview-aware storage
- somatic exchange network flows, Library of Babel, Korai / Lethe channels
- backup / restore / publish mesh workflows
- any separate `roko-hdc` crate

## Top Priorities

1. **HDC on Engram**: add a fingerprint field to `Engram` and make HDC a shared
   kernel-level capability instead of a neuro-only detail.
2. **Truth in docs**: keep `query_similar()` and cross-domain transfer labeled as
   not yet on `Substrate`.
3. **Narrow the frontier**: move demurrage, worldview, exchange, and backup
   systems into explicit future-work language.

## File Index

| File | Focus |
|------|-------|
| [A-knowledge-types-tiers-decay.md](A-knowledge-types-tiers-decay.md) | shipped knowledge kinds, tiers, and why demurrage stays deferred |
| [B-hdc-foundations-operations.md](B-hdc-foundations-operations.md) | real HDC substrate today and why `roko-hdc` is unnecessary |
| [C-query-crossdomain-context.md](C-query-crossdomain-context.md) | current query surface, `ContextAssembler`, and deferred cross-domain transfer |
| [D-distillation-progression.md](D-distillation-progression.md) | what distillation and tier progression actually ship |
| [E-somatic-exchange-backup.md](E-somatic-exchange-backup.md) | real somatic retrieval vs deferred exchange / backup stories |
| [F-status-frontier.md](F-status-frontier.md) | current frontier status after the audit |
| [BATCHES.md](BATCHES.md) | narrowed docs-refresh batches |
| [SOURCE-INDEX.md](SOURCE-INDEX.md) | fresh source anchors for the narrowed story |
| [AUDIT-LOG.md](AUDIT-LOG.md) | parity refresh log |

## Success Definition

PU06 is successful when:

- every parity file under `tmp/docs-parity/06/` describes current neuro/HDC
  reality in present tense,
- HDC-on-Engram is called out as the highest-value next step,
- `query_similar()` and cross-domain transfer are clearly marked as not yet on
  `Substrate`,
- and all exchange, demurrage, worldview, and backup claims are narrowed to
  deferred or target-state language.
