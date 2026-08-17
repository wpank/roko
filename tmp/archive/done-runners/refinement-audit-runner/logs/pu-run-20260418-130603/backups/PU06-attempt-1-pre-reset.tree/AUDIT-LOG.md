# AUDIT-LOG — Neuro Parity Re-Verification

Re-verification of `tmp/docs-parity/06/` against current source.

**Audit date**: 2026-04-16
**Scope**: All 8 files in `tmp/docs-parity/06/` — 6 category files (A–F),
SOURCE-INDEX.md, 00-INDEX.md. All 86 original items plus 32 newly-surfaced
items (118 total after audit).
**Method**: Six category files audited in parallel (two waves of three
agents), each agent constrained to edit only its own file. Agents re-opened
every cited `file:line`, diffed every struct field list, re-ran every
grep-negative claim against `crates/`, and applied Edit calls in place to
correct drift, expand thin items, and append missed items. SOURCE-INDEX.md
verified separately against the full set of code anchors (~195) and per-crate
LOC counts. 00-INDEX.md refreshed last from the new per-file counts.

---

## Overall delta

| Metric | Before | After | Δ |
|--------|--------|-------|---|
| Total items | 86 | 118 | +32 |
| DONE | 45 (52%) | 54 (46%) | +9 absolute / −6 pts ratio |
| PARTIAL | 19 (22%) | 25 (21%) | +6 |
| NOT DONE | 22 (26%) | 39 (33%) | +17 |
| SCAFFOLD | 0 | 0 | 0 |

DONE ratio dropped not because anything regressed, but because the audit
surfaced a large volume of previously-uncatalogued design-only surface
(Doc 11 §parasite/price-equation, Doc 12 §extract_warnings/HdcClusterer/
DistillationScheduler, Doc 14 §KORAI/inheritance/ingestion-safety, Doc 06
§ontology/compression, Doc 04 §SIMD). Absolute DONE count rose +9, driven by
F.11–F.15 — several PRD-side "not implemented" self-claims in Doc 16 were
contradicted by shipping code (Dreams cycle, cross-domain strategy transfer,
pheromones, typed CausalLink HDC encoder, AntiKnowledge floor).

---

## Status changes (old → new)

| ID | Old | New | Reason |
|----|-----|-----|--------|
| A.12 | DONE | PARTIAL | `CONFIRMATION_BOOST` is private `const` at `knowledge_store.rs:28`, not `pub const` as doc 02/07 claim. Boost applied lazily at retrieval time, not as in-place mutation. |

Zero other status flips across A.01–F.10. Every DONE in the original survived
verification against today's code; every PARTIAL's missing piece is still
missing; every NOT DONE's named symbols still return zero grep hits.

(Three new F items — F.13, F.14, F.15 — flip PRD-side self-claims in Doc 16
lines 115-117 from "not implemented" to shipping code, but those flips are
PRD-vs-reality, not parity-file status drift.)

---

## Items added (new IDs)

### Category A — Knowledge Types, Tiers, Decay (+7)

| ID | PRD § | Status | Summary |
|----|-------|--------|---------|
| A.17 | 11 §Reactive AntiKnowledge Checking | NOT DONE | `reactive_anti_check(&self, candidate)` + `ReactiveCheckResult`/`Contradiction` grep-negative in `roko-neuro/` |
| A.18 | 11 §Auto AntiKnowledge Synthesis | NOT DONE | `rg 'AntiKnowledge' crates/roko-gate/` = 0; no synthesis path in `orchestrate.rs` on gate failure |
| A.19 | 11 §Epistemic Parasite Detection + Price Equation | NOT DONE | `rg 'fitness\|preservation_rate\|decision_quality\|KnowledgeHealthAudit' crates/` = 0 |
| A.20 | 00 §Neuro as semantic wrapper around Substrate | NOT DONE | `rg 'Substrate' crates/roko-neuro/` = 0; `NeuroStore` has its own persistence, not a Substrate delegate |
| A.21 | 01 §CausalLink HDC encoding | DONE | `CausalLinkParts` at `neuro/hdc.rs:12-113` with directional `permute(CAUSE_SHIFT/EFFECT_SHIFT).bind(...)`; doc 01 "Missing" bullet is stale |
| A.22 | — (undocumented) | DONE | `inferred_retention_tier` at `knowledge_store.rs:769-785` — sole path to `Persistent` tier on ingest, absent from PRDs |
| A.23 | 02 §AntiKnowledge refuted peer | DONE | `knowledge_store.rs:210-211` halves named refuted peers on ingest; doc 02:245-246 over-promises as a general negative-outcome rule |

