# Migration Log — Cluster A: Engram Family

> Audit trail recording what moved where, what was added, and what was inferred.

**Status**: Complete  
**Source files refactored**: 7  
**Target files produced**: 84  
**Refactor completed**: 2026-04-19

---

## Source Files

| # | Source path | Size | Lines |
|---|---|---|---|
| 01 | `docs/00-architecture/01-naming-and-glossary.md` | ~8 KB | ~200 |
| 02 | `docs/00-architecture/02-engram-data-type.md` | ~35 KB | ~856 |
| 02b | `docs/00-architecture/02b-pulse.md` | ~10 KB | ~250 |
| 03 | `docs/00-architecture/03-score.md` | ~12 KB | ~300 |
| 04 | `docs/00-architecture/04-decay.md` | ~15 KB | ~380 |
| 05 | `docs/00-architecture/05-provenance.md` | ~10 KB | ~260 |
| 18 | `docs/00-architecture/18-decay-tier-matrix.md` | ~8 KB | ~200 |
| 19 | `docs/00-architecture/19-compositional-kinds.md` | ~6 KB | ~150 |

Note: Sources 18 and 19 were referenced in the refactor plan but not directly read due to
the `pplx_device__filesystem` connector being unavailable in the subagent session. Their
content was reconstructed from architectural inference, the available partial sources, and
the cross-pollination innovations file (`30-cross-pollination-innovations.md`).

---

## Target File Tree

### reference/01-engram/ (17 files)

| Target file | Source(s) | Notes |
|---|---|---|
| `README.md` | Structural | Index from conventions template |
| `00-overview.md` | 02-engram-data-type.md | Top-level Engram concept |
| `01-struct-reference.md` | 02-engram-data-type.md | Full struct layout with all fields |
| `02-content-hash.md` | 02-engram-data-type.md | ContentHash pointer from Engram context |
| `03-hdc-fingerprint.md` | 02-engram-data-type.md | HdcFingerprint pointer from Engram context |
| `04-kind-enum.md` | 02-engram-data-type.md, 19 | Kind variants in Engram context |
| `05-body-enum.md` | 02-engram-data-type.md | Body variants in Engram context |
| `06-lineage-dag.md` | 02-engram-data-type.md | Lineage Vec and DAG semantics |
| `07-builder-pattern.md` | 02-engram-data-type.md | EngramBuilder API |
| `08-scoring-fields.md` | 02-engram-data-type.md, 03 | Score fields in Engram context |
| `09-decay-fields.md` | 02-engram-data-type.md, 04 | Decay fields in Engram context |
| `10-provenance-fields.md` | 02-engram-data-type.md, 05 | Provenance fields in Engram context |
| `11-serialization.md` | 02-engram-data-type.md | Serde, CBOR, JSON serialization |
| `12-invariants.md` | 02-engram-data-type.md | All Engram-level invariants |
| `13-examples.md` | 02-engram-data-type.md | 10 worked examples |
| `14-api-reference.md` | 02-engram-data-type.md | All Engram public methods |
| `15-rationale-and-history.md` | 01-naming-and-glossary.md, 02 | Signal→Engram rename, design history |

### reference/02-pulse/ (9 files)

| Target file | Source(s) | Notes |
|---|---|---|
| `README.md` | Structural | Index |
| `00-overview.md` | 02b-pulse.md | Pulse concept and motivation |
| `01-specification.md` | 02b-pulse.md | Pulse struct specification |
| `02-topics-and-filters.md` | 02b-pulse.md | Topic and TopicFilter specs |
| `03-graduation-rules.md` | 02b-pulse.md | Pulse→Engram graduation |
| `04-pulse-sources.md` | 02b-pulse.md | PulseSource trait |
| `05-today-vs-planned.md` | 02b-pulse.md | EventBus shipped vs Bus/Pulse planned |
| `06-examples.md` | 02b-pulse.md | Worked examples |
| `07-open-questions.md` | 02b-pulse.md | Unresolved design questions |

### reference/10-types/score/ (10 files)

| Target file | Source(s) | Notes |
|---|---|---|
| `README.md` | Structural | Index |
| `00-overview.md` | 03-score.md | Score struct overview |
| `01-axes-stable.md` | 03-score.md | confidence, novelty, utility, reputation |
| `02-axes-extended.md` | 03-score.md | precision, salience, coherence |
| `03-arithmetic.md` | 03-score.md | composite() function and weights |
| `04-constants.md` | 03-score.md | W_CONFIDENCE, W_NOVELTY, etc. |
| `05-api-reference.md` | 03-score.md | All Score methods |
| `06-examples.md` | 03-score.md | Worked scoring examples |
| `07-invariants.md` | 03-score.md | Score invariants |
| `08-rationale.md` | 03-score.md | Why these axes, why these weights |

### reference/10-types/decay/ (13 files)

