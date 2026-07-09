# 06-Neuro Parity Analysis

Gap analysis of `docs/06-neuro/` against the current neuro, HDC, somatic, dreams, chain, compose, and orchestrator codepaths that actually consume neuro outputs.

Generated: 2026-04-16

---

## How To Use This Batch

This batch should be treated as **neuro runtime activation + contract cleanup**, not as a license to implement every advanced memory, chain, exchange, or frontier idea described across docs `00`-`16`.

- Prefer activating already-shipped neuro surfaces before inventing new ones.
- Treat `crates/roko-neuro/src/context.rs`, `knowledge_store.rs`, `tier_progression.rs`, and `crates/roko-cli/src/orchestrate.rs` as the main runtime seams.
- Keep cross-domain transfer, mesh sync, Korai, Lethe, token economics, and neurosymbolic frontier work explicitly bounded unless a batch says otherwise.
- Every batch should be able to stop with a clear `PASS`, `FAIL`, or `BLOCKED` result and leave behind evidence: files changed, commands run, outputs, and explicit deferrals.

Recommended single-agent serial order inside batch `06`:

`N1 -> N2 -> N3 -> N5 -> N6 -> N4 -> N7 -> N8 -> N9`

Reasoning:

- `N1` turns on the strongest currently-unused runtime seam first.
- `N2` makes the live neuro query contract less implicit once the assembler path is real.
- `N3` and `N4` separate the real distillation pipeline from the larger designed-only blocks.
- `N5` and `N6` cover the two biggest missing operational surfaces after retrieval: source ownership and backup/restore.
- `N7` and `N8` keep the large cross-domain and advanced-HDC stories bounded instead of letting them sprawl.
- `N9` is the cleanup pass once the runtime and docs boundaries are clearer.

---

## Document Index

| File | Docs Covered | Items | Status |
|------|--------------|-------|--------|
| [A-knowledge-types-tiers-decay.md](A-knowledge-types-tiers-decay.md) | 00, 01, 02, 03, 07, 11 | A.01-A.23 | 15 DONE / 3 PARTIAL / 5 NOT DONE |
| [B-hdc-foundations-operations.md](B-hdc-foundations-operations.md) | 04, 05, 06, 09 | B.01-B.23 | 12 DONE / 1 PARTIAL / 10 NOT DONE |
| [C-query-crossdomain-context.md](C-query-crossdomain-context.md) | 08, 10 | C.01-C.21 | 5 DONE / 7 PARTIAL / 9 NOT DONE |
| [D-distillation-progression.md](D-distillation-progression.md) | 12 | D.01-D.14 | 3 DONE / 7 PARTIAL / 4 NOT DONE |
| [E-somatic-exchange-backup.md](E-somatic-exchange-backup.md) | 13, 14, 15 | E.01-E.22 | 9 DONE / 5 PARTIAL / 8 NOT DONE |
| [F-status-frontier.md](F-status-frontier.md) | 16 | F.01-F.15 | 10 DONE / 2 PARTIAL / 3 NOT DONE |
| [BATCHES.md](BATCHES.md) | — | 9 batches | Execution contract |
| [SOURCE-INDEX.md](SOURCE-INDEX.md) | — | Verified code anchors | Reference |
| [AUDIT-LOG.md](AUDIT-LOG.md) | — | Re-verification delta | Historical reference |

Doc `INDEX.md` is absorbed into this file.

---

## Overall Parity: 54/118 items DONE (46%)

The neuro batch is in a split state:

- the **core neuro substrate is much more real than many readers would expect**,
- but the **best retrieval/composition path is still bypassed in production**,
- and several later docs still describe **design-only transfer, exchange, and backup systems as if they are nearer to runtime than they are**.

### Tier 1 — Should Exist Now (runtime-critical)

| ID | Title | Status | Severity |
|----|-------|--------|----------|
| C.07 | `ContextAssembler` is real but still not used on the main orchestrator path | PARTIAL | HIGH |
| B.15 | no explicit similarity-threshold or `min_similarity` contract exists in runtime code | NOT DONE | HIGH |
| D.13 | heuristic-promotion cross-validation and AntiKnowledge gates are absent | NOT DONE | HIGH |
| E.11 | inflow channels are effectively self-only despite a much richer doc story | PARTIAL | HIGH |
| E.15 | neuro backup / restore / publish CLI surface is absent | NOT DONE | HIGH |
| E.20 | ingestion safety staging pipeline is absent; ingest is unconditional | NOT DONE | HIGH |

### Tier 2 — Should Exist Soon (operational quality)

| ID | Title | Status | Severity |
|----|-------|--------|----------|
| C.03-C.05 | query scoring, stats, and confirmation-record contracts drift from the docs | PARTIAL | MEDIUM |
| C.12-C.16 | doc-08 cross-domain transfer block is still entirely design-only | NOT DONE | MEDIUM |
| D.03 | D3 playbook output path is parametric and uncalled | PARTIAL | MEDIUM |
| D.14 | scheduler / quality-report contract is absent | NOT DONE | MEDIUM |
| E.16 | `BackupManifest` and backup layout contract are absent | NOT DONE | MEDIUM |
| F.05 | code-side `roko-golem` dissolution is complete but meta-docs are stale | DONE (docs stale) | MEDIUM |