### Category B — HDC Foundations, Operations (+5)

| ID | PRD § | Status | Summary |
|----|-------|--------|---------|
| B.19 | 04 §SIMD intrinsics strategy | NOT DONE | Blocked by `#![deny(unsafe_code)]` in `roko-primitives/src/lib.rs:14` |
| B.20 | 06 §Knowledge Ontology Formal Schema + §Provenance Chain | NOT DONE | `KnowledgeOntology`, `TypeSchema`, `ProvenanceChain` grep-negative |
| B.21 | 06 §Episode compression via bundling | NOT DONE | `compress_episode`, `BundleDiversity` helper grep-negative |
| B.22 | — | DONE | k-medoids PAM HDC clustering at `roko-learn/src/hdc_clustering.rs` (498 LOC, 10 tests) consumed by pattern discovery |
| B.23 | 04/09 §BSC capacity formula, JL bound | DONE | D=10,240 bit dimension in `roko-primitives` matches doc; SNR/JL helpers remain doc-only (acceptable) |

### Category C — Query, Cross-Domain, Context (+5)

| ID | PRD § | Status | Summary |
|----|-------|--------|---------|
| C.17 | 10 §Key Constants `CONFIRMATION_BOOST` | PARTIAL | Cross-file duplicate of A.12 — visibility drift |
| C.18 | 10 §`DistillationBackend` trait | PARTIAL | Doc claims `distill(Episode) -> Vec<KnowledgeEntry>`, code has `complete(prompt) -> String` + `model()` |
| C.19 | 10 §TierProgression default thresholds | PARTIAL | Doc claims `min_support=5`, code default `DEFAULT_MIN_SUPPORT = 3` at `tier_progression.rs:24` |
| C.20 | 08 §Abstract role vector hierarchy | NOT DONE | `KnowledgeHdcEncoder::new()` role registry not in code; `KnowledgeHdcEncoder` is unit struct with 5 hard-coded roles |
| C.21 | 08 §Strategy-Regime Bidirectional Lookup | NOT DONE | No `query_by_pattern` or reverse HDC lookup; closest is keyword-based `query_kind(topic, StrategyFragment, limit)` |

### Category D — Distillation, Progression (+4)

| ID | PRD § | Status | Summary |
|----|-------|--------|---------|
| D.11 | 12 §285-357 `extract_warnings` | NOT DONE | `extract_warnings(&Episode)` + `WarningCategory` enum grep-negative; `Episode` shape drift (`gate_verdicts` not `gate_results`, `turns: u64` not `Vec<Turn>`) |
| D.12 | 12 §359-405 `HdcClusterer` | NOT DONE | No clusterer or delegate module; D2 is text-trigram only; `roko-learn::hdc_clustering::k_medoids_pam` exists but unused by D2 |
| D.13 | 12 §407-459 Cross-Validation + AntiKnowledge gates | NOT DONE | `cross_validation_check`/`anti_knowledge_check` unimplemented; `promote_heuristics` enforces only support + confidence |
| D.14 | 12 §569-762 `DistillationScheduler` + `DistillationQualityReport` | NOT DONE | None implemented; only `UpdateFrequency::distiller_every_n` exists (drives C-factor snapshots, not distillation) |

### Category E — Somatic, Exchange, Backup (+6)

| ID | PRD § | Status | Summary |
|----|-------|--------|---------|
| E.17 | 13 §Arousal-driven retrieval scope | PARTIAL | Yerkes-Dodson `effective_limit` formula not implemented; arousal only enters as additive score bias |
| E.18 | 13 §Emotional decay | PARTIAL | Doc claims 3-day half-life; `default_half_life_hours()` returns 4.0 hours; marker depotentiation only fires during Dreams cycles |
| E.19 | 15 §Publishing Policies `[neuro.publishing]` | NOT DONE | Config block design-only, grep-negative |
| E.20 | 14 §4-stage ingestion safety pipeline | NOT DONE | QUARANTINE → CONSENSUS → SANDBOX → ADOPT grep-negative; `NeuroStore::ingest()` is unconditional |
| E.21 | 14 §KORAI token economics | NOT DONE | Posting/query costs, demurrage, challenge staking — no chain economic primitives anywhere |
| E.22 | 14 §Inheritance lineage discount `0.85^N` | NOT DONE | Grep-negative; `KnowledgeEntry` has no lineage-depth counter |