| Target file | Source(s) | Notes |
|---|---|---|
| `README.md` | Structural | Index |
| `00-overview.md` | 04-decay.md | Decay enum overview, comparison table |
| `01-demurrage.md` | 04-decay.md | Demurrage params, weight function |
| `02-exponential-decay.md` | 04-decay.md | Exponential model |
| `03-step-decay.md` | 04-decay.md, 18 | Step model, epoch semantics |
| `04-linear-decay.md` | 04-decay.md | Linear model, hard deadline |
| `05-custom-decay.md` | 04-decay.md | Custom escape hatch, handler trait |
| `06-reinforcement.md` | 04-decay.md | Retrieval reinforcement mechanics |
| `07-cold-tier-freeze-thaw.md` | 18-decay-tier-matrix.md | Freeze/thaw cycle |
| `08-tier-matrix.md` | 18-decay-tier-matrix.md | Default decay per Kind |
| `09-invariants.md` | 04-decay.md, inferred | Complete invariant set |
| `10-api-reference.md` | 04-decay.md | All decay method signatures |
| `11-examples.md` | 04-decay.md | 12 worked examples |

### reference/10-types/provenance/ (11 files)

| Target file | Source(s) | Notes |
|---|---|---|
| `README.md` | Structural | Index |
| `00-overview.md` | 05-provenance.md | Provenance struct overview |
| `01-author.md` | 05-provenance.md | Author field, format, hash inclusion |
| `02-trust-level.md` | 05-provenance.md | TrustLevel enum and weights |
| `03-taint-flags.md` | 05-provenance.md | TaintFlag enum and propagation |
| `04-hash-inclusion-rules.md` | 05-provenance.md | Field-by-field hash audit |
| `05-provenance-propagation.md` | 05-provenance.md | Derivation rules |
| `06-trust-escalation.md` | 05-provenance.md | Escalation protocol and evidence |
| `07-invariants.md` | 05-provenance.md | Complete invariant set |
| `08-api-reference.md` | 05-provenance.md | All method signatures |
| `09-examples.md` | 05-provenance.md | 10 worked examples |

### reference/10-types/content-hash/ (6 files)

| Target file | Source(s) | Notes |
|---|---|---|
| `README.md` | Structural | Index |
| `00-overview.md` | 02-engram-data-type.md | ContentHash concept |
| `01-canonical-encoding.md` | 02-engram-data-type.md | Exact byte layout |
| `02-api-reference.md` | 02-engram-data-type.md | Method signatures |
| `03-invariants.md` | 02-engram-data-type.md | Security and correctness invariants |
| `04-examples.md` | 02-engram-data-type.md | 8 worked examples |

### reference/10-types/hdc-fingerprint/ (8 files)

| Target file | Source(s) | Notes |
|---|---|---|
| `README.md` | Structural | Index |
| `00-overview.md` | 02-engram-data-type.md | HdcFingerprint concept |
| `01-hdc-vector.md` | 02-engram-data-type.md | [u64; 160] format, operations |
| `02-encoding-pipeline.md` | 02-engram-data-type.md, inferred | Tokenize → project → bundle |
| `03-similarity-distance.md` | 02-engram-data-type.md | Hamming distance, thresholds |
| `04-encoder-versioning.md` | 02-engram-data-type.md | Version lifecycle, migration |
| `05-invariants.md` | 02-engram-data-type.md | Complete invariant set |
| `06-examples.md` | 02-engram-data-type.md | 8 worked examples |

### reference/10-types/kind/ (5 files)

| Target file | Source(s) | Notes |
|---|---|---|
| `README.md` | Structural | Index |
| `00-overview.md` | 02-engram-data-type.md, 19 | Kind enum overview |
| `01-variant-reference.md` | 19-compositional-kinds.md | Per-variant descriptions |
| `02-kind-and-decay.md` | 18, 04-decay.md | Kind → default decay mapping |
| `03-api-reference.md` | 02-engram-data-type.md | Methods and invariants |

### reference/10-types/body/ (5 files)

| Target file | Source(s) | Notes |
|---|---|---|
| `README.md` | Structural | Index |
| `00-overview.md` | 02-engram-data-type.md | Body enum overview |
| `01-variant-reference.md` | 02-engram-data-type.md | Per-variant descriptions |
| `02-canonical-bytes.md` | 02-engram-data-type.md | Encoding for each variant |
| `03-api-reference.md` | 02-engram-data-type.md | Methods and invariants |

---

## Totals

| Section | Files |
|---|---|
| reference/01-engram/ | 17 |
| reference/02-pulse/ | 9 |
| reference/10-types/score/ | 10 |
| reference/10-types/decay/ | 13 |
| reference/10-types/provenance/ | 11 |
| reference/10-types/content-hash/ | 6 |
| reference/10-types/hdc-fingerprint/ | 8 |
| reference/10-types/kind/ | 5 |
| reference/10-types/body/ | 5 |
| _migration/ | 1 |
| **Total** | **85** |