### Tier 3 — Future / Research / Frontier

| ID | Title | Status | Severity |
|----|-------|--------|----------|
| A.15, A.17-A.19 | on-chain demurrage, reactive AntiKnowledge, parasite diagnostics | NOT DONE | LOW |
| B.08-B.14 | bundle accumulator, item memory, resonator, structured query, three-tier search | NOT DONE | LOW |
| B.19-B.21 | SIMD, ontology schema, episode-compression helpers | NOT DONE | LOW |
| E.12-E.13, E.19, E.21-E.22 | mesh sync, Korai/Lethe, publishing policy, tokenomics, lineage discount | NOT DONE | LOW |
| F.07-F.10 | resonator frontier, mesh frontier, crystals, neurosymbolic storage | NOT DONE / PARTIAL | LOW |

### Already Shipped

| ID | Title | Status |
|----|-------|--------|
| A.01-A.14 | rename, tiering, decay, AntiKnowledge floor, and core lifecycle | DONE / PARTIAL where noted |
| A.21-A.23 | CausalLink HDC encoding, inferred retention tier, peer-confidence halving | DONE |
| B.01-B.07, B.16-B.18, B.22-B.23 | core 10,240-bit HDC math, helpers, crate split, k-medoids, math consistency | DONE |
| C.01-C.02, C.06, C.09-C.10 | `NeuroStore`, JSONL persistence, `MemoryIndex`, budget allocator, somatic bias path | DONE |
| D.04, D.09-D.10 | Distiller, episode-distillation hook, Dreams integration | DONE |
| E.01-E.07, E.10, E.14 | somatic landscape, strategy-space primitives, contrarian blend, attestation witness | DONE / PARTIAL where noted |
| F.01-F.04, F.11-F.15 | core status claims, stale “missing” claims corrected, Dreams / pheromone / transfer understatements fixed | DONE / PARTIAL where noted |

---

## Execution Boundaries

These are valid findings, but they should usually be handled outside the core runtime-hardening work of batch `06`:

| Item | Better Home | Why |
|------|-------------|-----|
| full mesh sync, Korai, Lethe, and publish-to-network flows | later exchange / network parity pass | they are not needed to make local neuro runtime honest |
| on-chain demurrage, staking, KORAI economics | later chain / economics pass | batch `06` should not invent token systems |
| `ResonatorNetwork`, SIMD HDC kernels, three-tier search, ontology schema | later HDC research / performance pass | none are prerequisites for activating current neuro runtime |
| `KnowledgeCrystal`, `MetabolismMetrics`, `NeurosymbolicStore` | later frontier pass | frontier concepts should be labeled, not implied to ship |
| full doc-08 resonance / confirmation network | later neuro-transfer pass unless a very small seam is chosen | the current codebase lacks the prerequisite structures |

Batch `06` should usually produce:

- one real production caller for the best existing neuro retrieval path,
- a more explicit query and distillation contract,
- a bounded story for source ownership and backup/restore,
- and cleaner truth-in-advertising across the later neuro docs.

---

## Critical Neuro Issues

1. **The best retrieval pipeline is built but still bypassed.** `ContextAssembler` already contains the budget, PAD, and contrarian logic the docs talk about, but orchestrator production paths still query the store directly.
2. **The query contract is too implicit.** Thresholds, stats shape, confirmation semantics, and config tunables are split between docs, constants, and assumptions rather than one obvious runtime contract.
3. **The distillation story is half-runtime, half-spec.** D1 and Dreams-side progression are real; warning extraction, promotion guards, scheduler, and quality-report blocks are still doc-first.
4. **Exchange and backup docs are much further ahead than the code.** Inflow channels, safety staging, restore lineage, and publishing policies are mostly design surfaces.
5. **Some status docs are wrong in both directions.** A few sections still overclaim unbuilt systems, while others still say “not implemented” for surfaces that already ship.

---

## Key Insight

Batch `06` does **not** mainly need more neuro theory.

It needs a tighter contract between:

- the **real neuro foundations** already in `roko-neuro`,
- the **production callers** that still bypass or underuse those foundations,
- and the **docs** that mix shipped runtime, thin scaffolding, and frontier design in the same tone.

That means the highest-value work here is usually:

1. activate the best existing runtime seam,
2. make thresholds, scoring, and distillation rules explicit,
3. choose clear boundaries for source ownership and backup/restore,
4. demote research, network, and frontier ideas to honest handoff status unless a batch explicitly owns them.

---

## Batch 06 Success Definition

Batch `06` is successful when:

- `ContextAssembler` is either on a real production path or explicitly documented as intentionally not in path,
- the neuro query contract is understandable without reverse-engineering markdown and grep results,
- the distillation pipeline has one honest runtime story and one honest list of designed-only surfaces,
- source-channel and backup/restore ownership are either implemented as a bounded MVP or clearly demoted,
- and later neuro docs no longer contradict shipped status.

Use [AUDIT-LOG.md](AUDIT-LOG.md) for the detailed re-verification delta, not as the primary execution brief.