### Category F — Status, Frontier (+5)

| ID | PRD § | Status | Summary |
|----|-------|--------|---------|
| F.11 | 16 §Core features — cross-ref | DONE | AntiKnowledge 0.3 floor + auto HDC ingest shipped (`knowledge_store.rs:26, :381-399, :749-759`) — contradicts any stale PRD "not done" entry |
| F.12 | 16 §Typed HDC encoder for directional CausalLinks | DONE | `neuro/src/hdc.rs` module fully verified (cross-ref A.21) |
| F.13 | 16:117 — claimed "not implemented" | DONE | Dreams cycle wired: `roko dream run/report/schedule` at `main.rs:372-391` + `DreamCycle::run_budgeted` in `cycle.rs:398-430` |
| F.14 | 16:115 — claimed "not implemented" | DONE | Cross-domain strategy transfer wired: `generate_cross_domain_strategy_hypotheses` at `dreams/cycle.rs:1622-1691` (Dreams-cluster based, not per-ingest HDC resonance) |
| F.15 | 16:116 — claimed "not implemented" | DONE | Pheromone context surface wired: `Kind::Pheromone` + `pheromone_context` in `roko-compose` |

---

## Items changed (drift corrected)

### Line drift corrected (quick-reference counts)

| File | IDs touched |
|------|-------------|
| A | A.01 (Cargo.toml `:60-64` → `:59-64`), A.04 (`:40-59` → `:39-59`), A.06 (`:96-108` → `:95-108`, tests `:394-399` → `:393-399`, `:428-450` → `:427-450`), A.07 (tests `:402-425` → `:401-425`, `recency_factor` anchor added), A.08 (tests `:453-466` → `:452-466`; `pub const` block quoted), A.14 (test ranges `:1219-` → `:1218-1254`, `:1257-` → `:1256-1291`, source fn `:240-258` → `:219-258`; Doc 02 `:277` → `:278`), A.16 (struct/enum ranges `:124-135` → `:123-135`, `:138-149` → `:137-149`, `:305-323` → `:304-323`, `:331-339` → `:330-339`) |
| B | Every item B.01–B.18 re-anchored for precision. Highlights: B.02 (fnv offset `:200` → `:194`), B.03 (`bind` fn `:105` → `:107`), B.04 (`bundle` fn `:115` → `:117`), B.05 (`permute` fn `:142` with explicit word-split at `:149-150`), B.06 (`similarity` fn `:210` → `:211`), B.07 (`similarity_archived` fn `:225` → `:226`). B.12 rewritten with correct line spans for `encode_query`, `encode_generic_entry`, `encode_causal_link`, `CausalLinkParts`, `parse_causal_content`. B.17 corrected test count (11 → 10). B.18 corrected consumer file count (10 `.rs` + README, 13 total `roko_primitives` users) |
| C | C.01 (removed incorrect `:11` cross-ref, added doc line range + re-export anchor), C.07 (`gather()` `:266-288` → `:267-288`; 15 tests enumerated; 5456 relabeled as `add` not `query`, 7085 relabeled as `skill_library`, 12927 relabeled as `update_entries`), C.08 (builder method ranges `:253-256`/`:260-263`), C.09 (contrarian block `:387-438`, main loop `:440-499`), C.10 (`PadState` `:148-159` not `:147-181`; `affect_bias` `:1224-1258`), C.11 (orchestrate.rs:7085 caller added) |
| D | D.10 (`DreamCycle::consolidate` → `DreamCycle::run_budgeted` at `:398-430`; `DreamCycleReport` fields `:67-79`; `review_insights_from_heuristics` at `:2044-2047`; orchestrate trigger at `:5322`) |
| E | E.02 (test range `:2315-2389` → `:2313-2386` with named tests), E.14 (added `WITNESS_TOPIC` constant anchor + specific test names), E.05 (`PadState` `:146-159` → `:148-159`, added live-PAD wiring at `orchestrate.rs:12871-12873`) |
| F | F.01 (`impl NeuroStore for KnowledgeStore` `:595-609` → `:590-610`), F.05 (`CLAUDE.md :103,114` → `:134` single line with exact quote), F.06 (`effective_candidate_bid` `:592-` → `:621-630`; Signal tag span `:411-422` → `:411-451`) |
| SOURCE-INDEX | `effective_candidate_bid` `prompt.rs:592-` → `:621-632` (drift of +29 lines) |