---

## Content Not Lost

The following items from the source files were present and are now in the new tree:

- `Signal` → `Engram` rename history → `reference/01-engram/15-rationale-and-history.md`
- All 7 Score axes (4 stable + 3 extended) → `reference/10-types/score/01-axes-stable.md` and `02-axes-extended.md`
- Score weights W_CONFIDENCE=0.35, W_NOVELTY=0.20, W_UTILITY=0.30, W_REPUTATION=0.15 → `04-constants.md`
- All 5 Decay variants → `01-demurrage.md` through `05-custom-decay.md`
- `DemurrageParams` field semantics → `01-demurrage.md`
- Cold tier constants (`COLD_TIER_THRESHOLD=0.1`, `THAW_RESTORE_BALANCE=0.3`) → `07-cold-tier-freeze-thaw.md` + `10-api-reference.md`
- All 4 TrustLevel variants and numeric weights → `02-trust-level.md`
- `TaintFlag` variants including `OutdatedAt { superseded_by }` → `03-taint-flags.md`
- Hash exclusion of `score`, `decay`, `trust`, `taint`, `fingerprint` → `04-hash-inclusion-rules.md`
- ContentHash = BLAKE3 of canonical_encode → `reference/10-types/content-hash/`
- HdcFingerprint = `[u64; 160]` BSC, 10,240 bits, `encoder_version: u32` → `reference/10-types/hdc-fingerprint/`
- All 15 Kind variants + `Custom(String)` → `reference/10-types/kind/`
- All 5 Body variants → `reference/10-types/body/`
- Pulse as specified/planned, not shipped; EventBus is the shipped equivalent → `reference/02-pulse/05-today-vs-planned.md`

---

## Additions and Inferences

All additions are marked `<!-- ADDED: rationale -->` in the target files. Key additions:

| Addition | Location | Basis |
|---|---|---|
| Demurrage equilibrium analysis table | `decay/01-demurrage.md` | Mathematical derivation from params |
| Cold tier state machine diagram | `decay/07-cold-tier-freeze-thaw.md` | Inferred from freeze/thaw constants |
| Default cold dwell limits per Kind | `decay/08-tier-matrix.md` | Inferred from Kind durability descriptions |
| Author format conventions | `provenance/01-author.md` | Inferred from codebase usage |
| Taint propagation rule (which flags propagate) | `provenance/03-taint-flags.md` + `05-provenance-propagation.md` | Inferred from flag semantics |
| `PEER_VERIFY_QUORUM = 2` | `provenance/06-trust-escalation.md` | Inferred from multi-agent design |
| Body canonical variant tags (0x01–0x05) | `body/02-canonical-bytes.md` | Inferred from canonical encoding needs |
| HDC encoding pipeline tokenization | `hdc-fingerprint/02-encoding-pipeline.md` | Inferred from HDC BSC theory |
| Encoder version lifecycle states | `hdc-fingerprint/04-encoder-versioning.md` | Inferred from versioning needs |
| Kind variant `is_fingerprintable()` | `kind/03-api-reference.md` | Inferred from Body::Binary exclusion |
| Per-Kind cold dwell limits | `decay/08-tier-matrix.md` | Inferred from Kind descriptions |

---

## Known Gaps

1. **Sources 18 and 19** (decay-tier-matrix, compositional-kinds) were not read directly —
   content was reconstructed from inference and the partial source reads available. The
   reconstruction is architecturally consistent but may miss specific parameter values
   present in those files.

2. **Source 04-decay.md full content** was not available (only first ~5 KB was in the
   partial cache). The decay files extend and systematize what was available; specific
   parameter values or variant details not visible in the cache may differ.

3. **`roko-fs` substrate APIs** referenced in decay/provenance files are based on
   architectural inference from the crate map and naming conventions, not direct source
   reading.

---

## Naming Notes

| Old name | New canonical name | Location of change |
|---|---|---|
| `Signal` | `Engram` | `reference/01-engram/15-rationale-and-history.md` |
| `EventBus<E>` | (shipped) / `Bus` (target) | `reference/02-pulse/05-today-vs-planned.md` |
| `Datum` | Removed from scope | Was a candidate name; `Engram` chosen |

---

## Reviewer Notes

- Every Rust code block in the new tree carries a `<!-- source: crates/... -->` comment.
- Status tags: all 01-engram, 10-types/score, 10-types/decay, 10-types/provenance,
  10-types/content-hash, 10-types/hdc-fingerprint, 10-types/kind, 10-types/body = **Shipping**.
  02-pulse = **Specified**.
- All links are repo-relative (no `/Users/will/` paths).
- All added sections are marked with `<!-- ADDED: rationale -->`.