### Field drift corrected

| ID | Drift |
|----|-------|
| A.03 | Field count 17 → 18. Doc-claim count 14 → 13 canonical; 5 extra fields enumerated (`source_model`, `model_generality`, `tier`, `emotional_tag`, `emotional_provenance`). Noted absence of `Default` derive. |
| B.01 | Field list confirmed complete; `from_bytes` constructor added to enumerated list (previously 9 → 11 sites). |
| B.12 | `KnowledgeHdcEncoder` confirmed as zero-sized unit struct with no `ItemMemory` fields (previous audit implicitly described it as stateful). |
| D.08 | `HeuristicRule` field count 11 → 12. `source_model` + `model_generality` attribute-metadata fields added. |
| E.05 | `PadState` span corrected; live-PAD wiring anchor added. |
| F.02 | `HdcVector` method count 11 → 12 pub fn; `zeros` / `random` added to enumerated list. |
| F.07 | Same count correction as F.02 (cross-ref). |
| F.08 | Witness file size 90 LOC → 199 LOC; full function line ranges added. |

### SOURCE-INDEX label corrections

Four `orchestrate.rs` anchors were mislabeled as "Raw `KnowledgeStore::query`"
but the actual operations at those lines are different. Inline NOTE flags
added:

- `orchestrate.rs:3220-3221` — actually `KnowledgeStore::init(...)` (store construction)
- `orchestrate.rs:5456` — actually `self.knowledge_store.add(anti_entry)` (AntiKnowledge ingestion on gate failure)
- `orchestrate.rs:7399` — actually `self.knowledge_store.ingest(vec![success_entry])`
- `orchestrate.rs:12927` — actually `self.knowledge_store.update_entries(...)` (tier feedback closure)

New anchor row added for `orchestrate.rs:1843` (`query_kind(...)` — the sole
direct `query_kind` call previously missing from the index).

### LOC / test count corrections (SOURCE-INDEX)

- `roko-index/src/hdc.rs` test count 11 → 10
- `roko-daimon/src/lib.rs` test count 20 → 19
- Per-crate LOC totals all within 5% of quoted figures; no crate-level refresh needed
- "Generated: 2026-04-16" complemented with "Last re-verified: 2026-04-16" header

---

## Items untouched

Items verified and confirmed unchanged (counts per file):

| File | Unchanged | Total |
|------|-----------|-------|
| A | 6 (A.02, A.05, A.10, A.11, A.13, A.15) | 16 original |
| B | 0 (every item touched for precision — most corrections were line-drift in the ±2 range) | 18 original |
| C | 5 (C.02, C.03, C.04, C.05, C.06; plus C.12–C.16 grep-negatives preserved) | 16 original |
| D | 7 (D.01, D.02, D.03, D.04, D.05, D.06, D.07) | 10 original |
| E | 11 (E.01, E.03, E.04, E.07, E.08, E.09, E.10, E.11, E.12, E.13, E.15, E.16) | 16 original |
| F | 4 (F.03, F.04, F.09, F.10) | 10 original |
| **Total** | **33** | **86** |

---

## Items expanded (thin → concrete)

All items with < 3 lines of **Reality** were expanded with specific `file:line`
anchors, test names, and grep outputs:

- **A**: A.01 (+crate listing), A.03 (+5-field list + Default note), A.06 (+derive attribute, f32 return clarified), A.07 (+Doc 02 stale bullet, `recency_factor` wrapper anchor), A.08 (+full `pub const` block, AntiKnowledge+INFINITY intent), A.12 (+multi-source gate, `KnowledgeConfirmationRecord` clarification), A.14 (+`evaluate_promotion`, `inferred_retention_tier` context), A.16 (+constructor + `coarse_emotion_label` + retrieval scoring linkage).
- **B**: B.03, B.04, B.05, B.06, B.07, B.08, B.09, B.10, B.11 expanded from 2-3 lines to concrete file:line-anchored passages. B.12, B.13, B.14, B.15 substantially rewritten with richer anchors.
- **C**: C.07 (15 test hits enumerated, orchestrate.rs call sites categorized), C.01 (+re-export anchor), C.08 (+full const inventory), C.09 (+winner re-sort + SelectionMode detail), C.10 (`apply_somatic_bias` is module-level `pub fn`, not a method).
- **D**: D.08 (+model-scope field callout + `applies_to_model`), D.09 (+call-site context, signature drift Fix sketch, X-ref to D.14), D.10 (+`DreamCycleReport`, `review_insights_from_heuristics`, orchestrate trigger; X-ref to D.11–D.14).
- **E**: E.02 (+clamp line refs + 3 test anchors), E.05 (+each bias formula's specific line + upstream orchestrate.rs wiring), E.06 (corrected `context.rs:798` misattribution; disambiguated 5 `0.15` literals), E.14 (+`WITNESS_TOPIC`, named both tests).
- **F**: F.01 (+NeuroStore method list + half-life constants), F.05 (refined CLAUDE.md reference to `:134` with quoted line), F.06 (expanded Signal tag enumeration to 7 tags), F.08 (+file size + fn line ranges), F.02 (+`zeros` / `random`).

---

## Systematic issues observed

1. **PRD Doc 16 (Current Status) is self-inconsistent**. Abstract and
   Implementation Plan Mapping (§147-155) acknowledge recent wins; Priority 3
   table at §115-117 still lists Dreams cycle, cross-domain transfer, and
   pheromones as unimplemented — all three ship. F.13–F.15 catalog this.

2. **Ghost `Fact` variant + `FACT_HALF_LIFE_DAYS=365.0`** survives in Docs
   01 (§345), 02 (§43, 65, 87, 112, 210), 03 (§181, 276), 07 (§229) even
   though the enum collapsed `Fact` into `Insight` via serde alias. A single
   find/replace across the PRDs would clear it.

3. **`KnowledgeEntry` PRD schema is 5 fields stale** (A.03). `source_model`,
   `model_generality`, `tier`, `emotional_tag`, `emotional_provenance` all
   exist with `#[serde(default)]` but appear in no PRD field table.

4. **`CONFIRMATION_BOOST` visibility drift** (A.12 / C.17). Docs 02 and 07
   claim `pub const`; code is private `const` at `knowledge_store.rs:28`
   applied lazily at retrieval time.

5. **Doc 12 §"Implementation Details" (§285-763) reads as normative code
   definitions but is entirely aspirational**. Every named type (`extract_warnings`,
   `WarningCategory`, `HdcClusterer`, `cross_validation_check`,
   `anti_knowledge_check`, `DistillationScheduler`, `DistillationQualityReport`)
   is grep-negative. The §"Missing" list at §775-781 alludes to this but the
   main body should carry a "Design — not yet implemented" banner.

6. **Doc 08 banner says "Implementation: Built"** (line 6) while §"Current
   Status and Gaps" (line 786) admits the resonance loop, confirmation
   protocol, analogy API, and strategy-regime lookup are all missing. The
   banner is wrong; the admission is correct.

7. **Doc 10 §"Integration Points" step 1 is uniformly aspirational** —
   `ContextAssembler` is fully tested in `crates/roko-neuro/src/context.rs`
   but never constructed from `orchestrate.rs`. 15 `ContextAssembler::new|.gather(`
   hits in `crates/` are all inside the assembler's own module. Highest-impact
   operational gap in the subsystem (C.07 Tier 1 HIGH).

8. **`KnowledgeMemoryIndex` → `MemoryIndex` naming drift** in docs 05/06/10.
   Doc refers to `KnowledgeMemoryIndex` throughout; real type is `MemoryIndex`
   (feature-gated under `#[cfg(feature="hdc")]`).

9. **`bardo-primitives` → `roko-primitives` doc references**. Docs
   `06-neuro/04-`, `05-`, `06-` still have §"Key sources" blocks pointing at
   `bardo-primitives/src/hdc.rs`; code renamed everywhere.

10. **HDC encoder doc-vs-code shape mismatch**:
    - Doc 06 §"Automatic HDC Encoding Pipeline" claims stateful struct with
      three `ItemMemory` fields; real `KnowledgeHdcEncoder` is zero-sized unit
      struct with `self` (not `&mut self`) receivers.
    - Doc 06 §"Trigram-Based Name Encoding" shows `.permute(pos)` per trigram;
      code omits positional permutation.
    - Doc 06 §"Code Symbol Fingerprinting" shows `symbol:*` seed prefix; code
      uses `roko:role:*` prefix.
    - Doc 06 `fingerprint_symbol` spec is `bundle(kind, name)`; real code is
      `bind(role, bundle(name, context))`.

11. **0.526 similarity threshold**: 23 doc references across Doc 09, zero
    code references. No `SIMILARITY_THRESHOLD` constant; no `NeuroQuery {
    min_similarity }` struct. B.15 Tier 1 HIGH.

12. **`#![deny(unsafe_code)]` in `roko-primitives/src/lib.rs:14`** blocks any
    future SIMD intrinsics from landing in that crate without a feature-gated
    carve-out (B.19).

13. **`roko-index/src/hdc.rs` inlines its own HDC** rather than depending on
    `roko_primitives::HdcVector` — a parity/duplication oddity carried over
    from pre-rename days (B.17 DONE-with-drift).

14. **Doc 13 coefficient drift**: mood-congruence `× 0.15` (doc) vs
    `0.20 + 0.20 × intensity` (code, cosine rather than dot product); arousal
    scope formula unimplemented (score bias only); emotional half-life 3 days
    (doc) vs 4 hours (code).

15. **Doc 14 is overwhelmingly vision**: MeshSync, KoraiChannel, LetheChannel,
    LibraryOfBabel, publishing policies, 4-stage ingestion safety, immune-memory
    Bloom filter, KORAI tokenomics, lineage discounting all grep-negative.
    Only `NeuroStore::ingest()` (channel 1 self-distillation) + `ChainWitnessEngine`
    (attestation-hash anchoring, not knowledge vectors) have shipping code.

16. **Doc 15 CLI surface is 100% design-only**: `NeuroCmd` has exactly 3
    variants (`Query`, `Stats`, `Gc`); `Backup`, `Restore`, `Publish`,
    `BackupManifest`, `agent create`, `agent delete` all grep-negative.

17. **Undocumented `inferred_retention_tier` heuristic** (A.22) at
    `knowledge_store.rs:769-785` is the only path to `Persistent` tier on
    ingest — no PRD mentions it. `evaluate_tier_progression` in
    `tier_progression.rs` stops at `Consolidated`.

18. **`.roko/neuro/PLAYBOOK.md` ghost path** (Doc 12): `write_playbook` is
    parametric and has zero call sites. The real canonical on-disk playbook is
    `.roko/memory/playbook.toml` maintained by `roko-dreams::PlaybookStore` — a
    different crate, different format, different artefact than Doc 12 describes.

19. **D3 stage is inert**: `compile_playbook` runs every Dream cycle as part
    of `analyze`, but the result is returned in-memory only. `write_playbook`
    is the only disk-writing path and nobody calls it.

20. **`KnowledgeMemoryIndex`/`MemoryIndex` + `orchestrate.rs` op-label
    cluster**: SOURCE-INDEX.md had four anchors labeled "Raw
    `KnowledgeStore::query`" that are actually `init` / `add` / `ingest` /
    `update_entries`. This inflated the "raw query call site" count in the
    Second Largest Gap paragraph (was "9+", now correctly described as 7 raw
    query sites plus init/add/ingest/update_entries).

---

## Verification smoke-tests (self-check)

1. **Random line-ref check (30)** — all sampled `file:line` references
   pointed to the cited symbol within ±5 lines after edits.
2. **Grep-negative spot check (10)** — every re-run grep-negative assertion
   still returned zero hits against `crates/`.
3. **Tally consistency** — sum of per-file DONE/PARTIAL/NOT DONE counts
   = 54 / 25 / 39 = 118 matches 00-INDEX.md totals.
4. **SOURCE-INDEX LOC accuracy** — each crate LOC quoted within 5% of actual
   `wc -l` over `crates/<crate>/src/*.rs`.
5. **Tier 1 smoke test** — `rg 'ContextAssembler::new|\.gather\(' crates/`
   confirmed 15 hits all in `crates/roko-neuro/src/context.rs`; C.07 gap
   description holds.
6. **AUDIT-LOG coverage** — every changed item in this log corresponds to
   an edit applied in the per-file Edit calls during Steps 1-2.

---

*End of audit log.*
